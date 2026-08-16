# Multiplayer development and testing

Gamekit separates three concerns. `bevy_game_multiplayer` owns secure direct
transport, certificate pinning, opaque connection codes, peer/session identities,
and rotating reconnect credentials. `bevy_game_discovery` owns secret-free session
listing and route selection. The adopting game owns admission policy, seats, lobby
rules, commands, authority, snapshots, and UI.

Discovery is optional and never grants admission. A discovered join requires the
host's temporary session passphrase. A privately shared `BGN1` code is an independent
high-entropy bearer credential and bypasses that passphrase. Reconnect uses the
rotating credential stored in the platform application-data directory.

## Ports and providers

- UDP `7777` carries the certificate-pinned WebTransport game session by default.
- UDP `7778` carries development-only tailnet discovery probes and responses.
- mDNS advertises `_bevy-gamekit._udp.local.` only on the local multicast domain.
- Tailscale is installed, authenticated, and routed outside the game. The adapter
  runs the fixed command `tailscale status --json`; it never logs in, requests an API
  token, manages routes, or distributes Tailscale.

MagicDNS names tailnet devices; it does not forward mDNS application advertisements.
Remote discovery therefore uses bounded unicast probes only to online addresses
reported by the local Tailscale client. This approximates a future remote lobby for
development—it does not prove Steam identity, invitations, relay, or NAT traversal.

## Host setup

The host form preselects an active non-loopback LAN address for the private `BGN1`
route. Verify the address shown in the lobby before sharing the code; override it
with the host's Tailscale address when the direct-code recipient is remote over the
tailnet. LAN and Tailscale discovery publish their own provider-specific addresses,
so enabling both does not force one route onto the other. Loopback addresses are
rejected whenever discovery is enabled.

Set an 8–64 character printable temporary passphrase and do not reuse an account or
important password. The host retains only a salted Argon2id verifier; clients clear
plaintext UI buffers after each attempt.

Allow inbound UDP `7777` for gameplay. For tailnet discovery also allow inbound UDP
`7778` on the Tailscale interface. LAN discovery additionally requires multicast DNS
on UDP `5353` in the local network/firewall policy.

## Manual routes

### Same LAN

1. Host with a reachable LAN address, passphrase, and **Discover on LAN** enabled.
2. On a second machine open **Find Sessions**, confirm the `LAN` badge and freshness.
3. Join with the passphrase, ready both seats, start, and play one complete turn.
4. Close the host and confirm the advertisement disappears rather than merely aging
   forever.

### Tailnet development route

1. Confirm both machines show online peers in `tailscale status` and can ping each
   other's tailnet address.
2. Host using the tailnet address with **Discover over Tailscale: On (Dev)**.
3. Browse from the peer, confirm the `TAILNET` badge, join, ready, and play.
4. Restart the guest process and use **Reconnect Reserved Seat**. The host must retain
   the guest's private state and issue a newly rotated credential.

### Private direct fallback

1. Disable both discovery providers and host a session.
2. Share the full `BGN1` code out of band and use **Join with BGN1 Code**.
3. Confirm admission does not request the discovery passphrase.

## Diagnostics and evidence

If tailnet discovery is unavailable, verify the CLI is installed and authenticated,
inspect `tailscale status --json`, confirm both peer addresses are online, and test
UDP `7778` firewall policy. If discovery succeeds but join fails, diagnose UDP `7777`,
the advertised address, certificate expiry/pinning, protocol/build compatibility,
and game-owned admission separately.

A listing proves only that discovery metadata reached the browser. A successful
encrypted connection proves transport. An in-memory link does not prove either.
A same-machine UDP test proves socket setup, BGN1 handoff, admission, and gameplay on
the selected non-loopback address when one is available, but it still cannot prove a
second machine's firewall or multicast path. Ready/start, private hands,
authoritative turns, duplicate-command handling, disconnect, and reconnection
require their own domain or multi-app evidence. Cross-machine LAN and tailnet walks
remain required release evidence for those routes.
