# Development and adopter checklist

All commands run from the repository root unless stated otherwise. The checked-in
toolchain pins Rust 1.97.1 for the workspace and its locked Bevy 0.19 dependencies.
Linux builds need the window,
input and ALSA development libraries installed by CI; live LAN browsing also needs
Avahi. Native targets are macOS/Linux/Windows. Capability wasm compile checks do not
promise packaged browser games. Repository maintenance and GameSkills runtime logic
are Rust; metadata and agent instructions remain data and Markdown.

The [Rust GameSkills CLI](../gameskills/cli/README.md) handles installation,
configuration, queues and command evidence. The separate
[repository tool](../devtools/README.md) owns validators, CI and
distribution checks. Both packages declare the tested Rust 1.97.1 minimum and remain
unpublished. Prebuilt runtime adoption does not require a Rust compiler.

```sh
cargo run                         # Labyrinth multiplayer menu
cargo run -- --local              # local Labyrinth battle
cargo run -p carterfight
cargo run -p deckbuilder
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

The standard application entry point is the [facade](../gamekit/facade/README.md):

```toml
[dependencies]
bevy-gamekit = { workspace = true, features = ["ui"] }

[dev-dependencies]
bevy-gamekit = { workspace = true, features = ["testing-ui"] }
```

Use `bevy_gamekit::ui` and `bevy_gamekit::testing`. Direct capability dependencies
remain supported for pure rules or narrowly scoped adapters. No facade feature
automatically starts a service or composes a game. Networking is an explicit choice.

## Local artifacts

`target/` is the only build tree; review output goes in `target/review/`. `.context/`
is short-lived agent scratch. Enduring requirements belong in normal docs. Build
output and scratch stay out of Git. The explicit exception is the deterministic
instruction snapshot under `gameskills/cli/bundle/`: commit it with its source
pin after `repo-devtools bundle prepare`, and verify it with `bundle check`.

Cleanup is explicit maintenance after stopping workspace processes, never an
automatic startup purge. Do not remove global caches/toolchains, game profiles,
reconnect credentials or Conductor metadata. A cold build recreates local output;
deleting it after every verification wastes that work.

## Documentation and skills

Current docs explain supported behavior and how to develop or troubleshoot it.
Keep docs with their owner; root docs cover shared concerns. A README is the
entrypoint, with a docs index when several topics need navigation. Put rationale
in the relevant page's Decisions section. Use substantial active/deferred plans
for future work; remove completed plans after updating current guides and links.
Git retains history. Do not keep separate decision or history archives.

Skills resolve adopter docs through `gameskills docs resolve --path PATH`. The
optional `[docs]` and `[targets.NAME.docs]` mappings identify root-relative indexes
and plan directories. Preserve existing adopter layouts; no empty plan directories
are required. Read the returned relevant sections, not every page recursively.

Update docs and skill pointers with their changed behavior. Packaged skill references
remain self-contained reusable guidance; local APIs/commands belong to source,
Rustdoc and current project docs. Do not edit installed bundles or frozen fixtures.

Run `gameskills run docs-check` with the selected candidate, or the repository
check directly while developing that candidate. Checks validate current local links,
heading anchors, declared indexes and active/deferred plan status. Review establishes
accuracy and completion; a structural check cannot infer either or delete files.

The local anchor checker supports ATX/setext headings, ordinary inline emphasis,
code/link text, common HTML entities, duplicate heading suffixes, Unicode letters,
percent-encoded destinations and explicit quoted HTML id/name anchors. It ignores
fenced examples and external URLs. It is a documented subset, not a full GitHub
Markdown implementation; use simple headings or explicit anchors for unsupported
formatting. Compatibility fixtures use their existing separate validators.
