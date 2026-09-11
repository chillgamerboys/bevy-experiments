# GameSkills Rust foundation

This unpublished `0.1.0-dev.2` candidate implements help/version and read-only
configuration validation. Installation, native clients, queues, execution and
evidence remain on the existing pinned candidate; their Rust commands fail with
`not_implemented`. The internal Rust library is not a supported public API.

```sh
cargo run --locked -p gameskills-cli -- --help
cargo run --locked -p gameskills-cli -- config validate --file gameskills.toml
cargo test --locked -p gameskills-cli --profile ci
cargo package --locked -p gameskills-cli
```

`config validate` is an additive command that needs neither setup nor a Git checkout.
It reads the named ordinary UTF-8 TOML file and reports normalized structure only.
Relative file paths resolve against `--root`, which defaults to the current directory.
It never executes configured commands, writes an installation or establishes client
readiness. The existing bare `config` command is not ported and does not bypass its
installation requirements. Help/version work outside a repository.

Results use JSON with `schema_version = 1`; errors have `error.code` and
`error.message`, exit 2. Help/version are text with exit 0. Broken output also exits
2. Diagnostic wording is not a compatibility promise. Source fixtures record prior
configuration observations and deliberate changes: strict integer schema, typed
project/target fields, portable target paths, duplicate target selection rejection
and JSON-compatible scalars. TOML 1.1 syntax is accepted by the Rust parser.

The toolchain and initial supported minimum are Rust 1.97.1, tested together rather
than asserting an untested older MSRV. Direct dependencies are Clap, Serde JSON and
TOML; there are no Bevy, GameKit, game or repository-tool dependencies. The workspace
lockfile records exact resolutions. `cargo install --locked --path <extracted-crate>`
can install the packaged foundation; prebuilt distribution and embedded instructions
arrive in later stages. `process_probe` is a test example, not an installed binary.

The workspace declares `MIT OR Apache-2.0`, inherited here. Actual license files,
notices and registry ownership still need the planned release audit; publication
remains disabled. The package contains its own sources and test data, without
workspace-relative runtime resource paths or build-time downloads.
