//! Executable GitHub protocol and failure boundaries, without a Python interpreter.

use gamekit_repo_tools::ci::{driver::parse_selection, Selection};
use serde_json::{json, Value};
use std::path::Path;
use std::process::{Command, Output};

struct Repo {
    directory: tempfile::TempDir,
    base: String,
    head: String,
}

fn git(root: &Path, args: &[&str]) -> String {
    let output = Command::new("git")
        .args(args)
        .current_dir(root)
        .env("GIT_CONFIG_NOSYSTEM", "1")
        .env(
            "GIT_CONFIG_GLOBAL",
            if cfg!(windows) { "NUL" } else { "/dev/null" },
        )
        .output()
        .expect("run Git");
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    String::from_utf8(output.stdout)
        .expect("UTF8 Git")
        .trim()
        .into()
}

impl Repo {
    fn new() -> Self {
        let directory = tempfile::tempdir().expect("temporary fixture");
        let root = directory.path();
        git(root, &["init", "--quiet"]);
        git(root, &["config", "user.name", "CI fixture"]);
        git(root, &["config", "user.email", "ci@example.invalid"]);
        std::fs::create_dir_all(root.join("gamekit/core/src")).expect("source directory");
        for (path, source) in [
            (
                "Cargo.toml",
                "[workspace]\nmembers = ['gamekit/*']\n[profile.ci]\ninherits = 'dev'\n",
            ),
            (
                "gamekit/core/Cargo.toml",
                "[package]\nname = 'core'\nversion = '0.1.0'\n",
            ),
            ("gamekit/core/src/lib.rs", "pub fn example() {}\n"),
            ("README.md", "Initial documentation\n"),
        ] {
            std::fs::write(root.join(path), source).expect("fixture source");
        }
        git(root, &["add", "."]);
        git(root, &["commit", "--quiet", "-m", "fixture"]);
        let base = git(root, &["rev-parse", "HEAD"]);
        std::fs::write(root.join("README.md"), "Revised documentation\n").expect("change docs");
        git(root, &["commit", "--quiet", "-am", "docs"]);
        let head = git(root, &["rev-parse", "HEAD"]);
        Self {
            directory,
            base,
            head,
        }
    }
    fn command(&self, args: &[&str]) -> Command {
        let mut command = Command::new(env!("CARGO_BIN_EXE_gamekit-repo"));
        command
            .arg("--root")
            .arg(self.directory.path())
            .args(args)
            .current_dir(std::env::temp_dir());
        for name in [
            "GITHUB_EVENT_NAME",
            "GITHUB_EVENT_PATH",
            "GITHUB_OUTPUT",
            "GITHUB_STEP_SUMMARY",
            "CI_SELECTION",
            "CI_NEEDS",
        ] {
            command.env_remove(name);
        }
        command
    }
    fn selection(&self) -> Selection {
        parse_selection(
            &String::from_utf8(
                self.command(&["ci", "select", "--base", &self.base])
                    .output()
                    .expect("select")
                    .stdout,
            )
            .expect("UTF8 selection"),
        )
        .expect("valid selection")
    }
}

