//! Actual adapter regressions for persist-before-ACK admission and attempt isolation.

use super::tests::{pump_until, socket_app};
use super::*;
use bevy::ecs::system::RunSystemOnce as _;
use bevy_gamekit::multiplayer::{CredentialStoreError, ReconnectCredentialStore};
use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc,
};

fn host(password: &str) -> App {
    let mut app = socket_app();
    start_host(app.world_mut(), configuration(password)).expect("host");
    app
}

fn configuration(password: &str) -> HostConfiguration {
    let socket = std::net::UdpSocket::bind("127.0.0.1:0").expect("free UDP port");
    let port = socket.local_addr().expect("port").port();
    drop(socket);
    HostConfiguration {
        session_name: "Admission regression".into(),
        password: password.into(),
        advertised_host: "127.0.0.1".into(),
        port,
        discover_lan: false,
        discover_tailnet: false,
    }
}

fn guest_file(path: &std::path::Path) -> App {
    let mut app = socket_app();
    app.insert_resource(ReconnectCredentialStorage::new(
        AtomicFileReconnectCredentialStore::new(path),
    ));
    app
}

fn admitted(host: &mut App, guest: &mut App) {
    assert!(
        pump_until(host, guest, Duration::from_secs(10), |host, guest| {
            guest.world().resource::<DeckNetworkState>().admitted
                && guest
                    .world()
                    .resource::<DeckNetworkState>()
                    .latest
                    .is_some()
                && host
                    .world()
                    .resource::<DeckAuthority>()
                    .snapshot(Seat::Guest)
                    .seats
                    .iter()
                    .any(|seat| seat.seat == Seat::Guest && seat.connected)
        }),
        "socket admission did not finish"
    );
}

fn stored(app: &App) -> StoredReconnectCredential {
    app.world()
        .resource::<ReconnectCredentialStorage>()
        .store()
        .load()
        .expect("read profile")
        .expect("durable credential")
}

fn disconnected(host: &mut App, guest: &mut App) {
    disconnect_guest(guest.world_mut());
    assert!(
        pump_until(host, guest, Duration::from_secs(8), |host, _| {
            host.world_mut()
                .query_filtered::<Entity, With<ConnectedClient>>()
                .iter(host.world())
                .count()
                == 0
        }),
        "host did not detach lost physical connection"
    );
}

#[derive(Resource, Default)]
struct HandshakeFault {
    drop_offer: bool,
    drop_ack: bool,
    dropped: usize,
}

fn discard_offer(mut messages: ResMut<Messages<DeckOffer>>, mut fault: ResMut<HandshakeFault>) {
    if fault.drop_offer {
        fault.dropped += messages.drain().count();
    }
}

fn discard_ack(mut messages: ResMut<Messages<DeckPersistence>>, mut fault: ResMut<HandshakeFault>) {
    if fault.drop_ack {
        fault.dropped += messages.drain().count();
    }
}

fn faults(app: &mut App, drop_offer: bool) {
    app.insert_resource(HandshakeFault {
        drop_offer,
        drop_ack: !drop_offer,
        dropped: 0,
    })
    .add_systems(
        PreUpdate,
        discard_offer
            .before(receive_offer)
            .in_set(bevy_gamekit::multiplayer::MultiplayerSystems::Receive),
    )
    .add_systems(
        PostUpdate,
        discard_ack.in_set(bevy_gamekit::multiplayer::MultiplayerSystems::Send),
    );
}

