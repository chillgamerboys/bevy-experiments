---
name: review
description: Evaluate a Bevy design or actual code/documentation change for actionable correctness and architecture findings before or after a PR exists. Use for review judgment; audit-pr assembles acceptance evidence for a specific PR.
---

# Review the actual change

Read [project context](../../references/project-context.md) and relevant
[Bevy craft](../../references/bevy-craft.md). Record scope, base/HEAD and dirty
state, or the design artifact identity. Read the authoritative implementation and
its callers/tests beyond the diff where a contract crosses those boundaries.

Select lenses that can expose material failures: authority/dependency direction,
error states, edge/terminal behavior, determinism/persistence, Bevy scheduling and
deferred work, feature/config wiring, compatibility and evidence altitude. Load
architecture, GameKit and selected specialist references only as needed. Include
an open-ended inspection for risks the listed lenses miss.

Ground each finding in a plausible failure scenario and actual source or observed
behavior. State severity, tight location, impact and a concrete correction or
missing observation. Distinguish an established defect from a hypothesis; do not
inflate style preferences or repeat findings to fill a quota. Check whether an
apparent regression is an accepted deliberate contract change.

Use existing valid checks; invoke `gameskills:test` for a necessary missing check,
not once per lens. Visual/gameplay claims need their corresponding evidence.
A source change invalidates affected review/checks. Fixes return to the authorized
implementer; reviewing alone does not take ownership of unrelated work.

Report actionable findings, or no findings with specific residual validation gaps.
A clean review is judgment about the reviewed scope/revision, not PR acceptance,
merge authorization or proof that the artifact plays correctly.
