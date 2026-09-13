//! Squad preparation and scenario controls; every character uses the unified editor.

use super::*;

#[cfg(test)]
#[path = "lobby_tests.rs"]
mod tests;

pub(super) fn lobby(world: &mut World, parent: Entity, view: &LabyrinthView, ui: &mut UiState) {
    let lobby = surface(world, parent, "Company Lobby");
    world.entity_mut(lobby).insert(Node {
        width: Val::Percent(100.0),
        flex_grow: 1.0,
        flex_shrink: 1.0,
        min_height: Val::Px(0.0),
        flex_direction: FlexDirection::Column,
        row_gap: Val::Px(12.0),
        padding: UiRect::all(Val::Px(16.0)),
        ..default()
    });
    label(
        world,
        lobby,
        "Lobby Title",
        "Prepare your company",
        UiTextRole::Title,
    );
    if let Some(scenario) = &view.scenario {
        label(
            world,
            lobby,
            "Scenario Title",
            format!("{}  ·  Seed {}", scenario.name, scenario.seed),
            UiTextRole::Supporting,
        );
    }
    let tabs = row(world, lobby, "Preparation Navigation");
    for (page, title) in [
        (0, "Party"),
        (1, "Enemies"),
        (2, "Scenario"),
        (3, "Players"),
    ] {
        let active = ui.lobby_page == page;
        let button = control(
            world,
            tabs,
            format!("Preparation {title}"),
            if active {
                format!("{title} · selected")
            } else {
                title.into()
            },
            Action::LobbyPage(page),
            false,
        );
        if active {
            let theme = world.resource::<UiTheme>();
            let overrides = UiSkinOverrides {
                background: Some(theme.control_hovered),
                border: Some(theme.accent),
                ..default()
            };
            world.entity_mut(button).insert(overrides);
        }
    }
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
            row_gap: Val::Px(16.0),
            padding: UiRect::right(Val::Px(8.0)),
            overflow: Overflow::scroll_y(),
            ..default()
        },
    );
    match ui.lobby_page {
        1 => roster(world, body, view, labyrinth_rules::Team::Enemies),
        2 => scenario_controls(world, body, view, ui),
        3 => participants(world, body, view),
        _ => roster(world, body, view, labyrinth_rules::Team::Heroes),
    }
    readiness(world, lobby, view);
}

