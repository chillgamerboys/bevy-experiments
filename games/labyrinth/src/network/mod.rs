//! Game-owned admission, authority, discovery, and projection composition.

mod admission;
mod discovery;
mod protocol;
mod start;
#[cfg(test)]
mod tests;

use aeronet::io::{
    connection::{Disconnect, Disconnected, PeerAddr},
    server::{Close, Closed as TransportClosed},
};
use bevy::prelude::*;
use bevy_game_discovery::{
    DiscoveryPlugin, DiscoveryRegistry, ExpectedSession, SessionMetadata, SessionPasswordVerifier,
};
use bevy_game_multiplayer::*;
use bevy_game_session::*;
use bevy_replicon::prelude::*;
use sha2::{Digest as _, Sha256};
use std::{
    collections::{BTreeMap, BTreeSet},
    sync::{mpsc, Arc, Mutex},
    time::{Duration, Instant, SystemTime, UNIX_EPOCH},
};

use crate::{
    session::{
        GameRequest, PartyAuthority, RequestResult, SessionCommand, SessionSnapshot,
        PLAYER_CAPACITY,
    },
    view::*,
};
use protocol::*;

const GUEST_CAPACITY: usize = labyrinth_rules::PARTY_SIZE - 1;
const PASSWORD_WORKERS: usize = GUEST_CAPACITY;
const MAX_HANDSHAKES: usize = GUEST_CAPACITY * 4;
const ADMISSION_TIMEOUT: Duration = Duration::from_secs(15);

use std::sync::atomic::{AtomicUsize, Ordering};

#[cfg(test)]
mod worker_tests;

/// Permits follow the work, not a connection or cancelled response receiver.
struct WorkerPermit(Arc<AtomicUsize>);
impl WorkerPermit {
    fn acquire(counter: &Arc<AtomicUsize>, limit: usize) -> Option<Self> {
        counter
            .fetch_update(Ordering::AcqRel, Ordering::Acquire, |count| {
                (count < limit).then_some(count.saturating_add(1))
            })
            .ok()?;
        Some(Self(Arc::clone(counter)))
    }
}
impl Drop for WorkerPermit {
    fn drop(&mut self) {
        self.0.fetch_sub(1, Ordering::AcqRel);
    }
}
#[derive(Resource, Default)]
struct HostPreparationBudget(Arc<AtomicUsize>);
#[derive(Resource, Default)]
struct PasswordWorkBudget(Arc<AtomicUsize>);

pub(crate) struct LabyrinthNetworkPlugin;

impl Plugin for LabyrinthNetworkPlugin {
    fn build(&self, app: &mut App) {
        if !app
            .world()
            .contains_resource::<ReconnectCredentialStorage>()
        {
            app.insert_resource(ReconnectCredentialStorage::new(
                MemoryReconnectCredentialStore::default(),
            ));
        }
        app.init_resource::<LabyrinthView>()
            .add_message::<LabyrinthIntent>()
            .add_plugins((GameMultiplayerPlugin, DiscoveryPlugin))
            .init_resource::<Runtime>()
            .init_resource::<HostPreparationBudget>()
            .init_resource::<PasswordWorkBudget>()
            .init_resource::<DisconnectQueue>()
            .init_resource::<ListenerClosedQueue>()
            .insert_resource(ExpectedSession {
                game_id: GAME_ID.into(),
                protocol_version: PROTOCOL.into(),
                build_id: fingerprint_text(),
            })
            .add_systems(
                PreUpdate,
                network_tick.in_set(MultiplayerSystems::GameAuthority),
            )
            .add_systems(PostUpdate, send_hello.in_set(MultiplayerSystems::Send))
            .add_systems(
                Update,
                discovery::poll.before(bevy_game_discovery::DiscoverySystems::Maintain),
            )
            .add_systems(
                Update,
                discovery::project.after(bevy_game_discovery::DiscoverySystems::Maintain),
            )
            .add_observer(disconnected)
            .add_observer(listener_closed);
        app.world_mut()
            .resource_mut::<ProtocolHasher>()
            .add_custom(SCHEMA);
        app.add_client_message::<Hello>(Channel::Ordered)
            .add_client_message::<Persisted>(Channel::Ordered)
            .add_client_message::<LeaveSession>(Channel::Ordered)
            .add_client_message::<GameRequest>(Channel::Ordered)
            .add_server_message::<Offer>(Channel::Ordered)
            .make_message_independent::<Offer>()
            .add_server_message::<Admitted>(Channel::Ordered)
            .make_message_independent::<Admitted>()
            .add_server_message::<Refusal>(Channel::Ordered)
            .make_message_independent::<Refusal>()
            .add_server_message::<Closed>(Channel::Ordered)
            .make_message_independent::<Closed>()
            .add_server_message::<RequestResult>(Channel::Ordered)
            .make_message_independent::<RequestResult>()
            .add_server_message::<SnapshotEnvelope>(Channel::Ordered)
            .make_message_independent::<SnapshotEnvelope>();
    }
}

