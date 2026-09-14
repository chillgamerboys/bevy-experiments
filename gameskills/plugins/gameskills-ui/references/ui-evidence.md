# UI verification evidence

Use this evidence when the change affects presentation or interaction. Logic-only
fixes do not automatically need agent captures or a UI walk. Record the affected
claim, build/source, route and relevant display/settings. Use the project's resolved
rigor and display target; a normal-size check is sufficient when that is the selected
scope. Add compatibility cases for an affected defect or explicit support requirement,
not because another project used them. Preserve retained regressions without making
every viewport/scale part of every run.

Choose only the evidence layers needed by the claim. The table describes their
boundaries, not three mandatory passes for each edit. End-to-end UI walks cover the
changed journey; do not tour unrelated menus or repeat an applicable observation.

| Layer | Inspect | Boundary |
|---|---|---|
| Structure and deterministic input | Semantic hierarchy, labels, enabled focus order, geometry, clipping/scroll reachability, typed activations | Does not prove pixels or native hit testing |
| Rendered frames | Composition, art/content treatment, emphasis, typography/readability, density, consistency, state clarity, overlap and stacking | Does not prove interaction, duration or motion |
| Native interaction | Complete task flow, discovery/correction costs, pointer/keyboard routes, focus, scroll, resizing, state refresh, containment and transitions | Does not prove all rule invariants or human enjoyment |

For a choice-heavy route, use a task that requires understanding a tradeoff rather
than reproducing the control sequence. From the interface alone, identify what a
choice changes, relevant restrictions, and how to inspect or recover before
committing. Check explanations against authoritative data, including modified and
unavailable choices. Record where information is absent, misleading or requires
unreasonable memorization. Agent inspection establishes information availability
and accuracy; intended players' observed comprehension is separate evidence.

Critique what is rendered, not only compliance with the plan. Is the main subject
given useful space and emphasis? Do art, typography, alignment and density create a
coherent composition across states? Challenge competing focal points, decorative
containers that crowd the task, weak differentiation and layout that hides a useful
relationship. Attractive art cannot compensate for a misleading hierarchy, and
unclipped controls do not establish visual quality. Cite the actual state and its
consequence; distinguish a consequential design finding from a taste preference.

When a complete flow changes, walk that task and a meaningful correction through
actual states.
Notice whether the next action can be discovered without foreknowledge, comparisons
require memorization, or navigation repeats editing and loses context. For example,
follow creation through selection and commitment rather than testing each dialog
in isolation. Judge feedback and transitions at the moment they matter. Propose a
specific correction tied to the observed friction, then inspect the affected task
and states again. Preserve material unresolved design findings even when structure
and input pass. Report agent critique, observed user feedback and mechanical results
separately; none silently upgrades the others into usability acceptance.

Use real production plugin/input/layout wiring for relevant acceptance. Fixtures
that directly set `Interaction` or geometry prove the selection logic that consumes
those fields; they do not prove that the native cursor hits a control. Capture
actual frames for visual claims. Authored screenshot states can freeze time, so
explicitly exercise hover duration, locking and animation outside those captures.

For the selected route, exercise affected hidden/disabled controls, focus, scrolling,
modal return, rebuilt views, source removal and correction paths.
Do not infer a legal gameplay action from text rendered about it; assert typed
intent and game-owned validation at the appropriate separate test seam.

Report failures with reproducible input path, viewport/scale and evidence layer.
Keep required unavailable rendering/native input explicit, even if headless tests
pass; evidence outside the selected scope is not pending acceptance. A configured
developer sanity gate belongs to milestone promotion, and an agent observation
cannot supply the developer's response.
Reuse equivalent evidence only while source, assets, settings and environment
remain applicable; a later HEAD cannot inherit an old capture by relabeling it.
