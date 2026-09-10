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
            format!("{} | {} | {state}", player.name, player.hero.name()),
            UiTextRole::Body,
        );
    }
    label(
        world,
        lobby,
        "Hero Choice Title",
        "Choose your class | classes may repeat; each player owns one hero",
        UiTextRole::Supporting,
    );
    let roles = row(world, lobby, "Hero Choices");
    for hero in HeroClass::ALL {
        control(
            world,
            roles,
            format!("Choose {hero:?}"),
            hero.name(),
            Action::Hero(hero),
            !view.admitted,
        );
    }
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
                .all(|player| player.occupied && player.connected && player.ready);
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
