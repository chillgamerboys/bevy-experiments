# Capability refactor and adopter migration

PR #14 was stabilized, verified on macOS/Linux/Windows, and merged before publishing
this breaking refactor. Legacy tactics/Carterfight sources and assets are unchanged.

## API changes

- Use `bevy_game_session` for engine-independent identities, password verification,
  connection codes, and endpoint data. Multiplayer retains compatibility re-exports;
  discovery has no dependency on Aeronet, Replicon, or WebTransport.
- `DiscoveryJoinRoute::endpoint(index)` returns `DiscoveryEndpoint`. Dispatch `Direct`
  to `PreparedDirectDiscoveryJoin` in the game composition root. `Service` contains
  an opaque public lobby locator and needs its own adapter. An unsupported adapter
  returns an explicit error; it does not reinterpret the locator as a UDP endpoint.
- Enable multiplayer's `direct` feature explicitly. `GameMultiplayerPlugin` never
  opens a socket by installation alone. `Receive` and `GameAuthority` run in
  `PreUpdate`, after Replicon's receive systems. `Send` runs in `PostUpdate` before
  Replicon's send systems. Lifecycle messages identify the actual endpoint entity.
  After game admission, attach `AuthenticatedPeer` to that connection; the plugin
  emits authentication, leaving, disconnect, failure, and listener-close messages.
- Native DNS-SD registration, updates, and browsing run on workers with bounded
  mailboxes. Metadata updates retain the provider's endpoint. DNS-SD retains live
  records and renews registry leases until removal or certificate expiry. Drop
  signals worker shutdown and unregisters asynchronously; it never blocks a frame.
- Tailscale is still external, optional development infrastructure. CLI work has a
  two-second deadline and a 1 MiB output cap. The deckbuilder starts at most one
  status request per browser, refreshes ten seconds after completion, and performs
  no CLI request when tailnet browsing is off. Receive work is capped at 64 packets.
- Attach `UiFocusId { scope, key }` to controls needing restoration across view
  rebuilds. Labels/`Name` are not identities. Duplicate identities cannot be used for
  fallback restoration. Modal focus is stacked; pointer and keyboard activation use
  the same ancestor/disabled/modal eligibility check. Scrollable nodes receive native
  `ScrollArea` support and focused controls scroll after layout. Styling is change-driven.
- Enable `bevy_game_test/ui` for headless UI and snapshot helpers. The default test
  capability has no renderer or shared UI dependency. `visible_control_rect` is a
  clipped axis-aligned structural observation, not a pixel/occlusion assertion.
- `TurnOrder`, discovery metadata, direct endpoints, and persisted reconnect bindings
  validate deserialization. Previously accepted invalid data is now rejected.

## Deckbuilder protocol and secrets

Deckbuilder protocol v2 uses monotonic per-seat request sequences. The host retains
a watermark for its entire live session, even after result-cache eviction and guest
disconnect. Refused commands consume their sequence too. Full private snapshots tell
the recipient its next sequence, allowing a fresh guest App to resume safely.
Older deckbuilder protocols are refused; BGN1 remains a route/admission code format,
not a promise of game-protocol compatibility. Host restart recovery is not provided.

Guests explicitly disconnect on menu exit and refusal. Hosts flush a generic refusal
then close the rejected connection; silent unauthenticated connections time out.
Private-code-only hosting permits an empty passphrase when both providers are off.
Providing a passphrase enables password admission independently of provider choice.

Owned UI message values and password/code buffers redact diagnostics and zeroize on
drop, replacement, and submission. Encoded connection codes and serialized credential
file buffers also zeroize. This is not a claim of complete process-memory erasure:
Bevy's native text editor, platform IME/clipboard, rendering, allocator history, and
transport serialization can have copies outside Gamekit's ownership. Never reuse an
important password; clipboard sharing remains an explicit user action.

## Skills

The baseline remains seven canonical skills, rendered for both clients. Installation
requires a real checked-out release tag or full commit SHA, not an arbitrary branch.
Manifest schema 2 records both the requested revision and resolved immutable SHA.
Sync checks the complete cached base file set and SHA-256 hashes before classifying
changes. Schema 1 installations may upgrade after base verification. Symlinked or
escaping generated destinations are rejected; overlays remain untouched.

The validator checks metadata, referenced files, fixture structure/contradictions,
and rendering parity. It explicitly does **not** evaluate whether an agent selects
or follows a skill correctly. Trigger-behavior evaluation requires recorded agent runs.

## Evidence and next adopter

CI tests capabilities independently as well as through the deckbuilder, with minimal
feature checks and a wasm32 compile check for pure core/UI. Native sockets and UI
remain tested on all three desktop platforms. Same-machine real UDP tests cover
admission, private state, commands, refusal cleanup, and a destroyed/recreated guest
App loading on-disk credentials. They do not emulate two machines' routing policies.

Cross-machine LAN and remote tailnet discovery/join remain explicit manual release
gates described in [multiplayer testing](multiplayer.md). Do not describe an ignored
multicast check, synthetic listing, or credential round trip as that evidence.

The next game is not invented here. Its two-player initial rules, eventual six-player
capacity, private information, and guest restart behavior will live in its own
composition root and pure rule model. Share only the proven geometry capability;
do not introduce world orchestration, occupancy rules, or a generic board-game engine.
