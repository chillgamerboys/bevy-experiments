//! Deterministic, game-owned positional combat for Labyrinth.
//!
//! This package owns rules, content and enemy decisions, not Bevy, sockets, player
//! identities or rendering. Commands are transactional and public snapshots are
//! validated on deserialization. A snapshot deliberately contains no host RNG.

mod combat;
mod content;
mod model;
mod status;

pub use combat::Combat;
pub use content::{skill_definition, skills_for, SkillDefinition, TargetRule};
pub use model::{
    ActorId, ActorKind, ActorSnapshot, CombatAction, CombatEvent, CombatEventKind, CombatOutcome,
    CombatPhase, CombatSnapshot, DamageKind, EnemyKind, HeroClass, InitiativeEntry, RuleError,
    SkillId, Team,
};
pub use status::{
    status_definition, Boundary, DurationClock, Effect, Modifier, Reapplication, RemovalReason,
    Stat, StatusDefinition, StatusInstance, StatusKind, StatusTag,
};

/// Algorithm/interpretation revision included with the canonical content fingerprint.
pub const RULES_VERSION: &str = "labyrinth-combat-v1-splitmix64-frozen-rounds";

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
        .map(|hero| {
            (
                hero,
                hero.name(),
                hero.stats(),
                hero.skills()
                    .iter()
                    .map(|skill| skill_definition(*skill))
                    .collect::<Vec<_>>(),
            )
        })
        .collect();
    let enemies: Vec<_> = EnemyKind::ALL
        .into_iter()
        .map(|enemy| {
            (
                enemy,
                enemy.name(),
                enemy.stats(),
                skills_for(ActorKind::Enemy(enemy))
                    .iter()
                    .map(|skill| skill_definition(*skill))
                    .collect::<Vec<_>>(),
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
    let bytes = serde_json::to_vec(&(RULES_VERSION, heroes, enemies, statuses, "rescue:25%-ceil;reposition:adjacent-swap;defend:brace;wait:consume-turn;enemy-death:compact;hero-down:retain-slot"))
        .expect("fixed typed catalogs contain only JSON-serializable values");
    format!("{:x}", Sha256::digest(bytes))
}
