//! Small game-owned setup seam; no inventory, equipment slots, or skill trees.

use crate::{ActorId, HeroClass, RuleError, SkillId, MAX_EQUIPPED_ABILITIES};
use serde::{
    de::{SeqAccess, Visitor},
    Deserialize, Deserializer, Serialize,
};
use std::collections::BTreeSet;

/// Ordered, bounded, duplicate-free equipped ability IDs for one character.
///
/// The class supplies only a starter preset. Any catalog ability may be equipped
/// by trusted encounter setup; no combat command equips or changes abilities.
/// An empty loadout is valid because universal actions remain available.
///
/// ```
/// use labyrinth_rules::{AbilityLoadout, ActorId, Combat, HeroClass, HeroSetup, SkillId};
///
/// // Six separate characters may share one class without sharing their resources.
/// let mut party = std::array::from_fn(|seat| {
///     HeroSetup::preset(ActorId(20 + seat as u16), HeroClass::Gatekeeper)
/// });
/// party[5].abilities = AbilityLoadout::new([SkillId::SnapShot, SkillId::Mend])?;
/// let combat = Combat::with_heroes(42, party)?;
/// let snapshot = combat.snapshot();
/// assert_eq!(snapshot.rank(ActorId(25)), Some(6));
/// assert_eq!(snapshot.actor(ActorId(25)).unwrap().skills(), &[SkillId::SnapShot, SkillId::Mend]);
/// # Ok::<(), labyrinth_rules::RuleError>(())
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(transparent)]
pub struct AbilityLoadout(Vec<SkillId>);

impl AbilityLoadout {
    /// Validates a loadout, preserving declared UI/decision enumeration order.
    pub fn new(skills: impl IntoIterator<Item = SkillId>) -> Result<Self, RuleError> {
        let mut equipped = Vec::new();
        let mut unique = BTreeSet::new();
        for skill in skills {
            if equipped.len() == MAX_EQUIPPED_ABILITIES || !unique.insert(skill) {
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

impl<'de> Deserialize<'de> for AbilityLoadout {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        struct LoadoutVisitor;
        impl<'de> Visitor<'de> for LoadoutVisitor {
            type Value = AbilityLoadout;
            fn expecting(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                write!(
                    f,
                    "at most {MAX_EQUIPPED_ABILITIES} unique equipped skill IDs"
                )
            }
            fn visit_seq<A: SeqAccess<'de>>(self, mut seq: A) -> Result<Self::Value, A::Error> {
                let mut skills = Vec::with_capacity(MAX_EQUIPPED_ABILITIES);
                let mut unique = BTreeSet::new();
                while let Some(skill) = seq.next_element::<SkillId>()? {
                    if skills.len() == MAX_EQUIPPED_ABILITIES || !unique.insert(skill) {
                        return Err(serde::de::Error::custom(RuleError::InvalidLoadout));
                    }
                    skills.push(skill);
                }
                Ok(AbilityLoadout(skills))
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
    pub abilities: AbilityLoadout,
}

impl HeroSetup {
    /// Builds a validated character setup. Roster-wide ID collisions are checked
    /// by [`crate::Combat::with_heroes`] rather than by this individual value.
    pub fn new(
        id: ActorId,
        class: HeroClass,
        abilities: AbilityLoadout,
    ) -> Result<Self, RuleError> {
        if id.0 == 0 {
            return Err(RuleError::InvalidActorId);
        }
        Ok(Self {
            id,
            class,
            abilities,
        })
    }

    /// Uses the class starter abilities. The complete setup is validated when
    /// passed to [`crate::Combat::with_heroes`], including the supplied ID.
    #[must_use]
    pub fn preset(id: ActorId, class: HeroClass) -> Self {
        Self {
            id,
            class,
            abilities: AbilityLoadout::new(class.skills().iter().copied())
                .expect("authored class presets are bounded and unique"),
        }
    }
}
