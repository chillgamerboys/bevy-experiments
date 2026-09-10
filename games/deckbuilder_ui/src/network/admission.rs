//! Game-owned two-seat admission policy around the shared persistence-ACK authority.

use super::*;

pub(super) fn handle_client_hellos(
    mut hellos: MessageReader<FromClient<DeckClientHello>>,
    mut hosted: Option<ResMut<HostedSession>>,
    connected: Query<(&PeerAddr, &ChildOf), With<ConnectedClient>>,
    mut commands: Commands,
    mut offered: MessageWriter<ToClients<DeckOffer>>,
    mut refused: MessageWriter<ToClients<DeckAdmissionRefusal>>,
    mut state: ResMut<DeckNetworkState>,
) {
    let Some(hosted) = hosted.as_mut() else {
        return;
    };
    for hello in hellos.read().take(8) {
        let Some(connection) = hello.client_id.entity() else {
            continue;
        };
        // Record the first hello immediately, before deferred authorization. Neither
        // duplicate credentials nor a different nonce may reset an active attempt.
        if !connected
            .get(connection)
            .is_ok_and(|(_, listener)| listener.parent() == hosted.server_entity)
            || hosted.attempts.contains_key(&connection)
        {
            continue;
        }
        if hosted.attempts.len() >= 8 {
            refused.write(ToClients {
                targets: SendTargets::Single(hello.client_id),
                message: DeckAdmissionRefusal {
                    attempt: hello.attempt,
                },
            });
            commands
                .entity(connection)
                .insert(RejectedConnection(Instant::now()));
            continue;
        }
        hosted.attempts.insert(connection, hello.attempt);
        if !hello.attempt.is_valid()
            || hello.session != hosted.security.session_id()
            || hello.protocol != PROTOCOL_SCHEMA
            || hello.build != BUILD_ID
        {
            refused.write(ToClients {
                targets: SendTargets::Single(hello.client_id),
                message: DeckAdmissionRefusal {
                    attempt: hello.attempt,
                },
            });
            commands
                .entity(connection)
                .insert(RejectedConnection(std::time::Instant::now()));
            continue;
        }
        let now = hosted.clock.elapsed();
        let offer = match &hello.credential {
            DeckCredential::Invite(token) => hosted.security.begin(
                connection.to_bits(),
                AdmissionCredential::Invite(*token),
                now,
            ),
            DeckCredential::Reconnect(credential) => hosted.security.begin(
                connection.to_bits(),
                AdmissionCredential::Reconnect(*credential),
                now,
            ),
            DeckCredential::Password(password) => {
                // Capacity includes disconnected and pending reservations. A new
                // password cannot take an existing guest's private hand.
                if hosted.security.reserved_peer_count() != 0 {
                    refused.write(ToClients {
                        targets: SendTargets::Single(hello.client_id),
                        message: DeckAdmissionRefusal {
                            attempt: hello.attempt,
                        },
                    });
                    commands
                        .entity(connection)
                        .insert(RejectedConnection(Instant::now()));
                    continue;
                }
                let Ok(source) = connected.get(connection).map(|(address, _)| address.0.ip())
                else {
                    continue;
                };
                SessionPassword::new(password.0.clone())
                    .map_err(|_error| AdmissionFlowError::InvalidInvite)
                    .and_then(|password| {
                        hosted
                            .verifier
                            .as_mut()
                            .ok_or(AdmissionFlowError::InvalidInvite)?
                            .verify(source, now, &password)
                            .map_err(|_error| AdmissionFlowError::InvalidInvite)
                    })
                    .and_then(|()| hosted.security.begin_external(connection.to_bits(), now))
            }
        };
        let Ok(offer) = offer else {
            refused.write(ToClients {
                targets: SendTargets::Single(hello.client_id),
                message: DeckAdmissionRefusal {
                    attempt: hello.attempt,
                },
            });
            commands
                .entity(connection)
                .insert(RejectedConnection(std::time::Instant::now()));
            continue;
        };
        hosted.guest_peer = Some(offer.peer);
        if let Err(error) = refresh_host_listing(hosted, 2) {
            state.notice = Some(error);
        }
        offered.write(ToClients {
            targets: SendTargets::Single(hello.client_id),
            message: DeckOffer {
                attempt: hello.attempt,
                reconnected: offer.reconnected,
                session_id: hosted.security.session_id(),
                peer_id: offer.peer,
                seat: Seat::Guest,
                reconnect_credential: offer.reconnect_credential,
            },
        });
    }
}

