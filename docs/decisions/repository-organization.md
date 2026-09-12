# Repository organization and Rust naming refactor

Status: implemented and locally verified September 12, 2026. The owner selected
root-level Gamekit, GameSkills and games, with hyphenated Cargo package names.
The accepted sequence and mappings below are retained as the migration record;
verification results and current limits follow at the end.

## Outcome and ownership

Make the reusable library, its development companion and their adopting games
easy to navigate and eventually separate into repositories. Gamekit and GameSkills
will stay together in that future split. Each game owns its rules, composition,
presentation, assets and game-specific documentation.

Use two implementation passes: directory/documentation organization first, Rust
package and command naming second. Each pass must have a verified checkpoint.
Keep one root Cargo workspace and lockfile for now, with Labyrinth as the default
application. Gameplay changes, new shared abstractions, dependency upgrades,
repository extraction and publication are outside this refactor.

## Target layout

```text
gamekit/
  README.md
  facade/
  hex/
  turns/
  ui/
  testing/
  session/
  discovery/
  multiplayer/
  docs/
gameskills/
  README.md
  cli/
    bundle/                 # Generated, self-contained instruction artifact
  plugins/                  # Canonical authored native plugin packages
  legacy/                   # Frozen compatibility material and migration redirects
  docs/
games/
  labyrinth/
    Cargo.toml
    src/
    rules/                  # Game-owned, pure Rust Cargo package
    examples/
    docs/
  carterfight/
  deckbuilder/
devtools/                   # One internal Rust package: checks, packaging and CI
  src/
  tests/
  README.md
  migration-contracts.json
  bundle-compatibility.json
docs/                       # Cross-product development, decisions and history
Cargo.toml
Cargo.lock
gameskills.toml
gameskills.lock.json
```

Retain required root integration/configuration such as `.github/`, `.agents/`,
`.claude-plugin/`, Rust policy files and the root README. Local `.context/`,
`.gameskills/` and `target/` keep their current purpose. Directory names describe
ownership; they do not have to equal Cargo package names.

## Directory mapping: pass one

| Current owner | Destination |
|---|---|
| `crates/bevy_gamekit/` | `gamekit/facade/` |
| `crates/bevy_game_hex/` | `gamekit/hex/` |
| `crates/bevy_game_turns/` | `gamekit/turns/` |
| `crates/bevy_game_ui/` | `gamekit/ui/` |
| `crates/bevy_game_test/` | `gamekit/testing/` |
| `crates/bevy_game_session/` | `gamekit/session/` |
| `crates/bevy_game_discovery/` | `gamekit/discovery/` |
| `crates/bevy_game_multiplayer/` | `gamekit/multiplayer/` |
| `tools/gameskills-cli/` | `gameskills/cli/` |
| `plugins/` | `gameskills/plugins/` |
| `tools/gamekit-repo-tools/` | `devtools/` |
| `tools/migration-contracts.json` | `devtools/migration-contracts.json` |
| `games/deckbuilder_ui/` | `games/deckbuilder/` |
| `skills/source/`, `skills/references/`, `skills/maintainer/` | Corresponding subdirectories of `gameskills/legacy/` |

Rewrite `skills/README.md` as the current `gameskills/README.md`; add a dedicated
legacy README. Audit `skills/tests/` against the catalog fixtures already in the
repository tool: keep active fixtures with their validating tests, retain distinct
legacy trigger inputs under `gameskills/legacy/`, and consolidate exact duplicates
only after checking their consumers. Generated Python caches are not source moves.

Labyrinth and Carterfight keep their application directory names. Labyrinth's
`rules/` remains inside the game; Deckbuilder and Carterfight retain their local
rules/backend modules. Internal game-module redesign is a separate task.

## Cargo package mapping: pass two

