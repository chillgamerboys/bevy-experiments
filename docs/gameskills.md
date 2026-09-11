# Using the GameSkills foundation

The current candidate is `0.1.0-dev.1`. It implements the approved 12-skill core,
nine optional skills, pinned packages, a durable coordination queue and a command
runner. It precedes the broad documentation/game refactors and the first playable
release. The [catalog](decisions/gameskills-catalog.md) owns workflow boundaries;
this document owns installation and implementation limits.

Python is the current tooling implementation. The foundation and
[bounded refinement trials](decisions/gameskills-refinement-results.md) have merged.
The [repository-wide Rust migration](decisions/gameskills-rust-cli.md) now has an
initial [Rust tool foundation](../tools/gameskills-cli/README.md) for configuration
validation and migration accounting. Installation and execution remain on the
Python candidate. The plan also covers CI, tests and installable distribution.
The installation and workflow commands below describe the current Python
candidate until their Rust replacements are implemented and verified. Skill
instructions remain Markdown and GameKit remains Rust throughout.

## Install a pinned candidate

Use Python 3.11 or newer, Git, and a locally installed Codex or Claude CLI. Inspect
the source revision before adopting it. A release tag or full source commit is
required; moving branches and dirty package sources are refused. Check out the
selected source revision before creating its bundle.

In these examples, set `skills_repo` to the clean GameSkills source checkout,
`skills_commit` to its full commit ID, `game_repo` to the adopter's Git root and
`bundle_dir` to a new temporary output directory. These are explicit paths, not
globally fixed installation locations.

```sh
python3 "$skills_repo/plugins/gameskills/scripts/gameskills.py" --root "$game_repo" \
  bundle --source "$skills_repo" --revision "$skills_commit" --out "$bundle_dir"
python3 "$skills_repo/plugins/gameskills/scripts/gameskills.py" --root "$game_repo" \
  setup --bundle "$bundle_dir"
python3 "$skills_repo/plugins/gameskills/scripts/gameskills.py" --root "$game_repo" \
  setup --bundle "$bundle_dir" --apply
```

This installs **only the 12 core skills**. To add a specialist package, pass the
same explicit selection to both `bundle` and `setup`, for example
`--packages gameskills gameskills-ui`. This repository explicitly selects UI,
turn-based, multiplayer and maintainer packages in its project configuration;
other games do not inherit those choices.

`setup` without `--apply` reports a proposal. Applying stages immutable files in
`.gameskills/bundles/`, writes a portable content lock in `gameskills.lock.json`,
and changes only the package selection in an existing `gameskills.toml`. Existing
project instructions, skills and client settings remain owned by the project/user.
Commit the configuration and lock; keep `.gameskills/` out of Git. A new checkout
needs explicit setup to hydrate the pinned bundle. Each bundle identity includes
source commit and package selection, so selections at the same revision coexist.

Use the installed core path printed through the setup destination, under
`plugins/gameskills/`. The examples below call this path `installed_core`.

```sh
python3 "$installed_core/scripts/gameskills.py" --root "$game_repo" status
python3 "$installed_core/scripts/gameskills.py" --root "$game_repo" native codex
python3 "$installed_core/scripts/gameskills.py" --root "$game_repo" native claude
```

`native` prints the proposed client command. Use `native codex --verify` to verify
native discovery without a model turn. Use `native codex --launch` or
`native claude --launch` to launch the actual client, adding `--` before further
client arguments. Codex first uses its supported app-server plugin/skill APIs to
materialize and verify the selected packages; configuration overrides alone do not
install an uncached plugin. The helper closes that temporary app-server before
starting the requested client. Claude uses session-scoped `--plugin-dir` arguments.

The adapters preserve global user configuration and use the clients' normal
package caches. Staging files or observing a skill catalog does not establish
model behavior. That requires an actual task with the selected candidate.

