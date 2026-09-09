//! Menus and admission forms; provider endpoints never enter these widgets.

use super::*;
use bevy_game_ui::{panel, screen_root, text_field, UiFocusId, UiViewportClass};

pub(super) fn present(
    world: &mut World,
    view: &LabyrinthView,
    ui: &mut UiState,
    metrics: ResolvedUiMetrics,
) {
    let key = format!(
        "{:?}:{:?}:{:?}:{}:{}:{:?}:{:?}:{:?}:{:?}:{:?}:{:?}:{}:{}:{:?}",
        view.mode,
        ui.form,
        metrics.viewport,
        ui.lan,
        ui.tailnet,
        view.players,
        view.invite_labels,
        view.listings,
        view.notice,
        ui.local_notice,
        view.provider_notices,
        view.admitted,
        view.session_name,
        view.player
    );
    if ui.shell_key.as_ref() == Some(&key) {
        return;
    }
    despawn_marked::<ShellRoot>(world);
    let root = world
        .spawn((screen_root("Labyrinth Screen"), ShellRoot))
        .id();
    world.entity_mut(root).insert(Node {
        width: Val::Percent(100.0),
        height: Val::Percent(100.0),
        flex_direction: FlexDirection::Column,
        align_items: AlignItems::Center,
        overflow: Overflow::scroll_y(),
        padding: UiRect::all(Val::Px(28.0)),
        row_gap: Val::Px(20.0),
        ..default()
    });
    let content = column(
        world,
        root,
        "Menu Content",
        Node {
            width: Val::Percent(100.0),
            max_width: Val::Px(if metrics.viewport == UiViewportClass::Wide {
                1600.0
            } else {
                1120.0
            }),
            flex_direction: FlexDirection::Column,
            row_gap: Val::Px(18.0),
            flex_shrink: 0.0,
            ..default()
        },
    );
    label(
        world,
        content,
        "Brand",
        "L A B Y R I N T H",
        UiTextRole::Display,
    );
    label(
        world,
        content,
        "Subtitle",
        "Four lanterns. One company. Hold the line together.",
        UiTextRole::Supporting,
    );
    if view.mode == ViewMode::Lobby {
        lobby(world, content, view);
    } else {
        match ui.form {
            Form::Menu => menu(world, content),
            Form::Host => host_form(world, content, ui),
            Form::Direct => direct_form(world, content, ui),
            Form::Browser => browser(world, content, view, ui),
            Form::Password => password_form(world, content, ui),
        }
    }
    if let Some(notice) = ui.local_notice.as_ref().or(view.notice.as_ref()) {
        label(
            world,
            content,
            "Session Notice",
            notice.clone(),
            UiTextRole::Body,
        );
    }
    for (index, notice) in view.provider_notices.iter().enumerate() {
        label(
            world,
            content,
            &format!("Provider Notice {index}"),
            notice.clone(),
            UiTextRole::Supporting,
        );
    }
    ui.shell_key = Some(key);
}

fn surface(world: &mut World, parent: Entity, name: &str) -> Entity {
    let entity = world.spawn((panel(name), ChildOf(parent))).id();
    world.entity_mut(entity).insert(Node {
        width: Val::Percent(100.0),
        flex_direction: FlexDirection::Column,
        row_gap: Val::Px(16.0),
        padding: UiRect::all(Val::Px(24.0)),
        border: UiRect::all(Val::Px(1.0)),
        flex_shrink: 0.0,
        ..default()
    });
    entity
}

fn row(world: &mut World, parent: Entity, name: &str) -> Entity {
    column(
        world,
        parent,
        name,
        Node {
            width: Val::Percent(100.0),
            flex_direction: FlexDirection::Row,
            flex_wrap: FlexWrap::Wrap,
            column_gap: Val::Px(12.0),
            row_gap: Val::Px(12.0),
            ..default()
        },
    )
}

