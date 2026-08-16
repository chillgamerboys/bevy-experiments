//! Deckbuilder-owned wire protocol and composition of optional Gamekit capabilities.

use std::{
    fmt,
    net::IpAddr,
    time::{Duration, SystemTime, UNIX_EPOCH},
};

use aeronet::io::{
    connection::{Disconnected, PeerAddr},
    server::Close,
};
use bevy::prelude::*;
use bevy_game_discovery::{
    DiscoveryJoinRoute, DiscoveryObservation, DiscoveryPlugin, ExpectedSession, MdnsAdvertiser,
    MdnsBrowser, MdnsSessionAdvertisement, SessionMetadata, SessionPassword,
    SessionPasswordVerifier, TailnetBrowser, TailnetResponder, TailscaleCli,
};
use bevy_game_multiplayer::{
    AdmissionCredential, AtomicFileReconnectCredentialStore, DirectConnectionCode, DirectEndpoint,
    DiscoveredDirectTarget, EncodedConnectionCode, GameMultiplayerPlugin, InviteToken,
    MemoryReconnectCredentialStore, PeerId, PreparedDirectHost, PreparedDirectJoin,
    PreparedDirectReconnect, ReconnectCredential, ReconnectCredentialStorage,
    ReconnectEndpointBinding, SessionId, SessionSecurityAuthority, StoredReconnectCredential,
};
use bevy_replicon::prelude::{
    AuthorizedClient, Channel, ClientId, ClientMessageAppExt as _, ClientState, ConnectedClient,
    FromClient, ProtocolHasher, SendTargets, ServerMessageAppExt as _, ToClients,
};
use serde::{Deserialize, Serialize};
use zeroize::Zeroize as _;

use crate::domain::{
    DeckAuthority, GameCommand, GameCommandResult, GameRequest, GameSnapshot, RequestId, Seat,
};

pub(crate) const GAME_ID: &str = "gamekit-deckbuilder";
pub(crate) const PROTOCOL_VERSION: &str = "1";
pub(crate) const BUILD_ID: &str = env!("CARGO_PKG_VERSION");
const PROTOCOL_SCHEMA: &str = "gamekit-deckbuilder/v1;seatless-commands;private-target-snapshots";
const TAILSCALE_REFRESH: Duration = Duration::from_secs(10);

pub(crate) struct DeckNetworkPlugin;

impl Plugin for DeckNetworkPlugin {
    fn build(&self, app: &mut App) {
        if !app
            .world()
            .contains_resource::<ReconnectCredentialStorage>()
        {
            let storage = directories::ProjectDirs::from("dev", "Gamekit", "Deckbuilder")
                .map_or_else(
                    || ReconnectCredentialStorage::new(MemoryReconnectCredentialStore::default()),
                    |project| {
                        ReconnectCredentialStorage::new(AtomicFileReconnectCredentialStore::new(
                            project.data_local_dir().join("reconnect.json"),
                        ))
                    },
                );
            app.insert_resource(storage);
        }
        app.add_plugins((GameMultiplayerPlugin, DiscoveryPlugin))
            .init_resource::<DeckNetworkState>()
            .insert_resource(ExpectedSession {
                game_id: GAME_ID.to_owned(),
                protocol_version: PROTOCOL_VERSION.to_owned(),
                build_id: BUILD_ID.to_owned(),
            })
            .add_systems(
                PreUpdate,
                (
                    send_pending_hello,
                    handle_client_hellos,
                    handle_remote_requests,
                    receive_welcome,
                    receive_refusal,
                    receive_results,
                    receive_snapshots,
                    receive_closed,
                )
                    .chain(),
            )
            .add_systems(Update, (poll_discovery, poll_host_discovery))
            .add_systems(Update, finish_pending_server_close)
            .add_observer(on_connected_client_removed)
            .add_observer(on_transport_disconnected);
        register_protocol(app);
    }
}

