//! Validated work plans and revision-guarded queues. Observations never launch
//! workers, execute checks, merge source, or imply human acceptance.

mod storage;
mod validation;

use clap::{Parser, Subcommand};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use std::{
    collections::BTreeMap,
    ffi::OsString,
    path::{Path, PathBuf},
    time::{SystemTime, UNIX_EPOCH},
};
use validation::{
    ancestor, clean, configuration, id, mapping, repository_identity, text, validate_graph,
    validate_order,
};

/// Persisted queues intentionally differ from Python's schema-one records.
pub const QUEUE_SCHEMA: u64 = 2;
/// The implementation that created a queue; never rewritten on historical reads.
pub const RUNTIME: &str = "rust";

#[derive(Parser)]
#[command(name = "gameskills workflow", disable_help_flag = true)]
struct Arguments {
    #[command(subcommand)]
    family: Family,
}
#[derive(Subcommand)]
enum Family {
    Plan {
        #[command(subcommand)]
        action: PlanAction,
    },
    Queue {
        #[command(subcommand)]
        action: QueueAction,
    },
}
#[derive(Subcommand)]
enum PlanAction {
    Validate {
        #[arg(long)]
        file: PathBuf,
    },
}
#[derive(Subcommand)]
enum QueueAction {
    Create {
        #[arg(long)]
        file: PathBuf,
    },
    Status {
        queue_id: String,
    },
    Inject {
        queue_id: String,
        #[arg(long)]
        file: PathBuf,
        #[arg(long)]
        expected_revision: u64,
    },
    Start {
        queue_id: String,
        order_id: String,
        #[arg(long)]
        worktree: PathBuf,
        #[arg(long)]
        expected_revision: u64,
    },
    Resume {
        queue_id: String,
        order_id: String,
        #[arg(long)]
        worktree: PathBuf,
        #[arg(long)]
        expected_revision: u64,
    },
    Report {
        queue_id: String,
        order_id: String,
        #[arg(long)]
        file: PathBuf,
        #[arg(long)]
        expected_revision: u64,
    },
    Block {
        queue_id: String,
        order_id: String,
        #[arg(long)]
        file: PathBuf,
        #[arg(long)]
        expected_revision: u64,
    },
    Integrated {
        queue_id: String,
        order_id: String,
        #[arg(long)]
        file: PathBuf,
        #[arg(long)]
        expected_revision: u64,
    },
}

#[derive(Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Queue {
    schema_version: u64,
    runtime: String,
    id: String,
    revision: u64,
    plan: Value,
    config_digest: String,
    events: Vec<Value>,
    orders: BTreeMap<String, Entry>,
}
#[derive(Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Entry {
    spec: Value,
    state: String,
    attempts: Vec<Value>,
    reports: Vec<Value>,
    blocks: Vec<Value>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    checkout: Option<Value>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    integration: Option<Value>,
}
impl Entry {
    fn pending(spec: Value) -> Self {
        Self {
            spec,
            state: "pending".into(),
            attempts: vec![],
            reports: vec![],
            blocks: vec![],
            checkout: None,
            integration: None,
        }
    }
}

/// Execute a plan or queue command against a ready installation.
pub fn execute(
    root: &Path,
    config: &Value,
    family: &str,
    args: &[OsString],
) -> Result<Value, String> {
    let parsed = Arguments::try_parse_from(
        std::iter::once(OsString::from("workflow"))
            .chain(std::iter::once(OsString::from(family)))
            .chain(args.iter().cloned()),
    )
    .map_err(|e| e.to_string())?;
    match parsed.family {
        Family::Plan {
            action: PlanAction::Validate { file },
        } => {
            Ok(json!({"ok":true,"plan":validate_plan(&storage::read_input(&file)?, root, config)?}))
        }
        Family::Queue { action } => match action {
            QueueAction::Create { file } => create(&storage::read_input(&file)?, root, config),
            QueueAction::Status { queue_id } => status(&queue_id, root, config),
            QueueAction::Inject {
                queue_id,
                file,
                expected_revision,
            } => mutate(
                &queue_id,
                "inject",
                None,
                Some(storage::read_input(&file)?),
                None,
                expected_revision,
                root,
                config,
            ),
            QueueAction::Start {
                queue_id,
                order_id,
                worktree,
                expected_revision,
            } => mutate(
                &queue_id,
                "start",
                Some(&order_id),
                None,
                Some(&worktree),
                expected_revision,
                root,
                config,
            ),
            QueueAction::Resume {
                queue_id,
                order_id,
                worktree,
                expected_revision,
            } => mutate(
                &queue_id,
                "resume",
                Some(&order_id),
                None,
                Some(&worktree),
                expected_revision,
                root,
                config,
            ),
            QueueAction::Report {
                queue_id,
                order_id,
                file,
                expected_revision,
            } => mutate(
                &queue_id,
                "report",
                Some(&order_id),
                Some(storage::read_input(&file)?),
                None,
                expected_revision,
                root,
                config,
            ),
            QueueAction::Block {
                queue_id,
                order_id,
                file,
                expected_revision,
            } => mutate(
                &queue_id,
                "block",
                Some(&order_id),
                Some(storage::read_input(&file)?),
                None,
                expected_revision,
                root,
                config,
            ),
            QueueAction::Integrated {
                queue_id,
                order_id,
                file,
                expected_revision,
            } => mutate(
                &queue_id,
                "integrated",
                Some(&order_id),
                Some(storage::read_input(&file)?),
                None,
                expected_revision,
                root,
                config,
            ),
        },
    }
}

