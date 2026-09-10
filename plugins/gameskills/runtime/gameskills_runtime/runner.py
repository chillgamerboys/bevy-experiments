"""Shell-free command graphs and source-bound, locally auditable observations.

This module provides local workflow integrity, not attestation against an owner
who can rewrite both records and digests. It never records review acceptance.
"""

from __future__ import annotations

import argparse
from concurrent.futures import FIRST_COMPLETED, ThreadPoolExecutor, wait
from contextlib import ExitStack
import hashlib
import json
import math
import os
from pathlib import Path, PurePosixPath, PureWindowsPath
import platform
import re
import shutil
import signal
import stat
import subprocess
import sys
import threading
import time
import uuid


SCHEMA_VERSION = 1
NAME = re.compile(r"[A-Za-z0-9][A-Za-z0-9_.-]{0,63}\Z")
RUN_ID = re.compile(r"[0-9a-f]{32}\Z")
MANAGED_FILES = ("gameskills.toml", "gameskills.lock.json")
_EXEC = (
    "import os,sys; fd=int(sys.argv[1]); os.fchdir(fd); "
    "os.close(fd); os.execvpe(sys.argv[2],sys.argv[2:],os.environ)"
)


def _json(value: object) -> bytes:
    return json.dumps(value, sort_keys=True, separators=(",", ":"),
                      ensure_ascii=True, allow_nan=False).encode("utf-8")


def _digest(value: object) -> str:
    return hashlib.sha256(_json(value)).hexdigest()


def _now() -> str:
    from datetime import datetime, timezone
    return datetime.now(timezone.utc).isoformat()


class _Directory:
    """Anchor all generated-state I/O to checked directory descriptors.

    Names are single path components; no symlink, hardlink, or FIFO is accepted
    as an evidence file. Atomic replacements cannot follow a raced symlink.
    """

    def __init__(self, fd: int):
        self.fd = fd

    @classmethod
    def root(cls, path: Path) -> _Directory:
        return cls(os.open(path, os.O_RDONLY | os.O_DIRECTORY | os.O_NOFOLLOW))

    def __enter__(self) -> _Directory:
        return self

    def __exit__(self, *unused: object) -> None:
        self.close()

    def close(self) -> None:
        if self.fd >= 0:
            os.close(self.fd)
            self.fd = -1

    @staticmethod
    def _name(name: str) -> None:
        if not name or name in (".", "..") or "/" in name or "\\" in name or "\x00" in name:
            raise ValueError(f"unsafe state filename: {name!r}")

    def child(self, name: str, *, create: bool = False, exclusive: bool = False) -> _Directory:
        self._name(name)
        if create:
            try:
                os.mkdir(name, mode=0o700, dir_fd=self.fd)
            except FileExistsError:
                if exclusive:
                    raise
        fd = os.open(name, os.O_RDONLY | os.O_DIRECTORY | os.O_NOFOLLOW, dir_fd=self.fd)
        info = os.fstat(fd)
        if info.st_uid != os.getuid() or info.st_mode & 0o022:
            os.close(fd)
            raise ValueError(f"state directory must be owned by this user and not writable by others: {name}")
        return _Directory(fd)

    def open(self, name: str, *, create: bool = False, exclusive: bool = False) -> int:
        self._name(name)
        flags = os.O_RDWR if create else os.O_RDONLY
        flags |= os.O_NOFOLLOW | os.O_NONBLOCK
        if create:
            # Separate create from open-existing. On the supported macOS host,
            # concurrent openat(O_CREAT | O_NOFOLLOW) of an absent filename can
            # report ENOENT to one contender. Exclusive creation gives a clear
            # EEXIST transition without retrying arbitrary path failures.
            try:
                fd = os.open(name, flags | os.O_CREAT | os.O_EXCL, 0o600, dir_fd=self.fd)
            except FileExistsError:
                if exclusive:
                    raise
                fd = os.open(name, flags, dir_fd=self.fd)
        else:
            fd = os.open(name, flags, dir_fd=self.fd)
        info = os.fstat(fd)
        if (not stat.S_ISREG(info.st_mode) or info.st_nlink != 1
                or info.st_uid != os.getuid() or info.st_mode & 0o022):
            os.close(fd)
            raise ValueError(f"unsafe state file: {name}")
        return fd

    def read(self, name: str) -> bytes:
        with os.fdopen(self.open(name), "rb") as handle:
            return handle.read()

    def file_digest(self, name: str) -> str:
        with os.fdopen(self.open(name), "rb") as handle:
            return hashlib.file_digest(handle, "sha256").hexdigest()

    def write_json(self, name: str, value: object) -> None:
        self._name(name)
        temporary = ".write-" + uuid.uuid4().hex
        try:
            with os.fdopen(self.open(temporary, create=True, exclusive=True), "wb") as handle:
                handle.write(_json(value) + b"\n")
                handle.flush()
                os.fsync(handle.fileno())
            os.replace(temporary, name, src_dir_fd=self.fd, dst_dir_fd=self.fd)
            os.fsync(self.fd)
        finally:
            try:
                os.unlink(temporary, dir_fd=self.fd)
            except FileNotFoundError:
                pass


