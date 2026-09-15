# Labyrinth content and build authoring

The pure rules crate owns validated content, build composition, saved battle
configuration and execution of frozen resolved Skills. Local/co-op setup and
future simulations use the same scenario validation and combat reducer. UI and
network integration are separate consumer acceptance; these rules tests do not
establish either route's usability.

## Authoring boundary

`rules/content/catalog.toml` is the embedded schema-2 catalog returned by
`ContentCatalog::builtin()`. Skills are active actions; Abilities are passive
contributions. The catalog keeps the 22 original active definitions and adds the
six generic weapon choices, five exact starter item variants, Assassin Feint,
two dagger-upgrade Abilities and Resilient. A complete alternate catalog can be
parsed with `ContentCatalog::from_toml(&str)` without filesystem access.

Stable lowercase IDs identify content independently of display text. Skills own
six-bit source/target rank masks, typed target eligibility, optional encounter uses
and ordered effects. Weapons have a stable `kind`, ordered `skills` and passive
`abilities` grants. Handedness remains descriptive: there is one optional weapon
slot, no inventory, ammunition or equipping command during combat. Dagger Throw
has unlimited uses.

For example, append these definitions to a complete schema-2 catalog:

```toml
[[skills]]
id = "training_lance_thrust"
name = "Training Lance Thrust"
description = "Reach one enemy in the front or middle ranks."
personal_selectable = false
source_ranks = 15
target_ranks = 15
target_rule = "EnemyStanding"
effects = [{ Damage = 6 }]

[[weapons]]
id = "training_lance"
kind = "spear"
name = "Training Lance"
description = "Grants a reach thrust."
handedness = "Two"
skills = ["training_lance_thrust"]
```

Both Skills and Abilities can declare equipment requirements independently of
where they are granted: `requirements = [{ Kind = "dagger" }]` accepts any dagger,
while `requirements = [{ Item = "medic_staff" }]` requires that exact item. At most
one kind and one exact item may be required, and both must match if present.
Unknown, duplicate or conflicting requirements fail catalog validation. No
requirements means equipment-independent. `personal_selectable = false` means
an equipment grant cannot be copied into personal selections, even by a manually
authored scenario. The default is personally selectable for newly authored content.

Adding known Skill effects or passive upgrades is data-only. New effect semantics
require a typed Rust primitive and rules compatibility revision. `StatusDamage` is
not an active Skill primitive because its magnitude needs a status instance.
There is no passive event scripting, recursive grants, reaction engine or skill tree.

`TargetPattern::Single` is the default. `FrontPair` captures distinct eligible
opponents/corpses occupying enemy ranks 1–2 before effects. Multi-rank actors are hit
once; corpse removal never adds a replacement target. Empty ranks contribute no
target. All captured targets resolve in front-to-back order before terminal checks.
Preview calls the same immediate resolver without advancing turns, spending uses,
consuming random state or mutating authority.

## Build resolution and ownership

`CharacterBuild` contains ordered personal `SkillGrant { skill, provenance }`
selections, personal Ability IDs and one optional weapon. `resolve_build` returns
an owned `ResolvedBuild` containing:

- `moveset: Moveset`, the canonical collection of eligible `ResolvedSkill` values;
- `abilities`, resolved passives with grant sources and any inactive reason;
- `inactive_skills`, retained selections with unmet-equipment explanations;
- the catalog fingerprint used to derive the entire view.

Resolution collects personal Skills followed by equipment Skills, preserving that
active ordering and collapsing repeated IDs while retaining grant paths. It gathers
personal/equipment Abilities and orders them by stable ID. Requirements determine
eligibility. An Ability whose required Skill is absent from the eligible Moveset is
inactive as a whole; none of that Ability's upgrades or runtime effects apply.
Applicable upgrades then run once per distinct Ability, regardless of grant count.
Repeated personal Ability IDs and identical personal Skill/provenance pairs fail.

Equipment removal drops only that item's grants. Personal selections remain in the
build while inactive and reactivate when suitable equipment returns. A different
source can keep the same definition granted. Missing catalog definitions and invalid
combined upgrades are errors; unmet equipment requirements are an ordinary valid
build state. Empty Movesets are valid because universal actions remain separate.
Rank, available targets and remaining uses determine action legality this turn;
they do not remove an equipment-eligible Skill from the frozen Moveset.

