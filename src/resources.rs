// Resource definitions for global game state
// Resources are singletons that can be accessed by any system

use bevy::prelude::*;
use std::collections::HashMap;

use crate::components::{Faction, GridPosition};
use crate::constants::{GRID_HEIGHT, GRID_WIDTH, TILE_SIZE};

// ===== GRID MANAGEMENT RESOURCE =====

/// Global resource managing the grid and coordinate conversions
#[derive(Resource)]
pub struct GridMap {
    pub width: i32,
    pub height: i32,
    pub tile_size: f32,
    /// Maps grid coordinates to tile entity IDs
    pub tiles: HashMap<(i32, i32), Entity>,
}

impl GridMap {
    pub fn new(width: i32, height: i32, tile_size: f32) -> Self {
        Self {
            width,
            height,
            tile_size,
            tiles: HashMap::new(),
        }
    }

    /// Convert world coordinates to grid position
    pub fn world_to_grid(&self, world_pos: Vec2) -> GridPosition {
        let x = (world_pos.x / self.tile_size).floor() as i32;
        let y = (world_pos.y / self.tile_size).floor() as i32;
        GridPosition::new(x, y)
    }

    /// Convert grid position to world coordinates (center of tile)
    pub fn grid_to_world(&self, grid_pos: &GridPosition) -> Vec2 {
        Vec2::new(
            grid_pos.x as f32 * self.tile_size + self.tile_size / 2.0,
            grid_pos.y as f32 * self.tile_size + self.tile_size / 2.0,
        )
    }

    /// Check if grid position is within bounds
    pub fn is_in_bounds(&self, pos: &GridPosition) -> bool {
        pos.x >= 0 && pos.x < self.width && pos.y >= 0 && pos.y < self.height
    }

    /// Register a tile entity at a grid position
    pub fn register_tile(&mut self, pos: GridPosition, entity: Entity) {
        self.tiles.insert((pos.x, pos.y), entity);
    }

    /// Get tile entity at a grid position
    pub fn get_tile(&self, pos: &GridPosition) -> Option<Entity> {
        self.tiles.get(&(pos.x, pos.y)).copied()
    }
}

impl Default for GridMap {
    fn default() -> Self {
        Self::new(GRID_WIDTH, GRID_HEIGHT, TILE_SIZE)
    }
}

// ===== TURN MANAGEMENT RESOURCE (for Phase 4) =====
// NOTE: Turn management is now handled by TurnState state machine in main.rs
// The TurnManager resource was redundant and has been removed.
// If you need to track turn counts in the future, create a separate TurnCounter resource.

// ===== SELECTION STATE RESOURCE (for Phase 3) =====

/// Tracks the currently selected unit and hovered position
#[derive(Resource, Default, Debug)]
pub struct SelectionState {
    pub selected_unit: Option<Entity>,
    pub hovered_tile: Option<GridPosition>,
}

impl SelectionState {
    pub fn clear_selection(&mut self) {
        self.selected_unit = None;
    }

    pub fn select_unit(&mut self, entity: Entity) {
        self.selected_unit = Some(entity);
    }
}

// ===== TURN TIMER RESOURCE (for Phase 4) =====

use bevy::time::Timer;
use bevy::time::TimerMode;

/// Timer for enemy turn delay
#[derive(Resource)]
pub struct EnemyTurnTimer {
    pub timer: Timer,
}

impl Default for EnemyTurnTimer {
    fn default() -> Self {
        Self {
            timer: Timer::from_seconds(1.5, TimerMode::Once),
        }
    }
}
