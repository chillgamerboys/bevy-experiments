//! Exercise actual binary status, JSON boundaries and read-only behavior.

use serde_json::Value;
use std::error::Error;
use std::fs;
use std::process::Command;

#[test]
fn fixture_commands_observe_exit_json_and_no_writes() -> Result<(), Box<dyn Error>> {
    let fixture: Value = serde_json::from_str(include_str!("fixtures/configuration.json"))?;
    for case in fixture
        .get("cases")
        .and_then(Value::as_array)
        .ok_or("cases missing")?
    {
        let directory = tempfile::tempdir()?;
        let input = case
            .get("input")
            .and_then(Value::as_str)
            .ok_or("input missing")?;
        fs::write(directory.path().join("gameskills.toml"), input)?;
        let args: Vec<_> = case
            .get("argv")
            .and_then(Value::as_array)
            .ok_or("argv missing")?
            .iter()
            .map(|item| item.as_str().ok_or("argument not string"))
            .collect::<Result<_, _>>()?;
        let output = Command::new(env!("CARGO_BIN_EXE_gameskills"))
            .args(args)
            .current_dir(directory.path())
            .output()?;
        let actual: Value = serde_json::from_slice(&output.stdout)?;
        let expected = case.get("expected").ok_or("expected missing")?;
        assert_eq!(
            output.status.code().map(i64::from),
            expected.get("exit_code").and_then(Value::as_i64),
            "{case:?}"
        );
        assert_eq!(actual.get("ok"), expected.get("ok"));
        assert_eq!(actual.get("schema_version"), Some(&Value::from(1)));
        if expected.get("ok") == Some(&Value::Bool(true)) {
            assert_eq!(actual.get("configuration"), expected.get("configuration"));
            assert_eq!(
                actual.get("scope").and_then(Value::as_str),
                Some("configuration_structure")
            );
        }
        assert!(output.stderr.is_empty());
        assert_eq!(
            fs::read_to_string(directory.path().join("gameskills.toml"))?,
            input
        );
        assert_eq!(fs::read_dir(directory.path())?.count(), 1);
    }
    Ok(())
}

#[test]
fn help_and_version_work_without_a_repository() -> Result<(), Box<dyn Error>> {
    let directory = tempfile::tempdir()?;
    for flag in ["--help", "--version"] {
        let output = Command::new(env!("CARGO_BIN_EXE_gameskills"))
            .arg(flag)
            .current_dir(directory.path())
            .output()?;
        assert!(output.status.success());
        assert!(String::from_utf8(output.stdout)?.contains("gameskills"));
    }
    assert_eq!(fs::read_dir(directory.path())?.count(), 0);
    Ok(())
}

#[test]
fn repository_commands_refuse_unconfigured_roots_without_creating_state(
) -> Result<(), Box<dyn Error>> {
    let directory = tempfile::tempdir()?;
    for command in [
        "config", "status", "bundle", "native", "plan", "queue", "run", "evidence", "legacy",
    ] {
        let output = Command::new(env!("CARGO_BIN_EXE_gameskills"))
            .arg(command)
            .current_dir(directory.path())
            .output()?;
        let result: Value = serde_json::from_slice(&output.stdout)?;
        assert_eq!(output.status.code(), Some(2));
        assert_eq!(
            result.pointer("/error/code").and_then(Value::as_str),
            Some("operation_failed"),
            "{command}"
        );
    }
    assert_eq!(fs::read_dir(directory.path())?.count(), 0);
    Ok(())
}

#[test]
fn parse_errors_missing_files_and_root_paths_are_explicit() -> Result<(), Box<dyn Error>> {
    let directory = tempfile::tempdir()?;
    for args in [
        vec!["--unknown"],
        vec!["config", "validate", "--file"],
        vec!["bogus"],
    ] {
        let output = Command::new(env!("CARGO_BIN_EXE_gameskills"))
            .args(args)
            .current_dir(directory.path())
            .output()?;
        let result: Value = serde_json::from_slice(&output.stdout)?;
        assert_eq!(output.status.code(), Some(2));
        assert_eq!(
            result.pointer("/error/code").and_then(Value::as_str),
            Some("invalid_arguments")
        );
    }
    let output = Command::new(env!("CARGO_BIN_EXE_gameskills"))
        .args(["config", "validate"])
        .current_dir(directory.path())
        .output()?;
    let result: Value = serde_json::from_slice(&output.stdout)?;
    assert_eq!(
        result.pointer("/error/code").and_then(Value::as_str),
        Some("configuration_io")
    );
    assert!(!output.status.success());
    let configured = directory.path().join("root with spaces");
    fs::create_dir(&configured)?;
    fs::write(configured.join("gameskills.toml"), "schema_version=1\n")?;
    let output = Command::new(env!("CARGO_BIN_EXE_gameskills"))
        .arg("--root")
        .arg(&configured)
        .args(["config", "validate"])
        .current_dir(directory.path())
        .output()?;
    assert!(output.status.success());
    Ok(())
}

#[test]
fn ordinary_file_boundary_refuses_directories() -> Result<(), Box<dyn Error>> {
    let directory = tempfile::tempdir()?;
    assert!(gameskills_cli::platform::read_ordinary_file(directory.path()).is_err());
    Ok(())
}

#[cfg(unix)]
#[test]
fn ordinary_file_boundary_refuses_symlinks() -> Result<(), Box<dyn Error>> {
    let directory = tempfile::tempdir()?;
    let source = directory.path().join("source");
    fs::write(&source, "schema_version=1\n")?;
    let alias = directory.path().join("alias");
    std::os::unix::fs::symlink(&source, &alias)?;
    assert!(gameskills_cli::platform::read_ordinary_file(&alias).is_err());
    Ok(())
}

#[test]
fn relative_root_native_paths_remain_absolute_after_child_cwd_change() -> Result<(), Box<dyn Error>>
{
    let parent = tempfile::tempdir()?;
    let root = parent.path().join("adopter with spaces");
    fs::create_dir(&root)?;
    for tail in [
        vec!["setup", "--apply"],
        vec!["native", "claude"],
        vec!["native", "codex"],
    ] {
        let output = Command::new(env!("CARGO_BIN_EXE_gameskills"))
            .args(["--root", "adopter with spaces"])
            .args(&tail)
            .current_dir(parent.path())
            .output()?;
        assert!(
            output.status.success(),
            "{}",
            String::from_utf8_lossy(&output.stdout)
        );
        let value: Value = serde_json::from_slice(&output.stdout)?;
        if tail.first() == Some(&"native") {
            assert_eq!(
                value
                    .get("cwd")
                    .and_then(Value::as_str)
                    .map(std::path::Path::new),
                Some(root.canonicalize()?.as_path())
            );
            let argv = value.get("argv").and_then(Value::as_array).ok_or("argv")?;
            if tail.last() == Some(&"claude") {
                let path = argv.get(2).and_then(Value::as_str).ok_or("plugin path")?;
                assert!(std::path::Path::new(path).is_absolute());
                assert!(std::path::Path::new(path).is_dir());
            }
        }
    }
    Ok(())
}
