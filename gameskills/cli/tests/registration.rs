//! Project-scoped native settings, pin-preserving repair and transaction boundaries.
use serde_json::{json, Value};
use std::{fs, path::Path, process::Command};
type Result<T = ()> = std::result::Result<T, Box<dyn std::error::Error>>;
const CONFIG: &str = ".codex/config.toml";
const RECORD: &str = ".gameskills/native-registration.json";
fn call(root: &Path, args: &[&str]) -> std::result::Result<Value, String> {
    let output = Command::new(env!("CARGO_BIN_EXE_gameskills"))
        .current_dir(root)
        .args(args)
        .output()
        .map_err(|e| e.to_string())?;
    let value: Value = serde_json::from_slice(&output.stdout)
        .map_err(|e| format!("{e}: {}", String::from_utf8_lossy(&output.stderr)))?;
    if output.status.success() {
        Ok(value)
    } else {
        Err(value.to_string())
    }
}
fn registration(root: &Path) -> Result<Value> {
    Ok(call(root, &["status"])?
        .pointer("/native_clients/codex")
        .ok_or("native status")?
        .clone())
}
fn copy(source: &Path, target: &Path) -> Result {
    fs::create_dir_all(target)?;
    for item in fs::read_dir(source)? {
        let item = item?;
        if item.file_type()?.is_dir() {
            copy(&item.path(), &target.join(item.file_name()))?;
        } else {
            fs::copy(item.path(), target.join(item.file_name()))?;
        }
    }
    Ok(())
}
#[test]
fn setup_registers_project_and_preserves_unrelated_settings_and_comments() -> Result {
    let temp = tempfile::tempdir()?;
    let root = temp.path();
    fs::create_dir(root.join(".codex"))?;
    let owned = "# Owner chooses this model.\nmodel = 'owner-model'\n[plugins.'custom@personal']\nenabled = false # keep disabled\n";
    fs::write(root.join(CONFIG), owned)?;
    let proposal = call(root, &["setup"])?;
    assert_eq!(
        proposal
            .pointer("/project_registration_change")
            .ok_or("response field")?,
        true
    );
    assert_eq!(fs::read_to_string(root.join(CONFIG))?, owned);
    assert!(!root.join(RECORD).exists());
    let applied = call(root, &["setup", "--apply"])?;
    assert_eq!(
        applied
            .pointer("/native_clients/codex/registration")
            .ok_or("response field")?,
        "registered"
    );
    assert_eq!(
        applied
            .pointer("/native_clients/claude/registration")
            .ok_or("response field")?,
        "unsupported"
    );
    let config = fs::read_to_string(root.join(CONFIG))?;
    assert!(config.starts_with(owned));
    assert!(!config.contains("trust_level"));
    let commented = config.replacen("enabled = true", "enabled = true # local note", 1);
    fs::write(root.join(CONFIG), &commented)?;
    let bytes = fs::read(root.join(RECORD))?;
    let repeated = call(root, &["native", "codex", "--register", "--apply"])?;
    assert_eq!(
        repeated
            .pointer("/registration_change")
            .ok_or("response field")?,
        false
    );
    assert_eq!(fs::read_to_string(root.join(CONFIG))?, commented);
    assert_eq!(fs::read(root.join(RECORD))?, bytes);
    assert!(registration(root)?
        .get("discovery")
        .ok_or("discovery")?
        .as_str()
        .ok_or("discovery")?
        .contains("unobserved"));
    Ok(())
}
#[test]
fn old_staging_repairs_same_pin_during_queue_and_never_writes_user_home() -> Result {
    let temp = tempfile::tempdir()?;
    let root = temp.path();
    call(root, &["setup", "--apply"])?;
    fs::remove_file(root.join(CONFIG))?;
    fs::remove_file(root.join(RECORD))?;
    assert_eq!(
        registration(root)?
            .get("registration")
            .ok_or("registration")?,
        "missing"
    );
    let lock = fs::read(root.join("gameskills.lock.json"))?;
    let config = fs::read(root.join("gameskills.toml"))?;
    fs::write(
        root.join(".gameskills/queues/active.json"),
        serde_json::to_vec(
            &json!({"schema_version":2,"runtime":"rust","orders":{"work":{"state":"running"}}}),
        )?,
    )?;
    assert!(call(root, &["setup", "--apply"]).is_err());
    let fake_home = tempfile::tempdir()?;
    fs::write(fake_home.path().join("config.toml"), "# user-owned\n")?;
    let output = Command::new(env!("CARGO_BIN_EXE_gameskills"))
        .current_dir(root)
        .args(["native", "codex", "--register", "--apply"])
        .env("CODEX_HOME", fake_home.path())
        .output()?;
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stdout)
    );
    assert_eq!(
        fs::read_to_string(fake_home.path().join("config.toml"))?,
        "# user-owned\n"
    );
    assert_eq!(fs::read_dir(fake_home.path())?.count(), 1);
    assert_eq!(fs::read(root.join("gameskills.lock.json"))?, lock);
    assert_eq!(fs::read(root.join("gameskills.toml"))?, config);
    assert_eq!(
        registration(root)?
            .get("registration")
            .ok_or("registration")?,
        "registered"
    );
    Ok(())
}
#[test]
fn local_disable_and_source_edits_are_conflicts_not_silent_repairs() -> Result {
    for replacement in ["enabled = false", "enabled = 'owner-value'"] {
        let temp = tempfile::tempdir()?;
        let root = temp.path();
        call(root, &["setup", "--apply"])?;
        let config =
            fs::read_to_string(root.join(CONFIG))?.replacen("enabled = true", replacement, 1);
        fs::write(root.join(CONFIG), &config)?;
        let record = fs::read(root.join(RECORD))?;
        assert_eq!(
            registration(root)?
                .get("registration")
                .ok_or("registration")?,
            "conflict"
        );
        assert!(call(root, &["native", "codex", "--register", "--apply"])
            .expect_err("local edit")
            .contains("local edit"));
        assert!(call(root, &["setup", "--apply"]).is_err());
        assert_eq!(fs::read_to_string(root.join(CONFIG))?, config);
        assert_eq!(fs::read(root.join(RECORD))?, record);
    }
    Ok(())
}
#[test]
fn relocation_is_outdated_and_idempotently_rebinds_same_pin() -> Result {
    let source = tempfile::tempdir()?;
    call(source.path(), &["setup", "--apply"])?;
    let moved = tempfile::tempdir()?;
    copy(source.path(), moved.path())?;
    assert_eq!(
        registration(moved.path())?
            .get("registration")
            .ok_or("registration")?,
        "outdated"
    );
    let old_lock = fs::read(moved.path().join("gameskills.lock.json"))?;
    call(moved.path(), &["native", "codex", "--register", "--apply"])?;
    let config = fs::read_to_string(moved.path().join(CONFIG))?;
    assert!(!config.contains(source.path().to_str().ok_or("source")?));
    assert_eq!(
        registration(moved.path())?
            .get("registration")
            .ok_or("registration")?,
        "registered"
    );
    assert_eq!(
        fs::read(moved.path().join("gameskills.lock.json"))?,
        old_lock
    );
    call(moved.path(), &["native", "codex", "--register", "--apply"])?;
    assert_eq!(fs::read_to_string(moved.path().join(CONFIG))?, config);
    Ok(())
}
#[test]
fn package_update_removes_only_unchanged_owned_pin_and_client_removal_cleans_registration() -> Result
{
    let temp = tempfile::tempdir()?;
    let root = temp.path();
    call(root, &["setup", "--apply"])?;
    let first: toml::Value = toml::from_str(&fs::read_to_string(root.join(CONFIG))?)?;
    let old_market = first
        .get("marketplaces")
        .ok_or("marketplaces")?
        .as_table()
        .ok_or("market")?
        .keys()
        .next()
        .ok_or("market")?
        .clone();
    call(
        root,
        &[
            "setup",
            "--packages",
            "gameskills",
            "gameskills-ui",
            "--apply",
        ],
    )?;
    let second: toml::Value = toml::from_str(&fs::read_to_string(root.join(CONFIG))?)?;
    assert!(!second
        .get("marketplaces")
        .ok_or("marketplaces")?
        .as_table()
        .ok_or("market")?
        .contains_key(&old_market));
    assert_eq!(
        second
            .get("plugins")
            .ok_or("plugins")?
            .as_table()
            .ok_or("plugins")?
            .len(),
        2
    );
    fs::write(
        root.join("gameskills.toml"),
        "schema_version=1\npackages=['gameskills','gameskills-ui']\nclients=['claude']\n",
    )?;
    call(root, &["setup", "--apply"])?;
    let removed: toml::Value = toml::from_str(&fs::read_to_string(root.join(CONFIG))?)?;
    assert!(removed.get("marketplaces").is_none());
    assert!(!root.join(RECORD).exists());
    assert!(call(root, &["native", "claude", "--register", "--apply"]).is_err());
    Ok(())
}
#[test]
fn registration_recovery_preserves_newer_edits_and_does_not_repin() -> Result {
    let temp = tempfile::tempdir()?;
    let root = temp.path();
    call(root, &["setup", "--apply"])?;
    let config = fs::read_to_string(root.join(CONFIG))?;
    let record = fs::read_to_string(root.join(RECORD))?;
    let lock = fs::read(root.join("gameskills.lock.json"))?;
    let journal = root.join(".gameskills/native-transaction.json");
    fs::write(
        &journal,
        serde_json::to_vec(
            &json!({CONFIG:{"before":config,"after":"# interrupted\n"},RECORD:{"before":record,"after":null}}),
        )?,
    )?;
    fs::write(root.join(CONFIG), "# newer local edit\n")?;
    assert!(call(root, &["native", "codex", "--register", "--recover"])
        .expect_err("local edit")
        .contains("local edit"));
    assert_eq!(
        fs::read_to_string(root.join(CONFIG))?,
        "# newer local edit\n"
    );
    assert_eq!(
        registration(root)?
            .get("registration")
            .ok_or("registration")?,
        "conflict"
    );
    fs::write(root.join(CONFIG), "# interrupted\n")?;
    fs::write(
        root.join(".gameskills/queues/pending.json"),
        serde_json::to_vec(
            &json!({"schema_version":2,"runtime":"rust","orders":{"work":{"state":"running"}}}),
        )?,
    )?;
    call(root, &["native", "codex", "--register", "--recover"])?;
    assert_eq!(fs::read_to_string(root.join(CONFIG))?, config);
    assert_eq!(fs::read_to_string(root.join(RECORD))?, record);
    assert_eq!(fs::read(root.join("gameskills.lock.json"))?, lock);
    assert!(!journal.exists());
    Ok(())
}
#[cfg(unix)]
#[test]
fn project_directory_file_and_record_symlinks_are_refused() -> Result {
    for attacked in [".codex", CONFIG, RECORD] {
        let temp = tempfile::tempdir()?;
        let root = temp.path();
        call(root, &["setup", "--apply"])?;
        let outside = tempfile::tempdir()?;
        let victim = outside.path().join("owned");
        fs::write(&victim, "# outside\n")?;
        if attacked == ".codex" {
            fs::remove_file(root.join(CONFIG))?;
            fs::remove_dir(root.join(".codex"))?;
            std::os::unix::fs::symlink(outside.path(), root.join(attacked))?;
        } else {
            fs::remove_file(root.join(attacked))?;
            std::os::unix::fs::symlink(&victim, root.join(attacked))?;
        }
        assert!(call(root, &["native", "codex", "--register", "--apply"]).is_err());
        assert!(call(root, &["setup", "--apply"]).is_err());
        assert_eq!(fs::read_to_string(&victim)?, "# outside\n");
    }
    Ok(())
}

