# Testing Guide for Bevy Turn-Based Game

This document provides systematic tests for each phase of the game to ensure everything works correctly.

## How to Run Tests

```bash
cargo run
```

Watch the terminal logs for `INFO` messages that show what's happening.

## Phase 5: Simple AI - Test Plan

### Test 1: Game Startup
**Steps:**
1. Run `cargo run`
2. Observe main menu

**Expected:**
- Window title: "Turn-Based Tactics - Phase 5: Simple AI"
- Menu shows "Phase 5: Simple AI"
- Log: `Setting up main menu`

**Pass/Fail:** ___

---

### Test 2: Grid and Unit Spawning
**Steps:**
1. From main menu, press Enter
2. Observe grid and units

**Expected:**
- 10×10 checkerboard grid appears
- 2 blue circles at positions (2,2) and (3,2) - PLAYER UNITS
- 2 red circles at positions (6,7) and (7,7) - ENEMY UNITS
- Top-left shows "Player Turn" in blue
- Logs show:
  ```
  Grid setup complete: 100 tiles spawned
  Spawned 4 units (2 player, 2 enemy)
  Starting player turn
  ```

**Pass/Fail:** ___

---

### Test 3: Unit Selection
**Steps:**
1. Click on blue unit at (2,2)
2. Observe visual feedback

**Expected:**
- Yellow ring appears around selected unit
- Green tiles appear on 4 adjacent squares (up, down, left, right)
- Log: `Selected player unit at (2, 2)`

**Pass/Fail:** ___

---

### Test 4: Player Unit Movement
**Steps:**
1. With unit at (2,2) selected, click on adjacent tile (2,3)
2. Observe unit movement

**Expected:**
- Blue unit moves from (2,2) to (2,3)
- Unit moves smoothly (no jumping)
- Log: `Player unit moved to (2, 3)`
- Yellow selection ring and green highlights update to new position

**Pass/Fail:** ___

---

### Test 5: Movement Restrictions
**Steps:**
1. Select unit at (2,3)
2. Click on NON-adjacent tile (5,5)

**Expected:**
- Unit does NOT move
- No error logs
- Unit remains at (2,3)

**Pass/Fail:** ___

---

### Test 6: Turn Transition
**Steps:**
1. Move both blue units (one click each)
2. Wait and observe

**Expected:**
- After second unit moves, log shows: `All player units have acted - switching to enemy turn`
- Top-left changes to "Enemy Turn" in red
- Wait 1.5 seconds (enemy timer delay)
- Log: `Starting enemy turn - AI will move units`

**Pass/Fail:** ___

---

### Test 7: AI Movement Behavior
**Steps:**
1. Continue from Test 6 (enemy turn active)
2. Watch red enemy units

**Expected:**
- Both red units move ONE tile closer to nearest blue unit
- Logs show (for each AI unit):
  ```
  AI moving from (X, Y) to (X2, Y2) - approaching target at (X3, Y3)
  ```
- AI units use Manhattan distance (move horizontally OR vertically, not diagonally)
- After both AI units move, turn switches back to "Player Turn"

**Pass/Fail:** ___

---

### Test 8: Full Turn Cycle
**Steps:**
1. Play for 3 complete rounds (player turn → enemy turn → repeat)
2. Observe AI behavior

**Expected:**
- Turns alternate correctly: Player → Enemy → Player → Enemy
- AI units consistently move toward player units
- No crashes or errors
- Turn indicator updates correctly each time

**Pass/Fail:** ___

---

### Test 9: Camera Controls
**Steps:**
1. During gameplay, press W, A, S, D keys
2. Observe camera movement

**Expected:**
- W: Camera pans up
- S: Camera pans down
- A: Camera pans left
- D: Camera pans right
- Units and grid move smoothly with camera
- No lag or stuttering

**Pass/Fail:** ___

---

### Test 10: Edge Cases
**Steps:**
1. Move player unit to grid edge (0,0 or 9,9)
2. Try to move beyond grid boundary

**Expected:**
- Movement restricted to grid bounds
- No crash or error when clicking outside grid
- Unit cannot move outside 10×10 grid

**Pass/Fail:** ___

---

## Common Issues and Solutions

### Issue: "Movement system: Failed to get unit from query"
**Cause:** Race condition between selection and movement systems
**Solution:** Ensure systems are `.chain()`ed in main.rs

### Issue: Units don't move at all
**Cause:** Query filters might be too restrictive
**Solution:** Check that `Without<AIControlled>` is on player movement query

### Issue: AI units don't move
**Cause:** AI system not running or enemy turn not triggering
**Solution:** Check logs for "Starting enemy turn" message

### Issue: Turn doesn't switch
**Cause:** Not all units marked as "has_acted"
**Solution:** Check that movement sets `turn_status.has_acted = true`

---

## Expected Log Output for One Full Turn

```
INFO bevy_game::systems: Starting player turn
INFO bevy_game::systems: Selected player unit at (2, 2)
INFO bevy_game::systems: Player unit moved to (2, 3)
INFO bevy_game::systems: Selected player unit at (3, 2)
INFO bevy_game::systems: Player unit moved to (3, 3)
INFO bevy_game::systems: All player units have acted - switching to enemy turn
INFO bevy_game::systems: Starting enemy turn - AI will move units
INFO bevy_game::systems: AI moving from (6, 7) to (5, 7) - approaching target at (2, 3)
INFO bevy_game::systems: AI moving from (7, 7) to (6, 7) - approaching target at (3, 3)
INFO bevy_game::systems: All enemy units have acted - switching to player turn
INFO bevy_game::systems: Starting player turn
```

---

## Success Criteria

All 10 tests must pass for Phase 5 to be considered complete.

**Test Results:**
- Total Tests: 10
- Passed: ___
- Failed: ___
- Success Rate: ___%
