//! Recoverable two-file installation transactions.
use super::{archive, files, normalized, sorted_bytes, strings, CONFIG, JOURNAL, LOCK};
use serde_json::{json, Value};
use std::{fs::File, path::Path};

fn configuration(existing: Option<&str>, selected: &[String]) -> Result<String, String> {
    let Some(existing) = existing else {
        return Ok(format!("# Project-owned GameSkills configuration. Commands execute only when requested.\nschema_version = 1\npackages = {}\nclients = [\"codex\", \"claude\"]\n\n[creative]\ndefault_level = 2\n\n[dispatch]\nenabled = false\nmax_workers = 5\n", serde_json::to_string(selected).map_err(|e| e.to_string())?));
    };
    let old = normalized(existing)?;
    if strings(&old, "packages")? == selected {
        return Ok(existing.to_owned());
    }
    let mut document = existing
        .parse::<toml_edit::DocumentMut>()
        .map_err(|e| e.to_string())?;
    let mut array = toml_edit::Array::new();
    for name in selected {
        array.push(name.as_str());
    }
    let mut value = toml_edit::Value::Array(array);
    if let Some(old) = document.get("packages").and_then(toml_edit::Item::as_value) {
        *value.decor_mut() = old.decor().clone();
    }
    document.insert("packages", toml_edit::Item::Value(value));
    let text = document.to_string();
    let mut expected = old;
    expected
        .as_object_mut()
        .ok_or("invalid configuration")?
        .insert("packages".into(), json!(selected));
    if normalized(&text)? != expected {
        return Err("package selection would alter other configuration".into());
    }
    Ok(text)
}

fn previous(directory: &files::Directory, name: &str) -> Result<Option<String>, String> {
    directory
        .read_optional(name)?
        .map(String::from_utf8)
        .transpose()
        .map_err(|e| e.to_string())
}

pub(super) fn inactive_history(directory: &files::Directory) -> Result<Option<File>, String> {
    let queues = directory.child(".gameskills/queues", true)?;
    let guard = queues.lock(".lock")?;
    for (name, bytes) in queues.tree()? {
        if name == ".lock" {
            continue;
        }
        if !name.ends_with(".json") || name.contains('/') {
            return Err("unexpected queue state while checking runtime transition".into());
        }
        let queue = archive::json(&bytes)?;
        match queue.get("schema_version").and_then(Value::as_u64) {
            Some(2) if queue.get("runtime").and_then(Value::as_str) == Some("rust") => {
                let orders = queue
                    .get("orders")
                    .and_then(Value::as_object)
                    .ok_or("invalid Rust queue")?;
                if orders
                    .values()
                    .any(|order| order.get("state").and_then(Value::as_str) != Some("integrated"))
                {
                    return Err(format!(
                        "unfinished Rust queue {name}; complete it before changing installation"
                    ));
                }
            }
            Some(1) => {
                let orders = queue
                    .get("orders")
                    .and_then(Value::as_object)
                    .ok_or("invalid historical queue")?;
                if orders
                    .values()
                    .any(|order| order.get("state").and_then(Value::as_str) != Some("integrated"))
                {
                    return Err(format!("unfinished Python queue {name}; finish its original runtime before Rust setup"));
                }
            }
            _ => {
                return Err(format!(
                    "unknown queue schema in {name}; cannot establish safe runtime transition"
                ))
            }
        }
    }
    Ok(Some(guard))
}

fn inactive_runs(directory: &files::Directory) -> Result<Vec<File>, String> {
    if !directory.exists(".gameskills/runs")? {
        return Ok(Vec::new());
    }
    let runs = directory.child(".gameskills/runs", false)?;
    let mut guards = Vec::new();
    for name in runs.names()? {
        if name.starts_with('.') {
            continue;
        }
        if name.len() != 32
            || !name
                .bytes()
                .all(|b| b.is_ascii_hexdigit() && !b.is_ascii_uppercase())
        {
            return Err("unknown run directory while checking installation transition".into());
        }
        let run = runs.child(&name, false)?;
        if !run.exists("active.lock")? {
            return Err(format!(
                "run {name} lacks active lock; restore its original state before setup"
            ));
        }
        guards.push(
            run.lock("active.lock")
                .map_err(|_| format!("cannot change installation while run {name} is active"))?,
        );
    }
    Ok(guards)
}

