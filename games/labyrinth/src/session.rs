//! Six-player policy and replay protection. None of these rules belong to transport.

use std::collections::{BTreeMap, BTreeSet, VecDeque};

use bevy::prelude::*;
use bevy_gamekit::session::PeerId;
use labyrinth_rules::{
    AbilityLoadout, ActorId, ActorKind, Combat, CombatAction, CombatEvent, CombatSnapshot,
    HeroClass, HeroSetup, PARTY_SIZE,
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
    pub peer: Option<PeerId>,
    pub occupied: bool,
    pub connected: bool,
    pub ready: bool,
}

impl PlayerState {
    pub fn view(&self) -> PlayerView {
        PlayerView {
            slot: self.slot,
            actors: Vec::new(),
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

/// A character is independent of participant admission and formation position.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CompanyMember {
    /// Stable character identity.
    pub actor: ActorId,
    /// Current visual and build preset.
    pub hero: HeroClass,
    /// Frozen active move selection for this character.
    pub abilities: AbilityLoadout,
    /// Participant controller, independent of formation rank.
    pub owner: u8,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) enum SessionCommand {
    ChooseHero {
        actor: ActorId,
        hero: HeroClass,
    },
    Assign {
        actor: ActorId,
        owner: u8,
    },
    AssignmentPause(bool),
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
    pub assignment_revision: u64,
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
    pub company: Vec<CompanyMember>,
    pub assignment_revision: u64,
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
    company: Vec<CompanyMember>,
    assignment_revision: u64,
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
            company: value.company,
            assignment_revision: value.assignment_revision,
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
    /// Validate participants separately from characters and mutable formation ranks.
    pub fn validate(&self) -> Result<(), &'static str> {
        if self.interruption == CombatInterruption::Reconnecting
            || self.paused != (self.interruption != CombatInterruption::None)
            || (self.combat.is_none() && self.interruption != CombatInterruption::None)
        {
            return Err("Invalid host suspension reason.");
        }
        if self.revision == 0
            || self.assignment_revision == 0
            || self.next_sequence == 0
            || self.players.len() != usize::from(PLAYER_CAPACITY)
            || self.company.is_empty()
            || self.company.len() > PARTY_SIZE
            || self
                .company
                .iter()
                .map(|h| usize::from(ActorKind::Hero(h.hero).footprint()))
                .sum::<usize>()
                > PARTY_SIZE
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
                || (!player.occupied && (player.connected || player.ready || player.peer.is_some()))
                || (player.slot == 0
                    && (!player.occupied || !player.connected || player.peer.is_some()))
                || player
                    .peer
                    .is_some_and(|peer| !peer.is_valid() || !peers.insert(peer))
            {
                return Err("Invalid or duplicate participant reservation.");
            }
        }
        for member in &self.company {
            if member.actor.0 == 0
                || !actors.insert(member.actor)
                || !self
                    .players
                    .iter()
                    .any(|p| p.slot == member.owner && p.occupied)
            {
                return Err("Every character requires one admitted controller.");
            }
        }
        if let Some(combat) = &self.combat {
            combat.validate().map_err(|_| "Invalid combat snapshot.")?;
            if self.encounter == 0
                || combat
                    .actors
                    .iter()
                    .filter(|a| a.team() == labyrinth_rules::Team::Heroes)
                    .count()
                    != self.company.len()
            {
                return Err("Combat must match the configured company.");
            }
            for member in &self.company {
                if !combat.actor(member.actor).is_some_and(|actor| {
                    actor.kind == ActorKind::Hero(member.hero)
                        && actor.abilities == member.abilities
                }) {
                    return Err("Combat actor does not match its configured build.");
                }
            }
        }
        let disconnected = self.required_controller_disconnected();
        if (self.interruption == CombatInterruption::WaitingForPlayers && !disconnected)
            || (!self.paused && disconnected)
        {
            return Err("Suspension does not match required controllers.");
        }
        if self.events.iter().any(|e| e.id == 0)
            || self.events.windows(2).any(|pair| pair[0].id >= pair[1].id)
        {
            return Err("Invalid session event order.");
        }
        Ok(())
    }

    fn required_controller_disconnected(&self) -> bool {
        self.combat.as_ref().is_some_and(|combat| {
            self.company.iter().any(|member| {
                combat
                    .actor(member.actor)
                    .is_some_and(|actor| actor.standing() || actor.dying())
                    && self
                        .players
                        .iter()
                        .any(|p| p.slot == member.owner && !p.connected)
            })
        })
    }

