//! Carterfight's transparent scene overlay and deliberately local pixel skin.

use super::{systems::CarterAssets, CarterfightIntent, CarterfightPhase, CarterfightView};
use bevy::prelude::*;
use bevy_gamekit::ui::{
    button, screen_root, text, ResolvedUiMetrics, UiControlMetrics, UiDisabled, UiFocusId,
    UiMotionPreference, UiRegionRole, UiScaleMode, UiScalePreference, UiSkin, UiSkinOverrides,
    UiTextRole, UiTextStyle,
};

const INK: Color = Color::srgb(0.06, 0.11, 0.23);
const PAPER: Color = Color::srgb(0.97, 0.96, 0.91);
const GOLD: Color = Color::srgb(1.0, 0.81, 0.36);

#[derive(Component)]
struct TextSlot(Slot);
#[derive(Clone, Copy)]
enum Slot {
    Phase,
    Player,
    Narration,
    Advance,
    Scale,
    Motion,
    Sound,
    Move(&'static str),
}
#[derive(Component)]
pub(super) struct CarterSprite;
#[derive(Component)]
pub(super) struct CarterHealth;
#[derive(Component)]
pub(super) struct HealthFill;
#[derive(Component)]
struct Cursor;
#[derive(Component)]
pub(super) struct SceneStage;

#[derive(Resource)]
pub(super) struct UiNodes {
    pub(super) confirm: Entity,
    pub(super) advance: Entity,
    cancel: Entity,
    move_list: Entity,
    pub(super) moves: Vec<Entity>,
}

fn label(
    world: &mut World,
    parent: Entity,
    name: &str,
    value: &str,
    role: UiTextRole,
    color: Color,
    size: f32,
    slot: Option<Slot>,
) -> Entity {
    let font = world.resource::<CarterAssets>().font.clone();
    let bundle = text(world.resource::<bevy_gamekit::ui::UiFonts>(), role, value);
    let entity = world
        .spawn((
            Name::new(name.to_owned()),
            bundle,
            UiTextStyle {
                base_size: Some(size),
                font: Some(font),
            },
            ChildOf(parent),
        ))
        .id();
    world.entity_mut(entity).insert(TextColor(color));
    if let Some(slot) = slot {
        world.entity_mut(entity).insert(TextSlot(slot));
    }
    entity
}

fn node(world: &mut World, parent: Entity, name: &str, layout: Node) -> Entity {
    world
        .spawn((Name::new(name.to_owned()), layout, ChildOf(parent)))
        .id()
}

fn control(
    world: &mut World,
    parent: Entity,
    name: &str,
    value: &str,
    action: CarterfightIntent,
    slot: Option<Slot>,
) -> Entity {
    let entity = world
        .spawn((
            button(name),
            UiFocusId::new("carterfight", name),
            action,
            UiSkin::Control,
            UiSkinOverrides {
                background: Some(PAPER),
                hovered: Some(Color::srgb(1.0, 0.90, 0.60)),
                pressed: Some(GOLD),
                disabled: Some(Color::srgb(0.56, 0.59, 0.64)),
                border: Some(INK),
                focus: Some(GOLD),
                ..default()
            },
            UiControlMetrics {
                min_size: Vec2::new(120.0, 48.0),
            },
            ChildOf(parent),
        ))
        .id();
    world.entity_mut(entity).insert((
        AccessibleLabel::new(value.to_owned()),
        Node {
            min_height: Val::Px(48.0),
            padding: UiRect::axes(Val::Px(14.0), Val::Px(10.0)),
            border: UiRect::all(Val::Px(3.0)),
            align_items: AlignItems::Center,
            justify_content: JustifyContent::Center,
            flex_shrink: 0.0,
            ..default()
        },
    ));
    label(
        world,
        entity,
        &format!("{name} label"),
        value,
        UiTextRole::Body,
        INK,
        18.0,
        slot,
    );
    entity
}

pub(super) fn mount(world: &mut World) {
    let root = world
        .spawn(screen_root("Carterfight transparent overlay"))
        .id();
    world.entity_mut(root).insert(Node {
        width: Val::Percent(100.0),
        height: Val::Percent(100.0),
        flex_direction: FlexDirection::Column,
        padding: UiRect::all(Val::Px(20.0)),
        row_gap: Val::Px(12.0),
        overflow: Overflow::clip(),
        ..default()
    });
    let header = node(
        world,
        root,
        "Carterfight header",
        Node {
            width: Val::Percent(100.0),
            flex_direction: FlexDirection::Row,
            flex_wrap: FlexWrap::Wrap,
            align_items: AlignItems::Center,
            justify_content: JustifyContent::SpaceBetween,
            column_gap: Val::Px(12.0),
            row_gap: Val::Px(8.0),
            max_height: Val::Percent(22.0),
            overflow: Overflow::scroll_y(),
            flex_shrink: 0.0,
            ..default()
        },
    );
    world
        .entity_mut(header)
        .insert(BackgroundColor(Color::srgb(0.055, 0.08, 0.15)));
    label(
        world,
        header,
        "Carterfight title",
        "CARTERFIGHT",
        UiTextRole::Title,
        GOLD,
        26.0,
        None,
    );
    let settings = node(
        world,
        header,
        "Accessibility controls",
        Node {
            flex_wrap: FlexWrap::Wrap,
            column_gap: Val::Px(8.0),
            row_gap: Val::Px(8.0),
            ..default()
        },
    );
    control(
        world,
        settings,
        "Semantic scale",
        "UI: Auto",
        CarterfightIntent::ToggleScale,
        Some(Slot::Scale),
    );
    control(
        world,
        settings,
        "Reduced motion",
        "Motion: on",
        CarterfightIntent::ToggleMotion,
        Some(Slot::Motion),
    );
    control(
        world,
        settings,
        "Dialogue sound",
        "Sound: on",
        CarterfightIntent::ToggleSound,
        Some(Slot::Sound),
    );

    let content = node(
        world,
        root,
        "Scrollable battle content",
        Node {
            width: Val::Percent(100.0),
            flex_grow: 1.0,
            flex_basis: Val::Px(0.0),
            min_height: Val::Px(0.0),
            flex_direction: FlexDirection::Column,
            overflow: Overflow::scroll_y(),
            row_gap: Val::Px(14.0),
            ..default()
        },
    );
    let stage = node(
        world,
        content,
        "Carter scene",
        Node {
            width: Val::Percent(100.0),
            height: Val::Px(220.0),
            min_height: Val::Px(220.0),
            flex_shrink: 0.0,
            padding: UiRect::all(Val::Px(16.0)),
            flex_direction: FlexDirection::Column,
            row_gap: Val::Px(18.0),
            ..default()
        },
    );
    world
        .entity_mut(stage)
        .insert((SceneStage, UiRegionRole::Hud));
    label(
        world,
        stage,
        "Phase label",
        "ENCOUNTER",
        UiTextRole::Supporting,
        GOLD,
        18.0,
        Some(Slot::Phase),
    );
    label(
        world,
        stage,
        "Player health",
        "PLAYER\n60 / 60 HP",
        UiTextRole::Body,
        PAPER,
        22.0,
        Some(Slot::Player),
    );

    // Authored art remains a world-space sprite, independently composed behind
    // the transparent overlay. Its health uses the immutable displayed HP view.
    let (portrait, font) = {
        let assets = world.resource::<CarterAssets>();
        (assets.carter.clone(), assets.font.clone())
    };
    world.spawn((
        Name::new("Carter portrait"),
        Sprite::from_image(portrait),
        Transform::default(),
        CarterSprite,
    ));
    world.spawn((
        Name::new("Carter health bar"),
        Sprite::from_color(GOLD, Vec2::new(200.0, 12.0)),
        bevy::sprite::Anchor::CENTER_LEFT,
        Transform::default(),
        HealthFill,
    ));
    world.spawn((
        Name::new("Carter health label"),
        Text2d::new("CARTER 60 / 60"),
        TextFont {
            font: font.into(),
            font_size: FontSize::Px(18.0),
            ..default()
        },
        TextColor(PAPER),
        Transform::default(),
        CarterHealth,
    ));

    let actions = node(
        world,
        content,
        "Move actions",
        Node {
            width: Val::Percent(100.0),
            flex_direction: FlexDirection::Column,
            row_gap: Val::Px(12.0),
            padding: UiRect::all(Val::Px(12.0)),
            flex_shrink: 0.0,
            ..default()
        },
    );
    world.entity_mut(actions).insert(UiRegionRole::ActionRail);
    let row = node(
        world,
        actions,
        "Move choices",
        Node {
            width: Val::Percent(100.0),
            flex_wrap: FlexWrap::Wrap,
            column_gap: Val::Px(12.0),
            row_gap: Val::Px(8.0),
            ..default()
        },
    );
    let moves = [
        ("jab", "1 Jab"),
        ("haymaker", "2 Haymaker"),
        ("headbutt", "3 Headbutt"),
    ]
    .into_iter()
    .map(|(id, title)| {
        control(
            world,
            row,
            &format!("Choose {id}"),
            title,
            CarterfightIntent::SelectMove(id),
            Some(Slot::Move(id)),
        )
    })
    .collect();
    label(
        world,
        actions,
        "Scroll hint",
        "1-3 choose. Space confirms. Tab / scroll reaches controls.",
        UiTextRole::Supporting,
        PAPER,
        18.0,
        None,
    );

    let dialogue = node(
        world,
        root,
        "Carterfight dialogue",
        Node {
            width: Val::Percent(100.0),
            min_height: Val::Px(110.0),
            max_height: Val::Percent(28.0),
            padding: UiRect::all(Val::Px(18.0)),
            border: UiRect::all(Val::Px(5.0)),
            flex_direction: FlexDirection::Column,
            overflow: Overflow::scroll_y(),
            flex_shrink: 0.0,
            ..default()
        },
    );
    world.entity_mut(dialogue).insert((
        BackgroundColor(PAPER),
        BorderColor::all(Color::srgb(0.29, 0.43, 0.65)),
        UiRegionRole::ActivityFeed,
    ));
    label(
        world,
        dialogue,
        "Narration",
        "",
        UiTextRole::Body,
        INK,
        22.0,
        Some(Slot::Narration),
    );
    let cursor_image = world.resource::<CarterAssets>().cursor.clone();
    world.spawn((
        Name::new("Narration cursor"),
        ImageNode::new(cursor_image),
        Node {
            position_type: PositionType::Absolute,
            right: Val::Px(8.0),
            bottom: Val::Px(8.0),
            width: Val::Px(20.0),
            height: Val::Px(12.0),
            ..default()
        },
        Cursor,
        ChildOf(dialogue),
    ));
    let footer = node(
        world,
        root,
        "Persistent decision controls",
        Node {
            width: Val::Percent(100.0),
            flex_wrap: FlexWrap::Wrap,
            column_gap: Val::Px(12.0),
            row_gap: Val::Px(8.0),
            flex_shrink: 0.0,
            ..default()
        },
    );
    world
        .entity_mut(footer)
        .insert(BackgroundColor(Color::srgb(0.055, 0.08, 0.15)));
    let move_list = control(
        world,
        footer,
        "Inspect move list",
        "Moves",
        CarterfightIntent::InspectMoves,
        None,
    );
    let confirm = control(
        world,
        footer,
        "Confirm move",
        "Confirm",
        CarterfightIntent::ConfirmMove,
        None,
    );
    let cancel = control(
        world,
        footer,
        "Cancel selection",
        "Cancel",
        CarterfightIntent::CancelSelection,
        None,
    );
    let advance = control(
        world,
        footer,
        "Advance dialogue",
        "Continue",
        CarterfightIntent::Advance,
        Some(Slot::Advance),
    );
    world.insert_resource(UiNodes {
        confirm,
        advance,
        cancel,
        move_list,
        moves,
    });
}

fn disabled(world: &mut World, entity: Entity, value: bool) {
    if world.get::<UiDisabled>(entity).is_some() != value {
        if value {
            world.entity_mut(entity).insert(UiDisabled);
        } else {
            world.entity_mut(entity).remove::<UiDisabled>();
        }
    }
}

pub(super) fn present(world: &mut World) {
    let view = world.resource::<CarterfightView>().clone();
    let scale = world.resource::<UiScalePreference>().0;
    let reduced = world.resource::<UiMotionPreference>().reduced;
    let slots = world
        .query::<(Entity, &TextSlot)>()
        .iter(world)
        .map(|(entity, slot)| (entity, slot.0))
        .collect::<Vec<_>>();
    for (entity, slot) in slots {
        let value = match slot {
            Slot::Phase => match view.phase {
                CarterfightPhase::Intro => "ENCOUNTER".to_owned(),
                CarterfightPhase::Battle => format!("TURN {}", view.turn + 1),
                CarterfightPhase::Outro => "FIGHT COMPLETE".to_owned(),
            },
            Slot::Player => format!("PLAYER\n{} / {} HP", view.player_hp, view.player_max),
            Slot::Narration => {
                if view.can_select {
                    view.selected_description.clone()
                } else {
                    view.narration.clone()
                }
            }
            Slot::Advance => view.continue_label.to_owned(),
            Slot::Scale => if scale == UiScaleMode::Percent200 {
                "UI: 200%"
            } else {
                "UI: Auto"
            }
            .to_owned(),
            Slot::Motion => if reduced { "Motion: off" } else { "Motion: on" }.to_owned(),
            Slot::Sound => if view.sound {
                "Sound: on"
            } else {
                "Sound: off"
            }
            .to_owned(),
            Slot::Move(id) => view
                .moves
                .iter()
                .enumerate()
                .find(|(_, option)| option.id == id)
                .map_or_else(String::new, |(index, option)| {
                    format!(
                        "{} {}{}",
                        index + 1,
                        option.name,
                        if option.selected { " *" } else { "" }
                    )
                }),
        };
        if let Some(parent) = world.get::<ChildOf>(entity).map(ChildOf::parent) {
            if world.get::<CarterfightIntent>(parent).is_some()
                && world
                    .get::<AccessibleLabel>(parent)
                    .is_none_or(|label| label.0 != value)
            {
                world
                    .entity_mut(parent)
                    .insert(AccessibleLabel::new(value.clone()));
            }
        }
        if let Some(mut current) = world.get_mut::<Text>(entity) {
            if current.0 != value {
                current.0 = value;
            }
        }
    }
    let (confirm, advance, cancel, move_list, moves) = {
        let nodes = world.resource::<UiNodes>();
        (
            nodes.confirm,
            nodes.advance,
            nodes.cancel,
            nodes.move_list,
            nodes.moves.clone(),
        )
    };
    disabled(world, confirm, !view.can_confirm);
    disabled(world, cancel, !view.can_confirm);
    disabled(world, move_list, !view.can_select);
    disabled(world, advance, !view.can_advance);
    for entity in moves {
        disabled(world, entity, !view.can_select);
    }
    let stage_height =
        (world.resource::<ResolvedUiMetrics>().logical_size.y - 520.0).clamp(220.0, 840.0);
    for mut node in world
        .query_filtered::<&mut Node, With<SceneStage>>()
        .iter_mut(world)
    {
        let next = Val::Px(stage_height);
        if node.height != next {
            node.height = next;
            node.min_height = next;
        }
    }
    for mut visibility in world
        .query_filtered::<&mut Visibility, With<Cursor>>()
        .iter_mut(world)
    {
        let next = if view.can_advance && !view.typing {
            Visibility::Visible
        } else {
            Visibility::Hidden
        };
        if *visibility != next {
            *visibility = next;
        }
    }
    // Use the same immutable view for world-space HUD text and fill.
    for mut text in world
        .query_filtered::<&mut Text2d, With<CarterHealth>>()
        .iter_mut(world)
    {
        let value = format!("CARTER {} / {}", view.opponent_hp, view.opponent_max);
        if text.0 != value {
            text.0 = value;
        }
    }
}

pub(super) fn position_scene(
    metrics: Res<ResolvedUiMetrics>,
    view: Res<CarterfightView>,
    stage: Query<(&ComputedNode, &UiGlobalTransform), With<SceneStage>>,
    mut portrait: Query<
        &mut Transform,
        (
            With<CarterSprite>,
            Without<CarterHealth>,
            Without<HealthFill>,
        ),
    >,
    mut health: Query<
        (&mut Transform, &mut TextFont),
        (
            With<CarterHealth>,
            Without<CarterSprite>,
            Without<HealthFill>,
        ),
    >,
    mut fill: Query<
        (&mut Transform, &mut Sprite),
        (
            With<HealthFill>,
            Without<CarterSprite>,
            Without<CarterHealth>,
        ),
    >,
) {
    let Ok((node, transform)) = stage.single() else {
        return;
    };
    let size = node.size() * node.inverse_scale_factor;
    let center = transform.translation * node.inverse_scale_factor;
    let portrait_size = (size.y - 85.0).min(size.x * 0.32).max(64.0);
    let position = Vec2::new(
        center.x + size.x * 0.25 - metrics.logical_size.x / 2.0,
        metrics.logical_size.y / 2.0 - center.y + 24.0,
    );
    for mut transform in &mut portrait {
        transform.translation = position.extend(-1.0);
        transform.scale = Vec3::splat(portrait_size / 256.0);
    }
    for (mut transform, mut font) in &mut health {
        transform.translation =
            Vec3::new(position.x, position.y - portrait_size / 2.0 - 34.0, -0.8);
        font.font_size = FontSize::Px(18.0 * metrics.content_scale);
    }
    for (mut transform, mut sprite) in &mut fill {
        transform.translation = Vec3::new(
            position.x - 100.0,
            position.y - portrait_size / 2.0 - 12.0,
            -0.9,
        );
        sprite.custom_size = Some(Vec2::new(
            200.0 * f32::from(view.opponent_hp) / f32::from(view.opponent_max.max(1)),
            10.0,
        ));
    }
}
