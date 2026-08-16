//! Explicit native WebTransport preparation with short-lived SPKI pinning.

use std::{fmt, sync::Arc, time::Duration};

use aeronet_replicon::{client::AeronetRepliconClient, server::AeronetRepliconServer};
use aeronet_webtransport::{
    client::{ClientConfig, WebTransportClient},
    server::{ServerConfig, SessionRequest, SessionResponse, WebTransportServer},
    wtransport::{
        self,
        tls::{self, rustls},
    },
};
use bevy::{
    ecs::system::EntityCommand as _,
    prelude::{Entity, On, World},
};
use rustls::{
    client::danger::{HandshakeSignatureValid, ServerCertVerified, ServerCertVerifier},
    crypto::WebPkiSupportedAlgorithms,
    pki_types::{CertificateDer, ServerName, UnixTime},
};
use sha2::{Digest as _, Sha256};
use x509_parser::{
    oid_registry::{OID_EC_P256, OID_KEY_TYPE_EC_PUBLIC_KEY},
    prelude::{FromDer as _, X509Certificate},
};

use crate::{
    CertificateFingerprint, DirectConnectionCode, DirectEndpoint, InviteToken,
    ReconnectEndpointBinding, SessionId,
};

/// Default editable direct-game UDP port.
pub const DEFAULT_DIRECT_PORT: u16 = 7777;
/// Fixed application path accepted by Gamekit direct hosts.
pub const DIRECT_SESSION_PATH: &str = "/bgn1";
const KEEP_ALIVE: Duration = Duration::from_secs(1);
const IDLE_TIMEOUT: Duration = Duration::from_secs(5);
const MAX_CERTIFICATE_LIFETIME_SECS: i64 = 14 * 24 * 60 * 60;

/// Prepared direct-listen endpoint and redacted share code.
///
/// Construction creates a fresh P-256 identity. Calling [`Self::open`] is the
/// explicit action that binds the UDP socket.
pub struct PreparedDirectHost {
    config: ServerConfig,
    connection_code: DirectConnectionCode,
}

impl PreparedDirectHost {
    /// Generates one per-session certificate and server configuration.
    pub fn new(
        advertised_endpoint: DirectEndpoint,
        session_id: SessionId,
        invite_token: InviteToken,
    ) -> Result<Self, DirectTransportError> {
        let (identity, fingerprint, certificate_expires_unix_seconds) =
            generate_identity(&advertised_endpoint)?;
        let config = ServerConfig::builder()
            .with_bind_default(advertised_endpoint.port())
            .with_identity(identity)
            .keep_alive_interval(Some(KEEP_ALIVE))
            .max_idle_timeout(Some(IDLE_TIMEOUT))
            .map_err(|_error| DirectTransportError::InvalidIdleTimeout)?
            .build();
        Ok(Self {
            config,
            connection_code: DirectConnectionCode {
                session_id,
                endpoint: advertised_endpoint,
                certificate_fingerprint: fingerprint,
                certificate_expires_unix_seconds,
                invite_token,
            },
        })
    }

    /// Connection code shown only by explicit copy/share UI.
    #[must_use]
    pub const fn connection_code(&self) -> &DirectConnectionCode {
        &self.connection_code
    }

    /// Opens the prepared server and marks it as a Replicon backend.
    pub fn open(self, world: &mut World) -> Entity {
        let server = world.spawn(AeronetRepliconServer).id();
        WebTransportServer::open(self.config).apply(world.entity_mut(server));
        server
    }
}

impl fmt::Debug for PreparedDirectHost {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("PreparedDirectHost")
            .field("connection_code", &self.connection_code)
            .field("config", &"[WEBTRANSPORT SERVER CONFIG]")
            .finish()
    }
}

/// Prepared pinned direct client connection for first admission.
pub struct PreparedDirectJoin {
    config: ClientConfig,
    target: String,
    invite_token: InviteToken,
    reconnect_binding: ReconnectEndpointBinding,
}

