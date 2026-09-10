# Architecture and ownership

Games depend on capabilities; capabilities never import their consuming games.
Share a stable contract with independent tests and two plausible consumers, not
merely similar code. Keep uncertain abstractions local until experiments establish
the common behavior.

| Owner | Responsibility | Excludes |
|---|---|---|
| `bevy_game_hex` | Coordinates, neighbors, distance, layout/picking | Boards, pieces, movement rules |
| `bevy_game_turns` | Validated ordered roster, cursor and rounds | Initiative rolls, legality, victory |
| `bevy_game_session` | Pure identity, credentials, admission security | Sockets, Bevy, seats, lobby rules |
| `bevy_game_discovery` | Public listings, provider lifetime, route handoff | Admission or connection construction |
| `bevy_game_multiplayer` | Secure transport adapter, lifecycle, credential stores | Game commands, authority, disclosure |
| `bevy_game_ui` | Input, scoped focus, metrics, opt-in skins and primitives | Screens, action enums, game view models |
| `bevy_game_test` | Deterministic App/input/layout helpers | Game fixtures or visual sign-off |
| Game | Rules, orchestration, schedules, views, content, assets and UX | Other games' private implementation |

Labyrinth's `rules/` package has no Bevy/network/filesystem dependency. Carterfight's
backend stays pure Rust and local to that game. They do not need identical layouts
or a common combat engine. Labyrinth uses rolled initiative; deckbuilder uses cyclic
turns. Carterfight's displayed HP follows narrated events after backend resolution.

## UI flow

Input eligibility -> entity activation -> game-local intent -> authoritative model
-> immutable view -> presentation. Independent consumers use independent readers;
never drain shared messages or mutate rules from a widget. Public system sets expose
actual dependencies; plugin insertion order is not a scheduling contract.

Input mechanics do not prescribe a skin. Games opt into semantic painting and may
override surfaces, fonts and interaction states. Scope focus identity with stable
keys, not displayed text. Games own selection, inspection and scene composition;
the toolkit owns reusable accessibility mechanics.

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
