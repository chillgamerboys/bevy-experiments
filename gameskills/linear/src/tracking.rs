//! Exact issue/PR linking and bounded, stable-identity issue creation.
use crate::{
    field,
    provider::{self, Provider},
    Config,
};
use serde_json::{json, Value};
use std::process::Command;
fn issue(p: &mut dyn Provider, c: &Config, id: &str, project: &str) -> Result<Value, String> {
    crate::uuid(id)?;
    provider::scope(p, c, project)?;
    let v=p.query("query($id:String!){issue(id:$id){id identifier title url description project{id} team{id} state{id type}}}",json!({"id":id}))?.get("issue").cloned().ok_or("issue unavailable")?;
    if v.get("id").and_then(Value::as_str) != Some(id)
        || v.pointer("/project/id").and_then(Value::as_str) != Some(project)
        || v.pointer("/team/id").and_then(Value::as_str) != Some(&c.team)
    {
        return Err("issue scope mismatch".into());
    }
    Ok(v)
}
fn pr(url: &str) -> Result<Value, String> {
    let pattern =
        regex::Regex::new(r"^https://github\.com/[A-Za-z0-9_.-]+/[A-Za-z0-9_.-]+/pull/[0-9]+$")
            .map_err(|e| e.to_string())?;
    if !pattern.is_match(url) {
        return Err("expected a GitHub PR URL".into());
    }
    let bytes = crate::process::provider_output(
        Command::new("gh").args([
            "pr",
            "view",
            url,
            "--json",
            "url,body,state,mergeCommit,baseRefName,headRefOid",
        ]),
        None,
    )?;
    let v: Value = serde_json::from_slice(&bytes).map_err(|_| "invalid GitHub observation")?;
    if v.get("url").and_then(Value::as_str) != Some(url) {
        return Err("GitHub returned another PR".into());
    }
    Ok(v)
}
/// Verify the two-way link without modifying either object.
pub fn observe(
    p: &mut dyn Provider,
    c: &Config,
    id: &str,
    project: &str,
    url: &str,
) -> Result<Value, String> {
    let issue = issue(p, c, id, project)?;
    let attachments=provider::pages(p,"query($id:String!,$after:String){issue(id:$id){attachments(first:100,after:$after,includeArchived:true){nodes{id url} pageInfo{hasNextPage endCursor}}}}",json!({"id":id,"after":null}),"/issue/attachments")?;
    let remote = pr(url)?;
    let linked = attachments
        .iter()
        .any(|a| a.get("url").and_then(Value::as_str) == Some(url))
        && field(&remote, "body")?.contains(field(&issue, "url")?);
    Ok(
        json!({"ok":linked,"linked":linked,"issue_id":id,"project_id":project,"pr_url":url,"issue_state":issue.get("state"),"pr_state":remote.get("state")}),
    )
}
/// Idempotently attach the PR and add the issue URL to the PR body, then reread both.
pub fn link(
    p: &mut dyn Provider,
    c: &Config,
    id: &str,
    project: &str,
    url: &str,
) -> Result<Value, String> {
    let issue = issue(p, c, id, project)?;
    let remote = pr(url)?;
    let response=p.query("mutation($input:AttachmentCreateInput!){attachmentCreate(input:$input){success attachment{id url}}}",json!({"input":{"issueId":id,"url":url,"title":"Implementation PR"}}))?;
    if response.pointer("/attachmentCreate/success") != Some(&json!(true)) {
        return Err("PR attachment not confirmed; observe before retrying".into());
    }
    let issue_url = field(&issue, "url")?;
    let body = field(&remote, "body")?;
    if !body.contains(issue_url) {
        use std::io::Write;
        let mut file = tempfile::NamedTempFile::new().map_err(|e| e.to_string())?;
        writeln!(file, "{body}\n\nTracking: {issue_url}").map_err(|e| e.to_string())?;
        crate::process::provider_output(
            Command::new("gh")
                .args(["pr", "edit", url, "--body-file"])
                .arg(file.path()),
            None,
        )?;
    }
    observe(p, c, id, project, url)
}
/// Create using a caller-persisted UUID; a retry first reconciles that exact ID.
pub fn create(
    p: &mut dyn Provider,
    c: &Config,
    id: &str,
    project: &str,
    title: &str,
    description: &str,
) -> Result<Value, String> {
    crate::uuid(id)?;
    provider::scope(p, c, project)?;
    if title.trim().is_empty() {
        return Err("nonempty title required".into());
    }
    let found=provider::pages(p,"query($id:ID!,$after:String){issues(first:100,after:$after,includeArchived:true,filter:{id:{eq:$id}}){nodes{id project{id} team{id} url} pageInfo{hasNextPage endCursor}}}",json!({"id":id,"after":null}),"/issues")?;
    if !found.is_empty() {
        return issue(p, c, id, project);
    }
    let result=p.query("mutation($input:IssueCreateInput!){issueCreate(input:$input){success issue{id url identifier project{id} team{id}}}}",json!({"input":{"id":id,"teamId":c.team,"projectId":project,"title":title,"description":description}}))?;
    if result.pointer("/issueCreate/success") != Some(&json!(true)) {
        return Err("creation unconfirmed; query this stable UUID before retrying".into());
    }
    issue(p, c, id, project)
}
/// Complete only after every caller-declared required PR is linked and merged.
/// This observes provider integration; human acceptance and task scope stay with the caller.
pub fn complete(
    p: &mut dyn Provider,
    c: &Config,
    id: &str,
    project: &str,
    state: &str,
    prs: &[String],
) -> Result<Value, String> {
    crate::uuid(state)?;
    issue(p, c, id, project)?;
    if prs.is_empty() {
        return Err("declare all required PRs before completing an issue".into());
    }
    for url in prs {
        if observe(p, c, id, project, url)?.get("ok") != Some(&json!(true)) {
            return Err("required PR link unavailable".into());
        }
        let remote = pr(url)?;
        if remote.get("state").and_then(Value::as_str) != Some("MERGED")
            || remote
                .pointer("/mergeCommit/oid")
                .and_then(Value::as_str)
                .is_none()
        {
            return Err("required PR is not merged".into());
        }
    }
    let s = p.query(
        "query($id:String!){workflowState(id:$id){id type team{id}}}",
        json!({"id":state}),
    )?;
    if s.pointer("/workflowState/type").and_then(Value::as_str) != Some("completed")
        || s.pointer("/workflowState/team/id").and_then(Value::as_str) != Some(&c.team)
    {
        return Err("state is not completed in this team".into());
    }
    let r=p.query("mutation($id:String!,$input:IssueUpdateInput!){issueUpdate(id:$id,input:$input){success issue{id state{id type}}}}",json!({"id":id,"input":{"stateId":state}}))?;
    if r.pointer("/issueUpdate/success") != Some(&json!(true)) {
        return Err("completion not confirmed".into());
    }
    issue(p, c, id, project)
}
