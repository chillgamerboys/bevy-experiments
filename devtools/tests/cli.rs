//! Executable validation output, exit status and independent checkout behavior.

use serde_json::Value;
use std::path::Path;
use std::process::Command;

fn run(root: &Path, args: &[&str]) -> (i32, Value) {
    let output = Command::new(env!("CARGO_BIN_EXE_repo-devtools"))
        .arg("--root")
        .arg(root)
        .args(args)
        .current_dir(std::env::temp_dir())
        .output()
        .expect("run validator");
    assert!(
        output.stderr.is_empty(),
        "unexpected stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let value: Value = serde_json::from_slice(&output.stdout).expect("one JSON result");
    assert_eq!(value.get("schema_version"), Some(&Value::from(1)));
    (output.status.code().expect("exit status"), value)
}

#[test]
fn repository_check_works_outside_git_and_rejection_is_read_only() {
    let temporary = tempfile::tempdir().expect("fixture");
    let root = temporary.path().join("workspace with spaces");
    std::fs::create_dir(&root).expect("fixture directory");
    std::fs::write(root.join("Cargo.toml"), "[workspace]\nmembers = []\n")
        .expect("fixture manifest");
    let (status, result) = run(&root, &["check"]);
    assert_eq!(status, 0);
    assert_eq!(result.get("ok"), Some(&Value::Bool(true)));
    let content = "[broken](missing.md)";
    std::fs::write(root.join("README.md"), content).expect("fixture README");
    let (status, result) = run(&root, &["check"]);
    assert_eq!(status, 1);
    assert_eq!(result.get("ok"), Some(&Value::Bool(false)));
    assert_eq!(
        std::fs::read_to_string(root.join("README.md")).expect("preserved README"),
        content
    );
    assert_eq!(
        std::fs::read_dir(&root).expect("fixture directory").count(),
        2
    );
}

#[test]
fn catalog_json_preserves_counts_and_structural_limitations_on_failure() {
    let temporary = tempfile::tempdir().expect("fixture");
    for command in [
        vec!["skills", "validate"],
        vec!["skills", "validate", "--json"],
    ] {
        let (status, result) = run(temporary.path(), &command);
        assert_eq!(status, 1);
        for (field, count) in [
            ("packages", 6),
            ("skills", 21),
            ("core_skills", 12),
            ("optional_skills", 9),
        ] {
            assert_eq!(result.get(field), Some(&Value::from(count)), "{field}");
        }
        assert_eq!(result.get("structural_only"), Some(&Value::Bool(true)));
        assert!(result
            .get("notice")
            .and_then(Value::as_str)
            .is_some_and(|notice| notice.contains("do not prove native installation")));
        assert!(result
            .get("failures")
            .and_then(Value::as_array)
            .is_some_and(|failures| !failures.is_empty()));
    }
    assert_eq!(
        std::fs::read_dir(temporary.path())
            .expect("fixture directory")
            .count(),
        0
    );
}

#[test]
fn malformed_arguments_and_unavailable_ci_inputs_fail_explicitly() {
    let temporary = tempfile::tempdir().expect("fixture");
    for command in [
        vec!["skills"],
        vec!["distribution", "check", "--case", "unknown"],
        vec!["ci", "select"],
        vec!["check", "--unknown"],
    ] {
        let (status, result) = run(temporary.path(), &command);
        assert_eq!(status, 2);
        assert_eq!(result.get("ok"), Some(&Value::Bool(false)));
        assert!(result
            .get("error")
            .and_then(|error| error.get("code"))
            .and_then(Value::as_str)
            .is_some());
    }
}

#[test]
fn unavailable_distribution_and_legacy_inputs_fail_without_writes() {
    let temporary = tempfile::tempdir().expect("fixture");
    for command in [
        vec!["distribution", "check", "--case", "empty"],
        vec!["skills", "legacy"],
    ] {
        let (status, result) = run(temporary.path(), &command);
        assert_eq!(status, 1);
        assert_eq!(result.get("ok"), Some(&Value::Bool(false)));
    }
    assert_eq!(
        std::fs::read_dir(temporary.path())
            .expect("fixture directory")
            .count(),
        0
    );
}