fn register_protocol(app: &mut App) {
    app.world_mut()
        .resource_mut::<ProtocolHasher>()
        .add_custom(PROTOCOL_SCHEMA);
    app.add_client_message::<DeckClientHello>(Channel::Ordered)
        .add_client_message::<GameRequest>(Channel::Ordered)
        .add_server_message::<DeckWelcome>(Channel::Ordered)
        .make_message_independent::<DeckWelcome>()
        .add_server_message::<DeckAdmissionRefusal>(Channel::Ordered)
        .make_message_independent::<DeckAdmissionRefusal>()
        .add_server_message::<GameCommandResult>(Channel::Ordered)
        .make_message_independent::<GameCommandResult>()
        .add_server_message::<GameSnapshot>(Channel::Ordered)
        .make_message_independent::<GameSnapshot>()
        .add_server_message::<DeckSessionClosed>(Channel::Ordered)
        .make_message_independent::<DeckSessionClosed>();
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum NetworkRole {
    None,
    Solo,
    Host,
    Guest,
}

#[derive(Resource, Debug)]
pub(crate) struct DeckNetworkState {
    pub(crate) role: NetworkRole,
    pub(crate) latest: Option<GameSnapshot>,
    pub(crate) notice: Option<String>,
    pub(crate) admitted: bool,
    pub(crate) session_id: Option<SessionId>,
}

impl Default for DeckNetworkState {
    fn default() -> Self {
        Self {
            role: NetworkRole::None,
            latest: None,
            notice: None,
            admitted: false,
            session_id: None,
        }
    }
}

#[derive(Resource)]
struct HostedSession {
    security: SessionSecurityAuthority,
    verifier: SessionPasswordVerifier,
    guest_peer: Option<PeerId>,
    encoded_code: EncodedConnectionCode,
    target: DiscoveredDirectTarget,
    metadata: SessionMetadata,
    mdns: Option<MdnsAdvertiser>,
    tailnet: Option<TailnetResponder>,
    server_entity: Entity,
}

impl fmt::Debug for HostedSession {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("HostedSession")
            .field("security", &self.security)
            .field("guest_peer", &self.guest_peer)
            .field("encoded_code", &self.encoded_code)
            .field("target", &self.target)
            .field("metadata", &self.metadata)
            .finish_non_exhaustive()
    }
}

#[derive(Resource, Debug)]
struct ActiveBrowser {
    mdns: Option<MdnsBrowser>,
    tailnet: Option<TailnetBrowser>,
    tailscale: TailscaleCli,
    next_tailnet_refresh: Duration,
}

#[derive(Resource)]
struct PendingHello {
    credential: DeckCredential,
    sent: bool,
}

#[derive(Resource, Debug)]
struct PendingServerClose {
    server_entity: Entity,
    frames_remaining: u8,
}

impl fmt::Debug for PendingHello {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("PendingHello")
            .field("credential", &self.credential)
            .field("sent", &self.sent)
            .finish()
    }
}

#[derive(Component, Debug, Clone, Copy)]
struct DeckAuthorized {
    peer: PeerId,
    seat: Seat,
}

#[derive(Message, Serialize, Deserialize)]
struct DeckClientHello {
    protocol: String,
    build: String,
    credential: DeckCredential,
}

impl fmt::Debug for DeckClientHello {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("DeckClientHello")
            .field("protocol", &self.protocol)
            .field("build", &self.build)
            .field("credential", &self.credential)
            .finish()
    }
}

#[derive(Serialize, Deserialize)]
enum DeckCredential {
    Invite(InviteToken),
    Password(PasswordWire),
    Reconnect(ReconnectCredential),
}

impl fmt::Debug for DeckCredential {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(match self {
            Self::Invite(_) => "Invite([REDACTED])",
            Self::Password(_) => "Password([REDACTED])",
            Self::Reconnect(_) => "Reconnect([REDACTED])",
        })
    }
}

#[derive(Serialize, Deserialize)]
struct PasswordWire(String);

impl Drop for PasswordWire {
    fn drop(&mut self) {
        self.0.zeroize();
    }
}

#[derive(Message, Debug, Clone, Copy, Serialize, Deserialize)]
struct DeckWelcome {
    session_id: SessionId,
    peer_id: PeerId,
    seat: Seat,
    reconnect_credential: ReconnectCredential,
}

#[derive(Message, Debug, Clone, Copy, Serialize, Deserialize)]
struct DeckAdmissionRefusal;

#[derive(Message, Debug, Clone, Copy, Serialize, Deserialize)]
struct DeckSessionClosed;

pub(crate) struct HostConfiguration {
    pub(crate) session_name: String,
    pub(crate) password: String,
    pub(crate) advertised_host: String,
    pub(crate) port: u16,
    pub(crate) discover_lan: bool,
    pub(crate) discover_tailnet: bool,
}

