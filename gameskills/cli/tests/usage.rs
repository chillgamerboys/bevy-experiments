//! Task usage receipts preserve observed counters and report unknowns honestly.
#![allow(
    clippy::indexing_slicing,
    reason = "Assertions intentionally fail when the versioned JSON contract omits a required field."
)]

use gameskills_cli::usage;
use serde_json::{json, Value};
use std::{error::Error, ffi::OsString, fs, path::Path};

type Test = Result<(), Box<dyn Error>>;

fn call(root: &Path, args: &[&str]) -> Result<Value, String> {
    usage::execute(
        root,
        &args.iter().map(OsString::from).collect::<Vec<OsString>>(),
    )
}

fn receipt(task: &str, attempt: &str, start: u64, end: u64) -> Value {
    json!({
        "schema_version": 1,
        "task": task,
        "thread": "thread-1",
        "attempt": attempt,
        "client": "codex",
        "role": "worker",
        "requested": {"model":"requested-model","effort":"high"},
        "observed": {"model":"observed-model","effort":"medium"},
        "start": {
            "at": start,
            "input_tokens": 100,
            "cached_input_tokens": 20,
            "output_tokens": 10,
            "reasoning_output_tokens": 2
        },
        "end": {
            "at": end,
            "input_tokens": 1100,
            "cached_input_tokens": 220,
            "output_tokens": 110,
            "reasoning_output_tokens": 22
        },
        "evidence_reference": "host:receipt-1"
    })
}

fn write_json(root: &Path, name: &str, value: &Value) -> Result<String, Box<dyn Error>> {
    let path = root.join(name);
    fs::write(&path, serde_json::to_vec(value)?)?;
    Ok(path.to_string_lossy().into_owned())
}

#[test]
fn duplicate_import_is_idempotent_but_conflicts_and_thread_overlap_are_rejected() -> Test {
    let directory = tempfile::tempdir()?;
    let root = directory.path();
    let path = write_json(root, "first.json", &receipt("task", "attempt-1", 10, 20))?;
    assert_eq!(call(root, &["import", "--file", &path])?["imported"], true);
    let duplicate = call(root, &["import", "--file", &path])?;
    assert_eq!(duplicate["imported"], false);
    assert_eq!(duplicate["duplicate"], true);

    let mut conflict = receipt("task", "attempt-1", 10, 20);
    conflict["evidence_reference"] = json!("host:different");
    let path = write_json(root, "conflict.json", &conflict)?;
    assert!(call(root, &["import", "--file", &path])
        .expect_err("same identity must conflict")
        .contains("conflicting"));

    let path = write_json(root, "overlap.json", &receipt("task", "attempt-2", 19, 30))?;
    assert!(call(root, &["import", "--file", &path])
        .expect_err("same thread ranges must not overlap")
        .contains("overlaps"));
    let report = call(root, &["report", "task"])?;
    assert_eq!(report["total"]["receipt_count"], 1);
    Ok(())
}

#[test]
fn decreasing_and_invalid_subset_counters_are_rejected() -> Test {
    let directory = tempfile::tempdir()?;
    let root = directory.path();
    let mut decreasing = receipt("task", "attempt-1", 10, 20);
    decreasing["end"]["input_tokens"] = json!(99);
    let path = write_json(root, "decreasing.json", &decreasing)?;
    assert!(call(root, &["import", "--file", &path])
        .expect_err("decrease must fail")
        .contains("decreased"));

    for (name, pointer, value, expected) in [
        (
            "cached.json",
            "/start/cached_input_tokens",
            101,
            "cached_input_tokens",
        ),
        (
            "reasoning.json",
            "/end/reasoning_output_tokens",
            111,
            "reasoning_output_tokens",
        ),
    ] {
        let mut invalid = receipt("task", name, 10, 20);
        *invalid.pointer_mut(pointer).ok_or("fixture pointer")? = json!(value);
        let path = write_json(root, name, &invalid)?;
        assert!(call(root, &["import", "--file", &path])
            .expect_err("subset must fail")
            .contains(expected));
    }
    Ok(())
}

#[test]
fn missing_counters_remain_visible_and_are_never_treated_as_zero_cost() -> Test {
    let directory = tempfile::tempdir()?;
    let root = directory.path();
    let mut partial = receipt("partial", "attempt-1", 10, 20);
    partial["start"]
        .as_object_mut()
        .ok_or("fixture")?
        .remove("cached_input_tokens");
    partial["end"]
        .as_object_mut()
        .ok_or("fixture")?
        .remove("cached_input_tokens");
    partial["start"]
        .as_object_mut()
        .ok_or("fixture")?
        .remove("reasoning_output_tokens");
    partial["end"]
        .as_object_mut()
        .ok_or("fixture")?
        .remove("reasoning_output_tokens");
    let path = write_json(root, "partial.json", &partial)?;
    call(root, &["import", "--file", &path])?;
    let report = call(root, &["report", "partial"])?;
    assert_eq!(report["total"]["totals"]["input_tokens"], 1000);
    assert!(report["total"]["totals"]["cached_input_tokens"].is_null());
    assert_eq!(report["total"]["complete"]["cached_input_tokens"], false);
    assert_eq!(report["cost_estimate"]["available"], false);
    assert_eq!(report["cost_estimate"]["reason"], "rates_not_supplied");
    Ok(())
}

