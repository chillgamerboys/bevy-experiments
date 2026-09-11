//! Deterministic, preparation-only instruction artifacts from committed canonical sources.

use crate::catalog::EXPECTED_SKILLS;
use crate::support::{parse_json, portable_relative, read_json};
use flate2::{Compression, GzBuilder};
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use std::collections::BTreeMap;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::Command;

const INPUT: &str = "tools/gamekit-repo-tools/bundle-compatibility.json";
/// Generated Cargo-local payload directory; never an installation.
pub const OUTPUT: &str = "tools/gameskills-cli/bundle";

fn hash(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}

fn git(root: &Path, args: &[&str]) -> Result<Vec<u8>, String> {
    let output = Command::new("git")
        .args(args)
        .current_dir(root)
        .output()
        .map_err(|error| format!("git {args:?}: {error}"))?;
    if !output.status.success() {
        return Err(format!(
            "git {args:?}: {}",
            String::from_utf8_lossy(&output.stderr)
        ));
    }
    Ok(output.stdout)
}

fn text(bytes: &[u8]) -> Result<&str, String> {
    std::str::from_utf8(bytes).map_err(|error| error.to_string())
}

fn selected(path: &str) -> Result<bool, String> {
    if path == INPUT {
        return Ok(true);
    }
    for (package, _) in EXPECTED_SKILLS {
        if let Some(relative) = path.strip_prefix(&format!("plugins/{package}/")) {
            if matches!(
                relative,
                ".codex-plugin/plugin.json" | ".claude-plugin/plugin.json"
            ) || package == &"gameskills" && relative == "catalog.json"
            {
                return Ok(true);
            }
            if relative.starts_with("skills/") || relative.starts_with("references/") {
                if !relative.ends_with(".md") {
                    return Err(format!("unsupported instruction asset {path}; extend the artifact contract explicitly"));
                }
                return Ok(true);
            }
        }
    }
    Ok(false)
}

fn inputs(root: &Path, revision: &str) -> Result<BTreeMap<String, Vec<u8>>, String> {
    let mut files = BTreeMap::new();
    let listing = git(root, &["ls-tree", "-rz", revision, "--", "plugins/", INPUT])?;
    for entry in listing
        .split(|byte| *byte == 0)
        .filter(|entry| !entry.is_empty())
    {
        let (metadata, path) = text(entry)?
            .split_once('\t')
            .ok_or("invalid Git tree record")?;
        if !selected(path)? {
            continue;
        }
        if !portable_relative(path) || path.contains(['\n', '\r']) {
            return Err(format!("nonportable instruction path {path:?}"));
        }
        let fields: Vec<_> = metadata.split(' ').collect();
        if fields.first() != Some(&"100644") || fields.get(1) != Some(&"blob") {
            return Err(format!(
                "instruction input must be an ordinary nonexecutable Git blob: {path}"
            ));
        }
        let object = fields.get(2).ok_or("missing Git object")?;
        files.insert(path.into(), git(root, &["cat-file", "blob", object])?);
    }
    for (package, skills) in EXPECTED_SKILLS {
        for required in [".codex-plugin/plugin.json", ".claude-plugin/plugin.json"]
            .into_iter()
            .map(str::to_owned)
            .chain(
                skills
                    .iter()
                    .map(|skill| format!("skills/{skill}/SKILL.md")),
            )
        {
            if !files.contains_key(&format!("plugins/{package}/{required}")) {
                return Err(format!("missing canonical input: {package}/{required}"));
            }
        }
    }
    if !files.contains_key(INPUT) || !files.contains_key("plugins/gameskills/catalog.json") {
        return Err("missing catalog or bundle compatibility input".into());
    }
    Ok(files)
}

