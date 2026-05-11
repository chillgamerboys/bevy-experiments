/// Interned identifier for a move. Trivially cheap to copy and compare;
/// upgrade to a numeric handle if the move count ever explodes.
pub type MoveId = &'static str;

/// Interned identifier for an ability. v1 doesn't dispatch on these — they're
/// just tagged on a `Character` for the resolver to inspect later when the
/// real ability system gets designed.
pub type AbilityId = &'static str;

#[derive(Debug, Clone)]
pub struct MoveDef {
    pub id: MoveId,
    pub name: &'static str,
    pub description: &'static str,
    pub effect: MoveEffect,
}

/// What a move does when it lands. New mechanics get added as variants — the
/// resolver's match arms then handle them. The frontend never matches on this.
#[derive(Debug, Clone)]
pub enum MoveEffect {
    Damage { amount: u16 },
    // Future: Heal { amount }, ApplyStatus { kind }, ModifyStat { ... }, MultiHit { ... }, etc.
}
