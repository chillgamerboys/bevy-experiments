# Labyrinth verification

Repeatable checks for the current game. Record observations against the tested
build and route; Git and PR history retain earlier results.

Choose coverage from the changed behavior and resolved project rigor. Development
and Testing use macOS; Windows/Linux are Release coverage. A logic-only fix needs
relevant owner tests, not an agent UI walkthrough. UI changes use the affected flow
at 1920×1080 Auto. End-to-end coverage follows affected journeys: an admission fix
does not automatically select process death, all six players or every provider.

The developer's manual sanity check gates only milestone batches affecting game
behavior. The route inventories below are selectable checks, not a checklist for
every PR. Docs/tooling without game effects need no gameplay check. Coverage outside
the selected scope is not unfinished acceptance. Stop once relevant checks pass
unless a new change, failure or unresolved concern justifies more.

Run from the repository root. Cargo's CI profile speeds compilation independently
of verification rigor; the quickstart uses ci, while Cargo defaults to dev when no profile is supplied. These
commands include broad suites for explicit use; the
[CI selector](../../../devtools/docs/ci.md) owns focused suite execution.

```sh
cargo test -p labyrinth-rules --profile ci
cargo test -p labyrinth --profile ci
cargo test -p labyrinth --lib network::tests::process::six_native_processes_survive_guest_kill_and_finish_the_fight --profile ci -- --ignored --exact --nocapture
cargo test --workspace --all-features --profile ci
cargo test --workspace --doc --all-features --profile ci
cargo clippy --workspace --all-targets --all-features --profile ci -- -D warnings
cargo fmt --all -- --check
cargo deny check
cargo run --locked -p repo-devtools --profile ci -- skills legacy
cargo test --locked -p gameskills-cli --profile ci
```

## Focused suites

`gameskills run labyrinth-mechanics --base dev --scope "labyrinth backend mechanics coverage"`
executes the public-backend acceptance scenarios. The
[mechanics coverage matrix](mechanics-coverage.md) maps every built-in Skill,
Ability and equipment grant to explicit expectations and retained coverage.
`labyrinth-hook-calibration` selects the single large-blocker Hook Shot scenario.
`rules-test` includes both this suite and the existing rules tests; do not run both
as duplicate final gates. Add `labyrinth-session` and `labyrinth-presentation` for
the affected command/projection and effect-description adapters.

The fixtures serialize and validate explicit Scenario inputs, configure External
controllers, and commit through `Combat::apply`. A faster source and next-turn
sentinel control initiative without depending on a lucky seed. Assertions use
authored constants for HP, ranks, effects and resource changes. Preview/replay
agreement supplements those assertions; agreement between two calls to the same
resolver alone cannot prove correct mechanics. Full RL training remains deferred.

The Hook Shot calibration distinguishes a rules change from a defect: the former
rank-budget restriction was implemented, but the accepted rule now pulls past
whole occupants. The custom-footprint push regression separately exposes an
appearance-derived size bug. Keep these separate when measuring rework or results.

`repo-devtools ci suite labyrinth-ui-normal` selects tests ending in `normal_1080`.
They run the relevant retained assertions at 1920×1080 Auto. Former compound matrix
tests share assertion helpers with separately named `compatibility` cases; those
retain smaller/larger windows, large text and resize-specific behavior without
running them in the normal suite. `labyrinth-editor` and `labyrinth-lobby` cover
ordinary state/intent tests without duplicating the normal wrappers. The suite runner
lists and executes selected tests and rejects zero-match success.

Select rules/session/admission/process suites for those affected boundaries instead
of using the UI suite for logic-only work. See the
[repository suite owner](../../../devtools/src/ci/suites.rs) for actual names and
selection. Automated native-input messages are behavioral coverage, not an agent
window walkthrough or developer sanity response.

## What each layer proves

