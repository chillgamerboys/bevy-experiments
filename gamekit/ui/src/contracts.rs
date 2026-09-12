//! Mechanics and appearance contracts, with no game fixture or renderer.

use super::*;

fn mechanics_app(skin: bool) -> App {
    let mut app = App::new();
    app.init_resource::<InputFocus>()
        .init_resource::<InputFocusVisible>()
        .init_resource::<ButtonInput<KeyCode>>()
        .init_resource::<ButtonInput<Key>>()
        .add_plugins(GameUiPlugin);
    if skin {
        app.add_plugins(GameUiSkinPlugin);
    }
    app
}

fn update_scale(app: &mut App, scale: UiScaleMode) {
    app.insert_resource(resolve_ui_metrics(Vec2::new(1920.0, 1080.0), scale));
    app.update();
}

#[derive(Resource, Default)]
struct FirstReader(Vec<Entity>);
#[derive(Resource, Default)]
struct SecondReader(Vec<Entity>);

fn read_first(mut messages: MessageReader<UiActivated>, mut seen: ResMut<FirstReader>) {
    seen.0.extend(messages.read().map(|message| message.entity));
}

fn read_second(mut messages: MessageReader<UiActivated>, mut seen: ResMut<SecondReader>) {
    seen.0.extend(messages.read().map(|message| message.entity));
}

#[test]
fn no_skin_preserves_custom_presentation_and_broadcasts_to_two_readers() {
    let mut app = mechanics_app(false);
    app.init_resource::<FirstReader>()
        .init_resource::<SecondReader>()
        .add_systems(
            Update,
            (read_first, read_second).after(GameUiSystems::EmitActivations),
        );
    let custom = Color::srgb(0.8, 0.3, 0.2);
    let entity = app
        .world_mut()
        .spawn(button("Diagnostic name"))
        .insert((
            BackgroundColor(custom),
            AccessibleLabel::new("Play the selected card"),
            Outline {
                color: custom,
                width: Val::Px(5.0),
                offset: Val::Px(1.0),
            },
            Interaction::Pressed,
        ))
        .id();
    let root = app
        .world_mut()
        .spawn(screen_root("Transparent scene overlay"))
        .add_child(entity)
        .id();
    app.update();
    assert_eq!(app.world().resource::<FirstReader>().0, vec![entity]);
    assert_eq!(app.world().resource::<SecondReader>().0, vec![entity]);
    assert_eq!(
        app.world()
            .get::<BackgroundColor>(entity)
            .expect("background")
            .0,
        custom
    );
    assert_eq!(
        app.world().get::<AccessibleLabel>(entity).expect("label").0,
        "Play the selected card"
    );
    assert_eq!(
        app.world()
            .get::<BackgroundColor>(root)
            .expect("background")
            .0,
        Color::NONE
    );
    assert_eq!(
        app.world()
            .get::<Outline>(entity)
            .expect("custom outline")
            .width,
        Val::Px(5.0)
    );
    app.world_mut().entity_mut(entity).insert(Interaction::None);
    app.world_mut()
        .resource_mut::<InputFocus>()
        .set(entity, FocusCause::Navigated);
    app.world_mut()
        .resource_mut::<ButtonInput<KeyCode>>()
        .press(KeyCode::Space);
    app.update();
    assert_eq!(
        app.world().resource::<FirstReader>().0,
        vec![entity, entity]
    );
    assert_eq!(
        app.world().resource::<SecondReader>().0,
        vec![entity, entity]
    );
    assert_eq!(
        app.world()
            .get::<BackgroundColor>(entity)
            .expect("background")
            .0,
        custom
    );
    assert!(app.world().get::<UiSkin>(entity).is_none());
}

#[test]
fn helper_geometry_does_not_impose_card_or_screen_appearance() {
    let mut app = mechanics_app(false);
    let card = app.world_mut().spawn(card("Custom card")).id();
    let modal = app.world_mut().spawn(modal("Modal scope")).id();
    app.update();
    let node = app.world().get::<Node>(card).expect("node");
    assert_eq!(node.min_width, Val::Auto);
    assert_eq!(node.min_height, Val::Auto);
    assert_eq!(
        app.world()
            .get::<BackgroundColor>(modal)
            .expect("background")
            .0,
        Color::NONE
    );
}

