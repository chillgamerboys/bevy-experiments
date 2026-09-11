//! Structural checks for the native GameSkills candidate and unexecuted scenario rubrics.

use crate::{markdown, support};
use regex::Regex;
use serde_json::Value;
use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};

/// Native packages and their exact supported skill inventories.
pub const EXPECTED_SKILLS: &[(&str, &[&str])] = &[
    (
        "gameskills",
        &[
            "setup",
            "plan",
            "dispatch",
            "debug",
            "test",
            "playtest",
            "review",
            "update-docs",
            "create-pr",
            "audit-pr",
            "merge-pr",
            "release",
        ],
    ),
    ("gameskills-ui", &["build-ui", "verify-ui"]),
    ("gameskills-turn-based", &["model-rules"]),
    (
        "gameskills-multiplayer",
        &["design-multiplayer", "verify-multiplayer"],
    ),
    (
        "gameskills-maintainer",
        &["evolve-gamekit", "author-skill", "evaluate-skills"],
    ),
    ("gameskills-bevy-contrib", &["prepare-contribution"]),
];

/// Passing this validator is not evidence of native installation or agent behavior.
pub const NOTICE: &str = "Structural checks and scenario fixtures do not prove native installation, skill selection, forward behavior, review acceptance, or measured cost savings.";

const REQUIRED_SCENARIO_TAGS: &[&str] = &[
    "routing",
    "bounded-dispatch",
    "economical-roles",
    "stale-evidence",
    "inject-preservation",
    "optional-absence",
    "human-upstream",
];

fn matches(pattern: &str, text: &str) -> bool {
    Regex::new(pattern)
        .expect("structural validator regex is valid")
        .is_match(text)
}

fn valid_name(name: &str) -> bool {
    matches(r"\A[a-z0-9]+(?:-[a-z0-9]+)*\z", name)
}

pub(crate) fn json(path: &Path, failures: &mut Vec<String>) -> Value {
    support::read_json(path).unwrap_or_else(|error| {
        failures.push(error);
        Value::Null
    })
}

fn member<'a>(value: &'a Value, key: &str) -> &'a Value {
    value.get(key).unwrap_or(&Value::Null)
}

fn strings(value: &Value) -> Option<Vec<&str>> {
    value
        .as_array()?
        .iter()
        .map(|item| item.as_str().filter(|text| !text.trim().is_empty()))
        .collect()
}

fn descriptive(value: &Value) -> bool {
    value.as_str().is_some_and(|text| {
        (24..=1024).contains(&text.trim().chars().count()) && text.split_whitespace().count() >= 4
    })
}

fn inventory(value: &Value, expected: &[&str], label: &str, failures: &mut Vec<String>) {
    let Some(values) = strings(value) else {
        failures.push(format!("{label}: expected a string list"));
        return;
    };
    let actual: BTreeSet<_> = values.iter().copied().collect();
    let expected: BTreeSet<_> = expected.iter().copied().collect();
    if values.len() != actual.len() || actual != expected {
        failures.push(format!(
            "{label}: inventory mismatch; expected {expected:?}, got {values:?}"
        ));
    }
}

fn scalar(raw: &str, continuation: &[String]) -> Result<String, String> {
    let raw = raw.trim();
    if [">", ">-", ">+", "|", "|-", "|+"].contains(&raw) {
        let value = continuation
            .iter()
            .map(|line| line.trim())
            .collect::<Vec<_>>()
            .join("\n");
        return Ok(if raw.starts_with('>') {
            value.split_whitespace().collect::<Vec<_>>().join(" ")
        } else {
            value.trim().to_owned()
        });
    }
    if raw.starts_with('"') {
        return serde_json::from_str::<String>(raw).map_err(|error| error.to_string());
    }
    if let Some(value) = raw.strip_prefix('\'') {
        return value
            .strip_suffix('\'')
            .map(|value| value.replace("''", "'"))
            .ok_or_else(|| "unterminated single-quoted string".to_owned());
    }
    if raw.is_empty()
        || raw.starts_with(['[', '{', '&', '*', '!'])
        || ["null", "true", "false", "~"].contains(&raw)
    {
        return Err("required field must be a string".to_owned());
    }
    if matches(r":\s", raw) {
        return Err("quote a required string containing colon-space".to_owned());
    }
    let comment = Regex::new(r"\s+#").map_err(|error| error.to_string())?;
    let value = comment
        .split(raw)
        .next()
        .unwrap_or_default()
        .trim()
        .to_owned();
    Ok(value)
}

