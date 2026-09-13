# Builder UI research and alternatives

Status: active implementation of the direction selected 2026-09-13: B with one unified character editor.
Detailed layout and usability validation remain outstanding. Reopens S6/S9 in the
[customization epic](../../../../docs/plans/labyrinth-customization-epic.md).
The research/grill direction decision is settled; carry it into the scoped UI
revision. Preserve all accepted customization, authority and inventory limits.

## Problem and player tasks

The integrated editor exposes data fields and selectable names but does not explain
weapon/skill decisions. Scrolling makes controls reachable; it does not organize
decisions. A player must be able to inspect without changing a build, understand
what an item grants, compare the resulting moves, see rank restrictions and skill
interactions, then apply or undo a draft. A host must also configure both formations,
switch among twelve actors and repeat a controlled test without rebuilding a party.

## Reference findings

- [Game Rant BG3 character creation guide](https://gamerant.com/baldurs-gate-3-character-creation-guide-ui-explained/),
  updated August 8, 2023: sections can be revisited in any order; the inspected class
  and cantrip screenshots show section navigation, a focused choice area, character
  presentation and a persistent build summary. Available and selected cantrips are
  separate. Borrow hierarchy and accumulated-result context, not its number of
  systems, cosmetic editor or prepared-spell limitations. This is a historical UI
  reference, not a source for current BG3 balance rules.
- [Larian's 2022 HUD redesign rationale](https://baldursgate3.game/news/community-update-15-absolute-frenzy_49):
  action categories, searchable character information and equipment-slot filtering
  reduce the set being browsed while retaining access to the whole. Applied here:
  weapon/innate/learned categories and a complete resulting moves view.
- [XCOM 2 official manual](https://cdn.cloudflare.steamstatic.com/steam/apps/268500/manuals/XCOM2_Manual_English.pdf?t=1600177724),
  pages 3 and 11: soldier switching and an Armory combining soldier customization,
  loadouts and upgrades. Its tactical actions also support inspection before use.
  Applied here: keep squad context available while editing one actor; separate
  scenario controls from the selected character's equipment decision.
- [Monster Hunter: World equipment upgrade support](https://www.capcom.co.jp/support/faq/platform_ps4_monsterhunter_world_0140331.html):
  Capcom describes comparisons before confirmation and equipped-state markers.
  Applied here: current versus proposed build, with inspected and equipped states
  visibly distinct. No crafting, ownership inventory or upgrade tree implied.
- [Blizzard's Diablo IV launch design discussion](https://news.blizzard.com/en-gb/article/23938756/make-sanctuary-yoursplay-your-way-in-diablo-iv):
  explicitly aims to explain connections between skills and their synergies.
  Applied here: a learned technique should name the move it changes and display
  that effective change in place. A large skill tree is not justified by our scope.
- [Xbox Accessibility Guideline 114](https://learn.microsoft.com/en-us/xbox/accessibility/xbox-accessibility-guidelines/114):
  context, meaningful grouping, predictable effects and definitions of game terms
  help users understand a menu before interacting. Applied here: core explanations
  remain visible; hover is supplementary. Disabled choices explain their reason.

These applications are design recommendations inferred from sources, not usability
results for Labyrinth. Game Rant HTML and two screenshots were retrieved directly
after the web tool returned 502; the article and screenshots were inspected.

## Alternatives

**A — Character dossier.** Battle overview opens a dedicated character screen.
Left: identity and free navigation between Weapon, Innate, Learned, Stats/conditions.
Middle: one browsable category with names and short tactical descriptions.
Right: persistent inspected details and resulting build. An always-available footer
shows unapplied changes and Apply/Discard. Most space for understanding one build;
more navigation when editing many characters. Closest to the supplied BG3 reference.

**B — Squad preparation.** Party/enemy formation strip stays visible above a selected
actor workspace. Each actor shows rank, owner and weapon. The workspace has category
navigation, a browser and persistent current/proposed comparison. Separate Scenario
and Players sections carry presets/seed/test parameters and ownership. Best starting
point for repeated 6v6 editing; less room for a large character presentation. At
small sizes, open actor details as a full pane rather than squeeze all columns.

**C — Guided creator.** Preset/identity → weapon → abilities → stats → review, with
progress and revisitable sections. Each page explains one kind of choice and keeps
a compact build summary. Strongest for a first unfamiliar character; repeated
editing of twelve actors involves more navigation. Avoid a compulsory linear wizard;
this may later serve as optional onboarding over A or B.

Selected: B as the battle preparation shell with exactly one unified character
editor. A and C above are retained as considered alternatives, not additional
screens to implement. The initial layout concepts in
`.context/ui-research/labyrinth-builder-options.html` predate answer 8; they illustrate
organization, not an accepted detailed layout, final artwork or working interface.

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

## Common requirements to carry into the selected design

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

## Discussion and verification

7. **Settled:** B, squad preparation.
8. **Settled:** one unified character editor, including future stats when they are
   implemented. The user explicitly rejected separating character editing into
   different editors and expects this screen to become a detailed in-game view.
   Multiple redesigns are acceptable; multiple character editor screens are not.
Questions 1–6 remain settled. Future material questions continue at 9. No further
interview is needed for these decisions; detailed layout is an implementation task.

Prototype the real tasks before another production UI pass: compare
dagger/greatsword from rank 1 and rank 4; inspect a learned upgrade; remove a redundant
grant; switch heroes with unapplied edits; configure an enemy and relaunch a preset.
Check accurate visible information separately from native interaction and actual
player comprehension. Keep the rejected captures and user correction as negative
evidence in the [GameSkills evaluation](../../../../gameskills/docs/plans/skill-evaluation.md).