#[derive(Default, Clone, Copy, PartialEq, Eq)]
enum Role {
    #[default]
    None,
    Local,
    Host,
    Guest,
}

#[derive(Resource)]
struct Runtime {
    role: Role,
    connection: Option<Entity>,
    session: Option<SessionId>,
    attempt: Option<SessionId>,
    connecting_since: Option<Instant>,
    player: Option<u8>,
    credential: Option<Credential>,
    latest: Option<SessionSnapshot>,
    sequence: u64,
    admitted: bool,
    started: Instant,
    next_enemy: Instant,
    published: u64,
    pending_disconnect: Option<(Entity, Instant)>,
    pending_close: Option<(Entity, Instant)>,
}
impl Default for Runtime {
    fn default() -> Self {
        Self {
            role: Role::None,
            connection: None,
            session: None,
            attempt: None,
            connecting_since: None,
            player: None,
            credential: None,
            latest: None,
            sequence: 1,
            admitted: false,
            started: Instant::now(),
            next_enemy: Instant::now(),
            published: 0,
            pending_disconnect: None,
            pending_close: None,
        }
    }
}

#[derive(Resource, Default)]
struct DisconnectQueue(Vec<Entity>);

#[derive(Resource, Default)]
struct ListenerClosedQueue(Vec<Entity>);

#[derive(Resource)]
struct Hosted {
    security: SessionAdmissionAuthority,
    server: Entity,
    template: DirectConnectionCode,
    invites: Vec<InviteToken>,
    verifier: Option<Arc<Mutex<SessionPasswordVerifier>>>,
    password_jobs: Arc<AtomicUsize>,
    connections: BTreeMap<Entity, (PeerId, u8)>,
    seen: BTreeSet<Entity>,
    attempts: BTreeMap<Entity, SessionId>,
    pending: BTreeMap<Entity, (Instant, Mutex<mpsc::Receiver<bool>>)>,
    rejected: BTreeMap<Entity, Instant>,
    observed: BTreeMap<Entity, Instant>,
    metadata: SessionMetadata,
    providers: discovery::HostProviders,
}

fn fingerprint() -> [u8; 32] {
    // Catalog fingerprint supplied by the pure rules crate; includes wire schema.
    let mut digest = Sha256::new();
    digest.update(SCHEMA.as_bytes());
    digest.update(labyrinth_rules::rules_fingerprint().as_bytes());
    digest.finalize().into()
}
fn fingerprint_text() -> String {
    fingerprint().iter().map(|b| format!("{b:02x}")).collect()
}
fn unix_now() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
}
fn now(world: &World) -> Duration {
    world.resource::<Runtime>().started.elapsed()
}
fn notice(world: &mut World, message: impl Into<String>) {
    world.resource_mut::<LabyrinthView>().notice = Some(message.into());
}
fn drain<M: Message>(world: &mut World) -> Vec<M> {
    world.resource_mut::<Messages<M>>().drain().collect()
}
fn to_client<M: Message>(world: &mut World, entity: Entity, message: M) {
    world.write_message(ToClients {
        targets: SendTargets::Single(ClientId::from(entity)),
        message,
    });
}

