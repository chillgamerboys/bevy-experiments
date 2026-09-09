# Labyrinth

An original four-player cooperative positional-combat prototype. This first slice
is **one battle**, not maze exploration yet. It has HP, per-round initiative,
rank-constrained abilities, movement, bleed, cleansing, downing and rescue. There
is no stress, PvP, campaign, loot, or host migration.

## Play

From the repository root:

```sh
cargo run -p labyrinth -- --local
```

Local mode controls all four heroes and opens no game transport. The default seed
is 42; use `--seed 91` to try a different reproducible fight. Plain
`cargo run -p labyrinth` opens the multiplayer menu; no `--all-features` is needed.

For four instances on one computer, build once, then run the resulting binary in
four terminals with distinct profiles:

```sh
cargo build -p labyrinth
./target/debug/labyrinth --profile host
./target/debug/labyrinth --profile guest-a
./target/debug/labyrinth --profile guest-b
./target/debug/labyrinth --profile guest-c
```

On Windows use `target\debug\labyrinth.exe`. Profiles isolate reconnect storage
and have an OS-held exclusive lock: accidentally launching two copies of one
profile fails rather than overwriting another guest's identity. `--data-dir PATH`
changes the application-data root; keep the same root/profile when restarting.
No admission secrets are accepted on the command line.

### Host and join

1. Host a company. For same-computer direct testing enter `127.0.0.1`; otherwise
   leave the advertised address empty to choose a local interface, or explicitly
   enter the reachable LAN/tailnet IP. The transport port defaults to UDP 7777.
2. Copy a **different** private BGN1 invitation for each guest. Invitations are
   single-use after acknowledged admission and initially expire after one hour.
   Reissue an invitation from an open lobby when necessary. These codes are bearer
   secrets; do not put them in screenshots, public chat, logs, or bug reports.
3. Guests paste their invitation into Join Direct. Alternatively, the host enables
   LAN discovery (or explicit development tailnet discovery) with an 8–64 character
   printable ASCII temporary passphrase (no leading/trailing spaces), and guests
   select its listing and enter it.
4. All four players ready up; the host starts. Each player controls one hero, not
   whichever hero occupies their original formation rank.

The temporary passphrase is **not an account password**. Do not reuse an important
password. Native Bevy's editable text currently displays typed characters; the form
warns about this and clears its owned buffers after submission. Passwords travel
only over the advertised certificate-pinned encrypted connection. The host keeps
an Argon2id verifier, not the plaintext passphrase. Application diagnostics redact
secret wrappers; this is not a claim that OS clipboard or networking-library buffers
can be scrubbed by the application.

LAN discovery requires a common multicast domain and OS local-network permission;
it does not cross Tailscale. Tailnet discovery uses an externally installed and
authenticated Tailscale CLI and UDP 7778, and is never enabled implicitly. Allow
UDP 7777 for gameplay and UDP 7778 for tailnet probes in host/peer firewall policy.
See [network diagnostics](../../docs/multiplayer.md) for route and permission checks.

### Controls and combat

Pointer activation and Tab / Shift+Tab, Enter / Space work on native UI controls.
Select an ability, inspect its source/target-rank requirements, select a target,
then Confirm. Unavailable abilities remain inspectable; confirmation explains why
the selected action cannot currently be submitted. Escape dismisses local overlays.

The initial party is Gatekeeper, Knifehand, Scout, and Field Medic. These are
original placeholder archetypes. Starting formation follows archetype rank even
when player slots choose different heroes. Both teams roll effective Speed + d8
each round; the current round's order stays fixed when actors move or speed changes.

Bleed deals 2 damage at the affected actor's next three turn starts. Reapplying it
refreshes duration without stacking damage. Brace reduces direct damage by 2 until
the owner's next turn starts, but does not reduce bleed. Rescue revives a downed
ally at 25% maximum HP, rounded up. Hero bodies retain their formation slots; dead
enemies are removed and remaining enemies compact forward. All heroes down loses;
all enemies dead wins. The host can return the party to the lobby for another fight.

### Disconnect and restart

If a guest process disappears, the host retains that hero and pauses combat once
transport loss is observed. Restart with the same profile and choose Reconnect.
The client persists an offered rotating credential **before** acknowledging it;
the host does not authorize commands before that acknowledgement. Private invitation
retries and persisted-credential retries recover the same pending hero after lost
offers/acknowledgements. If an initial **password** admission loses its offer before
the guest can persist a credential, its anonymous pending reservation expires after
15 seconds; wait for expiry before retrying the password to avoid a temporary extra
reservation. Established reconnection does not use the password.

Leaving the lobby explicitly releases the guest's reservation. Leaving during
combat keeps it reserved, like an interrupted connection. There is no bot takeover
or host-side kick in this slice. Return to the lobby and have a connected guest
leave to release its seat, or start a new company. Host process restart deliberately
ends the session; stored guest credentials cannot recover a lost host world.

## Engineering boundaries

`rules/` is a pure Rust package: no Bevy, network, window, or filesystem dependency.
It owns rules and content. `src/session.rs` owns players, readiness, replay policy,
pause, and encounter lifecycle. `src/network/` composes opt-in Gamekit capabilities.
`src/ui/` projects snapshots and sends typed intents; it never mutates authority.

See [architecture and extension contracts](labyrinth-architecture.md)
and [verification](labyrinth-testing.md). The future endless maze should
be another game-owned model/orchestrator, not a reason to put expedition rules in
Gamekit or to replace this combat kernel.
