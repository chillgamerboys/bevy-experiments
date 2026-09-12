//! Probe Cargo-selected library sources with an isolated external consumer.

use crate::support::{portable_relative, read_text};
use clap::ValueEnum;
use serde::Serialize;
use std::collections::BTreeSet;
use std::ffi::{OsStr, OsString};
use std::io::{self, Write};
use std::path::{Path, PathBuf};
use std::process::Command;
use toml_edit::{DocumentMut, Item};

const CONSUMER: &str = include_str!("../tests/fixtures/distribution/gamekit_consumer.rs");
const GENERATED: [&str; 3] = [".cargo_vcs_info.json", "Cargo.lock", "Cargo.toml.orig"];

/// External consumer feature selections supported by the distribution probe.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, ValueEnum)]
#[serde(rename_all = "lowercase")]
pub enum Case {
    /// Exercise every supported consumer graph in order.
    All,
    /// Exercise the facade with no enabled features or dependencies.
    Empty,
    /// Exercise pure algorithms and persistence without Bevy.
    Pure,
    /// Exercise headless UI without networking.
    Ui,
    /// Type-check native networking adapters and test the memory link.
    Network,
}

impl Case {
    fn name(self) -> &'static str {
        match self {
            Self::All => "all",
            Self::Empty => "empty",
            Self::Pure => "pure",
            Self::Ui => "ui",
            Self::Network => "network",
        }
    }

    fn selections(self) -> Vec<Self> {
        match self {
            Self::All => vec![Self::Empty, Self::Pure, Self::Ui, Self::Network],
            selected => vec![selected],
        }
    }
}

/// Successful verification of one activated dependency graph and consumer test.
#[derive(Debug, Serialize)]
pub struct CaseReport {
    /// The feature selection that was verified.
    pub case: Case,
    /// Number of distinct packages in Cargo's activated normal/build graph.
    pub resolved_packages: usize,
}

/// Successful distribution staging and external consumer verification.
#[derive(Debug, Serialize)]
pub struct Report {
    /// Capability package names selected for staging, in deterministic order.
    pub packages: Vec<String>,
    /// Number of ordinary source files staged from Cargo package listings.
    pub staged_files: usize,
    /// Completed consumer cases, in execution order.
    pub cases: Vec<CaseReport>,
}

/// Preserve shared workspace settings while restricting membership to libraries.
///
/// TOML tables, dotted keys, comments and inline tables are parsed structurally.
/// Only `members`, `default-members` and `exclude` are changed.
pub fn library_manifest(source: &str) -> Result<String, String> {
    let mut document = source
        .parse::<DocumentMut>()
        .map_err(|error| format!("invalid workspace manifest: {error}"))?;
    if document.contains_key("package") {
        return Err("expected a virtual workspace".into());
    }
    let workspace = document
        .get_mut("workspace")
        .and_then(Item::as_table_like_mut)
        .ok_or("expected a virtual workspace")?;
    if workspace.get("resolver").and_then(Item::as_str).is_none() {
        return Err("workspace resolver must be a string".into());
    }
    let members: toml_edit::Array = ["gamekit/*"].into_iter().collect();
    workspace.insert("members", toml_edit::value(members));
    workspace.remove("default-members");
    workspace.remove("exclude");
    Ok(document.to_string())
}

fn ordinary(path: &Path) -> Result<(), String> {
    let metadata = std::fs::symlink_metadata(path)
        .map_err(|error| format!("missing package source {}: {error}", path.display()))?;
    if metadata.file_type().is_symlink() {
        return Err(format!("symlinked package entry: {}", path.display()));
    }
    Ok(())
}

fn package_entry(name: &str, separator: char) -> Result<PathBuf, String> {
    // Cargo emits native separators on Windows, including files selected by include.
    // Normalize at that producer boundary before applying the portable path policy.
    let portable = if separator == '\\' {
        name.replace('\\', "/")
    } else {
        name.to_owned()
    };
    if !portable_relative(&portable) {
        return Err(format!("nonlocal package entry: {name}"));
    }
    Ok(PathBuf::from(portable))
}

/// Validate Cargo's package listing before copying any selected source files.
///
/// Generated Cargo entries are omitted. Every other entry must be an existing
/// ordinary file contained by the crate, without symlinks or portable path escapes.
/// A capability package must contain both `Cargo.toml` and `src/lib.rs`.
pub fn package_sources(crate_root: &Path, listing: &str) -> Result<Vec<PathBuf>, String> {
    for ancestor in crate_root.ancestors() {
        ordinary(ancestor)?;
    }
    let canonical = crate_root
        .canonicalize()
        .map_err(|error| format!("{}: {error}", crate_root.display()))?;
    let mut sources = Vec::new();
    let mut seen = BTreeSet::new();
    for name in listing.lines() {
        if GENERATED.contains(&name) {
            continue;
        }
        let relative = package_entry(name, std::path::MAIN_SEPARATOR)?;
        if !seen.insert(relative.clone()) {
            return Err(format!("duplicate package entry: {name}"));
        }
        let mut source = crate_root.to_path_buf();
        for component in relative.components() {
            source.push(component);
            ordinary(&source)?;
        }
        if !source.is_file() {
            return Err(format!("missing package source: {name}"));
        }
        let resolved = source
            .canonicalize()
            .map_err(|error| format!("{}: {error}", source.display()))?;
        if !resolved.starts_with(&canonical) {
            return Err(format!("package entry escapes crate: {name}"));
        }
        sources.push(relative);
    }
    if !seen.contains(Path::new("Cargo.toml")) || !seen.contains(Path::new("src/lib.rs")) {
        return Err("capability package must contain its manifest and library".into());
    }
    Ok(sources)
}