fn network_tick(world: &mut World) {
    for entity in std::mem::take(&mut world.resource_mut::<ListenerClosedQueue>().0) {
        if world
            .get_resource::<Hosted>()
            .is_some_and(|host| host.server == entity)
        {
            start::close(world);
            notice(world, "Host listener closed; check the UDP port and network interface before hosting again.");
        }
    }
    start::finish_host(world);
    receive(world);
    admission::host_messages(world);
    for intent in drain::<LabyrinthIntent>(world).into_iter().take(64) {
        if let Err(error) = handle_intent(world, intent) {
            notice(world, error);
        }
    }
    if let Some(mut authority) = world.remove_resource::<PartyAuthority>() {
        if Instant::now() >= world.resource::<Runtime>().next_enemy && authority.advance_enemy() {
            world.resource_mut::<Runtime>().next_enemy =
                Instant::now() + Duration::from_millis(220);
        }
        world.insert_resource(authority);
    }
    publish(world);
    finish_close(world);
    if world
        .resource::<Runtime>()
        .connecting_since
        .is_some_and(|at| at.elapsed() > Duration::from_secs(20))
    {
        start::disconnect_guest(world);
        notice(world, "Connection/admission timed out. Check the host address and UDP firewall, then retry or reconnect.");
    }
}

fn send_hello(world: &mut World) {
    if *world.resource::<State<ClientState>>().get() != ClientState::Connected {
        return;
    }
    let (credential, session, attempt) = {
        let mut runtime = world.resource_mut::<Runtime>();
        (runtime.credential.take(), runtime.session, runtime.attempt)
    };
    if let (Some(credential), Some(session), Some(attempt)) = (credential, session, attempt) {
        world.write_message(Hello {
            session,
            attempt,
            fingerprint: fingerprint(),
            credential,
        });
    }
}

fn receive(world: &mut World) {
    for offer in drain::<Offer>(world) {
        let runtime = world.resource::<Runtime>();
        if runtime.connection.is_none()
            || runtime.session != Some(offer.session)
            || runtime.attempt != Some(offer.attempt)
            || offer.slot == 0
            || offer.slot >= PLAYER_CAPACITY
            || !offer.peer.is_valid()
        {
            continue;
        }
        let Some(binding) = world.get_resource::<ReconnectEndpointBinding>().cloned() else {
            continue;
        };
        let stored = StoredReconnectCredential {
            session_id: offer.session,
            endpoint_binding: binding,
            peer_id: offer.peer,
            reconnect_credential: offer.credential,
        };
        if world
            .resource::<ReconnectCredentialStorage>()
            .store()
            .store_atomically(stored)
            .is_err()
        {
            start::disconnect_guest(world);
            notice(world, "Could not save reconnect credentials. Admission was not committed; retry after fixing profile storage.");
            continue;
        }
        world.write_message(Persisted {
            attempt: offer.attempt,
            credential: offer.credential,
        });
    }
    for admitted in drain::<Admitted>(world) {
        let mut runtime = world.resource_mut::<Runtime>();
        if runtime.connection.is_none()
            || runtime.session != Some(admitted.session)
            || runtime.attempt != Some(admitted.attempt)
            || admitted.slot == 0
            || admitted.slot >= PLAYER_CAPACITY
            || !admitted.peer.is_valid()
        {
            continue;
        }
        runtime.player = Some(admitted.slot);
        runtime.admitted = true;
        runtime.connecting_since = None;
        let connection = runtime.connection;
        if let Some(entity) = connection {
            if let Ok(mut entity) = world.get_entity_mut(entity) {
                entity.insert(AuthenticatedPeer {
                    peer: admitted.peer,
                    reconnected: admitted.reconnected,
                });
            }
        }
        world.resource_mut::<LabyrinthView>().notice = None;
    }
    for refused in drain::<Refusal>(world) {
        if world.resource::<Runtime>().attempt != Some(refused.attempt) {
            continue;
        }
        start::disconnect_guest(world);
        notice(world, refused.reason.notice());
    }
    for result in drain::<RequestResult>(world) {
        if let Some(rejection) = result.rejection {
            notice(world, rejection);
        }
    }
    for envelope in drain::<SnapshotEnvelope>(world) {
        let runtime = world.resource::<Runtime>();
        if runtime.role != Role::Guest
            || !runtime.admitted
            || runtime.connection.is_none()
            || runtime.attempt != Some(envelope.attempt)
        {
            continue;
        }
        let snapshot = envelope.snapshot;
        let identity = runtime
            .connection
            .and_then(|entity| world.get::<AuthenticatedPeer>(entity));
        if !runtime
            .player
            .zip(identity)
            .is_some_and(|(slot, identity)| {
                snapshot.validate_recipient(slot, identity.peer).is_ok()
            })
            || runtime
                .latest
                .as_ref()
                .is_some_and(|previous| snapshot.validate_successor(previous).is_err())
        {
            start::disconnect_guest(world);
            notice(
                world,
                "The host sent an invalid session snapshot; the saved profile was preserved.",
            );
            continue;
        }
        let mut runtime = world.resource_mut::<Runtime>();
        runtime.sequence = runtime.sequence.max(snapshot.next_sequence);
        if runtime
            .latest
            .as_ref()
            .is_none_or(|last| snapshot.revision >= last.revision)
        {
            runtime.latest = Some(snapshot);
        }
    }
    for closed in drain::<Closed>(world) {
        if world.resource::<Runtime>().attempt != Some(closed.attempt) {
            continue;
        }
        start::disconnect_guest(world);
        world.resource_mut::<Runtime>().latest = None;
        notice(
            world,
            "The host closed the session. Host restart recovery is not supported.",
        );
    }
}

