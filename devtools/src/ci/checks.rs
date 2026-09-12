//! Validate CI records, plan literal child arguments and audit required job results.

use super::{head, Job, Selection};
use serde_json::Value;
use std::path::Path;

const WASM: &[&str] = &[
    "bevy-gamekit-hex",
    "bevy-gamekit-session",
    "bevy-gamekit-turns",
    "bevy-gamekit-ui",
    "labyrinth-rules",
];
const PROCESS_TEST: &str =
    "network::tests::process::six_native_processes_survive_guest_kill_and_finish_the_fight";

/// Check the semantic invariants of a typed version-one selection record.
///
/// JSON shape and primitive types are checked when deserializing [`Selection`].
/// A conservative selection may retain an invalid requested base, so only the
/// resolved head is required to be a complete lowercase Git object ID.
pub fn validate(selection: &Selection) -> Result<(), String> {
    if selection.schema_version != 1 {
        return Err("invalid CI selection: schema_version must be 1".into());
    }
    if !matches!(selection.head.len(), 40 | 64)
        || !selection
            .head
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
    {
        return Err("invalid CI selection: head must be a full lowercase commit object ID".into());
    }
    for name in &selection.packages {
        let mut bytes = name.bytes();
        if !bytes
            .next()
            .is_some_and(|byte| byte.is_ascii_alphanumeric() || byte == b'_')
            || !bytes.all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'_' | b'-'))
        {
            return Err(format!(
                "invalid CI selection: invalid package name {name:?}"
            ));
        }
    }
    if selection.full
        && ![
            selection.skills,
            selection.rust,
            selection.policy,
            selection.distribution,
            selection.minimal,
            selection.wasm,
            selection.deny,
        ]
        .into_iter()
        .all(|selected| selected)
    {
        return Err("full selection omitted checks".into());
    }
    if selection.rust && !selection.full && selection.packages.is_empty() {
        return Err("selected Rust checks without packages".into());
    }
    Ok(())
}

/// Require successful classification and successful results for every selected job.
///
/// An unselected job may succeed or be skipped. Missing, malformed, cancelled or
/// failed results are never treated as intentional skips, even for unselected jobs.
pub fn gate(selection: &Selection, needs: &Value) -> Result<(), String> {
    validate(selection)?;
    for (name, selected) in [
        ("classify", true),
        ("skills", selection.skills),
        ("rust", selection.rust),
        ("policy", selection.policy),
    ] {
        let result = needs.get(name).and_then(|job| job.get("result"));
        if result.and_then(Value::as_str) != Some("success")
            && (selected || result.and_then(Value::as_str) != Some("skipped"))
        {
            let state = if selected { "selected" } else { "unselected" };
            return Err(format!("{name}: {state} job returned {result:?}"));
        }
    }
    Ok(())
}

fn argv(arguments: &[&str]) -> Vec<String> {
    arguments
        .iter()
        .map(|argument| (*argument).into())
        .collect()
}

fn cargo_packages(selection: &Selection) -> Vec<String> {
    if selection.full {
        argv(&["--workspace"])
    } else {
        selection
            .packages
            .iter()
            .flat_map(|name| ["-p".into(), name.clone()])
            .collect()
    }
}

fn package_command(command: &str, packages: &[String], suffix: &[&str]) -> Vec<String> {
    let mut arguments = argv(&["cargo", command]);
    arguments.extend_from_slice(packages);
    arguments.extend(argv(suffix));
    arguments
}

fn repository_command(command: &str, action: &str) -> Vec<String> {
    argv(&[
        "cargo",
        "run",
        "--locked",
        "-p",
        "repo-devtools",
        "--profile",
        "ci",
        "--",
        command,
        action,
    ])
}

