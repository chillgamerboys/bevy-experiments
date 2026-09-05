//! Explicit same-link mDNS/DNS-SD advertisement and browsing.

use std::{collections::BTreeMap, fmt, net::IpAddr, sync::Mutex, time::Duration};

use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine as _};
use dns_sd_native::{ServiceRegistration, ServiceRegistrationBuilder};
use mdns_sd_discovery::{
    BrowseEvent, DiscoveredService, RemovedService, ServiceBrowser, ServiceBrowserBuilder,
};

use crate::{
    DiscoveryObservation, DiscoveryProviderId, DiscoveryRoute, DiscoverySource, SessionMetadata,
    WireAnnouncement,
};
use bevy_game_multiplayer::{
    local_network_interface_index, CertificateFingerprint, DiscoveredDirectTarget, SessionId,
};

/// DNS-SD service type used by Gamekit LAN discovery.
pub const MDNS_SERVICE_TYPE: &str = "_bevy-gamekit._udp.local.";
const NATIVE_SERVICE_TYPE: &str = "_bevy-gamekit._udp";
const ROUTE_TTL: Duration = Duration::from_secs(10);
const RENEW_INTERVAL: Duration = Duration::from_secs(4);
const MAX_RECORDS: usize = 256;
const MAX_SERVICE_ID_BYTES: usize = 512;

const PROPERTY_SCHEMA: &str = "v";
const PROPERTY_SESSION: &str = "session";
const PROPERTY_GAME: &str = "game";
const PROPERTY_PROTOCOL: &str = "protocol";
const PROPERTY_BUILD: &str = "build";
const PROPERTY_NAME: &str = "name";
const PROPERTY_CLAIMED: &str = "claimed";
const PROPERTY_CAPACITY: &str = "capacity";
const PROPERTY_LOCKED: &str = "locked";
const PROPERTY_PIN: &str = "pin";
const PROPERTY_EXPIRES: &str = "expires";
const PROPERTY_ADDRESS: &str = "address";

/// Complete secret-free input for one explicitly discoverable LAN session.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MdnsSessionAdvertisement {
    /// Sanitized game-owned session metadata.
    pub metadata: SessionMetadata,
    /// Pinned direct target; contains no admission secret.
    pub target: DiscoveredDirectTarget,
}

impl MdnsSessionAdvertisement {
    /// Requires password-gated metadata and a valid concrete session.
    pub fn new(
        metadata: SessionMetadata,
        target: DiscoveredDirectTarget,
    ) -> Result<Self, MdnsDiscoveryError> {
        if !metadata.password_required() || !target.session_id.is_valid() {
            return Err(MdnsDiscoveryError::MalformedAnnouncement("session policy"));
        }
        let address = advertised_address(&target)?;
        if !usable_address(address) || address.is_loopback() {
            return Err(MdnsDiscoveryError::MalformedAnnouncement("LAN address"));
        }
        Ok(Self { metadata, target })
    }
}

/// Active publisher for one explicitly enabled LAN session.
pub struct MdnsAdvertiser {
    runtime: tokio::runtime::Runtime,
    registration: Mutex<Option<ServiceRegistration>>,
    current: MdnsSessionAdvertisement,
}

impl MdnsAdvertiser {
    /// Updates public metadata without replacing this provider's endpoint.
    pub fn refresh_metadata(
        &mut self,
        metadata: SessionMetadata,
    ) -> Result<bool, MdnsDiscoveryError> {
        self.refresh(MdnsSessionAdvertisement::new(
            metadata,
            self.current.target.clone(),
        )?)
    }

    /// Starts multicast advertisement; this is the socket-opening action.
    pub fn start(advertisement: MdnsSessionAdvertisement) -> Result<Self, MdnsDiscoveryError> {
        let runtime = tokio::runtime::Builder::new_multi_thread()
            .worker_threads(1)
            .enable_all()
            .build()
            .map_err(MdnsDiscoveryError::daemon)?;
        let registration = register_native(&runtime, &advertisement)?;
        Ok(Self {
            runtime,
            registration: Mutex::new(Some(registration)),
            current: advertisement,
        })
    }

