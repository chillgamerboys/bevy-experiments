"""Durable work orders and observed lifecycle state; never launches or merges work."""
from __future__ import annotations

import argparse
import contextlib
import copy
import datetime as dt
import fcntl
import hashlib
import json
import os
from pathlib import Path, PurePosixPath
import re
import subprocess
import tempfile


class WorkflowError(ValueError):
    """An invalid plan, stale observation, or unsafe state transition."""


_ID = re.compile(r"[A-Za-z0-9][A-Za-z0-9_-]{0,63}\Z")
_COMMIT = re.compile(r"[0-9a-f]{40}(?:[0-9a-f]{24})?\Z")
_LEVELS = {1, 2, 3, 4}
_TARGETS = {"design", "implementation", "pr", "merge", "release"}
_STATES = {"pending", "running", "blocked", "reported", "integrated"}


def _fail(message):
    raise WorkflowError(message)


def _text(value, label):
    if not isinstance(value, str) or not value.strip() or any(ord(c) < 32 for c in value):
        _fail(f"{label} must be a nonempty string without control characters")
    return value


def _id(value, label="id"):
    if not isinstance(value, str) or not _ID.fullmatch(value):
        _fail(f"{label} must be 1-64 letters, digits, underscores or hyphens, starting alphanumeric")
    return value


def _mapping(value, label):
    if not isinstance(value, dict):
        _fail(f"{label} must be an object")
    return value


def _strings(value, label):
    if not isinstance(value, list):
        _fail(f"{label} must be a list")
    for item in value:
        _text(item, label)
    if len(value) != len(set(value)):
        _fail(f"{label} contains duplicates")
    return value


def _digest(value):
    return hashlib.sha256(json.dumps(value, sort_keys=True, separators=(",", ":")).encode()).hexdigest()


def _now():
    return dt.datetime.now(dt.timezone.utc).isoformat()


def _git(root, *args, check=True):
    result = subprocess.run(["git", "-C", str(root), *args], text=True, capture_output=True)
    if check and result.returncode:
        _fail(f"git {' '.join(args)} failed: {result.stderr.strip()}")
    return result.stdout.strip() if check else result


def repository_identity(root: Path) -> dict:
    """Observe canonical checkout, common repository and current commit using git."""
    root = Path(root).resolve()
    top = Path(_git(root, "rev-parse", "--show-toplevel")).resolve()
    if top != root:
        _fail(f"expected checkout root {root}, found {top}")
    common = Path(_git(root, "rev-parse", "--git-common-dir"))
    if not common.is_absolute():
        common = root / common
    return {"root": str(root), "common_dir": str(common.resolve()), "head": _git(root, "rev-parse", "HEAD")}


def _commit(root, value, label):
    if not isinstance(value, str) or not _COMMIT.fullmatch(value):
        _fail(f"{label} must be a full lowercase commit object id")
    if _git(root, "rev-parse", "--verify", value + "^{commit}") != value:
        _fail(f"{label} is not a commit")
    return value


def _ancestor(root, ancestor, descendant):
    result = _git(root, "merge-base", "--is-ancestor", ancestor, descendant, check=False)
    if result.returncode not in (0, 1):
        _fail(f"cannot verify commit ancestry: {result.stderr.strip()}")
    return result.returncode == 0


def _clean(root):
    if _git(root, "diff", "--quiet", "HEAD", "--", check=False).returncode:
        return False
    untracked = _git(root, "ls-files", "--others", "--exclude-standard", "-z")
    return all(p == ".gameskills" or p.startswith(".gameskills/") for p in untracked.split("\0") if p)


def _configuration(config):
    dispatch = _mapping(config.get("dispatch", {}), "dispatch configuration")
    maximum = dispatch.get("max_workers", 5)
    if type(maximum) is not int or not 1 <= maximum <= 5:
        _fail("dispatch.max_workers must be an integer from 1 through 5")
    if type(dispatch.get("enabled", False)) is not bool:
        _fail("dispatch.enabled must be a boolean")
    return maximum


def _path(value):
    _text(value, "ownership path")
    path = PurePosixPath(value)
    if (path.is_absolute() or "\\" in value or any(part in ("", ".", "..") for part in value.rstrip("/").split("/"))
            or any(char in value for char in "*?[]") or value.rstrip("/") in {".gameskills", ".git"}
            or value.startswith((".gameskills/", ".git/"))):
        _fail("ownership paths must be literal, repository-relative paths without dot segments or glob characters")
    return value


