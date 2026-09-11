# GameKit repository tooling

This internal Rust package owns repository layout/link checks, native and legacy
skill validation, external GameKit consumer verification, migration accounting,
committed CI selection, selected command execution and the final CI gate.

```sh
cargo run --locked -p gamekit-repo-tools -- contracts check --verify-reference
cargo run --locked -p gamekit-repo-tools --profile ci -- check
cargo run --locked -p gamekit-repo-tools --profile ci -- skills validate
cargo run --locked -p gamekit-repo-tools --profile ci -- skills legacy
cargo run --locked -p gamekit-repo-tools --profile ci -- distribution check --case all
cargo run --locked -p gamekit-repo-tools --profile ci -- distribution archives --case all
cargo run --locked -p gamekit-repo-tools --profile ci -- bundle check
cargo run --locked -p gamekit-repo-tools --profile ci -- ci select --full
cargo test --locked -p gamekit-repo-tools --profile ci
cargo package --locked -p gamekit-repo-tools
```

`--root` defaults to the current directory and may appear before or after the
subcommand. Results are JSON objects with `schema_version = 1`; help and
version remain text. `ci run` streams child stdout/stderr before its final JSON
result. A validation or CI run/gate failure exits 1; invalid arguments or CI
selection input/output failures exit 2. `skills validate --json` explicitly selects the default JSON
format for scripts migrating from the old validator.

`check` reads manifests and Markdown without Git, Cargo child processes or writes.
It rejects nested workspaces/locks, retired root paths, invalid local links and
capability dependencies pointing at games or back at the facade. Markdown checks
cover inline/image destinations and reference definitions outside matching fences;
they check local targets, not remote availability or Markdown rendering.

`skills validate` checks the fixed 12 core and nine optional skills across six native
packages, metadata, frontmatter and unexecuted scenario rubrics. It rejects duplicate
JSON keys and native-package symlinks. `skills legacy` checks the seven frozen
canonical sources, client metadata and trigger fixtures. Both are structural checks;
they cannot prove agent selection, native activation, instruction-following or savings.
The legacy installer and pinned GameSkills runtime remain their existing Python owners.

`distribution check` stages Cargo-selected library files in a temporary workspace
and checks empty, pure, UI and network consumer graphs and tests. `--case` selects
one case or `all`. Windows Cargo separators are normalized before portable-path
and containment checks. The consumer source is embedded in this crate. Cargo diagnostics
go to stderr; stdout contains the final result. Temporary sources are removed on
success and failure; Cargo artifacts reuse the selected repository's `target/`.
This command needs Cargo and registry dependencies, and may compile Bevy explicitly.
It establishes independent source consumption, not crates.io publication.

`distribution archives` additionally produces actual Cargo library archives in a
temporary library workspace, inspects/extracts them and runs the same consumer cases
against extracted files. Staging adds versions to internal path dependencies and
uses `--exclude-lockfile --no-verify` while retaining `publish = false`. The consumer
uses explicit local patches and the repository lock as a resolution seed. The report
records transformations and archive hashes. These probes do not verify registry
resolution, licenses or publication readiness; library archive lockfiles remain a
release gate. The normal CLI Cargo package retains its lockfile and Cargo verification.

`bundle prepare --revision <full-commit-id>` generates `tools/gameskills-cli/bundle/`
from committed canonical instructions plus this package's `bundle-compatibility.json`.
Commit changed inputs first. `plugins/` remains the human-edited skill source.
The generated directory contains a manifest and deterministic gzip/tar payload, with
the same manifest inside. `bundle check` regenerates in memory and compares both
files, rejecting stale content, dirty inputs, extra outputs and changed bytes.
`source_commit` records the preparation commit; current committed input verification
is always required. When the historical commit is available, its inputs must also
match. If a squash merge removes it from a fresh clone, the report explicitly sets
`source_commit_verified = false`; exact content verification still runs. Other Git
errors and available-but-mismatched commits fail. CI fetches full history to retain
provenance where possible. Unrelated commits keep the payload valid; instruction or
compatibility changes require regeneration at a new preparation commit.
JSON keys, file order, modes and timestamps are canonicalized; payloads use Git blob
bytes, independent of checkout line endings. Generated manifest checkout uses LF.

`bundle export --out <new-archive-path>` writes the exact verified tarball without
overwriting an existing artifact. All six packages' Markdown, catalog and native
metadata are included; Python/runtime files are excluded. The content digest hashes
the canonical manifest identity excluding `source_commit` and `content_sha256`;
the archive digest also binds provenance. Default selection is core-only. CLI range,
schema versions and Bevy/GameKit coverage are declarations, not compatibility tests.
The payload is marked `preparation-only`: its current prose still refers to Python
helpers. R3 must port those instructions and installation before activating it.
No setup state, installed bundle, marketplace registration or executable changes.

After `cargo package --locked -p gameskills-cli`, run `bundle verify-package`.
It inspects `target/package/gameskills-cli-<version>.crate` (or `--archive <path>`),
requires the normal Cargo lockfile and exact prepared bundle bytes, and rejects Python
files. Cargo's preceding package command verifies the extracted build; inspection
alone reports `cargo_build_verified = false` and cannot substitute for that build.

`ci select --base <full-id> --head <full-id>` reads committed Git objects without
Cargo metadata or network access. Missing/uncertain comparisons select all checks;
an invalid tested head fails. It follows reverse consumers at both revisions and
Rust literal includes, and writes the GitHub outputs/step summary when configured.
`--full` and workflow dispatch request every check. Changes to CI code/tests/shared
entrypoints are full-suite inputs; narrative docs and isolated game changes retain
their scoped routing.

Build the controller separately from the Cargo commands it launches:

```sh
cargo run --locked -p gamekit-repo-tools --profile ci --target-dir target/ci-controller -- ci run rust
```

The workflow uses this separate directory for every `ci run` job. Its children
retain the ordinary workspace target directory and caches. This avoids Windows
locking the running controller when workspace tests rebuild that same binary;
an installed controller outside the workspace target also works.

`ci run skills|rust|policy` reads `CI_SELECTION`, checks the current HEAD and runs
ordered literal argument vectors, stopping at the first failed child. Python is
used only for the remaining skill runtime tests; `--python python3` selects an
alternate interpreter. `ci gate` reads `CI_SELECTION` and `CI_NEEDS`; it requires
successful classification and all selected results, allowing only intentional
unselected skips. Missing/malformed/duplicate data fail. The workflow builds the
checked-out Rust gate even if classification fails; bootstrap failures remain red.

The migration checker reads
`tools/migration-contracts.json` and compares complete accounting with the frozen
22-file/142-method inventory embedded from package-local test data. Typed JSON
deserialization rejects duplicate/unknown fields and malformed types. The check
requires dispositions, owners/stages and expected evidence; it does not validate
whether a planned port passed. The result always reports `ports_verified = false`.

`--verify-reference` additionally checks source hashes and test symbols against Git
objects at the accepted reference. Missing history fails explicitly. No Python is
invoked. The frozen Python test declarations use a restricted, verified layout;
the symbol check is not a general language parser. Snapshot JSON and contract
fixtures are data; future executable checks remain Rust.

The toolchain and initial supported minimum are Rust 1.97.1. Dependencies are Clap,
Serde/JSON, SHA-256, TOML/TOML Edit, Regex, Proc Macro 2, Syn, Tempfile, Semver,
Tar and Flate2. This package is independent
of the GameSkills runner and of Bevy/GameKit/games, and remains `publish = false`.
The declared workspace license is `MIT OR Apache-2.0`; the distribution plan retains
the license-file/notices audit before a public release.
