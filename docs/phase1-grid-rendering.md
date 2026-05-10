# Phase 1: Grid Rendering and Camera Setup

**Status:** ✅ Complete
**Goal:** Understand Bevy's rendering and ECS basics

## Overview

Phase 1 establishes the foundation of our turn-based game by creating a 10x10 grid of tiles and setting up a 2D camera. This phase teaches the fundamental concepts of Bevy's Entity Component System (ECS).

## What We Built

### File Structure

```
src/
├── main.rs          - App initialization and system registration
├── constants.rs     - Game configuration constants
├── components.rs    - Component definitions (data structures)
├── resources.rs     - Resource definitions (global state)
└── systems.rs       - System definitions (game logic)
```

### Components (Data)

Components are pure data with no logic - they're just Rust structs attached to entities.

#### `GridPosition` ([src/components.rs:11-35](../src/components.rs))

```rust
#[derive(Component, Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct GridPosition {
    pub x: i32,
    pub y: i32,
}
```

**Purpose:** Represents a position on the grid (not world coordinates).

**Helper Methods:**
- `adjacent()` - Returns the 4 adjacent grid positions (no diagonals)
- `distance_to()` - Calculates Manhattan distance to another position

**Usage:** Attached to both tiles and units to track their grid location.

#### `Tile` ([src/components.rs:38-62](../src/components.rs))

```rust
#[derive(Component, Debug, Clone, Copy)]
pub struct Tile {
    pub walkable: bool,
    pub tile_type: TileType,
}
```

**Purpose:** Defines properties of a grid tile.

**Fields:**
- `walkable` - Can units move through this tile?
- `tile_type` - What kind of terrain (Grass, Water, Mountain)

### Resources (Global State)

Resources are singletons that exist once per world and can be accessed by any system.

#### `GridMap` ([src/resources.rs:14-59](../src/resources.rs))

```rust
#[derive(Resource)]
pub struct GridMap {
    pub width: i32,
    pub height: i32,
    pub tile_size: f32,
    pub tiles: HashMap<(i32, i32), Entity>,
}
```

**Purpose:** Central authority for grid management and coordinate conversion.

**Key Methods:**
- `world_to_grid(Vec2)` - Convert pixel coordinates → grid coordinates
- `grid_to_world(GridPosition)` - Convert grid coordinates → pixel coordinates
- `is_in_bounds(GridPosition)` - Check if position is within grid
- `register_tile()` / `get_tile()` - Track tile entities by position

**Why This Is Important:** The grid is the foundation of our game. This resource provides the single source of truth for grid state and handles all coordinate conversions.

### Systems (Logic)

Systems are functions that operate on components and resources. They're where all game logic lives.

#### `setup_camera` ([src/systems.rs:12-20](../src/systems.rs))

```rust
pub fn setup_camera(mut commands: Commands) {
    commands.spawn(Camera2d);
    info!("Camera spawned");
}
```

**When:** Runs once at startup (registered with `Startup` schedule)

**What:** Creates a 2D camera entity that renders the game world

**ECS Pattern:** Uses the `Commands` buffer to spawn an entity with the `Camera2d` component

#### `setup_grid` ([src/systems.rs:24-67](../src/systems.rs))

```rust
pub fn setup_grid(mut commands: Commands, mut grid_map: ResMut<GridMap>) {
    for y in 0..GRID_HEIGHT {
        for x in 0..GRID_WIDTH {
            let grid_pos = GridPosition::new(x, y);
            let world_pos = grid_map.grid_to_world(&grid_pos);

            let color = if (x + y) % 2 == 0 {
                TILE_COLOR_LIGHT
            } else {
                TILE_COLOR_DARK
            };

            let tile_entity = commands.spawn((
                Tile::new_grass(),
                grid_pos,
                Sprite { color, custom_size: Some(Vec2::new(TILE_SIZE, TILE_SIZE)), ..default() },
                Transform::from_xyz(world_pos.x, world_pos.y, Z_TILE),
            )).id();

            grid_map.register_tile(grid_pos, tile_entity);
        }
    }
}
```

**When:** Runs once at startup, after camera setup

**What:** Spawns 100 tile entities (10×10 grid) with components

**ECS Patterns:**
1. **Component Bundle:** Each tile gets multiple components: `Tile`, `GridPosition`, `Sprite`, `Transform`
2. **Entity Tracking:** Stores entity IDs in the `GridMap` resource for later lookup
3. **Checkerboard Pattern:** Alternates tile colors using `(x + y) % 2`
4. **Z-Ordering:** Uses `Z_TILE` constant to ensure tiles render behind other objects

