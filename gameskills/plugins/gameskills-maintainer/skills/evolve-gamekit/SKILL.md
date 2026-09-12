---
name: evolve-gamekit
description: Design or refine a reusable GameKit capability, public contract and consumer migration from demonstrated Bevy game needs. Use for shared library boundaries, feature compatibility or API evolution; game-specific rules and presentation stay with their game.
---

# Evolve a useful GameKit capability

Read [project context](../../references/project-context.md) and
[capability design](../../references/capability-design.md). Establish the concrete
consumer problem and inspect the owning source/API, current examples and affected
consumers. Check existing Bevy/ecosystem options when the task proposes a new
capability, rather than assuming the engine lacks it.

Define the smallest useful optional contract, dependency direction, public seams,
errors/lifecycle and compatibility. Keep game composition, rules and branding local.
Use demonstrated needs to justify extraction; do not invent a generic framework
or transfer a convenience package upstream merely because it is reused.

Implement or propose according to the request's creative level and endpoint. Treat
API changes as consumer migrations: inspect feature graphs, production wiring and
retained behavior across affected games. Keep authoritative API documentation in
Rustdoc and runnable examples; reference actual source rather than stale manuals.

Verify capability tests, promised feature combinations and consumer seams. Distinguish
workspace/source probes from packaged-artifact installation. Include migration and
rollback/removal conditions where contracts change. Shared UI or networking claims
need their specialist evidence, not compilation alone.

Deliver the capability and consumer changes with actual compatibility results and
remaining gaps. Feed an observed guidance defect into `author-skill` when useful;
ordinary library work does not require changing skills or preparing Bevy proposals.
