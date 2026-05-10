# Phase 2: State Management and Input

**Status:** ✅ Complete
**Goal:** Learn Bevy's state system and input handling

## Overview

Phase 2 introduces Bevy's powerful state management system. We create a main menu and learn how to transition between different game states, making our systems run conditionally based on the current state.

## What We Built

### State Definition

#### `AppState` Enum ([src/main.rs:23-28](../src/main.rs))

```rust
#[derive(States, Debug, Clone, PartialEq, Eq, Hash, Default)]
pub enum AppState {
    #[default]
    MainMenu,   // Main menu screen (press Enter to start)
    GamePlay,   // Active gameplay
}
```

**Key Traits:**
- `States` - Marks this as a Bevy state machine
- `Default` - MainMenu is the starting state (`#[default]`)
- `Clone`, `PartialEq`, `Eq`, `Hash` - Required for state management

**Purpose:** Controls the overall game flow. The game starts in `MainMenu` and transitions to `GamePlay` when the user presses Enter.

### State Initialization

```rust
App::new()
    .init_state::<AppState>()  // Initialize the state machine
```

This creates the state machine and sets it to `AppState::default()` (MainMenu).

### Systems Added

#### `setup_main_menu` ([src/systems.rs:177-235](../src/systems.rs))

```rust
pub fn setup_main_menu(mut commands: Commands) {
    commands
        .spawn((
            Node { /* ... */ },
            BackgroundColor(Color::srgb(0.15, 0.15, 0.15)),
            MainMenuUI,  // Marker component for cleanup
        ))
        .with_children(|parent| {
            // Spawn text entities as children
            parent.spawn(Text::new("Turn-Based Tactics"));
            parent.spawn(Text::new("Press ENTER to start"));
            // ...
        });
}
```

**When:** Runs when entering `MainMenu` state (`OnEnter(AppState::MainMenu)`)

**What:** Creates a UI hierarchy with text elements

**New Bevy Concepts:**
1. **Node component** - Defines UI layout (flexbox-based)
2. **Parent-child relationships** - `.with_children()` creates a hierarchy
3. **Text component** - Renders text on screen
4. **Marker component** - `MainMenuUI` used to identify menu entities for cleanup

**UI Layout:**
- Full-screen dark gray background
- Centered column layout with:
  - Title text (60px, white)
  - Instructions (30px, gray)
  - Phase info (20px, green)

#### `cleanup_main_menu` ([src/systems.rs:237-246](../src/systems.rs))

```rust
pub fn cleanup_main_menu(mut commands: Commands, query: Query<Entity, With<MainMenuUI>>) {
    for entity in &query {
        commands.entity(entity).despawn();
    }
}
```

**When:** Runs when exiting `MainMenu` state (`OnExit(AppState::MainMenu)`)

**What:** Removes all menu UI entities

**Why This Matters:** State cleanup is crucial. Without this, menu UI would stay on screen during gameplay! `OnExit` systems ensure proper cleanup when transitioning states.

#### `menu_input_system` ([src/systems.rs:248-259](../src/systems.rs))

```rust
pub fn menu_input_system(
    keyboard: Res<ButtonInput<KeyCode>>,
    mut next_state: ResMut<NextState<AppState>>,
) {
    if keyboard.just_pressed(KeyCode::Enter) {
        info!("Starting game...");
        next_state.set(AppState::GamePlay);
    }
}
```

**When:** Runs every frame during `MainMenu` state (`.run_if(in_state(AppState::MainMenu))`)

**What:** Detects Enter key and transitions to GamePlay

**State Transition Pattern:**
- Use `ResMut<NextState<AppState>>` to change states
- Call `.set()` with the new state
- Transition happens at end of current frame
- `OnExit` systems run, then `OnEnter` systems for new state

**Input Pattern:**
- `keyboard.just_pressed()` - Detects single key press (not continuous)
- Better than `.pressed()` for menu navigation

