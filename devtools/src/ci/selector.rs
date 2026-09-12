//! Conservative dependency ownership derived solely from committed Git objects.

use super::{git, Selection};
use proc_macro2::{TokenStream, TokenTree};
use regex::Regex;
use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;
use std::process::Command;

type Names = BTreeSet<String>;
type Graph = BTreeMap<String, Names>;

struct Workspace {
    packages: BTreeMap<String, String>,
    consumers: Graph,
    included: Graph,
    dynamic: bool,
}

fn commit(root: &Path, revision: &str) -> Result<String, String> {
    if !matches!(revision.len(), 40 | 64)
        || !revision
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
    {
        return Err("expected a full commit object ID".into());
    }
    git(
        root,
        &["rev-parse", "--verify", &format!("{revision}^{{commit}}")],
    )
    .map(|value| value.trim().to_owned())
}

fn read_at(root: &Path, revision: &str, path: &str) -> Result<String, String> {
    git(root, &["show", &format!("{revision}:{path}")])
}

fn changed_paths(root: &Path, base: &str, head: &str) -> Result<Vec<String>, String> {
    let source = git(
        root,
        &["diff", "--name-status", "--find-renames", "-z", base, head],
    )?;
    let mut fields = source.split_terminator('\0');
    let mut paths = BTreeSet::new();
    while let Some(status) = fields.next() {
        let count = match status.as_bytes().first() {
            Some(b'R' | b'C') => 2,
            Some(b'A' | b'D' | b'M' | b'T' | b'U' | b'X' | b'B') => 1,
            _ => return Err("unsupported Git diff status".into()),
        };
        for _ in 0..count {
            let path = fields.next().ok_or("incomplete Git diff record")?;
            if path.is_empty() || path.contains(['\n', '\r']) {
                return Err("unsupported changed path".into());
            }
            paths.insert(path.to_owned());
        }
    }
    Ok(paths.into_iter().collect())
}

// Git paths always use POSIX separators, even on Windows. Reject absolute and
// escaping dependency/include paths instead of interpreting them on the host.
fn relative(parent: &str, path: &str) -> Result<String, String> {
    if path.starts_with('/') || path.contains(['\\', ':', '\0', '\n', '\r']) {
        return Err(format!("unsupported local input path: {path:?}"));
    }
    let mut parts = Vec::new();
    for part in parent.split('/').chain(path.split('/')) {
        match part {
            "" | "." => {}
            ".." => {
                parts.pop().ok_or("local input escapes repository")?;
            }
            _ => parts.push(part),
        }
    }
    Ok(parts.join("/"))
}

// Component-wise globbing preserves the old selector's single-directory '*' rule.
// Uncertain bracket syntax is an error, which selects the full suite.
fn glob(pattern: &str) -> Result<Regex, String> {
    let mut regex = String::from("^");
    let mut chars = pattern.chars().peekable();
    while let Some(ch) = chars.next() {
        match ch {
            '*' => regex.push_str("[^/]*"),
            '?' => regex.push_str("[^/]"),
            '[' => {
                regex.push('[');
                if chars.peek() == Some(&'!') {
                    chars.next();
                    regex.push('^');
                }
                let mut closed = false;
                for (count, item) in chars.by_ref().enumerate() {
                    if item == ']' && count > 0 {
                        closed = true;
                        break;
                    }
                    if matches!(item, '[' | '&' | '~' | '^' | '\\' | '/') {
                        return Err(format!("unsupported workspace glob: {pattern}"));
                    }
                    if item == ']' {
                        regex.push('\\');
                    }
                    regex.push(item);
                }
                if !closed {
                    return Err(format!("unclosed workspace glob: {pattern}"));
                }
                regex.push(']');
            }
            _ => regex.push_str(&regex::escape(&ch.to_string())),
        }
    }
    regex.push('$');
    Regex::new(&regex).map_err(|error| format!("invalid workspace glob {pattern}: {error}"))
}

fn patterns(settings: &toml::Table, key: &str, required: bool) -> Result<Vec<Regex>, String> {
    match settings.get(key) {
        Some(value) => value
            .as_array()
            .ok_or_else(|| format!("workspace.{key} must be an array"))?
            .iter()
            .map(|value| {
                let pattern = value.as_str().ok_or("workspace patterns must be strings")?;
                if pattern.starts_with('/')
                    || pattern
                        .split('/')
                        .any(|part| matches!(part, "" | "." | ".."))
                {
                    return Err(format!("unsupported workspace glob: {pattern}"));
                }
                glob(pattern)
            })
            .collect(),
        None if !required => Ok(Vec::new()),
        None => Err(format!("missing workspace.{key}")),
    }
}

