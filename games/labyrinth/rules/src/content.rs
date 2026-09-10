//! Small original typed content catalog. Masks use bit zero for front rank one.

use crate::{ActorKind, Effect, EnemyKind, HeroClass, SkillId, StatusKind, StatusTag};
use serde::Serialize;

// Six linear ranks form three explicit two-rank reach zones. These masks are
// content decisions, not an arithmetic expansion of the former four-rank board.
const FRONT: u8 = 0b00_0011;
const FRONT_MIDDLE: u8 = 0b00_1111;
const MIDDLE_REAR: u8 = 0b11_1100;
const REAR: u8 = 0b11_0000;
const ALL_RANKS: u8 = 0b11_1111;

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
    /// Reachable target ranks using the same six-bit convention.
    pub target_ranks: u8,
    /// Target allegiance and standing/downed constraints.
    pub target_rule: TargetRule,
    /// Encounter use limit, or unlimited.
    pub max_uses: Option<u8>,
    /// Effects execute in this explicit order after all target validation.
    pub effects: &'static [Effect],
}

impl SkillDefinition {
    /// Whether a one-based source rank is in this skill's six-rank reach mask.
    #[must_use]
    pub fn allows_source_rank(&self, rank: u8) -> bool {
        allows_rank(self.source_ranks, rank)
    }
    /// Whether a one-based target rank is in this skill's six-rank reach mask.
    #[must_use]
    pub fn allows_target_rank(&self, rank: u8) -> bool {
        allows_rank(self.target_ranks, rank)
    }
}

fn allows_rank(mask: u8, rank: u8) -> bool {
    rank != 0 && usize::from(rank) <= crate::PARTY_SIZE && mask & (1_u8 << (rank - 1)) != 0
}

