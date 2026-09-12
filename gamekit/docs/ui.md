# Shared UI integration

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
Previews appear immediately and are pointer-transparent. Leaving before the
configurable `UiTooltipSettings::lock_delay` (1 second by default) dismisses them
immediately. Continuous hover locks a card until explicit dismissal, source
replacement or a scope/disclosure change. An accent border and corner × indicate
the locked state; there is no Pin/footer row. Retained click focus is not hover;
keyboard inspection is explicit via T. This lifecycle is separate from placement
and never delays display to mask invalid geometry. `UiTooltipDismissOnActivate`
can suppress a transient hint on activation until its source is left.
`UiTooltipAvoid` marks same-host surfaces, such as a visible activity log, whose
measured rectangles placement should avoid. When space is insufficient, placement
minimizes overlap; adopters still own surface organization. Labyrinth omits the
log-toggle and game-menu tooltips entirely and separates non-interactive formation layout anchors
from fitted-art input rectangles. Sprite rendering and those hit rectangles use
the same art-fit calculation; moving the pointer over empty sky is not targeting.
