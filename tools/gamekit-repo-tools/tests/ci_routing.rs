//! CI routing contracts over committed Git fixtures, without Cargo metadata or game builds.

use gamekit_repo_tools::ci::{self, Selection};
use std::error::Error;
use std::path::Path;
use std::process::Command;

type TestResult<T = ()> = Result<T, Box<dyn Error>>;

const WORKSPACE: &str = r#"[workspace]
members = ["crates/*", "games/*", "games/labyrinth/rules"]
[workspace.dependencies]
shared = { package = "bevy_game_ui", path = "crates/ui" }
"#;

const PACKAGES: &[(&str, &str, &str)] = &[
    ("crates/ui", "bevy_game_ui", ""),
    (
        "crates/test",
        "bevy_game_test",
        "[dev-dependencies]\nshared.workspace = true\n",
    ),
    (
        "crates/facade",
        "bevy-gamekit",
        "[target.'cfg(unix)'.build-dependencies]\nhelper = { package = \"bevy_game_test\", path = \"../test\" }\n",
    ),
    ("games/labyrinth/rules", "labyrinth_rules", ""),
    (
        "games/labyrinth",
        "labyrinth",
        "[dependencies]\nfacade = { path = \"../../crates/facade\" }\nrules = { path = \"rules\" }\n",
    ),
    (
        "games/deckbuilder_ui",
        "deckbuilder_ui",
        "[dependencies]\nfacade = { path = \"../../crates/facade\" }\n",
    ),
    (
        "games/carterfight",
        "carterfight",
        "[dependencies]\nfacade = { path = \"../../crates/facade\" }\n",
    ),
];

struct Fixture {
    directory: tempfile::TempDir,
    base: String,
}

impl Fixture {
    fn new() -> TestResult<Self> {
        let mut fixture = Self {
            directory: tempfile::Builder::new()
                .prefix("ci routing $(literal) with spaces ")
                .tempdir()?,
            base: String::new(),
        };
        fixture.git(&["init", "-q"])?;
        for (key, value) in [
            ("user.name", "CI test"),
            ("user.email", "ci@example.invalid"),
            ("commit.gpgsign", "false"),
            ("core.autocrlf", "false"),
        ] {
            fixture.git(&["config", key, value])?;
        }
        fixture.write("Cargo.toml", WORKSPACE)?;
        for (folder, name, dependencies) in PACKAGES {
            fixture.package(folder, name, dependencies)?;
        }
        fixture.write("docs/existing.md", "Existing prose\n")?;
        fixture.rebase()?;
        Ok(fixture)
    }

    fn root(&self) -> &Path {
        self.directory.path()
    }

    fn git(&self, arguments: &[&str]) -> TestResult<String> {
        let output = Command::new("git")
            .args(arguments)
            .current_dir(self.root())
            .output()?;
        if !output.status.success() {
            return Err(format!(
                "Git {arguments:?} failed: {}",
                String::from_utf8_lossy(&output.stderr)
            )
            .into());
        }
        Ok(String::from_utf8(output.stdout)?.trim().to_owned())
    }

    fn write(&self, path: &str, text: &str) -> TestResult {
        let target = self.root().join(path);
        std::fs::create_dir_all(target.parent().ok_or("fixture file has no parent")?)?;
        std::fs::write(target, text)?;
        Ok(())
    }

    fn package(&self, folder: &str, name: &str, dependencies: &str) -> TestResult {
        self.write(
            &format!("{folder}/Cargo.toml"),
            &format!("[package]\nname = {name:?}\nversion = \"0.1.0\"\n{dependencies}"),
        )?;
        self.write(&format!("{folder}/src/lib.rs"), "// fixture\n")
    }

    fn replace(&self, path: &str, before: &str, after: &str) -> TestResult {
        let source = std::fs::read_to_string(self.root().join(path))?;
        assert!(source.contains(before), "fixture replacement must match");
        self.write(path, &source.replace(before, after))
    }

