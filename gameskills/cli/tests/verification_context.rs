//! Persisted scope, policy applicability, and candidate-specific human observations.

use gameskills_cli::verification_context as context;
use serde_json::{json, Value};
use std::error::Error;

fn at<'a>(value: &'a Value, pointer: &str) -> &'a Value {
    value.pointer(pointer).unwrap_or(&Value::Null)
}

fn configuration() -> Value {
    json!({
        "schema_version": 1,
        "project": {"delivery_base": "dev"},
        "verification": {
            "default_level": "development",
            "manual_sanity": "milestone",
            "branches": {"dev": "development", "main": "testing"},
            "display": {"width": 1920, "height": 1080, "scale": "auto"},
            "levels": {
                "development": {"platforms": ["macos"]},
                "testing": {"platforms": ["macos"]},
                "release": {"platforms": ["macos", "windows", "linux"]}
            }
        }
    })
}

fn selection(
    config: &Value,
    base: &str,
    level: Option<&str>,
    scope: &[&str],
    gameplay: bool,
) -> Result<Value, String> {
    let scope: Vec<_> = scope.iter().map(|item| (*item).into()).collect();
    context::resolve(config, Some(base), level, &scope, gameplay)
}

fn promoted_selection(
    config: &Value,
    base: &str,
    level: Option<&str>,
    scope: &[&str],
    gameplay: bool,
) -> Result<Value, String> {
    let scope: Vec<_> = scope.iter().map(|item| (*item).into()).collect();
    context::resolve_with_promotion(config, Some(base), level, &scope, gameplay, true)
}

fn receipt(selection: &Value) -> Value {
    json!({
        "schema_version": 1,
        "task_id": "milestone",
        "source_head": "1111111111111111111111111111111111111111",
        "verification_digest": at(selection, "/selection_digest"),
        "result": "passed",
        "observer": "developer",
        "journey": "Load the saved scenario and finish the affected battle",
        "evidence_reference": "conversation:developer-sanity-result"
    })
}

#[test]
fn scope_is_canonical_and_manual_sanity_only_gates_gameplay_milestones(
) -> Result<(), Box<dyn Error>> {
    let config = configuration();
    let dev = selection(&config, "dev", None, &["rules", "session", "rules"], true)?;
    let canonical = selection(&config, "dev", None, &["session", "rules"], true)?;
    assert_eq!(dev, canonical);
    assert_eq!(at(&dev, "/scope"), &json!(["rules", "session"]));
    assert_eq!(at(&dev, "/manual_sanity_required"), false);
    assert_eq!(
        context::manual_observation("feature", "head", &dev, None)?,
        json!({"required": false})
    );

    let tooling = selection(&config, "main", None, &["ci-routing"], false)?;
    assert_eq!(at(&tooling, "/manual_sanity_required"), false);
    let ordinary_gameplay = selection(&config, "main", None, &["session"], true)?;
    assert_eq!(at(&ordinary_gameplay, "/manual_sanity_required"), false);
    assert_eq!(at(&ordinary_gameplay, "/promotion"), false);
    let milestone = promoted_selection(&config, "main", None, &["session"], true)?;
    assert_eq!(at(&milestone, "/manual_sanity_required"), true);
    assert_eq!(at(&milestone, "/promotion"), true);
    assert!(context::manual_observation("milestone", "head", &milestone, None).is_err());
    let release_on_dev = selection(&config, "dev", Some("release"), &["session"], true)?;
    assert_eq!(at(&release_on_dev, "/manual_sanity_required"), false);
    let release_on_main = promoted_selection(&config, "main", Some("release"), &["session"], true)?;
    assert_eq!(at(&release_on_main, "/manual_sanity_required"), true);

    for invalid in ["", " ", "line\nbreak", "nul\0byte"] {
        assert!(selection(&config, "dev", None, &[invalid], false).is_err());
    }
    Ok(())
}

#[test]
fn old_records_remain_legacy_and_require_explicit_adoption_of_new_policy(
) -> Result<(), Box<dyn Error>> {
    let legacy_config = json!({"schema_version": 1});
    assert_eq!(
        context::resolve(&legacy_config, None, None, &[], false)?,
        Value::Null
    );
    context::validate(&legacy_config, &Value::Null)?;
    assert!(context::resolve(&legacy_config, None, None, &["rules".into()], false).is_err());
    assert!(context::resolve(&legacy_config, None, None, &[], true).is_err());
    let configured = configuration();
    assert!(context::validate(&configured, &Value::Null).is_err());
    let recorded = selection(&configured, "dev", None, &["rules"], false)?;
    assert!(context::validate(&legacy_config, &recorded).is_err());
    assert!(context::covers(&Value::Null, &Value::Null));
    assert!(!context::covers(&recorded, &Value::Null));
    assert!(!context::covers(&Value::Null, &recorded));
    Ok(())
}

