//! Optional receive budgets between transport reassembly and Replicon decoding.

use aeronet::{
    io::{
        connection::Disconnect,
        server::{Server, ServerEndpoint},
    },
    transport::{RecvMessage, Transport, TransportSystems},
};
use aeronet_replicon::{
    convert,
    server::{AeronetRepliconServer, ServerTransportSystems},
};
use bevy::prelude::*;
use bevy_replicon::prelude::{AuthorizedClient, ServerMessages};

/// Per-connection limits for one frame, measured before application deserialization.
#[derive(Debug, Clone, Copy)]
pub struct InboundMessageLimit {
    /// Maximum complete messages forwarded in one frame.
    pub messages: usize,
    /// Maximum total payload bytes forwarded in one frame.
    pub bytes: usize,
    /// Maximum bytes in any one complete message.
    pub message_bytes: usize,
}

/// Opt a server entity into receive budgeting. Games choose their own capacities.
///
/// Excess traffic closes only its physical connection, without interpreting game
/// commands or changing a reservation. Transport reassembly and its memory limit
/// precede this gate; these limits bound forwarding/decoding, not upstream IO.
#[derive(Component, Debug, Clone, Copy)]
pub struct InboundLimits {
    /// Maximum simultaneously tracked connections awaiting application admission.
    pub pending_connections: usize,
    /// Maximum admitted connections, counted separately from pending handshakes.
    pub admitted_connections: usize,
    /// Budget used until the game adds `AuthorizedClient` after admission.
    pub pending: InboundMessageLimit,
    /// Budget for admitted connections.
    pub admitted: InboundMessageLimit,
}

/// A connection whose traffic exceeded its budget. Games must ignore any queued
/// commands from this endpoint while the asynchronous disconnect completes.
#[derive(Component, Debug)]
pub struct InboundRejected;

#[derive(Component)]
struct InboundTracked;

/// Installs the receive gate; only servers carrying [`InboundLimits`] opt in.
/// Add alongside [`crate::GameMultiplayerPlugin`]. No sockets are opened here.
pub struct GameInboundBudgetPlugin;

impl Plugin for GameInboundBudgetPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            PreUpdate,
            receive
                .after(TransportSystems::Poll)
                .before(ServerTransportSystems::Poll),
        );
    }
}

fn stage_batch(
    messages: impl IntoIterator<Item = RecvMessage>,
    limit: InboundMessageLimit,
) -> Result<Vec<RecvMessage>, ()> {
    let mut batch = Vec::new();
    let mut bytes = 0usize;
    for message in messages {
        bytes = bytes.checked_add(message.payload.len()).ok_or(())?;
        if batch.len() >= limit.messages
            || bytes > limit.bytes
            || message.payload.len() > limit.message_bytes
        {
            return Err(());
        }
        batch.push(message);
    }
    Ok(batch)
}

fn receive(
    servers: Query<
        &InboundLimits,
        (
            With<Server>,
            With<ServerEndpoint>,
            With<AeronetRepliconServer>,
        ),
    >,
    mut clients: Query<(
        Entity,
        &ChildOf,
        &mut Transport,
        Has<AuthorizedClient>,
        Has<InboundRejected>,
        Has<InboundTracked>,
    )>,
    mut messages: ResMut<ServerMessages>,
    mut commands: Commands,
) {
    // Only opted-in listeners are counted. Existing tracked sockets get capacity
    // before newcomers; a new handshake cannot evict an existing pending socket.
    let mut counts = std::collections::BTreeMap::<Entity, (usize, usize)>::new();
    for tracked_pass in [true, false] {
        for (entity, parent, mut transport, admitted, rejected, tracked) in &mut clients {
            if tracked != tracked_pass {
                continue;
            }
            let Ok(limits) = servers.get(parent.parent()) else {
                continue;
            };
            if rejected {
                drop(transport.recv.msgs.drain());
                continue;
            }
            let (pending_count, admitted_count) = counts.entry(parent.parent()).or_default();
            let (count, capacity, limit) = if admitted {
                (admitted_count, limits.admitted_connections, limits.admitted)
            } else {
                (pending_count, limits.pending_connections, limits.pending)
            };
            let batch = if *count < capacity {
                *count += 1;
                stage_batch(transport.recv.msgs.drain(), limit)
            } else {
                drop(transport.recv.msgs.drain());
                Err(())
            };
            match batch {
                Ok(batch) => {
                    commands.entity(entity).insert(InboundTracked);
                    // This opt-in backend extension consumes the transport buffer;
                    // the normal adapter still owns state, ACKs and outgoing IO.
                    for message in batch {
                        messages.insert_received(
                            entity,
                            convert::to_channel_id(message.lane),
                            message.payload,
                        );
                    }
                }
                Err(()) => {
                    commands
                        .entity(entity)
                        .insert(InboundRejected)
                        .remove::<AuthorizedClient>();
                    commands.trigger(Disconnect::new(entity, "inbound traffic budget exceeded"));
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn message(size: usize) -> RecvMessage {
        RecvMessage {
            lane: convert::to_lane_index(0).expect("lane"),
            recv_at: bevy::platform::time::Instant::now(),
            payload: vec![0; size],
        }
    }

    #[test]
    fn budgets_reject_whole_batch_before_forwarding_and_leave_other_sources_independent() {
        let limit = InboundMessageLimit {
            messages: 3,
            bytes: 12,
            message_bytes: 6,
        };
        assert_eq!(
            stage_batch([message(6), message(6)], limit)
                .expect("byte edge")
                .len(),
            2
        );
        assert_eq!(
            stage_batch([message(4), message(4), message(4)], limit)
                .expect("count edge")
                .len(),
            3
        );
        assert!(stage_batch([message(0), message(0), message(0), message(0)], limit).is_err());
        assert!(stage_batch([message(6), message(6), message(1)], limit).is_err());
        assert!(stage_batch([message(7)], limit).is_err());
        assert_eq!(
            stage_batch([message(4)], limit).expect("quiet peer").len(),
            1
        );
        let mut inspected = 0;
        assert!(stage_batch(
            (0..10000).map(|_| {
                inspected += 1;
                message(1)
            }),
            limit
        )
        .is_err());
        assert_eq!(
            inspected, 4,
            "inspect only the budget plus one overflow item"
        );
    }
}
