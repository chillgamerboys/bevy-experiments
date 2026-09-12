//! Read-only configuration validation, independent of installation state.

use std::collections::BTreeSet;
use toml::{Table, Value};

/// Package identities at the R0 reference; complete bundle loading belongs to R3.
pub const PACKAGES: [&str; 7] = [
    "gameskills",
    "gameskills-ui",
    "gameskills-turn-based",
    "gameskills-multiplayer",
    "gameskills-maintainer",
    "gameskills-bevy-contrib",
    "gameskills-linear",
];

/// Parse and normalize configuration without executing commands or changing files.
pub fn parse(source: &str) -> Result<Table, String> {
    let mut value = source.parse::<Table>().map_err(|error| error.to_string())?;
    if value.get("schema_version").and_then(Value::as_integer) != Some(1) {
        return Err("gameskills.toml requires integer schema_version = 1".into());
    }
    let allowed = [
        "schema_version",
        "packages",
        "clients",
        "project",
        "creative",
        "dispatch",
        "commands",
        "agents",
        "targets",
        "tracking",
        "docs",
    ];
    if let Some(key) = value.keys().find(|key| !allowed.contains(&key.as_str())) {
        return Err(format!("unknown configuration field: {key}"));
    }
    value
        .entry("packages")
        .or_insert_with(|| strings(&["gameskills"]));
    value
        .entry("clients")
        .or_insert_with(|| strings(&["codex", "claude"]));
    for name in ["creative", "dispatch", "commands", "agents", "targets"] {
        value
            .entry(name)
            .or_insert_with(|| Value::Table(Table::new()));
        table(&value, name)?;
    }
    if value.contains_key("project") {
        let project = table(&value, "project")?;
        if let Some(endpoint) = project.get("delivery_target") {
            if !endpoint.as_str().is_some_and(|s| {
                ["design", "implementation", "pr", "merge", "release"].contains(&s)
            }) {
                return Err(
                    "project.delivery_target must be design, implementation, pr, merge or release"
                        .into(),
                );
            }
        }
    }
    if value.contains_key("tracking") {
        let tracking = table(&value, "tracking")?;
        if tracking
            .keys()
            .any(|s| !["required", "observer"].contains(&s.as_str()))
        {
            return Err("tracking accepts required and observer only".into());
        }
        if tracking
            .get("required")
            .is_some_and(|v| v.as_bool().is_none())
        {
            return Err("tracking.required must be boolean".into());
        }
        if let Some(observer) = tracking.get("observer") {
            let words = observer
                .as_array()
                .filter(|v| !v.is_empty())
                .ok_or("tracking.observer must be a nonempty argv")?
                .iter()
                .map(|v| {
                    v.as_str()
                        .ok_or("tracking.observer arguments must be strings")
                })
                .collect::<Result<Vec<_>, _>>()?;
            if words
                .iter()
                .any(|s| s.trim().is_empty() || s.contains('\0'))
            {
                return Err("tracking.observer requires nonempty arguments without NUL".into());
            }
        }
        if tracking.get("required").and_then(Value::as_bool) == Some(true)
            && !tracking.contains_key("observer")
        {
            return Err("required tracking needs an observer command".into());
        }
    }
    if let Some(docs) = value.get("docs") {
        crate::docs::validate_mapping(docs, "docs")?;
    }
    let packages = string_array(value.get("packages"), "packages", true)?;
    if !packages.contains(&"gameskills") || packages.iter().any(|name| !PACKAGES.contains(name)) {
        return Err("select gameskills core and only known optional packages".into());
    }
    let clients = string_array(value.get("clients"), "clients", true)?;
    if clients
        .iter()
        .any(|name| !["codex", "claude"].contains(name))
    {
        return Err("supported clients are codex and claude".into());
    }
    for (name, command) in table(&value, "commands")? {
        if !identifier(name) {
            return Err(format!("invalid command name: {name}"));
        }
        let command = command
            .as_table()
            .ok_or_else(|| format!("commands.{name} must be a table"))?;
        let argv = command
            .get("argv")
            .and_then(Value::as_array)
            .ok_or_else(|| format!("commands.{name}.argv must be a nonempty string array"))?;
        if argv.is_empty()
            || argv
                .iter()
                .any(|arg| arg.as_str().is_none_or(|arg| arg.contains('\0')))
        {
            return Err(format!(
                "commands.{name}.argv must be a nonempty string array without NUL"
            ));
        }
    }
    let mut target_paths = BTreeSet::new();
    for (name, target) in table(&value, "targets")? {
        if !identifier(name) {
            return Err(format!("invalid target: {name}"));
        }
        let target = target
            .as_table()
            .ok_or_else(|| format!("target {name} must be a table"))?;
        if let Some(docs) = target.get("docs") {
            crate::docs::validate_mapping(docs, &format!("targets.{name}.docs"))?;
            if !target.contains_key("path") {
                return Err(format!(
                    "target {name}: docs mapping requires a target path"
                ));
            }
        }
        if let Some(selected) = target.get("packages") {
            let selected = string_array(Some(selected), "target packages", false)?;
            if selected.iter().any(|package| !packages.contains(package)) {
                return Err(format!("target {name} selects an uninstalled package"));
            }
        }
        if let Some(path) = target.get("path") {
            let path = path
                .as_str()
                .ok_or_else(|| format!("target {name} path must be a string"))?;
            if !target_paths.insert(path) {
                return Err(format!("duplicate target path: {path}"));
            }
            if path.starts_with('/')
                || path.contains(['\\', ':', '\0'])
                || path.split('/').any(|part| part == "..")
            {
                return Err(format!(
                    "target {name} path must be portable and stay inside its repository"
                ));
            }
        }
    }
    let creative = table_mut(&mut value, "creative")?;
    creative.entry("default_level").or_insert(Value::Integer(2));
    integer_range(
        creative.get("default_level"),
        1,
        4,
        "creative.default_level",
    )?;
    let dispatch = table_mut(&mut value, "dispatch")?;
    dispatch.entry("enabled").or_insert(Value::Boolean(false));
    dispatch.entry("max_workers").or_insert(Value::Integer(5));
    integer_range(dispatch.get("max_workers"), 1, 5, "dispatch.max_workers")?;
    if dispatch.get("enabled").and_then(Value::as_bool).is_none() {
        return Err("dispatch.enabled must be boolean".into());
    }
    reject_datetime(&Value::Table(value.clone()))?;
    Ok(value)
}

