---
name: update-docs
description: Create or reconcile Bevy/GameKit project explanations, examples, API guidance and migration documentation. Use for documentation work or a changed contract; historical logs and execution queues are not current source documentation.
---

# Update authoritative documentation

Read [project context](../../references/project-context.md). Identify the reader,
topic owner and changed contract before editing. For a broad refactor, inventory
callers and classify each document as keep, revise, combine, move or retire;
resolve contradictory facts before moving files.

Read implementation, manifests, commands and examples for the relevant version.
Public API contracts belong in crate Rustdoc and tested examples; project choices
belong in project documentation; durable rationale belongs in decision records.
Skills carry decision guidance and source-navigation hints, not duplicated API
manuals. Preserve still-useful history while retiring obsolete operational claims.

Write runnable examples and explicit scope/compatibility. Update indexes, inbound
links, skill references and machine readers when paths change. Distinguish proposed,
implemented, verified and released status. Distill temporary work orders into durable
decisions and outcomes; old maps must not masquerade as current code ownership.

Run relevant link/structure/doc/example checks once for the edited inputs, using
`gameskills:test` and project commands where useful. Execute advertised commands
when practical; disclose unverified platform or environment claims. A spelling or
link-only change does not automatically need a native game walk.

For Bevy upstream documentation, follow the receiving project's current AI policy
and the optional contribution workflow; do not prepare agent-authored public prose.
For this project's authorized docs, deliver the reconciled material and evidence
through the active task's requested endpoint.

Keep the task's requested endpoint and remaining work in the delivery record.
Documentation completion does not shorten the task to local edits. Include final
scope and operational limitations in the PR and the task handoff.