fn menu(world: &mut World, parent: Entity) {
    let main = surface(world, parent, "Expedition Menu");
    label(
        world,
        main,
        "Menu Title",
        "THE LANTERN COMPANY",
        UiTextRole::Title,
    );
    label(world, main, "Menu Description", "An original cooperative positional battle. Each player commands one hero; the host directs the enemy. No campaign, no permanent loss: just one encounter worth solving together.", UiTextRole::Body);
    let actions = row(world, main, "Main Actions");
    control(
        world,
        actions,
        "Host Company",
        "Host a company",
        Action::Form(Form::Host),
        false,
    );
    control(
        world,
        actions,
        "Find Companies",
        "Find a company",
        Action::Form(Form::Browser),
        false,
    );
    control(
        world,
        actions,
        "Join Direct",
        "Join with BGN1 code",
        Action::Form(Form::Direct),
        false,
    );
    control(
        world,
        actions,
        "Reconnect",
        "Reclaim reserved hero",
        Action::Reconnect,
        false,
    );
    let local = surface(world, parent, "Local Workshop");
    label(
        world,
        local,
        "Workshop Title",
        "LOCAL WORKSHOP",
        UiTextRole::Title,
    );
    label(world, local, "Workshop Description", "Control all four heroes with no sockets. Useful for learning ranks and testing rules; this is not a multiplayer test.", UiTextRole::Supporting);
    let actions = row(world, local, "Workshop Actions");
    control(
        world,
        actions,
        "Start Local",
        "Play locally | all four heroes",
        Action::StartLocal,
        false,
    );
    control(
        world,
        actions,
        "Menu Settings",
        "Readability & motion",
        Action::Settings,
        false,
    );
}

fn field(world: &mut World, parent: Entity, name: &str, field: Field, value: &str, max: usize) {
    label(
        world,
        parent,
        &format!("{name} Label"),
        name,
        UiTextRole::Supporting,
    );
    let bundle = text_field(world.resource::<UiFonts>(), name, value, max);
    world.spawn((
        bundle,
        field,
        UiFocusId::new("labyrinth-form", format!("{field:?}")),
        ChildOf(parent),
    ));
}

fn host_form(world: &mut World, parent: Entity, ui: &UiState) {
    let form = surface(world, parent, "Host Form");
    label(
        world,
        form,
        "Host Title",
        "ASSEMBLE YOUR COMPANY",
        UiTextRole::Title,
    );
    field(
        world,
        form,
        "Session name",
        Field::Name,
        &ui.session_name,
        64,
    );
    field(
        world,
        form,
        "Advertised address (empty: choose automatically)",
        Field::Address,
        &ui.address,
        253,
    );
    field(
        world,
        form,
        "Game UDP port (empty: 7777)",
        Field::Port,
        &ui.port,
        5,
    );
    field(
        world,
        form,
        "Temporary passphrase (visible while typing)",
        Field::Password,
        &ui.password.0,
        64,
    );
    label(world, form, "Password Warning", "Discovery requires 8-64 printable characters. Use a temporary session passphrase, never an account password. Direct invitations do not require it.", UiTextRole::Supporting);
    let discovery = row(world, form, "Discovery Settings");
    control(
        world,
        discovery,
        "LAN Discovery",
        if ui.lan {
            "LAN discovery: ON"
        } else {
            "LAN discovery: OFF"
        },
        Action::ToggleLan,
        false,
    );
    control(
        world,
        discovery,
        "Tailnet Discovery",
        if ui.tailnet {
            "Tailnet (Dev): ON"
        } else {
            "Tailnet (Dev): OFF"
        },
        Action::ToggleTailnet,
        false,
    );
    label(world, form, "Host Advice", "Three independent private invitation codes will be available in the lobby. Tailnet discovery requires external Tailscale installation and login.", UiTextRole::Supporting);
    let actions = row(world, form, "Host Actions");
    control(
        world,
        actions,
        "Create Session",
        "Light the lanterns | Host",
        Action::Host,
        false,
    );
    control(
        world,
        actions,
        "Host Back",
        "Back",
        Action::Form(Form::Menu),
        false,
    );
}

fn direct_form(world: &mut World, parent: Entity, ui: &UiState) {
    let form = surface(world, parent, "Direct Form");
    label(
        world,
        form,
        "Direct Title",
        "PRIVATE INVITATION",
        UiTextRole::Title,
    );
    label(world, form, "Code Advice", "Ask the host for your own BGN1 code. Each invitation admits one player; do not share another guest's code. The code is cleared after the attempt.", UiTextRole::Supporting);
    field(
        world,
        form,
        "Private BGN1 connection code",
        Field::Code,
        &ui.code.0,
        2048,
    );
    let actions = row(world, form, "Direct Actions");
    control(
        world,
        actions,
        "Submit Direct",
        "Join direct",
        Action::JoinCode,
        false,
    );
    control(
        world,
        actions,
        "Direct Back",
        "Back",
        Action::Form(Form::Menu),
        false,
    );
}

