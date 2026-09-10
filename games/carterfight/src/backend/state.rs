use rand::SeedableRng;
use rand_chacha::ChaCha8Rng;

use super::moves::{AbilityId, MoveId};

/// Which side of the field a character belongs to.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Side {
    /// Human-controlled combatant.
    Player,
    /// Carter, controlled by the local opponent policy.
    Opponent,
}

impl Side {
    /// Returns the other combatant.
    pub fn opposite(self) -> Self {
        match self {
            Side::Player => Side::Opponent,
            Side::Opponent => Side::Player,
        }
    }
}

/// A combatant. Owns its current HP, moveset, and ability list.
#[derive(Debug, Clone)]
pub struct Character {
    /// Authored display name.
    pub name: String,
    /// Initial and maximum health.
    pub max_hp: u16,
    /// Authoritative remaining health.
    pub current_hp: u16,
    /// Ordered available move identifiers.
    pub moves: Vec<MoveId>,
    /// Game-owned ability identifiers.
    pub abilities: Vec<AbilityId>,
}

impl Character {
    /// Whether this combatant can still act.
    pub fn is_alive(&self) -> bool {
        self.current_hp > 0
    }
}

/// Internal turn-state machine, owned by `BattleState`. Frontend reads this to
/// decide whether to listen for input or wait for animations to finish.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BattlePhase {
    /// No turn is being resolved.
    WaitingForPlayerAction,
    /// The human has selected an action.
    WaitingForOpponentAction,
    /// A turn is currently being resolved.
    Resolving,
    /// Resolution completed; presentation may consume its returned events.
    Animating,
    /// Terminal battle result.
    Ended {
        /// Victorious combatant.
        winner: Side,
    },
}

/// Authoritative local battle state. No UI or Bevy ownership is required.
pub struct BattleState {
    /// Human-controlled combatant.
    pub player: Character,
    /// Locally controlled opponent.
    pub opponent: Character,
    /// Number of completed turns.
    pub turn_count: u32,
    /// Current rules phase.
    pub phase: BattlePhase,
    /// Seeded so the same inputs always produce the same event stream. Private
    /// to `backend/` — tests construct via `BattleState::new`. Unused in v1's
    /// deterministic damage formula; will be consumed by future mechanics
    /// (accuracy rolls, crits, randomized effects).
    pub(super) _rng: ChaCha8Rng,
}

impl BattleState {
    /// Creates a fresh deterministic battle from explicit combatants and seed.
    pub fn new(player: Character, opponent: Character, seed: u64) -> Self {
        Self {
            player,
            opponent,
            turn_count: 0,
            phase: BattlePhase::WaitingForPlayerAction,
            _rng: ChaCha8Rng::seed_from_u64(seed),
        }
    }

    /// Reads the indicated combatant.
    pub fn character(&self, side: Side) -> &Character {
        match side {
            Side::Player => &self.player,
            Side::Opponent => &self.opponent,
        }
    }

    pub(super) fn character_mut(&mut self, side: Side) -> &mut Character {
        match side {
            Side::Player => &mut self.player,
            Side::Opponent => &mut self.opponent,
        }
    }
}
