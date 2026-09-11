//! Independent Rust supervisors retain locks and observe coordinator socket EOF.
use super::{
    graph::{self, Spec},
    now,
    state::{self, Directory},
};
use nix::{
    sys::signal::{killpg, Signal},
    unistd::Pid,
};
use rustix::net::{
    self, RecvAncillaryBuffer, RecvAncillaryMessage, SendAncillaryBuffer, SendAncillaryMessage,
};
use serde_json::{json, Value};
use std::ffi::OsString;
use std::fs::File;
use std::io::{self, IoSlice, IoSliceMut, Read, Write};
use std::mem::MaybeUninit;
use std::net::Shutdown;
use std::os::fd::{AsFd, OwnedFd};
use std::os::unix::{
    net::UnixStream,
    process::{CommandExt, ExitStatusExt},
};
use std::path::Path;
use std::process::{Child, Command, Stdio};
use std::time::{Duration, Instant};

fn binding(fd: impl AsFd) -> Result<Value, String> {
    let stat = rustix::fs::fstat(fd).map_err(|e| e.to_string())?;
    Ok(json!({"device":stat.st_dev,"inode":stat.st_ino}))
}
pub(super) struct Running {
    child: Child,
    control: UnixStream,
    held: Vec<File>,
    pub(super) name: String,
}
impl Running {
    pub(super) fn cancel(&self) {
        let _ = self.control.shutdown(Shutdown::Write);
    }
    pub(super) fn poll(&mut self) -> Result<Option<std::process::ExitStatus>, String> {
        self.child.try_wait().map_err(|e| e.to_string())
    }
}
impl Drop for Running {
    fn drop(&mut self) {
        self.cancel();
        // Keep our duplicate locks until the supervisor has completed cleanup.
        let _ = self.child.wait();
        self.held.clear();
    }
}
pub(super) fn start(
    root: &Path,
    directory: &Directory,
    active: &File,
    held: Vec<File>,
    name: &str,
    spec: &Spec,
    helper: &Path,
    run_id: &str,
) -> Result<Running, String> {
    let cwd = Directory::cwd(root, &graph::relative(&spec.cwd)?)?;
    start_anchored(cwd, directory, active, held, name, spec, helper, run_id)
}
fn start_anchored(
    cwd: Directory,
    directory: &Directory,
    active: &File,
    held: Vec<File>,
    name: &str,
    spec: &Spec,
    helper: &Path,
    run_id: &str,
) -> Result<Running, String> {
    let mut fds = vec![cwd.0.as_fd(), directory.0.as_fd(), active.as_fd()];
    fds.extend(held.iter().map(AsFd::as_fd));
    let bindings = fds.iter().map(binding).collect::<Result<Vec<_>, _>>()?;
    let request =
        json!({"protocol":2,"run_id":run_id,"name":name,"command":spec,"bindings":bindings});
    directory.write_json(&format!("{name}.launch.json"), &request)?;
    let (mut control, incoming) = UnixStream::pair().map_err(|e| e.to_string())?;
    let mut child = Command::new(helper)
        .arg("__runner-supervisor")
        .stdin(Stdio::from(OwnedFd::from(incoming)))
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .process_group(0)
        .spawn()
        .map_err(|e| format!("start Rust supervisor: {e}"))?;
    let transfer = (|| {
        let message = SendAncillaryMessage::ScmRights(&fds);
        let mut bytes = vec![MaybeUninit::uninit(); message.size()];
        let mut ancillary = SendAncillaryBuffer::new(&mut bytes);
        if !ancillary.push(message) {
            return Err("cannot encode supervisor descriptors".into());
        }
        if net::sendmsg(
            &control,
            &[IoSlice::new(b"R")],
            &mut ancillary,
            net::SendFlags::empty(),
        )
        .map_err(|e| e.to_string())?
            != 1
        {
            return Err("incomplete supervisor handshake".into());
        }
        let request = serde_json::to_vec(&request).map_err(|e| e.to_string())?;
        let length = u32::try_from(request.len()).map_err(|e| e.to_string())?;
        control
            .write_all(&length.to_be_bytes())
            .and_then(|_| control.write_all(&request))
            .map_err(|e| e.to_string())
    })();
    if let Err(error) = transfer {
        drop(control);
        let _ = child.wait();
        return Err(error);
    }
    Ok(Running {
        child,
        control,
        held,
        name: name.into(),
    })
}