/// Secret-free route resolved by a discovery provider.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DiscoveredDirectTarget {
    /// Discovered host-process session identity.
    pub session_id: SessionId,
    /// Resolved endpoint for this provider route.
    pub endpoint: DirectEndpoint,
    /// Certificate pin announced by discovery.
    pub certificate_fingerprint: CertificateFingerprint,
    /// Exact certificate expiry announced by discovery.
    pub certificate_expires_unix_seconds: u64,
}

/// Prepared pinned connection whose admission will use a discovery password.
pub struct PreparedDirectDiscoveryJoin {
    config: ClientConfig,
    target: String,
    reconnect_binding: ReconnectEndpointBinding,
}

impl PreparedDirectDiscoveryJoin {
    /// Configures mandatory SPKI validation without a bearer invitation.
    pub fn new(target: &DiscoveredDirectTarget) -> Result<Self, DirectTransportError> {
        let reconnect_binding = ReconnectEndpointBinding::new(
            target.endpoint.clone(),
            target.certificate_fingerprint,
            target.certificate_expires_unix_seconds,
        )
        .map_err(|_error| DirectTransportError::InvalidCertificateIdentity)?;
        let config = pinned_client_config(
            target.certificate_fingerprint,
            target.certificate_expires_unix_seconds,
        )?;
        Ok(Self {
            config,
            target: direct_target(&target.endpoint),
            reconnect_binding,
        })
    }

    /// Verified endpoint binding to store only after password admission succeeds.
    #[must_use]
    pub const fn reconnect_binding(&self) -> &ReconnectEndpointBinding {
        &self.reconnect_binding
    }

    /// Creates the outgoing pinned WebTransport client.
    pub fn connect(self, world: &mut World) -> Entity {
        connect_client(world, self.config, self.target, self.reconnect_binding)
    }
}

impl fmt::Debug for PreparedDirectDiscoveryJoin {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("PreparedDirectDiscoveryJoin")
            .field("target", &self.target)
            .field("reconnect_binding", &self.reconnect_binding)
            .field("config", &"[PINNED WEBTRANSPORT CLIENT CONFIG]")
            .finish()
    }
}

impl PreparedDirectJoin {
    /// Configures mandatory SPKI validation from a parsed share code.
    pub fn new(connection_code: &DirectConnectionCode) -> Result<Self, DirectTransportError> {
        let config = pinned_client_config(
            connection_code.certificate_fingerprint,
            connection_code.certificate_expires_unix_seconds,
        )?;
        Ok(Self {
            config,
            target: direct_target(&connection_code.endpoint),
            invite_token: connection_code.invite_token,
            reconnect_binding: ReconnectEndpointBinding::new(
                connection_code.endpoint.clone(),
                connection_code.certificate_fingerprint,
                connection_code.certificate_expires_unix_seconds,
            )
            .map_err(|_error| DirectTransportError::InvalidCertificateIdentity)?,
        })
    }

    /// Invitation placed in the encrypted authentication handshake.
    #[must_use]
    pub const fn invite_token(&self) -> InviteToken {
        self.invite_token
    }

    /// Verified endpoint binding to store after successful authentication.
    #[must_use]
    pub const fn reconnect_binding(&self) -> &ReconnectEndpointBinding {
        &self.reconnect_binding
    }

    /// Creates the outgoing WebTransport session and Replicon client entity.
    pub fn connect(self, world: &mut World) -> Entity {
        connect_client(world, self.config, self.target, self.reconnect_binding)
    }
}

impl fmt::Debug for PreparedDirectJoin {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("PreparedDirectJoin")
            .field("target", &self.target)
            .field("invite_token", &self.invite_token)
            .field("reconnect_binding", &self.reconnect_binding)
            .field("config", &"[PINNED WEBTRANSPORT CLIENT CONFIG]")
            .finish()
    }
}

