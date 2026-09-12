//! Immediate, non-authoritative forecasts. Never a committed snapshot or wire message.

use crate::{
    ActorId, ActorSnapshot, CombatAction, CombatEventKind, CombatOutcome, CombatSnapshot,
    DamageKind, RemovalReason, RuleError, StatusInstance, StatusKind, MAX_COMBAT_WORK,
};

impl CombatSnapshot {
    /// Forecasts only the ordered immediate action effects using these current
    /// facts, even off-turn. Does not mutate this snapshot, allocate real IDs,
    /// consume RNG, advance a phase/boundary, or predict an intervening action.
    ///
    /// This is an all-facts rules calculation, not a disclosure boundary. Games
    /// must filter its output for the viewer before presenting or transmitting it.
    pub fn preview_action(
        &self,
        actor: ActorId,
        action: &CombatAction,
    ) -> Result<ActionPreview, RuleError> {
        self.validate()?;
        self.validate_action_target(actor, action)?;
        let mut candidate = self.clone();
        // Temporary identities only support the existing effect machinery. They
        // never escape the forecast; actual counters are private to the host.
        let mut next_event = 1;
        let mut next_status = self
            .actors
            .iter()
            .flat_map(|actor| &actor.statuses)
            .map(|status| status.id)
            .max()
            .unwrap_or(0)
            .saturating_add(1);
        let mut damage = Vec::new();
        let mut movement = Vec::new();
        let mut events = Vec::new();
        crate::resolve::EffectResolver {
            state: &mut candidate,
            next_event: &mut next_event,
            next_status: &mut next_status,
            damage: Some(&mut damage),
            movement: Some(&mut movement),
        }
        .resolve_immediate(
            actor,
            *action,
            &mut events,
            &mut crate::combat::Work(MAX_COMBAT_WORK),
        )?;
        let actors = self
            .actors
            .iter()
            .map(|before| {
                let after = candidate.actor(before.id).ok_or(RuleError::InvalidState)?;
                let mut after_state = actor_state(&candidate, after);
                if before.is_corpse() && after.life == crate::LifeState::Removed {
                    // Forecast depletion of the pool that existed before clearing.
                    // Removed authority identities retain no corpse durability.
                    after_state.max_hp = before.health().1;
                }
                Ok(ActorPreview {
                    actor: before.id,
                    before: actor_state(self, before),
                    after: after_state,
                })
            })
            .collect::<Result<Vec<_>, RuleError>>()?;
        let events = events
            .into_iter()
            .filter_map(|event| {
                if matches!(event.kind, CombatEventKind::Action { .. }) {
                    None
                } else {
                    Some(PreviewEvent::try_from(event.kind))
                }
            })
            .collect::<Result<Vec<_>, RuleError>>()?;
        Ok(ActionPreview {
            actor,
            action: *action,
            actors,
            damage,
            movement,
            events,
            outcome: candidate.outcome,
        })
    }
}

fn actor_state(snapshot: &CombatSnapshot, actor: &ActorSnapshot) -> ActorPreviewState {
    ActorPreviewState {
        hp: actor.health().0,
        max_hp: actor.health().1,
        life: actor.life,
        rank: snapshot.rank(actor.id),
        statuses: actor.statuses.iter().map(PreviewStatus::from).collect(),
    }
}

impl From<&StatusInstance> for PreviewStatus {
    fn from(status: &StatusInstance) -> Self {
        Self {
            kind: status.kind,
            bearer: status.bearer,
            source: status.source,
            potency: status.potency,
            remaining: status.remaining,
        }
    }
}

impl TryFrom<CombatEventKind> for PreviewEvent {
    type Error = RuleError;

    fn try_from(event: CombatEventKind) -> Result<Self, Self::Error> {
        Ok(match event {
            CombatEventKind::Damage {
                source,
                target,
                amount,
                kind,
            } => Self::Damage {
                source,
                target,
                amount,
                kind,
            },
            CombatEventKind::Healed {
                source,
                target,
                amount,
            } => Self::Healed {
                source,
                target,
                amount,
            },
            CombatEventKind::Downed { actor } => Self::Downed { actor },
            CombatEventKind::Defeated { actor } => Self::Defeated { actor },
            CombatEventKind::CorpseRemoved { actor, .. } => Self::CorpseRemoved { actor },
            CombatEventKind::DeathSave {
                actor, failures, ..
            } => Self::DeathSave { actor, failures },
            CombatEventKind::Rescued { source, actor, hp } => Self::Rescued { source, actor, hp },
            CombatEventKind::Moved { actor, rank } => Self::Moved { actor, rank },
            CombatEventKind::StatusApplied { instance } => {
                Self::StatusApplied(PreviewStatus::from(&instance))
            }
            CombatEventKind::StatusRefreshed { instance } => {
                Self::StatusRefreshed(PreviewStatus::from(&instance))
            }
            CombatEventKind::StatusRemoved {
                actor,
                kind,
                reason,
                ..
            } => Self::StatusRemoved {
                actor,
                kind,
                reason,
            },
            CombatEventKind::Finished { outcome } => Self::Finished { outcome },
            CombatEventKind::Action { .. }
            | CombatEventKind::RoundStarted { .. }
            | CombatEventKind::TurnStarted { .. }
            | CombatEventKind::TurnSkipped { .. }
            | CombatEventKind::StatusTriggered { .. } => return Err(RuleError::InvalidState),
        })
    }
}

