//! Carterfight battle engine — pure Rust, zero `use bevy::*`.
//!
//! The frontend (and tests) may import only from this `pub use` block.
//! Everything below is `pub(super)` or private. Adding a new line here is a
//! real design decision — pause and think about whether the new surface needs
//! to be public.

mod action;
mod data;
mod events;
mod moves;
mod resolve;
mod rng;
mod state;

pub use action::Action;
pub use data::{character_template, move_def};
pub use events::BattleEvent;
pub use moves::{AbilityId, MoveDef, MoveEffect, MoveId};
pub use resolve::resolve_turn;
pub use state::{BattlePhase, BattleState, Character, Side};
