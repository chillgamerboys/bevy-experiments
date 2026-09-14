# Development rigor and milestone promotion

Status: active
Phase: implementation merged; branch and installation rollout pending
Owner: GameSkills, with repository CI and game test-suite owners.
Implementation: PR #40 merged into `main` as `ebc77b73584f3e1f2a6cefcd036fcffb985ddd1a`.
Remaining endpoint: branch/default and installation rollout described below.
Tracking: [HEX-110](https://linear.app/chillgamerboys/issue/HEX-110/add-scoped-development-testing-and-release-rigor-to-gameskills).
Branch/default changes follow accepted integration and observed Development routing.
Development and Testing probes passed on the accepted implementation; a real
`dev` receiving branch and current-session candidate activation remain unobserved.

## Outcome and accepted decisions

Reduce early-development latency and agent tokens while retaining useful correctness
checks. Rigor controls verification depth; affected behavior controls its scope.
Creative involvement remains an independent choice.

| Level | Project use | Required coverage |
|---|---|---|
| Development | Feature PRs to `dev` | macOS, affected owner tests and relevant build/lint; changed UI flows at 1920×1080 Auto when warranted |
| Testing | Batch milestone PRs to `main` | macOS, affected regression/integration coverage across the combined batch, developer gameplay sanity for affected game behavior |
| Release | Explicit release preparation | macOS/Windows/Linux as applicable to supported capabilities, actual distributables, compatibility and explicitly supported displays |

The developer selected Windows/Linux at Release only, batch promotions, and a manual
sanity gate only at milestone promotion. Development sanity checks are optional and
do not hold ordinary PRs open. Narrative docs or tooling-only batches do not require
playing a game; shared changes with game effects must account for those consumers.

Logic-only fixes do not trigger agent screenshots, UI walkthroughs or resolution
sweeps. UI presentation/interaction changes receive relevant state/input checks and
inspection of the changed flow at normal 1080p. End-to-end checks follow affected
journeys and boundaries, not every game edit. Preserve existing tests, including
compatibility cases; select when they run rather than deleting coverage.

Adding a test, running an existing test and manually inspecting a game are separate
decisions. Do not add redundant tests or harnesses for small reversible changes.
Stop verification when applicable checks pass unless new changes, failures or
unresolved concerns justify more work. Coverage outside the selected scope is not
unfinished acceptance on every development task.

## Investigated starting point

Planning inspected source `1c6ff8b61e062a857f0d49608053778b66f211d1`, whose tree
matches merged main `cadbf5eb389da5688dd95fbbb8ba6ad93c9ea8fe`. Reobserve current
source/base when implementation begins; neither hash is an instruction to reset.

- `gameskills.toml` has creative levels and named command graphs, but no rigor policy.
  The CLI validates configuration in `gameskills/cli/src/config.rs`. Existing command
  dependencies can execute an expensive suite even when only a small command is selected.
- `gameskills/cli/src/delivery.rs` stores the receiving base and check list, defaults
  the base to `main`, and does not attest to human review. Workflow plans/orders and
  runner evidence also need to carry the resolved verification scope.
- `devtools/src/ci/{selector,driver,checks,mod}.rs` already selects changed packages
  and reverse consumers. Unknown or CI inputs broaden to all packages. The workflow
  has fixed three-OS matrices; policy checks run on Ubuntu.
- `checks.rs` runs unfiltered all-feature package tests before the explicit
  six-process test. Dropping only that explicit test still runs embedded UI/socket
  suites. `--lib` alone does not make those tests focused.
- Individual UI tests contain multiple viewport/scale cases, including in
  `games/labyrinth/src/ui/tests.rs` and `ui/tests/overlay_stability.rs`. Filtering the
  test name alone cannot avoid its embedded 720p/4K/200% work.
- Current owner guides request broad UI coverage despite portable guidance favoring
  relevant tests. Both instruction selection and actual execution need correction.
- Many local commands bind `refs/remotes/origin/main`. A branch transition must align
  receiving-base resolution and evidence inputs without rewriting historical records.

## Implementation contract

### One policy, separate scope selection

Add optional project-owned verification configuration with named levels
`development`, `testing`, and `release`, a default level, branch-to-level mappings,
platform lists, normal UI target, and milestone manual-sanity policy. Add a configured
default delivery base, independent of the current feature branch. This project opts
into Development and `dev`; projects without the new configuration preserve their
existing behavior. Do not silently change another adopter's branch or rigor.

Implement one deterministic policy resolver in the GameSkills CLI. Provide a read-only
JSON command, proposed as `gameskills verification resolve --base BRANCH [--level LEVEL]`,
which works without installing skills or launching an agent. It reports the effective
level, receiving branch, platforms, display target, manual-gate timing and reasons.
The exact CLI/config spelling can follow existing conventions during implementation;
the behavioral contract above is fixed. Rigor is distinct from Cargo's `--profile ci`.

The project adapter owns package/path/journey-to-command selection. CI consumes the
resolver's versioned policy output and combines it with the existing committed-input
selector; do not duplicate level defaults in YAML and skill prose or add a dependency
from GameSkills to this repository's tools/games. Keep `repo-devtools` independently
buildable by exchanging structured policy data rather than importing CLI internals.

An explicit level may request more verification. Receiving-branch acceptance cannot
be weakened by a lower local selection. Selecting Release never authorizes publication.
Unknown impact broadens investigation or affected component coverage within the
selected level; it must not silently add Windows/Linux, every display, or every E2E
journey. Distinguish full affected scope from Release rigor in the selection model.

### Selection that changes actual execution

Use existing test modules to define small named suites. Start with rules/content,
session/persistence, UI behavior, transport/admission, process recovery, and display
compatibility where those boundaries exist. Keep mappings local; do not build a
general automatic Rust call-graph or test-impact analysis engine.

| Example change | Development evidence |
|---|---|
| Damage, status or turn rule | Relevant pure owner tests and affected game compile check; no UI walk |
| Save/load logic | Serialization, roundtrip and failure tests; relevant integration when the boundary changes |
| Admission retry | Focused real host/guest admission recovery; process-death test only when that behavior is affected |
| Editor/input/help behavior | Relevant UI state/input tests and changed route at 1080p Auto |
| Shared capability | Relevant capability contract and affected consumer integration |
| Narrative documentation | Local documentation checks |

Use positive test selection or explicit suite commands. Where a single test loops
over normal and compatibility cases, split those cases into separately selectable
tests without dropping assertions. Check that required selections execute tests;
Cargo's zero-match success must not count as suite coverage. Exclusions cannot hide
new tests indefinitely: unmapped changed tests need classification or a bounded
broader owner check. Scope by behavior, not solely by filename; a logical change to
UI projection or input routing can require relevant automated UI behavior coverage
without a visual sweep.

Testing evaluates the combined `dev` versus `main` batch and widens relevant regression
coverage. Release adds supported platform/display/artifact checks; it still does not
select unrelated player journeys. Exact release display commitments remain future
release scope. Existing unsupported Windows runner operations remain unsupported;
portable Windows tests do not establish a new process-supervision backend.

### Durable scope and manual acceptance

Carry the effective level, actual receiving branch, policy identity, affected scope,
selected commands and reasons into plan/orders, delivery and runner observations.
Keep command execution results distinct from selection and human observations.
Preserve old records as historical evidence; do not relabel them with a new level or
weaken existing source/configuration/executable validation to avoid a rerun.

For milestone batches affecting game behavior, the audit requires the developer's
explicit sanity result tied to the candidate and relevant journey. Record the
source and actual response/reference in the promotion record. CI success, an agent
walk, or previous feature acceptance is not that response. Changed relevant game
inputs invalidate its applicability; unchanged coverage may be carried forward with
the reason recorded. Do not add a general human-approval service or pretend an
agent-supplied record independently authenticates a person.

Ordinary dev delivery has no manual gameplay gate. Automated CI reports its own
result separately from the milestone's pending/completed manual acceptance.

## Work sequence and owners

1. **Core configuration and policy resolution — GameSkills CLI.** Extend
   `config.rs`, `cli.rs`, a focused verification module, and relevant help/tests.
   Implement backward compatibility, branch/default resolution, validation and JSON
   output. Extend delivery/workflow/runner records only as needed to preserve and
   validate the selected policy. Tests cover old configs/records, invalid values,
   branch requirements, changed policy and correct receiving-base selection.
2. **Focused suite execution — game tests and repository tools.** Inventory current
   test groups and command prerequisites, split compound compatibility cases, and
   add explicit suite mappings/commands. Preserve production behavior and all retained
   regressions. Verify a logic-only selection, a UI selection and a network selection
   execute their intended tests without unrelated work.
3. **CI routing and gates — repository tools.** Extend selection/event/result contracts
   and `gamekit.yml` to consume the resolved policy and use dynamic platform matrices.
   Run Development/Testing build and test jobs on macOS, including the classification
   and aggregate-gate bootstrap programs as well as skills, Rust and policy checks.
   Do not retain an incidental Linux tooling build through a hardcoded runner.
   PRs to `dev` select Development; milestone PRs to `main` select Testing. Explicit
   Release dispatch selects release coverage. Unmapped bases use explicit project
   defaults and report them. Retain a stable aggregate check that rejects missing,
   failed, cancelled or unexpectedly skipped selected jobs/suites. Main push checks
   use Testing; post-merge selection must not accidentally escalate to Release.
4. **Skill and adopter integration — GameSkills maintainers.** Update existing plan,
   test, audit, delivery, UI/multiplayer verification and release guidance with one
   concise conditional rigor reference, keeping package references self-contained.
   Update creative-level guidance only to link the independent dimension. Reconcile
   root/GameSkills/CI/game testing docs and `gameskills.toml`; remove contradictory
   universal matrix instructions. Set runtime compatibility explicitly, commit
   canonical changes, then regenerate the instruction bundle from that committed
   source and inspect its Cargo package. Preserve installed caches, overlays, prior
   pin records and historical evidence; test the candidate in an isolated consumer.
   Plan a supported, explicit project installation update for the compatible CLI and
   bundle after candidate validation. Do not edit cache files or treat candidate
   source as the instructions already loaded into a running agent session.
5. **Observe delivery and transition branches — integration owner.** Deliver the
   implementation through the existing receiving branch and inspect its actual CI
   before changing defaults. Adopt the validated compatible CLI/bundle through the
   supported project update path and verify its pin and actual native readiness;
   a running session may need restart before it exposes the revised instructions.
   Create `dev` from the accepted integrated revision and establish its intended
   checks. Observe a successful real Development workflow on `dev` before making it
   the repository default. Use genuine follow-up tooling work or an explicit scoped
   workflow probe that exercises the intended jobs; a docs-only skip does not prove
   the macOS build/test route. Then change the default and reconcile new workspace/task
   base guidance and existing open PR bases deliberately, preserving current workspace
   branch names. Observe one Testing promotion workflow, using a genuine batch where
   possible instead of manufacturing game changes. Bind feature tasks to `dev` and
   milestone tasks to `main`. If the switch is not healthy, restore the prior default/settings
   without deleting branches or rewriting commits, and retain the concrete result.

Core resolution and test-suite inventory can proceed independently after the contract
is agreed. Integrate shared config/CLI/CI edits serially; coordinate Cargo resources.
This plan creates no dispatch queue or worker execution orders.

## Bounded acceptance and verification

- Configuration/resolver fixtures establish profile defaults, independent creative
  levels, legacy behavior, branch requirements and malformed-input rejection.
- Existing `devtools/tests/ci_routing.rs`, `ci_checks.rs`, and `ci_cli.rs` cover
  docs, logic, UI, networking, shared consumers, rename/delete and unknown inputs;
  receiving branches; Release dispatch; selected commands/platforms; and final-gate
  failures. Unknown/CI inputs do not promote Development/Testing into Release.
- Delivery/workflow/runner fixtures establish scope persistence, actual-base evidence,
  old-record preservation and no automatic claim of developer sanity. Milestone
  audit behavior is exercised with missing, applicable and stale manual observations.
- Run focused CLI and CI-controller tests, relevant suite smoke checks, formatting,
  lint, docs and skill validation on macOS. This is tooling/test selection work; it
  does not itself require a gameplay UI tour or a Windows/Linux acceptance run.
- Exercise a small candidate skill sample: a logic-only fix selects no UI walk,
  a UI change selects its 1080p flow, and a milestone retains its human sanity gate.
  Use real observations; static rubric success cannot prove agent compliance or token
  savings. Record available elapsed time/job counts without a broad benchmark project.
- Validate the changed candidate bundle/package once and preserve its actual identity.
  Reuse applicable results rather than rerunning them at every lifecycle skill.
- Actual GitHub results must demonstrate the intended Development and Testing jobs;
  routing fixtures alone do not establish deployment of the policy. Release routing
  fixtures establish command selection, not a completed cross-platform release.

Applicable existing required checks remain observable during bootstrap. Change the
workflow intentionally as part of the reviewed implementation; do not bypass existing
requirements or launch broad reruns solely to prepare this plan.

## Completion and deferred work

Implementation is complete when the profiles, selective execution, skill guidance,
adopter configuration and actual branch workflows agree, the developer sanity gate
is preserved for relevant promotions, and the authorized delivery endpoint is observed.
Move implemented contracts into current owner guides, preserve genuinely outstanding
work, then retire this plan and repair its index link.

Deferred: release publication, exact extra display commitments, Windows process
backend support, a general test-impact engine, CI cache redesign, broad model-cost
benchmarks, and changes to linked adopter repositories. None is an automatic gate
for introducing this development policy.
