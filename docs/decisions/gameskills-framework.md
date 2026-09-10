# GameSkills and GameKit: a companion to Bevy

Status: draft for discussion, September 10, 2026. The priorities below are agreed;
the milestone boundaries, skill inventory and documentation layout are proposals.
This document does not claim that the framework refactor or a release is complete.

## Agreed direction

Make creating games with Bevy easier, help improve Bevy upstream, and contribute
to the engine's community through useful software and learning resources. This
purpose guides the skills, library, games and release decisions.

The framework assumes Bevy and understands GameKit throughout. GameSkills should
also help an existing Bevy project before it adopts any GameKit crate. GameKit
remains a set of opt-in capabilities using Bevy's own concepts and extension
points. Learning resources and library examples should be usable by developers
without an agent installed.

Implement and use GameSkills before the broad documentation and code refactors,
so subsequent work develops under the workflows we intend to ship. Labyrinth's
playable release is the primary product milestone. Develop Deckbuilder alongside
it to test whether shared capabilities and guidance transfer between games.
Carterfight retains its small offline-consumer role.

GameSkills supplies development guidance and supporting tools. GameKit supplies
reusable implementations. The games supply real requirements, examples and
evidence. Problems encountered during development should lead to the appropriate
game, GameKit, Bevy, documentation or skill improvement. A successful upstream
change may let us simplify GameKit and retire a workaround.

The owner controls this repository and its priorities. The existing Bevy Hex Game
is a later adoption case with other contributors: its changes need separate PRs
for their review. Its integration must not become a prerequisite for starting
our internal refactor. See the deferred [Port Vila pilot](port-vila-adoption.md).

## Proposed order

| Stage | Work | Evidence needed to move on |
|---|---|---|
| 1. Working skills foundation | Redesign the core and optional offerings, migrate useful lessons, and adopt an installable pinned candidate in this repository | Real tasks in Codex and Claude demonstrate useful selection, correct execution, accurate evidence and complete delivery; installation and source ownership are explicit |
| 2. Documentation refactor | Use those skills to inventory, reconcile and restructure the documentation | One maintained owner per topic; working links and references; current commands and status; historical decisions remain recoverable |
| 3. GameKit and game refinement | Use the skills for bounded changes across Labyrinth and Deckbuilder | Both consumers retain their own rules and presentation; affected contracts and user paths are verified |
| 4. Playable candidate and packaging | Complete the agreed Labyrinth play loop, develop Deckbuilder alongside it, and prepare reproducible game/library/skill artifacts | Fresh game builds can be installed and played; library and skills can be consumed from the actual candidate artifacts |
| 5. Existing-game pilot | Revisit Port Vila with an immutable candidate and propose one useful UI integration | Reviewed adopter PRs establish installation, coexistence, runtime integration and a later update |
| 6. Supported releases | Resolve findings and publish the agreed support and compatibility commitments | Evidence matches the release artifacts and declared platforms; limitations, migration and rollback are documented |

Each stage should improve the skills when real work demonstrates a gap. A defect
in one workflow can be corrected without reopening the entire framework design.
Port Vila verifies existing-repository adoption; it is not the only evidence for
the games or an automatic blocker for a private Labyrinth playtest build.
Upstream investigation starts when useful evidence appears; it does not wait for
the final release stage or require every GameKit capability to move into Bevy.

## Stage 1: redesign the offerings before migrating the skills

The seven [existing craft skills](../../skills/README.md) are research inputs,
not a fixed inventory to preserve. Assess every instruction, reference and tool
against real development tasks. Keep useful principles and regression cases;
split, combine, move or retire the current entry points as the new design needs.

### Proposed core development offering

The core should help a developer go from a game idea or existing project to a
working, understandable and releasable game. These are workflow responsibilities;
the table does not fix a skill count or final command names. Separate entry points
need distinct user requests and useful instructions, established through trials.