fn recover(directory: &files::Directory) -> Result<Value, String> {
    let Some(bytes) = directory.read_optional(JOURNAL)? else {
        return Ok(json!({"ok":true,"recovered":false}));
    };
    // Recovery changes the same managed files as apply and must retain the same
    // queue/run exclusions while validating and restoring the transaction.
    let _queue_guard = inactive_history(directory)?;
    let _run_guards = inactive_runs(directory)?;
    let transaction = archive::json(&bytes)?;
    super::exact_keys(&transaction, &[CONFIG, LOCK])?;
    for name in [CONFIG, LOCK] {
        let versions = transaction
            .get(name)
            .ok_or("invalid setup recovery journal")?;
        super::exact_keys(versions, &["before", "after"])?;
        if ["before", "after"]
            .iter()
            .any(|key| !matches!(versions.get(*key), Some(Value::Null | Value::String(_))))
        {
            return Err("invalid setup recovery values".into());
        }
        let current = previous(directory, name)?;
        if !["before", "after"]
            .iter()
            .any(|key| current.as_deref() == versions.get(*key).and_then(Value::as_str))
        {
            return Err(format!("recovery would overwrite a local edit: {name}"));
        }
    }
    for name in [CONFIG, LOCK] {
        directory.write(
            name,
            transaction
                .get(name)
                .and_then(|v| v.get("before"))
                .and_then(Value::as_str)
                .map(str::as_bytes),
        )?;
    }
    directory.remove(JOURNAL)?;
    Ok(json!({"ok":true,"recovered":true,"action":"restored previous configuration and lock"}))
}

