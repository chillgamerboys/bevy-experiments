//! Project-local Codex registration, with value ownership and recoverable writes.
use super::{archive, files, installed, sorted_bytes, strings, InstructionBundle};
use serde_json::{json, Value};
use std::path::Path;

pub(super) const CONFIG: &str = ".codex/config.toml";
pub(super) const RECORD: &str = ".gameskills/native-registration.json";
pub(super) const JOURNAL: &str = ".gameskills/native-transaction.json";

pub(super) fn previous(directory: &files::Directory, name: &str) -> Result<Option<String>, String> {
    directory
        .read_optional(name)?
        .map(String::from_utf8)
        .transpose()
        .map_err(|e| e.to_string())
}

fn document(text: Option<&str>) -> Result<toml_edit::DocumentMut, String> {
    text.unwrap_or_default()
        .parse()
        .map_err(|e| format!("invalid {CONFIG}: {e}"))
}

fn value(document: &toml_edit::DocumentMut, path: &[String]) -> Result<Value, String> {
    let parsed: toml::Value = toml::from_str(&document.to_string()).map_err(|e| e.to_string())?;
    let mut item = serde_json::to_value(parsed).map_err(|e| e.to_string())?;
    for key in path {
        item = item.get(key).cloned().unwrap_or(Value::Null);
    }
    Ok(item)
}

fn set(
    document: &mut toml_edit::DocumentMut,
    path: &[String],
    value: &Value,
) -> Result<(), String> {
    if value.is_null() {
        fn remove(table: &mut dyn toml_edit::TableLike, path: &[String]) {
            let Some((key, remaining)) = path.split_first() else {
                return;
            };
            if remaining.is_empty() {
                table.remove(key);
                return;
            }
            if let Some(child) = table
                .get_mut(key)
                .and_then(toml_edit::Item::as_table_like_mut)
            {
                remove(child, remaining);
            }
        }
        remove(document.as_table_mut(), path);
        return Ok(());
    }
    let mut table: &mut dyn toml_edit::TableLike = document.as_table_mut();
    for key in path.iter().take(path.len() - 1) {
        if !table.contains_key(key) {
            if value.is_null() {
                return Ok(());
            }
            table.insert(key, toml_edit::Item::Table(toml_edit::Table::new()));
        }
        table = table
            .get_mut(key)
            .and_then(toml_edit::Item::as_table_like_mut)
            .ok_or_else(|| format!("conflicting Codex setting: {}", path.join(".")))?;
    }
    let key = path.last().ok_or("empty registration path")?;
    if value.is_null() {
        table.remove(key);
    } else {
        let mut replacement = match value {
            Value::String(text) => toml_edit::Value::from(text.as_str()),
            Value::Bool(enabled) => toml_edit::Value::from(*enabled),
            _ => return Err("invalid registration setting value".into()),
        };
        if let Some(old) = table.get(key).and_then(toml_edit::Item::as_value) {
            *replacement.decor_mut() = old.decor().clone();
        }
        table.insert(key, toml_edit::Item::Value(replacement));
    }
    Ok(())
}

fn desired(
    bundle: &InstructionBundle,
    bundle_path: &Path,
) -> Result<Vec<(Vec<String>, Value)>, String> {
    let market = archive::marketplace(bundle);
    let mut entries = vec![
        (
            vec!["marketplaces".into(), market.clone(), "source_type".into()],
            json!("local"),
        ),
        (
            vec!["marketplaces".into(), market.clone(), "source".into()],
            json!(bundle_path.to_str().ok_or("bundle path must be UTF-8")?),
        ),
    ];
    for package in &bundle.selected {
        entries.push((
            vec![
                "plugins".into(),
                format!("{package}@{market}"),
                "enabled".into(),
            ],
            json!(true),
        ));
    }
    Ok(entries)
}

