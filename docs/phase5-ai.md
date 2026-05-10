# Phase 5: Simple AI

**Status:** ✅ Complete
**Goal:** Learn system scheduling, AI decision-making, and pathfinding in ECS

## Overview

Phase 5 adds AI-controlled enemy units that automatically move toward player units during the enemy turn. This introduces autonomous behavior and demonstrates how to implement game AI using Bevy's ECS architecture.

## What We Built

### New Component

#### `AIControlled` Marker ([src/components.rs:109-110](../src/components.rs))

```rust
#[derive(Component, Debug)]
pub struct AIControlled;
```

**Purpose:** Tags enemy units as AI-controlled. This marker allows systems to distinguish between player-controlled and AI-controlled units.

**Pattern:** Zero-sized marker component - contains no data, only used for filtering queries.

### Updated Systems

#### `spawn_units` - Modified ([src/systems.rs:287-309](../src/systems.rs))

Added `AIControlled` component to enemy units:

```rust
commands.spawn((
    Unit { faction: Faction::Enemy },
    grid_pos,
    TurnStatus::default(),
    AIControlled,  // <- NEW: Marks this unit as AI-controlled
    // ... other components
));
```

#### `start_enemy_turn` - Simplified ([src/systems.rs:616-633](../src/systems.rs))

Removed placeholder code that auto-passed enemy turn. Now AI actually moves units:

```rust
pub fn start_enemy_turn(
    mut unit_query: Query<(&Unit, &mut TurnStatus)>,
    mut enemy_timer: ResMut<EnemyTurnTimer>,
) {
    info!("Starting enemy turn - AI will move units");

    // Reset timer for visual delay
    enemy_timer.timer.reset();

    // Reset turn status for all enemy units
    for (unit, mut status) in &mut unit_query {
        if unit.faction == Faction::Enemy {
            status.has_acted = false;
        }
    }
}
```

### New System

#### `ai_movement_system` ([src/systems.rs:695-776](../src/systems.rs))

The core AI logic that moves enemy units toward player units.

**When it runs:** Every frame during `EnemyTurn` state

**What it does:**
1. Find nearest player unit (using Manhattan distance)
2. Calculate which adjacent tile gets closer to target
3. Move unit to that tile
4. Mark unit as "has_acted"

**Key Query:**
```rust
mut ai_query: Query<
    (&mut GridPosition, &mut Transform, &mut TurnStatus),
    (With<AIControlled>, With<Unit>),
>,
```

**Filters:**
- `With<AIControlled>` - Only AI units
- `With<Unit>` - Must be a unit (not a tile)
- Mutable access to position, transform, and turn status

**AI Algorithm (Greedy Pathfinding):**

```
For each AI unit:
  1. Find all player units
  2. Calculate distance to each
  3. Target = closest player unit
  4. Get all 4 adjacent tiles
  5. Find adjacent tile that minimizes distance to target
  6. Move to that tile
  7. Mark as acted
```

This is called a "greedy" algorithm because it always makes the locally optimal choice (move closer) without considering long-term strategy.

### System Ordering Fix

**Critical Fix:** Added `.chain()` to main.rs system registration

**Before (Phase 4):**
```rust
.add_systems(
    Update,
    (
        unit_selection_system,
        movement_system,
        // ... other systems
    )
    .run_if(in_state(AppState::GamePlay)),
)
```

**Problem:** Systems run in parallel → race condition where `movement_system` tries to read `Selected` component before `unit_selection_system` adds it.

**After (Phase 5):**
```rust
.add_systems(
    Update,
    (
        unit_selection_system,  // Must run FIRST
        movement_system,        // Must run AFTER selection
        // ... other systems
    )
        .chain()  // CRITICAL: Forces sequential execution
        .run_if(in_state(AppState::GamePlay)),
)
```

