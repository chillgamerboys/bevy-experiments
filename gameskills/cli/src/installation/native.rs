//! Bounded native discovery; no model turns, installation RPCs or global edits.
use super::{archive, installed, strings};
#[cfg(unix)]
use super::{files, package_files, InstructionBundle};
use serde_json::{json, Value};
use std::{path::Path, process::Command};

#[cfg(unix)]
use std::{
    collections::{BTreeMap, BTreeSet},
    path::PathBuf,
};

/// Construct client arguments against exactly one immutable package selection.
pub fn native_argv(
    bundle_path: &Path,
    client: &str,
    extra: &[String],
) -> Result<Vec<String>, String> {
    let bundle = archive::source(bundle_path)?;
    let bundle_path = bundle_path.canonicalize().map_err(|e| e.to_string())?;
    let mut args = vec![client.to_owned()];
    match client {
        "claude" => {
            for package in &bundle.selected {
                args.extend([
                    "--plugin-dir".into(),
                    bundle_path
                        .join("plugins")
                        .join(package)
                        .to_str()
                        .ok_or("bundle path must be UTF-8")?
                        .to_owned(),
                ]);
            }
        }
        "codex" => {
            let market = archive::marketplace(&bundle);
            args.extend([
                "-c".into(),
                format!("marketplaces.{market}.source_type=\"local\""),
                "-c".into(),
                format!(
                    "marketplaces.{market}.source={}",
                    serde_json::to_string(bundle_path.to_str().ok_or("bundle path must be UTF-8")?)
                        .map_err(|e| e.to_string())?
                ),
            ]);
            for package in &bundle.selected {
                args.extend([
                    "-c".into(),
                    format!("plugins.{package}@{market}.enabled=true"),
                ]);
            }
        }
        _ => return Err("supported clients are codex and claude".into()),
    }
    args.extend(extra.iter().cloned());
    Ok(args)
}
pub(super) fn execute(root: &Path, args: &[String]) -> Result<Value, String> {
    let client = args.first().ok_or("native requires codex or claude")?;
    let (_, config, bundle) = installed(root)?;
    if !strings(&config, "clients")?.contains(client) {
        return Err("native client is not selected in project configuration".into());
    }
    let mut launch = false;
    let mut verify = false;
    let mut passthrough = false;
    let mut extra = Vec::new();
    for arg in args.iter().skip(1) {
        if passthrough {
            extra.push(arg.clone());
            continue;
        }
        match arg.as_str() {
            "--" => passthrough = true,
            "--launch" => launch = true,
            "--verify" => verify = true,
            _ => extra.push(arg.clone()),
        }
    }
    if launch && verify {
        return Err("select either --launch or --verify".into());
    }
    if verify {
        if client != "codex" || !extra.is_empty() {
            return Err("native --verify supports only codex without extra arguments".into());
        }
        return activate_codex(&bundle, root, Path::new("codex"), 30.0);
    }
    let argv = native_argv(&bundle, client, &extra)?;
    if launch {
        if client == "codex" {
            activate_codex(&bundle, root, Path::new("codex"), 30.0)?;
        }
        let status = Command::new(argv.first().ok_or("missing native program")?)
            .args(argv.iter().skip(1))
            .current_dir(root)
            .status()
            .map_err(|e| e.to_string())?;
        return Ok(
            json!({"ok":status.success(),"exit_code":status.code().unwrap_or(1).clamp(0,255),"client":client,"user_configuration_writes":false}),
        );
    }
    Ok(
        json!({"ok":true,"argv":argv,"cwd":root,"user_configuration_writes":false,"claim":"command construction only; native installation and behavior not observed"}),
    )
}

