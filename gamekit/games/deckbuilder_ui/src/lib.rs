//! A small game-owned adopter of the reusable Gamekit capabilities.

mod domain;
mod network;

use bevy::prelude::*;
use bevy_game_discovery::{
    Compatibility, DiscoveryEvent, DiscoveryJoinRoute, DiscoveryRegistry, DiscoverySource,
};
use bevy_game_multiplayer::SessionId;
use bevy_game_ui::{
    button, card, modal, panel, region, screen_root, text, text_field, GameUiSystems,
    ResolvedUiMetrics, UiActivated, UiDisabled, UiFonts, UiRegionRole, UiTextChanged, UiTextRole,
    UiTextSubmitted, UiViewportClass,
};

use crate::{
    domain::{CardKind, GameCommand, MatchPhase},
    network::{DeckNetworkState, HostConfiguration, NetworkRole},
};

/// Installs deckbuilder-owned presentation, rules, networking, and composition.
pub struct DeckbuilderPlugin;

impl Plugin for DeckbuilderPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(network::DeckNetworkPlugin)
            .init_resource::<DeckbuilderUi>()
            .init_resource::<PendingActions>()
            .init_resource::<UiDirty>()
            .init_resource::<BrowserTick>()
            .add_systems(Startup, render_if_dirty)
            .add_systems(
                Update,
                (
                    collect_activations.after(GameUiSystems::EmitActivations),
                    collect_text,
                    apply_pending_actions,
                    synchronize_network_screen,
                    mark_discovery_change,
                    mark_browser_tick,
                    mark_responsive_change,
                    render_if_dirty,
                )
                    .chain(),
            );
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Screen {
    Menu,
    Multiplayer,
    Host,
    Direct,
    Browser,
    Password,
    Lobby,
    Match,
}

#[derive(Resource, Debug)]
struct DeckbuilderUi {
    screen: Screen,
    previous: Screen,
    session_name: String,
    passphrase: String,
    advertised_host: String,
    port: String,
    direct_code: String,
    discover_lan: bool,
    discover_tailnet: bool,
    selected_session: Option<SessionId>,
    selected_target: Option<DiscoveryJoinRoute>,
    selected_card: Option<CardKind>,
    paused: bool,
    share_code: Option<String>,
    local_notice: Option<String>,
}

impl Default for DeckbuilderUi {
    fn default() -> Self {
        Self {
            screen: Screen::Menu,
            previous: Screen::Menu,
            session_name: "Tavern Table".to_owned(),
            passphrase: String::new(),
            advertised_host: network::default_advertised_host().unwrap_or_default(),
            port: "7777".to_owned(),
            direct_code: String::new(),
            discover_lan: true,
            discover_tailnet: false,
            selected_session: None,
            selected_target: None,
            selected_card: None,
            paused: false,
            share_code: None,
            local_notice: None,
        }
    }
}

#[derive(Resource, Debug, Default)]
struct PendingActions(Vec<DeckbuilderAction>);

#[derive(Resource, Debug)]
struct UiDirty(bool);

#[derive(Resource, Debug, Default)]
struct BrowserTick(u64);

impl Default for UiDirty {
    fn default() -> Self {
        Self(true)
    }
}

#[derive(Component, Debug, Clone, PartialEq, Eq)]
enum DeckbuilderAction {
    StartSolo,
    OpenMultiplayer,
    OpenHost,
    OpenDirect,
    OpenBrowser,
    Reconnect,
    Back,
    ToggleLan,
    ToggleTailnet,
    HostSession,
    CopyShareCode,
    JoinDirect,
    SelectSession(SessionId),
    JoinSelected,
    SubmitPassword,
    SetReady(bool),
    StartNetworkMatch,
    SelectCard(CardKind),
    PlaySelected,
    EndTurn,
    Pause,
    Resume,
    ReturnToMenu,
}

#[derive(Component, Debug, Clone, Copy, PartialEq, Eq)]
enum Field {
    SessionName,
    Passphrase,
    AdvertisedHost,
    Port,
    DirectCode,
    JoinPassphrase,
}

#[derive(Component)]
struct DeckbuilderUiRoot;

fn collect_activations(
    mut activations: MessageReader<UiActivated>,
    actions: Query<&DeckbuilderAction>,
    mut pending: ResMut<PendingActions>,
) {
    for activation in activations.read() {
        if let Ok(action) = actions.get(activation.entity) {
            pending.0.push(action.clone());
        }
    }
}

fn collect_text(
    mut changed: MessageReader<UiTextChanged>,
    mut submitted: MessageReader<UiTextSubmitted>,
    fields: Query<&Field>,
    mut ui: ResMut<DeckbuilderUi>,
    mut pending: ResMut<PendingActions>,
) {
    for change in changed.read() {
        if let Ok(field) = fields.get(change.entity) {
            set_field(&mut ui, *field, change.value.clone());
        }
    }
    for submission in submitted.read() {
        let Ok(field) = fields.get(submission.entity) else {
            continue;
        };
        set_field(&mut ui, *field, submission.value.clone());
        let action = match field {
            Field::DirectCode => Some(DeckbuilderAction::JoinDirect),
            Field::JoinPassphrase => Some(DeckbuilderAction::SubmitPassword),
            _ => None,
        };
        pending.0.extend(action);
    }
}

fn set_field(ui: &mut DeckbuilderUi, field: Field, value: String) {
    match field {
        Field::SessionName => ui.session_name = value,
        Field::Passphrase | Field::JoinPassphrase => ui.passphrase = value,
        Field::AdvertisedHost => ui.advertised_host = value,
        Field::Port => ui.port = value,
        Field::DirectCode => ui.direct_code = value,
    }
}

fn apply_pending_actions(world: &mut World) {
    let actions = std::mem::take(&mut world.resource_mut::<PendingActions>().0);
    for action in actions {
        apply_action(world, action);
    }
}

