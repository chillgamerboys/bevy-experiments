# Install and update GameSkills

Use this guide to adopt a candidate or change an explicit instruction pin.

## Install a candidate

The current candidate is unpublished `0.1.0-dev.3`. Source installation requires the
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
preserves unrelated configuration and project-owned instructions. Commit the
configuration and lock; keep `.gameskills/` out of Git. Hydrating a fresh checkout
is an explicit setup operation.

Use `setup --bundle PATH --apply` to select a verified compatible bundle for an
update or rollback. Export one with `gameskills bundle --out NEW_DIRECTORY`; pinned
source export uses `--source CHECKOUT --revision FULL_COMMIT`. Canonical preparation
is the repository tool's responsibility, not an implicit runtime download.
Old immutable bundles remain available. Modified installed content is an error;
edit canonical source or project-owned guidance instead of installed caches.
`setup --recover` restores an interrupted config/lock transaction and refuses to
overwrite subsequent user edits.

## Native clients

```sh
gameskills native codex
gameskills native claude
gameskills native codex --verify
gameskills native codex --launch
# Additional native arguments follow --.
gameskills native claude --launch -- --help
```

Without `--launch` or `--verify`, `native` returns a command proposal. Codex discovery
uses a bounded app-server session to materialize and hash-check the selected native
plugin cache without requesting a model turn. Launch verifies that discovery first.
Claude uses session-scoped `--plugin-dir` arguments. These operations preserve
global user configuration. Native discovery, authentication and actual model
behavior are different observations; report each against the actual client version.


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

The migration's private artifact checks precede public publishing. Registry naming,
license/notice completeness and public release ownership remain release gates in
[the distribution plan](../../docs/distribution.md). Labyrinth mechanical and visual changes are
the next collaborative product trial after review of this migration.

Documentation mappings are optional and require CLI 0.1.0-dev.3 or newer. Upgrade
the executable before adopting them. See [architecture](architecture.md#documentation-discovery)
for defaults and custom layouts, and [workflow](workflow.md#documentation-context) for use.