def _supported() -> None:
    if os.name != "posix" or not hasattr(os, "O_NOFOLLOW"):
        raise ValueError("command execution/evidence requires POSIX directory descriptors, flock, and process groups; Windows is not supported")


def _git(root: Path, *args: str) -> bytes:
    result = subprocess.run(["git", "-C", str(root), *args], capture_output=True, check=False)
    if result.returncode:
        raise ValueError("git " + " ".join(args) + ": " + result.stderr.decode(errors="replace").strip())
    return result.stdout


def _repository(root: Path) -> dict:
    canonical = root.resolve(strict=True)
    top = Path(os.fsdecode(_git(canonical, "rev-parse", "--show-toplevel")).strip()).resolve()
    if canonical != top:
        raise ValueError("--root must name the Git worktree root")
    return {
        "root": str(canonical),
        "git_dir": str(Path(os.fsdecode(_git(canonical, "rev-parse", "--absolute-git-dir")).strip()).resolve()),
        "common_dir": str(Path(os.fsdecode(_git(canonical, "rev-parse", "--path-format=absolute", "--git-common-dir")).strip()).resolve()),
        "head": _git(canonical, "rev-parse", "--verify", "HEAD").decode().strip(),
    }


def _relative(value: str) -> tuple[str, ...]:
    if not isinstance(value, str) or not value or "\\" in value or "\x00" in value:
        raise ValueError("command cwd must be a repository-relative directory")
    path = PurePosixPath(value)
    if path.is_absolute() or PureWindowsPath(value).drive or ".." in path.parts:
        raise ValueError("command cwd must stay inside the repository")
    if ".git" in path.parts or ".gameskills" in path.parts:
        raise ValueError("command cwd cannot use repository metadata or runner state")
    return path.parts


def _cwd(root: Path, relative: str) -> _Directory:
    directory = _Directory.root(root)
    try:
        for part in _relative(relative):
            # Source directories need not be private, but must not be symlinks.
            next_fd = os.open(part, os.O_RDONLY | os.O_DIRECTORY | os.O_NOFOLLOW, dir_fd=directory.fd)
            directory.close()
            directory = _Directory(next_fd)
        return directory
    except BaseException:
        directory.close()
        raise


def _number(value: object, label: str, *, minimum: float = 0) -> float:
    if isinstance(value, bool) or not isinstance(value, (int, float)) or not math.isfinite(value) or value < minimum:
        raise ValueError(f"{label} must be a finite number >= {minimum}")
    return float(value)


def _graph(root: Path, config: dict, selected: list[str]) -> tuple[dict, list[str]]:
    configured = config.get("commands", {})
    if not isinstance(configured, dict):
        raise ValueError("commands must be a mapping")
    normalized: dict[str, dict] = {}
    order: list[str] = []
    visiting: set[str] = set()

    def visit(name: str) -> None:
        if not isinstance(name, str) or not NAME.fullmatch(name):
            raise ValueError(f"invalid command name: {name!r}")
        if name in visiting:
            raise ValueError(f"command dependency cycle at {name}")
        if name in normalized:
            return
        if name not in configured or not isinstance(configured[name], dict):
            raise ValueError(f"unknown command: {name}")
        command = configured[name]
        argv = command.get("argv")
        if (not isinstance(argv, list) or not argv or
                any(not isinstance(arg, str) or "\x00" in arg for arg in argv) or not argv[0]):
            raise ValueError(f"commands.{name}.argv must be a nonempty list of strings")
        directory = command.get("cwd", ".")
        with _cwd(root, directory):
            pass
        requires = command.get("requires", [])
        resources = command.get("resources", [])
        for label, values in (("requires", requires), ("resources", resources)):
            if not isinstance(values, list) or any(not isinstance(item, str) or not item or "\x00" in item for item in values):
                raise ValueError(f"commands.{name}.{label} must be a list of nonempty strings")
            if len(values) != len(set(values)):
                raise ValueError(f"commands.{name}.{label} contains duplicates")
        timeout = _number(command.get("timeout_seconds", 600), f"commands.{name}.timeout_seconds", minimum=0.001)
        visiting.add(name)
        for dependency in sorted(requires):
            visit(dependency)
        visiting.remove(name)
        normalized[name] = {"argv": argv[:], "cwd": str(PurePosixPath(directory)),
                            "requires": sorted(requires), "resources": sorted(resources),
                            "timeout_seconds": timeout}
        order.append(name)

    for name in sorted(set(selected)):
        visit(name)
    return normalized, order


