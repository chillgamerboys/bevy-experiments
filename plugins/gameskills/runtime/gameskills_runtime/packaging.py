"""Stage immutable, project-local native packages while preserving consumer files."""

from __future__ import annotations

import argparse
from contextlib import contextmanager
import hashlib
import json
import os
from pathlib import Path
import re
import shutil
import subprocess
import tempfile
import tomllib

from . import config as settings


def digest(data: bytes) -> str:
    return hashlib.sha256(data).hexdigest()


def bundle_id(manifest: dict) -> str:
    selection = digest("\n".join(sorted(manifest["packages"])).encode())[:8]
    return f"{manifest['commit']}-{selection}"


def marketplace_name(manifest: dict) -> str:
    return f"gameskills-{bundle_id(manifest)}"


@contextmanager
def setup_guard(state: Path):
    """OS-owned lock releases on process exit; the persistent file is not a lease."""
    path = state / "setup.lock"
    settings.regular_file(path)
    fd = os.open(path, os.O_RDWR | os.O_CREAT | getattr(os, "O_NOFOLLOW", 0), 0o600)
    with os.fdopen(fd, "r+b") as handle:
        if os.name == "nt":
            import msvcrt
            if os.fstat(handle.fileno()).st_size == 0:
                handle.write(b"0")
                handle.flush()
            handle.seek(0)
            try:
                msvcrt.locking(handle.fileno(), msvcrt.LK_NBLCK, 1)
            except OSError as error:
                raise ValueError("another setup is running") from error
        else:
            import fcntl
            try:
                fcntl.flock(handle, fcntl.LOCK_EX | fcntl.LOCK_NB)
            except OSError as error:
                raise ValueError("another setup is running") from error
        try:
            yield
        finally:
            if os.name == "nt":
                handle.seek(0)
                msvcrt.locking(handle.fileno(), msvcrt.LK_UNLCK, 1)


def atomic_text(path: Path, value: str | None) -> None:
    settings.regular_file(path)
    if value is None:
        path.unlink(missing_ok=True)
        return
    fd, temporary = tempfile.mkstemp(prefix=".gameskills-config-", dir=path.parent)
    try:
        with os.fdopen(fd, "w", encoding="utf-8") as handle:
            handle.write(value)
            handle.flush()
            os.fsync(handle.fileno())
        os.replace(temporary, path)
    finally:
        if os.path.exists(temporary):
            os.unlink(temporary)


def recover_setup(root: Path, state: Path) -> dict:
    journal = state / "setup-transaction.json"
    settings.regular_file(journal)
    if not journal.exists():
        return {"ok": True, "recovered": False}
    transaction = json.loads(journal.read_text())
    if set(transaction) != {settings.CONFIG, settings.LOCK}:
        raise ValueError("invalid setup recovery journal")
    for name, versions in transaction.items():
        path = root / name
        settings.regular_file(path)
        if set(versions) != {"before", "after"} or any(
            value is not None and not isinstance(value, str) for value in versions.values()
        ):
            raise ValueError("invalid setup recovery values")
        current = path.read_text() if path.exists() else None
        if current not in (versions["before"], versions["after"]):
            raise ValueError(f"recovery would overwrite a local edit: {name}")
    for name, versions in transaction.items():
        atomic_text(root / name, versions["before"])
    journal.unlink()
    return {"ok": True, "recovered": True, "action": "restored previous configuration and lock"}


def catalog(core: Path) -> dict:
    value = json.loads((core / "catalog.json").read_text())
    if value.get("schema_version") != 1 or not isinstance(value.get("packages"), dict):
        raise ValueError("invalid package catalog")
    return value


def files(root: Path) -> dict[str, str]:
    if root.is_symlink() or not root.is_dir():
        raise ValueError(f"invalid package directory: {root}")
    result = {}
    for base, directories, names in os.walk(root, followlinks=False):
        directories[:] = [n for n in directories if n != "__pycache__"]
        for name in directories + names:
            if (Path(base) / name).is_symlink():
                raise ValueError(f"package contains a symlink: {name}")
        for name in names:
            if name.endswith((".pyc", ".pyo")):
                continue
            path = Path(base) / name
            result[path.relative_to(root).as_posix()] = digest(path.read_bytes())
    return dict(sorted(result.items()))