#[test]
fn lost_initial_offer_and_persisted_ack_retry_the_same_reserved_identity() {
    for drop_offer in [true, false] {
        let scratch = tempfile::tempdir().expect("profile directory");
        let path = scratch.path().join("reconnect.json");
        let mut host = host("");
        let mut guest = guest_file(&path);
        faults(&mut guest, drop_offer);
        let code = hosted_code(host.world()).expect("invite");
        start_direct_join(guest.world_mut(), &code).expect("initial attempt");
        assert!(pump_until(
            &mut host,
            &mut guest,
            Duration::from_secs(10),
            |_, guest| guest.world().resource::<HandshakeFault>().dropped > 0
        ));
        assert!(!guest.world().resource::<DeckNetworkState>().admitted);
        let peer = host
            .world()
            .resource::<HostedSession>()
            .guest_peer
            .expect("pending identity");
        assert_eq!(
            host.world_mut()
                .query_filtered::<Entity, With<DeckAuthorized>>()
                .iter(host.world())
                .count(),
            0
        );
        assert!(
            guest
                .world()
                .resource::<DeckNetworkState>()
                .latest
                .is_none(),
            "private snapshot must wait for ACK"
        );
        let offered = (!drop_offer).then(|| stored(&guest));
        disconnected(&mut host, &mut guest);
        drop(guest);
        let mut guest = guest_file(&path);
        if drop_offer {
            start_direct_join(guest.world_mut(), &code).expect("retry same invite");
        } else {
            reconnect(guest.world_mut()).expect("retry persisted successor");
        }
        admitted(&mut host, &mut guest);
        assert_eq!(stored(&guest).peer_id, peer);
        if let Some(offered) = offered {
            assert_eq!(
                stored(&guest).reconnect_credential,
                offered.reconnect_credential
            );
        }
        assert_eq!(
            host.world()
                .resource::<HostedSession>()
                .security
                .reserved_peer_count(),
            1
        );
    }
}

#[test]
fn expired_pending_offer_releases_capacity_without_consuming_the_invitation() {
    let mut host = host("");
    let mut guest = socket_app();
    faults(&mut guest, true);
    let code = hosted_code(host.world()).expect("invite");
    start_direct_join(guest.world_mut(), &code).expect("initial attempt");
    assert!(pump_until(
        &mut host,
        &mut guest,
        Duration::from_secs(10),
        |_, guest| guest.world().resource::<HandshakeFault>().dropped > 0
    ));
    let old_peer = host
        .world()
        .resource::<HostedSession>()
        .guest_peer
        .expect("pending peer");
    host.world_mut().resource_mut::<HostedSession>().clock = Instant::now()
        .checked_sub(Duration::from_secs(20))
        .expect("injected monotonic elapsed time");
    assert!(pump_until(
        &mut host,
        &mut guest,
        Duration::from_secs(5),
        |host, guest| {
            host.world()
                .resource::<HostedSession>()
                .security
                .reserved_peer_count()
                == 0
                && !guest.world().contains_resource::<GuestConnection>()
        }
    ));
    assert_eq!(
        host.world()
            .resource::<HostedSession>()
            .metadata
            .claimed_players(),
        1
    );
    assert!(host
        .world()
        .resource::<HostedSession>()
        .guest_peer
        .is_none());
    let mut guest = socket_app();
    start_direct_join(guest.world_mut(), &code).expect("unconsumed invitation retries");
    admitted(&mut host, &mut guest);
    assert_ne!(
        stored(&guest).peer_id,
        old_peer,
        "an expired provisional identity was intentionally released"
    );
}

