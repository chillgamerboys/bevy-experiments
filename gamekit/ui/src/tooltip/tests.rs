use super::*;

fn key(value: &str) -> UiTooltipSubject {
    UiTooltipSubject(value.to_owned())
}

fn close_action(app: &mut App, depth: usize) -> Entity {
    app.world_mut()
        .query::<(Entity, &view::TooltipAction)>()
        .iter(app.world())
        .find_map(|(entity, action)| {
            matches!(action, view::TooltipAction::Close(index) if *index == depth).then_some(entity)
        })
        .expect("card close control")
}

#[test]
fn immediate_preview_locks_only_after_continuous_hover_and_stays_until_dismissed() {
    let settings = UiTooltipSettings::default();
    let mut state = UiTooltipState::default();
    state.hover(Some(key("ability")), Duration::from_millis(349), &settings);
    assert_eq!(state.subjects(), &[key("ability")]);
    assert!(
        !state.is_pinned(),
        "first sample cannot count preceding frame time"
    );
    state.hover(
        Some(key("ability")),
        Duration::from_millis(1_999),
        &settings,
    );
    assert_eq!(state.subjects(), &[key("ability")]);
    assert!(!state.is_pinned());
    state.hover(Some(key("ability")), Duration::from_millis(1), &settings);
    assert!(state.is_pinned());
    state.follow(0, key("condition"), 4);
    state.follow(1, key("term"), 4);
    state.hover(None, Duration::from_secs(100), &settings);
    state.hover(Some(key("other")), Duration::from_secs(100), &settings);
    state.hover(Some(key("ability")), Duration::from_secs(100), &settings);
    assert_eq!(
        state.subjects(),
        &[key("ability"), key("condition"), key("term")]
    );
    assert!(state.is_pinned());
    state.chain.pop();
    assert_eq!(state.subjects(), &[key("ability"), key("condition")]);
    state.dismiss();
    assert!(!state.is_pinned());
}

#[test]
fn short_hover_leaves_immediately_and_new_sources_reset_lock_timer() {
    let settings = UiTooltipSettings::default();
    let mut state = UiTooltipState::default();
    state.hover(Some(key("a")), Duration::ZERO, &settings);
    state.hover(Some(key("a")), Duration::from_millis(999), &settings);
    state.hover(None, Duration::ZERO, &settings);
    assert!(state.subjects().is_empty());
    state.hover(Some(key("a")), Duration::ZERO, &settings);
    state.hover(Some(key("a")), Duration::from_millis(999), &settings);
    state.hover(Some(key("b")), Duration::from_secs(60), &settings);
    assert_eq!(state.subjects(), &[key("b")]);
    assert!(!state.is_pinned(), "a new source starts its own lock timer");
    state.hover(None, Duration::ZERO, &settings);
    assert!(
        state.subjects().is_empty(),
        "preview cannot capture the pointer"
    );
}

#[test]
fn branches_cycles_and_depth_are_bounded() {
    let mut state = UiTooltipState {
        chain: vec![key("root")],
        ..default()
    };
    state.follow(0, key("a"), 3);
    state.follow(1, key("b"), 3);
    state.follow(2, key("c"), 3);
    assert_eq!(state.subjects().len(), 3);
    state.follow(0, key("replacement"), 3);
    assert_eq!(state.subjects(), &[key("root"), key("replacement")]);
    state.follow(1, key("root"), 3);
    assert_eq!(state.subjects(), &[key("root")]);
}

#[test]
fn closing_under_stationary_pointer_does_not_immediately_reopen() {
    let mut state = UiTooltipState::default();
    let settings = UiTooltipSettings::default();
    state.hover(Some(key("root")), Duration::from_secs(1), &settings);
    state.suppressed = Some(key("root"));
    state.dismiss();
    state.hover(Some(key("root")), Duration::from_secs(1), &settings);
    assert!(state.subjects().is_empty());
    state.hover(None, Duration::from_secs(1), &settings);
    state.hover(Some(key("root")), Duration::from_secs(1), &settings);
    assert_eq!(state.subjects(), &[key("root")]);
}

// ECS lifecycle fixtures deliberately supply geometry without a renderer. Native
// picking/layout and gameplay-intent containment are tested by the adopters.
fn app() -> (App, Entity) {
    use bevy::{input::keyboard::Key, input_focus::InputFocusVisible};
    let mut app = App::new();
    app.init_resource::<InputFocus>()
        .init_resource::<InputFocusVisible>()
        .init_resource::<ButtonInput<KeyCode>>()
        .init_resource::<ButtonInput<Key>>()
        .init_resource::<Time<Real>>()
        .add_plugins((crate::GameUiPlugin, GameUiTooltipPlugin));
    let host = app
        .world_mut()
        .spawn((crate::screen_root("fixture"), UiTooltipHost))
        .insert((
            ComputedNode {
                size: Vec2::new(1280.0, 720.0),
                ..default()
            },
            UiGlobalTransform::from_xy(640.0, 360.0),
        ))
        .id();
    app.world_mut().resource_mut::<UiTooltipCatalog>().0.insert(
        key("root"),
        UiTooltipContent {
            title: "Card".to_owned(),
            body: "Current information".to_owned(),
            links: vec![UiTooltipLink {
                label: "Term".to_owned(),
                subject: key("term"),
            }],
            ..default()
        },
    );
    app.world_mut().resource_mut::<UiTooltipCatalog>().0.insert(
        key("term"),
        UiTooltipContent {
            title: "Term".to_owned(),
            body: "Definition".to_owned(),
            ..default()
        },
    );
    let anchor = app
        .world_mut()
        .spawn((
            crate::button("source"),
            UiTooltipSource(key("root")),
            UiInspectable,
            crate::UiFocusId::new("fixture", "source"),
            ChildOf(host),
        ))
        .insert((
            ComputedNode {
                size: Vec2::new(80.0, 44.0),
                ..default()
            },
            UiGlobalTransform::from_xy(100.0, 100.0),
        ))
        .id();
    (app, anchor)
}

