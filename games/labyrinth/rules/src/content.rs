//! Small original typed content catalog. Masks use bit zero for front rank one.

use crate::{ActorKind, Effect, EnemyKind, HeroClass, SkillId, StatusKind, StatusTag};
use serde::Serialize;

/// Target allegiance/life-state rule, separate from source and target rank masks.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub enum TargetRule {
    /// Standing actor on the opposing team.
    EnemyStanding,
    /// Standing actor on the source team, including self.
    AllyStanding,
    /// Exactly the acting standing character.
    SelfStanding,
    /// Another ally, including downed heroes.
    OtherAlly,
    /// Downed hero ally.
    AllyDowned,
}

/// Immutable skill content used by legality, AI, effects and UI inspection.
#[derive(Debug, Clone, Copy, Serialize)]
pub struct SkillDefinition {
    /// Catalog identity.
    pub id: SkillId,
    /// Original display name.
    pub name: &'static str,
    /// Plain-language effect explanation.
    pub description: &'static str,
    /// Allowed acting ranks, bit zero corresponding to front rank one.
    pub source_ranks: u8,
    /// Reachable target ranks using the same four-bit convention.
    pub target_ranks: u8,
    /// Target allegiance and standing/downed constraints.
    pub target_rule: TargetRule,
    /// Encounter use limit, or unlimited.
    pub max_uses: Option<u8>,
    /// Effects execute in this explicit order after all target validation.
    pub effects: &'static [Effect],
}

/// Skill IDs in UI order for a content-defined actor.
#[must_use]
pub const fn skills_for(kind: ActorKind) -> &'static [SkillId] {
    match kind {
        ActorKind::Hero(HeroClass::Gatekeeper) => &[
            SkillId::FrontStrike,
            SkillId::LongReach,
            SkillId::DrivingBlow,
            SkillId::FieldDressing,
        ],
        ActorKind::Hero(HeroClass::Knifehand) => &[
            SkillId::BleedingCut,
            SkillId::DeepStrike,
            SkillId::ThrownKnife,
            SkillId::CleanBlade,
        ],
        ActorKind::Hero(HeroClass::Scout) => &[
            SkillId::BackRankShot,
            SkillId::SnapShot,
            SkillId::HookShot,
            SkillId::Exchange,
        ],
        ActorKind::Hero(HeroClass::FieldMedic) => &[
            SkillId::Mend,
            SkillId::Staunch,
            SkillId::StaffStrike,
            SkillId::Rally,
        ],
        ActorKind::Enemy(EnemyKind::AshBrute | EnemyKind::IronBrute) => &[SkillId::BrutalStrike],
        ActorKind::Enemy(EnemyKind::WoundStalker) => &[SkillId::RaggedCut],
        ActorKind::Enemy(EnemyKind::HollowArcher) => &[SkillId::HollowBolt],
    }
}

