//! Game-local narration order and displayed health, not a reusable combat engine.

use super::{CarterfightIntent, CarterfightPhase, CarterfightView, MoveView};
use crate::backend::{
    character_template, move_def, resolve_turn, Action, BattleEvent, BattlePhase, BattleState,
    MoveId, Side,
};
use bevy::prelude::*;
use std::collections::VecDeque;

const SEED: u64 = 0xCA47_E12F_1234_5678;

struct QueuedEvent {
    event: BattleEvent,
    automatic: bool,
}
struct Line {
    text: String,
    shown: usize,
    elapsed: f32,
    automatic: bool,
}

impl Line {
    fn done(&self) -> bool {
        self.shown >= self.text.chars().count()
    }
    fn reveal(&mut self) {
        self.shown = self.text.chars().count();
    }
    fn tick(&mut self, delta: f32, reduced: bool) -> bool {
        if self.done() {
            return false;
        }
        if reduced {
            self.reveal();
            return false;
        }
        self.elapsed += delta.max(0.0);
        let before = self.shown;
        while self.elapsed >= 0.05 && !self.done() {
            self.elapsed -= 0.05;
            self.shown += 1;
        }
        self.shown > before
    }
}

#[derive(Resource)]
pub(super) struct Runtime {
    pub(super) battle: BattleState,
    pub(super) phase: CarterfightPhase,
    pub(super) pending: Option<MoveId>,
    pub(super) displayed_player: u16,
    pub(super) displayed_opponent: u16,
    pub(super) sound: bool,
    queue: VecDeque<QueuedEvent>,
    line: Option<Line>,
}

impl Default for Runtime {
    fn default() -> Self {
        let player = character_template("Player").expect("authored Player template");
        let opponent = character_template("Carter").expect("authored Carter template");
        let mut result = Self {
            displayed_player: player.current_hp,
            displayed_opponent: opponent.current_hp,
            battle: BattleState::new(player, opponent, SEED),
            phase: CarterfightPhase::Intro,
            pending: None,
            sound: true,
            queue: VecDeque::new(),
            line: None,
        };
        result.push_line("A wild CARTER appeared!", false);
        result.push_line("What will you do?", true);
        result
    }
}

impl Runtime {
    fn push_line(&mut self, text: impl Into<String>, automatic: bool) {
        self.queue.push_back(QueuedEvent {
            event: BattleEvent::Dialogue(text.into()),
            automatic,
        });
    }
    pub(super) fn idle(&self) -> bool {
        self.line.is_none() && self.queue.is_empty()
    }
    pub(super) fn can_select(&self) -> bool {
        self.phase == CarterfightPhase::Battle
            && self.idle()
            && !matches!(self.battle.phase, BattlePhase::Ended { .. })
    }
    /// Exactly one supplied intent is handled; a reveal never also advances.
    pub(super) fn apply(&mut self, intent: &CarterfightIntent) -> bool {
        match intent {
            CarterfightIntent::SelectMove(id)
                if self.can_select() && self.battle.player.moves.contains(id) =>
            {
                self.pending = Some(*id)
            }
            CarterfightIntent::CancelSelection => self.pending = None,
            CarterfightIntent::ConfirmMove if self.can_select() => {
                let Some(player_move) = self.pending.take() else {
                    return false;
                };
                let Some(opponent_move) = self.battle.opponent.moves.first().copied() else {
                    return false;
                };
                let events = resolve_turn(
                    &mut self.battle,
                    Action::UseMove(player_move),
                    Action::UseMove(opponent_move),
                );
                for event in events {
                    self.queue.push_back(QueuedEvent {
                        event,
                        automatic: false,
                    });
                }
                if matches!(self.battle.phase, BattlePhase::Animating) {
                    self.battle.phase = BattlePhase::WaitingForPlayerAction;
                    self.push_line("What will you do?", true);
                }
            }
            CarterfightIntent::Advance => {
                if let Some(line) = &mut self.line {
                    if !line.done() {
                        line.reveal();
                    } else if !line.automatic {
                        self.line = None;
                    }
                } else if self.idle() && self.phase == CarterfightPhase::Outro {
                    return true;
                }
            }
            CarterfightIntent::ToggleSound => self.sound = !self.sound,
            _ => {}
        }
        false
    }
    /// A queued visual effect happens only when its narration starts.
    pub(super) fn tick(&mut self, delta: f32, reduced: bool) -> bool {
        if self
            .line
            .as_ref()
            .is_some_and(|line| line.automatic && line.done())
        {
            self.line = None;
        }
        if self.line.is_none() {
            if let Some(queued) = self.queue.pop_front() {
                if let BattleEvent::Damage {
                    target, hp_after, ..
                } = &queued.event
                {
                    match target {
                        Side::Player => self.displayed_player = *hp_after,
                        Side::Opponent => self.displayed_opponent = *hp_after,
                    }
                }
                self.line = Some(Line {
                    text: queued.event.dialogue_text(&self.battle),
                    shown: 0,
                    elapsed: 0.0,
                    automatic: queued.automatic,
                });
            } else {
                match self.phase {
                    CarterfightPhase::Intro => self.phase = CarterfightPhase::Battle,
                    CarterfightPhase::Battle => {
                        if let BattlePhase::Ended { winner } = self.battle.phase {
                            self.phase = CarterfightPhase::Outro;
                            self.push_line(
                                if winner == Side::Player {
                                    "You beat Carter!"
                                } else {
                                    "Carter wins the fight..."
                                },
                                false,
                            );
                            self.push_line("Press SPACE to close.", true);
                        }
                    }
                    CarterfightPhase::Outro => {}
                }
            }
        }
        self.line
            .as_mut()
            .is_some_and(|line| line.tick(delta, reduced))
            && self.sound
    }
    pub(super) fn view(&self) -> CarterfightView {
        let ready = self.can_select();
        let (narration, typing, continue_label) = if let Some(line) = &self.line {
            (
                line.text.chars().take(line.shown).collect(),
                !line.done(),
                if line.done() { "Continue" } else { "Show line" },
            )
        } else if self.phase == CarterfightPhase::Outro && self.idle() {
            (
                "The fight is over. Thanks for playing!".to_owned(),
                false,
                "Close game",
            )
        } else {
            ("What will you do?".to_owned(), false, "Continue")
        };
        let selected_description = self.pending.and_then(move_def).map_or_else(
            || {
                if ready {
                    "Choose a move, then confirm. Nothing happens until you commit.".to_owned()
                } else {
                    "Read the narration before choosing your next move.".to_owned()
                }
            },
            |definition| format!("{}: {}", definition.name, definition.description),
        );
        CarterfightView {
            phase: self.phase,
            player_hp: self.displayed_player,
            opponent_hp: self.displayed_opponent,
            player_max: self.battle.player.max_hp,
            opponent_max: self.battle.opponent.max_hp,
            turn: self.battle.turn_count,
            moves: self
                .battle
                .player
                .moves
                .iter()
                .filter_map(|id| move_def(id))
                .map(|definition| MoveView {
                    id: definition.id,
                    name: definition.name,
                    selected: self.pending == Some(definition.id),
                })
                .collect(),
            selected_description,
            can_select: ready,
            can_confirm: ready && self.pending.is_some(),
            can_advance: self.line.is_some()
                || (self.idle() && self.phase == CarterfightPhase::Outro),
            narration,
            typing,
            continue_label,
            sound: self.sound,
        }
    }
}
