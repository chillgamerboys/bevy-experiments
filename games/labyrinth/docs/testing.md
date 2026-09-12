# Labyrinth verification

Repeatable checks for the current game. Record observations against the tested
build and route; Git and PR history retain earlier results.

Run from the repository root. CI profile uses the same source/features with faster unoptimized
compilation; ordinary play uses the default development profile.

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

## What each layer proves

| Evidence | Claims | Does not establish |
|---|---|---|
| Pure rules tests | Seeded order, rank legality, status boundaries, atomic rollback, complete fights | UI or networking |
| Session policy tests | Six-player readiness, repeated-class actor ownership, durable replay watermark, pause/rematch | Physical socket behavior |
| Multi-App socket tests | One host + five encrypted clients, password/direct admission, command convergence, dropped-sixth-App recovery, offer/ACK loss | OS process death or cross-machine reachability |
| Explicit six-process test | Abrupt sixth-seat guest kill, profile lock release, same-peer/actor/class/loadout/status recovery, subsequent completed fight | Cross-machine LAN/Tailscale reachability |
| Fake discovery tests | Provider-neutral listing/removal/compatibility and encrypted join handoff | Real multicast or Tailscale |
| UI behavioral/structural tests | Typed intents, focus/modal behavior, target eligibility, viewport control bounds | Visual quality or real pointer hardware |
| Native rendered frames | Static composition at the captured logical sizes | Interactive behavior or six-player correctness |
| Manual network routes | Behavior on the recorded machines/interfaces/firewalls | Arbitrary networks or future Steam integration |

Do not call a saved-credential round trip a reconnect test. Recovery tests must
re-establish the encrypted connection, retain the same peer/hero, compare exact
initiative/status state, and accept a subsequent legal command. Likewise, a fake
service endpoint is not evidence that a Steam transport adapter exists.

## Static frame review

```sh
cargo run -p labyrinth --example labyrinth_review --profile ci -- \
  target/review/labyrinth-1920-auto.png 1920 1080 auto combat
```

Repeat for 1280×720, 1920×1080 and 3840×2160, each with `auto` and `200` scaling.
Routes include `menu`, `host`, `lobby`, `game-menu`, `settings`, `leave`, `history`, `compact`, `combat`, `help`, `effects`, `inspect`, `order` and
`paused`. Help/effects/inspect use authored presentation fixtures, not input or gameplay claims. The offscreen render
uses exact logical dimensions rather than the desktop's window-size limit. Check
actor/rank readability, HP/status duration, action requirements, focus/disabled
contrast and inspector/activity scrolling. Do not approve from dimensions alone.

For the current description/dock correction, prioritize normal-scale play and
ordinary window resizing. The 200% option and existing automated regressions remain,
but a manual 200% review is deferred and is not a release gate for this pass.

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
  all six ready, correct hero ownership and host-only start/rematch. Pick repeated
  classes, verify independent actor control and uses, and reject a seventh guest.
  Changing a class invalidates readiness; six seats do not require six classes.
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
  at automatic/200% semantic scale, scroll the inspector/feed, and check focus
  restoration. Review damage/bleed feedback without allowing animation to gate turns.

Cross-machine LAN/tailnet and interactive results must be recorded with the tested
route and build. Passing deterministic CI is not a substitute for these manual gates.

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
- Expand history without disabling ability → target → Confirm. Expand an action
  and open its ability tooltip without emitting gameplay. Scroll to old entries,
  receive events, verify position/unread state, then activate Latest. Stable rows
  survive snapshot updates and the dock/actor anchors never move.
- Start with no log panel. Switch History → Compact → Hidden and reopen through
  the toolbar. Compact contains only two outcome summaries, not action expansion
  or Latest controls. Hidden has no focus/pointer surface; events remain retained
  and new arrivals cannot reopen it. Portrait, character and effects cards replace
  the old initiative/Inspect drawers without changing targeting or confirmation.
- Review main menu, settings, leave and history at normal scale, with pointer,
  keyboard and resizing. Automated layout tests are not an interactive walk.

The menu pass has static captures under `target/review/menus-*.png`. Native desktop
inspection currently returns `cgWindowNotFound`, including for the review app
bundle; a completed interactive walk is not claimed. Cross-machine network routes
also remain separate manual evidence.

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
pending effect markers and confirmation clarity at all supported canvas sizes.
Normal encounters remain fully revealed: a hidden-information fixture is neither
an implemented reveal ability nor evidence that network payloads are filtered.

No automated selection test, snapshot or forecast parity check establishes the
feel of the dock. A pointer/keyboard walk still checks hover-to-focus transitions,
off-turn inspection, ability -> target -> Confirm, modal return, overflow and
resizing. Record any missing interactive or cross-machine evidence explicitly.

Tooltip lifecycle regressions include immediate first-frame preview and departure,
one-second continuous hover to lock, persistence over empty space, source switching,
explicit keyboard inspection, modal cleanup, and ×/Escape/outside dismissal.
The native-layout test compares the card rectangle on every frame across locking:
the preview must use the same shorter geometry as the locked card, not reserve an
extra footer. Native pointer tests close the × over an underlying character and
verify that stationary-pointer dismissal does not reveal a new tooltip. Render
`labyrinth_review ... 1280 720 auto help` and `help-locked` for separate authored
presentation states; those captures freeze timing and do not prove hover duration.

## Unresolved verification

Historical local reviews did not establish cross-machine LAN/Tailscale behavior,
all window-resize paths or manual round-six corpse expiry. Deterministic lifecycle
and localhost tests prove different claims. Recheck these routes when the related
behavior is next reviewed; do not infer a fresh pass from a removed milestone report.
