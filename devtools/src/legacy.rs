//! Structural validation and source-equivalent client rendering for canonical Bevy skills.

use crate::{catalog, markdown, support};
use regex::Regex;
use serde_json::Value;
use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};

/// The seven canonical legacy skills; these are not new native package offerings.
pub const SKILLS: &[&str] = &[
    "architect-bevy-game",
    "model-turn-based-game",
    "build-bevy-ui",
    "test-bevy-game",
    "verify-bevy-ui",
    "debug-bevy-runtime",
    "review-bevy-change",
];

/// Supported canonical rendering targets.
pub const CLIENTS: &[&str] = &["codex", "claude"];

/// Passing canonical structure and rendering checks does not evaluate trigger behavior.
pub const NOTICE: &str = "Trigger behavior requires agent evaluation; source equivalence does not prove native client behavior.";

fn examples<'a>(
    fixture: &'a Value,
    name: &str,
    category: &str,
    failures: &mut Vec<String>,
) -> BTreeSet<&'a str> {
    let Some(values) = fixture.get(category).and_then(Value::as_array) else {
        failures.push(format!("{name}: invalid {category} examples"));
        return BTreeSet::new();
    };
    let mut result = BTreeSet::new();
    for value in values {
        if let Some(text) = value.as_str().filter(|text| !text.trim().is_empty()) {
            result.insert(text);
        } else {
            failures.push(format!("{name}: invalid {category} examples"));
        }
    }
    result
}

fn render(
    root: &Path,
    files: &[PathBuf],
    failures: &mut Vec<String>,
) -> BTreeMap<PathBuf, Vec<u8>> {
    let source = root.join("source");
    let references = root.join("references");
    let mut rendered = BTreeMap::new();
    for file in files {
        let reference = file.parent() == Some(references.as_path())
            && file.extension().is_some_and(|extension| extension == "md");
        let skill = SKILLS.iter().find_map(|name| {
            file.strip_prefix(source.join(name))
                .ok()
                .map(|relative| (*name, relative))
        });
        if !reference && skill.is_none() {
            continue;
        }
        let bytes = match std::fs::read(file) {
            Ok(bytes) => bytes,
            Err(error) => {
                failures.push(format!("{}: {error}", file.display()));
                continue;
            }
        };
        for (client, client_root) in [("codex", ".agents"), ("claude", ".claude")] {
            if reference {
                if let Some(name) = file.file_name() {
                    rendered.insert(
                        Path::new(client_root)
                            .join("bevy-gamekit/references")
                            .join(name),
                        bytes.clone(),
                    );
                }
            } else if let Some((name, relative)) = skill {
                if client == "claude" && relative.starts_with("agents") {
                    continue;
                }
                let data = if relative == Path::new("SKILL.md") {
                    match std::str::from_utf8(&bytes) {
                        Ok(text) => text
                            .replace("../../references/", "../../bevy-gamekit/references/")
                            .into_bytes(),
                        Err(error) => {
                            failures.push(format!("{}: {error}", file.display()));
                            continue;
                        }
                    }
                } else {
                    bytes.clone()
                };
                rendered.insert(
                    Path::new(client_root)
                        .join("skills")
                        .join(name)
                        .join(relative),
                    data,
                );
            }
        }
    }
    rendered
}