    /// Re-announces changed public metadata for the same concrete session.
    pub fn refresh(
        &mut self,
        advertisement: MdnsSessionAdvertisement,
    ) -> Result<bool, MdnsDiscoveryError> {
        self.poll_health()?;
        if advertisement.target.session_id != self.current.target.session_id {
            return Err(MdnsDiscoveryError::SessionChanged);
        }
        if advertisement == self.current {
            return Ok(false);
        }
        let slot = self
            .registration
            .get_mut()
            .map_err(|_error| MdnsDiscoveryError::DaemonStopped)?;
        if let Some(registration) = slot.take() {
            self.runtime
                .block_on(registration.unregister())
                .map_err(MdnsDiscoveryError::daemon)?;
        }
        *slot = Some(register_native(&self.runtime, &advertisement)?);
        self.current = advertisement;
        Ok(true)
    }

    /// Surfaces lazy daemon/socket failures.
    pub fn poll_health(&self) -> Result<(), MdnsDiscoveryError> {
        if self
            .registration
            .lock()
            .map_err(|_error| MdnsDiscoveryError::DaemonStopped)?
            .is_some()
        {
            Ok(())
        } else {
            Err(MdnsDiscoveryError::DaemonStopped)
        }
    }
}

impl fmt::Debug for MdnsAdvertiser {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("MdnsAdvertiser")
            .field("current", &self.current)
            .finish_non_exhaustive()
    }
}

impl Drop for MdnsAdvertiser {
    fn drop(&mut self) {
        if let Ok(slot) = self.registration.get_mut() {
            if let Some(registration) = slot.take() {
                let _status = self.runtime.block_on(registration.unregister());
            }
        }
    }
}

/// Active continuous browser for password-gated LAN sessions.
pub struct MdnsBrowser {
    browser: ServiceBrowser,
    runtime: tokio::runtime::Runtime,
    records: NativeRecords,
}

/// The OS owns DNS TTLs. Registry leases are renewed while its browse record exists.
#[derive(Default)]
struct NativeRecords {
    sessions: BTreeMap<String, MdnsSessionAdvertisement>,
    next_renewal: Duration,
}

impl NativeRecords {
    fn found(
        &mut self,
        key: String,
        record: MdnsSessionAdvertisement,
        now: Duration,
    ) -> Option<DiscoveryObservation> {
        if self.sessions.len() >= MAX_RECORDS && !self.sessions.contains_key(&key) {
            return None;
        }
        let observation = Self::observation(&record, now);
        self.sessions.insert(key, record);
        Some(observation)
    }

    fn lost(&mut self, key: &str) -> Option<DiscoveryObservation> {
        let session_id = self.sessions.remove(key)?.target.session_id;
        (!self
            .sessions
            .values()
            .any(|record| record.target.session_id == session_id))
        .then_some(DiscoveryObservation::Lost {
            session_id,
            provider: DiscoveryProviderId::MDNS,
        })
    }

    fn renew(&mut self, now: Duration, unix_seconds: u64) -> Vec<DiscoveryObservation> {
        if now < self.next_renewal {
            return Vec::new();
        }
        self.next_renewal = now + RENEW_INTERVAL;
        self.sessions
            .retain(|_, record| record.target.certificate_expires_unix_seconds > unix_seconds);
        self.sessions
            .values()
            .map(|record| Self::observation(record, now))
            .collect()
    }

    fn observation(record: &MdnsSessionAdvertisement, now: Duration) -> DiscoveryObservation {
        DiscoveryObservation::Found {
            metadata: record.metadata.clone(),
            route: DiscoveryRoute::new(
                DiscoveryProviderId::MDNS,
                DiscoverySource::Lan,
                record.target.clone(),
                now + ROUTE_TTL,
            ),
        }
    }
}

impl MdnsBrowser {
    /// Starts browsing; this is the socket-opening action.
    pub fn start() -> Result<Self, MdnsDiscoveryError> {
        let runtime = tokio::runtime::Builder::new_multi_thread()
            .worker_threads(1)
            .enable_all()
            .build()
            .map_err(MdnsDiscoveryError::daemon)?;
        let mut builder = ServiceBrowserBuilder::new();
        builder.service_type(NATIVE_SERVICE_TYPE).domain("local.");
        let browser = runtime
            .block_on(builder.browse())
            .map_err(MdnsDiscoveryError::daemon)?;
        Ok(Self {
            runtime,
            browser,
            records: NativeRecords::default(),
        })
    }