pub(super) fn handle_persistence(
    mut acknowledgements: MessageReader<FromClient<DeckPersistence>>,
    mut hosted: Option<ResMut<HostedSession>>,
    mut authority: Option<ResMut<DeckAuthority>>,
    connected: Query<&ChildOf, With<ConnectedClient>>,
    mut commands: Commands,
    mut accepted: MessageWriter<ToClients<DeckWelcome>>,
    mut refused: MessageWriter<ToClients<DeckAdmissionRefusal>>,
    mut snapshots: MessageWriter<ToClients<DeckSnapshot>>,
    mut state: ResMut<DeckNetworkState>,
) {
    let (Some(hosted), Some(authority)) = (hosted.as_mut(), authority.as_mut()) else {
        return;
    };
    for acknowledgement in acknowledgements.read().take(16) {
        let Some(connection) = acknowledgement.client_id.entity() else {
            continue;
        };
        if !connected
            .get(connection)
            .is_ok_and(|listener| listener.parent() == hosted.server_entity)
            || hosted.attempts.get(&connection) != Some(&acknowledgement.attempt)
        {
            continue;
        }
        let Some(credential) = acknowledgement.credential else {
            // Persistence failure may cancel only an unadmitted physical attempt.
            if !hosted.security.is_connection_admitted(connection.to_bits()) {
                let cleanup = hosted.security.abort(connection.to_bits());
                apply_admission_cleanup(hosted, cleanup, &mut commands);
                let claimed = if hosted.guest_peer.is_some() { 2 } else { 1 };
                if let Err(error) = refresh_host_listing(hosted, claimed) {
                    state.notice = Some(error);
                }
            }
            continue;
        };
        let now = hosted.clock.elapsed();
        let was_admitted = hosted.security.is_connection_admitted(connection.to_bits());
        let Ok(grant) = hosted
            .security
            .acknowledge(connection.to_bits(), credential, now)
        else {
            refused.write(ToClients {
                targets: SendTargets::Single(acknowledgement.client_id),
                message: DeckAdmissionRefusal {
                    attempt: acknowledgement.attempt,
                },
            });
            commands
                .entity(connection)
                .insert(RejectedConnection(Instant::now()));
            continue;
        };
        if !was_admitted {
            authority.set_connected(Seat::Guest, true);
            state.latest = Some(authority.snapshot(Seat::Host));
            commands.entity(connection).insert((
                AuthorizedClient,
                bevy_gamekit::multiplayer::AuthenticatedPeer {
                    peer: grant.peer,
                    reconnected: grant.reconnected,
                },
                DeckAuthorized {
                    session: hosted.security.session_id(),
                    peer: grant.peer,
                    seat: Seat::Guest,
                    attempt: acknowledgement.attempt,
                },
            ));
        }
        accepted.write(ToClients {
            targets: SendTargets::Single(acknowledgement.client_id),
            message: DeckWelcome {
                attempt: acknowledgement.attempt,
                session_id: hosted.security.session_id(),
                peer_id: grant.peer,
                reconnected: grant.reconnected,
            },
        });
        snapshots.write(ToClients {
            targets: SendTargets::Single(acknowledgement.client_id),
            message: DeckSnapshot {
                attempt: acknowledgement.attempt,
                snapshot: authority.snapshot(Seat::Guest),
            },
        });
    }
}

fn apply_admission_cleanup(
    hosted: &mut HostedSession,
    cleanup: AdmissionCleanup,
    commands: &mut Commands,
) {
    if hosted
        .guest_peer
        .is_some_and(|peer| cleanup.released_peers.contains(&peer))
    {
        hosted.guest_peer = None;
    }
    for connection in cleanup.disconnected_connections {
        if let Some(entity) = hosted
            .attempts
            .keys()
            .find(|entity| entity.to_bits() == connection)
            .copied()
        {
            commands.trigger(Disconnect::new(entity, "admission ended"));
        }
    }
}

pub(super) fn expire_admission(
    mut hosted: Option<ResMut<HostedSession>>,
    mut commands: Commands,
    mut state: ResMut<DeckNetworkState>,
) {
    let Some(hosted) = hosted.as_mut() else {
        return;
    };
    let now = hosted.clock.elapsed();
    let cleanup = hosted.security.expire(now);
    let released = !cleanup.released_peers.is_empty();
    apply_admission_cleanup(hosted, cleanup, &mut commands);
    if released {
        if let Err(error) = refresh_host_listing(hosted, 1) {
            state.notice = Some(error);
        }
    }
}
