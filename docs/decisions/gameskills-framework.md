# GameSkills and GameKit development sequence

Status: draft for discussion, September 10, 2026. The priorities below are agreed;
the milestone boundaries, skill inventory and documentation layout are proposals.
This document does not claim that the framework refactor or a release is complete.

## Agreed direction

Implement and use GameSkills before the broad documentation and code refactors,
so subsequent work develops under the workflows we intend to ship. Labyrinth's
playable release is the primary product milestone. Develop Deckbuilder alongside
it to test whether shared capabilities and guidance transfer between games.
Carterfight retains its small offline-consumer role.

GameKit supplies opt-in library capabilities. GameSkills supplies development
guidance and supporting tools. The games supply real requirements and evidence.
An observed failure may require a game fix, a clearer GameKit API, a tool change,
or a skill correction; do not automatically turn every failure into another rule.

The owner controls this repository and its priorities. The existing Bevy Hex Game
is a later adoption case with other contributors: its changes need separate PRs
for their review. Its integration must not become a prerequisite for starting
our internal refactor. See the deferred [Port Vila pilot](port-vila-adoption.md).

## Proposed order

| Stage | Work | Evidence needed to move on |
|---|---|---|
| 1. Working skills foundation | Evolve the existing pack, add the missing development workflows, and adopt a pinned candidate in this repository | Real tasks demonstrate useful selection, correct execution, accurate evidence and complete delivery; installation and source ownership are explicit |
| 2. Documentation refactor | Use those skills to inventory, reconcile and restructure the documentation | One maintained owner per topic; working links and references; current commands and status; historical decisions remain recoverable |
| 3. GameKit and game refinement | Use the skills for bounded changes across Labyrinth and Deckbuilder | Both consumers retain their own rules and presentation; affected contracts and user paths are verified |
| 4. Playable candidate and packaging | Complete the agreed Labyrinth play loop, develop Deckbuilder alongside it, and prepare reproducible game/library/skill artifacts | Fresh game builds can be installed and played; library and skills can be consumed from the actual candidate artifacts |
| 5. Existing-game pilot | Revisit Port Vila with an immutable candidate and propose one useful UI integration | Reviewed adopter PRs establish installation, coexistence, runtime integration and a later update |
| 6. Supported releases | Resolve findings and publish the agreed support and compatibility commitments | Evidence matches the release artifacts and declared platforms; limitations, migration and rollback are documented |

Each stage should improve the skills when real work demonstrates a gap. A defect
in one workflow can be corrected without reopening the entire framework design.
Port Vila verifies existing-repository adoption; it is not the only evidence for
the games or an automatic blocker for a private Labyrinth playtest build.

## Stage 1: enough skills to guide the next stages

Start from the seven [existing craft skills](../../skills/README.md), their
references and install/sync tools. Establish a baseline before changing them.
The first milestone needs the following outcomes; the exact skill names and
whether some outcomes share an entry point remain open:

- Plan a scoped change using actual code, contracts, user priorities and an
  explicit definition of done. Preserve decisions and authorization across turns.
- Implement and review Bevy/GameKit changes using the existing architecture,
  rules, UI, testing and debugging skills, refining their coverage as needed.
- Maintain documentation: identify the owner of a fact, reconcile stale claims,
  update references and preserve the rationale behind decisions.
- Deliver the requested change through commits and a reviewable PR, report CI
  accurately, and perform merge/release actions within the user's authorization.
- Improve GameSkills from observed failures, with reproducible examples and
  behavioral evaluation before promoting a new candidate.

Keep the first implementation proportionate. Instructions own judgment and
routing. Existing commands and small deterministic helpers own repeatable checks
and observations. Project configuration supplies commands, target branches and
capability-specific requirements. Issue trackers, cloud services and multiplayer
checks apply when the project uses them.

The initial bootstrap may use the current tooling and this agreed direction.
After a candidate is ready, use its installed workflows for the next changes.
Distinguish canonical source from installed/generated files and record the
candidate revision used. Explicitly update the installation when testing a new
candidate; editing source must not silently change what a consumer is running.

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
and a documentation update carried through a PR. Reuse historical failure
scenarios as regression cases while retaining independent tasks for evaluation.
The [evidence reference](../../skills/references/evidence.md) remains the current
contract until deliberately revised.

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

1. The minimum first skill inventory and the real tasks used to evaluate it.
2. Whether the first distribution candidate extends the current generated pack
   or introduces native plugin packaging; preserve consumer ownership either way.
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
These are design inputs; our own tasks must establish effectiveness.
