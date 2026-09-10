//! Local action inspection and legality explanations; never mutates authority.

use super::*;
use crate::presentation::{BattlePresentation, CombatDisclosure, ForecastDisplay};

pub(crate) fn select_skill_slot(view: &LabyrinthView, ui: &mut UiState, index: usize) {
    ui.inspected_status = None;
    if let Some(actor) = display_actor(view) {
        if let Some(skill) = actor.skills().get(index) {
            ui.selected = Some(Choice::Skill(*skill));
        }
    }
}

pub(crate) fn skills_disclosed(view: &LabyrinthView, disclosure: &CombatDisclosure) -> bool {
    display_actor(view).is_some_and(|actor| {
        let policy = disclosure.actor(actor.id);
        policy.details && policy.statuses
    })
}

pub(super) fn display_actor(view: &LabyrinthView) -> Option<&ActorSnapshot> {
    let snapshot = view.combat.as_ref()?;
    if view.local {
        snapshot
            .active_actor
            .and_then(|id| snapshot.actor(id))
            .filter(|actor| actor.team() == Team::Heroes)
            .or_else(|| {
                snapshot
                    .actors
                    .iter()
                    .find(|actor| actor.team() == Team::Heroes)
            })
    } else {
        let player = view
            .players
            .iter()
            .find(|player| Some(player.slot) == view.player)?;
        snapshot
            .actor(player.actor)
            .filter(|actor| actor.team() == Team::Heroes)
    }
}

pub(super) fn action_for(choice: Choice, target: Option<ActorId>) -> Result<CombatAction, String> {
    let target = || target.ok_or_else(|| "Select a character to target.".to_owned());
    Ok(match choice {
        Choice::Skill(skill) => CombatAction::Skill {
            skill,
            target: target()?,
        },
        Choice::Reposition => CombatAction::Reposition { ally: target()? },
        Choice::Rescue => CombatAction::Rescue { ally: target()? },
        Choice::Defend => CombatAction::Defend,
        Choice::Wait => CombatAction::Wait,
    })
}

pub(crate) fn selected_action(
    view: &LabyrinthView,
    ui: &UiState,
) -> Result<(ActorId, CombatAction), String> {
    if ui.settings || ui.show_inspector || ui.show_log || ui.show_timeline {
        return Err("Close the open details before confirming.".to_owned());
    }
    if view.paused {
        return Err("The company is waiting for a disconnected player.".to_owned());
    }
    if !view.admitted {
        return Err("Admission is not complete.".to_owned());
    }
    let snapshot = view
        .combat
        .as_ref()
        .ok_or_else(|| "No encounter is active.".to_owned())?;
    if snapshot.outcome.is_some() {
        return Err("The encounter is complete.".to_owned());
    }
    if ui.encounter != Some(view.encounter)
        || ui.decision != snapshot.active_actor.map(|actor| (snapshot.turn_id, actor))
    {
        return Err("The decision changed. Inspect and select the action again.".to_owned());
    }
    let actor =
        display_actor(view).ok_or_else(|| "You do not own a hero in this company.".to_owned())?;
    if snapshot.active_actor != Some(actor.id) {
        return Err("Wait for your hero's initiative turn.".to_owned());
    }
    let choice = ui
        .selected
        .ok_or_else(|| "Inspect a skill or choose a universal action.".to_owned())?;
    let action = action_for(choice, ui.target)?;
    snapshot
        .validate_action(actor.id, &action)
        .map_err(|error| error.to_string())?;
    Ok((actor.id, action))
}

pub(super) fn rank_mask(mask: u8) -> String {
    (1..=PARTY_SIZE)
        .map(|rank| {
            if mask & (1 << (rank - 1)) != 0 {
                rank.to_string()
            } else {
                "-".to_owned()
            }
        })
        .collect::<Vec<_>>()
        .join("")
}

pub(super) fn choice_title(choice: Option<Choice>) -> &'static str {
    match choice {
        Some(Choice::Skill(skill)) => skill_definition(skill).name,
        Some(Choice::Reposition) => "Move · swap with an adjacent ally",
        Some(Choice::Rescue) => "Rescue · revive a downed ally",
        Some(Choice::Defend) => "Guard · reduce direct damage by 2",
        Some(Choice::Wait) => "Wait · spend this turn",
        None => "Choose an ability · select a target · confirm",
    }
}

pub(super) fn forecast_display(
    world: &World,
    view: &LabyrinthView,
    ui: &UiState,
) -> Option<ForecastDisplay> {
    let snapshot = view.combat.as_ref()?;
    let actor = display_actor(view)?;
    let action = action_for(ui.selected?, ui.target).ok()?;
    ForecastDisplay::build(
        snapshot,
        world.resource::<CombatDisclosure>(),
        actor.id,
        &action,
    )
    .ok()
}