fn apply_action(world: &mut World, action: DeckbuilderAction) {
    match action {
        DeckbuilderAction::StartSolo => {
            network::start_solo(world);
            let mut ui = world.resource_mut::<DeckbuilderUi>();
            ui.screen = Screen::Match;
            ui.selected_card = None;
        }
        DeckbuilderAction::OpenMultiplayer => set_screen(world, Screen::Multiplayer),
        DeckbuilderAction::OpenHost => set_screen(world, Screen::Host),
        DeckbuilderAction::OpenDirect => set_screen(world, Screen::Direct),
        DeckbuilderAction::OpenBrowser => {
            let discover_tailnet = world.resource::<DeckbuilderUi>().discover_tailnet;
            network::start_browser(world, discover_tailnet);
            set_screen(world, Screen::Browser);
        }
        DeckbuilderAction::Reconnect => {
            if let Err(error) = network::reconnect(world) {
                set_notice(world, error);
            } else {
                world.resource_mut::<DeckbuilderUi>().local_notice = None;
                set_screen(world, Screen::Lobby);
            }
        }
        DeckbuilderAction::Back => {
            if world.resource::<DeckbuilderUi>().screen == Screen::Browser {
                network::stop_browser(world);
            }
            let previous = world.resource::<DeckbuilderUi>().previous;
            set_screen(world, previous);
        }
        DeckbuilderAction::ToggleLan => {
            let enabled = world.resource::<DeckbuilderUi>().discover_lan;
            world.resource_mut::<DeckbuilderUi>().discover_lan = !enabled;
        }
        DeckbuilderAction::ToggleTailnet => {
            let enabled = world.resource::<DeckbuilderUi>().discover_tailnet;
            world.resource_mut::<DeckbuilderUi>().discover_tailnet = !enabled;
        }
        DeckbuilderAction::HostSession => host_session(world),
        DeckbuilderAction::CopyShareCode => copy_share_code(world),
        DeckbuilderAction::JoinDirect => join_direct(world),
        DeckbuilderAction::SelectSession(session_id) => select_session(world, session_id),
        DeckbuilderAction::JoinSelected => {
            if world.resource::<DeckbuilderUi>().selected_target.is_some() {
                set_screen(world, Screen::Password);
            }
        }
        DeckbuilderAction::SubmitPassword => join_discovered(world),
        DeckbuilderAction::SetReady(ready) => {
            network::submit_command(world, GameCommand::SetReady(ready));
        }
        DeckbuilderAction::StartNetworkMatch => {
            network::submit_command(world, GameCommand::StartMatch);
        }
        DeckbuilderAction::SelectCard(card) => {
            world.resource_mut::<DeckbuilderUi>().selected_card = Some(card);
        }
        DeckbuilderAction::PlaySelected => {
            if let Some(card) = world.resource::<DeckbuilderUi>().selected_card {
                network::submit_command(world, GameCommand::PlayCard(card));
                world.resource_mut::<DeckbuilderUi>().selected_card = None;
            }
        }
        DeckbuilderAction::EndTurn => {
            network::submit_command(world, GameCommand::EndTurn);
            world.resource_mut::<DeckbuilderUi>().selected_card = None;
        }
        DeckbuilderAction::Pause => world.resource_mut::<DeckbuilderUi>().paused = true,
        DeckbuilderAction::Resume => world.resource_mut::<DeckbuilderUi>().paused = false,
        DeckbuilderAction::ReturnToMenu => {
            network::stop_browser(world);
            network::close_session(world);
            let mut ui = world.resource_mut::<DeckbuilderUi>();
            ui.screen = Screen::Menu;
            ui.previous = Screen::Menu;
            ui.paused = false;
            ui.passphrase.clear();
            ui.direct_code.clear();
            ui.selected_target = None;
            ui.selected_session = None;
        }
    }
    world.resource_mut::<UiDirty>().0 = true;
}

fn set_screen(world: &mut World, screen: Screen) {
    let mut ui = world.resource_mut::<DeckbuilderUi>();
    let current = ui.screen;
    ui.previous = match screen {
        Screen::Host | Screen::Direct | Screen::Browser => Screen::Multiplayer,
        Screen::Password => Screen::Browser,
        _ => current,
    };
    ui.screen = screen;
}

fn set_notice(world: &mut World, notice: impl Into<String>) {
    world.resource_mut::<DeckbuilderUi>().local_notice = Some(notice.into());
}

fn host_session(world: &mut World) {
    let config = {
        let ui = world.resource::<DeckbuilderUi>();
        ui.port.parse::<u16>().ok().map(|port| HostConfiguration {
            session_name: ui.session_name.clone(),
            password: ui.passphrase.clone(),
            advertised_host: ui.advertised_host.clone(),
            port,
            discover_lan: ui.discover_lan,
            discover_tailnet: ui.discover_tailnet,
        })
    };
    let Some(config) = config else {
        set_notice(world, "Port must be between 1 and 65535.");
        return;
    };
    match network::start_host(world, config) {
        Ok(()) => {
            let code = network::hosted_code(world);
            let mut ui = world.resource_mut::<DeckbuilderUi>();
            ui.share_code = code;
            ui.passphrase.clear();
            ui.local_notice = None;
            ui.screen = Screen::Lobby;
        }
        Err(error) => set_notice(world, error),
    }
}

fn join_direct(world: &mut World) {
    let mut code = std::mem::take(&mut world.resource_mut::<DeckbuilderUi>().direct_code);
    let result = network::start_direct_join(world, code.trim());
    code.clear();
    match result {
        Ok(()) => {
            world.resource_mut::<DeckbuilderUi>().local_notice = None;
            set_screen(world, Screen::Lobby);
        }
        Err(error) => set_notice(world, error),
    }
}

fn copy_share_code(world: &mut World) {
    let code = world.resource::<DeckbuilderUi>().share_code.clone();
    let result = code
        .ok_or_else(|| "No private connection code is available.".to_owned())
        .and_then(|code| {
            world
                .get_resource_mut::<Clipboard>()
                .ok_or_else(|| "System clipboard support is unavailable.".to_owned())?
                .set_text(code)
                .map_err(|error| format!("Could not copy the connection code: {error}"))
        });
    match result {
        Ok(()) => set_notice(world, "Private BGN1 code copied to the clipboard."),
        Err(error) => set_notice(world, error),
    }
}

fn select_session(world: &mut World, session_id: SessionId) {
    match world.resource::<DiscoveryRegistry>().resolve(session_id) {
        Ok(target) => {
            let mut ui = world.resource_mut::<DeckbuilderUi>();
            ui.selected_session = Some(session_id);
            ui.selected_target = Some(target);
            ui.local_notice = None;
        }
        Err(error) => set_notice(world, error.to_string()),
    }
}

fn join_discovered(world: &mut World) {
    let (target, mut password) = {
        let mut ui = world.resource_mut::<DeckbuilderUi>();
        (
            ui.selected_target.clone(),
            std::mem::take(&mut ui.passphrase),
        )
    };
    let result = target
        .as_ref()
        .ok_or_else(|| "The selected session is no longer available.".to_owned())
        .and_then(|target| network::start_discovered_join(world, target, password.clone()));
    password.clear();
    match result {
        Ok(()) => {
            network::stop_browser(world);
            world.resource_mut::<DeckbuilderUi>().local_notice = None;
            set_screen(world, Screen::Lobby);
        }
        Err(error) => set_notice(world, error),
    }
}

fn synchronize_network_screen(
    state: Res<DeckNetworkState>,
    mut ui: ResMut<DeckbuilderUi>,
    mut dirty: ResMut<UiDirty>,
) {
    let Some(snapshot) = state.latest.as_ref() else {
        if state.is_changed() {
            dirty.0 = true;
        }
        return;
    };
    let next = match snapshot.phase {
        MatchPhase::Lobby => Screen::Lobby,
        MatchPhase::Playing => Screen::Match,
    };
    if state.role != NetworkRole::None && ui.screen != Screen::Password && ui.screen != next {
        ui.screen = next;
        ui.selected_card = None;
        dirty.0 = true;
    }
    if state.is_changed() {
        dirty.0 = true;
    }
}

