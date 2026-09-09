//! Transport-independent bounded, versioned direct connection codes.

use std::{fmt, net::IpAddr};

use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine as _};
use serde::{Deserialize, Serialize};

use crate::{InviteToken, SessionId};

const PREFIX: &str = "BGN1.";
const MAX_HOST_BYTES: usize = 253;
const MAX_CODE_BYTES: usize = 768;

/// SHA-256 digest of a host certificate's DER-encoded SubjectPublicKeyInfo.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct CertificateFingerprint([u8; Self::BYTE_LENGTH]);

impl CertificateFingerprint {
    /// SHA-256 digest length.
    pub const BYTE_LENGTH: usize = 32;

    /// Constructs a fingerprint from exact SHA-256 bytes.
    #[must_use]
    pub const fn from_bytes(bytes: [u8; Self::BYTE_LENGTH]) -> Self {
        Self(bytes)
    }

    /// Returns exact digest bytes.
    #[must_use]
    pub const fn to_bytes(self) -> [u8; Self::BYTE_LENGTH] {
        self.0
    }
}

impl fmt::Debug for CertificateFingerprint {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("CertificateFingerprint(")?;
        for byte in self.0.iter().take(4) {
            write!(formatter, "{byte:02x}")?;
        }
        formatter.write_str("…)")
    }
}

/// Advertised DNS name/IP and UDP port for a direct host.
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize)]
pub struct DirectEndpoint {
    host: String,
    port: u16,
}

impl<'de> Deserialize<'de> for DirectEndpoint {
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        #[derive(Deserialize)]
        #[serde(deny_unknown_fields)]
        struct Wire {
            host: String,
            port: u16,
        }
        let wire = Wire::deserialize(deserializer)?;
        Self::new(wire.host, wire.port).map_err(serde::de::Error::custom)
    }
}

impl DirectEndpoint {
    /// Validates a bounded ASCII DNS name or textual IP literal and non-zero port.
    pub fn new(host: impl Into<String>, port: u16) -> Result<Self, ConnectionCodeError> {
        let host = host.into();
        if host.is_empty() || host.len() > MAX_HOST_BYTES {
            return Err(ConnectionCodeError::InvalidHostLength);
        }
        if !valid_host(&host) {
            return Err(ConnectionCodeError::InvalidHostSyntax);
        }
        if port == 0 {
            return Err(ConnectionCodeError::InvalidPort);
        }
        Ok(Self { host, port })
    }

    /// Advertised DNS name or textual IP literal.
    #[must_use]
    pub fn host(&self) -> &str {
        &self.host
    }

    /// Advertised UDP port.
    #[must_use]
    pub const fn port(&self) -> u16 {
        self.port
    }
}

fn valid_host(host: &str) -> bool {
    if !host.is_ascii() {
        return false;
    }
    let unwrapped = host
        .strip_prefix('[')
        .and_then(|value| value.strip_suffix(']'))
        .unwrap_or(host);
    if unwrapped.parse::<IpAddr>().is_ok() {
        return true;
    }
    if unwrapped != host {
        return false;
    }
    host.split('.').all(|label| {
        !label.is_empty()
            && label.len() <= 63
            && label
                .as_bytes()
                .first()
                .is_some_and(u8::is_ascii_alphanumeric)
            && label
                .as_bytes()
                .last()
                .is_some_and(u8::is_ascii_alphanumeric)
            && label
                .bytes()
                .all(|byte| byte.is_ascii_alphanumeric() || byte == b'-')
    })
}

/// Decoded secure direct-connect information.
#[derive(Clone, PartialEq, Eq)]
pub struct DirectConnectionCode {
    /// Host-process session identity.
    pub session_id: SessionId,
    /// Advertised host endpoint.
    pub endpoint: DirectEndpoint,
    /// Pinned certificate digest.
    pub certificate_fingerprint: CertificateFingerprint,
    /// Exact certificate expiry as Unix seconds.
    pub certificate_expires_unix_seconds: u64,
    /// One-time lobby admission secret.
    pub invite_token: InviteToken,
}

