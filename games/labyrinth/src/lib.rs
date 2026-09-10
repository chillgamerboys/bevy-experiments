//! Labyrinth: an independently composed cooperative positional-combat prototype.

mod network;
pub mod presentation;
pub mod profile;
mod scene;
mod session;
pub mod ui;
pub mod view;

use bevy::prelude::*;

/// Composes game-owned lobby, authority, transport, and presentation.
pub struct LabyrinthPlugin;

impl Plugin for LabyrinthPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<view::LabyrinthView>()
            .add_message::<view::LabyrinthIntent>()
            .add_plugins((network::LabyrinthNetworkPlugin, ui::LabyrinthUiPlugin));
    }
}
