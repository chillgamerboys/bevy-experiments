//! Read-only adopter documentation discovery, independent of plugin installation.
use serde_json::{json, Value as Json};
use std::{
    collections::BTreeSet,
    ffi::OsString,
    path::{Component, Path, PathBuf},
};
use toml::{Table, Value};

pub(crate) fn validate_mapping(value: &Value, label: &str) -> Result<(), String> {
    let table = value
        .as_table()
        .ok_or_else(|| format!("{label} must be a table"))?;
    for (key, value) in table {
        if !["index", "plans"].contains(&key.as_str()) {
            return Err(format!("unknown documentation field: {label}.{key}"));
        }
        let path = value
            .as_str()
            .ok_or_else(|| format!("{label}.{key} must be a path string"))?;
        portable(path, false).map_err(|error| format!("{label}.{key}: {error}"))?;
    }
    Ok(())
}

fn portable(raw: &str, allow_dot: bool) -> Result<PathBuf, String> {
    if raw.is_empty()
        || raw.starts_with('/')
        || raw.contains(['\\', ':', '\0', '#', '?'])
        || raw
            .split('/')
            .any(|part| part == ".." || (!allow_dot && (part.is_empty() || part == ".")))
    {
        return Err(format!(
            "expected a portable repository-relative path: {raw:?}"
        ));
    }
    Ok(Path::new(raw)
        .components()
        .filter(|c| *c != Component::CurDir)
        .collect())
}

// Check existing ancestors too, including when a proposed file/directory is absent.
fn contained(root: &Path, relative: &Path) -> Result<PathBuf, String> {
    let mut path = root.to_path_buf();
    for component in relative.components() {
        path.push(component);
        match std::fs::symlink_metadata(&path) {
            Ok(_) => {
                path = path.canonicalize().map_err(|e| e.to_string())?;
                if !path.starts_with(root) {
                    return Err(format!(
                        "documentation path escapes repository: {}",
                        relative.display()
                    ));
                }
            }
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => {}
            Err(e) => return Err(e.to_string()),
        }
    }
    Ok(path)
}

struct Owner {
    name: String,
    source: PathBuf,
    index: Option<PathBuf>,
    plans: PathBuf,
    configured_index: bool,
    configured_plans: bool,
}
impl Owner {
    fn load(
        root: &Path,
        name: &str,
        source: PathBuf,
        mapping: Option<&Value>,
    ) -> Result<Self, String> {
        let mapping = mapping.and_then(Value::as_table);
        let configured = |key| mapping.and_then(|m| m.get(key)).and_then(Value::as_str);
        let index = if let Some(path) = configured("index") {
            let relative = portable(path, false)?;
            if !contained(root, &relative)?.is_file() {
                return Err(format!(
                    "{name}: configured documentation index is missing or not a file: {path}"
                ));
            }
            Some(relative)
        } else {
            let mut found = None;
            for candidate in [source.join("docs/README.md"), source.join("README.md")] {
                if contained(root, &candidate)?.is_file() {
                    found = Some(candidate);
                    break;
                }
            }
            found
        };
        let plans = configured("plans")
            .map(|p| portable(p, false))
            .transpose()?
            .unwrap_or_else(|| source.join("docs/plans"));
        let destination = contained(root, &plans)?;
        if destination.exists() && !destination.is_dir() {
            return Err(format!("{name}: plans location is not a directory"));
        }
        Ok(Self {
            name: name.into(),
            source,
            index,
            plans,
            configured_index: configured("index").is_some(),
            configured_plans: configured("plans").is_some(),
        })
    }
    fn json(&self) -> Json {
        let path = |p: &Path| p.to_string_lossy().replace('\\', "/");
        json!({"owner":self.name,"source":path(&self.source),"index":self.index.as_deref().map(path),"index_origin":if self.configured_index {"configured"} else {"conventional"},"plans":path(&self.plans),"plans_origin":if self.configured_plans {"configured"} else {"conventional"}})
    }
    fn score(&self, path: &Path) -> Option<usize> {
        let mut roots = vec![self.source.as_path()];
        if self.configured_index {
            if let Some(index) = &self.index {
                if path == index {
                    return Some(usize::MAX);
                }
                // A root README maps that file, not the entire repository.
                if let Some(parent) = index.parent().filter(|p| !p.as_os_str().is_empty()) {
                    roots.push(parent);
                }
            }
        }
        if self.configured_plans {
            roots.push(&self.plans);
        }
        roots
            .into_iter()
            .filter(|p| path.starts_with(p))
            .map(|p| p.components().count())
            .max()
    }
}