    fn save(&self) -> TestResult<String> {
        self.git(&["add", "."])?;
        self.git(&["commit", "--allow-empty", "-qm", "fixture"])?;
        self.git(&["rev-parse", "HEAD"])
    }

    fn rebase(&mut self) -> TestResult {
        self.base = self.save()?;
        Ok(())
    }

    fn select(&self) -> TestResult<Selection> {
        let head = self.save()?;
        Ok(ci::select(self.root(), Some(&self.base), &head, false)?)
    }

    fn changed(&self, paths: &[&str]) -> TestResult<Selection> {
        for path in paths {
            self.write(path, "changed\n")?;
        }
        self.select()
    }

    fn reset(&self) -> TestResult {
        self.git(&["reset", "--hard", &self.base])?;
        Ok(())
    }

    fn add_tool(&mut self) -> TestResult {
        self.replace("Cargo.toml", "\"crates/*\",", "\"tools/*\", \"crates/*\",")?;
        self.package("tools/gamekit-repo-tools", "gamekit-repo-tools", "")?;
        self.write("tools/gamekit-repo-tools/src/catalog.rs", "// catalog\n")?;
        self.rebase()
    }
}

fn flags(selection: &Selection) -> [bool; 7] {
    [
        selection.skills,
        selection.rust,
        selection.policy,
        selection.distribution,
        selection.minimal,
        selection.wasm,
        selection.deny,
    ]
}

fn assert_full(selection: &Selection) {
    assert!(selection.full, "{selection:?}");
    assert!(
        flags(selection).into_iter().all(|value| value),
        "{selection:?}"
    );
    assert!(selection.packages.is_empty(), "{selection:?}");
    assert!(!selection.reasons.is_empty(), "{selection:?}");
}

fn assert_packages(selection: &Selection, packages: &[&str]) {
    assert!(!selection.full, "{selection:?}");
    assert_eq!(selection.packages, packages, "{selection:?}");
    assert!(selection.rust && selection.policy, "{selection:?}");
}

#[test]
fn test_docs_avoid_expensive_jobs() -> TestResult {
    let fixture = Fixture::new()?;
    let value = fixture.changed(&["docs/testing.md", "games/labyrinth/README.md", "README.md"])?;
    assert!(!value.full, "{value:?}");
    assert!(!flags(&value).into_iter().any(|flag| flag), "{value:?}");
    assert!(value.packages.is_empty());
    assert_eq!(value.schema_version, 1);
    assert_eq!(value.base.as_deref(), Some(fixture.base.as_str()));
    assert_eq!(value.head, fixture.git(&["rev-parse", "HEAD"])?);
    Ok(())
}

#[test]
fn test_skill_markdown_and_runtime_select_skills_only() -> TestResult {
    let fixture = Fixture::new()?;
    let value = fixture.changed(&[
        "plugins/gameskills/skills/plan/SKILL.md",
        "gameskills.toml",
        "plugins/gameskills/runtime/native.py",
    ])?;
    assert!(!value.full, "{value:?}");
    assert_eq!(
        flags(&value),
        [true, false, false, false, false, false, false]
    );
    Ok(())
}

#[test]
fn test_each_game_is_isolated() -> TestResult {
    let fixture = Fixture::new()?;
    for name in ["labyrinth", "deckbuilder_ui", "carterfight"] {
        let value = fixture.changed(&[&format!("games/{name}/src/lib.rs")])?;
        assert_packages(&value, &[name]);
        assert_eq!(
            flags(&value),
            [false, true, true, false, false, false, false]
        );
        fixture.reset()?;
    }
    Ok(())
}

#[test]
fn test_nested_rules_owner_and_consumer() -> TestResult {
    let fixture = Fixture::new()?;
    let value = fixture.changed(&["games/labyrinth/rules/src/lib.rs"])?;
    assert_packages(&value, &["labyrinth", "labyrinth_rules"]);
    assert_eq!(
        flags(&value),
        [false, true, true, false, false, true, false]
    );
    Ok(())
}