| Current package | Final package | Rust library identifier, where applicable |
|---|---|---|
| `bevy-gamekit` | `bevy-gamekit` | `bevy_gamekit` |
| `bevy_game_hex` | `bevy-gamekit-hex` | `bevy_gamekit_hex` |
| `bevy_game_turns` | `bevy-gamekit-turns` | `bevy_gamekit_turns` |
| `bevy_game_ui` | `bevy-gamekit-ui` | `bevy_gamekit_ui` |
| `bevy_game_test` | `bevy-gamekit-testing` | `bevy_gamekit_testing` |
| `bevy_game_session` | `bevy-gamekit-session` | `bevy_gamekit_session` |
| `bevy_game_discovery` | `bevy-gamekit-discovery` | `bevy_gamekit_discovery` |
| `bevy_game_multiplayer` | `bevy-gamekit-multiplayer` | `bevy_gamekit_multiplayer` |
| `gameskills-cli` | `gameskills-cli` | `gameskills_cli` |
| `gamekit-repo-tools` | `repo-devtools` | `repo_devtools` |
| `labyrinth` | `labyrinth` | `labyrinth` |
| `labyrinth_rules` | `labyrinth-rules` | `labyrinth_rules` |
| `deckbuilder_ui` | `deckbuilder` | `deckbuilder` |
| `carterfight` | `carterfight` | `carterfight` |

Use canonical hyphenated dependency keys in Cargo manifests and the corresponding
underscore identifiers in Rust. Update imports instead of introducing permanent
aliases for old capability names. Preserve facade module names such as
`bevy_gamekit::ui`, `::testing` and `::session`, and existing public feature names.

The GameSkills executable remains `gameskills`. Rename the internal executable
`gamekit-repo` to `repo-devtools`; the Deckbuilder executable becomes `deckbuilder`.
Update executable discovery and Cargo's `CARGO_BIN_EXE_*` test references together.
Native skill/plugin identifiers such as `gameskills-ui` retain their current names.
These are local naming decisions, not claims of registry name availability.

## Documentation ownership

- `gamekit/README.md` introduces capability selection and library adoption;
  `gamekit/docs/` owns library design and capability development guidance. Public
  APIs remain documented in Rustdoc rather than copied into another manual.
- `gameskills/README.md` introduces the current CLI and plugins;
  `gameskills/docs/` owns installation, workflow, catalog design, compatibility
  and skill-development guidance. Move `docs/gameskills.md` and the smoke-test
  guide here, along with relevant GameSkills decisions.
- `games/labyrinth/docs/` owns its architecture, repeatable testing instructions
  and footprint/death decision. Keep dated walkthroughs and test reports in a
  clearly marked history subdirectory, with their original revision and limits.
  Apply the same ownership principle to the other games without imposing extra
  directories where a short README is sufficient.
- `devtools/README.md` owns internal command usage and test/fixture conventions.
- Root `docs/` retains the workspace development guide, cross-product boundaries,
  coordinated distribution direction and this plan. Split mixed documents such as
  architecture, multiplayer and testing by their actual owner. Consolidate current
  roadmaps; move completed handoffs and migration narratives into marked history.

Update links and current command examples. Preserve historical facts, old package
names in historical observations, and pinned commit/source identities. Clearly
supersede the flat-layout requirement in decision 0001 and replace the outdated
extraction recipe with a future split checklist covering both `gamekit/` and
`gameskills/`, relevant `devtools/`, metadata, documentation and root configuration.
Do not carry out Git history filtering in this task.

## Execution sequence

### 0. Record the baseline

Record the current commit, worktree status, Cargo package/feature graph and rules
fingerprint. Inventory references with source-aware searches across manifests,
Rust, Markdown, JSON, TOML and workflow files. Classify each as a live repository
path, archive/install path, historical source path, or generated output.

Run existing repository/skill checks and appropriate baseline tests. Reuse valid
observations where possible; record pre-existing failures rather than attributing
them to the move. Inspect installation readiness only if running the installed
GameSkills workflow; a missing executable does not block source-level planning.

### 1. Relocate owners and update path-dependent infrastructure

Perform the directory mapping while retaining current Cargo package and executable
names. Update workspace membership, path dependencies, GameSkills targets/commands,
root marketplace source paths, documentation links and include/fixture paths.
Use explicit package membership, or equivalent manifest-aware discovery, so new
README/docs directories are never treated as crates.