**Why this matters:**
- Without `.chain()`, Bevy runs systems in parallel for performance
- Parallel execution can cause race conditions when systems depend on each other
- `.chain()` guarantees systems run in the order listed
- Trade-off: Slightly slower (sequential) but correct behavior

## Key Bevy Concepts Learned

### 1. System Scheduling and Ordering

**Parallel vs Sequential:**

```rust
// Parallel (default) - systems run simultaneously
.add_systems(Update, (system_a, system_b, system_c))

// Sequential - systems run in order
.add_systems(Update, (system_a, system_b, system_c).chain())
```

**When to use `.chain()`:**
- System B reads data that System A writes
- Order of execution matters for correctness
- Example: Selection must happen before movement

**When to use parallel (no .chain()):**
- Systems are independent
- Order doesn't matter
- Example: Camera movement and UI rendering

### 2. Query Filters for AI

**Multiple Filter Combinations:**

```rust
// AI units only
Query<Components, (With<AIControlled>, With<Unit>)>

// Player units only
Query<Components, (With<Unit>, Without<AIControlled>)>

// Selected player units only
Query<Components, (With<Selected>, Without<AIControlled>)>
```

**Pattern:** Use marker components to partition entities into groups.

### 3. Manhattan Distance

**Definition:** Sum of horizontal and vertical distance (no diagonals)

```rust
pub fn distance_to(&self, other: &GridPosition) -> u32 {
    ((self.x - other.x).abs() + (self.y - other.y).abs()) as u32
}
```

**Example:**
```
From (2,2) to (5,6):
  X distance: |2 - 5| = 3
  Y distance: |2 - 6| = 4
  Manhattan distance: 3 + 4 = 7
```

**Why Manhattan distance?**
- Matches our 4-directional movement (up, down, left, right)
- Diagonal distance would be `sqrt((x2-x1)² + (y2-y1)²)` - doesn't match grid movement

### 4. Greedy Algorithm for Pathfinding

**Greedy Strategy:** Always make the locally optimal choice

**Pros:**
- Simple to implement
- Fast (no complex calculations)
- Good enough for simple chase behavior

**Cons:**
- Can get stuck on obstacles (not implemented yet)
- Doesn't consider future moves
- Not optimal for complex mazes

**Better alternatives (for later):**
- A* (A-star) pathfinding
- Breadth-first search (BFS)
- Dijkstra's algorithm

### 5. AI State Machine

**Current AI States:**
```
Enemy Turn Start:
  → has_acted = false for all enemy units
  → Timer starts (1.5 seconds)

AI Execution:
  → Find target
  → Calculate best move
  → Execute move
  → has_acted = true

Turn End Check:
  → If all enemies have acted → Player Turn
```

## How to Test

1. **Run the game:** `cargo run`
2. **Start gameplay:** Press Enter
3. **Move both player units:** Click unit → click adjacent tile (repeat twice)
4. **Observe enemy turn:**
   - Screen shows "Enemy Turn" (red text)
   - Wait 1.5 seconds
   - Both red units move ONE tile closer to nearest blue unit
5. **Check logs:**
   ```
   AI moving from (6, 7) to (5, 7) - approaching target at (2, 3)
   ```
6. **Verify turn switch:** After AI moves, turn returns to "Player Turn"

## What Changed from Phase 4

### components.rs
- Added `AIControlled` marker component (line 109-110)

### systems.rs
- Added `AIControlled` import (line 6)
- Modified `spawn_units`: Added `AIControlled` to enemy units (line 299)
- Modified `start_enemy_turn`: Removed auto-pass logic (line 616-633)
- Added `ai_movement_system`: New AI logic (line 695-776)
- Updated menu text: "Phase 5: Simple AI" (line 221)

### main.rs
- Updated file comment: "Phase 5" (line 6)
- Updated window title: "Phase 5: Simple AI" (line 43)
- Added `ai_movement_system` to Update systems (line 82)
- **CRITICAL:** Added `.chain()` to prevent race conditions (line 87)