#[test]
fn control_and_spacing_scaling_round_trips_from_declared_baselines() {
    let mut app = mechanics_app(false);
    let control = app
        .world_mut()
        .spawn(button("Large game-owned control"))
        .insert((
            Node {
                min_width: Val::Px(180.0),
                min_height: Val::Px(52.0),
                width: Val::Percent(70.0),
                column_gap: Val::Px(7.0),
                ..default()
            },
            UiSpacing {
                padding: Some(UiInsets::axes(UiSpace::Pixels(20.0), UiSpace::Units(1.0))),
                row_gap: Some(UiSpace::Units(0.5)),
                ..default()
            },
        ))
        .id();
    app.update();
    let baseline = app.world().get::<Node>(control).expect("node").clone();
    for _ in 0..3 {
        update_scale(&mut app, UiScaleMode::Percent200);
        let node = app.world().get::<Node>(control).expect("node");
        assert_eq!(node.min_width, Val::Px(270.0));
        assert_eq!(node.min_height, Val::Px(78.0));
        assert_eq!(node.padding.left, Val::Px(25.0));
        assert_eq!(node.padding.top, Val::Px(15.0));
        assert_eq!(node.row_gap, Val::Px(7.5));
        assert_eq!(node.column_gap, Val::Px(7.0));
        assert_eq!(node.width, Val::Percent(70.0));
        update_scale(&mut app, UiScaleMode::Percent100);
        assert_eq!(app.world().get::<Node>(control).expect("node"), &baseline);
    }
    app.world_mut()
        .entity_mut(control)
        .insert(UiControlMetrics {
            min_size: Vec2::new(220.0, 20.0),
        });
    app.world_mut().resource_mut::<UiTheme>().spacing = 16.0;
    app.update();
    let node = app.world().get::<Node>(control).expect("node");
    assert_eq!(node.min_width, Val::Px(220.0));
    assert_eq!(node.min_height, Val::Px(44.0));
    assert_eq!(node.row_gap, Val::Px(8.0));
    assert_eq!(node.padding.top, Val::Px(16.0));
}

#[test]
fn newly_added_measurement_semantics_and_font_overrides_invalidate() {
    let mut app = mechanics_app(false);
    let text = app
        .world_mut()
        .spawn((
            Text::new("Essential"),
            TextFont::default(),
            TextColor(Color::BLACK),
        ))
        .id();
    app.update();
    app.world_mut().entity_mut(text).insert((
        UiTextRole::Body,
        UiTextStyle {
            base_size: Some(24.0),
            font: None,
        },
    ));
    app.update();
    assert_eq!(
        app.world().get::<TextFont>(text).expect("font").font_size,
        FontSize::Px(24.0)
    );
    update_scale(&mut app, UiScaleMode::Percent200);
    assert_eq!(
        app.world().get::<TextFont>(text).expect("font").font_size,
        FontSize::Px(48.0)
    );
    app.world_mut()
        .get_mut::<UiTextStyle>(text)
        .expect("override")
        .base_size = Some(10.0);
    update_scale(&mut app, UiScaleMode::Percent75);
    assert_eq!(
        app.world().get::<TextFont>(text).expect("font").font_size,
        FontSize::Px(18.0)
    );
    assert_eq!(
        app.world().get::<TextColor>(text).expect("color").0,
        Color::BLACK
    );
    app.world_mut().entity_mut(text).remove::<UiTextStyle>();
    update_scale(&mut app, UiScaleMode::Percent100);
    assert_eq!(
        app.world().get::<TextFont>(text).expect("font").font_size,
        FontSize::Px(20.0)
    );
    let container = app.world_mut().spawn(Node::default()).id();
    app.update();
    app.world_mut().entity_mut(container).insert(UiSpacing {
        row_gap: Some(UiSpace::Pixels(15.0)),
        ..default()
    });
    app.update();
    assert_eq!(
        app.world().get::<Node>(container).expect("node").row_gap,
        Val::Px(15.0)
    );
}

