# Optional Gamekit APIs

## `bevy_game_turns`

`TurnOrder<P>` owns only a unique ordered roster, current cursor, and one-based round.
`advance` returns a typed transition and increments the round on wrap. `remove` returns
a typed removal, selects a successor when needed, and never advances the round. Wrap it
in a game-owned resource if ECS access is useful. Do not move actions or game rules into
the crate.

## `bevy_game_ui`

`GameUiPlugin` owns semantic metrics, native focus, styling, and entity activation.
Attach a game-local action component to `UiAction`; translate `UiActivated.entity` in a
consumer system ordered after `GameUiSystems::EmitActivations`. Supply game branding
through `UiTheme` and `UiFonts`. Keep presentation models and intents local.
Use explicit `UiFocusId` scope/key pairs for restoration across rebuilds; never
derive identity from a displayed label. Pointer and keyboard eligibility is shared.

## `bevy_game_test`

`TestAppBuilder` installs only requested capabilities. `HeadlessUiPlugin` exercises the
real Bevy input/focus/text/UI stack without a renderer. Use `run_frames`, `run_until`,
input helpers, and `ui_tree_snapshot` as mechanics; keep fixtures and assertions in the
owning game.
UI helpers require the `ui` feature. Clipped control rectangles and structural
snapshots do not substitute for rendered and interactive review.

## `bevy_game_multiplayer`

`GameMultiplayerPlugin` composes the secure direct transport but opens no socket on
installation. `PreparedDirectHost`, direct/discovered/reconnect join preparations,
`BGN1` codes, session/peer IDs, and credential stores are infrastructure. Authenticate
before adding `AuthorizedClient`; derive the game seat server-side and keep commands
seatless. Rotate reconnect credentials on every successful reconnect and persist them
atomically without logging their values.
Enable `direct` explicitly. Lifecycle messages carry endpoint entities;
`AuthenticatedPeer` is attached only after game-owned admission. `Receive` and
`GameAuthority` use PreUpdate; `Send` uses PostUpdate before adapter transmission.

## `bevy_game_discovery`

`DiscoveryPlugin` maintains provider-neutral observations, deduplicates by
`SessionId`, prefers LAN then tailnet routes, filters compatibility, and resolves a
secret-free opaque `DiscoveryJoinRoute` that retains alternate routes without
exposing provider endpoints to game UI. The `mdns` and non-default `tailscale-cli`
features perform real I/O only after explicit runtime opt-in. `FakeDiscoveryProvider`
is the deterministic Steam-style/service simulation seam. Discovery metadata is
unauthenticated and must never carry admission secrets. Games own displayed metadata,
password prompts, capacity, and final admission.

`DiscoveryJoinRoute::endpoint(index)` returns `Direct` or opaque `Service` data.
The game composition root dispatches to an adapter. Discovery does not open a
connection. DNS-SD records retain OS-owned lifetimes; provider metadata refreshes
must preserve provider-specific routes. Native I/O is bounded and off-schedule.

## Pure capabilities

`bevy_game_session` owns transport-independent session security and endpoint data.
`bevy_game_hex` owns axial coordinates, neighbors, exact distance, and configurable
2D layout/picking. It knows nothing about board bounds, pieces, movement, or terrain.
