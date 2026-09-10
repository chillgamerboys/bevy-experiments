//! Viewer-facing combat facts and immediate forecasts, owned by Labyrinth.
//!
//! This is a presentation seam, not network secrecy: current multiplayer snapshots
//! contain all combat facts. Real concealment must also filter recipient snapshots,
//! events and logs before transmission. Unknown inputs never produce exact forecasts.

use std::collections::BTreeMap;

use bevy::prelude::Resource;
use labyrinth_rules::{
    skill_definition, status_definition, ActorId, ActorKind, Boundary, CombatAction,
    CombatSnapshot, Effect, PreviewEvent, RuleError, SkillId, StatusInstance, StatusKind,
    StatusTag,
};

/// A fact that the current viewer can or cannot know; absence is not zero.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Knowledge<T> {
    /// Disclosed, trustworthy presentation fact.
    Known(T),
    /// Not disclosed; no exact result should be inferred from it.
    Unknown,
}

impl<T> Knowledge<T> {
    /// Borrow a disclosed value without inventing a fallback.
    #[must_use]
    pub const fn as_known(&self) -> Option<&T> {
        match self {
            Self::Known(value) => Some(value),
            Self::Unknown => None,
        }
    }
}

/// Independent categories of actor information available to a viewer.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ActorDisclosure {
    /// Exact current and maximum HP; standing/downed state remains public.
    pub health: bool,
    /// Conditions, including modifiers that affect forecasts.
    pub statuses: bool,
    /// Speed, equipped abilities and spent uses.
    pub details: bool,
}

impl Default for ActorDisclosure {
    fn default() -> Self {
        Self {
            health: true,
            statuses: true,
            details: true,
        }
    }
}

/// Game-local viewer policy. Identity, allegiance, position and standing/downed
/// state remain public (including basic targeting legality derived from them).
/// Unspecified actors are fully disclosed in the current cooperative prototype.
#[derive(Resource, Debug, Clone, Default)]
pub struct CombatDisclosure {
    /// Explicit per-actor overrides; games may later derive these from reveal rules.
    pub actors: BTreeMap<ActorId, ActorDisclosure>,
}

impl CombatDisclosure {
    /// Resolve an actor's current visibility policy.
    #[must_use]
    pub fn actor(&self, id: ActorId) -> ActorDisclosure {
        self.actors.get(&id).copied().unwrap_or_default()
    }

    /// Whether free-form logs require conservative suppression in this viewer.
    #[must_use]
    pub fn has_unknown(&self) -> bool {
        self.actors
            .values()
            .any(|v| !v.health || !v.statuses || !v.details)
    }
}

/// Disclosed current/projected HP.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Health {
    /// Remaining HP.
    pub current: u16,
    /// HP ceiling.
    pub maximum: u16,
}

/// Optional inspectable actor details, separate from identity.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ActorDetails {
    /// Effective speed, not a predicted initiative roll.
    pub speed: u16,
    /// Equipped catalog abilities.
    pub skills: Vec<SkillId>,
    /// Uses already spent, not uses remaining.
    pub uses: BTreeMap<SkillId, u16>,
}

/// Information safe to render in actor controls, inspection and accessibility.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ActorPresentation {
    /// Stable identity, unrelated to rank or class.
    pub id: ActorId,
    /// Public artwork/class identity in this prototype.
    pub kind: ActorKind,
    /// Public name.
    pub name: String,
    /// Public standing/downed state, even when exact HP amounts are undisclosed.
    pub standing: bool,
    /// HP knowledge.
    pub health: Knowledge<Health>,
    /// Status knowledge; an unknown list is not an empty list.
    pub statuses: Knowledge<Vec<StatusInstance>>,
    /// Inspection knowledge.
    pub details: Knowledge<ActorDetails>,
}

/// Projection for a single viewer; does not mutate or grant combat authority.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BattlePresentation {
    actors: BTreeMap<ActorId, ActorPresentation>,
}