fn step(app: &mut App, millis: u64) {
    app.world_mut()
        .resource_mut::<Time<Real>>()
        .advance_by(Duration::from_millis(millis));
    app.update();
    *app.world_mut().resource_mut::<ButtonInput<KeyCode>>() = ButtonInput::default();
}

#[test]
fn using_a_control_suppresses_its_hint_until_hover_leaves() {
    for initial_dwell in [1, 400] {
        let (mut app, anchor) = app();
        app.world_mut()
            .entity_mut(anchor)
            .insert(UiTooltipDismissOnActivate);
        app.world_mut()
            .entity_mut(anchor)
            .remove::<UiTooltipSource>()
            .insert(UiContextHelp {
                title: "Combat log".to_owned(),
                body: String::new(),
            });
        app.world_mut()
            .entity_mut(anchor)
            .insert(Interaction::Hovered);
        step(&mut app, initial_dwell);
        app.world_mut()
            .write_message(UiActivated { entity: anchor });
        step(&mut app, 1);
        step(&mut app, 2000);
        assert!(app
            .world()
            .resource::<UiTooltipState>()
            .subjects()
            .is_empty());
        app.world_mut().entity_mut(anchor).insert(Interaction::None);
        step(&mut app, 500);
        app.world_mut()
            .entity_mut(anchor)
            .insert(Interaction::Hovered);
        step(&mut app, 400);
        assert_eq!(app.world().resource::<UiTooltipState>().subjects().len(), 1);
    }
}

#[test]
fn using_a_control_preserves_deliberately_pinned_inspection() {
    let (mut app, anchor) = app();
    app.world_mut()
        .entity_mut(anchor)
        .insert(UiTooltipDismissOnActivate);
    app.world_mut()
        .entity_mut(anchor)
        .insert(Interaction::Hovered);
    step(&mut app, 400);
    app.world_mut().write_message(UiTooltipRequest::Pin);
    step(&mut app, 1);
    app.world_mut()
        .write_message(UiActivated { entity: anchor });
    step(&mut app, 1);
    let state = app.world().resource::<UiTooltipState>();
    assert!(state.is_pinned());
    assert_eq!(state.subjects(), &[key("root")]);
}

#[test]
fn preview_is_pointer_transparent_and_pin_exposes_accessible_corner_close() {
    let (mut app, anchor) = app();
    app.world_mut()
        .entity_mut(anchor)
        .insert(Interaction::Hovered);
    step(&mut app, 1);
    let preview = app
        .world_mut()
        .query_filtered::<Entity, With<view::TooltipSurface>>()
        .single(app.world())
        .expect("preview on first frame");
    assert!(app.world().get::<Interaction>(preview).is_none());
    assert_eq!(
        app.world().get::<Pickable>(preview),
        Some(&Pickable::IGNORE)
    );
    assert_eq!(
        app.world_mut()
            .query::<&view::TooltipAction>()
            .iter(app.world())
            .count(),
        0
    );
    assert!(!app.world().resource::<UiTooltipState>().is_pinned());
    step(&mut app, 999);
    assert!(!app.world().resource::<UiTooltipState>().is_pinned());
    step(&mut app, 1);
    assert!(app.world().resource::<UiTooltipState>().is_pinned());
    let locked = app
        .world_mut()
        .query_filtered::<Entity, With<view::TooltipSurface>>()
        .single(app.world())
        .expect("locked card");
    assert_eq!(
        app.world()
            .get::<crate::UiSkinOverrides>(locked)
            .expect("border")
            .border,
        Some(app.world().resource::<crate::UiTheme>().accent)
    );
    let actions = app
        .world_mut()
        .query::<&view::TooltipAction>()
        .iter(app.world())
        .collect::<Vec<_>>();
    assert_eq!(actions.len(), 2, "related term and close are actionable");
    assert!(actions.iter().any(
        |action| matches!(action, view::TooltipAction::Link(0, subject) if subject == &key("term"))
    ));
    assert!(!app
        .world_mut()
        .query::<&Text>()
        .iter(app.world())
        .any(|text| matches!(text.0.as_str(), "Pin" | "Unpin" | "Close" | "Pin · T")));
    assert!(app
        .world_mut()
        .query::<&Text>()
        .iter(app.world())
        .any(|text| text.0 == "x"));
    let close = close_action(&mut app, 0);
    assert_eq!(
        app.world()
            .get::<AccessibleLabel>(close)
            .expect("close label")
            .0,
        "Close Card tooltip"
    );
    let node = app.world().get::<Node>(close).expect("close geometry");
    assert_eq!(node.position_type, PositionType::Absolute);
    assert_eq!(node.right, Val::Px(0.0));
    assert_eq!(node.top, Val::Px(0.0));
    assert_eq!(node.width, Val::Px(44.0));
    assert_eq!(node.height, Val::Px(44.0));
    let title = app
        .world_mut()
        .query::<(Entity, &Name)>()
        .iter(app.world())
        .find_map(|(entity, name)| (name.as_str() == "Tooltip Title").then_some(entity))
        .expect("title");
    let heading = app.world().get::<ChildOf>(title).expect("heading").parent();
    assert_eq!(
        app.world()
            .get::<Node>(heading)
            .expect("heading layout")
            .padding,
        UiRect::right(Val::Px(44.0))
    );
    settle_fixture_cards(&mut app);
    app.world_mut()
        .entity_mut(close)
        .insert(Interaction::Pressed);
    step(&mut app, 1);
    step(&mut app, 1000);
    assert!(app
        .world()
        .resource::<UiTooltipState>()
        .subjects()
        .is_empty());
}