Adapt repository policy to recognize the new owners: it currently rejects root
`gamekit/` as retired and recognizes capabilities only under `crates/`. Preserve
checks against capability-to-game and capability-to-facade dependencies, nested
workspaces and duplicate lockfiles. Centralize repeated live-layout constants in
devtools where useful without creating an adopter runtime dependency on devtools.

Update distribution staging, crate discovery, facade lookup, package contents,
CI path classification and fixture inputs. CI must recognize renames using both
base and head layouts; it must not skip checks because the old paths disappeared.
Keep conservative fallback coverage for uncertain changes.

Update the migration ledger's live destination paths and its destination validator,
which currently requires `tools/`. Keep its frozen historical source inventory,
reference commit, hashes and original source paths unchanged. Verify the historical
reference check still works after moving its current implementation and fixtures.

### 2. Preserve the instruction artifact contract

Canonical source moves to `gameskills/plugins/`; archive and installed paths retain
`plugins/<package>/...`. Explicitly map repository source paths to artifact paths
in bundle preparation, provenance validation and source-identity tests. Do not
globally replace `plugins/` in the runtime archive reader or generated installed
marketplace files. Root source marketplace files do need the new source paths.

Move the package-local generated bundle with the CLI. Preparation currently requires
committed canonical inputs: during implementation, establish a local source commit,
then regenerate the payload from that revision and check it before completing the
pass. Preserve real provenance and deterministic generation rather than bypassing
the committed-input check. Include the resulting artifact update in the reviewed
pass; no public release or remote push is required for generation.

Keep instruction/installation/queue schemas and native plugin identities unless
verification demonstrates a necessary compatibility change. Existing immutable
installed bundles, locks, overlays, queues and evidence are not rewritten by a
source move. Exercise update/install behavior in disposable adopter repositories;
old evidence stays tied to its original input identity.

### 3. Complete and verify pass one

Finish documentation ownership and current navigation. Verify all package paths,
source and archive consumers, canonical/legacy skill validation, bundle generation,
CLI packaging and CI routing. Build and start each game from both the root and its
package directory to catch working-directory and asset path assumptions.

### 4. Rename Cargo packages, imports and commands

Apply the package mapping after pass one is stable. Update workspace dependencies,
feature references, capability imports, executable names, test probes, distribution
allowlists, CI selection, config commands, instruction guidance and documentation.
Let Cargo update the lockfile while retaining external dependency versions; inspect
the resulting graph for accidental upgrades or feature changes.

Regenerate instruction artifacts again if authored instructions or compatibility
inputs change, following the same committed-input/provenance procedure. Verify the
final names through standalone consumer manifests as well as workspace builds.

## Verification and completion

Each pass gets focused tooling/packaging checks and a workspace compatibility gate.
Tests should exercise boundaries that can break during moves or renames; avoid
adding tests merely to assert that a file was moved.

| Claim | Required evidence |
|---|---|
| Workspace is coherent | Locked Cargo metadata, formatting, strict all-target/all-feature Clippy, workspace tests and doctests; inspect external dependency versions |
| Games retain ownership and behavior | Dependency-direction checks, unchanged Labyrinth rules fingerprint, existing game tests, built-in examples and bounded startup from root/package directories |
| Gamekit works without games | Existing empty, pure, UI and native feature consumer cases against source-selected and extracted Cargo archive inputs |
| CLI remains self-contained | Actual CLI Cargo archive builds; bundle contents/digests verify; install selected core/specialist packages from packaged artifacts in disposable adopters |
| Skills remain discoverable | Canonical and legacy validation, both source marketplace manifests, generated installed manifests and available native discovery checks |
| CI follows the refactor | Routing tests for moved/renamed packages, dependent games, plugins, docs and devtools; preserve uncertain-input fallback and final-gate behavior |
| Historical contracts remain verifiable | Migration accounting including frozen reference verification and cutover checks |
| Documentation is usable | Local-link checks, reviewed root/product entry points, correct current commands and clearly labeled historical records |

