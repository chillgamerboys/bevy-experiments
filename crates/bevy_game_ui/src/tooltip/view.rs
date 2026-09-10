//! Replaceable native presentation over the domain-neutral inspection lifecycle.

use super::*;
use crate::{UiControlMetrics, UiFonts, UiSkin, UiTextRole};
use bevy::input_focus::{tab_navigation::TabGroup, FocusCause};

#[derive(Component, Clone)]
pub(super) enum TooltipAction {
    Pin,
    Close(usize),
    Link(usize, UiTooltipSubject),
}

#[derive(Component)]
pub(super) struct TooltipSurface;

#[derive(Resource, Default)]
pub(super) struct TooltipView {
    root: Option<Entity>,
    host: Option<Entity>,
    cards: Vec<Entity>,
    focus: Option<Entity>,
    rendered: Vec<(UiTooltipSubject, UiTooltipContent)>,
    keyboard: bool,
}

fn label(world: &mut World, parent: Entity, name: &str, value: String, role: UiTextRole) {
    let fonts = world.resource::<UiFonts>().clone();
    world.spawn((
        crate::text(&fonts, role, value),
        Name::new(name.to_owned()),
        UiSkin::Text,
        Node {
            width: Val::Percent(100.0),
            flex_shrink: 0.0,
            ..default()
        },
        ChildOf(parent),
    ));
}

fn action(world: &mut World, parent: Entity, title: &str, action: TooltipAction) -> Entity {
    let entity = world
        .spawn((
            crate::button(format!("Tooltip {title}")),
            UiSkin::Control,
            UiControlMetrics::default(),
            action,
            ChildOf(parent),
        ))
        .id();
    world.entity_mut(entity).insert(Node {
        min_width: Val::Px(44.0),
        min_height: Val::Px(44.0),
        padding: UiRect::axes(Val::Px(8.0), Val::Px(4.0)),
        justify_content: JustifyContent::Center,
        align_items: AlignItems::Center,
        flex_shrink: 0.0,
        ..default()
    });
    label(
        world,
        entity,
        "Tooltip Control Label",
        title.to_owned(),
        UiTextRole::Body,
    );
    entity
}