#[test]
fn pointer_and_keyboard_close_remove_the_selected_branch_and_restore_focus() {
    for keyboard in [false, true] {
        let (mut app, anchor) = app();
        app.world_mut().resource_mut::<UiTooltipCatalog>().0.insert(
            key("detail"),
            UiTooltipContent {
                title: "Detail".to_owned(),
                ..default()
            },
        );
        app.world_mut()
            .resource_mut::<UiTooltipCatalog>()
            .0
            .get_mut(&key("term"))
            .expect("term")
            .links
            .push(UiTooltipLink {
                label: "Detail".to_owned(),
                subject: key("detail"),
            });
        app.world_mut()
            .resource_mut::<InputFocus>()
            .set(anchor, bevy::input_focus::FocusCause::Navigated);
        if keyboard {
            app.world_mut()
                .resource_mut::<ButtonInput<KeyCode>>()
                .press(KeyCode::KeyT);
            step(&mut app, 1);
        } else {
            app.world_mut()
                .entity_mut(anchor)
                .insert(Interaction::Hovered);
            step(&mut app, 1);
            app.world_mut().write_message(UiTooltipRequest::Pin);
            step(&mut app, 1);
        }
        for depth in [0, 1] {
            let link = app
                .world_mut()
                .query::<(Entity, &view::TooltipAction)>()
                .iter(app.world())
                .find_map(|(entity, action)| {
                    matches!(action, view::TooltipAction::Link(index, _) if *index == depth)
                        .then_some(entity)
                })
                .expect("related term");
            app.world_mut().write_message(UiActivated { entity: link });
            step(&mut app, 1);
        }
        assert_eq!(
            app.world().resource::<UiTooltipState>().subjects(),
            &[key("root"), key("term"), key("detail")]
        );
        for (depth, code, expected) in [
            (1, KeyCode::Enter, vec![key("root")]),
            (0, KeyCode::Space, vec![]),
        ] {
            settle_fixture_cards(&mut app);
            let close = close_action(&mut app, depth);
            app.world_mut()
                .resource_mut::<InputFocus>()
                .set(close, bevy::input_focus::FocusCause::Navigated);
            if keyboard {
                app.world_mut()
                    .resource_mut::<ButtonInput<KeyCode>>()
                    .press(code);
            } else {
                app.world_mut()
                    .entity_mut(close)
                    .insert(Interaction::Pressed);
            }
            step(&mut app, 1);
            assert_eq!(
                app.world().resource::<UiTooltipState>().subjects(),
                expected
            );
            assert_eq!(
                app.world().resource::<UiTooltipState>().is_pinned(),
                depth != 0
            );
            if keyboard {
                assert!(
                    app.world().resource::<UiTooltipState>().captures_keyboard(),
                    "closing key cannot reach game shortcuts"
                );
            }
        }
        assert_eq!(app.world().resource::<InputFocus>().get(), Some(anchor));
        step(&mut app, 1);
        assert!(!app.world().resource::<UiTooltipState>().captures_keyboard());
    }
}

#[test]
fn closing_a_card_does_not_preview_an_exposed_source_until_the_pointer_moves() {
    let (mut app, anchor) = app();
    let mut window = Window::default();
    window.set_cursor_position(Some(Vec2::new(100.0, 100.0)));
    let window = app.world_mut().spawn(window).id();
    app.world_mut()
        .entity_mut(anchor)
        .insert(Interaction::Hovered);
    step(&mut app, 1);
    app.world_mut().write_message(UiTooltipRequest::Pin);
    step(&mut app, 1);
    settle_fixture_cards(&mut app);
    let close = close_action(&mut app, 0);
    app.world_mut().entity_mut(anchor).insert(Interaction::None);
    app.world_mut()
        .entity_mut(close)
        .insert(Interaction::Pressed);
    step(&mut app, 1);
    app.world_mut()
        .entity_mut(anchor)
        .insert((UiTooltipSource(key("term")), Interaction::Hovered));
    step(&mut app, 2000);
    assert!(
        app.world()
            .resource::<UiTooltipState>()
            .subjects()
            .is_empty(),
        "revealed geometry is not fresh hover intent"
    );
    app.world_mut()
        .get_mut::<Window>(window)
        .expect("window")
        .set_cursor_position(Some(Vec2::new(101.0, 100.0)));
    step(&mut app, 1);
    assert_eq!(
        app.world().resource::<UiTooltipState>().subjects(),
        &[key("term")]
    );
    assert!(!app.world().resource::<UiTooltipState>().is_pinned());
}

#[test]
fn pointer_exit_never_falls_back_to_a_clicked_controls_focus() {
    let (mut app, anchor) = app();
    app.world_mut()
        .entity_mut(anchor)
        .insert(Interaction::Hovered);
    app.world_mut()
        .resource_mut::<InputFocus>()
        .set(anchor, bevy::input_focus::FocusCause::Pressed);
    step(&mut app, 1);
    assert_eq!(
        app.world().resource::<UiTooltipState>().subjects(),
        &[key("root")]
    );
    app.world_mut().entity_mut(anchor).insert(Interaction::None);
    step(&mut app, 1);
    assert_eq!(app.world().resource::<InputFocus>().get(), Some(anchor));
    assert!(app
        .world()
        .resource::<UiTooltipState>()
        .subjects()
        .is_empty());
    step(&mut app, 1000);
    assert!(app
        .world()
        .resource::<UiTooltipState>()
        .subjects()
        .is_empty());
    app.world_mut()
        .resource_mut::<ButtonInput<KeyCode>>()
        .press(KeyCode::KeyT);
    step(&mut app, 1);
    assert!(
        app.world().resource::<UiTooltipState>().is_pinned(),
        "explicit keyboard inspection still works"
    );
}

