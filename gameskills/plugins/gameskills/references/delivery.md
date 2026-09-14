# Delivery identity and authorization

Record the requested endpoint: design, implementation, reviewable PR, merge or
release. Follow established session authorization and the receiving project's
requirements. Finish useful preparation before seeking any missing final approval.
Creating a PR does not accept it; a successful audit does not authorize merging;
merging does not prove a release was published.

Honor explicit standing authorization for ordinary feature merges when its branch,
required checks and applicable review conditions are satisfied. Do not ask again
for each such PR. The project's promotion branch remains a distinct endpoint;
never infer promotion or release authority from feature-merge permission.

Keep one cohesive deliverable in one PR, including dependent worker commits.
Independent changes can be separate; use stacks only when their review value
justifies restacking and repeated integration checks. Observe each merge before
advancing the next. Use ancestry-preserving merges for recurring branch promotions;
squash feature PRs only when no unmerged descendants depend on those commit IDs.

## PR and audit identity

Identify the actual repository/provider, PR URL/number, source branch and HEAD,
target branch and base revision. Include relevant tree, configuration, dependency,
artifact and environment identities for checks. Read current source/CI/reviews
through the configured provider or CLI; do not infer state from a stale comment.
On an ambiguous mutation result, query the actual remote state before retrying.

An audit is an acceptance record, not merely a successful command. Associate each
planned requirement with review judgment, command evidence, documentation checks,
and applicable manual/playtest evidence. Name skipped, pending, failed and
unavailable checks within the resolved rigor and affected scope. Coverage outside
that scope is not a missing gate. Evidence reuse follows [verification](verification.md): changed
inputs invalidate affected records; never rewrite old records for a new HEAD.
Where a dependency is uncertain, rerun the check.

Do not post comments, contact reviewers or message others without authorization.
When publishing a PR is authorized, its concrete body describes the final problem,
result and evidence. Respect the receiving project's templates and attribution
rules. For Bevy itself, use the optional contribution workflow: human review and
human-authored upstream prose are required under its receiving policy; ordinary
agent-written PR delivery is not appropriate.

## Integrate and release

Immediately before an authorized merge, confirm the current source/base,
mergeability, required CI, review state, accepted findings and project rules.
Rebase/merge conflicts are a new implementation state requiring affected checks.
Integrate parallel streams serially and verify the combined result. Reuse passing CI
when its tested merge tree matches the observed integrated tree and required inputs
are unchanged; a new commit ID alone does not require identical tests again. A
different tree, advanced base or uncertain dependency needs affected verification. Observe the
remote merge identity and check the resulting target revision. A helper recording
an integration observation does not perform or independently prove the merge.

For a release, verify the artifact users will consume. Record its version/source,
checksums where applicable, selected platform/features and dependency compatibility.
Distinguish source consumption, packaged-artifact installation and actual published
availability. Test from a clean consumer/run location so workspace paths, caches,
assets and locally installed skills cannot conceal missing package contents.

Report the prepared artifact and remaining blocker when publication is outside
scope or not authorized. Preserve unreleased changes and existing releases; do not
delete or replace a published identity to make a failed attempt look successful.

## Durable solo tasks

Use the configured runtime prefix (normally `gameskills`) for these commands:

```text
gameskills delivery start TASK --goal "Concrete result" --endpoint pr --repo OWNER/REPO --base main --check CHECK_NAME
gameskills delivery show TASK
gameskills delivery bind TASK --pr https://github.com/OWNER/REPO/pull/NUMBER
gameskills delivery check TASK --evidence RUN_ID
```

`start` defaults to `project.delivery_target` and `project.delivery_base` (or
`main` when absent), and refuses to overwrite an existing task. Explicit narrower user scope takes precedence. Records under
`.gameskills/delivery/` retain intent across interruption; `show` is historical,
`check` rereads source and providers.
Use `delivery note TASK --remaining TEXT --authorization TEXT` to preserve a
concrete handoff; repeat remaining flags for multiple items. Omitting them clears
the recorded list only after the work is actually handled. Notes grant no authority. List existing record filenames when resuming
without an ID. Keep authorization in the session/handoff: a record is not a grant
of permission. Bindings are unverified until checked. Checks cannot force an
agent that never invokes them to finish delivery.

`design` and `implementation` avoid PR requirements. PR checks observe the actual
repository, remote source and base; merge additionally requires observed remote
integration. Release acceptance remains with the release skill. Commands and
external observations do not replace source review or gameplay acceptance.

## Milestone sanity

