//! Verification policy is deterministic, standalone and independent of task creativity.

use gameskills_cli::{config, verification};
use serde_json::{json, Value};
use std::{error::Error, fs, process::Command};

fn at<'a>(value: &'a Value, pointer: &str) -> &'a Value {
    value.pointer(pointer).unwrap_or(&Value::Null)
}

const POLICY: &str = r#"
schema_version = 1
[project]
delivery_base = "dev"
[verification]
default_level = "development"
manual_sanity = "milestone"
[verification.branches]
dev = "development"
main = "testing"
[verification.display]
width = 1920
height = 1080
scale = "auto"
[verification.levels.development]
platforms = ["macos"]
[verification.levels.testing]
platforms = ["macos"]
[verification.levels.release]
platforms = ["macos", "windows", "linux"]
"#;

fn parsed(source: &str) -> Result<Value, String> {
    serde_json::to_value(config::parse(source)?).map_err(|error| error.to_string())
}

#[test]
fn branches_defaults_and_explicit_levels_preserve_receiving_requirements(
) -> Result<(), Box<dyn Error>> {
    let config = parsed(POLICY)?;
    let development = verification::resolve(&config, None, None)?;
    assert_eq!(at(&development, "/receiving_branch"), "dev");
    assert_eq!(at(&development, "/level"), "development");
    assert_eq!(at(&development, "/platforms"), &json!(["macos"]));
    assert_eq!(
        at(&development, "/display"),
        &json!({"width":1920,"height":1080,"scale":"auto"})
    );
    assert_eq!(at(&development, "/manual_sanity"), "milestone");

    let testing = verification::resolve(&config, Some("main"), None)?;
    assert_eq!(at(&testing, "/branch_required_level"), "testing");
    assert_eq!(at(&testing, "/level"), "testing");
    assert_eq!(at(&testing, "/platforms"), &json!(["macos"]));
    let error = verification::resolve(&config, Some("main"), Some("development"))
        .expect_err("main cannot be weakened");
    assert!(error.contains("cannot weaken receiving branch main requirement testing"));

    let release = verification::resolve(&config, Some("dev"), Some("release"))?;
    assert_eq!(at(&release, "/level"), "release");
    assert_eq!(
        at(&release, "/platforms"),
        &json!(["macos", "windows", "linux"])
    );
    assert_eq!(
        at(&release, "/policy_digest"),
        at(&development, "/policy_digest")
    );
    assert!(release.get("publication_authorized").is_none());

    let unmapped = verification::resolve(&config, Some("milestones/experiment"), None)?;
    assert_eq!(at(&unmapped, "/level"), "development");
    assert!(at(&unmapped, "/branch_required_level").is_null());
    assert!(at(&unmapped, "/reasons")
        .to_string()
        .contains("unmapped receiving branch"));
    let testing_default = parsed(&POLICY.replace(
        "default_level = \"development\"",
        "default_level = \"testing\"",
    ))?;
    assert!(
        verification::resolve(&testing_default, Some("unmapped"), Some("development")).is_err()
    );
    assert!(verification::resolve(&config, None, Some("creative")).is_err());
    Ok(())
}

#[test]
fn absent_policy_does_not_relabel_old_adopters_or_require_new_configuration(
) -> Result<(), Box<dyn Error>> {
    let config = parsed("schema_version=1\n")?;
    assert!(config.get("verification").is_none());
    assert!(config.get("project").is_none());
    assert_eq!(verification::delivery_base(&config)?, "main");
    let legacy = verification::resolve(&config, None, None)?;
    assert_eq!(at(&legacy, "/configured"), false);
    assert_eq!(at(&legacy, "/receiving_branch"), "main");
    assert!(at(&legacy, "/level").is_null());
    assert!(at(&legacy, "/display").is_null());
    assert_eq!(at(&legacy, "/platforms"), &json!([]));
    assert_eq!(at(&legacy, "/manual_sanity"), "never");
    assert!(verification::resolve(&config, None, Some("development")).is_err());
    assert_eq!(
        at(
            &verification::resolve(&config, Some("maintenance"), None)?,
            "/receiving_branch"
        ),
        "maintenance"
    );
    let headless = parsed(
        &POLICY
            .replace("manual_sanity = \"milestone\"\n", "")
            .replace(
                "[verification.display]\nwidth = 1920\nheight = 1080\nscale = \"auto\"\n",
                "",
            ),
    )?;
    let headless = verification::resolve(&headless, None, None)?;
    assert_eq!(at(&headless, "/configured"), true);
    assert!(at(&headless, "/display").is_null());
    assert_eq!(at(&headless, "/manual_sanity"), "never");
    Ok(())
}