#[test]
fn locked_card_survives_empty_space_and_outside_click_until_escape() {
    let (mut app, anchor) = app();
    app.world_mut()
        .entity_mut(anchor)
        .insert(Interaction::Hovered);
    step(&mut app, 1);
    step(&mut app, 2000);
    app.world_mut().entity_mut(anchor).insert(Interaction::None);
    step(&mut app, 60_000);
    assert!(app.world().resource::<UiTooltipState>().is_pinned());
    app.init_resource::<ButtonInput<MouseButton>>();
    app.world_mut()
        .resource_mut::<ButtonInput<MouseButton>>()
        .press(MouseButton::Left);
    step(&mut app, 1);
    assert_eq!(
        app.world().resource::<UiTooltipState>().subjects(),
        &[key("root")]
    );
    assert!(app.world().resource::<UiTooltipState>().is_pinned());
    app.world_mut()
        .resource_mut::<ButtonInput<MouseButton>>()
        .clear();
    app.world_mut()
        .resource_mut::<ButtonInput<KeyCode>>()
        .press(KeyCode::Escape);
    step(&mut app, 1);
    assert!(app
        .world()
        .resource::<UiTooltipState>()
        .subjects()
        .is_empty());
}

#[test]
fn pinned_chain_ignores_other_hover_and_explicit_open_routes() {
    let (mut app, anchor) = app();
    app.world_mut()
        .entity_mut(anchor)
        .insert(Interaction::Hovered);
    step(&mut app, 1);
    step(&mut app, 2000);
    let link = app
        .world_mut()
        .query::<(Entity, &view::TooltipAction)>()
        .iter(app.world())
        .find_map(|(entity, action)| {
            matches!(action, view::TooltipAction::Link(_, _)).then_some(entity)
        })
        .expect("related term");
    app.world_mut().write_message(UiActivated { entity: link });
    step(&mut app, 1);
    assert_eq!(
        app.world().resource::<UiTooltipState>().subjects(),
        &[key("root"), key("term")]
    );

    let host = app.world().get::<ChildOf>(anchor).expect("host").parent();
    let other = app
        .world_mut()
        .spawn((
            crate::button("other source"),
            UiTooltipSource(key("term")),
            UiTooltipOpen(key("term")),
            ChildOf(host),
            ComputedNode {
                size: Vec2::new(80.0, 44.0),
                ..default()
            },
            UiGlobalTransform::from_xy(500.0, 100.0),
        ))
        .id();
    app.world_mut().entity_mut(anchor).insert(Interaction::None);
    app.world_mut()
        .entity_mut(other)
        .insert(Interaction::Hovered);
    step(&mut app, 2000);
    assert_eq!(
        app.world().resource::<UiTooltipState>().subjects(),
        &[key("root"), key("term")]
    );
    app.world_mut().write_message(UiActivated { entity: other });
    step(&mut app, 1);
    app.world_mut()
        .write_message(UiTooltipRequest::Open(key("term")));
    step(&mut app, 1);
    let state = app.world().resource::<UiTooltipState>();
    assert_eq!(state.subjects(), &[key("root"), key("term")]);
    assert_eq!(state.anchor, Some(anchor));
    assert!(state.is_pinned());
    assert!(
        !state.captures_keyboard(),
        "pointer reading does not capture game shortcuts"
    );

    for expected in [vec![key("root")], vec![]] {
        app.world_mut()
            .resource_mut::<ButtonInput<KeyCode>>()
            .press(KeyCode::Escape);
        step(&mut app, 1);
        assert_eq!(
            app.world().resource::<UiTooltipState>().subjects(),
            expected
        );
        assert!(app.world().resource::<UiTooltipState>().captures_keyboard());
    }
    step(&mut app, 2000);
    assert!(app
        .world()
        .resource::<UiTooltipState>()
        .subjects()
        .is_empty());
    app.world_mut().write_message(UiActivated { entity: other });
    step(&mut app, 1);
    assert_eq!(
        app.world().resource::<UiTooltipState>().subjects(),
        &[key("term")]
    );
    assert_eq!(app.world().resource::<UiTooltipState>().anchor, Some(other));
}

// These lifecycle-only fixtures do not install the layout engine. Supply
// measured geometry explicitly, just as we do for the source, and run the real
// placement/reveal code. Labyrinth's per-frame test separately uses real layout.
fn settle_fixture_cards(app: &mut App) {
    for _ in 0..2 {
        let cards = app
            .world_mut()
            .query_filtered::<(Entity, &Node), With<view::TooltipSurface>>()
            .iter(app.world())
            .map(|(entity, node)| (entity, node.clone()))
            .collect::<Vec<_>>();
        for (entity, node) in cards {
            let x = if let Val::Px(value) = node.left {
                value
            } else {
                0.0
            };
            let y = if let Val::Px(value) = node.top {
                value
            } else {
                0.0
            };
            app.world_mut().entity_mut(entity).insert((
                ComputedNode {
                    size: Vec2::new(340.0, 200.0),
                    ..default()
                },
                UiGlobalTransform::from_xy(x + 170.0, y + 100.0),
            ));
        }
        view::place(app.world_mut());
    }
    // The first app frame hides unmeasured cards and clears their focus. Once
    // synthetic measurement has revealed them, run the same reconciliation a
    // subsequent native frame uses to restore focus inside keyboard inspection.
    view::render(app.world_mut());
}