    /// Drains currently available records into provider-neutral observations.
    pub fn poll(
        &mut self,
        now: Duration,
        now_unix_seconds: u64,
    ) -> Result<Vec<DiscoveryObservation>, MdnsDiscoveryError> {
        let mut observations = Vec::new();
        for _event_budget in 0..64 {
            let event = self.runtime.block_on(async {
                tokio::time::timeout(Duration::from_millis(1), self.browser.recv()).await
            });
            match event {
                Err(_elapsed) => break,
                Ok(None) => return Err(MdnsDiscoveryError::DaemonStopped),
                Ok(Some(Err(error))) => return Err(MdnsDiscoveryError::daemon(error)),
                Ok(Some(Ok(BrowseEvent::Found(service)))) => {
                    if let Ok((metadata, target)) = parse_service(&service, now_unix_seconds) {
                        observations.extend(self.records.found(
                            service_key(&service),
                            MdnsSessionAdvertisement { metadata, target },
                            now,
                        ));
                    }
                }
                Ok(Some(Ok(BrowseEvent::Removed(service)))) => {
                    observations.extend(self.records.lost(&removed_service_key(&service)));
                }
            }
        }
        observations.extend(self.records.renew(now, now_unix_seconds));
        Ok(observations)
    }
}

impl fmt::Debug for MdnsBrowser {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("MdnsBrowser([ACTIVE BROWSER])")
    }
}

/// Failure to create or operate the opt-in LAN provider.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MdnsDiscoveryError {
    /// Local or received record violated the bounded schema.
    MalformedAnnouncement(&'static str),
    /// Refresh attempted to replace a different concrete session.
    SessionChanged,
    /// Background DNS-SD daemon stopped.
    DaemonStopped,
    /// Operating system or DNS-SD daemon refused an operation.
    ServiceUnavailable(String),
}

impl MdnsDiscoveryError {
    fn daemon(error: impl fmt::Display) -> Self {
        Self::ServiceUnavailable(error.to_string())
    }
}

impl fmt::Display for MdnsDiscoveryError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::MalformedAnnouncement(field) => {
                write!(formatter, "LAN discovery metadata has an invalid {field}")
            }
            Self::SessionChanged => formatter.write_str("LAN advertisement changed sessions"),
            Self::DaemonStopped => formatter.write_str("LAN discovery service stopped"),
            Self::ServiceUnavailable(reason) => {
                write!(formatter, "LAN discovery is unavailable: {reason}")
            }
        }
    }
}

impl std::error::Error for MdnsDiscoveryError {}

fn register_native(
    runtime: &tokio::runtime::Runtime,
    advertisement: &MdnsSessionAdvertisement,
) -> Result<ServiceRegistration, MdnsDiscoveryError> {
    let address = advertised_address(&advertisement.target)?;
    let interface_index = local_network_interface_index(address)
        .map_err(MdnsDiscoveryError::daemon)?
        .ok_or(MdnsDiscoveryError::MalformedAnnouncement("LAN interface"))?;
    let mut builder =
        ServiceRegistrationBuilder::new(NATIVE_SERVICE_TYPE, advertisement.target.endpoint.port());
    builder
        .name(instance_name(advertisement))
        .domain("local.")
        .interface_index(interface_index);
    for (key, value) in properties(advertisement) {
        builder.add_txt_record_key_string(key, value);
    }
    runtime
        .block_on(builder.register())
        .map_err(MdnsDiscoveryError::daemon)
}

fn instance_name(advertisement: &MdnsSessionAdvertisement) -> String {
    let session = encode_hex(&advertisement.target.session_id.to_bytes());
    let short = session.chars().take(8).collect::<String>();
    format!("{} {short}", advertisement.metadata.display_name())
}

fn advertised_address(target: &DiscoveredDirectTarget) -> Result<IpAddr, MdnsDiscoveryError> {
    target
        .endpoint
        .host()
        .trim_start_matches('[')
        .trim_end_matches(']')
        .parse()
        .map_err(|_error| MdnsDiscoveryError::MalformedAnnouncement("LAN address"))
}