pub(super) fn execute(root: &Path, args: &[String]) -> Result<Value, String> {
    if args == ["--help"] {
        return Ok(
            json!({"ok":true,"help":"gameskills setup [--packages NAME ...] [--bundle ARCHIVE_OR_DIRECTORY] [--apply]\ngameskills setup --recover\nProposals are read-only. Apply uses the embedded baseline unless a bundle is explicitly selected. Re-select a retained bundle to roll back."}),
        );
    }
    let mut apply = false;
    let mut recovery = false;
    let mut bundle_path = None;
    let mut selected = Vec::new();
    let mut iter = args.iter().peekable();
    while let Some(arg) = iter.next() {
        match arg.as_str() {
            "--apply" if !apply => apply = true,
            "--recover" if !recovery => recovery = true,
            "--bundle" if bundle_path.is_none() => {
                bundle_path = Some(iter.next().ok_or("--bundle requires a path")?)
            }
            "--packages" if selected.is_empty() => {
                while iter.peek().is_some_and(|v| !v.starts_with('-')) {
                    selected.push(iter.next().expect("peeked argument").to_owned());
                }
                if selected.is_empty() {
                    return Err("--packages requires a selection".into());
                }
            }
            _ => return Err(format!("unknown or duplicate setup argument: {arg}")),
        }
    }
    let directory = files::Directory::open(root)?;
    if recovery {
        if apply || bundle_path.is_some() || !selected.is_empty() {
            return Err("use setup --recover on its own".into());
        }
        if directory.read_optional(JOURNAL)?.is_none() {
            // Existing lock still excludes a concurrent mutator, even without a journal.
            if directory.read_optional(".gameskills/setup.lock")?.is_some() {
                let _guard = directory.lock(".gameskills/setup.lock")?;
                return recover(&directory);
            }
            return Ok(json!({"ok":true,"recovered":false}));
        }
        let _guard = directory.lock(".gameskills/setup.lock")?;
        return recover(&directory);
    }
    if directory.read_optional(JOURNAL)?.is_some() {
        return Err("interrupted setup; use setup --recover before continuing".into());
    }
    let existing = previous(&directory, CONFIG)?;
    if selected.is_empty() {
        selected = if let Some(text) = &existing {
            strings(&normalized(text)?, "packages")?
        } else {
            vec!["gameskills".into()]
        };
    }
    let text = configuration(existing.as_deref(), &selected)?;
    let configured = normalized(&text)?;
    let bundle = if let Some(path) = bundle_path {
        archive::source(&root.join(path))?
    } else {
        archive::embedded()?
    }
    .select(&selected)?;
    let identity = archive::installation_id(bundle.identity(), &selected)?;
    let relative = format!(".gameskills/bundles/{identity}");
    let lock = json!({"schema_version":2,"runtime":"rust","bundle":relative,"content_sha256":bundle.identity(),"packages":bundle.selected});
    let lock_text = String::from_utf8(sorted_bytes(&lock)?).map_err(|e| e.to_string())?;
    let previous_lock = previous(&directory, LOCK)?;
    let proposed = json!({"ok":true,"applied":apply,"packages":selected,"clients":configured.get("clients"),"max_workers":configured.pointer("/dispatch/max_workers"),"config_change":existing.as_deref()!=Some(text.as_str()),"lock_change":previous_lock.as_deref()!=Some(lock_text.as_str()),"content_sha256":bundle.identity(),"source_commit":bundle.manifest.get("source_commit"),"destination":root.join(&relative),"native_activation":"explicit client activation still required"});
    if !apply {
        return Ok(proposed);
    }
    directory.child(".gameskills", true)?;
    let _guard = directory.lock(".gameskills/setup.lock")?;
    let _queue_guard = inactive_history(&directory)?;
    let _run_guards = inactive_runs(&directory)?;
    if directory.read_optional(JOURNAL)?.is_some() {
        return Err("interrupted setup; use setup --recover before continuing".into());
    }
    if previous(&directory, CONFIG)? != existing || previous(&directory, LOCK)? != previous_lock {
        return Err("project configuration or lock changed during setup".into());
    }
    let bundles = directory.child(".gameskills/bundles", true)?;
    if bundles.child(&identity, false).is_ok() {
        let installed = archive::read_installed(&bundles, &identity)?;
        if installed.manifest != bundle.manifest || installed.selected != bundle.selected {
            return Err("existing immutable bundle differs".into());
        }
    } else {
        // A unique staging directory is never exposed as the active identity until verified.
        let staging = format!(
            ".install-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map_err(|e| e.to_string())?
                .as_nanos()
        );
        let stage = bundles.child(&staging, true)?;
        archive::materialize(&stage, &bundle)?;
        let verified = archive::read_installed(&bundles, &staging)?;
        if verified.manifest != bundle.manifest {
            return Err("bundle changed during installation".into());
        }
        // Refuse a symlink or file at the immutable destination before rename.
        if root.join(&relative).symlink_metadata().is_ok() {
            return Err("immutable bundle destination already exists or is unsafe".into());
        }
        bundles.rename(&staging, &identity)?;
    }
    if previous(&directory, CONFIG)? != existing || previous(&directory, LOCK)? != previous_lock {
        return Err("project configuration or lock changed during setup".into());
    }
    if existing.as_deref() == Some(text.as_str())
        && previous_lock.as_deref() == Some(lock_text.as_str())
    {
        return Ok(proposed);
    }
    directory.write(JOURNAL, Some(&sorted_bytes(&json!({CONFIG:{"before":existing,"after":text},LOCK:{"before":previous_lock,"after":lock_text}}))?))?;
    directory.write(CONFIG, Some(text.as_bytes()))?;
    directory.write(LOCK, Some(lock_text.as_bytes()))?;
    directory.remove(JOURNAL)?;
    Ok(proposed)
}
