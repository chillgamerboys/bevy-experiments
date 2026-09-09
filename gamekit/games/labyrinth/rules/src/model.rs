//! Public domain identities, commands, validated snapshots and typed outcomes.

use std::{
    collections::{BTreeMap, BTreeSet},
    fmt,
};

use serde::{Deserialize, Serialize};

use crate::{
    skill_definition, skills_for, status_definition, RemovalReason, Stat, StatusInstance,
    StatusKind, TargetRule,
};

/// Stable character identity; hero IDs are 1..4, enemy IDs are 101..104.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct ActorId(pub u16);

/// Combat allegiance, independent of ownership or formation position.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Team {
    /// Cooperative human-controlled party.
    Heroes,
    /// Host-controlled opponents.
    Enemies,
}

/// Four original playable roles; one of each is required in this encounter.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum HeroClass {
    /// Durable front-rank fighter.
    Gatekeeper,
    /// Fast melee bleed specialist.
    Knifehand,
    /// Rear-rank ranged and positioning specialist.
    Scout,
    /// Healing, cleansing and rescue specialist.
    FieldMedic,
}

impl HeroClass {
    /// Default front-to-back formation and stable lobby choice order.
    pub const ALL: [Self; 4] = [
        Self::Gatekeeper,
        Self::Knifehand,
        Self::Scout,
        Self::FieldMedic,
    ];

    /// Human-readable original role name.
    #[must_use]
    pub const fn name(self) -> &'static str {
        match self {
            Self::Gatekeeper => "Gatekeeper",
            Self::Knifehand => "Knifehand",
            Self::Scout => "Scout",
            Self::FieldMedic => "Field Medic",
        }
    }

    /// Starting immutable HP and Speed.
    #[must_use]
    pub const fn stats(self) -> (u16, u16) {
        match self {
            Self::Gatekeeper => (32, 2),
            Self::Knifehand => (26, 5),
            Self::Scout => (22, 7),
            Self::FieldMedic => (24, 4),
        }
    }

    /// Exactly four authored skills for this role.
    #[must_use]
    pub fn skills(self) -> &'static [SkillId] {
        skills_for(ActorKind::Hero(self))
    }
}

/// The authored encounter's four original opponents.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum EnemyKind {
    /// Front bruiser.
    AshBrute,
    /// Second front bruiser.
    IronBrute,
    /// Bleed-applying skirmisher.
    WoundStalker,
    /// Rear ranged attacker.
    HollowArcher,
}

impl EnemyKind {
    /// Authored front-to-back enemy roster.
    pub const ALL: [Self; 4] = [
        Self::AshBrute,
        Self::IronBrute,
        Self::WoundStalker,
        Self::HollowArcher,
    ];

    /// Display name.
    #[must_use]
    pub const fn name(self) -> &'static str {
        match self {
            Self::AshBrute => "Ash Brute",
            Self::IronBrute => "Iron Brute",
            Self::WoundStalker => "Wound Stalker",
            Self::HollowArcher => "Hollow Archer",
        }
    }

    /// Immutable maximum HP and Speed.
    #[must_use]
    pub const fn stats(self) -> (u16, u16) {
        match self {
            Self::AshBrute => (20, 2),
            Self::IronBrute => (20, 3),
            Self::WoundStalker => (16, 5),
            Self::HollowArcher => (14, 6),
        }
    }
}

/// Authored actor content, separate from mutable HP and position.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ActorKind {
    /// A player hero.
    Hero(HeroClass),
    /// An AI enemy.
    Enemy(EnemyKind),
}

impl ActorKind {
    /// Display name, never used as identity.
    #[must_use]
    pub const fn name(self) -> &'static str {
        match self {
            Self::Hero(kind) => kind.name(),
            Self::Enemy(kind) => kind.name(),
        }
    }
    /// Fixed allegiance.
    #[must_use]
    pub const fn team(self) -> Team {
        match self {
            Self::Hero(_) => Team::Heroes,
            Self::Enemy(_) => Team::Enemies,
        }
    }
    /// Immutable base stats.
    #[must_use]
    pub const fn stats(self) -> (u16, u16) {
        match self {
            Self::Hero(kind) => kind.stats(),
            Self::Enemy(kind) => kind.stats(),
        }
    }
}

