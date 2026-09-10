use super::*;

#[test]
fn main_menu_wrapped_footer_fits_inside_its_surface() {
    for (width, height) in [(1280, 720), (1920, 1080), (3840, 2160)] {
        let mut app = app(width, height, UiScaleMode::Auto);
        *app.world_mut().resource_mut::<LabyrinthView>() = LabyrinthView::default();
        run_frames(&mut app, 5);
        let advice = find_named(app.world_mut(), "Local Play Advice").expect("footer");
        let bounds = Rect::from_corners(Vec2::ZERO, Vec2::new(width as f32, height as f32));
        let visible = visible_control_rect(app.world(), advice, bounds).expect("visible footer");
        let node = app
            .world()
            .get::<ComputedNode>(advice)
            .copied()
            .expect("measured text");
        assert!(
            visible.height() + 0.5 >= node.size().y * node.inverse_scale_factor,
            "wrapped footer clipped at {width}x{height}: visible={visible:?}, measured={node:?} tree={}", ui_tree_snapshot(app.world_mut())
        );
    }
}

#[test]
fn local_navigation_blocks_only_own_input_and_leave_requires_confirmation() {
    let mut app = app(1280, 720, UiScaleMode::Auto);
    let before = app.world().resource::<LabyrinthView>().combat.clone();
    apply_action(app.world_mut(), Action::Choice(Choice::Wait));
    let source = find_named(app.world_mut(), "Battle Settings").expect("game menu");
    assert!(click_action(&mut app, source));
    run_frames(&mut app, 3);
    assert_eq!(
        app.world().resource::<UiState>().menus.current(),
        Some(&MenuPage::Game)
    );
    assert!(!app.world().resource::<LabyrinthView>().paused);
    assert!(!app.world().resource::<Time<Virtual>>().is_paused());
    assert!(battle::selected_action(
        app.world().resource::<LabyrinthView>(),
        app.world().resource::<UiState>()
    )
    .is_err());
    let settings = find_named(app.world_mut(), "Game Settings").expect("settings");
    assert!(click_action(&mut app, settings));
    tap_key(&mut app, KeyCode::Escape);
    run_frames(&mut app, 3);
    assert_eq!(
        app.world().resource::<UiState>().menus.current(),
        Some(&MenuPage::Game)
    );
    tap_key(&mut app, KeyCode::Escape);
    run_frames(&mut app, 3);
    assert!(!app.world().resource::<UiState>().menus.is_open());
    assert_eq!(
        app.world().resource::<UiState>().selected,
        Some(Choice::Wait)
    );
    assert_eq!(app.world().resource::<LabyrinthView>().combat, before);
    app.world_mut()
        .resource_mut::<Messages<LabyrinthIntent>>()
        .clear();
    apply_action(app.world_mut(), Action::Leave);
    assert_eq!(
        app.world_mut()
            .resource_mut::<Messages<LabyrinthIntent>>()
            .drain()
            .count(),
        0
    );
    run_frames(&mut app, 3);
    let leave = find_named(app.world_mut(), "Confirm Leave").expect("explicit confirmation");
    assert!(click_action(&mut app, leave));
    assert!(app
        .world_mut()
        .resource_mut::<Messages<LabyrinthIntent>>()
        .drain()
        .any(|intent| matches!(intent, LabyrinthIntent::Leave)));
}

#[test]
fn menu_close_cannot_clear_connection_or_fault_suspension() {
    for reason in [
        crate::view::CombatInterruption::WaitingForPlayers,
        crate::view::CombatInterruption::Reconnecting,
        crate::view::CombatInterruption::Halted,
    ] {
        let mut app = app(1280, 720, UiScaleMode::Auto);
        apply_action(app.world_mut(), Action::Settings);
        {
            let mut view = app.world_mut().resource_mut::<LabyrinthView>();
            view.paused = true;
            view.interruption = reason;
        }
        run_frames(&mut app, 3);
        tap_key(&mut app, KeyCode::Escape);
        run_frames(&mut app, 3);
        assert!(app.world().resource::<LabyrinthView>().paused);
        assert_eq!(app.world().resource::<LabyrinthView>().interruption, reason);
        assert!(find_named(app.world_mut(), "Reconnect Title").is_some());
        let wait = find_named(app.world_mut(), "Wait").expect("wait");
        assert!(!activation_eligible(app.world_mut(), wait));
    }
}

#[test]
fn admission_notice_refresh_preserves_native_field_entity_value_and_focus() {
    let mut app = app(1280, 720, UiScaleMode::Auto);
    *app.world_mut().resource_mut::<LabyrinthView>() = LabyrinthView::default();
    apply_action(app.world_mut(), Action::Form(Form::Direct));
    run_frames(&mut app, 3);
    let field = find_named(app.world_mut(), "Private BGN1 connection code").expect("field");
    app.world_mut()
        .entity_mut(field)
        .insert(bevy::text::EditableText::new("BGN1-incomplete"));
    run_frames(&mut app, 3);
    assert!(focus_action(app.world_mut(), field));
    {
        let mut view = app.world_mut().resource_mut::<LabyrinthView>();
        view.notice = Some("Still connecting".into());
        view.provider_notices.push("LAN unavailable".into());
    }
    run_frames(&mut app, 3);
    assert_eq!(
        find_named(app.world_mut(), "Private BGN1 connection code"),
        Some(field)
    );
    assert_eq!(app.world().resource::<InputFocus>().get(), Some(field));
    assert_eq!(app.world().resource::<UiState>().code.0, "BGN1-incomplete");
}
