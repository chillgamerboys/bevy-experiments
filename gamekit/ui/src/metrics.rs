//! Baseline-derived semantic measurements, independent of a visual skin.

use super::*;

/// Shared motion preference. Games decide which animations to reduce or omit.
#[derive(Resource, Debug, Default, Clone, Copy, PartialEq, Eq)]
pub struct UiMotionPreference {
    /// Whether nonessential motion should be reduced.
    pub reduced: bool,
}

/// Declared control minimum at 100 percent semantic scale.
///
/// Without this component, a new action or field captures its initial pixel
/// `Node.min_width` and `min_height` once (with a 44-pixel floor). Change this
/// component, not resolved `Node` minima, to change a control's baseline later.
/// A game's width, height, and layout constraints remain game-owned.
#[derive(Component, Debug, Clone, Copy, PartialEq)]
pub struct UiControlMetrics {
    /// Baseline minimum target, independently clamped to 44 pixels per axis.
    pub min_size: Vec2,
}

impl Default for UiControlMetrics {
    fn default() -> Self {
        Self {
            min_size: Vec2::splat(44.0),
        }
    }
}

/// A stable spacing declaration, never inferred from an already-scaled node.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum UiSpace {
    /// Logical pixels at baseline scale.
    Pixels(f32),
    /// Multiples of the current [`UiTheme::spacing`] token.
    Units(f32),
}

impl UiSpace {
    fn resolve(self, theme: &UiTheme, scale: f32) -> Val {
        let baseline = match self {
            Self::Pixels(value) => value,
            Self::Units(value) => value * theme.spacing,
        };
        Val::Px(finite_nonnegative(baseline) * scale)
    }
}

/// Four independently declared semantic insets.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct UiInsets {
    /// Left inset.
    pub left: UiSpace,
    /// Right inset.
    pub right: UiSpace,
    /// Top inset.
    pub top: UiSpace,
    /// Bottom inset.
    pub bottom: UiSpace,
}

impl UiInsets {
    /// Uses the same declaration on every edge.
    #[must_use]
    pub const fn all(value: UiSpace) -> Self {
        Self::axes(value, value)
    }

    /// Uses separate horizontal and vertical declarations.
    #[must_use]
    pub const fn axes(horizontal: UiSpace, vertical: UiSpace) -> Self {
        Self {
            left: horizontal,
            right: horizontal,
            top: vertical,
            bottom: vertical,
        }
    }

    fn resolve(self, theme: &UiTheme, scale: f32) -> UiRect {
        UiRect {
            left: self.left.resolve(theme, scale),
            right: self.right.resolve(theme, scale),
            top: self.top.resolve(theme, scale),
            bottom: self.bottom.resolve(theme, scale),
        }
    }
}

/// Opts selected layout fields into baseline-derived semantic spacing.
///
/// `None` leaves that field game-owned. Helpers do not insert this component,
/// so replacing their convenience `Node` does not create a hidden baseline.
/// Removing a declaration stops managing that field; it does not restore an
/// earlier value. Assign the desired value when relinquishing ownership.
#[derive(Component, Debug, Default, Clone, Copy, PartialEq)]
pub struct UiSpacing {
    /// Managed padding, if any.
    pub padding: Option<UiInsets>,
    /// Managed margin, if any.
    pub margin: Option<UiInsets>,
    /// Managed row gap, if any.
    pub row_gap: Option<UiSpace>,
    /// Managed column gap, if any.
    pub column_gap: Option<UiSpace>,
}

/// Game-owned typography overrides for a [`UiTextRole`].
///
/// Color is deliberately absent: use a game-owned `TextColor` or opt into a
/// [`UiSkin::Text`] surface. Essential roles retain an 18-pixel minimum.
#[derive(Component, Debug, Default, Clone, PartialEq)]
pub struct UiTextStyle {
    /// Baseline font size, otherwise the semantic theme token.
    pub base_size: Option<f32>,
    /// Font handle, otherwise the corresponding [`UiFonts`] handle.
    pub font: Option<Handle<Font>>,
}

