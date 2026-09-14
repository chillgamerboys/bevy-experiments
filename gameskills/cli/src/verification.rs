//! Project-owned verification depth, independent of installation and affected scope.
//!
//! This resolver selects policy only. Project adapters select affected commands and
//! journeys, and separate observations establish execution and human acceptance.

use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use std::{
    collections::{BTreeMap, BTreeSet},
    ffi::OsString,
    path::Path,
};

#[derive(Clone, Copy, Debug, Deserialize, Eq, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(rename_all = "snake_case")]
enum Level {
    Development,
    Testing,
    Release,
}

impl Level {
    fn name(self) -> &'static str {
        match self {
            Self::Development => "development",
            Self::Testing => "testing",
            Self::Release => "release",
        }
    }
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(rename_all = "snake_case")]
enum Platform {
    Macos,
    Windows,
    Linux,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct LevelPolicy {
    platforms: Vec<Platform>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct Display {
    width: u32,
    height: u32,
    scale: String,
}

#[derive(Clone, Copy, Debug, Default, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
enum ManualSanity {
    #[default]
    Never,
    Milestone,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct Policy {
    default_level: Level,
    levels: BTreeMap<Level, LevelPolicy>,
    #[serde(default)]
    branches: BTreeMap<String, Level>,
    #[serde(default)]
    display: Option<Display>,
    #[serde(default)]
    manual_sanity: ManualSanity,
}

/// Validate a short receiving branch name without invoking Git.
pub fn validate_branch(branch: &str) -> Result<(), String> {
    if branch.is_empty()
        || branch == "HEAD"
        || branch == "@"
        || branch.starts_with('-')
        || branch.starts_with("refs/")
        || branch.ends_with('.')
        || branch.contains("..")
        || branch.contains("@{")
        || branch
            .bytes()
            .any(|b| b <= b' ' || b == 0x7f || b"~^:?*[\\".contains(&b))
        || branch
            .split('/')
            .any(|part| part.is_empty() || part.starts_with('.') || part.ends_with(".lock"))
    {
        return Err(format!(
            "expected a short receiving branch name: {branch:?}"
        ));
    }
    Ok(())
}

fn policy(value: &Value) -> Result<Policy, String> {
    let mut policy: Policy =
        serde_json::from_value(value.clone()).map_err(|error| format!("verification: {error}"))?;
    let mut previous = BTreeSet::new();
    for level in [Level::Development, Level::Testing, Level::Release] {
        let selected = policy
            .levels
            .get_mut(&level)
            .ok_or_else(|| format!("verification.levels.{} is required", level.name()))?;
        let platforms: BTreeSet<_> = selected.platforms.iter().copied().collect();
        if platforms.is_empty() || platforms.len() != selected.platforms.len() {
            return Err(format!(
                "verification.levels.{}.platforms must be nonempty without duplicates",
                level.name()
            ));
        }
        if !previous.is_subset(&platforms) {
            return Err(format!(
                "verification.levels.{} cannot remove platforms required by a lower level",
                level.name()
            ));
        }
        selected.platforms = platforms.iter().copied().collect();
        previous = platforms;
    }
    for branch in policy.branches.keys() {
        validate_branch(branch).map_err(|error| format!("verification.branches: {error}"))?;
    }
    if let Some(display) = &policy.display {
        if display.width == 0 || display.height == 0 || display.scale != "auto" {
            return Err(
                "verification.display requires positive width/height and scale = \"auto\"".into(),
            );
        }
    }
    Ok(policy)
}

pub(crate) fn validate_configuration(value: &toml::Value) -> Result<(), String> {
    policy(&serde_json::to_value(value).map_err(|error| error.to_string())?).map(|_| ())
}

/// Return the configured receiving branch, preserving `main` for existing adopters.
pub fn delivery_base(config: &Value) -> Result<String, String> {
    let base = match config.pointer("/project/delivery_base") {
        None => "main",
        Some(value) => value
            .as_str()
            .ok_or("project.delivery_base must be a branch name")?,
    };
    validate_branch(base).map_err(|error| format!("project.delivery_base: {error}"))?;
    Ok(base.into())
}

/// Resolve a versioned policy document. No installation, Git, commands or writes.
///
/// An absent policy stays explicitly unconfigured: consumers retain their existing
/// checks instead of relabeling historical behavior as Development. The digest binds
/// the complete normalized policy and default delivery base, independently of scope.
pub fn resolve(
    config: &Value,
    base: Option<&str>,
    requested_level: Option<&str>,
) -> Result<Value, String> {
    let default_base = delivery_base(config)?;
    let receiving_branch = base.unwrap_or(&default_base);
    validate_branch(receiving_branch)?;
    let mut reasons = vec![if base.is_some() {
        "receiving branch supplied explicitly".to_owned()
    } else if config.pointer("/project/delivery_base").is_some() {
        "receiving branch uses project.delivery_base".to_owned()
    } else {
        "receiving branch uses the legacy main default".to_owned()
    }];
    let selected_policy = config.get("verification").map(policy).transpose()?;
    let identity = json!({
        "schema_version": 1,
        "delivery_base": default_base,
        "verification": selected_policy,
    });
    let policy_digest = format!("{:x}", Sha256::digest(identity.to_string().as_bytes()));
    let Some(policy) = selected_policy else {
        if requested_level.is_some() {
            return Err("--level requires a project verification policy; existing checks remain unchanged without one".into());
        }
        reasons.push("no verification policy configured; preserve existing project checks".into());
        return Ok(json!({
            "schema_version": 1, "ok": true, "configured": false,
            "receiving_branch": receiving_branch, "branch_required_level": null,
            "level": null, "platforms": [], "display": null, "manual_sanity": "never",
            "policy_digest": policy_digest, "reasons": reasons,
        }));
    };
    let branch_requirement = policy.branches.get(receiving_branch).copied();
    let required_level = branch_requirement.unwrap_or(policy.default_level);
    let level = if let Some(requested) = requested_level {
        let requested: Level = serde_json::from_value(json!(requested))
            .map_err(|_| "level must be development, testing or release".to_owned())?;
        if requested < required_level {
            return Err(format!(
                "level {} cannot weaken receiving branch {receiving_branch} requirement {}",
                requested.name(),
                required_level.name()
            ));
        }
        reasons.push(format!("explicit {} level selected", requested.name()));
        requested
    } else if let Some(required) = branch_requirement {
        reasons.push(format!(
            "receiving branch requires {} verification",
            required.name()
        ));
        required
    } else {
        reasons.push(format!(
            "unmapped receiving branch uses configured {} default",
            policy.default_level.name()
        ));
        policy.default_level
    };
    let level_policy = policy
        .levels
        .get(&level)
        .ok_or("resolved verification level is missing")?;
    Ok(json!({
        "schema_version": 1, "ok": true, "configured": true,
        "receiving_branch": receiving_branch, "branch_required_level": branch_requirement,
        "level": level, "platforms": level_policy.platforms, "display": policy.display,
        "manual_sanity": policy.manual_sanity, "policy_digest": policy_digest,
        "reasons": reasons,
    }))
}

pub(crate) fn execute(root: &Path, args: &[OsString]) -> Result<Value, String> {
    const USAGE: &str =
        "use verification resolve [--base BRANCH] [--level development|testing|release]";
    if args.first().is_none_or(|arg| arg != "resolve") {
        return Err(USAGE.into());
    }
    let mut base = None;
    let mut level = None;
    let mut rest = args.iter().skip(1);
    while let Some(flag) = rest.next() {
        let slot = if flag == "--base" {
            &mut base
        } else if flag == "--level" {
            &mut level
        } else {
            return Err(USAGE.into());
        };
        if slot.is_some() {
            return Err("verification flags cannot be repeated".into());
        }
        *slot = Some(
            rest.next()
                .and_then(|value| value.to_str())
                .filter(|value| !value.starts_with('-'))
                .ok_or(USAGE)?,
        );
    }
    let source = crate::platform::read_ordinary_file(&root.join("gameskills.toml"))
        .map_err(|error| error.to_string())?;
    let config = crate::config::parse(&source)?;
    resolve(
        &serde_json::to_value(config).map_err(|error| error.to_string())?,
        base,
        level,
    )
}
