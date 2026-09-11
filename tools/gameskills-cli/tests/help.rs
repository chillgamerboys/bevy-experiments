//! An installed executable must explain its operational flags before setup.

use std::{error::Error, process::Command};

#[test]
fn operational_and_nested_help_need_no_installation() -> Result<(), Box<dyn Error>> {
    let temporary = tempfile::tempdir()?;
    let missing = temporary.path().join("not a project");
    for (args, flag) in [
        (vec!["setup", "--help"], "--packages"),
        (vec!["run", "--help"], "--max-workers"),
        (vec!["native", "codex", "--help"], "--verify"),
        (vec!["plan", "validate", "--help"], "--file"),
        (vec!["queue", "start", "--help"], "--expected-revision"),
        (vec!["evidence", "validate", "--help"], "RUN_ID"),
    ] {
        let output = Command::new(env!("CARGO_BIN_EXE_gameskills"))
            .arg("--root")
            .arg(&missing)
            .args(args)
            .output()?;
        assert!(output.status.success(), "{:?}", output);
        assert!(String::from_utf8(output.stdout)?.contains(flag));
    }
    assert!(!missing.exists());
    Ok(())
}

#[test]
fn native_passthrough_help_remains_a_client_argument() -> Result<(), Box<dyn Error>> {
    let temporary = tempfile::tempdir()?;
    let invoke = |args: &[&str]| {
        Command::new(env!("CARGO_BIN_EXE_gameskills"))
            .current_dir(temporary.path())
            .args(args)
            .output()
    };
    assert!(invoke(&["setup", "--apply"])?.status.success());
    let output = invoke(&["native", "claude", "--", "--help"])?;
    assert!(output.status.success(), "{:?}", output);
    let value: serde_json::Value = serde_json::from_slice(&output.stdout)?;
    assert_eq!(
        value
            .get("argv")
            .and_then(serde_json::Value::as_array)
            .and_then(|argv| argv.last())
            .and_then(serde_json::Value::as_str),
        Some("--help")
    );
    Ok(())
}
