use bevy::prelude::*;

/// Tag for anything spawned while the carterfight game is running — used by
/// scene cleanup if/when we add it.
#[derive(Component)]
pub struct CarterfightEntity;

/// Text node showing HP and move list during battle.
#[derive(Component)]
pub struct BattleHudText;

/// The colored fill sprite of Carter's health bar. The update system shrinks
/// its width based on his current HP ratio.
#[derive(Component)]
pub struct CarterHealthBarFill;

/// World-space text under Carter's sprite showing "current/max" HP.
#[derive(Component)]
pub struct CarterHealthText;
