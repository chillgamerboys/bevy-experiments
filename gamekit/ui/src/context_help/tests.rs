//! Selection mechanics, not a substitute for a consumer's rendered/input walk.

use super::*;
use crate::{button, modal, GameUiPlugin, UiActivated, UiDisabled};
use bevy::input::keyboard::Key;
use bevy::input_focus::{FocusCause, InputFocusVisible};
use bevy::math::Affine2;
use bevy::ui::InteractionDisabled;

fn app() -> App {
    let mut app = App::new();
    app.init_resource::<InputFocus>()
        .init_resource::<InputFocusVisible>()
        .init_resource::<ButtonInput<KeyCode>>()
        .init_resource::<ButtonInput<Key>>()
        .add_plugins((GameUiPlugin, GameUiContextHelpPlugin));
    app
}

fn source(app: &mut App, name: &str) -> Entity {
    app.world_mut()
        .spawn((
            button(name.to_owned()),
            UiContextHelp {
                title: name.to_owned(),
                body: format!("Inspect {name}"),
            },
        ))
        // These are layout fixtures for selection-mechanics tests. The consumer
        // separately tests actual layout and native cursor/keyboard events.
        .insert((
            ComputedNode {
                size: Vec2::new(80.0, 44.0),
                ..default()
            },
            UiGlobalTransform::from_xy(100.0, 100.0),
        ))
        .id()
}

fn focus(app: &mut App, entity: Entity) {
    app.world_mut()
        .resource_mut::<InputFocus>()
        .set(entity, FocusCause::Navigated);
}

fn selected(app: &App) -> Option<Entity> {
    app.world().resource::<UiContextHelpState>().entity
}

#[test]
fn installing_mechanics_does_not_implicitly_install_context_help() {
    let mut app = App::new();
    app.add_plugins(GameUiPlugin);
    assert!(!app.world().contains_resource::<UiContextHelpState>());
}

#[test]
fn focus_wins_over_stationary_hover_until_fresh_pointer_activity() {
    let mut app = app();
    let card = source(&mut app, "Deckbuilder card");
    let movement = source(&mut app, "Carterfight move");
    let window = app.world_mut().spawn(Window::default()).id();
    app.world_mut()
        .get_mut::<Window>(window)
        .expect("window")
        .set_cursor_position(Some(Vec2::new(100.0, 100.0)));
    app.world_mut()
        .entity_mut(card)
        .insert(Interaction::Hovered);
    app.update();
    assert_eq!(selected(&app), Some(card));
    assert_eq!(app.world().resource::<InputFocus>().get(), None);

    focus(&mut app, movement);
    app.update();
    assert_eq!(selected(&app), Some(movement));
    app.update();
    assert_eq!(selected(&app), Some(movement));

    // Moving within the same control must not leave keyboard help stuck on top.
    app.world_mut()
        .get_mut::<Window>(window)
        .expect("window")
        .set_cursor_position(Some(Vec2::new(101.0, 100.0)));
    app.update();
    assert_eq!(selected(&app), Some(card));
    assert_eq!(app.world().resource::<InputFocus>().get(), Some(movement));
    app.world_mut().entity_mut(card).insert(Interaction::None);
    app.update();
    assert_eq!(selected(&app), Some(movement));
    assert!(app.world().resource::<Messages<UiActivated>>().is_empty());
}

#[test]
fn focus_without_help_does_not_revive_an_old_hover() {
    let mut app = app();
    let old_hover = source(&mut app, "Old hover");
    let plain = app.world_mut().spawn(button("Plain action")).id();
    app.world_mut()
        .entity_mut(old_hover)
        .insert(Interaction::Hovered);
    app.update();
    assert_eq!(selected(&app), Some(old_hover));
    focus(&mut app, plain);
    app.update();
    assert_eq!(selected(&app), None);
    app.update();
    assert_eq!(selected(&app), None);
}

