//! Turn-Based Grid Game - Learning Bevy ECS
//!
//! This is a simple turn-based tactical game built to learn Bevy's
//! Entity Component System (ECS) architecture.
//!
//! Phase 5: Simple AI - Enemy units move toward player automatically

use bevy::prelude::*;

// Use the library version of the game
use bevy_game::*;
use bevy_game::resources::{EnemyTurnTimer, GridMap, SelectionState};
use bevy_game::systems::*;

fn main() {
    App::new()
        // Add default Bevy plugins (rendering, input, windowing, etc.)
        .add_plugins(DefaultPlugins.set(WindowPlugin {
            primary_window: Some(Window {
                title: "Turn-Based Tactics - Phase 5: Simple AI".to_string(),
                resolution: (1280, 720).into(),
                ..default()
            }),
            ..default()
        }))
        // Initialize state machines
        .init_state::<AppState>()
        .init_state::<TurnState>()
        // Initialize resources (global state)
        .init_resource::<GridMap>()
        .init_resource::<SelectionState>()
        .init_resource::<EnemyTurnTimer>()
        // Startup systems (run once at the beginning, regardless of state)
        .add_systems(Startup, setup_camera)
        // Systems that run when entering MainMenu state
        .add_systems(OnEnter(AppState::MainMenu), setup_main_menu)
        // Systems that run when exiting MainMenu state
        .add_systems(OnExit(AppState::MainMenu), cleanup_main_menu)
        // Systems that run when entering GamePlay state
        .add_systems(
            OnEnter(AppState::GamePlay),
            (setup_grid, center_camera, spawn_units, setup_turn_ui).chain(),
        )
        // Systems for turn initialization (Phase 4)
        // OnEnter systems run ONCE when transitioning into a state
        .add_systems(OnEnter(TurnState::PlayerTurn), start_player_turn)  // Resets player unit status
        .add_systems(OnEnter(TurnState::EnemyTurn), start_enemy_turn)    // Resets enemy unit status
        // Systems that run every frame during MainMenu
        .add_systems(Update, menu_input_system.run_if(in_state(AppState::MainMenu)))
        // Systems that run every frame during GamePlay
        // IMPORTANT: These are CHAINED (.chain()) to guarantee execution order
        // Without .chain(), Bevy runs systems in parallel which can cause race conditions
        .add_systems(
            Update,
            (
                // === INPUT & GAME LOGIC (order matters!) ===
                unit_selection_system,       // 1. Handle clicks - adds/removes Selected component
                movement_system,             // 2. Move selected units - MUST run after selection

                // === VISUAL FEEDBACK (reads game state) ===
                highlight_selected_system,   // 3. Show yellow ring around selected unit
                highlight_movement_system,   // 4. Show green tiles for valid moves

                // === AI BEHAVIOR (Phase 5) ===
                ai_movement_system,          // 5. AI moves units toward player (enemy turn only)

                // === TURN MANAGEMENT ===
                check_turn_end_system,       // 6. Check if all units acted, switch turns
                update_turn_ui_system,       // 7. Update UI text ("Player Turn" / "Enemy Turn")

                // === CAMERA (runs last) ===
                camera_pan_system,           // 8. WASD camera control
            )
                .chain() // CRITICAL: Prevents race conditions between systems
                .run_if(in_state(AppState::GamePlay)),
        )
        // Run the app!
        .run();
}