#[cfg(unix)]
fn plugins(result: &Value, market: &str) -> Result<BTreeMap<String, Value>, String> {
    let entries = result
        .get("marketplaces")
        .and_then(Value::as_array)
        .ok_or("Codex plugin catalog is missing marketplaces")?;
    let entries = entries
        .iter()
        .filter(|entry| entry.get("name").and_then(Value::as_str) == Some(market))
        .collect::<Vec<_>>();
    if entries.len() != 1 {
        return Err("Codex did not discover exactly one pinned marketplace".into());
    }
    let rows = entries
        .first()
        .and_then(|v| v.get("plugins"))
        .and_then(Value::as_array)
        .ok_or("Codex marketplace lacks plugins")?;
    let mut out = BTreeMap::new();
    for plugin in rows {
        let id = plugin
            .get("id")
            .and_then(Value::as_str)
            .ok_or("invalid native plugin identity")?;
        if out.insert(id.to_owned(), plugin.clone()).is_some() {
            return Err("duplicate native plugin identity".into());
        }
    }
    Ok(out)
}
#[cfg(unix)]
enum DiscoveryError {
    Pending(String),
    Invalid(String),
}
#[cfg(unix)]
impl From<String> for DiscoveryError {
    fn from(value: String) -> Self {
        Self::Invalid(value)
    }
}
#[cfg(unix)]
fn verify_skills(
    result: &Value,
    root: &Path,
    bundle: &InstructionBundle,
    cache: &Path,
    market: &str,
) -> Result<Value, DiscoveryError> {
    let invalid = |message: &str| DiscoveryError::Invalid(message.into());
    let rows = result
        .get("data")
        .and_then(Value::as_array)
        .ok_or_else(|| invalid("Codex skill catalog is missing data"))?;
    let matching = rows
        .iter()
        .filter(|row| {
            row.get("cwd")
                .and_then(Value::as_str)
                .is_some_and(|cwd| Path::new(cwd).canonicalize().is_ok_and(|cwd| cwd == root))
        })
        .collect::<Vec<_>>();
    if matching.len() != 1 {
        return Err(invalid(
            "Codex did not return exactly one requested project skill catalog",
        ));
    }
    let skills = matching
        .first()
        .and_then(|row| row.get("skills"))
        .and_then(Value::as_array)
        .ok_or_else(|| invalid("Codex skill list is missing"))?;
    let expected = bundle
        .selected
        .iter()
        .map(|package| (format!("{package}@{market}"), package))
        .collect::<BTreeMap<_, _>>();
    let known = bundle
        .catalog
        .get("packages")
        .and_then(Value::as_object)
        .ok_or_else(|| invalid("invalid bundle catalog"))?;
    let mut found = BTreeMap::new();
    let mut roots = BTreeMap::<String, PathBuf>::new();
    for skill in skills {
        if skill.get("enabled") != Some(&Value::Bool(true)) {
            continue;
        }
        let Some(id) = skill.get("pluginId").and_then(Value::as_str) else {
            continue;
        };
        let package = id
            .split('@')
            .next()
            .ok_or_else(|| invalid("invalid plugin id"))?;
        if known.contains_key(package) && !expected.contains_key(id) {
            return Err(invalid(
                "another GameSkills pin or unselected package is enabled",
            ));
        }
        if !expected.contains_key(id) {
            continue;
        }
        let native_name = skill
            .get("name")
            .and_then(Value::as_str)
            .ok_or_else(|| invalid("invalid native skill name"))?;
        let prefix = format!("{package}:");
        let name = native_name.strip_prefix(&prefix).unwrap_or(native_name);
        let names = strings(
            known
                .get(package)
                .ok_or_else(|| invalid("unexpected package"))?,
            "skills",
        )?;
        if !names.iter().any(|n| n == name) {
            return Err(invalid("unexpected native skill"));
        }
        let path = Path::new(
            skill
                .get("path")
                .and_then(Value::as_str)
                .ok_or_else(|| invalid("invalid native skill path"))?,
        );
        if !path.is_absolute() || !path.ends_with(Path::new("skills").join(name).join("SKILL.md")) {
            return Err(invalid("invalid native skill path"));
        }
        let actual = path
            .parent()
            .and_then(Path::parent)
            .and_then(Path::parent)
            .ok_or_else(|| invalid("invalid package path"))?
            .canonicalize()
            .map_err(|e| e.to_string())?;
        let cache_package = cache.join(market).join(package);
        if !actual.starts_with(&cache_package) {
            return Err(invalid("Codex skill is outside the pinned native cache"));
        }
        if roots
            .get(package)
            .is_some_and(|previous| previous != &actual)
        {
            return Err(invalid("inconsistent native cache roots"));
        }
        roots.insert(package.to_owned(), actual);
        let key = (package.to_owned(), name.to_owned());
        if found.insert(key, json!({"package":package,"name":name,"native_name":native_name,"plugin_id":id,"path":path})).is_some() { return Err(invalid("duplicate native skill")); }
    }
    let mut wanted = BTreeSet::new();
    for package in &bundle.selected {
        for name in strings(
            known
                .get(package)
                .ok_or_else(|| invalid("missing catalog package"))?,
            "skills",
        )? {
            wanted.insert((package.clone(), name));
        }
    }
    if found.keys().cloned().collect::<BTreeSet<_>>() != wanted {
        return Err(DiscoveryError::Pending(
            "native GameSkills discovery is incomplete".into(),
        ));
    }
    for (package, actual) in roots {
        let actual = files::Directory::open(&actual)?
            .tree()?
            .iter()
            .map(|(name, bytes)| (name.clone(), super::hash(bytes)))
            .collect::<BTreeMap<_, _>>();
        if actual != package_files(bundle, &package) {
            return Err(invalid("native cache content differs from pinned bundle"));
        }
    }
    Ok(Value::Array(found.into_values().collect()))
}

