//! Bounded, acknowledgement-based admission for independently composed games.
//!
//! An offer is not admission: the client must durably store its offered credential
//! and acknowledge it over the same encrypted connection. Games reserve their own
//! capacity before beginning and authorize gameplay only after acknowledgement.

use std::{collections::BTreeMap, fmt, time::Duration};

use crate::{
    AdmissionCredential, AdmissionGrant, InviteToken, PeerId, ReconnectCredential, SessionId,
};

/// Resource limits, independent of any game's number or meaning of seats.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AdmissionLimits {
    /// Maximum reserved identities, including unacknowledged initial offers.
    pub max_peers: usize,
    /// Maximum outstanding independent invitations.
    pub max_invites: usize,
    /// Deadline for one physical connection to persist and acknowledge its offer.
    pub pending_timeout: Duration,
}

impl Default for AdmissionLimits {
    fn default() -> Self {
        Self {
            max_peers: 32,
            max_invites: 32,
            pending_timeout: Duration::from_secs(30),
        }
    }
}

/// Credential offer sent only over the authenticated, certificate-pinned channel.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AdmissionOffer {
    /// Reserved stable identity; not yet authorized for gameplay.
    pub peer: PeerId,
    /// Persist atomically before acknowledging; ordinary diagnostics redact it.
    pub reconnect_credential: ReconnectCredential,
    /// Whether the identity has previously completed admission.
    pub reconnected: bool,
    /// Explicit monotonic deadline for this acknowledgement attempt.
    pub expires_at: Duration,
}

/// Cleanup the game must apply to its own reservations and physical transport.
#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct AdmissionCleanup {
    /// Identities permanently released; remove their game-owned reservations.
    pub released_peers: Vec<PeerId>,
    /// Physical connections to close. Other reservations remain intact.
    pub disconnected_connections: Vec<u64>,
}

#[derive(Debug)]
struct Invitation {
    token: InviteToken,
    expires_at: Duration,
    pending_peer: Option<PeerId>,
}

#[derive(Debug)]
struct PeerRecord {
    current: Option<ReconnectCredential>,
    successor: Option<ReconnectCredential>,
    connection: Option<u64>,
    deadline: Option<Duration>,
    initial_invite: Option<InviteToken>,
    last_reconnected: bool,
}

/// Pure bounded admission with independent invitations and recoverable rotation.
///
/// Unlike the legacy [`crate::SessionSecurityAuthority`], admission is explicit:
/// `begin -> persist offered credential -> acknowledge`. Drive [`Self::expire`]
/// with monotonic time and apply every returned cleanup to game/transport state.
/// This authority does not implement passwords, seats, lobby policy, or sockets.
///
/// A disappeared guest must be detached with [`Self::disconnect`] before recovery
/// can begin; presenting a credential never evicts an existing active connection.
/// Deadlines describe host monotonic time, not client wall-clock timestamps.
///
/// ```
/// use std::time::Duration;
/// use bevy_game_session::{AdmissionCredential, AdmissionLimits, SessionAdmissionAuthority};
///
/// let mut host = SessionAdmissionAuthority::new(AdmissionLimits::default())?;
/// let invite = host.issue_invite(Duration::ZERO, Duration::from_secs(600))?;
/// let offer = host.begin(42, AdmissionCredential::Invite(invite), Duration::ZERO)?;
/// assert!(!host.is_connection_admitted(42));
/// // Send the offer over the pinned encrypted connection. The guest atomically
/// // persists offer.reconnect_credential, then sends it back in its encrypted ACK.
/// let grant = host.acknowledge(42, offer.reconnect_credential, Duration::from_secs(1))?;
/// assert_eq!(grant.peer, offer.peer);
/// assert!(host.is_connection_admitted(42));
/// # Ok::<(), bevy_game_session::AdmissionFlowError>(())
/// ```
#[derive(Debug)]
pub struct SessionAdmissionAuthority {
    session: SessionId,
    limits: AdmissionLimits,
    invitations: Vec<Invitation>,
    peers: BTreeMap<PeerId, PeerRecord>,
    connections: BTreeMap<u64, PeerId>,
    closed: bool,
}

impl SessionAdmissionAuthority {
    /// Creates a fresh session without issuing an implicit invitation.
    pub fn new(limits: AdmissionLimits) -> Result<Self, AdmissionFlowError> {
        Self::with_session(SessionId::generate(), limits)
    }

    /// Uses an explicit valid host-process identity, for composition and fixtures.
    pub fn with_session(
        session: SessionId,
        limits: AdmissionLimits,
    ) -> Result<Self, AdmissionFlowError> {
        if !session.is_valid() {
            return Err(AdmissionFlowError::InvalidSession);
        }
        if limits.max_peers == 0
            || limits.max_peers > 4096
            || limits.max_invites == 0
            || limits.max_invites > 4096
            || limits.pending_timeout.is_zero()
        {
            return Err(AdmissionFlowError::InvalidLimits);
        }
        Ok(Self {
            session,
            limits,
            invitations: Vec::new(),
            peers: BTreeMap::new(),
            connections: BTreeMap::new(),
            closed: false,
        })
    }

