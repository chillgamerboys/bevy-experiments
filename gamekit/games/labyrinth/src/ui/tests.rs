//! Structural/input evidence only: no claim about renderer pixels or real cursor picking.

use super::*;
use bevy_game_test::{
    click_action, find_named, focus_action, run_frames, tap_key, ui_tree_snapshot,
    visible_control_rect, HeadlessUiPlugin,
};
use bevy_game_ui::{activation_eligible, UiAction, UiDisabled};
use labyrinth_rules::{ActorKind, Combat, StatusInstance, StatusKind};

fn fixture() -> LabyrinthView {
    let combat = (0..128)
        .filter_map(|seed| Combat::new(seed, HeroClass::ALL).ok())
        .map(|combat| combat.snapshot())
        .find(|snapshot| {
            snapshot
                .active_actor
                .and_then(|id| snapshot.actor(id))
                .is_some_and(|actor| matches!(actor.kind, ActorKind::Hero(_)))
        })
        .expect("fixture with a hero decision");
    LabyrinthView {
        mode: ViewMode::Combat,
        local: true,
        host: true,
        admitted: true,
        player: Some(0),
        encounter: 1,
        combat: Some(combat),
        players: HeroClass::ALL
            .into_iter()
            .enumerate()
            .map(|(index, hero)| crate::view::PlayerView {
                slot: u8::try_from(index).expect("four player index"),
                hero,
                name: format!("Player {}", index + 1),
                occupied: true,
                connected: true,
                ready: true,
            })
            .collect(),
        ..LabyrinthView::default()
    }
}

fn app(width: u32, height: u32, scale: UiScaleMode) -> App {
    let mut app = App::new();
    app.add_plugins(HeadlessUiPlugin::new(width, height))
        .insert_resource(UiScalePreference(scale))
        .insert_resource(fixture())
        .add_plugins(LabyrinthUiPlugin);
    app.finish();
    app.cleanup();
    run_frames(&mut app, 5);
    app
}

#[test]
fn pointer_and_keyboard_commit_the_same_typed_owned_action() {
    for keyboard in [false, true] {
        let mut app = app(1920, 1080, UiScaleMode::Auto);
        let actor = app
            .world()
            .resource::<LabyrinthView>()
            .combat
            .as_ref()
            .and_then(|snapshot| snapshot.active_actor)
            .expect("acting hero");
        let wait = find_named(app.world_mut(), "Wait").expect("wait control");
        if keyboard {
            assert!(focus_action(app.world_mut(), wait));
            tap_key(&mut app, KeyCode::Enter);
        } else {
            assert!(click_action(&mut app, wait));
        }
        run_frames(&mut app, 2);
        let confirm = find_named(app.world_mut(), "Confirm Combat Action").expect("confirmation");
        assert!(app.world().get::<UiDisabled>(confirm).is_none());
        if keyboard {
            assert!(focus_action(app.world_mut(), confirm));
            tap_key(&mut app, KeyCode::Space);
        } else {
            assert!(click_action(&mut app, confirm));
        }
        let intents = app
            .world_mut()
            .resource_mut::<Messages<LabyrinthIntent>>()
            .drain()
            .collect::<Vec<_>>();
        assert!(intents.iter().any(|intent| matches!(intent, LabyrinthIntent::Combat { actor: source, action: CombatAction::Wait, encounter: 1, .. } if *source == actor)));
    }
}

