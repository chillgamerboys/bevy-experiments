//! Game-owned deckbuilder rules, authority, wire commands, and disclosure projections.

use std::collections::{BTreeMap, BTreeSet, VecDeque};

use bevy::prelude::{Message, Resource};
use bevy_game_turns::TurnOrder;
use serde::{Deserialize, Serialize};

const STARTING_ENERGY: u8 = 3;
const RESULT_CACHE_LIMIT: usize = 64;

/// Two game-owned seats; the shared multiplayer crates know nothing about them.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub(crate) enum Seat {
    Host,
    Guest,
}

impl Seat {
    pub(crate) const fn label(self) -> &'static str {
        match self {
            Self::Host => "Host",
            Self::Guest => "Guest",
        }
    }
}

/// Fixed cards owned by this demo, not by Gamekit.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub(crate) enum CardKind {
    Spark,
    Ward,
    Comet,
}

impl CardKind {
    pub(crate) const fn title(self) -> &'static str {
        match self {
            Self::Spark => "Spark",
            Self::Ward => "Ward",
            Self::Comet => "Comet",
        }
    }

    pub(crate) const fn rules(self) -> &'static str {
        match self {
            Self::Spark => "Deal 1 damage.",
            Self::Ward => "Gain 2 armor.",
            Self::Comet => "Deal 5 damage.",
        }
    }

    pub(crate) const fn cost(self) -> u8 {
        match self {
            Self::Spark => 1,
            Self::Ward => 2,
            Self::Comet => 5,
        }
    }
}

#[derive(Debug, Clone)]
struct PlayerState {
    connected: bool,
    ready: bool,
    energy: u8,
    hand: Vec<CardKind>,
    played: BTreeSet<CardKind>,
}

impl PlayerState {
    fn new(connected: bool) -> Self {
        Self {
            connected,
            ready: false,
            energy: STARTING_ENERGY,
            hand: vec![CardKind::Spark, CardKind::Ward, CardKind::Comet],
            played: BTreeSet::new(),
        }
    }
}

/// Game-owned session phase.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) enum MatchPhase {
    Lobby,
    Playing,
}

/// Monotonic per-seat sequence, restored from the running host after reconnect.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub(crate) struct RequestId(pub(crate) u64);

impl RequestId {
    #[cfg(test)]
    pub(crate) const fn fixture(value: u8) -> Self {
        Self(value as u64)
    }
}

/// Authenticated request payload. It intentionally contains no seat field.
#[derive(Message, Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) struct GameRequest {
    pub(crate) request_id: RequestId,
    pub(crate) command: GameCommand,
}

/// Deckbuilder-specific authoritative command vocabulary.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) enum GameCommand {
    SetReady(bool),
    StartMatch,
    PlayCard(CardKind),
    EndTurn,
}

/// Disclosure-safe command refusal.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) enum CommandRefusal {
    WrongPhase,
    NotConnected,
    NotHost,
    WaitingForPlayers,
    NotCurrentTurn,
    CardUnavailable,
    InsufficientEnergy,
    StaleRequest,
}

/// Idempotent command outcome.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) enum CommandOutcome {
    Accepted,
    Duplicate { original_sequence: u64 },
    Refused(CommandRefusal),
}

/// Ordered result returned to the command source.
#[derive(Message, Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) struct GameCommandResult {
    pub(crate) request_id: RequestId,
    pub(crate) sequence: u64,
    pub(crate) outcome: CommandOutcome,
}

/// Public seat projection safe for either participant.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) struct PublicSeat {
    pub(crate) seat: Seat,
    pub(crate) connected: bool,
    pub(crate) ready: bool,
    pub(crate) energy: u8,
    pub(crate) hand_count: usize,
}

/// One recipient's private hand projection.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) struct PrivateCard {
    pub(crate) kind: CardKind,
    pub(crate) played: bool,
}

/// Full target-specific authoritative snapshot.
#[derive(Message, Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) struct GameSnapshot {
    pub(crate) sequence: u64,
    pub(crate) next_request: u64,
    pub(crate) phase: MatchPhase,
    pub(crate) round: u64,
    pub(crate) current_turn: Seat,
    pub(crate) seats: Vec<PublicSeat>,
    pub(crate) recipient: Seat,
    pub(crate) own_hand: Vec<PrivateCard>,
    pub(crate) activity: Vec<String>,
}

