# Command execution and evidence

Use the installed runtime through its actual path, shown here as
`<gameskills-executable> --root <repository>`. It requires
Git and a committed Git worktree. Prebuilt execution requires neither Cargo nor an interpreter. Readiness/configuration
is a separate operation; reading evidence does not perform setup.

## Commands and dependencies

The project owns `gameskills.toml`. A command names an argument vector, an
optional repository-relative working directory, timeout, prerequisite commands,
and exclusive resources:

```toml
[dispatch]
max_workers = 5

[commands.rules]
argv = ["cargo", "test", "-p", "my_rules"]
cwd = "."
timeout_seconds = 600
resources = ["project:cargo-target"]
requires = []

[commands.app]
argv = ["cargo", "test", "-p", "my_game"]
requires = ["rules"]
resources = ["project:cargo-target", "gpu", "window"]
```

```text
gameskills --root ROOT run rules app
gameskills --root ROOT run app --max-workers 2 --resource-wait-seconds 60
gameskills --root ROOT evidence list
gameskills --root ROOT evidence show RUN_ID
gameskills --root ROOT evidence validate RUN_ID
gameskills --root ROOT run app --resume RUN_ID
```

`run` executes the selected commands and their transitive `requires` graph.
Selection, prerequisite traversal and ready-work ordering are deterministic by
command name. Actual completion order depends on the processes. Each selected
node executes at most once in a run. A failed, timed-out, interrupted or skipped
prerequisite skips its dependents; independent useful work continues.

Command names use 1–64 ASCII letters, digits, underscores, dots or hyphens, starting
with a letter or digit. Cycles, unknown prerequisites, invalid argument vectors,
unsafe working directories and invalid limits fail before command execution.
Working directories must stay inside the repository and cannot traverse symlinks,
`.git` or `.gameskills`. Commands are trusted project programs; this is not a
program sandbox. Arguments are passed without shell interpolation. Explicitly
configuring a shell executable still deliberately runs that shell.

The default per-command timeout is 600 seconds. Ready commands wait up to 30
seconds for resources unless `--resource-wait-seconds` changes that limit. A wait
that expires records a skip, never a pass. At most `dispatch.max_workers` commands
run within one invocation; its default is five. `--max-workers` can lower that
cap. Separate invocations have separate worker pools. Shared resource locks
coordinate those invocations, so commands that compete for a machine or target
directory must declare the same resource names.

Resources are exclusive strings, acquired as one set without deadlocking. Plain
names such as `gpu`, `window`, `audio`, and `port:5000` share one user-wide namespace
across repositories. Names beginning with `project:` are additionally scoped to
the Git common directory, so worktrees of one repository share those resources.
Other programs that do not use these locks are not coordinated automatically.
The runtime stores persistent lock files under the checked, owner-owned
`/tmp/gameskills-resources-<uid>` directory; process-held OS locks, not file
existence or stale PID guesses, determine availability. Do not delete live lock
files to unlock resources: that creates a second lock identity.

## What is observed

Each run creates an exclusive 32-character hexadecimal run-ID directory at
`ROOT/.gameskills/runs/RUN_ID/`. It keeps `record.json`, an active-run lock, and
separate stdout/stderr files for every command that starts. Records contain the
selected graph, actual arguments and working directory, start/end times, elapsed
time, observed exit code, command states, output digests and any cleanup errors.
Failures and partial output remain available. Missing executables also produce
a failed observation and diagnostic log. There is no command that accepts a
caller-written success, acceptance or human-review status.

Execution and resource cleanup use POSIX `flock`, directory descriptors and new
process groups. macOS and Linux are the supported mechanism; Windows currently
fails explicitly instead of providing weaker cleanup or locking guarantees.
The runner closes stdin, captures output directly to files, and terminates then
kills the command process group after timeout, SIGINT or SIGTERM. It also cleans
up group descendants left behind by a successful command leader. Commands must
not deliberately detach into new sessions or leave daemons running. Uncatchable
termination such as SIGKILL or a machine crash can leave an incomplete record
and processes that need external inspection/cleanup; it cannot establish success.
Launched command leaders inherit the active-run and resource locks, so a leader
that survives SIGKILL still prevents a competing run or immediate resume. Programs
that deliberately close those descriptors or detach children are outside that
recovery protection.

The source identity includes canonical worktree and Git directories, HEAD and its
symbolic branch identity, and a digest of actual Git ref names, tips and symbolic
targets. Ref storage as loose files or packed refs does not change this identity.
Advancing a review base invalidates prior evidence even when HEAD is unchanged.
This is conservative: creating or changing unrelated branches/tags can also
invalidate evidence until commands support narrower declared Git inputs.
The identity also includes staged changes, file modes and contents of tracked and nonignored untracked
files, and nested Git identities for populated submodules. Generated
`.gameskills/` state is excluded. `gameskills.toml` and `gameskills.lock.json`
have separate raw-file digests; effective project configuration and the selected
normalized command graph also have digests. The identity records resolved command
executables and their digests, Rust executable/platform identity and a digest of the
inherited environment, excluding only shell bookkeeping (`PWD`, `OLDPWD`,
`SHLVL`, `_`). Environment values are not copied into the record.

Inputs are checked before and after execution. A command can exit zero while the
run is stale because inputs changed during the check. Validation recomputes the
current identity and checks the complete result graph, zero exit codes and
stored output digests. `evidence show` includes that validation alongside the
record. `evidence list` only lists recorded states and explicitly does not
validate them. Treat `ok: true` from `evidence validate`, with its command-only
claim, as the result for reuse; a saved `passed` string by itself is insufficient.

Records are written atomically and contain a digest of their canonical contents.
State access rejects traversals, symlinks, hardlinked evidence files, special
files and state writable by other users. A run never overwrites another run.
These are local workflow integrity checks, not cryptographic attestation against
the machine owner: someone able to rewrite records and recompute their digests
can forge them. Use trusted execution/CI and its independent records when an
external acceptance process needs stronger provenance.

Evidence does not fully describe arbitrary external inputs. Ignored build caches,
external assets, services, toolchain internals, network responses, hardware state,
deliberately escaped processes and files reached through source symlinks are not
all fingerprinted. Changing and restoring an input between snapshots is also
outside this model. Declare necessary resources and keep relevant inputs in the
repository/configuration; rerun when an untracked dependency may have changed.
No general guarantee of hermeticity or granular dependency-aware reuse is made.

## Interruption, resume and judgment

`run ... --resume RUN_ID` requires the same selected graph and current input
identity as that prior run and refuses an active runner. It creates a new record,
links `resumed_from`, and runs the complete graph again. It does not copy passed
states, relabel an old record or change its source identity. A changed input needs
a fresh `run` without `--resume`; the earlier record stays available. Inspect and
clean up possible surviving processes after an uncatchable termination before
starting replacement work.

Successful command evidence supports only the behavior those commands actually
observe. Pure tests, Bevy `App` tests, structural checks, rendered captures,
native interaction and clean-log observations answer different questions. A
passing script alone does not establish visual correctness, native interaction,
playability, human enjoyment, review acceptance, complete PR audit, publication
authorization or merge authorization. Keep review findings, manual observations,
player feedback and their limitations separate. This helper performs no review
acceptance, PR creation, merge or release automation.

## Runtime transition

Rust execution records use schema 2 and identify runtime `rust`. Existing Python
records remain historical and cannot validate or resume as Rust execution. Keep
original records intact and rerun checks to create new evidence. Setup coordinates
with run registration and refuses updates while active run locks are held; an
abrupt coordinator exit does not release a supervisor's command locks early.
