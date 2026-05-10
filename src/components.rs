// Component definitions for the turn-based game
// In ECS, components are pure data - no logic!

use bevy::prelude::*;

// ===== GRID & POSITIONING COMPONENTS =====

/// Represents a position on the grid (not world coordinates)
#[derive(Component, Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct GridPosition {
    pub x: i32,
    pub y: i32,
}

impl GridPosition {
    pub fn new(x: i32, y: i32) -> Self {
        Self { x, y }
    }

    /// Get all adjacent positions (4-directional, no diagonals)
    pub fn adjacent(&self) -> Vec<GridPosition> {
        vec![
            GridPosition::new(self.x + 1, self.y),
            GridPosition::new(self.x - 1, self.y),
            GridPosition::new(self.x, self.y + 1),
            GridPosition::new(self.x, self.y - 1),
        ]
    }

    /// Calculate Manhattan distance to another position
    pub fn distance_to(&self, other: &GridPosition) -> u32 {
        ((self.x - other.x).abs() + (self.y - other.y).abs()) as u32
    }
}

/// Component for tile entities
#[derive(Component, Debug, Clone, Copy)]
pub struct Tile {
    pub walkable: bool,
    pub tile_type: TileType,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TileType {
    Grass,    // Normal walkable terrain
    Water,    // Non-walkable
    Mountain, // Non-walkable
}

impl Tile {
    pub fn new_grass() -> Self {
        Self {
            walkable: true,
            tile_type: TileType::Grass,
        }
    }

    pub fn new_water() -> Self {
        Self {
            walkable: false,
            tile_type: TileType::Water,
        }
    }
}

// ===== UNIT COMPONENTS (for Phase 3) =====

/// Marker component that identifies an entity as a unit
#[derive(Component, Debug)]
pub struct Unit {
    pub faction: Faction,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Faction {
    Player,
    Enemy,
}

/// Component tracking unit's status in the current turn
#[derive(Component, Debug, Clone)]
pub struct TurnStatus {
    pub has_acted: bool,
    pub has_moved: bool,
}

impl Default for TurnStatus {
    fn default() -> Self {
        Self {
            has_acted: false,
            has_moved: false,
        }
    }
}

// ===== SELECTION COMPONENTS (for Phase 3) =====

/// Marker component: indicates this unit is currently selected
#[derive(Component, Debug)]
pub struct Selected;

/// Marker component: indicates this entity can be hovered over
#[derive(Component, Debug)]
pub struct Hoverable;

// ===== AI COMPONENTS (for Phase 5) =====

/// Marker component: indicates this unit is controlled by AI
#[derive(Component, Debug)]
pub struct AIControlled;

// ===== COMBAT COMPONENTS (for Phase 6) =====

/// Stats for units in combat
#[derive(Component, Debug, Clone)]
pub struct Stats {
    pub max_hp: i32,
    pub current_hp: i32,
    pub attack: i32,
    pub defense: i32,
}

impl Stats {
    pub fn new(max_hp: i32, attack: i32, defense: i32) -> Self {
        Self {
            max_hp,
            current_hp: max_hp,
            attack,
            defense,
        }
    }

    pub fn is_alive(&self) -> bool {
        self.current_hp > 0
    }
}
