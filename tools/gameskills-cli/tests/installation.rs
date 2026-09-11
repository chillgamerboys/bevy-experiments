//! Immutable instruction, transaction and migration boundary regressions.
use gameskills_cli::installation::{execute, verify_archive};
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use std::{
    collections::BTreeMap,
    ffi::OsString,
    fs,
    io::{Read, Write},
    path::Path,
};

type Result<T = ()> = std::result::Result<T, Box<dyn std::error::Error>>;
fn call(root: &Path, family: &str, args: &[&str]) -> std::result::Result<Value, String> {
    execute(
        root,
        family,
        &args.iter().map(OsString::from).collect::<Vec<_>>(),
    )
}
fn snapshot(root: &Path) -> Result<BTreeMap<String, Vec<u8>>> {
    fn visit(root: &Path, path: &Path, out: &mut BTreeMap<String, Vec<u8>>) -> Result {
        for item in fs::read_dir(path)? {
            let item = item?;
            if item.file_type()?.is_dir() {
                visit(root, &item.path(), out)?;
            } else {
                out.insert(
                    item.path()
                        .strip_prefix(root)?
                        .to_string_lossy()
                        .into_owned(),
                    fs::read(item.path())?,
                );
            }
        }
        Ok(())
    }
    let mut out = BTreeMap::new();
    visit(root, root, &mut out)?;
    Ok(out)
}
fn installed(root: &Path) -> Result<Value> {
    Ok(call(root, "setup", &["--apply"])?)
}
fn export(root: &Path, name: &str, packages: &[&str]) -> Result {
    let mut args = vec!["--out", name, "--packages"];
    args.extend(packages);
    call(root, "bundle", &args)?;
    Ok(())
}
fn hash(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}
fn archive_files() -> Result<Vec<(String, Vec<u8>)>> {
    let decoder =
        flate2::read::GzDecoder::new(include_bytes!("../bundle/instructions.tar.gz").as_slice());
    let mut archive = tar::Archive::new(decoder);
    let mut out = Vec::new();
    for entry in archive.entries()? {
        let mut entry = entry?;
        let name = entry.path()?.to_string_lossy().into_owned();
        let mut bytes = Vec::new();
        entry.read_to_end(&mut bytes)?;
        out.push((name, bytes));
    }
    Ok(out)
}
fn make_archive(files: &[(String, Vec<u8>)]) -> Result<Vec<u8>> {
    let encoder = flate2::write::GzEncoder::new(Vec::new(), flate2::Compression::default());
    let mut tar = tar::Builder::new(encoder);
    for (name, bytes) in files {
        let mut header = tar::Header::new_gnu();
        header.set_size(bytes.len() as u64);
        header.set_mode(0o644);
        header.set_cksum();
        tar.append_data(&mut header, name, bytes.as_slice())?;
    }
    Ok(tar.into_inner()?.finish()?)
}

