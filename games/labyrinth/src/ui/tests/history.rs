use super::*;

#[test]
fn log_toggle_has_no_tooltip_before_or_after_pointer_and_keyboard_activation_normal_1080() {
    log_toggle_has_no_tooltip_before_or_after_pointer_and_keyboard_activation(
        1920,
        1080,
        UiScaleMode::Auto,
    );
}

#[test]
fn log_toggle_has_no_tooltip_before_or_after_pointer_and_keyboard_activation_compatibility() {
    log_toggle_has_no_tooltip_before_or_after_pointer_and_keyboard_activation(
        1280,
        720,
        UiScaleMode::Auto,
    );
}

fn log_toggle_has_no_tooltip_before_or_after_pointer_and_keyboard_activation(
    width: u32,
    height: u32,
    scale: UiScaleMode,
) {
    for keyboard in [false, true] {
        let mut app = app(width, height, scale);
        let toolbar = find_named(app.world_mut(), "Battle Log Toggle").expect("log toggle");
        let point = visible_control_rect(
            app.world(),
            toolbar,
            Rect::from_corners(Vec2::ZERO, Vec2::new(width as f32, height as f32)),
        )
        .expect("toolbar geometry")
        .center();
        if keyboard {
            assert!(focus_action(app.world_mut(), toolbar));
        } else {
            overlay_stability::hover_at(&mut app, point);
        }
        run_frames(&mut app, 3);
        assert!(app
            .world()
            .resource::<bevy_gamekit::ui::UiTooltipState>()
            .subjects()
            .is_empty());
        if keyboard {
            tap_key(&mut app, KeyCode::Enter);
        } else {
            overlay_stability::native_pointer_click(&mut app, point);
        }
        run_frames(&mut app, 30);
        assert!(find_named(app.world_mut(), "Combat History").is_some());
        assert!(app
            .world()
            .resource::<bevy_gamekit::ui::UiTooltipState>()
            .subjects()
            .is_empty());
        assert!(find_named(app.world_mut(), "Tooltip Card 0").is_none());
    }
}

#[test]
fn game_menu_has_no_tooltip_and_temporarily_hides_locked_inspection_normal_1080() {
    game_menu_has_no_tooltip_and_temporarily_hides_locked_inspection(1920, 1080, UiScaleMode::Auto);
}

#[test]
fn game_menu_has_no_tooltip_and_temporarily_hides_locked_inspection_compatibility() {
    game_menu_has_no_tooltip_and_temporarily_hides_locked_inspection(1280, 720, UiScaleMode::Auto);
}

fn game_menu_has_no_tooltip_and_temporarily_hides_locked_inspection(
    width: u32,
    height: u32,
    scale: UiScaleMode,
) {
    use bevy_gamekit::ui::{UiContextHelp, UiTooltipRequest, UiTooltipState};
    for keyboard in [false, true] {
        let mut app = app(width, height, scale);
        let toolbar = find_named(app.world_mut(), "Battle Settings").expect("game menu");
        assert!(app.world().get::<UiContextHelp>(toolbar).is_none());
        let source = find_named(app.world_mut(), "Skill 0").expect("skill");
        assert!(focus_action(app.world_mut(), source));
        tap_key(&mut app, KeyCode::KeyT);
        assert!(app.world().resource::<UiTooltipState>().is_pinned());
        // Return keyboard arbitration to the game, then open a mouse-locked card.
        app.world_mut().write_message(UiTooltipRequest::Dismiss);
        run_frames(&mut app, 1);
        let subject = app
            .world()
            .get::<bevy_gamekit::ui::UiTooltipSource>(source)
            .expect("source")
            .0
            .clone();
        app.world_mut()
            .write_message(UiTooltipRequest::Open(subject.clone()));
        run_frames(&mut app, 1);
        if keyboard {
            assert!(focus_action(app.world_mut(), toolbar));
            tap_key(&mut app, KeyCode::Enter);
        } else {
            let viewport = Rect::from_corners(Vec2::ZERO, Vec2::new(width as f32, height as f32));
            let point = visible_control_rect(app.world(), toolbar, viewport)
                .expect("menu")
                .center();
            overlay_stability::native_pointer_click(&mut app, point);
        }
        run_frames(&mut app, 8);
        assert!(find_named(app.world_mut(), "Game Menu Title").is_some());
        assert_eq!(
            app.world().resource::<UiTooltipState>().subjects(),
            std::slice::from_ref(&subject)
        );
        assert!(app.world().resource::<UiTooltipState>().is_suspended());
        assert!(find_named(app.world_mut(), "Tooltip Card 0").is_none());
        tap_key(&mut app, KeyCode::Escape);
        run_frames(&mut app, 3);
        assert_eq!(
            app.world().resource::<UiTooltipState>().subjects(),
            &[subject]
        );
        assert!(find_named(app.world_mut(), "Tooltip Card 0").is_some());
    }
}