pub(super) fn render(world: &mut World) {
    let host = world
        .query_filtered::<Entity, With<UiTooltipHost>>()
        .iter(world)
        .min();
    let state = world.resource::<UiTooltipState>();
    let keyboard = state.keyboard;
    let wanted = state
        .chain
        .iter()
        .map_while(|key| content(world, state, key).map(|value| (key.clone(), value)))
        .collect::<Vec<_>>();
    world.resource_scope(|world, mut view: Mut<TooltipView>| {
        if view.host != host
            || view.rendered != wanted
            || view
                .root
                .is_some_and(|entity| world.get_entity(entity).is_err())
        {
            // Seed rebuilt cards from their previous layout. In particular,
            // opening a child must not blink an unchanged parent off/on.
            let previous_nodes = if view.host == host {
                view.rendered
                    .iter()
                    .zip(&view.cards)
                    .filter_map(|((key, _), entity)| {
                        world
                            .get::<Node>(*entity)
                            .map(|node| (key.clone(), node.clone()))
                    })
                    .collect::<BTreeMap<_, _>>()
            } else {
                BTreeMap::new()
            };
            if let Some(root) = view.root.take() {
                let _ = world.despawn(root);
            }
            view.cards.clear();
            view.focus = None;
            view.rendered.clone_from(&wanted);
            view.host = host;
            if let Some(host) = host.filter(|_| !wanted.is_empty()) {
                let root = world
                    .spawn((
                        Name::new("Tooltip Layer"),
                        Node {
                            position_type: PositionType::Absolute,
                            left: Val::Px(0.0),
                            top: Val::Px(0.0),
                            width: Val::Percent(100.0),
                            height: Val::Percent(100.0),
                            ..default()
                        },
                        GlobalZIndex(80),
                        bevy::ui::FocusPolicy::Pass,
                        Pickable::IGNORE,
                        ChildOf(host),
                    ))
                    .id();
                view.root = Some(root);
                for (depth, (subject, content)) in wanted.iter().enumerate() {
                    let previous = previous_nodes.get(subject);
                    let card = world
                        .spawn((
                            Name::new(format!("Tooltip Card {depth}")),
                            Node {
                                position_type: PositionType::Absolute,
                                left: previous.map_or(Val::Auto, |node| node.left),
                                top: previous.map_or(Val::Auto, |node| node.top),
                                width: Val::Px(340.0),
                                max_width: Val::Percent(92.0),
                                max_height: previous
                                    .map_or(Val::Percent(65.0), |node| node.max_height),
                                flex_direction: FlexDirection::Column,
                                padding: UiRect::all(Val::Px(12.0)),
                                row_gap: Val::Px(6.0),
                                overflow: Overflow::scroll_y(),
                                border: UiRect::all(Val::Px(1.0)),
                                ..default()
                            },
                            UiSkin::Panel,
                            GlobalZIndex(80 + i32::try_from(depth).unwrap_or(0)),
                            TooltipSurface,
                            // Layout must measure the card before we can place
                            // it. Hidden retains layout without flashing at (0,0).
                            Visibility::Hidden,
                            Interaction::None,
                            bevy::ui::FocusPolicy::Block,
                            Pickable {
                                should_block_lower: true,
                                is_hoverable: true,
                            },
                            ChildOf(root),
                        ))
                        .id();
                    label(
                        world,
                        card,
                        "Tooltip Title",
                        content.title.clone(),
                        UiTextRole::Title,
                    );
                    for fact in &content.facts {
                        label(world, card, "Tooltip Fact", fact.clone(), UiTextRole::Body);
                    }
                    if !content.body.trim().is_empty()
                        && content.body.trim() != content.title.trim()
                    {
                        label(
                            world,
                            card,
                            "Tooltip Description",
                            content.body.clone(),
                            UiTextRole::Body,
                        );
                    }
                    for link in &content.links {
                        if world
                            .resource::<UiTooltipCatalog>()
                            .0
                            .contains_key(&link.subject)
                        {
                            action(
                                world,
                                card,
                                &format!("{} ›", link.label),
                                TooltipAction::Link(depth, link.subject.clone()),
                            );
                        }
                    }
                    let controls = world
                        .spawn((
                            Node {
                                column_gap: Val::Px(6.0),
                                flex_shrink: 0.0,
                                ..default()
                            },
                            ChildOf(card),
                        ))
                        .id();
                    let pin = action(world, controls, "Pin · T", TooltipAction::Pin);
                    action(world, controls, "Close", TooltipAction::Close(depth));
                    view.cards.push(card);
                    view.focus = Some(pin);
                }
            }
        }
        if let Some(root) = view.root {
            if let Some(anchor) = world.resource::<UiTooltipState>().anchor {
                world.entity_mut(root).insert(TooltipOrigin(anchor));
            }
            if keyboard {
                world.entity_mut(root).insert(TabGroup::modal());
            } else {
                world.entity_mut(root).remove::<TabGroup>();
            }
            let focus_inside =
                world
                    .resource::<InputFocus>()
                    .get()
                    .is_some_and(|mut entity| loop {
                        if entity == root {
                            return true;
                        }
                        let Some(parent) = world.get::<ChildOf>(entity) else {
                            return false;
                        };
                        entity = parent.parent();
                    });
            if keyboard && (!view.keyboard || !focus_inside) {
                if let Some(entity) = view.focus {
                    world
                        .resource_mut::<InputFocus>()
                        .set(entity, FocusCause::Navigated);
                }
            }
        }
        view.keyboard = keyboard;
    });
    let pinned = world.resource::<UiTooltipState>().is_pinned();
    let pins = world
        .query::<(Entity, &TooltipAction)>()
        .iter(world)
        .filter_map(|(entity, action)| matches!(action, TooltipAction::Pin).then_some(entity))
        .collect::<Vec<_>>();
    for entity in pins {
        let title = if pinned { "Unpin" } else { "Pin" };
        let children = world
            .get::<Children>(entity)
            .map(|children| children.to_vec())
            .unwrap_or_default();
        for child in children {
            if let Some(mut text) = world.get_mut::<Text>(child) {
                if text.0 != title {
                    text.0 = title.to_owned();
                }
            }
        }
        if world
            .get::<AccessibleLabel>(entity)
            .is_none_or(|label| label.0 != title)
        {
            world.entity_mut(entity).insert(AccessibleLabel::new(title));
        }
    }
}

pub(super) fn scroll(world: &mut World, direction: i8) {
    let Some(entity) = world.resource::<TooltipView>().cards.last().copied() else {
        return;
    };
    let Some(node) = world.get::<ComputedNode>(entity) else {
        return;
    };
    let height = node.size().y * node.inverse_scale_factor;
    let maximum = ((node.content_size().y - node.size().y) * node.inverse_scale_factor).max(0.0);
    let current = world
        .get::<ScrollPosition>(entity)
        .map_or(0.0, |position| position.0.y);
    let next = match direction {
        100.. => maximum,
        ..=-100 => 0.0,
        _ => (current + f32::from(direction) * height * 0.85).clamp(0.0, maximum),
    };
    world
        .entity_mut(entity)
        .insert(ScrollPosition(Vec2::new(0.0, next)));
}

