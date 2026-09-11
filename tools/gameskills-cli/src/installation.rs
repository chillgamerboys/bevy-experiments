//! Installation, immutable instructions and native client integration.
use serde_json::Value;
use std::{ffi::OsString, path::Path};
/// Execute an installation command.
pub fn execute(_root: &Path, _family: &str, _args: &[OsString]) -> Result<Value, String> {
    Err("installation implementation pending".into())
}
/// Validate installation readiness and return normalized configuration.
pub fn ready_config(_root: &Path) -> Result<Value, String> {
    Err("installation implementation pending".into())
}
