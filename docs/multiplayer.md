# Multiplayer operations and diagnostics

Discovery is an untrusted availability hint, not admission or proof that a gameplay
socket is reachable. The composition root resolves a provider-neutral route to a
transport adapter. The game owns seats, capacity, readiness, commands and disclosure.
See [architecture](architecture.md) and the [Labyrinth](../games/labyrinth/README.md)
or [deckbuilder](../games/deckbuilder_ui/README.md) instructions for their distinct UX.

## Routes and infrastructure

| Route | Requirements | Scope |
|---|---|---|
| Private BGN1 invitation | Reachable host address; pinned encrypted connection | Direct fallback; no discovery required |
| LAN discovery | Local multicast, UDP 5353, OS permissions | Same multicast domain only |
| Tailnet discovery | External authenticated Tailscale, UDP 7778 | Explicit development opt-in only |
| Gameplay | Inbound host UDP 7777 by default | Independent of discovery success |

BGN1 is a bearer secret, not an account or a game-protocol compatibility guarantee.
Do not include complete codes, passwords, or reconnect credentials in logs or reports.
For same-computer private tests use loopback deliberately; never advertise loopback
to remote guests. Games reject it for discoverable hosting. Verify the selected LAN
or tailnet address before sharing invitations. Each provider retains its own address
when metadata/occupancy changes; choosing a tailnet private route must not replace LAN.

mDNS advertises `_bevy-gamekit._udp.local.`. Bonjour on macOS, native DNS-SD on Windows
and Avahi/D-Bus on Linux own record lifetime and goodbye removal. Browsers renew their
registry lease while the OS has the record; abrupt host loss may remain visible until
DNS expiry. A long-lived unchanged advertisement must not disappear as if DNS emitted
heartbeats. Local-network permission/firewalls and multicast isolation still apply.

Tailscale remains external routing infrastructure. MagicDNS names devices; it does
not forward application mDNS advertisements. The optional adapter reads fixed-argument
`tailscale status --json`, with a deadline/output cap, and sends bounded unicast probes
only to reported reachable peers. It never installs Tailscale, logs in, manages routes
or requests API credentials. This does not prove Steam identity, lobbies, invitations,
relay or NAT traversal. Keep optional I/O off game schedules with bounded queues.

## Admission, persistence and failures

Discoverable hosts require an 8–64 character printable temporary passphrase. Do not
reuse an account password. The host stores a salted Argon2id verifier; clients send
the passphrase only over the certificate-pinned encrypted connection. Public listing
metadata carries no admission secrets. Private invitations and established reconnect
credentials are independent admission routes and bypass the discovery passphrase.

Password verification permits five failures per source IP and 30 globally in an
inclusive 60-second rolling window. The fifth source failure also starts a
30-second cooldown. Both conditions must clear before another hash is attempted,
even for a correct password. Blocked attempts do not extend the limits; successful
verification clears source history but leaves global failure history intact.

Admission is an offer -> atomic client persistence -> acknowledgement -> authorization
flow. Transport connection alone must not permit game commands. Storage failure cancels
the attempt; it is not a successful join with a warning. Credential rotation retains
the recovery window needed for lost offers/ACKs. Games still own reservation policy,
expiry, attempt identity and exactly which snapshots each recipient may see.

Labyrinth and Deckbuilder opt into `GameInboundBudgetPlugin` and attach
`InboundLimits` to their hosted listener. The gate sits after transport reassembly
and before Replicon decoding. Pending connections allow eight messages / 4 KiB per
frame; admitted connections allow 144 messages / 64 KiB. Either phase rejects an
individual envelope larger than 1 KiB. Current client envelopes contain bounded
credentials, protocol identifiers and fixed game commands. Revisit these limits
with tests when adding larger client payloads. Upstream transport memory/IO remains
subject to its own limits; this gate bounds application forwarding and decoding.

Labyrinth retains up to 128 pending requests per admitted physical connection
(640 across five guest slots), dispatching FIFO within each connection and rotating
between connections. Per-frame ceilings are 16 per guest and 128 total. Pending or
unauthorized traffic cannot enter those gameplay queues. Overflow disconnects the
sender and discards its unprocessed queue without clearing its reservation or
sequence watermark; reconnect obtains the host's current sequence and decision.
An already applied command cannot execute again. An admitted player's disconnect
still pauses a live encounter under the existing game policy.

Native text editing currently shows typed characters in these prototypes. Owned secret
wrappers, submitted messages and credential serialization buffers redact diagnostics
and zeroize on drop/replacement. This does not promise complete erasure of OS clipboard,
IME, renderer, allocator or networking-library copies. Copying an invite is explicit.
Do not remove game profiles during repository cleanup or silently delete incompatible
credentials. Host restart ends the session; persistent host recovery is out of scope.

## Diagnose the failed stage

1. No listing: check host opt-in, matching network/multicast domain, provider error,
   OS local-network permission, Bonjour/Avahi and UDP 5353. Tailnet browsing needs its
   own opt-in, installed/authenticated CLI and UDP 7778; it is not mDNS-over-Tailscale.
2. Listing but no connection: inspect the advertised route and UDP 7777, certificate
   pin/expiry, route reachability and provider handoff. A listing is not a socket test.
3. Connection but no admission: check game/protocol compatibility, capacity/reservation,
   expired/used invitation, generic password rejection/cooldown, or credential storage.
4. Admission but wrong gameplay: inspect host-derived identity, game-owned readiness,
   command sequence watermark and recipient-specific snapshots. Do not blame discovery.
5. Restart fails: use the same application-data root/profile against the still-running
   host. Verify that a fresh process actually loaded and acknowledged its credential.

Inspect routing with tester-run Tailscale diagnostics and firewall policy; redact
addresses if necessary and always redact secrets. Do not repeatedly toggle unrelated
providers or log plaintext authentication while diagnosing a different stage.

## Required real-machine routes

- LAN: sustain an unchanged listing for at least a minute; test wrong/right passphrase,
  occupancy refresh, gameplay and host closure/removal.
- Tailnet: confirm both peers reachable, discover and join explicitly, play, terminate
  the guest process, then recover its reserved state against the live host.
- Direct: disable both providers, share a private code, join and play without asking
  for the discovery password.

Record revision, machines, route, duration and result for each stage. Fake providers,
in-memory links and same-machine sockets cannot replace these checks. Current automated
coverage and interactive acceptance live in each game; see [evidence rules](testing.md).