#[test]
fn test_reverse_normal_dev_target_build_consumers() -> TestResult {
    let fixture = Fixture::new()?;
    let value = fixture.changed(&["crates/ui/src/lib.rs"])?;
    assert_packages(
        &value,
        &[
            "bevy-gamekit",
            "bevy_game_test",
            "bevy_game_ui",
            "carterfight",
            "deckbuilder_ui",
            "labyrinth",
        ],
    );
    assert_eq!(flags(&value), [false, true, true, true, true, true, false]);
    Ok(())
}

#[test]
fn test_mixed_changes_union_jobs() -> TestResult {
    let fixture = Fixture::new()?;
    let value = fixture.changed(&[
        "games/carterfight/assets/scene.png",
        "skills/README.md",
        "docs/test.md",
    ])?;
    assert_packages(&value, &["carterfight"]);
    assert_eq!(
        flags(&value),
        [true, true, true, false, false, false, false]
    );
    Ok(())
}

#[test]
fn test_shared_unknown_and_routing_inputs_select_full() -> TestResult {
    let fixture = Fixture::new()?;
    for path in [
        "unknown/config.json",
        "Cargo.lock",
        "deny.toml",
        "rust-toolchain",
        "rust-toolchain.toml",
        ".cargo/config.toml",
        ".github/workflows/gamekit.yml",
        "scripts/ci.py",
        "scripts/tests/test_ci.py",
        "unknown/README.md",
    ] {
        assert_full(&fixture.changed(&[path])?);
        fixture.reset()?;
    }
    Ok(())
}

#[test]
fn test_missing_history_and_bad_graph_select_full() -> TestResult {
    let fixture = Fixture::new()?;
    for base in [
        None,
        Some("main"),
        Some("0000000000000000000000000000000000000000"),
    ] {
        assert_full(&ci::select(fixture.root(), base, &fixture.base, false)?);
    }
    fixture.write("crates/ui/Cargo.toml", "broken TOML")?;
    assert_full(&fixture.select()?);
    Ok(())
}

#[test]
fn test_invalid_head_is_error() -> TestResult {
    let fixture = Fixture::new()?;
    for head in [
        "main",
        "HEAD",
        "--help",
        "0000000000000000000000000000000000000000",
        "AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA",
    ] {
        assert!(
            ci::select(fixture.root(), Some(&fixture.base), head, false).is_err(),
            "{head}"
        );
        assert!(
            ci::select(fixture.root(), None, head, true).is_err(),
            "{head}"
        );
    }
    Ok(())
}

#[test]
fn test_deleted_source_selects_owner() -> TestResult {
    let fixture = Fixture::new()?;
    std::fs::remove_file(fixture.root().join("games/carterfight/src/lib.rs"))?;
    assert_packages(&fixture.select()?, &["carterfight"]);
    Ok(())
}

#[test]
fn test_rename_selects_both_owners() -> TestResult {
    let fixture = Fixture::new()?;
    fixture.git(&[
        "mv",
        "games/carterfight/src/lib.rs",
        "games/deckbuilder_ui/src/moved.rs",
    ])?;
    let value = fixture.select()?;
    assert_packages(&value, &["carterfight", "deckbuilder_ui"]);
    assert_eq!(
        value.paths,
        [
            "games/carterfight/src/lib.rs",
            "games/deckbuilder_ui/src/moved.rs"
        ]
    );
    Ok(())
}

#[test]
fn test_included_docs_and_old_graph_consumers() -> TestResult {
    let mut fixture = Fixture::new()?;
    fixture.write(
        "games/carterfight/src/lib.rs",
        "const HELP: &str = include_str!(\"../../../docs/existing.md\");\n",
    )?;
    fixture.rebase()?;
    assert_packages(&fixture.changed(&["docs/existing.md"])?, &["carterfight"]);
    assert_packages(
        &fixture.changed(&["games/carterfight/src/lib.rs"])?,
        &["carterfight"],
    );
    Ok(())
}

