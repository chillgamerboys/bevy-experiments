//! Required-result failures and selected command scopes preserve the Python CI contracts.

use repo_devtools::ci::{checks, Job, Selection};
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
            "repo-devtools",
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
            "repo-devtools",
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
    value.packages.push("deckbuilder".into());
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
                "deckbuilder",
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
            "bevy-gamekit-discovery",
            "--no-default-features"
        ]))
    );
    assert_eq!(
        commands.get(6),
        Some(&argv(&[
            "cargo",
            "check",
            "-p",
            "bevy-gamekit-multiplayer",
            "--no-default-features"
        ]))
    );
    assert_eq!(
        commands.get(7),
        Some(&argv(&[
            "cargo",
            "test",
            "-p",
            "bevy-gamekit-testing",
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
            "bevy-gamekit-ui",
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
            "bevy-gamekit-hex",
            "-p",
            "bevy-gamekit-session",
            "-p",
            "bevy-gamekit-turns",
            "-p",
            "bevy-gamekit-ui",
            "-p",
            "labyrinth-rules",
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
        "labyrinth-rules".into(),
        "carterfight".into(),
        "bevy-gamekit-ui".into(),
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
            "bevy-gamekit-ui",
            "-p",
            "labyrinth-rules",
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

fn configured(level: &str, packages: &[&str], paths: &[&str]) -> Selection {
    let mut value = selection();
    value.packages = packages.iter().map(|p| (*p).into()).collect();
    value.paths = paths.iter().map(|p| (*p).into()).collect();
    value.verification = Some(json!({
        "schema_version":1,"ok":true,"configured":true,
        "receiving_branch": if level == "development" {"dev"} else {"main"},
        "branch_required_level":if level == "development" {"development"} else {"testing"},
        "level":level,
        "platforms":if level == "release" {vec!["macos","windows","linux"]} else {vec!["macos"]},
        "display":{"width":1920,"height":1080,"scale":"auto"},
        "manual_sanity":"milestone","policy_digest":"a".repeat(64),"reasons":[]
    }));
    value.suites = repo_devtools::ci::suites::select(&value);
    value
}

#[test]
fn configured_logic_runs_positive_authority_without_ui_or_sockets() -> TestResult {
    let value = configured(
        "development",
        &["labyrinth", "labyrinth-rules"],
        &["games/labyrinth/rules/src/turn.rs"],
    );
    let commands = checks::commands(&value, Job::Rust)?;
    assert_eq!(value.suites, ["labyrinth-session"]);
    assert!(commands
        .iter()
        .any(|cmd| cmd == &argv(&["repo-devtools", "ci", "suite", "labyrinth-session"])));
    assert!(
        !commands
            .iter()
            .any(|cmd| cmd.get(1).is_some_and(|v| v == "test")
                && cmd.iter().any(|v| v == "labyrinth"))
    );
    assert_eq!(
        repo_devtools::ci::verification::runners(&value),
        ["macos-latest"]
    );
    Ok(())
}

#[test]
fn development_compiles_and_tests_affected_scope_without_batch_lint_or_feature_sweeps() -> TestResult
{
    let value = configured(
        "development",
        &["bevy-gamekit-ui", "deckbuilder"],
        &["gamekit/ui/src/lib.rs"],
    );
    let rust = checks::commands(&value, Job::Rust)?;
    assert!(rust.iter().any(|command| command
        == &argv(&[
            "cargo",
            "check",
            "-p",
            "deckbuilder",
            "--locked",
            "--profile",
            "ci",
        ])));
    assert!(rust.iter().any(|command| {
        command.get(1).is_some_and(|arg| arg == "test")
            && command.contains(&"bevy-gamekit-ui".into())
            && !command.contains(&"--doc".into())
    }));
    assert!(!rust
        .iter()
        .flatten()
        .any(|argument| matches!(argument.as_str(), "--all-targets" | "--all-features")));

    let policy = checks::commands(&value, Job::Policy)?;
    assert_eq!(policy.len(), 2);
    assert!(!policy.iter().flatten().any(|argument| argument == "clippy"));
    Ok(())
}

#[test]
fn testing_retains_affected_batch_compile_lint_and_feature_coverage() -> TestResult {
    let value = configured("testing", &["bevy-gamekit-ui"], &["gamekit/ui/src/lib.rs"]);
    let rust = checks::commands(&value, Job::Rust)?;
    assert!(rust
        .first()
        .is_some_and(|command| command.contains(&"--all-targets".into())
            && command.contains(&"--all-features".into())));
    assert!(rust.iter().any(|command| {
        command.get(1).is_some_and(|arg| arg == "test")
            && command.contains(&"bevy-gamekit-ui".into())
            && command.contains(&"--all-features".into())
            && !command.contains(&"--doc".into())
    }));
    let policy = checks::commands(&value, Job::Policy)?;
    assert!(policy
        .iter()
        .any(|command| command.get(1).is_some_and(|arg| arg == "clippy")
            && command.contains(&"--all-targets".into())
            && command.contains(&"--all-features".into())));
    Ok(())
}

#[test]
fn configured_non_release_cannot_restore_artifact_or_distribution_checks() {
    for level in ["development", "testing"] {
        for flag in ["distribution", "minimal", "wasm", "deny"] {
            let mut value = serde_json::to_value(configured(level, &["bevy-gamekit-ui"], &[]))
                .expect("selection value");
            *value.get_mut(flag).expect("configured selection flag") = json!(true);
            let value = serde_json::from_value(value).expect("typed selection");
            assert!(checks::validate(&value).is_err(), "{level} {flag}");
        }
    }
}

#[test]
fn release_retains_artifact_distribution_and_cross_target_checks() -> TestResult {
    let mut value = configured("release", &["bevy-gamekit-ui"], &["gamekit/ui/src/lib.rs"]);
    value.distribution = true;
    value.minimal = true;
    value.wasm = true;
    value.deny = true;
    checks::validate(&value)?;
    let rust = checks::commands(&value, Job::Rust)?;
    for action in ["check", "archives"] {
        assert!(rust.iter().any(|command| {
            command
                .windows(2)
                .any(|pair| pair == ["distribution", action])
        }));
    }
    let policy = checks::commands(&value, Job::Policy)?;
    assert!(policy.iter().flatten().any(|argument| argument == "clippy"));
    assert!(policy
        .iter()
        .any(|command| command.contains(&"wasm32-unknown-unknown".into())));
    Ok(())
}

#[test]
fn tooltip_owner_change_runs_owner_and_relevant_consumer_regressions() -> TestResult {
    let value = configured(
        "development",
        &["bevy-gamekit-ui", "carterfight", "deckbuilder", "labyrinth"],
        &["gamekit/ui/src/tooltip/view.rs"],
    );
    assert_eq!(
        value.suites,
        [
            "deckbuilder-tooltip-consumers",
            "gamekit-ui-tooltip",
            "labyrinth-tooltip-consumers",
        ]
    );
    let commands = checks::commands(&value, Job::Rust)?;
    for suite in &value.suites {
        assert!(commands
            .iter()
            .any(|command| command == &argv(&["repo-devtools", "ci", "suite", suite])));
    }
    assert!(!commands.iter().any(|command| {
        command.get(1).is_some_and(|arg| arg == "test")
            && command.contains(&"bevy-gamekit-ui".into())
            && !command.contains(&"--doc".into())
    }));
    assert!(commands.iter().any(|command| {
        command.contains(&"bevy-gamekit-ui".into()) && command.contains(&"--doc".into())
    }));
    assert!(!value
        .suites
        .iter()
        .any(|suite| suite.starts_with("carterfight")));
    Ok(())
}

#[test]
fn admission_and_process_are_distinct_affected_journeys() -> TestResult {
    let admission = configured(
        "development",
        &["labyrinth"],
        &["games/labyrinth/src/network/admission.rs"],
    );
    assert!(admission.suites.contains(&"labyrinth-admission".into()));
    assert!(!admission.suites.contains(&"labyrinth-process".into()));
    assert!(!admission.suites.iter().any(|s| s.contains("ui")));
    let process = configured(
        "testing",
        &["labyrinth"],
        &["games/labyrinth/src/network/tests/process.rs"],
    );
    assert!(process.suites.contains(&"labyrinth-process".into()));
    checks::validate(&process)?;
    Ok(())
}

#[test]
fn ui_normal_and_release_compatibility_are_separate_positive_suites() -> TestResult {
    for level in ["development", "testing", "release"] {
        let value = configured(
            level,
            &["labyrinth"],
            &["games/labyrinth/src/ui/battle/help.rs"],
        );
        assert!(value.suites.contains(&"labyrinth-ui-normal".into()));
        assert_eq!(
            value.suites.contains(&"labyrinth-ui-compatibility".into()),
            level == "release"
        );
        assert!(!value.suites.contains(&"labyrinth-admission".into()));
        assert_eq!(
            repo_devtools::ci::verification::runners(&value).len(),
            if level == "release" { 3 } else { 1 }
        );
        checks::validate(&value)?;
    }
    Ok(())
}

#[test]
fn full_affected_scope_does_not_force_release_or_every_journey() -> TestResult {
    let mut value = configured(
        "testing",
        &["labyrinth", "deckbuilder", "carterfight"],
        &[".github/workflows/gamekit.yml"],
    );
    value.full = true;
    value.skills = true;
    value.distribution = false;
    value.minimal = false;
    value.wasm = false;
    value.deny = false;
    checks::validate(&value)?;
    assert_eq!(
        value.suites,
        [
            "carterfight-rules",
            "deckbuilder-domain",
            "labyrinth-presentation",
            "labyrinth-session"
        ]
    );
    assert_eq!(
        repo_devtools::ci::verification::runners(&value),
        ["macos-latest"]
    );
    Ok(())
}

#[test]
fn policy_and_suite_tampering_fail_before_execution() -> TestResult {
    let mut value = configured(
        "development",
        &["labyrinth"],
        &["games/labyrinth/src/session/mod.rs"],
    );
    value.suites.clear();
    assert!(checks::validate(&value).is_err());
    let mut value = configured("development", &["labyrinth"], &[]);
    value.verification.as_mut().ok_or("policy")?["branch_required_level"] = json!("testing");
    assert!(checks::validate(&value).is_err());
    assert!(repo_devtools::ci::suites::get("--help").is_err());
    Ok(())
}

#[test]
fn test_listing_and_zero_execution_cannot_establish_coverage() {
    use repo_devtools::ci::suites::{listed_tests, passed_tests};
    assert_eq!(listed_tests("0 tests, 0 benchmarks\n"), 0);
    assert_eq!(
        listed_tests("network::one: test\nnetwork::two: test\n2 tests, 0 benchmarks\n"),
        2
    );
    assert_eq!(
        passed_tests(
            "test result: ok. 0 passed; 0 failed; 2 ignored; 7 filtered out; finished in 0.0s"
        ),
        0
    );
    assert_eq!(
        passed_tests(
            "test result: ok. 2 passed; 0 failed; 0 ignored; 7 filtered out; finished in 0.1s"
        ),
        2
    );
    assert_eq!(passed_tests("network::one: test"), 0);
}

#[test]
fn explicit_full_release_covers_preserved_game_cases_and_process_recovery() -> TestResult {
    let mut value = configured("release", &["labyrinth", "deckbuilder", "carterfight"], &[]);
    value.full = true;
    value.skills = true;
    value.suites = repo_devtools::ci::suites::select(&value);
    assert_eq!(
        value.suites,
        [
            "carterfight-all",
            "deckbuilder-all",
            "labyrinth-all",
            "labyrinth-process"
        ]
    );
    checks::validate(&value)?;
    Ok(())
}

#[test]
fn configured_jobs_do_not_duplicate_classifier_or_cli_runtime_tests() -> TestResult {
    let mut value = configured(
        "testing",
        &["gameskills-cli", "repo-devtools"],
        &["devtools/src/ci/checks.rs"],
    );
    value.skills = true;
    let skills = checks::commands(&value, Job::Skills)?;
    let rust = checks::commands(&value, Job::Rust)?;
    assert!(!skills.iter().flatten().any(|arg| arg == "ci_routing"));
    assert!(!rust
        .iter()
        .any(|cmd| cmd.get(1).is_some_and(|a| a == "test")
            && cmd
                .iter()
                .any(|a| a == "gameskills-cli" || a == "repo-devtools")));
    assert!(skills
        .iter()
        .any(|cmd| cmd.get(1).is_some_and(|a| a == "test")
            && cmd.iter().any(|a| a == "gameskills-cli")));
    Ok(())
}

#[test]
fn test_body_classification_tracks_assertion_changes_and_ignores_comments() -> TestResult {
    use repo_devtools::ci::suites::test_bodies;
    let source =
        "#[cfg(test)] mod tests { #[test] fn case() { assert_eq!(1, 1); } fn helper() {} }";
    let original = test_bodies(source, "network")?;
    assert_eq!(original.len(), 1);
    assert!(original.contains_key("network::tests::case"));
    assert_eq!(
        original,
        test_bodies(
            &source.replace("assert_eq!", "/* comment */ assert_eq!"),
            "network"
        )?
    );
    assert_ne!(
        original,
        test_bodies(&source.replace("1, 1", "1, 2"), "network")?
    );
    Ok(())
}

#[test]
fn test_only_module_changes_do_not_claim_gameplay_changed() -> TestResult {
    use repo_devtools::ci::suites::production_source;
    let before =
        "pub fn rule() -> u32 { 1 } #[cfg(test)] mod tests { #[test] fn old() { assert!(true); } }";
    let after = "pub fn rule() -> u32 { 1 } #[cfg(test)] mod tests { #[test] fn new_normal_1080() { assert_eq!(2, 2); } }";
    assert_eq!(production_source(before)?, production_source(after)?);
    assert_ne!(
        production_source(before)?,
        production_source(&after.replace("{ 1 }", "{ 2 }"))?
    );
    Ok(())
}

#[test]
fn changed_repository_test_targets_execute_or_require_classification() -> TestResult {
    for target in ["inputs", "cli"] {
        let value = configured(
            "development",
            &["repo-devtools"],
            &[&format!("devtools/tests/{target}.rs")],
        );
        let commands = checks::commands(&value, Job::Rust)?;
        assert!(commands
            .iter()
            .any(|cmd| cmd.windows(2).any(|pair| pair == ["--test", target])));
    }
    let value = configured(
        "development",
        &["repo-devtools"],
        &["devtools/tests/new_contract.rs"],
    );
    assert!(checks::commands(&value, Job::Rust)
        .expect_err("unmapped target")
        .contains("new_contract"));
    let value = configured(
        "development",
        &["repo-devtools"],
        &["devtools/src/new_internal_boundary.rs"],
    );
    let commands = checks::commands(&value, Job::Rust)?;
    assert!(commands.iter().any(|command| {
        command.get(1).is_some_and(|arg| arg == "test")
            && command.contains(&"repo-devtools".into())
            && !command.contains(&"--test".into())
    }));
    Ok(())
}

#[test]
fn unknown_scope_and_compiled_markdown_keep_owner_regressions() -> TestResult {
    for paths in [
        Vec::new(),
        vec![
            "devtools/src/ci/checks.rs",
            "devtools/tests/fixtures/contract.md",
        ],
    ] {
        let value = configured("development", &["repo-devtools"], &paths);
        let commands = checks::commands(&value, Job::Rust)?;
        assert!(commands.iter().any(|command| {
            command.get(1).is_some_and(|arg| arg == "test")
                && command.contains(&"repo-devtools".into())
                && !command.contains(&"--test".into())
        }));
    }
    let value = configured(
        "development",
        &["bevy-gamekit-ui", "labyrinth"],
        &[
            "gamekit/ui/src/tooltip.rs",
            "gamekit/ui/tests/fixtures/layout.md",
        ],
    );
    assert!(!value.suites.contains(&"gamekit-ui-tooltip".into()));
    assert!(value.suites.contains(&"labyrinth-ui-normal".into()));
    Ok(())
}
