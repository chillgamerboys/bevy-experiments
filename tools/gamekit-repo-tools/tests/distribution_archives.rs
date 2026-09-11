//! Actual Cargo artifact probes and adversarial extraction boundaries.

use gamekit_repo_tools::distribution::{archives_with_runner, inspect_archive, Case};
use std::error::Error;
use std::ffi::OsString;
use std::io::Write;
use std::path::Path;
use std::process::Command;

fn write(root: &Path, relative: &str, bytes: impl AsRef<[u8]>) -> Result<(), Box<dyn Error>> {
    let path = root.join(relative);
    std::fs::create_dir_all(path.parent().ok_or("no parent")?)?;
    std::fs::write(path, bytes)?;
    Ok(())
}

fn cargo(cwd: &Path, arguments: &[OsString]) -> Result<String, String> {
    let executable = std::env::var_os("CARGO").unwrap_or_else(|| "cargo".into());
    let output = Command::new(executable)
        .args(arguments)
        .arg("--offline")
        .current_dir(cwd)
        .output()
        .map_err(|error| error.to_string())?;
    if !output.status.success() {
        return Err(format!(
            "Cargo {arguments:?}: {}",
            String::from_utf8_lossy(&output.stderr)
        ));
    }
    String::from_utf8(output.stdout).map_err(|error| error.to_string())
}

fn fixture() -> Result<tempfile::TempDir, Box<dyn Error>> {
    let directory = tempfile::Builder::new()
        .prefix("archive $(literal) fixture ")
        .tempdir()?;
    let root = directory.path();
    write(
        root,
        "Cargo.toml",
        r#"[workspace]
resolver = "3"
members = ["crates/*"]
[workspace.package]
version = "0.1.0"
edition = "2021"
publish = false
[workspace.dependencies]
archive_probe_helper = { path = "crates/helper" }
"#,
    )?;
    write(
        root,
        "crates/helper/Cargo.toml",
        r#"[package]
name = "archive_probe_helper"
version.workspace = true
edition.workspace = true
publish.workspace = true
"#,
    )?;
    write(
        root,
        "crates/helper/src/lib.rs",
        "pub fn answer() -> u32 { 42 }\n",
    )?;
    write(
        root,
        "crates/bevy_gamekit/Cargo.toml",
        r#"[package]
name = "bevy-gamekit"
version.workspace = true
edition.workspace = true
publish.workspace = true
include = ["src/**", "Cargo.toml"]
[dependencies]
archive_probe_helper = { workspace = true, optional = true }
[features]
default = []
hex = ["dep:archive_probe_helper"]
turns-serde = []
testing-ui = []
direct = []
mdns = []
tailscale-cli = []
"#,
    )?;
    write(
        root,
        "crates/bevy_gamekit/src/lib.rs",
        "#[cfg(feature = \"hex\")]\npub fn answer() -> u32 { archive_probe_helper::answer() }\n",
    )?;
    write(root, "crates/bevy_gamekit/excluded.rs", "not selected\n")?;
    cargo(root, &["generate-lockfile".into()])?;
    Ok(directory)
}

