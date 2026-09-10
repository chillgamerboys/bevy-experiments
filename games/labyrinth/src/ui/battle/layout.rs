//! Game-owned stage-first composition. Appearance never determines organization.

use super::*;
use crate::ui::glyphs::Glyph;

pub(super) fn mount(world: &mut World, snapshot: &CombatSnapshot, viewport: UiViewportClass) {
    let root = world
        .spawn((
            bevy_game_ui::screen_root("Labyrinth Battlefield"),
            BattleRoot,
            bevy_game_ui::UiTooltipHost,
        ))
        .id();
    world.entity_mut(root).insert(Node {
        width: Val::Percent(100.0),
        height: Val::Percent(100.0),
        flex_direction: FlexDirection::Column,
        padding: UiRect::all(Val::Px(10.0)),
        row_gap: Val::Px(5.0),
        overflow: Overflow::clip(),
        ..default()
    });
    let hud = row(world, root, "Battle HUD");
    world.entity_mut(hud).insert(UiRegionRole::Hud);
    let title = text_slot(world, hud, "Battle Status", Slot::Hud, UiTextRole::Body);
    world.entity_mut(title).insert(Node {
        flex_grow: 1.0,
        align_self: AlignSelf::Center,
        ..default()
    });
    for (key, title, glyph, action) in [
        (
            "Timeline Toggle",
            "Initiative details",
            Glyph::Order,
            Action::ToggleTimeline,
        ),
        (
            "Inspector Toggle",
            "Inspect selected actor and ability",
            Glyph::Inspect,
            Action::ToggleInspector,
        ),
        (
            "Battle Log Toggle",
            "Combat log",
            Glyph::Book,
            Action::ToggleLog,
        ),
        (
            "Battle Settings",
            "Readability and motion settings",
            Glyph::Settings,
            Action::Settings,
        ),
    ] {
        dock::glyph_control(world, hud, key, "", title, "", glyph, action);
    }
    let timeline = timeline::mount(world, root, snapshot);
    if let Some(mut node) = world.get_mut::<Node>(timeline) {
        node.flex_shrink = 0.0;
    }
    let formations = column(
        world,
        root,
        "Facing Formations",
        Node {
            width: Val::Percent(100.0),
            min_height: Val::Px(140.0),
            flex_grow: 1.0,
            flex_direction: FlexDirection::Row,
            column_gap: Val::Percent(2.0),
            align_items: AlignItems::Stretch,
            ..default()
        },
    );
    world
        .entity_mut(formations)
        .insert(crate::scene::SceneStageAnchor);
    let feedback = text_slot(
        world,
        root,
        "Combat Outcome Feedback",
        Slot::Feedback,
        UiTextRole::Supporting,
    );
    world.entity_mut(feedback).insert(Node {
        position_type: PositionType::Absolute,
        top: Val::Percent(20.0),
        left: Val::Percent(25.0),
        width: Val::Percent(50.0),
        ..default()
    });
    let heroes = formation(world, formations, "Your Company", "", false);
    let enemies = formation(world, formations, "The Opposition", "", false);
    for actor in &snapshot.actors {
        mount_actor(
            world,
            if actor.team() == Team::Heroes {
                heroes
            } else {
                enemies
            },
            actor,
        );
    }
    // Secondary detail floats over the scene; it never pushes actors off-screen.
    let drawer = column(
        world,
        root,
        "Battle Detail Drawer",
        Node {
            position_type: PositionType::Absolute,
            right: Val::Px(16.0),
            top: Val::Px(110.0),
            width: Val::Percent(48.0),
            height: Val::Percent(55.0),
            flex_direction: FlexDirection::Column,
            overflow: Overflow::clip(),
            padding: UiRect::all(Val::Px(16.0)),
            ..default()
        },
    );
    let detail_color = world.resource::<LabyrinthAppearance>().detail;
    world.entity_mut(drawer).insert((
        GlobalZIndex(50),
        bevy_game_ui::UiModalScope,
        bevy::input_focus::tab_navigation::TabGroup::modal(),
        BackgroundColor(detail_color),
        bevy::ui::FocusPolicy::Block,
    ));
    let drawer_controls = row(world, drawer, "Detail Controls");
    control(
        world,
        drawer_controls,
        "Close Details",
        "Close",
        Action::Cancel,
        false,
    );
    dock::glyph_control(
        world,
        drawer_controls,
        "Combat Leave",
        "Menu",
        "Return to menu",
        "Leave this encounter and return to the main menu.",
        Glyph::Exit,
        Action::Leave,
    );
    control(
        world,
        drawer_controls,
        "Previous Detail Page",
        "Page up",
        Action::ScrollDetails(-1),
        false,
    );
    control(
        world,
        drawer_controls,
        "Next Detail Page",
        "Page down",
        Action::ScrollDetails(1),
        false,
    );
    let drawer_body = column(
        world,
        drawer,
        "Detail Scroll",
        Node {
            width: Val::Percent(100.0),
            flex_grow: 1.0,
            min_height: Val::Px(0.0),
            flex_direction: FlexDirection::Column,
            overflow: Overflow::scroll_y(),
            ..default()
        },
    );
    let inspector = column(
        world,
        drawer_body,
        "Actor Inspector",
        Node {
            width: Val::Percent(100.0),
            flex_direction: FlexDirection::Column,
            flex_shrink: 0.0,
            ..default()
        },
    );
    text_slot(
        world,
        inspector,
        "Inspector Text",
        Slot::Inspector,
        UiTextRole::Supporting,
    );
    let log = column(
        world,
        drawer_body,
        "Combat Log",
        Node {
            width: Val::Percent(100.0),
            flex_shrink: 0.0,
            ..default()
        },
    );
    world.entity_mut(log).insert(UiRegionRole::ActivityFeed);
    text_slot(world, log, "Log Text", Slot::Log, UiTextRole::Supporting);
    let order = column(
        world,
        drawer_body,
        "Initiative Details",
        Node {
            width: Val::Percent(100.0),
            flex_shrink: 0.0,
            ..default()
        },
    );
    text_slot(
        world,
        order,
        "Initiative Rolls",
        Slot::Order,
        UiTextRole::Supporting,
    );
    let dock = dock::mount(world, root, formations);
    world.insert_resource(BattleNodes {
        heroes,
        enemies,
        skills: dock.skills,
        loadout: Vec::new(),
        confirm: dock.confirm,
        rematch: dock.rematch,
        dock,
        inspector,
        log,
        drawer,
        drawer_body,
        order,
        viewport,
        last_event: None,
        was_paused: false,
        feedback: BTreeMap::new(),
    });
}

fn row(world: &mut World, parent: Entity, name: &str) -> Entity {
    column(
        world,
        parent,
        name,
        Node {
            width: Val::Percent(100.0),
            flex_direction: FlexDirection::Row,
            flex_wrap: FlexWrap::NoWrap,
            column_gap: Val::Px(8.0),
            row_gap: Val::Px(6.0),
            flex_shrink: 0.0,
            ..default()
        },
    )
}