def atomic_json(path: Path, value: dict) -> None:
    settings.regular_file(path)
    fd, name = tempfile.mkstemp(prefix=".gameskills-", dir=path.parent)
    try:
        with os.fdopen(fd, "w", encoding="utf-8") as handle:
            json.dump(value, handle, indent=2, sort_keys=True)
            handle.write("\n")
            handle.flush()
            os.fsync(handle.fileno())
        os.replace(name, path)
    finally:
        if os.path.exists(name):
            os.unlink(name)


def verify_revision(source: Path, revision: str) -> str:
    def git(*args):
        return subprocess.run(["git", *args], cwd=source, capture_output=True,
                              check=True, text=True).stdout.strip()
    reference = revision if re.fullmatch(r"[0-9a-fA-F]{40}", revision) else f"refs/tags/{revision}"
    if not revision or revision.lower() in {"head", "main", "master", "latest"}:
        raise ValueError("use a full source commit or immutable release tag")
    commit = git("rev-parse", "--verify", "--end-of-options", f"{reference}^{{commit}}")
    if git("rev-parse", "HEAD") != commit:
        raise ValueError("source checkout does not match the selected revision")
    if git("status", "--porcelain", "--", "plugins", ".agents/plugins", ".claude-plugin"):
        raise ValueError("canonical packages have uncommitted changes")
    return commit


def marketplace_entries(packages: list[str], version: str, name: str) -> tuple[dict, dict]:
    codex = {"name": name, "interface": {"displayName": "GameSkills"}, "plugins": []}
    claude = {"name": name, "owner": {"name": "GameKit contributors"}, "plugins": []}
    for package in packages:
        codex["plugins"].append({"name": package, "source": {"source": "local", "path": f"./plugins/{package}"},
                                 "policy": {"installation": "AVAILABLE", "authentication": "ON_INSTALL"},
                                 "category": "Productivity"})
        claude["plugins"].append({"name": package, "source": f"./plugins/{package}", "version": version})
    return codex, claude


def bundle(source: Path, out: Path, revision: str, packages: list[str]) -> dict:
    commit = verify_revision(source, revision)
    index = json.loads(subprocess.run(
        ["git", "show", f"{commit}:plugins/gameskills/catalog.json"], cwd=source,
        capture_output=True, check=True, text=True).stdout)
    if "gameskills" not in packages or len(set(packages)) != len(packages) or set(packages) - index["packages"].keys():
        raise ValueError("bundle requires core and a unique selection of known packages")
    if out.exists() or out.is_symlink():
        raise ValueError("bundle destination already exists; immutable bundles are never overwritten")
    out.parent.mkdir(parents=True, exist_ok=True)
    stage = Path(tempfile.mkdtemp(prefix=".gameskills-bundle-", dir=out.parent))
    try:
        manifests = {}
        for name in packages:
            # Materialize Git blobs, never ignored/local working-tree bytes.
            prefix = f"plugins/{name}/"
            listing = subprocess.run(["git", "ls-tree", "-rz", commit, "--", prefix],
                                     cwd=source, capture_output=True, check=True).stdout
            if not listing:
                raise ValueError(f"selected package absent from pinned tree: {name}")
            for entry in listing.split(b"\0"):
                if not entry:
                    continue
                metadata, raw_path = entry.split(b"\t", 1)
                mode, kind, blob = metadata.decode().split()
                relative = raw_path.decode("utf-8")
                if (kind != "blob" or mode not in {"100644", "100755"}
                        or not relative.startswith(prefix) or ".." in Path(relative).parts):
                    raise ValueError(f"unsupported package tree entry: {relative}")
                if "__pycache__" in Path(relative).parts or relative.endswith((".pyc", ".pyo")):
                    raise ValueError("compiled Python caches do not belong in canonical packages")
                path = stage / relative
                path.parent.mkdir(parents=True, exist_ok=True)
                data = subprocess.run(["git", "cat-file", "blob", blob], cwd=source,
                                      capture_output=True, check=True).stdout
                path.write_bytes(data)
                path.chmod(0o755 if mode == "100755" else 0o644)
            manifests[name] = files(stage / "plugins" / name)
        identity = {"schema_version": 1, "commit": commit, "version": index["version"],
                    "packages": manifests}
        market = marketplace_name(identity)
        codex, claude = marketplace_entries(packages, index["version"], market)
        for directory, data in ((".agents/plugins", codex), (".claude-plugin", claude)):
            (stage / directory).mkdir(parents=True)
            atomic_json(stage / directory / "marketplace.json", data)
        atomic_json(stage / "bundle.json", identity)
        stage.rename(out)
    finally:
        if stage.exists():
            shutil.rmtree(stage)
    return {"ok": True, "bundle": str(out), "marketplace": market, **identity}


