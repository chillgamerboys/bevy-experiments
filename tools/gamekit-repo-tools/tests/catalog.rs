//! Candidate mutations preserve structural contracts without claiming agent evaluation.

use gamekit_repo_tools::catalog::{self, EXPECTED_SKILLS};
use serde_json::{Value, json};
use std::error::Error;
use std::path::{Path, PathBuf};

type TestResult = Result<(), Box<dyn Error>>;
const CATALOG: &str = "plugins/gameskills/catalog.json";
const SCENARIOS: &str = "skills/tests/gameskills-scenarios.json";
const DESCRIPTION: &str =
    "Inspect the owning implementation and verify the requested game behavior.";
const BODY: &str = "# Scoped behavior\n\nRead the owning source, preserve the requested scope, and report actual observations with their limitations.\n";

struct Candidate(tempfile::TempDir);

impl Candidate {
    fn new() -> Result<Self, Box<dyn Error>> {
        let fixture = Self(tempfile::tempdir()?);
        let version = "0.1.0-test.1";
        let mut packages = serde_json::Map::new();
        let mut codex = Vec::new();
        let mut claude = Vec::new();
        for &(package, skills) in EXPECTED_SKILLS {
            let description = "Scoped guidance for building and reviewing game changes.";
            packages.insert(package.into(), json!({"skills": skills, "requires": if package == "gameskills" {vec![]} else {vec!["gameskills"]}, "description": description}));
            for skill in skills {
                fixture.write(
                    &format!("plugins/{package}/skills/{skill}/SKILL.md"),
                    &format!("---\nname: {skill}\ndescription: {DESCRIPTION}\n---\n\n{BODY}"),
                )?;
            }
            fixture.write(
                &format!("plugins/{package}/references/local.md"),
                "# Local reference\n\nA maintained package-local reference.\n",
            )?;
            fixture.write_json(&format!("plugins/{package}/.claude-plugin/plugin.json"), &json!({"name": package, "version": version, "description": description, "author": {"name": "Fixture"}}))?;
            fixture.write_json(&format!("plugins/{package}/.codex-plugin/plugin.json"), &json!({"name": package, "version": version, "description": description, "author": {"name": "Fixture"}, "skills": "./skills/", "interface": {"displayName": "Fixture", "capabilities": []}}))?;
            codex.push(json!({"name": package, "source": {"source": "local", "path": format!("./plugins/{package}")}}));
            claude.push(json!({"name": package, "source": format!("./plugins/{package}"), "version": version}));
        }
        fixture.write_json(
            CATALOG,
            &json!({"schema_version": 1, "version": version, "packages": packages}),
        )?;
        fixture.write_json(
            ".agents/plugins/marketplace.json",
            &json!({"name": "gameskills", "plugins": codex}),
        )?;
        fixture.write_json(
            ".claude-plugin/marketplace.json",
            &json!({"name": "gameskills", "plugins": claude}),
        )?;
        fixture.write(
            SCENARIOS,
            include_str!("fixtures/catalog/gameskills-scenarios.json"),
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
        self.path("plugins/gameskills/skills/plan/SKILL.md")
    }
    fn write(&self, relative: &str, text: &str) -> TestResult {
        let path = self.path(relative);
        std::fs::create_dir_all(path.parent().ok_or("missing fixture parent")?)?;
        std::fs::write(path, text)?;
        Ok(())
    }
    fn write_json(&self, relative: &str, value: &Value) -> TestResult {
        self.write(relative, &serde_json::to_string_pretty(value)?)
    }
    fn edit(&self, relative: &str, pointer: &str, value: Value) -> TestResult {
        let mut data: Value = serde_json::from_str(&std::fs::read_to_string(self.path(relative))?)?;
        *data.pointer_mut(pointer).ok_or("missing fixture pointer")? = value;
        self.write_json(relative, &data)
    }
    fn append(&self, path: &Path, text: &str) -> TestResult {
        let mut content = std::fs::read_to_string(path)?;
        content.push_str(text);
        std::fs::write(path, content)?;
        Ok(())
    }
    fn failures(&self) -> Vec<String> {
        catalog::validate(self.root())
    }
    fn assert_failure(&self, text: &str) {
        let failures = self.failures();
        assert!(
            failures.iter().any(|failure| failure.contains(text)),
            "expected {text:?}: {failures:#?}"
        );
    }
    fn assert_valid(&self) {
        assert_eq!(self.failures(), Vec::<String>::new());
    }
}

#[test]
fn coherent_candidate_needs_no_literal_negative_phrase_or_ui_metadata() -> TestResult {
    let fixture = Candidate::new()?;
    fixture.assert_valid();
    assert!(
        !fixture
            .skill()
            .parent()
            .ok_or("skill parent missing")?
            .join("agents/openai.yaml")
            .exists()
    );
    Ok(())
}

#[test]
fn missing_or_duplicate_catalog_skill_is_not_hidden_by_a_valid_manifest() -> TestResult {
    for skills in [
        json!(["plan"]),
        json!([
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
            "plan"
        ]),
    ] {
        let fixture = Candidate::new()?;
        fixture.edit(CATALOG, "/packages/gameskills/skills", skills)?;
        fixture.assert_failure("catalog gameskills skills: inventory mismatch");
    }
    Ok(())
}

#[test]
fn extra_native_skill_and_missing_native_body_are_detected() -> TestResult {
    let fixture = Candidate::new()?;
    fixture.write(
        "plugins/gameskills/skills/inject/SKILL.md",
        &std::fs::read_to_string(fixture.skill())?.replace("name: plan", "name: inject"),
    )?;
    std::fs::remove_file(fixture.path("plugins/gameskills/skills/debug/SKILL.md"))?;
    fixture.assert_failure("gameskills native skill directories: inventory mismatch");
    fixture.assert_failure("discovered SKILL.md inventory mismatch");
    Ok(())
}

#[test]
fn optional_package_cannot_discover_sibling_core_skills() -> TestResult {
    let fixture = Candidate::new()?;
    fixture.edit(
        "plugins/gameskills-ui/.codex-plugin/plugin.json",
        "/skills",
        json!("../gameskills/skills/"),
    )?;
    fixture.assert_failure("link escapes owning boundary");
    fixture.assert_failure("skills must resolve to this package");
    Ok(())
}

#[test]
fn manifest_identity_and_versions_match_catalog() -> TestResult {
    let fixture = Candidate::new()?;
    let manifest = "plugins/gameskills-ui/.claude-plugin/plugin.json";
    fixture.edit(manifest, "/name", json!("gameskills"))?;
    fixture.edit(manifest, "/version", json!("8.0.0"))?;
    fixture.assert_failure("package name mismatch");
    fixture.assert_failure("version does not match catalog");
    Ok(())
}

#[test]
fn native_metadata_types_are_checked_without_assuming_client_execution() -> TestResult {
    let fixture = Candidate::new()?;
    fixture.edit(
        "plugins/gameskills/.codex-plugin/plugin.json",
        "/interface",
        json!({"capabilities": "skills", "displayName": 7}),
    )?;
    fixture.assert_failure("interface.capabilities must be a string list");
    fixture.assert_failure("interface.displayName must be a nonempty string");
    Ok(())
}

#[test]
fn marketplace_inventory_and_source_mapping_are_both_checked() -> TestResult {
    let fixture = Candidate::new()?;
    let mut data: Value = serde_json::from_str(&std::fs::read_to_string(
        fixture.path(".agents/plugins/marketplace.json"),
    )?)?;
    let entries = data
        .get_mut("plugins")
        .and_then(Value::as_array_mut)
        .ok_or("missing entries")?;
    entries.push(entries.first().ok_or("missing first entry")?.clone());
    fixture.write_json(".agents/plugins/marketplace.json", &data)?;
    fixture.edit(
        ".claude-plugin/marketplace.json",
        "/plugins/1/source",
        json!("./plugins/gameskills"),
    )?;
    fixture.assert_failure("marketplace.json packages: inventory mismatch");
    fixture.assert_failure("gameskills-ui source does not resolve to its package");
    Ok(())
}

#[test]
fn existing_encoded_link_outside_package_is_rejected() -> TestResult {
    let fixture = Candidate::new()?;
    fixture.append(
        &fixture.path("plugins/gameskills-ui/skills/build-ui/SKILL.md"),
        "\n[Core](%2e%2e/%2e%2e/%2e%2e/gameskills/references/local.md)\n",
    )?;
    fixture.assert_failure("link escapes owning boundary");
    Ok(())
}

#[test]
fn reference_links_are_checked_and_fenced_examples_are_not_followed() -> TestResult {
    let fixture = Candidate::new()?;
    fixture.append(&fixture.skill(), "\n```markdown\n[Example](missing-example.md)\n```\n\n[Docs][detail]\n\n[detail]: ../../references/missing.md\n")?;
    let failures = fixture.failures();
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
fn valid_space_and_fragment_local_links_are_portable() -> TestResult {
    let fixture = Candidate::new()?;
    fixture.write("plugins/gameskills/references/space name.md", "# Heading\n")?;
    fixture.append(&fixture.skill(), "\n[By angle](<../../references/space name.md#heading>)\n[By URL](../../references/space%20name.md)\n")?;
    fixture.assert_valid();
    Ok(())
}

#[cfg(unix)]
#[test]
fn symlink_cannot_make_an_external_reference_look_package_local() -> TestResult {
    let fixture = Candidate::new()?;
    fixture.write("outside.md", "Outside package content.\n")?;
    std::os::unix::fs::symlink(
        fixture.path("outside.md"),
        fixture.path("plugins/gameskills-ui/references/escape.md"),
    )?;
    fixture.append(
        &fixture.path("plugins/gameskills-ui/skills/verify-ui/SKILL.md"),
        "\n[Reference](../../references/escape.md)\n",
    )?;
    fixture.assert_failure("symlink escapes native package");
    fixture.assert_failure("link escapes owning boundary");
    Ok(())
}

#[cfg(unix)]
#[test]
fn symlink_loop_is_a_validation_error_without_a_traceback() -> TestResult {
    let fixture = Candidate::new()?;
    std::os::unix::fs::symlink(
        "loop.md",
        fixture.path("plugins/gameskills/references/loop.md"),
    )?;
    fixture.assert_failure("unresolvable package symlink");
    Ok(())
}

#[test]
fn malformed_structural_types_report_failures_instead_of_crashing() -> TestResult {
    let fixture = Candidate::new()?;
    fixture.edit(
        ".agents/plugins/marketplace.json",
        "/plugins/0/name",
        json!([]),
    )?;
    fixture.edit(SCENARIOS, "/scenarios/0/routing/primary", json!([]))?;
    fixture.edit(CATALOG, "/schema_version", json!(true))?;
    fixture.assert_failure("expected a string list");
    fixture.assert_failure("unknown primary skill");
    fixture.assert_failure("schema_version must be 1");
    Ok(())
}

#[test]
fn duplicate_json_keys_are_an_error_instead_of_last_value_wins() -> TestResult {
    for text in [
        r#"{"name":"wrong", "name":"gameskills"}"#,
        r#"{"interface":{"name":"wrong", "name":"gameskills"}}"#,
        r#"{"items":[{"name":1,"name":2}]}"#,
    ] {
        let fixture = Candidate::new()?;
        fixture.write("plugins/gameskills/.codex-plugin/plugin.json", text)?;
        fixture.assert_failure("duplicate JSON key");
    }
    Ok(())
}

#[test]
fn required_frontmatter_supports_folded_and_quoted_strings() -> TestResult {
    let fixture = Candidate::new()?;
    for description in [
        ">-\n  Investigate the scoped game request\n  and carry it through authorized delivery.",
        "|-\n  Investigate the scoped game request\n  and carry it through authorized delivery.",
        "'Investigate the scoped game request and the user''s authorized delivery.'",
        "\"Investigate the scoped game request: carry it through authorized delivery.\"",
    ] {
        std::fs::write(
            fixture.skill(),
            format!(
                "---\nname: \"plan\"\ndescription: {description}\nmetadata:\n  short-description: Local guidance\n---\n\n{BODY}"
            ),
        )?;
        fixture.assert_valid();
    }
    Ok(())
}

#[test]
fn mismatched_empty_and_duplicate_frontmatter_fail() -> TestResult {
    let fixture = Candidate::new()?;
    for (name, error) in [
        ("review", "frontmatter name must match"),
        ("plan\nname: plan", "duplicate frontmatter key name"),
        ("", "required field must be a string"),
    ] {
        std::fs::write(
            fixture.skill(),
            format!("---\nname: {name}\ndescription: {DESCRIPTION}\n---\n\n{BODY}"),
        )?;
        fixture.assert_failure(error);
    }
    Ok(())
}

#[test]
fn underspecified_description_fails_without_enforcing_trigger_words() -> TestResult {
    let fixture = Candidate::new()?;
    let content = std::fs::read_to_string(fixture.skill())?;
    std::fs::write(fixture.skill(), content.replace(DESCRIPTION, "Do stuff"))?;
    fixture.assert_failure("description must be a descriptive string");
    Ok(())
}

#[test]
fn optional_ui_metadata_cannot_point_at_another_skill() -> TestResult {
    let fixture = Candidate::new()?;
    fixture.write("plugins/gameskills/skills/plan/agents/openai.yaml", "interface:\n  display_name: \"Plan\"\n  short_description: \"Plan a scoped game task\"\n  default_prompt: \"Use $release for this task.\"\n")?;
    fixture.assert_failure("default_prompt does not reference this skill");
    Ok(())
}

#[test]
fn scenario_rubrics_require_observable_criteria_and_consistent_routing() -> TestResult {
    let fixture = Candidate::new()?;
    fixture.edit(
        SCENARIOS,
        "/scenarios/0/routing/avoid",
        json!(["gameskills:plan"]),
    )?;
    fixture.edit(SCENARIOS, "/scenarios/1/rubric/evidence", json!([]))?;
    fixture.assert_failure("contradictory routing rubric");
    fixture.assert_failure("missing nonempty rubric evidence");
    Ok(())
}

#[test]
fn fixtures_cannot_claim_execution_or_lose_required_boundary_cases() -> TestResult {
    let fixture = Candidate::new()?;
    let mut data: Value = serde_json::from_str(&std::fs::read_to_string(fixture.path(SCENARIOS))?)?;
    data.get_mut("scenarios")
        .and_then(Value::as_array_mut)
        .ok_or("missing scenarios")?
        .retain(|scenario| {
            scenario
                .get("tags")
                .and_then(Value::as_array)
                .is_none_or(|tags| !tags.contains(&json!("human-upstream")))
        });
    fixture.write_json(SCENARIOS, &data)?;
    fixture.edit(SCENARIOS, "/evidence_status", json!("passed"))?;
    fixture.assert_failure("unexecuted-rubrics evidence_status");
    fixture.assert_failure("missing scenario coverage tags");
    Ok(())
}

#[test]
fn structural_scope_and_failure_status_are_available_to_the_cli() -> TestResult {
    let fixture = Candidate::new()?;
    fixture.assert_valid();
    assert!(catalog::NOTICE.contains("do not prove native installation"));
    assert_eq!(EXPECTED_SKILLS.len(), 6);
    assert_eq!(
        EXPECTED_SKILLS
            .iter()
            .map(|(_, skills)| skills.len())
            .sum::<usize>(),
        21
    );
    std::fs::remove_file(fixture.skill())?;
    assert!(!fixture.failures().is_empty());
    Ok(())
}

#[test]
fn invalid_json_objects_versions_and_requirements_fail() -> TestResult {
    for (pointer, value, message) in [
        ("/schema_version", json!(1.0), "schema_version must be 1"),
        (
            "/version",
            json!(1),
            "version must have semantic-version form",
        ),
        (
            "/version",
            json!("1.2"),
            "version must have semantic-version form",
        ),
        (
            "/packages/gameskills-ui/requires",
            json!([]),
            "catalog gameskills-ui requirements: inventory mismatch",
        ),
        (
            "/packages/gameskills/description",
            json!("Tiny"),
            "missing descriptive package description",
        ),
        (
            "/packages/gameskills",
            json!([]),
            "missing package definition gameskills",
        ),
    ] {
        let fixture = Candidate::new()?;
        fixture.edit(CATALOG, pointer, value)?;
        fixture.assert_failure(message);
    }
    for content in ["[]", "null", "{", "{\"schema_version\":1} trailing"] {
        let fixture = Candidate::new()?;
        fixture.write(CATALOG, content)?;
        assert!(!fixture.failures().is_empty());
    }
    Ok(())
}

#[test]
fn malformed_frontmatter_and_unfinished_bodies_are_rejected() -> TestResult {
    for (frontmatter, message) in [
        ("name: plan", "missing frontmatter opening delimiter"),
        ("---\nname: plan", "missing frontmatter closing delimiter"),
        (
            "---\nname: plan\n---",
            "missing required frontmatter description",
        ),
        (
            "---\nname: plan\ndescription: []\n---",
            "required field must be a string",
        ),
        (
            "---\nname: plan\ndescription: false\n---",
            "required field must be a string",
        ),
        (
            "---\nname: plan\ndescription: 'unfinished\n---",
            "unterminated single-quoted string",
        ),
        (
            "---\nname: plan\ndescription: Contains colon: must quote this required scalar\n---",
            "quote a required string containing colon-space",
        ),
        (
            "---\n\tname: plan\ndescription: words\n---",
            "tab-indented frontmatter is unsupported",
        ),
        (
            "---\n  name: plan\ndescription: words\n---",
            "unexpected indented frontmatter",
        ),
        (
            "---\nname:plan\ndescription: words\n---",
            "invalid top-level frontmatter line",
        ),
    ] {
        let fixture = Candidate::new()?;
        std::fs::write(fixture.skill(), format!("{frontmatter}\n\n{BODY}"))?;
        fixture.assert_failure(message);
    }
    let fixture = Candidate::new()?;
    std::fs::write(
        fixture.skill(),
        format!("---\nname: plan\ndescription: {DESCRIPTION}\n---\nTODO [INSERT text]\n"),
    )?;
    fixture.assert_failure("skill body is empty or unfinished");
    fixture.assert_failure("unfinished scaffold placeholder");
    Ok(())
}

#[test]
fn unknown_malformed_and_contradictory_scenarios_are_rejected() -> TestResult {
    for (pointer, value, message) in [
        ("/schema_version", json!(true), "require schema_version 1"),
        ("/scenarios", json!([]), "missing scenarios"),
        ("/scenarios/0", json!([]), "scenario must be an object"),
        ("/scenarios/0/id", json!([]), "invalid or duplicate id"),
        ("/scenarios/0/prompt", json!(7), "missing prompt"),
        ("/scenarios/0/fixture", json!(" "), "missing fixture"),
        (
            "/scenarios/0/allowed_actions",
            json!(null),
            "missing allowed_actions",
        ),
        (
            "/scenarios/0/split",
            json!("training"),
            "split must be held-out",
        ),
        (
            "/scenarios/0/tags",
            json!([1]),
            "tags must be a string list",
        ),
        ("/scenarios/0/routing", json!([]), "missing routing rubric"),
        (
            "/scenarios/0/routing/primary",
            json!("gameskills:unknown"),
            "unknown primary skill",
        ),
        (
            "/scenarios/0/routing/allowed",
            json!(["gameskills:unknown"]),
            "invalid routing allowed",
        ),
        (
            "/scenarios/0/routing/avoid",
            json!([null]),
            "invalid routing avoid",
        ),
        (
            "/scenarios/0/rubric",
            json!(null),
            "missing observation rubric",
        ),
        (
            "/scenarios/0/rubric/must_observe",
            json!("a claim"),
            "missing nonempty rubric must_observe",
        ),
        (
            "/scenarios/0/rubric/must_not_observe",
            json!([]),
            "missing nonempty rubric must_not_observe",
        ),
    ] {
        let fixture = Candidate::new()?;
        fixture.edit(SCENARIOS, pointer, value)?;
        let failures = catalog::validate_scenarios(&fixture.path(SCENARIOS));
        assert!(
            failures.iter().any(|failure| failure.contains(message)),
            "{pointer}: {failures:?}"
        );
    }
    let fixture = Candidate::new()?;
    fixture.edit(
        SCENARIOS,
        "/scenarios/0/rubric/must_observe",
        json!(["same observation"]),
    )?;
    fixture.edit(
        SCENARIOS,
        "/scenarios/0/rubric/must_not_observe",
        json!(["same observation"]),
    )?;
    fixture.assert_failure("contradictory observation rubric");
    let first: Value = serde_json::from_str(&std::fs::read_to_string(fixture.path(SCENARIOS))?)?;
    fixture.edit(
        SCENARIOS,
        "/scenarios/1/id",
        first
            .pointer("/scenarios/0/id")
            .ok_or("missing id")?
            .clone(),
    )?;
    fixture.assert_failure("invalid or duplicate id");
    Ok(())
}

#[test]
fn unsupported_local_paths_and_nested_discovery_are_rejected() -> TestResult {
    for raw in [
        "file:local.md",
        "//example.com/local.md",
        "/tmp/local.md",
        "..%5Clocal.md",
        "%00local.md",
        "bad%zz.md",
    ] {
        let fixture = Candidate::new()?;
        fixture.append(&fixture.skill(), &format!("\n[Reference]({raw})\n"))?;
        assert!(!fixture.failures().is_empty(), "{raw}");
    }
    let fixture = Candidate::new()?;
    fixture.write("plugins/gameskills/skills/plan/nested/SKILL.md", BODY)?;
    fixture.assert_failure("discovered SKILL.md inventory mismatch");
    fixture.write("plugins/unexpected/readme.md", BODY)?;
    fixture.assert_failure("native package directories: inventory mismatch");
    Ok(())
}

#[test]
fn empty_manifests_and_invalid_optional_fields_are_not_valid_candidates() -> TestResult {
    let fixture = Candidate::new()?;
    fixture.write_json("plugins/gameskills/.codex-plugin/plugin.json", &json!({}))?;
    fixture.assert_failure("package name mismatch");
    for (pointer, value, message) in [
        (
            "/author",
            json!("Fixture"),
            "author must be an object with a name",
        ),
        ("/interface", json!([]), "interface must be an object"),
        (
            "/skills",
            json!([]),
            "this candidate must discover its package-local skills directory",
        ),
    ] {
        let fixture = Candidate::new()?;
        fixture.edit(
            "plugins/gameskills/.codex-plugin/plugin.json",
            pointer,
            value,
        )?;
        fixture.assert_failure(message);
    }
    Ok(())
}

#[cfg(unix)]
#[test]
fn directory_symlinks_and_contained_leaf_symlinks_are_explicitly_rejected() -> TestResult {
    let fixture = Candidate::new()?;
    std::os::unix::fs::symlink(
        "local.md",
        fixture.path("plugins/gameskills/references/alias.md"),
    )?;
    fixture.assert_failure("native package entries must not be symlinks");
    std::os::unix::fs::symlink(
        fixture.path("plugins/gameskills/references"),
        fixture.path("plugins/gameskills-ui/references/outside-directory"),
    )?;
    fixture.assert_failure("symlink escapes native package");
    let package = fixture.path("plugins/gameskills-ui");
    let moved = fixture.path("outside-package");
    std::fs::rename(&package, &moved)?;
    std::os::unix::fs::symlink(moved, package)?;
    fixture.assert_failure("native package root cannot be a symlink");
    let fixture = Candidate::new()?;
    std::fs::rename(fixture.path("plugins"), fixture.path("outside-plugins"))?;
    std::os::unix::fs::symlink(fixture.path("outside-plugins"), fixture.path("plugins"))?;
    fixture.assert_failure("native plugin directory cannot be a symlink");
    Ok(())
}

#[test]
fn failures_are_deterministic_and_invalid_utf8_is_an_error() -> TestResult {
    let fixture = Candidate::new()?;
    std::fs::write(fixture.skill(), [255, 254])?;
    let first = fixture.failures();
    assert!(!first.is_empty());
    assert_eq!(first, fixture.failures());
    Ok(())
}
