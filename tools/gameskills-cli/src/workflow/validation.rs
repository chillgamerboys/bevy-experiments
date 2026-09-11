//! Pure contract checks plus read-only Git observations.
use super::{at, list, put, string};
use serde_json::{json, Map, Value};
use std::{
    collections::{BTreeMap, BTreeSet},
    path::{Component, Path},
    process::{Command, Output},
};

pub(super) fn text<'a>(value: &'a Value, label: &str) -> Result<&'a str, String> {
    value
        .as_str()
        .filter(|s| !s.trim().is_empty() && !s.chars().any(|c| (c as u32) < 32))
        .ok_or_else(|| format!("{label} must be a nonempty string without control characters"))
}
pub(super) fn id<'a>(value: &'a Value, label: &str) -> Result<&'a str, String> {
    let name = value.as_str().unwrap_or("");
    if name.is_empty()
        || name.len() > 64
        || !name
            .as_bytes()
            .first()
            .is_some_and(u8::is_ascii_alphanumeric)
        || !name
            .bytes()
            .all(|c| c.is_ascii_alphanumeric() || c == b'_' || c == b'-')
    {
        return Err(format!(
            "{label} must be 1-64 letters, digits, underscores or hyphens, starting alphanumeric"
        ));
    }
    Ok(name)
}
pub(super) fn mapping<'a>(value: &'a Value, label: &str) -> Result<&'a Map<String, Value>, String> {
    value
        .as_object()
        .ok_or_else(|| format!("{label} must be an object"))
}
fn strings(value: &Value, label: &str) -> Result<Vec<String>, String> {
    let entries = value
        .as_array()
        .ok_or_else(|| format!("{label} must be a list"))?;
    let result: Vec<_> = entries
        .iter()
        .map(|v| text(v, label).map(str::to_owned))
        .collect::<Result<_, _>>()?;
    if result.iter().collect::<BTreeSet<_>>().len() != result.len() {
        return Err(format!("{label} contains duplicates"));
    }
    Ok(result)
}
pub(super) fn configuration(config: &Value) -> Result<usize, String> {
    mapping(config, "configuration")?;
    let dispatch = config.get("dispatch").cloned().unwrap_or(json!({}));
    mapping(&dispatch, "dispatch configuration")?;
    let maximum = dispatch.get("max_workers").cloned().unwrap_or(json!(5));
    let maximum = maximum
        .as_u64()
        .filter(|n| (1..=5).contains(n))
        .ok_or("dispatch.max_workers must be an integer from 1 through 5")?;
    if dispatch.get("enabled").is_some_and(|v| !v.is_boolean()) {
        return Err("dispatch.enabled must be a boolean".into());
    }
    usize::try_from(maximum).map_err(|e| e.to_string())
}
fn git(root: &Path, args: &[&str]) -> Result<Output, String> {
    Command::new("git")
        .arg("-C")
        .arg(root)
        .args(args)
        .env("GIT_OPTIONAL_LOCKS", "0")
        .output()
        .map_err(|e| format!("cannot execute git: {e}"))
}
fn git_text(root: &Path, args: &[&str]) -> Result<String, String> {
    let result = git(root, args)?;
    if !result.status.success() {
        return Err(format!(
            "git {} failed: {}",
            args.join(" "),
            String::from_utf8_lossy(&result.stderr).trim()
        ));
    }
    String::from_utf8(result.stdout)
        .map(|s| s.trim_end_matches(['\r', '\n']).to_owned())
        .map_err(|e| format!("git output is not UTF-8: {e}"))
}
pub(super) fn repository_identity(root: &Path) -> Result<Value, String> {
    let root = root
        .canonicalize()
        .map_err(|e| format!("cannot resolve checkout: {e}"))?;
    let top = Path::new(&git_text(&root, &["rev-parse", "--show-toplevel"])?)
        .canonicalize()
        .map_err(|e| e.to_string())?;
    if top != root {
        return Err(format!(
            "expected checkout root {}, found {}",
            root.display(),
            top.display()
        ));
    }
    let common = std::path::PathBuf::from(git_text(&root, &["rev-parse", "--git-common-dir"])?);
    let common = if common.is_absolute() {
        common
    } else {
        root.join(common)
    }
    .canonicalize()
    .map_err(|e| e.to_string())?;
    let root = root.to_str().ok_or("checkout paths must be UTF-8")?;
    let common = common
        .to_str()
        .ok_or("common repository paths must be UTF-8")?;
    Ok(
        json!({"root":root,"common_dir":common,"head":git_text(Path::new(root),&["rev-parse","HEAD"])?}),
    )
}
fn commit(root: &Path, value: &Value, label: &str) -> Result<(), String> {
    let name = value.as_str().unwrap_or("");
    if ![40, 64].contains(&name.len())
        || !name
            .bytes()
            .all(|c| c.is_ascii_digit() || (b'a'..=b'f').contains(&c))
    {
        return Err(format!("{label} must be a full lowercase commit object id"));
    }
    if git_text(
        root,
        &["rev-parse", "--verify", &format!("{name}^{{commit}}")],
    )? != name
    {
        return Err(format!("{label} is not a commit"));
    }
    Ok(())
}
pub(super) fn ancestor(root: &Path, ancestor: &str, descendant: &str) -> Result<bool, String> {
    commit(root, &json!(ancestor), "ancestor")?;
    commit(root, &json!(descendant), "descendant")?;
    let result = git(root, &["merge-base", "--is-ancestor", ancestor, descendant])?;
    match result.status.code() {
        Some(0) => Ok(true),
        Some(1) => Ok(false),
        _ => Err(format!(
            "cannot verify commit ancestry: {}",
            String::from_utf8_lossy(&result.stderr)
        )),
    }
}
pub(super) fn clean(root: &Path) -> Result<bool, String> {
    let result = git(
        root,
        &[
            "diff",
            "--no-ext-diff",
            "--no-textconv",
            "--quiet",
            "HEAD",
            "--",
        ],
    )?;
    match result.status.code() {
        Some(0) => {}
        Some(1) => return Ok(false),
        _ => {
            return Err(format!(
                "cannot observe clean checkout: {}",
                String::from_utf8_lossy(&result.stderr)
            ))
        }
    }
    let result = git(root, &["ls-files", "--others", "--exclude-standard", "-z"])?;
    if !result.status.success() {
        return Err(format!(
            "cannot inspect untracked files: {}",
            String::from_utf8_lossy(&result.stderr)
        ));
    }
    Ok(result
        .stdout
        .split(|c| *c == 0)
        .filter(|p| !p.is_empty())
        .all(|p| p == b".gameskills" || p.starts_with(b".gameskills/")))
}
fn ownership_path(value: &Value, root: &Path) -> Result<(), String> {
    let path = text(value, "ownership path")?;
    let trimmed = path.strip_suffix('/').unwrap_or(path);
    if path.starts_with('/')
        || path.contains('\\')
        || trimmed.split('/').any(|p| ["", ".", ".."].contains(&p))
        || path.chars().any(|c| "*?[]:".contains(c))
        || [".gameskills", ".git"].contains(&trimmed)
        || path.starts_with(".gameskills/")
        || path.starts_with(".git/")
    {
        return Err("ownership paths must be literal, repository-relative paths without dot segments or glob characters".into());
    }
    let mut current = root.to_path_buf();
    for component in Path::new(trimmed).components() {
        if !matches!(component, Component::Normal(_)) {
            return Err("ownership path must be repository-relative".into());
        }
        current.push(component);
        match std::fs::symlink_metadata(&current) {
            Ok(meta) if meta.file_type().is_symlink() => {
                return Err("ownership paths may not pass through symbolic links".into())
            }
            Ok(_) => {}
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => {}
            Err(e) => return Err(format!("cannot inspect ownership path: {e}")),
        }
    }
    Ok(())
}
pub(super) fn validate_order(order: &Value, plan: &Value, config: &Value) -> Result<Value, String> {
    mapping(order, "order")?;
    let mut order = order.clone();
    id(at(&order, &["id"]), "order id")?;
    for key in ["goal", "artifact", "target", "creative_scope"] {
        text(at(&order, &[key]), &format!("order {key}"))?;
    }
    if order.get("creative_level").is_none() {
        put(
            &mut order,
            &["creative_level"],
            at(plan, &["creative_level"]).clone(),
        )?;
    }
    level(at(&order, &["creative_level"]))?;
    mapping(at(&order, &["owner"]), "owner")?;
    if !["agent", "human"].contains(&string(at(&order, &["owner"]), "kind")) {
        return Err("owner.kind must be agent or human".into());
    }
    text(at(&order, &["owner", "name"]), "owner.name")?;
    mapping(at(&order, &["execution"]), "execution")?;
    for key in ["role", "effort"] {
        text(at(&order, &["execution", key]), &format!("execution.{key}"))?;
    }
    for key in [
        "decisions",
        "investigation",
        "references",
        "acceptance",
        "expected_evidence",
        "coordination",
        "resources",
    ] {
        let items = strings(at(&order, &[key]), &format!("order {key}"))?;
        if ["acceptance", "expected_evidence"].contains(&key) && items.is_empty() {
            return Err("orders need acceptance checks and expected evidence".into());
        }
    }
    if order.get("packages").is_none() {
        put(&mut order, &["packages"], at(plan, &["packages"]).clone())?;
    }
    let packages = strings(at(&order, &["packages"]), "order packages")?;
    let planned = list(plan, "packages");
    let selected = list(config, "packages");
    if packages
        .iter()
        .any(|p| !planned.contains(p) || !selected.contains(p))
    {
        return Err(
            "order packages must be selected by the plan and current project configuration".into(),
        );
    }
    let files = at(&order, &["files"])
        .as_array()
        .ok_or("order files must be a list")?;
    for entry in files {
        let map = mapping(entry, "file ownership")?;
        ownership_path(
            at(entry, &["path"]),
            Path::new(string(at(plan, &["repository"]), "root")),
        )?;
        if map
            .keys()
            .any(|key| !["path", "lines"].contains(&key.as_str()))
        {
            return Err("file ownership supports only path and optional lines".into());
        }
        if let Some(lines) = entry.get("lines") {
            let range = lines
                .as_array()
                .ok_or("file lines must be an inclusive [start, end] range on a file")?;
            let start = range.first().and_then(Value::as_u64);
            let end = range.get(1).and_then(Value::as_u64);
            if string(entry, "path").ends_with('/')
                || range.len() != 2
                || start.is_none_or(|n| n == 0)
                || end.is_none()
                || start > end
            {
                return Err("file lines must be an inclusive [start, end] range on a file".into());
            }
        }
    }
    for key in ["dispatch_blockers", "merge_blockers"] {
        for dep in strings(at(&order, &[key]), key)? {
            id(&json!(dep), key)?;
            if dep == string(&order, "id") {
                return Err("an order cannot depend on itself".into());
            }
        }
    }
    if let Some(reason) = order.get("reason") {
        text(reason, "injection reason")?;
    }
    Ok(order)
}
fn level(value: &Value) -> Result<(), String> {
    if value.as_u64().is_none_or(|n| !(1..=4).contains(&n)) {
        Err("creative_level must be an integer from 1 through 4".into())
    } else {
        Ok(())
    }
}
pub(super) fn file_conflict(left: &Value, right: &Value) -> bool {
    let Some(a_files) = left.get("files").and_then(Value::as_array) else {
        return false;
    };
    let Some(b_files) = right.get("files").and_then(Value::as_array) else {
        return false;
    };
    for a in a_files {
        for b in b_files {
            let a_path = string(a, "path");
            let b_path = string(b, "path");
            let ap = a_path.trim_end_matches('/');
            let bp = b_path.trim_end_matches('/');
            if ap == bp {
                let ar = a.get("lines").and_then(Value::as_array);
                let br = b.get("lines").and_then(Value::as_array);
                if let (Some(ar), Some(br)) = (ar, br) {
                    let start = ar
                        .first()
                        .and_then(Value::as_u64)
                        .unwrap_or(0)
                        .max(br.first().and_then(Value::as_u64).unwrap_or(0));
                    let end = ar
                        .get(1)
                        .and_then(Value::as_u64)
                        .unwrap_or(u64::MAX)
                        .min(br.get(1).and_then(Value::as_u64).unwrap_or(u64::MAX));
                    if start <= end {
                        return true;
                    }
                } else {
                    return true;
                }
            } else if (a_path.ends_with('/') && bp.starts_with(&format!("{ap}/")))
                || (b_path.ends_with('/') && ap.starts_with(&format!("{bp}/")))
            {
                return true;
            }
        }
    }
    false
}
pub(super) fn resource_conflict(left: &Value, right: &Value) -> bool {
    let right = list(right, "resources");
    list(left, "resources").iter().any(|v| right.contains(v))
}
pub(super) fn depends(orders: &BTreeMap<String, Value>, child: &str, ancestor: &str) -> bool {
    let mut pending = vec![child.to_owned()];
    let mut seen = BTreeSet::new();
    while let Some(name) = pending.pop() {
        if !seen.insert(name.clone()) {
            continue;
        }
        if let Some(order) = orders.get(&name) {
            for dep in list(order, "dispatch_blockers") {
                if dep == ancestor {
                    return true;
                }
                pending.push(dep);
            }
        }
    }
    false
}
pub(super) fn validate_graph(
    orders: &BTreeMap<String, Value>,
    ownership_ids: Option<&BTreeSet<String>>,
) -> Result<(), String> {
    // Kahn's algorithm avoids overflowing the stack on a deeply chained input.
    let mut remaining = BTreeMap::new();
    let mut consumers: BTreeMap<String, Vec<String>> = BTreeMap::new();
    for (name, order) in orders {
        let deps = list(order, "dispatch_blockers")
            .into_iter()
            .chain(list(order, "merge_blockers"))
            .collect::<BTreeSet<_>>();
        remaining.insert(name.clone(), deps.len());
        for dep in deps {
            if !orders.contains_key(&dep) {
                return Err(format!("unknown dependency {dep} in {name}"));
            }
            consumers.entry(dep).or_default().push(name.clone());
        }
    }
    let mut ready = remaining
        .iter()
        .filter(|(_, n)| **n == 0)
        .map(|(name, _)| name.clone())
        .collect::<Vec<_>>();
    let mut visited = 0;
    while let Some(name) = ready.pop() {
        visited += 1;
        if let Some(children) = consumers.get(&name) {
            for child in children {
                if let Some(count) = remaining.get_mut(child) {
                    *count = count.saturating_sub(1);
                    if *count == 0 {
                        ready.push(child.clone());
                    }
                }
            }
        }
    }
    if visited != orders.len() {
        return Err("dependency cycle detected".into());
    }
    let values = orders
        .iter()
        .filter(|(name, _)| ownership_ids.is_none_or(|ids| ids.contains(*name)))
        .collect::<Vec<_>>();
    for (index, (left_name, left)) in values.iter().enumerate() {
        for (right_name, right) in values.iter().skip(index + 1) {
            if file_conflict(left, right)
                && !depends(orders, left_name, right_name)
                && !depends(orders, right_name, left_name)
            {
                return Err(format!("file ownership collision between {left_name} and {right_name}; use a dispatch dependency"));
            }
        }
    }
    Ok(())
}
pub(super) fn validate_plan(
    plan: &Value,
    root: &Path,
    config: &Value,
    current: bool,
) -> Result<Value, String> {
    configuration(config)?;
    mapping(plan, "plan")?;
    let mut plan = plan.clone();
    if plan.get("schema_version").and_then(Value::as_u64) != Some(1) {
        return Err("plan schema_version must be 1".into());
    }
    id(at(&plan, &["id"]), "plan id")?;
    text(at(&plan, &["goal"]), "plan goal")?;
    if !["design", "implementation", "pr", "merge", "release"]
        .contains(&string(&plan, "delivery_target"))
    {
        return Err("delivery_target must be design, implementation, pr, merge or release".into());
    }
    if plan.get("creative_level").is_none() {
        put(
            &mut plan,
            &["creative_level"],
            config
                .pointer("/creative/default_level")
                .cloned()
                .unwrap_or(json!(2)),
        )?;
    }
    level(at(&plan, &["creative_level"]))?;
    strings(at(&plan, &["decisions"]), "plan decisions")?;
    let packages = strings(at(&plan, &["packages"]), "plan packages")?;
    let selected = list(config, "packages");
    if packages.iter().any(|p| !selected.contains(p)) {
        return Err("plan packages must be selected in project configuration".into());
    }
    mapping(at(&plan, &["versions"]), "versions")?;
    text(at(&plan, &["versions", "bevy"]), "versions.bevy")?;
    if let Some(version) = at(&plan, &["versions"])
        .get("gamekit")
        .filter(|v| !v.is_null())
    {
        text(version, "versions.gamekit")?;
    }
    mapping(at(&plan, &["repository"]), "repository")?;
    let actual = repository_identity(root)?;
    let claimed = text(at(&plan, &["repository", "root"]), "repository.root")?;
    if Path::new(claimed)
        .canonicalize()
        .map_err(|e| e.to_string())?
        != Path::new(string(&actual, "root"))
    {
        return Err("plan repository.root does not match the configured checkout".into());
    }
    put(
        &mut plan,
        &["repository", "root"],
        at(&actual, &["root"]).clone(),
    )?;
    if at(&plan, &["repository"])
        .get("common_dir")
        .is_some_and(|v| Some(v) != actual.get("common_dir"))
    {
        return Err("plan repository.common_dir does not match git".into());
    }
    put(
        &mut plan,
        &["repository", "common_dir"],
        at(&actual, &["common_dir"]).clone(),
    )?;
    for key in ["base_commit", "source_commit"] {
        commit(
            root,
            at(&plan, &["repository", key]),
            &format!("repository.{key}"),
        )?;
    }
    if current && at(&plan, &["repository"]).get("source_commit") != actual.get("head") {
        return Err("plan source_commit is stale relative to the checkout HEAD".into());
    }
    if !ancestor(
        root,
        string(at(&plan, &["repository"]), "base_commit"),
        string(at(&plan, &["repository"]), "source_commit"),
    )? {
        return Err("plan source_commit does not descend from base_commit".into());
    }
    let orders = at(&plan, &["orders"])
        .as_array()
        .filter(|o| !o.is_empty())
        .ok_or("plan orders must be a nonempty list")?
        .iter()
        .map(|order| validate_order(order, &plan, config))
        .collect::<Result<Vec<_>, _>>()?;
    let specs: BTreeMap<_, _> = orders
        .iter()
        .map(|o| (string(o, "id").to_owned(), o.clone()))
        .collect();
    if specs.len() != orders.len() {
        return Err("duplicate order ids".into());
    }
    validate_graph(&specs, None)?;
    put(&mut plan, &["orders"], json!(orders))?;
    Ok(plan)
}
