//! Verify the migration ledger against a frozen, independently captured inventory.

use serde::Deserialize;
use sha2::{Digest, Sha256};
use std::collections::BTreeSet;
use std::path::Path;
use std::process::Command;

/// Accepted reference containing the behavior being migrated.
pub const REFERENCE: &str = "08a38a594750a44411756bcf8f21e99da5dddf8b";
const BASELINE: &str = include_str!("../tests/fixtures/legacy-inventory.json");

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Baseline {
    reference_commit: String,
    files: Vec<SourceFile>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct SourceFile {
    path: String,
    sha256: String,
    tests: Vec<String>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Inventory {
    schema_version: u32,
    reference_commit: String,
    files: Vec<FileContract>,
    tests: Vec<TestContract>,
    changes: Vec<ChangeContract>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct FileContract {
    source: String,
    destination: String,
    stage: String,
    disposition: String,
    rationale: String,
    status: String,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct TestContract {
    id: String,
    source: String,
    symbol: String,
    destination: String,
    stage: String,
    disposition: String,
    rationale: String,
    evidence: String,
    status: String,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct ChangeContract {
    id: String,
    before: String,
    after: String,
    rationale: String,
    evidence: String,
}

fn nonempty(value: &str, field: &str) -> Result<(), String> {
    if value.trim().is_empty() {
        Err(format!("missing {field}"))
    } else {
        Ok(())
    }
}

fn path(value: &str) -> bool {
    !value.is_empty()
        && !value.starts_with('/')
        && !value.contains(['\\', ':', '\0'])
        && value
            .split('/')
            .all(|part| !part.is_empty() && part != "." && part != "..")
}

fn disposition(stage: &str, decision: &str, status: &str, destination: &str) -> Result<(), String> {
    if !["R0", "R1", "R2", "R3", "R4", "R5", "R6", "R7"].contains(&stage)
        || !["retained", "corrected", "retired"].contains(&decision)
        || !["planned", "partial", "implemented", "retired"].contains(&status)
        || !destination.starts_with("tools/")
        || !path(destination)
    {
        return Err("invalid migration stage, disposition, status or destination".into());
    }
    Ok(())
}

fn baseline() -> Result<Baseline, String> {
    let baseline: Baseline = serde_json::from_str(BASELINE).map_err(|error| error.to_string())?;
    let paths: BTreeSet<_> = baseline.files.iter().map(|file| &file.path).collect();
    let symbols: BTreeSet<_> = baseline
        .files
        .iter()
        .flat_map(|file| {
            file.tests
                .iter()
                .map(|name| format!("{}::{name}", file.path))
        })
        .collect();
    if baseline.reference_commit != REFERENCE
        || baseline.files.len() != 22
        || paths.len() != 22
        || symbols.len() != 142
        || baseline
            .files
            .iter()
            .map(|file| file.tests.len())
            .sum::<usize>()
            != 142
        || baseline.files.iter().any(|file| {
            !path(&file.path)
                || file.sha256.len() != 64
                || !file.sha256.bytes().all(|byte| byte.is_ascii_hexdigit())
        })
    {
        return Err("frozen reference inventory is invalid".into());
    }
    Ok(baseline)
}

/// Validate complete file/test accounting. This validates a plan, not execution evidence.
///
/// Typed deserialization rejects duplicate JSON fields, unknown fields and wrong types.
pub fn validate(source: &str) -> Result<(), String> {
    let inventory: Inventory = serde_json::from_str(source).map_err(|error| error.to_string())?;
    let baseline = baseline()?;
    if inventory.schema_version != 1 || inventory.reference_commit != REFERENCE {
        return Err("unsupported inventory schema or changed reference commit".into());
    }
    let expected_files: BTreeSet<_> = baseline
        .files
        .iter()
        .map(|file| file.path.as_str())
        .collect();
    let mut files = BTreeSet::new();
    for file in &inventory.files {
        if !files.insert(file.source.as_str()) {
            return Err(format!("duplicate source file: {}", file.source));
        }
        disposition(
            &file.stage,
            &file.disposition,
            &file.status,
            &file.destination,
        )?;
        nonempty(&file.rationale, "file rationale")?;
    }
    if files != expected_files {
        return Err("source files do not match the 22-file reference inventory".into());
    }
    let expected_tests: BTreeSet<_> = baseline
        .files
        .iter()
        .flat_map(|file| {
            file.tests
                .iter()
                .map(|name| format!("{}::{name}", file.path))
        })
        .collect();
    let mut tests = BTreeSet::new();
    let mut ids = BTreeSet::new();
    for test in &inventory.tests {
        if !ids.insert(test.id.as_str())
            || !tests.insert(format!("{}::{}", test.source, test.symbol))
        {
            return Err(format!("duplicate test contract: {}", test.id));
        }
        for (field, name) in [
            (&test.id, "test id"),
            (&test.rationale, "test rationale"),
            (&test.evidence, "expected evidence"),
        ] {
            nonempty(field, name)?;
        }
        disposition(
            &test.stage,
            &test.disposition,
            &test.status,
            &test.destination,
        )?;
    }
    if tests != expected_tests {
        return Err("test contracts do not match the 142-method reference inventory".into());
    }
    if inventory.changes.is_empty() {
        return Err("intentional foundation differences must be recorded".into());
    }
    for change in &inventory.changes {
        if !ids.insert(change.id.as_str()) {
            return Err("duplicate change identifier".into());
        }
        for (field, name) in [
            (&change.id, "change id"),
            (&change.before, "prior behavior"),
            (&change.after, "new behavior"),
            (&change.rationale, "change rationale"),
            (&change.evidence, "change evidence"),
        ] {
            nonempty(field, name)?;
        }
    }
    Ok(())
}

/// Enforce final cutover structure without converting ledger declarations into test evidence.
///
/// Every original behavior needs a completed or retired disposition and an existing
/// Rust destination. Tracked interpreter sources and Python CI setup are rejected.
pub fn cutover(root: &Path, source: &str) -> Result<(), String> {
    validate(source)?;
    let inventory: Inventory = serde_json::from_str(source).map_err(|error| error.to_string())?;
    let contracts = inventory
        .files
        .iter()
        .map(|row| (&row.status, &row.destination))
        .chain(
            inventory
                .tests
                .iter()
                .map(|row| (&row.status, &row.destination)),
        );
    for (status, destination) in contracts {
        if !matches!(status.as_str(), "implemented" | "retired") {
            return Err(format!(
                "unfinished cutover contract: {destination} ({status})"
            ));
        }
        let path = root.join(destination);
        let metadata = std::fs::symlink_metadata(&path)
            .map_err(|error| format!("cutover destination {destination}: {error}"))?;
        if !metadata.file_type().is_file() || path.extension().is_none_or(|value| value != "rs") {
            return Err(format!(
                "cutover destination must be an ordinary Rust file: {destination}"
            ));
        }
    }
    let paths = git(root, &["ls-files", "-z"])?;
    for path in paths
        .split(|byte| *byte == 0)
        .filter(|path| !path.is_empty())
    {
        let path = std::str::from_utf8(path).map_err(|error| error.to_string())?;
        if matches!(
            Path::new(path)
                .extension()
                .and_then(|extension| extension.to_str())
                .map(str::to_ascii_lowercase)
                .as_deref(),
            Some("py" | "pyc" | "pyo")
        ) {
            return Err(format!("tracked interpreter source remains: {path}"));
        }
        if path.starts_with(".github/workflows/") {
            let source = crate::support::read_text(&root.join(path))?;
            if source.contains("setup-python") {
                return Err(format!("Python CI setup remains: {path}"));
            }
        }
    }
    Ok(())
}

fn git(root: &Path, args: &[&str]) -> Result<Vec<u8>, String> {
    let result = Command::new("git")
        .args(args)
        .current_dir(root)
        .output()
        .map_err(|error| error.to_string())?;
    if !result.status.success() {
        return Err(format!(
            "Git reference verification failed: {}",
            String::from_utf8_lossy(&result.stderr).trim()
        ));
    }
    Ok(result.stdout)
}

/// Verify the compiled baseline against immutable Git objects, without running Python.
///
/// Requires the accepted reference in local history; shallow/missing history is an error.
pub fn verify_reference(root: &Path) -> Result<(), String> {
    let baseline = baseline()?;
    let listing = git(root, &["ls-tree", "-r", "--name-only", REFERENCE])?;
    let listing = String::from_utf8(listing).map_err(|error| error.to_string())?;
    let observed: BTreeSet<_> = listing
        .lines()
        .filter(|path| path.ends_with(".py"))
        .collect();
    let expected: BTreeSet<_> = baseline
        .files
        .iter()
        .map(|file| file.path.as_str())
        .collect();
    if observed != expected {
        return Err("baseline files differ from the pinned Git tree".into());
    }
    for file in baseline.files {
        let bytes = git(root, &["show", &format!("{REFERENCE}:{}", file.path)])?;
        let digest = format!("{:x}", Sha256::digest(&bytes));
        if digest != file.sha256 {
            return Err(format!("baseline source digest differs: {}", file.path));
        }
        let source = String::from_utf8(bytes).map_err(|error| error.to_string())?;
        if test_symbols(&source) != file.tests.into_iter().collect() {
            return Err(format!("baseline test symbols differ: {}", file.path));
        }
    }
    Ok(())
}

// The pinned sources use ordinary class-level test definitions. This is an inventory
// check for those frozen files, not a general Python parser or executable interpreter.
fn test_symbols(source: &str) -> BTreeSet<String> {
    let mut class = "";
    let mut result = BTreeSet::new();
    for line in source.lines() {
        if let Some(rest) = line.strip_prefix("class ") {
            class = rest.split(['(', ':']).next().unwrap_or("");
        }
        if let Some(rest) = line.strip_prefix("    def test_") {
            if let Some(name) = rest.split('(').next() {
                result.insert(format!("{class}.test_{name}"));
            }
        }
    }
    result
}
