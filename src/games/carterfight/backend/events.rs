use super::moves::{AbilityId, MoveId};
use super::state::Side;

/// One thing that happened during a turn. The frontend drains a `Vec<BattleEvent>`
/// into its dialogue queue / HUD updates. Every variant carries enough
/// structured data for a UI to animate it, *and* a uniform text view via
/// `dialogue_text()` for the dialogue box.
#[derive(Debug, Clone)]
pub enum BattleEvent {
    UseMove {
        side: Side,
        move_id: MoveId,
    },
    Damage {
        target: Side,
        amount: u16,
        hp_after: u16,
    },
    /// Free-form narration emitted by a move or ability.
    Dialogue(String),
    AbilityTriggered {
        side: Side,
        ability: AbilityId,
        message: String,
    },
    Fainted {
        side: Side,
    },
    BattleEnded {
        winner: Side,
    },
}

impl BattleEvent {
    /// Canonical one-line phrasing for the dialogue box. The dialogue box can
    /// always just render this string; it doesn't have to know which variant
    /// it's looking at unless it wants to do something fancier (HP-bar
    /// animation tied to `Damage`, screen flash on `Fainted`, etc.).
    pub fn dialogue_text(&self) -> String {
        match self {
            BattleEvent::UseMove { side, move_id } => {
                format!("{} used {}!", side_name(*side), move_id)
            }
            BattleEvent::Damage { target, amount, .. } => {
                format!("{} took {} damage.", side_name(*target), amount)
            }
            BattleEvent::Dialogue(line) => line.clone(),
            BattleEvent::AbilityTriggered { side, ability, message } => {
                format!("{}'s {}: {}", side_name(*side), ability, message)
            }
            BattleEvent::Fainted { side } => {
                format!("{} fainted!", side_name(*side))
            }
            BattleEvent::BattleEnded { winner } => {
                format!("{} wins the battle!", side_name(*winner))
            }
        }
    }
}

fn side_name(side: Side) -> &'static str {
    match side {
        Side::Player => "Player",
        Side::Opponent => "Opponent",
    }
}