#[test]
fn font_handles_follow_changed_resources_and_explicit_overrides() {
    let mut app = mechanics_app(false);
    let heading = bevy::asset::uuid_handle!("1833da2f-c105-4bc0-9ed8-292456eb26e5");
    let body = bevy::asset::uuid_handle!("1a14def8-afd9-4595-b5ba-72009e82fa35");
    let custom = bevy::asset::uuid_handle!("f55bd42e-8e43-4a80-8191-1be2bfae1a52");
    app.insert_resource(UiFonts {
        heading: heading.clone(),
        body: body.clone(),
    });
    let entity = app
        .world_mut()
        .spawn(super::text(&UiFonts::default(), UiTextRole::Body, "Body"))
        .id();
    app.update();
    assert_eq!(
        app.world().get::<TextFont>(entity).expect("font").font,
        body.clone().into()
    );
    app.world_mut().entity_mut(entity).insert(UiTextStyle {
        font: Some(custom.clone()),
        ..default()
    });
    app.update();
    assert_eq!(
        app.world().get::<TextFont>(entity).expect("font").font,
        custom.clone().into()
    );
    app.world_mut().resource_mut::<UiFonts>().body = heading.clone();
    app.update();
    assert_eq!(
        app.world().get::<TextFont>(entity).expect("font").font,
        custom.into()
    );
    app.world_mut().entity_mut(entity).remove::<UiTextStyle>();
    app.update();
    assert_eq!(
        app.world().get::<TextFont>(entity).expect("font").font,
        heading.into()
    );
}

#[test]
fn controls_and_fields_preserve_labels_and_share_modal_keyboard_eligibility() {
    let mut app = mechanics_app(false);
    let outside = app
        .world_mut()
        .spawn(button("Outside"))
        .insert(TabIndex(7))
        .id();
    let scope = app.world_mut().spawn(modal("Modal")).id();
    let later = app
        .world_mut()
        .spawn((button("Later"), ChildOf(scope)))
        .insert(TabIndex(8))
        .id();
    let field = app
        .world_mut()
        .spawn((
            text_field(&UiFonts::default(), "Internal", "", 32),
            ChildOf(scope),
        ))
        .insert((AccessibleLabel::new("Player name"), TabIndex(2)))
        .id();
    let skipped = app
        .world_mut()
        .spawn((button("Skipped"), ChildOf(scope)))
        .insert(TabIndex(-1))
        .id();
    app.update();
    assert_eq!(app.world().resource::<InputFocus>().get(), Some(field));
    assert_eq!(app.world().get::<TabIndex>(outside).expect("tab").0, -1);
    assert_eq!(app.world().get::<TabIndex>(skipped).expect("tab").0, -1);
    assert_eq!(
        app.world().get::<AccessibleLabel>(field).expect("label").0,
        "Player name"
    );
    app.world_mut()
        .entity_mut(field)
        .insert(InteractionDisabled);
    app.update();
    assert_eq!(app.world().resource::<InputFocus>().get(), Some(later));
    assert_eq!(app.world().get::<TabIndex>(field).expect("tab").0, -1);
    app.world_mut().entity_mut(later).insert(Visibility::Hidden);
    app.update();
    assert_eq!(app.world().resource::<InputFocus>().get(), None);
    app.world_mut().despawn(scope);
    app.update();
    assert_eq!(app.world().get::<TabIndex>(outside).expect("tab").0, 7);
    app.world_mut().entity_mut(outside).insert(UiTabOrder(3));
    app.update();
    assert_eq!(app.world().get::<TabIndex>(outside).expect("tab").0, 3);
}

#[test]
fn submission_uses_the_same_eligibility_contract() {
    let mut app = mechanics_app(false);
    let field = app
        .world_mut()
        .spawn(text_field(&UiFonts::default(), "Field", "example", 32))
        .id();
    app.update();
    app.world_mut()
        .resource_mut::<InputFocus>()
        .set(field, FocusCause::Navigated);
    app.world_mut()
        .resource_mut::<ButtonInput<Key>>()
        .press(Key::Enter);
    app.world_mut().entity_mut(field).insert(UiDisabled);
    emit_text_field_messages(app.world_mut());
    assert!(app
        .world()
        .resource::<Messages<UiTextSubmitted>>()
        .is_empty());
    app.world_mut().entity_mut(field).remove::<UiDisabled>();
    emit_text_field_messages(app.world_mut());
    assert_eq!(app.world().resource::<Messages<UiTextSubmitted>>().len(), 1);
}

#[derive(Resource, Default)]
struct ChangedOutputs(usize);

fn count_changed_outputs(
    outputs: Query<
        Entity,
        Or<(
            Changed<BackgroundColor>,
            Changed<BorderColor>,
            Changed<TextColor>,
            Changed<TextFont>,
            Changed<Node>,
            Changed<Outline>,
        )>,
    >,
    mut count: ResMut<ChangedOutputs>,
) {
    count.0 = outputs.iter().count();
}