#### `center_camera` ([src/systems.rs:68-87](../src/systems.rs))

```rust
pub fn center_camera(
    grid_map: Res<GridMap>,
    mut camera_query: Query<&mut Transform, With<Camera2d>>,
) {
    if let Ok(mut camera_transform) = camera_query.single_mut() {
        let grid_center_x = (grid_map.width as f32 * grid_map.tile_size) / 2.0;
        let grid_center_y = (grid_map.height as f32 * grid_map.tile_size) / 2.0;

        camera_transform.translation.x = grid_center_x;
        camera_transform.translation.y = grid_center_y;
    }
}
```

**When:** Runs once at startup, after grid setup

**What:** Positions the camera at the center of the grid

**ECS Patterns:**
1. **Query:** `Query<&mut Transform, With<Camera2d>>` finds the camera entity
2. **Filter:** `With<Camera2d>` ensures we only get camera entities
3. **Single Entity:** `.single_mut()` assumes exactly one camera exists
4. **Resource Access:** Reads grid dimensions from `GridMap` resource

#### `camera_pan_system` ([src/systems.rs:91-115](../src/systems.rs))

```rust
pub fn camera_pan_system(
    keyboard: Res<ButtonInput<KeyCode>>,
    mut camera_query: Query<&mut Transform, With<Camera2d>>,
    time: Res<Time>,
) {
    if let Ok(mut camera_transform) = camera_query.single_mut() {
        let camera_speed = 300.0; // pixels per second
        let delta = camera_speed * time.delta_secs();

        if keyboard.pressed(KeyCode::KeyW) || keyboard.pressed(KeyCode::ArrowUp) {
            camera_transform.translation.y += delta;
        }
        // ... more directions
    }
}
```

**When:** Runs every frame (registered with `Update` schedule)

**What:** Allows panning the camera with WASD or arrow keys

**ECS Patterns:**
1. **Input Resource:** `Res<ButtonInput<KeyCode>>` provides keyboard state
2. **Time-Based Movement:** Uses `time.delta_secs()` for frame-rate independent movement
3. **Continuous Input:** `keyboard.pressed()` checks if key is held down

#### `mouse_input_system` ([src/systems.rs:119-161](../src/systems.rs))

```rust
pub fn mouse_input_system(
    buttons: Res<ButtonInput<MouseButton>>,
    windows: Query<&Window>,
    camera_query: Query<(&Camera, &GlobalTransform), With<Camera2d>>,
    grid_map: Res<GridMap>,
) {
    if !buttons.just_pressed(MouseButton::Left) {
        return;
    }

    let Ok(window) = windows.single() else { return; };
    let Ok((camera, camera_transform)) = camera_query.single() else { return; };

    if let Some(cursor_pos) = window.cursor_position() {
        if let Ok(world_pos) = camera.viewport_to_world_2d(camera_transform, cursor_pos) {
            let grid_pos = grid_map.world_to_grid(world_pos);

            if grid_map.is_in_bounds(&grid_pos) {
                info!("Clicked tile at grid position ({}, {})", grid_pos.x, grid_pos.y);
            }
        }
    }
}
```

**When:** Runs every frame

**What:** Detects mouse clicks and converts to grid coordinates

**ECS Patterns:**
1. **Event Detection:** `buttons.just_pressed()` detects single clicks (not continuous)
2. **Multiple Queries:** Queries both window and camera entities
3. **Coordinate Conversion Chain:** Screen → Viewport → World → Grid
4. **Early Returns:** Uses guard clauses to handle errors gracefully

## Key Bevy Concepts Learned

### 1. Entity Component System (ECS)

**Entities** are just IDs (like `Entity(5v0)`) - they have no data themselves.

**Components** are data attached to entities:
```rust
// This entity has three components
commands.spawn((
    GridPosition { x: 0, y: 0 },
    Tile { walkable: true, tile_type: TileType::Grass },
    Sprite { color: Color::GREEN, ... },
));
```

**Systems** are functions that query entities and operate on their components:
```rust
fn my_system(query: Query<(&GridPosition, &Tile)>) {
    for (pos, tile) in &query {
        // Logic here operates on entities that have both components
    }
}
```

**Why ECS?**
- **Performance:** Systems can process components in parallel
- **Flexibility:** Add/remove components to change entity behavior
- **Composition:** Build complex entities from simple components

