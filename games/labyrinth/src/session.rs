//! Four-player policy and replay protection. None of these rules belong to transport.

use std::collections::{BTreeMap, VecDeque};

use bevy::prelude::*;
use bevy_game_session::PeerId;
use labyrinth_rules::{ActorId, Combat, CombatAction, CombatEvent, CombatSnapshot, HeroClass};
use serde::{Deserialize, Serialize};

use crate::view::{PlayerView, PresentedEvent};

#[cfg(test)]
mod tests;

const RESULT_CACHE: usize = 64;
const LOG_LIMIT: usize = 80;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) struct PlayerState {
    pub slot: u8,
    pub hero: HeroClass,
    pub peer: Option<PeerId>,
    pub occupied: bool,
    pub connected: bool,
    pub ready: bool,
}

impl PlayerState {
    pub fn view(&self) -> PlayerView {
        PlayerView {
            slot: self.slot,
            hero: self.hero,
            name: if self.slot == 0 {
                "Host".into()
            } else {
                format!("Player {}", self.slot + 1)
            },
            occupied: self.occupied,
            connected: self.connected,
            ready: self.ready,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) enum SessionCommand {
    ChooseHero(HeroClass),
    Ready(bool),
    Start,
    Rematch,
    Act {
        actor: ActorId,
        action: CombatAction,
    },
}

#[derive(Message, Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) struct GameRequest {
    pub sequence: u64,
    pub encounter: u64,
    pub decision: u64,
    pub command: SessionCommand,
}

#[derive(Message, Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) struct RequestResult {
    pub sequence: u64,
    pub rejection: Option<String>,
}

#[derive(Message, Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) struct SessionSnapshot {
    pub revision: u64,
    pub encounter: u64,
    pub next_sequence: u64,
    pub players: Vec<PlayerState>,
    pub combat: Option<CombatSnapshot>,
    pub log: Vec<String>,
    pub events: Vec<PresentedEvent>,
    pub paused: bool,
}

#[derive(Resource)]
pub(crate) struct PartyAuthority {
    players: Vec<PlayerState>,
    combat: Option<Combat>,
    sequence: BTreeMap<u8, u64>,
    results: BTreeMap<u8, VecDeque<RequestResult>>,
    revision: u64,
    encounter: u64,
    seed: u64,
    local: bool,
    log: VecDeque<String>,
    events: VecDeque<PresentedEvent>,
    next_event: u64,
    faulted: bool,
}

impl PartyAuthority {
    pub fn new(seed: u64, local: bool) -> Self {
        let players = HeroClass::ALL
            .into_iter()
            .enumerate()
            .map(|(index, hero)| PlayerState {
                slot: u8::try_from(index).unwrap_or_default(),
                hero,
                peer: None,
                occupied: local || index == 0,
                connected: local || index == 0,
                ready: local,
            })
            .collect();
        Self {
            players,
            combat: None,
            sequence: BTreeMap::new(),
            results: BTreeMap::new(),
            revision: 1,
            encounter: 0,
            seed,
            local,
            log: VecDeque::new(),
            events: VecDeque::new(),
            next_event: 1,
            faulted: false,
        }
    }

