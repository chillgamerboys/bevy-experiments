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
