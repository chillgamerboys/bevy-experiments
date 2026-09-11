//! Legacy rendered imports preserve user material and reject invalid provenance.
use gameskills_cli::installation::execute;
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use std::{ffi::OsString, fs, path::Path};
type Result<T = ()> = std::result::Result<T, Box<dyn std::error::Error>>;
const SKILLS: [&str; 7] = [
    "architect-bevy-game",
    "model-turn-based-game",
    "build-bevy-ui",
    "test-bevy-game",
    "verify-bevy-ui",
    "debug-bevy-runtime",
    "review-bevy-change",
];
fn fixture(root: &Path) -> Result {
    let name = ".agents/skills/build-bevy-ui/SKILL.md";
    let base = root.join(".bevy-gamekit/base").join(name);
    fs::create_dir_all(base.parent().ok_or("parent")?)?;
    fs::write(&base, "canonical")?;
    let local = root.join(name);
    fs::create_dir_all(local.parent().ok_or("parent")?)?;
    fs::write(&local, "user modified")?;
    fs::create_dir_all(root.join(".bevy-gamekit/overlays"))?;
    fs::write(
        root.join(".bevy-gamekit/overlays/game.md"),
        "game owned overlay",
    )?;
    let manifest = json!({"schema_version":2,"clients":["codex","claude"],"skills":SKILLS,"source":{"repository":"fixture","revision":"v1","resolved_sha":"a".repeat(40)},"generated":{name:format!("{:x}",Sha256::digest(b"canonical"))}});
    fs::write(
        root.join(".bevy-gamekit/skills.json"),
        serde_json::to_vec(&manifest)?,
    )?;
    Ok(())
}
fn call(root: &Path, args: &[&str]) -> std::result::Result<Value, String> {
    execute(
        root,
        "legacy",
        &args.iter().map(OsString::from).collect::<Vec<_>>(),
    )
}
#[test]
fn import_is_explicit_idempotent_and_preserves_base_edits_overlays() -> Result {
    let root = tempfile::tempdir()?;
    fixture(root.path())?;
    let original = fs::read(root.path().join(".bevy-gamekit/skills.json"))?;
    let report = call(root.path(), &["import"])?;
    assert_eq!(at(&report, "/applied"), false);
    assert!(!root.path().join(".gameskills").exists());
    for _ in 0..2 {
        let report = call(root.path(), &["import", "--apply"])?;
        assert_eq!(
            at(&report, "/import/preserved_generated_edits"),
            &json!([".agents/skills/build-bevy-ui/SKILL.md"])
        );
        assert_eq!(
            fs::read(root.path().join(".bevy-gamekit/skills.json"))?,
            original
        );
        assert_eq!(
            fs::read_to_string(root.path().join(".agents/skills/build-bevy-ui/SKILL.md"))?,
            "user modified"
        );
        assert_eq!(
            fs::read_to_string(
                root.path()
                    .join(".bevy-gamekit/base/.agents/skills/build-bevy-ui/SKILL.md")
            )?,
            "canonical"
        );
        assert_eq!(
            fs::read_to_string(root.path().join(".bevy-gamekit/overlays/game.md"))?,
            "game owned overlay"
        );
    }
    Ok(())
}
#[test]
fn modified_missing_or_extra_base_fails_before_installation() -> Result {
    for mode in ["changed", "missing", "extra"] {
        let root = tempfile::tempdir()?;
        fixture(root.path())?;
        let base = root
            .path()
            .join(".bevy-gamekit/base/.agents/skills/build-bevy-ui/SKILL.md");
        match mode {
            "changed" => fs::write(&base, "changed")?,
            "missing" => fs::remove_file(&base)?,
            _ => fs::write(base.with_file_name("extra.md"), "extra")?,
        };
        assert!(call(root.path(), &["import", "--apply"]).is_err());
        assert!(!root.path().join(".gameskills").exists());
    }
    Ok(())
}
#[test]
fn unsafe_and_noncanonical_legacy_manifest_names_never_write() -> Result {
    for name in [
        "../outside",
        r".agents\skills\build-bevy-ui\SKILL.md",
        ".agents//skills/build-bevy-ui/SKILL.md",
        "./.agents/skills/build-bevy-ui/SKILL.md",
        ".other/skills/a/SKILL.md",
    ] {
        let root = tempfile::tempdir()?;
        fixture(root.path())?;
        let path = root.path().join(".bevy-gamekit/skills.json");
        let mut value: Value = serde_json::from_slice(&fs::read(&path)?)?;
        *value.pointer_mut("/generated").expect("fixture field") = json!({name:"0".repeat(64)});
        fs::write(&path, serde_json::to_vec(&value)?)?;
        assert!(call(root.path(), &["import", "--apply"]).is_err());
        assert!(!root.path().join(".gameskills").exists());
    }
    Ok(())
}
#[test]
fn legacy_install_and_sync_are_intentionally_retired() -> Result {
    let root = tempfile::tempdir()?;
    fixture(root.path())?;
    for command in ["install", "sync"] {
        assert!(call(root.path(), &[command])
            .expect_err("retired")
            .contains("retired"));
    }
    Ok(())
}
#[cfg(unix)]
#[test]
fn legacy_symlink_metadata_and_destinations_fail_before_install() -> Result {
    use std::os::unix::fs::symlink;
    let root = tempfile::tempdir()?;
    fixture(root.path())?;
    let local = root.path().join(".agents/skills/build-bevy-ui/SKILL.md");
    fs::remove_file(&local)?;
    symlink(root.path().join(".bevy-gamekit/overlays/game.md"), &local)?;
    assert!(call(root.path(), &["import", "--apply"]).is_err());
    assert!(!root.path().join(".gameskills").exists());
    Ok(())
}

fn at<'a>(value: &'a Value, path: &str) -> &'a Value {
    value.pointer(path).expect("fixture field")
}

#[test]
fn legacy_provenance_requires_pins_and_marks_schema1_history_unverified() -> Result {
    for (source, schema, valid) in [
        (
            json!({"repository":"fixture","revision":"HEAD","resolved_sha":"a".repeat(40)}),
            2,
            false,
        ),
        (
            json!({"repository":"fixture","revision":"v1","resolved_sha":"abcd"}),
            2,
            false,
        ),
        (json!({"repository":"fixture","revision":"v1"}), 2, false),
        (json!({"repository":"fixture","revision":"v1"}), 1, true),
    ] {
        let root = tempfile::tempdir()?;
        fixture(root.path())?;
        let path = root.path().join(".bevy-gamekit/skills.json");
        let mut value: Value = serde_json::from_slice(&fs::read(&path)?)?;
        *value.get_mut("schema_version").ok_or("schema")? = json!(schema);
        *value.get_mut("source").ok_or("source")? = source;
        fs::write(&path, serde_json::to_vec(&value)?)?;
        let result = call(root.path(), &["import"]);
        if valid {
            assert!(result?
                .pointer("/import/source_provenance")
                .and_then(Value::as_str)
                .ok_or("provenance")?
                .contains("unavailable"));
        } else {
            assert!(result.is_err());
        }
        assert!(!root.path().join(".gameskills").exists());
    }
    Ok(())
}
