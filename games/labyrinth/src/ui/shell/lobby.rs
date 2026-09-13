//! Local company readiness, role choices and private invitation controls.

use super::*;

pub(super) fn lobby(world: &mut World, parent: Entity, view: &LabyrinthView) {
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
    for member in &view.company {
        let section = surface(world, lobby, &format!("Character {}", member.actor.0));
        label(
            world,
            section,
            &format!("Character {} Label", member.actor.0),
            format!("{} · character {}", member.hero.name(), member.actor.0),
            UiTextRole::Body,
        );
        let roles = row(
            world,
            section,
            &format!("Character {} Builds", member.actor.0),
        );
        for hero in HeroClass::ALL {
            control(
                world,
                roles,
                format!("Character {} Choose {hero:?}", member.actor.0),
                hero.name(),
                Action::Hero(member.actor, hero),
                !view.admitted || (!view.host && Some(member.owner) != view.player),
            );
        }
    }
    assignments(world, lobby, view);
    let ready = view
        .players
        .iter()
        .find(|player| Some(player.slot) == view.player)
        .is_some_and(|player| player.ready);
    let actions = row(world, lobby, "Lobby Actions");
    control(
        world,
        actions,
        "Toggle Ready",
        if ready { "Not ready" } else { "Ready" },
        Action::Ready(!ready),
        !view.admitted,
    );
    if view.host {
        let can_start = !view.players.is_empty()
            && view
                .players
                .iter()
                .filter(|player| player.occupied)
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
            format!("{} · controller", member.hero.name()),
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