/// Typed skill catalog IDs; their order is also deterministic AI tie-breaking.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum SkillId {
    /// Gatekeeper's strong front attack.
    FrontStrike,
    /// Gatekeeper's weaker extended attack.
    LongReach,
    /// Gatekeeper's damaging displacement.
    DrivingBlow,
    /// Gatekeeper's limited self-heal.
    FieldDressing,
    /// Knifehand's bleed attack.
    BleedingCut,
    /// Knifehand's direct attack.
    DeepStrike,
    /// Knifehand's ranged fallback.
    ThrownKnife,
    /// Knifehand's self bleed cleanse.
    CleanBlade,
    /// Scout's strong rear-target attack.
    BackRankShot,
    /// Scout's flexible fallback.
    SnapShot,
    /// Scout's damaging pull.
    HookShot,
    /// Scout swaps places with an ally.
    Exchange,
    /// Medic's limited heal.
    Mend,
    /// Medic's bleed cleanse.
    Staunch,
    /// Medic's weak attack.
    StaffStrike,
    /// Medic's limited stronger rescue.
    Rally,
    /// Bruiser front attack.
    BrutalStrike,
    /// Stalker's bleeding attack.
    RaggedCut,
    /// Archer's ranged attack.
    HollowBolt,
}

/// A single action. Expected turn/player identity belongs in the app envelope.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum CombatAction {
    /// Use an owned skill on a stable character ID.
    Skill {
        /// Authored skill.
        skill: SkillId,
        /// Chosen target.
        target: ActorId,
    },
    /// Swap with one adjacent ally, including a downed hero.
    Reposition {
        /// Adjacent ally.
        ally: ActorId,
    },
    /// Restore a downed ally to 25 percent maximum HP, rounded up.
    Rescue {
        /// Downed ally at any rank.
        ally: ActorId,
    },
    /// Apply Brace until the acting character's next turn starts.
    Defend,
    /// Spend the action without another effect.
    Wait,
}

/// Explicit phase names. Public committed snapshots are at input or terminal boundaries.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum CombatPhase {
    /// Initiative is rolling.
    RoundStart,
    /// Start-boundary effects are resolving.
    TurnStart,
    /// The active character can submit exactly one action.
    AwaitingAction,
    /// End-boundary effects are resolving.
    TurnEnd,
    /// Round-end effects are resolving.
    RoundEnd,
    /// A terminal result has been committed.
    Finished,
}

/// Cooperative encounter outcome; simultaneous terminal conditions favor defeat.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum CombatOutcome {
    /// No living enemies remain.
    Victory,
    /// All heroes are down.
    Defeat,
}

/// Damage category determines whether direct modifiers apply.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum DamageKind {
    /// A skill hit, affected by Brace and outgoing modifiers.
    Direct,
    /// A periodic status tick, unaffected by direct-damage modifiers.
    Bleed,
}

/// One resolved initiative entry; speed is captured at roll time.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct InitiativeEntry {
    /// Stable actor identity.
    pub actor: ActorId,
    /// Effective Speed at round start.
    pub speed: u16,
    /// Inclusive d8 result.
    pub roll: u8,
    /// Sum of speed and roll.
    pub total: u16,
    /// Unique seeded random ordering key for otherwise exact ties.
    pub tie_breaker: u8,
    /// This entry has already been acted on or skipped.
    pub completed: bool,
}

/// Public character state. Dead enemies remain inspectable but leave formation.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ActorSnapshot {
    /// Stable encounter-local identity.
    pub id: ActorId,
    /// Immutable content identity.
    pub kind: ActorKind,
    /// Current HP; zero means downed for heroes and dead for enemies.
    pub hp: u16,
    /// Immutable maximum HP.
    pub max_hp: u16,
    /// Immutable base speed.
    pub base_speed: u16,
    /// Character-bound live effects.
    pub statuses: Vec<StatusInstance>,
    /// Number of times each limited skill has been used this encounter.
    pub skill_uses: BTreeMap<SkillId, u8>,
}