/// Prefer above the source; then below, with every edge clamped to the viewport.
fn placement(anchor: Rect, size: Vec2, viewport: Vec2) -> Vec2 {
    let margin = 8.0;
    let preferred_y = if anchor.min.y >= size.y + margin * 2.0 {
        anchor.min.y - size.y - margin
    } else {
        anchor.max.y + margin
    };
    Vec2::new(anchor.center().x - size.x * 0.5, preferred_y).clamp(
        Vec2::splat(margin),
        (viewport - size - Vec2::splat(margin)).max(Vec2::splat(margin)),
    )
}

pub(super) fn place(world: &mut World) {
    let view = world.resource::<TooltipView>();
    let Some(host) = view.host else {
        return;
    };
    let Some(computed) = world.get::<ComputedNode>(host) else {
        return;
    };
    let scale = computed.inverse_scale_factor;
    let viewport = computed.size() * scale;
    let full = Rect::from_corners(Vec2::ZERO, viewport);
    let requested = world
        .get::<UiTooltipBounds>(host)
        .map_or(full, |bounds| bounds.0)
        .intersect(full);
    let bounds = if requested.is_empty() || !requested.min.is_finite() || !requested.max.is_finite()
    {
        full
    } else {
        requested
    };
    let origin = world
        .get::<UiGlobalTransform>(host)
        .map_or(Vec2::ZERO, |transform| {
            transform.translation * scale - viewport * 0.5
        });
    let anchor = world
        .resource::<UiTooltipState>()
        .anchor
        .and_then(|entity| {
            let node = world.get::<ComputedNode>(entity)?;
            let transform = world.get::<UiGlobalTransform>(entity)?;
            Some(Rect::from_center_size(
                transform.translation * scale - origin,
                node.size() * scale,
            ))
        })
        .unwrap_or_else(|| Rect::from_center_size(viewport * Vec2::new(0.5, 0.8), Vec2::ZERO));
    let cards = view.cards.clone();
    let mut previous = None::<Rect>;
    for entity in cards {
        let Some(computed) = world.get::<ComputedNode>(entity) else {
            continue;
        };
        let size = computed.size() * scale;
        let position = previous.map_or_else(
            || {
                placement(
                    Rect::from_corners(anchor.min - bounds.min, anchor.max - bounds.min),
                    size,
                    bounds.size(),
                ) + bounds.min
            },
            |parent| {
                let x = if parent.max.x + 8.0 + size.x <= bounds.max.x - 8.0 {
                    parent.max.x + 8.0
                } else {
                    parent.min.x - size.x - 8.0
                };
                Vec2::new(x, parent.min.y).clamp(
                    bounds.min + Vec2::splat(8.0),
                    (bounds.max - size - Vec2::splat(8.0)).max(bounds.min + Vec2::splat(8.0)),
                )
            },
        );
        let laid_out_position = world
            .get::<UiGlobalTransform>(entity)
            .map(|transform| transform.translation * scale - origin - size * 0.5);
        let mut pending_layout = size.min_element() <= 0.0
            || !size.is_finite()
            // Layout rounds physical edges. Compare per axis with one physical
            // pixel of tolerance (converted to logical units), not Euclidean
            // half-pixel distance, which can leave valid cards hidden forever.
            || laid_out_position.is_none_or(|actual| {
                (actual - position).abs().max_element() > scale.max(f32::EPSILON)
            });
        if let Some(mut node) = world.get_mut::<Node>(entity) {
            let maximum = Val::Px((bounds.height() - 16.0).max(44.0));
            if node.max_height != maximum {
                node.max_height = maximum;
                pending_layout = true;
            }
            if node.left != Val::Px(position.x) || node.top != Val::Px(position.y) {
                node.left = Val::Px(position.x);
                node.top = Val::Px(position.y);
            }
        }
        // Node offsets are inputs to NEXT frame's layout, not a transform for
        // this frame. Reveal only when computed geometry has consumed them.
        // This also guards content resizing and viewport/bounds changes.
        let visibility = if pending_layout {
            Visibility::Hidden
        } else {
            Visibility::Inherited
        };
        if world.get::<Visibility>(entity) != Some(&visibility) {
            world.entity_mut(entity).insert(visibility);
        }
        previous = Some(Rect::from_corners(position, position + size));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn placement_keeps_cards_inside_every_viewport_edge() {
        for size in [Vec2::new(1280.0, 720.0), Vec2::new(1920.0, 1080.0)] {
            for anchor in [
                Vec2::ZERO,
                size,
                Vec2::new(size.x, 0.0),
                Vec2::new(0.0, size.y),
            ] {
                let card = Vec2::new(340.0, 320.0);
                let position = placement(
                    Rect::from_center_size(anchor, Vec2::splat(44.0)),
                    card,
                    size,
                );
                assert!(position.cmpge(Vec2::splat(8.0)).all());
                assert!((position + card).cmple(size - Vec2::splat(8.0)).all());
            }
        }
    }
}