/// Reject game/tool dependencies and feature leakage in one activated graph.
///
/// `non_library_packages` contains the package names owned by games, GameSkills or repository tooling.
/// `All` is a request to run separate cases, and is not itself an activated graph.
pub fn validate_graph(
    case: Case,
    names: &BTreeSet<String>,
    non_library_packages: &BTreeSet<String>,
) -> Result<(), String> {
    if case == Case::All {
        return Err("validate one activated consumer case, not all".into());
    }
    let forbidden: Vec<_> = names.intersection(non_library_packages).collect();
    if !forbidden.is_empty() {
        return Err(format!("consumer resolved games or tools: {forbidden:?}"));
    }
    let base = BTreeSet::from(["gamekit_external_probe".into(), "bevy-gamekit".into()]);
    if !base.is_subset(names) {
        return Err("consumer graph is missing the probe or facade".into());
    }
    if case == Case::Empty && names != &base {
        return Err(format!("empty facade has dependencies: {names:?}"));
    }
    if case == Case::Pure && names.contains("bevy") {
        return Err("pure algorithms pulled in Bevy".into());
    }
    if case != Case::Network {
        let forbidden: Vec<_> = [
            "bevy-gamekit-multiplayer",
            "bevy-gamekit-discovery",
            "aeronet_webtransport",
        ]
        .into_iter()
        .filter(|name| names.contains(*name))
        .collect();
        if !forbidden.is_empty() {
            return Err(format!(
                "offline consumer pulled in networking: {forbidden:?}"
            ));
        }
    }
    Ok(())
}

fn package_name(manifest: &Path) -> Result<String, String> {
    let document = read_text(manifest)?
        .parse::<DocumentMut>()
        .map_err(|error| format!("{}: {error}", manifest.display()))?;
    document
        .get("package")
        .and_then(|package| package.get("name"))
        .and_then(Item::as_str)
        .filter(|name| !name.is_empty())
        .map(str::to_owned)
        .ok_or_else(|| format!("{}: package.name must be a string", manifest.display()))
}

fn non_library_packages(root: &Path) -> Result<BTreeSet<String>, String> {
    let mut names = BTreeSet::new();
    for directory in ["games", "gameskills/cli", "gameskills/linear", "devtools"] {
        let directory = root.join(directory);
        if !directory.exists() {
            continue;
        }
        ordinary(&directory)?;
        let mut pending = vec![directory];
        while let Some(directory) = pending.pop() {
            let mut entries = std::fs::read_dir(&directory)
                .map_err(|error| format!("{}: {error}", directory.display()))?
                .map(|entry| entry.map(|entry| entry.path()))
                .collect::<Result<Vec<_>, _>>()
                .map_err(|error| error.to_string())?;
            entries.sort();
            for source in entries {
                if source.file_name().is_some_and(|name| {
                    [".git", ".context", ".gameskills", "target", "__pycache__"]
                        .iter()
                        .any(|ignored| name == *ignored)
                }) {
                    continue;
                }
                // Do not silently omit a linked directory that could own a package.
                ordinary(&source)?;
                if source.is_dir() {
                    pending.push(source);
                } else if source.file_name().is_some_and(|name| name == "Cargo.toml") {
                    names.insert(package_name(&source)?);
                }
            }
        }
    }
    Ok(names)
}

fn capability_manifests(root: &Path) -> Result<Vec<PathBuf>, String> {
    let directory = root.join("gamekit");
    ordinary(&directory)?;
    let mut manifests = Vec::new();
    for entry in std::fs::read_dir(&directory)
        .map_err(|error| format!("{}: {error}", directory.display()))?
    {
        let entry = entry.map_err(|error| error.to_string())?;
        ordinary(&entry.path())?;
        if entry.path().is_dir() {
            let manifest = entry.path().join("Cargo.toml");
            if manifest.exists() {
                package_name(&manifest)?;
                manifests.push(manifest);
            }
        }
    }
    manifests.sort();
    if manifests.is_empty() {
        return Err("no capability manifests found under gamekit/".into());
    }
    Ok(manifests)
}

fn write(path: &Path, contents: impl AsRef<[u8]>) -> Result<(), String> {
    std::fs::write(path, contents).map_err(|error| format!("{}: {error}", path.display()))
}

fn copy(source: &Path, destination: &Path) -> Result<(), String> {
    std::fs::copy(source, destination)
        .map(|_| ())
        .map_err(|error| {
            format!(
                "copy {} to {}: {error}",
                source.display(),
                destination.display()
            )
        })
}

fn mkdir(path: &Path) -> Result<(), String> {
    std::fs::create_dir_all(path).map_err(|error| format!("{}: {error}", path.display()))
}