#[test]
fn test_dynamic_include_prevents_docs_shortcut() -> TestResult {
    let mut fixture = Fixture::new()?;
    fixture.write(
        "games/carterfight/src/lib.rs",
        "include!(concat!(env!(\"OUT_DIR\"), \"/generated.rs\"));\n",
    )?;
    fixture.rebase()?;
    assert_full(&fixture.changed(&["docs/existing.md"])?);
    Ok(())
}

#[test]
fn test_cross_game_asset_keeps_owner_and_include_consumer() -> TestResult {
    let mut fixture = Fixture::new()?;
    fixture.write("games/carterfight/assets/shared.bin", "asset")?;
    fixture.write(
        "games/labyrinth/src/lib.rs",
        "const DATA: &[u8] = include_bytes!(\"../../carterfight/assets/shared.bin\");\n",
    )?;
    fixture.rebase()?;
    assert_packages(
        &fixture.changed(&["games/carterfight/assets/shared.bin"])?,
        &["carterfight", "labyrinth"],
    );
    Ok(())
}

#[test]
fn test_brace_and_bracket_includes_select_compiled_docs() -> TestResult {
    for (opening, closing) in [("{", "}"), ("[", "]")] {
        let mut fixture = Fixture::new()?;
        fixture.write(
            "games/carterfight/src/lib.rs",
            &format!(
                "const HELP: &str = include_str!{opening}\"../../../docs/existing.md\"{closing};\n"
            ),
        )?;
        fixture.rebase()?;
        assert_packages(&fixture.changed(&["docs/existing.md"])?, &["carterfight"]);
    }
    Ok(())
}

#[test]
fn test_dynamic_include_also_prevents_asset_shortcut() -> TestResult {
    let mut fixture = Fixture::new()?;
    fixture.write("games/labyrinth/src/lib.rs", "include_bytes!(concat!(env!(\"CARGO_MANIFEST_DIR\"), \"/../carterfight/assets/shared.bin\"));\n")?;
    fixture.rebase()?;
    assert_full(&fixture.changed(&["games/carterfight/assets/shared.bin"])?);
    Ok(())
}

#[test]
fn test_explicit_build_script_prevents_docs_shortcut() -> TestResult {
    let mut fixture = Fixture::new()?;
    fixture.replace(
        "games/carterfight/Cargo.toml",
        "[package]",
        "[package]\nbuild = \"codegen.rs\"",
    )?;
    fixture.write(
        "games/carterfight/codegen.rs",
        "fn main() { let _ = std::fs::read(\"../../docs/existing.md\"); }",
    )?;
    fixture.rebase()?;
    assert_full(&fixture.changed(&["docs/existing.md"])?);
    Ok(())
}

#[test]
fn test_included_skill_keeps_skill_checks() -> TestResult {
    let mut fixture = Fixture::new()?;
    fixture.write("plugins/help.md", "Skill instructions")?;
    fixture.write(
        "games/carterfight/src/lib.rs",
        "const HELP: &str = include_str!(\"../../../plugins/help.md\");\n",
    )?;
    fixture.rebase()?;
    let value = fixture.changed(&["plugins/help.md"])?;
    assert_packages(&value, &["carterfight"]);
    assert!(value.skills);
    Ok(())
}

#[test]
fn test_manual_full_run() -> TestResult {
    let fixture = Fixture::new()?;
    assert_full(&ci::select(fixture.root(), None, &fixture.base, true)?);
    Ok(())
}

#[test]
fn test_repository_tool_changes_keep_owned_validation_without_game_tests() -> TestResult {
    let mut fixture = Fixture::new()?;
    fixture.add_tool()?;
    // The CI cutover makes lib.rs a shared classifier input. Catalog behavior
    // retains R2a's selective tool-only contract.
    let value = fixture.changed(&["tools/gamekit-repo-tools/src/catalog.rs"])?;
    assert_packages(&value, &["gamekit-repo-tools"]);
    assert_eq!(flags(&value), [true, true, true, true, false, false, false]);
    fixture.reset()?;
    let value = fixture.changed(&["tools/gamekit-repo-tools/README.md"])?;
    assert!(!value.full, "{value:?}");
    assert!(!flags(&value).into_iter().any(|flag| flag), "{value:?}");
    Ok(())
}