def validate_order(order: dict, plan: dict, config: dict) -> dict:
    """Validate and copy an immutable work-order specification, without changing state."""
    order = copy.deepcopy(_mapping(order, "order"))
    _id(order.get("id"), "order id")
    for key in ("goal", "artifact", "target", "creative_scope"):
        _text(order.get(key), f"order {key}")
    level = order.setdefault("creative_level", plan["creative_level"])
    if type(level) is not int or level not in _LEVELS:
        _fail("creative_level must be an integer from 1 through 4")
    owner = _mapping(order.get("owner"), "owner")
    if owner.get("kind") not in {"agent", "human"}:
        _fail("owner.kind must be agent or human")
    _text(owner.get("name"), "owner.name")
    role = _mapping(order.get("execution"), "execution")
    _text(role.get("role"), "execution.role")
    _text(role.get("effort"), "execution.effort")
    for key in ("decisions", "investigation", "references", "acceptance", "expected_evidence", "coordination", "resources"):
        _strings(order.get(key), f"order {key}")
    if not order["acceptance"] or not order["expected_evidence"]:
        _fail("orders need acceptance checks and expected evidence")
    packages = _strings(order.setdefault("packages", plan["packages"][:]), "order packages")
    if not set(packages).issubset(plan["packages"]) or not set(packages).issubset(config.get("packages", [])):
        _fail("order packages must be selected by the plan and current project configuration")
    files = order.get("files")
    if not isinstance(files, list):
        _fail("order files must be a list")
    for entry in files:
        _mapping(entry, "file ownership")
        path = _path(entry.get("path"))
        checkout_root = Path(plan["repository"]["root"])
        candidate = checkout_root / path
        if candidate.resolve() != candidate.absolute():
            _fail("ownership paths may not pass through symbolic links")
        if set(entry) - {"path", "lines"}:
            _fail("file ownership supports only path and optional lines")
        if "lines" in entry:
            lines = entry["lines"]
            if (path.endswith("/") or not isinstance(lines, list) or len(lines) != 2
                    or any(type(n) is not int for n in lines) or not 1 <= lines[0] <= lines[1]):
                _fail("file lines must be an inclusive [start, end] range on a file")
    for key in ("dispatch_blockers", "merge_blockers"):
        for dependency in _strings(order.get(key), key):
            _id(dependency, key)
            if dependency == order["id"]:
                _fail("an order cannot depend on itself")
    if "reason" in order:
        _text(order["reason"], "injection reason")
    return order


def _file_conflict(left, right):
    for a in left["files"]:
        for b in right["files"]:
            ap, bp = a["path"].rstrip("/"), b["path"].rstrip("/")
            if ap == bp:
                if "lines" not in a or "lines" not in b or max(a["lines"][0], b["lines"][0]) <= min(a["lines"][1], b["lines"][1]):
                    return True
            elif (a["path"].endswith("/") and bp.startswith(ap + "/")) or (b["path"].endswith("/") and ap.startswith(bp + "/")):
                return True
    return False


def _resource_conflict(left, right):
    return bool(set(left["resources"]) & set(right["resources"]))


def _depends(orders, child, ancestor, visiting=None):
    visiting = set() if visiting is None else visiting
    if child in visiting:
        return False
    visiting.add(child)
    return any(dep == ancestor or _depends(orders, dep, ancestor, visiting)
               for dep in orders[child]["dispatch_blockers"])


def _validate_graph(orders, ownership_ids=None):
    visiting, visited = set(), set()

    def visit(name):
        if name in visiting:
            _fail("dependency cycle detected")
        if name in visited:
            return
        visiting.add(name)
        for dependency in orders[name]["dispatch_blockers"] + orders[name]["merge_blockers"]:
            if dependency not in orders:
                _fail(f"unknown dependency {dependency} in {name}")
            visit(dependency)
        visiting.remove(name)
        visited.add(name)

    for name in orders:
        visit(name)
    values = [order for name, order in orders.items() if ownership_ids is None or name in ownership_ids]
    for index, left in enumerate(values):
        for right in values[index + 1:]:
            if (_file_conflict(left, right) and not _depends(orders, left["id"], right["id"])
                    and not _depends(orders, right["id"], left["id"])):
                _fail(f"file ownership collision between {left['id']} and {right['id']}; use a dispatch dependency")


