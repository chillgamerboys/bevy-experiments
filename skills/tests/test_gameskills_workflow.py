"""Git-backed queue behavior, concurrency, and failure-atomicity checks."""
from concurrent.futures import ThreadPoolExecutor
import copy
import importlib.util
import json
import os
from pathlib import Path
import subprocess
import tempfile
import unittest


MODULE = Path(__file__).resolve().parents[2] / "plugins/gameskills/runtime/gameskills_runtime/workflow.py"
spec = importlib.util.spec_from_file_location("gameskills_workflow_test_module", MODULE)
workflow = importlib.util.module_from_spec(spec)
spec.loader.exec_module(workflow)


@unittest.skipUnless(os.name == "posix", "queue operations require POSIX advisory locking")
class WorkflowTests(unittest.TestCase):
    def setUp(self):
        self.temporary = tempfile.TemporaryDirectory(prefix="gameskills-workflow-")
        self.addCleanup(self.temporary.cleanup)
        self.home = Path(self.temporary.name).resolve()
        self.root = self.home / "repo"
        self.root.mkdir()
        self.git("init", "-q", "-b", "main")
        self.git("config", "user.name", "Test")
        self.git("config", "user.email", "test@example.invalid")
        (self.root / "README.md").write_text("initial\n")
        self.git("add", ".")
        self.git("commit", "-qm", "initial")
        self.base = self.git("rev-parse", "HEAD")
        self.config = {"dispatch": {"enabled": True, "max_workers": 5},
                       "packages": ["gameskills", "gameskills-ui"], "creative": {"default_level": 2}}

    def git(self, *args, cwd=None):
        return subprocess.check_output(["git", "-C", str(cwd or self.root), *args], text=True, stderr=subprocess.PIPE).strip()

    def worktree(self, name, commit=None):
        path = self.home / name
        self.git("worktree", "add", "-q", "-b", name, str(path), commit or self.base)
        return path

    def order(self, name, **kwargs):
        value = {"id": name, "goal": "Bounded change", "artifact": "Committed source", "target": "game",
                 "creative_scope": "Preserve the established rules", "owner": {"kind": "agent", "name": name},
                 "execution": {"role": "implementation", "effort": "default"}, "decisions": [],
                 "investigation": ["Observed the current source"], "references": [],
                 "acceptance": ["Check the changed behavior"], "expected_evidence": ["Source-bound check result"],
                 "coordination": ["Ask the coordinator before expanding ownership"], "resources": [],
                 "files": [{"path": name + ".rs"}], "dispatch_blockers": [], "merge_blockers": []}
        value.update(kwargs)
        return value

    def plan(self, *orders):
        return {"schema_version": 1, "id": "wave", "goal": "Improve the game", "delivery_target": "pr",
                "packages": ["gameskills"], "decisions": [], "versions": {"bevy": "0.18", "gamekit": None},
                "repository": {"root": str(self.root), "base_commit": self.base, "source_commit": self.base},
                "orders": list(orders or [self.order("one")])}

    def create(self, *orders):
        return workflow.create_queue(self.plan(*orders), self.root, self.config)

    def mutate(self, action, order="one", payload=None, worktree=None, revision=None, config=None):
        if revision is None:
            revision = self.read()["revision"]
        return workflow.mutate_queue("wave", action, self.root, config or self.config, revision,
                                     order_id=order, payload=payload, worktree=worktree)

    def read(self):
        return json.loads(self.queue_path().read_text())

    def queue_path(self):
        return self.root / ".gameskills/queues/wave.json"

    def assert_unchanged_error(self, callback, contains=None):
        before = self.queue_path().read_bytes()
        with self.assertRaises(workflow.WorkflowError) as caught:
            callback()
        if contains:
            self.assertIn(contains, str(caught.exception))
        self.assertEqual(before, self.queue_path().read_bytes())

    def report(self, tree, order="one", **kwargs):
        value = {"summary": "Implemented bounded work", "worktree": str(tree), "head": self.git("rev-parse", "HEAD", cwd=tree),
                 "base_commit": self.base, "checks_running": False,
                 "evidence": [{"kind": "test", "reference": "run:caller-reference", "summary": "Caller reports test success"}]}
        value.update(kwargs)
        return self.mutate("report", order, payload=value)

    def commit(self, tree, filename, content):
        (tree / filename).write_text(content)
        self.git("add", filename, cwd=tree)
        self.git("commit", "-qm", "change " + filename, cwd=tree)
        return self.git("rev-parse", "HEAD", cwd=tree)

    def integrate(self, order, source):
        self.git("merge", "--no-edit", source)
        return self.mutate("integrated", order, payload={"summary": "Coordinator observed integration",
                           "head": self.git("rev-parse", "HEAD"), "source_head": source,
                           "evidence_reference": "review-and-combined-checks-reference"})

    def test_plan_preserves_input_and_normalizes_identity(self):
        plan = self.plan()
        original = copy.deepcopy(plan)
        validated = workflow.validate_plan(plan, self.root, self.config)
        self.assertEqual(plan, original)
        self.assertEqual(validated["creative_level"], 2)
        self.assertEqual(validated["repository"]["common_dir"], str(self.root / ".git"))
        self.assertEqual(validated["orders"][0]["creative_level"], 2)

    def test_invalid_ids_dependencies_and_cycles(self):
        for identity in ("../escape", "a/b", ".", "x\\y", "", "a" * 65):
            plan = self.plan()
            plan["id"] = identity
            with self.subTest(identity=identity), self.assertRaises(workflow.WorkflowError):
                workflow.validate_plan(plan, self.root, self.config)
        for orders in ([self.order("a", dispatch_blockers=["missing"])],
                       [self.order("a", dispatch_blockers=["b"]), self.order("b", merge_blockers=["a"])],
                       [self.order("a"), self.order("a")]):
            with self.assertRaises(workflow.WorkflowError):
                workflow.validate_plan(self.plan(*orders), self.root, self.config)
        self.assertFalse((self.root / ".gameskills").exists())

    def test_line_ranges_and_directories_enforce_ownership(self):
        first = self.order("a", files=[{"path": "src/game.rs", "lines": [1, 20]}])
        separate = self.order("b", files=[{"path": "src/game.rs", "lines": [21, 50]}])
        workflow.validate_plan(self.plan(first, separate), self.root, self.config)
        for ownership in ([{"path": "src/game.rs", "lines": [20, 30]}], [{"path": "src/game.rs"}], [{"path": "src/"}]):
            with self.assertRaises(workflow.WorkflowError):
                workflow.validate_plan(self.plan(first, self.order("b", files=ownership)), self.root, self.config)
        serialized = self.order("b", files=first["files"], dispatch_blockers=["a"])
        workflow.validate_plan(self.plan(first, serialized), self.root, self.config)
        for path in ("../src/x", "/src/x", "a/../x", "a//x", "*.rs", ".git/config", ".gameskills"):
            with self.subTest(path=path), self.assertRaises(workflow.WorkflowError):
                workflow.validate_plan(self.plan(self.order("a", files=[{"path": path}])), self.root, self.config)
        (self.root / "alias").symlink_to(self.root / "README.md")
        with self.assertRaisesRegex(workflow.WorkflowError, "symbolic"):
            workflow.validate_plan(self.plan(self.order("a", files=[{"path": "alias"}])), self.root, self.config)

    def test_repository_source_and_base_identity(self):
        tree = self.worktree("other")
        plan = self.plan()
        plan["repository"]["root"] = str(tree)
        with self.assertRaises(workflow.WorkflowError):
            workflow.validate_plan(plan, self.root, self.config)
        future = self.commit(tree, "other.rs", "future")
        plan = self.plan()
        plan["repository"]["source_commit"] = future
        with self.assertRaisesRegex(workflow.WorkflowError, "stale"):
            workflow.validate_plan(plan, self.root, self.config)
        plan = self.plan()
        plan["repository"]["base_commit"] = future
        with self.assertRaisesRegex(workflow.WorkflowError, "descend"):
            workflow.validate_plan(plan, self.root, self.config)

    def test_injection_is_atomic_add_only_and_source_bound(self):
        self.create()
        addition = self.order("two", reason="Verified additional scope", verified_source=self.base)
        self.mutate("inject", payload=addition)
        self.assertEqual(set(self.read()["orders"]), {"one", "two"})
        self.assertEqual(self.read()["revision"], 2)
        self.assert_unchanged_error(lambda: self.mutate("inject", payload=addition), "add-only")
        bad = self.order("three", files=[{"path": "one.rs"}], reason="Need another fix", verified_source=self.base)
        self.assert_unchanged_error(lambda: self.mutate("inject", payload=bad), "collision")
        bad["dispatch_blockers"] = ["one"]
        self.mutate("inject", payload=bad)
        stale = self.order("four", reason="Addition", verified_source="0" * 40)
        self.assert_unchanged_error(lambda: self.mutate("inject", payload=stale), "verified_source")
        self.assert_unchanged_error(lambda: self.mutate("inject", payload=stale, revision=1), "stale revision")
        existing = copy.deepcopy(self.read()["orders"]["one"]["spec"])
        self.assertEqual(existing["files"], [{"path": "one.rs"}])

    def test_injection_shared_resources_require_sequencing(self):
        self.create(self.order("one", resources=["gpu/window"]))
        addition = self.order("two", resources=["gpu/window"], reason="Visual check", verified_source=self.base)
        self.assert_unchanged_error(lambda: self.mutate("inject", payload=addition), "resource collision")
        addition["dispatch_blockers"] = ["one"]
        self.mutate("inject", payload=addition)

    def test_injection_can_reuse_integrated_territory(self):
        self.create()
        tree = self.worktree("one")
        self.mutate("start", worktree=tree)
        self.report(tree)
        self.integrate("one", self.base)
        addition = self.order("two", files=[{"path": "one.rs"}], reason="Follow-up correction", verified_source=self.base)
        self.mutate("inject", payload=addition)
        self.assertEqual(self.read()["orders"]["two"]["state"], "pending")

    def test_returned_dependency_change_is_rejected(self):
        self.create(self.order("one"), self.order("two", dispatch_blockers=["one"]))
        tree, consumer = self.worktree("one"), self.worktree("two")
        self.mutate("start", worktree=tree)
        self.report(tree)
        self.commit(tree, "one.rs", "changed after reporting")
        self.assert_unchanged_error(lambda: self.mutate("start", "two", worktree=consumer), "changed since its report")

    def test_slots_cap_five_and_human_ownership(self):
        orders = [self.order(f"worker{i}") for i in range(6)]
        human = self.order("human", owner={"kind": "human", "name": "Contributor"}, resources=["gpu"])
        self.create(*orders, human, self.order("visual", resources=["gpu"]))
        self.mutate("start", "human", worktree=self.root)
        for i in range(5):
            self.mutate("start", f"worker{i}", worktree=self.worktree(f"worker{i}"))
        view = workflow.queue_status("wave", self.root, self.config)
        self.assertEqual(view["active_workers"], 5)
        self.assert_unchanged_error(lambda: self.mutate("start", "worker5", worktree=self.worktree("worker5")), "worker limit")
        self.assertIn("shared resource held by human", view["orders"]["visual"]["waiting_reasons"])
        bad_config = copy.deepcopy(self.config)
        bad_config["dispatch"]["max_workers"] = 6
        with self.assertRaises(workflow.WorkflowError):
            workflow.queue_status("wave", self.root, bad_config)

    def test_human_pending_reservation_and_shared_agent_resources(self):
        human = self.order("human", owner={"kind": "human", "name": "Contributor"}, resources=["audio"])
        self.create(human, self.order("one", resources=["audio"]), self.order("gpu1", resources=["gpu"]), self.order("gpu2", resources=["gpu"]))
        self.assert_unchanged_error(lambda: self.mutate("start", worktree=self.worktree("one")), "held by human")
        self.mutate("start", "gpu1", worktree=self.worktree("gpu1"))
        self.assert_unchanged_error(lambda: self.mutate("start", "gpu2", worktree=self.worktree("gpu2")), "shared resource")
        self.mutate("block", "gpu1", payload={"reason": "Window closed", "checks_running": False})
        self.mutate("start", "gpu2", worktree=self.home / "gpu2")

    def test_start_and_resume_recheck_current_package_selection(self):
        plan = self.plan(self.order("one", packages=["gameskills-ui"]))
        plan["packages"] = ["gameskills", "gameskills-ui"]
        workflow.create_queue(plan, self.root, self.config)
        tree = self.worktree("one")
        changed = copy.deepcopy(self.config)
        changed["packages"] = ["gameskills"]
        self.assert_unchanged_error(lambda: self.mutate("start", worktree=tree, config=changed), "not selected")
        view = workflow.queue_status("wave", self.root, changed)
        self.assertEqual(view["active_workers"], 0)
        self.assertTrue(any("gameskills-ui" in reason for reason in view["orders"]["one"]["waiting_reasons"]))
        self.mutate("start", worktree=tree)
        self.mutate("block", payload={"reason": "Pause before continuing", "checks_running": False})
        self.assert_unchanged_error(lambda: self.mutate("resume", worktree=tree, config=changed), "not selected")
        self.assertEqual(len(self.read()["orders"]["one"]["attempts"]), 1)
        self.mutate("resume", worktree=tree)
        self.assertEqual(self.read()["orders"]["one"]["state"], "running")

    def test_reported_human_predecessor_allows_only_sequenced_consumers(self):
        human = self.order("human", owner={"kind": "human", "name": "Contributor"},
                           files=[{"path": "shared.rs"}], resources=["gpu"])
        consumer = self.order("after", files=human["files"], resources=["gpu"], dispatch_blockers=["human"])
        unrelated = self.order("unrelated", resources=["gpu"])
        self.create(human, consumer, unrelated)
        human_tree = self.worktree("human")
        consumer_tree = self.worktree("after")
        unrelated_tree = self.worktree("unrelated")
        self.assert_unchanged_error(lambda: self.mutate("start", "after", worktree=consumer_tree), "held by human")
        self.mutate("start", "human", worktree=human_tree)
        self.assert_unchanged_error(lambda: self.mutate("start", "after", worktree=consumer_tree), "held by human")
        head = self.commit(human_tree, "shared.rs", "human result")
        self.report(human_tree, "human")
        self.assert_unchanged_error(lambda: self.mutate("start", "unrelated", worktree=unrelated_tree), "held by human")
        self.assert_unchanged_error(lambda: self.mutate("start", "after", worktree=consumer_tree), "does not contain")
        self.git("merge", "--ff-only", head, cwd=consumer_tree)
        self.mutate("block", "human", payload={"reason": "Human reviewing follow-up", "checks_running": False})
        self.assert_unchanged_error(lambda: self.mutate("start", "after", worktree=consumer_tree), "held by human")
        self.mutate("resume", "human", worktree=human_tree)
        self.report(human_tree, "human")
        (human_tree / "shared.rs").write_text("unreported human edit")
        self.assert_unchanged_error(lambda: self.mutate("start", "after", worktree=consumer_tree), "changed since its report")
        self.git("restore", "shared.rs", cwd=human_tree)
        self.mutate("start", "after", worktree=consumer_tree)
        self.assertEqual(self.read()["orders"]["human"]["state"], "reported")
        self.assertEqual(self.read()["orders"]["after"]["state"], "running")

    def test_checkout_must_be_isolated_same_repository_and_unique(self):
        self.create(self.order("one"), self.order("two"))
        self.assert_unchanged_error(lambda: self.mutate("start", worktree=self.root), "isolated")
        unrelated = self.home / "clone"
        self.git("clone", "-q", str(self.root), str(unrelated))
        self.assert_unchanged_error(lambda: self.mutate("start", worktree=unrelated), "different repository")
        tree = self.worktree("one")
        self.mutate("start", worktree=tree)
        self.assert_unchanged_error(lambda: self.mutate("start", "two", worktree=tree), "already owned")
        self.mutate("block", payload={"reason": "Paused partial work", "checks_running": False})
        self.assert_unchanged_error(lambda: self.mutate("start", "two", worktree=tree), "already owned")

    def test_full_lifecycle_and_distinct_dependency_gates(self):
        self.create(self.order("one"), self.order("merge-only", merge_blockers=["one"]),
                    self.order("after", dispatch_blockers=["one"]))
        merge_tree = self.worktree("merge-only")
        self.mutate("start", "merge-only", worktree=merge_tree)
        self.report(merge_tree, "merge-only")
        self.assert_unchanged_error(lambda: self.mutate("integrated", "merge-only", payload={}), "merge dependency")
        after = self.worktree("after")
        self.assert_unchanged_error(lambda: self.mutate("start", "after", worktree=after), "dispatch dependency")
        tree = self.worktree("one")
        self.mutate("start", worktree=tree)
        head = self.commit(tree, "one.rs", "implemented")
        self.report(tree)
        self.assert_unchanged_error(lambda: self.mutate("integrated", payload={"summary": "claim", "head": self.base,
            "source_head": head, "evidence_reference": "checks"}), "not an ancestor")
        self.assert_unchanged_error(lambda: self.mutate("start", "after", worktree=after), "does not contain")
        self.git("merge", "--ff-only", head, cwd=after)
        self.mutate("start", "after", worktree=after)
        self.assertEqual(self.read()["orders"]["one"]["state"], "reported")
        self.assertEqual(self.git("rev-parse", "HEAD"), self.base)
        self.assert_unchanged_error(lambda: self.mutate("integrated", "merge-only", payload={}), "merge dependency")
        self.integrate("one", head)
        self.integrate("merge-only", self.base)
        self.assertEqual(self.read()["orders"]["one"]["state"], "integrated")
        self.assertEqual(self.read()["orders"]["after"]["state"], "running")

    def test_resume_rechecks_identity_and_preserves_old_evidence(self):
        self.create()
        tree = self.worktree("one")
        other = self.worktree("other")
        self.mutate("start", worktree=tree)
        self.report(tree)
        first_report = copy.deepcopy(self.read()["orders"]["one"]["reports"][0])
        self.mutate("block", payload={"reason": "Review fix needed", "checks_running": False})
        self.assert_unchanged_error(lambda: self.mutate("resume", worktree=other), "original worktree")
        self.mutate("resume", worktree=tree)
        changed_head = self.commit(tree, "one.rs", "changed")
        self.report(tree)
        reports = self.read()["orders"]["one"]["reports"]
        self.assertEqual(reports[0], first_report)
        self.assertEqual(reports[1]["identity"]["head"], changed_head)
        self.assertIn("not verified", reports[1]["evidence_status"])
        self.mutate("block", payload={"reason": "Another review finding", "checks_running": False})
        blocks = self.read()["orders"]["one"]["blocks"]
        self.assertEqual(len(blocks), 2)
        self.assertEqual(blocks[0]["observation"]["reason"], "Review fix needed")

    def test_invalid_transitions_and_incomplete_or_stale_reports(self):
        self.create()
        tree = self.worktree("one")
        self.assert_unchanged_error(lambda: self.report(tree), "running state")
        self.assert_unchanged_error(lambda: self.mutate("resume", worktree=tree), "blocked state")
        self.mutate("start", worktree=tree)
        self.assert_unchanged_error(lambda: self.mutate("block", payload={"reason": "Still testing", "checks_running": True}), "checks_running")
        self.assert_unchanged_error(lambda: self.report(tree, head="0" * 40), "head")
        self.assert_unchanged_error(lambda: self.report(tree, checks_running=True), "checks_running")
        (tree / "README.md").write_text("uncommitted")
        self.assert_unchanged_error(lambda: self.report(tree), "clean committed")
        self.git("restore", "README.md", cwd=tree)
        self.report(tree)
        changed = copy.deepcopy(self.config)
        changed["creative"]["default_level"] = 3
        payload = {"summary": "Integrated", "head": self.base, "source_head": self.base, "evidence_reference": "actual checks"}
        self.assert_unchanged_error(lambda: self.mutate("integrated", payload=payload, config=changed), "configuration changed")
        self.commit(tree, "one.rs", "post-report change")
        self.assert_unchanged_error(lambda: self.mutate("integrated", payload=payload), "source or configuration changed")

    def test_parallel_mutations_compare_and_swap_one_revision(self):
        self.create(self.order("one"), self.order("two"))
        one, two = self.worktree("one"), self.worktree("two")

        def attempt(name, tree):
            try:
                self.mutate("start", name, worktree=tree, revision=1)
                return "started"
            except workflow.WorkflowError as error:
                return str(error)

        with ThreadPoolExecutor(max_workers=2) as workers:
            outcomes = list(workers.map(lambda pair: attempt(*pair), [("one", one), ("two", two)]))
        self.assertEqual(outcomes.count("started"), 1)
        self.assertEqual(sum("stale revision" in value for value in outcomes), 1)
        queue = self.read()
        self.assertEqual(queue["revision"], 2)
        self.assertEqual(len(queue["events"]), 2)
        self.assertEqual(sum(entry["state"] == "running" for entry in queue["orders"].values()), 1)
        self.assertFalse(list(self.queue_path().parent.glob("*.tmp")))

    def test_cli_and_symlink_state_protection(self):
        path = self.home / "plan.json"
        path.write_text(json.dumps(self.plan()))
        result = workflow.main(["plan", "validate", "--file", str(path)], self.root, self.config)
        self.assertTrue(result["ok"])
        workflow.main(["queue", "create", "--file", str(path)], self.root, self.config)
        result = workflow.main(["queue", "status", "wave"], self.root, self.config)
        self.assertEqual(result["queue"]["revision"], 1)
        backup = self.home / "queue-backup.json"
        self.queue_path().rename(backup)
        self.queue_path().symlink_to(backup)
        with self.assertRaisesRegex(workflow.WorkflowError, "symlinks"):
            workflow.queue_status("wave", self.root, self.config)


if __name__ == "__main__":
    unittest.main()