def verify_bundle(path: Path) -> dict:
    if path.is_symlink() or not path.is_dir():
        raise ValueError("bundle must be an ordinary directory")
    for relative in ("plugins", ".agents", ".agents/plugins", ".claude-plugin"):
        directory = path / relative
        if directory.is_symlink() or not directory.is_dir():
            raise ValueError(f"invalid bundle directory: {relative}")
    settings.regular_file(path / "bundle.json")
    value = json.loads((path / "bundle.json").read_text())
    if set(value) != {"schema_version", "commit", "version", "packages"}:
        raise ValueError("unexpected bundle identity fields")
    if value.get("schema_version") != 1 or not re.fullmatch(r"[a-f0-9]{40}", value.get("commit", "")):
        raise ValueError("invalid bundle identity")
    if not isinstance(value.get("packages"), dict) or "gameskills" not in value["packages"]:
        raise ValueError("bundle lacks core")
    for name, expected in value["packages"].items():
        if not re.fullmatch(r"gameskills(?:-[a-z-]+)?", name) or files(path / "plugins" / name) != expected:
            raise ValueError(f"bundle package content mismatch: {name}")
    if set(p.name for p in (path / "plugins").iterdir()) != set(value["packages"]):
        raise ValueError("bundle contains unrecorded packages")
    codex, claude = marketplace_entries(list(value["packages"]), value["version"],
                                        marketplace_name(value))
    for relative, expected in ((".agents/plugins/marketplace.json", codex),
                                (".claude-plugin/marketplace.json", claude)):
        file = path / relative
        settings.regular_file(file)
        # Entry order does not affect selection; identity and paths do.
        actual = json.loads(file.read_text())
        actual["plugins"] = sorted(actual.get("plugins", []), key=lambda entry: entry["name"])
        expected["plugins"] = sorted(expected["plugins"], key=lambda entry: entry["name"])
        if actual != expected:
            raise ValueError("bundle marketplace disagrees with its recorded package selection")
    return value


def setup(root: Path, core: Path, argv: list[str]) -> dict:
    parser = argparse.ArgumentParser(prog="gameskills setup")
    parser.add_argument("--packages", nargs="+")
    parser.add_argument("--bundle", type=Path)
    parser.add_argument("--apply", action="store_true")
    parser.add_argument("--recover", action="store_true", help="restore files from an interrupted setup")
    args = parser.parse_args(argv)
    state = root / ".gameskills"
    if state.is_symlink() or (state.exists() and not state.is_dir()):
        raise ValueError(".gameskills must be an ordinary project-local directory")
    if args.recover:
        if args.apply or args.bundle or args.packages:
            raise ValueError("use setup --recover on its own")
        if not state.exists():
            return {"ok": True, "recovered": False}
        with setup_guard(state):
            return recover_setup(root, state)
    journal = state / "setup-transaction.json"
    settings.regular_file(journal)
    if journal.exists():
        raise ValueError("interrupted setup; use setup --recover before continuing")
    index = catalog(core)
    config_path, lock_path = root / settings.CONFIG, root / settings.LOCK
    settings.regular_file(config_path)
    settings.regular_file(lock_path)
    existing = config_path.read_text() if config_path.exists() else None
    selected = args.packages or (tomllib.loads(existing).get("packages", ["gameskills"]) if existing else ["gameskills"])
    text = settings.select_packages(existing, selected) if existing else settings.initial_text(selected)
    configured = settings.validate(tomllib.loads(text), set(index["packages"]))
    proposed = {"ok": True, "applied": False, "packages": configured["packages"],
                "clients": configured["clients"], "max_workers": configured["dispatch"]["max_workers"],
                "config_change": existing != text}
    if args.bundle is None:
        if args.apply:
            raise ValueError("setup --apply requires --bundle with verified immutable packages")
        return {**proposed, "next": "stage an immutable bundle, then setup --bundle PATH --apply"}
    if args.bundle.is_symlink():
        raise ValueError("bundle must not be a symlink")
    origin = args.bundle.resolve()
    manifest = verify_bundle(origin)
    if set(manifest["packages"]) != set(selected):
        raise ValueError("bundle selection must exactly match configured packages")
    destination = state / "bundles" / bundle_id(manifest)
    if (state / "bundles").is_symlink() or destination.is_symlink():
        raise ValueError("bundle storage must not contain symlinks")
    lock = {"schema_version": 1, "bundle": destination.relative_to(root).as_posix(),
            **manifest}
    proposed.update({"commit": manifest["commit"], "destination": str(destination)})
    if not args.apply:
        return proposed
    state.mkdir(exist_ok=True)
    with setup_guard(state):
        if journal.exists():
            raise ValueError("interrupted setup; use setup --recover before continuing")
        previous_lock = lock_path.read_text() if lock_path.exists() else None
        if destination.exists():
            if verify_bundle(destination) != manifest:
                raise ValueError("existing immutable bundle differs")
        else:
            destination.parent.mkdir(exist_ok=True)
            stage = Path(tempfile.mkdtemp(prefix=".install-", dir=destination.parent))
            try:
                shutil.copytree(origin, stage, dirs_exist_ok=True)
                if verify_bundle(stage) != manifest:
                    raise ValueError("source bundle changed while being installed")
                stage.rename(destination)
            finally:
                if stage.exists():
                    shutil.rmtree(stage)
        # Recheck ownership before writing consumer files; unrelated edits are preserved.
        if (config_path.read_text() if config_path.exists() else None) != existing:
            raise ValueError("project configuration changed during setup")
        lock_text = json.dumps(lock, indent=2, sort_keys=True) + "\n"
        atomic_json(journal, {settings.CONFIG: {"before": existing, "after": text},
                              settings.LOCK: {"before": previous_lock, "after": lock_text}})
        atomic_text(config_path, text)
        atomic_text(lock_path, lock_text)
        journal.unlink()
    return {**proposed, "applied": True, "native_activation": "explicit client activation still required"}


