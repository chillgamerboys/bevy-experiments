//! Recovery cannot cross an active runtime or bypass queue serialization.

#[cfg(unix)]
#[test]
fn actual_cli_recovery_waits_for_queue_and_run_exclusions() -> Result<(), Box<dyn std::error::Error>>
{
    use serde_json::{json, Value};
    use std::{fs, process::Command};
    let temporary = tempfile::tempdir()?;
    let root = temporary.path();
    let cli = |args: &[&str]| {
        Command::new(env!("CARGO_BIN_EXE_gameskills"))
            .current_dir(root)
            .args(args)
            .output()
    };
    let installed = cli(&["setup", "--apply"])?;
    assert!(installed.status.success(), "{:?}", installed);
    let config = fs::read_to_string(root.join("gameskills.toml"))?;
    let lock = fs::read_to_string(root.join("gameskills.lock.json"))?;
    let journal = serde_json::to_vec(&json!({
        "gameskills.toml": {"before": config, "after": "schema_version=1\n"},
        "gameskills.lock.json": {"before": lock, "after": null}
    }))?;
    let journal_path = root.join(".gameskills/setup-transaction.json");
    fs::write(&journal_path, &journal)?;
    fs::write(root.join("gameskills.toml"), "schema_version=1\n")?;
    let assert_unchanged = || -> Result<(), Box<dyn std::error::Error>> {
        assert_eq!(fs::read(&journal_path)?, journal);
        assert_eq!(
            fs::read_to_string(root.join("gameskills.toml"))?,
            "schema_version=1\n"
        );
        assert_eq!(fs::read_to_string(root.join("gameskills.lock.json"))?, lock);
        Ok(())
    };
    let queue_guard = fs::OpenOptions::new()
        .read(true)
        .write(true)
        .open(root.join(".gameskills/queues/.lock"))?;
    queue_guard.lock()?;
    assert!(!cli(&["setup", "--recover"])?.status.success());
    assert_unchanged()?;
    drop(queue_guard);
    let queue_path = root.join(".gameskills/queues/pending.json");
    fs::write(
        &queue_path,
        serde_json::to_vec(&json!({
            "schema_version":2,"runtime":"rust","orders":{"work":{"state":"running"}}
        }))?,
    )?;
    assert!(!cli(&["setup", "--recover"])?.status.success());
    assert_unchanged()?;
    fs::write(
        &queue_path,
        serde_json::to_vec(&json!({
            "schema_version":2,"runtime":"rust","orders":{"work":{"state":"integrated"}}
        }))?,
    )?;
    let run = root.join(".gameskills/runs").join("a".repeat(32));
    fs::create_dir_all(&run)?;
    let active = fs::File::create(run.join("active.lock"))?;
    active.lock()?;
    assert!(!cli(&["setup", "--recover"])?.status.success());
    assert_unchanged()?;
    drop(active);
    let recovered = cli(&["setup", "--recover"])?;
    assert!(recovered.status.success(), "{:?}", recovered);
    let value: Value = serde_json::from_slice(&recovered.stdout)?;
    assert_eq!(value.get("recovered"), Some(&json!(true)));
    assert_eq!(fs::read_to_string(root.join("gameskills.toml"))?, config);
    assert_eq!(fs::read_to_string(root.join("gameskills.lock.json"))?, lock);
    assert!(!journal_path.exists());
    Ok(())
}
