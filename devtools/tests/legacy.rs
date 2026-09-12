//! Canonical skill regression cases and malformed trigger fixtures use package-local data.

use repo_devtools::legacy::{self, SKILLS};
use serde_json::{json, Value};
use std::error::Error;
use std::path::{Path, PathBuf};

type TestResult = Result<(), Box<dyn Error>>;
const DESCRIPTION: &str =
    "Use for a scoped Bevy implementation and its evidence. Do not use for unrelated work.";
const NAME: &str = "architect-bevy-game";

struct Canonical(tempfile::TempDir);

impl Canonical {
    fn new() -> Result<Self, Box<dyn Error>> {
        let fixture = Self(tempfile::tempdir()?);
        for name in SKILLS {
            fixture.write(&format!("source/{name}/SKILL.md"), &format!("---\nname: {name}\ndescription: {DESCRIPTION}\n---\n\n# Scoped guidance\n\nRead the [shared reference](../../references/shared.md) and verify the requested behavior.\n"))?;
            fixture.write(
                &format!("source/{name}/agents/openai.yaml"),
                &format!("interface:\n  default_prompt: Use ${name} for this task.\n"),
            )?;
        }
        fixture.write(
            "references/shared.md",
            "# Shared reference\n\nA source-equivalent reference for both clients.\n",
        )?;
        fixture.write(
            "tests/trigger-fixtures.json",
            include_str!("fixtures/catalog/trigger-fixtures.json"),
        )?;
        Ok(fixture)
    }
    fn root(&self) -> &Path {
        self.0.path()
    }
    fn path(&self, relative: &str) -> PathBuf {
        self.root().join(relative)
    }
    fn skill(&self) -> PathBuf {
        self.path(&format!("source/{NAME}/SKILL.md"))
    }
    fn write(&self, relative: &str, content: &str) -> TestResult {
        let path = self.path(relative);
        std::fs::create_dir_all(path.parent().ok_or("missing parent")?)?;
        std::fs::write(path, content)?;
        Ok(())
    }
    fn edit(&self, pointer: &str, value: Value) -> TestResult {
        let path = self.path("tests/trigger-fixtures.json");
        let mut data: Value = serde_json::from_str(&std::fs::read_to_string(&path)?)?;
        *data.pointer_mut(pointer).ok_or("missing fixture pointer")? = value;
        std::fs::write(path, serde_json::to_string_pretty(&data)?)?;
        Ok(())
    }
    fn replace(&self, from: &str, to: &str) -> TestResult {
        std::fs::write(
            self.skill(),
            std::fs::read_to_string(self.skill())?.replace(from, to),
        )?;
        Ok(())
    }
    fn assert_failure(&self, message: &str) {
        let failures = legacy::validate(self.root());
        assert!(
            failures.iter().any(|failure| failure.contains(message)),
            "expected {message:?}: {failures:#?}"
        );
    }
}

#[test]
fn seven_canonical_skills_validate_with_two_source_equivalent_clients() -> TestResult {
    let fixture = Canonical::new()?;
    assert_eq!(legacy::validate(fixture.root()), Vec::<String>::new());
    assert_eq!(SKILLS.len(), 7);
    assert_eq!(legacy::CLIENTS, &["codex", "claude"]);
    assert!(legacy::NOTICE.contains("requires agent evaluation"));
    fixture.replace("\n", "\r\n")?;
    assert_eq!(legacy::validate(fixture.root()), Vec::<String>::new());
    Ok(())
}

#[test]
fn repository_root_and_direct_skills_root_are_supported() -> TestResult {
    let fixture = Canonical::new()?;
    let repository = tempfile::tempdir()?;
    std::fs::create_dir(repository.path().join("gameskills"))?;
    std::fs::rename(fixture.root(), repository.path().join("gameskills/legacy"))?;
    assert_eq!(legacy::validate(repository.path()), Vec::<String>::new());
    assert_eq!(
        legacy::validate(&repository.path().join("gameskills/legacy")),
        Vec::<String>::new()
    );
    Ok(())
}

#[test]
fn required_frontmatter_identity_triggers_and_placeholders_are_checked() -> TestResult {
    for (from, to, message) in [
        ("---\nname:", "name:", "invalid frontmatter"),
        (
            "name: architect-bevy-game",
            "name: wrong-name",
            "frontmatter name mismatch",
        ),
        (
            DESCRIPTION,
            "A description without the canonical routing boundary.",
            "description lacks positive or negative triggers",
        ),
        (
            "# Scoped guidance",
            "# TODO guidance",
            "contains TODO placeholder",
        ),
    ] {
        let fixture = Canonical::new()?;
        fixture.replace(from, to)?;
        fixture.assert_failure(message);
    }
    Ok(())
}

