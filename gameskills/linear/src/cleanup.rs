//! Explicitly scoped manual cleanup with verified exports and recoverable deletion.
use crate::{
    field,
    provider::{self, Provider},
    store::Store,
    Config,
};
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use std::path::Path;
use time::{format_description::well_known::Rfc3339, OffsetDateTime};
const BASIC:&str="id identifier title description createdAt updatedAt completedAt canceledAt trashed url priority estimate labelIds state{id name type} team{id name} project{id name} parent{id identifier state{id type}}";
const PAGE: &str = "pageInfo{hasNextPage endCursor}";
/// Options for one invocation; retention never installs a schedule.
pub struct Options<'a> {
    /// Exact project UUID.
    pub project: &'a str,
    /// Minimum full completed days.
    pub retention_days: u32,
    /// Explicit mutation bound; preview may list all candidates.
    pub limit: usize,
    /// Apply the currently authorized sweep.
    pub apply: bool,
    /// User-designated backed-up private directory.
    pub export_dir: Option<&'a Path>,
    /// UTC instant, injectable for boundary tests.
    pub now: OffsetDateTime,
}
fn instant(value: &str) -> Result<OffsetDateTime, String> {
    OffsetDateTime::parse(value, &Rfc3339)
        .map_err(|_| "invalid completion/history timestamp".into())
}
/// Fetch the complete ticket and all relationships, discussion and document records.
pub fn snapshot(p: &mut dyn Provider, id: &str) -> Result<Value, String> {
    let mut v = p
        .query(
            &format!("query($id:String!){{issue(id:$id){{{BASIC}}}}}"),
            json!({"id":id}),
        )?
        .get("issue")
        .filter(|v| v.is_object())
        .cloned()
        .ok_or("issue unavailable; deletion is not inferred")?;
    for (connection,fields) in [
        ("children","id identifier title state{id type}"),
        ("relations","id type updatedAt issue{id identifier state{id type}} relatedIssue{id identifier state{id type}}"),
        ("inverseRelations","id type updatedAt issue{id identifier state{id type}} relatedIssue{id identifier state{id type}}"),
        ("comments","id body createdAt updatedAt user{id name}"),
        ("attachments","id title subtitle url metadata source bodyData createdAt updatedAt"),
        ("documents","id title content url createdAt updatedAt"),
        ("history","id createdAt updatedAt fromState{id type} toState{id type} fromStateId toStateId changes"),
    ] {
        let q=format!("query($id:String!,$after:String){{issue(id:$id){{{connection}(first:100,after:$after,includeArchived:true){{nodes{{{fields}}} {PAGE}}}}}}}");
        crate::put(&mut v,connection,json!(provider::pages(p,&q,json!({"id":id,"after":null}),&format!("/issue/{connection}"))?))?;
    }
    // Document discussions are part of their export, not merely document links.
    for doc in v
        .get_mut("documents")
        .and_then(Value::as_array_mut)
        .ok_or("missing documents")?
    {
        let q=format!("query($id:String!,$after:String){{document(id:$id){{comments(first:100,after:$after,includeArchived:true){{nodes{{id body createdAt updatedAt user{{id name}}}} {PAGE}}}}}}}");
        let comments = json!(provider::pages(
            p,
            &q,
            json!({"id":field(doc,"id")?,"after":null}),
            "/document/comments"
        )?);
        crate::put(doc, "comments", comments)?;
    }
    Ok(v)
}
fn done(v: &Value) -> bool {
    v.pointer("/state/type").and_then(Value::as_str) == Some("completed")
}
/// Explain why a full current snapshot is ineligible; never use updatedAt as age.
pub fn eligibility(v: &Value, c: &Config, o: &Options<'_>) -> Result<(), String> {
    if v.pointer("/project/id").and_then(Value::as_str) != Some(o.project)
        || v.pointer("/team/id").and_then(Value::as_str) != Some(&c.team)
    {
        return Err("outside exact project/team scope".into());
    }
    if c.keep_projects.iter().any(|id| id == o.project) {
        return Err("project keep policy".into());
    }
    if !done(v)
        || v.get("trashed") == Some(&json!(true))
        || v.get("canceledAt").is_some_and(|v| !v.is_null())
    {
        return Err("not a current completed issue".into());
    }
    let completed = instant(field(v, "completedAt")?)?;
    if o.now - completed < time::Duration::days(i64::from(o.retention_days)) {
        return Err("retention period has not elapsed".into());
    }
    // Require a trustworthy most recent transition into completed. Reopened/imported
    // histories without this observation are deliberately skipped.
    let history = v
        .get("history")
        .and_then(Value::as_array)
        .ok_or("missing lifecycle history")?;
    let mut transitions = Vec::new();
    for h in history {
        if h.get("toStateId").is_some_and(|v| !v.is_null()) {
            let state = h
                .pointer("/toState/type")
                .and_then(Value::as_str)
                .ok_or("unresolvable historical state")?;
            transitions.push((instant(field(h, "createdAt")?)?, state));
        }
    }
    transitions.sort_by_key(|(at, _)| *at);
    let (last, state) = transitions
        .last()
        .ok_or("no reliable completion transition")?;
    if *state != "completed" || (*last - completed).abs() > time::Duration::seconds(2) {
        return Err("completion timestamp and latest lifecycle transition disagree".into());
    }
    if v.get("parent")
        .filter(|p| !p.is_null())
        .is_some_and(|p| !done(p))
    {
        return Err("unfinished parent work".into());
    }
    for child in v
        .get("children")
        .and_then(Value::as_array)
        .ok_or("missing children")?
    {
        if !done(child) {
            return Err("unfinished child work".into());
        }
    }
    for connection in ["relations", "inverseRelations"] {
        for relation in v
            .get(connection)
            .and_then(Value::as_array)
            .ok_or("missing relationships")?
        {
            for key in ["issue", "relatedIssue"] {
                if !done(relation.get(key).ok_or("unresolved related issue")?) {
                    return Err("unfinished related work".into());
                }
            }
        }
    }
    Ok(())
}
fn hash(v: &Value) -> String {
    format!("{:x}", Sha256::digest(v.to_string().as_bytes()))
}
fn content_links(v: &Value) -> Result<(), String> {
    // Preserve inline uploads and external documents only with a supported durable
    // content exporter. A URL alone is insufficient. This initial adapter refuses
    // those tickets instead of silently losing their context.
    let links = regex::Regex::new(r#"https?://[^\s\"<>\\)]+"#).map_err(|e| e.to_string())?;
    let github = regex::Regex::new(
        r"^https://github\.com/[A-Za-z0-9_.-]+/[A-Za-z0-9_.-]+/pull/[0-9]+(?:#[A-Za-z0-9_.-]+)?$",
    )
    .map_err(|e| e.to_string())?;
    for link in links.find_iter(&v.to_string()) {
        if Some(link.as_str()) != v.get("url").and_then(Value::as_str)
            && !github.is_match(link.as_str())
        {
            return Err(
                "external referenced content needs a durable exporter; retained in Linear".into(),
            );
        }
    }
    let text = v.to_string();
    if text.contains("uploads.linear.app") || text.contains("/document/") || text.contains("![") {
        return Err(
            "referenced file/document needs a durable content exporter; retained in Linear".into(),
        );
    }
    for a in v
        .get("attachments")
        .and_then(Value::as_array)
        .ok_or("missing attachments")?
    {
        let url = field(a, "url")?;
        if !url.starts_with("https://github.com/") || !url.contains("/pull/") {
            return Err("unsupported attachment export; retained in Linear".into());
        }
    }
    Ok(())
}
fn export_context(v: &Value) -> Result<Value, String> {
    content_links(v)?;
    let mut prs = Vec::new();
    let pattern =
        regex::Regex::new(r"https://github\.com/[A-Za-z0-9_.-]+/[A-Za-z0-9_.-]+/pull/[0-9]+")
            .map_err(|e| e.to_string())?;
    let text = v.to_string();
    let urls: std::collections::BTreeSet<_> =
        pattern.find_iter(&text).map(|m| m.as_str()).collect();
    for url in urls {
        let bytes = crate::process::provider_output(
            std::process::Command::new("gh").args([
                "pr",
                "view",
                url,
                "--json",
                "url,title,body,state,headRefOid,baseRefName,mergeCommit,mergedAt",
            ]),
            None,
        )?;
        let pr: Value = serde_json::from_slice(&bytes).map_err(|_| "invalid PR export")?;
        if pr.get("url").and_then(Value::as_str) != Some(url) {
            return Err("PR export identity mismatch".into());
        }
        prs.push(pr);
    }
    Ok(json!({"issue":v,"pull_requests":prs}))
}
/// Preview or apply one bounded sweep. API failures stop; no implicit destructive retries.
pub fn run(p: &mut dyn Provider, c: &Config, o: &Options<'_>) -> Result<Value, String> {
    provider::scope(p, c, o.project)?;
    if o.apply && (o.limit == 0 || o.limit > 100) {
        return Err("apply requires an explicit limit between 1 and 100".into());
    }
    let store = if o.apply {
        let path = o
            .export_dir
            .or(c.export_dir.as_deref())
            .ok_or("apply requires a backed-up private export directory")?;
        // Public repositories and temporary workspace state are not a durable store.
        if std::process::Command::new("git")
            .arg("-C")
            .arg(path)
            .args(["rev-parse", "--is-inside-work-tree"])
            .output()
            .is_ok_and(|r| r.status.success())
            || path.components().any(|p| {
                p.as_os_str() == ".context"
                    || p.as_os_str() == ".gameskills"
                    || p.as_os_str() == "tmp"
            })
        {
            return Err("exports must live outside repositories and temporary task storage".into());
        }
        Some(Store::open(path)?)
    } else {
        None
    };
    let query=format!("query($project:ID!,$after:String){{issues(first:100,after:$after,includeArchived:true,filter:{{project:{{id:{{eq:$project}}}}}}){{nodes{{id identifier completedAt}} {PAGE}}}}}");
    let mut list = provider::pages(
        p,
        &query,
        json!({"project":o.project,"after":null}),
        "/issues",
    )?;
    list.sort_by(|a, b| {
        a.get("completedAt")
            .and_then(Value::as_str)
            .cmp(&b.get("completedAt").and_then(Value::as_str))
            .then_with(|| {
                a.get("id")
                    .and_then(Value::as_str)
                    .cmp(&b.get("id").and_then(Value::as_str))
            })
    });
    let mut results = Vec::new();
    let mut selected = 0;
    for item in list {
        let id = field(&item, "id")?;
        crate::uuid(id)?;
        let identifier = field(&item, "identifier")?;
        let current = snapshot(p, id)?;
        let reason = eligibility(&current, c, o).and_then(|_| content_links(&current));
        if let Err(reason) = reason {
            results
                .push(json!({"id":id,"identifier":identifier,"status":"skipped","reason":reason}));
            continue;
        }
        if !o.apply {
            results.push(json!({"id":id,"identifier":identifier,"status":"eligible","export":"not yet verified"}));
            continue;
        }
        if selected >= o.limit {
            results.push(
                json!({"id":id,"identifier":identifier,"status":"skipped","reason":"batch limit"}),
            );
            continue;
        }
        selected += 1;
        let store = store.as_ref().ok_or("missing private store")?;
        let key = format!("{}-{}-{id}", c.workspace, o.project);
        let journal = format!("{key}-operation.json");
        if let Some(bytes) = store.read(&journal)? {
            let prior: Value =
                serde_json::from_slice(&bytes).map_err(|_| "invalid prior operation")?;
            if prior.get("phase").and_then(Value::as_str) == Some("delete-started") {
                return Err(format!("ambiguous prior deletion for {identifier}; reconcile remote trash state before another apply"));
            }
            if prior.get("phase").and_then(Value::as_str) == Some("deleted") {
                let old_completion = prior.get("completedAt").and_then(Value::as_str).ok_or(
                    "prior operation lacks completion identity; reconcile before applying",
                )?;
                if Some(old_completion) == current.get("completedAt").and_then(Value::as_str) {
                    results.push(json!({"id":id,"identifier":identifier,"status":"skipped","reason":"previous deletion confirmed for this lifecycle"}));
                    continue;
                }
                // A restored, reopened and re-completed issue has a new retention
                // period. Preserve the previous verified operation before replacing
                // the current journal for this exact stable issue identity.
                store.write(&format!("{key}-operation-{}.json", hash(&prior)), &bytes)?;
            }
        }
        let context = export_context(&current)?;
        let digest = hash(&context);
        let filename = format!("{key}-{digest}.json");
        let manifest = json!({"schema_version":1,"workspace":c.workspace,"team":c.team,"project":o.project,"issue":id,"sha256":digest,"content":context});
        let bytes = serde_json::to_vec_pretty(&manifest).map_err(|e| e.to_string())?;
        if let Some(old) = store.read(&filename)? {
            if old != bytes {
                return Err("existing export digest mismatch".into());
            }
        } else {
            store.write(&filename, &bytes)?;
        }
        // Full snapshot comparison catches comments, relations and document changes,
        // not only issue.updatedAt. Linear has no conditional issueDelete revision.
        let fresh = snapshot(p, id)?;
        if fresh != current || eligibility(&fresh, c, o).is_err() {
            results.push(json!({"id":id,"identifier":identifier,"status":"skipped","reason":"changed before deletion; exported snapshot retained"}));
            continue;
        }
        store.write(&journal,&serde_json::to_vec(&json!({"phase":"delete-started","manifest":filename,"issue":id,"completedAt":current.get("completedAt")})).map_err(|e|e.to_string())?)?;
        let deletion=p.query("mutation($id:String!){issueDelete(id:$id,permanentlyDelete:false){success lastSyncId}}",json!({"id":id}))?;
        if deletion.pointer("/issueDelete/success") != Some(&json!(true)) {
            return Err("deletion not confirmed; journal retained for reconciliation".into());
        }
        store.write(
            &journal,
            &serde_json::to_vec(
                &json!({"phase":"deleted","manifest":filename,"issue":id,"completedAt":current.get("completedAt"),"observation":deletion}),
            )
            .map_err(|e| e.to_string())?,
        )?;
        results
            .push(json!({"id":id,"identifier":identifier,"status":"deleted","manifest":filename}));
    }
    Ok(
        json!({"ok":true,"mode":if o.apply{"apply"}else{"preview"},"workspace":c.workspace,"project":o.project,"retention_days":o.retention_days,"results":results,"quota_effect":"unmeasured","limits":["Linear issueDelete has no conditional revision; a read/delete race remains","unsupported referenced binary content is retained in Linear","exports preserve records, not a tested lossless Linear restore"]}),
    )
}

/// Reconcile an interrupted deletion using an explicit remote trash observation.
/// Missing or inaccessible issues remain ambiguous; this command never deletes.
pub fn reconcile(
    p: &mut dyn Provider,
    c: &Config,
    project: &str,
    id: &str,
    path: &Path,
) -> Result<Value, String> {
    crate::uuid(id)?;
    provider::scope(p, c, project)?;
    let store = Store::open(path)?;
    let journal = format!("{}-{project}-{id}-operation.json", c.workspace);
    let bytes = store
        .read(&journal)?
        .ok_or("no operation journal for this issue")?;
    let mut record: Value =
        serde_json::from_slice(&bytes).map_err(|_| "invalid operation journal")?;
    if record.get("issue").and_then(Value::as_str) != Some(id) {
        return Err("journal issue mismatch".into());
    }
    if record.get("phase").and_then(Value::as_str) == Some("deleted") {
        return Ok(
            json!({"ok":true,"issue":id,"status":"previous deletion confirmed","record":record}),
        );
    }
    let filename = field(&record, "manifest")?;
    let export: Value =
        serde_json::from_slice(&store.read(filename)?.ok_or("missing durable export")?)
            .map_err(|_| "invalid export")?;
    if export.get("workspace").and_then(Value::as_str) != Some(&c.workspace)
        || export.get("project").and_then(Value::as_str) != Some(project)
        || export.get("issue").and_then(Value::as_str) != Some(id)
        || export.get("sha256").and_then(Value::as_str)
            != Some(&hash(
                export.get("content").ok_or("missing export content")?,
            ))
    {
        return Err("export identity/digest mismatch".into());
    }
    let result = p.query(
        "query($id:String!){issue(id:$id){id trashed project{id} team{id}}}",
        json!({"id":id}),
    )?;
    let remote = result.get("issue").ok_or("deletion remains unverifiable")?;
    if remote.get("id").and_then(Value::as_str) != Some(id)
        || remote.pointer("/project/id").and_then(Value::as_str) != Some(project)
        || remote.pointer("/team/id").and_then(Value::as_str) != Some(&c.team)
    {
        return Err("remote reconciliation identity unavailable or changed".into());
    }
    let phase = match remote.get("trashed").and_then(Value::as_bool) {
        Some(true) => "deleted",
        Some(false) => "not-deleted",
        None => return Err("remote trash state unavailable".into()),
    };
    crate::put(&mut record, "phase", json!(phase))?;
    crate::put(&mut record, "reconciliation", result)?;
    store.write(
        &journal,
        &serde_json::to_vec(&record).map_err(|e| e.to_string())?,
    )?;
    Ok(json!({"ok":true,"issue":id,"status":phase,"record":record}))
}