pub(super) fn seed_recent(app: &mut App) {
    let view = app.world().resource::<LabyrinthView>();
    let history = crate::view::EncounterHistory::from_events(view.encounter, &view.events)
        .expect("contiguous history fixture");
    app.world_mut().insert_resource(history);
}

fn records(count: u64) -> Vec<crate::view::PresentedEvent> {
    (1..=count)
        .map(|id| crate::view::PresentedEvent {
            id,
            event: labyrinth_rules::CombatEvent {
                id,
                kind: if id % 2 == 1 {
                    labyrinth_rules::CombatEventKind::Action {
                        actor: ActorId(105),
                        action: CombatAction::Wait,
                    }
                } else {
                    labyrinth_rules::CombatEventKind::Damage {
                        source: ActorId(105),
                        target: ActorId(4),
                        amount: 4,
                        kind: labyrinth_rules::DamageKind::Direct,
                    }
                },
            },
        })
        .collect()
}

fn install_records(app: &mut App, records: &[crate::view::PresentedEvent]) {
    let encounter = app.world().resource::<LabyrinthView>().encounter;
    app.world_mut().insert_resource(
        crate::view::EncounterHistory::from_events(encounter, records).expect("history"),
    );
    app.world_mut().resource_mut::<LabyrinthView>().events = records
        .iter()
        .rev()
        .take(80)
        .cloned()
        .collect::<Vec<_>>()
        .into_iter()
        .rev()
        .collect();
}

fn panel_visible(app: &mut App) -> bool {
    find_named(app.world_mut(), "Combat History")
        .and_then(|panel| app.world().get::<Node>(panel))
        .is_some_and(|node| node.display != Display::None)
}

#[test]
fn compact_history_visibility_retains_rows_and_reading_state_normal_1080() {
    compact_history_visibility_retains_rows_and_reading_state(1920, 1080, UiScaleMode::Auto);
}

#[test]
fn compact_history_visibility_retains_rows_and_reading_state_compatibility() {
    compact_history_visibility_retains_rows_and_reading_state(1280, 720, UiScaleMode::Auto);
    compact_history_visibility_retains_rows_and_reading_state(3840, 2160, UiScaleMode::Auto);
}

