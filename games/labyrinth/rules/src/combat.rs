//! Atomic reducer and bounded automatic phase resolution.

use std::collections::{BTreeMap, BTreeSet};

use crate::{
    skill_definition, status_definition, AbilityLoadout, ActorId, ActorKind, ActorSnapshot,
    Boundary, CombatAction, CombatEvent, CombatEventKind, CombatPhase, CombatSnapshot, Effect,
    HeroClass, HeroSetup, InitiativeEntry, RemovalReason, RuleError, Team, DEFAULT_ENEMY_IDS,
    DEFAULT_ENEMY_ROSTER, MAX_ACTORS, MAX_COMBAT_WORK, PARTY_SIZE,
};

#[cfg(test)]
use crate::{CombatOutcome, DamageKind, Stat, StatusInstance, StatusKind, StatusTag};

const MAX_WORK: usize = MAX_COMBAT_WORK;

/// Host-owned deterministic combat authority, independent of players and transport.
///
/// The pinned PRNG is SplitMix64 with rejection-sampled bounded integers. Every
/// mutation occurs on a private candidate and commits only if the entire command,
/// its ordered effects and immediate phase work succeed. Clients receive snapshots
/// and outcomes, never this RNG state. Host-process restoration is not supported.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Combat {
    state: CombatSnapshot,
    rng: Rng,
    cursor: usize,
    next_event: u64,
    next_status: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct Rng(u64);

impl Rng {
    fn next(&mut self) -> u64 {
        self.0 = self.0.wrapping_add(0x9e37_79b9_7f4a_7c15);
        let mut value = self.0;
        value = (value ^ (value >> 30)).wrapping_mul(0xbf58_476d_1ce4_e5b9);
        value = (value ^ (value >> 27)).wrapping_mul(0x94d0_49bb_1331_11eb);
        value ^ (value >> 31)
    }

    fn bounded(&mut self, bound: u64, work: &mut Work) -> Result<u64, RuleError> {
        if bound == 0 {
            return Err(RuleError::InvalidState);
        }
        let threshold = bound.wrapping_neg() % bound;
        loop {
            work.spend()?;
            let value = self.next();
            if value >= threshold {
                return Ok(value % bound);
            }
        }
    }
}

pub(super) struct Work(pub(super) usize);
impl Work {
    pub(super) fn spend(&mut self) -> Result<(), RuleError> {
        self.0 = self.0.checked_sub(1).ok_or(RuleError::WorkLimit)?;
        Ok(())
    }
}