fn consumer_manifest(facade: &Path) -> Result<String, String> {
    let facade = facade
        .to_str()
        .ok_or("external consumer path must be valid UTF-8 for Cargo TOML")?;
    // Serialize a TOML string rather than applying shell or JSON escaping.
    let facade = toml_edit::Value::from(facade);
    Ok(format!(
        r#"[package]
name = "gamekit_external_probe"
version = "0.0.0"
edition = "2021"
publish = false

[dependencies]
bevy-gamekit = {{ path = {facade}, default-features = false }}
serde_json = {{ version = "1", optional = true }}

[features]
default = []
pure = ["bevy-gamekit/hex", "bevy-gamekit/turns-serde", "dep:serde_json"]
ui = ["bevy-gamekit/testing-ui"]
network = ["bevy-gamekit/direct", "bevy-gamekit/mdns", "bevy-gamekit/tailscale-cli"]

[profile.ci]
inherits = "dev"
debug = "line-tables-only"
codegen-units = 4
"#
    ))
}

fn execute_cargo(program: &OsStr, cwd: &Path, args: &[OsString]) -> Result<String, String> {
    writeln!(io::stderr().lock(), "+ {program:?} {args:?}")
        .map_err(|error| format!("write Cargo command diagnostic: {error}"))?;
    let output = Command::new(program)
        .args(args)
        .current_dir(cwd)
        .output()
        .map_err(|error| format!("cannot run {program:?} in {}: {error}", cwd.display()))?;
    io::stderr()
        .lock()
        .write_all(&output.stderr)
        .map_err(|error| format!("write Cargo diagnostics: {error}"))?;
    if !output.status.success() {
        return Err(format!(
            "{program:?} {args:?} failed in {} with {}: {}",
            cwd.display(),
            output.status,
            String::from_utf8_lossy(&output.stdout).trim()
        ));
    }
    String::from_utf8(output.stdout).map_err(|error| format!("Cargo output is not UTF-8: {error}"))
}

/// Stage capability sources and run the selected external Cargo consumer cases.
///
/// The repository lock seeds the consumer resolution; Cargo may prune unused
/// workspace entries. Temporary sources are deleted on success and failure.
/// Cargo diagnostics go to stderr and build artifacts reuse `root/target`.
pub fn check(root: &Path, case: Case) -> Result<Report, String> {
    let cargo = std::env::var_os("CARGO").unwrap_or_else(|| OsString::from("cargo"));
    check_with_runner(root, case, |cwd, args| execute_cargo(&cargo, cwd, args))
}

/// Run distribution verification with an injectable Cargo command runner.
///
/// This seam allows package staging, literal command arguments, failure handling
/// and all activated graphs to be tested without network or native dependencies.
/// The runner must return captured stdout and fail when its child command fails.
pub fn check_with_runner(
    root: &Path,
    case: Case,
    mut run: impl FnMut(&Path, &[OsString]) -> Result<String, String>,
) -> Result<Report, String> {
    let root = root
        .canonicalize()
        .map_err(|error| format!("{}: {error}", root.display()))?;
    let workspace = library_manifest(&read_text(&root.join("Cargo.toml"))?)?;
    let forbidden = non_library_packages(&root)?;
    let manifests = capability_manifests(&root)?;
    let lock = root.join("Cargo.lock");
    read_text(&lock)?;
    let temporary = tempfile::Builder::new()
        .prefix("gamekit-consumer-")
        .tempdir()
        .map_err(|error| format!("create consumer staging directory: {error}"))?;
    let staging = temporary
        .path()
        .canonicalize()
        .map_err(|error| error.to_string())?;
    let library = staging.join("library");
    mkdir(&library)?;
    write(&library.join("Cargo.toml"), workspace)?;
    let mut report = Report {
        packages: Vec::new(),
        staged_files: 0,
        cases: Vec::new(),
    };
    for manifest in manifests {
        let package = package_name(&manifest)?;
        let crate_root = manifest
            .parent()
            .ok_or("capability manifest has no parent")?;
        let crate_directory = crate_root
            .file_name()
            .ok_or("capability crate has no directory name")?;
        let args = ["package", "-p", &package, "--list", "--allow-dirty"].map(OsString::from);
        let listing = run(&root, &args)?;
        for relative in package_sources(crate_root, &listing)? {
            let destination = library
                .join("gamekit")
                .join(crate_directory)
                .join(&relative);
            mkdir(destination.parent().ok_or("package entry has no parent")?)?;
            copy(&crate_root.join(relative), &destination)?;
            report.staged_files += 1;
        }
        report.packages.push(package);
    }
    let facade = library.join("gamekit/facade");
    if package_name(&facade.join("Cargo.toml"))? != "bevy-gamekit" {
        return Err("expected bevy-gamekit facade at gamekit/facade".into());
    }
    let consumer = staging.join("consumer");
    mkdir(&consumer.join("src"))?;
    write(&consumer.join("Cargo.toml"), consumer_manifest(&facade)?)?;
    write(&consumer.join("src/lib.rs"), CONSUMER)?;
    // Seed with the owning lock: Cargo prunes game/tool entries during resolution.
    copy(&lock, &consumer.join("Cargo.lock"))?;
    for selected in case.selections() {
        let features = if selected == Case::Empty {
            Vec::new()
        } else {
            vec![
                OsString::from("--features"),
                OsString::from(selected.name()),
            ]
        };
        let mut args = [
            "tree",
            "--edges",
            "normal,build",
            "--prefix",
            "none",
            "--format",
            "{p}",
        ]
        .map(OsString::from)
        .to_vec();
        args.extend(features.iter().cloned());
        let graph = run(&consumer, &args)?;
        let names = graph
            .lines()
            .filter_map(|line| line.split_whitespace().next())
            .map(str::to_owned)
            .collect();
        validate_graph(selected, &names, &forbidden)
            .map_err(|error| format!("{} consumer: {error}", selected.name()))?;
        let mut args = ["test", "--profile", "ci", "--target-dir"]
            .map(OsString::from)
            .to_vec();
        args.push(root.join("target").into_os_string());
        args.extend(features);
        run(&consumer, &args)?;
        report.cases.push(CaseReport {
            case: selected,
            resolved_packages: names.len(),
        });
    }
    Ok(report)
}

