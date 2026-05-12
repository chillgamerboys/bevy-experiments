//! Drains `BattleEventQueue` one event at a time, applying the event's visual
//! side-effects (HP bar, sprite changes, ...) and starting its dialogue line
//! together. Dialogue rendering itself still lives in
//! [`super::dialogue::tick_dialogue`] — this module just decides *when* the
//! next event is allowed to start.

use bevy::prelude::*;

use super::super::backend::{BattleEvent, Side};
use super::dialogue::{BattleEventQueue, DialogueState};
use super::resources::{BattleStateRes, DisplayedCombatants};

/// Whether the next event pops automatically once typing finishes, or whether
/// the player has to press Space first. Stored with each queued event so the
/// pusher decides (narration waits; prompts auto-advance).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AdvanceMode {
    WaitForInput,
    AutoAfterTypewriter,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SequencerPhase {
    Idle,
    Presenting { advance: AdvanceMode },
}

#[derive(Resource)]
pub struct Sequencer {
    pub phase: SequencerPhase,
}

impl Default for Sequencer {
    fn default() -> Self {
        Self { phase: SequencerPhase::Idle }
    }
}

impl Sequencer {
    pub fn is_idle(&self) -> bool {
        matches!(self.phase, SequencerPhase::Idle)
    }
}

/// Apply the *visual* side-effect of an event. The authoritative `BattleState`
/// has already mutated in `resolve_turn`; this just brings the displayed
/// mirror into agreement, so the HUD updates in lockstep with the dialogue
/// line for this event.
fn apply_event_visuals(
    event: &BattleEvent,
    displayed: &mut DisplayedCombatants,
) {
    match event {
        BattleEvent::Damage { target, hp_after, .. } => match target {
            Side::Player => displayed.player_hp = *hp_after,
            Side::Opponent => displayed.opponent_hp = *hp_after,
        },
        // No visual effect beyond the dialogue line itself. Add new arms here
        // when future event variants need sprite/animation hooks.
        BattleEvent::UseMove { .. }
        | BattleEvent::Dialogue(_)
        | BattleEvent::AbilityTriggered { .. }
        | BattleEvent::Fainted { .. }
        | BattleEvent::BattleEnded { .. } => {}
    }
}

/// Runs every frame, just before `tick_dialogue`. Either pops the next event
/// off the queue (when idle) or watches the typewriter for an auto-advance
/// event to finish. `WaitForInput` transitions live in
/// [`super::dialogue::dialogue_input`].
pub fn advance_sequencer(
    mut sequencer: ResMut<Sequencer>,
    mut queue: ResMut<BattleEventQueue>,
    mut dialogue: ResMut<DialogueState>,
    state: Res<BattleStateRes>,
    mut displayed: ResMut<DisplayedCombatants>,
) {
    match sequencer.phase {
        SequencerPhase::Idle => {
            let Some(queued) = queue.0.pop_front() else {
                return;
            };
            apply_event_visuals(&queued.event, &mut displayed);
            dialogue.start(queued.event.dialogue_text(&state.0));
            sequencer.phase = SequencerPhase::Presenting { advance: queued.advance };
        }
        SequencerPhase::Presenting { advance: AdvanceMode::AutoAfterTypewriter } => {
            if dialogue.is_done {
                sequencer.phase = SequencerPhase::Idle;
            }
        }
        SequencerPhase::Presenting { advance: AdvanceMode::WaitForInput } => {
            // `dialogue_input` flips this back to `Idle` on Space.
        }
    }
}
