"""Routing over real Git revisions and required-result failure semantics."""

from pathlib import Path
import subprocess
import sys
import tempfile
import unittest
from unittest.mock import patch

sys.path.insert(0, str(Path(__file__).resolve().parents[1]))
import ci


class Routing(unittest.TestCase):
    def setUp(self):
        temporary = tempfile.TemporaryDirectory()
        self.addCleanup(temporary.cleanup)
        self.root = Path(temporary.name)
        self.git("init", "-q")
        for name, value in (("user.name", "CI test"), ("user.email", "ci@example.invalid"), ("commit.gpgsign", "false"), ("core.autocrlf", "false")):
            self.git("config", name, value)
        self.write("Cargo.toml", '[workspace]\nmembers = ["crates/*", "games/*", "games/labyrinth/rules"]\n[workspace.dependencies]\nshared = { package = "bevy_game_ui", path = "crates/ui" }\n')
        for folder, name, deps in (
            ("crates/ui", "bevy_game_ui", ""),
            ("crates/test", "bevy_game_test", "[dev-dependencies]\nshared.workspace = true\n"),
            ("crates/facade", "bevy-gamekit", '[target.\'cfg(unix)\'.build-dependencies]\nhelper = { package = "bevy_game_test", path = "../test" }\n'),
            ("games/labyrinth/rules", "labyrinth_rules", ""),
            ("games/labyrinth", "labyrinth", '[dependencies]\nfacade = { path = "../../crates/facade" }\nrules = { path = "rules" }\n'),
            ("games/deckbuilder_ui", "deckbuilder_ui", '[dependencies]\nfacade = { path = "../../crates/facade" }\n'),
            ("games/carterfight", "carterfight", '[dependencies]\nfacade = { path = "../../crates/facade" }\n'),
        ):
            self.write(folder + "/Cargo.toml", f'[package]\nname = "{name}"\nversion = "0.1.0"\n' + deps)
            self.write(folder + "/src/lib.rs", "// fixture\n")
        self.write("docs/existing.md", "Existing prose\n")
        self.base = self.save()

    def git(self, *args):
        return ci.git(self.root, *args).strip()

    def write(self, path, text):
        target = self.root / path
        target.parent.mkdir(parents=True, exist_ok=True)
        target.write_text(text, encoding="utf-8")

    def save(self):
        self.git("add", ".")
        self.git("commit", "-qm", "fixture")
        return self.git("rev-parse", "HEAD")

    def changed(self, *paths):
        for path in paths:
            self.write(path, "changed\n")
        return ci.select(self.root, self.base, self.save())

    def test_docs_avoid_expensive_jobs(self):
        value = self.changed("docs/testing.md", "games/labyrinth/README.md", "README.md")
        self.assertFalse(value["full"], value)
        self.assertFalse(any(value[job] for job in ci.FLAGS))

    def test_skill_markdown_and_runtime_select_skills_only(self):
        value = self.changed("plugins/gameskills/skills/plan/SKILL.md", "gameskills.toml", "plugins/gameskills/runtime/native.py")
        self.assertTrue(value["skills"])
        self.assertFalse(value["rust"])
        self.assertFalse(value["policy"])

    def test_each_game_is_isolated(self):
        for name in ("labyrinth", "deckbuilder_ui", "carterfight"):
            with self.subTest(name=name):
                value = self.changed(f"games/{name}/src/lib.rs")
                self.assertEqual(value["packages"], [name], value)
                self.assertFalse(value["skills"])
                self.assertFalse(value["distribution"])
                self.git("reset", "--hard", self.base)

    def test_nested_rules_owner_and_consumer(self):
        value = self.changed("games/labyrinth/rules/src/lib.rs")
        self.assertEqual(value["packages"], ["labyrinth", "labyrinth_rules"])
        self.assertTrue(value["wasm"])
        self.assertFalse(value["distribution"])

    def test_reverse_normal_dev_target_build_consumers(self):
        value = self.changed("crates/ui/src/lib.rs")
        self.assertEqual(set(value["packages"]), {"bevy_game_ui", "bevy_game_test", "bevy-gamekit", "labyrinth", "deckbuilder_ui", "carterfight"})
        self.assertTrue(all(value[key] for key in ("distribution", "minimal", "wasm")))
        self.assertFalse(value["skills"])
        self.assertFalse(value["deny"])

    def test_mixed_changes_union_jobs(self):
        value = self.changed("games/carterfight/assets/scene.png", "skills/README.md", "docs/test.md")
        self.assertTrue(value["skills"])
        self.assertEqual(value["packages"], ["carterfight"])

    def test_shared_unknown_and_routing_inputs_select_full(self):
        for path in ("unknown/config.json", "Cargo.lock", "deny.toml", ".cargo/config.toml", ".github/workflows/gamekit.yml", "scripts/ci.py", "scripts/tests/test_ci.py"):
            with self.subTest(path=path):
                self.assertTrue(self.changed(path)["full"])
                self.git("reset", "--hard", self.base)

    def test_missing_history_and_bad_graph_select_full(self):
        self.assertTrue(ci.select(self.root, "0" * 40, self.base)["full"])
        self.write("crates/ui/Cargo.toml", "broken TOML")
        self.assertTrue(ci.select(self.root, self.base, self.save())["full"])

    def test_invalid_head_is_error(self):
        with self.assertRaises(ValueError):
            ci.select(self.root, self.base, "main")

    def test_deleted_source_selects_owner(self):
        (self.root / "games/carterfight/src/lib.rs").unlink()
        self.assertEqual(ci.select(self.root, self.base, self.save())["packages"], ["carterfight"])

    def test_rename_selects_both_owners(self):
        self.git("mv", "games/carterfight/src/lib.rs", "games/deckbuilder_ui/src/moved.rs")
        self.assertEqual(ci.select(self.root, self.base, self.save())["packages"], ["carterfight", "deckbuilder_ui"])

    def test_included_docs_and_old_graph_consumers(self):
        self.write("games/carterfight/src/lib.rs", 'const HELP: &str = include_str!("../../../docs/existing.md");\n')
        self.base = self.save()
        self.assertEqual(self.changed("docs/existing.md")["packages"], ["carterfight"])
        self.assertEqual(self.changed("games/carterfight/src/lib.rs")["packages"], ["carterfight"])

    def test_dynamic_include_prevents_docs_shortcut(self):
        self.write("games/carterfight/src/lib.rs", 'include!(concat!(env!("OUT_DIR"), "/generated.rs"));\n')
        self.base = self.save()
        self.assertTrue(self.changed("docs/existing.md")["full"])

    def test_cross_game_asset_keeps_owner_and_include_consumer(self):
        self.write("games/carterfight/assets/shared.bin", "asset")
        self.write("games/labyrinth/src/lib.rs", 'const DATA: &[u8] = include_bytes!("../../carterfight/assets/shared.bin");\n')
        self.base = self.save()
        self.assertEqual(self.changed("games/carterfight/assets/shared.bin")["packages"], ["carterfight", "labyrinth"])

    def test_brace_and_bracket_includes_select_compiled_docs(self):
        for opening, closing in (("{", "}"), ("[", "]")):
            with self.subTest(opening=opening):
                self.write("games/carterfight/src/lib.rs", f'const HELP: &str = include_str!{opening}"../../../docs/existing.md"{closing};\n')
                self.write("docs/existing.md", f"before {opening}")
                self.base = self.save()
                self.assertEqual(self.changed("docs/existing.md")["packages"], ["carterfight"])

    def test_dynamic_include_also_prevents_asset_shortcut(self):
        self.write("games/labyrinth/src/lib.rs", 'include_bytes!(concat!(env!("CARGO_MANIFEST_DIR"), "/../carterfight/assets/shared.bin"));\n')
        self.base = self.save()
        self.assertTrue(self.changed("games/carterfight/assets/shared.bin")["full"])

    def test_explicit_build_script_prevents_docs_shortcut(self):
        path = self.root / "games/carterfight/Cargo.toml"
        path.write_text(path.read_text().replace('[package]', '[package]\nbuild = "codegen.rs"'))
        self.write("games/carterfight/codegen.rs", 'fn main() { let _ = std::fs::read("../../docs/existing.md"); }')
        self.base = self.save()
        self.assertTrue(self.changed("docs/existing.md")["full"])

    def test_included_skill_keeps_skill_checks(self):
        self.write("plugins/help.md", "Skill instructions")
        self.write("games/carterfight/src/lib.rs", 'const HELP: &str = include_str!("../../../plugins/help.md");\n')
        self.base = self.save()
        value = self.changed("plugins/help.md")
        self.assertTrue(value["skills"])
        self.assertEqual(value["packages"], ["carterfight"])

    def test_manual_full_run(self):
        self.assertTrue(ci.select(self.root, None, self.base, True)["full"])

    def test_repository_tool_changes_keep_owned_validation_without_game_tests(self):
        path = self.root / "Cargo.toml"
        path.write_text(path.read_text().replace('"crates/*",', '"tools/*", "crates/*",'))
        self.write("tools/gamekit-repo-tools/Cargo.toml", '[package]\nname = "gamekit-repo-tools"\nversion = "0.1.0"\n')
        self.write("tools/gamekit-repo-tools/src/lib.rs", "// fixture\n")
        self.base = self.save()
        value = self.changed("tools/gamekit-repo-tools/src/lib.rs")
        self.assertFalse(value["full"])
        self.assertEqual(value["packages"], ["gamekit-repo-tools"])
        self.assertTrue(all(value[key] for key in ("skills", "rust", "policy", "distribution")))
        self.assertFalse(any(value[key] for key in ("minimal", "wasm", "deny")))
        self.git("reset", "--hard", self.base)
        value = self.changed("tools/gamekit-repo-tools/README.md")
        self.assertFalse(any(value[key] for key in ci.FLAGS), value)