#[cfg(test)]
mod tests {
    use super::{execute_cargo, package_entry};
    use std::ffi::OsString;

    #[test]
    fn native_cargo_separators_are_normalized_before_containment_checks() {
        assert_eq!(
            package_entry("src\\lib.rs", '\\'),
            Ok(std::path::PathBuf::from("src/lib.rs"))
        );
        assert!(package_entry("src\\lib.rs", '/').is_err());
        for name in [
            "..\\outside",
            "src\\..\\..\\outside",
            "\\absolute",
            "\\\\server\\share",
            "C:\\outside",
            "C:/outside",
            "src\\.\\lib.rs",
            "src\\\\lib.rs",
            "src\\bad\0",
        ] {
            assert!(package_entry(name, '\\').is_err(), "{name:?}");
        }
    }

    #[test]
    fn child_arguments_are_literal_and_failures_propagate() -> Result<(), Box<dyn std::error::Error>>
    {
        let current = std::env::current_exe()?;
        let profile = current
            .parent()
            .and_then(std::path::Path::parent)
            .ok_or("test executable has no profile directory")?;
        let probe = profile.join("examples").join(format!(
            "distribution_process_probe{}",
            std::env::consts::EXE_SUFFIX
        ));
        assert!(
            probe.is_file(),
            "Cargo must build the distribution_process_probe example for this test: {}",
            probe.display()
        );
        let directory = tempfile::Builder::new()
            .prefix("distribution child with spaces ")
            .tempdir()?;
        let executable = directory
            .path()
            .join(format!("fake cargo{}", std::env::consts::EXE_SUFFIX));
        std::fs::copy(probe, &executable)?;
        let literal = "spaces ; $(touch injected) `touch injected` \"quoted\"";
        let output = execute_cargo(
            executable.as_os_str(),
            directory.path(),
            &[OsString::from("echo"), OsString::from(literal)],
        )?;
        assert_eq!(output, literal);
        assert!(!directory.path().join("injected").exists());
        let error = execute_cargo(
            executable.as_os_str(),
            directory.path(),
            &[OsString::from("fail")],
        )
        .err()
        .ok_or("child unexpectedly succeeded")?;
        assert!(error.contains("23"));
        assert!(error.contains("failed probe"));
        assert!(execute_cargo(
            directory.path().join("missing cargo").as_os_str(),
            directory.path(),
            &[]
        )
        .is_err());
        Ok(())
    }
}

/// One Cargo-produced library archive inspected before external consumption.
#[derive(Debug, Serialize)]
pub struct ArchiveReport {
    /// Cargo package name.
    pub package: String,
    /// Resolved package version.
    pub version: String,
    /// SHA-256 of the original compressed Cargo artifact.
    pub sha256: String,
    /// Ordinary archive files, relative to the Cargo package directory.
    pub files: Vec<String>,
}

/// Verification of actual Cargo artifacts, with explicit pre-publication limits.
#[derive(Debug, Serialize)]
pub struct ArchivesReport {
    /// Genuine Cargo archives inspected and consumed from extracted sources.
    pub archives: Vec<ArchiveReport>,
    /// Internal path dependencies given matching versions only in temporary staging.
    pub staged_version_requirements: Vec<String>,
    /// Registry package names patched to extracted archives in the external consumer.
    pub local_patches: Vec<String>,
    /// Library artifacts deliberately omit lockfiles while siblings are unpublished.
    pub lockfile_omitted: bool,
    /// False: local patches do not verify resolution against a published registry.
    pub registry_resolution_verified: bool,
    /// Completed external feature cases.
    pub cases: Vec<CaseReport>,
}

