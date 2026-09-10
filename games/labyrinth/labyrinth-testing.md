# Labyrinth verification

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
python3 skills/scripts/validate_skills.py
python3 -m unittest discover -s skills/tests -v
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
Routes include `menu`, `host`, `lobby`, `combat`, `help`, `effects`, `inspect`, `order` and
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