/// Resolve the root and affected owners. No document content or external provider is loaded.
pub fn resolve(root: &Path, config: &Table, paths: &[String]) -> Result<Json, String> {
    let root = root.canonicalize().map_err(|e| e.to_string())?;
    let mut owners = vec![Owner::load(
        &root,
        "root",
        PathBuf::new(),
        config.get("docs"),
    )?];
    if let Some(targets) = config.get("targets").and_then(Value::as_table) {
        for (name, value) in targets {
            let Some(source) = value.get("path").and_then(Value::as_str) else {
                if value.get("docs").is_some() {
                    return Err(format!(
                        "target {name}: docs mapping requires a target path"
                    ));
                }
                continue;
            };
            let source = portable(source, true)?;
            contained(&root, &source)?;
            owners.push(Owner::load(&root, name, source, value.get("docs"))?);
        }
    }
    let mut selected = BTreeSet::from([0]);
    for raw in paths {
        let path = portable(raw, true)?;
        contained(&root, &path)?;
        let candidates: Vec<_> = owners
            .iter()
            .enumerate()
            .skip(1)
            .filter_map(|(i, o)| o.score(&path).map(|score| (i, score)))
            .collect();
        if let Some(best) = candidates.iter().map(|(_, score)| score).max() {
            let matches: Vec<_> = candidates
                .iter()
                .filter(|(_, score)| score == best)
                .collect();
            if matches.len() != 1 {
                return Err(format!(
                    "ambiguous documentation ownership for {raw}: {}",
                    matches
                        .iter()
                        .filter_map(|(i, _)| owners.get(*i).map(|o| o.name.as_str()))
                        .collect::<Vec<_>>()
                        .join(", ")
                ));
            }
            if let Some((i, _)) = matches.first() {
                selected.insert(*i);
            }
        }
    }
    let selected: Vec<_> = selected.iter().filter_map(|i| owners.get(*i)).collect();
    let mut seen = BTreeSet::new();
    let indexes: Vec<_> = selected
        .iter()
        .filter_map(|o| o.index.as_ref())
        .filter(|p| seen.insert((*p).clone()))
        .map(|p| p.to_string_lossy().replace('\\', "/"))
        .collect();
    let missing: Vec<_> = selected
        .iter()
        .filter(|o| o.index.is_none())
        .map(|o| format!("{}: no conventional documentation index found", o.name))
        .collect();
    Ok(
        json!({"schema_version":1,"ok":true,"claim":"documentation locations only; content and behavior not verified","owners":selected.iter().map(|o|o.json()).collect::<Vec<_>>(),"indexes":indexes,"diagnostics":missing}),
    )
}

pub(crate) fn execute(root: &Path, args: &[OsString]) -> Result<Json, String> {
    if args.first().is_none_or(|s| s != "resolve") {
        return Err("use docs resolve [--path PATH]...".into());
    }
    let mut paths = Vec::new();
    let mut rest = args.iter().skip(1);
    while let Some(flag) = rest.next() {
        if flag != "--path" {
            return Err("use docs resolve [--path PATH]...".into());
        }
        paths.push(
            rest.next()
                .and_then(|s| s.to_str())
                .ok_or("--path requires a UTF-8 repository-relative path")?
                .into(),
        );
    }
    let config_path = root.join("gameskills.toml");
    let config = if config_path.exists() {
        crate::config::parse(
            &crate::platform::read_ordinary_file(&config_path).map_err(|e| e.to_string())?,
        )?
    } else {
        crate::config::parse("schema_version = 1")?
    };
    resolve(root, &config, &paths)
}
