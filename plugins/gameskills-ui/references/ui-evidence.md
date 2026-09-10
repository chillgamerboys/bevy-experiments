# UI verification evidence

Write the presentation/input claim and record build/source, route, window logical
size, device scale, semantic scale, assets and relevant settings. Choose the
viewport matrix from target support and the reported defect. Do not add a maximum
scale gate simply because a previous project used one; preserve explicitly retained
automated regressions even when a manual pass is deferred.

| Layer | Inspect | Boundary |
|---|---|---|
| Structure and deterministic input | Semantic hierarchy, labels, enabled focus order, geometry, clipping/scroll reachability, typed activations | Does not prove pixels or native hit testing |
| Rendered frames | Text contrast/readability, hierarchy, density, alignment, selected/disabled states, overlap, help/modal stacking | Does not prove interaction, duration or motion |
| Native interaction | Pointer and keyboard routes, focus trap/restore, scroll, resizing, state refresh, input containment, transitions | Does not prove all rule invariants or human enjoyment |

Use real production plugin/input/layout wiring for relevant acceptance. Fixtures
that directly set `Interaction` or geometry prove the selection logic that consumes
those fields; they do not prove that the native cursor hits a control. Capture
actual frames for visual claims. Authored screenshot states can freeze time, so
explicitly exercise hover duration, locking and animation outside those captures.

Walk the promised route to completion, including hidden/disabled controls, focused
content after scrolling, modal return, rebuilt views, source removal and mistakes.
Do not infer a legal gameplay action from text rendered about it; assert typed
intent and game-owned validation at the appropriate separate test seam.

Report failures with reproducible input path, viewport/scale and evidence layer.
Keep unavailable rendering/native input explicit, even if headless tests pass.
Reuse equivalent evidence only while source, assets, settings and environment
remain applicable; a later HEAD cannot inherit an old capture by relabeling it.
