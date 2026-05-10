//! Bevy Turn-Based Tactics Game Library
//!
//! This library exposes the game's components, systems, and resources
//! for testing and modular development.

// Re-export all public modules
pub mod components;
pub mod constants;
pub mod resources;
pub mod systems;

// Re-export the state enums from main
pub use bevy::prelude::*;

/// Main application states - controls the overall game flow
#[derive(States, Debug, Clone, PartialEq, Eq, Hash, Default)]
pub enum AppState {
    #[default]
    MainMenu,   // Main menu screen (press Enter to start)
    GamePlay,   // Active gameplay
}

/// Turn states - controls whose turn it is
#[derive(States, Debug, Clone, PartialEq, Eq, Hash, Default)]
pub enum TurnState {
    #[default]
    PlayerTurn,  // Player's turn to move units
    EnemyTurn,   // Enemy's turn (AI controlled)
}
