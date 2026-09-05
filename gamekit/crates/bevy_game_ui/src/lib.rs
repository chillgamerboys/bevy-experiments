//! Reusable native-Bevy UI foundations.
//!
//! The crate owns presentation mechanics, not game presentation models or
//! gameplay authority. Consumers attach their own typed action components to
//! [`UiAction`] entities and map [`UiActivated`] messages into local intents.

mod focus;
mod style;

pub use focus::activation_eligible;
use focus::{
    emit_activations, emit_text_field_messages, prepare_actions, prepare_scrolling,
    remember_scoped_focus, retain_modal_focus, scroll_focused_into_view, sync_action_reachability,
    ModalFocusState, ScopedFocusMemory,
};
use style::{
    apply_semantic_style, apply_text_field_style, paint_interactions, paint_keyboard_focus,
};

use bevy::input::keyboard::Key;
use bevy::input_focus::{
    tab_navigation::{TabGroup, TabIndex, TabNavigationPlugin},
    FocusCause, InputFocus, InputFocusVisible,
};
use bevy::prelude::*;
use bevy::text::{EditableText, TextCursorStyle};
use bevy::ui::InteractionDisabled;
use bevy::ui_widgets::ScrollIntoView;

/// Installs responsive metrics, semantic styling, activation, and keyboard focus.
pub struct GameUiPlugin;

/// Public ordering seams for consumer systems.
#[derive(SystemSet, Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum GameUiSystems {
    /// Resolve the logical canvas and semantic density.
    ResolveMetrics,
    /// Translate pointer and keyboard input into [`UiActivated`] messages.
    EmitActivations,
    /// Apply focus, reachability, and semantic presentation.
    Present,
}

impl Plugin for GameUiPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(TabNavigationPlugin)
            .init_resource::<UiTheme>()
            .init_resource::<UiFonts>()
            .init_resource::<UiScalePreference>()
            .init_resource::<ResolvedUiMetrics>()
            .init_resource::<ModalFocusState>()
            .init_resource::<ScopedFocusMemory>()
            .add_message::<UiActivated>()
            .add_message::<UiTextChanged>()
            .add_message::<UiTextSubmitted>()
            .configure_sets(
                Update,
                (
                    GameUiSystems::ResolveMetrics,
                    GameUiSystems::EmitActivations,
                )
                    .chain(),
            )
            .configure_sets(PostUpdate, GameUiSystems::Present)
            .add_systems(
                Update,
                (
                    resolve_metrics,
                    prepare_actions,
                    sync_action_reachability,
                    retain_modal_focus,
                    remember_scoped_focus,
                )
                    .chain()
                    .in_set(GameUiSystems::ResolveMetrics),
            )
            .add_systems(
                Update,
                (
                    emit_activations,
                    emit_text_field_messages,
                    paint_interactions,
                )
                    .chain()
                    .in_set(GameUiSystems::EmitActivations),
            )
            .add_systems(
                PostUpdate,
                (
                    prepare_actions,
                    prepare_scrolling,
                    sync_action_reachability,
                    retain_modal_focus,
                    paint_keyboard_focus,
                    apply_semantic_style,
                    apply_text_field_style,
                )
                    .chain()
                    .in_set(GameUiSystems::Present)
                    .before(bevy::ui::UiSystems::Prepare),
            )
            .add_systems(
                PostUpdate,
                scroll_focused_into_view.after(bevy::ui::UiSystems::Layout),
            );
    }
}

/// Semantic visual tokens supplied or replaced by a game.
#[derive(Resource, Debug, Clone, PartialEq)]
pub struct UiTheme {
    /// Full-screen background.
    pub background: Color,
    /// Raised panel background.
    pub panel: Color,
    /// Card background.
    pub card: Color,
    /// Resting control background.
    pub control: Color,
    /// Hovered control background.
    pub control_hovered: Color,
    /// Pressed control background.
    pub control_pressed: Color,
    /// Disabled control background.
    pub control_disabled: Color,
    /// Primary readable text.
    pub text: Color,
    /// Supporting readable text.
    pub muted_text: Color,
    /// Primary accent and keyboard focus color.
    pub accent: Color,
    /// Borders and dividers.
    pub edge: Color,
    /// Display text size at 100 percent.
    pub display_size: f32,
    /// Screen-title size at 100 percent.
    pub title_size: f32,
    /// Essential body size at 100 percent.
    pub body_size: f32,
    /// Supporting text size at 100 percent.
    pub supporting_size: f32,
    /// Optional metadata size at 100 percent.
    pub metadata_size: f32,
    /// Shared baseline gap and padding unit.
    pub spacing: f32,
}

