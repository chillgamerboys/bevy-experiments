//! Small OS boundaries for read-only foundation commands.
//!
//! Process supervision, lock ownership and atomic mutation are later migration work.

use std::ffi::OsString;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};

/// Capture a bounded provider command without shell interpolation or pipe deadlock.
/// Errors deliberately omit provider output, which can contain private records.
pub fn provider_output(command: &mut Command, input: Option<&[u8]>) -> Result<Vec<u8>, String> {
    use std::io::{Read, Seek, Write};
    use std::time::{Duration, Instant};
    let mut output = tempfile::tempfile().map_err(|e| e.to_string())?;
    let error = tempfile::tempfile().map_err(|e| e.to_string())?;
    let mut stdin = tempfile::tempfile().map_err(|e| e.to_string())?;
    stdin
        .write_all(input.unwrap_or_default())
        .map_err(|e| e.to_string())?;
    stdin.rewind().map_err(|e| e.to_string())?;
    command
        .stdin(stdin)
        .stdout(output.try_clone().map_err(|e| e.to_string())?)
        .stderr(error.try_clone().map_err(|e| e.to_string())?);
    #[cfg(unix)]
    {
        use std::os::unix::process::CommandExt;
        command.process_group(0);
    }
    let mut child = command
        .spawn()
        .map_err(|e| format!("provider could not start: {e}"))?;
    let start = Instant::now();
    let observed = (|| -> Result<_, String> {
        Ok(loop {
            if let Some(status) = child.try_wait().map_err(|e| e.to_string())? {
                break Some(status);
            }
            if start.elapsed() > Duration::from_secs(45)
                || output.metadata().map_err(|e| e.to_string())?.len() > 16 * 1024 * 1024
                || error.metadata().map_err(|e| e.to_string())?.len() > 16 * 1024 * 1024
            {
                break None;
            }
            std::thread::sleep(Duration::from_millis(20));
        })
    })();
    let status = observed.as_ref().ok().and_then(|s| *s);
    #[cfg(unix)]
    if let Ok(pid) = i32::try_from(child.id()) {
        let _ = nix::sys::signal::killpg(
            nix::unistd::Pid::from_raw(pid),
            nix::sys::signal::Signal::SIGKILL,
        );
    }
    if status.is_none() {
        let _ = child.kill();
    }
    let _ = child.wait();
    let status =
        observed?.ok_or("provider exceeded time or output limit; result is unverifiable")?;
    if !status.success() {
        return Err(format!(
            "provider exited {}; result is unverifiable",
            status.code().unwrap_or(-1)
        ));
    }
    output.rewind().map_err(|e| e.to_string())?;
    let mut bytes = Vec::new();
    output
        .take(16 * 1024 * 1024 + 1)
        .read_to_end(&mut bytes)
        .map_err(|e| e.to_string())?;
    if bytes.len() > 16 * 1024 * 1024 {
        return Err("provider output exceeds limit".into());
    }
    Ok(bytes)
}

/// Read an ordinary UTF-8 file, refusing a symlink or special file at this path.
///
/// This read-only helper is not a race-safe filesystem capability for future writes.
pub fn read_ordinary_file(path: &Path) -> io::Result<String> {
    let metadata = fs::symlink_metadata(path)?;
    if !metadata.file_type().is_file() {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "expected an ordinary file",
        ));
    }
    fs::read_to_string(path)
}

/// An argument vector and working directory for a short-lived process.
#[derive(Debug)]
pub struct ProcessSpec {
    /// Executable path or name, resolved by the OS.
    pub program: OsString,
    /// Literal arguments; no shell parses these values.
    pub args: Vec<OsString>,
    /// Explicit process working directory.
    pub cwd: PathBuf,
}

impl ProcessSpec {
    /// Capture a short-lived process and wait for it.
    ///
    /// There is no timeout, descendant cleanup or resource-lock guarantee here.
    /// Never use this helper as the later command runner or native-client supervisor.
    pub fn output(&self) -> io::Result<Output> {
        Command::new(&self.program)
            .args(&self.args)
            .current_dir(&self.cwd)
            .output()
    }
}
