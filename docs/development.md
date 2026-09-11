# Development and adopter checklist

All commands run from the repository root unless stated otherwise. The checked-in
toolchain pins Rust 1.97.1 for the workspace and its locked Bevy 0.19 dependencies.
Linux builds need the window,
input and ALSA development libraries installed by CI; live LAN browsing also needs
Avahi. Native targets are macOS/Linux/Windows. Capability wasm compile checks do not
promise packaged browser games. Repository/skill tooling uses Python 3.11 or newer
(standard library only); CI selects Python 3.13 explicitly.

The [Rust tooling foundation](../tools/gameskills-cli/README.md) adds explicit
configuration validation and [migration accounting](../tools/gamekit-repo-tools/README.md).
Its new commands run alongside the current Python candidate; installation and
execution have not switched runtimes. Both tool packages initially declare the
tested Rust 1.97.1 minimum and remain unpublished.

```sh
cargo run                         # Labyrinth multiplayer menu
cargo run -- --local              # local Labyrinth battle
cargo run -p carterfight
cargo run -p deckbuilder_ui
```

Running from a game's directory selects that package. Assets must not depend on
the caller's directory. Ordinary runs use dev optimization; automated checks use
`--profile ci`. Changing profiles/features can rebuild dependencies. An all-feature
launch is not the documented default.

## Add a game

1. Create `games/<name>/Cargo.toml`, a composition entrypoint and README with commands,
   controls and initial non-goals. Workspace membership includes `games/*`.
2. Inherit workspace metadata/dependencies; opt into only needed capabilities and
   Bevy features. Keep pure domain code free of Bevy and I/O.
3. Define local domain IDs, typed commands/intents and immutable presentation views.
   Compose schedules explicitly. Offline games do not need native networking.
4. Add local domain fixtures and production-plugin tests. Reuse mechanics, not another
   game's assertions. Shared contracts need independent capability tests.
5. Configure game-owned assets and retain license/provenance. Supply a skin without
   game-name branches in shared UI. Add static and interactive presentation checks.
6. Update navigation and verify launch from root and the game directory.

The standard application entry point is the [facade](../crates/bevy_gamekit/README.md):

```toml
[dependencies]
bevy_gamekit = { workspace = true, features = ["ui"] }

[dev-dependencies]
bevy_gamekit = { workspace = true, features = ["testing-ui"] }
```

Use `bevy_gamekit::ui` and `bevy_gamekit::testing`. Direct capability dependencies
remain supported for pure rules or narrowly scoped adapters. No facade feature
automatically starts a service or composes a game. Networking is an explicit choice.

## Local artifacts

`target/` is the only build tree; review output goes in `target/review/`. `.context/`
is short-lived agent scratch. Enduring requirements belong in normal docs. Neither
generated output nor scratch belongs in Git.

Cleanup is explicit maintenance after stopping workspace processes, never an
automatic startup purge. Do not remove global caches/toolchains, game profiles,
reconnect credentials or Conductor metadata. A cold build recreates local output;
deleting it after every verification wastes that work.
