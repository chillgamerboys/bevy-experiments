# Rust GameSkills adoption record

The R3–R7 implementation replaces the maintained Python runtime with an installable
Rust CLI, immutable instructions, durable queues and supervised command evidence.
The owner authorized correcting the self-audit finding and merging the migration
after verification, before the collaborative Labyrinth mechanical and visual trial.
This record describes private adoption, not publication.

## Candidate identity

| Item | Observed identity |
| --- | --- |
| CLI / tested toolchain | `0.1.0-dev.2` / Rust `1.97.1` |
| Final packaged implementation | `dc472ca03e96b1b01ce642cc54c4379a0a39543e` |
| Local prebuilt target | `aarch64-apple-darwin` |
| CLI executable SHA-256 | `ded6b5a676ed500807a3f91284afd9e010b3b1613f359a7dfe96bb8a8b5f863a` |
| Actual Cargo archive SHA-256 | `bbe46bc6cc349b14b465d2963938119712e8a5675d5a2b15abfe1e272bfe9138` |
| Instruction catalog | `0.1.0-dev.1`; 12 core and nine optional skills across six packages |
| Instruction content SHA-256 | `5215cb966addc4f85812dc8f9b11c818ee851946f912733cc2355c743d31a8e2` |
| Canonical instruction source | `0a3d337cd814a2b0392cce7a8b421ecb2448cb31` |

Cargo packaged and verified the actual CLI archive. Its normalized standalone
manifest, lockfile and embedded payload were inspected. The archive was extracted
outside the workspace and `cargo install --locked --offline --path ...` produced
the candidate there, with Python excluded from the build PATH. Runtime adoption
then copied that executable into an environment containing only it and Git.
The initial behavioral review used `4e991782`. Hosted verification identified
platform-specific imports and a `Stat` comparison, Windows setup reading its own
mandatory queue lock, and fractional timing values losing precision during JSON
parsing and invalidating record digests. The corrected candidate skips lock data
before opening queue entries and enables exact float roundtrips. Regressions verify
4,096 fractional observations, protected record reload and rejection of a one-ULP
edit. Resource-scope probes now synchronize explicitly; native write-deadline
coverage forces actual stdin backpressure with one large request instead of relying
on retry timing and platform pipe capacity.

Strict all-target Clippy passed for macOS and when compiled for Windows and Linux.
The rebuilt archive was installed again, and packaged adoption and actual Codex
discovery were repeated. Cross-compilation is not a claim of native execution;
the final PR carries the hosted platform evidence.

The final archive also contains portable test fixtures: Git receives relative
worktree paths, and simulated installation lock owners explicitly unlock while
duplicate descriptors remain open. The latter models descriptor retention without
claiming to reproduce hosted fork timing. Those fixture corrections changed only
tests; the earlier `56a61b6` executable was byte-identical to the verified `1ebfea1`
binary. The later self-audit correction below changes runtime behavior.
Native probe observations are published by rename so a deliberately terminated
writer cannot leave an empty counter. The affected deadline case passed 20
consecutive process runs, and all ten native integration tests passed.

The self-audit at `efcc58d` ran 280 tooling tests through the packaged CLI and
reproduced an interrupted-queue-write recovery defect with a separate compiled
Rust probe: `.queue-<pid>-<serial>.tmp` residue blocked setup as unknown history.
The corrected `558061d` candidate recognizes only the queue writer's canonical
temporary names, validates ordinary file safety under the queue lock, and preserves
their bytes without parsing them as queues. Unknown entries, unsafe files and active
owners still block changes. All 26 installation/recovery tests passed, including
three new regressions for update/rollback/recovery, malformed names and unsafe files.
Strict all-target Clippy passed on macOS and for Linux/Windows cross-compilation.
The actual Cargo archive was rebuilt, installed externally without Python, and
passed packaged adoption plus the standalone Rust residue probe.

The final `dc472ca` archive also corrects an installation test-harness race observed
on hosted macOS: in-process locks could briefly be inherited by other tests between
fork and exec. Installation test operations now use actual CLI processes, matching
the production lock lifecycle. All 25 installation tests passed five consecutive
runs, the recovery exclusion test passed, and strict all-target Clippy passed on
macOS and for Linux/Windows cross-compilation. The archive was rebuilt and externally
installed again; packaged adoption passed. Its executable is byte-identical to the
corrected `558061d` candidate because this follow-up changes only tests.

## Observed checks

