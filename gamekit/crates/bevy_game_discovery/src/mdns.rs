//! Explicit same-link mDNS/DNS-SD advertisement and browsing.

use std::{collections::BTreeMap, fmt, net::IpAddr, time::Duration};

use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine as _};
use mdns_sd::{
    DaemonEvent, Receiver, ResolvedService, ServiceDaemon, ServiceEvent, ServiceInfo, TryRecvError,
};

use crate::{
    DiscoveryObservation, DiscoveryProviderId, DiscoveryRoute, DiscoverySource, SessionMetadata,
    WireAnnouncement,
};
use bevy_game_multiplayer::{CertificateFingerprint, DiscoveredDirectTarget, SessionId};

/// DNS-SD service type used by Gamekit LAN discovery.
pub const MDNS_SERVICE_TYPE: &str = "_bevy-gamekit._udp.local.";
const ROUTE_TTL: Duration = Duration::from_secs(10);
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
        Ok(Self { metadata, target })
    }
}

/// Active publisher for one explicitly enabled LAN session.
pub struct MdnsAdvertiser {
    daemon: ServiceDaemon,
    monitor: Receiver<DaemonEvent>,
    fullname: String,
    current: MdnsSessionAdvertisement,
}

impl MdnsAdvertiser {
    /// Starts multicast advertisement; this is the socket-opening action.
    pub fn start(advertisement: MdnsSessionAdvertisement) -> Result<Self, MdnsDiscoveryError> {
        let daemon = ServiceDaemon::new().map_err(MdnsDiscoveryError::daemon)?;
        let monitor = daemon.monitor().map_err(MdnsDiscoveryError::daemon)?;
        let info = service_info(&advertisement)?;
        let fullname = info.get_fullname().to_owned();
        daemon.register(info).map_err(MdnsDiscoveryError::daemon)?;
        Ok(Self {
            daemon,
            monitor,
            fullname,
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
        self.daemon
            .register(service_info(&advertisement)?)
            .map_err(MdnsDiscoveryError::daemon)?;
        self.current = advertisement;
        Ok(true)
    }

    /// Surfaces lazy daemon/socket failures.
    pub fn poll_health(&self) -> Result<(), MdnsDiscoveryError> {
        loop {
            match self.monitor.try_recv() {
                Ok(DaemonEvent::Error(error)) => return Err(MdnsDiscoveryError::daemon(error)),
                Ok(_) => {}
                Err(TryRecvError::Empty) => return Ok(()),
                Err(TryRecvError::Disconnected) => return Err(MdnsDiscoveryError::DaemonStopped),
            }
        }
    }
}

impl fmt::Debug for MdnsAdvertiser {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("MdnsAdvertiser")
            .field("fullname", &self.fullname)
            .field("current", &self.current)
            .finish_non_exhaustive()
    }
}

impl Drop for MdnsAdvertiser {
    fn drop(&mut self) {
        let _status = self.daemon.unregister(&self.fullname);
        let _status = self.daemon.shutdown();
    }
}

/// Active continuous browser for password-gated LAN sessions.
pub struct MdnsBrowser {
    daemon: ServiceDaemon,
    events: Receiver<ServiceEvent>,
    monitor: Receiver<DaemonEvent>,
    sessions: BTreeMap<String, SessionId>,
}

impl MdnsBrowser {
    /// Starts browsing; this is the socket-opening action.
    pub fn start() -> Result<Self, MdnsDiscoveryError> {
        let daemon = ServiceDaemon::new().map_err(MdnsDiscoveryError::daemon)?;
        let monitor = daemon.monitor().map_err(MdnsDiscoveryError::daemon)?;
        let events = daemon
            .browse(MDNS_SERVICE_TYPE)
            .map_err(MdnsDiscoveryError::daemon)?;
        Ok(Self {
            daemon,
            events,
            monitor,
            sessions: BTreeMap::new(),
        })
    }

    /// Drains currently available records into provider-neutral observations.
    pub fn poll(
        &mut self,
        now: Duration,
        now_unix_seconds: u64,
    ) -> Result<Vec<DiscoveryObservation>, MdnsDiscoveryError> {
        self.poll_health()?;
        let mut observations = Vec::new();
        loop {
            match self.events.try_recv() {
                Ok(ServiceEvent::ServiceResolved(service)) => {
                    if let Ok((metadata, target)) = parse_service(&service, now_unix_seconds) {
                        self.sessions
                            .insert(service.fullname.clone(), target.session_id);
                        observations.push(DiscoveryObservation::Found {
                            metadata,
                            route: DiscoveryRoute::new(
                                DiscoveryProviderId::MDNS,
                                DiscoverySource::Lan,
                                target,
                                now + ROUTE_TTL,
                            ),
                        });
                    }
                }
                Ok(ServiceEvent::ServiceRemoved(_service_type, fullname)) => {
                    if let Some(session_id) = self.sessions.remove(&fullname) {
                        observations.push(DiscoveryObservation::Lost {
                            session_id,
                            provider: DiscoveryProviderId::MDNS,
                        });
                    }
                }
                Ok(_) => {}
                Err(TryRecvError::Empty) => return Ok(observations),
                Err(TryRecvError::Disconnected) => return Err(MdnsDiscoveryError::DaemonStopped),
            }
        }
    }

    fn poll_health(&self) -> Result<(), MdnsDiscoveryError> {
        loop {
            match self.monitor.try_recv() {
                Ok(DaemonEvent::Error(error)) => return Err(MdnsDiscoveryError::daemon(error)),
                Ok(_) => {}
                Err(TryRecvError::Empty) => return Ok(()),
                Err(TryRecvError::Disconnected) => return Err(MdnsDiscoveryError::DaemonStopped),
            }
        }
    }
}

impl fmt::Debug for MdnsBrowser {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("MdnsBrowser([ACTIVE BROWSER])")
    }
}

