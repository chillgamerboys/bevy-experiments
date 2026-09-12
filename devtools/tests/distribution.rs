//! Distribution boundaries exercised with inert Cargo output and local sources.

use gamekit_repo_tools::distribution::{
    check_with_runner, library_manifest, package_sources, validate_graph, Case,
};
use std::collections::BTreeSet;
use std::error::Error;
use std::ffi::{OsStr, OsString};
use std::path::{Path, PathBuf};

const WORKSPACE: &str = r#"[workspace] # keep shared settings
resolver = "3"
members = ["games/*", "gamekit/*", "devtools"]
default-members = ["games/labyrinth"]
exclude = ["gamekit/unused"]
[workspace.package]
version = "0.1.0"
edition = "2021"
[workspace.dependencies]
cap = { path = "gamekit/hex", default-features = false }
[workspace.lints.rust]
unsafe_code = "forbid"
[profile.ci]
inherits = "dev"
debug = "line-tables-only"
"#;

fn names(extra: &[&str]) -> BTreeSet<String> {
    ["gamekit_external_probe", "bevy-gamekit"]
        .into_iter()
        .chain(extra.iter().copied())
        .map(str::to_owned)
        .collect()
}

fn write(root: &Path, relative: &str, contents: impl AsRef<[u8]>) -> Result<(), Box<dyn Error>> {
    let path = root.join(relative);
    std::fs::create_dir_all(path.parent().ok_or("file has no parent")?)?;
    std::fs::write(path, contents)?;
    Ok(())
}

fn fixture() -> Result<tempfile::TempDir, Box<dyn Error>> {
    let directory = tempfile::Builder::new()
        .prefix("distribution $(literal) with spaces ")
        .tempdir()?;
    let root = directory.path();
    write(root, "Cargo.toml", WORKSPACE)?;
    write(root, "Cargo.lock", "owning lock versions\n")?;
    for (directory, package) in [
        ("gamekit/facade", "bevy-gamekit"),
        ("gamekit/hex", "bevy_game_hex"),
        ("games/labyrinth", "labyrinth"),
        ("games/labyrinth/rules", "labyrinth-rules"),
        ("devtools", "gamekit-repo-tools"),
    ] {
        write(
            root,
            &format!("{directory}/Cargo.toml"),
            format!("[package]\nname = {package:?}\nversion = \"0.1.0\"\n"),
        )?;
        write(root, &format!("{directory}/src/lib.rs"), "// capability\n")?;
        write(root, &format!("{directory}/README.md"), "Selected bytes\n")?;
        write(
            root,
            &format!("{directory}/not-packaged.rs"),
            "not selected",
        )?;
    }
    Ok(directory)
}

fn strings(args: &[&str]) -> Vec<OsString> {
    args.iter().map(OsString::from).collect()
}

fn graph(case: Case) -> String {
    let extra = match case {
        Case::Pure => vec!["serde_json", "bevy_game_hex", "bevy_game_turns"],
        Case::Ui => vec!["bevy", "bevy_game_ui", "bevy_game_test"],
        Case::Network => vec![
            "bevy",
            "bevy_game_multiplayer",
            "bevy_game_discovery",
            "aeronet_webtransport",
        ],
        Case::Empty | Case::All => Vec::new(),
    };
    names(&extra)
        .into_iter()
        .map(|name| format!("{name} v0.1.0 (/path with spaces)\n"))
        .collect()
}

fn selected(args: &[OsString]) -> Case {
    match args.last().and_then(|arg| arg.to_str()) {
        Some("pure") => Case::Pure,
        Some("ui") => Case::Ui,
        Some("network") => Case::Network,
        _ => Case::Empty,
    }
}

const LISTING: &str =
    "Cargo.toml\nsrc/lib.rs\nREADME.md\nCargo.toml.orig\nCargo.lock\n.cargo_vcs_info.json\n";

#[test]
fn staging_removes_game_membership_preserving_shared_settings() -> Result<(), Box<dyn Error>> {
    let before: toml::Value = toml::from_str(WORKSPACE)?;
    let rewritten = library_manifest(WORKSPACE)?;
    let after: toml::Value = toml::from_str(&rewritten)?;
    let workspace = after.get("workspace").ok_or("no workspace")?;
    assert_eq!(
        workspace.get("members"),
        Some(&toml::Value::Array(vec!["gamekit/*".into()]))
    );
    assert!(workspace.get("default-members").is_none());
    assert!(workspace.get("exclude").is_none());
    let original = before.get("workspace").ok_or("no original workspace")?;
    for key in ["resolver", "package", "dependencies", "lints"] {
        assert_eq!(workspace.get(key), original.get(key), "{key}");
    }
    assert_eq!(after.get("profile"), before.get("profile"));
    assert!(rewritten.contains("# keep shared settings"));
    Ok(())
}