/// Observe native Codex installation and discovery within a single deadline.
///
/// This may populate Codex's native cache, but never sends model turns or writes
/// user configuration. POSIX process-group supervision is required.
pub fn activate_codex(
    bundle: &Path,
    root: &Path,
    executable: &Path,
    timeout_seconds: f64,
) -> Result<Value, String> {
    if !timeout_seconds.is_finite() || timeout_seconds <= 0.0 || timeout_seconds > 120.0 {
        return Err("native timeout must be greater than zero and at most 120 seconds".into());
    }
    #[cfg(not(unix))]
    {
        let _ = (bundle, root, executable);
        Err("native verification requires the tested POSIX process-group backend; Windows native verification is unsupported".into())
    }
    #[cfg(unix)]
    {
        posix::activate(bundle, root, executable, timeout_seconds)
    }
}

#[cfg(unix)]
mod posix {
    use super::*;
    use nix::{
        sys::signal::{killpg, Signal},
        unistd::Pid,
    };
    use std::{
        io::{Read, Write},
        os::unix::process::CommandExt,
        process::{Child, ChildStdin, Stdio},
        sync::{
            atomic::{AtomicBool, Ordering},
            mpsc, Arc,
        },
        thread,
        time::{Duration, Instant},
    };
    struct Rpc {
        child: Child,
        input: Option<ChildStdin>,
        messages: mpsc::Receiver<Result<Value, String>>,
        reader: Option<thread::JoinHandle<()>>,
        deadline: Instant,
        next: u64,
        stopped: Arc<AtomicBool>,
    }
    impl Drop for Rpc {
        fn drop(&mut self) {
            self.stopped.store(true, Ordering::Relaxed);
            self.input.take();
            if let Ok(id) = i32::try_from(self.child.id()) {
                let _ = killpg(Pid::from_raw(id), Signal::SIGTERM);
                let limit = Instant::now() + Duration::from_millis(250);
                while Instant::now() < limit {
                    if self.child.try_wait().ok().flatten().is_some() {
                        break;
                    }
                    thread::sleep(Duration::from_millis(10));
                }
                let _ = killpg(Pid::from_raw(id), Signal::SIGKILL);
            }
            let _ = self.child.kill();
            let _ = self.child.wait();
            if let Some(reader) = self.reader.take() {
                let _ = reader.join();
            }
        }
    }
    impl Rpc {
        fn send(&mut self, value: &Value) -> Result<(), String> {
            let mut bytes = serde_json::to_vec(value).map_err(|e| e.to_string())?;
            bytes.push(b'\n');
            let mut offset = 0;
            let mut backpressured = false;
            while offset < bytes.len() {
                let remaining = self
                    .deadline
                    .checked_duration_since(Instant::now())
                    .filter(|remaining| !remaining.is_zero())
                    .ok_or_else(|| if backpressured {
                        "Codex native activation timed out while sending request: stdin backpressure".to_owned()
                    } else {
                        "Codex native activation timed out while sending request".to_owned()
                    })?;
                match self.input.as_mut().ok_or("Codex input closed")?.write(
                    bytes
                        .get(offset..)
                        .expect("write progress stays inside request"),
                ) {
                    Ok(0) => return Err("Codex input closed before request was written".into()),
                    Ok(written) => offset += written,
                    Err(error) if error.kind() == std::io::ErrorKind::WouldBlock => {
                        backpressured = true;
                        thread::sleep(remaining.min(Duration::from_millis(5)));
                    }
                    Err(error) if error.kind() == std::io::ErrorKind::Interrupted => {}
                    Err(error) => return Err(format!("Codex input closed: {error}")),
                }
            }
            Ok(())
        }
        fn call(&mut self, method: &str, params: Value) -> Result<Value, String> {
            let id = self.next;
            self.next += 1;
            self.send(&json!({"id":id,"method":method,"params":params}))?;
            loop {
                let remaining = self
                    .deadline
                    .checked_duration_since(Instant::now())
                    .ok_or_else(|| format!("Codex native activation timed out during {method}"))?;
                let message = self.messages.recv_timeout(remaining).map_err(|_| {
                    format!("Codex native activation timed out or exited during {method}")
                })??;
                if message.get("method").is_some() {
                    if message.get("id").is_some() {
                        return Err("Codex requested unexpected interactive action".into());
                    }
                    continue;
                }
                if message.get("id").and_then(Value::as_u64) != Some(id) {
                    return Err("Codex returned mismatched response id".into());
                }
                if let Some(error) = message.get("error") {
                    return Err(format!("Codex {method} failed: {error}"));
                }
                let result = message
                    .get("result")
                    .filter(|v| v.is_object())
                    .ok_or("Codex returned invalid result")?;
                return Ok(result.clone());
            }
        }
    }
    fn start_rpc(
        executable: &Path,
        args: &[String],
        root: &Path,
        timeout: f64,
    ) -> Result<Rpc, String> {
        let errors = tempfile::tempfile().map_err(|e| e.to_string())?;
        let mut child = Command::new(executable)
            .args(args.iter().skip(1))
            .current_dir(root)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(errors)
            .process_group(0)
            .spawn()
            .map_err(|e| format!("cannot start Codex native activation: {e}"))?;
        let input = child.stdin.take();
        let mut output = child.stdout.take().ok_or("Codex output unavailable")?;
        let (sender, receiver) = mpsc::sync_channel(16);
        // Both directions share one deadline. A peer may stop draining stdin
        // while continuing to send responses, and a descendant may retain stdout.
        let nonblocking = (|| {
            let stdin = input.as_ref().ok_or(nix::errno::Errno::EBADF)?;
            let input_flags = nix::fcntl::fcntl(stdin, nix::fcntl::FcntlArg::F_GETFL)?;
            nix::fcntl::fcntl(
                stdin,
                nix::fcntl::FcntlArg::F_SETFL(
                    nix::fcntl::OFlag::from_bits_truncate(input_flags)
                        | nix::fcntl::OFlag::O_NONBLOCK,
                ),
            )?;
            let flags = nix::fcntl::fcntl(&output, nix::fcntl::FcntlArg::F_GETFL)?;
            nix::fcntl::fcntl(
                &output,
                nix::fcntl::FcntlArg::F_SETFL(
                    nix::fcntl::OFlag::from_bits_truncate(flags) | nix::fcntl::OFlag::O_NONBLOCK,
                ),
            )?;
            Ok::<(), nix::errno::Errno>(())
        })();
        if let Err(error) = nonblocking {
            if let Ok(id) = i32::try_from(child.id()) {
                let _ = killpg(Pid::from_raw(id), Signal::SIGKILL);
            }
            let _ = child.kill();
            let _ = child.wait();
            return Err(error.to_string());
        }
        let deadline = Instant::now() + Duration::from_secs_f64(timeout);
        let stopped = Arc::new(AtomicBool::new(false));
        let reader_stopped = stopped.clone();
        let reader = thread::spawn(move || {
            let mut pending = Vec::new();
            let mut bytes = [0_u8; 8192];
            while !reader_stopped.load(Ordering::Relaxed)
                && Instant::now() <= deadline + Duration::from_millis(300)
            {
                match output.read(&mut bytes) {
                    Ok(0) => {
                        let _ = sender
                            .try_send(Err("Codex app-server exited before responding".into()));
                        return;
                    }
                    Ok(count) => {
                        pending.extend_from_slice(bytes.get(..count).expect("read length"));
                        while let Some(end) = pending.iter().position(|byte| *byte == b'\n') {
                            if end > 4 * 1024 * 1024 {
                                let _ = sender.try_send(Err(
                                    "Codex response exceeds bounded message size".into(),
                                ));
                                return;
                            }
                            let line = pending.drain(..=end).collect::<Vec<_>>();
                            let value = archive::json(&line).and_then(|value| {
                                if value.is_object() {
                                    Ok(value)
                                } else {
                                    Err("Codex returned a non-object response".into())
                                }
                            });
                            if sender.try_send(value).is_err() {
                                return;
                            }
                        }
                        if pending.len() > 4 * 1024 * 1024 {
                            let _ = sender.try_send(Err(
                                "Codex response exceeds bounded message size".into(),
                            ));
                            return;
                        }
                    }
                    Err(error) if error.kind() == std::io::ErrorKind::WouldBlock => {
                        thread::sleep(Duration::from_millis(5))
                    }
                    Err(error) => {
                        let _ = sender.try_send(Err(format!("invalid Codex response: {error}")));
                        return;
                    }
                }
            }
        });
        Ok(Rpc {
            child,
            input,
            messages: receiver,
            reader: Some(reader),
            deadline,
            next: 0,
            stopped,
        })
    }

