//! Bounded admission and cleanup tied to actual connection entities.

use super::*;
use bevy_gamekit::discovery::SessionPassword;

pub(super) fn host_messages(world: &mut World) {
    let lost = std::mem::take(&mut world.resource_mut::<DisconnectQueue>().0);
    let hellos = drain::<FromClient<Hello>>(world);
    let acknowledgements = drain::<FromClient<Persisted>>(world);
    let requests = drain::<FromClient<GameRequest>>(world);
    let leaves = drain::<FromClient<LeaveSession>>(world);
    for entity in &lost {
        if world.resource::<Runtime>().connection == Some(*entity) {
            let mut runtime = world.resource_mut::<Runtime>();
            runtime.connection = None;
            runtime.admitted = false;
            runtime.credential = None;
            runtime.attempt = None;
            runtime.connecting_since = None;
            notice(
                world,
                "Connection lost. Reconnect with this profile to reclaim your reserved hero.",
            );
        }
    }
    let Some(mut hosted) = world.remove_resource::<Hosted>() else {
        return;
    };
    let at = now(world);
    for entity in lost {
        hosted.security.disconnect(entity.to_bits());
        hosted.pending.remove(&entity);
        hosted.seen.remove(&entity);
        hosted.attempts.remove(&entity);
        hosted.observed.remove(&entity);
        hosted.rejected.remove(&entity);
        if let Some((peer, _)) = hosted.connections.remove(&entity) {
            world
                .resource_mut::<PartyAuthority>()
                .connected(peer, false);
        }
    }
    for leave in leaves {
        let Some(entity) = leave.client_id.entity() else {
            continue;
        };
        if world.resource::<PartyAuthority>().in_lobby() {
            if let Some((peer, _)) = hosted.connections.remove(&entity) {
                cleanup(world, hosted.security.revoke_peer(peer));
                world.resource_mut::<PartyAuthority>().release(peer);
            }
        }
    }
    // Reject new sockets beyond a bounded handshake budget, even before a hello.
    let connected = {
        let mut query = world.query_filtered::<Entity, With<ConnectedClient>>();
        query.iter(world).collect::<Vec<_>>()
    };
    for entity in connected {
        if !hosted.observed.contains_key(&entity) {
            if hosted.observed.len() >= MAX_HANDSHAKES {
                world.trigger(Disconnect::new(entity, "admission queue full"));
                continue;
            }
            hosted.observed.insert(entity, Instant::now());
        }
    }
    for hello in hellos.into_iter().take(32) {
        let Some(entity) = hello.client_id.entity() else {
            continue;
        };
        if !hosted.observed.contains_key(&entity) || !hosted.seen.insert(entity) {
            continue;
        }
        hosted.attempts.insert(entity, hello.attempt);
        if !hello.attempt.is_valid()
            || hello.session != hosted.security.session_id()
            || hello.fingerprint != fingerprint()
        {
            refuse(world, &mut hosted, entity, Refused::Incompatible);
            continue;
        }
        match hello.message.credential {
            Credential::Password(mut password) => {
                if !world.resource::<PartyAuthority>().has_space() {
                    refuse(world, &mut hosted, entity, Refused::Full);
                    continue;
                }
                let Some(verifier) = hosted.verifier.clone() else {
                    refuse(world, &mut hosted, entity, Refused::Admission);
                    continue;
                };
                if hosted.pending.len() >= PASSWORD_WORKERS {
                    refuse(world, &mut hosted, entity, Refused::Admission);
                    continue;
                }
                let Some(permit) = WorkerPermit::acquire(&hosted.password_jobs, PASSWORD_WORKERS)
                else {
                    refuse(world, &mut hosted, entity, Refused::Admission);
                    continue;
                };
                let Ok(password) = SessionPassword::new(std::mem::take(&mut password.0)) else {
                    refuse(world, &mut hosted, entity, Refused::Admission);
                    continue;
                };
                let Some(source) = world.get::<PeerAddr>(entity).map(|peer| peer.0.ip()) else {
                    refuse(world, &mut hosted, entity, Refused::Admission);
                    continue;
                };
                let (sender, receiver) = mpsc::sync_channel(1);
                let worker = std::thread::Builder::new()
                    .name("labyrinth-password".into())
                    .spawn(move || {
                        let _permit = permit;
                        let valid = verifier.lock().is_ok_and(|mut verifier| {
                            verifier.verify(source, at, &password).is_ok()
                        });
                        let _sent = sender.send(valid);
                    });
                if worker.is_err() {
                    refuse(world, &mut hosted, entity, Refused::Admission);
                    continue;
                }
                hosted
                    .pending
                    .insert(entity, (Instant::now(), Mutex::new(receiver)));
            }
            credential => {
                let credential = match credential {
                    Credential::Invite(token) => AdmissionCredential::Invite(token),
                    Credential::Reconnect(token) => AdmissionCredential::Reconnect(token),
                    Credential::Password(_) => continue,
                };
                // Reconnect and retried pending invites may reclaim a full party.
                let result = hosted.security.begin(entity.to_bits(), credential, at);
                match result {
                    Ok(offer) => offer_peer(world, &mut hosted, entity, offer),
                    Err(bevy_gamekit::session::AdmissionFlowError::PeerLimit) => {
                        refuse(world, &mut hosted, entity, Refused::Full)
                    }
                    Err(_) => refuse(world, &mut hosted, entity, Refused::Admission),
                }
            }
        }
    }
    let completed = hosted
        .pending
        .iter()
        .filter_map(|(entity, (_, receiver))| {
            receiver
                .lock()
                .ok()?
                .try_recv()
                .ok()
                .map(|valid| (*entity, valid))
        })
        .collect::<Vec<_>>();
    for (entity, valid) in completed {
        hosted.pending.remove(&entity);
        if world.get::<ConnectedClient>(entity).is_none() {
            continue;
        }
        if !valid {
            refuse(world, &mut hosted, entity, Refused::Admission);
            continue;
        }
        if !world.resource::<PartyAuthority>().has_space() {
            refuse(world, &mut hosted, entity, Refused::Full);
            continue;
        }
        match hosted.security.begin_external(entity.to_bits(), at) {
            Ok(offer) => offer_peer(world, &mut hosted, entity, offer),
            Err(_) => refuse(world, &mut hosted, entity, Refused::Admission),
        }
    }
    for acknowledged in acknowledgements.into_iter().take(32) {
        let Some(entity) = acknowledged.client_id.entity() else {
            continue;
        };
        if hosted.attempts.get(&entity) != Some(&acknowledged.attempt) {
            continue;
        }
        let Some((peer, slot)) = hosted.connections.get(&entity).copied() else {
            continue;
        };
        match hosted
            .security
            .acknowledge(entity.to_bits(), acknowledged.credential, at)
        {
            Ok(grant) => {
                if world.get::<AuthorizedClient>(entity).is_none() {
                    world.entity_mut(entity).insert((
                        AuthorizedClient,
                        AuthenticatedPeer {
                            peer,
                            reconnected: grant.reconnected,
                        },
                    ));
                    world.resource_mut::<PartyAuthority>().connected(peer, true);
                }
                to_client(
                    world,
                    entity,
                    Admitted {
                        session: hosted.security.session_id(),
                        attempt: acknowledged.attempt,
                        peer,
                        slot,
                        reconnected: grant.reconnected,
                    },
                );
                let snapshot = world.resource::<PartyAuthority>().snapshot(slot);
                to_client(
                    world,
                    entity,
                    SnapshotEnvelope {
                        attempt: acknowledged.attempt,
                        snapshot,
                    },
                );
            }
            Err(_) => refuse(world, &mut hosted, entity, Refused::Admission),
        }
    }
    let mut per_client = BTreeMap::<Entity, usize>::new();
    for request in requests.into_iter().take(128) {
        let Some(entity) = request.client_id.entity() else {
            continue;
        };
        let count = per_client.entry(entity).or_default();
        *count += 1;
        if *count > 16 || !hosted.security.is_connection_admitted(entity.to_bits()) {
            continue;
        }
        let Some((_, slot)) = hosted.connections.get(&entity).copied() else {
            continue;
        };
        let result = world
            .resource_mut::<PartyAuthority>()
            .apply(slot, request.message);
        to_client(world, entity, result);
    }
    cleanup(world, hosted.security.expire(at));
    let overdue = hosted
        .observed
        .iter()
        .filter(|(entity, begun)| {
            !hosted.security.is_connection_admitted(entity.to_bits())
                && begun.elapsed() >= ADMISSION_TIMEOUT
        })
        .map(|(entity, _)| *entity)
        .collect::<Vec<_>>();
    for entity in overdue {
        refuse(world, &mut hosted, entity, Refused::Admission);
    }
    let rejected = hosted
        .rejected
        .iter()
        .filter(|(_, at)| at.elapsed() >= Duration::from_millis(250))
        .map(|(entity, _)| *entity)
        .collect::<Vec<_>>();
    for entity in rejected {
        hosted.rejected.remove(&entity);
        world.trigger(Disconnect::new(entity, "admission refused"));
    }
    let occupied = world.resource::<PartyAuthority>().occupied();
    if hosted.metadata.claimed_players() != occupied {
        if let Ok(metadata) = SessionMetadata::new(
            GAME_ID,
            PROTOCOL,
            fingerprint_text(),
            hosted.metadata.display_name(),
            occupied,
            PLAYER_CAPACITY,
            hosted.verifier.is_some(),
        ) {
            hosted.providers.refresh(&metadata);
            hosted.metadata = metadata;
        }
    }
    world.insert_resource(hosted);
}

