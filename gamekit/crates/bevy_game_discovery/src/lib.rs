//! Provider-neutral multiplayer session discovery.
//!
//! Discovery publishes sanitized, unauthenticated availability hints. Transport
//! establishes a pinned encrypted connection, shared security authenticates it, and
//! each game retains admission, seat, lobby, command, and disclosure policy.

#[cfg(feature = "mdns")]
mod mdns;
mod password;
#[cfg(feature = "tailscale-cli")]
mod tailscale;

use std::{
    collections::{BTreeMap, BTreeSet, VecDeque},
    fmt,
    time::Duration,
};

use bevy::prelude::*;
use bevy_game_multiplayer::{
    CertificateFingerprint, DirectEndpoint, DirectTransportError, DiscoveredDirectTarget,
    PreparedDirectDiscoveryJoin, SessionId,
};
use serde::{Deserialize, Serialize};

#[cfg(feature = "mdns")]
pub use mdns::{
    MdnsAdvertiser, MdnsBrowser, MdnsDiscoveryError, MdnsSessionAdvertisement, MDNS_SERVICE_TYPE,
};
pub use password::{
    PasswordAttemptError, PasswordPolicyError, SessionPassword, SessionPasswordVerifier,
};
#[cfg(feature = "tailscale-cli")]
pub use tailscale::{
    TailnetBrowser, TailnetDiscoveryError, TailnetResponder, TailscaleCli, TailscalePeer,
    TAILNET_DISCOVERY_PORT,
};

const MAX_GAME_ID_BYTES: usize = 64;
const MAX_VERSION_BYTES: usize = 64;
const MAX_BUILD_BYTES: usize = 128;
const MAX_DISPLAY_NAME_BYTES: usize = 96;

/// Installs the provider-neutral discovery registry and lifecycle messages.
pub struct DiscoveryPlugin;

/// Stable ordering seams for provider observations and game presentation.
#[derive(SystemSet, Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum DiscoverySystems {
    /// Apply observations and expire stale routes.
    Maintain,
    /// Resolve game/UI join requests after maintenance.
    ResolveJoins,
}

impl Plugin for DiscoveryPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<DiscoveryRegistry>()
            .add_message::<DiscoveryObservation>()
            .add_message::<DiscoveryEvent>()
            .add_message::<JoinDiscoveredSession>()
            .add_message::<DiscoveredJoinReady>()
            .configure_sets(
                Update,
                (DiscoverySystems::Maintain, DiscoverySystems::ResolveJoins).chain(),
            )
            .add_systems(Update, maintain_registry.in_set(DiscoverySystems::Maintain))
            .add_systems(
                Update,
                resolve_join_requests.in_set(DiscoverySystems::ResolveJoins),
            );
    }
}

/// Stable provider identifier; games may define additional provider IDs.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct DiscoveryProviderId(pub u64);

impl DiscoveryProviderId {
    /// Local DNS-SD provider.
    pub const MDNS: Self = Self(10);
    /// Development-only Tailscale CLI provider.
    pub const TAILSCALE: Self = Self(20);
    /// Deterministic provider used by tests and service simulations.
    pub const FAKE: Self = Self(100);
}

/// Broad source category suitable for player-facing badges.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum DiscoverySource {
    /// Same-link mDNS/DNS-SD.
    Lan,
    /// Development tailnet probe.
    Tailnet,
    /// Deterministic or future service provider.
    Service,
}

/// Pre-connect compatibility hint; final admission must check exact values again.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Compatibility {
    /// Exact advertised game/protocol/build match.
    Compatible,
    /// Advertised values are known not to match.
    Incompatible,
    /// Provider could not establish compatibility.
    Unknown,
}

/// Sanitized player-visible metadata shared by every discovery provider.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SessionMetadata {
    game_id: String,
    protocol_version: String,
    build_id: String,
    display_name: String,
    claimed_players: u8,
    player_capacity: u8,
    password_required: bool,
}

impl SessionMetadata {
    /// Validates bounded metadata and sensible occupancy.
    pub fn new(
        game_id: impl Into<String>,
        protocol_version: impl Into<String>,
        build_id: impl Into<String>,
        display_name: impl Into<String>,
        claimed_players: u8,
        player_capacity: u8,
        password_required: bool,
    ) -> Result<Self, DiscoveryDataError> {
        let game_id = bounded(game_id.into(), MAX_GAME_ID_BYTES)?;
        let protocol_version = bounded(protocol_version.into(), MAX_VERSION_BYTES)?;
        let build_id = bounded(build_id.into(), MAX_BUILD_BYTES)?;
        let display_name = bounded(display_name.into(), MAX_DISPLAY_NAME_BYTES)?;
        if player_capacity == 0 || claimed_players > player_capacity {
            return Err(DiscoveryDataError::InvalidOccupancy);
        }
        Ok(Self {
            game_id,
            protocol_version,
            build_id,
            display_name,
            claimed_players,
            player_capacity,
            password_required,
        })
    }

