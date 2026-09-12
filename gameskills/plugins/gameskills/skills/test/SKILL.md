---
name: test
description: Establish engineering behavior for a Bevy game, library or skill change using appropriate tests and observed command results. Use for selecting or executing pure, app, runtime, integration or performance checks; player enjoyment belongs to playtest.
---

# Test the claim at its owner

Read [project context](../../references/project-context.md),
[Bevy craft](../../references/bevy-craft.md) and
[verification](../../references/verification.md). State the behavior to prove and
choose the narrowest useful altitude: pure owner, minimal production-plugin app,
headless structure/input, real runtime or target-platform integration.

Use deterministic time, seeds, stable IDs and bounded progress. Assert typed
state/transitions before logs or rendered text. Exercise relevant rejection,
edge, round-trip, schedule and lifecycle behavior. Hidden convenience plugins
can conceal production omissions; mocks prove only their simulated boundary.
Performance claims need a representative workload, build/hardware identity and
comparable baseline, not a single uncontextualized timing.

Use project-configured commands and prerequisites. Execute a named graph with
`run NAME [NAME ...]` through the common runtime prefix; it records actual results
and dependency skips. Avoid simultaneous native/GPU/port work that conflicts with
other owners. A started, interrupted, timed-out or skipped check is not a pass.

Use focused checks during iteration and the applicable integration/feature/platform
gates at the final revision. Reuse unchanged evidence only after validation;
a skill invocation or another review lens is not itself a reason to rerun a suite.
For changed or uncertain inputs, rerun affected checks. UI and multiplayer packages
supply relevant specialist cases within this graph, not duplicate test pipelines.

Report source and commands, passed/failed/pending/unavailable results and what each
artifact proves. Code tests cannot certify rendered quality, native interactions,
cross-machine routes or human enjoyment without their corresponding evidence.

Attach relevant run IDs to the existing delivery task. A passing command is
verification evidence, not proof of push, PR, tracking or integration. If evidence
is stale, report the changed input paths from validation before deciding what to
rerun; preserve the original evidence.
