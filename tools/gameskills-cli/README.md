# GameSkills Rust CLI

The unpublished `0.1.0-dev.2` candidate installs immutable GameSkills instructions,
constructs native Codex/Claude invocations, coordinates durable work queues and
executes configured command graphs with verifiable evidence. Its 12 core skills
use `plan` as the default entrypoint; nine optional skills live in five specialist
packages. The internal Rust modules are not a stable public API.

```sh
cargo install --locked --path tools/gameskills-cli
# In the adopter's Git root:
gameskills catalog
gameskills setup
gameskills setup --apply
gameskills status
gameskills native codex --verify
```

Initial setup uses the instruction snapshot embedded in the executable. It needs
no source checkout, network bundle fetch, Cargo or interpreter at runtime. Git is
required for repository operations. The source installation needs Rust 1.97.1;
prebuilt candidates are target-specific. There is no Bevy, GameKit or repository-tool
runtime dependency. Configuration and instructions remain TOML, JSON and Markdown.

`setup` reports a proposal until `--apply` is supplied. Core-only installation is
the default. `--packages gameskills gameskills-ui` selects an explicit combination.
Use `setup --bundle PATH --apply` for a compatible immutable update or rollback,
and `setup --recover` to recover an interrupted config/lock transaction. Local
instructions, overlays and previous bundles retain their ownership. Updates that
conflict with newer local edits fail with paths to inspect.

An interrupted queue write may leave a `.queue-<pid>-<serial>.tmp` file. Setup and
recovery preserve recognized ordinary files under the queue lock without treating
their incomplete contents as queue state. Unknown entries and unsafe files still
block the operation, as do unfinished queues and active command runs.

`config validate --file PATH` checks standalone TOML without setup or Git. Bare
`config` requires installation readiness. Global `--root` defaults to the current
directory. `catalog`, help and version do not require an installation. `native`
prints literal client arguments; `--launch` starts the selected client and
`native codex --verify` checks bounded app-server discovery without a model turn.
Neither structural validation nor discovery proves model behavior.

Plans retain authored schema 1; Rust installation locks, queues and evidence use
schema 2 with an explicit runtime identity. Finish active Python queues with their
original runtime before adoption. Existing records and bundles remain untouched;
old observations cannot become validated Rust passes. Rerun checks with Rust to
obtain new evidence. `legacy import` preserves the old seven-skill installation
and project overlays while proposing the new installation.

Queue operations, command supervision and native Codex verification currently require
POSIX support. Windows receives explicit unsupported diagnostics for those operations;
portable configuration, catalog, packaging and installation checks have separate
coverage. Authenticated native-client behavior and game playtests are separately
recorded acceptance evidence, not inferred from an operating-system test matrix.

```sh
cargo test --locked -p gameskills-cli --profile ci
cargo clippy --locked -p gameskills-cli --all-targets --profile ci -- -D warnings
cargo package --locked -p gameskills-cli
```

Successful commands return JSON; diagnostic failures have `error.code` and
`error.message` with exit 2. A failed execution observation exits 1. Help/version
are text. Native launch preserves its process status. Diagnostic wording and the
internal supervision protocol are not public compatibility promises.

`bundle/` contains a generated manifest and deterministic archive from canonical
`plugins/` instructions. Maintainers regenerate with `gamekit-repo bundle prepare`
and verify with `bundle check`; Cargo packages carry those exact bytes. No build
script downloads content and no runtime path points back into this workspace.
Compiled examples are test probes, not additional installed programs.

Publication remains disabled. The workspace declares `MIT OR Apache-2.0`; the
actual license/notice files, registry names and release ownership still require
the planned public-release audit. See [distribution](../../docs/extraction.md) and
[adopter guidance](../../docs/gameskills.md).