#[test]
fn evidence_must_cover_scope_and_base_at_the_same_or_higher_rigor() -> Result<(), Box<dyn Error>> {
    let config = configuration();
    let task = selection(&config, "dev", Some("testing"), &["rules", "session"], true)?;
    assert!(context::covers(&task, &task));
    let stronger = selection(
        &config,
        "dev",
        Some("release"),
        &["ui", "rules", "session"],
        false,
    )?;
    // Command observations do not attest to a human walkthrough.
    assert!(context::covers(&task, &stronger));
    for run in [
        selection(
            &config,
            "dev",
            Some("development"),
            &["rules", "session"],
            true,
        )?,
        selection(
            &config,
            "main",
            Some("testing"),
            &["rules", "session"],
            true,
        )?,
        selection(&config, "dev", Some("testing"), &["ui"], true)?,
        selection(&config, "dev", Some("testing"), &["rules"], true)?,
        selection(&config, "dev", Some("testing"), &[], false)?,
    ] {
        assert!(!context::covers(&task, &run), "{run}");
    }
    let mut changed = config.clone();
    *changed
        .pointer_mut("/verification/display/width")
        .expect("fixture field") = json!(2560);
    let other_policy = selection(
        &changed,
        "dev",
        Some("testing"),
        &["rules", "session"],
        true,
    )?;
    assert!(!context::covers(&task, &other_policy));
    Ok(())
}

#[test]
fn changed_policy_and_tampered_context_do_not_validate_as_current() -> Result<(), Box<dyn Error>> {
    let config = configuration();
    let recorded = selection(&config, "main", None, &["rules"], true)?;
    context::validate(&config, &recorded)?;
    let original = recorded.clone();
    let mut changed_config = config.clone();
    *changed_config
        .pointer_mut("/verification/manual_sanity")
        .expect("fixture field") = json!("never");
    assert!(context::validate(&changed_config, &recorded).is_err());
    assert_eq!(recorded, original);

    for (pointer, value) in [
        ("/policy/schema_version", json!(2)),
        ("/policy/policy_digest", json!("other-policy")),
        ("/policy/level", json!("development")),
        ("/policy/platforms", json!(["linux"])),
        ("/scope", json!(["session"])),
        ("/gameplay", json!(false)),
        ("/manual_sanity_required", json!(false)),
        ("/selection_digest", json!("other-selection")),
    ] {
        let mut altered = recorded.clone();
        *altered
            .pointer_mut(pointer)
            .ok_or("missing fixture pointer")? = value;
        assert!(context::validate(&config, &altered).is_err(), "{pointer}");
    }
    Ok(())
}

#[test]
fn manual_receipt_is_bound_to_task_candidate_policy_and_selected_journeys(
) -> Result<(), Box<dyn Error>> {
    let config = configuration();
    let milestone = promoted_selection(&config, "main", None, &["session"], true)?;
    let human = receipt(&milestone);
    let head = at(&human, "/source_head").as_str().ok_or("missing head")?;
    let observed = context::manual_observation("milestone", head, &milestone, Some(&human))?;
    assert_eq!(at(&observed, "/required"), true);
    assert_eq!(at(&observed, "/observation"), &human);
    assert!(at(&observed, "/claim")
        .as_str()
        .is_some_and(|claim| claim.contains("not independently authenticated")));

    for (field, wrong) in [
        ("schema_version", json!(2)),
        ("task_id", json!("other-task")),
        (
            "source_head",
            json!("2222222222222222222222222222222222222222"),
        ),
        ("verification_digest", json!("other-policy-scope")),
        ("result", json!("failed")),
        ("observer", json!(" ")),
        ("journey", json!("")),
        ("evidence_reference", json!("")),
    ] {
        let mut wrong_receipt = human.clone();
        *wrong_receipt
            .get_mut(field)
            .ok_or("missing receipt fixture field")? = wrong;
        assert!(
            context::manual_observation("milestone", head, &milestone, Some(&wrong_receipt))
                .is_err(),
            "{field}"
        );
    }

    let expanded = selection(&config, "main", None, &["session", "transport"], true)?;
    assert!(context::manual_observation("milestone", head, &expanded, Some(&human)).is_err());
    let mut changed_policy = config;
    *changed_policy
        .pointer_mut("/verification/display/height")
        .expect("fixture field") = json!(1440);
    let reconfigured = selection(&changed_policy, "main", None, &["session"], true)?;
    assert!(context::manual_observation("milestone", head, &reconfigured, Some(&human)).is_err());

    let development = selection(&changed_policy, "dev", None, &["session"], true)?;
    assert!(context::manual_observation("milestone", head, &development, Some(&human)).is_err());
    Ok(())
}