    pub(super) fn activate(
        bundle_path: &Path,
        root: &Path,
        executable: &Path,
        timeout: f64,
    ) -> Result<Value, String> {
        let bundle = archive::source(bundle_path)?;
        let bundle_path = bundle_path.canonicalize().map_err(|e| e.to_string())?;
        let root = root.canonicalize().map_err(|e| e.to_string())?;
        let market = archive::marketplace(&bundle);
        let args = native_argv(
            &bundle_path,
            "codex",
            &["app-server".into(), "--stdio".into()],
        )?;
        let mut rpc = start_rpc(executable, &args, &root, timeout)?;
        let deadline = rpc.deadline;
        let initialized = rpc.call("initialize", json!({"clientInfo":{"name":"gameskills","version":env!("CARGO_PKG_VERSION")},"capabilities":{"experimentalApi":true}}))?;
        let home = Path::new(
            initialized
                .get("codexHome")
                .and_then(Value::as_str)
                .ok_or("Codex initialization did not identify native cache home")?,
        );
        if !home.is_absolute() {
            return Err("Codex native home must be absolute".into());
        }
        let cache = home
            .canonicalize()
            .map_err(|e| e.to_string())?
            .join("plugins/cache");
        rpc.send(&json!({"method":"initialized","params":{}}))?;
        let params = json!({"cwds":[root],"forceRefetch":false,"marketplaceKinds":["local"]});
        let initial = plugins(&rpc.call("plugin/list", params.clone())?, &market)?;
        let expected = bundle
            .selected
            .iter()
            .map(|p| format!("{p}@{market}"))
            .collect::<Vec<_>>();
        if expected.iter().any(|id| !initial.contains_key(id)) {
            return Err("Codex plugin catalog is missing selected packages".into());
        }
        loop {
            let result = verify_skills(
                &rpc.call("skills/list", json!({"cwds":[root],"forceReload":true}))?,
                &root,
                &bundle,
                &cache,
                &market,
            );
            let pending = match result {
                Ok(skills) => {
                    let final_plugins =
                        plugins(&rpc.call("plugin/list", params.clone())?, &market)?;
                    if expected.iter().any(|id| {
                        final_plugins.get(id).and_then(|v| v.get("enabled"))
                            != Some(&Value::Bool(true))
                    }) {
                        return Err("Codex did not enable selected plugins".into());
                    }
                    if expected.iter().all(|id| {
                        final_plugins.get(id).and_then(|v| v.get("installed"))
                            == Some(&Value::Bool(true))
                    }) {
                        return Ok(
                            json!({"ok":true,"client":"codex","client_identity":initialized.get("userAgent"),"content_sha256":bundle.identity(),"marketplace":market,"packages":bundle.selected,"skills":skills,"installed_plugin_ids":expected,"claim":"native installation and skill discovery observed; model invocation and behavior not tested"}),
                        );
                    }
                    "Codex has not confirmed selected plugins are installed".to_owned()
                }
                Err(DiscoveryError::Invalid(error)) => return Err(error),
                Err(DiscoveryError::Pending(error)) => error,
            };
            let remaining = deadline
                .checked_duration_since(Instant::now())
                .ok_or_else(|| format!("Codex native activation timed out: {pending}"))?;
            thread::sleep(remaining.min(Duration::from_millis(100)));
        }
    }