    pub fn player_views(&self) -> Vec<PlayerView> {
        self.players
            .iter()
            .map(|player| {
                let mut view = player.view();
                view.actors = self
                    .company
                    .iter()
                    .filter(|member| member.owner == player.slot)
                    .filter(|member| {
                        self.combat.as_ref().is_none_or(|combat| {
                            combat
                                .actor(member.actor)
                                .is_some_and(|actor| actor.standing() || actor.dying())
                        })
                    })
                    .map(|member| member.actor)
                    .collect();
                view
            })
            .collect()
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
        if self.assignment_revision < previous.assignment_revision {
            return Err("Controller revision moved backwards.");
        }
        if self.encounter == previous.encounter && previous.combat.is_some() {
            if self
                .players
                .iter()
                .zip(&previous.players)
                .any(|(a, b)| a.slot != b.slot || a.peer != b.peer || a.occupied != b.occupied)
                || self.company.len() != previous.company.len()
                || self.company.iter().zip(&previous.company).any(|(a, b)| {
                    a.actor != b.actor || a.hero != b.hero || a.abilities != b.abilities
                })
            {
                return Err("Participant identities or builds changed during combat.");
            }
            if self.company != previous.company
                && self.assignment_revision <= previous.assignment_revision
            {
                return Err("Controller change requires a new revision.");
            }
        }
        Ok(())
    }
}