fn browser(world: &mut World, parent: Entity, view: &LabyrinthView, ui: &UiState) {
    let form = surface(world, parent, "Session Browser");
    label(
        world,
        form,
        "Browser Title",
        "COMPANIES NEARBY",
        UiTextRole::Title,
    );
    label(world, form, "Browser Advice", "Listings are availability hints, not admission. LAN is local multicast; Tailnet uses explicitly enabled development discovery.", UiTextRole::Supporting);
    let actions = row(world, form, "Browser Actions");
    control(
        world,
        actions,
        "Browser Refresh",
        "Refresh",
        Action::Browse,
        false,
    );
    control(
        world,
        actions,
        "Browser Tailnet",
        if ui.tailnet {
            "Tailnet browser: ON"
        } else {
            "Tailnet browser: OFF"
        },
        Action::ToggleTailnet,
        false,
    );
    control(
        world,
        actions,
        "Browser Direct",
        "Use private code",
        Action::Form(Form::Direct),
        false,
    );
    control(
        world,
        actions,
        "Browser Back",
        "Back",
        Action::Form(Form::Menu),
        false,
    );
    if view.listings.is_empty() {
        label(world, form, "Empty Browser", "No companies are visible yet. Keep this browser open, verify host discovery is enabled, or use a private code.", UiTextRole::Body);
    }
    for (index, listing) in view.listings.iter().enumerate() {
        label(
            world,
            form,
            &format!("Listing {index} Description"),
            listing.label.clone(),
            UiTextRole::Body,
        );
        control(
            world,
            form,
            format!("Join Listing {index}"),
            if listing.compatible {
                "Join locked company"
            } else {
                "Incompatible game or rules"
            },
            Action::Session(listing.id),
            !listing.compatible,
        );
    }
}

fn password_form(world: &mut World, parent: Entity, ui: &UiState) {
    let form = surface(world, parent, "Discovery Password Form");
    label(
        world,
        form,
        "Discovery Password Title",
        "ENTER THE COMPANY PASSPHRASE",
        UiTextRole::Title,
    );
    field(
        world,
        form,
        "Temporary passphrase (visible while typing)",
        Field::Password,
        &ui.password.0,
        64,
    );
    label(world, form, "Discovery Password Advice", "Sent only after the selected host's encrypted certificate-pinned connection opens. Do not reuse an important password.", UiTextRole::Supporting);
    let actions = row(world, form, "Password Actions");
    control(
        world,
        actions,
        "Submit Discovery",
        "Join selected company",
        Action::JoinDiscovered,
        ui.selected_session.is_none(),
    );
    control(
        world,
        actions,
        "Password Back",
        "Back to companies",
        Action::Form(Form::Browser),
        false,
    );
}

fn lobby(world: &mut World, parent: Entity, view: &LabyrinthView) {
    let lobby = surface(world, parent, "Company Lobby");
    label(
        world,
        lobby,
        "Lobby Title",
        if view.session_name.is_empty() {
            "YOUR COMPANY".to_owned()
        } else {
            view.session_name.to_uppercase()
        },
        UiTextRole::Title,
    );
    for player in &view.players {
        let state = if !player.occupied {
            "OPEN"
        } else if !player.connected {
            "RESERVED | disconnected"
        } else if player.ready {
            "READY"
        } else {
            "NOT READY"
        };
        label(
            world,
            lobby,
            &format!("Player {}", player.slot),
            format!("{} | {} | {state}", player.name, player.hero.name()),
            UiTextRole::Body,
        );
    }
    label(
        world,
        lobby,
        "Hero Choice Title",
        "Choose your hero | each role belongs to one player",
        UiTextRole::Supporting,
    );
    let roles = row(world, lobby, "Hero Choices");
    for hero in HeroClass::ALL {
        let taken = view.players.iter().any(|player| {
            player.occupied && Some(player.slot) != view.player && player.hero == hero
        });
        control(
            world,
            roles,
            format!("Choose {hero:?}"),
            hero.name(),
            Action::Hero(hero),
            taken,
        );
    }
    let ready = view
        .players
        .iter()
        .find(|player| Some(player.slot) == view.player)
        .is_some_and(|player| player.ready);
    let actions = row(world, lobby, "Lobby Actions");
    control(
        world,
        actions,
        "Toggle Ready",
        if ready { "Not ready" } else { "Ready" },
        Action::Ready(!ready),
        !view.admitted,
    );
    if view.host {
        let can_start = view.players.len() == 4
            && view
                .players
                .iter()
                .all(|player| player.occupied && player.connected && player.ready);
        control(
            world,
            actions,
            "Start Encounter",
            "Enter the breach",
            Action::Start,
            !can_start,
        );
    }
    control(
        world,
        actions,
        "Lobby Settings",
        "Readability & motion",
        Action::Settings,
        false,
    );
    control(
        world,
        actions,
        "Lobby Leave",
        "Return to menu",
        Action::Leave,
        false,
    );
    if view.host {
        let invites = surface(world, parent, "Private Invitations");
        label(
            world,
            invites,
            "Invite Title",
            "THREE GUESTS | THREE INVITATIONS",
            UiTextRole::Title,
        );
        label(world, invites, "Invite Advice", "Copy one distinct invitation for each friend. Codes stay out of the screen and diagnostics.", UiTextRole::Supporting);
        for (index, invite) in view.invite_labels.iter().enumerate() {
            let row = row(world, invites, &format!("Invitation {index}"));
            label(
                world,
                row,
                &format!("Invitation {index} Label"),
                invite.clone(),
                UiTextRole::Body,
            );
            control(
                world,
                row,
                format!("Copy Invitation {index}"),
                "Copy private code",
                Action::Copy(index),
                false,
            );
            control(
                world,
                row,
                format!("Reissue Invitation {index}"),
                "Replace unused code",
                Action::Reissue(index),
                false,
            );
        }
    }
}

