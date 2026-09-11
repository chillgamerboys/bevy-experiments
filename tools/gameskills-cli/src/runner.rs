//! Local command observations with protected state and supervised process groups.
//!
//! Records establish local workflow integrity, not attestation or review approval.
//! Historical Python records can be inspected but never validated or resumed.
use serde_json::Value;
use std::{ffi::OsString, path::Path};
#[cfg(unix)]
mod graph;
#[cfg(unix)]
mod identity;
#[cfg(unix)]
mod process;
#[cfg(unix)]
mod run;
#[cfg(unix)]
mod state;

/// Execute `run` or `evidence list|show|validate` against a ready installation.
pub fn execute(
    root: &Path,
    config: &Value,
    family: &str,
    args: &[OsString],
) -> Result<Value, String> {
    #[cfg(unix)]
    {
        run::execute(
            root,
            config,
            family,
            args,
            &std::env::current_exe().map_err(|e| e.to_string())?,
        )
    }
    #[cfg(not(unix))]
    {
        let _ = (root, config, family, args);
        Err("Windows runner/evidence is unsupported: a verified process-tree and protected-state backend is required".into())
    }
}
/// Enter the private supervisor through the inherited Unix descriptor handshake.
/// The CLI routes `__runner-supervisor` here before installation validation.
/// No command is accepted through arguments or an unbound state path.
pub fn supervisor(args: &[OsString]) -> Result<Value, String> {
    #[cfg(unix)]
    {
        process::supervisor(args)
    }
    #[cfg(not(unix))]
    {
        let _ = args;
        Err("Windows runner supervisor is unsupported".into())
    }
}
#[cfg(unix)]
fn hash(bytes: impl AsRef<[u8]>) -> String {
    use sha2::{Digest, Sha256};
    format!("{:x}", Sha256::digest(bytes.as_ref()))
}
#[cfg(unix)]
fn digest(value: &impl serde::Serialize) -> Result<String, String> {
    let mut value = serde_json::to_value(value).map_err(|e| e.to_string())?;
    value.sort_all_objects();
    serde_json::to_vec(&value)
        .map(hash)
        .map_err(|e| e.to_string())
}
#[cfg(unix)]
fn now() -> f64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs_f64()
}
#[cfg(unix)]
fn identifier() -> String {
    use std::sync::atomic::{AtomicU64, Ordering};
    static SEQUENCE: AtomicU64 = AtomicU64::new(0);
    hash(format!(
        "{:?}:{}:{}",
        std::time::SystemTime::now(),
        std::process::id(),
        SEQUENCE.fetch_add(1, Ordering::Relaxed)
    ))
    .chars()
    .take(32)
    .collect()
}
#[cfg(unix)]
fn run_id(value: &str) -> bool {
    value.len() == 32
        && value
            .bytes()
            .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
}

#[cfg(all(test, unix))]
mod tests {
    #[test]
    fn canonical_digest_ignores_nested_object_insertion_order() -> Result<(), String> {
        let first: serde_json::Value =
            serde_json::from_str(r#"{"z":{"b":2,"a":1},"a":0}"#).map_err(|e| e.to_string())?;
        let second: serde_json::Value =
            serde_json::from_str(r#"{"a":0,"z":{"a":1,"b":2}}"#).map_err(|e| e.to_string())?;
        assert_eq!(super::digest(&first)?, super::digest(&second)?);
        assert_ne!(
            super::digest(&first)?,
            super::digest(&serde_json::json!({"a":0,"z":{"a":2,"b":1}}))?
        );
        Ok(())
    }
}