fn compact_history_visibility_retains_rows_and_reading_state(
    width: u32,
    height: u32,
    scale: UiScaleMode,
) {
    let mut app = app(width, height, scale);
    assert!(!panel_visible(&mut app));
    let before = app.world().resource::<LabyrinthView>().combat.clone();
    install_records(&mut app, &records(300));
    run_frames(&mut app, 3);
    assert!(
        !panel_visible(&mut app),
        "incoming records never reopen the log"
    );
    let toolbar = find_named(app.world_mut(), "Battle Log Toggle").expect("log toggle");
    assert!(click_action(&mut app, toolbar));
    run_frames(&mut app, 4);
    assert!(panel_visible(&mut app));
    assert!(
        find_named(app.world_mut(), "History Toggle").is_none(),
        "no separate log modes"
    );
    assert!(
        find_named(app.world_mut(), "History Expand 1").is_none(),
        "plain text rows"
    );
    let scroll = find_named(app.world_mut(), "History Scroll").expect("scroll");
    tap_key(&mut app, KeyCode::Home);
    run_frames(&mut app, 3);
    let first = find_named(app.world_mut(), "History Entry 1").expect("first archived record");
    let hide = find_named(app.world_mut(), "History Hide").expect("hide");
    assert!(focus_action(app.world_mut(), hide));
    tap_key(&mut app, KeyCode::Enter);
    run_frames(&mut app, 3);
    assert!(!panel_visible(&mut app));
    assert!(!activation_eligible(app.world_mut(), hide));
    assert_eq!(app.world().resource::<InputFocus>().get(), Some(toolbar));
    assert_eq!(
        app.world()
            .resource::<crate::view::EncounterHistory>()
            .loaded_len(),
        300
    );
    assert!(click_action(&mut app, toolbar));
    run_frames(&mut app, 4);
    assert_eq!(find_named(app.world_mut(), "History Entry 1"), Some(first));
    assert_eq!(
        app.world()
            .get::<ScrollPosition>(scroll)
            .expect("retained reading offset")
            .y,
        0.0
    );
    assert!(!app
        .world()
        .get::<bevy_gamekit::ui::UiFeedScroll>(scroll)
        .expect("feed")
        .follows_latest());
    assert_eq!(app.world().resource::<LabyrinthView>().combat, before);
}

#[test]
fn compact_history_pages_and_arrivals_preserve_anchor_until_latest_normal_1080() {
    use crate::view::{EncounterHistory, HistoryBounds};
    let mut app = app(1920, 1080, UiScaleMode::Auto);
    let all = records(300);
    let encounter = app.world().resource::<LabyrinthView>().encounter;
    let bounds = HistoryBounds {
        first: 1,
        next: 301,
    };
    app.world_mut()
        .resource_mut::<EncounterHistory>()
        .observe(encounter, bounds, all.get(220..).expect("recent 80"))
        .expect("recent snapshot");
    apply_action(app.world_mut(), Action::ToggleLog);
    run_frames(&mut app, 4);
    let actor = find_named(app.world_mut(), "Actor 1").expect("actor");
    let geometry = app.world().get::<UiGlobalTransform>(actor).copied();
    let scroll = find_named(app.world_mut(), "History Scroll").expect("scroll");
    let mut intents = bevy::ecs::message::MessageCursor::<LabyrinthIntent>::default();
    tap_key(&mut app, KeyCode::Home);
    run_frames(&mut app, 3);
    let anchored = find_named(app.world_mut(), "History Entry 1").expect("stable loading row");
    assert!(intents.read(app.world().resource::<Messages<LabyrinthIntent>>())
        .any(|intent| matches!(intent, LabyrinthIntent::HistoryPage { encounter: e, from: 1 } if *e == encounter)));
    let page = crate::session::history::HistoryPage {
        request_id: 1,
        encounter,
        bounds,
        from: 1,
        events: all.get(..64).expect("page").to_vec(),
    };
    app.world_mut()
        .resource_mut::<EncounterHistory>()
        .merge_page(&page)
        .expect("earlier page");
    run_frames(&mut app, 3);
    assert_eq!(
        find_named(app.world_mut(), "History Entry 1"),
        Some(anchored)
    );
    assert_eq!(
        app.world().get::<ScrollPosition>(scroll).expect("offset").y,
        0.0
    );
    let next = crate::view::PresentedEvent {
        id: 301,
        event: labyrinth_rules::CombatEvent {
            id: 301,
            kind: labyrinth_rules::CombatEventKind::RoundStarted { round: 30 },
        },
    };
    app.world_mut()
        .resource_mut::<EncounterHistory>()
        .observe(
            encounter,
            HistoryBounds {
                next: 302,
                ..bounds
            },
            &[next],
        )
        .expect("new arrival");
    run_frames(&mut app, 3);
    assert_eq!(
        find_named(app.world_mut(), "History Entry 1"),
        Some(anchored)
    );
    assert_eq!(
        app.world().get::<ScrollPosition>(scroll).expect("offset").y,
        0.0
    );
    assert_eq!(
        app.world()
            .get::<bevy_gamekit::ui::UiFeedScroll>(scroll)
            .expect("feed")
            .unread(),
        1
    );
    let mounted = app
        .world_mut()
        .query::<&Name>()
        .iter(app.world())
        .filter(|name| name.as_str().starts_with("History Entry "))
        .count();
    assert!(
        mounted <= 32,
        "only a bounded visible/page range is mounted: {mounted}"
    );
    assert_eq!(
        app.world().get::<UiGlobalTransform>(actor).copied(),
        geometry
    );
    assert!(activation_eligible(app.world_mut(), actor));
    let latest = find_named(app.world_mut(), "History Latest").expect("Latest");
    let rect = visible_control_rect(
        app.world(),
        latest,
        Rect::from_corners(Vec2::ZERO, Vec2::new(1920.0, 1080.0)),
    )
    .expect("visible Latest");
    overlay_stability::native_pointer_click(&mut app, rect.center());
    run_frames(&mut app, 3);
    assert_eq!(
        app.world()
            .get::<bevy_gamekit::ui::UiFeedScroll>(scroll)
            .expect("feed")
            .unread(),
        0
    );
    assert!(find_named(app.world_mut(), "History Entry 301").is_some());
    assert!(
        app.world()
            .get::<ScrollPosition>(scroll)
            .expect("latest offset")
            .y
            > 1000.0
    );
    let offset = app.world().get::<ScrollPosition>(scroll).expect("offset").y;
    tap_key(&mut app, KeyCode::Escape);
    run_frames(&mut app, 3);
    tap_key(&mut app, KeyCode::Home);
    assert_eq!(
        app.world()
            .get::<ScrollPosition>(scroll)
            .expect("modal-contained offset")
            .y,
        offset
    );
    assert!(!activation_eligible(app.world_mut(), latest));
    tap_key(&mut app, KeyCode::Escape);
    run_frames(&mut app, 3);
    assert!(activation_eligible(app.world_mut(), latest));
}