#[test]
fn battlefield_and_status_identity_survive_snapshot_and_rank_changes() {
    let mut app = app(1920, 1080, UiScaleMode::Auto);
    let actor = find_named(app.world_mut(), "Actor 1").expect("hero control");
    {
        let mut view = app.world_mut().resource_mut::<LabyrinthView>();
        let snapshot = view.combat.as_mut().expect("combat");
        snapshot.hero_formation.swap(0, 1);
        let boundary = snapshot.boundary_sequence;
        let hero = snapshot
            .actors
            .iter_mut()
            .find(|actor| actor.id == ActorId(1))
            .expect("hero");
        hero.hp -= 2;
        hero.statuses.push(StatusInstance {
            id: 500,
            kind: StatusKind::Bleed,
            bearer: hero.id,
            source: ActorId(103),
            potency: 2,
            remaining: 3,
            eligible_boundary: boundary + 1,
        });
        snapshot.revision += 1;
    }
    run_frames(&mut app, 4);
    assert_eq!(find_named(app.world_mut(), "Actor 1"), Some(actor));
    let badge = find_named(app.world_mut(), "Status 1 500").expect("bleed badge");
    {
        let mut view = app.world_mut().resource_mut::<LabyrinthView>();
        let snapshot = view.combat.as_mut().expect("combat");
        let hero = snapshot
            .actors
            .iter_mut()
            .find(|actor| actor.id == ActorId(1))
            .expect("hero");
        hero.statuses.first_mut().expect("bleed").remaining = 2;
        snapshot.revision += 1;
    }
    run_frames(&mut app, 3);
    assert_eq!(find_named(app.world_mut(), "Status 1 500"), Some(badge));
    assert!(click_action(&mut app, badge));
    run_frames(&mut app, 2);
    let tree = ui_tree_snapshot(app.world_mut()).to_string();
    assert!(tree.contains("next 3 turns"));
    assert!(tree.contains("2 boundaries left"));
}

#[test]
fn invalid_skills_remain_inspectable_and_remote_ownership_blocks_commit() {
    let mut app = app(1920, 1080, UiScaleMode::Auto);
    app.world_mut().resource_mut::<LabyrinthView>().local = false;
    let snapshot = app
        .world()
        .resource::<LabyrinthView>()
        .combat
        .clone()
        .expect("combat");
    let active_kind = snapshot
        .active_actor
        .and_then(|id| snapshot.actor(id))
        .expect("active")
        .kind;
    let other = HeroClass::ALL
        .into_iter()
        .position(|hero| ActorKind::Hero(hero) != active_kind)
        .expect("different hero");
    app.world_mut().resource_mut::<LabyrinthView>().player =
        Some(u8::try_from(other).expect("slot"));
    run_frames(&mut app, 3);
    let skill = find_named(app.world_mut(), "Skill 0").expect("inspectable skill");
    assert!(focus_action(app.world_mut(), skill));
    tap_key(&mut app, KeyCode::Enter);
    run_frames(&mut app, 2);
    let confirm = find_named(app.world_mut(), "Confirm Combat Action").expect("confirmation");
    assert!(app.world().get::<UiDisabled>(confirm).is_some());
    assert!(ui_tree_snapshot(app.world_mut())
        .to_string()
        .contains("Wait for your hero"));
}

#[test]
fn reconnect_overlay_traps_focus_and_restores_after_recovery() {
    let mut app = app(1920, 1080, UiScaleMode::Auto);
    let wait = find_named(app.world_mut(), "Wait").expect("wait");
    assert!(focus_action(app.world_mut(), wait));
    app.world_mut().resource_mut::<LabyrinthView>().paused = true;
    if let Some(player) = app
        .world_mut()
        .resource_mut::<LabyrinthView>()
        .players
        .get_mut(2)
    {
        player.connected = false;
    }
    run_frames(&mut app, 4);
    assert!(!activation_eligible(app.world_mut(), wait));
    let focused = app
        .world()
        .resource::<InputFocus>()
        .get()
        .expect("modal owns focus");
    assert!(activation_eligible(app.world_mut(), focused));
    app.world_mut().resource_mut::<LabyrinthView>().paused = false;
    run_frames(&mut app, 4);
    assert!(activation_eligible(app.world_mut(), wait));
    assert_eq!(app.world().resource::<InputFocus>().get(), Some(wait));
}

