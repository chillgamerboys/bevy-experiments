//! Menus and admission forms; provider endpoints never enter these widgets.

mod forms;
mod lobby;
mod settings;

use forms::{browser, direct_form, host_form, password_form};
use lobby::lobby;
pub(super) use settings::overlays;

use super::*;
use bevy_game_ui::{panel, screen_root, text_field, UiFocusId, UiViewportClass};

#[derive(Component)]
struct ShellNotice;

#[derive(Component)]
struct ShellBackdrop;

fn backdrop(world: &mut World) {
    let Some(root) = world
        .query_filtered::<Entity, With<ShellRoot>>()
        .iter(world)
        .next()
    else {
        return;
    };
    let Some(handle) = world
        .resource::<crate::scene::SceneAppearance>()
        .backdrop_image
        .clone()
    else {
        return;
    };
    if world
        .query_filtered::<Entity, With<ShellBackdrop>>()
        .iter(world)
        .next()
        .is_some()
    {
        return;
    }
    let mut image = ImageNode::new(handle);
    image.color = Color::srgb(0.42, 0.42, 0.42);
    world.spawn((
        Name::new("Menu Environment"),
        ShellBackdrop,
        image,
        Node {
            position_type: PositionType::Absolute,
            width: Val::Percent(100.0),
            height: Val::Percent(100.0),
            left: Val::Px(0.0),
            top: Val::Px(0.0),
            ..default()
        },
        GlobalZIndex(1),
        Pickable::IGNORE,
        bevy::ui::FocusPolicy::Pass,
        ChildOf(root),
    ));
}

fn notices(world: &mut World, view: &LabyrinthView, ui: &UiState) {
    let value = ui
        .local_notice
        .as_ref()
        .or(view.notice.as_ref())
        .into_iter()
        .chain(view.provider_notices.iter())
        .cloned()
        .collect::<Vec<_>>()
        .join("\n");
    let mut query = world.query_filtered::<&mut Text, With<ShellNotice>>();
    for mut text in query.iter_mut(world) {
        if text.0 != value {
            text.0.clone_from(&value);
        }
    }
}

pub(super) fn present(
    world: &mut World,
    view: &LabyrinthView,
    ui: &mut UiState,
    metrics: ResolvedUiMetrics,
) {
    // Notices and background discovery updates must not recreate an active
    // admission field, lose its cursor, or reset keyboard focus.
    notices(world, view, ui);
    backdrop(world);
    let lobby_data = (view.mode == ViewMode::Lobby).then_some((
        &view.players,
        &view.invite_labels,
        &view.session_name,
    ));
    let listings = (ui.form == Form::Browser).then_some(&view.listings);
    let key = format!(
        "{:?}:{:?}:{:?}:{}:{}:{:?}:{:?}:{}:{:?}",
        view.mode,
        ui.form,
        metrics.viewport,
        ui.lan,
        ui.tailnet,
        lobby_data,
        listings,
        view.admitted,
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
            align_items: AlignItems::Center,
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
    world.entity_mut(content).insert(GlobalZIndex(2));
    backdrop(world);
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
        "Six lanterns. One company. Hold the line together.",
        UiTextRole::Supporting,
    );
    if view.mode == ViewMode::Lobby {
        lobby(world, content, view);
    } else {
        match ui.form {
            Form::Menu => menu(world, content),
            Form::Multiplayer => multiplayer_menu(world, content),
            Form::Host => host_form(world, content, ui),
            Form::Direct => direct_form(world, content, ui),
            Form::Browser => browser(world, content, view, ui),
            Form::Password => password_form(world, content, ui),
        }
    }
    let notice = label(world, content, "Session Notice", "", UiTextRole::Body);
    world.entity_mut(notice).insert(ShellNotice);
    notices(world, view, ui);
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
    let main = world
        .spawn((bevy_game_ui::menu_panel("Expedition Menu"), ChildOf(parent)))
        .insert(Node {
            max_height: Val::Auto,
            flex_shrink: 0.0,
            ..bevy_game_ui::menu_panel_node()
        })
        .id();
    label(
        world,
        main,
        "Menu Title",
        "Enter the Labyrinth",
        UiTextRole::Title,
    );
    let actions = column(
        world,
        main,
        "Main Actions",
        bevy_game_ui::menu_actions_node(),
    );
    control(
        world,
        actions,
        "Start Local",
        "Play locally",
        Action::StartLocal,
        false,
    );
    control(
        world,
        actions,
        "Multiplayer",
        "Play with friends",
        Action::Form(Form::Multiplayer),
        false,
    );
    control(
        world,
        actions,
        "Menu Settings",
        "Settings",
        Action::Settings,
        false,
    );
    label(
        world,
        main,
        "Local Play Advice",
        "Local play controls all six heroes. Play with friends hosts or joins a company.",
        UiTextRole::Supporting,
    );
}

fn multiplayer_menu(world: &mut World, parent: Entity) {
    let main = world
        .spawn((bevy_game_ui::menu_panel("Company Menu"), ChildOf(parent)))
        .insert(Node {
            max_height: Val::Auto,
            flex_shrink: 0.0,
            ..bevy_game_ui::menu_panel_node()
        })
        .id();
    label(
        world,
        main,
        "Menu Title",
        "THE LANTERN COMPANY",
        UiTextRole::Title,
    );
    let actions = column(
        world,
        main,
        "Company Actions",
        bevy_game_ui::menu_actions_node(),
    );
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
    control(
        world,
        actions,
        "Company Back",
        "Back",
        Action::Form(Form::Menu),
        false,
    );
}
