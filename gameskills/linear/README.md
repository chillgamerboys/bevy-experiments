# GameSkills Linear

Optional direct-API helper for explicitly selected programmatic operations.
Normal tracking can use connected Linear MCP tools without this executable. Core
GameSkills does not depend on this executable, its credentials, or a Linear account.

Install from its Cargo archive or `cargo install --path gameskills/linear --locked`.
Requires `curl`, GitHub CLI for PR context/linking, and a personal Linear API key
in the configured environment variable. Cleanup apply currently requires POSIX
private storage. Run `gameskills-linear --help` for the exact command surface.

Configuration lives in an explicit `gameskills-linear.toml`, never a credential
cache. Supply UUIDs for `workspace`, `team`, and default `project`; optional fields
are `key_env` (default `LINEAR_API_KEY`), `retention_days` (30), `export_dir`,
`keep_projects` and `[routes]` mapping repository-relative directories to projects.

## Deferred cleanup prototype

Deletion is deferred and mandatory ticket backups are not part of the accepted
workflow. The code below remains an experimental older implementation, not a
requirement for normal tracking or a recommended cleanup path. Reconcile or remove
it when deletion is selected again.

The prototype cleanup is manual and previews by default. Apply requires an explicit project,
`--limit`, and an existing user-designated backed-up private directory (mode 0700)
outside source repositories and temporary storage. Original records and operation
journals are atomically written, synced and read back before ordinary recoverable
deletion. No scheduler is created. Unknown lifecycle transitions, unfinished
relations, incomplete pages, unsupported attachments and export errors retain
issues in Linear. The initial content exporter retains tickets with binary uploads
or external documents that it cannot durably preserve. Export is not a tested
lossless restore. Linear offers no revision-conditional issueDelete operation;
there remains a documented read/delete race despite full snapshot rechecking.

A failed mutation is never retried automatically. Stable creation UUIDs and
private deletion journals support reconciliation. Capacity recovery is a skill-led
workflow: a verified quota error can route to preview, but never silently authorize
apply or broaden scope. Quota effects require observation, not an assumption that
archiving or deleting increases the allowance.

API source: Linear's official SDK `packages/sdk/src/schema.graphql`, inspected
September 12, 2026, and https://linear.app/developers/graphql. Runtime GraphQL errors,
including partial-data responses, fail closed. Credentials are supplied to curl
through private stdin, never command arguments or logs.