fn properties(advertisement: &MdnsSessionAdvertisement) -> Vec<(String, String)> {
    vec![
        (PROPERTY_SCHEMA.to_owned(), "1".to_owned()),
        (
            PROPERTY_SESSION.to_owned(),
            URL_SAFE_NO_PAD.encode(advertisement.target.session_id.to_bytes()),
        ),
        (
            PROPERTY_GAME.to_owned(),
            advertisement.metadata.game_id().to_owned(),
        ),
        (
            PROPERTY_PROTOCOL.to_owned(),
            advertisement.metadata.protocol_version().to_owned(),
        ),
        (
            PROPERTY_BUILD.to_owned(),
            advertisement.metadata.build_id().to_owned(),
        ),
        (
            PROPERTY_NAME.to_owned(),
            advertisement.metadata.display_name().to_owned(),
        ),
        (
            PROPERTY_CLAIMED.to_owned(),
            advertisement.metadata.claimed_players().to_string(),
        ),
        (
            PROPERTY_CAPACITY.to_owned(),
            advertisement.metadata.player_capacity().to_string(),
        ),
        (PROPERTY_LOCKED.to_owned(), "1".to_owned()),
        (
            PROPERTY_PIN.to_owned(),
            URL_SAFE_NO_PAD.encode(advertisement.target.certificate_fingerprint.to_bytes()),
        ),
        (
            PROPERTY_EXPIRES.to_owned(),
            advertisement
                .target
                .certificate_expires_unix_seconds
                .to_string(),
        ),
        (
            PROPERTY_ADDRESS.to_owned(),
            advertisement.target.endpoint.host().to_owned(),
        ),
    ]
}

fn parse_service(
    service: &DiscoveredService,
    now_unix_seconds: u64,
) -> Result<(SessionMetadata, DiscoveredDirectTarget), MdnsDiscoveryError> {
    if service.service_type != NATIVE_SERVICE_TYPE
        || service.name.is_empty()
        || service.name.len() > MAX_SERVICE_ID_BYTES
        || service.port == 0
        || property(service, PROPERTY_SCHEMA)? != "1"
        || property(service, PROPERTY_LOCKED)? != "1"
    {
        return Err(MdnsDiscoveryError::MalformedAnnouncement(
            "service identity",
        ));
    }
    let session_id = SessionId::from_bytes(decode_exact::<16>(
        PROPERTY_SESSION,
        property(service, PROPERTY_SESSION)?,
    )?);
    if !session_id.is_valid() {
        return Err(MdnsDiscoveryError::MalformedAnnouncement("session"));
    }
    let claimed = parse_u8(service, PROPERTY_CLAIMED)?;
    let capacity = parse_u8(service, PROPERTY_CAPACITY)?;
    let metadata = SessionMetadata::new(
        property(service, PROPERTY_GAME)?,
        property(service, PROPERTY_PROTOCOL)?,
        property(service, PROPERTY_BUILD)?,
        property(service, PROPERTY_NAME)?,
        claimed,
        capacity,
        true,
    )
    .map_err(|_error| MdnsDiscoveryError::MalformedAnnouncement("metadata"))?;
    let fingerprint = CertificateFingerprint::from_bytes(decode_exact::<32>(
        PROPERTY_PIN,
        property(service, PROPERTY_PIN)?,
    )?);
    let expires = property(service, PROPERTY_EXPIRES)?
        .parse::<u64>()
        .map_err(|_error| MdnsDiscoveryError::MalformedAnnouncement("certificate expiry"))?;
    if expires <= now_unix_seconds {
        return Err(MdnsDiscoveryError::MalformedAnnouncement(
            "certificate expiry",
        ));
    }
    let host = property(service, PROPERTY_ADDRESS)?
        .trim_start_matches('[')
        .trim_end_matches(']')
        .parse::<IpAddr>()
        .map_err(|_error| MdnsDiscoveryError::MalformedAnnouncement("reachable address"))?;
    if !usable_address(host) || host.is_loopback() || !service.addresses.contains(&host) {
        return Err(MdnsDiscoveryError::MalformedAnnouncement(
            "reachable address",
        ));
    }
    let wire = WireAnnouncement {
        schema: 1,
        session_id,
        metadata: metadata.clone(),
        endpoint_port: service.port,
        certificate_fingerprint: fingerprint,
        certificate_expires_unix_seconds: expires,
    };
    let target = wire
        .target(host.to_string())
        .map_err(|_error| MdnsDiscoveryError::MalformedAnnouncement("reachable address"))?;
    Ok((metadata, target))
}

fn property<'a>(
    service: &'a DiscoveredService,
    key: &'static str,
) -> Result<&'a str, MdnsDiscoveryError> {
    service
        .txt(key)
        .ok_or(MdnsDiscoveryError::MalformedAnnouncement(key))
        .and_then(|value| {
            std::str::from_utf8(value)
                .map_err(|_error| MdnsDiscoveryError::MalformedAnnouncement(key))
        })
}

fn parse_u8(service: &DiscoveredService, key: &'static str) -> Result<u8, MdnsDiscoveryError> {
    property(service, key)?
        .parse::<u8>()
        .map_err(|_error| MdnsDiscoveryError::MalformedAnnouncement(key))
}