Use the existing scoped CI matrix for platform-specific verification. Do not claim
remote CI or native interaction passed from a local build. These structural changes
need asset/startup checks, not a new gameplay-design or balance acceptance study.
Registry resolution and cross-machine networking retain their existing evidence
limits; archive staging is not registry publication.

Completion means both passes satisfy the gates; active references use final paths
and names; intentional historical/archive spellings are documented; adopters need
no repository-layout knowledge; and all three games remain independently runnable.
Report actual checks, failures and remaining limits at the final source revision.

Use separate reviewable commits for the passes and their generated artifacts.
Recovery is a normal revert of the affected pass, not deletion of local state or
history rewriting. This plan creates no implementation queue and performs no game
changes, package publication, repository split or linked-workspace migration.

## Implementation result — September 12, 2026

The baseline was `74e4955b159b5f4263302fe6aebf10825f326bc5`. Directory ownership
was committed in `46e8dd2`, with its generated instruction provenance in `0a6840b`.
Package/import naming was committed in `0531ca2`, followed by the updated skill
bundle in `c5bc54e`. All 14 Cargo packages retain their feature and dependency
contracts after normalizing the intended names; all 694 external package
name/version/source/checksum identities match the baseline lockfile.

The active catalog scenario fixture now lives once under
`devtools/tests/fixtures/catalog/`; the legacy trigger fixture remains with legacy
source. Old root Python caches were moved into ignored local context, not copied
into either product. The migration ledger retains its original reference commit
and frozen 22-file/142-test inventory while pointing to the relocated Rust owners.

The CLI can export from both historical and current source layouts. Authored
instructions live under `gameskills/plugins/`; archives and installations retain
`plugins/<package>/...`. Pass one preserved the instruction content digest exactly.
Pass two updated capability-name guidance and generated digest
`3d876fd981642ff0b7b4cb9dc81270bb9f9a944b828467d38f75896a56c3364a`.
Existing project installation locks, bundles, queues and prior evidence were not
rewritten, and the linked adopter workspaces were not migrated.

Local macOS verification:

- Workspace all-feature game, library, CLI and devtools tests and doctests passed
  across the full run and focused reruns after correcting generated manifest keys
  and renamed test fixtures. The final embedded bundle's installation tests were
  rerun separately. No failing target remains from that run.
- Formatting, strict workspace all-target/all-feature Clippy, dependency policy,
  repository/link checks and current/legacy skill validation passed.
- Source-selected and actual extracted Cargo archive consumers passed empty, pure,
  UI and native-network feature cases. CLI and devtools Cargo packages built from
  their packaged sources; packaged instruction bytes/provenance verified.
- The actual packaged CLI was installed into an isolated local prefix. A fresh
  disposable adopter installed the core, added UI/turn-based specialists, checked
  readiness and exported the portable bundle. Native Codex discovery passed without
  starting a model task. Both native-client manifests pass structural checks.
- Each game built with its examples and started for six seconds from both the root
  and its package directory, with no observed startup/asset errors. Labyrinth used
  isolated local profiles. These are startup checks, not interactive visual review.
- The explicitly enabled six-process Labyrinth guest-kill/reconnect test passed.
  Minimal discovery, multiplayer and testing configurations and standalone UI tests
  passed. Browser-compatible hex, turns, session, UI and rules packages passed the
  WebAssembly check after installing the missing target for the existing toolchain.
- CI routing tests cover the current plugin/docs owners and a historical-to-current
  directory move. Selection against the real baseline conservatively requests the
  full matrix because shared manifests and workflow inputs changed.

Labyrinth's rules fingerprint is unchanged:
`0a1fdad6fcc990599e90a545c1ab40e4d313b5de1ce51060f70d8cc5cb2142e9`.
No combat, presentation or session behavior was redesigned in this refactor.
Local execution logs and contract comparisons are under
`.context/reorganization/`; build/install artifacts are under `target/`.

Remote Linux/Windows CI, authenticated Claude behavior, registry publication and
cross-machine multiplayer were not verified by this local refactor. Archive tests
still use explicit local patches for unpublished siblings and do not claim registry
resolution. No remote push, release or repository extraction was performed.
