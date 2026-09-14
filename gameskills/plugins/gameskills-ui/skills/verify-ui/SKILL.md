---
name: verify-ui
description: Critique rendered native Bevy/GameKit UI for visual quality and task usability, and verify actual pointer/keyboard interaction, lifecycle and supported scales. Use for UI acceptance or regressions; does not certify gameplay rules or player enjoyment.
---

# Verify the rendered and interactive UI

Read [project context](../../references/project-context.md),
[UI evidence](../../references/ui-evidence.md) and relevant
[UI contracts](../../references/ui-contracts.md). State the exact claims and
candidate identity, affected route, inputs and the project's resolved rigor/display
target. Select this work when presentation or interaction changes require it; a
logic-only fix does not automatically need agent screenshots or a native walk.

For decision-heavy views, verify the information needed for a concrete player
task, not just reachable controls. Exercise inspection, comparison, consequences
and correction using the presented information; check its accuracy against the
game-owned model. Record missing or misleading decision information as a finding.

For changed presentation or task flow, critique the relevant rendered states using the
evidence guide. Challenge visual quality, attention competition, discoverability,
unnecessary steps, repeated editing and context loss. Cite the problematic state,
its consequence and a concrete correction. Keep material design findings open
even when every control is reachable and unclipped; separate agent judgment from
mechanical checks and observed user feedback.

Collect complementary evidence at the layers required by the change. Structural
checks cover semantics, eligibility and reachability. Actual rendered frames cover
composition, hierarchy, readability and clipping. A native walk covers pointer/keyboard
parity, focus, scrolling, modal return or timing when affected. Resize checks apply
when their behavior or display support is in scope. A missing required layer stays
visible; an unselected layer is not unfinished acceptance. Don't substitute a log
or fabricated capture.

Exercise the relevant production route, including hidden/disabled controls, focus
after scrolling/rebuilds and state refresh. When help changes, test hover/locking,
dismissal/click-through, keyboard containment, scope transitions and revocation.
Inspect moving transitions directly; authored still states do not prove duration.

Use audience/platform expectations instead of universal size or maximum-scale
gates. At Development, use the configured normal display for the changed flow;
compatibility resolutions/scales run when selected by project policy or the defect.
Keep retained regressions selectable without running every case on every change.
Do not add a developer sanity gate before a configured milestone. Make source, assets and environment identity
available to the core audit so unchanged checks can be reused responsibly.

Stop when the affected claims are established unless a concrete concern remains.
Report reproducible findings, actual artifacts and each required evidence gap.
Distinguish a usable input path from game-rule legality and from intended players'
feedback; those belong to their owning tests and core playtest.