class Results(unittest.TestCase):
    def selection(self):
        value = ci.full_selection("a" * 40, "b" * 40, "fixture")
        value.update(full=False, packages=["carterfight"], skills=False)
        return value

    def needs(self):
        return {name: {"result": "skipped" if name == "skills" else "success"} for name in ("classify", *ci.JOBS)}

    def test_intentional_skip_passes(self):
        ci.gate(self.selection(), self.needs())

    def test_selected_failure_cancellation_missing_and_skip_fail(self):
        for job in ("classify", "rust", "policy"):
            for status in ("failure", "cancelled", "skipped", None):
                with self.subTest(job=job, status=status), self.assertRaises(ValueError):
                    needs = self.needs()
                    needs[job] = {"result": status}
                    ci.gate(self.selection(), needs)

    def test_malformed_selection_fails(self):
        for value in ({}, {"schema_version": 1}, {**self.selection(), "rust": "false"}, {**self.selection(), "packages": ["--help"]}):
            with self.subTest(value=value), self.assertRaises(ValueError):
                ci.gate(value, self.needs())

    def test_unselected_failure_not_hidden(self):
        needs = self.needs()
        needs["skills"]["result"] = "failure"
        with self.assertRaises(ValueError):
            ci.gate(self.selection(), needs)

    def test_game_commands_exclude_distribution_and_other_games(self):
        value = self.selection()
        value.update(distribution=False, minimal=False, wasm=False, deny=False)
        with patch.object(ci, "git", return_value=value["head"]), patch.object(subprocess, "run") as run:
            ci.run_checks(Path("."), "rust", value)
        commands = [call.args[0] for call in run.call_args_list]
        self.assertEqual(len(commands), 2)
        self.assertTrue(all("carterfight" in argv and "--workspace" not in argv for argv in commands))

    def test_labyrinth_keeps_real_process_check(self):
        value = self.selection()
        value.update(packages=["labyrinth"], distribution=False)
        with patch.object(ci, "git", return_value=value["head"]), patch.object(subprocess, "run") as run:
            ci.run_checks(Path("."), "rust", value)
        self.assertTrue(any("--ignored" in call.args[0] for call in run.call_args_list))

    def test_other_checkout_cannot_reuse_selection(self):
        with patch.object(ci, "git", return_value="c" * 40), self.assertRaises(ValueError):
            ci.run_checks(Path("."), "rust", self.selection())

    def test_skills_and_distribution_call_rust_validators(self):
        value = ci.full_selection("a" * 40, "b" * 40, "fixture")
        with patch.object(ci, "git", return_value=value["head"]), patch.object(subprocess, "run") as run:
            ci.run_checks(Path("."), "skills", value)
            ci.run_checks(Path("."), "rust", value)
        commands = [call.args[0] for call in run.call_args_list]
        prefix = ("cargo", "run", "--locked", "-p", "gamekit-repo-tools", "--profile", "ci", "--")
        for suffix in (("skills", "legacy"), ("skills", "validate"), ("distribution", "check")):
            self.assertIn(prefix + suffix, commands)
        self.assertFalse(any(arg.endswith(("check_distribution.py", "validate_skills.py", "validate_gameskills.py")) for argv in commands for arg in argv))


if __name__ == "__main__":
    unittest.main()