#[test]
fn proposal_is_read_only_and_defaults_to_five_workers() -> Result {
    let root = tempfile::tempdir()?;
    let before = snapshot(root.path())?;
    let result = call(root.path(), "setup", &[])?;
    assert_eq!(at(&result, "/applied"), false);
    assert_eq!(at(&result, "/max_workers"), 5);
    assert_eq!(before, snapshot(root.path())?);
    Ok(())
}
#[test]
fn embedded_baseline_installs_offline_and_is_idempotent() -> Result {
    let root = tempfile::tempdir()?;
    installed(root.path())?;
    let before = snapshot(root.path())?;
    installed(root.path())?;
    assert_eq!(before, snapshot(root.path())?);
    assert_eq!(at(&call(root.path(), "status", &[])?, "/runtime"), "rust");
    assert!(before.keys().all(|name| !name.ends_with(".py")));
    Ok(())
}
#[test]
fn selection_changes_and_rollback_preserve_immutable_bundles() -> Result {
    let root = tempfile::tempdir()?;
    let first = installed(root.path())?;
    let second = call(
        root.path(),
        "setup",
        &["--packages", "gameskills", "gameskills-ui", "--apply"],
    )?;
    assert_ne!(at(&first, "/destination"), at(&second, "/destination"));
    let third = call(
        root.path(),
        "setup",
        &["--packages", "gameskills", "--apply"],
    )?;
    assert_eq!(at(&first, "/destination"), at(&third, "/destination"));
    assert!(Path::new(at(&second, "/destination").as_str().ok_or("destination")?).is_dir());
    Ok(())
}
#[test]
fn preserves_quoted_config_comments_and_client_overlays() -> Result {
    for declaration in [
        "  packages = [\"gameskills\"]\n",
        "\"packages\" = [\"gameskills\"]\n",
        "",
    ] {
        let root = tempfile::tempdir()?;
        let text=format!("# owned\nschema_version = 1\n{declaration}\n[project]\nname = \"Owner\"\n\n[commands.check]\nargv = [\"custom-check\"]\n");
        fs::write(root.path().join("gameskills.toml"), &text)?;
        fs::write(root.path().join("CLAUDE.md"), "owned")?;
        fs::create_dir(root.path().join(".agents"))?;
        fs::write(root.path().join(".agents/custom"), "custom")?;
        installed(root.path())?;
        assert_eq!(
            fs::read_to_string(root.path().join("gameskills.toml"))?,
            text
        );
        call(
            root.path(),
            "setup",
            &["--packages", "gameskills", "gameskills-ui", "--apply"],
        )?;
        let updated = fs::read_to_string(root.path().join("gameskills.toml"))?;
        assert!(updated.contains("# owned"));
        assert!(updated.contains("name = \"Owner\""));
        assert_eq!(fs::read_to_string(root.path().join("CLAUDE.md"))?, "owned");
        assert_eq!(
            fs::read_to_string(root.path().join(".agents/custom"))?,
            "custom"
        );
    }
    Ok(())
}
#[test]
fn exported_selection_is_verified_and_destinations_are_never_overwritten() -> Result {
    let root = tempfile::tempdir()?;
    export(root.path(), "export", &["gameskills"])?;
    assert!(call(root.path(), "bundle", &["--out", "export"]).is_err());
    call(root.path(), "setup", &["--bundle", "export", "--apply"])?;
    let tampered = root
        .path()
        .join("export/plugins/gameskills/skills/plan/SKILL.md");
    fs::write(&tampered, "tampered")?;
    assert!(call(root.path(), "setup", &["--bundle", "export"]).is_err());
    Ok(())
}
#[test]
fn rejects_tampered_marketplace_and_unrecorded_files() -> Result {
    for path in [
        ".agents/plugins/marketplace.json",
        "plugins/gameskills/unrecorded.md",
    ] {
        let root = tempfile::tempdir()?;
        export(root.path(), "export", &["gameskills"])?;
        fs::write(root.path().join("export").join(path), "{}")?;
        assert!(call(root.path(), "setup", &["--bundle", "export"]).is_err());
    }
    Ok(())
}
#[test]
fn lock_is_portable_and_selection_changes_require_setup() -> Result {
    let root = tempfile::tempdir()?;
    installed(root.path())?;
    let other = tempfile::tempdir()?;
    for (name, bytes) in snapshot(root.path())? {
        let path = other.path().join(name);
        fs::create_dir_all(path.parent().ok_or("parent")?)?;
        fs::write(path, bytes)?;
    }
    call(other.path(), "status", &[])?;
    fs::write(
        root.path().join("gameskills.toml"),
        "schema_version=1\npackages=[\"gameskills\",\"gameskills-ui\"]\n",
    )?;
    assert!(call(root.path(), "status", &[]).is_err());
    fs::remove_dir_all(other.path().join(".gameskills/bundles"))?;
    assert!(call(other.path(), "status", &[]).is_err());
    Ok(())
}
fn journal(root: &Path, edited: bool) -> Result<BTreeMap<String, String>> {
    installed(root)?;
    let mut before = BTreeMap::new();
    for name in ["gameskills.toml", "gameskills.lock.json"] {
        before.insert(name.into(), fs::read_to_string(root.join(name))?);
    }
    let transaction = json!({"gameskills.toml":{"before":before.get("gameskills.toml"),"after":"schema_version=1\n"},"gameskills.lock.json":{"before":before.get("gameskills.lock.json"),"after":null}});
    fs::write(
        root.join(".gameskills/setup-transaction.json"),
        serde_json::to_vec(&transaction)?,
    )?;
    fs::write(
        root.join("gameskills.toml"),
        if edited {
            "# user edit\nschema_version=1\n"
        } else {
            "schema_version=1\n"
        },
    )?;
    Ok(before)
}
#[test]
fn recovery_restores_interrupted_transaction_and_refuses_local_edits() -> Result {
    let root = tempfile::tempdir()?;
    let before = journal(root.path(), false)?;
    assert!(call(root.path(), "status", &[]).is_err());
    assert!(call(root.path(), "setup", &[]).is_err());
    assert_eq!(
        at(&call(root.path(), "setup", &["--recover"])?, "/recovered"),
        true
    );
    for (name, value) in before {
        assert_eq!(fs::read_to_string(root.path().join(name))?, value);
    }
    journal(root.path(), true)?;
    let before = snapshot(root.path())?;
    assert!(call(root.path(), "setup", &["--recover"])
        .expect_err("local edits")
        .contains("overwrite a local edit"));
    assert_eq!(before, snapshot(root.path())?);
    Ok(())
}
#[test]
fn malformed_recovery_journal_never_overwrites_files() -> Result {
    let root = tempfile::tempdir()?;
    installed(root.path())?;
    for value in [
        json!({"../outside":{"before":null,"after":null}}),
        json!({"gameskills.toml":{"before":4,"after":null},"gameskills.lock.json":{"before":null,"after":null}}),
    ] {
        fs::write(
            root.path().join(".gameskills/setup-transaction.json"),
            serde_json::to_vec(&value)?,
        )?;
        let before = snapshot(root.path())?;
        assert!(call(root.path(), "setup", &["--recover"]).is_err());
        assert_eq!(before, snapshot(root.path())?);
    }
    Ok(())
}
#[test]
fn setup_lock_and_active_runs_exclude_updates() -> Result {
    let root = tempfile::tempdir()?;
    installed(root.path())?;
    let lock = gameskills_cli::installation::lifecycle_guard(root.path())?;
    assert!(call(root.path(), "setup", &["--apply"]).is_err());
    assert!(call(root.path(), "setup", &["--recover"]).is_err());
    drop(lock);
    let run = root.path().join(".gameskills/runs").join("a".repeat(32));
    fs::create_dir_all(&run)?;
    let active = fs::File::create(run.join("active.lock"))?;
    active.lock()?;
    assert!(call(root.path(), "setup", &["--apply"])
        .expect_err("active run")
        .contains("active"));
    drop(active);
    installed(root.path())?;
    Ok(())
}
#[test]
fn active_python_and_rust_queues_block_transition_without_relabeling() -> Result {
    for (schema, runtime) in [(1, Value::Null), (2, json!("rust"))] {
        let root = tempfile::tempdir()?;
        installed(root.path())?;
        let path = root.path().join(".gameskills/queues/old.json");
        let value = json!({"schema_version":schema,"runtime":runtime,"orders":{"work":{"state":"reported"}}});
        fs::write(&path, serde_json::to_vec(&value)?)?;
        assert!(call(root.path(), "setup", &["--apply"])
            .expect_err("unfinished")
            .contains("unfinished"));
        assert_eq!(serde_json::from_slice::<Value>(&fs::read(&path)?)?, value);
        let mut done = value;
        *done
            .pointer_mut("/orders/work/state")
            .expect("fixture field") = json!("integrated");
        fs::write(&path, serde_json::to_vec(&done)?)?;
        installed(root.path())?;
        assert_eq!(serde_json::from_slice::<Value>(&fs::read(&path)?)?, done);
    }
    Ok(())
}
#[test]
fn archive_rejects_corruption_duplicates_extensions_and_incompatible_identity() -> Result {
    let files = archive_files()?;
    verify_archive(&make_archive(&files)?)?;
    let mut duplicate = files.clone();
    duplicate.push(files.first().ok_or("file")?.clone());
    assert!(verify_archive(&make_archive(&duplicate)?).is_err());
    let mut corrupted = files.clone();
    corrupted
        .iter_mut()
        .find(|(name, _)| name.ends_with("plan/SKILL.md"))
        .ok_or("skill")?
        .1
        .push(b'!');
    assert!(verify_archive(&make_archive(&corrupted)?).is_err());
    let mut bytes = make_archive(&files)?;
    bytes.push(0);
    assert!(verify_archive(&bytes).is_err());
    for (key, value) in [
        ("activation", json!("preparation-only")),
        ("queue_schema", json!(1)),
        ("evidence_schema", json!(1)),
        ("cli_version_range", json!(">9")),
    ] {
        let mut changed = files.clone();
        let manifest = changed
            .iter_mut()
            .find(|(name, _)| name == "bundle.json")
            .ok_or("manifest")?;
        let mut value_manifest: Value = serde_json::from_slice(&manifest.1)?;
        put(&mut value_manifest, &format!("/compatibility/{key}"), value);
        let mut identity = value_manifest.clone();
        identity
            .as_object_mut()
            .ok_or("object")?
            .remove("content_sha256");
        identity
            .as_object_mut()
            .ok_or("object")?
            .remove("source_commit");
        identity.sort_all_objects();
        let mut identity = serde_json::to_vec_pretty(&identity)?;
        identity.push(b'\n');
        *value_manifest
            .pointer_mut("/content_sha256")
            .expect("fixture field") = json!(hash(&identity));
        manifest.1 = serde_json::to_vec(&value_manifest)?;
        assert!(verify_archive(&make_archive(&changed)?).is_err());
    }
    Ok(())
}
#[test]
fn archive_rejects_bounded_compression_bomb_and_duplicate_json_keys() -> Result {
    let mut encoder = flate2::write::GzEncoder::new(Vec::new(), flate2::Compression::fast());
    let block = [0_u8; 65536];
    for _ in 0..1025 {
        encoder.write_all(&block)?;
    }
    assert!(verify_archive(&encoder.finish()?)
        .expect_err("size limit")
        .contains("bounded"));
    let mut files = archive_files()?;
    files
        .iter_mut()
        .find(|(name, _)| name == "bundle.json")
        .ok_or("manifest")?
        .1 = b"{\"schema_version\":1,\"schema_version\":1}".to_vec();
    assert!(verify_archive(&make_archive(&files)?)
        .expect_err("duplicate")
        .contains("duplicate"));
    Ok(())
}
#[cfg(unix)]
#[test]
fn symlink_and_hardlinked_state_bundle_files_are_rejected() -> Result {
    use std::os::unix::fs::symlink;
    let root = tempfile::tempdir()?;
    let outside = tempfile::tempdir()?;
    symlink(outside.path(), root.path().join(".gameskills"))?;
    assert!(call(root.path(), "setup", &[]).is_err());
    fs::remove_file(root.path().join(".gameskills"))?;
    export(root.path(), "export", &["gameskills"])?;
    symlink(root.path().join("export"), root.path().join("alias"))?;
    assert!(call(root.path(), "setup", &["--bundle", "alias"]).is_err());
    fs::write(outside.path().join("file"), "user")?;
    fs::hard_link(
        outside.path().join("file"),
        root.path().join("gameskills.toml"),
    )?;
    assert!(call(root.path(), "setup", &[]).is_err());
    assert_eq!(fs::read_to_string(outside.path().join("file"))?, "user");
    Ok(())
}

