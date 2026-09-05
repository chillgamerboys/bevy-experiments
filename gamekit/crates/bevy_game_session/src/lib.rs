//! Engine-independent session identity, admission, and public endpoint descriptions.
//!
//! No sockets, Bevy schedules, discovery providers, seats, or game rules live here.
//! Transport adapters authenticate these contracts; games decide what admission grants.

mod connection_code;
mod password;
mod security;

pub use connection_code::{
    CertificateFingerprint, ConnectionCodeError, DirectConnectionCode, DirectEndpoint,
    EncodedConnectionCode,
};
pub use password::{
    PasswordAttemptError, PasswordPolicyError, SessionPassword, SessionPasswordVerifier,
};
pub use security::{
    AdmissionCredential, AdmissionError, AdmissionGrant, InviteToken, PeerId, ReconnectCredential,
    SessionId, SessionSecurityAuthority,
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
