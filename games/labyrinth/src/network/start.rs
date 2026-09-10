//! Explicit socket and session startup; expensive password setup runs off-schedule.

use super::*;
use bevy_gamekit::discovery::{DiscoveryEndpoint, SessionPassword};

struct Prepared {
    settings: HostSettings,
    security: SessionAdmissionAuthority,
    transport: PreparedDirectHost,
    invites: Vec<InviteToken>,
    verifier: Option<SessionPasswordVerifier>,
}
#[derive(Resource)]
struct Preparing(Mutex<mpsc::Receiver<Result<Prepared, String>>>);

pub(super) fn host(world: &mut World, mut settings: HostSettings) -> Result<(), String> {
    if world.contains_resource::<Hosted>()
        || world.contains_resource::<Preparing>()
        || world.resource::<Runtime>().pending_close.is_some()
        || world.resource::<Runtime>().pending_disconnect.is_some()
    {
        return Err("A host is already starting, running, or closing.".into());
    }
    if settings.address.trim().is_empty() {
        settings.address = local_network_addresses()
            .ok()
            .and_then(|addresses| addresses.into_iter().next())
            .map(|ip| ip.to_string())
            .ok_or("No LAN address found; enter a reachable address explicitly.")?;
    }
    let endpoint =
        DirectEndpoint::new(settings.address.trim(), settings.port).map_err(|e| e.to_string())?;
    if (settings.lan || settings.tailnet) && is_loopback(endpoint.host()) {
        return Err("Discovery requires a reachable LAN or tailnet address, not loopback.".into());
    }
    SessionMetadata::new(
        GAME_ID,
        PROTOCOL,
        fingerprint_text(),
        &settings.name,
        1,
        PLAYER_CAPACITY,
        true,
    )
    .map_err(|e| e.to_string())?;
    let permit = WorkerPermit::acquire(&world.resource::<HostPreparationBudget>().0, 1)
        .ok_or("The previous host preparation is still finishing. Please retry shortly.")?;
    disconnect_guest(world);
    world.remove_resource::<discovery::Browser>();
    world.remove_resource::<PartyAuthority>();
    {
        let mut runtime = world.resource_mut::<Runtime>();
        runtime.role = Role::None;
        runtime.latest = None;
        runtime.player = None;
    }
    let (sender, receiver) = mpsc::sync_channel(1);
    let at = now(world);
    std::thread::Builder::new()
        .name("labyrinth-host-prepare".into())
        .spawn(move || {
            let _permit = permit;
            let result = (|| {
                let password = std::mem::take(&mut settings.password.0);
                let verifier = if settings.lan || settings.tailnet || !password.is_empty() {
                    let password = SessionPassword::new(password).map_err(|e| e.to_string())?;
                    Some(SessionPasswordVerifier::new(&password).map_err(|e| e.to_string())?)
                } else {
                    None
                };
                let mut security = SessionAdmissionAuthority::new(AdmissionLimits {
                    max_peers: GUEST_CAPACITY,
                    max_invites: GUEST_CAPACITY,
                    pending_timeout: ADMISSION_TIMEOUT,
                })
                .map_err(|e| e.to_string())?;
                let mut invites = Vec::new();
                for _ in 0..GUEST_CAPACITY {
                    invites.push(
                        security
                            .issue_invite(at, at + Duration::from_secs(3600))
                            .map_err(|e| e.to_string())?,
                    );
                }
                let first = *invites.first().ok_or("Invitation preparation failed.")?;
                let transport = PreparedDirectHost::new(endpoint, security.session_id(), first)
                    .map_err(|e| e.to_string())?;
                Ok(Prepared {
                    settings,
                    security,
                    transport,
                    invites,
                    verifier,
                })
            })();
            let _sent = sender.send(result);
        })
        .map_err(|_| "Unable to start host preparation worker.")?;
    world.insert_resource(Preparing(Mutex::new(receiver)));
    notice(
        world,
        "Preparing encrypted host and temporary admission credentials...",
    );
    Ok(())
}

