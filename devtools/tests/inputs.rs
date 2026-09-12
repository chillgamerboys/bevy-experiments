//! Shared input regressions, including malformed data and containment boundaries.

use gamekit_repo_tools::{markdown, support};

#[test]
fn duplicate_json_keys_at_any_depth_and_malformed_numbers_fail() {
    for source in [
        r#"{"x":1,"x":2}"#,
        r#"{"outer":[{"x":1,"x":2}]}"#,
        r#"{"x": NaN}"#,
        r#"{"x":1e999}"#,
        r#"{} trailing"#,
    ] {
        assert!(support::parse_json(source).is_err(), "{source}");
    }
    assert!(support::parse_json(r#"{"a":[1,2.5,true,null,{"x":"text"}]}"#).is_ok());
    let directory = tempfile::tempdir().expect("fixture");
    let path = directory.path().join("input.json");
    std::fs::write(&path, "[]").expect("fixture file");
    assert!(support::read_json(&path).is_err());
}

#[test]
fn matching_fences_references_and_titles() {
    let source = "[first](one.md \"title\") ![image](<two image.png>)\n~~~~rust\n[hidden](absent)\n```\n[still hidden](absent)\n~~~\n[still hidden](absent)\n~~~~\n[reference]: <three file.md> \"title\"\n[reference]\n````\n```\n[hidden](absent)\n````\n[last](last.md)";
    assert_eq!(
        markdown::links(source),
        ["one.md", "two image.png", "last.md", "three file.md"]
    );
}

#[test]
fn encoded_local_paths_and_nonportable_paths() {
    let temporary = tempfile::tempdir().expect("fixture");
    let root = temporary.path();
    let source = root.join("README.md");
    std::fs::write(root.join("café file.md"), "").expect("fixture file");
    assert!(
        support::local_target(root, &source, "caf%C3%A9%20file.md#section")
            .expect("valid encoded link")
            .is_some()
    );
    for target in [
        "HTTPS://example.invalid/file",
        "MailTo:a@example.invalid",
        "#heading",
        "?query",
    ] {
        assert_eq!(support::local_target(root, &source, target), Ok(None));
    }
    for target in [
        "%ZZ",
        "%FF",
        "%2Fetc/passwd",
        "%5Cwindows",
        "a%00b",
        "C:/path",
        "file:///etc/passwd",
        "//example.invalid/path",
        "missing.md",
        "../outside.md",
    ] {
        assert!(
            support::local_target(root, &source, target).is_err(),
            "{target}"
        );
    }
}

#[cfg(unix)]
#[test]
fn symlinks_cannot_escape_link_boundary_or_supply_input_file() {
    let inside = tempfile::tempdir().expect("inside fixture");
    let outside = tempfile::tempdir().expect("outside fixture");
    let sentinel = outside.path().join("private.md");
    std::fs::write(&sentinel, "unchanged").expect("sentinel");
    let link = inside.path().join("link.md");
    std::os::unix::fs::symlink(&sentinel, &link).expect("fixture symlink");
    assert!(support::read_text(&link).is_err());
    assert!(
        support::local_target(inside.path(), &inside.path().join("README.md"), "link.md").is_err()
    );
    assert_eq!(
        std::fs::read_to_string(sentinel).expect("sentinel retained"),
        "unchanged"
    );
}
