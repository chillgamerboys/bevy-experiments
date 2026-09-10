//! Small, game-local art seam. No asset handles enter rules or the wire protocol.

use bevy::prelude::*;
use labyrinth_rules::{ActorKind, EnemyKind, HeroClass};

/// Replace the optional sheet or colors without changing layout, input or rules.
#[derive(Resource, Clone)]
pub(crate) struct SceneAppearance {
    /// Transparent fixed 4x2 sheet: four heroes above the four enemy archetypes.
    pub actor_sheet: Option<Handle<Image>>,
    /// Independently replaceable large cutouts; no atlas coordinate assumptions.
    pub wagon: Option<Handle<Image>>,
    /// Two-rank monster cutout.
    pub hauler: Option<Handle<Image>>,
    /// Fraction of each cell's left edge to trim, in sheet order. Clean replacement
    /// sheets can use all zeroes; the bundled Medic/Archer contain neighbor pixels.
    pub cell_left_trim: [f32; 8],
    /// Optional environment plate; primitives remain the loading/error fallback.
    pub backdrop_image: Option<Handle<Image>>,
    /// The painted floor's vertical fraction in the uncropped environment plate.
    pub backdrop_floor: f32,
    pub backdrop: Color,
    pub ground: Color,
    pub mist: Color,
    pub selected: Color,
    pub active: Color,
}

impl Default for SceneAppearance {
    fn default() -> Self {
        Self {
            actor_sheet: None,
            wagon: None,
            hauler: None,
            cell_left_trim: [0.0, 0.0, 0.0, 44.0 / 384.0, 0.0, 0.0, 0.0, 26.0 / 384.0],
            backdrop_image: None,
            backdrop_floor: 0.66,
            backdrop: Color::srgb(0.025, 0.034, 0.038),
            ground: Color::srgb(0.06, 0.063, 0.057),
            mist: Color::srgba(0.32, 0.36, 0.33, 0.055),
            selected: Color::srgb(0.88, 0.71, 0.35),
            active: Color::srgb(0.40, 0.72, 0.64),
        }
    }
}

impl SceneAppearance {
    pub fn actor_image(&self, kind: ActorKind) -> Option<&Handle<Image>> {
        match kind {
            ActorKind::Hero(HeroClass::LanternWagon) => self.wagon.as_ref(),
            ActorKind::Enemy(EnemyKind::OssuaryHauler) => self.hauler.as_ref(),
            _ => self.actor_sheet.as_ref(),
        }
    }
    pub fn actor_rect(&self, size: UVec2, kind: ActorKind) -> Option<Rect> {
        if kind.footprint() > 1 {
            return (size.min_element() > 0)
                .then(|| Rect::from_corners(Vec2::ZERO, size.as_vec2()));
        }
        let trim = *self.cell_left_trim.get(sheet_index(kind) as usize)?;
        sheet_rect(size, kind, trim)
    }
}

pub(super) fn sheet_index(kind: ActorKind) -> u32 {
    match kind {
        ActorKind::Hero(HeroClass::Gatekeeper) => 0,
        ActorKind::Hero(HeroClass::Knifehand) => 1,
        ActorKind::Hero(HeroClass::Scout) => 2,
        ActorKind::Hero(HeroClass::FieldMedic) => 3,
        ActorKind::Enemy(EnemyKind::AshBrute) => 4,
        ActorKind::Enemy(EnemyKind::IronBrute) => 5,
        ActorKind::Enemy(EnemyKind::WoundStalker) => 6,
        ActorKind::Enemy(EnemyKind::HollowArcher) => 7,
        ActorKind::Hero(HeroClass::LanternWagon) => 8,
        ActorKind::Enemy(EnemyKind::OssuaryHauler) => 9,
    }
}

fn sheet_rect(size: UVec2, kind: ActorKind, left_trim: f32) -> Option<Rect> {
    if size.x < 4
        || size.y < 2
        || !size.x.is_multiple_of(4)
        || !size.y.is_multiple_of(2)
        || !left_trim.is_finite()
        || !(0.0..1.0).contains(&left_trim)
    {
        return None;
    }
    let cell = size.as_vec2() / Vec2::new(4.0, 2.0);
    let index = sheet_index(kind);
    let origin = Vec2::new((index % 4) as f32, (index / 4) as f32) * cell;
    Some(Rect::from_corners(
        origin + Vec2::new(cell.x * left_trim, 0.0),
        origin + cell,
    ))
}

pub(super) fn fallback_color(kind: ActorKind) -> Color {
    match kind {
        ActorKind::Hero(HeroClass::Gatekeeper) => Color::srgb(0.33, 0.45, 0.49),
        ActorKind::Hero(HeroClass::Knifehand) => Color::srgb(0.46, 0.31, 0.24),
        ActorKind::Hero(HeroClass::Scout) => Color::srgb(0.29, 0.43, 0.31),
        ActorKind::Hero(HeroClass::FieldMedic) => Color::srgb(0.42, 0.35, 0.46),
        ActorKind::Hero(HeroClass::LanternWagon) => Color::srgb(0.46, 0.38, 0.22),
        ActorKind::Enemy(_) => Color::srgb(0.40, 0.24, 0.21),
    }
}
