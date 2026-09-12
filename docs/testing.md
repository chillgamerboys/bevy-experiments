# Tests and evidence

From the repository root, with Rust 1.97.1:

```sh
cargo fmt --all -- --check
cargo test --workspace --all-features --profile ci
cargo test --workspace --doc --all-features --profile ci
cargo clippy --workspace --all-targets --all-features --profile ci -- -D warnings
cargo deny check
cargo run --locked -p repo-devtools --profile ci -- check
cargo run --locked -p repo-devtools --profile ci -- distribution check
cargo run --locked -p repo-devtools --profile ci -- distribution archives
cargo run --locked -p repo-devtools --profile ci -- bundle check
cargo test --locked -p repo-devtools --profile ci --test ci_routing --test ci_checks --test ci_cli
cargo run --locked -p repo-devtools --profile ci -- skills legacy
cargo run --locked -p repo-devtools --profile ci -- skills validate
cargo test --locked -p gameskills-cli --profile ci
```

Use `cargo test -p <package> --profile ci` for focused iteration. The list above is
the broad validation set, not a requirement to rebuild all games for narrative
documentation. `cargo deny` is a separately installed dependency-policy tool.

For Rust tooling, use the focused package tests and contract checker:

```sh
cargo test --locked -p gameskills-cli -p repo-devtools --profile ci
cargo clippy --locked -p gameskills-cli -p repo-devtools --all-targets --profile ci -- -D warnings
cargo run --locked -p repo-devtools --profile ci -- contracts check --verify-reference --cutover
cargo package --locked -p gameskills-cli
cargo run --locked -p repo-devtools --profile ci -- bundle verify-package
cargo package --locked -p repo-devtools
```

The contract check accounts for the frozen 22 Python files and 142 test methods;
it verifies complete accounting, not test execution. The original reference remains
in Git history. Rust fixtures cover configuration, immutable installation/recovery,
real worktrees, native protocols, DAG execution, resource locks, stale evidence and
POSIX process cleanup. Compiled Rust probes replace interpreter-based children.
Historical records retain their original runtime identity.

`tests/adoption.rs` copies the actual CLI outside the workspace and exercises a
fresh POSIX Git adopter with Cargo, rustc and Python absent from its PATH. It checks
embedded setup, package changes/rollback, interrupted recovery, owner-file preservation, command execution
and evidence invalidation. `GAMESKILLS_CANDIDATE_BINARY` can select the executable
built from an extracted Cargo archive for that same trial. Native authentication
and visual game quality remain separate evidence.

Archive tests inspect actual Cargo output, normalized manifests, safe paths and
external library consumers. Bundle tests verify committed provenance, reproducibility,
corruption/drift rejection and exact payload bytes in the CLI Cargo package. Queue
operations, runner supervision and native Codex verification report unsupported on
Windows until their process/state backend is implemented and independently tested.
Portable Windows tests do not establish those capabilities.

## Specialized verification

See [CI selection](../devtools/docs/ci.md), [Labyrinth forecasts](../games/labyrinth/docs/forecasts.md)
and each game's verification guide for the owning checks.
