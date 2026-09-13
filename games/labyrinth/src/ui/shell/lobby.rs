//! Local company readiness, role choices and private invitation controls.

use super::*;

pub(super) fn lobby(world: &mut World, parent: Entity, view: &LabyrinthView, ui: &mut UiState) {
    let lobby = surface(world, parent, "Company Lobby");
    label(
        world,
        lobby,
        "Lobby Title",
        if view.session_name.is_empty() {
            "YOUR COMPANY".to_owned()
        } else {
            view.session_name.to_uppercase()
        },
        UiTextRole::Title,
    );
    for player in &view.players {
        let state = if !player.occupied {
            "OPEN"
        } else if !player.connected {
            "RESERVED | disconnected"
        } else if player.ready {
            "READY"
        } else {
            "NOT READY"
        };
        label(
            world,
            lobby,
            &format!("Player {}", player.slot),
            format!(
                "{} | {} | {state}",
                player.name,
                if player.actors.is_empty() {
                    "SPECTATOR".to_owned()
                } else {
                    format!("{} characters", player.actors.len())
                }
            ),
            UiTextRole::Body,
        );
    }
    if let Some(scenario) = &view.scenario {
        label(
            world,
            lobby,
            "Scenario Title",
            format!("{} · seed {}", scenario.name, scenario.seed),
            UiTextRole::Title,
        );
        let stocks = row(world, lobby, "Stock Encounters");
        for (index, name) in [
            "Prototype",
            "Weapon comparison",
            "Two-rank cleave",
            "Rescue and status",
        ]
        .into_iter()
        .enumerate()
        {
            control(
                world,
                stocks,
                format!("Stock Scenario {index}"),
                name,
                Action::Setup(setup::SetupAction::Stock(index)),
                !view.host,
            );
        }
        forms::field(
            world,
            lobby,
            "Scenario seed",
            Field::ScenarioSeed,
            &ui.scenario_seed,
            20,
        );
        control(
            world,
            lobby,
            "Apply Scenario Seed",
            "Apply seed",
            Action::Setup(setup::SetupAction::ApplySeed),
            !view.host,
        );
        forms::field(
            world,
            lobby,
            "Scenario file (blank = labyrinth-scenario.json)",
            Field::ScenarioPath,
            &ui.scenario_path,
            1024,
        );
        let files = row(world, lobby, "Scenario Files");
        control(
            world,
            files,
            "Save Scenario",
            "Save setup",
            Action::Setup(setup::SetupAction::SaveFile),
            false,
        );
        control(
            world,
            files,
            "Load Scenario",
            "Load setup",
            Action::Setup(setup::SetupAction::LoadFile),
            !view.host,
        );
        for (team, roster) in [
            (labyrinth_rules::Team::Heroes, &scenario.heroes),
            (labyrinth_rules::Team::Enemies, &scenario.enemies),
        ] {
            let used = roster
                .iter()
                .map(|a| usize::from(a.actor.footprint))
                .sum::<usize>();
            label(
                world,
                lobby,
                &format!("{team:?} Setup Title"),
                format!("{team:?} · {used}/6 spaces · front to back"),
                UiTextRole::Body,
            );
            let mut rank = 1_usize;
            for (index, actor) in roster.iter().enumerate() {
                let row = row(world, lobby, &format!("Actor {} Setup", actor.id.0));
                let editable = view.host
                    || view
                        .company
                        .iter()
                        .any(|m| m.actor == actor.id && Some(m.owner) == view.player);
                label(
                    world,
                    row,
                    &format!("Actor {} Setup Label", actor.id.0),
                    format!(
                        "Ranks {}–{} · {} · HP {} · speed {}",
                        rank,
                        rank + usize::from(actor.actor.footprint) - 1,
                        actor.actor.name,
                        actor.actor.max_hp,
                        actor.actor.base_speed
                    ),
                    UiTextRole::Body,
                );
                rank += usize::from(actor.actor.footprint);
                control(
                    world,
                    row,
                    format!("Edit Actor {}", actor.id.0),
                    "Edit character",
                    Action::Setup(setup::SetupAction::Edit(actor.id)),
                    !editable,
                );
                control(
                    world,
                    row,
                    format!("Move Actor {} Forward", actor.id.0),
                    "Forward",
                    Action::Setup(setup::SetupAction::Move(actor.id, -1)),
                    !view.host || index == 0,
                );
                control(
                    world,
                    row,
                    format!("Move Actor {} Back", actor.id.0),
                    "Back",
                    Action::Setup(setup::SetupAction::Move(actor.id, 1)),
                    !view.host || index + 1 == roster.len(),
                );
                control(
                    world,
                    row,
                    format!("Remove Actor {}", actor.id.0),
                    "Remove",
                    Action::Setup(setup::SetupAction::Remove(actor.id)),
                    !view.host || roster.len() == 1,
                );
            }
            control(
                world,
                lobby,
                format!("Add {team:?}"),
                format!("Add {team:?} character"),
                Action::Setup(setup::SetupAction::Add(team)),
                !view.host || used >= 6,
            );
        }
    }
    assignments(world, lobby, view);
    let ready = view
        .players
        .iter()
        .find(|player| Some(player.slot) == view.player)
        .is_some_and(|player| player.ready);
    let spectator = view
        .players
        .iter()
        .find(|p| Some(p.slot) == view.player)
        .is_none_or(|p| p.actors.is_empty());
    let actions = row(world, lobby, "Lobby Actions");
    control(
        world,
        actions,
        "Toggle Ready",
        if spectator {
            "Spectating"
        } else if ready {
            "Not ready"
        } else {
            "Ready"
        },
        Action::Ready(!ready),
        !view.admitted || spectator,
    );
    if view.host {
        let can_start = !view.players.is_empty()
            && view
                .players
                .iter()
                .filter(|player| player.occupied && !player.actors.is_empty())
                .all(|player| player.connected && player.ready);
        control(
            world,
            actions,
            "Start Encounter",
            "Enter the breach",
            Action::Start,
            !can_start,
        );
    }
    control(
        world,
        actions,
        "Lobby Settings",
        "Readability & motion",
        Action::Settings,
        false,
    );
    control(
        world,
        actions,
        "Lobby Leave",
        "Return to menu",
        Action::Leave,
        false,
    );
    if view.host {
        let invites = surface(world, parent, "Private Invitations");
        label(
            world,
            invites,
            "Invite Title",
            "PRIVATE INVITATIONS | UP TO SIX PLAYERS",
            UiTextRole::Title,
        );
        label(world, invites, "Invite Advice", "Copy one distinct invitation for each friend. Codes stay out of the screen and diagnostics.", UiTextRole::Supporting);
        for (index, invite) in view.invite_labels.iter().enumerate() {
            let row = row(world, invites, &format!("Invitation {index}"));
            label(
                world,
                row,
                &format!("Invitation {index} Label"),
                invite.clone(),
                UiTextRole::Body,
            );
            control(
                world,
                row,
                format!("Copy Invitation {index}"),
                "Copy private code",
                Action::Copy(index),
                false,
            );
            control(
                world,
                row,
                format!("Reissue Invitation {index}"),
                "Replace unused code",
                Action::Reissue(index),
                false,
            );
        }
    }
}