fn owned_path(entry: &Value) -> Result<Vec<String>, String> {
    super::exact_keys(entry, &["path", "before", "after"])?;
    let path = strings(entry, "path")?;
    let [section, name, field] = path.as_slice() else {
        return Err("invalid owned registration path".into());
    };
    let market = if section == "marketplaces" && matches!(field.as_str(), "source" | "source_type")
    {
        name.as_str()
    } else if section == "plugins" && field == "enabled" {
        let (package, market) = name
            .split_once('@')
            .ok_or("invalid owned plugin identity")?;
        if !archive::embedded()?
            .catalog
            .get("packages")
            .is_some_and(|p| p.get(package).is_some())
        {
            return Err("unknown owned plugin package".into());
        }
        market
    } else {
        return Err("invalid owned registration path".into());
    };
    let identity = market
        .strip_prefix("gameskills-")
        .ok_or("invalid owned marketplace")?;
    let (content, selection) = identity
        .split_once('-')
        .ok_or("invalid owned marketplace")?;
    if content.len() != 64
        || selection.len() != 8
        || !content
            .bytes()
            .chain(selection.bytes())
            .all(|b| b.is_ascii_hexdigit() && !b.is_ascii_uppercase())
    {
        return Err("invalid owned marketplace".into());
    }
    if !matches!(
        entry.get("before"),
        Some(Value::Null | Value::String(_) | Value::Bool(_))
    ) || !matches!(entry.get("after"), Some(Value::String(_) | Value::Bool(_)))
    {
        return Err("invalid owned registration value".into());
    }
    Ok(path)
}

pub(super) struct Plan {
    managed: bool,
    pub(super) before_config: Option<String>,
    pub(super) after_config: Option<String>,
    pub(super) before_record: Option<String>,
    pub(super) after_record: Option<String>,
}

impl Plan {
    pub(super) fn changed(&self) -> bool {
        self.before_config != self.after_config || self.before_record != self.after_record
    }
    pub(super) fn journal(&self) -> Value {
        if !self.managed {
            return json!({});
        }
        json!({CONFIG:{"before":self.before_config,"after":self.after_config}, RECORD:{"before":self.before_record,"after":self.after_record}})
    }
    pub(super) fn unchanged(&self, directory: &files::Directory) -> Result<bool, String> {
        if !self.managed {
            return Ok(previous(directory, RECORD)?.is_none());
        }
        Ok(previous(directory, CONFIG)? == self.before_config
            && previous(directory, RECORD)? == self.before_record)
    }
    pub(super) fn write(&self, directory: &files::Directory) -> Result<(), String> {
        if self.before_config != self.after_config {
            directory.write(CONFIG, self.after_config.as_deref().map(str::as_bytes))?;
        }
        if self.before_record != self.after_record {
            directory.write(RECORD, self.after_record.as_deref().map(str::as_bytes))?;
        }
        Ok(())
    }
}

