//! Optional contextual-information selection without imposing a popup or skin.

use bevy::input_focus::InputFocus;
use bevy::prelude::*;
use bevy::ui::{CalculatedClip, ComputedStackIndex, ComputedUiRenderTargetInfo};

use crate::{activation_eligible, GameUiSystems};

/// Game-authored, already-disclosed information associated with a native UI node.
///
/// This component does not make a node interactive, focusable, or activatable.
/// Attach it to existing controls or explicitly focusable information regions.
/// Hidden, modal-blocked, fully clipped, and not-yet-laid-out sources cannot
/// supply help. Disabled controls require an explicit [`crate::UiInspectable`]
/// marker; this never grants activation or keyboard action focus. A game's
/// skillbook or separate inspection control provides keyboard access to them.
/// Never put secrets or undisclosed domain state in these strings.
#[derive(Component, Debug, Clone, PartialEq, Eq)]
pub struct UiContextHelp {
    /// Short contextual heading, such as an ability, card, or move name.
    pub title: String,
    /// Explanation owned by the game, including any disclosed restrictions.
    pub body: String,
}

/// The selected contextual information; games decide where and how to render it.
///
/// Both fields are `None` when there is no eligible source. The plugin updates
/// this resource only when the entity or its content changes. It neither changes
/// input focus nor emits activation messages. The entity is a runtime UI identity,
/// not a durable game identity; pinning an inspector remains game-owned.
#[derive(Resource, Debug, Default, Clone, PartialEq, Eq)]
pub struct UiContextHelpState {
    /// Existing node supplying the information.
    pub entity: Option<Entity>,
    /// Copy of the selected source's current, game-authored information.
    pub content: Option<UiContextHelp>,
}

/// Ordering seams for the opt-in contextual-information capability.
#[derive(SystemSet, Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum UiContextHelpSystems {
    /// Select help in `Update`, after [`GameUiSystems::EmitActivations`].
    /// Game-owned dock/tooltip consumers may run after this set.
    Resolve,
    /// Revalidate the selected source after layout in `PostUpdate`.
    /// This clears stale content after a source is removed or scrolled away.
    Validate,
}

/// Selects pointer/focus contextual information without drawing a global popup.
///
/// Add beside [`crate::GameUiPlugin`]. Games may render a stable command dock,
/// a tooltip, or nothing, and keep their own inspector pinning and appearance.
/// A focus change wins over a stationary pointer; a fresh pointer move or hover
/// entry switches back to pointer help. Simultaneous focus and pointer changes
/// favor focus. Multiple pointer sources use Bevy's stack order, then entity ID
/// as a deterministic runtime tie-breaker. Leaving a pointer source falls back
/// to eligible focus. Focus without context does not revive stale hover content.
///
/// Selection uses the preceding layout in `Update`, and is checked again after
/// layout. New nodes cannot provide help until they have positive visible bounds.
/// The plugin relies on native interaction for pointer hit testing; it does not
/// perform an independent hit test or claim arbitrary visual occlusion handling.
pub struct GameUiContextHelpPlugin;

impl Plugin for GameUiContextHelpPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<UiContextHelpState>()
            .init_resource::<HelpInputMemory>()
            .configure_sets(
                Update,
                UiContextHelpSystems::Resolve.after(GameUiSystems::EmitActivations),
            )
            .configure_sets(
                PostUpdate,
                UiContextHelpSystems::Validate.after(bevy::ui::UiSystems::PostLayout),
            )
            .add_systems(
                Update,
                resolve_context_help.in_set(UiContextHelpSystems::Resolve),
            )
            .add_systems(
                PostUpdate,
                validate_context_help.in_set(UiContextHelpSystems::Validate),
            );
    }
}

#[derive(Default, PartialEq, Eq)]
enum HelpInputMode {
    #[default]
    Pointer,
    Focus,
}

#[derive(Resource, Default)]
struct HelpInputMemory {
    focus: Option<Entity>,
    hovered: Vec<(Entity, Interaction)>,
    cursors: Vec<(Entity, Option<Vec2>)>,
    mode: HelpInputMode,
}

