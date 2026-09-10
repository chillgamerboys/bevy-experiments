//! Local readability preferences and connection-interruption overlays.

use super::*;

pub(crate) fn overlays(world: &mut World, view: &LabyrinthView, ui: &mut UiState) {
    let key = format!(
        "{}:{}:{}:{:?}:{:?}",
        ui.settings,
        view.paused,
        world.resource::<UiMotionPreference>().reduced,
        view.players,
        world.resource::<UiScalePreference>().0
    );
    if ui.overlay_key.as_ref() == Some(&key) {
        return;
    }
    despawn_marked::<OverlayRoot>(world);
    ui.overlay_key = Some(key);
    if !ui.settings && !view.paused {
        return;
    }
    let root = world
        .spawn((
            bevy_game_ui::modal("Labyrinth Blocking Overlay"),
            UiSkin::Modal,
            OverlayRoot,
        ))
        .id();
    let panel = surface(world, root, "Overlay Panel");
    world.entity_mut(panel).insert(Node {
        width: Val::Percent(88.0),
        max_width: Val::Px(1000.0),
        max_height: Val::Percent(90.0),
        flex_direction: FlexDirection::Column,
        row_gap: Val::Px(16.0),
        padding: UiRect::all(Val::Px(28.0)),
        overflow: Overflow::scroll_y(),
        ..default()
    });
    if view.paused {
        label(
            world,
            panel,
            "Reconnect Title",
            "THE COMPANY WAITS",
            UiTextRole::Title,
        );
        let missing = view
            .players
            .iter()
            .filter(|player| player.occupied && !player.connected)
            .map(|player| player.name.as_str())
            .collect::<Vec<_>>()
            .join(", ");
        label(world, panel, "Reconnect Detail", format!("Waiting for {missing}. Combat is frozen at the committed decision. Returning players reclaim the same hero and status effects."), UiTextRole::Body);
        if !view.connected_local() {
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
                "Abort encounter to lobby",
                Action::Rematch,
                false,
            );
        }
        control(
            world,
            panel,
            "Paused Leave",
            "Return to menu",
            Action::Leave,
            false,
        );
    } else {
        label(
            world,
            panel,
            "Settings Title",
            "READABILITY & MOTION",
            UiTextRole::Title,
        );
        label(world, panel, "Settings Advice", "These settings change only this window. The company keeps playing; local settings do not pause a network encounter.", UiTextRole::Supporting);
        control(
            world,
            panel,
            "Toggle Semantic Scale",
            if world.resource::<UiScalePreference>().0 == UiScaleMode::Percent200 {
                "Text scale: 200% | switch to Auto"
            } else {
                "Text scale: Auto | switch to 200%"
            },
            Action::Scale,
            false,
        );
        control(
            world,
            panel,
            "Toggle Reduced Motion",
            if world.resource::<UiMotionPreference>().reduced {
                "Reduced motion: ON"
            } else {
                "Reduced motion: OFF"
            },
            Action::ReducedMotion,
            false,
        );
        control(
            world,
            panel,
            "Close Settings",
            "Back to the company",
            Action::Cancel,
            false,
        );
    }
}

trait LocalPresence {
    fn connected_local(&self) -> bool;
}
impl LocalPresence for LabyrinthView {
    fn connected_local(&self) -> bool {
        self.local
            || self.player.is_some_and(|slot| {
                self.players
                    .iter()
                    .any(|player| player.slot == slot && player.connected)
            })
    }
}
