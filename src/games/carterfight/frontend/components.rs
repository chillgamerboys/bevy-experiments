use bevy::prelude::*;

/// Tag for anything spawned while the carterfight game is running — used by
/// scene cleanup if/when we add it.
#[derive(Component)]
pub struct CarterfightEntity;

/// The placeholder dialogue box at the bottom of the screen. The colleague
/// replaces this with a real one; the engine doesn't care which renderer
/// reads `DialogueQueue`.
#[derive(Component)]
pub struct DialogueBoxStub;

/// Text node showing the current dialogue line.
#[derive(Component)]
pub struct DialogueLineText;

/// Text node showing HP and move list during battle.
#[derive(Component)]
pub struct BattleHudText;
