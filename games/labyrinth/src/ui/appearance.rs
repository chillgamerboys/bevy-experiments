//! Replaceable game-owned paint tokens. Layout and authority never depend on them.

use super::*;

/// Labyrinth's flat HUD palette. Insert a replacement before adding the UI plugin.
#[derive(Resource, Clone, Debug)]
pub struct LabyrinthAppearance {
    /// Command dock backing.
    pub dock: Color,
    /// Opaque inspection backing.
    pub detail: Color,
    /// Essential text and glyphs.
    pub ink: Color,
    /// Secondary readable text.
    pub muted: Color,
    /// Quiet separators.
    pub line: Color,
    /// Selected action and keyboard focus.
    pub accent: Color,
    /// Hovered action surface.
    pub hovered: Color,
    /// Pressed action surface.
    pub pressed: Color,
    /// Disabled action surface.
    pub disabled: Color,
    /// Available target or successful outcome.
    pub positive: Color,
    /// Damage forecast and harmful outcomes.
    pub damage: Color,
    /// Healing forecast and beneficial outcomes.
    pub healing: Color,
    /// Explicitly undisclosed information.
    pub unknown: Color,
    /// Glyph line thickness in the normalized 24-unit drawing grid.
    pub glyph_stroke: f32,
}

impl Default for LabyrinthAppearance {
    fn default() -> Self {
        Self {
            dock: Color::srgba(0.075, 0.065, 0.085, 0.97),
            detail: Color::srgb(0.095, 0.080, 0.105),
            ink: Color::srgb(0.95, 0.93, 0.85),
            muted: Color::srgb(0.77, 0.79, 0.77),
            line: Color::srgba(0.59, 0.55, 0.48, 0.35),
            accent: Color::srgb(0.94, 0.77, 0.44),
            hovered: Color::srgba(0.76, 0.73, 0.62, 0.025),
            pressed: Color::srgba(0.94, 0.77, 0.44, 0.055),
            disabled: Color::srgba(0.30, 0.28, 0.29, 0.20),
            positive: Color::srgb(0.58, 0.79, 0.66),
            damage: Color::srgb(0.95, 0.48, 0.42),
            healing: Color::srgb(0.54, 0.85, 0.68),
            unknown: Color::srgb(0.71, 0.68, 0.87),
            glyph_stroke: 1.8,
        }
    }
}

impl LabyrinthAppearance {
    pub(super) fn control(&self, selected: bool) -> UiSkinOverrides {
        UiSkinOverrides {
            background: Some(if selected { self.pressed } else { Color::NONE }),
            hovered: Some(self.hovered),
            pressed: Some(self.pressed),
            disabled: Some(self.disabled),
            border: Some(if selected { self.accent } else { Color::NONE }),
            focus: Some(self.accent),
            ..default()
        }
    }
}

pub(super) fn apply(appearance: Res<LabyrinthAppearance>, mut theme: ResMut<UiTheme>) {
    if !appearance.is_changed() {
        return;
    }
    theme.background = appearance.dock;
    theme.panel = appearance.detail;
    theme.card = appearance.detail;
    theme.control = appearance.dock;
    theme.control_hovered = appearance.hovered;
    theme.control_pressed = appearance.pressed;
    theme.control_disabled = appearance.disabled;
    theme.accent = appearance.accent;
    theme.text = appearance.ink;
    theme.muted_text = appearance.muted;
    theme.edge = appearance.line;
}
