//! Bounded model selection for agent clients.
//!
//! This module only resolves a requested model against capabilities supplied by
//! the host. It does not launch a client or claim which model was observed.

use serde_json::{json, Value};
use std::ffi::OsString;
use std::fs;
use std::path::Path;

const TIERS: [&str; 3] = ["small", "standard", "strong"];

/// Resolve a bounded model request. `args` contains the arguments after
/// `agents`, and `host` is resolved relative to `root` unless absolute.
pub fn execute(root: &Path, config: &Value, args: &[OsString]) -> Result<Value, String> {
    let parsed = parse_args(args)?;
    let client = parsed.client.as_deref().unwrap_or("codex");
    let routing = config.get("agents").and_then(|v| v.get("routing"));

    let host_path = parsed
        .host
        .ok_or_else(|| "--host is required".to_string())?;
    let host_path = if host_path.is_absolute() {
        host_path
    } else {
        root.join(host_path)
    };
    let host: Value = serde_json::from_slice(
        &fs::read(&host_path).map_err(|e| format!("read host capabilities: {e}"))?,
    )
    .map_err(|e| format!("invalid host capabilities JSON: {e}"))?;
    validate_host(&host, client)?;

    let reason = parsed
        .reason
        .unwrap_or_else(|| "caller did not provide a reason".into());
    if parsed.override_tier.is_some() && reason == "caller did not provide a reason" {
        return Err("--tier requires --reason".into());
    }
    let Some(routing) = routing else {
        return Ok(json!({
            "schema_version": 1,
            "ok": true,
            "scope": "agent_model_resolution",
            "client": client,
            "policy": "inherit-host",
            "reason": reason,
        }));
    };
    let policy = parse_policy(routing, client)?;
    let requested = parsed.kind.as_deref().unwrap_or("bounded");
    if !matches!(requested, "bounded" | "standard" | "complex") {
        return Err(format!(
            "invalid --kind {requested:?}; expected bounded, standard, or complex"
        ));
    }
    let mut tier = if requested == "complex" {
        "strong"
    } else if requested == "bounded" {
        policy.default_tier.as_str()
    } else {
        "standard"
    };
    if let Some(override_tier) = parsed.override_tier.as_deref() {
        if !TIERS.contains(&override_tier) {
            return Err(format!("invalid --tier {override_tier:?}"));
        }
        tier = override_tier;
    }
    let attempt = parsed.attempt.unwrap_or(0);
    if attempt >= policy.max_attempts {
        return Err(format!(
            "attempt {attempt} is outside max_attempts {}",
            policy.max_attempts
        ));
    }
    if attempt >= policy.escalation_after_failures {
        tier = escalate(tier, attempt - policy.escalation_after_failures + 1);
    }
    let selected = policy
        .tiers
        .get(tier)
        .ok_or_else(|| format!("missing tier {tier}"))?;
    require_capability(&host, client, &selected.model, &selected.effort)?;
    Ok(json!({
        "schema_version": 1,
        "ok": true,
        "scope": "agent_model_resolution",
        "client": client,
        "policy": "configured",
        "selection": {"model": selected.model, "effort": selected.effort, "tier": tier, "reason": reason},
        "attempt": attempt,
        "override": parsed.override_tier,
    }))
}

/// Validate the optional routing table while preserving unrelated agent keys.
pub fn validate_configuration(config: &toml::Value) -> Result<(), String> {
    let Some(routing) = config.get("agents").and_then(|v| v.get("routing")) else {
        return Ok(());
    };
    let value = serde_json::to_value(routing).map_err(|e| e.to_string())?;
    for client in client_names(&value)? {
        parse_policy(&value, &client)?;
    }
    Ok(())
}

fn client_names(value: &Value) -> Result<Vec<String>, String> {
    let clients = value.get("clients").ok_or("routing.clients is required")?;
    let object = clients
        .as_object()
        .ok_or("routing.clients must be a table")?;
    if object.is_empty() {
        return Err("routing.clients must not be empty".into());
    }
    Ok(object.keys().cloned().collect())
}

struct Tier {
    model: String,
    effort: String,
}
struct Policy {
    default_tier: String,
    escalation_after_failures: usize,
    max_attempts: usize,
    tiers: std::collections::HashMap<String, Tier>,
}