fn dependencies<'a>(
    manifest: &'a toml::Table,
    result: &mut Vec<(&'a str, &'a toml::Value)>,
) -> Result<(), String> {
    for key in ["dependencies", "dev-dependencies", "build-dependencies"] {
        if let Some(value) = manifest.get(key) {
            let table = value
                .as_table()
                .ok_or_else(|| format!("{key} must be a table"))?;
            result.extend(table.iter().map(|(alias, value)| (alias.as_str(), value)));
        }
    }
    if let Some(value) = manifest.get("target") {
        for value in value.as_table().ok_or("target must be a table")?.values() {
            dependencies(
                value.as_table().ok_or("target settings must be tables")?,
                result,
            )?;
        }
    }
    Ok(())
}

impl Workspace {
    fn owner(&self, path: &str) -> Option<&str> {
        self.packages
            .iter()
            .filter(|(_, parent)| {
                path.strip_prefix(parent.as_str())
                    .is_some_and(|tail| tail.starts_with('/'))
            })
            .max_by_key(|(_, parent)| parent.len())
            .map(|(name, _)| name.as_str())
    }
}

fn includes(stream: TokenStream, targets: &mut Vec<Option<String>>) {
    let mut tokens = stream.into_iter().peekable();
    while let Some(token) = tokens.next() {
        match token {
            TokenTree::Ident(ident)
                if matches!(
                    ident.to_string().trim_start_matches("r#"),
                    "include" | "include_str" | "include_bytes"
                ) && matches!(tokens.peek(), Some(TokenTree::Punct(punct)) if punct.as_char() == '!') =>
            {
                tokens.next();
                let target = match tokens.next() {
                    Some(TokenTree::Group(group)) => {
                        let mut arguments = group.stream().into_iter();
                        let literal = match arguments.next() {
                            Some(TokenTree::Literal(literal)) => {
                                syn::parse_str::<syn::LitStr>(&literal.to_string())
                                    .ok()
                                    .map(|value| value.value())
                            }
                            _ => None,
                        };
                        let tail_ok = match arguments.next() {
                            None => true,
                            Some(TokenTree::Punct(punct)) if punct.as_char() == ',' => {
                                arguments.next().is_none()
                            }
                            _ => false,
                        };
                        literal.filter(|_| tail_ok)
                    }
                    _ => None,
                };
                targets.push(target);
            }
            TokenTree::Group(group) => includes(group.stream(), targets),
            _ => {}
        }
    }
}

