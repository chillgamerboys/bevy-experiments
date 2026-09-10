//! Game-owned stage-first composition. Appearance never determines organization.

use super::*;
use crate::ui::glyphs::Glyph;

pub(super) fn mount(world: &mut World, snapshot: &CombatSnapshot, viewport: UiViewportClass) {
    let root = world
        .spawn((
            bevy_gamekit::ui::screen_root("Labyrinth Battlefield"),
            BattleRoot,
            bevy_gamekit::ui::UiTooltipHost,
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
            "Battle Log Toggle",
            "Combat log",
            Glyph::Book,
            Action::ToggleLog,
        ),
        (
            "Battle Settings",
            "Game menu",
            Glyph::Settings,
            Action::GameMenu,
        ),
    ] {
        let control = dock::glyph_control(world, hud, key, "", title, "", glyph, action);
        world
            .entity_mut(control)
            .remove::<bevy_gamekit::ui::UiContextHelp>();
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
    let dock = dock::mount(world, root, formations);
    world.insert_resource(BattleNodes {
        heroes,
        enemies,
        skills: dock.skills,
        loadout: Vec::new(),
        confirm: dock.confirm,
        rematch: dock.rematch,
        dock,
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
