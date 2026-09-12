# Labyrinth: footprints and death

Status: implementation milestone, September 2026. These are Labyrinth rules, not
Gamekit contracts. Gamekit continues to own UI/input and networking mechanics.

## Plan and contracts

1. Represent a formation as ordered unique occupants, each with a content-defined
   contiguous footprint. Six is the capacity in spaces, not a required actor count.
2. Validate sizes, life states, unique identities, initiative and status persistence
   on snapshot ingress. Use the same rank-range legality in previews and commits.
3. Resolve death and corpse destruction transactionally; retain original identity for
   ownership, status sources and event references. No resurrection in this milestone.
4. Integrate a two-rank rear Lantern Wagon and a two-rank midline Ossuary Hauler with
   player ownership and replacement art independent of domain rules.
5. Test pure rules, session/reconnect ownership, UI structure, static frames and native
   interaction separately. A screenshot is not multiplayer evidence.

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
