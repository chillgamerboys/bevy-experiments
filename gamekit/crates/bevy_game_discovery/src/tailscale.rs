//! Development-only discovery across peers reported by an installed Tailscale client.

use std::{
    collections::BTreeSet,
    fmt, io,
    net::{IpAddr, SocketAddr, UdpSocket},
    path::{Path, PathBuf},
    process::Command,
    time::Duration,
};

use bevy_game_multiplayer::DiscoveredDirectTarget;
use serde_json::Value;

use crate::{
    DiscoveryObservation, DiscoveryProviderId, DiscoveryRoute, DiscoverySource, SessionMetadata,
    WireAnnouncement,
};

/// Fixed development-only UDP discovery port.
pub const TAILNET_DISCOVERY_PORT: u16 = 7778;
const PROBE_PREFIX: &str = "BGDISC1:";
const MAX_STATUS_BYTES: usize = 1024 * 1024;
const MAX_DATAGRAM_BYTES: usize = 4096;
const MAX_PROBE_ADDRESSES: usize = 256;
const ROUTE_TTL: Duration = Duration::from_secs(15);

/// Reachable peer returned by `tailscale status --json`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TailscalePeer {
    /// Player-facing device DNS name when available.
    pub dns_name: String,
    /// Tailnet addresses eligible for bounded unicast probes.
    pub addresses: Vec<IpAddr>,
}

/// Fixed-argument wrapper around an externally installed Tailscale CLI.
#[derive(Debug, Clone)]
pub struct TailscaleCli {
    program: PathBuf,
}

impl TailscaleCli {
    /// Uses `tailscale` (or `tailscale.exe`) from the process path.
    #[must_use]
    pub fn from_path() -> Self {
        Self {
            program: PathBuf::from(if cfg!(windows) {
                "tailscale.exe"
            } else {
                "tailscale"
            }),
        }
    }

    /// Uses one explicit executable, primarily for deterministic tests.
    #[must_use]
    pub fn with_program(program: impl Into<PathBuf>) -> Self {
        Self {
            program: program.into(),
        }
    }

    /// Exact executable path/name; arguments are not configurable.
    #[must_use]
    pub fn program(&self) -> &Path {
        &self.program
    }

    /// Runs fixed `status --json` arguments and returns online peer addresses.
    pub fn peers(&self) -> Result<Vec<TailscalePeer>, TailnetDiscoveryError> {
        let root = self.status()?;
        parse_peers(&root)
    }

    /// Returns this device's tailnet addresses from the same fixed status command.
    pub fn local_addresses(&self) -> Result<Vec<IpAddr>, TailnetDiscoveryError> {
        let root = self.status()?;
        let addresses = root
            .get("Self")
            .and_then(|value| value.get("TailscaleIPs"))
            .and_then(Value::as_array)
            .into_iter()
            .flatten()
            .filter_map(Value::as_str)
            .filter_map(|value| value.parse::<IpAddr>().ok())
            .filter(|address| !address.is_unspecified() && !address.is_multicast())
            .collect::<BTreeSet<_>>()
            .into_iter()
            .collect::<Vec<_>>();
        if addresses.is_empty() {
            Err(TailnetDiscoveryError::ClientDisconnected)
        } else {
            Ok(addresses)
        }
    }

    fn status(&self) -> Result<Value, TailnetDiscoveryError> {
        let output = Command::new(&self.program)
            .args(["status", "--json"])
            .output()
            .map_err(|error| match error.kind() {
                io::ErrorKind::NotFound => TailnetDiscoveryError::CliUnavailable,
                _ => TailnetDiscoveryError::Io(error),
            })?;
        if !output.status.success() {
            return Err(TailnetDiscoveryError::ClientDisconnected);
        }
        if output.stdout.len() > MAX_STATUS_BYTES {
            return Err(TailnetDiscoveryError::MalformedStatus);
        }
        serde_json::from_slice(&output.stdout)
            .map_err(|_error| TailnetDiscoveryError::MalformedStatus)
    }
}

