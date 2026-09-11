//! Viewer-facing combat facts and immediate forecasts, owned by Labyrinth.
//!
//! This is a presentation seam, not network secrecy: current multiplayer snapshots
//! contain all combat facts. Real concealment must also filter recipient snapshots,
//! events and logs before transmission. Unknown inputs never produce exact forecasts.

use std::collections::BTreeMap;

use bevy::prelude::Resource;
use labyrinth_rules::{
    skill_definition, status_definition, ActorId, ActorKind, ActorSnapshot, Boundary, CombatAction,
    CombatSnapshot, Effect, LifeState, PreviewEvent, RuleError, SkillId, StatusInstance,
    StatusKind, StatusTag,
};

/// Character names belong to the game's presentation, not combat authority.
/// The encounter roster retains dead/removed actors, so ordering by stable ID
/// survives formation changes, snapshot serialization and repeated hero classes.
#[must_use]
pub fn actor_name(snapshot: &CombatSnapshot, actor: &ActorSnapshot) -> String {
    const HERO_NAMES: [&str; 6] = ["Alden", "Mara", "Rowan", "Iris", "Ember", "Sera"];
    if matches!(actor.kind, ActorKind::Enemy(_)) {
        return actor.kind.name().to_owned();
    }
    let index = snapshot
        .actors
        .iter()
        .filter(|other| other.team() == actor.team() && other.id < actor.id)
        .count();
    HERO_NAMES
        .get(index)
        .map_or_else(|| format!("Hero {}", actor.id.0), |name| (*name).to_owned())
}

