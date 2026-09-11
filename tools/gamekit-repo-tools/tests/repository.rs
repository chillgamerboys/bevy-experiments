//! Repository policy regression cases; no Cargo build or network dependency.

use gamekit_repo_tools::repository::check;
use std::path::Path;

fn write(root: &Path, name: &str, content: &str) {
    let path = root.join(name);
    std::fs::create_dir_all(path.parent().expect("fixture parent")).expect("fixture directory");
    std::fs::write(path, content).expect("fixture file");
}

fn fixture() -> tempfile::TempDir {
    let temporary = tempfile::tempdir().expect("fixture root");
    write(
        temporary.path(),
        "Cargo.toml",
        "[workspace]\nmembers = [\"crates/*\", \"games/*\"]\n",
    );
    write(
        temporary.path(),
        "games/example/Cargo.toml",
        "[package]\nname = \"example\"\nversion = \"0.1.0\"\n",
    );
    temporary
}

#[test]
fn valid_workspace_and_ignored_output() {
    let temporary = fixture();
    let root = temporary.path();
    write(
        root,
        "README.md",
        "[Game](games/example/Cargo.toml)\n```sh\n[x](absent)\n```\n",
    );
    for directory in [
        "target",
        ".context",
        ".gameskills/bundles/candidate",
        ".git",
        "__pycache__",
    ] {
        write(root, &format!("{directory}/README.md"), "[broken](absent)");
        write(root, &format!("{directory}/Cargo.toml"), "invalid TOML");
    }
    assert_eq!(check(root), Vec::<String>::new());
}

#[test]
fn missing_and_escaping_links() {
    let temporary = fixture();
    write(
        temporary.path(),
        "README.md",
        "[missing](missing.md) [outside](../Cargo.toml)",
    );
    assert_eq!(check(temporary.path()).len(), 2);
}

#[test]
fn game_dependency_through_workspace_alias() {
    let temporary = fixture();
    write(temporary.path(), "Cargo.toml", "[workspace]\nmembers = []\n[workspace.dependencies]\nalias = { package = \"example\", path = \"games/example\" }\n");
    write(temporary.path(), "crates/cap/Cargo.toml", "[package]\nname = \"cap\"\nversion = \"0.1.0\"\n[dev-dependencies]\nalias.workspace = true\n");
    assert!(check(temporary.path())
        .iter()
        .any(|error| error.contains("depends on game")));
}

#[test]
fn legacy_and_duplicate_workspace() {
    let temporary = fixture();
    write(temporary.path(), "src/lib.rs", "");
    write(temporary.path(), "games/example/Cargo.lock", "");
    write(
        temporary.path(),
        "games/example/Cargo.toml",
        "[workspace]\nmembers = []\n",
    );
    assert_eq!(check(temporary.path()).len(), 3);
}

#[test]
fn capability_cannot_depend_back_on_facade() {
    let temporary = fixture();
    write(temporary.path(), "Cargo.toml", "[workspace]\nmembers = []\n[workspace.dependencies]\nbevy_gamekit = { package = \"bevy-gamekit\", path = \"crates/bevy_gamekit\" }\n");
    write(temporary.path(), "crates/cap/Cargo.toml", "[package]\nname = \"cap\"\nversion = \"0.1.0\"\n[target.'cfg(unix)'.build-dependencies]\nbevy_gamekit.workspace = true\n");
    assert!(check(temporary.path())
        .iter()
        .any(|error| error.contains("depends on facade")));
}

#[test]
fn invalid_and_missing_manifests_return_failures() {
    let temporary = tempfile::tempdir().expect("fixture");
    assert_eq!(check(temporary.path()).len(), 1);
    write(temporary.path(), "Cargo.toml", "[workspace");
    assert!(check(temporary.path())
        .iter()
        .any(|error| error.contains("invalid Cargo manifest")));
    write(
        temporary.path(),
        "Cargo.toml",
        "[package]\nname = \"legacy\"\n[workspace]\n",
    );
    assert!(check(temporary.path())
        .iter()
        .any(|error| error.contains("virtual workspace")));
    write(temporary.path(), "Cargo.toml", "[workspace]\n");
    write(temporary.path(), "crates/broken/Cargo.toml", "[package");
    assert!(check(temporary.path())
        .iter()
        .any(|error| error.contains("invalid Cargo manifest")));
}

#[test]
fn all_dependency_classes_and_missing_game_paths_are_checked() {
    for heading in [
        "dependencies",
        "dev-dependencies",
        "build-dependencies",
        "target.'cfg(unix)'.dependencies",
        "target.'cfg(unix)'.dev-dependencies",
        "target.'cfg(unix)'.build-dependencies",
    ] {
        let temporary = fixture();
        write(temporary.path(), "crates/cap/Cargo.toml", &format!("[package]\nname = \"cap\"\n[{heading}]\nalias = {{ path = \"../../games/missing\" }}\n"));
        assert!(
            check(temporary.path())
                .iter()
                .any(|error| error.contains("depends on game")),
            "{heading}"
        );
    }
}

#[test]
fn missing_workspace_alias_and_wrong_path_type_fail() {
    let temporary = fixture();
    write(
        temporary.path(),
        "crates/cap/Cargo.toml",
        "[package]\nname = \"cap\"\n[dependencies]\nunknown.workspace = true\nbad.path = 3\n",
    );
    let failures = check(temporary.path());
    assert!(failures
        .iter()
        .any(|error| error.contains("missing workspace dependency unknown")));
    assert!(failures
        .iter()
        .any(|error| error.contains("path must be a string")));
}

#[test]
fn dependency_paths_do_not_gain_absolute_semantics_on_another_host() {
    for path in [
        "C:/games/example",
        "games\\example",
        "/absolute/games/example",
    ] {
        let temporary = fixture();
        write(
            temporary.path(),
            "crates/cap/Cargo.toml",
            &format!("[package]\nname = 'cap'\n[dependencies]\nalias.path = '{path}'\n"),
        );
        assert!(!check(temporary.path()).is_empty(), "{path}");
    }
}

#[cfg(unix)]
#[test]
fn ownership_resolves_symlink_ancestor_before_missing_leaf() {
    let temporary = fixture();
    std::os::unix::fs::symlink("games", temporary.path().join("alias")).expect("fixture symlink");
    write(
        temporary.path(),
        "crates/cap/Cargo.toml",
        "[package]\nname = \"cap\"\n[dependencies]\nalias = { path = \"../../alias/missing\" }\n",
    );
    assert!(check(temporary.path())
        .iter()
        .any(|error| error.contains("depends on game")));
}
