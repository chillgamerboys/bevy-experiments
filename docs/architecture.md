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
| `bevy_game_ui` | Input, scoped focus, metrics, opt-in contextual help, skins and primitives | Screens, action enums, game view models |
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

### Local menus and activity feeds

`UiMenuStack<Route>` stores bounded, deduplicated local page history using a
game-owned route type. `menu_overlay`, `menu_panel` and `menu_actions_node` provide
native layout and the existing modal focus contract. Labyrinth and deckbuilder
adopt these templates, but own their menu labels, pages, leave consequences and
settings actions. Opening a menu does not pause `Time<Virtual>`, send a network
command, or suspend authority. A party-wide pause would be a separate game policy.

`GameUiFeedPlugin` and `UiFeedScroll` follow a native scroll container's latest
content until the reader scrolls away. Games supply a monotonic revision, choose
retention/grouping and render a Latest/unread control; replace the component when
switching feeds. The shared component holds no combat events or log strings.
Labyrinth retains its typed authoritative history, groups action outcomes and
opens disclosed ability explanations through the existing tooltip catalog. History
is non-modal and can be fully hidden. Labyrinth's actor/initiative details use
contextual cards rather than a second inspection menu. Only local game menus and
explicit keyboard tooltip reading capture their respective input scopes.
Passive tooltip previews do not consume Escape or history paging keys. Pinned,
nested or actively read tooltips retain their own navigation priority.

### Contextual information without a shared screen design

`GameUiContextHelpPlugin` is opt-in beside `GameUiPlugin`. Games attach plain,
already-disclosed `UiContextHelp { title, body }` content to existing native
controls and consume `UiContextHelpState` after `UiContextHelpSystems::Resolve`
in `Update`. This selects a hovered or keyboard-focused source; it does not draw
a popup, change focus, emit an action, or introduce another pointer hit-test path.
It shares activation eligibility for disabled, hidden and modal-blocked controls,
and excludes sources with no visible clipped bounds. A new keyboard focus wins
over stationary hover; fresh pointer activity can reclaim the context. Post-layout
validation clears a removed, hidden or scrolled-away source.

The same selection contract can serve a deckbuilder card, a Carterfight move, or
a Labyrinth ability. The games still choose the information, inspection action,
placement and skin. Labyrinth uses a stable character command dock and pinnable
contextual cards; another adopter need not use either. An unavailable action that
remains inspectable is a game-owned distinction, not permission for a disabled
`UiAction` to activate. Keep explanations on an explicitly eligible inspection
control when disabling activation entirely.

Tooltip sizing is constrained before native layout. Floating placement translates
the measured card and every descendant after `UiSystems::Layout` and before
`PostLayout` clipping/text processing, within the same frame. It does not write
late `Node` offsets or wait for a later frame to reveal valid geometry. The host
is a full-target screen root; cards retain native layout and scroll behavior.
The configurable 150 ms preview dwell is a reading affordance, not a rendering
workaround. `UiTooltipDismissOnActivate` lets surface-opening controls suppress
their hints until hover/focus leaves. It does not suppress ability explanations
by default or close explicitly pinned inspection.
`UiTooltipAvoid` marks same-host surfaces, such as a visible activity log, whose
measured rectangles placement should avoid. When space is insufficient, placement
minimizes overlap; adopters still own surface organization. Labyrinth omits the
log-toggle tooltip entirely and separates non-interactive formation layout anchors
from fitted-art input rectangles. Sprite rendering and those hit rectangles use
the same art-fit calculation; moving the pointer over empty sky is not targeting.

### Labyrinth forecasts and viewer knowledge

Labyrinth keeps its combat organization, glyph vocabulary, ability/rank diagrams,
target selection and confirmation local. Flat prototype surfaces are replaceable
appearance, not a contract imposed on other games or future Labyrinth art.

The pure rules package forecasts immediate ordered effects through the same
resolver used by committed actions. A forecast does not consume RNG, roll
initiative, advance a turn, or authorize a command. Labyrinth's presentation layer
separates public authored/base effects from target-specific outcomes and marks
undisclosed information as unknown instead of substituting zero. Periodic effects
describe their timing and remaining opportunities, not guaranteed future totals.
These calculations and disclosure policies do not belong in `bevy_game_ui`.
The current viewer contract leaves identity, allegiance, rank and standing/downed
state public; exact HP amounts, conditions and inspection details can be unknown.
Basic targeting legality can therefore still reflect public standing state.

**Current disclosure is a presentation seam, not network secrecy.** Labyrinth's
multiplayer snapshots still contain the complete combat state, and the normal
encounter is fully revealed. Hidden-information fixtures exercise what the UI can
represent. Actual reveal abilities or concealed enemy facts will require
recipient-filtered snapshots, events and logs before transmission, plus disclosure
tests at that boundary. Hiding a widget or suppressing a log on a client cannot
remove information already sent to it.

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
