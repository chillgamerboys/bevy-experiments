---
name: setup
description: Adopt or deliberately reconfigure GameSkills in a Bevy project, including package selection, pinned installation, project commands and local ownership. Use for initial setup or an installation update; ordinary readiness checks belong to the requested workflow.
---

# Set up GameSkills

Read [project context](../../references/project-context.md). Inspect existing
project instructions, local skills/overlays, configuration, installed packages,
Bevy/GameKit versions and CI before proposing changes.

Select core plus relevant optional packages from actual project needs. Preserve
the game's composition root and local conventions. Detection can recommend a
package; it does not authorize installing every package. Level 2 (refine) is the
creative default; dispatch requires task authorization or applicable standing
permission even when the configured maximum is five workers.

Use the compatible Rust `gameskills` executable's `--help` and read-only `status`/`catalog`/`config`
commands to inspect supported settings. `setup --packages ...` prepares the
configuration proposal; `--apply` installs its embedded baseline. An explicit
`--bundle <immutable-bundle> --apply` installs or rolls back to verified compatible
instructions. Both operations update local configuration, its content lock and
supported project host registration. Package names after `--packages` are
space-separated. Use `setup --recover` after an interrupted update; it restores
the transaction's previous files and refuses to overwrite newer edits.
Use exact command syntax from the runtime, inspect the proposal and preserve
existing owner-controlled values. For an existing Codex pin, `native codex
--register` proposes project registration and `--apply` applies it without repinning;
`--recover` recovers an interrupted registration. Do not overwrite explicit user
disablement or unrelated host settings. Registration is not discovery, and neither
establishes behavioral use. Finish any active queue with its original
runtime before adoption; old queues and evidence remain historical records.

For an installation/update, use the supported host mechanism and an immutable
source identity. Compare installed, previous canonical and new canonical content
before replacing owned files. Preserve local skills, project instructions and
configuration; surface conflicts with concrete paths. Do not follow `latest`,
edit an installed cache as canonical source, or silently migrate an adopter.

Verify selected-package discovery in the actual host and record candidate/client
identities separately from structural checks. For Codex, `native codex
--verify-project` checks ordinary project discovery without injecting enabling
settings. `--verify` checks the generated session-launch configuration only; its
success cannot establish persistent registration or the current session's catalog.
Report a staged-but-undiscoverable installation as an actionable readiness gap,
with the failed boundary and recovery path. Do not silently change host trust or
global settings; state when a fresh session is needed, and identify clients whose
persistent registration is unsupported. Exercise update/recovery/removal
when those are in scope. Report configured choices, observed installation state,
conflicts, compatibility evidence and remaining readiness gaps. A read-only setup
request ends with findings; an authorized adoption continues through verification.

Inspect the adopter docs before proposing `[docs]` or target docs mappings. Keep
its existing layout; mappings need a compatible CLI and must survive setup/update
without overwriting owner choices. Verify `docs resolve` on representative paths.