Supported passive upgrades add damage to a base Damage effect, append an effect,
or extend source/target rank masks. They cannot replace definitions, change use
limits, address another upgrade's appended effects or grant new Skills. Assassin
Feint is a personally selectable Skill requiring a dagger. Bleeding Dagger and
Duelist Dagger are passive upgrades requiring both a dagger and eligible Dagger Stab.
With Knifehand Daggers equipped, those upgrades remain inactive because that item
grants different attacks; the reason names the missing target Skill.

Resilient is the first narrow runtime passive. On each new or refreshed finite
`Debuff`, it removes one tick from that status's own duration clock, with a minimum
of one tick. Bleed changes from three owner-turn starts to two; Weakened changes
from two owner-turn ends to one. Buffs are unaffected. The same Ability granted
personally and by equipment applies once. Distinct duration-reducing Abilities
combine additively with the same minimum. Default starting statuses use this rule;
explicit starting `remaining` and restored live snapshots already describe remaining
clocks and are never shortened again. Dying/death clock changes retain the instance's
remaining duration. Resilient does not cleanse statuses every turn.

Actor HP, live statuses and use counters belong to runtime actors. Catalog
resolution and inspection never reset them. `ActorPreset` copies editable defaults;
appearance, name, stats, footprint and selections vary independently. Formation,
allegiance and controller ownership remain scenario/session responsibilities.

### Starter source migration

All ten presets retain exactly their existing active Skill IDs, effects and use
limits. Five explicit starter item variants preserve armed attacks without adding
the generic comparison weapons' different attacks:

| Preset | Equipment grants | Personal Skills retained |
|---|---|---|
| Gatekeeper | Gatekeeper Spear: Front Strike, Long Reach, Driving Blow | Field Dressing |
| Knifehand | Knifehand Daggers: Bleeding Cut, Deep Strike, Thrown Knife | Clean Blade |
| Scout | Scout Bow: Back-rank Shot, Snap Shot, Hook Shot | Exchange |
| Field Medic | Medic Staff: Staff Strike | Mend, Staunch, Rally |
| Hollow Archer | Hollow Bow: Hollow Bolt | None |

Lantern Wagon, Ash Brute, Iron Brute, Wound Stalker and Ossuary Hauler retain their
natural/support Skills personally and require no equipment. These are editable
creation defaults, not permanent classes or a second grant-authorization path.

## Validation and replay identity

Catalog and scenario schema 2, build resolver v2 and combat rules v6 replace the
old mixed active-Ability/learned format. Old formats are rejected with a request to
recreate them; no implicit conversion or overwrite occurs. Network adopters must
also change their wire protocol and refuse stale peers. This pure rules migration
does not establish that app/network migration has been delivered.


`ContentCatalog::new` and catalog deserialization reject duplicate definitions,
missing references, invalid presets and incompatible upgrades. TOML unknown fields
and unsupported enum variants fail parsing. Errors include the definition/field
path; parse errors preserve the TOML diagnostic. Resolved views deserialize as
untrusted DTOs: call `catalog.validate_resolved(build, resolved)` before accepting
one. This recomputes the build and compares the entire view, including provenance.

Catalog input is limited to 1 MiB, 256 definitions per category, 64 distinct granted Skills and 64 distinct Abilities
per actor and 16 effects per effective move. HP and direct damage/healing are
bounded to 1–10000, speed to 0–100 and footprint to 1–6. These are validation/resource
bounds, independent of eight numeric shortcuts. Overflow returns an error; no move
is silently dropped. Transport must separately bound raw payloads before parsing
and account for all actor/provenance data in its envelope limits.

Catalog category order is canonicalized by stable ID. Ordered grants/effects remain
ordered data. The SHA-256 fingerprint includes all catalog data, the build resolver
revision and the existing combat rules/content identity, including status defaults.
Changing authored values changes the fingerprint. Freeze the validated catalog and
resolved builds at encounter start; a content reload belongs to the next encounter.

## Scenarios and frozen runtime

`Scenario` contains schema version, name, seed and separate ordered `heroes` and
`enemies` vectors. Both use `ScenarioActor { id, actor: ActorBuild, controller,
starting_hp, starting_statuses }`. Vector order is formation order. A team needs
1–6 actors occupying at most six ranks; IDs must be nonzero and unique across both
teams. Appearance does not determine allegiance, footprint, HP, speed or moves.

