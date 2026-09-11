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
    let members: toml_edit::Array = ["crates/*"].into_iter().collect();
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
        if !portable_relative(name) {
            return Err(format!("nonlocal package entry: {name}"));
        }
        let relative = PathBuf::from(name);
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
/// `non_library_packages` contains the package names owned by `games/` or `tools/`.
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
            "bevy_game_multiplayer",
            "bevy_game_discovery",
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
    for directory in ["games", "tools"] {
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
    let directory = root.join("crates");
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
                manifests.push(manifest);
            }
        }
    }
    manifests.sort();
    if manifests.is_empty() {
        return Err("no capability manifests found under crates/".into());
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
bevy_gamekit = {{ package = "bevy-gamekit", path = {facade}, default-features = false }}
serde_json = {{ version = "1", optional = true }}

[features]
default = []
pure = ["bevy_gamekit/hex", "bevy_gamekit/turns-serde", "dep:serde_json"]
ui = ["bevy_gamekit/testing-ui"]
network = ["bevy_gamekit/direct", "bevy_gamekit/mdns", "bevy_gamekit/tailscale-cli"]

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
            let destination = library.join("crates").join(crate_directory).join(&relative);
            mkdir(destination.parent().ok_or("package entry has no parent")?)?;
            copy(&crate_root.join(relative), &destination)?;
            report.staged_files += 1;
        }
        report.packages.push(package);
    }
    let facade = library.join("crates/bevy_gamekit");
    if package_name(&facade.join("Cargo.toml"))? != "bevy-gamekit" {
        return Err("expected bevy-gamekit facade at crates/bevy_gamekit".into());
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
    use super::execute_cargo;
    use std::ffi::OsString;

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
