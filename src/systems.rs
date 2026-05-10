// System definitions - where all the game logic lives
// Systems are just functions that operate on components and resources

use bevy::prelude::*;

use crate::components::{AIControlled, Faction, GridPosition, Hoverable, Selected, Tile, TurnStatus, Unit};
use crate::constants::*;
use crate::resources::{EnemyTurnTimer, GridMap, SelectionState};
use crate::{AppState, TurnState};

// ===== SETUP SYSTEMS =====

/// Sets up the 2D camera for the game
pub fn setup_camera(mut commands: Commands) {
    // Spawn a 2D camera
    // The camera will be centered at (0, 0) by default
    commands.spawn(Camera2d);

    info!("Camera spawned");
}

/// Creates the grid of tiles
/// This system runs once at startup to initialize the game board
pub fn setup_grid(mut commands: Commands, mut grid_map: ResMut<GridMap>) {
    info!(
        "Setting up grid: {}x{} tiles of size {}",
        GRID_WIDTH, GRID_HEIGHT, TILE_SIZE
    );

    // Iterate through all grid positions and spawn tile entities
    for y in 0..GRID_HEIGHT {
        for x in 0..GRID_WIDTH {
            let grid_pos = GridPosition::new(x, y);

            // Calculate world position for this tile
            let world_pos = grid_map.grid_to_world(&grid_pos);

            // Checkerboard pattern for tile colors
            let color = if (x + y) % 2 == 0 {
                TILE_COLOR_LIGHT
            } else {
                TILE_COLOR_DARK
            };

            // Spawn the tile entity with all its components
            let tile_entity = commands
                .spawn((
                    Tile::new_grass(),
                    grid_pos,
                    Sprite {
                        color,
                        custom_size: Some(Vec2::new(TILE_SIZE, TILE_SIZE)),
                        ..default()
                    },
                    Transform::from_xyz(world_pos.x, world_pos.y, Z_TILE),
                ))
                .id();

            // Register this tile in the grid map
            grid_map.register_tile(grid_pos, tile_entity);
        }
    }

    info!("Grid setup complete: {} tiles spawned", grid_map.tiles.len());
}

/// Centers the camera on the grid
/// Runs after grid setup to position camera properly
pub fn center_camera(
    grid_map: Res<GridMap>,
    mut camera_query: Query<&mut Transform, With<Camera2d>>,
) {
    if let Ok(mut camera_transform) = camera_query.single_mut() {
        // Calculate the center of the grid in world coordinates
        let grid_center_x = (grid_map.width as f32 * grid_map.tile_size) / 2.0;
        let grid_center_y = (grid_map.height as f32 * grid_map.tile_size) / 2.0;

        // Position camera at grid center
        camera_transform.translation.x = grid_center_x;
        camera_transform.translation.y = grid_center_y;

        info!(
            "Camera centered at ({}, {})",
            grid_center_x, grid_center_y
        );
    }
}

// ===== CAMERA CONTROL SYSTEMS (for Phase 2) =====

/// Allows panning the camera with WASD or arrow keys
pub fn camera_pan_system(
    keyboard: Res<ButtonInput<KeyCode>>,
    mut camera_query: Query<&mut Transform, With<Camera2d>>,
    time: Res<Time>,
) {
    if let Ok(mut camera_transform) = camera_query.single_mut() {
        let camera_speed = 300.0; // pixels per second
        let delta = camera_speed * time.delta_secs();

        // WASD or arrow keys for panning
        if keyboard.pressed(KeyCode::KeyW) || keyboard.pressed(KeyCode::ArrowUp) {
            camera_transform.translation.y += delta;
        }
        if keyboard.pressed(KeyCode::KeyS) || keyboard.pressed(KeyCode::ArrowDown) {
            camera_transform.translation.y -= delta;
        }
        if keyboard.pressed(KeyCode::KeyA) || keyboard.pressed(KeyCode::ArrowLeft) {
            camera_transform.translation.x -= delta;
        }
        if keyboard.pressed(KeyCode::KeyD) || keyboard.pressed(KeyCode::ArrowRight) {
            camera_transform.translation.x += delta;
        }
    }
}