def _identity(root: Path, config: dict, commands: dict) -> dict:
    repository = _repository(root)
    source = hashlib.sha256()
    # Includes staged changes, tracked files even when ignored, and nonignored
    # untracked files. Generated state and managed config have separate owners.
    source.update(hashlib.sha256(_git(root, "diff", "--cached", "--binary", "--no-ext-diff", "HEAD", "--", ".", ":(exclude).gameskills", *[f":(exclude){name}" for name in MANAGED_FILES])).digest())
    files = sorted(set(_git(root, "ls-files", "--cached", "--others", "--exclude-standard", "-z").split(b"\0")) - {b""})
    for raw in files:
        relative = os.fsdecode(raw)
        if relative == ".gameskills" or relative.startswith(".gameskills/") or relative in MANAGED_FILES:
            continue
        entry = {"path": relative}
        try:
            # Anchor every ancestor too: replacing a tracked directory with a
            # symlink must not hash files outside the worktree.
            parent = str(PurePosixPath(relative).parent)
            with _cwd(root, parent) as directory:
                name = PurePosixPath(relative).name
                info = os.stat(name, dir_fd=directory.fd, follow_symlinks=False)
                entry["mode"] = stat.S_IMODE(info.st_mode)
                if stat.S_ISLNK(info.st_mode):
                    entry.update(kind="symlink", target=os.readlink(name, dir_fd=directory.fd))
                elif stat.S_ISREG(info.st_mode):
                    fd = os.open(name, os.O_RDONLY | os.O_NOFOLLOW | os.O_NONBLOCK, dir_fd=directory.fd)
                    with os.fdopen(fd, "rb") as handle:
                        before = os.fstat(handle.fileno())
                        if not stat.S_ISREG(before.st_mode) or (before.st_dev, before.st_ino) != (info.st_dev, info.st_ino):
                            raise ValueError(f"source changed while fingerprinting: {relative}")
                        entry.update(kind="file", sha256=hashlib.file_digest(handle, "sha256").hexdigest())
                        after = os.fstat(handle.fileno())
                        if (before.st_size, before.st_mtime_ns, before.st_ctime_ns) != (after.st_size, after.st_mtime_ns, after.st_ctime_ns):
                            raise ValueError(f"source changed while fingerprinting: {relative}")
                elif stat.S_ISDIR(info.st_mode):
                    # A gitlink is a separately versioned input. Recurse
                    # conservatively; an unpopulated submodule needs setup.
                    path = root / relative
                    if (path / ".git").exists():
                        entry.update(kind="submodule", identity=_identity(path, {}, {}))
                    else:
                        raise ValueError(f"tracked directory has no readable Git identity: {relative}")
                else:
                    raise ValueError(f"unsupported source file type: {relative}")
        except FileNotFoundError:
            entry.update(kind="missing")
        source.update(_json(entry))
    managed = {}
    with _Directory.root(root) as directory:
        for name in MANAGED_FILES:
            try:
                managed[name] = directory.file_digest(name)
            except FileNotFoundError:
                managed[name] = None
    executables = {}
    for name, command in commands.items():
        executable = command["argv"][0]
        if "/" in executable:
            resolved = (root / command["cwd"] / executable).resolve()
        else:
            # execvpe searches relative PATH entries after fchdir; fingerprint
            # that same location rather than the coordinator's own cwd.
            search_path = os.pathsep.join(str((root / command["cwd"] / entry).resolve())
                                          for entry in os.get_exec_path())
            found = shutil.which(executable, path=search_path)
            resolved = Path(found).resolve() if found else None
        if resolved and resolved.is_file():
            with resolved.open("rb") as handle:
                executables[name] = {"path": str(resolved), "sha256": hashlib.file_digest(handle, "sha256").hexdigest()}
        else:
            executables[name] = {"path": str(resolved) if resolved else None, "missing": True}
    environment = {key: value for key, value in os.environ.items() if key not in {"PWD", "OLDPWD", "SHLVL", "_"}}
    return {"repository": repository, "source_digest": source.hexdigest(),
            "config_digest": _digest(config), "managed_files": managed,
            "commands_digest": _digest(commands), "executables": executables,
            "environment_digest": _digest(environment),
            "runtime": {"python": sys.version, "executable": sys.executable,
                        "platform": platform.platform()}}