    /// Stable game/application identifier.
    #[must_use]
    pub fn game_id(&self) -> &str {
        &self.game_id
    }

    /// Game-owned protocol version.
    #[must_use]
    pub fn protocol_version(&self) -> &str {
        &self.protocol_version
    }

    /// Exact game build/content identifier.
    #[must_use]
    pub fn build_id(&self) -> &str {
        &self.build_id
    }

    /// Host-selected player-facing session name.
    #[must_use]
    pub fn display_name(&self) -> &str {
        &self.display_name
    }

    /// Current public occupancy.
    #[must_use]
    pub const fn claimed_players(&self) -> u8 {
        self.claimed_players
    }

    /// Maximum game-owned player capacity.
    #[must_use]
    pub const fn player_capacity(&self) -> u8 {
        self.player_capacity
    }

    /// Whether discovered admission requires a session password.
    #[must_use]
    pub const fn password_required(&self) -> bool {
        self.password_required
    }
}

fn bounded(value: String, maximum: usize) -> Result<String, DiscoveryDataError> {
    if value.is_empty() || value.len() > maximum || value.chars().any(char::is_control) {
        Err(DiscoveryDataError::InvalidText)
    } else {
        Ok(value)
    }
}

/// One secret-free provider route to a session.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DiscoveryRoute {
    provider: DiscoveryProviderId,
    source: DiscoverySource,
    target: DiscoveredDirectTarget,
    expires_at: Duration,
}

impl DiscoveryRoute {
    /// Creates a provider route with an absolute registry expiry.
    #[must_use]
    pub const fn new(
        provider: DiscoveryProviderId,
        source: DiscoverySource,
        target: DiscoveredDirectTarget,
        expires_at: Duration,
    ) -> Self {
        Self {
            provider,
            source,
            target,
            expires_at,
        }
    }
}

/// Opaque ordered transport handoff for one selected discovered session.
///
/// Provider endpoints and certificate material stay internal. Games may inspect
/// broad source categories and open a pinned connection without learning whether
/// the concrete address came from mDNS, Tailscale, or a future lobby service.
#[derive(Clone)]
pub struct DiscoveryJoinRoute {
    session_id: SessionId,
    routes: Vec<(DiscoverySource, DiscoveredDirectTarget)>,
}

impl DiscoveryJoinRoute {
    /// Selected stable session identity.
    #[must_use]
    pub const fn session_id(&self) -> SessionId {
        self.session_id
    }

    /// Broad source used by the preferred route.
    #[must_use]
    pub fn preferred_source(&self) -> Option<DiscoverySource> {
        self.routes.first().map(|(source, _target)| *source)
    }

    /// Number of provider routes retained for bounded retry orchestration.
    #[must_use]
    pub fn route_count(&self) -> usize {
        self.routes.len()
    }

    /// Opens the preferred certificate-pinned encrypted connection.
    pub fn connect_preferred(&self, world: &mut World) -> Result<Entity, DiscoveryConnectError> {
        self.connect_route(world, 0)
    }

    /// Opens one retained route by preference index.
    pub fn connect_route(
        &self,
        world: &mut World,
        index: usize,
    ) -> Result<Entity, DiscoveryConnectError> {
        let (_source, target) = self
            .routes
            .get(index)
            .ok_or(DiscoveryConnectError::UnknownRoute)?;
        let prepared =
            PreparedDirectDiscoveryJoin::new(target).map_err(DiscoveryConnectError::Transport)?;
        Ok(prepared.connect(world))
    }
}

impl fmt::Debug for DiscoveryJoinRoute {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("DiscoveryJoinRoute")
            .field("session_id", &self.session_id)
            .field(
                "sources",
                &self
                    .routes
                    .iter()
                    .map(|(source, _target)| *source)
                    .collect::<Vec<_>>(),
            )
            .finish_non_exhaustive()
    }
}

