#!/usr/bin/env python3
"""End-to-end tests for versioned skill installation and synchronization."""

from __future__ import annotations

import json
import shutil
import subprocess
import sys
import tempfile
import unittest
from pathlib import Path, PurePosixPath, PureWindowsPath
from unittest.mock import patch


ROOT = Path(__file__).resolve().parents[1]
TOOL = ROOT / "scripts/skills_tool.py"
sys.path.insert(0, str(ROOT / "scripts"))
import skills_tool


class SkillsToolTests(unittest.TestCase):
    """Exercise clean install, idempotency, upgrades, and conflicts."""

    def setUp(self) -> None:
        self.scratch = tempfile.TemporaryDirectory()
        self.target = Path(self.scratch.name) / "adopter"
        self.target.mkdir()
        subprocess.run(["git", "init", "--quiet"], cwd=self.target, check=True)
        self.source = Path(self.scratch.name) / "source"
        shutil.copytree(ROOT, self.source, ignore=shutil.ignore_patterns("__pycache__"))
        subprocess.run(["git", "init", "--quiet"], cwd=self.source, check=True)
        self.commit_source(self.source, "v0.1.0")

    def commit_source(self, source: Path, tag: str) -> None:
        """Pin fixture changes with a real commit and tag."""
        subprocess.run(["git", "add", "."], cwd=source, check=True)
        subprocess.run(["git", "-c", "user.name=Fixture", "-c", "user.email=fixture@example.invalid",
                        "commit", "--quiet", "-m", tag], cwd=source, check=True)
        subprocess.run(["git", "tag", tag], cwd=source, check=True)

    def tearDown(self) -> None:
        self.scratch.cleanup()

    def run_tool(self, *arguments: str, expected: int = 0) -> subprocess.CompletedProcess[str]:
        """Run the CLI and assert its exit status."""

        result = subprocess.run(
            [sys.executable, str(TOOL), *arguments, "--source-root", str(self.source)] if "--source-root" not in arguments else [sys.executable, str(TOOL), *arguments],
            check=False,
            capture_output=True,
            text=True,
        )
        self.assertEqual(result.returncode, expected, result.stdout + result.stderr)
        return result

    def install(self) -> None:
        """Install the current canonical fixture."""

        self.run_tool(
            "install",
            "--target",
            str(self.target),
            "--repository",
            "ssh://example.invalid/bevy-game-library",
            "--revision",
            "v0.1.0",
            "--apply",
        )

    def test_install_is_complete_and_idempotent_for_both_clients(self) -> None:
        self.install()
        manifest = json.loads((self.target / ".bevy-gamekit/skills.json").read_text())
        self.assertEqual(len(manifest["skills"]), 7)
        self.assertRegex(manifest["source"]["resolved_sha"], r"^[0-9a-f]{40}$")
        for skill in manifest["skills"]:
            codex = self.target / ".agents/skills" / skill / "SKILL.md"
            claude = self.target / ".claude/skills" / skill / "SKILL.md"
            self.assertEqual(codex.read_bytes(), claude.read_bytes())
        before = sorted(
            (path.relative_to(self.target), path.read_bytes())
            for path in self.target.rglob("*")
            if path.is_file() and ".git" not in path.parts
        )
        self.run_tool(
            "install",
            "--target",
            str(self.target),
            "--repository",
            "ssh://example.invalid/bevy-game-library",
            "--revision",
            "v0.1.0",
            "--apply",
            "--allow-dirty",
        )
        after = sorted(
            (path.relative_to(self.target), path.read_bytes())
            for path in self.target.rglob("*")
            if path.is_file() and ".git" not in path.parts
        )
        self.assertEqual(before, after)

    def test_sync_updates_untouched_files_and_preserves_overlays(self) -> None:
        self.install()
        overlay = self.target / ".bevy-gamekit/overlays/build-bevy-ui.md"
        overlay.write_text("Game-local UI convention.\n")
        next_source = Path(self.scratch.name) / "next-source"
        shutil.copytree(self.source, next_source)
        skill = next_source / "source/architect-bevy-game/SKILL.md"
        skill.write_text(skill.read_text() + "\nPinned upgrade fixture.\n")
        addition = next_source / "source/architect-bevy-game/references/upgrade.md"
        addition.parent.mkdir()
        addition.write_text("Upgrade reference.\n")
        (next_source / "source/debug-bevy-runtime/agents/openai.yaml").unlink()
        self.commit_source(next_source, "v0.2.0")
        self.run_tool(
            "sync",
            "--target",
            str(self.target),
            "--revision",
            "v0.2.0",
            "--source-root",
            str(next_source),
            "--apply",
            "--allow-dirty",
        )
        self.assertEqual(overlay.read_text(), "Game-local UI convention.\n")
        manifest = json.loads((self.target / ".bevy-gamekit/skills.json").read_text())
        self.assertEqual(manifest["source"]["revision"], "v0.2.0")
        self.assertIn(
            "Pinned upgrade fixture.",
            (self.target / ".agents/skills/architect-bevy-game/SKILL.md").read_text(),
        )
        self.assertTrue(
            (self.target / ".claude/skills/architect-bevy-game/references/upgrade.md").is_file()
        )
        self.assertFalse(
            (self.target / ".agents/skills/debug-bevy-runtime/agents/openai.yaml").exists()
        )

    def test_sync_reports_generated_file_conflict_without_overwrite(self) -> None:
        self.install()
        generated = self.target / ".agents/skills/build-bevy-ui/SKILL.md"
        generated.write_text(generated.read_text() + "\nLocal generated edit.\n")
        next_source = Path(self.scratch.name) / "next-source"
        shutil.copytree(self.source, next_source)
        source_skill = next_source / "source/build-bevy-ui/SKILL.md"
        source_skill.write_text(source_skill.read_text() + "\nUpstream edit.\n")
        self.commit_source(next_source, "v0.2.0")
        result = self.run_tool(
            "sync",
            "--target",
            str(self.target),
            "--revision",
            "v0.2.0",
            "--source-root",
            str(next_source),
            "--apply",
            "--allow-dirty",
            expected=2,
        )
        self.assertIn("CONFLICT", result.stdout)
        self.assertIn("Local generated edit.", generated.read_text())
        manifest = json.loads((self.target / ".bevy-gamekit/skills.json").read_text())
        self.assertEqual(manifest["source"]["revision"], "v0.1.0")

    def test_moving_revision_is_rejected(self) -> None:
        self.run_tool(
            "install",
            "--target",
            str(self.target),
            "--repository",
            "ssh://example.invalid/bevy-game-library",
            "--revision",
            "latest",
            expected=2,
        )

    def test_arbitrary_branch_is_not_a_pin(self) -> None:
        subprocess.run(["git", "branch", "release-looking-branch"], cwd=self.source, check=True)
        self.run_tool("install", "--target", str(self.target), "--repository", "fixture",
                      "--revision", "release-looking-branch", expected=2)

    def test_sync_rejects_modified_or_missing_cached_base(self) -> None:
        self.install()
        base = self.target / ".bevy-gamekit/base/.agents/skills/build-bevy-ui/SKILL.md"
        base.write_text("tampered base")
        result = self.run_tool("sync", "--target", str(self.target), "--revision", "v0.1.0",
                              "--allow-dirty", "--apply", expected=2)
        self.assertIn("does not match recorded hashes", result.stderr)
        base.unlink()
        self.run_tool("sync", "--target", str(self.target), "--revision", "v0.1.0",
                      "--allow-dirty", expected=2)

    def test_sync_rejects_manifest_path_traversal(self) -> None:
        self.install()
        path = self.target / ".bevy-gamekit/skills.json"
        manifest = json.loads(path.read_text())
        manifest["generated"]["../outside"] = "0" * 64
        path.write_text(json.dumps(manifest))
        result = self.run_tool("sync", "--target", str(self.target), "--revision", "v0.1.0",
                              "--allow-dirty", expected=2)
        self.assertIn("unsafe generated path", result.stderr)

    def test_manifest_and_base_hashes_are_portable_between_path_flavors(self) -> None:
        self.install()
        manifest_path = self.target / ".bevy-gamekit/skills.json"
        original = json.loads(manifest_path.read_text())
        rendered = skills_tool.render(self.source)
        expected = {path.as_posix(): skills_tool.digest(data) for path, data in rendered.items()}
        for flavor in (PureWindowsPath, PurePosixPath):
            with self.subTest(flavor=flavor.__name__):
                foreign_paths = {flavor(path.as_posix()): data for path, data in rendered.items()}
                skills_tool.write_manifest(
                    self.target, original["source"]["repository"], "v0.1.0",
                    original["source"]["resolved_sha"], foreign_paths,
                )
                manifest = json.loads(manifest_path.read_text())
                self.assertEqual(manifest["generated"], expected)
                self.assertTrue(all("\\" not in name for name in manifest["generated"]))
                self.assertEqual(skills_tool.verified_base(self.target, manifest), rendered)
                # Simulate a base read on either OS, without requiring that host's
                # filesystem. Hash comparison must not depend on pathlib flavor.
                with patch.object(skills_tool, "read_tree", return_value=foreign_paths):
                    self.assertEqual(skills_tool.verified_base(self.target, manifest), foreign_paths)
                result = self.run_tool(
                    "sync", "--target", str(self.target), "--revision", "v0.1.0",
                    "--allow-dirty", "--apply",
                )
                self.assertNotIn("CONFLICT", result.stdout)
                self.assertEqual(json.loads(manifest_path.read_text())["generated"], expected)

    def test_sync_rejects_legacy_or_noncanonical_path_names_without_changes(self) -> None:
        self.install()
        manifest_path = self.target / ".bevy-gamekit/skills.json"
        original = manifest_path.read_text()
        for name in (
            r".agents\skills\build-bevy-ui\SKILL.md",
            ".agents//skills/build-bevy-ui/SKILL.md",
            "./.agents/skills/build-bevy-ui/SKILL.md",
        ):
            with self.subTest(name=name):
                manifest = json.loads(original)
                checksum = manifest["generated"].pop(".agents/skills/build-bevy-ui/SKILL.md")
                manifest["generated"][name] = checksum
                manifest_path.write_text(json.dumps(manifest))
                before = {
                    path.relative_to(self.target): path.read_bytes()
                    for path in self.target.rglob("*")
                    if path.is_file() and ".git" not in path.parts
                }
                result = self.run_tool(
                    "sync", "--target", str(self.target), "--revision", "v0.1.0",
                    "--allow-dirty", "--apply", expected=2,
                )
                self.assertIn("canonical", result.stderr)
                after = {
                    path.relative_to(self.target): path.read_bytes()
                    for path in self.target.rglob("*")
                    if path.is_file() and ".git" not in path.parts
                }
                self.assertEqual(before, after)


if __name__ == "__main__":
    unittest.main()