| Responsibility | Useful outcome |
|---|---|
| Plan a playable change | Identify the player goal, current code, smallest useful slice and acceptance evidence; preserve decisions and authorization across turns |
| Build and refactor with Bevy | Use ECS, schedules, states and plugin composition appropriately; choose existing Bevy/ecosystem capabilities or GameKit based on the problem; keep game ownership explicit |
| Diagnose problems | Reproduce the failure with known versions, features, assets and runtime state; isolate game, plugin or engine responsibility before changing code |
| Verify engineering behavior | Select meaningful pure, minimal-app, runtime and performance checks; report what each observation establishes and what remains untested |
| Playtest and refine | Exercise the actual player journey, controls, feedback, onboarding and accessibility; keep human judgments of clarity and feel separate from automated checks |
| Review a change | Assess correctness, architecture, scope and evidence using the project's selected capability guidance |
| Document and teach | Maintain the authoritative explanation, Rustdoc, tested examples and migration instructions; make patterns understandable outside the agent workflow |
| Deliver and release | Complete commits, reviewable PRs and observed CI; handle merge and publication within authorization; verify actual artifacts and declared compatibility |
| Prepare a Bevy contribution | Investigate engine or documentation gaps, isolate evidence, explain technical tradeoffs and support a human contributor through Bevy's current process |

Routine implementation does not need a skill that merely repeats ordinary agent
instructions. Bevy-specific craft guidance may live in focused references loaded
by the relevant workflow. Setup and readiness checks should be deterministic
commands where possible, with a skill only where interpretation adds value.

### Proposed optional packages

Optional packages follow capabilities used by a game. Each can add specialist
skills, references, review criteria and verification commands to the same core
workflows. Avoid separate copies of planning, testing and PR delivery per genre.

| Package area | Initial purpose |
|---|---|
| UI and interaction | Bevy UI and GameKit UI integration, focus/input/modal ownership, responsive layout, visual and native interaction verification |
| Turn-based rules | Legal actions, state transitions, deterministic replay, stable identity and game-owned rules |
| Multiplayer | Authority, disclosure, sessions, admission, reconnect and real transport verification |
| Framework maintenance | GameKit API design and compatibility; skill authoring, evaluations, packaging and migrations for maintainers of this framework |

Simulation/balance, content pipelines, procedural worlds and specialized platform
publishing are later candidates when actual work justifies them. Labyrinth and
Deckbuilder should exercise the first packages; their selection should not define
requirements for every Bevy game. A project can select packages per application
in a monorepo. Detection may recommend a package; selection remains explicit.

### What to carry forward

The current architecture skill's ownership and scheduling lessons are useful core
craft guidance. Turn-based modeling belongs in an optional package. UI building
and UI verification should be evaluated together as a specialist offering.
Testing, debugging and review contain core lessons, with networking requirements
moved into conditional guidance. Current fixed presentation thresholds need a
reason tied to the relevant player context before becoming defaults.

The current installer hardcodes seven skills and installs all of them. Its pinned
source, three-way update and local-ownership protections are useful requirements
for the replacement. Its generated directory layout is not a settled design.
The validator checks structure and matching generated bodies; it does not prove
that either agent discovers or correctly uses a skill.

JXP PR #166 provides useful examples of optional native packages, project-owned
configuration, observed command results, source-bound evidence and update/recovery
behavior. Its issue-tracker, cloud and organizational requirements do not become
GameSkills defaults. Its refactor remains in progress; incomplete Claude and
model-driven workflow trials must not be presented as demonstrated parity.

### Installation and cross-agent support

Recommend one canonical skill source with thin packaging for Codex and Claude.
Prefer native distribution where current clients support it. Both the initial
installation and later updates must preserve project instructions, local skills
and configuration. Track selected packages and immutable source identity, and
declare tested Bevy/GameKit/client compatibility. Changes to canonical source
must not silently change an installed consumer candidate.

Prove packaging before committing to a directory layout. Current OpenAI guidance
prefers a portable root `plugin.json` and supports legacy manifests; Claude's
documented layout uses `.claude-plugin/plugin.json`. JXP's dual-manifest layout is
a concrete reference, not proof of what all supported client versions require.
Keep host differences in adapters and test them in the actual clients.

Acceptance must include installation and discovery, explicit and implicit skill
use, optional package selection, coexistence with local skills, and two projects
using different pinned candidates. Exercise update, interruption/recovery and
removal. Identical Markdown or successful copying is insufficient evidence of
cross-agent behavior. Add hooks only for a demonstrated need; document any trust
or activation requirements rather than making normal workflows depend on them
without verification.

### Bootstrap and evaluate with real work

Keep the first implementation proportionate. Instructions own judgment and
routing. Existing commands and small deterministic helpers own repeatable checks
and observations. Project configuration supplies commands, target branches and
capability-specific requirements. Issue trackers, cloud services and multiplayer
checks apply when the project uses them.