pub(crate) fn start_solo(world: &mut World) {
    let authority = DeckAuthority::solo();
    let snapshot = authority.snapshot(Seat::Host);
    world.insert_resource(authority);
    let mut state = world.resource_mut::<DeckNetworkState>();
    state.role = NetworkRole::Solo;
    state.latest = Some(snapshot);
    state.admitted = true;
    state.session_id = None;
    state.notice = None;
}

pub(crate) fn start_host(world: &mut World, config: HostConfiguration) -> Result<(), String> {
    let endpoint = DirectEndpoint::new(config.advertised_host, config.port)
        .map_err(|error| error.to_string())?;
    let password = SessionPassword::new(config.password).map_err(|error| error.to_string())?;
    let verifier = SessionPasswordVerifier::new(&password).map_err(|error| error.to_string())?;
    let security = SessionSecurityAuthority::new();
    let prepared =
        PreparedDirectHost::new(endpoint, security.session_id(), security.invite_token())
            .map_err(|error| error.to_string())?;
    let code = prepared.connection_code().clone();
    let target = DiscoveredDirectTarget {
        session_id: code.session_id,
        endpoint: code.endpoint.clone(),
        certificate_fingerprint: code.certificate_fingerprint,
        certificate_expires_unix_seconds: code.certificate_expires_unix_seconds,
    };
    let metadata = SessionMetadata::new(
        GAME_ID,
        PROTOCOL_VERSION,
        BUILD_ID,
        config.session_name,
        1,
        2,
        true,
    )
    .map_err(|error| error.to_string())?;
    let server_entity = prepared.open(world);

    let mut notices = Vec::new();
    let mdns = if config.discover_lan {
        match MdnsSessionAdvertisement::new(metadata.clone(), target.clone())
            .and_then(MdnsAdvertiser::start)
        {
            Ok(advertiser) => Some(advertiser),
            Err(error) => {
                notices.push(format!("LAN discovery unavailable: {error}"));
                None
            }
        }
    } else {
        None
    };
    let tailnet = if config.discover_tailnet {
        let cli = TailscaleCli::default();
        match cli
            .local_addresses()
            .and_then(|addresses| {
                addresses
                    .first()
                    .copied()
                    .ok_or(bevy_game_discovery::TailnetDiscoveryError::ClientDisconnected)
            })
            .and_then(|address| TailnetResponder::bind(address, metadata.clone(), target.clone()))
        {
            Ok(responder) => Some(responder),
            Err(error) => {
                notices.push(format!("Tailnet discovery unavailable: {error}"));
                None
            }
        }
    } else {
        None
    };
    let encoded_code = code.encode();
    let authority = DeckAuthority::lobby();
    let snapshot = authority.snapshot(Seat::Host);
    world.insert_resource(authority);
    let session_id = security.session_id();
    world.insert_resource(HostedSession {
        security,
        verifier,
        guest_peer: None,
        encoded_code,
        target,
        metadata,
        mdns,
        tailnet,
        server_entity,
    });
    let mut state = world.resource_mut::<DeckNetworkState>();
    state.role = NetworkRole::Host;
    state.latest = Some(snapshot);
    state.admitted = true;
    state.session_id = Some(session_id);
    state.notice = (!notices.is_empty()).then(|| notices.join(" "));
    Ok(())
}

pub(crate) fn hosted_code(world: &World) -> Option<String> {
    world
        .get_resource::<HostedSession>()
        .map(|hosted| hosted.encoded_code.expose_for_sharing().to_owned())
}

