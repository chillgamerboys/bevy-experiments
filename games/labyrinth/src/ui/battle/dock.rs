//! Fixed glyph rail. Explanations live in the shared inspection layer.

use super::*;
use crate::ui::glyphs::{self, Glyph};
use bevy_game_ui::{UiContextHelp, UiControlMetrics};

pub(super) struct DockNodes {
    pub skills: Entity,
    pub confirm: Entity,
    pub rematch: Entity,
    portrait: Entity,
    identity: Entity,
    root: Entity,
    rail: Entity,
}

#[derive(Component)]
struct DockControl;

#[derive(Component)]
struct CommandSurface;

#[expect(
    clippy::too_many_arguments,
    reason = "A named control groups its label, help, glyph and typed action at one call site."
)]
pub(super) fn glyph_control(
    world: &mut World,
    parent: Entity,
    name: &str,
    short: &str,
    title: &str,
    body: &str,
    glyph: Glyph,
    action: Action,
) -> Entity {
    let appearance = world.resource::<LabyrinthAppearance>().clone();
    let entity = world
        .spawn((
            bevy_game_ui::button(name),
            UiSkin::Control,
            appearance.control(false),
            UiControlMetrics::default(),
            bevy_game_ui::UiFocusId::new("labyrinth", name),
            UiContextHelp {
                title: title.to_owned(),
                body: body.to_owned(),
            },
            DockControl,
            bevy_game_ui::UiInspectable,
            action,
            ChildOf(parent),
        ))
        .id();
    world.entity_mut(entity).insert(AccessibleLabel::new(title));
    world.entity_mut(entity).insert(Node {
        min_width: Val::Px(44.0),
        min_height: Val::Px(44.0),
        flex_direction: FlexDirection::Column,
        justify_content: JustifyContent::Center,
        align_items: AlignItems::Center,
        padding: UiRect::axes(Val::Px(7.0), Val::Px(3.0)),
        border: UiRect::bottom(Val::Px(2.0)),
        flex_shrink: 0.0,
        ..default()
    });
    glyphs::mount(world, entity, glyph);
    if !short.is_empty() {
        let text = label(world, entity, "Control Label", short, UiTextRole::Body);
        world.entity_mut(text).insert(TextLayout::no_wrap());
    }
    entity
}

