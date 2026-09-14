# Builder UI research and alternatives

Status: active follow-up acceptance context; prior direction implemented in PR #39. The current
[battle UI and character customization plan](ui-and-character-builds.md) carries
the subsequent user feedback and supersedes this document's editor terminology
and layout details. Answers 9–10 remain the spatial-construction and ownership
foundation. Preserve separate S6/S9 package/evaluation acceptance in the
[customization epic](../../../../docs/plans/labyrinth-customization-epic.md).

## Problem and player tasks

The rejected editor exposed data fields and selectable names but did not explain
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
Questions 1–8 remain settled; answers 9–10 below refine preparation without
forking the character editor. Future material questions continue at 11. Specify
the visual design and full interaction journeys before the next implementation.

Prototype the real tasks before another production UI pass: compare
dagger/greatsword from rank 1 and rank 4; inspect a learned upgrade; remove a redundant
grant; switch heroes with unapplied edits; configure an enemy and relaunch a preset.
Check accurate visible information separately from native interaction and actual
player comprehension. Keep the rejected captures and user correction as negative
evidence in the [GameSkills evaluation](../../../../gameskills/docs/plans/skill-evaluation.md).

## Prior card revision and review findings (subsequently rejected)

Preparation now separates Party, Enemies, Scenario and Players. Formation cards
show rank, ownership, weapon and current battle parameters. Both teams enter the
same character editor; previous/next navigation changes its subject. Equipment,
Innate, Learned, Parameters and Resulting moves are sections of one draft, with
Apply/Discard controls outside the scrolling body. The editor opens as a full pane
to give decisions enough space; it retains the selected actor's team/rank/owner
context rather than squeezing a twelve-actor roster beside the inspector.

Wide layouts pair a choice browser with its inspector. Narrow layouts and larger
text use a browser/detail route with Back to choices. Browsing does not equip or
learn. Effective damage, acting/target ranks, affected targets, prerequisites and
actual added/removed/changed moves support an explicit draft change. Switching
away from a dirty actor requires an explicit discard decision. Existing numerical
fields remain prototype battle parameters, not a new character-stat system.

Rendered review found two further problems and drove corrections: reusing the
combat HUD breakpoint created one oversized squad card at 1280 pixels, and verbose
introductory copy pushed all mechanical facts below the first fold at 200% text.
The roster now uses its own readable-width rules. The inspector puts effective
move facts before optional description/provenance; accessibility text is not
shrunk to make it fit. These are observed information/layout corrections, not
evidence that a player understood or enjoyed the new route.

Production-plugin tests cover source-bound inspection, explicit choice changes,
rank-only upgrades, duplicate grants, prerequisites, draft retention, stale
authority, compact navigation and visible footer/mechanics. Metal frames establish
rendered appearance. Desktop automation is unavailable in this session, so the
pointer/keyboard usability walk and human comprehension remain pending. The
original rejection remains negative evidence in the evaluation ledger.

## Hands-on feedback and accepted construction model

The user tried the running game and rejected the preparation/player menus as
clunky. Removing and adding an actor did not offer a type choice before creation.
The card grid represented the formation through text instead of using the game's
six-rank spatial presentation. This is a second negative usability observation,
not a successful usability result inferred from previous test/render passes.

9. **Settled:** constructor positions may contain gaps while assembling a lineup.
   Deployment must reject a lineup with internal gaps; combat remains compact.
   This does not introduce empty ranks into combat rules. Retain smaller test
   rosters: their occupied footprint is contiguous from the front, with unused
   capacity behind it. Show which gaps block deployment. Do not silently compact
   the draft or change the player's selected positions on deployment. A separate,
   explicit compact action can be considered in the detailed design.
10. **Settled:** ownership is managed directly on the formation, with a compact
    player strip for connection/readiness/assignments. Invitations and connection
    details belong in an expandable lobby panel. Preserve the earlier host-owned
    assignment of places and player-owned selection/customization of characters,
    including zero/multiple characters and spectators.

The constructor therefore needs a spatial draft distinct from the compact
authoritative battle specification. It must retain position/subject identity
through empty slots, actor replacement and multi-rank occupancy, then validate
before constructing a deployable scenario. Do not misrepresent gaps as dummy
actors or change targeting/death-compaction rules merely to support this UI.

## Spatial design artifact and review

The implementation specification in `.context/spatial-ui/visual-spec.md` uses
actual available character art, authored
types and representative long names. Specify the facing formations, rank anchors,
selected/hovered/owned/empty states, character-picker placement, player strip,
primary actions, detail surfaces and secondary test controls. Describe proportions,
spacing, type hierarchy, contrast/emphasis, information density and responsive
changes; a list of panels and controls is insufficient. Preserve the unified
editor, explanations, comparisons and the settled authority rules.

Walk these tasks in the design and later in the rendered production route:

- Select a rank, inspect a character type and its footprint/build, preview placement
  and choose it before mutating the lineup. Repeat for an enemy.
- Replace and move an existing character; show displacement/collision consequences
  before committing, including a two-rank creature.
- Leave an internal gap during construction, recognize why deployment is blocked,
  and repair it without losing the selected types/builds/owners.
- Assign places/characters to a player from the board, let that player customize
  within authority, and recognize multiple-character ownership or spectating.
- Open the same character editor, compare a weapon/technique, return with retained
  context, and launch a valid small or full test encounter.

Critique composition and task friction as well as factual information and input
correctness: what draws attention first, what is hard to recognize, which action
is ambiguous, where context is lost, how much navigation/repetition is required,
and whether extra panels/text conceal the game. Preserve hierarchy and accessible
explanations while making the interaction more visual. Findings need a concrete
state/task, observed consequence and correction; unclipped controls alone cannot
accept the design. Human feedback, agent visual judgment and automated checks
remain distinct.

The spatial candidate is implemented at `c1afeea`, with the model follow-up at
`615d5fc`. The board retains sparse positions and shows a chosen type before an
explicit place/replace action. Multi-rank movement targets exact ranks, ownership
is assigned in the selected-place context, and both teams still open one editor.
Deploy never compacts a draft; local play has one atomic Deploy action without a
hidden co-op readiness step. Portable saves remain complete battle scenarios.

Coordinator review of actual Metal frames drove further changes: remove redundant
local participant controls, put type mechanics and current-rank usability before
secondary details, show collision reasons above the browser, distinguish disabled
placement, and shorten the board's unused headroom. Auto and 200% retain facing
rank context; the compact picker switches between types and focused detail.
The player strip shows all six participants at Auto with names, readiness and
character counts. The five final 1280 states and review notes are retained under
`.context/spatial-ui/`. These are agent visual judgments, not acceptance by the
user who rejected the preceding revisions. Production input/policy/socket checks
and final publication evidence remain separate from that human observation.
