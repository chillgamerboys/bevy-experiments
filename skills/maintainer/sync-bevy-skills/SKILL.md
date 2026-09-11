---
name: sync-bevy-skills
description: Migrate an existing seven-skill Bevy Gamekit installation to the Rust GameSkills framework while preserving recorded bases and local overlays. Use for legacy adoption; new projects should use gameskills:setup.
---

# Migrate the legacy installation

The former Python `sync` entrypoint is retired. Read the
[GameSkills adoption guide](../../../docs/gameskills.md) and inspect the existing
`.bevy-gamekit/skills.json`, recorded base files, generated client skills and local
overlays. Use the compatible Rust executable:

```sh
gameskills --root <repo> legacy import
```

Inspect the proposal and any concrete conflicts. An authorized migration can apply
the supported `legacy import --apply` operation. Preserve generated files, overlays
and project instructions; adoption does not authorize removing them. Finish active
queues with their original runtime. Verify the selected new packages and actual
client discovery before declaring adoption complete.
