//! Fixed-size identities, credentials, and host-owned credential rotation.

use std::{
    collections::BTreeMap,
    fmt,
    hash::{Hash, Hasher},
};

use rand::{rngs::OsRng, RngCore as _};
use serde::{Deserialize, Serialize};

macro_rules! public_id {
    ($name:ident, $length:expr, $doc:literal) => {
        #[doc = $doc]
        #[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
        pub struct $name([u8; $length]);

        impl $name {
            /// Encoded byte length.
            pub const BYTE_LENGTH: usize = $length;

            /// Generates a non-zero identifier from the operating system RNG.
            #[must_use]
            pub fn generate() -> Self {
                loop {
                    let mut bytes = [0_u8; $length];
                    OsRng.fill_bytes(&mut bytes);
                    if bytes.iter().any(|byte| *byte != 0) {
                        return Self(bytes);
                    }
                }
            }

            /// Constructs an identifier from exact bytes.
            #[must_use]
            pub const fn from_bytes(bytes: [u8; $length]) -> Self {
                Self(bytes)
            }

            /// Returns exact encoded bytes.
            #[must_use]
            pub const fn to_bytes(self) -> [u8; $length] {
                self.0
            }

            /// Whether this identifier is non-zero.
            #[must_use]
            pub fn is_valid(self) -> bool {
                self.0.iter().any(|byte| *byte != 0)
            }
        }

        impl fmt::Debug for $name {
            fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
                write!(formatter, concat!(stringify!($name), "("))?;
                for byte in self.0.iter().take(4) {
                    write!(formatter, "{byte:02x}")?;
                }
                formatter.write_str("…)")
            }
        }
    };
}

public_id!(
    SessionId,
    16,
    "Stable identity of one host-process session."
);
public_id!(PeerId, 16, "Stable identity assigned to one admitted peer.");

macro_rules! secret {
    ($name:ident, $length:expr, $doc:literal) => {
        #[doc = $doc]
        #[derive(Clone, Copy, Eq, Serialize, Deserialize)]
        pub struct $name([u8; $length]);

        impl $name {
            /// Encoded byte length.
            pub const BYTE_LENGTH: usize = $length;

            /// Generates a credential from the operating system RNG.
            #[must_use]
            pub fn generate() -> Self {
                let mut bytes = [0_u8; $length];
                OsRng.fill_bytes(&mut bytes);
                Self(bytes)
            }

            /// Constructs a credential from exact bytes for decoding and tests.
            #[must_use]
            pub const fn from_bytes(bytes: [u8; $length]) -> Self {
                Self(bytes)
            }

            /// Copies credential bytes for encrypted encoding or protected storage.
            #[must_use]
            pub const fn to_bytes(self) -> [u8; $length] {
                self.0
            }

            /// Compares credentials without a data-dependent early return.
            #[must_use]
            pub fn matches(self, presented: Self) -> bool {
                constant_time_equal(&self.0, &presented.0)
            }
        }

        impl fmt::Debug for $name {
            fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
                formatter.write_str(concat!(stringify!($name), "([REDACTED])"))
            }
        }

        impl PartialEq for $name {
            fn eq(&self, other: &Self) -> bool {
                constant_time_equal(&self.0, &other.0)
            }
        }

        impl Hash for $name {
            fn hash<H: Hasher>(&self, state: &mut H) {
                self.0.hash(state);
            }
        }
    };
}

secret!(
    InviteToken,
    16,
    "One-time bearer secret carried by a direct code."
);
secret!(
    ReconnectCredential,
    32,
    "Private credential rotated after every successful reconnection."
);

fn constant_time_equal(left: &[u8], right: &[u8]) -> bool {
    if left.len() != right.len() {
        return false;
    }
    let mut difference = 0_u8;
    for (&left_byte, &right_byte) in left.iter().zip(right) {
        difference |= left_byte ^ right_byte;
    }
    difference == 0
}

/// Credential presented by an encrypted connection during shared authentication.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AdmissionCredential {
    /// First admission through a private direct invitation.
    Invite(InviteToken),
    /// Reclaim a previously admitted peer identity.
    Reconnect(ReconnectCredential),
}