    pub fn in_lobby(&self) -> bool {
        self.combat.is_none()
    }
    pub fn occupied(&self) -> u8 {
        u8::try_from(self.players.iter().filter(|player| player.occupied).count()).unwrap_or(4)
    }
    pub fn has_space(&self) -> bool {
        self.in_lobby() && self.occupied() < 4
    }
    pub fn slot_for(&self, peer: PeerId) -> Option<u8> {
        self.players
            .iter()
            .find(|player| player.peer == Some(peer))
            .map(|player| player.slot)
    }
    pub fn reserve(&mut self, peer: PeerId) -> Result<u8, String> {
        if let Some(slot) = self.slot_for(peer) {
            return Ok(slot);
        }
        if !self.has_space() {
            return Err("Party is full or the encounter has started.".into());
        }
        let available = HeroClass::ALL
            .into_iter()
            .find(|hero| {
                !self
                    .players
                    .iter()
                    .any(|player| player.occupied && player.hero == *hero)
            })
            .ok_or("No hero is available.")?;
        let player = self
            .players
            .iter_mut()
            .find(|player| !player.occupied)
            .ok_or("Party is full.")?;
        player.peer = Some(peer);
        player.occupied = true;
        player.connected = false;
        player.ready = false;
        player.hero = available;
        let slot = player.slot;
        self.revision += 1;
        Ok(slot)
    }
    pub fn connected(&mut self, peer: PeerId, connected: bool) {
        if let Some(player) = self
            .players
            .iter_mut()
            .find(|player| player.peer == Some(peer))
        {
            player.connected = connected;
            if self.combat.is_none() {
                player.ready = false;
            }
            self.revision += 1;
        }
    }
    pub fn release(&mut self, peer: PeerId) {
        if self.combat.is_some() {
            self.connected(peer, false);
            return;
        }
        if let Some(player) = self
            .players
            .iter_mut()
            .find(|player| player.peer == Some(peer))
        {
            player.peer = None;
            player.occupied = false;
            player.connected = false;
            player.ready = false;
            // New identity never inherits an old seat's replay history.
            self.sequence.remove(&player.slot);
            self.results.remove(&player.slot);
            self.revision += 1;
        }
    }
    pub fn next_sequence(&self, slot: u8) -> u64 {
        self.sequence
            .get(&slot)
            .copied()
            .unwrap_or(0)
            .saturating_add(1)
    }
    pub fn snapshot(&self, slot: u8) -> SessionSnapshot {
        SessionSnapshot {
            revision: self.revision,
            encounter: self.encounter,
            next_sequence: self.next_sequence(slot),
            players: self.players.clone(),
            combat: self.combat.as_ref().map(Combat::snapshot),
            log: self.log.iter().cloned().collect(),
            events: self.events.iter().cloned().collect(),
            paused: self.paused(),
        }
    }
    pub fn paused(&self) -> bool {
        self.faulted
            || (self.combat.is_some() && !self.local && self.players.iter().any(|p| !p.connected))
    }
    pub fn apply(&mut self, slot: u8, request: GameRequest) -> RequestResult {
        if let Some(result) = self.results.get(&slot).and_then(|cache| {
            cache
                .iter()
                .find(|result| result.sequence == request.sequence)
        }) {
            return result.clone();
        }
        let previous = self.sequence.get(&slot).copied().unwrap_or(0);
        if request.sequence <= previous || request.sequence == u64::MAX {
            return RequestResult {
                sequence: request.sequence,
                rejection: Some("Stale request.".into()),
            };
        }
        self.sequence.insert(slot, request.sequence);
        let rejection = self.reduce(slot, request).err();
        let result = RequestResult {
            sequence: self.sequence.get(&slot).copied().unwrap_or(0),
            rejection,
        };
        let cache = self.results.entry(slot).or_default();
        cache.push_back(result.clone());
        while cache.len() > RESULT_CACHE {
            cache.pop_front();
        }
        result
    }
    fn reduce(&mut self, slot: u8, request: GameRequest) -> Result<(), String> {
        let player = self
            .players
            .iter()
            .find(|p| p.slot == slot && p.connected)
            .ok_or("Player is not admitted.")?;
        if request.encounter != self.encounter {
            return Err("That encounter has ended.".into());
        }
        match request.command {
            SessionCommand::ChooseHero(hero) => {
                if !self.in_lobby() {
                    return Err("Heroes are chosen in the lobby.".into());
                }
                if self
                    .players
                    .iter()
                    .any(|p| p.occupied && p.slot != slot && p.hero == hero)
                {
                    return Err("Another player has chosen that hero.".into());
                }
                if let Some(player) = self.players.iter_mut().find(|p| p.slot == slot) {
                    player.hero = hero;
                    player.ready = false;
                }
            }
            SessionCommand::Ready(ready) => {
                if !self.in_lobby() {
                    return Err("The encounter has already begun.".into());
                }
                if let Some(player) = self.players.iter_mut().find(|p| p.slot == slot) {
                    player.ready = ready;
                }
            }
            SessionCommand::Start => {
                if slot != 0 || !self.in_lobby() {
                    return Err("Only the host can start from the lobby.".into());
                }
                if self.players.iter().any(|p| !p.connected || !p.ready) {
                    return Err("All four players must be ready.".into());
                }
                let heroes: [HeroClass; 4] = self
                    .players
                    .iter()
                    .map(|p| p.hero)
                    .collect::<Vec<_>>()
                    .try_into()
                    .map_err(|_| "Invalid party size.")?;
                self.combat = Some(
                    Combat::new(self.seed.wrapping_add(self.encounter), heroes)
                        .map_err(|e| e.to_string())?,
                );
                self.encounter += 1;
                self.log.clear();
                self.events.clear();
                self.faulted = false;
                self.log.push_back("The party enters the Labyrinth.".into());
            }
            SessionCommand::Rematch => {
                if slot != 0 {
                    return Err("Only the host can return the party to the lobby.".into());
                }
                self.combat = None;
                self.faulted = false;
                self.encounter += 1;
                for p in &mut self.players {
                    p.ready = self.local;
                }
            }
            SessionCommand::Act { actor, action } => {
                if self.paused() {
                    return Err("Combat is paused for a disconnected player.".into());
                }
                if !self.local && actor != ActorId(u16::from(player.slot) + 1) {
                    return Err("That is not your hero.".into());
                }
                let combat = self.combat.as_mut().ok_or("No encounter is running.")?;
                if request.decision != combat.snapshot().turn_id {
                    return Err("That turn has already ended.".into());
                }
                let events = combat.apply(actor, action).map_err(|e| e.to_string())?;
                self.record(events);
            }
        }
        self.revision += 1;
        Ok(())
    }
    pub fn advance_enemy(&mut self) -> bool {
        if self.paused() {
            return false;
        }
        let Some(combat) = self.combat.as_mut() else {
            return false;
        };
        let Some(action) = combat.ai_action() else {
            return false;
        };
        let Some(actor) = combat.snapshot().active_actor else {
            return false;
        };
        match combat.apply(actor, action) {
            Ok(events) => {
                self.record(events);
                self.revision += 1;
                true
            }
            Err(error) => {
                self.log.push_back(format!(
                    "Encounter halted: {error}. Return to the lobby to restart."
                ));
                self.faulted = true;
                self.revision += 1;
                false
            }
        }
    }
    fn record(&mut self, events: impl IntoIterator<Item = CombatEvent>) {
        for event in events {
            self.log.push_back(event.to_string());
            self.events.push_back(PresentedEvent {
                id: self.next_event,
                event,
            });
            self.next_event = self.next_event.saturating_add(1);
        }
        while self.log.len() > LOG_LIMIT {
            self.log.pop_front();
        }
        while self.events.len() > LOG_LIMIT {
            self.events.pop_front();
        }
    }
}
