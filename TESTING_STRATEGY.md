# Testing Strategy for Bevy Turn-Based Game

## Current Bugs Found

### Critical Bugs
1. **Player can move multiple times in one turn**
   - Status: NEEDS FIX
   - Test: `test_unit_cannot_move_twice_in_one_turn` (PASSING - but doesn't catch the real issue)
   - Root cause: Movement system doesn't check `has_acted` before allowing movement

2. **Highlights don't update when selection changes**
   - Status: NEEDS FIX
   - Test: `test_selection_updates_highlights` (FAILING)
   - Root cause: Highlight system depends on change detection which doesn't work properly in tests

3. **Turn transitions may not work correctly**
   - Status: NEEDS INVESTIGATION
   - Test: `test_turn_transition_after_all_units_move` (FAILING)

## Test Categories

### 1. Unit Tests (`tests/movement_tests.rs`)
**Purpose**: Test individual functions and pure logic
**Status**: ✅ 7/7 passing
**Coverage**:
- Grid position calculations (adjacency, distance)
- Coordinate conversions
- Bounds checking
- Basic collision detection logic

**Limitations**:
- Don't test system interactions
- Don't catch timing/ordering issues
- Don't test actual gameplay flow

### 2. Integration Tests (`tests/integration_tests.rs`)
**Purpose**: Test multiple systems working together
**Status**: ⚠️ 3/6 passing
**Coverage**:
- Multi-system interactions
- Turn-based gameplay flow
- AI behavior
- Selection and highlighting coordination

**Current Failures**:
- Highlight systems (change detection issues in tests)
- Turn transition logic

**Limitations**:
- Hard to simulate actual user input (mouse clicks, keyboard)
- Bevy's change detection doesn't work the same in tests
- Can't test visual/rendering issues

### 3. Manual Testing
**Purpose**: Verify actual gameplay experience
**Status**: ❌ Major bugs found
**Required Tests**:
- [ ] Player movement (one move per turn)
- [ ] Highlight follows selection
- [ ] AI moves correctly
- [ ] Turn transitions work
- [ ] Collision detection prevents overlaps
- [ ] Camera controls work
- [ ] UI updates correctly

## Known Test Limitations

### Bevy Change Detection in Tests
**Problem**: Systems that use `.is_changed()` don't work in tests because:
- Resources inserted before `app.update()` aren't marked as changed
- Need to manually trigger change detection

**Solutions**:
1. Use `ResMut` to modify resources (marks as changed)
2. Add systems that modify resources instead of direct insertion
3. Test the underlying logic without change detection

### Mouse Input Simulation
**Problem**: Can't easily simulate mouse clicks in integration tests
**Current Workaround**: Manually set positions and status
**Better Solution**: Need input event simulation

### Visual Bugs
**Problem**: Can't test rendering or visual feedback
**Solution**: Manual testing + screenshots

## Testing Strategy Going Forward

### Immediate Priorities
1. **Fix movement_system** to check `has_acted` before allowing movement
2. **Fix highlight tests** to work with change detection
3. **Add manual test checklist** for each PR/commit

### Short Term
1. Add input simulation helpers for integration tests
2. Create test utilities for common setup (spawn units, select units, etc.)
3. Add more edge case tests (grid boundaries, blocked units, etc.)

### Long Term
1. Consider property-based testing (QuickCheck style)
2. Add performance benchmarks
3. Consider visual regression testing for UI

## Test Running Guide

```bash
# Run all tests
cargo test

# Run specific test file
cargo test --test integration_tests

# Run specific test
cargo test test_unit_cannot_move_twice_in_one_turn

# Run with output
cargo test -- --nocapture

# Run and show exactly what failed
cargo test -- --test-threads=1 --nocapture
```

## Bug Tracking

### To Fix
- [ ] Movement system: Add `has_acted` check before allowing movement
- [ ] Highlight system: Fix change detection or refactor to not depend on it
- [ ] Turn system: Investigate why transitions fail in tests
- [ ] Selection: Ensure highlights update when selection changes

### To Test
- [ ] Units cannot move after acting
- [ ] Highlights show only valid moves
- [ ] Highlights update when switching selection
- [ ] AI doesn't move during player turn
- [ ] Turn switches after all units act
- [ ] Collision prevents movement onto occupied tiles

## Manual Testing Checklist

Before each commit:
- [ ] Run `cargo test` - all tests pass
- [ ] Run `cargo run` - game starts without panics
- [ ] Select a unit - yellow ring appears, green highlights show
- [ ] Move unit once - unit moves, highlights disappear
- [ ] Try to move same unit again - should NOT move
- [ ] Select different unit - highlights move to new unit
- [ ] Move both units - turn should switch to Enemy
- [ ] Wait for AI - AI units should move toward player
- [ ] New player turn - units can move again

## Notes

- Integration tests are closer to real gameplay but harder to debug
- Unit tests are faster but may miss interaction bugs
- **Always do manual testing** - automated tests alone aren't enough
- Consider adding debug logs to systems during testing