/// Validate a plan without mutating caller input or repository state.
pub fn validate_plan(plan: &Value, root: &Path, config: &Value) -> Result<Value, String> {
    validation::validate_plan(plan, root, config, true)
}
fn current_configuration(root: &Path, config: &Value) -> Result<(), String> {
    if storage::current_configuration(root)? != *config {
        return Err("configuration changed after it was read; reload current configuration before retrying the queue mutation".into());
    }
    Ok(())
}
fn digest(value: &Value) -> Result<String, String> {
    let mut sorted = value.clone();
    sorted.sort_all_objects();
    Ok(format!(
        "{:x}",
        Sha256::digest(serde_json::to_vec(&sorted).map_err(|e| e.to_string())?)
    ))
}
fn now() -> Value {
    let elapsed = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default();
    json!({"unix_seconds":elapsed.as_secs(),"nanoseconds":elapsed.subsec_nanos()})
}
fn event(
    queue: &mut Queue,
    action: &str,
    order_id: Option<&str>,
    config: &Value,
) -> Result<(), String> {
    queue.events.push(json!({"revision":queue.revision,"at":now(),"action":action,"order_id":order_id,"config_digest":digest(config)?}));
    Ok(())
}
fn specs(queue: &Queue) -> BTreeMap<String, Value> {
    queue
        .orders
        .iter()
        .map(|(name, entry)| (name.clone(), entry.spec.clone()))
        .collect()
}
fn at<'a>(value: &'a Value, keys: &[&str]) -> &'a Value {
    keys.iter()
        .fold(value, |value, key| value.get(key).unwrap_or(&Value::Null))
}
fn put(value: &mut Value, keys: &[&str], replacement: Value) -> Result<(), String> {
    let (key, parents) = keys.split_last().ok_or("empty JSON field path")?;
    let mut value = value;
    for parent in parents {
        value = value
            .get_mut(parent)
            .ok_or_else(|| format!("missing object {parent}"))?;
    }
    value
        .as_object_mut()
        .ok_or("expected mutable JSON object")?
        .insert((*key).to_owned(), replacement);
    Ok(())
}
fn string<'a>(value: &'a Value, key: &str) -> &'a str {
    value.get(key).and_then(Value::as_str).unwrap_or("")
}
fn list(value: &Value, key: &str) -> Vec<String> {
    value
        .get(key)
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter_map(Value::as_str)
        .map(str::to_owned)
        .collect()
}
fn get_entry<'a>(queue: &'a Queue, name: &str) -> Result<&'a Entry, String> {
    queue
        .orders
        .get(name)
        .ok_or_else(|| format!("unknown order {name}"))
}
fn last_report(entry: &Entry) -> Result<&Value, String> {
    entry
        .reports
        .last()
        .ok_or_else(|| "returned order has no report".into())
}

