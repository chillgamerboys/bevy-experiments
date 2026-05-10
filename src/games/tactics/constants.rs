// Game constants for grid-based turn-based game

use bevy::prelude::*;

// Grid dimensions
pub const GRID_WIDTH: i32 = 10;
pub const GRID_HEIGHT: i32 = 10;
pub const TILE_SIZE: f32 = 64.0;

// Colors for tiles
pub const TILE_COLOR_LIGHT: Color = Color::srgb(0.8, 0.8, 0.7);  // Light beige
pub const TILE_COLOR_DARK: Color = Color::srgb(0.6, 0.6, 0.5);   // Dark beige
pub const GRID_LINE_COLOR: Color = Color::srgb(0.3, 0.3, 0.3);   // Dark gray

// Colors for units
pub const PLAYER_COLOR: Color = Color::srgb(0.2, 0.5, 0.9);      // Blue
pub const ENEMY_COLOR: Color = Color::srgb(0.9, 0.2, 0.2);       // Red
pub const SELECTED_COLOR: Color = Color::srgb(1.0, 0.9, 0.2);    // Yellow
pub const MOVEMENT_HIGHLIGHT: Color = Color::srgba(0.2, 0.9, 0.2, 0.5); // Semi-transparent green

// Z-layers for rendering order
pub const Z_TILE: f32 = 0.0;
pub const Z_OVERLAY: f32 = 1.0;
pub const Z_UNIT: f32 = 2.0;
pub const Z_SELECTION: f32 = 3.0;
pub const Z_UI: f32 = 10.0;

// Unit properties
pub const UNIT_RADIUS: f32 = 24.0;  // Visual radius of unit circle
pub const SELECTION_RING_RADIUS: f32 = 28.0;  // Radius of selection indicator