#[derive(Resource)]
pub(crate) struct PartyAuthority {
    players: Vec<PlayerState>,
    company: Vec<CompanyMember>,
    assignment_revision: u64,
    assignment_pause: bool,
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
        Self::with_roster(seed, local, &labyrinth_rules::PROTOTYPE_HERO_ROSTER)
            .expect("authored company fills six spaces")
    }

    /// Game-owned company setup; an explicit six-human roster is also supported.
    pub(crate) fn with_roster(
        seed: u64,
        local: bool,
        roster: &[HeroClass],
    ) -> Result<Self, &'static str> {
        if roster.is_empty()
            || roster.len() > PARTY_SIZE
            || roster
                .iter()
                .map(|h| usize::from(ActorKind::Hero(*h).footprint()))
                .sum::<usize>()
                > PARTY_SIZE
        {
            return Err("A company needs one to six formation spaces.");
        }
        let players = (0..PLAYER_CAPACITY)
            .map(|slot| PlayerState {
                slot,
                peer: None,
                occupied: slot == 0,
                connected: slot == 0,
                ready: local && slot == 0,
            })
            .collect();
        let company = ACTORS
            .into_iter()
            .zip(roster.iter().copied())
            .map(|(actor, hero)| CompanyMember {
                actor,
                hero,
                abilities: HeroSetup::preset(actor, hero).abilities,
                owner: 0,
            })
            .collect();
        Ok(Self {
            players,
            company,
            assignment_revision: 1,
            assignment_pause: false,
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
        })
    }

    pub fn in_lobby(&self) -> bool {
        self.combat.is_none()
    }
    pub fn occupied(&self) -> u8 {
        u8::try_from(self.players.iter().filter(|player| player.occupied).count())
            .unwrap_or(PLAYER_CAPACITY)
    }
    pub fn has_space(&self) -> bool {
        self.in_lobby() && self.players.iter().any(|p| !p.occupied)
    }
    pub fn capacity(&self) -> u8 {
        PLAYER_CAPACITY
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
            // Returning a reservation to the lobby returns its characters to the host.
            for member in &mut self.company {
                if member.owner == player.slot {
                    member.owner = 0;
                }
            }
            self.assignment_revision += 1;
            // A new identity must not inherit replay history.
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
            company: self.company.clone(),
            assignment_revision: self.assignment_revision,
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
        } else if self.combat.is_some() && self.assignment_pause {
            CombatInterruption::Assignments
        } else if self.combat.as_ref().is_some_and(|combat| {
            self.company.iter().any(|member| {
                combat
                    .snapshot()
                    .actor(member.actor)
                    .is_some_and(|actor| actor.standing() || actor.dying())
                    && self
                        .players
                        .iter()
                        .any(|p| p.slot == member.owner && !p.connected)
            })
        }) {
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
        let _player = self
            .players
            .iter()
            .find(|p| p.slot == slot && p.connected)
            .ok_or("Player is not admitted.")?;
        if request.encounter != self.encounter {
            return Err("That encounter has ended.".into());
        }
        match request.command {
            SessionCommand::ChooseHero { actor, hero } => {
                if !self.in_lobby() {
                    return Err("Builds are chosen in the lobby.".into());
                }
                let member = self
                    .company
                    .iter()
                    .find(|m| m.actor == actor)
                    .ok_or("Unknown character.")?;
                if slot != 0 && member.owner != slot {
                    return Err("That is not your character.".into());
                }
                let used: usize = self
                    .company
                    .iter()
                    .filter(|m| m.actor != actor)
                    .map(|m| usize::from(ActorKind::Hero(m.hero).footprint()))
                    .sum();
                if used + usize::from(ActorKind::Hero(hero).footprint()) > PARTY_SIZE {
                    return Err("That build exceeds six formation spaces. The host must adjust the roster first.".into());
                }
                if member.hero != hero {
                    let member = self
                        .company
                        .iter_mut()
                        .find(|m| m.actor == actor)
                        .ok_or("Unknown character.")?;
                    member.hero = hero;
                    member.abilities = HeroSetup::preset(actor, hero).abilities;
                    for player in &mut self.players {
                        player.ready = false;
                    }
                }
            }
            SessionCommand::AssignmentPause(paused) => {
                if slot != 0 || self.combat.is_none() {
                    return Err("Only the host can pause an encounter for assignment.".into());
                }
                if self.faulted {
                    return Err("A halted encounter must return to the lobby.".into());
                }
                self.assignment_pause = paused;
            }
            SessionCommand::Assign { actor, owner } => {
                if slot != 0 {
                    return Err("Only the host assigns characters.".into());
                }
                if self.combat.is_some() && !self.assignment_pause {
                    return Err("Pause assignments before changing combat control.".into());
                }
                if self.faulted {
                    return Err("A halted encounter must return to the lobby.".into());
                }
                if !self
                    .players
                    .iter()
                    .any(|p| p.slot == owner && p.occupied && p.connected)
                {
                    return Err("Assign to a connected participant.".into());
                }
                if self.combat.as_ref().is_some_and(|c| {
                    c.snapshot()
                        .actor(actor)
                        .is_none_or(|a| !a.standing() && !a.dying())
                }) {
                    return Err("Only surviving characters can be reassigned.".into());
                }
                let member = self
                    .company
                    .iter_mut()
                    .find(|m| m.actor == actor)
                    .ok_or("Unknown hero.")?;
                if member.owner != owner {
                    member.owner = owner;
                    self.assignment_revision += 1;
                    if self.in_lobby() {
                        for p in &mut self.players {
                            p.ready = false;
                        }
                    }
                }
            }
            SessionCommand::Ready(ready) => {
                if !self.in_lobby() {
                    return Err("The encounter has already begun.".into());
                }
                for player in &mut self.players {
                    if player.slot == slot {
                        player.ready = ready;
                    }
                }
            }
            SessionCommand::Start => {
                if slot != 0 || !self.in_lobby() {
                    return Err("Only the host can start from the lobby.".into());
                }
                if self
                    .players
                    .iter()
                    .any(|p| p.occupied && (!p.connected || !p.ready))
                {
                    return Err("Every reserved participant must be connected and ready.".into());
                }
                let heroes: Vec<HeroSetup> = self
                    .company
                    .iter()
                    .map(|p| HeroSetup {
                        id: p.actor,
                        class: p.hero,
                        abilities: p.abilities.clone(),
                    })
                    .collect();
                // Company order is the host-selected starting formation.
                self.combat = Some(
                    Combat::with_party(self.seed.wrapping_add(self.encounter), heroes)
                        .map_err(|e| e.to_string())?,
                );
                self.encounter += 1;
                self.log.clear();
                self.events.clear();
                self.faulted = false;
                self.assignment_pause = false;
                self.log.push_back("The party enters the Labyrinth.".into());
            }
            SessionCommand::Rematch => {
                if slot != 0 {
                    return Err("Only the host can return the party to the lobby.".into());
                }
                self.combat = None;
                self.faulted = false;
                self.assignment_pause = false;
                self.encounter += 1;
                for p in &mut self.players {
                    p.ready = self.local && p.occupied;
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
                if request.assignment_revision != self.assignment_revision {
                    return Err("Character control changed. Refresh before acting.".into());
                }
                if !self
                    .company
                    .iter()
                    .any(|m| m.actor == actor && m.owner == slot)
                {
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
