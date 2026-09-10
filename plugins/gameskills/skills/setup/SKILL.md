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

Use the installed core runtime's `--help` and read-only `status`/`catalog`/`config`
commands to inspect supported settings. `setup --packages ...` prepares the
configuration proposal; `--apply` is the deliberate local configuration mutation.
Use exact command syntax from the runtime, inspect the proposal and preserve
existing owner-controlled values. This helper does not establish native client
installation or behavioral discovery.

For an installation/update, use the supported host mechanism and an immutable
source identity. Compare installed, previous canonical and new canonical content
before replacing owned files. Preserve local skills, project instructions and
configuration; surface conflicts with concrete paths. Do not follow `latest`,
edit an installed cache as canonical source, or silently migrate an adopter.

Verify selected-package discovery in the actual host and record candidate/client
identities separately from structural checks. Exercise update/recovery/removal
when those are in scope. Report configured choices, observed installation state,
conflicts, compatibility evidence and remaining readiness gaps. A read-only setup
request ends with findings; an authorized adoption continues through verification.
