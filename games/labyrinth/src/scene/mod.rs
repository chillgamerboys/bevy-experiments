//! Labyrinth's rendered battlefield, anchored to native UI hit regions.
//!
//! This layer owns only sprites and atmosphere. The UI owns all input, focus,
//! selection and modal blocking; the host owns every combat transition. There is
//! deliberately no animation queue or replay of historical reconnect outcomes.

mod appearance;
mod geometry;
#[cfg(test)]
mod tests;

use std::collections::BTreeMap;

use bevy::input_focus::{InputFocus, InputFocusVisible};
use bevy::prelude::*;
use labyrinth_rules::{ActorId, ActorSnapshot};

use crate::view::{LabyrinthView, ViewMode};
use appearance::fallback_color;
pub(crate) use appearance::SceneAppearance;
use geometry::{anchor_frame, floor_aligned_rect, AnchorFrame};

/// Reuse the world-art identity in native HUD portraits; no duplicate portrait atlas.
pub(crate) fn portrait_image(world: &World, kind: labyrinth_rules::ActorKind) -> ImageNode {
    let Some(appearance) = world.get_resource::<SceneAppearance>() else {
        return ImageNode::solid_color(fallback_color(kind));
    };
    let Some(handle) = appearance.actor_sheet.as_ref() else {
        return ImageNode::solid_color(fallback_color(kind));
    };
    let Some(image) = world
        .get_resource::<Assets<Image>>()
        .and_then(|assets| assets.get(handle))
    else {
        return ImageNode::solid_color(fallback_color(kind));
    };
    let Some(cell) = appearance.actor_rect(image.size(), kind) else {
        return ImageNode::solid_color(fallback_color(kind));
    };
    // Square upper-body crop. This is artwork presentation, not actor/rank identity.
    let side = cell.width() * 0.78;
    let left = cell.min.x + (cell.width() - side) * 0.5;
    ImageNode {
        rect: Some(Rect::from_corners(
            Vec2::new(left, cell.min.y + cell.height() * 0.04),
            Vec2::new(left + side, cell.min.y + cell.height() * 0.04 + side),
        )),
        ..ImageNode::new(handle.clone())
    }
}

/// The native, transparent actor button's art-only rectangle.
#[derive(Component)]
pub(crate) struct SceneActorAnchor {
    pub actor: ActorId,
}

/// Non-interactive layout space from which artwork and its tight hit region are derived.
#[derive(Component)]
pub(crate) struct SceneActorLayout(pub Entity);

/// Local targeting emphasis, not permission to issue a combat command.
#[derive(Component, Default)]
pub(crate) struct SceneActorEmphasis {
    pub selected: bool,
}

/// The open battlefield rectangle, excluding the fixed HUD and action rail.
#[derive(Component)]
pub(crate) struct SceneStageAnchor;

/// Same atlas fit as the renderer, expressed in UI logical dimensions.
pub(crate) fn actor_art_size(
    world: &World,
    kind: labyrinth_rules::ActorKind,
    area: Vec2,
) -> Option<Vec2> {
    let appearance = world.get_resource::<SceneAppearance>()?;
    let handle = appearance.actor_sheet.as_ref()?;
    let image = world.get_resource::<Assets<Image>>()?.get(handle)?;
    let cell = appearance.actor_rect(image.size(), kind)?;
    Some(fit_actor_art(area, cell.size()))
}

fn fit_actor_art(area: Vec2, cell: Vec2) -> Vec2 {
    cell * ((area * Vec2::new(0.94, 0.93)) / cell).min_element()
}

/// Derive input geometry from the same measured column and atlas fit as rendering.
/// These transparent controls have no children: position/size are finalized before
/// clipping and picking, without modifying next frame's layout or sprite anchors.
fn fit_actor_hit_regions(world: &mut World) {
    let Some(snapshot) = world
        .get_resource::<LabyrinthView>()
        .and_then(|view| view.combat.clone())
    else {
        return;
    };
    let regions = world
        .query::<(Entity, &SceneActorAnchor, &SceneActorLayout)>()
        .iter(world)
        .filter_map(|(entity, anchor, layout)| {
            let actor = snapshot.actor(anchor.actor)?;
            let node = world.get::<ComputedNode>(layout.0)?;
            let transform = world.get::<UiGlobalTransform>(layout.0)?;
            let inverse = node.inverse_scale_factor;
            let area = node.size() * inverse;
            if area.min_element() <= 0.0 || !area.is_finite() {
                return None;
            }
            let (size, center_y) = actor_art_size(world, actor.kind, area)
                .map_or((area * Vec2::new(0.48, 0.785), area.y * 0.0425), |art| {
                    (art, area.y * 0.46 - art.y * 0.5)
                });
            let size = size.max(Vec2::splat(44.0)).min(area);
            let mut affine = transform.affine();
            affine.translation = transform.transform_point2(Vec2::new(0.0, center_y / inverse));
            Some((
                entity,
                size / inverse,
                inverse,
                UiGlobalTransform::from(affine),
            ))
        })
        .collect::<Vec<_>>();
    for (entity, size, inverse, transform) in regions {
        if let Some(mut node) = world.get_mut::<ComputedNode>(entity) {
            node.size = size;
            node.unrounded_size = size;
            node.inverse_scale_factor = inverse;
        }
        world.entity_mut(entity).insert(transform);
    }
}