/// Stable actor/participant keys survive admission updates and controller changes.
pub(super) fn assignments(world: &mut World, parent: Entity, view: &LabyrinthView) {
    for member in &view.company {
        if view.combat.as_ref().is_some_and(|combat| {
            combat
                .actor(member.actor)
                .is_none_or(|a| !a.standing() && !a.dying())
        }) {
            continue;
        }
        let controls = row(
            world,
            parent,
            &format!("Character {} Controller", member.actor.0),
        );
        label(
            world,
            controls,
            &format!("Character {} Owner", member.actor.0),
            format!(
                "{} · controller",
                view.scenario
                    .as_ref()
                    .and_then(|s| s.heroes.iter().find(|a| a.id == member.actor))
                    .map_or(member.hero.name(), |a| a.actor.name.as_str())
            ),
            UiTextRole::Supporting,
        );
        for player in view.players.iter().filter(|p| p.occupied) {
            control(
                world,
                controls,
                format!("Assign {} To {}", member.actor.0, player.slot),
                format!(
                    "{}{}",
                    player.name,
                    if member.owner == player.slot {
                        " ✓"
                    } else {
                        ""
                    }
                ),
                Action::Assign(member.actor, player.slot),
                !view.host || !player.connected || member.owner == player.slot,
            );
        }
    }
}