- The complete workspace at `56a61b6` passed **613 tests and doctests** with all
  features enabled. The existing real-LAN discovery and six-process restart gates
  remained explicitly ignored; no additional tests were skipped for the migration.
- The combined Rust CLI suite at `1ebfea1` passed **124 tests**, including real installation,
  Git worktrees, command descendants, resource contention, coordinator/supervisor
  SIGKILL, recovery and stale evidence. The repository-tool suite passed **156**.
  Both packages passed strict all-target Clippy and formatting. After the final
  portable fixture corrections, all **50 affected installation, recovery and
  workflow tests** were rerun successfully, followed by packaged-binary adoption.
- The actual packaged executable passed the external adoption test: embedded
  setup, package changes and rollback, owner-file preservation, interrupted
  transaction recovery, command execution and deliberate evidence invalidation.
  Cargo, rustc and Python were absent from its runtime PATH.
- A separate bounded documentation task used the Rust plan/queue lifecycle with
  a real isolated Git worktree. Queue revision 4 records integration of source
  `0319c0b3703f325c846f2c8ea11da797e0b0fb40`; run
  `9eea63f1cee6b54142f73f1a754ae04e` passed its declared `git diff --check`
  command and evidence validation after integration. The resulting
  [smoke-test guide](../gameskills-smoke-test.md) is delivered in the migration PR.
  This was coordinator-operated bookkeeping and command verification, not a
  model-driven game task or proof of visual quality.
- Actual Codex Desktop **0.153.4** discovered all **20 selected skills** in the
  core, UI, turn-based, multiplayer and maintainer packages. Native cache bytes
  matched the candidate. User Codex configuration and Claude settings hashes
  were unchanged before/after discovery. No Codex model turn was requested.
- Actual GameKit Cargo archives passed empty, pure, UI and network consumer cases
  against extracted sources. Their activated graphs contained 2, 15, 254 and
  258 packages respectively. These cases use local patches and staged sibling
  version requirements; registry resolution remains unverified.
- All pre-existing queues finished using their original runtime before the active
  repository installation changed. All **232** recorded old bundle, run and
  adopter-owned files remained byte-identical. Old observations keep their
  original identity and are not upgraded into Rust passes.

## Review and cutover

Independent review found and resolved five issues: native stdin write deadlines,
self-consistent but incomplete instruction archives, recovery crossing active
queue/run ownership, durable directory/journal writes, and command lock lifetime
after supervisor death. Each applicable failure has a focused Rust regression;
POSIX fsync correctness is a source-level durability judgment, not a simulated
power-loss claim. A related under-lock queue check prevents interrupted setup
from invalidating pre-read readiness, and operational help works before setup.

The final cutover removes the remaining 13 Python files, completing the migration
of all 22 files and 142 reference methods. Four obsolete legacy install/sync test
contracts are explicitly retired in favor of preservation/import checks. Historical
reference data remains in Git and language-neutral fixtures. Current configuration,
instructions and CI use Rust; `contracts check --verify-reference --cutover`
enforces completed accounting and absence of tracked interpreter sources.
Accounting remains separate from test execution.

A fresh clone at `7be5201` built both tools into a new external target directory
and passed all **156 repository-tool tests and 120 CLI tests** with Python excluded
from its explicit build/test PATH. The allowlist included Rust, Git, compiler tools
and the native `mkfifo`/`ps` fixture utilities. Early attempts exposed those two
missing fixture utilities; after adding them, the relevant package suites passed.
The fresh checkout's repository checks and `--cutover --verify-reference` gate
also passed. No cached installed runtime or interpreter was used by these checks.

## Limits and next trial

Claude Code **2.1.220** reported logged-in status, but the bounded behavioral probe
produced no authenticated model response before termination. Its relative-root
plugin-path bug was fixed and tested; authenticated Claude skill behavior remains
unverified. Native construction and protocol regressions are separate evidence.

Windows supports the portable CLI/installation surface but receives explicit
unsupported errors for queue operations, command execution/evidence and Codex
verification. POSIX supervision covers process groups, not commands deliberately
escaping into another session. Hosted checks in the final PR validate the current
supported platform matrix; they do not establish unsupported Windows execution.

No public package or release was published. Registry names, release ownership,
license/notice completeness and public registry resolution remain release gates
in the [distribution plan](../extraction.md). The next product trial uses this
CLI to make and visually verify owner-selected mechanical and presentation changes
to Labyrinth; Deckbuilder remains the companion pattern and Port Vila is deferred.
