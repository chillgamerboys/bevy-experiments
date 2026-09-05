//! Entity-bound lifecycle emitted from actual transport and admission transitions.

use crate::PeerId;
use aeronet::io::{
    connection::{Disconnect, DisconnectReason, Disconnected},
    server::{Closed, Server},
    Session, SessionEndpoint,
};
use bevy::prelude::*;

/// Game admission attaches this to the exact authenticated transport entity.
/// Adding it emits [`MultiplayerLifecycle::Authenticated`]. It does not grant
/// game seats or bypass the game's admission checks.
#[derive(Component, Debug, Clone, Copy)]
pub struct AuthenticatedPeer {
    /// Stable session identity assigned by the session authority.
    pub peer: PeerId,
    /// Whether this transport reclaimed a reserved identity.
    pub reconnected: bool,
}

/// Actual connection transitions; every event identifies its endpoint entity.
#[derive(Message, Debug, Clone, PartialEq, Eq)]
pub enum MultiplayerLifecycle {
    /// An outgoing or incoming transport is being established.
    Connecting {
        /// Endpoint being established.
        entity: Entity,
    },
    /// A listener or encrypted connection opened; not yet game admission.
    Opened {
        /// Listener or connection.
        entity: Entity,
    },
    /// Game admission accepted a peer on this exact connection.
    Authenticated {
        /// Authenticated connection.
        entity: Entity,
        /// Stable peer identity.
        peer: PeerId,
        /// Reclaimed identity.
        reconnected: bool,
    },
    /// Explicit disconnect has been requested; cleanup is deferred.
    Leaving {
        /// Closing connection.
        entity: Entity,
    },
    /// A connection ended, whether or not it reached admission.
    Disconnected {
        /// Ended connection.
        entity: Entity,
        /// Identity when previously admitted.
        peer: Option<PeerId>,
    },
    /// A listener closed.
    Closed {
        /// Closed listener.
        entity: Entity,
    },
    /// A connection ended with an I/O/protocol error; raw details may contain
    /// sensitive endpoint information and are deliberately not forwarded.
    Failed {
        /// Failed connection.
        entity: Entity,
    },
}

pub(crate) fn install(app: &mut App) {
    app.add_message::<MultiplayerLifecycle>()
        .add_observer(connecting)
        .add_observer(opened)
        .add_observer(listening)
        .add_observer(authenticated)
        .add_observer(leaving)
        .add_observer(disconnected)
        .add_observer(closed);
}

fn connecting(event: On<Add, SessionEndpoint>, mut messages: MessageWriter<MultiplayerLifecycle>) {
    messages.write(MultiplayerLifecycle::Connecting {
        entity: event.entity,
    });
}
fn opened(event: On<Add, Session>, mut messages: MessageWriter<MultiplayerLifecycle>) {
    messages.write(MultiplayerLifecycle::Opened {
        entity: event.entity,
    });
}
fn listening(event: On<Add, Server>, mut messages: MessageWriter<MultiplayerLifecycle>) {
    messages.write(MultiplayerLifecycle::Opened {
        entity: event.entity,
    });
}
fn authenticated(
    event: On<Add, AuthenticatedPeer>,
    peers: Query<&AuthenticatedPeer>,
    mut messages: MessageWriter<MultiplayerLifecycle>,
) {
    if let Ok(peer) = peers.get(event.entity) {
        messages.write(MultiplayerLifecycle::Authenticated {
            entity: event.entity,
            peer: peer.peer,
            reconnected: peer.reconnected,
        });
    }
}
fn leaving(event: On<Disconnect>, mut messages: MessageWriter<MultiplayerLifecycle>) {
    messages.write(MultiplayerLifecycle::Leaving {
        entity: event.entity,
    });
}
fn disconnected(
    event: On<Disconnected>,
    peers: Query<&AuthenticatedPeer>,
    mut messages: MessageWriter<MultiplayerLifecycle>,
) {
    if matches!(event.reason, DisconnectReason::ByError(_)) {
        messages.write(MultiplayerLifecycle::Failed {
            entity: event.entity,
        });
    }
    messages.write(MultiplayerLifecycle::Disconnected {
        entity: event.entity,
        peer: peers.get(event.entity).ok().map(|peer| peer.peer),
    });
}
fn closed(event: On<Closed>, mut messages: MessageWriter<MultiplayerLifecycle>) {
    messages.write(MultiplayerLifecycle::Closed {
        entity: event.entity,
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn admission_and_disconnect_messages_identify_the_real_entity() {
        let mut app = App::new();
        app.add_plugins((MinimalPlugins, crate::GameMultiplayerPlugin));
        app.finish();
        let peer = PeerId::from_bytes([4; 16]);
        let entity = app
            .world_mut()
            .spawn(AuthenticatedPeer {
                peer,
                reconnected: true,
            })
            .id();
        app.world_mut().flush();
        let events = app
            .world_mut()
            .resource_mut::<Messages<MultiplayerLifecycle>>()
            .drain()
            .collect::<Vec<_>>();
        assert!(events.contains(&MultiplayerLifecycle::Authenticated {
            entity,
            peer,
            reconnected: true
        }));
        app.world_mut().trigger(Disconnect::new(entity, "fixture"));
        app.world_mut().flush();
        let events = app
            .world_mut()
            .resource_mut::<Messages<MultiplayerLifecycle>>()
            .drain()
            .collect::<Vec<_>>();
        assert!(events.contains(&MultiplayerLifecycle::Leaving { entity }));
        assert!(events.contains(&MultiplayerLifecycle::Disconnected {
            entity,
            peer: Some(peer)
        }));
        assert!(app.world().get_entity(entity).is_err());
    }
}
