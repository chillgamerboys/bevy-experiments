# Character editor design constraints

The selected direction is squad preparation with one unified character editor
for heroes and enemies (grill answers 7–8). Answers 9–10 refined the preparation
shell into facing spatial formations with direct ownership assignment. These
constraints survive UI redesigns. Current implementation belongs to the
[architecture](architecture.md); remaining human acceptance belongs to the
[customization follow-up](../../../docs/plans/labyrinth-customization-epic.md).

## Accepted unified character screen contract

- Squad preparation selects a hero or enemy and opens the same character screen.
  Equipment, innate abilities, learned skills and character parameters belong to
  that screen. Do not create a second advanced, testing or enemy character editor.
- Internal sections/navigation and bounded scrolling organize the one screen;
  unified does not require every catalog entry and field to be visible at once.
  Keep character identity, inspected versus equipped state, the resulting moves
  and one draft/apply/discard lifecycle coherent across sections.
- The future character stat system is not implemented and is not introduced by
  this UI revision. Existing prototype fields for HP, speed, footprint and starting
  conditions are battle parameters, not a completed character attribute/progression
  system. Preserve their existing customization in this same editor; future stats
  join it when designed. Do not invent attributes or progression now.
- Scenario presets, seed, save/load and participant assignments belong to battle
  preparation. They must not become another route for editing character builds or
  character parameters through a separate editor.
- Expect the screen to be redesigned over time. Keep one game-owned character
  presentation and editing flow reusable across entry points, including a future
  detailed in-game character view. Editing permissions depend on context and
  authority; future reuse does not grant mid-battle build editing now.
- Preserve host/guest authority and both-team customization. A compact layout or
  read-only viewing state remains the same character screen, not a forked UI.

## Interaction requirements

- Inspect first; explicit Equip/Learn updates a clearly marked draft. Show all
  added, removed and upgraded moves and their provenance from the actual resolver.
  A move retained through another grant must not be falsely marked lost.
- Give each browser row a short tactical description. Details show effective
  damage/effects, source and target ranks, target pattern, uses, prerequisites and
  interactions. Labels and rank diagrams reinforce each other; no unexplained dots.
- Distinguish currently unusable from absent: dagger throw still belongs to a
  front-rank character, but its current rank restriction must be explained.
- Scope scrolling to the active browser/details, preserve character context and
  Apply/Discard access, and avoid stacking every category on a giant form. Large
  text/narrow windows use list-to-detail navigation with Back preserving position.
- Core explanations work through selection/focus, not hover alone. Long condition
  explanations can be inspected separately. Complete builds remain inspectable.
- Both teams retain full authorized build/formation and existing battle-parameter
  customization. Character parameters stay in the unified editor; scenario/file/
  seed controls stay in battle preparation. Content definitions remain authored
  outside the UI. No inventory grid, character stat system or skill tree is added.

## Spatial construction and ownership

Choose a rank and inspect a character type before placement. Construction may
contain gaps, but deployment requires occupied ranks contiguous from the front.
Trailing unused capacity permits small encounters. Explain blocking gaps; do not
silently compact or introduce dummy actors. The sparse draft is distinct from the
compact authoritative battle scenario and preserves identity through replacement
and multi-rank occupancy.

Manage ownership on the formation with a compact player strip for readiness,
connection and assignments. Connection details belong in an expandable lobby
panel. Host owns starting places/enemies; players customize assigned characters.
Zero or multiple owned characters and spectators remain valid.

## Evidence and future design work

The earlier button lists and card-grid preparation were rejected by the user.
Passing geometry tests and unclipped screenshots did not establish understandable
choices. Future revisions should specify real content, facing formations, rank
anchors, selected/owned/empty states, character picker, hierarchy, spacing,
contrast and responsive behavior, then observe complete tasks.

Retain that negative evidence and independent evaluation requirements in the
[GameSkills evaluation](../../../gameskills/docs/plans/skill-evaluation.md).
The [original research and alternatives](https://github.com/chillgamerboys/bevy-experiments/blob/bf42159a8b2872ca293fafe1a5e5a66eb4f1f666/games/labyrinth/docs/plans/builder-ui-research.md)
are historical context, not another implementation queue.