#[test]
fn known_rates_price_uncached_cached_and_output_using_the_observed_model() -> Test {
    let directory = tempfile::tempdir()?;
    let root = directory.path();
    let receipt_path = write_json(root, "receipt.json", &receipt("priced", "a", 10, 20))?;
    call(root, &["import", "--file", &receipt_path])?;
    let rates = json!({
        "schema_version": 1,
        "as_of": "2026-09-14",
        "source": "rate-card:test",
        "currency": "USD",
        "rates": [{
            "client": "codex",
            "model": "observed-model",
            "uncached_input_per_million": 2.0,
            "cached_input_per_million": 1.0,
            "output_per_million": 10.0
        }]
    });
    let rates_path = write_json(root, "rates.json", &rates)?;
    let report = call(root, &["report", "priced", "--rates", &rates_path])?;
    assert_eq!(report["cost_estimate"]["available"], true);
    let amount = report["cost_estimate"]["amount"]
        .as_f64()
        .ok_or("cost amount")?;
    assert!((amount - 0.0028).abs() < 1e-12, "{amount}");
    assert_eq!(report["cost_estimate"]["currency"], "USD");
    Ok(())
}

fn native_log(path: &Path, input: u64, cached: u64, output: u64, secret: &str) -> Test {
    let lines = [
        json!({"type":"session_meta","payload":{"id":"thread-native","client":"codex"}}),
        json!({"type":"turn_context","payload":{"model":"native-model","effort":"high"}}),
        json!({"type":"event_msg","payload":{"type":"agent_message","message":secret}}),
        json!({"type":"event_msg","payload":{"type":"token_count","info":{"total_token_usage":{
            "input_tokens":input,
            "cached_input_tokens":cached,
            "output_tokens":output,
            "reasoning_output_tokens":output / 2
        }}}}),
    ];
    let body = lines
        .iter()
        .map(Value::to_string)
        .collect::<Vec<_>>()
        .join("\n")
        + "\n";
    fs::write(path, body)?;
    Ok(())
}

#[test]
fn native_checkpoint_pair_records_one_delta_without_disclosing_transcript_text() -> Test {
    let directory = tempfile::tempdir()?;
    let root = directory.path();
    let log = root.join("session.jsonl");
    let secret = "PRIVATE TRANSCRIPT CONTENT MUST NOT ESCAPE";
    native_log(&log, 100, 20, 40, secret)?;
    let log_text = log.to_string_lossy();
    let start = call(
        root,
        &[
            "checkpoint",
            "native-task",
            "--log",
            &log_text,
            "--phase",
            "start",
            "--role",
            "worker",
            "--attempt",
            "attempt-1",
        ],
    )?;
    assert_eq!(start["recorded"], true);
    assert!(!start.to_string().contains(secret));

    native_log(&log, 500, 120, 140, secret)?;
    let end = call(
        root,
        &[
            "checkpoint",
            "native-task",
            "--log",
            &log_text,
            "--phase",
            "end",
            "--role",
            "worker",
            "--attempt",
            "attempt-1",
        ],
    )?;
    assert_eq!(end["imported"], true);
    assert!(!end.to_string().contains(secret));
    let duplicate = call(
        root,
        &[
            "checkpoint",
            "native-task",
            "--log",
            &log_text,
            "--phase",
            "end",
            "--role",
            "worker",
            "--attempt",
            "attempt-1",
        ],
    )?;
    assert_eq!(duplicate["duplicate"], true);

    let report = call(root, &["report", "native-task"])?;
    assert_eq!(report["total"]["totals"]["input_tokens"], 400);
    assert_eq!(report["total"]["totals"]["cached_input_tokens"], 100);
    assert_eq!(report["total"]["totals"]["output_tokens"], 100);
    assert!(!report.to_string().contains(secret));
    Ok(())
}

#[test]
fn native_checkpoint_refuses_missing_identity_or_counters_instead_of_fabricating() -> Test {
    let directory = tempfile::tempdir()?;
    let root = directory.path();
    let log = root.join("empty.jsonl");
    fs::write(
        &log,
        json!({"type":"session_meta","payload":{"id":"thread-native","client":"codex"}})
            .to_string()
            + "\n",
    )?;
    let error = call(
        root,
        &[
            "checkpoint",
            "task",
            "--log",
            &log.to_string_lossy(),
            "--phase",
            "start",
            "--role",
            "coordinator",
        ],
    )
    .expect_err("missing counters must fail");
    assert!(error.contains("counters are unavailable"));
    assert!(!root
        .join(".gameskills/usage/checkpoints")
        .read_dir()?
        .any(|entry| {
            entry
                .ok()
                .is_some_and(|entry| entry.file_name().to_string_lossy().ends_with(".json"))
        }));
    Ok(())
}

#[cfg(unix)]
#[test]
fn protected_usage_state_refuses_symlink_redirection() -> Test {
    let directory = tempfile::tempdir()?;
    let outside = tempfile::tempdir()?;
    fs::create_dir(directory.path().join(".gameskills"))?;
    std::os::unix::fs::symlink(outside.path(), directory.path().join(".gameskills/usage"))?;
    let path = write_json(
        directory.path(),
        "receipt.json",
        &receipt("task", "a", 1, 2),
    )?;
    assert!(call(directory.path(), &["import", "--file", &path]).is_err());
    assert!(outside.path().read_dir()?.next().is_none());
    Ok(())
}
