"""Exercise installation with real immutable Git sources and project-owned files."""
from __future__ import annotations

import json
import os
from pathlib import Path
import shutil
import subprocess
import sys
import tempfile
import unittest
from unittest.mock import patch

ROOT = Path(__file__).resolve().parents[2]
sys.path.insert(0, str(ROOT / "plugins/gameskills/runtime"))
from gameskills_runtime import config, packaging


class PackagingTests(unittest.TestCase):
    def setUp(self):
        self.temporary = tempfile.TemporaryDirectory()
        self.addCleanup(self.temporary.cleanup)
        self.path = Path(self.temporary.name).resolve()
        self.source = self.path / "source"
        self.source.mkdir()
        shutil.copytree(ROOT / "plugins", self.source / "plugins",
                        ignore=shutil.ignore_patterns("__pycache__", "*.pyc"))
        self.git(self.source, "init", "-q")
        self.disable_automatic_maintenance(self.source)
        self.git(self.source, "add", ".")
        self.commit = self.commit_source()
        self.core = self.source / "plugins/gameskills"
        self.consumer = self.path / "consumer"
        self.consumer.mkdir()
        self.git(self.consumer, "init", "-q")
        self.disable_automatic_maintenance(self.consumer)
        (self.consumer / "README.md").write_text("A Bevy adopter\n")
        self.git(self.consumer, "add", ".")
        self.git(self.consumer, "-c", "user.name=Test", "-c", "user.email=test@example.test",
                 "commit", "-qm", "adopter")

    def git(self, root, *args):
        return subprocess.run(["git", *args], cwd=root, capture_output=True, text=True,
                              check=True).stdout.strip()

    def disable_automatic_maintenance(self, root):
        # Keep Git's background lock files from racing the no-writes snapshot.
        self.git(root, "config", "maintenance.auto", "false")
        self.git(root, "config", "gc.auto", "0")

    def commit_source(self):
        self.git(self.source, "-c", "user.name=Test", "-c", "user.email=test@example.test",
                 "commit", "-qam", "candidate")
        return self.git(self.source, "rev-parse", "HEAD")

    def stage(self, name="bundle", selected=None):
        destination = self.path / name
        packaging.bundle(self.source, destination, self.commit, selected or ["gameskills"])
        return destination

    def install(self, bundle, selected=None):
        return packaging.setup(self.consumer, self.core, ["--bundle", str(bundle), "--apply",
                                "--packages", *(selected or ["gameskills"])])

    def installed_core(self):
        lock = json.loads((self.consumer / config.LOCK).read_text())
        return self.consumer / lock["bundle"] / "plugins/gameskills"

    def test_proposal_has_no_writes_and_five_worker_default(self):
        before = sorted(p.relative_to(self.consumer) for p in self.consumer.rglob("*"))
        result = packaging.setup(self.consumer, self.core, [])
        self.assertFalse(result["applied"])
        self.assertEqual(result["max_workers"], 5)
        self.assertEqual(before, sorted(p.relative_to(self.consumer) for p in self.consumer.rglob("*")))

    def test_only_pinned_clean_source_can_be_bundled(self):
        for ref in ("main", "HEAD", "latest", self.commit[:8]):
            with self.assertRaises((ValueError, subprocess.CalledProcessError)):
                packaging.bundle(self.source, self.path / "bad", ref, ["gameskills"])
        (self.core / "catalog.json").write_text("changed")
        with self.assertRaisesRegex(ValueError, "uncommitted"):
            self.stage()

    def test_tag_is_resolved_to_commit_and_destination_is_immutable(self):
        self.git(self.source, "tag", "v0.1-fixture")
        bundle = self.path / "tagged"
        result = packaging.bundle(self.source, bundle, "v0.1-fixture", ["gameskills"])
        self.assertEqual(result["commit"], self.commit)
        with self.assertRaisesRegex(ValueError, "never overwritten"):
            packaging.bundle(self.source, bundle, self.commit, ["gameskills"])

    def test_selected_packages_can_change_at_same_commit_and_rollback(self):
        core_bundle = self.stage()
        first = self.install(core_bundle)
        with_ui = ["gameskills", "gameskills-ui"]
        ui_bundle = self.stage("with-ui", with_ui)
        second = self.install(ui_bundle, with_ui)
        self.assertNotEqual(first["destination"], second["destination"])
        self.assertEqual(packaging.status(self.consumer, self.installed_core())["packages"], with_ui)
        third = self.install(core_bundle)
        self.assertEqual(third["destination"], first["destination"])
        self.assertTrue(Path(second["destination"]).is_dir())

    def test_preserves_local_configuration_and_other_client_files(self):
        owned = config.initial_text(["gameskills"], enabled=True)
        owned += '\n# Preserve this override.\n[commands.check]\nargv = ["python3", "check.py"]\n'
        (self.consumer / config.CONFIG).write_text(owned)
        (self.consumer / "CLAUDE.md").write_text("Owner instructions\n")
        (self.consumer / ".agents").mkdir()
        (self.consumer / ".agents/custom.txt").write_text("local guidance\n")
        self.install(self.stage())
        self.assertEqual((self.consumer / config.CONFIG).read_text(), owned)
        self.assertEqual((self.consumer / "CLAUDE.md").read_text(), "Owner instructions\n")
        self.assertEqual((self.consumer / ".agents/custom.txt").read_text(), "local guidance\n")

    def test_update_pin_requires_explicit_setup_and_old_bundle_remains_usable(self):
        old_bundle = self.stage()
        self.install(old_bundle)
        previous_core = self.installed_core()
        (self.core / "change.txt").write_text("new candidate")
        self.git(self.source, "add", ".")
        self.commit = self.commit_source()
        with self.assertRaisesRegex(ValueError, "invoked core differs"):
            packaging.status(self.consumer, self.core)
        self.assertTrue(packaging.status(self.consumer, previous_core)["ok"])
        self.install(self.stage("updated"))
        self.assertEqual(packaging.status(self.consumer, self.installed_core())["commit"], self.commit)
        self.install(old_bundle)
        self.assertEqual(self.installed_core(), previous_core)

    def test_tampered_content_marketplace_and_unrecorded_package_are_rejected(self):
        bundle = self.stage()
        changed = bundle / "plugins/gameskills/catalog.json"
        original = changed.read_bytes()
        changed.write_text("tampered")
        with self.assertRaisesRegex(ValueError, "content mismatch"):
            packaging.verify_bundle(bundle)
        changed.write_bytes(original)
        market = bundle / ".agents/plugins/marketplace.json"
        original_market = market.read_bytes()
        value = json.loads(original_market)
        value["plugins"][0]["source"]["path"] = "../../outside"
        market.write_text(json.dumps(value))
        with self.assertRaisesRegex(ValueError, "marketplace disagrees"):
            packaging.verify_bundle(bundle)
        market.write_bytes(original_market)
        (bundle / "plugins/unrecorded").mkdir()
        with self.assertRaisesRegex(ValueError, "unrecorded packages"):
            packaging.verify_bundle(bundle)

    def test_portable_lock_rehydrates_in_another_clone_and_checks_selection(self):
        self.install(self.stage())
        other = self.path / "other"
        shutil.copytree(self.consumer, other)
        self.assertTrue(packaging.status(other, self.installed_core())["ok"])
        shutil.rmtree(other / ".gameskills")
        with self.assertRaisesRegex(ValueError, "not staged"):
            packaging.status(other, self.installed_core())
        text = (self.consumer / config.CONFIG).read_text()
        (self.consumer / config.CONFIG).write_text(config.select_packages(text, ["gameskills", "gameskills-ui"]))
        with self.assertRaisesRegex(ValueError, "selection changed"):
            packaging.status(self.consumer, self.installed_core())

    def test_interrupted_update_requires_explicit_recovery_and_restores_old_pin(self):
        self.install(self.stage())
        before = {name: (self.consumer / name).read_text() for name in (config.CONFIG, config.LOCK)}
        bundle = self.stage("ui", ["gameskills", "gameskills-ui"])
        original = packaging.atomic_text
        def fail_lock(path, value):
            if path.name == config.LOCK:
                raise OSError("simulated interruption")
            original(path, value)
        with patch.object(packaging, "atomic_text", side_effect=fail_lock):
            with self.assertRaisesRegex(OSError, "interruption"):
                self.install(bundle, ["gameskills", "gameskills-ui"])
        with self.assertRaisesRegex(ValueError, "interrupted setup"):
            packaging.setup(self.consumer, self.core, [])
        result = packaging.setup(self.consumer, self.core, ["--recover"])
        self.assertTrue(result["recovered"])
        self.assertEqual(before, {name: (self.consumer / name).read_text() for name in before})
        self.assertTrue(packaging.status(self.consumer, self.installed_core())["ok"])

    def test_recovery_refuses_new_local_edits(self):
        bundle = self.stage()
        original = packaging.atomic_text
        def interrupt(path, value):
            if path.name == config.LOCK:
                raise OSError("interrupted")
            original(path, value)
        with patch.object(packaging, "atomic_text", side_effect=interrupt):
            with self.assertRaises(OSError):
                self.install(bundle)
        local = (self.consumer / config.CONFIG).read_text() + "\n# New owner edit\n"
        (self.consumer / config.CONFIG).write_text(local)
        with self.assertRaisesRegex(ValueError, "overwrite a local edit"):
            packaging.setup(self.consumer, self.core, ["--recover"])
        self.assertEqual((self.consumer / config.CONFIG).read_text(), local)

    def test_setup_lock_excludes_second_mutator(self):
        state = self.consumer / ".gameskills"
        state.mkdir()
        with packaging.setup_guard(state):
            with self.assertRaisesRegex(ValueError, "another setup"):
                packaging.setup(self.consumer, self.core, ["--recover"])
        self.assertFalse(packaging.setup(self.consumer, self.core, ["--recover"])["recovered"])

    def test_native_commands_use_only_selected_bundle_and_preserve_user_files(self):
        bundle = self.stage()
        manifest = packaging.verify_bundle(bundle)
        codex = packaging.native_argv(bundle, "codex", ["exec", "--ephemeral"])
        claude = packaging.native_argv(bundle, "claude", ["--print", "test"])
        self.assertEqual(claude, ["claude", "--plugin-dir", str(bundle / "plugins/gameskills"), "--print", "test"])
        self.assertIn(f'plugins.gameskills@{packaging.marketplace_name(manifest)}.enabled=true', codex)
        self.assertNotIn("gameskills-ui", " ".join(codex))
        self.assertFalse((self.consumer / config.CONFIG).exists())

    def test_cli_applies_and_validates_installed_candidate(self):
        self.install(self.stage())
        script = self.installed_core() / "scripts/gameskills.py"
        result = subprocess.run([sys.executable, str(script), "--root", str(self.consumer), "status"],
                                capture_output=True, text=True, check=True)
        self.assertTrue(json.loads(result.stdout)["ok"])
        result = subprocess.run([sys.executable, str(script), "--root", str(self.consumer),
                                 "native", "codex", "--", "exec", "--ephemeral"],
                                capture_output=True, text=True, check=True)
        self.assertEqual(json.loads(result.stdout)["argv"][-2:], ["exec", "--ephemeral"])

    def test_worker_limits_are_strict_and_configuration_is_not_code(self):
        for count in (0, 6, True, "5"):
            value = config.defaults()
            value["dispatch"]["max_workers"] = count
            with self.assertRaisesRegex(ValueError, "1..5"):
                config.validate(value, {"gameskills"})
        original = '# user\nschema_version = 1\npackages = [\n "gameskills",\n]\n[project]\nname = "Owned"\n'
        selected = config.select_packages(original, ["gameskills", "gameskills-ui"])
        self.assertIn('[project]\nname = "Owned"', selected)

    def test_ignored_files_are_not_packaged_under_the_commit_identity(self):
        (self.source / ".gitignore").write_text("*.log\n")
        self.git(self.source, "add", ".gitignore")
        self.commit = self.commit_source()
        (self.core / "local-output.log").write_text("uncommitted local output")
        bundle = self.stage()
        self.assertFalse((bundle / "plugins/gameskills/local-output.log").exists())
        self.assertTrue(packaging.verify_bundle(bundle))

    def test_valid_quoted_indented_or_default_package_config_is_preserved(self):
        bundle = self.stage()
        for declaration in ('  packages = ["gameskills"]\n', '"packages" = ["gameskills"]\n', ""):
            content = "schema_version = 1\n" + declaration
            (self.consumer / config.CONFIG).write_text(content)
            self.install(bundle)
            self.assertEqual((self.consumer / config.CONFIG).read_text(), content)
            selected = config.select_packages(content, ["gameskills", "gameskills-ui"])
            self.assertIn("gameskills-ui", selected)

    @unittest.skipIf(os.name == "nt", "Creating symlinks requires optional Windows privileges")
    def test_symlink_state_bundle_and_files_are_rejected(self):
        outside = self.path / "outside"
        outside.mkdir()
        (self.consumer / ".gameskills").symlink_to(outside, target_is_directory=True)
        with self.assertRaisesRegex(ValueError, "ordinary project-local"):
            packaging.setup(self.consumer, self.core, [])
        (self.consumer / ".gameskills").unlink()
        bundle = self.stage()
        alias = self.path / "alias"
        alias.symlink_to(bundle, target_is_directory=True)
        with self.assertRaisesRegex(ValueError, "symlink"):
            self.install(alias)
        (bundle / "plugins/gameskills/unexpected").symlink_to(self.consumer / "README.md")
        with self.assertRaisesRegex(ValueError, "symlink"):
            packaging.verify_bundle(bundle)


if __name__ == "__main__":
    unittest.main()