`starting_hp` defaults to maximum. Zero starts a hero Dying; enemy zero HP and an
entirely downed starting party are rejected. `StartingStatus` names a typed kind,
optional source actor (default self), and optional remaining boundaries (default
full authored duration). Source identities must exist; durations are positive and
cannot exceed the condition's initial duration. Potency retains existing typed
status defaults. Initial statuses enter the ordinary turn-boundary machinery;
starting with bleed on the first actor can immediately trigger its first tick.

`ControllerPolicy::Manual` names human session control, `Ai` enables the default
policy, and `External` leaves decisions to an adapter such as the future gym. No
participant IDs, ownership, reconnect secrets or credentials belong in saved game
configuration. The session still authorizes the actual player. `legal_actions`,
`validate_action` and `apply` are common to all policies. `ai_action()` acts only
when the current actor's configured policy is Ai, on either team.

`Scenario::stock(choice, seed, catalog)` supplies Prototype, WeaponComparison,
Cleave and RescueStatus options. Each returns the same editable validated model.
`Scenario::from_json` and `to_json` handle bounded portable save data without I/O.
`validate` is non-mutating; applications keep the old draft when it fails. The
scenario fingerprint includes catalog/rules identity and all initial configuration,
including seed and policy. Equal frozen inputs and ordered commands replay equally.

`Combat::from_scenario(catalog, scenario)` freezes the catalog and each actor's
resolved build. Actor snapshots retain build selections and full resolved views;
snapshot validation recomputes and compares the views against that frozen catalog.
Custom display names, explicit allegiance and footprint remain actor properties.
Runtime HP, statuses and use counters stay separate and actor-owned. Definition
reloads affect later encounters, not the running one.

`CombatAction::Skill { index, target }` uses a bounded actor-local index into the
frozen resolved list. The surrounding session command must bind actor, encounter
and current decision/assignment context; an index is not globally meaningful.
`actor.skill(index)`, `actor.moveset()` and `resolved_skills()` expose effective definitions and
provenance for every consumer. Legal action enumeration, AI, preview and execution
read these owned definitions rather than a global hard-coded weapon map.

`CombatAction::LegacySkill` is an explicit fixed-enum compatibility adapter. It
translates to the same actor-local index and use counter; `legal_actions` and AI
emit indexed Skill actions. `HeroSetup`/`LegacySkillLoadout` remain trusted fixture
adapters: `legacy_catalog` authors bounded exact equipment, and `legacy_build`
selects that item. Parsed scenarios never invoke those helpers or manufacture new
equipment. The runtime's `legacy_skills()` only recognizes old enum IDs; new
consumers must inspect the complete Moveset. `legacy_skill_definition` and
`LegacySkillDefinition` are named compatibility content, not current catalog APIs.

Catalogs are bounded to 1 MiB and saved scenarios to 128 KiB. Scenario validation
estimates the complete frozen configuration plus 128 KiB reserved for bounded
runtime fields and rejects configurations exceeding the **1 MiB combat snapshot**
budget before Ready/Start. Snapshot validation also bounds actual JSON bytes.
Session/network envelopes must allow their additional catalog, lobby and event
fields above this combat budget and reject oversized input before deserialization.
There is no truncation of abilities or provenance to fit a packet.

## Verification

`cargo test --locked -p labyrinth-rules --profile ci` covers catalog/build rejection,
exact preset Skill sets, source restrictions, equipment removal/reactivation,
kind/exact-item prerequisites, passive deduplication, twelve-move serialization, custom stats, all six
weapon moves through real preview/apply, unlimited throw, passive upgrades and Resilient application/refresh/restoration and
legacy aliases sharing resources, twelve arbitrary authored moves executed by
index, AI/external policy routing, stock JSON reload/replay at seeds 42 and 91,
cleave across two actors or one large actor, corpse compaction without retargeting,
terminal target completion, malformed commands and transactional work exhaustion.

A bounded fixture measures stock snapshot sizes plus twelve actors each holding
64 distinct moves under `target/review/runtime-wire-sizes.json`. These are
actual fixture sizes, not a claim that every syntactically valid catalog/build
combination fits the envelope; preparation explicitly rejects oversized results.
The frozen rules/content compatibility revision changes with the runtime migration.
Pure tests do not establish native UI reachability, networking, interactive clarity
or balance. Those claims need the integrating application's owner checks.
