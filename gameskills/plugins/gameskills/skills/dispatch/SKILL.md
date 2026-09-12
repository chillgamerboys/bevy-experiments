---
name: dispatch
description: Coordinate an established, authorized wave of independent GameSkills work orders, worker lifecycle, resources and serial integration. Use --inject to append justified work to its existing queue; use plan to settle or amend scope.
---

# Dispatch an established wave

Read [project context](../../references/project-context.md) and
[work orders](../../references/work-orders.md). Confirm explicit task or standing
authorization for parallel agents. A configured cap is a ceiling, not permission
or a target to fill. Preserve session model defaults unless overrides are
authorized; resolve configured roles/effort to actually supported host choices.
Pass authorized model/effort mappings explicitly at launch; an order label does
not select the model. Use review capability appropriate to the work, defaulting
to a role no weaker than implementation when the project uses that role policy.

Inspect live queue revision, source/base, ownership and resources before each
start or resume. Dispatch only settled work whose start blockers are satisfied.
Human streams reserve territory and integration order without using agent slots.
Shared manifests, lockfiles, fixtures, assets and native windows need explicit
owners or sequencing; different agent IDs do not make a shared checkout isolated.

Use the host's real worker mechanism and verify each worker's worktree, branch
and base. Record start/resume with the queue helper and collect running checks to
completion. Respect CPU/memory, GPU/window, ports and other resource constraints.
Start useful independent work when a slot opens; name the concrete blocker when
waiting. Keep useful partial workers on resume instead of rediscovering their work.

Require returned changes, source identity, artifacts/PR and evidence references,
findings and remaining blockers. Review their combination, coordinate fixes with
owners and integrate accepted streams serially within authorization. Record the
observed integration afterward; `queue integrated` does not merge code. Recheck
the combined application and invalidate affected evidence when the base changes.
Communicate shared source/environment defects promptly to affected workers.

## Inject work

Treat `--inject` as this skill's mode, not a separate skill or runtime switch.
Verify the reported defect or addition against current facts; check relevance,
authorization and planned/running/human ownership. Prepare an order with the same
contract, explicit addition reason and start/merge blockers. Append with `queue
inject QUEUE_ID --file ORDER.json --expected-revision N` and inspect the result.
The queue is authoritative; derive status views from it. An existing stream's
ownership/design amendment returns to `plan`; within-scope fixes stay with its
owner. Unrelated discoveries do not silently expand the wave.

Report completed/blocked streams, reviewed integration and remaining work. Track
actual coordinator/worker usage and elapsed time when available; missing telemetry
is unavailable, not zero. Worker launch, budget exhaustion and stopped checks are
not completion.

Work orders identify affected owner docs and any proposed constraint changes.
Treat shared docs/index edits as owned files; reconcile links at integration.
