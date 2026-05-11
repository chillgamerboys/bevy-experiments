use bevy::prelude::*;

pub const WINDOW_TITLE: &str = "Carterfight";
pub const WINDOW_SIZE: (u32, u32) = (1280, 720);

pub const BATTLE_RNG_SEED: u64 = 0xCA47_E12F_1234_5678;

pub const FONT_PATH: &str = "fonts/PressStart2P-Regular.ttf";

pub const HUD_FONT_SIZE: f32 = 28.0;
pub const DIALOGUE_FONT_SIZE: f32 = 24.0;
pub const DIALOGUE_BG: Color = Color::srgba(0.05, 0.05, 0.1, 0.92);
pub const DIALOGUE_TEXT: Color = Color::srgb(0.95, 0.95, 0.95);
pub const HUD_TEXT: Color = Color::srgb(0.85, 0.9, 1.0);

// Dialogue box layout
pub const DIALOGUE_BOX_HEIGHT: f32 = 160.0;
pub const DIALOGUE_BOX_MARGIN: f32 = 16.0;
pub const DIALOGUE_BOX_BORDER: f32 = 8.0;
pub const DIALOGUE_BOX_PADDING: f32 = 16.0;
pub const DIALOGUE_BOX_RADIUS: f32 = 12.0;
pub const DIALOGUE_BOX_BG: Color = Color::WHITE;
pub const DIALOGUE_BOX_TEXT_COLOR: Color = Color::BLACK;
pub const DIALOGUE_BOX_BORDER_COLOR: Color = Color::srgb(0.05, 0.13, 0.39);
pub const DIALOGUE_BOX_FONT_SIZE: f32 = 26.0;
pub const DIALOGUE_CHARS_PER_SEC: f32 = 20.0;

// Dialogue cursor (triangle indicator)
pub const DIALOGUE_CURSOR_W: f32 = 20.0;
pub const DIALOGUE_CURSOR_H: f32 = 12.0;
pub const DIALOGUE_CURSOR_OFFSET: f32 = 8.0;
