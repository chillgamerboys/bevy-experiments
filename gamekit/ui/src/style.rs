//! Explicit, replaceable neutral skin. Mechanics never call this painter.

use super::*;

/// Installs the optional neutral skin for entities explicitly carrying [`UiSkin`].
///
/// Use alongside [`GameUiPlugin`]. Unmarked controls and surfaces are untouched;
/// a game can combine skinned controls with entirely custom scene presentation.
pub struct GameUiSkinPlugin;

impl Plugin for GameUiSkinPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(PostUpdate, apply_skin.in_set(GameUiSystems::Style));
    }
}

/// Explicit appearance ownership. Helpers and action markers never add a skin.
#[derive(Component, Debug, Clone, Copy, PartialEq, Eq)]
pub enum UiSkin {
    /// Opaque screen background.
    Screen,
    /// Raised panel background and edge.
    Panel,
    /// Card background and edge; interactive when also an action.
    Card,
    /// Control background, edge, and keyboard focus outline.
    Control,
    /// Editable field background, text, edge, and focus outline.
    Field,
    /// Dimmed full-screen modal backdrop.
    Modal,
    /// Semantic text color without a background.
    Text,
}

/// Per-surface overrides for the optional skin, including interaction states.
#[derive(Component, Debug, Default, Clone, PartialEq)]
pub struct UiSkinOverrides {
    /// Resting surface color.
    pub background: Option<Color>,
    /// Hovered interactive surface color.
    pub hovered: Option<Color>,
    /// Pressed interactive surface color.
    pub pressed: Option<Color>,
    /// Disabled or otherwise unreachable interactive surface color.
    pub disabled: Option<Color>,
    /// Text color.
    pub text: Option<Color>,
    /// Border color.
    pub border: Option<Color>,
    /// Keyboard focus outline color.
    pub focus: Option<Color>,
}

#[derive(Component)]
struct SkinOutline;

// Recompute desired values to account for hierarchy/modal eligibility, without
// fragile change filters. Writes are change-driven: unchanged outputs are never
// marked changed, including pointer, focus, theme, and newly added semantics.
fn apply_skin(world: &mut World) {
    let theme = world.resource::<UiTheme>().clone();
    let focused = world.resource::<InputFocus>().get();
    let focus_visible = world.resource::<InputFocusVisible>().0;
    let entities = {
        let mut query = world.query_filtered::<Entity, With<UiSkin>>();
        query.iter(world).collect::<Vec<_>>()
    };
    for entity in entities {
        let Some(skin) = world.get::<UiSkin>(entity).copied() else {
            continue;
        };
        let overrides = world
            .get::<UiSkinOverrides>(entity)
            .cloned()
            .unwrap_or_default();
        let interactive =
            world.get::<UiAction>(entity).is_some() || world.get::<UiTextField>(entity).is_some();
        let reachable = !interactive || activation_eligible(world, entity);
        if skin != UiSkin::Text {
            let base = overrides.background.unwrap_or(match skin {
                UiSkin::Screen => theme.background,
                UiSkin::Panel => theme.panel,
                UiSkin::Card => theme.card,
                UiSkin::Modal => Color::srgba(0.0, 0.0, 0.0, 0.72),
                UiSkin::Control | UiSkin::Field | UiSkin::Text => theme.control,
            });
            let color = if !reachable {
                overrides.disabled.unwrap_or(theme.control_disabled)
            } else if interactive {
                match world
                    .get::<Interaction>(entity)
                    .copied()
                    .unwrap_or_default()
                {
                    Interaction::Hovered => overrides.hovered.unwrap_or(theme.control_hovered),
                    Interaction::Pressed => overrides.pressed.unwrap_or(theme.control_pressed),
                    Interaction::None => base,
                }
            } else {
                base
            };
            set_component(world, entity, BackgroundColor(color));
        }
        if matches!(
            skin,
            UiSkin::Panel | UiSkin::Card | UiSkin::Control | UiSkin::Field
        ) {
            set_component(
                world,
                entity,
                BorderColor::all(overrides.border.unwrap_or(theme.edge)),
            );
        }
        if matches!(skin, UiSkin::Text | UiSkin::Field) {
            let color = overrides
                .text
                .unwrap_or(match world.get::<UiTextRole>(entity) {
                    Some(UiTextRole::Title) => theme.accent,
                    Some(UiTextRole::Supporting | UiTextRole::Metadata) => theme.muted_text,
                    _ => theme.text,
                });
            set_component(world, entity, TextColor(color));
        }
        if interactive && reachable && focus_visible && focused == Some(entity) {
            set_component(
                world,
                entity,
                Outline {
                    color: overrides.focus.unwrap_or(theme.accent),
                    width: Val::Px(3.0),
                    offset: Val::Px(2.0),
                },
            );
            if world.get::<SkinOutline>(entity).is_none() {
                world.entity_mut(entity).insert(SkinOutline);
            }
        } else if world.get::<SkinOutline>(entity).is_some() {
            world.entity_mut(entity).remove::<(Outline, SkinOutline)>();
        }
    }
    // Relinquishing a skin leaves colors in place but removes its transient
    // focus ring. A custom outline on an unskinned control is never removed.
    let stale = {
        let mut query = world.query_filtered::<Entity, (With<SkinOutline>, Without<UiSkin>)>();
        query.iter(world).collect::<Vec<_>>()
    };
    for entity in stale {
        world.entity_mut(entity).remove::<(Outline, SkinOutline)>();
    }
}

fn set_component<T: Component<Mutability = bevy::ecs::component::Mutable> + PartialEq>(
    world: &mut World,
    entity: Entity,
    value: T,
) {
    if let Some(mut current) = world.get_mut::<T>(entity) {
        current.set_if_neq(value);
    } else {
        world.entity_mut(entity).insert(value);
    }
}
