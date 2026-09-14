use gameskills_cli::agents::{execute, validate_configuration};
use serde_json::{Value, json};
use std::ffi::OsString;
use std::fs;

fn config() -> Value {
    json!({"agents":{"routing":{"default_tier":"small","escalation_after_failures":1,"max_attempts":2,"clients":{"codex":{"small":{"model":"m-small","effort":"low"},"standard":{"model":"m-standard","effort":"medium"},"strong":{"model":"m-strong","effort":"high"}}}}}})
}
fn host(dir: &std::path::Path) -> std::path::PathBuf {
    let path = dir.join("host.json");
    fs::write(&path, json!({"version":1,"client":"codex","models":[{"id":"m-small","efforts":["low"]},{"id":"m-standard","efforts":["medium"]},{"id":"m-strong","efforts":["high"]}]}).to_string()).unwrap();
    path
}
fn args(values: &[&str]) -> Vec<OsString> {
    values.iter().map(OsString::from).collect()
}

#[test]
fn small_default_and_reason_are_returned() {
    let dir = tempfile::tempdir().unwrap();
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
    .unwrap();
    assert_eq!(result["selection"]["tier"], "small");
    assert_eq!(result["selection"]["model"], "m-small");
    assert_eq!(result["selection"]["reason"], "lint");
}

#[test]
fn escalation_and_exhaustion_are_bounded() {
    let dir = tempfile::tempdir().unwrap();
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
    .unwrap();
    assert_eq!(result["selection"]["tier"], "standard");
    assert!(
        execute(
            dir.path(),
            &config(),
            &args(&["resolve", "--host", "host.json", "--attempt", "2"])
        )
        .is_err()
    );
}

#[test]
fn complex_starts_strong_and_capability_mismatch_fails() {
    let dir = tempfile::tempdir().unwrap();
    host(dir.path());
    let result = execute(
        dir.path(),
        &config(),
        &args(&["resolve", "--host", "host.json", "--kind", "complex"]),
    )
    .unwrap();
    assert_eq!(result["selection"]["tier"], "strong");
    let bad = json!({"version":1,"client":"codex","models":[{"id":"m-small","efforts":["low"]}]});
    fs::write(dir.path().join("bad.json"), bad.to_string()).unwrap();
    assert!(
        execute(
            dir.path(),
            &config(),
            &args(&["resolve", "--host", "bad.json", "--kind", "complex"])
        )
        .is_err()
    );
}

#[test]
fn invalid_configuration_and_legacy_config_are_handled() {
    let legacy: toml::Value = "[agents]\nlegacy = true\n".parse().unwrap();
    validate_configuration(&legacy).unwrap();
    let invalid: toml::Value = "[agents.routing]\ndefault_tier='small'\nescalation_after_failures=1\nmax_attempts=0\n[agents.routing.clients.codex.small]\nmodel=''\neffort='low'\n".parse().unwrap();
    assert!(validate_configuration(&invalid).is_err());
}

#[test]
fn unconfigured_routing_inherits_host_without_model_claim() {
    let dir = tempfile::tempdir().unwrap();
    host(dir.path());
    let result = execute(
        dir.path(),
        &json!({"agents":{"legacy":true}}),
        &args(&["resolve", "--host", "host.json"]),
    )
    .unwrap();
    assert_eq!(result["policy"], "inherit-host");
    assert!(result.get("selection").is_none());
}