def _open_runs(stack: ExitStack, root: Path, *, create: bool) -> _Directory:
    directory = stack.enter_context(_Directory.root(root))
    state = stack.enter_context(directory.child(".gameskills", create=create))
    return stack.enter_context(state.child("runs", create=create))


def _save(directory: _Directory, record: dict) -> None:
    directory.write_json("record.json", {"record": record, "sha256": _digest(record)})


def _load(directory: _Directory) -> dict:
    try:
        envelope = json.loads(directory.read("record.json"))
        record = envelope["record"]
        if not isinstance(record, dict) or envelope["sha256"] != _digest(record):
            raise ValueError("record digest mismatch; evidence was modified or damaged")
        if record.get("schema_version") != SCHEMA_VERSION:
            raise ValueError("unsupported evidence schema")
        return record
    except (KeyError, TypeError, json.JSONDecodeError) as error:
        raise ValueError("invalid evidence record") from error


class _Resources:
    def __init__(self, common_dir: str):
        import fcntl
        self.fcntl = fcntl
        self.common_dir = common_dir
        # A fixed owner-scoped path coordinates distinct checkouts and processes.
        # TMPDIR is deliberately not used: per-process overrides would split locks.
        with _Directory.root(Path("/tmp").resolve()) as temporary:
            self.directory = temporary.child(f"gameskills-resources-{os.getuid()}", create=True)

    def close(self) -> None:
        self.directory.close()

    def acquire(self, resources: list[str]) -> list[int] | None:
        held = []
        try:
            for resource in sorted(resources):
                scope = self.common_dir if resource.startswith("project:") else "global"
                name = hashlib.sha256(_json([scope, resource])).hexdigest() + ".lock"
                fd = self.directory.open(name, create=True)
                try:
                    self.fcntl.flock(fd, self.fcntl.LOCK_EX | self.fcntl.LOCK_NB)
                except BlockingIOError:
                    os.close(fd)
                    self.release(held)
                    return None
                except BaseException:
                    os.close(fd)
                    raise
                held.append(fd)
            return held
        except BaseException:
            self.release(held)
            raise

    @staticmethod
    def release(held: list[int]) -> None:
        for fd in held:
            os.close(fd)


def _cleanup(process: subprocess.Popen) -> list[str]:
    """Clean descendants even when their leader already exited successfully."""
    errors = []
    for sig in (signal.SIGTERM, signal.SIGKILL):
        try:
            os.killpg(process.pid, sig)
        except ProcessLookupError:
            break
        except OSError as error:
            errors.append(str(error))
        if sig == signal.SIGTERM:
            time.sleep(0.1)
    try:
        process.wait(timeout=2)
    except (OSError, subprocess.TimeoutExpired) as error:
        errors.append(str(error))
    return errors