    #[cfg(test)]
    mod tests {
        use super::*;

        #[test]
        fn full_stdin_pipe_observes_write_deadline_and_reaps_actual_peer(
        ) -> Result<(), Box<dyn std::error::Error>> {
            let directory = tempfile::tempdir()?;
            let executable = std::env::current_exe()?
                .parent()
                .and_then(Path::parent)
                .ok_or("test profile directory")?
                .join("examples")
                .join(format!("native_probe{}", std::env::consts::EXE_SUFFIX));
            if !executable.is_file() {
                return Err(
                    format!("compiled native probe missing: {}", executable.display()).into(),
                );
            }
            let cwd = directory.path().to_owned();
            let (pid_sender, pid_receiver) = mpsc::sync_channel(1);
            let (result_sender, result_receiver) = mpsc::sync_channel(1);
            let started = Instant::now();
            let worker = thread::spawn(move || {
                let result = (|| -> Result<String, String> {
                    // Use the same process setup as activation, including its
                    // nonblocking descriptor flags and cleanup ownership.
                    let mut rpc = start_rpc(
                        &executable,
                        &["native_probe".into(), "--hold-stdout".into()],
                        &cwd,
                        1.5,
                    )?;
                    pid_sender
                        .send(rpc.child.id())
                        .map_err(|error| error.to_string())?;
                    // One request is larger than the host's ordinary pipe
                    // capacity. The peer never reads, so this cannot merely
                    // time out between small discovery retries.
                    let result = rpc.send(&json!({"id":0,"method":"fixture/full-pipe","params":{"bytes":"x".repeat(4 * 1024 * 1024)}}));
                    let error =
                        result.expect_err("an undrained pipe cannot accept the full request");
                    drop(rpc);
                    Ok(error)
                })();
                let _ = result_sender.send(result);
            });
            let pid = pid_receiver.recv_timeout(Duration::from_secs(5))?;
            let observed = result_receiver.recv_timeout(Duration::from_secs(5));
            // An old blocking write must fail this test without stranding its
            // worker or Cargo. The watchdog owns only the actual spawned group.
            if observed.is_err() {
                let _ = killpg(Pid::from_raw(i32::try_from(pid)?), Signal::SIGKILL);
            }
            worker.join().expect("native write worker");
            let error =
                observed.map_err(|error| format!("native writer exceeded watchdog: {error}"))??;
            assert!(
                error.contains("timed out while sending request: stdin backpressure"),
                "{error}"
            );
            assert!(started.elapsed() < Duration::from_secs(4));
            assert!(
                nix::sys::signal::kill(Pid::from_raw(i32::try_from(pid)?), None).is_err(),
                "native peer was not reaped"
            );
            Ok(())
        }
    }
}