impl Default for UiTheme {
    fn default() -> Self {
        Self {
            background: Color::srgb(0.035, 0.045, 0.07),
            panel: Color::srgba(0.07, 0.085, 0.12, 0.98),
            card: Color::srgba(0.11, 0.13, 0.18, 0.99),
            control: Color::srgb(0.14, 0.17, 0.23),
            control_hovered: Color::srgb(0.20, 0.24, 0.32),
            control_pressed: Color::srgb(0.28, 0.31, 0.38),
            control_disabled: Color::srgb(0.09, 0.10, 0.13),
            text: Color::srgb(0.96, 0.97, 0.99),
            muted_text: Color::srgb(0.73, 0.76, 0.82),
            accent: Color::srgb(0.42, 0.82, 0.96),
            edge: Color::srgba(0.82, 0.88, 1.0, 0.24),
            display_size: 48.0,
            title_size: 32.0,
            body_size: 20.0,
            supporting_size: 18.0,
            metadata_size: 16.0,
            spacing: 12.0,
        }
    }
}

/// Runtime fonts used by semantic text helpers.
#[derive(Resource, Debug, Clone, Default)]
pub struct UiFonts {
    /// Display and heading face.
    pub heading: Handle<Font>,
    /// Body and control face.
    pub body: Handle<Font>,
}

/// Persistable semantic content-scale choice.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub enum UiScaleMode {
    /// Resolve scale from the logical canvas.
    #[default]
    Auto,
    /// Seventy-five percent content scale.
    Percent75,
    /// Baseline content scale.
    Percent100,
    /// One hundred twenty-five percent content scale.
    Percent125,
    /// One hundred fifty percent content scale.
    Percent150,
    /// One hundred seventy-five percent content scale.
    Percent175,
    /// Two hundred percent content scale.
    Percent200,
}

impl UiScaleMode {
    const fn manual_factor(self) -> Option<f32> {
        match self {
            Self::Auto => None,
            Self::Percent75 => Some(0.75),
            Self::Percent100 => Some(1.0),
            Self::Percent125 => Some(1.25),
            Self::Percent150 => Some(1.5),
            Self::Percent175 => Some(1.75),
            Self::Percent200 => Some(2.0),
        }
    }
}

/// Game-owned preference projected into semantic metrics.
#[derive(Resource, Debug, Default, Clone, Copy, PartialEq, Eq)]
pub struct UiScalePreference(pub UiScaleMode);

/// Responsive layout class after semantic density adjustment.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UiViewportClass {
    /// Constrained layout where secondary panes should collapse.
    Compact,
    /// Ordinary desktop layout.
    Standard,
    /// Extra-horizontal layout suitable for persistent secondary panes.
    Wide,
}

/// Resolved logical canvas and semantic scale factors.
#[derive(Resource, Debug, Clone, Copy, PartialEq)]
pub struct ResolvedUiMetrics {
    /// Logical window size.
    pub logical_size: Vec2,
    /// Essential text multiplier.
    pub content_scale: f32,
    /// Moderated heading multiplier.
    pub heading_scale: f32,
    /// Moderated control multiplier, never below one.
    pub control_scale: f32,
    /// Moderated layout-density multiplier.
    pub spacing_scale: f32,
    /// Density-adjusted layout class.
    pub viewport: UiViewportClass,
}

impl Default for ResolvedUiMetrics {
    fn default() -> Self {
        resolve_ui_metrics(Vec2::new(1920.0, 1080.0), UiScaleMode::Auto)
    }
}

/// Resolves semantic metrics without mutating Bevy's global UI scale.
#[must_use]
pub fn resolve_ui_metrics(size: Vec2, mode: UiScaleMode) -> ResolvedUiMetrics {
    let content_scale = mode
        .manual_factor()
        .unwrap_or_else(|| (size.x / 1920.0).min(size.y / 1080.0).clamp(1.0, 1.5));
    let delta = content_scale - 1.0;
    let heading_scale = (1.0 + 0.5 * delta).clamp(0.875, 1.5);
    let control_scale = (1.0 + 0.5 * delta).clamp(1.0, 1.5);
    let spacing_scale = (1.0 + 0.25 * delta).clamp(0.9375, 1.25);
    let effective = size / content_scale.max(spacing_scale);
    let viewport = if effective.x < 1440.0 || effective.y < 810.0 {
        UiViewportClass::Compact
    } else if effective.x >= 2400.0 {
        UiViewportClass::Wide
    } else {
        UiViewportClass::Standard
    };
    ResolvedUiMetrics {
        logical_size: size,
        content_scale,
        heading_scale,
        control_scale,
        spacing_scale,
        viewport,
    }
}

