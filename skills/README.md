# GameSkills

GameSkills is GameKit's development companion for Bevy games. The default package
has **12 core skills**, with **`gameskills:plan` as the daily entrypoint**:

`setup`, `plan`, `dispatch`, `debug`, `test`, `playtest`, `review`, `update-docs`,
`create-pr`, `audit-pr`, `merge-pr`, `release`.

`setup` handles initial adoption. `plan` investigates the task, defines the useful
work and evidence, and routes into the toolbox. Authorized independent work can
use `dispatch`, including its add-only injection mode, with up to five workers.
The invoking agent carries work through the endpoint the user requested.

| Optional package | Skills |
|---|---|
| `gameskills-ui` | `build-ui`, `verify-ui` |
| `gameskills-turn-based` | `model-rules` |
| `gameskills-multiplayer` | `design-multiplayer`, `verify-multiplayer` |
| `gameskills-maintainer` | `evolve-gamekit`, `author-skill`, `evaluate-skills` |
| `gameskills-bevy-contrib` | `prepare-contribution` |

Each package has one canonical source under [`plugins/`](../plugins), with native
Codex and Claude manifests. Optional packages are explicitly selected; the default
installation does not include all 21 skills. The contribution package is reserved
for human-led investigation of verified Bevy bugs or compelling engine-level gaps.

Start with the [installation and workflow guide](../docs/gameskills.md),
[complete contracts](../docs/decisions/gameskills-catalog.md) and
[Bevy companion direction](../docs/decisions/gameskills-framework.md).

## Validation

Python 3.11 or newer and Git are required; repository checks use the standard
library. The queue and command runner currently require macOS or Linux.

```sh
python3 skills/scripts/validate_gameskills.py
python3 -m unittest discover -s skills/tests -v
```

Structural checks, real process/worktree tests and native agent behavior establish
different claims. Scenario fixtures are evaluation inputs, not evidence that an
agent passed them. See the implementation guide for candidate evidence and limits.

## Legacy migration

`source/`, `references/`, the old installer and its tests are frozen compatibility
material for existing seven-skill adopters. They are not part of the new default
offering and receive no new feature work. Keeping the old updater during migration
preserves local overlays and conflict detection; it does not retain seven extra
core workflows. The [lesson migration map](../plugins/gameskills-maintainer/references/legacy-migration.md)
records what was retained, rewritten or retired.

Do not delete an adopter's generated files or `.bevy-gamekit/overlays/` blindly.
Compare their existing installation with its recorded source, preserve local
changes, and retire the legacy installation only after the selected new packages
work in that adopter. The old [install](maintainer/install-bevy-skills/SKILL.md)
and [sync](maintainer/sync-bevy-skills/SKILL.md) workflows remain available for that
transition; they are not the new adoption entrypoint.
