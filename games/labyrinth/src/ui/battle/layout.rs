//! Game-owned battlefield regions and responsive layout.

use super::*;

pub(super) fn mount(world: &mut World, snapshot: &CombatSnapshot, viewport: UiViewportClass) {
    let stacked = viewport == UiViewportClass::Compact
        && world.resource::<ResolvedUiMetrics>().content_scale > 1.25;
    let root = world
        .spawn((
            bevy_game_ui::screen_root("Labyrinth Battlefield"),
            UiSkin::Screen,
            BattleRoot,
        ))
        .id();
    world.entity_mut(root).insert(Node {
        width: Val::Percent(100.0),
        height: Val::Percent(100.0),
        flex_direction: FlexDirection::Column,
        padding: UiRect::all(Val::Px(18.0)),
        row_gap: Val::Px(12.0),
        overflow: Overflow::clip(),
        ..default()
    });
    let hud = column(
        world,
        root,
        "Battle HUD",
        Node {
            width: Val::Percent(100.0),
            flex_direction: FlexDirection::Row,
            flex_wrap: FlexWrap::Wrap,
            justify_content: JustifyContent::SpaceBetween,
            row_gap: Val::Px(8.0),
            column_gap: Val::Px(14.0),
            flex_shrink: 0.0,
            ..default()
        },
    );
    world.entity_mut(hud).insert(UiRegionRole::Hud);
    text_slot(world, hud, "Battle Status", Slot::Hud, UiTextRole::Body);
    control(
        world,
        hud,
        "Timeline Toggle",
        "Order",
        Action::ToggleTimeline,
        false,
    );
    control(
        world,
        hud,
        "Battle Settings",
        "Settings",
        Action::Settings,
        false,
    );
    control(
        world,
        hud,
        "Battle Log Toggle",
        "Details",
        Action::ToggleLog,
        false,
    );
    let scroll = column(
        world,
        root,
        "Battlefield Scroll",
        Node {
            width: Val::Percent(100.0),
            flex_grow: 1.0,
            min_height: Val::Px(0.0),
            flex_direction: FlexDirection::Column,
            row_gap: Val::Px(12.0),
            overflow: Overflow::scroll_y(),
            ..default()
        },
    );
    world.entity_mut(scroll).insert(UiRegionRole::ScrollList);
    text_slot(
        world,
        scroll,
        "Initiative Timeline",
        Slot::Timeline,
        UiTextRole::Supporting,
    );
    let formations = column(
        world,
        scroll,
        "Facing Formations",
        Node {
            width: Val::Percent(100.0),
            flex_direction: if stacked {
                FlexDirection::Column
            } else {
                FlexDirection::Row
            },
            row_gap: Val::Px(14.0),
            column_gap: Val::Px(22.0),
            flex_shrink: 0.0,
            ..default()
        },
    );
    let heroes = formation(
        world,
        formations,
        "Your Company",
        "YOUR COMPANY | rear 4  3  2  1 front",
        stacked,
    );
    let enemies = formation(
        world,
        formations,
        "The Opposition",
        "THE OPPOSITION | front 1  2  3  4 rear",
        stacked,
    );
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
    let inspector = world
        .spawn((
            bevy_game_ui::panel("Actor Inspector"),
            UiSkin::Panel,
            ChildOf(scroll),
        ))
        .id();
    text_slot(
        world,
        inspector,
        "Inspector Text",
        Slot::Inspector,
        UiTextRole::Supporting,
    );
    let log = world
        .spawn((
            bevy_game_ui::panel("Combat Log"),
            UiSkin::Panel,
            UiRegionRole::ActivityFeed,
            ChildOf(scroll),
        ))
        .id();
    text_slot(world, log, "Log Text", Slot::Log, UiTextRole::Supporting);
    let rail = column(
        world,
        root,
        "Combat Action Rail",
        Node {
            width: Val::Percent(100.0),
            max_height: Val::Percent(34.0),
            min_height: Val::Px(100.0),
            flex_direction: FlexDirection::Column,
            row_gap: Val::Px(10.0),
            padding: UiRect::all(Val::Px(12.0)),
            overflow: Overflow::scroll_y(),
            flex_shrink: 0.0,
            ..default()
        },
    );
    world.entity_mut(rail).insert((
        UiRegionRole::ActionRail,
        BackgroundColor(Color::srgb(0.055, 0.075, 0.085)),
    ));
    let skills = column(
        world,
        rail,
        "Hero Skills",
        Node {
            width: Val::Percent(100.0),
            flex_direction: FlexDirection::Row,
            flex_wrap: FlexWrap::Wrap,
            column_gap: Val::Px(8.0),
            row_gap: Val::Px(8.0),
            ..default()
        },
    );
    for index in 0..4 {
        let entity = control(
            world,
            skills,
            format!("Skill {index}"),
            "Skill",
            Action::SkillSlot(index),
            false,
        );
        if let Some(text) = world
            .get::<Children>(entity)
            .and_then(|children| children.first())
            .copied()
        {
            world.entity_mut(text).insert(Slot::Skill(index));
        }
    }
    let universal = column(
        world,
        rail,
        "Universal Actions",
        Node {
            width: Val::Percent(100.0),
            flex_direction: FlexDirection::Row,
            flex_wrap: FlexWrap::Wrap,
            column_gap: Val::Px(8.0),
            row_gap: Val::Px(8.0),
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
    let selected = text_slot(
        world,
        rail,
        "Selected Skill",
        Slot::Selected,
        UiTextRole::Body,
    );
    let reason = text_slot(
        world,
        rail,
        "Target Legality",
        Slot::Reason,
        UiTextRole::Supporting,
    );
    world
        .entity_mut(rail)
        .replace_children(&[selected, reason, skills, universal]);
    let commit = column(
        world,
        root,
        "Commit Actions",
        Node {
            width: Val::Percent(100.0),
            flex_direction: FlexDirection::Row,
            flex_wrap: FlexWrap::Wrap,
            column_gap: Val::Px(10.0),
            row_gap: Val::Px(10.0),
            flex_shrink: 0.0,
            ..default()
        },
    );
    let confirm = control(
        world,
        commit,
        "Confirm Combat Action",
        "Confirm action",
        Action::Confirm,
        true,
    );
    control(
        world,
        commit,
        "Cancel Combat Selection",
        "Cancel selection",
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
        formations,
        heroes,
        enemies,
        confirm,
        rematch,
        inspector,
        log,
        viewport,
        stacked,
        last_event: None,
        was_paused: false,
        feedback: BTreeMap::new(),
    });
}