#[test]
fn inspect_during_new_source_dwell_replaces_only_an_unpinned_preview() {
    for pinned in [false, true] {
        let (mut app, anchor) = app();
        app.world_mut()
            .entity_mut(anchor)
            .insert(Interaction::Hovered);
        step(&mut app, 400);
        app.world_mut().resource_mut::<UiTooltipState>().pinned = pinned;
        app.world_mut()
            .entity_mut(anchor)
            .insert(UiTooltipSource(key("term")));
        app.world_mut()
            .resource_mut::<ButtonInput<KeyCode>>()
            .press(KeyCode::KeyT);
        step(&mut app, 1);
        let state = app.world().resource::<UiTooltipState>();
        assert_eq!(
            state.subjects(),
            &[key(if pinned { "root" } else { "term" })]
        );
        assert!(state.is_pinned());
        assert!(state.captures_keyboard());
    }
}

#[test]
fn disabled_inspection_never_emits_activation_and_hidden_sources_disappear() {
    let (mut app, anchor) = app();
    app.world_mut()
        .entity_mut(anchor)
        .insert((crate::UiDisabled, Interaction::Pressed));
    let mut cursor = MessageCursor::<UiActivated>::default();
    step(&mut app, 400);
    assert_eq!(
        app.world().resource::<UiTooltipState>().subjects(),
        &[key("root")]
    );
    assert_eq!(
        cursor
            .read(app.world().resource::<Messages<UiActivated>>())
            .count(),
        0
    );
    assert!(!crate::activation_eligible(app.world_mut(), anchor));
    app.world_mut()
        .get_mut::<Node>(anchor)
        .expect("node")
        .display = Display::None;
    step(&mut app, 500);
    assert!(app
        .world()
        .resource::<UiTooltipState>()
        .subjects()
        .is_empty());
}

#[test]
fn passive_preview_does_not_consume_page_or_escape_navigation() {
    let (mut app, anchor) = app();
    app.world_mut()
        .entity_mut(anchor)
        .insert(Interaction::Hovered);
    step(&mut app, 400);
    assert_eq!(
        app.world().resource::<UiTooltipState>().subjects(),
        &[key("root")]
    );
    for code in [
        KeyCode::Home,
        KeyCode::End,
        KeyCode::PageUp,
        KeyCode::PageDown,
    ] {
        app.world_mut()
            .resource_mut::<ButtonInput<KeyCode>>()
            .press(code);
        step(&mut app, 1);
        assert!(!app.world().resource::<UiTooltipState>().captures_keyboard());
    }
    app.world_mut()
        .resource_mut::<ButtonInput<KeyCode>>()
        .press(KeyCode::Escape);
    step(&mut app, 1);
    let state = app.world().resource::<UiTooltipState>();
    assert!(state.subjects().is_empty());
    assert!(
        !state.captures_keyboard(),
        "the game's Back action also receives Escape"
    );
}

#[test]
fn native_link_activation_and_escape_restore_focus_without_gameplay_actions() {
    let (mut app, anchor) = app();
    app.world_mut()
        .resource_mut::<InputFocus>()
        .set(anchor, bevy::input_focus::FocusCause::Navigated);
    app.world_mut()
        .resource_mut::<ButtonInput<KeyCode>>()
        .press(KeyCode::KeyT);
    step(&mut app, 1);
    assert!(app.world().resource::<UiTooltipState>().captures_keyboard());
    settle_fixture_cards(&mut app);
    let link = app
        .world_mut()
        .query::<(Entity, &view::TooltipAction)>()
        .iter(app.world())
        .find_map(|(entity, action)| {
            matches!(action, view::TooltipAction::Link(_, _)).then_some(entity)
        })
        .expect("native link");
    assert_eq!(app.world().resource::<InputFocus>().get(), Some(link));
    app.world_mut()
        .entity_mut(link)
        .insert(Interaction::Pressed);
    step(&mut app, 10);
    assert_eq!(
        app.world().resource::<UiTooltipState>().subjects(),
        &[key("root"), key("term")]
    );
    settle_fixture_cards(&mut app);
    let focused = app
        .world()
        .resource::<InputFocus>()
        .get()
        .expect("leaf focus");
    assert!(
        matches!(
            app.world().get::<view::TooltipAction>(focused),
            Some(view::TooltipAction::Close(1))
        ),
        "a leaf focuses its close control"
    );
    app.world_mut()
        .resource_mut::<ButtonInput<KeyCode>>()
        .press(KeyCode::Escape);
    step(&mut app, 1);
    assert_eq!(
        app.world().resource::<UiTooltipState>().subjects(),
        &[key("root")]
    );
    app.world_mut()
        .resource_mut::<ButtonInput<KeyCode>>()
        .press(KeyCode::Escape);
    step(&mut app, 1);
    assert!(app
        .world()
        .resource::<UiTooltipState>()
        .subjects()
        .is_empty());
    assert_eq!(app.world().resource::<InputFocus>().get(), Some(anchor));
    assert!(
        app.world().resource::<UiTooltipState>().captures_keyboard(),
        "closing Escape consumed"
    );
    step(&mut app, 1);
    assert!(!app.world().resource::<UiTooltipState>().captures_keyboard());
}

