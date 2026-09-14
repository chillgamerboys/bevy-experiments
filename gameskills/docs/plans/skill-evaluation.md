# Labyrinth mechanics session and GameSkills evaluation

Status: active; current evaluation follows the September battle UI and character
customization iteration. PR #39 is merged; earlier endpoints and observations below
retain their original scope and do not authorize merging later PRs. Broader
independent native behavioral and comparative cost trials remain unobserved.
Owners: Labyrinth for mechanics/playability; GameSkills for workflow quality; Gamekit for reusable capability quality.
Historical tracking: [HEX-98](https://linear.app/chillgamerboys/issue/HEX-98/strengthen-gameskills-delivery-and-add-optional-linear-workflows), merged PR #38. These are not the new implementation's delivery identities.

## Scope and priority

### Current iteration: useful guidance and total usage

The user reconfirmed ongoing evaluation and token efficiency on 2026-09-14. Use
the current [UI implementation plan's evaluation section](../../../games/labyrinth/docs/plans/ui-and-character-builds.md#evaluate-gameskills-during-implementation)
for the four batches, token checkpoints, quality observations and optimization
recommendations. Read that compact current scope before retrieving historical
sections below. This round has native per-thread token telemetry available; older
unavailable-telemetry statements remain accurate for their original trials.

Evaluate actual implementation without replaying every task or imposing a broader
benchmark suite. Keep normal-1080 affected UI verification, focused logic/E2E checks,
macOS Development/Testing and milestone-only developer sanity. Record combined
coordinator/worker usage, cache counts, retries and outcomes without double-counting
cumulative counters. Candidate promotion and cross-client efficacy claims still
require their own evidence; this iteration does not establish those automatically.

### Observed costs during the current implementation

The pure rules owner verified Skills/Abilities/Moveset and Resilient with 84 rules
regressions and one Rustdoc, without unrelated UI or platform sweeps. Batch1's normal
UI suite reached 60 passing tests after correcting old Escape-only fixtures and real
close-focus/menu-recovery defects. These are actual engineering outcomes, not a
controlled token-savings comparison.

- Routine queue JSON repeats the entire plan. The coordinator now extracts status,
  revision and errors; a supported concise output mode is a candidate improvement.
- Queue injection help omits required `reason` and `verified_source`, causing a
  failed attempt and source lookup. Provide a compact valid example in help.
- `docs-check` omitted `git_refs = []`, so unrelated branch/ref changes made a graph
  stale even though its source and all commands were unchanged/passed. Its project
  command now declares worktree inputs and the runner still binds the receiving
  base. Preserve the stale record; never relabel it as accepted evidence.
- `verification_context::resolve` treats every gameplay PR to a Testing branch as
  a milestone. This does not represent the user's milestone-promotion-only intent
  while ordinary development still targets main. Add an explicit promotion trigger
  rather than inferring a milestone from the branch alone; no fabricated manual
  sanity observation or scope downgrade is acceptable.
- Native per-thread deltas are recorded in `.context/gameskills-usage/`. Cached
  input is part of input; reasoning output is part of output. Root coordination
  and backend work overlap in time, so allocate by thread/batch rather than adding
  cumulative session totals or claiming all parallel work belongs to Batch1.

- Batch2 CI initially stopped because four changed protocol/reconnect tests were
  missing from the selected suites. They now belong to the existing exact gameplay
  group, including the protocol-listing rejection; a full discovery sweep is not
  needed. Run changed-test classification before publishing migrated tests.
- Shared Cargo targets can reuse incompatible local rules metadata across worktrees.
  Serialize compilation and clean only `labyrinth-rules` when switching incompatible
  APIs; retain expensive Bevy dependency artifacts.
- A broad `history::` filter accidentally selected eight existing UI tests, including
  compatibility cases. Record that extra cost honestly; do not rerun passing tests
  just to improve the label. Configured history filters should name owner modules.
- Bounded independent review found stale editor source labels after equipment changes.
  It caught a real correctness issue while normal-1080 tests and three affected
  rendered pages supplied distinct behavior/layout evidence. Fresh review tasks can
  receive a small file/acceptance brief instead of inheriting the full transcript.

- History review found that fixed virtual rows clipped valid long names without a
  reveal route. Adding a full-text tooltip exposed a second issue: word-only wrapping
  still clipped an unbroken name. One normal-1080 regression now measures horizontal
  fit and keyboard access to the final line, plus pin/virtualization/menu/disclosure
  lifecycle. This supports testing relevant content extremes at the normal target,
  rather than replaying a resolution matrix.
- Broader CI caught an old lobby→editor header assertion missed by the editor suite.
  The correction preserves actual occupied ranks in the new preview and tests that
  exact moved-wagon route at normal1080. Search consumers of renamed semantic UI
  identifiers before selecting verification; UI owner directories are not the whole
  interaction dependency graph.
- PR43's CI selector change triggered workspace-wide macOS Testing fallback; its
  Rust job took 14m7s. Local checks stayed scoped. Investigate focused controller
  validation and reuse of accepted prerequisite evidence between dependent PRs,
  preserving changed-test coverage checks. Do not describe the CI run as narrow or
  claim that cached token counts establish causal performance savings.

- Final history classification caught an additional explicit boundary registry:
  new network history source/tests needed entries there as well as suite selection.
  The initial PR push began before that failed preflight was inspected; that was a
  coordination mistake. Gate publication on the actual preflight result and consider
  one owner/suite declaration instead of duplicated path and filter registries.

### MCP delivery contradiction and verification handoff

The user challenged the extra standalone Linear check despite working connected
MCP. The Linear setup skill already recommended MCP, while core configuration and
delivery required an executable argv. Recording that mismatch as open work had not
resolved the ordinary delivery path. This is a package integration failure, not a
connector-authentication limitation.

The correction makes MCP the default when no command observer is configured and
adopts that mode here. The invoking agent preserves a fresh tool response and
supplies a normalized task/source-bound snapshot. Core checks exact issue/project/
PR identities, age and the live GitHub backlink; it retains the snapshot/digest as
caller-supplied evidence without pretending to invoke or authenticate MCP. Existing
command adopters keep their explicit adapter. Regression coverage includes absent
and one-way links, stale/future data, wrong bindings, misleading boolean claims,
unsafe receipt files and incompatible modes. Current instructions, workflow and
troubleshooting now describe the same implemented contract.

The requested next-agent endpoint is environment verification, a critical PR audit,
then authorized merge of PR #39. Preserve the original instruction pin and prior
evidence. A fresh conversation must inspect its actual exposed catalog separately
from the ordinary app-server discovery already observed. UI/player acceptance and
unrun independent behavior/cost or cross-machine trials remain distinct; the handoff
does not turn those observations into passes or authorize a release.

### Continuing evaluation scope

Improve Labyrinth's mechanics and playability while comprehensively exercising
and improving GameSkills and Gamekit through that same development work. The user
explicitly gives the game and package improvements equal importance. Evaluation
was deferred to let PR #38 land; it is part of this work, not another deferred
prerequisite. Plan concrete acceptance for each owner and make evidence-backed
package corrections throughout implementation, with explicit untested coverage.

### Observed builder design failure (2026-09-13)

The user rejected the integrated character editor's wall of weapon/ability buttons:
it offered names without decision-relevant explanations and mixed many tasks into
a long form. Existing Metal captures (`target/review/labyrinth-editor-1280.png`,
`labyrinth-editor-actions-1280.png`, `labyrinth-editor-actions-200.png`) and earlier
layout/input checks remain evidence of rendering/reachability, not usability.
Preserve them as the negative case. S6/S9 usability acceptance is reopened; PR #39
remains draft. Research and numbered discussion subsequently selected the revised
organization below; implementation is authorized, while usability remains unaccepted.

This was an execution failure as well as a guidance gap. Existing creative-level
guidance and this evaluation plan already called for alternatives and confusion
observations. The agent treated unresolved screen organization as routine wiring.
S6 specified editable fields, authority and input paths; the useful descriptions,
rank restrictions and provenance requirements were confined to combat S7. The
`build-ui` sequence starts with views/intents/states, while `verify-ui` distinguishes
structural/rendered/native evidence without an explicit decision-information check.

The first candidate correction was authored in canonical
[plan](../../plugins/gameskills/skills/plan/SKILL.md),
[grill](../../plugins/gameskills/skills/grill/SKILL.md),
[build-ui](../../plugins/gameskills-ui/skills/build-ui/SKILL.md) and
[verify-ui](../../plugins/gameskills-ui/skills/verify-ui/SKILL.md), with package-local
UI contracts/evidence references. Plan and grill investigate the player's decisions,
needed information, consequences and view hierarchy before construction. They
recover recurring views and authority across contexts rather than assuming each
entry point needs another editor. An accepted mechanic is not an accepted screen
design. Material unanswered choices receive bounded alternatives and recommendations;
settled answers and routine fixes do not trigger another interview or approval.
Build and verification carry those decisions into accurate effective descriptions,
comparisons and task-based information checks, separately from input correctness
and observed player comprehension. No universal panel layout is prescribed.

The correcting agent exercised this candidate on a fictional robot-racing brief:
four owned racers, terrain-specific motor moves, a prerequisite firmware upgrade,
captain-owned starting positions and read-only inspection during races. Its response
recommended team preparation plus a recurring racer sheet, explained a guided
alternative, asked one upstream audience/task question, and specified checks for
the actual terrain/duration/compatibility tradeoffs. A settled-answer continuation
retained one sheet without another interview. A routine clipping-fix brief proceeded
without redesign. An unexplained-button artifact failed decision-information review
despite preserving its narrow reachability/rendering passes.

These are bounded self-review responses with known correction intent, not fresh
agent or native-client trials and not a measurement of human comprehension. Raw
briefs, responses and limitations are retained in coordinator scratch
`.context/ui-research/workflow-evidence.md`. The authoring base is `a4e4aa3`;
the committed canonical candidate is identified by the worker handoff/integration
record. Native skill structural validation and the host skill-creator validator
pass for the four changed skills. The host validator initially lacked PyYAML;
an isolated scratch virtual environment supplied it without changing project or
installed skill dependencies. These validators do not execute a model. The repository
check initially rejected four inherited plan status lines; this owned plan now uses
the required active status, and the coordinator owns the other three repairs.
Bundle/archive checks remain outstanding for this instruction revision. Wording
and structural success do not prove improvement.

The coordinator then completed one bounded independent planning comparison using
two fresh agents with no inherited conversation (`fork_turns="none"`):
`ui_plan_trial` used candidate `3e453f8`, and `ui_baseline_trial` used baseline
`a4e4aa3`. Both received the same raw tactical-space-fleet brief and were explicitly
barred from reading Labyrinth plans, evaluation reports or repository history.
Full responses are retained in those session agent records; the comparison summary
is in `.context/ui-research/workflow-evidence.md`. This author records the coordinator's
reported observations rather than claiming to have rerun or independently scored them.

The candidate asked two material questions (fleet versus individual-ship emphasis,
and action-first versus source-first organization), distinguished accepted mechanics
from accepted screens, and proposed a shared campaign inspector with appropriate
authority. Its plan covered effective grants/upgrades, duplicate-source comparison
before committing, task-based acceptance and unknown source facts. The baseline
also produced useful planning: one overview-versus-all-twelve-details question with
a recommendation and sketches, effective move provenance and rule-owned duplicate
handling, restricted campaign inspection, mutation authority, draft lifecycle and
a comprehension walkthrough. The baseline did not reproduce the original failure.

This single pair shows the candidate preserves useful planning behavior on this
brief. It does not demonstrate improvement over the baseline or establish that the
instruction change caused a fix. Neither agent built an interface, exercised native
input or observed player comprehension; question count alone is not a quality or
cost measure. Keep these independent planning observations separate from the earlier
known-answer self-review exercises and from native skill selection/install evidence.
The canonical wording is unchanged following this comparison; no further change is
justified merely to produce a preferred evaluation result.

Further evaluation should include produced interface artifacts, not only plans,
without prescribing a layout or mentioning this failure. A separate reviewer uses
only the result to choose a front-pair attack, distinguish a new move from an
upgrade, explain an incompatible choice, undo it and switch characters without
confusing draft and committed state. Record correct explanations from visible
evidence, missing information and unnecessary backtracking. Static review can
establish information availability; actual player comprehension requires a player
observation. Do not claim the correction works merely because its text exists.

The [research brief](../../../games/labyrinth/docs/plans/builder-ui-research.md)
contains sources, concrete alternatives and settled answers 7–8: squad preparation
with one unified character editor, designed for eventual in-game detailed viewing.
Future character stats belong in that same screen when implemented. Preserve this
cross-context consistency constraint through UI redesigns and evaluation; do not
split character editing into separate testing and gameplay screens. Canonical skill
fallback and the immutable installed pin remain unchanged.

The scoped implementation follows the epic:
[HEX-99](https://linear.app/chillgamerboys/issue/HEX-99/epic-labyrinth-customization-and-configurable-co-op-battles-with),
with [full child scopes and dependencies](../../../docs/plans/labyrinth-customization-epic.md).
Implementation ends at observed, reviewable PRs with evaluation findings
and explicit evidence gaps. GameSkills is tracked in
[HEX-100](https://linear.app/chillgamerboys/issue/HEX-100/evaluate-and-improve-gameskills-throughout-the-labyrinth-customization); Gamekit in
[HEX-107](https://linear.app/chillgamerboys/issue/HEX-107/exercise-and-improve-gamekit-capabilities-through-the-customization).
The user has now selected weapons, composed character movesets and configurable
battles as the design direction; see the discussion scope below. Resolve the
material choices with `gameskills:grill` before implementing. Play observations
should inform this direction rather than reopen selection of an unrelated mechanic.

## Current design discussion (2026-09-13)

Accepted direction:

- Movesets combine innate abilities, learned skills and equipped-item grants.
  Start with weapons whose basic moves can be extended or upgraded by skills.
- Learned skills both grant new moves and upgrade existing moves. Active/passive
  behavior is independent of provenance; sources can include innate, dagger,
  pyromancy, assassin or bulwark. Future classes/trees may organize those sources.
  Passive bonuses and reactions (for example retaliation when hit) are deferred.
- Dagger throws are unlimited for now. Consumption, losing the equipped weapon
  and retrieval are future design questions, not requirements for this slice.
- Establish co-op lobby scaffolding: the host assigns player ranks/places; players
  select their characters, equipment and abilities. The host always controls enemy
  configuration. The host controls unclaimed party characters and may assign zero,
  one or multiple characters to any player. Zero-character players are spectators;
  players also become spectators when their characters die.
- The host may reassign characters during battle. Use the discussed paused-combat
  approach and BG3's multiplayer/session assignment pattern as guidance. Ownership
  changes do not move actors, rebuild characters or grant extra turns.
- One weapon may be equipped initially. Handedness can remain content metadata;
  main/offhand slots, shields and dual wielding are deferred with inventory design.
- All granted active moves remain available in combat; there is no eight-move
  gameplay selection limit. The hotbar is a major design concern, with Baldur's
  Gate 3 a suggested reference to inspect during UI design. Its presentation must
  scale to the available moves without silently dropping actions. Customizable
  hotbars, favorites and player rearrangement are explicitly deferred. Prioritize
  all abilities being visible, inspectable and usable in an automatic layout.
- Enemies use AI by default. Provide a controller boundary for the future RL gym;
  a manual enemy-control UI was not selected. AI and future external policies must
  submit the same validated actions and must not own a second combat rules engine.
- Discuss dagger stab/throw, greatsword cleave across ranks 1–2 plus a single-target
  thrust, and a two-handed axe overhead chop, alongside a small starter roster.
  Exact damage, reach, resource rules and skill upgrades remain design choices.
- Establish item/equipment scaffolding while leaving the eventual inventory's
  storage, acquisition, layout and management mechanics open.
- Fully customize both characters and enemies for the 6v6 battle, with stock test
  options. Configurable encounters must support manual testing and provide a
  reusable deterministic input for the eventual RL gym and stat/balance tuning.
- Items, skills and related content must be easy to add/change through formalized
  code/data scaffolding. Content-definition editing does not need a UI. Distinguish
  authoring the catalog from selecting builds and scenarios in the battle UI.
- Improving GameSkills and Gamekit is equally important to improving Labyrinth.
  The earlier proposed 45-minute supplement cap does not govern this expanded scope.
  Ask further design questions whenever material uncertainty remains.
- Explain the approach, then use `gameskills:grill` to discuss material choices.
  The design discussion produced the scoped epic and accepted implementation plan.

Working defaults after the user's positive response to the proposal: six formation
spaces per side, an in-game setup editor with saved scenario data, and freely
composed builds with equipment prerequisites for weapon techniques. These defaults
were stated back to the user; the earlier individual questions were not explicitly
answered. The user has explicitly settled learned moves/upgrades, unlimited throws
and the co-op lobby ownership model above. Questions 1–6 have been answered. The
main scope is settled; routine implementation choices can be proposed in the
[Labyrinth implementation plan](../../../games/labyrinth/docs/plans/weapons-and-battle-setup.md).
Ask numbered follow-ups starting at 7 only if material uncertainty remains.

BG3 reference check: a [player report on Larian's forum](https://forums.larian.com/ubbthreads.php?Number=883773&ubb=showflat)
describes the multiplayer screen grouping characters under players and moving
assignments between columns. This is a community account, not an observed native
BG3 session or official promise of exact current behavior. Larian's
[Patch 8 notes](https://baldursgate3.game/news/the-final-patch-new-subclasses-photo-mode-and-cross-play_138)
also document a session-manager refresh bug when players connected while it was
open; use dynamic roster refresh as an explicit Labyrinth UI regression case.

Proposed scaffolding (design, not implemented):

- A game-owned validated catalog of stable weapon, ability, learned-skill and actor
  preset IDs. Definitions compose typed effects and parameters; ordinary content
  additions do not require another central enum variant or edits across UI/AI.
  New effect semantics remain explicit tested Rust behavior. Human-editable data
  files are the proposed authoring surface, with format a routine implementation
  choice after reviewing serialization and distribution needs.
- Character/enemy build inputs retain innate grants, learned skills and equipped
  weapon separately. A pure resolver produces ordered moves and their provenance,
  checks prerequisites and resolves upgrades/duplicates without losing source facts.
  Inventory storage, acquisition and management remain open.
- Keep behavior, grants and upgrades distinct: an active ability describes an
  action; a grant records which source provides it; an upgrade alters an identified
  ability when prerequisites apply. Sources have stable identities and display
  names, with no requirement to implement classes or trees now. Preserve all grant
  paths for duplicate abilities and record upgrade contributions separately from
  the original grant. Future passives use the same provenance approach; do not
  accept inert passive/reaction definitions as if they execute today. The eventual
  distinction between passive bonuses and event-triggered reactions needs its own
  bounded rules design when that work is selected.
- A validated battle specification supplies both formations, build selections,
  stat/starting-state overrides, seed and test control policy. Stock scenarios are
  ordinary editable presets. The native setup screen assembles this input; a future
  headless/RL adapter can use the same input and authoritative rules.
- Resolve and freeze the complete content/configuration identity at encounter start.
  Reproducibility must include content and rule versions as well as seed/actions;
  file edits must not silently change a running battle. UI/AI/network consumers
  use the same resolved definitions. Content loading/I/O stays outside pure combat.
- Separate player identity, character identity, controller assignment and formation
  footprint/rank. A player may control multiple characters or none; moving an actor
  never changes ownership. Keep lobby authority and combat-action authorization
  explicit. Host rank assignment and player build edits must validate footprint
  capacity and readiness against the same authoritative party definition.
- Proposed spectator interpretation: a player with no assigned surviving characters
  observes without action authority; permanent death is distinct from a rescueable
  Dying state. Downed characters retain their assignment so rescue can restore
  control without a new ownership grant. A player with another surviving character
  continues to control that character. Spectator connections should not prevent
  combat progress; controller-disconnect policy requires corresponding review.

Source inspection found `PlayerState` and `PlayerView` currently store exactly one
actor per player, snapshots require player count to equal hero count, and any
disconnected player pauses combat. These must be revised with authority, replay,
reconnect and presentation tests; spectator support is not just a lobby label.
Participant capacity must be separate from formation capacity, even if both retain
the same numeric bound initially. Spectator admission does not imply unlimited
connections or authorize changing host-restart guarantees.

The existing source supports hero loadout overrides and explicit rosters, but
stats/footprints derive from archetypes, enemy setup uses presets, and abilities
come from a fixed catalog. Full symmetric customization is new work, not merely
exposing an existing complete editor. The RL integration itself remains future
work; the battle input and authoritative rules should be reusable by it.

## Starting evidence

The candidate is CLI/instructions `0.1.0-dev.3`; its instruction source is
`50d305928febb1617a7cc535db062359639b7f58`. Read the current lock and run `gameskills
status` before choosing the evaluation pin; do not assume later source is identical.
PR #38 records 313 passing Rust tests, structural checks, actual Cargo-archive
consumers and preserved adopter configuration. Codex discovered 13 core and 15
core-plus-Linear skills without model turns. Solo authored docs exercises were
self-review with visible intent, not independent behavioral evidence.

Planning observations on 2026-09-12:

- GitHub reports PR #38 merged at `5419bce91681774fc792568e647e3f09f1e36144`.
  Local HEAD is `ec741abdc712310058f9c949ea5f0067a1917c00`; local `origin/main`
  is the merge commit, one commit ahead with an identical tree. Refresh remote
  state and start implementation from that merge or a current descendant without
  renaming the workspace branch or discarding local work.
- `gameskills` is absent from PATH; `target/ci/gameskills status` succeeds with
  CLI/bundle `0.1.0-dev.3`, source `50d305928febb1617a7cc535db062359639b7f58`,
  content SHA-256 `b069a3108dcb094766b89b3a35ad451f3088608a0e98e02bfdedbebed51c1f99`.
  The lock selects core, Linear, maintainer, multiplayer, turn-based and UI packages.
- This host exposes no GameSkills native skill entries/resolver. Planning uses
  canonical `gameskills/plugins/<package>/skills/<skill>/SKILL.md` as explicit
  source fallback, including `plan` and `evaluate-skills`. Canonical plugin source
  has no diff from the installed source commit. This is not native activation.
  Configured Codex/Claude clients are not observed client versions or behavior.
- `docs resolve` returns root plus Labyrinth/GameSkills indexes as appropriate.
  Relevant current architecture, rules, disclosure, development and testing docs
  were read. No game was launched and no behavioral trial was run in planning.

## Session sequence

1. **Establish identity and carry the endpoint.** Recheck status, lock, source/base
   and available host skills. Record executable identity, bundle hash, canonical
   fallback revision if used, actual client/version/model/effort where exposed,
   platform and relevant configuration. Read [contributing](../contributing.md)
   and resolve `gameskills:plan` and `gameskills-maintainer:evaluate-skills`.
   Resume a delivery task for this combined work, or start its coordinator record with endpoint
   `pr`, base `main` and the agreed checks. Do not reuse completed HEX-98/PR #38.
   Required tracking uses the adopted Linear workflow and connected MCP; record an
   observer mismatch as a delivery limitation rather than requesting another API
   key or expanding this task into an MCP rewrite.
2. **Resolve the selected design and observe the baseline.** Continue `grill` for
   weapons, composed movesets and battle customization. Use `gameskills:playtest` with the current
   [local battle](../../../games/labyrinth/README.md), initially seed 42 and a
   second reproducible seed such as 91. Walk ability → target → Confirm, an illegal
   action, positioning and the relevant status/death boundaries. Record concrete
   moments of confusion or weak choices, plus the user's feedback. If native input
   is unavailable, report it and use available observations/user play; captures
   cannot substitute for interaction. Compare design alternatives within the
   selected customization scope and agree the before/after rules and player benefit.
3. **Write the acceptance contract and implement.** Record a reproducible starting
   state/action, expected outcome, rejection/edge cases, preserved invariants and
   a before/after play route. Use `gameskills-turn-based:model-rules`; load UI or
   multiplayer specialists only where the change touches their boundaries. Inspect
   rules and all producers/consumers before editing: AI, session authority,
   forecasts, presentation and snapshots. Keep rules/content in Labyrinth; change
   Gamekit only for demonstrated reuse. Update `RULES_VERSION` for semantic changes
   and review content fingerprint/wire compatibility as applicable. Do not redesign
   the framework to prepare for an unobserved problem.
4. **Verify and compare play.** Use `gameskills:test`, focused debugging when needed,
   and the [game's verification guide](../../../games/labyrinth/docs/testing.md).
   Run configured `labyrinth-test` (which includes `rules-test`), `labyrinth-lint`,
   `rust-format` and `docs-check` for the resulting game change. Add focused owner
   regressions for the accepted mechanic, including deterministic rejection and
   preview/commit behavior where affected. Select UI renders/native interaction and
   session/network checks based on the actual change; shared capability or protocol
   edits require their additional owner checks. Revisit the same player decision
   before/after under recorded seeds, inputs and settings. Do not require identical
   RNG trajectories after intentionally changed rules. Keep correctness, visual
   inspection, native interaction and human feedback as separate observations.
5. **Settle bounded evaluation findings.** Maintain the evidence ledger below
   throughout the work, then run the supplemental cases that the real task did not
   cover. Make only observed, narrow skill corrections in canonical source, with
   relevant current-doc/pointer updates. Preserve the original failure and candidate
   identity, record the corrected identity, and rerun affected cases. Apply the
   maintainer's source/bundle/package checks if changing the distributed candidate;
   do not edit installed bundles or silently repin adopters. End with a supported
   recommendation and explicit gaps, even if native evaluation remains unavailable.
6. **Deliver all owner outcomes.** Reconcile Labyrinth rules/architecture/testing docs,
   Gamekit capability/Rustdoc guidance and GameSkills contributing/troubleshooting claims. Use `create-pr` and `audit-pr`
   for the final scope, checks, play observations, framework findings and limitations.
   Observe and bind the actual PR plus adopted tracker, then run `delivery check`
   against current source. An unavailable provider/observer remains unverifiable;
   local tests do not establish publication or complete tracking. PR creation does
   not authorize merge. Retire completed plans only after preserving outstanding
   work and repairing links; PR #38's docs-refactor plan is a cleanup candidate.

## Bounded behavioral coverage

Keep real game-development evidence in the working checkout; isolate synthetic
adopter inputs and outputs under `.context/skill-evaluation/`. Evaluation notes may
start there, but publish durable conclusions/evidence in the PR and current owner
guidance before retiring this plan. Keep raw tasks separate from assessor criteria;
do not give expected answers to an independent evaluator.

| Case | Use real work or a bounded supplement | Assess actual behavior |
|---|---|---|
| Combined/resumed task | Carry this brief through mechanic selection and PR delivery; resume an actual interruption if one occurs | Both outcomes survive; exact PR/tracker and remaining work are observed. Without an interruption, resume remains untested. |
| Native skills unavailable | Current source-fallback path, if still needed next session | The agent reports the limit and uses canonical current skills without claiming activation. |
| README-only adopter | One small isolated documentation task, with no configured docs tree | It finds the README and finishes without inventing setup, folders or Gamekit requirements. |
| Mixed owners/custom docs | One isolated fixture with two owners and explicit custom indexes | It reads the correct owner guidance and preserves local layout/overlays; real mixed-owner work alone does not test custom indexes. |
| Stale-doc contradiction | One docs/code disagreement in that fixture | It investigates authority and repairs the owning fact and pointers rather than blindly copying stale guidance. |
| Completed plan/moved heading | Cleanup fixture containing completed and unfinished work and an inbound anchor link | It preserves the unfinished requirement and repairs the moved anchor before retiring the completed material. |
| Focused negative trigger | A direct review/debug request in the fixture | It performs the requested specialist work without a new implementation plan, ticket or parallel wave. |

Begin with the real customization task and three fixture exercises (README-only;
mixed/custom docs with contradiction; cleanup with a focused request). Expand
evaluation around observed gaps and the agreed package acceptance, without treating
it as leftover time after game development. Do not add unrelated gameplay merely
to cover a trial. These disclosed cases are not a held-out
benchmark for this planning agent. Solo exercises are useful self-review, not
independent forward-agent evidence. This plan launches no workers; independent
agents require applicable authorization for that later session. Compare a prior
candidate or no-skill baseline only on an equivalent bounded task if practical;
otherwise mark the comparison untested and make no quality/cost improvement claim.

For each case, record input/raw task, candidate and client identity, skills/docs
actually selected, useful guidance, missed instructions, unnecessary reads or
workflow steps, user corrections, output/evidence references and endpoint status.
Record elapsed time, retries, review rounds and usage where exposed; missing
telemetry is unavailable, not zero. Use pass/fail/partial/unavailable/not-run with
concrete observations, not a score inferred from structural validation.

### Observed planning feedback

The user challenged the description of this epic as requiring solo work. The
planner over-weighted the handoff's delegation caveat and under-weighted the
owner-approved parallel configuration in `gameskills.toml`. An execution permission
check was misleadingly presented as an epic scope constraint. Corrected the epic
to allow either execution mode using applicable existing authorization and runtime
capacity. This is an observed interpretation/communication failure; do not encode
a new mandatory approval step in the skill as its remedy.

On 2026-09-13 the user requested numbered grill questions so answers can refer to
question numbers. Earlier rounds used unnumbered question titles and bullets,
making multi-question replies harder to reference. Record this as a potential
`gameskills:grill` improvement: give each question a visible number, preserve that
number across tool prompts and prose, and map responses back to the correct open
decision. Apply it immediately in this conversation. At the time of the correction, the canonical skill
recommended one to three questions per round without requiring numbering.

Evidence type: actual user correction during canonical-source-fallback planning;
not independent native-client evidence. Canonical commit
`10d9e38b00752063c8e4c3a9be37933b72a230d4` adds visible, stable question numbers
across rounds and tools. Subsequent user replies numbered 1–3 and 4–6 matched the
corresponding decisions without an observed mapping correction. This supports the
local usability correction, not comparative native-client performance. Installed
pins remain unchanged; distribution alignment and additional findings follow.

### Wave 1 evaluation (2026-09-13, HEX-100)

Recommendation: continue the scoped rollout and evaluation. The observed user
corrections justify the small planning changes below. Self-review exercises and
structural results do not justify promoting the candidate as release-ready or
claiming better model performance. HEX-100 remains active through the epic; this
wave does not establish completed delivery, gameplay acceptance or native parity.

Identities and environment:

- Canonical starting source: `10d9e38b00752063c8e4c3a9be37933b72a230d4`.
  Runtime: locally built CLI `0.1.0-dev.3`; `gameskills` absent from PATH.
- Installed instruction source: `50d305928febb1617a7cc535db062359639b7f58`;
  content SHA-256 `b069a3108dcb094766b89b3a35ad451f3088608a0e98e02bfdedbebed51c1f99`.
  Read-only status succeeds with Codex/Claude configured and six selected packages.
  Configured clients are not observed native client versions.
- Host: Codex session in Conductor on macOS. Exact client build, selected model/effort,
  token usage, cache usage and monetary cost are unavailable in this trial.
  No GameSkills native resolver is exposed. Canonical files were read explicitly;
  native activation, implicit routing and Claude execution are untested.
- Worker inherited the discussion and assessor criteria. Cases were authored and
  executed by that worker, not independent forward agents or held-out tasks.
  No old-candidate/no-skill behavioral control was run; no comparative claim follows.
- Raw task, immutable input copies, edited output copies, resolver JSON, review and
  content hashes are isolated under `.context/skill-evaluation/wave1-skills/` in the
  coordinating workspace. Those scratch artifacts are not required for the durable
  observations below and are not part of an adopter or installed bundle.

Actual context selection: core `plan`, `grill`, `review` and project context;
maintainer `author-skill`, `evaluate-skills` and their evaluation/context references;
host `skill-creator`; resolved root and GameSkills indexes, architecture, development,
testing and candidate-verification guidance. Fixtures used only relevant owner
indexes/guides after `docs resolve`. An attempted `review-pr` source lookup failed
because the actual focused skill is `review`; the latter was read and used.
`debug` and `audit-pr` were also inspected during routing, but were unnecessary
for these fixture tasks and were not executed. No authoring change is justified
by that lookup mistake or those extra reads. `rg` was unavailable; local
file inspection used the available shell/Python tools.

| Case and raw task | Observed output | Evidence and limits |
|---|---|---|
| README-only: “Clarify how to run the tiny dice game locally. Keep the existing README-only layout. Finish the documentation edit locally; do not publish.” | Resolver selected only `README.md` without configuration/setup. Read the Development paragraph; replaced vague “Run with Cargo” with its existing `cargo run -- --seed 7` command and project-root location. Only README changed; no docs tree, ticket, queue or Gamekit dependency added. | Pass within self-review scope. Fixture has no runnable Cargo package; the command was restated from owner guidance, not launch-verified. |
| Custom mixed owners: “Review and correct the documented action ownership across game and shared library. Preserve our custom docs layout and local overlay. The accepted design keeps damage and targeting in the game; shared UI displays supplied values. Finish the docs correction locally.” | Resolver returned `handbook/index.md`, `handbook/game/index.md`, and `handbook/ui/index.md` for game/UI paths. Read both linked contracts and `game/combat.rs`/`ui/view.rs`. The game guide incorrectly assigned damage/legality to UI, contradicting source and accepted intent. Corrected only the game rule guide, linked the existing UI contract and retained the local overlay byte-for-byte. | Pass within self-review scope. No code execution; observed contradiction was in synthetic source/docs. Existing mappings and local layout remained intact. |
| Focused cleanup review: “Review only this proposed documentation cleanup. We moved the connection guide from the completed plan into architecture and deleted the plan. Identify actionable issues; do not create an implementation plan, ticket, or agent wave.” | Direct `review` produced two medium findings: README still pointed to deleted `docs/plans/connection.md#reconnect`; deletion also lost outstanding cross-machine reconnect/spectator acceptance owned by multiplayer. Review made no implementation edits, plan, ticket or wave. | Pass within self-review scope for negative planner trigger and semantic cleanup review. Skill choice was explicit, not observed automatic discovery. |
| Cleanup follow-up: “Apply the two documentation review findings locally. Preserve the unfinished validation requirement and its owner; do not claim it passed.” | Repaired README to `docs/architecture.md#resume-a-connection`; retained the unfinished requirement and multiplayer owner in an Outstanding verification section. Only README and architecture changed. | Pass within self-review scope. Direct artifact checks confirm target heading and requirement; network behavior remains untested. |

The three exercises completed without retries; cleanup included one intentionally
separate review and correction phase. The recorded fixture wall-clock interval is 42 seconds (raw
identity/results JSON); it excludes prior setup/context reading and later report/
verification work and is not coordinator-plus-worker task cost. Two corrective
user interventions are evidenced in the surrounding planning conversation (numbering
and solo-only framing); the fixture exercises had none. Missing total cost/timing
telemetry is unavailable, not zero.

Preserved negative results and narrow corrections:

1. **User question numbering:** the baseline already contains the canonical grill
   correction. Its wording preserves one-to-three-question guidance without adding
   more questions or an approval gate. Keep it; no further grill change is needed
   on this evidence. The conversation supports reply mapping only.
2. **Authorization mistaken for epic scope:** the planner framed the epic as solo
   despite recorded owner-approved parallel work. The current user also explicitly
   requested parallel agents. Refine `plan` to recover inherited authorization and
   separate strategy from scope; rename its prose “solo record” to “delivery record”
   without changing the runtime API. Keep the capacity-versus-permission boundary.
   This is an observed instruction-interpretation failure with a narrow correction,
   not evidence for a mandatory fresh-approval gate. Forward efficacy remains untested.
3. **Canonical/distribution drift:** at starting HEAD, `repo-devtools check` and
   `skills validate` passed, but `bundle check` failed with “recorded source commit
   inputs differ from HEAD; prepare from the new committed inputs.” Preserve this
   failure: structural skill validation did not prove the distributable contained
   numbered grill. Prepare the bundle after committing canonical inputs, then
   check its content/provenance and actual Cargo archive. Never rewrite the installed
   pin, old bundle directories, overlays or legacy compatibility fixtures.

Verification after the canonical correction:

- Prepared source `c17cab770bfe92304eddd68324f9cb725f7607e5`; generated bundle
  committed at `66bc86f`. Content SHA-256:
  `32a72d8bb7e30e6b013ad30d5b431e5251eab21d1baae4539cf2fefef3f9abeb`.
  Archive SHA-256:
  `b9d0a7492760658fc3e640b0e53fc9f97c998f5e3a0a40914876a4cca4f3145d`.
  `bundle check` verifies both current inputs and the recorded source commit.
- `repo-devtools check`, `skills validate` (24 skills, seven packages) and
  `skills legacy` pass. They establish structure and fixture compatibility only.
- `cargo test --locked -p gameskills-cli --profile ci`: 140 tests pass after
  rebuilding the corrected embedded bundle. This includes process-based consumer,
  installation, recovery and fake native-peer regressions; fake peers do not
  establish actual Codex/Claude behavior. An earlier suite also passed but predates
  the final bundle and is not the corrected-candidate evidence.
- `cargo clippy --locked -p gameskills-cli --all-targets --profile ci -- -D warnings`
  passes. Tests and lint use the worker checkout's own `target/`.

- `cargo package --locked -p gameskills-cli` packages 46 files and successfully
  builds the extracted crate. `bundle verify-package` separately confirms its normal
  lockfile and exact bundle bytes. The inspector itself reports
  `cargo_build_verified = false`; the preceding Cargo build log supplies that
  separate observation. Crate SHA-256:
  `965f0236de4ece70d62941c141d2b96edde71ff7775bc2e5d49b429e5cb2df64`.
- The archive-built `target/debug/gameskills` was supplied through
  `GAMESKILLS_CANDIDATE_BINARY` to the existing adoption test; it passes (one test).
  That trial copies the binary into an external temporary consumer with build tools
  absent from PATH and verifies core-only setup, selected-package changes, rollback,
  recovery and preserved owner files. This is actual consumer/runtime evidence,
  not native model behavior or a registry release.

The host skill-creator quick validator was attempted for `plan` and `grill`, but
both runs failed before validation because Python `yaml` (PyYAML) is unavailable.
No dependency was installed. Repository Rust structural validation remains the
available check; do not report the host Python validator as passed.

The fixture checks found no further missing portable instruction. Avoid adding
case-specific wording to already adequate README discovery, contradiction, cleanup
or focused-review guidance. Remaining acceptance includes independent raw-task
forward trials, native discovery/invocation, two-pin coexistence, install/update/
recovery/removal in claimed clients, cross-machine play, broader creative-level
comparisons, and total cost telemetry. Real parallel integration and final PR
observations belong to the coordinator's ongoing delivery record.

### Parallel evidence ref inputs (HEX-100 follow-up)

Real parallel work exposed an additional limitation: catalog worker run
`3fb6eab9e5c036187e74b33350a990b4` completed its commands, but the coordinator
reported failed reuse because other workers advanced shared Git refs. Investigation
started at `73a58454f820a936019bafe6e3a188b3d0049cf3`. Existing runner tests
explicitly require all-ref invalidation, so removing it globally would break a
deliberate contract. The coordinator accepted additive per-command declaration.

A separate local adopter reproduced the issue with the previous executable:
`/usr/bin/true` passed, adding another branch left HEAD/source and record bytes
unchanged, and validation reported only `identity/repository/refs_digest` changed.
The original reproducer record is preserved. The corrected executable created a
fresh record which stayed valid when the unrelated branch advanced; editing the
README then invalidated `identity/source_digest`. Revalidating the old record with
the new executable still fails and leaves its bytes untouched. Raw commands,
identities and outputs are in coordinator scratch `.context/evidence-ref-inputs/`
(`baseline-repro.json` and `candidate-repro.json`). These are real CLI/process
observations, not native model selection or an independent forward-agent trial.

The correction adds `git_refs = "all"` (unchanged default) or an exact list of
full `refs/...` names per command. Lists are validated without Git/setup; missing
refs fail before command execution. Graphs use the union of selected commands and
prerequisites, and any all-ref dependency keeps the graph conservative. HEAD, its
symbolic identity, worktree/index, config, executable/runtime and environment
remain mandatory. Delivery observations and nested submodules remain all-ref.
Ordinary test graphs can declare their known review base; commands that enumerate
branches/tags or use unknown Git inputs should retain the default. No argv inference
or default weakening is introduced.

Focused configuration and runner coverage passed (44 tests), including actual
linked-worktree commits, declared-base changes/deletion, dependency policy union,
symbolic ref retargeting, same-OID branch changes, packed refs and mid-run input
changes. Existing source/index/config/lock/environment/executable protections are
also exercised in scoped mode. An initial focused invocation failed all runner
fixtures before behavior because this fresh target lacked `runner_probe`; that
negative log is retained, followed by the explicit probe build and successful run.

This is a demonstrated workflow correction selected by the real epic, not a
synthetic performance claim. Prior evidence must be preserved and fresh runs
collected after upgrading the runtime or changing configured ref inputs. Earlier
development binaries may ignore the additive field and retain all-ref behavior;
matching `0.1.0-dev.3` version text alone does not identify the corrected binary.

Final source and verification for this follow-up:

- Runtime, tests and canonical reference source:
  `1d7452fe2acc446077824bafef2d6211bcda00e9`; generated bundle committed at
  `2743df9`. Bundle content SHA-256:
  `a3248a8e6ff90cba5955f7d913f628ab556086a3def665faae37d70a71e18d7f`.
- All 145 CLI tests pass after preparing/rebuilding that bundle; final all-target
  CLI clippy, repository/docs checks, native skill structure, legacy structure
  and bundle provenance checks pass. Logs are in the same isolated scratch folder.
- Cargo successfully builds the extracted `gameskills-cli` archive; separate
  archive inspection verifies the normal lockfile and exact bundle bytes. Crate
  SHA-256: `3d693185e1f3671bef158a9e564f5037b39edec60e9aa7d666918c6392bc8888`.
  The existing external consumer test passes using that archive-built executable
  (one test; build tools removed from the consumer PATH).
- `archive-ref-repro.json` records a direct trial of the archive-built CLI with
  `git_refs = ["refs/heads/review-base"]`: fresh run
  `4914a4ca00ceec7432a86c57cc5b2d25` remains valid after an unrelated branch advances,
  then becomes invalid when its declared review base advances. Record bytes remain
  unchanged. Earlier `candidate-repro.json` identifies the intermediate binary
  before the updated instruction bundle was embedded; it is not the final package
  identity. Neither trial retroactively validates the catalog worker's old run.

The adopter config remains coordinator-owned: every command/prerequisite in a
scoped test graph must declare its known refs, and a rebuilt runtime must collect
fresh evidence. Installed bundles, pins, frozen fixtures, earlier skill-worktree
commits and historical run records were not changed by this follow-up. Independent
forward skill performance, actual native clients and total cost comparison remain
unobserved; those gaps do not negate the measured local CLI correction.

## Gamekit capability evaluation

Resolve Gamekit owner docs and use `gameskills-maintainer:evolve-gamekit` for
demonstrated capability changes. Assess shared UI form composition, focus,
validation feedback and selection/scrolling while building the setup screen;
deterministic testing helpers while exercising builds/scenarios; and transport,
session integration and compatibility if co-op setup is selected. Record what
worked, awkward APIs, missing contracts, defects and concrete consumer evidence.

Make justified reusable fixes with capability-level tests and affected adopter
checks. Confirm migrations retain other games' behavior; inspect actual callers
and supported feature combinations. Do not move Labyrinth weapons, inventory
semantics, balance or encounter policy into shared crates to manufacture package
changes. Useful package outcomes include observed fixes, clearer contracts and
validated reuse, with explicit findings for every exercised capability. Select
additional isolated consumer/package trials where source-only integration cannot
support the intended claim. GameSkills must also be assessed on whether it guides
these ownership, testing and migration choices effectively.

## Integrated adopter findings (2026-09-13)

Parallel implementation used isolated worktrees and revision-guarded work orders;
ancestry-preserving integration retained worker sources and historical evidence.
The coordinator preserved the implementation PR endpoint through the ownership,
content, editor and verification work. Final publication remains a separate
observed requirement, not a consequence of successful worker reports.

Gamekit's current native controls, scroll/focus helpers, editable fields and
contextual cards support the game-owned setup and complete ability list without
an inventory/combat abstraction. The shared stale-focus correction is exercised
by both capability regressions and the rebuilt Labyrinth controls. Game-specific
formation, build validation, actor ownership and provenance remain in Labyrinth.
This is concrete reuse and a bounded shared fix, not an extraction count target.

Independent setup review then exposed an integration defect: Bevy editable layout
can mark `EditableText` changed without changing its string, so repeated
`UiTextChanged` messages cancelled Labyrinth's pending save acknowledgment.
The adopter now compares strings before invalidating a submitted draft. The
no-op save regression is retained; ordinary saves and cursor-only updates share
that path. Reload also ignores events from an unmounted replacement draft, preserving the
new authoritative text. Seed fields track loaded configuration while retaining
unapplied text through participant refreshes; Escape closes the editor without
opening background menus. Render review replaced an unsupported selected-grant
checkmark with a visible ASCII marker. No Gamekit event-semantics change is claimed;
deduplicating semantic text events centrally remains a possible separately reviewed
improvement.

Larger frozen-content snapshots exposed a test assumption that admission ACK and
the initial snapshot arrived together. The reconnect flood test now explicitly
waits for the authoritative baseline before checking the preserved watermark.
Another new integration test initially selected a dagger throw from an illegal
front rank; it now asserts a legal front-rank weapon action before submitting.
These were test corrections, not failures of replay protection or weapon legality.
Source review also closed a retained preset-adapter bypass: guest preset changes
now preserve host-assigned footprint and reject obsolete assignment revisions.

The local macOS test linker warns that `__eh_frame` exceeds its compact-unwind
encoding range; a failed assertion can subsequently abort in libunwind. Captured
`--nocapture` output preserves the real assertions before that abort. No global
compiler workaround or successful failed-run claim is introduced. Desktop
inspection is unavailable because the CUA tool reports
`CUA_REPL_ENABLED_SURFACES is required`; production-plugin input and rendered
fixtures do not establish an interactive desktop or cross-machine playthrough.

Recommendation remains a scoped rollout with continued evaluation. The observed
planning and evidence-runtime defects justify their corrections, and the game
provides real consumer evidence for the focus fix and existing primitives.
Independent skill routing/forward efficacy and comparative cost remain untested.
Installed pins, adopter overlays and historical runs remain unchanged.

## Completion and retained gaps

The session is complete when the agreed mechanic has appropriate owner checks and
recorded playability evidence (or explicit missing observations), a reviewable PR
is observed, adopted delivery requirements are checked, and framework findings,
bounded fixes and untested cases are reported. Missing required acceptance remains
pending; documenting a gap does not turn it into a pass. Finish the evaluation
report even if the recommendation is continued evaluation or limited use only.

This session alone does not establish authenticated Claude behavior, Windows
process supervision, native install/update/recovery/removal, two-pin coexistence,
independent held-out performance, creative-level comparisons or broad cost savings.
Keep those wider trials in the existing evaluation/framework follow-ups as untested
unless separately exercised. Existing cross-machine LAN/Tailscale and interactive
verification gaps remain explicit. Retain PR #38's admission/refusal timeout
observations: affected tests passed locally and on CI retry without code changes.

MCP observer alignment, live Linear deletion/pilot, backup export, registry release
and linked-workspace migration are separate. This plan does not select maze,
campaign, inventory or other future systems as the mechanic to build.

## Publication packaging finding (2026-09-13)

The UI revision's configured local graph passed at `bf87c33`, including tooling
and skill tests, while CI correctly rejected the CLI's embedded instruction bundle
as stale against the changed canonical source. The coordinator omitted bundle
preparation/package verification from that local graph. This was an execution
coverage gap: structural skill validation and tooling tests do not establish that
the distributable contains the current authored instructions. Existing devtools
packaging guidance already requires committed inputs followed by `bundle prepare`,
`bundle check`, Cargo packaging and `bundle verify-package`; weakening that check
or changing the adopter's installed pin would conceal the failure.

Regenerate only the source-owned CLI bundle from committed canonical inputs and
verify the actual Cargo artifact, then refresh publication/source evidence. Keep
the original failed CI and earlier local passes under their actual source identities.
This correction supplies packaging evidence, not native activation or proof of
better planning behavior. No additional skill rule is inferred from this one
missed step; apply the existing delivery/package instructions and retain the
finding when assessing end-to-end execution.

## Second hands-on rejection and planning refinement

After trying the published UI, the user again found preparation and player
management clunky. Addition offered no up-front character-type selection, and
box-shaped roster cards failed to exploit the six-rank battle presentation.
Earlier rendered reviews found information/overflow defects but did not challenge
the underlying interaction strongly enough. Preserve the prior narrow passes and
this new negative result together; added descriptions and passing tests did not
establish a good UI.

The next plan/grill refinement must treat visual/spatial interaction as one of
several core planning considerations, retaining information hierarchy. It must
investigate user goals and task frequency; the appropriate visual/spatial model;
information priority and comparison; view/context reuse; action/state/feedback
and recovery; authority; and supported input/accessibility/responsive behavior.
Do not prescribe spatial manipulation for every UI or replace explanations with
icons. Investigate consequential ambiguities with concrete alternatives and
recommendations; carry settled answers forward and own routine craft decisions.

Require concrete design specifications proportional to the change: representative
content and art, layout/proportions, emphasis/typography/density, primary/secondary
actions, selected/empty/invalid/read-only states, transitions and narrow/large-text
behavior. An annotated visual design plus task walkthrough should make the planned
result reviewable before broad production wiring. This is design work, not a new
mandatory user approval or repetitive questionnaire. The criterion is whether an
implementer can understand the intended screen and behavior without inventing its
central decisions from a list of controls.

Strengthen visual-walk judgment as well as coverage: actively challenge composition,
clarity, consistency, visual quality, discoverability, unnecessary steps, repeated
editing, attention competition, context loss and correction costs. Walk complete
creation/replacement/positioning/assignment/customization/deployment tasks through
actual states. Cite the problematic state, consequence and proposed correction;
retain material unresolved design findings instead of accepting mere absence of
clipping. Separate agent judgment, observed user feedback and mechanical test
evidence. Evaluate produced designs and interfaces, not just articulate plans.

The second canonical revision now carries these requirements in plan/grill and
the core package-local [interface design reference](../../plugins/gameskills/references/interface-design.md),
plus build-ui/verify-ui and their UI contracts/evidence references. Entry skills
route to focused design guidance instead of adding a mandatory questionnaire.
Keep Gamekit composite quality tied to the same task and visual standards. The
[Labyrinth design](../../../games/labyrinth/docs/plans/builder-ui-research.md) records
answers 9–10 and the required next artifact; future grill numbering starts at 11.

The author performed a bounded critique of three preserved real UI captures from
the rejected revision, identified in `.context/spatial-ui/workflow-evidence.md`.
The preparation frame fails formation comprehension at a glance: large repeated
cards put later ranks below the fold and translate a single rank sequence into
two rows. That finding remains consequential even if scrolling reaches everything.
The equipment frame does provide a positive information case: visible Greatsword
Cleave details explain hitting both front ranks and the removed/added moves; the
browser/detail organization need not become spatial. Its draft-versus-preview
wording remains ambiguous. The large-text learned frame clearly explains the
5→7 upgrade while devoting much of its height to navigation; that is a composition
tradeoff to inspect through the complete task, not a native-input failure inferred
from a still. Settled-answer and routine-copy-fix continuations retain the selected
design without another interview. These known-context author exercises neither
compare independent agents nor establish efficacy, human comprehension or cost gain.
New interface artifacts and complete native tasks still require critical review.

Existing local `repo-devtools check`, `skills validate` (24 skills, seven packages)
and the host skill-creator quick validator for all four changed entry skills pass.
No Cargo/GPU work was run by this author; the coordinator owns generated bundle,
archive and integrated publication verification after canonical source is committed.
The candidate is canonical source fallback in this session and has not been installed.

Native availability now has a narrower observed boundary. The coordinator's frozen
CLI `native codex --verify` succeeded for the original installed content pin
`b069a3108dcb094766b89b3a35ad451f3088608a0e98e02bfdedbebed51c1f99`, discovering 23 skills
through generated marketplace/plugin launch flags; the record is
`.context/spatial-ui/native-verification.json`. This establishes that pin's native
installation/discovery, not invocation or behavior of this new candidate. The
current Conductor session still exposes no GameSkills catalog entries, and the
coordinator observed no persistent GameSkills entries in the user configuration.
That host registration/session exposure gap is separate from a broken bundle or
global native unavailability. The user has requested its correction; root owns the
separate runtime/setup work. Preserve the installed pin and these distinct outcomes.

### Installation readiness was a missed serious gap

The user explicitly challenged why the missing host catalog had not been treated
as a package improvement. The coordinator had repeatedly reported canonical fallback
without investigating the adoption boundary. That was an evaluation/execution
failure: a valid staged bundle and a successful specially configured launch did
not establish a usable ordinary installation. The old status caveat was truthful
but insufficiently actionable. Preserve the user's correction as negative evidence.

The runtime correction adds project Codex registration to setup, a pin-preserving
`native codex --register --apply` repair, owned-setting/container preservation,
recoverable transactions and explicit per-client registration readiness. It does
not overwrite local disablement, user comments, unrelated settings or host trust.
Claude-only setup without managed Codex state leaves unrelated Codex files alone.
Independent review found and corrected both that cross-client coupling and pruning
of pre-existing empty tables/comments. Ignored registration/configuration files
also participate in verification identity; changing them can no longer retain a
passing run as current. Setup/project-context guidance now treats selected skills
missing from the host as an actionable readiness gap rather than normalizing it.

The original root pin was registered using a frozen candidate executable with
SHA-256 `ebfd082cefcd51f04e397b461d0a412fe39f0d38cb8dea4b94f4606b1c8e347b`.
`native codex --verify-project` then succeeded with exact argv
`codex app-server --stdio`, discovering all 23 selected skills for content
`b069a3108dcb094766b89b3a35ad451f3088608a0e98e02bfdedbebed51c1f99`.
Client identity was `codex_sdk_ts/0.153.4` on macOS 26.6.2/arm64. This verifier did
not supply marketplace/plugin enable overrides. Hash comparison established that
`gameskills.toml`, `gameskills.lock.json` and the user's global Codex configuration
were unchanged. No trust setting was needed or changed. Evidence is retained in
`.context/spatial-ui/native-registration-applied.json`,
`native-project-verification.json` and `native-before-hashes.json`.

This executable precedes final checked-access/lint cleanup; final committed-source
verification is recorded separately rather than rewriting this observation.
The current Conductor conversation still exposes its original skill catalog;
fresh-process discovery does not demonstrate hot reload or native invocation in
this conversation. The revised instruction candidate remains canonical source
fallback here, with its generated distribution checked separately. Claude's
persistent registration and Windows native verification are explicitly unsupported;
neither is silently labeled ready. Discovery is not behavioral or comparative
efficacy evidence. The registration problem is corrected without changing the
evaluation baseline or claiming the broader independent trials are complete.

Committed runtime `2ef59fee` subsequently passed all-target CLI Clippy and the
11 final registration regressions, with earlier focused installation/native/recovery
and ignored-setting staleness tests retained separately. Its frozen executable
SHA-256 is `30879fecb0a44e919d4271a186f1c73ba71a06811226790aec8d35cf6d49ea3d`.
The coordinator repeated registration (a no-op) and ordinary discovery using that
executable: 23 skills, unchanged original pin, and the actual project configuration
layer reported `loaded`. Final records are
`.context/spatial-ui/native-registration-final.json` and `native-project-final.json`.
Independent follow-up source review confirmed both preservation corrections.

### Spatial artifact and integration critique

The implemented constructor uses actual facing character art, stable sparse ranks,
type preview before placement, place ownership and the one character editor.
Coordinator review of actual frames rejected the first picker for burying moves,
redundant local ownership controls and an oversized preview highlight. Subsequent
review caught current-rank limitations and collision reasons below the fold, plus
insufficient disabled-state emphasis. The final 1280 Auto/200% frames put these
facts before secondary details and preserve selected formation context. A wide
frame exposes more moves in the same detail route. Independent source review also
caught hidden local readiness, stale local placement candidates, unreachable
overlapping multi-rank movement and reconnect eligibility; corrections have
production-input/policy/socket regressions. These findings demonstrate concrete
review interventions, not proof that wording changes caused better design or that
the user accepts this third iteration. The two preceding user rejections remain
negative evidence, and actual desktop/human acceptance is still outstanding.