fn service_key(service: &DiscoveredService) -> String {
    identity_key(
        &service.name,
        &service.service_type,
        &service.domain,
        service.interface_index,
    )
}

fn removed_service_key(service: &RemovedService) -> String {
    identity_key(
        &service.name,
        &service.service_type,
        &service.domain,
        service.interface_index,
    )
}

fn identity_key(
    name: &str,
    service_type: &str,
    domain: &str,
    interface_index: Option<std::num::NonZeroU32>,
) -> String {
    format!(
        "{name}.{service_type}.{domain}@{}",
        interface_index.map_or(0, std::num::NonZeroU32::get)
    )
}

fn decode_exact<const LENGTH: usize>(
    field: &'static str,
    encoded: &str,
) -> Result<[u8; LENGTH], MdnsDiscoveryError> {
    URL_SAFE_NO_PAD
        .decode(encoded)
        .map_err(|_error| MdnsDiscoveryError::MalformedAnnouncement(field))?
        .try_into()
        .map_err(|_length_error| MdnsDiscoveryError::MalformedAnnouncement(field))
}

fn usable_address(address: IpAddr) -> bool {
    match address {
        IpAddr::V4(address) => !address.is_unspecified() && !address.is_multicast(),
        IpAddr::V6(address) => {
            !address.is_unspecified()
                && !address.is_multicast()
                && !address.is_loopback()
                && !address.is_unicast_link_local()
        }
    }
}

fn encode_hex(bytes: &[u8]) -> String {
    use fmt::Write as _;
    let mut encoded = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        let _written = write!(encoded, "{byte:02x}");
    }
    encoded
}

#[cfg(test)]
mod tests {
    use super::*;
    use bevy_game_multiplayer::{DirectEndpoint, SessionId};
    use mdns_sd_discovery::TxtRecord;

    fn advertisement() -> MdnsSessionAdvertisement {
        MdnsSessionAdvertisement::new(
            SessionMetadata::new("deckbuilder", "1", "build-a", "Copper Table", 1, 2, true)
                .expect("valid metadata"),
            DiscoveredDirectTarget {
                session_id: SessionId::from_bytes([7; 16]),
                endpoint: DirectEndpoint::new("192.168.1.20", 7777).expect("valid endpoint"),
                certificate_fingerprint: CertificateFingerprint::from_bytes([3; 32]),
                certificate_expires_unix_seconds: 2_000_000_000,
            },
        )
        .expect("valid advertisement")
    }

    fn resolved_service(
        advertisement: &MdnsSessionAdvertisement,
        addresses: Vec<IpAddr>,
    ) -> DiscoveredService {
        DiscoveredService {
            name: instance_name(advertisement),
            service_type: NATIVE_SERVICE_TYPE.to_owned(),
            domain: "local".to_owned(),
            host_name: "fixture.local".to_owned(),
            port: advertisement.target.endpoint.port(),
            addresses,
            txt_records: properties(advertisement)
                .into_iter()
                .map(|(key, value)| TxtRecord {
                    key,
                    value: Some(value.into_bytes()),
                })
                .collect(),
            interface_index: None,
        }
    }

    #[test]
    fn properties_contain_no_password_or_invitation() {
        let properties = properties(&advertisement());
        let debug = format!("{properties:?}");
        assert!(!debug.contains("password"));
        assert!(!debug.contains("invite"));
        assert!(!debug.contains("BGN1"));
        assert!(properties
            .iter()
            .all(|(key, value)| key.len() + value.len() < u8::MAX as usize));
    }

    #[test]
    fn unchanged_native_record_survives_registry_leases_then_goodbye_removes_it() {
        let mut records = NativeRecords::default();
        let mut registry = crate::DiscoveryRegistry::default();
        let found = records
            .found("wifi".into(), advertisement(), Duration::ZERO)
            .expect("record accepted");
        registry.apply(found, None);
        for seconds in 0..60 {
            let now = Duration::from_secs(seconds);
            for observation in records.renew(now, 1_900_000_000) {
                registry.apply(observation, None);
            }
            registry.expire(now);
            assert_eq!(
                registry.sessions(now).len(),
                1,
                "live record expired at {seconds}"
            );
        }
        registry.apply(records.lost("wifi").expect("goodbye"), None);
        assert!(registry.sessions(Duration::from_secs(60)).is_empty());
        assert!(records
            .renew(Duration::from_secs(80), 1_900_000_000)
            .is_empty());
    }