    /// Stable identity shared by every invitation and provider route.
    #[must_use]
    pub const fn session_id(&self) -> SessionId {
        self.session
    }

    /// Issues one independent, one-use invitation with an explicit expiry.
    ///
    /// Call [`Self::expire`] first to release expired records and apply cleanup.
    pub fn issue_invite(
        &mut self,
        now: Duration,
        expires_at: Duration,
    ) -> Result<InviteToken, AdmissionFlowError> {
        self.ensure_open()?;
        if expires_at <= now {
            return Err(AdmissionFlowError::InvalidExpiry);
        }
        if self.invitations.len() >= self.limits.max_invites {
            return Err(AdmissionFlowError::InviteLimit);
        }
        let token = loop {
            let token = InviteToken::generate();
            if !self
                .invitations
                .iter()
                .any(|entry| entry.token.matches(token))
            {
                break token;
            }
        };
        self.invitations.push(Invitation {
            token,
            expires_at,
            pending_peer: None,
        });
        Ok(token)
    }

    /// Revokes an unused invitation and any still-unacknowledged initial peer.
    pub fn revoke_invite(&mut self, token: InviteToken) -> AdmissionCleanup {
        let Some(index) = self
            .invitations
            .iter()
            .position(|entry| entry.token.matches(token))
        else {
            return AdmissionCleanup::default();
        };
        let invitation = self.invitations.remove(index);
        invitation
            .pending_peer
            .map_or_else(AdmissionCleanup::default, |peer| self.remove_peer(peer))
    }

    /// Begins an invitation or reconnect attempt after game-owned preflight checks.
    ///
    /// Repeated attempts after disconnection reuse the same pending peer and
    /// successor. An existing live connection always prevents takeover.
    pub fn begin(
        &mut self,
        connection: u64,
        credential: AdmissionCredential,
        now: Duration,
    ) -> Result<AdmissionOffer, AdmissionFlowError> {
        self.ensure_new_connection(connection)?;
        match credential {
            AdmissionCredential::Invite(token) => self.begin_invite(connection, token, now),
            AdmissionCredential::Reconnect(token) => self.begin_reconnect(connection, token, now),
        }
    }

    /// Reserves a fresh identity after an external password/provider succeeds.
    ///
    /// The caller must enforce capacity before calling. Without an acknowledgement
    /// this reservation expires; it cannot accumulate beyond `max_peers`.
    pub fn begin_external(
        &mut self,
        connection: u64,
        now: Duration,
    ) -> Result<AdmissionOffer, AdmissionFlowError> {
        self.ensure_new_connection(connection)?;
        self.create_peer(connection, None, self.deadline(now)?)
    }

    /// Commits only the credential offered to this exact physical connection.
    ///
    /// The game calls this after receiving the client's encrypted persistence ACK,
    /// then authorizes gameplay. Duplicate matching ACKs are idempotent. An old
    /// credential ceases to work immediately upon successful acknowledgement.
    pub fn acknowledge(
        &mut self,
        connection: u64,
        offered: ReconnectCredential,
        now: Duration,
    ) -> Result<AdmissionGrant, AdmissionFlowError> {
        self.ensure_open()?;
        let peer = *self
            .connections
            .get(&connection)
            .ok_or(AdmissionFlowError::UnknownConnection)?;
        let record = self
            .peers
            .get_mut(&peer)
            .ok_or(AdmissionFlowError::UnknownConnection)?;
        if let Some(deadline) = record.deadline {
            if now >= deadline {
                return Err(AdmissionFlowError::AdmissionExpired);
            }
            if !record.successor.is_some_and(|token| token.matches(offered)) {
                return Err(AdmissionFlowError::InvalidAcknowledgement);
            }
            record.current = record.successor.take();
            record.deadline = None;
            if let Some(token) = record.initial_invite.take() {
                self.invitations.retain(|entry| !entry.token.matches(token));
            }
        } else if !record.current.is_some_and(|token| token.matches(offered)) {
            return Err(AdmissionFlowError::InvalidAcknowledgement);
        }
        Ok(AdmissionGrant {
            peer,
            reconnect_credential: offered,
            reconnected: record.last_reconnected,
        })
    }

    /// Detaches a physical connection, retaining bounded recovery state.
    pub fn disconnect(&mut self, connection: u64) -> Option<PeerId> {
        let peer = self.connections.remove(&connection)?;
        if let Some(record) = self.peers.get_mut(&peer) {
            record.connection = None;
        }
        Some(peer)
    }

    /// Cancels a failed attempt; initial invitations become usable again.
    ///
    /// An established peer and its possible persisted successor remain recoverable.
    pub fn abort(&mut self, connection: u64) -> AdmissionCleanup {
        let Some(&peer) = self.connections.get(&connection) else {
            return AdmissionCleanup::default();
        };
        if self
            .peers
            .get(&peer)
            .is_some_and(|record| record.current.is_none())
        {
            self.remove_peer(peer)
        } else {
            self.disconnect(connection);
            if let Some(record) = self.peers.get_mut(&peer) {
                record.deadline = None;
            }
            AdmissionCleanup {
                released_peers: Vec::new(),
                disconnected_connections: vec![connection],
            }
        }
    }

