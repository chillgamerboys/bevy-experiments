# GameSkills and GameKit: a companion to Bevy

Status: direction and initial skill catalog approved, September 10, 2026.
The foundation merged in [PR #24](https://github.com/chillgamerboys/bevy-experiments/pull/24),
with up to five workers authorized. Bounded workflow trials and Rust migration
sequencing are proposed next; the documentation layout and playable-release scope
remain future collaborative work.
See [current installation and runtime boundaries](../gameskills.md); this decision
does not claim that the full refactor or a release is complete.

## Agreed direction

Make creating games with Bevy easier through useful packages, development
guidance, games and learning resources. Support Bevy selectively with verified
bug fixes and compelling improvements to its offering as a game engine. This
purpose guides the skills, library, games and release decisions.

The framework assumes Bevy and understands GameKit throughout. GameSkills should
also help an existing Bevy project before it adopts any GameKit crate. GameKit
remains a maintained set of opt-in capabilities using Bevy's own concepts and
extension points. Convenient packages such as a simple UI tooltip system are
valuable GameKit offerings in their own right. Bevy is our established engine
foundation and continues to evolve; an additional extension does not by itself
demonstrate a missing engine capability. Learning resources and library examples
should be usable by developers without an agent installed.

Implement and use GameSkills before the broad documentation and code refactors,
so subsequent work develops under the workflows we intend to ship. Labyrinth's
playable release is the primary product milestone. Develop Deckbuilder alongside
it to test whether shared capabilities and guidance transfer between games.
Carterfight retains its small offline-consumer role.

GameSkills supplies development guidance and supporting tools. GameKit supplies
reusable implementations. The games supply real requirements, examples and
evidence. Most improvements belong in the games, GameKit, our documentation or
the skills. Credible engine defects and significant engine-level gaps can justify
upstream investigation, with a compelling rationale and human review before any
submission. An upstream bug fix may retire a local workaround while GameKit
continues to provide the convenient package built on that engine capability.

The owner controls this repository and its priorities. The existing Bevy Hex Game
is a later adoption case with other contributors: its changes need separate PRs
for their review. Its integration must not become a prerequisite for starting
our internal refactor. See the deferred [Port Vila pilot](port-vila-adoption.md).

## Development order

| Stage | Work | Evidence needed to move on |
|---|---|---|
| 1. Working skills foundation | Merged: core and optional offerings, support tooling and a pinned candidate adopted in this repository | Helper checks and Codex planning evidence exist; full delivery trials and Claude behavioral validation remain open |
| 1a. Bounded workflow refinement (proposed) | CI scoping, one small docs cleanup and one shared UI change through the installed skills | Actual delivery and scoped checks; concrete workflow findings resolved; see the [trial and CI plan](gameskills-ci-scope.md) |
| 1b. Rust CLI follow-up | Freeze the refined contracts and replace the initial Python helpers with independent Rust development tooling | Language-neutral behavior checks, native client invocation and an adopter update work without Python; see the [migration plan](gameskills-rust-cli.md) |
| 2. Documentation refactor | Use those skills to inventory, reconcile and restructure the documentation | One maintained owner per topic; working links and references; current commands and status; historical decisions remain recoverable |
| 3. GameKit and game refinement | Use the skills for bounded changes across Labyrinth and Deckbuilder | Both consumers retain their own rules and presentation; affected contracts and user paths are verified |
| 4. Playable candidate and packaging | Complete the agreed Labyrinth play loop, develop Deckbuilder alongside it, and prepare reproducible game/library/skill artifacts | Fresh game builds can be installed and played; library and skills can be consumed from the actual candidate artifacts |
| 5. Existing-game pilot | Revisit Port Vila with an immutable candidate and propose one useful UI integration | Reviewed adopter PRs establish installation, coexistence, runtime integration and a later update |
| 6. Supported releases | Resolve findings and publish the agreed support and compatibility commitments | Evidence matches the release artifacts and declared platforms; limitations, migration and rollback are documented |

Each stage should improve the skills when real work demonstrates a gap. A defect
in one workflow can be corrected without reopening the entire framework design.
Port Vila verifies existing-repository adoption; it is not the only evidence for
the games or an automatic blocker for a private Labyrinth playtest build.
Investigate credible engine defects when encountered. Ordinary feature work has
no upstreaming requirement, and GameKit releases do not depend on Bevy accepting
our additions.

## Stage 1: redesign the offerings before migrating the skills

The seven [existing craft skills](../../skills/README.md) are research inputs,
not a fixed inventory to preserve. Assess every instruction, reference and tool
against real development tasks. Keep useful principles and regression cases;
split, combine, move or retire the current entry points as the new design needs.

### Initial core development offering

The core should help a developer go from a game idea or existing project to a
working, understandable and releasable game. The [initial catalog and
pipeline](gameskills-catalog.md) defines 12 core skills and 9 optional skills,
their responsibilities and the shared execution contracts. These approved
boundaries provide the starting point for validation through real tasks.

`gameskills:plan` is the default entry. It routes bounded work to the current
agent and authorized independent work to `dispatch`. The toolbox covers debugging,
testing, playtesting, review, documentation, PR creation/audit/merge and release;
`setup` handles adoption and deliberate installation updates. `dispatch --inject`
adds verified work to an active queue using the same ownership contract.

Routine implementation does not need a skill that merely repeats ordinary agent
instructions. Bevy-specific craft guidance belongs in focused references loaded
by the relevant workflow: ECS ownership, schedules, states, plugin composition,
assets, compatibility and GameKit integration. Setup and readiness checks should
be deterministic commands where possible, with a skill only where interpretation
adds value.

### Levels of creative involvement

Planning and playtesting use the same levels so the developer can control the
kind of creative input sought. Levels apply to a task or bounded feature, with a
project default for otherwise open-ended work. They describe creative latitude;
engineering rigor and honest evidence apply at every level.

| Level | Creative responsibility | Example |
|---|---|---|
| 1. Implement | Carry out the established design, resolve engineering details and surface contradictions or defects | Implement the specified tooltip behavior while preserving its content and interaction design |
| 2. Refine | Critique and improve clarity, feedback, usability and tuning within the established player experience and task scope | Improve tooltip readability and placement; identify confusing explanations without changing the underlying rules |
| 3. Co-design | Develop alternatives and tradeoffs for a bounded feature or system, agree on its direction and prototype or implement within the authorized scope | Work together on a targeting interaction or deck-building decision, using player goals and playtest evidence |
| 4. Explore | Investigate new concepts or major alternatives through explicitly scoped experiments with time/content limits and evaluation criteria | Prototype alternative core loops in an experiment and compare what players understand and enjoy |

Recommend level 2 as the default for Labyrinth and Deckbuilder refinement, level
1 for precise fixes or already specified behavior, level 3 for collaborative
feature design, and level 4 for requested exploratory work. These are accepted
starting points that task-specific instructions can change. Higher levels do not imply higher quality or require
passing through lower levels first.

Use the developer's task instructions and existing decisions before the project
default. Preserve the selected scope across turns; do not ask for a level on every
request. Routine engineering decisions and improvements already authorized by the
task should continue without extra confirmation. A request to propose a redesign
authorizes design work; implementation follows the scope the developer has given.
The level itself grants no additional authority to change product direction,
publish work or make an upstream submission.

For playtesting, level 1 checks the specified experience, level 2 recommends
improvements within it, level 3 compares feature alternatives, and level 4 tests
new concepts. Record observations separately from design hypotheses. Automated
checks and agent interaction do not establish human enjoyment or replace feedback
from the intended players. Evaluate the levels using the same task at different
settings: the creative output should change while engineering quality and respect
for the brief remain consistent.

### Proposed optional packages

Optional packages follow game capabilities or specialist work. Each can add
focused skills, references, review criteria and verification commands to the same
core workflows. Avoid separate copies of planning, testing and PR delivery per genre.

| Package area | Initial purpose |
|---|---|
| UI and interaction | Bevy UI and GameKit UI integration, focus/input/modal ownership, responsive layout, visual and native interaction verification |
| Turn-based rules | Legal actions, state transitions, deterministic replay, stable identity and game-owned rules |
| Multiplayer | Authority, disclosure, sessions, admission, reconnect and real transport verification |
| Framework maintenance | GameKit API design and compatibility; skill authoring, evaluations, packaging and migrations for maintainers of this framework |
| Bevy contributions, when needed | Prepare evidence for verified engine bugs or compelling engine-level gaps, with human review and Bevy's contribution requirements |

Core debugging and review can recognize a credible engine problem. Contribution
preparation is an optional workflow used when that problem warrants pursuit. It
does not run as a routine stage of each game or GameKit change, and its package
need not ship before the initial core and UI candidate is useful.

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
The [catalog investigation](gameskills-catalog.md#jxp-investigation-and-provenance)
also examines the later PR head, legacy plan/dispatch/inject contracts, measured
token costs and the explicit legacy-rule migration ledger.

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

## GameKit's role and selective contributions to Bevy

GameKit should make useful Bevy capabilities easy to adopt: small packages,
sensible defaults, clear examples and tested integration. A simple tooltip system
can remain in GameKit indefinitely. Its reuse across several games validates its
value as a package; upstream suitability needs a separate engine-level reason.

For a development finding, identify the appropriate owner:

| Finding | Likely destination |
|---|---|
| A game's rules, presentation or content choice | The game |
| Convenient reusable mechanics, defaults, composition or an extension | GameKit or an existing ecosystem project, after checking what already exists |
| A reproducible defect in expected engine behavior | Bevy bug investigation, followed by human review of a focused contribution |
| A significant missing engine capability | Investigate the engine-level need and existing options; human review determines whether to pursue an upstream proposal |
| A discoverability or learning gap | Our documentation, focused examples or a community learning resource; an incorrect Bevy API contract can instead be investigated as a documentation defect |
| An agent made a poor decision or reported unsupported evidence | The relevant skill/tool, verified against the actual failure |

Bevy's [upstreaming guidance](https://bevy.org/learn/contribute/project-information/upstreaming/)
considers engine expectations, usefulness, quality and maintenance cost. Popularity
or the absence of an existing solution is insufficient on its own. Our focus is:

- **Verified bugs:** establish the expected and actual engine behavior, a minimal
  reproduction, affected versions and a regression check where appropriate. A
  real bug does not need to affect several of our games to justify investigation.
- **Key engine gaps:** demonstrate a real game-development need and why it belongs
  in the engine's offering. Inspect existing APIs, plugins and upstream plans;
  explain why current options fall short, who benefits, and the integration and
  maintenance tradeoffs. Convenience, reuse or novelty alone is insufficient.

Before an upstream issue, proposal or PR, a human must review the evidence and
engine-level rationale, and the design and code for any proposed implementation.
Follow existing authorization and the receiving project's review process. The
skills can surface a candidate and support investigation; they cannot declare
their own work human-reviewed or treat review as satisfied by a general request
to help Bevy. Acceptance remains with Bevy's maintainers.

For example, if our tooltip implementation encounters a reproducible Bevy input
defect, isolate and consider contributing a fix for that defect. The tooltip's
convenient API, defaults and presentation remain a GameKit package. This is an
illustration of the boundary, not a claim that such a Bevy defect has been found.

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

Our larger games and GameKit examples teach complete application patterns and
remain community resources. If a reviewed upstream bug fix or engine improvement
calls for an example, follow Bevy's [example guidelines](https://bevy.org/learn/contribute/helping-out/creating-examples/),
including avoiding ecosystem-crate dependencies. Keep our own community examples
runnable and show the relevant Bevy versions.

Assess success through time to a first playable change, avoidable user
interventions, reproducible defects resolved, clarity of examples, compatibility
and adoption feedback. Where upstream work is justified, record the engine problem
resolved and any retired workaround. Set no upstream contribution quota or target
to transfer GameKit's features into Bevy. Bevy's [ecosystem guide](https://bevy.org/learn/quick-start/plugin-development/)
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

## Decisions settled and still ahead

The initial offering is 12 core skills, with nine optional skills across five
packages. `plan` is the default entry, level 2 is the creative default, and the
initial dispatch cap is five workers. The owner authorized implementation.

Native compatibility and workflow effectiveness need actual candidate evidence.
The first playable release audience, platform/mode scope and delivery channel,
and the documentation ownership map/layout remain later decisions. They do not
block implementation of this foundation.

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