fn strings(values: &[&str]) -> Value {
    Value::Array(
        values
            .iter()
            .map(|value| Value::String((*value).into()))
            .collect(),
    )
}

fn table<'a>(value: &'a Table, name: &str) -> Result<&'a Table, String> {
    value
        .get(name)
        .and_then(Value::as_table)
        .ok_or_else(|| format!("{name} must be a table"))
}

fn table_mut<'a>(value: &'a mut Table, name: &str) -> Result<&'a mut Table, String> {
    value
        .get_mut(name)
        .and_then(Value::as_table_mut)
        .ok_or_else(|| format!("{name} must be a table"))
}

fn string_array<'a>(
    value: Option<&'a Value>,
    name: &str,
    nonempty: bool,
) -> Result<Vec<&'a str>, String> {
    let array = value
        .and_then(Value::as_array)
        .ok_or_else(|| format!("{name} must be a string array"))?;
    let values: Vec<&str> = array
        .iter()
        .map(|item| {
            item.as_str()
                .ok_or_else(|| format!("{name} must contain strings"))
        })
        .collect::<Result<_, _>>()?;
    if nonempty && values.is_empty() {
        return Err(format!("{name} must be nonempty"));
    }
    if values.iter().copied().collect::<BTreeSet<_>>().len() != values.len() {
        return Err(format!("{name} contains duplicates"));
    }
    Ok(values)
}

fn identifier(name: &str) -> bool {
    (1..=64).contains(&name.len())
        && name.as_bytes().first().is_some_and(u8::is_ascii_lowercase)
        && name
            .bytes()
            .all(|ch| ch.is_ascii_lowercase() || ch.is_ascii_digit() || ch == b'_' || ch == b'-')
}

fn integer_range(value: Option<&Value>, low: i64, high: i64, name: &str) -> Result<(), String> {
    if value
        .and_then(Value::as_integer)
        .is_some_and(|value| (low..=high).contains(&value))
    {
        Ok(())
    } else {
        Err(format!("{name} must be an integer in {low}..{high}"))
    }
}

fn reject_datetime(value: &Value) -> Result<(), String> {
    match value {
        Value::Datetime(_) => {
            Err("configuration dates must be quoted strings for portable JSON output".into())
        }
        Value::Float(value) if !value.is_finite() => {
            Err("configuration numbers must be finite for portable JSON output".into())
        }
        Value::Array(array) => array.iter().try_for_each(reject_datetime),
        Value::Table(table) => table.values().try_for_each(reject_datetime),
        _ => Ok(()),
    }
}