impl DirectConnectionCode {
    /// Encodes this payload as `BGN1.<base64url>` without padding.
    #[must_use]
    pub fn encode(&self) -> EncodedConnectionCode {
        let host = self.endpoint.host().as_bytes();
        let mut payload = Vec::with_capacity(
            SessionId::BYTE_LENGTH
                + 2
                + host.len()
                + 2
                + CertificateFingerprint::BYTE_LENGTH
                + 8
                + InviteToken::BYTE_LENGTH,
        );
        payload.extend_from_slice(&self.session_id.to_bytes());
        payload.extend_from_slice(&u16::try_from(host.len()).unwrap_or(u16::MAX).to_be_bytes());
        payload.extend_from_slice(host);
        payload.extend_from_slice(&self.endpoint.port().to_be_bytes());
        payload.extend_from_slice(&self.certificate_fingerprint.to_bytes());
        payload.extend_from_slice(&self.certificate_expires_unix_seconds.to_be_bytes());
        payload.extend_from_slice(&self.invite_token.to_bytes());
        EncodedConnectionCode(format!("{PREFIX}{}", URL_SAFE_NO_PAD.encode(payload)))
    }

    /// Parses and validates a version-one code without opening a socket.
    pub fn parse(encoded: &str) -> Result<Self, ConnectionCodeError> {
        if encoded.len() > MAX_CODE_BYTES {
            return Err(ConnectionCodeError::CodeTooLong);
        }
        let body = encoded
            .strip_prefix(PREFIX)
            .ok_or(ConnectionCodeError::WrongVersion)?;
        let decoded = URL_SAFE_NO_PAD
            .decode(body)
            .map_err(|_error| ConnectionCodeError::InvalidBase64)?;
        let mut remaining = decoded.as_slice();
        let session_id = SessionId::from_bytes(take::<16>(&mut remaining)?);
        if !session_id.is_valid() {
            return Err(ConnectionCodeError::InvalidSession);
        }
        let host_length = usize::from(u16::from_be_bytes(take::<2>(&mut remaining)?));
        if host_length == 0 || host_length > MAX_HOST_BYTES {
            return Err(ConnectionCodeError::InvalidHostLength);
        }
        let (host, tail) = remaining
            .split_at_checked(host_length)
            .ok_or(ConnectionCodeError::Truncated)?;
        remaining = tail;
        let host = std::str::from_utf8(host).map_err(|_error| ConnectionCodeError::InvalidUtf8)?;
        let port = u16::from_be_bytes(take::<2>(&mut remaining)?);
        let certificate_fingerprint =
            CertificateFingerprint::from_bytes(take::<32>(&mut remaining)?);
        let certificate_expires_unix_seconds = u64::from_be_bytes(take::<8>(&mut remaining)?);
        if certificate_expires_unix_seconds == 0 {
            return Err(ConnectionCodeError::InvalidCertificateExpiry);
        }
        let invite_token = InviteToken::from_bytes(take::<16>(&mut remaining)?);
        if !remaining.is_empty() {
            return Err(ConnectionCodeError::TrailingData);
        }
        Ok(Self {
            session_id,
            endpoint: DirectEndpoint::new(host, port)?,
            certificate_fingerprint,
            certificate_expires_unix_seconds,
            invite_token,
        })
    }
}

impl fmt::Debug for DirectConnectionCode {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("DirectConnectionCode")
            .field("session_id", &self.session_id)
            .field("endpoint", &self.endpoint)
            .field("certificate_fingerprint", &self.certificate_fingerprint)
            .field(
                "certificate_expires_unix_seconds",
                &self.certificate_expires_unix_seconds,
            )
            .field("invite_token", &self.invite_token)
            .finish()
    }
}

/// Encoded connection code whose ordinary formatting is always redacted.
#[derive(Clone, PartialEq, Eq)]
pub struct EncodedConnectionCode(String);

impl Drop for EncodedConnectionCode {
    fn drop(&mut self) {
        zeroize::Zeroize::zeroize(&mut self.0);
    }
}

impl EncodedConnectionCode {
    /// Borrows the complete code only for explicit copy/share UI.
    #[must_use]
    pub fn expose_for_sharing(&self) -> &str {
        &self.0
    }
}