fn put(value: &mut Value, path: &str, new: Value) {
    *value.pointer_mut(path).expect("fixture field") = new;
}

fn at<'a>(value: &'a Value, path: &str) -> &'a Value {
    value.pointer(path).expect("fixture field")
}

fn git(root: &Path, args: &[&str]) -> Result<String> {
    let output = std::process::Command::new("git")
        .current_dir(root)
        .args(args)
        .env("GIT_CONFIG_NOSYSTEM", "1")
        .output()?;
    if !output.status.success() {
        return Err(String::from_utf8_lossy(&output.stderr).into_owned().into());
    }
    Ok(String::from_utf8(output.stdout)?.trim().to_owned())
}
fn source_fixture(root: &Path) -> Result<String> {
    fs::create_dir(root)?;
    for (name, bytes) in archive_files()? {
        if name == "bundle.json" {
            continue;
        }
        let path = root.join(name);
        fs::create_dir_all(path.parent().ok_or("parent")?)?;
        fs::write(path, bytes)?;
    }
    let bundle = root.join("tools/gameskills-cli/bundle");
    fs::create_dir_all(&bundle)?;
    fs::write(
        bundle.join("bundle.json"),
        include_bytes!("../bundle/bundle.json"),
    )?;
    fs::write(
        bundle.join("instructions.tar.gz"),
        include_bytes!("../bundle/instructions.tar.gz"),
    )?;
    fs::write(root.join(".gitignore"), "*.log\n")?;
    git(root, &["init", "-q"])?;
    git(root, &["config", "user.name", "Fixture"])?;
    git(root, &["config", "user.email", "fixture@example.invalid"])?;
    git(root, &["config", "commit.gpgsign", "false"])?;
    git(root, &["config", "gc.auto", "0"])?;
    git(root, &["add", "."])?;
    git(root, &["commit", "-qm", "Prepared snapshot"])?;
    git(root, &["rev-parse", "HEAD"])
}
#[test]
fn pinned_source_export_checks_full_commit_tags_cleanliness_and_prepared_bytes() -> Result {
    let root = tempfile::tempdir()?;
    let source = root.path().join("source");
    let commit = source_fixture(&source)?;
    git(&source, &["tag", "v0.1-fixture"])?;
    git(&source, &["branch", "release-looking"])?;
    for reference in [
        "HEAD",
        "main",
        "latest",
        "release-looking",
        commit.get(..8).ok_or("prefix")?,
    ] {
        assert!(call(
            root.path(),
            "bundle",
            &[
                "--source",
                "source",
                "--revision",
                reference,
                "--out",
                "bad"
            ]
        )
        .is_err());
        assert!(!root.path().join("bad").exists());
    }
    let result = call(
        root.path(),
        "bundle",
        &[
            "--source",
            "source",
            "--revision",
            "v0.1-fixture",
            "--out",
            "tagged",
        ],
    )?;
    assert_eq!(
        result.get("content_sha256"),
        verify_archive(include_bytes!("../bundle/instructions.tar.gz"))?
            .manifest
            .get("content_sha256")
    );
    fs::write(
        source.join("plugins/gameskills/local-output.log"),
        "ignored local output",
    )?;
    call(
        root.path(),
        "bundle",
        &[
            "--source",
            "source",
            "--revision",
            &commit,
            "--out",
            "ignored",
        ],
    )?;
    assert!(!root
        .path()
        .join("ignored/plugins/gameskills/local-output.log")
        .exists());
    let skill = source.join("plugins/gameskills/skills/plan/SKILL.md");
    fs::OpenOptions::new()
        .append(true)
        .open(&skill)?
        .write_all(b"\nchanged\n")?;
    assert!(call(
        root.path(),
        "bundle",
        &[
            "--source",
            "source",
            "--revision",
            &commit,
            "--out",
            "dirty"
        ]
    )
    .expect_err("dirty source")
    .contains("uncommitted"));
    git(&source, &["add", "."])?;
    git(
        &source,
        &["commit", "-qm", "Changed canonical instructions"],
    )?;
    let changed = git(&source, &["rev-parse", "HEAD"])?;
    assert!(call(
        root.path(),
        "bundle",
        &[
            "--source",
            "source",
            "--revision",
            &changed,
            "--out",
            "stale"
        ]
    )
    .expect_err("stale prepared snapshot")
    .contains("prepared snapshot differs"));
    assert!(call(
        root.path(),
        "bundle",
        &[
            "--source",
            "source",
            "--revision",
            &commit,
            "--out",
            "wrong-head"
        ]
    )
    .expect_err("head mismatch")
    .contains("does not match"));
    Ok(())
}
#[test]
fn raw_tar_links_longname_extensions_and_unsafe_paths_are_rejected() -> Result {
    for kind in [
        tar::EntryType::Symlink,
        tar::EntryType::Link,
        tar::EntryType::Directory,
    ] {
        let encoder = flate2::write::GzEncoder::new(Vec::new(), flate2::Compression::default());
        let mut archive = tar::Builder::new(encoder);
        let mut header = tar::Header::new_gnu();
        header.set_entry_type(kind);
        header.set_mode(0o644);
        header.set_size(0);
        header.set_path("plugins/gameskills/references/link.md")?;
        if kind != tar::EntryType::Directory {
            header.set_link_name("../../outside")?;
        }
        header.set_cksum();
        archive.append(&header, std::io::empty())?;
        assert!(verify_archive(&archive.into_inner()?.finish()?).is_err());
    }
    let files = vec![(
        format!("plugins/gameskills/references/{}.md", "x".repeat(200)),
        b"extension".to_vec(),
    )];
    assert!(verify_archive(&make_archive(&files)?)
        .expect_err("extension")
        .contains("extensions"));
    let mut files = archive_files()?;
    files.push((
        "plugins/unknown/references/file.md".into(),
        b"unknown".to_vec(),
    ));
    assert!(verify_archive(&make_archive(&files)?).is_err());
    Ok(())
}