/// Include a hero's class when there is room for inspection detail.
#[must_use]
pub fn actor_title(snapshot: &CombatSnapshot, actor: &ActorSnapshot) -> String {
    let name = actor_name(snapshot, actor);
    match actor.kind {
        ActorKind::Hero(_) => format!("{name} · {}", actor.kind.name()),
        ActorKind::Enemy(_) => name,
    }
}

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
                        name: actor_name(snapshot, actor),
                        standing: actor.standing(),
                        health: if policy.health {
                            Knowledge::Known(Health {
                                current: actor.health().0,
                                maximum: actor.health().1,
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

/// Distinguishes depletion of a living pool from damage to permanent remains.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ForecastOutcome {
    /// Ordinary living HP/status/position change.
    Living,
    /// Permanent death creates a separate corpse pool.
    CorpseCreated,
    /// Existing remains retain some durability.
    CorpseDamaged,
    /// Existing remains are destroyed and leave formation.
    CorpseCleared,
}

/// Public rank change after all immediate effects, including displaced neighbours.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PositionForecast {
    /// Leading rank before the action.
    pub from: u8,
    /// Leading rank after the action.
    pub to: u8,
    /// Whole occupied width at either position.
    pub footprint: u8,
}

/// A single actor's immediate preview, not its state after the next turn starts.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ActorForecast {
    /// Recipient or otherwise affected actor.
    pub actor: ActorId,
    /// Disclosed outcome and the kind of pool being depleted.
    pub outcome: Knowledge<ForecastOutcome>,
    /// HP after this action only, using the before-pool ceiling when it is cleared.
    pub health: Knowledge<Health>,
    /// Disclosed movement only; absent for unchanged, removed or uncertain actors.
    pub position: Option<PositionForecast>,
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
    /// Push/pull result and any resolver-owned limit; absent for concealed outcomes.
    pub movement: Option<String>,
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
                    outcome: Knowledge::Unknown,
                    health: Knowledge::Unknown,
                    position: None,
                    summary: "Outcome unknown".to_owned(),
                }],
                uncertainty: true,
                movement: None,
            });
        }
        let preview = snapshot.preview_action(actor, action)?;
        let movement = (!preview.movement.is_empty()).then(|| {
            preview
                .movement
                .iter()
                .map(|movement| {
                    let distance = movement.from.abs_diff(movement.to);
                    let direction = if movement.requested < 0 {
                        "Pull"
                    } else {
                        "Push"
                    };
                    let mut text = format!(
                        "{direction} {distance}/{} ranks",
                        movement.requested.unsigned_abs()
                    );
                    match movement.limit {
                        Some(labyrinth_rules::MovementLimit::FormationEdge) => {
                            text.push_str(" · formation edge");
                        }
                        Some(labyrinth_rules::MovementLimit::Footprint { actor, ranks }) => {
                            let name = snapshot.actor(actor).map_or_else(
                                || "unit".to_owned(),
                                |actor| actor_name(snapshot, actor),
                            );
                            text.push_str(&format!(" · {name} needs {ranks} ranks to pass"));
                        }
                        None => {}
                    }
                    text
                })
                .collect::<Vec<_>>()
                .join(" · ")
        });
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
                        let life = preview
                            .actor(status.bearer)
                            .ok_or(RuleError::UnknownActor)?
                            .after
                            .life;
                        let clock = match definition.effective_timing(life).trigger {
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
                PreviewEvent::Defeated { .. } => effects.push("Dies; leaves a corpse".to_owned()),
                // The affected actor's typed forecast below explains clearing.
                PreviewEvent::CorpseRemoved { .. } => {}
                PreviewEvent::DeathSave { failures, .. } => {
                    effects.push(format!("Death save: {failures}/3 failures"))
                }
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
                let outcome = match (change.before.life, change.after.life) {
                    (LifeState::Corpse { .. }, LifeState::Removed) => {
                        ForecastOutcome::CorpseCleared
                    }
                    (LifeState::Corpse { .. }, _) => ForecastOutcome::CorpseDamaged,
                    (_, LifeState::Corpse { .. }) => ForecastOutcome::CorpseCreated,
                    _ => ForecastOutcome::Living,
                };
                let newly_dead = outcome == ForecastOutcome::CorpseCreated;
                let cleared = outcome == ForecastOutcome::CorpseCleared;
                let health = if disclosure.actor(change.actor).health {
                    Knowledge::Known(Health {
                        current: if newly_dead { 0 } else { change.after.hp },
                        maximum: if newly_dead || cleared {
                            change.before.max_hp
                        } else {
                            change.after.max_hp
                        },
                    })
                } else {
                    Knowledge::Unknown
                };
                let mut summary = health.as_known().map_or_else(
                    || "HP unknown".to_owned(),
                    |hp| format!("HP {} → {} / {}", change.before.hp, hp.current, hp.maximum),
                );
                if newly_dead {
                    summary = health.as_known().map_or_else(
                        || "Dies; corpse HP unknown".into(),
                        |_| {
                            format!(
                                "Dies; corpse {}/{} HP",
                                change.after.hp, change.after.max_hp
                            )
                        },
                    );
                }
                if matches!(
                    outcome,
                    ForecastOutcome::CorpseDamaged | ForecastOutcome::CorpseCleared
                ) {
                    summary = health.as_known().map_or_else(
                        || "Corpse durability unknown".into(),
                        |hp| {
                            format!(
                                "Corpse durability {} → {} / {}",
                                change.before.hp, hp.current, hp.maximum
                            )
                        },
                    );
                    if cleared {
                        summary.push_str(" · Corpse cleared; formation closes");
                    }
                }
                let position = change
                    .before
                    .rank
                    .zip(change.after.rank)
                    .filter(|(from, to)| from != to)
                    .map(|(from, to)| PositionForecast {
                        from,
                        to,
                        footprint: snapshot
                            .actor(change.actor)
                            .map_or(1, |actor| actor.kind.footprint()),
                    });
                if let Some(position) = position {
                    summary.push_str(&format!(" · rank {} → {}", position.from, position.to));
                }
                ActorForecast {
                    actor: change.actor,
                    outcome: if disclosure.actor(change.actor).health {
                        Knowledge::Known(outcome)
                    } else {
                        Knowledge::Unknown
                    },
                    health,
                    position,
                    summary,
                }
            })
            .collect::<Vec<_>>();
        if let Some(target) = actors.iter().find(|change| change.actor == target) {
            effects.push(target.summary.clone());
        }
        if let Some(movement) = &movement {
            effects.push(movement.clone());
        }
        if effects.is_empty() {
            effects.push("No immediate HP change".to_owned());
        }
        Ok(Self {
            base,
            summary: effects.join(" · "),
            actors,
            uncertainty: false,
            movement,
        })
    }
}