#[test]
fn alternate_workspace_toml_preserves_unrelated_values() -> Result<(), Box<dyn Error>> {
    for source in [
        "  [ workspace ] # spaced header\nresolver = '3'\nmembers=['games/*']\nexclude=['gamekit/hex']\n[workspace.package]\nversion='1.0.0'\n",
        "workspace.resolver = '3'\nworkspace.members = ['tools/*']\nworkspace.default-members = ['tools/cli']\nworkspace.dependencies.cap = { path = 'gamekit/cap' }\n",
        "workspace = {resolver='3', members=['games/*'], default-members=['games/a'], exclude=['gamekit/a'], package={version='1.0.0'}, lints={rust={unsafe_code='forbid'}}}\n",
    ] {
        let before: toml::Value = toml::from_str(source)?;
        let rewritten = library_manifest(source)?;
        let after: toml::Value = toml::from_str(&rewritten)?;
        let mut expected = before;
        let workspace = expected.get_mut("workspace").and_then(toml::Value::as_table_mut).ok_or("workspace missing")?;
        workspace.insert("members".into(), toml::Value::Array(vec!["gamekit/*".into()]));
        workspace.remove("default-members");
        workspace.remove("exclude");
        assert_eq!(after, expected);
    }
    Ok(())
}

#[test]
fn malformed_or_nonvirtual_workspace_manifests_fail() {
    for source in [
        "[workspace\nresolver = '3'",
        "[package]\nname = 'not-virtual'\n[workspace]\nresolver = '3'",
        "[dependencies]\nserde = '1'",
        "workspace = 'invalid'",
        "[workspace]\nmembers = []",
        "[workspace]\nresolver = 3",
        "[workspace]\nresolver = '3'\n[workspace.dependencies]\ncap = {path = 'gamekit/cap'",
        "[workspace]\nresolver='3'\nresolver='2'",
    ] {
        assert!(library_manifest(source).is_err(), "{source}");
    }
}

#[test]
fn package_list_requires_library_and_rejects_escapes() -> Result<(), Box<dyn Error>> {
    let directory = fixture()?;
    let crate_root = directory.path().join("gamekit/facade").canonicalize()?;
    assert_eq!(
        package_sources(&crate_root, LISTING)?,
        vec![
            PathBuf::from("Cargo.toml"),
            PathBuf::from("src/lib.rs"),
            PathBuf::from("README.md")
        ]
    );
    let absolute = crate_root.join("src/lib.rs");
    for bad in [
        "../game.rs",
        "src/../../game.rs",
        "missing.rs",
        "Cargo.toml",
        "src/lib.rs",
        "Cargo.toml\nsrc",
        "Cargo.toml\nsrc/lib.rs\nREADME.md\nREADME.md",
        "C:\\game.rs",
        "C:/game.rs",
        "src\\lib.rs",
        "\0bad",
        "",
        "\n",
        absolute.to_str().ok_or("non-UTF-8 test path")?,
    ] {
        assert!(package_sources(&crate_root, bad).is_err(), "{bad:?}");
    }
    std::fs::remove_file(crate_root.join("src/lib.rs"))?;
    assert!(
        package_sources(&crate_root, LISTING).is_err(),
        "missing library unexpectedly passed"
    );
    Ok(())
}

#[test]
fn graph_rejects_game_and_feature_leakage() -> Result<(), Box<dyn Error>> {
    let forbidden = BTreeSet::from(["labyrinth".into(), "gamekit-repo-tools".into()]);
    validate_graph(Case::Empty, &names(&[]), &forbidden)?;
    validate_graph(
        Case::Pure,
        &names(&["bevy_game_hex", "serde_json"]),
        &forbidden,
    )?;
    validate_graph(Case::Ui, &names(&["bevy", "bevy_game_ui"]), &forbidden)?;
    validate_graph(
        Case::Network,
        &names(&[
            "bevy_game_multiplayer",
            "bevy_game_discovery",
            "aeronet_webtransport",
        ]),
        &forbidden,
    )?;
    for case in [Case::Empty, Case::Pure, Case::Ui, Case::Network] {
        for extra in ["labyrinth", "gamekit-repo-tools"] {
            assert!(
                validate_graph(case, &names(&[extra]), &forbidden).is_err(),
                "{case:?}: {extra}"
            );
        }
        assert!(validate_graph(case, &BTreeSet::new(), &forbidden).is_err());
        assert!(
            validate_graph(case, &BTreeSet::from(["bevy-gamekit".into()]), &forbidden).is_err()
        );
    }
    for case in [Case::Empty, Case::Pure, Case::Ui] {
        for extra in [
            "bevy_game_multiplayer",
            "bevy_game_discovery",
            "aeronet_webtransport",
        ] {
            assert!(
                validate_graph(case, &names(&[extra]), &forbidden).is_err(),
                "{case:?}: {extra}"
            );
        }
    }
    for case in [Case::Empty, Case::Pure] {
        assert!(validate_graph(case, &names(&["bevy"]), &forbidden).is_err());
    }
    assert!(validate_graph(Case::All, &names(&[]), &forbidden).is_err());
    Ok(())
}

