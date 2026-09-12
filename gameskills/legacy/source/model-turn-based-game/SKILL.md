---
name: model-turn-based-game
description: Model, implement, or review deterministic turn-based gameplay in Bevy 0.19, including ordered participants, rounds, actions, legality, phase transitions, removal, and terminal states. Use for card games, tactics games, board games, initiative systems, or replayable reducers. Do not use for real-time gameplay with no discrete decision ownership.
---

# Model Turn-Based Game

Separate generic sequencing from game-owned rules and effects.

Read `.bevy-gamekit/overlays/model-turn-based-game.md` first when it exists; treat it as adopter context without weakening safety or evidence requirements.

## Workflow

1. Name stable participant and object IDs. Never use entity allocation order as domain identity.
2. Define the authoritative domain state and the complete typed action vocabulary.
3. Specify legality as data or pure queries before applying effects.
4. Make one reducer/system responsible for each mutation. Return typed outcomes instead of inferring success from UI or logs.
5. Define round start, advancement, participant removal, empty roster, and terminal behavior explicitly.
6. Test boundary actions, invalid actions, repeated actions, removal at every cursor position, wraparound, and deterministic replay.
7. Project immutable presentation models after domain mutation; UI emits intent and never writes authoritative state directly.

## Shared Sequencing

When the game uses `bevy_game_turns`, read `../../references/gamekit-apis.md`. Use `TurnOrder` only for roster/cursor/round facts. Keep cards, mana, movement, combat, AI, timers, and victory conditions in the game.

Read `../../references/architecture-boundaries.md` when considering extraction and `../../references/evidence.md` before choosing tests.

## Invariants

- Equal initial state plus equal ordered actions produces equal outcomes.
- A rejected action leaves authoritative state unchanged.
- Advancing is explicit; unrelated removal or rendering never consumes a turn.
- Presentation cannot create facts absent from the domain model.
