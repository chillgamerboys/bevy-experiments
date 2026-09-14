# Install and update GameSkills

Use this guide to adopt a candidate or change an explicit instruction pin.

## Install a candidate

The current source candidate is unpublished `0.1.0-dev.5`. Source installation requires the
pinned Rust 1.97.1 toolchain. A verified prebuilt executable requires neither Cargo
nor Python at runtime. Git is needed for repository operations; native activation
also needs the selected Codex or Claude CLI.

```sh
cargo install --locked --path gameskills/cli
# Change to the adopter's Git root, or supply --root explicitly.
gameskills catalog
gameskills setup
gameskills setup --apply
gameskills status
```

The executable embeds its compatible instruction baseline. Initial setup requires
no GameSkills source checkout or network fetch. Setup installs **only the 13 core
skills** by default. Add optional packages deliberately, for example:

```sh
gameskills setup --packages gameskills gameskills-ui
gameskills setup --packages gameskills gameskills-ui --apply
```

This repository selects UI, turn-based, multiplayer, maintainer and Linear packages in
`gameskills.toml`. Other games do not inherit those selections. Proposals are
read-only until `--apply`. Applying stages immutable files in `.gameskills/bundles/`,
records their content and runtime compatibility in `gameskills.lock.json`, and
registers selected Codex plugins in project `.codex/config.toml`, preserving
unrelated settings and comments. Commit the
configuration and lock; keep `.gameskills/` out of Git. Recreating the ignored bundle and registration in a fresh checkout is explicit;
it is not the same as selecting a newer pin. See the checkout procedure below.

Codex local marketplaces require an absolute source path, so generated registration
entries in `.codex/config.toml` are machine/worktree-local: exclude a generated-only
file from shared commits, or omit the generated entries when that file also contains
shared owner settings. GameSkills does not change Git ignores or global Codex config.
Each checkout needs its own hydration/registration; a copied path is reported as
outdated. Ownership lives in `.gameskills/native-registration.json`.

Use `setup --bundle PATH --apply` to select a verified compatible bundle for an
update or rollback. Export one with `gameskills bundle --out NEW_DIRECTORY`; pinned
source export uses `--source CHECKOUT --revision FULL_COMMIT`; that checkout must
be at the selected revision with a matching prepared bundle and clean canonical inputs.
Canonical preparation
is the repository tool's responsibility, not an implicit runtime download.
Old immutable bundles remain available. Modified installed content is an error;
edit canonical source or project-owned guidance instead of installed caches.
`setup --recover` restores an interrupted config/lock/project-registration transaction
and refuses to overwrite subsequent user edits. Changed or disabled owned native
values are conflicts; setup will not silently re-enable them. Updates remove obsolete
owned registration values only while they still match the previous write.

## This checkout and existing pins

For repository development, build a local executable without a global install:

```sh
cargo build --locked -p gameskills-cli --profile ci
./target/ci/gameskills --version
./target/ci/gameskills status
./target/ci/gameskills setup
```

These last commands inspect state and propose changes. `setup --apply` without
`--bundle` selects the executable's embedded baseline, even when a different pin
already exists. It is an update operation in that case, not merely hydration.
Compare the proposal's `lock_change` and content identity with `gameskills.lock.json`.

To preserve an existing pin in a fresh worktree, obtain its verified immutable bundle
from a retained installation or pinned source export, then run
`setup --bundle /absolute/path/to/bundle` and inspect the proposal before applying
that same command with `--apply`. Recheck `status` and project registration.
If the current executable's embedded content matches the recorded pin, ordinary
`setup --apply` can restore it. A checked-in lock alone does not provide the ignored
bundle files, and copying `.codex/config.toml` does not relocate absolute paths.

The repository's [setup guide](../../docs/setup-and-launch.md#current-rollout-state)
records its rollout state. CLI version, embedded candidate and installed instructions
are distinct identities: rebuilding or installing the CLI does not update the pin or
reload a running Codex/Conductor session. Read `status.version`, `cli_version`,
`source_commit` and `content_sha256` together. Native agent launch below starts Codex
or Claude; it does not start Labyrinth or another game.