/// Semantic text hierarchy.
#[derive(Component, Debug, Clone, Copy, PartialEq, Eq)]
pub enum UiTextRole {
    /// Large display title.
    Display,
    /// Screen or section title.
    Title,
    /// Essential body or control copy.
    Body,
    /// Supporting copy that remains essential.
    Supporting,
    /// Optional metadata.
    Metadata,
}

/// Marks a full-screen presentation root.
#[derive(Component, Debug, Clone, Copy)]
pub struct UiScreenRoot;

/// Marks a raised panel.
#[derive(Component, Debug, Clone, Copy)]
pub struct UiPanel;

/// Marks a card surface.
#[derive(Component, Debug, Clone, Copy)]
pub struct UiCard;

/// Marks a control whose activation is translated into [`UiActivated`].
#[derive(Component, Debug, Clone, Copy)]
pub struct UiAction;

/// Marks a native editable single-line text field.
#[derive(Component, Debug, Clone, Copy)]
pub struct UiTextField;

/// Explicit stable identity used to restore focus after a game rebuilds a view.
/// Labels and `Name` are not identities. Duplicate identities fail closed.
#[derive(Component, Debug, Clone, PartialEq, Eq)]
pub struct UiFocusId {
    /// Game-owned scope, unique among simultaneously mounted views.
    pub scope: String,
    /// Game-owned control key within that scope.
    pub key: String,
}

impl UiFocusId {
    /// Creates a scoped identity without imposing a game presentation model.
    #[must_use]
    pub fn new(scope: impl Into<String>, key: impl Into<String>) -> Self {
        Self {
            scope: scope.into(),
            key: key.into(),
        }
    }
}

/// Disables pointer and keyboard activation for a [`UiAction`].
#[derive(Component, Debug, Clone, Copy)]
pub struct UiDisabled;

/// Marks a full-screen blocking modal and its focus scope.
#[derive(Component, Debug, Clone, Copy)]
pub struct UiModalScope;

/// Semantic regions used by gameplay layouts and structural tests.
#[derive(Component, Debug, Clone, Copy, PartialEq, Eq)]
pub enum UiRegionRole {
    /// Persistent status display.
    Hud,
    /// Primary actions for the current decision.
    ActionRail,
    /// Chronological activity or combat information.
    ActivityFeed,
    /// Scrollable collection.
    ScrollList,
}

/// An entity-level activation emitted by pointer or keyboard input.
#[derive(Message, Debug, Clone, Copy, PartialEq, Eq)]
pub struct UiActivated {
    /// Activated entity carrying the game's typed action component.
    pub entity: Entity,
}

/// Current value of a changed native text field.
#[derive(Message, Clone, PartialEq, Eq)]
pub struct UiTextChanged {
    /// Changed field entity carrying the game's typed marker component.
    pub entity: Entity,
    /// Current committed text, excluding an active IME pre-edit range.
    pub value: String,
}

/// Enter submission from a focused native text field.
#[derive(Message, Clone, PartialEq, Eq)]
pub struct UiTextSubmitted {
    /// Submitted field entity carrying the game's typed marker component.
    pub entity: Entity,
    /// Current committed text.
    pub value: String,
}

macro_rules! sensitive_text_message {
    ($message:ty) => {
        impl std::fmt::Debug for $message {
            fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                f.debug_struct(stringify!($message))
                    .field("entity", &self.entity)
                    .field("value", &"[REDACTED]")
                    .finish()
            }
        }
        impl Drop for $message {
            fn drop(&mut self) {
                zeroize::Zeroize::zeroize(&mut self.value);
            }
        }
    };
}
sensitive_text_message!(UiTextChanged);
sensitive_text_message!(UiTextSubmitted);

#[derive(Component, Debug, Clone, Copy)]
struct ResponsiveControl {
    applied_scale: f32,
}

#[derive(Component, Debug, Clone, Copy)]
struct LogicalTabIndex(i32);