### Modified Systems

All gameplay systems now run conditionally:

```rust
.add_systems(
    Update,
    (mouse_input_system, camera_pan_system).run_if(in_state(AppState::GamePlay)),
)
```

**Before Phase 2:** Systems ran every frame
**After Phase 2:** Systems only run in the appropriate state

## State Lifecycle

```
App Start
    │
    ▼
MainMenu State
    │
    ├─ OnEnter(MainMenu) → setup_main_menu
    │
    ├─ Update (if in MainMenu) → menu_input_system
    │      │
    │      └─ Enter pressed → NextState::set(GamePlay)
    │
    ├─ OnExit(MainMenu) → cleanup_main_menu
    │
    ▼
GamePlay State
    │
    ├─ OnEnter(GamePlay) → setup_grid, center_camera
    │
    └─ Update (if in GamePlay) → mouse_input_system, camera_pan_system
```

## Key Bevy Concepts Learned

### 1. State Machines

**Define States:**
```rust
#[derive(States, Default, ...)]
enum MyState {
    #[default]
    Start,
    Playing,
    Paused,
}
```

**Initialize:**
```rust
app.init_state::<MyState>()
```

**Transition:**
```rust
fn my_system(mut next_state: ResMut<NextState<MyState>>) {
    next_state.set(MyState::Playing);
}
```

### 2. State-Based Scheduling

**OnEnter** - Runs once when entering a state:
```rust
.add_systems(OnEnter(AppState::GamePlay), setup_game)
```

**OnExit** - Runs once when exiting a state:
```rust
.add_systems(OnExit(AppState::GamePlay), cleanup_game)
```

**Update with run_if** - Runs every frame in a state:
```rust
.add_systems(Update, gameplay_system.run_if(in_state(AppState::GamePlay)))
```

### 3. UI System Basics

**Node Component** - Layout container:
```rust
Node {
    width: Val::Percent(100.0),      // Full width
    height: Val::Percent(100.0),     // Full height
    align_items: AlignItems::Center, // Center vertically
    justify_content: JustifyContent::Center, // Center horizontally
    flex_direction: FlexDirection::Column,   // Stack children vertically
    ..default()
}
```

**Text Components:**
```rust
Text::new("My text"),
TextFont { font_size: 30.0, ..default() },
TextColor(Color::WHITE),
```

**Parent-Child Hierarchy:**
```rust
commands.spawn(/* parent */)
    .with_children(|parent| {
        parent.spawn(/* child 1 */);
        parent.spawn(/* child 2 */);
    });
```

### 4. Input Detection Patterns

**Single Press (Menu Navigation):**
```rust
if keyboard.just_pressed(KeyCode::Enter) {
    // Fires once per key press
}
```

**Continuous (Movement):**
```rust
if keyboard.pressed(KeyCode::KeyW) {
    // Fires every frame while held
}
```

**Released:**
```rust
if keyboard.just_released(KeyCode::Space) {
    // Fires once when key is released
}
```

## Changes from Phase 1

### main.rs

**Added:**
- `AppState` enum definition
- `.init_state::<AppState>()`
- `OnEnter` and `OnExit` system scheduling
- `.run_if(in_state(...))` for conditional systems

**Removed:**
- Direct `Startup` scheduling for grid setup (moved to `OnEnter(GamePlay)`)

### systems.rs

**Added:**
- `MainMenuUI` marker component
- `setup_main_menu` system
- `cleanup_main_menu` system
- `menu_input_system` system
- `use crate::AppState;` import

**Modified:**
- None - existing systems unchanged, just scheduled differently

## How to Test

1. **Run the game:** `cargo run`
2. **Main Menu:** You should see:
   - Dark gray background
   - "Turn-Based Tactics" title
   - "Press ENTER to start" instruction
   - "Phase 2: State Management" info
