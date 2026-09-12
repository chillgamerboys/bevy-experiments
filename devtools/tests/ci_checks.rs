//! Required-result failures and selected command scopes preserve the Python CI contracts.

use gamekit_repo_tools::ci::{checks, Job, Selection};
use serde_json::{json, Value};
use std::error::Error;
use std::path::Path;
use std::process::Command;

type TestResult = Result<(), Box<dyn Error>>;

fn full() -> Selection {
    Selection::full("a".repeat(40), Some("b".repeat(40)), "fixture".into())
}

fn selection() -> Selection {
    Selection {
        full: false,
        packages: vec!["carterfight".into()],
        skills: false,
        distribution: false,
        minimal: false,
        wasm: false,
        deny: false,
        ..full()
    }
}

fn needs() -> Value {
    json!({
        "classify": {"result": "success", "outputs": {"selection": "other output"}},
        "skills": {"result": "skipped"},
        "rust": {"result": "success"},
        "policy": {"result": "success"}
    })
}

fn argv(arguments: &[&str]) -> Vec<String> {
    arguments
        .iter()
        .map(|argument| (*argument).into())
        .collect()
}

fn git(root: &Path, arguments: &[&str]) -> Result<String, Box<dyn Error>> {
    let output = Command::new("git")
        .args(arguments)
        .current_dir(root)
        .output()?;
    if !output.status.success() {
        return Err(format!(
            "fixture Git {arguments:?} failed: {}",
            String::from_utf8_lossy(&output.stderr)
        )
        .into());
    }
    Ok(String::from_utf8(output.stdout)?.trim().to_owned())
}

fn checkout() -> Result<(tempfile::TempDir, Selection), Box<dyn Error>> {
    let root = tempfile::tempdir()?;
    git(root.path(), &["init", "-q"])?;
    git(root.path(), &["config", "user.name", "CI test"])?;
    git(root.path(), &["config", "user.email", "ci@example.invalid"])?;
    git(root.path(), &["config", "commit.gpgsign", "false"])?;
    git(root.path(), &["commit", "--allow-empty", "-qm", "fixture"])?;
    let mut value = selection();
    value.head = git(root.path(), &["rev-parse", "HEAD"])?;
    Ok((root, value))
}

#[test]
fn intentional_skip_passes() -> TestResult {
    checks::gate(&selection(), &needs())?;
    let mut results = needs();
    *results
        .pointer_mut("/skills/result")
        .ok_or("skills result")? = json!("success");
    checks::gate(&selection(), &results)?;
    Ok(())
}

#[test]
fn selected_failure_cancellation_missing_and_skip_fail() -> TestResult {
    for job in ["classify", "rust", "policy"] {
        for status in [
            json!("failure"),
            json!("cancelled"),
            json!("skipped"),
            Value::Null,
        ] {
            let mut results = needs();
            *results.get_mut(job).ok_or("job result")? = json!({"result": status});
            let error =
                checks::gate(&selection(), &results).expect_err("selected job must succeed");
            assert!(error.contains(job), "{job}: {error}");
        }
        let mut results = needs();
        results.as_object_mut().ok_or("needs object")?.remove(job);
        assert!(
            checks::gate(&selection(), &results).is_err(),
            "missing {job}"
        );
    }
    let mut results = needs();
    *results
        .pointer_mut("/skills/result")
        .ok_or("skills result")? = json!("skipped");
    assert!(checks::gate(&full(), &results).is_err());
    Ok(())
}

#[test]
fn malformed_selection_fails() -> TestResult {
    for value in [
        json!({}),
        json!({"schema_version": 1}),
        json!({"schema_version": true}),
    ] {
        assert!(serde_json::from_value::<Selection>(value).is_err());
    }
    for (field, replacement) in [
        ("rust", json!("false")),
        ("full", json!(1)),
        ("skills", Value::Null),
        ("schema_version", json!(true)),
        ("packages", json!([1])),
        ("paths", json!([false])),
        ("reasons", json!("fixture")),
    ] {
        let mut value = serde_json::to_value(selection())?;
        *value.get_mut(field).ok_or("selection field")? = replacement;
        assert!(
            serde_json::from_value::<Selection>(value).is_err(),
            "{field}"
        );
    }
    let mut value = selection();
    value.packages = vec!["--help".into()];
    assert!(checks::gate(&value, &needs()).is_err());
    Ok(())
}