    /// Permanently revokes an identity and all of its recovery material.
    pub fn revoke_peer(&mut self, peer: PeerId) -> AdmissionCleanup {
        if let Some(token) = self
            .peers
            .get(&peer)
            .and_then(|record| record.initial_invite)
        {
            self.invitations.retain(|entry| !entry.token.matches(token));
        }
        self.remove_peer(peer)
    }

    /// Expires invitations and unacknowledged attempts using caller-owned time.
    ///
    /// Initial pending peers are released. Established peers remain reserved;
    /// their sole pending successor remains recoverable even if the ACK was lost
    /// after the client persisted it. Only the physical attempt expires.
    pub fn expire(&mut self, now: Duration) -> AdmissionCleanup {
        let mut cleanup = AdmissionCleanup::default();
        let expired_invites: Vec<_> = self
            .invitations
            .iter()
            .filter(|entry| now >= entry.expires_at)
            .map(|entry| entry.token)
            .collect();
        for token in expired_invites {
            cleanup.extend(self.revoke_invite(token));
        }
        let expired_peers: Vec<_> = self
            .peers
            .iter()
            .filter(|(_, record)| record.deadline.is_some_and(|deadline| now >= deadline))
            .map(|(&peer, _)| peer)
            .collect();
        for peer in expired_peers {
            if self
                .peers
                .get(&peer)
                .is_some_and(|record| record.current.is_none())
            {
                cleanup.extend(self.remove_peer(peer));
            } else if let Some(record) = self.peers.get_mut(&peer) {
                record.deadline = None;
                if let Some(connection) = record.connection.take() {
                    self.connections.remove(&connection);
                    cleanup.disconnected_connections.push(connection);
                }
            }
        }
        cleanup
    }

    /// Closes the session and returns every reservation/connection for cleanup.
    pub fn close(&mut self) -> AdmissionCleanup {
        self.closed = true;
        self.invitations.clear();
        let peers: Vec<_> = self.peers.keys().copied().collect();
        let mut cleanup = AdmissionCleanup::default();
        for peer in peers {
            cleanup.extend(self.remove_peer(peer));
        }
        cleanup
    }

    /// Identity bound to an offered or admitted connection; not an authorization.
    #[must_use]
    pub fn peer_for_connection(&self, connection: u64) -> Option<PeerId> {
        self.connections.get(&connection).copied()
    }

    /// Whether this physical connection completed its persistence acknowledgement.
    #[must_use]
    pub fn is_connection_admitted(&self, connection: u64) -> bool {
        self.connections
            .get(&connection)
            .and_then(|peer| self.peers.get(peer))
            .is_some_and(|record| record.current.is_some() && record.deadline.is_none())
    }

    /// Number of reserved identities, including initial pending offers.
    #[must_use]
    pub fn reserved_peer_count(&self) -> usize {
        self.peers.len()
    }

    fn ensure_open(&self) -> Result<(), AdmissionFlowError> {
        if self.closed {
            Err(AdmissionFlowError::SessionClosed)
        } else {
            Ok(())
        }
    }

    fn ensure_new_connection(&self, connection: u64) -> Result<(), AdmissionFlowError> {
        self.ensure_open()?;
        if self.connections.contains_key(&connection) {
            return Err(AdmissionFlowError::DuplicateConnection);
        }
        Ok(())
    }

    fn deadline(&self, now: Duration) -> Result<Duration, AdmissionFlowError> {
        now.checked_add(self.limits.pending_timeout)
            .ok_or(AdmissionFlowError::InvalidExpiry)
    }

    fn begin_invite(
        &mut self,
        connection: u64,
        token: InviteToken,
        now: Duration,
    ) -> Result<AdmissionOffer, AdmissionFlowError> {
        let index = self
            .invitations
            .iter()
            .position(|entry| entry.token.matches(token))
            .ok_or(AdmissionFlowError::InvalidInvite)?;
        let invitation = self
            .invitations
            .get(index)
            .ok_or(AdmissionFlowError::InvalidInvite)?;
        if now >= invitation.expires_at {
            return Err(AdmissionFlowError::InvalidInvite);
        }
        if let Some(peer) = invitation.pending_peer {
            return self.offer_existing(peer, connection, now);
        }
        let deadline = self.deadline(now)?.min(invitation.expires_at);
        let offer = self.create_peer(connection, Some(token), deadline)?;
        if let Some(invitation) = self.invitations.get_mut(index) {
            invitation.pending_peer = Some(offer.peer);
        }
        Ok(offer)
    }

    fn begin_reconnect(
        &mut self,
        connection: u64,
        token: ReconnectCredential,
        now: Duration,
    ) -> Result<AdmissionOffer, AdmissionFlowError> {
        let peer = self
            .peers
            .iter()
            .find(|(_, record)| {
                record.current.is_some_and(|current| current.matches(token))
                    || record
                        .successor
                        .is_some_and(|pending| pending.matches(token))
            })
            .map(|(&peer, _)| peer)
            .ok_or(AdmissionFlowError::InvalidReconnect)?;
        self.offer_existing(peer, connection, now)
    }

