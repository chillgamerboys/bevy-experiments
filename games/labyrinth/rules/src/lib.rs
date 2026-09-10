//! Deterministic, game-owned positional combat for Labyrinth.
//!
//! This package owns rules, content and enemy decisions, not Bevy, sockets, player
//! identities or rendering. Commands are transactional and public snapshots are
//! validated on deserialization. A snapshot deliberately contains no host RNG.

mod combat;
mod content;
mod formation;
mod life;
mod loadout;
mod model;
mod preview;
mod resolve;
mod status;

pub use combat::Combat;
pub use content::{skill_definition, skills_for, SkillDefinition, TargetRule};
pub use life::{LifeState, CORPSE_ROUNDS, DEATH_SAVE_FAILURES, DEATH_SAVE_TARGET};
pub use loadout::{AbilityLoadout, HeroSetup};
pub use model::{
    ActorId, ActorKind, ActorSnapshot, CombatAction, CombatEvent, CombatEventKind, CombatOutcome,
    CombatPhase, CombatSnapshot, DamageKind, EnemyKind, HeroClass, InitiativeEntry, RuleError,
    SkillId, Team,
};
pub use preview::{
    ActionPreview, ActorPreview, ActorPreviewState, DamagePreview, PreviewEvent, PreviewStatus,
};
pub use status::{
    status_definition, Boundary, DurationClock, Effect, Modifier, Reapplication, RemovalReason,
    Stat, StatusDefinition, StatusInstance, StatusKind, StatusTag, StatusTiming,
};

/// Algorithm/interpretation revision included with the canonical content fingerprint.
pub const RULES_VERSION: &str = "labyrinth-combat-v3-footprints-corpses-death-saves";

/// Maximum human seats and linear rank capacity per team (not a required roster length).
pub const PARTY_SIZE: usize = 6;
/// Maximum actors, including dead identities retained for event/source references.
pub const MAX_ACTORS: usize = PARTY_SIZE * 2;
/// Maximum distinct equipped abilities per actor; universal actions are separate.
pub const MAX_EQUIPPED_ABILITIES: usize = 8;
/// Maximum live status instances per actor.
pub const MAX_STATUSES: usize = 16;
/// Bounded effect/automatic-phase work for one atomic command.
pub const MAX_COMBAT_WORK: usize = 1024;
/// Default playable company: four original roles and one two-rank supply wagon.
pub const PROTOTYPE_HERO_ROSTER: [HeroClass; 5] = [
    HeroClass::Gatekeeper,
    HeroClass::Knifehand,
    HeroClass::Scout,
    HeroClass::FieldMedic,
    HeroClass::LanternWagon,
];
/// Default encounter: each original enemy plus one two-rank Hauler, with stable IDs.
pub const PROTOTYPE_ENEMY_ROSTER: [(ActorId, EnemyKind); 5] = [
    (ActorId(103), EnemyKind::AshBrute),
    (ActorId(104), EnemyKind::IronBrute),
    (ActorId(101), EnemyKind::OssuaryHauler),
    (ActorId(105), EnemyKind::WoundStalker),
    (ActorId(106), EnemyKind::HollowArcher),
];
/// Six-single-rank fixture for capacity and repeated-class tests, not the playable default.
pub const DEFAULT_HERO_ROSTER: [HeroClass; PARTY_SIZE] = [
    HeroClass::Gatekeeper,
    HeroClass::Knifehand,
    HeroClass::Knifehand,
    HeroClass::Scout,
    HeroClass::FieldMedic,
    HeroClass::FieldMedic,
];
/// Six-single-rank fixture with distinct actors sharing content presets.
pub const DEFAULT_ENEMY_ROSTER: [EnemyKind; PARTY_SIZE] = [
    EnemyKind::AshBrute,
    EnemyKind::IronBrute,
    EnemyKind::WoundStalker,
    EnemyKind::WoundStalker,
    EnemyKind::HollowArcher,
    EnemyKind::HollowArcher,
];
/// Stable authored enemy IDs. Hero setup IDs must not collide with these IDs.
pub const DEFAULT_ENEMY_IDS: [ActorId; PARTY_SIZE] = [
    ActorId(101),
    ActorId(102),
    ActorId(103),
    ActorId(104),
    ActorId(105),
    ActorId(106),
];

/// SHA-256 of canonical typed content plus the explicit rules algorithm revision.
///
/// Arrays use fixed catalog order and JSON object fields follow their declaration
/// order. No hash-map, pointer, platform, clock or random value enters this digest.
/// Changing effect semantics requires incrementing [`RULES_VERSION`]; changing
/// authored numbers, skills, status timing or display rules changes this hash directly.
#[must_use]
pub fn rules_fingerprint() -> String {
    use sha2::{Digest, Sha256};
    let heroes: Vec<_> = HeroClass::ALL
        .into_iter()
        .map(|hero| (hero, hero.name(), hero.stats(), hero.skills()))
        .collect();
    let enemies: Vec<_> = EnemyKind::ALL
        .into_iter()
        .map(|enemy| {
            (
                enemy,
                enemy.name(),
                enemy.stats(),
                skills_for(ActorKind::Enemy(enemy)),
            )
        })
        .collect();
    let statuses: Vec<_> = [
        StatusKind::Bleed,
        StatusKind::Brace,
        StatusKind::Haste,
        StatusKind::Weakened,
    ]
    .into_iter()
    .map(status_definition)
    .collect();
    let catalog: Vec<_> = SkillId::ALL.into_iter().map(skill_definition).collect();
    let rosters = (
        DEFAULT_HERO_ROSTER,
        DEFAULT_ENEMY_ROSTER,
        DEFAULT_ENEMY_IDS,
        PROTOTYPE_HERO_ROSTER,
        PROTOTYPE_ENEMY_ROSTER,
    );
    let limits = (
        PARTY_SIZE,
        MAX_ACTORS,
        MAX_EQUIPPED_ABILITIES,
        MAX_STATUSES,
        MAX_COMBAT_WORK,
        100_u16,
        8_u8,
        CORPSE_ROUNDS,
        DEATH_SAVE_TARGET,
        DEATH_SAVE_FAILURES,
    );
    let bytes = serde_json::to_vec(&(RULES_VERSION, heroes, enemies, statuses, catalog, rosters, limits, "formation:ordered-unique-occupants;width:Wagon+Hauler=2,others=1;reach:any-occupied-rank;move:whole-footprints-within-rank-budget;loadout:0..8-unique-catalog-ids;rescue:25%-ceil-dying-only;reposition:adjacent-whole-swap;defend:brace;wait:consume-turn;death:corpse-quarter-hp-ceil;corpse:round-end-ticks-before-expiry,creation-round-excluded;dying:d20-success-holds,damage-one-failure;terminal:no-standing-heroes-defeat,no-standing-enemies-victory"))
        .expect("fixed typed catalogs contain only JSON-serializable values");
    format!("{:x}", Sha256::digest(bytes))
}
