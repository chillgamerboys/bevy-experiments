//! Regression coverage for task endpoints and observed remote delivery.
#![cfg(unix)]
use serde_json::{json, Value};
use std::os::unix::fs::PermissionsExt;
use std::{error::Error, path::Path, process::Command};

fn git(root: &Path, args: &[&str]) -> Result<String, Box<dyn Error>> {
    let r = Command::new("git").args(args).current_dir(root).output()?;
    assert!(r.status.success(), "{}", String::from_utf8_lossy(&r.stderr));
    Ok(String::from_utf8(r.stdout)?.trim().into())
}
fn cli(root: &Path, args: &[&str]) -> Result<Value, Box<dyn Error>> {
    let r = Command::new(env!("CARGO_BIN_EXE_gameskills"))
        .arg("--root")
        .arg(root)
        .args(args)
        .env(
            "PATH",
            format!("{}:{}", root.join("bin").display(), std::env::var("PATH")?),
        )
        .env("GH_FIXTURE", root.join("pr.json"))
        .output()?;
    Ok(serde_json::from_slice(&r.stdout)?)
}
fn fixture() -> Result<tempfile::TempDir, Box<dyn Error>> {
    let d = tempfile::tempdir()?;
    let root = d.path();
    git(root, &["init", "-b", "main"])?;
    git(root, &["config", "user.name", "Fixture"])?;
    git(root, &["config", "user.email", "fixture@example.invalid"])?;
    std::fs::write(root.join(".gitignore"), ".gameskills/\nbin/\npr.json\n")?;
    assert_eq!(at(&cli(root, &["setup", "--apply"])?, "/ok"), true);
    let mut config = std::fs::read_to_string(root.join("gameskills.toml"))?;
    config.push_str("\n[project]\ndelivery_target = \"pr\"\n");
    std::fs::write(root.join("gameskills.toml"), config)?;
    git(root, &["add", "."])?;
    git(root, &["commit", "-m", "fixture"])?;
    std::fs::create_dir(root.join("bin"))?;
    std::fs::write(
        root.join("bin/gh"),
        r#"#!/bin/sh
case "$1" in
repo) test "$3" = "test/game" || exit 2; echo '{"nameWithOwner":"test/game"}' ;;
api) case "$2" in
  *compare*) echo '{"status":"identical"}' ;;
  *) echo '{"object":{"sha":"base-sha"}}' ;;
esac ;;
*) cat "$GH_FIXTURE" ;;
esac
"#,
    )?;
    std::fs::set_permissions(root.join("bin/gh"), std::fs::Permissions::from_mode(0o755))?;
    Ok(d)
}
fn remote(root: &Path) -> Result<Value, Box<dyn Error>> {
    Ok(
        json!({"url":"https://github.com/test/game/pull/1","headRefOid":git(root,&["rev-parse","HEAD"])? ,"baseRefOid":"base-sha","baseRefName":"main","state":"OPEN","reviewDecision":"","statusCheckRollup":[{"name":"ci","conclusion":"SUCCESS"}]}),
    )
}
#[test]
fn resumed_pr_task_cannot_finish_at_local_commits() -> Result<(), Box<dyn Error>> {
    let d = fixture()?;
    let root = d.path();
    let started = cli(
        root,
        &[
            "delivery",
            "start",
            "fix",
            "--goal",
            "Improve the game",
            "--repo",
            "test/game",
        ],
    )?;
    assert_eq!(at(&started, "/record/endpoint"), "pr");
    assert_eq!(at(&cli(root, &["delivery", "check", "fix"])?, "/ok"), false);
    assert_eq!(
        cli(
            root,
            &[
                "delivery",
                "start",
                "fix",
                "--goal",
                "Override",
                "--endpoint",
                "implementation"
            ]
        )?
        .get("ok")
        .expect("result"),
        false
    );
    assert_eq!(
        cli(root, &["delivery", "show", "fix"])?
            .pointer("/record/endpoint")
            .expect("record endpoint"),
        "pr"
    );
    cli(
        root,
        &[
            "delivery",
            "bind",
            "fix",
            "--pr",
            "https://github.com/test/game/pull/1",
        ],
    )?;
    let mut pr = remote(root)?;
    set(&mut pr, "/headRefOid", json!("stale"));
    std::fs::write(root.join("pr.json"), pr.to_string())?;
    assert_eq!(at(&cli(root, &["delivery", "check", "fix"])?, "/ok"), false);
    pr = remote(root)?;
    std::fs::write(root.join("pr.json"), pr.to_string())?;
    assert_eq!(at(&cli(root, &["delivery", "check", "fix"])?, "/ok"), true);
    set(&mut pr, "/statusCheckRollup/0/conclusion", json!(""));
    std::fs::write(root.join("pr.json"), pr.to_string())?;
    assert_eq!(at(&cli(root, &["delivery", "check", "fix"])?, "/ok"), false);
    Ok(())
}
#[test]
fn explicit_plan_scope_and_unavailable_provider_remain_distinct() -> Result<(), Box<dyn Error>> {
    let d = fixture()?;
    let root = d.path();
    cli(
        root,
        &[
            "delivery",
            "start",
            "design",
            "--goal",
            "Discuss",
            "--endpoint",
            "design",
        ],
    )?;
    assert_eq!(
        at(&cli(root, &["delivery", "check", "design"])?, "/ok"),
        true
    );
    cli(
        root,
        &[
            "delivery",
            "start",
            "pr",
            "--goal",
            "Ship",
            "--repo",
            "test/game",
        ],
    )?;
    cli(
        root,
        &[
            "delivery",
            "bind",
            "pr",
            "--pr",
            "https://github.com/test/game/pull/1",
        ],
    )?;
    std::fs::write(root.join("bin/gh"), "#!/bin/sh\nexit 1\n")?;
    let result = cli(root, &["delivery", "check", "pr"])?;
    assert_eq!(at(&result, "/ok"), false);
    assert!(at(&result, "/reasons").to_string().contains("unverifiable"));
    Ok(())
}
#[test]
fn merge_endpoint_rejects_open_closed_wrong_base_and_dirty_source() -> Result<(), Box<dyn Error>> {
    let d = fixture()?;
    let root = d.path();
    cli(
        root,
        &[
            "delivery",
            "start",
            "merge",
            "--goal",
            "Ship",
            "--endpoint",
            "merge",
            "--repo",
            "test/game",
        ],
    )?;
    cli(
        root,
        &[
            "delivery",
            "bind",
            "merge",
            "--pr",
            "https://github.com/test/game/pull/1",
        ],
    )?;
    for (state, base) in [("OPEN", "main"), ("CLOSED", "main"), ("MERGED", "wrong")] {
        let mut pr = remote(root)?;
        set(&mut pr, "/state", json!(state));
        set(&mut pr, "/baseRefName", json!(base));
        set(&mut pr, "/mergeCommit", json!({"oid":"merge-sha"}));
        std::fs::write(root.join("pr.json"), pr.to_string())?;
        assert_eq!(
            at(&cli(root, &["delivery", "check", "merge"])?, "/ok"),
            false
        );
    }
    std::fs::write(root.join("dirty"), "changes")?;
    assert_eq!(
        at(&cli(root, &["delivery", "check", "merge"])?, "/ok"),
        false
    );
    Ok(())
}