#[test]
fn interrupted_rotation_keeps_one_successor_until_ack_and_then_rejects_old_credentials() {
    for drop_offer in [true, false] {
        let scratch = tempfile::tempdir().expect("profile directory");
        let path = scratch.path().join("reconnect.json");
        let mut host = host("");
        let mut guest = guest_file(&path);
        start_direct_join(
            guest.world_mut(),
            &hosted_code(host.world()).expect("invite"),
        )
        .expect("join");
        admitted(&mut host, &mut guest);
        let before = stored(&guest);
        let old_attempt = guest.world().resource::<GuestAttempt>().attempt;
        let expected = host
            .world()
            .resource::<DeckAuthority>()
            .snapshot(Seat::Guest)
            .own_hand;
        disconnected(&mut host, &mut guest);
        drop(guest);
        let mut guest = guest_file(&path);
        faults(&mut guest, drop_offer);
        reconnect(guest.world_mut()).expect("rotation attempt");
        assert!(pump_until(
            &mut host,
            &mut guest,
            Duration::from_secs(10),
            |_, guest| guest.world().resource::<HandshakeFault>().dropped > 0
        ));
        assert!(!guest.world().resource::<DeckNetworkState>().admitted);
        let pending = stored(&guest);
        disconnected(&mut host, &mut guest);
        drop(guest);
        let mut guest = guest_file(&path);
        reconnect(guest.world_mut()).expect("retry rotation");
        admitted(&mut host, &mut guest);
        let after = stored(&guest);
        assert_eq!(after.peer_id, before.peer_id);
        assert_ne!(after.reconnect_credential, before.reconnect_credential);
        if !drop_offer {
            assert_eq!(after.reconnect_credential, pending.reconnect_credential);
        }
        assert_eq!(
            guest
                .world()
                .resource::<DeckNetworkState>()
                .latest
                .as_ref()
                .expect("snapshot")
                .own_hand,
            expected
        );
        let snapshot = guest
            .world()
            .resource::<DeckNetworkState>()
            .latest
            .clone()
            .expect("new snapshot");
        let mut old_snapshot = snapshot.clone();
        old_snapshot.sequence = old_snapshot.sequence.saturating_add(100);
        old_snapshot.own_hand.clear();
        guest.world_mut().write_message(DeckSnapshot {
            attempt: old_attempt,
            snapshot: old_snapshot,
        });
        guest.world_mut().write_message(DeckWelcome {
            attempt: old_attempt,
            session_id: before.session_id,
            peer_id: PeerId::generate(),
            reconnected: false,
        });
        guest.world_mut().write_message(DeckAdmissionRefusal {
            attempt: old_attempt,
        });
        guest.world_mut().write_message(DeckSessionClosed {
            attempt: old_attempt,
        });
        guest.update();
        assert!(guest.world().resource::<DeckNetworkState>().admitted);
        assert_eq!(
            guest.world().resource::<DeckNetworkState>().latest,
            Some(snapshot)
        );
        assert_eq!(stored(&guest), after);
        disconnected(&mut host, &mut guest);
        let mut rejected = socket_app();
        rejected
            .world()
            .resource::<ReconnectCredentialStorage>()
            .store()
            .store_atomically(before)
            .expect("old credential fixture");
        reconnect(rejected.world_mut()).expect("old token reaches host");
        assert!(pump_until(
            &mut host,
            &mut rejected,
            Duration::from_secs(5),
            |_, guest| guest
                .world()
                .resource::<DeckNetworkState>()
                .notice
                .as_deref()
                == Some("The host refused session admission.")
        ));
        assert_eq!(
            host.world()
                .resource::<HostedSession>()
                .security
                .reserved_peer_count(),
            1
        );
    }
}

struct SwitchableStore {
    fail: Arc<AtomicBool>,
    inner: MemoryReconnectCredentialStore,
}

impl ReconnectCredentialStore for SwitchableStore {
    fn load(&self) -> Result<Option<StoredReconnectCredential>, CredentialStoreError> {
        self.inner.load()
    }
    fn store_atomically(
        &self,
        value: StoredReconnectCredential,
    ) -> Result<(), CredentialStoreError> {
        if self.fail.load(Ordering::SeqCst) {
            Err(CredentialStoreError::Unavailable)
        } else {
            self.inner.store_atomically(value)
        }
    }
    fn delete_if_session(&self, session: SessionId) -> Result<bool, CredentialStoreError> {
        self.inner.delete_if_session(session)
    }
    fn delete_if_expired(&self, now: u64) -> Result<bool, CredentialStoreError> {
        self.inner.delete_if_expired(now)
    }
}

