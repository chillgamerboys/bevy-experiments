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
            #[arg(long, default_value = "main")]
            base: String,
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
            Operation::Bind { id, .. }
            | Operation::Show { id }
            | Operation::Check { id, .. }
            | Operation::Note { id, .. } => (id, false),
        };
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
            if goal.trim().is_empty() || base.starts_with('-') || base.is_empty() {
                return Err("goal and base must be nonempty".into());
            }
            for check in checks {
                if config.get("commands").and_then(|c| c.get(check)).is_none() {
                    return Err(format!("unknown configured check: {check}"));
                }
            }
            let record = json!({"schema_version":1,"id":id,"goal":goal,"endpoint":endpoint,"repo":repo,"base":base,"checks":checks,"initial_source":before,"instruction_lock":crate::platform::read_ordinary_file(&root.join("gameskills.lock.json")).map_err(|e|e.to_string())?,"cli_version":env!("CARGO_PKG_VERSION"),"binding":{},"remaining_work":[],"authorization":"session authorization must be consulted; this record grants none","last_observation":null});
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
            Operation::Check { evidence, .. } => {
                let mut reasons = Vec::new();
                if let Some(work) = record.get("remaining_work").and_then(Value::as_array) {
                    for item in work {
                        reasons.push(format!(
                            "remaining work: {}",
                            item.as_str().ok_or("invalid remaining work")?
                        ));
                    }
                }
                let mut observations =
                    json!({"source":before,"evidence":[],"pr":null,"tracker":null});
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
                if config
                    .pointer("/tracking/required")
                    .and_then(Value::as_bool)
                    == Some(true)
                    && ["pr", "merge", "release"].contains(&endpoint)
                {
                    match observe_tracker(root, config, &record) {
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
        let mut v=json_command(root,"gh",&["pr","view",pr,"--repo",repo,"--json","url,state,headRefOid,headRefName,baseRefName,baseRefOid,mergeable,mergeStateStatus,reviewDecision,statusCheckRollup,mergeCommit"])?;
        if v.get("headRefOid") != source.pointer("/repository/head") {
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
        if v.get("state").and_then(Value::as_str) == Some("MERGED") {
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
    fn pr_reasons(record: &Value, source: &Value, pr: &Value) -> Vec<String> {
        let mut r = Vec::new();
        if pr.get("baseRefName") != record.get("base") {
            r.push("PR targets the wrong base".into());
        }
        if pr.get("url") != record.pointer("/binding/pr") {
            r.push("provider returned a different PR".into());
        }
        if pr.get("headRefOid") != source.pointer("/repository/head") {
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
    fn observe_tracker(root: &Path, config: &Value, record: &Value) -> Result<Value, String> {
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
}