#[test]
fn claude_only_setup_does_not_read_or_change_unmanaged_codex_config() -> Result {
    let root = tempfile::tempdir()?;
    fs::write(
        root.path().join("gameskills.toml"),
        "schema_version=1\nclients=['claude']\n",
    )?;
    fs::create_dir(root.path().join(".codex"))?;
    fs::write(
        root.path().join(CONFIG),
        "this is not valid TOML and belongs to another tool",
    )?;
    call(root.path(), &["setup", "--apply"])?;
    assert_eq!(
        fs::read_to_string(root.path().join(CONFIG))?,
        "this is not valid TOML and belongs to another tool"
    );
    assert!(!root.path().join(RECORD).exists());
    #[cfg(unix)]
    {
        fs::remove_file(root.path().join(CONFIG))?;
        let outside = tempfile::tempdir()?;
        fs::write(outside.path().join("owned"), "untouched")?;
        std::os::unix::fs::symlink(outside.path().join("owned"), root.path().join(CONFIG))?;
        call(root.path(), &["setup", "--apply"])?;
        assert_eq!(
            fs::read_to_string(outside.path().join("owned"))?,
            "untouched"
        );
    }
    Ok(())
}

#[test]
fn setup_recovery_restores_extended_registration_journal_atomically() -> Result {
    let root = tempfile::tempdir()?;
    call(root.path(), &["setup", "--apply"])?;
    let mut transaction = json!({});
    let paths = ["gameskills.toml", "gameskills.lock.json", CONFIG, RECORD];
    for path in paths {
        transaction.as_object_mut().ok_or("journal")?.insert(
            path.into(),
            json!({"before":fs::read_to_string(root.path().join(path))?,"after":null}),
        );
    }
    let journal = root.path().join(".gameskills/setup-transaction.json");
    fs::write(&journal, serde_json::to_vec(&transaction)?)?;
    for path in paths {
        fs::remove_file(root.path().join(path))?;
    }
    fs::write(root.path().join(CONFIG), "# intervening local edit\n")?;
    assert!(call(root.path(), &["setup", "--recover"])
        .expect_err("local edit")
        .contains("local edit"));
    assert!(!root.path().join("gameskills.toml").exists());
    fs::remove_file(root.path().join(CONFIG))?;
    call(root.path(), &["setup", "--recover"])?;
    for path in paths {
        assert_eq!(
            fs::read_to_string(root.path().join(path))?,
            transaction
                .get(path)
                .and_then(|v| v.get("before"))
                .ok_or("before")?
                .as_str()
                .ok_or("before")?
        );
    }
    assert!(!journal.exists());
    Ok(())
}