    fn offer_existing(
        &mut self,
        peer: PeerId,
        connection: u64,
        now: Duration,
    ) -> Result<AdmissionOffer, AdmissionFlowError> {
        let next_deadline = self.deadline(now)?;
        let record = self
            .peers
            .get_mut(&peer)
            .ok_or(AdmissionFlowError::InvalidReconnect)?;
        if record.connection.is_some() {
            return Err(AdmissionFlowError::DuplicateActivePeer);
        }
        if record.current.is_none() && record.deadline.is_some_and(|deadline| now >= deadline) {
            return Err(AdmissionFlowError::AdmissionExpired);
        }
        // Initial offers cannot be kept alive forever by retrying. Established
        // identities may reconnect for this host session's lifetime.
        let deadline = if record.current.is_none() {
            record
                .deadline
                .ok_or(AdmissionFlowError::AdmissionExpired)?
        } else {
            next_deadline
        };
        let successor = *record
            .successor
            .get_or_insert_with(ReconnectCredential::generate);
        record.connection = Some(connection);
        record.deadline = Some(deadline);
        record.last_reconnected = record.current.is_some();
        self.connections.insert(connection, peer);
        Ok(AdmissionOffer {
            peer,
            reconnect_credential: successor,
            reconnected: record.last_reconnected,
            expires_at: deadline,
        })
    }

    fn create_peer(
        &mut self,
        connection: u64,
        invite: Option<InviteToken>,
        deadline: Duration,
    ) -> Result<AdmissionOffer, AdmissionFlowError> {
        if self.peers.len() >= self.limits.max_peers {
            return Err(AdmissionFlowError::PeerLimit);
        }
        let peer = loop {
            let candidate = PeerId::generate();
            if !self.peers.contains_key(&candidate) {
                break candidate;
            }
        };
        let reconnect_credential = ReconnectCredential::generate();
        self.peers.insert(
            peer,
            PeerRecord {
                current: None,
                successor: Some(reconnect_credential),
                connection: Some(connection),
                deadline: Some(deadline),
                initial_invite: invite,
                last_reconnected: false,
            },
        );
        self.connections.insert(connection, peer);
        Ok(AdmissionOffer {
            peer,
            reconnect_credential,
            reconnected: false,
            expires_at: deadline,
        })
    }

    fn remove_peer(&mut self, peer: PeerId) -> AdmissionCleanup {
        let Some(record) = self.peers.remove(&peer) else {
            return AdmissionCleanup::default();
        };
        for invitation in &mut self.invitations {
            if invitation.pending_peer == Some(peer) {
                invitation.pending_peer = None;
            }
        }
        let mut cleanup = AdmissionCleanup {
            released_peers: vec![peer],
            disconnected_connections: Vec::new(),
        };
        if let Some(connection) = record.connection {
            self.connections.remove(&connection);
            cleanup.disconnected_connections.push(connection);
        }
        cleanup
    }
}

impl AdmissionCleanup {
    fn extend(&mut self, other: Self) {
        self.released_peers.extend(other.released_peers);
        self.disconnected_connections
            .extend(other.disconnected_connections);
    }
}

/// Typed non-secret reasons an admission step was rejected.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AdmissionFlowError {
    /// Session was explicitly closed.
    SessionClosed,
    /// Limits were zero, excessive, or had no acknowledgement duration.
    InvalidLimits,
    /// Explicit session identity was invalid.
    InvalidSession,
    /// Deadline was not in the future or overflowed monotonic time.
    InvalidExpiry,
    /// Outstanding invitations reached the configured bound.
    InviteLimit,
    /// Reserved identities reached the configured bound.
    PeerLimit,
    /// Invitation was invalid, expired, revoked, or consumed.
    InvalidInvite,
    /// Reconnect credential was invalid, revoked, or superseded after ACK.
    InvalidReconnect,
    /// Physical connection already has an offer or admission.
    DuplicateConnection,
    /// Identity already has a live physical connection.
    DuplicateActivePeer,
    /// Acknowledgement or initial reservation expired.
    AdmissionExpired,
    /// ACK did not match this connection's offered credential.
    InvalidAcknowledgement,
    /// Physical connection has no outstanding offer or admission.
    UnknownConnection,
}

impl fmt::Display for AdmissionFlowError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(match self {
            Self::SessionClosed => "session is closed",
            Self::InvalidLimits => "admission limits are invalid",
            Self::InvalidSession => "session identity is invalid",
            Self::InvalidExpiry => "admission expiry is invalid",
            Self::InviteLimit => "outstanding invitation limit reached",
            Self::PeerLimit => "reserved identity limit reached",
            Self::InvalidInvite => "invitation is invalid",
            Self::InvalidReconnect => "reconnect credential is invalid",
            Self::DuplicateConnection => "connection already has an admission attempt",
            Self::DuplicateActivePeer => "reserved peer is already connected",
            Self::AdmissionExpired => "admission attempt expired",
            Self::InvalidAcknowledgement => "admission acknowledgement is invalid",
            Self::UnknownConnection => "connection has no admission attempt",
        })
    }
}

