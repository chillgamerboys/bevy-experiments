# GameSkills

GameSkills is GameKit's development companion for Bevy games. The default package
has **13 core skills**, with **`gameskills:plan` as the daily entrypoint**:

`setup`, `plan`, `grill`, `dispatch`, `debug`, `test`, `playtest`, `review`, `update-docs`,
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
| `gameskills-linear` | `track`, `cleanup` |

Each package has one canonical source under [`plugins/`](plugins), with native
Codex and Claude manifests. Optional packages are explicitly selected; the default
installation does not include all 24 skills. The contribution package is reserved
for human-led investigation of verified Bevy bugs or compelling engine-level gaps.

Start with the [installation and workflow guide](docs/development.md),
[complete contracts](docs/decisions/gameskills-catalog.md) and
[Bevy companion direction](docs/decisions/gameskills-framework.md).

The [workflow reliability and optional Linear plan](docs/decisions/workflow-reliability-and-linear.md)
tracks the next audit, legacy consolidation, planning grill, and retention pilot.
Implementation and evaluation are in progress; installed pins are updated explicitly.

## Validation

Rust 1.97.1 builds the tools and Git supplies repository identity. Prebuilt CLI
execution needs no compiler or interpreter. Queue mutation and command supervision
currently require POSIX support.

```sh
cargo run --locked -p repo-devtools --profile ci -- skills validate
cargo test --locked -p gameskills-cli --profile ci
```

Structural checks, real process/worktree tests and native agent behavior establish
different claims. Scenario fixtures are evaluation inputs, not evidence that an
agent passed them. See the implementation guide for candidate evidence and limits.

## Legacy migration

`devtools/tests/fixtures/legacy/source/`, `devtools/tests/fixtures/legacy/references/` and their trigger fixtures are frozen compatibility
material for existing seven-skill adopters. They are not additional core workflows.
The Python installer and sync entrypoints are retired. Use `gameskills legacy import`
to inspect an existing installation, then deliberately apply the Rust installation.
The [lesson migration map](plugins/gameskills-maintainer/references/legacy-migration.md)
records what was retained, rewritten or retired.

Preserve generated client files, `.bevy-gamekit/overlays/` and recorded base snapshots.
The importer checks their identity and does not delete local guidance. Finish active
queues with their original runtime; old evidence remains historical. The retired
[install](../devtools/tests/fixtures/legacy/maintainer/install-bevy-skills/SKILL.md) and
[sync](../devtools/tests/fixtures/legacy/maintainer/sync-bevy-skills/SKILL.md) guidance redirects to this migration path.

The [CLI](cli/README.md), [plugins](plugins) and [legacy compatibility](../devtools/tests/fixtures/legacy/README.md)
have separate owners. Active catalog evaluation fixtures live with the devtools tests.

The optional `gameskills-linear` package adds `track` and `cleanup`. Its separately
installed Rust helper owns credentials and manual retention sweeps; installing core
does not connect Linear or schedule deletion. See the helper's source README and
the plugin's setup reference for the supported configuration.
