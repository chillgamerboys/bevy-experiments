---
name: verify-ui
description: Verify rendered native Bevy/GameKit UI and actual pointer/keyboard interactions, including focus, hidden state, modal lifecycle, clipping and supported scales. Use for UI acceptance or regressions; does not certify gameplay rules or player enjoyment.
---

# Verify the rendered and interactive UI

Read [project context](../../references/project-context.md),
[UI evidence](../../references/ui-evidence.md) and relevant
[UI contracts](../../references/ui-contracts.md). State the exact claims and
candidate identity, route, supported inputs and viewport/scale matrix.

Collect complementary evidence at the layers required by the change. Structural
checks cover semantics, eligibility and reachability. Actual rendered frames cover
static hierarchy, contrast and clipping. A native walk covers pointer/keyboard
parity, focus, scrolling, resize, modal return and timing. Missing one layer stays
visible; don't substitute a log or fabricated capture.

Exercise the relevant production route, including hidden/disabled controls, focus
after scrolling/rebuilds and state refresh. When help changes, test hover/locking,
dismissal/click-through, keyboard containment, scope transitions and revocation.
Inspect moving transitions directly; authored still states do not prove duration.

Use audience/platform expectations instead of universal size or maximum-scale
gates. Follow explicitly accepted manual deferrals while retaining relevant
existing automated regressions. Make source, assets and environment identity
available to the core audit so unchanged checks can be reused responsibly.

Report reproducible findings, actual artifacts and each remaining evidence gap.
Distinguish a usable input path from game-rule legality and from intended players'
feedback; those belong to their owning tests and core playtest.
