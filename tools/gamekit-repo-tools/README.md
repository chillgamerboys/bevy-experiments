# GameKit repository tooling

This internal Rust package owns repository layout/link checks, native and legacy
skill validation, external GameKit consumer verification and migration accounting.
CI selection and its final gate remain Python until R2b; `ci` still fails explicitly.

```sh
cargo run --locked -p gamekit-repo-tools -- contracts check --verify-reference
cargo run --locked -p gamekit-repo-tools --profile ci -- check
cargo run --locked -p gamekit-repo-tools --profile ci -- skills validate
cargo run --locked -p gamekit-repo-tools --profile ci -- skills legacy
cargo run --locked -p gamekit-repo-tools --profile ci -- distribution check --case all
cargo test --locked -p gamekit-repo-tools --profile ci
cargo package --locked -p gamekit-repo-tools
```

`--root` defaults to the current directory and may appear before or after the
subcommand. Every result is one JSON object with `schema_version = 1`; help and
version remain text. A validation failure exits 1, an invalid argument or unavailable
operation exits 2. `skills validate --json` explicitly selects the default JSON
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
one case or `all`. The consumer source is embedded in this crate. Cargo diagnostics
go to stderr; stdout contains the final result. Temporary sources are removed on
success and failure; Cargo artifacts reuse the selected repository's `target/`.
This command needs Cargo and registry dependencies, and may compile Bevy explicitly.
It establishes independent source consumption, not crates.io publication.

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
Serde/JSON, SHA-256, TOML/TOML Edit, Regex and Tempfile. This package is independent
of the GameSkills runner and of Bevy/GameKit/games, and remains `publish = false`.
The declared workspace license is `MIT OR Apache-2.0`; the distribution plan retains
the license-file/notices audit before a public release.
