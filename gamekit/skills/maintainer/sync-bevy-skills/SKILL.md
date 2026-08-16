---
name: sync-bevy-skills
description: Audit and synchronize an installed Bevy Gamekit skill pack to an explicitly selected tag or commit while preserving adopter overlays and surfacing generated-file conflicts. Use when a game already has `.bevy-gamekit/skills.json`. Do not use for first-time installation.
---

# Sync Bevy Skills

Synchronize conservatively from the pinned rendered base.

1. Read `.bevy-gamekit/skills.json`; stop if its schema, source pin, or base snapshot is missing.
2. Check out the requested target tag or SHA. Never resolve a moving `latest`, `HEAD`, `main`, or `master` value, and reject a dirty or mismatched source checkout.
3. Audit without mutation:

   ```sh
   python3 ../../scripts/skills_tool.py sync --target <repo> --revision <tag-or-sha>
   ```

4. Present ADD, UPDATE, DELETE, NO-OP, and CONFLICT classifications. For conflicts, compare pinned base, desired head, and adopter file.
5. Resolve generated-file conflicts explicitly before applying. Never overwrite `.bevy-gamekit/overlays`.
6. After confirmation and a conflict-free audit, rerun with `--apply`. Use `--allow-dirty` only after auditing unrelated changes.
7. Confirm the manifest pin and base snapshot advance together, then run the skill validator.

If an adopter intentionally wants different guidance, place it in the matching overlay rather than editing generated skills.