pub(super) fn prepare(
    directory: &files::Directory,
    bundle: &InstructionBundle,
    bundle_path: &Path,
    selected: bool,
) -> Result<Plan, String> {
    if directory.read_optional(JOURNAL)?.is_some() {
        return Err(
            "interrupted native registration; use native codex --register --recover".into(),
        );
    }
    let before_record = previous(directory, RECORD)?;
    if !selected && before_record.is_none() {
        return Ok(Plan {
            managed: false,
            before_config: None,
            after_config: None,
            before_record: None,
            after_record: None,
        });
    }
    let before_config = previous(directory, CONFIG)?;
    let mut doc = document(before_config.as_deref())?;
    let mut original = doc.clone();
    let mut touched = std::collections::BTreeSet::new();
    let mut formerly_owned = std::collections::BTreeSet::new();
    let mut old_tables = std::collections::BTreeSet::new();
    // Restore only values we changed, and only while they still equal our last write.
    if let Some(record) = &before_record {
        let record = archive::json(record.as_bytes())?;
        super::exact_keys(&record, &["schema_version", "entries", "created_tables"])?;
        if record.get("schema_version") != Some(&json!(1)) {
            return Err("unsupported native registration record".into());
        }
        let mut seen = std::collections::BTreeSet::new();
        for entry in record
            .get("entries")
            .and_then(Value::as_array)
            .ok_or("invalid native registration entries")?
        {
            let path = owned_path(entry)?;
            if !seen.insert(path.clone()) {
                return Err("duplicate owned registration path".into());
            }
            touched.insert(path.clone());
            formerly_owned.insert(path.iter().take(2).cloned().collect::<Vec<_>>());
            if value(&doc, &path)? != entry["after"] {
                return Err(format!(
                    "native registration would overwrite a local edit: {}",
                    path.join(".")
                ));
            }
            set(&mut doc, &path, &entry["before"])?;
        }
        for table in record
            .get("created_tables")
            .and_then(Value::as_array)
            .ok_or("invalid created registration tables")?
        {
            let path = strings(&json!({"path":table}), "path")?;
            if !(1..=2).contains(&path.len())
                || !seen.iter().any(|owned| owned.starts_with(&path))
                || !old_tables.insert(path.clone())
            {
                return Err("invalid or duplicate created registration table".into());
            }
        }
        for path in old_tables.iter().rev() {
            if value(&doc, path)?
                .as_object()
                .is_some_and(|table| table.is_empty())
            {
                set(&mut doc, path, &Value::Null)?;
            }
        }
    }
    let mut created_tables = std::collections::BTreeSet::new();
    let mut entries = Vec::new();
    let desired = if selected {
        desired(bundle, bundle_path)?
    } else {
        Vec::new()
    };
    let wanted_parents = desired
        .iter()
        .map(|(path, _)| path.iter().take(2).cloned().collect::<Vec<_>>())
        .collect::<std::collections::BTreeSet<_>>();
    if selected {
        for (path, after) in desired {
            touched.insert(path.clone());
            let before = value(&doc, &path)?;
            if !before.is_null() && before != after {
                return Err(format!(
                    "conflicting project Codex setting: {}",
                    path.join(".")
                ));
            }
            if before != after {
                for depth in 1..path.len() {
                    let parent = path.get(..depth).ok_or("invalid registration parent")?;
                    if value(&doc, parent)?.is_null() {
                        created_tables.insert(parent.to_vec());
                    }
                }
                set(&mut doc, &path, &after)?;
                entries.push(json!({"path":path,"before":before,"after":after}));
            }
        }
    }
    for parent in formerly_owned.difference(&wanted_parents) {
        if value(&doc, parent)?
            .as_object()
            .is_some_and(|table| !table.is_empty())
        {
            return Err(format!("obsolete native registration has additional local settings; preserve or remove them explicitly before updating: {}",parent.join(".")));
        }
    }
    if selected {
        let known = bundle
            .catalog
            .get("packages")
            .and_then(Value::as_object)
            .ok_or("invalid package catalog")?;
        let plugins = value(&doc, &["plugins".into()])?;
        if let Some(plugins) = plugins.as_object() {
            for (id, settings) in plugins {
                if id
                    .split_once('@')
                    .is_some_and(|(package, _)| known.contains_key(package))
                    && !wanted_parents.contains(&vec!["plugins".into(), id.clone()])
                    && settings.get("enabled") != Some(&Value::Bool(false))
                {
                    return Err(format!("another project GameSkills registration remains enabled: {id}; resolve it without overwriting owner settings"));
                }
            }
        }
    }
    // Apply only semantic differences to the original document: idempotent repair
    // preserves the owner's formatting and comments even on managed values.
    for path in touched {
        let after = value(&doc, &path)?;
        if value(&original, &path)? != after {
            set(&mut original, &path, &after)?;
        }
    }
    // Empty parent tables supplied by the owner are never pruned. Remove only
    // obsolete containers recorded as created by GameSkills, after their leaves.
    for path in old_tables.iter().rev() {
        if value(&doc, path)?.is_null()
            && value(&original, path)?
                .as_object()
                .is_some_and(|table| table.is_empty())
        {
            set(&mut original, path, &Value::Null)?;
        }
    }
    let text = original.to_string();
    let after_config = if before_config.is_none() && text.is_empty() {
        None
    } else {
        Some(text)
    };
    let after_record = if selected {
        Some(
            String::from_utf8(sorted_bytes(
                &json!({"schema_version":1,"entries":entries,"created_tables":created_tables}),
            )?)
            .map_err(|e| e.to_string())?,
        )
    } else {
        None
    };
    Ok(Plan {
        managed: true,
        before_config,
        after_config,
        before_record,
        after_record,
    })
}