/// Provider observation consumed by the registry.
#[derive(Message, Debug, Clone, PartialEq, Eq)]
pub enum DiscoveryObservation {
    /// Add or refresh one provider route.
    Found {
        /// Sanitized metadata.
        metadata: SessionMetadata,
        /// Secret-free provider route.
        route: DiscoveryRoute,
    },
    /// Remove one route immediately.
    Lost {
        /// Stable session identity.
        session_id: SessionId,
        /// Provider that lost the route.
        provider: DiscoveryProviderId,
    },
    /// Provider is currently unavailable.
    Unavailable {
        /// Affected provider.
        provider: DiscoveryProviderId,
        /// Non-secret player-facing reason.
        reason: String,
    },
    /// Provider encountered a recoverable failure.
    Failed {
        /// Affected provider.
        provider: DiscoveryProviderId,
        /// Non-secret player-facing reason.
        reason: String,
    },
}

/// Player-visible discovered session with provider-specific routing removed.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DiscoveredSession {
    /// Stable host-process session identity.
    pub session_id: SessionId,
    /// Sanitized shared metadata.
    pub metadata: SessionMetadata,
    /// Exact local compatibility hint.
    pub compatibility: Compatibility,
    /// Available provider badges, ordered by route preference.
    pub sources: Vec<DiscoverySource>,
    /// Remaining lifetime of the freshest route at projection time.
    pub freshness: Duration,
}

#[derive(Debug, Clone)]
struct RegistryEntry {
    metadata: SessionMetadata,
    compatibility: Compatibility,
    routes: BTreeMap<DiscoveryProviderId, DiscoveryRoute>,
}

/// Deduplicated session registry owned by [`DiscoveryPlugin`].
#[derive(Resource, Debug, Default)]
pub struct DiscoveryRegistry {
    entries: BTreeMap<SessionId, RegistryEntry>,
    provider_notices: BTreeMap<DiscoveryProviderId, String>,
}

/// Deterministic observation source for tests and future service-provider simulations.
///
/// It performs no I/O and intentionally exposes the same provider-neutral lifecycle
/// used by mDNS, tailnet discovery, and a future Steam lobby adapter.
#[derive(Debug, Default)]
pub struct FakeDiscoveryProvider {
    queued: VecDeque<DiscoveryObservation>,
}

impl FakeDiscoveryProvider {
    /// Queues a visible add or update with a caller-controlled expiry.
    pub fn publish(
        &mut self,
        metadata: SessionMetadata,
        target: DiscoveredDirectTarget,
        expires_at: Duration,
    ) {
        self.queued.push_back(DiscoveryObservation::Found {
            metadata,
            route: DiscoveryRoute::new(
                DiscoveryProviderId::FAKE,
                DiscoverySource::Service,
                target,
                expires_at,
            ),
        });
    }

    /// Queues immediate removal of one simulated service listing.
    pub fn remove(&mut self, session_id: SessionId) {
        self.queued.push_back(DiscoveryObservation::Lost {
            session_id,
            provider: DiscoveryProviderId::FAKE,
        });
    }

    /// Queues a typed provider-unavailable observation.
    pub fn unavailable(&mut self, reason: impl Into<String>) {
        self.queued.push_back(DiscoveryObservation::Unavailable {
            provider: DiscoveryProviderId::FAKE,
            reason: reason.into(),
        });
    }

    /// Queues a typed recoverable provider-failure observation.
    pub fn failed(&mut self, reason: impl Into<String>) {
        self.queued.push_back(DiscoveryObservation::Failed {
            provider: DiscoveryProviderId::FAKE,
            reason: reason.into(),
        });
    }

    /// Drains observations in insertion order.
    pub fn drain(&mut self) -> impl Iterator<Item = DiscoveryObservation> + '_ {
        self.queued.drain(..)
    }
}

