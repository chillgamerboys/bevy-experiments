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
        files.push(serde_json::json!({"source": file.path, "destination": "devtools/fixture.rs", "stage": "R2", "disposition": "retained", "rationale": "Synthetic valid test decision", "status": "planned"}));
        for symbol in file.tests {
            tests.push(serde_json::json!({"id": format!("{}::{symbol}", file.path), "source": file.path, "symbol": symbol, "destination": "devtools/fixture.rs", "stage": "R2", "disposition": "retained", "rationale": "Synthetic valid test decision", "evidence": "A future focused regression", "status": "planned"}));
        }
    }
    Ok(serde_json::to_string_pretty(
        &serde_json::json!({"schema_version": 1, "reference_commit": snapshot.reference_commit, "files": files, "tests": tests, "changes": [{"id": "fixture", "before": "Original contract", "after": "Revised contract", "rationale": "Synthetic fixture", "evidence": "Focused test"}]}),
    )?)
}

#[test]
fn complete_inventory_is_valid() -> Result<(), Box<dyn Error>> {
    repo_devtools::contracts::validate(&valid()?)?;
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
        assert!(repo_devtools::contracts::validate(&missing.to_string()).is_err());
        let mut duplicate: Value = serde_json::from_str(&valid)?;
        let rows = duplicate
            .get_mut(group)
            .and_then(Value::as_array_mut)
            .ok_or("array missing")?;
        let first = rows.first().ok_or("empty group")?.clone();
        rows.push(first);
        assert!(repo_devtools::contracts::validate(&duplicate.to_string()).is_err());
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
        assert!(repo_devtools::contracts::validate(&unknown.to_string()).is_err());
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
    assert!(repo_devtools::contracts::validate(&duplicate).is_err());
    for (pointer, replacement) in [
        ("/schema_version", Value::Bool(true)),
        ("/reference_commit", Value::String("main".into())),
        (
            "/files/0/destination",
            Value::String("devtools/../../escape".into()),
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
            repo_devtools::contracts::validate(&value.to_string()).is_err(),
            "{pointer}"
        );
    }
    Ok(())
}

#[test]
fn actual_cli_marks_accounting_as_distinct_from_verification() -> Result<(), Box<dyn Error>> {
    let valid = valid()?;
    let directory = tempfile::tempdir()?;
    std::fs::create_dir(directory.path().join("devtools"))?;
    let path = directory.path().join("devtools/migration-contracts.json");
    std::fs::write(&path, &valid)?;
    let result = std::process::Command::new(env!("CARGO_BIN_EXE_repo-devtools"))
        .args(["contracts", "check"])
        .current_dir(directory.path())
        .output()?;
    assert!(result.status.success());
    let value: Value = serde_json::from_slice(&result.stdout)?;
    assert_eq!(value.get("ports_verified"), Some(&Value::Bool(false)));
    assert_eq!(value.get("reference_verified"), Some(&Value::Bool(false)));
    assert_eq!(std::fs::read_to_string(&path)?, valid);
    let missing_history = std::process::Command::new(env!("CARGO_BIN_EXE_repo-devtools"))
        .args(["contracts", "check", "--verify-reference"])
        .current_dir(directory.path())
        .output()?;
    assert_eq!(missing_history.status.code(), Some(2));
    std::fs::write(&path, "{}")?;
    let bad_inventory = std::process::Command::new(env!("CARGO_BIN_EXE_repo-devtools"))
        .args(["contracts", "check"])
        .current_dir(directory.path())
        .output()?;
    assert_eq!(bad_inventory.status.code(), Some(2));
    Ok(())
}

#[test]
fn cutover_rejects_unfinished_missing_owners_python_sources_and_ci_setup(
) -> Result<(), Box<dyn Error>> {
    use repo_devtools::contracts::cutover;
    let directory = tempfile::tempdir()?;
    let root = directory.path();
    let mut inventory: Value = serde_json::from_str(&valid()?)?;
    std::fs::create_dir(root.join("devtools"))?;
    std::fs::write(
        root.join("devtools/fixture.rs"),
        "//! Synthetic Rust owner.\n",
    )?;
    let git = |args: &[&str]| -> Result<(), Box<dyn Error>> {
        let output = std::process::Command::new("git")
            .current_dir(root)
            .args(args)
            .output()?;
        if !output.status.success() {
            return Err(String::from_utf8(output.stderr)?.into());
        }
        Ok(())
    };
    git(&["init", "-q"])?;
    assert!(cutover(root, &inventory.to_string()).is_err());
    for group in ["files", "tests"] {
        for row in inventory
            .get_mut(group)
            .and_then(Value::as_array_mut)
            .ok_or("rows")?
        {
            *row.get_mut("status").ok_or("status")? = Value::from("implemented");
        }
    }
    cutover(root, &inventory.to_string())?;
    std::fs::remove_file(root.join("devtools/fixture.rs"))?;
    assert!(cutover(root, &inventory.to_string()).is_err());
    std::fs::write(
        root.join("devtools/fixture.rs"),
        "//! Synthetic Rust owner.\n",
    )?;
    for name in ["old.py", "old.pyi", "old.PYC", "old.pyo"] {
        std::fs::write(
            root.join(name),
            "# Interpreter source must not remain tracked.\n",
        )?;
        git(&["add", name])?;
        assert!(cutover(root, &inventory.to_string()).is_err());
        git(&["rm", "-f", name])?;
    }
    std::fs::create_dir_all(root.join(".github/workflows"))?;
    std::fs::write(
        root.join(".github/workflows/test.yml"),
        "steps:\n  - uses: actions/setup-python@v6\n",
    )?;
    git(&["add", ".github/workflows/test.yml"])?;
    assert!(cutover(root, &inventory.to_string()).is_err());
    std::fs::write(
        root.join(".github/workflows/test.yml"),
        "steps:\n  - run: cargo test\n",
    )?;
    cutover(root, &inventory.to_string())?;
    Ok(())
}