pub(crate) fn start_browser(world: &mut World, discover_tailnet: bool) {
    let mut notices = Vec::new();
    let mdns = match MdnsBrowser::start() {
        Ok(browser) => Some(browser),
        Err(error) => {
            let reason = format!("LAN discovery unavailable: {error}");
            world.write_message(DiscoveryObservation::Unavailable {
                provider: bevy_game_discovery::DiscoveryProviderId::MDNS,
                reason: reason.clone(),
            });
            notices.push(reason);
            None
        }
    };
    let tailscale = TailscaleCli::default();
    let tailnet = if discover_tailnet {
        match tailscale
            .local_addresses()
            .and_then(|addresses| {
                addresses
                    .first()
                    .copied()
                    .ok_or(bevy_game_discovery::TailnetDiscoveryError::ClientDisconnected)
            })
            .and_then(TailnetBrowser::bind)
        {
            Ok(browser) => Some(browser),
            Err(error) => {
                let reason = format!("Tailnet discovery unavailable: {error}");
                world.write_message(DiscoveryObservation::Unavailable {
                    provider: bevy_game_discovery::DiscoveryProviderId::TAILSCALE,
                    reason: reason.clone(),
                });
                notices.push(reason);
                None
            }
        }
    } else {
        None
    };
    world.insert_resource(ActiveBrowser {
        mdns,
        tailnet,
        tailscale,
        next_tailnet_refresh: Duration::ZERO,
    });
    world.resource_mut::<DeckNetworkState>().notice =
        (!notices.is_empty()).then(|| notices.join(" "));
}

pub(crate) fn stop_browser(world: &mut World) {
    world.remove_resource::<ActiveBrowser>();
}

pub(crate) fn start_direct_join(world: &mut World, encoded: &str) -> Result<(), String> {
    let code = DirectConnectionCode::parse(encoded).map_err(|error| error.to_string())?;
    let prepared = PreparedDirectJoin::new(&code).map_err(|error| error.to_string())?;
    let credential = DeckCredential::Invite(prepared.invite_token());
    prepared.connect(world);
    begin_guest_connect(world, credential);
    Ok(())
}

pub(crate) fn start_discovered_join(
    world: &mut World,
    route: &DiscoveryJoinRoute,
    password: String,
) -> Result<(), String> {
    let password = SessionPassword::new(password).map_err(|error| error.to_string())?;
    let wire = PasswordWire(password.expose_for_encrypted_transport().to_owned());
    route
        .connect_preferred(world)
        .map_err(|error| error.to_string())?;
    begin_guest_connect(world, DeckCredential::Password(wire));
    Ok(())
}

pub(crate) fn reconnect(world: &mut World) -> Result<(), String> {
    let storage = world
        .get_resource::<ReconnectCredentialStorage>()
        .cloned()
        .ok_or_else(|| "Reconnect storage is unavailable.".to_owned())?;
    let stored = storage
        .store()
        .load()
        .map_err(|error| error.to_string())?
        .ok_or_else(|| "No reserved session is stored.".to_owned())?;
    let prepared = PreparedDirectReconnect::new(&stored.endpoint_binding)
        .map_err(|error| error.to_string())?;
    prepared.connect(world);
    begin_guest_connect(
        world,
        DeckCredential::Reconnect(stored.reconnect_credential),
    );
    Ok(())
}

fn begin_guest_connect(world: &mut World, credential: DeckCredential) {
    world.insert_resource(PendingHello {
        credential,
        sent: false,
    });
    world.remove_resource::<DeckAuthority>();
    let mut state = world.resource_mut::<DeckNetworkState>();
    state.role = NetworkRole::Guest;
    state.latest = None;
    state.admitted = false;
    state.session_id = None;
    state.notice = Some("Connecting to the host…".to_owned());
}

pub(crate) fn submit_command(world: &mut World, command: GameCommand) {
    let request = GameRequest {
        request_id: RequestId::generate(),
        command,
    };
    let role = world.resource::<DeckNetworkState>().role;
    match role {
        NetworkRole::Solo | NetworkRole::Host => {
            let source = Seat::Host;
            let result = world.resource_mut::<DeckAuthority>().apply(source, request);
            if matches!(result.outcome, crate::domain::CommandOutcome::Accepted) {
                publish_snapshots(world);
            } else {
                world.resource_mut::<DeckNetworkState>().notice =
                    Some(format!("Action refused: {:?}", result.outcome));
            }
        }
        NetworkRole::Guest => {
            world.write_message(request);
        }
        NetworkRole::None => {}
    }
}

pub(crate) fn close_session(world: &mut World) {
    if let Some(mut hosted) = world.remove_resource::<HostedSession>() {
        hosted.security.close();
        world.write_message(ToClients {
            targets: SendTargets::All,
            message: DeckSessionClosed,
        });
        world.insert_resource(PendingServerClose {
            server_entity: hosted.server_entity,
            frames_remaining: 2,
        });
    }
    world.remove_resource::<PendingHello>();
    world.remove_resource::<DeckAuthority>();
    let mut state = world.resource_mut::<DeckNetworkState>();
    state.role = NetworkRole::None;
    state.latest = None;
    state.admitted = false;
    state.session_id = None;
    state.notice = None;
}

