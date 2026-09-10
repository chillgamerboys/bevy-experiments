#!/usr/bin/env python3
"""Check layout, local documentation links and capability dependency ownership."""

from __future__ import annotations

import os
from pathlib import Path
import re
import sys
import tomllib
from urllib.parse import unquote, urlsplit


def source_files(root: Path):
    """Walk source without build output, scratch or symlinked directories."""
    for base, directories, files in os.walk(root, followlinks=False):
        directories[:] = [n for n in directories if n not in {".git", ".context", ".gameskills", "target", "__pycache__"}]
        for name in files:
            yield Path(base) / name


def document_links(path: Path):
    """Yield local Markdown destinations outside fenced code blocks."""
    fenced = False
    for line in path.read_text(encoding="utf-8").splitlines():
        if line.lstrip().startswith(("```", "~~~")):
            fenced = not fenced
            continue
        if not fenced:
            for destination in re.findall(r"!?\[[^\]]*\]\(([^)]+)\)", line):
                parsed = urlsplit(destination.strip().strip("<>"))
                if not parsed.scheme and not parsed.netloc and parsed.path:
                    yield unquote(parsed.path)


def dependencies(manifest: dict):
    """Include normal, test, build and target-specific dependencies."""
    for key in ("dependencies", "dev-dependencies", "build-dependencies"):
        yield from manifest.get(key, {}).items()
    for target in manifest.get("target", {}).values():
        yield from dependencies(target)


def check(root: Path) -> list[str]:
    """Return actionable failures without modifying the repository."""
    root = root.resolve()
    failures = []
    manifest_path = root / "Cargo.toml"
    if not manifest_path.is_file():
        return ["missing root Cargo.toml"]
    manifest = tomllib.loads(manifest_path.read_text())
    if "workspace" not in manifest or "package" in manifest:
        failures.append("root must be a virtual workspace, not a legacy game package")
    for old in ("gamekit", "src", "tests", "assets", "wasm"):
        if (root / old).exists():
            failures.append(f"retired root path remains: {old}")
    files = list(source_files(root))
    games = set()
    for path in files:
        if path.name == "Cargo.toml" and path.is_relative_to(root / "games"):
            package = tomllib.loads(path.read_text()).get("package", {}).get("name")
            if package:
                games.add(package)
    for path in files:
        relative = path.relative_to(root)
        if path.name == "Cargo.lock" and path != root / "Cargo.lock":
            failures.append(f"duplicate dependency lockfile: {relative}")
        if path.suffix == ".md":
            for destination in document_links(path):
                resolved = (path.parent / destination).resolve()
                if not resolved.is_relative_to(root) or not resolved.exists():
                    failures.append(f"{relative}: missing/nonportable link {destination}")
        if path.name != "Cargo.toml":
            continue
        package = tomllib.loads(path.read_text())
        if path != manifest_path and "workspace" in package:
            failures.append(f"nested workspace: {relative}")
        if not path.is_relative_to(root / "crates"):
            continue
        for name, dependency in dependencies(package):
            dependency = dependency if isinstance(dependency, dict) else {}
            base = path.parent
            if dependency.get("workspace"):
                dependency = manifest.get("workspace", {}).get("dependencies", {}).get(name, {})
                dependency = dependency if isinstance(dependency, dict) else {}
                base = root
            target = dependency.get("package", name)
            destination = (base / dependency.get("path", ".")).resolve()
            if target in games or destination.is_relative_to(root / "games"):
                failures.append(f"{relative}: capability depends on game {target}")
            if (package.get("package", {}).get("name") != "bevy-gamekit"
                    and target == "bevy-gamekit"):
                failures.append(f"{relative}: capability depends on facade; dependency direction is reversed")
    return failures


if __name__ == "__main__":
    errors = check(Path(__file__).resolve().parents[1])
    for error in errors:
        print(f"FAIL {error}")
    if errors:
        sys.exit(1)
    print("Repository layout, local documentation links and capability ownership passed.")
