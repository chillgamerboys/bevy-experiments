---
name: review-bevy-change
description: Review a Bevy 0.19 code change, branch, or diff for correctness, architecture, silent failures, schedule and deferred-command mistakes, determinism, persistence, UI evidence, and test coverage. Use for pre-merge audits or evidence-backed code review. Do not use when asked to implement the change rather than review it.
---

# Review Bevy Change

Produce findings tied to the exact reviewed revision and observable evidence.

Read `.bevy-gamekit/overlays/review-bevy-change.md` first when it exists; apply local gates in addition to the lenses below.

## Workflow

1. Record the base, head, dirty state, manifests, and files in scope.
2. Read the owning implementation and its tests; do not review only the diff when authority lives elsewhere.
3. Audit in these lenses:
   - single authority and dependency direction;
   - explicit errors versus silent defaults;
   - edge values, removal, empty state, and terminal state;
   - deterministic ordering and serialization round trips;
   - Bevy plugin, schedule, run-condition, message, hierarchy, and deferred-command behavior;
   - feature/config/persistence wiring from definition through consumer;
   - test altitude and evidence boundaries.
4. Run focused tests first, then the full all-feature gate if the change is merge-bound.
5. For UI changes, require the structural/static/interactive split in `../../references/evidence.md`.
6. Report only actionable findings with severity, file/line, failure scenario, and missing evidence. If no findings remain, say so and list residual validation gaps.

Read `../../references/bevy-0.19.md` for API traps and `../../references/architecture-boundaries.md` for extraction or ownership changes. Read `../../references/gamekit-apis.md` when Gamekit crates are in the dependency graph.
