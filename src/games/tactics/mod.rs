//! Turn-based tactics — the first experiment.
//!
//! 10×10 grid, player vs AI units, click-to-select / click-to-move,
//! turn-based loop with a simple "approach the nearest player" AI.
//!
//! Entry point: [`run`].

use bevy::prelude::*;

pub mod components;
pub mod constants;
pub mod resources;
pub mod systems;

use resources::{EnemyTurnTimer, GameOverInfo, GridMap, SelectionState};
use systems::*;

/// Main application states — controls the overall game flow.
#[derive(States, Debug, Clone, PartialEq, Eq, Hash, Default)]
pub enum AppState {
    #[default]
    MainMenu,
    GamePlay,
    GameOver,
}

/// Turn states — controls whose turn it is.
#[derive(States, Debug, Clone, PartialEq, Eq, Hash, Default)]
pub enum TurnState {
    #[default]
    PlayerTurn,
    EnemyTurn,
}

pub fn run() {
    App::new()
        .add_plugins(DefaultPlugins.set(WindowPlugin {
            primary_window: Some(Window {
                title: "Turn-Based Tactics".to_string(),
                resolution: (1280, 720).into(),
                ..default()
            }),
            ..default()
        }))
        .init_state::<AppState>()
        .init_state::<TurnState>()
        .init_resource::<GridMap>()
        .init_resource::<SelectionState>()
        .init_resource::<EnemyTurnTimer>()
        .init_resource::<GameOverInfo>()
        .add_systems(Startup, setup_camera)
        .add_systems(OnEnter(AppState::MainMenu), setup_main_menu)
        .add_systems(OnExit(AppState::MainMenu), cleanup_main_menu)
        .add_systems(
            OnEnter(AppState::GamePlay),
            (setup_grid, center_camera, spawn_units, setup_turn_ui).chain(),
        )
        .add_systems(OnExit(AppState::GamePlay), cleanup_gameplay)
        .add_systems(OnEnter(AppState::GameOver), setup_game_over_ui)
        .add_systems(OnExit(AppState::GameOver), cleanup_game_over_ui)
        .add_systems(OnEnter(TurnState::PlayerTurn), start_player_turn)
        .add_systems(OnEnter(TurnState::EnemyTurn), start_enemy_turn)
        .add_systems(Update, menu_input_system.run_if(in_state(AppState::MainMenu)))
        .add_systems(
            Update,
            gameover_input_system.run_if(in_state(AppState::GameOver)),
        )
        .add_systems(
            Update,
            (
                unit_selection_system,
                movement_system,
                highlight_selected_system,
                highlight_movement_system,
                ai_movement_system,
                check_turn_end_system,
                check_game_over_system,
                update_turn_ui_system,
                camera_pan_system,
            )
                .chain()
                .run_if(in_state(AppState::GamePlay)),
        )
        .run();
}