def status(root: Path, core: Path) -> dict:
    if (root / ".gameskills/setup-transaction.json").exists():
        raise ValueError("interrupted setup; use setup --recover before continuing")
    index = catalog(core)
    configured = settings.load(root, set(index["packages"]))
    path = root / settings.LOCK
    settings.regular_file(path)
    if not path.is_file():
        raise ValueError("configuration exists but no installation lock; run setup with a bundle")
    locked = json.loads(path.read_text())
    if (not isinstance(locked, dict) or set(locked) != {"schema_version", "bundle", "commit", "version", "packages"}
            or not isinstance(locked.get("packages"), dict)
            or not re.fullmatch(r"[a-f0-9]{40}", str(locked.get("commit", "")))):
        raise ValueError("invalid installation lock")
    relative = Path(locked.get("bundle", ""))
    if (relative.is_absolute() or ".." in relative.parts or len(relative.parts) != 3
            or relative.parts[:2] != (".gameskills", "bundles")
            or relative.name != bundle_id(locked)):
        raise ValueError("invalid bundle location in lock")
    current = root
    for part in relative.parts:
        current /= part
        if current.is_symlink():
            raise ValueError("bundle storage must not contain symlinks")
    if not (root / relative).is_dir():
        raise ValueError("pinned bundle is not staged in this checkout; use explicit setup")
    manifest = verify_bundle(root / relative)
    if any(locked.get(key) != manifest[key] for key in ("schema_version", "commit", "version", "packages")):
        raise ValueError("installation lock and bundle disagree")
    if set(configured["packages"]) != set(locked["packages"]):
        raise ValueError("package selection changed; use explicit setup")
    if files(core) != locked["packages"]["gameskills"]:
        raise ValueError("invoked core differs from project pin; invoke the installed candidate")
    return {"ok": True, "commit": locked["commit"], "version": locked["version"],
            "packages": configured["packages"], "clients": configured["clients"],
            "creative_level": configured["creative"]["default_level"],
            "max_workers": configured["dispatch"]["max_workers"], "bundle": str(root / relative),
            "native_activation": "not inferred from local package staging"}


def native_argv(bundle_path: Path, client: str, extra: list[str]) -> list[str]:
    """Load native plugins for this invocation without editing a global registry."""
    manifest = verify_bundle(bundle_path)
    selected = list(manifest["packages"])
    if client == "claude":
        args = ["claude"]
        for package in selected:
            args.extend(["--plugin-dir", str(bundle_path / "plugins" / package)])
    elif client == "codex":
        market = marketplace_name(manifest)
        args = ["codex", "-c", f"marketplaces.{market}.source_type=\"local\"", "-c",
                f"marketplaces.{market}.source={json.dumps(str(bundle_path))}"]
        for package in selected:
            # Codex's CLI dotted-key parser treats quotes as literal key bytes.
            # These names contain only our validated ASCII identifiers, no dots.
            args.extend(["-c", f"plugins.{package}@{market}.enabled=true"])
    else:
        raise ValueError("supported clients are codex and claude")
    return args + extra