fn mark_discovery_change(mut events: MessageReader<DiscoveryEvent>, mut dirty: ResMut<UiDirty>) {
    if events.read().count() > 0 {
        dirty.0 = true;
    }
}

fn mark_browser_tick(
    time: Res<Time<Real>>,
    ui: Res<DeckbuilderUi>,
    mut tick: ResMut<BrowserTick>,
    mut dirty: ResMut<UiDirty>,
) {
    let next = time.elapsed_secs() as u64;
    if ui.screen == Screen::Browser && tick.0 != next {
        tick.0 = next;
        dirty.0 = true;
    }
}

fn mark_responsive_change(metrics: Res<ResolvedUiMetrics>, mut dirty: ResMut<UiDirty>) {
    if metrics.is_changed() {
        dirty.0 = true;
    }
}

fn render_if_dirty(
    mut commands: Commands,
    ui: Res<DeckbuilderUi>,
    state: Res<DeckNetworkState>,
    registry: Res<DiscoveryRegistry>,
    time: Res<Time<Real>>,
    metrics: Res<ResolvedUiMetrics>,
    fonts: Res<UiFonts>,
    mut dirty: ResMut<UiDirty>,
    roots: Query<Entity, With<DeckbuilderUiRoot>>,
) {
    if !dirty.0 {
        return;
    }
    for root in &roots {
        commands.entity(root).try_despawn();
    }
    match ui.screen {
        Screen::Menu => spawn_menu(&mut commands, &fonts, &ui, &state),
        Screen::Multiplayer => spawn_multiplayer(&mut commands, &fonts, &ui, &state),
        Screen::Host => spawn_host(&mut commands, &fonts, &ui, &state),
        Screen::Direct => spawn_direct(&mut commands, &fonts, &ui, &state),
        Screen::Browser => spawn_browser(
            &mut commands,
            &fonts,
            &ui,
            &state,
            &registry,
            time.elapsed(),
        ),
        Screen::Password => spawn_password(&mut commands, &fonts, &ui, &state),
        Screen::Lobby => spawn_lobby(&mut commands, &fonts, &ui, &state),
        Screen::Match => spawn_match(&mut commands, &fonts, &ui, &state, metrics.viewport),
    }
    dirty.0 = false;
}

fn spawn_menu(
    commands: &mut Commands,
    fonts: &UiFonts,
    ui: &DeckbuilderUi,
    state: &DeckNetworkState,
) {
    commands
        .spawn((screen_root("Deckbuilder Menu"), DeckbuilderUiRoot))
        .with_children(|root| {
            root.spawn(text(fonts, UiTextRole::Display, "Arcana Workshop"));
            root.spawn(text(
                fonts,
                UiTextRole::Supporting,
                "Game-owned rules on opt-in Gamekit capabilities.",
            ));
            spawn_action(
                root,
                fonts,
                "Start Solo",
                DeckbuilderAction::StartSolo,
                false,
            );
            spawn_action(
                root,
                fonts,
                "Multiplayer",
                DeckbuilderAction::OpenMultiplayer,
                false,
            );
            spawn_notices(root, fonts, ui, state);
        });
}

fn spawn_multiplayer(
    commands: &mut Commands,
    fonts: &UiFonts,
    ui: &DeckbuilderUi,
    state: &DeckNetworkState,
) {
    commands
        .spawn((screen_root("Multiplayer Menu"), DeckbuilderUiRoot))
        .with_children(|root| {
            root.spawn(text(fonts, UiTextRole::Display, "Multiplayer"));
            root.spawn(text(
                fonts,
                UiTextRole::Supporting,
                "Host, discover, or use a private BGN1 code.",
            ));
            spawn_action(
                root,
                fonts,
                "Host Session",
                DeckbuilderAction::OpenHost,
                false,
            );
            spawn_action(
                root,
                fonts,
                "Find Sessions",
                DeckbuilderAction::OpenBrowser,
                false,
            );
            spawn_action(
                root,
                fonts,
                if ui.discover_tailnet {
                    "Tailnet Browser: On (Dev)"
                } else {
                    "Tailnet Browser: Off (Dev)"
                },
                DeckbuilderAction::ToggleTailnet,
                false,
            );
            spawn_action(
                root,
                fonts,
                "Join with BGN1 Code",
                DeckbuilderAction::OpenDirect,
                false,
            );
            spawn_action(
                root,
                fonts,
                "Reconnect Reserved Seat",
                DeckbuilderAction::Reconnect,
                false,
            );
            spawn_action(root, fonts, "Back", DeckbuilderAction::Back, false);
            spawn_notices(root, fonts, ui, state);
        });
}

fn spawn_host(
    commands: &mut Commands,
    fonts: &UiFonts,
    ui: &DeckbuilderUi,
    state: &DeckNetworkState,
) {
    commands
        .spawn((screen_root("Host Session Form"), DeckbuilderUiRoot))
        .with_children(|root| {
            root.spawn(panel("Host Settings"))
                .insert((
                    ScrollPosition::default(),
                    Node {
                        width: Val::Percent(96.0),
                        max_height: Val::Percent(94.0),
                        flex_direction: FlexDirection::Column,
                        row_gap: Val::Px(12.0),
                        padding: UiRect::all(Val::Px(18.0)),
                        border: UiRect::all(Val::Px(1.0)),
                        border_radius: BorderRadius::all(Val::Px(10.0)),
                        overflow: Overflow::scroll_y(),
                        ..default()
                    },
                ))
                .with_children(|form| {
                    form.spawn(text(fonts, UiTextRole::Title, "Host a Session"));
                    spawn_labeled_field(
                        form,
                        fonts,
                        "Session name",
                        Field::SessionName,
                        &ui.session_name,
                        48,
                    );
                    spawn_labeled_field(
                        form,
                        fonts,
                        "Temporary session passphrase",
                        Field::Passphrase,
                        &ui.passphrase,
                        64,
                    );
                    form.spawn(text(
                    fonts,
                    UiTextRole::Metadata,
                    "Use 8–64 printable characters. Do not reuse an account or important password.",
                ));
                    spawn_labeled_field(
                        form,
                        fonts,
                        "Direct-code advertised address",
                        Field::AdvertisedHost,
                        &ui.advertised_host,
                        255,
                    );
                    form.spawn(text(
                        fonts,
                        UiTextRole::Metadata,
                        "Pre-filled from an active LAN interface. BGN1 embeds this address; LAN and Tailscale discovery publish their own routes.",
                    ));
                    spawn_labeled_field(form, fonts, "Game port", Field::Port, &ui.port, 5);
                    spawn_action(
                        form,
                        fonts,
                        if ui.discover_lan {
                            "Discover on LAN: On"
                        } else {
                            "Discover on LAN: Off"
                        },
                        DeckbuilderAction::ToggleLan,
                        false,
                    );
                    spawn_action(
                        form,
                        fonts,
                        if ui.discover_tailnet {
                            "Discover over Tailscale: On (Dev)"
                        } else {
                            "Discover over Tailscale: Off (Dev)"
                        },
                        DeckbuilderAction::ToggleTailnet,
                        false,
                    );
                    spawn_action(form, fonts, "Host", DeckbuilderAction::HostSession, false);
                    spawn_action(form, fonts, "Back", DeckbuilderAction::Back, false);
                    spawn_notices(form, fonts, ui, state);
                });
        });
}