fn frontmatter(content: &str) -> Result<(String, String, String), String> {
    let mut lines = content.lines();
    if lines.next() != Some("---") {
        return Err("missing frontmatter opening delimiter".to_owned());
    }
    let lines: Vec<_> = lines.collect();
    let end = lines
        .iter()
        .position(|line| *line == "---")
        .ok_or("missing frontmatter closing delimiter")?;
    let (header, body) = lines.split_at(end);
    let field = Regex::new(r"\A([A-Za-z][A-Za-z0-9_-]*):(?:\s+(.*))?\z")
        .map_err(|error| error.to_string())?;
    let mut fields = BTreeMap::<String, (String, Vec<String>)>::new();
    let mut current = None;
    for &line in header {
        if line.trim().is_empty() || line.trim_start().starts_with('#') {
            continue;
        }
        if line.starts_with('\t') {
            return Err("tab-indented frontmatter is unsupported".to_owned());
        }
        if line.starts_with(' ') {
            let Some((_, continuation)) = current.as_ref().and_then(|key| fields.get_mut(key))
            else {
                return Err("unexpected indented frontmatter".to_owned());
            };
            continuation.push(line.to_owned());
            continue;
        }
        let captures = field
            .captures(line)
            .ok_or_else(|| format!("invalid top-level frontmatter line: {line}"))?;
        let key = captures
            .get(1)
            .map_or("", |capture| capture.as_str())
            .to_owned();
        let raw = captures
            .get(2)
            .map_or("", |capture| capture.as_str())
            .to_owned();
        if fields.insert(key.clone(), (raw, Vec::new())).is_some() {
            return Err(format!("duplicate frontmatter key {key}"));
        }
        current = Some(key);
    }
    let required = |key: &str| {
        let (raw, continuation) = fields
            .get(key)
            .ok_or_else(|| format!("missing required frontmatter {key}"))?;
        scalar(raw, continuation)
    };
    Ok((
        required("name")?,
        required("description")?,
        body.iter()
            .skip(1)
            .copied()
            .collect::<Vec<_>>()
            .join("\n")
            .trim()
            .to_owned(),
    ))
}

fn skill(path: &Path, name: &str, failures: &mut Vec<String>) {
    match support::read_text(path)
        .and_then(|content| frontmatter(&content).map(|fields| (content, fields)))
    {
        Ok((content, (found_name, description, body))) => {
            if found_name != name || !valid_name(&found_name) || name.len() > 64 {
                failures.push(format!(
                    "{}: frontmatter name must match skill directory {name}",
                    path.display()
                ));
            }
            if !descriptive(&Value::String(description)) {
                failures.push(format!(
                    "{}: description must be a descriptive string of 24–1024 characters",
                    path.display()
                ));
            }
            if body.split_whitespace().count() < 12 {
                failures.push(format!(
                    "{}: skill body is empty or unfinished",
                    path.display()
                ));
            }
            if matches(r"\b(?:TODO|FIXME|TBD)\b|\[INSERT[^\]]*\]", &content) {
                failures.push(format!(
                    "{}: unfinished scaffold placeholder",
                    path.display()
                ));
            }
        }
        Err(error) => failures.push(format!("{}: {error}", path.display())),
    }
    let metadata = path.parent().unwrap_or(path).join("agents/openai.yaml");
    if metadata.symlink_metadata().is_ok() {
        match support::read_text(&metadata) {
            Ok(text) => {
                if !matches(r"(?m)^interface:\s*$", &text) {
                    failures.push(format!(
                        "{}: expected an interface mapping",
                        metadata.display()
                    ));
                }
                for field in ["display_name", "short_description"] {
                    if !matches(&format!(r"(?m)^  {field}:\s*\S"), &text) {
                        failures.push(format!("{}: missing interface {field}", metadata.display()));
                    }
                }
                if let Some(prompt) = text
                    .lines()
                    .find_map(|line| line.strip_prefix("  default_prompt:"))
                {
                    if !prompt.contains(&format!("${name}"))
                        && !prompt.contains(&format!(":{name}"))
                    {
                        failures.push(format!(
                            "{}: default_prompt does not reference this skill",
                            metadata.display()
                        ));
                    }
                }
            }
            Err(error) => failures.push(error),
        }
    }
}