#[test]
fn persistence_failure_never_admits_and_releases_the_initial_invite_for_retry() {
    let mut host = host("");
    let mut guest = socket_app();
    let fail = Arc::new(AtomicBool::new(true));
    guest.insert_resource(ReconnectCredentialStorage::new(SwitchableStore {
        fail: Arc::clone(&fail),
        inner: MemoryReconnectCredentialStore::default(),
    }));
    let code = hosted_code(host.world()).expect("invite");
    start_direct_join(guest.world_mut(), &code).expect("attempt");
    assert!(pump_until(
        &mut host,
        &mut guest,
        Duration::from_secs(10),
        |host, guest| {
            guest
                .world()
                .resource::<DeckNetworkState>()
                .notice
                .as_ref()
                .is_some_and(|notice| notice.contains("could not be stored"))
                && host
                    .world()
                    .resource::<HostedSession>()
                    .security
                    .reserved_peer_count()
                    == 0
                && !guest.world().contains_resource::<GuestConnection>()
        }
    ));
    assert!(!guest.world().resource::<DeckNetworkState>().admitted);
    assert!(host
        .world()
        .resource::<HostedSession>()
        .guest_peer
        .is_none());
    assert_eq!(
        host.world()
            .resource::<HostedSession>()
            .metadata
            .claimed_players(),
        1
    );
    fail.store(false, Ordering::SeqCst);
    start_direct_join(guest.world_mut(), &code).expect("same invite remains usable");
    admitted(&mut host, &mut guest);

    // Failed rotation preserves the last durable file and established private seat.
    let previous = stored(&guest);
    fail.store(true, Ordering::SeqCst);
    disconnected(&mut host, &mut guest);
    reconnect(guest.world_mut()).expect("rotation attempt");
    assert!(pump_until(
        &mut host,
        &mut guest,
        Duration::from_secs(10),
        |_, guest| {
            guest
                .world()
                .resource::<DeckNetworkState>()
                .notice
                .as_ref()
                .is_some_and(|notice| notice.contains("could not be stored"))
                && !guest.world().contains_resource::<GuestConnection>()
        }
    ));
    assert!(!guest.world().resource::<DeckNetworkState>().admitted);
    assert_eq!(stored(&guest), previous);
    assert_eq!(
        host.world()
            .resource::<HostedSession>()
            .security
            .reserved_peer_count(),
        1
    );
    fail.store(false, Ordering::SeqCst);
    reconnect(guest.world_mut()).expect("retry old durable credential");
    admitted(&mut host, &mut guest);
    assert_eq!(stored(&guest).peer_id, previous.peer_id);
    assert_ne!(
        stored(&guest).reconnect_credential,
        previous.reconnect_credential
    );
}

#[test]
fn fake_discovery_password_join_uses_the_same_acknowledged_path_and_keeps_two_seat_capacity() {
    let mut host = host("temporary-passphrase");
    let mut guest = socket_app();
    let route = {
        let hosted = host.world().resource::<HostedSession>();
        let mut registry = bevy_gamekit::discovery::DiscoveryRegistry::default();
        registry.apply(
            DiscoveryObservation::Found {
                metadata: hosted.metadata.clone(),
                route: bevy_gamekit::discovery::DiscoveryRoute::new(
                    bevy_gamekit::discovery::DiscoveryProviderId::FAKE,
                    bevy_gamekit::discovery::DiscoverySource::Service,
                    hosted.target.clone(),
                    Duration::from_secs(60),
                ),
            },
            None,
        );
        registry
            .resolve(hosted.target.session_id)
            .expect("synthetic public listing")
    };
    start_discovered_join(guest.world_mut(), &route, "temporary-passphrase".into())
        .expect("password join");
    admitted(&mut host, &mut guest);
    let identity = stored(&guest);
    assert_eq!(
        host.world().resource::<HostedSession>().guest_peer,
        Some(identity.peer_id)
    );
    let mut refused = socket_app();
    start_discovered_join(refused.world_mut(), &route, "temporary-passphrase".into())
        .expect("third player connects");
    assert!(pump_until(
        &mut host,
        &mut refused,
        Duration::from_secs(5),
        |_, guest| guest
            .world()
            .resource::<DeckNetworkState>()
            .notice
            .as_deref()
            == Some("The host refused session admission.")
    ));
    assert!(!refused.world().resource::<DeckNetworkState>().admitted);
    assert_eq!(
        host.world()
            .resource::<HostedSession>()
            .security
            .reserved_peer_count(),
        1
    );
    assert_eq!(
        host.world().resource::<HostedSession>().guest_peer,
        Some(identity.peer_id)
    );
    assert_eq!(stored(&guest), identity);
}