impl ActorSnapshot {
    /// Human-readable name.
    #[must_use]
    pub const fn name(&self) -> &'static str {
        self.kind.name()
    }
    /// Allegiance.
    #[must_use]
    pub const fn team(&self) -> Team {
        self.kind.team()
    }
    /// Whether this actor can act or be hit.
    #[must_use]
    pub const fn standing(&self) -> bool {
        self.hp > 0
    }
    /// Owned skill catalog.
    #[must_use]
    pub fn skills(&self) -> &'static [SkillId] {
        skills_for(self.kind)
    }
    /// Effective speed; existing initiative entries never use this retroactively.
    #[must_use]
    pub fn speed(&self) -> u16 {
        u16::try_from((i32::from(self.base_speed) + self.modifier(Stat::Speed)).clamp(0, 100))
            .expect("clamped speed fits u16")
    }
    /// Sum of additive status contributions, derived without changing base stats.
    #[must_use]
    pub fn modifier(&self, stat: Stat) -> i32 {
        self.statuses
            .iter()
            .flat_map(|instance| {
                status_definition(instance.kind)
                    .modifiers
                    .iter()
                    .map(move |modifier| (instance, modifier))
            })
            .filter(|(_, modifier)| modifier.stat == stat)
            .map(|(instance, modifier)| {
                i32::from(instance.potency) * i32::from(modifier.per_potency)
            })
            .sum()
    }
    /// Remaining uses of a limited skill; `None` means unlimited.
    #[must_use]
    pub fn remaining_uses(&self, skill: SkillId) -> Option<u8> {
        skill_definition(skill)
            .max_uses
            .map(|limit| limit.saturating_sub(self.skill_uses.get(&skill).copied().unwrap_or(0)))
    }
}