pub(super) fn mount(world: &mut World, root: Entity, _stage: Entity) -> DockNodes {
    let appearance = world.resource::<LabyrinthAppearance>().clone();
    let dock = column(
        world,
        root,
        "Combat Command Dock",
        Node {
            width: Val::Percent(100.0),
            flex_direction: FlexDirection::Column,
            padding: UiRect::axes(Val::Px(8.0), Val::Px(6.0)),
            row_gap: Val::Px(4.0),
            flex_shrink: 0.0,
            ..default()
        },
    );
    world
        .entity_mut(dock)
        .insert((BackgroundColor(appearance.dock), CommandSurface));
    let rail = column(
        world,
        dock,
        "Combat Action Rail",
        Node {
            width: Val::Percent(100.0),
            flex_direction: FlexDirection::Row,
            align_items: AlignItems::Center,
            column_gap: Val::Px(10.0),
            ..default()
        },
    );
    world.entity_mut(rail).insert(UiRegionRole::ActionRail);
    let identity_box = column(
        world,
        rail,
        "Commanding Hero",
        Node {
            width: Val::Percent(23.0),
            max_width: Val::Px(290.0),
            min_width: Val::Px(150.0),
            flex_shrink: 0.0,
            flex_direction: FlexDirection::Row,
            align_items: AlignItems::Center,
            column_gap: Val::Px(6.0),
            overflow: Overflow::clip(),
            ..default()
        },
    );
    let portrait = world
        .spawn((
            Name::new("Command Hero Portrait"),
            ImageNode::default(),
            Node {
                width: Val::Px(48.0),
                height: Val::Px(62.0),
                flex_shrink: 0.0,
                ..default()
            },
            ChildOf(identity_box),
        ))
        .id();
    let identity = label(
        world,
        identity_box,
        "Command Hero Identity",
        "",
        UiTextRole::Body,
    );
    world.entity_mut(identity).insert(Node {
        min_width: Val::Px(0.0),
        flex_basis: Val::Px(0.0),
        flex_grow: 1.0,
        ..default()
    });
    world.entity_mut(identity).insert(TextLayout::no_wrap());
    let scroller = column(
        world,
        rail,
        "Command Scroll",
        Node {
            min_width: Val::Px(0.0),
            flex_grow: 1.0,
            flex_direction: FlexDirection::Row,
            overflow: Overflow::scroll_x(),
            column_gap: Val::Px(16.0),
            padding: UiRect::all(Val::Px(3.0)),
            ..default()
        },
    );
    let skills = column(
        world,
        scroller,
        "Hero Skills",
        Node {
            flex_direction: FlexDirection::Row,
            column_gap: Val::Px(3.0),
            flex_shrink: 0.0,
            ..default()
        },
    );
    let utilities = column(
        world,
        scroller,
        "Universal Actions",
        Node {
            flex_direction: FlexDirection::Row,
            column_gap: Val::Px(3.0),
            flex_shrink: 0.0,
            border: UiRect::left(Val::Px(1.0)),
            padding: UiRect::left(Val::Px(10.0)),
            ..default()
        },
    );
    world
        .entity_mut(utilities)
        .insert(BorderColor::all(appearance.line));
    for (name, _short, body, glyph, choice) in [
        (
            "Reposition",
            "Move",
            "Swap with an adjacent ally. Uses your turn.",
            Glyph::Swap,
            Choice::Reposition,
        ),
        (
            "Rescue",
            "Rescue",
            "Revive a downed ally at 25% maximum HP. Uses your turn.",
            Glyph::Heal,
            Choice::Rescue,
        ),
        (
            "Defend",
            "Guard",
            "Brace reduces direct damage by 2 until your next turn; not bleed.",
            Glyph::Shield,
            Choice::Defend,
        ),
        (
            "Wait",
            "Wait",
            "Spend this turn without another effect.",
            Glyph::Wait,
            Choice::Wait,
        ),
    ] {
        glyph_control(
            world,
            utilities,
            name,
            "",
            name,
            body,
            glyph,
            Action::Choice(choice),
        );
    }
    let confirm = glyph_control(
        world,
        rail,
        "Confirm Combat Action",
        "Confirm",
        "Confirm combat action",
        "Commit the selected action. Selection never spends your turn.",
        Glyph::Confirm,
        Action::Confirm,
    );
    set_disabled(world, confirm, true);
    glyph_control(
        world,
        rail,
        "Cancel Combat Selection",
        "",
        "Cancel combat selection",
        "Clear the selection without spending your turn.",
        Glyph::Cancel,
        Action::Cancel,
    );
    glyph_control(
        world,
        rail,
        "Skillbook Toggle",
        "",
        "Skillbook · K",
        "Browse equipped abilities and related conditions without selecting an action.",
        Glyph::Book,
        Action::ToggleSkillbook,
    );
    let rematch = control(
        world,
        dock,
        "Combat Rematch",
        "Return party to lobby",
        Action::Rematch,
        true,
    );
    if let Some(mut node) = world.get_mut::<Node>(rematch) {
        node.position_type = PositionType::Absolute;
        node.bottom = Val::Percent(100.0);
        node.right = Val::Px(8.0);
    }
    DockNodes {
        skills,
        confirm,
        rematch,
        portrait,
        identity,
        root: dock,
        rail,
    }
}

pub(super) fn mount_skills(world: &mut World, parent: Entity, loadout: &[SkillId]) {
    let children = world
        .get::<Children>(parent)
        .map(|v| v.to_vec())
        .unwrap_or_default();
    for child in children {
        world.despawn(child);
    }
    for (index, skill) in loadout.iter().copied().enumerate() {
        let definition = skill_definition(skill);
        let entity = glyph_control(
            world,
            parent,
            &format!("Skill {index}"),
            &format!("{}", index + 1),
            definition.name,
            definition.description,
            glyphs::for_skill(skill),
            Action::SkillSlot(index),
        );
        world
            .entity_mut(entity)
            .insert(bevy_game_ui::UiTooltipSource(
                super::tooltips::ability_subject(skill),
            ))
            .insert(bevy_game_ui::UiFocusId::new(
                "labyrinth-skills",
                format!("{skill:?}"),
            ));
    }
}