/// Complete model held only by solo/listen-host authority.
#[derive(Resource, Debug)]
pub(crate) struct DeckAuthority {
    phase: MatchPhase,
    turns: TurnOrder<Seat>,
    players: BTreeMap<Seat, PlayerState>,
    activity: Vec<String>,
    sequence: u64,
    results: BTreeMap<(Seat, RequestId), GameCommandResult>,
    result_order: VecDeque<(Seat, RequestId)>,
    request_watermarks: BTreeMap<Seat, u64>,
}

impl DeckAuthority {
    pub(crate) fn solo() -> Self {
        let mut authority = Self::lobby();
        for player in authority.players.values_mut() {
            player.connected = true;
            player.ready = true;
        }
        authority.phase = MatchPhase::Playing;
        authority.activity = vec!["The solo duel begins.".to_owned()];
        authority
    }

    pub(crate) fn lobby() -> Self {
        let players = BTreeMap::from([
            (Seat::Host, PlayerState::new(true)),
            (Seat::Guest, PlayerState::new(false)),
        ]);
        Self {
            phase: MatchPhase::Lobby,
            turns: TurnOrder::new([Seat::Host, Seat::Guest])
                .expect("fixed deckbuilder roster is unique"),
            players,
            activity: vec!["Waiting for both duelists.".to_owned()],
            sequence: 0,
            results: BTreeMap::new(),
            result_order: VecDeque::new(),
            request_watermarks: BTreeMap::new(),
        }
    }

    pub(crate) fn set_connected(&mut self, seat: Seat, connected: bool) {
        if let Some(player) = self.players.get_mut(&seat) {
            player.connected = connected;
            if self.phase == MatchPhase::Lobby && !connected {
                player.ready = false;
            }
        }
        self.sequence = self.sequence.saturating_add(1);
        self.activity.push(format!(
            "{} {}.",
            seat.label(),
            if connected {
                "connected"
            } else {
                "disconnected"
            }
        ));
    }

    pub(crate) fn apply(&mut self, source: Seat, request: GameRequest) -> GameCommandResult {
        if let Some(previous) = self.results.get(&(source, request.request_id)) {
            return GameCommandResult {
                request_id: request.request_id,
                sequence: self.sequence,
                outcome: CommandOutcome::Duplicate {
                    original_sequence: previous.sequence,
                },
            };
        }
        let watermark = self.request_watermarks.entry(source).or_default();
        if request.request_id.0 <= *watermark {
            return GameCommandResult {
                request_id: request.request_id,
                sequence: self.sequence,
                outcome: CommandOutcome::Refused(CommandRefusal::StaleRequest),
            };
        }
        // Retained for the entire live host session, independently of result eviction.
        // Refused commands consume their sequence too: retries must not become legal later.
        *watermark = request.request_id.0;
        let outcome = self.reduce(source, request.command);
        if outcome == CommandOutcome::Accepted {
            self.sequence = self.sequence.saturating_add(1);
        }
        let result = GameCommandResult {
            request_id: request.request_id,
            sequence: self.sequence,
            outcome,
        };
        self.results.insert((source, request.request_id), result);
        self.result_order.push_back((source, request.request_id));
        while self.result_order.len() > RESULT_CACHE_LIMIT {
            if let Some(oldest) = self.result_order.pop_front() {
                self.results.remove(&oldest);
            }
        }
        result
    }

    pub(crate) fn snapshot(&self, recipient: Seat) -> GameSnapshot {
        let seats = self
            .players
            .iter()
            .map(|(&seat, player)| PublicSeat {
                seat,
                connected: player.connected,
                ready: player.ready,
                energy: player.energy,
                hand_count: player.hand.len().saturating_sub(player.played.len()),
            })
            .collect();
        let own_hand = self
            .players
            .get(&recipient)
            .map(|player| {
                player
                    .hand
                    .iter()
                    .map(|&kind| PrivateCard {
                        kind,
                        played: player.played.contains(&kind),
                    })
                    .collect()
            })
            .unwrap_or_default();
        GameSnapshot {
            sequence: self.sequence,
            next_request: self.next_request(recipient),
            phase: self.phase,
            round: self.turns.round(),
            current_turn: self.turns.current().copied().unwrap_or(Seat::Host),
            seats,
            recipient,
            own_hand,
            activity: self.activity.clone(),
        }
    }

