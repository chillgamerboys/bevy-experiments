//! Validate CI records, plan literal child arguments and audit required job results.

use super::{head, suites, verification, Job, Selection};
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
        && verification::level(selection).is_none()
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
    if selection.rust
        && (!selection.full || verification::level(selection).is_some())
        && selection.packages.is_empty()
    {
        return Err("selected Rust checks without packages".into());
    }
    if selection.full
        && verification::level(selection).is_some()
        && !(selection.skills && selection.rust && selection.policy)
    {
        return Err("full affected scope omitted an owner job".into());
    }
    if let Some(policy) = &selection.verification {
        verification::validate(policy)?;
    }
    if matches!(
        verification::level(selection),
        Some("development" | "testing")
    ) && [
        selection.distribution,
        selection.minimal,
        selection.wasm,
        selection.deny,
    ]
    .into_iter()
    .any(|selected| selected)
    {
        return Err("non-release verification selected release-only checks".into());
    }
    for name in &selection.suites {
        let suite = suites::get(name)?;
        if !selection.packages.iter().any(|p| p == suite.package) {
            return Err(format!("suite {name} has no selected owner"));
        }
    }
    if verification::level(selection).is_some() && selection.suites != suites::select(selection) {
        return Err("positive suites differ from selected impact".into());
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
    if selection.full && verification::level(selection).is_none() {
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

fn repository_tests(selection: &Selection) -> Result<Vec<&'static str>, String> {
    let mut targets = std::collections::BTreeSet::new();
    for path in &selection.paths {
        if let Some(target) = path
            .strip_prefix("devtools/tests/")
            .and_then(|p| p.strip_suffix(".rs"))
            .filter(|p| !p.contains('/'))
        {
            if !matches!(target, "ci_routing" | "ci_checks" | "ci_cli") {
                let known = [
                    "inputs",
                    "cli",
                    "bundle",
                    "catalog",
                    "contracts",
                    "legacy",
                    "repository",
                    "distribution",
                    "distribution_archives",
                ];
                let target = known
                    .into_iter()
                    .find(|known| *known == target)
                    .ok_or_else(|| format!("classify new repository test target {target}"))?;
                targets.insert(target);
            }
        }
        for (source, tests) in [
            ("bundle", &["bundle"] as &[&str]),
            ("distribution", &["distribution", "distribution_archives"]),
            ("catalog", &["catalog"]),
            ("contracts", &["contracts"]),
            ("legacy", &["legacy"]),
            ("repository", &["repository"]),
            ("markdown", &["repository"]),
            ("support", &["inputs"]),
        ] {
            if path == &format!("devtools/src/{source}.rs")
                || path.starts_with(&format!("devtools/src/{source}/"))
                || path.starts_with(&format!("devtools/tests/{source}"))
            {
                targets.extend(tests.iter().copied());
            }
        }
        if matches!(
            path.as_str(),
            "devtools/src/main.rs" | "devtools/src/lib.rs"
        ) {
            targets.insert("cli");
        }
    }
    Ok(targets.into_iter().collect())
}

fn repository_ci_only(selection: &Selection) -> bool {
    selection.paths.iter().all(|path| {
        path.ends_with(".md")
            || path == ".github/workflows/gamekit.yml"
            || path.starts_with("devtools/src/ci/")
            || matches!(
                path.as_str(),
                "devtools/tests/ci_routing.rs"
                    | "devtools/tests/ci_checks.rs"
                    | "devtools/tests/ci_cli.rs"
            )
    })
}

fn documentation_packages(selection: &Selection) -> Vec<String> {
    let mut packages = std::collections::BTreeSet::new();
    for package in &selection.packages {
        let root = match package.as_str() {
            "bevy-gamekit" => "gamekit/facade",
            "bevy-gamekit-discovery" => "gamekit/discovery",
            "bevy-gamekit-hex" => "gamekit/hex",
            "bevy-gamekit-multiplayer" => "gamekit/multiplayer",
            "bevy-gamekit-session" => "gamekit/session",
            "bevy-gamekit-testing" => "gamekit/testing",
            "bevy-gamekit-turns" => "gamekit/turns",
            "bevy-gamekit-ui" => "gamekit/ui",
            "gameskills-cli" => "gameskills/cli",
            "gameskills-linear" => "gameskills/linear",
            "repo-devtools" => "devtools",
            "labyrinth-rules" => "games/labyrinth/rules",
            name if suites::game(name) => match name {
                "labyrinth" => "games/labyrinth",
                "deckbuilder" => "games/deckbuilder",
                "carterfight" => "games/carterfight",
                _ => "",
            },
            _ => "",
        };
        if root.is_empty() {
            continue;
        }
        let library = format!("{root}/src/lib.rs");
        let examples = format!("{root}/examples/");
        let public_library_source = root.starts_with("gamekit/")
            && selection
                .paths
                .iter()
                .any(|path| path.starts_with(&format!("{root}/src/")) && path.ends_with(".rs"));
        if public_library_source
            || selection
                .paths
                .iter()
                .any(|path| path == &library || path.starts_with(&examples))
        {
            packages.insert(package.clone());
        }
    }
    packages.into_iter().collect()
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
            if verification::level(selection).is_none() {
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
            }
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
        Job::Rust if verification::level(selection).is_some() => {
            let broad = matches!(verification::level(selection), Some("testing" | "release"));
            if broad {
                commands.push(package_command(
                    "check",
                    &packages,
                    &[
                        "--locked",
                        "--all-targets",
                        "--all-features",
                        "--profile",
                        "ci",
                    ],
                ));
            } else {
                // Positive suites compile game libraries; check their binaries as consumers.
                let game_packages: Vec<String> = selection
                    .packages
                    .iter()
                    .filter(|package| suites::game(package))
                    .flat_map(|package| ["-p".into(), package.clone()])
                    .collect();
                if !game_packages.is_empty() {
                    commands.push(package_command(
                        "check",
                        &game_packages,
                        &["--locked", "--profile", "ci"],
                    ));
                }
            }
            for package in selection.packages.iter().filter(|p| !suites::game(p)) {
                if package == "gameskills-cli" && selection.skills {
                    continue;
                }
                if selection
                    .suites
                    .iter()
                    .filter_map(|name| suites::get(name).ok())
                    .any(|suite| suite.package == package)
                {
                    continue;
                }
                if package == "repo-devtools" {
                    let targets = repository_tests(selection)?;
                    if targets.is_empty() {
                        if repository_ci_only(selection) {
                            // Classification owns all three CI regression targets.
                            continue;
                        }
                        let test_suffix = if broad {
                            &["--locked", "--all-features", "--profile", "ci"][..]
                        } else {
                            &["--locked", "--profile", "ci"][..]
                        };
                        commands.push(package_command(
                            "test",
                            &["-p".into(), package.clone()],
                            test_suffix,
                        ));
                    }
                    for target in targets {
                        commands.push(package_command(
                            "test",
                            &["-p".into(), package.clone()],
                            &["--locked", "--profile", "ci", "--test", target],
                        ));
                    }
                } else {
                    let test_suffix = if broad {
                        &["--locked", "--all-features", "--profile", "ci"][..]
                    } else {
                        &["--locked", "--profile", "ci"][..]
                    };
                    commands.push(package_command(
                        "test",
                        &["-p".into(), package.clone()],
                        test_suffix,
                    ));
                }
            }
            for name in &selection.suites {
                commands.push(vec![
                    "repo-devtools".into(),
                    "ci".into(),
                    "suite".into(),
                    name.clone(),
                ]);
            }
            let documented: Vec<String> = documentation_packages(selection)
                .into_iter()
                .filter(|package| {
                    suites::game(package)
                        || selection
                            .suites
                            .iter()
                            .filter_map(|name| suites::get(name).ok())
                            .any(|suite| suite.package == package)
                        || (package == "gameskills-cli" && selection.skills)
                        || (package == "repo-devtools" && repository_ci_only(selection))
                })
                .flat_map(|package| ["-p".into(), package])
                .collect();
            if !documented.is_empty() {
                let doc_suffix = if broad {
                    &["--locked", "--doc", "--all-features", "--profile", "ci"][..]
                } else {
                    &["--locked", "--doc", "--profile", "ci"][..]
                };
                commands.push(package_command("test", &documented, doc_suffix));
            }
            if verification::level(selection) == Some("release") && selection.distribution {
                commands.push(repository_command("distribution", "check"));
                commands.push(repository_command("distribution", "archives"));
            }
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
            if !matches!(verification::level(selection), Some("development")) {
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
            }
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