/// Successful shared authentication result; games still decide seats and capacity.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AdmissionGrant {
    /// Stable identity games can bind to their own seat model.
    pub peer: PeerId,
    /// Fresh credential the client must atomically replace before future reconnects.
    pub reconnect_credential: ReconnectCredential,
    /// Whether this reclaimed an existing identity.
    pub reconnected: bool,
}

#[derive(Debug, Clone, Copy)]
struct PeerSecurity {
    reconnect_credential: ReconnectCredential,
    active_connection: Option<u64>,
}

/// Pure session-security authority independent of any game lobby or seat type.
#[derive(Debug)]
pub struct SessionSecurityAuthority {
    session: SessionId,
    invite: InviteToken,
    invite_consumed: bool,
    peers: BTreeMap<PeerId, PeerSecurity>,
    connections: BTreeMap<u64, PeerId>,
    revoked: Vec<ReconnectCredential>,
    closed: bool,
}

impl SessionSecurityAuthority {
    /// Creates a fresh session, identifier, and one-time direct invitation.
    #[must_use]
    pub fn new() -> Self {
        Self::with_secrets(SessionId::generate(), InviteToken::generate())
    }

    /// Creates deterministic initial state for tests and replay fixtures.
    #[must_use]
    pub fn with_secrets(session: SessionId, invite: InviteToken) -> Self {
        Self {
            session,
            invite,
            invite_consumed: false,
            peers: BTreeMap::new(),
            connections: BTreeMap::new(),
            revoked: Vec::new(),
            closed: false,
        }
    }

    /// Stable identity of this host-process session.
    #[must_use]
    pub const fn session_id(&self) -> SessionId {
        self.session
    }

    /// One-time token for the current private connection code.
    #[must_use]
    pub const fn invite_token(&self) -> InviteToken {
        self.invite
    }

    /// Authenticates one physical connection without assigning a game seat.
    pub fn authenticate(
        &mut self,
        connection: u64,
        credential: AdmissionCredential,
    ) -> Result<AdmissionGrant, AdmissionError> {
        if self.closed {
            return Err(AdmissionError::SessionClosed);
        }
        if self.connections.contains_key(&connection) {
            return Err(AdmissionError::DuplicateConnection);
        }
        match credential {
            AdmissionCredential::Invite(token) => self.authenticate_invite(connection, token),
            AdmissionCredential::Reconnect(credential) => {
                self.authenticate_reconnect(connection, credential)
            }
        }
    }

    /// Issues shared peer security after an external provider has authenticated admission.
    ///
    /// Call this only after a discovery password or future platform provider succeeds.
    /// Game capacity and seat assignment still occur after this method returns.
    pub fn authenticate_external(
        &mut self,
        connection: u64,
    ) -> Result<AdmissionGrant, AdmissionError> {
        if self.closed {
            return Err(AdmissionError::SessionClosed);
        }
        if self.connections.contains_key(&connection) {
            return Err(AdmissionError::DuplicateConnection);
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
            PeerSecurity {
                reconnect_credential,
                active_connection: Some(connection),
            },
        );
        self.connections.insert(connection, peer);
        Ok(AdmissionGrant {
            peer,
            reconnect_credential,
            reconnected: false,
        })
    }

    /// Marks a physical connection absent while preserving its reconnect reservation.
    pub fn disconnect(&mut self, connection: u64) -> Option<PeerId> {
        let peer = self.connections.remove(&connection)?;
        if let Some(record) = self.peers.get_mut(&peer) {
            record.active_connection = None;
        }
        Some(peer)
    }

    /// Revokes one reserved peer and returns its active connection, if any.
    pub fn revoke_peer(&mut self, peer: PeerId) -> Option<u64> {
        let record = self.peers.remove(&peer)?;
        self.revoked.push(record.reconnect_credential);
        if let Some(connection) = record.active_connection {
            self.connections.remove(&connection);
        }
        record.active_connection
    }

    /// Invalidates all admission and reconnect material.
    pub fn close(&mut self) {
        self.closed = true;
        self.connections.clear();
        for record in self.peers.values() {
            self.revoked.push(record.reconnect_credential);
        }
        self.peers.clear();
        self.invite = InviteToken::generate();
        self.invite_consumed = true;
    }