fn offer_peer(world: &mut World, hosted: &mut Hosted, entity: Entity, offer: AdmissionOffer) {
    match world.resource_mut::<PartyAuthority>().reserve(offer.peer) {
        Ok(slot) => {
            let Some(attempt) = hosted.attempts.get(&entity).copied() else {
                cleanup(world, hosted.security.abort(entity.to_bits()));
                return;
            };
            hosted.connections.insert(entity, (offer.peer, slot));
            to_client(
                world,
                entity,
                Offer {
                    session: hosted.security.session_id(),
                    attempt,
                    peer: offer.peer,
                    slot,
                    credential: offer.reconnect_credential,
                    reconnected: offer.reconnected,
                },
            );
        }
        Err(_) => {
            cleanup(world, hosted.security.abort(entity.to_bits()));
            refuse(world, hosted, entity, Refused::Full);
        }
    }
}

fn refuse(world: &mut World, hosted: &mut Hosted, entity: Entity, reason: Refused) {
    if hosted.rejected.contains_key(&entity) {
        return;
    }
    if let Some(attempt) = hosted.attempts.get(&entity).copied() {
        to_client(world, entity, Refusal { attempt, reason });
    }
    hosted.rejected.insert(entity, Instant::now());
}

fn cleanup(world: &mut World, cleanup: bevy_gamekit::session::AdmissionCleanup) {
    for peer in cleanup.released_peers {
        world.resource_mut::<PartyAuthority>().release(peer);
    }
    for connection in cleanup.disconnected_connections {
        if let Some(entity) = Entity::try_from_bits(connection) {
            world.trigger(Disconnect::new(entity, "admission expired"));
        }
    }
}

pub(super) fn reissue(world: &mut World, index: usize) -> Result<(), String> {
    if !world
        .get_resource::<PartyAuthority>()
        .is_some_and(PartyAuthority::has_space)
    {
        return Err("Invitations can be reissued only for an open lobby slot.".into());
    }
    let at = now(world);
    let mut hosted = world
        .remove_resource::<Hosted>()
        .ok_or("Only a host can issue invitations.")?;
    let result = (|| {
        let old = *hosted.invites.get(index).ok_or("Unknown invitation.")?;
        cleanup(world, hosted.security.revoke_invite(old));
        let new = hosted
            .security
            .issue_invite(at, at + Duration::from_secs(3600))
            .map_err(|e| e.to_string())?;
        if let Some(invite) = hosted.invites.get_mut(index) {
            *invite = new;
        }
        Ok(())
    })();
    world.insert_resource(hosted);
    if result.is_ok() {
        notice(
            world,
            "Invitation replaced; the previous unused code is no longer valid.",
        );
    }
    result
}
