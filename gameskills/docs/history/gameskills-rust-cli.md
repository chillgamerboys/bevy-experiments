# Plan: finish the repository tooling migration to Rust

Status: the owner authorized completing the remaining migration without per-stage
approval on September 11, 2026. R0/R1 merged in PR31 and R2a/R2b/R2c merged in
PRs32–34 with passing hosted checks. R3 installation, R4 queues and R5 supervision
are implemented and independently reviewed; 124 combined CLI tests passed locally.
R6 packaged installation/adoption and actual Codex discovery passed. R7 uses the Rust installation/configuration/CI path and removes all maintained
Python. Fresh-checkout checks passed with Python excluded from PATH; final hosted
checks are attached to the review PR. The [adoption record](gameskills-rust-adoption.md)
contains the candidate identities and evidence. The owner authorized fixing the
self-audit recovery finding and merging after updated checks pass. Mechanical and visual Labyrinth changes
are the subsequent collaborative product trial.

The prepared embedded instructions now declare Rust runtime compatibility and use
Rust command guidance. This replaces the earlier CLI-only proposal, which excluded
CI routing and repository checks. The final state includes **all repository-owned
executable tooling and tests**. This status is progress tracking; passing migration
accounting alone does not establish implementation or acceptance evidence.

The [refinement observations](gameskills-refinement-results.md) retain the actual
native walks, CI findings and evidence limits. The tooltip heading/close scrolling
issue remains a bounded GameKit UI follow-up; the owner accepted the current work.
It does not need to delay this tooling migration. Claude's authenticated behavioral
probe remains unavailable; starting Rust work does not establish client parity.

## What entirely Rust means

GameKit, games, the GameSkills executable, installation/migration helpers,
validators, CI selection/gating and test programs will all be Rust. No maintained
Python source, interpreter-dependent test fixture, Python wrapper or Python setup
step remains at cutover. Do not replace Python with shell, JavaScript or PowerShell
business logic, or embed the old programs as Rust string literals.

Markdown skills and documentation, JSON/TOML/YAML configuration and fixtures,
artwork, fonts, licenses and lockfiles remain appropriate repository data. Native
agent clients, Git, Cargo and GitHub's supported actions remain external tools;
this decision does not require rewriting their internals. Workflow YAML may invoke
Cargo and documented external commands, but repository-owned routing and execution
logic lives in Rust. Prebuilt GameSkills adoption must need neither Python nor a
Rust compiler; building the tools from source requires Cargo and the pinned toolchain.

The 12 core skills, nine optional skills, `plan` entrypoint, Bevy/GameKit direction
and maximum of five authorized workers stay stable. This is not a new scheduler,
agent host, skill catalog, docs overhaul or game refactor.

## Inventory and destination

The accepted refinement tree `08a38a594750a44411756bcf8f21e99da5dddf8b` contains
22 tracked Python files: 13 executable/support modules and nine test modules.
The migration must account for every one, including the frozen legacy tooling.

| Current owner | Rust destination and retained behavior |
|---|---|
| `plugins/gameskills/scripts/gameskills.py` and six modules in `runtime/gameskills_runtime/` | `tools/gameskills-cli`: CLI plus an internal library for catalog/configuration, bundle setup, native adapters, plan/queue state, execution and evidence |
| `scripts/check_repo.py` | `tools/gamekit-repo-tools`: repository layout, local documentation links and capability ownership |
| `scripts/check_distribution.py` | Repository-tool distribution command: Cargo-selected external consumers, empty/pure/UI/network graphs and fixture containment |
| `scripts/ci.py` | Repository-tool `ci select`, `ci run` and `ci gate`: affected ownership, reverse consumers, conservative fallback and the stable final result |
| `skills/scripts/validate_gameskills.py` | Rust validation of catalogs, frontmatter, native metadata, references and scenario structure |
| `skills/scripts/validate_skills.py` and `skills/scripts/skills_tool.py` | Rust compatibility validation/import for existing seven-skill installs, then retirement of the old updater entrypoints |
| Three modules in `scripts/tests/` and six in `skills/tests/` | Rust unit/integration tests, fixture data and compiled subprocess probes; no Python child programs |
| `.github/workflows/gamekit.yml`, `gameskills.toml`, current guides and canonical runtime instructions | Cargo/binary invocations, declared tool identities and updated installation/evidence examples |