impl BattlePresentation {
    /// Project disclosed facts before any widget formats them.
    #[must_use]
    pub fn new(snapshot: &CombatSnapshot, disclosure: &CombatDisclosure) -> Self {
        let actors = snapshot
            .actors
            .iter()
            .map(|actor| {
                let policy = disclosure.actor(actor.id);
                (
                    actor.id,
                    ActorPresentation {
                        id: actor.id,
                        kind: actor.kind,
                        name: actor.kind.name().to_owned(),
                        standing: actor.standing(),
                        health: if policy.health {
                            Knowledge::Known(Health {
                                current: actor.hp,
                                maximum: actor.max_hp,
                            })
                        } else {
                            Knowledge::Unknown
                        },
                        statuses: if policy.statuses {
                            Knowledge::Known(actor.statuses.clone())
                        } else {
                            Knowledge::Unknown
                        },
                        // Effective speed is derived from statuses; do not leak concealed Haste.
                        details: if policy.details && policy.statuses {
                            Knowledge::Known(ActorDetails {
                                speed: actor.speed(),
                                skills: actor.skills().to_vec(),
                                uses: actor
                                    .skill_uses
                                    .iter()
                                    .map(|(skill, uses)| (*skill, u16::from(*uses)))
                                    .collect(),
                            })
                        } else {
                            Knowledge::Unknown
                        },
                    },
                )
            })
            .collect();
        Self { actors }
    }

    /// Look up by stable actor identity.
    #[must_use]
    pub fn actor(&self, id: ActorId) -> Option<&ActorPresentation> {
        self.actors.get(&id)
    }
}

/// A single actor's immediate preview, not its state after the next turn starts.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ActorForecast {
    /// Recipient or otherwise affected actor.
    pub actor: ActorId,
    /// HP after this action only.
    pub health: Knowledge<Health>,
    /// Concise immediate outcome, or explicit uncertainty.
    pub summary: String,
}

/// Game-specific forecast text. Catalog/base power is separate from target effects.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ForecastDisplay {
    /// Public authored effects before target modifiers.
    pub base: String,
    /// Target-specific immediate outcome or uncertainty explanation.
    pub summary: String,
    /// Changed actors (plus the selected recipient).
    pub actors: Vec<ActorForecast>,
    /// Exact resolution is unavailable to this viewer.
    pub uncertainty: bool,
}