pub(super) fn finish_host(world: &mut World) {
    let result = world
        .get_resource::<Preparing>()
        .and_then(|pending| pending.0.lock().ok()?.try_recv().ok());
    let Some(result) = result else {
        return;
    };
    world.remove_resource::<Preparing>();
    let prepared = match result {
        Ok(prepared) => prepared,
        Err(error) => {
            notice(world, error);
            return;
        }
    };
    let code = prepared.transport.connection_code().clone();
    let target = DiscoveredDirectTarget {
        session_id: code.session_id,
        endpoint: code.endpoint.clone(),
        certificate_fingerprint: code.certificate_fingerprint,
        certificate_expires_unix_seconds: code.certificate_expires_unix_seconds,
    };
    let metadata = match SessionMetadata::new(
        GAME_ID,
        PROTOCOL,
        fingerprint_text(),
        &prepared.settings.name,
        1,
        PLAYER_CAPACITY,
        prepared.verifier.is_some(),
    ) {
        Ok(metadata) => metadata,
        Err(error) => {
            notice(world, error.to_string());
            return;
        }
    };
    let providers = discovery::HostProviders::start(&prepared.settings, metadata.clone(), target);
    let session_id = code.session_id;
    let entity = prepared.transport.open(world);
    let password_jobs = Arc::clone(&world.resource::<PasswordWorkBudget>().0);
    world.insert_resource(Hosted {
        security: prepared.security,
        server: entity,
        template: code,
        invites: prepared.invites,
        verifier: prepared.verifier.map(|v| Arc::new(Mutex::new(v))),
        password_jobs,
        connections: BTreeMap::new(),
        seen: BTreeSet::new(),
        attempts: BTreeMap::new(),
        pending: BTreeMap::new(),
        rejected: BTreeMap::new(),
        observed: BTreeMap::new(),
        metadata,
        providers,
    });
    world.insert_resource(PartyAuthority::new(prepared.settings.seed, false));
    {
        let mut runtime = world.resource_mut::<Runtime>();
        runtime.role = Role::Host;
        runtime.player = Some(0);
        runtime.admitted = true;
        runtime.published = 0;
        runtime.session = Some(session_id);
    }
    let view = &mut *world.resource_mut::<LabyrinthView>();
    view.session_name = prepared.settings.name;
    view.invite_labels = (1..=GUEST_CAPACITY)
        .map(|index| format!("Guest invitation {index}"))
        .collect();
    view.notice = Some("Host started. Copy a separate invitation for each guest, or use discovery with the temporary passphrase.".into());
}

fn is_loopback(host: &str) -> bool {
    host.eq_ignore_ascii_case("localhost")
        || host.ends_with(".localhost")
        || host
            .trim_matches(['[', ']'])
            .parse::<std::net::IpAddr>()
            .is_ok_and(|ip| ip.is_loopback())
}

fn begin_guest(world: &mut World, entity: Entity, session: SessionId, credential: Credential) {
    world.remove_resource::<discovery::Browser>();
    world.remove_resource::<PartyAuthority>();
    let mut runtime = world.resource_mut::<Runtime>();
    runtime.role = Role::Guest;
    runtime.connection = Some(entity);
    runtime.session = Some(session);
    runtime.credential = Some(credential);
    runtime.attempt = Some(SessionId::generate());
    runtime.connecting_since = Some(Instant::now());
    runtime.latest = None;
    runtime.sequence = 1;
    runtime.player = None;
    runtime.admitted = false;
    notice(world, "Connecting to the pinned host...");
}

fn can_join(world: &World) -> Result<(), String> {
    if world.contains_resource::<Hosted>() || world.contains_resource::<Preparing>() {
        return Err("Close your running or preparing host before joining another session.".into());
    }
    let runtime = world.resource::<Runtime>();
    if runtime.pending_close.is_some() || runtime.pending_disconnect.is_some() {
        return Err("The previous session is still closing. Please retry shortly.".into());
    }
    Ok(())
}

