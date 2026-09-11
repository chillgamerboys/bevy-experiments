# GameSkills refinement observations

Status: implementation and review observations, September 10, 2026. Native game
walkthroughs and human player feedback remain incomplete. This record does not
declare the trials finished or authorize the Rust migration. The
[refinement plan](gameskills-ci-scope.md) owns their scope.

## Delivery slices and observed behavior

| Slice | Result | Evidence and limit |
|---|---|---|
| [CI scope, PR26](https://github.com/chillgamerboys/bevy-experiments/pull/26) | Component selection, reverse consumers, conservative unknown-input fallback and a stable final gate | 35 real-Git/routing/gate tests pass. Independent review found three edge cases and verified their fixes at `d61addbba23bcb300c983b7f9ce1c755030a079f`; hosted PR checks own platform/integration status. |
| [Documentation entry points, PR27](https://github.com/chillgamerboys/bevy-experiments/pull/27) | Current workflow/direction is separate from resolved historical findings | Local links/layout pass. Hosted docs-only runs pass while deliberately skipping skills, Rust and policy jobs. |
| [Card inspection, PR28](https://github.com/chillgamerboys/bevy-experiments/pull/28) | Accurate unavailable-card reasons and separate keyboard inspection using existing GameKit mechanics | 37 Deckbuilder tests and scoped strict Clippy pass at worker source `ed02353a1186e802694897d82f6c51c150c99a8d`. Authored restriction captures exist at `fb83817a26a74aaee09b0a447e415413eb8c60c9`; native walks remain pending. |
| Shared tooltip close mark | An ordinary ASCII close mark renders with both default and game-owned fonts | Rendered Deckbuilder and Labyrinth captures at `e76197c` show the corrected mark. Shared UI tests pass (47 unit tests and one doctest). This is a small correction discovered during the UI trial, not another feature trial. |

The close-mark captures are `target/review/deckbuilder-close-fixed-1280.png` and
`target/review/labyrinth-close-fixed-1280.png`. Earlier Deckbuilder captures cover
energy and turn restrictions at 1280×720 and a played card at 1920×1080. These
are actual images rendered by the production plugins from authored action routes;
they are not recordings of a native pointer/keyboard walk. Later CI/docs-only
integration does not relabel these captures with a newer game source identity.
Generated images and full logs stay in local review output; they are not promised
as downloadable GitHub artifacts or included in repository source.

## What improved and what the trials exposed

**Proportionate work.** The documentation cleanup required three files, local
checks and a PR. It did not need a worker queue or a game launch. The UI correction
was independent of CI work and used one isolated implementation worker, then one
independent CI reviewer. Maximum active concurrency was the coordinator plus one
worker, within the accepted five-worker ceiling.

**Correct ownership.** `UiInspectable` permits inspecting disabled controls but
does not make their gameplay actions keyboard-focusable. Deckbuilder needed an
enabled, game-owned Inspect control using the existing tooltip catalog. Reasons
and private-hand disclosure remain game-owned. No new shared gameplay API was
needed. The subsequent close-glyph failure belonged to the shared renderer and
was checked with both consumers.

**Review found things the original tests missed.** Static include consumers must
be combined with the input's owner. All Rust include delimiters need detection,
and explicit Cargo `package.build` paths need the same uncertainty treatment as
conventional `build.rs`. Each reproduced finding gained a regression; unsupported
or dynamic include/build inputs now select full coverage. The reviewed selector
does not claim to infer arbitrary undeclared runtime file readers.

**Queue evidence is useful but bounded.** The installed candidate
`99be57217d4aab6b2e70272531ea6dcc5838a35b` coordinated real isolated worktrees in
the `refinement-trials` queue, including review injection and block/resume after
findings. Source commits, caller-supplied reports and observed integration remain
distinct from test results and human acceptance. Model/token telemetry was not
available for this trial; no model-cost saving is inferred from worker count.

**Verification must be sequenced with Git operations.** In run
`7cb7af9c55ce4c2c810d7aef51f9a0b2`, both repository commands exited successfully,
but a concurrent push changed Git references. The runner correctly returned
`stale`; inspection showed the reference digest was the sole identity difference.
The source, commands and environment were unchanged. This is retained as a stale
observation, not reusable passing evidence. Collect recorded checks after Git
changes settle. Before freezing Rust contracts, discuss whether explicitly
declared Git inputs would make parallel work more useful while retaining base
and source checks. This trial does not weaken the current conservative binding.

## Observed CI execution

The narrative planning change in [PR25's run](https://github.com/chillgamerboys/bevy-experiments/actions/runs/34544638673)
ran the old full matrix for 1,395 combined job seconds (23m15s). The first
[docs-only trial run](https://github.com/chillgamerboys/bevy-experiments/actions/runs/34552074342)
used 10 seconds for classification/repository checks and 8 seconds for the final
gate, with all expensive jobs skipped. These are two observed runs on different
changes and runner/cache states, not a controlled benchmark, billing estimate or
promise about future timings. The concrete improvement is the removal of
unrelated execution from the docs path.

## Remaining acceptance

The native UI tool reported a locked Mac and could not unlock it automatically.
The owner was asked to unlock it; no native walkthrough is claimed. Resume with
Deckbuilder's energy, played and off-turn explanations; Tab/Enter inspection,
T/Escape, modal return, pointer dismissal, scrolling and resizing. Exercise the
corresponding Labyrinth inspection/confirmation paths using the corrected shared
renderer. Retain applicable automated scale regressions and the accepted normal-
scale manual priority.

Human feedback must establish whether the explanations and secondary Inspect
controls are clear and useful. Current hosted CI results must match each PR's
actual source/base before acceptance or an authorized merge. Native Claude
behavior remains unverified because the foundation trial encountered HTTP 401;
this work does not establish cross-client parity or a release. Resolve material
trial findings before freezing contracts for the [Rust port](gameskills-rust-cli.md).