fn normalized_manifest(source: &str, package: &str, version: &str) -> Result<(), String> {
    let document = source
        .parse::<DocumentMut>()
        .map_err(|error| format!("invalid normalized manifest: {error}"))?;
    for forbidden in ["workspace", "patch", "replace"] {
        if document.contains_key(forbidden) {
            return Err(format!("normalized manifest retains {forbidden}"));
        }
    }
    let metadata = document
        .get("package")
        .ok_or("missing normalized package")?;
    if metadata.get("name").and_then(Item::as_str) != Some(package)
        || metadata.get("version").and_then(Item::as_str) != Some(version)
    {
        return Err("normalized manifest package identity differs from archive".into());
    }
    fn inspect(table: &dyn toml_edit::TableLike, dependency: bool) -> Result<(), String> {
        for (key, value) in table.iter() {
            if key == "workspace" || (dependency && key == "path") {
                return Err(format!("normalized manifest retains {key}"));
            }
            if let Some(child) = value.as_table_like() {
                inspect(
                    child,
                    dependency
                        || ["dependencies", "dev-dependencies", "build-dependencies"]
                            .contains(&key),
                )?;
            }
        }
        Ok(())
    }
    // Cargo may preserve arbitrary package metadata; it is not a build boundary.
    for (key, value) in document.iter() {
        if key != "package" {
            if let Some(table) = value.as_table_like() {
                inspect(
                    table,
                    ["dependencies", "dev-dependencies", "build-dependencies"].contains(&key),
                )?;
            }
        }
    }
    for (key, value) in metadata
        .as_table_like()
        .ok_or("package is not a table")?
        .iter()
    {
        if key == "workspace" || (key != "metadata" && value.is_table_like()) {
            return Err(format!("normalized package retains inherited {key}"));
        }
    }
    Ok(())
}

/// Inspect and safely extract one actual Cargo archive into a new directory.
///
/// Only ordinary files under the exact name/version prefix are accepted. Duplicate
/// paths, links, platform-specific escapes, inherited manifests and oversized data
/// fail. Cargo's normalized manifest and library must both be present. Callers own
/// cleanup of the destination, including partial extraction after an error.
pub fn inspect_archive(
    archive: &Path,
    package: &str,
    version: &str,
    destination: &Path,
) -> Result<ArchiveReport, String> {
    inspect_archive_bounded(archive, package, version, destination, 64 * 1024 * 1024)
}

fn inspect_archive_bounded(
    archive: &Path,
    package: &str,
    version: &str,
    destination: &Path,
    expanded_limit: u64,
) -> Result<ArchiveReport, String> {
    use sha2::{Digest, Sha256};
    use std::io::Read;
    let prefix = format!("{package}-{version}");
    if !portable_relative(&prefix) || prefix.contains('/') {
        return Err("invalid archive package identity".into());
    }
    let metadata = std::fs::symlink_metadata(archive).map_err(|error| error.to_string())?;
    if !metadata.file_type().is_file() {
        return Err("archive input must be an ordinary file".into());
    }
    match std::fs::symlink_metadata(destination) {
        Ok(_) => return Err("archive extraction destination already exists".into()),
        Err(error) if error.kind() == io::ErrorKind::NotFound => {}
        Err(error) => return Err(error.to_string()),
    }
    for ancestor in destination.ancestors().skip(1) {
        match std::fs::symlink_metadata(ancestor) {
            Ok(metadata) if !metadata.file_type().is_dir() => {
                return Err("archive extraction ancestor must be an ordinary directory".into());
            }
            Ok(_) => {}
            Err(error) if error.kind() == io::ErrorKind::NotFound => {}
            Err(error) => return Err(error.to_string()),
        }
    }
    if metadata.len() > 64 * 1024 * 1024 {
        return Err("compressed library archive exceeds inspection limits".into());
    }
    let compressed = std::fs::read(archive).map_err(|error| error.to_string())?;
    let digest = format!("{:x}", Sha256::digest(&compressed));
    let decoder = flate2::bufread::GzDecoder::new(compressed.as_slice());
    // Bound decompression before tar eagerly materializes GNU/PAX extensions.
    // Per-file checks below only see entries after those extensions are parsed.
    let mut tar = tar::Archive::new(decoder.take(expanded_limit));
    let mut files = BTreeSet::new();
    let mut size = 0_u64;
    for entry in tar.entries().map_err(|error| error.to_string())? {
        let mut entry = entry.map_err(|error| error.to_string())?;
        if !entry.header().entry_type().is_file() {
            return Err("archive contains a non-ordinary entry".into());
        }
        let bytes = entry.path_bytes();
        let path = std::str::from_utf8(&bytes).map_err(|_| "archive entry path is not UTF-8")?;
        let relative = path
            .strip_prefix(&format!("{prefix}/"))
            .ok_or("archive entry has unexpected package prefix")?;
        if !portable_relative(relative) || !files.insert(relative.to_owned()) {
            return Err(format!("invalid or duplicate archive entry: {path}"));
        }
        let relative = relative.to_owned();
        let length = entry.size();
        size = size.checked_add(length).ok_or("archive size overflow")?;
        if length > 64 * 1024 * 1024 || size > 512 * 1024 * 1024 || files.len() > 100_000 {
            return Err("library archive exceeds inspection limits".into());
        }
        let path = destination.join(&relative);
        mkdir(path.parent().ok_or("archive file has no parent")?)?;
        let mut output = std::fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&path)
            .map_err(|error| format!("{}: {error}", path.display()))?;
        let copied = io::copy(&mut entry, &mut output).map_err(|error| error.to_string())?;
        if copied != length {
            return Err(format!("truncated archive file: {relative}"));
        }
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let mode = entry.header().mode().map_err(|error| error.to_string())?;
            std::fs::set_permissions(&path, std::fs::Permissions::from_mode(mode & 0o777))
                .map_err(|error| error.to_string())?;
        }
    }
    // Finish the gzip stream so a corrupt checksum cannot hide after the tar EOF.
    let mut decoder = tar.into_inner();
    let mut trailing = Vec::new();
    decoder
        .by_ref()
        .take(1025)
        .read_to_end(&mut trailing)
        .map_err(|error| error.to_string())?;
    if trailing.len() > 1024
        || trailing.iter().any(|byte| *byte != 0)
        || decoder.limit() == 0
        || !decoder.into_inner().into_inner().is_empty()
    {
        return Err("archive has excess data after tar entries".into());
    }
    if !files.contains("Cargo.toml") || !files.contains("src/lib.rs") {
        return Err("archive lacks normalized manifest or library".into());
    }
    normalized_manifest(
        &read_text(&destination.join("Cargo.toml"))?,
        package,
        version,
    )?;
    normalized_targets(&read_text(&destination.join("Cargo.toml"))?, &files)?;
    Ok(ArchiveReport {
        package: package.to_owned(),
        version: version.to_owned(),
        sha256: digest,
        files: files.into_iter().collect(),
    })
}

