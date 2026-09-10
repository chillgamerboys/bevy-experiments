#!/usr/bin/env python3
"""Validate the candidate's structural contracts, not native or agent behavior.

Uses only the standard library. Required frontmatter scalars support ordinary,
quoted and block YAML strings; this is not a general YAML/schema implementation.
"""
from __future__ import annotations

import argparse
import json
import re
from pathlib import Path
from urllib.parse import unquote, urlsplit

EXPECTED_SKILLS = {
    "gameskills": ("setup", "plan", "dispatch", "debug", "test", "playtest", "review",
                   "update-docs", "create-pr", "audit-pr", "merge-pr", "release"),
    "gameskills-ui": ("build-ui", "verify-ui"),
    "gameskills-turn-based": ("model-rules",),
    "gameskills-multiplayer": ("design-multiplayer", "verify-multiplayer"),
    "gameskills-maintainer": ("evolve-gamekit", "author-skill", "evaluate-skills"),
    "gameskills-bevy-contrib": ("prepare-contribution",),
}
LOGICAL_SKILLS = {f"{package}:{skill}" for package, skills in EXPECTED_SKILLS.items() for skill in skills}
REQUIRED_SCENARIO_TAGS = {"routing", "bounded-dispatch", "economical-roles", "stale-evidence",
                          "inject-preservation", "optional-absence", "human-upstream"}
NAME = re.compile(r"[a-z0-9]+(?:-[a-z0-9]+)*\Z")
VERSION = re.compile(r"\d+\.\d+\.\d+(?:-[0-9A-Za-z.-]+)?(?:\+[0-9A-Za-z.-]+)?\Z")
NOTICE = ("Structural checks and scenario fixtures do not prove native installation, skill "
          "selection, forward behavior, review acceptance, or measured cost savings.")


def _object(pairs: list[tuple[str, object]]) -> dict:
    result = {}
    for key, value in pairs:
        if key in result:
            raise ValueError(f"duplicate JSON key {key!r}")
        result[key] = value
    return result


def _json(path: Path, failures: list[str]) -> dict:
    try:
        value = json.loads(path.read_text(encoding="utf-8"), object_pairs_hook=_object)
        if not isinstance(value, dict):
            raise ValueError("expected an object")
        return value
    except (OSError, UnicodeError, ValueError) as exc:
        failures.append(f"{path}: {exc}")
        return {}


def _strings(value: object) -> bool:
    return isinstance(value, list) and all(isinstance(item, str) and item.strip() for item in value)


def _descriptive(value: object) -> bool:
    return isinstance(value, str) and 24 <= len(value.strip()) <= 1024 and len(value.split()) >= 4


def _inventory(value: object, expected: set[str], label: str, failures: list[str]) -> None:
    if not _strings(value):
        failures.append(f"{label}: expected a string list")
    elif len(value) != len(set(value)) or set(value) != expected:
        failures.append(f"{label}: inventory mismatch; expected {sorted(expected)}, got {value}")


def _scalar(raw: str, continuation: list[str]) -> str:
    raw = raw.strip()
    if raw in {">", ">-", ">+", "|", "|-", "|+"}:
        value = "\n".join(line.strip() for line in continuation).strip()
        return " ".join(value.split()) if raw.startswith(">") else value
    if raw.startswith('"'):
        value = json.loads(raw)
        if not isinstance(value, str):
            raise ValueError("required field must be a string")
        return value
    if raw.startswith("'"):
        if not raw.endswith("'") or len(raw) < 2:
            raise ValueError("unterminated single-quoted string")
        return raw[1:-1].replace("''", "'")
    if not raw or raw[0] in "[{&*!" or raw in {"null", "true", "false", "~"}:
        raise ValueError("required field must be a string")
    if re.search(r":\s", raw):
        raise ValueError("quote a required string containing colon-space")
    return re.split(r"\s+#", raw, maxsplit=1)[0].strip()


def frontmatter(content: str) -> tuple[dict[str, str], str]:
    """Read the required scalars without claiming to validate arbitrary YAML."""
    lines = content.splitlines()
    if not lines or lines[0] != "---":
        raise ValueError("missing frontmatter opening delimiter")
    try:
        end = lines.index("---", 1)
    except ValueError as exc:
        raise ValueError("missing frontmatter closing delimiter") from exc
    fields: dict[str, tuple[str, list[str]]] = {}
    current = None
    for line in lines[1:end]:
        if not line.strip() or line.lstrip().startswith("#"):
            continue
        if line.startswith("\t"):
            raise ValueError("tab-indented frontmatter is unsupported")
        if line.startswith(" "):
            if current is None:
                raise ValueError("unexpected indented frontmatter")
            fields[current][1].append(line)
            continue
        match = re.fullmatch(r"([A-Za-z][A-Za-z0-9_-]*):(?:\s+(.*))?", line)
        if not match:
            raise ValueError(f"invalid top-level frontmatter line: {line}")
        current = match.group(1)
        if current in fields:
            raise ValueError(f"duplicate frontmatter key {current}")
        fields[current] = (match.group(2) or "", [])
    required = {}
    for key in ("name", "description"):
        if key not in fields:
            raise ValueError(f"missing required frontmatter {key}")
        required[key] = _scalar(*fields[key])
    return required, "\n".join(lines[end + 1:]).strip()


