//! Explicit life-state transitions and bounded, game-owned death policy.

use serde::{Deserialize, Serialize};

/// Corpse lifetime counts complete rounds after the creation round.
pub const CORPSE_ROUNDS: u32 = 3;
/// Prototype death save: a seeded d20 roll at least this high holds on.
pub const DEATH_SAVE_TARGET: u8 = 10;
/// Cumulative failed saves while dying before permanent death.
pub const DEATH_SAVE_FAILURES: u8 = 3;

/// Living HP and corpse HP are deliberately separate pools.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub enum LifeState {
    /// Standing and able to act.
    Alive,
    /// A zero-HP hero awaiting rescue or death saves; successes do not heal.
    Dying {
        /// Failed death saves since the most recent downing.
        failures: u8,
    },
    /// Permanently dead but still occupies its original footprint.
    Corpse {
        /// Independently damageable remains.
        hp: u16,
        /// Initial corpse durability, one quarter of living maximum HP (ceil).
        max_hp: u16,
        /// Creation round; this round does not consume lifetime.
        created_round: u32,
    },
    /// Dead remains destroyed or expired; identity is retained for event sources.
    Removed,
}