#[test]
fn rust_ci_implementation_and_regressions_are_shared_inputs() -> TestResult {
    let mut fixture = Fixture::new()?;
    fixture.add_tool()?;
    for path in [
        "tools/gamekit-repo-tools/src/lib.rs",
        "tools/gamekit-repo-tools/src/main.rs",
        "tools/gamekit-repo-tools/src/support.rs",
        "tools/gamekit-repo-tools/src/ci/mod.rs",
        "tools/gamekit-repo-tools/src/ci/checks.rs",
        "tools/gamekit-repo-tools/tests/ci_routing.rs",
        "tools/gamekit-repo-tools/tests/fixtures/ci/selection.json",
    ] {
        assert_full(&fixture.changed(&[path])?);
        fixture.reset()?;
    }
    Ok(())
}

#[test]
fn literal_include_whitespace_comments_raw_strings_and_escapes_keep_consumers() -> TestResult {
    for source in [
        "const HELP: &str = include_str /* before bang */ ! /* before group */ ( /* before literal */ \"../../../docs/existing.md\" /* after literal */, /* trailing */ );\n",
        "const HELP: &str = include_str!(r#\"../../../docs/existing.md\"#);\n",
        "const HELP: &str = include_str!(\"../../../docs/existing\\x2emd\");\n",
        "const HELP: &str = include_str!(\"../../../docs/existing\\u{2e}md\");\n",
    ] {
        let mut fixture = Fixture::new()?;
        fixture.write("games/carterfight/src/lib.rs", source)?;
        fixture.rebase()?;
        assert_packages(&fixture.changed(&["docs/existing.md"])?, &["carterfight"]);
    }
    Ok(())
}

#[test]
fn include_spelling_in_comments_and_strings_is_not_a_compiled_input() -> TestResult {
    let mut fixture = Fixture::new()?;
    fixture.write("games/carterfight/src/lib.rs", "// include_str!(concat!(\"not code\"));\n/* include!(env!(\"NOT_CODE\")); */\nconst EXAMPLE: &str = r#\"include_bytes!(dynamic())\"#;\n")?;
    fixture.rebase()?;
    let value = fixture.changed(&["docs/existing.md"])?;
    assert!(!value.full, "{value:?}");
    assert!(!flags(&value).into_iter().any(|flag| flag), "{value:?}");
    Ok(())
}

#[test]
fn removed_include_and_deleted_compiled_doc_retain_old_consumers() -> TestResult {
    let mut fixture = Fixture::new()?;
    fixture.write(
        "crates/ui/src/lib.rs",
        "const HELP: &str = include_str!(\"../../../docs/existing.md\");\n",
    )?;
    fixture.rebase()?;
    fixture.write("crates/ui/src/lib.rs", "// include removed\n")?;
    std::fs::remove_file(fixture.root().join("docs/existing.md"))?;
    assert_packages(
        &fixture.select()?,
        &[
            "bevy-gamekit",
            "bevy_game_test",
            "bevy_game_ui",
            "carterfight",
            "deckbuilder_ui",
            "labyrinth",
        ],
    );
    Ok(())
}

#[test]
fn deleted_and_renamed_narrative_docs_do_not_select_jobs() -> TestResult {
    for rename in [false, true] {
        let fixture = Fixture::new()?;
        if rename {
            fixture.git(&["mv", "docs/existing.md", "docs/renamed.md"])?;
        } else {
            std::fs::remove_file(fixture.root().join("docs/existing.md"))?;
        }
        let value = fixture.select()?;
        assert!(!value.full, "{value:?}");
        assert!(!flags(&value).into_iter().any(|flag| flag), "{value:?}");
        assert!(value.paths.iter().any(|path| path == "docs/existing.md"));
    }
    Ok(())
}