    #[test]
    fn removing_one_interface_keeps_the_other_and_certificates_still_expire() {
        let mut records = NativeRecords::default();
        records.found("wifi".into(), advertisement(), Duration::ZERO);
        records.found("ethernet".into(), advertisement(), Duration::ZERO);
        assert!(records.lost("wifi").is_none());
        assert_eq!(
            records.renew(Duration::from_secs(12), 1_900_000_000).len(),
            1
        );
        assert!(records
            .renew(Duration::from_secs(24), 2_000_000_000)
            .is_empty());
    }

    #[test]
    fn malformed_advertisement_is_rejected_without_partial_metadata() {
        let info = DiscoveredService {
            name: "malformed".to_owned(),
            service_type: NATIVE_SERVICE_TYPE.to_owned(),
            domain: "local".to_owned(),
            host_name: "fixture.local".to_owned(),
            port: 7777,
            addresses: vec!["192.168.1.42".parse().expect("fixture address")],
            txt_records: vec![TxtRecord {
                key: PROPERTY_SCHEMA.to_owned(),
                value: Some(b"1".to_vec()),
            }],
            interface_index: None,
        };
        assert!(parse_service(&info, 1_900_000_000).is_err());
    }

    #[test]
    fn resolved_record_uses_reachable_address_and_round_trips_metadata() {
        let mut advertisement = advertisement();
        advertisement.target.endpoint =
            DirectEndpoint::new("192.168.1.42", 7777).expect("fixture endpoint");
        let info = resolved_service(
            &advertisement,
            vec![
                "100.64.0.9".parse().expect("tailnet fixture"),
                "192.168.1.42".parse().expect("LAN fixture"),
            ],
        );
        let (metadata, target) = parse_service(&info, 1_900_000_000).expect("record resolves");
        assert_eq!(metadata.display_name(), "Copper Table");
        assert_eq!(target.endpoint.host(), "192.168.1.42");
    }

    #[test]
    fn advertisement_publishes_only_its_explicit_lan_address() {
        let addresses = properties(&advertisement())
            .into_iter()
            .filter_map(|(key, value)| (key == PROPERTY_ADDRESS).then_some(value))
            .collect::<Vec<_>>();
        assert_eq!(addresses, vec!["192.168.1.20".to_owned()]);
    }

    #[test]
    fn loopback_advertisement_is_rejected() {
        let mut advertisement = advertisement();
        advertisement.target.endpoint =
            DirectEndpoint::new("127.0.0.1", 7777).expect("syntactically valid endpoint");
        let error = MdnsSessionAdvertisement::new(advertisement.metadata, advertisement.target)
            .expect_err("loopback cannot represent a LAN route");
        assert!(matches!(
            error,
            MdnsDiscoveryError::MalformedAnnouncement("LAN address")
        ));
    }

    #[test]
    #[ignore = "requires a real non-loopback LAN interface"]
    fn native_advertiser_is_visible_to_an_independent_browser() {
        let Some(address) = bevy_game_multiplayer::local_network_addresses()
            .expect("local interfaces")
            .into_iter()
            .next()
        else {
            return;
        };
        let mut advertisement = advertisement();
        advertisement.target.session_id =
            bevy_game_multiplayer::SessionSecurityAuthority::new().session_id();
        advertisement.target.endpoint =
            DirectEndpoint::new(address.to_string(), 7777).expect("detected LAN endpoint");
        let expected_session = advertisement.target.session_id;
        let mut browser = MdnsBrowser::start().expect("independent browser starts");
        let advertiser = MdnsAdvertiser::start(advertisement).expect("native advertiser starts");
        let deadline = std::time::Instant::now() + Duration::from_secs(5);
        let mut discovered = false;
        while std::time::Instant::now() < deadline {
            advertiser.poll_health().expect("advertiser remains active");
            let observations = browser
                .poll(Duration::ZERO, 1_900_000_000)
                .expect("browser poll");
            if observations.iter().any(|observation| {
                matches!(
                    observation,
                    DiscoveryObservation::Found { route, .. }
                        if route.target.session_id == expected_session
                            && route.target.endpoint.host() == address.to_string()
                )
            }) {
                discovered = true;
                break;
            }
            std::thread::sleep(Duration::from_millis(50));
        }
        assert!(
            discovered,
            "the independent browser did not resolve the native LAN advertisement"
        );
    }
}
