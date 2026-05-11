//! Bevy presentation layer. Talks to the backend only through
//! `super::backend::{...}` re-exports. No move/damage logic lives here.

use bevy::prelude::*;

pub mod components;
pub mod constants;
pub mod dialogue;
pub mod resources;
pub mod systems;

pub use dialogue::{DialoguePlugin, DialogueQueue, DialogueState};

use super::AppState;
use systems::*;

/// Registers all carterfight frontend systems on a Bevy `App`. Call from the
/// game's `run()` after `DefaultPlugins`.
pub fn install(app: &mut App) {
    app.init_state::<AppState>()
        .add_plugins(DialoguePlugin)
        .add_systems(Startup, (setup_scene, spawn_battle_state))
        .add_systems(OnEnter(AppState::IntroDialogue), enqueue_intro_script)
        .add_systems(OnEnter(AppState::OutroDialogue), enqueue_outro_script)
        .add_systems(
            Update,
            intro_to_battle_when_empty.run_if(in_state(AppState::IntroDialogue)),
        )
        .add_systems(
            Update,
            (update_battle_hud, battle_input)
                .chain()
                .run_if(in_state(AppState::Battle)),
        )
        .add_systems(
            Update,
            outro_exit_when_empty.run_if(in_state(AppState::OutroDialogue)),
        );
}