/// Starter-preset skill IDs in UI order. An actor's equipped loadout is separate.
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
        ActorKind::Hero(HeroClass::LanternWagon) => &[SkillId::HurledScrap, SkillId::SpareBandage],
        ActorKind::Enemy(EnemyKind::OssuaryHauler) => &[SkillId::CrushingBlow],
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
        SkillId::HurledScrap => (
            "Hurled Scrap",
            "Deal 2 damage. Supplies, not fighting, are the wagon's purpose.",
            ALL_RANKS,
            ALL_RANKS,
            TargetRule::EnemyStanding,
            None,
            &[Effect::Damage(2)],
        ),
        SkillId::SpareBandage => (
            "Spare Bandage",
            "Restore 3 HP to a standing ally. 2 uses.",
            ALL_RANKS,
            ALL_RANKS,
            TargetRule::AllyStanding,
            Some(2),
            &[Effect::Heal(3)],
        ),
        SkillId::CrushingBlow => (
            "Crushing Blow",
            "Reach over the frontline to deal 7 damage to a front or middle target.",
            FRONT_MIDDLE,
            FRONT_MIDDLE,
            TargetRule::EnemyStanding,
            None,
            &[Effect::Damage(7)],
        ),
        SkillId::FrontStrike => (
            "Front Strike",
            "Deal 7 damage to a front enemy.",
            FRONT,
            FRONT,
            TargetRule::EnemyStanding,
            None,
            &[Effect::Damage(7)],
        ),
        SkillId::LongReach => (
            "Long Reach",
            "Deal 5 damage with extended reach.",
            FRONT_MIDDLE,
            FRONT_MIDDLE,
            TargetRule::EnemyStanding,
            None,
            &[Effect::Damage(5)],
        ),
        SkillId::DrivingBlow => (
            "Driving Blow",
            "Deal 4 damage, then push the surviving target back one rank.",
            FRONT,
            FRONT,
            TargetRule::EnemyStanding,
            None,
            &[Effect::Damage(4), Effect::Move(1)],
        ),
        SkillId::FieldDressing => (
            "Field Dressing",
            "Restore 8 HP to yourself. Does not cleanse bleed. 2 uses.",
            ALL_RANKS,
            ALL_RANKS,
            TargetRule::SelfStanding,
            Some(2),
            &[Effect::Heal(8)],
        ),
        SkillId::BleedingCut => (
            "Bleeding Cut",
            "Deal 3 damage and apply Bleed: 2 damage at the next 3 turn starts.",
            FRONT_MIDDLE,
            FRONT_MIDDLE,
            TargetRule::EnemyStanding,
            None,
            &[Effect::Damage(3), Effect::ApplyStatus(StatusKind::Bleed)],
        ),
        SkillId::DeepStrike => (
            "Deep Strike",
            "Deal 6 damage to a front enemy.",
            FRONT_MIDDLE,
            FRONT,
            TargetRule::EnemyStanding,
            None,
            &[Effect::Damage(6)],
        ),
        SkillId::ThrownKnife => (
            "Thrown Knife",
            "Deal 4 damage to any enemy rank.",
            MIDDLE_REAR,
            ALL_RANKS,
            TargetRule::EnemyStanding,
            None,
            &[Effect::Damage(4)],
        ),
        SkillId::CleanBlade => (
            "Clean Blade",
            "Remove your Bleed without healing HP.",
            ALL_RANKS,
            ALL_RANKS,
            TargetRule::SelfStanding,
            None,
            &[Effect::Cleanse(StatusTag::Bleeding)],
        ),
        SkillId::BackRankShot => (
            "Back-rank Shot",
            "Deal 7 damage to an enemy in rear ranks 5 and 6.",
            MIDDLE_REAR,
            REAR,
            TargetRule::EnemyStanding,
            None,
            &[Effect::Damage(7)],
        ),
        SkillId::SnapShot => (
            "Snap Shot",
            "Deal 4 damage from any rank to any enemy rank.",
            ALL_RANKS,
            ALL_RANKS,
            TargetRule::EnemyStanding,
            None,
            &[Effect::Damage(4)],
        ),
        SkillId::HookShot => (
            "Hook Shot",
            "Deal 3 damage, then pull the surviving target forward one rank.",
            MIDDLE_REAR,
            MIDDLE_REAR,
            TargetRule::EnemyStanding,
            None,
            &[Effect::Damage(3), Effect::Move(-1)],
        ),
        SkillId::Exchange => (
            "Exchange",
            "Swap places with any other ally, including a downed ally.",
            ALL_RANKS,
            ALL_RANKS,
            TargetRule::OtherAlly,
            None,
            &[Effect::SwapWithSource],
        ),
        SkillId::Mend => (
            "Mend",
            "Restore 8 HP to a standing ally. Does not cleanse bleed. 3 uses.",
            MIDDLE_REAR,
            ALL_RANKS,
            TargetRule::AllyStanding,
            Some(3),
            &[Effect::Heal(8)],
        ),
        SkillId::Staunch => (
            "Staunch",
            "Remove Bleed from a standing ally, including yourself.",
            ALL_RANKS,
            ALL_RANKS,
            TargetRule::AllyStanding,
            None,
            &[Effect::Cleanse(StatusTag::Bleeding)],
        ),
        SkillId::StaffStrike => (
            "Staff Strike",
            "Deal 3 damage to an enemy in front or middle ranks 1 to 4.",
            ALL_RANKS,
            FRONT_MIDDLE,
            TargetRule::EnemyStanding,
            None,
            &[Effect::Damage(3)],
        ),
        SkillId::Rally => (
            "Rally",
            "Rescue a downed ally at 50% maximum HP. 2 uses.",
            ALL_RANKS,
            ALL_RANKS,
            TargetRule::AllyDowned,
            Some(2),
            &[Effect::Rescue(50)],
        ),
        SkillId::BrutalStrike => (
            "Brutal Strike",
            "Deal 5 damage to a standing front-rank hero.",
            FRONT,
            FRONT,
            TargetRule::EnemyStanding,
            None,
            &[Effect::Damage(5)],
        ),
        SkillId::RaggedCut => (
            "Ragged Cut",
            "Deal 2 damage and apply Bleed.",
            ALL_RANKS,
            FRONT_MIDDLE,
            TargetRule::EnemyStanding,
            None,
            &[Effect::Damage(2), Effect::ApplyStatus(StatusKind::Bleed)],
        ),
        SkillId::HollowBolt => (
            "Hollow Bolt",
            "Deal 4 damage to any standing hero.",
            ALL_RANKS,
            ALL_RANKS,
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
