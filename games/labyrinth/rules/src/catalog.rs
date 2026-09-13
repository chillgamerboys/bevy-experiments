//! Versioned game-owned content. Parsing and validation perform no filesystem I/O.

use crate::build::{CharacterBuild, ResolvedBuild};
use crate::{ActorKind, Effect, TargetRule};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::{collections::BTreeSet, fmt};

/// Supported authored catalog schema (effect semantics also require rules identity).
pub const CATALOG_SCHEMA_VERSION: u32 = 1;
/// Maximum encoded catalog accepted before parsing.
pub const MAX_CATALOG_BYTES: usize = 1_048_576;
/// Maximum definitions per category; unrelated to the number of UI shortcuts.
pub const MAX_CATALOG_ENTRIES: usize = 256;
/// Maximum distinct active abilities on one resolved actor.
pub const MAX_RESOLVED_ABILITIES: usize = 64;
/// Maximum ordered effects on one resolved ability.
pub const MAX_ABILITY_EFFECTS: usize = 16;
/// Maximum configured maximum HP and authored direct damage/healing.
pub const MAX_CONTENT_POWER: u16 = 10_000;

/// Stable lowercase ASCII content key; independent of presentation labels.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(try_from = "String", into = "String")]
pub struct ContentId(String);
impl ContentId {
    /// Validate a 1–64 byte key containing lowercase letters, digits, `_`, `-`, `.`.
    pub fn new(value: impl Into<String>) -> Result<Self, ContentError> {
        let value = value.into();
        if value.is_empty()
            || value.len() > 64
            || !value
                .bytes()
                .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b"_-.".contains(&b))
        {
            return Err(ContentError::new(
                "id",
                "expected 1–64 lowercase ASCII letters/digits or _-.",
            ));
        }
        Ok(Self(value))
    }
    /// Stable key as text.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}
impl TryFrom<String> for ContentId {
    type Error = ContentError;
    fn try_from(value: String) -> Result<Self, Self::Error> {
        Self::new(value)
    }
}
impl From<ContentId> for String {
    fn from(value: ContentId) -> Self {
        value.0
    }
}
impl fmt::Display for ContentId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0.fmt(f)
    }
}

/// Useful source/field context for malformed content or incompatible builds.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ContentError {
    /// Definition and field path where validation failed.
    pub path: String,
    /// Human-readable failure reason.
    pub message: String,
}
impl ContentError {
    pub(crate) fn new(path: impl Into<String>, message: impl Into<String>) -> Self {
        Self {
            path: path.into(),
            message: message.into(),
        }
    }
}
impl fmt::Display for ContentError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}: {}", self.path, self.message)
    }
}
impl std::error::Error for ContentError {}

