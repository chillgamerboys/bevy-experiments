//! Deterministic rules/presentation/input/layout evidence, not rendered or audio proof.

use super::*;
use bevy::input_focus::InputFocus;
use bevy_game_test::{
    click_action, find_named, focus_action, run_frames, tap_key, visible_control_rect,
    HeadlessUiPlugin,
};
use bevy_game_ui::{
    activation_eligible, GameUiSkinPlugin, UiAction, UiScaleMode, UiScalePreference, UiTextStyle,
};
use sequencer::Runtime;

fn ready_runtime() -> Runtime {
    let mut runtime = Runtime::default();
    runtime.sound = false;
    for _ in 0..16 {
        runtime.tick(0.0, true);
        if runtime.can_select() {
            return runtime;
        }
        runtime.apply(&CarterfightIntent::Advance);
    }
    assert!(runtime.can_select(), "bounded introduction");
    runtime
}

fn app(width: u32, height: u32, scale: UiScaleMode, battle: bool) -> App {
    let mut app = App::new();
    app.add_plugins((HeadlessUiPlugin::new(width, height), GameUiSkinPlugin))
        .insert_resource(bevy::time::TimeUpdateStrategy::ManualDuration(
            std::time::Duration::ZERO,
        ))
        .insert_resource(UiScalePreference(scale))
        .init_asset::<AudioSource>();
    let font = app
        .world_mut()
        .resource_mut::<Assets<Font>>()
        .add(Font::from_bytes(
            include_bytes!("../../assets/fonts/PressStart2P-Regular.ttf").to_vec(),
        ));
    app.insert_resource(systems::CarterAssets {
        font,
        carter: default(),
        cursor: default(),
        chime: default(),
    });
    let mut runtime = if battle {
        ready_runtime()
    } else {
        Runtime::default()
    };
    runtime.sound = false;
    app.insert_resource(runtime).add_plugins(CarterfightPlugin);
    app.finish();
    app.cleanup();
    run_frames(&mut app, 4);
    app
}

#[test]
fn selection_does_not_resolve_and_displayed_hp_waits_for_its_damage_event() {
    let mut runtime = ready_runtime();
    runtime.apply(&CarterfightIntent::SelectMove("jab"));
    assert_eq!(runtime.battle.turn_count, 0);
    runtime.apply(&CarterfightIntent::ConfirmMove);
    assert_eq!(runtime.battle.turn_count, 1);
    assert_eq!(runtime.battle.opponent.current_hp, 52);
    assert_eq!(runtime.displayed_opponent, 60);
    runtime.tick(0.0, true); // UseMove, not Damage.
    assert_eq!(runtime.displayed_opponent, 60);
    runtime.apply(&CarterfightIntent::Advance);
    runtime.tick(0.0, true);
    assert_eq!(runtime.displayed_opponent, 52);
    assert_eq!(runtime.displayed_player, 60);
}

#[test]
fn a_single_advance_reveals_but_does_not_also_acknowledge() {
    let mut runtime = Runtime::default();
    runtime.tick(0.05, false);
    assert!(runtime.view().typing);
    runtime.apply(&CarterfightIntent::Advance);
    runtime.tick(0.0, false);
    assert_eq!(runtime.view().narration, "A wild CARTER appeared!");
    assert!(!runtime.view().typing);
    assert_eq!(runtime.phase, CarterfightPhase::Intro);
    runtime.apply(&CarterfightIntent::Advance);
    runtime.tick(0.0, true);
    assert_eq!(runtime.view().narration, "What will you do?");
}

#[test]
fn intro_battle_and_outro_preserve_the_complete_local_game() {
    let mut runtime = ready_runtime();
    for _ in 0..3 {
        runtime.apply(&CarterfightIntent::SelectMove("haymaker"));
        runtime.apply(&CarterfightIntent::ConfirmMove);
        for _ in 0..32 {
            runtime.tick(0.0, true);
            if runtime.can_select() || runtime.phase == CarterfightPhase::Outro {
                break;
            }
            assert!(!runtime.apply(&CarterfightIntent::Advance));
        }
    }
    assert_eq!(runtime.battle.opponent.current_hp, 0);
    assert_eq!(runtime.battle.turn_count, 3);
    assert_eq!(runtime.phase, CarterfightPhase::Outro);
    let mut closed = false;
    for _ in 0..12 {
        runtime.tick(0.0, true);
        closed |= runtime.apply(&CarterfightIntent::Advance);
    }
    assert!(closed);
}