#[test]
fn real_two_crate_archives_compile_after_staging_is_removed() -> Result<(), Box<dyn Error>> {
    let fixture = fixture()?;
    let root = fixture.path();
    let original = std::fs::read(root.join("Cargo.toml"))?;
    let original_facade = std::fs::read(root.join("crates/bevy_gamekit/Cargo.toml"))?;
    let mut staged = None;
    let mut tested = false;
    let report = archives_with_runner(root, Case::Empty, |cwd, args| {
        if args.first().is_some_and(|arg| arg == "package")
            && args.iter().any(|arg| arg == "--workspace")
        {
            assert!(args.iter().any(|arg| arg == "--exclude-lockfile"));
            assert!(args.iter().any(|arg| arg == "--no-verify"));
            let manifest = std::fs::read_to_string(cwd.join("Cargo.toml"))
                .map_err(|error| error.to_string())?;
            let parsed: toml::Value =
                toml::from_str(&manifest).map_err(|error| error.to_string())?;
            assert_eq!(
                parsed
                    .get("workspace")
                    .and_then(|v| v.get("package"))
                    .and_then(|v| v.get("publish"))
                    .and_then(toml::Value::as_bool),
                Some(false)
            );
            staged = Some(cwd.to_path_buf());
        }
        if args
            .first()
            .is_some_and(|arg| arg == "tree" || arg == "test")
        {
            assert!(staged.as_ref().is_some_and(|path| !path.exists()));
            let document: toml::Value = toml::from_str(
                &std::fs::read_to_string(cwd.join("Cargo.toml"))
                    .map_err(|error| error.to_string())?,
            )
            .map_err(|error| error.to_string())?;
            let patches = document
                .get("patch")
                .and_then(|v| v.get("crates-io"))
                .and_then(toml::Value::as_table)
                .ok_or("missing patches")?;
            assert_eq!(patches.len(), 2);
            for (package, patch) in patches {
                let path = Path::new(
                    patch
                        .get("path")
                        .and_then(toml::Value::as_str)
                        .ok_or("missing patch path")?,
                );
                assert!(path.starts_with(
                    cwd.parent()
                        .ok_or("consumer has no parent")?
                        .join("extracted")
                ));
                let source = std::fs::read_to_string(path.join("Cargo.toml"))
                    .map_err(|error| error.to_string())?;
                let archive: toml::Value =
                    toml::from_str(&source).map_err(|error| error.to_string())?;
                assert!(archive.get("workspace").is_none());
                assert_eq!(
                    archive
                        .get("package")
                        .and_then(|v| v.get("publish"))
                        .and_then(toml::Value::as_bool),
                    Some(false)
                );
                assert!(!path.join("Cargo.lock").exists());
                assert!(!path.join("excluded.rs").exists());
                if package == "bevy-gamekit" {
                    let dep = archive
                        .get("dependencies")
                        .and_then(|v| v.get("archive_probe_helper"))
                        .ok_or("missing normalized dependency")?;
                    assert_eq!(
                        dep.get("version").and_then(toml::Value::as_str),
                        Some("=0.1.0")
                    );
                    assert!(dep.get("path").is_none());
                    assert!(dep.get("workspace").is_none());
                }
            }
        }
        if args.first().is_some_and(|arg| arg == "test") {
            // Enable the fixture's actual two-crate API without enabling the full
            // production probe's pure/UI/network source blocks.
            std::fs::write(
                cwd.join("src/lib.rs"),
                "#[test] fn extracted_dependency() { assert_eq!(bevy_gamekit::answer(), 42); }\n",
            )
            .map_err(|error| error.to_string())?;
            let mut args = args.to_vec();
            args.extend(["--features".into(), "bevy_gamekit/hex".into()]);
            tested = true;
            cargo(cwd, &args)
        } else {
            cargo(cwd, args)
        }
    })?;
    assert!(tested);
    assert_eq!(report.archives.len(), 2);
    assert_eq!(report.cases.len(), 1);
    assert_eq!(report.staged_version_requirements.len(), 1);
    assert!(report.lockfile_omitted);
    assert!(!report.registry_resolution_verified);
    for archive in report.archives {
        assert_eq!(archive.sha256.len(), 64);
        assert_eq!(
            archive.files,
            ["Cargo.toml", "Cargo.toml.orig", "src/lib.rs"]
        );
    }
    assert_eq!(std::fs::read(root.join("Cargo.toml"))?, original);
    assert_eq!(
        std::fs::read(root.join("crates/bevy_gamekit/Cargo.toml"))?,
        original_facade
    );
    assert!(staged.is_some_and(|path| path.parent().is_some_and(|parent| !parent.exists())));
    Ok(())
}

const MANIFEST: &str = "[package]\nname='probe'\nversion='0.1.0'\nedition='2021'\n";

fn artifact(
    root: &Path,
    entries: &[(&str, &[u8], tar::EntryType)],
) -> Result<std::path::PathBuf, Box<dyn Error>> {
    let mut builder = tar::Builder::new(Vec::new());
    for (name, content, kind) in entries {
        let mut header = tar::Header::new_ustar();
        // Set raw bytes so adversarial paths reach the extractor, not the writer's policy.
        header
            .as_mut_bytes()
            .get_mut(..name.len())
            .ok_or("path too long")?
            .copy_from_slice(name.as_bytes());
        header.set_size(content.len() as u64);
        header.set_mode(0o644);
        header.set_entry_type(*kind);
        if kind.is_symlink() || kind.is_hard_link() {
            header.set_link_name("outside")?;
        }
        header.set_cksum();
        builder.append(&header, *content)?;
    }
    let tar = builder.into_inner()?;
    let path = root.join("probe.crate");
    let mut gzip = flate2::write::GzEncoder::new(Vec::new(), flate2::Compression::default());
    gzip.write_all(&tar)?;
    std::fs::write(&path, gzip.finish()?)?;
    Ok(path)
}

