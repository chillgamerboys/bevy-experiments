#!/usr/bin/env python3
"""Render, audit, install, and synchronize the Bevy Gamekit skill pack."""

from __future__ import annotations

import argparse
import hashlib
import json
import shutil
import subprocess
import sys
from dataclasses import dataclass
from pathlib import Path


SKILLS = (
    "architect-bevy-game",
    "model-turn-based-game",
    "build-bevy-ui",
    "test-bevy-game",
    "verify-bevy-ui",
    "debug-bevy-runtime",
    "review-bevy-change",
)
CLIENTS = ("codex", "claude")
MANIFEST = Path(".bevy-gamekit/skills.json")
BASE = Path(".bevy-gamekit/base")
FORBIDDEN_REVISIONS = {"latest", "head", "main", "master"}


@dataclass(frozen=True)
class Change:
    """One generated path and its synchronization category."""

    category: str
    path: Path


def digest(data: bytes) -> str:
    """Return a stable content digest."""

    return hashlib.sha256(data).hexdigest()


def validate_revision(revision: str) -> None:
    """Reject absent or moving revisions."""

    if not revision.strip() or revision.strip().lower() in FORBIDDEN_REVISIONS:
        raise ValueError("revision must be an explicit release tag or commit SHA")


def verify_source_revision(source_root: Path, revision: str, allow_unverified: bool) -> None:
    """Require canonical files to come from the explicitly pinned checkout."""

    if allow_unverified:
        return
    try:
        repository_root = Path(
            subprocess.run(
                ["git", "rev-parse", "--show-toplevel"],
                cwd=source_root,
                check=True,
                capture_output=True,
                text=True,
            ).stdout.strip()
        )
        head = subprocess.run(
            ["git", "rev-parse", "HEAD^{commit}"],
            cwd=repository_root,
            check=True,
            capture_output=True,
            text=True,
        ).stdout.strip()
        pinned = subprocess.run(
            ["git", "rev-parse", f"{revision}^{{commit}}"],
            cwd=repository_root,
            check=True,
            capture_output=True,
            text=True,
        ).stdout.strip()
        status = subprocess.run(
            ["git", "status", "--porcelain", "--", str(source_root)],
            cwd=repository_root,
            check=True,
            capture_output=True,
            text=True,
        ).stdout
    except subprocess.CalledProcessError as error:
        raise ValueError("source revision cannot be verified in its Git checkout") from error
    if head != pinned:
        raise ValueError("source checkout HEAD does not match the requested revision")
    if status.strip():
        raise ValueError("canonical skill source has uncommitted changes")


def ensure_target(target: Path, allow_dirty: bool) -> None:
    """Validate target repository safety preconditions."""

    if not target.is_dir() or not (target / ".git").exists():
        raise ValueError(f"target is not a Git repository: {target}")
    status = subprocess.run(
        ["git", "status", "--porcelain"],
        cwd=target,
        check=True,
        capture_output=True,
        text=True,
    ).stdout
    if status.strip() and not allow_dirty:
        raise ValueError("target worktree is dirty; inspect it or pass --allow-dirty")


def default_source_root() -> Path:
    """Return the skill package containing this script."""

    return Path(__file__).resolve().parents[1]


def render(source_root: Path) -> dict[Path, bytes]:
    """Render canonical skills into Codex and Claude target layouts."""

    source = source_root / "source"
    references = source_root / "references"
    rendered: dict[Path, bytes] = {}
    for client, client_root in (("codex", Path(".agents")), ("claude", Path(".claude"))):
        for reference in sorted(references.glob("*.md")):
            rendered[client_root / "bevy-gamekit/references" / reference.name] = reference.read_bytes()
        for skill in SKILLS:
            skill_root = source / skill
            if not (skill_root / "SKILL.md").is_file():
                raise ValueError(f"missing canonical skill: {skill}")
            for item in sorted(path for path in skill_root.rglob("*") if path.is_file()):
                relative = item.relative_to(skill_root)
                if client == "claude" and relative.parts[:1] == ("agents",):
                    continue
                data = item.read_bytes()
                if relative == Path("SKILL.md"):
                    data = data.replace(
                        b"../../references/", b"../../bevy-gamekit/references/"
                    )
                rendered[client_root / "skills" / skill / relative] = data
    return rendered


def read_tree(root: Path) -> dict[Path, bytes]:
    """Read a rendered tree by relative file path."""

    if not root.exists():
        return {}
    return {
        path.relative_to(root): path.read_bytes()
        for path in root.rglob("*")
        if path.is_file()
    }


def classify(base: dict[Path, bytes], head: dict[Path, bytes], target: Path) -> list[Change]:
    """Classify three-way generated-file changes."""

    changes: list[Change] = []
    for path in sorted(set(base) | set(head), key=str):
        old = base.get(path)
        new = head.get(path)
        target_path = target / path
        local = target_path.read_bytes() if target_path.is_file() else None
        if old is None and new is not None:
            category = "NO-OP" if local == new else ("ADD" if local is None else "CONFLICT")
        elif old is not None and new is None:
            category = "NO-OP" if local is None else ("DELETE" if local == old else "CONFLICT")
        elif old == new:
            category = "NO-OP" if local == old else "CONFLICT"
        elif local == old:
            category = "UPDATE"
        elif local == new:
            category = "NO-OP"
        else:
            category = "CONFLICT"
        changes.append(Change(category, path))
    return changes