impl Drop for MdnsBrowser {
    fn drop(&mut self) {
        let _status = self.daemon.stop_browse(MDNS_SERVICE_TYPE);
        let _status = self.daemon.shutdown();
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

fn service_info(
    advertisement: &MdnsSessionAdvertisement,
) -> Result<ServiceInfo, MdnsDiscoveryError> {
    let session = encode_hex(&advertisement.target.session_id.to_bytes());
    let short = session.chars().take(8).collect::<String>();
    let instance = format!("{} {short}", advertisement.metadata.display_name());
    let hostname = format!("bevy-gamekit-{session}.local.");
    ServiceInfo::new(
        MDNS_SERVICE_TYPE,
        &instance,
        &hostname,
        "",
        advertisement.target.endpoint.port(),
        properties(advertisement).as_slice(),
    )
    .map(ServiceInfo::enable_addr_auto)
    .map_err(MdnsDiscoveryError::daemon)
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
    ]
}

fn parse_service(
    service: &ResolvedService,
    now_unix_seconds: u64,
) -> Result<(SessionMetadata, DiscoveredDirectTarget), MdnsDiscoveryError> {
    if service.ty_domain != MDNS_SERVICE_TYPE
        || service.fullname.is_empty()
        || service.fullname.len() > MAX_SERVICE_ID_BYTES
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
    let host = preferred_address(service).ok_or(MdnsDiscoveryError::MalformedAnnouncement(
        "reachable address",
    ))?;
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
    service: &'a ResolvedService,
    key: &'static str,
) -> Result<&'a str, MdnsDiscoveryError> {
    service
        .get_property_val_str(key)
        .ok_or(MdnsDiscoveryError::MalformedAnnouncement(key))
}

fn parse_u8(service: &ResolvedService, key: &'static str) -> Result<u8, MdnsDiscoveryError> {
    property(service, key)?
        .parse::<u8>()
        .map_err(|_error| MdnsDiscoveryError::MalformedAnnouncement(key))
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

fn preferred_address(service: &ResolvedService) -> Option<IpAddr> {
    let mut addresses = service
        .get_addresses()
        .iter()
        .map(mdns_sd::ScopedIp::to_ip_addr)
        .filter(|address| usable_address(*address))
        .collect::<Vec<_>>();
    addresses.sort_by_key(|address| (address_rank(*address), address_bytes(*address)));
    addresses.into_iter().next()
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

fn address_rank(address: IpAddr) -> u8 {
    match address {
        IpAddr::V4(address) if address.is_private() => 0,
        IpAddr::V4(address) if !address.is_link_local() && !address.is_loopback() => 1,
        IpAddr::V6(_) => 2,
        IpAddr::V4(address) if address.is_link_local() => 3,
        IpAddr::V4(_) => 4,
    }
}

fn address_bytes(address: IpAddr) -> [u8; 16] {
    match address {
        IpAddr::V4(address) => address.to_ipv6_mapped().octets(),
        IpAddr::V6(address) => address.octets(),
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

    fn advertisement() -> MdnsSessionAdvertisement {
        MdnsSessionAdvertisement::new(
            SessionMetadata::new("deckbuilder", "1", "build-a", "Copper Table", 1, 2, true)
                .expect("valid metadata"),
            DiscoveredDirectTarget {
                session_id: SessionId::from_bytes([7; 16]),
                endpoint: DirectEndpoint::new("127.0.0.1", 7777).expect("valid endpoint"),
                certificate_fingerprint: CertificateFingerprint::from_bytes([3; 32]),
                certificate_expires_unix_seconds: 2_000_000_000,
            },
        )
        .expect("valid advertisement")
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
    fn malformed_advertisement_is_rejected_without_partial_metadata() {
        let incomplete = vec![(PROPERTY_SCHEMA.to_owned(), "1".to_owned())];
        let info = ServiceInfo::new(
            MDNS_SERVICE_TYPE,
            "malformed",
            "fixture.local.",
            "192.168.1.42",
            7777,
            incomplete.as_slice(),
        )
        .expect("service fixture")
        .as_resolved_service();
        assert!(parse_service(&info, 1_900_000_000).is_err());
    }

    #[test]
    fn resolved_record_uses_reachable_address_and_round_trips_metadata() {
        let advertisement = advertisement();
        let info = ServiceInfo::new(
            MDNS_SERVICE_TYPE,
            "Copper Table fixture",
            "fixture.local.",
            "100.64.0.9,192.168.1.42",
            7777,
            properties(&advertisement).as_slice(),
        )
        .expect("valid service")
        .as_resolved_service();
        let (metadata, target) = parse_service(&info, 1_900_000_000).expect("record resolves");
        assert_eq!(metadata.display_name(), "Copper Table");
        assert_eq!(target.endpoint.host(), "192.168.1.42");
    }

    #[test]
    #[ignore = "requires real local multicast sockets"]
    fn advertiser_and_browser_exchange_on_local_link() {
        let _advertiser = MdnsAdvertiser::start(advertisement()).expect("advertiser starts");
        let mut browser = MdnsBrowser::start().expect("browser starts");
        let deadline = std::time::Instant::now() + Duration::from_secs(5);
        let mut resolved = false;
        while std::time::Instant::now() < deadline {
            if !browser
                .poll(Duration::ZERO, 1_900_000_000)
                .expect("poll succeeds")
                .is_empty()
            {
                resolved = true;
                break;
            }
            std::thread::sleep(Duration::from_millis(50));
        }
        assert!(
            resolved,
            "local browser did not resolve the advertised session"
        );
    }
}
