#!/usr/bin/env python3
"""Build an external consumer from Cargo-selected library files, with no games.

This is a release-boundary probe, not a publishing or archive command. Temporary
sources are removed on exit; Cargo artifacts reuse the repository target directory.
"""

from __future__ import annotations

import argparse
import json
from pathlib import Path
import re
import shutil
import subprocess
import tempfile
import tomllib


GENERATED = {".cargo_vcs_info.json", "Cargo.lock", "Cargo.toml.orig"}
CASES = ("empty", "pure", "ui", "network")


def library_manifest(source: str) -> str:
    """Retain shared settings, replacing only the workspace membership table."""
    parsed = tomllib.loads(source)
    if "package" in parsed or "workspace" not in parsed:
        raise ValueError("expected a virtual workspace")
    resolver = parsed["workspace"]["resolver"]
    result, count = re.subn(
        r"(?ms)^\[workspace\]\s*\n.*?(?=^\[|\Z)",
        f'[workspace]\nresolver = {json.dumps(resolver)}\nmembers = ["crates/*"]\n\n',
        source,
        count=1,
    )
    if count != 1:
        raise ValueError("cannot locate workspace table")
    return result


def package_sources(crate: Path, listing: str) -> list[Path]:
    """Reject escapes and symlinks before accepting Cargo's package file list."""
    result = []
    for name in listing.splitlines():
        if name in GENERATED:
            continue
        relative = Path(name)
        source = crate / relative
        if relative.is_absolute() or ".." in relative.parts:
            raise ValueError(f"nonlocal package entry: {name}")
        if not source.resolve().is_relative_to(crate.resolve()):
            raise ValueError(f"package entry escapes crate: {name}")
        if any(part.is_symlink() for part in [source, *source.parents]):
            raise ValueError(f"symlinked package entry: {name}")
        if not source.is_file():
            raise ValueError(f"missing package source: {name}")
        result.append(relative)
    if Path("Cargo.toml") not in result or Path("src/lib.rs") not in result:
        raise ValueError("capability package must contain its manifest and library")
    return result


def validate_graph(case: str, names: set[str], games: set[str]) -> None:
    """Check the activated graph, not the optional packages in Cargo.lock."""
    if names & games:
        raise ValueError(f"consumer resolved games: {sorted(names & games)}")
    if case == "empty" and names != {"gamekit_external_probe", "bevy-gamekit"}:
        raise ValueError(f"empty facade has dependencies: {sorted(names)}")
    if case == "pure" and "bevy" in names:
        raise ValueError("pure algorithms pulled in Bevy")
    if case in {"empty", "pure", "ui"}:
        forbidden = {"bevy_game_multiplayer", "bevy_game_discovery", "aeronet_webtransport"}
        if names & forbidden:
            raise ValueError(f"offline consumer pulled in networking: {sorted(names & forbidden)}")


def run(command: list[str], cwd: Path, *, capture: bool = False) -> str:
    print("+", " ".join(command), flush=True)
    return subprocess.run(command, cwd=cwd, check=True, text=True,
                          stdout=subprocess.PIPE if capture else None).stdout or ""


def check(root: Path, case: str) -> None:
    root = root.resolve()
    games = {tomllib.loads(p.read_text())["package"]["name"]
             for p in (root / "games").rglob("Cargo.toml")}
    with tempfile.TemporaryDirectory(prefix="gamekit-consumer-") as temporary:
        staging = Path(temporary).resolve()
        library = staging / "library"
        library.mkdir()
        (library / "Cargo.toml").write_text(library_manifest((root / "Cargo.toml").read_text()))
        for manifest in sorted((root / "crates").glob("*/Cargo.toml")):
            package = tomllib.loads(manifest.read_text())["package"]["name"]
            listing = run(["cargo", "package", "-p", package, "--list", "--allow-dirty"], root, capture=True)
            for relative in package_sources(manifest.parent, listing):
                destination = library / "crates" / manifest.parent.name / relative
                destination.parent.mkdir(parents=True, exist_ok=True)
                shutil.copyfile(manifest.parent / relative, destination)

        consumer = staging / "consumer"
        (consumer / "src").mkdir(parents=True)
        facade = json.dumps(str(library / "crates/bevy_gamekit"))
        (consumer / "Cargo.toml").write_text(f'''[package]
name = "gamekit_external_probe"
version = "0.0.0"
edition = "2021"
publish = false

[dependencies]
bevy_gamekit = {{ package = "bevy-gamekit", path = {facade}, default-features = false }}
serde_json = {{ version = "1", optional = true }}

[features]
default = []
pure = ["bevy_gamekit/hex", "bevy_gamekit/turns-serde", "dep:serde_json"]
ui = ["bevy_gamekit/testing-ui"]
network = ["bevy_gamekit/direct", "bevy_gamekit/mdns", "bevy_gamekit/tailscale-cli"]

[profile.ci]
inherits = "dev"
debug = "line-tables-only"
codegen-units = 4
''')
        shutil.copyfile(root / "scripts/fixtures/gamekit_consumer.rs", consumer / "src/lib.rs")
        # Pin the probe to the same dependency versions; Cargo prunes game entries.
        shutil.copyfile(root / "Cargo.lock", consumer / "Cargo.lock")
        for selected in CASES if case == "all" else (case,):
            features = [] if selected == "empty" else ["--features", selected]
            graph = run(["cargo", "tree", "--edges", "normal,build", "--prefix", "none",
                         "--format", "{p}", *features], consumer, capture=True)
            names = {line.split()[0] for line in graph.splitlines() if line.strip()}
            validate_graph(selected, names, games)
            run(["cargo", "test", "--profile", "ci", "--target-dir", str(root / "target"),
                 *features], consumer)
            print(f"External consumer passed: {selected}", flush=True)


if __name__ == "__main__":
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--case", choices=(*CASES, "all"), default="all")
    arguments = parser.parse_args()
    check(Path(__file__).resolve().parents[1], arguments.case)
