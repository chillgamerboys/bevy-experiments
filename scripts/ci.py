#!/usr/bin/env python3
"""Select CI from committed inputs, run selected checks, and audit job results.

The selector reads Git objects and TOML without Cargo, dependencies or a network.
Uncertain ownership selects the full suite; a broken selector fails the final gate.
"""

from __future__ import annotations

import argparse
import fnmatch
import json
import os
from pathlib import Path, PurePosixPath
import posixpath
import re
import subprocess
import sys
import tomllib

JOBS = ("skills", "rust", "policy")
FLAGS = (*JOBS, "distribution", "minimal", "wasm", "deny")
WASM = {"bevy_game_hex", "bevy_game_turns", "bevy_game_session", "bevy_game_ui", "labyrinth_rules"}
FULL_INPUTS = {"Cargo.toml", "Cargo.lock", "deny.toml", "rust-toolchain", "rust-toolchain.toml"}
SKILL_INPUTS = {"gameskills.toml", "gameskills.lock.json"}
PROSE_ROOTS = {"README.md", "LICENSE", "LICENSE-MIT", "LICENSE-APACHE", "CHANGELOG.md"}


def git(root: Path, *args: str) -> str:
    return subprocess.run(["git", *args], cwd=root, check=True, capture_output=True,
                          text=True, encoding="utf-8").stdout


def commit(root: Path, revision: str) -> str:
    if not re.fullmatch(r"[0-9a-f]{40}|[0-9a-f]{64}", revision or ""):
        raise ValueError("expected a full commit object ID")
    return git(root, "rev-parse", "--verify", f"{revision}^{{commit}}").strip()


def files_at(root: Path, revision: str) -> list[str]:
    return git(root, "ls-tree", "-r", "--name-only", "-z", revision).rstrip("\0").split("\0")


def read_at(root: Path, revision: str, path: str) -> str:
    return git(root, "show", f"{revision}:{path}")


def changed_paths(root: Path, base: str, head: str) -> list[str]:
    fields = git(root, "diff", "--name-status", "--find-renames", "-z", base, head).split("\0")
    paths = set()
    while fields and fields[0]:
        status = fields.pop(0)
        count = 2 if status.startswith(("R", "C")) else 1
        for _ in range(count):
            path = fields.pop(0)
            if not path or "\n" in path or "\r" in path:
                raise ValueError("unsupported changed path")
            paths.add(path)
    return sorted(paths)


def matches(path: str, pattern: str) -> bool:
    return (len(PurePosixPath(path).parts) == len(PurePosixPath(pattern).parts)
            and all(fnmatch.fnmatchcase(part, glob) for part, glob in
                    zip(PurePosixPath(path).parts, PurePosixPath(pattern).parts)))


def dependencies(manifest: dict):
    for key in ("dependencies", "dev-dependencies", "build-dependencies"):
        yield from manifest.get(key, {}).items()
    for target in manifest.get("target", {}).values():
        yield from dependencies(target)