fn normalized_targets(source: &str, files: &BTreeSet<String>) -> Result<(), String> {
    let document: toml::Value = toml::from_str(source).map_err(|error| error.to_string())?;
    let check = |path: &str| {
        if !portable_relative(path) || !files.contains(path) {
            Err(format!(
                "normalized manifest references a missing or nonlocal file: {path}"
            ))
        } else {
            Ok(())
        }
    };
    if let Some(package) = document.get("package") {
        for key in ["build", "readme", "license-file"] {
            if let Some(path) = package.get(key).and_then(toml::Value::as_str) {
                check(path)?;
            }
        }
    }
    for kind in ["lib", "bin", "test", "bench", "example"] {
        if let Some(target) = document.get(kind) {
            let targets = target
                .as_array()
                .map(Vec::as_slice)
                .unwrap_or(std::slice::from_ref(target));
            for target in targets {
                if let Some(path) = target.get("path").and_then(toml::Value::as_str) {
                    check(path)?;
                }
            }
        }
    }
    Ok(())
}

fn normalized_siblings(
    source: &str,
    versions: &std::collections::BTreeMap<String, String>,
) -> Result<(), String> {
    fn inspect(
        table: &toml::Table,
        versions: &std::collections::BTreeMap<String, String>,
    ) -> Result<(), String> {
        for (key, value) in table {
            let Some(child) = value.as_table() else {
                continue;
            };
            if ["dependencies", "dev-dependencies", "build-dependencies"].contains(&key.as_str()) {
                for (alias, dependency) in child {
                    let name = dependency
                        .get("package")
                        .and_then(toml::Value::as_str)
                        .unwrap_or(alias);
                    if let Some(version) = versions.get(name) {
                        if dependency.get("version").and_then(toml::Value::as_str)
                            != Some(format!("={version}").as_str())
                        {
                            return Err(format!(
                                "normalized sibling {name} has no matching staged version"
                            ));
                        }
                    }
                }
            } else if key != "package" {
                inspect(child, versions)?;
            }
        }
        Ok(())
    }
    let document: toml::Table = toml::from_str(source).map_err(|error| error.to_string())?;
    inspect(&document, versions)
}

fn package_version(manifest: &Path, workspace: &DocumentMut) -> Result<String, String> {
    let document = read_text(manifest)?
        .parse::<DocumentMut>()
        .map_err(|error| error.to_string())?;
    let version = document
        .get("package")
        .and_then(|value| value.get("version"));
    let resolved = if version
        .and_then(|value| value.get("workspace"))
        .and_then(Item::as_bool)
        == Some(true)
    {
        workspace
            .get("workspace")
            .and_then(|value| value.get("package"))
            .and_then(|value| value.get("version"))
            .and_then(Item::as_str)
    } else {
        version.and_then(Item::as_str)
    };
    resolved
        .filter(|value| !value.is_empty())
        .map(str::to_owned)
        .ok_or_else(|| format!("{}: unresolved package version", manifest.display()))
}

fn version_paths(
    table: &mut dyn toml_edit::TableLike,
    directory: &Path,
    packages: &std::collections::BTreeMap<PathBuf, (String, String)>,
    context: &str,
    rewrites: &mut Vec<String>,
) -> Result<(), String> {
    for (key, value) in table.iter_mut() {
        let Some(child) = value.as_table_like_mut() else {
            continue;
        };
        if ["dependencies", "build-dependencies", "dev-dependencies"].contains(&key.get()) {
            for (alias, dependency) in child.iter_mut() {
                let Some(specification) = dependency.as_table_like_mut() else {
                    continue;
                };
                let Some(path) = specification.get("path").and_then(Item::as_str) else {
                    continue;
                };
                let target = directory
                    .join(path)
                    .canonicalize()
                    .map_err(|error| format!("{context}: dependency path {path}: {error}"))?;
                let (name, version) = packages.get(&target).ok_or_else(|| {
                    format!("{context}: dependency path is not a packaged library: {path}")
                })?;
                let declared = specification
                    .get("package")
                    .and_then(Item::as_str)
                    .unwrap_or(alias.get());
                if declared != name {
                    return Err(format!("{context}: dependency {declared} points to {name}"));
                }
                let required = format!("={version}");
                if specification.get("version").and_then(Item::as_str) != Some(&required) {
                    specification.insert("version", toml_edit::value(&required));
                    rewrites.push(format!("{context}: {} -> {name} {required}", alias.get()));
                }
            }
        } else if key.get() != "package" {
            version_paths(child, directory, packages, context, rewrites)?;
        }
    }
    Ok(())
}