fn handle_intent(world: &mut World, intent: LabyrinthIntent) -> Result<(), String> {
    match intent {
        LabyrinthIntent::StartLocal(seed) => {
            start::close(world);
            if world.resource::<Runtime>().pending_close.is_some() {
                return Err("Wait for the previous host to close.".into());
            }
            let mut authority = PartyAuthority::new(seed, true);
            let result = authority.apply(
                0,
                GameRequest {
                    sequence: 1,
                    encounter: 0,
                    decision: 0,
                    command: SessionCommand::Start,
                },
            );
            if let Some(error) = result.rejection {
                return Err(error);
            }
            world.insert_resource(authority);
            let mut runtime = world.resource_mut::<Runtime>();
            runtime.role = Role::Local;
            runtime.player = Some(0);
            runtime.admitted = true;
            runtime.published = 0;
        }
        LabyrinthIntent::Host(settings) => start::host(world, settings)?,
        LabyrinthIntent::JoinCode(code) => start::join_code(world, &code.0)?,
        LabyrinthIntent::Browse { tailnet } => discovery::browse(world, tailnet),
        LabyrinthIntent::StopBrowsing => {
            world.remove_resource::<discovery::Browser>();
        }
        LabyrinthIntent::JoinDiscovered {
            session,
            mut password,
        } => start::join_discovered(world, session, std::mem::take(&mut password.0))?,
        LabyrinthIntent::Reconnect => start::reconnect(world)?,
        LabyrinthIntent::Leave => start::close(world),
        LabyrinthIntent::SelectHero(hero) => submit(world, SessionCommand::ChooseHero(hero))?,
        LabyrinthIntent::Ready(ready) => submit(world, SessionCommand::Ready(ready))?,
        LabyrinthIntent::StartEncounter => submit(world, SessionCommand::Start)?,
        LabyrinthIntent::Rematch => submit(world, SessionCommand::Rematch)?,
        LabyrinthIntent::Combat {
            actor,
            action,
            encounter,
            decision,
        } => submit_at(
            world,
            SessionCommand::Act { actor, action },
            Some((encounter, decision)),
        )?,
        LabyrinthIntent::CopyInvite(index) => {
            let code = hosted_code(world, index).ok_or("No invitation is available.")?;
            world
                .get_resource_mut::<Clipboard>()
                .ok_or("System clipboard is unavailable.")?
                .set_text(code)
                .map_err(|_| "Clipboard write failed.")?;
            notice(
                world,
                "Private invitation copied. Share it only with the intended guest.",
            );
        }
        LabyrinthIntent::ReissueInvite(index) => admission::reissue(world, index)?,
    }
    Ok(())
}

fn submit(world: &mut World, command: SessionCommand) -> Result<(), String> {
    submit_at(world, command, None)
}