fn spawn_direct(
    commands: &mut Commands,
    fonts: &UiFonts,
    ui: &DeckbuilderUi,
    state: &DeckNetworkState,
) {
    commands
        .spawn((screen_root("Direct Join Form"), DeckbuilderUiRoot))
        .with_children(|root| {
            root.spawn(panel("Direct Join")).with_children(|form| {
                form.spawn(text(fonts, UiTextRole::Title, "Private Direct Join"));
                form.spawn(text(
                    fonts,
                    UiTextRole::Supporting,
                    "Paste the complete high-entropy BGN1 code from the host.",
                ));
                spawn_labeled_field(
                    form,
                    fonts,
                    "BGN1 connection code",
                    Field::DirectCode,
                    &ui.direct_code,
                    4096,
                );
                spawn_action(
                    form,
                    fonts,
                    "Join Direct",
                    DeckbuilderAction::JoinDirect,
                    false,
                );
                spawn_action(form, fonts, "Back", DeckbuilderAction::Back, false);
                spawn_notices(form, fonts, ui, state);
            });
        });
}

fn spawn_browser(
    commands: &mut Commands,
    fonts: &UiFonts,
    ui: &DeckbuilderUi,
    state: &DeckNetworkState,
    registry: &DiscoveryRegistry,
    now: std::time::Duration,
) {
    commands.spawn((screen_root("Session Browser"), DeckbuilderUiRoot)).with_children(|root| {
        root.spawn(text(fonts, UiTextRole::Display, "Nearby Sessions"));
        root.spawn(text(fonts, UiTextRole::Supporting, "LAN uses mDNS. Tailnet discovery is development tooling; Direct remains available."));
        root.spawn(region("Discovered Sessions", UiRegionRole::ScrollList)).with_children(|list| {
            let sessions = registry.sessions(now);
            if sessions.is_empty() {
                list.spawn(text(fonts, UiTextRole::Body, "Searching - no sessions found yet."));
            }
            for session in sessions {
                let badges = session.sources.iter().map(|source| match source { DiscoverySource::Lan => "LAN", DiscoverySource::Tailnet => "TAILNET", DiscoverySource::Service => "SERVICE" }).collect::<Vec<_>>().join(" + ");
                let label = format!("{}  [{badges}]  {}/{}  LOCKED  {}s", session.metadata.display_name(), session.metadata.claimed_players(), session.metadata.player_capacity(), session.freshness.as_secs());
                let disabled = session.compatibility != Compatibility::Compatible;
                spawn_action(list, fonts, &label, DeckbuilderAction::SelectSession(session.session_id), disabled);
            }
        });
        spawn_action(root, fonts, "Join Selected", DeckbuilderAction::JoinSelected, ui.selected_target.is_none());
        spawn_action(root, fonts, "Back", DeckbuilderAction::Back, false);
        for notice in registry.provider_notices().values() {
            root.spawn(text(fonts, UiTextRole::Metadata, notice.clone()));
        }
        spawn_notices(root, fonts, ui, state);
    });
}

fn spawn_password(
    commands: &mut Commands,
    fonts: &UiFonts,
    ui: &DeckbuilderUi,
    state: &DeckNetworkState,
) {
    commands.spawn((screen_root("Discovery Password"), DeckbuilderUiRoot)).with_children(|root| {
        root.spawn(panel("Password Prompt")).with_children(|form| {
            form.spawn(text(fonts, UiTextRole::Title, "Session Admission"));
            form.spawn(text(fonts, UiTextRole::Supporting, "This temporary passphrase is sent only through the certificate-pinned encrypted connection."));
            spawn_labeled_field(form, fonts, "Temporary session passphrase", Field::JoinPassphrase, &ui.passphrase, 64);
            // Validation happens in the action handler. Keeping this actionable avoids
            // rebuilding the whole form (and destroying text focus) on every keystroke.
            spawn_action(form, fonts, "Join Session", DeckbuilderAction::SubmitPassword, false);
            spawn_action(form, fonts, "Back", DeckbuilderAction::Back, false);
            spawn_notices(form, fonts, ui, state);
        });
    });
}

fn spawn_lobby(
    commands: &mut Commands,
    fonts: &UiFonts,
    ui: &DeckbuilderUi,
    state: &DeckNetworkState,
) {
    commands
        .spawn((screen_root("Ready Lobby"), DeckbuilderUiRoot))
        .with_children(|root| {
            root.spawn(text(fonts, UiTextRole::Display, "Ready Lobby"));
            if let Some(snapshot) = state.latest.as_ref() {
                root.spawn(panel("Seat Status")).with_children(|seats| {
                    for seat in &snapshot.seats {
                        seats.spawn(text(
                            fonts,
                            UiTextRole::Body,
                            format!(
                                "{} — {} — {}",
                                seat.seat.label(),
                                if seat.connected {
                                    "Connected"
                                } else {
                                    "Reserved / Offline"
                                },
                                if seat.ready { "Ready" } else { "Not ready" }
                            ),
                        ));
                    }
                });
                let own = snapshot
                    .seats
                    .iter()
                    .find(|seat| seat.seat == snapshot.recipient);
                let ready = own.is_some_and(|seat| seat.ready);
                spawn_action(
                    root,
                    fonts,
                    if ready { "Not Ready" } else { "Ready" },
                    DeckbuilderAction::SetReady(!ready),
                    !state.admitted,
                );
                if state.role == NetworkRole::Host {
                    let can_start = snapshot
                        .seats
                        .iter()
                        .all(|seat| seat.connected && seat.ready);
                    spawn_action(
                        root,
                        fonts,
                        "Start Network Match",
                        DeckbuilderAction::StartNetworkMatch,
                        !can_start,
                    );
                }
            } else {
                root.spawn(text(
                    fonts,
                    UiTextRole::Body,
                    "Connecting and authenticating…",
                ));
            }
            if let Some(code) = ui
                .share_code
                .as_ref()
                .filter(|_| state.role == NetworkRole::Host)
            {
                root.spawn(text(
                    fonts,
                    UiTextRole::Supporting,
                    "Private fallback code (share out of band):",
                ));
                root.spawn(text(fonts, UiTextRole::Metadata, code.clone()));
                spawn_action(
                    root,
                    fonts,
                    "Copy Private BGN1 Code",
                    DeckbuilderAction::CopyShareCode,
                    false,
                );
            }
            spawn_action(
                root,
                fonts,
                "Return to Menu",
                DeckbuilderAction::ReturnToMenu,
                false,
            );
            spawn_notices(root, fonts, ui, state);
        });
}