#[test]
fn durable_pin_survives_anchor_rebuild_but_not_content_revocation() {
    let (mut app, anchor) = app();
    app.world_mut()
        .entity_mut(anchor)
        .insert(Interaction::Hovered);
    step(&mut app, 400);
    assert_eq!(app.world().resource::<InputFocus>().get(), None);
    app.world_mut().write_message(UiTooltipRequest::Pin);
    step(&mut app, 1);
    app.world_mut().despawn(anchor);
    step(&mut app, 1000);
    assert!(app.world().resource::<UiTooltipState>().is_pinned());
    app.world_mut()
        .resource_mut::<UiTooltipCatalog>()
        .0
        .get_mut(&key("root"))
        .expect("content")
        .body = "Updated".to_owned();
    step(&mut app, 1);
    assert!(app
        .world_mut()
        .query::<&Text>()
        .iter(app.world())
        .any(|text| text.0 == "Updated"));
    app.world_mut()
        .resource_mut::<UiTooltipCatalog>()
        .0
        .remove(&key("root"));
    step(&mut app, 1);
    assert!(app
        .world()
        .resource::<UiTooltipState>()
        .subjects()
        .is_empty());
    assert!(app
        .world_mut()
        .query::<&view::TooltipSurface>()
        .iter(app.world())
        .next()
        .is_none());
}

#[test]
fn inspection_inherits_source_modal_scope_and_yields_to_a_higher_modal() {
    let (mut app, anchor) = app();
    let modal = app
        .world_mut()
        .spawn(crate::modal("source modal"))
        .insert(GlobalZIndex(30))
        .id();
    app.world_mut().entity_mut(anchor).insert(ChildOf(modal));
    app.world_mut()
        .resource_mut::<InputFocus>()
        .set(anchor, bevy::input_focus::FocusCause::Navigated);
    app.world_mut()
        .resource_mut::<ButtonInput<KeyCode>>()
        .press(KeyCode::KeyT);
    step(&mut app, 1);
    settle_fixture_cards(&mut app);
    let link = app
        .world_mut()
        .query::<(Entity, &view::TooltipAction)>()
        .iter(app.world())
        .find_map(|(entity, action)| {
            matches!(action, view::TooltipAction::Link(_, _)).then_some(entity)
        })
        .expect("link");
    assert!(crate::activation_eligible(app.world_mut(), link));
    app.world_mut()
        .spawn(crate::modal("higher modal"))
        .insert(GlobalZIndex(100));
    step(&mut app, 1);
    assert!(app
        .world()
        .resource::<UiTooltipState>()
        .subjects()
        .is_empty());
}

#[test]
fn suspension_preserves_valid_pins_but_releases_modal_input_and_old_focus() {
    let (mut app, anchor) = app();
    let host = app
        .world()
        .get::<ChildOf>(anchor)
        .expect("source host")
        .parent();
    app.world_mut()
        .resource_mut::<InputFocus>()
        .set(anchor, bevy::input_focus::FocusCause::Navigated);
    app.world_mut()
        .resource_mut::<ButtonInput<KeyCode>>()
        .press(KeyCode::KeyT);
    step(&mut app, 1);
    let link = app
        .world_mut()
        .query::<(Entity, &view::TooltipAction)>()
        .iter(app.world())
        .find_map(|(entity, action)| {
            matches!(action, view::TooltipAction::Link(_, _)).then_some(entity)
        })
        .expect("related term");
    app.world_mut().write_message(UiActivated { entity: link });
    step(&mut app, 1);
    let close = close_action(&mut app, 0);
    let modal = app
        .world_mut()
        .spawn((crate::modal("menu"), ChildOf(host)))
        .insert(GlobalZIndex(100))
        .id();
    let menu_control = app
        .world_mut()
        .spawn((crate::button("menu action"), ChildOf(modal)))
        .id();
    app.world_mut()
        .resource_mut::<InputFocus>()
        .set(menu_control, bevy::input_focus::FocusCause::Navigated);
    app.world_mut().resource_mut::<UiTooltipSuspension>().0 = true;
    app.world_mut().write_message(UiActivated { entity: close });
    app.world_mut()
        .write_message(UiTooltipRequest::Open(key("term")));
    for code in [KeyCode::Escape, KeyCode::KeyT, KeyCode::PageDown] {
        app.world_mut()
            .resource_mut::<ButtonInput<KeyCode>>()
            .press(code);
    }
    step(&mut app, 3000);
    let state = app.world().resource::<UiTooltipState>();
    assert!(state.is_suspended());
    assert!(state.is_pinned());
    assert!(!state.captures_keyboard());
    assert_eq!(state.subjects(), &[key("root"), key("term")]);
    assert_eq!(
        app.world().resource::<InputFocus>().get(),
        Some(menu_control)
    );
    assert_eq!(
        app.world_mut()
            .query_filtered::<Entity, Or<(With<view::TooltipSurface>, With<view::TooltipAction>)>>()
            .iter(app.world())
            .count(),
        0,
        "suspended cards have no rendered or actionable entities"
    );

    app.world_mut().despawn(anchor);
    {
        let mut catalog = app.world_mut().resource_mut::<UiTooltipCatalog>();
        catalog.0.get_mut(&key("root")).expect("root content").body =
            "Refreshed while hidden".to_owned();
        catalog.0.remove(&key("term"));
    }
    step(&mut app, 1);
    assert_eq!(
        app.world().resource::<UiTooltipState>().subjects(),
        &[key("root")]
    );

    app.world_mut().despawn(modal);
    // Let the menu's own focus stack unwind before the game restores its target.
    step(&mut app, 1);
    let current = app
        .world_mut()
        .spawn((crate::button("current"), ChildOf(host)))
        .id();
    app.world_mut()
        .resource_mut::<InputFocus>()
        .set(current, bevy::input_focus::FocusCause::Navigated);
    app.world_mut().resource_mut::<UiTooltipSuspension>().0 = false;
    step(&mut app, 1);
    let state = app.world().resource::<UiTooltipState>();
    assert!(!state.is_suspended());
    assert!(state.is_pinned(), "resume does not replay the dwell timer");
    assert!(!state.captures_keyboard());
    assert_eq!(app.world().resource::<InputFocus>().get(), Some(current));
    assert!(app
        .world_mut()
        .query::<&Text>()
        .iter(app.world())
        .any(|text| text.0 == "Refreshed while hidden"));
    app.world_mut()
        .resource_mut::<ButtonInput<KeyCode>>()
        .press(KeyCode::Escape);
    step(&mut app, 1);
    assert_eq!(
        app.world().resource::<InputFocus>().get(),
        Some(current),
        "dismissal cannot restore the pre-menu focus"
    );
}

