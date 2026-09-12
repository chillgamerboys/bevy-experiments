# Workflow reliability follow-up

Status: active
Owner: GameSkills; [HEX-98](https://linear.app/chillgamerboys/issue/HEX-98/strengthen-gameskills-delivery-and-add-optional-linear-workflows), PR #38.

## Current state

Solo delivery records, observed PR checks, stale-evidence diagnostics, legacy craft
consolidation, selective grill and optional Linear tracking are implemented in the
draft candidate. Current behavior and limitations belong in workflow, architecture,
installation and troubleshooting docs. The docs-and-skills refactor is tracked in
[its shared plan](../../../docs/plans/docs-and-skills.md).

## Remaining work

- Make the connected Linear MCP the primary tracking path. The skill can use the
  host connection directly; the CLI currently launches a standalone observer instead.
  This is an implementation choice, not an MCP limitation. Keep the helper optional
  and distinguish actual host/MCP verification from checks executed by the CLI.
  Do not require another API key for ordinary tracking. The current required observer
  remains unaligned; do not report it passed merely because MCP verified the links.
- Independent skill evaluation moves to the [next-session plan](skill-evaluation.md).
  The user explicitly removed it from PR #38 merge acceptance; keep it discoverable
  without treating the unrun trials as passed.
- Reconcile observed delivery and remaining acceptance before declaring PR #38 ready.
  Merge/public release need their own authorization.

## Deferred deletion

Issue deletion and the live Hex pilot are deferred until requested. Mandatory ticket
backups are removed from the accepted workflow; no export directory or deletion key
is needed now. The existing helper remains an experimental implementation of the
older export contract, not the currently recommended tracking entrypoint. Revisiting
cleanup must resolve or remove that prototype rather than treat its old guards as new
user requirements. Do not schedule deletion or infer reclaimed quota from Done state.

## Evidence to preserve

Implementation source `7bcee8d2858650f6b0a45e05097099e315634fad` passed 304 local
Rust tests and nine CI checks. Its packaged consumer/native discovery and schema
checks are historical observations, not validation of subsequent source changes.
Native discovery then observed 13 core and 15 core-plus-Linear skills without model
turns. All 20 GraphQL operations matched the inspected official schema; authenticated
provider behavior, quota effects and live deletion were not exercised. No tickets
were deleted. Preserve original run/bundle records; do not relabel these passes.

Completion requires the remaining tracking work to be resolved or explicitly
scoped out, current docs reconciled, and actual PR delivery verified. Retire this plan
when the whole outcome is settled; retain durable rationale in current topic docs.