pub(crate) struct LabyrinthScenePlugin;

impl Plugin for LabyrinthScenePlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<SceneAppearance>()
            .init_resource::<SceneState>()
            .add_systems(
                PostUpdate,
                fit_actor_hit_regions
                    .after(bevy::ui::UiSystems::Layout)
                    .before(bevy_gamekit::ui::UiTooltipSystems::Place)
                    .before(bevy::ui::UiSystems::PostLayout),
            )
            .add_systems(
                PostUpdate,
                present
                    .after(bevy::ui::UiSystems::PostLayout)
                    .after(bevy::transform::TransformSystems::Propagate)
                    .before(bevy::camera::visibility::VisibilitySystems::CalculateBounds)
                    .before(bevy::camera::visibility::VisibilitySystems::VisibilityPropagate),
            );
        if app
            .world()
            .contains_resource::<bevy::asset::io::embedded::EmbeddedAssetRegistry>()
            && app.world().contains_resource::<AssetServer>()
        {
            bevy::asset::embedded_asset!(app, "assets/actors.png");
            bevy::asset::embedded_asset!(app, "assets/room.png");
            if app
                .world()
                .resource::<SceneAppearance>()
                .actor_sheet
                .is_none()
            {
                let handle = bevy::asset::load_embedded_asset!(app, "assets/actors.png");
                app.world_mut()
                    .resource_mut::<SceneAppearance>()
                    .actor_sheet = Some(handle);
            }
            if app
                .world()
                .resource::<SceneAppearance>()
                .backdrop_image
                .is_none()
            {
                let handle = bevy::asset::load_embedded_asset!(app, "assets/room.png");
                app.world_mut()
                    .resource_mut::<SceneAppearance>()
                    .backdrop_image = Some(handle);
            }
        }
    }
}

#[derive(Component)]
struct SceneOwned;

#[derive(Component)]
struct ActorBody;

#[derive(Clone, Copy)]
struct ActorNodes {
    body: Entity,
    head: Entity,
    eyes: Entity,
    ground: Entity,
}

#[derive(Resource, Default)]
struct SceneState {
    actors: BTreeMap<ActorId, ActorNodes>,
    stage: Vec<Entity>,
}

struct ActorAnchor {
    actor: ActorId,
    frame: AnchorFrame,
    selected: bool,
    focused: bool,
    hovered: bool,
}

fn frame_for(world: &World, entity: Entity) -> Option<AnchorFrame> {
    // Inspect current hierarchy state, not the previous frame's inherited
    // visibility: this system deliberately precedes visibility propagation.
    let mut ancestor = Some(entity);
    while let Some(current) = ancestor {
        if world
            .get::<Node>(current)
            .is_some_and(|node| node.display == Display::None)
            || world.get::<Visibility>(current) == Some(&Visibility::Hidden)
        {
            return None;
        }
        ancestor = world.get::<ChildOf>(current).map(ChildOf::parent);
    }
    let camera = world.get::<ComputedUiTargetCamera>(entity)?.get()?;
    anchor_frame(
        world.get::<ComputedNode>(entity)?,
        world.get::<UiGlobalTransform>(entity)?,
        world.get::<Camera>(camera)?,
        world.get::<GlobalTransform>(camera)?,
    )
}

fn primitive(world: &mut World, name: String) -> Entity {
    world
        .spawn((
            Name::new(name),
            SceneOwned,
            Sprite::from_color(Color::WHITE, Vec2::ONE),
            Visibility::Hidden,
        ))
        .id()
}

fn actor_nodes(world: &mut World, actor: ActorId) -> ActorNodes {
    let body = primitive(world, format!("Scene actor {}", actor.0));
    world.entity_mut(body).insert(ActorBody);
    ActorNodes {
        body,
        head: primitive(world, format!("Scene actor {} fallback head", actor.0)),
        eyes: primitive(world, format!("Scene actor {} fallback eyes", actor.0)),
        ground: primitive(world, format!("Scene actor {} ground mark", actor.0)),
    }
}

