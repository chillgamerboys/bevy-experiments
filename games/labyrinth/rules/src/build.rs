//! Pure character/equipment composition and a frozen, inspectable active Moveset.
use crate::{
    catalog::{
        apply_upgrade, text_field, unique_ids, validate_stats, AbilityDefinition, ContentCatalog,
        ContentError, ContentId, PassiveEffect, SkillDefinition, SkillUpgrade, MAX_BUILD_ABILITIES,
        MAX_MOVESET_SKILLS,
    },
    status_definition, ActorKind, StatusKind, StatusTag,
};
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;

/// A personally selected active Skill and its descriptive origin.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SkillGrant {
    /// Active Skill identity.
    pub skill: ContentId,
    /// Descriptive source identity, never an equipment authorization.
    pub provenance: ContentId,
}
/// Encounter selections; this is one weapon slot, not an inventory.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CharacterBuild {
    /// Ordered personal active selections, including temporarily inactive Skills.
    #[serde(default)]
    pub skills: Vec<SkillGrant>,
    /// Personal passive selections, including temporarily inactive Abilities.
    #[serde(default)]
    pub abilities: Vec<ContentId>,
    /// Optional equipped weapon.
    pub weapon: Option<ContentId>,
}
/// Ownership source, independent of active/passive behavior or requirements.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum GrantKind {
    /// A personal selection.
    Character,
    /// An equipped item.
    Equipment,
}
/// One retained grant path; duplicate sources never multiply an effect.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct GrantSource {
    /// Character or equipment grant.
    pub kind: GrantKind,
    /// Granting Skill/Ability or item identity.
    pub definition: ContentId,
    /// Descriptive origin.
    pub provenance: ContentId,
}
/// One applied passive contribution, with all provenance retained by its Ability.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct UpgradeContribution {
    /// The Ability identity, independent of how many sources granted it.
    pub ability: ContentId,
    /// The exact additive contribution.
    pub upgrade: SkillUpgrade,
}
/// A frozen effective active Skill; uses remain on its runtime actor.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ResolvedSkill {
    /// Effective definition after passive contributions.
    pub definition: SkillDefinition,
    /// All distinct grants, in deterministic order.
    pub grants: Vec<GrantSource>,
    /// Passive contributions in stable Ability-ID order.
    pub upgrades: Vec<UpgradeContribution>,
}
/// The canonical collection of eligible active Skills. Never contains passives.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Moveset {
    /// Actor-local order, frozen at encounter start.
    pub skills: Vec<ResolvedSkill>,
}
/// A selected Skill whose equipment requirements are currently unmet.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct InactiveSkill {
    /// Unmodified active definition.
    pub definition: SkillDefinition,
    /// Its retained grant paths.
    pub grants: Vec<GrantSource>,
    /// Player-facing unmet prerequisite.
    pub reason: String,
}
/// One passive Ability, applied at most once even with multiple sources.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ResolvedAbility {
    /// Frozen passive definition.
    pub definition: AbilityDefinition,
    /// Every retained grant source.
    pub grants: Vec<GrantSource>,
    /// Absent when active; otherwise this Ability contributes no effects.
    pub inactive_reason: Option<String>,
}
impl ResolvedAbility {
    /// Whether this passive currently contributes to its actor.
    #[must_use]
    pub fn active(&self) -> bool {
        self.inactive_reason.is_none()
    }
}
/// Complete derived build; validate untrusted input with `catalog.validate_resolved`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ResolvedBuild {
    /// Catalog/rules compatibility identity.
    pub catalog_fingerprint: String,
    /// Eligible active Skills with effective values.
    pub moveset: Moveset,
    /// Selected/equipped passives, including explicit inactive reasons.
    pub abilities: Vec<ResolvedAbility>,
    /// Selected active Skills retained outside the Moveset while ineligible.
    pub inactive_skills: Vec<InactiveSkill>,
}
impl ResolvedBuild {
    /// Duration for a newly applied/refreshed status. Restored remaining clocks
    /// must bypass this calculation, so snapshot reads never shorten them again.
    #[must_use]
    pub fn status_duration(&self, kind: StatusKind, ticks: u8) -> u8 {
        if !status_definition(kind).tags.contains(&StatusTag::Debuff) {
            return ticks;
        }
        let reduction = self
            .abilities
            .iter()
            .filter(|a| a.active())
            .flat_map(|a| &a.definition.effects)
            .fold(0u8, |total, effect| match effect {
                PassiveEffect::ReduceNegativeStatusDuration { amount } => {
                    total.saturating_add(*amount)
                }
            });
        ticks.saturating_sub(reduction).max(1)
    }
}
/// Fully explicit actor setup; appearance supplies no hidden stat/build authority.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ActorBuild {
    /// Display name.
    pub name: String,
    /// Existing visual archetype only.
    pub appearance: ActorKind,
    /// Configured maximum HP.
    pub max_hp: u16,
    /// Configured base speed.
    pub base_speed: u16,
    /// Configured occupied ranks.
    pub footprint: u8,
    /// Composed build choices.
    pub build: CharacterBuild,
}
impl ActorBuild {
    /// Copy defaults for editing, without retaining an invariant tied to the preset.
    pub fn from_preset(catalog: &ContentCatalog, id: &ContentId) -> Result<Self, ContentError> {
        let p = catalog
            .actor_preset(id)
            .ok_or_else(|| ContentError::new("actor.preset", format!("missing preset {id}")))?;
        Ok(Self {
            name: p.name.clone(),
            appearance: p.appearance,
            max_hp: p.max_hp,
            base_speed: p.base_speed,
            footprint: p.footprint,
            build: p.build.clone(),
        })
    }
    /// Validate configurable stats and return the effective encounter build.
    pub fn resolve(&self, catalog: &ContentCatalog) -> Result<ResolvedBuild, ContentError> {
        text_field("actor.name", &self.name, 128)?;
        validate_stats("actor", self.max_hp, self.base_speed, self.footprint)?;
        catalog.resolve_build(&self.build)
    }
}

