# Efficient feature delivery

Read this when an adopter opts into model routing, task usage reporting or standing
feature-merge authorization. The project owns branch, model and check choices;
these recommendations neither change another adopter's defaults nor grant permission.

## Match the workflow to the change

Optimize total cost per accepted change, including coordinator context/reasoning,
workers, retries, review, waiting turns and rework. Track quality and elapsed time;
a cheaper worker alone does not establish a saving.

For a settled constant, copy or similarly bounded edit, use the lean solo path:
record the goal, owned files, affected checks and endpoint in the existing delivery
task; inspect relevant source/owner guidance once; implement; run one focused check
batch; review the diff; deliver. No separate plan document, queue, worker or second
reviewer is implied. Add one only when uncertainty or independent work justifies it.
A lifecycle skill supplies its missing judgment, not another discovery/test pipeline.

Prefer a short-context small-model session for bounded work when the host supports
it. Avoid keeping a strong coordinator reasoning alongside a worker doing the same
edit. A host that cannot switch the current session must report that limit; a model
label or `agents resolve` does not change it. Preserve supported host settings.
For opted-in mappings, resolve actual capabilities and pass model/effort explicitly.
Use standard/strong tiers for consequential ambiguity with a reason, and bounded
escalation after a failed attempt instead of repeated speculative fixes.

When dispatch is worthwhile, provide exact owned paths, source identity, resolved
constraints, executable prefix, affected consumers/tests and a stop condition.
Do not fork full conversation history or fill available slots by default. Integrate
worker evidence rather than repeating its completed investigation.

## Select and measure once

Resolve verification from the receiving project and changed behavior once. Reuse
that selection across implementation, review and delivery. Shared contract changes
include affected consumers; narrow does not mean library-only. Unknown impact calls
for bounded owner coverage. Changed inputs/failures can invalidate evidence; merely
entering another skill cannot. Finish the affected doc/test sweep before publication.

When native counters are exposed, mark implementation, verification and delivery
boundaries with `usage mark`, then `usage finish`. Each transition closes the previous
interval and opens the next from one snapshot. Record an active skill set only when
known; mixed sets are phase attribution, not exact costs caused by individual skills.
Capture the first baseline before work, including coordinator and every worker.
Existing `usage checkpoint` and source-backed imports remain valid for other hosts.
Missing counters or prices are unavailable, never zero. Emit identity/counters only,
not transcript contents. Do not repeatedly collect telemetry during idle waiting.

Use one compact final `usage report`: outcome/source, observed models, input/cache/
output totals, phases, elapsed time and rework. Cached input is a subset of input;
reasoning is a subset of output. Explicit dated model rates permit an estimate,
not a billing claim. Compare equivalent accepted outcomes before claiming savings.

## Wait and integrate

Use a persistent provider watcher or host completion event for required CI; keep
polling inside the process, not in repeated model turns. Redirect verbose progress
to a local log and surface terminal status or actionable failures. On GitHub, an
available `gh run watch RUN_ID --exit-status --compact --interval 30` supplies this
behavior; it does not merge or grant approval. Continue independent useful work.
If the host cannot resume on completion, report the limitation and use its longest
supported nonblocking wait; do not invent further audits just to fill the wait.

One cohesive feature belongs in one PR unless independent acceptance warrants a
split. Merge promptly under standing authorization after required checks and review.
Verify actual remote source/base and integration; preserve explicit milestone/main
approval. Reuse applicable integration evidence after checking source/tree identity.
A merge label cannot establish that an untested combined tree passed.

Project-approved Development checks may defer broad compile/lint to milestone
Testing and artifact/platform matrices to Release. Keep affected regressions and
fail-closed selected gates. Remove duplicate push checks only when the receiving
workflow ensures PR evidence covers the integrated tree. A timing fix needs affected
timing/consumer coverage, not every game, display or E2E journey; visual changes still
need relevant UI evidence. Never silently weaken agreed acceptance to meet a budget.
