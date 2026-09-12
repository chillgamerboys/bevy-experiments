//! Bounded official GraphQL transport and fail-closed pagination.
use serde_json::{json, Value};
use std::process::Command;

/// Test seam for the official GraphQL API. Mutations are never automatically retried.
pub trait Provider {
    /// Execute a GraphQL operation, rejecting errors even with partial data.
    fn query(&mut self, query: &str, variables: Value) -> Result<Value, String>;
}
/// Personal-key transport. Secrets are sent through private stdin, never argv or logs.
pub struct Linear {
    key: String,
}
impl Linear {
    /// Load only the explicitly selected environment variable.
    pub fn from_env(name: &str) -> Result<Self, String> {
        let key =
            std::env::var(name).map_err(|_| format!("set {name} to a Linear personal API key"))?;
        if key.is_empty() || key.contains(['\r', '\n', '\0']) {
            return Err("invalid API key".into());
        }
        Ok(Self { key })
    }
}
impl Provider for Linear {
    fn query(&mut self, query: &str, variables: Value) -> Result<Value, String> {
        let body = json!({"query":query,"variables":variables}).to_string();
        // JSON string escaping is compatible with curl's quoted config values.
        let config=format!("url = \"https://api.linear.app/graphql\"\nheader = {}\nheader = \"Content-Type: application/json\"\ndata = {}\n",json!(format!("Authorization: {}",self.key)),json!(body));
        let bytes = crate::process::provider_output(
            Command::new("curl").args([
                "--disable",
                "--silent",
                "--show-error",
                "--fail-with-body",
                "--max-time",
                "40",
                "--proto",
                "=https",
                "--config",
                "-",
            ]),
            Some(config.as_bytes()),
        )?;
        decode(&bytes)
    }
}
/// Reject partial success and classify only explicit structured quota/rate-limit codes.
pub fn decode(bytes: &[u8]) -> Result<Value, String> {
    let v: Value = serde_json::from_slice(bytes).map_err(|_| "invalid provider JSON")?;
    if v.get("errors")
        .is_some_and(|e| !e.is_null() && !e.is_array())
    {
        return Err("malformed GraphQL errors".into());
    }
    if let Some(errors) = v
        .get("errors")
        .and_then(Value::as_array)
        .filter(|e| !e.is_empty())
    {
        // Never classify quota from a generic message containing 'limit'. Unknown codes block.
        let codes: Vec<_> = errors
            .iter()
            .filter_map(|e| e.pointer("/extensions/code").and_then(Value::as_str))
            .collect();
        if codes.contains(&"RATELIMITED") {
            return Err("rate limited; retry this invocation after the provider reset".into());
        }
        return Err("GraphQL operation failed; no success inferred and no mutation retried".into());
    }
    v.get("data")
        .filter(|v| v.is_object())
        .cloned()
        .ok_or_else(|| "missing provider data".into())
}
/// Read every page, detect cursor loops and duplicate/missing identities.
pub fn pages(
    provider: &mut dyn Provider,
    query: &str,
    mut variables: Value,
    pointer: &str,
) -> Result<Vec<Value>, String> {
    let mut out = Vec::new();
    let mut cursors = std::collections::BTreeSet::new();
    let mut ids = std::collections::BTreeSet::new();
    for _ in 0..1000 {
        let v = provider.query(query, variables.clone())?;
        let c = v.pointer(pointer).ok_or("missing connection")?;
        for node in c
            .get("nodes")
            .and_then(Value::as_array)
            .ok_or("missing connection nodes")?
        {
            if !ids.insert(crate::field(node, "id")?.to_owned()) {
                return Err("duplicate identity during pagination; retry fresh preview".into());
            }
            out.push(node.clone());
        }
        match c.pointer("/pageInfo/hasNextPage").and_then(Value::as_bool) {
            Some(false) => return Ok(out),
            Some(true) => {}
            None => return Err("missing pagination state".into()),
        }
        let cursor = c
            .pointer("/pageInfo/endCursor")
            .and_then(Value::as_str)
            .filter(|s| !s.is_empty())
            .ok_or("missing next cursor")?;
        if !cursors.insert(cursor.to_owned()) {
            return Err("pagination cursor loop".into());
        }
        crate::put(&mut variables, "after", json!(cursor))?;
    }
    Err("pagination safety bound exceeded".into())
}
/// Confirm that credentials address the configured organization and team.
pub fn scope(
    provider: &mut dyn Provider,
    config: &crate::Config,
    project: &str,
) -> Result<Value, String> {
    crate::uuid(project)?;
    let v=provider.query("query($team:String!,$project:String!){organization{id} team(id:$team){id} project(id:$project){id name}}",json!({"team":config.team,"project":project}))?;
    if v.pointer("/organization/id").and_then(Value::as_str) != Some(&config.workspace)
        || v.pointer("/team/id").and_then(Value::as_str) != Some(&config.team)
        || v.pointer("/project/id").and_then(Value::as_str) != Some(project)
    {
        return Err("provider workspace/team/project does not match configured scope".into());
    }
    Ok(v)
}
