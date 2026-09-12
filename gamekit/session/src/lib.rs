//! Engine-independent session identity, admission, and public endpoint descriptions.
//!
//! No sockets, Bevy schedules, discovery providers, seats, or game rules live here.
//! Transport adapters authenticate these contracts; games decide what admission grants.
//!
//! [`SessionAdmissionAuthority`] supports bounded independent one-use invitations
//! and a recoverable `offer -> persist -> acknowledge` handshake. It retains only
//! a current and at most one pending reconnect credential per reserved identity.
//! Games must drive expiry, apply returned cleanup, and authorize gameplay only
//! after acknowledgement. There is no immediate-admission credential rotation API.

mod admission;
mod connection_code;
mod password;
mod security;

pub use admission::{
    AdmissionCleanup, AdmissionFlowError, AdmissionLimits, AdmissionOffer,
    SessionAdmissionAuthority,
};
pub use connection_code::{
    CertificateFingerprint, ConnectionCodeError, DirectConnectionCode, DirectEndpoint,
    EncodedConnectionCode,
};
pub use password::{
    PasswordAttemptError, PasswordPolicyError, SessionPassword, SessionPasswordVerifier,
};
pub use security::{
    AdmissionCredential, AdmissionGrant, InviteToken, PeerId, ReconnectCredential, SessionId,
};

/// Secret-free direct route advertised by a provider, not an open transport.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DiscoveredDirectTarget {
    /// Host-process identity, shared by all routes to the session.
    pub session_id: SessionId,
    /// Provider-specific address.
    pub endpoint: DirectEndpoint,
    /// Certificate pin to verify before sending admission credentials.
    pub certificate_fingerprint: CertificateFingerprint,
    /// Exact advertised certificate expiry.
    pub certificate_expires_unix_seconds: u64,
}