/// Sanitized, validated public combat projection; it cannot restore host RNG.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(try_from = "SnapshotWire")]
pub struct CombatSnapshot {
    /// Committed action revision.
    pub revision: u64,
    /// One-based round number.
    pub round: u32,
    /// Monotonic decision identity; skipped turns also consume an ID.
    pub turn_id: u64,
    /// Exact committed phase; reconnect never executes its entry behavior.
    pub phase: CombatPhase,
    /// Current actor, absent after completion.
    pub active_actor: Option<ActorId>,
    /// Every authored actor, including defeated enemies.
    pub actors: Vec<ActorSnapshot>,
    /// Four hero ranks, front to back; downing does not remove a rank.
    pub hero_formation: Vec<ActorId>,
    /// Living enemy ranks, compacted front to back.
    pub enemy_formation: Vec<ActorId>,
    /// Frozen current-round ordering.
    pub initiative: Vec<InitiativeEntry>,
    /// Terminal outcome, if any.
    pub outcome: Option<CombatOutcome>,
    /// Committed boundary sequence for status activation validation.
    pub boundary_sequence: u64,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct SnapshotWire {
    revision: u64,
    round: u32,
    turn_id: u64,
    phase: CombatPhase,
    active_actor: Option<ActorId>,
    actors: Vec<ActorSnapshot>,
    hero_formation: Vec<ActorId>,
    enemy_formation: Vec<ActorId>,
    initiative: Vec<InitiativeEntry>,
    outcome: Option<CombatOutcome>,
    boundary_sequence: u64,
}

impl TryFrom<SnapshotWire> for CombatSnapshot {
    type Error = RuleError;
    fn try_from(wire: SnapshotWire) -> Result<Self, Self::Error> {
        let snapshot = Self {
            revision: wire.revision,
            round: wire.round,
            turn_id: wire.turn_id,
            phase: wire.phase,
            active_actor: wire.active_actor,
            actors: wire.actors,
            hero_formation: wire.hero_formation,
            enemy_formation: wire.enemy_formation,
            initiative: wire.initiative,
            outcome: wire.outcome,
            boundary_sequence: wire.boundary_sequence,
        };
        snapshot.validate()?;
        Ok(snapshot)
    }
}

impl CombatSnapshot {
    /// Find a character without trusting a client-supplied rank.
    #[must_use]
    pub fn actor(&self, id: ActorId) -> Option<&ActorSnapshot> {
        self.actors.iter().find(|actor| actor.id == id)
    }
    /// Current one-based rank, absent for a defeated enemy.
    #[must_use]
    pub fn rank(&self, id: ActorId) -> Option<u8> {
        self.hero_formation
            .iter()
            .position(|entry| *entry == id)
            .or_else(|| self.enemy_formation.iter().position(|entry| *entry == id))
            .and_then(|index| u8::try_from(index + 1).ok())
    }
    /// Validate all trusted projection invariants, including nested derived serde data.
    pub fn validate(&self) -> Result<(), RuleError> {
        if self.round == 0
            || self.turn_id == 0
            || self.actors.len() != 8
            || self.hero_formation.len() != 4
            || self.enemy_formation.len() > 4
            || self.initiative.is_empty()
            || self.initiative.len() > 8
        {
            return Err(RuleError::InvalidState);
        }
        let mut actor_ids = BTreeSet::new();
        let mut classes = BTreeSet::new();
        let mut instances = BTreeSet::new();
        for actor in &self.actors {
            if !actor_ids.insert(actor.id)
                || (actor.max_hp, actor.base_speed) != actor.kind.stats()
                || actor.hp > actor.max_hp
                || actor.statuses.len() > 16
            {
                return Err(RuleError::InvalidState);
            }
            match actor.kind {
                ActorKind::Hero(class)
                    if (1..=4).contains(&actor.id.0) && classes.insert(class) => {}
                ActorKind::Enemy(kind)
                    if (101..=104).contains(&actor.id.0)
                        && EnemyKind::ALL.get(usize::from(actor.id.0 - 101)) == Some(&kind) => {}
                _ => return Err(RuleError::InvalidState),
            }
            if actor.skill_uses.iter().any(|(skill, used)| {
                !actor.skills().contains(skill)
                    || skill_definition(*skill)
                        .max_uses
                        .is_none_or(|limit| *used == 0 || *used > limit)
            }) {
                return Err(RuleError::InvalidState);
            }
            let mut kinds = BTreeSet::new();
            for status in &actor.statuses {
                let definition = status_definition(status.kind);
                if status.id == 0
                    || !instances.insert(status.id)
                    || !kinds.insert(status.kind)
                    || status.bearer != actor.id
                    || self.actor(status.source).is_none()
                    || status.potency != definition.potency
                    || status.remaining == 0
                    || status.remaining > definition.duration.ticks
                    || status.eligible_boundary == 0
                    || status.eligible_boundary > self.boundary_sequence.saturating_add(1)
                    || (!actor.standing()
                        && (definition.remove_on_downed || actor.team() == Team::Enemies))
                {
                    return Err(RuleError::InvalidState);
                }
            }
        }
        if classes.len() != 4 {
            return Err(RuleError::InvalidState);
        }
        let hero_ids: BTreeSet<_> = self
            .actors
            .iter()
            .filter(|actor| actor.team() == Team::Heroes)
            .map(|actor| actor.id)
            .collect();
        let enemy_ids: BTreeSet<_> = self
            .actors
            .iter()
            .filter(|actor| actor.team() == Team::Enemies && actor.standing())
            .map(|actor| actor.id)
            .collect();
        if self.hero_formation.iter().copied().collect::<BTreeSet<_>>() != hero_ids
            || self
                .enemy_formation
                .iter()
                .copied()
                .collect::<BTreeSet<_>>()
                != enemy_ids
            || self.enemy_formation.len() != enemy_ids.len()
        {
            return Err(RuleError::InvalidState);
        }
        let mut entries = BTreeSet::new();
        let mut tiebreakers = BTreeSet::new();
        let mut pending_seen = false;
        for entry in &self.initiative {
            if !actor_ids.contains(&entry.actor)
                || !entries.insert(entry.actor)
                || !(1..=8).contains(&entry.roll)
                || entry.speed > 100
                || entry.total != entry.speed + u16::from(entry.roll)
                || entry.tie_breaker >= 8
                || !tiebreakers.insert(entry.tie_breaker)
                || (pending_seen && entry.completed)
            {
                return Err(RuleError::InvalidState);
            }
            pending_seen |= !entry.completed;
        }
        if self.initiative.windows(2).any(|pair| {
            pair.first().zip(pair.get(1)).is_some_and(|(a, b)| {
                (a.total, a.speed, std::cmp::Reverse(a.tie_breaker))
                    < (b.total, b.speed, std::cmp::Reverse(b.tie_breaker))
            })
        }) {
            return Err(RuleError::InvalidState);
        }
        let heroes_up = self
            .actors
            .iter()
            .any(|actor| actor.team() == Team::Heroes && actor.standing());
        let enemies_up = !self.enemy_formation.is_empty();
        match self.outcome {
            None if self.phase == CombatPhase::AwaitingAction
                && heroes_up
                && enemies_up
                && self.active_actor.is_some_and(|id| {
                    self.actor(id).is_some_and(ActorSnapshot::standing)
                        && self
                            .initiative
                            .iter()
                            .find(|entry| !entry.completed)
                            .is_some_and(|entry| entry.actor == id)
                }) => {}
            Some(CombatOutcome::Defeat)
                if self.phase == CombatPhase::Finished
                    && self.active_actor.is_none()
                    && !heroes_up => {}
            Some(CombatOutcome::Victory)
                if self.phase == CombatPhase::Finished
                    && self.active_actor.is_none()
                    && heroes_up
                    && !enemies_up => {}
            _ => return Err(RuleError::InvalidState),
        }
        Ok(())
    }
    /// Shared legality contract for UI previews, enemy AI and the authoritative reducer.
    pub fn validate_action(&self, actor: ActorId, action: &CombatAction) -> Result<(), RuleError> {
        if self.outcome.is_some() {
            return Err(RuleError::Finished);
        }
        if self.phase != CombatPhase::AwaitingAction || self.active_actor != Some(actor) {
            return Err(RuleError::WrongActor);
        }
        let source = self.actor(actor).ok_or(RuleError::UnknownActor)?;
        if !source.standing() {
            return Err(RuleError::NotStanding);
        }
        match *action {
            CombatAction::Wait | CombatAction::Defend => Ok(()),
            CombatAction::Rescue { ally } => {
                let target = self.actor(ally).ok_or(RuleError::UnknownActor)?;
                if source.team() != Team::Heroes
                    || target.team() != Team::Heroes
                    || target.standing()
                {
                    return Err(RuleError::NotDowned);
                }
                Ok(())
            }
            CombatAction::Reposition { ally } => {
                let target = self.actor(ally).ok_or(RuleError::UnknownActor)?;
                if source.team() != target.team() || actor == ally {
                    return Err(RuleError::WrongTarget);
                }
                if self
                    .rank(actor)
                    .zip(self.rank(ally))
                    .is_none_or(|(left, right)| left.abs_diff(right) != 1)
                {
                    return Err(RuleError::NotAdjacent);
                }
                Ok(())
            }
            CombatAction::Skill { skill, target } => {
                if !source.skills().contains(&skill) {
                    return Err(RuleError::UnknownSkill);
                }
                let definition = skill_definition(skill);
                if self
                    .rank(actor)
                    .is_none_or(|rank| definition.source_ranks & (1 << (rank - 1)) == 0)
                {
                    return Err(RuleError::WrongRank);
                }
                if source.remaining_uses(skill) == Some(0) {
                    return Err(RuleError::NoUses);
                }
                let recipient = self.actor(target).ok_or(RuleError::UnknownActor)?;
                if self
                    .rank(target)
                    .is_none_or(|rank| definition.target_ranks & (1 << (rank - 1)) == 0)
                {
                    return Err(RuleError::WrongTargetRank);
                }
                let valid_target = match definition.target_rule {
                    TargetRule::EnemyStanding => {
                        source.team() != recipient.team() && recipient.standing()
                    }
                    TargetRule::AllyStanding => {
                        source.team() == recipient.team() && recipient.standing()
                    }
                    TargetRule::SelfStanding => actor == target && recipient.standing(),
                    TargetRule::OtherAlly => source.team() == recipient.team() && actor != target,
                    TargetRule::AllyDowned => {
                        source.team() == Team::Heroes
                            && recipient.team() == Team::Heroes
                            && !recipient.standing()
                    }
                };
                if valid_target {
                    Ok(())
                } else {
                    Err(RuleError::WrongTarget)
                }
            }
        }
    }
    /// Enumerate all legal actions in stable catalog/actor order.
    #[must_use]
    pub fn legal_actions(&self, actor: ActorId) -> Vec<CombatAction> {
        let Some(source) = self.actor(actor) else {
            return Vec::new();
        };
        let mut candidates = Vec::new();
        for skill in source.skills() {
            for target in &self.actors {
                candidates.push(CombatAction::Skill {
                    skill: *skill,
                    target: target.id,
                });
            }
        }
        for target in &self.actors {
            candidates.push(CombatAction::Rescue { ally: target.id });
            candidates.push(CombatAction::Reposition { ally: target.id });
        }
        candidates.extend([CombatAction::Defend, CombatAction::Wait]);
        candidates
            .into_iter()
            .filter(|action| self.validate_action(actor, action).is_ok())
            .collect()
    }
}

/// Typed, non-mutating command or state rejection.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum RuleError {
    /// Supplied hero lineup is not exactly one of each class.
    DuplicateHero,
    /// Data violates a trusted domain invariant.
    InvalidState,
    /// Character identity does not exist.
    UnknownActor,
    /// The requested actor does not own the current decision.
    WrongActor,
    /// The encounter is already terminal.
    Finished,
    /// A downed/dead actor cannot perform this action.
    NotStanding,
    /// Rescue requires a downed hero ally.
    NotDowned,
    /// The skill belongs to another actor kind.
    UnknownSkill,
    /// The acting rank cannot use this skill.
    WrongRank,
    /// Target rank is outside the skill reach.
    WrongTargetRank,
    /// Target team, identity or standing state does not match the skill.
    WrongTarget,
    /// Reposition requires an adjacent ally.
    NotAdjacent,
    /// Encounter-limited skill has no uses remaining.
    NoUses,
    /// Bounded status/automatic-effect budget would be exceeded.
    WorkLimit,
    /// A monotonic counter cannot advance without wrapping.
    CounterExhausted,
}

