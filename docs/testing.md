# Tests and evidence

Resolve `gameskills verification resolve --base BRANCH` before selecting checks.
Feature PRs target `dev` (Development); explicitly approved milestone batches
target `main` (Testing).
Both levels run on macOS. Windows/Linux are Release checks. Creative involvement and Cargo's `ci`
build profile are independent of this policy.

Development requires affected regression tests and necessary compilation, reusing
test compilation where sufficient. It omits blanket all-target/all-feature builds,
Clippy and packaging. Shared changes include relevant consumer regressions. Testing
adds broader affected-component compilation and Clippy for the combined milestone.
Release owns distributable archives and platform/feature compatibility checks.

Choose the affected owner suites and required compile checks. Logic-only changes
need no agent screenshots or UI walkthrough. For presentation or interaction changes,
inspect the changed flow at 1920×1080 Auto. End-to-end checks follow affected journeys
and boundaries; process recovery or every UI route is not implied by any game edit.
Retain compatibility tests and select them when display support or the defect needs
them. Check that filtered tests actually run; zero matches do not prove coverage.

Milestone promotions affecting game behavior require the developer's actual sanity
response tied to the candidate and journey. No manual gate holds individual dev PRs
open. Narrative docs/tooling without game effects need no gameplay sanity check.
Adding a test, running a test and manually checking the game are separate choices.
Stop when applicable checks pass unless a new change, failure or unresolved concern
justifies more; out-of-scope coverage is not unfinished acceptance.

The [CI selector](../devtools/docs/ci.md) owns package, suite and branch routing. The
commands below are the broad available validation set, run from the repository root
with Rust 1.97.1; they are not a default development checklist:

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

Use named suites or an explicit test target/filter for focused iteration; an
unfiltered game package includes every non-ignored test, including retained display
and network cases. The list above is
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

See [CI selection](../devtools/docs/ci.md), [Labyrinth forecasts](../games/labyrinth/docs/testing.md)
and each game's verification guide for the owning checks.