    pub(crate) fn next_request(&self, seat: Seat) -> u64 {
        self.request_watermarks
            .get(&seat)
            .copied()
            .unwrap_or(0)
            .saturating_add(1)
    }

    fn reduce(&mut self, source: Seat, command: GameCommand) -> CommandOutcome {
        let connected = self
            .players
            .get(&source)
            .is_some_and(|player| player.connected);
        if !connected {
            return CommandOutcome::Refused(CommandRefusal::NotConnected);
        }
        match command {
            GameCommand::SetReady(ready) => {
                if self.phase != MatchPhase::Lobby {
                    return CommandOutcome::Refused(CommandRefusal::WrongPhase);
                }
                if let Some(player) = self.players.get_mut(&source) {
                    player.ready = ready;
                }
                self.activity.push(format!(
                    "{} is {}.",
                    source.label(),
                    if ready { "ready" } else { "not ready" }
                ));
                CommandOutcome::Accepted
            }
            GameCommand::StartMatch => {
                if self.phase != MatchPhase::Lobby {
                    return CommandOutcome::Refused(CommandRefusal::WrongPhase);
                }
                if source != Seat::Host {
                    return CommandOutcome::Refused(CommandRefusal::NotHost);
                }
                if !self
                    .players
                    .values()
                    .all(|player| player.connected && player.ready)
                {
                    return CommandOutcome::Refused(CommandRefusal::WaitingForPlayers);
                }
                self.phase = MatchPhase::Playing;
                self.activity.push("The network duel begins.".to_owned());
                CommandOutcome::Accepted
            }
            GameCommand::PlayCard(card) => {
                if self.phase != MatchPhase::Playing {
                    return CommandOutcome::Refused(CommandRefusal::WrongPhase);
                }
                if self.turns.current() != Some(&source) {
                    return CommandOutcome::Refused(CommandRefusal::NotCurrentTurn);
                }
                let Some(player) = self.players.get_mut(&source) else {
                    return CommandOutcome::Refused(CommandRefusal::NotConnected);
                };
                if !player.hand.contains(&card) || player.played.contains(&card) {
                    return CommandOutcome::Refused(CommandRefusal::CardUnavailable);
                }
                if player.energy < card.cost() {
                    return CommandOutcome::Refused(CommandRefusal::InsufficientEnergy);
                }
                player.energy -= card.cost();
                player.played.insert(card);
                self.activity
                    .push(format!("{} played {}.", source.label(), card.title()));
                CommandOutcome::Accepted
            }
            GameCommand::EndTurn => {
                if self.phase != MatchPhase::Playing {
                    return CommandOutcome::Refused(CommandRefusal::WrongPhase);
                }
                if self.turns.current() != Some(&source) {
                    return CommandOutcome::Refused(CommandRefusal::NotCurrentTurn);
                }
                let Ok(transition) = self.turns.advance() else {
                    return CommandOutcome::Refused(CommandRefusal::WrongPhase);
                };
                if let Some(next) = self.players.get_mut(&transition.current) {
                    next.energy = STARTING_ENERGY;
                    next.played.clear();
                }
                self.activity.push(format!(
                    "{} ended their turn; {} begins.",
                    source.label(),
                    transition.current.label()
                ));
                CommandOutcome::Accepted
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn request(id: u8, command: GameCommand) -> GameRequest {
        GameRequest {
            request_id: RequestId::fixture(id),
            command,
        }
    }

    #[test]
    fn replay_watermark_survives_result_eviction_and_guest_reconnection() {
        let mut authority = DeckAuthority::lobby();
        authority.set_connected(Seat::Guest, true);
        let old = request(1, GameCommand::SetReady(true));
        assert_eq!(
            authority.apply(Seat::Guest, old).outcome,
            CommandOutcome::Accepted
        );
        for id in 2..=100 {
            authority.apply(Seat::Guest, request(id, GameCommand::SetReady(false)));
        }
        authority.set_connected(Seat::Guest, false);
        authority.set_connected(Seat::Guest, true);
        let before = authority.snapshot(Seat::Guest);
        assert_eq!(
            authority.apply(Seat::Guest, old).outcome,
            CommandOutcome::Refused(CommandRefusal::StaleRequest)
        );
        assert_eq!(authority.snapshot(Seat::Guest), before);
        assert_eq!(before.next_request, 101);
        assert_eq!(
            authority
                .apply(Seat::Guest, request(101, GameCommand::SetReady(true)))
                .outcome,
            CommandOutcome::Accepted
        );
    }

    #[test]
    fn lobby_requires_connected_ready_players_and_host_start() {
        let mut authority = DeckAuthority::lobby();
        assert_eq!(
            authority
                .apply(Seat::Host, request(1, GameCommand::StartMatch))
                .outcome,
            CommandOutcome::Refused(CommandRefusal::WaitingForPlayers)
        );
        authority.set_connected(Seat::Guest, true);
        assert_eq!(
            authority
                .apply(Seat::Host, request(2, GameCommand::SetReady(true)))
                .outcome,
            CommandOutcome::Accepted
        );
        assert_eq!(
            authority
                .apply(Seat::Guest, request(3, GameCommand::SetReady(true)))
                .outcome,
            CommandOutcome::Accepted
        );
        assert_eq!(
            authority
                .apply(Seat::Guest, request(4, GameCommand::StartMatch))
                .outcome,
            CommandOutcome::Refused(CommandRefusal::NotHost)
        );
        assert_eq!(
            authority
                .apply(Seat::Host, request(5, GameCommand::StartMatch))
                .outcome,
            CommandOutcome::Accepted
        );
    }

    #[test]
    fn each_seat_acts_only_on_its_turn_and_duplicates_apply_once() {
        let mut authority = DeckAuthority::solo();
        let play = request(1, GameCommand::PlayCard(CardKind::Spark));
        assert_eq!(
            authority.apply(Seat::Host, play).outcome,
            CommandOutcome::Accepted
        );
        let after_first = authority.snapshot(Seat::Host);
        assert!(matches!(
            authority.apply(Seat::Host, play).outcome,
            CommandOutcome::Duplicate { .. }
        ));
        assert_eq!(authority.snapshot(Seat::Host), after_first);
        assert_eq!(
            authority
                .apply(Seat::Guest, request(2, GameCommand::EndTurn))
                .outcome,
            CommandOutcome::Refused(CommandRefusal::NotCurrentTurn)
        );
        assert_eq!(
            authority
                .apply(Seat::Host, request(3, GameCommand::EndTurn))
                .outcome,
            CommandOutcome::Accepted
        );
        assert_eq!(
            authority
                .apply(
                    Seat::Guest,
                    request(4, GameCommand::PlayCard(CardKind::Ward))
                )
                .outcome,
            CommandOutcome::Accepted
        );
    }

    #[test]
    fn projections_never_include_opponent_hand_identities() {
        let authority = DeckAuthority::solo();
        let host = authority.snapshot(Seat::Host);
        let guest = authority.snapshot(Seat::Guest);
        assert_eq!(host.own_hand.len(), 3);
        assert_eq!(guest.own_hand.len(), 3);
        assert_eq!(host.recipient, Seat::Host);
        assert_eq!(guest.recipient, Seat::Guest);
        let encoded = serde_json::to_string(&host).expect("snapshot serializes");
        assert_eq!(encoded.matches("Spark").count(), 1);
    }

    #[test]
    fn lobby_disconnect_resets_ready_but_match_disconnect_reserves_state() {
        let mut authority = DeckAuthority::lobby();
        authority.set_connected(Seat::Guest, true);
        authority.apply(Seat::Guest, request(1, GameCommand::SetReady(true)));
        authority.set_connected(Seat::Guest, false);
        let guest = authority
            .snapshot(Seat::Guest)
            .seats
            .into_iter()
            .find(|seat| seat.seat == Seat::Guest)
            .expect("guest projection");
        assert!(!guest.connected);
        assert!(!guest.ready);
    }
}
