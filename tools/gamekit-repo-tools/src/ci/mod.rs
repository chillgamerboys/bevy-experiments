//! CI selection from committed Git inputs and validation of selected job results.

use clap::ValueEnum;
use serde::{Deserialize, Serialize};
use std::path::Path;
use std::process::Command;

pub mod checks;

/// Conditional jobs exposed by the GitHub Actions workflow.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize, ValueEnum)]
#[serde(rename_all = "lowercase")]
pub enum Job {
    /// Native skill validation and the remaining runtime tests.
    Skills,
    /// Selected Rust package tests and external consumers.
    Rust,
    /// Formatting, Clippy and applicable feature/dependency policies.
    Policy,
}

impl Job {
    /// Stable GitHub job and command name.
    pub fn name(self) -> &'static str {
        match self {
            Self::Skills => "skills",
            Self::Rust => "rust",
            Self::Policy => "policy",
        }
    }
}

/// The version-one selection record consumed by all jobs and the final gate.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Selection {
    /// Integer schema version, currently one.
    pub schema_version: u32,
    /// Resolved tested Git commit object ID.
    pub head: String,
    /// Requested base, or the resolved base when comparison succeeded.
    pub base: Option<String>,
    /// Whether conservative or manual selection requires every check.
    pub full: bool,
    /// Selected workspace package names; empty for a full selection.
    pub packages: Vec<String>,
    /// Changed paths including both sides of renames.
    pub paths: Vec<String>,
    /// Human-readable reasons for selection.
    pub reasons: Vec<String>,
    /// Select the skill job.
    pub skills: bool,
    /// Select the Rust job.
    pub rust: bool,
    /// Select the policy job.
    pub policy: bool,
    /// Verify external library consumers.
    pub distribution: bool,
    /// Check applicable minimal library features.
    pub minimal: bool,
    /// Check browser-compatible core packages.
    pub wasm: bool,
    /// Check dependency policy.
    pub deny: bool,
}

impl Selection {
    /// Select every check for a manual request or uncertain committed inputs.
    pub fn full(head: String, base: Option<String>, reason: String) -> Self {
        Self {
            schema_version: 1,
            head,
            base,
            full: true,
            packages: Vec::new(),
            paths: Vec::new(),
            reasons: vec![reason],
            skills: true,
            rust: true,
            policy: true,
            distribution: true,
            minimal: true,
            wasm: true,
            deny: true,
        }
    }

    /// Whether a conditional job is required by this selection.
    pub fn selected(&self, job: Job) -> bool {
        match job {
            Job::Skills => self.skills,
            Job::Rust => self.rust,
            Job::Policy => self.policy,
        }
    }
}

pub(crate) fn git(root: &Path, arguments: &[&str]) -> Result<String, String> {
    let output = Command::new("git")
        .args(arguments)
        .current_dir(root)
        .output()
        .map_err(|error| format!("cannot run Git in {}: {error}", root.display()))?;
    if !output.status.success() {
        return Err(format!(
            "Git {arguments:?} failed: {}",
            String::from_utf8_lossy(&output.stderr).trim()
        ));
    }
    String::from_utf8(output.stdout).map_err(|error| format!("Git output is not UTF-8: {error}"))
}

/// Resolve the checkout identity without starting Cargo or contacting a remote.
pub fn head(root: &Path) -> Result<String, String> {
    git(root, &["rev-parse", "--verify", "HEAD^{commit}"]).map(|head| head.trim().to_owned())
}

/// Compute selection using committed base/head trees; unavailable comparisons select all checks.
pub fn select(
    _root: &Path,
    _base: Option<&str>,
    _head: &str,
    _force_full: bool,
) -> Result<Selection, String> {
    Err("CI selection implementation is in progress; the active caller remains Python".into())
}