// ===== INPUT SYSTEMS (for Phase 2) =====
// NOTE: The simple mouse_input_system has been replaced by unit_selection_system
// which provides more comprehensive click handling with unit selection logic.

// ===== MAIN MENU SYSTEMS (Phase 2) =====

/// Marker component for main menu UI entities
#[derive(Component)]
pub struct MainMenuUI;

/// Sets up the main menu UI
/// Runs when entering MainMenu state
pub fn setup_main_menu(mut commands: Commands) {
    info!("Setting up main menu");

    // Spawn UI container
    commands
        .spawn((
            Node {
                width: Val::Percent(100.0),
                height: Val::Percent(100.0),
                align_items: AlignItems::Center,
                justify_content: JustifyContent::Center,
                flex_direction: FlexDirection::Column,
                ..default()
            },
            BackgroundColor(Color::srgb(0.15, 0.15, 0.15)),
            MainMenuUI,
        ))
        .with_children(|parent| {
            // Title text
            parent.spawn((
                Text::new("Turn-Based Tactics"),
                TextFont {
                    font_size: 60.0,
                    ..default()
                },
                TextColor(Color::WHITE),
                Node {
                    margin: UiRect::all(Val::Px(20.0)),
                    ..default()
                },
            ));

            // Instruction text
            parent.spawn((
                Text::new("Press ENTER to start"),
                TextFont {
                    font_size: 30.0,
                    ..default()
                },
                TextColor(Color::srgb(0.7, 0.7, 0.7)),
                Node {
                    margin: UiRect::all(Val::Px(10.0)),
                    ..default()
                },
            ));

            // Phase info text
            parent.spawn((
                Text::new("Phase 5: Simple AI"),
                TextFont {
                    font_size: 20.0,
                    ..default()
                },
                TextColor(Color::srgb(0.5, 0.8, 0.5)),
                Node {
                    margin: UiRect::top(Val::Px(40.0)),
                    ..default()
                },
            ));
        });
}

/// Cleans up the main menu UI
/// Runs when exiting MainMenu state
pub fn cleanup_main_menu(mut commands: Commands, query: Query<Entity, With<MainMenuUI>>) {
    info!("Cleaning up main menu");

    // Despawn all menu UI entities (and their children)
    for entity in &query {
        commands.entity(entity).despawn();
    }
}

/// Handles input in the main menu
/// Pressing Enter transitions to GamePlay state
pub fn menu_input_system(
    keyboard: Res<ButtonInput<KeyCode>>,
    mut next_state: ResMut<NextState<AppState>>,
) {
    if keyboard.just_pressed(KeyCode::Enter) {
        info!("Starting game...");
        next_state.set(AppState::GamePlay);
    }
}

// ===== UNIT SYSTEMS (Phase 3) =====

/// Spawns initial units on the grid
/// Runs when entering GamePlay state
pub fn spawn_units(mut commands: Commands, grid_map: Res<GridMap>) {
    info!("Spawning units");

    // Spawn 2 player units (blue circles)
    let player_positions = vec![GridPosition::new(2, 2), GridPosition::new(3, 2)];

    for grid_pos in player_positions {
        let world_pos = grid_map.grid_to_world(&grid_pos);

        commands.spawn((
            Unit {
                faction: Faction::Player,
            },
            grid_pos,
            TurnStatus::default(), // Track if unit has acted this turn
            Sprite {
                color: PLAYER_COLOR,
                custom_size: Some(Vec2::new(UNIT_RADIUS * 2.0, UNIT_RADIUS * 2.0)),
                ..default()
            },
            Transform::from_xyz(world_pos.x, world_pos.y, Z_UNIT),
            Hoverable, // Can be hovered over with mouse
        ));
    }

    // Spawn 2 enemy units (red circles) - AI controlled
    let enemy_positions = vec![GridPosition::new(6, 7), GridPosition::new(7, 7)];

    for grid_pos in enemy_positions {
        let world_pos = grid_map.grid_to_world(&grid_pos);

        commands.spawn((
            Unit {
                faction: Faction::Enemy,
            },
            grid_pos,
            TurnStatus::default(), // Track if unit has acted this turn
            AIControlled,          // Mark as AI-controlled (Phase 5)
            Sprite {
                color: ENEMY_COLOR,
                custom_size: Some(Vec2::new(UNIT_RADIUS * 2.0, UNIT_RADIUS * 2.0)),
                ..default()
            },
            Transform::from_xyz(world_pos.x, world_pos.y, Z_UNIT),
            Hoverable,
        ));
    }

    info!("Spawned 4 units (2 player, 2 enemy)");
}