fn load(
    directory: &storage::Directory,
    queue_id: &str,
    root: &Path,
    config: &Value,
) -> Result<Queue, String> {
    let value = directory.read(&format!("{}.json", id(&json!(queue_id), "queue id")?))?;
    if value.get("schema_version").and_then(Value::as_u64) == Some(1) {
        return Err("historical Python queue cannot be mutated by Rust; finish it with the original runtime and create a fresh Rust queue".into());
    }
    let queue: Queue = serde_json::from_value(value).map_err(|e| format!("invalid queue: {e}"))?;
    if queue.schema_version != QUEUE_SCHEMA
        || queue.runtime != RUNTIME
        || queue.id != queue_id
        || queue.revision == 0
    {
        return Err("invalid queue identity, runtime or schema".into());
    }
    let mut recorded_config = config.clone();
    // Selection changes are observed by blockers, not confused with corrupt history.
    put(
        &mut recorded_config,
        &["packages"],
        queue.plan.get("packages").cloned().unwrap_or(Value::Null),
    )?;
    validation::validate_plan(&queue.plan, root, &recorded_config, false)?;
    for (name, entry) in &queue.orders {
        if string(&entry.spec, "id") != name
            || !["pending", "running", "blocked", "reported", "integrated"]
                .contains(&entry.state.as_str())
        {
            return Err("invalid queue order identity or state".into());
        }
        validate_order(&entry.spec, &queue.plan, &recorded_config)?;
        if ["running", "reported", "integrated"].contains(&entry.state.as_str())
            && entry.checkout.is_none()
        {
            return Err("invalid queue: started order has no checkout".into());
        }
        if ["reported", "integrated"].contains(&entry.state.as_str()) && entry.reports.is_empty() {
            return Err("invalid queue: returned order has no report".into());
        }
    }
    validate_graph(
        &specs(&queue),
        Some(
            &queue
                .orders
                .iter()
                .filter(|(_, e)| e.state != "integrated")
                .map(|(n, _)| n.clone())
                .collect(),
        ),
    )?;
    if queue
        .events
        .last()
        .and_then(|e| e.get("revision"))
        .and_then(Value::as_u64)
        != Some(queue.revision)
    {
        return Err("invalid queue event revision".into());
    }
    Ok(queue)
}
fn create(plan: &Value, root: &Path, config: &Value) -> Result<Value, String> {
    let plan = validate_plan(plan, root, config)?;
    let directory = storage::Directory::lock(root, true)?;
    current_configuration(root, config)?;
    directory.reject_active_legacy()?;
    let name = string(&plan, "id").to_owned();
    let filename = format!("{name}.json");
    if directory.exists(&filename)? {
        return Err("queue already exists; injection is add-only".into());
    }
    let orders = plan
        .get("orders")
        .and_then(Value::as_array)
        .ok_or("plan orders missing")?
        .iter()
        .map(|order| {
            (
                string(order, "id").to_owned(),
                Entry::pending(order.clone()),
            )
        })
        .collect();
    let mut queue = Queue {
        schema_version: QUEUE_SCHEMA,
        runtime: RUNTIME.into(),
        id: name,
        revision: 1,
        plan,
        config_digest: digest(config)?,
        events: vec![],
        orders,
    };
    event(&mut queue, "create", None, config)?;
    directory.write(
        &filename,
        &serde_json::to_value(&queue).map_err(|e| e.to_string())?,
    )?;
    Ok(json!({"ok":true,"queue":queue}))
}
fn blockers(queue: &Queue, order_id: &str, config: &Value) -> Result<Vec<String>, String> {
    let entry = get_entry(queue, order_id)?;
    let spec = &entry.spec;
    let mut reasons = vec![];
    let selected = list(config, "packages");
    let missing: Vec<_> = list(spec, "packages")
        .into_iter()
        .filter(|p| !selected.contains(p))
        .collect();
    if !missing.is_empty() {
        reasons.push(format!(
            "order packages are not selected in current configuration: {}",
            missing.join(", ")
        ));
    }
    for dep in list(spec, "dispatch_blockers") {
        if !["reported", "integrated"].contains(&get_entry(queue, &dep)?.state.as_str()) {
            reasons.push(format!("dispatch dependency {dep} has no returned work"));
        }
    }
    if string(at(spec, &["owner"]), "kind") == "agent" {
        if config.pointer("/dispatch/enabled").and_then(Value::as_bool) != Some(true) {
            reasons.push("dispatch is disabled in project configuration".into());
        }
        if active_workers(queue) >= configuration(config)? {
            reasons.push("worker limit reached".into());
        }
    }
    let specifications = specs(queue);
    for (name, other) in &queue.orders {
        if name == order_id || other.state == "integrated" {
            continue;
        }
        if other.state == "running" || string(at(&other.spec, &["owner"]), "kind") == "human" {
            if other.state == "pending" && validation::depends(&specifications, name, order_id) {
                continue;
            }
            if other.state == "reported" && validation::depends(&specifications, order_id, name) {
                continue;
            }
            if validation::file_conflict(spec, &other.spec) {
                reasons.push(format!("file ownership held by {name}"));
            }
            if validation::resource_conflict(spec, &other.spec) {
                reasons.push(format!("shared resource held by {name}"));
            }
        }
    }
    Ok(reasons)
}
fn active_workers(queue: &Queue) -> usize {
    queue
        .orders
        .values()
        .filter(|e| e.state == "running" && string(at(&e.spec, &["owner"]), "kind") == "agent")
        .count()
}
fn status(queue_id: &str, root: &Path, config: &Value) -> Result<Value, String> {
    configuration(config)?;
    id(&json!(queue_id), "queue id")?;
    let directory = storage::Directory::lock(root, false)?;
    let original = directory.read(&format!("{queue_id}.json"))?;
    if original.get("schema_version").and_then(Value::as_u64) == Some(1) {
        let actual = repository_identity(root)?;
        if string(&original, "id") != queue_id
            || original.pointer("/plan/repository/root") != actual.get("root")
            || original.pointer("/plan/repository/common_dir") != actual.get("common_dir")
        {
            return Err("historical queue belongs to a different repository or checkout".into());
        }
        return Ok(
            json!({"ok":true,"queue":original,"compatibility":{"runtime":"python","historical":true,"mutable":false,"action":"Finish active work with the original runtime; create a fresh Rust queue after it is complete."}}),
        );
    }
    let queue = load(&directory, queue_id, root, config)?;
    let mut view = serde_json::to_value(&queue).map_err(|e| e.to_string())?;
    put(
        &mut view,
        &["config_changed"],
        json!(queue.config_digest != digest(config)?),
    )?;
    put(
        &mut view,
        &["active_workers"],
        json!(active_workers(&queue)),
    )?;
    for (name, entry) in &queue.orders {
        put(
            &mut view,
            &["orders", name, "waiting_reasons"],
            json!(if ["pending", "blocked"].contains(&entry.state.as_str()) {
                blockers(&queue, name, config)?
            } else {
                vec![]
            }),
        )?;
        put(
            &mut view,
            &["orders", name, "merge_waiting_reasons"],
            json!(list(&entry.spec, "merge_blockers")
                .into_iter()
                .filter(|dep| queue
                    .orders
                    .get(dep)
                    .is_none_or(|e| e.state != "integrated"))
                .map(|dep| format!("merge dependency {dep} is not integrated"))
                .collect::<Vec<_>>()),
        )?;
    }
    Ok(json!({"ok":true,"queue":view}))
}
fn checkout(queue: &Queue, order_id: &str, worktree: &Path, root: &Path) -> Result<Value, String> {
    let entry = get_entry(queue, order_id)?;
    let mut actual = repository_identity(worktree)?;
    let repo = at(&queue.plan, &["repository"]);
    if actual.get("common_dir") != repo.get("common_dir") {
        return Err("worker worktree belongs to a different repository".into());
    }
    if string(at(&entry.spec, &["owner"]), "kind") == "agent"
        && actual.get("root") == repository_identity(root)?.get("root")
    {
        return Err(
            "agent workers require a worktree isolated from the coordinator checkout".into(),
        );
    }
    if !ancestor(root, string(repo, "base_commit"), string(&actual, "head"))?
        || !ancestor(root, string(repo, "source_commit"), string(&actual, "head"))?
    {
        return Err("worktree HEAD does not descend from the planned source and base".into());
    }
    for dep in list(&entry.spec, "dispatch_blockers") {
        let predecessor = get_entry(queue, &dep)?;
        let returned = &last_report(predecessor)?["identity"];
        if predecessor.state == "reported" {
            let returned_root = Path::new(string(returned, "root"));
            let current = repository_identity(returned_root)?;
            if ["root", "common_dir", "head"]
                .iter()
                .any(|key| current.get(key) != returned.get(key))
                || !clean(returned_root)?
            {
                return Err(format!(
                    "returned dispatch dependency {dep} changed since its report"
                ));
            }
        }
        if !ancestor(root, string(returned, "head"), string(&actual, "head"))? {
            return Err(format!(
                "worktree does not contain returned dispatch dependency {dep}"
            ));
        }
    }
    for (name, other) in &queue.orders {
        if name != order_id
            && other.state != "integrated"
            && other.checkout.as_ref().and_then(|v| v.get("root")) == actual.get("root")
        {
            return Err("worktree is already owned by an unfinished stream".into());
        }
    }
    put(
        &mut actual,
        &["base_commit"],
        at(repo, &["base_commit"]).clone(),
    )?;
    put(
        &mut actual,
        &["source_commit"],
        at(repo, &["source_commit"]).clone(),
    )?;
    Ok(actual)
}
fn observation(
    queue: &Queue,
    order_id: &str,
    payload: &Value,
    root: &Path,
) -> Result<Value, String> {
    mapping(payload, "observation")?;
    text(at(payload, &["summary"]), "observation.summary")?;
    if payload.get("checks_running").and_then(Value::as_bool) != Some(false) {
        return Err("observation must explicitly record checks_running: false".into());
    }
    let tree = text(at(payload, &["worktree"]), "observation.worktree")?;
    let identity = checkout(queue, order_id, Path::new(tree), root)?;
    if get_entry(queue, order_id)?
        .checkout
        .as_ref()
        .and_then(|v| v.get("root"))
        != identity.get("root")
    {
        return Err("observation worktree differs from the started worktree".into());
    }
    for key in ["head", "base_commit"] {
        if payload.get(key) != identity.get(key) {
            return Err(format!(
                "observation {key} does not match the verified checkout identity"
            ));
        }
    }
    if !clean(Path::new(string(&identity, "root")))? {
        return Err("report requires a clean committed worktree".into());
    }
    let evidence = payload
        .get("evidence")
        .and_then(Value::as_array)
        .filter(|a| !a.is_empty())
        .ok_or("observation needs evidence references")?;
    for item in evidence {
        mapping(item, "evidence reference")?;
        for key in ["kind", "reference", "summary"] {
            text(at(item, &[key]), &format!("evidence.{key}"))?;
        }
    }
    Ok(
        json!({"at":now(),"identity":identity,"observation":payload,"evidence_status":"caller-supplied references; execution and acceptance not verified"}),
    )
}
fn mutate(
    queue_id: &str,
    action: &str,
    order_id: Option<&str>,
    payload: Option<Value>,
    worktree: Option<&Path>,
    expected_revision: u64,
    root: &Path,
    config: &Value,
) -> Result<Value, String> {
    configuration(config)?;
    if expected_revision == 0 {
        return Err("expected_revision must be a positive integer".into());
    }
    let directory = storage::Directory::lock(root, false)?;
    current_configuration(root, config)?;
    let mut queue = load(&directory, queue_id, root, config)?;
    directory.reject_active_legacy()?;
    if queue.revision != expected_revision {
        return Err(format!(
            "stale revision: expected {expected_revision}, current {}",
            queue.revision
        ));
    }
    let next = queue
        .revision
        .checked_add(1)
        .ok_or("queue revision exhausted")?;
    let payload = payload.unwrap_or(Value::Null);
    let name;
    if action == "inject" {
        let spec = validate_order(&payload, &queue.plan, config)?;
        text(at(&spec, &["reason"]), "injection reason")?;
        if list(&spec, "investigation").is_empty() {
            return Err("injection needs current investigation evidence".into());
        }
        let actual = repository_identity(root)?;
        if spec.get("verified_source") != actual.get("head") {
            return Err("injection verified_source must match the current coordinator HEAD".into());
        }
        if !ancestor(
            root,
            string(at(&queue.plan, &["repository"]), "source_commit"),
            string(&actual, "head"),
        )? {
            return Err("injection checkout does not descend from the planned source".into());
        }
        name = string(&spec, "id").to_owned();
        if queue.orders.contains_key(&name) {
            return Err("injection is add-only; order already exists".into());
        }
        let mut specifications = specs(&queue);
        specifications.insert(name.clone(), spec.clone());
        let mut live = queue
            .orders
            .iter()
            .filter(|(_, entry)| entry.state != "integrated")
            .map(|(name, _)| name.clone())
            .collect::<std::collections::BTreeSet<_>>();
        live.insert(name.clone());
        validate_graph(&specifications, Some(&live))?;
        for (other_name, other) in &queue.orders {
            if other.state != "integrated"
                && validation::resource_conflict(&spec, &other.spec)
                && !validation::depends(&specifications, &name, other_name)
            {
                return Err(format!("injected shared resource collision with {other_name}; add a dispatch dependency"));
            }
        }
        queue.orders.insert(name.clone(), Entry::pending(spec));
    } else {
        name = id(&json!(order_id), "order id")?.to_owned();
        let mut entry = get_entry(&queue, &name)?.clone();
        match action {
            "start" | "resume" => {
                let required = if action == "start" {
                    "pending"
                } else {
                    "blocked"
                };
                if entry.state != required {
                    return Err(format!(
                        "{action} requires {required} state, found {}",
                        entry.state
                    ));
                }
                let reasons = blockers(&queue, &name, config)?;
                if !reasons.is_empty() {
                    return Err(reasons.join("; "));
                }
                let tree = worktree.ok_or("start and resume require an actual worktree")?;
                let observed = checkout(&queue, &name, tree, root)?;
                if action == "resume"
                    && entry
                        .checkout
                        .as_ref()
                        .is_some_and(|v| v.get("root") != observed.get("root"))
                {
                    return Err("resume must use the original worktree; replan a relocation".into());
                }
                entry.checkout = Some(observed.clone());
                entry
                    .attempts
                    .push(json!({"at":now(),"identity":observed,"config_digest":digest(config)?}));
                entry.state = "running".into();
            }
            "block" => {
                if !["pending", "running", "reported"].contains(&entry.state.as_str()) {
                    return Err(format!("cannot block an order in {} state", entry.state));
                }
                mapping(&payload, "block observation")?;
                text(at(&payload, &["reason"]), "block reason")?;
                if payload.get("checks_running").and_then(Value::as_bool) != Some(false) {
                    return Err(
                        "block requires checks_running: false; collect or stop actual checks first"
                            .into(),
                    );
                }
                entry.blocks.push(json!({"at":now(),"observation":payload,"revision":next,"config_digest":digest(config)?,"last_observed_checkout":entry.checkout,"status":"caller reports no running checks"}));
                entry.state = "blocked".into();
            }
            "report" => {
                if entry.state != "running" {
                    return Err(format!(
                        "report requires running state, found {}",
                        entry.state
                    ));
                }
                let mut report = observation(&queue, &name, &payload, root)?;
                put(&mut report, &["config_digest"], json!(digest(config)?))?;
                put(&mut report, &["revision"], json!(next))?;
                entry.reports.push(report);
                entry.state = "reported".into();
            }
            "integrated" => {
                if entry.state != "reported" {
                    return Err(format!(
                        "integrated requires reported state, found {}",
                        entry.state
                    ));
                }
                for dep in list(&entry.spec, "merge_blockers") {
                    if get_entry(&queue, &dep)?.state != "integrated" {
                        return Err(format!("merge dependency {dep} is not integrated"));
                    }
                }
                mapping(&payload, "integration observation")?;
                text(at(&payload, &["summary"]), "integration summary")?;
                text(
                    at(&payload, &["evidence_reference"]),
                    "integration evidence_reference",
                )?;
                let mut actual = repository_identity(root)?;
                let report = last_report(&entry)?;
                if payload.get("head") != actual.get("head")
                    || payload.get("source_head") != at(report, &["identity"]).get("head")
                {
                    return Err(
                        "integration observation is stale or names a different source".into(),
                    );
                }
                let current =
                    repository_identity(Path::new(string(at(report, &["identity"]), "root")))?;
                if ["root", "common_dir", "head"]
                    .iter()
                    .any(|key| current.get(key) != at(report, &["identity"]).get(key))
                    || report.get("config_digest") != Some(&json!(digest(config)?))
                {
                    return Err(
                        "reported source or configuration changed; collect a new report".into(),
                    );
                }
                if !ancestor(
                    root,
                    string(at(report, &["identity"]), "head"),
                    string(&actual, "head"),
                )? {
                    return Err(
                        "reported source is not an ancestor of the actual integration HEAD".into(),
                    );
                }
                if !clean(root)? || !clean(Path::new(string(&current, "root")))? {
                    return Err("integration observation requires clean committed source and coordinator checkouts".into());
                }
                put(&mut actual, &["at"], now())?;
                put(&mut actual, &["observation"], payload)?;
                put(
                    &mut actual,
                    &["status"],
                    json!("git ancestry observed; review and combined checks are caller-supplied"),
                )?;
                entry.integration = Some(actual);
                entry.state = "integrated".into();
            }
            _ => return Err(format!("unknown queue action {action}")),
        }
        queue.orders.insert(name.clone(), entry);
    }
    queue.revision = next;
    event(&mut queue, action, Some(&name), config)?;
    directory.write(
        &format!("{queue_id}.json"),
        &serde_json::to_value(&queue).map_err(|e| e.to_string())?,
    )?;
    Ok(json!({"ok":true,"queue":queue}))
}
