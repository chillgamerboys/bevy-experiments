//! Carterfight: an independent, dialogue-bookended local battle.
//!
//! The pure [`backend`] owns rules. [`CarterfightPlugin`] owns this game's input,
//! narration, projections and scene; Gamekit supplies optional UI mechanics.

use bevy::prelude::*;
use bevy_game_ui::{GameUiPlugin, GameUiSkinPlugin};

pub mod backend;
mod frontend;

pub use frontend::{
    CarterfightIntent, CarterfightPhase, CarterfightPlugin, CarterfightSystems, CarterfightView,
};

/// Absolute asset root, independent of the directory used to launch Cargo.
#[must_use]
pub fn asset_root() -> String {
    format!("{}/assets", env!("CARGO_MANIFEST_DIR"))
}

/// Starts the normal local game with its original authored assets and audio.
pub fn run() {
    App::new()
        .add_plugins(
            DefaultPlugins
                .set(WindowPlugin {
                    primary_window: Some(Window {
                        title: "Carterfight".to_owned(),
                        resolution: (1280, 720).into(),
                        ..default()
                    }),
                    ..default()
                })
                .set(AssetPlugin {
                    file_path: asset_root(),
                    ..default()
                })
                .set(ImagePlugin::default_nearest()),
        )
        .insert_resource(ClearColor(Color::srgb(0.055, 0.08, 0.15)))
        .add_plugins((GameUiPlugin, GameUiSkinPlugin, CarterfightPlugin))
        .add_systems(Startup, |mut commands: Commands| {
            commands.spawn(Camera2d);
        })
        .run();
}