## AI Behavior Examples

### Example 1: Simple Chase

```
Initial Setup:
  Player at (2, 2)
  Enemy at (6, 6)

Turn 1 (Enemy):
  Distance from (6,6) to (2,2) = 8
  Adjacent tiles:
    (5,6): distance = 7 ✓ BEST
    (7,6): distance = 9
    (6,5): distance = 7 ✓ TIED
    (6,7): distance = 9

  Enemy moves to (5,6) - gets closer!

Turn 2 (Enemy):
  Distance from (5,6) to (2,2) = 7
  Enemy moves to (4,6)

... continues until adjacent to player
```

### Example 2: Multiple Targets

```
Setup:
  Player1 at (2, 2)
  Player2 at (8, 8)
  Enemy at (5, 5)

AI Decision:
  Distance to Player1: |5-2| + |5-2| = 6
  Distance to Player2: |5-8| + |5-8| = 6

  Target = Player1 (first found)

Enemy moves toward (2, 2)
```

## Common Patterns

### Pattern 1: Marker Components for Behavior

```rust
// Define behavior markers
#[derive(Component)]
struct AIControlled;

#[derive(Component)]
struct PlayerControlled;

// Use in queries
Query<&Unit, With<AIControlled>>     // AI units
Query<&Unit, Without<AIControlled>>  // Player units
```

### Pattern 2: State-Dependent Systems

```rust
pub fn ai_system(..., turn_state: Res<State<TurnState>>) {
    if *turn_state.get() != TurnState::EnemyTurn {
        return;  // Don't run during player turn
    }
    // AI logic here
}
```

### Pattern 3: Target Selection

```rust
// Find nearest target
let mut nearest: Option<Entity> = None;
let mut min_dist = u32::MAX;

for target in &targets {
    let dist = current_pos.distance_to(target);
    if dist < min_dist {
        min_dist = dist;
        nearest = Some(target);
    }
}
```

## Best Practices

### ✅ Do This

```rust
// Use marker components for AI identification
#[derive(Component)]
struct AIControlled;

// Chain systems when order matters
.add_systems(Update, (select, move, ai).chain())

// Check turn state in AI systems
if *turn_state.get() != TurnState::EnemyTurn { return; }

// Skip units that have already acted
if turn_status.has_acted { continue; }
```

### ❌ Don't Do This

```rust
// Don't hardcode AI behavior in turn start
pub fn start_enemy_turn(...) {
    // Move all units here - NO!
    // Let ai_movement_system handle it
}

// Don't forget to check turn state
pub fn ai_system(...) {
    // AI runs every frame - WRONG!
    // Should only run during enemy turn
}

// Don't use Euclidean distance for grid movement
let dist = sqrt((x2-x1)² + (y2-y1)²);  // NO!
let dist = abs(x2-x1) + abs(y2-y1);    // YES!
```

## What's Next?

**Phase 6 (Optional): Basic Combat**
- Add `Stats` component (HP, attack, defense)
- Combat when units move onto same tile
- Death and entity despawning
- Win/lose conditions
- Health bars

This will teach:
- Event systems
- Entity despawning
- UI rendering (health bars)
- Game over states

## Exercises

1. **Modify AI Behavior:** Make enemies flee from player instead of chase
2. **Smart AI:** Make AI choose the weakest player unit as target
3. **Random Movement:** Add 20% chance for AI to move randomly
4. **AI Speed:** Make AI units move twice per turn
5. **Defensive AI:** Make AI units protect each other (stay close)

## Resources

- [Bevy Query Guide](https://docs.rs/bevy/0.18.0/bevy/ecs/query/index.html)
- [System Ordering](https://bevy-cheatbook.github.io/programming/system-order.html)
- [Pathfinding Algorithms](https://www.redblobgames.com/pathfinding/a-star/introduction.html)
- [Manhattan Distance](https://en.wikipedia.org/wiki/Taxicab_geometry)