### 2. Commands Buffer

```rust
fn setup(mut commands: Commands) {
    let entity = commands.spawn(SomeComponent).id();
    commands.entity(entity).insert(AnotherComponent);
}
```

**Important:** Changes made with `commands` don't happen immediately - they're buffered and applied at the end of the current stage. This prevents issues with concurrent system execution.

### 3. Queries

```rust
Query<&Transform>                    // Read-only access to Transform
Query<&mut Transform>                 // Mutable access to Transform
Query<&Transform, With<Camera2d>>     // Only entities with Camera2d
Query<&Transform, Without<Unit>>      // Only entities without Unit
Query<(&Sprite, &Transform)>         // Entities with both components
```

Queries are how systems access components. Bevy automatically determines which systems can run in parallel based on their queries.

### 4. Resources

```rust
fn system(grid_map: Res<GridMap>) {          // Read-only
    let width = grid_map.width;
}

fn system(mut grid_map: ResMut<GridMap>) {   // Mutable
    grid_map.width = 20;
}
```

Resources are singletons - one instance per world. Use them for:
- Global game state (turn manager, score)
- Configuration (grid dimensions)
- Shared utilities (asset handles)

### 5. System Scheduling

```rust
App::new()
    .add_systems(Startup, (setup_camera, setup_grid).chain())
    .add_systems(Update, (input_system, camera_system))
```

**Startup** schedule runs once when app starts.
**Update** schedule runs every frame.
**`.chain()`** ensures systems run in order (setup_camera before setup_grid).

Without `.chain()`, Bevy runs systems in parallel when possible!

## Constants Used

From [src/constants.rs](../src/constants.rs):

```rust
pub const GRID_WIDTH: i32 = 10;
pub const GRID_HEIGHT: i32 = 10;
pub const TILE_SIZE: f32 = 64.0;

pub const TILE_COLOR_LIGHT: Color = Color::srgb(0.8, 0.8, 0.7);
pub const TILE_COLOR_DARK: Color = Color::srgb(0.6, 0.6, 0.5);

pub const Z_TILE: f32 = 0.0;
pub const Z_OVERLAY: f32 = 1.0;
pub const Z_UNIT: f32 = 2.0;
pub const Z_SELECTION: f32 = 3.0;
```

**Z-Ordering:** Higher Z values render on top. We define layers to ensure correct rendering order.

## How to Test

1. **Run the game:** `cargo run`
2. **Verify grid:** You should see a 10×10 checkerboard grid
3. **Pan camera:** Press WASD or arrow keys to move the view
4. **Click tiles:** Click on tiles and check terminal for logged coordinates

## What's Next?

In **Phase 2**, we'll add:
- State management (MainMenu, GamePlay states)
- Better input handling
- Transition between game states

This will teach you how to structure larger games with multiple screens and modes.

## Common Questions

**Q: Why separate components and resources?**

A: Components are attached to entities (many tiles, many units). Resources are global singletons (one GridMap, one TurnManager). Use components for things that vary per entity, resources for game-wide state.

**Q: Why use `Commands` instead of spawning directly?**

A: Bevy systems run in parallel. Direct spawning would require locks and hurt performance. The commands buffer allows batching changes safely.

**Q: What's the difference between `Transform` and `GlobalTransform`?**

A: `Transform` is local position. `GlobalTransform` is computed world position (accounting for parent transforms). Bevy automatically maintains `GlobalTransform` based on hierarchy.

**Q: Why `Res<>` and `ResMut<>` instead of just passing resources?**

A: The type signature tells Bevy's scheduler whether the system reads or writes the resource. This allows parallel execution of systems that only read the same resource.

## Exercises

Try these to solidify your understanding:

1. **Change grid size:** Modify `GRID_WIDTH` and `GRID_HEIGHT` to create a 15×15 grid
2. **Add new tile type:** Add a "Road" tile type with a different color
3. **Highlight hovered tile:** Make the tile under the mouse cursor change color
4. **Zoom controls:** Add mouse wheel zoom to the camera system
5. **Grid lines:** Spawn line entities to draw borders around tiles

## Resources

- [Bevy ECS Guide](https://docs.rs/bevy/0.18.0/bevy/ecs/index.html)
- [Bevy Examples - 2D Rendering](https://github.com/bevyengine/bevy/tree/v0.18.0/examples/2d)
- [Unofficial Bevy Cheat Book](https://bevy-cheatbook.github.io/)