#[test]
fn help_neither_steals_focus_nor_changes_activation_parity() {
    let mut app = app();
    let control = source(&mut app, "Playable ability");
    focus(&mut app, control);
    app.world_mut()
        .resource_mut::<ButtonInput<KeyCode>>()
        .press(KeyCode::Enter);
    let mut cursor = app.world().resource::<Messages<UiActivated>>().get_cursor();
    app.update();
    assert_eq!(
        cursor
            .read(app.world().resource::<Messages<UiActivated>>())
            .map(|message| message.entity)
            .collect::<Vec<_>>(),
        vec![control]
    );
    assert_eq!(selected(&app), Some(control));
    assert_eq!(app.world().resource::<InputFocus>().get(), Some(control));

    app.world_mut()
        .resource_mut::<ButtonInput<KeyCode>>()
        .clear();
    app.world_mut()
        .entity_mut(control)
        .insert(Interaction::Pressed);
    app.update();
    assert_eq!(
        cursor
            .read(app.world().resource::<Messages<UiActivated>>())
            .map(|message| message.entity)
            .collect::<Vec<_>>(),
        vec![control]
    );
    app.world_mut().entity_mut(control).insert(UiDisabled);
    app.world_mut()
        .resource_mut::<ButtonInput<KeyCode>>()
        .press(KeyCode::Space);
    app.update();
    assert_eq!(selected(&app), None);
    assert_eq!(
        cursor
            .read(app.world().resource::<Messages<UiActivated>>())
            .count(),
        0
    );
}

#[test]
fn nested_modals_exclude_stale_hover_and_restore_the_prior_source() {
    let mut app = app();
    let outside = source(&mut app, "Outside");
    focus(&mut app, outside);
    app.world_mut()
        .entity_mut(outside)
        .insert(Interaction::Hovered);
    app.update();
    let scope = app.world_mut().spawn(modal("Inspector")).id();
    let inside = source(&mut app, "Inside inspector");
    app.world_mut().entity_mut(scope).add_child(inside);
    app.update();
    assert_eq!(selected(&app), Some(inside));
    let nested = app
        .world_mut()
        .spawn(modal("Nested inspector"))
        .insert(GlobalZIndex(200))
        .id();
    let topmost = source(&mut app, "Inside nested inspector");
    app.world_mut().entity_mut(nested).add_child(topmost);
    app.world_mut()
        .entity_mut(inside)
        .insert(Interaction::Hovered);
    app.update();
    assert_eq!(selected(&app), Some(topmost));
    app.world_mut().despawn(nested);
    app.update();
    assert_eq!(selected(&app), Some(inside));
    app.world_mut().despawn(scope);
    app.update();
    assert_eq!(selected(&app), Some(outside));
}

#[test]
fn hidden_disabled_removed_and_rebuilt_sources_cannot_leave_stale_help() {
    let mut app = app();
    let parent = app.world_mut().spawn(Node::default()).id();
    let control = source(&mut app, "Information");
    app.world_mut().entity_mut(parent).add_child(control);
    app.world_mut()
        .entity_mut(control)
        .insert(Interaction::Hovered);
    app.update();
    assert_eq!(selected(&app), Some(control));

    app.world_mut()
        .entity_mut(parent)
        .insert(Visibility::Hidden);
    app.update();
    assert_eq!(selected(&app), None);
    app.world_mut()
        .entity_mut(parent)
        .insert(Visibility::Visible);
    app.update();
    assert_eq!(selected(&app), Some(control));
    app.world_mut()
        .entity_mut(parent)
        .insert(InteractionDisabled);
    app.update();
    assert_eq!(selected(&app), None);
    app.world_mut()
        .entity_mut(parent)
        .remove::<InteractionDisabled>();
    app.world_mut()
        .get_mut::<Node>(parent)
        .expect("node")
        .display = Display::None;
    app.update();
    assert_eq!(selected(&app), None);
    app.world_mut()
        .get_mut::<Node>(parent)
        .expect("node")
        .display = Display::Flex;
    app.update();
    assert_eq!(selected(&app), Some(control));
    let content = app.world_mut().entity_mut(control).take::<UiContextHelp>();
    app.update();
    assert_eq!(selected(&app), None);
    app.world_mut()
        .entity_mut(control)
        .insert(content.expect("help"));
    app.update();
    assert_eq!(selected(&app), Some(control));
    app.world_mut().despawn(control);
    app.update();
    assert_eq!(selected(&app), None);
    assert_eq!(app.world().resource::<UiContextHelpState>().content, None);
}