Updates and rollbacks repeat explicit `bundle`/`setup` with the chosen source and
selection. Old immutable bundles remain available. Modified installed content is
an error; make canonical changes in source or keep adopter guidance in project
instructions/local files. If setup is interrupted, `setup --recover` restores its
previous configuration and lock. Recovery refuses to overwrite subsequent local
edits, which require inspection. Removing a package from the next selected bundle
does not delete unrelated local skills.

## Work and evidence

Use `plan` for the normal entrypoint and a toolbox skill directly for a bounded
request. Creative levels are implement, refine, co-design and explore; level 2 is
the default. Creative latitude does not authorize publication, delegation or merge.
The project's initial worker cap is five; actual host capacity may be lower.

The installed CLI offers `catalog`, `config`, `status`, `plan`, `queue`, `run` and
`evidence`. Full contracts and JSON examples live with the portable core:

- [Work orders, add-only injection and queue transitions](../plugins/gameskills/references/work-orders.md).
- [Command execution, resource locks and evidence validation](../plugins/gameskills/references/verification.md).
- [PR delivery, audit and integration boundaries](../plugins/gameskills/references/delivery.md).

`gameskills.toml` owns project commands, prerequisites, exclusive resources, target
paths, creative defaults and the worker cap. Commands are argument arrays, not
shell strings. Run only the relevant checks, for example:

```sh
python3 "$installed_core/scripts/gameskills.py" --root "$game_repo" \
  run repo-check skills-test repo-test
python3 "$installed_core/scripts/gameskills.py" --root "$game_repo" \
  evidence validate <returned-run-id>
```

The queue validates actual linked worktrees and records reservations and returned
work. It does not launch models. `dispatch` must use the host's supported worker
mechanism and verify actual model/effort/CWD settings. Role labels in work orders
do not select a cheaper model. Inherit current host settings unless an authorized,
validated mapping exists; record real usage before claiming cost improvement.

Worker reports retain caller-supplied references, separately from runner-observed
command exits. Successful commands do not establish acceptance, visual quality,
human enjoyment, PR review or authorization. Dispatch prerequisites may use
reported commits before merge if the consumer actually contains those commits;
merge prerequisites require observed integration.

## Compatibility and remaining validation

Packaging/configuration and structural checks are portable Python. Queue locking
and process/resource cleanup currently target POSIX local filesystems on macOS
and Linux. Windows explicitly lacks these execution mechanisms; CI must not
report skipped Windows execution tests as parity. Native client loading and
agent behavior require separate observations of a specific client and bundle.

The native adapters were inspected with Codex `0.153.4` and Claude Code `2.1.220`
on macOS. Codex discovery verified this repository's pinned candidate
`99be57217d4aab6b2e70272531ea6dcc5838a35b`: all 12 core skills and eight selected
optional skills, with native cache contents matching the immutable bundle.
The Bevy contribution package is not selected here. Claude discovered all 12 core
skills in a preliminary source snapshot, but HTTP 401 authentication failures
prevented any model response or Skill invocation. Claude behavioral compatibility
remains unverified. Native manifests for all six packages pass both validators.

The queue's integration transition currently verifies ancestry-preserving merges
or fast-forwards. Squash/rebase integrations that omit the returned source commit
must retain their externally verified PR evidence; the queue refuses to label
them integrated. The runner uses local digests and before/after input snapshots,
not external attestation or hermetic execution. SIGKILL, machine failure and
deliberately detached processes require external inspection before resuming.

Repository tests cover actual Git state, ownership/dependencies, package changes,
preserved local configuration, recovery, command failure, cancellation and stale
evidence. Native behavior and held-out workflow evaluations are tracked separately
from the structural [scenario fixtures](../skills/tests/gameskills-scenarios.json).
Do not claim a released framework, all-client parity, broad evaluation coverage or
measured cost savings from these checks alone. Labyrinth/Deckbuilder refinement
and the deferred [Port Vila pilot](decisions/port-vila-adoption.md) provide the next
real adopter evidence after this foundation. Begin with the bounded internal
trials; Port Vila remains a later, separately reviewed integration.
