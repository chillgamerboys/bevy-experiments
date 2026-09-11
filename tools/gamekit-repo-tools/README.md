# GameKit repository tooling foundation

This internal package implements the migration contract checker. It does not yet
replace layout, catalog, distribution or CI commands; those names fail explicitly.
The repository continues to use its existing Python checks until R2.

```sh
cargo run --locked -p gamekit-repo-tools -- contracts check --verify-reference
cargo test --locked -p gamekit-repo-tools --profile ci
```

`--root` defaults to the current directory. The checker reads
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
Serde, Serde JSON and SHA-256, with Tempfile for tests. This package is independent
of the GameSkills runner and of Bevy/GameKit/games, and remains `publish = false`.
The declared workspace license is `MIT OR Apache-2.0`; the distribution plan retains
the license-file/notices audit before a public release.
