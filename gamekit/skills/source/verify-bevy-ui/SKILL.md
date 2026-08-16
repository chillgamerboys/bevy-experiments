---
name: verify-bevy-ui
description: Verify native Bevy 0.19 game UI after implementation using structural snapshots, deterministic rendered frames, keyboard/pointer walks, viewport matrices, and accessibility checks. Use for visual regressions, responsive review, modal/focus behavior, clipping, hierarchy, and motion or feel validation. Do not use as proof of gameplay rules.
---

# Verify Bevy UI

Collect complementary evidence; do not ask one artifact to prove another layer.

Read `.bevy-gamekit/overlays/verify-bevy-ui.md` first when it exists; treat it as adopter context without collapsing evidence layers.

## Workflow

1. Read `../../references/evidence.md` and write the specific presentation and interaction claims under review.
2. Run structural checks for stable names, semantic regions, accessible labels, enabled focus order, minimum targets, clipping, and scroll reachability.
3. Capture deterministic frames at compact, standard, and wide logical canvases, plus the maximum supported semantic scale.
4. Compare hierarchy, contrast, density, alignment, disabled/selected states, and modal stacking in static frames.
5. Walk the interface using pointer and keyboard only. Verify Tab and Shift-Tab order, Enter/Space parity, modal focus trap/restore, scrolling, resizing, and state refresh.
6. Observe motion and feel directly. A still image cannot validate transition timing, camera behavior, feedback, or input latency.
7. Report each failure with the viewport, scale, route, input path, and evidence type that exposed it.

When Gamekit UI is present, read `../../references/gamekit-apis.md`. Read `../../references/bevy-0.19.md` before diagnosing focus or layout behavior.
