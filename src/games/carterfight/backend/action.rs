use super::moves::MoveId;

/// What a side chose to do this turn. Frontend constructs one of these per
/// side and hands them to `resolve_turn`.
#[derive(Debug, Clone)]
pub enum Action {
    UseMove(MoveId),
}