fn send_pending_hello(
    state: Option<Res<State<ClientState>>>,
    mut pending: Option<ResMut<PendingHello>>,
    mut hellos: MessageWriter<DeckClientHello>,
) {
    let (Some(state), Some(pending)) = (state, pending.as_mut()) else {
        return;
    };
    if *state.get() != ClientState::Connected || pending.sent {
        return;
    }
    let credential = std::mem::replace(
        &mut pending.credential,
        DeckCredential::Invite(InviteToken::from_bytes([0; 16])),
    );
    hellos.write(DeckClientHello {
        protocol: PROTOCOL_VERSION.to_owned(),
        build: BUILD_ID.to_owned(),
        credential,
    });
    pending.sent = true;
}

fn handle_client_hellos(
    time: Res<Time<Real>>,
    mut hellos: MessageReader<FromClient<DeckClientHello>>,
    mut hosted: Option<ResMut<HostedSession>>,
    mut authority: Option<ResMut<DeckAuthority>>,
    connected: Query<&PeerAddr, With<ConnectedClient>>,
    mut commands: Commands,
    mut accepted: MessageWriter<ToClients<DeckWelcome>>,
    mut refused: MessageWriter<ToClients<DeckAdmissionRefusal>>,
    mut snapshots: MessageWriter<ToClients<GameSnapshot>>,
    mut state: ResMut<DeckNetworkState>,
) {
    let (Some(hosted), Some(authority)) = (hosted.as_mut(), authority.as_mut()) else {
        return;
    };
    for hello in hellos.read() {
        let Some(connection) = hello.client_id.entity() else {
            continue;
        };
        if hello.protocol != PROTOCOL_VERSION || hello.build != BUILD_ID {
            refused.write(ToClients {
                targets: SendTargets::Single(hello.client_id),
                message: DeckAdmissionRefusal,
            });
            continue;
        }
        let grant = match &hello.credential {
            DeckCredential::Invite(token) => hosted
                .security
                .authenticate(connection.to_bits(), AdmissionCredential::Invite(*token)),
            DeckCredential::Reconnect(credential) => hosted.security.authenticate(
                connection.to_bits(),
                AdmissionCredential::Reconnect(*credential),
            ),
            DeckCredential::Password(password) => {
                let source = connected
                    .get(connection)
                    .map(|address| address.0.ip())
                    .unwrap_or(IpAddr::V4(std::net::Ipv4Addr::UNSPECIFIED));
                SessionPassword::new(password.0.clone())
                    .map_err(|_error| bevy_game_multiplayer::AdmissionError::InvalidInvite)
                    .and_then(|password| {
                        hosted
                            .verifier
                            .verify(source, time.elapsed(), &password)
                            .map_err(|_error| bevy_game_multiplayer::AdmissionError::InvalidInvite)
                    })
                    .and_then(|()| hosted.security.authenticate_external(connection.to_bits()))
            }
        };
        let Ok(grant) = grant else {
            refused.write(ToClients {
                targets: SendTargets::Single(hello.client_id),
                message: DeckAdmissionRefusal,
            });
            continue;
        };
        if hosted
            .guest_peer
            .is_some_and(|reserved| reserved != grant.peer)
        {
            hosted.security.revoke_peer(grant.peer);
            refused.write(ToClients {
                targets: SendTargets::Single(hello.client_id),
                message: DeckAdmissionRefusal,
            });
            continue;
        }
        hosted.guest_peer = Some(grant.peer);
        authority.set_connected(Seat::Guest, true);
        if let Err(error) = refresh_host_listing(hosted, 2) {
            state.notice = Some(error);
        }
        state.latest = Some(authority.snapshot(Seat::Host));
        commands.entity(connection).insert((
            AuthorizedClient,
            DeckAuthorized {
                peer: grant.peer,
                seat: Seat::Guest,
            },
        ));
        accepted.write(ToClients {
            targets: SendTargets::Single(hello.client_id),
            message: DeckWelcome {
                session_id: hosted.security.session_id(),
                peer_id: grant.peer,
                seat: Seat::Guest,
                reconnect_credential: grant.reconnect_credential,
            },
        });
        snapshots.write(ToClients {
            targets: SendTargets::Single(hello.client_id),
            message: authority.snapshot(Seat::Guest),
        });
    }
}