#[test]
fn unselected_failure_not_hidden() -> TestResult {
    for status in [
        json!("failure"),
        json!("cancelled"),
        json!("pending"),
        json!(true),
        json!({"result": "skipped"}),
        Value::Null,
    ] {
        let mut results = needs();
        *results
            .pointer_mut("/skills/result")
            .ok_or("skills result")? = status;
        assert!(checks::gate(&selection(), &results).is_err());
    }
    let mut results = needs();
    results
        .as_object_mut()
        .ok_or("needs object")?
        .remove("skills");
    assert!(checks::gate(&selection(), &results).is_err());
    Ok(())
}

#[test]
fn malformed_needs_objects_fail_without_panicking() -> TestResult {
    for results in [
        Value::Null,
        json!([]),
        json!("success"),
        json!(42),
        json!({}),
    ] {
        assert!(checks::gate(&selection(), &results).is_err());
    }
    for result in [
        json!([]),
        json!("success"),
        json!({}),
        json!({"result": ["success"]}),
    ] {
        let mut results = needs();
        *results.get_mut("classify").ok_or("classifier")? = result;
        assert!(checks::gate(&selection(), &results).is_err());
    }
    Ok(())
}

#[test]
fn docs_only_gate_still_requires_successful_classification() -> TestResult {
    let mut value = selection();
    value.packages.clear();
    value.rust = false;
    value.policy = false;
    let results = json!({
        "classify": {"result": "success"},
        "skills": {"result": "skipped"},
        "rust": {"result": "skipped"},
        "policy": {"result": "skipped"}
    });
    checks::gate(&value, &results)?;
    assert!(checks::gate(&value, &json!({"classify": {"result": "failure"}})).is_err());
    Ok(())
}

#[test]
fn semantic_validation_rejects_unsupported_schemas_heads_and_package_arguments() -> TestResult {
    for version in [0, 2, u32::MAX] {
        let mut value = selection();
        value.schema_version = version;
        assert!(checks::validate(&value).is_err());
    }
    for head in [
        String::new(),
        "main".into(),
        "a".repeat(39),
        "a".repeat(41),
        "G".repeat(40),
        "A".repeat(40),
        "\u{0661}".repeat(20),
    ] {
        let mut value = selection();
        value.head = head;
        assert!(checks::validate(&value).is_err());
    }
    for name in [
        "", "--help", "-p", "a b", "a/b", "a\\b", "a:b", "a\n", "café", "$(bad)",
    ] {
        let mut value = selection();
        value.packages = vec![name.into()];
        assert!(checks::validate(&value).is_err(), "{name:?}");
    }
    let mut value = selection();
    value.head = "0123456789abcdef".repeat(4);
    value.packages = vec!["9legacy".into(), "_package".into(), "bevy-gamekit".into()];
    value.base = Some("invalid requested base retained by fallback".into());
    checks::validate(&value)?;
    Ok(())
}

#[test]
fn full_selection_requires_every_flag_and_selective_rust_requires_packages() -> TestResult {
    for field in [
        "skills",
        "rust",
        "policy",
        "distribution",
        "minimal",
        "wasm",
        "deny",
    ] {
        let mut value = serde_json::to_value(full())?;
        *value.get_mut(field).ok_or("selection flag")? = json!(false);
        assert!(
            checks::validate(&serde_json::from_value(value)?).is_err(),
            "{field}"
        );
    }
    let mut value = selection();
    value.packages.clear();
    assert!(checks::validate(&value).is_err());
    checks::validate(&full())?;
    Ok(())
}

#[test]
fn game_commands_exclude_distribution_and_other_games() -> TestResult {
    let commands = checks::commands(&selection(), Job::Rust)?;
    assert_eq!(
        commands,
        vec![
            argv(&[
                "cargo",
                "test",
                "-p",
                "carterfight",
                "--all-features",
                "--profile",
                "ci"
            ]),
            argv(&[
                "cargo",
                "test",
                "-p",
                "carterfight",
                "--doc",
                "--all-features",
                "--profile",
                "ci"
            ]),
        ]
    );
    Ok(())
}

