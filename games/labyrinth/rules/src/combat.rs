//! Atomic reducer and bounded automatic phase resolution.

use std::collections::BTreeMap;

use crate::{
    skill_definition, status_definition, ActorId, ActorKind, ActorSnapshot, Boundary, CombatAction,
    CombatEvent, CombatEventKind, CombatOutcome, CombatPhase, CombatSnapshot, DamageKind, Effect,
    EnemyKind, HeroClass, InitiativeEntry, Reapplication, RemovalReason, RuleError, Stat,
    StatusInstance, StatusKind, StatusTag, Team,
};

const MAX_STATUSES: usize = 16;
const MAX_WORK: usize = 512;

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

struct Work(usize);
impl Work {
    fn spend(&mut self) -> Result<(), RuleError> {
        self.0 = self.0.checked_sub(1).ok_or(RuleError::WorkLimit)?;
        Ok(())
    }
}

impl Combat {
    /// Begin the authored encounter with one of every hero in player/seat order.
    /// IDs 1..4 preserve that ownership order. Initial formation is independently
    /// sorted into Gatekeeper, Knifehand, Scout, Medic ranks regardless of seat.
    /// All start-boundary effects are committed before the first snapshot is exposed.
    pub fn new(seed: u64, heroes: [HeroClass; 4]) -> Result<Self, RuleError> {
        if heroes
            .iter()
            .collect::<std::collections::BTreeSet<_>>()
            .len()
            != 4
        {
            return Err(RuleError::DuplicateHero);
        }
        let mut actors = Vec::with_capacity(8);
        for (index, kind) in heroes
            .into_iter()
            .map(ActorKind::Hero)
            .chain(EnemyKind::ALL.into_iter().map(ActorKind::Enemy))
            .enumerate()
        {
            let id = ActorId(
                u16::try_from(if index < 4 { index + 1 } else { index + 97 })
                    .map_err(|_| RuleError::InvalidState)?,
            );
            let (max_hp, base_speed) = kind.stats();
            actors.push(ActorSnapshot {
                id,
                kind,
                hp: max_hp,
                max_hp,
                base_speed,
                statuses: Vec::new(),
                skill_uses: BTreeMap::new(),
            });
        }
        let mut hero_formation: Vec<_> = actors
            .iter()
            .filter_map(|actor| match actor.kind {
                ActorKind::Hero(class) => Some((class, actor.id)),
                ActorKind::Enemy(_) => None,
            })
            .collect();
        hero_formation.sort_by_key(|(class, _)| *class);
        let state = CombatSnapshot {
            revision: 0,
            round: 0,
            turn_id: 0,
            phase: CombatPhase::RoundStart,
            active_actor: None,
            actors,
            hero_formation: hero_formation.into_iter().map(|(_, id)| id).collect(),
            enemy_formation: (101..=104).map(ActorId).collect(),
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
                    self.state
                        .actor(*target)
                        .map(|target_state| ((target_state.hp, *target, *skill), *action))
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
        work.spend()?;
        let id = self.next_event;
        self.next_event = id.checked_add(1).ok_or(RuleError::CounterExhausted)?;
        events.push(CombatEvent { id, kind });
        Ok(())
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
            .filter(|actor| actor.standing())
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
        self.emit(
            CombatEventKind::Action {
                actor: source,
                action,
            },
            events,
            work,
        )?;
        match action {
            CombatAction::Skill { skill, target } => {
                let definition = skill_definition(skill);
                if definition.max_uses.is_some() {
                    let used = self.actor_mut(source)?.skill_uses.entry(skill).or_default();
                    *used = used.checked_add(1).ok_or(RuleError::CounterExhausted)?;
                }
                for effect in definition.effects {
                    self.effect(source, target, *effect, 0, events, work)?;
                    if self.state.outcome.is_some() {
                        break;
                    }
                }
            }
            CombatAction::Reposition { ally } => self.swap(source, ally, events, work)?,
            CombatAction::Rescue { ally } => {
                self.effect(source, ally, Effect::Rescue(25), 0, events, work)?
            }
            CombatAction::Defend => {
                self.add_status(source, source, StatusKind::Brace, events, work)?
            }
            CombatAction::Wait => {}
        }
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
        work.spend()?;
        let recipient = self.state.actor(target).ok_or(RuleError::UnknownActor)?;
        if !recipient.standing() && !matches!(effect, Effect::Rescue(_) | Effect::SwapWithSource) {
            return Ok(());
        }
        match effect {
            Effect::Damage(base) => {
                let source_modifier = self
                    .state
                    .actor(source)
                    .ok_or(RuleError::UnknownActor)?
                    .modifier(Stat::OutgoingDamage);
                let amount = (i32::from(base)
                    + source_modifier
                    + recipient.modifier(Stat::IncomingDirectDamage))
                .clamp(0, i32::from(u16::MAX));
                self.damage(
                    source,
                    target,
                    u16::try_from(amount).map_err(|_| RuleError::InvalidState)?,
                    DamageKind::Direct,
                    events,
                    work,
                )?;
            }
            Effect::StatusDamage(kind) => {
                self.damage(source, target, potency, kind, events, work)?
            }
            Effect::Heal(amount) => {
                let recipient = self.actor_mut(target)?;
                let restored = amount.min(recipient.max_hp - recipient.hp);
                recipient.hp += restored;
                self.emit(
                    CombatEventKind::Healed {
                        source,
                        target,
                        amount: restored,
                    },
                    events,
                    work,
                )?;
            }
            Effect::ApplyStatus(kind) => self.add_status(source, target, kind, events, work)?,
            Effect::Cleanse(tag) => self.cleanse(target, tag, events, work)?,
            Effect::Move(offset) => self.move_actor(target, offset, events, work)?,
            Effect::SwapWithSource => self.swap(source, target, events, work)?,
            Effect::Rescue(percent) => {
                if recipient.team() != Team::Heroes
                    || recipient.standing()
                    || percent == 0
                    || percent > 100
                {
                    return Err(RuleError::NotDowned);
                }
                let hp =
                    u16::try_from((u32::from(recipient.max_hp) * u32::from(percent)).div_ceil(100))
                        .map_err(|_| RuleError::InvalidState)?
                        .max(1);
                self.actor_mut(target)?.hp = hp;
                self.emit(
                    CombatEventKind::Rescued {
                        source,
                        actor: target,
                        hp,
                    },
                    events,
                    work,
                )?;
            }
        }
        Ok(())
    }

    fn damage(
        &mut self,
        source: ActorId,
        target: ActorId,
        amount: u16,
        kind: DamageKind,
        events: &mut Vec<CombatEvent>,
        work: &mut Work,
    ) -> Result<(), RuleError> {
        let recipient = self.actor_mut(target)?;
        let removed = amount.min(recipient.hp);
        recipient.hp -= removed;
        let downed = !recipient.standing();
        let team = recipient.team();
        self.emit(
            CombatEventKind::Damage {
                source,
                target,
                amount: removed,
                kind,
            },
            events,
            work,
        )?;
        if downed {
            if team == Team::Heroes {
                self.emit(CombatEventKind::Downed { actor: target }, events, work)?;
            } else {
                self.state.enemy_formation.retain(|actor| *actor != target);
                self.emit(CombatEventKind::Defeated { actor: target }, events, work)?;
                self.emit_positions(Team::Enemies, events, work)?;
            }
            let removed: Vec<_> = self
                .actor_mut(target)?
                .statuses
                .iter()
                .filter(|status| {
                    team == Team::Enemies || status_definition(status.kind).remove_on_downed
                })
                .map(|status| status.id)
                .collect();
            for id in removed {
                self.remove_status(
                    target,
                    id,
                    if team == Team::Heroes {
                        RemovalReason::Downed
                    } else {
                        RemovalReason::Defeated
                    },
                    events,
                    work,
                )?;
            }
            self.check_outcome(events, work)?;
        }
        Ok(())
    }

    fn check_outcome(
        &mut self,
        events: &mut Vec<CombatEvent>,
        work: &mut Work,
    ) -> Result<bool, RuleError> {
        if self.state.outcome.is_some() {
            return Ok(true);
        }
        let heroes = self
            .state
            .actors
            .iter()
            .any(|actor| actor.team() == Team::Heroes && actor.standing());
        let outcome = if !heroes {
            Some(CombatOutcome::Defeat)
        } else if self.state.enemy_formation.is_empty() {
            Some(CombatOutcome::Victory)
        } else {
            None
        };
        if let Some(outcome) = outcome {
            self.state.outcome = Some(outcome);
            self.state.phase = CombatPhase::Finished;
            self.state.active_actor = None;
            self.emit(CombatEventKind::Finished { outcome }, events, work)?;
            return Ok(true);
        }
        Ok(false)
    }

    fn add_status(
        &mut self,
        source: ActorId,
        bearer: ActorId,
        kind: StatusKind,
        events: &mut Vec<CombatEvent>,
        work: &mut Work,
    ) -> Result<(), RuleError> {
        work.spend()?;
        let definition = status_definition(kind);
        let eligible_boundary = self
            .state
            .boundary_sequence
            .checked_add(1)
            .ok_or(RuleError::CounterExhausted)?;
        if self.state.actor(source).is_none() {
            return Err(RuleError::UnknownActor);
        }
        if !self
            .state
            .actor(bearer)
            .is_some_and(ActorSnapshot::standing)
        {
            return Err(RuleError::NotStanding);
        }
        if let Some(instance) = self
            .actor_mut(bearer)?
            .statuses
            .iter_mut()
            .find(|instance| instance.kind == kind)
        {
            match definition.reapplication {
                Reapplication::Refresh => {
                    instance.source = source;
                    instance.remaining = definition.duration.ticks;
                    instance.eligible_boundary = eligible_boundary;
                }
            }
            let instance = instance.clone();
            return self.emit(CombatEventKind::StatusRefreshed { instance }, events, work);
        }
        if self.actor_mut(bearer)?.statuses.len() >= MAX_STATUSES {
            return Err(RuleError::WorkLimit);
        }
        let id = self.next_status;
        self.next_status = id.checked_add(1).ok_or(RuleError::CounterExhausted)?;
        let instance = StatusInstance {
            id,
            kind,
            bearer,
            source,
            potency: definition.potency,
            remaining: definition.duration.ticks,
            eligible_boundary,
        };
        self.actor_mut(bearer)?.statuses.push(instance.clone());
        self.emit(CombatEventKind::StatusApplied { instance }, events, work)
    }

    fn remove_status(
        &mut self,
        bearer: ActorId,
        id: u64,
        reason: RemovalReason,
        events: &mut Vec<CombatEvent>,
        work: &mut Work,
    ) -> Result<(), RuleError> {
        let statuses = &mut self.actor_mut(bearer)?.statuses;
        if let Some(index) = statuses.iter().position(|status| status.id == id) {
            let instance = statuses.remove(index);
            self.emit(
                CombatEventKind::StatusRemoved {
                    actor: bearer,
                    instance_id: id,
                    kind: instance.kind,
                    reason,
                },
                events,
                work,
            )?;
        }
        Ok(())
    }

    fn cleanse(
        &mut self,
        bearer: ActorId,
        tag: StatusTag,
        events: &mut Vec<CombatEvent>,
        work: &mut Work,
    ) -> Result<(), RuleError> {
        let ids: Vec<_> = self
            .actor_mut(bearer)?
            .statuses
            .iter()
            .filter(|status| status_definition(status.kind).tags.contains(&tag))
            .map(|status| status.id)
            .collect();
        for id in ids {
            self.remove_status(bearer, id, RemovalReason::Cleansed, events, work)?;
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
            // A prior effect may refresh an already-queued ID. That instance is
            // newly activated and must wait for a future boundary just like a new ID.
            if instance.eligible_boundary > sequence {
                continue;
            }
            if definition.trigger == Some(boundary) {
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
            if definition.duration.boundary == boundary {
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

    fn formation_mut(&mut self, team: Team) -> &mut Vec<ActorId> {
        match team {
            Team::Heroes => &mut self.state.hero_formation,
            Team::Enemies => &mut self.state.enemy_formation,
        }
    }

    fn swap(
        &mut self,
        source: ActorId,
        target: ActorId,
        events: &mut Vec<CombatEvent>,
        work: &mut Work,
    ) -> Result<(), RuleError> {
        let team = self
            .state
            .actor(source)
            .ok_or(RuleError::UnknownActor)?
            .team();
        if self
            .state
            .actor(target)
            .is_none_or(|actor| actor.team() != team)
        {
            return Err(RuleError::WrongTarget);
        }
        let formation = self.formation_mut(team);
        let source_index = formation
            .iter()
            .position(|id| *id == source)
            .ok_or(RuleError::InvalidState)?;
        let target_index = formation
            .iter()
            .position(|id| *id == target)
            .ok_or(RuleError::InvalidState)?;
        formation.swap(source_index, target_index);
        self.emit_positions(team, events, work)
    }

    fn move_actor(
        &mut self,
        target: ActorId,
        offset: i8,
        events: &mut Vec<CombatEvent>,
        work: &mut Work,
    ) -> Result<(), RuleError> {
        let team = self
            .state
            .actor(target)
            .ok_or(RuleError::UnknownActor)?
            .team();
        let formation = self.formation_mut(team);
        let previous = formation
            .iter()
            .position(|id| *id == target)
            .ok_or(RuleError::InvalidState)?;
        let next = (isize::try_from(previous).map_err(|_| RuleError::InvalidState)?
            + isize::from(offset))
        .clamp(
            0,
            isize::try_from(formation.len() - 1).map_err(|_| RuleError::InvalidState)?,
        );
        let next = usize::try_from(next).map_err(|_| RuleError::InvalidState)?;
        if previous != next {
            formation.remove(previous);
            formation.insert(next, target);
            self.emit_positions(team, events, work)?;
        }
        Ok(())
    }

    fn emit_positions(
        &mut self,
        team: Team,
        events: &mut Vec<CombatEvent>,
        work: &mut Work,
    ) -> Result<(), RuleError> {
        let positions = self.formation_mut(team).clone();
        for (index, actor) in positions.into_iter().enumerate() {
            self.emit(
                CombatEventKind::Moved {
                    actor,
                    rank: u8::try_from(index + 1).map_err(|_| RuleError::InvalidState)?,
                },
                events,
                work,
            )?;
        }
        Ok(())
    }
}

#[cfg(test)]
#[path = "tests.rs"]
mod tests;