fn receive() -> Result<(UnixStream, Vec<OwnedFd>, Value), String> {
    let fd = rustix::io::dup(io::stdin())
        .map_err(|e| format!("supervisor requires inherited Unix socket: {e}"))?;
    let mut socket = UnixStream::from(fd);
    rustix::io::fcntl_setfd(&socket, rustix::io::FdFlags::CLOEXEC).map_err(|e| e.to_string())?;
    socket
        .set_read_timeout(Some(Duration::from_secs(5)))
        .map_err(|e| e.to_string())?;
    let mut byte = [0_u8; 1];
    let mut ancillary_bytes = vec![MaybeUninit::uninit(); rustix::cmsg_space!(ScmRights(203))];
    let mut ancillary = RecvAncillaryBuffer::new(&mut ancillary_bytes);
    let received = net::recvmsg(
        &socket,
        &mut [IoSliceMut::new(&mut byte)],
        &mut ancillary,
        net::RecvFlags::empty(),
    )
    .map_err(|e| format!("supervisor handshake: {e}"))?;
    if received.bytes != 1 || byte != *b"R" || !received.flags.is_empty() {
        return Err("invalid supervisor handshake".into());
    }
    let mut descriptors = Vec::new();
    for message in ancillary.drain() {
        if let RecvAncillaryMessage::ScmRights(fds) = message {
            descriptors.extend(fds);
        } else {
            return Err("unexpected ancillary message".into());
        }
    }
    for fd in &descriptors {
        rustix::io::fcntl_setfd(fd, rustix::io::FdFlags::CLOEXEC).map_err(|e| e.to_string())?;
    }
    let mut length = [0_u8; 4];
    socket.read_exact(&mut length).map_err(|e| e.to_string())?;
    let length = usize::try_from(u32::from_be_bytes(length)).map_err(|e| e.to_string())?;
    if length > 16 * 1024 * 1024 {
        return Err("supervisor handshake exceeds 16 MiB".into());
    }
    let mut bytes = vec![0_u8; length];
    socket.read_exact(&mut bytes).map_err(|e| e.to_string())?;
    let request = serde_json::from_slice(&bytes).map_err(|e| e.to_string())?;
    socket.set_read_timeout(None).map_err(|e| e.to_string())?;
    socket.set_nonblocking(true).map_err(|e| e.to_string())?;
    Ok((socket, descriptors, request))
}
fn cleanup(child: &mut Child) -> Vec<String> {
    let mut errors = Vec::new();
    let Ok(pid) = i32::try_from(child.id()) else {
        return vec!["invalid process group ID".into()];
    };
    for signal in [Signal::SIGTERM, Signal::SIGKILL] {
        match killpg(Pid::from_raw(pid), signal) {
            Ok(()) => (),
            Err(nix::errno::Errno::ESRCH) => break,
            Err(e) => errors.push(e.to_string()),
        }
        if signal == Signal::SIGTERM {
            std::thread::sleep(Duration::from_millis(100));
        }
    }
    let deadline = Instant::now() + Duration::from_secs(2);
    loop {
        match child.try_wait() {
            Ok(Some(_)) => break,
            Err(e) => {
                errors.push(e.to_string());
                break;
            }
            Ok(None) if Instant::now() < deadline => std::thread::sleep(Duration::from_millis(10)),
            _ => {
                errors.push("process did not reap after SIGKILL".into());
                break;
            }
        }
    }
    errors
}
fn observe(
    mut socket: UnixStream,
    cwd: Directory,
    directory: &Directory,
    locks: &[OwnedFd],
    name: &str,
    spec: &Spec,
) -> Result<Value, String> {
    let started = Instant::now();
    let mut result = serde_json::to_value(spec).map_err(|e| e.to_string())?;
    let object = result.as_object_mut().ok_or("invalid command")?;
    object.insert("status".into(), "error".into());
    object.insert("exit_code".into(), Value::Null);
    object.insert("started_at".into(), json!(now()));
    let stdout = directory.open(&format!("{name}.stdout.log"), true, true)?;
    let mut stderr = directory.open(&format!("{name}.stderr.log"), true, true)?;
    rustix::process::fchdir(&cwd.0).map_err(|e| e.to_string())?;
    let mut argv = spec.argv.iter();
    let program = argv.next().ok_or("empty command")?;
    let mut command = Command::new(program);
    command
        .args(argv)
        .stdin(Stdio::null())
        .stdout(stdout.try_clone().map_err(|e| e.to_string())?)
        .stderr(stderr.try_clone().map_err(|e| e.to_string())?)
        .process_group(0);
    // This dedicated supervisor has one thread and one command spawn. Only the
    // active/resource lock descriptions cross exec: a surviving command (and
    // its descendants) must retain them even if this supervisor is killed.
    // Cwd, run directory and lifecycle socket remain private CLOEXEC descriptors.
    let mut spawn = (|| {
        for fd in locks {
            rustix::io::fcntl_setfd(fd, rustix::io::FdFlags::empty())?;
        }
        command.spawn()
    })();
    // Restore every descriptor before any further work, including after a
    // partial flag change or spawn failure. Restoration failure after a spawn
    // must clean the command up rather than lose a live child on an error path.
    let mut restore_errors: Vec<_> = locks
        .iter()
        .filter_map(|fd| {
            rustix::io::fcntl_setfd(fd, rustix::io::FdFlags::CLOEXEC)
                .err()
                .map(|error| error.to_string())
        })
        .collect();
    if !restore_errors.is_empty() {
        if let Ok(child) = &mut spawn {
            restore_errors.extend(cleanup(child));
        }
        return Err(format!(
            "cannot restore private supervisor lock descriptors: {}",
            restore_errors.join("; ")
        ));
    }
    match spawn {
        Err(error) => {
            writeln!(stderr, "cannot execute {program:?}: {error}").map_err(|e| e.to_string())?;
            object.insert("status".into(), "failed".into());
            object.insert("exit_code".into(), json!(127));
            object.insert("error".into(), error.to_string().into());
        }
        Ok(mut child) => {
            object.insert("pid".into(), json!(child.id()));
            let status = loop {
                let mut marker = [0_u8; 1];
                match socket.read(&mut marker) {
                    Ok(0) => break "interrupted",
                    Ok(_) => break "interrupted",
                    Err(e) if e.kind() == io::ErrorKind::WouldBlock => (),
                    Err(_) => break "interrupted",
                }
                match child.try_wait() {
                    Ok(Some(exit)) => break if exit.success() { "passed" } else { "failed" },
                    Ok(None) => (),
                    Err(error) => {
                        object.insert("error".into(), error.to_string().into());
                        break "error";
                    }
                }
                if started.elapsed().as_secs_f64() >= spec.timeout_seconds {
                    break "timeout";
                }
                std::thread::sleep(Duration::from_millis(15));
            };
            object.insert("status".into(), status.into());
            let errors = cleanup(&mut child);
            if !errors.is_empty() {
                object.insert("cleanup_errors".into(), json!(errors));
                if status == "passed" {
                    object.insert("status".into(), "error".into());
                }
            }
            if let Ok(Some(exit)) = child.try_wait() {
                object.insert(
                    "exit_code".into(),
                    json!(exit.code().or_else(|| exit.signal().map(|signal| -signal))),
                );
            }
        }
    }
    stdout
        .sync_all()
        .and_then(|_| stderr.sync_all())
        .map_err(|e| e.to_string())?;
    object.insert("finished_at".into(), json!(now()));
    object.insert(
        "duration_seconds".into(),
        json!(started.elapsed().as_secs_f64()),
    );
    for stream in ["stdout", "stderr"] {
        let file = format!("{name}.{stream}.log");
        object.insert(
            stream.into(),
            json!({"file":file,"sha256":directory.file_digest(&file)?}),
        );
    }
    Ok(result)
}
pub(super) fn supervisor(args: &[OsString]) -> Result<Value, String> {
    if !args.is_empty() {
        return Err("supervisor accepts only its inherited descriptor handshake".into());
    }
    let (socket, descriptors, request) = receive()?;
    let name = request
        .get("name")
        .and_then(Value::as_str)
        .filter(|name| graph::name(name))
        .ok_or("invalid supervisor command name")?;
    let run_id = request
        .get("run_id")
        .and_then(Value::as_str)
        .filter(|id| super::run_id(id))
        .ok_or("invalid supervisor run ID")?;
    if request.get("protocol") != Some(&json!(2)) {
        return Err("unsupported supervisor protocol".into());
    }
    let spec: Spec = serde_json::from_value(
        request
            .get("command")
            .ok_or("missing supervisor command")?
            .clone(),
    )
    .map_err(|e| e.to_string())?;
    if descriptors.len() != spec.resources.len() + 3 {
        return Err("wrong supervisor descriptor count".into());
    }
    let bindings = descriptors
        .iter()
        .map(binding)
        .collect::<Result<Vec<_>, _>>()?;
    if request.get("bindings") != Some(&json!(bindings)) {
        return Err("supervisor descriptor identity mismatch".into());
    }
    let mut descriptors = descriptors.into_iter();
    let cwd = Directory::checked(descriptors.next().ok_or("missing cwd descriptor")?, false)?;
    let directory = Directory::checked(descriptors.next().ok_or("missing run descriptor")?, true)?;
    let locks: Vec<_> = descriptors.collect();
    for fd in &locks {
        state::ordinary(fd)?;
    }
    let actual_active = directory.open("active.lock", false, false)?;
    if binding(&actual_active)? != binding(locks.first().ok_or("missing active descriptor")?)? {
        return Err("active lock does not belong to run".into());
    }
    let launch: Value = serde_json::from_slice(&directory.read(&format!("{name}.launch.json"))?)
        .map_err(|e| e.to_string())?;
    if launch != request {
        return Err("supervisor request does not match protected launch record".into());
    }
    let record = super::run::load(&directory)?;
    if record.get("schema_version") != Some(&json!(2))
        || record.get("runtime") != Some(&json!("rust"))
        || record.get("run_id") != Some(&json!(run_id))
        || record.get("status") != Some(&json!("running"))
        || record.get("commands").and_then(|c| c.get(name)) != request.get("command")
    {
        return Err("supervisor request is not bound to an active Rust run".into());
    }
    let observed = observe(socket, cwd, &directory, &locks, name, &spec)
        .unwrap_or_else(|error| json!({"status":"error","exit_code":null,"error":error}));
    directory.write_json(&format!("{name}.result.json"), &observed)?;
    drop(locks);
    Ok(json!({"exit_code":0}))
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn cwd_swap_after_open_cannot_redirect_process_outside_repository(
    ) -> Result<(), Box<dyn std::error::Error>> {
        let scratch = tempfile::tempdir()?;
        let root = scratch.path().canonicalize()?;
        std::fs::create_dir(root.join("nested"))?;
        std::fs::create_dir(root.join("outside"))?;
        let cwd = Directory::cwd(&root, Path::new("nested"))?;
        std::fs::rename(root.join("nested"), root.join("moved"))?;
        std::os::unix::fs::symlink("outside", root.join("nested"))?;
        let directory = Directory::root(&root)?.child("run", true, true)?;
        let active = directory.open("active.lock", true, true)?;
        assert!(state::lock(&active)?);
        let current = std::env::current_exe()?;
        let helper = current
            .parent()
            .and_then(Path::parent)
            .ok_or("no profile")?
            .join("examples/runner_probe");
        let spec = Spec {
            argv: vec![helper.to_string_lossy().into_owned(), "cwd".into()],
            cwd: "nested".into(),
            requires: vec![],
            resources: vec![],
            timeout_seconds: 2.0,
        };
        let id = super::super::identifier();
        let record = json!({"schema_version":2,"runtime":"rust","run_id":id,"status":"running","commands":{"where":spec}});
        directory.write_json(
            "record.json",
            &json!({"sha256":super::super::digest(&record)?,"record":record}),
        )?;
        let mut running = start_anchored(
            cwd,
            &directory,
            &active,
            vec![],
            "where",
            &spec,
            &helper,
            &id,
        )?;
        while running.poll()?.is_none() {
            std::thread::sleep(Duration::from_millis(10));
        }
        let output = String::from_utf8(directory.read("where.stdout.log")?)?;
        assert_eq!(output.trim(), root.join("moved").to_string_lossy());
        assert_eq!(
            serde_json::from_slice::<Value>(&directory.read("where.result.json")?)?.get("status"),
            Some(&json!("passed"))
        );
        Ok(())
    }
}