fn target(
    boundary: &Path,
    source: &Path,
    raw: &str,
    failures: &mut Vec<String>,
) -> Option<PathBuf> {
    match support::local_target(boundary, source, raw) {
        Ok(target) => target,
        Err(error) => {
            failures.push(format!("{}: {error}", source.display()));
            None
        }
    }
}

fn resolves_to(actual: Option<PathBuf>, expected: &Path) -> bool {
    actual.is_some_and(|actual| {
        expected
            .canonicalize()
            .is_ok_and(|expected| actual == expected)
    })
}

fn manifests(package: &Path, name: &str, version: &Value, failures: &mut Vec<String>) {
    for client in ["codex", "claude"] {
        let path = package.join(format!(".{client}-plugin/plugin.json"));
        let data = json(&path, failures);
        if !data.is_object() {
            continue;
        }
        let label = path.display();
        if data.get("name").and_then(Value::as_str) != Some(name) {
            failures.push(format!("{label}: package name mismatch"));
        }
        if data.get("version") != Some(version) {
            failures.push(format!("{label}: version does not match catalog"));
        }
        if !descriptive(member(&data, "description")) {
            failures.push(format!("{label}: missing descriptive package description"));
        }
        if data.get("author").is_some_and(|author| {
            !author.is_object() || author.get("name").and_then(Value::as_str).is_none()
        }) {
            failures.push(format!("{label}: author must be an object with a name"));
        }
        let skills = match data.get("skills") {
            Some(value) => value.as_str(),
            None if client == "claude" => Some("./skills/"),
            None => None,
        };
        if let Some(skills) = skills {
            if !resolves_to(
                target(package, &package.join("manifest-root"), skills, failures),
                &package.join("skills"),
            ) {
                failures.push(format!(
                    "{label}: skills must resolve to this package's skills directory"
                ));
            }
        } else {
            failures.push(format!(
                "{label}: this candidate must discover its package-local skills directory"
            ));
        }
        if client == "codex" {
            if let Some(interface) = data.get("interface") {
                if interface.is_object() {
                    for key in [
                        "displayName",
                        "shortDescription",
                        "longDescription",
                        "developerName",
                        "category",
                        "defaultPrompt",
                    ] {
                        if interface.get(key).is_some_and(|value| {
                            value.as_str().is_none_or(|text| text.trim().is_empty())
                        }) {
                            failures.push(format!(
                                "{label}: interface.{key} must be a nonempty string"
                            ));
                        }
                    }
                    if interface
                        .get("capabilities")
                        .is_some_and(|value| strings(value).is_none())
                    {
                        failures.push(format!(
                            "{label}: interface.capabilities must be a string list"
                        ));
                    }
                } else {
                    failures.push(format!("{label}: interface must be an object"));
                }
            }
        }
    }
}