#[test]
fn labyrinth_keeps_real_process_check() -> TestResult {
    let mut value = selection();
    value.packages = vec!["labyrinth".into()];
    let commands = checks::commands(&value, Job::Rust)?;
    assert_eq!(commands.len(), 3);
    assert_eq!(
        commands.get(1),
        Some(&argv(&[
            "cargo",
            "test",
            "-p",
            "labyrinth",
            "--lib",
            "network::tests::process::six_native_processes_survive_guest_kill_and_finish_the_fight",
            "--profile",
            "ci",
            "--",
            "--ignored",
            "--exact",
            "--nocapture",
        ]))
    );
    Ok(())
}

#[test]
fn skills_and_distribution_call_rust_validators() -> TestResult {
    let value = full();
    let skills = checks::commands(&value, Job::Skills)?;
    let rust = checks::commands(&value, Job::Rust)?;
    assert_eq!(
        skills.first(),
        Some(&argv(&[
            "cargo",
            "test",
            "--locked",
            "-p",
            "gamekit-repo-tools",
            "--profile",
            "ci",
            "--test",
            "ci_routing",
            "--test",
            "ci_checks",
            "--test",
            "ci_cli",
        ]))
    );
    for (commands, suffix) in [
        (&skills, ["skills", "legacy"]),
        (&skills, ["skills", "validate"]),
        (&skills, ["bundle", "check"]),
        (&rust, ["bundle", "check"]),
        (&rust, ["distribution", "check"]),
        (&rust, ["distribution", "archives"]),
    ] {
        let mut expected = argv(&[
            "cargo",
            "run",
            "--locked",
            "-p",
            "gamekit-repo-tools",
            "--profile",
            "ci",
            "--",
        ]);
        expected.extend(argv(&suffix));
        assert!(commands.contains(&expected));
    }
    assert_eq!(
        skills.last(),
        Some(&argv(&[
            "cargo",
            "test",
            "--locked",
            "-p",
            "gameskills-cli",
            "--profile",
            "ci",
        ]))
    );
    assert!(!skills
        .iter()
        .chain(&rust)
        .flatten()
        .any(|argument| argument.ends_with(".py")));
    assert_eq!(rust.len(), 6);
    assert!(rust
        .get(3)
        .is_some_and(|command| command.contains(&"--workspace".into())));
    assert!(rust
        .last()
        .is_some_and(|command| command.contains(&"--doc".into())));
    Ok(())
}

#[test]
fn selected_policy_avoids_shared_checks_and_preserves_package_arguments() -> TestResult {
    let mut value = selection();
    value.packages.push("deckbuilder_ui".into());
    let commands = checks::commands(&value, Job::Policy)?;
    assert_eq!(
        commands,
        vec![
            argv(&["cargo", "fmt", "--all", "--", "--check"]),
            argv(&[
                "rustfmt",
                "--check",
                "--edition",
                "2021",
                "devtools/tests/fixtures/distribution/gamekit_consumer.rs"
            ]),
            argv(&[
                "cargo",
                "clippy",
                "-p",
                "carterfight",
                "-p",
                "deckbuilder_ui",
                "--all-targets",
                "--all-features",
                "--profile",
                "ci",
                "--",
                "-D",
                "warnings"
            ]),
        ]
    );
    Ok(())
}

#[test]
fn full_policy_keeps_deny_minimal_and_sorted_wasm_checks() -> TestResult {
    let commands = checks::commands(&full(), Job::Policy)?;
    assert_eq!(commands.len(), 11);
    assert_eq!(
        commands.get(3),
        Some(&argv(&["cargo", "install", "cargo-deny", "--locked"]))
    );
    assert_eq!(commands.get(4), Some(&argv(&["cargo", "deny", "check"])));
    assert_eq!(
        commands.get(5),
        Some(&argv(&[
            "cargo",
            "check",
            "-p",
            "bevy_game_discovery",
            "--no-default-features"
        ]))
    );
    assert_eq!(
        commands.get(6),
        Some(&argv(&[
            "cargo",
            "check",
            "-p",
            "bevy_game_multiplayer",
            "--no-default-features"
        ]))
    );
    assert_eq!(
        commands.get(7),
        Some(&argv(&[
            "cargo",
            "test",
            "-p",
            "bevy_game_test",
            "--no-default-features",
            "--profile",
            "ci"
        ]))
    );
    assert_eq!(
        commands.get(8),
        Some(&argv(&[
            "cargo",
            "test",
            "-p",
            "bevy_game_ui",
            "--profile",
            "ci"
        ]))
    );
    assert_eq!(
        commands.get(9),
        Some(&argv(&[
            "rustup",
            "target",
            "add",
            "wasm32-unknown-unknown"
        ]))
    );
    assert_eq!(
        commands.last(),
        Some(&argv(&[
            "cargo",
            "check",
            "-p",
            "bevy_game_hex",
            "-p",
            "bevy_game_session",
            "-p",
            "bevy_game_turns",
            "-p",
            "bevy_game_ui",
            "-p",
            "labyrinth_rules",
            "--target",
            "wasm32-unknown-unknown",
        ]))
    );
    Ok(())
}

