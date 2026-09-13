# Weapons, character builds and configurable battles

Status: active UI acceptance revision; both earlier preparation designs were rejected. [Selected UI direction](builder-ui-research.md): facing spatial construction with one unified character editor; implemented, with final delivery and interactive usability acceptance outstanding.
Owner: Labyrinth. GameSkills and Gamekit have equal-priority evaluation and
improvement outcomes in the [combined session plan](../../../../gameskills/docs/plans/skill-evaluation.md).
Tracking: [HEX-99](https://linear.app/chillgamerboys/issue/HEX-99/epic-labyrinth-customization-and-configurable-co-op-battles-with); see the
[detailed epic and child issues](../../../../docs/plans/labyrinth-customization-epic.md).
Current endpoint: reviewable implementation PR and observed delivery, not an unrequested merge.

## Accepted scope

Use squad preparation (grill answer 7) and exactly one unified character editor
(answer 8) for heroes and enemies. Equipment, innate/learned abilities and existing
character battle parameters share that screen and one draft lifecycle. Future
character stats belong there when implemented; this revision does not introduce
that stat system. Plan for the same screen to later serve as the detailed in-game
character view, with context-appropriate edit permissions. Internal organization
may evolve, but do not fork separate character editors for testing or gameplay.

Answers 9–10 refine preparation: choose a rank and inspect a type before placement,
allow gaps while constructing, require contiguous occupied ranks from the front
before deployment, and assign ownership on the board with a compact player strip.
Trailing unused capacity supports small tests. Combat stays compact; sparse lobby
metadata is not a new battle rule or portable scenario schema.

Moves come from innate grants, learned skills and one equipped weapon. Learned
skills can add active moves or upgrade existing ones. Behavior and provenance are
separate: an ability may be active or eventually passive, and its source may be
innate, a weapon, or a named discipline such as Assassin or Pyromancy. Actual
passive bonuses/reactions, including retaliation, remain deferred.

All granted active abilities remain available. Use an automatic, readable ability
layout; favorites, rearrangement and customizable hotbars are future work. Do not
silently truncate at the current eight-ability limit. Pointer/keyboard access,
inspection, targeting, previews and confirmation must work for the complete list.

Fully configure both teams and supply editable stock scenarios. Preserve the
working six-spaces-per-side formation contract, including multi-space actors.
Items, skills and actor presets should be easy to author outside the UI. The
setup UI chooses builds and encounter parameters; it need not author new effects.

Co-op players can own zero, one or several characters. Host assigns starting
formation places and ownership, controls unclaimed heroes, and owns enemy setup.
Players customize their assigned builds. Spectators have no character commands;
losing all characters also produces spectator behavior. Host can reassign surviving
characters during battle using a paused assignment flow. AI controls enemies by
default, with a controller interface reusable by the eventual RL gym.

## Implementation sequence

1. **Content and build model.** Replace the fixed-catalog authoring bottleneck with
   stable definition IDs and validated weapon/ability/learned-skill/actor catalogs.
   Existing effects remain typed Rust behavior; a new item using those effects is
   a definition change. Keep loading/I/O outside pure combat. Separate definitions,
   grant provenance, upgrades and mutable actor state. Resolve prerequisites,
   duplicate grants, ability ordering and conflicting upgrades deterministically;
   reject unsupported conflicts rather than depending on file order. Retain grant
   and upgrade sources for inspection. Do not build inventory storage or skill trees.
2. **Symmetric encounter input.** Define a validated battle specification for both
   rosters, stable actor IDs, order/footprints, builds, stats, starting conditions
   and seed. Freeze resolved content at encounter start. Presets and saved scenario
   inputs use the same validation path as manual setup and future simulation.
   Include content/rules/configuration identity in reproducibility evidence.
3. **Weapons and multi-target resolution.** Add the small proposed weapon roster
   below and representative learned additions/upgrades. Extend legality, action
   enumeration, transactional effects, forecasts and AI together. For cleave,
   resolve ranks 1–2 to distinct targets before effects: a two-rank actor is hit
   once and clearing remains never selects a new replacement target mid-action.
   Damage, reach and technique details are provisional tuning decisions to record
   in content and test against authored scenarios. Dagger throws are unlimited.
4. **Ownership and session lifecycle.** Separate admitted players from party actors
   and formation positions. Each hero has one controller; a player has zero or
   more heroes. Validate host-only assignment/enemy edits and player-owned build
   edits, ready-state invalidation and footprint capacity. Paused reassignment
   changes only control. Reject stale commands from a former controller even if
   the same actor/turn is still active; reconnect observes current ownership.
   Preserve initiative, HP, conditions and spent uses. Treat a rescueable Dying
   character separately from permanent death. A spectator disconnect does not
   block combat; loss of a required controller pauses until reconnect or deliberate
   host reassignment. An assignment action cannot clear a rules-failure suspension.
   Separate participant bounds from formation bounds; retain the existing six
   participant limit initially. No host migration or persisted host-world recovery.
5. **Setup and battle UI.** Add local/co-op scenario setup, save/load, stock choices
   and repeat-seed restart. Show assignment by player plus formation order, with
   explicit spectator/unclaimed states and host-only editing where appropriate.
   Refresh safely when peers join/reconnect while the screen is open. Players edit
   each assigned character's build. Expose every active move in stable automatic
   groups/rows with measured overflow handling and provenance in inspection.
   Switching viewed characters never grants action authority; confirmation uses
   the active character's current ownership and legal action.
6. **Verify and deliver all outcomes.** Integrate the owner checks below with the
   GameSkills/Gamekit evaluation graph. Fix observed package deficiencies and
   verify affected consumers. Update current game rules/architecture/testing and
   package guidance with actual implemented contracts. Preserve the requested PR
   endpoint and report observed delivery, findings and manual/platform gaps.

## Proposed initial content and scenarios

The first roster exercises distinct rules with a small catalog: dagger (stab,
unlimited throw), greatsword (front-two-rank cleave, single-target thrust), two-handed
axe (overhead chop), spear (reach thrust, shove), bow (aimed/flexible shots), and
staff (strike/push). These are initial definitions to tune, not final balance or
additional inventory commitments. Include at least one learned new move and one
learned upgrade so both build-composition paths are demonstrated in real play.

Stock battle options: the current encounter adapted to the new builds; a weapon
comparison encounter; large-footprint/cleave cases; and rescue/status cases. Allow
small rosters for isolated tests as well as full formations. Document each preset's
purpose and seed. Author balance changes through catalog or scenario data; the
future gym should not need to patch class enums or maintain alternative rules.

## Acceptance and evidence

- Add/edit a weapon using existing effects through a definition alone. Invalid
  IDs/references, malformed bounds and incompatible upgrades fail with useful
  diagnostics. Original definitions and mutable per-actor uses remain separate.
- Different sources can grant the same move without duplicate UI actions; upgrades
  and provenance survive setup, snapshot, reconnect and inspection. All granted
  moves beyond eight can be inspected, selected, targeted and confirmed.
- Custom heroes and enemies support stats/builds independent of visual presets.
  Saved scenario reload plus the same content/seed/actions reproduces combat.
  Invalid scenarios fail before starting rather than partially applying a roster.
- Cleave preview and commit agree; multi-space targets are hit once; lethal and
  corpse transitions do not retarget newly exposed actors; rejected actions leave
  state and RNG untouched. AI chooses through the same legality surface.
- Exercise one player with multiple characters, a deliberate spectator, death to
  spectator, rescue, reassignment, stale former-owner input, disconnect/reconnect,
  and a roster update while the assignment UI is open. Reassignment cannot grant
  an extra turn or refill resources, and spectators cannot submit combat actions.
- Run configured game/rules tests, lint, formatting and docs checks, plus affected
  UI/session/socket/process tests. Inspect native frames and pointer/keyboard
  routes at supported layouts; code tests alone do not prove usability. Record
  any unavailable cross-machine/native evidence against the tested build.
- Gamekit acceptance includes exercised control/focus/layout/testing contracts and
  justified fixes with adopter checks. GameSkills acceptance includes actual skill
  selection, useful guidance, misses/friction, user corrections and observed PR
  delivery. Use the combined plan for candidate identities and supplemental cases.

## Deferred work

Inventory storage/layout, acquisition, consumable ammunition, retrieval, offhand
equipment, dual wielding, progression/skill trees, passive/reaction execution,
custom hotbars, manual enemy-control UI and a complete RL training integration.
These deferrals do not remove the selected data, ownership and controller seams.