fn spawn_match(
    commands: &mut Commands,
    fonts: &UiFonts,
    ui: &DeckbuilderUi,
    state: &DeckNetworkState,
    viewport: UiViewportClass,
) {
    let Some(snapshot) = state.latest.as_ref() else {
        return spawn_menu(commands, fonts, ui, state);
    };
    let own = snapshot
        .seats
        .iter()
        .find(|seat| seat.seat == snapshot.recipient);
    let own_energy = own.map_or(0, |seat| seat.energy);
    let own_turn = snapshot.current_turn == snapshot.recipient;
    commands
        .spawn((screen_root("Deckbuilder Match"), DeckbuilderUiRoot))
        .with_children(|root| {
            root.spawn(region("Match HUD", UiRegionRole::Hud))
                .insert(Node {
                    width: Val::Percent(94.0),
                    flex_direction: FlexDirection::Row,
                    justify_content: JustifyContent::SpaceBetween,
                    ..default()
                })
                .with_children(|hud| {
                    hud.spawn(text(
                        fonts,
                        UiTextRole::Body,
                        format!("You: {}", snapshot.recipient.label()),
                    ));
                    hud.spawn(text(
                        fonts,
                        UiTextRole::Body,
                        format!("Turn: {}", snapshot.current_turn.label()),
                    ));
                    hud.spawn(text(
                        fonts,
                        UiTextRole::Body,
                        format!("Round: {}", snapshot.round),
                    ));
                    hud.spawn(text(
                        fonts,
                        UiTextRole::Body,
                        format!("Energy: {own_energy}"),
                    ));
                });
            root.spawn((
                Name::new("Match Content"),
                Node {
                    width: Val::Percent(94.0),
                    height: Val::Percent(72.0),
                    flex_direction: if viewport == UiViewportClass::Compact {
                        FlexDirection::Column
                    } else {
                        FlexDirection::Row
                    },
                    column_gap: Val::Px(18.0),
                    row_gap: Val::Px(12.0),
                    ..default()
                },
            ))
            .with_children(|content| {
                content
                    .spawn(panel("Hand Panel"))
                    .insert(Node {
                        flex_grow: 1.0,
                        min_width: Val::Px(0.0),
                        flex_direction: FlexDirection::Column,
                        row_gap: Val::Px(12.0),
                        padding: UiRect::all(Val::Px(18.0)),
                        border: UiRect::all(Val::Px(1.0)),
                        border_radius: BorderRadius::all(Val::Px(10.0)),
                        ..default()
                    })
                    .with_children(|hand| {
                        hand.spawn(text(fonts, UiTextRole::Title, "Your Private Hand"));
                        hand.spawn(region("Card Scroll List", UiRegionRole::ScrollList))
                            .insert(Node {
                                width: Val::Percent(100.0),
                                flex_direction: FlexDirection::Row,
                                flex_wrap: FlexWrap::Wrap,
                                column_gap: Val::Px(12.0),
                                row_gap: Val::Px(12.0),
                                overflow: Overflow::scroll_y(),
                                ..default()
                            })
                            .with_children(|cards| {
                                for private in &snapshot.own_hand {
                                    let disabled = private.played
                                        || private.kind.cost() > own_energy
                                        || !own_turn;
                                    spawn_card(
                                        cards,
                                        fonts,
                                        private.kind,
                                        ui.selected_card == Some(private.kind),
                                        disabled,
                                    );
                                }
                            });
                    });
                content
                    .spawn(panel("Activity Panel"))
                    .insert(Node {
                        width: if viewport == UiViewportClass::Compact {
                            Val::Percent(100.0)
                        } else {
                            Val::Px(340.0)
                        },
                        max_height: Val::Percent(100.0),
                        flex_direction: FlexDirection::Column,
                        row_gap: Val::Px(12.0),
                        padding: UiRect::all(Val::Px(18.0)),
                        border: UiRect::all(Val::Px(1.0)),
                        border_radius: BorderRadius::all(Val::Px(10.0)),
                        ..default()
                    })
                    .with_children(|activity_panel| {
                        activity_panel.spawn(text(fonts, UiTextRole::Title, "Activity"));
                        activity_panel
                            .spawn(region("Activity Feed", UiRegionRole::ActivityFeed))
                            .with_children(|activity| {
                                for line in &snapshot.activity {
                                    activity.spawn(text(
                                        fonts,
                                        UiTextRole::Supporting,
                                        line.clone(),
                                    ));
                                }
                            });
                    });
            });
            root.spawn(region("Action Rail", UiRegionRole::ActionRail))
                .insert(Node {
                    width: Val::Percent(94.0),
                    flex_direction: FlexDirection::Row,
                    justify_content: JustifyContent::Center,
                    column_gap: Val::Px(12.0),
                    ..default()
                })
                .with_children(|actions| {
                    let can_play = ui.selected_card.is_some_and(|selected| {
                        snapshot.own_hand.iter().any(|card| {
                            card.kind == selected && !card.played && selected.cost() <= own_energy
                        })
                    }) && own_turn;
                    spawn_action(
                        actions,
                        fonts,
                        "Play Selected",
                        DeckbuilderAction::PlaySelected,
                        !can_play,
                    );
                    spawn_action(
                        actions,
                        fonts,
                        "End Turn",
                        DeckbuilderAction::EndTurn,
                        !own_turn,
                    );
                    spawn_action(actions, fonts, "Pause", DeckbuilderAction::Pause, false);
                });
            spawn_notices(root, fonts, ui, state);
            if ui.paused {
                spawn_pause_modal(root, fonts);
            }
        });
}

fn spawn_card(
    parent: &mut ChildSpawnerCommands,
    fonts: &UiFonts,
    kind: CardKind,
    selected: bool,
    disabled: bool,
) {
    let mut entity = parent.spawn((
        card(format!("Card {}", kind.title())),
        DeckbuilderAction::SelectCard(kind),
        Button,
        bevy_game_ui::UiAction,
        bevy::input_focus::tab_navigation::TabIndex(0),
    ));
    if disabled {
        entity.insert(UiDisabled);
    }
    entity.with_children(|surface| {
        surface.spawn(text(fonts, UiTextRole::Title, kind.title()));
        surface.spawn(text(
            fonts,
            UiTextRole::Body,
            format!("Cost {}", kind.cost()),
        ));
        surface.spawn(text(fonts, UiTextRole::Supporting, kind.rules()));
        if selected {
            surface.spawn(text(fonts, UiTextRole::Body, "SELECTED"));
        }
    });
}