#[test]
fn missing_canonical_skill_metadata_and_reference_are_failures() -> TestResult {
    for (relative, message) in [
        (
            "source/architect-bevy-game/SKILL.md",
            "missing canonical skill",
        ),
        (
            "source/architect-bevy-game/agents/openai.yaml",
            "openai.yaml",
        ),
        ("references/shared.md", "referenced file"),
    ] {
        let fixture = Canonical::new()?;
        std::fs::remove_file(fixture.path(relative))?;
        fixture.assert_failure(message);
    }
    let fixture = Canonical::new()?;
    fixture.write(
        "source/architect-bevy-game/agents/openai.yaml",
        "default_prompt: Use $review-bevy-change instead.",
    )?;
    fixture.assert_failure("default prompt does not name the skill");
    Ok(())
}

#[test]
fn malformed_empty_and_contradictory_trigger_examples_do_not_panic() -> TestResult {
    for value in [
        json!(null),
        json!({}),
        json!("text"),
        json!([]),
        json!([null]),
        json!([{}]),
        json!([" "]),
    ] {
        let fixture = Canonical::new()?;
        fixture.edit("/architect-bevy-game/should_trigger", value)?;
        let failures = legacy::validate(fixture.root());
        assert!(!failures.is_empty());
    }
    let fixture = Canonical::new()?;
    fixture.edit("/architect-bevy-game", json!([]))?;
    fixture.assert_failure("missing positive or negative trigger fixtures");
    let fixture = Canonical::new()?;
    fixture.edit(
        "/architect-bevy-game/should_trigger",
        json!(["same prompt"]),
    )?;
    fixture.edit(
        "/architect-bevy-game/should_not_trigger",
        json!(["same prompt"]),
    )?;
    fixture.assert_failure("contradictory trigger fixtures");
    Ok(())
}

#[test]
fn malformed_and_recursive_duplicate_fixture_json_are_validation_failures() -> TestResult {
    for content in [
        "[]",
        "{",
        r#"{"architect-bevy-game":{"should_trigger":["a"],"should_trigger":["b"]}}"#,
    ] {
        let fixture = Canonical::new()?;
        fixture.write("tests/trigger-fixtures.json", content)?;
        assert!(!legacy::validate(fixture.root()).is_empty());
    }
    Ok(())
}

#[test]
fn local_reference_definitions_are_checked_and_fenced_examples_ignored() -> TestResult {
    let fixture = Canonical::new()?;
    let text = std::fs::read_to_string(fixture.skill())?;
    std::fs::write(
        fixture.skill(),
        format!(
            "{text}\n~~~markdown\n[Example](missing-example.md)\n~~~\n\n[Guide][guide]\n[guide]: ../../references/missing.md\n"
        ),
    )?;
    let failures = legacy::validate(fixture.root());
    assert!(
        failures
            .iter()
            .any(|failure| failure.contains("references/missing.md")),
        "{failures:?}"
    );
    assert!(
        !failures
            .iter()
            .any(|failure| failure.contains("missing-example.md")),
        "{failures:?}"
    );
    Ok(())
}

#[test]
fn encoded_spaces_fragments_and_external_links_are_valid() -> TestResult {
    let fixture = Canonical::new()?;
    fixture.write("references/space name.md", "# Heading\n")?;
    fixture.replace(
        "../../references/shared.md",
        "../../references/space%20name.md#heading",
    )?;
    let text = std::fs::read_to_string(fixture.skill())?;
    std::fs::write(
        fixture.skill(),
        format!(
            "{text}\n[Website](https://example.com/docs) [Mail](mailto:help@example.com) [Local](#heading)\n"
        ),
    )?;
    assert_eq!(legacy::validate(fixture.root()), Vec::<String>::new());
    Ok(())
}

#[test]
fn references_must_be_files_contained_in_the_canonical_package() -> TestResult {
    let fixture = Canonical::new()?;
    fixture.replace("../../references/shared.md", "../../references/")?;
    fixture.assert_failure("missing referenced file");
    let fixture = Canonical::new()?;
    let outside = tempfile::NamedTempFile::new()?;
    fixture.replace(
        "../../references/shared.md",
        &outside.path().to_string_lossy(),
    )?;
    fixture.assert_failure("nonportable local path");
    Ok(())
}

#[cfg(unix)]
#[test]
fn canonical_symlink_files_and_directories_cannot_escape() -> TestResult {
    let fixture = Canonical::new()?;
    let outside = tempfile::tempdir()?;
    std::fs::write(outside.path().join("outside.md"), "outside")?;
    std::os::unix::fs::symlink(outside.path(), fixture.path("references/escape"))?;
    fixture.replace(
        "../../references/shared.md",
        "../../references/escape/outside.md",
    )?;
    fixture.assert_failure("symlink escapes native package");
    fixture.assert_failure("link escapes owning boundary");
    std::os::unix::fs::symlink("loop.md", fixture.path("references/loop.md"))?;
    fixture.assert_failure("unresolvable package symlink");
    Ok(())
}

#[test]
fn invalid_utf8_and_missing_root_return_deterministic_failures() -> TestResult {
    let fixture = Canonical::new()?;
    std::fs::write(fixture.skill(), [255, 254])?;
    let failures = legacy::validate(fixture.root());
    assert!(!failures.is_empty());
    assert_eq!(failures, legacy::validate(fixture.root()));
    assert!(!legacy::validate(&fixture.path("missing-root")).is_empty());
    Ok(())
}