pub(super) fn slot_value(
    slot: Slot,
    view: &LabyrinthView,
    ui: &UiState,
    snapshot: &CombatSnapshot,
    disclosure: &CombatDisclosure,
) -> String {
    match slot {
        Slot::Hud => {
            let actor = snapshot
                .active_actor
                .and_then(|id| snapshot.actor(id))
                .map_or_else(
                    || "No actor".to_owned(),
                    |actor| format!("{} · {}", actors::token(snapshot, actor), actor.name()),
                );
            let ending = snapshot.outcome.map(|outcome| match outcome {
                CombatOutcome::Victory => "VICTORY",
                CombatOutcome::Defeat => "DEFEAT",
            });
            format!("Round {} · {}", snapshot.round, ending.unwrap_or(&actor))
        }
        Slot::Order => timeline::disclosed_value(snapshot, disclosure, true),
        Slot::Feedback => String::new(),
        Slot::Reason => {
            let target = ui
                .target
                .and_then(|id| snapshot.actor(id))
                .map_or_else(|| "—".to_owned(), |actor| actors::token(snapshot, actor));
            let obscured = display_actor(view).is_some_and(|actor| {
                let policy = disclosure.actor(actor.id);
                !policy.health || !policy.statuses || !policy.details
            }) || ui.target.is_some_and(|actor| {
                let policy = disclosure.actor(actor);
                !policy.health || !policy.statuses || !policy.details
            });
            if obscured {
                return format!("Target: {target} · Outcome uncertain; details concealed");
            }
            let reason = selected_action(view, ui).map_or_else(
                |reason| {
                    if ui.settings || ui.show_inspector || ui.show_log || ui.show_timeline {
                        "Close details to confirm".to_owned()
                    } else if view.paused {
                        "Waiting for reconnection".to_owned()
                    } else if !view.admitted {
                        "Waiting for admission".to_owned()
                    } else if snapshot.outcome.is_some() {
                        "Encounter complete".to_owned()
                    } else if ui.encounter != Some(view.encounter)
                        || ui.decision
                            != snapshot.active_actor.map(|actor| (snapshot.turn_id, actor))
                    {
                        "Decision changed; select again".to_owned()
                    } else if display_actor(view)
                        .is_none_or(|actor| Some(actor.id) != snapshot.active_actor)
                    {
                        "Waiting for your turn".to_owned()
                    } else if ui.selected.is_none() {
                        "Choose an ability".to_owned()
                    } else {
                        reason
                    }
                },
                |_| "Ready to confirm".to_owned(),
            );
            format!("Target: {target} · {reason}")
        }
        Slot::Inspector => {
            let Some(actor) = ui
                .inspected
                .and_then(|id| snapshot.actor(id))
                .or_else(|| display_actor(view))
            else {
                return "Choose an actor or a status badge to inspect it.".to_owned();
            };
            let owner = view
                .players
                .iter()
                .find(|player| player.actor == actor.id)
                .map_or("Host AI", |player| player.name.as_str());
            let presentation = BattlePresentation::new(snapshot, disclosure);
            let Some(presented) = presentation.actor(actor.id) else {
                return String::new();
            };
            let health = presented.health.as_known().map_or_else(
                || "HP unknown".to_owned(),
                |health| format!("{} / {} HP", health.current, health.maximum),
            );
            let speed = presented.details.as_known().map_or_else(
                || "Speed unknown".to_owned(),
                |details| format!("Speed {}", details.speed),
            );
            let mut value = format!(
                "{} · {}\n{health} · {speed}\n{}",
                actors::token(snapshot, actor),
                actor_label(snapshot, actor),
                if view.local && actor.team() == Team::Heroes {
                    "Local control"
                } else {
                    owner
                }
            );
            if let Some(Choice::Skill(skill)) = ui.selected {
                let definition = skill_definition(skill);
                value.push_str(&format!(
                    "\n\n{}\nFrom ranks [{}] → target ranks [{}]\n{}",
                    definition.name,
                    rank_mask(definition.source_ranks),
                    rank_mask(definition.target_ranks),
                    definition.description
                ));
            }
            if let Some(source) = display_actor(view) {
                if let Some(choice) = ui.selected {
                    if let Ok(action) = action_for(choice, ui.target) {
                        if let Ok(forecast) =
                            ForecastDisplay::build(snapshot, disclosure, source.id, &action)
                        {
                            value.push_str(&format!("\n\nACTION PREVIEW\n{}\n{}\nImmediate effects only; future turns are not committed by this preview.", forecast.base, forecast.summary));
                        }
                    }
                }
            }
            if let Some(statuses) = presented.statuses.as_known() {
                for status in statuses {
                    let definition = status_definition(status.kind);
                    value.push_str(&format!(
                        "\n\n{} · potency {} · {} boundaries left. {}",
                        definition.name, status.potency, status.remaining, definition.description
                    ));
                }
            } else {
                value.push_str("\n\nStatus effects unknown.");
            }
            value
        }
        Slot::Log => {
            if disclosure.has_unknown() {
                "Combat details are concealed in this view.".to_owned()
            } else if view.log.is_empty() {
                "No outcomes yet. Combat events appear here in host order.".to_owned()
            } else {
                view.log
                    .iter()
                    .rev()
                    .take(12)
                    .rev()
                    .cloned()
                    .collect::<Vec<_>>()
                    .join("\n")
            }
        }
    }
}

fn actor_label(snapshot: &CombatSnapshot, actor: &ActorSnapshot) -> String {
    let rank = snapshot
        .rank(actor.id)
        .map_or_else(|| "out".to_owned(), |rank| rank.to_string());
    format!("{} | rank {rank} | #{}", actor.name(), actor.id.0)
}