impl DiscoveryRegistry {
    /// Applies one provider observation and returns its visible lifecycle event, if any.
    pub fn apply(
        &mut self,
        observation: DiscoveryObservation,
        expected: Option<&ExpectedSession>,
    ) -> Option<DiscoveryEvent> {
        match observation {
            DiscoveryObservation::Found { metadata, route } => {
                let session_id = route.target.session_id;
                let existed = self.entries.contains_key(&session_id);
                let compatibility = expected.map_or(Compatibility::Unknown, |expected| {
                    expected.compatibility(&metadata)
                });
                let entry = self
                    .entries
                    .entry(session_id)
                    .or_insert_with(|| RegistryEntry {
                        metadata: metadata.clone(),
                        compatibility,
                        routes: BTreeMap::new(),
                    });
                entry.metadata = metadata;
                entry.compatibility = compatibility;
                entry.routes.insert(route.provider, route);
                Some(if existed {
                    DiscoveryEvent::Updated { session_id }
                } else {
                    DiscoveryEvent::Added { session_id }
                })
            }
            DiscoveryObservation::Lost {
                session_id,
                provider,
            } => {
                let entry = self.entries.get_mut(&session_id)?;
                entry.routes.remove(&provider);
                if entry.routes.is_empty() {
                    self.entries.remove(&session_id);
                    Some(DiscoveryEvent::Removed { session_id })
                } else {
                    Some(DiscoveryEvent::Updated { session_id })
                }
            }
            DiscoveryObservation::Unavailable { provider, reason } => {
                self.provider_notices.insert(provider, reason.clone());
                Some(DiscoveryEvent::ProviderUnavailable { provider, reason })
            }
            DiscoveryObservation::Failed { provider, reason } => {
                self.provider_notices.insert(provider, reason.clone());
                Some(DiscoveryEvent::ProviderFailed { provider, reason })
            }
        }
    }

    /// Expires stale routes and returns visible lifecycle events.
    pub fn expire(&mut self, now: Duration) -> Vec<DiscoveryEvent> {
        let sessions = self.entries.keys().copied().collect::<Vec<_>>();
        let mut events = Vec::new();
        for session_id in sessions {
            let Some(entry) = self.entries.get_mut(&session_id) else {
                continue;
            };
            entry.routes.retain(|_, route| route.expires_at > now);
            if entry.routes.is_empty() {
                self.entries.remove(&session_id);
                events.push(DiscoveryEvent::Removed { session_id });
            }
        }
        events
    }

    /// Returns sanitized deduplicated sessions in stable name/identity order.
    #[must_use]
    pub fn sessions(&self, now: Duration) -> Vec<DiscoveredSession> {
        let mut sessions = self
            .entries
            .iter()
            .filter_map(|(&session_id, entry)| {
                let freshest = entry.routes.values().map(|route| route.expires_at).max()?;
                let sources = entry
                    .routes
                    .values()
                    .map(|route| route.source)
                    .collect::<BTreeSet<_>>()
                    .into_iter()
                    .collect();
                Some(DiscoveredSession {
                    session_id,
                    metadata: entry.metadata.clone(),
                    compatibility: entry.compatibility,
                    sources,
                    freshness: freshest.saturating_sub(now),
                })
            })
            .collect::<Vec<_>>();
        sessions.sort_by(|left, right| {
            left.metadata
                .display_name
                .cmp(&right.metadata.display_name)
                .then(left.session_id.cmp(&right.session_id))
        });
        sessions
    }

    /// Resolves an opaque ordered route handoff for a selected session.
    pub fn resolve(&self, session_id: SessionId) -> Result<DiscoveryJoinRoute, DiscoveryDataError> {
        let entry = self
            .entries
            .get(&session_id)
            .ok_or(DiscoveryDataError::UnknownSession)?;
        if entry.compatibility == Compatibility::Incompatible {
            return Err(DiscoveryDataError::IncompatibleSession);
        }
        let mut routes = entry
            .routes
            .values()
            .map(|route| (route.source, route.provider, route.target.clone()))
            .collect::<Vec<_>>();
        routes.sort_by_key(|(source, provider, _target)| (provider_priority(*source), *provider));
        if routes.is_empty() {
            return Err(DiscoveryDataError::UnknownSession);
        }
        Ok(DiscoveryJoinRoute {
            session_id,
            routes: routes
                .into_iter()
                .map(|(source, _provider, target)| (source, target))
                .collect(),
        })
    }

    /// Current non-secret provider notices.
    #[must_use]
    pub const fn provider_notices(&self) -> &BTreeMap<DiscoveryProviderId, String> {
        &self.provider_notices
    }
}

const fn provider_priority(source: DiscoverySource) -> u8 {
    match source {
        DiscoverySource::Lan => 0,
        DiscoverySource::Tailnet => 1,
        DiscoverySource::Service => 2,
    }
}

/// Exact game-owned identity used to pre-filter discovery results.
#[derive(Resource, Debug, Clone, PartialEq, Eq)]
pub struct ExpectedSession {
    /// Stable game ID.
    pub game_id: String,
    /// Game-owned wire protocol version.
    pub protocol_version: String,
    /// Exact build/content identifier.
    pub build_id: String,
}

