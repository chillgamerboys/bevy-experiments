# Labyrinth mechanics session and GameSkills evaluation

Status: active implementation and bounded self-review exercises; independent native behavioral trials remain unobserved.
Owners: Labyrinth for mechanics/playability; GameSkills for workflow quality; Gamekit for reusable capability quality.
Historical tracking: [HEX-98](https://linear.app/chillgamerboys/issue/HEX-98/strengthen-gameskills-delivery-and-add-optional-linear-workflows), merged PR #38. These are not the new implementation's delivery identities.

## Scope and priority

Improve Labyrinth's mechanics and playability while comprehensively exercising
and improving GameSkills and Gamekit through that same development work. The user
explicitly gives the game and package improvements equal importance. Evaluation
was deferred to let PR #38 land; it is part of this work, not another deferred
prerequisite. Plan concrete acceptance for each owner and make evidence-backed
package corrections throughout implementation, with explicit untested coverage.

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
separate review and correction phase. Their wall-clock interval is recorded in raw
identity/results JSON; it is not coordinator-plus-worker task cost. Two corrective
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
