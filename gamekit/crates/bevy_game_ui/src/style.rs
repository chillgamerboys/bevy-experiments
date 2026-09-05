//! Change-driven semantic and interaction styling.

use super::*;

pub(crate) fn paint_keyboard_focus(
    focus: Res<InputFocus>,
    visible: Res<InputFocusVisible>,
    theme: Res<UiTheme>,
    actions: Query<Entity, Or<(With<UiAction>, With<UiTextField>)>>,
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

pub(crate) fn paint_interactions(
    theme: Res<UiTheme>,
    mut actions: Query<
        (
            Entity,
            Ref<Interaction>,
            Option<Ref<UiDisabled>>,
            &mut BackgroundColor,
        ),
        With<UiAction>,
    >,
    mut removed: RemovedComponents<UiDisabled>,
) {
    let enabled = removed.read().collect::<Vec<_>>();
    for (entity, interaction, disabled, mut background) in &mut actions {
        if !theme.is_changed()
            && !interaction.is_changed()
            && !background.is_added()
            && !disabled
                .as_ref()
                .is_some_and(|disabled| disabled.is_added())
            && !enabled.contains(&entity)
        {
            continue;
        }
        background.0 = if disabled.is_some() {
            theme.control_disabled
        } else {
            match *interaction {
                Interaction::Pressed => theme.control_pressed,
                Interaction::Hovered => theme.control_hovered,
                Interaction::None => theme.control,
            }
        };
    }
}

pub(crate) fn apply_semantic_style(
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
    mut actions: Query<
        &mut BorderColor,
        (
            Or<(With<UiAction>, With<UiTextField>)>,
            Without<UiPanel>,
            Without<UiCard>,
        ),
    >,
    mut text_query: Query<(Ref<UiTextRole>, &mut TextFont, &mut TextColor)>,
    mut controls: Query<(&mut Node, &mut ResponsiveControl)>,
) {
    for mut background in &mut roots {
        if !theme.is_changed() && !background.is_added() {
            continue;
        }
        background.0 = theme.background;
    }
    for (mut background, mut border) in &mut panels {
        if !theme.is_changed() && !background.is_added() {
            continue;
        }
        background.0 = theme.panel;
        *border = BorderColor::all(theme.edge);
    }
    for (mut background, mut border) in &mut cards {
        if !theme.is_changed() && !background.is_added() {
            continue;
        }
        background.0 = theme.card;
        *border = BorderColor::all(theme.edge);
    }
    for mut border in &mut actions {
        if !theme.is_changed() && !border.is_added() {
            continue;
        }
        *border = BorderColor::all(theme.edge);
    }
    for (role, mut font, mut color) in &mut text_query {
        if !theme.is_changed() && !metrics.is_changed() && !role.is_changed() && !font.is_added() {
            continue;
        }
        let (size, wanted_color) = match *role {
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
        if !control.is_added() && !metrics.is_changed() {
            continue;
        }
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

pub(crate) fn apply_text_field_style(
    theme: Res<UiTheme>,
    metrics: Res<ResolvedUiMetrics>,
    mut fields: Query<(&mut BackgroundColor, &mut TextColor, &mut TextFont), With<UiTextField>>,
) {
    for (mut background, mut color, mut font) in &mut fields {
        if !theme.is_changed() && !metrics.is_changed() && !background.is_added() {
            continue;
        }
        background.0 = theme.control;
        color.0 = theme.text;
        font.font_size = FontSize::Px((theme.body_size * metrics.content_scale).max(18.0));
    }
}
