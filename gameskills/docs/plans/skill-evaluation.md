# Evaluate the GameSkills candidate

Status: deferred
Owner: GameSkills. Pick this up in the next GameSkills session.
Tracking context: [HEX-98](https://linear.app/chillgamerboys/issue/HEX-98/strengthen-gameskills-delivery-and-add-optional-linear-workflows), PR #38.

## Scope and priority

The user explicitly moved independent skill evaluation out of PR #38's merge
acceptance so the existing refactor can land. This follow-up remains required work,
not a claim that skill behavior has been validated. Do not block that PR on these
trials or silently drop them after merging. Start the next session from this plan
and the maintainer `evaluate-skills` skill.

## Starting evidence

The candidate is CLI/instructions `0.1.0-dev.3`; its instruction source is
`50d305928febb1617a7cc535db062359639b7f58`. Read the current lock and run `gameskills
status` before choosing the evaluation pin; do not assume later source is identical.
PR #38 records 313 passing Rust tests, structural checks, actual Cargo-archive
consumers and preserved adopter configuration. Codex discovered 13 core and 15
core-plus-Linear skills without model turns. Solo authored docs exercises were
self-review with visible intent, not independent behavioral evidence.

## Next session

1. Read [contributing](../contributing.md) and resolve the maintainer evaluation
   skill through the host, or explicitly report canonical source fallback. Inspect
   PR #38's actual landing state and record candidate, bundle, client and source IDs.
2. Choose a small independent trial set: a resumed task that must reach its requested
   PR endpoint; unavailable native skill discovery; a README-only adopter; mixed
   owners with custom docs; a stale-doc contradiction; and completed-plan cleanup
   with a moved heading and unfinished work. Include a focused request that should
   not trigger a new plan, ticket or parallel wave.
3. Use isolated artifacts and raw tasks without revealing expected answers to the
   evaluator. Launch agents only with applicable session authorization. Compare a
   prior candidate or solo baseline where useful; do not add gameplay scope merely
   to fill a matrix.
4. Inspect actual context selection, resulting artifacts, endpoint completion and
   evidence accuracy. Record failed cases and make bounded corrections separately.
   Report missing client access or telemetry as unavailable. Codex discovery does
   not establish authenticated Claude behavior or Windows process supervision.
5. Summarize the supported behavioral claims and remaining gaps in current
   contributing/troubleshooting guidance. Retire this plan once its agreed trials
   are settled; preserve results in the relevant PR history.

No live Linear deletion, backup export, registry release or linked-workspace
migration is required for this evaluation.