Implement the minimum candidate needed for the next documentation and development
tasks before expanding every optional offering. The bootstrap may use current
tooling and this direction, but should exercise the proposed distribution early.
After a candidate is ready, use its installed workflows for subsequent changes
and record which revision was used.

Evaluate meaningful behavior in addition to file structure:

- Compare representative tasks with the current pack and the candidate, with
  a no-skill baseline where useful. Include fresh tasks beyond the examples used
  to write the instructions, and record the model, harness and skill revision.
- Test when a skill should activate, when it should stay inactive, and how it
  coexists with other skills and local project instructions.
- Check the resulting code/artifacts and the evidence reported. A successful
  command must not be mistaken for proof of rendering, interaction or release.
- Exercise incomplete checks, changed inputs and resumed work. The workflow
  should report the actual state and continue appropriately within scope.
- Track corrective user interventions and unnecessary process alongside
  correctness. A larger instruction set is not itself an improvement.

First useful trials include a narrow UI correction, a rules or networking fix,
and a documentation update carried through a PR. Include a fresh Bevy project
and a small engine-only reproduction to check that instructions do not assume
our monorepo or require adopting GameKit. Reuse historical failure
scenarios as regression cases while retaining independent tasks for evaluation.
The [evidence reference](../../skills/references/evidence.md) remains the current
contract until deliberately revised.

## Improving Bevy and the community through real use

For a recurring difficulty, first identify where an improvement belongs:

| Finding | Likely destination |
|---|---|
| A game's rules, presentation or content choice | The game |
| Reusable application mechanics or an experimental capability | GameKit or an existing ecosystem project, after checking what already exists |
| An engine defect or broadly useful engine primitive | Bevy investigation and a focused contribution where maintainers agree it belongs |
| A discoverability or learning gap | API documentation, a focused example or a community learning resource |
| An agent made a poor decision or reported unsupported evidence | The relevant skill/tool, verified against the actual failure |

This is a routing aid, not a rule that every finding produces all five outputs.
Useful ecosystem code can remain independent. Bevy's [upstreaming guidance](https://bevy.org/learn/contribute/project-information/upstreaming/)
weighs usefulness, quality and maintenance cost, and supports contributing small
shared primitives when that achieves the main benefit.

For an upstream candidate, reproduce against a known Bevy revision, check existing
issues and proposals, and isolate the smallest relevant engine-only case. Compare
the supported release and current upstream state where relevant. Preserve the
technical observations, tests and design questions for the human contributor.
Larger design changes need early discussion under Bevy's [contribution process](https://bevy.org/learn/contribute/helping-out/opening-pull-requests/).
Track local workarounds and their removal conditions; retire them after adopting
an upstream version that resolves the problem.

Bevy's [AI policy](https://bevy.org/learn/contribute/policies/ai/), checked September
10, 2026, permits careful human-led assistance in some code-related areas. It
requires human understanding and ownership, disclosure of AI use, and human commit
authorship. It prohibits AI-generated public prose, communications and media.
The upstream workflow must therefore support investigation and understanding,
while the human contributor authors upstream issues, PR descriptions, docs and
communications. Ordinary delivery automation for this repository must not be
applied unchanged to Bevy. Recheck the receiving project's policy when preparing
a contribution; do not treat an agent-written draft as ready-to-post upstream
prose. This document governs our companion project, not Bevy itself.

