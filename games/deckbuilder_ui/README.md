# Deckbuilder regression example

A small two-seat listen-host card game that exercises Gamekit UI, turns, discovery
and encrypted transport. It remains runnable; new game-feature development belongs
primarily to Labyrinth. Energy, card effects, private hands and victory stay local.

From the repository root or this directory:

```sh
cargo run -p deckbuilder_ui
cargo test -p deckbuilder_ui --profile ci
```

No `--all-features` is needed for ordinary play. Use pointer or Tab/Shift+Tab and
Enter/Space. Start a local match, select a card, play it, end the turn, or open the
local Game menu. Escape opens it or returns to the previous page. Back to game
closes it; returning to the main menu requires confirmation. The match and
networking continue while the menu is open. Inspect disabled/selected states and
the activity feed. Menu navigation/layout uses shared Gamekit primitives.

Hover a card to preview its rules and current availability; hold the pointer for
one second to keep the explanation open. Each card also has a separate Inspect
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

An established disconnected guest retains its seat/private state; the listing remains
occupied. A lost initial offer expires and frees the pending reservation. Restart the
guest and use its saved credential against the still-running host. Do not manually
delete application-data credentials as part of code/cache cleanup. Host restart is
not recovered. For ports, password limits, permissions and route diagnosis, use
[shared network operations](../../docs/multiplayer.md).

Tests distinguish actual encrypted joins/recovery/private gameplay from fake-provider
listing/handoff. They cover wrong passwords, extra guests, duplicate Hello/ACK, lost
initial/rotating offers and ACKs, stale-attempt messages and failed persistence. They
do not establish cross-machine LAN/Tailscale behavior.

## UI evidence

```sh
cargo run -p deckbuilder_ui --example review_capture --profile ci -- \
  target/review/deckbuilder-1920-auto.png 1920 1080 auto match
```

Use `200` for semantic scaling and `match`, `multiplayer`, `host` or `browser` for
the route. `help-energy`, `help-played` and `help-turn` capture the corresponding
card restriction through the existing production UI actions. These authored
captures do not establish native keyboard/pointer interaction or hover timing.
Review all three supported viewport sizes at Auto/200%, then separately
walk keyboard/pointer, focus, modal, scrolling and resizing. Screenshots are not
gameplay or interaction assertions.
