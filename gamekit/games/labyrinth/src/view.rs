//! Immutable application projections and local intents, independent of network authority.

use bevy::prelude::*;
use bevy_game_multiplayer::SessionId;
use labyrinth_rules::{ActorId, CombatAction, CombatEvent, CombatSnapshot, HeroClass};
use serde::{Deserialize, Serialize};
use zeroize::Zeroize as _;

/// Owned UI input which redacts diagnostics and clears its allocation on drop.
#[derive(Clone, Default)]
pub struct SecretText(pub String);

impl std::fmt::Debug for SecretText {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("SecretText([REDACTED])")
    }
}
impl Drop for SecretText {
    fn drop(&mut self) {
        self.0.zeroize();
    }
}

/// Coarse authoritative screen; forms/inspection are local UI state.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum ViewMode {
    /// No admitted session.
    #[default]
    Menu,
    /// Four-player ready lobby.
    Lobby,
    /// Live or completed encounter.
    Combat,
}

/// One player reservation displayed in the lobby and combat HUD.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PlayerView {
    /// Stable zero-based player slot, never a formation rank.
    pub slot: u8,
    /// Selected hero.
    pub hero: HeroClass,
    /// Human-readable label.
    pub name: String,
    /// Reserved/admitted slot rather than vacant.
    pub occupied: bool,
    /// Actual admitted transport is present.
    pub connected: bool,
    /// Prepared to begin the encounter.
    pub ready: bool,
}

/// Sanitized session-browser item; no route or credential.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ListingView {
    /// Provider-neutral identity for join selection.
    pub id: SessionId,
    /// Session name and provider/occupancy/freshness text.
    pub label: String,
    /// Whether protocol and rules match.
    pub compatible: bool,
}

/// A bounded presentation outcome identified across all encounters in a session.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PresentedEvent {
    /// Monotonic session identity used to avoid replaying effects after snapshots.
    pub id: u64,
    /// Typed authoritative rule outcome, never a command to mutate client state.
    pub event: CombatEvent,
}

/// Entire non-authoritative presentation input. Secret codes are never placed here.
#[derive(Resource, Debug, Clone, Default)]
pub struct LabyrinthView {
    /// Session/encounter phase.
    pub mode: ViewMode,
    /// Local development controls all heroes without sockets.
    pub local: bool,
    /// This application owns the host.
    pub host: bool,
    /// Local player slot when admitted.
    pub player: Option<u8>,
    /// Local admission completed.
    pub admitted: bool,
    /// Combat is waiting for reserved players.
    pub paused: bool,
    /// Monotonic projection revision.
    pub revision: u64,
    /// Encounter identity captured with a confirmed action, never guessed at send time.
    pub encounter: u64,
    /// Current lobby reservations.
    pub players: Vec<PlayerView>,
    /// Read-only pure rules snapshot; never host RNG or authority.
    pub combat: Option<CombatSnapshot>,
    /// Ordered readable combat outcomes.
    pub log: Vec<String>,
    /// Recent typed outcomes; reconnecting clients establish a new animation baseline.
    pub events: Vec<PresentedEvent>,
    /// Most recent local/transport/admission diagnostic.
    pub notice: Option<String>,
    /// Public host label.
    pub session_name: String,
    /// Each entry labels an independent invitation; copy is an explicit intent.
    pub invite_labels: Vec<String>,
    /// Browser observations projected from all providers.
    pub listings: Vec<ListingView>,
    /// Public provider failure notices.
    pub provider_notices: Vec<String>,
}

/// Validated by hosting, rather than trusted because it came from a form.
#[derive(Debug, Clone)]
pub struct HostSettings {
    /// Public name.
    pub name: String,
    /// Temporary session admission secret.
    pub password: SecretText,
    /// Private-code destination; providers retain independent routes.
    pub address: String,
    /// Direct transport UDP port.
    pub port: u16,
    /// Opt-in local discovery.
    pub lan: bool,
    /// Explicit development tailnet opt-in.
    pub tailnet: bool,
    /// Reproducible initial encounter seed.
    pub seed: u64,
}

/// UI returns intent; the application validates and routes it to authority.
#[derive(Message, Debug, Clone)]
pub enum LabyrinthIntent {
    /// Start a four-hero local rules session.
    StartLocal(u64),
    /// Open a listen host.
    Host(HostSettings),
    /// Join through a private bearer invitation.
    JoinCode(SecretText),
    /// Open/refresh session browsing; tailnet must be explicit.
    Browse {
        /// Whether tailnet CLI use is enabled.
        tailnet: bool,
    },
    /// Close discovery browsing.
    StopBrowsing,
    /// Resolve a listed route and use encrypted password admission.
    JoinDiscovered {
        /// Provider-neutral selected session.
        session: SessionId,
        /// Temporary passphrase.
        password: SecretText,
    },
    /// Recover the profile's stored reserved identity.
    Reconnect,
    /// Explicitly leave/close the current session.
    Leave,
    /// Choose an available archetype in the lobby.
    SelectHero(HeroClass),
    /// Change local readiness.
    Ready(bool),
    /// Host starts after all four players are ready.
    StartEncounter,
    /// Host returns the current party to the lobby.
    Rematch,
    /// Submit an owned hero's action at the visible decision boundary.
    Combat {
        /// Acting hero; authority validates ownership.
        actor: ActorId,
        /// Typed game-owned action.
        action: CombatAction,
        /// Encounter shown when the user confirmed this action.
        encounter: u64,
        /// Decision/turn shown when the user confirmed this action.
        decision: u64,
    },
    /// Explicit platform clipboard write for one invitation.
    CopyInvite(usize),
    /// Revoke and replace an unused guest invitation.
    ReissueInvite(usize),
}
