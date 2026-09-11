//! Command execution and evidence validation.
use serde_json::Value;
use std::{ffi::OsString, path::Path};
/// Execute a run or evidence command against a ready installation.
pub fn execute(
    _root: &Path,
    _config: &Value,
    _family: &str,
    _args: &[OsString],
) -> Result<Value, String> {
    Err("runner implementation pending".into())
}