impl fmt::Debug for EncodedConnectionCode {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("EncodedConnectionCode([REDACTED])")
    }
}

/// Why a direct connection code was rejected before network activity.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConnectionCodeError {
    /// Unsupported prefix/version.
    WrongVersion,
    /// Input exceeded the defensive cap.
    CodeTooLong,
    /// Payload was not unpadded URL-safe Base64.
    InvalidBase64,
    /// Payload ended before a required field.
    Truncated,
    /// Session identifier was all zeroes.
    InvalidSession,
    /// Host length was empty or too large.
    InvalidHostLength,
    /// Host bytes were not UTF-8.
    InvalidUtf8,
    /// Host syntax was not a DNS name or IP literal.
    InvalidHostSyntax,
    /// UDP port was zero.
    InvalidPort,
    /// Certificate expiry was zero.
    InvalidCertificateExpiry,
    /// Payload contained trailing bytes.
    TrailingData,
}

impl fmt::Display for ConnectionCodeError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(match self {
            Self::WrongVersion => "unsupported connection-code version",
            Self::CodeTooLong => "connection code is too long",
            Self::InvalidBase64 => "connection code is not valid base64url",
            Self::Truncated => "connection code is truncated",
            Self::InvalidSession => "connection code has an invalid session",
            Self::InvalidHostLength => "connection code has an invalid host length",
            Self::InvalidUtf8 => "connection code host is not UTF-8",
            Self::InvalidHostSyntax => "connection code host is invalid",
            Self::InvalidPort => "connection code port is invalid",
            Self::InvalidCertificateExpiry => "connection code certificate expiry is invalid",
            Self::TrailingData => "connection code contains trailing data",
        })
    }
}

impl std::error::Error for ConnectionCodeError {}

fn take<const LENGTH: usize>(bytes: &mut &[u8]) -> Result<[u8; LENGTH], ConnectionCodeError> {
    let (head, tail) = bytes
        .split_at_checked(LENGTH)
        .ok_or(ConnectionCodeError::Truncated)?;
    *bytes = tail;
    head.try_into()
        .map_err(|_error| ConnectionCodeError::Truncated)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn code() -> DirectConnectionCode {
        DirectConnectionCode {
            session_id: SessionId::from_bytes([1; 16]),
            endpoint: DirectEndpoint::new("host.local", 7777).expect("valid endpoint"),
            certificate_fingerprint: CertificateFingerprint::from_bytes([2; 32]),
            certificate_expires_unix_seconds: 2_000_000_000,
            invite_token: InviteToken::from_bytes([3; 16]),
        }
    }

    #[test]
    fn deterministic_round_trip_is_redacted() {
        let code = code();
        let encoded = code.encode();
        assert!(encoded.expose_for_sharing().starts_with("BGN1."));
        assert_eq!(
            DirectConnectionCode::parse(encoded.expose_for_sharing()),
            Ok(code)
        );
        assert_eq!(format!("{encoded:?}"), "EncodedConnectionCode([REDACTED])");
    }

    #[test]
    fn endpoint_accepts_dns_ipv4_and_ipv6() {
        assert!(DirectEndpoint::new("game.local", 7777).is_ok());
        assert!(DirectEndpoint::new("192.168.1.4", 7777).is_ok());
        assert!(DirectEndpoint::new("fd7a:115c:a1e0::1", 7777).is_ok());
        assert!(DirectEndpoint::new("[fd7a:115c:a1e0::1]", 7777).is_ok());
        assert!(DirectEndpoint::new("bad host", 7777).is_err());
        assert!(DirectEndpoint::new("host", 0).is_err());
    }

    #[test]
    fn malformed_inputs_fail_without_panicking() {
        for input in ["", "BGN2.aaaa", "BGN1.%", "BGN1.AA"] {
            assert!(DirectConnectionCode::parse(input).is_err());
        }
        let too_long = format!("BGN1.{}", "A".repeat(MAX_CODE_BYTES));
        assert_eq!(
            DirectConnectionCode::parse(&too_long),
            Err(ConnectionCodeError::CodeTooLong)
        );
    }
}