impl Default for TailscaleCli {
    fn default() -> Self {
        Self::from_path()
    }
}

/// Host-side unicast responder bound only to an explicit tailnet address.
pub struct TailnetResponder {
    socket: UdpSocket,
    announcement: WireAnnouncement,
}

impl TailnetResponder {
    /// Opens the explicit tailnet responder socket.
    pub fn bind(
        local_tailnet_address: IpAddr,
        metadata: SessionMetadata,
        target: DiscoveredDirectTarget,
    ) -> Result<Self, TailnetDiscoveryError> {
        if !metadata.password_required() || target.endpoint.port() == 0 {
            return Err(TailnetDiscoveryError::MalformedAnnouncement);
        }
        let socket = UdpSocket::bind(SocketAddr::new(
            local_tailnet_address,
            TAILNET_DISCOVERY_PORT,
        ))?;
        socket.set_nonblocking(true)?;
        Ok(Self {
            socket,
            announcement: WireAnnouncement {
                schema: 1,
                session_id: target.session_id,
                metadata,
                endpoint_port: target.endpoint.port(),
                certificate_fingerprint: target.certificate_fingerprint,
                certificate_expires_unix_seconds: target.certificate_expires_unix_seconds,
            },
        })
    }

    /// Responds to up to `budget` currently queued compatible probes.
    pub fn poll(&self, budget: usize) -> Result<usize, TailnetDiscoveryError> {
        let encoded = serde_json::to_vec(&self.announcement)
            .map_err(|_error| TailnetDiscoveryError::MalformedAnnouncement)?;
        if encoded.len() > MAX_DATAGRAM_BYTES {
            return Err(TailnetDiscoveryError::MalformedAnnouncement);
        }
        let expected = format!("{PROBE_PREFIX}{}", self.announcement.metadata.game_id());
        let mut buffer = [0_u8; MAX_DATAGRAM_BYTES];
        let mut responses = 0;
        for _ in 0..budget {
            let (length, source) = match self.socket.recv_from(&mut buffer) {
                Ok(value) => value,
                Err(error) if error.kind() == io::ErrorKind::WouldBlock => return Ok(responses),
                Err(error) => return Err(TailnetDiscoveryError::Io(error)),
            };
            if buffer.get(..length) == Some(expected.as_bytes()) {
                self.socket.send_to(&encoded, source)?;
                responses += 1;
            }
        }
        Ok(responses)
    }

    /// Updates sanitized public metadata while preserving the bound responder.
    pub fn refresh_metadata(
        &mut self,
        metadata: SessionMetadata,
    ) -> Result<bool, TailnetDiscoveryError> {
        if !metadata.password_required() {
            return Err(TailnetDiscoveryError::MalformedAnnouncement);
        }
        if self.announcement.metadata == metadata {
            return Ok(false);
        }
        self.announcement.metadata = metadata;
        Ok(true)
    }

    /// Exact local address selected by the host.
    pub fn local_addr(&self) -> Result<SocketAddr, TailnetDiscoveryError> {
        self.socket.local_addr().map_err(TailnetDiscoveryError::Io)
    }
}

impl fmt::Debug for TailnetResponder {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("TailnetResponder")
            .field("local_addr", &self.socket.local_addr().ok())
            .field("session_id", &self.announcement.session_id)
            .finish_non_exhaustive()
    }
}

/// Client-side bounded probe socket for peers returned by Tailscale.
pub struct TailnetBrowser {
    socket: UdpSocket,
    allowed_peers: BTreeSet<IpAddr>,
}

impl TailnetBrowser {
    /// Opens an ephemeral socket on an explicit local tailnet address.
    pub fn bind(local_tailnet_address: IpAddr) -> Result<Self, TailnetDiscoveryError> {
        let socket = UdpSocket::bind(SocketAddr::new(local_tailnet_address, 0))?;
        socket.set_nonblocking(true)?;
        Ok(Self {
            socket,
            allowed_peers: BTreeSet::new(),
        })
    }