fn at<'a>(v: &'a Value, path: &str) -> &'a Value {
    v.pointer(path).expect("fixture pointer")
}
fn set(v: &mut Value, path: &str, value: Value) {
    if path.matches('/').count() == 1 {
        v.as_object_mut()
            .expect("fixture object")
            .insert(path.trim_start_matches('/').into(), value);
    } else {
        *v.pointer_mut(path).expect("fixture pointer") = value;
    }
}

#[test]
fn required_tracker_and_remaining_work_are_checked_without_shortening_scope(
) -> Result<(), Box<dyn Error>> {
    let d = fixture()?;
    let root = d.path();
    let mut config = std::fs::read_to_string(root.join("gameskills.toml"))?;
    config.push_str("\n[tracking]\nrequired=true\nobserver=[\"tracker\"]\n");
    std::fs::write(root.join("gameskills.toml"), config)?;
    git(root, &["add", "gameskills.toml"])?;
    git(root, &["commit", "-m", "tracking policy"])?;
    cli(
        root,
        &[
            "delivery",
            "start",
            "task",
            "--goal",
            "Ship",
            "--repo",
            "test/game",
        ],
    )?;
    cli(
        root,
        &[
            "delivery",
            "bind",
            "task",
            "--pr",
            "https://github.com/test/game/pull/1",
            "--issue",
            "issue-id",
            "--project",
            "project-id",
        ],
    )?;
    std::fs::write(root.join("pr.json"), remote(root)?.to_string())?;
    assert_eq!(
        at(&cli(root, &["delivery", "check", "task"])?, "/ok"),
        false
    );
    std::fs::write(
        root.join("bin/tracker"),
        r#"#!/bin/sh
echo '{"ok":true,"linked":true,"issue_id":"issue-id","project_id":"wrong","pr_url":"https://github.com/test/game/pull/1"}'
"#,
    )?;
    std::fs::set_permissions(
        root.join("bin/tracker"),
        std::fs::Permissions::from_mode(0o755),
    )?;
    assert_eq!(
        at(&cli(root, &["delivery", "check", "task"])?, "/ok"),
        false
    );
    let script = std::fs::read_to_string(root.join("bin/tracker"))?.replace("wrong", "project-id");
    std::fs::write(root.join("bin/tracker"), script)?;
    assert_eq!(at(&cli(root, &["delivery", "check", "task"])?, "/ok"), true);
    cli(
        root,
        &[
            "delivery",
            "note",
            "task",
            "--remaining",
            "Live pilot needs export storage",
        ],
    )?;
    assert_eq!(
        at(&cli(root, &["delivery", "check", "task"])?, "/ok"),
        false
    );
    assert_eq!(
        at(
            &cli(root, &["delivery", "show", "task"])?,
            "/record/endpoint"
        ),
        "pr"
    );
    Ok(())
}
