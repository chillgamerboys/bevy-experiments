# Maintain GameSkills

Author canonical instructions in `plugins/` and runtime code in `cli/`. Use the
maintainer `author-skill` and `evaluate-skills` guidance when changing behavior.
Review current docs and skill prose together. Keep API inventories in Rustdoc;
keep reusable craft in package-local references with explicit loading criteria.

## Candidate verification

Resolve project rigor and affected scope first. Use focused runtime/validator
tests for tooling changes; no gameplay walk is required without game effects.
The following are available candidate checks from the repository root, not a
requirement to rerun the whole list at every skill or review step:

```sh
cargo run --locked -p repo-devtools --profile ci -- check
cargo run --locked -p repo-devtools --profile ci -- skills validate
cargo test --locked -p gameskills-cli --profile ci
cargo clippy --locked -p gameskills-cli --all-targets --profile ci -- -D warnings
```

Use the [internal tool commands](../../devtools/README.md) to prepare/check a bundle
from committed canonical source. Align native/catalog versions and minimum CLI
compatibility. For ordinary Development instruction updates, verify the prepared bundle and the
affected installation/runtime contracts. Archive construction and clean archive
consumer trials belong to Release, or changes specifically affecting packaging.
Those trials apply core-only and selected optional bundles and verify that updates
preserve local guidance; local source checks do not establish archive acceptance.

For changes to documentation discovery, exercise conventional/custom docs layouts and README-only projects. Verify missing
or moved headings, owner selection for mixed work, and a docs/code contradiction.
For changed guidance, use a bounded realistic sample to check that the skill reads
the relevant guide and produces a useful action;
shorter prose and valid links alone do not prove behavior improved.

## Evidence and support

Record candidate/source/CLI/bundle/client identities with observations. Distinguish
structural validators, process regressions, actual Cargo-archive consumers, native
skill discovery, model behavior and human acceptance. A failed or unavailable client
trial stays explicit. Do not turn successful Codex discovery into Claude parity.
Forward agent evaluation requires applicable authorization; useful solo exercises
remain possible without extra agents. Select cases affected by the candidate; a
logic-only sample should not acquire a UI tour, a UI change should use its configured
display, and a milestone should preserve any required developer response. Structural
checks alone cannot prove these behaviors or token savings. Report missing timing/cost
data as unavailable. Coverage outside the selected scope does not block every
instruction update.

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