fn marketplaces(root: &Path, version: &Value, failures: &mut Vec<String>) {
    let expected: Vec<_> = EXPECTED_SKILLS.iter().map(|(name, _)| *name).collect();
    for (client, relative) in [
        ("codex", ".agents/plugins/marketplace.json"),
        ("claude", ".claude-plugin/marketplace.json"),
    ] {
        let path = root.join(relative);
        let data = json(&path, failures);
        let label = path.display();
        let Some(entries) = data
            .get("plugins")
            .and_then(Value::as_array)
            .filter(|entries| entries.iter().all(Value::is_object))
        else {
            failures.push(format!("{label}: plugins must be an object list"));
            continue;
        };
        inventory(
            &Value::Array(
                entries
                    .iter()
                    .map(|entry| member(entry, "name").clone())
                    .collect(),
            ),
            &expected,
            &format!("{label} packages"),
            failures,
        );
        for entry in entries {
            let Some(name) = entry
                .get("name")
                .and_then(Value::as_str)
                .filter(|name| expected.contains(name))
            else {
                continue;
            };
            let mut source = member(entry, "source");
            if client == "codex" {
                if !source.is_object()
                    || source.get("source").and_then(Value::as_str) != Some("local")
                {
                    failures.push(format!("{label}: {name} must use a local package source"));
                    continue;
                }
                source = member(source, "path");
            }
            let Some(source) = source.as_str() else {
                failures.push(format!("{label}: {name} missing source path"));
                continue;
            };
            if !resolves_to(
                target(root, &root.join("marketplace-root"), source, failures),
                &root.join("plugins").join(name),
            ) {
                failures.push(format!(
                    "{label}: {name} source does not resolve to its package"
                ));
            }
            if entry.get("version").is_some_and(|value| value != version) {
                failures.push(format!("{label}: {name} version differs from catalog"));
            }
        }
    }
}

/// Check evaluation inputs and observable rubrics, without scoring fixture behavior.
pub fn validate_scenarios(path: &Path) -> Vec<String> {
    let mut failures = Vec::new();
    let data = json(path, &mut failures);
    let label = path.display();
    if data.get("schema_version").and_then(Value::as_u64) != Some(1)
        || data.get("evidence_status").and_then(Value::as_str) != Some("unexecuted-rubrics")
    {
        failures.push(format!(
            "{label}: require schema_version 1 and unexecuted-rubrics evidence_status"
        ));
    }
    let Some(scenarios) = data
        .get("scenarios")
        .and_then(Value::as_array)
        .filter(|scenarios| !scenarios.is_empty())
    else {
        failures.push(format!("{label}: missing scenarios"));
        return failures;
    };
    let logical: BTreeSet<String> = EXPECTED_SKILLS
        .iter()
        .flat_map(|(package, skills)| skills.iter().map(move |skill| format!("{package}:{skill}")))
        .collect();
    let mut ids = BTreeSet::new();
    let mut tags = BTreeSet::new();
    for scenario in scenarios {
        if !scenario.is_object() {
            failures.push(format!("{label}: scenario must be an object"));
            continue;
        }
        let identity = member(scenario, "id");
        let label = format!("{label} scenario {identity}");
        if identity
            .as_str()
            .is_none_or(|identity| !valid_name(identity) || !ids.insert(identity))
        {
            failures.push(format!("{label}: invalid or duplicate id"));
        }
        for key in ["prompt", "fixture", "allowed_actions"] {
            if scenario
                .get(key)
                .and_then(Value::as_str)
                .is_none_or(|text| text.trim().is_empty())
            {
                failures.push(format!("{label}: missing {key}"));
            }
        }
        if scenario.get("split").and_then(Value::as_str) != Some("held-out") {
            failures.push(format!("{label}: split must be held-out"));
        }
        if let Some(values) = strings(member(scenario, "tags")) {
            tags.extend(values);
        } else {
            failures.push(format!("{label}: tags must be a string list"));
        }
        let routing = member(scenario, "routing");
        if routing.is_object() {
            let primary = member(routing, "primary");
            if !primary.is_null()
                && primary
                    .as_str()
                    .is_none_or(|primary| !logical.contains(primary))
            {
                failures.push(format!("{label}: unknown primary skill {primary}"));
            }
            for key in ["allowed", "avoid"] {
                if strings(member(routing, key))
                    .is_none_or(|values| values.iter().any(|value| !logical.contains(*value)))
                {
                    failures.push(format!("{label}: invalid routing {key}"));
                }
            }
            if let (Some(allowed), Some(avoid)) = (
                strings(member(routing, "allowed")),
                strings(member(routing, "avoid")),
            ) {
                if allowed.iter().any(|value| avoid.contains(value))
                    || primary
                        .as_str()
                        .is_some_and(|primary| avoid.contains(&primary))
                {
                    failures.push(format!("{label}: contradictory routing rubric"));
                }
            }
        } else {
            failures.push(format!("{label}: missing routing rubric"));
        }
        let rubric = member(scenario, "rubric");
        if rubric.is_object() {
            for key in ["must_observe", "must_not_observe", "evidence"] {
                if strings(member(rubric, key)).is_none_or(|values| values.is_empty()) {
                    failures.push(format!("{label}: missing nonempty rubric {key}"));
                }
            }
            if let (Some(observe), Some(avoid)) = (
                strings(member(rubric, "must_observe")),
                strings(member(rubric, "must_not_observe")),
            ) {
                if observe.iter().any(|value| avoid.contains(value)) {
                    failures.push(format!("{label}: contradictory observation rubric"));
                }
            }
        } else {
            failures.push(format!("{label}: missing observation rubric"));
        }
    }
    let missing: Vec<_> = REQUIRED_SCENARIO_TAGS
        .iter()
        .filter(|tag| !tags.contains(**tag))
        .collect();
    if !missing.is_empty() {
        failures.push(format!(
            "{}: missing scenario coverage tags {missing:?}",
            path.display()
        ));
    }
    failures
}

