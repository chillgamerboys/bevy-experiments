# Efficient feature delivery

Read this when an adopter opts into model routing, task usage reporting or standing
feature-merge authorization. The project owns its branch and model mappings; these
recommendations do not change another adopter's defaults or authorize side effects.

## Choose work and models

Optimize total cost per accepted change with quality preserved. Report tokens and
elapsed time alongside cost where measured; retries, review and rework count.
Do not optimize worker usage while hiding coordinator effort. Compare equivalent
outcomes before claiming savings; one small-model success is a trial, not proof of
general superiority.

Use `agents resolve` with the project's opt-in mapping and the host's actual model
capabilities. Small models suit clear bounded fixes, inventory and mechanical work;
shared architecture, ambiguous failures and consequential review can justify a
stronger initial tier with a reason. After a bounded failed attempt, escalate using
the configured attempt limit rather than looping indefinitely. Resolve the policy,
then pass model/effort explicitly to the host. Record requested and observed choices
separately. Unsupported mappings need correction, not a silent substitute.

Give workers the goal, owned files, accepted constraints, current source, executable
prefix, relevant evidence and stop conditions. Reference durable artifacts instead
of forking the complete conversation. Do not fill every worker slot or commission
another strong-model review without a concrete correctness benefit. Tiny changes
can remain solo if spawning and coordination cost more than doing the work.

## Measure once

Use `usage checkpoint` around native tasks where counters are exposed, or
`usage import` with source-backed thread/attempt receipts. Capture a coordinator
baseline before work and include all worker attempts, including failures. Record
thread identity, requested/observed model and effort, timestamps and raw evidence
references. Import each measured interval once; do not relabel an overlapping
interval as another task. Missing counters or prices remain unavailable, never zero.

Use `usage report TASK` for the combined report. Cached input is a subset of input;
reasoning is a subset of output. Pricing, when supplied, needs an explicit model
mapping, source and date; computed cost is an estimate, not a billing observation.
Native transcript parsing must emit counters/identity only, not prompt contents.

Keep reports compact: accepted outcome and source, actual models, available input/
cache/output totals, elapsed time, attempts/escalations, cost availability and any
quality/rework limitation. Do not print full stored plans or logs for routine status.

## Integrate promptly

The adopter selects feature and promotion branches. One cohesive feature PR can
contain several worker commits. Merge promptly once required checks and applicable
review pass within standing authorization; an open PR is not a completed merge task.
Keep milestone promotion explicit, preserve ancestry between long-lived branches,
and inspect the actual remote result before syncing or advancing dependent work.

Verification follows changed behavior and configured rigor. A timing-only tooltip
fix selects its timing/lifecycle coverage; it does not automatically select every
game, resolution, network journey or a full UI walkthrough. A changed visual flow
still needs its relevant UI evidence. Never weaken acceptance to meet a usage target.
