//! Validated plans and durable work queues.
use serde_json::Value;
use std::{ffi::OsString, path::Path};
/// Execute a plan or queue command against a ready installation.
pub fn execute(
    _root: &Path,
    _config: &Value,
    _family: &str,
    _args: &[OsString],
) -> Result<Value, String> {
    Err("workflow implementation pending".into())
}
