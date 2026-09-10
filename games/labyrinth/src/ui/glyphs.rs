//! Small code-native line glyphs. No font symbol coverage or raster atlas required.

use super::*;

#[derive(Clone, Copy)]
pub(super) enum Glyph {
    Blade,
    Reach,
    Push,
    Heal,
    Bleed,
    Arrow,
    Aim,
    Clean,
    Hook,
    Swap,
    Shield,
    Wait,
    Book,
    Settings,
    Confirm,
    Cancel,
}

type Stroke = (f32, f32, f32, f32);

impl Glyph {
    fn strokes(self) -> &'static [Stroke] {
        match self {
            Self::Blade => &[
                (5., 19., 19., 5.),
                (15., 5., 19., 5.),
                (19., 5., 19., 9.),
                (5., 13., 11., 19.),
                (4., 20., 7., 17.),
            ],
            Self::Reach => &[
                (3., 21., 20., 4.),
                (14., 4., 20., 4.),
                (20., 4., 20., 10.),
                (5., 15., 9., 19.),
            ],
            Self::Push => &[
                (3., 12., 17., 12.),
                (12., 7., 17., 12.),
                (17., 12., 12., 17.),
                (21., 5., 21., 19.),
            ],
            Self::Heal => &[(12., 3., 12., 21.), (3., 12., 21., 12.)],
            Self::Bleed => &[
                (12., 3., 5., 14.),
                (5., 14., 6., 18.),
                (6., 18., 12., 21.),
                (12., 21., 18., 18.),
                (18., 18., 19., 14.),
                (19., 14., 12., 3.),
            ],
            Self::Arrow => &[
                (3., 18., 20., 5.),
                (13., 5., 20., 5.),
                (20., 5., 19., 12.),
                (3., 13., 8., 18.),
            ],
            Self::Aim => &[
                (4., 8., 4., 4.),
                (4., 4., 8., 4.),
                (16., 4., 20., 4.),
                (20., 4., 20., 8.),
                (20., 16., 20., 20.),
                (20., 20., 16., 20.),
                (8., 20., 4., 20.),
                (4., 20., 4., 16.),
                (8., 12., 16., 12.),
                (12., 8., 12., 16.),
            ],
            Self::Clean => &[
                (12., 2., 12., 9.),
                (12., 15., 12., 22.),
                (2., 12., 9., 12.),
                (15., 12., 22., 12.),
                (6., 6., 18., 18.),
                (6., 18., 18., 6.),
            ],
            Self::Hook => &[(3., 18., 17., 4.), (17., 4., 21., 8.), (21., 8., 17., 12.)],
            Self::Swap => &[
                (3., 8., 21., 8.),
                (17., 4., 21., 8.),
                (3., 16., 21., 16.),
                (3., 16., 7., 20.),
            ],
            Self::Shield => &[
                (5., 4., 19., 4.),
                (19., 4., 18., 15.),
                (18., 15., 12., 21.),
                (12., 21., 6., 15.),
                (6., 15., 5., 4.),
            ],
            Self::Wait => &[
                (5., 3., 19., 3.),
                (5., 21., 19., 21.),
                (7., 3., 7., 7.),
                (7., 7., 17., 17.),
                (17., 17., 17., 21.),
                (17., 3., 17., 7.),
                (17., 7., 7., 17.),
                (7., 17., 7., 21.),
            ],
            Self::Book => &[
                (4., 4., 20., 4.),
                (20., 4., 20., 20.),
                (20., 20., 4., 20.),
                (4., 20., 4., 4.),
                (8., 4., 8., 20.),
                (11., 9., 17., 9.),
                (11., 14., 17., 14.),
            ],
            Self::Settings => &[
                (4., 6., 20., 6.),
                (4., 12., 20., 12.),
                (4., 18., 20., 18.),
                (8., 3., 8., 9.),
                (16., 9., 16., 15.),
                (10., 15., 10., 21.),
            ],
            Self::Confirm => &[(3., 12., 9., 18.), (9., 18., 21., 5.)],
            Self::Cancel => &[(5., 5., 19., 19.), (19., 5., 5., 19.)],
        }
    }
}

