# Labyrinth architecture and extension contracts

The game is an adopter of Gamekit, not a new shared engine. Deckbuilder and the
Carterfight retain their own composition roots and rules.

| Layer | Owns | Explicitly excludes |
|---|---|---|
| `labyrinth-rules` | Actors, formations, content, statuses, deterministic combat/AI | Bevy entities, peers, sockets, files, animation |
| Labyrinth session | Up to six reservations within six formation spaces, explicit actor ownership/loadouts, ready/rematch, pause, command watermark | Certificates and discovery provider mechanics |
| Labyrinth network | Wire schema, admission policy, target-specific snapshots, capability composition | Combat legality/effect implementation |
| Labyrinth UI | Forms, selection, inspection, status summaries and accessible native controls | Mutable authoritative state, sprite artwork |
| Labyrinth scene | ActorId-keyed sprites, art catalog, camera-aware anchors, environment | Input dispatch, legality, combat timing, networking |
| Gamekit capabilities | Identity/security, transport, discovery, UI primitives, test mechanics | Hero classes, six-seat policy, battle or maze rules |

## Deterministic battle kernel

`Combat::with_party(seed, Vec<HeroSetup>)` builds the prototype Hauler encounter;
`Combat::with_rosters` accepts explicitly composed formations on both teams.
`Combat::with_heroes(seed, [HeroSetup; PARTY_SIZE])` retains the six-single-rank
fixture API. All constructors create the authority from
explicit unique actor IDs, class presets, and ability loadouts in front-to-rear
rank order. `Combat::new(seed, [HeroClass; PARTY_SIZE])` is a convenience using
default hero IDs and class starter abilities. Repeated classes are legal; class,
actor identity, owner, and rank are independent. `snapshot()` returns validated
public state without host RNG; `apply(actor, action)` either commits one legal
action and automatic boundaries or returns a typed error without changing state
or consuming RNG. `ai_action()` chooses through the same action/legality surface.
Actor IDs are stable and independent of mutable formation rank. Player slot ownership
stays in the application. Do not put Bevy Entity IDs in these models.

`PARTY_SIZE` fixes this game's current six-rank contract; `HeroClass::ALL` enumerates
five content presets, not seats. The bounded actors, six-rank masks, formation rules,
and both default rosters participate in compatibility validation/fingerprinting.

Formations contain unique occupant IDs, never repeated cells. `ranks(id)` returns
the complete footprint; `occupant(team, rank)` maps either covered space to one ID.
Life states distinguish living HP, dying heroes, independent corpse HP, and removed
remains. See the [formation and death decision](decisions/footprints-and-death.md)
for precise clocks, movement, targeting and provisional death-save semantics.

### Ability composition seam

`AbilityLoadout` is an ordered, duplicate-free, validated list of up to eight
catalog `SkillId`s on each actor. Empty loadouts still have universal actions.
Actions, legality, per-instance use limits, AI and UI consume this actual loadout;
class only supplies base stats/visuals and a starter preset. Same-class actors
never share uses, statuses, or ownership. Guest snapshots preserve equipped
abilities through reconnect rather than reconstructing them from class.

Future equipment and skill-tree resolvers belong to Labyrinth: resolve their
grants into this loadout at an explicit authoritative preparation boundary, then
pass it through validated setup. Resolve duplicate grants/order/limits there;
do not infer them from presentation. There is deliberately no inventory schema,
tree model, grant provenance, mid-combat equip command, or shared Gamekit combat
framework yet. Extend those rules and compatibility together when designed.

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
ACK retires the old credential. Labyrinth reserves five guests; that number is
configuration, not a library seat model.

Labyrinth binds handshake responses and snapshots to a fresh physical-attempt nonce.
Late packets from a previous connection cannot admit its successor or replace its
snapshot. Guest persistence failure refuses admission rather than creating a session
which claims restart support but cannot resume. Deckbuilder also uses the shared
acknowledged admission authority, but retains its own two-seat policy, handshake
composition, and game-specific snapshots; Labyrinth's six-seat rules do not leak
into that adopter.

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

## Stage and overlay presentation

The battle is a native 2D scene beneath a transparent Bevy UI. Twelve art-only
buttons supply the same stable focus/activation contract as other Gamekit controls;
`SceneActorAnchor` projects each laid-out rectangle through its actual target camera.
Sprites are keyed by ActorId, never by class or rank. The renderer does not pick
characters separately, and replacing an image cannot change a hit region or permission.
UI post-layout precedes scene synchronization, which precedes visibility/bounds.
The camera conversion accounts for viewport offsets, device scale, pan and zoom.

Organization remains game-owned: one facing formation, compact rolled-order strip,
equipped-ability rail, target/legality line and explicit confirmation. Actor,
condition and current-round initiative details use the shared contextual-card
system, with encounter/actor-scoped subjects and disclosure-filtered content.
Portrait activation pins a card without changing the selected target; there is no
separate inspection/initiative drawer. The primary battlefield never scrolls;
overflowing ability loadouts scroll horizontally and focus brings controls into view.

Local game/settings/leave pages compose Gamekit's `UiMenuStack` and menu templates.
They never gate network schedules, pause Bevy time, or change host authority.
`CombatInterruption` distinguishes missing players, local reconnect admission and
halted rules. The validated host snapshot excludes local reconnect/menu state and
checks its compatibility `paused` flag against the authoritative reason. One player
returning does not resume combat while another is missing, and reconnection does
not clear a rules fault.

Log mode is a local Hidden/Compact/History enum, initially Hidden. Hidden removes
the entire input surface but retains authoritative events. Compact shows two
outcome summaries without history navigation; History provides the non-modal
scrollable overlay. It groups typed authoritative events by
action and turn boundaries, handles a partially retained first action explicitly,
and reuses unchanged rows by event identity. New encounters reset local expansion
and reading state. Gamekit only supplies follow-latest scrolling; game-local code
owns summaries, detail links, bounded retention and disclosure. As elsewhere,
concealing a history panel does not remove facts already sent over the network.

Statuses use one stable effects control per actor, with priority derived from the
effect definition. A compact name/potency/clock and overflow count are backed by
complete accessible descriptions and a scrollable inspector containing every effect.
The compact H/E tokens identify actors, not rank. Full names, ownership, HP maximum,
rank and modifiers remain available through inspection.

Appearance is intentionally separate from organization. `SceneAppearance` owns
optional image handles, colors, atlas insets and the environment floor anchor;
`UiTheme`, `UiFonts`, and semantic controls own HUD styling. Images and the licensed
font are embedded so crate-directory and workspace-directory launches behave alike.
Primitive scene fallbacks tolerate missing assets, including headless tests.
The original prototype art and generation prompts live under `src/scene/assets`;
the font's OFL license is under `src/ui/assets`.

Later pixel-art sampling, sprite animation or custom materials belong at the render
leaf, not in the rules, wire schema or control actions. No shader pipeline, theme
editor, rearrangeable dashboard or shared RPG scene manager is introduced here.
Gamekit's existing unskinned controls, focus scopes, metrics and scrolling proved
sufficient; game-specific scene/HUD composition stays in Labyrinth.

## Expansion path

Add another encounter/content catalog before inventing a generic battle engine.
Keep a future maze, expedition progression, resources and encounter transitions in
Labyrinth. Extract a capability only when another game establishes a reusable
algorithm/primitive contract; preserve dependency direction from game to capability.
New mechanics must extend legality, transactional resolution, compatibility hashing,
snapshot validation and tests together. New presentation remains a snapshot consumer.
