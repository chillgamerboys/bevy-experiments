#!/usr/bin/env python3
"""End-to-end tests for versioned skill installation and synchronization."""

from __future__ import annotations

import json
import shutil
import subprocess
import sys
import tempfile
import unittest
from pathlib import Path


ROOT = Path(__file__).resolve().parents[1]
TOOL = ROOT / "scripts/skills_tool.py"


class SkillsToolTests(unittest.TestCase):
    """Exercise clean install, idempotency, upgrades, and conflicts."""

    def setUp(self) -> None:
        self.scratch = tempfile.TemporaryDirectory()
        self.target = Path(self.scratch.name) / "adopter"
        self.target.mkdir()
        subprocess.run(["git", "init", "--quiet"], cwd=self.target, check=True)

    def tearDown(self) -> None:
        self.scratch.cleanup()

    def run_tool(self, *arguments: str, expected: int = 0) -> subprocess.CompletedProcess[str]:
        """Run the CLI and assert its exit status."""

        result = subprocess.run(
            [sys.executable, str(TOOL), *arguments],
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
            "--allow-unverified-source",
        )

    def test_install_is_complete_and_idempotent_for_both_clients(self) -> None:
        self.install()
        manifest = json.loads((self.target / ".bevy-gamekit/skills.json").read_text())
        self.assertEqual(len(manifest["skills"]), 7)
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
            "--allow-unverified-source",
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
        shutil.copytree(ROOT, next_source)
        skill = next_source / "source/architect-bevy-game/SKILL.md"
        skill.write_text(skill.read_text() + "\nPinned upgrade fixture.\n")
        addition = next_source / "source/architect-bevy-game/references/upgrade.md"
        addition.parent.mkdir()
        addition.write_text("Upgrade reference.\n")
        (next_source / "source/debug-bevy-runtime/agents/openai.yaml").unlink()
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
            "--allow-unverified-source",
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
        shutil.copytree(ROOT, next_source)
        source_skill = next_source / "source/build-bevy-ui/SKILL.md"
        source_skill.write_text(source_skill.read_text() + "\nUpstream edit.\n")
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
            "--allow-unverified-source",
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
            "--allow-unverified-source",
            expected=2,
        )


if __name__ == "__main__":
    unittest.main()