#[test]
fn pointer_enter_space_and_number_shortcuts_share_the_selection_commit_path() {
    for route in 0..3 {
        let mut app = app(1280, 720, UiScaleMode::Auto, true);
        let choose = find_named(app.world_mut(), "Choose jab").expect("move button");
        match route {
            0 => {
                assert!(click_action(&mut app, choose));
            }
            1 => {
                assert!(focus_action(app.world_mut(), choose));
                tap_key(&mut app, KeyCode::Enter);
            }
            _ => tap_key(&mut app, KeyCode::Digit1),
        }
        run_frames(&mut app, 3);
        assert_eq!(app.world().resource::<Runtime>().battle.turn_count, 0);
        assert!(app.world().resource::<CarterfightView>().can_confirm);
        let confirm = find_named(app.world_mut(), "Confirm move").expect("confirmation");
        assert_eq!(app.world().resource::<InputFocus>().get(), Some(confirm));
        if route == 0 {
            assert!(click_action(&mut app, confirm));
        } else {
            tap_key(&mut app, KeyCode::Space);
        }
        assert_eq!(app.world().resource::<Runtime>().battle.turn_count, 1);
        assert_eq!(
            app.world().resource::<Runtime>().battle.opponent.current_hp,
            52
        );
    }
}

#[test]
fn disabled_moves_do_not_interrupt_narration_and_space_only_reveals() {
    let mut app = app(1280, 720, UiScaleMode::Auto, false);
    let choose = find_named(app.world_mut(), "Choose jab").expect("move button");
    assert!(!activation_eligible(app.world_mut(), choose));
    tap_key(&mut app, KeyCode::Digit1);
    assert_eq!(app.world().resource::<Runtime>().pending, None);
    tap_key(&mut app, KeyCode::Space);
    assert_eq!(
        app.world().resource::<CarterfightView>().narration,
        "A wild CARTER appeared!"
    );
    assert_eq!(app.world().resource::<Runtime>().battle.turn_count, 0);
}

#[test]
fn duplicate_confirm_messages_cannot_cross_the_narration_boundary() {
    let mut app = app(1280, 720, UiScaleMode::Auto, true);
    app.world_mut()
        .resource_mut::<Runtime>()
        .apply(&CarterfightIntent::SelectMove("jab"));
    for _ in 0..2 {
        app.world_mut()
            .write_message(CarterfightIntent::ConfirmMove);
    }
    app.update();
    assert_eq!(app.world().resource::<Runtime>().battle.turn_count, 1);
    app.update();
    assert_eq!(app.world().resource::<Runtime>().battle.turn_count, 1);
}

#[test]
fn six_viewports_keep_all_enabled_controls_reachable_and_labels_readable() {
    for (width, height) in [(1280, 720), (1920, 1080), (3840, 2160)] {
        for scale in [UiScaleMode::Auto, UiScaleMode::Percent200] {
            let mut app = app(width, height, scale, true);
            let actions = app
                .world_mut()
                .query_filtered::<Entity, With<UiAction>>()
                .iter(app.world())
                .collect::<Vec<_>>();
            for action in actions {
                if !activation_eligible(app.world_mut(), action) {
                    continue;
                }
                assert!(focus_action(app.world_mut(), action));
                run_frames(&mut app, 4);
                let visible = visible_control_rect(
                    app.world(),
                    action,
                    Rect::from_corners(Vec2::ZERO, Vec2::new(width as f32, height as f32)),
                )
                .expect("focused control visible");
                assert!(
                    visible.width() >= 43.5 && visible.height() >= 43.5,
                    "{width}x{height} {scale:?}: {visible:?}"
                );
            }
            assert!(app
                .world_mut()
                .query::<&UiTextStyle>()
                .iter(app.world())
                .all(|style| style.base_size.is_none_or(|size| size >= 18.0)));
        }
    }
}

#[test]
fn reduced_motion_reveals_without_audio_and_does_not_skip_acknowledgement() {
    let mut runtime = Runtime::default();
    assert!(!runtime.tick(0.01, true));
    assert!(!runtime.view().typing);
    assert_eq!(runtime.view().narration, "A wild CARTER appeared!");
    runtime.tick(10.0, true);
    assert_eq!(runtime.phase, CarterfightPhase::Intro);
}
