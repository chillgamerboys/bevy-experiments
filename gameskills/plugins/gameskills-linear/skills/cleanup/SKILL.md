---
name: cleanup
description: Manually sweep old completed Linear issues, including an explicitly requested capacity cleanup. Preview by default; bounded apply requires an authorized sweep and verified durable private exports. Never schedule deletion.
---

# Cleanup

Read [setup](../../references/setup.md) and [cleanup contract](../../references/cleanup.md).
Resolve explicit project UUID and the user-owned durable private export store.
Retention is an age threshold, not a schedule. Default to preview with reasons.

Use the supported `gameskills-linear cleanup` command. An authorized bounded
apply does not need repeated per-ticket confirmation. Preserve existing session
authorization; a quota error alone is not authorization. Do not bypass skip reasons
or reduce retention to manufacture candidates. Missing durable storage blocks
apply while preview remains useful.

Report selected, exported, deleted, skipped and unverifiable counts separately,
with original identifiers and private manifest locations. Do not put ticket bodies
or exports in a public repository. Rerun safely to reconcile interruption; never
interpret failed authentication or lookup as evidence of deletion. Report observed
quota effects separately from deletion results. No scheduled job is installed.