def workspace(root: Path, revision: str) -> tuple[dict, dict, dict]:
    paths = files_at(root, revision)
    manifest = tomllib.loads(read_at(root, revision, "Cargo.toml"))
    settings = manifest["workspace"]
    members = settings["members"]
    excluded = settings.get("exclude", [])
    packages, manifests = {}, {}
    for path in paths:
        parent = str(PurePosixPath(path).parent)
        if (path.endswith("/Cargo.toml") and any(matches(parent, item) for item in members)
                and not any(matches(parent, item) for item in excluded)):
            value = tomllib.loads(read_at(root, revision, path))
            name = value["package"]["name"]
            if not re.fullmatch(r"[A-Za-z0-9_][A-Za-z0-9_-]*", name) or name in packages:
                raise ValueError("invalid or duplicate workspace package name")
            packages[name], manifests[name] = parent, value
    if not packages:
        raise ValueError("no recognized workspace packages")
    by_path = {path: name for name, path in packages.items()}
    consumers = {name: set() for name in packages}
    for name, value in manifests.items():
        for alias, dependency in dependencies(value):
            if not isinstance(dependency, dict):
                continue
            parent = packages[name]
            if dependency.get("workspace"):
                dependency = settings.get("dependencies", {})[alias]
                parent = "."
            if isinstance(dependency, dict) and "path" in dependency:
                path = posixpath.normpath(posixpath.join(parent, dependency["path"]))
                if path not in by_path:
                    raise ValueError(f"local dependency outside recognized workspace: {path}")
                consumers[by_path[path]].add(name)
    # Explicit source inclusions can make docs/assets inputs to another package.
    included = {}
    try:
        candidates = git(root, "grep", "-l", "-z", "-e", "include", revision, "--", "*.rs")
    except subprocess.CalledProcessError as error:
        if error.returncode != 1:
            raise
        candidates = ""
    for candidate in candidates.rstrip("\0").split("\0"):
        if not candidate:
            continue
        path = candidate.split(":", 1)[1]
        owner = owner_of(path, packages)
        source = read_at(root, revision, path)
        expressions = re.findall(r"\binclude(?:_str|_bytes)?!\s*[({\[](.*?)[)}\]]", source, re.S)
        if len(expressions) != len(re.findall(r"\binclude(?:_str|_bytes)?!", source)):
            included.setdefault("*", set()).add(owner or "unknown")
        for expression in expressions:
            literal = re.fullmatch(r'\s*"([^"\\]+)"\s*,?\s*', expression)
            if literal and owner:
                target = posixpath.normpath(posixpath.join(str(PurePosixPath(path).parent), literal[1]))
                included.setdefault(target, set()).add(owner)
            else:
                included.setdefault("*", set()).add(owner or "unknown")
    if (any(p.endswith("/build.rs") for p in paths)
            or any(value["package"].get("build") not in (None, False)
                   for value in manifests.values())):
        included.setdefault("*", set()).add("build-script")
    return packages, consumers, included


def owner_of(path: str, packages: dict) -> str | None:
    matches_ = [(len(parent), name) for name, parent in packages.items()
                if path.startswith(parent + "/")]
    return max(matches_)[1] if matches_ else None


def expand(names: set[str], consumers: dict) -> set[str]:
    result = set(names)
    pending = list(names)
    while pending:
        for name in consumers.get(pending.pop(), set()) - result:
            result.add(name)
            pending.append(name)
    return result


def full_selection(head: str, base: str | None, reason: str) -> dict:
    return {"schema_version": 1, "head": head, "base": base, "full": True,
            "packages": [], "paths": [], "reasons": [reason], **dict.fromkeys(FLAGS, True)}