impl std::error::Error for AdmissionFlowError {}

#[cfg(test)]
mod tests {
    use super::*;

    fn seconds(value: u64) -> Duration {
        Duration::from_secs(value)
    }

    fn authority() -> SessionAdmissionAuthority {
        SessionAdmissionAuthority::with_session(
            SessionId::from_bytes([1; 16]),
            AdmissionLimits {
                max_peers: 3,
                max_invites: 3,
                pending_timeout: seconds(10),
            },
        )
        .expect("valid limits")
    }

    fn invite(authority: &mut SessionAdmissionAuthority) -> InviteToken {
        authority
            .issue_invite(Duration::ZERO, seconds(100))
            .expect("invitation")
    }

    fn admitted(authority: &mut SessionAdmissionAuthority, connection: u64) -> AdmissionGrant {
        let token = invite(authority);
        let offer = authority
            .begin(
                connection,
                AdmissionCredential::Invite(token),
                Duration::ZERO,
            )
            .expect("initial offer");
        authority
            .acknowledge(connection, offer.reconnect_credential, seconds(1))
            .expect("initial persistence acknowledged")
    }

    #[test]
    fn three_independent_invitations_admit_three_distinct_peers() {
        let mut authority = authority();
        let invitations = [
            invite(&mut authority),
            invite(&mut authority),
            invite(&mut authority),
        ];
        assert_ne!(invitations[0], invitations[1]);
        assert_ne!(invitations[1], invitations[2]);
        assert_eq!(
            authority.issue_invite(Duration::ZERO, seconds(100)),
            Err(AdmissionFlowError::InviteLimit)
        );
        let mut peers = Vec::new();
        for (connection, token) in (10..).zip(invitations) {
            let offer = authority
                .begin(
                    connection,
                    AdmissionCredential::Invite(token),
                    Duration::ZERO,
                )
                .expect("separate invite offers admission");
            assert!(!authority.is_connection_admitted(connection));
            let grant = authority
                .acknowledge(connection, offer.reconnect_credential, seconds(1))
                .expect("persisted offer commits");
            assert!(!grant.reconnected);
            assert!(authority.is_connection_admitted(connection));
            assert_eq!(authority.peer_for_connection(connection), Some(grant.peer));
            assert!(!peers.contains(&grant.peer));
            peers.push(grant.peer);
            assert_eq!(
                authority.begin(
                    connection + 100,
                    AdmissionCredential::Invite(token),
                    seconds(1)
                ),
                Err(AdmissionFlowError::InvalidInvite)
            );
        }
        assert_eq!(authority.reserved_peer_count(), 3);
    }

    #[test]
    fn lost_initial_offer_and_failed_persistence_retry_same_peer_and_credential() {
        let mut authority = authority();
        let token = invite(&mut authority);
        let first = authority
            .begin(10, AdmissionCredential::Invite(token), Duration::ZERO)
            .expect("offer");
        assert_eq!(authority.disconnect(10), Some(first.peer));
        // The first offer was never delivered or persisted: retry the private code.
        let retry = authority
            .begin(11, AdmissionCredential::Invite(token), seconds(2))
            .expect("retry");
        assert_eq!(retry, first);
        authority.disconnect(11);
        // If persistence succeeded but ACK was never sent, stored credentials also work.
        let persisted = authority
            .begin(
                12,
                AdmissionCredential::Reconnect(first.reconnect_credential),
                seconds(3),
            )
            .expect("persisted retry");
        assert_eq!(persisted, first);
        assert_eq!(authority.reserved_peer_count(), 1);
        let grant = authority
            .acknowledge(12, persisted.reconnect_credential, seconds(4))
            .expect("ACK");
        assert_eq!(grant.peer, first.peer);
        assert!(!grant.reconnected);
        assert_eq!(
            authority.begin(13, AdmissionCredential::Invite(token), seconds(4)),
            Err(AdmissionFlowError::InvalidInvite)
        );
    }

    #[test]
    fn duplicate_connections_peers_and_wrong_acks_cannot_steal_admission() {
        let mut authority = authority();
        let first_token = invite(&mut authority);
        let other_token = invite(&mut authority);
        let first = authority
            .begin(10, AdmissionCredential::Invite(first_token), Duration::ZERO)
            .expect("offer");
        assert_eq!(
            authority.begin(10, AdmissionCredential::Invite(other_token), seconds(1)),
            Err(AdmissionFlowError::DuplicateConnection)
        );
        assert_eq!(
            authority.begin_external(10, seconds(1)),
            Err(AdmissionFlowError::DuplicateConnection)
        );
        assert_eq!(
            authority.begin(11, AdmissionCredential::Invite(first_token), seconds(1)),
            Err(AdmissionFlowError::DuplicateActivePeer)
        );
        assert_eq!(
            authority.begin(
                11,
                AdmissionCredential::Reconnect(first.reconnect_credential),
                seconds(1)
            ),
            Err(AdmissionFlowError::DuplicateActivePeer)
        );
        let other = authority
            .begin(11, AdmissionCredential::Invite(other_token), seconds(1))
            .expect("other offer");
        assert_eq!(
            authority.acknowledge(11, first.reconnect_credential, seconds(2)),
            Err(AdmissionFlowError::InvalidAcknowledgement)
        );
        assert_eq!(
            authority.acknowledge(99, first.reconnect_credential, seconds(2)),
            Err(AdmissionFlowError::UnknownConnection)
        );
        assert!(!authority.is_connection_admitted(10));
        assert!(!authority.is_connection_admitted(11));
        assert_eq!(
            authority.acknowledge(10, other.reconnect_credential, seconds(2)),
            Err(AdmissionFlowError::InvalidAcknowledgement)
        );
        authority
            .acknowledge(10, first.reconnect_credential, seconds(2))
            .expect("correct ACK");
        authority
            .acknowledge(11, other.reconnect_credential, seconds(2))
            .expect("other correct ACK");
        assert_eq!(authority.reserved_peer_count(), 2);
    }