fn ensure_current(root: &Path, expected: &BTreeMap<String, Vec<u8>>) -> Result<(), String> {
    if inputs(root, "HEAD")? != *expected {
        return Err(
            "pinned instruction inputs differ from HEAD; prepare from the new committed inputs"
                .into(),
        );
    }
    // Check only canonical inputs. Unrelated edits and generated outputs are allowed.
    let changed = git(
        root,
        &[
            "status",
            "--porcelain=v1",
            "-z",
            "--untracked-files=all",
            "--",
            "plugins/",
            INPUT,
        ],
    )?;
    if !changed.is_empty() {
        return Err("commit canonical plugin/compatibility changes before bundle preparation or verification".into());
    }
    for path in expected.keys() {
        let mut at = root.to_path_buf();
        for part in path.split('/') {
            at.push(part);
            if std::fs::symlink_metadata(&at)
                .map_err(|error| error.to_string())?
                .file_type()
                .is_symlink()
            {
                return Err(format!("symlink in instruction source: {path}"));
            }
        }
        // Git applies checkout filters (including CRLF) when checking cleanliness.
        // Artifact bytes always come from blobs, never platform checkout bytes.
    }
    Ok(())
}

fn compatibility(source: &[u8]) -> Result<Value, String> {
    let value = parse_json(text(source)?)?;
    let expected = [
        "schema_version",
        "cli_version_range",
        "config_schema",
        "queue_schema",
        "evidence_schema",
        "bevy",
        "gamekit",
        "activation",
    ];
    let object = value.as_object().ok_or("compatibility must be an object")?;
    if object.len() != expected.len() || expected.iter().any(|key| !object.contains_key(*key)) {
        return Err("unexpected or missing bundle compatibility fields".into());
    }
    for key in [
        "schema_version",
        "config_schema",
        "queue_schema",
        "evidence_schema",
    ] {
        if value.get(key).and_then(Value::as_u64) != Some(1) {
            return Err(format!("unsupported {key}; expected integer 1"));
        }
    }
    for key in ["cli_version_range", "bevy", "gamekit"] {
        if value
            .get(key)
            .and_then(Value::as_str)
            .is_none_or(|value| value.trim().is_empty())
        {
            return Err(format!("expected nonempty compatibility {key}"));
        }
    }
    if value.get("activation").and_then(Value::as_str) != Some("preparation-only") {
        return Err("R2 bundle activation must remain preparation-only".into());
    }
    semver::VersionReq::parse(
        value
            .get("cli_version_range")
            .and_then(Value::as_str)
            .ok_or("CLI range")?,
    )
    .map_err(|error| format!("invalid CLI version range: {error}"))?;
    Ok(value)
}

fn pretty(value: &Value) -> Result<Vec<u8>, String> {
    // Preserve identity even when a consumer enables serde_json/preserve_order.
    let mut canonical = value.clone();
    canonical.sort_all_objects();
    let mut bytes = serde_json::to_vec_pretty(&canonical).map_err(|error| error.to_string())?;
    bytes.push(b'\n');
    Ok(bytes)
}

fn archive(files: &BTreeMap<String, Vec<u8>>) -> Result<Vec<u8>, String> {
    let encoder = GzBuilder::new()
        .mtime(0)
        .operating_system(255)
        .write(Vec::new(), Compression::default());
    let mut tar = tar::Builder::new(encoder);
    for (path, bytes) in files {
        let mut header = tar::Header::new_gnu();
        header.set_mode(0o644);
        header.set_uid(0);
        header.set_gid(0);
        header.set_mtime(0);
        header.set_size(bytes.len() as u64);
        header.set_cksum();
        tar.append_data(&mut header, path, bytes.as_slice())
            .map_err(|error| error.to_string())?;
    }
    tar.into_inner()
        .map_err(|error| error.to_string())?
        .finish()
        .map_err(|error| error.to_string())
}

struct Prepared {
    manifest: Value,
    manifest_bytes: Vec<u8>,
    archive: Vec<u8>,
}