#[test]
fn stale_attempt_messages_and_unpersisted_welcome_cannot_admit_or_replace_state() {
    let mut guest = socket_app();
    let session = SessionId::generate();
    let entity = guest.world_mut().spawn_empty().id();
    guest.world_mut().insert_resource(GuestConnection(entity));
    begin_guest_connect(
        guest.world_mut(),
        session,
        None,
        DeckCredential::Invite(InviteToken::generate()),
    );
    let attempt = guest.world().resource::<GuestAttempt>().attempt;
    let old = SessionId::generate();
    let peer = PeerId::generate();
    let world = guest.world_mut();
    world.write_message(DeckOffer {
        attempt: old,
        reconnected: false,
        session_id: session,
        peer_id: peer,
        seat: Seat::Guest,
        reconnect_credential: ReconnectCredential::generate(),
    });
    // Even a correctly scoped welcome cannot authorize before a persisted offer.
    world.write_message(DeckWelcome {
        attempt,
        session_id: session,
        peer_id: peer,
        reconnected: false,
    });
    world.write_message(DeckSnapshot {
        attempt: old,
        snapshot: DeckAuthority::solo().snapshot(Seat::Guest),
    });
    world.write_message(DeckAdmissionRefusal { attempt: old });
    world.write_message(DeckSessionClosed { attempt: old });
    guest.update();
    let state = guest.world().resource::<DeckNetworkState>();
    assert!(!state.admitted);
    assert!(state.latest.is_none());
    assert_eq!(state.notice.as_deref(), Some("Connecting to the host…"));
    assert!(guest
        .world()
        .resource::<ReconnectCredentialStorage>()
        .store()
        .load()
        .expect("storage")
        .is_none());
    assert!(guest
        .world()
        .resource::<Messages<DeckPersistence>>()
        .is_empty());
}

#[test]
fn duplicate_hello_and_ack_are_idempotent_and_old_protocol_is_explicitly_incompatible() {
    let mut host = host("");
    let mut guest = socket_app();
    let code =
        DirectConnectionCode::parse(&hosted_code(host.world()).expect("invite")).expect("code");
    start_direct_join(guest.world_mut(), code.encode().expose_for_sharing()).expect("join");
    admitted(&mut host, &mut guest);
    let attempt = guest.world().resource::<GuestAttempt>().attempt;
    let saved = stored(&guest);
    let before = host
        .world()
        .resource::<DeckAuthority>()
        .snapshot(Seat::Guest);
    guest.world_mut().write_message(DeckClientHello {
        session: code.session_id,
        attempt: SessionId::generate(),
        protocol: PROTOCOL_SCHEMA.into(),
        build: BUILD_ID.into(),
        credential: DeckCredential::Invite(code.invite_token),
    });
    guest.world_mut().write_message(DeckPersistence {
        attempt,
        credential: Some(saved.reconnect_credential),
    });
    for _ in 0..20 {
        host.update();
        guest.update();
        std::thread::sleep(Duration::from_millis(5));
    }
    assert_eq!(stored(&guest), saved);
    assert_eq!(
        host.world()
            .resource::<DeckAuthority>()
            .snapshot(Seat::Guest),
        before
    );
    assert_eq!(
        host.world()
            .resource::<HostedSession>()
            .security
            .reserved_peer_count(),
        1
    );
    disconnected(&mut host, &mut guest);
    let mut stale = socket_app();
    start_direct_join(stale.world_mut(), code.encode().expose_for_sharing())
        .expect("stale game attempts transport");
    stale.world_mut().resource_mut::<PendingHello>().sent = true;
    assert!(pump_until(
        &mut host,
        &mut stale,
        Duration::from_secs(5),
        |_, app| *app.world().resource::<State<ClientState>>().get() == ClientState::Connected
    ));
    let attempt = stale.world().resource::<GuestAttempt>().attempt;
    stale.world_mut().write_message(DeckClientHello {
        session: code.session_id,
        attempt,
        protocol: "2".into(),
        build: BUILD_ID.into(),
        credential: DeckCredential::Reconnect(saved.reconnect_credential),
    });
    assert!(pump_until(
        &mut host,
        &mut stale,
        Duration::from_secs(5),
        |_, app| app.world().resource::<DeckNetworkState>().notice.as_deref()
            == Some("The host refused session admission.")
    ));
    assert!(!stale.world().resource::<DeckNetworkState>().admitted);
}