/// Validate a repository's `skills` directory, or a directly supplied canonical skills directory.
/// All seven skills, trigger fixture shapes, local links, and client source equivalence are checked.
/// Rendering is in memory and never invokes the legacy Python installer or either native client.
pub fn validate(root: &Path) -> Vec<String> {
    let root = if root.join("gameskills/legacy").is_dir() {
        root.join("gameskills/legacy")
    } else {
        root.to_owned()
    };
    let mut failures = Vec::new();
    let files = catalog::package_files(&root, &mut failures);
    if root.is_symlink() {
        return failures;
    }
    let frontmatter = match Regex::new(r"\A---\nname: ([^\n]+)\ndescription: ([^\n]+)\n---\n") {
        Ok(regex) => regex,
        Err(error) => {
            failures.push(error.to_string());
            return failures;
        }
    };
    for name in SKILLS {
        let skill_root = root.join("source").join(name);
        let path = skill_root.join("SKILL.md");
        let content = match support::read_text(&path) {
            Ok(content) => content.replace("\r\n", "\n").replace('\r', "\n"),
            Err(error) => {
                failures.push(format!("{name}: missing canonical skill: {error}"));
                continue;
            }
        };
        let Some(fields) = frontmatter.captures(&content) else {
            failures.push(format!("{name}: invalid frontmatter"));
            continue;
        };
        if fields.get(1).map(|field| field.as_str()) != Some(name) {
            failures.push(format!("{name}: frontmatter name mismatch"));
        }
        let description = fields.get(2).map_or("", |field| field.as_str());
        if !description.contains("Use ") || !description.contains("Do not use") {
            failures.push(format!(
                "{name}: description lacks positive or negative triggers"
            ));
        }
        if content.contains("TODO") {
            failures.push(format!("{name}: contains TODO placeholder"));
        }
        for link in markdown::links(&content) {
            match support::local_target(&root, &path, &link) {
                Ok(Some(target)) if !target.is_file() => {
                    failures.push(format!("{name}: missing referenced file {link}"))
                }
                Ok(_) => {}
                Err(error) => failures.push(format!(
                    "{name}: missing or invalid referenced file {link}: {error}"
                )),
            }
        }
        match support::read_text(&skill_root.join("agents/openai.yaml")) {
            Ok(metadata) if !metadata.contains(&format!("${name}")) => {
                failures.push(format!("{name}: default prompt does not name the skill"))
            }
            Ok(_) => {}
            Err(error) => failures.push(error),
        }
    }
    let fixtures = catalog::json(&root.join("tests/trigger-fixtures.json"), &mut failures);
    for name in SKILLS {
        let fixture = fixtures.get(*name).unwrap_or(&Value::Null);
        if ["should_trigger", "should_not_trigger"]
            .iter()
            .any(|category| {
                fixture
                    .get(category)
                    .and_then(Value::as_array)
                    .is_none_or(Vec::is_empty)
            })
        {
            failures.push(format!(
                "{name}: missing positive or negative trigger fixtures"
            ));
        }
        let positive = examples(fixture, name, "should_trigger", &mut failures);
        let negative = examples(fixture, name, "should_not_trigger", &mut failures);
        if !positive.is_disjoint(&negative) {
            failures.push(format!("{name}: contradictory trigger fixtures"));
        }
    }
    let rendered = render(&root, &files, &mut failures);
    for name in SKILLS {
        let codex = rendered.get(&Path::new(".agents/skills").join(name).join("SKILL.md"));
        let claude = rendered.get(&Path::new(".claude/skills").join(name).join("SKILL.md"));
        if codex.is_none() || claude.is_none() {
            failures.push(format!("{name}: missing rendered canonical skill"));
        } else if codex != claude {
            failures.push(format!("{name}: Codex and Claude SKILL.md differ"));
        }
    }
    failures
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn client_rendering_preserves_bytes_and_omits_only_claude_agents(
    ) -> Result<(), Box<dyn std::error::Error>> {
        let scratch = tempfile::tempdir()?;
        let root = scratch.path();
        let name = "architect-bevy-game";
        let skill = root.join("source").join(name);
        std::fs::create_dir_all(skill.join("agents"))?;
        std::fs::create_dir_all(root.join("references"))?;
        std::fs::write(
            skill.join("SKILL.md"),
            "[Reference](../../references/shared.md)\n",
        )?;
        std::fs::write(skill.join("agents/openai.yaml"), "agent bytes\n")?;
        std::fs::write(skill.join("asset.bin"), [0, 255, 13, 10])?;
        std::fs::write(root.join("references/shared.md"), "reference bytes\n")?;
        let mut failures = Vec::new();
        let files = catalog::package_files(root, &mut failures);
        let output = render(root, &files, &mut failures);
        assert!(failures.is_empty(), "{failures:?}");
        for client in [".agents", ".claude"] {
            assert_eq!(
                output
                    .get(&Path::new(client).join("skills").join(name).join("SKILL.md"))
                    .map(Vec::as_slice),
                Some(b"[Reference](../../bevy-gamekit/references/shared.md)\n".as_slice())
            );
            assert_eq!(
                output
                    .get(
                        &Path::new(client)
                            .join("skills")
                            .join(name)
                            .join("asset.bin")
                    )
                    .map(Vec::as_slice),
                Some([0, 255, 13, 10].as_slice())
            );
            assert_eq!(
                output
                    .get(&Path::new(client).join("bevy-gamekit/references/shared.md"))
                    .map(Vec::as_slice),
                Some(b"reference bytes\n".as_slice())
            );
        }
        assert!(output.contains_key(
            &Path::new(".agents/skills")
                .join(name)
                .join("agents/openai.yaml")
        ));
        assert!(!output.contains_key(
            &Path::new(".claude/skills")
                .join(name)
                .join("agents/openai.yaml")
        ));
        Ok(())
    }
}
