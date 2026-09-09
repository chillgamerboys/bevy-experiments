# Labyrinth verification

Run from the repository root. CI profile uses the same source/features with faster unoptimized
compilation; ordinary play uses the default development profile.

```sh
cargo test -p labyrinth_rules --profile ci
cargo test -p labyrinth --profile ci
cargo test -p labyrinth --lib network::tests::process::four_native_processes_survive_guest_kill_and_finish_the_fight --profile ci -- --ignored --exact --nocapture
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
| Session policy tests | Four-player readiness, actor ownership, durable replay watermark, pause/rematch | Physical socket behavior |
| Multi-App socket tests | One host + three encrypted clients, password/direct admission, command convergence, dropped-App recovery, offer/ACK loss | OS process death or cross-machine reachability |
| Explicit four-process test | Abrupt OS guest kill, profile lock release, same-peer/hero exact-state recovery, subsequent completed fight | Cross-machine LAN/Tailscale reachability |
| Fake discovery tests | Provider-neutral listing/removal/compatibility and encrypted join handoff | Real multicast or Tailscale |
| UI behavioral/structural tests | Typed intents, focus/modal behavior, target eligibility, viewport control bounds | Visual quality or real pointer hardware |
| Native rendered frames | Static composition at the captured logical sizes | Interactive behavior or four-player correctness |
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
Routes include `menu`, `host`, `lobby`, `combat` and `paused`. The offscreen render
uses exact logical dimensions rather than the desktop's window-size limit. Check
actor/rank readability, HP/status duration, action requirements, focus/disabled
contrast and inspector/activity scrolling. Do not approve from dimensions alone.

## Interactive four-player checklist

Use independent profiles as described in [the game README](README.md).
Keep artifacts in a private temporary directory and redact codes and credentials.

- Direct route: both discovery providers disabled, three different invitations,
  all four ready, correct hero ownership and host-only start/rematch.
- Same LAN: host discovery enabled, a passphrase, real guest listing present for
  at least a minute, wrong password rejected and correct password admitted. Check
  occupancy updates and removal when the host closes.
- Remote tailnet: Tailscale installed/authenticated externally; explicit runtime
  opt-in, reachable reported address, provider errors visible, password join and
  actual combat. This does not test Steam identity/lobbies/relay.
- Play out a turn involving damage, bleed, cleansing and movement. Confirm that
  status remains on the actor after movement and initiative remains frozen for
  that round. Inspect disabled actions via both pointer and keyboard.
- Kill a guest **process** during combat. Once detected, the host pauses without
  advancing initiative/statuses. Restart the same profile, reconnect without the
  passphrase, verify the same peer/hero and exact boundary, then act successfully.
- Close the host. All guests become disconnected; a new host process is a new
  session, not a persisted campaign.
- Resize during action selection and modal use. Traverse all relevant controls
  at automatic/200% semantic scale, scroll the inspector/feed, and check focus
  restoration. Review damage/bleed feedback without allowing animation to gate turns.

Cross-machine LAN/tailnet and interactive results must be recorded with the tested
route and build. Passing deterministic CI is not a substitute for these manual gates.
