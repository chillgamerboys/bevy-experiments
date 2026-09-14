use super::*;

#[test]
fn escape_opens_menu_with_selected_action_and_pinned_help_normal_1080() {
    use bevy_gamekit::ui::UiTooltipState;
    let mut app = app(1920, 1080, UiScaleMode::Auto);
    let source = find_named(app.world_mut(), "Skill 0").expect("skill source");
    assert!(focus_action(app.world_mut(), source));
    tap_key(&mut app, KeyCode::KeyT);
    run_frames(&mut app, 3);
    let pins = app.world().resource::<UiTooltipState>().subjects().to_vec();
    assert!(!pins.is_empty());
    assert!(app.world().resource::<UiTooltipState>().captures_keyboard());
    apply_action(app.world_mut(), Action::Choice(Choice::Wait));
    app.world_mut().resource_mut::<UiState>().log_mode = LogMode::Compact;
    tap_key(&mut app, KeyCode::Escape);
    run_frames(&mut app, 3);
    assert_eq!(
        app.world().resource::<UiState>().menus.current(),
        Some(&MenuPage::Game)
    );
    assert_eq!(app.world().resource::<UiTooltipState>().subjects(), pins);
    assert!(find_named(app.world_mut(), "Tooltip Card 0").is_none());
    assert!(!app.world().resource::<UiTooltipState>().captures_keyboard());
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
    assert_eq!(app.world().resource::<UiTooltipState>().subjects(), pins);
    assert!(find_named(app.world_mut(), "Tooltip Card 0").is_some());
}

#[test]
fn party_page_requires_explicit_host_pause_and_back_does_not_resume_normal_1080() {
    use crate::view::CombatInterruption;
    let mut app = app(1920, 1080, UiScaleMode::Auto);
    network_ownership(&mut app.world_mut().resource_mut::<LabyrinthView>());
    apply_action(app.world_mut(), Action::GameMenu);
    run_frames(&mut app, 3);
    let title = find_named(app.world_mut(), "Game Menu Title").expect("title first");
    let party = find_named(app.world_mut(), "Manage Assignments").expect("party page");
    let viewport = Rect::from_corners(Vec2::ZERO, Vec2::new(1920., 1080.));
    assert!(
        visible_control_rect(app.world(), title, viewport)
            .expect("visible menu content")
            .max
            .y
            <= visible_control_rect(app.world(), party, viewport)
                .expect("visible menu content")
                .min
                .y
    );
    app.world_mut()
        .resource_mut::<Messages<LabyrinthIntent>>()
        .clear();
    assert!(click_action(&mut app, party));
    run_frames(&mut app, 3);
    assert!(find_named(app.world_mut(), "Party Management Title").is_some());
    assert!(find_named(app.world_mut(), "Game Menu Title").is_none());
    assert!(!app.world().resource::<LabyrinthView>().paused);
    assert_eq!(
        app.world_mut()
            .resource_mut::<Messages<LabyrinthIntent>>()
            .drain()
            .count(),
        0
    );
    let pause = find_named(app.world_mut(), "Pause For Assignments").expect("explicit pause");
    assert!(click_action(&mut app, pause));
    assert!(app
        .world_mut()
        .resource_mut::<Messages<LabyrinthIntent>>()
        .drain()
        .any(|intent| matches!(intent, LabyrinthIntent::AssignmentPause(true))));
    {
        let mut view = app.world_mut().resource_mut::<LabyrinthView>();
        view.paused = true;
        view.interruption = CombatInterruption::Assignments;
    }
    run_frames(&mut app, 3);
    assert!(find_named(app.world_mut(), "Resume After Assignment").is_some());
    let back = find_named(app.world_mut(), "Party Back").expect("party back");
    assert!(click_action(&mut app, back));
    run_frames(&mut app, 3);
    assert_eq!(
        app.world().resource::<UiState>().menus.current(),
        Some(&MenuPage::Game)
    );
    assert!(app.world().resource::<LabyrinthView>().paused);
    assert!(!app
        .world_mut()
        .resource_mut::<Messages<LabyrinthIntent>>()
        .drain()
        .any(|intent| matches!(intent, LabyrinthIntent::AssignmentPause(false))));
    app.world_mut().resource_mut::<LabyrinthView>().host = false;
    apply_action(app.world_mut(), Action::PartyManagement);
    run_frames(&mut app, 3);
    assert!(find_named(app.world_mut(), "Resume After Assignment").is_none());
    assert!(find_named(app.world_mut(), "Pause For Assignments").is_none());
}

