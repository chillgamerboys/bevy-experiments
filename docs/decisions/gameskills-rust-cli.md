# Follow-up: move the GameSkills CLI to Rust

Status: requested by the owner September 10, 2026. Finish and review the initial
Python foundation, then begin this follow-up after its PR merges. The owner
accepted Python for that first implementation, not as the intended permanent
runtime. GameKit remains a Rust library; GameSkills' canonical instructions remain
portable Markdown with native client metadata.

## Scope and compatibility contract

Move the executable support into a Rust CLI package in this workspace. Preserve
the `gameskills.toml` schema, portable installation lock, JSON command interface,
immutable bundle selection, queue semantics and evidence distinctions unless a
specific migration is reviewed and documented. The CLI is development tooling;
its dependencies must not enter GameKit's runtime dependency graph or require
Bevy merely to install/run skills.

Use the GameSkills `plan`, maintainer `evolve-gamekit`/`author-skill`, `test` and
delivery workflows to drive the migration. Keep at most five authorized workers,
real worktree ownership and serial integration. The host remains responsible for
worker launch/model selection; a Rust port must not imply a new agent runtime.

## Implementation sequence

1. Extract language-neutral black-box fixtures from the Python behavior tests.
   Record supported CLI/error/status contracts and deliberate unsupported cases.
2. Add the independent Rust CLI crate and configuration/catalog/identity parsing.
   Use ordinary Rust CLI/serialization libraries after reviewing their current
   maintenance, license and compatibility. Keep the same canonical skill bodies.
3. Port bundle creation, preservation of adopter settings, update/rollback and
   interruption recovery. Reuse actual pinned Git fixtures and native client tests.
4. Port plan validation and durable queues, including ownership, dependencies,
   five-worker cap and add-only injection. Keep caller reports separate from
   observed command evidence and integration.
5. Port command scheduling, resource exclusion, timeout/interruption cleanup,
   source/config binding and stale-evidence checks. Preserve failure logs and
   explicit recovery boundaries. Add Windows execution only with real process,
   locking and cancellation evidence; do not silently weaken guarantees.
6. Run both implementations against the same black-box contract, exercise native
   Codex/Claude workflows using the Rust executable, and validate an adopter update.
   Ship a reproducible binary/package, update installation docs and retire the
   Python executable after the replacement passes. Preserve relevant historical
   fixtures instead of maintaining two runtime implementations indefinitely.

## Acceptance

The release/adoption path must work without Python. Rust checks cover equivalent
real Git/process behavior, not a line-for-line translation. Native discovery and
meaningful skill invocation must still work with the actual candidate. A prior
Python record must either be validated under an explicit compatible format or
rejected with a clear rerun/migration action; never relabel old evidence as Rust
execution. Keep client/platform limitations visible until separately verified.

This follow-up changes tooling language; it does not expand the 12-skill core or
make optional specialist packages mandatory. Broad docs/game refactors and the
deferred Port Vila pilot remain separate work.
