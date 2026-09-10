#!/usr/bin/env python3
"""Real Git/process regression tests for command observations and invalidation."""

from __future__ import annotations

import copy
import importlib.util
import json
import os
from pathlib import Path
import signal
import subprocess
import sys
import tempfile
import time
import unittest
import uuid
from unittest.mock import patch


ROOT = Path(__file__).resolve().parents[2]
MODULE = ROOT / "plugins/gameskills/runtime/gameskills_runtime/runner.py"
SPEC = importlib.util.spec_from_file_location("gameskills_runner_under_test", MODULE)
runner = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(runner)
HARNESS = """
import importlib.util,json,sys
from pathlib import Path
spec=importlib.util.spec_from_file_location('runner',sys.argv[1])
runner=importlib.util.module_from_spec(spec); spec.loader.exec_module(runner)
config=json.loads(Path(sys.argv[3]).read_text())
try:
    result=runner.main(sys.argv[4:],Path(sys.argv[2]),config)
except (OSError,ValueError) as error:
    print(json.dumps({'ok':False,'error':str(error)})); sys.exit(2)
print(json.dumps(result)); sys.exit(0 if result['ok'] else 1)
"""


@unittest.skipUnless(os.name == "posix", "runner requires POSIX flock, process groups, and directory descriptors")
class GameSkillsRunnerTests(unittest.TestCase):
    def setUp(self) -> None:
        self.scratch = tempfile.TemporaryDirectory(prefix="gameskills-runner-test-")
        self.root = Path(self.scratch.name) / "project"
        self.root.mkdir()
        self.git("init", "--quiet")
        (self.root / ".gitignore").write_text(".gameskills/\n")
        (self.root / "source.txt").write_text("original\n")
        self.commit("initial")
        self.config = {"commands": {}, "dispatch": {"max_workers": 5}}
        self.processes = []

    def tearDown(self) -> None:
        for process in self.processes:
            if process.poll() is None:
                process.terminate()
            try:
                process.communicate(timeout=5)
            except subprocess.TimeoutExpired:
                process.kill()
                process.communicate()
        self.scratch.cleanup()

    def git(self, *argv: str) -> str:
        result = subprocess.run(["git", *argv], cwd=self.root, check=True,
                                capture_output=True, text=True)
        return result.stdout.strip()

    def commit(self, message: str) -> None:
        self.git("add", ".")
        self.git("-c", "user.name=Fixture", "-c", "user.email=fixture@example.invalid",
                 "commit", "--quiet", "-m", message)

    def command(self, name: str, code: str, **options: object) -> None:
        self.config["commands"][name] = {"argv": [sys.executable, "-c", code], **options}

    def run_checks(self, *args: str) -> dict:
        return runner.main(["run", *args], self.root, self.config)

    def evidence(self, run_id: str, operation: str = "validate") -> dict:
        return runner.main(["evidence", operation, run_id], self.root, self.config)

    def record_path(self, run_id: str) -> Path:
        return self.root / ".gameskills/runs" / run_id / "record.json"

    def launch(self, *args: str) -> subprocess.Popen:
        config_path = Path(self.scratch.name) / (uuid.uuid4().hex + ".json")
        config_path.write_text(json.dumps(self.config))
        process = subprocess.Popen([sys.executable, "-c", HARNESS, str(MODULE), str(self.root),
                                    str(config_path), *args], text=True,
                                   stdout=subprocess.PIPE, stderr=subprocess.PIPE)
        self.processes.append(process)
        return process

    def collect(self, process: subprocess.Popen, expected: int = 0) -> dict:
        out, err = process.communicate(timeout=12)
        diagnostic = out + err
        if process.returncode != expected and out:
            try:
                result = json.loads(out)
                diagnostic += self.record_path(result["run_id"]).read_text()
            except (ValueError, OSError, KeyError):
                pass
        self.assertEqual(process.returncode, expected, diagnostic)
        return json.loads(out)

    def await_file(self, path: Path) -> None:
        deadline = time.monotonic() + 8
        while not path.exists():
            if time.monotonic() > deadline:
                self.fail(f"process did not create {path}")
            time.sleep(0.02)

    def test_evidence_list_is_read_only_and_missing_setup_does_not_create_state(self) -> None:
        self.assertEqual(runner.main(["evidence", "list"], self.root, self.config)["runs"], [])
        self.assertFalse((self.root / ".gameskills").exists())

    def test_deterministic_dag_runs_shared_dependency_once_and_captures_output(self) -> None:
        self.command("base", "print('base')")
        self.command("zeta", "print('zeta')", requires=["base"])
        self.command("alpha", "import sys; print('alpha'); print('diagnostic',file=sys.stderr)", requires=["base"])
        result = self.run_checks("zeta", "alpha", "alpha")
        self.assertTrue(result["ok"], result)
        shown = self.evidence(result["run_id"], "show")
        self.assertTrue(shown["ok"], shown)
        self.assertEqual(shown["record"]["order"], ["base", "alpha", "zeta"])
        self.assertEqual(set(result["results"]), {"base", "alpha", "zeta"})
        log = self.record_path(result["run_id"]).parent / "alpha.stderr.log"
        self.assertEqual(log.read_text(), "diagnostic\n")
        self.assertEqual(result["claim"], "observed command checks only")

    def test_arguments_are_not_interpreted_by_a_shell(self) -> None:
        literal = "$(touch SHELL_EXPANDED); * > redirect"
        self.config["commands"]["literal"] = {"argv": [sys.executable, "-c", "import sys; print(sys.argv[1])", literal]}
        result = self.run_checks("literal")
        self.assertTrue(result["ok"])
        self.assertEqual((self.record_path(result["run_id"]).parent / "literal.stdout.log").read_text(), literal + "\n")
        self.assertFalse((self.root / "SHELL_EXPANDED").exists())

    def test_failure_preserves_logs_and_skips_dependents_but_runs_independent_work(self) -> None:
        self.command("fail", "import sys; print('before failure'); print('why',file=sys.stderr); sys.exit(7)")
        self.command("dependent", "raise AssertionError('must not execute')", requires=["fail"])
        self.command("independent", "print('still useful')")
        result = self.run_checks("dependent", "independent")
        self.assertFalse(result["ok"])
        self.assertEqual(result["results"]["fail"]["exit_code"], 7)
        self.assertEqual(result["results"]["dependent"]["status"], "skipped")
        self.assertEqual(result["results"]["independent"]["status"], "passed")
        self.assertEqual((self.record_path(result["run_id"]).parent / "fail.stderr.log").read_text(), "why\n")
        self.assertFalse(self.evidence(result["run_id"])["ok"])

    def test_invalid_graphs_and_traversal_are_rejected_before_writing(self) -> None:
        invalid = [
            {"a": {"argv": []}},
            {"a": {"argv": "echo untrusted"}},
            {"a": {"argv": [sys.executable], "cwd": "../outside"}},
            {"a": {"argv": [sys.executable], "cwd": "C:\\outside"}},
            {"a": {"argv": [sys.executable], "requires": ["missing"]}},
            {"a": {"argv": [sys.executable], "requires": ["a"]}},
            {"a": {"argv": [sys.executable], "timeout_seconds": float("nan")}},
        ]
        for commands in invalid:
            with self.subTest(commands=commands), self.assertRaises(ValueError):
                runner.main(["run", "a"], self.root, {"commands": commands})
        self.assertFalse((self.root / ".gameskills").exists())

    def test_cwd_symlink_is_rejected_and_real_subdirectory_is_used(self) -> None:
        sub = self.root / "nested"
        sub.mkdir()
        (self.root / "alias").symlink_to(sub, target_is_directory=True)
        self.command("a", "import os; print(os.getcwd())", cwd="alias")
        with self.assertRaises((ValueError, OSError)):
            self.run_checks("a")
        self.config["commands"]["a"]["cwd"] = "nested"
        result = self.run_checks("a")
        self.assertTrue(result["ok"])
        self.assertEqual((self.record_path(result["run_id"]).parent / "a.stdout.log").read_text().strip(), str(sub.resolve()))

    def test_timeout_kills_descendant_and_preserves_partial_output(self) -> None:
        marker = self.root / ".gameskills/escaped-timeout"
        child = f"import time; from pathlib import Path; time.sleep(0.8); Path({str(marker)!r}).write_text('escaped')"
        code = f"import subprocess,sys,time; subprocess.Popen([sys.executable,'-c',{child!r}]); print('started',flush=True); time.sleep(10)"
        self.command("hang", code, timeout_seconds=0.2)
        result = self.run_checks("hang")
        self.assertEqual(result["results"]["hang"]["status"], "timeout")
        self.assertIsNotNone(result["results"]["hang"]["exit_code"])
        time.sleep(0.85)
        self.assertFalse(marker.exists())
        self.assertIn("started", (self.record_path(result["run_id"]).parent / "hang.stdout.log").read_text())

    def test_successful_leader_cannot_leave_background_descendants(self) -> None:
        marker = self.root / ".gameskills/escaped-success"
        child = f"import time; from pathlib import Path; time.sleep(0.8); Path({str(marker)!r}).write_text('escaped')"
        self.command("spawn", f"import subprocess,sys; subprocess.Popen([sys.executable,'-c',{child!r}])")
        result = self.run_checks("spawn")
        self.assertTrue(result["ok"], result)
        time.sleep(0.85)
        self.assertFalse(marker.exists())

    def test_sigterm_cleans_descendants_records_interruption_and_skips_pending(self) -> None:
        ready = self.root / ".gameskills/signal-ready"
        escaped = self.root / ".gameskills/escaped-signal"
        child = f"import time; from pathlib import Path; time.sleep(0.8); Path({str(escaped)!r}).write_text('escaped')"
        self.command("hang", f"import subprocess,sys,time; from pathlib import Path; subprocess.Popen([sys.executable,'-c',{child!r}]); Path({str(ready)!r}).write_text('ready'); time.sleep(10)")
        self.command("later", "print('must not run')", requires=["hang"])
        process = self.launch("run", "later")
        self.await_file(ready)
        process.send_signal(signal.SIGTERM)
        result = self.collect(process, expected=1)
        self.assertEqual(result["status"], "interrupted")
        self.assertEqual(result["results"]["hang"]["status"], "interrupted")
        self.assertEqual(result["results"]["later"]["status"], "skipped")
        time.sleep(0.85)
        self.assertFalse(escaped.exists())

    def test_concurrency_is_useful_and_shared_resources_do_not_overlap(self) -> None:
        code = "import time; print(time.monotonic(),flush=True); time.sleep(.25); print(time.monotonic(),flush=True)"
        self.command("a", code)
        self.command("b", code)
        concurrent = self.run_checks("a", "b", "--max-workers", "2")
        self.assertTrue(concurrent["ok"])

        def interval(result: dict, name: str) -> list[float]:
            return [float(line) for line in (self.record_path(result["run_id"]).parent / f"{name}.stdout.log").read_text().splitlines()]

        a, b = interval(concurrent, "a"), interval(concurrent, "b")
        self.assertLess(max(a[0], b[0]), min(a[1], b[1]))
        resource = "test:" + uuid.uuid4().hex
        self.config["commands"]["a"]["resources"] = [resource]
        self.config["commands"]["b"]["resources"] = [resource]
        serial = self.run_checks("a", "b", "--max-workers", "2")
        a, b = interval(serial, "a"), interval(serial, "b")
        self.assertGreaterEqual(b[0], a[1])

    def test_resource_locks_coordinate_separate_runner_processes(self) -> None:
        self.command("work", "import time; print(time.monotonic(),flush=True); time.sleep(.4); print(time.monotonic(),flush=True)",
                     resources=["test:" + uuid.uuid4().hex])
        a = self.launch("run", "work")
        b = self.launch("run", "work")
        results = [self.collect(a), self.collect(b)]
        intervals = []
        for result in results:
            intervals.append([float(line) for line in (self.record_path(result["run_id"]).parent / "work.stdout.log").read_text().splitlines()])
        intervals.sort()
        self.assertGreaterEqual(intervals[1][0], intervals[0][1])

    def test_resource_wait_expiry_is_skipped_not_a_pass_and_does_not_block_unrelated_command(self) -> None:
        resource = "test:" + uuid.uuid4().hex
        repository = runner._repository(self.root)
        locks = runner._Resources(repository["common_dir"])
        held = locks.acquire([resource])
        self.assertIsNotNone(held)
        try:
            self.command("blocked", "print('must not run')", resources=[resource])
            self.command("useful", "print('done')")
            result = self.run_checks("blocked", "useful", "--resource-wait-seconds", "0")
            self.assertFalse(result["ok"])
            self.assertEqual(result["results"]["blocked"]["reason"], "resource wait expired")
            self.assertEqual(result["results"]["useful"]["status"], "passed")
        finally:
            locks.release(held)
            locks.close()

    def test_configured_worker_cap_is_enforced(self) -> None:
        self.command("a", "print('a')")
        self.config["dispatch"]["max_workers"] = 1
        with self.assertRaisesRegex(ValueError, "configured cap 1"):
            self.run_checks("a", "--max-workers", "2")
        with self.assertRaises(ValueError):
            self.run_checks("a", "--max-workers", "0")

    def test_changed_worktree_untracked_file_index_head_config_lock_and_environment_invalidate(self) -> None:
        self.command("a", "print('ok')")
        result = self.run_checks("a")
        run_id = result["run_id"]
        self.assertTrue(self.evidence(run_id)["ok"])
        source = self.root / "source.txt"
        source.write_text("changed\n")
        self.assertFalse(self.evidence(run_id)["ok"])
        source.write_text("original\n")
        self.assertTrue(self.evidence(run_id)["ok"])
        untracked = self.root / "untracked.txt"
        untracked.write_text("new input")
        self.assertFalse(self.evidence(run_id)["ok"])
        untracked.unlink()
        for filename in ("gameskills.toml", "gameskills.lock.json"):
            path = self.root / filename
            path.write_text("managed input")
            self.assertFalse(self.evidence(run_id)["ok"])
            path.unlink()
        modified_config = copy.deepcopy(self.config)
        modified_config["commands"]["a"]["timeout_seconds"] = 123
        self.assertFalse(runner.main(["evidence", "validate", run_id], self.root, modified_config)["ok"])
        with patch.dict(os.environ, {"GAMESKILLS_TEST_INPUT": "different"}):
            self.assertFalse(self.evidence(run_id)["ok"])
        source.write_text("staged\n")
        self.git("add", "source.txt")
        source.write_text("original\n")
        self.assertFalse(self.evidence(run_id)["ok"])
        self.git("reset", "--quiet", "HEAD", "source.txt")
        self.assertTrue(self.evidence(run_id)["ok"])
        self.git("-c", "user.name=Fixture", "-c", "user.email=fixture@example.invalid", "commit", "--quiet", "--allow-empty", "-m", "new head")
        self.assertFalse(self.evidence(run_id)["ok"])

    def test_input_change_during_successful_command_marks_run_stale(self) -> None:
        self.command("mutate", "from pathlib import Path; Path('source.txt').write_text('changed')")
        result = self.run_checks("mutate")
        self.assertEqual(result["results"]["mutate"]["status"], "passed")
        self.assertEqual(result["status"], "stale")
        self.assertFalse(result["ok"])
        self.assertFalse(self.evidence(result["run_id"])["ok"])

    def test_resume_reruns_graph_preserves_failed_record_and_rejects_new_inputs(self) -> None:
        self.command("retry", "from pathlib import Path; import sys; p=Path('.gameskills/retry-marker'); ready=p.exists(); p.write_text('attempt'); sys.exit(0 if ready else 3)")
        failed = self.run_checks("retry")
        prior = self.record_path(failed["run_id"]).read_bytes()
        self.assertFalse(failed["ok"])
        resumed = self.run_checks("retry", "--resume", failed["run_id"])
        self.assertTrue(resumed["ok"], resumed)
        self.assertNotEqual(resumed["run_id"], failed["run_id"])
        self.assertEqual(self.record_path(failed["run_id"]).read_bytes(), prior)
        shown = self.evidence(resumed["run_id"], "show")
        self.assertEqual(shown["record"]["resumed_from"], failed["run_id"])
        (self.root / "source.txt").write_text("new input")
        with self.assertRaisesRegex(ValueError, "inputs changed"):
            self.run_checks("retry", "--resume", failed["run_id"])

    def test_active_run_cannot_be_resumed(self) -> None:
        ready = self.root / ".gameskills/active-ready"
        self.command("wait", f"from pathlib import Path; import time; Path({str(ready)!r}).write_text('ready'); time.sleep(10)")
        process = self.launch("run", "wait")
        self.await_file(ready)
        run_id = next((self.root / ".gameskills/runs").iterdir()).name
        attempt = self.launch("run", "wait", "--resume", run_id)
        denied = self.collect(attempt, expected=2)
        self.assertIn("active run", denied["error"])
        process.terminate()
        self.collect(process, expected=1)

    def test_modified_record_and_modified_log_never_validate(self) -> None:
        self.command("a", "print('authentic output')")
        result = self.run_checks("a")
        path = self.record_path(result["run_id"])
        original = path.read_bytes()
        envelope = json.loads(original)
        envelope["record"]["results"]["a"]["argv"] = ["invented"]
        path.write_text(json.dumps(envelope))
        validation = self.evidence(result["run_id"])
        self.assertFalse(validation["ok"])
        self.assertIn("digest mismatch", " ".join(validation["reasons"]))
        path.write_bytes(original)
        (path.parent / "a.stdout.log").write_text("replacement output")
        self.assertFalse(self.evidence(result["run_id"])["ok"])

    def test_record_relocated_to_another_run_or_repository_is_invalid(self) -> None:
        self.command("a", "print('ok')")
        result = self.run_checks("a")
        new_id = uuid.uuid4().hex
        original = self.record_path(result["run_id"]).parent
        original.rename(original.with_name(new_id))
        self.assertFalse(self.evidence(new_id)["ok"])
        original.with_name(new_id).rename(original)
        moved = self.root.with_name("moved-project")
        self.root.rename(moved)
        self.root = moved
        self.assertFalse(self.evidence(result["run_id"])["ok"])

    def test_state_symlinks_record_symlinks_hardlinks_and_fifo_are_rejected(self) -> None:
        self.command("a", "print('ok')")
        external = Path(self.scratch.name) / "external"
        external.mkdir()
        state = self.root / ".gameskills"
        state.symlink_to(external, target_is_directory=True)
        with self.assertRaises((ValueError, OSError)):
            self.run_checks("a")
        self.assertEqual(list(external.iterdir()), [])
        state.unlink()
        result = self.run_checks("a")
        log = self.record_path(result["run_id"]).parent / "a.stdout.log"
        original = log.read_bytes()
        log.unlink()
        outside = external / "log"
        outside.write_bytes(original)
        log.symlink_to(outside)
        self.assertFalse(self.evidence(result["run_id"])["ok"])
        log.unlink()
        os.link(outside, log)
        self.assertFalse(self.evidence(result["run_id"])["ok"])
        log.unlink()
        os.mkfifo(log)
        self.assertFalse(self.evidence(result["run_id"])["ok"])

    def test_evidence_path_traversal_is_rejected(self) -> None:
        for value in ("../escape", "/absolute", "a" * 33, "ABCDEF", ".."):
            with self.subTest(value=value), self.assertRaises(ValueError):
                self.evidence(value)

    def test_swapped_state_directory_does_not_redirect_atomic_record_writes(self) -> None:
        state = self.root / ".gameskills"
        state.mkdir()
        original = state / "anchored"
        original.mkdir()
        elsewhere = Path(self.scratch.name) / "elsewhere"
        elsewhere.mkdir()
        with runner._Directory.root(state) as parent, parent.child("anchored") as directory:
            original.rename(state / "moved")
            original.symlink_to(elsewhere, target_is_directory=True)
            directory.write_json("record.json", {"observation": "anchored"})
        self.assertEqual(json.loads((state / "moved/record.json").read_text()), {"observation": "anchored"})
        self.assertFalse((elsewhere / "record.json").exists())

    def test_cwd_swap_after_open_cannot_redirect_process_outside_repository(self) -> None:
        nested = self.root / "nested"
        nested.mkdir()
        elsewhere = Path(self.scratch.name) / "outside-cwd"
        elsewhere.mkdir()
        self.command("where", "import os; print(os.getcwd())", cwd="nested")
        original_popen = subprocess.Popen

        def swap_then_launch(argv: list[str], *args: object, **kwargs: object) -> subprocess.Popen:
            if len(argv) > 2 and argv[2] == runner._EXEC:
                nested.rename(self.root / "moved")
                nested.symlink_to(elsewhere, target_is_directory=True)
            return original_popen(argv, *args, **kwargs)

        with patch.object(runner.subprocess, "Popen", side_effect=swap_then_launch):
            result = self.run_checks("where")
        self.assertEqual(result["status"], "stale")
        output = (self.record_path(result["run_id"]).parent / "where.stdout.log").read_text().strip()
        self.assertEqual(output, str((self.root / "moved").resolve()))

    def test_changed_external_executable_invalidates_evidence(self) -> None:
        executable = Path(self.scratch.name) / "external-command"
        executable.write_text("#!/bin/sh\nprintf original\\n\n")
        executable.chmod(0o755)
        self.config["commands"]["external"] = {"argv": [str(executable)]}
        result = self.run_checks("external")
        self.assertTrue(result["ok"], result)
        executable.write_text("#!/bin/sh\nprintf changed\\n\n")
        self.assertFalse(self.evidence(result["run_id"])["ok"])

    def test_relative_path_entry_resolves_from_actual_command_cwd(self) -> None:
        executable = self.root / "relative-program"
        executable.write_text("#!/bin/sh\nprintf 'local program'\n")
        executable.chmod(0o755)
        self.config["commands"]["relative"] = {"argv": ["relative-program"]}
        with patch.dict(os.environ, {"PATH": "." + os.pathsep + os.environ.get("PATH", os.defpath)}):
            result = self.run_checks("relative")
            self.assertTrue(result["ok"], result)
            record = self.evidence(result["run_id"], "show")["record"]
            self.assertEqual(record["identity"]["executables"]["relative"]["path"], str(executable.resolve()))

    def test_tracked_parent_replaced_by_symlink_is_not_followed_during_fingerprinting(self) -> None:
        nested = self.root / "nested"
        nested.mkdir()
        (nested / "input.txt").write_text("tracked")
        self.commit("nested source")
        self.command("a", "print('ok')")
        prior = self.run_checks("a")
        nested.rename(self.root / ".gameskills/moved-source")
        nested.symlink_to(self.root / ".gameskills/moved-source", target_is_directory=True)
        self.assertFalse(self.evidence(prior["run_id"])["ok"])
        with self.assertRaises((ValueError, OSError)):
            self.run_checks("a")

    def test_hard_killed_runner_cannot_be_resumed_while_command_retains_active_lock(self) -> None:
        ready = self.root / ".gameskills/hard-kill-ready"
        code = f"from pathlib import Path; import os,time; p=Path({str(ready)!r}); first=not p.exists(); p.write_text(str(os.getpid())); time.sleep(10 if first else 0); print('completed')"
        self.command("wait", code)
        process = self.launch("run", "wait")
        self.await_file(ready)
        command_pid = int(ready.read_text())
        run_id = next((self.root / ".gameskills/runs").iterdir()).name
        process.kill()
        process.communicate(timeout=5)
        self.assertEqual(process.returncode, -signal.SIGKILL)
        try:
            denied = self.collect(self.launch("run", "wait", "--resume", run_id), expected=2)
            self.assertIn("active run", denied["error"])
            self.assertFalse(self.evidence(run_id)["ok"])
        finally:
            try:
                os.killpg(command_pid, signal.SIGTERM)
            except ProcessLookupError:
                pass
        time.sleep(0.1)
        resumed = self.collect(self.launch("run", "wait", "--resume", run_id))
        self.assertTrue(resumed["ok"])
        self.assertNotEqual(resumed["run_id"], run_id)

    def test_resource_initialization_failure_preserves_a_failed_record(self) -> None:
        self.command("a", "print('must not execute')")
        with patch.object(runner, "_Resources", side_effect=ValueError("unsafe resource directory")):
            result = self.run_checks("a")
        self.assertFalse(result["ok"])
        self.assertEqual(result["status"], "failed")
        envelope = json.loads(self.record_path(result["run_id"]).read_text())
        self.assertEqual(envelope["record"]["runner_error"], "unsafe resource directory")
        self.assertEqual(envelope["record"]["results"]["a"]["status"], "skipped")

    def test_missing_executable_is_failed_observation_with_real_error_log(self) -> None:
        self.config["commands"]["missing"] = {"argv": ["gameskills-definitely-missing-" + uuid.uuid4().hex]}
        result = self.run_checks("missing")
        self.assertFalse(result["ok"])
        self.assertEqual(result["results"]["missing"]["status"], "failed")
        self.assertNotEqual(result["results"]["missing"]["exit_code"], 0)
        self.assertIn("FileNotFoundError", (self.record_path(result["run_id"]).parent / "missing.stderr.log").read_text())


if __name__ == "__main__":
    unittest.main()
