---
name: build-ui
description: Design or implement native Bevy/GameKit game UI, including views, typed intent, responsive layout, focus, input, tooltips and modal lifecycle. Use for game interfaces; excludes egui developer tooling, non-Bevy frontends and asset-only generation.
---

# Build a native game interface

Read [project context](../../references/project-context.md) and
[UI contracts](../../references/ui-contracts.md). Preserve the task's creative
level and accepted player experience. Establish the game-owned view, typed intents,
semantic regions and control states before wiring presentation to authority.

Inspect the exact Bevy/GameKit source, current plugin composition and style system.
Keep views, actions and branding in the game. Use existing shared mechanics where
their contract fits; introducing GameKit is a separate project choice. Order intent
translation through the producer's public seam and preserve independent consumers.

Implement responsive measurements from declared baselines, accessible control
semantics and eligible focus. Keep primary actions reachable. Cover pointer and
keyboard activation, modal containment/restoration and hidden/disabled/rebuilt
controls. Load the tooltip portion of the contracts when context help is involved;
follow source/design timings and revoke content when disclosure changes.

At level 1, implement the specified design; level 2 permits refinements within it.
For level 3/4, compare bounded alternatives against player goals before committing
to a material direction. Creative latitude does not widen the delivery endpoint.

Use [UI evidence](../../references/ui-evidence.md) to define structural, rendered
and native interaction checks. Invoke `gameskills-ui:verify-ui` when verification
is needed and share its artifacts with an active core audit. Report implemented
behavior and actual evidence; headless success cannot certify the interface's
rendered quality or native input path.