    /// Sends one fixed probe to every unique address reported by the CLI.
    pub fn refresh(
        &mut self,
        peers: &[TailscalePeer],
        game_id: &str,
    ) -> Result<usize, TailnetDiscoveryError> {
        if game_id.is_empty() || game_id.len() > 64 || !game_id.is_ascii() {
            return Err(TailnetDiscoveryError::MalformedProbe);
        }
        self.allowed_peers.clear();
        for address in peers
            .iter()
            .flat_map(|peer| peer.addresses.iter().copied())
            .take(MAX_PROBE_ADDRESSES)
        {
            self.allowed_peers.insert(address);
        }
        let probe = format!("{PROBE_PREFIX}{game_id}");
        for address in &self.allowed_peers {
            self.socket.send_to(
                probe.as_bytes(),
                SocketAddr::new(*address, TAILNET_DISCOVERY_PORT),
            )?;
        }
        Ok(self.allowed_peers.len())
    }

    /// Drains currently available valid responses into provider-neutral observations.
    pub fn poll(
        &self,
        now: Duration,
        now_unix_seconds: u64,
    ) -> Result<Vec<DiscoveryObservation>, TailnetDiscoveryError> {
        let mut observations = Vec::new();
        let mut buffer = [0_u8; MAX_DATAGRAM_BYTES];
        loop {
            let (length, source) = match self.socket.recv_from(&mut buffer) {
                Ok(value) => value,
                Err(error) if error.kind() == io::ErrorKind::WouldBlock => return Ok(observations),
                Err(error) => return Err(TailnetDiscoveryError::Io(error)),
            };
            if !self.allowed_peers.contains(&source.ip()) {
                continue;
            }
            let announcement: WireAnnouncement = serde_json::from_slice(
                buffer
                    .get(..length)
                    .ok_or(TailnetDiscoveryError::MalformedAnnouncement)?,
            )
            .map_err(|_error| TailnetDiscoveryError::MalformedAnnouncement)?;
            if announcement.certificate_expires_unix_seconds <= now_unix_seconds
                || !announcement.metadata.password_required()
            {
                continue;
            }
            let target = announcement
                .target(source.ip().to_string())
                .map_err(|_error| TailnetDiscoveryError::MalformedAnnouncement)?;
            observations.push(DiscoveryObservation::Found {
                metadata: announcement.metadata,
                route: DiscoveryRoute::new(
                    DiscoveryProviderId::TAILSCALE,
                    DiscoverySource::Tailnet,
                    target,
                    now + ROUTE_TTL,
                ),
            });
        }
    }
}

impl fmt::Debug for TailnetBrowser {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("TailnetBrowser")
            .field("local_addr", &self.socket.local_addr().ok())
            .field("allowed_peer_count", &self.allowed_peers.len())
            .finish_non_exhaustive()
    }
}

/// Failure from the optional Tailscale CLI/unicast provider.
#[derive(Debug)]
pub enum TailnetDiscoveryError {
    /// Tailscale executable was not installed or on the configured path.
    CliUnavailable,
    /// CLI reported a disconnected or failing client.
    ClientDisconnected,
    /// CLI JSON exceeded bounds or violated the expected shape.
    MalformedStatus,
    /// Caller supplied an invalid game probe.
    MalformedProbe,
    /// Response or host announcement violated the bounded schema.
    MalformedAnnouncement,
    /// Socket or process I/O failed.
    Io(io::Error),
}

impl From<io::Error> for TailnetDiscoveryError {
    fn from(error: io::Error) -> Self {
        Self::Io(error)
    }
}

impl fmt::Display for TailnetDiscoveryError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(match self {
            Self::CliUnavailable => "Tailscale CLI is not installed",
            Self::ClientDisconnected => "Tailscale client is not connected",
            Self::MalformedStatus => "Tailscale status output is malformed",
            Self::MalformedProbe => "tailnet discovery probe is invalid",
            Self::MalformedAnnouncement => "tailnet discovery response is malformed",
            Self::Io(_) => "tailnet discovery I/O failed",
        })
    }
}