fn generate(root: &Path, revision: &str) -> Result<Prepared, String> {
    if !matches!(revision.len(), 40 | 64)
        || !revision
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
    {
        return Err("revision must be a full lowercase Git commit ID".into());
    }
    let resolved = git(
        root,
        &["rev-parse", "--verify", &format!("{revision}^{{commit}}")],
    )?;
    if text(&resolved)?.trim() != revision {
        return Err("revision must identify a commit directly".into());
    }
    let mut files = inputs(root, revision)?;
    ensure_current(root, &files)?;
    let compatibility = compatibility(&files.remove(INPUT).ok_or("missing compatibility")?)?;
    let catalog = parse_json(text(
        files
            .get("plugins/gameskills/catalog.json")
            .ok_or("missing catalog")?,
    )?)?;
    let version = catalog
        .get("version")
        .and_then(Value::as_str)
        .filter(|value| !value.is_empty())
        .ok_or("missing catalog version")?;
    let digests: BTreeMap<_, _> = files
        .iter()
        .map(|(path, bytes)| (path.clone(), hash(bytes)))
        .collect();
    let identity = json!({"format":"gameskills-instructions", "schema_version":1,"catalog_version":version,"compatibility":compatibility,"default_packages":["gameskills"],"files":digests});
    let digest = hash(&pretty(&identity)?);
    let mut manifest = identity;
    let object = manifest.as_object_mut().ok_or("manifest object")?;
    object.insert("source_commit".into(), revision.into());
    object.insert("content_sha256".into(), digest.into());
    let manifest_bytes = pretty(&manifest)?;
    files.insert("bundle.json".into(), manifest_bytes.clone());
    Ok(Prepared {
        manifest,
        manifest_bytes,
        archive: archive(&files)?,
    })
}

fn directory(root: &Path) -> Result<PathBuf, String> {
    let mut at = root.to_path_buf();
    for part in OUTPUT.split('/') {
        at.push(part);
        match std::fs::symlink_metadata(&at) {
            Ok(metadata) if !metadata.is_dir() || metadata.file_type().is_symlink() => {
                return Err(format!(
                    "expected ordinary output directory: {}",
                    at.display()
                ))
            }
            Ok(_) => {}
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
                std::fs::create_dir(&at).map_err(|error| error.to_string())?
            }
            Err(error) => return Err(error.to_string()),
        }
    }
    Ok(at)
}

fn write_atomic(path: &Path, bytes: &[u8]) -> Result<(), String> {
    match std::fs::symlink_metadata(path) {
        Ok(metadata) if !metadata.is_file() || metadata.file_type().is_symlink() => {
            return Err(format!("refusing nonordinary output {}", path.display()))
        }
        Ok(_) => {}
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
        Err(error) => return Err(error.to_string()),
    }
    let mut temporary = tempfile::NamedTempFile::new_in(path.parent().ok_or("output parent")?)
        .map_err(|error| error.to_string())?;
    temporary
        .write_all(bytes)
        .map_err(|error| error.to_string())?;
    temporary.persist(path).map_err(|error| error.to_string())?;
    Ok(())
}

fn report(prepared: &Prepared) -> Value {
    json!({"source_commit":prepared.manifest.get("source_commit"),"content_sha256":prepared.manifest.get("content_sha256"),"archive_sha256":hash(&prepared.archive),"files":prepared.manifest.get("files").and_then(Value::as_object).map(|files| files.len()),"packages":EXPECTED_SKILLS.len(),"core_skills":12,"optional_skills":9,"activation":"preparation-only"})
}

/// Prepare the fixed package-local snapshot from a full committed source ID.
/// Canonical sources must match HEAD and disk; unrelated changes are permitted.
pub fn prepare(root: &Path, revision: &str) -> Result<Value, String> {
    let prepared = generate(root, revision)?;
    let output = directory(root)?;
    write_atomic(&output.join("instructions.tar.gz"), &prepared.archive)?;
    write_atomic(&output.join("bundle.json"), &prepared.manifest_bytes)?;
    check(root)
}