#[test]
fn suspension_set_after_resolve_hides_existing_cards_in_the_same_frame() {
    let (mut app, anchor) = app();
    app.world_mut()
        .resource_mut::<InputFocus>()
        .set(anchor, bevy::input_focus::FocusCause::Navigated);
    app.world_mut()
        .resource_mut::<ButtonInput<KeyCode>>()
        .press(KeyCode::KeyT);
    step(&mut app, 1);
    settle_fixture_cards(&mut app);
    assert!(app.world().resource::<UiTooltipState>().captures_keyboard());
    app.add_systems(
        PostUpdate,
        (|mut suspension: ResMut<UiTooltipSuspension>| suspension.0 = true)
            .before(UiTooltipSystems::Render),
    );
    step(&mut app, 1);
    let state = app.world().resource::<UiTooltipState>();
    assert!(state.is_suspended());
    assert!(state.is_pinned());
    assert!(!state.captures_keyboard());
    assert_eq!(app.world().resource::<InputFocus>().get(), None);
    assert_eq!(
        app.world_mut()
            .query::<&view::TooltipSurface>()
            .iter(app.world())
            .count(),
        0
    );
}

#[test]
fn suspended_preview_and_open_requests_do_not_accumulate_hover_time() {
    let (mut app, anchor) = app();
    app.world_mut()
        .entity_mut(anchor)
        .insert((Interaction::Hovered, UiTooltipOpen(key("root"))));
    step(&mut app, 1);
    step(&mut app, 600);
    assert!(!app.world().resource::<UiTooltipState>().is_pinned());
    app.world_mut().resource_mut::<UiTooltipSuspension>().0 = true;
    app.world_mut()
        .write_message(UiTooltipRequest::Open(key("root")));
    app.world_mut().write_message(UiTooltipRequest::Pin);
    app.world_mut()
        .write_message(UiActivated { entity: anchor });
    app.world_mut()
        .resource_mut::<ButtonInput<KeyCode>>()
        .press(KeyCode::KeyT);
    step(&mut app, 5000);
    assert!(app
        .world()
        .resource::<UiTooltipState>()
        .subjects()
        .is_empty());
    app.world_mut().resource_mut::<UiTooltipSuspension>().0 = false;
    step(&mut app, 5000);
    assert_eq!(
        app.world().resource::<UiTooltipState>().subjects(),
        &[key("root")]
    );
    assert!(!app.world().resource::<UiTooltipState>().is_pinned());
    step(&mut app, 999);
    assert!(!app.world().resource::<UiTooltipState>().is_pinned());
    step(&mut app, 1);
    assert!(app.world().resource::<UiTooltipState>().is_pinned());
}

#[test]
fn suspended_pins_still_observe_disclosure_host_and_lifecycle_invalidation() {
    for invalidation in [
        "catalog",
        "host removed",
        "host replaced",
        "dismiss",
        "back",
    ] {
        let (mut app, anchor) = app();
        let host = app
            .world()
            .get::<ChildOf>(anchor)
            .expect("source host")
            .parent();
        app.world_mut()
            .write_message(UiTooltipRequest::Open(key("root")));
        step(&mut app, 1);
        app.world_mut().resource_mut::<UiTooltipSuspension>().0 = true;
        step(&mut app, 1);
        match invalidation {
            "catalog" => {
                app.world_mut()
                    .resource_mut::<UiTooltipCatalog>()
                    .0
                    .remove(&key("root"));
            }
            "host removed" => {
                app.world_mut().despawn(host);
            }
            "host replaced" => {
                app.world_mut().despawn(host);
                app.world_mut()
                    .spawn((crate::screen_root("new host"), UiTooltipHost));
            }
            "dismiss" => {
                app.world_mut().write_message(UiTooltipRequest::Dismiss);
            }
            "back" => {
                app.world_mut().write_message(UiTooltipRequest::Back);
            }
            _ => unreachable!(),
        }
        step(&mut app, 1);
        assert!(
            app.world()
                .resource::<UiTooltipState>()
                .subjects()
                .is_empty(),
            "{invalidation}"
        );
        assert!(
            !app.world().resource::<UiTooltipState>().is_pinned(),
            "{invalidation}"
        );
    }
}

