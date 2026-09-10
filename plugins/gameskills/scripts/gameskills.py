#!/usr/bin/env python3
"""Invoke the installed GameSkills core without additional Python dependencies."""

from __future__ import annotations

import argparse
import json
from pathlib import Path
import subprocess
import sys

CORE = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(CORE / "runtime"))

from gameskills_runtime import config, packaging  # noqa: E402


def main(argv=None) -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--root", type=Path, default=Path.cwd())
    parser.add_argument("command", choices=["catalog", "status", "config", "setup", "bundle",
                                           "native", "plan", "queue", "run", "evidence"])
    parser.add_argument("arguments", nargs=argparse.REMAINDER)
    args = parser.parse_args(argv)
    try:
        root = config.repository(args.root)
        index = packaging.catalog(CORE)
        if args.command == "catalog":
            result = {"ok": True, **index}
        elif args.command == "setup":
            result = packaging.setup(root, CORE, args.arguments)
        elif args.command == "bundle":
            child = argparse.ArgumentParser(prog="gameskills bundle")
            child.add_argument("--source", type=Path, default=CORE.parents[1])
            child.add_argument("--out", type=Path, required=True)
            child.add_argument("--revision", required=True)
            child.add_argument("--packages", nargs="+", default=["gameskills"])
            options = child.parse_args(args.arguments)
            result = packaging.bundle(options.source.resolve(), options.out.absolute(),
                                      options.revision, options.packages)
        else:
            identity = packaging.status(root, CORE)
            configured = config.load(root, set(index["packages"]))
            if args.command == "status":
                result = identity
            elif args.command == "config":
                result = {"ok": True, "configuration": configured}
            elif args.command == "native":
                child = argparse.ArgumentParser(prog="gameskills native")
                child.add_argument("client", choices=["codex", "claude"])
                child.add_argument("--launch", action="store_true")
                child.add_argument("--verify", action="store_true", help="verify Codex native skill discovery without a model turn")
                child.add_argument("args", nargs=argparse.REMAINDER)
                options = child.parse_args(args.arguments)
                if options.client not in configured["clients"]:
                    raise ValueError("native client is not selected in project configuration")
                rest = options.args
                # REMAINDER intentionally reserves arguments after '--' for the native client.
                launch = options.launch or (rest and rest[0] == "--launch")
                verify = options.verify or (rest and rest[0] == "--verify")
                if rest and rest[0] in {"--launch", "--verify"}:
                    rest = rest[1:]
                if rest and rest[0] == "--":
                    rest = rest[1:]
                command = packaging.native_argv(Path(identity["bundle"]), options.client, rest)
                if verify and (options.client != "codex" or rest):
                    raise ValueError("native --verify currently supports Codex discovery without additional client arguments")
                observed = None
                if (launch or verify) and options.client == "codex":
                    from gameskills_runtime.native import activate_codex
                    observed = activate_codex(Path(identity["bundle"]), root)
                if launch:
                    return subprocess.run(command, cwd=root, check=False).returncode
                result = ({"ok": True, "discovery": observed} if verify else
                          {"ok": True, "argv": command, "cwd": str(root),
                           "activation": "use native --launch; Codex first verifies discovery via its app-server",
                           "user_configuration_writes": False})
            elif args.command in {"plan", "queue"}:
                from gameskills_runtime import workflow
                result = workflow.main([args.command, *args.arguments], root, configured)
            else:
                from gameskills_runtime import runner
                result = runner.main([args.command, *args.arguments], root, configured)
        print(json.dumps(result, indent=2, sort_keys=True))
        return 0 if result.get("ok", True) else 1
    except (ValueError, OSError, subprocess.CalledProcessError) as error:
        print(json.dumps({"ok": False, "error": str(error)}))
        return 2


if __name__ == "__main__":
    raise SystemExit(main())