fn resolve_context_help(world: &mut World) {
    let focus = world.get_resource::<InputFocus>().and_then(InputFocus::get);
    let mut hovered = {
        let mut query = world
            .query_filtered::<(Entity, &Interaction, &ComputedStackIndex), With<UiContextHelp>>();
        query
            .iter(world)
            .filter(|(_, interaction, _)| **interaction != Interaction::None)
            .map(|(entity, interaction, stack)| (entity, *interaction, stack.0))
            .collect::<Vec<_>>()
    };
    hovered.retain(|(entity, _, _)| help_eligible(world, *entity));
    hovered.sort_by_key(|(entity, _, stack)| (*stack, entity.to_bits()));
    let pointer_source = hovered.last().map(|(entity, _, _)| *entity);
    let mut hovered = hovered
        .into_iter()
        .map(|(entity, interaction, _)| (entity, interaction))
        .collect::<Vec<_>>();
    hovered.sort_by_key(|(entity, _)| entity.to_bits());
    let mut cursors = {
        let mut query = world.query::<(Entity, &Window)>();
        query
            .iter(world)
            .map(|(entity, window)| (entity, window.cursor_position()))
            .collect::<Vec<_>>()
    };
    cursors.sort_by_key(|(entity, _)| entity.to_bits());
    let focus_mode = {
        let mut memory = world.resource_mut::<HelpInputMemory>();
        if focus != memory.focus {
            memory.mode = HelpInputMode::Focus;
        } else if hovered != memory.hovered || cursors != memory.cursors {
            memory.mode = HelpInputMode::Pointer;
        }
        memory.focus = focus;
        memory.hovered = hovered;
        memory.cursors = cursors;
        memory.mode == HelpInputMode::Focus
    };
    let focus_source = focus.filter(|entity| help_eligible(world, *entity));
    let selected = if focus_mode && focus.is_some() {
        focus_source
    } else {
        pointer_source.or(focus_source)
    };
    set_help_state(world, selected);
}

fn validate_context_help(world: &mut World) {
    let current = world.resource::<UiContextHelpState>().entity;
    let selected = current.filter(|entity| help_eligible(world, *entity));
    set_help_state(world, selected);
}

fn set_help_state(world: &mut World, selected: Option<Entity>) {
    let content = selected.and_then(|entity| world.get::<UiContextHelp>(entity));
    let current = world.resource::<UiContextHelpState>();
    if current.entity == selected && current.content.as_ref() == content {
        return;
    }
    let next = UiContextHelpState {
        entity: selected,
        content: content.cloned(),
    };
    *world.resource_mut::<UiContextHelpState>() = next;
}

fn help_eligible(world: &mut World, entity: Entity) -> bool {
    world.get::<UiContextHelp>(entity).is_some()
        && if world.get::<crate::UiInspectable>(entity).is_some() {
            crate::inspection_eligible(world, entity)
        } else {
            activation_eligible(world, entity)
        }
        && has_visible_geometry(world, entity)
}

fn has_visible_geometry(world: &World, entity: Entity) -> bool {
    let Some(node) = world.get::<ComputedNode>(entity) else {
        return false;
    };
    let Some(transform) = world.get::<UiGlobalTransform>(entity) else {
        return false;
    };
    if !node.size.is_finite() || node.is_empty() || transform.try_inverse().is_none() {
        return false;
    }
    let half = node.size * 0.5;
    let mut polygon = [
        Vec2::new(-half.x, -half.y),
        Vec2::new(half.x, -half.y),
        Vec2::new(half.x, half.y),
        Vec2::new(-half.x, half.y),
    ]
    .map(|point| transform.transform_point2(point))
    .to_vec();
    if polygon.iter().any(|point| !point.is_finite()) {
        return false;
    }
    if let Some(clip) = world.get::<CalculatedClip>(entity) {
        clip_polygon(&mut polygon, clip.clip);
    }
    if let Some(target) = world.get::<ComputedUiRenderTargetInfo>(entity) {
        let size = target.physical_size();
        if size != UVec2::ZERO {
            clip_polygon(&mut polygon, Rect::from_corners(Vec2::ZERO, size.as_vec2()));
        }
    }
    // Use polygon area, not only an AABB: a rotated control can have overlapping
    // bounds while all of its actual area is clipped. Reflections remain valid.
    let twice_area: f32 = polygon
        .iter()
        .zip(polygon.iter().cycle().skip(1))
        .map(|(a, b)| a.perp_dot(*b))
        .sum();
    twice_area.is_finite() && twice_area.abs() > f32::EPSILON
}

fn clip_polygon(polygon: &mut Vec<Vec2>, clip: Rect) {
    if clip.is_empty() || clip.min.is_nan() || clip.max.is_nan() {
        polygon.clear();
        return;
    }
    for (normal, limit) in [
        (Vec2::NEG_X, -clip.min.x),
        (Vec2::X, clip.max.x),
        (Vec2::NEG_Y, -clip.min.y),
        (Vec2::Y, clip.max.y),
    ] {
        let mut output = Vec::with_capacity(polygon.len() + 1);
        for (start, end) in polygon.iter().zip(polygon.iter().cycle().skip(1)) {
            let from = start.dot(normal) - limit;
            let to = end.dot(normal) - limit;
            if from <= 0.0 {
                output.push(*start);
            }
            if (from <= 0.0) != (to <= 0.0) {
                output.push(start.lerp(*end, from / (from - to)));
            }
        }
        *polygon = output;
    }
}

#[cfg(test)]
mod tests;