pub(super) fn update(world: &mut World, nodes: &DockNodes, view: &LabyrinthView, ui: &UiState) {
    let Some(snapshot) = view.combat.as_ref() else {
        return;
    };
    let actor = inspection::display_actor(view);
    let identity = actor.map_or_else(
        || "Watching".to_owned(),
        |actor| {
            let state = if view.paused {
                "Waiting"
            } else if snapshot.active_actor == Some(actor.id) {
                "Your turn"
            } else {
                "Your hero"
            };
            format!(
                "{} {}\n{state}",
                actors::token(snapshot, actor),
                actor.name()
            )
        },
    );
    set_text(world, nodes.identity, identity);
    if let Some(actor) = actor {
        let image = crate::scene::portrait_image(world, actor.kind);
        if world.get::<ImageNode>(nodes.portrait).is_none_or(|old| {
            old.image != image.image || old.rect != image.rect || old.color != image.color
        }) {
            world.entity_mut(nodes.portrait).insert(image);
        }
    }
    fixed_geometry(world, nodes);
    let reason = inspection::slot_value(
        Slot::Reason,
        view,
        ui,
        snapshot,
        world.resource::<crate::presentation::CombatDisclosure>(),
    );
    let selected = inspection::choice_title(ui.selected);
    let help = UiContextHelp {
        title: "Confirm combat action".to_owned(),
        body: format!("{selected}\n{reason}"),
    };
    if world.get::<UiContextHelp>(nodes.confirm) != Some(&help) {
        world.entity_mut(nodes.confirm).insert(help);
    }
    let appearance = world.resource::<LabyrinthAppearance>().clone();
    let controls = world
        .query_filtered::<Entity, With<DockControl>>()
        .iter(world)
        .collect::<Vec<_>>();
    for entity in controls {
        let selected = match world.get::<Action>(entity) {
            Some(Action::Choice(choice)) => ui.selected == Some(*choice),
            Some(Action::SkillSlot(index)) => actor
                .and_then(|actor| actor.skills().get(*index))
                .is_some_and(|skill| ui.selected == Some(Choice::Skill(*skill))),
            Some(Action::Confirm) => selected_action(view, ui).is_ok(),
            _ => false,
        };
        let wanted = appearance.control(selected);
        if world.get::<UiSkinOverrides>(entity) != Some(&wanted) {
            world.entity_mut(entity).insert(wanted);
        }
        if let Some(Action::SkillSlot(index)) = world.get::<Action>(entity) {
            if let Some(skill) = actor.and_then(|actor| actor.skills().get(*index)) {
                let definition = skill_definition(*skill);
                let uses = actor
                    .filter(|actor| {
                        let policy = world
                            .resource::<crate::presentation::CombatDisclosure>()
                            .actor(actor.id);
                        policy.details && policy.statuses
                    })
                    .and_then(|actor| actor.remaining_uses(*skill))
                    .map_or_else(String::new, |left| format!(" {left} uses remaining."));
                let title = format!("{}. {}{uses}", index + 1, definition.name);
                if world.get::<AccessibleLabel>(entity).map(|v| &v.0) != Some(&title) {
                    world.entity_mut(entity).insert(AccessibleLabel::new(title));
                }
            }
        }
    }
    let surfaces = world
        .query_filtered::<Entity, With<CommandSurface>>()
        .iter(world)
        .collect::<Vec<_>>();
    for entity in surfaces {
        if world.get::<BackgroundColor>(entity) != Some(&BackgroundColor(appearance.dock)) {
            world
                .entity_mut(entity)
                .insert(BackgroundColor(appearance.dock));
        }
    }
}

fn fixed_geometry(world: &mut World, nodes: &DockNodes) {
    let metrics = *world.resource::<ResolvedUiMetrics>();
    let rail = (62.0 * metrics.content_scale).max(44.0 * metrics.control_scale);
    for (entity, height) in [(nodes.root, rail + 12.0), (nodes.rail, rail)] {
        if let Some(mut node) = world.get_mut::<Node>(entity) {
            if node.height != Val::Px(height) {
                node.height = Val::Px(height);
                node.min_height = Val::Px(height);
                node.flex_shrink = 0.0;
            }
        }
    }
}