/// Handles unit selection with mouse clicks
/// Only player units can be selected
pub fn unit_selection_system(
    buttons: Res<ButtonInput<MouseButton>>,
    windows: Query<&Window>,
    camera_query: Query<(&Camera, &GlobalTransform), With<Camera2d>>,
    grid_map: Res<GridMap>,
    unit_query: Query<(Entity, &GridPosition, &Unit), With<Hoverable>>,
    selected_query: Query<Entity, With<Selected>>,
    mut commands: Commands,
    mut selection_state: ResMut<SelectionState>,
) {
    // Only process if left mouse button was just pressed
    if !buttons.just_pressed(MouseButton::Left) {
        return;
    }

    let Ok(window) = windows.single() else {
        return;
    };

    let Ok((camera, camera_transform)) = camera_query.single() else {
        return;
    };

    if let Some(cursor_pos) = window.cursor_position() {
        if let Ok(world_pos) = camera.viewport_to_world_2d(camera_transform, cursor_pos) {
            let clicked_grid_pos = grid_map.world_to_grid(world_pos);

            // Find if there's a unit at the clicked position
            let mut clicked_unit: Option<(Entity, &Unit)> = None;
            for (entity, unit_grid_pos, unit) in &unit_query {
                if unit_grid_pos.x == clicked_grid_pos.x && unit_grid_pos.y == clicked_grid_pos.y
                {
                    clicked_unit = Some((entity, unit));
                    break;
                }
            }

            // If we clicked on a unit
            if let Some((entity, unit)) = clicked_unit {
                // Only allow selecting player units
                if unit.faction == Faction::Player {
                    // Deselect previously selected unit
                    for selected_entity in &selected_query {
                        commands.entity(selected_entity).remove::<Selected>();
                    }

                    // Select the new unit
                    commands.entity(entity).insert(Selected);
                    selection_state.select_unit(entity);

                    info!(
                        "Selected player unit at ({}, {})",
                        clicked_grid_pos.x, clicked_grid_pos.y
                    );
                } else {
                    info!(
                        "Clicked enemy unit at ({}, {}) - cannot select",
                        clicked_grid_pos.x, clicked_grid_pos.y
                    );
                }
            }
            // Note: We DON'T deselect on empty tile clicks anymore
            // This allows movement_system to handle clicks on movement targets
            // Units stay selected until you select a different unit
        }
    }
}

/// Marker component for selection visual indicators
#[derive(Component)]
pub struct SelectionRing;

/// Adds visual feedback for selected units
/// Spawns a yellow ring around selected units
/// Only updates when selection changes
pub fn highlight_selected_system(
    mut commands: Commands,
    selected_query: Query<Entity, (With<Unit>, With<Selected>)>,
    ring_query: Query<Entity, With<SelectionRing>>,
    selection_state: Res<SelectionState>,
) {
    // Only update if selection state changed
    if !selection_state.is_changed() {
        return;
    }

    // Remove old selection rings
    for ring_entity in &ring_query {
        commands.entity(ring_entity).despawn();
    }

    // Add selection ring to currently selected unit
    if let Some(selected_entity) = selection_state.selected_unit {
        // Check if unit still has Selected component
        if selected_query.get(selected_entity).is_ok() {
            // Spawn a selection ring as a child of the unit
            commands.entity(selected_entity).with_children(|parent| {
                parent.spawn((
                    Sprite {
                        color: SELECTED_COLOR,
                        custom_size: Some(Vec2::new(
                            SELECTION_RING_RADIUS * 2.0,
                            SELECTION_RING_RADIUS * 2.0,
                        )),
                        ..default()
                    },
                    Transform::from_xyz(0.0, 0.0, Z_SELECTION - Z_UNIT), // Relative to parent
                    SelectionRing,
                ));
            });
        }
    }
}

