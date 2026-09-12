---
name: merge-pr
description: Integrate an accepted PR when the active session and receiving project authorize merging, then verify the observed target revision. Use for the merge step; a clean review or successful audit alone does not grant merge authority.
---

# Merge and verify integration

Read [project context](../../references/project-context.md) and
[delivery](../../references/delivery.md). Recover session authorization and the
receiving project's merge requirements. Finish concrete preparation before asking
for any missing final authorization; do not ask again when it already exists.

Immediately before merging, refresh the actual repository/PR, source HEAD,
target/base, mergeability, required CI/reviews and unresolved findings. Compare
these inputs with the accepted audit. Changed inputs require affected checks and
judgment, even when the PR's title and number are unchanged.

Resolve authorized conflicts in the owning branch/worktree and reassess their
behavior. Integrate parallel streams serially. Use the project's permitted merge
method and provider; where supported, bind the mutation to the expected HEAD.
Do not bypass required gates or perform destructive rewrites to force acceptance.

Observe the remote outcome and merge/target revision. After an uncertain response,
inspect the PR and target before retrying. Run the appropriate combined integration
checks on the resulting source. Separately record any queue integration observation;
the record helper does not perform the Git/provider mutation.

Report the actual merged identity and integration results, or the precise blocker
and prepared state. Preserve unfinished checks visibly. Invoke `gameskills:release`
only when release work is part of the active task.

After the authorized merge, observe the merge commit on the actual remote target
and refresh the delivery task. Reconcile adopted tracking only after all required
PRs/work are complete; closing an unmerged PR never completes its issue. Preserve
an unavailable observation as a blocker instead of reporting an inferred success.
