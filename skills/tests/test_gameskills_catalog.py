#!/usr/bin/env python3
"""Exercise catalog/discovery/link failures; fixtures are not agent evaluations."""
from __future__ import annotations

import json
import shutil
import subprocess
import sys
import tempfile
import unittest
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
sys.path.insert(0, str(ROOT / "skills/scripts"))
import validate_gameskills as validator


class GameSkillsCatalogTests(unittest.TestCase):
    """Mutate a complete portable candidate and inspect useful validation failures."""

    def setUp(self) -> None:
        self.scratch = tempfile.TemporaryDirectory()
        self.addCleanup(self.scratch.cleanup)
        self.root = Path(self.scratch.name) / "candidate"
        self.root.mkdir()
        self.version = "0.1.0-test.1"
        packages = {}
        codex_entries = []
        claude_entries = []
        for package, skills in validator.EXPECTED_SKILLS.items():
            package_root = self.root / "plugins" / package
            packages[package] = {"skills": list(skills), "requires": [] if package == "gameskills" else ["gameskills"],
                                 "description": "Scoped guidance for building and reviewing game changes."}
            for skill in skills:
                path = package_root / "skills" / skill / "SKILL.md"
                path.parent.mkdir(parents=True)
                path.write_text(f"---\nname: {skill}\ndescription: Inspect the owning implementation and verify the requested game behavior.\n---\n\n"
                                "# Scoped behavior\n\nRead the owning source, preserve the requested scope, and report actual observations with their limitations.\n")
            reference = package_root / "references/local.md"
            reference.parent.mkdir()
            reference.write_text("# Local reference\n\nA maintained package-local reference.\n")
            common = {"name": package, "version": self.version,
                      "description": packages[package]["description"], "author": {"name": "Fixture"}}
            self.write_json(package_root / ".claude-plugin/plugin.json", common)
            self.write_json(package_root / ".codex-plugin/plugin.json", {**common, "skills": "./skills/",
                            "interface": {"displayName": "Fixture", "capabilities": []}})
            codex_entries.append({"name": package, "source": {"source": "local", "path": f"./plugins/{package}"}})
            claude_entries.append({"name": package, "source": f"./plugins/{package}", "version": self.version})
        self.write_json(self.root / "plugins/gameskills/catalog.json", {"schema_version": 1, "version": self.version, "packages": packages})
        self.write_json(self.root / ".agents/plugins/marketplace.json", {"name": "gameskills", "plugins": codex_entries})
        self.write_json(self.root / ".claude-plugin/marketplace.json", {"name": "gameskills", "plugins": claude_entries})
        fixture = self.root / "skills/tests/gameskills-scenarios.json"
        fixture.parent.mkdir(parents=True)
        shutil.copyfile(ROOT / "skills/tests/gameskills-scenarios.json", fixture)

    @staticmethod
    def write_json(path: Path, data: dict) -> None:
        path.parent.mkdir(parents=True, exist_ok=True)
        path.write_text(json.dumps(data, indent=2) + "\n")

    def edit_json(self, relative: str, change) -> None:
        path = self.root / relative
        data = json.loads(path.read_text())
        change(data)
        self.write_json(path, data)

    def failures(self) -> list[str]:
        return validator.validate(self.root)

    def assert_failure(self, text: str) -> None:
        failures = self.failures()
        self.assertTrue(any(text in failure for failure in failures), "\n".join(failures))

    def skill_path(self, package: str = "gameskills", skill: str = "plan") -> Path:
        return self.root / "plugins" / package / "skills" / skill / "SKILL.md"

    def test_coherent_candidate_needs_no_literal_negative_phrase_or_ui_metadata(self) -> None:
        self.assertEqual(self.failures(), [])
        self.assertFalse(any((self.root / "plugins").rglob("openai.yaml")))

    def test_missing_or_duplicate_catalog_skill_is_not_hidden_by_a_valid_manifest(self) -> None:
        self.edit_json("plugins/gameskills/catalog.json", lambda data: data["packages"]["gameskills"]["skills"].append("plan"))
        self.assert_failure("catalog gameskills skills: inventory mismatch")

    def test_extra_native_skill_and_missing_native_body_are_detected(self) -> None:
        extra = self.root / "plugins/gameskills/skills/inject/SKILL.md"
        extra.parent.mkdir()
        extra.write_text(self.skill_path().read_text().replace("name: plan", "name: inject"))
        self.skill_path(skill="debug").unlink()
        self.assert_failure("gameskills native skill directories: inventory mismatch")
        self.assert_failure("discovered SKILL.md inventory mismatch")

    def test_optional_package_cannot_discover_sibling_core_skills(self) -> None:
        self.edit_json("plugins/gameskills-ui/.codex-plugin/plugin.json", lambda data: data.update(skills="../gameskills/skills/"))
        self.assert_failure("link escapes native package")
        self.assert_failure("skills must resolve to this package")

    def test_manifest_identity_and_versions_match_catalog(self) -> None:
        self.edit_json("plugins/gameskills-ui/.claude-plugin/plugin.json", lambda data: data.update(name="gameskills", version="8.0.0"))
        self.assert_failure("package name mismatch")
        self.assert_failure("version does not match catalog")

    def test_native_metadata_types_are_checked_without_assuming_client_execution(self) -> None:
        self.edit_json("plugins/gameskills/.codex-plugin/plugin.json", lambda data: data.update(interface={"capabilities": "skills", "displayName": 7}))
        self.assert_failure("interface.capabilities must be a string list")
        self.assert_failure("interface.displayName must be a nonempty string")

    def test_marketplace_inventory_and_source_mapping_are_both_checked(self) -> None:
        self.edit_json(".agents/plugins/marketplace.json", lambda data: data["plugins"].append(data["plugins"][0].copy()))
        self.edit_json(".claude-plugin/marketplace.json", lambda data: data["plugins"][1].update(source="./plugins/gameskills"))
        self.assert_failure("marketplace.json packages: inventory mismatch")
        self.assert_failure("gameskills-ui source does not resolve to its package")

    def test_existing_encoded_link_outside_package_is_rejected(self) -> None:
        path = self.skill_path("gameskills-ui", "build-ui")
        with path.open("a") as stream:
            stream.write("\n[Core](%2e%2e/%2e%2e/%2e%2e/gameskills/references/local.md)\n")
        self.assert_failure("link escapes native package")

    def test_reference_links_are_checked_and_fenced_examples_are_not_followed(self) -> None:
        path = self.skill_path()
        with path.open("a") as stream:
            stream.write("\n```markdown\n[Example](missing-example.md)\n```\n\n[Docs][detail]\n\n[detail]: ../../references/missing.md\n")
        failures = self.failures()
        self.assertTrue(any("references/missing.md" in failure for failure in failures))
        self.assertFalse(any("missing-example.md" in failure for failure in failures))

    def test_valid_space_and_fragment_local_links_are_portable(self) -> None:
        reference = self.root / "plugins/gameskills/references/space name.md"
        reference.write_text("# Heading\n")
        with self.skill_path().open("a") as stream:
            stream.write('\n[By angle](<../../references/space name.md#heading>)\n[By URL](../../references/space%20name.md)\n')
        self.assertEqual(self.failures(), [])

    def test_symlink_cannot_make_an_external_reference_look_package_local(self) -> None:
        outside = self.root / "outside.md"
        outside.write_text("Outside package content.\n")
        reference = self.root / "plugins/gameskills-ui/references/escape.md"
        reference.symlink_to(outside)
        with self.skill_path("gameskills-ui", "verify-ui").open("a") as stream:
            stream.write("\n[Reference](../../references/escape.md)\n")
        self.assert_failure("symlink escapes native package")
        self.assert_failure("link escapes native package")

    def test_symlink_loop_is_a_validation_error_without_a_traceback(self) -> None:
        reference = self.root / "plugins/gameskills/references/loop.md"
        reference.symlink_to("loop.md")
        self.assert_failure("unresolvable package symlink")

    def test_malformed_structural_types_report_failures_instead_of_crashing(self) -> None:
        self.edit_json(".agents/plugins/marketplace.json", lambda data: data["plugins"][0].update(name=[]))
        self.edit_json("skills/tests/gameskills-scenarios.json", lambda data: data["scenarios"][0]["routing"].update(primary=[]))
        self.edit_json("plugins/gameskills/catalog.json", lambda data: data.update(schema_version=True))
        self.assert_failure("expected a string list")
        self.assert_failure("unknown primary skill")
        self.assert_failure("schema_version must be 1")

    def test_duplicate_json_keys_are_an_error_instead_of_last_value_wins(self) -> None:
        path = self.root / "plugins/gameskills/.codex-plugin/plugin.json"
        path.write_text('{"name":"wrong", "name":"gameskills"}')
        self.assert_failure("duplicate JSON key 'name'")

    def test_required_frontmatter_supports_folded_and_quoted_strings(self) -> None:
        path = self.skill_path()
        _, body = validator.frontmatter(path.read_text())
        path.write_text('---\nname: "plan"\ndescription: >-\n  Investigate the scoped game request\n  and carry it through authorized delivery.\nmetadata:\n  short-description: Local guidance\n---\n\n' + body)
        self.assertEqual(self.failures(), [])

    def test_mismatched_empty_and_duplicate_frontmatter_fail(self) -> None:
        path = self.skill_path()
        path.write_text(path.read_text().replace("name: plan", "name: review"))
        self.assert_failure("frontmatter name must match")
        path.write_text(path.read_text().replace("name: review", "name: plan\nname: plan"))
        self.assert_failure("duplicate frontmatter key name")

    def test_underspecified_description_fails_without_enforcing_trigger_words(self) -> None:
        path = self.skill_path()
        text = path.read_text()
        original = validator.frontmatter(text)[0]["description"]
        path.write_text(text.replace(original, "Do stuff"))
        self.assert_failure("description must be a descriptive string")

    def test_optional_ui_metadata_cannot_point_at_another_skill(self) -> None:
        path = self.skill_path().parent / "agents/openai.yaml"
        path.parent.mkdir()
        path.write_text('interface:\n  display_name: "Plan"\n  short_description: "Plan a scoped game task"\n  default_prompt: "Use $release for this task."\n')
        self.assert_failure("default_prompt does not reference this skill")

    def test_scenario_rubrics_require_observable_criteria_and_consistent_routing(self) -> None:
        relative = "skills/tests/gameskills-scenarios.json"
        self.edit_json(relative, lambda data: data["scenarios"][0]["routing"]["avoid"].append("gameskills:plan"))
        self.edit_json(relative, lambda data: data["scenarios"][1]["rubric"].update(evidence=[]))
        self.assert_failure("contradictory routing rubric")
        self.assert_failure("missing nonempty rubric evidence")

    def test_fixtures_cannot_claim_execution_or_lose_required_boundary_cases(self) -> None:
        relative = "skills/tests/gameskills-scenarios.json"
        self.edit_json(relative, lambda data: data.update(evidence_status="passed"))
        self.edit_json(relative, lambda data: data.update(scenarios=[item for item in data["scenarios"] if "human-upstream" not in item["tags"]]))
        self.assert_failure("unexecuted-rubrics evidence_status")
        self.assert_failure("missing scenario coverage tags")

    def test_cli_reports_structural_scope_and_propagates_failure(self) -> None:
        command = [sys.executable, str(ROOT / "skills/scripts/validate_gameskills.py"), "--root", str(self.root), "--json"]
        result = subprocess.run(command, capture_output=True, text=True, check=False)
        self.assertEqual(result.returncode, 0, result.stderr + result.stdout)
        report = json.loads(result.stdout)
        self.assertTrue(report["ok"])
        self.assertTrue(report["structural_only"])
        self.assertIn("do not prove native installation", report["notice"])
        self.skill_path().unlink()
        result = subprocess.run(command, capture_output=True, text=True, check=False)
        self.assertEqual(result.returncode, 1)
        self.assertFalse(json.loads(result.stdout)["ok"])
        self.assertNotIn("Traceback", result.stderr)


if __name__ == "__main__":
    unittest.main()
