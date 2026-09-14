//! Model routing respects configured bounds and actual host capabilities.
use gameskills_cli::agents::{execute, validate_configuration};
use serde_json::{json, Value};
use std::ffi::OsString;
use std::fs;

fn config() -> Value {
    json!({"agents":{"routing":{"default_tier":"small","escalation_after_failures":1,"max_attempts":2,"clients":{"codex":{"small":{"model":"m-small","effort":"low"},"standard":{"model":"m-standard","effort":"medium"},"strong":{"model":"m-strong","effort":"high"}}}}}})
}
fn host(dir: &std::path::Path) -> std::path::PathBuf {
    let path = dir.join("host.json");
    fs::write(&path, json!({"version":1,"client":"codex","models":[{"id":"m-small","efforts":["low"]},{"id":"m-standard","efforts":["medium"]},{"id":"m-strong","efforts":["high"]}]}).to_string()).expect("valid test fixture");
    path
}
fn args(values: &[&str]) -> Vec<OsString> {
    values.iter().map(OsString::from).collect()
}

#[test]
fn small_default_and_reason_are_returned() {
    let dir = tempfile::tempdir().expect("valid test fixture");
    host(dir.path());
    let result = execute(
        dir.path(),
        &config(),
        &args(&[
            "resolve",
            "--host",
            "host.json",
            "--kind",
            "bounded",
            "--reason",
            "lint",
        ]),
    )
    .expect("valid test fixture");
    assert_eq!(
        result.pointer("/selection/tier").expect("selected tier"),
        "small"
    );
    assert_eq!(
        result.pointer("/selection/model").expect("selected model"),
        "m-small"
    );
    assert_eq!(
        result
            .pointer("/selection/reason")
            .expect("selection reason"),
        "lint"
    );
}

#[test]
fn configured_routing_defaults_to_bounded_small() {
    let dir = tempfile::tempdir().expect("valid test fixture");
    host(dir.path());
    let mut value = config();
    let routing = value
        .pointer_mut("/agents/routing")
        .expect("routing")
        .as_object_mut()
        .expect("valid test fixture");
    routing.remove("default_tier");
    routing.remove("escalation_after_failures");
    routing.remove("max_attempts");
    let result = execute(
        dir.path(),
        &value,
        &args(&["resolve", "--host", "host.json"]),
    )
    .expect("valid test fixture");
    assert_eq!(
        result.pointer("/selection/tier").expect("selected tier"),
        "small"
    );
}

#[test]
fn escalation_and_exhaustion_are_bounded() {
    let dir = tempfile::tempdir().expect("valid test fixture");
    host(dir.path());
    let result = execute(
        dir.path(),
        &config(),
        &args(&[
            "resolve",
            "--host",
            "host.json",
            "--kind",
            "bounded",
            "--attempt",
            "1",
        ]),
    )
    .expect("valid test fixture");
    assert_eq!(
        result.pointer("/selection/tier").expect("selected tier"),
        "standard"
    );
    assert!(execute(
        dir.path(),
        &config(),
        &args(&["resolve", "--host", "host.json", "--attempt", "2"])
    )
    .is_err());
}

#[test]
fn complex_starts_strong_and_capability_mismatch_fails() {
    let dir = tempfile::tempdir().expect("valid test fixture");
    host(dir.path());
    let result = execute(
        dir.path(),
        &config(),
        &args(&["resolve", "--host", "host.json", "--kind", "complex"]),
    )
    .expect("valid test fixture");
    assert_eq!(
        result.pointer("/selection/tier").expect("selected tier"),
        "strong"
    );
    let bad = json!({"version":1,"client":"codex","models":[{"id":"m-small","efforts":["low"]}]});
    fs::write(dir.path().join("bad.json"), bad.to_string()).expect("valid test fixture");
    assert!(execute(
        dir.path(),
        &config(),
        &args(&["resolve", "--host", "bad.json", "--kind", "complex"])
    )
    .is_err());
}

#[test]
fn invalid_configuration_and_legacy_config_are_handled() {
    let legacy: toml::Value = "[agents]\nlegacy = true\n"
        .parse::<toml::Table>()
        .map(toml::Value::Table)
        .expect("valid test fixture");
    validate_configuration(&legacy).expect("valid test fixture");
    let invalid: toml::Value = "[agents.routing]\ndefault_tier='small'\nescalation_after_failures=1\nmax_attempts=0\n[agents.routing.clients.codex.small]\nmodel=''\neffort='low'\n".parse::<toml::Table>().map(toml::Value::Table).expect("valid test fixture");
    assert!(validate_configuration(&invalid).is_err());
}

#[test]
fn invalid_types_and_unknown_keys_are_rejected() {
    let wrong_type: toml::Value = "[agents.routing]\nmax_attempts='2'\n[agents.routing.clients.codex.small]\nmodel='m'\neffort='low'\n".parse::<toml::Table>().map(toml::Value::Table).expect("valid test fixture");
    assert!(validate_configuration(&wrong_type).is_err());
    let unknown: toml::Value = "[agents.routing]\nmax_atempts=2\n[agents.routing.clients.codex.small]\nmodel='m'\neffort='low'\n".parse::<toml::Table>().map(toml::Value::Table).expect("valid test fixture");
    assert!(validate_configuration(&unknown).is_err());
}

#[test]
fn explicit_tier_override_requires_reason() {
    let dir = tempfile::tempdir().expect("valid test fixture");
    host(dir.path());
    assert!(execute(
        dir.path(),
        &config(),
        &args(&["resolve", "--host", "host.json", "--tier", "strong"])
    )
    .is_err());
}

#[test]
fn unconfigured_routing_inherits_host_without_model_claim() {
    let dir = tempfile::tempdir().expect("valid test fixture");
    host(dir.path());
    let result = execute(
        dir.path(),
        &json!({"agents":{"legacy":true}}),
        &args(&["resolve", "--host", "host.json"]),
    )
    .expect("valid test fixture");
    assert_eq!(result.get("policy").expect("policy"), "inherit-host");
    assert!(result.get("selection").is_none());
}
