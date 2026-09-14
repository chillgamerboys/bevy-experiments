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

### Dynamic editors and long control lists

Use `UiFocusId` keys derived from stable subject and field identities, never row
indices or displayed labels. The remembered key belongs to the actual focused
entity; removing a different, unkeyed control or clearing focus cannot revive a
previous row. Duplicate eligible identities fail closed. This restores focus when
a view replaces entities; it does not own drafts, text selections or IME state.
Keep `EditableText` entities mounted while adding/reordering unrelated rows when
possible. Games retain draft values and decide how incoming authority changes
resolve concurrent edits. Consume `UiTextChanged` after
`GameUiSystems::EmitActivations`, before replacing the edited view. A notification
is the current field value, not proof that the string differs from the draft:
native layout/caret work can mark `EditableText` changed. Compare values before
marking a draft dirty, clearing errors or cancelling a pending save.

Bevy flex/grid layout and `Node::overflow` provide automatic rows and scrolling;
Gamekit imposes no control-count limit. `GameUiPlugin` installs native `ScrollArea`
on scroll nodes and requests `ScrollIntoView` when focus changes. `UiTabOrder`
provides logical order independently of transient entities. Long-list capability
tests navigate and activate twelve controls and measure the focused control's
visible bounds; field tests rebuild reordered rows and submit their retained drafts.
These are deterministic input/layout checks, not rendered or real-pointer evidence.
Games still choose wrapping/grouping, reachable confirmation controls, supported
viewport/scale combinations and keyboard inspection for unavailable actions.

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
immediately. Continuous hover locks a card until Escape dismisses it, deepest
linked card first and then the root. Pinned chains ignore other hover sources,
explicit-open actions and ordinary gameplay/outside clicks; linked navigation
within the chain remains available. An accent border indicates the locked state;
there is no close button or Pin/footer row. Source-entity rebuilds preserve stable
subjects, while screen/modal scope changes and disclosure revocation still clear
invalid content. Adopters can explicitly clear state through lifecycle requests.
Retained click focus is not hover; keyboard inspection is explicit via T, with
focus on the deepest card's first link or the card itself when it has no links.
This lifecycle is separate from placement
and never delays display to mask invalid geometry. `UiTooltipDismissOnActivate`
can suppress a transient hint on activation until its source is left.
`UiTooltipAvoid` marks same-host surfaces, such as a visible activity log, whose
measured rectangles placement should avoid. When space is insufficient, placement
minimizes overlap; adopters still own surface organization. Labyrinth omits the
log-toggle and game-menu tooltips entirely and separates non-interactive formation layout anchors
from fitted-art input rectangles. Sprite rendering and those hit rectangles use
the same art-fit calculation; moving the pointer over empty sky is not targeting.


## Composite controls: action bar and character editor

Labyrinth's dynamic action bar and character editor are current consumers of the
primitives above, not shared Gamekit widgets. Use them to establish the following
boundary before extracting a composite. One game-owned character editor should
serve setup and later character inspection through explicit capabilities/modes;
a second screen must not develop a competing character schema or draft lifecycle.
Read-only inspection is a use of that same composition, not permission to expose
undisclosed information. Future stats need no placeholder fields or shared schema.

| Concern | Game supplies and owns | Reusable UI responsibility |
| --- | --- | --- |
| Action content | Stable action/subject IDs, disclosed labels/glyphs, provenance, formatted effects and availability reasons | Render supplied content and selected/disabled states; arrange and navigate all entries |
| Action intent | Selection, targeting, preview calculation, confirmation and authoritative validation | Distinguish selection, explicit inspection and activation; emit identity-bearing UI intent |
| Character content | Subject identity, field/choice descriptors, disclosed current values and editable/read-only capabilities | Compose native controls, sections, labels and field-local explanations |
| Character lifecycle | Draft, parsing/domain validation, source revision, save/cancel/reload policy, pending acknowledgment and conflict resolution | Retain mounted control state where possible; reconcile stable identities and focus when structure changes |
| Presentation | Skin, grouping, terminology and viewport policy | Native sizing/overflow, logical navigation, visible focus and accessible control semantics |

These are ownership boundaries, not proposed Rust types. Rank mechanics, equipment
slots, learned grants, build resolution, ownership and network authority remain in
the game. The UI consumes a game projection; it must not calculate its own damage
or invent a second eligibility rule. Tooltip, preview and control explanations
should derive from the same disclosed source, including a reason when activation
is unavailable. Shared UI may format supplied presentation data without importing
game definitions.

### Action bar consumption contract

