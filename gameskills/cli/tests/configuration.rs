//! Language-neutral configuration contracts, exercised without an interpreter.

use serde_json::Value;
use std::error::Error;

#[test]
fn configuration_contract_fixtures() -> Result<(), Box<dyn Error>> {
    let fixtures: Value = serde_json::from_str(include_str!("fixtures/configuration.json"))?;
    let cases = fixtures
        .get("cases")
        .and_then(Value::as_array)
        .ok_or("fixture cases missing")?;
    for case in cases {
        let id = case
            .get("id")
            .and_then(Value::as_str)
            .ok_or("fixture id missing")?;
        let input = case
            .get("input")
            .and_then(Value::as_str)
            .ok_or("fixture input missing")?;
        let expected = case.get("expected").ok_or("expected missing")?;
        let actual = gameskills_cli::config::parse(input);
        assert_eq!(
            actual.is_ok(),
            expected
                .get("ok")
                .and_then(Value::as_bool)
                .ok_or("ok missing")?,
            "{id}: {actual:?}"
        );
        if let Ok(configuration) = actual {
            assert_eq!(
                serde_json::to_value(configuration)?,
                *expected
                    .get("configuration")
                    .ok_or("configuration missing")?,
                "{id}"
            );
        }
    }
    Ok(())
}

#[test]
fn git_ref_selection_validates_without_git_or_setup() {
    for refs in [
        r#""all""#,
        "[]",
        r#"["refs/heads/main", "refs/remotes/origin/main", "refs/tags/v1"]"#,
    ] {
        let source =
            format!("schema_version = 1\n[commands.check]\nargv = [\"true\"]\ngit_refs = {refs}\n");
        assert!(gameskills_cli::config::parse(&source).is_ok(), "{refs}");
    }
    for refs in [
        r#""head""#,
        "1",
        "{}",
        "[1]",
        r#"["HEAD"]"#,
        r#"["main"]"#,
        r#"["refs/"]"#,
        r#"["refs/heads/main", "refs/heads/main"]"#,
        r#"["refs/heads/*.rs"]"#,
        r#"["refs/heads/a..b"]"#,
        r#"["refs/heads/a@{b"]"#,
        r#"["refs/heads/.hidden"]"#,
        r#"["refs/heads/locked.lock"]"#,
        r#"["refs/heads/a."]"#,
        r#"["refs//main"]"#,
    ] {
        let source =
            format!("schema_version = 1\n[commands.check]\nargv = [\"true\"]\ngit_refs = {refs}\n");
        let error = gameskills_cli::config::parse(&source).expect_err(refs);
        assert!(error.contains("git_refs"), "{refs}: {error}");
    }
}
