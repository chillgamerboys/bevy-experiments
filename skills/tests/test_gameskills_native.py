"""Native protocol, source identity, and process-lifetime boundaries."""
from __future__ import annotations

import copy
import json
import os
from pathlib import Path
import shutil
import subprocess
import sys
import tempfile
import time
import unittest
from unittest.mock import patch

ROOT = Path(__file__).resolve().parents[2]
sys.path.insert(0, str(ROOT / "plugins/gameskills/runtime"))
from gameskills_runtime import native, packaging


class NativeTests(unittest.TestCase):
    def setUp(self):
        self.temporary = tempfile.TemporaryDirectory(prefix="gameskills-native-test-")
        self.addCleanup(self.temporary.cleanup)
        self.root = Path(self.temporary.name).resolve()
        self.bundle = self.root / "bundle"
        self.home = self.root / "native-home"
        catalog = {"schema_version": 1, "version": "0.1.0", "packages": {
            "gameskills": {"skills": ["plan", "test"]}, "gameskills-ui": {"skills": ["build-ui"]}}}
        for package, entry in catalog["packages"].items():
            for name in entry["skills"]:
                skill = self.bundle / "plugins" / package / "skills" / name / "SKILL.md"
                skill.parent.mkdir(parents=True)
                skill.write_text(f"---\nname: {name}\ndescription: Native test skill.\n---\nRead fixture source.\n")
        (self.bundle / "plugins/gameskills/catalog.json").write_text(json.dumps(catalog))
        self.manifest = {"schema_version": 1, "commit": "a" * 40, "version": "0.1.0",
                         "packages": {name: packaging.files(self.bundle / "plugins" / name) for name in catalog["packages"]}}
        self.market = packaging.marketplace_name(self.manifest)
        (self.bundle / "bundle.json").write_text(json.dumps(self.manifest))
        markets = packaging.marketplace_entries(list(catalog["packages"]), "0.1.0", self.market)
        for folder, content in zip([".agents/plugins", ".claude-plugin"], markets):
            (self.bundle / folder).mkdir(parents=True)
            (self.bundle / folder / "marketplace.json").write_text(json.dumps(content))
        self.skills = []
        for package, entry in catalog["packages"].items():
            destination = self.home / "plugins/cache" / self.market / package / "0.1.0"
            shutil.copytree(self.bundle / "plugins" / package, destination)
            for name in entry["skills"]:
                self.skills.append({"name": package + ":" + name, "pluginId": package + "@" + self.market,
                                    "description": "Native test skill.", "enabled": True, "scope": "user",
                                    "path": str(destination / "skills" / name / "SKILL.md")})
        self.scenario = {"home": str(self.home), "market": self.market, "skills": self.skills,
                         "plugins": [{"id": package + "@" + self.market, "enabled": True, "installed": True}
                                     for package in catalog["packages"]], "mode": "success"}
        self.scenario_path = self.root / "scenario.json"
        self.trace = self.root / "requests.jsonl"
        self.executable = self.root / "fake-codex"
        self.executable.write_text(f"#!{sys.executable}\n" + r'''
import json, os, signal, subprocess, sys, time
from pathlib import Path
root = Path(__file__).parent
scenario = json.loads((root / "scenario.json").read_text())
(root / "pid").write_text(str(os.getpid()))
seen_skills = 0
for line in sys.stdin:
    message = json.loads(line)
    with (root / "requests.jsonl").open("a") as trace: trace.write(line)
    if "id" not in message: continue
    method = message["method"]
    if scenario["mode"] == "exit": sys.exit(3)
    if scenario["mode"] == "malformed": print("not JSON", flush=True); continue
    if scenario["mode"] == "error": print(json.dumps({"id": message["id"], "error": {"code": -1, "message": "fixture error"}}), flush=True); continue
    if method == "initialize": result = {"codexHome": scenario["home"], "userAgent": "fake-codex/1"}
    elif method == "plugin/list": result = {"marketplaces": [{"name": scenario["market"], "plugins": scenario["plugins"]}]}
    elif method == "skills/list":
        if scenario["mode"] == "hang": time.sleep(60)
        if scenario["mode"] == "child_pipe":
            child = subprocess.Popen([sys.executable, "-c", "import time; time.sleep(60)"])
            (root / "child-pid").write_text(str(child.pid))
            sys.exit(0)
        seen_skills += 1
        skills = [] if scenario["mode"] == "delayed" and seen_skills == 1 else scenario["skills"]
        result = {"data": [{"cwd": message["params"]["cwds"][0], "skills": skills, "errors": []}]}
    else: raise AssertionError("unexpected RPC: " + method)
    print(json.dumps({"id": message["id"], "result": result}), flush=True)
''')
        self.executable.chmod(0o755)
        self.started_processes = []

    def activate(self, timeout=2):
        self.scenario_path.write_text(json.dumps(self.scenario))
        original = subprocess.Popen
        def start(command, **options):
            process = original([sys.executable, *command], **options)
            self.started_processes.append(process)
            return process
        with patch.object(native.subprocess, "Popen", side_effect=start):
            return native.activate_codex(self.bundle, self.root, executable=self.executable, timeout_seconds=timeout)

    def assert_process_gone(self):
        self.assertIsNotNone(self.started_processes[-1].poll())

    def test_discovers_exact_selected_packages_and_preserves_observed_paths(self):
        result = self.activate()
        self.assertEqual(result["packages"], ["gameskills", "gameskills-ui"])
        self.assertEqual(len(result["skills"]), 3)
        self.assertEqual({item["path"] for item in result["skills"]}, {item["path"] for item in self.skills})
        self.assertIn("behavior not tested", result["claim"])
        messages = [json.loads(line) for line in self.trace.read_text().splitlines()]
        self.assertEqual([item["method"] for item in messages], ["initialize", "initialized", "plugin/list", "skills/list", "plugin/list"])
        self.assertTrue(messages[3]["params"]["forceReload"])
        self.assert_process_gone()

    def test_delayed_materialization_is_retried_within_one_deadline(self):
        self.scenario["mode"] = "delayed"
        self.assertTrue(self.activate()["ok"])
        self.assertEqual(self.trace.read_text().count('"method": "skills/list"'), 2)

    def test_missing_selected_plugin_fails_without_a_skill_claim(self):
        self.scenario["plugins"].pop()
        with self.assertRaisesRegex(ValueError, "missing selected packages"):
            self.activate()
        self.assert_process_gone()

    def test_missing_skills_exhaust_bounded_discovery_wait(self):
        self.scenario["skills"] = self.skills[:1]
        started = time.monotonic()
        with self.assertRaisesRegex(ValueError, "timed out"):
            self.activate(timeout=0.5)
        self.assertLess(time.monotonic() - started, 3)
        self.assert_process_gone()

    def test_wrong_native_pin_or_unselected_package_is_rejected(self):
        wrong = copy.deepcopy(self.skills[0]); wrong["pluginId"] = "gameskills@older-pin"
        self.scenario["skills"].append(wrong)
        with self.assertRaisesRegex(ValueError, "another GameSkills pin"):
            self.activate()

    def test_native_bytes_must_match_entire_pinned_package(self):
        Path(self.skills[0]["path"]).write_text("edited native cache")
        with self.assertRaisesRegex(ValueError, "cache content differs"):
            self.activate()

    def test_source_directory_is_not_evidence_of_native_installation(self):
        self.scenario["skills"][0]["path"] = str(self.bundle / "plugins/gameskills/skills/plan/SKILL.md")
        with self.assertRaisesRegex(ValueError, "outside the pinned native cache"):
            self.activate()

    def test_rpc_errors_malformed_output_and_early_exit_fail_cleanly(self):
        for mode in ["error", "malformed", "exit"]:
            with self.subTest(mode=mode):
                self.scenario["mode"] = mode
                with self.assertRaises(ValueError):
                    self.activate()
                self.assert_process_gone()

    def test_unresponsive_native_process_is_terminated(self):
        self.scenario["mode"] = "hang"
        started = time.monotonic()
        with self.assertRaisesRegex(ValueError, "timed out"):
            self.activate(timeout=0.5)
        self.assertLess(time.monotonic() - started, 3)
        self.assert_process_gone()

    @unittest.skipUnless(os.name == "posix", "POSIX process-group boundary")
    def test_child_holding_stdout_cannot_hang_shutdown(self):
        self.scenario["mode"] = "child_pipe"
        started = time.monotonic()
        with self.assertRaises(ValueError):
            self.activate(timeout=0.5)
        self.assertLess(time.monotonic() - started, 3)
        self.assert_process_gone()
        child = int((self.root / "child-pid").read_text())
        result = subprocess.run(["ps", "-o", "stat=", "-p", str(child)], capture_output=True, text=True)
        self.assertTrue(not result.stdout.strip() or result.stdout.strip().startswith("Z"))


if __name__ == "__main__":
    unittest.main()