def _without_fences(content: str) -> str:
    lines = []
    fence = None
    for line in content.splitlines():
        match = re.match(r"^\s{0,3}(`{3,}|~{3,})", line)
        if match:
            delimiter = match.group(1)
            if fence is None:
                fence = delimiter
            elif delimiter[0] == fence[0] and len(delimiter) >= len(fence):
                fence = None
            continue
        if fence is None:
            lines.append(line)
    return "\n".join(lines)


def _links(content: str) -> list[str]:
    """Find ordinary inline/image and reference-definition Markdown targets."""
    text = _without_fences(content)
    targets = []
    pattern = r"\]\(\s*(<[^>\n]+>|[^\s)]+)(?:\s+['\"][^\n]*?['\"])?\s*\)"
    targets.extend(match.group(1).strip("<>") for match in re.finditer(pattern, text))
    definitions = r"^\s{0,3}\[[^\]\n]+\]:\s*(<[^>\n]+>|\S+)"
    targets.extend(match.group(1).strip("<>") for match in re.finditer(definitions, text, re.MULTILINE))
    return targets


def _local_target(package: Path, source: Path, raw: str, failures: list[str]) -> Path | None:
    try:
        parsed = urlsplit(raw)
    except ValueError:
        failures.append(f"{source}: malformed link {raw!r}")
        return None
    if parsed.scheme in {"http", "https", "mailto"}:
        return None
    if parsed.scheme or parsed.netloc:
        failures.append(f"{source}: unsupported nonportable local link {raw!r}")
        return None
    path = unquote(parsed.path)
    if not path:
        return None
    if path.startswith("/") or "\\" in path or "\x00" in path:
        failures.append(f"{source}: nonportable local path {raw!r}")
        return None
    try:
        target = (source.parent / path).resolve()
    except (OSError, RuntimeError, ValueError) as exc:
        failures.append(f"{source}: cannot resolve {raw!r}: {exc}")
        return None
    if not target.is_relative_to(package.resolve()):
        failures.append(f"{source}: link escapes native package: {raw}")
        return None
    if not target.exists():
        failures.append(f"{source}: missing package-local link: {raw}")
        return None
    return target


def _skill(path: Path, name: str, failures: list[str]) -> None:
    try:
        content = path.read_text(encoding="utf-8")
        fields, body = frontmatter(content)
        if fields["name"] != name or not NAME.fullmatch(fields["name"]) or len(name) > 64:
            failures.append(f"{path}: frontmatter name must match skill directory {name}")
        if not _descriptive(fields["description"]):
            failures.append(f"{path}: description must be a descriptive string of 24–1024 characters")
        if len(body.split()) < 12:
            failures.append(f"{path}: skill body is empty or unfinished")
        if re.search(r"\b(?:TODO|FIXME|TBD)\b|\[INSERT[^\]]*\]", content):
            failures.append(f"{path}: unfinished scaffold placeholder")
    except (OSError, UnicodeError, ValueError) as exc:
        failures.append(f"{path}: {exc}")
    metadata = path.parent / "agents/openai.yaml"
    if metadata.exists():
        try:
            text = metadata.read_text(encoding="utf-8")
            if not re.search(r"(?m)^interface:\s*$", text):
                failures.append(f"{metadata}: expected an interface mapping")
            for field in ("display_name", "short_description"):
                if not re.search(rf"(?m)^  {field}:\s*\S", text):
                    failures.append(f"{metadata}: missing interface {field}")
            prompt = re.search(r"(?m)^  default_prompt:\s*(.+)$", text)
            if prompt and f"${name}" not in prompt.group(1) and f":{name}" not in prompt.group(1):
                failures.append(f"{metadata}: default_prompt does not reference this skill")
        except (OSError, UnicodeError) as exc:
            failures.append(f"{metadata}: {exc}")