#[test]
fn equivalent_package_orders_share_verified_bundle_without_rewriting_config() -> Result {
    let root = tempfile::tempdir()?;
    let text = "schema_version=1\npackages=[\"gameskills-ui\",\"gameskills\"]\n";
    fs::write(root.path().join("gameskills.toml"), text)?;
    let first = installed(root.path())?;
    call(root.path(), "status", &[])?;
    assert_eq!(
        fs::read_to_string(root.path().join("gameskills.toml"))?,
        text
    );
    let second = call(
        root.path(),
        "setup",
        &["--packages", "gameskills", "gameskills-ui", "--apply"],
    )?;
    assert_eq!(first.get("destination"), second.get("destination"));
    call(root.path(), "status", &[])?;
    Ok(())
}

#[test]
fn explicit_content_update_keeps_previous_pin_and_allows_rollback() -> Result {
    let root = tempfile::tempdir()?;
    let original = installed(root.path())?;
    let mut files = archive_files()?;
    let changed = files
        .iter_mut()
        .find(|(name, _)| name.ends_with("skills/plan/SKILL.md"))
        .ok_or("skill")?;
    changed
        .1
        .extend_from_slice(b"\nA compatible instruction correction.\n");
    let name = changed.0.clone();
    let digest = hash(&changed.1);
    let manifest = files
        .iter_mut()
        .find(|(name, _)| name == "bundle.json")
        .ok_or("manifest")?;
    let mut value: Value = serde_json::from_slice(&manifest.1)?;
    *value
        .get_mut("files")
        .and_then(|v| v.get_mut(&name))
        .ok_or("file hash")? = json!(digest);
    *value.get_mut("source_commit").ok_or("commit")? = json!("b".repeat(40));
    let mut identity = value.clone();
    identity
        .as_object_mut()
        .ok_or("object")?
        .remove("content_sha256");
    identity
        .as_object_mut()
        .ok_or("object")?
        .remove("source_commit");
    identity.sort_all_objects();
    let mut identity = serde_json::to_vec_pretty(&identity)?;
    identity.push(b'\n');
    *value.get_mut("content_sha256").ok_or("content")? = json!(hash(&identity));
    manifest.1 = serde_json::to_vec(&value)?;
    fs::write(root.path().join("update.tar.gz"), make_archive(&files)?)?;
    call(root.path(), "setup", &["--bundle", "update.tar.gz"])?;
    assert_eq!(
        call(root.path(), "status", &[])?.get("content_sha256"),
        original.get("content_sha256")
    );
    let updated = call(
        root.path(),
        "setup",
        &["--bundle", "update.tar.gz", "--apply"],
    )?;
    assert_ne!(
        updated.get("content_sha256"),
        original.get("content_sha256")
    );
    assert!(Path::new(
        original
            .get("destination")
            .and_then(Value::as_str)
            .ok_or("destination")?
    )
    .is_dir());
    assert_eq!(
        call(root.path(), "status", &[])?.get("content_sha256"),
        updated.get("content_sha256")
    );
    let rollback = installed(root.path())?;
    assert_eq!(
        rollback.get("content_sha256"),
        original.get("content_sha256")
    );
    assert!(Path::new(
        updated
            .get("destination")
            .and_then(Value::as_str)
            .ok_or("destination")?
    )
    .is_dir());
    Ok(())
}
#[test]
fn actual_cli_installs_and_validates_pinned_candidate() -> Result {
    let root = tempfile::tempdir()?;
    for args in [
        vec!["setup", "--apply"],
        vec!["status"],
        vec!["native", "codex", "--", "exec", "--ephemeral"],
    ] {
        let output = std::process::Command::new(env!("CARGO_BIN_EXE_gameskills"))
            .arg("--root")
            .arg(root.path())
            .args(args)
            .output()?;
        assert!(
            output.status.success(),
            "{}",
            String::from_utf8_lossy(&output.stdout)
        );
        let value: Value = serde_json::from_slice(&output.stdout)?;
        assert_eq!(value.get("ok"), Some(&json!(true)));
    }
    Ok(())
}