#[test]
fn opt_in_skin_supports_contrasting_surfaces_and_never_rewrites_unchanged_outputs() {
    let mut app = mechanics_app(true);
    app.init_resource::<ChangedOutputs>().add_systems(
        PostUpdate,
        count_changed_outputs.after(GameUiSystems::Style),
    );
    let custom = app
        .world_mut()
        .spawn(button("Unskinned custom control"))
        .insert(BackgroundColor(Color::WHITE))
        .id();
    let neutral = app
        .world_mut()
        .spawn((button("Neutral"), UiSkin::Control))
        .id();
    let light = app
        .world_mut()
        .spawn((
            panel("Light dialogue"),
            UiSkin::Panel,
            UiSkinOverrides {
                background: Some(Color::WHITE),
                border: Some(Color::BLACK),
                ..default()
            },
        ))
        .id();
    let text = app
        .world_mut()
        .spawn((
            super::text(&UiFonts::default(), UiTextRole::Body, "Black dialogue"),
            UiSkin::Text,
            UiSkinOverrides {
                text: Some(Color::BLACK),
                ..default()
            },
            ChildOf(light),
        ))
        .id();
    app.update();
    assert_eq!(
        app.world()
            .get::<BackgroundColor>(neutral)
            .expect("background")
            .0,
        UiTheme::default().control
    );
    assert_eq!(
        app.world()
            .get::<BackgroundColor>(custom)
            .expect("background")
            .0,
        Color::WHITE
    );
    assert_eq!(
        app.world()
            .get::<BackgroundColor>(light)
            .expect("background")
            .0,
        Color::WHITE
    );
    assert_eq!(
        app.world().get::<TextColor>(text).expect("text").0,
        Color::BLACK
    );
    app.update();
    assert_eq!(app.world().resource::<ChangedOutputs>().0, 0);
    app.world_mut().resource_mut::<UiTheme>().control = Color::srgb(0.2, 0.1, 0.0);
    app.update();
    assert_eq!(
        app.world()
            .get::<BackgroundColor>(neutral)
            .expect("background")
            .0,
        Color::srgb(0.2, 0.1, 0.0)
    );
    assert_eq!(
        app.world()
            .get::<BackgroundColor>(light)
            .expect("background")
            .0,
        Color::WHITE
    );
    app.update();
    assert_eq!(app.world().resource::<ChangedOutputs>().0, 0);
}

#[test]
fn added_skin_and_changed_or_removed_overrides_repaint_without_touching_bare_actions() {
    let mut app = mechanics_app(true);
    let button = app.world_mut().spawn(button("Late skin")).id();
    app.update();
    app.world_mut()
        .resource_mut::<InputFocus>()
        .set(button, FocusCause::Navigated);
    app.world_mut().resource_mut::<InputFocusVisible>().0 = true;
    app.update();
    assert!(app.world().get::<Outline>(button).is_none());
    app.world_mut().entity_mut(button).insert((
        UiSkin::Control,
        UiSkinOverrides {
            background: Some(Color::WHITE),
            focus: Some(Color::BLACK),
            ..default()
        },
    ));
    app.update();
    assert_eq!(
        app.world()
            .get::<BackgroundColor>(button)
            .expect("background")
            .0,
        Color::WHITE
    );
    assert_eq!(
        app.world().get::<Outline>(button).expect("outline").color,
        Color::BLACK
    );
    app.world_mut()
        .get_mut::<UiSkinOverrides>(button)
        .expect("override")
        .background = Some(Color::BLACK);
    app.update();
    assert_eq!(
        app.world()
            .get::<BackgroundColor>(button)
            .expect("background")
            .0,
        Color::BLACK
    );
    app.world_mut()
        .entity_mut(button)
        .remove::<UiSkinOverrides>();
    app.update();
    assert_eq!(
        app.world()
            .get::<BackgroundColor>(button)
            .expect("background")
            .0,
        UiTheme::default().control
    );
    app.world_mut().entity_mut(button).insert(UiDisabled);
    app.update();
    assert_eq!(
        app.world()
            .get::<BackgroundColor>(button)
            .expect("background")
            .0,
        UiTheme::default().control_disabled
    );
    assert!(app.world().get::<Outline>(button).is_none());
    app.world_mut().entity_mut(button).remove::<UiDisabled>();
    app.update();
    assert_eq!(
        app.world()
            .get::<BackgroundColor>(button)
            .expect("background")
            .0,
        UiTheme::default().control
    );
}
