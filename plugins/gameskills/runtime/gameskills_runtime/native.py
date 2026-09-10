"""Bounded native Codex cache warmup and source-bound skill discovery.

Only initialization and read/discovery RPCs are sent. No task, model turn,
plugin-install/configuration RPC, global registration, or custom daemon is used.
"""
from __future__ import annotations

import json
import os
import signal
from pathlib import Path
import queue
import subprocess
import tempfile
import threading
import time

from . import packaging


class _PendingDiscovery(ValueError):
    pass


class _RPC:
    def __init__(self, process, deadline):
        self.process = process
        self.deadline = deadline
        self.messages = queue.Queue()
        self.next_id = 0
        self.reader = threading.Thread(target=self._read, daemon=True)
        self.reader.start()

    def _read(self):
        try:
            while True:
                line = self.process.stdout.readline(4 * 1024 * 1024 + 1)
                if not line:
                    self.messages.put(ValueError("Codex app-server exited before responding"))
                    return
                if len(line) > 4 * 1024 * 1024:
                    raise ValueError("Codex app-server response exceeds the bounded message size")
                value = json.loads(line)
                if not isinstance(value, dict):
                    raise ValueError("Codex app-server returned a non-object response")
                self.messages.put(value)
        except (ValueError, OSError) as error:
            self.messages.put(ValueError(f"invalid Codex app-server response: {error}"))

    def notify(self, method, params):
        try:
            self.process.stdin.write((json.dumps({"method": method, "params": params}) + "\n").encode())
            self.process.stdin.flush()
        except (BrokenPipeError, OSError) as error:
            raise ValueError(f"Codex app-server input closed: {error}") from error

    def call(self, method, params):
        request_id = self.next_id
        self.next_id += 1
        try:
            self.process.stdin.write((json.dumps({"id": request_id, "method": method, "params": params}) + "\n").encode())
            self.process.stdin.flush()
        except (BrokenPipeError, OSError) as error:
            raise ValueError(f"Codex app-server input closed: {error}") from error
        while True:
            remaining = self.deadline - time.monotonic()
            if remaining <= 0:
                raise ValueError(f"Codex native activation timed out during {method}")
            try:
                message = self.messages.get(timeout=remaining)
            except queue.Empty as error:
                raise ValueError(f"Codex native activation timed out during {method}") from error
            if isinstance(message, Exception):
                raise message
            if "method" in message:
                if "id" in message:
                    raise ValueError("Codex app-server requested an unexpected interactive action")
                continue
            if message.get("id") != request_id:
                raise ValueError("Codex app-server returned a mismatched response id")
            if "error" in message:
                raise ValueError(f"Codex {method} failed: {message['error']}")
            result = message.get("result")
            if not isinstance(result, dict):
                raise ValueError(f"Codex {method} returned an invalid result")
            return result


def _plugins(result, market):
    entries = result.get("marketplaces")
    if not isinstance(entries, list):
        raise ValueError("Codex plugin catalog is missing marketplaces")
    matches = [entry for entry in entries if isinstance(entry, dict) and entry.get("name") == market]
    if len(matches) != 1 or not isinstance(matches[0].get("plugins"), list):
        raise ValueError("Codex did not discover the pinned marketplace")
    return {entry.get("id"): entry for entry in matches[0]["plugins"] if isinstance(entry, dict)}


def _verify_skills(result, root, manifest, catalog, cache_root, market):
    rows = result.get("data")
    if not isinstance(rows, list):
        raise ValueError("Codex skill catalog is missing data")
    matching = [row for row in rows if isinstance(row, dict) and row.get("cwd") and Path(row["cwd"]).resolve() == root]
    if len(matching) != 1 or not isinstance(matching[0].get("skills"), list):
        raise ValueError("Codex did not return the requested project's skill catalog")
    expected = {f"{package}@{market}": package for package in manifest["packages"]}
    found = {}
    package_roots = {}
    for skill in matching[0]["skills"]:
        if not isinstance(skill, dict) or skill.get("enabled") is not True:
            continue
        plugin_id = skill.get("pluginId")
        if not isinstance(plugin_id, str):
            continue
        package = plugin_id.split("@", 1)[0]
        if package in catalog["packages"] and plugin_id not in expected:
            raise ValueError(f"another GameSkills pin or unselected package is enabled: {plugin_id}")
        if plugin_id not in expected:
            continue
        native_name = skill.get("name")
        name = native_name.removeprefix(package + ":") if isinstance(native_name, str) else native_name
        if name not in catalog["packages"][package]["skills"]:
            raise ValueError(f"unexpected native skill in {plugin_id}: {name}")
        path = Path(skill.get("path", ""))
        relative = Path("skills") / name / "SKILL.md"
        if not path.is_absolute() or path.parts[-3:] != relative.parts or not path.is_file():
            raise ValueError(f"Codex returned an invalid skill path for {plugin_id}:{name}")
        actual_package = path.parents[2].resolve()
        if not actual_package.is_relative_to(cache_root / market / package):
            raise ValueError(f"Codex skill is outside the pinned native cache: {path}")
        if package in package_roots and package_roots[package] != actual_package:
            raise ValueError(f"Codex returned inconsistent cache roots for {package}")
        key = (package, name)
        if key in found:
            raise ValueError(f"Codex returned a duplicate skill: {plugin_id}:{name}")
        package_roots[package] = actual_package
        found[key] = {"package": package, "name": name, "native_name": native_name, "plugin_id": plugin_id, "path": str(path.resolve())}
    wanted = {(package, name) for package in manifest["packages"] for name in catalog["packages"][package]["skills"]}
    if set(found) != wanted:
        raise _PendingDiscovery(f"native GameSkills discovery is incomplete: missing {sorted(wanted - set(found))}")
    for package, actual in package_roots.items():
        if packaging.files(actual) != manifest["packages"][package]:
            raise ValueError(f"native cache content differs from the pinned bundle: {package}")
    return [found[key] for key in sorted(found)]


