use bevy::prelude::*;

use super::super::backend::{BattleState, MoveId};

/// Bevy-side wrapper around the pure-Rust `BattleState`. Systems hold this as
/// a `ResMut`. All gameplay logic stays inside `backend::*`; this is just the
/// handle Bevy needs to track it as a resource.
#[derive(Resource)]
pub struct BattleStateRes(pub BattleState);

/// Two-stage move selection: number keys set this; Space confirms it and
/// runs the turn. `None` means no move is queued and the next number-key
/// press starts a new selection.
#[derive(Resource, Default)]
pub struct PendingMove(pub Option<MoveId>);