#[test]
fn unmanaged_old_pin_and_extra_owned_table_settings_are_not_deleted() -> Result {
    let root = tempfile::tempdir()?;
    call(root.path(), &["setup", "--apply"])?;
    fs::remove_file(root.path().join(RECORD))?;
    let owner_config = fs::read_to_string(root.path().join(CONFIG))?;
    assert!(call(
        root.path(),
        &[
            "setup",
            "--packages",
            "gameskills",
            "gameskills-ui",
            "--apply"
        ]
    )
    .expect_err("unmanaged old pin")
    .contains("another project GameSkills registration"));
    assert_eq!(fs::read_to_string(root.path().join(CONFIG))?, owner_config);
    let root = tempfile::tempdir()?;
    call(root.path(), &["setup", "--apply"])?;
    let owner_config = fs::read_to_string(root.path().join(CONFIG))?.replace(
        "enabled = true",
        "enabled = true\nowner_note = 'keep this value'",
    );
    fs::write(root.path().join(CONFIG), &owner_config)?;
    assert!(call(
        root.path(),
        &[
            "setup",
            "--packages",
            "gameskills",
            "gameskills-ui",
            "--apply"
        ]
    )
    .expect_err("extra settings")
    .contains("additional local settings"));
    assert_eq!(fs::read_to_string(root.path().join(CONFIG))?, owner_config);
    Ok(())
}

#[test]
fn preexisting_empty_parent_tables_and_comments_survive_update_and_cleanup() -> Result {
    let root = tempfile::tempdir()?;
    fs::create_dir(root.path().join(".codex"))?;
    let owner = "[plugins] # keep this owner note\n\n[marketplaces] # shared catalog settings\n";
    fs::write(root.path().join(CONFIG), owner)?;
    call(root.path(), &["setup", "--apply"])?;
    call(
        root.path(),
        &[
            "setup",
            "--packages",
            "gameskills",
            "gameskills-ui",
            "--apply",
        ],
    )?;
    let updated = fs::read_to_string(root.path().join(CONFIG))?;
    assert!(updated.contains("[plugins] # keep this owner note"));
    assert!(updated.contains("[marketplaces] # shared catalog settings"));
    fs::write(
        root.path().join("gameskills.toml"),
        "schema_version=1\npackages=['gameskills','gameskills-ui']\nclients=['claude']\n",
    )?;
    call(root.path(), &["setup", "--apply"])?;
    assert_eq!(fs::read_to_string(root.path().join(CONFIG))?, owner);
    Ok(())
}