3. **Press Enter:** Game transitions to gameplay
4. **Verify Grid:** 10×10 grid should appear
5. **Test Input:** WASD to pan, click tiles to see coordinates

## State Transition Details

When you press Enter in the main menu:

```
1. menu_input_system detects Enter
2. Sets NextState to AppState::GamePlay
3. Current frame completes
4. OnExit(MainMenu) runs:
   - cleanup_main_menu despawns menu UI
5. State changes to GamePlay
6. OnEnter(GamePlay) runs:
   - setup_grid spawns 100 tiles
   - center_camera positions camera
7. Update systems with run_if(GamePlay) start running
8. Menu systems stop running (different state)
```

## Common Patterns

### Multiple States

You can have multiple state machines:
```rust
#[derive(States, ...)]
enum AppState { Menu, Playing, Paused }

#[derive(States, ...)]
enum GameMode { Story, Endless, Tutorial }

app.init_state::<AppState>()
   .init_state::<GameMode>();
```

### State-Specific Resources

Initialize resources only when needed:
```rust
.add_systems(OnEnter(AppState::GamePlay), |mut commands: Commands| {
    commands.init_resource::<GameplayResource>();
})
.add_systems(OnExit(AppState::GamePlay), |mut commands: Commands| {
    commands.remove_resource::<GameplayResource>();
})
```

### State Queries

Check current state in systems:
```rust
fn my_system(state: Res<State<AppState>>) {
    match state.get() {
        AppState::Menu => { /* ... */ },
        AppState::GamePlay => { /* ... */ },
    }
}
```

Better: Use `.run_if()` instead of checking inside system!

## Best Practices

### ✅ Do This

```rust
// Use OnEnter/OnExit for state-specific setup/cleanup
.add_systems(OnEnter(AppState::Menu), setup_menu)
.add_systems(OnExit(AppState::Menu), cleanup_menu)

// Use run_if for conditional systems
.add_systems(Update, gameplay_logic.run_if(in_state(AppState::Playing)))

// Use marker components for cleanup
#[derive(Component)]
struct MenuUI;

// Despawn by querying marker
fn cleanup(mut commands: Commands, query: Query<Entity, With<MenuUI>>) {
    for entity in &query {
        commands.entity(entity).despawn();
    }
}
```

### ❌ Don't Do This

```rust
// Don't check state manually in systems
fn bad_system(state: Res<State<AppState>>) {
    if *state.get() == AppState::Menu {  // Use .run_if() instead!
        // ...
    }
}

// Don't forget cleanup
.add_systems(OnEnter(AppState::Menu), setup_menu)
// Missing OnExit cleanup - menu UI will stay on screen!

// Don't manually track entities for cleanup
static mut MENU_ENTITIES: Vec<Entity> = Vec::new();  // Don't do this!
// Use marker components and queries instead
```

## What's Next?

In **Phase 3**, we'll add:
- Unit entities (player and enemy)
- Selection system (click to select units)
- Visual feedback (highlight selected units)
- Faction-aware selection (only select your units)

This will teach you about:
- Query filters (`With<T>`, `Without<T>`)
- Marker components for selection
- Dynamic component insertion/removal
- Entity hierarchy for visual effects

## Exercises

1. **Add Pause State:** Create a `Paused` state that shows "PAUSED - Press ESC to resume"
2. **State Counter:** Display "Games Played: X" on main menu that increments each time you return from gameplay
3. **Fade Transition:** Make the menu fade out when transitioning (use a timer and alpha value)
4. **Settings Menu:** Add a second menu state for game settings
5. **Quit Button:** Add a "Press Q to Quit" option that closes the window

## Resources

- [Bevy States Guide](https://docs.rs/bevy/0.18.0/bevy/state/index.html)
- [Bevy UI Guide](https://docs.rs/bevy/0.18.0/bevy/ui/index.html)
- [State Management Discussion](https://github.com/bevyengine/bevy/discussions/8234)
