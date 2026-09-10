---
name: create-pr
description: Deliver an implemented Bevy/GameKit/GameSkills change as a concrete pull request, or update its existing PR. Use when PR preparation/publication is requested or already authorized; review acceptance and merging are separate workflows.
---

# Create the review artifact

Read [project context](../../references/project-context.md) and
[delivery](../../references/delivery.md). Confirm the actual repository, target
branch, source branch/HEAD and requested endpoint. Inspect local changes and the
remote for an existing PR for this work before creating a duplicate.

Prepare logical commits containing only the authorized change and relevant docs.
Preserve other contributors' work. Verify applicable checks and identify missing
manual/CI evidence honestly. Use the receiving project's PR template when present;
describe the final problem, resulting behavior, important tradeoffs and validation
for a reviewer who has not seen the conversation. Rewrite stale scope/history.

Inspect the concrete commits/body before publication. When pushing and creating
or updating this PR are covered by the session, proceed with the configured
provider/CLI; otherwise finish preparation and identify the specific remaining
authorization. Do not infer authorization to contact reviewers or post additional
communications. Bevy upstream requires its optional human-led contribution path.

Observe publication through the provider: confirm PR URL/number, repository,
source HEAD and base. After an uncertain result, inspect remote state before retrying.
Record the actual PR and remaining checks. Invoke `gameskills:audit-pr` when the
active endpoint includes acceptance; creating the artifact cannot declare it
accepted, merged or released.