#[test]
fn host_close_before_welcome_deletes_the_persisted_unacknowledged_offer() {
    let scratch = tempfile::tempdir().expect("profile directory");
    let mut host = host("");
    let mut guest = guest_file(&scratch.path().join("reconnect.json"));
    faults(&mut guest, false);
    start_direct_join(
        guest.world_mut(),
        &hosted_code(host.world()).expect("invite"),
    )
    .expect("join");
    assert!(pump_until(
        &mut host,
        &mut guest,
        Duration::from_secs(10),
        |_, guest| { guest.world().resource::<HandshakeFault>().dropped > 0 }
    ));
    let session = stored(&guest).session_id;
    assert_eq!(
        guest.world().resource::<DeckNetworkState>().session_id,
        None
    );
    assert!(!guest.world().resource::<DeckNetworkState>().admitted);
    assert_eq!(guest.world().resource::<GuestAttempt>().session, session);
    close_session(host.world_mut());
    assert!(pump_until(
        &mut host,
        &mut guest,
        Duration::from_secs(5),
        |_, guest| {
            guest
                .world()
                .resource::<DeckNetworkState>()
                .notice
                .as_deref()
                == Some("The host closed the session.")
        }
    ));
    assert!(guest
        .world()
        .resource::<ReconnectCredentialStorage>()
        .store()
        .load()
        .expect("read profile after close")
        .is_none());
    assert!(!guest.world().contains_resource::<GuestAttempt>());
}

