# Labyrinth rules

Current formation, life-state and fixture-content behavior. APIs and invariants live
in the game-owned pure rules package; see [architecture](architecture.md).

## Formation

One actor has one HP pool, initiative entry, Moveset, passive Abilities and controller. Targeting
either occupied rank resolves to the same ID. Source and target reach use intersection
with any occupied rank. Greatsword FrontPair cleave captures the distinct eligible
occupants of target ranks 1–2 before effects. A two-rank actor is hit once; clearing
a corpse does not retarget a replacement. All captured targets resolve before
terminal victory checks. Preview and actual resolution share this target expansion. Reposition swaps adjacent whole occupants;
Exchange swaps whole occupants at any distance. Pushes measure rank distance
and cross only whole neighboring footprints within their distance budget. Pulls
count whole preceding occupants, regardless of either actor's size. Hook Shot's
one pull step puts its surviving target in front of the previous occupant: pulling
a one-rank Stalker past a two-rank Hauler shifts the Stalker forward two ranks and
the Hauler back one. All sizes can be pulled; there is no movement resistance.
Movement uses configured footprints rather than appearance defaults. No wrap.
Clearing remains compacts the formation; death alone does not.

## Skills, Abilities and Moveset

Skills are active actions; Abilities are passive contributions. Each can originate
from a character or equipped item and can require a kind of equipment, an exact
item or no equipment. Personally selected content remains selected while its
requirements are unmet. Such Skills are absent from Moveset, and such Abilities
have no effect. Equipment-only definitions cannot be personally selected through
the editor or authored input. Removing equipment removes its own grants; restoring
appropriate equipment reactivates retained personal selections.

Moveset is the frozen collection of eligible active Skills with applicable passive
upgrades. It excludes passive Abilities and universal actions. Rank, targets and
remaining uses control legal actions without changing the Moveset during combat.
All effects, previews, AI and player commands use the same effective definitions.
Grant-source deduplication never multiplies a passive's contribution. See
[content authoring](content.md) for deterministic ordering and preset source mapping.

Resilient shortens newly applied/refreshed finite negative statuses by one tick of
their own duration clock, minimum one. Bleed lasts two owner-turn starts; Weakened
lasts one owner-turn end. Buffs are unchanged. Default starting conditions use the
same calculation, but explicit starting remaining durations and restored live
snapshots do not shorten again. This is a duration passive, not periodic cleansing.

## Life cycle

Alive -> Dying (heroes only) -> Corpse -> Removed. Enemies skip Dying. Living HP remains
zero on a corpse; corpse HP starts at ceil(max living HP / 4). Ordinary damage skills
can hit opposing living/dying actors and corpses on either team, within Skill reach.
Healing never heals a corpse. Rescue only accepts Dying and resets death-save failures.

The provisional, deliberately small death-save policy rolls a seeded d20 at each dying
hero's initiative slot. A roll below 10 adds a failure; 10+ holds on without healing or
stabilization. Three failures cause permanent death. Damage while dying adds one failure
per nonzero hit. Dying heroes receive initiative slots but never choose actions. An
all-dying party still loses the encounter; there is no automatic recovery after defeat.
This is an original prototype policy, not a complete BG3/DD rules implementation.

Corpses expire at the end of creation round + 3: created in round 2, expires at round 5
end. Surviving status instances retain identity, source, potency and remaining duration.
Their trigger and duration clocks run at round end on corpses, before expiry. Bleed
persists through dying and death; modifier buffs do not persist through death. Poison
is not content yet; the explicit persistence policy supports another condition later.
Corpse destruction/expiry removes remaining statuses and frees all occupied spaces.
Corpses never act and do not prevent victory.

## Adopter and content

Local and multiplayer defaults use Gatekeeper, Knifehand, Scout, Field Medic and the
player-controlled wagon: five unique combatants filling six spaces. The encounter uses
the Hauler plus Ash Brute, Iron Brute, Wound Stalker and Hollow Archer, also once each.
Participant capacity is always six, independent of actor count or footprint.
The host assigns zero/one/multiple heroes to each participant, controls unassigned
heroes and edits enemies. Only assigned controllers must connect and ready;
spectators never gate Start. Repeated presets and smaller rosters are supported.
Weapon Comparison, Cleave and Rescue/Status are additional editable stock scenarios.
The wagon has a 2-damage ranged scrap attack, a limited 3-HP bandage, and universal
actions. Its eventual supply/navigation utility is intentionally not simulated yet.
The Hauler occupies ranks 3–4 and reaches over the two frontline Brutes with a
7-damage strike against hero ranks 1–4. Its attack remains usable if it advances
after corpses clear. Ash Brute and Iron Brute start at ranks 1 and 2 so both can
use their frontline attacks; Stalker and Archer occupy ranks 5 and 6.
Both are balance fixtures, not balanced release content.

Rules/content fingerprints change; all multiplayer participants must run matching builds.
Catalog/scenario schema 2 and combat rules v6 reject incompatible saved formats rather than
silently migrating the changed Skills/Abilities contract. Original files stay intact.

## Decisions

Formation capacity counts spaces, not actors. Footprints preserve one actor identity
and HP pool across occupied ranks. Life-state transitions and corpse clocks belong
to Labyrinth, not Gamekit or a universal shared combat schema. The death-save policy
and fixture content are prototype choices, not a promise of balanced release content.