#[test]
fn compact_history_is_readonly_and_revokes_disclosed_rows_normal_1080() {
    let mut app = app(1920, 1080, UiScaleMode::Auto);
    install_records(&mut app, &records(8));
    let before = app.world().resource::<LabyrinthView>().combat.clone();
    let mut intents = bevy::ecs::message::MessageCursor::<LabyrinthIntent>::default();
    apply_action(app.world_mut(), Action::ToggleLog);
    run_frames(&mut app, 3);
    assert!(app
        .world_mut()
        .query::<&Name>()
        .iter(app.world())
        .any(|name| name.as_str() == "History Entry 2"));
    assert_eq!(
        intents
            .read(app.world().resource::<Messages<LabyrinthIntent>>())
            .count(),
        0
    );
    apply_action(app.world_mut(), Action::HideLog);
    run_frames(&mut app, 3);
    assert!(!panel_visible(&mut app));
    app.world_mut()
        .resource_mut::<crate::presentation::CombatDisclosure>()
        .actors
        .insert(
            ActorId(105),
            crate::presentation::ActorDisclosure {
                health: false,
                statuses: false,
                details: false,
            },
        );
    run_frames(&mut app, 3);
    let text = app
        .world_mut()
        .query::<(&Name, &Text)>()
        .iter(app.world())
        .filter(|(name, _)| name.as_str() == "History Summary")
        .map(|(_, text)| text.0.clone())
        .collect::<Vec<_>>();
    assert_eq!(text, ["Combat details are concealed."]);
    assert!(!panel_visible(&mut app));
    apply_action(app.world_mut(), Action::ToggleLog);
    run_frames(&mut app, 3);
    assert!(panel_visible(&mut app));
    assert!(find_named(app.world_mut(), "History Entry 2").is_none());
    assert_eq!(app.world().resource::<LabyrinthView>().combat, before);
}