pub(super) fn join_code(world: &mut World, encoded: &str) -> Result<(), String> {
    can_join(world)?;
    let code = DirectConnectionCode::parse(encoded).map_err(|e| e.to_string())?;
    if code.certificate_expires_unix_seconds <= unix_now() {
        return Err("This invitation's certificate has expired.".into());
    }
    let prepared = PreparedDirectJoin::new(&code).map_err(|e| e.to_string())?;
    disconnect_guest(world);
    let entity = prepared.connect(world);
    begin_guest(
        world,
        entity,
        code.session_id,
        Credential::Invite(code.invite_token),
    );
    Ok(())
}

pub(super) fn join_discovered(
    world: &mut World,
    session: SessionId,
    password: String,
) -> Result<(), String> {
    let password = SessionPassword::new(password).map_err(|e| e.to_string())?;
    can_join(world)?;
    let route = world
        .resource::<DiscoveryRegistry>()
        .resolve(session)
        .map_err(|e| e.to_string())?;
    let Some(DiscoveryEndpoint::Direct(target)) = route.endpoint(0) else {
        return Err("This service requires a transport adapter not installed by Labyrinth.".into());
    };
    let prepared = PreparedDirectDiscoveryJoin::new(target).map_err(|e| e.to_string())?;
    disconnect_guest(world);
    let entity = prepared.connect(world);
    begin_guest(
        world,
        entity,
        session,
        Credential::Password(WirePassword(
            password.expose_for_encrypted_transport().to_owned(),
        )),
    );
    Ok(())
}

pub(super) fn reconnect(world: &mut World) -> Result<(), String> {
    can_join(world)?;
    let stored = world
        .resource::<ReconnectCredentialStorage>()
        .store()
        .load()
        .map_err(|e| e.to_string())?
        .ok_or("This profile has no stored reservation.")?;
    if stored.is_expired_at(unix_now()) {
        return Err("The stored host certificate has expired.".into());
    }
    let prepared =
        PreparedDirectReconnect::new(&stored.endpoint_binding).map_err(|e| e.to_string())?;
    disconnect_guest(world);
    let entity = prepared.connect(world);
    begin_guest(
        world,
        entity,
        stored.session_id,
        Credential::Reconnect(stored.reconnect_credential),
    );
    Ok(())
}

pub(super) fn disconnect_guest(world: &mut World) {
    let connection = {
        let mut runtime = world.resource_mut::<Runtime>();
        runtime.credential = None;
        runtime.admitted = false;
        runtime.attempt = None;
        runtime.connecting_since = None;
        runtime.connection.take()
    };
    if let Some(entity) = connection {
        world.trigger(Disconnect::new(entity, "new connection attempt"));
    }
    // Messages from a previous physical connection cannot authorize its successor.
    drain::<Offer>(world);
    drain::<Admitted>(world);
    drain::<SnapshotEnvelope>(world);
    drain::<Refusal>(world);
    drain::<Closed>(world);
    drain::<RequestResult>(world);
}

pub(super) fn close(world: &mut World) {
    world.remove_resource::<Preparing>();
    world.remove_resource::<discovery::Browser>();
    if let Some(mut hosted) = world.remove_resource::<Hosted>() {
        let _cleanup = hosted.security.close();
        for (entity, attempt) in &hosted.attempts {
            to_client(world, *entity, Closed { attempt: *attempt });
        }
        world.resource_mut::<Runtime>().pending_close = Some((hosted.server, Instant::now()));
    }
    let connection = world.resource_mut::<Runtime>().connection.take();
    if let Some(entity) = connection {
        world.write_message(LeaveSession);
        world.resource_mut::<Runtime>().pending_disconnect = Some((entity, Instant::now()));
    }
    world.remove_resource::<PartyAuthority>();
    let mut runtime = world.resource_mut::<Runtime>();
    runtime.role = Role::None;
    runtime.admitted = false;
    runtime.latest = None;
    runtime.player = None;
    runtime.credential = None;
    runtime.published = 0;
    runtime.attempt = None;
    runtime.connecting_since = None;
    let view = &mut *world.resource_mut::<LabyrinthView>();
    view.invite_labels.clear();
    view.notice = None;
}