#[test]
fn policy_identity_binds_all_levels_but_not_creative_level_or_toml_spelling(
) -> Result<(), Box<dyn Error>> {
    let identity = |source: &str| -> Result<Value, String> {
        Ok(at(
            &verification::resolve(&parsed(source)?, None, None)?,
            "/policy_digest",
        )
        .clone())
    };
    let original = identity(POLICY)?;
    assert_eq!(original.as_str().map(str::len), Some(64));
    assert_eq!(
        identity(&format!("{POLICY}\n[creative]\ndefault_level=1\n"))?,
        original
    );
    assert_eq!(
        identity(&format!("# Formatting is not policy\n{POLICY}\n"))?,
        original
    );
    assert_eq!(
        identity(&POLICY.replace(
            "[\"macos\", \"windows\", \"linux\"]",
            "[\"linux\", \"macos\", \"windows\"]"
        ))?,
        original
    );
    for changed in [
        POLICY.replace("delivery_base = \"dev\"", "delivery_base = \"main\""),
        POLICY.replace("main = \"testing\"", "main = \"release\""),
        POLICY.replace("manual_sanity = \"milestone\"", "manual_sanity = \"never\""),
        POLICY.replace("width = 1920", "width = 2560"),
        POLICY.replace(
            "[\"macos\", \"windows\", \"linux\"]",
            "[\"macos\", \"linux\"]",
        ),
    ] {
        assert_ne!(identity(&changed)?, original, "{changed}");
    }
    Ok(())
}

#[test]
fn invalid_verification_configuration_is_rejected_before_installation() {
    for source in [
        POLICY.replace(
            "default_level = \"development\"",
            "default_level = \"fast\"",
        ),
        POLICY.replace(
            "manual_sanity = \"milestone\"",
            "manual_sanity = \"always\"",
        ),
        POLICY.replace("main = \"testing\"", "main = \"fast\""),
        POLICY.replace("width = 1920", "width = 0"),
        POLICY.replace("height = 1080", "height = -1"),
        POLICY.replace("scale = \"auto\"", "scale = \"retina\""),
        POLICY.replace("[\"macos\"]", "[]"),
        POLICY.replace("[\"macos\"]", "[\"macos\", \"macos\"]"),
        POLICY.replace("[\"macos\"]", "[\"android\"]"),
        POLICY.replace(
            "[verification.levels.testing]\nplatforms = [\"macos\"]",
            "[verification.levels.testing]\nplatforms = [\"linux\"]",
        ),
        POLICY.replace("[verification.levels.testing]\nplatforms = [\"macos\"]", ""),
        POLICY.replace("[verification]", "[verification]\nunknown = true"),
        POLICY.replace(
            "[verification.display]",
            "[verification.display]\nsweep = true",
        ),
        POLICY.replace(
            "[verification.levels.release]",
            "[verification.levels.release]\nall_journeys = true",
        ),
    ] {
        let error = config::parse(&source).expect_err(&source);
        assert!(error.contains("verification"), "{error}");
    }
    for branch in [
        "",
        "HEAD",
        "@",
        "-main",
        "refs/heads/main",
        "with space",
        "one..two",
        "a@{b",
        "a.lock",
        "a/.b",
        "a/",
        "a\\b",
    ] {
        let source = format!(
            "schema_version=1\n[project]\ndelivery_base={}\n",
            json!(branch)
        );
        assert!(config::parse(&source).is_err(), "{branch}");
        assert!(
            verification::resolve(&json!({}), Some(branch), None).is_err(),
            "{branch}"
        );
    }
    assert!(config::parse("schema_version=1\n[project]\ndelivery_base=3\n").is_err());
    assert!(
        config::parse(&POLICY.replace("main = \"testing\"", "\"bad branch\" = \"testing\""))
            .is_err()
    );
}

#[test]
fn standalone_resolution_needs_no_git_installation_or_state_writes() -> Result<(), Box<dyn Error>> {
    let directory = tempfile::tempdir()?;
    let path = directory.path().join("gameskills.toml");
    fs::write(&path, POLICY)?;
    let output = Command::new(env!("CARGO_BIN_EXE_gameskills"))
        .current_dir(directory.path())
        .args(["verification", "resolve", "--base", "main"])
        .output()?;
    assert!(output.status.success(), "{output:?}");
    assert!(output.stderr.is_empty());
    let resolved: Value = serde_json::from_slice(&output.stdout)?;
    assert_eq!(
        resolved,
        verification::resolve(&parsed(POLICY)?, Some("main"), None)?
    );
    assert_eq!(at(&resolved, "/schema_version"), 1);
    assert_eq!(at(&resolved, "/ok"), true);
    for args in [
        vec!["resolve", "--base", "main", "--level", "development"],
        vec!["resolve", "--base"],
        vec!["resolve", "--base", "main", "--base", "dev"],
        vec!["resolve", "--level", "testing", "--level", "release"],
        vec!["resolve", "--unknown", "main"],
    ] {
        let output = Command::new(env!("CARGO_BIN_EXE_gameskills"))
            .current_dir(directory.path())
            .arg("verification")
            .args(args)
            .output()?;
        assert!(!output.status.success(), "{output:?}");
        let rejected: Value = serde_json::from_slice(&output.stdout)?;
        assert_eq!(at(&rejected, "/ok"), false);
    }
    assert_eq!(fs::read_to_string(&path)?, POLICY);
    assert_eq!(fs::read_dir(directory.path())?.count(), 1);
    Ok(())
}
