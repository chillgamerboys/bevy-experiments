---
name: install-bevy-skills
description: Audit and install the complete version-pinned Bevy Gamekit skill pack into a Codex and Claude game repository. Use from a checked-out Gamekit source when adopting the seven Bevy 0.19 craft skills for the first time. Do not use to update an existing adoption; use sync-bevy-skills.
---

# Install Bevy Skills

Install only after showing the audit to the user.

1. Check out the explicit release tag or commit SHA. Reject `latest`, `HEAD`, moving branches, a mismatched checkout, or uncommitted canonical skill files.
2. Verify the target is a Git repository and inspect its dirty state and existing `.agents`, `.claude`, and `.bevy-gamekit` paths.
3. Run the tool without `--apply`:

   ```sh
   python3 ../../scripts/skills_tool.py install --target <repo> --repository <source-url> --revision <tag-or-sha>
   ```

4. Present every ADD, NO-OP, or CONFLICT. Do not infer permission from the request to inspect.
5. After confirmation, rerun with `--apply`. Use `--allow-dirty` only after identifying every unrelated dirty file.
6. Verify both client layouts, `.bevy-gamekit/skills.json`, the pinned base snapshot, and an unchanged overlay directory.
7. Run `python3 ../../scripts/validate_skills.py` from the Gamekit checkout.

The tool installs all seven craft skills. Game-specific rules belong in `.bevy-gamekit/overlays/<skill-name>.md`.
