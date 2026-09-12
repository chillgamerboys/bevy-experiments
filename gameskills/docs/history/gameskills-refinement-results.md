# GameSkills refinement observations

Status: the owner accepted the implementation and native walkthrough results,
authorized merging the refinement stack and requested the Rust refactor plan on
September 10, 2026. The bounded trials are accepted; the scrolling follow-up and
client/platform release evidence remain open. The
[refinement plan](../decisions/gameskills-ci-scope.md) owns their scope.

## Delivery slices and observed behavior

| Slice | Result | Evidence and limit |
|---|---|---|
| [CI scope, PR26](https://github.com/chillgamerboys/bevy-experiments/pull/26) | Component selection, reverse consumers, conservative unknown-input fallback and a stable final gate | 35 real-Git/routing/gate tests pass. Independent review found three edge cases and verified their fixes at `d61addbba23bcb300c983b7f9ce1c755030a079f`; hosted PR checks own platform/integration status. |
| [Documentation entry points, PR27](https://github.com/chillgamerboys/bevy-experiments/pull/27) | Current workflow/direction is separate from resolved historical findings | Local links/layout pass. Hosted docs-only runs pass while deliberately skipping skills, Rust and policy jobs. |
| [Card inspection, PR28](https://github.com/chillgamerboys/bevy-experiments/pull/28) | Accurate unavailable-card reasons and separate keyboard inspection using existing GameKit mechanics | 37 Deckbuilder tests and scoped strict Clippy pass at worker source `ed02353a1186e802694897d82f6c51c150c99a8d`. Authored restriction captures exist at `fb83817a26a74aaee09b0a447e415413eb8c60c9`; the bounded native walk below passes. |
| [Shared tooltip close mark, PR29](https://github.com/chillgamerboys/bevy-experiments/pull/29) | An ordinary ASCII close mark renders with both default and game-owned fonts | Rendered Deckbuilder and Labyrinth captures at `e76197c` show the corrected mark. Shared UI tests pass (47 unit tests and one doctest). This is a small correction discovered during the UI trial, not another feature trial. |

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

**Hosted execution exposed a fixture race.** The macOS skills job in run
`34552810052` failed its no-writes assertion because Git background maintenance
removed `.git/objects/maintenance.lock` between snapshots. The fixture now disables
automatic maintenance before its initial commits, retaining the complete path
comparison. All 17 packaging tests passed locally after this correction at
`c841f73`; fresh hosted checks remain the platform evidence.

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

## Native walkthrough after unlocking

The owner unlocked the Mac, and native app control succeeded. Both executables
were rebuilt from `30d8706cfeecc49da7d9295009bab90077cb52b4` using
`cargo build -p deckbuilder_ui -p labyrinth --bins --profile ci`. Local app bundles
under `target/review` contain these binaries. Labyrinth used `--local --seed 42`
with an isolated `refinement` profile and data directory under local review output.
The following observations came from actual pointer/keyboard actions, accessibility
state and window screenshots, independently of the earlier authored captures.

- **Deckbuilder:** started Solo; Comet explained insufficient energy at both 3
  and 2 energy. Playing Spark reduced energy and added one activity entry; its
  Inspect control then explained that it had already been played. After End Turn,
  Ward and Comet explained the off-turn restriction. Tab/Enter reached and opened
  an unavailable card's Inspect control. T focused the tooltip close control;
  Escape dismissed help and restored inspection focus. Pointer close also worked.
  The game menu cleared card help, contained Tab navigation and prevented T from
  opening background help. Native resizing switched to the stacked layout while
  keeping explanations and bottom actions visible.
- **Labyrinth:** Snap Shot help and nested Formation ranks help were readable,
  with visible close marks. Escape removed the child first, then the parent and
  restored ability focus. With no target, clicking Confirm and pressing T showed
  the missing-target explanation without advancing combat. After selecting E5,
  inspecting the acting hero, Current conditions, Bleed and Condition timing
  preserved Snap Shot, the selected enemy and its 14-to-10 HP preview. Dismissing
  all help preserved that choice; explicitly clicking Confirm applied the four
  damage and advanced to the next hero. Smaller-window scrolling reached the
  final lines of Condition timing, exposing the follow-up below.

Local native screenshots are `target/review/deckbuilder-native-off-turn-resized.png`,
`target/review/labyrinth-native-scrolled-help.png` and
`target/review/labyrinth-native-confirmed-action.png`. They remain local artifacts.
This is a bounded walkthrough, not exhaustive native scale/hover coverage,
network verification, screen-reader acceptance or human player feedback.

### Follow-up found by native scrolling

In the shortened Labyrinth window, scrolling Condition timing to its final lines
also scrolled its heading and close control out of view. Scrolling back restored
them, and Escape remained available. The renderer currently scrolls the entire
card, including the heading; the behavior is not a new close-glyph regression.

Keep the heading and close control visible while scrolling the body, facts and
related links. Verify native wheel and keyboard scrolling, nested dismissal and
focus restoration in both games, with no placement change when a preview locks.
This is an open shared-renderer usability refinement before release acceptance;
no implementation of that follow-up is claimed here.

## Owner acceptance and remaining release work

The owner reviewed the reported work, said it looked good and authorized merging.
This accepts the bounded refinement scope; it does not establish a broad player
study or release readiness. Keep the scrolling correction as a separate UI task.
Current hosted CI results must match each PR's actual source/base before merging.
Native Claude behavior remains unverified because the foundation trial encountered
HTTP 401; this work does not establish cross-client parity or a release. Carry the
documented workflow lessons and client/platform limits into the
[Rust migration](gameskills-rust-cli.md).