/// Prepared pinned connection for a persisted rotating credential.
pub struct PreparedDirectReconnect {
    config: ClientConfig,
    target: String,
    reconnect_binding: ReconnectEndpointBinding,
}

impl PreparedDirectReconnect {
    /// Revalidates the persisted endpoint and configures mandatory SPKI pinning.
    pub fn new(binding: &ReconnectEndpointBinding) -> Result<Self, DirectTransportError> {
        let reconnect_binding = ReconnectEndpointBinding::new(
            binding.endpoint.clone(),
            binding.certificate_fingerprint,
            binding.certificate_expires_unix_seconds,
        )
        .map_err(|_error| DirectTransportError::InvalidCertificateIdentity)?;
        let config = pinned_client_config(
            reconnect_binding.certificate_fingerprint,
            reconnect_binding.certificate_expires_unix_seconds,
        )?;
        Ok(Self {
            target: direct_target(&reconnect_binding.endpoint),
            config,
            reconnect_binding,
        })
    }

    /// Creates the outgoing WebTransport session and Replicon client entity.
    pub fn connect(self, world: &mut World) -> Entity {
        connect_client(world, self.config, self.target, self.reconnect_binding)
    }
}

impl fmt::Debug for PreparedDirectReconnect {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("PreparedDirectReconnect")
            .field("target", &self.target)
            .field("reconnect_binding", &self.reconnect_binding)
            .field("config", &"[PINNED WEBTRANSPORT CLIENT CONFIG]")
            .finish()
    }
}

fn connect_client(
    world: &mut World,
    config: ClientConfig,
    target: String,
    reconnect_binding: ReconnectEndpointBinding,
) -> Entity {
    world.insert_resource(reconnect_binding);
    let client = world.spawn(AeronetRepliconClient).id();
    WebTransportClient::connect(config, target).apply(world.entity_mut(client));
    client
}

fn pinned_client_config(
    fingerprint: CertificateFingerprint,
    expires: u64,
) -> Result<ClientConfig, DirectTransportError> {
    let verifier = Arc::new(SpkiPinVerifier::with_expiry(fingerprint, expires));
    let tls_config = tls::client::build_default_tls_config(
        Arc::new(rustls::RootCertStore::empty()),
        Some(verifier),
    );
    let config = ClientConfig::builder()
        .with_bind_default()
        .with_custom_tls(tls_config)
        .keep_alive_interval(Some(KEEP_ALIVE))
        .max_idle_timeout(Some(IDLE_TIMEOUT))
        .map_err(|_error| DirectTransportError::InvalidIdleTimeout)?
        .build();
    Ok(config)
}

/// Rustls verifier for one exact SHA-256 SubjectPublicKeyInfo pin.
#[derive(Debug)]
pub struct SpkiPinVerifier {
    expected: CertificateFingerprint,
    expected_expiry: Option<u64>,
    supported_algorithms: WebPkiSupportedAlgorithms,
}

impl SpkiPinVerifier {
    /// Creates a verifier for an exact fingerprint.
    #[must_use]
    pub fn new(expected: CertificateFingerprint) -> Self {
        Self {
            expected,
            expected_expiry: None,
            supported_algorithms: rustls::crypto::ring::default_provider()
                .signature_verification_algorithms,
        }
    }

    fn with_expiry(expected: CertificateFingerprint, expected_expiry: u64) -> Self {
        Self {
            expected,
            expected_expiry: Some(expected_expiry),
            supported_algorithms: rustls::crypto::ring::default_provider()
                .signature_verification_algorithms,
        }
    }
}

