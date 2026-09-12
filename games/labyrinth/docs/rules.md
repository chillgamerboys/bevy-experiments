# Labyrinth rules

Current formation, life-state and fixture-content behavior. APIs and invariants live
in the game-owned pure rules package; see [architecture](architecture.md).

## Formation

One actor has one HP pool, initiative entry, equipped loadout and controller. Targeting
either occupied rank resolves to the same ID. Source and target reach use intersection
with any occupied rank. No area attack is introduced yet; future target expansion must
deduplicate IDs before resolving effects. Reposition swaps adjacent whole occupants;
Exchange swaps whole occupants at any distance. Forced movement measures rank distance
and crosses only whole neighboring footprints within its distance budget. No wrap.
Clearing remains compacts the formation; death alone does not.

## Life cycle

Alive -> Dying (heroes only) -> Corpse -> Removed. Enemies skip Dying. Living HP remains
zero on a corpse; corpse HP starts at ceil(max living HP / 4). Ordinary damage skills
can hit opposing living/dying actors and corpses on either team, within ability reach.
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
Replacing the wagon with a single-rank class opens a sixth player slot. Selecting a
wagon consumes an unoccupied slot, never an admitted player. Repeated classes remain
supported as a party choice and explicit six-player test fixture, not the default.
Every remaining member must connect and ready.
The wagon has a 2-damage ranged scrap attack, a limited 3-HP bandage, and universal
actions. Its eventual supply/navigation utility is intentionally not simulated yet.
The Hauler occupies ranks 3–4 and reaches over the two frontline Brutes with a
7-damage strike against hero ranks 1–4. Its attack remains usable if it advances
after corpses clear. Ash Brute and Iron Brute start at ranks 1 and 2 so both can
use their frontline attacks; Stalker and Archer occupy ranks 5 and 6.
Both are balance fixtures, not balanced release content.

Rules/content fingerprints change; all multiplayer participants must run matching builds.
Existing saves/wire snapshots are not silently migrated to the new life-state schema.

## Decisions

Formation capacity counts spaces, not actors. Footprints preserve one actor identity
and HP pool across occupied ranks. Life-state transitions and corpse clocks belong
to Labyrinth, not Gamekit or a universal shared combat schema. The death-save policy
and fixture content are prototype choices, not a promise of balanced release content.