pub(crate) fn prepare_control_metrics(
    controls: Query<
        (Entity, &Node),
        (
            Or<(With<UiAction>, With<UiTextField>)>,
            Without<UiControlMetrics>,
        ),
    >,
    mut commands: Commands,
) {
    for (entity, node) in &controls {
        let pixels = |value| match value {
            Val::Px(value) => finite_nonnegative(value).max(44.0),
            _ => 44.0,
        };
        commands.entity(entity).insert(UiControlMetrics {
            min_size: Vec2::new(pixels(node.min_width), pixels(node.min_height)),
        });
    }
}

pub(crate) fn apply_control_metrics(
    metrics: Res<ResolvedUiMetrics>,
    mut controls: Query<(Ref<UiControlMetrics>, &mut Node)>,
) {
    for (baseline, mut node) in &mut controls {
        if !baseline.is_changed() && !node.is_added() && !metrics.is_changed() {
            continue;
        }
        let scale = metrics.control_scale.max(1.0);
        let width = Val::Px(finite_nonnegative(baseline.min_size.x).max(44.0) * scale);
        let height = Val::Px(finite_nonnegative(baseline.min_size.y).max(44.0) * scale);
        if node.min_width != width || node.min_height != height {
            node.min_width = width;
            node.min_height = height;
        }
    }
}

pub(crate) fn apply_spacing(
    metrics: Res<ResolvedUiMetrics>,
    theme: Res<UiTheme>,
    mut nodes: Query<(Ref<UiSpacing>, &mut Node)>,
) {
    for (spacing, mut node) in &mut nodes {
        if !spacing.is_changed() && !node.is_added() && !metrics.is_changed() && !theme.is_changed()
        {
            continue;
        }
        let mut next = node.clone();
        let scale = metrics.spacing_scale;
        if let Some(value) = spacing.padding {
            next.padding = value.resolve(&theme, scale);
        }
        if let Some(value) = spacing.margin {
            next.margin = value.resolve(&theme, scale);
        }
        if let Some(value) = spacing.row_gap {
            next.row_gap = value.resolve(&theme, scale);
        }
        if let Some(value) = spacing.column_gap {
            next.column_gap = value.resolve(&theme, scale);
        }
        node.set_if_neq(next);
    }
}

pub(crate) fn apply_typography(
    theme: Res<UiTheme>,
    fonts: Res<UiFonts>,
    metrics: Res<ResolvedUiMetrics>,
    mut texts: Query<(
        Entity,
        Ref<UiTextRole>,
        Option<Ref<UiTextStyle>>,
        &mut TextFont,
    )>,
    mut removed_styles: RemovedComponents<UiTextStyle>,
) {
    let removed = removed_styles.read().collect::<Vec<_>>();
    for (entity, role, style, mut font) in &mut texts {
        if !theme.is_changed()
            && !fonts.is_changed()
            && !metrics.is_changed()
            && !role.is_changed()
            && !font.is_added()
            && !style.as_ref().is_some_and(|style| style.is_changed())
            && !removed.contains(&entity)
        {
            continue;
        }
        let (baseline, scale, default_font) = match *role {
            UiTextRole::Display => (theme.display_size, metrics.heading_scale, &fonts.heading),
            UiTextRole::Title => (theme.title_size, metrics.heading_scale, &fonts.heading),
            UiTextRole::Body => (theme.body_size, metrics.content_scale, &fonts.body),
            UiTextRole::Supporting => (theme.supporting_size, metrics.content_scale, &fonts.body),
            UiTextRole::Metadata => (theme.metadata_size, metrics.content_scale, &fonts.body),
        };
        let size = finite_nonnegative(
            style
                .as_ref()
                .and_then(|style| style.base_size)
                .unwrap_or(baseline),
        ) * scale;
        let minimum = if *role == UiTextRole::Metadata {
            1.0
        } else {
            18.0
        };
        let handle = style
            .as_ref()
            .and_then(|style| style.font.as_ref())
            .unwrap_or(default_font);
        let mut next = font.clone();
        next.font_size = FontSize::Px(size.max(minimum));
        next.font = handle.clone().into();
        font.set_if_neq(next);
    }
}

fn finite_nonnegative(value: f32) -> f32 {
    if value.is_finite() {
        value.max(0.0)
    } else {
        0.0
    }
}