#[test]
fn optional_policy_flags_and_wasm_intersection_are_independent() -> TestResult {
    let mut value = selection();
    value.packages = vec![
        "labyrinth_rules".into(),
        "carterfight".into(),
        "bevy_game_ui".into(),
    ];
    value.wasm = true;
    let commands = checks::commands(&value, Job::Policy)?;
    assert_eq!(commands.len(), 5);
    assert_eq!(
        commands.last(),
        Some(&argv(&[
            "cargo",
            "check",
            "-p",
            "bevy_game_ui",
            "-p",
            "labyrinth_rules",
            "--target",
            "wasm32-unknown-unknown",
        ]))
    );
    value.wasm = false;
    value.deny = true;
    assert_eq!(checks::commands(&value, Job::Policy)?.len(), 5);
    value.deny = false;
    value.minimal = true;
    assert_eq!(checks::commands(&value, Job::Policy)?.len(), 7);
    Ok(())
}

#[test]
fn other_checkout_cannot_reuse_selection() -> TestResult {
    let (root, mut value) = checkout()?;
    value.head = "c".repeat(40);
    let mut invoked = 0;
    let error = checks::run_with(root.path(), &value, Job::Rust, |_| {
        invoked += 1;
        Ok(())
    })
    .expect_err("selection identifies another checkout");
    assert!(error.contains("does not match tested checkout"), "{error}");
    assert_eq!(invoked, 0);
    Ok(())
}

#[test]
fn runner_rejects_invalid_unselected_and_unavailable_checkouts_before_children() -> TestResult {
    let (root, mut value) = checkout()?;
    let mut invoked = 0;
    let mut callback = |_: &[String]| {
        invoked += 1;
        Ok(())
    };
    assert!(checks::run_with(root.path(), &value, Job::Skills, &mut callback).is_err());
    assert!(checks::commands(&value, Job::Skills).is_err());
    value.schema_version = 2;
    assert!(checks::run_with(root.path(), &value, Job::Rust, &mut callback).is_err());
    value.schema_version = 1;
    let other = tempfile::tempdir()?;
    assert!(checks::run_with(other.path(), &value, Job::Rust, &mut callback).is_err());
    assert_eq!(invoked, 0);
    Ok(())
}

#[test]
fn runner_preserves_command_order_and_returns_completed_count() -> TestResult {
    let (root, value) = checkout()?;
    let expected = checks::commands(&value, Job::Rust)?;
    let mut invoked = Vec::new();
    let count = checks::run_with(root.path(), &value, Job::Rust, |arguments| {
        invoked.push(arguments.to_vec());
        Ok(())
    })?;
    assert_eq!(invoked, expected);
    assert_eq!(count, 2);
    Ok(())
}

#[test]
fn runner_stops_at_first_failed_child_and_reports_its_position() -> TestResult {
    let (root, mut value) = checkout()?;
    value.packages = vec!["labyrinth".into()];
    for failing_command in 1..=3 {
        let mut invoked = 0;
        let error = checks::run_with(root.path(), &value, Job::Rust, |_| {
            invoked += 1;
            if invoked == failing_command {
                Err("child exit 23".into())
            } else {
                Ok(())
            }
        })
        .expect_err("selected child failure must stop the job");
        assert_eq!(invoked, failing_command);
        assert!(
            error.contains(&format!("rust command {failing_command}")),
            "{error}"
        );
        assert!(error.contains("child exit 23"), "{error}");
    }
    Ok(())
}