/// Build the ordered literal argument vectors for one selected CI job.
///
/// Paths containing spaces remain one argument; no shell expansion is performed.
/// Planning does not launch commands or check the current checkout identity.
pub fn commands(selection: &Selection, job: Job) -> Result<Vec<Vec<String>>, String> {
    validate(selection)?;
    if !selection.selected(job) {
        return Err(format!(
            "attempted to execute unselected job {}",
            job.name()
        ));
    }
    let packages = cargo_packages(selection);
    let mut commands = Vec::new();
    match job {
        Job::Skills => {
            commands.push(argv(&[
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
            ]));
            commands.push(repository_command("skills", "legacy"));
            commands.push(repository_command("skills", "validate"));
            commands.push(repository_command("bundle", "check"));
            commands.push(argv(&[
                "cargo",
                "test",
                "--locked",
                "-p",
                "gameskills-cli",
                "--profile",
                "ci",
            ]));
        }
        Job::Rust => {
            if selection.full
                || selection
                    .packages
                    .iter()
                    .any(|name| matches!(name.as_str(), "gameskills-cli" | "repo-devtools"))
            {
                commands.push(repository_command("bundle", "check"));
            }
            if selection.distribution {
                commands.push(repository_command("distribution", "check"));
                commands.push(repository_command("distribution", "archives"));
            }
            commands.push(package_command(
                "test",
                &packages,
                &["--all-features", "--profile", "ci"],
            ));
            if selection.full || selection.packages.iter().any(|name| name == "labyrinth") {
                commands.push(argv(&[
                    "cargo",
                    "test",
                    "-p",
                    "labyrinth",
                    "--lib",
                    PROCESS_TEST,
                    "--profile",
                    "ci",
                    "--",
                    "--ignored",
                    "--exact",
                    "--nocapture",
                ]));
            }
            commands.push(package_command(
                "test",
                &packages,
                &["--doc", "--all-features", "--profile", "ci"],
            ));
        }
        Job::Policy => {
            commands.push(argv(&["cargo", "fmt", "--all", "--", "--check"]));
            commands.push(argv(&[
                "rustfmt",
                "--check",
                "--edition",
                "2021",
                "devtools/tests/fixtures/distribution/gamekit_consumer.rs",
            ]));
            commands.push(package_command(
                "clippy",
                &packages,
                &[
                    "--all-targets",
                    "--all-features",
                    "--profile",
                    "ci",
                    "--",
                    "-D",
                    "warnings",
                ],
            ));
            if selection.deny {
                commands.push(argv(&["cargo", "install", "cargo-deny", "--locked"]));
                commands.push(argv(&["cargo", "deny", "check"]));
            }
            if selection.minimal {
                for name in ["bevy-gamekit-discovery", "bevy-gamekit-multiplayer"] {
                    commands.push(argv(&[
                        "cargo",
                        "check",
                        "-p",
                        name,
                        "--no-default-features",
                    ]));
                }
                commands.push(argv(&[
                    "cargo",
                    "test",
                    "-p",
                    "bevy-gamekit-testing",
                    "--no-default-features",
                    "--profile",
                    "ci",
                ]));
                commands.push(argv(&[
                    "cargo",
                    "test",
                    "-p",
                    "bevy-gamekit-ui",
                    "--profile",
                    "ci",
                ]));
            }
            if selection.wasm {
                let wasm_packages: Vec<String> = WASM
                    .iter()
                    .filter(|name| {
                        selection.full || selection.packages.iter().any(|package| package == *name)
                    })
                    .flat_map(|name| ["-p".into(), (*name).into()])
                    .collect();
                commands.push(argv(&["rustup", "target", "add", "wasm32-unknown-unknown"]));
                commands.push(package_command(
                    "check",
                    &wasm_packages,
                    &["--target", "wasm32-unknown-unknown"],
                ));
            }
        }
    }
    Ok(commands)
}

/// Execute a selected plan through a caller-provided child command runner.
///
/// The selection must identify this checkout's current commit before any child
/// is invoked. Execution stops on the first error; success returns the number
/// of completed commands. The caller owns process I/O and working directory.
pub fn run_with(
    root: &Path,
    selection: &Selection,
    job: Job,
    mut callback: impl FnMut(&[String]) -> Result<(), String>,
) -> Result<usize, String> {
    validate(selection)?;
    if head(root)? != selection.head {
        return Err("selection does not match tested checkout".into());
    }
    let commands = commands(selection, job)?;
    for (index, arguments) in commands.iter().enumerate() {
        callback(arguments).map_err(|error| {
            format!(
                "{} command {} {arguments:?} failed: {error}",
                job.name(),
                index + 1
            )
        })?;
    }
    Ok(commands.len())
}