Historical records can still describe the Python implementation accurately. Active
instructions must use the Rust path once cut over. Preserve adopter-owned overlays
and files; removing Python from this repository does not authorize deleting them.
Replace historical links to retired source files with immutable commit links so
the documentation checker does not require retaining obsolete executable files.

## Architecture and commands

Keep the existing **single Cargo workspace and lockfile**. The current repository
validator deliberately rejects nested workspaces and duplicate lockfiles. Add the
two `tools/` packages explicitly to workspace membership; preserve the existing
default game target. Cargo supports explicit package selection, so tooling checks
can build these packages without compiling Bevy when their dependency graphs stay
independent. See [Cargo workspace and package selection](https://doc.rust-lang.org/cargo/reference/workspaces.html).

- `gameskills-cli`, binary `gameskills`: the installable developer tool. Use modules
  and an internal library for behavior tests; avoid committing to a public library
  API merely to split files. It must have no GameKit/game/Bevy dependencies.
- `gamekit-repo-tools`, binary `gamekit-repo`: the small repository/CI maintenance
  tool. Keep the classification and docs path free of native client or game startup.
  Do not make it compile the full command-runner implementation just to check links.

The [distribution proposal](../../../docs/extraction.md) recommends crates.io for the GameKit
libraries and GameSkills executable, GitHub Releases for prebuilt binaries, and a
baseline skill bundle embedded in the CLI. Repository tools remain unpublished.
These are working package names, not a claim that registry names are reserved.
Distribution names, versions, license audit and an explicit supported Rust version
are resolved in R0/R1 before publishing anything. Prefer established
CLI/serialization and platform libraries with reviewed dependencies; select exact
versions at implementation time. Preserve the workspace's safety/lint policy and
review any necessary platform abstraction before adopting it.

Proposed command families, finalized against the compatibility fixtures in R0:

```text
gameskills --root <repo> catalog|status|config|setup|bundle|native|plan|queue|run|evidence
gameskills --root <repo> legacy import ...
gamekit-repo check
gamekit-repo skills validate
gamekit-repo distribution --case <empty|pure|ui|network>
gamekit-repo ci select|run|gate
cargo test --locked -p gameskills-cli -p gamekit-repo-tools
```

The legacy import is an intentional new transition command, not a promise of
existing Python syntax. The other command names are contracts to preserve where
useful; internal Python module structure is not a Rust design requirement.

## Contracts to freeze before implementation

1. **Inputs and output.** Inventory argument vectors, exit codes, JSON payloads,
   duplicate-key rejection, TOML types, path validation and deliberate unsupported
   cases. Version structured output and normalize only nondeterminism in fixtures.
   Preserve meaningful errors, not incidental argparse formatting.
2. **Bundle and executable identity.** A skill bundle remains an immutable source
   selection with its package inventory/content digest. Record the CLI version,
   build/source identity and executable digest separately. A newer binary must
   check supported bundle/config/state schemas before reading or mutating them.
   Resolve the selected bundle explicitly from the installation, not the executable's
   former location beside Python files or an ambient source checkout.
   The embedded baseline and standalone bundle must come from the same deterministic
   package-local snapshot of canonical instructions. A packaged crate must build
   without sibling workspace resources; initial setup must work without fetching a
   bundle. Default selection remains core-only even if optional payloads are embedded.
3. **Owned state.** Preserve local configuration, selected packages and client
   settings. Setup proposals remain read-only; apply/update/recovery keep locking,
   conflict detection and atomic writes. No live queue changes runtime midway.
4. **Queues.** Preserve revision checks, real worktree/base identities, ownership,
   dependencies, resource exclusions, the five-worker ceiling and add-only injection.
   Caller reports, observed source integration and human acceptance remain separate.
5. **Evidence.** Persist actual argv/cwd, tool/environment/source identity, durations,
   status, logs and cleanup errors. Failed, interrupted, stale and skipped never mean
   passed. Resume creates a fresh run and executes its graph again. The recorded
   result is not a model evaluation, native playtest, approval or release attestation.
6. **Reference changes.** The trial exposed a successful command becoming stale
   during a concurrent push. Initially preserve conservative Git-reference binding
   and document sequencing checks after Git changes settle. Narrow declared Git
   inputs may be a separately tested schema change; do not weaken evidence while
   translating languages or relabel old records to avoid reruns.
7. **Prior installations and records.** Keep immutable old records readable as
   historical observations. Validate only formats whose semantics remain supported;
   reject unsupported reuse with a clear migration/rerun action. An approved
   schema migration must preserve originals and report its exact changes. Old
   Python execution never becomes Rust execution merely because Rust reads it.

## CI without Python

Build the small repository tool from the checked-out revision using a locked Cargo
command and a toolchain/lockfile cache key. A cache miss must compile source; it must
not download a moving helper from `main`, trust an unrelated old binary, or skip
classification. The final gate must still run after classifier failure and report
failure, including when the helper cannot build. A failed bootstrap cannot yield a
green result. Design and test that failure path in workflow YAML as well as Rust.

Narrative docs run this lightweight Rust bootstrap, layout/link checks, relevant
routing/gate tests and the final result. **Cold docs CI will compile a small Rust
tool; it will not build Bevy or run the GameSkills execution suite.** This is the
explicit tradeoff for removing the Python bootstrap while preserving scoped CI.

Tooling source selects its Rust tests and applicable platform checks. Game changes
select their owners/consumers; ordinary tool changes do not select games. Root
manifest/lock changes, routing/workflow changes and uncertain inputs still receive
broader verification. Shared lockfile changes can therefore legitimately select
the full matrix. Do not split workspaces just to avoid a correct shared-input check.

Retain renamed/deleted inputs, longest package ownership, target/dev/build local
dependencies, all Rust include delimiters and custom Cargo build scripts. Preserve
all external-distribution cases, applicable WASM/minimal-feature checks and the
Labyrinth six-process test. Port the Git-backed routing and final-gate failure
fixtures before removing their Python owners. Demonstrate docs-only, tools-only,
game-only and shared changes in hosted CI. Tooling package declarations must not
be mistaken for GameKit capability crates by distribution or graph validation.

## Reviewed implementation slices

| Slice | Concrete deliverable | Exit evidence and dependencies |
|---|---|---|
| R0. Contract baseline | Command/schema inventory, reference commit, language-neutral fixtures, artifact/compatibility contracts and intentional differences | Every Python behavior/test group has a retained contract, explicit correction or retirement owner; active state can stay on the old runtime safely; public naming, license and MSRV decisions have owners |
| R1. Rust foundation | Two tools packages, command parsing, diagnostics, dependency boundaries, packaging scaffold and compiled test probes | `cargo test -p ...` builds no Bevy; CLI help/config/error contracts pass; extracted CLI crate builds without sibling files; no global runtime switch or publication; depends on R0 |
| R2. Repository and CI tooling | Port layout/link/catalog/legacy validation, distribution selection and CI commands/tests; add archive inspection and bundle preparation; cut callers over together | Hosted routing/gates retain coverage, bootstrap failure stays red and distribution consumers exclude tools; deterministic snapshot checks; delete replaced repository Python scripts/tests after parity; depends on R1 |
| R3. Installation and compatibility | Catalog/config, embedded baseline, immutable bundles, setup/update/rollback/recovery, native command construction and legacy import | Real pinned-Git fixtures; initial setup without checkout/network bundle fetch; preservation/conflict/path checks; explicit compatible updates and interrupted recovery; no Python requirement; depends on R1, integrates R2 bundle preparation |
| R4. Plans and queues | Validated plans, durable queues, revision-guarded mutations and injection | Real isolated worktrees, ownership/dependency checks, block/resume and observed integration; no cross-runtime active queue; depends on R3 |
| R5. Runner and evidence | DAG execution, shared locks, deadlines/cancellation, logs, stale detection and recovery | Real subprocess/descendant/lock contention tests, including coordinator death and retained handles; prior record compatibility remains honest; depends on R3 and joins R4 before pipeline trials |
| R6. Candidate adoption | Private distribution rehearsal: actual CLI source/prebuilt artifacts and immutable skills, GameKit consumers, native adapters and a bounded task through PR delivery | Fresh consumer installs/invokes/updates/recovers; package contents/licenses and claimed targets verified; actual Codex and authenticated Claude checks separately recorded; runtime/bundle identities match; public publishing is a later endpoint; depends on R2–R5 |
| R7. Final cutover | Switch active instructions/configuration/CI; delete remaining Python runtime/tests/legacy updater and transition harness | No-Python acceptance below passes from a fresh checkout/artifact; one authoritative Rust implementation remains; depends on R6 |

Each slice gets a bounded plan, appropriate tests, independent review when useful,
and a PR. Current merge authorization applies to the accepted refinement stack;
future implementation PRs follow the normal review/merge endpoint. R2 and R3 can
run independently after the foundation interfaces settle. R4 and R5 can run in
parallel with explicit file/resource ownership. Use fewer than five workers when
there is no useful independent work; integrate serially. Do not create workers,
queues or issues merely to match this table.

Freeze the Python reference at the accepted commit. During transition, a temporary
Rust-driven comparison harness may invoke it in a controlled reference checkout.
It must not become an adopter dependency or a permanent second implementation.
Correct known bugs through documented contract changes and Rust regressions rather
than copying them for parity. R7 removes the comparison harness's Python dependency;
Git history and language-neutral cases retain the lessons.

## Implementation order and ownership

Start with one PR containing R0 and the smallest R1 foundation. Then deliver R2
repository tooling and R3 installation as separately reviewable work. Split R2
into validator/distribution and CI cutover PRs if the combined diff obscures review;
switch each command together with its configuration, callers and tests. R4 queues
and R5 execution follow R3. Finish with R6 candidate adoption, then R7 deletion and
instruction cutover. Each stage's exit evidence above remains required even when
the stage takes multiple PRs.

Use one integration owner for `Cargo.toml`, `Cargo.lock`, toolchain configuration,
workflow YAML, `gameskills.toml`, shared fixture schemas and release metadata.
After R1 establishes interfaces, the useful independent owners are repository
validation/CI, installation/native protocol, queue state and process/evidence.
They are work boundaries, not a request to start four workers immediately. Do not
assign whole CLI directories to two concurrent workers; declare concrete module
paths in work orders. Native protocol tests from R3 join the real process cleanup
backend in R5 before native execution can pass R6.

The coordinator uses `plan`, then `dispatch` only for useful authorized parallel
work. Each wave records actual worktree/source identities and dispatch versus merge
dependencies. Apply the five-worker ceiling and any lower host limit. Shared
manifest edits and integration are serialized. Use `test` and `update-docs` for
changed contracts, then `create-pr` and `audit-pr` for each delivery; `merge-pr`
and `release` follow the session's actual endpoint. Game UI/playtest skills are
used only when the adopted real task makes those claims.

## First implementation PR: contracts and a usable foundation

The first PR establishes a buildable migration target with verified interfaces.
Its review artifact includes the following concrete changes:

1. Add `tools/migration-contracts.json`, a language-neutral inventory covering all
   22 Python files and 142 test methods at the accepted reference revision. Each
   behavior has a stable identifier, source test/symbol, retained/corrected/retired
   disposition and rationale, destination stage/module, required evidence and
   implementation status. A Rust check rejects missing, duplicate or unaccounted
   entries against the pinned reference inventory. Test totals describe coverage
   to assess, not evidence that tests passed.
2. Capture representative command/config/error cases as package-local data under
   each tool's `tests/fixtures/`. Record argv, inputs, exit status, structured output,
   expected writes and required platform/preconditions. Keep fixture paths and
   timestamps deterministic without normalizing away source or integrity checks.
   Generate Git repositories/worktrees during tests; do not commit nested repos or
   executable Python fixtures. Expand fixtures as each later owner ports behavior.
3. Add `tools/gameskills-cli` and `tools/gamekit-repo-tools` to the existing workspace,
   with package-local internal libraries and thin binary entrypoints. Implement
   help/version, structured failures and initial configuration parsing/validation
   tests. Preserve the current public command's setup prerequisites unless R0
   records an intentional change. Unported operations fail explicitly; no success
   stubs or forwarding to Python. No existing consumer is switched to these binaries.
4. Establish reusable compiled test-probe utilities and minimal filesystem/process
   interfaces for later stages. Keep platform policy behind explicit boundaries;
   the first PR does not claim a working runner. Use the workspace's existing lint
   policy, including `unsafe_code = "forbid"`, and reviewed safe dependencies for
   needed OS behavior. Dependency selection and exact versions belong in this PR.
5. Set the development toolchain deliberately and document each tool package's MSRV.
   At the R0 baseline, hosted Rust jobs used moving `stable` with no tracked pin.
   The foundation pins Rust 1.97.1 and initially declares that tested minimum for
   both tools. Check the minimum against the workspace resolver and dependencies
   before declaring it. Record license/package metadata decisions and preserve
   `publish = false`; public registry names remain provisional until checked.
6. Package the minimal CLI, inspect its contents, extract it outside the checkout
   and build/install it without sibling workspace paths. Record the source identity
   and package listing. This proves the foundation can be packaged; the complete
   embedded catalog and update experience remain R2/R3 acceptance.

First-PR acceptance: focused Cargo tests, formatting and lint checks pass; the
selected tools' build graphs exclude Bevy/GameKit/games; extracted-package checks
pass; existing repository/skills checks still pass; `gameskills.lock.json` and
installed bundles are unchanged. `gameskills.toml` may add reviewed Rust check
commands while retaining the current runtime as its active entrypoint.
Because this PR changes the root manifest/lock and CI inputs, broader hosted checks
are expected. It must not weaken shared-input CI just to obtain a tools-only run.

The contract inventory belongs to the migration, while executable fixtures live
inside the package that needs them. A distributed CLI never reads the inventory
from a source checkout at runtime. Packaging the later embedded bundle is covered
by the [distribution proposal](../../../docs/extraction.md).

## Existing regression groups and port owners

This is the source inventory observed during planning; exact test-to-contract
mapping is R0's deliverable. Proposed Rust paths below are not existing files.

| Python test module | Methods | Rust destination and stage |
|---|---:|---|
| `scripts/tests/test_check_repo.py` | 5 | `gamekit-repo-tools/tests/repository.rs` — R2 layout, links and dependency direction |
| `scripts/tests/test_check_distribution.py` | 4 | `gamekit-repo-tools/tests/distribution.rs` — R2 source boundaries, supplemented by actual-archive tests |
| `scripts/tests/test_ci.py` | 26 | `gamekit-repo-tools/tests/ci_routing.rs`, `ci_checks.rs`, `ci_cli.rs` — R2 Git-backed ownership/selection, command selection and failing final gates |
| `skills/tests/test_gameskills_catalog.py` | 21 | `gamekit-repo-tools/tests/catalog.rs` — R2 package/frontmatter/native metadata, references and scenario structure |
| `skills/tests/test_gameskills_packaging.py` | 17 | `gameskills-cli/tests/installation.rs` — R1 initial config cases, R3 immutable setup/update/recovery |
| `skills/tests/test_gameskills_native.py` | 10 | `gameskills-cli/tests/native.rs` — R3 protocol/discovery contracts, R5 real subprocess shutdown, R6 actual-client observations |
| `skills/tests/test_gameskills_workflow.py` | 18 | `gameskills-cli/tests/workflow.rs` — R4 revisions, ownership, dependencies, worktrees and integration observations |
| `skills/tests/test_gameskills_runner.py` | 32 | `gameskills-cli/tests/runner.rs` — R5 DAG execution, cancellation/descendants, locks, identity and evidence |
| `skills/tests/test_skills_tool.py` | 9 | `gameskills-cli/tests/legacy.rs` — R3 preservation/import contracts; obsolete install/sync behavior explicitly retired by R7 |

Rust paths are relative to `tools/`. The 142 methods are a completeness baseline,
not a required one-to-one test translation or a coverage target. Retain meaningful
negative cases, race and recovery tests; combine redundant setup and add missing
packaging/platform regressions. Historical output formats may remain readable
without preserving every obsolete command. Explain such changes in the inventory.

## Transition and recovery checkpoints

- **Before the first cutover:** the existing pinned Python candidate remains usable.
  Run transition comparisons only against an isolated reference checkout. New Rust
  checks run alongside relevant existing checks until their component's parity is
  reviewed, rather than running the full migration suite for narrative docs.
- **At R2:** switch repository commands and CI together with their Rust regression
  tests, and remove only the Python owners they replace. Prove classifier build
  failure, selected job failure and missing outputs cannot pass the final gate.
  Record the last working source revision so an ordinary revert can restore that
  CI slice; do not depend on a runtime fallback that silently invokes Python.
- **At R3–R5:** exercise installation and state transitions in temporary adopters.
  The current repository's installation is not a migration test fixture. An update
  refuses active queues/runs that would cross runtime identity. Old records and
  bundles remain historical; rollback preserves the original state and user edits.
  Changing schema support must include fixtures for both acceptance and rejection.
- **At R6:** use the candidate on a bounded repository task through PR delivery,
  selecting one after the candidate is usable. The owner reserved Labyrinth mechanical/visual
  changes for the collaborative trial after end-result review; use a bounded
  tooling/documentation task for this private pipeline rehearsal. Record failures as findings;
  do not combine an unresolved product fix with proof of a successful tool migration.
- **At R7:** delete the remaining Python only after the full candidate passes its
  supported-platform gates. An unavailable authenticated client check remains a
  stated support limitation; it cannot be counted as a pass. Finish the current
  documentation/command updates before the separate broad docs restructure begins.

## Process and platform acceptance

Replacing `subprocess` is more than spawning and waiting for one child. Rust's
[`Child` lifecycle](https://doc.rust-lang.org/std/process/struct.Child.html) still
requires explicit waiting and cleanup. Preserve whole-command-group termination,
resource-lock lifetime and cancellation evidence on macOS/Linux. Use Rust test
executables that sleep, fork/spawn descendants, retain stdout/locks, exit early and
ignore termination; do not shell out to Python to manufacture these cases.

Target equivalent Windows execution using a reviewed backend, including process-tree
containment and OS locks. [Windows Job Objects](https://learn.microsoft.com/en-us/windows/win32/procthread/job-objects)
provide a relevant containment primitive, but adoption must address creation/assignment
races, handle inheritance and abrupt coordinator death. Passing serialization tests
on Windows is not runner support. Preserve an explicit unsupported-operation result
until the backend's real tests pass; do not make a Windows support claim merely to
finish the port. The Rust-only cutover can retain that declared capability limit,
while a three-platform execution release requires the backend evidence.

Native agent discovery must resolve only selected immutable packages. Test command
construction separately from actual client startup, invocation and a complete task.
Use bounded evaluations with recorded client/candidate versions. The earlier Claude
401 is not a Rust defect and need not block porting; verified cross-client behavior
requires a successful authenticated evaluation before making that release claim.

## Final no-Python acceptance

- No tracked `.py`/`.pyi` files, Python shebangs, embedded executable Python fixtures,
  repository-owned Python launchers or Python CI setup/invocations remain. An audit
  also covers other executable script languages and generated/package contents;
  filename deletion alone is insufficient.
- Current docs, canonical skill instructions, `gameskills.toml`, bundle manifests,
  native adapters and installation/upgrade commands resolve the Rust implementation.
  Historical references are explicitly historical and cannot be the active entrypoint.
- A fresh checkout builds and tests repository-owned tooling with Python unavailable;
  real process probes are compiled Rust. Audit launched commands so an ignored cache,
  system Python or test-only interpreter cannot conceal an undeclared dependency.
- A fresh consumer uses the actual packaged binary and pinned skills to install,
  invoke, run, inspect evidence, update and recover without Python or Cargo. Check
  source/versions, checksums and dependency/license boundaries for the artifact.
  Separately build/install the CLI from its Cargo archive with no repository sibling
  paths. Verify embedded and standalone bundles have matching source/content identity
  and reject incompatible updates. Use the distribution proposal's actual-archive
  gates; source staging alone must not be reported as registry verification.
- CI demonstrates the intended scopes and fails on selected job failure/cancellation,
  missing outputs, unexpected skips and classifier/bootstrap errors. GameKit consumers
  still exclude tool/game dependencies and retain their feature contracts.
- Old installation updates and state recovery preserve user edits and originals.
  No live queue is silently migrated; stale observations and client/platform limits
  remain visible. No record is upgraded into an unobserved passing claim.

The runtime migration is implemented; the status above and final adoption record
track its verification and review endpoint.
The full docs restructure, tooltip scrolling correction, playable Labyrinth release,
companion Deckbuilder development and deferred Port Vila integration remain explicit
subsequent work. Their delivery should use the Rust framework once it is accepted.
