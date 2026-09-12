---
name: test-bevy-game
description: Add, repair, or organize deterministic tests for Bevy 0.19 games, plugins, schedules, deferred commands, state transitions, UI structure, persistence, or simulation rules. Use when selecting test altitude, building headless apps, writing quick/full gates, or proving Bevy behavior. Do not use when the request is only manual visual critique.
---

# Test Bevy Game

Use the narrowest test that exercises the real owner of the behavior.

Read `.bevy-gamekit/overlays/test-bevy-game.md` first when it exists; treat it as adopter context without weakening deterministic or merge-gate requirements.

## Test Altitudes

- Pure unit test: algorithms, reducers, serialization, and edge values without an `App`.
- Minimal app: resources, messages, system ordering, state transitions, and deferred commands.
- Headless UI: real input, focus, hierarchy, layout, and semantic structure without a renderer.
- Runtime/visual: asset loading, rendering, animation, camera, and feel.

## Workflow

1. State the claim and identify its authoritative owner.
2. Build only the capabilities the owner requires; hidden convenience plugins can mask missing production wiring.
3. Use deterministic time, seeds, inputs, and stable IDs.
4. Run enough frames for state transitions and deferred commands, but keep all waits bounded.
5. Assert typed state and messages before logs or rendered text.
6. Test edge values, rejection without mutation, round trips, repeatability, and schedule boundaries.
7. Keep a focused quick gate for iteration and a workspace-wide all-feature gate for integration.

For multiplayer, use fake providers for deterministic discovery logic, but do not
describe an in-memory link as transport evidence. Add a real bounded socket test for
connection-code handoff and authenticated gameplay. Prove listing, route selection,
authentication, authority, disclosure, disconnect, and reconnect as separate claims.
A same-machine socket test cannot prove cross-machine firewall or multicast behavior;
reserve real multicast and Tailscale for explicit integration jobs.

If `bevy_game_test` is present, read `../../references/gamekit-apis.md`. Always apply `../../references/evidence.md`; a structural snapshot cannot prove gameplay and a screenshot cannot prove interaction.

Read `../../references/bevy-0.19.md` before assembling plugins or schedules.
