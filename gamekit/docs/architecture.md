# Gamekit architecture

Games depend on capabilities; capabilities never import their consuming games.
Share a stable contract with independent tests and demonstrated needs, not
merely similar code. Keep uncertain abstractions local until experiments establish
the common behavior.

Applications may consume the feature-gated [bevy-gamekit facade](../facade/README.md)
or individual capabilities. The facade only re-exports types; it owns no plugin,
rules or runtime state. Capabilities cannot depend back on it. Library-only external
consumer checks enforce a boundary independent of the games' workspace builds.
See the [consolidation plan](plans/capability-development.md) for staged extractions and
the game-adapted balance harness; no universal combat model is planned.

| Owner | Responsibility | Excludes |
|---|---|---|
| `bevy-gamekit-hex` | Coordinates, neighbors, distance, layout/picking | Boards, pieces, movement rules |
| `bevy-gamekit-turns` | Validated ordered roster, cursor and rounds | Initiative rolls, legality, victory |
| `bevy-gamekit-session` | Pure identity, credentials, admission security | Sockets, Bevy, seats, lobby rules |
| `bevy-gamekit-discovery` | Public listings, provider lifetime, route handoff | Admission or connection construction |
| `bevy-gamekit-multiplayer` | Secure transport adapter, lifecycle, credential stores | Game commands, authority, disclosure |
| `bevy-gamekit-ui` | Input, scoped focus, metrics, opt-in contextual help, skins and primitives | Screens, action enums, game view models |
| `bevy-gamekit-testing` | Deterministic App/input/layout helpers | Game fixtures or visual sign-off |
| Game | Rules, orchestration, schedules, views, content, assets and UX | Other games' private implementation |

Labyrinth's `rules/` package has no Bevy/network/filesystem dependency. Carterfight's
backend stays pure Rust and local to that game. They do not need identical layouts
or a common combat engine. Labyrinth uses rolled initiative; deckbuilder uses cyclic
turns. Carterfight's displayed HP follows narrated events after backend resolution.

See [shared UI integration](ui.md) and
[Labyrinth disclosure](../../games/labyrinth/docs/disclosure.md) for detailed contracts.

## Network flow

Discovery resolves public route data; the composition root chooses its adapter.
An encrypted, certificate-pinned connection still needs admission. An acknowledged,
persisted credential authorizes commands; the host derives the seat from the
connection. Games own readiness, capacity, replay watermarks, private snapshots and
disconnect policy. Lost peers cannot silently become local AI.

Errors are typed and visible. Invalid persisted/network data cannot bypass domain
invariants. Optional discovery failure does not disable Direct joining. Worker
queues, deadlines and frame work stay bounded. See Rustdoc for public contracts and
[network operations](multiplayer.md) for diagnostics.

## Decisions

Games own rules, orchestration and presentation. Capabilities never import their games.
A second consumer supports an extraction decision; it is not a numeric prerequisite.
Keep uncertain abstractions local, and use the smallest independent contract that
solves a demonstrated need. Game-specific status semantics do not become a shared
engine merely because two games use similar names.
