use bevy::prelude::*;

/// Tag for anything spawned while the carterfight game is running — used by
/// scene cleanup if/when we add it.
#[derive(Component)]
pub struct CarterfightEntity;

/// Text node showing HP and move list during battle.
#[derive(Component)]
pub struct BattleHudText;