def select(root: Path, base: str | None, head: str, force_full: bool = False) -> dict:
    head = commit(root, head)
    if force_full:
        return full_selection(head, base, "Manual full-suite run")
    try:
        base = commit(root, base or "")
        paths = changed_paths(root, base, head)
        old, old_consumers, old_included = workspace(root, base)
        new, new_consumers, new_included = workspace(root, head)
        if paths and ("*" in old_included or "*" in new_included):
            raise ValueError("dynamic or unsupported include/build-script inputs cannot be scoped safely")
        result = {"schema_version": 1, "head": head, "base": base, "full": False,
                  "packages": [], "paths": paths, "reasons": [], **dict.fromkeys(FLAGS, False)}
        affected = set()
        for path in paths:
            owner = owner_of(path, new) or owner_of(path, old)
            if (path in FULL_INPUTS or path.endswith("/Cargo.toml")
                    or path.startswith((".github/", ".cargo/", "scripts/ci", "scripts/tests/test_ci"))):
                raise ValueError(f"shared configuration or CI input: {path}")
            included = old_included.get(path, set()) | new_included.get(path, set())
            if included:
                affected.update(included)
                result["reasons"].append(f"{path}: compiled input to {', '.join(sorted(included))}")
            if path.startswith(("plugins/", "skills/", ".claude-plugin/", ".codex-plugin/", ".agents/")) or path in SKILL_INPUTS:
                result["skills"] = True
                result["reasons"].append(f"{path}: GameSkills instructions, packaging or runtime")
            elif path.endswith(".md") or path in PROSE_ROOTS:
                if included:
                    if owner:
                        affected.add(owner)
                elif owner or path.startswith("docs/") or path in PROSE_ROOTS:
                    result["reasons"].append(f"{path}: narrative documentation")
                else:
                    raise ValueError(f"unmapped Markdown: {path}")
            elif owner:
                affected.add(owner)
                result["reasons"].append(f"{path}: package {owner}")
            elif path in {"scripts/check_distribution.py", "scripts/fixtures/gamekit_consumer.rs", "scripts/tests/test_check_distribution.py"}:
                result.update(rust=True, policy=True, distribution=True)
                affected.update(name for name, folder in new.items() if folder.startswith("crates/"))
                result["reasons"].append(f"{path}: external library distribution contract")
            elif path in {"scripts/check_repo.py", "scripts/tests/test_check_repo.py"}:
                result["skills"] = True
                result["reasons"].append(f"{path}: always-run repository checks")
            elif not included:
                raise ValueError(f"unmapped input: {path}")
        # Reach a fixed point over both graphs, including changed/deleted edges.
        previous = None
        while previous != affected:
            previous = set(affected)
            affected = expand(expand(affected, old_consumers), new_consumers)
        if affected - new.keys():
            raise ValueError("changed or removed package graph")
        result["packages"] = sorted(affected)
        if affected:
            result.update(rust=True, policy=True)
        if any(new[name].startswith("crates/") for name in affected):
            result.update(distribution=True, minimal=True)
        result["wasm"] = bool(affected & WASM)
        return result
    except (ValueError, KeyError, IndexError, TypeError, subprocess.CalledProcessError, UnicodeError) as error:
        return full_selection(head, base, f"Conservative fallback: {error}")


def validate(selection: dict) -> None:
    if (selection.get("schema_version") != 1 or type(selection.get("full")) is not bool
            or any(type(selection.get(flag)) is not bool for flag in FLAGS)
            or not isinstance(selection.get("packages"), list)
            or any(not isinstance(name, str) or not re.fullmatch(r"[A-Za-z0-9_][A-Za-z0-9_-]*", name)
                   for name in selection["packages"])
            or not re.fullmatch(r"[0-9a-f]{40}|[0-9a-f]{64}", selection.get("head", ""))):
        raise ValueError("invalid CI selection")
    if selection["full"] and not all(selection[flag] for flag in FLAGS):
        raise ValueError("full selection omitted checks")
    if selection["rust"] and not selection["full"] and not selection["packages"]:
        raise ValueError("selected Rust checks without packages")


def gate(selection: dict, needs: dict) -> None:
    validate(selection)
    for job in ("classify", *JOBS):
        result = needs.get(job, {}).get("result")
        selected = job == "classify" or selection[job]
        if result != "success" and (selected or result != "skipped"):
            raise ValueError(f"{job}: {'selected' if selected else 'unselected'} job returned {result!r}")