def validate_plan(plan: dict, root: Path, config: dict) -> dict:
    """Validate a version-one plan against current repository and configured packages."""
    _configuration(config)
    plan = copy.deepcopy(_mapping(plan, "plan"))
    if type(plan.get("schema_version")) is not int or plan["schema_version"] != 1:
        _fail("plan schema_version must be 1")
    _id(plan.get("id"), "plan id")
    _text(plan.get("goal"), "plan goal")
    if plan.get("delivery_target") not in _TARGETS:
        _fail("delivery_target must be design, implementation, pr, merge or release")
    level = plan.setdefault("creative_level", config.get("creative", {}).get("default_level", 2))
    if type(level) is not int or level not in _LEVELS:
        _fail("creative_level must be an integer from 1 through 4")
    for key in ("decisions", "packages"):
        _strings(plan.get(key), f"plan {key}")
    if not set(plan["packages"]).issubset(config.get("packages", [])):
        _fail("plan packages must be selected in project configuration")
    versions = _mapping(plan.get("versions"), "versions")
    _text(versions.get("bevy"), "versions.bevy")
    if versions.get("gamekit") is not None:
        _text(versions["gamekit"], "versions.gamekit")
    repo = _mapping(plan.get("repository"), "repository")
    actual = repository_identity(root)
    if not isinstance(repo.get("root"), str) or Path(repo["root"]).resolve() != Path(actual["root"]):
        _fail("plan repository.root does not match the configured checkout")
    repo["root"] = actual["root"]
    if repo.get("common_dir", actual["common_dir"]) != actual["common_dir"]:
        _fail("plan repository.common_dir does not match git")
    repo["common_dir"] = actual["common_dir"]
    _commit(root, repo.get("base_commit"), "repository.base_commit")
    _commit(root, repo.get("source_commit"), "repository.source_commit")
    if repo["source_commit"] != actual["head"]:
        _fail("plan source_commit is stale relative to the checkout HEAD")
    if not _ancestor(root, repo["base_commit"], repo["source_commit"]):
        _fail("plan source_commit does not descend from base_commit")
    if not isinstance(plan.get("orders"), list) or not plan["orders"]:
        _fail("plan orders must be a nonempty list")
    orders = [validate_order(order, plan, config) for order in plan["orders"]]
    if len({order["id"] for order in orders}) != len(orders):
        _fail("duplicate order ids")
    _validate_graph({order["id"]: order for order in orders})
    plan["orders"] = orders
    return plan


@contextlib.contextmanager
def _locked(root, create=False):
    directory = Path(root) / ".gameskills" / "queues"
    for path in (directory.parent, directory):
        if path.is_symlink():
            _fail(f"state directories may not be symlinks: {path}")
        if create:
            path.mkdir(exist_ok=True)
        elif not path.is_dir():
            _fail("queue state does not exist")
    flags = os.O_RDWR | os.O_CREAT | getattr(os, "O_NOFOLLOW", 0)
    try:
        descriptor = os.open(directory / ".lock", flags, 0o600)
    except OSError as exc:
        _fail(f"cannot lock queue state: {exc}")
    try:
        fcntl.flock(descriptor, fcntl.LOCK_EX)
        yield directory
    finally:
        fcntl.flock(descriptor, fcntl.LOCK_UN)
        os.close(descriptor)


def _read(path):
    if path.is_symlink():
        _fail("queue and input files may not be symlinks")
    try:
        return json.loads(path.read_text(encoding="utf-8"))
    except (OSError, ValueError) as exc:
        _fail(f"cannot read JSON {path}: {exc}")


def _write(path, value):
    if path.is_symlink():
        _fail("queue files may not be symlinks")
    descriptor, temporary = tempfile.mkstemp(prefix=".queue-", suffix=".tmp", dir=path.parent)
    try:
        with os.fdopen(descriptor, "w", encoding="utf-8") as stream:
            json.dump(value, stream, indent=2, sort_keys=True)
            stream.write("\n")
            stream.flush()
            os.fsync(stream.fileno())
        os.replace(temporary, path)
        directory_fd = os.open(path.parent, os.O_RDONLY)
        try:
            os.fsync(directory_fd)
        finally:
            os.close(directory_fd)
    finally:
        if os.path.exists(temporary):
            os.unlink(temporary)


