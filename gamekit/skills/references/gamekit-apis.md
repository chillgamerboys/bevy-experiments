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

## `bevy_game_test`

`TestAppBuilder` installs only requested capabilities. `HeadlessUiPlugin` exercises the
real Bevy input/focus/text/UI stack without a renderer. Use `run_frames`, `run_until`,
input helpers, and `ui_tree_snapshot` as mechanics; keep fixtures and assertions in the
owning game.
