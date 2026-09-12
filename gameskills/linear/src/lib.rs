//! Optional Linear adapter. Core GameSkills never requires this crate or its credentials.
pub mod cleanup;
mod process;
pub mod provider;
mod store;
pub mod tracking;

use serde::Deserialize;
use std::path::{Path, PathBuf};

/// Explicit provider scope and optional private export policy.
#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Config {
    /// Exact Linear organization UUID.
    pub workspace: String,
    /// Exact team UUID.
    pub team: String,
    /// Repository default project UUID.
    pub project: String,
    /// Environment variable containing a personal API key.
    #[serde(default = "key_env")]
    pub key_env: String,
    /// Backed-up private directory explicitly designated by its owner.
    pub export_dir: Option<PathBuf>,
    /// Minimum full days since completion.
    #[serde(default = "retention")]
    pub retention_days: u32,
    /// Projects whose history must never be deleted.
    #[serde(default)]
    pub keep_projects: Vec<String>,
    /// Optional game path to project routing. Longest matching prefix wins.
    #[serde(default)]
    pub routes: std::collections::BTreeMap<String, String>,
}
fn key_env() -> String {
    "LINEAR_API_KEY".into()
}
fn retention() -> u32 {
    30
}
impl Config {
    /// Read explicit local configuration; credentials remain outside it.
    pub fn read(path: &Path) -> Result<Self, String> {
        let config: Self =
            toml::from_str(&std::fs::read_to_string(path).map_err(|e| e.to_string())?)
                .map_err(|e| e.to_string())?;
        for id in [&config.workspace, &config.team, &config.project]
            .into_iter()
            .chain(config.keep_projects.iter())
            .chain(config.routes.values())
        {
            uuid(id)?;
        }
        for path in config.routes.keys() {
            if path.is_empty()
                || path.starts_with('/')
                || path.split('/').any(|part| part == ".." || part == ".")
            {
                return Err("routes require repository-relative directory paths".into());
            }
        }
        Ok(config)
    }
    /// Resolve one repository-relative path; mixed scopes remain a planning decision.
    pub fn route(&self, path: &str) -> &str {
        self.routes
            .iter()
            .filter(|(prefix, _)| {
                path == prefix.as_str()
                    || path
                        .strip_prefix(prefix.as_str())
                        .is_some_and(|s| s.starts_with('/'))
            })
            .max_by_key(|(prefix, _)| prefix.len())
            .map_or(self.project.as_str(), |(_, id)| id.as_str())
    }
}
/// Require exact UUIDs rather than mutable names or ticket prefixes.
pub fn uuid(value: &str) -> Result<(), String> {
    if value.len() != 36
        || !value.bytes().enumerate().all(|(i, b)| {
            if [8, 13, 18, 23].contains(&i) {
                b == b'-'
            } else {
                b.is_ascii_hexdigit()
            }
        })
    {
        return Err("expected a canonical UUID".into());
    }
    Ok(())
}
/// Read a required provider string without logging the surrounding private object.
pub fn field<'a>(value: &'a serde_json::Value, name: &str) -> Result<&'a str, String> {
    value
        .get(name)
        .and_then(serde_json::Value::as_str)
        .ok_or_else(|| format!("missing provider field: {name}"))
}

/// Insert into a provider object only after checking its shape.
pub fn put(v: &mut serde_json::Value, key: &str, value: serde_json::Value) -> Result<(), String> {
    v.as_object_mut()
        .ok_or("expected provider object")?
        .insert(key.into(), value);
    Ok(())
}