fn workspace(root: &Path, revision: &str) -> Result<Workspace, String> {
    let source = git(root, &["ls-tree", "-r", "--name-only", "-z", revision])?;
    let paths: Vec<_> = source.split_terminator('\0').collect();
    let root_manifest: toml::Table = toml::from_str(&read_at(root, revision, "Cargo.toml")?)
        .map_err(|error| error.to_string())?;
    let settings = root_manifest
        .get("workspace")
        .and_then(toml::Value::as_table)
        .ok_or("missing workspace table")?;
    let members = patterns(settings, "members", true)?;
    let excluded = patterns(settings, "exclude", false)?;
    let mut result = Workspace {
        packages: BTreeMap::new(),
        consumers: BTreeMap::new(),
        included: BTreeMap::new(),
        dynamic: false,
    };
    let mut manifests = BTreeMap::new();
    for path in &paths {
        let Some(parent) = path.strip_suffix("/Cargo.toml") else {
            continue;
        };
        if !members.iter().any(|pattern| pattern.is_match(parent))
            || excluded.iter().any(|pattern| pattern.is_match(parent))
        {
            continue;
        }
        let manifest: toml::Table =
            toml::from_str(&read_at(root, revision, path)?).map_err(|error| error.to_string())?;
        let package = manifest
            .get("package")
            .and_then(toml::Value::as_table)
            .ok_or("missing package table")?;
        let name = package
            .get("name")
            .and_then(toml::Value::as_str)
            .ok_or("missing package.name")?;
        if !valid_name(name)
            || result
                .packages
                .insert(name.to_owned(), parent.to_owned())
                .is_some()
        {
            return Err("invalid or duplicate workspace package name".into());
        }
        result.dynamic |= package
            .get("build")
            .is_some_and(|value| value.as_bool() != Some(false));
        manifests.insert(name.to_owned(), manifest);
    }
    if result.packages.is_empty() {
        return Err("no recognized workspace packages".into());
    }
    let by_path: BTreeMap<_, _> = result
        .packages
        .iter()
        .map(|(name, path)| (path.as_str(), name.as_str()))
        .collect();
    for (name, manifest) in &manifests {
        let mut values = Vec::new();
        dependencies(manifest, &mut values)?;
        for (alias, value) in values {
            let mut parent = result
                .packages
                .get(name)
                .ok_or("missing package directory")?
                .as_str();
            let mut value = value;
            if let Some(inherited) = value.as_table().and_then(|table| table.get("workspace")) {
                match inherited.as_bool() {
                    Some(true) => {
                        value = settings
                            .get("dependencies")
                            .and_then(toml::Value::as_table)
                            .and_then(|table| table.get(alias))
                            .ok_or("unknown workspace dependency alias")?;
                        parent = "";
                    }
                    Some(false) => {}
                    None => return Err("dependency.workspace must be boolean".into()),
                }
            }
            match value {
                toml::Value::String(_) => {}
                toml::Value::Table(table) => {
                    if let Some(path) = table.get("path") {
                        let path = relative(
                            parent,
                            path.as_str().ok_or("dependency.path must be a string")?,
                        )?;
                        let dependency = by_path.get(path.as_str()).ok_or_else(|| {
                            format!("local dependency outside recognized workspace: {path}")
                        })?;
                        result
                            .consumers
                            .entry((*dependency).to_owned())
                            .or_default()
                            .insert(name.clone());
                    }
                }
                _ => return Err("dependency must be a version string or table".into()),
            }
        }
    }
    result.dynamic |= paths.iter().any(|path| path.ends_with("/build.rs"));
    let candidates = Command::new("git")
        .args(["grep", "-l", "-z", "-e", "include", revision, "--", "*.rs"])
        .current_dir(root)
        .output()
        .map_err(|error| error.to_string())?;
    if !candidates.status.success() && candidates.status.code() != Some(1) {
        return Err(format!(
            "cannot inspect source inclusions: {}",
            String::from_utf8_lossy(&candidates.stderr)
        ));
    }
    let candidates = String::from_utf8(candidates.stdout).map_err(|error| error.to_string())?;
    for candidate in candidates.split_terminator('\0') {
        let (_, path) = candidate.split_once(':').ok_or("malformed Git grep path")?;
        let stream: TokenStream = read_at(root, revision, path)?
            .parse()
            .map_err(|error| format!("unsupported Rust input {path}: {error}"))?;
        let mut targets = Vec::new();
        includes(stream, &mut targets);
        let owner = result.owner(path).map(str::to_owned);
        for target in targets {
            if let (Some(target), Some(owner)) = (target, &owner) {
                let parent = path.rsplit_once('/').map_or("", |(parent, _)| parent);
                result
                    .included
                    .entry(relative(parent, &target)?)
                    .or_default()
                    .insert(owner.clone());
            } else {
                result.dynamic = true;
            }
        }
    }
    Ok(result)
}

fn valid_name(name: &str) -> bool {
    name.as_bytes()
        .first()
        .is_some_and(|byte| byte.is_ascii_alphanumeric() || *byte == b'_')
        && name
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'_' | b'-'))
}

fn ci_input(path: &str) -> bool {
    matches!(
        path,
        "Cargo.toml"
            | "Cargo.lock"
            | "deny.toml"
            | "rust-toolchain"
            | "rust-toolchain.toml"
            | "devtools/src/lib.rs"
            | "devtools/src/main.rs"
            | "devtools/src/support.rs"
    ) || path.ends_with("/Cargo.toml")
        || [
            ".github/",
            ".cargo/",
            "scripts/ci",
            "scripts/tests/test_ci",
            "devtools/src/ci/",
            "devtools/tests/ci",
            "devtools/tests/fixtures/ci/",
        ]
        .iter()
        .any(|prefix| path.starts_with(prefix))
}