fn submit_at(
    world: &mut World,
    command: SessionCommand,
    boundary: Option<(u64, u64)>,
) -> Result<(), String> {
    let runtime = world.resource::<Runtime>();
    if !runtime.admitted {
        return Err("Join the session first.".into());
    }
    let role = runtime.role;
    let player = runtime.player.ok_or("No player is assigned.")?;
    let snapshot = if let Some(authority) = world.get_resource::<PartyAuthority>() {
        authority.snapshot(player)
    } else {
        runtime.latest.clone().ok_or("Waiting for host state.")?
    };
    let sequence = if world.contains_resource::<PartyAuthority>() {
        snapshot.next_sequence
    } else {
        runtime.sequence
    };
    let (encounter, decision) = boundary.unwrap_or_else(|| {
        (
            snapshot.encounter,
            snapshot.combat.as_ref().map_or(0, |combat| combat.turn_id),
        )
    });
    let request = GameRequest {
        sequence,
        encounter,
        decision,
        command,
    };
    if matches!(role, Role::Host | Role::Local) {
        let result = world
            .resource_mut::<PartyAuthority>()
            .apply(player, request);
        if let Some(error) = result.rejection {
            return Err(error);
        }
    } else {
        world.resource_mut::<Runtime>().sequence = sequence.saturating_add(1);
        world.write_message(request);
    }
    Ok(())
}

fn publish(world: &mut World) {
    if let Some(authority) = world.get_resource::<PartyAuthority>() {
        let snapshot = authority.snapshot(0);
        if snapshot.revision != world.resource::<Runtime>().published {
            let guests = world
                .get_resource::<Hosted>()
                .map(|host| {
                    host.connections
                        .iter()
                        .filter(|(entity, _)| {
                            host.security.is_connection_admitted(entity.to_bits())
                        })
                        .map(|(entity, (_, slot))| (*entity, *slot))
                        .collect::<Vec<_>>()
                })
                .unwrap_or_default();
            for (entity, slot) in guests {
                let snapshot = world.resource::<PartyAuthority>().snapshot(slot);
                if let Some(attempt) = world.resource::<Hosted>().attempts.get(&entity).copied() {
                    to_client(world, entity, SnapshotEnvelope { attempt, snapshot });
                }
            }
            let mut runtime = world.resource_mut::<Runtime>();
            runtime.published = snapshot.revision;
            runtime.latest = Some(snapshot);
        }
    }
    let runtime = world.resource::<Runtime>();
    let data = (
        runtime.role,
        runtime.player,
        runtime.admitted,
        runtime.latest.clone(),
    );
    let view = &mut *world.resource_mut::<LabyrinthView>();
    view.local = data.0 == Role::Local;
    view.host = matches!(data.0, Role::Host | Role::Local);
    view.player = data.1;
    view.admitted = data.2;
    if let Some(snapshot) = data.3.filter(|_| data.2 || data.0 == Role::Guest) {
        view.mode = if snapshot.combat.is_some() {
            ViewMode::Combat
        } else {
            ViewMode::Lobby
        };
        view.revision = snapshot.revision;
        view.encounter = snapshot.encounter;
        view.players = snapshot.players.iter().map(|p| p.view()).collect();
        view.combat = snapshot.combat;
        view.paused = snapshot.paused || !data.2;
        view.log = snapshot.log;
        view.events = snapshot.events;
    } else {
        view.mode = ViewMode::Menu;
        view.combat = None;
        view.players.clear();
        view.events.clear();
        view.paused = false;
    }
}

pub(crate) fn hosted_code(world: &World, index: usize) -> Option<String> {
    let hosted = world.get_resource::<Hosted>()?;
    let mut code = hosted.template.clone();
    code.invite_token = *hosted.invites.get(index)?;
    Some(code.encode().expose_for_sharing().to_owned())
}

fn disconnected(event: On<Disconnected>, mut queue: ResMut<DisconnectQueue>) {
    queue.0.push(event.entity);
}
fn listener_closed(event: On<TransportClosed>, mut queue: ResMut<ListenerClosedQueue>) {
    queue.0.push(event.entity);
}
fn finish_close(world: &mut World) {
    let (disconnect, close) = {
        let mut runtime = world.resource_mut::<Runtime>();
        let disconnect = runtime
            .pending_disconnect
            .filter(|(_, at)| at.elapsed() >= Duration::from_millis(250));
        if disconnect.is_some() {
            runtime.pending_disconnect = None;
        }
        let close = runtime
            .pending_close
            .filter(|(_, at)| at.elapsed() >= Duration::from_millis(250));
        if close.is_some() {
            runtime.pending_close = None;
        }
        (disconnect, close)
    };
    if let Some((entity, _)) = disconnect {
        world.trigger(Disconnect::new(entity, "left session"));
    }
    if let Some((entity, _)) = close {
        world.trigger(Close::new(entity, "host closed session"));
    }
}
