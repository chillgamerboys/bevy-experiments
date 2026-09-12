//! Replaceable native presentation over the domain-neutral inspection lifecycle.

use super::*;
use crate::{UiFonts, UiSkin, UiTextRole};
use bevy::input_focus::{tab_navigation::TabGroup, FocusCause};

#[derive(Component, Clone)]
pub(super) enum TooltipAction {
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
    pinned: bool,
}

fn label(world: &mut World, parent: Entity, name: &str, value: String, role: UiTextRole) -> Entity {
    let fonts = world.resource::<UiFonts>().clone();
    world
        .spawn((
            crate::text(&fonts, role, value),
            Name::new(name.to_owned()),
            UiSkin::Text,
            bevy::ui::FocusPolicy::Pass,
            Node {
                width: Val::Percent(100.0),
                flex_shrink: 0.0,
                ..default()
            },
            ChildOf(parent),
        ))
        .id()
}

fn action(world: &mut World, parent: Entity, title: &str, action: TooltipAction) -> Entity {
    let role = if matches!(action, TooltipAction::Close(_)) {
        UiTextRole::Title
    } else {
        UiTextRole::Body
    };
    let entity = world
        .spawn((
            crate::button(format!("Tooltip {title}")),
            UiSkin::Control,
            action,
            ChildOf(parent),
        ))
        .id();
    world.entity_mut(entity).insert(action_node());
    label(
        world,
        entity,
        "Tooltip Control Label",
        title.to_owned(),
        role,
    );
    entity
}

// Identical content geometry for inert related terms and their locked buttons.
// Changing interactivity must not change measurement or floating placement.
fn action_node() -> Node {
    Node {
        min_width: Val::Px(44.0),
        min_height: Val::Px(44.0),
        padding: UiRect::axes(Val::Px(8.0), Val::Px(4.0)),
        justify_content: JustifyContent::Center,
        align_items: AlignItems::Center,
        flex_shrink: 0.0,
        ..default()
    }
}

