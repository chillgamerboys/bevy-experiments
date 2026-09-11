# Labyrinth verification

## Handoff follow-up — September 10, 2026

The follow-up to `e073da9` passes 113 application tests, 52 rules
tests, and the complete workspace all-feature suite (331 tests/doctests). The
separately enabled six-process abrupt guest-restart gate also passed. New coverage
includes encrypted request bursts, a noisy prefix followed by a quiet peer,
unauthorized requests, queue overflow followed by same-identity reconnect with the
watermark preserved, and an oversized encrypted Hello rejected before decoding.
Shared budget tests cover count/byte edges and independently served peers.

Presentation tests cover partial corpse damage, clearing with the before-pool HP
maximum, ordinary lethal damage, dying-hero death/rescue, hidden-health bystanders,
forecast bars and accessible labels, effective condition timing, and revocation of
open help when status disclosure is removed. Catalog prose changed the content
fingerprint to `53e0f0d6572c2408d126b0b82ce61a6a03af2f8d98398aca26105665281c622f`;
all multiplayer participants need matching builds.

The native local walk used the real reducer: damage and Bleed; Ash Brute death;
corpse Bleed at round end; a displayed `3 → 0 / 5` clear forecast followed by actual
removal/formation compaction; rescue of a dying Scout; subsequent death saves and
permanent death; and the wagon moving from ranks 5–6 to 4–5 while the Medic moved to
rank 6. The Scout corpse remained through round 5 after creation in round 3. The Mac
locked before the planned round-6 expiry and resize checks; these remain incomplete
as native interaction evidence. Pure lifecycle regressions cover expiry separately.

Authored capture routes `corpse-forecast` and `corpse-help` reproduce the two fixed
presentation paths. Normal-scale 1280×720 and 1920×1080 images are under
`target/review/followup-corpse-*.png`; they are presentation fixtures, not a gameplay
recording. At 1280×720 the actor forecast card uses its existing scroll viewport;
the full clear explanation is visible without scrolling at 1920×1080. Local Clippy,
formatting, dependency policy, library-only consumers, repository/skill tooling
checks passed. WebAssembly checks passed for hex, turns, session, UI and rules;
discovery/multiplayer builds without default features, the minimal test helper,
and standalone UI tests also passed. Cross-machine networking and remote CI
results were not verified in this run. Earlier milestone evidence follows.

## Footprint and corpse milestone baseline

The footprint/corpse milestone adds pure lifecycle tests, a five-App real UDP wagon
admission/action/fresh-client reconnect test, and native UI geometry/selection tests.
Existing six-player transport and abrupt-process-restart tests remain in place.
Capture routes `footprints` and `corpses` show the large units alive and as authored
corpse presentation fixtures. They are static evidence, not simulated death saves.

Final local macOS verification for this milestone: 51 rules tests and 105 application
tests pass, plus the explicitly enabled six-process guest-kill/restart test. The full
workspace all-feature suite (including doctests), strict Clippy, formatting, dependency
policy and repository ownership/link checks pass. The default formation regression
requires every enemy to have an in-range attack: Brutes 1–2, Hauler 3–4, Stalker/Archer
5–6. Five-App encrypted wagon admission/action/fresh-client reconnection is covered
separately from the explicit six-single-rank capacity fixture.

Rendered review covers the new living lineup and authored corpse layout at 1280×720,
and living lineup at 1920×1080. Native interaction checked keyboard ability selection,
focus traversal, pointer target selection and confirmation advancing to the next hero.
This is not a complete manual death-save playthrough or cross-machine multiplayer test.

Run from the repository root. CI profile uses the same source/features with faster unoptimized
compilation; ordinary play uses the default development profile.

```sh
cargo test -p labyrinth_rules --profile ci
cargo test -p labyrinth --profile ci
cargo test -p labyrinth --lib network::tests::process::six_native_processes_survive_guest_kill_and_finish_the_fight --profile ci -- --ignored --exact --nocapture
cargo test --workspace --all-features --profile ci
cargo test --workspace --doc --all-features --profile ci
cargo clippy --workspace --all-targets --all-features --profile ci -- -D warnings
cargo fmt --all -- --check
cargo deny check
cargo run --locked -p gamekit-repo-tools --profile ci -- skills legacy
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

Use independent profiles as described in [the game README](README.md).
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

## Six-player foundation review

Local macOS checks cover the six-seat reducer, repeated-class ownership, custom
equipped loadouts, real encrypted six-App sessions, and an explicit six-process
kill/restart followed by a completed fight. CI runs that process gate on all three
platforms; a local macOS pass is not a report of remote CI results.

## Stage-first UI review

The sprite/HUD pass retains twelve visible art hit regions at all six size/scale
combinations. Essential identities and current HP remain near actors; full names,
owners and effect definitions are inspectable. The battlefield does not scroll.
Long equipped-ability rows scroll horizontally; primary target legality and Confirm
remain outside that scroll area. Static frames cover combat, effect overflow,
inspection and the disconnected-player overlay.

Tests include explicit ability → target → Confirm with no early intent, native
window cursor/mouse-message hit testing (without assigning Interaction), modal
pointer blocking, drawer focus trapping/restoration, shortcut suppression, keyboard
scrolling to final content, and camera/DPI projection. These are complementary to
the existing authority, restart and replay tests, not replacements.

Local macOS workspace all-feature tests/doctests, strict Clippy, formatting,
dependency policy and repository boundary/link checks passed for this pass. The
six-process restart gate also passed on the final UI revision during pre-merge
verification. Real cross-machine routes remain unverified. Build output retains
the macOS linker `__eh_frame` size warning.

The native review build launches. The computer-use tool still cannot attach to this
unbundled executable (`Invalid app`); a fallback desktop walk was stopped when focus
changed. No completed interactive walk is claimed. A user pointer/keyboard/resizing
pass and cross-machine discovery/play remain manual gates; neither is established
by captures or same-machine tests.