/// Produce real library Cargo archives and consume their extracted package sources.
///
/// Temporary manifests receive exact sibling versions. `publish=false` is kept;
/// Cargo's library-only `--exclude-lockfile` avoids pretending unpublished siblings
/// are available in a registry. The external consumer uses explicit local patches
/// and is seeded from the owning lockfile. No registry upload is performed.
pub fn archives(root: &Path, case: Case) -> Result<ArchivesReport, String> {
    let cargo = std::env::var_os("CARGO").unwrap_or_else(|| OsString::from("cargo"));
    archives_with_runner(root, case, |cwd, args| execute_cargo(&cargo, cwd, args))
}

/// Run archive verification with an injectable Cargo runner for boundary tests.
///
/// Unlike list-only staging tests, a successful runner must create genuine Cargo
/// archives under the requested target directory. Consumers never use the staging
/// workspace after packaging; all local patches point to inspected extracted files.
pub fn archives_with_runner(
    root: &Path,
    case: Case,
    mut run: impl FnMut(&Path, &[OsString]) -> Result<String, String>,
) -> Result<ArchivesReport, String> {
    let root = root.canonicalize().map_err(|error| error.to_string())?;
    let workspace_source = read_text(&root.join("Cargo.toml"))?;
    let mut workspace = library_manifest(&workspace_source)?
        .parse::<DocumentMut>()
        .map_err(|error| error.to_string())?;
    let forbidden = non_library_packages(&root)?;
    let manifests = capability_manifests(&root)?;
    let mut packages = std::collections::BTreeMap::new();
    let mut names = BTreeSet::new();
    for manifest in &manifests {
        let name = package_name(manifest)?;
        if !names.insert(name.clone()) {
            return Err(format!("duplicate library package: {name}"));
        }
        let version = package_version(manifest, &workspace)?;
        packages.insert(
            manifest
                .parent()
                .ok_or("manifest parent missing")?
                .canonicalize()
                .map_err(|error| error.to_string())?,
            (name, version),
        );
    }
    let lock = root.join("Cargo.lock");
    read_text(&lock)?;
    let temporary = tempfile::Builder::new()
        .prefix("gamekit-archives-")
        .tempdir()
        .map_err(|error| error.to_string())?;
    let staging = temporary
        .path()
        .canonicalize()
        .map_err(|error| error.to_string())?;
    let library = staging.join("library");
    mkdir(&library)?;
    let mut report = ArchivesReport {
        archives: Vec::new(),
        staged_version_requirements: Vec::new(),
        local_patches: Vec::new(),
        lockfile_omitted: true,
        registry_resolution_verified: false,
        cases: Vec::new(),
    };
    version_paths(
        workspace.as_table_mut(),
        &root,
        &packages,
        "Cargo.toml",
        &mut report.staged_version_requirements,
    )?;
    write(&library.join("Cargo.toml"), workspace.to_string())?;
    copy(&lock, &library.join("Cargo.lock"))?;
    let mut selected = std::collections::BTreeMap::new();
    for manifest in manifests {
        let crate_root = manifest.parent().ok_or("manifest parent missing")?;
        let (package, _) = packages.get(crate_root).ok_or("missing package identity")?;
        let folder = crate_root.file_name().ok_or("missing crate folder")?;
        let listing = run(
            &root,
            &["package", "-p", package, "--list", "--allow-dirty"].map(OsString::from),
        )?;
        let sources = package_sources(crate_root, &listing)?;
        let destination = library.join("gamekit").join(folder);
        for relative in &sources {
            let target = destination.join(relative);
            mkdir(target.parent().ok_or("package source parent missing")?)?;
            copy(&crate_root.join(relative), &target)?;
        }
        let mut document = read_text(&destination.join("Cargo.toml"))?
            .parse::<DocumentMut>()
            .map_err(|error| error.to_string())?;
        version_paths(
            document.as_table_mut(),
            crate_root,
            &packages,
            &format!("gamekit/{}/Cargo.toml", folder.to_string_lossy()),
            &mut report.staged_version_requirements,
        )?;
        write(&destination.join("Cargo.toml"), document.to_string())?;
        selected.insert(package.clone(), (destination, sources));
    }
    let target = staging.join("cargo-output");
    let mut args = [
        "package",
        "--workspace",
        "--no-verify",
        "--exclude-lockfile",
        "--allow-dirty",
        "--target-dir",
    ]
    .map(OsString::from)
    .to_vec();
    args.push(target.clone().into_os_string());
    run(&library, &args)?;
    let extracted = staging.join("extracted");
    let mut extracted_packages = std::collections::BTreeMap::new();
    let versions = packages.values().cloned().collect();
    for (package, version) in packages.values() {
        let destination = extracted.join(format!("{package}-{version}"));
        let record = inspect_archive(
            &target
                .join("package")
                .join(format!("{package}-{version}.crate")),
            package,
            version,
            &destination,
        )?;
        normalized_siblings(&read_text(&destination.join("Cargo.toml"))?, &versions)?;
        let (source, expected) = selected
            .get(package)
            .ok_or("missing staged source listing")?;
        let expected_names: BTreeSet<_> = expected
            .iter()
            .map(|path| path.to_string_lossy().replace('\\', "/"))
            .chain(["Cargo.toml.orig".to_owned()])
            .collect();
        let actual_names: BTreeSet<_> = record.files.iter().cloned().collect();
        if actual_names != expected_names {
            return Err(format!("{package}: archive files differ from selected sources: expected {expected_names:?}, got {actual_names:?}"));
        }
        for relative in expected {
            let archived = if relative == Path::new("Cargo.toml") {
                Path::new("Cargo.toml.orig")
            } else {
                relative
            };
            let original =
                std::fs::read(source.join(relative)).map_err(|error| error.to_string())?;
            let packed =
                std::fs::read(destination.join(archived)).map_err(|error| error.to_string())?;
            if original != packed {
                return Err(format!(
                    "{package}: Cargo changed selected source {}",
                    relative.display()
                ));
            }
        }
        report.archives.push(record);
        extracted_packages.insert(package.clone(), destination);
    }
    // Delete staging sources before any consumer command to prove containment.
    std::fs::remove_dir_all(&library).map_err(|error| error.to_string())?;
    let facade = extracted_packages
        .get("bevy-gamekit")
        .ok_or("missing bevy-gamekit facade")?;
    let consumer = staging.join("consumer");
    mkdir(&consumer.join("src"))?;
    let mut document = consumer_manifest(facade)?
        .parse::<DocumentMut>()
        .map_err(|error| error.to_string())?;
    let mut patches = toml_edit::Table::new();
    for (package, path) in &extracted_packages {
        let path = path.to_str().ok_or("extracted archive path is not UTF-8")?;
        let mut specification = toml_edit::InlineTable::new();
        specification.insert("path", path.into());
        patches.insert(package, toml_edit::value(specification));
        report.local_patches.push(package.clone());
    }
    let mut patch = toml_edit::Table::new();
    patch.insert("crates-io", Item::Table(patches));
    document.insert("patch", Item::Table(patch));
    write(&consumer.join("Cargo.toml"), document.to_string())?;
    write(&consumer.join("src/lib.rs"), CONSUMER)?;
    copy(&lock, &consumer.join("Cargo.lock"))?;
    for selected in case.selections() {
        let features = if selected == Case::Empty {
            Vec::new()
        } else {
            vec![
                OsString::from("--features"),
                OsString::from(selected.name()),
            ]
        };
        let mut args = [
            "tree",
            "--edges",
            "normal,build",
            "--prefix",
            "none",
            "--format",
            "{p}",
        ]
        .map(OsString::from)
        .to_vec();
        args.extend(features.iter().cloned());
        let graph = run(&consumer, &args)?;
        let names = graph
            .lines()
            .filter_map(|line| line.split_whitespace().next())
            .map(str::to_owned)
            .collect();
        validate_graph(selected, &names, &forbidden)
            .map_err(|error| format!("{} archive consumer: {error}", selected.name()))?;
        let mut args = ["test", "--locked", "--profile", "ci", "--target-dir"]
            .map(OsString::from)
            .to_vec();
        args.push(root.join("target").into_os_string());
        args.extend(features);
        run(&consumer, &args)?;
        report.cases.push(CaseReport {
            case: selected,
            resolved_packages: names.len(),
        });
    }
    report.staged_version_requirements.sort();
    Ok(report)
}

