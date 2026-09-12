//! Read-only workspace layout, documentation and capability dependency checks.

use crate::{markdown, support};
use std::collections::{BTreeMap, BTreeSet};
use std::path::{Component, Path, PathBuf};
use toml::Value;

fn manifest(path: &Path) -> Result<Value, String> {
    toml::from_str(&support::read_text(path)?)
        .map_err(|error| format!("{}: invalid Cargo manifest: {error}", path.display()))
}

fn package_name(manifest: &Value) -> Option<&str> {
    manifest.get("package")?.get("name")?.as_str()
}

fn dependencies<'a>(manifest: &'a Value, output: &mut Vec<(&'a str, &'a Value)>) {
    for key in ["dependencies", "dev-dependencies", "build-dependencies"] {
        if let Some(table) = manifest.get(key).and_then(Value::as_table) {
            output.extend(table.iter().map(|(name, value)| (name.as_str(), value)));
        }
    }
    if let Some(targets) = manifest.get("target").and_then(Value::as_table) {
        for target in targets.values() {
            dependencies(target, output);
        }
    }
}

// Resolve existing ancestors before handling .., including when the leaf is absent.
// This is ownership classification, not an assertion that Cargo can build the dependency.
fn destination(base: &Path, value: &str) -> Result<PathBuf, String> {
    if value.contains(['\\', ':', '\0']) {
        return Err(format!("nonportable dependency path {value:?}"));
    }
    let mut result = base.to_path_buf();
    for component in Path::new(value).components() {
        match component {
            Component::Prefix(_) | Component::RootDir => {
                return Err(format!("dependency path must be relative: {value:?}"));
            }
            Component::CurDir => {}
            Component::ParentDir => {
                result.pop();
            }
            Component::Normal(part) => {
                result.push(part);
                match std::fs::symlink_metadata(&result) {
                    Ok(_) => result = result.canonicalize().map_err(|error| error.to_string())?,
                    Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
                    Err(error) => return Err(error.to_string()),
                }
            }
        }
    }
    Ok(result)
}

/// Return actionable failures without modifying files or invoking Cargo.
pub fn check(root: &Path) -> Vec<String> {
    let root = match root.canonicalize() {
        Ok(root) => root,
        Err(error) => return vec![format!("{}: {error}", root.display())],
    };
    let manifest_path = root.join("Cargo.toml");
    let workspace = match manifest(&manifest_path) {
        Ok(manifest) => manifest,
        Err(error) => return vec![error],
    };
    let mut failures = Vec::new();
    if !workspace.get("workspace").is_some_and(Value::is_table)
        || workspace.get("package").is_some()
    {
        failures.push("root must be a virtual workspace, not a legacy game package".into());
    }
    for old in [
        "crates", "tools", "plugins", "src", "tests", "assets", "wasm",
    ] {
        if std::fs::symlink_metadata(root.join(old)).is_ok() {
            failures.push(format!("retired root path remains: {old}"));
        }
    }
    let files = match support::source_files(&root) {
        Ok(files) => files,
        Err(error) => {
            failures.push(error);
            return failures;
        }
    };
    let mut manifests = BTreeMap::new();
    for path in &files {
        if path.file_name().is_some_and(|name| name == "Cargo.toml") {
            match manifest(path) {
                Ok(manifest) => {
                    manifests.insert(path, manifest);
                }
                Err(error) => failures.push(error),
            }
        }
    }
    let games: BTreeSet<_> = manifests
        .iter()
        .filter_map(|(path, manifest)| {
            path.starts_with(root.join("games"))
                .then(|| package_name(manifest))
                .flatten()
        })
        .collect();
    for path in files.iter() {
        let relative = path.strip_prefix(&root).unwrap_or(path).display();
        if path.file_name().is_some_and(|name| name == "Cargo.lock")
            && *path != root.join("Cargo.lock")
        {
            failures.push(format!("duplicate dependency lockfile: {relative}"));
        }
        // Frozen compatibility bytes carry historical relative links. Their own
        // validator checks skill references and the exact provenance hashes.
        if path.extension().is_some_and(|extension| extension == "md")
            && !path.starts_with(root.join("devtools/tests/fixtures/legacy"))
        {
            match support::read_text(path) {
                Ok(source) => {
                    for target in markdown::links(&source) {
                        if let Err(error) = support::local_target(&root, path, &target) {
                            failures.push(format!("{relative}: {error}"));
                        }
                    }
                }
                Err(error) => failures.push(error),
            }
        }
        let Some(package) = manifests.get(path) else {
            continue;
        };
        if *path != manifest_path && package.get("workspace").is_some() {
            failures.push(format!("nested workspace: {relative}"));
        }
        if !path.starts_with(root.join("gamekit")) {
            continue;
        }
        let mut entries = Vec::new();
        dependencies(package, &mut entries);
        for (name, dependency) in entries {
            let inherited = dependency.get("workspace").and_then(Value::as_bool) == Some(true);
            let (dependency, base) = if inherited {
                let Some(dependency) = workspace
                    .get("workspace")
                    .and_then(|table| table.get("dependencies"))
                    .and_then(|table| table.get(name))
                else {
                    failures.push(format!("{relative}: missing workspace dependency {name}"));
                    continue;
                };
                (dependency, root.as_path())
            } else {
                (dependency, path.parent().unwrap_or(&root))
            };
            let target = dependency
                .get("package")
                .and_then(Value::as_str)
                .unwrap_or(name);
            let mut is_game = games.contains(target);
            if let Some(value) = dependency.get("path") {
                match value
                    .as_str()
                    .ok_or_else(|| "dependency path must be a string".to_owned())
                    .and_then(|value| destination(base, value))
                {
                    Ok(destination) => is_game |= destination.starts_with(root.join("games")),
                    Err(error) => failures.push(format!("{relative}: {name}: {error}")),
                }
            }
            if is_game {
                failures.push(format!("{relative}: capability depends on game {target}"));
            }
            if package_name(package) != Some("bevy-gamekit") && target == "bevy-gamekit" {
                failures.push(format!(
                    "{relative}: capability depends on facade; dependency direction is reversed"
                ));
            }
        }
    }
    failures.sort();
    failures
}