/// Regenerate and compare both artifacts without writing files.
/// The pinned source may predate unrelated commits but must contain current inputs.
pub fn check(root: &Path) -> Result<Value, String> {
    let output = root.join(OUTPUT);
    // Refuse symlinked ancestors before reading generated files.
    let mut at = root.to_path_buf();
    for part in OUTPUT.split('/') {
        at.push(part);
        let metadata = std::fs::symlink_metadata(&at).map_err(|error| error.to_string())?;
        if !metadata.is_dir() || metadata.file_type().is_symlink() {
            return Err("nonordinary bundle directory".into());
        }
    }
    let manifest = read_json(&output.join("bundle.json"))?;
    let revision = manifest
        .get("source_commit")
        .and_then(Value::as_str)
        .ok_or("missing bundle source_commit")?;
    let prepared = generate(root, revision)?;
    for (name, expected) in [
        ("bundle.json", &prepared.manifest_bytes),
        ("instructions.tar.gz", &prepared.archive),
    ] {
        let path = output.join(name);
        let metadata = std::fs::symlink_metadata(&path).map_err(|error| error.to_string())?;
        if !metadata.is_file()
            || metadata.file_type().is_symlink()
            || std::fs::read(&path).map_err(|error| error.to_string())? != *expected
        {
            return Err(format!(
                "bundle drift: {name}; run bundle prepare with the intended committed inputs"
            ));
        }
    }
    if std::fs::read_dir(&output)
        .map_err(|error| error.to_string())?
        .count()
        != 2
    {
        return Err("unexpected files in generated bundle directory".into());
    }
    Ok(report(&prepared))
}

/// Export the exact verified package payload without overwriting an existing path.
pub fn export(root: &Path, output: &Path) -> Result<Value, String> {
    let report = check(root)?;
    let bytes = std::fs::read(root.join(OUTPUT).join("instructions.tar.gz"))
        .map_err(|error| error.to_string())?;
    let mut temporary = tempfile::NamedTempFile::new_in(
        output
            .parent()
            .filter(|path| !path.as_os_str().is_empty())
            .unwrap_or(Path::new(".")),
    )
    .map_err(|error| error.to_string())?;
    temporary
        .write_all(&bytes)
        .map_err(|error| error.to_string())?;
    temporary
        .persist_noclobber(output)
        .map_err(|error| error.to_string())?;
    Ok(report)
}

/// Inspect a genuine CLI Cargo archive and compare its package-local bundle bytes.
/// This verifies packaging; Cargo's separate package command proves the extracted build.
pub fn verify_package(root: &Path, archive: Option<&Path>) -> Result<Value, String> {
    let bundle = check(root)?;
    let manifest: toml::Value = toml::from_str(&crate::support::read_text(
        &root.join("tools/gameskills-cli/Cargo.toml"),
    )?)
    .map_err(|error| error.to_string())?;
    let version = manifest
        .get("package")
        .and_then(|package| package.get("version"))
        .and_then(toml::Value::as_str)
        .ok_or("missing CLI package version")?;
    let temporary = tempfile::tempdir().map_err(|error| error.to_string())?;
    let extracted = temporary
        .path()
        .canonicalize()
        .map_err(|error| error.to_string())?
        .join("cli");
    let default_archive = root.join(format!("target/package/gameskills-cli-{version}.crate"));
    let report = crate::distribution::inspect_archive(
        archive.unwrap_or(&default_archive),
        "gameskills-cli",
        version,
        &extracted,
    )?;
    if !report.files.iter().any(|path| path == "Cargo.lock")
        || report
            .files
            .iter()
            .any(|path| path.ends_with(".py") || path.ends_with(".pyc"))
    {
        return Err("CLI package must retain Cargo.lock and exclude Python files".into());
    }
    for name in ["bundle.json", "instructions.tar.gz"] {
        let packaged = std::fs::read(extracted.join("bundle").join(name))
            .map_err(|error| error.to_string())?;
        if packaged
            != std::fs::read(root.join(OUTPUT).join(name)).map_err(|error| error.to_string())?
        {
            return Err(format!("CLI archive has different bundle bytes: {name}"));
        }
    }
    Ok(
        json!({"bundle":bundle,"cargo_archive":report,"activation":"preparation-only","cargo_build_verified":false}),
    )
}