impl fmt::Display for RuleError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::DuplicateHero => "Choose exactly one of each hero",
            Self::InvalidState => "Invalid combat state",
            Self::UnknownActor => "Unknown character",
            Self::WrongActor => "It is not this character's turn",
            Self::Finished => "The encounter has ended",
            Self::NotStanding => "This character is down",
            Self::NotDowned => "Choose a downed hero ally",
            Self::UnknownSkill => "This character does not own that skill",
            Self::WrongRank => "Move to an allowed source rank",
            Self::WrongTargetRank => "That target rank is out of reach",
            Self::WrongTarget => "That target is not eligible",
            Self::NotAdjacent => "Choose an adjacent ally",
            Self::NoUses => "No uses remain this encounter",
            Self::WorkLimit => "Combat effect budget exceeded",
            Self::CounterExhausted => "Combat counter exhausted",
        })
    }
}
impl std::error::Error for RuleError {}

/// Ordered public event, independent of networking and presentation timing.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CombatEvent {
    /// Monotonic encounter-local event ID, suitable for animation deduplication.
    pub id: u64,
    /// Domain outcome.
    pub kind: CombatEventKind,
}

/// Typed combat outcomes; clients need not infer gameplay from formatted strings.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum CombatEventKind {
    /// Initiative has been rolled for a new round.
    RoundStarted {
        /// One-based round.
        round: u32,
    },
    /// An actor's turn boundary has started, before any status ticks.
    TurnStarted {
        /// Actor starting.
        actor: ActorId,
        /// Monotonic decision ID.
        turn_id: u64,
    },
    /// A dead/downed entry consumed its initiative slot.
    TurnSkipped {
        /// Actor skipped.
        actor: ActorId,
    },
    /// One complete action was accepted.
    Action {
        /// Acting character.
        actor: ActorId,
        /// Applied action.
        action: CombatAction,
    },
    /// HP damage actually dealt after clamps/modifiers.
    Damage {
        /// Source actor, possibly already dead for a persistent status.
        source: ActorId,
        /// Recipient.
        target: ActorId,
        /// Actual HP removed.
        amount: u16,
        /// Damage classification.
        kind: DamageKind,
    },
    /// HP restored to a standing character.
    Healed {
        /// Source.
        source: ActorId,
        /// Recipient.
        target: ActorId,
        /// Actual HP restored.
        amount: u16,
    },
    /// A hero reached zero HP.
    Downed {
        /// Downed hero.
        actor: ActorId,
    },
    /// An enemy died and left its formation.
    Defeated {
        /// Defeated enemy.
        actor: ActorId,
    },
    /// A downed hero was rescued without insertion into initiative.
    Rescued {
        /// Rescuer.
        source: ActorId,
        /// Rescued hero.
        actor: ActorId,
        /// Restored HP.
        hp: u16,
    },
    /// An actor's rank changed; its identity and statuses did not.
    Moved {
        /// Moved character.
        actor: ActorId,
        /// New one-based rank.
        rank: u8,
    },
    /// An instance was newly applied.
    StatusApplied {
        /// Public instance.
        instance: StatusInstance,
    },
    /// An existing instance was refreshed, not stacked.
    StatusRefreshed {
        /// Refreshed public instance.
        instance: StatusInstance,
    },
    /// A status's boundary effect fired.
    StatusTriggered {
        /// Bearer.
        actor: ActorId,
        /// Instance identity.
        instance_id: u64,
        /// Catalog identity.
        kind: StatusKind,
    },
    /// An instance stopped applying.
    StatusRemoved {
        /// Bearer.
        actor: ActorId,
        /// Instance identity.
        instance_id: u64,
        /// Catalog identity.
        kind: StatusKind,
        /// Exact removal cause.
        reason: RemovalReason,
    },
    /// Encounter completion.
    Finished {
        /// Terminal result.
        outcome: CombatOutcome,
    },
}

