#!/usr/bin/env python3
"""Validate canonical Bevy skill structure and cross-client rendering."""

from __future__ import annotations

import json
import re
import sys
from pathlib import Path

from skills_tool import CLIENTS, SKILLS, default_source_root, render


FRONTMATTER = re.compile(r"\A---\nname: ([^\n]+)\ndescription: ([^\n]+)\n---\n")


def main() -> int:
    """Return nonzero after printing every validation failure."""

    root = default_source_root()
    failures: list[str] = []
    for name in SKILLS:
        skill_root = root / "source" / name
        content = (skill_root / "SKILL.md").read_text()
        match = FRONTMATTER.match(content)
        if match is None:
            failures.append(f"{name}: invalid frontmatter")
            continue
        if match.group(1) != name:
            failures.append(f"{name}: frontmatter name mismatch")
        description = match.group(2)
        if "Use " not in description or "Do not use" not in description:
            failures.append(f"{name}: description lacks positive or negative triggers")
        if "TODO" in content:
            failures.append(f"{name}: contains TODO placeholder")
        for link in re.findall(r"\]\(([^)]+)\)", content):
            if "://" not in link and not link.startswith("#"):
                if not (skill_root / link.split("#", 1)[0]).is_file():
                    failures.append(f"{name}: missing referenced file {link}")
        metadata = (skill_root / "agents/openai.yaml").read_text()
        if f"${name}" not in metadata:
            failures.append(f"{name}: default prompt does not name the skill")

    fixtures = json.loads((root / "tests/trigger-fixtures.json").read_text())
    for name in SKILLS:
        fixture = fixtures.get(name, {})
        if not fixture.get("should_trigger") or not fixture.get("should_not_trigger"):
            failures.append(f"{name}: missing positive or negative trigger fixtures")
        for category in ("should_trigger", "should_not_trigger"):
            examples = fixture.get(category)
            if not isinstance(examples, list) or any(not isinstance(item, str) or not item.strip() for item in examples):
                failures.append(f"{name}: invalid {category} examples")
        if set(fixture.get("should_trigger", [])) & set(fixture.get("should_not_trigger", [])):
            failures.append(f"{name}: contradictory trigger fixtures")

    rendered = render(root)
    for name in SKILLS:
        codex = rendered.get(Path(f".agents/skills/{name}/SKILL.md"))
        claude = rendered.get(Path(f".claude/skills/{name}/SKILL.md"))
        if codex != claude:
            failures.append(f"{name}: Codex and Claude SKILL.md differ")
    if tuple(CLIENTS) != ("codex", "claude"):
        failures.append("client set changed unexpectedly")

    if failures:
        for failure in failures:
            print(f"FAIL {failure}")
        return 1
    print(f"structurally validated {len(SKILLS)} skills for {len(CLIENTS)} clients; trigger behavior requires agent evaluation")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
