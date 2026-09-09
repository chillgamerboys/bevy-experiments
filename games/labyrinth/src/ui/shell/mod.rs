//! Menus and admission forms; provider endpoints never enter these widgets.

mod forms;
mod lobby;
mod settings;

use forms::{browser, direct_form, host_form, password_form};
use lobby::lobby;
pub(super) use settings::overlays;

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
        .spawn((screen_root("Labyrinth Screen"), UiSkin::Screen, ShellRoot))
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
    let entity = world
        .spawn((panel(name), UiSkin::Panel, ChildOf(parent)))
        .id();
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