#[test]
fn interruptions_remain_visible_with_recovery_inside_open_pages_normal_1080() {
    use crate::view::CombatInterruption;
    for page in [Action::GameMenu, Action::Settings, Action::PartyManagement] {
        let mut app = app(1920, 1080, UiScaleMode::Auto);
        network_ownership(&mut app.world_mut().resource_mut::<LabyrinthView>());
        apply_action(app.world_mut(), page);
        {
            let mut view = app.world_mut().resource_mut::<LabyrinthView>();
            view.paused = true;
            view.interruption = CombatInterruption::Halted;
        }
        run_frames(&mut app, 3);
        assert!(app.world().resource::<UiState>().menus.is_open());
        assert!(find_named(app.world_mut(), "Menu Interruption Notice").is_some());
        assert!(find_named(app.world_mut(), "Abort To Lobby").is_some());
        assert!(find_named(app.world_mut(), "Pause For Assignments").is_none());
        if let Some(advice) = find_named(app.world_mut(), "Settings Advice") {
            assert!(!app
                .world()
                .get::<Text>(advice)
                .expect("visible menu content")
                .0
                .contains("continues"));
        }
        {
            let mut view = app.world_mut().resource_mut::<LabyrinthView>();
            view.host = false;
            view.admitted = false;
            view.interruption = CombatInterruption::Reconnecting;
        }
        run_frames(&mut app, 3);
        assert!(find_named(app.world_mut(), "Menu Interruption Notice").is_some());
        assert!(find_named(app.world_mut(), "Overlay Reconnect").is_some());
        assert!(find_named(app.world_mut(), "Abort To Lobby").is_none());
    }
}

#[test]
fn main_menu_wrapped_footer_fits_inside_its_surface_normal_1080() {
    main_menu_wrapped_footer_fits_inside_its_surface(1920, 1080, UiScaleMode::Auto);
}

#[test]
fn main_menu_wrapped_footer_fits_inside_its_surface_compatibility() {
    main_menu_wrapped_footer_fits_inside_its_surface(1280, 720, UiScaleMode::Auto);
    main_menu_wrapped_footer_fits_inside_its_surface(3840, 2160, UiScaleMode::Auto);
}

fn main_menu_wrapped_footer_fits_inside_its_surface(width: u32, height: u32, scale: UiScaleMode) {
    let mut app = app(width, height, scale);
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

#[test]
fn local_navigation_blocks_only_own_input_and_leave_requires_confirmation_normal_1080() {
    local_navigation_blocks_only_own_input_and_leave_requires_confirmation(
        1920,
        1080,
        UiScaleMode::Auto,
    );
}

#[test]
fn local_navigation_blocks_only_own_input_and_leave_requires_confirmation_compatibility() {
    local_navigation_blocks_only_own_input_and_leave_requires_confirmation(
        1280,
        720,
        UiScaleMode::Auto,
    );
}

fn local_navigation_blocks_only_own_input_and_leave_requires_confirmation(
    width: u32,
    height: u32,
    scale: UiScaleMode,
) {
    let mut app = app(width, height, scale);
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
fn menu_close_cannot_clear_connection_or_fault_suspension_normal_1080() {
    menu_close_cannot_clear_connection_or_fault_suspension(1920, 1080, UiScaleMode::Auto);
}

#[test]
fn menu_close_cannot_clear_connection_or_fault_suspension_compatibility() {
    menu_close_cannot_clear_connection_or_fault_suspension(1280, 720, UiScaleMode::Auto);
}

fn menu_close_cannot_clear_connection_or_fault_suspension(
    width: u32,
    height: u32,
    scale: UiScaleMode,
) {
    for reason in [
        crate::view::CombatInterruption::WaitingForPlayers,
        crate::view::CombatInterruption::Reconnecting,
        crate::view::CombatInterruption::Halted,
    ] {
        let mut app = app(width, height, scale);
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
fn admission_notice_refresh_preserves_native_field_entity_value_and_focus_normal_1080() {
    admission_notice_refresh_preserves_native_field_entity_value_and_focus(
        1920,
        1080,
        UiScaleMode::Auto,
    );
}

#[test]
fn admission_notice_refresh_preserves_native_field_entity_value_and_focus_compatibility() {
    admission_notice_refresh_preserves_native_field_entity_value_and_focus(
        1280,
        720,
        UiScaleMode::Auto,
    );
}

fn admission_notice_refresh_preserves_native_field_entity_value_and_focus(
    width: u32,
    height: u32,
    scale: UiScaleMode,
) {
    let mut app = app(width, height, scale);
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
