# Implemented CI scope and bounded GameSkills trials

Status: refinement sequence accepted after the foundation merged in
[PR #24](https://github.com/chillgamerboys/bevy-experiments/pull/24). CI routing is
implemented in [scripts/ci.py](../../scripts/ci.py); the owner accepted the trial
results and authorized their merge on September 10, 2026.
[Testing guidance](../testing.md#ci-selection) owns current commands. The rationale
and acceptance criteria below guide the bounded trials
before the [repository-wide Rust tooling migration](gameskills-rust-cli.md), which
will also replace the current Python classifier and repository checks.

## Problem and intended result

The foundation's workflow ran on every PR and push to `main`. Every change received
the three-OS Python and full Rust/game test
matrix, external consumer checks, Labyrinth's six-process restart test, and an
Ubuntu policy job covering formatting, Clippy, dependencies and capability graphs.
Even narrative documentation changes received all of those checks.

The final foundation [CI run](https://github.com/chillgamerboys/bevy-experiments/actions/runs/34541177767)
used approximately 4m23s on Linux, 3m45s on macOS, 7m50s on Windows and 1m17s for
policy: about 17m15s of combined job time. These are observed job durations for one
run, not a billing estimate or a demonstrated saving from the proposed changes.

Select checks from the changed inputs and their consumers. Narrative docs should
get relevant documentation checks. Game changes should test the affected game;
they should not automatically run GameSkills runtime tests or other games. Shared
inputs need broader coverage, and uncertain impact must fall back to the full suite.

## Proposed routing

Always run a lightweight change classifier, repository/link checks and one stable
final `ci` result. Split the existing matrix into independently selected jobs.
Keep the existing OS coverage for a selected component until evidence justifies a
separate platform decision. Mixed changes take the union of their checks.

| Changed inputs | Selected verification |
|---|---|
| Narrative docs and prose-only README files | Repository layout and local links; relevant documentation/example checks. No Rust/game build when those files are not build or test inputs. |
| Skill bodies, references, routing guidance or evaluation fixtures | Skill structure, links and catalog consistency, plus focused behavioral evaluation of changed guidance. Markdown that instructs agents is a behavioral input. |
| GameSkills executable, installer, native metadata, configuration, lock or tooling tests | Relevant packaging, configuration, queue, runner and native adapter tests across the supported OS matrix; actual client evaluation when loading or behavior changes. No game builds without a shared input change. |
| Code, assets or executable examples owned by one game | That game's affected build, tests and examples; appropriate smoke/playtest evidence for changed assets or interaction. Other games stay unselected unless they consume the changed input. |
| A GameKit capability or the facade | Affected library tests, doctests and Clippy; reverse dependency consumers; applicable minimal-feature, WASM and external distribution contracts. |
| Labyrinth networking or shared session, discovery or multiplayer behavior | Relevant game/library checks and the real six-process restart test. Initially retain this test for all Labyrinth or transitive library code changes; narrow further only after explicit input mapping. |
| Root Cargo manifest/lock, toolchain, `.cargo`, dependency policy, CI workflow/classifier or unknown ownership | Full relevant matrix and policy checks. Classifier or dependency-graph failures also select the full suite. |

Keep formatting cheap and tied to Rust/fixture changes. Scope Clippy to affected
packages and targets. Dependency/license checks follow manifests, the lockfile
and policy changes, with full validation before releases. A game asset can also
be compiled via `include_bytes!`; assets cannot be classified as harmless prose.
Documentation included by Rustdoc or an executable test remains a build input.

Native model evaluations can consume paid usage and need working authentication.
Define bounded evaluation commands and their required scope for the change;
report unavailable checks explicitly. Routine narrative docs should neither invoke
models nor install both native clients. Conditional skips of unsupported Windows
execution tests must remain distinct from demonstrated Windows runtime support.

## Selection and final-result contract

1. Record exact base/head revisions and changed paths. For PRs, compare the event's
   base commit with the tested merge tree; for pushes, compare before/after revisions.
   Handle renames and deletions using both paths. Fetch required history; if the
   comparison is unavailable, run the full suite rather than treating it as empty.
2. Map files to owners using the longest matching package path. In particular,
   `games/labyrinth/rules` owns `labyrinth_rules`, not the outer game package.
   Use base and head workspace metadata and include normal, development, build and
   target-specific local dependencies when computing reverse consumers. Start
   conservatively: a changed package manifest or graph selects broader checks.
3. Keep a small reviewed map for non-Cargo inputs: skill packages, installers,
   repository validators, distribution fixtures, native client metadata, assets
   and policy files. Unmapped paths select the full suite. Produce selected jobs,
   packages, source revisions and reasons as a readable CI summary.
4. Keep the workflow itself present on every PR/main push; select jobs inside it.
   The final `ci` job uses `always()` and depends on the classifier, cheap checks
   and every selectable job. It passes only if classification and cheap checks
   succeeded and every selected job succeeded. Failure, cancellation, malformed
   selection or an unexpectedly skipped selected job must fail that result.
   An intentionally unselected job can be skipped.
5. Apply the same scope rules to `main` pushes. Provide a manual full-suite run and
   run the complete suite on release candidates. Cancel obsolete runs for the
   same PR; preserve main integration runs. Add merge-queue event support if the
   repository later enables that feature.

GitHub distinguishes a skipped workflow from a conditionally skipped job: path
filters can leave required checks pending, while skipped jobs can report success.
Its guidance recommends `always()` with `needs` for dependent required checks.
That is why this design keeps a stable aggregate and inspects its dependencies.
See [GitHub's required-check guidance](https://docs.github.com/en/pull-requests/how-tos/merge-and-close-pull-requests/troubleshooting-required-status-checks).
Main had no branch protection configured when inspected for this proposal;
requiring the new aggregate is a separate repository-settings decision after its
behavior is verified. This plan does not change those settings.

Current local dependencies support this split: the three games consume the
GameKit facade, and Labyrinth additionally consumes `labyrinth_rules`. The facade
consumes the capability crates; discovery and multiplayer consume session, and
the test crate consumes UI. Thus shared capability edits may legitimately select
all game consumers, while a game-only change need not do so. Recompute this graph
from manifests; do not preserve this paragraph as executable routing logic.

## CI implementation and acceptance

Deliver the classifier, job split and final result together in a focused PR.
Start with component/package scope; defer finer source-file exclusions until their
coverage is demonstrable. Keep existing test commands and required environments
where applicable. Do not turn off slow tests solely because they are expensive.

Verify routing fixtures for narrative docs, skill Markdown, runtime-only changes,
each game, nested rules, shared UI and networking, manifests/lockfiles, CI changes,
mixed changes, deleted/renamed inputs and unknown paths. Test unavailable history
and metadata, selected-job failure/cancellation and unexpected skips of selected
jobs. A missing classifier result must never produce a successful aggregate.

Use actual CI runs to demonstrate that docs avoid Rust and runtime jobs, game-only
changes avoid unrelated components, and shared changes retain their consumers.
The CI implementation PR itself should select the full suite because it changes
the routing mechanism. Preserve a manual full-suite escape hatch and compare the
selected commands against the previous workflow before accepting it. Record job
and wall times with source revisions; report measured changes without projecting
unobserved token or monetary savings.

## Three bounded workflow trials before Rust

Use the installed, pinned GameSkills candidate and `plan` as the entry point.
Record the actual bundle/client versions and any deliberate update. The five-worker
ceiling remains, limited by available host capacity; use workers only for useful
independent tasks with explicit ownership and serial integration.

Accepted operating defaults: small tasks receive a brief conversational plan;
save a durable plan for work spanning sessions, significant dependencies or
coordinated workers. Balance completion time and total usage, including planning
and integration overhead; the worker ceiling is not a target.

| Trial | Work and skills to exercise | Completion evidence |
|---|---|---|
| 1. CI scope | Implement the routing above through `plan`, `test`, `review`, `update-docs`, `create-pr`, `audit-pr` and an authorized `merge-pr`. Use maintainer guidance where workflow contracts change. | Routing and failure fixtures pass; actual CI selection is observable; all previous coverage has a mapped owner; the reviewed PR is delivered. |
| 2. Small docs cleanup | Select one contradictory or duplicated operational topic from the inventory, reconcile its owner and callers, and deliver it through the documentation/PR workflow. | Working local links and current instructions; actual docs-only CI selection; review findings resolved. Do not start the full documentation restructure. |
| 3. Shared UI change | Inspect Labyrinth and Deckbuilder together and select one concrete tooltip or interaction improvement. Use UI skills, affected library tests, both consumers and focused playtesting. Exercise `dispatch` when game-side work can proceed independently. | Both games demonstrate the intended behavior; consumer and interaction evidence match the candidate; worker ownership and integration are observable where used. The user reviews changes to the player experience. |

These are three useful changes, not a new feature program. Choose the specific
documentation topic and UI contract during their plans from inspected problems.
Do not manufacture parallel work, queue injection or failures just to tick a box.
Where a pilot needs newly discovered work or resume/recovery, exercise the real
flow; use controlled contract tests for failure paths not naturally encountered.

For each trial, retain the task brief, plan/routing choices, actual invocations and
check results, source/bundle identities, review findings, corrective interventions
and remaining limitations. Capture elapsed time and model/token usage only when
available from the actual tools. Human playability feedback remains separate from
automated checks. Record what changed in the skills or runtime because of a finding
and rerun the affected verification after that change.

Begin the Rust contract-freeze slice after these bounded tasks have exercised
delivery and their material workflow defects are addressed. Carry unresolved
client/platform limitations explicitly into migration and release acceptance.
Do not wait for a playable game release, all 25 scenario rubrics or the deferred
Port Vila adoption before starting the port. Keep the 12-skill core and optional
packages stable unless concrete findings warrant a reviewed change.