/// Explain a disclosed instance using the exact clocks used by the rules reducer.
#[must_use]
pub fn status_description(status: &StatusInstance, life: LifeState) -> String {
    let definition = status_definition(status.kind);
    let timing = definition.effective_timing(life);
    let boundary_name = |boundary| match boundary {
        Boundary::OwnerTurnStart => "turn start",
        Boundary::OwnerTurnEnd => "turn end",
        Boundary::RoundEnd => "round end",
    };
    let duration = match timing.duration_boundary {
        Boundary::OwnerTurnStart => "turn-start",
        Boundary::OwnerTurnEnd => "turn-end",
        Boundary::RoundEnd => "round-end",
    };
    let ticks = if status.remaining == 1 {
        "tick"
    } else {
        "ticks"
    };
    let boundaries = if status.remaining == 1 {
        "boundary"
    } else {
        "boundaries"
    };
    let mut text = if let Some(trigger) = timing.trigger.filter(|_| {
        definition
            .effects
            .iter()
            .any(|effect| matches!(effect, Effect::StatusDamage(_)))
    }) {
        format!(
            "{} · {} damage at {}; up to {} {duration} {ticks} if retained",
            definition.name,
            status.potency,
            boundary_name(trigger),
            status.remaining
        )
    } else {
        format!(
            "{} · potency {} · expires after {} {duration} {boundaries}",
            definition.name, status.potency, status.remaining
        )
    };
    match life {
        LifeState::Corpse { .. } => text
            .push_str(". Corpses never take turns; clocks use round end while the corpse remains"),
        LifeState::Dying { .. } => {
            text.push_str(". Turn start is the dying hero's initiative slot; it chooses no action")
        }
        _ => {}
    }
    text
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
    fn corpse_forecasts_preserve_durability_and_disclosure_through_removal() {
        for (hp, remaining, outcome) in [
            (5, 1, ForecastOutcome::CorpseDamaged),
            (1, 0, ForecastOutcome::CorpseCleared),
        ] {
            let mut snapshot = Combat::new(42, DEFAULT_HERO_ROSTER)
                .expect("combat")
                .snapshot();
            let corpse = snapshot
                .actors
                .iter_mut()
                .find(|a| a.id == ActorId(101))
                .expect("enemy");
            corpse.hp = 0;
            corpse.life = LifeState::Corpse {
                hp,
                max_hp: 5,
                created_round: snapshot.round,
            };
            snapshot.validate().expect("valid corpse");
            let before = snapshot.clone();
            let action = CombatAction::Skill {
                skill: SkillId::SnapShot,
                target: ActorId(101),
            };
            let preview = snapshot
                .preview_action(ActorId(4), &action)
                .expect("preview");
            let target = preview.actor(ActorId(101)).expect("corpse");
            assert_eq!((target.after.hp, target.after.max_hp), (remaining, 5));
            let mut disclosure = CombatDisclosure::default();
            disclosure.actors.insert(
                ActorId(102),
                ActorDisclosure {
                    health: false,
                    ..Default::default()
                },
            );
            let forecast = ForecastDisplay::build(&snapshot, &disclosure, ActorId(4), &action)
                .expect("forecast");
            let target = forecast
                .actors
                .iter()
                .find(|a| a.actor == ActorId(101))
                .expect("target");
            assert_eq!(
                target.health,
                Knowledge::Known(Health {
                    current: remaining,
                    maximum: 5
                })
            );
            assert_eq!(target.outcome, Knowledge::Known(outcome));
            assert!(target
                .summary
                .contains(&format!("Corpse durability {hp} → {remaining} / 5")));
            if remaining == 0 {
                assert!(target.summary.contains("Corpse cleared"));
                let bystander = forecast
                    .actors
                    .iter()
                    .find(|a| a.actor == ActorId(102))
                    .expect("compacted neighbor");
                assert_eq!(bystander.health, Knowledge::Unknown);
                assert!(bystander.summary.contains("HP unknown"));
            }
            disclosure.actors.insert(
                ActorId(101),
                ActorDisclosure {
                    health: false,
                    ..Default::default()
                },
            );
            let hidden = ForecastDisplay::build(&snapshot, &disclosure, ActorId(4), &action)
                .expect("hidden forecast");
            assert!(hidden.uncertainty);
            assert_eq!(
                hidden.actors.first().expect("target").health,
                Knowledge::Unknown
            );
            assert_eq!(
                hidden.actors.first().expect("target").outcome,
                Knowledge::Unknown
            );
            assert_eq!(snapshot, before, "inspection does not mutate state");
        }
    }

    #[test]
    fn dying_hero_death_and_rescue_forecast_separate_living_and_corpse_pools() {
        let mut snapshot = Combat::new(42, DEFAULT_HERO_ROSTER)
            .expect("combat")
            .snapshot();
        let target = if snapshot.active_actor == Some(ActorId(1)) {
            ActorId(2)
        } else {
            ActorId(1)
        };
        let hero = snapshot
            .actors
            .iter_mut()
            .find(|a| a.id == target)
            .expect("hero");
        let maximum = hero.max_hp;
        hero.hp = 0;
        hero.life = LifeState::Dying { failures: 2 };
        snapshot.validate().expect("dying fixture");
        let death = ForecastDisplay::build(
            &snapshot,
            &CombatDisclosure::default(),
            ActorId(101),
            &CombatAction::Skill {
                skill: SkillId::BrutalStrike,
                target,
            },
        )
        .expect("death forecast");
        let hero = death
            .actors
            .iter()
            .find(|a| a.actor == target)
            .expect("hero");
        assert_eq!(
            hero.outcome,
            Knowledge::Known(ForecastOutcome::CorpseCreated)
        );
        assert_eq!(
            hero.health,
            Knowledge::Known(Health {
                current: 0,
                maximum
            })
        );
        assert!(hero
            .summary
            .contains(&format!("corpse {0}/{0} HP", maximum.div_ceil(4))));
        let rescue = ForecastDisplay::build(
            &snapshot,
            &CombatDisclosure::default(),
            ActorId(5),
            &CombatAction::Rescue { ally: target },
        )
        .expect("rescue forecast");
        let hero = rescue
            .actors
            .iter()
            .find(|a| a.actor == target)
            .expect("hero");
        assert_eq!(hero.outcome, Knowledge::Known(ForecastOutcome::Living));
        assert_eq!(
            hero.health,
            Knowledge::Known(Health {
                current: maximum.div_ceil(4),
                maximum
            })
        );
    }

    #[test]
    fn status_help_explains_effective_ticks_and_modifier_expiry() {
        let mut status = StatusInstance {
            id: 99,
            kind: StatusKind::Bleed,
            bearer: ActorId(1),
            source: ActorId(101),
            potency: 2,
            remaining: 2,
            eligible_boundary: 1,
        };
        let living = status_description(&status, LifeState::Alive);
        assert!(living.contains("2 damage at turn start; up to 2 turn-start ticks if retained"));
        let dying = status_description(&status, LifeState::Dying { failures: 1 });
        assert!(dying.contains("initiative slot; it chooses no action"));
        let corpse = status_description(
            &status,
            LifeState::Corpse {
                hp: 3,
                max_hp: 5,
                created_round: 1,
            },
        );
        assert!(corpse.contains("2 damage at round end; up to 2 round-end ticks if retained"));
        assert!(!corpse.contains("at turn start"));
        status.kind = StatusKind::Brace;
        let modifier = status_description(&status, LifeState::Alive);
        assert!(modifier.contains("expires after 2 turn-start boundaries"));
        assert!(!modifier.contains("damage at"));
    }

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
    fn lethal_forecast_never_shows_corpse_health_as_living_healing() {
        let mut snapshot = Combat::new(42, DEFAULT_HERO_ROSTER)
            .expect("combat")
            .snapshot();
        snapshot
            .actors
            .iter_mut()
            .find(|a| a.id == ActorId(101))
            .expect("enemy")
            .hp = 1;
        let forecast = ForecastDisplay::build(
            &snapshot,
            &CombatDisclosure::default(),
            ActorId(1),
            &CombatAction::Skill {
                skill: SkillId::FrontStrike,
                target: ActorId(101),
            },
        )
        .expect("lethal forecast");
        let enemy = forecast
            .actors
            .iter()
            .find(|a| a.actor == ActorId(101))
            .expect("target");
        assert_eq!(
            enemy.health,
            Knowledge::Known(Health {
                current: 0,
                maximum: 20
            })
        );
        assert!(enemy.summary.contains("corpse 5/5 HP"));
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
