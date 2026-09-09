use super::action::Action;
use super::data;
use super::events::BattleEvent;
use super::moves::MoveEffect;
use super::state::{BattlePhase, BattleState, Side};

/// Run one full turn. Pure modulo the `&mut state` write: deterministic given
/// the current state, both actions, and the RNG seed. Returns events in
/// chronological order.
///
/// For v1 the action order is fixed: player goes first. When mechanics need
/// speed/priority, sort here.
pub fn resolve_turn(
    state: &mut BattleState,
    player_action: Action,
    opponent_action: Action,
) -> Vec<BattleEvent> {
    let mut events = Vec::new();
    state.phase = BattlePhase::Resolving;

    execute_action(state, Side::Player, &player_action, &mut events);

    if state.character(Side::Opponent).is_alive() {
        execute_action(state, Side::Opponent, &opponent_action, &mut events);
    }

    state.turn_count += 1;

    if let Some(winner) = check_battle_end(state) {
        events.push(BattleEvent::BattleEnded { winner });
        state.phase = BattlePhase::Ended { winner };
    } else {
        state.phase = BattlePhase::Animating;
    }

    events
}

fn execute_action(
    state: &mut BattleState,
    side: Side,
    action: &Action,
    events: &mut Vec<BattleEvent>,
) {
    if !state.character(side).is_alive() {
        return;
    }

    match action {
        Action::UseMove(move_id) => {
            let Some(move_def) = data::move_def(move_id) else {
                // Unknown move id — emit nothing, skip silently. The frontend
                // shouldn't be able to construct one of these in normal play.
                return;
            };
            events.push(BattleEvent::UseMove {
                side,
                move_id: move_def.id,
            });
            apply_effect(state, side, &move_def.effect, events);
        }
    }
}

fn apply_effect(
    state: &mut BattleState,
    attacker: Side,
    effect: &MoveEffect,
    events: &mut Vec<BattleEvent>,
) {
    match effect {
        MoveEffect::Damage { amount } => {
            let target_side = attacker.opposite();
            let target = state.character_mut(target_side);
            let applied = (*amount).min(target.current_hp);
            target.current_hp = target.current_hp.saturating_sub(applied);
            let hp_after = target.current_hp;
            events.push(BattleEvent::Damage {
                target: target_side,
                amount: applied,
                hp_after,
            });
            if hp_after == 0 {
                events.push(BattleEvent::Fainted { side: target_side });
            }
        }
    }
}

fn check_battle_end(state: &BattleState) -> Option<Side> {
    match (state.player.is_alive(), state.opponent.is_alive()) {
        (false, _) => Some(Side::Opponent),
        (_, false) => Some(Side::Player),
        _ => None,
    }
}