pub(super) fn overlays(world: &mut World, view: &LabyrinthView, ui: &mut UiState) {
    let key = format!(
        "{}:{}:{}:{:?}:{:?}",
        ui.settings,
        view.paused,
        ui.reduced_motion,
        view.players,
        world.resource::<UiScalePreference>().0
    );
    if ui.overlay_key.as_ref() == Some(&key) {
        return;
    }
    despawn_marked::<OverlayRoot>(world);
    ui.overlay_key = Some(key);
    if !ui.settings && !view.paused {
        return;
    }
    let root = world
        .spawn((
            bevy_game_ui::modal("Labyrinth Blocking Overlay"),
            OverlayRoot,
        ))
        .id();
    let panel = surface(world, root, "Overlay Panel");
    world.entity_mut(panel).insert(Node {
        width: Val::Percent(88.0),
        max_width: Val::Px(1000.0),
        max_height: Val::Percent(90.0),
        flex_direction: FlexDirection::Column,
        row_gap: Val::Px(16.0),
        padding: UiRect::all(Val::Px(28.0)),
        overflow: Overflow::scroll_y(),
        ..default()
    });
    if view.paused {
        label(
            world,
            panel,
            "Reconnect Title",
            "THE COMPANY WAITS",
            UiTextRole::Title,
        );
        let missing = view
            .players
            .iter()
            .filter(|player| player.occupied && !player.connected)
            .map(|player| player.name.as_str())
            .collect::<Vec<_>>()
            .join(", ");
        label(world, panel, "Reconnect Detail", format!("Waiting for {missing}. Combat is frozen at the committed decision. Returning players reclaim the same hero and status effects."), UiTextRole::Body);
        if !view.connected_local() {
            control(
                world,
                panel,
                "Overlay Reconnect",
                "Reconnect reserved hero",
                Action::Reconnect,
                false,
            );
        }
        if view.host {
            control(
                world,
                panel,
                "Abort To Lobby",
                "Abort encounter to lobby",
                Action::Rematch,
                false,
            );
        }
        control(
            world,
            panel,
            "Paused Leave",
            "Return to menu",
            Action::Leave,
            false,
        );
    } else {
        label(
            world,
            panel,
            "Settings Title",
            "READABILITY & MOTION",
            UiTextRole::Title,
        );
        label(world, panel, "Settings Advice", "These settings change only this window. The company keeps playing; local settings do not pause a network encounter.", UiTextRole::Supporting);
        control(
            world,
            panel,
            "Toggle Semantic Scale",
            if world.resource::<UiScalePreference>().0 == UiScaleMode::Percent200 {
                "Text scale: 200% | switch to Auto"
            } else {
                "Text scale: Auto | switch to 200%"
            },
            Action::Scale,
            false,
        );
        control(
            world,
            panel,
            "Toggle Reduced Motion",
            if ui.reduced_motion {
                "Reduced motion: ON"
            } else {
                "Reduced motion: OFF"
            },
            Action::ReducedMotion,
            false,
        );
        control(
            world,
            panel,
            "Close Settings",
            "Back to the company",
            Action::Cancel,
            false,
        );
    }
}

trait LocalPresence {
    fn connected_local(&self) -> bool;
}
impl LocalPresence for LabyrinthView {
    fn connected_local(&self) -> bool {
        self.local
            || self.player.is_some_and(|slot| {
                self.players
                    .iter()
                    .any(|player| player.slot == slot && player.connected)
            })
    }
}
