//! Six-player policy and replay protection. None of these rules belong to transport.

use std::collections::{BTreeMap, BTreeSet, VecDeque};

use bevy::prelude::*;
use bevy_gamekit::session::PeerId;
use labyrinth_rules::{
    AbilityLoadout, ActorId, ActorKind, Combat, CombatAction, CombatEvent, CombatSnapshot,
    HeroClass, HeroSetup, DEFAULT_HERO_ROSTER, PARTY_SIZE,
};
use serde::{Deserialize, Serialize};

use crate::view::{CombatInterruption, PlayerView, PresentedEvent};

#[cfg(test)]
mod tests;

const RESULT_CACHE: usize = 64;
const LOG_LIMIT: usize = 80;
pub(crate) const PLAYER_CAPACITY: u8 = PARTY_SIZE as u8;
const ACTORS: [ActorId; PARTY_SIZE] = [
    ActorId(1),
    ActorId(2),
    ActorId(3),
    ActorId(4),
    ActorId(5),
    ActorId(6),
];

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) struct PlayerState {
    pub slot: u8,
    pub actor: ActorId,
    pub hero: HeroClass,
    pub abilities: AbilityLoadout,
    pub peer: Option<PeerId>,
    pub occupied: bool,
    pub connected: bool,
    pub ready: bool,
}