#[test]
fn six_viewports_preserve_keyboard_reachability_and_semantic_regions() {
    for (width, height) in [(1280, 720), (1920, 1080), (3840, 2160)] {
        for scale in [UiScaleMode::Auto, UiScaleMode::Percent200] {
            let mut app = app(width, height, scale);
            let tree = ui_tree_snapshot(app.world_mut()).to_string();
            for name in [
                "Battle HUD",
                "Combat Action Rail",
                "Initiative Timeline",
                "Actor 1",
                "Actor 104",
            ] {
                assert!(tree.contains(name), "missing {name}");
            }
            let actions = app
                .world_mut()
                .query_filtered::<Entity, With<UiAction>>()
                .iter(app.world())
                .collect::<Vec<_>>();
            for entity in actions {
                if !activation_eligible(app.world_mut(), entity) {
                    continue;
                }
                assert!(focus_action(app.world_mut(), entity));
                run_frames(&mut app, 4);
                let viewport =
                    Rect::from_corners(Vec2::ZERO, Vec2::new(width as f32, height as f32));
                let visible = visible_control_rect(app.world(), entity, viewport)
                    .expect("focused control is not entirely clipped");
                assert!(
                    visible.width() >= 43.5 && visible.height() >= 43.5,
                    "{width}x{height} {scale:?} {:?}: {visible:?}",
                    app.world().get::<Name>(entity)
                );
            }
        }
    }
}

#[test]
fn native_forms_clear_secret_buffers_and_emit_only_typed_intents() {
    let mut app = app(1280, 720, UiScaleMode::Percent200);
    *app.world_mut().resource_mut::<LabyrinthView>() = LabyrinthView::default();
    run_frames(&mut app, 3);
    let direct = find_named(app.world_mut(), "Join Direct").expect("direct menu");
    click_action(&mut app, direct);
    run_frames(&mut app, 3);
    let field = find_named(app.world_mut(), "Private BGN1 connection code").expect("native field");
    app.world_mut()
        .entity_mut(field)
        .insert(bevy::text::EditableText::new("BGN1-fixture-secret"));
    run_frames(&mut app, 3);
    assert_eq!(app.world().resource::<UiState>().code.0.len(), 19);
    let submit = find_named(app.world_mut(), "Submit Direct").expect("submit");
    click_action(&mut app, submit);
    assert!(app.world().resource::<UiState>().code.0.is_empty());
    let native = app
        .world()
        .get::<bevy::text::EditableText>(field)
        .expect("native field remains");
    assert!(native.value().to_string().is_empty());
    let intents = app
        .world_mut()
        .resource_mut::<Messages<LabyrinthIntent>>()
        .drain()
        .collect::<Vec<_>>();
    assert!(intents.iter().any(|intent| matches!(intent, LabyrinthIntent::JoinCode(code) if code.0 == "BGN1-fixture-secret")));
    assert!(!format!("{intents:?}").contains("fixture-secret"));
}

#[test]
fn stale_selection_is_not_relabelled_as_the_next_decision_or_encounter() {
    for new_encounter in [false, true] {
        let mut app = app(1920, 1080, UiScaleMode::Auto);
        apply_action(app.world_mut(), Action::Choice(Choice::Wait));
        {
            let mut view = app.world_mut().resource_mut::<LabyrinthView>();
            if new_encounter {
                view.encounter += 1;
            } else {
                view.combat.as_mut().expect("combat").turn_id += 1;
            }
        }
        // Input can arrive before the presentation system reconciles its local selection.
        apply_action(app.world_mut(), Action::Confirm);
        assert!(!app
            .world_mut()
            .resource_mut::<Messages<LabyrinthIntent>>()
            .drain()
            .any(|intent| matches!(intent, LabyrinthIntent::Combat { .. })));
    }
}

#[test]
fn damage_feedback_deduplicates_events_and_does_not_replay_on_recovery() {
    let mut app = app(1920, 1080, UiScaleMode::Auto);
    let event = crate::view::PresentedEvent {
        id: 100,
        event: labyrinth_rules::CombatEvent {
            id: 1,
            kind: labyrinth_rules::CombatEventKind::Damage {
                source: ActorId(103),
                target: ActorId(1),
                amount: 2,
                kind: labyrinth_rules::DamageKind::Bleed,
            },
        },
    };
    app.world_mut()
        .resource_mut::<LabyrinthView>()
        .events
        .push(event.clone());
    present(app.world_mut());
    assert!(ui_tree_snapshot(app.world_mut())
        .to_string()
        .contains("-2 BLEED"));
    app.world_mut()
        .resource_mut::<Time>()
        .advance_by(std::time::Duration::from_secs(2));
    present(app.world_mut());
    assert!(!ui_tree_snapshot(app.world_mut())
        .to_string()
        .contains("-2 BLEED"));
    app.world_mut().resource_mut::<LabyrinthView>().paused = true;
    present(app.world_mut());
    {
        let mut view = app.world_mut().resource_mut::<LabyrinthView>();
        view.paused = false;
        view.events
            .push(crate::view::PresentedEvent { id: 101, ..event });
    }
    present(app.world_mut());
    assert!(!ui_tree_snapshot(app.world_mut())
        .to_string()
        .contains("-2 BLEED"));
}