| Evidence | Claims | Does not establish |
|---|---|---|
| Pure rules tests | Catalog/build validation, scenario roundtrip, seeded order, transactional cleave/preview parity, statuses and complete fights | UI or networking |
| Session policy tests | Independent participants/actors, sparse typed construction, rank reservations, compact deployment, owned/stale setup, spectator readiness, reassignment replay guards, exact-seed rematch | Physical socket behavior |
| Multi-App socket tests | One host + five encrypted clients, password/direct admission, large authored build/save-load commands, spatial reservation/type/gap convergence, dropped-sixth-App recovery, offer/ACK loss | OS process death or cross-machine reachability |
| Explicit six-process test | Abrupt sixth-seat guest kill, profile lock release, same-peer/actor/class/loadout/status recovery, subsequent completed fight | Cross-machine LAN/Tailscale reachability |
| Fake discovery tests | Provider-neutral listing/removal/compatibility and encrypted join handoff | Real multicast or Tailscale |
| UI behavioral/structural tests | Draft preservation/save ACK, 12+ authored moves via native keyboard/pointer messages, scoped provenance/forecast, focus/modal behavior and bounds | Visual quality or real pointer hardware |
| Native rendered frames | Static composition at the captured logical sizes | Interactive behavior or six-player correctness |
| Manual network routes | Behavior on the recorded machines/interfaces/firewalls | Arbitrary networks or future Steam integration |

The delivery-order regressions capture actual host admission/snapshot messages
after encrypted offer and persistence ACK, then deliver the snapshot first at
separate receive boundaries. They verify delayed publication, stale-attempt
isolation and peer validation without waiting for another host revision.

The Scenario UI route covers invalid seed correction and visibility of subsequent
save/load feedback. The encrypted custom-build/save-load route checks that invalid
JSON preserves setup and a corrected load replaces its earlier error with success.

Do not call a saved-credential round trip a reconnect test. Recovery tests must
re-establish the encrypted connection, retain the same peer/hero, compare exact
initiative/status state, and accept a subsequent legal command. Likewise, a fake
service endpoint is not evidence that a Steam transport adapter exists.

### Complete encounter history backend

`history_tests::` in `labyrinth-rules` checks initial round/turn/status outcomes and
constructor-state parity. `session::tests::history::` checks retention beyond 80
events, bounded snapshot/page decoding, request admission/ranges, ordered overlap,
conflict and stale-encounter rejection, incremental gap recovery and rematch reset.
`network::tests::history::` uses one host and one guest over encrypted loopback:
missing recent windows, a destroyed/recreated guest App with persisted reconnect
credentials, exact recovered archive, stale attempt/encounter replies and unchanged
combat/sequences while reading. Queue overflow and unadmitted requests also exercise
the production host history handler. This is same-machine multi-App evidence, not
OS-process death, discovery or cross-machine coverage. Select these filters for
history work; rendered scrolling and disclosure guard acceptance remain UI checks.
`gameskills run labyrinth-history` selects the session/network history tests only.
For a changed log interface, `labyrinth-history-ui-normal` separately selects its
formatter and eight normal-1080 interaction cases, skipping compatibility wrappers.
This keeps logic-only history fixes independent of rendered UI verification.

## Static frame review

```sh
cargo run -p labyrinth --example labyrinth_review --profile ci -- \
  target/review/labyrinth-1920-auto.png 1920 1080 auto combat
```

For Development/Testing UI changes, inspect only the affected route at 1920×1080
Auto. Other sizes/scales are compatibility cases selected for a relevant defect
or explicit Release support, not routine acceptance. Retain their automated tests.
Routes include `menu`, `host`, `lobby`, `game-menu`, `settings`, `leave`, `history`, `history-long`, `history-older`, `combat`, `help`, `effects`, `inspect`, `order` and
`paused`, `skills`, `skill-help`, `editor-parameters`, `editor-compare`,
`editor-abilities` and `editor-actions`. Help/effects/inspect use authored presentation fixtures, not input or gameplay claims. The offscreen render
uses exact logical dimensions rather than the desktop's window-size limit. Check
actor/rank readability, HP/status duration, action requirements, focus/disabled
contrast and inspector/activity scrolling. Do not approve from dimensions alone.

