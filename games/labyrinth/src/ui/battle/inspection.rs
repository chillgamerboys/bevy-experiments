//! Local action inspection and legality explanations; never mutates authority.

use super::*;

pub(crate) fn select_skill_slot(view: &LabyrinthView, ui: &mut UiState, index: usize) {
    ui.inspected_status = None;
    if let Some(actor) = display_actor(view) {
        if let Some(skill) = actor.skills().get(index) {
            ui.selected = Some(Choice::Skill(*skill));
        }
    }
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
            .actors
            .iter()
            .find(|actor| actor.kind == ActorKind::Hero(player.hero))
    }
}

pub(super) fn action_for(choice: Choice, target: Option<ActorId>) -> Result<CombatAction, String> {
    let target = || target.ok_or_else(|| "Choose an actor tile as the target.".to_owned());
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
    (1..=4)
        .map(|rank| {
            if mask & (1 << (rank - 1)) != 0 {
                rank.to_string()
            } else {
                "-".to_owned()
            }
        })
        .collect::<Vec<_>>()
        .join(" ")
}

pub(super) fn slot_value(
    slot: Slot,
    view: &LabyrinthView,
    ui: &UiState,
    snapshot: &CombatSnapshot,
    viewport: UiViewportClass,
) -> String {
    match slot {
        Slot::Hud => {
            let actor = snapshot
                .active_actor
                .and_then(|id| snapshot.actor(id))
                .map_or("No actor", ActorSnapshot::name);
            let ending = snapshot.outcome.map(|outcome| match outcome {
                CombatOutcome::Victory => "VICTORY",
                CombatOutcome::Defeat => "DEFEAT",
            });
            let mode = if view.local {
                "LOCAL | all four heroes".to_owned()
            } else {
                format!(
                    "CO-OP | {}/4 connected",
                    view.players
                        .iter()
                        .filter(|player| player.connected)
                        .count()
                )
            };
            if viewport == UiViewportClass::Compact {
                format!(
                    "{} | R{} | {}",
                    if view.local { "LOCAL" } else { "CO-OP" },
                    snapshot.round,
                    ending.unwrap_or(actor)
                )
            } else {
                format!(
                    "LABYRINTH | {mode}\nRound {} | {}",
                    snapshot.round,
                    ending.unwrap_or(actor)
                )
            }
        }
        Slot::Timeline => timeline::value(snapshot, ui, viewport),
        Slot::Selected => {
            if let Some((actor, id)) = ui.inspected_status {
                if let Some(status) = snapshot
                    .actor(actor)
                    .and_then(|actor| actor.statuses.iter().find(|status| status.id == id))
                {
                    return format!(
                        "{} | potency {} | {} boundaries left. {}",
                        status_definition(status.kind).name,
                        status.potency,
                        status.remaining,
                        status_definition(status.kind).description
                    );
                }
            }
            match ui.selected {
            Some(Choice::Skill(skill)) => {
                let definition = skill_definition(skill);
                format!("{} | src [{}] tgt [{}]\n{}", definition.name, rank_mask(definition.source_ranks), rank_mask(definition.target_ranks), definition.description)
            }
            Some(Choice::Reposition) => "MOVE | swap with an adjacent ally, including a downed ally. Costs this turn.".to_owned(),
            Some(Choice::Rescue) => "RESCUE | revive any downed ally at 25% maximum HP. Costs this turn.".to_owned(),
            Some(Choice::Defend) => "DEFEND | Brace reduces direct damage by 2 until your next turn starts; not bleed.".to_owned(),
            Some(Choice::Wait) => "WAIT | spend this turn without another effect.".to_owned(),
            None => "Inspect a skill (1-4), select an actor tile, then Confirm. Inspection never spends a turn.".to_owned(),
        }
        }
        Slot::Reason => {
            let target = ui
                .target
                .and_then(|id| snapshot.actor(id))
                .map_or("none", ActorSnapshot::name);
            let reason = selected_action(view, ui).map_or_else(
                |reason| reason,
                |_| "Legal action | ready to confirm".to_owned(),
            );
            format!("Target: {target} | {reason}")
        }
        Slot::Inspector => {
            let Some(actor) = ui
                .inspected
                .and_then(|id| snapshot.actor(id))
                .or_else(|| display_actor(view))
            else {
                return "Choose an actor or a status badge to inspect it.".to_owned();
            };
            let mut value = format!("{} | {} / {} HP | Speed {}\nRank is position, not ownership. Heroes at 0 HP are downed and can be rescued.", actor.name(), actor.hp, actor.max_hp, actor.speed());
            for status in &actor.statuses {
                if ui.inspected_status.is_none_or(|(_, id)| id == status.id) {
                    let definition = status_definition(status.kind);
                    value.push_str(&format!(
                        "\n{} | potency {} | {} boundaries left. {}",
                        definition.name, status.potency, status.remaining, definition.description
                    ));
                }
            }
            value
        }
        Slot::Log => {
            if view.log.is_empty() {
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
        Slot::Skill(index) => display_actor(view)
            .and_then(|actor| actor.skills().get(index).map(|skill| (actor, *skill)))
            .map_or_else(
                || "No skill".to_owned(),
                |(actor, skill)| {
                    let selected = if ui.selected == Some(Choice::Skill(skill)) {
                        " | selected"
                    } else {
                        ""
                    };
                    let uses = actor
                        .remaining_uses(skill)
                        .map_or_else(String::new, |remaining| format!(" | {remaining} uses"));
                    format!(
                        "{}. {}{uses}{selected}",
                        index + 1,
                        skill_definition(skill).name
                    )
                },
            ),
    }
}

pub(super) fn paint_choices(world: &mut World, view: &LabyrinthView, ui: &UiState) {
    let actor = display_actor(view);
    let controls = world
        .query::<(Entity, &Action)>()
        .iter(world)
        .filter_map(|(entity, action)| {
            let choice = match action {
                Action::Choice(choice) => Some(*choice),
                Action::SkillSlot(index) => actor
                    .and_then(|actor| actor.skills().get(*index))
                    .copied()
                    .map(Choice::Skill),
                _ => return None,
            };
            Some((entity, choice.is_some() && choice == ui.selected))
        })
        .collect::<Vec<_>>();
    for (entity, selected) in controls {
        let wanted = UiSkinOverrides {
            background: selected.then_some(Color::srgb(0.22, 0.23, 0.14)),
            border: selected.then_some(Color::srgb(0.89, 0.75, 0.43)),
            ..default()
        };
        if world.get::<UiSkinOverrides>(entity) != Some(&wanted) {
            world.entity_mut(entity).insert(wanted);
        }
    }
}