/// Creates an opaque, keyboard-navigable full-screen root.
#[must_use]
pub fn screen_root(name: impl Into<String>) -> impl Bundle {
    (
        Name::new(name.into()),
        UiScreenRoot,
        TabGroup::new(0),
        Node {
            width: Val::Percent(100.0),
            height: Val::Percent(100.0),
            align_items: AlignItems::Center,
            justify_content: JustifyContent::Center,
            flex_direction: FlexDirection::Column,
            row_gap: Val::Px(16.0),
            overflow: Overflow::clip(),
            ..default()
        },
        BackgroundColor::default(),
    )
}

/// Creates a raised column panel.
#[must_use]
pub fn panel(name: impl Into<String>) -> impl Bundle {
    (
        Name::new(name.into()),
        UiPanel,
        Node {
            flex_direction: FlexDirection::Column,
            row_gap: Val::Px(12.0),
            padding: UiRect::all(Val::Px(18.0)),
            border: UiRect::all(Val::Px(1.0)),
            border_radius: BorderRadius::all(Val::Px(10.0)),
            ..default()
        },
        BorderColor::default(),
        BackgroundColor::default(),
    )
}

/// Creates a card surface with a minimum interactive footprint.
#[must_use]
pub fn card(name: impl Into<String>) -> impl Bundle {
    (
        Name::new(name.into()),
        UiCard,
        Node {
            min_width: Val::Px(172.0),
            min_height: Val::Px(220.0),
            flex_direction: FlexDirection::Column,
            row_gap: Val::Px(10.0),
            padding: UiRect::all(Val::Px(14.0)),
            border: UiRect::all(Val::Px(2.0)),
            border_radius: BorderRadius::all(Val::Px(10.0)),
            ..default()
        },
        BorderColor::default(),
        BackgroundColor::default(),
    )
}

/// Creates a standard semantic action button.
#[must_use]
pub fn button(name: impl Into<String>) -> impl Bundle {
    let name = name.into();
    (
        Name::new(name.clone()),
        AccessibleLabel::new(name),
        Button,
        UiAction,
        TabIndex(0),
        LogicalTabIndex(0),
        ResponsiveControl { applied_scale: 1.0 },
        Node {
            min_width: Val::Px(132.0),
            min_height: Val::Px(48.0),
            align_items: AlignItems::Center,
            justify_content: JustifyContent::Center,
            padding: UiRect::axes(Val::Px(18.0), Val::Px(10.0)),
            border: UiRect::all(Val::Px(1.0)),
            border_radius: BorderRadius::all(Val::Px(7.0)),
            ..default()
        },
        BorderColor::default(),
        BackgroundColor::default(),
    )
}

/// Creates a native single-line field with bounded input, focus, IME, and clipboard editing.
#[must_use]
pub fn text_field(
    fonts: &UiFonts,
    name: impl Into<String>,
    initial_value: impl AsRef<str>,
    max_characters: usize,
) -> impl Bundle {
    let name = name.into();
    let mut editable = EditableText::new(initial_value);
    editable.max_characters = Some(max_characters);
    editable.visible_lines = Some(1.0);
    editable.visible_width = Some(32.0);
    (
        Name::new(name.clone()),
        AccessibleLabel::new(name),
        UiTextField,
        editable,
        TextCursorStyle::default(),
        TextLayout::no_wrap(),
        TextFont {
            font: fonts.body.clone().into(),
            font_size: FontSize::Px(20.0),
            ..default()
        },
        TextColor::default(),
        TabIndex(0),
        LogicalTabIndex(0),
        ResponsiveControl { applied_scale: 1.0 },
        Node {
            width: Val::Percent(100.0),
            min_width: Val::Px(220.0),
            min_height: Val::Px(48.0),
            padding: UiRect::axes(Val::Px(14.0), Val::Px(10.0)),
            border: UiRect::all(Val::Px(1.0)),
            border_radius: BorderRadius::all(Val::Px(7.0)),
            overflow: Overflow::clip_x(),
            overflow_clip_margin: OverflowClipMargin {
                visual_box: VisualBox::ContentBox,
                ..default()
            },
            ..default()
        },
        BorderColor::default(),
        BackgroundColor::default(),
    )
}

/// Creates semantic text using the supplied runtime font handles.
#[must_use]
pub fn text(fonts: &UiFonts, role: UiTextRole, value: impl Into<String>) -> impl Bundle {
    let font = match role {
        UiTextRole::Display | UiTextRole::Title => fonts.heading.clone(),
        UiTextRole::Body | UiTextRole::Supporting | UiTextRole::Metadata => fonts.body.clone(),
    };
    (
        role,
        Text::new(value.into()),
        TextFont {
            font: font.into(),
            ..TextFont::default()
        },
        TextColor::default(),
        Pickable::IGNORE,
    )
}

