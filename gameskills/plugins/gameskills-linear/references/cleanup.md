# Manual retention sweeps

```text
gameskills-linear cleanup --project PROJECT_UUID --retention-days 30
gameskills-linear cleanup --project PROJECT_UUID --retention-days 30 --limit 3 --export-dir /absolute/private/store --apply
```

Preview lists eligibility and skip reasons without exporting or deleting. Apply
requires explicit scope and batch size, current cleanup authorization, and a
user-designated backed-up private directory. The directory must already exist,
be user-owned with mode 0700, and contain no symlink components. Source repositories,
`.context`, `.gameskills` and temporary storage are refused. A configured
`export_dir` can replace the flag; the operator owns verifying its backup policy.
Installing the plugin never creates a cron job, workflow schedule or daemon.

Age uses the current completed timestamp in UTC and the latest recorded transition
into completed. Missing/contradictory lifecycle data is skipped. Reopening cancels
eligibility; completion again starts a new period. Creation and general update
ages do not count. Canceled/duplicate states do not implicitly count as completed.
Project keep policy disables deletion. Exact organization, team and project UUIDs
bound each invocation. Complete paginated children, parents and both relation
directions must show no unfinished work; uncertainties retain the issue.

Exports retain original IDs, descriptions, comments, lifecycle history, project and
state identity, relationships, and supported PR context. Unsupported binary uploads
or external documents are retained in Linear with a skip reason until a durable
content exporter can preserve them. An expiring URL alone is not accepted. Private
hash manifests and operation journals are atomically synced and reread before
mutation. Export is a readable record, not a tested lossless Linear restore.

A full second snapshot immediately before deletion must match the export. Linear's
schema has no conditional revision argument on `issueDelete`; a read/delete race
remains. Use normal recoverable deletion (`permanentlyDelete: false`). Never infer
deletion from an unavailable lookup or authentication failure. An interrupted
`delete-started` journal blocks further mutation until remote trash state has been
reconciled with `gameskills-linear reconcile --project UUID --issue UUID
--export-dir STORE`. It rereads remote trash state and never deletes. Missing or
inaccessible remote state stays ambiguous. Confirmed deletions are not repeated. Failures never expand the batch.

Report quota impact only when observed. Linear's recovery window after deletion is
separate from our retention age before deletion. API/schema and recovery references:
[official SDK schema](https://github.com/linear/linear/blob/master/packages/sdk/src/schema.graphql),
[GraphQL](https://linear.app/developers/graphql),
[deletion and archives](https://linear.app/docs/delete-archive-issues).