#[test]
fn compact_history_long_text_inspection_preserves_pin_menu_and_disclosure_normal_1080() {
    use bevy_gamekit::ui::{UiTooltipCatalog, UiTooltipSource, UiTooltipState};
    let mut app = app(1920, 1080, UiScaleMode::Auto);
    let actor_name = format!("{} actor", "A".repeat(122));
    let target_name = format!("{} target", "B".repeat(121));
    let skill_name = format!("{} skill", "C".repeat(122));
    {
        let mut view = app.world_mut().resource_mut::<LabyrinthView>();
        let snapshot = view.combat.as_mut().expect("combat");
        let actor = snapshot
            .actors
            .iter_mut()
            .find(|actor| actor.id == ActorId(1))
            .expect("actor");
        actor.display_name = actor_name.clone();
        actor
            .resolved_build
            .moveset
            .skills
            .first_mut()
            .expect("skill")
            .definition
            .name = skill_name.clone();
        snapshot
            .actors
            .iter_mut()
            .find(|actor| actor.id == ActorId(101))
            .expect("target")
            .display_name = target_name.clone();
    }
    let event = crate::view::PresentedEvent {
        id: 1,
        event: labyrinth_rules::CombatEvent {
            id: 1,
            kind: labyrinth_rules::CombatEventKind::Action {
                actor: ActorId(1),
                action: CombatAction::Skill {
                    index: 0,
                    target: ActorId(101),
                },
            },
        },
    };
    install_records(&mut app, std::slice::from_ref(&event));
    let before = app.world().resource::<LabyrinthView>().combat.clone();
    let mut intents = bevy::ecs::message::MessageCursor::<LabyrinthIntent>::default();
    apply_action(app.world_mut(), Action::ToggleLog);
    run_frames(&mut app, 4);
    let row = find_named(app.world_mut(), "History Entry 1").expect("row");
    let source = app
        .world()
        .get::<UiTooltipSource>(row)
        .expect("full-text inspection")
        .0
        .clone();
    let full_text = app
        .world()
        .resource::<UiTooltipCatalog>()
        .0
        .get(&source)
        .expect("disclosed text")
        .body
        .clone();
    assert!(full_text.contains(&actor_name));
    assert!(full_text.contains(&target_name));
    assert!(full_text.contains(&skill_name));
    let viewport = Rect::from_corners(Vec2::ZERO, Vec2::new(1920.0, 1080.0));
    let point = visible_control_rect(app.world(), row, viewport)
        .expect("plain row geometry")
        .center();
    overlay_stability::hover_at(&mut app, point);
    assert!(find_named(app.world_mut(), "Tooltip Card 0").is_some());
    assert!(!app.world().resource::<UiTooltipState>().is_pinned());
    run_frames(&mut app, 15);
    assert!(app.world().resource::<UiTooltipState>().is_pinned());
    assert!(app
        .world_mut()
        .query::<&Text>()
        .iter(app.world())
        .any(|text| text.0 == full_text));
    // Virtualization can remove the source without revoking a valid pinned record.
    let mut extended = vec![event];
    extended.extend(records(300).into_iter().skip(1));
    install_records(&mut app, &extended);
    run_frames(&mut app, 4);
    assert!(find_named(app.world_mut(), "History Entry 1").is_none());
    assert_eq!(
        app.world().resource::<UiTooltipState>().subjects(),
        std::slice::from_ref(&source)
    );
    assert_eq!(
        app.world()
            .resource::<UiTooltipCatalog>()
            .0
            .get(&source)
            .expect("recycled pin content")
            .body,
        full_text
    );
    let menu = find_named(app.world_mut(), "Battle Settings").expect("menu");
    assert!(click_action(&mut app, menu));
    run_frames(&mut app, 3);
    assert!(app.world().resource::<UiTooltipState>().is_suspended());
    assert!(find_named(app.world_mut(), "Tooltip Card 0").is_none());
    tap_key(&mut app, KeyCode::Escape);
    run_frames(&mut app, 3);
    assert!(find_named(app.world_mut(), "Tooltip Card 0").is_some());
    let close = find_named(app.world_mut(), "Tooltip Close").expect("pin ×");
    assert!(click_action(&mut app, close));
    run_frames(&mut app, 3);
    assert!(app
        .world()
        .resource::<UiTooltipState>()
        .subjects()
        .is_empty());
    // Keyboard activation opens the same full-text route without gameplay.
    tap_key(&mut app, KeyCode::Home);
    run_frames(&mut app, 3);
    let row = find_named(app.world_mut(), "History Entry 1").expect("older row");
    assert!(focus_action(app.world_mut(), row));
    tap_key(&mut app, KeyCode::Enter);
    run_frames(&mut app, 3);
    assert!(app.world().resource::<UiTooltipState>().is_pinned());
    tap_key(&mut app, KeyCode::End);
    run_frames(&mut app, 3);
    let description = find_named(app.world_mut(), "Tooltip Description").expect("full text");
    let node = app
        .world()
        .get::<ComputedNode>(description)
        .expect("text bounds");
    let layout = app
        .world()
        .get::<bevy::text::TextLayoutInfo>(description)
        .expect("shaped full text");
    assert!(
        layout.size.x <= node.size().x + 1.0,
        "long words wrap within the inspection card"
    );
    let visible = visible_control_rect(app.world(), description, viewport)
        .expect("End reaches full-text tail");
    let transform = app
        .world()
        .get::<UiGlobalTransform>(description)
        .expect("text geometry");
    let bottom = transform.translation.y + node.size().y * node.inverse_scale_factor * 0.5;
    assert!(
        visible.max.y + 1.0 >= bottom,
        "the final line is reachable by keyboard: {visible:?}, bottom {bottom}"
    );
    apply_action(app.world_mut(), Action::HideLog);
    app.world_mut()
        .resource_mut::<crate::presentation::CombatDisclosure>()
        .actors
        .insert(
            ActorId(105),
            crate::presentation::ActorDisclosure {
                health: false,
                statuses: false,
                details: false,
            },
        );
    run_frames(&mut app, 3);
    assert!(!panel_visible(&mut app));
    assert!(app
        .world()
        .resource::<UiTooltipState>()
        .subjects()
        .is_empty());
    assert!(!app
        .world()
        .resource::<UiTooltipCatalog>()
        .0
        .contains_key(&source));
    assert_eq!(app.world().resource::<LabyrinthView>().combat, before);
    assert_eq!(
        intents
            .read(app.world().resource::<Messages<LabyrinthIntent>>())
            .count(),
        0
    );
}

#[test]
fn compact_history_new_encounter_resets_reading_normal_1080() {
    let mut app = app(1920, 1080, UiScaleMode::Auto);
    install_records(&mut app, &records(300));
    apply_action(app.world_mut(), Action::ToggleLog);
    run_frames(&mut app, 4);
    tap_key(&mut app, KeyCode::Home);
    run_frames(&mut app, 3);
    app.world_mut().resource_mut::<LabyrinthView>().encounter += 1;
    let next = records(400).into_iter().skip(300).collect::<Vec<_>>();
    install_records(&mut app, &next);
    run_frames(&mut app, 4);
    let scroll = find_named(app.world_mut(), "History Scroll").expect("scroll");
    assert!(app
        .world()
        .get::<bevy_gamekit::ui::UiFeedScroll>(scroll)
        .expect("new feed")
        .follows_latest());
    assert!(find_named(app.world_mut(), "History Entry 1").is_none());
    assert!(find_named(app.world_mut(), "History Entry 400").is_some());
}