#[cfg(test)]
mod expansion_tests {
    use super::inspect_archive_bounded;
    use std::error::Error;

    #[test]
    fn gnu_extension_payload_is_bounded_before_tar_materializes_it() -> Result<(), Box<dyn Error>> {
        let temporary = tempfile::tempdir()?;
        let root = temporary.path().canonicalize()?;
        let compressed = flate2::write::GzEncoder::new(Vec::new(), flate2::Compression::default());
        let mut builder = tar::Builder::new(compressed);
        let mut header = tar::Header::new_gnu();
        header.set_path("././@LongLink")?;
        header.set_entry_type(tar::EntryType::GNULongName);
        header.set_size(4096);
        header.set_mode(0o644);
        header.set_cksum();
        builder.append(&header, vec![b'x'; 4096].as_slice())?;
        let archive = root.join("extension.crate");
        std::fs::write(&archive, builder.into_inner()?.finish()?)?;
        // A 1 KiB stream cap includes the extension header and payload, although
        // tar never exposes that extension as an ordinary entry to our loop.
        let error =
            inspect_archive_bounded(&archive, "fixture", "0.1.0", &root.join("extracted"), 1024)
                .expect_err("extension must exhaust the bounded stream");
        assert!(error.contains("EOF"), "{error}");
        assert!(!root.join("extracted").exists());
        Ok(())
    }
}