## Native clients

```sh
gameskills native codex
gameskills native claude
gameskills native codex --verify
gameskills native codex --register
gameskills native codex --register --apply
gameskills native codex --verify-project
gameskills native codex --launch
# Additional native arguments follow --.
gameskills native claude --launch -- --help
```

Bare `native codex` or `native claude` returns a command proposal. Codex discovery
uses a bounded app-server session to materialize and hash-check the selected native
plugin cache without requesting a model turn. Launch verifies that discovery first.
Claude uses session-scoped `--plugin-dir` arguments. These operations preserve
global user configuration. Native discovery, authentication and actual model
behavior are different observations; report each against the actual client version.

For an installation staged by an older runtime, `native codex --register --apply`
registers its **existing pin**, without selecting the new executable's embedded
bundle or changing `gameskills.toml`/`gameskills.lock.json`. Its proposal is read-only.
This targeted repair may run during an unfinished queue because it does not change
the pin; it still takes the setup lock. Native settings changes remain source/evidence
inputs and may invalidate an active check's observation. Recover an interrupted
repair with `native codex --register --recover`.

Use `--verify-project` to test an ordinary `codex app-server --stdio` process with
no generated marketplace/plugin enable flags. It checks selected native skills and
cache bytes, and fails if project loading, client support or discovery is missing.
The older `--verify` tests explicit session overrides; it does not establish that an
ordinary host will discover the plugins. Project config loads only for trusted
projects; GameSkills never grants that trust or overrides host policy.

`status` reports bundle validity separately from each selected client's registration,
discovery and session state. A successful registration is not a discovery observation.
Start a new Codex/Conductor session or restart its host after changing registration;
verification of a new process does not prove that an existing session reloaded it.
Claude currently retains session-scoped launch support; persistent registration is
reported as unsupported rather than inferred from staging or Codex success.

The supported registration fields and trusted-project loading are documented in
the official [Codex configuration reference](https://learn.chatgpt.com/docs/config-file/config-reference)
and [configuration basics](https://learn.chatgpt.com/docs/config-file/config-basic).


## Compatibility and support

Configuration and authored plans remain schema 1. Rust locks, queues and execution
records use schema 2 and identify their runtime. Finish any active Python queue with
its original runtime before adoption. Historical bundles, records and local overlays
remain intact. Old records can be inspected as history, but cannot validate or reuse
passes as Rust execution. Run the checks again with the new executable.

The old seven-skill installer's install/sync entrypoints are retired. Use
`gameskills legacy import` to inspect and deliberately adopt the new framework while
preserving `.bevy-gamekit/`, local skills and client-owned instructions.

Queue operations, command execution/evidence supervision and Codex verification
currently require POSIX support. Windows receives explicit unsupported errors for
those operations; its portable configuration and installation tests do not establish
process-supervision support. Authenticated Claude behavior remains a separate
acceptance item when credentials are unavailable. The CLI is not a sandbox for
configured commands.

Private artifact checks precede public publishing. Registry naming, license/notice
completeness and public release ownership remain gates in
[the distribution plan](../../docs/plans/distribution.md). Product trials are tracked
in [framework follow-ups](plans/framework-followups.md).

Documentation mappings are optional and require CLI 0.1.0-dev.3 or newer. Upgrade
the executable before adopting them. See [architecture](architecture.md#documentation-discovery)
for defaults and custom layouts, and [workflow](workflow.md#documentation-context) for use.


Verification policy and `project.delivery_base` are optional adopter choices. Existing
projects retain their branch and checks until explicitly configured. Update to a
compatible executable and bundle before adopting the
[verification fields](../cli/README.md#verification-policy). Validate the candidate
in an isolated consumer, then use supported `setup --bundle PATH --apply`; preserve
prior pins, overlays, records and installed caches. After native readiness is observed,
a running host may still need a new session to load revised skill instructions.
