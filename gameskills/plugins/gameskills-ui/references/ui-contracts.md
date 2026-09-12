# Native UI ownership and lifecycle

## View and intent

Render from a game-owned immutable presentation model and return typed intent.
Keep combat, inventory, networking authority, balance and disclosure decisions out
of reusable rendering. Games own branding and content; shared UI can own focus,
measurement and activation mechanics. Previewing or explaining an action does not
grant authority to execute it.

Resolve the installed UI crate source with Cargo metadata. Inspect its module
Rustdoc, production plugin and tests before choosing symbols or scheduling seams.
For GameKit, useful search terms are `GameUiPlugin`, `GameUiSystems`, `UiActivated`,
`UiAction`, `UiFocusId`, `UiControlMetrics`, `UiSpacing`, `UiSkin` and `UiTooltip`.
These hints are not a copied API contract. Read the actual enabled features and
public exports; optional skin, tooltip and context plugins remain composition-root
choices. When messages have multiple consumers, keep independent readers rather
than draining the shared stream. Follow the producer's public ordering seam.

## Layout and input

Use semantic color, typography, spacing and control roles. Resolve metrics from
declared baselines so repeated resizing/scaling does not compound previous output.
Logical canvas size, OS device scale and semantic accessibility scale are distinct
inputs. Establish target sizes and typography for the actual audience/platform;
fixed 18-pixel text or 44-pixel controls are not universal GameSkills gates.

Keep primary actions reachable as secondary regions collapse. Use stable semantic
names, accessible labels and focus identity independent of displayed text. Apply
pointer/keyboard activation parity for supported controls. Hidden, clipped or
disabled controls must not retain actionable focus; inspect focused targets after
scrolling, clipping and rebuilds. Focus restoration requires a still-eligible
control in the correct scope, not merely a surviving entity ID.

Blocking modal ownership includes trapping focus at the top active scope, input
containment, closing order and restoring eligible prior focus. Handle Tab/reverse
Tab, Enter/Space, Escape, scrolling and text entry according to the game's actual
control contract. An inspectable disabled action may explain itself without being
enabled. Pointer-transparent help must not become a gameplay input blocker.

## Context and tooltip regressions

Read timing/defaults from the installed source or accepted game design rather than
copying a previous game's one-second setting. Test preview departure, continuous
hover threshold, locking/persistence, source replacement, nested help, explicit
keyboard inspection, outside/close/Escape dismissal and modal cleanup separately.
Pointer hover should not silently steal keyboard focus. Retained click focus must
not accidentally reopen a dismissed preview. Stationary-pointer close events must
not click through or expose an underlying source immediately.

Keep preview/locked geometry intentional: a hidden footer must not reserve space.
Check geometry across the state transition, not only two authored still frames.
Contain game shortcuts when a help scope captures keyboard input. Use stable
subject keys and revoke cached/locked content on disclosure or scope changes;
labels, previews and nested help cannot disclose facts absent from the player view.

For multiplayer menus, treat discovered metadata as untrusted presentation. Show
meaningful compatibility/lock/freshness states, keep provider endpoints and secrets
out of listings, clear transient password/code buffers after attempts and retain
the configured direct-join route when optional discovery is unavailable.
