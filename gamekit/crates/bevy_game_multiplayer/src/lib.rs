//! Opt-in multiplayer transport and session-security foundations for Bevy games.
//!
//! This crate deliberately does not own game admission policy, seats, lobby rules,
//! commands, snapshots, disclosure, or simulation. Installing [`GameMultiplayerPlugin`]
//! registers transport support but never opens a socket.

mod credentials;
#[cfg(feature = "direct")]
mod direct;
mod lifecycle;
mod local_network;
mod testing;

use aeronet::AeronetPlugins;
use aeronet_replicon::{client::AeronetRepliconClientPlugin, server::AeronetRepliconServerPlugin};
use bevy::prelude::*;
use bevy_replicon::prelude::{AuthMethod, RepliconPlugins, RepliconSharedPlugin};

pub use bevy_game_session::{
    AdmissionCredential, AdmissionError, AdmissionGrant, InviteToken, PeerId, ReconnectCredential,
    SessionId, SessionSecurityAuthority,
};
pub use bevy_game_session::{
    CertificateFingerprint, ConnectionCodeError, DirectConnectionCode, DirectEndpoint,
    DiscoveredDirectTarget, EncodedConnectionCode,
};
pub use credentials::{
    AtomicFileReconnectCredentialStore, CredentialStoreError, MemoryReconnectCredentialStore,
    ReconnectCredentialStorage, ReconnectCredentialStore, ReconnectEndpointBinding,
    StoredReconnectCredential,
};
#[cfg(feature = "direct")]
pub use direct::{
    DirectTransportError, PreparedDirectDiscoveryJoin, PreparedDirectHost, PreparedDirectJoin,
    PreparedDirectReconnect, SpkiPinVerifier, DEFAULT_DIRECT_PORT, DIRECT_SESSION_PATH,
};
pub use lifecycle::{AuthenticatedPeer, MultiplayerLifecycle};
pub use local_network::{
    local_network_addresses, local_network_interface_index, LocalNetworkAddressError,
};
pub use testing::{InMemoryEndpoint, InMemorySessionLink, LinkError};

/// Installs the common Aeronet/Replicon stack and lifecycle vocabulary.
pub struct GameMultiplayerPlugin;

/// Stable ordering seams exposed to game-owned networking systems.
#[derive(SystemSet, Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum MultiplayerSystems {
    /// Receive and authenticate transport input.
    Receive,
    /// Let the game validate admission and reduce commands.
    GameAuthority,
    /// Project and send game-owned outcomes and snapshots.
    Send,
}

impl Plugin for GameMultiplayerPlugin {
    fn build(&self, app: &mut App) {
        if !app.is_plugin_added::<bevy::state::app::StatesPlugin>() {
            app.add_plugins(bevy::state::app::StatesPlugin);
        }
        app.add_plugins(AeronetPlugins)
            .add_plugins(RepliconPlugins.set(RepliconSharedPlugin {
                auth_method: AuthMethod::Custom,
            }))
            .add_plugins((AeronetRepliconClientPlugin, AeronetRepliconServerPlugin))
            .configure_sets(
                PreUpdate,
                (
                    MultiplayerSystems::Receive,
                    MultiplayerSystems::GameAuthority,
                )
                    .chain()
                    .after(bevy_replicon::prelude::ClientSystems::Receive)
                    .after(bevy_replicon::prelude::ServerSystems::Receive),
            );

        lifecycle::install(app);
        app.configure_sets(
            PostUpdate,
            MultiplayerSystems::Send
                .before(bevy_replicon::prelude::ClientSystems::Send)
                .before(bevy_replicon::prelude::ServerSystems::Send),
        );

        #[cfg(feature = "direct")]
        app.add_plugins((
            aeronet_webtransport::client::WebTransportClientPlugin,
            aeronet_webtransport::server::WebTransportServerPlugin,
        ))
        .add_observer(direct::respond_to_direct_session);
    }
}

#[cfg(test)]
mod tests {
    use aeronet::io::{
        server::{Server, ServerEndpoint},
        SessionEndpoint,
    };

    use super::*;

    #[test]
    fn plugin_does_not_open_a_socket() {
        let mut app = App::new();
        app.add_plugins((MinimalPlugins, GameMultiplayerPlugin));
        app.finish();

        let sessions = app
            .world_mut()
            .query_filtered::<Entity, With<SessionEndpoint>>()
            .iter(app.world())
            .count();
        let servers = app
            .world_mut()
            .query_filtered::<Entity, (With<ServerEndpoint>, With<Server>)>()
            .iter(app.world())
            .count();
        assert_eq!(sessions, 0);
        assert_eq!(servers, 0);
    }
}