def _execute(root: Path, directory: _Directory, name: str, command: dict, stop: threading.Event,
             inherited_locks: tuple[int, ...] = ()) -> dict:
    result = {"status": "error", "argv": command["argv"], "cwd": command["cwd"],
              "requires": command["requires"], "resources": command["resources"],
              "timeout_seconds": command["timeout_seconds"], "exit_code": None,
              "started_at": _now()}
    started = time.monotonic()
    process = None
    try:
        with ExitStack() as stack:
            cwd = stack.enter_context(_cwd(root, command["cwd"]))
            stdout = stack.enter_context(os.fdopen(directory.open(name + ".stdout.log", create=True, exclusive=True), "wb"))
            stderr = stack.enter_context(os.fdopen(directory.open(name + ".stderr.log", create=True, exclusive=True), "wb"))
            if stop.is_set():
                result["status"] = "interrupted"
            else:
                # fchdir in a fresh interpreter anchors cwd against ancestor
                # rename/symlink races without thread-unsafe preexec_fn. exec
                # replaces the launcher, preserving the process group identity.
                process = subprocess.Popen([sys.executable, "-c", _EXEC, str(cwd.fd), *command["argv"]],
                                           pass_fds=(cwd.fd, *inherited_locks), start_new_session=True,
                                           stdout=stdout, stderr=stderr, stdin=subprocess.DEVNULL)
                result["pid"] = process.pid
                while process.poll() is None:
                    if stop.wait(0.03):
                        result["status"] = "interrupted"
                        break
                    if time.monotonic() - started >= command["timeout_seconds"]:
                        result["status"] = "timeout"
                        break
                else:
                    result["status"] = "passed" if process.returncode == 0 else "failed"
                cleanup = _cleanup(process)
                result["exit_code"] = process.returncode
                if cleanup:
                    result["cleanup_errors"] = cleanup
                    if result["status"] == "passed":
                        result["status"] = "error"
            for handle in (stdout, stderr):
                handle.flush()
                os.fsync(handle.fileno())
    except BaseException as error:
        result["error"] = f"{type(error).__name__}: {error}"
        if result["status"] == "passed":
            result["status"] = "error"
        if process is not None:
            cleanup = _cleanup(process)
            result["exit_code"] = process.returncode
            if cleanup:
                result["cleanup_errors"] = cleanup
    result["finished_at"] = _now()
    result["duration_seconds"] = round(time.monotonic() - started, 6)
    for stream in ("stdout", "stderr"):
        filename = f"{name}.{stream}.log"
        try:
            result[stream] = {"file": filename, "sha256": directory.file_digest(filename)}
        except (OSError, ValueError) as error:
            result["status"] = "error"
            result[f"{stream}_error"] = str(error)
    return result


def _validate_record(directory: _Directory, record: dict, root: Path, config: dict) -> dict:
    reasons = []
    if record.get("status") != "passed":
        reasons.append(f"run status is {record.get('status', 'missing')}")
    try:
        commands, order = _graph(root, config, record["selected"])
        current = _identity(root, config, commands)
        if current != record["identity"]:
            reasons.append("repository, source, configuration, command, or environment inputs changed")
        if record.get("final_identity") != record["identity"]:
            reasons.append("inputs changed during execution or final identity is unavailable")
        if record.get("order") != order or set(record.get("results", {})) != set(order):
            reasons.append("command graph/results are incomplete")
        for name in order:
            result = record.get("results", {}).get(name, {})
            if result.get("status") != "passed" or result.get("exit_code") != 0:
                reasons.append(f"{name}: no observed zero-exit completion")
            if result.get("argv") != commands[name]["argv"] or result.get("cwd") != commands[name]["cwd"]:
                reasons.append(f"{name}: recorded command does not match current command")
            for stream in ("stdout", "stderr"):
                output = result.get(stream, {})
                filename = f"{name}.{stream}.log"
                if output.get("file") != filename or directory.file_digest(filename) != output.get("sha256"):
                    reasons.append(f"{name}: {stream} output digest mismatch")
    except (OSError, ValueError, KeyError, TypeError) as error:
        reasons.append(f"cannot validate evidence: {error}")
    return {"ok": not reasons, "status": "valid" if not reasons else "invalid",
            "run_id": record.get("run_id"), "recorded_status": record.get("status"),
            "reasons": reasons, "claim": "observed command checks only"}