fn roster(world: &mut World, parent: Entity, view: &LabyrinthView, team: labyrinth_rules::Team) {
    let Some(scenario) = &view.scenario else {
        return;
    };
    let roster = if team == labyrinth_rules::Team::Heroes {
        &scenario.heroes
    } else {
        &scenario.enemies
    };
    let used = roster
        .iter()
        .map(|a| usize::from(a.actor.footprint))
        .sum::<usize>();
    label(
        world,
        parent,
        &format!("{team:?} Setup Title"),
        format!(
            "{} · {used}/6 formation spaces",
            if team == labyrinth_rules::Team::Heroes {
                "Party"
            } else {
                "Enemies"
            }
        ),
        UiTextRole::Body,
    );
    label(
        world,
        parent,
        "Formation Advice",
        "Front to back, in rank order. Open a character to inspect its moves and customize its build.",
        UiTextRole::Supporting,
    );
    let metrics = *world.resource::<ResolvedUiMetrics>();
    let columns = match metrics.viewport {
        UiViewportClass::Compact => 1,
        _ => 3,
    };
    let grid = column(
        world,
        parent,
        "Formation Cards",
        Node {
            display: Display::Grid,
            width: Val::Percent(100.0),
            flex_shrink: 0.0,
            grid_template_columns: RepeatedGridTrack::flex(columns, 1.0),
            column_gap: Val::Px(12.0),
            row_gap: Val::Px(12.0),
            ..default()
        },
    );
    let mut rank = 1;
    for (index, actor) in roster.iter().enumerate() {
        let card = surface(world, grid, &format!("Actor {} Setup", actor.id.0));
        if let Some(mut node) = world.get_mut::<Node>(card) {
            node.padding = UiRect::all(Val::Px(12.0));
            node.row_gap = Val::Px(8.0);
            node.min_width = Val::Px(0.0);
        }
        let end = rank + usize::from(actor.actor.footprint) - 1;
        let ranks = if end == rank {
            format!("Rank {rank}")
        } else {
            format!("Ranks {rank}–{end}")
        };
        rank = end + 1;
        label(
            world,
            card,
            &format!("Actor {} Rank", actor.id.0),
            ranks,
            UiTextRole::Supporting,
        );
        label(
            world,
            card,
            &format!("Actor {} Setup Label", actor.id.0),
            &actor.actor.name,
            UiTextRole::Title,
        );
        let owner = view.company.iter().find(|member| member.actor == actor.id);
        let controller = if team == labyrinth_rules::Team::Enemies {
            format!(
                "{} · host configures",
                match actor.controller {
                    labyrinth_rules::scenario::ControllerPolicy::Ai => "AI controlled",
                    labyrinth_rules::scenario::ControllerPolicy::Manual => "Manual controller",
                    labyrinth_rules::scenario::ControllerPolicy::External => "External controller",
                }
            )
        } else {
            owner
                .and_then(|member| view.players.iter().find(|p| p.slot == member.owner))
                .map_or_else(
                    || "Host controlled".into(),
                    |player| format!("Controlled by {}", player.name),
                )
        };
        label(
            world,
            card,
            &format!("Actor {} Controller Label", actor.id.0),
            controller,
            UiTextRole::Supporting,
        );
        let weapon = actor.actor.build.weapon.as_ref().and_then(|id| {
            view.catalog
                .as_ref()?
                .definition()
                .weapons
                .iter()
                .find(|w| &w.id == id)
        });
        label(
            world,
            card,
            &format!("Actor {} Equipment", actor.id.0),
            format!(
                "{}  ·  {} HP",
                weapon.map_or("Unarmed", |w| w.name.as_str()),
                actor.actor.max_hp
            ),
            UiTextRole::Body,
        );
        let editable = view.host || owner.is_some_and(|m| Some(m.owner) == view.player);
        control(
            world,
            card,
            format!("Edit Actor {}", actor.id.0),
            if editable {
                "Open character"
            } else {
                "Assigned to another player"
            },
            Action::Setup(setup::SetupAction::Edit(actor.id)),
            !editable,
        );
        if view.host {
            let order = row(
                world,
                card,
                &format!("Actor {} Formation Actions", actor.id.0),
            );
            control(
                world,
                order,
                format!("Move Actor {} Forward", actor.id.0),
                "Forward",
                Action::Setup(setup::SetupAction::Move(actor.id, -1)),
                index == 0,
            );
            control(
                world,
                order,
                format!("Move Actor {} Back", actor.id.0),
                "Back",
                Action::Setup(setup::SetupAction::Move(actor.id, 1)),
                index + 1 == roster.len(),
            );
            control(
                world,
                order,
                format!("Remove Actor {}", actor.id.0),
                "Remove",
                Action::Setup(setup::SetupAction::Remove(actor.id)),
                roster.len() == 1,
            );
        }
    }
    if view.host {
        control(
            world,
            parent,
            format!("Add {team:?}"),
            format!(
                "Add {} character",
                if team == labyrinth_rules::Team::Heroes {
                    "party"
                } else {
                    "enemy"
                }
            ),
            Action::Setup(setup::SetupAction::Add(team)),
            used >= 6,
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
        "Loading a stock or saved encounter replaces both formations. Save your current setup to keep it. Character changes use Open character in Party or Enemies.",
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
        "Players & assignments",
        UiTextRole::Title,
    );
    label(
        world,
        parent,
        "Assignment Advice",
        "A player may control several characters or spectate. The host assigns characters; formation rank is configured in Party.",
        UiTextRole::Supporting,
    );
    for player in &view.players {
        let state = if !player.occupied {
            "Open slot"
        } else if !player.connected {
            "Disconnected · reserved"
        } else if player.actors.is_empty() {
            "Spectating"
        } else if player.ready {
            "Ready"
        } else {
            "Not ready"
        };
        label(
            world,
            parent,
            &format!("Player {}", player.slot),
            format!(
                "{} · {} characters · {state}",
                player.name,
                player.actors.len()
            ),
            UiTextRole::Body,
        );
    }
    assignments(world, parent, view);
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

fn readiness(world: &mut World, parent: Entity, view: &LabyrinthView) {
    let current = view
        .players
        .iter()
        .find(|player| Some(player.slot) == view.player);
    let ready = current.is_some_and(|p| p.ready);
    let spectator = current.is_none_or(|p| p.actors.is_empty());
    let required = view
        .players
        .iter()
        .filter(|p| p.occupied && !p.actors.is_empty())
        .collect::<Vec<_>>();
    let waiting = required
        .iter()
        .filter(|p| !p.connected || !p.ready)
        .map(|p| p.name.as_str())
        .collect::<Vec<_>>();
    let can_start = !view.players.is_empty() && waiting.is_empty();
    label(
        world,
        parent,
        "Readiness Summary",
        if waiting.is_empty() {
            "Company ready".into()
        } else {
            format!("Waiting for {}", waiting.join(", "))
        },
        UiTextRole::Supporting,
    );
    let actions = row(world, parent, "Lobby Actions");
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
        control(
            world,
            actions,
            "Start Encounter",
            "Start battle",
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