fn parse_policy(value: &Value, client: &str) -> Result<Policy, String> {
    let allowed = [
        "default_tier",
        "escalation_after_failures",
        "max_attempts",
        "clients",
    ];
    if let Some(object) = value.as_object() {
        for key in object.keys() {
            if !allowed.contains(&key.as_str()) {
                return Err(format!("unknown routing key {key:?}"));
            }
        }
    }
    let default_tier = match value.get("default_tier") {
        None => "small",
        Some(value) => value
            .as_str()
            .ok_or("routing.default_tier must be a string")?,
    };
    if !TIERS.contains(&default_tier) {
        return Err(format!("invalid default_tier {default_tier:?}"));
    }
    let failures = match value.get("escalation_after_failures") {
        None => 1,
        Some(value) => value
            .as_u64()
            .ok_or("routing.escalation_after_failures must be a nonnegative integer")?
            as usize,
    };
    let max = match value.get("max_attempts") {
        None => 2,
        Some(value) => value
            .as_u64()
            .ok_or("routing.max_attempts must be a nonnegative integer")?
            as usize,
    };
    if max == 0 || max > 100 {
        return Err("routing.max_attempts must be between 1 and 100".into());
    }
    if failures == 0 || failures > max {
        return Err("routing.escalation_after_failures must be between 1 and max_attempts".into());
    }
    let client_value = value
        .pointer(&format!("/clients/{client}"))
        .ok_or_else(|| format!("routing client {client:?} is not configured"))?;
    let client_object = client_value
        .as_object()
        .ok_or_else(|| format!("routing client {client:?} must be a table"))?;
    for key in client_object.keys() {
        if !TIERS.contains(&key.as_str()) {
            return Err(format!("unknown tier {client}.{key}"));
        }
    }
    let mut tiers = std::collections::HashMap::new();
    for tier in TIERS {
        let item = client_value
            .get(tier)
            .ok_or_else(|| format!("routing client {client:?} missing tier {tier}"))?;
        let item_object = item
            .as_object()
            .ok_or_else(|| format!("{client}.{tier} must be a table"))?;
        for key in item_object.keys() {
            if key != "model" && key != "effort" {
                return Err(format!("unknown key {client}.{tier}.{key}"));
            }
        }
        let model = item
            .get("model")
            .and_then(Value::as_str)
            .filter(|x| !x.is_empty())
            .ok_or_else(|| format!("{client}.{tier}.model must be nonempty"))?
            .to_string();
        let effort = item
            .get("effort")
            .and_then(Value::as_str)
            .filter(|x| !x.is_empty())
            .ok_or_else(|| format!("{client}.{tier}.effort must be nonempty"))?
            .to_string();
        tiers.insert(tier.to_string(), Tier { model, effort });
    }
    Ok(Policy {
        default_tier: default_tier.into(),
        escalation_after_failures: failures,
        max_attempts: max,
        tiers,
    })
}

fn escalate(tier: &str, count: usize) -> &str {
    let index = TIERS.iter().position(|x| *x == tier).unwrap_or(1);
    TIERS
        .get(index.saturating_add(count))
        .copied()
        .unwrap_or("strong")
}

fn validate_host(host: &Value, client: &str) -> Result<(), String> {
    let version = host.get("version").or_else(|| host.get("schema_version"));
    if version.and_then(Value::as_u64) != Some(1) {
        return Err("host capabilities version must be 1".into());
    }
    if host.get("client").and_then(Value::as_str) != Some(client) {
        return Err("host capabilities client does not match --client".into());
    }
    if host.get("models").and_then(Value::as_array).is_none() {
        return Err("host capabilities models must be an array".into());
    }
    Ok(())
}

fn require_capability(
    host: &Value,
    _client: &str,
    model: &str,
    effort: &str,
) -> Result<(), String> {
    let supported = host
        .get("models")
        .and_then(Value::as_array)
        .ok_or("host models must be an array")?
        .iter()
        .any(|entry| {
            entry.get("id").and_then(Value::as_str) == Some(model)
                && entry
                    .get("efforts")
                    .and_then(Value::as_array)
                    .map(|a| a.iter().any(|e| e.as_str() == Some(effort)))
                    .unwrap_or(false)
        });
    if supported {
        Ok(())
    } else {
        Err(format!(
            "host does not support model {model:?} with effort {effort:?}"
        ))
    }
}

struct Parsed {
    client: Option<String>,
    host: Option<std::path::PathBuf>,
    kind: Option<String>,
    attempt: Option<usize>,
    reason: Option<String>,
    override_tier: Option<String>,
}
fn parse_args(args: &[OsString]) -> Result<Parsed, String> {
    let mut p = Parsed {
        client: None,
        host: None,
        kind: None,
        attempt: None,
        reason: None,
        override_tier: None,
    };
    let mut i = 0;
    while i < args.len() {
        let key = args
            .get(i)
            .ok_or("missing agents argument")?
            .to_str()
            .ok_or("agents arguments must be UTF-8")?;
        let take = |i: &mut usize, key: &str, args: &[OsString]| -> Result<String, String> {
            *i += 1;
            args.get(*i)
                .map(|x| x.to_string_lossy().into_owned())
                .ok_or_else(|| format!("{key} requires a value"))
        };
        match key {
            "resolve" => {}
            "--client" => p.client = Some(take(&mut i, "--client", args)?),
            "--host" => p.host = Some(take(&mut i, "--host", args)?.into()),
            "--kind" => p.kind = Some(take(&mut i, "--kind", args)?),
            "--attempt" => {
                p.attempt = Some(
                    take(&mut i, "--attempt", args)?
                        .parse()
                        .map_err(|_| "--attempt must be a nonnegative integer")?,
                )
            }
            "--reason" => p.reason = Some(take(&mut i, "--reason", args)?),
            "--tier" => p.override_tier = Some(take(&mut i, "--tier", args)?),
            other => return Err(format!("unexpected agents argument {other:?}")),
        }
        i += 1;
    }
    Ok(p)
}