// ===== TURN-BASED MOVEMENT SYSTEMS (Phase 4) =====

/// Marker component for movement highlight overlays
#[derive(Component)]
pub struct MovementHighlight;

/// Highlights valid movement tiles for the selected unit
/// Shows green overlay on adjacent tiles that are unoccupied
pub fn highlight_movement_system(
    mut commands: Commands,
    selected_query: Query<(&GridPosition, &Unit), With<Selected>>,
    highlight_query: Query<Entity, With<MovementHighlight>>,
    // Query all units to check for collisions
    all_player_units: Query<&GridPosition, (With<Unit>, Without<AIControlled>)>,
    ai_units: Query<&GridPosition, (With<Unit>, With<AIControlled>)>,
    selection_state: Res<SelectionState>,
    grid_map: Res<GridMap>,
    turn_state: Res<State<TurnState>>,
) {
    // Only update if selection changed
    if !selection_state.is_changed() {
        return;
    }

    // Remove old highlights
    for highlight_entity in &highlight_query {
        commands.entity(highlight_entity).despawn();
    }

    // Only show movement during player turn
    if *turn_state.get() != TurnState::PlayerTurn {
        return;
    }

    // Highlight valid moves for selected unit
    if let Some(selected_entity) = selection_state.selected_unit {
        if let Ok((grid_pos, _)) = selected_query.get(selected_entity) {
            // Get adjacent tiles (4-directional movement)
            let adjacent_positions = grid_pos.adjacent();

            for adj_pos in adjacent_positions {
                // Check if position is in bounds
                if !grid_map.is_in_bounds(&adj_pos) {
                    continue;
                }

                // **COLLISION DETECTION:** Only highlight unoccupied tiles
                let occupied_by_player = all_player_units.iter()
                    .any(|unit_pos| unit_pos.x == adj_pos.x && unit_pos.y == adj_pos.y);

                let occupied_by_ai = ai_units.iter()
                    .any(|unit_pos| unit_pos.x == adj_pos.x && unit_pos.y == adj_pos.y);

                // Skip occupied tiles - don't highlight them
                if occupied_by_player || occupied_by_ai {
                    continue;
                }

                let world_pos = grid_map.grid_to_world(&adj_pos);

                // Spawn highlight overlay
                commands.spawn((
                    Sprite {
                        color: MOVEMENT_HIGHLIGHT,
                        custom_size: Some(Vec2::new(TILE_SIZE, TILE_SIZE)),
                        ..default()
                    },
                    Transform::from_xyz(world_pos.x, world_pos.y, Z_OVERLAY),
                    MovementHighlight,
                ));
            }
        }
    }
}

