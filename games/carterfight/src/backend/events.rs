use super::moves::{AbilityId, MoveId};
use super::state::{BattleState, Side};

/// One thing that happened during a turn. The frontend drains a `Vec<BattleEvent>`
/// into its dialogue queue / HUD updates. Every variant carries enough
/// structured data for a UI to animate it, *and* a uniform text view via
/// `dialogue_text()` for the dialogue box.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BattleEvent {
    /// A combatant begins its selected move.
    UseMove {
        /// Acting combatant.
        side: Side,
        /// Stable game-owned move identifier.
        move_id: MoveId,
    },
    /// Actual applied damage and the resulting health, in presentation order.
    Damage {
        /// Damaged combatant.
        target: Side,
        /// Applied damage, capped by the previous health.
        amount: u16,
        /// Health after this event, before any subsequent event.
        hp_after: u16,
    },
    /// Free-form narration emitted by a move or ability.
    Dialogue(String),
    /// Authored ability narration.
    AbilityTriggered {
        /// Combatant whose ability triggered.
        side: Side,
        /// Game-owned ability identifier.
        ability: AbilityId,
        /// Authored narration.
        message: String,
    },
    /// A combatant reached zero health.
    Fainted {
        /// Defeated combatant.
        side: Side,
    },
    /// Terminal outcome after all applicable actions.
    BattleEnded {
        /// Victorious combatant.
        winner: Side,
    },
}

impl BattleEvent {
    /// Canonical one-line phrasing for the dialogue box. Pulls character
    /// names from the battle state so narration reads "Carter used jab!"
    /// rather than "Opponent used jab!". The dialogue box can always just
    /// render this string; it doesn't have to know which variant it's
    /// looking at unless it wants to do something fancier.
    pub fn dialogue_text(&self, state: &BattleState) -> String {
        let name = |side: Side| state.character(side).name.as_str();
        match self {
            BattleEvent::UseMove { side, move_id } => {
                format!("{} used {}!", name(*side), move_id)
            }
            BattleEvent::Damage { target, amount, .. } => {
                format!("{} took {} damage.", name(*target), amount)
            }
            BattleEvent::Dialogue(line) => line.clone(),
            BattleEvent::AbilityTriggered {
                side,
                ability,
                message,
            } => {
                format!("{}'s {}: {}", name(*side), ability, message)
            }
            BattleEvent::Fainted { side } => {
                format!("{} fainted!", name(*side))
            }
            BattleEvent::BattleEnded { winner } => {
                format!("{} wins the battle!", name(*winner))
            }
        }
    }
}
