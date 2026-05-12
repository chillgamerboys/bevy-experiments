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

/// Frontend mirror of the parts of `BattleState` that the HUD renders.
/// The sequencer writes into this as it processes each `BattleEvent`, so the
/// health bar only updates in sync with its corresponding dialogue line
/// instead of snapping to the authoritative state the moment `resolve_turn`
/// runs. Max HP is read from `BattleStateRes` since it never changes.
#[derive(Resource, Default)]
pub struct DisplayedCombatants {
    pub player_hp: u16,
    pub opponent_hp: u16,
}