/// Handles player unit movement on mouse click (Phase 4)
///
/// This system only runs during PlayerTurn state.
/// IMPORTANT: Must run AFTER unit_selection_system in the same frame to avoid race conditions.
///
/// Movement Rules:
/// - Only selected units can move
/// - Can only move to adjacent tiles (4-directional, no diagonals)
/// - Tiles must be within grid bounds
/// - After moving, unit is marked as "has_acted" for turn management
pub fn movement_system(
    buttons: Res<ButtonInput<MouseButton>>,
    windows: Query<&Window>,
    camera_query: Query<(&Camera, &GlobalTransform), With<Camera2d>>,
    grid_map: Res<GridMap>,
    // Query filters: With<Selected> = only selected units, Without<AIControlled> = exclude enemy units
    mut unit_query: Query<
        (&mut GridPosition, &mut Transform, &mut TurnStatus, &Unit),
        (With<Selected>, Without<AIControlled>),
    >,
    // Query for other player units (to check collisions)
    other_player_units: Query<&GridPosition, (With<Unit>, Without<Selected>, Without<AIControlled>)>,
    // Query for AI units (to check collisions)
    ai_units: Query<&GridPosition, (With<Unit>, With<AIControlled>)>,
    selection_state: Res<SelectionState>,
    turn_state: Res<State<TurnState>>,
) {
    // Only allow movement during player turn (AI moves during enemy turn)
    if *turn_state.get() != TurnState::PlayerTurn {
        return;
    }

    // Only process if left mouse button was just pressed (not held)
    if !buttons.just_pressed(MouseButton::Left) {
        return;
    }

    // Get window for cursor position
    let Ok(window) = windows.single() else {
        return;
    };

    // Get camera for screen-to-world conversion
    let Ok((camera, camera_transform)) = camera_query.single() else {
        return;
    };

    // Convert cursor position to grid coordinates
    if let Some(cursor_pos) = window.cursor_position() {
        if let Ok(world_pos) = camera.viewport_to_world_2d(camera_transform, cursor_pos) {
            let clicked_grid_pos = grid_map.world_to_grid(world_pos);

            // Try to move the selected unit (if one exists)
            if let Some(selected_entity) = selection_state.selected_unit {
                // Get the selected unit's components (this might fail if unit was just selected)
                if let Ok((mut unit_grid_pos, mut unit_transform, mut turn_status, _)) =
                    unit_query.get_mut(selected_entity)
                {
                    // **CRITICAL CHECK:** Unit can only move once per turn
                    if turn_status.has_acted {
                        info!("Unit has already acted this turn - cannot move again");
                        return;
                    }

                    // Check if clicked tile is adjacent to unit's current position
                    let adjacent_positions = unit_grid_pos.adjacent();
                    let is_adjacent = adjacent_positions
                        .iter()
                        .any(|pos| pos.x == clicked_grid_pos.x && pos.y == clicked_grid_pos.y);

                    // Execute movement if valid
                    if is_adjacent && grid_map.is_in_bounds(&clicked_grid_pos) {
                        // **COLLISION DETECTION:** Check if destination is occupied by any other unit
                        let occupied_by_player = other_player_units.iter()
                            .any(|unit_pos| unit_pos.x == clicked_grid_pos.x && unit_pos.y == clicked_grid_pos.y);

                        let occupied_by_ai = ai_units.iter()
                            .any(|unit_pos| unit_pos.x == clicked_grid_pos.x && unit_pos.y == clicked_grid_pos.y);

                        if occupied_by_player || occupied_by_ai {
                            info!("Cannot move to ({}, {}) - tile occupied by another unit",
                                clicked_grid_pos.x, clicked_grid_pos.y);
                            return;
                        }

                        // Calculate new world position for rendering
                        let new_world_pos = grid_map.grid_to_world(&clicked_grid_pos);

                        // Update grid position (logical position)
                        *unit_grid_pos = clicked_grid_pos;

                        // Update transform (visual position)
                        unit_transform.translation.x = new_world_pos.x;
                        unit_transform.translation.y = new_world_pos.y;

                        // Mark unit as having acted this turn
                        turn_status.has_acted = true;

                        info!("Player unit moved to ({}, {})", clicked_grid_pos.x, clicked_grid_pos.y);
                    }
                }
            }
        }
    }
}

