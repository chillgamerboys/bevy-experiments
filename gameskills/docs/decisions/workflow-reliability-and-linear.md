# GameSkills workflow reliability and optional Linear integration

Status: approved; implementation and validation in progress, September 12, 2026.
Tracking: [HEX-98](https://linear.app/chillgamerboys/issue/HEX-98/strengthen-gameskills-delivery-and-add-optional-linear-workflows), in Bevy Games.
Baseline: `5a19b55e6ca94e8f97a4981f3d151b9c769905a2`, the merge of
[PR #37](https://github.com/chillgamerboys/bevy-experiments/pull/37).

## Outcome and scope

Make GameSkills carry authorized work to its requested endpoint, expose missing
delivery steps, preserve the useful legacy craft lessons in one current catalog,
and offer optional Linear tracking and on-demand retention cleanup.

The user approved implementation and the manual-sweep revision. PR #38 now carries
the implementation, with HEX-98 kept in progress until the program's acceptance
work is complete. The live Hex pilot still requires the private export destination
and separately configured supported API credentials. No ticket has been deleted.

No gameplay changes, repository extraction, new projects for the other games,
team-prefix changes, or automatic adoption into linked workspaces are included.
Keep the `HEX` team identifier. Use one tracking issue for this program initially;
split it only when independently deliverable work benefits from separate tracking.

## Accepted decisions

| Area | Decision |
|---|---|
| Delivery | Preserve the requested endpoint across planning, execution, interruption, and resume. Observe external results before reporting completion. |
| Skill activation | Distinguish source guidance, installed content, native discovery, loaded skills, and actual model behavior. |
| Legacy | Consolidate craft instructions into current owners; retain only necessary compatibility material and adopter-owned state. |
| Planning | Adapt jxp's read-only grill for material ambiguity; use small rounds, recommendations, and explicit settled/deferred decisions. |
| Linear | Add a separately selected `gameskills-linear` plugin. Core-only adoption requires no Linear connection or credentials. |
| Routing | Bevy Games owns repository/shared work; Labyrinth owns game-specific work; Hex owns the separate bevy-hex-game backlog. |
| Retention | Manually invoked sweeps of tickets completed at least 30 days ago; duration remains configurable. No scheduled deletion. |
| Cleanup | Export durably before deletion; protect unfinished related work, handle reopening, and make retries safe. An authorized cleanup invocation needs no repeated per-ticket approval. |
| Capacity recovery | A verified issue-limit error can invoke the same cleanup preview. Apply a sweep when covered by the user's cleanup instruction; lack of capacity alone does not authorize deletion. |
| Pilot | Preview old Hex tickets, then exercise the real cleanup command on at most three eligible tickets. |

The user revised the earlier automatic-retention decision to manual sweeps.
This supersedes the daily scheduler proposal and its hosting decision. The
30-day eligibility threshold and export-before-delete contracts remain accepted.

## Observed failure and investigation boundaries

The existing core plan skill already requires delivery through the authorized PR
endpoint. This repository also configured `project.delivery_target = "pr"`.
Nevertheless, the agent stopped after five local commits and ad hoc verification;
the user had to request publication. Reading those instructions did not constitute
using a complete installed workflow. GameSkills was not exposed in that session's
native skill catalog, and source files were read directly.

The CLI currently records command execution and validates source/input identity.
Those records do not prove push, PR creation, tracking, review acceptance, or merge.
The original task endpoint was not checked against observed delivery before the
agent reported completion. No Linear plugin or project mapping existed then.
Linear absence and failure to publish are separate findings: publication was
already required without Linear.

The [legacy migration map](../../plugins/gameskills-maintainer/references/legacy-migration.md)
records intended lesson destinations but explicitly does not establish behavioral
parity. The old `legacy/README.md` also retains a retired package command, showing
that compatibility documentation needs review along with source consolidation.

The audit must distinguish demonstrated failures from hypotheses. In particular,
do not claim that missing native discovery alone caused the delivery omission, or
that an installed older pin is corrupt merely because canonical source advanced.
Do not promise a runtime helper can force an agent that never invokes it to comply.

## Implementation sequence

### 1. Reproduce the failure and establish the contracts

Inspect the actual host launch/discovery paths, selected package pins, project
instructions, config loading, and entrypoint routing. Compare canonical source,
the installed lock/bundle, and instructions observable in a real native session.
Produce a short failure report with evidence, root causes, and unresolved causes.

Define the delivery record and completion contract before adding commands. Cover
solo work as well as existing queue workflows. Keep a plan-only conversation free
of mandatory execution queues, tickets, and publication unless requested.

Prepare realistic regression tasks from PR #37 before changing the instructions.
Include an interrupted task resumed with "continue", a detached checkout, passing
tests with no push, and a source-only skill fallback. Record the old behavior;
do not instruct the evaluator which answer or failure it should produce.

Review command-evidence invalidation diagnostics: the pre-merge record became
stale without a source-commit change. Report which input category changed so
unnecessary reruns can be diagnosed. Narrow fingerprint scope only with evidence
and corresponding tests; do not weaken checks merely to obtain a passing record.

### 2. Add durable delivery state and complete workflow routing

Extend the existing Rust CLI with a small solo-task delivery record and inspect/
check operations. Reuse current identity and atomic storage mechanisms; do not
create a second queue or a universal task orchestration engine.

Records identify the task, requested endpoint, repository, source/base, selected
instruction/runtime identities, relevant checks, external PR, optional tracker
binding, remaining work, and authorized actions. Distinguish recorded intent from
provider observations. Explicitly retain plan-only and implementation-only scope.

Completion inspection must identify which required deliverable is missing or
unverifiable. For a PR endpoint, check the actual remote source commit, repository,
base, and PR URL. For merge, also check required review/CI and observed integration.
An unavailable provider is not evidence that no PR exists. After an ambiguous
mutation response, query by stable identity before retrying.

Update `plan`, `test`, `update-docs`, `create-pr`, `audit-pr`, and `merge-pr` at their
existing ownership boundaries. Core delivery reads optional tracker results
through a narrow interface; core-only operation has no Linear dependency.
Respect existing authorization and expose concrete gaps without repeatedly asking
for already granted permission. Provide a concise final delivery summary from the
record while retaining human review and gameplay judgments as separate evidence.

Verify setup and actual host routing. Any generated project instructions must be
small, owned, and installed through supported setup; do not silently edit global
agent settings or installed caches. Report fallback use plainly when native
activation is unavailable.

### 3. Consolidate legacy guidance and adapt grill

For every original craft skill and shared reference, record a current destination,
an observable behavior, and a test/evaluation or a reasoned retirement. Existing
intentional retirements, such as universal UI pixel minima, must not be reintroduced
just to make the texts identical.

| Legacy capability | Canonical owner to verify |
|---|---|
| Architecture and extraction | Core plan/review/debug and architecture references; maintainer extraction guidance |
| Building and verifying UI | `gameskills-ui` build/verify skills and contracts |
| Turn-based domain rules | `gameskills-turn-based:model-rules` |
| Runtime debugging | Core debug and Bevy craft |
| Testing and evidence | Core test/verification, with UI and multiplayer specifics in their packages |
| Code review | Core review; delivery acceptance in audit-pr |
| Installation and sync | Core setup and supported legacy importer |

Audit whether architecture needs a more discoverable entrypoint; decide from
observed routing failures rather than adding another skill speculatively. Remove
the duplicate legacy authoring tree once parity and migration contracts pass.
Necessary frozen fixture bytes belong with compatibility tests, with provenance.
Keep recorded overlays, generated adopter files, base snapshots, and old evidence
intact. Do not recreate the retired Python installer or sync pipeline.

Add a core `grill` skill, adapted from jxp's
`plugins/jxp/skills/grill/SKILL.md`, inspected at
`b11265ad9681f54a0e365416ce655d279b53f437`. Preserve applicable source attribution.
Research facts without asking the user to do that research. Ask only decisions
that materially change the work, normally one to three questions per round, with
recommended answers. Record accepted, delegated, and deferred decisions; reopen
contradictions explicitly. Clear tasks proceed directly to planning. Routine
implementation choices do not require interviewing the user. Grill itself writes
no code, tickets, or external state and is not a test/audit gate.

### 4. Add the optional Linear package and delivery integration

Author the self-contained native package at
`gameskills/plugins/gameskills-linear/`. Provide focused tracking and cleanup
skills with the necessary setup/configuration reference, using the existing core
plan and PR lifecycle. Avoid duplicating those workflows in the new package.

The cleanup command needs deterministic selection, export, and mutation behavior.
Use an optional Rust helper under `gameskills/linear/`, package/binary
`gameskills-linear`, if the adapter cannot fit an already supported optional
runtime mechanism. Keep Linear-specific network dependencies and credentials out
of default core installation. Freeze the concrete command/config schemas in phase
1; proposed names in this plan are not existing CLI commands.

Provider operations must support create/reuse, exact lookup, attachment/linking,
state reconciliation, export, and deletion observation. Test against the actual
supported Linear API. The current MCP tool catalog has no issue-delete tool;
verify a supported authenticated API path for the cleanup command. Never extract
interactive connector credentials or assume a connected agent session supplies
credentials to a separate executable.

Persist bindings by workspace/team/project/issue UUID and repository/PR identity.
On opted-in implementation work, resolve or create the ticket in planning, link
the PR in both directions, and verify both objects. A closed unmerged PR does not
complete an issue. A multi-PR issue completes only after its required work is done.
Missing tracking configuration blocks the tracking requirement while independent
implementation/preparation can continue. Optional adoption remains explicit.

Current project mappings, to verify again before adoption:

| Project | UUID | Scope |
|---|---|---|
| Bevy Games | `401f99ba-c1ec-4494-ab0d-ebf0134d8b79` | bevy-experiments repository, Gamekit, GameSkills, devtools, and work across games |
| Labyrinth | `4196590c-753e-4152-9291-ff8c137eb468` | games/labyrinth and its rules |
| Hex | `6935183d-4284-405f-845a-e6ea9bbe6d42` | bevy-hex-game repository |

All currently use team `28b8704f-ced3-4884-9601-4ea07b2ca778` and prefix `HEX`.
The prefix is never a project selector or a cleanup boundary. Shared and game-local
parts can be related issues in their respective projects when separate work is
useful; do not force a ticket for every file or commit.

### 5. Implement manual sweeps, durable exports, and capacity recovery

The optional plugin offers an on-demand cleanup command. Thirty days determines
eligibility, not when a job runs. Installing the plugin does not schedule cleanup
or delete tickets. Require explicit project scope; begin with Hex during the pilot.

Proposed command shape, to finalize with the adapter interface:

```text
gameskills-linear cleanup --project PROJECT_UUID --retention-days 30
gameskills-linear cleanup --project PROJECT_UUID --retention-days 30 --limit 3 --export-dir DURABLE_DIRECTORY --apply
```

The first form previews candidates and skip reasons. The second exports and
deletes a bounded batch under the existing cleanup authorization. These are
proposed interfaces, not commands available in the current CLI. Support a
configured durable export destination as well as an explicit per-run destination.

Eligibility requires all of the following:

- Exact workspace/project match, current completed status, and a trustworthy
  completion timestamp at least 30 full days old, evaluated in UTC.
- No unfinished children, active parent work requiring the ticket, or unfinished
  dependent work that would lose necessary context. Inspect complete paginated
  relations; uncertain relationships mean skip with a reason.
- A successfully stored, verified, durable export of the current ticket and its
  delivery context. Export failure prevents deletion.

Use the completion date, not creation or general update time. Project moves do
not reset retention. Reopening cancels eligibility; completing again starts a new
period. Missing or ambiguous lifecycle history requires skipping, not guessing.
Canceled and duplicate states are not implicitly completed tickets.

Export descriptions, comments, relationships, project/status identity, important
decisions, PR/merge links, and referenced attachments/documents needed to preserve
the record. Resolve temporary download URLs into durable content where necessary;
an expiring link alone is not an export. Retain original identifiers and store a
manifest with hashes and observed revisions. Verify persistence in the selected
durable store before deleting; local temporary files and expiring CI artifacts
are insufficient. A backed-up user-owned directory or private remote store can
satisfy this contract; a new repository is not a mandatory prerequisite.

Separate preview, export, delete, and reconcile stages. Recheck current eligibility
and revision immediately before deletion; refresh changed exports. Investigate API
conditional-mutation support, and document any remaining read/delete race rather
than claiming atomicity. Use standard recoverable deletion, not a permanent-delete
shortcut. Record provider outcomes. Authentication failures and failed lookups must
not be interpreted as successful deletion.

Use stable operation identities, bounded batches, concurrency control, rate-limit
handling, and interruption/retry recovery. A repeated run must not duplicate
exports unnecessarily, delete outside the selected set, or fail on a previously
verified deletion. Retain a keep policy that disables deletion for projects where
history should remain in Linear.

When ticket creation receives a verified issue-limit response, invoke the same
scoped preview and report how many tickets qualify. Do not classify authentication,
network, rate-limit, or unrelated validation failures as exhausted capacity. If
the current instruction authorizes cleanup, execute the bounded sweep; otherwise
offer the concrete sweep to the user. Never broaden project scope, shorten the
retention period, or delete unfinished tickets to force ticket creation through.
After verified deletion, check whether capacity was recovered and retry the
original creation once, reconciling any ambiguous prior result to avoid duplicate
tickets. If capacity remains unavailable, report that blocker without looping.

Credentials use the command's supported secret mechanism. Do not log ticket
bodies or export them into this public source repository. No cron, GitHub Actions
schedule, background daemon, or scheduler infrastructure is part of this plan.

Linear documents a 30-day recovery window after deletion; that window is separate
from our 30 days before deletion. Verify actual quota accounting before claiming
that deletion or auto-archiving increases the free-plan allowance. Sources:
[deletion and archive behavior](https://linear.app/docs/delete-archive-issues),
[billing and plans](https://linear.app/docs/billing-and-plans).

### 6. Package and evaluate the combined candidate

Update native catalogs, scenario fixtures, migration accounting, CLI package
contents, documentation, and bundle provenance. Keep source changes and generated
artifacts reviewable. Verify an immutable candidate from actual Cargo archives,
including core-only and Linear-enabled adopters. Explicitly update this repo's
installation when adopting the candidate; canonical edits do not update the lock.

Run deterministic tests for delivery endpoints, source/base changes, unavailable
providers, existing bindings, failed mutations, missing exports, and retention
boundaries. Test native skill discovery, direct/implicit routing, coexistence with
local guidance, and update/recovery behavior. Test cleanup preview/apply on the
actual supported command platforms. Cover true quota errors, unrelated failures,
authorization boundaries, bounded retry, and no eligible candidates.

Run bounded forward tasks through available native clients using pinned candidate
and control identities. Separate discovery-only results from actual model behavior.
Independent evaluators require applicable delegation authorization; otherwise
state the evaluation limit. Include tasks withheld from instruction authoring and
report cost, retries, elapsed time, and corrective user interventions where exposed.

### 7. Run the Hex pilot and finish delivery

The September 12 read-only inventory found 38 completed Hex tickets older than 30
days. They are age candidates only; dependencies and exportability have not been
qualified. Refresh this inventory before selecting any live batch.

First preview all candidates with eligibility/skip reasons. Select at most three
eligible tickets, preferring standalone completed work. Preserve the full exports
in the approved durable store, run the real cleanup operation, verify remote
deletion state, and rerun to demonstrate idempotency. Check unaffected tickets and
both other projects. Verify export readability and recovery behavior where the API
supports it; do not claim export is a lossless Linear restore until tested.

Use real completion dates and the chosen 30-day duration; do not alter timestamps
or lower retention merely to manufacture eligible records. Record observed quota
effects separately. Any subsequent sweep uses the same qualified command and its
explicit scope; the pilot does not schedule recurring cleanup.

Deliver logical implementation PRs linked to HEX-98, with combined validation of
the final candidate. Refresh external checks at each actual PR source/base. Merge
only under applicable authorization for those PRs; PR #37's merge instruction
does not authorize every future merge. Public package release and migrations of
other linked workspaces remain separate actions.

## Acceptance matrix

| Claim | Required evidence |
|---|---|
| Delivery no longer silently stops locally | Realistic PR-endpoint and resume tasks observe pushed source, PR, configured tracker, or an accurate blocker; plan-only tasks remain bounded |
| Activation claims are honest | Source/install/discovery/model-use observations are distinct in supported native clients |
| Legacy craft survives consolidation | Complete lesson inventory with current destinations and exercised behaviors; migration fixtures and overlays preserved |
| Grill improves material decisions | Fuzzy tasks surface meaningful choices; clear tasks skip interviews; accepted decisions survive planning handoff |
| Linear is optional and correctly scoped | Core-only adopter needs no credentials; enabled adopter routes by project UUID and links/reuses the correct ticket and PR |
| Completion reflects actual delivery | Unmerged/partially delivered issues remain open; required merged work reconciles accurately |
| Retention is safe to repeat | Boundary, reopen, relationship, pagination, export failure, rate limit, crash, ambiguity, and concurrent-change cases pass |
| Manual cleanup is predictable | Preview has no destructive effects; an authorized bounded apply persists verified exports and deletes only qualified tickets; no scheduler is installed |
| Capacity recovery is bounded | A verified quota error routes to the same scoped cleanup; other errors do not; creation retries once after observed recovery without duplicates |
| Hex pilot works | Up to three qualified tickets exported/deleted; rerun safe; skipped and out-of-scope tickets unchanged; quota effect reported honestly |
| Candidate is usable | Packaged artifacts, relevant Rust checks, native discovery/behavior, and adopted pins verified with explicit limits |

## Setup inputs before the live pilot

The scheduler-hosting question is withdrawn. No material scope question remains.
Configure or supply a durable private export destination and the supported
cleanup credentials before applying the live pilot. These are operational setup
inputs, not a requirement to create infrastructure during planning. A missing
export destination still allows preview and blocks deletion with a clear reason.
