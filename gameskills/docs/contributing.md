# Maintain GameSkills

Author canonical instructions in `plugins/` and runtime code in `cli/`. Use the
maintainer `author-skill` and `evaluate-skills` guidance when changing behavior.
Review current docs and skill prose together. Keep API inventories in Rustdoc;
keep reusable craft in package-local references with explicit loading criteria.

## Candidate verification

Run from the repository root:

```sh
cargo run --locked -p repo-devtools --profile ci -- check
cargo run --locked -p repo-devtools --profile ci -- skills validate
cargo test --locked -p gameskills-cli --profile ci
cargo clippy --locked -p gameskills-cli --all-targets --profile ci -- -D warnings
```

Use the [internal tool commands](../../devtools/README.md) to prepare/check a bundle
from committed canonical source. Align native/catalog versions and minimum CLI
compatibility. Then package the CLI and inspect its actual archive. Install from
that archive into a clean consumer outside this repository, apply core-only and
selected optional bundles, and verify an update preserves local guidance.

Exercise conventional/custom docs layouts and README-only projects. Verify missing
or moved headings, owner selection for mixed work, and a docs/code contradiction.
Check that a focused skill reads the relevant guide and produces a useful action;
shorter prose and valid links alone do not prove behavior improved.

## Evidence and support

Record candidate/source/CLI/bundle/client identities with observations. Distinguish
structural validators, process regressions, actual Cargo-archive consumers, native
skill discovery, model behavior and human acceptance. A failed or unavailable client
trial stays explicit. Do not turn successful Codex discovery into Claude parity.
Forward agent evaluation requires applicable authorization; useful solo exercises
remain possible without extra agents. Report missing timing/cost data as unavailable.

The current candidate has no demonstrated registry release or authenticated Claude
behavior claim. Windows portable installation does not imply POSIX runner support.
Cross-machine multiplayer acceptance belongs to games and the multiplayer capability.
Remaining product/runtime trials live in [framework follow-ups](plans/framework-followups.md)
and [workflow reliability](plans/workflow-reliability.md).

## Compatibility fixtures

The frozen seven-skill sources and migration accounting live in devtools test fixtures.
They are compatibility inputs, not current docs and not extra authoring locations.
Preserve their bytes, provenance, existing adopter overlays and old run records.
Current skill lessons and intentional retirements are explained by the maintainer's
[legacy migration reference](../plugins/gameskills-maintainer/references/legacy-migration.md).