/// Checks if all units have acted and transitions turn
pub fn check_turn_end_system(
    unit_query: Query<(&Unit, &TurnStatus)>,
    turn_state: Res<State<TurnState>>,
    mut next_turn_state: ResMut<NextState<TurnState>>,
    time: Res<Time>,
    mut enemy_timer: ResMut<EnemyTurnTimer>,
) {
    match turn_state.get() {
        TurnState::PlayerTurn => {
            // Check if all player units have acted
            let all_player_acted = unit_query
                .iter()
                .filter(|(unit, _)| unit.faction == Faction::Player)
                .all(|(_, status)| status.has_acted);

            if all_player_acted {
                info!("All player units have acted - switching to enemy turn");
                next_turn_state.set(TurnState::EnemyTurn);
            }
        }
        TurnState::EnemyTurn => {
            // Tick the timer
            enemy_timer.timer.tick(time.delta());

            // Only check for turn end after timer finishes
            if enemy_timer.timer.just_finished() {
                // Check if all enemy units have acted
                let all_enemy_acted = unit_query
                    .iter()
                    .filter(|(unit, _)| unit.faction == Faction::Enemy)
                    .all(|(_, status)| status.has_acted);

                if all_enemy_acted {
                    info!("All enemy units have acted - switching to player turn");
                    next_turn_state.set(TurnState::PlayerTurn);
                }
            }
        }
    }
}

/// Resets turn status for player units at start of player turn
pub fn start_player_turn(mut unit_query: Query<(&Unit, &mut TurnStatus)>) {
    info!("Starting player turn");

    for (unit, mut status) in &mut unit_query {
        if unit.faction == Faction::Player {
            status.has_acted = false;
        }
    }
}

/// Resets turn status for enemy units at start of enemy turn
/// AI will automatically move units during the enemy turn
pub fn start_enemy_turn(
    mut unit_query: Query<(&Unit, &mut TurnStatus)>,
    mut enemy_timer: ResMut<EnemyTurnTimer>,
) {
    info!("Starting enemy turn - AI will move units");

    // Reset the timer
    enemy_timer.timer.reset();

    // Reset turn status for enemy units
    for (unit, mut status) in &mut unit_query {
        if unit.faction == Faction::Enemy {
            status.has_acted = false;
        }
    }
}

// ===== TURN UI SYSTEMS (Phase 4) =====

/// Marker component for turn indicator UI
#[derive(Component)]
pub struct TurnIndicatorUI;

/// Sets up the turn indicator UI
pub fn setup_turn_ui(mut commands: Commands) {
    info!("Setting up turn UI");

    // Spawn turn indicator in top-left corner
    commands.spawn((
        Text::new("Player Turn"),
        TextFont {
            font_size: 30.0,
            ..default()
        },
        TextColor(PLAYER_COLOR),
        Node {
            position_type: PositionType::Absolute,
            left: Val::Px(20.0),
            top: Val::Px(20.0),
            ..default()
        },
        TurnIndicatorUI,
    ));
}

/// Updates turn indicator text based on current turn
pub fn update_turn_ui_system(
    mut query: Query<(&mut Text, &mut TextColor), With<TurnIndicatorUI>>,
    turn_state: Res<State<TurnState>>,
) {
    if !turn_state.is_changed() {
        return;
    }

    for (mut text, mut color) in &mut query {
        match turn_state.get() {
            TurnState::PlayerTurn => {
                **text = "Player Turn".to_string();
                *color = TextColor(PLAYER_COLOR);
            }
            TurnState::EnemyTurn => {
                **text = "Enemy Turn".to_string();
                *color = TextColor(ENEMY_COLOR);
            }
        }
    }
}

// ===== AI SYSTEMS (Phase 5) =====

