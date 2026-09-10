//! Game-owned stage-first composition. Appearance never determines organization.

use super::*;

pub(super) fn mount(world: &mut World, snapshot: &CombatSnapshot, viewport: UiViewportClass) {
    let root = world
        .spawn((
            bevy_game_ui::screen_root("Labyrinth Battlefield"),
            BattleRoot,
        ))
        .id();
    world.entity_mut(root).insert(Node {
        width: Val::Percent(100.0),
        height: Val::Percent(100.0),
        flex_direction: FlexDirection::Column,
        padding: UiRect::all(Val::Px(16.0)),
        row_gap: Val::Px(8.0),
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
    for (key, title, action) in [
        ("Timeline Toggle", "Order", Action::ToggleTimeline),
        ("Inspector Toggle", "Inspect", Action::ToggleInspector),
        ("Battle Log Toggle", "Log", Action::ToggleLog),
        ("Battle Settings", "Settings", Action::Settings),
    ] {
        control(world, hud, key, title, action, false);
    }
    let timeline = text_slot(
        world,
        root,
        "Initiative Timeline",
        Slot::Timeline,
        UiTextRole::Supporting,
    );
    world.entity_mut(timeline).insert(Node {
        flex_shrink: 0.0,
        ..default()
    });
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
    world.entity_mut(drawer).insert((
        GlobalZIndex(50),
        bevy_game_ui::UiModalScope,
        bevy::input_focus::tab_navigation::TabGroup::modal(),
        BackgroundColor(Color::srgba(0.025, 0.035, 0.045, 0.98)),
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
    let rail = column(
        world,
        root,
        "Combat Action Rail",
        Node {
            width: Val::Percent(100.0),
            min_height: Val::Px(44.0),
            flex_direction: FlexDirection::Row,
            row_gap: Val::Px(6.0),
            padding: UiRect::axes(Val::Px(12.0), Val::Px(8.0)),
            overflow: Overflow::scroll_x(),
            flex_shrink: 0.0,
            ..default()
        },
    );
    world.entity_mut(rail).insert((
        UiRegionRole::ActionRail,
        BackgroundColor(Color::srgba(0.025, 0.035, 0.045, 0.94)),
    ));
    let skills = row(world, rail, "Hero Skills");
    if let Some(mut node) = world.get_mut::<Node>(skills) {
        node.width = Val::Auto;
        node.flex_wrap = FlexWrap::NoWrap;
    }
    text_slot(
        world,
        root,
        "Target Legality",
        Slot::Reason,
        UiTextRole::Supporting,
    );
    let commit = row(world, root, "Commit Actions");
    let universal = column(
        world,
        commit,
        "Universal Actions",
        Node {
            flex_direction: FlexDirection::Row,
            column_gap: Val::Px(8.0),
            ..default()
        },
    );
    for (key, title, choice) in [
        ("Reposition", "Move", Choice::Reposition),
        ("Rescue", "Rescue", Choice::Rescue),
        ("Defend", "Defend", Choice::Defend),
        ("Wait", "Wait", Choice::Wait),
    ] {
        control(world, universal, key, title, Action::Choice(choice), false);
    }
    text_slot(
        world,
        inspector,
        "Selected Skill",
        Slot::Selected,
        UiTextRole::Body,
    );
    let confirm = control(
        world,
        commit,
        "Confirm Combat Action",
        "Confirm",
        Action::Confirm,
        true,
    );
    control(
        world,
        commit,
        "Cancel Combat Selection",
        "Cancel",
        Action::Cancel,
        false,
    );
    let rematch = control(
        world,
        commit,
        "Combat Rematch",
        "Return party to lobby",
        Action::Rematch,
        true,
    );
    control(world, commit, "Combat Leave", "Menu", Action::Leave, false);
    world.insert_resource(BattleNodes {
        heroes,
        enemies,
        skills,
        loadout: Vec::new(),
        confirm,
        rematch,
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
            flex_wrap: FlexWrap::Wrap,
            column_gap: Val::Px(8.0),
            row_gap: Val::Px(6.0),
            flex_shrink: 0.0,
            ..default()
        },
    )
}

pub(super) fn mount_skills(world: &mut World, parent: Entity, loadout: &[SkillId]) {
    let children = world
        .get::<Children>(parent)
        .map(|children| children.to_vec())
        .unwrap_or_default();
    for child in children {
        world.despawn(child);
    }
    for (index, skill) in loadout.iter().enumerate() {
        let entity = control(
            world,
            parent,
            format!("Skill {index}"),
            skill_definition(*skill).name,
            Action::SkillSlot(index),
            false,
        );
        world
            .entity_mut(entity)
            .insert(bevy_game_ui::UiFocusId::new(
                "labyrinth-skills",
                format!("{skill:?}"),
            ));
        if let Some(text) = world
            .get::<Children>(entity)
            .and_then(|children| children.first())
            .copied()
        {
            world.entity_mut(text).insert(Slot::Skill(index));
        }
    }
}
