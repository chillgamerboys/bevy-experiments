use bevy::prelude::*;

use super::super::backend::BattleState;

/// Bevy-side wrapper around the pure-Rust `BattleState`. Systems hold this as
/// a `ResMut`. All gameplay logic stays inside `backend::*`; this is just the
/// handle Bevy needs to track it as a resource.
#[derive(Resource)]
pub struct BattleStateRes(pub BattleState);