// Full package walk: ignored repository-directory names still matter in a native package.
// Never traverse a symlink directory or accept an escape through a symlink leaf.
pub(crate) fn package_files(package: &Path, failures: &mut Vec<String>) -> Vec<PathBuf> {
    let Ok(boundary) = package.canonicalize() else {
        failures.push(format!("{}: missing package directory", package.display()));
        return Vec::new();
    };
    if package.is_symlink() {
        failures.push(format!(
            "{}: native package root cannot be a symlink",
            package.display()
        ));
        return Vec::new();
    }
    let mut pending = vec![package.to_owned()];
    let mut files = Vec::new();
    while let Some(directory) = pending.pop() {
        let entries = match std::fs::read_dir(&directory) {
            Ok(entries) => entries,
            Err(error) => {
                failures.push(format!("{}: {error}", directory.display()));
                continue;
            }
        };
        let mut paths = Vec::new();
        for entry in entries {
            match entry {
                Ok(entry) => paths.push(entry.path()),
                Err(error) => failures.push(format!("{}: {error}", directory.display())),
            }
        }
        paths.sort();
        for path in paths {
            if path.is_symlink() {
                match path.canonicalize() {
                    Ok(target) if !target.starts_with(&boundary) => failures.push(format!(
                        "{}: symlink escapes native package",
                        path.display()
                    )),
                    Ok(_) => failures.push(format!(
                        "{}: native package entries must not be symlinks",
                        path.display()
                    )),
                    Err(error) => failures.push(format!(
                        "{}: unresolvable package symlink: {error}",
                        path.display()
                    )),
                }
            } else if path.is_dir() {
                pending.push(path);
            } else if path.is_file() {
                files.push(path);
            } else {
                failures.push(format!(
                    "{}: expected an ordinary package file or directory",
                    path.display()
                ));
            }
        }
    }
    files.sort();
    files
}

fn directories(path: &Path, failures: &mut Vec<String>) -> Value {
    let mut names = Vec::new();
    match std::fs::read_dir(path) {
        Ok(entries) => {
            for entry in entries {
                match entry {
                    Ok(entry) if entry.path().is_dir() => {
                        names.push(entry.file_name().to_string_lossy().into_owned())
                    }
                    Ok(_) => {}
                    Err(error) => failures.push(format!("{}: {error}", path.display())),
                }
            }
        }
        Err(error) => failures.push(format!("{}: {error}", path.display())),
    }
    names.sort();
    names.into()
}

