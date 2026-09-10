//! Opt-in Gamekit capabilities, not a shared engine or game composition root.
//!
//! No default features, plugins, renderer, or networking are installed. Enable
//! capabilities in Cargo, then compose their plugins explicitly in your game.
//! Each underlying capability remains independently usable. These are re-exports,
//! not wrappers: types retain their identity across direct and facade imports.
//!
//! Game rules, authority policy, domain models, assets and presentation remain
//! outside this crate. See the crate README for the capability feature table.

/// Provider-neutral discovery; listing a session does not grant admission.
#[cfg(feature = "discovery")]
pub use bevy_game_discovery as discovery;
/// Pure hex geometry; no board or occupancy rules.
#[cfg(feature = "hex")]
pub use bevy_game_hex as hex;
/// Explicitly composed transport and connection lifecycle infrastructure.
#[cfg(feature = "multiplayer")]
pub use bevy_game_multiplayer as multiplayer;
/// Transport-independent identities, credentials and admission security.
#[cfg(feature = "session")]
pub use bevy_game_session as session;
/// Deterministic Bevy test infrastructure, without game-specific assertions.
#[cfg(feature = "testing")]
pub use bevy_game_test as testing;
/// Pure ordered-participant sequencing; no action legality or victory rules.
#[cfg(feature = "turns")]
pub use bevy_game_turns as turns;
/// Native Bevy UI primitives and interaction infrastructure.
#[cfg(feature = "ui")]
pub use bevy_game_ui as ui;
