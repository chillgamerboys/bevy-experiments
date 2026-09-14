use super::*;

fn key(value: &str) -> UiTooltipSubject {
    UiTooltipSubject(value.to_owned())
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
    state.hover(Some(key("ability")), Duration::from_millis(999), &settings);
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
fn preview_is_pointer_transparent_and_pin_has_links_without_close_controls() {
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
    assert_eq!(actions.len(), 1, "only the related term is actionable");
    assert!(
        matches!(actions.first(), Some(view::TooltipAction::Link(0, subject)) if subject == &key("term"))
    );
    assert!(!app
        .world_mut()
        .query::<&Text>()
        .iter(app.world())
        .any(|text| matches!(text.0.as_str(), "Pin" | "Unpin" | "Close" | "Pin · T" | "x")));
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
        UiRect::default()
    );
    app.world_mut()
        .resource_mut::<ButtonInput<KeyCode>>()
        .press(KeyCode::Escape);
    step(&mut app, 1);
    step(&mut app, 1000);
    assert!(app
        .world()
        .resource::<UiTooltipState>()
        .subjects()
        .is_empty());
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
    step(&mut app, 1000);
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
    step(&mut app, 1000);
    let link = app
        .world_mut()
        .query_filtered::<Entity, With<view::TooltipAction>>()
        .single(app.world())
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
    assert_eq!(
        app.world()
            .get::<Name>(focused)
            .expect("leaf card")
            .as_str(),
        "Tooltip Card 1"
    );
    assert!(
        app.world().get::<crate::UiAction>(focused).is_none(),
        "a card cannot activate gameplay"
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