impl ServerCertVerifier for SpkiPinVerifier {
    fn verify_server_cert(
        &self,
        end_entity: &CertificateDer<'_>,
        intermediates: &[CertificateDer<'_>],
        _server_name: &ServerName<'_>,
        _ocsp_response: &[u8],
        now: UnixTime,
    ) -> Result<ServerCertVerified, rustls::Error> {
        verify_pinned_certificate(
            self.expected,
            self.expected_expiry,
            end_entity,
            intermediates,
            now,
        )?;
        Ok(ServerCertVerified::assertion())
    }

    fn verify_tls12_signature(
        &self,
        message: &[u8],
        certificate: &CertificateDer<'_>,
        signature: &rustls::DigitallySignedStruct,
    ) -> Result<HandshakeSignatureValid, rustls::Error> {
        rustls::crypto::verify_tls12_signature(
            message,
            certificate,
            signature,
            &self.supported_algorithms,
        )
    }

    fn verify_tls13_signature(
        &self,
        message: &[u8],
        certificate: &CertificateDer<'_>,
        signature: &rustls::DigitallySignedStruct,
    ) -> Result<HandshakeSignatureValid, rustls::Error> {
        rustls::crypto::verify_tls13_signature(
            message,
            certificate,
            signature,
            &self.supported_algorithms,
        )
    }

    fn supported_verify_schemes(&self) -> Vec<rustls::SignatureScheme> {
        self.supported_algorithms.supported_schemes()
    }
}

fn verify_pinned_certificate(
    expected: CertificateFingerprint,
    expected_expiry: Option<u64>,
    end_entity: &CertificateDer<'_>,
    intermediates: &[CertificateDer<'_>],
    now: UnixTime,
) -> Result<(), rustls::Error> {
    if !intermediates.is_empty() {
        return Err(rustls::CertificateError::UnknownIssuer.into());
    }
    let (remaining, certificate) = X509Certificate::from_der(end_entity.as_ref())
        .map_err(|_error| rustls::CertificateError::BadEncoding)?;
    if !remaining.is_empty() {
        return Err(rustls::CertificateError::BadEncoding.into());
    }
    let now =
        i64::try_from(now.as_secs()).map_err(|_error| rustls::CertificateError::BadEncoding)?;
    let not_before = certificate.validity().not_before.timestamp();
    let not_after = certificate.validity().not_after.timestamp();
    if now < not_before {
        return Err(rustls::CertificateError::NotValidYet.into());
    }
    if now > not_after {
        return Err(rustls::CertificateError::Expired.into());
    }
    if expected_expiry.is_some_and(|value| u64::try_from(not_after) != Ok(value)) {
        return Err(rustls::CertificateError::BadEncoding.into());
    }
    let lifetime = not_after
        .checked_sub(not_before)
        .ok_or(rustls::CertificateError::BadEncoding)?;
    if !(1..=MAX_CERTIFICATE_LIFETIME_SECS).contains(&lifetime) {
        return Err(rustls::CertificateError::UnknownIssuer.into());
    }
    let public_key = certificate.public_key();
    if public_key.algorithm.algorithm != OID_KEY_TYPE_EC_PUBLIC_KEY {
        return Err(rustls::CertificateError::UnknownIssuer.into());
    }
    if !matches!(
        public_key
            .algorithm
            .parameters
            .as_ref()
            .map(|parameters| parameters.as_oid()),
        Some(Ok(oid)) if oid == OID_EC_P256
    ) {
        return Err(rustls::CertificateError::UnknownIssuer.into());
    }
    let actual = CertificateFingerprint::from_bytes(Sha256::digest(public_key.raw).into());
    if actual != expected {
        return Err(rustls::CertificateError::UnknownIssuer.into());
    }
    Ok(())
}

fn generate_identity(
    endpoint: &DirectEndpoint,
) -> Result<(wtransport::Identity, CertificateFingerprint, u64), DirectTransportError> {
    let identity = wtransport::Identity::self_signed_builder()
        .subject_alt_names([certificate_san(endpoint.host())])
        .from_now_utc()
        .validity_days(14)
        .build()
        .map_err(|_error| DirectTransportError::InvalidCertificateIdentity)?;
    let certificate = identity
        .certificate_chain()
        .as_slice()
        .first()
        .ok_or(DirectTransportError::MissingLeafCertificate)?;
    let fingerprint = spki_fingerprint(certificate.der())?;
    let (_remaining, parsed) = X509Certificate::from_der(certificate.der())
        .map_err(|_error| DirectTransportError::InvalidCertificateEncoding)?;
    let expires = u64::try_from(parsed.validity().not_after.timestamp())
        .map_err(|_error| DirectTransportError::InvalidCertificateEncoding)?;
    Ok((identity, fingerprint, expires))
}

fn spki_fingerprint(der: &[u8]) -> Result<CertificateFingerprint, DirectTransportError> {
    let (remaining, certificate) = X509Certificate::from_der(der)
        .map_err(|_error| DirectTransportError::InvalidCertificateEncoding)?;
    if !remaining.is_empty() {
        return Err(DirectTransportError::InvalidCertificateEncoding);
    }
    Ok(CertificateFingerprint::from_bytes(
        Sha256::digest(certificate.public_key().raw).into(),
    ))
}

fn certificate_san(host: &str) -> &str {
    host.strip_prefix('[')
        .and_then(|value| value.strip_suffix(']'))
        .unwrap_or(host)
}

fn direct_target(endpoint: &DirectEndpoint) -> String {
    let host = endpoint.host();
    let authority = if host.contains(':') && !(host.starts_with('[') && host.ends_with(']')) {
        format!("[{host}]")
    } else {
        host.to_owned()
    };
    format!(
        "https://{authority}:{}{DIRECT_SESSION_PATH}",
        endpoint.port()
    )
}

pub(crate) fn respond_to_direct_session(mut request: On<SessionRequest>) {
    let response = if request.path == DIRECT_SESSION_PATH {
        SessionResponse::Accepted
    } else {
        SessionResponse::NotFound
    };
    request.respond(response);
}

/// Failure while preparing a direct encrypted endpoint.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DirectTransportError {
    /// Hostname/IP could not form a short-lived identity.
    InvalidCertificateIdentity,
    /// Generated identity unexpectedly omitted a leaf certificate.
    MissingLeafCertificate,
    /// Generated certificate DER could not be parsed exactly.
    InvalidCertificateEncoding,
    /// Fixed idle timeout was rejected by the transport builder.
    InvalidIdleTimeout,
}

impl fmt::Display for DirectTransportError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(match self {
            Self::InvalidCertificateIdentity => "could not generate the direct host identity",
            Self::MissingLeafCertificate => "direct host identity has no leaf certificate",
            Self::InvalidCertificateEncoding => "direct host certificate encoding is invalid",
            Self::InvalidIdleTimeout => "direct transport idle timeout is invalid",
        })
    }
}

impl std::error::Error for DirectTransportError {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn generated_identity_matches_share_code_and_debug_is_redacted() {
        let prepared = PreparedDirectHost::new(
            DirectEndpoint::new("127.0.0.1", DEFAULT_DIRECT_PORT).expect("valid endpoint"),
            SessionId::from_bytes([1; 16]),
            InviteToken::from_bytes([7; 16]),
        )
        .expect("direct host prepares");
        assert!(PreparedDirectJoin::new(prepared.connection_code()).is_ok());
        assert!(format!("{prepared:?}").contains("[WEBTRANSPORT SERVER CONFIG]"));
        assert!(!format!("{prepared:?}").contains("07070707"));
    }

    #[test]
    fn target_brackets_ipv6_and_uses_fixed_path() {
        let ipv4 = DirectEndpoint::new("127.0.0.1", 7777).expect("valid endpoint");
        assert_eq!(direct_target(&ipv4), "https://127.0.0.1:7777/bgn1");
        let ipv6 = DirectEndpoint::new("::1", 7777).expect("valid endpoint");
        assert_eq!(direct_target(&ipv6), "https://[::1]:7777/bgn1");
    }
}