    /// Returns the stable peer bound to an authenticated physical connection.
    #[must_use]
    pub fn peer_for_connection(&self, connection: u64) -> Option<PeerId> {
        self.connections.get(&connection).copied()
    }

    fn authenticate_invite(
        &mut self,
        connection: u64,
        presented: InviteToken,
    ) -> Result<AdmissionGrant, AdmissionError> {
        if self.invite_consumed || !self.invite.matches(presented) {
            return Err(AdmissionError::InvalidInvite);
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
            PeerSecurity {
                reconnect_credential,
                active_connection: Some(connection),
            },
        );
        self.connections.insert(connection, peer);
        self.invite_consumed = true;
        Ok(AdmissionGrant {
            peer,
            reconnect_credential,
            reconnected: false,
        })
    }

    fn authenticate_reconnect(
        &mut self,
        connection: u64,
        presented: ReconnectCredential,
    ) -> Result<AdmissionGrant, AdmissionError> {
        if self.revoked.contains(&presented) {
            return Err(AdmissionError::InvalidReconnect);
        }
        let Some((&peer, record)) = self
            .peers
            .iter()
            .find(|(_, record)| record.reconnect_credential.matches(presented))
        else {
            return Err(AdmissionError::InvalidReconnect);
        };
        if record.active_connection.is_some() {
            return Err(AdmissionError::DuplicateActivePeer);
        }
        let previous = record.reconnect_credential;
        let rotated = ReconnectCredential::generate();
        if let Some(record) = self.peers.get_mut(&peer) {
            record.reconnect_credential = rotated;
            record.active_connection = Some(connection);
        }
        self.revoked.push(previous);
        self.connections.insert(connection, peer);
        Ok(AdmissionGrant {
            peer,
            reconnect_credential: rotated,
            reconnected: true,
        })
    }
}

impl Default for SessionSecurityAuthority {
    fn default() -> Self {
        Self::new()
    }
}

/// Why shared session authentication failed before game admission.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AdmissionError {
    /// The host has closed the session.
    SessionClosed,
    /// This transport connection already authenticated.
    DuplicateConnection,
    /// The one-time direct invitation was invalid or already consumed.
    InvalidInvite,
    /// The reconnect credential was invalid, old, or revoked.
    InvalidReconnect,
    /// The reserved peer already has an active connection.
    DuplicateActivePeer,
}

impl fmt::Display for AdmissionError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(match self {
            Self::SessionClosed => "session is closed",
            Self::DuplicateConnection => "connection already authenticated",
            Self::InvalidInvite => "invitation is invalid",
            Self::InvalidReconnect => "reconnect credential is invalid",
            Self::DuplicateActivePeer => "reserved peer is already connected",
        })
    }
}

impl std::error::Error for AdmissionError {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn invite_is_one_time_and_reconnect_rotates() {
        let invite = InviteToken::from_bytes([2; 16]);
        let mut authority =
            SessionSecurityAuthority::with_secrets(SessionId::from_bytes([1; 16]), invite);
        let first = authority
            .authenticate(10, AdmissionCredential::Invite(invite))
            .expect("first invite authenticates");
        assert_eq!(
            authority.authenticate(11, AdmissionCredential::Invite(invite)),
            Err(AdmissionError::InvalidInvite)
        );
        assert_eq!(authority.disconnect(10), Some(first.peer));
        let second = authority
            .authenticate(
                12,
                AdmissionCredential::Reconnect(first.reconnect_credential),
            )
            .expect("disconnected peer reconnects");
        assert!(second.reconnected);
        assert_eq!(second.peer, first.peer);
        assert_ne!(second.reconnect_credential, first.reconnect_credential);
        assert_eq!(authority.disconnect(12), Some(first.peer));
        assert_eq!(
            authority.authenticate(
                13,
                AdmissionCredential::Reconnect(first.reconnect_credential)
            ),
            Err(AdmissionError::InvalidReconnect)
        );
    }

    #[test]
    fn credentials_and_debug_are_redacted() {
        let invite = InviteToken::from_bytes([7; 16]);
        let reconnect = ReconnectCredential::from_bytes([9; 32]);
        assert_eq!(format!("{invite:?}"), "InviteToken([REDACTED])");
        assert_eq!(format!("{reconnect:?}"), "ReconnectCredential([REDACTED])");
    }
}
