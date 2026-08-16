//! Reusable native-Bevy UI foundations.
//!
//! The crate owns presentation mechanics, not game presentation models or
//! gameplay authority. Consumers attach their own typed action components to
//! [`UiAction`] entities and map [`UiActivated`] messages into local intents.

use bevy::input_focus::{
    tab_navigation::{TabGroup, TabIndex, TabNavigationPlugin},
    FocusCause, InputFocus, InputFocusVisible,
};
use bevy::prelude::*;
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
            .init_resource::<NamedFocusMemory>()
            .add_message::<UiActivated>()
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
                (resolve_metrics, remember_named_focus)
                    .chain()
                    .in_set(GameUiSystems::ResolveMetrics),
            )
            .add_systems(
                Update,
                (
                    emit_pointer_activation,
                    emit_keyboard_activation,
                    paint_interactions,
                )
                    .chain()
                    .in_set(GameUiSystems::EmitActivations),
            )
            .add_systems(
                PostUpdate,
                (
                    prepare_actions,
                    sync_action_reachability,
                    retain_modal_focus,
                    scroll_focused_into_view,
                    paint_keyboard_focus,
                    apply_semantic_style,
                )
                    .chain()
                    .in_set(GameUiSystems::Present)
                    .before(bevy::ui::UiSystems::Prepare),
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

#[derive(Component, Debug, Clone, Copy)]
struct ResponsiveControl {
    applied_scale: f32,
}

#[derive(Component, Debug, Clone, Copy)]
struct LogicalTabIndex(i32);

#[derive(Resource, Default)]
struct ModalFocusState {
    active: Option<Entity>,
    return_focus: Option<Entity>,
    return_focus_name: Option<String>,
}

#[derive(Resource, Default)]
struct NamedFocusMemory(Option<String>);

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

fn remember_named_focus(
    focus: Res<InputFocus>,
    names: Query<&Name>,
    mut memory: ResMut<NamedFocusMemory>,
) {
    let Some(entity) = focus.get() else { return };
    let Ok(name) = names.get(entity) else { return };
    memory.0 = Some(name.as_str().to_owned());
}

fn emit_pointer_activation(
    interactions: Query<
        (Entity, &Interaction, Option<&UiDisabled>),
        (Changed<Interaction>, With<UiAction>),
    >,
    mut activated: MessageWriter<UiActivated>,
) {
    for (entity, interaction, disabled) in &interactions {
        if disabled.is_none() && *interaction == Interaction::Pressed {
            activated.write(UiActivated { entity });
        }
    }
}

fn emit_keyboard_activation(
    keys: Res<ButtonInput<KeyCode>>,
    focus: Res<InputFocus>,
    actions: Query<
        (),
        (
            With<UiAction>,
            Without<UiDisabled>,
            Without<InteractionDisabled>,
        ),
    >,
    mut activated: MessageWriter<UiActivated>,
) {
    if !keys.any_just_pressed([KeyCode::Enter, KeyCode::Space]) {
        return;
    }
    let Some(entity) = focus.get() else { return };
    if actions.contains(entity) {
        activated.write(UiActivated { entity });
    }
}

fn prepare_actions(
    added: Query<(Entity, Option<&Name>, Option<&TabIndex>), Added<UiAction>>,
    mut commands: Commands,
) {
    for (entity, name, tab_index) in &added {
        let logical = tab_index.map_or(0, |index| index.0);
        let mut entity_commands = commands.entity(entity);
        entity_commands.insert((LogicalTabIndex(logical), TabIndex(logical)));
        if let Some(name) = name {
            entity_commands.insert(AccessibleLabel::new(name.as_str().to_owned()));
        }
    }
}

fn sync_action_reachability(world: &mut World) {
    let actions = {
        let mut query = world.query_filtered::<(Entity, &LogicalTabIndex), With<UiAction>>();
        query
            .iter(world)
            .map(|(entity, index)| (entity, index.0, world.get::<UiDisabled>(entity).is_some()))
            .collect::<Vec<_>>()
    };
    for (entity, logical_index, disabled) in actions {
        let Ok(mut entity_mut) = world.get_entity_mut(entity) else {
            continue;
        };
        if disabled {
            entity_mut.insert((InteractionDisabled, TabIndex(-1)));
        } else {
            entity_mut.remove::<InteractionDisabled>();
            entity_mut.insert(TabIndex(logical_index));
        }
    }
    let focused = world.resource::<InputFocus>().get();
    if focused.is_some_and(|entity| !is_reachable(world, entity)) {
        world.resource_mut::<InputFocus>().clear();
    }
}

fn retain_modal_focus(world: &mut World) {
    let topmost = {
        let mut query =
            world.query_filtered::<(Entity, Option<&GlobalZIndex>), With<UiModalScope>>();
        query
            .iter(world)
            .filter(|(entity, _)| is_reachable(world, *entity))
            .max_by_key(|(entity, z)| (z.map_or(0, |index| index.0), entity.to_bits()))
            .map(|(entity, _)| entity)
    };
    let current_focus = world.resource::<InputFocus>().get();
    let previous_modal = world.resource::<ModalFocusState>().active;
    match topmost {
        Some(root) => {
            if previous_modal != Some(root) {
                let return_focus =
                    current_focus.filter(|entity| !is_descendant(world, *entity, root));
                let return_focus_name = return_focus
                    .and_then(|entity| world.get::<Name>(entity))
                    .map(|name| name.as_str().to_owned())
                    .or_else(|| world.resource::<NamedFocusMemory>().0.clone());
                let mut state = world.resource_mut::<ModalFocusState>();
                state.active = Some(root);
                if state.return_focus.is_none() {
                    state.return_focus = return_focus;
                    state.return_focus_name = return_focus_name;
                }
            }
            if current_focus.is_none_or(|entity| {
                !is_descendant(world, entity, root) || !is_reachable(world, entity)
            }) {
                let target = first_reachable_action(world, root);
                let mut focus = world.resource_mut::<InputFocus>();
                if let Some(target) = target {
                    focus.set(target, FocusCause::Navigated);
                } else {
                    focus.clear();
                }
            }
        }
        None if previous_modal.is_some() => {
            let (return_focus, return_focus_name) = {
                let mut state = world.resource_mut::<ModalFocusState>();
                state.active = None;
                (state.return_focus.take(), state.return_focus_name.take())
            };
            let target = return_focus
                .filter(|entity| is_reachable(world, *entity))
                .or_else(|| {
                    let wanted = return_focus_name.as_deref()?;
                    let mut names = world.query::<(Entity, &Name)>();
                    names.iter(world).find_map(|(entity, name)| {
                        (name.as_str() == wanted && is_reachable(world, entity)).then_some(entity)
                    })
                });
            if let Some(target) = target {
                world
                    .resource_mut::<InputFocus>()
                    .set(target, FocusCause::Navigated);
            }
        }
        None => {}
    }
}

fn first_reachable_action(world: &World, root: Entity) -> Option<Entity> {
    let mut stack = vec![root];
    while let Some(entity) = stack.pop() {
        if world.get::<UiAction>(entity).is_some() && is_reachable(world, entity) {
            return Some(entity);
        }
        if let Some(children) = world.get::<Children>(entity) {
            stack.extend(children.iter().rev());
        }
    }
    None
}

fn is_descendant(world: &World, mut entity: Entity, root: Entity) -> bool {
    loop {
        if entity == root {
            return true;
        }
        let Some(parent) = world.get::<ChildOf>(entity) else {
            return false;
        };
        entity = parent.parent();
    }
}

fn is_reachable(world: &World, mut entity: Entity) -> bool {
    if world.get_entity(entity).is_err() {
        return false;
    }
    loop {
        if world.get::<InteractionDisabled>(entity).is_some()
            || world
                .get::<Visibility>(entity)
                .is_some_and(|visibility| *visibility == Visibility::Hidden)
            || world
                .get::<Node>(entity)
                .is_some_and(|node| node.display == Display::None)
        {
            return false;
        }
        let Some(parent) = world.get::<ChildOf>(entity) else {
            return true;
        };
        entity = parent.parent();
    }
}

fn scroll_focused_into_view(focus: Res<InputFocus>, mut commands: Commands) {
    if focus.is_changed() {
        if let Some(entity) = focus.get() {
            commands.trigger(ScrollIntoView { entity });
        }
    }
}

fn paint_keyboard_focus(
    focus: Res<InputFocus>,
    visible: Res<InputFocusVisible>,
    theme: Res<UiTheme>,
    actions: Query<Entity, With<UiAction>>,
    mut commands: Commands,
) {
    if !focus.is_changed() && !visible.is_changed() && !theme.is_changed() {
        return;
    }
    for entity in &actions {
        if visible.0 && focus.get() == Some(entity) {
            commands.entity(entity).insert(Outline {
                color: theme.accent,
                width: Val::Px(3.0),
                offset: Val::Px(2.0),
            });
        } else {
            commands.entity(entity).remove::<Outline>();
        }
    }
}

fn paint_interactions(
    theme: Res<UiTheme>,
    mut actions: Query<(&Interaction, Option<&UiDisabled>, &mut BackgroundColor), With<UiAction>>,
) {
    if !theme.is_changed() && actions.is_empty() {
        return;
    }
    for (interaction, disabled, mut background) in &mut actions {
        background.0 = if disabled.is_some() {
            theme.control_disabled
        } else {
            match interaction {
                Interaction::Pressed => theme.control_pressed,
                Interaction::Hovered => theme.control_hovered,
                Interaction::None => theme.control,
            }
        };
    }
}

fn apply_semantic_style(
    theme: Res<UiTheme>,
    metrics: Res<ResolvedUiMetrics>,
    mut roots: Query<
        &mut BackgroundColor,
        (
            With<UiScreenRoot>,
            Without<UiPanel>,
            Without<UiCard>,
            Without<UiAction>,
        ),
    >,
    mut panels: Query<
        (&mut BackgroundColor, &mut BorderColor),
        (With<UiPanel>, Without<UiCard>, Without<UiAction>),
    >,
    mut cards: Query<
        (&mut BackgroundColor, &mut BorderColor),
        (With<UiCard>, Without<UiPanel>, Without<UiAction>),
    >,
    mut actions: Query<&mut BorderColor, (With<UiAction>, Without<UiPanel>, Without<UiCard>)>,
    mut text_query: Query<(&UiTextRole, &mut TextFont, &mut TextColor)>,
    mut controls: Query<(&mut Node, &mut ResponsiveControl)>,
) {
    for mut background in &mut roots {
        background.0 = theme.background;
    }
    for (mut background, mut border) in &mut panels {
        background.0 = theme.panel;
        *border = BorderColor::all(theme.edge);
    }
    for (mut background, mut border) in &mut cards {
        background.0 = theme.card;
        *border = BorderColor::all(theme.edge);
    }
    for mut border in &mut actions {
        *border = BorderColor::all(theme.edge);
    }
    for (role, mut font, mut color) in &mut text_query {
        let (size, wanted_color) = match role {
            UiTextRole::Display => (theme.display_size * metrics.heading_scale, theme.text),
            UiTextRole::Title => (theme.title_size * metrics.heading_scale, theme.accent),
            UiTextRole::Body => (
                (theme.body_size * metrics.content_scale).max(18.0),
                theme.text,
            ),
            UiTextRole::Supporting => (
                (theme.supporting_size * metrics.content_scale).max(18.0),
                theme.muted_text,
            ),
            UiTextRole::Metadata => (
                theme.metadata_size * metrics.content_scale,
                theme.muted_text,
            ),
        };
        font.font_size = FontSize::Px(size);
        color.0 = wanted_color;
    }
    let next_scale = metrics.control_scale.max(1.0);
    for (mut node, mut control) in &mut controls {
        let ratio = next_scale / control.applied_scale.max(1.0);
        if let Val::Px(width) = node.min_width {
            node.min_width = Val::Px((width * ratio).max(44.0 * next_scale));
        }
        if let Val::Px(height) = node.min_height {
            node.min_height = Val::Px((height * ratio).max(44.0 * next_scale));
        }
        control.applied_scale = next_scale;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

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