def activate_codex(bundle_path: Path, root: Path, executable="codex", *, timeout_seconds=30.0) -> dict:
    """Materialize through native discovery and verify selected skills/cache bytes.

    This observes native installation/discovery, not model skill use or behavior.
    The native app-server may write its package cache. No user settings are edited.
    """
    if not isinstance(timeout_seconds, (int, float)) or isinstance(timeout_seconds, bool) or not 0 < timeout_seconds <= 120:
        raise ValueError("native activation timeout must be greater than zero and at most 120 seconds")
    root = Path(root).resolve()
    bundle_path = Path(bundle_path)
    manifest = packaging.verify_bundle(bundle_path)
    bundle_path = bundle_path.resolve()
    catalog = packaging.catalog(bundle_path / "plugins/gameskills")
    market = packaging.marketplace_name(manifest)
    command = packaging.native_argv(bundle_path, "codex", ["app-server", "--stdio"])
    command[0] = str(executable)
    deadline = time.monotonic() + timeout_seconds
    with tempfile.TemporaryFile() as errors:
        try:
            process = subprocess.Popen(command, cwd=root, stdin=subprocess.PIPE, stdout=subprocess.PIPE, stderr=errors, start_new_session=(os.name == "posix"))
        except OSError as error:
            raise ValueError(f"cannot start Codex native activation: {error}") from error
        rpc = _RPC(process, deadline)
        try:
            initialized = rpc.call("initialize", {"clientInfo": {"name": "gameskills", "version": "0.1.0"},
                                                   "capabilities": {"experimentalApi": True}})
            native_home = initialized.get("codexHome")
            if not isinstance(native_home, str) or not Path(native_home).is_absolute():
                raise ValueError("Codex initialization did not identify its native cache home")
            rpc.notify("initialized", {})
            params = {"cwds": [str(root)], "forceRefetch": False, "marketplaceKinds": ["local"]}
            initial_plugins = _plugins(rpc.call("plugin/list", params), market)
            expected_ids = {f"{package}@{market}" for package in manifest["packages"]}
            if not expected_ids.issubset(initial_plugins):
                raise ValueError("Codex plugin catalog is missing selected packages")
            while True:
                try:
                    skills = _verify_skills(rpc.call("skills/list", {"cwds": [str(root)], "forceReload": True}),
                                            root, manifest, catalog, (Path(native_home) / "plugins/cache").resolve(), market)
                    final_plugins = _plugins(rpc.call("plugin/list", params), market)
                    if any(final_plugins.get(name, {}).get("enabled") is not True for name in expected_ids):
                        raise ValueError("Codex did not enable selected plugins")
                    if any(final_plugins.get(name, {}).get("installed") is not True for name in expected_ids):
                        raise _PendingDiscovery("Codex has not confirmed selected plugins are installed")
                    break
                except _PendingDiscovery as error:
                    remaining = deadline - time.monotonic()
                    if remaining <= 0:
                        raise ValueError(f"Codex native activation timed out: {error}") from error
                    time.sleep(min(0.1, remaining))
            return {"ok": True, "client": "codex", "client_identity": initialized.get("userAgent"),
                    "commit": manifest["commit"], "marketplace": market, "packages": list(manifest["packages"]),
                    "skills": skills, "installed_plugin_ids": sorted(expected_ids),
                    "claim": "native installation and skill discovery observed; model invocation and behavior not tested"}
        finally:
            if os.name == "posix":
                try:
                    os.killpg(process.pid, signal.SIGTERM)
                except ProcessLookupError:
                    pass
                except PermissionError:
                    if process.poll() is None:
                        process.terminate()
            elif process.poll() is None:
                process.terminate()
            try:
                process.wait(timeout=2)
            except subprocess.TimeoutExpired:
                process.kill()
                process.wait(timeout=2)
            if os.name == "posix":
                # A child may outlive its parent while holding the stdout pipe.
                try:
                    os.killpg(process.pid, signal.SIGKILL)
                except (ProcessLookupError, PermissionError):
                    pass
            rpc.reader.join(timeout=1)
            try:
                process.stdin.close()
            except OSError:
                pass
            # Closing a buffered reader still owned by a blocked thread can hang.
            if not rpc.reader.is_alive():
                process.stdout.close()