def _load(directory, queue_id, root):
    queue = _mapping(_read(directory / f"{_id(queue_id, 'queue id')}.json"), "queue")
    if queue.get("schema_version") != 1 or queue.get("id") != queue_id:
        _fail("invalid queue identity or schema")
    repo = queue["plan"]["repository"]
    actual = repository_identity(root)
    if repo["root"] != actual["root"] or repo["common_dir"] != actual["common_dir"]:
        _fail("queue belongs to a different repository or checkout")
    return queue


def _event(queue, action, order_id=None, **details):
    queue["events"].append({"revision": queue["revision"], "at": _now(), "action": action,
                            "order_id": order_id, **details})


def create_queue(plan, root, config):
    plan = validate_plan(plan, root, config)
    with _locked(root, create=True) as directory:
        path = directory / f"{plan['id']}.json"
        if path.exists() or path.is_symlink():
            _fail("queue already exists; injection is add-only")
        queue = {"schema_version": 1, "id": plan["id"], "revision": 1, "plan": plan,
                 "config_digest": _digest(config), "events": [],
                 "orders": {order["id"]: {"spec": order, "state": "pending", "attempts": [], "reports": [], "blocks": []}
                            for order in plan["orders"]}}
        _event(queue, "create")
        _write(path, queue)
        return queue


def _specs(queue):
    return {name: entry["spec"] for name, entry in queue["orders"].items()}


def _blockers(queue, order_id, config):
    entry = queue["orders"][order_id]
    spec = entry["spec"]
    reasons = []
    for dep in spec["dispatch_blockers"]:
        if queue["orders"][dep]["state"] not in {"reported", "integrated"}:
            reasons.append(f"dispatch dependency {dep} has no returned work")
    if spec["owner"]["kind"] == "agent":
        if not config.get("dispatch", {}).get("enabled", False):
            reasons.append("dispatch is disabled in project configuration")
        active = sum(item["state"] == "running" and item["spec"]["owner"]["kind"] == "agent"
                     for item in queue["orders"].values())
        if active >= _configuration(config):
            reasons.append("worker limit reached")
    for name, other in queue["orders"].items():
        if name == order_id or other["state"] == "integrated":
            continue
        if other["state"] == "running" or other["spec"]["owner"]["kind"] == "human":
            # A future human stream explicitly sequenced after this one has not reserved its resources yet.
            if other["state"] == "pending" and _depends(_specs(queue), name, order_id):
                continue
            if _file_conflict(spec, other["spec"]):
                reasons.append(f"file ownership held by {name}")
            if _resource_conflict(spec, other["spec"]):
                reasons.append(f"shared resource held by {name}")
    return reasons


def queue_status(queue_id, root, config):
    _configuration(config)
    with _locked(root) as directory:
        queue = _load(directory, queue_id, root)
        view = copy.deepcopy(queue)
        view["config_changed"] = queue["config_digest"] != _digest(config)
        view["active_workers"] = sum(entry["state"] == "running" and entry["spec"]["owner"]["kind"] == "agent"
                                      for entry in queue["orders"].values())
        for name, entry in view["orders"].items():
            entry["waiting_reasons"] = _blockers(queue, name, config) if entry["state"] in {"pending", "blocked"} else []
            entry["merge_waiting_reasons"] = [f"merge dependency {dep} is not integrated"
                                              for dep in entry["spec"]["merge_blockers"]
                                              if queue["orders"][dep]["state"] != "integrated"]
        return view


def _checkout(queue, entry, worktree, root):
    actual = repository_identity(Path(worktree))
    repo = queue["plan"]["repository"]
    if actual["common_dir"] != repo["common_dir"]:
        _fail("worker worktree belongs to a different repository")
    if entry["spec"]["owner"]["kind"] == "agent" and actual["root"] == str(Path(root).resolve()):
        _fail("agent workers require a worktree isolated from the coordinator checkout")
    if not _ancestor(root, repo["base_commit"], actual["head"]) or not _ancestor(root, repo["source_commit"], actual["head"]):
        _fail("worktree HEAD does not descend from the planned source and base")
    for dep in entry["spec"]["dispatch_blockers"]:
        prerequisite = queue["orders"][dep]
        if not prerequisite["reports"]:
            _fail(f"dispatch dependency {dep} has no returned source")
        returned = prerequisite["reports"][-1]["identity"]
        if prerequisite["state"] == "reported":
            current = repository_identity(Path(returned["root"]))
            if (any(current[key] != returned[key] for key in ("root", "common_dir", "head"))
                    or not _clean(Path(returned["root"]))):
                _fail(f"returned dispatch dependency {dep} changed since its report")
        if not _ancestor(root, returned["head"], actual["head"]):
            _fail(f"worktree does not contain returned dispatch dependency {dep}")
    for other in queue["orders"].values():
        if other is not entry and other["state"] != "integrated" and other.get("checkout", {}).get("root") == actual["root"]:
            _fail("worktree is already owned by an unfinished stream")
    return {**actual, "base_commit": repo["base_commit"], "source_commit": repo["source_commit"]}