/// Return deterministic structural failures for a repository candidate.
/// This performs no native client execution, agent evaluation, or Python subprocesses.
pub fn validate(root: &Path) -> Vec<String> {
    let root = root.canonicalize().unwrap_or_else(|_| root.to_owned());
    let mut failures = Vec::new();
    let plugin_root = root.join("plugins");
    if plugin_root.is_symlink() {
        return vec![format!(
            "{}: native plugin directory cannot be a symlink",
            plugin_root.display()
        )];
    }
    let path = root.join("plugins/gameskills/catalog.json");
    let catalog = json(&path, &mut failures);
    if catalog.get("schema_version").and_then(Value::as_u64) != Some(1) {
        failures.push(format!("{}: schema_version must be 1", path.display()));
    }
    let version = member(&catalog, "version");
    if version.as_str().is_none_or(|version| {
        !matches(
            r"\A[0-9]+\.[0-9]+\.[0-9]+(?:-[0-9A-Za-z.-]+)?(?:\+[0-9A-Za-z.-]+)?\z",
            version,
        )
    }) {
        failures.push(format!(
            "{}: version must have semantic-version form",
            path.display()
        ));
    }
    let packages = member(&catalog, "packages");
    let names: Vec<_> = EXPECTED_SKILLS.iter().map(|(name, _)| *name).collect();
    let actual: Value = packages
        .as_object()
        .map(|object| object.keys().cloned().collect::<Vec<_>>())
        .unwrap_or_default()
        .into();
    inventory(
        &actual,
        &names,
        &format!("{} packages", path.display()),
        &mut failures,
    );
    inventory(
        &directories(&plugin_root, &mut failures),
        &names,
        "native package directories",
        &mut failures,
    );
    for &(name, expected) in EXPECTED_SKILLS {
        let package = plugin_root.join(name);
        let definition = member(packages, name);
        if !definition.is_object() {
            failures.push(format!(
                "{}: missing package definition {name}",
                path.display()
            ));
        }
        inventory(
            member(definition, "skills"),
            expected,
            &format!("catalog {name} skills"),
            &mut failures,
        );
        inventory(
            member(definition, "requires"),
            if name == "gameskills" {
                &[]
            } else {
                &["gameskills"]
            },
            &format!("catalog {name} requirements"),
            &mut failures,
        );
        if !descriptive(member(definition, "description")) {
            failures.push(format!(
                "catalog {name}: missing descriptive package description"
            ));
        }
        let files = package_files(&package, &mut failures);
        if package.is_symlink() {
            continue;
        }
        let skill_root = package.join("skills");
        inventory(
            &directories(&skill_root, &mut failures),
            expected,
            &format!("{name} native skill directories"),
            &mut failures,
        );
        let found: BTreeSet<_> = files
            .iter()
            .filter(|file| file.file_name().is_some_and(|name| name == "SKILL.md"))
            .filter_map(|file| file.parent()?.strip_prefix(&skill_root).ok())
            .map(|relative| relative.to_string_lossy().replace('\\', "/"))
            .collect();
        let expected_set: BTreeSet<_> = expected.iter().map(|name| (*name).to_owned()).collect();
        if found != expected_set {
            failures.push(format!(
                "{name}: discovered SKILL.md inventory mismatch: {found:?}"
            ));
        }
        for skill_name in expected {
            skill(
                &skill_root.join(skill_name).join("SKILL.md"),
                skill_name,
                &mut failures,
            );
        }
        manifests(&package, name, version, &mut failures);
        for file in files
            .iter()
            .filter(|file| file.extension().is_some_and(|extension| extension == "md"))
        {
            match support::read_text(file) {
                Ok(content) => {
                    for link in markdown::links(&content) {
                        target(&package, file, &link, &mut failures);
                    }
                }
                Err(error) => failures.push(error),
            }
        }
    }
    marketplaces(&root, version, &mut failures);
    failures.extend(validate_scenarios(
        &root.join("skills/tests/gameskills-scenarios.json"),
    ));
    failures
}