pub(super) fn status(
    root: &Path,
    bundle: &InstructionBundle,
    bundle_path: &Path,
    clients: &[String],
) -> Value {
    let mut statuses = serde_json::Map::new();
    for client in clients {
        let status = if client == "codex" {
            let inspect = || -> Result<Value, String> {
                let directory = files::Directory::open(root)?;
                let plan = prepare(&directory, bundle, bundle_path, true)?;
                let doc = document(plan.before_config.as_deref())?;
                let mismatches = desired(bundle, bundle_path)?
                    .into_iter()
                    .filter_map(|(path, expected)| match value(&doc, &path) {
                        Ok(actual) if actual == expected => None,
                        _ => Some(path.join(".")),
                    })
                    .collect::<Vec<_>>();
                Ok(
                    json!({"registration":if mismatches.is_empty(){"registered"}else if plan.before_config.is_none(){"missing"}else{"outdated"},"mismatched_settings":mismatches}),
                )
            };
            let mut state =
                inspect().unwrap_or_else(|error| json!({"registration":"conflict","detail":error}));
            let fields = state
                .as_object_mut()
                .expect("constructed registration status object");
            let ready = fields.get("registration").and_then(Value::as_str) == Some("registered");
            fields.insert("project_registration_ready".into(), json!(ready));
            fields.insert("scope".into(), json!("project"));
            fields.insert("path".into(), json!(root.join(CONFIG)));
            fields.insert(
                "discovery".into(),
                json!("unobserved; run native codex --verify-project"),
            );
            fields.insert(
                "session".into(),
                json!("existing sessions may require restart; session activation not observed"),
            );
            fields.insert(
                "trust".into(),
                json!("trusted-project loading required; trust is not changed or inferred"),
            );
            state
        } else {
            json!({"registration":"unsupported","project_registration_ready":false,"scope":"session launch only","discovery":"unobserved","next_action":"native claude --launch"})
        };
        statuses.insert(client.clone(), status);
    }
    Value::Object(statuses)
}

pub(super) fn execute(root: &Path, apply: bool, recover: bool) -> Result<Value, String> {
    let directory = files::Directory::open(root)?;
    let _guard = if apply || recover {
        Some(directory.lock(".gameskills/setup.lock")?)
    } else {
        None
    };
    if recover {
        if apply {
            return Err("use --register --recover without --apply".into());
        }
        if directory.read_optional(super::JOURNAL)?.is_some() {
            return Err("interrupted setup; use setup --recover".into());
        }
        let Some(bytes) = directory.read_optional(JOURNAL)? else {
            return Ok(json!({"ok":true,"recovered":false}));
        };
        let transaction = archive::json(&bytes)?;
        super::exact_keys(&transaction, &[CONFIG, RECORD])?;
        super::setup::restore(&directory, &transaction, &[CONFIG, RECORD])?;
        directory.remove(JOURNAL)?;
        return Ok(
            json!({"ok":true,"recovered":true,"action":"restored previous project Codex registration"}),
        );
    }
    let (bundle, config, path) = installed(root)?;
    if !strings(&config, "clients")?
        .iter()
        .any(|client| client == "codex")
    {
        return Err("native client is not selected in project configuration".into());
    }
    let plan = prepare(&directory, &bundle, &path, true)?;
    let changed = plan.changed();
    if apply && changed {
        if !plan.unchanged(&directory)? {
            return Err("project native settings changed during registration".into());
        }
        directory.write(JOURNAL, Some(&sorted_bytes(&plan.journal())?))?;
        plan.write(&directory)?;
        directory.remove(JOURNAL)?;
    }
    Ok(
        json!({"ok":true,"applied":apply,"registration_change":changed,"client":"codex","scope":"project","path":root.join(CONFIG),"content_sha256":bundle.identity(),"pin_change":false,"settings":desired(&bundle,&path)?.into_iter().map(|(path,value)|json!({"path":path,"value":value})).collect::<Vec<_>>(),"global_configuration_writes":false,"native_clients":status(root,&bundle,&path,&["codex".into()]),"claim":if apply{"project registration confirmed; native discovery and current session activation not observed"}else{"registration proposal only; no configuration written"}}),
    )
}