pub(super) fn render(world: &mut World) {
    let host = world
        .query_filtered::<Entity, With<UiTooltipHost>>()
        .iter(world)
        .min();
    let state = world.resource::<UiTooltipState>();
    let keyboard = state.keyboard;
    let pinned = state.pinned;
    let wanted = state
        .chain
        .iter()
        .map_while(|key| content(world, state, key).map(|value| (key.clone(), value)))
        .collect::<Vec<_>>();
    world.resource_scope(|world, mut view: Mut<TooltipView>| {
        if view.host != host
            || view.rendered != wanted
            || view.pinned != pinned
            || view
                .root
                .is_some_and(|entity| world.get_entity(entity).is_err())
        {
            if let Some(root) = view.root.take() {
                let _ = world.despawn(root);
            }
            view.cards.clear();
            view.focus = None;
            view.rendered.clone_from(&wanted);
            view.host = host;
            view.pinned = pinned;
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
                for (depth, (_, content)) in wanted.iter().enumerate() {
                    let card = world
                        .spawn((
                            Name::new(format!("Tooltip Card {depth}")),
                            Node {
                                position_type: PositionType::Absolute,
                                left: Val::Px(0.0),
                                top: Val::Px(0.0),
                                width: Val::Px(340.0),
                                max_width: Val::Percent(92.0),
                                flex_direction: FlexDirection::Column,
                                padding: UiRect::all(Val::Px(12.0)),
                                row_gap: Val::Px(6.0),
                                overflow: Overflow::scroll_y(),
                                border: UiRect::all(Val::Px(2.0)),
                                ..default()
                            },
                            UiSkin::Panel,
                            GlobalZIndex(80 + i32::try_from(depth).unwrap_or(0)),
                            TooltipSurface,
                            Visibility::Inherited,
                            if pinned {
                                bevy::ui::FocusPolicy::Block
                            } else {
                                bevy::ui::FocusPolicy::Pass
                            },
                            Pickable {
                                should_block_lower: pinned,
                                is_hoverable: pinned,
                            },
                            ChildOf(root),
                        ))
                        .id();
                    if pinned {
                        world.entity_mut(card).insert(Interaction::None);
                    }
                    let heading = world
                        .spawn((
                            Node {
                                min_height: Val::Px(44.0),
                                padding: UiRect::right(Val::Px(44.0)),
                                flex_shrink: 0.0,
                                align_items: AlignItems::Center,
                                ..default()
                            },
                            Pickable::IGNORE,
                            bevy::ui::FocusPolicy::Pass,
                            ChildOf(card),
                        ))
                        .id();
                    label(
                        world,
                        heading,
                        "Tooltip Title",
                        content.title.clone(),
                        UiTextRole::Title,
                    );
                    if pinned {
                        // ASCII remains visible with the default Bevy font as well as game fonts.
                        let close = action(world, heading, "x", TooltipAction::Close(depth));
                        world.entity_mut(close).insert((
                            Name::new("Tooltip Close"),
                            AccessibleLabel::new("Close tooltip"),
                            Node {
                                position_type: PositionType::Absolute,
                                right: Val::Px(0.0),
                                top: Val::Px(0.0),
                                width: Val::Px(44.0),
                                height: Val::Px(44.0),
                                min_width: Val::Px(44.0),
                                min_height: Val::Px(44.0),
                                justify_content: JustifyContent::Center,
                                align_items: AlignItems::Center,
                                ..default()
                            },
                            crate::UiSkinOverrides {
                                background: Some(Color::NONE),
                                border: Some(Color::NONE),
                                ..default()
                            },
                        ));
                        if let Some(children) = world.get::<Children>(close) {
                            let labels = children.to_vec();
                            for child in labels {
                                world.entity_mut(child).insert(Node::default());
                            }
                        }
                        view.focus = Some(close);
                    }
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
                            if pinned {
                                action(
                                    world,
                                    card,
                                    &format!("{} ›", link.label),
                                    TooltipAction::Link(depth, link.subject.clone()),
                                );
                            } else {
                                let row = world
                                    .spawn((
                                        action_node(),
                                        Pickable::IGNORE,
                                        bevy::ui::FocusPolicy::Pass,
                                        ChildOf(card),
                                    ))
                                    .id();
                                label(
                                    world,
                                    row,
                                    "Tooltip Related Term",
                                    format!("{} ›", link.label),
                                    UiTextRole::Body,
                                );
                            }
                        }
                    }
                    view.cards.push(card);
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
    let border = {
        let theme = world.resource::<crate::UiTheme>();
        if pinned {
            theme.accent
        } else {
            theme.edge
        }
    };
    let cards = world
        .query_filtered::<Entity, With<TooltipSurface>>()
        .iter(world)
        .collect::<Vec<_>>();
    for entity in cards {
        let value = crate::UiSkinOverrides {
            border: Some(border),
            ..default()
        };
        if world.get::<crate::UiSkinOverrides>(entity) != Some(&value) {
            world.entity_mut(entity).insert(value);
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

/// Prefer above/below, then either side. Minimize overlap with the source after
/// clamping, so a tall character hit region cannot spawn a card under its cursor.
fn placement(anchor: Rect, size: Vec2, viewport: Vec2, avoid: &[Rect]) -> Vec2 {
    let margin = 8.0;
    let centered_x = anchor.center().x - size.x * 0.5;
    let centered_y = anchor.center().y - size.y * 0.5;
    let mut candidates = vec![
        Vec2::new(centered_x, anchor.min.y - size.y - margin),
        Vec2::new(centered_x, anchor.max.y + margin),
        Vec2::new(anchor.max.x + margin, centered_y),
        Vec2::new(anchor.min.x - size.x - margin, centered_y),
    ];
    for region in avoid {
        candidates.extend([
            Vec2::new(region.min.x - size.x - margin, centered_y),
            Vec2::new(region.max.x + margin, centered_y),
            Vec2::new(centered_x, region.min.y - size.y - margin),
            Vec2::new(centered_x, region.max.y + margin),
        ]);
    }
    let mut best = Vec2::splat(margin);
    let mut overlap = (f32::INFINITY, f32::INFINITY);
    for candidate in candidates {
        let position = candidate.clamp(
            Vec2::splat(margin),
            (viewport - size - Vec2::splat(margin)).max(Vec2::splat(margin)),
        );
        let intersection = Rect::from_corners(position, position + size).intersect(anchor);
        let area = if intersection.is_empty() {
            0.0
        } else {
            intersection.width() * intersection.height()
        };
        let blocked = avoid
            .iter()
            .map(|region| {
                let intersection = Rect::from_corners(position, position + size).intersect(*region);
                if intersection.is_empty() {
                    0.0
                } else {
                    intersection.width() * intersection.height()
                }
            })
            .sum::<f32>();
        if (blocked, area) < overlap {
            best = position;
            overlap = (blocked, area);
        }
    }
    best
}

fn safe_bounds(world: &World, host: Entity, viewport: Vec2) -> Rect {
    let full = Rect::from_corners(Vec2::ZERO, viewport);
    let requested = world
        .get::<UiTooltipBounds>(host)
        .map_or(full, |bounds| bounds.0)
        .intersect(full);
    if requested.is_empty() || !requested.min.is_finite() || !requested.max.is_finite() {
        full
    } else {
        requested
    }
}

/// Apply size constraints before native measurement, using this frame's render
/// target information rather than last frame's computed card geometry.
pub(super) fn constrain(world: &mut World) {
    let view = world.resource::<TooltipView>();
    let Some(host) = view.host else { return };
    let viewport = world
        .get::<ComputedUiRenderTargetInfo>(host)
        .map(ComputedUiRenderTargetInfo::logical_size)
        .filter(|size| size.min_element() > 0.0)
        .or_else(|| {
            world
                .get::<ComputedNode>(host)
                .map(|node| node.size() * node.inverse_scale_factor)
        });
    let Some(viewport) = viewport else { return };
    let bounds = safe_bounds(world, host, viewport);
    let cards = view.cards.clone();
    for entity in cards {
        if let Some(mut node) = world.get_mut::<Node>(entity) {
            let height = Val::Px((bounds.height() - 16.0).max(44.0));
            let width = Val::Px((bounds.width() - 16.0).max(44.0));
            if node.max_height != height {
                node.max_height = height;
            }
            if node.max_width != width {
                node.max_width = width;
            }
        }
    }
}

/// Floating cards do not participate in the screen's flow. Native layout owns
/// their measurement and internal geometry; this pass owns their final screen
/// translation. Move all descendants before PostLayout computes clipping, so
/// rendering, text, scrolling, and next frame's picking use the same geometry.
/// Never write Node offsets here: they are inputs to a future layout pass.
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
    let bounds = safe_bounds(world, host, viewport);
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
    let avoid = world
        .query_filtered::<(Entity, &ComputedNode, &UiGlobalTransform), With<UiTooltipAvoid>>()
        .iter(world)
        .filter_map(|(entity, node, transform)| {
            let mut ancestor = entity;
            while ancestor != host {
                ancestor = world.get::<ChildOf>(ancestor)?.parent();
            }
            if node.size().min_element() <= 0.0
                || world.get::<Visibility>(entity) == Some(&Visibility::Hidden)
            {
                return None;
            }
            Some(Rect::from_center_size(
                transform.translation * scale - origin - bounds.min,
                node.size() * scale,
            ))
        })
        .collect::<Vec<_>>();
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
                    &avoid,
                ) + bounds.min
            },
            |parent| {
                let x = if parent.max.x + 8.0 + size.x <= bounds.max.x - 8.0 {
                    parent.max.x + 8.0
                } else {
                    parent.min.x - size.x - 8.0
                };
                let preferred = Vec2::new(x, parent.min.y).clamp(
                    bounds.min + Vec2::splat(8.0),
                    (bounds.max - size - Vec2::splat(8.0)).max(bounds.min + Vec2::splat(8.0)),
                );
                if avoid.iter().any(|region| {
                    !Rect::from_corners(preferred - bounds.min, preferred - bounds.min + size)
                        .intersect(*region)
                        .is_empty()
                }) {
                    placement(
                        Rect::from_corners(parent.min - bounds.min, parent.max - bounds.min),
                        size,
                        bounds.size(),
                        &avoid,
                    ) + bounds.min
                } else {
                    preferred
                }
            },
        );
        let laid_out_position = world
            .get::<UiGlobalTransform>(entity)
            .map(|transform| transform.translation * scale - origin - size * 0.5);
        let invalid_geometry = size.min_element() <= 0.0
            || !size.is_finite()
            || !scale.is_finite()
            || scale <= 0.0
            || laid_out_position.is_none_or(|actual| !actual.is_finite());
        if !invalid_geometry {
            let delta = (position - laid_out_position.expect("validated geometry")) / scale;
            translate_subtree(world, entity, delta);
        }
        // This only guards unavailable/invalid geometry, never a timed wait or
        // a position mismatch. Valid measured cards are placed this frame.
        let visibility = if invalid_geometry {
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

fn translate_subtree(world: &mut World, root: Entity, delta: Vec2) {
    if delta == Vec2::ZERO {
        return;
    }
    let mut pending = vec![root];
    while let Some(entity) = pending.pop() {
        if let Some(children) = world.get::<Children>(entity) {
            pending.extend(children.iter());
        }
        if let Some(mut transform) = world.get_mut::<UiGlobalTransform>(entity) {
            let mut affine = transform.affine();
            affine.translation += delta;
            *transform = affine.into();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn protected_surfaces_take_priority_over_source_proximity() {
        let log = Rect::from_corners(Vec2::new(900.0, 100.0), Vec2::new(1270.0, 500.0));
        let anchor = Rect::from_corners(Vec2::new(1100.0, 600.0), Vec2::new(1200.0, 700.0));
        let size = Vec2::new(340.0, 300.0);
        let position = placement(anchor, size, Vec2::new(1280.0, 720.0), &[log]);
        let card = Rect::from_corners(position, position + size);
        assert!(card.intersect(log).is_empty());
        assert!(card.intersect(anchor).is_empty());
    }

    #[test]
    fn tall_sources_use_side_placement_without_occluding_their_hit_region() {
        let anchor = Rect::from_corners(Vec2::new(900.0, 100.0), Vec2::new(1100.0, 900.0));
        let size = Vec2::new(340.0, 300.0);
        let position = placement(anchor, size, Vec2::new(1280.0, 600.0), &[]);
        assert!(Rect::from_corners(position, position + size)
            .intersect(anchor)
            .is_empty());
    }
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
                    &[],
                );
                assert!(position.cmpge(Vec2::splat(8.0)).all());
                assert!((position + card).cmple(size - Vec2::splat(8.0)).all());
            }
        }
    }
}
