# Labyrinth content and build authoring

The pure rules crate owns validated content, build composition, saved battle
configuration and execution of frozen resolved abilities. Local/co-op setup and
future simulations use the same scenario validation and combat reducer. UI and
network integration are separate consumer acceptance; these rules tests do not
establish either route's usability.

## Authoring boundary

`rules/content/catalog.toml` is the embedded catalog returned by
`ContentCatalog::builtin()`. It contains all 22 legacy active abilities and ten
actor defaults, six starter weapons, one learned active addition and two learned
upgrades. `ContentCatalog::from_toml(&str)` accepts another complete catalog with
no filesystem access. The application owns reading, saving and transport.

Definitions use stable lowercase IDs, owned display text, six-bit source/target
rank masks, typed target eligibility, optional encounter uses and ordered effects.
One equipped weapon grants its ordered moves. Handedness is descriptive metadata;
there is no inventory container, offhand slot, ammunition, retrieval or equipping
command during battle. Dagger Throw has unlimited uses.

For example, adding another known-effect move and weapon requires data only:

```toml
[[abilities]]
id = "training_lance_thrust"
name = "Training Lance Thrust"
description = "Reach one enemy in the front or middle ranks."
source_ranks = 15
target_ranks = 15
target_rule = "EnemyStanding"
effects = [{ Damage = 6 }]

[[weapons]]
id = "training_lance"
name = "Training Lance"
description = "Grants a reach thrust."
handedness = "Two"
grants = ["training_lance_thrust"]
```

These tables append to a complete schema-1 catalog. Changing damage or reach edits
only those fields. New effect semantics still require a tested Rust primitive and
rules compatibility revision. `StatusDamage` is rejected in active abilities
because its magnitude requires a status instance. Passive/reaction behavior and
skill-to-skill links are unsupported, so a cyclic skill graph cannot be authored.

`TargetPattern::Single` is the default. `FrontPair` records the greatsword's target
collection rule: distinct eligible opponents/corpses occupying enemy ranks 1–2.
The reducer captures those identities before effects and hits a multi-rank actor
once. Clearing a corpse never adds its replacement occupant to the action. Empty
ranks contribute no target. Allied corpses remain targetable by Single attacks,
but a FrontPair attack only captures the opposing formation. Targets resolve in
front-to-back order, each receiving effects in authored order. Terminal evaluation
waits until the entire captured set completes. Preview calls this exact immediate
resolver without advancing turns, consuming random state or mutating authority.

## Build resolution and ownership

`CharacterBuild` selects ordered `InnateGrant { ability, provenance }` values,
learned skill IDs and one optional weapon. `ContentCatalog::resolve_build`:

1. Collects innate grants, then the weapon's grants, then learned grants sorted by
   stable learned-skill ID. Definition file order cannot reorder the catalog.
2. Collapses repeated ability IDs while retaining each distinct grant source.
3. Applies learned upgrades in stable skill-ID order after all grants exist.
   An upgrade whose move is absent fails as a missing prerequisite.
4. Returns an owned `ResolvedBuild` with effective abilities, original grant paths,
   exact upgrade contributions and the catalog fingerprint.

Learned skills may add a move, upgrade a move, or both. `provenance` such as
`assassin` or `duelist` is descriptive identity, independent of active behavior.
Supported upgrades add positive damage to a specified base Damage effect, append
an effect, or extend source/target rank masks. They cannot replace definitions,
change use limits, target an appended effect or create recursive grants. Invalid
indexes, target-shape conflicts and combined power/effect overflow fail explicitly.
Repeated learned selections and identical innate source/ability pairs are errors.

Removing a grant source removes only its contribution on re-resolution. If another
source still grants the move, it remains; if an upgrade loses its prerequisite,
the build fails for the editor to explain. Empty builds remain valid for universal
actions. Actor HP, statuses and remaining uses belong to runtime actor state, not
catalog definitions or resolved views. Re-resolving inspection must never reset
those resources.

`ActorPreset` supplies name, appearance, HP, speed, footprint and a default build.
`ActorBuild::from_preset` copies those values for editing. `ActorBuild::resolve`
validates explicit values without comparing them to archetype defaults. Appearance,
stats, occupied ranks and the build can therefore vary independently. Team,
formation placement and controller ownership belong to scenario/session setup.

## Validation and replay identity

`ContentCatalog::new` and catalog deserialization reject duplicate definitions,
missing references, invalid presets and incompatible upgrades. TOML unknown fields
and unsupported enum variants fail parsing. Errors include the definition/field
path; parse errors preserve the TOML diagnostic. Resolved views deserialize as
untrusted DTOs: call `catalog.validate_resolved(build, resolved)` before accepting
one. This recomputes the build and compares the entire view, including provenance.

Catalog input is limited to 1 MiB, 256 definitions per category, 64 distinct moves
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
resolved build. Actor snapshots retain build selections and full effective views;
snapshot validation recomputes and compares the views against that frozen catalog.
Custom display names, explicit allegiance and footprint remain actor properties.
Runtime HP, statuses and use counters stay separate and actor-owned. Definition
reloads affect later encounters, not the running one.

`CombatAction::Ability { index, target }` uses a bounded actor-local index into the
frozen resolved list. The surrounding session command must bind actor, encounter
and current decision/assignment context; an index is not globally meaningful.
`actor.ability(index)` and `resolved_abilities()` expose effective definitions and
provenance for every consumer. Legal action enumeration, AI, preview and execution
read these owned definitions rather than a global hard-coded weapon map.

Legacy `Skill` actions translate the old enum to the matching authored ID and then
the same effective index. Upgrades and remaining uses therefore apply identically
to both command spellings. `skill_uses` is keyed by this actor-local index. Legacy
`HeroSetup`/`AbilityLoadout` constructors remain convenience adapters, and the
`skills()` helper returns only legacy-recognized IDs; new consumers must use the
complete resolved ability list. Adding enum variants is not a content extension.

Catalogs are bounded to 1 MiB and saved scenarios to 128 KiB. Scenario validation
estimates the complete frozen configuration plus 128 KiB reserved for bounded
runtime fields and rejects configurations exceeding the **1 MiB combat snapshot**
budget before Ready/Start. Snapshot validation also bounds actual JSON bytes.
Session/network envelopes must allow their additional catalog, lobby and event
fields above this combat budget and reject oversized input before deserialization.
There is no truncation of abilities or provenance to fit a packet.

## Verification

`cargo test --locked -p labyrinth-rules --profile ci` covers catalog/build rejection,
legacy defaults, source removal, twelve-move serialization, custom stats, all six
weapon moves through real preview/apply, unlimited throw, learned upgrades and
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
