//! Imported and native task usage observations.
//!
//! Usage is measured only from caller receipts or native client counters. Missing
//! telemetry remains missing; this module never estimates token counts.

use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::ffi::OsString;
use std::path::{Path, PathBuf};

const MAX_INPUT_BYTES: u64 = 4 * 1024 * 1024;
const MAX_SESSION_META_BYTES: u64 = 1024 * 1024;

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
struct CounterSnapshot {
    at: u64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    input_tokens: Option<u64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    cached_input_tokens: Option<u64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    output_tokens: Option<u64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    reasoning_output_tokens: Option<u64>,
}

#[derive(Clone, Debug, Default, Deserialize, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
struct Selection {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    model: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    effort: Option<String>,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
struct Receipt {
    schema_version: u64,
    task: String,
    thread: String,
    attempt: String,
    client: String,
    role: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    stage: Option<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    active_skills: Vec<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    segment: Option<u64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    requested: Option<Selection>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    observed: Option<Selection>,
    start: CounterSnapshot,
    end: CounterSnapshot,
    evidence_reference: String,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct TaskRecord {
    schema_version: u64,
    task: String,
    receipts: Vec<Receipt>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct Checkpoint {
    schema_version: u64,
    task: String,
    thread: String,
    attempt: String,
    client: String,
    role: String,
    observed: Selection,
    snapshot: CounterSnapshot,
    evidence_reference: String,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
struct ActiveInterval {
    stage: String,
    active_skills: Vec<String>,
    segment: u64,
    observed: Selection,
    snapshot: CounterSnapshot,
    evidence_reference: String,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct MarkRecord {
    schema_version: u64,
    task: String,
    thread: String,
    attempt: String,
    client: String,
    role: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    active: Option<ActiveInterval>,
    receipts: Vec<Receipt>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct Rates {
    schema_version: u64,
    as_of: String,
    source: String,
    currency: String,
    rates: Vec<Rate>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct Rate {
    client: String,
    model: String,
    uncached_input_per_million: f64,
    cached_input_per_million: f64,
    output_per_million: f64,
}

#[derive(Clone, Copy, Default)]
struct Delta {
    input: Option<u64>,
    cached: Option<u64>,
    output: Option<u64>,
    reasoning: Option<u64>,
}

/// Execute a usage ledger subcommand.
///
/// `import` and `checkpoint` preserve the receipt-oriented compatibility
/// contract. `mark` records native stage transitions, and `report` summarizes
/// both forms without reading transcript content.
pub fn execute(root: &Path, args: &[OsString]) -> Result<Value, String> {
    #[cfg(unix)]
    {
        execute_unix(root, args)
    }
    #[cfg(not(unix))]
    {
        let _ = (root, args);
        Err("usage state is unsupported on this platform: protected atomic state requires the Unix backend".into())
    }
}

#[cfg(unix)]
fn execute_unix(root: &Path, args: &[OsString]) -> Result<Value, String> {
    let (command, rest) = args
        .split_first()
        .ok_or("usage requires import, report, checkpoint, mark, or finish")?;
    match text(command)? {
        "import" => import(root, rest),
        "report" => report(root, rest),
        "checkpoint" => checkpoint(root, rest),
        "mark" => mark(root, rest),
        "finish" => finish(root, rest),
        _ => Err("usage requires import, report, checkpoint, mark, or finish".into()),
    }
}

fn text(value: &OsString) -> Result<&str, String> {
    value
        .to_str()
        .ok_or_else(|| "usage arguments must be UTF-8".into())
}

fn identifier(kind: &str, value: &str) -> Result<(), String> {
    if value.is_empty()
        || value.len() > 128
        || !value
            .bytes()
            .next()
            .is_some_and(|b| b.is_ascii_alphanumeric())
        || !value
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || matches!(b, b'.' | b'_' | b'-'))
    {
        return Err(format!(
            "invalid {kind}: use 1-128 ASCII letters, digits, '.', '_' or '-', starting with a letter or digit"
        ));
    }
    Ok(())
}

fn validate_selection(kind: &str, selection: &Selection) -> Result<(), String> {
    for (field, value) in [
        ("model", selection.model.as_deref()),
        ("effort", selection.effort.as_deref()),
    ] {
        if value.is_some_and(|value| value.is_empty() || value.len() > 256 || value.contains('\0'))
        {
            return Err(format!("invalid {kind} {field}"));
        }
    }
    Ok(())
}

fn validate_stage(stage: &str) -> Result<(), String> {
    if matches!(stage, "implementation" | "verification" | "delivery") {
        Ok(())
    } else {
        Err("usage stage must be implementation, verification, or delivery".into())
    }
}

fn validate_skills(skills: &[String]) -> Result<(), String> {
    let mut previous: Option<&str> = None;
    for skill in skills {
        if skill.is_empty()
            || skill.len() > 256
            || !skill
                .bytes()
                .next()
                .is_some_and(|byte| byte.is_ascii_alphanumeric())
            || !skill.bytes().all(|byte| {
                byte.is_ascii_alphanumeric() || matches!(byte, b'.' | b'_' | b'-' | b':')
            })
        {
            return Err("invalid usage skill identifier".into());
        }
        if previous.is_some_and(|value| value >= skill.as_str()) {
            return Err("usage active_skills must be a sorted set".into());
        }
        previous = Some(skill);
    }
    Ok(())
}

fn validate_snapshot(label: &str, snapshot: &CounterSnapshot) -> Result<(), String> {
    if let (Some(cached), Some(input)) = (snapshot.cached_input_tokens, snapshot.input_tokens) {
        if cached > input {
            return Err(format!(
                "{label} cached_input_tokens must not exceed input_tokens"
            ));
        }
    }
    if let (Some(reasoning), Some(output)) =
        (snapshot.reasoning_output_tokens, snapshot.output_tokens)
    {
        if reasoning > output {
            return Err(format!(
                "{label} reasoning_output_tokens must not exceed output_tokens"
            ));
        }
    }
    Ok(())
}

fn difference(name: &str, start: Option<u64>, end: Option<u64>) -> Result<Option<u64>, String> {
    match (start, end) {
        (Some(start), Some(end)) if end >= start => Ok(Some(end - start)),
        (Some(_), Some(_)) => Err(format!("{name} decreased between start and end")),
        _ => Ok(None),
    }
}

fn validate_receipt(receipt: &Receipt) -> Result<Delta, String> {
    if receipt.schema_version != 1 {
        return Err("usage receipt schema_version must be 1".into());
    }
    for (kind, value) in [
        ("task", receipt.task.as_str()),
        ("thread", receipt.thread.as_str()),
        ("attempt", receipt.attempt.as_str()),
        ("client", receipt.client.as_str()),
    ] {
        identifier(kind, value)?;
    }
    if !matches!(receipt.role.as_str(), "coordinator" | "worker") {
        return Err("usage role must be coordinator or worker".into());
    }
    if let Some(stage) = &receipt.stage {
        validate_stage(stage)?;
    }
    validate_skills(&receipt.active_skills)?;
    if receipt.evidence_reference.is_empty()
        || receipt.evidence_reference.len() > 4096
        || receipt.evidence_reference.contains('\0')
    {
        return Err("invalid evidence_reference".into());
    }
    if let Some(selection) = &receipt.requested {
        validate_selection("requested", selection)?;
    }
    if let Some(selection) = &receipt.observed {
        validate_selection("observed", selection)?;
    }
    if receipt.end.at < receipt.start.at {
        return Err("usage end timestamp precedes start timestamp".into());
    }
    validate_snapshot("start", &receipt.start)?;
    validate_snapshot("end", &receipt.end)?;
    let delta = Delta {
        input: difference(
            "input_tokens",
            receipt.start.input_tokens,
            receipt.end.input_tokens,
        )?,
        cached: difference(
            "cached_input_tokens",
            receipt.start.cached_input_tokens,
            receipt.end.cached_input_tokens,
        )?,
        output: difference(
            "output_tokens",
            receipt.start.output_tokens,
            receipt.end.output_tokens,
        )?,
        reasoning: difference(
            "reasoning_output_tokens",
            receipt.start.reasoning_output_tokens,
            receipt.end.reasoning_output_tokens,
        )?,
    };
    if let (Some(cached), Some(input)) = (delta.cached, delta.input) {
        if cached > input {
            return Err("cached_input_tokens delta must not exceed input_tokens delta".into());
        }
    }
    if let (Some(reasoning), Some(output)) = (delta.reasoning, delta.output) {
        if reasoning > output {
            return Err("reasoning_output_tokens delta must not exceed output_tokens delta".into());
        }
    }
    Ok(delta)
}

fn read_bounded(path: &Path, limit: u64, kind: &str) -> Result<Vec<u8>, String> {
    use std::io::Read;
    let metadata = std::fs::symlink_metadata(path)
        .map_err(|error| format!("cannot inspect {kind} {}: {error}", path.display()))?;
    if !metadata.file_type().is_file() || metadata.file_type().is_symlink() {
        return Err(format!("{kind} must be an ordinary file"));
    }
    if metadata.len() > limit {
        return Err(format!("{kind} exceeds {limit} bytes"));
    }
    let mut bytes = Vec::with_capacity(usize::try_from(metadata.len()).unwrap_or(0));
    std::fs::File::open(path)
        .and_then(|mut file| file.read_to_end(&mut bytes))
        .map_err(|error| format!("cannot read {kind} {}: {error}", path.display()))?;
    if u64::try_from(bytes.len()).unwrap_or(u64::MAX) > limit {
        return Err(format!(
            "{kind} changed while reading and exceeds {limit} bytes"
        ));
    }
    Ok(bytes)
}

#[cfg(unix)]
struct State {
    _lock: std::fs::File,
    tasks: crate::runner::state::Directory,
    checkpoints: crate::runner::state::Directory,
    marks: crate::runner::state::Directory,
}

#[cfg(unix)]
fn state(root: &Path) -> Result<State, String> {
    use crate::runner::state::{lock, Directory};
    let root = Directory::root(root)?;
    let metadata = root.child(".gameskills", true, false)?;
    let usage = metadata.child("usage", true, false)?;
    let lock_file = usage.open(".lock", true, false)?;
    if !lock(&lock_file)? {
        return Err("another usage operation is active".into());
    }
    Ok(State {
        _lock: lock_file,
        tasks: usage.child("tasks", true, false)?,
        checkpoints: usage.child("checkpoints", true, false)?,
        marks: usage.child("marks", true, false)?,
    })
}

fn task_file(task: &str) -> String {
    format!("{task}.json")
}

#[cfg(unix)]
fn load_mark(state: &State, file: &str) -> Result<MarkRecord, String> {
    let record: MarkRecord = serde_json::from_slice(&state.marks.read(file)?)
        .map_err(|error| format!("invalid usage mark state {file}: {error}"))?;
    if record.schema_version != 1 {
        return Err(format!("invalid usage mark state schema in {file}"));
    }
    for value in [
        ("task", record.task.as_str()),
        ("thread", record.thread.as_str()),
        ("attempt", record.attempt.as_str()),
        ("client", record.client.as_str()),
    ] {
        identifier(value.0, value.1)?;
    }
    if !matches!(record.role.as_str(), "coordinator" | "worker") {
        return Err(format!("invalid usage mark role in {file}"));
    }
    for receipt in &record.receipts {
        validate_receipt(receipt)?;
        if receipt.task != record.task
            || receipt.thread != record.thread
            || receipt.attempt != record.attempt
            || receipt.client != record.client
            || receipt.role != record.role
        {
            return Err(format!("invalid usage mark receipt identity in {file}"));
        }
    }
    if let Some(active) = &record.active {
        validate_stage(&active.stage)?;
        validate_skills(&active.active_skills)?;
        validate_selection("active observed", &active.observed)?;
        validate_snapshot("active", &active.snapshot)?;
    }
    Ok(record)
}

#[cfg(unix)]
fn mark_receipts(state: &State) -> Result<Vec<Receipt>, String> {
    let mut receipts = Vec::new();
    for file in state.marks.entries()? {
        receipts.extend(load_mark(state, &file)?.receipts);
    }
    Ok(receipts)
}

#[cfg(unix)]
fn open_intervals(state: &State, task: &str) -> Result<Vec<Value>, String> {
    let mut intervals = Vec::new();
    for file in state.marks.entries()? {
        let record = load_mark(state, &file)?;
        if record.task != task {
            continue;
        }
        if let Some(active) = record.active {
            intervals.push((active.stage, record.thread, record.attempt));
        }
    }
    intervals.sort();
    Ok(intervals
        .into_iter()
        .map(|(stage, thread, attempt)| {
            json!({
                "stage": stage,
                "thread": thread,
                "attempt": attempt,
            })
        })
        .collect())
}

#[cfg(unix)]
fn load_task(state: &State, task: &str) -> Result<TaskRecord, String> {
    let file = task_file(task);
    if !state.tasks.entries()?.iter().any(|entry| entry == &file) {
        return Ok(TaskRecord {
            schema_version: 1,
            task: task.into(),
            receipts: Vec::new(),
        });
    }
    let record: TaskRecord = serde_json::from_slice(&state.tasks.read(&file)?)
        .map_err(|error| format!("invalid usage state for {task}: {error}"))?;
    if record.schema_version != 1 || record.task != task {
        return Err(format!("invalid usage state identity for {task}"));
    }
    for receipt in &record.receipts {
        validate_receipt(receipt)?;
        if receipt.task != task {
            return Err("usage state contains a receipt for another task".into());
        }
    }
    Ok(record)
}

fn same_identity(left: &Receipt, right: &Receipt) -> bool {
    left.task == right.task
        && left.thread == right.thread
        && left.attempt == right.attempt
        && left.client == right.client
        && left.segment == right.segment
}

fn overlaps(left: &Receipt, right: &Receipt) -> bool {
    fn counters(
        left_start: Option<u64>,
        left_end: Option<u64>,
        right_start: Option<u64>,
        right_end: Option<u64>,
    ) -> bool {
        matches!(
            (left_start, left_end, right_start, right_end),
            (Some(ls), Some(le), Some(rs), Some(re)) if ls < re && rs < le
        )
    }
    left.client == right.client
        && left.thread == right.thread
        && (left.start.at < right.end.at && right.start.at < left.end.at
            || counters(
                left.start.input_tokens,
                left.end.input_tokens,
                right.start.input_tokens,
                right.end.input_tokens,
            )
            || counters(
                left.start.output_tokens,
                left.end.output_tokens,
                right.start.output_tokens,
                right.end.output_tokens,
            ))
}

fn reject_overlap(existing: &Receipt, receipt: &Receipt) -> Result<(), String> {
    if overlaps(existing, receipt) {
        return Err(format!(
            "usage receipt overlaps an existing time or cumulative-token range for client/thread {}/{} (existing task {})",
            receipt.client, receipt.thread, existing.task
        ));
    }
    Ok(())
}

#[cfg(unix)]
fn add_receipt(state: &State, receipt: Receipt) -> Result<bool, String> {
    validate_receipt(&receipt)?;
    let mut record = load_task(state, &receipt.task)?;
    for existing in &record.receipts {
        if same_identity(existing, &receipt) {
            if existing == &receipt {
                return Ok(false);
            }
            return Err(
                "conflicting usage receipt for the same task/thread/attempt/client identity".into(),
            );
        }
        reject_overlap(existing, &receipt)?;
    }
    for entry in state.tasks.entries()? {
        let Some(other_task) = entry.strip_suffix(".json") else {
            continue;
        };
        if other_task == receipt.task {
            continue;
        }
        identifier("stored task", other_task)?;
        for existing in load_task(state, other_task)?.receipts {
            reject_overlap(&existing, &receipt)?;
        }
    }
    for existing in mark_receipts(state)? {
        if same_identity(&existing, &receipt) {
            if existing == receipt {
                return Ok(false);
            }
            return Err(
                "conflicting usage receipt for the same task/thread/attempt/client identity".into(),
            );
        }
        reject_overlap(&existing, &receipt)?;
    }
    record.receipts.push(receipt);
    record.receipts.sort_by(|left, right| {
        (left.start.at, &left.thread, &left.attempt).cmp(&(
            right.start.at,
            &right.thread,
            &right.attempt,
        ))
    });
    state.tasks.write_json(&task_file(&record.task), &record)?;
    Ok(true)
}

#[cfg(unix)]
fn import(root: &Path, args: &[OsString]) -> Result<Value, String> {
    let path = caller_path(root, one_option(args, "--file")?);
    let bytes = read_bounded(&path, MAX_INPUT_BYTES, "usage receipt")?;
    let receipt: Receipt = serde_json::from_slice(&bytes)
        .map_err(|error| format!("invalid usage receipt: {error}"))?;
    validate_receipt(&receipt)?;
    let task = receipt.task.clone();
    let state = state(root)?;
    let imported = add_receipt(&state, receipt)?;
    Ok(json!({
        "schema_version": 1,
        "task": task,
        "imported": imported,
        "duplicate": !imported,
        "claim": "caller-supplied cumulative usage receipt; raw evidence is not independently authenticated"
    }))
}

fn one_option(args: &[OsString], option: &str) -> Result<PathBuf, String> {
    let [provided, path] = args else {
        return Err(format!("expected {option} FILE"));
    };
    if text(provided)? != option {
        return Err(format!("expected {option} FILE"));
    }
    Ok(PathBuf::from(path))
}

fn parsed_report_args(args: &[OsString]) -> Result<(&str, Option<PathBuf>, bool), String> {
    let task = args.first().ok_or("usage report requires TASK")?;
    let task = text(task)?;
    identifier("task", task)?;
    let mut rates = None;
    let mut details = false;
    let mut index = 1;
    while index < args.len() {
        match text(args.get(index).ok_or("missing report option")?)? {
            "--rates" if rates.is_none() => {
                rates = Some(PathBuf::from(
                    args.get(index + 1).ok_or("--rates requires FILE")?,
                ));
                index += 2;
            }
            "--details" if !details => {
                details = true;
                index += 1;
            }
            option => return Err(format!("unknown or repeated report option {option}")),
        }
    }
    Ok((task, rates, details))
}

fn caller_path(root: &Path, path: PathBuf) -> PathBuf {
    if path.is_absolute() {
        path
    } else {
        root.join(path)
    }
}

fn value_or_null(value: Option<u64>) -> Value {
    value.map_or(Value::Null, Value::from)
}

fn summarize<'a>(receipts: impl Iterator<Item = &'a Receipt>) -> Result<Value, String> {
    let receipts: Vec<&Receipt> = receipts.collect();
    let deltas: Vec<Delta> = receipts
        .iter()
        .map(|receipt| validate_receipt(receipt))
        .collect::<Result<_, _>>()?;
    let field = |read: fn(&Delta) -> Option<u64>| {
        let known: Vec<u64> = deltas.iter().filter_map(read).collect();
        let complete = !deltas.is_empty() && known.len() == deltas.len();
        let total = if known.is_empty() {
            None
        } else {
            Some(
                known
                    .into_iter()
                    .try_fold(0_u64, u64::checked_add)
                    .ok_or("usage token total exceeds u64")?,
            )
        };
        Ok::<_, String>((total, complete))
    };
    let (input, input_complete) = field(|d| d.input)?;
    let (cached, cached_complete) = field(|d| d.cached)?;
    let (output, output_complete) = field(|d| d.output)?;
    let (reasoning, reasoning_complete) = field(|d| d.reasoning)?;
    let mut incomplete = Vec::new();
    for (name, complete) in [
        ("input_tokens", input_complete),
        ("cached_input_tokens", cached_complete),
        ("output_tokens", output_complete),
        ("reasoning_output_tokens", reasoning_complete),
    ] {
        if !complete {
            incomplete.push(name);
        }
    }
    Ok(json!({
        "receipt_count": receipts.len(),
        "totals": {
            "input_tokens": value_or_null(input),
            "cached_input_tokens": value_or_null(cached),
            "output_tokens": value_or_null(output),
            "reasoning_output_tokens": value_or_null(reasoning),
        },
        "complete": {
            "input_tokens": input_complete,
            "cached_input_tokens": cached_complete,
            "output_tokens": output_complete,
            "reasoning_output_tokens": reasoning_complete,
        },
        "incomplete_fields": incomplete,
    }))
}

fn model_summary(receipts: &[Receipt], requested: bool) -> Value {
    let mut models = std::collections::BTreeSet::new();
    let mut unavailable = 0_u64;
    for receipt in receipts {
        let selection = if requested {
            receipt.requested.as_ref()
        } else {
            receipt.observed.as_ref()
        };
        if let Some(model) = selection.and_then(|selection| selection.model.as_deref()) {
            models.insert(model);
        } else {
            unavailable += 1;
        }
    }
    json!({"models":models,"unavailable_receipts":unavailable})
}

fn classified_summaries(receipts: &[Receipt]) -> Result<(Value, Value), String> {
    let mut stages: std::collections::BTreeMap<String, Vec<&Receipt>> =
        std::collections::BTreeMap::new();
    let mut skill_sets: std::collections::BTreeMap<(bool, Vec<String>), Vec<&Receipt>> =
        std::collections::BTreeMap::new();
    for receipt in receipts {
        stages
            .entry(receipt.stage.as_deref().unwrap_or("unclassified").into())
            .or_default()
            .push(receipt);
        skill_sets
            .entry((receipt.stage.is_some(), receipt.active_skills.clone()))
            .or_default()
            .push(receipt);
    }
    let stages = stages
        .into_iter()
        .map(|(stage, members)| summarize(members.into_iter()).map(|summary| (stage, summary)))
        .collect::<Result<std::collections::BTreeMap<_, _>, _>>()?;
    let skill_sets = skill_sets
        .into_iter()
        .map(|((classified, skills), members)| {
            let attribution = if !classified {
                "unclassified"
            } else if skills.is_empty() {
                "empty_active_set"
            } else if skills.len() == 1 {
                "active_set"
            } else {
                "mixed_active_set"
            };
            Ok(json!({
                "active_skills": skills,
                "attribution": attribution,
                "usage": summarize(members.into_iter())?,
            }))
        })
        .collect::<Result<Vec<_>, String>>()?;
    Ok((json!(stages), json!(skill_sets)))
}

fn parse_rates(path: &Path) -> Result<Rates, String> {
    let bytes = read_bounded(path, MAX_INPUT_BYTES, "usage rates")?;
    let rates: Rates =
        serde_json::from_slice(&bytes).map_err(|error| format!("invalid usage rates: {error}"))?;
    if rates.schema_version != 1 || rates.as_of.is_empty() || rates.source.is_empty() {
        return Err("usage rates require schema_version 1, as_of, and source".into());
    }
    identifier("currency", &rates.currency)?;
    for rate in &rates.rates {
        identifier("rate client", &rate.client)?;
        if rate.model.is_empty() || rate.model.len() > 256 {
            return Err("invalid rate model".into());
        }
        for value in [
            rate.uncached_input_per_million,
            rate.cached_input_per_million,
            rate.output_per_million,
        ] {
            if !value.is_finite() || value < 0.0 {
                return Err("usage rates must be finite nonnegative numbers".into());
            }
        }
    }
    Ok(rates)
}

fn cost(receipts: &[Receipt], rates: Option<&Rates>) -> Result<Value, String> {
    let Some(rates) = rates else {
        return Ok(json!({"available":false,"reason":"rates_not_supplied"}));
    };
    let mut amount = 0.0;
    for receipt in receipts {
        let delta = validate_receipt(receipt)?;
        let Some(model) = receipt
            .observed
            .as_ref()
            .and_then(|selection| selection.model.as_deref())
        else {
            return Ok(
                json!({"available":false,"reason":"observed_model_unavailable","currency":rates.currency,"as_of":rates.as_of,"source":rates.source}),
            );
        };
        let Some(rate) = rates
            .rates
            .iter()
            .find(|rate| rate.client == receipt.client && rate.model == model)
        else {
            return Ok(
                json!({"available":false,"reason":"matching_rate_unavailable","currency":rates.currency,"as_of":rates.as_of,"source":rates.source}),
            );
        };
        let (Some(input), Some(cached), Some(output)) = (delta.input, delta.cached, delta.output)
        else {
            return Ok(
                json!({"available":false,"reason":"required_token_telemetry_unavailable","currency":rates.currency,"as_of":rates.as_of,"source":rates.source}),
            );
        };
        if cached > input {
            return Err("cached input delta exceeds input delta".into());
        }
        amount += ((input - cached) as f64 * rate.uncached_input_per_million
            + cached as f64 * rate.cached_input_per_million
            + output as f64 * rate.output_per_million)
            / 1_000_000.0;
        if !amount.is_finite() {
            return Err("usage cost estimate exceeds finite numeric range".into());
        }
    }
    Ok(json!({
        "available": true,
        "amount": amount,
        "currency": rates.currency,
        "as_of": rates.as_of,
        "source": rates.source,
        "basis": "observed model and imported token deltas"
    }))
}

#[cfg(unix)]
fn report(root: &Path, args: &[OsString]) -> Result<Value, String> {
    let (task, rates_path, details) = parsed_report_args(args)?;
    let rates_path = rates_path.map(|path| caller_path(root, path));
    let rates = rates_path.as_deref().map(parse_rates).transpose()?;
    let state = state(root)?;
    let mut receipts = load_task(&state, task)?.receipts;
    receipts.extend(
        mark_receipts(&state)?
            .into_iter()
            .filter(|receipt| receipt.task == task),
    );
    receipts.sort_by(|left, right| {
        (left.start.at, &left.thread, &left.attempt, left.segment).cmp(&(
            right.start.at,
            &right.thread,
            &right.attempt,
            right.segment,
        ))
    });
    if receipts.is_empty() {
        return Err(format!("no usage receipts recorded for task {task}"));
    }
    let coordinator = summarize(receipts.iter().filter(|r| r.role == "coordinator"))?;
    let workers = summarize(receipts.iter().filter(|r| r.role == "worker"))?;
    let total = summarize(receipts.iter())?;
    let estimate = cost(&receipts, rates.as_ref())?;
    let (stages, active_skill_sets) = classified_summaries(&receipts)?;
    let open_intervals = open_intervals(&state, task)?;
    let first = receipts
        .iter()
        .map(|receipt| receipt.start.at)
        .min()
        .ok_or("usage report has no receipt start")?;
    let last = receipts
        .iter()
        .map(|receipt| receipt.end.at)
        .max()
        .ok_or("usage report has no receipt end")?;
    let thread_seconds = receipts
        .iter()
        .try_fold(0_u64, |total, receipt| {
            total.checked_add(receipt.end.at - receipt.start.at)
        })
        .ok_or("summed usage thread seconds exceeds u64")?;
    let attempts: std::collections::BTreeSet<_> = receipts
        .iter()
        .map(|receipt| (&receipt.client, &receipt.thread, &receipt.attempt))
        .collect();
    let mut result = json!({
        "schema_version": 1,
        "task": task,
        "contributions": {"coordinator": coordinator, "workers": workers},
        "stages": stages,
        "active_skill_sets": active_skill_sets,
        "skill_attribution": "tokens are attributed to the exact active skill set; mixed sets are not split into invented per-skill causation",
        "open_intervals": open_intervals,
        "totals_scope": "completed intervals only; open intervals are excluded",
        "total": total,
        "receipt_count": receipts.len(),
        "attempt_count": attempts.len(),
        "observed_span_seconds": last - first,
        "summed_thread_seconds": thread_seconds,
        "models": {
            "requested": model_summary(&receipts, true),
            "observed": model_summary(&receipts, false)
        },
        "cost_estimate": estimate,
        "claim": "known deltas from cumulative counters; missing telemetry is reported as unavailable"
    });
    if details {
        result
            .as_object_mut()
            .ok_or("usage report must be an object")?
            .insert(
                "receipts".into(),
                serde_json::to_value(receipts).map_err(|error| error.to_string())?,
            );
    }
    Ok(result)
}

#[derive(Default)]
struct CheckpointArgs {
    task: String,
    log: Option<PathBuf>,
    phase: Option<String>,
    role: Option<String>,
    attempt: String,
}

fn checkpoint_args(args: &[OsString]) -> Result<CheckpointArgs, String> {
    let mut parsed = CheckpointArgs {
        task: text(args.first().ok_or("usage checkpoint requires TASK")?)?.into(),
        attempt: "default".into(),
        ..CheckpointArgs::default()
    };
    identifier("task", &parsed.task)?;
    let mut attempt_seen = false;
    let mut index = 1;
    while index < args.len() {
        let option = text(args.get(index).ok_or("missing checkpoint option")?)?;
        let value = args
            .get(index + 1)
            .ok_or_else(|| format!("missing value for {option}"))?;
        match option {
            "--log" if parsed.log.is_none() => parsed.log = Some(PathBuf::from(value)),
            "--phase" if parsed.phase.is_none() => parsed.phase = Some(text(value)?.into()),
            "--role" if parsed.role.is_none() => parsed.role = Some(text(value)?.into()),
            "--attempt" if !attempt_seen => {
                parsed.attempt = text(value)?.into();
                attempt_seen = true;
            }
            _ => return Err(format!("unknown or repeated checkpoint option {option}")),
        }
        index += 2;
    }
    identifier("attempt", &parsed.attempt)?;
    if !matches!(parsed.phase.as_deref(), Some("start" | "end")) {
        return Err("usage checkpoint --phase must be start or end".into());
    }
    if !matches!(parsed.role.as_deref(), Some("coordinator" | "worker")) {
        return Err("usage checkpoint --role must be coordinator or worker".into());
    }
    if parsed.log.is_none() {
        return Err("usage checkpoint requires --log PATH".into());
    }
    Ok(parsed)
}

fn checkpoint_file(task: &str, thread: &str, attempt: &str) -> String {
    use sha2::{Digest, Sha256};
    format!(
        "{:x}.json",
        Sha256::digest(format!("{task}\0{thread}\0{attempt}"))
    )
}

fn unix_now() -> Result<u64, String> {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|duration| duration.as_secs())
        .map_err(|error| format!("system clock precedes Unix epoch: {error}"))
}

#[derive(Debug)]
struct NativeObservation {
    thread: String,
    client: String,
    observed: Selection,
    counters: CounterSnapshot,
}

fn interval_selection(start: &Selection, end: &Selection) -> Option<Selection> {
    let selection = Selection {
        model: if start.model.is_some() && start.model == end.model {
            start.model.clone()
        } else {
            None
        },
        effort: if start.effort.is_some() && start.effort == end.effort {
            start.effort.clone()
        } else {
            None
        },
    };
    (selection.model.is_some() || selection.effort.is_some()).then_some(selection)
}

fn native_observation(path: &Path) -> Result<NativeObservation, String> {
    use std::io::{Read, Seek, SeekFrom};
    let metadata = std::fs::symlink_metadata(path).map_err(|error| {
        format!(
            "cannot inspect native usage log {}: {error}",
            path.display()
        )
    })?;
    if !metadata.file_type().is_file() || metadata.file_type().is_symlink() {
        return Err("native usage log must be an ordinary file".into());
    }
    let mut file = std::fs::File::open(path)
        .map_err(|error| format!("cannot open native usage log {}: {error}", path.display()))?;
    let head_len = metadata.len().min(MAX_SESSION_META_BYTES);
    let mut head = vec![0; usize::try_from(head_len).map_err(|e| e.to_string())?];
    file.read_exact(&mut head).map_err(|e| e.to_string())?;
    let meta = head
        .split(|byte| *byte == b'\n')
        .filter_map(|line| serde_json::from_slice::<Value>(line).ok())
        .find(|value| value.get("type").and_then(Value::as_str) == Some("session_meta"))
        .ok_or("native usage log has no session_meta in its bounded header")?;
    let payload = meta.get("payload").unwrap_or(&meta);
    let thread = payload
        .get("id")
        .and_then(Value::as_str)
        .ok_or("native session_meta has no thread id")?
        .to_owned();
    identifier("thread", &thread)?;
    let client = payload
        .get("client")
        .and_then(Value::as_str)
        .unwrap_or("codex")
        .to_owned();
    identifier("client", &client)?;

    let tail_start = metadata.len().saturating_sub(MAX_INPUT_BYTES);
    file.seek(SeekFrom::Start(tail_start))
        .map_err(|e| e.to_string())?;
    let mut tail = Vec::new();
    file.take(MAX_INPUT_BYTES)
        .read_to_end(&mut tail)
        .map_err(|e| e.to_string())?;
    if tail_start > 0 {
        if let Some(newline) = tail.iter().position(|byte| *byte == b'\n') {
            tail.drain(..=newline);
        } else {
            return Err("native usage log has no complete JSON record in its bounded tail".into());
        }
    }
    let values: Vec<Value> = tail
        .split(|byte| *byte == b'\n')
        .filter(|line| !line.is_empty())
        .filter_map(|line| serde_json::from_slice(line).ok())
        .collect();
    let mut observed = Selection::default();
    let mut usage = None;
    for value in &values {
        let payload = value.get("payload").unwrap_or(value);
        let event_type = payload
            .get("type")
            .and_then(Value::as_str)
            .or_else(|| value.get("type").and_then(Value::as_str));
        if event_type == Some("turn_context")
            || value.get("type").and_then(Value::as_str) == Some("turn_context")
        {
            if let Some(model) = payload.get("model").and_then(Value::as_str) {
                observed.model = Some(model.into());
            }
            if let Some(effort) = payload
                .get("effort")
                .or_else(|| payload.get("reasoning_effort"))
                .and_then(Value::as_str)
            {
                observed.effort = Some(effort.into());
            }
        }
        if event_type == Some("token_count") {
            if let Some(total) = payload.pointer("/info/total_token_usage") {
                let read = |name: &str| total.get(name).and_then(Value::as_u64);
                usage = Some(CounterSnapshot {
                    at: 0,
                    input_tokens: read("input_tokens"),
                    cached_input_tokens: read("cached_input_tokens"),
                    output_tokens: read("output_tokens"),
                    reasoning_output_tokens: read("reasoning_output_tokens"),
                });
            }
        }
    }
    validate_selection("observed", &observed)?;
    let mut counters = usage.ok_or("native usage counters are unavailable; take a start checkpoint after the client emits token_count")?;
    if [
        counters.input_tokens,
        counters.cached_input_tokens,
        counters.output_tokens,
        counters.reasoning_output_tokens,
    ]
    .iter()
    .all(Option::is_none)
    {
        return Err("native token_count contains no usable cumulative counters".into());
    }
    counters.at = unix_now()?;
    validate_snapshot("native", &counters)?;
    Ok(NativeObservation {
        thread,
        client,
        observed,
        counters,
    })
}

#[cfg(unix)]
fn checkpoint(root: &Path, args: &[OsString]) -> Result<Value, String> {
    let args = checkpoint_args(args)?;
    let log = args.log.ok_or("usage checkpoint requires --log")?;
    let log = caller_path(root, log);
    let phase = args
        .phase
        .as_deref()
        .ok_or("usage checkpoint requires --phase")?;
    let role = args
        .role
        .as_deref()
        .ok_or("usage checkpoint requires --role")?;
    let observation = native_observation(&log)?;
    let file = checkpoint_file(&args.task, &observation.thread, &args.attempt);
    let state = state(root)?;
    if phase == "start" {
        if state
            .checkpoints
            .entries()?
            .iter()
            .any(|entry| entry == &file)
        {
            return Err("a start checkpoint already exists for this task/thread/attempt".into());
        }
        let checkpoint = Checkpoint {
            schema_version: 1,
            task: args.task.clone(),
            thread: observation.thread.clone(),
            attempt: args.attempt.clone(),
            client: observation.client.clone(),
            role: role.into(),
            observed: observation.observed,
            snapshot: observation.counters,
            evidence_reference: log.display().to_string(),
        };
        state.checkpoints.write_json(&file, &checkpoint)?;
        return Ok(json!({
            "schema_version": 1,
            "task": args.task,
            "thread": observation.thread,
            "attempt": args.attempt,
            "phase": "start",
            "recorded": true,
            "counters_available": true
        }));
    }
    let bytes = state
        .checkpoints
        .read(&file)
        .map_err(|_| "missing start checkpoint for this task/thread/attempt".to_string())?;
    let start: Checkpoint = serde_json::from_slice(&bytes)
        .map_err(|error| format!("invalid native start checkpoint: {error}"))?;
    if start.schema_version != 1
        || start.task != args.task
        || start.thread != observation.thread
        || start.attempt != args.attempt
        || start.client != observation.client
        || start.role != role
    {
        return Err("native end checkpoint does not match its start identity".into());
    }
    let observed = interval_selection(&start.observed, &observation.observed);
    let receipt = Receipt {
        schema_version: 1,
        task: args.task.clone(),
        thread: observation.thread.clone(),
        attempt: args.attempt.clone(),
        client: observation.client,
        role: role.into(),
        stage: None,
        active_skills: Vec::new(),
        segment: None,
        requested: None,
        observed,
        start: start.snapshot,
        end: observation.counters,
        evidence_reference: log.display().to_string(),
    };
    validate_receipt(&receipt)?;
    let existing = load_task(&state, &args.task)?;
    if existing.receipts.iter().any(|saved| {
        same_identity(saved, &receipt)
            && saved.start == receipt.start
            && saved.end.input_tokens == receipt.end.input_tokens
            && saved.end.cached_input_tokens == receipt.end.cached_input_tokens
            && saved.end.output_tokens == receipt.end.output_tokens
            && saved.end.reasoning_output_tokens == receipt.end.reasoning_output_tokens
            && saved.observed == receipt.observed
            && saved.evidence_reference == receipt.evidence_reference
    }) {
        return Ok(json!({
            "schema_version": 1,
            "task": args.task,
            "thread": observation.thread,
            "attempt": args.attempt,
            "phase": "end",
            "imported": false,
            "duplicate": true,
            "counters_available": true
        }));
    }
    let imported = add_receipt(&state, receipt)?;
    Ok(json!({
        "schema_version": 1,
        "task": args.task,
        "thread": observation.thread,
        "attempt": args.attempt,
        "phase": "end",
        "imported": imported,
        "duplicate": !imported,
        "counters_available": true
    }))
}

#[derive(Default)]
struct MarkArgs {
    task: String,
    log: Option<PathBuf>,
    stage: Option<String>,
    role: Option<String>,
    attempt: String,
    skills: Vec<String>,
}

fn mark_args(args: &[OsString]) -> Result<MarkArgs, String> {
    let mut parsed = MarkArgs {
        task: text(args.first().ok_or("usage mark requires TASK")?)?.into(),
        attempt: "default".into(),
        ..MarkArgs::default()
    };
    identifier("task", &parsed.task)?;
    let mut attempt_seen = false;
    let mut index = 1;
    while index < args.len() {
        let option = text(args.get(index).ok_or("missing mark option")?)?;
        let value = args
            .get(index + 1)
            .ok_or_else(|| format!("missing value for {option}"))?;
        match option {
            "--log" if parsed.log.is_none() => parsed.log = Some(PathBuf::from(value)),
            "--stage" if parsed.stage.is_none() => parsed.stage = Some(text(value)?.into()),
            "--role" if parsed.role.is_none() => parsed.role = Some(text(value)?.into()),
            "--attempt" if !attempt_seen => {
                parsed.attempt = text(value)?.into();
                attempt_seen = true;
            }
            "--skill" => parsed.skills.push(text(value)?.into()),
            _ => return Err(format!("unknown or repeated mark option {option}")),
        }
        index += 2;
    }
    identifier("attempt", &parsed.attempt)?;
    if !matches!(parsed.role.as_deref(), Some("coordinator" | "worker")) {
        return Err("usage mark --role must be coordinator or worker".into());
    }
    match parsed.stage.as_deref() {
        Some(stage) => validate_stage(stage)?,
        None => return Err("usage mark requires --stage".into()),
    }
    parsed.skills.sort();
    parsed.skills.dedup();
    validate_skills(&parsed.skills)?;
    if parsed.log.is_none() {
        return Err("usage mark requires --log PATH".into());
    }
    Ok(parsed)
}

#[cfg(unix)]
fn reject_mark_overlap(state: &State, current_file: &str, receipt: &Receipt) -> Result<(), String> {
    for entry in state.tasks.entries()? {
        let Some(task) = entry.strip_suffix(".json") else {
            continue;
        };
        identifier("stored task", task)?;
        for existing in load_task(state, task)?.receipts {
            reject_overlap(&existing, receipt)?;
        }
    }
    for file in state.marks.entries()? {
        if file == current_file {
            continue;
        }
        for existing in load_mark(state, &file)?.receipts {
            reject_overlap(&existing, receipt)?;
        }
    }
    Ok(())
}

fn receipt_from_active(
    record: &MarkRecord,
    active: &ActiveInterval,
    observation: &NativeObservation,
) -> Receipt {
    Receipt {
        schema_version: 1,
        task: record.task.clone(),
        thread: record.thread.clone(),
        attempt: record.attempt.clone(),
        client: record.client.clone(),
        role: record.role.clone(),
        stage: Some(active.stage.clone()),
        active_skills: active.active_skills.clone(),
        segment: Some(active.segment),
        requested: None,
        observed: interval_selection(&active.observed, &observation.observed),
        start: active.snapshot.clone(),
        end: observation.counters.clone(),
        evidence_reference: active.evidence_reference.clone(),
    }
}

#[cfg(unix)]
fn mark(root: &Path, args: &[OsString]) -> Result<Value, String> {
    let args = mark_args(args)?;
    let log = caller_path(root, args.log.ok_or("usage mark requires --log")?);
    let stage = args.stage.as_deref().ok_or("usage mark requires --stage")?;
    let role = args.role.as_deref().ok_or("usage mark requires --role")?;
    let observation = native_observation(&log)?;
    let file = checkpoint_file(&args.task, &observation.thread, &args.attempt);
    let state = state(root)?;
    let exists = state.marks.entries()?.iter().any(|entry| entry == &file);
    if !exists {
        let record = MarkRecord {
            schema_version: 1,
            task: args.task.clone(),
            thread: observation.thread.clone(),
            attempt: args.attempt.clone(),
            client: observation.client.clone(),
            role: role.into(),
            active: Some(ActiveInterval {
                stage: stage.into(),
                active_skills: args.skills,
                segment: 0,
                observed: observation.observed,
                snapshot: observation.counters,
                evidence_reference: log.display().to_string(),
            }),
            receipts: Vec::new(),
        };
        state.marks.write_json(&file, &record)?;
        return Ok(json!({
            "schema_version": 1,
            "task": args.task,
            "thread": observation.thread,
            "attempt": args.attempt,
            "stage": stage,
            "opened": true,
            "closed": false,
            "duplicate": false,
            "counters_available": true,
            "claim": "observed cumulative baseline; usage before this mark is not measured"
        }));
    }

    let mut record = load_mark(&state, &file)?;
    if record.task != args.task
        || record.thread != observation.thread
        || record.attempt != args.attempt
        || record.client != observation.client
        || record.role != role
    {
        return Err("usage mark does not match its active identity".into());
    }
    let Some(active) = record.active.clone() else {
        return Err("usage mark attempt is already finished".into());
    };
    let receipt = receipt_from_active(&record, &active, &observation);
    validate_receipt(&receipt)?;
    if active.stage == stage && active.active_skills == args.skills {
        return Ok(json!({
            "schema_version": 1,
            "task": args.task,
            "thread": observation.thread,
            "attempt": args.attempt,
            "stage": stage,
            "opened": false,
            "closed": false,
            "duplicate": true,
            "counters_available": true
        }));
    }

    for existing in &record.receipts {
        reject_overlap(existing, &receipt)?;
    }
    reject_mark_overlap(&state, &file, &receipt)?;
    record.receipts.push(receipt);
    record.active = Some(ActiveInterval {
        stage: stage.into(),
        active_skills: args.skills,
        segment: active
            .segment
            .checked_add(1)
            .ok_or("usage mark segment exceeds u64")?,
        observed: observation.observed,
        snapshot: observation.counters,
        evidence_reference: log.display().to_string(),
    });
    state.marks.write_json(&file, &record)?;
    Ok(json!({
        "schema_version": 1,
        "task": args.task,
        "thread": observation.thread,
        "attempt": args.attempt,
        "stage": stage,
        "opened": true,
        "closed": true,
        "duplicate": false,
        "counters_available": true
    }))
}

#[derive(Default)]
struct FinishArgs {
    task: String,
    log: Option<PathBuf>,
    attempt: String,
}

fn finish_args(args: &[OsString]) -> Result<FinishArgs, String> {
    let mut parsed = FinishArgs {
        task: text(args.first().ok_or("usage finish requires TASK")?)?.into(),
        attempt: "default".into(),
        ..FinishArgs::default()
    };
    identifier("task", &parsed.task)?;
    let mut attempt_seen = false;
    let mut index = 1;
    while index < args.len() {
        let option = text(args.get(index).ok_or("missing finish option")?)?;
        let value = args
            .get(index + 1)
            .ok_or_else(|| format!("missing value for {option}"))?;
        match option {
            "--log" if parsed.log.is_none() => parsed.log = Some(PathBuf::from(value)),
            "--attempt" if !attempt_seen => {
                parsed.attempt = text(value)?.into();
                attempt_seen = true;
            }
            _ => return Err(format!("unknown or repeated finish option {option}")),
        }
        index += 2;
    }
    identifier("attempt", &parsed.attempt)?;
    if parsed.log.is_none() {
        return Err("usage finish requires --log PATH".into());
    }
    Ok(parsed)
}

#[cfg(unix)]
fn finish(root: &Path, args: &[OsString]) -> Result<Value, String> {
    let args = finish_args(args)?;
    let log = caller_path(root, args.log.ok_or("usage finish requires --log")?);
    let observation = native_observation(&log)?;
    let file = checkpoint_file(&args.task, &observation.thread, &args.attempt);
    let state = state(root)?;
    if !state.marks.entries()?.iter().any(|entry| entry == &file) {
        return Err("usage finish has no marked attempt".into());
    }
    let mut record = load_mark(&state, &file)?;
    if record.task != args.task
        || record.thread != observation.thread
        || record.attempt != args.attempt
        || record.client != observation.client
    {
        return Err("usage finish does not match its marked attempt".into());
    }
    let Some(active) = record.active.clone() else {
        return Ok(json!({
            "schema_version": 1,
            "task": args.task,
            "thread": observation.thread,
            "attempt": args.attempt,
            "finished": false,
            "duplicate": true,
            "counters_available": true
        }));
    };
    let receipt = receipt_from_active(&record, &active, &observation);
    validate_receipt(&receipt)?;
    for existing in &record.receipts {
        reject_overlap(existing, &receipt)?;
    }
    reject_mark_overlap(&state, &file, &receipt)?;
    record.receipts.push(receipt);
    record.active = None;
    state.marks.write_json(&file, &record)?;
    Ok(json!({
        "schema_version": 1,
        "task": args.task,
        "thread": observation.thread,
        "attempt": args.attempt,
        "finished": true,
        "duplicate": false,
        "counters_available": true
    }))
}