fn compare(root: &Path, base: &str, head: &str) -> Result<Selection, String> {
    let base = commit(root, base)?;
    let paths = changed_paths(root, &base, head)?;
    let old = workspace(root, &base)?;
    let new = workspace(root, head)?;
    if !paths.is_empty() && (old.dynamic || new.dynamic) {
        return Err(
            "dynamic or unsupported include/build-script inputs cannot be scoped safely".into(),
        );
    }
    let mut result = Selection {
        schema_version: 1,
        head: head.into(),
        base: Some(base),
        full: false,
        packages: Vec::new(),
        paths,
        reasons: Vec::new(),
        skills: false,
        rust: false,
        policy: false,
        distribution: false,
        minimal: false,
        wasm: false,
        deny: false,
    };
    let mut affected = Names::new();
    for path in &result.paths {
        let owner = new.owner(path).or_else(|| old.owner(path));
        if ci_input(path) {
            return Err(format!("shared configuration or CI input: {path}"));
        }
        let included: Names = old
            .included
            .get(path)
            .into_iter()
            .flatten()
            .chain(new.included.get(path).into_iter().flatten())
            .cloned()
            .collect();
        if !included.is_empty() {
            affected.extend(included.iter().cloned());
            result.reasons.push(format!(
                "{path}: compiled input to {}",
                included.iter().cloned().collect::<Vec<_>>().join(", ")
            ));
        }
        let prose = matches!(
            path.as_str(),
            "README.md"
                | "gamekit/README.md"
                | "gameskills/README.md"
                | "LICENSE"
                | "LICENSE-MIT"
                | "LICENSE-APACHE"
                | "CHANGELOG.md"
        );
        if matches!(path.as_str(), "gameskills.toml" | "gameskills.lock.json")
            || [
                "plugins/",
                "skills/",
                "gameskills/plugins/",
                "gameskills/legacy/",
                ".claude-plugin/",
                ".codex-plugin/",
                ".agents/",
            ]
            .iter()
            .any(|prefix| path.starts_with(prefix))
        {
            result.skills = true;
            result.reasons.push(format!(
                "{path}: GameSkills instructions, packaging or runtime"
            ));
        } else if path.ends_with(".md") || prose {
            if !included.is_empty() {
                affected.extend(owner.map(str::to_owned));
            } else if owner.is_some()
                || ["docs/", "gamekit/docs/", "gameskills/docs/"]
                    .iter()
                    .any(|prefix| path.starts_with(prefix))
                || prose
            {
                result
                    .reasons
                    .push(format!("{path}: narrative documentation"));
            } else {
                return Err(format!("unmapped Markdown: {path}"));
            }
        } else if let Some(owner) = owner {
            affected.insert(owner.to_owned());
            result.reasons.push(format!("{path}: package {owner}"));
        } else if matches!(
            path.as_str(),
            "scripts/check_distribution.py"
                | "scripts/fixtures/gamekit_consumer.rs"
                | "scripts/tests/test_check_distribution.py"
        ) {
            result.rust = true;
            result.policy = true;
            result.distribution = true;
            affected.extend(
                new.packages
                    .iter()
                    .filter(|(_, path)| path.starts_with("gamekit/") || path.starts_with("crates/"))
                    .map(|(name, _)| name.clone()),
            );
            result
                .reasons
                .push(format!("{path}: external library distribution contract"));
        } else if matches!(
            path.as_str(),
            "scripts/check_repo.py" | "scripts/tests/test_check_repo.py"
        ) {
            result.skills = true;
            result
                .reasons
                .push(format!("{path}: always-run repository checks"));
        } else if included.is_empty() {
            return Err(format!("unmapped input: {path}"));
        }
    }
    loop {
        let previous = affected.len();
        let consumers: Names = affected
            .iter()
            .flat_map(|name| {
                old.consumers
                    .get(name)
                    .into_iter()
                    .flatten()
                    .chain(new.consumers.get(name).into_iter().flatten())
            })
            .cloned()
            .collect();
        affected.extend(consumers);
        if previous == affected.len() {
            break;
        }
    }
    if affected.iter().any(|name| !new.packages.contains_key(name)) {
        return Err("changed or removed package graph".into());
    }
    if !affected.is_empty() {
        result.rust = true;
        result.policy = true;
    }
    if affected.contains("repo-devtools") {
        result.skills = true;
        result.distribution = true;
        result.reasons.push(
            "repository validator implementation: native catalogs and external consumer contract"
                .into(),
        );
    }
    if affected.iter().any(|name| {
        new.packages
            .get(name)
            .is_some_and(|path| path.starts_with("gamekit/") || path.starts_with("crates/"))
    }) {
        result.distribution = true;
        result.minimal = true;
    }
    result.wasm = affected.iter().any(|name| {
        matches!(
            name.as_str(),
            "bevy-gamekit-hex"
                | "bevy-gamekit-turns"
                | "bevy-gamekit-session"
                | "bevy-gamekit-ui"
                | "labyrinth-rules"
        )
    });
    result.packages = affected.into_iter().collect();
    Ok(result)
}

pub(super) fn select(
    root: &Path,
    base: Option<&str>,
    head: &str,
    full: bool,
) -> Result<Selection, String> {
    let head = commit(root, head)?;
    let requested = base.map(str::to_owned);
    if full {
        return Ok(Selection::full(
            head,
            requested,
            "Manual full-suite run".into(),
        ));
    }
    Ok(
        compare(root, base.unwrap_or_default(), &head).unwrap_or_else(|error| {
            Selection::full(head, requested, format!("Conservative fallback: {error}"))
        }),
    )
}