The `sparse` route retains a hero, wagon and Hauler with empty back ranks. Compare
it with `footprints` at 1920×1080 Auto when changing rank sizing. The normal-1080
footprint regression checks initial sparse rosters, death versus corpse removal,
stable controls and projected movement alignment; authored frames do not establish
those transitions. `gameskills run ui-tooltip-test` covers the shared tooltip
lifecycle. Labyrinth's normal UI tests cover pointer hover, pinned persistence
through other Skill clicks, × branch dismissal, Escape menu routing and nested inspection.

When customization presentation or interaction changes, review the affected lobby,
character editor or ability-overflow path at 1920×1080 Auto. Automatic focus scrolling is part
of usability; verify controls can be reached beyond the first visible rows.
Static frames and headless native-input messages remain distinct from a desktop walk.

The spatial preparation routes are `construction`, `construction-picker`,
`construction-gap`, `construction-enemy` and `construction-owners`. Review the
facing formation, actual multi-rank art/span, selected destination, type mechanics,
ownership and blocked-deployment reason at 1920×1080 Auto when those views change. The
selected type's useful moves must be visible before placement; large text uses a
focused list/detail route rather than hiding facts beneath repeated navigation.
Judge the complete task, including removing a character, choosing another type,
repairing a gap, assigning control and returning from the one character editor.
Reachable controls alone do not establish a clear or efficient composition.

`session::tests::spatial` covers construction/authority/snapshot invariants;
`network::tests::spatial` covers encrypted shared construction and local deployment
after edits. Local Deploy skips the co-op Ready step atomically, while both modes
retain complete/compact validation. Keep the ordinary co-op readiness regressions.

The overlay regressions compare all twelve native actor anchors and actual atlas
sprite transforms across selection, targeting, utilities and detail drawers. They
check measured text bounds, description click containment, and wheel scrolling
using Winit-shaped aggregate window events. A persistent message reader checks
that inspection does not emit combat commands. A separate real-rule transition
test covers Scout confirmation followed by Medic loadout, description and skin
refresh. These are not GPU-rendering or interactive-motion evidence.

## Interactive six-player checklist

Use independent profiles as described in [the game README](../README.md).
Keep artifacts in a private temporary directory and redact codes and credentials.

- Direct route: both discovery providers disabled, five different invitations,
  assign zero/one/multiple heroes, then verify controller-only readiness and host-only
  Start/rematch. Spectators (including host) do not gate Start or pause on disconnect.
  Verify six participant capacity with a two-rank wagon and reject a seventh guest.
  Change a build and verify readiness invalidation and rejection of stale drafts.
- Configure both sides' stats/builds and all six weapon types. Save/load the same
  Scenario JSON and repeat the exact seed. Verify 12+ moves, upgrades/provenance,
  unlimited throws and front-pair cleave against distinct and multi-rank occupants.
- Pause for assignment, move heroes between connected participants and resume.
  Confirm no combat resource/clock changes and rejection of old commands after
  assignment away/back. Dying retains ownership for rescue; dead-only owners spectate.
- Same LAN: host discovery enabled, a passphrase, real guest listing present for
  at least a minute, wrong password rejected and correct password admitted. Check
  occupancy updates and removal when the host closes.
- Remote tailnet: Tailscale installed/authenticated externally; explicit runtime
  opt-in, reachable reported address, provider errors visible, password join and
  actual combat. This does not test Steam identity/lobbies/relay.
- Play out a turn involving damage, bleed, cleansing and movement. Confirm that
  status remains on the actor after movement and initiative remains frozen for
  that round. Inspect disabled actions via both pointer and keyboard.
- Kill the sixth-seat guest **process** during combat. Once detected, the host pauses without
  advancing initiative/statuses. Restart the same profile, reconnect without the
  passphrase, verify the same peer/actor/class/loadout/status and exact boundary,
  then act successfully even if another player chose the same class.