def _run(root: Path, config: dict, args: argparse.Namespace) -> dict:
    commands, order = _graph(root, config, args.names)
    dispatch = config.get("dispatch", {})
    if not isinstance(dispatch, dict):
        raise ValueError("dispatch must be a mapping")
    cap = dispatch.get("max_workers", 5)
    if isinstance(cap, bool) or not isinstance(cap, int) or cap < 1:
        raise ValueError("dispatch.max_workers must be a positive integer")
    workers = args.max_workers if args.max_workers is not None else cap
    if not 1 <= workers <= cap:
        raise ValueError(f"--max-workers must be between 1 and configured cap {cap}")
    resource_wait = _number(args.resource_wait_seconds, "--resource-wait-seconds")
    identity = _identity(root, config, commands)
    with ExitStack() as stack:
        runs = _open_runs(stack, root, create=True)
        if args.resume:
            if not RUN_ID.fullmatch(args.resume):
                raise ValueError("invalid resume run ID")
            with runs.child(args.resume) as previous_directory:
                previous = _load(previous_directory)
                if previous.get("run_id") != args.resume:
                    raise ValueError("resume record has the wrong run identity")
                if previous.get("identity") != identity or previous.get("selected") != sorted(set(args.names)):
                    raise ValueError("cannot resume: source or execution inputs changed; start a new run")
                # A crashed writer may leave running state. An OS lock, not a
                # recorded PID, determines whether its runner is still active.
                import fcntl
                active_fd = previous_directory.open("active.lock")
                try:
                    try:
                        fcntl.flock(active_fd, fcntl.LOCK_EX | fcntl.LOCK_NB)
                    except BlockingIOError as error:
                        raise ValueError("cannot resume an active run") from error
                finally:
                    os.close(active_fd)
        run_id = uuid.uuid4().hex
        directory = stack.enter_context(runs.child(run_id, create=True, exclusive=True))
        import fcntl
        active_fd = directory.open("active.lock", create=True, exclusive=True)
        stack.callback(os.close, active_fd)
        fcntl.flock(active_fd, fcntl.LOCK_EX | fcntl.LOCK_NB)
        record = {"schema_version": SCHEMA_VERSION, "run_id": run_id, "status": "running",
                  "selected": sorted(set(args.names)), "order": order, "commands": commands,
                  "identity": identity, "results": {}, "started_at": _now(),
                  "max_workers": workers, "resource_wait_seconds": resource_wait,
                  "resumed_from": args.resume, "claim": "observed command checks only"}
        _save(directory, record)
        stop = threading.Event()
        received = []

        def interrupt(signum: int, unused: object) -> None:
            received.append(signum)
            stop.set()

        if threading.current_thread() is threading.main_thread():
            for signum in (signal.SIGINT, signal.SIGTERM):
                prior = signal.signal(signum, interrupt)
                stack.callback(signal.signal, signum, prior)
        try:
            resources = _Resources(identity["repository"]["common_dir"])
        except (OSError, ValueError) as error:
            record.update(status="failed", runner_error=str(error), finished_at=_now())
            for name in order:
                record["results"][name] = {"status": "skipped", "reason": "resource coordinator unavailable", "exit_code": None}
            _save(directory, record)
            return {"ok": False, "status": "failed", "run_id": run_id, "results": record["results"],
                    "error": str(error), "claim": record["claim"]}
        stack.callback(resources.close)
        pending = set(order)
        ready_since: dict[str, float] = {}
        active = {}
        executor = ThreadPoolExecutor(max_workers=workers)
        try:
            while pending or active:
                if stop.is_set():
                    for name in order:
                        if name in pending:
                            record["results"][name] = {"status": "skipped", "reason": "runner interrupted before start", "exit_code": None}
                    pending.clear()
                for name in order:
                    if name not in pending or len(active) >= workers:
                        continue
                    command = commands[name]
                    if any(dependency not in record["results"] for dependency in command["requires"]):
                        continue
                    failed = [dependency for dependency in command["requires"] if record["results"][dependency]["status"] != "passed"]
                    if failed:
                        record["results"][name] = {"status": "skipped", "reason": "dependency did not pass", "dependencies": failed, "exit_code": None}
                        pending.remove(name)
                        continue
                    ready_since.setdefault(name, time.monotonic())
                    held = resources.acquire(command["resources"])
                    if held is None:
                        if time.monotonic() - ready_since[name] >= resource_wait:
                            record["results"][name] = {"status": "skipped", "reason": "resource wait expired", "resources": command["resources"], "exit_code": None}
                            pending.remove(name)
                        continue
                    pending.remove(name)
                    # Commands retain the active/resource locks if the runner
                    # suffers SIGKILL; an immediate resume cannot overlap that
                    # surviving leader. Normal cleanup still belongs here.
                    future = executor.submit(_execute, root, directory, name, command, stop, (active_fd, *held))
                    active[future] = (name, held)
                if active:
                    completed, _ = wait(active, timeout=0.03, return_when=FIRST_COMPLETED)
                    for future in sorted(completed, key=lambda item: order.index(active[item][0])):
                        name, held = active.pop(future)
                        try:
                            record["results"][name] = future.result()
                        finally:
                            resources.release(held)
                        _save(directory, record)
                elif pending:
                    stop.wait(0.03)
        except BaseException as error:
            stop.set()
            record["runner_error"] = f"{type(error).__name__}: {error}"
        finally:
            executor.shutdown(wait=True, cancel_futures=False)
            for future, (name, held) in active.items():
                try:
                    record["results"][name] = future.result()
                except BaseException as error:
                    record["results"][name] = {"status": "error", "error": str(error), "exit_code": None}
                finally:
                    resources.release(held)
            for name in order:
                record["results"].setdefault(name, {"status": "skipped", "reason": "runner did not complete", "exit_code": None})
        try:
            record["final_identity"] = _identity(root, config, commands)
        except (OSError, ValueError) as error:
            record["identity_error"] = str(error)
        if stop.is_set() and "runner_error" not in record:
            record["status"] = "interrupted"
        elif any(result["status"] != "passed" for result in record["results"].values()) or "runner_error" in record:
            record["status"] = "failed"
        elif record.get("final_identity") != identity:
            record["status"] = "stale"
        else:
            record["status"] = "passed"
        record["signals"] = received
        record["finished_at"] = _now()
        _save(directory, record)
        response = {"ok": record["status"] == "passed", "status": record["status"], "run_id": run_id,
                    "record_path": str(root / ".gameskills" / "runs" / run_id / "record.json"),
                    "results": record["results"], "claim": record["claim"]}
        for key in ("runner_error", "identity_error"):
            if key in record:
                response[key] = record[key]
        return response


