//! Pure build composition: immutable definitions and inspectable provenance.

use crate::{
    catalog::{
        apply_upgrade, text_field, unique_ids, validate_stats, AbilityDefinition, AbilityUpgrade,
        ContentCatalog, ContentError, ContentId, MAX_RESOLVED_ABILITIES,
    },
    ActorKind,
};
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;

/// An innate grant can carry a named origin without defining classes or trees.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct InnateGrant {
    /// Granted active ability.
    pub ability: ContentId,
    /// Source identity, e.g. innate, bulwark or pyromancy.
    pub provenance: ContentId,
}
/// Encounter build selections. This is equipment configuration, not an inventory.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CharacterBuild {
    /// Ordered innate grants (different sources may grant the same ability).
    #[serde(default)]
    pub innate: Vec<InnateGrant>,
    /// Selected learned skills. Resolver sorts by stable ID for deterministic upgrades.
    #[serde(default)]
    pub learned_skills: Vec<ContentId>,
    /// Exactly one optional equipped weapon.
    pub weapon: Option<ContentId>,
}
/// Grant category independent of active/passive behavior.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum GrantKind {
    /// Direct actor grant.
    Innate,
    /// Equipped weapon grant.
    Weapon,
    /// Selected learned-skill grant or upgrade.
    Learned,
}
/// A retained grant path. Multiple paths never duplicate an active move.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct GrantSource {
    /// Direct, weapon or learned grant.
    pub kind: GrantKind,
    /// Granting definition identity (ability identity for direct innate grants).
    pub definition: ContentId,
    /// Named source/discipline used for inspection.
    pub provenance: ContentId,
}
/// An applied upgrade retained separately from base grant sources.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct UpgradeContribution {
    /// Learned skill and its descriptive provenance.
    pub source: GrantSource,
    /// Exact operations used to derive the effective ability.
    pub upgrade: AbilityUpgrade,
}
/// One effective ability with every grant and upgrade source, without mutable uses.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ResolvedAbility {
    /// Owned effective definition, frozen for the encounter.
    pub definition: AbilityDefinition,
    /// Distinct grant paths in deterministic order.
    pub grants: Vec<GrantSource>,
    /// Applied learned contributions in stable skill-ID order.
    pub upgrades: Vec<UpgradeContribution>,
}
/// Derived build view; untrusted deserialization needs `catalog.validate_resolved`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ResolvedBuild {
    /// Catalog/rules identity used for this resolution.
    pub catalog_fingerprint: String,
    /// Every granted active move, ordered innate → weapon → learned stable IDs.
    pub abilities: Vec<ResolvedAbility>,
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
    /// Resolve once at preparation time, never mutating input or catalog.
    ///
    /// All grants are collected before upgrades, so a learned skill may upgrade a
    /// weapon move or one granted by another selected skill. Skills cannot grant
    /// skills, hence cycles are structurally unavailable. Missing prerequisite
    /// moves fail instead of silently ignoring upgrades. Re-resolution after source
    /// removal removes only that source's grants/contributions; resources belong
    /// to runtime actors and must not be reset by resolving a presentation view.
    pub fn resolve_build(&self, build: &CharacterBuild) -> Result<ResolvedBuild, ContentError> {
        if build.innate.len() > MAX_RESOLVED_ABILITIES {
            return Err(ContentError::new("build.innate", "too many innate grants"));
        }
        unique_ids(
            "build.learned_skills",
            &build.learned_skills.iter().collect::<Vec<_>>(),
            MAX_RESOLVED_ABILITIES,
        )?;
        let mut abilities = Vec::<ResolvedAbility>::new();
        let mut innate_paths = BTreeSet::new();
        for innate in &build.innate {
            if !innate_paths.insert((&innate.ability, &innate.provenance)) {
                return Err(ContentError::new(
                    "build.innate",
                    "duplicate ability/provenance grant",
                ));
            }
            add_grant(
                self,
                &mut abilities,
                &innate.ability,
                GrantSource {
                    kind: GrantKind::Innate,
                    definition: innate.ability.clone(),
                    provenance: innate.provenance.clone(),
                },
            )?;
        }
        if let Some(id) = &build.weapon {
            let weapon = self
                .weapon(id)
                .ok_or_else(|| ContentError::new("build.weapon", format!("missing weapon {id}")))?;
            for ability in &weapon.grants {
                add_grant(
                    self,
                    &mut abilities,
                    ability,
                    GrantSource {
                        kind: GrantKind::Weapon,
                        definition: id.clone(),
                        provenance: id.clone(),
                    },
                )?;
            }
        }
        let mut learned = build.learned_skills.iter().collect::<Vec<_>>();
        learned.sort();
        for id in &learned {
            let skill = self.learned_skill(id).ok_or_else(|| {
                ContentError::new(
                    "build.learned_skills",
                    format!("missing learned skill {id}"),
                )
            })?;
            for ability in &skill.grants {
                add_grant(
                    self,
                    &mut abilities,
                    ability,
                    GrantSource {
                        kind: GrantKind::Learned,
                        definition: skill.id.clone(),
                        provenance: skill.provenance.clone(),
                    },
                )?;
            }
        }
        for id in learned {
            let skill = self.learned_skill(id).ok_or_else(|| {
                ContentError::new(
                    "build.learned_skills",
                    format!("missing learned skill {id}"),
                )
            })?;
            for upgrade in &skill.upgrades {
                let path = format!("build.learned_skills.{}", skill.id);
                let resolved = abilities
                    .iter_mut()
                    .find(|a| a.definition.id == upgrade.ability)
                    .ok_or_else(|| {
                        ContentError::new(
                            &path,
                            format!("upgrade requires granted ability {}", upgrade.ability),
                        )
                    })?;
                let base = self
                    .ability(&upgrade.ability)
                    .ok_or_else(|| ContentError::new(&path, "missing base ability"))?;
                apply_upgrade(base, &mut resolved.definition, upgrade, &path)?;
                resolved.upgrades.push(UpgradeContribution {
                    source: GrantSource {
                        kind: GrantKind::Learned,
                        definition: skill.id.clone(),
                        provenance: skill.provenance.clone(),
                    },
                    upgrade: upgrade.clone(),
                });
            }
        }
        Ok(ResolvedBuild {
            catalog_fingerprint: self.fingerprint(),
            abilities,
        })
    }
}
fn add_grant(
    catalog: &ContentCatalog,
    abilities: &mut Vec<ResolvedAbility>,
    id: &ContentId,
    source: GrantSource,
) -> Result<(), ContentError> {
    if let Some(existing) = abilities.iter_mut().find(|a| &a.definition.id == id) {
        existing.grants.push(source);
        return Ok(());
    }
    if abilities.len() == MAX_RESOLVED_ABILITIES {
        return Err(ContentError::new(
            "build.abilities",
            "too many distinct granted abilities (maximum 64)",
        ));
    }
    let definition = catalog
        .ability(id)
        .ok_or_else(|| ContentError::new("build.grants", format!("missing ability {id}")))?
        .clone();
    abilities.push(ResolvedAbility {
        definition,
        grants: vec![source],
        upgrades: Vec::new(),
    });
    Ok(())
}
