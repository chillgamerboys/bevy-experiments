# Battle UI corrections and character customization

Status: active implementation, authorized 2026-09-14 after planning. Deliver the
four batches as reviewable PRs; merge is not authorized.
Owners: Labyrinth rules/content/app/UI, shared tooltip mechanics in Gamekit, and
GameSkills for workflow evaluation and optimization recommendations.
This is the current plan for the September UI reports and accepted customization
design. It supersedes the older editor terminology and tooltip Escape-only policy.
Preserve the spatial preparation and multiplayer ownership contracts from
[weapons and battle setup](weapons-and-battle-setup.md).

## Outcome and accepted constraints

Keep battle scale stable for every legal roster, make pinned help dismissible and
persistent, organize menus into separate pages, provide compact complete encounter
history, and make character customization reflect one coherent backend model.

- Six formation spaces per side define scale, independent of actor count or corpses.
- Hover previews appear immediately; one second of continuous hover pins them.
  Pinned cards have visible accessible × controls and ignore unrelated inspection.
- Escape opens the Game menu from combat; menus temporarily hide all tooltips.
- Game and Party management are separate pages. Local navigation does not pause
  shared combat; multiplayer assignment pause remains explicit and host-owned.
- Skills are active actions. Abilities are passive effects. Either can come from
  the character or equipment and can require equipment or be independent of it.
- Moveset is the canonical UI and backend name for currently eligible active
  Skills, resolved with applicable Ability modifiers.
- Character-owned selections survive missing equipment as inactive selections.
  Ineligible Skills are absent from Moveset; ineligible Abilities have no effect.
- The editor defaults to Parameters, then Equipment, Skills, Abilities, Moveset.
  Its sprite preview and save controls remain visible. Presets belong to creation.
- Skills and Abilities each include a read-only From equipment section. Such
  grants cannot be added or removed as personal selections through UI or input data.
- Combat history has one compact presentation with the full current encounter,
  rather than a separate bulky History mode or only two recent outcomes.

## Current implementation and remaining delivery

Observed on 2026-09-14:

- Batch1 extends [PR #42](https://github.com/chillgamerboys/bevy-experiments/pull/42),
  branch `codex/fixed-formation-tooltips`, base `main`. Local changes restore ×,
  retain valid pins through menus, separate Party management and preserve visible
  interruption recovery. The obsolete flex-row actor fixture is repaired; its
  focused geometry regression passes. The old remote Escape-only implementation
  and its failed CI remain superseded inputs until the updated branch is published.
- Shared tooltip suspension and focus/close regression tests pass. Combined
  normal1080 UI verification, affected renders and current PR checks are pending;
  historical screenshots or old successful commands do not certify new inputs.
- Batch2's pure rules model is complete in isolated work; 83 rules tests plus one
  Rustdoc and strict rules lint pass. Application/schema consumers are being
  migrated before publication. Editor redesign and compact full-history delivery
  remain required in Batches3/4.
- [PR #41](https://github.com/chillgamerboys/bevy-experiments/pull/41), the separate
  setup/launch audit, remains open. Its normal-window defaults and `--window-size`
  option are absent from this branch. Do not document that option here as available.
- The installed GameSkills bundle is `0.1.0-dev.3`; the local executable is
  `./target/ci/gameskills` version `0.1.0-dev.4`. Installed-pin status passed.
  Project base is still `main`; the proposed `dev` default rollout is separate work.
- Delivery tasks `formation-tooltip` and `character-build-model` bind the actual
  issues/PRs as available. Queue `ui-builds-20260914` tracks isolated workers.
  `.context/gameskills-usage/` retains per-thread implementation counters and
  observed workflow costs. No merge is authorized.

## Requirement and regression inventory

| ID | Report or accepted change | Required result |
|---|---|---|
| U1 | Fewer than six characters zooms the battle and breaks surrounding UI | Six fixed columns per side in live battle and movement preview; identical rank width/art fit at full and sparse occupancy. Multi-rank actors retain their spans. |
| U2 | Corpses disappearing changes scale | Corpses occupy their ranks until removal; remaining actors advance toward the center and leave noninteractive empty back ranks without zooming or moving menus/HUD. |
| U3 | Pinned cards disappear on other hover; × was removed | Keep one-second pinning; other hover, explicit inspection and ordinary/gameplay clicks preserve the chain. × closes that branch and descendants; root × clears the chain. No immediate repin or click-through after closing. |
| U4 | Escape must reach a usable menu; assignment button sits above its title | One Escape from ordinary combat opens a title-first Game menu even with selected actions or pinned help. Game, Settings, Party management and leave confirmation have explicit routes and Back/Escape behavior. |
| U5 | Tooltips conflict with menus | Hide previews and pins, prevent hidden tooltip input/focus, and restore only still-valid pins after leaving the menu. Never restore undisclosed or expired subjects. |
| U6 | Bulky combat history and compact view truncation | A single compact, translucent, scrollable log retains every current-encounter event. Hide/× and Latest remain; mode switches and oversized expandable buttons go. |
| U7 | Editor order, missing sprite, redundant presets and clutter | Parameters first/default; persistent actual character sprite and summary; five agreed tabs; no editor preset chooser; list/details layout and reachable fixed footer. |
| U8 | Innate/Learned/Resulting moves blur mechanics | Canonical Skills/Abilities/Moveset across backend, catalog, runtime, serialization and UI; equipment grants, personal grants and requirements remain distinct. |

For U1/U2, capacity is measured in rank spaces, not actor IDs: the prototype's
five actors already fill six spaces because the wagon spans two. Preparation keeps
its editable gaps and explicit contiguous-deployment rule; this presentation fix
does not silently reposition setup drafts or change corpse timing.

## Batch 1: finish formation, tooltip and menu corrections

Use PR #42 for this coherent stability batch, updating its description and delivery
record when implementation is authorized. Do not rename the current branch.

1. Repair the failing actor-overlay fixture, retain the fixed formation code, and
   update the adopter test that still asserts pinned Tooltip Close is absent
   (`src/ui/tests/overlay_stability.rs`). Its unpinned-preview assertion stays absent.
2. Finish the existing shared × patch. Cover root/nested branch close, keyboard
   activation, focus restoration, stationary-pointer suppression and one-second
   continuous-hover behavior. Preserve pin content through source-entity rebuilds
   using stable subjects; unrelated hover must neither replace it nor add previews.
3. Add an explicit shared tooltip suspension mechanism usable by a blocking game
   overlay. Suspension hides rendering and disables tooltip hit testing, activation
   and keyboard capture while retaining valid pins. Continue catalog/disclosure and
   lifetime invalidation; do not treat a temporary menu as permanent subject loss.
   Resume without replaying pin timing or restoring an obsolete focus target.
4. In Labyrinth, route Escape at the game layer. Outside an active editor/dialog,
   Escape opens Game directly from combat, returns from a subpage, and closes Game
   on return. A pinned chain cannot consume those presses first. Keep text-input,
   top-dialog and dirty-editor handling scoped to their owners. Use an adopter
   setting/seam; do not change every game's Escape policy to Labyrinth's choice.
5. Refactor `src/ui/shell/settings.rs` and `MenuPage` into separate pages:
   - Game: title, Back to game, Settings, Party management when applicable, then
     a separated leave/main-menu action with the existing confirmation.
   - Party management: title, assignments, explicit host pause/resume and Back.
     Mutation controls are multiplayer host-only; local play has no player setup.
   - Settings and Leave: retain their own headings, actions and return route.
   - Required-controller loss, reconnect and halted encounters remain explicit
     interruption states. Navigating menus cannot resume combat or clear a fault.
6. Opening Game or Party management is local navigation. Assignment editing still
   requires the existing explicit host pause. Resume is a distinct authority action;
   returning to Game or Back to game never implicitly resumes shared combat.
7. Preserve focus and usable controls across menu refreshes, including player and
   interruption changes. Suppress tooltips for the editor and blocking confirmations
   as well as Game/Settings/Party pages. Returning restores only valid pinned cards.

Owners: `src/ui/mod.rs`, `src/ui/shell/settings.rs`, `src/ui/battle/actors.rs`,
movement preview and tooltip adopters; shared `gamekit/ui/src/tooltip*` and
`gamekit/docs/ui.md`. Resolve actual enum/module locations before editing.

## Batch 2: one backend Skills, Abilities and Moveset contract

Keep this migration coherent and compiling across consumers before rebuilding the
editor layout. Do not preserve misleading active Ability names behind renamed tabs.

1. In `rules/src/catalog.rs` and `build.rs`, define canonical `SkillDefinition`,
   `AbilityDefinition`, `ResolvedSkill` and `Moveset`. Character builds explicitly
   store personal Skill/Ability selections and equipped items. Equipment definitions
   can grant either type. Grant source and equipment requirements are separate data.
2. Model prerequisites against equipment kinds, with exact-item requirements only
   when authored. Keep the present single-weapon slot; no inventory or new slot
   system is needed. Support explicitly character-selectable versus equipment-only
   content. Enforce this in pure validation, including manually authored scenarios.
3. Use one deterministic resolver for editor previews and encounter freezing:
   gather grants, evaluate requirements, retain inactive-selection explanations,
   deduplicate by definition ID with all provenance, construct eligible Moveset,
   then apply applicable passive contributions in stable order. Apply one effective
   Ability once even if multiple sources grant it; do not multiply upgrades by source
   count. Reject malformed IDs/conflicts, but not a valid inactive personal selection.
4. Removing an item removes that item's grants and reevaluates personal selections.
   Restoring suitable equipment reactivates them. A move-targeted upgrade with no
   eligible target is inactive with a reason. Current rank, targets and remaining
   uses are combat-legality concerns: an eligible Skill stays in Moveset even when
   temporarily unusable this turn. Passives never become action-bar buttons.
5. Migrate all built-in definitions and all ten actor presets, plus stock scenarios.
   Assassin Feint is a Skill; Bleeding Dagger and Duelist Dagger are passive Abilities.
   Move armed attacks such as Scout shots and Medic Staff Strike to suitable equipment
   grants; keep personal support/natural actions as Skills. Record the exact mapping
   in content guidance. Preserve existing effects, use limits and actor identity/art
   unless the accepted mechanics require a change; do not accidentally duplicate or
   remove a preset's usable attacks when equipping its replacement source.
6. Wire canonical names through `model.rs`, `combat.rs`, `resolve.rs`, `preview.rs`,
   `loadout.rs`, `scenario.rs`, app session/presentation and every UI consumer.
   `CombatAction::Ability { index, target }` currently means a general active move,
   while `CombatAction::Skill { skill: SkillId, target }` is a legacy alias. Make
   indexed Skill the canonical action and isolate/rename or remove the old adapter.
   Migrate snapshots, actor/company fields, glyphs, constructor previews, tooltips,
   action rails and history references; no second resolver or alternate active model.
7. Explicitly version changed catalog/scenario schemas, resolver fingerprint salt,
   rules interpretation and network protocol (currently 1, 1, `labyrinth-build-v1`,
   rules v4 and protocol 6). Preserve payload validation and reject stale peers.
   Early-development default: reject incompatible old save formats with an actionable
   message rather than guessing changed semantics; keep original files untouched.
   A save-conversion utility is outside this batch unless separately requested.

Passive scope: use bounded typed effects, starting with the existing move upgrades
and support for equipment-granted passives. Do not build a universal event scripting
engine, reactions, skill trees or progression. Equipment-independent Abilities are
part of the contract even when a specific content example is deferred.

Resilient is included by the user's 2026-09-14 instruction. Use the communicated
recommended default: `ReduceNegativeStatusDuration { amount: 1 }`, applied once
on application/refresh to finite `Debuff` durations, respecting each status's clock,
with a minimum one tick. The user explicitly chose inclusion; the one-tick floor
is the stated implementation default, not a separate quoted user decision.
Do not shorten again on each turn, snapshot or load. Explicit saved remaining
duration is already remaining; avoid double application. Cover application,
refresh, initial conditions, preview parity and repeated serialization in tests.

## Batch 3: implement the accepted character screen

Use the Batch 2 projection in `src/ui/setup.rs`, `setup/layout.rs` and
`setup/details.rs`. Retain one editor for both teams and one draft lifecycle.

- Fixed header: Customize character, Previous/Next navigation, accessible close ×.
- Persistent left column: actual draft sprite, name, compact stat/equipment summary
  and relevant owner/read-only state. Use existing appearance fitting for ordinary
  and multi-rank actors. Do not infer sprite choice from Skills or Equipment.
- One tab row: Parameters, Equipment, Skills, Abilities, Moveset; Parameters opens
  by default. Tabs remain reachable and do not reset edits or scroll unnecessarily.
- Parameters: name, maximum HP, speed, size in ranks; separate Starting conditions
  group for initial HP/statuses. No preset selector or empty inspector column.
- Equipment: compact item list and selected-item details with grants, prerequisites,
  explicit Equip/Unequip and consequences for the draft's Skills/Abilities.
- Skills and Abilities: personal selections/catalog with explicit Add/Remove;
  separate inspect-only From equipment rows, source labels and unmet requirements.
  Abilities display active/inactive state and their effective contribution.
- Moveset: read-only eligible Skills, final values, source labels and applied
  upgrades. A row selection inspects; it never adds a grant or executes a turn.
- Details show descriptions, effects, relevant ranks/targets, uses and requirements
  directly, so editing does not depend on tooltips hidden by the modal.
- Only content/list/details areas scroll. The preview, tabs and footer stay visible.
  Footer: draft status/errors, Discard changes, primary Save & close.
- Preview draft changes immediately. Discard resets while staying in the editor;
  close/navigation protects dirty work with Keep editing/Discard. Save waits for
  host acknowledgment. Preserve stale-revision rejection and useful reload feedback.
- Preserve host/owned-character editing, inspect-only foreign characters, host-only
  placement/size constraints, text caret/input continuity and complete catalog access.

## Batch 4: compact complete encounter history

The current `src/ui/battle/history.rs` compact projection takes only two outcomes;
`src/session.rs` retains 80 typed events and validates that limit on snapshots.
Simply removing a toggle or raising a constant does not provide full history.

1. Replace Hidden/Compact/History with hidden/visible state for the one compact
   panel. Keep initial visibility unchanged (hidden) unless requested otherwise.
   Use concise text rows, the compact translucent appearance and a bounded scroll
   area. Keep the battlefield geometry, actor summaries and action rail fixed.
2. Keep typed events as the source; format action, target and consequences compactly,
   including statuses, movement, death saves, corpse removal and turn/round context.
   Avoid making every event a large expandable button. Inspection may open disclosed
   terms without executing actions or blocking unrelated gameplay input.
3. Full history means the current encounter from its beginning, including after
   reconnect; it does not promise an archive of past games. Host/local authority owns
   the encounter event archive. Keep bounded recent-event snapshots and serve older
   records in bounded pages through game-owned session/network requests. Use existing
   authenticated request transport; do not expand shared Gamekit into a combat logger.
4. Scope pages to encounter identity and ordered event IDs. Validate page count/byte
   limits, request ranges and admitted requester; deduplicate overlaps and fetch gaps
   after reconnect or missed recent windows. Stale responses cannot repopulate a new
   encounter. History fetching has no combat command/turn side effect. Local play
   reads the same archive directly. Preserve disclosure in live and older records.
5. Retain the entire current archive for the encounter lifetime, independently of
   the 80-event snapshot window. Keep UI work proportional to the visible/page range,
   rather than mounting the entire log on every event. If memory requires storage
   backing, use app-owned temporary encounter storage; never silently truncate and
   still label the result full history. Persistent history export is outside scope.
6. Reuse `UiFeedScroll` follow-latest behavior. While reading older entries, incoming
   events or prepended pages preserve the visible anchor; show an unobtrusive Latest
   control/new-event indication. × hides without discarding. A new encounter resets
   history and reading state; finished encounter history remains while that encounter
   is displayed. Menu transitions do not cause log scroll/input to escape modal scope.

History can be developed independently after the Skill/action contract is fixed.
Integrate shared `session.rs`, network protocol and action naming changes serially
with Batch 2; do not have workers overwrite those boundaries concurrently.

## Verification and delivery

Follow [Labyrinth testing](../testing.md) and [repository testing](../../../../docs/testing.md).
Run only the affected suites and required CI checks. Development/Testing use macOS
at normal 1920×1080 Auto for changed UI flows; Windows/Linux are Release coverage.
No routine resolution matrix, six-process suite, generic E2E sweep or repeated
screenshots for backend-only edits. Retain existing compatibility tests without
selecting them automatically. Developer sanity gates milestone promotion only.

| Batch | Focused automated coverage | Rendered/interaction evidence |
|---|---|---|
| 1 | Shared `tooltip::` tests; failing actor fixture; sparse/multi-rank/corpse-clear and movement assertions; menu/Escape/suspension/focus cases; host assignment policy where routing changes | One normal-1080 route exercising sparse battle, pin and ×, menu/subpage navigation, valid restoration and a corpse-clear transition. Reuse unchanged formation evidence only when its inputs still match. |
| 2 | Catalog/build resolution, prerequisites, source restrictions, deduplication/upgrades, all preset mappings, serialization rejection/roundtrip, runtime Skill→preview→apply/use parity; applicable passive semantics; focused customization session and authenticated custom-build network roundtrip | No standalone UI walkthrough for backend-only changes; render final consumers in Batch 3. |
| 3 | `labyrinth-editor` and changed normal-1080 editor cases: ordering, draft sprite, explicit mutation, readonly equipment rows, missing requirements, complete Moveset, save ACK/conflict/discard/ownership | One normal-1080 editor journey through all tabs, equipment removal/reactivation, inspect-only rows and Save/Discard/Close. Include ordinary and multi-rank previews. |
| 4 | Event grouping, >80-event retention, bounded pages, ordering/deduplication/stale encounter rejection, gap/reconnect recovery, disclosure, hidden-state retention and scroll anchoring | Normal-1080 compact log with long history, incoming events while scrolled up, Latest, hide/show and menu input containment. One focused host/guest history path; no unrelated discovery/process coverage. |

During implementation, use configured `gameskills` command/evidence records and
the repository suite selector. Useful existing entries include `ui-tooltip-test`,
`rules-test`, `labyrinth-ui-model`, `labyrinth-ui-normal`, `labyrinth-editor`,
`labyrinth-presentation` and affected session/network suites. Add a focused history
suite only if the existing selector cannot select its meaningful tests. Confirm
filtered commands select tests. Finish with affected-owner lint, formatting and
docs checks; stop once applicable checks pass unless fresh evidence warrants more.

Update current `README`, `docs/{architecture,content,rules,testing}.md`, shared
`gamekit/docs/ui.md` and relevant API docs when behavior lands. Repair old
Innate/Learned/active-Ability/Resulting moves explanations, Escape-only dismissal,
menu ordering, log modes/retention, launch instructions and render route names.
Keep old contracts marked as superseded in active plans; retire completed plans
only after carrying their useful facts into current docs and preserving other work.

For future implementation, resume `formation-tooltip` delivery bound to PR #42
and HEX-112 for Batch 1; use separate reviewable PR batches for the model, editor
and history work, with the actual configured receiving branch observed at that
time. Establish adopted tracking then, not during this plan-only turn. Rewrite
each PR around its final scope and bind acceptance to its actual head and inputs.
Do not merge or create/change the default `dev` branch as part of planning.

Known tooling discrepancy: the local delivery observer applies a developer-sanity
requirement to every main/Testing gameplay PR, but the user requires it only for
milestone promotion. Record that discrepancy for the separate rigor rollout;
do not fabricate a response, misclassify work or add a per-fix approval gate.

## Evaluate GameSkills during implementation

The user explicitly included ongoing GameSkills evaluation, token tracking and
efficiency recommendations. Evaluate the actual four batches as they happen;
evaluation is part of the work, not a separate final retrospective or a requirement
to replay each task with multiple agents. The broader
[skill evaluation plan](../../../../gameskills/docs/plans/skill-evaluation.md)
retains historical evidence and separate native/distribution trial obligations.

Record one compact checkpoint at each batch boundary and a short entry for material
failures/retries. Bind it to the actual source, installed instruction pin, CLI,
client/model/effort where exposed, selected skills and completed outcome. Record:

- Coordinator and every worker's input, cached input, output and reasoning usage
  where provided, plus wall time, retries, review rounds, corrective user feedback
  and tests unnecessarily repeated or selected outside the accepted rigor.
- Raw per-thread start/end counters and their timestamps. Sum thread deltas once,
  including resumed/new workers; do not attribute the whole long conversation to
  this implementation. Cached input is a subset of input, and reported reasoning
  is part of output; do not double-count either. Report unreported end-of-turn work
  or absent counters as unavailable rather than estimating billed tokens.
- Whether guidance selected the right skill, preserved settled decisions and scope,
  helped find a real defect, produced needless reads/tests/agents, or required a user
  correction. Separate observed behavior from inferred causes and self-review from
  independent evidence. Total usage cannot isolate skill overhead by itself.
- A short recommendation: observation, proposed change, expected benefit, evidence
  needed to test it, and any quality tradeoff. Do not claim measured savings without
  equivalent successful outcomes and actual comparable counters.

Telemetry is available from this session's native `token_count` records. A baseline
for the coordinator and ten existing descendant threads is preserved in
`.context/gameskills-usage/baseline-2026-09-14.json`. These are cumulative historical
counters, not implementation cost. Capture a fresh start checkpoint when Batch 1
actually begins. Keep metadata/counters only; do not copy conversation contents.
The implementation ledger is `.context/gameskills-usage/README.md`; carry relevant
findings and per-batch totals into the delivery report. No token ceiling was set by
the user, so report usage without inventing a budget or sacrificing acceptance.

Use these efficiency practices immediately:

1. Read the active plan and relevant owner sections; reuse already-read unchanged
   skill contracts and accepted decisions. Search headings before loading historical
   plans. Read summaries first and fetch full failures only when diagnosis needs them.
2. Reuse source-bound test/evidence results across plan, test, review and audit. A
   later skill invocation does not justify repeating the same check. Run the narrow
   failing test first and broaden only for changed boundaries or unresolved findings.
3. Delegate only independent work with a useful expected outcome and bounded files.
   Prefer compact task packets to copying the entire conversation. Measure total
   coordinator plus worker usage; faster elapsed time alone is not lower cost.
4. Keep one current decision record and lightweight handoff. Preserve evidence, but
   remove superseded scratch instructions after consolidating them. Do not treat
   deleting local files as a reduction in already-consumed conversation tokens.
5. Keep performance/cost observations at batch boundaries rather than running a
   telemetry tool after every action. Read only recent token metadata when possible.

Initial optimization candidates, to assess with actual batch outcomes:

- Compact resumable task summaries and section-level doc pointers. The existing
  evaluation plan is nearly 900 lines; reading it whole in this turn produced
  truncated output. This is an observed retrieval/execution cost, not evidence that
  longer guidance improves quality or that a particular prompt caused the waste.
- An optional native usage-import helper for coordinator/worker checkpoints. The
  existing CLI search found no token-counter ingestion; manual transcript metadata
  extraction works, but a bounded adapter could reduce repeated collection work.
- Reuse evidence by owner/source/command identity across lifecycle skills, and keep
  affected-test selection and milestone-only sanity policy consistent in the CLI.
  The already-observed per-PR sanity mismatch is a concrete candidate for correction.

Recommend narrow changes in canonical `gameskills/plugins/` or CLI source from
observed failures. Preserve the installed evaluation pin. Installing a candidate,
running broad cross-client benchmarks or promoting a release is separate work;
this evaluation does not silently add those gates to ordinary UI implementation.

## Handoff and exclusions

Next step after this plan is implementation when requested. No product question is
currently pending. Establish batch boundaries against current remote state, capture
the usage start checkpoint, and start Batch 1. Parallel work may use isolated owner
files after contracts are agreed;
root integration, Cargo/resource use and external delivery remain coordinated.

No broader inventory, dual wielding, progression, new stat system, sprite authoring,
main-menu redesign, global pause-policy change, host migration or platform support
rollout is included. GameSkills evaluation and recommendations are included as above;
broader native/package release trials and launch-audit delivery remain separate
tracked work, not prerequisites invented for this UI plan.
