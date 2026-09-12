//! Adopter documentation discovery without native installation.
#![allow(
    clippy::indexing_slicing,
    reason = "Assertions address known JSON fixture fields; a missing field must fail the test."
)]
use gameskills_cli::{config, docs};
use std::{fs, path::Path};
fn write(root: &Path, path: &str, text: &str) {
    let p = root.join(path);
    fs::create_dir_all(p.parent().expect("test fixture")).expect("test fixture");
    fs::write(p, text).expect("test fixture");
}
fn parse(extra: &str) -> toml::Table {
    config::parse(&format!("schema_version=1\n{extra}")).expect("test fixture")
}
#[test]
fn discovery_is_read_only_and_supports_readme_only_and_missing_docs() {
    let d = tempfile::tempdir().expect("test fixture");
    let c = parse("");
    let empty = docs::resolve(d.path(), &c, &[]).expect("test fixture");
    assert_eq!(empty["indexes"], serde_json::json!([]));
    assert_eq!(
        empty["diagnostics"].as_array().expect("test fixture").len(),
        1
    );
    write(d.path(), "README.md", "# Consumer");
    let r = docs::resolve(d.path(), &c, &[]).expect("test fixture");
    assert_eq!(r["indexes"], serde_json::json!(["README.md"]));
    assert!(!d.path().join(".gameskills").exists());
    write(d.path(), "docs/README.md", "# Docs");
    assert_eq!(
        docs::resolve(d.path(), &c, &[]).expect("test fixture")["indexes"],
        serde_json::json!(["docs/README.md"])
    );
}
#[test]
fn nested_mixed_and_external_docs_resolve_by_components() {
    let d = tempfile::tempdir().expect("test fixture");
    for p in [
        "README.md",
        "games/a/README.md",
        "games/a/rules/README.md",
        "manual/b/index.md",
    ] {
        write(d.path(), p, "# Guide");
    }
    let c=parse("[targets.a]\npath='games/a'\n[targets.rules]\npath='games/a/rules'\n[targets.b]\npath='games/b'\n[targets.b.docs]\nindex='manual/b/index.md'\nplans='planning/b'");
    let r = docs::resolve(
        d.path(),
        &c,
        &[
            "games/a/rules/new.rs".into(),
            "manual/b/topic.md".into(),
            "planning/b/task.md".into(),
        ],
    )
    .expect("test fixture");
    assert_eq!(
        r["indexes"],
        serde_json::json!(["README.md", "manual/b/index.md", "games/a/rules/README.md"])
    );
    assert_eq!(
        docs::resolve(d.path(), &c, &["games/a-other/a.rs".into()]).expect("test fixture")
            ["indexes"],
        serde_json::json!(["README.md"])
    );
}
#[test]
fn explicit_mapping_failures_and_ambiguous_ownership_are_actionable() {
    let d = tempfile::tempdir().expect("test fixture");
    let c = parse("[docs]\nindex='missing.md'");
    assert!(docs::resolve(d.path(), &c, &[])
        .expect_err("invalid fixture")
        .contains("configured documentation index"));
    for p in ["manual/a.md", "manual/b.md"] {
        write(d.path(), p, "# Guide");
    }
    let c=parse("[targets.a]\npath='a'\n[targets.a.docs]\nindex='manual/a.md'\n[targets.b]\npath='b'\n[targets.b.docs]\nindex='manual/b.md'");
    assert!(docs::resolve(d.path(), &c, &["manual/topic.md".into()])
        .expect_err("invalid fixture")
        .contains("ambiguous"));
    assert!(docs::resolve(d.path(), &c, &["../outside".into()]).is_err());
}
#[test]
fn mapping_schema_rejects_wrong_types_unknown_keys_and_escaping_paths() {
    for text in [
        "[docs]\nindex=1",
        "[docs]\nindex='../out'",
        "[docs]\nindex='/out'",
        "[docs]\nunknown='x'",
        "[targets.a.docs]\nindex='README.md'",
        "[targets.a]\npath='same'\n[targets.b]\npath='same'",
    ] {
        assert!(
            config::parse(&format!("schema_version=1\n{text}")).is_err(),
            "{text}"
        );
    }
}
#[cfg(unix)]
#[test]
fn symlinks_cannot_escape_even_for_missing_plan_leaves() {
    let d = tempfile::tempdir().expect("test fixture");
    let outside = tempfile::tempdir().expect("test fixture");
    write(outside.path(), "README.md", "# Outside");
    std::os::unix::fs::symlink(outside.path(), d.path().join("escape")).expect("test fixture");
    for c in [
        parse("[docs]\nindex='escape/README.md'"),
        parse("[docs]\nplans='escape/not-created'"),
    ] {
        assert!(docs::resolve(d.path(), &c, &[])
            .expect_err("invalid fixture")
            .contains("escapes"));
    }
}
#[test]
fn cli_resolves_before_setup_and_rejects_incomplete_arguments() {
    let d = tempfile::tempdir().expect("test fixture");
    write(d.path(), "README.md", "# Consumer");
    let args = vec![
        "gameskills".into(),
        "--root".into(),
        d.path().as_os_str().into(),
        "docs".into(),
        "resolve".into(),
    ];
    assert_eq!(gameskills_cli::cli::execute(args.clone()).exit_code, 0);
    let mut invalid = args;
    invalid.push("--path".into());
    assert_ne!(gameskills_cli::cli::execute(invalid).exit_code, 0);
}