impl ForecastDisplay {
    /// Forecast known immediate effects without touching live combat or consuming RNG.
    /// Does not grant permission to commit an off-turn action.
    pub fn build(
        snapshot: &CombatSnapshot,
        disclosure: &CombatDisclosure,
        actor: ActorId,
        action: &CombatAction,
    ) -> Result<Self, RuleError> {
        let target = match *action {
            CombatAction::Skill { target, .. } => target,
            CombatAction::Reposition { ally } | CombatAction::Rescue { ally } => ally,
            CombatAction::Defend | CombatAction::Wait => actor,
        };
        let base = base_description(action);
        snapshot.actor(actor).ok_or(RuleError::UnknownActor)?;
        snapshot.actor(target).ok_or(RuleError::UnknownActor)?;
        // Be deliberately conservative: validation and follow-up effects can reveal
        // hidden health, equipment or modifiers too. Do not resolve first and hide later.
        let known = [actor, target]
            .into_iter()
            .all(|id| disclosure.actor(id) == ActorDisclosure::default());
        if !known {
            return Ok(Self {
                base,
                summary: "Outcome uncertain · undisclosed combat details".to_owned(),
                actors: vec![ActorForecast {
                    actor: target,
                    health: Knowledge::Unknown,
                    summary: "Outcome unknown".to_owned(),
                }],
                uncertainty: true,
            });
        }
        let preview = snapshot.preview_action(actor, action)?;
        let mut effects = Vec::new();
        for damage in &preview.damage {
            effects.push(format!(
                "{} damage ({} HP lost)",
                damage.effective, damage.hp_loss
            ));
        }
        for event in &preview.events {
            match event {
                PreviewEvent::Healed { amount, .. } => effects.push(format!("Restore {amount} HP")),
                PreviewEvent::Rescued { hp, .. } => effects.push(format!("Rescue to {hp} HP")),
                PreviewEvent::StatusApplied(status) | PreviewEvent::StatusRefreshed(status) => {
                    let definition = status_definition(status.kind);
                    if definition
                        .effects
                        .iter()
                        .any(|effect| matches!(effect, Effect::StatusDamage(_)))
                    {
                        let clock = match definition.trigger {
                            Some(Boundary::OwnerTurnStart) => "turn start",
                            Some(Boundary::OwnerTurnEnd) => "turn end",
                            Some(Boundary::RoundEnd) => "round end",
                            None => "trigger",
                        };
                        effects.push(format!(
                            "{} {} / {clock}, up to {} ticks if retained",
                            definition.name, status.potency, status.remaining
                        ));
                    } else {
                        effects.push(format!(
                            "{} · {} remaining",
                            definition.name, status.remaining
                        ));
                    }
                }
                PreviewEvent::StatusRemoved { kind, .. } => {
                    effects.push(format!("Remove {}", status_definition(*kind).name))
                }
                PreviewEvent::Downed { .. } => effects.push("Downed".to_owned()),
                PreviewEvent::Defeated { .. } => effects.push("Defeated".to_owned()),
                _ => {}
            }
        }
        let actors = preview
            .actors
            .iter()
            .filter(|change| change.actor == target || change.before != change.after)
            .map(|change| {
                // Formation compaction can affect actors other than the direct target.
                // Never expose their undisclosed HP through an otherwise known attack.
                let health = if disclosure.actor(change.actor).health {
                    Knowledge::Known(Health {
                        current: change.after.hp,
                        maximum: change.after.max_hp,
                    })
                } else {
                    Knowledge::Unknown
                };
                let mut summary = health.as_known().map_or_else(
                    || "HP unknown".to_owned(),
                    |hp| format!("HP {} → {} / {}", change.before.hp, hp.current, hp.maximum),
                );
                if change.before.rank != change.after.rank {
                    if let Some(rank) = change.after.rank {
                        summary.push_str(&format!(" · rank {rank}"));
                    }
                }
                ActorForecast {
                    actor: change.actor,
                    health,
                    summary,
                }
            })
            .collect::<Vec<_>>();
        if let Some(target) = actors.iter().find(|change| change.actor == target) {
            effects.push(target.summary.clone());
        }
        if effects.is_empty() {
            effects.push("No immediate HP change".to_owned());
        }
        Ok(Self {
            base,
            summary: effects.join(" · "),
            actors,
            uncertainty: false,
        })
    }
}

