//! Shared immediate effect resolution, deliberately without RNG or turn advancement.

use crate::combat::Work;
use crate::{
    skill_definition, status_definition, ActorId, ActorSnapshot, CombatAction, CombatEvent,
    CombatEventKind, CombatOutcome, CombatPhase, CombatSnapshot, DamageKind, DamagePreview, Effect,
    Reapplication, RemovalReason, RuleError, Stat, StatusInstance, StatusKind, StatusTag, Team,
    MAX_STATUSES,
};

pub(super) struct EffectResolver<'a> {
    pub state: &'a mut CombatSnapshot,
    pub next_event: &'a mut u64,
    pub next_status: &'a mut u64,
    pub damage: Option<&'a mut Vec<DamagePreview>>,
}

impl EffectResolver<'_> {
    pub(super) fn actor_mut(&mut self, id: ActorId) -> Result<&mut ActorSnapshot, RuleError> {
        self.state
            .actors
            .iter_mut()
            .find(|actor| actor.id == id)
            .ok_or(RuleError::UnknownActor)
    }

    pub(super) fn emit(
        &mut self,
        kind: CombatEventKind,
        events: &mut Vec<CombatEvent>,
        work: &mut Work,
    ) -> Result<(), RuleError> {
        work.spend()?;
        let id = *self.next_event;
        *self.next_event = id.checked_add(1).ok_or(RuleError::CounterExhausted)?;
        events.push(CombatEvent { id, kind });
        Ok(())
    }

    pub(super) fn resolve_immediate(
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
        Ok(())
    }

    pub(super) fn effect(
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
        let allowed = recipient.standing()
            || match effect {
                Effect::Damage(_) | Effect::StatusDamage(_) => recipient.damageable(),
                Effect::Rescue(_) | Effect::SwapWithSource => recipient.dying(),
                _ => false,
            };
        if !allowed {
            return Ok(());
        }
        match effect {
            Effect::Damage(base) => {
                self.hit(source, target, base, DamageKind::Direct, events, work)?
            }
            Effect::StatusDamage(kind) => self.hit(source, target, potency, kind, events, work)?,
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
                    || !recipient.dying()
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
                self.actor_mut(target)?.life = crate::LifeState::Alive;
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

    pub(super) fn hit(
        &mut self,
        source: ActorId,
        target: ActorId,
        base: u16,
        kind: DamageKind,
        events: &mut Vec<CombatEvent>,
        work: &mut Work,
    ) -> Result<(), RuleError> {
        let source_actor = self.state.actor(source).ok_or(RuleError::UnknownActor)?;
        let recipient = self.state.actor(target).ok_or(RuleError::UnknownActor)?;
        let modifier = if kind == DamageKind::Direct {
            source_actor.modifier(Stat::OutgoingDamage)
                + recipient.modifier(Stat::IncomingDirectDamage)
        } else {
            0
        };
        let effective = u16::try_from((i32::from(base) + modifier).clamp(0, i32::from(u16::MAX)))
            .map_err(|_| RuleError::InvalidState)?;
        let preview = DamagePreview {
            source,
            target,
            kind,
            base,
            effective,
            hp_loss: effective.min(recipient.health().0),
        };
        if let Some(damage) = self.damage.as_deref_mut() {
            damage.push(preview);
        }
        self.damage(source, target, effective, kind, events, work)
    }

    pub(super) fn damage(
        &mut self,
        source: ActorId,
        target: ActorId,
        amount: u16,
        kind: DamageKind,
        events: &mut Vec<CombatEvent>,
        work: &mut Work,
    ) -> Result<(), RuleError> {
        let recipient = self.actor_mut(target)?;
        if !recipient.damageable() {
            return Ok(());
        }
        if let crate::LifeState::Corpse {
            hp,
            max_hp,
            created_round,
        } = recipient.life
        {
            let removed = amount.min(hp);
            recipient.life = crate::LifeState::Corpse {
                hp: hp - removed,
                max_hp,
                created_round,
            };
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
            if removed == hp {
                self.remove_corpse(target, false, events, work)?;
            }
            return Ok(());
        }
        if let crate::LifeState::Dying { failures } = recipient.life {
            if amount > 0 {
                let failures = failures + 1;
                recipient.life = crate::LifeState::Dying { failures };
                self.emit(
                    CombatEventKind::DeathSave {
                        actor: target,
                        roll: None,
                        failures,
                    },
                    events,
                    work,
                )?;
                if failures >= crate::DEATH_SAVE_FAILURES {
                    self.make_corpse(target, events, work)?;
                }
            }
            return Ok(());
        }
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
                self.actor_mut(target)?.life = crate::LifeState::Dying { failures: 0 };
                self.emit(CombatEventKind::Downed { actor: target }, events, work)?;
            } else {
                self.make_corpse(target, events, work)?;
            }
            let removed: Vec<_> = self
                .actor_mut(target)?
                .statuses
                .iter()
                .filter(|status| {
                    team == Team::Heroes && status_definition(status.kind).remove_on_downed
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

    pub(super) fn make_corpse(
        &mut self,
        actor: ActorId,
        events: &mut Vec<CombatEvent>,
        work: &mut Work,
    ) -> Result<(), RuleError> {
        let created_round = self.state.round;
        let target = self.actor_mut(actor)?;
        let max_hp = target.max_hp.div_ceil(4);
        target.hp = 0;
        target.life = crate::LifeState::Corpse {
            hp: max_hp,
            max_hp,
            created_round,
        };
        let removed: Vec<_> = target
            .statuses
            .iter()
            .filter(|s| !status_definition(s.kind).persist_on_death)
            .map(|s| s.id)
            .collect();
        for id in removed {
            self.remove_status(actor, id, RemovalReason::Defeated, events, work)?;
        }
        self.emit(CombatEventKind::Defeated { actor }, events, work)
    }

    pub(super) fn remove_corpse(
        &mut self,
        actor: ActorId,
        expired: bool,
        events: &mut Vec<CombatEvent>,
        work: &mut Work,
    ) -> Result<(), RuleError> {
        let target = self.actor_mut(actor)?;
        let team = target.team();
        target.life = crate::LifeState::Removed;
        let ids: Vec<_> = target.statuses.iter().map(|s| s.id).collect();
        for id in ids {
            self.remove_status(actor, id, RemovalReason::Defeated, events, work)?;
        }
        self.formation_mut(team).retain(|id| *id != actor);
        self.emit(
            CombatEventKind::CorpseRemoved { actor, expired },
            events,
            work,
        )?;
        self.emit_positions(team, events, work)
    }

    pub(super) fn check_outcome(
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
        } else if !self
            .state
            .actors
            .iter()
            .any(|a| a.team() == Team::Enemies && a.standing())
        {
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

    pub(super) fn add_status(
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
        let id = *self.next_status;
        *self.next_status = id.checked_add(1).ok_or(RuleError::CounterExhausted)?;
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

    pub(super) fn remove_status(
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

    pub(super) fn cleanse(
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

    pub(super) fn formation_mut(&mut self, team: Team) -> &mut Vec<ActorId> {
        match team {
            Team::Heroes => &mut self.state.hero_formation,
            Team::Enemies => &mut self.state.enemy_formation,
        }
    }

    pub(super) fn swap(
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

    pub(super) fn move_actor(
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
        let formation = self.state.formation(team);
        let previous = formation
            .iter()
            .position(|id| *id == target)
            .ok_or(RuleError::InvalidState)?;
        // A displacement measures rank distance, not number of occupants. Never
        // split another footprint or silently turn a one-rank push into two.
        let mut next = previous;
        let mut remaining = offset.unsigned_abs();
        loop {
            let adjacent = if offset < 0 {
                next.checked_sub(1)
            } else {
                next.checked_add(1).filter(|i| *i < formation.len())
            };
            let Some(adjacent) = adjacent else {
                break;
            };
            let width = self
                .state
                .actor(*formation.get(adjacent).ok_or(RuleError::InvalidState)?)
                .ok_or(RuleError::InvalidState)?
                .kind
                .footprint();
            if remaining < width {
                break;
            }
            remaining -= width;
            next = adjacent;
        }
        if previous != next {
            let formation = self.formation_mut(team);
            formation.remove(previous);
            formation.insert(next, target);
            self.emit_positions(team, events, work)?;
        }
        Ok(())
    }

    pub(super) fn emit_positions(
        &mut self,
        team: Team,
        events: &mut Vec<CombatEvent>,
        work: &mut Work,
    ) -> Result<(), RuleError> {
        let positions = self.formation_mut(team).clone();
        for actor in positions {
            let rank = self.state.rank(actor).ok_or(RuleError::InvalidState)?;
            self.emit(CombatEventKind::Moved { actor, rank }, events, work)?;
        }
        Ok(())
    }
}
