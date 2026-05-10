//! Bevy presentation layer. Talks to the backend only through
//! `super::backend::{...}` re-exports. No move/damage logic lives here.

use bevy::prelude::*;

pub mod components;
pub mod constants;
pub mod resources;
pub mod systems;

use super::AppState;
use resources::DialogueQueue;
use systems::*;

/// Registers all carterfight frontend systems on a Bevy `App`. Call from the
/// game's `run()` after `DefaultPlugins`.
pub fn install(app: &mut App) {
    app.init_state::<AppState>()
        .init_resource::<DialogueQueue>()
        .add_systems(Startup, (setup_scene, spawn_battle_state))
        .add_systems(OnEnter(AppState::IntroDialogue), enqueue_intro_script)
        .add_systems(OnEnter(AppState::OutroDialogue), enqueue_outro_script)
        .add_systems(
            Update,
            (
                dialogue_advance_input,
                update_dialogue_text,
                intro_to_battle_when_empty,
            )
                .chain()
                .run_if(in_state(AppState::IntroDialogue)),
        )
        .add_systems(
            Update,
            (
                dialogue_advance_input,
                update_dialogue_text,
                update_battle_hud,
                battle_input,
            )
                .chain()
                .run_if(in_state(AppState::Battle)),
        )
        .add_systems(
            Update,
            (
                dialogue_advance_input,
                update_dialogue_text,
                outro_exit_when_empty,
            )
                .chain()
                .run_if(in_state(AppState::OutroDialogue)),
        );
}
