//! Consume the standalone GameSkills resolver protocol without depending on its crate.

use super::{selector, suites, Selection};
use serde_json::Value;
use std::path::Path;

/// Validate the versioned resolver observation, including the branch's minimum rigor.
pub fn validate(policy: &Value) -> Result<(), String> {
    if policy.get("schema_version").and_then(Value::as_u64) != Some(1)
        || policy.get("ok").and_then(Value::as_bool) != Some(true)
    {
        return Err("unsupported or unsuccessful verification policy".into());
    }
    let configured = policy
        .get("configured")
        .and_then(Value::as_bool)
        .ok_or("verification policy needs configured boolean")?;
    let branch = policy
        .get("receiving_branch")
        .and_then(Value::as_str)
        .filter(|value| !value.is_empty() && !value.starts_with('-'))
        .ok_or("verification policy needs receiving_branch")?;
    let _ = branch;
    let platforms = policy
        .get("platforms")
        .and_then(Value::as_array)
        .ok_or("verification policy needs platforms")?;
    if !configured {
        if !policy.get("level").is_some_and(Value::is_null) || !platforms.is_empty() {
            return Err("legacy verification policy cannot select a level or platforms".into());
        }
        return Ok(());
    }
    let rank = |value: &str| match value {
        "development" => Some(0),
        "testing" => Some(1),
        "release" => Some(2),
        _ => None,
    };
    let level = policy
        .get("level")
        .and_then(Value::as_str)
        .and_then(rank)
        .ok_or("invalid verification level")?;
    if let Some(required) = policy.get("branch_required_level").and_then(Value::as_str) {
        if level < rank(required).ok_or("invalid required verification level")? {
            return Err("verification level is below receiving-branch acceptance".into());
        }
    }
    let mut seen = std::collections::BTreeSet::new();
    for platform in platforms {
        let platform = platform.as_str().ok_or("invalid verification platform")?;
        if !matches!(platform, "macos" | "linux" | "windows") || !seen.insert(platform) {
            return Err("unknown or duplicate verification platform".into());
        }
    }
    if seen.is_empty() {
        return Err("configured verification requires a platform".into());
    }
    let digest = policy
        .get("policy_digest")
        .and_then(Value::as_str)
        .unwrap_or_default();
    if digest.len() != 64 || !digest.bytes().all(|b| b.is_ascii_hexdigit()) {
        return Err("verification policy needs its SHA-256 identity".into());
    }
    Ok(())
}

/// Configured level; `None` preserves pre-policy behavior.
pub fn level(selection: &Selection) -> Option<&str> {
    selection
        .verification
        .as_ref()
        .filter(|p| p["configured"] == true)
        .and_then(|p| p["level"].as_str())
}

/// Runner names are derived from resolved policy, never inferred from affected scope.
pub fn runners(selection: &Selection) -> Vec<String> {
    let Some(policy) = selection
        .verification
        .as_ref()
        .filter(|p| p["configured"] == true)
    else {
        return ["ubuntu-latest", "macos-latest", "windows-latest"]
            .map(String::from)
            .to_vec();
    };
    policy["platforms"]
        .as_array()
        .into_iter()
        .flatten()
        .filter_map(|p| match p.as_str() {
            Some("macos") => Some("macos-latest".into()),
            Some("linux") => Some("ubuntu-latest".into()),
            Some("windows") => Some("windows-latest".into()),
            _ => None,
        })
        .collect()
}

/// Add receiving-project rigor to the existing committed impact selection.
pub fn apply(root: &Path, selection: &mut Selection, policy: Value) -> Result<(), String> {
    validate(&policy)?;
    selection.verification = Some(policy);
    let Some(level) = level(selection).map(str::to_owned) else {
        return Ok(());
    };
    let narrative_doc = |path: &str| {
        path == "README.md"
            || path == "devtools/docs/ci.md"
            || ["docs/", "gamekit/docs/", "gameskills/docs/"]
                .iter()
                .any(|prefix| path.starts_with(prefix) && path.ends_with(".md"))
            || (path.starts_with("games/") && path.ends_with("/README.md"))
    };
    let development_ci_only = level == "development"
        && selection.full
        && selection.reasons.iter().any(|reason| {
            reason.starts_with("Conservative fallback: shared configuration or CI input:")
        })
        && selection.paths.iter().any(|path| !narrative_doc(path))
        && selection.paths.iter().all(|path| {
            narrative_doc(path)
                || path == ".github/workflows/gamekit.yml"
                || path.starts_with("devtools/src/ci/")
                || matches!(
                    path.as_str(),
                    "devtools/tests/ci_routing.rs"
                        | "devtools/tests/ci_checks.rs"
                        | "devtools/tests/ci_cli.rs"
                )
        });
    if development_ci_only {
        selection.full = false;
        selection.packages = vec!["repo-devtools".into()];
        selection.skills = false;
        selection.rust = false;
        selection.policy = true;
        selection
            .reasons
            .push("Development: bounded CI controller and workflow tooling".into());
    } else if selection.full {
        selection.packages = selector::package_names(root, &selection.head)?;
    }
    // Compatibility/distributable probes are release work. An uncertain diff expands
    // owners, not platforms, display cases or unrelated end-to-end journeys.
    if level != "release" {
        selection.distribution = false;
        selection.minimal = false;
        selection.wasm = false;
        selection.deny = false;
    }
    selection.suites = suites::select(selection);
    suites::validate_changes(root, selection)?;
    selection.gameplay_affected = suites::gameplay_affected(root, selection);
    selection.reasons.push(format!(
        "Verification: {level}; positive suites: {}",
        selection.suites.join(", ")
    ));
    Ok(())
}
