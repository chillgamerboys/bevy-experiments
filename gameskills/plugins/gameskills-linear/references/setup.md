# Optional Linear setup and routing

Select `gameskills-linear` explicitly alongside core. Use the connected Linear
MCP for normal lookup, create/reuse, updates and two-way PR links. No extra API key
is required for those connector operations. Resolve current identities through
supported tools; never extract credentials from an interactive connector.

The optional standalone helper uses direct GraphQL via curl and authenticated gh.
It requires separate configuration when deliberately selected. Its observer is not
a bridge to MCP authentication. An adopter using it must report missing credentials
as an observation gap, not as evidence a ticket or link does not exist.

Deletion is deferred; see [helper status](cleanup.md). No export directory or
live deletion pilot is required. The following config belongs to the optional
helper, not a prerequisite for using the connector:

Create an explicit `gameskills-linear.toml`:

```toml
workspace = "ORGANIZATION_UUID"
team = "TEAM_UUID"
project = "DEFAULT_PROJECT_UUID"
key_env = "LINEAR_API_KEY"
retention_days = 30
keep_projects = []


[routes]
"games/labyrinth" = "LABYRINTH_PROJECT_UUID"
```

Replace placeholders with exact verified UUIDs. `route PATH` selects the longest
matching directory. Mixed/shared work uses the repository's default unless the
user chooses separate deliverables. The team prefix never selects a project.
For this repository, Bevy Games owns shared work and Labyrinth owns that game;
Hex owns the separate bevy-hex-game backlog. Keep the shared HEX prefix.

For required tracking in core `gameskills.toml`:

```toml
[tracking]
required = true
observer = ["gameskills-linear", "observe", "--config", "gameskills-linear.toml"]
```

The observer rereads the exact issue/project/PR links in both directions. Bind the
issue and project UUIDs to the existing core delivery task. `link --issue UUID
--project UUID --pr URL` attaches the PR and adds the issue URL to its body.
`create --id UUID --title TEXT --description-file FILE` uses a persisted caller
UUID and reconciles it before creating. Never regenerate the ID after an ambiguous
response. `complete --issue UUID --project UUID --state UUID --pr URL` requires all
required PRs as repeated flags; the caller still owns completeness and acceptance.
A closed unmerged PR cannot complete an issue.

If issue capacity prevents creation, report the exact provider failure. Deletion
is deferred; do not route an ordinary tracking task into automatic cleanup or
request backup storage. Reconcile ambiguous creates before any bounded retry.
