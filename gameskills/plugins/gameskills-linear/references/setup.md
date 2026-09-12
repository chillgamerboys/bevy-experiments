# Optional Linear setup and routing

Select `gameskills-linear` explicitly alongside core through supported GameSkills
setup. Install the separate `gameskills-linear` Rust executable from the same
candidate Cargo archives. Core-only adoption needs neither this binary nor a key.
The executable uses the official GraphQL endpoint via `curl`; PR observations and
links use authenticated `gh`. It never reads a connector's interactive credentials.

Keep a personal API key in the configured environment variable (default
`LINEAR_API_KEY`), never in source or a command argument. A connected Linear MCP
session does not automatically authenticate a separate executable. Missing delete
capability in MCP requires this supported direct API path; never mine token caches.

Create an explicit `gameskills-linear.toml`:

```toml
workspace = "ORGANIZATION_UUID"
team = "TEAM_UUID"
project = "DEFAULT_PROJECT_UUID"
key_env = "LINEAR_API_KEY"
retention_days = 30
keep_projects = []
# export_dir = "/absolute/backed-up/private/directory"

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

A verified quota failure can route to a scoped cleanup preview. Unknown GraphQL,
authentication and rate-limit errors never count as quota proof. Current transport
fails these closed and leaves classification to the supported provider's explicit
error observation. After authorized verified cleanup, reconcile the original
create UUID and retry once. Report remaining capacity uncertainty without looping.