#[test]
fn leaving_the_browser_by_escape_stops_provider_activity() {
    let mut app = app(1280, 720, UiScaleMode::Auto);
    *app.world_mut().resource_mut::<LabyrinthView>() = LabyrinthView::default();
    apply_action(app.world_mut(), Action::Form(Form::Browser));
    app.world_mut()
        .resource_mut::<Messages<LabyrinthIntent>>()
        .clear();
    apply_action(app.world_mut(), Action::Cancel);
    assert!(app
        .world_mut()
        .resource_mut::<Messages<LabyrinthIntent>>()
        .drain()
        .any(|intent| matches!(intent, LabyrinthIntent::StopBrowsing)));
}

#[test]
fn backing_out_of_admission_forms_cancels_pending_work() {
    for form in [Form::Host, Form::Direct, Form::Password] {
        for escape in [false, true] {
            let mut app = app(1280, 720, UiScaleMode::Auto);
            *app.world_mut().resource_mut::<LabyrinthView>() = LabyrinthView::default();
            apply_action(app.world_mut(), Action::Form(form));
            app.world_mut()
                .resource_mut::<Messages<LabyrinthIntent>>()
                .clear();
            apply_action(
                app.world_mut(),
                if escape {
                    Action::Cancel
                } else {
                    Action::Form(Form::Menu)
                },
            );
            assert!(app
                .world_mut()
                .resource_mut::<Messages<LabyrinthIntent>>()
                .drain()
                .any(|intent| matches!(intent, LabyrinthIntent::Leave)));
        }
    }
}

#[test]
fn compact_scaled_admission_forms_scroll_all_controls_and_fields_into_view() {
    let mut app = app(1280, 720, UiScaleMode::Percent200);
    *app.world_mut().resource_mut::<LabyrinthView>() = LabyrinthView::default();
    for form in [Form::Host, Form::Direct, Form::Browser, Form::Password] {
        apply_action(app.world_mut(), Action::Form(form));
        run_frames(&mut app, 4);
        let controls = app
            .world_mut()
            .query_filtered::<Entity, Or<(With<UiAction>, With<UiTextField>)>>()
            .iter(app.world())
            .collect::<Vec<_>>();
        for entity in controls {
            if !activation_eligible(app.world_mut(), entity) {
                continue;
            }
            app.world_mut()
                .resource_mut::<InputFocus>()
                .set(entity, bevy::input_focus::FocusCause::Navigated);
            run_frames(&mut app, 4);
            let visible = visible_control_rect(
                app.world(),
                entity,
                Rect::from_corners(Vec2::ZERO, Vec2::new(1280.0, 720.0)),
            )
            .expect("focused field or control must be visible");
            assert!(
                visible.width() >= 43.5 && visible.height() >= 43.5,
                "{:?}: {visible:?}",
                app.world().get::<Name>(entity)
            );
        }
    }
}

#[test]
fn default_720_battle_overview_shows_every_actor_name_and_hp_without_scrolling() {
    let mut app = app(1280, 720, UiScaleMode::Auto);
    for actor in [1, 2, 3, 4, 101, 102, 103, 104] {
        let text = find_named(app.world_mut(), &format!("Actor {actor} Summary"))
            .expect("actor name and HP summary");
        let node = app.world().get::<ComputedNode>(text).expect("layout");
        let expected = node.size() * node.inverse_scale_factor;
        let visible = visible_control_rect(
            app.world(),
            text,
            Rect::from_corners(Vec2::ZERO, Vec2::new(1280.0, 720.0)),
        )
        .expect("actor summary visible in initial overview");
        assert!(
            visible.width() + 0.5 >= expected.x && visible.height() + 0.5 >= expected.y,
            "actor {actor} summary clipped: {visible:?} / {expected:?}"
        );
    }
}
