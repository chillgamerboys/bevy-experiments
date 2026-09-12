---
name: prepare-contribution
description: Investigate a verified Bevy engine bug or compelling engine-level gap and prepare technical evidence for human review under Bevy's contribution policy. Optional specialist workflow; ordinary game or GameKit features do not require upstream proposals.
---

# Prepare evidence for a human contribution

Read [project context](../../references/project-context.md) and
[contribution boundary](../../references/contribution-boundary.md), then recheck
the linked receiving-project policies. Confirm why this finding belongs to engine
behavior rather than a game choice, convenient extension or local skill failure.

For a bug, establish expected/actual behavior and a minimal reproduction against
known Bevy revisions with relevant features/platforms. Check current upstream and
existing reports when relevant. For a gap, document the real game-development need,
existing APIs/plugins/plans and engine-level benefit versus maintenance cost.
Do not require several games to reproduce one legitimate engine defect.

Prepare local technical observations, reproduction code, tests and bounded design
options for the human to inspect and understand. Verify any proposed code at the
relevant engine seam. Keep GameKit conveniences independent and record workaround
removal conditions; upstream acceptance is not a prerequisite for local usefulness.

Before any upstream issue, proposal or PR, require actual human review of evidence,
rationale and proposed design/code. The human must author upstream public prose
and communications under Bevy's AI policy; do not provide an agent-written issue,
PR body or docs draft as ready for submission. Preserve human commit authorship and
disclosure requirements. Do not equate a general request to help Bevy with completed
human review or authorization to communicate with others.

Deliver the verified technical package, known limitations and exact human handoff.
Continue useful authorized investigation; do not invoke the ordinary automated
PR/publication pipeline for Bevy. Its maintainers decide acceptance.
