//! Persisted verification selection, separate from command and human observations.
use serde_json::{json, Value};

/// Resolve policy and retain caller-classified behavior without claiming execution.
pub fn resolve(
    config: &Value,
    base: Option<&str>,
    level: Option<&str>,
    scope: &[String],
    gameplay: bool,
) -> Result<Value, String> {
    let policy = crate::verification::resolve(config, base, level)?;
    if policy.get("configured") != Some(&json!(true)) {
        if !scope.is_empty() || gameplay {
            return Err("verification scope requires configured verification policy".into());
        }
        return Ok(Value::Null);
    }
    let mut scope = scope.to_vec();
    if scope
        .iter()
        .any(|s| s.trim().is_empty() || s.chars().any(char::is_control))
    {
        return Err(
            "verification scope must contain nonempty descriptions without control characters"
                .into(),
        );
    }
    scope.sort();
    scope.dedup();
    let manual_required = gameplay
        && policy.get("manual_sanity").and_then(Value::as_str) == Some("milestone")
        && policy.get("branch_required_level").and_then(Value::as_str) == Some("testing");
    use sha2::{Digest, Sha256};
    let digest = format!("{:x}", Sha256::digest(json!({"policy_digest":policy.get("policy_digest"), "base":policy.get("receiving_branch"), "level":policy.get("level"), "scope":scope, "gameplay":gameplay}).to_string().as_bytes()));
    Ok(
        json!({"selection_digest":digest,"policy":policy,"scope":scope,"gameplay":gameplay,
        "manual_sanity_required":manual_required,
        "claim":"resolved policy and caller-classified scope; not execution or human acceptance"}),
    )
}

/// Recompute a recorded selection without reinterpreting historical policy.
pub fn validate(config: &Value, recorded: &Value) -> Result<(), String> {
    if recorded.is_null() {
        if config.get("verification").is_some() {
            return Err(
                "verification policy is not recorded; bind current scope or run fresh checks"
                    .into(),
            );
        }
        return Ok(());
    }
    let policy = recorded
        .get("policy")
        .ok_or("missing verification policy")?;
    let base = policy
        .get("receiving_branch")
        .and_then(Value::as_str)
        .ok_or("missing verification receiving branch")?;
    let level = policy
        .get("level")
        .and_then(Value::as_str)
        .ok_or("missing verification level")?;
    let scope: Vec<String> = serde_json::from_value(
        recorded
            .get("scope")
            .cloned()
            .ok_or("missing verification scope")?,
    )
    .map_err(|_| "invalid verification scope")?;
    let gameplay = recorded
        .get("gameplay")
        .and_then(Value::as_bool)
        .ok_or("missing verification gameplay classification")?;
    let current = resolve(config, Some(base), Some(level), &scope, gameplay)?;
    for key in [
        "schema_version",
        "configured",
        "policy_digest",
        "receiving_branch",
        "branch_required_level",
        "level",
        "platforms",
        "display",
        "manual_sanity",
    ] {
        if policy.get(key) != current.get("policy").and_then(|p| p.get(key)) {
            return Err(format!(
                "verification policy changed: {key}; resolve scope and run applicable checks"
            ));
        }
    }
    for key in [
        "selection_digest",
        "scope",
        "gameplay",
        "manual_sanity_required",
    ] {
        if recorded.get(key) != current.get(key) {
            return Err(format!("verification selection changed: {key}"));
        }
    }
    Ok(())
}

/// Commands for a different base or weaker level cannot satisfy task evidence.
pub fn covers(task: &Value, run: &Value) -> bool {
    if task.is_null() {
        return run.is_null();
    }
    let rank = |v: &Value| match v.pointer("/policy/level").and_then(Value::as_str) {
        Some("development") => 1,
        Some("testing") => 2,
        Some("release") => 3,
        _ => 0,
    };
    !run.is_null()
        && task.pointer("/policy/policy_digest") == run.pointer("/policy/policy_digest")
        && task.pointer("/policy/receiving_branch") == run.pointer("/policy/receiving_branch")
        && rank(run) >= rank(task)
        && task
            .get("scope")
            .and_then(Value::as_array)
            .is_some_and(|required| {
                run.get("scope")
                    .and_then(Value::as_array)
                    .is_some_and(|actual| required.iter().all(|item| actual.contains(item)))
            })
}

/// Validate a supplied human observation's binding, never authenticate the person.
pub fn manual_observation(
    task_id: &str,
    head: &str,
    context: &Value,
    observation: Option<&Value>,
) -> Result<Value, String> {
    if context.get("manual_sanity_required") != Some(&json!(true)) {
        if observation.is_some() {
            return Err(
                "manual sanity observation is only applicable to a gameplay milestone".into(),
            );
        }
        return Ok(json!({"required":false}));
    }
    let v = observation
        .ok_or("milestone requires the developer's candidate-specific manual sanity result")?;
    if v.get("schema_version") != Some(&json!(1))
        || v.get("task_id").and_then(Value::as_str) != Some(task_id)
        || v.get("source_head").and_then(Value::as_str) != Some(head)
        || v.get("verification_digest") != context.get("selection_digest")
        || v.get("result").and_then(Value::as_str) != Some("passed")
    {
        return Err(
            "manual sanity observation is failed, stale, or bound to another task/candidate".into(),
        );
    }
    for key in ["observer", "journey", "evidence_reference"] {
        if !v
            .get(key)
            .and_then(Value::as_str)
            .is_some_and(|s| !s.trim().is_empty())
        {
            return Err(format!("manual sanity observation requires {key}"));
        }
    }
    Ok(
        json!({"required":true,"observation":v,"claim":"caller-supplied developer observation; bindings checked, person and result not independently authenticated"}),
    )
}