impl ExpectedSession {
    fn compatibility(&self, metadata: &SessionMetadata) -> Compatibility {
        if self.game_id == metadata.game_id
            && self.protocol_version == metadata.protocol_version
            && self.build_id == metadata.build_id
        {
            Compatibility::Compatible
        } else {
            Compatibility::Incompatible
        }
    }
}

/// Registry lifecycle emitted after deduplication.
#[derive(Message, Debug, Clone, PartialEq, Eq)]
pub enum DiscoveryEvent {
    /// First route for a session appeared.
    Added {
        /// Stable session identity.
        session_id: SessionId,
    },
    /// Session metadata or routes changed.
    Updated {
        /// Stable session identity.
        session_id: SessionId,
    },
    /// Every route disappeared or expired.
    Removed {
        /// Stable session identity.
        session_id: SessionId,
    },
    /// Optional provider is not installed or connected.
    ProviderUnavailable {
        /// Provider identity.
        provider: DiscoveryProviderId,
        /// Non-secret reason.
        reason: String,
    },
    /// Provider failed but Direct remains usable.
    ProviderFailed {
        /// Provider identity.
        provider: DiscoveryProviderId,
        /// Non-secret reason.
        reason: String,
    },
}

/// Game/UI request to join one selected discovered session.
#[derive(Message, Debug, Clone, Copy, PartialEq, Eq)]
pub struct JoinDiscoveredSession {
    /// Stable session selected by the player.
    pub session_id: SessionId,
}

/// Opaque handoff emitted after provider-neutral route selection.
#[derive(Message, Debug, Clone)]
pub struct DiscoveredJoinReady {
    /// Stable selected session.
    pub session_id: SessionId,
    /// Ordered opaque routes for a password-authenticated pinned connection.
    pub route: DiscoveryJoinRoute,
}

/// Invalid discovery metadata or resolution request.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DiscoveryDataError {
    /// Required text was empty, too long, or contained controls.
    InvalidText,
    /// Occupancy exceeded capacity or capacity was zero.
    InvalidOccupancy,
    /// Selected session no longer exists.
    UnknownSession,
    /// Selected session is known to be incompatible.
    IncompatibleSession,
}

impl fmt::Display for DiscoveryDataError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(match self {
            Self::InvalidText => "discovery metadata text is invalid",
            Self::InvalidOccupancy => "discovery occupancy is invalid",
            Self::UnknownSession => "discovered session is no longer available",
            Self::IncompatibleSession => "discovered session is incompatible",
        })
    }
}

impl std::error::Error for DiscoveryDataError {}

/// Failure while turning an opaque discovered route into a pinned connection.
#[derive(Debug)]
pub enum DiscoveryConnectError {
    /// Requested alternate route does not exist in this handoff.
    UnknownRoute,
    /// Secure direct transport rejected or could not open the route.
    Transport(DirectTransportError),
}

impl fmt::Display for DiscoveryConnectError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(match self {
            Self::UnknownRoute => "discovery retry route is unavailable",
            Self::Transport(_) => "discovered secure connection could not be opened",
        })
    }
}

impl std::error::Error for DiscoveryConnectError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Transport(error) => Some(error),
            Self::UnknownRoute => None,
        }
    }
}

fn maintain_registry(
    time: Res<Time<Real>>,
    expected: Option<Res<ExpectedSession>>,
    mut observations: MessageReader<DiscoveryObservation>,
    mut registry: ResMut<DiscoveryRegistry>,
    mut events: MessageWriter<DiscoveryEvent>,
) {
    for observation in observations.read() {
        if let Some(event) = registry.apply(observation.clone(), expected.as_deref()) {
            events.write(event);
        }
    }
    for event in registry.expire(time.elapsed()) {
        events.write(event);
    }
}

