//! Deterministic, game-owned positional combat for Labyrinth.
//!
//! This package owns rules, content and enemy decisions, not Bevy, sockets, player
//! identities or rendering. Commands are transactional and public snapshots are
//! validated on deserialization. A snapshot deliberately contains no host RNG.

mod combat;
mod content;
mod loadout;
mod model;
mod preview;
mod resolve;
mod status;

pub use combat::Combat;
pub use content::{skill_definition, skills_for, SkillDefinition, TargetRule};
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
    Stat, StatusDefinition, StatusInstance, StatusKind, StatusTag,
};

/// Algorithm/interpretation revision included with the canonical content fingerprint.
pub const RULES_VERSION: &str = "labyrinth-combat-v2-six-ranks-explicit-actors-equipped-abilities";

/// Human party size and linear rank capacity per team.
pub const PARTY_SIZE: usize = 6;
/// Total authored actors, including defeated enemies retained for inspection.
pub const MAX_ACTORS: usize = PARTY_SIZE * 2;
/// Maximum distinct equipped abilities per actor; universal actions are separate.
pub const MAX_EQUIPPED_ABILITIES: usize = 8;
/// Maximum live status instances per actor.
pub const MAX_STATUSES: usize = 16;
/// Bounded effect/automatic-phase work for one atomic command.
pub const MAX_COMBAT_WORK: usize = 1024;
/// Initial six-hero formation; class catalog order is intentionally independent.
pub const DEFAULT_HERO_ROSTER: [HeroClass; PARTY_SIZE] = [
    HeroClass::Gatekeeper,
    HeroClass::Knifehand,
    HeroClass::Knifehand,
    HeroClass::Scout,
    HeroClass::FieldMedic,
    HeroClass::FieldMedic,
];
/// Initial six-enemy formation, with distinct actors sharing content presets.
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
    let rosters = (DEFAULT_HERO_ROSTER, DEFAULT_ENEMY_ROSTER, DEFAULT_ENEMY_IDS);
    let limits = (
        PARTY_SIZE,
        MAX_ACTORS,
        MAX_EQUIPPED_ABILITIES,
        MAX_STATUSES,
        MAX_COMBAT_WORK,
        100_u16,
        8_u8,
    );
    let bytes = serde_json::to_vec(&(RULES_VERSION, heroes, enemies, statuses, catalog, rosters, limits, "hero-default-ids:1..6;formation:explicit-array-order;loadout:0..8-unique-catalog-ids;rescue:25%-ceil;reposition:adjacent-swap;defend:brace;wait:consume-turn;enemy-death:compact;hero-down:retain-slot"))
        .expect("fixed typed catalogs contain only JSON-serializable values");
    format!("{:x}", Sha256::digest(bytes))
}