fn spawn_action(
    parent: &mut ChildSpawnerCommands,
    fonts: &UiFonts,
    name: &str,
    action: DeckbuilderAction,
    disabled: bool,
) {
    let mut entity = parent.spawn((button(name), action));
    if disabled {
        entity.insert(UiDisabled);
    }
    entity.with_child(text(fonts, UiTextRole::Body, name.to_owned()));
}

fn spawn_labeled_field(
    parent: &mut ChildSpawnerCommands,
    fonts: &UiFonts,
    label: &'static str,
    field: Field,
    value: &str,
    max: usize,
) {
    parent.spawn(text(fonts, UiTextRole::Supporting, label));
    parent.spawn((text_field(fonts, label, value, max), field));
}

fn spawn_notices(
    parent: &mut ChildSpawnerCommands,
    fonts: &UiFonts,
    ui: &DeckbuilderUi,
    state: &DeckNetworkState,
) {
    if let Some(notice) = ui.local_notice.as_ref().or(state.notice.as_ref()) {
        parent.spawn((
            Name::new("Session Notice"),
            text(fonts, UiTextRole::Supporting, notice.clone()),
        ));
    }
}

fn spawn_pause_modal(parent: &mut ChildSpawnerCommands, fonts: &UiFonts) {
    parent.spawn(modal("Pause Modal")).with_children(|overlay| {
        overlay.spawn(panel("Pause Panel")).with_children(|pause| {
            pause.spawn(text(fonts, UiTextRole::Title, "Paused"));
            spawn_action(pause, fonts, "Resume", DeckbuilderAction::Resume, false);
            spawn_action(
                pause,
                fonts,
                "Return to Menu",
                DeckbuilderAction::ReturnToMenu,
                false,
            );
        });
    });
}

#[cfg(test)]
mod tests {
    use super::*;
    use bevy::input_focus::InputFocus;
    use bevy_game_test::{
        click_action, find_named, focus_action, run_frames, tap_key, ui_tree_snapshot,
        TestAppBuilder,
    };
    use bevy_game_ui::{UiScaleMode, UiScalePreference};

    fn test_app(width: u32, height: u32, scale: UiScaleMode) -> App {
        let mut builder = TestAppBuilder::new().with_ui(width, height);
        builder
            .app_mut()
            .insert_resource(UiScalePreference(scale))
            .add_plugins(DeckbuilderPlugin);
        builder.build()
    }

    fn start_solo(app: &mut App) {
        run_frames(app, 3);
        let start = find_named(app.world_mut(), "Start Solo").expect("menu contains Start Solo");
        assert!(click_action(app, start));
        run_frames(app, 3);
    }

    #[test]
    fn pointer_flow_uses_authoritative_game_owned_reducer() {
        let mut app = test_app(1920, 1080, UiScaleMode::Auto);
        start_solo(&mut app);
        let spark = find_named(app.world_mut(), "Card Spark").expect("hand contains Spark");
        assert!(click_action(&mut app, spark));
        run_frames(&mut app, 2);
        let play = find_named(app.world_mut(), "Play Selected").expect("action rail contains Play");
        assert!(click_action(&mut app, play));
        run_frames(&mut app, 2);
        let snapshot = network::latest_snapshot(app.world()).expect("snapshot exists");
        assert_eq!(
            snapshot
                .seats
                .iter()
                .find(|seat| seat.seat == crate::domain::Seat::Host)
                .expect("host")
                .energy,
            2
        );
    }

    #[test]
    fn keyboard_activation_matches_pointer_activation() {
        let mut app = test_app(1920, 1080, UiScaleMode::Auto);
        run_frames(&mut app, 3);
        let multiplayer = find_named(app.world_mut(), "Multiplayer").expect("menu control");
        assert!(focus_action(app.world_mut(), multiplayer));
        tap_key(&mut app, KeyCode::Enter);
        run_frames(&mut app, 3);
        assert_eq!(
            app.world().resource::<DeckbuilderUi>().screen,
            Screen::Multiplayer
        );
    }

    #[test]
    fn pause_modal_traps_and_restores_focus() {
        let mut app = test_app(1920, 1080, UiScaleMode::Auto);
        start_solo(&mut app);
        let pause = find_named(app.world_mut(), "Pause").expect("Pause exists");
        assert!(focus_action(app.world_mut(), pause));
        assert!(click_action(&mut app, pause));
        run_frames(&mut app, 3);
        let resume = find_named(app.world_mut(), "Resume").expect("Resume exists");
        assert_eq!(app.world().resource::<InputFocus>().get(), Some(resume));
        assert!(click_action(&mut app, resume));
        run_frames(&mut app, 3);
        let restored = find_named(app.world_mut(), "Pause").expect("Pause restored");
        assert_eq!(app.world().resource::<InputFocus>().get(), Some(restored));
    }

    #[test]
    fn responsive_matrix_preserves_regions_and_target_sizes() {
        for (width, height, scale) in [
            (1280, 720, UiScaleMode::Auto),
            (1920, 1080, UiScaleMode::Auto),
            (3840, 2160, UiScaleMode::Auto),
            (1280, 720, UiScaleMode::Percent200),
            (1920, 1080, UiScaleMode::Percent200),
            (3840, 2160, UiScaleMode::Percent200),
        ] {
            let mut app = test_app(width, height, scale);
            start_solo(&mut app);
            let snapshot = ui_tree_snapshot(app.world_mut());
            let rendered = snapshot.to_string();
            assert!(rendered.contains("Match HUD [hud]"));
            assert!(rendered.contains("Action Rail [action-rail]"));
            assert!(rendered.contains("Activity Feed [activity-feed]"));
            for node in snapshot
                .nodes
                .iter()
                .filter(|node| node.action && !node.disabled)
            {
                assert!(node.size.x >= 44.0, "{} is too narrow", node.path);
                assert!(node.size.y >= 44.0, "{} is too short", node.path);
            }
        }
    }

