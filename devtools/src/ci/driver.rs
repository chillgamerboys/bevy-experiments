//! Executable CI boundary: explicit arguments, GitHub file protocol and child jobs.

use super::{checks, head, select, Job, Selection};
use crate::support;
use clap::Subcommand;
use serde_json::{json, Value};
use std::fs::OpenOptions;
use std::io::Write;
use std::path::Path;
use std::process::{Command, Stdio};

/// CI command arguments shared by the executable and its tests.
#[derive(Subcommand)]
pub enum Operation {
    /// Select jobs using committed inputs, or all checks when comparison is uncertain.
    Select {
        /// Full base commit ID; defaults to the pull-request base or push event before ID.
        #[arg(long)]
        base: Option<String>,
        /// Full tested commit ID; defaults to the current checkout HEAD.
        #[arg(long)]
        head: Option<String>,
        /// Select every check; workflow_dispatch also requests this behavior.
        #[arg(long)]
        full: bool,
    },
    /// Run a selected job from CI_SELECTION, streaming child logs before the JSON result.
    Run {
        /// Selected conditional job.
        #[arg(value_enum)]
        job: Job,
    },
    /// Fail unless classification and every selected job succeeded.
    Gate,
}

/// Decode one complete selection without duplicate keys or silently ignored fields.
pub fn parse_selection(source: &str) -> Result<Selection, String> {
    let selection: Selection = serde_json::from_value(support::parse_json(source)?)
        .map_err(|error| format!("invalid CI selection: {error}"))?;
    checks::validate(&selection)?;
    Ok(selection)
}

fn environment(name: &str) -> Result<Option<String>, String> {
    match std::env::var(name) {
        Ok(value) if value.is_empty() => Ok(None),
        Ok(value) => Ok(Some(value)),
        Err(std::env::VarError::NotPresent) => Ok(None),
        Err(error) => Err(format!("{name}: {error}")),
    }
}

fn required(name: &str) -> Result<String, String> {
    environment(name)?.ok_or_else(|| format!("missing {name}"))
}

fn append(name: &str, content: &str) -> Result<(), String> {
    if let Some(path) = environment(name)? {
        OpenOptions::new()
            .create(true)
            .append(true)
            .open(&path)
            .and_then(|mut file| file.write_all(content.as_bytes()))
            .map_err(|error| format!("cannot append {name}: {error}"))?;
    }
    Ok(())
}

fn event_base(event: &Value) -> Result<Option<String>, String> {
    if !event.is_object() {
        return Err("GitHub event must be an object".into());
    }
    // Reject malformed enclosing fields rather than interpreting them as absent.
    let mut current = event;
    for name in ["pull_request", "base"] {
        match current.get(name) {
            None => {
                current = &Value::Null;
                break;
            }
            Some(value) if value.is_object() => current = value,
            Some(_) => return Err(format!("GitHub event {name} must be an object")),
        }
    }
    match current.get("sha").or_else(|| event.get("before")) {
        Some(Value::String(value)) => Ok(Some(value.clone())),
        Some(Value::Null) | None => Ok(None),
        Some(_) => Err("GitHub base commit must be a string".into()),
    }
}

fn run_child(root: &Path, argv: &[String]) -> Result<(), String> {
    let (program, arguments) = argv.split_first().ok_or("empty CI command")?;
    writeln!(std::io::stderr().lock(), "+ {argv:?}").map_err(|error| error.to_string())?;
    let status = Command::new(program)
        .args(arguments)
        .current_dir(root)
        .stdin(Stdio::null())
        .status()
        .map_err(|error| format!("cannot run {program}: {error}"))?;
    if status.success() {
        Ok(())
    } else {
        Err(format!("CI command {argv:?} failed: {status}"))
    }
}

/// Execute one CI command. GitHub writes and child failures are propagated to the caller.
pub fn execute(root: &Path, operation: Operation) -> Result<Value, String> {
    match operation {
        Operation::Select {
            base,
            head: requested_head,
            full,
        } => {
            let event = environment("GITHUB_EVENT_PATH")?
                .map(|path| support::read_json(Path::new(&path)))
                .transpose()?
                .unwrap_or_else(|| json!({}));
            let base = base.or(event_base(&event)?);
            let head = requested_head.map_or_else(|| head(root), Ok)?;
            let selection = select(
                root,
                base.as_deref(),
                &head,
                full || environment("GITHUB_EVENT_NAME")?.as_deref() == Some("workflow_dispatch"),
            )?;
            checks::validate(&selection)?;
            let encoded = serde_json::to_string(&selection).map_err(|error| error.to_string())?;
            let mut outputs = format!("selection={encoded}\n");
            for job in [Job::Skills, Job::Rust, Job::Policy] {
                outputs.push_str(&format!("{}={}\n", job.name(), selection.selected(job)));
            }
            append("GITHUB_OUTPUT", &outputs)?;
            let pretty =
                serde_json::to_string_pretty(&selection).map_err(|error| error.to_string())?;
            append(
                "GITHUB_STEP_SUMMARY",
                &format!("### Selected CI\n\n```json\n{pretty}\n```\n"),
            )?;
            serde_json::to_value(selection).map_err(|error| error.to_string())
        }
        Operation::Gate => {
            let selection = parse_selection(&required("CI_SELECTION")?)?;
            checks::gate(&selection, &support::parse_json(&required("CI_NEEDS")?)?)?;
            Ok(
                json!({"schema_version":1,"ok":true,"scope":"ci_gate","message":"All selected CI jobs succeeded; remaining jobs were intentionally skipped."}),
            )
        }
        Operation::Run { job } => {
            let selection = parse_selection(&required("CI_SELECTION")?)?;
            let commands = checks::run_with(root, &selection, job, |argv| run_child(root, argv))?;
            Ok(
                json!({"schema_version":1,"ok":true,"scope":"ci_run","job":job.name(),"commands_completed":commands}),
            )
        }
    }
}
