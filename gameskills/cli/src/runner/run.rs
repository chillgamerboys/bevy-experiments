use super::{
    digest,
    graph::{self, Spec},
    identity, now,
    process::{self, Running},
    state::{self, Directory, Resources},
};
use clap::{Parser, Subcommand};
use serde_json::{json, Value};
use std::collections::{BTreeMap, BTreeSet};
use std::ffi::OsString;
use std::path::Path;
use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc,
};
use std::time::{Duration, Instant};

const CLAIM: &str = "observed command checks only";
#[derive(Parser)]
struct Arguments {
    #[arg(required = true)]
    names: Vec<String>,
    #[arg(long)]
    max_workers: Option<usize>,
    #[arg(long, default_value = "30")]
    resource_wait_seconds: f64,
    #[arg(long)]
    resume: Option<String>,
}
#[derive(Parser)]
struct Evidence {
    #[command(subcommand)]
    operation: Operation,
}
#[derive(Subcommand)]
enum Operation {
    List,
    Show { run_id: String },
    Validate { run_id: String },
}
struct Signals {
    stop: Arc<AtomicBool>,
    ids: Vec<signal_hook::SigId>,
}
impl Signals {
    fn new() -> Result<Self, String> {
        let stop = Arc::new(AtomicBool::new(false));
        let mut result = Self {
            stop,
            ids: Vec::new(),
        };
        for signal in [signal_hook::consts::SIGINT, signal_hook::consts::SIGTERM] {
            result.ids.push(
                signal_hook::flag::register(signal, Arc::clone(&result.stop))
                    .map_err(|e| e.to_string())?,
            );
        }
        Ok(result)
    }
}
impl Drop for Signals {
    fn drop(&mut self) {
        for id in &self.ids {
            signal_hook::low_level::unregister(*id);
        }
    }
}
fn save(directory: &Directory, record: &Value) -> Result<(), String> {
    directory.write_json(
        "record.json",
        &json!({"record":record,"sha256":digest(record)?}),
    )
}
pub(super) fn load(directory: &Directory) -> Result<Value, String> {
    let envelope: Value = serde_json::from_slice(&directory.read("record.json")?)
        .map_err(|e| format!("invalid evidence record: {e}"))?;
    let record = envelope
        .get("record")
        .filter(|v| v.is_object())
        .ok_or("invalid evidence record")?;
    if record.get("schema_version") == Some(&json!(1)) {
        return Ok(record.clone());
    }
    if record.get("schema_version") != Some(&json!(2))
        || record.get("runtime") != Some(&json!("rust"))
    {
        return Err("unsupported evidence schema/runtime".into());
    }
    if envelope.get("sha256").and_then(Value::as_str) != Some(digest(record)?.as_str()) {
        return Err("record digest mismatch; evidence was modified or damaged".into());
    }
    Ok(record.clone())
}
fn invalid(id: &str, reasons: Vec<String>, recorded: &Value) -> Value {
    json!({"ok":false,"status":"invalid","run_id":id,"recorded_status":recorded,"reasons":reasons,"claim":CLAIM})
}
fn validate(directory: &Directory, record: &Value, root: &Path, config: &Value, id: &str) -> Value {
    let mut reasons = Vec::new();
    if record.get("schema_version") != Some(&json!(2))
        || record.get("runtime") != Some(&json!("rust"))
    {
        return invalid(id,vec!["historical Python evidence is not reusable or validated by the Rust runtime; run fresh checks".into()],record.get("status").unwrap_or(&Value::Null));
    }
    if record.get("run_id") != Some(&json!(id)) {
        reasons.push("record has the wrong run identity".into());
    }
    if record.get("status") != Some(&json!("passed")) {
        reasons.push(format!(
            "run status is {}",
            record.get("status").unwrap_or(&Value::Null)
        ));
    }
    let check = (|| -> Result<(), String> {
        let selected = graph::strings(record.get("selected"), "selected")?;
        let (commands, order) = graph::graph(root, config, &selected)?;
        let current = identity::identity(root, config, &commands)?;
        if record.get("identity") != Some(&current) {
            reasons.extend(identity::differences(
                record.get("identity").unwrap_or(&Value::Null),
                &current,
            ));
        }
        if record.get("final_identity") != record.get("identity") {
            reasons.push("inputs changed during execution or final identity is unavailable".into());
        }
        let results = record
            .get("results")
            .and_then(Value::as_object)
            .ok_or("missing results")?;
        if record.get("order") != Some(&json!(order))
            || results.keys().collect::<BTreeSet<_>>() != commands.keys().collect()
        {
            reasons.push("command graph/results are incomplete".into());
        }
        for (name, spec) in commands {
            let result = results.get(&name).ok_or("missing command observation")?;
            if result.get("status") != Some(&json!("passed"))
                || result.get("exit_code") != Some(&json!(0))
            {
                reasons.push(format!("{name}: no observed zero-exit completion"));
            }
            let expected = serde_json::to_value(&spec).map_err(|e| e.to_string())?;
            for (key, value) in expected.as_object().ok_or("invalid spec")? {
                if result.get(key) != Some(value) {
                    reasons.push(format!(
                        "{name}: recorded {key} does not match current command"
                    ));
                }
            }
            if result.get("cleanup_errors").is_some() {
                reasons.push(format!("{name}: cleanup did not complete"));
            }
            for stream in ["stdout", "stderr"] {
                let file = format!("{name}.{stream}.log");
                let observed = result.get(stream).ok_or("missing log observation")?;
                if observed.get("file") != Some(&json!(file))
                    || observed.get("sha256") != Some(&json!(directory.file_digest(&file)?))
                {
                    reasons.push(format!("{name}: {stream} output digest mismatch"));
                }
            }
        }
        Ok(())
    })();
    if let Err(error) = check {
        reasons.push(format!("cannot validate evidence: {error}"));
    }
    if reasons.is_empty() {
        json!({"ok":true,"status":"valid","run_id":id,"recorded_status":record.get("status"),"reasons":[],"claim":CLAIM})
    } else {
        invalid(id, reasons, record.get("status").unwrap_or(&Value::Null))
    }
}
fn evidence(root: &Path, config: &Value, args: &[OsString]) -> Result<Value, String> {
    let args = Evidence::try_parse_from(
        std::iter::once(OsString::from("evidence")).chain(args.iter().cloned()),
    )
    .map_err(|e| e.to_string())?;
    let id = match &args.operation {
        Operation::List => None,
        Operation::Show { run_id } | Operation::Validate { run_id } => Some(run_id.as_str()),
    };
    if id.is_some_and(|id| !super::run_id(id)) {
        return Err("invalid run ID".into());
    }
    // A missing list is read-only. A symlink or malformed existing state is an error.
    let root_dir = Directory::root(root)?;
    if !root.join(".gameskills").exists()
        && std::fs::symlink_metadata(root.join(".gameskills")).is_err()
    {
        return if id.is_none() {
            Ok(json!({"ok":true,"status":"listed","runs":[]}))
        } else {
            Err("no command evidence exists".into())
        };
    }
    let state = root_dir.child(".gameskills", false, false)?;
    if matches!(
        rustix::fs::statat(&state.0, "runs", rustix::fs::AtFlags::SYMLINK_NOFOLLOW),
        Err(rustix::io::Errno::NOENT)
    ) {
        return if id.is_none() {
            Ok(json!({"ok":true,"status":"listed","runs":[]}))
        } else {
            Err("no command evidence exists".into())
        };
    }
    let runs = state.child("runs", false, false)?;
    let Some(id) = id else {
        let mut entries = Vec::new();
        for name in runs.entries()?.into_iter().filter(|id| super::run_id(id)) {
            match runs.child(&name,false,false).and_then(|dir|load(&dir)) {
                Ok(record)=>entries.push(json!({"run_id":name,"recorded_status":record.get("status"),"runtime":record.get("runtime").unwrap_or(&json!("python")),"started_at":record.get("started_at"),"validated":false})),
                Err(error)=>entries.push(json!({"run_id":name,"recorded_status":"invalid","error":error,"validated":false})),
            }
        }
        return Ok(json!({"ok":true,"status":"listed","runs":entries}));
    };
    match runs
        .child(id, false, false)
        .and_then(|dir| Ok((load(&dir)?, dir)))
    {
        Ok((record, dir)) => {
            let mut result = validate(&dir, &record, root, config, id);
            if matches!(args.operation, Operation::Show { .. }) {
                result
                    .as_object_mut()
                    .ok_or("invalid response")?
                    .insert("record".into(), record);
            }
            Ok(result)
        }
        Err(error) => Ok(invalid(id, vec![error], &Value::Null)),
    }
}
fn skipped(reason: &str) -> Value {
    json!({"status":"skipped","reason":reason,"exit_code":null})
}
fn schedule(
    root: &Path,
    directory: &Directory,
    active_lock: &std::fs::File,
    commands: &BTreeMap<String, Spec>,
    order: &[String],
    workers: usize,
    wait: f64,
    helper: &Path,
    record: &mut Value,
) -> Result<(), String> {
    let common = record
        .pointer("/identity/repository/common_dir")
        .and_then(Value::as_str)
        .ok_or("missing common Git directory")?;
    let resources = Resources::new(common)?;
    let signals = Signals::new()?;
    let mut pending: BTreeSet<_> = order.iter().cloned().collect();
    let mut active: Vec<Running> = Vec::new();
    let mut ready: BTreeMap<String, Instant> = BTreeMap::new();
    let run_id = record
        .get("run_id")
        .and_then(Value::as_str)
        .ok_or("missing run ID")?
        .to_owned();
    let scheduled = (|| {
        while !pending.is_empty() || !active.is_empty() {
            let interrupted = signals.stop.load(Ordering::Relaxed);
            if interrupted {
                let results = record
                    .get_mut("results")
                    .and_then(Value::as_object_mut)
                    .ok_or("invalid results")?;
                for name in &pending {
                    results.insert(name.clone(), skipped("runner interrupted before start"));
                }
                pending.clear();
                for running in &active {
                    running.cancel();
                }
                record
                    .as_object_mut()
                    .ok_or("invalid record")?
                    .insert("interrupted".into(), true.into());
            }
            for name in order {
                if !pending.contains(name) || active.len() >= workers {
                    continue;
                }
                let spec = commands.get(name).ok_or("missing command")?;
                let results = record
                    .get_mut("results")
                    .and_then(Value::as_object_mut)
                    .ok_or("invalid results")?;
                if spec
                    .requires
                    .iter()
                    .any(|dependency| !results.contains_key(dependency))
                {
                    continue;
                }
                if spec.requires.iter().any(|dependency| {
                    results.get(dependency).and_then(|r| r.get("status")) != Some(&json!("passed"))
                }) {
                    results.insert(name.clone(), skipped("dependency did not pass"));
                    pending.remove(name);
                    continue;
                }
                let since = ready.entry(name.clone()).or_insert_with(Instant::now);
                let Some(held) = resources.acquire(&spec.resources)? else {
                    if since.elapsed().as_secs_f64() >= wait {
                        results.insert(name.clone(), skipped("resource wait expired"));
                        pending.remove(name);
                    }
                    continue;
                };
                pending.remove(name);
                match process::start(
                    root,
                    directory,
                    active_lock,
                    held,
                    name,
                    spec,
                    helper,
                    &run_id,
                ) {
                    Ok(running) => active.push(running),
                    Err(error) => {
                        results.insert(
                            name.clone(),
                            json!({"status":"error","exit_code":null,"error":error}),
                        );
                    }
                }
            }
            let mut index = 0;
            while index < active.len() {
                let running = active.get_mut(index).ok_or("invalid active index")?;
                if let Some(exit) = running.poll()? {
                    let running = active.remove(index);
                    let name = running.name.clone();
                    let result = if exit.success() {
                        directory
                            .read(&format!("{name}.result.json"))
                            .and_then(|bytes| {
                                serde_json::from_slice(&bytes).map_err(|e| e.to_string())
                            })
                    } else {
                        Err(format!("Rust supervisor failed with {exit}"))
                    };
                    record
                        .get_mut("results")
                        .and_then(Value::as_object_mut)
                        .ok_or("invalid results")?
                        .insert(
                            name,
                            result.unwrap_or_else(
                                |error| json!({"status":"error","error":error,"exit_code":null}),
                            ),
                        );
                    drop(running);
                    save(directory, record)?;
                } else {
                    index += 1;
                }
            }
            if !pending.is_empty() || !active.is_empty() {
                std::thread::sleep(Duration::from_millis(15));
            }
        }
        Ok(())
    })();
    // A coordinator error must still collect observations for commands that ran.
    for running in &active {
        running.cancel();
    }
    for running in active {
        let name = running.name.clone();
        drop(running);
        let result = directory
            .read(&format!("{name}.result.json"))
            .and_then(|bytes| serde_json::from_slice::<Value>(&bytes).map_err(|e| e.to_string()))
            .unwrap_or_else(|error| json!({"status":"error","exit_code":null,"error":error}));
        record
            .get_mut("results")
            .and_then(Value::as_object_mut)
            .ok_or("invalid results")?
            .insert(name, result);
    }
    scheduled
}
fn run(root: &Path, config: &Value, args: &[OsString], helper: &Path) -> Result<Value, String> {
    let mut args = Arguments::try_parse_from(
        std::iter::once(OsString::from("run")).chain(args.iter().cloned()),
    )
    .map_err(|e| e.to_string())?;
    args.names.sort();
    args.names.dedup();
    let (commands, order) = graph::graph(root, config, &args.names)?;
    let dispatch = config.get("dispatch").cloned().unwrap_or_else(|| json!({}));
    let dispatch = dispatch.as_object().ok_or("dispatch must be a mapping")?;
    let cap = dispatch
        .get("max_workers")
        .unwrap_or(&json!(5))
        .as_u64()
        .filter(|cap| (1..=5).contains(cap))
        .ok_or("dispatch.max_workers must be an integer between 1 and 5")?;
    let workers = args
        .max_workers
        .unwrap_or(usize::try_from(cap).map_err(|e| e.to_string())?);
    if workers == 0 || workers > usize::try_from(cap).map_err(|e| e.to_string())? {
        return Err(format!(
            "--max-workers must be between 1 and configured cap {cap}"
        ));
    }
    if !args.resource_wait_seconds.is_finite() || args.resource_wait_seconds < 0.0 {
        return Err("resource wait must be finite and nonnegative".into());
    }
    if args.resume.as_deref().is_some_and(|id| !super::run_id(id)) {
        return Err("invalid resume run ID".into());
    }
    let state = Directory::root(root)?.child(".gameskills", true, false)?;
    let lifecycle = state.open("setup.lock", true, false)?;
    let registration_deadline = Instant::now() + Duration::from_secs(5);
    while !state::lock(&lifecycle)? {
        if Instant::now() >= registration_deadline {
            return Err("setup/update registration is busy; retry after it completes".into());
        }
        std::thread::sleep(Duration::from_millis(15));
    }
    let current = crate::installation::ready_config(root)?;
    if &current != config {
        return Err(
            "installation/configuration changed before run registration; reload configuration"
                .into(),
        );
    }
    let identity = identity::identity(root, config, &commands)?;
    let runs = state.child("runs", true, false)?;
    if let Some(id) = &args.resume {
        let directory = runs.child(id, false, false)?;
        let previous = load(&directory)?;
        if previous.get("schema_version") != Some(&json!(2))
            || previous.get("runtime") != Some(&json!("rust"))
        {
            return Err(
                "historical Python evidence cannot be resumed; start a new Rust run".into(),
            );
        }
        if previous.get("run_id") != Some(&json!(id)) {
            return Err("resume record has the wrong run identity".into());
        }
        if previous.get("identity") != Some(&identity)
            || previous.get("selected") != Some(&json!(args.names))
        {
            return Err(
                "cannot resume: source or execution inputs changed; start a new run".into(),
            );
        }
        if !state::lock(&directory.open("active.lock", false, false)?)? {
            return Err("cannot resume an active run".into());
        }
    }
    let id = super::identifier();
    let directory = runs.child(&id, true, true)?;
    let active = directory.open("active.lock", true, true)?;
    if !state::lock(&active)? {
        return Err("new active lock unexpectedly held".into());
    }
    let mut record = json!({"schema_version":2,"runtime":"rust","run_id":id,"status":"running","selected":args.names,"order":order,"commands":commands,"identity":identity,"results":{},"started_at":now(),"max_workers":workers,"resource_wait_seconds":args.resource_wait_seconds,"resumed_from":args.resume,"claim":CLAIM});
    save(&directory, &record)?;
    drop(lifecycle);
    if let Err(error) = schedule(
        root,
        &directory,
        &active,
        &commands,
        &order,
        workers,
        args.resource_wait_seconds,
        helper,
        &mut record,
    ) {
        record
            .as_object_mut()
            .ok_or("invalid record")?
            .insert("runner_error".into(), error.into());
    }
    let results = record
        .get_mut("results")
        .and_then(Value::as_object_mut)
        .ok_or("invalid results")?;
    for name in &order {
        results
            .entry(name.clone())
            .or_insert_with(|| skipped("runner did not complete"));
    }
    let failed = results
        .values()
        .any(|r| r.get("status") != Some(&json!("passed")));
    let final_identity = identity::identity(root, config, &commands);
    let object = record.as_object_mut().ok_or("invalid record")?;
    match final_identity {
        Ok(value) => {
            object.insert("final_identity".into(), value);
        }
        Err(error) => {
            object.insert("identity_error".into(), error.into());
        }
    }
    let status = if object.get("interrupted") == Some(&json!(true)) {
        "interrupted"
    } else if failed || object.contains_key("runner_error") {
        "failed"
    } else if object.get("identity") != object.get("final_identity") {
        "stale"
    } else {
        "passed"
    };
    object.insert("status".into(), status.into());
    object.insert("finished_at".into(), json!(now()));
    save(&directory, &record)?;
    Ok(
        json!({"ok":status=="passed","status":status,"run_id":id,"record_path":root.join(".gameskills/runs").join(&id).join("record.json"),"results":record.get("results"),"runner_error":record.get("runner_error"),"identity_error":record.get("identity_error"),"claim":CLAIM}),
    )
}
pub(super) fn execute(
    root: &Path,
    config: &Value,
    family: &str,
    args: &[OsString],
    helper: &Path,
) -> Result<Value, String> {
    let root = root.canonicalize().map_err(|e| e.to_string())?;
    identity::repository(&root)?;
    match family {
        "run" => run(&root, config, args, helper),
        "evidence" => evidence(&root, config, args),
        _ => Err("expected run or evidence command family".into()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn fractional_records_survive_protected_save_and_load() -> Result<(), String> {
        let scratch = tempfile::tempdir().map_err(|e| e.to_string())?;
        let directory = Directory::root(scratch.path())?;
        let record = json!({
            "schema_version": 2,
            "runtime": "rust",
            "status": "failed",
            "started_at": 1_789_150_486.000_000_2_f64,
            "results": {"a": {"duration_seconds": 0.015000000000000001_f64}},
        });
        save(&directory, &record)?;
        assert_eq!(load(&directory)?, record);
        // Exact parsing must preserve integrity checks, including modifications
        // small enough that the default approximate parser could discard them.
        let mut envelope: Value =
            serde_json::from_slice(&directory.read("record.json")?).map_err(|e| e.to_string())?;
        *envelope
            .pointer_mut("/record/results/a/duration_seconds")
            .ok_or("missing duration")? = json!(0.015_f64);
        directory.write_json("record.json", &envelope)?;
        assert!(load(&directory)
            .err()
            .is_some_and(|error| error.contains("digest mismatch")));
        Ok(())
    }
}
