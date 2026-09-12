//! Actual helper subprocesses exercise stable creation and optional tracker observations.
#![cfg(unix)]
use serde_json::{json, Value};
use std::{error::Error, os::unix::fs::PermissionsExt, path::Path, process::Command};
const ID: &str = "44444444-4444-4444-8444-444444444444";
const PROJECT: &str = "33333333-3333-4333-8333-333333333333";
fn write(root: &Path, name: &str, text: &str) -> Result<(), Box<dyn Error>> {
    std::fs::write(root.join(name), text)?;
    std::fs::set_permissions(root.join(name), std::fs::Permissions::from_mode(0o700))?;
    Ok(())
}
fn fixture() -> Result<tempfile::TempDir, Box<dyn Error>> {
    let d = tempfile::tempdir()?;
    let r = d.path();
    write(
        r,
        "gameskills-linear.toml",
        r#"workspace="11111111-1111-4111-8111-111111111111"
team="22222222-2222-4222-8222-222222222222"
project="33333333-3333-4333-8333-333333333333"
"#,
    )?;
    write(r, "description.md", "The planned change")?;
    write(
        r,
        "curl",
        r#"#!/bin/sh
input=$(cat)
issue='{"id":"44444444-4444-4444-8444-444444444444","identifier":"HEX-1","url":"https://linear.app/test/issue/HEX-1","title":"Work","project":{"id":"33333333-3333-4333-8333-333333333333"},"team":{"id":"22222222-2222-4222-8222-222222222222"},"state":{"type":"started"}}'
case "$input" in
*organization*) echo '{"data":{"organization":{"id":"11111111-1111-4111-8111-111111111111"},"team":{"id":"22222222-2222-4222-8222-222222222222"},"project":{"id":"33333333-3333-4333-8333-333333333333"}}}' ;;
*issueCreate*) echo created >> "$FIXTURE_ROOT/creates"; touch "$FIXTURE_ROOT/exists"; exit 1 ;;
*'issues(first:'*)
 if test -f "$FIXTURE_ROOT/exists"; then nodes="[$issue]"; else nodes='[]'; fi
 echo "{\"data\":{\"issues\":{\"nodes\":$nodes,\"pageInfo\":{\"hasNextPage\":false}}}}" ;;
*'attachments(first:'*) echo '{"data":{"issue":{"attachments":{"nodes":[{"id":"attachment","url":"https://github.com/test/game/pull/1"}],"pageInfo":{"hasNextPage":false}}}}}' ;;
*'issue(id:'*) echo "{\"data\":{\"issue\":$issue}}" ;;
*) exit 2 ;;
esac
"#,
    )?;
    write(
        r,
        "gh",
        r#"#!/bin/sh
echo '{"url":"https://github.com/test/game/pull/1","body":"Tracking: https://linear.app/test/issue/HEX-1","state":"OPEN","mergeCommit":null}'
"#,
    )?;
    Ok(d)
}
fn run(root: &Path, args: &[&str]) -> Result<Value, Box<dyn Error>> {
    let r = Command::new(env!("CARGO_BIN_EXE_gameskills-linear"))
        .args(args)
        .current_dir(root)
        .env(
            "PATH",
            format!("{}:{}", root.display(), std::env::var("PATH")?),
        )
        .env("LINEAR_API_KEY", "fixture-key-never-log")
        .env("FIXTURE_ROOT", root)
        .output()?;
    assert!(!String::from_utf8_lossy(&r.stdout).contains("fixture-key-never-log"));
    assert!(!String::from_utf8_lossy(&r.stderr).contains("fixture-key-never-log"));
    Ok(serde_json::from_slice(&r.stdout)?)
}
#[test]
fn ambiguous_creation_reconciles_stable_id_before_retry() -> Result<(), Box<dyn Error>> {
    let d = fixture()?;
    let args = [
        "create",
        "--id",
        ID,
        "--title",
        "Work",
        "--description-file",
        "description.md",
    ];
    assert_eq!(run(d.path(), &args)?.get("ok"), Some(&json!(false)));
    assert_eq!(run(d.path(), &args)?.get("id"), Some(&json!(ID)));
    assert_eq!(
        std::fs::read_to_string(d.path().join("creates"))?,
        "created\n"
    );
    Ok(())
}
#[test]
fn observer_checks_both_links_and_open_pr_cannot_complete() -> Result<(), Box<dyn Error>> {
    let d = fixture()?;
    let args = [
        "observe",
        "--issue",
        ID,
        "--project",
        PROJECT,
        "--pr",
        "https://github.com/test/game/pull/1",
    ];
    let observed = run(d.path(), &args)?;
    assert_eq!(observed.get("ok"), Some(&json!(true)), "{observed}");
    let result = run(
        d.path(),
        &[
            "complete",
            "--issue",
            ID,
            "--project",
            PROJECT,
            "--state",
            ID,
            "--pr",
            "https://github.com/test/game/pull/1",
        ],
    )?;
    assert_eq!(result.get("ok"), Some(&json!(false)));
    write(
        d.path(),
        "gh",
        r#"#!/bin/sh
echo '{"url":"https://github.com/test/game/pull/1","body":"No tracking link","state":"OPEN"}'
"#,
    )?;
    assert_eq!(run(d.path(), &args)?.get("ok"), Some(&json!(false)));
    Ok(())
}
