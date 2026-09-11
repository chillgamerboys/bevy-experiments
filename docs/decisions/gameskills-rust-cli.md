# Follow-up: move the GameSkills CLI to Rust

Status: Rust migration requested by the owner September 10, 2026. The Python
foundation merged in [PR #24](https://github.com/chillgamerboys/bevy-experiments/pull/24)
at `67c88c7f36ba96c62a0c6d5986af5bc4010bb4f5`. The sequencing below is a
recommendation for discussion, not an implemented migration. The owner accepted
Python for the foundation, not as the intended permanent runtime. GameKit remains
a Rust library; GameSkills' canonical instructions remain portable Markdown with
native client metadata.

## Recommendation: refine briefly, then port

Use three bounded tasks to exercise the installed workflow before freezing its
contracts. Start with [CI scope and workflow trials](gameskills-ci-scope.md), then
a small documentation cleanup and one useful UI change exercised in Labyrinth and
Deckbuilder. Complete these before the broad documentation and game refactors.
Each task should deliver a reviewed PR and record its actual checks and findings;
merging follows the owner's authorization for that PR.

The foundation has substantial automated coverage of its helper behavior and an
actual Codex planning probe. That does not yet demonstrate a complete development
pipeline, effective worker coordination through delivery, or measured cost savings.
Claude discovery succeeded, but authentication blocked behavioral evaluation.
Rewriting now would carry untested workflow assumptions into a second runtime.
The [compatibility record](../gameskills.md#compatibility-and-remaining-validation)
owns the current evidence and limitations.

Keep this refinement bounded: fix friction found by those tasks, rerun affected
checks, and review the resulting contracts. Add no new skill catalog, scheduler
or unrelated Python features. If a pilot reveals a fundamental design problem,
bring that specific finding back into planning instead of silently extending the
trial period. Unavailable Claude authentication is an explicit evaluation gap;
it need not block starting the Rust implementation, but it does block a claim of
verified Claude workflow support at release.

Rust remains the intended destination because it aligns the supported development
tool with this Rust project and can provide a standalone executable without an
adopter Python environment. Changing language alone establishes neither faster
agent work nor cross-platform process correctness.

## Scope and compatibility contract

Move the executable support into an independent Rust CLI package in this workspace.
The proposed location is `tools/gameskills-cli`, added explicitly to workspace
members so library distribution remains separate. Confirm the crate/binary name
and package publishing approach during the first Rust slice. Preserve
the `gameskills.toml` schema, portable installation lock, JSON command interface,
immutable bundle selection, queue semantics and evidence distinctions unless a
specific migration is reviewed and documented. The CLI is development tooling;
its dependencies must not enter GameKit's runtime dependency graph or require
Bevy merely to install/run skills.

Keep skill source identity and executable identity distinct. A downloaded binary
must select and validate an immutable skill bundle, report both identities and
reject unsupported schema versions with an actionable migration path. Review
binary verification, supported OS/architecture targets, installation and rollback
before distribution. Do not bundle a moving skill source into a supposedly pinned
installation. A small CI classifier or repository check is outside this runtime
port; docs CI should not need to compile the CLI merely to choose its checks.

Use the GameSkills `plan`, maintainer `evolve-gamekit`/`author-skill`, `test` and
delivery workflows to drive the migration. Keep at most five authorized workers,
real worktree ownership and serial integration. The host remains responsible for
worker launch/model selection; a Rust port must not imply a new agent runtime.

## Implementation sequence

Start after the bounded trials have exposed the workflow contracts we want to
keep. Use small reviewed slices, with one authoritative executable for adopters
until the replacement is ready. Do not switch a live queue between implementations
mid-operation or maintain two independently evolving command contracts.

1. Extract language-neutral black-box fixtures from the Python behavior tests and
   pilot findings. Record command arguments, JSON output and error/exit contracts;
   TOML and lock schemas; persisted queue/run records; and deliberate unsupported
   cases. Normalize only nondeterministic values such as temporary paths and run
   IDs. Keep actual Git, process and interruption fixtures alongside format cases.
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

For each slice, keep the Python implementation as the compatibility reference,
not the implementation design. Correct a discovered bug explicitly in the contract
and regression case rather than reproducing it for parity. Run the selected Rust
CLI checks without building Bevy; root workspace/dependency changes still receive
the broader CI coverage defined in the scope plan.

## Acceptance

The release/adoption path must work without Python. Rust checks cover equivalent
real Git/process behavior, not a line-for-line translation. Native discovery and
meaningful skill invocation must still work with the actual candidate. A prior
Python record must either be validated under an explicit compatible format or
rejected with a clear rerun/migration action; never relabel old evidence as Rust
execution. Keep client/platform limitations visible until separately verified.

Accept the replacement only after a fresh adopter can install, discover/invoke
selected skills, run a bounded task, update and recover using the actual Rust
artifact. Verify supported platforms with real locking, child-process cleanup
and cancellation tests. Windows execution support remains a separate capability
decision; it is not automatically gained by translating POSIX code to Rust.

This follow-up changes tooling language; it does not expand the 12-skill core or
make optional specialist packages mandatory. Broad docs/game refactors and the
deferred Port Vila pilot remain separate work.