/// Public authored effects, before any actor-specific calculation.
#[must_use]
pub fn base_description(action: &CombatAction) -> String {
    match action {
        CombatAction::Skill { skill, .. } => skill_definition(*skill)
            .effects
            .iter()
            .map(|effect| match *effect {
                Effect::Damage(amount) => format!("{amount} base damage"),
                Effect::Heal(amount) => format!("{amount} base healing"),
                Effect::ApplyStatus(kind) => format!("Apply {}", status_definition(kind).name),
                Effect::Cleanse(tag) => format!(
                    "Cleanse {}",
                    match tag {
                        StatusTag::Buff => "buffs",
                        StatusTag::Debuff => "debuffs",
                        StatusTag::Bleeding => "bleed",
                    }
                ),
                Effect::Move(amount) => format!(
                    "Move {} rank {}",
                    amount.unsigned_abs(),
                    if amount < 0 { "forward" } else { "back" }
                ),
                Effect::SwapWithSource => "Swap positions".to_owned(),
                Effect::Rescue(percent) => format!("Rescue to {percent}% HP"),
                Effect::StatusDamage(_) => "Status damage".to_owned(),
            })
            .collect::<Vec<_>>()
            .join(" · "),
        CombatAction::Reposition { .. } => "Swap with an adjacent ally".to_owned(),
        CombatAction::Rescue { .. } => "Rescue to 25% HP".to_owned(),
        CombatAction::Defend => status_definition(StatusKind::Brace).description.to_owned(),
        CombatAction::Wait => "Spend this action".to_owned(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use labyrinth_rules::{Combat, DEFAULT_HERO_ROSTER};

    #[test]
    fn unknown_health_and_modifiers_never_become_exact_forecasts() {
        let snapshot = Combat::new(42, DEFAULT_HERO_ROSTER)
            .expect("combat")
            .snapshot();
        let target = ActorId(101);
        let mut disclosure = CombatDisclosure::default();
        disclosure.actors.insert(
            target,
            ActorDisclosure {
                health: false,
                statuses: false,
                details: false,
            },
        );
        let projected = BattlePresentation::new(&snapshot, &disclosure);
        let enemy = projected.actor(target).expect("enemy");
        assert_eq!(enemy.health, Knowledge::Unknown);
        assert_eq!(enemy.statuses, Knowledge::Unknown);
        assert_eq!(enemy.details, Knowledge::Unknown);
        let action = CombatAction::Skill {
            skill: SkillId::FrontStrike,
            target,
        };
        let forecast =
            ForecastDisplay::build(&snapshot, &disclosure, ActorId(1), &action).expect("forecast");
        assert_eq!(forecast.base, "7 base damage");
        assert!(forecast.uncertainty);
        assert!(forecast.summary.contains("uncertain"));
        assert_eq!(
            forecast.actors.first().expect("target").health,
            Knowledge::Unknown
        );
        // Concealed changes cannot influence even the text/error shape of this forecast.
        let mut changed = snapshot.clone();
        let enemy = changed
            .actors
            .iter_mut()
            .find(|a| a.id == target)
            .expect("enemy");
        enemy.hp = 1;
        assert_eq!(
            forecast,
            ForecastDisplay::build(&changed, &disclosure, ActorId(1), &action).expect("forecast")
        );
    }

    #[test]
    fn base_power_and_immediate_hp_are_distinct_and_forecasts_do_not_mutate() {
        let snapshot = Combat::new(42, DEFAULT_HERO_ROSTER)
            .expect("combat")
            .snapshot();
        let before = snapshot.clone();
        let forecast = ForecastDisplay::build(
            &snapshot,
            &CombatDisclosure::default(),
            ActorId(1),
            &CombatAction::Skill {
                skill: SkillId::FrontStrike,
                target: ActorId(101),
            },
        )
        .expect("off-turn inspection");
        assert_eq!(forecast.base, "7 base damage");
        assert!(forecast.summary.contains("7 damage"));
        assert!(forecast.summary.contains("HP 20 → 13 / 20"));
        assert_eq!(snapshot, before);
    }

    #[test]
    fn bleed_describes_conditional_ticks_not_promised_future_total() {
        let snapshot = Combat::new(42, DEFAULT_HERO_ROSTER)
            .expect("combat")
            .snapshot();
        let forecast = ForecastDisplay::build(
            &snapshot,
            &CombatDisclosure::default(),
            ActorId(2),
            &CombatAction::Skill {
                skill: SkillId::BleedingCut,
                target: ActorId(101),
            },
        )
        .expect("forecast");
        assert!(forecast
            .summary
            .contains("2 / turn start, up to 3 ticks if retained"));
        assert!(!forecast.summary.contains("6 damage"));
    }

    #[test]
    fn hidden_modifiers_cannot_be_inferred_from_effective_speed() {
        let mut snapshot = Combat::new(42, DEFAULT_HERO_ROSTER)
            .expect("combat")
            .snapshot();
        let mut disclosure = CombatDisclosure::default();
        disclosure.actors.insert(
            ActorId(101),
            ActorDisclosure {
                statuses: false,
                ..Default::default()
            },
        );
        let before = BattlePresentation::new(&snapshot, &disclosure);
        let actor = snapshot
            .actors
            .iter_mut()
            .find(|actor| actor.id == ActorId(101))
            .expect("enemy");
        actor.statuses.push(StatusInstance {
            id: 1000,
            kind: StatusKind::Haste,
            bearer: actor.id,
            source: actor.id,
            potency: 3,
            remaining: 2,
            eligible_boundary: 1,
        });
        assert_eq!(before, BattlePresentation::new(&snapshot, &disclosure));
        assert_eq!(
            before.actor(ActorId(101)).expect("enemy").details,
            Knowledge::Unknown
        );
    }
}
