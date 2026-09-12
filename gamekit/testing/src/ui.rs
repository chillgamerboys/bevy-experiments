//! Opt-in UI input and structural evidence helpers.

use bevy::input_focus::{FocusCause, InputFocus};
use bevy::prelude::*;
use bevy::window::WindowResolution;
use bevy_game_ui::{
    GameUiPlugin, GameUiSystems, UiAction, UiCard, UiDisabled, UiPanel, UiRegionRole, UiScreenRoot,
    UiTextField, UiTextRole,
};
use std::fmt;

/// Renderer-free plugin for real Bevy UI mechanics and layout, without a skin.
/// Games explicitly install their visual styling, as in their production app.
/// Unless supplied before installation, time advances by [`crate::DEFAULT_FIXED_STEP`]
/// each frame. Tooltip dwell, fades, and input tests must not depend on CPU speed.
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
        let custom_time = app
            .world()
            .contains_resource::<bevy::time::TimeUpdateStrategy>();
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
        if !custom_time {
            app.insert_resource(bevy::time::TimeUpdateStrategy::ManualDuration(
                crate::DEFAULT_FIXED_STEP,
            ));
        }
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

/// Gives an eligible action or editable field keyboard focus.
pub fn focus_action(world: &mut World, entity: Entity) -> bool {
    if (world.get::<UiAction>(entity).is_none() && world.get::<UiTextField>(entity).is_none())
        || !bevy_game_ui::activation_eligible(world, entity)
    {
        return false;
    }
    world
        .resource_mut::<InputFocus>()
        .set(entity, FocusCause::Navigated);
    true
}

/// Sends native keyboard press/release messages across deterministic frames.
/// This exercises Bevy's input dispatch and Tab navigation, not just key state.
pub fn tap_key(app: &mut App, key: KeyCode) {
    send_key(app.world_mut(), key, bevy::input::ButtonState::Pressed);
    app.update();
    send_key(app.world_mut(), key, bevy::input::ButtonState::Released);
    app.update();
}

