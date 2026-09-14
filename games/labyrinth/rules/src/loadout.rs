//! Small game-owned setup seam; no inventory, equipment slots, or skill trees.

use crate::{ActorId, HeroClass, RuleError, SkillId, MAX_LEGACY_SKILLS};
use serde::{
    de::{SeqAccess, Visitor},
    Deserialize, Deserializer, Serialize,
};
use std::collections::BTreeSet;

/// Ordered, bounded, duplicate-free equipped skill IDs for one character.
///
/// The class supplies only a starter preset. Any catalog skill may be equipped
/// by trusted encounter setup; no combat command equips or changes skills.
/// An empty loadout is valid because universal actions remain available.
///
/// ```
/// use labyrinth_rules::{LegacySkillLoadout, ActorId, Combat, HeroClass, HeroSetup, SkillId};
///
/// // Six separate characters may share one class without sharing their resources.
/// let mut party = std::array::from_fn(|seat| {
///     HeroSetup::preset(ActorId(20 + seat as u16), HeroClass::Gatekeeper)
/// });
/// party[5].skills = LegacySkillLoadout::new([SkillId::SnapShot, SkillId::Mend])?;
/// let combat = Combat::with_heroes(42, party)?;
/// let snapshot = combat.snapshot();
/// assert_eq!(snapshot.rank(ActorId(25)), Some(6));
/// assert_eq!(snapshot.actor(ActorId(25)).unwrap().legacy_skills(), &[SkillId::SnapShot, SkillId::Mend]);
/// # Ok::<(), labyrinth_rules::RuleError>(())
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(transparent)]
pub struct LegacySkillLoadout(Vec<SkillId>);

impl LegacySkillLoadout {
    /// Validates a loadout, preserving declared UI/decision enumeration order.
    pub fn new(skills: impl IntoIterator<Item = SkillId>) -> Result<Self, RuleError> {
        let mut equipped = Vec::new();
        let mut unique = BTreeSet::new();
        for skill in skills {
            if equipped.len() == MAX_LEGACY_SKILLS || !unique.insert(skill) {
                return Err(RuleError::InvalidLoadout);
            }
            equipped.push(skill);
        }
        Ok(Self(equipped))
    }

    /// Equipped skills in their explicit order; not inferred from class.
    #[must_use]
    pub fn as_slice(&self) -> &[SkillId] {
        &self.0
    }
}

impl<'de> Deserialize<'de> for LegacySkillLoadout {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        struct LoadoutVisitor;
        impl<'de> Visitor<'de> for LoadoutVisitor {
            type Value = LegacySkillLoadout;
            fn expecting(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                write!(f, "at most {MAX_LEGACY_SKILLS} unique equipped skill IDs")
            }
            fn visit_seq<A: SeqAccess<'de>>(self, mut seq: A) -> Result<Self::Value, A::Error> {
                let mut skills = Vec::with_capacity(MAX_LEGACY_SKILLS);
                let mut unique = BTreeSet::new();
                while let Some(skill) = seq.next_element::<SkillId>()? {
                    if skills.len() == MAX_LEGACY_SKILLS || !unique.insert(skill) {
                        return Err(serde::de::Error::custom(RuleError::InvalidLoadout));
                    }
                    skills.push(skill);
                }
                Ok(LegacySkillLoadout(skills))
            }
        }
        deserializer.deserialize_seq(LoadoutVisitor)
    }
}

/// Trusted game-authority input for one hero; array order determines initial rank.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HeroSetup {
    /// Stable character identity, independent of class, player, and rank.
    pub id: ActorId,
    /// Base-stat/visual archetype; never used as equipment authorization.
    pub class: HeroClass,
    /// Explicit immutable encounter loadout, potentially supplied by future systems.
    pub skills: LegacySkillLoadout,
}

impl HeroSetup {
    /// Builds a validated character setup. Roster-wide ID collisions are checked
    /// by [`crate::Combat::with_heroes`] rather than by this individual value.
    pub fn new(
        id: ActorId,
        class: HeroClass,
        skills: LegacySkillLoadout,
    ) -> Result<Self, RuleError> {
        if id.0 == 0 {
            return Err(RuleError::InvalidActorId);
        }
        Ok(Self { id, class, skills })
    }

    /// Uses the class starter skills. The complete setup is validated when
    /// passed to [`crate::Combat::with_heroes`], including the supplied ID.
    #[must_use]
    pub fn preset(id: ActorId, class: HeroClass) -> Self {
        Self {
            id,
            class,
            skills: LegacySkillLoadout::new(class.skills().iter().copied())
                .expect("authored class presets are bounded and unique"),
        }
    }
}
