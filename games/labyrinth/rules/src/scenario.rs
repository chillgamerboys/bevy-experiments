//! Saved battle configuration shared by local play, co-op setup and simulations.

use crate::{
    build::{ActorBuild, CharacterBuild, InnateGrant},
    catalog::{text_field, ContentCatalog, ContentError, ContentId},
    status_definition, ActorId, ActorKind, EnemyKind, HeroClass, StatusKind, Team,
};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::BTreeSet;

/// Scenario format revision; saved files carry configuration, never peer credentials.
pub const SCENARIO_SCHEMA_VERSION: u32 = 1;
/// Maximum saved scenario input size before parsing.
pub const MAX_SCENARIO_BYTES: usize = 131_072;
/// Decision producer policy; every producer must still use legal actions and apply.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ControllerPolicy {
    /// Human commands supplied by the local/co-op session.
    Manual,
    /// Host runs the deterministic default policy.
    Ai,
    /// Session player or future gym supplies commands; no identity is stored here.
    External,
}
/// Initial condition using existing status semantics, without runtime instance IDs.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct StartingStatus {
    /// Existing typed condition.
    pub kind: StatusKind,
    /// Optional source identity; absent uses this actor itself.
    pub source: Option<ActorId>,
    /// Remaining boundaries, absent uses the definition's starting duration.
    pub remaining: Option<u8>,
}
/// Explicit actor configuration; position is order in the selected team vector.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ScenarioActor {
    /// Nonzero stable encounter identity, unique across both teams.
    pub id: ActorId,
    /// Editable name, visual preset, stats, footprint and composed build.
    pub actor: ActorBuild,
    /// Default policy hint, without any session participant identity.
    pub controller: ControllerPolicy,
    /// Initial living HP; zero starts a hero Dying, absent starts at maximum.
    pub starting_hp: Option<u16>,
    /// Ordered initial conditions, with per-actor unique kinds.
    #[serde(default)]
    pub starting_statuses: Vec<StartingStatus>,
}
/// Complete reproducible battle input. Parse/validate before replacing a lobby draft.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Scenario {
    /// Version of this file contract.
    pub schema_version: u32,
    /// Display label for saved and stock choices.
    pub name: String,
    /// Explicit deterministic combat seed.
    pub seed: u64,
    /// Hero occupants in front-to-back order, occupying at most six ranks.
    pub heroes: Vec<ScenarioActor>,
    /// Enemy occupants with the identical build/stat configuration surface.
    pub enemies: Vec<ScenarioActor>,
}
/// Authored stock choices, all editable through the same Scenario model.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum StockScenario {
    /// Existing five-actor prototype and Hauler formation.
    Prototype,
    /// Six single-rank heroes, one of each weapon.
    WeaponComparison,
    /// Greatsword against a two-rank creature and a rear opponent.
    Cleave,
    /// Medic, dying ally, and a bleeding standing ally.
    RescueStatus,
}
impl StockScenario {
    /// Stable menu order.
    pub const ALL: [Self; 4] = [
        Self::Prototype,
        Self::WeaponComparison,
        Self::Cleave,
        Self::RescueStatus,
    ];
}
impl Scenario {
    /// Validate complete rosters and initial conditions against a frozen catalog.
    pub fn validate(&self, catalog: &ContentCatalog) -> Result<(), ContentError> {
        if self.schema_version != SCENARIO_SCHEMA_VERSION {
            return Err(ContentError::new(
                "scenario.schema_version",
                "unsupported scenario schema",
            ));
        }
        text_field("scenario.name", &self.name, 128)?;
        let mut ids = BTreeSet::new();
        for (team, actors) in [(Team::Heroes, &self.heroes), (Team::Enemies, &self.enemies)] {
            if actors.is_empty() || actors.len() > crate::PARTY_SIZE {
                return Err(ContentError::new(
                    "scenario.roster",
                    "each team needs 1..6 actors",
                ));
            }
            let mut spaces = 0_usize;
            for actor in actors {
                let path = format!("scenario.actors.{}", actor.id.0);
                if actor.id.0 == 0 || !ids.insert(actor.id) {
                    return Err(ContentError::new(
                        &path,
                        "actor IDs must be nonzero and unique across both teams",
                    ));
                }
                actor
                    .actor
                    .resolve(catalog)
                    .map_err(|e| ContentError::new(format!("{path}.{}", e.path), e.message))?;
                spaces += usize::from(actor.actor.footprint);
                if actor
                    .starting_hp
                    .is_some_and(|hp| hp > actor.actor.max_hp || hp == 0 && team == Team::Enemies)
                {
                    return Err(ContentError::new(
                        format!("{path}.starting_hp"),
                        "HP exceeds maximum or starts an enemy downed",
                    ));
                }
                if actor.starting_statuses.len() > crate::MAX_STATUSES {
                    return Err(ContentError::new(
                        format!("{path}.starting_statuses"),
                        "too many conditions",
                    ));
                }
                let mut kinds = BTreeSet::new();
                for status in &actor.starting_statuses {
                    if !kinds.insert(status.kind)
                        || status.remaining.is_some_and(|n| {
                            n == 0 || n > status_definition(status.kind).duration.ticks
                        })
                    {
                        return Err(ContentError::new(
                            format!("{path}.starting_statuses"),
                            "duplicate condition or invalid remaining duration",
                        ));
                    }
                }
            }
            if spaces > crate::PARTY_SIZE {
                return Err(ContentError::new(
                    "scenario.formation",
                    "team occupies more than six ranks",
                ));
            }
        }
        if self.heroes.iter().all(|a| a.starting_hp == Some(0)) {
            return Err(ContentError::new(
                "scenario.heroes",
                "at least one hero must start standing",
            ));
        }
        for actor in self.heroes.iter().chain(&self.enemies) {
            for status in &actor.starting_statuses {
                if status.source.is_some_and(|id| !ids.contains(&id)) {
                    return Err(ContentError::new(
                        "scenario.starting_statuses.source",
                        "unknown source actor",
                    ));
                }
            }
        }
        if self.to_json()?.len() > MAX_SCENARIO_BYTES {
            return Err(ContentError::new(
                "scenario",
                "encoded scenario exceeds byte limit",
            ));
        }
        // Preserve headroom for bounded initiative, counters, status instances,
        // corpse fields and JSON field names that arise after initial preparation.
        let resolved = self
            .heroes
            .iter()
            .chain(&self.enemies)
            .map(|actor| actor.actor.resolve(catalog))
            .collect::<Result<Vec<_>, _>>()?;
        let bytes = serde_json::to_vec(&(catalog, self, resolved))
            .map_err(|e| ContentError::new("scenario.payload", e.to_string()))?;
        if bytes.len() + 131_072 > crate::MAX_COMBAT_SNAPSHOT_BYTES {
            return Err(ContentError::new(
                "scenario.payload",
                "resolved battle exceeds the 1 MiB combat budget including runtime headroom",
            ));
        }
        Ok(())
    }
    /// Parse a bounded JSON saved scenario and validate before returning it.
    pub fn from_json(source: &str, catalog: &ContentCatalog) -> Result<Self, ContentError> {
        if source.len() > MAX_SCENARIO_BYTES {
            return Err(ContentError::new(
                "scenario",
                "encoded scenario exceeds byte limit",
            ));
        }
        let scenario: Self = serde_json::from_str(source)
            .map_err(|e| ContentError::new("scenario.json", e.to_string()))?;
        scenario.validate(catalog)?;
        Ok(scenario)
    }
    /// Encode portable game configuration without filesystem I/O or peer identities.
    pub fn to_json(&self) -> Result<String, ContentError> {
        serde_json::to_string_pretty(self)
            .map_err(|e| ContentError::new("scenario.json", e.to_string()))
    }
    /// Same rules/catalog, scenario, seed and ordered commands reproduce this encounter.
    pub fn fingerprint(&self, catalog: &ContentCatalog) -> Result<String, ContentError> {
        self.validate(catalog)?;
        let bytes = serde_json::to_vec(&(catalog.fingerprint(), self))
            .map_err(|e| ContentError::new("scenario", e.to_string()))?;
        Ok(format!("{:x}", Sha256::digest(bytes)))
    }
    /// Build an editable stock encounter from the same catalog used by custom input.
    pub fn stock(
        choice: StockScenario,
        seed: u64,
        catalog: &ContentCatalog,
    ) -> Result<Self, ContentError> {
        let preset_actor = |id: u16, kind: ActorKind| -> Result<ScenarioActor, ContentError> {
            let preset = catalog
                .definition()
                .actor_presets
                .iter()
                .find(|p| p.appearance == kind)
                .ok_or_else(|| {
                    ContentError::new(
                        "stock.preset",
                        "required stock appearance preset is missing",
                    )
                })?;
            Ok(ScenarioActor {
                id: ActorId(id),
                actor: ActorBuild::from_preset(catalog, &preset.id)?,
                controller: if matches!(kind, ActorKind::Hero(_)) {
                    ControllerPolicy::Manual
                } else {
                    ControllerPolicy::Ai
                },
                starting_hp: None,
                starting_statuses: vec![],
            })
        };
        let mut heroes = crate::PROTOTYPE_HERO_ROSTER
            .into_iter()
            .enumerate()
            .map(|(i, k)| preset_actor(i as u16 + 1, ActorKind::Hero(k)))
            .collect::<Result<Vec<_>, _>>()?;
        let mut enemies = crate::PROTOTYPE_ENEMY_ROSTER
            .into_iter()
            .map(|(id, k)| preset_actor(id.0, ActorKind::Enemy(k)))
            .collect::<Result<Vec<_>, _>>()?;
        let name = match choice {
            StockScenario::Prototype => "Prototype encounter",
            StockScenario::WeaponComparison => {
                heroes.clear();
                for (i, weapon) in [
                    "greatsword",
                    "two_handed_axe",
                    "dagger",
                    "spear",
                    "bow",
                    "staff",
                ]
                .into_iter()
                .enumerate()
                {
                    let mut actor =
                        preset_actor(i as u16 + 1, ActorKind::Hero(HeroClass::Gatekeeper))?;
                    actor.actor.name = format!("{} tester", weapon.replace('_', " "));
                    actor.actor.max_hp = 40;
                    actor.actor.base_speed = 6;
                    actor.actor.footprint = 1;
                    actor.actor.build = CharacterBuild {
                        weapon: Some(ContentId::new(weapon)?),
                        ..Default::default()
                    };
                    if weapon == "dagger" {
                        actor.actor.build.learned_skills = vec![
                            ContentId::new("assassin_feint_training")?,
                            ContentId::new("assassin_bleeding_dagger")?,
                        ];
                    }
                    heroes.push(actor);
                }
                enemies = (0..6)
                    .map(|i| preset_actor(101 + i, ActorKind::Enemy(EnemyKind::AshBrute)))
                    .collect::<Result<Vec<_>, _>>()?;
                for enemy in &mut enemies {
                    enemy.actor.max_hp = 35;
                    enemy.actor.build.weapon = Some(ContentId::new("spear")?);
                }
                "Six weapon comparison"
            }
            StockScenario::Cleave => {
                heroes.truncate(1);
                if let Some(hero) = heroes.first_mut() {
                    hero.actor.build = CharacterBuild {
                        weapon: Some(ContentId::new("greatsword")?),
                        ..Default::default()
                    };
                    hero.actor.base_speed = 100;
                }
                enemies = vec![
                    preset_actor(101, ActorKind::Enemy(EnemyKind::OssuaryHauler))?,
                    preset_actor(102, ActorKind::Enemy(EnemyKind::HollowArcher))?,
                ];
                "Two-rank cleave"
            }
            StockScenario::RescueStatus => {
                heroes = vec![
                    preset_actor(1, ActorKind::Hero(HeroClass::FieldMedic))?,
                    preset_actor(2, ActorKind::Hero(HeroClass::Gatekeeper))?,
                    preset_actor(3, ActorKind::Hero(HeroClass::Knifehand))?,
                ];
                if let Some(hero) = heroes.first_mut() {
                    hero.actor.base_speed = 100;
                }
                if let Some(hero) = heroes.get_mut(1) {
                    hero.starting_hp = Some(0);
                }
                if let Some(hero) = heroes.get_mut(2) {
                    hero.starting_statuses.push(StartingStatus {
                        kind: StatusKind::Bleed,
                        source: None,
                        remaining: None,
                    });
                }
                enemies = vec![preset_actor(101, ActorKind::Enemy(EnemyKind::AshBrute))?];
                "Rescue and status boundaries"
            }
        };
        let scenario = Self {
            schema_version: SCENARIO_SCHEMA_VERSION,
            name: name.into(),
            seed,
            heroes,
            enemies,
        };
        scenario.validate(catalog)?;
        Ok(scenario)
    }
}
/// Compatibility adapter for old enums; new authored content never needs an enum arm.
#[must_use]
pub fn legacy_skill_id(skill: crate::SkillId) -> ContentId {
    let value = serde_json::to_value(skill).expect("legacy skill IDs serialize");
    let name = value.as_str().expect("legacy skill is a unit variant");
    let mut key = String::new();
    for (index, ch) in name.chars().enumerate() {
        if index > 0 && ch.is_uppercase() {
            key.push('_');
        }
        key.extend(ch.to_lowercase());
    }
    ContentId::new(key).expect("legacy identifiers are valid content keys")
}
/// Convert a trusted legacy loadout into a build with explicit innate provenance.
#[must_use]
pub fn legacy_build(skills: &[crate::SkillId]) -> CharacterBuild {
    CharacterBuild {
        innate: skills
            .iter()
            .map(|skill| InnateGrant {
                ability: legacy_skill_id(*skill),
                provenance: ContentId::new("legacy_preset").expect("static ID"),
            })
            .collect(),
        ..Default::default()
    }
}
