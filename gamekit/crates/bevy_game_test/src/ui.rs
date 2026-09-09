//! Opt-in UI input and structural evidence helpers.

use bevy::input_focus::{FocusCause, InputFocus};
use bevy::prelude::*;
use bevy::window::WindowResolution;
use bevy_game_ui::{
    GameUiPlugin, GameUiSystems, UiAction, UiCard, UiDisabled, UiPanel, UiRegionRole, UiScreenRoot,
    UiTextRole,
};
use std::fmt;

/// Renderer-free plugin for real Bevy UI schedules and layout.
pub struct HeadlessUiPlugin {
    physical_size: UVec2,
    scale_factor: f32,
}

impl HeadlessUiPlugin {
    /// Creates a one-to-one physical and logical canvas.
    #[must_use]
    pub const fn new(width: u32, height: u32) -> Self {
        Self {
            physical_size: UVec2::new(width, height),
            scale_factor: 1.0,
        }
    }

    /// Creates a canvas with an explicit device scale.
    #[must_use]
    pub const fn with_scale_factor(width: u32, height: u32, scale_factor: f32) -> Self {
        Self {
            physical_size: UVec2::new(width, height),
            scale_factor,
        }
    }
}

impl Default for HeadlessUiPlugin {
    fn default() -> Self {
        Self::new(1920, 1080)
    }
}

impl Plugin for HeadlessUiPlugin {
    fn build(&self, app: &mut App) {
        assert!(
            self.scale_factor.is_finite() && self.scale_factor > 0.0,
            "headless UI scale factor must be finite and positive"
        );
        app.add_plugins((
            MinimalPlugins,
            bevy::transform::TransformPlugin,
            bevy::camera::visibility::VisibilityPlugin,
            bevy::input::InputPlugin,
            bevy::input_focus::InputFocusPlugin,
            bevy::input_focus::InputDispatchPlugin,
            bevy::window::WindowPlugin {
                primary_window: Some(Window {
                    resolution: WindowResolution::new(self.physical_size.x, self.physical_size.y)
                        .with_scale_factor_override(self.scale_factor),
                    ..default()
                }),
                ..default()
            },
            bevy::asset::AssetPlugin {
                watch_for_changes_override: Some(false),
                ..default()
            },
            bevy::image::ImagePlugin::default(),
            bevy::mesh::MeshPlugin,
            bevy::text::TextPlugin,
            bevy::ui::UiPlugin,
        ));
        app.init_asset::<bevy::image::TextureAtlasLayout>();
        app.add_plugins(bevy::picking::DefaultPickingPlugins)
            .add_plugins(bevy::ui_widgets::UiWidgetsPlugins)
            .add_plugins(GameUiPlugin)
            .init_resource::<PendingTestInput>()
            .add_systems(
                Update,
                apply_pending_test_input.before(GameUiSystems::EmitActivations),
            );
        let physical_size = self.physical_size;
        let scale_factor = self.scale_factor;
        app.add_systems(Startup, move |mut commands: Commands| {
            commands.spawn((
                Camera2d,
                bevy::camera::Camera {
                    computed: bevy::camera::ComputedCameraValues {
                        target_info: Some(bevy::camera::RenderTargetInfo {
                            physical_size,
                            scale_factor,
                        }),
                        ..default()
                    },
                    ..default()
                },
            ));
        });
    }
}

/// Gives an enabled action keyboard focus.
pub fn focus_action(world: &mut World, entity: Entity) -> bool {
    if world.get::<UiAction>(entity).is_none() || !bevy_game_ui::activation_eligible(world, entity)
    {
        return false;
    }
    world
        .resource_mut::<InputFocus>()
        .set(entity, FocusCause::Navigated);
    true
}

/// Presses and releases a key across deterministic frames.
pub fn tap_key(app: &mut App, key: KeyCode) {
    app.world_mut().resource_mut::<PendingTestInput>().key = Some(key);
    app.update();
    app.update();
}

/// Applies a pointer press and release to one action across deterministic frames.
pub fn click_action(app: &mut App, entity: Entity) -> bool {
    if app.world().get::<Interaction>(entity).is_none() {
        return false;
    }
    app.world_mut().resource_mut::<PendingTestInput>().pointer = Some(entity);
    app.update();
    app.update();
    true
}

/// Axis-aligned control rectangle after layout, clipped by ancestors and viewport.
/// This is structural reachability evidence, not a rendered-frame or occlusion test.
#[must_use]
pub fn visible_control_rect(world: &World, entity: Entity, viewport: Rect) -> Option<Rect> {
    let node = world.get::<ComputedNode>(entity)?;
    let transform = world.get::<UiGlobalTransform>(entity)?;
    let center = transform.translation * node.inverse_scale_factor;
    let mut rect = Rect::from_center_size(center, node.size() * node.inverse_scale_factor);
    if let Some(clip) = world.get::<CalculatedClip>(entity) {
        rect = rect.intersect(Rect::from_corners(
            clip.clip.min * node.inverse_scale_factor,
            clip.clip.max * node.inverse_scale_factor,
        ));
    }
    rect = rect.intersect(viewport);
    (rect.width() > 0.0 && rect.height() > 0.0).then_some(rect)
}

