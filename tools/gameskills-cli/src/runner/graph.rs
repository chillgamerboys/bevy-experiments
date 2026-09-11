use super::state::Directory;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub(super) struct Spec {
    pub(super) argv: Vec<String>,
    pub(super) cwd: String,
    pub(super) requires: Vec<String>,
    pub(super) resources: Vec<String>,
    pub(super) timeout_seconds: f64,
}
pub(super) fn name(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 64
        && value
            .bytes()
            .next()
            .is_some_and(|c| c.is_ascii_alphanumeric())
        && value
            .bytes()
            .all(|c| c.is_ascii_alphanumeric() || b"_.-".contains(&c))
}
pub(super) fn strings(value: Option<&Value>, label: &str) -> Result<Vec<String>, String> {
    let Some(value) = value else {
        return Ok(Vec::new());
    };
    let mut values = Vec::new();
    for value in value
        .as_array()
        .ok_or_else(|| format!("{label} must be a list"))?
    {
        values.push(
            value
                .as_str()
                .filter(|s| !s.is_empty() && !s.contains('\0'))
                .ok_or_else(|| format!("{label} must contain nonempty strings"))?
                .to_owned(),
        );
    }
    if values.iter().collect::<BTreeSet<_>>().len() != values.len() {
        return Err(format!("{label} contains duplicates"));
    }
    values.sort();
    Ok(values)
}
pub(super) fn number(value: &Value, label: &str, minimum: f64) -> Result<f64, String> {
    value
        .as_f64()
        .filter(|n| n.is_finite() && *n >= minimum)
        .ok_or_else(|| format!("{label} must be a finite number >= {minimum}"))
}
pub(super) fn relative(value: &str) -> Result<PathBuf, String> {
    if value.is_empty() || value.contains(['\\', ':', '\0']) {
        return Err("command cwd must be repository-relative".into());
    }
    let path = Path::new(value);
    for component in path.components() {
        match component {
            std::path::Component::CurDir => (),
            std::path::Component::Normal(name) if name != ".git" && name != ".gameskills" => (),
            _ => return Err("command cwd must stay inside repository outside metadata".into()),
        }
    }
    Ok(path.to_path_buf())
}
pub(super) fn graph(
    root: &Path,
    config: &Value,
    selected: &[String],
) -> Result<(BTreeMap<String, Spec>, Vec<String>), String> {
    let configured = config
        .get("commands")
        .and_then(Value::as_object)
        .ok_or("commands must be a mapping")?;
    let mut specs = BTreeMap::new();
    let mut order = Vec::new();
    let mut visiting = BTreeSet::new();
    fn visit(
        root: &Path,
        configured: &serde_json::Map<String, Value>,
        selected: &str,
        specs: &mut BTreeMap<String, Spec>,
        order: &mut Vec<String>,
        visiting: &mut BTreeSet<String>,
    ) -> Result<(), String> {
        if !name(selected) {
            return Err(format!("invalid command name: {selected}"));
        }
        if visiting.contains(selected) {
            return Err(format!("command dependency cycle at {selected}"));
        }
        if specs.contains_key(selected) {
            return Ok(());
        }
        let command = configured
            .get(selected)
            .and_then(Value::as_object)
            .ok_or_else(|| format!("unknown command: {selected}"))?;
        let argv = command
            .get("argv")
            .and_then(Value::as_array)
            .ok_or("argv must be a nonempty list of strings")?
            .iter()
            .map(|a| {
                a.as_str()
                    .filter(|s| !s.contains('\0'))
                    .map(str::to_owned)
                    .ok_or("argv must contain strings without NUL".to_owned())
            })
            .collect::<Result<Vec<_>, _>>()?;
        if argv.first().is_none_or(|a| a.is_empty()) {
            return Err("argv must be a nonempty list of strings".into());
        }
        let directory = command
            .get("cwd")
            .map(|v| v.as_str().ok_or("cwd must be a string"))
            .transpose()?
            .unwrap_or(".");
        let relative = relative(directory)?;
        Directory::cwd(root, &relative)?;
        let requires = strings(command.get("requires"), "requires")?;
        let resources = strings(command.get("resources"), "resources")?;
        if resources.len() > 200 {
            return Err("at most 200 resource locks are supported per command".into());
        }
        let timeout_seconds = number(
            command.get("timeout_seconds").unwrap_or(&Value::from(600)),
            "timeout_seconds",
            0.001,
        )?;
        visiting.insert(selected.into());
        for dependency in &requires {
            visit(root, configured, dependency, specs, order, visiting)?;
        }
        visiting.remove(selected);
        specs.insert(
            selected.into(),
            Spec {
                argv,
                cwd: relative.to_string_lossy().into_owned(),
                requires,
                resources,
                timeout_seconds,
            },
        );
        order.push(selected.into());
        Ok(())
    }
    for selected in selected {
        visit(
            root,
            configured,
            selected,
            &mut specs,
            &mut order,
            &mut visiting,
        )?;
    }
    Ok((specs, order))
}