#[test]
fn closing_host_revokes_old_connections_before_solo_or_replacement_gameplay() {
    for replacement_host in [false, true] {
        let mut host = host("");
        let mut guest = socket_app();
        start_direct_join(
            guest.world_mut(),
            &hosted_code(host.world()).expect("invite"),
        )
        .expect("join");
        admitted(&mut host, &mut guest);
        let (old_entity, old_authorized) = {
            let world = host.world_mut();
            let mut query = world.query::<(Entity, &DeckAuthorized)>();
            let (entity, authorized) = query.single(world).expect("only admitted guest");
            (entity, *authorized)
        };
        close_session(host.world_mut());
        assert!(
            host.world().get::<ConnectedClient>(old_entity).is_some(),
            "the close notice still has a transport to drain"
        );
        assert!(host.world().get::<DeckAuthorized>(old_entity).is_none());
        assert!(host.world().get::<AuthorizedClient>(old_entity).is_none());
        assert!(host
            .world()
            .get::<bevy_gamekit::multiplayer::AuthenticatedPeer>(old_entity)
            .is_none());

        let new_entity = if replacement_host {
            start_host(host.world_mut(), configuration("")).expect("replacement host");
            let code = DirectConnectionCode::parse(&hosted_code(host.world()).expect("new invite"))
                .expect("new connection code");
            host.world_mut()
                .resource_mut::<Messages<ToClients<DeckOffer>>>()
                .clear();
            host.world_mut().write_message(FromClient {
                client_id: ClientId::from(old_entity),
                message: DeckClientHello {
                    session: code.session_id,
                    attempt: SessionId::generate(),
                    protocol: PROTOCOL_SCHEMA.into(),
                    build: BUILD_ID.into(),
                    credential: DeckCredential::Invite(code.invite_token),
                },
            });
            host.world_mut()
                .run_system_once(handle_client_hellos)
                .expect("old listener cannot admit into the new session");
            assert_eq!(
                host.world()
                    .resource::<HostedSession>()
                    .security
                    .reserved_peer_count(),
                0
            );
            assert!(host.world().resource::<HostedSession>().attempts.is_empty());
            assert!(host
                .world()
                .resource::<Messages<ToClients<DeckOffer>>>()
                .is_empty());
            let entity = host
                .world_mut()
                .spawn(ConnectedClient { max_size: 1200 })
                .id();
            let attempt = SessionId::generate();
            let authorized = {
                let mut hosted = host.world_mut().resource_mut::<HostedSession>();
                let offer = hosted
                    .security
                    .begin_external(entity.to_bits(), Duration::ZERO)
                    .expect("new session peer");
                hosted
                    .security
                    .acknowledge(entity.to_bits(), offer.reconnect_credential, Duration::ZERO)
                    .expect("persisted fixture acknowledgement");
                hosted.guest_peer = Some(offer.peer);
                hosted.attempts.insert(entity, attempt);
                DeckAuthorized {
                    session: hosted.security.session_id(),
                    peer: offer.peer,
                    seat: Seat::Guest,
                    attempt,
                }
            };
            host.world_mut()
                .entity_mut(entity)
                .insert((AuthorizedClient, authorized));
            host.world_mut()
                .resource_mut::<DeckAuthority>()
                .set_connected(Seat::Guest, true);
            Some(entity)
        } else {
            start_solo(host.world_mut());
            submit_command(host.world_mut(), GameCommand::EndTurn);
            None
        };
        // Defense in depth: even an obsolete marker surviving some other teardown
        // path must not authorize messages, disclosure, or delayed disconnects.
        host.world_mut()
            .entity_mut(old_entity)
            .insert((AuthorizedClient, old_authorized));
        let before = host
            .world()
            .resource::<DeckAuthority>()
            .snapshot(Seat::Guest);
        host.world_mut()
            .resource_mut::<Messages<ToClients<DeckSnapshot>>>()
            .clear();
        host.world_mut()
            .resource_mut::<Messages<ToClients<DeckResult>>>()
            .clear();
        host.world_mut().write_message(FromClient {
            client_id: ClientId::from(old_entity),
            message: DeckRequest {
                attempt: old_authorized.attempt,
                request: GameRequest {
                    request_id: RequestId(1),
                    command: if replacement_host {
                        GameCommand::SetReady(true)
                    } else {
                        GameCommand::PlayCard(crate::domain::CardKind::Spark)
                    },
                },
            },
        });
        host.world_mut()
            .run_system_once(handle_remote_requests)
            .expect("receive stale request");
        assert_eq!(
            host.world()
                .resource::<DeckAuthority>()
                .snapshot(Seat::Guest),
            before
        );
        assert!(host
            .world()
            .resource::<Messages<ToClients<DeckResult>>>()
            .is_empty());
        assert!(host
            .world()
            .resource::<Messages<ToClients<DeckSnapshot>>>()
            .is_empty());
        publish_snapshots(host.world_mut());
        let publications = host
            .world_mut()
            .resource_mut::<Messages<ToClients<DeckSnapshot>>>()
            .drain()
            .collect::<Vec<_>>();
        assert_eq!(publications.len(), usize::from(replacement_host));
        assert!(publications.iter().all(|message| {
            matches!(message.targets, SendTargets::Single(id) if id.entity() == new_entity)
        }));
        host.world_mut()
            .entity_mut(old_entity)
            .remove::<ConnectedClient>();
        assert_eq!(
            host.world()
                .resource::<DeckAuthority>()
                .snapshot(Seat::Guest),
            before,
            "late removal from the old session must not disconnect the new guest"
        );
        if let Some(entity) = new_entity {
            assert!(host
                .world()
                .resource::<HostedSession>()
                .security
                .is_connection_admitted(entity.to_bits()));
        }
    }
}
