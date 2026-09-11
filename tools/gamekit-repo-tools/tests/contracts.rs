//! Mutation tests ensure that migration accounting cannot silently omit source behavior.

use serde_json::Value;
use std::error::Error;

// Build synthetic decisions around the immutable source inventory. The real ledger
// is checked by the command in CI; do not maintain a second authored copy here.
fn valid() -> Result<String, Box<dyn Error>> {
    #[derive(serde::Deserialize)]
    struct Snapshot {
        reference_commit: String,
        files: Vec<File>,
    }
    #[derive(serde::Deserialize)]
    struct File {
        path: String,
        tests: Vec<String>,
    }
    let snapshot: Snapshot = serde_json::from_str(include_str!("fixtures/legacy-inventory.json"))?;
    let mut files = Vec::new();
    let mut tests = Vec::new();
    for file in snapshot.files {
        files.push(serde_json::json!({"source": file.path, "destination": "tools/fixture.rs", "stage": "R2", "disposition": "retained", "rationale": "Synthetic valid test decision", "status": "planned"}));
        for symbol in file.tests {
            tests.push(serde_json::json!({"id": format!("{}::{symbol}", file.path), "source": file.path, "symbol": symbol, "destination": "tools/fixture.rs", "stage": "R2", "disposition": "retained", "rationale": "Synthetic valid test decision", "evidence": "A future focused regression", "status": "planned"}));
        }
    }
    Ok(serde_json::to_string_pretty(
        &serde_json::json!({"schema_version": 1, "reference_commit": snapshot.reference_commit, "files": files, "tests": tests, "changes": [{"id": "fixture", "before": "Original contract", "after": "Revised contract", "rationale": "Synthetic fixture", "evidence": "Focused test"}]}),
    )?)
}

#[test]
fn complete_inventory_is_valid() -> Result<(), Box<dyn Error>> {
    gamekit_repo_tools::contracts::validate(&valid()?)?;
    Ok(())
}

#[test]
fn missing_duplicate_and_unknown_entries_are_rejected() -> Result<(), Box<dyn Error>> {
    let valid = valid()?;
    for group in ["files", "tests"] {
        let mut missing: Value = serde_json::from_str(&valid)?;
        missing
            .get_mut(group)
            .and_then(Value::as_array_mut)
            .ok_or("array missing")?
            .pop();
        assert!(gamekit_repo_tools::contracts::validate(&missing.to_string()).is_err());
        let mut duplicate: Value = serde_json::from_str(&valid)?;
        let rows = duplicate
            .get_mut(group)
            .and_then(Value::as_array_mut)
            .ok_or("array missing")?;
        let first = rows.first().ok_or("empty group")?.clone();
        rows.push(first);
        assert!(gamekit_repo_tools::contracts::validate(&duplicate.to_string()).is_err());
        let mut unknown: Value = serde_json::from_str(&valid)?;
        let first = unknown
            .get_mut(group)
            .and_then(Value::as_array_mut)
            .and_then(|rows| rows.first_mut())
            .ok_or("first missing")?;
        first
            .as_object_mut()
            .ok_or("object missing")?
            .insert("source".into(), Value::String("unknown.py".into()));
        assert!(gamekit_repo_tools::contracts::validate(&unknown.to_string()).is_err());
    }
    Ok(())
}

#[test]
fn malformed_types_duplicate_fields_and_unexplained_changes_fail() -> Result<(), Box<dyn Error>> {
    let valid = valid()?;
    let duplicate = valid.replacen(
        "\"schema_version\": 1",
        "\"schema_version\": 1, \"schema_version\": 1",
        1,
    );
    assert!(gamekit_repo_tools::contracts::validate(&duplicate).is_err());
    for (pointer, replacement) in [
        ("/schema_version", Value::Bool(true)),
        ("/reference_commit", Value::String("main".into())),
        (
            "/files/0/destination",
            Value::String("tools/../../escape".into()),
        ),
        ("/tests/0/stage", Value::String("done".into())),
        ("/tests/0/evidence", Value::String(String::new())),
        ("/tests/0/disposition", Value::String("ignored".into())),
        ("/changes/0/rationale", Value::String(String::new())),
    ] {
        let mut value: Value = serde_json::from_str(&valid)?;
        *value
            .pointer_mut(pointer)
            .ok_or("fixture pointer missing")? = replacement;
        assert!(
            gamekit_repo_tools::contracts::validate(&value.to_string()).is_err(),
            "{pointer}"
        );
    }
    Ok(())
}

#[test]
fn actual_cli_marks_accounting_as_distinct_from_verification() -> Result<(), Box<dyn Error>> {
    let valid = valid()?;
    let directory = tempfile::tempdir()?;
    std::fs::create_dir(directory.path().join("tools"))?;
    let path = directory.path().join("tools/migration-contracts.json");
    std::fs::write(&path, &valid)?;
    let result = std::process::Command::new(env!("CARGO_BIN_EXE_gamekit-repo"))
        .args(["contracts", "check"])
        .current_dir(directory.path())
        .output()?;
    assert!(result.status.success());
    let value: Value = serde_json::from_slice(&result.stdout)?;
    assert_eq!(value.get("ports_verified"), Some(&Value::Bool(false)));
    assert_eq!(value.get("reference_verified"), Some(&Value::Bool(false)));
    assert_eq!(std::fs::read_to_string(&path)?, valid);
    let missing_history = std::process::Command::new(env!("CARGO_BIN_EXE_gamekit-repo"))
        .args(["contracts", "check", "--verify-reference"])
        .current_dir(directory.path())
        .output()?;
    assert_eq!(missing_history.status.code(), Some(2));
    std::fs::write(&path, "{}")?;
    let bad_inventory = std::process::Command::new(env!("CARGO_BIN_EXE_gamekit-repo"))
        .args(["contracts", "check"])
        .current_dir(directory.path())
        .output()?;
    assert_eq!(bad_inventory.status.code(), Some(2));
    Ok(())
}
