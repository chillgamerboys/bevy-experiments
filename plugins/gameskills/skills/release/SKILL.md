---
name: release
description: Prepare or publish a Bevy game, GameKit library or GameSkills release within the requested scope, verifying actual distributable artifacts and compatibility. Use for release candidates, packaging or authorized publication; a merged PR is not a release.
---

# Release an actual artifact

Read [project context](../../references/project-context.md) and
[delivery](../../references/delivery.md). Establish the requested product/version,
players or consumers, platform/features, distribution channel and endpoint.
Separate a private playable candidate from a supported public release; use the
project's actual compatibility and approval requirements.

Build from known source/dependency identities and record artifact identity and
checksums where useful. Verify the packaged artifact from a clean consumer/run
location. For games, check included assets, startup and the promised play loop.
For libraries, install/consume the selected feature combinations from the actual
candidate. For skills, use the native installed package in each claimed client;
source copying and manifest validity cannot prove discovery or behavior.

Collect relevant engineering, native interaction and human feedback evidence for
the promised support. Check licenses/provenance, migration instructions and recovery
or rollback expectations. Do not claim platforms or modes based solely on a
capability compile check. Use `gameskills-maintainer:evaluate-skills` for skill
promotion when available.

Prepare accurate release information and concrete artifacts first. Publish/tag/upload
only within existing authorization and receiving-project rules; Bevy upstream
release prose must remain human-authored. Query the channel after publication and
record observed availability and immutable identity. On ambiguous results, inspect
before retrying; do not replace an existing published version to hide a failure.

Report what can actually be installed or played, its verified versions/platforms,
remaining limitations and publication state. A prepared release outside publication
scope is a valid handoff; it is not a published release.