fn result(output: Output, success: bool) -> Value {
    assert_eq!(
        output.status.success(),
        success,
        "stdout={} stderr={}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    let value: Value = serde_json::from_slice(&output.stdout).expect("one JSON result");
    assert_eq!(value.get("schema_version"), Some(&json!(1)));
    value
}

#[test]
fn github_outputs_round_trip_and_intentional_skips_pass_final_gate() {
    let repo = Repo::new();
    let files = tempfile::tempdir().expect("protocol files");
    let event = files.path().join("event.json");
    let output = files.path().join("output");
    let summary = files.path().join("summary");
    std::fs::write(
        &event,
        json!({"pull_request":{"base":{"sha":repo.base}}}).to_string(),
    )
    .expect("event");
    std::fs::write(&output, "earlier=preserved\n").expect("previous output");
    let value = result(
        repo.command(&["ci", "select"])
            .env("GITHUB_EVENT_PATH", &event)
            .env("GITHUB_OUTPUT", &output)
            .env("GITHUB_STEP_SUMMARY", &summary)
            .output()
            .expect("select"),
        true,
    );
    assert_eq!(value.get("head"), Some(&json!(repo.head)));
    assert_eq!(value.get("full"), Some(&json!(false)));
    for flag in ["skills", "rust", "policy"] {
        assert_eq!(value.get(flag), Some(&json!(false)));
    }
    let text = std::fs::read_to_string(output).expect("protocol output");
    assert!(text.starts_with("earlier=preserved\n"));
    for line in ["skills=false", "rust=false", "policy=false"] {
        assert!(text.lines().any(|actual| actual == line));
    }
    let encoded = text
        .lines()
        .find_map(|line| line.strip_prefix("selection="))
        .expect("selection output");
    assert_eq!(
        parse_selection(encoded).expect("roundtrip"),
        serde_json::from_value(value).expect("stdout selection")
    );
    assert!(std::fs::read_to_string(summary)
        .expect("summary")
        .contains("### Selected CI"));
    let needs = json!({"classify":{"result":"success","outputs":{}},"skills":{"result":"skipped"},"rust":{"result":"skipped"},"policy":{"result":"skipped"}});
    result(
        repo.command(&["ci", "gate"])
            .env("CI_SELECTION", encoded)
            .env("CI_NEEDS", needs.to_string())
            .output()
            .expect("gate"),
        true,
    );
}

#[test]
fn push_event_explicit_overrides_and_manual_full_requests() {
    let repo = Repo::new();
    let files = tempfile::tempdir().expect("protocol files");
    let event = files.path().join("event.json");
    std::fs::write(&event, json!({"before":repo.base}).to_string()).expect("event");
    let value = result(
        repo.command(&["ci", "select"])
            .env("GITHUB_EVENT_PATH", &event)
            .output()
            .expect("push select"),
        true,
    );
    assert_eq!(value.get("base"), Some(&json!(repo.base)));
    let value = result(
        repo.command(&["ci", "select", "--base", &repo.head, "--head", &repo.head])
            .env("GITHUB_EVENT_PATH", &event)
            .output()
            .expect("override select"),
        true,
    );
    assert_eq!(value.get("paths"), Some(&json!([])));
    for args in [vec!["ci", "select", "--full"], vec!["ci", "select"]] {
        let value = result(
            repo.command(&args)
                .env("GITHUB_EVENT_NAME", "workflow_dispatch")
                .output()
                .expect("manual full"),
            true,
        );
        assert_eq!(value.get("full"), Some(&json!(true)));
        assert_eq!(value.get("deny"), Some(&json!(true)));
    }
}

#[test]
fn invalid_head_fails_and_missing_comparison_conservatively_selects_full() {
    let repo = Repo::new();
    result(
        repo.command(&["ci", "select", "--head", "HEAD"])
            .output()
            .expect("invalid head"),
        false,
    );
    let value = result(
        repo.command(&["ci", "select"])
            .output()
            .expect("missing base"),
        true,
    );
    assert_eq!(value.get("full"), Some(&json!(true)));
}

#[test]
fn malformed_event_and_unwritable_output_cannot_publish_success() {
    let repo = Repo::new();
    let files = tempfile::tempdir().expect("protocol files");
    let event = files.path().join("event.json");
    for source in [
        "{",
        "[]",
        "{\"before\":1}",
        "{\"pull_request\":null}",
        "{\"before\":\"x\",\"before\":\"y\"}",
    ] {
        std::fs::write(&event, source).expect("event");
        result(
            repo.command(&["ci", "select"])
                .env("GITHUB_EVENT_PATH", &event)
                .output()
                .expect("bad event"),
            false,
        );
    }
    result(
        repo.command(&["ci", "select", "--full"])
            .env("GITHUB_OUTPUT", files.path())
            .output()
            .expect("unwritable output"),
        false,
    );
    result(
        repo.command(&["ci", "select", "--full"])
            .env("GITHUB_STEP_SUMMARY", files.path())
            .output()
            .expect("unwritable summary"),
        false,
    );
}

#[test]
fn final_gate_rejects_missing_malformed_or_duplicate_inputs() {
    let repo = Repo::new();
    let selection = serde_json::to_string(&repo.selection()).expect("encode selection");
    for source in [
        "",
        "{",
        "{}",
        "null",
        "{\"schema_version\":1,\"schema_version\":1}",
    ] {
        result(
            repo.command(&["ci", "gate"])
                .env("CI_SELECTION", source)
                .env("CI_NEEDS", "{}")
                .output()
                .expect("invalid selection"),
            false,
        );
    }
    for needs in [
        "",
        "{",
        "[]",
        "{\"classify\":{\"result\":\"success\",\"result\":\"failure\"}}",
    ] {
        result(
            repo.command(&["ci", "gate"])
                .env("CI_SELECTION", &selection)
                .env("CI_NEEDS", needs)
                .output()
                .expect("invalid needs"),
            false,
        );
    }
    let mut value: Value = serde_json::from_str(&selection).expect("selection");
    value
        .as_object_mut()
        .expect("object")
        .insert("schema_version".into(), json!(true));
    assert!(parse_selection(&value.to_string()).is_err());
    value
        .as_object_mut()
        .expect("object")
        .insert("schema_version".into(), json!(1));
    value
        .as_object_mut()
        .expect("object")
        .insert("extra".into(), json!(true));
    assert!(parse_selection(&value.to_string()).is_err());
}

#[test]
fn classifier_failure_and_every_non_success_selected_result_fail_final_gate() {
    let repo = Repo::new();
    let selection = serde_json::to_string(&Selection::full(
        repo.head.clone(),
        Some(repo.base.clone()),
        "fixture".into(),
    ))
    .expect("encode selection");
    for job in ["classify", "skills", "rust", "policy"] {
        for state in ["failure", "cancelled", "skipped", "missing"] {
            let mut needs = json!({"classify":{"result":"success"},"skills":{"result":"success"},"rust":{"result":"success"},"policy":{"result":"success"}});
            let jobs = needs.as_object_mut().expect("needs object");
            if state == "missing" {
                jobs.remove(job);
            } else {
                jobs.insert(job.into(), json!({"result":state}));
            }
            result(
                repo.command(&["ci", "gate"])
                    .env("CI_SELECTION", &selection)
                    .env("CI_NEEDS", needs.to_string())
                    .output()
                    .expect("failed gate"),
                false,
            );
        }
    }
}

#[test]
fn run_refuses_unselected_job_wrong_checkout_and_invalid_job_before_execution() {
    let repo = Repo::new();
    let mut selection = repo.selection();
    result(
        repo.command(&["ci", "run", "rust"])
            .env(
                "CI_SELECTION",
                serde_json::to_string(&selection).expect("selection"),
            )
            .output()
            .expect("unselected run"),
        false,
    );
    selection.skills = true;
    selection.head = repo.base.clone();
    result(
        repo.command(&["ci", "run", "skills"])
            .env(
                "CI_SELECTION",
                serde_json::to_string(&selection).expect("selection"),
            )
            .output()
            .expect("wrong checkout"),
        false,
    );
    result(
        repo.command(&["ci", "run", "unknown"])
            .output()
            .expect("bad job"),
        false,
    );
    result(
        repo.command(&["ci", "run"]).output().expect("missing job"),
        false,
    );
}

#[test]
fn actual_cargo_child_failure_streams_logs_and_stops_the_job() {
    let repo = Repo::new();
    std::fs::write(repo.directory.path().join("gamekit/core/src/lib.rs"),
        "#[test] fn intentional_child_failure() { assert!(false, \"intentional CI process fixture\"); }\n")
        .expect("failing child source");
    git(
        repo.directory.path(),
        &["commit", "--quiet", "-am", "failing child"],
    );
    let head = git(repo.directory.path(), &["rev-parse", "HEAD"]);
    let selection = Selection {
        head,
        rust: true,
        distribution: false,
        packages: vec!["core".into()],
        ..repo.selection()
    };
    let mut command = repo.command(&["ci", "run", "rust"]);
    // Cargo supplies its executable path to tests. Retain Git/toolchain discovery
    // while making this integration independent of the caller's Cargo PATH entry.
    let cargo = std::env::var_os("CARGO").expect("Cargo test executable");
    let mut paths = vec![Path::new(&cargo)
        .parent()
        .expect("Cargo parent")
        .to_path_buf()];
    paths.extend(std::env::split_paths(
        &std::env::var_os("PATH").unwrap_or_default(),
    ));
    let output = command
        .env(
            "CI_SELECTION",
            serde_json::to_string(&selection).expect("selection"),
        )
        .env("PATH", std::env::join_paths(paths).expect("test PATH"))
        .env("CARGO_NET_OFFLINE", "true")
        .env("CARGO_TARGET_DIR", repo.directory.path().join("target"))
        .env_remove("RUSTFLAGS")
        .env_remove("CARGO_ENCODED_RUSTFLAGS")
        .output()
        .expect("run actual failing child");
    assert!(!output.status.success());
    let stdout = String::from_utf8(output.stdout).expect("child stdout");
    let stderr = String::from_utf8(output.stderr).expect("child stderr");
    assert!(stdout.contains("intentional_child_failure"), "{stdout}");
    assert!(stderr.contains("+ [\"cargo\", \"test\""), "{stderr}");
    assert!(
        !stderr.contains("\"--doc\""),
        "later command must not run: {stderr}"
    );
    let final_line = stdout.lines().last().expect("final JSON");
    let value: Value = serde_json::from_str(final_line).expect("final failure result");
    assert_eq!(value.get("ok"), Some(&json!(false)));
    assert!(value
        .pointer("/error/message")
        .and_then(Value::as_str)
        .is_some_and(|message| message.contains("rust command 1")));
}

#[test]
fn isolated_controller_allows_cargo_to_rebuild_the_tested_binary() {
    let repo = Repo::new();
    let root = repo.directory.path();
    // Integration tests make Cargo build the same named executable that the CI
    // controller is running. Windows rejects replacement if both use one target.
    std::fs::write(root.join("gamekit/core/src/main.rs"), "fn main() {}\n")
        .expect("fixture binary");
    std::fs::write(root.join("gamekit/core/Cargo.toml"),
        "[package]\nname = 'core'\nversion = '0.1.0'\n[[bin]]\nname = 'gamekit-repo'\npath = 'src/main.rs'\n").expect("binary manifest");
    std::fs::create_dir(root.join("gamekit/core/tests")).expect("integration test directory");
    std::fs::write(root.join("gamekit/core/tests/binary.rs"),
        "#[test] fn compiled_binary_runs() { assert!(std::process::Command::new(env!(\"CARGO_BIN_EXE_gamekit-repo\")).status().expect(\"compiled binary\").success()); }\n").expect("binary consumer");
    git(root, &["add", "."]);
    git(root, &["commit", "--quiet", "-m", "binary regression"]);
    let mut selection = repo.selection();
    selection.full = false;
    selection.packages = vec!["core".into()];
    selection.distribution = false;
    selection.rust = true;
    let controller = root
        .join("target/ci-controller/ci")
        .join(format!("gamekit-repo{}", std::env::consts::EXE_SUFFIX));
    std::fs::create_dir_all(controller.parent().expect("controller parent"))
        .expect("controller directory");
    let source = std::fs::canonicalize(env!("CARGO_BIN_EXE_gamekit-repo"))
        .expect("compiled controller path");
    let original = std::fs::read(&source).expect("controller source");
    // A parallel Unix fork can inherit fs::copy's temporarily writable file and
    // make exec fail with ETXTBSY. Alias the already-built immutable executable
    // without opening it for writing; a symlink also works across filesystems.
    #[cfg(unix)]
    {
        std::os::unix::fs::symlink(&source, &controller).expect("isolated controller alias");
        assert_eq!(
            std::fs::read_link(&controller).expect("controller is an alias"),
            source
        );
    }
    // Windows must run a real copy at the isolated path: preventing replacement
    // of a running executable in Cargo's build directory is this regression.
    #[cfg(not(unix))]
    std::fs::copy(&source, &controller).expect("isolated controller");
    let cargo = std::env::var_os("CARGO").expect("Cargo executable");
    let mut paths = vec![Path::new(&cargo)
        .parent()
        .expect("Cargo parent")
        .to_path_buf()];
    paths.extend(std::env::split_paths(
        &std::env::var_os("PATH").unwrap_or_default(),
    ));
    let mut command = Command::new(&controller);
    command
        .arg("--root")
        .arg(root)
        .args(["ci", "run", "rust"])
        .current_dir(std::env::temp_dir())
        .env(
            "CI_SELECTION",
            serde_json::to_string(&selection).expect("selection"),
        )
        .env("PATH", std::env::join_paths(paths).expect("test PATH"))
        .env("CARGO_TARGET_DIR", root.join("target"))
        .env("CARGO_NET_OFFLINE", "true")
        .env_remove("RUSTFLAGS")
        .env_remove("CARGO_ENCODED_RUSTFLAGS");
    let output = command.output().expect("run isolated controller");
    assert!(
        output.status.success(),
        "stdout={} stderr={}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8(output.stdout).expect("stdout");
    assert!(stdout.contains("compiled_binary_runs"), "{stdout}");
    let result: Value =
        serde_json::from_str(stdout.lines().last().expect("final line")).expect("final result");
    assert_eq!(result.get("commands_completed"), Some(&json!(2)));
    assert_eq!(
        std::fs::read(&controller).expect("preserved controller"),
        original
    );
    let rebuilt = root
        .join("target/ci")
        .join(format!("gamekit-repo{}", std::env::consts::EXE_SUFFIX));
    assert!(rebuilt.is_file());
    assert_ne!(
        std::fs::canonicalize(&controller).expect("preserved controller path"),
        std::fs::canonicalize(&rebuilt).expect("rebuilt binary path")
    );
    assert_ne!(
        std::fs::read(rebuilt).expect("actual Cargo-built fixture binary"),
        original
    );
}