Our larger games and GameKit examples can teach complete application patterns.
An example proposed for Bevy must meet its own [example guidelines](https://bevy.org/learn/contribute/helping-out/creating-examples/),
including avoiding ecosystem-crate dependencies. Extract a small Bevy-only lesson
where useful rather than proposing the whole GameKit-based game as an upstream
example. Keep community examples runnable and show the relevant Bevy versions.

Assess success through time to a first playable change, avoidable user
interventions, reproducible defects resolved, clarity of examples, compatibility
and adoption feedback. Include useful upstream outcomes and reduced downstream
workarounds. Skill count, framework size and contribution volume are not quality
measures. Bevy's [ecosystem guide](https://bevy.org/learn/quick-start/plugin-development/)
also informs small optional dependencies, compatibility documentation and useful
examples. Promotion should follow demonstrated usefulness and the community's
contribution and communication practices.

## Stage 2: documentation ownership before file movement

Use the new documentation workflow to classify every current document as keep,
revise, combine, move or retire. Record its intended reader, authoritative topic,
callers and replacement where applicable. Resolve contradictory facts before
moving files. The inventory should cover root instructions, skill references,
game documentation, crate examples and scripts that read documentation.

JXP's separation of indexes, operational guidance, durable design decisions and
temporary execution material is a useful model. A proposed destination map is:

| Material | Proposed owner/location |
|---|---|
| Repository purpose and getting started | Root README and a short documentation index |
| Working agreements and routing | Concise repository agent instructions |
| Build, test, review, installation and release procedures | `docs/development/` |
| Shared priorities and delivery state | One `docs/planning/roadmap.md` |
| Durable architectural rationale | One design-record collection, with the existing `docs/decisions/` deliberately retained or migrated |
| Game rules, design and verification | Each game's documentation |
| Public API contracts | Crate Rustdoc and tested examples |
| Skill instructions, conditional context and maintenance | Canonical skill sources, focused references and maintainer material |
| Logs, captures and temporary execution notes | Generated work directories; durable evidence links where needed |

Choose the final paths during that refactor. Update skill links, generated
layouts and any machine readers with the documents they reference. Remove
superseded operational prose only after preserving its still-useful decisions
and evidence. Keep unfinished checks distinct from completed milestones.

## Stages 3–6: use real development to refine the framework

The [GameKit consolidation plan](../gamekit-consolidation.md) retains the
capability boundaries and candidate work: shared UI mechanics, bounded
networking migrations and the independent balance harness. Order those slices
by the agreed playable-release needs. A balance harness is not automatically a
prerequisite for the first playable build.

Define Labyrinth's release bar together before selecting feature scope:
intended players, local/multiplayer modes, supported platforms, complete play
loop, onboarding, persistence expectations and distribution channel. Define
Deckbuilder's companion milestone at the same time. Engineering checks and
human judgments of clarity, feel and playability need their own evidence.

Prepare GameKit and GameSkills as a tested version pair. The current decision
is a coordinated initial tag; games may have their own build identities while
recording the library/skill versions used. The current external source probe
does not establish installation from a published artifact. Candidate packaging
must make that difference observable before making a release claim.

Use Port Vila only after the internal refactor and a suitable candidate exist.
Its feedback can improve this repository immediately; adoption remains subject
to that project's review and release process. A delayed adopter PR must not be
reported as a failed library contract or as completed integration.

## Decisions to settle next

1. The core workflow boundaries, first optional packages and real evaluation tasks.
2. The native packaging and compatibility contract demonstrated in both clients,
   including how current installations migrate without losing local ownership.
3. The first playable release audience, platform/mode scope and delivery channel.
4. The documentation ownership map, followed by the exact directory layout.

## References and confidence

The JXP reference is [PR #166](https://github.com/jxp-software/jxp-skills/pull/166),
inspected at `b11265ad9681f54a0e365416ce655d279b53f437`. Its native refactor is
ongoing. The [execution/evidence separation](https://github.com/jxp-software/jxp-skills/blob/b11265ad9681f54a0e365416ce655d279b53f437/docs/development/plugin-execution.md),
[planning-document lifecycle](https://github.com/jxp-software/jxp-skills/blob/b11265ad9681f54a0e365416ce655d279b53f437/docs/planning/README.md)
and [skill provenance](https://github.com/jxp-software/jxp-skills/blob/b11265ad9681f54a0e365416ce655d279b53f437/docs/development/skill-histories.md)
inform this proposal; its full runtime and organizational requirements are not
dependencies of GameSkills.

The [Agent Skills specification](https://agentskills.io/specification) describes
portable entry points and progressive disclosure. [OpenAI's skill guidance](https://learn.chatgpt.com/docs/build-skills)
recommends focused tasks, trigger testing and plugin distribution for reusable
skills. [Anthropic's evaluation guidance](https://platform.claude.com/docs/en/agents-and-tools/agent-skills/best-practices#build-evaluations-first)
supports baselines and iterative evaluation before extensive instructions.
The current [OpenAI plugin format](https://developers.openai.com/plugins/build/plugins)
and [Claude plugin documentation](https://code.claude.com/docs/en/plugins) inform
the proposed adapters; native installation and behavioral parity remain to be
demonstrated by this project.
These are design inputs; our own tasks must establish effectiveness.
