//! Local menus never mutate encounter suspension or stop network processing.
use super::*;

pub(crate) fn overlays(world: &mut World, view: &LabyrinthView, ui: &mut UiState) {
    let key = format!(
        "{:?}:{}:{}:{:?}:{:?}:{:?}:{:?}",
        ui.menus,
        view.paused,
        view.admitted,
        view.players,
        view.interruption,
        (view.mode, view.host, view.local),
        (
            world.resource::<UiMotionPreference>().reduced,
            world.resource::<UiScalePreference>().0
        )
    );
    if ui.overlay_key.as_ref() == Some(&key) {
        return;
    }
    despawn_marked::<OverlayRoot>(world);
    ui.overlay_key = Some(key);
    if !ui.menus.is_open() && !view.paused {
        return;
    }
    let root = world
        .spawn((
            bevy_game_ui::menu_overlay("Labyrinth Blocking Overlay"),
            OverlayRoot,
        ))
        .id();
    let panel = world
        .spawn((bevy_game_ui::menu_panel("Overlay Panel"), ChildOf(root)))
        .id();
    if ui.menus.current() == Some(&MenuPage::Leave) {
        label(
            world,
            panel,
            "Leave Title",
            "Leave the company?",
            UiTextRole::Title,
        );
        label(
            world,
            panel,
            "Leave Detail",
            if view.host && !view.local {
                "You are hosting. Leaving closes the session for everyone."
            } else if view.local {
                "This ends your local encounter."
            } else if view.mode == ViewMode::Lobby {
                "Leaving the lobby releases your seat in this company."
            } else {
                "Your hero stays reserved. The company waits until you reconnect."
            },
            UiTextRole::Body,
        );
        control(
            world,
            panel,
            "Keep Playing",
            "Stay with the company",
            Action::Cancel,
            false,
        );
        control(
            world,
            panel,
            "Confirm Leave",
            "Leave to main menu",
            Action::ConfirmLeave,
            false,
        );
    } else if view.paused {
        let (title, detail) = match view.interruption {
            crate::view::CombatInterruption::Halted => (
                "ENCOUNTER HALTED",
                "A rules error halted the encounter. The host can return the party to the lobby."
                    .to_owned(),
            ),
            crate::view::CombatInterruption::Reconnecting => (
                "RECONNECTING",
                "Your connection is not admitted. Reconnect to recover the current encounter."
                    .to_owned(),
            ),
            _ => {
                let missing = view
                    .players
                    .iter()
                    .filter(|p| p.occupied && !p.connected)
                    .map(|p| p.name.as_str())
                    .collect::<Vec<_>>()
                    .join(", ");
                ("THE COMPANY WAITS", format!("Waiting for {missing}. Combat resumes only when all reserved players reconnect."))
            }
        };
        label(world, panel, "Reconnect Title", title, UiTextRole::Title);
        label(world, panel, "Reconnect Detail", detail, UiTextRole::Body);
        if !view.admitted {
            control(
                world,
                panel,
                "Overlay Reconnect",
                "Reconnect reserved hero",
                Action::Reconnect,
                false,
            );
        }
        if view.host {
            control(
                world,
                panel,
                "Abort To Lobby",
                "Return party to lobby",
                Action::Rematch,
                false,
            );
        }
        control(
            world,
            panel,
            "Paused Leave",
            "Leave company…",
            Action::Leave,
            false,
        );
    } else if ui.menus.current() == Some(&MenuPage::Settings) {
        label(
            world,
            panel,
            "Settings Title",
            "Settings",
            UiTextRole::Title,
        );
        label(
            world,
            panel,
            "Settings Advice",
            "Local preferences only. The shared encounter continues.",
            UiTextRole::Supporting,
        );
        control(
            world,
            panel,
            "Toggle Semantic Scale",
            if world.resource::<UiScalePreference>().0 == UiScaleMode::Percent200 {
                "Text scale: 200% · switch to Auto"
            } else {
                "Text scale: Auto · switch to 200%"
            },
            Action::Scale,
            false,
        );
        control(
            world,
            panel,
            "Toggle Reduced Motion",
            if world.resource::<UiMotionPreference>().reduced {
                "Reduced motion: on"
            } else {
                "Reduced motion: off"
            },
            Action::ReducedMotion,
            false,
        );
        control(
            world,
            panel,
            "Close Settings",
            "Back",
            Action::Cancel,
            false,
        );
    } else {
        label(
            world,
            panel,
            "Game Menu Title",
            "Game menu",
            UiTextRole::Title,
        );
        label(
            world,
            panel,
            "Game Menu Advice",
            "This menu is local. The company keeps playing.",
            UiTextRole::Supporting,
        );
        control(
            world,
            panel,
            "Back To Game",
            "Back to game",
            Action::Cancel,
            false,
        );
        control(
            world,
            panel,
            "Game Settings",
            "Settings",
            Action::Settings,
            false,
        );
        control(
            world,
            panel,
            "Menu Leave",
            "Leave company…",
            Action::Leave,
            false,
        );
    }
}