def _observation(queue, entry, observation, root):
    observation = copy.deepcopy(_mapping(observation, "observation"))
    _text(observation.get("summary"), "observation.summary")
    if observation.get("checks_running") is not False:
        _fail("observation must explicitly record checks_running: false")
    _text(observation.get("worktree"), "observation.worktree")
    checkout = _checkout(queue, entry, observation["worktree"], root)
    if not entry.get("checkout") or checkout["root"] != entry["checkout"]["root"]:
        _fail("observation worktree differs from the started worktree")
    for key in ("head", "base_commit"):
        if observation.get(key) != checkout[key]:
            _fail(f"observation {key} does not match the verified checkout identity")
    if not _clean(Path(checkout["root"])):
        _fail("report requires a clean committed worktree")
    evidence = observation.get("evidence")
    if not isinstance(evidence, list) or not evidence:
        _fail("observation needs evidence references")
    for item in evidence:
        _mapping(item, "evidence reference")
        for key in ("kind", "reference", "summary"):
            _text(item.get(key), f"evidence.{key}")
    return {"at": _now(), "identity": checkout, "observation": observation,
            "evidence_status": "caller-supplied references; execution and acceptance not verified"}


def mutate_queue(queue_id, action, root, config, expected_revision, *, order_id=None, payload=None, worktree=None):
    """Compare-and-swap one transition under an advisory lock; failures never write the queue."""
    _configuration(config)
    if type(expected_revision) is not int or expected_revision < 1:
        _fail("expected_revision must be a positive integer")
    with _locked(root) as directory:
        queue = _load(directory, queue_id, root)
        if queue["revision"] != expected_revision:
            _fail(f"stale revision: expected {expected_revision}, current {queue['revision']}")
        if action == "inject":
            spec = validate_order(payload, queue["plan"], config)
            _text(spec.get("reason"), "injection reason")
            if not spec["investigation"]:
                _fail("injection needs current investigation evidence")
            actual = repository_identity(root)
            if spec.get("verified_source") != actual["head"]:
                _fail("injection verified_source must match the current coordinator HEAD")
            if not _ancestor(root, queue["plan"]["repository"]["source_commit"], actual["head"]):
                _fail("injection checkout does not descend from the planned source")
            if spec["id"] in queue["orders"]:
                _fail("injection is add-only; order already exists")
            specs = {**_specs(queue), spec["id"]: spec}
            live = {name for name, other in queue["orders"].items() if other["state"] != "integrated"}
            _validate_graph(specs, live | {spec["id"]})
            for name, other in queue["orders"].items():
                if other["state"] != "integrated" and _resource_conflict(spec, other["spec"]) and not _depends(specs, spec["id"], name):
                    _fail(f"injected shared resource collision with {name}; add a dispatch dependency")
            queue["orders"][spec["id"]] = {"spec": spec, "state": "pending", "attempts": [], "reports": [], "blocks": []}
            order_id = spec["id"]
        else:
            _id(order_id, "order id")
            if order_id not in queue["orders"]:
                _fail("unknown order")
            entry = queue["orders"][order_id]
            state = entry["state"]
            if action in {"start", "resume"}:
                required = "pending" if action == "start" else "blocked"
                if state != required:
                    _fail(f"{action} requires {required} state, found {state}")
                reasons = _blockers(queue, order_id, config)
                if reasons:
                    _fail("; ".join(reasons))
                if not worktree:
                    _fail("start and resume require an actual worktree")
                checkout = _checkout(queue, entry, worktree, root)
                if action == "resume" and entry.get("checkout", {}).get("root", checkout["root"]) != checkout["root"]:
                    _fail("resume must use the original worktree; replan a relocation")
                entry["checkout"] = checkout
                entry["attempts"].append({"at": _now(), "identity": checkout, "config_digest": _digest(config)})
                entry["state"] = "running"
            elif action == "block":
                if state not in {"pending", "running", "reported"}:
                    _fail(f"cannot block an order in {state} state")
                payload = _mapping(payload, "block observation")
                _text(payload.get("reason"), "block reason")
                if payload.get("checks_running") is not False:
                    _fail("block requires checks_running: false; collect or stop actual checks first")
                block = {"at": _now(), "observation": copy.deepcopy(payload),
                         "revision": expected_revision + 1, "config_digest": _digest(config),
                         "last_observed_checkout": entry.get("checkout"),
                         "status": "caller reports no running checks"}
                entry["blocks"].append(block)
                entry["state"] = "blocked"
            elif action == "report":
                if state != "running":
                    _fail(f"report requires running state, found {state}")
                report = _observation(queue, entry, payload, root)
                report["config_digest"] = _digest(config)
                report["revision"] = expected_revision + 1
                entry["reports"].append(report)
                entry["state"] = "reported"
            elif action == "integrated":
                if state != "reported":
                    _fail(f"integrated requires reported state, found {state}")
                for dep in entry["spec"]["merge_blockers"]:
                    if queue["orders"][dep]["state"] != "integrated":
                        _fail(f"merge dependency {dep} is not integrated")
                payload = _mapping(payload, "integration observation")
                _text(payload.get("summary"), "integration summary")
                _text(payload.get("evidence_reference"), "integration evidence_reference")
                actual = repository_identity(root)
                report = entry["reports"][-1]
                if payload.get("head") != actual["head"] or payload.get("source_head") != report["identity"]["head"]:
                    _fail("integration observation is stale or names a different source")
                current = repository_identity(Path(report["identity"]["root"]))
                if (any(current[key] != report["identity"][key] for key in ("root", "common_dir", "head"))
                        or report["config_digest"] != _digest(config)):
                    _fail("reported source or configuration changed; collect a new report")
                if not _ancestor(root, report["identity"]["head"], actual["head"]):
                    _fail("reported source is not an ancestor of the actual integration HEAD")
                if not _clean(Path(root)) or not _clean(Path(current["root"])):
                    _fail("integration observation requires clean committed source and coordinator checkouts")
                entry["integration"] = {**actual, "at": _now(), "observation": copy.deepcopy(payload),
                                        "status": "git ancestry observed; review and combined checks are caller-supplied"}
                entry["state"] = "integrated"
            else:
                _fail(f"unknown queue action {action}")
        queue["revision"] += 1
        _event(queue, action, order_id, config_digest=_digest(config))
        _write(directory / f"{queue_id}.json", queue)
        return queue