    #[test]
    fn service_style_fake_provider_uses_the_same_browser_and_join_selection() {
        use bevy_game_discovery::{ExpectedSession, FakeDiscoveryProvider, SessionMetadata};
        use bevy_game_multiplayer::{
            CertificateFingerprint, DirectEndpoint, DiscoveredDirectTarget,
        };

        let mut app = test_app(1920, 1080, UiScaleMode::Auto);
        run_frames(&mut app, 2);
        let session_id = SessionId::from_bytes([9; 16]);
        let mut provider = FakeDiscoveryProvider::default();
        provider.publish(
            SessionMetadata::new(
                network::GAME_ID,
                network::PROTOCOL_VERSION,
                network::BUILD_ID,
                "Steam-like Table",
                1,
                2,
                true,
            )
            .expect("valid fake metadata"),
            DiscoveredDirectTarget {
                session_id,
                endpoint: DirectEndpoint::new("service.invalid", 7777)
                    .expect("valid fixture endpoint"),
                certificate_fingerprint: CertificateFingerprint::from_bytes([7; 32]),
                certificate_expires_unix_seconds: 2_000_000_000,
            },
            std::time::Duration::from_secs(60),
        );
        let expected = app.world().resource::<ExpectedSession>().clone();
        for observation in provider.drain() {
            app.world_mut()
                .resource_mut::<DiscoveryRegistry>()
                .apply(observation, Some(&expected));
        }
        app.world_mut().resource_mut::<DeckbuilderUi>().screen = Screen::Browser;
        app.world_mut().resource_mut::<UiDirty>().0 = true;
        run_frames(&mut app, 2);

        let listing = {
            let world = app.world_mut();
            let mut names = world.query::<(Entity, &Name)>();
            names
                .iter(world)
                .find_map(|(entity, name)| {
                    name.as_str()
                        .starts_with("Steam-like Table")
                        .then_some(entity)
                })
                .expect("service listing uses shared browser action")
        };
        assert!(click_action(&mut app, listing));
        run_frames(&mut app, 2);
        let selected = app
            .world()
            .resource::<DeckbuilderUi>()
            .selected_target
            .as_ref()
            .expect("selection creates an opaque join handoff");
        assert_eq!(selected.session_id(), session_id);
        assert_eq!(selected.preferred_source(), Some(DiscoverySource::Service));
    }

    #[test]
    fn host_form_uses_native_fields_and_secret_buffers_clear_after_attempts() {
        let mut app = test_app(1280, 720, UiScaleMode::Auto);
        run_frames(&mut app, 3);
        let multiplayer = find_named(app.world_mut(), "Multiplayer").expect("menu control");
        assert!(click_action(&mut app, multiplayer));
        run_frames(&mut app, 2);
        let host = find_named(app.world_mut(), "Host Session").expect("host control");
        assert!(click_action(&mut app, host));
        run_frames(&mut app, 2);

        let passphrase = find_named(app.world_mut(), "Temporary session passphrase")
            .expect("host passphrase field");
        assert!(app
            .world()
            .get::<bevy::text::EditableText>(passphrase)
            .is_some());
        assert!(app
            .world()
            .get::<bevy_game_ui::UiTextField>(passphrase)
            .is_some());
        let node = app.world().get::<Node>(passphrase).expect("field node");
        assert!(matches!(node.min_height, Val::Px(height) if height >= 44.0));

        {
            let mut ui = app.world_mut().resource_mut::<DeckbuilderUi>();
            ui.passphrase = "short".to_owned();
            ui.screen = Screen::Password;
        }
        apply_action(app.world_mut(), DeckbuilderAction::SubmitPassword);
        assert!(app
            .world()
            .resource::<DeckbuilderUi>()
            .passphrase
            .is_empty());

        app.world_mut().resource_mut::<DeckbuilderUi>().direct_code = "invalid".to_owned();
        apply_action(app.world_mut(), DeckbuilderAction::JoinDirect);
        assert!(app
            .world()
            .resource::<DeckbuilderUi>()
            .direct_code
            .is_empty());
    }

    #[test]
    fn direct_join_button_activates_after_real_field_edit() {
        let mut app = test_app(1280, 720, UiScaleMode::Auto);
        run_frames(&mut app, 3);
        let multiplayer = find_named(app.world_mut(), "Multiplayer").expect("menu control");
        assert!(click_action(&mut app, multiplayer));
        run_frames(&mut app, 2);
        let direct =
            find_named(app.world_mut(), "Join with BGN1 Code").expect("direct join control");
        assert!(click_action(&mut app, direct));
        run_frames(&mut app, 2);

        let field = find_named(app.world_mut(), "BGN1 connection code").expect("direct field");
        app.world_mut()
            .get_mut::<bevy::text::EditableText>(field)
            .expect("editable direct field")
            .editor_mut()
            .set_text("invalid code");
        run_frames(&mut app, 2);
        assert_eq!(
            app.world().resource::<DeckbuilderUi>().direct_code,
            "invalid code"
        );

        let join = find_named(app.world_mut(), "Join Direct").expect("join button");
        assert!(click_action(&mut app, join));
        run_frames(&mut app, 2);
        assert!(app
            .world()
            .resource::<DeckbuilderUi>()
            .local_notice
            .as_deref()
            .is_some_and(|notice| notice.contains("unsupported connection-code version")));
        assert!(app
            .world()
            .resource::<DeckbuilderUi>()
            .direct_code
            .is_empty());
    }

    #[test]
    fn hosted_code_has_an_explicit_copy_action() {
        let mut app = test_app(1280, 720, UiScaleMode::Auto);
        run_frames(&mut app, 3);
        {
            let mut ui = app.world_mut().resource_mut::<DeckbuilderUi>();
            ui.screen = Screen::Lobby;
            ui.share_code = Some("BGN1.private-fixture".to_owned());
        }
        app.world_mut()
            .resource_mut::<network::DeckNetworkState>()
            .role = network::NetworkRole::Host;
        app.world_mut().resource_mut::<UiDirty>().0 = true;
        run_frames(&mut app, 2);

        let copy = find_named(app.world_mut(), "Copy Private BGN1 Code")
            .expect("host can copy the code without retyping it");
        assert!(click_action(&mut app, copy));
        run_frames(&mut app, 2);
        let notice = app
            .world()
            .resource::<DeckbuilderUi>()
            .local_notice
            .as_deref()
            .expect("copy reports success or a typed platform failure");
        assert!(notice.contains("clipboard"));
        assert!(!notice.contains("private-fixture"));
    }