def _manifests(package: Path, name: str, version: object, failures: list[str]) -> None:
    for client in ("codex", "claude"):
        path = package / f".{client}-plugin/plugin.json"
        data = _json(path, failures)
        if not data:
            continue
        if data.get("name") != name:
            failures.append(f"{path}: package name mismatch")
        if data.get("version") != version:
            failures.append(f"{path}: version does not match catalog")
        if not _descriptive(data.get("description")):
            failures.append(f"{path}: missing descriptive package description")
        if "author" in data and (not isinstance(data["author"], dict) or not isinstance(data["author"].get("name"), str)):
            failures.append(f"{path}: author must be an object with a name")
        skills = data.get("skills", "./skills/" if client == "claude" else None)
        if not isinstance(skills, str):
            failures.append(f"{path}: this candidate must discover its package-local skills directory")
        else:
            target = _local_target(package, package / "manifest-root", skills, failures)
            if target != (package / "skills").resolve():
                failures.append(f"{path}: skills must resolve to this package's skills directory")
        if client == "codex" and "interface" in data:
            interface = data["interface"]
            if not isinstance(interface, dict):
                failures.append(f"{path}: interface must be an object")
            else:
                for key in ("displayName", "shortDescription", "longDescription", "developerName", "category", "defaultPrompt"):
                    if key in interface and (not isinstance(interface[key], str) or not interface[key].strip()):
                        failures.append(f"{path}: interface.{key} must be a nonempty string")
                if "capabilities" in interface and not _strings(interface["capabilities"]):
                    failures.append(f"{path}: interface.capabilities must be a string list")


def _marketplaces(root: Path, version: object, failures: list[str]) -> None:
    for client, relative in (("codex", ".agents/plugins/marketplace.json"), ("claude", ".claude-plugin/marketplace.json")):
        path = root / relative
        data = _json(path, failures)
        entries = data.get("plugins")
        if not isinstance(entries, list) or not all(isinstance(entry, dict) for entry in entries):
            failures.append(f"{path}: plugins must be an object list")
            continue
        _inventory([entry.get("name") for entry in entries], set(EXPECTED_SKILLS), f"{path} packages", failures)
        for entry in entries:
            name = entry.get("name")
            if not isinstance(name, str) or name not in EXPECTED_SKILLS:
                continue
            source = entry.get("source")
            if client == "codex":
                if not isinstance(source, dict) or source.get("source") != "local":
                    failures.append(f"{path}: {name} must use a local package source")
                    continue
                source = source.get("path")
            if not isinstance(source, str):
                failures.append(f"{path}: {name} missing source path")
                continue
            target = _local_target(root, root / "marketplace-root", source, failures)
            if target != (root / "plugins" / name).resolve():
                failures.append(f"{path}: {name} source does not resolve to its package")
            if "version" in entry and entry["version"] != version:
                failures.append(f"{path}: {name} version differs from catalog")


def validate_scenarios(path: Path) -> list[str]:
    """Check evaluation inputs/rubrics; never score behavior from fixture presence."""
    failures: list[str] = []
    data = _json(path, failures)
    if type(data.get("schema_version")) is not int or data["schema_version"] != 1 or data.get("evidence_status") != "unexecuted-rubrics":
        failures.append(f"{path}: require schema_version 1 and unexecuted-rubrics evidence_status")
    scenarios = data.get("scenarios")
    if not isinstance(scenarios, list) or not scenarios:
        return failures + [f"{path}: missing scenarios"]
    ids = set()
    tags = set()
    for scenario in scenarios:
        if not isinstance(scenario, dict):
            failures.append(f"{path}: scenario must be an object")
            continue
        identity = scenario.get("id")
        label = f"{path} scenario {identity!r}"
        if not isinstance(identity, str) or not NAME.fullmatch(identity) or identity in ids:
            failures.append(f"{label}: invalid or duplicate id")
        else:
            ids.add(identity)
        for key in ("prompt", "fixture", "allowed_actions"):
            if not isinstance(scenario.get(key), str) or not scenario[key].strip():
                failures.append(f"{label}: missing {key}")
        if scenario.get("split") != "held-out":
            failures.append(f"{label}: split must be held-out")
        if not _strings(scenario.get("tags")):
            failures.append(f"{label}: tags must be a string list")
        else:
            tags.update(scenario["tags"])
        routing = scenario.get("routing")
        if not isinstance(routing, dict):
            failures.append(f"{label}: missing routing rubric")
        else:
            primary = routing.get("primary")
            if primary is not None and (not isinstance(primary, str) or primary not in LOGICAL_SKILLS):
                failures.append(f"{label}: unknown primary skill {primary!r}")
            for key in ("allowed", "avoid"):
                values = routing.get(key)
                if not _strings(values) or any(value not in LOGICAL_SKILLS for value in values):
                    failures.append(f"{label}: invalid routing {key}")
            if _strings(routing.get("allowed")) and _strings(routing.get("avoid")):
                if set(routing["allowed"]) & set(routing["avoid"]) or primary in routing["avoid"]:
                    failures.append(f"{label}: contradictory routing rubric")
        rubric = scenario.get("rubric")
        if not isinstance(rubric, dict):
            failures.append(f"{label}: missing observation rubric")
        else:
            for key in ("must_observe", "must_not_observe", "evidence"):
                if not _strings(rubric.get(key)) or not rubric[key]:
                    failures.append(f"{label}: missing nonempty rubric {key}")
    if not REQUIRED_SCENARIO_TAGS <= tags:
        failures.append(f"{path}: missing scenario coverage tags {sorted(REQUIRED_SCENARIO_TAGS - tags)}")
    return failures