/// Simple AI system that moves enemy units toward the nearest player unit (Phase 5)
///
/// AI Strategy:
/// 1. For each AI-controlled enemy unit
/// 2. Find the nearest player unit (using Manhattan distance)
/// 3. Move one tile closer to that player unit
/// 4. Mark unit as "has_acted" when done
///
/// This creates a simple "chase" behavior - enemies always move toward the closest player.
///
/// Only runs during EnemyTurn state. Skips units that have already acted.
///
/// Learning Notes:
/// - Uses Query<> with multiple filters: With<AIControlled> and With<Unit>
/// - Demonstrates pathfinding using "greedy" algorithm (always move closer)
/// - Manhattan distance: sum of horizontal + vertical distance (no diagonals)
pub fn ai_movement_system(
    // Query for AI units - get mutable access to position, transform, and turn status
    mut ai_query: Query<
        (Entity, &mut GridPosition, &mut Transform, &mut TurnStatus),
        (With<AIControlled>, With<Unit>),
    >,
    // Query for player units - only need to read their positions for targeting
    player_query: Query<&GridPosition, (With<Unit>, Without<AIControlled>)>,
    grid_map: Res<GridMap>,
    turn_state: Res<State<TurnState>>,
) {
    // Only run during enemy turn (player turn uses movement_system)
    if *turn_state.get() != TurnState::EnemyTurn {
        return;
    }

    // Collect all AI positions before mutating to check for collisions
    let ai_positions: Vec<(Entity, GridPosition)> = ai_query
        .iter()
        .map(|(entity, pos, _, _)| (entity, *pos))
        .collect();

    // Process each AI-controlled unit
    for (ai_entity, mut ai_pos, mut ai_transform, mut turn_status) in &mut ai_query {
        // Skip units that have already moved this turn
        if turn_status.has_acted {
            continue;
        }

        // === STEP 1: Find the nearest player unit to target ===
        let mut nearest_player_pos: Option<GridPosition> = None;
        let mut min_distance = u32::MAX;

        for player_pos in &player_query {
            // Calculate Manhattan distance (sum of x and y distances)
            let distance = ai_pos.distance_to(player_pos);
            if distance < min_distance {
                min_distance = distance;
                nearest_player_pos = Some(*player_pos);
            }
        }

        // === STEP 2: Move toward the target if one exists ===
        if let Some(target_pos) = nearest_player_pos {
            // Get all 4 adjacent tiles (up, down, left, right - no diagonals)
            let adjacent_positions = ai_pos.adjacent();

            // Find which adjacent tile gets us closest to the target
            // This is a "greedy" pathfinding algorithm - always move closer
            let mut best_move: Option<GridPosition> = None;
            let mut best_distance = ai_pos.distance_to(&target_pos);

            for adj_pos in adjacent_positions {
                // Check if tile is within grid bounds
                if !grid_map.is_in_bounds(&adj_pos) {
                    continue;
                }

                // **COLLISION DETECTION:** Check if position is occupied by any unit
                // Check player positions
                let occupied_by_player = player_query.iter()
                    .any(|player_pos| player_pos.x == adj_pos.x && player_pos.y == adj_pos.y);

                // Check other AI unit positions (not the current unit)
                let occupied_by_other_ai = ai_positions.iter()
                    .any(|(entity, ai_pos_check)| *entity != ai_entity && ai_pos_check.x == adj_pos.x && ai_pos_check.y == adj_pos.y);

                if occupied_by_player || occupied_by_other_ai {
                    continue;  // Skip occupied tiles - can't move through units
                }

                // Check if this move gets us closer to target
                let distance_from_adj = adj_pos.distance_to(&target_pos);
                if distance_from_adj < best_distance {
                    best_distance = distance_from_adj;
                    best_move = Some(adj_pos);
                }
            }

            // === STEP 3: Execute the move ===
            if let Some(new_pos) = best_move {
                let new_world_pos = grid_map.grid_to_world(&new_pos);

                info!(
                    "AI moving from ({}, {}) to ({}, {}) - approaching target at ({}, {})",
                    ai_pos.x, ai_pos.y, new_pos.x, new_pos.y, target_pos.x, target_pos.y
                );

                // Update grid position (logical)
                *ai_pos = new_pos;

                // Update world position (visual)
                ai_transform.translation.x = new_world_pos.x;
                ai_transform.translation.y = new_world_pos.y;

                // Mark unit as having acted this turn
                turn_status.has_acted = true;
            } else {
                // No better position found (unit is already adjacent or blocked)
                info!("AI unit at ({}, {}) has no valid moves", ai_pos.x, ai_pos.y);
                turn_status.has_acted = true;
            }
        } else {
            // No player units found (shouldn't happen in normal gameplay)
            turn_status.has_acted = true;
        }
    }
}