impl PlayerState {
    pub fn view(&self) -> PlayerView {
        PlayerView {
            slot: self.slot,
            actor: self.actor,
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
#[serde(try_from = "UncheckedSessionSnapshot")]
pub(crate) struct SessionSnapshot {
    pub revision: u64,
    pub encounter: u64,
    pub next_sequence: u64,
    pub players: Vec<PlayerState>,
    pub combat: Option<CombatSnapshot>,
    pub log: Vec<String>,
    pub events: Vec<PresentedEvent>,
    pub paused: bool,
    pub interruption: CombatInterruption,
}

#[derive(Deserialize)]
struct UncheckedSessionSnapshot {
    revision: u64,
    encounter: u64,
    next_sequence: u64,
    players: Vec<PlayerState>,
    combat: Option<CombatSnapshot>,
    log: Vec<String>,
    events: Vec<PresentedEvent>,
    paused: bool,
    interruption: CombatInterruption,
}

impl TryFrom<UncheckedSessionSnapshot> for SessionSnapshot {
    type Error = &'static str;

    fn try_from(value: UncheckedSessionSnapshot) -> Result<Self, Self::Error> {
        let snapshot = Self {
            revision: value.revision,
            encounter: value.encounter,
            next_sequence: value.next_sequence,
            players: value.players,
            combat: value.combat,
            log: value.log,
            events: value.events,
            paused: value.paused,
            interruption: value.interruption,
        };
        snapshot.validate()?;
        Ok(snapshot)
    }
}

impl SessionSnapshot {
    /// Validate owner identity separately from class and mutable formation rank.
    pub fn validate(&self) -> Result<(), &'static str> {
        if self.interruption == CombatInterruption::Reconnecting
            || self.paused != (self.interruption != CombatInterruption::None)
        {
            return Err("Invalid host suspension reason.");
        }
        let disconnected = self.combat.is_some() && self.players.iter().any(|p| !p.connected);
        if (self.interruption == CombatInterruption::WaitingForPlayers && !disconnected)
            || (self.combat.is_none() && self.interruption != CombatInterruption::None)
        {
            return Err("Suspension reason does not match the encounter.");
        }
        if self.revision == 0
            || self.next_sequence == 0
            || self.players.len() != PARTY_SIZE
            || self.log.len() > LOG_LIMIT
            || self.events.len() > LOG_LIMIT
        {
            return Err("Invalid session snapshot bounds.");
        }
        let mut slots = BTreeSet::new();
        let mut actors = BTreeSet::new();
        let mut peers = BTreeSet::new();
        for player in &self.players {
            if player.slot >= PLAYER_CAPACITY
                || !slots.insert(player.slot)
                || player.actor.0 == 0
                || !actors.insert(player.actor)
                || (!player.occupied && (player.connected || player.ready || player.peer.is_some()))
                || (player.slot == 0
                    && (!player.occupied || !player.connected || player.peer.is_some()))
                || player
                    .peer
                    .is_some_and(|peer| !peer.is_valid() || !peers.insert(peer))
            {
                return Err("Invalid or duplicate player ownership.");
            }
        }
        if let Some(combat) = &self.combat {
            combat.validate().map_err(|_| "Invalid combat snapshot.")?;
            if self.encounter == 0 || self.players.iter().any(|player| !player.occupied) {
                return Err("Combat requires the complete reserved party.");
            }
            for player in &self.players {
                if !combat.actor(player.actor).is_some_and(|actor| {
                    actor.kind == ActorKind::Hero(player.hero)
                        && actor.abilities == player.abilities
                }) {
                    return Err("Combat actor does not match its owner and loadout.");
                }
            }
            if !self.paused && self.players.iter().any(|player| !player.connected) {
                return Err("Disconnected combat must be paused.");
            }
        }
        if self.events.iter().any(|event| event.id == 0)
            || self.events.windows(2).any(|pair| {
                pair.first()
                    .zip(pair.get(1))
                    .is_some_and(|(a, b)| a.id >= b.id)
            })
        {
            return Err("Invalid session event order.");
        }
        Ok(())
    }

    pub fn validate_recipient(&self, slot: u8, peer: PeerId) -> Result<(), &'static str> {
        self.validate()?;
        if self
            .players
            .iter()
            .any(|player| player.slot != 0 && player.occupied && player.peer.is_none())
            || !self.players.iter().any(|player| {
                player.slot == slot
                    && player.peer == Some(peer)
                    && player.occupied
                    && player.connected
            })
        {
            return Err("Snapshot does not belong to the admitted player.");
        }
        Ok(())
    }

    pub fn validate_successor(&self, previous: &Self) -> Result<(), &'static str> {
        for player in &self.players {
            let prior = previous
                .players
                .iter()
                .find(|prior| prior.slot == player.slot)
                .ok_or("Player slot changed within the session.")?;
            if prior.actor != player.actor {
                return Err("Actor ownership changed within the session.");
            }
            if self.encounter == previous.encounter
                && previous.combat.is_some()
                && (prior.peer != player.peer
                    || prior.hero != player.hero
                    || prior.abilities != player.abilities)
            {
                return Err("Combat ownership or loadout changed during the encounter.");
            }
        }
        Ok(())
    }
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
        let players = ACTORS
            .into_iter()
            .zip(DEFAULT_HERO_ROSTER)
            .enumerate()
            .map(|(index, (actor, hero))| PlayerState {
                slot: u8::try_from(index).unwrap_or_default(),
                actor,
                hero,
                abilities: HeroSetup::preset(actor, hero).abilities,
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
        u8::try_from(self.players.iter().filter(|player| player.occupied).count())
            .unwrap_or(PLAYER_CAPACITY)
    }
    pub fn has_space(&self) -> bool {
        self.in_lobby() && self.occupied() < PLAYER_CAPACITY
    }
    pub fn slot_for(&self, peer: PeerId) -> Option<u8> {
        self.players
            .iter()
            .find(|player| player.peer == Some(peer))
            .map(|player| player.slot)
    }
    pub fn reserve(&mut self, peer: PeerId) -> Result<u8, String> {
        if !peer.is_valid() {
            return Err("Invalid player identity.".into());
        }
        if let Some(slot) = self.slot_for(peer) {
            return Ok(slot);
        }
        if !self.has_space() {
            return Err("Party is full or the encounter has started.".into());
        }
        let player = self
            .players
            .iter_mut()
            .find(|player| !player.occupied)
            .ok_or("Party is full.")?;
        player.peer = Some(peer);
        player.occupied = true;
        player.connected = false;
        player.ready = false;
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
            if let Some(hero) = DEFAULT_HERO_ROSTER.get(usize::from(player.slot)) {
                player.hero = *hero;
                player.abilities = HeroSetup::preset(player.actor, *hero).abilities;
            }
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
            interruption: self.interruption(),
        }
    }
    pub fn paused(&self) -> bool {
        self.interruption() != CombatInterruption::None
    }
    fn interruption(&self) -> CombatInterruption {
        if self.faulted {
            CombatInterruption::Halted
        } else if self.combat.is_some() && !self.local && self.players.iter().any(|p| !p.connected)
        {
            CombatInterruption::WaitingForPlayers
        } else {
            CombatInterruption::None
        }
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
                let changed = player.hero != hero;
                if let Some(player) = self.players.iter_mut().find(|p| p.slot == slot) {
                    player.hero = hero;
                    if changed {
                        player.abilities = HeroSetup::preset(player.actor, hero).abilities;
                    }
                }
                if changed {
                    for player in &mut self.players {
                        player.ready = false;
                    }
                }
            }
            SessionCommand::Ready(ready) => {
                if !self.in_lobby() {
                    return Err("The encounter has already begun.".into());
                }
                for player in &mut self.players {
                    if self.local || player.slot == slot {
                        player.ready = ready;
                    }
                }
            }
            SessionCommand::Start => {
                if slot != 0 || !self.in_lobby() {
                    return Err("Only the host can start from the lobby.".into());
                }
                if self.players.iter().any(|p| !p.connected || !p.ready) {
                    return Err("All six players must be connected and ready.".into());
                }
                let heroes: [HeroSetup; PARTY_SIZE] = self
                    .players
                    .iter()
                    .map(|p| HeroSetup {
                        id: p.actor,
                        class: p.hero,
                        abilities: p.abilities.clone(),
                    })
                    .collect::<Vec<_>>()
                    .try_into()
                    .map_err(|_| "Invalid party size.")?;
                self.combat = Some(
                    Combat::with_heroes(self.seed.wrapping_add(self.encounter), heroes)
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
                    return Err(if self.faulted {
                        "Encounter halted; return to the lobby."
                    } else {
                        "Combat is waiting for disconnected players."
                    }
                    .into());
                }
                if !self.local && actor != player.actor {
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