Every granted action must remain visible or reachable through an obvious native
scroll/wrap path at supported viewport and UI scales. A numeric shortcut limit is
not a content limit. Keep confirmation and target feedback reachable as the list
grows. Select, inspect and commit are distinct operations: opening a tooltip must
never spend an action, and keyboard confirmation must not bypass a disabled state.
An unavailable but inspectable action needs an eligible inspection path and a
textual explanation, not only a color change.

Identify a rendered entry by subject and action identity, independently of label,
position or transient entity. Bind its emitted intent to the source it represented
and let the game reject stale intents after subject/loadout/encounter changes.
A game-side comparison against the last presented subject/build can enforce this
without changing Gamekit's entity-level activation message. Apply that guard to
confirmation of an existing selection as well as new selection. Stable `UiFocusId`
alone does not make a positional action safe. A keyboard slot
shortcut may deliberately resolve the current visible slot; a queued activation
from an older rendered button must not silently select a replacement slot's action.
Changing subjects clears or revalidates selection, targeting and preview together.

### Character editor consumption contract

Keep one subject/draft lifecycle behind editable and read-only presentations.
Capabilities state which fields/actions are available and why; the game still
validates every submitted change. Retain field entities, text selection and scroll
position through unrelated participant refresh. Stable focus identity supports
structural replacement but does not preserve native caret/IME state by itself.
Subject replacement, explicit reload, preset application, save, cancel and exit
need deliberate draft/focus transitions; old field events must not overwrite the
new draft.

An authority change can retain the user's draft while making Apply unavailable
with a conflict explanation and a reload path. A no-op save still needs an
acknowledgment; a later edit must not be discarded by an older acknowledgment.
Field-value equality must make repeat text notifications harmless. Keep validation
errors near the responsible field and make errors and pending state readable
without relying on color. Read-only mode offers inspection without enabled editing
or a misleading Save action. This policy belongs to the game, including whether
and when unsaved changes need confirmation.

### Extraction and quality evidence

Before adding a shared composite API, identify the repeated mechanical contract
that existing native layout, controls and tooltip APIs cannot express cleanly.
Describe opaque inputs, typed intents, ownership, scheduling and disposal; keep
styling optional and avoid a generic RPG model. Migrate an actual consumer and
show retained behavior. A second consumer is useful transfer evidence, not a quota.
If the only commonality is screen appearance, keep the composition game-owned.

Use this checklist for both local compositions and a proposed shared extraction:

- **Design and task:** establish the player's frequent task, decision information,
  view hierarchy and interaction model before choosing widgets. Use representative
  content and intended art to specify proportions, emphasis, density, transitions
  and recovery at supported scales. Spatial interaction is useful when position
  matters; it does not replace descriptions or comparison. Critique the rendered
  result for context loss, repeated navigation and competing emphasis, even if
  every control fits. A generic card grid is not evidence of a usable composition.
- **Data and intent:** exercise empty and long lists, duplicate labels, reordered
  entries, unavailable actions and subject replacement. Verify IDs and source
  checks prevent stale events from acting on another entry. Selection/inspection
  must not submit gameplay or editor commands.
- **Lifecycle:** test unrelated projection refresh, changed source revision,
  value-equal text notifications, validation recovery, no-op acknowledgment,
  edit-after-submit, reload/cancel and scope exit. Assert draft and emitted-intent
  behavior, not just component existence.
- **Input and layout:** use deterministic fixtures for keyboard order, focus
  restoration, clipping/scroll visibility, disabled activation and modal isolation.
  Exercise actual large content rather than a hard-coded eight-action example.
- **Native interaction:** inspect real pointer targets, keyboard paths, retained
  text caret/selection/IME, tooltip overlap and supported narrow/wide viewport and
  scale combinations. Record the consumer, source revision and observed surfaces.
  Screenshot review does not establish keyboard or IME behavior; deterministic
  fixtures do not establish rendering or pointer feel.
- **Accessibility and disclosure:** provide readable labels/reasons and visible
  focus/selection beyond color, inspect keyboard access to disabled explanations,
  and check that closing or changing scope removes stale disclosed content. Verify
  assistive-technology behavior separately before claiming platform accessibility.
- **Compatibility:** for an API change, check the promised feature combinations,
  affected production consumers and packaged consumer separately. State migration
  and removal/rollback conditions; compilation alone is not composite UI signoff.

Current evidence anchors are the shared [contract tests](../ui/src/contracts.rs),
[tooltip API](../ui/src/tooltip.rs), Labyrinth's
[action controls](../../games/labyrinth/src/ui/battle/dock.rs) and
[editor lifecycle fixtures](../../games/labyrinth/src/ui/setup_tests.rs).
Those sources demonstrate existing mechanics and regression coverage; they do not
establish a shared action-bar/editor API or complete the native interaction checks.