impl fmt::Display for CombatEvent {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match &self.kind {
            CombatEventKind::RoundStarted { round } => {
                write!(f, "Round {round}: initiative rolled")
            }
            CombatEventKind::TurnStarted { actor, .. } => {
                write!(f, "Character {} starts its turn", actor.0)
            }
            CombatEventKind::TurnSkipped { actor } => write!(f, "Character {} cannot act", actor.0),
            CombatEventKind::Action { actor, action } => write!(
                f,
                "Character {}: {}",
                actor.0,
                match action {
                    CombatAction::Skill { skill, .. } => skill_definition(*skill).name,
                    CombatAction::Reposition { .. } => "Reposition",
                    CombatAction::Rescue { .. } => "Rescue",
                    CombatAction::Defend => "Defend",
                    CombatAction::Wait => "Wait",
                }
            ),
            CombatEventKind::Damage {
                target,
                amount,
                kind,
                ..
            } => write!(
                f,
                "Character {} takes {amount} {}damage",
                target.0,
                if *kind == DamageKind::Bleed {
                    "bleed "
                } else {
                    ""
                }
            ),
            CombatEventKind::Healed { target, amount, .. } => {
                write!(f, "Character {} recovers {amount} HP", target.0)
            }
            CombatEventKind::Downed { actor } => write!(f, "Character {} is downed", actor.0),
            CombatEventKind::Defeated { actor } => write!(f, "Character {} is defeated", actor.0),
            CombatEventKind::Rescued { actor, hp, .. } => {
                write!(f, "Character {} is rescued with {hp} HP", actor.0)
            }
            CombatEventKind::Moved { actor, rank } => {
                write!(f, "Character {} moves to rank {rank}", actor.0)
            }
            CombatEventKind::StatusApplied { instance } => write!(
                f,
                "Character {} gains {}",
                instance.bearer.0,
                status_definition(instance.kind).name
            ),
            CombatEventKind::StatusRefreshed { instance } => write!(
                f,
                "Character {}: {} refreshed",
                instance.bearer.0,
                status_definition(instance.kind).name
            ),
            CombatEventKind::StatusTriggered { actor, kind, .. } => write!(
                f,
                "Character {}: {} triggers",
                actor.0,
                status_definition(*kind).name
            ),
            CombatEventKind::StatusRemoved {
                actor,
                kind,
                reason,
                ..
            } => write!(
                f,
                "Character {}: {} removed ({reason:?})",
                actor.0,
                status_definition(*kind).name
            ),
            CombatEventKind::Finished { outcome } => write!(f, "{outcome:?}"),
        }
    }
}