#[test]
fn automatic_build_script_prevents_narrow_selection() -> TestResult {
    let mut fixture = Fixture::new()?;
    fixture.write("games/carterfight/build.rs", "fn main() {}\n")?;
    fixture.rebase()?;
    assert_full(&fixture.changed(&["docs/existing.md"])?);
    Ok(())
}

#[test]
fn disabled_explicit_build_without_build_file_allows_docs_selection() -> TestResult {
    let mut fixture = Fixture::new()?;
    fixture.replace(
        "games/carterfight/Cargo.toml",
        "[package]",
        "[package]\nbuild = false",
    )?;
    fixture.rebase()?;
    let value = fixture.changed(&["docs/existing.md"])?;
    assert!(!value.full, "{value:?}");
    assert!(!flags(&value).into_iter().any(|flag| flag), "{value:?}");
    Ok(())
}

#[test]
fn inherited_and_direct_aliases_cover_every_dependency_table() -> TestResult {
    for table in [
        "dependencies",
        "dev-dependencies",
        "build-dependencies",
        "target.'cfg(unix)'.dependencies",
        "target.'cfg(windows)'.dev-dependencies",
        "target.'cfg(target_arch = \"wasm32\")'.build-dependencies",
    ] {
        for dependency in [
            "shared.workspace = true",
            "renamed = { package = \"bevy_game_ui\", path = \"../ui/./\" }",
        ] {
            let mut fixture = Fixture::new()?;
            fixture.package(
                "crates/test",
                "bevy_game_test",
                &format!("[{table}]\n{dependency}\n"),
            )?;
            fixture.rebase()?;
            let value = fixture.changed(&["crates/ui/src/lib.rs"])?;
            assert_packages(
                &value,
                &[
                    "bevy-gamekit",
                    "bevy_game_test",
                    "bevy_game_ui",
                    "carterfight",
                    "deckbuilder_ui",
                    "labyrinth",
                ],
            );
        }
    }
    Ok(())
}

#[test]
fn member_component_globs_and_exclusions_preserve_ownership() -> TestResult {
    let mut fixture = Fixture::new()?;
    fixture.replace(
        "Cargo.toml",
        "\"crates/*\"",
        "\"crates/[uf]*\", \"crates/tes?\"",
    )?;
    fixture.replace(
        "Cargo.toml",
        "[workspace.dependencies]",
        "exclude = [\"crates/unused\"]\n[workspace.dependencies]",
    )?;
    fixture.write("crates/unused/Cargo.toml", "excluded invalid TOML")?;
    fixture.rebase()?;
    let value = fixture.changed(&["crates/ui/src/lib.rs"])?;
    assert_packages(
        &value,
        &[
            "bevy-gamekit",
            "bevy_game_test",
            "bevy_game_ui",
            "carterfight",
            "deckbuilder_ui",
            "labyrinth",
        ],
    );
    Ok(())
}

#[test]
fn single_component_member_glob_does_not_claim_nested_packages() -> TestResult {
    let mut fixture = Fixture::new()?;
    fixture.replace("Cargo.toml", ", \"games/labyrinth/rules\"", "")?;
    fixture.replace(
        "games/labyrinth/Cargo.toml",
        "rules = { path = \"rules\" }\n",
        "",
    )?;
    fixture.rebase()?;
    let value = fixture.changed(&["games/labyrinth/rules/src/lib.rs"])?;
    assert_packages(&value, &["labyrinth"]);
    assert!(!value.wasm);
    Ok(())
}

