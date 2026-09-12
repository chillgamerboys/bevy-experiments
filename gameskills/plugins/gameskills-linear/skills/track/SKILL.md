---
name: track
description: Track opted-in implementation work in Linear and link its PR in both directions. Use for issue reuse, project routing and delivery reconciliation; core-only or plan-only work does not require Linear.
---

# Track

Read [setup](../../references/setup.md). Resolve the configured workspace, team
and project UUID; the team prefix never identifies the project. Shared repository
work uses its default project; game-only work uses the matching path route. Reuse
an exact existing binding before searching or creating. Missing connector access or
mapping leaves required tracking unverifiable, while independent work can proceed.

Use the connected supported Linear tools for create/reuse, issue updates and PR
attachments, or the optional helper. Persist the UUID binding with repository and
PR identity in the core delivery record. Inspect both objects after linking;
include the issue URL in the PR body and the PR URL in Linear. Query by stable
identity before retrying any ambiguous mutation. Do not create duplicates on
network, authentication or rate-limit failures.

If capacity prevents creation, report the exact failure and current scope. Deletion
is deferred; use `cleanup` for a specifically requested assessment, not an automatic
apply or a new credential/backup requirement. Do not loop ambiguous retries.

Reconcile completion from all required work, including every required PR. An open
or closed-unmerged PR keeps the issue incomplete. Observe remote integration and
recheck acceptance before setting a completed workflow state. The core lifecycle
owns planning, review and merge authorization; this skill does not duplicate it.