    #[test]
    fn direct_join_button_hands_a_valid_code_to_real_udp_transport() {
        let advertised_host = network::default_advertised_host()
            .unwrap_or_else(|| std::net::Ipv4Addr::LOCALHOST.to_string());
        let advertised_address = advertised_host
            .parse::<std::net::IpAddr>()
            .expect("detected host is an IP address");
        let probe = std::net::UdpSocket::bind((advertised_address, 0))
            .expect("reserve an available UDP port");
        let port = probe.local_addr().expect("probe address").port();
        drop(probe);

        let mut host = test_app(1280, 720, UiScaleMode::Auto);
        let mut guest = test_app(1280, 720, UiScaleMode::Auto);
        run_frames(&mut host, 3);
        run_frames(&mut guest, 3);
        network::start_host(
            host.world_mut(),
            network::HostConfiguration {
                session_name: "UI Socket Table".to_owned(),
                password: "temporary-passphrase".to_owned(),
                advertised_host,
                port,
                discover_lan: false,
                discover_tailnet: false,
            },
        )
        .expect("host opens real UDP transport");
        let code = network::hosted_code(host.world()).expect("host exposes BGN1 code");

        let multiplayer = find_named(guest.world_mut(), "Multiplayer").expect("menu control");
        assert!(click_action(&mut guest, multiplayer));
        run_frames(&mut guest, 2);
        let direct =
            find_named(guest.world_mut(), "Join with BGN1 Code").expect("direct join control");
        assert!(click_action(&mut guest, direct));
        run_frames(&mut guest, 2);
        let field = find_named(guest.world_mut(), "BGN1 connection code").expect("direct field");
        guest
            .world_mut()
            .get_mut::<bevy::text::EditableText>(field)
            .expect("editable direct field")
            .editor_mut()
            .set_text(&code);
        run_frames(&mut guest, 2);
        let join = find_named(guest.world_mut(), "Join Direct").expect("join button");
        assert!(click_action(&mut guest, join));

        let deadline = std::time::Instant::now() + std::time::Duration::from_secs(10);
        while std::time::Instant::now() < deadline
            && !guest
                .world()
                .resource::<network::DeckNetworkState>()
                .admitted
        {
            host.update();
            guest.update();
            std::thread::sleep(std::time::Duration::from_millis(5));
        }
        assert!(
            guest
                .world()
                .resource::<network::DeckNetworkState>()
                .admitted,
            "the real direct transport never admitted the UI guest"
        );
        assert_eq!(
            guest.world().resource::<DeckbuilderUi>().screen,
            Screen::Lobby
        );
    }

    #[test]
    fn discovery_password_button_activates_after_real_field_edit() {
        let mut app = test_app(1280, 720, UiScaleMode::Auto);
        run_frames(&mut app, 3);
        {
            let mut ui = app.world_mut().resource_mut::<DeckbuilderUi>();
            ui.screen = Screen::Password;
            ui.previous = Screen::Browser;
        }
        app.world_mut().resource_mut::<UiDirty>().0 = true;
        run_frames(&mut app, 2);

        let field =
            find_named(app.world_mut(), "Temporary session passphrase").expect("password field");
        app.world_mut()
            .get_mut::<bevy::text::EditableText>(field)
            .expect("editable password field")
            .editor_mut()
            .set_text("temporary-passphrase");
        run_frames(&mut app, 2);
        assert_eq!(
            app.world().resource::<DeckbuilderUi>().passphrase,
            "temporary-passphrase"
        );

        let join = find_named(app.world_mut(), "Join Session").expect("join button");
        assert!(app.world().get::<UiDisabled>(join).is_none());
        assert!(click_action(&mut app, join));
        run_frames(&mut app, 2);
        assert_eq!(
            app.world()
                .resource::<DeckbuilderUi>()
                .local_notice
                .as_deref(),
            Some("The selected session is no longer available.")
        );
        assert!(app
            .world()
            .resource::<DeckbuilderUi>()
            .passphrase
            .is_empty());
    }

    #[test]
    fn discovered_route_password_button_reaches_real_udp_transport() {
        use bevy_game_discovery::{ExpectedSession, FakeDiscoveryProvider, SessionMetadata};
        use bevy_game_multiplayer::{DirectConnectionCode, DiscoveredDirectTarget};

        let advertised_host = network::default_advertised_host()
            .unwrap_or_else(|| std::net::Ipv4Addr::LOCALHOST.to_string());
        let advertised_address = advertised_host
            .parse::<std::net::IpAddr>()
            .expect("detected host is an IP address");
        let probe = std::net::UdpSocket::bind((advertised_address, 0))
            .expect("reserve an available UDP port");
        let port = probe.local_addr().expect("probe address").port();
        drop(probe);

        let mut host = test_app(1280, 720, UiScaleMode::Auto);
        let mut guest = test_app(1280, 720, UiScaleMode::Auto);
        run_frames(&mut host, 3);
        run_frames(&mut guest, 3);
        network::start_host(
            host.world_mut(),
            network::HostConfiguration {
                session_name: "Discovered Socket Table".to_owned(),
                password: "temporary-passphrase".to_owned(),
                advertised_host,
                port,
                discover_lan: false,
                discover_tailnet: false,
            },
        )
        .expect("host opens real UDP transport");
        let code = network::hosted_code(host.world()).expect("host exposes BGN1 code");
        let code = DirectConnectionCode::parse(&code).expect("host code parses");

        let mut provider = FakeDiscoveryProvider::default();
        provider.publish(
            SessionMetadata::new(
                network::GAME_ID,
                network::PROTOCOL_VERSION,
                network::BUILD_ID,
                "Discovered Socket Table",
                1,
                2,
                true,
            )
            .expect("discovery metadata"),
            DiscoveredDirectTarget {
                session_id: code.session_id,
                endpoint: code.endpoint,
                certificate_fingerprint: code.certificate_fingerprint,
                certificate_expires_unix_seconds: code.certificate_expires_unix_seconds,
            },
            std::time::Duration::from_secs(60),
        );
        let expected = guest.world().resource::<ExpectedSession>().clone();
        for observation in provider.drain() {
            guest
                .world_mut()
                .resource_mut::<DiscoveryRegistry>()
                .apply(observation, Some(&expected));
        }
        guest.world_mut().resource_mut::<DeckbuilderUi>().screen = Screen::Browser;
        guest.world_mut().resource_mut::<UiDirty>().0 = true;
        run_frames(&mut guest, 2);

        let listing = {
            let world = guest.world_mut();
            let mut names = world.query::<(Entity, &Name)>();
            names
                .iter(world)
                .find_map(|(entity, name)| {
                    name.as_str()
                        .starts_with("Discovered Socket Table")
                        .then_some(entity)
                })
                .expect("discovered listing")
        };
        assert!(click_action(&mut guest, listing));
        run_frames(&mut guest, 2);
        let select = find_named(guest.world_mut(), "Join Selected").expect("selected action");
        assert!(click_action(&mut guest, select));
        run_frames(&mut guest, 2);
        let field =
            find_named(guest.world_mut(), "Temporary session passphrase").expect("password field");
        guest
            .world_mut()
            .get_mut::<bevy::text::EditableText>(field)
            .expect("editable password field")
            .editor_mut()
            .set_text("temporary-passphrase");
        run_frames(&mut guest, 2);
        let join = find_named(guest.world_mut(), "Join Session").expect("join action");
        assert!(click_action(&mut guest, join));

        let deadline = std::time::Instant::now() + std::time::Duration::from_secs(10);
        while std::time::Instant::now() < deadline
            && !guest
                .world()
                .resource::<network::DeckNetworkState>()
                .admitted
        {
            host.update();
            guest.update();
            std::thread::sleep(std::time::Duration::from_millis(5));
        }
        assert!(
            guest
                .world()
                .resource::<network::DeckNetworkState>()
                .admitted,
            "the discovered password route never admitted the UI guest"
        );
        assert!(guest
            .world()
            .resource::<DeckbuilderUi>()
            .passphrase
            .is_empty());
    }
}