#[cfg(windows)]
#[test]
fn windows_cargo_listing_accepts_native_separators_but_rejects_alias_duplicates(
) -> Result<(), Box<dyn Error>> {
    let directory = fixture()?;
    let root = directory.path().join("gamekit/facade").canonicalize()?;
    assert_eq!(
        package_sources(&root, "Cargo.toml\nsrc\\lib.rs\n")?,
        vec![PathBuf::from("Cargo.toml"), PathBuf::from("src/lib.rs")]
    );
    assert!(package_sources(&root, "Cargo.toml\nsrc\\lib.rs\nsrc/lib.rs\n").is_err());
    assert!(package_sources(&root, "Cargo.toml\nsrc\\..\\..\\outside.rs\n").is_err());
    Ok(())
}

#[cfg(unix)]
#[test]
fn package_rejects_symlinked_source_and_ancestors() -> Result<(), Box<dyn Error>> {
    use std::os::unix::fs::symlink;
    let directory = fixture()?;
    let crate_root = directory.path().join("gamekit/facade").canonicalize()?;
    symlink(crate_root.join("src/lib.rs"), crate_root.join("linked.rs"))?;
    symlink(crate_root.join("src"), crate_root.join("linked-src"))?;
    symlink(
        directory.path().join("Cargo.lock"),
        crate_root.join("escape.rs"),
    )?;
    symlink(
        crate_root.join("missing.rs"),
        crate_root.join("dangling.rs"),
    )?;
    for source in ["linked.rs", "linked-src/lib.rs", "escape.rs", "dangling.rs"] {
        let error = package_sources(&crate_root, source)
            .err()
            .ok_or("symlink unexpectedly passed")?;
        assert!(error.contains("symlinked"), "{source}: {error}");
    }
    let linked_crate = directory.path().canonicalize()?.join("linked-crate");
    symlink(&crate_root, &linked_crate)?;
    assert!(package_sources(&linked_crate, LISTING)
        .err()
        .ok_or("linked crate passed")?
        .contains("symlinked"));
    Ok(())
}

#[cfg(unix)]
#[test]
fn staging_rejects_linked_crates_and_hidden_nonlibrary_packages() -> Result<(), Box<dyn Error>> {
    use std::os::unix::fs::symlink;
    for relative in ["gamekit/linked", "gameskills/cli/linked", "games/linked"] {
        let directory = fixture()?;
        std::fs::create_dir_all(directory.path().join(relative).parent().ok_or("parent")?)?;
        symlink(
            directory.path().join("games/labyrinth"),
            directory.path().join(relative),
        )?;
        let mut ran = false;
        let error = check_with_runner(directory.path(), Case::Empty, |_, _| {
            ran = true;
            Err("Cargo must not run".into())
        })
        .err()
        .ok_or("linked package passed")?;
        assert!(error.contains("symlinked"), "{error}");
        assert!(!ran);
    }
    let directory = fixture()?;
    let source = directory.path().join("gamekit/facade/src/lib.rs");
    std::fs::remove_file(&source)?;
    symlink(directory.path().join("games/labyrinth/src/lib.rs"), &source)?;
    let mut calls = 0;
    let error = check_with_runner(directory.path(), Case::Empty, |_, args| {
        calls += 1;
        assert_eq!(
            args.first().map(OsString::as_os_str),
            Some(OsStr::new("package"))
        );
        Ok(LISTING.into())
    })
    .err()
    .ok_or("linked library source passed")?;
    assert!(error.contains("symlinked"));
    assert_eq!(calls, 1);
    Ok(())
}

