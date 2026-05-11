//! Carterfight — a 1v1 dialogue-bookended battle.
//!
//! The game is split into two physically separate halves:
//!
//! - [`backend`] — pure-Rust battle engine (state, moves, turn resolution).
//!   Zero Bevy imports. Exposes a narrow `pub use` API surface.
//! - [`frontend`] — Bevy presentation (dialogue box, HUD, input plumbing).
//!   Talks to the backend only through its public surface.
//!
//! `mod.rs` is glue: it declares the carterfight-local [`AppState`] and
//! delegates Bevy app setup to [`frontend::install`].

use bevy::prelude::*;

pub mod backend;
pub mod frontend;

/// Game-flow state machine. Battle's *internal* turn phases live as
/// [`backend::BattlePhase`] inside the engine — not as substates here — so
/// they stay testable in pure Rust.
#[derive(States, Debug, Clone, PartialEq, Eq, Hash, Default)]
pub enum AppState {
    #[default]
    IntroDialogue,
    Battle,
    OutroDialogue,
}

pub fn run() {
    let mut app = App::new();
    app.add_plugins(DefaultPlugins.set(WindowPlugin {
        primary_window: Some(Window {
            title: frontend::constants::WINDOW_TITLE.to_string(),
            resolution: frontend::constants::WINDOW_SIZE.into(),
            ..default()
        }),
        ..default()
    }));
    frontend::install(&mut app);
    app.run();
}
