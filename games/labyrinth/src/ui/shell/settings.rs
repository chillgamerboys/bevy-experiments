//! Local menus never mutate encounter suspension or stop network processing.
use super::*;

pub(crate) fn overlays(world: &mut World, view: &LabyrinthView, ui: &mut UiState) {
    let key = format!(
        "{:?}:{}:{}:{:?}:{:?}:{:?}:{:?}",
        ui.menus,
        view.paused,
        view.admitted,
        (&view.players, &view.company),
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
            bevy_gamekit::ui::menu_overlay("Labyrinth Blocking Overlay"),
            OverlayRoot,
        ))
        .id();
    let panel = world
        .spawn((bevy_gamekit::ui::menu_panel("Overlay Panel"), ChildOf(root)))
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
    } else if ui.menus.current() == Some(&MenuPage::Party) && !view.local {
        party(world, panel, view);
    } else if view.paused && !ui.menus.is_open() {
        let (title, detail) = match view.interruption {
            crate::view::CombatInterruption::Assignments => (
                "CHARACTER ASSIGNMENTS",
                "Combat is paused. The host can assign surviving characters; character stats, positions and turns remain unchanged.".to_owned(),
            ),
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
                    .filter(|p| p.occupied && !p.connected && !p.actors.is_empty())
                    .map(|p| p.name.as_str())
                    .collect::<Vec<_>>()
                    .join(", ");
                ("THE COMPANY WAITS", format!("Waiting for {missing}. Reconnect or ask the host to reassign their characters."))
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
        if !view.local && view.admitted {
            control(
                world,
                panel,
                "Manage Assignments",
                "Party management",
                Action::PartyManagement,
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
        if !view.local && !view.paused {
            label(
                world,
                panel,
                "Game Menu Advice",
                "Combat continues while this menu is open.",
                UiTextRole::Supporting,
            );
        }
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
        if !view.local && view.mode == ViewMode::Combat {
            control(
                world,
                panel,
                "Manage Assignments",
                "Party management",
                Action::PartyManagement,
                false,
            );
        }
        let leave = control(
            world,
            panel,
            "Menu Leave",
            if view.local {
                "Return to main menu…"
            } else {
                "Leave company…"
            },
            Action::Leave,
            false,
        );
        world
            .get_mut::<Node>(leave)
            .expect("menu control")
            .margin
            .top = Val::Px(20.0);
    }
}

fn party(world: &mut World, panel: Entity, view: &LabyrinthView) {
    use crate::view::CombatInterruption;
    label(
        world,
        panel,
        "Party Management Title",
        "Party management",
        UiTextRole::Title,
    );
    let editing =
        view.host && view.admitted && view.interruption == CombatInterruption::Assignments;
    if editing {
        label(
            world,
            panel,
            "Party Management Detail",
            "Combat is paused. Assign characters, then resume when ready.",
            UiTextRole::Supporting,
        );
        lobby::assignments(world, panel, view);
        control(
            world,
            panel,
            "Resume After Assignment",
            "Resume combat",
            Action::AssignmentPause(false),
            false,
        );
    } else {
        for member in &view.company {
            let character = view
                .scenario
                .as_ref()
                .and_then(|s| s.heroes.iter().find(|a| a.id == member.actor))
                .map_or(member.hero.name(), |a| a.actor.name.as_str());
            let owner = view
                .players
                .iter()
                .find(|p| p.slot == member.owner)
                .map_or("Unassigned", |p| p.name.as_str());
            label(
                world,
                panel,
                &format!("Party Character {}", member.actor.0),
                format!("{character} · {owner}"),
                UiTextRole::Body,
            );
        }
        if view.host && view.admitted && view.interruption != CombatInterruption::Halted {
            control(
                world,
                panel,
                "Pause For Assignments",
                "Pause and edit assignments",
                Action::AssignmentPause(true),
                false,
            );
        } else {
            label(
                world,
                panel,
                "Party Management Detail",
                "The host manages character assignments.",
                UiTextRole::Supporting,
            );
        }
    }
    control(world, panel, "Party Back", "Back", Action::Cancel, false);
}