impl std::error::Error for TailnetDiscoveryError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Io(error) => Some(error),
            _ => None,
        }
    }
}

fn parse_peers(root: &Value) -> Result<Vec<TailscalePeer>, TailnetDiscoveryError> {
    let peers = root
        .get("Peer")
        .and_then(Value::as_object)
        .ok_or(TailnetDiscoveryError::MalformedStatus)?;
    let mut parsed = Vec::new();
    for peer in peers.values() {
        if peer.get("Online").and_then(Value::as_bool) == Some(false) {
            continue;
        }
        let dns_name = peer
            .get("DNSName")
            .and_then(Value::as_str)
            .unwrap_or("tailnet peer")
            .trim_end_matches('.')
            .to_owned();
        let addresses = peer
            .get("TailscaleIPs")
            .and_then(Value::as_array)
            .into_iter()
            .flatten()
            .filter_map(Value::as_str)
            .filter_map(|value| value.parse::<IpAddr>().ok())
            .filter(|address| !address.is_unspecified() && !address.is_multicast())
            .collect::<BTreeSet<_>>()
            .into_iter()
            .collect::<Vec<_>>();
        if !addresses.is_empty() {
            parsed.push(TailscalePeer {
                dns_name,
                addresses,
            });
        }
    }
    parsed.sort_by(|left, right| left.dns_name.cmp(&right.dns_name));
    Ok(parsed)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn status_parser_filters_offline_and_invalid_addresses() {
        let fixture = br#"{
            "Peer": {
                "a": {"DNSName":"beta.example.ts.net.","Online":true,"TailscaleIPs":["100.64.0.2","fd7a:115c:a1e0::2"]},
                "b": {"DNSName":"offline.example.ts.net.","Online":false,"TailscaleIPs":["100.64.0.3"]},
                "c": {"DNSName":"bad.example.ts.net.","Online":true,"TailscaleIPs":["not-an-ip"]}
            }
        }"#;
        let root: Value = serde_json::from_slice(fixture).expect("fixture JSON");
        let peers = parse_peers(&root).expect("fixture parses");
        assert_eq!(peers.len(), 1);
        let peer = peers.first().expect("one online peer");
        assert_eq!(peer.dns_name, "beta.example.ts.net");
        assert_eq!(peer.addresses.len(), 2);
    }

    #[test]
    fn malformed_status_fails_closed() {
        assert!(parse_peers(&serde_json::json!({})).is_err());
    }

    #[test]
    fn missing_cli_is_typed_unavailable() {
        let cli = TailscaleCli::with_program("definitely-missing-gamekit-tailscale");
        assert!(matches!(
            cli.peers(),
            Err(TailnetDiscoveryError::CliUnavailable)
        ));
    }

    #[cfg(unix)]
    #[test]
    fn fake_executable_requires_fixed_status_json_arguments() {
        use std::os::unix::fs::PermissionsExt as _;

        let directory = tempfile::tempdir().expect("temporary directory");
        let executable = directory.path().join("tailscale-fixture");
        std::fs::write(
            &executable,
            "#!/bin/sh\n[ \"$1\" = status ] && [ \"$2\" = --json ] || exit 9\nprintf '%s' '{\"Self\":{\"TailscaleIPs\":[\"100.64.0.1\"]},\"Peer\":{}}'\n",
        )
        .expect("fixture executable is written");
        let mut permissions = std::fs::metadata(&executable)
            .expect("fixture metadata")
            .permissions();
        permissions.set_mode(0o700);
        std::fs::set_permissions(&executable, permissions).expect("fixture is executable");

        let cli = TailscaleCli::with_program(executable);
        assert_eq!(
            cli.local_addresses().expect("fixed arguments accepted"),
            vec!["100.64.0.1".parse::<IpAddr>().expect("valid address")]
        );
    }
}
