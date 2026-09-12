---
name: architect-bevy-game
description: Design or refactor Bevy 0.19 game architecture, crate boundaries, plugins, schedules, resources, composition roots, or multiplayer ownership. Use for new game structure, deciding whether code belongs in a reusable crate, separating discovery/transport/admission/gameplay, untangling authority, or preventing a shared library from becoming a mandatory engine. Do not use for a narrow implementation whose ownership and scheduling are already settled.
---

# Architect Bevy Game

Preserve independent game composition while extracting stable, opt-in capabilities.

Read `.bevy-gamekit/overlays/architect-bevy-game.md` first when it exists; treat it as adopter context without weakening safety or evidence requirements.

## Workflow

1. Inspect the workspace manifests, application entrypoint, plugins, resources, states, events/messages, and system ordering before proposing boundaries.
2. Identify the authoritative owner of each fact. Require one owner for mutable domain state; use projections or messages at boundaries.
3. Classify proposed shared code with the extraction rubric in `../../references/architecture-boundaries.md`.
4. Keep each game as its composition root. The game selects plugins, schedules, domain models, presentation models, and adapters.
5. Define crate dependency direction before moving code. Capability crates must not depend on consuming games or genre rules.
6. Make schedule ordering explicit only where a real data dependency exists. Prefer public `SystemSet` seams over global ordering.
7. Record failure semantics and test seams with the design. Loading and configuration errors must not silently become initial state.

## Gamekit Awareness

If any `bevy_game_*` crate appears in manifests, read `../../references/gamekit-apis.md`. Treat Gamekit as optional capabilities, not required architecture. For multiplayer, map discovery, encrypted transport, admission, game authority, disclosure, and presentation to distinct owners.

Read `../../references/bevy-0.19.md` before naming Bevy APIs. Apply the evidence boundaries in `../../references/evidence.md` when defining acceptance criteria.

## Deliverable

State the ownership map, dependency direction, public seams, schedule/data flow, failure behavior, and tests. Explicitly identify what remains game-local and why.
