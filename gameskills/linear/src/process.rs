//! Small private subprocess boundary for this independently packaged optional binary.
//! Mirrors core's provider bounds without pulling the core instruction archive into
//! a standalone network helper. Keep transport regressions on both consumers.
use std::process::Command;
/// Capture a bounded provider command without shell interpolation or pipe deadlock.
/// Errors deliberately omit provider output, which can contain private records.
pub(crate) fn provider_output(
    command: &mut Command,
    input: Option<&[u8]>,
) -> Result<Vec<u8>, String> {
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
