//! Durable task intent and observed GitHub delivery; not human review attestation.

use serde_json::Value;
use std::{ffi::OsString, path::Path};

/// Execute delivery commands against a ready installation.
pub fn execute(root: &Path, config: &Value, args: &[OsString]) -> Result<Value, String> {
    #[cfg(unix)]
    {
        posix::execute(root, config, args)
    }
    #[cfg(not(unix))]
    {
        let _ = (root, config, args);
        Err("delivery state currently requires the POSIX protected-state backend".into())
    }
}

#[cfg(unix)]
mod posix {
    use super::*;
    use crate::runner::{identity, state::Directory};
    use clap::{Parser, Subcommand};
    use serde_json::json;
    use std::process::Command;

    #[derive(Parser)]
    struct Args {
        #[command(subcommand)]
        command: Operation,
    }
    #[derive(Subcommand)]
    enum Operation {
        Start {
            id: String,
            #[arg(long)]
            goal: String,
            #[arg(long)]
            endpoint: Option<String>,
            #[arg(long)]
            repo: Option<String>,
            #[arg(long)]
            base: Option<String>,
            #[arg(long)]
            level: Option<String>,
            #[arg(long = "scope")]
            scope: Vec<String>,
            #[arg(long)]
            gameplay: bool,
            #[arg(long)]
            promotion: bool,
            #[arg(long = "check")]
            checks: Vec<String>,
        },
        Scope {
            id: String,
            #[arg(long)]
            level: Option<String>,
            #[arg(long = "scope")]
            scope: Vec<String>,
            #[arg(long)]
            gameplay: bool,
            #[arg(long)]
            promotion: bool,
            #[arg(long = "check")]
            checks: Vec<String>,
        },
        Bind {
            id: String,
            #[arg(long)]
            pr: Option<String>,
            #[arg(long)]
            issue: Option<String>,
            #[arg(long)]
            project: Option<String>,
        },
        Show {
            id: String,
        },
        Note {
            id: String,
            #[arg(long = "remaining")]
            remaining: Vec<String>,
            #[arg(long)]
            authorization: Option<String>,
        },
        Check {
            id: String,
            #[arg(long = "evidence")]
            evidence: Vec<String>,
            #[arg(long)]
            tracker_observation: Option<std::path::PathBuf>,
            #[arg(long)]
            manual_observation: Option<std::path::PathBuf>,
        },
    }
    fn text<'a>(v: &'a Value, key: &str) -> Result<&'a str, String> {
        v.get(key)
            .and_then(Value::as_str)
            .ok_or_else(|| format!("missing {key}"))
    }
    fn json_command(root: &Path, program: &str, args: &[&str]) -> Result<Value, String> {
        let bytes = crate::platform::provider_output(
            Command::new(program).current_dir(root).args(args),
            None,
        )?;
        serde_json::from_slice(&bytes).map_err(|_| "provider returned invalid JSON".into())
    }
    fn envelope(record: &Value) -> Value {
        use sha2::{Digest, Sha256};
        json!({"sha256":format!("{:x}",Sha256::digest(record.to_string().as_bytes())),"record":record})
    }
    fn read(directory: &Directory, file: &str) -> Result<Value, String> {
        let e: Value = serde_json::from_slice(&directory.read(file)?).map_err(|e| e.to_string())?;
        let r = e.get("record").ok_or("missing delivery record")?;
        if envelope(r) != e {
            return Err("delivery record digest mismatch".into());
        }
        Ok(r.clone())
    }
    fn snapshot(root: &Path) -> Result<Value, String> {
        Ok(
            json!({"repository":identity::repository(root)?,"status":String::from_utf8(identity::git(root,&["status","--porcelain","--untracked-files=all"])?).map_err(|e|e.to_string())?}),
        )
    }
    pub(super) fn execute(root: &Path, config: &Value, args: &[OsString]) -> Result<Value, String> {
        let args = Args::try_parse_from(
            std::iter::once(OsString::from("delivery")).chain(args.iter().cloned()),
        )
        .map_err(|e| e.to_string())?;
        let (id, create) = match &args.command {
            Operation::Start { id, .. } => (id, true),
            Operation::Scope { id, .. }
            | Operation::Bind { id, .. }
            | Operation::Show { id }
            | Operation::Check { id, .. }
            | Operation::Note { id, .. } => (id, false),
        };
        let id = id.clone();
        if id.is_empty()
            || id.len() > 64
            || !id
                .bytes()
                .all(|b| b.is_ascii_alphanumeric() || b == b'-' || b == b'_')
        {
            return Err(
                "task ID must be 1–64 ASCII letters, digits, hyphens or underscores".into(),
            );
        }
        let before = snapshot(root)?;
        let directory = Directory::root(root)?
            .child(".gameskills", false, false)?
            .child("delivery", create, false)?;
        let lock = directory.open(".lock", true, false)?;
        if !crate::runner::state::lock(&lock)? {
            return Err("another delivery operation is active".into());
        }
        let file = format!("{id}.json");
        if let Operation::Start {
            goal,
            endpoint,
            repo,
            base,
            level,
            scope,
            gameplay,
            promotion,
            checks,
            ..
        } = &args.command
        {
            if directory.entries()?.contains(&file) {
                return Err(
                    "task already exists; show or bind it instead of replacing its endpoint".into(),
                );
            }
            let endpoint = endpoint
                .as_deref()
                .or_else(|| {
                    config
                        .pointer("/project/delivery_target")
                        .and_then(Value::as_str)
                })
                .unwrap_or("implementation");
            if !["design", "implementation", "pr", "merge", "release"].contains(&endpoint) {
                return Err("invalid delivery endpoint".into());
            }
            let base = base
                .clone()
                .map(Ok)
                .unwrap_or_else(|| crate::verification::delivery_base(config))?;
            let verification = crate::verification_context::resolve_with_promotion(
                config,
                Some(&base),
                level.as_deref(),
                scope,
                *gameplay,
                *promotion,
            )?;
            if goal.trim().is_empty() || base.starts_with('-') || base.is_empty() {
                return Err("goal and base must be nonempty".into());
            }
            for check in checks {
                if config.get("commands").and_then(|c| c.get(check)).is_none() {
                    return Err(format!("unknown configured check: {check}"));
                }
            }
            let record = json!({"schema_version":1,"id":id,"goal":goal,"endpoint":endpoint,"repo":repo,"base":base,"checks":checks,"initial_source":before,"instruction_lock":crate::platform::read_ordinary_file(&root.join("gameskills.lock.json")).map_err(|e|e.to_string())?,"cli_version":env!("CARGO_PKG_VERSION"),"binding":{},"remaining_work":[],"authorization":"session authorization must be consulted; this record grants none","last_observation":null,"promotion":promotion,"verification":verification});
            directory.write_json(&file, &envelope(&record))?;
            return Ok(
                json!({"ok":true,"record":record,"claim":"task intent; no delivery observed"}),
            );
        }
        let mut record = read(&directory, &file)?;
        if record.pointer("/initial_source/repository/root") != before.pointer("/repository/root") {
            return Err("task belongs to another worktree".into());
        }
        match args.command {
            Operation::Show { .. } => Ok(
                json!({"ok":true,"record":record,"claim":"stored intent and historical observations; run check for current delivery"}),
            ),
            Operation::Scope {
                level,
                scope,
                gameplay,
                promotion,
                checks,
                ..
            } => {
                let selected = crate::verification_context::resolve_with_promotion(
                    config,
                    Some(text(&record, "base")?),
                    level.as_deref().or_else(|| {
                        record
                            .pointer("/verification/policy/level")
                            .and_then(Value::as_str)
                    }),
                    &scope,
                    gameplay,
                    promotion,
                )?;
                for check in &checks {
                    if config.get("commands").and_then(|c| c.get(check)).is_none() {
                        return Err(format!("unknown configured check: {check}"));
                    }
                }
                let prior = record.get("verification").cloned().unwrap_or(Value::Null);
                let fields = record.as_object_mut().ok_or("invalid task")?;
                fields
                    .entry("verification_history")
                    .or_insert_with(|| json!([]))
                    .as_array_mut()
                    .ok_or("invalid scope history")?
                    .push(prior);
                if !checks.is_empty() {
                    let prior_checks =
                        fields.get("checks").cloned().ok_or("missing task checks")?;
                    fields
                        .entry("check_history")
                        .or_insert_with(|| json!([]))
                        .as_array_mut()
                        .ok_or("invalid check history")?
                        .push(prior_checks);
                    fields.insert("checks".into(), json!(checks));
                }
                fields.insert("verification".into(), selected);
                fields.insert("promotion".into(), json!(promotion));
                fields.insert("last_observation".into(), Value::Null);
                directory.write_json(&file, &envelope(&record))?;
                Ok(
                    json!({"ok":true,"record":record,"claim":"resolved scope; no checks or manual acceptance observed"}),
                )
            }
            Operation::Bind {
                pr, issue, project, ..
            } => {
                let binding = record
                    .get_mut("binding")
                    .and_then(Value::as_object_mut)
                    .ok_or("invalid binding")?;
                for (k, v) in [("pr", pr), ("issue", issue), ("project", project)] {
                    if let Some(v) = v {
                        if v.trim().is_empty() {
                            return Err("empty binding".into());
                        }
                        binding.insert(k.into(), json!(v));
                    }
                }
                record
                    .as_object_mut()
                    .ok_or("invalid delivery object")?
                    .insert("last_observation".into(), Value::Null);
                directory.write_json(&file, &envelope(&record))?;
                Ok(json!({"ok":true,"record":record,"claim":"binding only; not verified"}))
            }
            Operation::Note {
                remaining,
                authorization,
                ..
            } => {
                let fields = record.as_object_mut().ok_or("invalid task")?;
                fields.insert("remaining_work".into(), json!(remaining));
                if let Some(text) = authorization {
                    fields.insert("authorization".into(), json!(text));
                }
                fields.insert("last_observation".into(), Value::Null);
                directory.write_json(&file, &envelope(&record))?;
                Ok(
                    json!({"ok":true,"record":record,"claim":"recorded task handoff, not a grant of authority"}),
                )
            }
            Operation::Check {
                evidence,
                tracker_observation,
                manual_observation,
                ..
            } => {
                let tracking_required = config
                    .pointer("/tracking/required")
                    .and_then(Value::as_bool)
                    == Some(true)
                    && matches!(text(&record, "endpoint")?, "pr" | "merge" | "release");
                if tracker_observation.is_some()
                    && (!tracking_required || config.pointer("/tracking/observer").is_some())
                {
                    return Err("--tracker-observation requires required MCP tracking for a PR, merge or release task".into());
                }
                let mut reasons = Vec::new();
                let verification = record.get("verification").unwrap_or(&Value::Null);
                if let Err(error) = crate::verification_context::validate(config, verification) {
                    reasons.push(format!("verification scope: {error}; use delivery scope after reviewing the changed requirements"));
                }
                if let Some(work) = record.get("remaining_work").and_then(Value::as_array) {
                    for item in work {
                        reasons.push(format!(
                            "remaining work: {}",
                            item.as_str().ok_or("invalid remaining work")?
                        ));
                    }
                }
                let mut observations = json!({"source":before,"evidence":[],"pr":null,"tracker":null,"verification":verification,"manual_sanity":null});
                let human = manual_observation
                    .as_deref()
                    .map(|file| {
                        let bytes = crate::platform::read_ordinary_file(&root.join(file))
                            .map_err(|e| e.to_string())?;
                        serde_json::from_str::<Value>(&bytes)
                            .map_err(|e| format!("invalid manual observation: {e}"))
                    })
                    .transpose()?;
                match crate::verification_context::manual_observation(
                    &id,
                    before
                        .pointer("/repository/head")
                        .and_then(Value::as_str)
                        .ok_or("missing source head")?,
                    verification,
                    human.as_ref(),
                ) {
                    Ok(result) => {
                        *observations
                            .get_mut("manual_sanity")
                            .ok_or("missing manual sanity observation field")? = result;
                    }
                    Err(error) => reasons.push(error),
                }
                if before.get("status").and_then(Value::as_str) != Some("") {
                    reasons.push("worktree has uncommitted changes".to_string());
                }
                let mut covered = std::collections::BTreeSet::new();
                for run in &evidence {
                    let result = crate::runner::execute(
                        root,
                        config,
                        "evidence",
                        &["show".into(), run.into()],
                    )?;
                    // `show` embeds a separately recomputed validation.
                    let valid = crate::runner::execute(
                        root,
                        config,
                        "evidence",
                        &["validate".into(), run.into()],
                    )?;
                    if valid.get("ok") != Some(&json!(true)) {
                        reasons.push(format!("evidence {run} is invalid"));
                    } else if !crate::verification_context::covers(
                        verification,
                        result
                            .pointer("/record/verification")
                            .unwrap_or(&Value::Null),
                    ) {
                        reasons.push(format!(
                            "evidence {run} has a different verification policy/base or lower level"
                        ));
                    } else if let Some(commands) = result
                        .pointer("/record/commands")
                        .and_then(Value::as_object)
                    {
                        covered.extend(commands.keys().cloned());
                    }
                    observations
                        .get_mut("evidence")
                        .and_then(Value::as_array_mut)
                        .ok_or("invalid observation")?
                        .push(valid);
                }
                for check in record
                    .get("checks")
                    .and_then(Value::as_array)
                    .ok_or("invalid checks")?
                {
                    if !covered.contains(check.as_str().ok_or("invalid check")?) {
                        reasons.push(format!("missing current command evidence: {check}"));
                    }
                }
                let endpoint = text(&record, "endpoint")?;
                if ["pr", "merge", "release"].contains(&endpoint) {
                    match observe_pr(root, &record, &before) {
                        Ok(pr) => {
                            reasons.extend(pr_reasons(&record, &before, &pr));
                            observations
                                .as_object_mut()
                                .ok_or("invalid delivery object")?
                                .insert("pr".into(), pr);
                        }
                        Err(e) => reasons.push(format!("PR unverifiable: {e}")),
                    }
                }
                if tracking_required {
                    match observe_tracker(
                        root,
                        config,
                        &record,
                        &before,
                        observations.get("pr").unwrap_or(&Value::Null),
                        tracker_observation.as_deref(),
                    ) {
                        Ok(v) => {
                            observations
                                .as_object_mut()
                                .ok_or("invalid delivery object")?
                                .insert("tracker".into(), v);
                        }
                        Err(e) => reasons.push(format!("tracking unverifiable: {e}")),
                    }
                }
                if endpoint == "release" {
                    reasons.push("release artifact acceptance must be verified by the release workflow; this checker does not observe releases".into());
                }
                if snapshot(root)? != before {
                    reasons.push("source or refs changed during delivery observation".into());
                }
                let result = json!({"ok":reasons.is_empty(),"endpoint":endpoint,"observed_at":std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map_err(|e|e.to_string())?.as_secs(),"reasons":reasons,"observations":observations,"claim":"observed delivery and command checks; not human review, skill activation, or gameplay acceptance"});
                record
                    .as_object_mut()
                    .ok_or("invalid delivery object")?
                    .insert("last_observation".into(), result.clone());
                directory.write_json(&file, &envelope(&record))?;
                Ok(result)
            }
            Operation::Start { .. } => Err("unexpected start".into()),
        }
    }
    fn observe_pr(root: &Path, record: &Value, source: &Value) -> Result<Value, String> {
        let repo = text(record, "repo")?;
        let pr = record
            .pointer("/binding/pr")
            .and_then(Value::as_str)
            .ok_or("no PR binding; push and create the PR first")?;
        if !pr.starts_with(&format!("https://github.com/{repo}/pull/"))
            || !pr
                .rsplit('/')
                .next()
                .is_some_and(|s| !s.is_empty() && s.bytes().all(|b| b.is_ascii_digit()))
        {
            return Err("PR URL does not match the task repository".into());
        }
        if repo.split('/').count() != 2
            || !repo.split('/').all(|part| {
                !part.is_empty()
                    && part
                        .bytes()
                        .all(|b| b.is_ascii_alphanumeric() || b"_.-".contains(&b))
            })
        {
            return Err("invalid GitHub repository identity".into());
        }
        let base = text(record, "base")?;
        identity::git(root, &["check-ref-format", "--branch", base])?;
        let repository = json_command(
            root,
            "gh",
            &["repo", "view", repo, "--json", "nameWithOwner"],
        )?;
        if repository.get("nameWithOwner").and_then(Value::as_str) != Some(repo) {
            return Err("remote repository identity mismatch".into());
        }
        let mut v = json_command(
            root,
            "gh",
            &[
                "pr",
                "view",
                pr,
                "--repo",
                repo,
                "--json",
                "url,body,state,headRefOid,headRefName,baseRefName,baseRefOid,mergeable,mergeStateStatus,reviewDecision,statusCheckRollup,mergeCommit",
            ],
        )?;
        let state = v.get("state").and_then(Value::as_str);
        let local_head = source
            .pointer("/repository/head")
            .and_then(Value::as_str)
            .ok_or("missing source head")?;
        let pr_head = v
            .get("headRefOid")
            .and_then(Value::as_str)
            .ok_or("remote PR source unavailable")?;
        if state != Some("MERGED") && pr_head != local_head {
            return Err(
                "remote PR source differs from current HEAD; push the current commits".into(),
            );
        }
        let target = json_command(
            root,
            "gh",
            &["api", &format!("repos/{repo}/git/ref/heads/{base}")],
        )?;
        let target_sha = target
            .pointer("/object/sha")
            .and_then(Value::as_str)
            .ok_or("target revision unavailable")?;
        if v.get("state").and_then(Value::as_str) == Some("OPEN")
            && v.get("baseRefOid").and_then(Value::as_str) != Some(target_sha)
        {
            return Err("PR base observation differs from current remote target".into());
        }
        if state == Some("MERGED") {
            let merge = v
                .pointer("/mergeCommit/oid")
                .and_then(Value::as_str)
                .ok_or("merge identity unavailable")?;
            if !merge.bytes().all(|b| b.is_ascii_hexdigit()) || merge.len() != 40 {
                return Err("invalid merge identity".into());
            }
            let integrated = json_command(
                root,
                "gh",
                &[
                    "api",
                    &format!("repos/{repo}/compare/{merge}...{target_sha}"),
                ],
            )?;
            if !matches!(
                integrated.get("status").and_then(Value::as_str),
                Some("ahead" | "identical")
            ) {
                return Err("merge is not contained in the current remote target".into());
            }
            if pr_head != local_head {
                if !git_ancestor(root, merge, local_head)
                    || !git_ancestor(root, local_head, target_sha)
                {
                    return Err(
                        "local HEAD is not an integrated revision of the current remote target"
                            .into(),
                    );
                }
            }
            v.as_object_mut()
                .ok_or("invalid delivery object")?
                .insert("integration".into(), integrated);
        }
        v.as_object_mut()
            .ok_or("invalid delivery object")?
            .insert("remote_repository".into(), repository);
        v.as_object_mut()
            .ok_or("invalid delivery object")?
            .insert("remote_target".into(), target);
        Ok(v)
    }
    fn git_ancestor(root: &Path, ancestor: &str, descendant: &str) -> bool {
        Command::new("git")
            .current_dir(root)
            .args(["merge-base", "--is-ancestor", ancestor, descendant])
            .output()
            .map(|output| output.status.success())
            .unwrap_or(false)
    }
    fn pr_reasons(record: &Value, source: &Value, pr: &Value) -> Vec<String> {
        let mut r = Vec::new();
        if pr.get("baseRefName") != record.get("base") {
            r.push("PR targets the wrong base".into());
        }
        if pr.get("url") != record.pointer("/binding/pr") {
            r.push("provider returned a different PR".into());
        }
        if pr.get("state").and_then(Value::as_str) != Some("MERGED")
            && pr.get("headRefOid") != source.pointer("/repository/head")
        {
            r.push("remote source mismatch".into());
        }
        let state = pr.get("state").and_then(Value::as_str);
        if !matches!(state, Some("OPEN" | "MERGED")) {
            r.push("PR is closed without merge or its state is unknown".into());
        }
        if record.get("endpoint").and_then(Value::as_str) != Some("pr")
            && (state != Some("MERGED")
                || pr
                    .pointer("/mergeCommit/oid")
                    .and_then(Value::as_str)
                    .is_none())
        {
            r.push("requested merge has not been observed".into());
        }
        if pr
            .get("reviewDecision")
            .and_then(Value::as_str)
            .is_some_and(|v| matches!(v, "CHANGES_REQUESTED" | "REVIEW_REQUIRED"))
        {
            r.push("required review is unresolved".into());
        }
        match pr.get("statusCheckRollup").and_then(Value::as_array) {
            Some(checks) => {
                for check in checks {
                    let state = check
                        .get("state")
                        .or_else(|| check.get("conclusion"))
                        .and_then(Value::as_str);
                    if !matches!(state, Some("SUCCESS" | "NEUTRAL" | "SKIPPED")) {
                        r.push(format!(
                            "CI pending or failed: {}",
                            check
                                .get("name")
                                .or_else(|| check.get("context"))
                                .unwrap_or(&Value::Null)
                        ));
                    }
                }
            }
            None => r.push("CI state unavailable".into()),
        }
        r
    }
    fn observe_tracker(
        root: &Path,
        config: &Value,
        record: &Value,
        source: &Value,
        remote_pr: &Value,
        host_file: Option<&Path>,
    ) -> Result<Value, String> {
        let issue = record
            .pointer("/binding/issue")
            .and_then(Value::as_str)
            .ok_or("no issue binding")?;
        let project = record
            .pointer("/binding/project")
            .and_then(Value::as_str)
            .ok_or("no project binding")?;
        let pr = record
            .pointer("/binding/pr")
            .and_then(Value::as_str)
            .ok_or("no PR binding")?;
        if config.pointer("/tracking/observer").is_none() {
            return observe_mcp(root, record, source, remote_pr, host_file);
        }
        let argv = config
            .pointer("/tracking/observer")
            .and_then(Value::as_array)
            .ok_or("no configured tracker observer")?;
        let mut words = argv
            .iter()
            .map(|v| v.as_str().ok_or("observer must contain strings"));
        let program = words.next().ok_or("empty observer")??;
        let mut args = words.collect::<Result<Vec<_>, _>>()?;
        args.extend(["--issue", issue, "--project", project, "--pr", pr]);
        let v = json_command(root, program, &args)?;
        if v.get("ok") != Some(&json!(true))
            || v.get("issue_id").and_then(Value::as_str) != Some(issue)
            || v.get("project_id").and_then(Value::as_str) != Some(project)
            || v.get("pr_url").and_then(Value::as_str) != Some(pr)
            || v.get("linked") != Some(&json!(true))
        {
            return Err("tracker has not verified the exact issue/project/PR link".into());
        }
        Ok(v)
    }

    fn observe_mcp(
        root: &Path,
        record: &Value,
        source: &Value,
        remote_pr: &Value,
        file: Option<&Path>,
    ) -> Result<Value, String> {
        use sha2::{Digest, Sha256};
        let file = file.ok_or("use connected tracker MCP tools, then pass a fresh --tracker-observation FILE; no standalone executable or API key is required")?;
        let bytes = crate::platform::read_ordinary_file(&root.join(file))
            .map_err(|e| format!("cannot read MCP observation: {e}"))?;
        let receipt: Value =
            serde_json::from_str(&bytes).map_err(|_| "invalid MCP observation JSON")?;
        if receipt.get("schema_version") != Some(&json!(1))
            || receipt.get("transport").and_then(Value::as_str) != Some("mcp")
        {
            return Err("MCP observation needs schema_version 1 and transport mcp".into());
        }
        for field in ["tool", "evidence_reference"] {
            if text(&receipt, field)?.trim().is_empty() {
                return Err(format!("MCP observation needs {field}"));
            }
        }
        if receipt.get("task_id") != record.get("id")
            || receipt.get("source_head") != source.pointer("/repository/head")
        {
            return Err("MCP observation belongs to another task or source HEAD".into());
        }
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map_err(|e| e.to_string())?
            .as_secs();
        let observed_at = receipt
            .get("observed_at")
            .and_then(Value::as_u64)
            .ok_or("MCP observation needs observed_at as Unix seconds")?;
        if observed_at > now || now - observed_at > 300 {
            return Err("MCP observation is future-dated or older than 300 seconds; query the connector again".into());
        }
        let issue = receipt
            .get("issue")
            .ok_or("MCP observation needs an issue snapshot")?;
        if issue.get("id") != record.pointer("/binding/issue")
            || issue.get("project_id") != record.pointer("/binding/project")
        {
            return Err("MCP observation issue/project differs from the task binding".into());
        }
        let pr_url = record
            .pointer("/binding/pr")
            .and_then(Value::as_str)
            .ok_or("no PR binding")?;
        let attached = issue
            .get("attachment_urls")
            .and_then(Value::as_array)
            .is_some_and(|urls| urls.iter().any(|url| url.as_str() == Some(pr_url)));
        let issue_url = text(issue, "url")?;
        if !issue_url.starts_with("https://")
            || issue_url.len() <= 8
            || issue_url.chars().any(char::is_whitespace)
        {
            return Err("MCP issue snapshot needs an HTTPS issue URL".into());
        }
        let backlink = remote_pr
            .get("body")
            .and_then(Value::as_str)
            .is_some_and(|body| contains_url(body, issue_url));
        if !attached || !backlink {
            return Err("MCP issue attachment and current PR body must link the exact objects in both directions".into());
        }
        Ok(
            json!({"ok":true,"linked":true,"issue_id":issue.get("id"),"project_id":issue.get("project_id"),"pr_url":pr_url,"transport":"mcp","observation":receipt,"observation_sha256":format!("{:x}", Sha256::digest(bytes.as_bytes())),"claim":"caller-supplied MCP snapshot and provenance; exact bindings and live GitHub backlink checked by CLI; MCP invocation, timestamp and evidence reference are not independently authenticated"}),
        )
    }

    fn contains_url(body: &str, url: &str) -> bool {
        // Match a complete URL, not an issue slug prefix or a URL embedded in another URL.
        body.split(|c: char| {
            c.is_whitespace() || matches!(c, '(' | ')' | '<' | '>' | '[' | ']' | '"' | '\'' | '`')
        })
        .any(|token| token.trim_end_matches(['.', ',', ';', '!']) == url)
    }
}