fn handle_remote_requests(
    mut requests: MessageReader<FromClient<GameRequest>>,
    clients: Query<&DeckAuthorized, With<AuthorizedClient>>,
    mut authority: Option<ResMut<DeckAuthority>>,
    mut results: MessageWriter<ToClients<GameCommandResult>>,
    mut snapshots: MessageWriter<ToClients<GameSnapshot>>,
    client_query: Query<(Entity, &DeckAuthorized), With<AuthorizedClient>>,
    mut state: ResMut<DeckNetworkState>,
) {
    let Some(authority) = authority.as_mut() else {
        return;
    };
    for request in requests.read() {
        let Some(connection) = request.client_id.entity() else {
            continue;
        };
        let Ok(client) = clients.get(connection) else {
            continue;
        };
        let result = authority.apply(client.seat, **request);
        results.write(ToClients {
            targets: SendTargets::Single(request.client_id),
            message: result,
        });
        if matches!(result.outcome, crate::domain::CommandOutcome::Accepted) {
            state.latest = Some(authority.snapshot(Seat::Host));
            for (entity, authorized) in &client_query {
                snapshots.write(ToClients {
                    targets: SendTargets::Single(ClientId::from(entity)),
                    message: authority.snapshot(authorized.seat),
                });
            }
        }
    }
}

fn receive_welcome(
    mut welcome: MessageReader<DeckWelcome>,
    binding: Option<Res<ReconnectEndpointBinding>>,
    storage: Option<Res<ReconnectCredentialStorage>>,
    mut state: ResMut<DeckNetworkState>,
    mut commands: Commands,
) {
    for welcome in welcome.read() {
        state.admitted = true;
        state.session_id = Some(welcome.session_id);
        state.notice = None;
        commands.remove_resource::<PendingHello>();
        if let (Some(binding), Some(storage)) = (binding.as_ref(), storage.as_ref()) {
            let stored = StoredReconnectCredential {
                session_id: welcome.session_id,
                endpoint_binding: (**binding).clone(),
                peer_id: welcome.peer_id,
                reconnect_credential: welcome.reconnect_credential,
            };
            if storage.store().store_atomically(stored).is_err() {
                state.notice = Some(
                    "Connected, but reconnect storage failed; this seat cannot survive another restart."
                        .to_owned(),
                );
            }
        }
    }
}

fn receive_refusal(
    mut refusal: MessageReader<DeckAdmissionRefusal>,
    mut state: ResMut<DeckNetworkState>,
) {
    for _ in refusal.read() {
        state.notice = Some("The host refused session admission.".to_owned());
    }
}

fn receive_results(
    mut results: MessageReader<GameCommandResult>,
    mut state: ResMut<DeckNetworkState>,
) {
    for result in results.read() {
        if !matches!(result.outcome, crate::domain::CommandOutcome::Accepted) {
            state.notice = Some(format!("Action refused: {:?}", result.outcome));
        }
    }
}

fn receive_snapshots(
    mut snapshots: MessageReader<GameSnapshot>,
    mut state: ResMut<DeckNetworkState>,
) {
    for snapshot in snapshots.read() {
        if state
            .latest
            .as_ref()
            .is_none_or(|current| snapshot.sequence > current.sequence)
        {
            state.latest = Some(snapshot.clone());
        }
    }
}

fn receive_closed(
    mut closed: MessageReader<DeckSessionClosed>,
    storage: Option<Res<ReconnectCredentialStorage>>,
    mut state: ResMut<DeckNetworkState>,
) {
    for _ in closed.read() {
        if let (Some(storage), Some(session_id)) = (storage.as_ref(), state.session_id) {
            let _deleted = storage.store().delete_if_session(session_id);
        }
        state.role = NetworkRole::None;
        state.latest = None;
        state.admitted = false;
        state.session_id = None;
        state.notice = Some("The host closed the session.".to_owned());
    }
}

