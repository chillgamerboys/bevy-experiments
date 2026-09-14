# Labyrinth customization and package-quality epic

Status: active acceptance revision; hands-on feedback rejected the card-based preparation/player menus. Spatial construction and integrated ownership are implemented, with one unified editor; final integrated delivery and human acceptance remain outstanding.
Keep this plan active through remaining human/CI acceptance.
Owners: Labyrinth, Gamekit and GameSkills, with equal-priority acceptance.
Current endpoint: the user authorized the fresh verification session to audit the
environment and PR #39, resolve blockers, and merge after acceptance and required
checks pass. Verification is underway; integration has not been observed.
Release remains outside scope. This supersedes older endpoint notes below.

## Integrated outcome

The content/scenario, combat, ownership, setup and complete-ability UI streams are
integrated together because their frozen model and wire changes share one boundary.
Gamekit carries the demonstrated focus fix; GameSkills carries numbered grill,
authorization interpretation and explicit verification-ref inputs. Independent
worker sources and earlier evidence are retained in the durable execution queue.

Combined local evidence covers catalog/build/scenario validation, both-team edits,
actual encrypted custom builds and save/load, multi-character/spectator/reassignment
policy, 12+ move input/inspection and a six-process guest kill/reconnect/completed
fight. Metal frames cover setup/overflow at Auto/200%; static rendering is not a
desktop playthrough. Draft [PR #39](https://github.com/chillgamerboys/bevy-experiments/pull/39)
exists. The final source-bound command graph and delivery observation remain pending.

S6/S9 builder usability acceptance was reopened: the user rejected the
unexplained button lists and organization despite the preceding checks. The
replacement separates squad preparation from one character editor, with explicit
inspection, effective mechanics, build comparisons and retained drafts. See the
[research and alternatives](../../games/labyrinth/docs/plans/builder-ui-research.md).
Reachability and unclipped screenshots did not establish understandable choices.
Other remaining acceptance includes S9/S10: current CI/review state, interactive desktop
feel and cross-machine network routes where available. S1 retains independent
forward/native-client/cost evaluation gaps. MCP tracking now has a fresh snapshot
path in the delivery checker; the helper and its credentials are optional. Retain
pending/unavailable observations without calling them passes, and distinguish
actual merge requirements from broader evaluation coverage during the fresh audit.
The deferred inventory/passive/gym scope remains unchanged.

## Historical evidence at the prior UI revision

The combined preflight at `0291105` passed configured Labyrinth, rules, Gamekit UI,
Clippy, formatting, docs and skill checks. Final publication-source results belong
in PR #39 and the durable delivery observation; this preflight is not relabeled
when the final copy or Git source changes.

| Slice | Implemented evidence | Remaining acceptance |
|---|---|---|
| S1 GameSkills | Numbered grill, explicit UI planning/verification, tested evidence-ref correction; independent baseline/candidate planning pair | Native routing, installed candidate and comparative cost unobserved; pair showed no demonstrated improvement |
| S2 Catalog | Data-authored definitions, deterministic provenance-aware resolution and validation tests | No inventory or passive execution claimed |
| S3 Scenarios | Both-team validated scenarios, stock variants, seeds, JSON round trips and controller seam | Full gym/training deferred |
| S4 Weapons | Six weapon definitions, additive/upgrading learned techniques, rank/target and transactional cleave tests | Balance remains experimental |
| S5 Co-op | Ownership, spectator, reassignment, replay and encrypted session/process regressions | Cross-machine LAN/Tailscale unobserved |
| S6 Editor | One editor, both teams, inspect-before-change, comparison, drafts, revision/authority guards; production input tests and Metal frames | Desktop usability and actual player comprehension pending |
| S7 Hotbar | Complete granted list, overflow/input, provenance/forecast and stale displayed-build regression | Desktop feel and resizing walk pending |
| S8 Gamekit | Shared focus fix and UI tests; composite source/disclosure/input/lifecycle contracts; demonstrated consumer hotbar fix | No newly extracted composite widget or package-release claim |
| S9 Integration | Combined automated suites and rendered review; historical six-process kill/reconnect/fight evidence | Final-source process observation and unavailable manual routes must remain distinct |
| S10 Delivery | Existing PR #39 and exact HEX-99/project bindings; MCP snapshot delivery and original-pin native registration corrections | Fresh-agent environment/acceptance audit and authorized merge; release outside scope |

The source/static review found no further concrete hotbar or squad-shell defect
after correction. Rendered review separately caught the roster breakpoint,
200% first-fold information and misleading Parameters instructions. Correcting
those findings supports visible information and layout claims; it does not supply
the missing player observations. Plans remain active for that outstanding work.

## Spatial preparation and installation correction

The second hands-on rejection exposed an interaction-model failure: card grids
obscured the actual six-rank formation, and adding a character skipped type choice.
Answers 9–10 now drive one facing formation board with visual type previews,
explicit placement, sparse construction, compact deployment and place ownership.
Current contracts live in the game architecture/README; all character fields still
use one editor. Focused policy, encrypted construction and production-input tests
cover the new model. Actual rendered review corrected buried mechanics/rank limits,
collision explanations and disabled-state emphasis. Human acceptance remains open.

GameSkills plan/grill/build-ui/verify-ui now connect task goals, information
hierarchy, interaction model, recurring views, authority and concrete final
appearance. Spatial UI is one option, not a replacement for those other concerns.
Review must challenge composition and complete-task friction despite passing bounds.

The user also identified native installation readiness as a serious package gap.
The original pin was staged correctly and discovered with generated launch flags,
but ordinary host registration was absent. The added runtime scope is persistent
Codex project registration, preservation/recovery, explicit readiness and ordinary
discovery without injected enabling settings. The fresh verification session now
exposes all 23 selected skills, and its ordinary `codex app-server --stdio` probe
passes for the preserved original pin. This closes the observed registration and
fresh-session catalog gap; it does not establish activation or behavioral efficacy
of the revised instruction candidate. Preserve the original pin and earlier
negative evidence; final source-bound validation and the observed environment
identities belong in the evaluation ledger and PR delivery record.

## Agreed contract

See the [game implementation contract](../../games/labyrinth/docs/plans/weapons-and-battle-setup.md)
and [GameSkills evaluation plan](../../gameskills/docs/plans/skill-evaluation.md).
The numbered grill decisions are retained there. This epic supplies integration
contracts, actionable child scopes and dependency/acceptance boundaries.

## Integration contracts

- **Content/build:** stable IDs reference versioned game-owned definitions; grants
  retain provenance and upgrades retain their contributions. New content using
  known effects is data-only; new effect semantics are typed/tested Rust. Authoring
  format defaults to TOML. Catalog/resource bounds protect validation/transport
  while permitting every move granted by a valid build, including more than eight.
- **Scenario/replay:** parsed battle input is independent of I/O and Bevy. Same
  canonical content, rules, scenario, seed and ordered actions reproduce behavior.
  Freeze content for the encounter. Save game configuration without credentials
  or machine-specific peer assignments; new lobbies rebind controllers explicitly.
- **Compatibility:** initial co-op requires matching code/rules/catalog identity;
  host sends the authoritative scenario/resolved build view and clients validate it.
  Do not add mod downloading. Change wire/rules versions with action/snapshot/event
  semantics; old clients reject clearly. Review actual payload budgets.
- **Authority:** host owns scenario/enemies/formation/controller assignment; players
  edit owned builds in lobby. Every gameplay command names an actor and current
  decision/assignment context. Stale authority must fail even after assignment
  moves away and back. Preserve sequence replay protection and combat resources.
- **Pause/reconnect:** add host-controlled assignment pause distinct from local
  menus, missing-controller suspension and rules failure. Spectators never gate
  Ready/Start or pause on disconnect. Live/dying assigned heroes preserve the
  controller reconnect requirement until explicit reassignment. Host can spectate
  while retaining session authority. No new host restart or late-join guarantees.
- **UI/controller:** UI and AI consume resolved immutable views and submit the same
  legal action vocabulary. Selecting/inspecting another actor never changes control.
  All active abilities remain accessible in automatic groups/rows; no favorites or
  layout customization. Test policies use the same combat boundary as future RL.
- **Shared packages:** capabilities stay optional and independent of game nouns.
  Assessment begins with implementation and produces findings plus justified fixes,
  not a forced extraction quota. Skill guidance is measured on actual selection,
  execution and endpoint preservation, separately from tests/discovery.

## Sequencing and review slices

Start S1 immediately alongside S2. Integrate S2 → S3, then S4 and S5; S6 follows
S3/S5 and S7 follows S4/S5. S8 evaluates capabilities from the start but cannot close
before S6/S7 consumer evidence. S9 integrates S4/S6/S7. S10 closes reviewable delivery
only after S1/S8/S9 evidence is settled. These dependencies do not authorize agents.

Use coherent PR slices for content/scenarios, weapons/rules, session ownership,
setup/ability UI and observed package fixes where each builds and preserves a
usable intermediate state. Combine coupled wire/model migrations when needed.
Do not publish a partially wired playable route simply to match issue boundaries.
Every actual PR needs its own relevant source-bound checks and issue links; the
epic tracks the complete set. No exact dates or parallel staffing are assumed.

Execution can be solo or delegated; this epic does not require one worker.
`gameskills.toml` records owner-approved parallel implementation with at most five
workers. Reconcile applicable existing authorization with current session
instructions and runtime capacity before assigning independent scopes. Do not turn
a handoff caveat or missing execution decision into a permanent epic constraint.

## Child issues

Epic: [HEX-99](https://linear.app/chillgamerboys/issue/HEX-99/epic-labyrinth-customization-and-configurable-co-op-battles-with) in Bevy Games.
Epic UUID: `2d2cd063-105a-4ea0-ad4f-71e8925a2550`; project UUID: `401f99ba-c1ec-4494-ab0d-ebf0134d8b79`.

| Slice | Issue | Owner | Completion dependencies |
|---|---|---|---|
| S1 | [HEX-100](https://linear.app/chillgamerboys/issue/HEX-100/evaluate-and-improve-gameskills-throughout-the-labyrinth-customization) | GameSkills | None |
| S2 | [HEX-101](https://linear.app/chillgamerboys/issue/HEX-101/build-a-validated-content-catalog-and-provenance-aware-character-build) | Labyrinth | None |
| S3 | [HEX-102](https://linear.app/chillgamerboys/issue/HEX-102/add-reproducible-custom-battle-specifications-and-a-controller-seam) | Labyrinth | HEX-101 |
| S4 | [HEX-103](https://linear.app/chillgamerboys/issue/HEX-103/implement-starter-weapons-learned-techniques-and-deterministic-multi) | Labyrinth | HEX-101, HEX-102 |
| S5 | [HEX-104](https://linear.app/chillgamerboys/issue/HEX-104/support-multi-character-controllers-spectators-and-paused-host) | Labyrinth | HEX-102 |
| S6 | [HEX-105](https://linear.app/chillgamerboys/issue/HEX-105/build-the-battle-editor-and-co-op-lobby-for-scenarios-builds-and) | Labyrinth | HEX-102, HEX-104 |
| S7 | [HEX-106](https://linear.app/chillgamerboys/issue/HEX-106/display-every-granted-ability-with-provenance-targeting-and-multi) | Labyrinth | HEX-103, HEX-104 |
| S8 | [HEX-107](https://linear.app/chillgamerboys/issue/HEX-107/exercise-and-improve-gamekit-capabilities-through-the-customization) | Gamekit | HEX-105, HEX-106 |
| S9 | [HEX-108](https://linear.app/chillgamerboys/issue/HEX-108/playtest-and-validate-custom-battles-weapons-and-co-op-end-to-end) | Labyrinth | HEX-103, HEX-105, HEX-106 |
| S10 | [HEX-109](https://linear.app/chillgamerboys/issue/HEX-109/audit-all-epic-outcomes-and-deliver-the-implementation-prs-with) | Shared delivery | HEX-100, HEX-107, HEX-108 |

All ten children, project routes and dependency edges were read back through the
connected Linear MCP on 2026-09-13 during planning. That historical observation
does not establish their current status or completion. PR #39 publishes the plan
and implementation; Linear descriptions retain the substantive scopes independently. The design delivery record is
`labyrinth-customization-epic-plan`; later implementation uses PR-endpoint records.

### S1: Evaluate and improve GameSkills throughout the Labyrinth customization epic

#### Result and scope

Start before implementation and keep the evaluation ledger active through final delivery. Record installed CLI/bundle/source hashes, actual client/model/effort where exposed, native availability versus canonical fallback, configuration and baseline revision. Assess plan, grill, model-rules, build-ui, multiplayer, testing, debugging, docs, PR and audit behavior on the actual work.

Implement the observed grill improvement through author-skill: visible stable question numbers in both tools and prose, answered-decision recovery, and no repeated interview for settled choices. Preserve the initial user correction and test numbered replies. Add narrowly justified corrections discovered during the epic.

Run supplemental README-only, mixed/custom docs plus contradiction, and completed-plan/moved-anchor/focused-request exercises. Track resumed endpoint preservation on real interruptions; do not manufacture an independence claim. Use equivalent baseline comparisons when feasible. Explicit independent-agent authorization is still required.

#### Ownership and dependencies

Owner: GameSkills.
Source areas: `gameskills/plugins/, gameskills/docs/, gameskills/cli/ only for observed runtime defects`.
Completion dependencies: none; begin at epic start.
Assessment begins early and remains active while dependent game work develops.

#### Acceptance

- [ ] Every implementation slice records useful guidance, missed instructions, friction, corrective interventions, outputs and endpoint evidence; unavailable telemetry is labeled unavailable.
- [ ] The numbered-grill change is demonstrated by actual answer mapping; original failure remains recorded. Candidate identities distinguish pre-fix and post-fix runs.
- [ ] Canonical skills, current docs and required distribution bundle/archive checks stay aligned; installed bundles, overlays, pins and historical records are preserved.
- [ ] Final recommendation states supported claims and untested clients/environments. Rust tests and native discovery are not behavioral acceptance.
- [ ] Maintain documented follow-ups for untested independent/native/update/recovery/two-pin cases; no artificial 45-minute cutoff.

#### Verification

Configured skills-validate, focused CLI/Linear tests for affected runtime paths, bundle/archive consumer checks when distributing changed instructions; inspect behavioral outputs directly.

#### Outside this slice

No automatic delegation, release, adopter migration, Linear deletion or prerequisite framework rewrite.

#### Delivery

Part of the Labyrinth customization epic. Follow repository GameSkills workflow, preserve accepted decisions, and link current-source evidence and actual PRs. No extra agents, merge or release are implied. A PR-endpoint result is distinct from merged/Done tracking state. Consult owner architecture/testing docs and the three active epic/game/evaluation plans.

### S2: Build a validated content catalog and provenance-aware character build resolver

#### Result and scope

Introduce stable IDs and versioned, human-editable catalog definitions for abilities, weapons, learned skills and actor presets. Prefer TOML for authored files unless a demonstrated parsing/distribution issue warrants another format. New items using existing effects require data changes only; new behavior remains an explicit tested Rust effect.

Separate immutable definitions, grant sources, upgrade contributions and actor-owned mutable resources. Resolve innate grants + selected learned skills + one equipped weapon into ordered active moves with full provenance. Learned skills can add moves or apply explicit compatible upgrades; future passive/reactive execution remains deferred. Preserve multiple grant paths without duplicate move entries. Reject unsupported conflicts/cycles or invalid references before play.

Make class/visual presets defaults instead of authority over HP, speed, footprint or loadout. Replace the eight-move gameplay limit with validated catalog/resource bounds that permit all granted moves. Retain universal actions and deterministic ordering.

#### Ownership and dependencies

Owner: Labyrinth.
Source areas: `games/labyrinth/rules/src/{content,loadout,model}.rs; proposed rules catalog/build modules and game-owned content files`.
Completion dependencies: none; begin at epic start.


#### Acceptance

- [ ] Add a weapon and adjust its damage/reach through content definitions without adding another item enum arm or hard-coded UI/AI mapping.
- [ ] Invalid IDs, duplicate definitions, missing references, bad rank masks/values, incompatible upgrades and unsupported effect types report useful source/field errors.
- [ ] One example adds a learned active move and another upgrades a weapon move; inspection data retains base grant and upgrade sources.
- [ ] Duplicate grants collapse to one move; removing a source removes only its contributions. Empty weapon selection is a valid test build with universal actions.
- [ ] Different actors sharing definitions retain independent HP/statuses/use counters; resolved builds serialize/validate with more than eight active moves.
- [ ] Legacy presets remain available through the new catalog and consumers never reconstruct authority from archetype enums.

#### Verification

Pure catalog/build/serialization regressions, configured rules-test and rust-format; update the local architecture/rules authoring contract.

#### Outside this slice

Inventory containers, acquisition, offhand slots, dual wielding, progression trees, arbitrary scripting, passive/reaction execution.

#### Delivery

Part of the Labyrinth customization epic. Follow repository GameSkills workflow, preserve accepted decisions, and link current-source evidence and actual PRs. No extra agents, merge or release are implied. A PR-endpoint result is distinct from merged/Done tracking state. Consult owner architecture/testing docs and the three active epic/game/evaluation plans.

### S3: Add reproducible custom battle specifications and a controller seam for simulation

#### Result and scope

Define the shared validated input used by local play, co-op setup, saved presets and future headless experiments: seed, both ordered rosters, stable actor IDs, visual presets, footprint, max/current HP, speed, builds and supported starting conditions. Support small test encounters through full six-space formations. Keep I/O in the app; rules accept parsed validated data.

Provide versioned scenario save/load and immutable resolved content/configuration identity. Catalog edits cannot change a running fight. Initial co-op requires compatible rules/catalog versions; host supplies the authoritative scenario and clients validate its references and derived builds. Do not add a mod downloader.

Expose a controller boundary that consumes the authoritative decision/observation and legal actions. Existing enemy AI is the default policy; deterministic test policies can replace it using the same apply/reject path. Preserve reset-from-spec/seed and action enumeration for the eventual gym.

#### Ownership and dependencies

Owner: Labyrinth.
Source areas: `games/labyrinth/rules/src/{combat,model}.rs; proposed battle-spec module; game-owned scenario files and app loading adapter`.
Completion dependencies: S2.


#### Acceptance

- [ ] Hero and enemy stats, footprint, loadout and starting order are configurable independently of appearance; both teams use the same build validation.
- [ ] Save/load plus the same content/rules/seed/actions yields the same transitions; scenario identity distinguishes changed stats/content.
- [ ] Invalid full/small rosters, collisions, unsupported states or malformed definitions fail atomically with no partially started encounter.
- [ ] Current encounter remains a stock scenario; weapon comparison, cleave/large-actor and rescue/status stock scenarios are authored and documented.
- [ ] AI and a test policy act through identical legality/commit semantics; stale/illegal policy output does not mutate state.
- [ ] Rules run headlessly without Bevy, filesystem, transport or Python dependencies; no reward/training framework is required.

#### Verification

Pure battle setup/replay/reset/serialization/controller tests; configured rules-test; app load/save smoke checks.

#### Outside this slice

Full Gym/Python bridge, rewards, training, campaign saves, runtime content hot reload, mod distribution.

#### Delivery

Part of the Labyrinth customization epic. Follow repository GameSkills workflow, preserve accepted decisions, and link current-source evidence and actual PRs. No extra agents, merge or release are implied. A PR-endpoint result is distinct from merged/Done tracking state. Consult owner architecture/testing docs and the three active epic/game/evaluation plans.

### S4: Implement starter weapons, learned techniques and deterministic multi-target cleave

#### Result and scope

Ship dagger (stab, unlimited throw), greatsword (cleave enemy ranks 1–2, single-target thrust), two-handed axe (overhead chop), spear (reach thrust, shove), bow (aimed/flexible shots) and staff (strike/push). Each requires only one equipped-weapon selection. Populate coherent party/enemy stock builds and at least one learned new move plus one learned upgrade.

Author provisional HP/damage/rank/use values explicitly for tuning. Reuse the existing effect primitives where possible. Cleave captures distinct eligible occupants of ranks 1–2 before resolution: a large actor covering both receives one hit. Preserve deterministic effect order and prevent corpse clearing from selecting newly exposed replacements mid-action. Use the same expansion for legality, forecasts, AI and commit.

#### Ownership and dependencies

Owner: Labyrinth.
Source areas: `games/labyrinth/rules/src/{content,resolve,preview,combat,model}.rs; game content and scenario definitions`.
Completion dependencies: S2, S3.


#### Acceptance

- [ ] All six weapon definitions are usable in appropriate authored formations; dagger throw neither consumes the weapon nor an encounter charge.
- [ ] Greatsword cleave hits two single-rank occupants once each and one two-rank occupant once; one-target/empty-rank/corpse fixtures obey the documented eligibility contract.
- [ ] Captured targets stay fixed across lethal damage/formation compaction; outcome checks do not silently drop required effects.
- [ ] Preview matches immediate committed outcomes without RNG/use/turn mutation; each actor contributes damage/status only as specified.
- [ ] Learned additions/upgrades have visible provenance and don't duplicate, reset or share use counters.
- [ ] Changing semantics updates rules compatibility and every affected snapshot/action/event consumer.

#### Verification

Rules and preview parity tests for multi-target/life-state boundaries; legal-action/AI tests and stock encounter simulation; initial play observations remain separate.

#### Outside this slice

Final balance claims, armour systems solely for axe flavor, consumable ammunition, inventory management, passives.

#### Delivery

Part of the Labyrinth customization epic. Follow repository GameSkills workflow, preserve accepted decisions, and link current-source evidence and actual PRs. No extra agents, merge or release are implied. A PR-endpoint result is distinct from merged/Done tracking state. Consult owner architecture/testing docs and the three active epic/game/evaluation plans.

### S5: Support multi-character controllers, spectators and paused host reassignment

#### Result and scope

Replace one-player/one-actor assumptions with distinct participant, actor, controller-assignment and formation identities. Each hero has exactly one controller; each player has zero or more heroes. Host controls initially unclaimed heroes, assigns initial places/owners and controls all enemy configuration. Players edit their assigned builds. Retain six participant connections initially, independently bounded from six formation spaces.

Add explicit host pause/reassignment/resume policy. Reassignment changes authority only and is atomic between decisions; it never changes rank, stats, status, uses or initiative. Revoke old-controller requests with assignment revision/current ownership checks while preserving replay protection. Reconnect observes current assignments rather than restoring stale ownership.

Zero-hero participants are spectators; permanent loss of all heroes removes action authority. Dying heroes retain assignment for rescue. Spectator loss does not pause play. Controller loss pauses until reconnect or host reassignment; intentional pause and rules-failure reasons remain distinct.

#### Ownership and dependencies

Owner: Labyrinth.
Source areas: `games/labyrinth/src/{session,view}.rs; src/network/{protocol,requests,admission,start}.rs and owning tests`.
Completion dependencies: S3.


#### Acceptance

- [ ] One player can control multiple heroes, a player can control none, and a full formation works with fewer connected players.
- [ ] Guest commands cannot edit other builds, assign owners/ranks, start/resume encounters or configure enemies; host-only operations are enforced by authority.
- [ ] Reassign the active actor while paused; old-owner queued/replayed commands are rejected even after assignment away and back. New owner acts once without resource refresh.
- [ ] Disconnect/reconnect retains the same participant, current assignment and exact combat boundary; a spectator cannot block or resume combat.
- [ ] Death-to-spectator, surviving-second-character, dying/rescue and host-as-spectator cases preserve command eligibility correctly.
- [ ] Protocol/catalog compatibility, payload bounds, invitation accounting, ready invalidation and UI projections migrate together; old builds fail clearly.

#### Verification

Pure session tests, real encrypted multi-App tests and process-reconnect tests; affected multiplayer budgets/admission tests. Record cross-machine evidence separately.

#### Outside this slice

Host migration, host process persistence, new mid-combat admission beyond existing reconnect, unlimited spectators, silent AI takeover.

#### Delivery

Part of the Labyrinth customization epic. Follow repository GameSkills workflow, preserve accepted decisions, and link current-source evidence and actual PRs. No extra agents, merge or release are implied. A PR-endpoint result is distinct from merged/Done tracking state. Consult owner architecture/testing docs and the three active epic/game/evaluation plans.

### S6: Build the battle editor and co-op lobby for scenarios, builds and assignments

#### Result and scope

Provide stock scenario selection, clone/edit, saved scenario load/save, seed editing and repeat-seed restart. Host configures both rosters, order/footprints and enemy builds/stats. Each player chooses preset, innate/learned abilities and weapon for each assigned hero; host edits its own/unclaimed builds. Show invalid fields and aggregate formation conflicts before Ready/Start.

Display participants with their assigned characters plus a distinct formation view. Zero assignments show Spectator. Allow host reassignment through the agreed paused-session flow without treating a local menu as a global pause. Refresh joins/reconnects/ownership changes without destroying unrelated form edits or keyboard focus.

Full customization means roster/build/stat setup; the UI need not create weapon definitions, effect types, inventory grids or skill trees.

Accepted UI revision (grill 7–8): B, squad preparation, with exactly one unified
character editor for heroes and enemies. Keep equipment, innate/learned abilities
and existing character battle parameters in that screen with a single draft
lifecycle. The future character stat system remains unimplemented and out of this
revision; when added, it belongs in the same editor. Plan for reuse as the detailed
in-game character view without authorizing mid-battle editing. Internal navigation
and responsive layouts may change; separate character editors may not proliferate.

#### Ownership and dependencies

Owner: Labyrinth.
Source areas: `games/labyrinth/src/ui/shell/{lobby,forms}.rs; profile.rs, main.rs, view.rs; game UI tests`.
Completion dependencies: S3, S5.


#### Acceptance

- [ ] Create a fresh custom local encounter and a co-op encounter from stock or saved data without editing Rust.
- [ ] Squad selection opens one unified character editor for either team; character parameters and builds share its draft/apply/discard flow. Scenario settings do not introduce another character editor.
- [ ] Before equipping/learning, a player can inspect effective moves, rank restrictions, prerequisites and the proposed changes; visible information agrees with the game-owned resolver, including upgrades and duplicate grants.
- [ ] Switching characters or sections preserves or explicitly resolves draft changes; browsing never silently equips a choice. Separate information-availability evidence from actual observed player comprehension.
- [ ] Host assigns multiple heroes to a guest and none to another; owners edit only permitted builds, all peers see accepted authoritative changes.
- [ ] Changing footprint cannot overfill six spaces or evict another player's character silently; affected readiness is invalidated coherently.
- [ ] Saved scenario contains game configuration, not connection credentials/peer identity; load is validated and failure leaves the prior draft intact.
- [ ] Stock/full/small formations and at least two seeds launch/restart reproducibly.
- [ ] Native pointer/keyboard and resize walks cover setup, validation, save/load, ready/start, spectator and paused reassignment; focus survives roster updates.

#### Verification

Production UI intent/focus/layout tests plus native interaction/render checks; session authority tests reused from S5.

#### Outside this slice

Content authoring UI, customizable hotbars, inventory UI, mid-fight stat/equipment editing.

#### Delivery

Part of the Labyrinth customization epic. Follow repository GameSkills workflow, preserve accepted decisions, and link current-source evidence and actual PRs. No extra agents, merge or release are implied. A PR-endpoint result is distinct from merged/Done tracking state. Consult owner architecture/testing docs and the three active epic/game/evaluation plans.

### S7: Display every granted ability with provenance, targeting and multi-character control

#### Result and scope

Replace fixed-eight assumptions with an automatic stable layout for all granted active moves and universal actions. Prioritize visibility and usability; no favorites, drag/reorder or customization. Use responsive rows/groups and explicit scrolling only when needed, keeping actor state, targets and Confirm readable. Mouse and keyboard can reach every move beyond eight.

Show effective move descriptions, source/upgrade provenance, rank requirements, illegal-action reasons and immediate forecasts from resolved combat data. Multi-target cleave highlights every captured target and forecasts the same effects commit will apply. Changing viewed/active/owned characters refreshes the list without changing authority or leaking a stale pending command.

Spectators can inspect all currently disclosed information but cannot submit combat commands. Existing hidden-information fixtures must not gain exact facts from provenance/derived forecasts.

#### Ownership and dependencies

Owner: Labyrinth.
Source areas: `games/labyrinth/src/ui/battle/{dock,inspection,tooltips,actors,layout}.rs; presentation.rs and UI tests`.
Completion dependencies: S4, S5.


#### Acceptance

- [ ] A build with at least twelve active moves shows and reaches each move by pointer and keyboard at supported layouts; none disappear due to shortcuts or clipping.
- [ ] Off-turn/illegal moves remain inspectable with reasons; Confirm acts only for the current authorized actor.
- [ ] Grant and upgrade sources are legible without duplicate abilities, including multi-source grants and long content names.
- [ ] Cleave target highlights and HP/effect forecasts agree with rules; inspection never commits or changes the selected target accidentally.
- [ ] Character turn changes, death, reassignment and reconnect clear invalid selections and refresh effective moves/ownership.
- [ ] Automatic layout passes production geometry/input tests, rendered review and native use; no customizable hotbar work enters this slice.

#### Verification

Focused UI/presentation/preview regressions and native walks at 1280x720 and wider layouts, automatic and relevant large-text scale; owner testing guide determines final coverage.

#### Outside this slice

Custom hotbars, favorites, drag/drop, new art pipeline or passive activation UI.

#### Delivery

Part of the Labyrinth customization epic. Follow repository GameSkills workflow, preserve accepted decisions, and link current-source evidence and actual PRs. No extra agents, merge or release are implied. A PR-endpoint result is distinct from merged/Done tracking state. Consult owner architecture/testing docs and the three active epic/game/evaluation plans.

### S8: Exercise and improve Gamekit capabilities through the customization workflows

#### Result and scope

Start capability assessment during design and early integration, even though completion waits for both UI routes. Inventory the actual shared APIs used by setup forms, validation feedback, focus/scrolling, contextual help, dynamic rosters, deterministic input/layout tests and transport/admission. Record successful reuse, awkward contracts and concrete defects.

Make demonstrated reusable corrections with minimal optional APIs and precise public scheduling/lifecycle contracts. Keep game-specific equipment, six-rank policy, combat ownership and scene organization in Labyrinth. A second consumer is useful evidence, not a quota or permission to invent feature work.

For each changed capability inspect real callers, supported feature combinations and consumer migration/removal implications. Validate reusable behavior independently and in affected adopters; source compilation is distinct from packaged consumer success.

#### Ownership and dependencies

Owner: Gamekit.
Source areas: `gamekit/ui/, gamekit/testing/, affected session/multiplayer primitives; Gamekit Rustdoc and consumer wiring`.
Completion dependencies: S6, S7.
Assessment begins early and remains active while dependent game work develops.

#### Acceptance

- [ ] A capability assessment maps exercised APIs to observed behavior, findings, chosen fixes or evidence-backed no-change conclusions.
- [ ] Observed actionable deficiencies are resolved or explicitly retained with impact/owner; equal priority is not measured by a forced code-change count.
- [ ] Shared crates remain independent of Labyrinth rules and can be adopted without starting services or imposing game layouts.
- [ ] Capability tests and affected adopter checks support the changed contract; UI mechanics include production/native evidence where claimed.
- [ ] Rustdoc, current integration docs and examples match the final API; source and actual package-consumer checks are distinct.
- [ ] No incompatible migration or regression in other games is hidden by only testing Labyrinth.

#### Verification

Configured ui-test plus affected capability/consumer tests, feature checks and targeted external package-consumer verification when packaging contracts change.

#### Outside this slice

Universal inventory/combat frameworks, unrelated feature extraction, release or upstream Bevy work without a demonstrated need.

#### Delivery

Part of the Labyrinth customization epic. Follow repository GameSkills workflow, preserve accepted decisions, and link current-source evidence and actual PRs. No extra agents, merge or release are implied. A PR-endpoint result is distinct from merged/Done tracking state. Consult owner architecture/testing docs and the three active epic/game/evaluation plans.

### S9: Playtest and validate custom battles, weapons and co-op end to end

#### Result and scope

Exercise the complete player journeys using the same recorded build/content/seed identities: edit/load scenario → assign owners → customize builds → ready/start → use every weapon/learned move → death/rescue → pause/reassign → reconnect → finish/restart.

Run current encounter, weapon comparison, multi-rank cleave and rescue/status scenarios with small and full formations and multiple reproducible seeds. Tune obviously unusable values through data and preserve before/after observations; no RL training or balanced-release claim is required.

Test localhost encrypted/process routes and available cross-machine routes as separate evidence. Observe actual pointer/keyboard interactions and responsive layouts; retain prior admission/refusal timeout history without assuming a fresh failure is the same flake.

#### Ownership and dependencies

Owner: Labyrinth.
Source areas: `games/labyrinth owning tests/docs and target/review artifacts`.
Completion dependencies: S4, S6, S7.


#### Acceptance

- [ ] Acceptance matrix links each selected mechanic, build/configuration contract and session permission to passed/failed/pending/unavailable evidence.
- [ ] Complete fights and rejected-action/replay invariants pass on the final source with content fingerprints recorded.
- [ ] Human play observations, agent interaction, rendered frames, pure rules and network process checks are distinguished.
- [ ] Every granted ability is reachable/usable when legal; all six weapons and both learned composition modes have demonstrated routes.
- [ ] Co-op test covers multi-hero player, explicit spectator, last-hero death, paused reassignment and subsequent legal action after reconnect.
- [ ] Material gaps remain explicit and prevent unsupported acceptance claims; bounded tuning outcomes and current testing guidance are updated.

#### Verification

Configured labyrinth-test, labyrinth-lint, rust-format, docs-check; relevant rendered/native/process tests and impacted wider CI gates. Reuse valid unchanged evidence.

#### Outside this slice

Full balance optimization, performance benchmarks without a concrete claim, RL training and global platform-parity claims.

#### Delivery

Part of the Labyrinth customization epic. Follow repository GameSkills workflow, preserve accepted decisions, and link current-source evidence and actual PRs. No extra agents, merge or release are implied. A PR-endpoint result is distinct from merged/Done tracking state. Consult owner architecture/testing docs and the three active epic/game/evaluation plans.

### S10: Audit all epic outcomes and deliver the implementation PRs with evidence

#### Result and scope

Deliver the final scoped outcome through reviewable PRs against main. Plan foundations/gameplay, co-op/session, UI and package fixes as review slices when each leaves a coherent build; adjust boundaries to avoid landing an unusable intermediate state. Do not require one giant PR or create empty PRs just for the issue layout.

Collect requirement-linked evidence for Labyrinth, Gamekit and GameSkills equally. Track every actual PR/source/base and adopted issue/project UUID. Update PR bodies for final scope, link both directions, inspect remote publication and CI/reviews, and run delivery observation on the actual record. Required tracking uses a fresh connected MCP issue lookup and task/source-bound snapshot; the command observer is an optional route for deliberately configured automation.

Preserve outstanding work when retiring completed plans and repair links. Carry package findings into current documentation and the behavioral recommendation. No merge/release permission is implied by PR creation.

#### Ownership and dependencies

Owner: Shared delivery.
Source areas: `owner docs/plans, GameSkills delivery records, GitHub PRs and Linear bindings`.
Completion dependencies: S1, S8, S9.


#### Acceptance

- [ ] Every required child outcome is satisfied or explicitly pending; no tracker completion is inferred from local tests or PR creation.
- [ ] Each actual PR is observed on the right repo/base/source and linked to the right Linear object, with exact UUID bindings persisted.
- [ ] Final audit maps all epic criteria to current-source evidence and names missing interactive/network/native-client observations.
- [ ] GameSkills promotion/limited-use/rejection recommendation and Gamekit capability findings are published with negative results retained.
- [ ] Current owner docs and skill pointers agree with implementation, finished plans are retired only after remaining work is preserved.
- [ ] Tracker Done requires authorized integration and acceptance of all required PRs; this issue can remain In Review at the reviewable-PR endpoint.

#### Verification

Review judgment, required CI, current command evidence validation, and fresh MCP
issue/PR reads with exact two-way links. Pass each task's fresh normalized snapshot
to its delivery check. Check PR acceptance before the authorized merge, then observe
integration through the separate merge task from the unchanged source checkout.

#### Outside this slice

Unrequested merge, release, messages to reviewers, credential extraction, unrelated cleanup.

#### Delivery

Part of the Labyrinth customization epic. Follow repository GameSkills workflow, preserve accepted decisions, and link current-source evidence and actual PRs. No extra agents, merge or release are implied. A PR-endpoint result is distinct from merged/Done tracking state. Consult owner architecture/testing docs and the three active epic/game/evaluation plans.

## Exit criteria and evidence

Use the owner tests, native routes and behavioral trials named in each child.
Engineering correctness, rendered appearance, native input, human play feedback,
network routes, package consumers, skill behavior and remote delivery are distinct
claims. Reuse evidence only while inputs remain valid. Record failures/retries and
missing telemetry honestly. Maintain an acceptance matrix keyed by S1–S10 with
actual source/config/content and artifact/run identities.

This planning task is complete after the parent, ten child descriptions, owner
routing, dependency edges and local references have been verified in Linear.
Gameplay/PR completion is not claimed by that result. The later epic cannot be
marked Done from tests or an open PR; all required authorized integrations and
acceptance must be observed. Preserve deferred work and repair links when retiring
plans. The earlier requirement for a standalone Linear observer is resolved by the
adopted MCP delivery contract. Preserve the actual connector response and check a
fresh task/source-bound snapshot against the live GitHub backlink; no standalone
helper or separate API key is required for this route.

## Exclusions

Full inventory management, offhand/dual wielding, ammunition/retrieval, progression
or skill trees, passive/reaction execution, configurable hotbars, manual enemy
control UI, full Gym/training stack, host migration, new mid-combat admission,
registry release, linked-workspace migration and Linear cleanup are deferred.