#[derive(Resource, Default)]
struct PendingTestInput {
    pointer: Option<Entity>,
    release_pointer: Option<Entity>,
    key: Option<KeyCode>,
    release_key: Option<KeyCode>,
}

fn apply_pending_test_input(
    mut pending: ResMut<PendingTestInput>,
    mut keys: ResMut<ButtonInput<KeyCode>>,
    mut interactions: Query<&mut Interaction>,
) {
    if let Some(entity) = pending.release_pointer.take() {
        if let Ok(mut interaction) = interactions.get_mut(entity) {
            *interaction = Interaction::None;
        }
    }
    if let Some(key) = pending.release_key.take() {
        keys.release(key);
    }
    if let Some(entity) = pending.pointer.take() {
        if let Ok(mut interaction) = interactions.get_mut(entity) {
            *interaction = Interaction::Pressed;
            pending.release_pointer = Some(entity);
        }
    }
    if let Some(key) = pending.key.take() {
        keys.press(key);
        pending.release_key = Some(key);
    }
}

/// Stable presentation-only observation of one named UI entity.
#[derive(Debug, Clone, PartialEq)]
pub struct UiNodeSnapshot {
    /// Hierarchical stable-name path.
    pub path: String,
    /// Semantic kind used by structural review.
    pub kind: &'static str,
    /// Text value when the entity owns text.
    pub text: Option<String>,
    /// Whether the entity accepts activation.
    pub action: bool,
    /// Whether activation is disabled.
    pub disabled: bool,
    /// Whether this entity owns keyboard focus.
    pub focused: bool,
    /// Logical keyboard order, or `None` for a non-focusable node.
    pub tab_index: Option<i32>,
    /// Logical size after layout.
    pub size: Vec2,
}

/// Stable UI-tree observation that omits entity IDs and gameplay claims.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct UiTreeSnapshot {
    /// Named presentation nodes in stable path order.
    pub nodes: Vec<UiNodeSnapshot>,
}

impl fmt::Display for UiTreeSnapshot {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        for node in &self.nodes {
            writeln!(
                formatter,
                "{} [{}] text={:?} action={} disabled={} focused={} tab={:?} size={:.0}x{:.0}",
                node.path,
                node.kind,
                node.text,
                node.action,
                node.disabled,
                node.focused,
                node.tab_index,
                node.size.x,
                node.size.y
            )?;
        }
        Ok(())
    }
}

/// Observes named presentation structure without inferring gameplay correctness.
#[must_use]
pub fn ui_tree_snapshot(world: &mut World) -> UiTreeSnapshot {
    let focused = world.get_resource::<InputFocus>().and_then(InputFocus::get);
    let entities = {
        let mut query = world.query::<(Entity, &Name)>();
        query
            .iter(world)
            .map(|(entity, _)| entity)
            .collect::<Vec<_>>()
    };
    let mut nodes = entities
        .into_iter()
        .map(|entity| {
            let path = named_path(world, entity);
            let kind = if world.get::<UiScreenRoot>(entity).is_some() {
                "screen"
            } else if world.get::<UiPanel>(entity).is_some() {
                "panel"
            } else if world.get::<UiCard>(entity).is_some() {
                "card"
            } else if world.get::<UiAction>(entity).is_some() {
                "action"
            } else if let Some(role) = world.get::<UiRegionRole>(entity) {
                match role {
                    UiRegionRole::Hud => "hud",
                    UiRegionRole::ActionRail => "action-rail",
                    UiRegionRole::ActivityFeed => "activity-feed",
                    UiRegionRole::ScrollList => "scroll-list",
                }
            } else if world.get::<UiTextRole>(entity).is_some() {
                "text"
            } else {
                "named"
            };
            let text = world.get::<Text>(entity).map(|text| text.0.clone());
            let size = world
                .get::<ComputedNode>(entity)
                .map_or(Vec2::ZERO, |node| node.size() * node.inverse_scale_factor);
            UiNodeSnapshot {
                path,
                kind,
                text,
                action: world.get::<UiAction>(entity).is_some(),
                disabled: world.get::<UiDisabled>(entity).is_some(),
                focused: focused == Some(entity),
                tab_index: world
                    .get::<bevy::input_focus::tab_navigation::TabIndex>(entity)
                    .map(|index| index.0),
                size,
            }
        })
        .collect::<Vec<_>>();
    nodes.sort_by(|left, right| left.path.cmp(&right.path));
    UiTreeSnapshot { nodes }
}

fn named_path(world: &World, mut entity: Entity) -> String {
    let mut names = Vec::new();
    loop {
        if let Some(name) = world.get::<Name>(entity) {
            names.push(name.as_str().to_owned());
        }
        let Some(parent) = world.get::<ChildOf>(entity) else {
            break;
        };
        entity = parent.parent();
    }
    names.reverse();
    names.join("/")
}