#[test]
fn help_requires_visible_non_degenerate_geometry_and_honors_rotated_clipping() {
    let mut app = app();
    let control = source(&mut app, "Clipped information");
    app.world_mut()
        .entity_mut(control)
        .insert(Interaction::Hovered);
    app.update();
    assert_eq!(selected(&app), Some(control));
    app.world_mut().entity_mut(control).insert(CalculatedClip {
        clip: Rect::from_corners(Vec2::ZERO, Vec2::new(20.0, 20.0)),
    });
    app.update();
    assert_eq!(selected(&app), None);
    app.world_mut().entity_mut(control).insert(CalculatedClip {
        clip: Rect::from_corners(Vec2::ZERO, Vec2::new(100.0, 100.0)),
    });
    app.update();
    assert_eq!(selected(&app), Some(control));

    // A 45-degree diamond occupies the middle, not its bounding box's corners.
    app.world_mut().entity_mut(control).insert((
        ComputedNode {
            size: Vec2::splat(20.0),
            ..default()
        },
        UiGlobalTransform::from(Affine2::from_angle(std::f32::consts::FRAC_PI_4)),
        CalculatedClip {
            clip: Rect::from_corners(Vec2::splat(11.0), Vec2::splat(14.0)),
        },
    ));
    app.update();
    assert_eq!(selected(&app), None);
    app.world_mut()
        .entity_mut(control)
        .remove::<CalculatedClip>();
    app.world_mut()
        .entity_mut(control)
        .insert(UiGlobalTransform::from_scale(Vec2::new(-1.0, 1.0)));
    app.update();
    assert_eq!(selected(&app), Some(control));
    app.world_mut()
        .entity_mut(control)
        .insert(UiGlobalTransform::from_scale(Vec2::ZERO));
    app.update();
    assert_eq!(selected(&app), None);
    app.world_mut()
        .entity_mut(control)
        .insert((UiGlobalTransform::default(), ComputedNode::default()));
    app.update();
    assert_eq!(selected(&app), None);
}

#[derive(Resource, Default)]
struct HelpChanges(usize);

fn count_help_changes(help: Res<UiContextHelpState>, mut count: ResMut<HelpChanges>) {
    if help.is_changed() {
        count.0 += 1;
    }
}

#[test]
fn unchanged_frames_preserve_change_detection_but_content_and_identity_invalidate() {
    let mut app = app();
    app.init_resource::<HelpChanges>().add_systems(
        Update,
        count_help_changes.after(UiContextHelpSystems::Resolve),
    );
    let first = source(&mut app, "Same content");
    let second = source(&mut app, "Same content");
    focus(&mut app, first);
    app.update();
    assert_eq!(app.world().resource::<HelpChanges>().0, 1);
    app.update();
    app.update();
    assert_eq!(app.world().resource::<HelpChanges>().0, 1);
    app.world_mut()
        .get_mut::<UiContextHelp>(first)
        .expect("source")
        .body = "Newly disclosed information".to_owned();
    app.update();
    assert_eq!(app.world().resource::<HelpChanges>().0, 2);
    assert_eq!(
        app.world()
            .resource::<UiContextHelpState>()
            .content
            .as_ref()
            .expect("help")
            .body,
        "Newly disclosed information"
    );
    let content = app
        .world()
        .get::<UiContextHelp>(first)
        .expect("help")
        .clone();
    app.world_mut().entity_mut(second).insert(content);
    focus(&mut app, second);
    app.update();
    assert_eq!(app.world().resource::<HelpChanges>().0, 3);
    app.world_mut().despawn(second);
    app.update();
    assert_eq!(selected(&app), None);
    assert_eq!(app.world().resource::<HelpChanges>().0, 4);
    app.update();
    assert_eq!(app.world().resource::<HelpChanges>().0, 4);
}

#[test]
fn overlapping_native_hover_uses_stack_priority_without_query_order_dependence() {
    let mut app = app();
    let high = source(&mut app, "Upper pass-through region");
    let low = source(&mut app, "Lower pass-through region");
    app.world_mut()
        .entity_mut(high)
        .insert((Interaction::Hovered, ComputedStackIndex(9)));
    app.world_mut()
        .entity_mut(low)
        .insert((Interaction::Hovered, ComputedStackIndex(2)));
    app.update();
    assert_eq!(selected(&app), Some(high));
    app.world_mut().entity_mut(high).insert(UiDisabled);
    app.update();
    assert_eq!(selected(&app), Some(low));
}

#[test]
fn post_layout_validation_clears_sources_removed_after_update_selection() {
    fn remove_selected(mut commands: Commands, help: Res<UiContextHelpState>) {
        if let Some(entity) = help.entity {
            commands.entity(entity).despawn();
        }
    }
    let mut app = app();
    let control = source(&mut app, "Transient source");
    focus(&mut app, control);
    app.add_systems(Update, remove_selected.after(UiContextHelpSystems::Resolve));
    app.update();
    assert_eq!(selected(&app), None);
    assert_eq!(app.world().resource::<UiContextHelpState>().content, None);
}
