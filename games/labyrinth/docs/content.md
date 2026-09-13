# Labyrinth content and build authoring

The pure rules crate now has a validated content/build preparation seam. The
playable combat, network and UI still use the legacy `SkillId`/eight-slot path;
this foundation does not claim that authored weapons or twelve-move builds are
playable yet. The next integration must move all consumers to frozen resolved
ability definitions together.

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
collection intent: distinct eligible occupants of enemy ranks 1–2. The runtime
integration must capture those targets before effects and hit a multi-rank actor
once. This module supplies metadata and validates compatible target/effect shape;
it does not implement a second combat resolver.

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

## Migration and verification

The next runtime migration should use `ContentId` in actions and resolve legality,
AI, preview, damage, use counters and inspection from each actor's frozen effective
`AbilityDefinition`. Mapping authored IDs into more `SkillId` enum arms would defeat
the data-authoring contract. Replace the legacy `ActorKind` equality checks for
stats/footprint with independent validated scenario values. Update rules/wire
identity and snapshot validation together, and retain legacy IDs/preset defaults as
compatibility content rather than runtime authority.

`cargo test --locked -p labyrinth-rules --profile ci` covers embedded/legacy parity,
data-only definitions, all six weapon builds, unlimited throws, additions/upgrades,
source removal, twelve-move serialization and tamper rejection, immutable per-build
copies, configurable actor defaults, deterministic ordering/fingerprints, malformed
content and accumulated resource bounds. Existing runtime tests retain their own
HP/status/use independence evidence. These tests do not establish native UI access,
cleave execution, networking or balance; those remain integration acceptance.
