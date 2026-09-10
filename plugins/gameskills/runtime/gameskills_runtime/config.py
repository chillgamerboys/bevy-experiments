"""Load project-owned workflow settings without evaluating configuration as code."""

from __future__ import annotations

import json
import re
import subprocess
import tomllib
from pathlib import Path

CONFIG = "gameskills.toml"
LOCK = "gameskills.lock.json"
IDENTIFIER = re.compile(r"[a-z][a-z0-9_-]{0,63}\Z")


def repository(root: Path) -> Path:
    root = root.resolve()
    result = subprocess.run(["git", "rev-parse", "--show-toplevel"], cwd=root,
                            capture_output=True, text=True, check=True)
    if Path(result.stdout.strip()).resolve() != root:
        raise ValueError("--root must name the Git worktree root")
    return root


def regular_file(path: Path) -> None:
    if path.is_symlink() or (path.exists() and not path.is_file()):
        raise ValueError(f"expected an ordinary file: {path}")


def defaults() -> dict:
    return {"schema_version": 1, "packages": ["gameskills"],
            "clients": ["codex", "claude"], "creative": {"default_level": 2},
            "dispatch": {"enabled": False, "max_workers": 5},
            "commands": {}, "agents": {}, "targets": {}}


def validate(value: dict, known_packages: set[str]) -> dict:
    if not isinstance(value, dict) or value.get("schema_version") != 1:
        raise ValueError("gameskills.toml requires schema_version = 1")
    unknown = set(value) - {"schema_version", "packages", "clients", "project", "creative",
                            "dispatch", "commands", "agents", "targets"}
    if unknown:
        raise ValueError(f"unknown configuration fields: {sorted(unknown)}")
    result = defaults()
    result.update(value)
    for key in ("packages", "clients"):
        items = result[key]
        if not isinstance(items, list) or not items or any(not isinstance(x, str) for x in items):
            raise ValueError(f"{key} must be a nonempty string array")
        if len(set(items)) != len(items):
            raise ValueError(f"{key} contains duplicates")
    if "gameskills" not in result["packages"] or set(result["packages"]) - known_packages:
        raise ValueError("select gameskills core and only known optional packages")
    if set(result["clients"]) - {"codex", "claude"}:
        raise ValueError("supported clients are codex and claude")
    for key in ("creative", "dispatch", "commands", "agents", "targets"):
        if not isinstance(result[key], dict):
            raise ValueError(f"{key} must be a table")
    result["creative"] = {**defaults()["creative"], **result["creative"]}
    result["dispatch"] = {**defaults()["dispatch"], **result["dispatch"]}
    level = result["creative"]["default_level"]
    workers = result["dispatch"]["max_workers"]
    if type(level) is not int or not 1 <= level <= 4:
        raise ValueError("creative.default_level must be 1..4")
    if type(workers) is not int or not 1 <= workers <= 5:
        raise ValueError("dispatch.max_workers must be 1..5")
    if type(result["dispatch"]["enabled"]) is not bool:
        raise ValueError("dispatch.enabled must be boolean")
    for name, command in result["commands"].items():
        if not IDENTIFIER.fullmatch(name) or not isinstance(command, dict):
            raise ValueError(f"invalid command name/table: {name}")
        argv = command.get("argv")
        if not isinstance(argv, list) or not argv or any(not isinstance(x, str) or "\0" in x for x in argv):
            raise ValueError(f"commands.{name}.argv must be a nonempty string array")
    for name, target in result["targets"].items():
        if not IDENTIFIER.fullmatch(name) or not isinstance(target, dict):
            raise ValueError(f"invalid target: {name}")
        packages = target.get("packages", [])
        if not isinstance(packages, list) or any(p not in result["packages"] for p in packages):
            raise ValueError(f"target {name} selects an uninstalled package")
        path = Path(target.get("path", "."))
        if path.is_absolute() or ".." in path.parts:
            raise ValueError(f"target {name} path must stay inside its repository")
    return result


def load(root: Path, known_packages: set[str]) -> dict:
    path = root / CONFIG
    regular_file(path)
    if not path.exists():
        raise ValueError("GameSkills is not configured; run setup to inspect adoption")
    return validate(tomllib.loads(path.read_text(encoding="utf-8")), known_packages)


def initial_text(packages: list[str], enabled: bool = False) -> str:
    return ("# Project-owned GameSkills configuration. Commands execute only when requested.\n"
            "schema_version = 1\n" + f"packages = {json.dumps(packages)}\n"
            'clients = ["codex", "claude"]\n\n[creative]\ndefault_level = 2\n\n'
            f"[dispatch]\nenabled = {str(enabled).lower()}\nmax_workers = 5\n")


def select_packages(text: str, packages: list[str]) -> str:
    """Change only the root package field, preserving comments and local tables."""
    parsed = tomllib.loads(text)
    if parsed.get("packages", ["gameskills"]) == packages:
        return text
    boundary = re.search(r"(?m)^\s*\[", text)
    end = boundary.start() if boundary else len(text)
    prefix, suffix = text[:end], text[end:]
    expression = re.compile(r'''(?ms)^[ \t]*(?:packages|"packages"|'packages')\s*=\s*\[.*?\]''')
    replacement = f"packages = {json.dumps(packages)}"
    updated, count = expression.subn(lambda _: replacement, prefix)
    if count == 0:
        updated += replacement + "\n"
    if count > 1:
        raise ValueError("ambiguous package configuration")
    candidate = updated + suffix
    expected = tomllib.loads(text)
    expected["packages"] = packages
    if tomllib.loads(candidate) != expected:
        raise ValueError("package selection cannot be changed without altering other settings")
    return candidate