class _Parser(argparse.ArgumentParser):
    def error(self, message: str) -> None:
        raise ValueError(message)


def main(argv: list[str], root: Path, config: dict) -> dict:
    """Route `run ...` and `evidence list|show|validate ...` for the shared CLI.

    A failed observation returns ok=False. Invalid requests raise ValueError or
    OSError for the outer CLI to report. No entry point accepts a success claim.
    """
    _supported()
    parser = _Parser(prog="gameskills")
    modes = parser.add_subparsers(dest="mode", required=True)
    run = modes.add_parser("run")
    run.add_argument("names", nargs="+")
    run.add_argument("--max-workers", type=int)
    run.add_argument("--resource-wait-seconds", type=float, default=30)
    run.add_argument("--resume")
    evidence = modes.add_parser("evidence")
    operations = evidence.add_subparsers(dest="operation", required=True)
    operations.add_parser("list")
    for operation in ("show", "validate"):
        operations.add_parser(operation).add_argument("run_id")
    args = parser.parse_args(argv)
    root = root.resolve(strict=True)
    _repository(root)
    if args.mode == "run":
        return _run(root, config, args)
    if args.operation != "list" and not RUN_ID.fullmatch(args.run_id):
        raise ValueError("invalid run ID")
    with ExitStack() as stack:
        try:
            runs = _open_runs(stack, root, create=False)
        except FileNotFoundError:
            if args.operation == "list":
                return {"ok": True, "status": "listed", "runs": []}
            raise ValueError("no command evidence exists") from None
        if args.operation == "list":
            entries = []
            for name in sorted(os.listdir(runs.fd)):
                if not RUN_ID.fullmatch(name):
                    continue
                try:
                    with runs.child(name) as directory:
                        record = _load(directory)
                        entries.append({"run_id": name, "recorded_status": record.get("status"),
                                        "started_at": record.get("started_at"), "validated": False})
                except (OSError, ValueError) as error:
                    entries.append({"run_id": name, "recorded_status": "invalid", "error": str(error), "validated": False})
            return {"ok": True, "status": "listed", "runs": entries}
        try:
            with runs.child(args.run_id) as directory:
                record = _load(directory)
                if record.get("run_id") != args.run_id:
                    raise ValueError("record has the wrong run identity")
                result = _validate_record(directory, record, root, config)
                if args.operation == "show":
                    result["record"] = record
                return result
        except (OSError, ValueError) as error:
            return {"ok": False, "status": "invalid", "run_id": args.run_id,
                    "reasons": [str(error)], "claim": "observed command checks only"}