    #[test]
    fn reconnect_reuses_successor_until_ack_then_rejects_previous_forever() {
        let mut authority = authority();
        let first = admitted(&mut authority, 10);
        assert_eq!(
            authority.begin(
                11,
                AdmissionCredential::Reconnect(first.reconnect_credential),
                seconds(2)
            ),
            Err(AdmissionFlowError::DuplicateActivePeer)
        );
        authority.disconnect(10);
        let next = authority
            .begin(
                11,
                AdmissionCredential::Reconnect(first.reconnect_credential),
                seconds(2),
            )
            .expect("reconnect offer");
        assert!(next.reconnected);
        assert_eq!(next.peer, first.peer);
        assert_ne!(next.reconnect_credential, first.reconnect_credential);
        assert!(!authority.is_connection_admitted(11));
        authority.disconnect(11);
        let old_retry = authority
            .begin(
                12,
                AdmissionCredential::Reconnect(first.reconnect_credential),
                seconds(3),
            )
            .expect("old credential recovers lost offer");
        assert_eq!(old_retry.reconnect_credential, next.reconnect_credential);
        assert_eq!(old_retry.peer, first.peer);
        authority.disconnect(12);
        let new_retry = authority
            .begin(
                13,
                AdmissionCredential::Reconnect(next.reconnect_credential),
                seconds(4),
            )
            .expect("stored successor recovers lost ACK");
        assert_eq!(new_retry.reconnect_credential, next.reconnect_credential);
        assert_eq!(
            authority.acknowledge(13, first.reconnect_credential, seconds(5)),
            Err(AdmissionFlowError::InvalidAcknowledgement)
        );
        authority
            .acknowledge(13, next.reconnect_credential, seconds(5))
            .expect("commit successor");
        authority.disconnect(13);
        assert_eq!(
            authority.begin(
                14,
                AdmissionCredential::Reconnect(first.reconnect_credential),
                seconds(6)
            ),
            Err(AdmissionFlowError::InvalidReconnect)
        );
        let later = authority
            .begin(
                14,
                AdmissionCredential::Reconnect(next.reconnect_credential),
                seconds(6),
            )
            .expect("next reconnect");
        assert_ne!(later.reconnect_credential, next.reconnect_credential);
        assert_eq!(later.peer, first.peer);
    }

    #[test]
    fn duplicate_ack_is_idempotent_and_lost_ack_response_does_not_strand_client() {
        let mut authority = authority();
        let first = admitted(&mut authority, 10);
        assert_eq!(
            authority.acknowledge(10, first.reconnect_credential, seconds(2)),
            Ok(first)
        );
        // The guest never received the host's confirmation, but already stored first.
        authority.disconnect(10);
        let next = authority
            .begin(
                11,
                AdmissionCredential::Reconnect(first.reconnect_credential),
                seconds(3),
            )
            .expect("stored credential survives lost confirmation");
        assert_eq!(next.peer, first.peer);
        assert!(next.reconnected);
        let grant = authority
            .acknowledge(11, next.reconnect_credential, seconds(4))
            .expect("ACK");
        assert_eq!(
            authority.acknowledge(11, next.reconnect_credential, seconds(5)),
            Ok(grant)
        );
    }

    #[test]
    fn reconnect_timeout_keeps_both_recovery_credentials_but_drops_connection() {
        let mut authority = authority();
        let first = admitted(&mut authority, 10);
        authority.disconnect(10);
        let next = authority
            .begin(
                11,
                AdmissionCredential::Reconnect(first.reconnect_credential),
                seconds(2),
            )
            .expect("offer");
        assert_eq!(
            authority.acknowledge(11, next.reconnect_credential, seconds(12)),
            Err(AdmissionFlowError::AdmissionExpired)
        );
        let cleanup = authority.expire(seconds(12));
        assert!(cleanup.released_peers.is_empty());
        assert_eq!(cleanup.disconnected_connections, vec![11]);
        assert_eq!(authority.reserved_peer_count(), 1);
        assert_eq!(
            authority.acknowledge(11, next.reconnect_credential, seconds(12)),
            Err(AdmissionFlowError::UnknownConnection)
        );
        let from_old = authority
            .begin(
                12,
                AdmissionCredential::Reconnect(first.reconnect_credential),
                seconds(13),
            )
            .expect("recover from pre-persistence loss");
        assert_eq!(from_old.reconnect_credential, next.reconnect_credential);
        authority.disconnect(12);
        authority.expire(seconds(100));
        let from_new = authority
            .begin(
                13,
                AdmissionCredential::Reconnect(next.reconnect_credential),
                seconds(101),
            )
            .expect("recover from post-persistence ACK loss");
        assert_eq!(from_new.reconnect_credential, next.reconnect_credential);
        authority
            .acknowledge(13, from_new.reconnect_credential, seconds(102))
            .expect("ACK");
    }