#[test]
fn all_cases_stage_only_cargo_sources_embed_fixture_and_seed_lock() -> Result<(), Box<dyn Error>> {
    let directory = fixture()?;
    let root = directory.path().canonicalize()?;
    let mut calls = Vec::new();
    let mut staged = None;
    let report = check_with_runner(&root, Case::All, |cwd, args| {
        calls.push(args.to_vec());
        match args.first().and_then(|arg| arg.to_str()) {
            Some("package") => {
                assert_eq!(cwd, root);
                let package = args.get(2).ok_or("package missing")?;
                assert!(matches!(
                    package.to_str(),
                    Some("bevy-gamekit" | "bevy_game_hex")
                ));
                assert_eq!(
                    args,
                    [
                        OsString::from("package"),
                        OsString::from("-p"),
                        package.clone(),
                        OsString::from("--list"),
                        OsString::from("--allow-dirty")
                    ]
                );
                Ok(LISTING.into())
            }
            Some("tree") => {
                let staging = cwd.parent().ok_or("consumer has no parent")?;
                staged = Some(staging.to_path_buf());
                let library = staging.join("library");
                assert!(!library.join("games").exists());
                assert!(!library.join("devtools").exists());
                assert!(!root.join("scripts").exists());
                for capability in ["facade", "hex"] {
                    let capability = library.join("gamekit").join(capability);
                    assert!(!capability.join("not-packaged.rs").exists());
                    assert!(!capability.join("Cargo.lock").exists());
                    assert!(!capability.join("Cargo.toml.orig").exists());
                    assert_eq!(
                        std::fs::read_to_string(capability.join("README.md"))
                            .map_err(|error| error.to_string())?,
                        "Selected bytes\n"
                    );
                }
                assert_eq!(
                    std::fs::read_to_string(cwd.join("src/lib.rs"))
                        .map_err(|error| error.to_string())?,
                    include_str!("fixtures/distribution/gamekit_consumer.rs")
                );
                assert_eq!(
                    std::fs::read_to_string(cwd.join("Cargo.lock"))
                        .map_err(|error| error.to_string())?,
                    "owning lock versions\n"
                );
                let manifest: toml::Value = toml::from_str(
                    &std::fs::read_to_string(cwd.join("Cargo.toml"))
                        .map_err(|error| error.to_string())?,
                )
                .map_err(|error| error.to_string())?;
                let facade = manifest
                    .get("dependencies")
                    .and_then(|value| value.get("bevy_gamekit"))
                    .ok_or("facade dependency missing")?;
                assert_eq!(
                    facade.get("path").and_then(toml::Value::as_str),
                    library.join("gamekit/facade").to_str()
                );
                assert_eq!(
                    facade
                        .get("default-features")
                        .and_then(toml::Value::as_bool),
                    Some(false)
                );
                let mut expected = strings(&[
                    "tree",
                    "--edges",
                    "normal,build",
                    "--prefix",
                    "none",
                    "--format",
                    "{p}",
                ]);
                if let Some(feature) = feature_name(selected(args)) {
                    expected.extend(strings(&["--features", feature]));
                }
                assert_eq!(args, expected);
                Ok(graph(selected(args)))
            }
            Some("test") => {
                let mut expected = strings(&["test", "--profile", "ci", "--target-dir"]);
                expected.push(root.join("target").into_os_string());
                if let Some(feature) = feature_name(selected(args)) {
                    expected.extend(strings(&["--features", feature]));
                }
                assert_eq!(args, expected);
                Ok("captured child test output\n".into())
            }
            _ => Err("unexpected command".into()),
        }
    })?;
    assert_eq!(report.packages, ["bevy-gamekit", "bevy_game_hex"]);
    assert_eq!(report.staged_files, 6);
    assert_eq!(
        report
            .cases
            .iter()
            .map(|case| case.case)
            .collect::<Vec<_>>(),
        [Case::Empty, Case::Pure, Case::Ui, Case::Network]
    );
    assert_eq!(calls.len(), 10);
    assert!(!staged.ok_or("staging not inspected")?.exists());
    let json = serde_json::to_value(&report)?;
    assert_eq!(
        json.pointer("/cases/0/case")
            .and_then(serde_json::Value::as_str),
        Some("empty")
    );
    assert_eq!(
        std::fs::read_to_string(root.join("Cargo.lock"))?,
        "owning lock versions\n"
    );
    Ok(())
}

fn feature_name(case: Case) -> Option<&'static str> {
    match case {
        Case::Pure => Some("pure"),
        Case::Ui => Some("ui"),
        Case::Network => Some("network"),
        Case::Empty | Case::All => None,
    }
}