def parser():
    result = argparse.ArgumentParser(prog="gameskills workflow")
    groups = result.add_subparsers(dest="group", required=True)
    plan = groups.add_parser("plan").add_subparsers(dest="action", required=True)
    plan.add_parser("validate").add_argument("--file", required=True, type=Path)
    queue = groups.add_parser("queue").add_subparsers(dest="action", required=True)
    queue.add_parser("create").add_argument("--file", required=True, type=Path)
    queue.add_parser("status").add_argument("queue_id")
    for action in ("inject", "start", "resume", "report", "block", "integrated"):
        command = queue.add_parser(action)
        command.add_argument("queue_id")
        if action != "inject":
            command.add_argument("order_id")
        command.add_argument("--expected-revision", type=int, required=True)
        if action in {"start", "resume"}:
            command.add_argument("--worktree", type=Path, required=True)
        else:
            command.add_argument("--file", type=Path, required=True)
    return result


def main(argv, root: Path, config: dict) -> dict:
    args = parser().parse_args(argv)
    if args.group == "plan":
        return {"ok": True, "plan": validate_plan(_read(args.file), root, config)}
    if args.action == "create":
        return {"ok": True, "queue": create_queue(_read(args.file), root, config)}
    if args.action == "status":
        return {"ok": True, "queue": queue_status(args.queue_id, root, config)}
    return {"ok": True, "queue": mutate_queue(args.queue_id, args.action, root, config, args.expected_revision,
                                              order_id=getattr(args, "order_id", None),
                                              payload=_read(args.file) if hasattr(args, "file") else None,
                                              worktree=getattr(args, "worktree", None))}
