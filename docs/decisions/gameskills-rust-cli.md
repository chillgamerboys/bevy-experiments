# Plan: finish the repository tooling migration to Rust

Status: the owner accepted the refinement work and authorized its merge on
September 10, 2026, then requested this repository-wide Rust migration plan.
Implementation has not started. This replaces the earlier CLI-only proposal,
which explicitly excluded CI routing and repository checks. The final state now
includes **all repository-owned executable tooling and tests**.

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

The [distribution proposal](../extraction.md) recommends crates.io for the GameKit
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

The immediate next implementation is **R0 plus the smallest usable R1 scaffold**.
The full docs restructure, tooltip scrolling correction, playable Labyrinth release,
companion Deckbuilder development and deferred Port Vila integration remain explicit
subsequent work. Their delivery should use the Rust framework once it is accepted.