/// Resolve a skill's complete typed definition.
#[must_use]
pub const fn skill_definition(id: SkillId) -> SkillDefinition {
    let (name, description, source_ranks, target_ranks, target_rule, max_uses, effects): (
        _,
        _,
        _,
        _,
        _,
        _,
        &'static [Effect],
    ) = match id {
        SkillId::FrontStrike => (
            "Front Strike",
            "Deal 7 damage to a front enemy.",
            0b0011,
            0b0011,
            TargetRule::EnemyStanding,
            None,
            &[Effect::Damage(7)],
        ),
        SkillId::LongReach => (
            "Long Reach",
            "Deal 5 damage with extended reach.",
            0b0111,
            0b0111,
            TargetRule::EnemyStanding,
            None,
            &[Effect::Damage(5)],
        ),
        SkillId::DrivingBlow => (
            "Driving Blow",
            "Deal 4 damage, then push the surviving target back one rank.",
            0b0011,
            0b0011,
            TargetRule::EnemyStanding,
            None,
            &[Effect::Damage(4), Effect::Move(1)],
        ),
        SkillId::FieldDressing => (
            "Field Dressing",
            "Restore 8 HP to yourself. Does not cleanse bleed. 2 uses.",
            0b1111,
            0b1111,
            TargetRule::SelfStanding,
            Some(2),
            &[Effect::Heal(8)],
        ),
        SkillId::BleedingCut => (
            "Bleeding Cut",
            "Deal 3 damage and apply Bleed: 2 damage at the next 3 turn starts.",
            0b0011,
            0b0111,
            TargetRule::EnemyStanding,
            None,
            &[Effect::Damage(3), Effect::ApplyStatus(StatusKind::Bleed)],
        ),
        SkillId::DeepStrike => (
            "Deep Strike",
            "Deal 6 damage to a front enemy.",
            0b0111,
            0b0011,
            TargetRule::EnemyStanding,
            None,
            &[Effect::Damage(6)],
        ),
        SkillId::ThrownKnife => (
            "Thrown Knife",
            "Deal 4 damage to any enemy rank.",
            0b1110,
            0b1111,
            TargetRule::EnemyStanding,
            None,
            &[Effect::Damage(4)],
        ),
        SkillId::CleanBlade => (
            "Clean Blade",
            "Remove your Bleed without healing HP.",
            0b1111,
            0b1111,
            TargetRule::SelfStanding,
            None,
            &[Effect::Cleanse(StatusTag::Bleeding)],
        ),
        SkillId::BackRankShot => (
            "Back-rank Shot",
            "Deal 7 damage to an enemy in the rear two ranks.",
            0b1100,
            0b1100,
            TargetRule::EnemyStanding,
            None,
            &[Effect::Damage(7)],
        ),
        SkillId::SnapShot => (
            "Snap Shot",
            "Deal 4 damage from any rank to any enemy rank.",
            0b1111,
            0b1111,
            TargetRule::EnemyStanding,
            None,
            &[Effect::Damage(4)],
        ),
        SkillId::HookShot => (
            "Hook Shot",
            "Deal 3 damage, then pull the surviving target forward one rank.",
            0b1110,
            0b1110,
            TargetRule::EnemyStanding,
            None,
            &[Effect::Damage(3), Effect::Move(-1)],
        ),
        SkillId::Exchange => (
            "Exchange",
            "Swap places with any other ally, including a downed ally.",
            0b1111,
            0b1111,
            TargetRule::OtherAlly,
            None,
            &[Effect::SwapWithSource],
        ),
        SkillId::Mend => (
            "Mend",
            "Restore 8 HP to a standing ally. Does not cleanse bleed. 3 uses.",
            0b1100,
            0b1111,
            TargetRule::AllyStanding,
            Some(3),
            &[Effect::Heal(8)],
        ),
        SkillId::Staunch => (
            "Staunch",
            "Remove Bleed from a standing ally, including yourself.",
            0b1111,
            0b1111,
            TargetRule::AllyStanding,
            None,
            &[Effect::Cleanse(StatusTag::Bleeding)],
        ),
        SkillId::StaffStrike => (
            "Staff Strike",
            "Deal 3 damage to an enemy in the front three ranks.",
            0b1111,
            0b0111,
            TargetRule::EnemyStanding,
            None,
            &[Effect::Damage(3)],
        ),
        SkillId::Rally => (
            "Rally",
            "Rescue a downed ally at 50% maximum HP. 2 uses.",
            0b1111,
            0b1111,
            TargetRule::AllyDowned,
            Some(2),
            &[Effect::Rescue(50)],
        ),
        SkillId::BrutalStrike => (
            "Brutal Strike",
            "Deal 5 damage to a standing front-rank hero.",
            0b0011,
            0b0011,
            TargetRule::EnemyStanding,
            None,
            &[Effect::Damage(5)],
        ),
        SkillId::RaggedCut => (
            "Ragged Cut",
            "Deal 2 damage and apply Bleed.",
            0b1111,
            0b0111,
            TargetRule::EnemyStanding,
            None,
            &[Effect::Damage(2), Effect::ApplyStatus(StatusKind::Bleed)],
        ),
        SkillId::HollowBolt => (
            "Hollow Bolt",
            "Deal 4 damage to any standing hero.",
            0b1111,
            0b1111,
            TargetRule::EnemyStanding,
            None,
            &[Effect::Damage(4)],
        ),
    };
    SkillDefinition {
        id,
        name,
        description,
        source_ranks,
        target_ranks,
        target_rule,
        max_uses,
        effects,
    }
}
