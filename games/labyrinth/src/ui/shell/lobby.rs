//! Squad preparation and scenario controls; every character uses the unified editor.

use super::*;

#[cfg(test)]
#[path = "lobby_tests.rs"]
mod tests;

pub(super) fn lobby(world: &mut World, parent: Entity, view: &LabyrinthView, ui: &mut UiState) {
    let lobby = column(
        world,
        parent,
        "Company Lobby",
        Node {
            width: Val::Percent(100.0),
            flex_grow: 1.0,
            flex_shrink: 1.0,
            min_height: Val::Px(0.0),
            flex_direction: FlexDirection::Column,
            row_gap: Val::Px(8.0),
            ..default()
        },
    );
    let header = row(world, lobby, "Preparation Header");
    let title = label(
        world,
        header,
        "Lobby Title",
        "Prepare for battle",
        UiTextRole::Title,
    );
    world.entity_mut(title).insert(Node {
        flex_grow: 1.0,
        ..default()
    });
    control(
        world,
        header,
        "Preparation Scenario",
        if ui.lobby_page == 2 {
            "Back to formation"
        } else {
            "Scenario"
        },
        Action::LobbyPage(2),
        false,
    );
    if !view.local {
        control(
            world,
            header,
            "Preparation Lobby",
            if ui.lobby_page == 3 {
                "Back to formation"
            } else {
                "Lobby"
            },
            Action::LobbyPage(3),
            false,
        );
    }
    control(
        world,
        header,
        "Lobby Settings",
        "Settings",
        Action::Settings,
        false,
    );
    control(world, header, "Lobby Leave", "Leave", Action::Leave, false);
    player_strip(world, lobby, view);
    let body = column(
        world,
        lobby,
        "Preparation Content",
        Node {
            width: Val::Percent(100.0),
            min_height: Val::Px(0.0),
            flex_grow: 1.0,
            flex_basis: Val::Px(0.0),
            flex_direction: FlexDirection::Column,
            row_gap: Val::Px(8.0),
            padding: UiRect::right(Val::Px(8.0)),
            overflow: if ui.lobby_page == 0 {
                Overflow::clip()
            } else {
                Overflow::scroll_y()
            },
            ..default()
        },
    );
    match ui.lobby_page {
        2 => scenario_controls(world, body, view, ui),
        3 => participants(world, body, view),
        _ => {
            constructor::board(world, body, view, ui);
            let context = column(
                world,
                body,
                "Construction Detail Scroll",
                Node {
                    width: Val::Percent(100.0),
                    min_height: Val::Px(0.0),
                    flex_grow: 1.0,
                    flex_basis: Val::Px(0.0),
                    flex_direction: FlexDirection::Column,
                    overflow: Overflow::scroll_y(),
                    ..default()
                },
            );
            constructor::context(world, context, view, ui);
        }
    }
    readiness(world, lobby, view, ui);
}

fn player_strip(world: &mut World, parent: Entity, view: &LabyrinthView) {
    let strip = column(
        world,
        parent,
        "Company Player Strip",
        Node {
            width: Val::Percent(100.0),
            flex_direction: FlexDirection::Row,
            column_gap: Val::Px(20.0),
            overflow: Overflow::scroll_x(),
            flex_shrink: 0.0,
            padding: UiRect::vertical(Val::Px(4.0)),
            ..default()
        },
    );
    if view.local {
        label(
            world,
            strip,
            "Player 0",
            "You · Whole company",
            UiTextRole::Supporting,
        );
        if let Some(scenario) = &view.scenario {
            label(
                world,
                strip,
                "Scenario Title",
                format!("{} · Seed {}", scenario.name, scenario.seed),
                UiTextRole::Supporting,
            );
        }
        return;
    }
    for player in view.players.iter().filter(|p| p.occupied) {
        let entry = column(
            world,
            strip,
            &format!("Player {} Summary", player.slot),
            Node {
                flex_grow: 1.0,
                flex_basis: Val::Px(0.0),
                min_width: Val::Px(150.0),
                flex_shrink: 0.0,
                flex_direction: FlexDirection::Column,
                ..default()
            },
        );
        label(
            world,
            entry,
            &format!("Player {}", player.slot),
            format!("P{} · {}", player.slot + 1, player.name),
            UiTextRole::Body,
        );
        label(
            world,
            entry,
            &format!("Player {} State", player.slot),
            format!(
                "{} · {}",
                if !player.connected {
                    "Offline"
                } else if player.actors.is_empty() {
                    "Spectating"
                } else if player.ready {
                    "Ready"
                } else {
                    "Preparing"
                },
                if player.actors.is_empty() {
                    "No characters".to_owned()
                } else {
                    format!(
                        "{} {}",
                        player.actors.len(),
                        if player.actors.len() == 1 {
                            "character"
                        } else {
                            "characters"
                        }
                    )
                }
            ),
            UiTextRole::Supporting,
        );
    }
}