Select an actual promotion explicitly (`delivery start/scope --promotion`), not
merely from a branch name or Testing rigor. Ordinary gameplay PRs are not implicit
milestones. Receiving-branch verification still applies independently.

Resolve the receiving branch's verification policy and review the combined batch's
affected journeys. When its manual-sanity policy requires a milestone game check,
record the developer's explicit response, actual source/reference, candidate identity
and covered journey. An agent walk, CI success or a previous feature acceptance is
not this observation. Do not fabricate a human response or treat caller-supplied
metadata as independently authenticated testimony.

Relevant game changes invalidate that observation. Carry unchanged coverage forward
only with its applicability reason; do not relabel historical evidence. Ordinary
development PRs have no milestone gate. Documentation or tooling changes without
game effects need no gameplay sanity check. Automated CI and manual acceptance have
separate statuses; a missing required human observation leaves the promotion pending,
while coverage outside the selected scope is not unfinished work.

## Tracking observations

Core-only installations omit tracking. For adopted tracking, connected host MCP is
the normal path: `[tracking] required = true` with `mode = "mcp"`. Bind
`--issue UUID --project UUID` on the task. Use the adopted tracker skill and actual
connected tools to reread the issue, its project and PR attachment, preserving the
tool response as evidence. No standalone executable or separate API key is needed.

Pass a fresh normalized snapshot to each check:

```text
gameskills delivery check TASK --evidence RUN_ID --tracker-observation FILE.json
```

```json
{
  "schema_version": 1,
  "transport": "mcp",
  "tool": "actual connected issue lookup tool",
  "evidence_reference": "path or host reference to the actual tool response",
  "task_id": "TASK",
  "source_head": "FULL_CURRENT_GIT_HEAD",
  "observed_at": 1789335177,
  "issue": {
    "id": "ISSUE_UUID",
    "project_id": "PROJECT_UUID",
    "url": "https://tracker.example/issue/identifier",
    "attachment_urls": ["https://github.com/OWNER/REPO/pull/NUMBER"]
  }
}
```

Use the actual observation time in Unix seconds and map stable IDs from the tool
response, not display identifiers. The example timestamp is not reusable evidence.
The checker requires the same task, source HEAD and bound issue/project/PR, an
observation no older than 300 seconds and no future timestamp, and a matching issue
URL in the freshly queried GitHub PR body. Missing one-way links or connector access
remain verification gaps. Query again after changes or expiry; never retimestamp an
old response or substitute `ok: true` for observed fields.

The result retains the supplied snapshot and its digest, explicitly labeled as
caller-supplied MCP evidence. The CLI checks identities and the live GitHub backlink;
it cannot invoke the host's MCP tools, authenticate that invocation, or independently
verify the supplied timestamp/reference. Review the actual tool evidence separately.
Stored `show` output is historical; each `check` requires its explicit snapshot.

For deliberate headless use, `mode = "command"` selects a configured executable
argv such as `observer = ["gameskills-linear", "observe"]`, with no shell
interpolation. The observer receives `--issue UUID --project UUID --pr URL` and
returns JSON with `ok`, `linked`, `issue_id`, `project_id` and `pr_url`. All exact
identities must agree. Existing argv-only configurations retain command behavior;
without an argv, omitted mode defaults to MCP. Mixed modes are rejected, and an
MCP snapshot cannot override command mode. The separate plugin owns provider setup.

When the user extends an existing PR task to merge, retain its record and create a
separate `--endpoint merge` task with the same bindings. Check PR acceptance before
merging, then check the merge task after integration from the source checkout. A
premerge merge check correctly reports that integration has not happened yet.


Configured tasks accept `delivery start ... --base BRANCH --level LEVEL --scope LABEL
--gameplay`; repeat scope labels as needed. `--gameplay` is the caller's explicit
classification of game effects. `delivery scope TASK --scope LABEL [--gameplay]`
binds updated scope/policy while preserving the original task/base and history.
Omitting `--level` retains the recorded level. Repeated `--check COMMAND` options
replace the required check list with its previous value retained in history. Missing
flags do not infer a game's effects; inspect the actual change when classifying.

For a gameplay milestone, `delivery check ... --manual-observation FILE` accepts
schema 1 JSON with `task_id`, `source_head`, `verification_digest` (the task's
`verification.selection_digest`), `result: "passed"`, `observer`, `journey`, and
`evidence_reference` to the developer's actual response. Paths are relative to the
adopter root. The checker binds the supplied observation to task, candidate and
selected scope; it does not authenticate the person or gameplay result. Never
manufacture that response. Ordinary Development and tooling-only tasks need none.