/// An immediate action forecast using the currently supplied facts. No next-turn
/// ticks, enemy decisions, initiative rolls, or visibility policy are predicted.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ActionPreview {
    /// Inspected source, which need not own the current initiative turn.
    pub actor: ActorId,
    /// Inspected action; this value grants no authority to commit it.
    pub action: CombatAction,
    /// Before/after facts for every actor, including formation compaction.
    pub actors: Vec<ActorPreview>,
    /// Damage calculations in immediate effect order, not promised future ticks.
    pub damage: Vec<DamagePreview>,
    /// Displacements actually attempted by the shared resolver, including limits.
    pub movement: Vec<MovementPreview>,
    /// Immediate outcomes without event or speculative status-instance identities.
    pub events: Vec<PreviewEvent>,
    /// Outcome if this immediate action alone ends the encounter.
    pub outcome: Option<CombatOutcome>,
}

impl ActionPreview {
    /// Finds one actor's projected facts without treating array order as identity.
    #[must_use]
    pub fn actor(&self, actor: ActorId) -> Option<&ActorPreview> {
        self.actors.iter().find(|change| change.actor == actor)
    }
}

/// Why the remaining displacement could not be completed.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MovementLimit {
    /// No further occupant exists in the requested direction.
    FormationEdge,
    /// The remaining distance cannot cross this occupant's whole footprint.
    Footprint {
        /// Adjacent occupant, not an arbitrary formation index.
        actor: ActorId,
        /// Number of ranks required to pass it.
        ranks: u8,
    },
}

/// An attempted push/pull, observed in the same resolver used for commits.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MovementPreview {
    /// Displaced character.
    pub actor: ActorId,
    /// Signed authored distance: negative is forward, positive is back.
    pub requested: i8,
    /// Leading rank before movement.
    pub from: u8,
    /// Leading rank after movement.
    pub to: u8,
    /// Absent when the complete requested distance was covered.
    pub limit: Option<MovementLimit>,
}

/// One actor's immediate projected change. Viewers must filter undisclosed facts.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ActorPreview {
    /// Stable character identity.
    pub actor: ActorId,
    /// Current facts before any immediate effect.
    pub before: ActorPreviewState,
    /// Facts after ordered immediate effects, before any combat boundary.
    pub after: ActorPreviewState,
}

/// Forecast facts, deliberately not a serializable authoritative actor snapshot.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ActorPreviewState {
    /// Explicit projected life state, so corpse HP is not mistaken for revival.
    pub life: crate::LifeState,
    /// Current/projected HP.
    pub hp: u16,
    /// Ceiling of the displayed pool; corpse clearing retains its before-pool ceiling.
    pub max_hp: u16,
    /// One-based rank, absent for a defeated enemy.
    pub rank: Option<u8>,
    /// Effects remaining immediately afterward, not after the next boundary.
    pub statuses: Vec<PreviewStatus>,
}

/// Status semantics without an allocated instance identity or timeline counter.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PreviewStatus {
    /// Catalog identity.
    pub kind: StatusKind,
    /// Character carrying this effect.
    pub bearer: ActorId,
    /// Most recent applying source.
    pub source: ActorId,
    /// Current/projected potency.
    pub potency: u16,
    /// Remaining future matching boundaries; not guaranteed damage ticks.
    pub remaining: u8,
}

/// One actual immediate hit, separating authored power from effective HP loss.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DamagePreview {
    /// Damage source.
    pub source: ActorId,
    /// Recipient.
    pub target: ActorId,
    /// Direct damage or status damage; only direct damage uses modifiers.
    pub kind: DamageKind,
    /// Authored direct base or status potency, before modifiers.
    pub base: u16,
    /// Damage after modifiers and numeric clamps, before remaining-HP cap.
    pub effective: u16,
    /// Actual HP removed, capped by the recipient's current HP.
    pub hp_loss: u16,
}

/// Ordered immediate effects. No event IDs, turn starts, or future status triggers.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PreviewEvent {
    /// Corpse destruction frees its occupied spaces.
    CorpseRemoved {
        /// Removed identity.
        actor: ActorId,
    },
    /// Damage to a dying hero causes an automatic failure, not a predicted die roll.
    DeathSave {
        /// Dying identity.
        actor: ActorId,
        /// Resulting failure count.
        failures: u8,
    },
    /// HP removed after all modifiers and caps.
    Damage {
        /// Damage source.
        source: ActorId,
        /// Recipient.
        target: ActorId,
        /// Actual HP loss.
        amount: u16,
        /// Damage category.
        kind: DamageKind,
    },
    /// HP restored to a standing actor.
    Healed {
        /// Healing source.
        source: ActorId,
        /// Recipient.
        target: ActorId,
        /// Actual HP restored after its cap.
        amount: u16,
    },
    /// A hero would be downed.
    Downed {
        /// Hero identity.
        actor: ActorId,
    },
    /// An enemy would leave the formation.
    Defeated {
        /// Enemy identity.
        actor: ActorId,
    },
    /// A downed hero would be rescued.
    Rescued {
        /// Rescuing source.
        source: ActorId,
        /// Rescued hero.
        actor: ActorId,
        /// Restored HP.
        hp: u16,
    },
    /// A formation member's resulting rank, including compaction after a kill.
    Moved {
        /// Character identity.
        actor: ActorId,
        /// New one-based rank.
        rank: u8,
    },
    /// A new effect would be applied; no real instance ID has been allocated.
    StatusApplied(PreviewStatus),
    /// An existing effect would be refreshed rather than stacked.
    StatusRefreshed(PreviewStatus),
    /// An effect would be removed by this action.
    StatusRemoved {
        /// Bearer.
        actor: ActorId,
        /// Removed catalog kind.
        kind: StatusKind,
        /// Removal cause.
        reason: RemovalReason,
    },
    /// Immediate action completes combat.
    Finished {
        /// Projected outcome.
        outcome: CombatOutcome,
    },
}