/// Behavior category is independent of where an ability was granted.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum AbilityBehavior {
    /// Explicitly selected combat action. Passive/reaction execution is deferred.
    #[default]
    Active,
}
/// Target collection shape; resolved targets must be captured before effects.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum TargetPattern {
    /// One selected eligible actor.
    #[default]
    Single,
    /// Distinct eligible occupants of ranks 1–2; a two-rank actor is hit once.
    FrontPair,
}
/// Fully owned ability definition; adding known effects needs no ability enum arm.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AbilityDefinition {
    /// Stable key.
    pub id: ContentId,
    /// Display label.
    pub name: String,
    /// Authored explanation; consumers should also describe resolved effects.
    pub description: String,
    /// Active versus future passive behavior, never inferred from provenance.
    #[serde(default)]
    pub behavior: AbilityBehavior,
    /// Allowed source ranks, bit zero is rank one.
    pub source_ranks: u8,
    /// Allowed target ranks, using the same six-bit mask.
    pub target_ranks: u8,
    /// Allegiance and life-state eligibility.
    pub target_rule: TargetRule,
    /// Target collection shape.
    #[serde(default)]
    pub target_pattern: TargetPattern,
    /// Per-actor encounter uses; absent means unlimited.
    pub max_uses: Option<u8>,
    /// Explicit execution order, using tested Rust primitives.
    pub effects: Vec<Effect>,
}
impl AbilityDefinition {
    /// Whether one-based source rank is in this move's mask.
    #[must_use]
    pub fn allows_source_rank(&self, rank: u8) -> bool {
        rank > 0 && rank <= 6 && self.source_ranks & (1 << (rank - 1)) != 0
    }
    /// Whether one-based target rank is in this move's mask.
    #[must_use]
    pub fn allows_target_rank(&self, rank: u8) -> bool {
        rank > 0 && rank <= 6 && self.target_ranks & (1 << (rank - 1)) != 0
    }
}
/// Descriptive handedness only; this slice has one weapon and no inventory slots.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Handedness {
    /// One-handed weapon.
    One,
    /// Two-handed weapon.
    Two,
}
/// Weapon grants base moves with its own provenance.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct WeaponDefinition {
    /// Stable item key.
    pub id: ContentId,
    /// Display label.
    pub name: String,
    /// Item explanation.
    pub description: String,
    /// Metadata without imposing inventory policy.
    pub handedness: Handedness,
    /// Ordered ability grants.
    pub grants: Vec<ContentId>,
}
/// Explicit additive upgrade operations. No replacements or recursive skill links.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum UpgradeOperation {
    /// Add power to an existing Damage primitive (fails for another effect kind).
    AddDamage {
        /// Zero-based base effect index; upgrades may not address appended effects.
        effect_index: usize,
        /// Positive additive damage.
        amount: u16,
    },
    /// Append an independently specified effect after the base effects.
    AppendEffect(Effect),
    /// Union this six-rank mask into allowed acting ranks.
    ExtendSourceRanks(u8),
    /// Union this six-rank mask into allowed target ranks.
    ExtendTargetRanks(u8),
}
/// A learned contribution to a separately granted ability.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AbilityUpgrade {
    /// The ability must already be granted in the completed build.
    pub ability: ContentId,
    /// Ordered contributions applied once per selected learned skill.
    pub operations: Vec<UpgradeOperation>,
}
/// A learned skill can add moves, upgrade moves, or do both.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct LearnedSkillDefinition {
    /// Stable key.
    pub id: ContentId,
    /// Display label.
    pub name: String,
    /// Explanation of the learned skill.
    pub description: String,
    /// Descriptive source identity, e.g. assassin; not a class restriction/tree.
    pub provenance: ContentId,
    /// Ordered active grants.
    #[serde(default)]
    pub grants: Vec<ContentId>,
    /// Explicit upgrades to moves granted elsewhere or by this skill.
    #[serde(default)]
    pub upgrades: Vec<AbilityUpgrade>,
}
/// Actor preset supplies editable defaults, never authority over resolved stats.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ActorPreset {
    /// Stable key.
    pub id: ContentId,
    /// Default actor label.
    pub name: String,
    /// Existing appearance preset; does not constrain custom stats or footprint.
    pub appearance: ActorKind,
    /// Default maximum HP.
    pub max_hp: u16,
    /// Default initiative statistic, 0–100.
    pub base_speed: u16,
    /// Default occupied ranks, 1–6.
    pub footprint: u8,
    /// Default grants, learned skills and optional weapon.
    pub build: CharacterBuild,
}
/// Untrusted authored input. Validate with [`ContentCatalog::new`] before use.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CatalogDefinition {
    /// Parser/schema contract.
    pub schema_version: u32,
    /// Author-controlled descriptive content revision; hash verifies actual data.
    pub revision: String,
    /// Active ability definitions.
    #[serde(default)]
    pub abilities: Vec<AbilityDefinition>,
    /// One-weapon choices.
    #[serde(default)]
    pub weapons: Vec<WeaponDefinition>,
    /// Learned grant/upgrade definitions.
    #[serde(default)]
    pub learned_skills: Vec<LearnedSkillDefinition>,
    /// Defaults for either team.
    #[serde(default)]
    pub actor_presets: Vec<ActorPreset>,
}
/// Immutable validated catalog, safe to retain for an encounter.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(try_from = "CatalogDefinition", into = "CatalogDefinition")]
pub struct ContentCatalog {
    definition: CatalogDefinition,
    fingerprint: String,
}
impl TryFrom<CatalogDefinition> for ContentCatalog {
    type Error = ContentError;
    fn try_from(value: CatalogDefinition) -> Result<Self, Self::Error> {
        Self::new(value)
    }
}
impl From<ContentCatalog> for CatalogDefinition {
    fn from(value: ContentCatalog) -> Self {
        value.definition
    }
}
impl ContentCatalog {
    /// Parse bounded TOML then validate all definitions and preset builds.
    pub fn from_toml(source: &str) -> Result<Self, ContentError> {
        if source.len() > MAX_CATALOG_BYTES {
            return Err(ContentError::new(
                "catalog",
                "encoded catalog exceeds byte limit",
            ));
        }
        let raw =
            toml::from_str(source).map_err(|e| ContentError::new("catalog.toml", e.to_string()))?;
        Self::new(raw)
    }
    /// Validate references, effects, bounds and upgrades; canonicalize category order.
    pub fn new(mut definition: CatalogDefinition) -> Result<Self, ContentError> {
        if definition.schema_version != CATALOG_SCHEMA_VERSION {
            return Err(ContentError::new(
                "schema_version",
                "unsupported catalog schema",
            ));
        }
        text_field("revision", &definition.revision, 128)?;
        for (name, ids) in [
            (
                "abilities",
                definition
                    .abilities
                    .iter()
                    .map(|x| &x.id)
                    .collect::<Vec<_>>(),
            ),
            (
                "weapons",
                definition.weapons.iter().map(|x| &x.id).collect(),
            ),
            (
                "learned_skills",
                definition.learned_skills.iter().map(|x| &x.id).collect(),
            ),
            (
                "actor_presets",
                definition.actor_presets.iter().map(|x| &x.id).collect(),
            ),
        ] {
            unique_ids(name, &ids, MAX_CATALOG_ENTRIES)?;
        }
        definition.abilities.sort_by(|a, b| a.id.cmp(&b.id));
        definition.weapons.sort_by(|a, b| a.id.cmp(&b.id));
        definition.learned_skills.sort_by(|a, b| a.id.cmp(&b.id));
        definition.actor_presets.sort_by(|a, b| a.id.cmp(&b.id));
        let bytes = serde_json::to_vec(&(
            "labyrinth-build-v1",
            crate::rules_fingerprint(),
            &definition,
        ))
        .map_err(|e| ContentError::new("catalog", e.to_string()))?;
        let fingerprint = format!("{:x}", Sha256::digest(bytes));
        let catalog = Self {
            definition,
            fingerprint,
        };
        for ability in &catalog.definition.abilities {
            validate_ability(ability)?;
        }
        for weapon in &catalog.definition.weapons {
            let path = format!("weapons.{}", weapon.id);
            text_field(&format!("{path}.name"), &weapon.name, 128)?;
            text_field(&format!("{path}.description"), &weapon.description, 2048)?;
            catalog.validate_grants(&path, &weapon.grants)?;
        }
        for skill in &catalog.definition.learned_skills {
            let path = format!("learned_skills.{}", skill.id);
            text_field(&format!("{path}.name"), &skill.name, 128)?;
            text_field(&format!("{path}.description"), &skill.description, 2048)?;
            catalog.validate_grants(&path, &skill.grants)?;
            unique_ids(
                &format!("{path}.upgrades"),
                &skill
                    .upgrades
                    .iter()
                    .map(|u| &u.ability)
                    .collect::<Vec<_>>(),
                MAX_RESOLVED_ABILITIES,
            )?;
            for upgrade in &skill.upgrades {
                let base = catalog.ability(&upgrade.ability).ok_or_else(|| {
                    ContentError::new(&path, format!("missing ability {}", upgrade.ability))
                })?;
                let mut effective = base.clone();
                apply_upgrade(base, &mut effective, upgrade, &path)?;
            }
        }
        for preset in &catalog.definition.actor_presets {
            let path = format!("actor_presets.{}", preset.id);
            text_field(&format!("{path}.name"), &preset.name, 128)?;
            validate_stats(&path, preset.max_hp, preset.base_speed, preset.footprint)?;
            catalog
                .resolve_build(&preset.build)
                .map_err(|e| ContentError::new(format!("{path}.{}", e.path), e.message))?;
        }
        // Also bounds programmatically constructed/JSON-deserialized catalogs.
        let bytes = serde_json::to_vec(&catalog.definition)
            .map_err(|e| ContentError::new("catalog", e.to_string()))?;
        if bytes.len() > MAX_CATALOG_BYTES {
            return Err(ContentError::new(
                "catalog",
                "canonical catalog exceeds byte limit",
            ));
        }
        Ok(catalog)
    }
    /// Embedded authored catalog including legacy definitions and starter weapons.
    pub fn builtin() -> Result<Self, ContentError> {
        Self::from_toml(include_str!("../content/catalog.toml"))
    }
    /// Canonically sorted, immutable definitions for editors and scenario freezing.
    #[must_use]
    pub fn definition(&self) -> &CatalogDefinition {
        &self.definition
    }
    /// Find an ability by stable key.
    #[must_use]
    pub fn ability(&self, id: &ContentId) -> Option<&AbilityDefinition> {
        self.definition.abilities.iter().find(|a| &a.id == id)
    }
    /// Find a weapon by stable key.
    #[must_use]
    pub fn weapon(&self, id: &ContentId) -> Option<&WeaponDefinition> {
        self.definition.weapons.iter().find(|a| &a.id == id)
    }
    /// Find a learned skill by stable key.
    #[must_use]
    pub fn learned_skill(&self, id: &ContentId) -> Option<&LearnedSkillDefinition> {
        self.definition.learned_skills.iter().find(|a| &a.id == id)
    }
    /// Find editable actor defaults by stable key.
    #[must_use]
    pub fn actor_preset(&self, id: &ContentId) -> Option<&ActorPreset> {
        self.definition.actor_presets.iter().find(|a| &a.id == id)
    }
    /// Find a default actor preset for an existing visual archetype.
    #[must_use]
    pub fn preset_for_appearance(&self, appearance: ActorKind) -> Option<&ActorPreset> {
        self.definition
            .actor_presets
            .iter()
            .find(|preset| preset.appearance == appearance)
    }
    /// SHA-256 over canonical data, schema and resolver interpretation revision.
    #[must_use]
    pub fn fingerprint(&self) -> String {
        self.fingerprint.clone()
    }
    /// Verify an untrusted resolved view by recomputing all grants/contributions.
    pub fn validate_resolved(
        &self,
        build: &CharacterBuild,
        resolved: &ResolvedBuild,
    ) -> Result<(), ContentError> {
        if self.resolve_build(build)? != *resolved {
            return Err(ContentError::new(
                "resolved_build",
                "does not match catalog and build",
            ));
        }
        Ok(())
    }
    fn validate_grants(&self, path: &str, grants: &[ContentId]) -> Result<(), ContentError> {
        unique_ids(
            &format!("{path}.grants"),
            &grants.iter().collect::<Vec<_>>(),
            MAX_RESOLVED_ABILITIES,
        )?;
        for grant in grants {
            if self.ability(grant).is_none() {
                return Err(ContentError::new(
                    format!("{path}.grants"),
                    format!("missing ability {grant}"),
                ));
            }
        }
        Ok(())
    }
}
pub(crate) fn unique_ids(path: &str, ids: &[&ContentId], max: usize) -> Result<(), ContentError> {
    if ids.len() > max {
        return Err(ContentError::new(path, format!("exceeds {max} entries")));
    }
    let mut seen = BTreeSet::new();
    for id in ids {
        if !seen.insert(*id) {
            return Err(ContentError::new(path, format!("duplicate id {id}")));
        }
    }
    Ok(())
}
pub(crate) fn text_field(path: &str, value: &str, max: usize) -> Result<(), ContentError> {
    if value.trim().is_empty()
        || value.len() > max
        || value.chars().any(|c| c.is_control() && c != '\n')
    {
        return Err(ContentError::new(
            path,
            format!("expected nonempty text up to {max} bytes without control characters"),
        ));
    }
    Ok(())
}
pub(crate) fn validate_stats(
    path: &str,
    hp: u16,
    speed: u16,
    footprint: u8,
) -> Result<(), ContentError> {
    if hp == 0 || hp > MAX_CONTENT_POWER {
        return Err(ContentError::new(
            format!("{path}.max_hp"),
            "expected 1..10000",
        ));
    }
    if speed > 100 {
        return Err(ContentError::new(
            format!("{path}.base_speed"),
            "expected 0..100",
        ));
    }
    if footprint == 0 || usize::from(footprint) > crate::PARTY_SIZE {
        return Err(ContentError::new(
            format!("{path}.footprint"),
            "expected 1..6 occupied ranks",
        ));
    }
    Ok(())
}
fn rank_mask(path: &str, mask: u8) -> Result<(), ContentError> {
    if mask == 0 || mask & !0b11_1111 != 0 {
        return Err(ContentError::new(
            path,
            "expected nonempty six-bit rank mask",
        ));
    }
    Ok(())
}
fn validate_effect(path: &str, effect: &Effect) -> Result<(), ContentError> {
    let valid = match effect {
        Effect::Damage(n) | Effect::Heal(n) => *n > 0 && *n <= MAX_CONTENT_POWER,
        Effect::Move(n) => *n != 0 && n.unsigned_abs() < crate::PARTY_SIZE as u8,
        Effect::Rescue(n) => *n > 0 && *n <= 100,
        // StatusDamage needs status-instance potency; it is not an active move primitive.
        Effect::StatusDamage(_) => false,
        Effect::ApplyStatus(_) | Effect::Cleanse(_) | Effect::SwapWithSource => true,
    };
    if !valid {
        return Err(ContentError::new(
            path,
            "invalid value or effect unsupported for active abilities",
        ));
    }
    Ok(())
}
pub(crate) fn validate_ability(ability: &AbilityDefinition) -> Result<(), ContentError> {
    let path = format!("abilities.{}", ability.id);
    text_field(&format!("{path}.name"), &ability.name, 128)?;
    text_field(&format!("{path}.description"), &ability.description, 2048)?;
    rank_mask(&format!("{path}.source_ranks"), ability.source_ranks)?;
    rank_mask(&format!("{path}.target_ranks"), ability.target_ranks)?;
    if ability.max_uses == Some(0) {
        return Err(ContentError::new(
            format!("{path}.max_uses"),
            "must be positive or absent for unlimited",
        ));
    }
    if ability.effects.is_empty() || ability.effects.len() > MAX_ABILITY_EFFECTS {
        return Err(ContentError::new(
            format!("{path}.effects"),
            "expected 1..16 ordered effects",
        ));
    }
    if ability.target_pattern == TargetPattern::FrontPair
        && (ability.target_rule != TargetRule::EnemyStanding
            || ability.target_ranks != 3
            || ability
                .effects
                .iter()
                .any(|e| matches!(e, Effect::SwapWithSource | Effect::Rescue(_))))
    {
        return Err(ContentError::new(
            format!("{path}.target_pattern"),
            "FrontPair requires enemy ranks 1–2 and compatible effects",
        ));
    }
    for (index, effect) in ability.effects.iter().enumerate() {
        validate_effect(&format!("{path}.effects[{index}]"), effect)?;
        if matches!(effect, Effect::Rescue(_)) && ability.target_rule != TargetRule::AllyDowned
            || matches!(effect, Effect::SwapWithSource)
                && ability.target_rule != TargetRule::OtherAlly
        {
            return Err(ContentError::new(
                format!("{path}.effects[{index}]"),
                "effect incompatible with target rule",
            ));
        }
    }
    Ok(())
}
pub(crate) fn apply_upgrade(
    base: &AbilityDefinition,
    effective: &mut AbilityDefinition,
    upgrade: &AbilityUpgrade,
    path: &str,
) -> Result<(), ContentError> {
    if upgrade.operations.is_empty() || upgrade.operations.len() > MAX_ABILITY_EFFECTS {
        return Err(ContentError::new(path, "upgrade requires 1..16 operations"));
    }
    for operation in &upgrade.operations {
        match operation {
            UpgradeOperation::AddDamage {
                effect_index,
                amount,
            } => {
                if *amount == 0
                    || !matches!(base.effects.get(*effect_index), Some(Effect::Damage(_)))
                {
                    return Err(ContentError::new(
                        path,
                        "damage upgrade requires positive amount and base Damage effect index",
                    ));
                }
                let Some(Effect::Damage(power)) = effective.effects.get_mut(*effect_index) else {
                    return Err(ContentError::new(path, "incompatible damage upgrade"));
                };
                *power = power
                    .checked_add(*amount)
                    .filter(|n| *n <= MAX_CONTENT_POWER)
                    .ok_or_else(|| {
                        ContentError::new(path, "combined damage exceeds content power limit")
                    })?;
            }
            UpgradeOperation::AppendEffect(effect) => {
                validate_effect(path, effect)?;
                effective.effects.push(*effect);
            }
            UpgradeOperation::ExtendSourceRanks(mask) => {
                rank_mask(path, *mask)?;
                effective.source_ranks |= mask;
            }
            UpgradeOperation::ExtendTargetRanks(mask) => {
                rank_mask(path, *mask)?;
                effective.target_ranks |= mask;
            }
        }
    }
    validate_ability(effective)
        .map_err(|e| ContentError::new(format!("{path}.{}", e.path), e.message))
}