#[test]
fn selected_case_only_runs_its_graph_and_tests() -> Result<(), Box<dyn Error>> {
    let directory = fixture()?;
    for case in [Case::Empty, Case::Pure, Case::Ui, Case::Network] {
        let mut commands = Vec::new();
        let report = check_with_runner(directory.path(), case, |_, args| {
            let command = args.first().ok_or("command missing")?;
            commands.push(command.clone());
            if command == "package" {
                Ok(LISTING.into())
            } else {
                assert_eq!(selected(args), case);
                Ok(graph(case))
            }
        })?;
        assert_eq!(commands, strings(&["package", "package", "tree", "test"]));
        assert_eq!(report.cases.len(), 1);
        assert_eq!(report.cases.first().ok_or("case missing")?.case, case);
    }
    Ok(())
}

#[test]
fn cargo_failures_stop_verification_and_remove_temporary_sources() -> Result<(), Box<dyn Error>> {
    let directory = fixture()?;
    for failed in ["package", "tree", "test"] {
        let mut failure_seen = false;
        let mut staging = None;
        let result = check_with_runner(directory.path(), Case::All, |cwd, args| {
            assert!(!failure_seen, "command ran after a child failure");
            let command = args.first().ok_or("command missing")?;
            if command != "package" {
                staging = cwd.parent().map(Path::to_path_buf);
            }
            if command == failed {
                failure_seen = true;
                return Err(format!("synthetic {failed} exit 23"));
            }
            Ok(if command == "package" {
                LISTING.into()
            } else {
                graph(selected(args))
            })
        });
        assert_eq!(result.err(), Some(format!("synthetic {failed} exit 23")));
        assert!(failure_seen);
        if let Some(staging) = staging {
            assert!(!staging.exists());
        }
    }
    Ok(())
}

#[test]
fn forbidden_activated_graphs_never_reach_cargo_test() -> Result<(), Box<dyn Error>> {
    let directory = fixture()?;
    for case in [Case::Empty, Case::Pure, Case::Ui, Case::Network] {
        for forbidden in ["labyrinth", "labyrinth-rules", "gamekit-repo-tools"] {
            let result = check_with_runner(directory.path(), case, |_, args| {
                match args.first().and_then(|arg| arg.to_str()) {
                    Some("package") => Ok(LISTING.into()),
                    Some("tree") => Ok(format!("{}{forbidden} v0.1.0\n", graph(case))),
                    _ => Err("test must not run".into()),
                }
            });
            let error = result.err().ok_or("forbidden graph passed")?;
            assert!(
                error.contains("consumer resolved games or tools"),
                "{error}"
            );
            assert!(error.contains(forbidden));
        }
    }
    Ok(())
}

#[test]
fn missing_and_malformed_inputs_fail_before_cargo() -> Result<(), Box<dyn Error>> {
    for (path, replacement) in [
        ("Cargo.toml", None),
        ("Cargo.lock", None),
        (
            "Cargo.toml",
            Some("[workspace]\nresolver='3'\n[workspace.dependencies]\nbad={path="),
        ),
        ("games/labyrinth/Cargo.toml", Some("[package]\nname = [")),
        ("devtools/Cargo.toml", Some("[package]\nname = false")),
        (
            "gamekit/hex/Cargo.toml",
            Some("[package]\nname = 'hex'\n[dependencies]\nbad={"),
        ),
        (
            "gamekit/hex/Cargo.toml",
            Some("[package]\nversion = '0.1.0'"),
        ),
    ] {
        let directory = fixture()?;
        match replacement {
            Some(contents) => write(directory.path(), path, contents)?,
            None => std::fs::remove_file(directory.path().join(path))?,
        }
        let mut ran = false;
        let result = check_with_runner(directory.path(), Case::Empty, |_, _| {
            ran = true;
            Err("Cargo should not run for invalid inputs".into())
        });
        assert!(result.is_err(), "{path}");
        assert!(!ran, "{path}");
    }
    Ok(())
}

#[test]
fn invalid_package_list_stops_before_graph_commands() -> Result<(), Box<dyn Error>> {
    for listing in [
        "Cargo.toml",
        "Cargo.toml\nsrc/lib.rs\n../outside.rs",
        "Cargo.toml\nsrc/lib.rs\nmissing.rs",
    ] {
        let directory = fixture()?;
        let mut calls = 0;
        let result = check_with_runner(directory.path(), Case::Empty, |_, args| {
            calls += 1;
            assert_eq!(
                args.first().map(OsString::as_os_str),
                Some(OsStr::new("package"))
            );
            Ok(listing.into())
        });
        assert!(result.is_err());
        assert_eq!(calls, 1);
    }
    Ok(())
}