pub(super) fn for_skill(skill: SkillId) -> Glyph {
    match skill {
        SkillId::FrontStrike
        | SkillId::DeepStrike
        | SkillId::StaffStrike
        | SkillId::BrutalStrike
        | SkillId::CrushingBlow => Glyph::Blade,
        SkillId::LongReach => Glyph::Reach,
        SkillId::DrivingBlow => Glyph::Push,
        SkillId::FieldDressing | SkillId::Mend | SkillId::Rally | SkillId::SpareBandage => {
            Glyph::Heal
        }
        SkillId::BleedingCut | SkillId::RaggedCut => Glyph::Bleed,
        SkillId::BackRankShot => Glyph::Aim,
        SkillId::ThrownKnife | SkillId::SnapShot | SkillId::HollowBolt | SkillId::HurledScrap => {
            Glyph::Arrow
        }
        SkillId::CleanBlade | SkillId::Staunch => Glyph::Clean,
        SkillId::HookShot => Glyph::Hook,
        SkillId::Exchange => Glyph::Swap,
    }
}

#[derive(Component)]
pub(super) struct GlyphInk {
    midpoint_y: f32,
}

#[derive(Component)]
pub(super) struct GlyphRoot;

pub(super) fn mount(world: &mut World, parent: Entity, glyph: Glyph) -> Entity {
    let appearance = world.resource::<LabyrinthAppearance>().clone();
    let root = column(
        world,
        parent,
        "Command Glyph",
        Node {
            width: Val::Px(26.0),
            height: Val::Px(26.0),
            flex_shrink: 0.0,
            ..default()
        },
    );
    world.entity_mut(root).insert(GlyphRoot);
    for &(x1, y1, x2, y2) in glyph.strokes() {
        let vector = Vec2::new(x2 - x1, y2 - y1);
        let length = vector.length();
        let stroke = appearance.glyph_stroke.clamp(1.0, 4.0);
        world.spawn((
            Node {
                position_type: PositionType::Absolute,
                left: Val::Percent(((x1 + x2 - length) * 0.5) / 24.0 * 100.0),
                top: Val::Percent(((y1 + y2 - stroke) * 0.5) / 24.0 * 100.0),
                width: Val::Percent(length / 24.0 * 100.0),
                height: Val::Percent(stroke / 24.0 * 100.0),
                ..default()
            },
            UiTransform::from_rotation(Rot2::radians(vector.y.atan2(vector.x))),
            BackgroundColor(appearance.ink),
            GlyphInk {
                midpoint_y: (y1 + y2) * 0.5,
            },
            ChildOf(root),
        ));
    }
    root
}

/// Only changed tokens/metrics or freshly mounted glyphs need geometry writes.
pub(super) fn refresh(
    appearance: Res<LabyrinthAppearance>,
    metrics: Res<ResolvedUiMetrics>,
    mut lines: Query<(Ref<GlyphInk>, &mut BackgroundColor, &mut Node), Without<GlyphRoot>>,
    mut roots: Query<(Ref<GlyphRoot>, &mut Node), Without<GlyphInk>>,
) {
    let thickness = if appearance.glyph_stroke.is_finite() {
        appearance.glyph_stroke.clamp(1.0, 4.0)
    } else {
        1.8
    };
    for (line, mut color, mut node) in &mut lines {
        if !appearance.is_changed() && !line.is_added() {
            continue;
        }
        color.set_if_neq(BackgroundColor(appearance.ink));
        let height = Val::Percent(thickness / 24.0 * 100.0);
        let top = Val::Percent((line.midpoint_y - thickness * 0.5) / 24.0 * 100.0);
        if node.height != height || node.top != top {
            node.height = height;
            node.top = top;
        }
    }
    for (root, mut node) in &mut roots {
        if !metrics.is_changed() && !root.is_added() {
            continue;
        }
        // Glyphs are redundant with essential labels; their optical scale grows
        // more slowly than text so the command dock retains room for the scene.
        let size = Val::Px(26.0 * metrics.content_scale.clamp(1.0, 1.25));
        if node.width != size || node.height != size {
            node.width = size;
            node.height = size;
        }
    }
}
