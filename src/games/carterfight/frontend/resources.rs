use std::collections::VecDeque;

use bevy::prelude::*;

use super::super::backend::{BattleEvent, BattleState};

/// Bevy-side wrapper around the pure-Rust `BattleState`. Systems hold this as
/// a `ResMut`. All gameplay logic stays inside `backend::*`; this is just the
/// handle Bevy needs to track it as a resource.
#[derive(Resource)]
pub struct BattleStateRes(pub BattleState);

/// One thing the dialogue box should display next. The colleague's renderer
/// either calls `.display_text()` for the uniform string view, or pattern-
/// matches on the inner `BattleEvent` to layer in HP-bar animations / screen
/// effects / etc. Either path keeps the backend stable.
#[derive(Debug, Clone)]
pub enum DialogueEntry {
    /// Authored narration (intro/outro scripts).
    Text(String),
    /// A backend event to render. The renderer can call `event.dialogue_text()`
    /// for the canonical line, or look at the structured fields for richer UI.
    Event(BattleEvent),
}

impl DialogueEntry {
    /// Uniform fallback: gives a single string for any entry. The simplest
    /// possible dialogue-box implementation just calls this.
    pub fn display_text(&self) -> String {
        match self {
            DialogueEntry::Text(t) => t.clone(),
            DialogueEntry::Event(e) => e.dialogue_text(),
        }
    }
}

/// FIFO of dialogue entries waiting to be shown. The frontend pushes onto the
/// back; the dialogue-box system reads/pops from the front.
#[derive(Resource, Default)]
pub struct DialogueQueue {
    pub items: VecDeque<DialogueEntry>,
}

impl DialogueQueue {
    pub fn push_text(&mut self, s: impl Into<String>) {
        self.items.push_back(DialogueEntry::Text(s.into()));
    }
    pub fn push_event(&mut self, e: BattleEvent) {
        self.items.push_back(DialogueEntry::Event(e));
    }
    pub fn pop(&mut self) -> Option<DialogueEntry> {
        self.items.pop_front()
    }
    pub fn peek(&self) -> Option<&DialogueEntry> {
        self.items.front()
    }
    pub fn is_empty(&self) -> bool {
        self.items.is_empty()
    }
}
