# GameSkills catalog

The [workflow guide](workflow.md) describes daily use. This page identifies current
skill responsibilities; plugin source and native manifests own the installed inventory.

## Current catalog

The default installation contains **13 core skills**. Six optional packages offer
eleven additional skills, selected explicitly for the project. Each skill has a focused
body with conditional references; installing 24 skills does not mean loading 24
bodies on every task. Names below are logical names. Native invocation syntax and
package metadata must be verified independently in Codex and Claude.

### Core package: gameskills

| Skill | Trigger and responsibility | Result / handoff |
|---|---|---|
| `setup` | Adopt GameSkills, configure a project or deliberately update its installation; inspect existing instructions and supported Bevy/GameKit versions | Selected packages, project commands, compatibility and pinned installation; local ownership preserved |
| `plan` | Start or substantially revise a task; investigate once, define creative scope, decisions, ownership and acceptance | A bounded plan, or work orders and a dispatch queue when parallel execution is authorized |
| `grill` | Resolve material ambiguity through read-only research and small question rounds | Accepted, delegated and deferred decisions handed to plan; clear tasks skip interviews |
| `dispatch` | Execute an established wave of independent work; own worker lifecycle, resources, returned results and integration | Completed/blocked stream state, reviewed PRs and verified integration within authorization; `--inject` adds justified work |
| `debug` | Investigate an observed failure or unexplained behavior | Reproduction, causal explanation, proposed correction and regression evidence; unresolved investigation remains explicit |
| `test` | Establish engineering behavior for a change or artifact | Appropriate command graph and actual results for pure, app, runtime or performance checks; selected packages supply specialist cases |
| `playtest` | Assess a player journey or compare game-design hypotheses | Observations, player feedback and prioritized improvements using the task's creative level; human enjoyment is not inferred from passing tests |
| `review` | Evaluate a design or actual change, including code before a PR exists | Actionable findings from relevant Bevy/package guidance plus an open-ended inspection; fixes return to the authorized implementer |
| `update-docs` | Create or reconcile explanations, examples, API guidance and migration documentation | Updated authoritative material and checked references; completed plans are retired after updating current guides and Decisions sections |
| `create-pr` | Deliver an implemented change for review | Logical commits, pushed source, correct base, concrete PR body and verified PR identity; update an existing PR when appropriate |
| `audit-pr` | Determine whether a specific PR satisfies its planned acceptance and project requirements | An evidence record tied to repository, PR, HEAD, base and relevant inputs; coordinates review, tests, documentation checks and applicable playtests |
| `merge-pr` | Integrate a reviewed change when the session and receiving project authorize it | Revalidated source/CI/review state, observed merge result and appropriate integration checks; incomplete steps stay visible |
| `release` | Prepare or publish a game, library or skill release within the requested scope | Verified distributable artifacts, compatibility, version identities, release information and observed publication when authorized |

`review` owns judgment about the work. `audit-pr` owns the complete acceptance
record for a particular PR. `create-pr` prepares the review artifact; it cannot
declare that artifact accepted. `merge-pr` checks the current state before
integration. `release` concerns actual distributable artifacts and support
commitments. These boundaries allow each task to be requested independently.

Implementation is an agent responsibility supported by Bevy craft references and
the selected specialist skills. There is no additional generic `implement` skill.
Readiness, queue validation, command execution and evidence recording are helpers,
not extra skill bodies. `inject` is a mode of `dispatch`, sharing its queue and
ownership model, rather than a second implementation of coordination.

### Optional packages

| Package | Skill | Responsibility |
|---|---|---|
| `gameskills-ui` | `build-ui` | Design and implement Bevy/GameKit UI within the chosen creative scope; handle view/intent boundaries, layout, focus, input, tooltips and modal lifecycle |
| `gameskills-ui` | `verify-ui` | Inspect rendered layout and native interactions, including focus, hidden state, input methods and accessibility expectations |
| `gameskills-turn-based` | `model-rules` | Model game-owned actions and transitions, legality, stable identity, deterministic replay and adapters for tests/simulation |
| `gameskills-multiplayer` | `design-multiplayer` | Define authority, visibility, protocol/session boundaries, admission and reconnect behavior from the game's requirements |
| `gameskills-multiplayer` | `verify-multiplayer` | Verify required process/transport scenarios, failures, disclosure and lifecycle; record the actual machines and network used |
| `gameskills-maintainer` | `evolve-gamekit` | Design or refine a reusable GameKit capability, its contract and consumer migration using demonstrated game needs |
| `gameskills-maintainer` | `author-skill` | Create or revise a canonical skill, its triggers, focused references and deterministic helpers from observed needs |
| `gameskills-maintainer` | `evaluate-skills` | Compare behavior, quality, installation and cost across candidates and supported agents; recommend or reject promotion |
| `gameskills-bevy-contrib` | `prepare-contribution` | Investigate a verified engine bug or compelling engine-level gap and prepare technical evidence for human review under Bevy's contribution policy |

`gameskills-linear` adds optional `track` and `cleanup` skills. It owns exact
project routing and two-way PR links through connected tools or an optional
standalone helper. Deletion assessment is available, while implementation is deferred. Core-only adoption needs no Linear account.

Each package can also contribute references and check recipes used by core
planning, testing and review. A networking change selects the multiplayer
guidance in those workflows; it does not create another planning or PR pipeline.
Package-specific verification can be a step in the core test/audit graph, avoiding
duplicate command execution. Core playtesting owns the overall player journey;
UI verification owns the particular rendering and interaction evidence.

Initial adoption can use core plus UI, followed by the rules and multiplayer
packages as their tasks arise. Maintainer skills serve this repository's own
refactor. The contribution package is optional and can follow later; no ordinary
game or GameKit change needs an upstream proposal.

Simulation/balance, content pipelines, procedural worlds and platform-specific
publishing remain later candidates. They are outside this current
catalog; concrete skills and support promises need real tasks first. Version
migrations, ordinary profiling and project architecture are covered by the core
and its references rather than separate skills for every Bevy subsystem.


## Decisions

`plan` is the default implementation entrypoint; direct toolbox requests keep their
narrower scope. `grill` is selective planning help, not a mandatory interview. Core
has no Linear or Gamekit requirement. Add optional packages deliberately for the work.
Creative levels change creative involvement, not evidence rigor or authorization.
Configured worker capacity does not itself authorize launching agents.