fn on_connected_client_removed(
    trigger: On<Remove, ConnectedClient>,
    authorized: Query<&DeckAuthorized>,
    mut hosted: Option<ResMut<HostedSession>>,
    mut authority: Option<ResMut<DeckAuthority>>,
    mut state: ResMut<DeckNetworkState>,
) {
    let entity = trigger.event_target();
    if let Some(hosted) = hosted.as_mut() {
        hosted.security.disconnect(entity.to_bits());
    }
    if let (Ok(client), Some(authority)) = (authorized.get(entity), authority.as_mut()) {
        let _peer = client.peer;
        authority.set_connected(client.seat, false);
        state.latest = Some(authority.snapshot(Seat::Host));
        if let Some(hosted) = hosted.as_mut() {
            if let Err(error) = refresh_host_listing(hosted, 1) {
                state.notice = Some(error);
            }
        }
    }
}

fn on_transport_disconnected(_trigger: On<Disconnected>, mut state: ResMut<DeckNetworkState>) {
    if state.role == NetworkRole::Guest {
        state.admitted = false;
        state.notice =
            Some("Connection lost. Your reserved seat can be reclaimed with Reconnect.".to_owned());
    }
}

fn finish_pending_server_close(
    mut commands: Commands,
    mut pending: Option<ResMut<PendingServerClose>>,
) {
    let Some(pending) = pending.as_mut() else {
        return;
    };
    if pending.frames_remaining > 0 {
        pending.frames_remaining -= 1;
        return;
    }
    commands.trigger(Close::new(pending.server_entity, "host closed the session"));
    commands.remove_resource::<PendingServerClose>();
}

fn refresh_host_listing(hosted: &mut HostedSession, claimed_players: u8) -> Result<(), String> {
    let metadata = SessionMetadata::new(
        hosted.metadata.game_id(),
        hosted.metadata.protocol_version(),
        hosted.metadata.build_id(),
        hosted.metadata.display_name(),
        claimed_players,
        hosted.metadata.player_capacity(),
        true,
    )
    .map_err(|error| error.to_string())?;
    if let Some(mdns) = hosted.mdns.as_mut() {
        let advertisement = MdnsSessionAdvertisement::new(metadata.clone(), hosted.target.clone())
            .map_err(|error| error.to_string())?;
        mdns.refresh(advertisement)
            .map_err(|error| error.to_string())?;
    }
    if let Some(tailnet) = hosted.tailnet.as_mut() {
        tailnet
            .refresh_metadata(metadata.clone())
            .map_err(|error| error.to_string())?;
    }
    hosted.metadata = metadata;
    Ok(())
}

fn publish_snapshots(world: &mut World) {
    let host = world.resource::<DeckAuthority>().snapshot(Seat::Host);
    world.resource_mut::<DeckNetworkState>().latest = Some(host);
    let guests = {
        let mut query = world.query_filtered::<(Entity, &DeckAuthorized), With<AuthorizedClient>>();
        query
            .iter(world)
            .map(|(entity, client)| (entity, client.seat))
            .collect::<Vec<_>>()
    };
    for (entity, seat) in guests {
        let snapshot = world.resource::<DeckAuthority>().snapshot(seat);
        world.write_message(ToClients {
            targets: SendTargets::Single(ClientId::from(entity)),
            message: snapshot,
        });
    }
}

fn poll_discovery(
    time: Res<Time<Real>>,
    mut browser: Option<ResMut<ActiveBrowser>>,
    mut observations: MessageWriter<DiscoveryObservation>,
    mut state: ResMut<DeckNetworkState>,
) {
    let Some(browser) = browser.as_mut() else {
        return;
    };
    if let Some(mdns) = browser.mdns.as_mut() {
        match mdns.poll(time.elapsed(), current_unix_seconds()) {
            Ok(found) => {
                for observation in found {
                    observations.write(observation);
                }
            }
            Err(error) => {
                browser.mdns = None;
                let reason = format!("LAN discovery stopped: {error}");
                observations.write(DiscoveryObservation::Failed {
                    provider: bevy_game_discovery::DiscoveryProviderId::MDNS,
                    reason: reason.clone(),
                });
                state.notice = Some(reason);
            }
        }
    }
    if time.elapsed() >= browser.next_tailnet_refresh {
        browser.next_tailnet_refresh = time.elapsed() + TAILSCALE_REFRESH;
        let peers = browser.tailscale.peers();
        if let (Some(tailnet), Ok(peers)) = (browser.tailnet.as_mut(), peers) {
            if let Err(error) = tailnet.refresh(&peers, GAME_ID) {
                let reason = format!("Tailnet discovery refresh failed: {error}");
                observations.write(DiscoveryObservation::Failed {
                    provider: bevy_game_discovery::DiscoveryProviderId::TAILSCALE,
                    reason: reason.clone(),
                });
                state.notice = Some(reason);
            }
        }
    }
    if let Some(tailnet) = browser.tailnet.as_ref() {
        match tailnet.poll(time.elapsed(), current_unix_seconds()) {
            Ok(found) => {
                for observation in found {
                    observations.write(observation);
                }
            }
            Err(error) => {
                let reason = format!("Tailnet discovery stopped: {error}");
                observations.write(DiscoveryObservation::Failed {
                    provider: bevy_game_discovery::DiscoveryProviderId::TAILSCALE,
                    reason: reason.clone(),
                });
                state.notice = Some(reason);
            }
        }
    }
}