fn scenario_controls(world: &mut World, parent: Entity, view: &LabyrinthView, ui: &UiState) {
    label(
        world,
        parent,
        "Stock Title",
        "Stock test encounters",
        UiTextRole::Title,
    );
    label(
        world,
        parent,
        "Stock Advice",
        "Loading a stock or saved encounter replaces both formations. Save your current setup to keep it. Select a character on the formation and choose Customize to edit its build.",
        UiTextRole::Supporting,
    );
    for (index, title, description) in [
        (
            0,
            "Prototype",
            "The original encounter, adapted to configurable builds.",
        ),
        (
            1,
            "Weapon comparison",
            "Compare the starter weapons and their different movesets.",
        ),
        (
            2,
            "Two-rank cleave",
            "Exercise front-pair attacks and creatures occupying multiple ranks.",
        ),
        (
            3,
            "Rescue and status",
            "Exercise rescue and starting conditions.",
        ),
    ] {
        let preset = row(world, parent, &format!("Stock Encounter {index}"));
        control(
            world,
            preset,
            format!("Stock Scenario {index}"),
            format!("Load {title}"),
            Action::Setup(setup::SetupAction::Stock(index)),
            !view.host,
        );
        label(
            world,
            preset,
            &format!("Stock Description {index}"),
            description,
            UiTextRole::Supporting,
        );
    }
    label(
        world,
        parent,
        "Reproducibility Title",
        "Repeat a test",
        UiTextRole::Title,
    );
    label(
        world,
        parent,
        "Reproducibility Advice",
        "The same setup, content, seed and actions reproduce the same battle. Rematch keeps the seed.",
        UiTextRole::Supporting,
    );
    forms::field(
        world,
        parent,
        "Scenario seed",
        Field::ScenarioSeed,
        &ui.scenario_seed,
        20,
    );
    control(
        world,
        parent,
        "Apply Scenario Seed",
        "Apply seed",
        Action::Setup(setup::SetupAction::ApplySeed),
        !view.host,
    );
    forms::field(
        world,
        parent,
        "Scenario file (blank = labyrinth-scenario.json)",
        Field::ScenarioPath,
        &ui.scenario_path,
        1024,
    );
    let files = row(world, parent, "Scenario Files");
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
}

fn participants(world: &mut World, parent: Entity, view: &LabyrinthView) {
    label(
        world,
        parent,
        "Players Title",
        "Invitations & connection",
        UiTextRole::Title,
    );
    label(world, parent, "Assignment Advice", "Select a place on the formation to assign its player. Each player can choose and customize their own characters, or spectate.", UiTextRole::Supporting);
    if !view.host {
        return;
    }
    label(
        world,
        parent,
        "Invite Title",
        "Private invitations",
        UiTextRole::Title,
    );
    label(
        world,
        parent,
        "Invite Advice",
        "Copy one distinct invitation for each friend. Codes stay out of the screen and diagnostics.",
        UiTextRole::Supporting,
    );
    for (index, invite) in view.invite_labels.iter().enumerate() {
        let entry = row(world, parent, &format!("Invitation {index}"));
        label(
            world,
            entry,
            &format!("Invitation {index} Label"),
            invite,
            UiTextRole::Body,
        );
        control(
            world,
            entry,
            format!("Copy Invitation {index}"),
            "Copy private code",
            Action::Copy(index),
            false,
        );
        control(
            world,
            entry,
            format!("Reissue Invitation {index}"),
            "Replace unused code",
            Action::Reissue(index),
            false,
        );
    }
}

fn readiness(world: &mut World, parent: Entity, view: &LabyrinthView, ui: &UiState) {
    let current = view.players.iter().find(|p| Some(p.slot) == view.player);
    let ready = current.is_some_and(|p| p.ready);
    let spectator = current.is_none_or(|p| p.actors.is_empty());
    let waiting = view
        .players
        .iter()
        .filter(|p| p.occupied && !p.actors.is_empty() && (!p.connected || !p.ready))
        .map(|p| p.name.as_str())
        .collect::<Vec<_>>();
    let error = view.deployment_error.clone().or_else(|| {
        view.scenario
            .as_ref()
            .and_then(|s| constructor::formation(view).deployment_error(s))
    });
    let footer = row(world, parent, "Lobby Actions");
    world
        .entity_mut(footer)
        .insert(BackgroundColor(Color::srgba(0.025, 0.04, 0.04, 0.96)));
    let text = label(
        world,
        footer,
        "Readiness Summary",
        if let Some(error) = &error {
            constructor::deployment_summary(view).unwrap_or_else(|| error.clone())
        } else if waiting.is_empty() || view.local {
            "Formation ready".into()
        } else {
            format!("Waiting for {}", waiting.join(", "))
        },
        UiTextRole::Supporting,
    );
    world.entity_mut(text).insert(Node {
        flex_grow: 1.0,
        flex_basis: Val::Px(200.0),
        min_width: Val::Px(0.0),
        ..default()
    });
    if ui.lobby_page == 0 {
        constructor::commit(world, footer, view, ui);
    }
    if !view.local {
        control(
            world,
            footer,
            "Toggle Ready",
            if spectator {
                "Spectating"
            } else if ready {
                "Not ready"
            } else {
                "Ready"
            },
            Action::Ready(!ready),
            !view.admitted || spectator || error.is_some(),
        );
    }
    if view.host {
        let start = control(
            world,
            footer,
            "Start Encounter",
            "Deploy",
            Action::Start,
            error.is_some() || (!view.local && (view.players.is_empty() || !waiting.is_empty())),
        );
        world.entity_mut(start).insert(UiSkinOverrides {
            background: Some(if ui.constructor.selection.is_none() {
                Color::srgb(0.26, 0.25, 0.13)
            } else {
                Color::NONE
            }),
            disabled: Some(Color::srgb(0.07, 0.08, 0.08)),
            border: Some(if error.is_some() {
                Color::srgb(0.23, 0.26, 0.25)
            } else {
                Color::srgb(0.60, 0.55, 0.35)
            }),
            ..default()
        });
        if error.is_some() {
            let children = world
                .get::<Children>(start)
                .map(|c| c.iter().collect::<Vec<_>>())
                .unwrap_or_default();
            for child in children {
                world.entity_mut(child).insert(UiSkinOverrides {
                    text: Some(Color::srgb(0.45, 0.48, 0.46)),
                    ..default()
                });
            }
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