- Close the host. All guests become disconnected; a new host process is a new
  session, not a persisted campaign.
- Resize during action selection and modal use. Traverse all relevant controls
  at the selected display/scale, scroll the inspector/feed, and check focus
  restoration. Review damage/bleed feedback without allowing animation to gate turns.

Cross-machine LAN/tailnet and interactive results must be recorded with the tested
route and build when those claims are selected. Deterministic CI cannot establish
cross-machine operation or supply a milestone developer sanity response. It does
not make every route above a manual gate.

## Local menu and history acceptance

- Hover the game-menu and log toggles, then activate them by pointer or keyboard. No tooltip is
  shown for this control. Actor/ability cards avoid the visible log surface.
- Hover/click the empty column above a character: it must not highlight, inspect,
  or select them. The body hit rectangle follows fitted sprite bounds (with a
  minimum target size); the non-interactive layout column never receives input.
- Tooltip regressions inspect the first measured frame, content replacement,
  child-card opening, and viewport resizing: surface/text geometry must be placed
  before clipping with no hidden settling frame. Native pointer tests also check
  that a tall actor's own preview cannot intercept the target click. Static
  captures establish presentation only; desktop motion still needs visual review.
- The fourteen-ability route loads the real atlas with one hero and two enemies
  at the selected display/scale. Effects and ranks must be visible in the first help fold, paging
  must reach the authored explanation, and cards must leave HP, character
  summaries and Confirm clear. A visible title alone does not establish usable help.
- Open host and guest menus/settings during a live encounter: snapshots and peer
  lifecycle processing continue. Only the menu owner's gameplay input is blocked.
  The socket regression uses six real Apps with full UI stacks on host and one
  guest; direct test intents advance authority while both local menus remain open.
- Navigate Game menu → Settings → Back, then close to restore combat focus and
  selection. Leave opens a confirmation; cancel emits no leave request. Async
  notices and unrelated discovery updates must not recreate focused join fields.
- Disconnect two players. Reconnecting one must not resume combat. Closing a local
  menu cannot dismiss a connection interruption; a rules fault has its own reason
  and persists after reconnection.
- Open the compact combat log without disabling Skill → target → Confirm. Read
  the complete current encounter beyond the recent 80-event snapshot. Inspect a
  long-name row to reach its full text without emitting gameplay. Scroll to old
  entries, load missing pages and receive events; verify the event-ID anchor and
  unread state, then activate Latest. At most 32 rows are mounted and dock/actor
  anchors never move. Rematch resets the archive and reading position.
- Start with no log panel. Close with × and reopen through the toolbar; hidden
  rows have no focus/pointer surface, retained reading state survives, and new
  arrivals never reopen it. Revoking disclosure clears rows and inspection even
  while hidden. Row inspection follows the existing immediate preview,
  two-second pin, × dismissal and menu suspension lifecycle.
- Review main menu, settings, leave and history at normal scale, with pointer,
  keyboard and resizing. Automated layout tests are not an interactive walk.

The menu pass has static captures under `target/review/menus-*.png`. Native desktop
automation must verify that the target process is active and its window is on
screen before attributing missing clicks to the game. A cached window-ID capture
or a successful activation request alone does not establish foreground input. On
macOS, a normal binary launched through an app wrapper can provide the native
activation/Accessibility route when an unbundled CLI launch cannot. Record the
actual binary and observed route; cross-machine network evidence remains separate.

## Forecast verification


The shared contextual-help tests are independent of Labyrinth. They exercise
focus/pointer precedence, modal scope/restoration, hidden/disabled/removed sources,
clipping, activation non-interference and unchanged-resource detection. Their
explicit `Interaction` and geometry fixtures prove selection mechanics, not native
cursor hit testing or rendered tooltip placement. Each adopter must also exercise
real input and layout through its production plugin stack; helpful text appearing
does not prove that the associated action can be selected and confirmed.

Keep Labyrinth's forecast evidence at two separate levels:

- Pure rules: base power versus effective damage and actual HP loss, shared
  immediate resolution, status application/removal, position changes, no mutation
  or random/turn advancement, and off-turn previews granting no commit authority.
- Presentation: known versus unknown HP, modifiers and details; uncertainty text
  and absent exact projections; conditional periodic-effect explanations; and
  matching disclosure in labels, inspection, logs and contextual information.

Vary concealed inputs while keeping public facts fixed and compare the resulting
presentation, including error shape and derived values. Test partial disclosure,
not only an entirely concealed actor. Separately review projected HP segments,
pending effect markers and confirmation clarity at the selected display target;
use other supported sizes only when compatibility is in scope.
Normal encounters remain fully revealed: a hidden-information fixture is neither
an implemented reveal ability nor evidence that network payloads are filtered.

No automated selection test, snapshot or forecast parity check establishes the
feel of the dock. When those interactions change, a pointer/keyboard walk checks hover-to-focus transitions,
off-turn inspection, ability -> target -> Confirm, modal return, overflow and
resizing when affected. Record missing required interaction/network evidence;
unselected routes are not pending gates.

Tooltip lifecycle regressions include immediate first-frame preview and departure,
two-second continuous hover to lock, persistence over empty space and other sources,
explicit keyboard inspection, visible × branch dismissal, and temporary modal suspension.
Escape belongs to the Game menu; it does not dismiss Labyrinth's pinned cards.
The native-layout test compares the card rectangle on every frame across locking:
the preview must use the same shorter geometry as the locked card, not reserve an
extra footer. Native pointer tests hover and activate another Skill while pinned, then verify
that × dismissal does not immediately repin under a stationary pointer. Escape
hides pins through the Game menu and restores valid subjects after returning. Render
`labyrinth_review ... 1920 1080 auto help` and `help-locked` for separate authored
presentation states; those captures freeze timing and do not prove hover duration.

## Additional selectable routes and coverage boundaries

The unified editor and preparation require decision-information checks as well as
input/layout checks. Compare a dagger and greatsword at different acting ranks,
inspect an Ability that upgrades a Skill’s ranks/damage, remove one of several grants, and explain
the actual added/removed/changed moves before applying. Exercise category changes,
dirty character switching, discard/reload, server conflicts and source replacement.
The normal-1080 editor checks cover Parameters-first layout, a live draft summary,
read-only equipment grants, Moveset inspection, explicit reversible Unequip, current
passive effects, dirty navigation, permissions, text focus and host acknowledgment.
Actual render routes `editor-parameters`, `editor-compare` and `editor-abilities`
show the normal layout and existing sprite assets; they do not prove hardware input.
Both teams use the same editor. Do not infer understandable choices from unclipped
buttons alone; record what is visible separately from observed player comprehension.

`ui::shell::lobby::tests` covers preparation navigation, retained seed drafts and
the footer at Auto/200%. Its real-atlas pointer fixture selects an occupied actor
through window cursor and mouse-button events without forced focus or direct
`UiActivated` injection. The movement regression checks live inspection after an
accepted projection, then rejection of a newly stale movement proposal.
Editor lifecycle coverage in `ui/setup_tests.rs` includes permission loss/return
without remounting fields, retained drafts/carets, and independent footprint/choice
restrictions. Decision coverage in `ui/setup/tests.rs` verifies that prerequisite
rejections use catalog move names while resolution still controls eligibility.
These checks do not establish a native walk; select one when the changed
interaction requires it. The queued-hotbar regression sends a
synthetic `UiActivated` batch through production translation before Present, with
an unchanged positive control and a replaced valid build. It tests source binding,
not a desktop pointer reproduction. Preserve the baseline wrong-action failure
and the fixed result in the UI revision evidence.

Historical local reviews did not establish cross-machine LAN/Tailscale behavior,
all window-resize paths or manual round-six corpse expiry. Deterministic lifecycle
and localhost tests prove different claims. Recheck these routes when the related
behavior and rigor require them; they do not block unrelated development work.
Do not infer a fresh pass from a removed milestone report.