impl Combat {
    fn resolver(&mut self) -> crate::resolve::EffectResolver<'_> {
        crate::resolve::EffectResolver {
            state: &mut self.state,
            next_event: &mut self.next_event,
            next_status: &mut self.next_status,
            damage: None,
            movement: None,
        }
    }

    /// Begin the six-versus-six encounter with convenience IDs 1..6.
    /// The supplied array is front-to-back formation, not a unique-class catalog.
    /// Repeated classes receive independent loadouts, HP, uses, and statuses.
    pub fn new(seed: u64, heroes: [HeroClass; PARTY_SIZE]) -> Result<Self, RuleError> {
        let mut next_id = 0_u16;
        Self::with_heroes(
            seed,
            heroes.map(|class| {
                next_id += 1;
                HeroSetup::preset(ActorId(next_id), class)
            }),
        )
    }

    /// Begins the encounter from game-owned character identities and loadouts.
    ///
    /// Array order alone determines initial formation; class and numeric ID do
    /// not sort or move actors. IDs must be nonzero, unique, and distinct from
    /// the authored enemy IDs. No player IDs, items, or skill trees enter rules.
    pub fn with_heroes(seed: u64, heroes: [HeroSetup; PARTY_SIZE]) -> Result<Self, RuleError> {
        Self::with_rosters(
            seed,
            heroes.to_vec(),
            DEFAULT_ENEMY_IDS
                .into_iter()
                .zip(DEFAULT_ENEMY_ROSTER)
                .collect(),
        )
    }

    /// Prototype encounter: a two-space Hauler followed by four smaller enemies.
    pub fn with_party(seed: u64, heroes: Vec<HeroSetup>) -> Result<Self, RuleError> {
        Self::with_rosters(seed, heroes, crate::PROTOTYPE_ENEMY_ROSTER.to_vec())
    }

    /// Trusted encounter composition; either team may use variable-width actors.
    /// Roster order is front to back, each identity appears once, maximum six spaces.
    pub fn with_rosters(
        seed: u64,
        heroes: Vec<HeroSetup>,
        enemies: Vec<(ActorId, crate::EnemyKind)>,
    ) -> Result<Self, RuleError> {
        let hero_spaces: usize = heroes
            .iter()
            .map(|h| usize::from(ActorKind::Hero(h.class).footprint()))
            .sum();
        let enemy_spaces: usize = enemies
            .iter()
            .map(|(_, k)| usize::from(ActorKind::Enemy(*k).footprint()))
            .sum();
        if hero_spaces == 0
            || hero_spaces > PARTY_SIZE
            || enemy_spaces == 0
            || enemy_spaces > PARTY_SIZE
        {
            return Err(RuleError::InvalidState);
        }
        let mut ids = BTreeSet::new();
        for hero in &heroes {
            if hero.id.0 == 0 || DEFAULT_ENEMY_IDS.contains(&hero.id) {
                return Err(RuleError::InvalidActorId);
            }
            if !ids.insert(hero.id) {
                return Err(RuleError::DuplicateActor);
            }
        }
        for (id, _) in &enemies {
            if id.0 == 0 {
                return Err(RuleError::InvalidActorId);
            }
            if !ids.insert(*id) {
                return Err(RuleError::DuplicateActor);
            }
        }
        let enemy_formation = enemies.iter().map(|(id, _)| *id).collect();
        let hero_formation = heroes.iter().map(|hero| hero.id).collect();
        let mut actors = Vec::with_capacity(MAX_ACTORS);
        for (id, kind, abilities) in heroes
            .into_iter()
            .map(|hero| (hero.id, ActorKind::Hero(hero.class), hero.abilities))
            .chain(enemies.into_iter().map(|(id, kind)| {
                let kind = ActorKind::Enemy(kind);
                (
                    id,
                    kind,
                    AbilityLoadout::new(crate::skills_for(kind).iter().copied())
                        .expect("authored enemy presets are bounded and unique"),
                )
            }))
        {
            let (max_hp, base_speed) = kind.stats();
            actors.push(ActorSnapshot {
                id,
                kind,
                hp: max_hp,
                life: crate::LifeState::Alive,
                max_hp,
                base_speed,
                abilities,
                statuses: Vec::new(),
                skill_uses: BTreeMap::new(),
            });
        }
        let state = CombatSnapshot {
            revision: 0,
            round: 0,
            turn_id: 0,
            phase: CombatPhase::RoundStart,
            active_actor: None,
            actors,
            hero_formation,
            enemy_formation,
            initiative: Vec::new(),
            outcome: None,
            boundary_sequence: 0,
        };
        let mut combat = Self {
            state,
            rng: Rng(seed),
            cursor: 0,
            next_event: 1,
            next_status: 1,
        };
        let mut events = Vec::new();
        let mut work = Work(MAX_WORK);
        combat.start_round(&mut events, &mut work)?;
        combat.seek_decision(&mut events, &mut work)?;
        combat.state.validate()?;
        Ok(combat)
    }

    /// Public full snapshot containing committed phases, rolls and status lifetimes.
    #[must_use]
    pub fn snapshot(&self) -> CombatSnapshot {
        self.state.clone()
    }

    /// Shared current-turn legality query without changing state or RNG.
    pub fn validate_action(&self, actor: ActorId, action: &CombatAction) -> Result<(), RuleError> {
        self.state.validate_action(actor, action)
    }

    /// Legal actions in stable catalog/actor order, useful to deterministic harnesses.
    #[must_use]
    pub fn legal_actions(&self, actor: ActorId) -> Vec<CombatAction> {
        self.state.legal_actions(actor)
    }

    /// Atomically commit one action and automatic boundaries up to the next decision.
    /// Enemy actions are deliberately separate calls, so a paused app never runs AI.
    pub fn apply(
        &mut self,
        actor: ActorId,
        action: CombatAction,
    ) -> Result<Vec<CombatEvent>, RuleError> {
        self.apply_with_budget(actor, action, MAX_WORK)
    }

    fn apply_with_budget(
        &mut self,
        actor: ActorId,
        action: CombatAction,
        budget: usize,
    ) -> Result<Vec<CombatEvent>, RuleError> {
        self.state.validate_action(actor, &action)?;
        let mut candidate = self.clone();
        let mut events = Vec::new();
        let mut work = Work(budget);
        candidate.resolve_action(actor, action, &mut events, &mut work)?;
        candidate.state.revision = candidate
            .state
            .revision
            .checked_add(1)
            .ok_or(RuleError::CounterExhausted)?;
        candidate.state.validate()?;
        *self = candidate;
        Ok(events)
    }

    /// Choose a deterministic legal action only when the active actor is an enemy.
    /// Prefer damaging skills against lowest HP, then stable target identity. A
    /// displaced bruiser repositions toward the front when no attack is reachable.
    #[must_use]
    pub fn ai_action(&self) -> Option<CombatAction> {
        let actor = self.state.active_actor?;
        let source = self.state.actor(actor)?;
        if source.team() != Team::Enemies {
            return None;
        }
        let legal = self.state.legal_actions(actor);
        legal
            .iter()
            .filter_map(|action| match action {
                CombatAction::Skill { skill, target }
                    if skill_definition(*skill)
                        .effects
                        .iter()
                        .any(|effect| matches!(effect, Effect::Damage(_))) =>
                {
                    self.state.actor(*target).map(|target_state| {
                        (
                            (
                                target_state.is_corpse(),
                                target_state.health().0,
                                *target,
                                *skill,
                            ),
                            *action,
                        )
                    })
                }
                _ => None,
            })
            .min_by_key(|(key, _)| *key)
            .map(|(_, action)| action)
            .or_else(|| {
                legal.iter().copied().find(|action| match action {
                    CombatAction::Reposition { ally } => {
                        self.state.rank(*ally) < self.state.rank(actor)
                    }
                    _ => false,
                })
            })
            .or(Some(CombatAction::Defend))
    }

    fn actor_mut(&mut self, id: ActorId) -> Result<&mut ActorSnapshot, RuleError> {
        self.state
            .actors
            .iter_mut()
            .find(|actor| actor.id == id)
            .ok_or(RuleError::UnknownActor)
    }

    fn emit(
        &mut self,
        kind: CombatEventKind,
        events: &mut Vec<CombatEvent>,
        work: &mut Work,
    ) -> Result<(), RuleError> {
        self.resolver().emit(kind, events, work)
    }

    fn start_round(
        &mut self,
        events: &mut Vec<CombatEvent>,
        work: &mut Work,
    ) -> Result<(), RuleError> {
        work.spend()?;
        self.state.phase = CombatPhase::RoundStart;
        self.state.round = self
            .state
            .round
            .checked_add(1)
            .ok_or(RuleError::CounterExhausted)?;
        let mut actors: Vec<_> = self
            .state
            .actors
            .iter()
            .filter(|actor| actor.standing() || actor.dying())
            .map(|actor| (actor.id, actor.speed()))
            .collect();
        actors.sort_by_key(|(id, _)| *id);
        let mut entries = Vec::with_capacity(actors.len());
        for (actor, speed) in actors {
            let roll = u8::try_from(self.rng.bounded(8, work)? + 1)
                .map_err(|_| RuleError::InvalidState)?;
            entries.push(InitiativeEntry {
                actor,
                speed,
                roll,
                total: speed + u16::from(roll),
                tie_breaker: 0,
                completed: false,
            });
        }
        let mut random_order: Vec<_> = (0..entries.len()).collect();
        for index in (1..random_order.len()).rev() {
            let selected = usize::try_from(self.rng.bounded(
                u64::try_from(index + 1).map_err(|_| RuleError::InvalidState)?,
                work,
            )?)
            .map_err(|_| RuleError::InvalidState)?;
            random_order.swap(index, selected);
        }
        for (order, index) in random_order.into_iter().enumerate() {
            entries
                .get_mut(index)
                .ok_or(RuleError::InvalidState)?
                .tie_breaker = u8::try_from(order).map_err(|_| RuleError::InvalidState)?;
        }
        entries.sort_by_key(|entry| {
            (
                std::cmp::Reverse(entry.total),
                std::cmp::Reverse(entry.speed),
                entry.tie_breaker,
            )
        });
        self.state.initiative = entries;
        self.cursor = 0;
        self.emit(
            CombatEventKind::RoundStarted {
                round: self.state.round,
            },
            events,
            work,
        )
    }

    fn seek_decision(
        &mut self,
        events: &mut Vec<CombatEvent>,
        work: &mut Work,
    ) -> Result<(), RuleError> {
        loop {
            work.spend()?;
            if self.check_outcome(events, work)? {
                return Ok(());
            }
            let Some(entry) = self.state.initiative.get(self.cursor).cloned() else {
                self.state.phase = CombatPhase::RoundEnd;
                self.boundary(Boundary::RoundEnd, None, events, work)?;
                self.expire_corpses(events, work)?;
                if self.check_outcome(events, work)? {
                    return Ok(());
                }
                self.start_round(events, work)?;
                continue;
            };
            self.state.turn_id = self
                .state
                .turn_id
                .checked_add(1)
                .ok_or(RuleError::CounterExhausted)?;
            if self
                .state
                .actor(entry.actor)
                .is_some_and(ActorSnapshot::dying)
            {
                self.boundary(Boundary::OwnerTurnStart, Some(entry.actor), events, work)?;
                if self
                    .state
                    .actor(entry.actor)
                    .is_some_and(ActorSnapshot::dying)
                {
                    let roll = u8::try_from(self.rng.bounded(20, work)? + 1)
                        .map_err(|_| RuleError::InvalidState)?;
                    self.death_save(entry.actor, roll, events, work)?;
                }
            }
            if !self
                .state
                .actor(entry.actor)
                .is_some_and(ActorSnapshot::standing)
            {
                self.emit(
                    CombatEventKind::TurnSkipped { actor: entry.actor },
                    events,
                    work,
                )?;
                self.complete_entry()?;
                continue;
            }
            self.state.active_actor = Some(entry.actor);
            self.state.phase = CombatPhase::TurnStart;
            self.emit(
                CombatEventKind::TurnStarted {
                    actor: entry.actor,
                    turn_id: self.state.turn_id,
                },
                events,
                work,
            )?;
            self.boundary(Boundary::OwnerTurnStart, Some(entry.actor), events, work)?;
            if self.check_outcome(events, work)? {
                return Ok(());
            }
            if !self
                .state
                .actor(entry.actor)
                .is_some_and(ActorSnapshot::standing)
            {
                self.emit(
                    CombatEventKind::TurnSkipped { actor: entry.actor },
                    events,
                    work,
                )?;
                self.complete_entry()?;
                continue;
            }
            self.state.phase = CombatPhase::AwaitingAction;
            return Ok(());
        }
    }

    fn complete_entry(&mut self) -> Result<(), RuleError> {
        self.state
            .initiative
            .get_mut(self.cursor)
            .ok_or(RuleError::InvalidState)?
            .completed = true;
        self.cursor = self
            .cursor
            .checked_add(1)
            .ok_or(RuleError::CounterExhausted)?;
        self.state.active_actor = None;
        Ok(())
    }

    fn resolve_action(
        &mut self,
        source: ActorId,
        action: CombatAction,
        events: &mut Vec<CombatEvent>,
        work: &mut Work,
    ) -> Result<(), RuleError> {
        self.resolver()
            .resolve_immediate(source, action, events, work)?;
        if self.state.outcome.is_some() {
            return Ok(());
        }
        self.state.phase = CombatPhase::TurnEnd;
        self.boundary(Boundary::OwnerTurnEnd, Some(source), events, work)?;
        if self.check_outcome(events, work)? {
            return Ok(());
        }
        self.complete_entry()?;
        self.seek_decision(events, work)
    }

    fn effect(
        &mut self,
        source: ActorId,
        target: ActorId,
        effect: Effect,
        potency: u16,
        events: &mut Vec<CombatEvent>,
        work: &mut Work,
    ) -> Result<(), RuleError> {
        self.resolver()
            .effect(source, target, effect, potency, events, work)
    }

    #[cfg(test)]
    fn damage(
        &mut self,
        source: ActorId,
        target: ActorId,
        amount: u16,
        kind: DamageKind,
        events: &mut Vec<CombatEvent>,
        work: &mut Work,
    ) -> Result<(), RuleError> {
        self.resolver()
            .damage(source, target, amount, kind, events, work)
    }

    fn check_outcome(
        &mut self,
        events: &mut Vec<CombatEvent>,
        work: &mut Work,
    ) -> Result<bool, RuleError> {
        self.resolver().check_outcome(events, work)
    }

    #[cfg(test)]
    fn add_status(
        &mut self,
        source: ActorId,
        bearer: ActorId,
        kind: StatusKind,
        events: &mut Vec<CombatEvent>,
        work: &mut Work,
    ) -> Result<(), RuleError> {
        self.resolver()
            .add_status(source, bearer, kind, events, work)
    }

    fn remove_status(
        &mut self,
        bearer: ActorId,
        id: u64,
        reason: RemovalReason,
        events: &mut Vec<CombatEvent>,
        work: &mut Work,
    ) -> Result<(), RuleError> {
        self.resolver()
            .remove_status(bearer, id, reason, events, work)
    }

    #[cfg(test)]
    fn cleanse(
        &mut self,
        bearer: ActorId,
        tag: StatusTag,
        events: &mut Vec<CombatEvent>,
        work: &mut Work,
    ) -> Result<(), RuleError> {
        self.resolver().cleanse(bearer, tag, events, work)
    }

    fn death_save(
        &mut self,
        actor: ActorId,
        roll: u8,
        events: &mut Vec<CombatEvent>,
        work: &mut Work,
    ) -> Result<(), RuleError> {
        let crate::LifeState::Dying { mut failures } = self.actor_mut(actor)?.life else {
            return Ok(());
        };
        if roll < crate::DEATH_SAVE_TARGET {
            failures += 1;
        }
        self.actor_mut(actor)?.life = crate::LifeState::Dying { failures };
        self.emit(
            CombatEventKind::DeathSave {
                actor,
                roll: Some(roll),
                failures,
            },
            events,
            work,
        )?;
        if failures >= crate::DEATH_SAVE_FAILURES {
            self.resolver().make_corpse(actor, events, work)?;
        }
        Ok(())
    }

    fn expire_corpses(
        &mut self,
        events: &mut Vec<CombatEvent>,
        work: &mut Work,
    ) -> Result<(), RuleError> {
        let expired: Vec<_> = self.state.actors.iter().filter(|a| matches!(a.life,
            crate::LifeState::Corpse { created_round, .. } if self.state.round.saturating_sub(created_round) >= crate::CORPSE_ROUNDS
        )).map(|a| a.id).collect();
        for id in expired {
            self.resolver().remove_corpse(id, true, events, work)?;
        }
        Ok(())
    }

    fn boundary(
        &mut self,
        boundary: Boundary,
        owner: Option<ActorId>,
        events: &mut Vec<CombatEvent>,
        work: &mut Work,
    ) -> Result<(), RuleError> {
        work.spend()?;
        self.state.boundary_sequence = self
            .state
            .boundary_sequence
            .checked_add(1)
            .ok_or(RuleError::CounterExhausted)?;
        let sequence = self.state.boundary_sequence;
        let mut pending: Vec<_> = self
            .state
            .actors
            .iter()
            .filter(|actor| owner.is_none_or(|id| id == actor.id))
            .flat_map(|actor| actor.statuses.iter())
            .filter(|status| status.eligible_boundary <= sequence)
            .map(|status| {
                (
                    status_definition(status.kind).priority,
                    status.id,
                    status.bearer,
                )
            })
            .collect();
        pending.sort();
        for (_, id, bearer) in pending {
            work.spend()?;
            let Some(instance) = self
                .actor_mut(bearer)?
                .statuses
                .iter()
                .find(|status| status.id == id)
                .cloned()
            else {
                continue;
            };
            let definition = status_definition(instance.kind);
            let life = self
                .state
                .actor(bearer)
                .ok_or(RuleError::UnknownActor)?
                .life;
            let timing = definition.effective_timing(life);
            // A prior effect may refresh an already-queued ID. That instance is
            // newly activated and must wait for a future boundary just like a new ID.
            if instance.eligible_boundary > sequence {
                continue;
            }
            if timing.trigger == Some(boundary) {
                self.emit(
                    CombatEventKind::StatusTriggered {
                        actor: bearer,
                        instance_id: id,
                        kind: instance.kind,
                    },
                    events,
                    work,
                )?;
                for effect in definition.effects {
                    self.effect(
                        instance.source,
                        bearer,
                        *effect,
                        instance.potency,
                        events,
                        work,
                    )?;
                    if self.state.outcome.is_some() {
                        return Ok(());
                    }
                    if !self
                        .state
                        .actor(bearer)
                        .is_some_and(ActorSnapshot::standing)
                    {
                        break;
                    }
                }
            }
            if timing.duration_boundary == boundary {
                let expired = if let Some(status) = self
                    .actor_mut(bearer)?
                    .statuses
                    .iter_mut()
                    .find(|status| status.id == id)
                {
                    status.remaining = status
                        .remaining
                        .checked_sub(1)
                        .ok_or(RuleError::InvalidState)?;
                    status.remaining == 0
                } else {
                    false
                };
                if expired {
                    self.remove_status(bearer, id, RemovalReason::Expired, events, work)?;
                }
            }
            if owner.is_some()
                && !self
                    .state
                    .actor(bearer)
                    .is_some_and(ActorSnapshot::standing)
            {
                break;
            }
        }
        Ok(())
    }

    #[cfg(test)]
    fn move_actor(
        &mut self,
        target: ActorId,
        offset: i8,
        events: &mut Vec<CombatEvent>,
        work: &mut Work,
    ) -> Result<(), RuleError> {
        self.resolver().move_actor(target, offset, events, work)
    }
}

#[cfg(test)]
#[path = "tests.rs"]
mod tests;

#[cfg(test)]
#[path = "setup_tests.rs"]
mod setup_tests;

#[cfg(test)]
#[path = "preview_tests.rs"]
mod preview_tests;

#[cfg(test)]
#[path = "lifecycle_tests.rs"]
mod lifecycle_tests;
