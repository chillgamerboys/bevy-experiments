use rand::SeedableRng;
use rand_chacha::ChaCha8Rng;

use super::moves::{AbilityId, MoveId};

/// Which side of the field a character belongs to.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Side {
    Player,
    Opponent,
}

impl Side {
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
    pub name: String,
    pub max_hp: u16,
    pub current_hp: u16,
    pub moves: Vec<MoveId>,
    pub abilities: Vec<AbilityId>,
}

impl Character {
    pub fn is_alive(&self) -> bool {
        self.current_hp > 0
    }
}

/// Internal turn-state machine, owned by `BattleState`. Frontend reads this to
/// decide whether to listen for input or wait for animations to finish.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BattlePhase {
    WaitingForPlayerAction,
    WaitingForOpponentAction,
    Resolving,
    Animating,
    Ended { winner: Side },
}

pub struct BattleState {
    pub player: Character,
    pub opponent: Character,
    pub turn_count: u32,
    pub phase: BattlePhase,
    /// Seeded so the same inputs always produce the same event stream. Private
    /// to `backend/` — tests construct via `BattleState::new`. Unused in v1's
    /// deterministic damage formula; will be consumed by future mechanics
    /// (accuracy rolls, crits, randomized effects).
    #[allow(dead_code)]
    pub(super) rng: ChaCha8Rng,
}

impl BattleState {
    pub fn new(player: Character, opponent: Character, seed: u64) -> Self {
        Self {
            player,
            opponent,
            turn_count: 0,
            phase: BattlePhase::WaitingForPlayerAction,
            rng: ChaCha8Rng::seed_from_u64(seed),
        }
    }

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