fn paint(world: &mut World, entity: Entity, sprite: Sprite, transform: Transform) {
    // These are unparented world entities. Update the matching global transform
    // directly because UI layout runs after normal transform propagation.
    // Sprite has no PartialEq in Bevy 0.19. Compare the fields this renderer owns
    // rather than dirtying bounds/material extraction on every unchanged frame.
    let unchanged = world.get::<Sprite>(entity).is_some_and(|previous| {
        previous.image == sprite.image
            && previous.rect == sprite.rect
            && previous.custom_size == sprite.custom_size
            && previous.color == sprite.color
    });
    if !unchanged {
        world.entity_mut(entity).insert(sprite);
    }
    if let Some(mut value) = world.get_mut::<Transform>(entity) {
        value.set_if_neq(transform);
    }
    if let Some(mut value) = world.get_mut::<GlobalTransform>(entity) {
        value.set_if_neq(GlobalTransform::from(transform));
    }
    if let Some(mut value) = world.get_mut::<Visibility>(entity) {
        value.set_if_neq(Visibility::Visible);
    }
}

fn color_rect(
    world: &mut World,
    entity: Entity,
    frame: AnchorFrame,
    center: Vec2,
    size: Vec2,
    depth: f32,
    color: Color,
) {
    paint(
        world,
        entity,
        Sprite::from_color(color, frame.size() * size),
        frame.transform(center.x, center.y, depth),
    );
}

fn hide(world: &mut World, entity: Entity) {
    if let Some(mut visible) = world.get_mut::<Visibility>(entity) {
        visible.set_if_neq(Visibility::Hidden);
    }
}

fn present(world: &mut World) {
    let Some(view) = world.get_resource::<LabyrinthView>() else {
        return;
    };
    if view.mode != ViewMode::Combat || view.combat.is_none() {
        let entities: Vec<_> = world
            .query_filtered::<Entity, With<SceneOwned>>()
            .iter(world)
            .collect();
        for entity in entities {
            world.despawn(entity);
        }
        *world.resource_mut::<SceneState>() = SceneState::default();
        return;
    }
    let Some(snapshot) = view.combat.clone() else {
        return;
    };
    let appearance = world.resource::<SceneAppearance>().clone();
    let focus = world.get_resource::<InputFocus>().and_then(InputFocus::get);
    let visible_focus = world
        .get_resource::<InputFocusVisible>()
        .is_some_and(|visible| visible.0);
    let anchors: Vec<_> = world
        .query::<(Entity, &SceneActorAnchor)>()
        .iter(world)
        .filter_map(|(entity, anchor)| {
            Some(ActorAnchor {
                actor: anchor.actor,
                frame: frame_for(
                    world,
                    world
                        .get::<SceneActorLayout>(entity)
                        .map_or(entity, |layout| layout.0),
                )?,
                selected: world
                    .get::<SceneActorEmphasis>(entity)
                    .is_some_and(|state| state.selected),
                focused: visible_focus && focus == Some(entity),
                hovered: world.get::<Interaction>(entity) == Some(&Interaction::Hovered),
            })
        })
        .collect();
    let stage = world
        .query_filtered::<Entity, With<SceneStageAnchor>>()
        .iter(world)
        .find_map(|entity| frame_for(world, entity));
    let sheet = appearance.actor_sheet.as_ref().and_then(|handle| {
        world
            .get_resource::<Assets<Image>>()?
            .get(handle)
            .map(|image| (handle.clone(), image.size()))
    });
    let backdrop = appearance.backdrop_image.as_ref().and_then(|handle| {
        world
            .get_resource::<Assets<Image>>()?
            .get(handle)
            .map(|image| (handle, image.size()))
    });
    world.resource_scope(|world, mut state: Mut<SceneState>| {
        state.actors.retain(|actor, nodes| {
            if snapshot.actor(*actor).is_none() {
                for entity in [nodes.body, nodes.head, nodes.eyes, nodes.ground] {
                    world.despawn(entity);
                }
                return false;
            }
            if !anchors.iter().any(|anchor| anchor.actor == *actor) {
                for entity in [nodes.body, nodes.head, nodes.eyes, nodes.ground] {
                    hide(world, entity);
                }
            }
            true
        });
        if let Some(stage) = stage {
            let floor = anchors
                .iter()
                .filter_map(|anchor| {
                    let foot = anchor.frame.point(0.0, -0.46) - stage.center;
                    let fraction = 0.5 - foot.dot(stage.up) / stage.up.length_squared();
                    (fraction.is_finite() && (0.0..1.0).contains(&fraction)).then_some(fraction)
                })
                .max_by(f32::total_cmp)
                .unwrap_or(0.66)
                .clamp(0.05, 0.95);
            paint_stage(world, &mut state.stage, stage, &appearance, backdrop, floor);
        } else {
            for entity in &state.stage {
                hide(world, *entity);
            }
        }
        for anchor in anchors {
            let Some(actor) = snapshot.actor(anchor.actor) else {
                continue;
            };
            let nodes = *state
                .actors
                .entry(actor.id)
                .or_insert_with(|| actor_nodes(world, actor.id));
            paint_actor(
                world,
                nodes,
                &anchor,
                actor,
                snapshot.active_actor == Some(actor.id),
                &appearance,
                sheet.as_ref(),
            );
        }
    });
}

