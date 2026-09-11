//! Real Git provenance, reproducible payloads and drift rejection.

use gamekit_repo_tools::{bundle, catalog::EXPECTED_SKILLS};
use serde_json::{json, Value};
use std::{collections::BTreeMap, error::Error, io::Read, path::Path, process::Command};

type TestResult = Result<(), Box<dyn Error>>;

fn write(root: &Path, path: &str, contents: impl AsRef<[u8]>) -> TestResult {
    let path = root.join(path);
    std::fs::create_dir_all(path.parent().ok_or("parent")?)?;
    std::fs::write(path, contents)?;
    Ok(())
}
fn git(root: &Path, args: &[&str]) -> Result<String, Box<dyn Error>> {
    let output = Command::new("git").current_dir(root).args(args).output()?;
    assert!(
        output.status.success(),
        "{args:?}: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    Ok(String::from_utf8(output.stdout)?.trim().into())
}
fn commit(root: &Path) -> Result<String, Box<dyn Error>> {
    git(root, &["add", "."])?;
    git(root, &["commit", "-qm", "fixture"])?;
    git(root, &["rev-parse", "HEAD"])
}
fn fixture() -> Result<(tempfile::TempDir, String), Box<dyn Error>> {
    let temp = tempfile::Builder::new()
        .prefix("bundle $(literal) ")
        .tempdir()?;
    let root = temp.path();
    git(root, &["init", "-q"])?;
    git(root, &["config", "user.name", "Bundle test"])?;
    git(root, &["config", "user.email", "bundle@example.invalid"])?;
    git(root, &["config", "commit.gpgsign", "false"])?;
    git(root, &["config", "core.autocrlf", "false"])?;
    write(
        root,
        "tools/gamekit-repo-tools/bundle-compatibility.json",
        include_str!("../bundle-compatibility.json"),
    )?;
    for (package, skills) in EXPECTED_SKILLS {
        for client in ["codex", "claude"] {
            write(
                root,
                &format!("plugins/{package}/.{client}-plugin/plugin.json"),
                format!("{{\"name\":{package:?}}}\n"),
            )?;
        }
        for skill in skills.iter() {
            write(
                root,
                &format!("plugins/{package}/skills/{skill}/SKILL.md"),
                format!("# {skill}\n"),
            )?;
        }
        write(
            root,
            &format!("plugins/{package}/references/context.md"),
            "# Context\n",
        )?;
    }
    write(
        root,
        "plugins/gameskills/catalog.json",
        "{\"version\":\"0.1.0-dev.1\"}\n",
    )?;
    write(
        root,
        "plugins/gameskills/scripts/gameskills.py",
        "print('not bundled')\n",
    )?;
    write(root, "plugins/gameskills/runtime/old.py", "# not bundled\n")?;
    let revision = commit(root)?;
    Ok((temp, revision))
}
fn payload(root: &Path) -> Result<BTreeMap<String, Vec<u8>>, Box<dyn Error>> {
    let gzip = flate2::read::GzDecoder::new(std::fs::File::open(
        root.join(bundle::OUTPUT).join("instructions.tar.gz"),
    )?);
    let mut tar = tar::Archive::new(gzip);
    let mut files = BTreeMap::new();
    for entry in tar.entries()? {
        let mut entry = entry?;
        assert!(entry.header().entry_type().is_file());
        assert_eq!(entry.header().mtime()?, 0);
        assert_eq!(entry.header().uid()?, 0);
        assert_eq!(entry.header().mode()?, 0o644);
        let path = entry.path()?.to_str().ok_or("UTF-8 path")?.to_owned();
        let mut bytes = Vec::new();
        entry.read_to_end(&mut bytes)?;
        assert!(files.insert(path, bytes).is_none());
    }
    Ok(files)
}

#[test]
fn repeated_preparation_and_export_share_exact_payload_and_manifest() -> TestResult {
    let (temp, revision) = fixture()?;
    let root = temp.path();
    let first = bundle::prepare(root, &revision)?;
    assert_eq!(first, bundle::prepare(root, &revision)?);
    let files = payload(root)?;
    assert!(!files.keys().any(|path| path.ends_with(".py")));
    assert_eq!(
        files.get("bundle.json"),
        Some(&std::fs::read(
            root.join(bundle::OUTPUT).join("bundle.json")
        )?)
    );
    for (package, _) in EXPECTED_SKILLS {
        for client in ["claude", "codex"] {
            let path = format!("plugins/{package}/.{client}-plugin/plugin.json");
            assert_eq!(files.get(&path), Some(&std::fs::read(root.join(&path))?));
        }
    }
    let manifest: Value = serde_json::from_slice(files.get("bundle.json").ok_or("manifest")?)?;
    assert_eq!(
        manifest.get("default_packages"),
        Some(&json!(["gameskills"]))
    );
    let exported = root.join("standalone.tar.gz");
    assert_eq!(first, bundle::export(root, &exported)?);
    let expected = std::fs::read(root.join(bundle::OUTPUT).join("instructions.tar.gz"))?;
    assert_eq!(std::fs::read(&exported)?, expected);
    assert!(bundle::export(root, &exported).is_err());
    assert_eq!(std::fs::read(&exported)?, expected);
    Ok(())
}

#[test]
fn unrelated_commits_keep_pin_valid_but_changed_inputs_require_new_pin() -> TestResult {
    let (temp, revision) = fixture()?;
    let root = temp.path();
    let original = bundle::prepare(root, &revision)?;
    write(root, "README.md", "Unrelated\n")?;
    commit(root)?;
    assert_eq!(original, bundle::check(root)?);
    write(
        root,
        "plugins/gameskills/references/context.md",
        "Changed\n",
    )?;
    assert!(bundle::check(root).is_err());
    let current = commit(root)?;
    assert!(bundle::check(root).is_err());
    assert!(bundle::prepare(root, &revision).is_err());
    let changed = bundle::prepare(root, &current)?;
    assert_ne!(
        original.get("content_sha256"),
        changed.get("content_sha256")
    );
    Ok(())
}

#[test]
fn generated_corruption_extra_files_and_unpinned_revisions_are_rejected() -> TestResult {
    let (temp, revision) = fixture()?;
    let root = temp.path();
    for invalid in ["HEAD", "main", "--help", "12345"] {
        assert!(bundle::prepare(root, invalid).is_err());
    }
    bundle::prepare(root, &revision)?;
    write(
        root,
        &format!("{}/instructions.tar.gz", bundle::OUTPUT),
        b"corrupt",
    )?;
    assert!(bundle::check(root).is_err());
    bundle::prepare(root, &revision)?;
    write(root, &format!("{}/extra.txt", bundle::OUTPUT), "extra")?;
    assert!(bundle::check(root).is_err());
    std::fs::remove_file(root.join(bundle::OUTPUT).join("extra.txt"))?;
    let path = root.join(bundle::OUTPUT).join("bundle.json");
    let mut manifest: Value = serde_json::from_slice(&std::fs::read(&path)?)?;
    manifest
        .as_object_mut()
        .ok_or("manifest object")?
        .insert("content_sha256".into(), "forged".into());
    std::fs::write(path, serde_json::to_vec(&manifest)?)?;
    assert!(bundle::check(root).is_err());
    Ok(())
}

#[test]
fn missing_instruction_unsupported_asset_and_bad_compatibility_fail() -> TestResult {
    for (path, bytes) in [
        (
            "plugins/gameskills/skills/plan/code.py",
            "# forbidden asset",
        ),
        (
            "tools/gamekit-repo-tools/bundle-compatibility.json",
            "{\"schema_version\":true}",
        ),
    ] {
        let (temp, _) = fixture()?;
        write(temp.path(), path, bytes)?;
        let revision = commit(temp.path())?;
        assert!(bundle::prepare(temp.path(), &revision).is_err());
        assert!(!temp.path().join(bundle::OUTPUT).exists());
    }
    let (temp, _) = fixture()?;
    std::fs::remove_file(temp.path().join("plugins/gameskills/skills/plan/SKILL.md"))?;
    let revision = commit(temp.path())?;
    assert!(bundle::prepare(temp.path(), &revision).is_err());
    Ok(())
}

#[cfg(unix)]
#[test]
fn symlink_sources_and_outputs_are_never_followed() -> TestResult {
    use std::os::unix::fs::symlink;
    let (temp, revision) = fixture()?;
    let root = temp.path();
    let outside = tempfile::tempdir()?;
    std::fs::create_dir_all(root.join("tools/gameskills-cli"))?;
    symlink(outside.path(), root.join(bundle::OUTPUT))?;
    assert!(bundle::prepare(root, &revision).is_err());
    assert_eq!(std::fs::read_dir(outside.path())?.count(), 0);
    std::fs::remove_file(root.join(bundle::OUTPUT))?;
    let source = root.join("plugins/gameskills/references/context.md");
    std::fs::remove_file(&source)?;
    symlink("../skills/plan/SKILL.md", source)?;
    let revision = commit(root)?;
    assert!(bundle::prepare(root, &revision).is_err());
    Ok(())
}

#[test]
fn checkout_line_endings_do_not_change_canonical_blob_identity() -> TestResult {
    let (temp, revision) = fixture()?;
    let root = temp.path();
    let original = bundle::prepare(root, &revision)?;
    git(root, &["config", "core.autocrlf", "true"])?;
    let path = root.join("plugins/gameskills/references/context.md");
    let bytes = std::fs::read_to_string(&path)?.replace('\n', "\r\n");
    std::fs::write(path, bytes)?;
    assert_eq!(original, bundle::check(root)?);
    Ok(())
}