#[test]
fn suspended_transient_pin_retains_modal_covered_help_but_not_a_missing_source() {
    let (mut app, anchor) = app();
    app.world_mut()
        .entity_mut(anchor)
        .remove::<UiTooltipSource>()
        .insert((
            UiContextHelp {
                title: "Transient".to_owned(),
                body: "Disclosed".to_owned(),
            },
            Interaction::Hovered,
        ));
    step(&mut app, 1);
    app.world_mut().write_message(UiTooltipRequest::Pin);
    step(&mut app, 1);
    app.world_mut().resource_mut::<UiTooltipSuspension>().0 = true;
    app.world_mut()
        .spawn(crate::modal("cover"))
        .insert(GlobalZIndex(100));
    step(&mut app, 1);
    assert!(app.world().resource::<UiTooltipState>().is_pinned());
    app.world_mut().despawn(anchor);
    step(&mut app, 1);
    assert!(app
        .world()
        .resource::<UiTooltipState>()
        .subjects()
        .is_empty());
}

#[test]
fn adopter_can_disable_escape_dismissal_without_changing_the_default() {
    for dismiss_key in [Some(KeyCode::Escape), None] {
        let (mut app, _) = app();
        assert_eq!(
            app.world().resource::<UiTooltipSettings>().dismiss_key,
            Some(KeyCode::Escape)
        );
        app.world_mut()
            .resource_mut::<UiTooltipSettings>()
            .dismiss_key = dismiss_key;
        app.world_mut()
            .write_message(UiTooltipRequest::Open(key("root")));
        step(&mut app, 1);
        app.world_mut()
            .resource_mut::<ButtonInput<KeyCode>>()
            .press(KeyCode::Escape);
        step(&mut app, 1);
        assert_eq!(
            app.world().resource::<UiTooltipState>().is_pinned(),
            dismiss_key.is_none()
        );
        assert_eq!(
            app.world().resource::<UiTooltipState>().captures_keyboard(),
            dismiss_key.is_some()
        );
    }
}

#[test]
fn activation_opened_pin_returns_focus_after_keyboard_close_without_t_inspection() {
    let (mut app, anchor) = app();
    app.world_mut()
        .entity_mut(anchor)
        .insert(UiTooltipOpen(key("root")));
    app.world_mut()
        .resource_mut::<InputFocus>()
        .set(anchor, bevy::input_focus::FocusCause::Navigated);
    app.world_mut()
        .resource_mut::<ButtonInput<KeyCode>>()
        .press(KeyCode::Enter);
    step(&mut app, 1);
    assert!(app.world().resource::<UiTooltipState>().is_pinned());
    assert!(!app.world().resource::<UiTooltipState>().captures_keyboard());
    settle_fixture_cards(&mut app);
    let close = close_action(&mut app, 0);
    app.world_mut()
        .resource_mut::<InputFocus>()
        .set(close, bevy::input_focus::FocusCause::Navigated);
    app.world_mut()
        .resource_mut::<ButtonInput<KeyCode>>()
        .press(KeyCode::Enter);
    step(&mut app, 1);
    assert!(app
        .world()
        .resource::<UiTooltipState>()
        .subjects()
        .is_empty());
    assert_eq!(app.world().resource::<InputFocus>().get(), Some(anchor));
    assert!(
        app.world().resource::<UiTooltipState>().captures_keyboard(),
        "closing Enter stays consumed after returning focus"
    );
    step(&mut app, 1);
    assert!(!app.world().resource::<UiTooltipState>().captures_keyboard());
}

#[test]
fn pointer_pin_close_returns_latest_outside_focus_only_when_the_card_had_focus() {
    for focus_close in [false, true] {
        let (mut app, anchor) = app();
        let host = app
            .world()
            .get::<ChildOf>(anchor)
            .expect("source host")
            .parent();
        app.world_mut()
            .resource_mut::<InputFocus>()
            .set(anchor, bevy::input_focus::FocusCause::Navigated);
        app.world_mut()
            .entity_mut(anchor)
            .insert(Interaction::Hovered);
        step(&mut app, 1);
        app.world_mut().write_message(UiTooltipRequest::Pin);
        step(&mut app, 1);
        let latest = app
            .world_mut()
            .spawn((crate::button("latest focus"), ChildOf(host)))
            .id();
        app.world_mut()
            .resource_mut::<InputFocus>()
            .set(latest, bevy::input_focus::FocusCause::Navigated);
        step(&mut app, 1);
        settle_fixture_cards(&mut app);
        let close = close_action(&mut app, 0);
        if focus_close {
            app.world_mut()
                .resource_mut::<InputFocus>()
                .set(close, bevy::input_focus::FocusCause::Navigated);
        }
        app.world_mut()
            .entity_mut(close)
            .insert(Interaction::Pressed);
        step(&mut app, 1);
        assert!(app
            .world()
            .resource::<UiTooltipState>()
            .subjects()
            .is_empty());
        assert_eq!(app.world().resource::<InputFocus>().get(), Some(latest));
    }
}

#[test]
fn focused_pin_close_never_restores_a_hidden_outside_control() {
    let (mut app, anchor) = app();
    app.world_mut()
        .resource_mut::<InputFocus>()
        .set(anchor, bevy::input_focus::FocusCause::Navigated);
    app.world_mut()
        .write_message(UiTooltipRequest::Open(key("root")));
    step(&mut app, 1);
    settle_fixture_cards(&mut app);
    let close = close_action(&mut app, 0);
    app.world_mut()
        .resource_mut::<InputFocus>()
        .set(close, bevy::input_focus::FocusCause::Navigated);
    app.world_mut()
        .entity_mut(anchor)
        .insert(Visibility::Hidden);
    app.world_mut()
        .entity_mut(close)
        .insert(Interaction::Pressed);
    step(&mut app, 1);
    assert!(app
        .world()
        .resource::<UiTooltipState>()
        .subjects()
        .is_empty());
    assert_eq!(app.world().resource::<InputFocus>().get(), None);
}