fn paint_stage(
    world: &mut World,
    nodes: &mut Vec<Entity>,
    frame: AnchorFrame,
    appearance: &SceneAppearance,
    backdrop: Option<(&Handle<Image>, UVec2)>,
    floor: f32,
) {
    const COUNT: usize = 13;
    while nodes.len() < COUNT {
        nodes.push(primitive(
            world,
            format!("Labyrinth atmosphere {}", nodes.len()),
        ));
    }
    if let Some((image, source)) = backdrop {
        if let Some((plate, fallbacks)) = nodes.split_first() {
            let mut sprite = Sprite::from_image(image.clone());
            // The decorative plate fills the native stage rectangle. Actor/rank
            // alignment never depends on its pixels or aspect ratio.
            sprite.custom_size = Some(frame.size());
            sprite.rect = floor_aligned_rect(source, appearance.backdrop_floor, floor);
            paint(world, *plate, sprite, frame.transform(0.0, 0.0, -50.0));
            for entity in fallbacks {
                hide(world, *entity);
            }
        }
        return;
    }
    let mut next = nodes.iter().copied();
    let mut rect = |center, size, depth, color| {
        if let Some(entity) = next.next() {
            color_rect(world, entity, frame, center, size, depth, color);
        }
    };
    rect(Vec2::ZERO, Vec2::ONE, -50.0, appearance.backdrop);
    rect(
        Vec2::new(0.0, -floor * 0.5),
        Vec2::new(1.0, 1.0 - floor),
        -40.0,
        appearance.ground,
    );
    // Subdued architecture and horizontal haze create depth without requiring
    // external assets, post-processing, particles, or nonessential motion.
    for index in 0..7 {
        rect(
            Vec2::new(-0.48 + index as f32 * 0.16, 0.12),
            Vec2::new(0.024, 0.65),
            -48.0,
            Color::srgb(0.042, 0.048, 0.047),
        );
    }
    for index in 0..4 {
        rect(
            Vec2::new(0.0, -0.25 + index as f32 * 0.12),
            Vec2::new(1.0, 0.07),
            -44.0,
            appearance.mist,
        );
    }
}

fn paint_actor(
    world: &mut World,
    nodes: ActorNodes,
    anchor: &ActorAnchor,
    actor: &ActorSnapshot,
    active: bool,
    appearance: &SceneAppearance,
    sheet: Option<&(Handle<Image>, UVec2)>,
) {
    let frame = anchor.frame;
    let emphasis = if anchor.selected || anchor.focused {
        appearance.selected
    } else if active {
        appearance.active
    } else if anchor.hovered {
        Color::srgb(0.62, 0.63, 0.51)
    } else {
        Color::srgba(0.12, 0.13, 0.11, 0.8)
    };
    color_rect(
        world,
        nodes.ground,
        frame,
        Vec2::new(0.0, -0.47),
        Vec2::new(0.80, 0.018),
        -2.0,
        emphasis,
    );
    let tint = if actor.standing() {
        Color::WHITE
    } else {
        Color::srgba(0.35, 0.35, 0.35, 0.8)
    };
    if let Some((handle, rect)) = sheet.and_then(|(handle, size)| {
        appearance
            .actor_rect(*size, actor.kind)
            .map(|rect| (handle, rect))
    }) {
        hide(world, nodes.head);
        hide(world, nodes.eyes);
        let size = fit_actor_art(frame.size(), rect.size());
        let height = size.y / frame.size().y;
        let mut sprite = Sprite::from_image(handle.clone());
        sprite.rect = Some(rect);
        sprite.custom_size = Some(size);
        sprite.color = tint;
        paint(
            world,
            nodes.body,
            sprite,
            frame.transform(0.0, -0.46 + height * 0.5, 0.0),
        );
    } else {
        let color = if actor.standing() {
            fallback_color(actor.kind)
        } else {
            Color::srgb(0.16, 0.16, 0.15)
        };
        color_rect(
            world,
            nodes.body,
            frame,
            Vec2::new(0.0, -0.16),
            Vec2::new(0.48, 0.55),
            0.0,
            color,
        );
        color_rect(
            world,
            nodes.head,
            frame,
            Vec2::new(0.0, 0.21),
            Vec2::new(0.30, 0.28),
            0.1,
            color,
        );
        color_rect(
            world,
            nodes.eyes,
            frame,
            Vec2::new(0.0, 0.22),
            Vec2::new(0.18, 0.015),
            0.2,
            appearance.selected,
        );
    }
}