    #[test]
    fn initial_deadlines_cannot_be_extended_by_retries_and_cleanup_releases_capacity() {
        let mut authority = authority();
        let token = invite(&mut authority);
        let offer = authority
            .begin(10, AdmissionCredential::Invite(token), Duration::ZERO)
            .expect("offer");
        authority.disconnect(10);
        let retry = authority
            .begin(11, AdmissionCredential::Invite(token), seconds(9))
            .expect("last second retry");
        assert_eq!(retry.expires_at, seconds(10));
        assert_eq!(
            authority.acknowledge(11, retry.reconnect_credential, seconds(10)),
            Err(AdmissionFlowError::AdmissionExpired)
        );
        let cleanup = authority.expire(seconds(10));
        assert_eq!(cleanup.released_peers, vec![offer.peer]);
        assert_eq!(cleanup.disconnected_connections, vec![11]);
        assert_eq!(authority.reserved_peer_count(), 0);
        assert_eq!(
            authority.begin(
                12,
                AdmissionCredential::Reconnect(offer.reconnect_credential),
                seconds(11)
            ),
            Err(AdmissionFlowError::InvalidReconnect)
        );
        // Initial expiry frees a still-valid invitation, without revoking it forever.
        assert!(authority
            .begin(12, AdmissionCredential::Invite(token), seconds(11))
            .is_ok());
    }

    #[test]
    fn invitation_expiry_caps_offer_and_revocation_releases_only_its_pending_peer() {
        let mut authority = authority();
        let first = authority
            .issue_invite(Duration::ZERO, seconds(3))
            .expect("short invitation");
        let second = invite(&mut authority);
        let offer = authority
            .begin(10, AdmissionCredential::Invite(first), seconds(1))
            .expect("short offer");
        assert_eq!(offer.expires_at, seconds(3));
        let other = authority
            .begin(11, AdmissionCredential::Invite(second), seconds(1))
            .expect("other offer");
        let cleanup = authority.expire(seconds(3));
        assert_eq!(cleanup.released_peers, vec![offer.peer]);
        assert_eq!(cleanup.disconnected_connections, vec![10]);
        assert_eq!(authority.peer_for_connection(11), Some(other.peer));
        assert_eq!(
            authority.begin(12, AdmissionCredential::Invite(first), seconds(3)),
            Err(AdmissionFlowError::InvalidInvite)
        );
        let revoked = authority.revoke_invite(second);
        assert_eq!(revoked.released_peers, vec![other.peer]);
        assert_eq!(revoked.disconnected_connections, vec![11]);
        assert_eq!(
            authority.begin(12, AdmissionCredential::Invite(second), seconds(3)),
            Err(AdmissionFlowError::InvalidInvite)
        );
        assert_eq!(authority.expire(seconds(100)), AdmissionCleanup::default());
    }

    #[test]
    fn external_pending_peers_are_bounded_and_expire_without_affecting_admitted_peers() {
        let mut authority = authority();
        let established = admitted(&mut authority, 10);
        let first = authority
            .begin_external(11, seconds(2))
            .expect("password succeeded");
        let second = authority
            .begin_external(12, seconds(2))
            .expect("password succeeded");
        assert_eq!(
            authority.begin_external(13, seconds(2)),
            Err(AdmissionFlowError::PeerLimit)
        );
        authority.disconnect(12);
        let cleanup = authority.expire(seconds(12));
        assert_eq!(cleanup.released_peers.len(), 2);
        assert!(cleanup.released_peers.contains(&first.peer));
        assert!(cleanup.released_peers.contains(&second.peer));
        assert_eq!(cleanup.disconnected_connections, vec![11]);
        assert_eq!(authority.reserved_peer_count(), 1);
        assert_eq!(authority.peer_for_connection(10), Some(established.peer));
        assert!(authority.begin_external(13, seconds(13)).is_ok());
    }

