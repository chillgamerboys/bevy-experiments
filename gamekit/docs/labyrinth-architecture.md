# Labyrinth architecture and extension contracts

The game is an adopter of Gamekit, not a new shared engine. Deckbuilder and the
legacy experiments retain their own composition roots and rules.

| Layer | Owns | Explicitly excludes |
|---|---|---|
| `labyrinth_rules` | Actors, formations, content, statuses, deterministic combat/AI | Bevy entities, peers, sockets, files, animation |
| Labyrinth session | Four reservations, hero ownership, ready/rematch, pause, command watermark | Certificates and discovery provider mechanics |
| Labyrinth network | Wire schema, admission policy, target-specific snapshots, capability composition | Combat legality/effect implementation |
| Labyrinth UI | Forms, selection, inspection, effects and accessible native controls | Mutable authoritative state |
| Gamekit capabilities | Identity/security, transport, discovery, UI primitives, test mechanics | Hero classes, four-seat policy, battle or maze rules |

## Deterministic battle kernel

`Combat::new(seed, heroes)` creates the authority. `snapshot()` returns validated
public state without host RNG; `apply(actor, action)` either commits one legal
action and automatic boundaries or returns a typed error without changing state
or consuming RNG. `ai_action()` chooses through the same action/legality surface.
Actor IDs are stable and independent of mutable formation rank. Player slot ownership
stays in the application. Do not put Bevy Entity IDs in these models.

Initiative is rerolled per round using pinned deterministic SplitMix64 sampling.
Effective Speed + d8 sorts descending, then Speed and a seeded tie-breaker. The
round roster is frozen: movement does not reorder it, speed affects the next roll,
and rescue never inserts a bonus turn. Boundary processing is bounded and atomic;
lethal turn-start bleed resolves before any command can act. Enemy resolution is
automatic in the app but not gated on animation completion. An unexpected AI error
halts the encounter with a diagnostic instead of retrying a bad transition forever.

The canonical content fingerprint hashes hero/enemy stats, abilities, status
definitions and an explicit algorithm/interpretation revision. Network compatibility
adds the application wire schema. Change `RULES_VERSION` whenever semantics change;
authored catalog changes automatically change the digest. The fingerprint is a
compatibility check, not proof that a remote executable is trustworthy.

## Statuses are explicit data plus typed behavior

Definitions separate tags, effects, modifier contributions, duration clocks,
reapplication and removal policy. Instances carry identity, source, bearer,
potency, lifetime and boundary activation information. Tags classify effects;
they do not silently dispatch behavior. Character effects move with their bearer.

Initial primitives cover direct/status damage, healing, applying/removing effects,
movement and additive stat modifiers. Bleed and Brace exercise timed harm and
defence; Haste/Weakened definitions exercise future speed/power modifiers in pure
tests. Effects with new semantics require a new tested enum variant; this is not
an unbounded callback or scripting system. Shields, retaliation, auras, arbitrary
stacking and position-bound hazards are not claimed as implemented.

At each boundary, eligible status identities are captured in deterministic order.
New effects cannot recursively trigger at their own application boundary, removed
effects do not fire later, and lethal damage is finalized immediately. Resolution
work and active instances are bounded. Snapshot deserialization rechecks invariants
instead of trusting derived field deserialization. These contracts make later
complexity testable without requiring a universal shared combat/status crate now.

## Admission and replay

`SessionAdmissionAuthority` is the new opt-in pure Gamekit API:

`begin -> encrypted offer -> atomic client persistence -> acknowledge -> admitted`

It has bounded identities/invitations/pending deadlines and returns explicit cleanup.
Games must apply both released identities and disconnected connections, drive expiry
with monotonic time, and authorize gameplay only after ACK. An active connection
cannot be evicted by someone presenting its reconnect credential. Initial invitation
retry and interrupted credential rotation reuse the pending identity/successor;
ACK retires the old credential. Labyrinth reserves three guests; that number is
configuration, not a library seat model.

Labyrinth binds handshake responses and snapshots to a fresh physical-attempt nonce.
Late packets from a previous connection cannot admit its successor or replace its
snapshot. Guest persistence failure refuses admission rather than creating a session
which claims restart support but cannot resume. The existing deckbuilder continues
using the legacy `SessionSecurityAuthority`; this addition does **not** claim to
migrate its handshake or give it Labyrinth's persistence-ACK guarantee.

Every guest request has a per-reservation sequence, encounter and decision boundary.
An ordered bounded result cache supports recent idempotent replies; an independent
live-session high watermark rejects old commands even after cache eviction. Rematch
changes encounter identity without resetting that reservation's watermark. Only a
new player reservation gets a fresh sequence space. Host checks ownership by actor
identity, not formation rank, and applies the same pure reducer used locally.

All cooperative battle state is public, so peers receive full authoritative
snapshots; only request sequence is recipient-specific. Host RNG, admission secrets
and password verifier are never sent in them. Recent typed outcomes have monotonic
session IDs, letting UI effects deduplicate while a reconnect establishes a fresh
baseline. A cosmetic effect is never gameplay evidence or a scheduling dependency.

## Expansion path

Add another encounter/content catalog before inventing a generic battle engine.
Keep a future maze, expedition progression, resources and encounter transitions in
Labyrinth. Extract a capability only when another game establishes a reusable
algorithm/primitive contract; preserve dependency direction from game to capability.
New mechanics must extend legality, transactional resolution, compatibility hashing,
snapshot validation and tests together. New presentation remains a snapshot consumer.