def validate(root: Path) -> list[str]:
    """Return actionable structural failures for one repository candidate."""
    root = root.resolve()
    failures: list[str] = []
    catalog_path = root / "plugins/gameskills/catalog.json"
    catalog = _json(catalog_path, failures)
    if type(catalog.get("schema_version")) is not int or catalog["schema_version"] != 1:
        failures.append(f"{catalog_path}: schema_version must be 1")
    version = catalog.get("version")
    if not isinstance(version, str) or not VERSION.fullmatch(version):
        failures.append(f"{catalog_path}: version must have semantic-version form")
    packages = catalog.get("packages")
    if not isinstance(packages, dict):
        packages = {}
    _inventory(list(packages), set(EXPECTED_SKILLS), f"{catalog_path} packages", failures)
    plugin_root = root / "plugins"
    actual = [path.name for path in plugin_root.iterdir() if path.is_dir()] if plugin_root.is_dir() else []
    _inventory(actual, set(EXPECTED_SKILLS), "native package directories", failures)
    for name, expected in EXPECTED_SKILLS.items():
        package = plugin_root / name
        definition = packages.get(name)
        if not isinstance(definition, dict):
            failures.append(f"{catalog_path}: missing package definition {name}")
            definition = {}
        _inventory(definition.get("skills"), set(expected), f"catalog {name} skills", failures)
        _inventory(definition.get("requires"), set() if name == "gameskills" else {"gameskills"}, f"catalog {name} requirements", failures)
        if not _descriptive(definition.get("description")):
            failures.append(f"catalog {name}: missing descriptive package description")
        if package.is_symlink():
            failures.append(f"{package}: native package root cannot be a symlink")
        skill_root = package / "skills"
        actual_skills = [path.name for path in skill_root.iterdir() if path.is_dir()] if skill_root.is_dir() else []
        _inventory(actual_skills, set(expected), f"{name} native skill directories", failures)
        found = {path.parent.relative_to(skill_root).as_posix() for path in skill_root.rglob("SKILL.md")} if skill_root.is_dir() else set()
        if found != set(expected):
            failures.append(f"{name}: discovered SKILL.md inventory mismatch: {sorted(found)}")
        for skill in expected:
            _skill(skill_root / skill / "SKILL.md", skill, failures)
        _manifests(package, name, version, failures)
        for path in package.rglob("*"):
            if path.is_symlink():
                try:
                    if not path.resolve(strict=True).is_relative_to(package.resolve()):
                        failures.append(f"{path}: symlink escapes native package")
                        continue
                except (OSError, RuntimeError, ValueError) as exc:
                    failures.append(f"{path}: unresolvable package symlink: {exc}")
                    continue
            if path.suffix == ".md" and path.is_file():
                try:
                    for link in _links(path.read_text(encoding="utf-8")):
                        _local_target(package, path, link, failures)
                except (OSError, UnicodeError) as exc:
                    failures.append(f"{path}: {exc}")
    _marketplaces(root, version, failures)
    failures.extend(validate_scenarios(root / "skills/tests/gameskills-scenarios.json"))
    return failures


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--root", type=Path, default=Path(__file__).resolve().parents[2])
    parser.add_argument("--json", action="store_true", help="emit machine-readable structural results")
    args = parser.parse_args(argv)
    failures = validate(args.root)
    core_count = len(EXPECTED_SKILLS["gameskills"])
    optional_count = len(LOGICAL_SKILLS) - core_count
    if args.json:
        print(json.dumps({"ok": not failures, "structural_only": True, "packages": len(EXPECTED_SKILLS),
                          "skills": len(LOGICAL_SKILLS), "core_skills": core_count,
                          "optional_skills": optional_count, "failures": failures, "notice": NOTICE}, indent=2))
    else:
        for failure in failures:
            print(f"FAIL {failure}")
        print(f"{'Failed' if failures else 'Passed'} structural validation: {core_count} core skills + {optional_count} optional skills in {len(EXPECTED_SKILLS)} packages.")
        print(NOTICE)
    return int(bool(failures))


if __name__ == "__main__":
    raise SystemExit(main())