impl ContentCatalog {
    /// Resolve selections once without mutating source content or runtime resources.
    pub fn resolve_build(&self, build: &CharacterBuild) -> Result<ResolvedBuild, ContentError> {
        if build.skills.len() > MAX_MOVESET_SKILLS {
            return Err(ContentError::new("build.skills", "too many Skill grants"));
        }
        unique_ids(
            "build.abilities",
            &build.abilities.iter().collect::<Vec<_>>(),
            MAX_BUILD_ABILITIES,
        )?;
        let mut skills = Vec::<ResolvedSkill>::new();
        let mut abilities = Vec::<ResolvedAbility>::new();
        let mut paths = BTreeSet::new();
        for grant in &build.skills {
            if !paths.insert((&grant.skill, &grant.provenance)) {
                return Err(ContentError::new(
                    "build.skills",
                    "duplicate Skill/provenance grant",
                ));
            }
            let definition = self.skill(&grant.skill).ok_or_else(|| {
                ContentError::new("build.skills", format!("missing Skill {}", grant.skill))
            })?;
            if !definition.personal_selectable {
                return Err(ContentError::new(
                    "build.skills",
                    format!("{} is equipment-only", grant.skill),
                ));
            }
            add_skill(
                self,
                &mut skills,
                &grant.skill,
                GrantSource {
                    kind: GrantKind::Character,
                    definition: grant.skill.clone(),
                    provenance: grant.provenance.clone(),
                },
            )?;
        }
        for id in &build.abilities {
            let definition = self.ability(id).ok_or_else(|| {
                ContentError::new("build.abilities", format!("missing Ability {id}"))
            })?;
            if !definition.personal_selectable {
                return Err(ContentError::new(
                    "build.abilities",
                    format!("{id} is equipment-only"),
                ));
            }
            add_ability(
                self,
                &mut abilities,
                id,
                GrantSource {
                    kind: GrantKind::Character,
                    definition: id.clone(),
                    provenance: definition.provenance.clone(),
                },
            )?;
        }
        if let Some(id) = &build.weapon {
            let weapon = self
                .weapon(id)
                .ok_or_else(|| ContentError::new("build.weapon", format!("missing weapon {id}")))?;
            let source = GrantSource {
                kind: GrantKind::Equipment,
                definition: id.clone(),
                provenance: id.clone(),
            };
            for id in &weapon.skills {
                add_skill(self, &mut skills, id, source.clone())?;
            }
            for id in &weapon.abilities {
                add_ability(self, &mut abilities, id, source.clone())?;
            }
        }
        let mut inactive_skills = Vec::new();
        let mut eligible = Vec::new();
        for skill in skills {
            if let Some(reason) = self.unmet_requirements(build, &skill.definition.requirements) {
                inactive_skills.push(InactiveSkill {
                    definition: skill.definition,
                    grants: skill.grants,
                    reason,
                });
            } else {
                eligible.push(skill);
            }
        }
        abilities.sort_by(|a, b| a.definition.id.cmp(&b.definition.id));
        for ability in &mut abilities {
            ability.inactive_reason = self
                .unmet_requirements(build, &ability.definition.requirements)
                .or_else(|| {
                    ability
                        .definition
                        .upgrades
                        .iter()
                        .find(|upgrade| {
                            !eligible
                                .iter()
                                .any(|skill| skill.definition.id == upgrade.skill)
                        })
                        .map(|upgrade| format!("Requires eligible Skill {}", upgrade.skill))
                });
            if !ability.active() {
                continue;
            }
            for upgrade in &ability.definition.upgrades {
                let skill = eligible
                    .iter_mut()
                    .find(|skill| skill.definition.id == upgrade.skill)
                    .expect("validated eligible upgrade target");
                let base = self
                    .skill(&upgrade.skill)
                    .expect("catalog validated upgrade target");
                apply_upgrade(
                    base,
                    &mut skill.definition,
                    upgrade,
                    &format!("build.abilities.{}", ability.definition.id),
                )?;
                skill.upgrades.push(UpgradeContribution {
                    ability: ability.definition.id.clone(),
                    upgrade: upgrade.clone(),
                });
            }
        }
        Ok(ResolvedBuild {
            catalog_fingerprint: self.fingerprint(),
            moveset: Moveset { skills: eligible },
            abilities,
            inactive_skills,
        })
    }
}
fn add_skill(
    catalog: &ContentCatalog,
    skills: &mut Vec<ResolvedSkill>,
    id: &ContentId,
    source: GrantSource,
) -> Result<(), ContentError> {
    if let Some(skill) = skills.iter_mut().find(|s| &s.definition.id == id) {
        skill.grants.push(source);
        return Ok(());
    }
    if skills.len() >= MAX_MOVESET_SKILLS {
        return Err(ContentError::new(
            "build.skills",
            "too many distinct granted Skills (maximum 64)",
        ));
    }
    let definition = catalog
        .skill(id)
        .ok_or_else(|| ContentError::new("build.skills", format!("missing Skill {id}")))?
        .clone();
    skills.push(ResolvedSkill {
        definition,
        grants: vec![source],
        upgrades: vec![],
    });
    Ok(())
}
fn add_ability(
    catalog: &ContentCatalog,
    abilities: &mut Vec<ResolvedAbility>,
    id: &ContentId,
    source: GrantSource,
) -> Result<(), ContentError> {
    if let Some(ability) = abilities.iter_mut().find(|a| &a.definition.id == id) {
        ability.grants.push(source);
        return Ok(());
    }
    if abilities.len() >= MAX_BUILD_ABILITIES {
        return Err(ContentError::new(
            "build.abilities",
            "too many distinct granted Abilities (maximum 64)",
        ));
    }
    let definition = catalog
        .ability(id)
        .ok_or_else(|| ContentError::new("build.abilities", format!("missing Ability {id}")))?
        .clone();
    abilities.push(ResolvedAbility {
        definition,
        grants: vec![source],
        inactive_reason: None,
    });
    Ok(())
}