#[test]
fn malformed_committed_graphs_conservatively_select_every_check() -> TestResult {
    for (path, source) in [
        ("Cargo.toml", "[workspace]\nmembers = \"crates/*\"\n"),
        ("Cargo.toml", "[workspace]\nmembers = [1]\n"),
        ("Cargo.toml", "[workspace]\nmembers = []\n"),
        ("Cargo.toml", "[workspace]\nmembers = [\"crates/*\"]\nexclude = 1\n"),
        ("crates/ui/Cargo.toml", "[package]\nname = \"--help\"\n"),
        ("crates/ui/Cargo.toml", "[package]\nname = \"bevy_game_test\"\n"),
        ("crates/ui/Cargo.toml", "[package]\nname = 42\n"),
        ("crates/test/Cargo.toml", "[package]\nname = \"bevy_game_test\"\n[dependencies]\nmissing.workspace = true\n"),
        ("crates/test/Cargo.toml", "[package]\nname = \"bevy_game_test\"\n[dependencies]\nexternal = { path = \"../absent\" }\n"),
        ("crates/test/Cargo.toml", "dependencies = 3\n[package]\nname = \"bevy_game_test\"\n"),
    ] {
        let mut fixture = Fixture::new()?;
        fixture.write(path, source)?;
        fixture.rebase()?;
        assert_full(&fixture.changed(&["docs/existing.md"])?);
    }
    Ok(())
}

#[test]
fn removed_dependency_or_package_never_hides_previous_consumers() -> TestResult {
    for remove_package in [false, true] {
        let fixture = Fixture::new()?;
        if remove_package {
            std::fs::remove_dir_all(fixture.root().join("crates/ui"))?;
        } else {
            fixture.package("crates/test", "bevy_game_test", "")?;
        }
        fixture.write("docs/existing.md", "changed\n")?;
        // Cargo manifests are shared inputs; a removed edge cannot underselect
        // the old graph even when the new graph has fewer reverse consumers.
        assert_full(&fixture.select()?);
    }
    Ok(())
}

#[test]
fn selection_reads_committed_objects_despite_unstaged_and_staged_changes() -> TestResult {
    let fixture = Fixture::new()?;
    fixture.write("docs/existing.md", "committed prose\n")?;
    let head = fixture.save()?;
    fixture.write("Cargo.toml", "invalid uncommitted manifest")?;
    fixture.write("games/carterfight/src/lib.rs", "include!(dynamic());\n")?;
    fixture.git(&["add", "."])?;
    fixture.write("unknown/secret.txt", "untracked input\n")?;
    let value = ci::select(fixture.root(), Some(&fixture.base), &head, false)?;
    assert!(!value.full, "{value:?}");
    assert!(!flags(&value).into_iter().any(|flag| flag), "{value:?}");
    assert_eq!(value.paths, ["docs/existing.md"]);
    Ok(())
}

#[test]
fn empty_committed_diff_selects_no_jobs_even_with_dynamic_inputs() -> TestResult {
    let mut fixture = Fixture::new()?;
    fixture.write("games/carterfight/src/lib.rs", "include!(dynamic());\n")?;
    fixture.rebase()?;
    let value = ci::select(fixture.root(), Some(&fixture.base), &fixture.base, false)?;
    assert!(!value.full, "{value:?}");
    assert!(!flags(&value).into_iter().any(|flag| flag), "{value:?}");
    assert!(value.paths.is_empty());
    Ok(())
}

#[test]
fn raw_include_identifiers_preserve_literal_and_dynamic_inputs() -> TestResult {
    for source in [
        r#"pub const HELP: &str = r#include_str!("../../../docs/existing.md");"#,
        r#"pub const HELP: &[u8] = r#include_bytes!("../../../docs/existing.md");"#,
        r#"r#include!("../../../docs/existing.md");"#,
    ] {
        let mut fixture = Fixture::new()?;
        fixture.write("games/carterfight/src/lib.rs", source)?;
        fixture.rebase()?;
        assert_packages(&fixture.changed(&["docs/existing.md"])?, &["carterfight"]);
    }
    for source in [
        r#"r#include_str!(concat!("../../../docs/", "existing.md"));"#,
        r#"r#include!(env!("GENERATED_SOURCE"));"#,
    ] {
        let mut fixture = Fixture::new()?;
        fixture.write("games/carterfight/src/lib.rs", source)?;
        fixture.rebase()?;
        assert_full(&fixture.changed(&["docs/existing.md"])?);
    }
    Ok(())
}
