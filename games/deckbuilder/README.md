# Deckbuilder regression example

A small two-seat listen-host card game that exercises Gamekit UI, turns, discovery
and encrypted transport. It remains runnable; new game-feature development belongs
primarily to Labyrinth. Energy, card effects, private hands and victory stay local.

From the repository root or this directory:

```sh
cargo run --locked -p deckbuilder --profile ci
```

The initial window request is 1440×900 logical pixels. See [setup and launch](../../docs/setup-and-launch.md)
for build profiles and the separate 1080p verification target. No test suite or
`--all-features` is needed for ordinary play. Use pointer or Tab/Shift+Tab and
Enter/Space. Start a local match, select a card, play it, end the turn, or open the
local Game menu. Escape opens it or returns to the previous page. Back to game
closes it; returning to the main menu requires confirmation. The match and
networking continue while the menu is open. Inspect disabled/selected states and
the activity feed. Menu navigation/layout uses shared Gamekit primitives.

Hover a card to preview its rules and current availability; hold the pointer for
two seconds to keep the explanation open. Each card also has a separate Inspect
control: use Tab/Shift+Tab and Enter/Space to read it even when the card cannot be
played. `T` pins the hovered or focused explanation and enters keyboard inspection;
Escape closes pinned inspection; press Escape again to open the Game menu.
Inspection never selects or plays a card. Unavailable cards explain already-played status first, then turn ownership,
then insufficient energy. Help is rebuilt from your current private hand whenever
the match view refreshes.

## Two-player checks

Host with a reachable advertised address. Copy the private BGN1 invitation to the
guest, or enable LAN/tailnet discovery with a temporary passphrase and select the
listing. Ready both seats and start. Each recipient must see only its own hand;
the host validates actions and retains a live-session command watermark even after
evicting cached results. Refused commands also consume their sequence.

The current game protocol is v3. Its custom-auth Hello explicitly checks the
deckbuilder schema; BGN1 is a route format, not compatibility with older game builds.
Old peers are refused, not silently adapted. Private invitations initially expire
after one hour. Pending admission does not authorize gameplay: persist the offered
credential, acknowledge it, then accept the matching welcome. A storage failure
cancels joining rather than admitting a guest that cannot recover after restart.

The welcome and private snapshot use separate ordered channels. If the snapshot
arrives first, the guest holds one matching-attempt snapshot until the persisted
offer's welcome is accepted. It does not expose that hand before admission, and a
replacement attempt cannot inherit it.

An established disconnected guest retains its seat/private state; the listing remains
occupied. A lost initial offer expires and frees the pending reservation. Restart the
guest and use its saved credential against the still-running host. Do not manually
delete application-data credentials as part of code/cache cleanup. Host restart is
not recovered. For ports, password limits, permissions and route diagnosis, use
[shared network operations](../../gamekit/docs/multiplayer.md).

Tests distinguish actual encrypted joins/recovery/private gameplay from fake-provider
listing/handoff. They cover wrong passwords, extra guests, duplicate Hello/ACK, lost
initial/rotating offers and ACKs, stale-attempt messages and failed persistence. They
do not establish cross-machine LAN/Tailscale behavior.
Socket-admission failures retain bounded handshake stages, host connection and
reservation counts, frame gaps and sanitized disconnect categories to distinguish
transport failure from game admission without exposing credentials or endpoints.

## UI evidence

```sh
cargo run -p deckbuilder --example review_capture --profile ci -- \
  target/review/deckbuilder-1920-auto.png 1920 1080 auto match
```

Use `200` for semantic scaling and `match`, `multiplayer`, `host` or `browser` for
the route. `help-energy`, `help-played` and `help-turn` capture the corresponding
card restriction through the existing production UI actions. These authored
captures do not establish native keyboard/pointer interaction or hover timing.
For changed UI presentation or interaction, inspect the affected route at 1920×1080
Auto and exercise its relevant inputs. Logic-only fixes need no agent UI walk.
The `deckbuilder-ui-normal` suite runs normal-display behavioral checks; retained
`compatibility` tests cover other sizes/scales when a defect or Release support
requires them. End-to-end checks follow affected journeys. Screenshots are not
gameplay or interaction assertions. Follow the shared
[rigor and milestone policy](../../docs/testing.md), including milestone-only
developer sanity for game effects.