fn resolve_join_requests(
    mut requests: MessageReader<JoinDiscoveredSession>,
    registry: Res<DiscoveryRegistry>,
    mut ready: MessageWriter<DiscoveredJoinReady>,
    mut events: MessageWriter<DiscoveryEvent>,
) {
    for request in requests.read() {
        match registry.resolve(request.session_id) {
            Ok(route) => {
                ready.write(DiscoveredJoinReady {
                    session_id: request.session_id,
                    route,
                });
            }
            Err(error) => {
                events.write(DiscoveryEvent::ProviderFailed {
                    provider: DiscoveryProviderId::FAKE,
                    reason: error.to_string(),
                });
            }
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct WireAnnouncement {
    schema: u8,
    session_id: SessionId,
    metadata: SessionMetadata,
    endpoint_port: u16,
    certificate_fingerprint: CertificateFingerprint,
    certificate_expires_unix_seconds: u64,
}

impl WireAnnouncement {
    fn target(
        &self,
        host: impl Into<String>,
    ) -> Result<DiscoveredDirectTarget, DiscoveryDataError> {
        let endpoint = DirectEndpoint::new(host, self.endpoint_port)
            .map_err(|_error| DiscoveryDataError::InvalidText)?;
        if self.schema != 1 || self.certificate_expires_unix_seconds == 0 {
            return Err(DiscoveryDataError::InvalidText);
        }
        Ok(DiscoveredDirectTarget {
            session_id: self.session_id,
            endpoint,
            certificate_fingerprint: self.certificate_fingerprint,
            certificate_expires_unix_seconds: self.certificate_expires_unix_seconds,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn metadata(name: &str) -> SessionMetadata {
        SessionMetadata::new("deckbuilder", "1", "build-a", name, 1, 2, true)
            .expect("valid metadata")
    }

    fn route(
        provider: DiscoveryProviderId,
        source: DiscoverySource,
        expiry: u64,
    ) -> DiscoveryRoute {
        DiscoveryRoute::new(
            provider,
            source,
            DiscoveredDirectTarget {
                session_id: SessionId::from_bytes([1; 16]),
                endpoint: DirectEndpoint::new("host.local", 7777).expect("valid endpoint"),
                certificate_fingerprint: CertificateFingerprint::from_bytes([2; 32]),
                certificate_expires_unix_seconds: 2_000_000_000,
            },
            Duration::from_secs(expiry),
        )
    }

    #[test]
    fn registry_deduplicates_routes_prefers_lan_and_expires() {
        let mut registry = DiscoveryRegistry::default();
        let expected = ExpectedSession {
            game_id: "deckbuilder".to_owned(),
            protocol_version: "1".to_owned(),
            build_id: "build-a".to_owned(),
        };
        registry.apply(
            DiscoveryObservation::Found {
                metadata: metadata("Game"),
                route: route(DiscoveryProviderId::TAILSCALE, DiscoverySource::Tailnet, 20),
            },
            Some(&expected),
        );
        registry.apply(
            DiscoveryObservation::Found {
                metadata: metadata("Game"),
                route: route(DiscoveryProviderId::MDNS, DiscoverySource::Lan, 10),
            },
            Some(&expected),
        );
        let sessions = registry.sessions(Duration::from_secs(1));
        assert_eq!(sessions.len(), 1);
        assert_eq!(
            sessions.first().expect("one session").sources,
            vec![DiscoverySource::Lan, DiscoverySource::Tailnet]
        );
        let resolved = registry
            .resolve(SessionId::from_bytes([1; 16]))
            .expect("compatible session resolves");
        assert_eq!(resolved.preferred_source(), Some(DiscoverySource::Lan));
        assert_eq!(resolved.route_count(), 2);
        assert!(registry.expire(Duration::from_secs(11)).is_empty());
        assert_eq!(registry.sessions(Duration::from_secs(11)).len(), 1);
        assert_eq!(registry.expire(Duration::from_secs(21)).len(), 1);
    }

    #[test]
    fn public_projection_contains_no_routing_or_secret_fields() {
        let mut registry = DiscoveryRegistry::default();
        registry.apply(
            DiscoveryObservation::Found {
                metadata: metadata("Safe"),
                route: route(DiscoveryProviderId::MDNS, DiscoverySource::Lan, 10),
            },
            None,
        );
        let debug = format!("{:?}", registry.sessions(Duration::ZERO));
        assert!(!debug.contains("host.local"));
        assert!(!debug.contains("invite"));
        assert!(!debug.contains("BGN1"));
        assert!(!debug.contains("copper comet"));
    }

    #[test]
    fn fake_provider_simulates_service_listing_update_and_removal() {
        let mut provider = FakeDiscoveryProvider::default();
        provider.publish(
            metadata("Steam-like Lobby"),
            route(DiscoveryProviderId::FAKE, DiscoverySource::Service, 10).target,
            Duration::from_secs(10),
        );
        let mut registry = DiscoveryRegistry::default();
        for observation in provider.drain() {
            registry.apply(observation, None);
        }
        assert_eq!(registry.sessions(Duration::ZERO).len(), 1);
        provider.remove(SessionId::from_bytes([1; 16]));
        for observation in provider.drain() {
            registry.apply(observation, None);
        }
        assert!(registry.sessions(Duration::ZERO).is_empty());
    }
}