def run_checks(root: Path, kind: str, selection: dict) -> None:
    validate(selection)
    if git(root, "rev-parse", "HEAD").strip() != selection["head"]:
        raise ValueError("selection does not match tested checkout")
    if not selection[kind]:
        raise ValueError(f"attempted to execute unselected job {kind}")

    def run(*argv: str):
        print("+", " ".join(argv), flush=True)
        subprocess.run(argv, cwd=root, check=True)

    packages = ["--workspace"] if selection["full"] else [arg for name in selection["packages"] for arg in ("-p", name)]
    if kind == "skills":
        run(sys.executable, "-m", "unittest", "discover", "-s", "scripts/tests", "-v")
        run(sys.executable, "skills/scripts/validate_skills.py")
        run(sys.executable, "skills/scripts/validate_gameskills.py")
        run(sys.executable, "-m", "unittest", "discover", "-s", "skills/tests", "-v")
    elif kind == "rust":
        if selection["distribution"]:
            run(sys.executable, "scripts/check_distribution.py")
        run("cargo", "test", *packages, "--all-features", "--profile", "ci")
        if selection["full"] or "labyrinth" in selection["packages"]:
            run("cargo", "test", "-p", "labyrinth", "--lib", "network::tests::process::six_native_processes_survive_guest_kill_and_finish_the_fight", "--profile", "ci", "--", "--ignored", "--exact", "--nocapture")
        run("cargo", "test", *packages, "--doc", "--all-features", "--profile", "ci")
    elif kind == "policy":
        run("cargo", "fmt", "--all", "--", "--check")
        run("rustfmt", "--check", "--edition", "2021", "scripts/fixtures/gamekit_consumer.rs")
        run("cargo", "clippy", *packages, "--all-targets", "--all-features", "--profile", "ci", "--", "-D", "warnings")
        if selection["deny"]:
            run("cargo", "install", "cargo-deny", "--locked")
            run("cargo", "deny", "check")
        if selection["minimal"]:
            for name in ("bevy_game_discovery", "bevy_game_multiplayer"):
                run("cargo", "check", "-p", name, "--no-default-features")
            run("cargo", "test", "-p", "bevy_game_test", "--no-default-features", "--profile", "ci")
            run("cargo", "test", "-p", "bevy_game_ui", "--profile", "ci")
        if selection["wasm"]:
            names = WASM if selection["full"] else WASM.intersection(selection["packages"])
            run("rustup", "target", "add", "wasm32-unknown-unknown")
            run("cargo", "check", *[arg for name in sorted(names) for arg in ("-p", name)], "--target", "wasm32-unknown-unknown")


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("command", choices=("select", "run", "gate"))
    parser.add_argument("kind", nargs="?", choices=JOBS)
    parser.add_argument("--base")
    parser.add_argument("--head")
    parser.add_argument("--full", action="store_true")
    args = parser.parse_args()
    root = Path(__file__).resolve().parents[1]
    if args.command == "select":
        event = json.loads(Path(os.environ["GITHUB_EVENT_PATH"]).read_text()) if os.environ.get("GITHUB_EVENT_PATH") else {}
        base = args.base or event.get("pull_request", {}).get("base", {}).get("sha") or event.get("before")
        head = args.head or git(root, "rev-parse", "HEAD").strip()
        selection = select(root, base, head, args.full or os.environ.get("GITHUB_EVENT_NAME") == "workflow_dispatch")
        validate(selection)
        encoded = json.dumps(selection, sort_keys=True)
        print(json.dumps(selection, indent=2, sort_keys=True))
        if os.environ.get("GITHUB_OUTPUT"):
            with open(os.environ["GITHUB_OUTPUT"], "a", encoding="utf-8") as output:
                output.write(f"selection={encoded}\n")
                for job in JOBS:
                    output.write(f"{job}={str(selection[job]).lower()}\n")
        if os.environ.get("GITHUB_STEP_SUMMARY"):
            with open(os.environ["GITHUB_STEP_SUMMARY"], "a", encoding="utf-8") as summary:
                summary.write("### Selected CI\n\n```json\n" + json.dumps(selection, indent=2) + "\n```\n")
    else:
        selection = json.loads(os.environ["CI_SELECTION"])
        if args.command == "gate":
            gate(selection, json.loads(os.environ["CI_NEEDS"]))
            print("All selected CI jobs succeeded; remaining jobs were intentionally skipped.")
        else:
            if not args.kind:
                parser.error("run requires a job kind")
            run_checks(root, args.kind, selection)


if __name__ == "__main__":
    try:
        main()
    except (ValueError, KeyError, subprocess.CalledProcessError) as failure:
        print(f"CI failure: {failure}", file=sys.stderr)
        sys.exit(1)