fn send_key(world: &mut World, key_code: KeyCode, state: bevy::input::ButtonState) {
    use bevy::input::keyboard::{Key, KeyboardInput, NativeKey};
    let window = world
        .query_filtered::<Entity, With<bevy::window::PrimaryWindow>>()
        .single(world)
        .expect("headless UI has one primary window");
    let logical_key = match key_code {
        KeyCode::Enter | KeyCode::NumpadEnter => Key::Enter,
        KeyCode::Space => Key::Space,
        KeyCode::Tab => Key::Tab,
        KeyCode::Escape => Key::Escape,
        KeyCode::ArrowLeft => Key::ArrowLeft,
        KeyCode::ArrowRight => Key::ArrowRight,
        KeyCode::ArrowUp => Key::ArrowUp,
        KeyCode::ArrowDown => Key::ArrowDown,
        KeyCode::Home => Key::Home,
        KeyCode::End => Key::End,
        KeyCode::Backspace => Key::Backspace,
        KeyCode::Delete => Key::Delete,
        _ => Key::Unidentified(NativeKey::Unidentified),
    };
    world.write_message(KeyboardInput {
        key_code,
        logical_key,
        state,
        text: None,
        repeat: false,
        window,
    });
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

/// Axis-aligned bounds of transformed control corners, clipped by ancestors and viewport.
///
/// Uses the complete UI transform, including scale, rotation, and reflection, and
/// returns logical pixels. Degenerate/non-finite geometry has no visible bounds.
/// For rotated or sheared controls this AABB can overestimate the actual visible
/// polygon. It does not prove a 44-pixel hit target, pointer eligibility, occlusion,
/// or rendered appearance; test those separately.
#[must_use]
pub fn visible_control_rect(world: &World, entity: Entity, viewport: Rect) -> Option<Rect> {
    let node = world.get::<ComputedNode>(entity)?;
    let transform = world.get::<UiGlobalTransform>(entity)?;
    if !node.size().is_finite()
        || node.size().min_element() <= 0.0
        || !node.inverse_scale_factor.is_finite()
        || node.inverse_scale_factor <= 0.0
        || transform.try_inverse().is_none()
    {
        return None;
    }
    let half = node.size() * 0.5;
    let mut minimum = Vec2::splat(f32::INFINITY);
    let mut maximum = Vec2::splat(f32::NEG_INFINITY);
    for corner in [
        Vec2::new(-half.x, -half.y),
        Vec2::new(half.x, -half.y),
        Vec2::new(half.x, half.y),
        Vec2::new(-half.x, half.y),
    ] {
        let point = transform.transform_point2(corner) * node.inverse_scale_factor;
        if !point.is_finite() {
            return None;
        }
        minimum = minimum.min(point);
        maximum = maximum.max(point);
    }
    let mut rect = Rect::from_corners(minimum, maximum);
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
}

fn apply_pending_test_input(
    mut pending: ResMut<PendingTestInput>,
    mut interactions: Query<&mut Interaction>,
) {
    if let Some(entity) = pending.release_pointer.take() {
        if let Ok(mut interaction) = interactions.get_mut(entity) {
            *interaction = Interaction::None;
        }
    }
    if let Some(entity) = pending.pointer.take() {
        if let Ok(mut interaction) = interactions.get_mut(entity) {
            *interaction = Interaction::Pressed;
            pending.release_pointer = Some(entity);
        }
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
    /// Explicit accessible label, independent of the diagnostic entity name.
    pub accessible_label: Option<String>,
    /// Whether the entity accepts activation.
    pub action: bool,
    /// Whether activation is disabled.
    pub disabled: bool,
    /// Current control eligibility, including ancestors and the highest modal.
    pub activation_eligible: bool,
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
                "{} [{}] text={:?} label={:?} action={} disabled={} eligible={} focused={} tab={:?} size={:.0}x{:.0}",
                node.path,
                node.kind,
                node.text,
                node.accessible_label,
                node.action,
                node.disabled,
                node.activation_eligible,
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
            } else if world.get::<UiTextField>(entity).is_some() {
                "field"
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
                accessible_label: world
                    .get::<AccessibleLabel>(entity)
                    .map(|label| label.0.clone()),
                action: world.get::<UiAction>(entity).is_some(),
                disabled: world.get::<UiDisabled>(entity).is_some(),
                activation_eligible: (world.get::<UiAction>(entity).is_some()
                    || world.get::<UiTextField>(entity).is_some())
                    && bevy_game_ui::activation_eligible(world, entity),
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{run_frames, TestAppBuilder};
    use bevy_game_ui::{button, modal, screen_root, text_field, UiFonts, UiTabOrder};

    #[derive(Resource, Default)]
    struct KeyEvidence {
        activations: Vec<Entity>,
        submissions: Vec<Entity>,
    }

    fn collect_key_evidence(
        mut activated: MessageReader<bevy_game_ui::UiActivated>,
        mut submitted: MessageReader<bevy_game_ui::UiTextSubmitted>,
        mut evidence: ResMut<KeyEvidence>,
    ) {
        evidence
            .activations
            .extend(activated.read().map(|event| event.entity));
        evidence
            .submissions
            .extend(submitted.read().map(|event| event.entity));
    }

    #[test]
    fn native_keypad_enter_matches_enter_and_space_activation() {
        let mut app = TestAppBuilder::new().with_ui(1280, 720).build();
        app.init_resource::<KeyEvidence>().add_systems(
            Update,
            collect_key_evidence.after(GameUiSystems::EmitActivations),
        );
        let root = app.world_mut().spawn(screen_root("Keyboard controls")).id();
        let action = app
            .world_mut()
            .spawn((button("Confirm"), ChildOf(root)))
            .id();
        let field = app
            .world_mut()
            .spawn((
                text_field(&UiFonts::default(), "Name", "Name", 16),
                ChildOf(root),
            ))
            .id();
        run_frames(&mut app, 3);
        assert!(focus_action(app.world_mut(), action));
        for key in [KeyCode::Enter, KeyCode::Space, KeyCode::NumpadEnter] {
            tap_key(&mut app, key);
        }
        assert_eq!(
            app.world().resource::<KeyEvidence>().activations,
            vec![action, action, action]
        );
        assert!(focus_action(app.world_mut(), field));
        tap_key(&mut app, KeyCode::NumpadEnter);
        let evidence = app.world().resource::<KeyEvidence>();
        assert_eq!(evidence.activations, vec![action, action, action]);
        assert_eq!(evidence.submissions, vec![field]);
    }

    #[test]
    fn native_tab_skips_disabled_controls_and_respects_modal_scopes() {
        let mut app = TestAppBuilder::new().with_ui(1280, 720).build();
        let root = app.world_mut().spawn(screen_root("Scene")).id();
        let first = app
            .world_mut()
            .spawn((button("First"), UiTabOrder(1), ChildOf(root)))
            .id();
        let skipped = app
            .world_mut()
            .spawn((button("Skipped"), UiTabOrder(2), ChildOf(root)))
            .id();
        let last = app
            .world_mut()
            .spawn((button("Last"), UiTabOrder(3), ChildOf(root)))
            .id();
        run_frames(&mut app, 3);
        assert!(focus_action(app.world_mut(), first));
        // Changed between frames: eligibility must update before native dispatch.
        app.world_mut().entity_mut(skipped).insert(UiDisabled);
        tap_key(&mut app, KeyCode::Tab);
        assert_eq!(app.world().resource::<InputFocus>().get(), Some(last));
        let modal = app.world_mut().spawn(modal("Dialog")).id();
        let field = app
            .world_mut()
            .spawn((
                text_field(&UiFonts::default(), "Name", "", 16),
                ChildOf(modal),
            ))
            .id();
        let close = app
            .world_mut()
            .spawn((button("Close"), ChildOf(modal)))
            .id();
        run_frames(&mut app, 2);
        assert_eq!(app.world().resource::<InputFocus>().get(), Some(field));
        tap_key(&mut app, KeyCode::Tab);
        assert_eq!(app.world().resource::<InputFocus>().get(), Some(close));
        tap_key(&mut app, KeyCode::Tab);
        assert_eq!(app.world().resource::<InputFocus>().get(), Some(field));
        app.world_mut().despawn(modal);
        run_frames(&mut app, 2);
        assert_eq!(app.world().resource::<InputFocus>().get(), Some(last));
    }

    #[test]
    fn bare_custom_controls_materialize_declared_order_for_native_tab_navigation() {
        use bevy::input_focus::tab_navigation::TabIndex;
        let mut app = TestAppBuilder::new().with_ui(1280, 720).build();
        let root = app.world_mut().spawn(screen_root("Custom controls")).id();
        let first = app
            .world_mut()
            .spawn((button("First"), UiTabOrder(1), ChildOf(root)))
            .id();
        let custom = app
            .world_mut()
            .spawn((
                Button,
                UiAction,
                UiTabOrder(3),
                Name::new("Custom action"),
                ChildOf(root),
            ))
            .id();
        let field = app
            .world_mut()
            .spawn((
                Node::default(),
                UiTextField,
                bevy::text::EditableText::new(""),
                UiTabOrder(4),
                Name::new("Custom field"),
                ChildOf(root),
            ))
            .id();
        assert!(app.world().get::<TabIndex>(custom).is_none());
        assert!(app.world().get::<TabIndex>(field).is_none());
        run_frames(&mut app, 3);
        assert_eq!(app.world().get::<TabIndex>(custom), Some(&TabIndex(3)));
        assert_eq!(app.world().get::<TabIndex>(field), Some(&TabIndex(4)));
        assert!(focus_action(app.world_mut(), first));
        tap_key(&mut app, KeyCode::Tab);
        assert_eq!(app.world().resource::<InputFocus>().get(), Some(custom));
        tap_key(&mut app, KeyCode::Tab);
        assert_eq!(app.world().resource::<InputFocus>().get(), Some(field));
        app.world_mut().entity_mut(custom).remove::<TabIndex>();
        assert!(focus_action(app.world_mut(), first));
        tap_key(&mut app, KeyCode::Tab);
        assert_eq!(app.world().resource::<InputFocus>().get(), Some(custom));
        assert_eq!(app.world().get::<TabIndex>(custom), Some(&TabIndex(3)));
    }

    #[test]
    fn normalized_snapshot_observes_accessible_labels_and_ancestor_eligibility() {
        let mut app = TestAppBuilder::new().with_ui(1280, 720).build();
        let root = app
            .world_mut()
            .spawn((screen_root("Scene"), UiDisabled))
            .id();
        app.world_mut()
            .spawn((button("Internal ID"), ChildOf(root)))
            .insert(AccessibleLabel::new("Begin the expedition"));
        run_frames(&mut app, 3);
        let snapshot = ui_tree_snapshot(app.world_mut());
        let control = snapshot
            .nodes
            .iter()
            .find(|node| node.action)
            .expect("control");
        assert_eq!(control.path, "Scene/Internal ID");
        assert_eq!(
            control.accessible_label.as_deref(),
            Some("Begin the expedition")
        );
        assert!(!control.activation_eligible);
        assert_eq!(control.tab_index, Some(-1));
        assert!(!snapshot.to_string().contains("Entity"));
    }

    #[test]
    fn transformed_control_bounds_include_scale_rotation_and_device_conversion() {
        for device_scale in [1.0, 2.0] {
            let mut builder = TestAppBuilder::new();
            builder
                .app_mut()
                .add_plugins(HeadlessUiPlugin::with_scale_factor(
                    (1280.0 * device_scale) as u32,
                    (720.0 * device_scale) as u32,
                    device_scale,
                ));
            let mut app = builder.build();
            let root = app
                .world_mut()
                .spawn(screen_root("Transformed controls"))
                .id();
            let control = app
                .world_mut()
                .spawn((
                    Button,
                    UiAction,
                    Name::new("Scaled control"),
                    Node {
                        width: Val::Px(44.0),
                        height: Val::Px(44.0),
                        ..default()
                    },
                    UiTransform::from_scale(Vec2::splat(0.5)),
                    ChildOf(root),
                ))
                .id();
            run_frames(&mut app, 3);
            let viewport = Rect::from_corners(Vec2::ZERO, Vec2::new(1280.0, 720.0));
            let scaled = visible_control_rect(app.world(), control, viewport).expect("visible");
            assert!((scaled.width() - 22.0).abs() < 0.01, "{scaled:?}");
            assert!((scaled.height() - 22.0).abs() < 0.01, "{scaled:?}");
            app.world_mut().entity_mut(control).insert(UiTransform {
                scale: Vec2::new(-0.5, 0.5),
                rotation: Rot2::radians(std::f32::consts::FRAC_PI_4),
                ..default()
            });
            run_frames(&mut app, 3);
            let rotated = visible_control_rect(app.world(), control, viewport).expect("rotated");
            assert!(
                (rotated.width() - 22.0 * std::f32::consts::SQRT_2).abs() < 0.01,
                "{rotated:?}"
            );
            assert!(
                (rotated.height() - 22.0 * std::f32::consts::SQRT_2).abs() < 0.01,
                "{rotated:?}"
            );
            let clipped_viewport = Rect::from_corners(rotated.center(), viewport.max);
            let clipped =
                visible_control_rect(app.world(), control, clipped_viewport).expect("clipped");
            assert!((clipped.width() - rotated.width() * 0.5).abs() < 0.01);
            app.world_mut()
                .entity_mut(control)
                .insert(UiTransform::from_scale(Vec2::ZERO));
            run_frames(&mut app, 3);
            assert!(visible_control_rect(app.world(), control, viewport).is_none());
        }
    }
}