#[test]
fn archive_inspection_rejects_paths_links_duplicates_and_wrong_identities(
) -> Result<(), Box<dyn Error>> {
    for (name, kind) in [
        ("probe-0.1.0/../outside", tar::EntryType::Regular),
        ("probe-0.1.0/src/../../outside", tar::EntryType::Regular),
        ("probe-0.1.0/src\\lib.rs", tar::EntryType::Regular),
        ("probe-0.1.0/C:/outside", tar::EntryType::Regular),
        ("probe-0.1.0//outside", tar::EntryType::Regular),
        ("/probe-0.1.0/outside", tar::EntryType::Regular),
        ("different-0.1.0/src/lib.rs", tar::EntryType::Regular),
        ("probe-0.1.0/src/lib.rs", tar::EntryType::Regular),
        ("probe-0.1.0/link", tar::EntryType::Symlink),
        ("probe-0.1.0/hard", tar::EntryType::Link),
        ("probe-0.1.0/device", tar::EntryType::Char),
        ("probe-0.1.0/fifo", tar::EntryType::Fifo),
    ] {
        let directory = tempfile::tempdir()?;
        let path = artifact(
            directory.path(),
            &[
                (
                    "probe-0.1.0/Cargo.toml",
                    MANIFEST.as_bytes(),
                    tar::EntryType::Regular,
                ),
                (
                    "probe-0.1.0/src/lib.rs",
                    b"// source",
                    tar::EntryType::Regular,
                ),
                (name, b"", kind),
            ],
        )?;
        assert!(
            inspect_archive(&path, "probe", "0.1.0", &directory.path().join("extract")).is_err(),
            "{name}"
        );
        assert!(!directory.path().join("outside").exists());
    }
    Ok(())
}

#[test]
fn archive_inspection_requires_complete_standalone_normalized_manifests(
) -> Result<(), Box<dyn Error>> {
    for manifest in [
        "[package]\nname='other'\nversion='0.1.0'\n".to_owned(),
        "[package]\nname='probe'\nversion.workspace=true\n".to_owned(),
        format!("{MANIFEST}\n[workspace]\nmembers=[]\n"),
        format!("{MANIFEST}\n[patch.crates-io]\na={{path='outside'}}\n"),
        format!("{MANIFEST}\n[dependencies]\na={{path='../outside', version='1'}}\n"),
        format!("{MANIFEST}\n[target.'cfg(unix)'.build-dependencies]\na={{workspace=true}}\n"),
        format!("{MANIFEST}\n[lints]\nworkspace=true\n"),
        format!("{MANIFEST}\n[lib]\npath='../outside.rs'\n"),
        format!("{MANIFEST}\n[[example]]\nname='missing'\npath='examples/missing.rs'\n"),
    ] {
        let directory = tempfile::tempdir()?;
        let path = artifact(
            directory.path(),
            &[
                (
                    "probe-0.1.0/Cargo.toml",
                    manifest.as_bytes(),
                    tar::EntryType::Regular,
                ),
                (
                    "probe-0.1.0/src/lib.rs",
                    b"// source",
                    tar::EntryType::Regular,
                ),
            ],
        )?;
        assert!(
            inspect_archive(&path, "probe", "0.1.0", &directory.path().join("extract")).is_err(),
            "{manifest}"
        );
    }
    let directory = tempfile::tempdir()?;
    let path = artifact(
        directory.path(),
        &[(
            "probe-0.1.0/Cargo.toml",
            MANIFEST.as_bytes(),
            tar::EntryType::Regular,
        )],
    )?;
    assert!(inspect_archive(&path, "probe", "0.1.0", &directory.path().join("extract")).is_err());
    Ok(())
}

#[test]
fn archive_inspection_rejects_corrupt_and_concatenated_gzip() -> Result<(), Box<dyn Error>> {
    for fault in ["checksum", "truncated", "trailing", "concatenated"] {
        let directory = tempfile::tempdir()?;
        let path = artifact(
            directory.path(),
            &[
                (
                    "probe-0.1.0/Cargo.toml",
                    MANIFEST.as_bytes(),
                    tar::EntryType::Regular,
                ),
                (
                    "probe-0.1.0/src/lib.rs",
                    b"// source",
                    tar::EntryType::Regular,
                ),
            ],
        )?;
        let mut bytes = std::fs::read(&path)?;
        match fault {
            "checksum" => {
                let offset = bytes.len().checked_sub(8).ok_or("gzip too short")?;
                *bytes.get_mut(offset).ok_or("no checksum byte")? ^= 1;
            }
            "truncated" => bytes.truncate(bytes.len().checked_sub(4).ok_or("gzip too short")?),
            "trailing" => bytes.extend_from_slice(b"ignored junk"),
            "concatenated" => bytes.extend(bytes.clone()),
            _ => return Err("unknown gzip fault".into()),
        }
        std::fs::write(&path, bytes)?;
        assert!(
            inspect_archive(&path, "probe", "0.1.0", &directory.path().join("extract")).is_err(),
            "{fault}"
        );
    }
    Ok(())
}

