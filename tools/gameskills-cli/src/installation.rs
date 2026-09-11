//! Immutable instruction installation and explicitly scoped native integration.
mod archive;
mod files;
mod legacy;
mod native;
mod setup;

use serde_json::{json, Value};
use std::{
    ffi::OsString,
    path::{Path, PathBuf},
};

pub use archive::{verify_archive, InstructionBundle};
pub use native::{activate_codex, native_argv};

#[cfg(unix)]
use std::collections::BTreeMap;

const CONFIG: &str = "gameskills.toml";
const LOCK: &str = "gameskills.lock.json";
const JOURNAL: &str = ".gameskills/setup-transaction.json";
const LIMIT: u64 = 64 * 1024 * 1024;

/// Execute an installation command without a source checkout or interpreter.
pub fn execute(root: &Path, family: &str, args: &[OsString]) -> Result<Value, String> {
    let args = args
        .iter()
        .map(|s| {
            s.to_str()
                .map(str::to_owned)
                .ok_or("arguments must be UTF-8".to_owned())
        })
        .collect::<Result<Vec<_>, _>>()?;
    match family {
        "catalog" if args.is_empty() => Ok(archive::embedded()?.catalog),
        "config" if args.is_empty() => {
            ready_config(root).map(|config| json!({"ok":true,"configuration":config}))
        }
        "status" if args.is_empty() => status(root),
        "setup" => setup::execute(root, &args),
        "bundle" => archive::export(root, &args),
        "native" => native::execute(root, &args),
        "legacy" => legacy::execute(root, &args),
        _ => Err(format!("unsupported {family} arguments")),
    }
}

/// Validate installation readiness and return normalized configuration.
pub fn ready_config(root: &Path) -> Result<Value, String> {
    installed(root).map(|(_, config, _)| config)
}

fn normalized(text: &str) -> Result<Value, String> {
    serde_json::to_value(crate::config::parse(text)?).map_err(|e| e.to_string())
}

fn strings(value: &Value, field: &str) -> Result<Vec<String>, String> {
    value
        .get(field)
        .and_then(Value::as_array)
        .ok_or_else(|| format!("{field} must be an array"))?
        .iter()
        .map(|item| {
            item.as_str()
                .map(str::to_owned)
                .ok_or_else(|| format!("{field} must contain strings"))
        })
        .collect()
}

fn installed(root: &Path) -> Result<(InstructionBundle, Value, PathBuf), String> {
    let directory = files::Directory::open(root)?;
    if directory.read_optional(JOURNAL)?.is_some() {
        return Err("interrupted setup; use setup --recover".into());
    }
    let config = normalized(&directory.text(CONFIG)?)?;
    let lock = archive::json(&directory.read(LOCK)?)?;
    if lock.get("schema_version") != Some(&json!(2)) || lock.get("runtime") != Some(&json!("rust"))
    {
        return Err(
            "historical installation lock; use explicit Rust setup after all Python queues finish"
                .into(),
        );
    }
    exact_keys(
        &lock,
        &[
            "schema_version",
            "runtime",
            "bundle",
            "content_sha256",
            "packages",
        ],
    )?;
    let selected = strings(&lock, "packages")?;
    let content = lock
        .get("content_sha256")
        .and_then(Value::as_str)
        .ok_or("lock lacks content identity")?;
    let identity = archive::installation_id(content, &selected)?;
    let relative = format!(".gameskills/bundles/{identity}");
    if lock.get("bundle").and_then(Value::as_str) != Some(relative.as_str()) {
        return Err("invalid bundle location in lock".into());
    }
    let bundle = archive::read_installed(&directory, &relative)?;
    if bundle.identity() != content || bundle.selected != selected {
        return Err("installation lock and bundle disagree".into());
    }
    if strings(&config, "packages")?
        .into_iter()
        .collect::<std::collections::BTreeSet<_>>()
        != selected.iter().cloned().collect()
    {
        return Err("package selection changed; use explicit setup".into());
    }
    Ok((bundle, config, root.join(relative)))
}

fn status(root: &Path) -> Result<Value, String> {
    let (bundle, config, path) = installed(root)?;
    Ok(
        json!({"ok":true,"schema_version":2,"runtime":"rust","cli_version":env!("CARGO_PKG_VERSION"),"content_sha256":bundle.identity(),"source_commit":bundle.manifest.get("source_commit"),"version":bundle.manifest.get("catalog_version"),"packages":bundle.selected,"clients":config.get("clients"),"creative_level":config.pointer("/creative/default_level"),"max_workers":config.pointer("/dispatch/max_workers"),"bundle":path,"native_activation":"not inferred from local package staging"}),
    )
}

fn exact_keys(value: &Value, keys: &[&str]) -> Result<(), String> {
    let map = value.as_object().ok_or("expected JSON object")?;
    if map.len() != keys.len() || !keys.iter().all(|key| map.contains_key(*key)) {
        return Err("unexpected or missing identity fields".into());
    }
    Ok(())
}

fn sorted_bytes(value: &Value) -> Result<Vec<u8>, String> {
    let mut value = value.clone();
    value.sort_all_objects();
    let mut bytes = serde_json::to_vec_pretty(&value).map_err(|e| e.to_string())?;
    bytes.push(b'\n');
    Ok(bytes)
}

fn hash(bytes: &[u8]) -> String {
    use sha2::{Digest, Sha256};
    format!("{:x}", Sha256::digest(bytes))
}

#[cfg(unix)]
fn package_files(bundle: &InstructionBundle, package: &str) -> BTreeMap<String, String> {
    let prefix = format!("plugins/{package}/");
    bundle
        .files
        .iter()
        .filter_map(|(name, bytes)| {
            name.strip_prefix(&prefix)
                .map(|relative| (relative.to_owned(), hash(bytes)))
        })
        .collect()
}

/// Serialize installation changes with new run registration.
///
/// Hold until a new run's active lock exists; existing active locks then keep
/// setup from changing the installation while a supervisor is alive.
pub fn lifecycle_guard(root: &Path) -> Result<std::fs::File, String> {
    files::Directory::open(root)?.lock(".gameskills/setup.lock")
}