    #[test]
    fn capacity_refusal_does_not_consume_invite_and_abort_releases_initial_reservation() {
        let mut authority = authority();
        let grants = [
            admitted(&mut authority, 10),
            admitted(&mut authority, 11),
            admitted(&mut authority, 12),
        ];
        let token = invite(&mut authority);
        assert_eq!(
            authority.begin(13, AdmissionCredential::Invite(token), seconds(2)),
            Err(AdmissionFlowError::PeerLimit)
        );
        let cleanup = authority.revoke_peer(grants[0].peer);
        assert_eq!(cleanup.disconnected_connections, vec![10]);
        let offered = authority
            .begin(13, AdmissionCredential::Invite(token), seconds(2))
            .expect("invite still usable");
        let aborted = authority.abort(13);
        assert_eq!(aborted.released_peers, vec![offered.peer]);
        assert_eq!(aborted.disconnected_connections, vec![13]);
        let retry = authority
            .begin(14, AdmissionCredential::Invite(token), seconds(3))
            .expect("game refusal preserves invite");
        authority
            .acknowledge(14, retry.reconnect_credential, seconds(4))
            .expect("ACK");
    }

    #[test]
    fn aborted_reconnect_retains_persisted_successor_and_peer_revocation_removes_both() {
        let mut authority = authority();
        let first = admitted(&mut authority, 10);
        authority.disconnect(10);
        let next = authority
            .begin(
                11,
                AdmissionCredential::Reconnect(first.reconnect_credential),
                seconds(2),
            )
            .expect("offer");
        let aborted = authority.abort(11);
        assert!(aborted.released_peers.is_empty());
        assert_eq!(aborted.disconnected_connections, vec![11]);
        let retry = authority
            .begin(
                12,
                AdmissionCredential::Reconnect(next.reconnect_credential),
                seconds(3),
            )
            .expect("stored successor survives abort");
        assert_eq!(retry.peer, first.peer);
        assert_eq!(retry.reconnect_credential, next.reconnect_credential);
        let revoked = authority.revoke_peer(first.peer);
        assert_eq!(revoked.released_peers, vec![first.peer]);
        assert_eq!(revoked.disconnected_connections, vec![12]);
        for token in [first.reconnect_credential, next.reconnect_credential] {
            assert_eq!(
                authority.begin(13, AdmissionCredential::Reconnect(token), seconds(4)),
                Err(AdmissionFlowError::InvalidReconnect)
            );
        }
    }

    #[test]
    fn close_invalidates_every_invite_credential_and_pending_offer() {
        let mut authority = authority();
        let established = admitted(&mut authority, 10);
        let token = invite(&mut authority);
        let pending = authority
            .begin(11, AdmissionCredential::Invite(token), seconds(2))
            .expect("offer");
        let cleanup = authority.close();
        assert_eq!(cleanup.released_peers.len(), 2);
        assert!(cleanup.released_peers.contains(&established.peer));
        assert!(cleanup.released_peers.contains(&pending.peer));
        assert_eq!(cleanup.disconnected_connections.len(), 2);
        assert_eq!(authority.reserved_peer_count(), 0);
        assert_eq!(
            authority.begin(12, AdmissionCredential::Invite(token), seconds(3)),
            Err(AdmissionFlowError::SessionClosed)
        );
        assert_eq!(
            authority.begin(
                12,
                AdmissionCredential::Reconnect(established.reconnect_credential),
                seconds(3)
            ),
            Err(AdmissionFlowError::SessionClosed)
        );
        assert_eq!(
            authority.begin_external(12, seconds(3)),
            Err(AdmissionFlowError::SessionClosed)
        );
        assert_eq!(
            authority.acknowledge(11, pending.reconnect_credential, seconds(3)),
            Err(AdmissionFlowError::SessionClosed)
        );
        assert_eq!(
            authority.issue_invite(seconds(3), seconds(100)),
            Err(AdmissionFlowError::SessionClosed)
        );
        assert_eq!(authority.close(), AdmissionCleanup::default());
    }

    #[test]
    fn malformed_limits_ids_deadlines_and_diagnostics_are_safe() {
        let limits = AdmissionLimits::default();
        assert!(matches!(
            SessionAdmissionAuthority::with_session(SessionId::from_bytes([0; 16]), limits),
            Err(AdmissionFlowError::InvalidSession)
        ));
        for invalid in [
            AdmissionLimits {
                max_peers: 0,
                ..limits
            },
            AdmissionLimits {
                max_invites: 4097,
                ..limits
            },
            AdmissionLimits {
                pending_timeout: Duration::ZERO,
                ..limits
            },
        ] {
            assert!(matches!(
                SessionAdmissionAuthority::new(invalid),
                Err(AdmissionFlowError::InvalidLimits)
            ));
        }
        let mut authority = authority();
        assert_eq!(
            authority.issue_invite(seconds(2), seconds(2)),
            Err(AdmissionFlowError::InvalidExpiry)
        );
        assert_eq!(
            authority.begin_external(10, Duration::MAX),
            Err(AdmissionFlowError::InvalidExpiry)
        );
        let token = invite(&mut authority);
        let offer = authority
            .begin(10, AdmissionCredential::Invite(token), seconds(1))
            .expect("offer");
        let debug = format!("{authority:?} {offer:?}");
        assert!(debug.contains("InviteToken([REDACTED])"));
        assert!(debug.contains("ReconnectCredential([REDACTED])"));
        assert!(!debug.contains(&format!("{:?}", token.to_bytes())));
        assert!(!debug.contains(&format!("{:?}", offer.reconnect_credential.to_bytes())));
    }
}