#[test]
fn changed_archive_sources_and_sibling_requirements_fail_before_consumption(
) -> Result<(), Box<dyn Error>> {
    use std::io::Read;
    for mutation in ["source", "version"] {
        let fixture = fixture()?;
        let error = archives_with_runner(fixture.path(), Case::Empty, |cwd, args| {
            let output = cargo(cwd, args)?;
            if args.iter().any(|arg| arg == "--workspace") {
                let target = args.last().ok_or("missing package target")?;
                let path = Path::new(target).join("package/bevy-gamekit-0.1.0.crate");
                let gzip = flate2::read::GzDecoder::new(
                    std::fs::File::open(&path).map_err(|error| error.to_string())?,
                );
                let mut archive = tar::Archive::new(gzip);
                let mut entries = Vec::new();
                for entry in archive.entries().map_err(|error| error.to_string())? {
                    let mut entry = entry.map_err(|error| error.to_string())?;
                    let name = String::from_utf8(entry.path_bytes().to_vec())
                        .map_err(|error| error.to_string())?;
                    let mut content = Vec::new();
                    entry
                        .read_to_end(&mut content)
                        .map_err(|error| error.to_string())?;
                    if mutation == "source" && name.ends_with("/src/lib.rs") {
                        content.extend_from_slice(b"// changed artifact\n");
                    }
                    if mutation == "version" && name.ends_with("/Cargo.toml") {
                        content = String::from_utf8(content)
                            .map_err(|error| error.to_string())?
                            .replace("=0.1.0", "=9.0.0")
                            .into_bytes();
                    }
                    entries.push((name, content, tar::EntryType::Regular));
                }
                let references: Vec<_> = entries
                    .iter()
                    .map(|(name, content, kind)| (name.as_str(), content.as_slice(), *kind))
                    .collect();
                let altered = artifact(cwd, &references).map_err(|error| error.to_string())?;
                std::fs::copy(altered, path).map_err(|error| error.to_string())?;
            }
            if args
                .first()
                .is_some_and(|arg| arg == "tree" || arg == "test")
            {
                return Err("must reject before consumer commands".into());
            }
            Ok(output)
        })
        .err()
        .ok_or("altered artifact passed")?;
        let expected = if mutation == "source" {
            "Cargo changed selected source"
        } else {
            "normalized sibling"
        };
        assert!(error.contains(expected), "{mutation}: {error}");
    }
    Ok(())
}

#[test]
fn cargo_failure_is_reported_and_temporary_sources_are_cleaned() -> Result<(), Box<dyn Error>> {
    let fixture = fixture()?;
    let mut temporary = None;
    let result = archives_with_runner(fixture.path(), Case::Empty, |cwd, args| {
        if args.iter().any(|arg| arg == "--workspace") {
            temporary = cwd.parent().map(Path::to_path_buf);
            Err("real packaging failure fixture".into())
        } else {
            cargo(cwd, args)
        }
    });
    assert_eq!(
        result.err().as_deref(),
        Some("real packaging failure fixture")
    );
    assert!(temporary.is_some_and(|path| !path.exists()));
    Ok(())
}

#[test]
fn inspection_rejects_nonfiles_and_existing_destinations() -> Result<(), Box<dyn Error>> {
    let directory = tempfile::tempdir()?;
    let root = directory.path().canonicalize()?;
    let path = artifact(
        &root,
        &[
            (
                "probe-0.1.0/Cargo.toml",
                MANIFEST.as_bytes(),
                tar::EntryType::Regular,
            ),
            (
                "probe-0.1.0/src/lib.rs",
                b"// source",
                tar::EntryType::Regular,
            ),
        ],
    )?;
    assert!(inspect_archive(&root, "probe", "0.1.0", &root.join("extract")).is_err());
    write(&root, "occupied", b"keep me")?;
    assert!(inspect_archive(&path, "probe", "0.1.0", &root.join("occupied")).is_err());
    assert_eq!(std::fs::read(root.join("occupied"))?, b"keep me");
    #[cfg(unix)]
    {
        use std::os::unix::fs::symlink;
        symlink(&path, root.join("linked.crate"))?;
        assert!(inspect_archive(
            &root.join("linked.crate"),
            "probe",
            "0.1.0",
            &root.join("extract")
        )
        .is_err());
        symlink(root.join("missing-target"), root.join("dangling"))?;
        assert!(inspect_archive(&path, "probe", "0.1.0", &root.join("dangling")).is_err());
        symlink(&root, root.join("linked-parent"))?;
        assert!(
            inspect_archive(&path, "probe", "0.1.0", &root.join("linked-parent/extract")).is_err()
        );
        let status = Command::new("mkfifo")
            .arg(root.join("pipe.crate"))
            .status()?;
        assert!(status.success());
        assert!(inspect_archive(
            &root.join("pipe.crate"),
            "probe",
            "0.1.0",
            &root.join("extract")
        )
        .is_err());
    }
    Ok(())
}