fn poll_host_discovery(hosted: Option<Res<HostedSession>>, mut state: ResMut<DeckNetworkState>) {
    let Some(hosted) = hosted else { return };
    if let Some(advertiser) = hosted.mdns.as_ref() {
        if let Err(error) = advertiser.poll_health() {
            state.notice = Some(format!("LAN advertisement stopped: {error}"));
        }
    }
    if let Some(responder) = hosted.tailnet.as_ref() {
        if let Err(error) = responder.poll(32) {
            state.notice = Some(format!("Tailnet responder stopped: {error}"));
        }
    }
}

#[cfg(test)]
pub(crate) fn latest_snapshot(world: &World) -> Option<&GameSnapshot> {
    world.resource::<DeckNetworkState>().latest.as_ref()
}

fn current_unix_seconds() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |duration| duration.as_secs())
}

#[cfg(test)]
mod tests {
    use super::*;
    use bevy_game_multiplayer::{InMemorySessionLink, ReconnectCredentialStore as _};

    #[test]
    fn wire_snapshots_are_target_specific() {
        let authority = DeckAuthority::solo();
        let host = serde_json::to_string(&authority.snapshot(Seat::Host))
            .expect("host snapshot serializes");
        let guest = serde_json::to_string(&authority.snapshot(Seat::Guest))
            .expect("guest snapshot serializes");
        assert_ne!(host, guest);
        assert!(host.contains("\"recipient\":\"Host\""));
        assert!(guest.contains("\"recipient\":\"Guest\""));
    }

    #[test]
    fn two_app_link_carries_commands_and_fresh_snapshot_without_ids() {
        let (host_link, guest_link) = InMemorySessionLink::pair(8, 4096);
        let request = GameRequest {
            request_id: RequestId::fixture(1),
            command: GameCommand::SetReady(true),
        };
        guest_link
            .send(serde_json::to_vec(&request).expect("request serializes"))
            .expect("guest sends");
        let bytes = host_link
            .try_receive()
            .expect("host receives")
            .expect("request exists");
        let decoded: GameRequest = serde_json::from_slice(&bytes).expect("request decodes");
        let mut authority = DeckAuthority::lobby();
        authority.set_connected(Seat::Guest, true);
        authority.apply(Seat::Guest, decoded);
        host_link
            .send(
                serde_json::to_vec(&authority.snapshot(Seat::Guest)).expect("snapshot serializes"),
            )
            .expect("host sends snapshot");
        let received: GameSnapshot = serde_json::from_slice(
            &guest_link
                .try_receive()
                .expect("guest receives")
                .expect("snapshot exists"),
        )
        .expect("snapshot decodes");
        assert_eq!(received.recipient, Seat::Guest);
    }

    #[test]
    fn rotating_credential_can_be_persisted_for_fresh_client_app() {
        let store = bevy_game_multiplayer::MemoryReconnectCredentialStore::default();
        let stored = StoredReconnectCredential {
            session_id: SessionId::from_bytes([1; 16]),
            endpoint_binding: ReconnectEndpointBinding::new(
                DirectEndpoint::new("host.local", 7777).expect("endpoint"),
                bevy_game_multiplayer::CertificateFingerprint::from_bytes([2; 32]),
                2_000_000_000,
            )
            .expect("binding"),
            peer_id: PeerId::from_bytes([3; 16]),
            reconnect_credential: ReconnectCredential::from_bytes([4; 32]),
        };
        store
            .store_atomically(stored.clone())
            .expect("store succeeds");
        assert_eq!(store.load().expect("load succeeds"), Some(stored));
    }
}
