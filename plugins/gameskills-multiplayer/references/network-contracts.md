# Multiplayer authority and boundaries

## Map owners before protocols

Separate discovery observations, route selection, encrypted transport, admission,
authoritative game state and recipient presentation. A connected transport is not
an admitted player. Authorize before accepting game commands; bind identity/seat
server-side instead of trusting a seat claimed in payloads. Games own capacity,
roster/lobby transitions, action legality, rejection and disclosure.

Discovery is untrusted, sanitized availability. It cannot contain passwords,
bearer invitations, reconnect credentials or full connection codes. Keep provider
endpoints out of game UI; pass opaque route data to a composition-root adapter.
Transport-independent identity/security types belong below discovery and concrete
transport. Discovery handoff is data, not a function that directly mutates a World
to open a specific transport. Test providers with no direct endpoint as well as
LAN-like ones.

Keep service lifetime, cancellation, endpoint ownership, error state and I/O budgets
explicit. Bound polling and perform blocking provider work off the game schedule.
Optional discovery failure should leave the configured secure direct route usable.
Provider-specific refreshes must preserve other usable routes. LAN multicast and
a tailnet provider are distinct mechanisms; do not diagnose mDNS across a tailnet
as a test of the tailnet adapter.

## Session and disclosure lifecycle

Register protocol messages/compatibility identities before opening endpoints.
Bind admission attempts to their actual connection and reject stale responses.
Persist reconnect credentials atomically without logging secrets. When the
protocol uses credential acknowledgement before admission, verify the persist →
acknowledge → authorize ordering; a sent credential alone is not durable recovery.
Rotate credentials after successful reconnect and test replay/revocation, including
after any cache eviction. Establish client restart and host restart as separate
requirements with their own state owners.

Generate each recipient's allowed snapshot from game-owned visibility. Exercise
partial disclosure and hidden-input variation with constant public state, including
derived values, errors, logs, labels and help. A game view being filtered does not
prove network payloads are filtered. Inspect serialized output for each recipient.

## Inspect installed APIs

Resolve dependency source through Cargo metadata and read version-matched Rustdoc,
public types, production composition and tests. For GameKit, search
`bevy_game_session`, `SessionAdmissionAuthority`, `bevy_game_multiplayer`,
`GameMultiplayerPlugin`, `AuthorizedClient`, `AuthenticatedPeer`,
`bevy_game_discovery`, `DiscoveryJoinRoute` and `FakeDiscoveryProvider`.
These identify seams to inspect, not current signatures or a complete API manual.
Opt-in features and installing a plugin are not evidence a socket/service started.