/// Creates a blocking full-screen modal overlay.
#[must_use]
pub fn modal(name: impl Into<String>) -> impl Bundle {
    (
        Name::new(name.into()),
        UiModalScope,
        TabGroup::modal(),
        GlobalZIndex(100),
        Node {
            position_type: PositionType::Absolute,
            width: Val::Percent(100.0),
            height: Val::Percent(100.0),
            align_items: AlignItems::Center,
            justify_content: JustifyContent::Center,
            ..default()
        },
        BackgroundColor(Color::srgba(0.0, 0.0, 0.0, 0.72)),
    )
}

/// Creates a semantic region container.
#[must_use]
pub fn region(name: impl Into<String>, role: UiRegionRole) -> impl Bundle {
    let overflow = if role == UiRegionRole::ScrollList || role == UiRegionRole::ActivityFeed {
        Overflow::scroll_y()
    } else {
        Overflow::DEFAULT
    };
    (
        Name::new(name.into()),
        role,
        Node {
            flex_direction: FlexDirection::Column,
            row_gap: Val::Px(8.0),
            overflow,
            ..default()
        },
        ScrollPosition::default(),
    )
}

fn resolve_metrics(
    preference: Res<UiScalePreference>,
    windows: Query<&Window, With<bevy::window::PrimaryWindow>>,
    mut metrics: ResMut<ResolvedUiMetrics>,
) {
    let Ok(window) = windows.single() else { return };
    let next = resolve_ui_metrics(Vec2::new(window.width(), window.height()), preference.0);
    if *metrics != next {
        *metrics = next;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn input_app() -> App {
        let mut app = App::new();
        app.init_resource::<InputFocus>()
            .init_resource::<ButtonInput<KeyCode>>()
            .init_resource::<ModalFocusState>()
            .init_resource::<ScopedFocusMemory>()
            .add_message::<UiActivated>()
            .add_systems(Update, emit_activations);
        app
    }

    #[test]
    fn pointer_and_keyboard_share_ancestor_and_modal_eligibility() {
        let mut app = input_app();
        let action = app
            .world_mut()
            .spawn((UiAction, Node::default(), Interaction::Pressed))
            .id();
        let ancestor = app
            .world_mut()
            .spawn(Node {
                display: Display::None,
                ..default()
            })
            .add_child(action)
            .id();
        app.world_mut()
            .resource_mut::<InputFocus>()
            .set(action, FocusCause::Navigated);
        app.world_mut()
            .resource_mut::<ButtonInput<KeyCode>>()
            .press(KeyCode::Enter);
        app.update();
        assert_eq!(
            app.world_mut()
                .resource_mut::<Messages<UiActivated>>()
                .drain()
                .count(),
            0
        );
        app.world_mut()
            .get_mut::<Node>(ancestor)
            .expect("parent")
            .display = Display::Flex;
        app.world_mut().entity_mut(action).insert(UiDisabled);
        assert!(!activation_eligible(app.world_mut(), action));
        app.world_mut().entity_mut(action).remove::<UiDisabled>();
        let modal = app
            .world_mut()
            .spawn((UiModalScope, Node::default(), GlobalZIndex(1)))
            .id();
        assert!(!activation_eligible(app.world_mut(), action));
        app.world_mut().entity_mut(modal).add_child(action);
        assert!(activation_eligible(app.world_mut(), action));
        app.world_mut()
            .entity_mut(action)
            .insert(Interaction::Pressed);
        app.update();
        // Simultaneous keyboard and pointer input still emits one activation.
        assert_eq!(
            app.world_mut()
                .resource_mut::<Messages<UiActivated>>()
                .drain()
                .count(),
            1
        );
    }

    #[test]
    fn restoration_uses_scoped_identity_not_matching_labels() {
        let mut app = input_app();
        let original = app
            .world_mut()
            .spawn((UiAction, UiFocusId::new("left", "play"), Name::new("Play")))
            .id();
        app.world_mut()
            .resource_mut::<InputFocus>()
            .set(original, FocusCause::Navigated);
        let modal = app.world_mut().spawn((UiModalScope, Node::default())).id();
        retain_modal_focus(app.world_mut());
        app.world_mut().despawn(original);
        let wrong = app
            .world_mut()
            .spawn((UiAction, UiFocusId::new("right", "play"), Name::new("Play")))
            .id();
        let replacement = app
            .world_mut()
            .spawn((
                UiAction,
                UiFocusId::new("left", "play"),
                Name::new("Changed label"),
            ))
            .id();
        app.world_mut().despawn(modal);
        retain_modal_focus(app.world_mut());
        assert_eq!(
            app.world().resource::<InputFocus>().get(),
            Some(replacement)
        );
        assert_ne!(replacement, wrong);
    }

    #[test]
    fn text_messages_redact_plaintext_from_debug() {
        let message = UiTextChanged {
            entity: Entity::PLACEHOLDER,
            value: "private-test-value".to_owned(),
        };
        assert!(!format!("{message:?}").contains("private-test-value"));
    }

    #[test]
    fn unchanged_scoped_control_recovers_focus_after_a_view_rebuild() {
        let mut app = input_app();
        app.add_systems(PreUpdate, remember_scoped_focus);
        let original = app
            .world_mut()
            .spawn((UiAction, UiFocusId::new("browser", "row-1")))
            .id();
        app.world_mut()
            .resource_mut::<InputFocus>()
            .set(original, FocusCause::Navigated);
        app.update();
        app.world_mut().despawn(original);
        let replacement = app
            .world_mut()
            .spawn((UiAction, UiFocusId::new("browser", "row-1")))
            .id();
        sync_action_reachability(app.world_mut());
        assert_eq!(
            app.world().resource::<InputFocus>().get(),
            Some(replacement)
        );
    }

    #[test]
    fn scrollable_nodes_receive_native_scrolling_support() {
        let mut app = App::new();
        app.add_systems(Update, prepare_scrolling);
        let entity = app
            .world_mut()
            .spawn(Node {
                overflow: Overflow::scroll_y(),
                ..default()
            })
            .id();
        app.update();
        assert!(app
            .world()
            .get::<bevy::ui_widgets::ScrollArea>(entity)
            .is_some());
        app.world_mut()
            .get_mut::<Node>(entity)
            .expect("node")
            .overflow = Overflow::DEFAULT;
        app.update();
        assert!(app
            .world()
            .get::<bevy::ui_widgets::ScrollArea>(entity)
            .is_none());
    }

    #[test]
    fn nested_modal_close_restores_the_parent_control_then_original_focus() {
        let mut app = input_app();
        let original = app.world_mut().spawn(UiAction).id();
        app.world_mut()
            .resource_mut::<InputFocus>()
            .set(original, FocusCause::Navigated);
        let outer = app
            .world_mut()
            .spawn((UiModalScope, Node::default(), GlobalZIndex(1)))
            .id();
        let outer_action = app.world_mut().spawn((UiAction, ChildOf(outer))).id();
        retain_modal_focus(app.world_mut());
        assert_eq!(
            app.world().resource::<InputFocus>().get(),
            Some(outer_action)
        );
        let inner = app
            .world_mut()
            .spawn((UiModalScope, Node::default(), GlobalZIndex(2)))
            .id();
        let inner_action = app.world_mut().spawn((UiAction, ChildOf(inner))).id();
        retain_modal_focus(app.world_mut());
        assert_eq!(
            app.world().resource::<InputFocus>().get(),
            Some(inner_action)
        );
        app.world_mut().despawn(inner);
        retain_modal_focus(app.world_mut());
        assert_eq!(
            app.world().resource::<InputFocus>().get(),
            Some(outer_action)
        );
        app.world_mut().despawn(outer);
        retain_modal_focus(app.world_mut());
        assert_eq!(app.world().resource::<InputFocus>().get(), Some(original));
    }

    #[test]
    fn semantic_metrics_cover_compact_standard_wide_and_accessibility_scale() {
        assert_eq!(
            resolve_ui_metrics(Vec2::new(1280.0, 720.0), UiScaleMode::Auto).viewport,
            UiViewportClass::Compact
        );
        assert_eq!(
            resolve_ui_metrics(Vec2::new(1920.0, 1080.0), UiScaleMode::Auto).viewport,
            UiViewportClass::Standard
        );
        assert_eq!(
            resolve_ui_metrics(Vec2::new(3840.0, 2160.0), UiScaleMode::Auto).viewport,
            UiViewportClass::Wide
        );
        let doubled = resolve_ui_metrics(Vec2::new(1920.0, 1080.0), UiScaleMode::Percent200);
        assert_eq!(doubled.viewport, UiViewportClass::Compact);
        assert_eq!(doubled.content_scale, 2.0);
        assert!(doubled.control_scale >= 1.0);
    }
}