def print_report(changes: list[Change]) -> None:
    """Print a stable audit report."""

    counts: dict[str, int] = {}
    for change in changes:
        counts[change.category] = counts.get(change.category, 0) + 1
    summary = " ".join(f"{name}={counts[name]}" for name in sorted(counts))
    print(f"skill sync audit: {summary or 'no generated files'}")
    for change in changes:
        if change.category != "NO-OP":
            print(f"{change.category:8} {change.path}")


def write_rendered(target: Path, rendered: dict[Path, bytes]) -> None:
    """Replace the pinned rendered-base snapshot."""

    base_root = target / BASE
    if base_root.exists():
        shutil.rmtree(base_root)
    for relative, data in rendered.items():
        output = base_root / relative
        output.parent.mkdir(parents=True, exist_ok=True)
        output.write_bytes(data)


def write_manifest(target: Path, repository: str, revision: str, rendered: dict[Path, bytes]) -> None:
    """Write adoption metadata after a complete successful apply."""

    manifest_path = target / MANIFEST
    manifest_path.parent.mkdir(parents=True, exist_ok=True)
    payload = {
        "schema_version": 1,
        "source": {"repository": repository, "revision": revision},
        "clients": list(CLIENTS),
        "skills": list(SKILLS),
        "generated": {str(path): digest(data) for path, data in sorted(rendered.items(), key=lambda item: str(item[0]))},
    }
    manifest_path.write_text(json.dumps(payload, indent=2, sort_keys=True) + "\n")
    overlays = target / ".bevy-gamekit/overlays"
    overlays.mkdir(parents=True, exist_ok=True)
    (overlays / ".gitkeep").touch(exist_ok=True)


def load_manifest(target: Path) -> dict[str, object]:
    """Load and validate existing adoption metadata."""

    manifest_path = target / MANIFEST
    if not manifest_path.is_file():
        raise ValueError("target has no .bevy-gamekit/skills.json; run install first")
    payload = json.loads(manifest_path.read_text())
    if payload.get("schema_version") != 1:
        raise ValueError("unsupported or missing skill manifest schema_version")
    if payload.get("clients") != list(CLIENTS) or payload.get("skills") != list(SKILLS):
        raise ValueError("installed client or skill set differs from the complete v1 pack")
    return payload


def apply_changes(target: Path, head: dict[Path, bytes], changes: list[Change]) -> None:
    """Apply only conflict-free generated changes."""

    if any(change.category == "CONFLICT" for change in changes):
        raise ValueError("generated-file conflicts must be resolved before apply")
    for change in changes:
        target_path = target / change.path
        if change.category in {"ADD", "UPDATE"}:
            target_path.parent.mkdir(parents=True, exist_ok=True)
            target_path.write_bytes(head[change.path])
        elif change.category == "DELETE" and target_path.exists():
            target_path.unlink()


def install(args: argparse.Namespace) -> int:
    """Audit or install the initial pack."""

    target = args.target.resolve()
    ensure_target(target, args.allow_dirty)
    validate_revision(args.revision)
    verify_source_revision(args.source_root.resolve(), args.revision, args.allow_unverified_source)
    if (target / MANIFEST).exists():
        return synchronize(args)
    head = render(args.source_root.resolve())
    changes = classify({}, head, target)
    print_report(changes)
    if not args.apply:
        return 0
    apply_changes(target, head, changes)
    write_rendered(target, head)
    write_manifest(target, args.repository, args.revision, head)
    return 0


def synchronize(args: argparse.Namespace) -> int:
    """Audit or apply a pinned pack synchronization."""

    target = args.target.resolve()
    ensure_target(target, args.allow_dirty)
    validate_revision(args.revision)
    verify_source_revision(args.source_root.resolve(), args.revision, args.allow_unverified_source)
    manifest = load_manifest(target)
    source = manifest.get("source")
    if not isinstance(source, dict) or not isinstance(source.get("repository"), str):
        raise ValueError("manifest source repository is missing")
    repository = getattr(args, "repository", None) or source["repository"]
    base = read_tree(target / BASE)
    if not base:
        raise ValueError("pinned rendered-base snapshot is missing")
    head = render(args.source_root.resolve())
    changes = classify(base, head, target)
    print_report(changes)
    if not args.apply:
        return 0
    apply_changes(target, head, changes)
    write_rendered(target, head)
    write_manifest(target, repository, args.revision, head)
    return 0


def parser() -> argparse.ArgumentParser:
    """Build the command-line parser."""

    root = argparse.ArgumentParser(description=__doc__)
    commands = root.add_subparsers(dest="command", required=True)
    for name in ("install", "sync"):
        command = commands.add_parser(name)
        command.add_argument("--target", required=True, type=Path)
        command.add_argument("--revision", required=True)
        command.add_argument("--source-root", type=Path, default=default_source_root())
        command.add_argument("--apply", action="store_true")
        command.add_argument("--allow-dirty", action="store_true")
        command.add_argument("--allow-unverified-source", action="store_true", help=argparse.SUPPRESS)
        if name == "install":
            command.add_argument("--repository", required=True)
    return root


def main(argv: list[str] | None = None) -> int:
    """Run the requested operation with concise failures."""

    args = parser().parse_args(argv)
    try:
        if args.command == "install":
            return install(args)
        return synchronize(args)
    except (OSError, ValueError, subprocess.CalledProcessError, json.JSONDecodeError) as error:
        print(f"error: {error}", file=sys.stderr)
        return 2


if __name__ == "__main__":
    raise SystemExit(main())
