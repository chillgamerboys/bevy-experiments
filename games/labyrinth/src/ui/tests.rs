//! Structural/native-input evidence only: no renderer or desktop interaction claim.

use super::*;
use bevy_gamekit::testing::{
    click_action, find_named, focus_action, run_frames, tap_key, ui_tree_snapshot,
    visible_control_rect, HeadlessUiPlugin,
};
use bevy_gamekit::ui::{activation_eligible, UiAction, UiDisabled};
use labyrinth_rules::{
    ActorKind, Combat, HeroSetup, LegacySkillLoadout, StatusInstance, StatusKind,
    DEFAULT_HERO_ROSTER, MAX_LEGACY_SKILLS,
};

mod dock;
mod footprints;
mod history;
mod menus;
mod movement;
mod overlay_stability;
mod skills;
mod turn_refresh;

fn resolved_legacy(skills: &[SkillId]) -> labyrinth_rules::build::ResolvedBuild {
    let catalog = labyrinth_rules::catalog::ContentCatalog::builtin().expect("catalog");
    let catalog = labyrinth_rules::scenario::legacy_catalog(&catalog, [skills])
        .expect("explicit legacy catalog");
    catalog
        .resolve_build(&labyrinth_rules::scenario::legacy_build(skills))
        .expect("resolved build")
}

fn set_legacy_skills(
    snapshot: &mut labyrinth_rules::CombatSnapshot,
    id: ActorId,
    skills: &[SkillId],
) {
    let build = labyrinth_rules::scenario::legacy_build(skills);
    snapshot.catalog = labyrinth_rules::scenario::legacy_catalog(&snapshot.catalog, [skills])
        .expect("legacy equipment");
    for actor in &mut snapshot.actors {
        actor.resolved_build = snapshot
            .catalog
            .resolve_build(&actor.build)
            .expect("updated catalog identity");
    }
    let skills = snapshot
        .catalog
        .resolve_build(&build)
        .expect("resolved build");
    let actor = snapshot
        .actors
        .iter_mut()
        .find(|actor| actor.id == id)
        .expect("actor");
    actor.build = build;
    actor.resolved_build = skills;
    actor.skill_uses.clear();
}

fn fixture() -> LabyrinthView {
    let combat = (0..128)
        .filter_map(|seed| Combat::new(seed, DEFAULT_HERO_ROSTER).ok())
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
        assignment_revision: 1,
        company: DEFAULT_HERO_ROSTER
            .into_iter()
            .enumerate()
            .map(|(index, hero)| {
                let actor = ActorId(u16::try_from(index + 1).expect("actor"));
                crate::session::CompanyMember {
                    actor,
                    hero,
                    resolved_build: resolved_legacy(
                        HeroSetup::preset(actor, hero).skills.as_slice(),
                    ),
                    owner: 0,
                }
            })
            .collect(),
        players: DEFAULT_HERO_ROSTER
            .into_iter()
            .enumerate()
            .map(|(index, _hero)| crate::view::PlayerView {
                slot: u8::try_from(index).expect("six player index"),
                actors: if index == 0 {
                    (1..=6).map(ActorId).collect()
                } else {
                    Vec::new()
                },
                name: format!("Player {}", index + 1),
                occupied: true,
                connected: true,
                ready: true,
            })
            .collect(),
        ..LabyrinthView::default()
    }
}

fn network_ownership(view: &mut LabyrinthView) {
    view.local = false;
    for (index, member) in view.company.iter_mut().enumerate() {
        member.owner = u8::try_from(index).expect("six participant fixture");
    }
    for player in &mut view.players {
        player.actors = view
            .company
            .iter()
            .filter(|m| m.owner == player.slot)
            .map(|m| m.actor)
            .collect();
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
fn pointer_and_keyboard_commit_the_same_typed_owned_action_normal_1080() {
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
fn battlefield_and_status_identity_survive_snapshot_and_rank_changes_normal_1080() {
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
    let badge = find_named(app.world_mut(), "Actor 1 Effects").expect("bleed badge");
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
    assert_eq!(find_named(app.world_mut(), "Actor 1 Effects"), Some(badge));
    assert!(click_action(&mut app, badge));
    run_frames(&mut app, 2);
    let tree = ui_tree_snapshot(app.world_mut()).to_string();
    assert!(tree.contains("up to 2 turn-start ticks if retained"));
    let definition =
        find_named(app.world_mut(), "Tooltip Bleed ›").expect("condition definition link");
    assert!(click_action(&mut app, definition));
    run_frames(&mut app, 3);
    assert!(ui_tree_snapshot(app.world_mut())
        .to_string()
        .contains("next 3 turns"));
}

#[test]
fn invalid_skills_remain_inspectable_and_remote_ownership_blocks_commit_normal_1080() {
    let mut app = app(1920, 1080, UiScaleMode::Auto);
    network_ownership(&mut app.world_mut().resource_mut::<LabyrinthView>());
    let snapshot = app
        .world()
        .resource::<LabyrinthView>()
        .combat
        .clone()
        .expect("combat");
    let active = snapshot.active_actor.expect("active");
    let other = app
        .world()
        .resource::<LabyrinthView>()
        .players
        .iter()
        .position(|player| !player.actors.contains(&active))
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
    assert!(app
        .world()
        .get::<bevy_gamekit::ui::UiContextHelp>(confirm)
        .expect("disabled explanation")
        .body
        .contains("Waiting for your turn"));
}

#[test]
fn reconnect_overlay_traps_focus_and_restores_after_recovery_normal_1080() {
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
fn six_viewports_preserve_keyboard_reachability_and_semantic_regions_normal_1080() {
    six_viewports_preserve_keyboard_reachability_and_semantic_regions(
        1920,
        1080,
        UiScaleMode::Auto,
    );
}

#[test]
fn six_viewports_preserve_keyboard_reachability_and_semantic_regions_compatibility() {
    six_viewports_preserve_keyboard_reachability_and_semantic_regions(1280, 720, UiScaleMode::Auto);
    six_viewports_preserve_keyboard_reachability_and_semantic_regions(
        1280,
        720,
        UiScaleMode::Percent200,
    );
    six_viewports_preserve_keyboard_reachability_and_semantic_regions(
        1920,
        1080,
        UiScaleMode::Percent200,
    );
    six_viewports_preserve_keyboard_reachability_and_semantic_regions(
        3840,
        2160,
        UiScaleMode::Auto,
    );
    six_viewports_preserve_keyboard_reachability_and_semantic_regions(
        3840,
        2160,
        UiScaleMode::Percent200,
    );
}

fn six_viewports_preserve_keyboard_reachability_and_semantic_regions(
    width: u32,
    height: u32,
    scale: UiScaleMode,
) {
    let mut app = app(width, height, scale);
    let tree = ui_tree_snapshot(app.world_mut()).to_string();
    for name in [
        "Battle HUD",
        "Combat Action Rail",
        "Initiative Timeline",
        "Actor 1",
        "Actor 106",
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
        let viewport = Rect::from_corners(Vec2::ZERO, Vec2::new(width as f32, height as f32));
        let visible = visible_control_rect(app.world(), entity, viewport)
            .expect("focused control is not entirely clipped");
        assert!(
            visible.width() >= 43.5 && visible.height() >= 43.5,
            "{width}x{height} {scale:?} {:?}: {visible:?}",
            app.world().get::<Name>(entity)
        );
    }
}

#[test]
fn native_forms_clear_secret_buffers_and_emit_only_typed_intents_normal_1080() {
    native_forms_clear_secret_buffers_and_emit_only_typed_intents(1920, 1080, UiScaleMode::Auto);
}

#[test]
fn native_forms_clear_secret_buffers_and_emit_only_typed_intents_compatibility() {
    native_forms_clear_secret_buffers_and_emit_only_typed_intents(
        1280,
        720,
        UiScaleMode::Percent200,
    );
}

fn native_forms_clear_secret_buffers_and_emit_only_typed_intents(
    width: u32,
    height: u32,
    scale: UiScaleMode,
) {
    let mut app = app(width, height, scale);
    *app.world_mut().resource_mut::<LabyrinthView>() = LabyrinthView::default();
    run_frames(&mut app, 3);
    apply_action(app.world_mut(), Action::Form(Form::Multiplayer));
    run_frames(&mut app, 2);
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
fn stale_selection_is_not_relabelled_as_the_next_decision_or_encounter_normal_1080() {
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
fn damage_feedback_deduplicates_events_and_does_not_replay_on_recovery_normal_1080() {
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
fn leaving_the_browser_by_escape_stops_provider_activity_normal_1080() {
    leaving_the_browser_by_escape_stops_provider_activity(1920, 1080, UiScaleMode::Auto);
}

#[test]
fn leaving_the_browser_by_escape_stops_provider_activity_compatibility() {
    leaving_the_browser_by_escape_stops_provider_activity(1280, 720, UiScaleMode::Auto);
}

fn leaving_the_browser_by_escape_stops_provider_activity(
    width: u32,
    height: u32,
    scale: UiScaleMode,
) {
    let mut app = app(width, height, scale);
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
fn backing_out_of_admission_forms_cancels_pending_work_normal_1080() {
    backing_out_of_admission_forms_cancels_pending_work(1920, 1080, UiScaleMode::Auto);
}

#[test]
fn backing_out_of_admission_forms_cancels_pending_work_compatibility() {
    backing_out_of_admission_forms_cancels_pending_work(1280, 720, UiScaleMode::Auto);
}

fn backing_out_of_admission_forms_cancels_pending_work(
    width: u32,
    height: u32,
    scale: UiScaleMode,
) {
    for form in [Form::Host, Form::Direct, Form::Password] {
        for escape in [false, true] {
            let mut app = app(width, height, scale);
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
fn compact_scaled_admission_forms_scroll_all_controls_and_fields_into_view_normal_1080() {
    compact_scaled_admission_forms_scroll_all_controls_and_fields_into_view(
        1920,
        1080,
        UiScaleMode::Auto,
    );
}

#[test]
fn compact_scaled_admission_forms_scroll_all_controls_and_fields_into_view_compatibility() {
    compact_scaled_admission_forms_scroll_all_controls_and_fields_into_view(
        1280,
        720,
        UiScaleMode::Percent200,
    );
}

fn compact_scaled_admission_forms_scroll_all_controls_and_fields_into_view(
    width: u32,
    height: u32,
    scale: UiScaleMode,
) {
    let mut app = app(width, height, scale);
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
                Rect::from_corners(Vec2::ZERO, Vec2::new(width as f32, height as f32)),
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
fn default_720_battle_overview_shows_every_actor_identity_and_hp_without_scrolling_compatibility() {
    let mut app = app(1280, 720, UiScaleMode::Auto);
    for actor in [1, 2, 3, 4, 5, 6, 101, 102, 103, 104, 105, 106] {
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

#[test]
fn large_text_fits_actor_overlays_without_overlapping_neighbors_compatibility() {
    let mut app = app(1280, 720, UiScaleMode::Percent200);
    for actor in [1, 2, 3, 4, 5, 6, 101, 102, 103, 104, 105, 106] {
        let text = find_named(app.world_mut(), &format!("Actor {actor} Summary")).expect("summary");
        let glyphs = app
            .world()
            .get::<bevy::text::TextLayoutInfo>(text)
            .expect("shaped text");
        let node = app.world().get::<ComputedNode>(text).expect("text box");
        assert!(!glyphs.glyphs.is_empty(), "must measure real text");
        // Layout advance includes trailing spaces; test the rendered glyph quads.
        for glyph in &glyphs.glyphs {
            let rect = Rect::from_center_size(glyph.position, glyph.atlas_info.rect.size());
            if rect.width() > 0.0 && rect.height() > 0.0 {
                assert!(
                    rect.min.x >= -0.5 && rect.max.x <= node.size().x + 0.5,
                    "actor {actor} glyph {rect:?} overflows {:?}",
                    node.size()
                );
            }
        }
    }
}

#[test]
fn another_consumer_sees_activations_after_labyrinth_translates_them_normal_1080() {
    another_consumer_sees_activations_after_labyrinth_translates_them(
        1920,
        1080,
        UiScaleMode::Auto,
    );
}

#[test]
fn another_consumer_sees_activations_after_labyrinth_translates_them_compatibility() {
    another_consumer_sees_activations_after_labyrinth_translates_them(1280, 720, UiScaleMode::Auto);
}

fn another_consumer_sees_activations_after_labyrinth_translates_them(
    width: u32,
    height: u32,
    scale: UiScaleMode,
) {
    #[derive(Resource, Default)]
    struct Audit(Vec<Entity>);
    fn record(mut messages: MessageReader<UiActivated>, mut audit: ResMut<Audit>) {
        audit
            .0
            .extend(messages.read().map(|message| message.entity));
    }
    let mut app = app(width, height, scale);
    app.init_resource::<Audit>()
        .add_systems(Update, record.after(LabyrinthUiSystems::Input));
    let wait = find_named(app.world_mut(), "Wait").expect("wait");
    assert!(click_action(&mut app, wait));
    run_frames(&mut app, 3);
    assert_eq!(app.world().resource::<Audit>().0, [wait]);
    assert_eq!(
        app.world().resource::<UiState>().selected,
        Some(Choice::Wait)
    );
}

#[test]
fn semantic_scale_round_trip_preserves_actor_identity_and_baseline_dimensions_compatibility() {
    let mut app = app(1280, 720, UiScaleMode::Auto);
    let actor = find_named(app.world_mut(), "Actor 1").expect("actor");
    let baseline = app.world().get::<Node>(actor).expect("node").min_height;
    app.world_mut().resource_mut::<UiScalePreference>().0 = UiScaleMode::Percent200;
    run_frames(&mut app, 5);
    assert_eq!(find_named(app.world_mut(), "Actor 1"), Some(actor));
    app.world_mut().resource_mut::<UiScalePreference>().0 = UiScaleMode::Auto;
    run_frames(&mut app, 5);
    assert_eq!(
        app.world().get::<Node>(actor).expect("node").min_height,
        baseline
    );
    assert_eq!(find_named(app.world_mut(), "Actor 1"), Some(actor));
}

#[test]
fn facing_front_ranks_are_presentation_only_and_selection_is_distinct_normal_1080() {
    let mut app = app(1920, 1080, UiScaleMode::Auto);
    let before = app.world().resource::<LabyrinthView>().combat.clone();
    let heroes = find_named(app.world_mut(), "Your Company Ranks").expect("formation");
    let visual = app
        .world()
        .get::<Children>(heroes)
        .expect("tiles")
        .iter()
        .filter(|entity| {
            app.world()
                .get::<Name>(*entity)
                .is_some_and(|name| name.as_str().starts_with("Actor "))
        })
        .map(|entity| {
            app.world()
                .get::<Name>(entity)
                .expect("tile name")
                .as_str()
                .to_owned()
        })
        .collect::<Vec<_>>();
    assert_eq!(
        visual,
        [
            "Actor 6 Tile",
            "Actor 5 Tile",
            "Actor 4 Tile",
            "Actor 3 Tile",
            "Actor 2 Tile",
            "Actor 1 Tile"
        ]
    );
    let wait = find_named(app.world_mut(), "Wait").expect("wait");
    click_action(&mut app, wait);
    run_frames(&mut app, 3);
    assert!(app
        .world()
        .get::<UiSkinOverrides>(wait)
        .expect("skin")
        .background
        .is_some());
    assert_eq!(app.world().resource::<LabyrinthView>().combat, before);
}

#[test]
fn reduced_motion_uses_the_shared_preference_without_changing_rules_normal_1080() {
    reduced_motion_uses_the_shared_preference_without_changing_rules(1920, 1080, UiScaleMode::Auto);
}

#[test]
fn reduced_motion_uses_the_shared_preference_without_changing_rules_compatibility() {
    reduced_motion_uses_the_shared_preference_without_changing_rules(1280, 720, UiScaleMode::Auto);
}

fn reduced_motion_uses_the_shared_preference_without_changing_rules(
    width: u32,
    height: u32,
    scale: UiScaleMode,
) {
    let mut app = app(width, height, scale);
    let before = app.world().resource::<LabyrinthView>().combat.clone();
    apply_action(app.world_mut(), Action::ReducedMotion);
    assert!(app.world().resource::<UiMotionPreference>().reduced);
    run_frames(&mut app, 3);
    assert_eq!(app.world().resource::<LabyrinthView>().combat, before);
}

#[test]
fn repeated_classes_project_the_explicit_owner_not_the_first_class_or_slot_rank_normal_1080() {
    let mut app = app(1920, 1080, UiScaleMode::Auto);
    let mut combat =
        Combat::new(42, [HeroClass::Gatekeeper; PARTY_SIZE]).expect("repeated classes");
    for _ in 0..labyrinth_rules::MAX_ACTORS {
        let active = combat.snapshot().active_actor.expect("live decision");
        if active == ActorId(1) {
            break;
        }
        combat
            .apply(active, CombatAction::Wait)
            .expect("advance to first hero");
    }
    assert_eq!(combat.snapshot().active_actor, Some(ActorId(1)));
    {
        let mut view = app.world_mut().resource_mut::<LabyrinthView>();
        network_ownership(&mut view);
        view.player = Some(5);
        for member in &mut view.company {
            member.hero = HeroClass::Gatekeeper;
            member.resolved_build = resolved_legacy(
                HeroSetup::preset(member.actor, member.hero)
                    .skills
                    .as_slice(),
            );
            // Deliberately not slot+1: character control is independent of seats.
            if member.actor == ActorId(1) {
                member.owner = 5;
            }
            if member.actor == ActorId(6) {
                member.owner = 0;
            }
        }
        let company = view.company.clone();
        for player in &mut view.players {
            player.actors = company
                .iter()
                .filter(|m| m.owner == player.slot)
                .map(|m| m.actor)
                .collect();
        }
        view.combat = Some(combat.snapshot());
    }
    run_frames(&mut app, 4);
    apply_action(app.world_mut(), Action::Choice(Choice::Wait));
    assert_eq!(
        battle::selected_action(
            app.world().resource::<LabyrinthView>(),
            app.world().resource::<UiState>()
        ),
        Ok((ActorId(1), CombatAction::Wait))
    );
    let summary = find_named(app.world_mut(), "Actor 1").expect("owned actor");
    assert!(app
        .world()
        .get::<AccessibleLabel>(summary)
        .expect("accessible owner")
        .0
        .contains("Player 6"));
    app.world_mut().resource_mut::<LabyrinthView>().player = Some(0);
    run_frames(&mut app, 3);
    apply_action(app.world_mut(), Action::Choice(Choice::Wait));
    assert!(battle::selected_action(
        app.world().resource::<LabyrinthView>(),
        app.world().resource::<UiState>()
    )
    .is_err());
}

#[test]
fn ability_controls_follow_equipped_loadouts_with_eight_shortcuts_and_empty_loadouts_normal_1080() {
    let mut app = app(1920, 1080, UiScaleMode::Auto);
    let mut next_id = 0;
    let heroes = DEFAULT_HERO_ROSTER.map(|class| {
        next_id += 1;
        let mut hero = HeroSetup::preset(ActorId(next_id), class);
        hero.skills = LegacySkillLoadout::new(SkillId::ALL.into_iter().take(MAX_LEGACY_SKILLS))
            .expect("custom loadout");
        hero
    });
    {
        let mut view = app.world_mut().resource_mut::<LabyrinthView>();
        network_ownership(&mut view);
        view.player = Some(5);
        view.combat = Some(
            Combat::with_heroes(42, heroes)
                .expect("custom setup")
                .snapshot(),
        );
    }
    run_frames(&mut app, 4);
    let actor = find_named(app.world_mut(), "Actor 6").expect("sixth hero");
    assert!(find_named(app.world_mut(), "Skill 7").is_some());
    assert!(find_named(app.world_mut(), "Skill 8").is_none());
    tap_key(&mut app, KeyCode::Digit8);
    assert_eq!(
        app.world().resource::<UiState>().selected,
        Some(Choice::Skill(7))
    );
    {
        let mut view = app.world_mut().resource_mut::<LabyrinthView>();
        set_legacy_skills(view.combat.as_mut().expect("combat"), ActorId(6), &[]);
    }
    run_frames(&mut app, 4);
    assert_eq!(find_named(app.world_mut(), "Actor 6"), Some(actor));
    assert!(find_named(app.world_mut(), "Skill 0").is_none());
    assert!(find_named(app.world_mut(), "Wait").is_some());
    assert_eq!(app.world().resource::<UiState>().selected, None);
}

#[test]
fn six_participant_lobby_requires_controllers_ready_and_allows_unready_spectators_normal_1080() {
    six_participant_lobby_requires_controllers_ready_and_allows_unready_spectators(
        1920,
        1080,
        UiScaleMode::Auto,
    );
}

#[test]
fn six_participant_lobby_requires_controllers_ready_and_allows_unready_spectators_compatibility() {
    six_participant_lobby_requires_controllers_ready_and_allows_unready_spectators(
        1280,
        720,
        UiScaleMode::Auto,
    );
}

fn six_participant_lobby_requires_controllers_ready_and_allows_unready_spectators(
    width: u32,
    height: u32,
    scale: UiScaleMode,
) {
    let mut app = app(width, height, scale);
    {
        let mut view = app.world_mut().resource_mut::<LabyrinthView>();
        view.mode = ViewMode::Lobby;
        network_ownership(&mut view);
        view.invite_labels = (1..PARTY_SIZE)
            .map(|index| format!("Guest {index}"))
            .collect();
        view.players.last_mut().expect("sixth seat").ready = false;
    }
    run_frames(&mut app, 4);
    let start = find_named(app.world_mut(), "Start Encounter").expect("start");
    assert!(app.world().get::<UiDisabled>(start).is_some());
    apply_action(app.world_mut(), Action::LobbyPage(3));
    run_frames(&mut app, 3);
    assert!(find_named(app.world_mut(), "Copy Invitation 4").is_some());
    {
        let mut view = app.world_mut().resource_mut::<LabyrinthView>();
        view.players
            .last_mut()
            .expect("sixth participant")
            .actors
            .clear();
        view.players
            .first_mut()
            .expect("host")
            .actors
            .push(ActorId(6));
        view.company
            .iter_mut()
            .find(|member| member.actor == ActorId(6))
            .expect("hero")
            .owner = 0;
        view.player = Some(5);
    }
    run_frames(&mut app, 4);
    let start = find_named(app.world_mut(), "Start Encounter").expect("start");
    assert!(app.world().get::<UiDisabled>(start).is_none());
    let ready = find_named(app.world_mut(), "Toggle Ready").expect("spectator readiness");
    assert!(app.world().get::<UiDisabled>(ready).is_some());
    assert_eq!(app.world().resource::<LabyrinthView>().players.len(), 6);
}

#[test]
fn ability_and_target_selection_never_commit_without_explicit_confirmation_normal_1080() {
    ability_and_target_selection_never_commit_without_explicit_confirmation(
        1920,
        1080,
        UiScaleMode::Auto,
    );
}

#[test]
fn ability_and_target_selection_never_commit_without_explicit_confirmation_compatibility() {
    ability_and_target_selection_never_commit_without_explicit_confirmation(
        1280,
        720,
        UiScaleMode::Auto,
    );
}

fn ability_and_target_selection_never_commit_without_explicit_confirmation(
    width: u32,
    height: u32,
    scale: UiScaleMode,
) {
    for keyboard in [false, true] {
        let mut app = app(width, height, scale);
        let snapshot = app
            .world()
            .resource::<LabyrinthView>()
            .combat
            .clone()
            .expect("combat");
        let actor = snapshot.active_actor.expect("hero decision");
        let source = snapshot.actor(actor).expect("source");
        let (index, _skill, target) = source
            .legacy_skills()
            .iter()
            .enumerate()
            .find_map(|(index, skill)| {
                snapshot
                    .actors
                    .iter()
                    .find(|target| {
                        snapshot
                            .validate_action(
                                actor,
                                &CombatAction::LegacySkill {
                                    skill: *skill,
                                    target: target.id,
                                },
                            )
                            .is_ok()
                    })
                    .map(|target| (index, *skill, target.id))
            })
            .expect("legal equipped skill");
        let skill_control = find_named(app.world_mut(), &format!("Skill {index}")).expect("skill");
        let target_control =
            find_named(app.world_mut(), &format!("Actor {}", target.0)).expect("target");
        for control in [skill_control, target_control] {
            if keyboard {
                assert!(focus_action(app.world_mut(), control));
                tap_key(&mut app, KeyCode::Enter);
            } else {
                pointer_control(&mut app, control, Vec2::new(width as f32, height as f32));
            }
            run_frames(&mut app, 2);
            assert!(!app
                .world_mut()
                .resource_mut::<Messages<LabyrinthIntent>>()
                .drain()
                .any(|intent| matches!(intent, LabyrinthIntent::Combat { .. })));
            assert_eq!(
                app.world().resource::<LabyrinthView>().combat.as_ref(),
                Some(&snapshot)
            );
        }
        let confirm = find_named(app.world_mut(), "Confirm Combat Action").expect("confirm");
        assert!(activation_eligible(app.world_mut(), confirm));
        if keyboard {
            assert!(focus_action(app.world_mut(), confirm));
            tap_key(&mut app, KeyCode::Space);
        } else {
            pointer_control(&mut app, confirm, Vec2::new(width as f32, height as f32));
        }
        let intents: Vec<_> = app
            .world_mut()
            .resource_mut::<Messages<LabyrinthIntent>>()
            .drain()
            .collect();
        assert_eq!(
            intents
                .iter()
                .filter(|intent| matches!(intent, LabyrinthIntent::Combat { .. }))
                .count(),
            1
        );
        assert!(intents.iter().any(|intent| matches!(intent, LabyrinthIntent::Combat { actor: who, action: CombatAction::Skill { index: chosen, target: hit }, .. } if *who == actor && usize::from(*chosen) == index && *hit == target)));
    }
}

#[test]
fn all_twelve_art_hit_regions_are_initially_visible_and_detail_does_not_reflow_stage_normal_1080() {
    all_twelve_art_hit_regions_are_initially_visible_and_detail_does_not_reflow_stage(
        1920,
        1080,
        UiScaleMode::Auto,
    );
}

#[test]
fn all_twelve_art_hit_regions_are_initially_visible_and_detail_does_not_reflow_stage_compatibility()
{
    all_twelve_art_hit_regions_are_initially_visible_and_detail_does_not_reflow_stage(
        1280,
        720,
        UiScaleMode::Auto,
    );
    all_twelve_art_hit_regions_are_initially_visible_and_detail_does_not_reflow_stage(
        1280,
        720,
        UiScaleMode::Percent200,
    );
    all_twelve_art_hit_regions_are_initially_visible_and_detail_does_not_reflow_stage(
        1920,
        1080,
        UiScaleMode::Percent200,
    );
    all_twelve_art_hit_regions_are_initially_visible_and_detail_does_not_reflow_stage(
        3840,
        2160,
        UiScaleMode::Auto,
    );
    all_twelve_art_hit_regions_are_initially_visible_and_detail_does_not_reflow_stage(
        3840,
        2160,
        UiScaleMode::Percent200,
    );
}

fn all_twelve_art_hit_regions_are_initially_visible_and_detail_does_not_reflow_stage(
    width: u32,
    height: u32,
    scale: UiScaleMode,
) {
    let mut app = app(width, height, scale);
    let viewport = Rect::from_corners(Vec2::ZERO, Vec2::new(width as f32, height as f32));
    let stage = find_named(app.world_mut(), "Facing Formations").expect("stage");
    assert_eq!(
        app.world()
            .get::<Node>(stage)
            .expect("stage layout")
            .overflow,
        Overflow::DEFAULT
    );
    let mut rectangles = Vec::new();
    for id in [1, 2, 3, 4, 5, 6, 101, 102, 103, 104, 105, 106] {
        let entity = find_named(app.world_mut(), &format!("Actor {id}")).expect("actor");
        let rect = visible_control_rect(app.world(), entity, viewport)
            .expect("art visible before focus/scroll");
        assert!(
            rect.width() >= 43.5 && rect.height() >= 43.5,
            "{width} {scale:?} actor{id}: {rect:?}"
        );
        rectangles.push((entity, rect));
    }
    apply_action(app.world_mut(), Action::InspectActor(ActorId(1)));
    run_frames(&mut app, 3);
    for (entity, before) in rectangles {
        assert_eq!(
            visible_control_rect(app.world(), entity, viewport),
            Some(before)
        );
    }
    let close = find_named(app.world_mut(), "Tooltip Close").expect("close actor inspection");
    assert!(click_action(&mut app, close));
    assert!(app
        .world()
        .resource::<bevy_gamekit::ui::UiTooltipState>()
        .subjects()
        .is_empty());
}

/// Native UI hit-test evidence: unlike click_action, this never assigns Interaction.
fn pointer_at(app: &mut App, position: Vec2) {
    use bevy::input::{mouse::MouseButtonInput, ButtonState};
    let (entity, mut window) = app
        .world_mut()
        .query::<(Entity, &mut Window)>()
        .single_mut(app.world_mut())
        .expect("window");
    window.set_cursor_position(Some(position));
    run_frames(app, 2);
    for state in [ButtonState::Pressed, ButtonState::Released] {
        app.world_mut().write_message(MouseButtonInput {
            button: MouseButton::Left,
            state,
            window: entity,
        });
        app.update();
    }
}

fn pointer_control(app: &mut App, entity: Entity, viewport: Vec2) {
    let visible = visible_control_rect(
        app.world(),
        entity,
        Rect::from_corners(Vec2::ZERO, viewport),
    )
    .expect("native pointer target is visible without synthetic activation");
    assert!(visible.width() >= 43.5 && visible.height() >= 43.5);
    pointer_at(app, visible.center());
}

#[test]
fn game_menu_blocks_pointer_fallthrough_to_world_anchored_actor_controls_normal_1080() {
    game_menu_blocks_pointer_fallthrough_to_world_anchored_actor_controls(
        1920,
        1080,
        UiScaleMode::Auto,
    );
}

#[test]
fn game_menu_blocks_pointer_fallthrough_to_world_anchored_actor_controls_compatibility() {
    game_menu_blocks_pointer_fallthrough_to_world_anchored_actor_controls(
        1280,
        720,
        UiScaleMode::Auto,
    );
}

fn game_menu_blocks_pointer_fallthrough_to_world_anchored_actor_controls(
    width: u32,
    height: u32,
    scale: UiScaleMode,
) {
    let mut app = app(width, height, scale);
    let viewport = Rect::from_corners(Vec2::ZERO, Vec2::new(width as f32, height as f32));
    let target = find_named(app.world_mut(), "Actor 103").expect("target");
    let point = visible_control_rect(app.world(), target, viewport)
        .expect("bounds")
        .center();
    pointer_at(&mut app, point);
    assert_eq!(
        app.world().resource::<UiState>().target,
        Some(ActorId(103)),
        "real hit-test reaches actor"
    );
    apply_action(app.world_mut(), Action::Cancel);
    app.world_mut().resource_mut::<UiState>().target = None;
    apply_action(app.world_mut(), Action::Settings);
    run_frames(&mut app, 3);
    pointer_at(&mut app, point);
    assert_eq!(app.world().resource::<UiState>().target, None);
}

#[test]
fn history_is_non_modal_and_keyboard_can_reach_older_and_latest_entries_normal_1080() {
    history_is_non_modal_and_keyboard_can_reach_older_and_latest_entries(
        1920,
        1080,
        UiScaleMode::Auto,
    );
}

#[test]
fn history_is_non_modal_and_keyboard_can_reach_older_and_latest_entries_compatibility() {
    history_is_non_modal_and_keyboard_can_reach_older_and_latest_entries(
        1280,
        720,
        UiScaleMode::Percent200,
    );
}

fn history_is_non_modal_and_keyboard_can_reach_older_and_latest_entries(
    width: u32,
    height: u32,
    scale: UiScaleMode,
) {
    let mut app = app(width, height, scale);
    let actor = find_named(app.world_mut(), "Actor 1").expect("actor");
    assert!(focus_action(app.world_mut(), actor));
    {
        let mut view = app.world_mut().resource_mut::<LabyrinthView>();
        view.events = (1..=24)
            .map(|id| crate::view::PresentedEvent {
                id,
                event: labyrinth_rules::CombatEvent {
                    id,
                    kind: labyrinth_rules::CombatEventKind::Action {
                        actor: ActorId(1),
                        action: CombatAction::Wait,
                    },
                },
            })
            .collect();
    }
    apply_action(app.world_mut(), Action::ToggleLog);
    run_frames(&mut app, 4);
    assert!(activation_eligible(app.world_mut(), actor));
    let close = find_named(app.world_mut(), "History Toggle").expect("collapse");
    assert!(activation_eligible(app.world_mut(), close));
    apply_action(app.world_mut(), Action::Choice(Choice::Wait));
    assert!(battle::selected_action(
        app.world().resource::<LabyrinthView>(),
        app.world().resource::<UiState>()
    )
    .is_ok());
    let scroll = find_named(app.world_mut(), "History Scroll").expect("scroll");
    tap_key(&mut app, KeyCode::End);
    run_frames(&mut app, 3);
    let node = app.world().get::<ComputedNode>(scroll).expect("computed");
    let maximum = (node.content_size().y - node.size().y) * node.inverse_scale_factor;
    assert!(maximum > 100.0, "fixture must overflow");
    let offset = app
        .world()
        .get::<ScrollPosition>(scroll)
        .expect("position")
        .0
        .y;
    assert!(
        (offset - maximum).abs() < 1.0,
        "last content reachable: {offset}/{maximum}"
    );
    tap_key(&mut app, KeyCode::Home);
    run_frames(&mut app, 2);
    assert_eq!(
        app.world()
            .get::<ScrollPosition>(scroll)
            .expect("position")
            .0
            .y,
        0.0
    );
    let retained = find_named(app.world_mut(), "History Entry 1").expect("stable old entry");
    app.world_mut()
        .resource_mut::<LabyrinthView>()
        .events
        .push(crate::view::PresentedEvent {
            id: 25,
            event: labyrinth_rules::CombatEvent {
                id: 25,
                kind: labyrinth_rules::CombatEventKind::Action {
                    actor: ActorId(2),
                    action: CombatAction::Wait,
                },
            },
        });
    run_frames(&mut app, 3);
    assert_eq!(
        find_named(app.world_mut(), "History Entry 1"),
        Some(retained)
    );
    assert_eq!(
        app.world()
            .get::<ScrollPosition>(scroll)
            .expect("position")
            .y,
        0.0
    );
    assert_eq!(
        app.world()
            .get::<bevy_gamekit::ui::UiFeedScroll>(scroll)
            .expect("feed")
            .unread(),
        1
    );
    let latest = find_named(app.world_mut(), "History Latest").expect("latest");
    assert!(click_action(&mut app, latest));
    run_frames(&mut app, 3);
    assert_eq!(
        app.world()
            .get::<bevy_gamekit::ui::UiFeedScroll>(scroll)
            .expect("feed")
            .unread(),
        0
    );
    assert!(
        app.world()
            .get::<ScrollPosition>(scroll)
            .expect("position")
            .y
            > 100.0
    );
    assert!(focus_action(app.world_mut(), actor));
    tap_key(&mut app, KeyCode::Escape);
    run_frames(&mut app, 3);
    assert_eq!(
        app.world().resource::<UiState>().menus.current(),
        Some(&MenuPage::Game)
    );
    tap_key(&mut app, KeyCode::Escape);
    run_frames(&mut app, 3);
    assert!(activation_eligible(app.world_mut(), actor));
    assert_eq!(app.world().resource::<InputFocus>().get(), Some(actor));
}

#[test]
fn multiple_owned_characters_follow_active_turn_and_spectators_cannot_confirm_normal_1080() {
    multiple_owned_characters_follow_active_turn_and_spectators_cannot_confirm(
        1920,
        1080,
        UiScaleMode::Auto,
    );
}

#[test]
fn multiple_owned_characters_follow_active_turn_and_spectators_cannot_confirm_compatibility() {
    multiple_owned_characters_follow_active_turn_and_spectators_cannot_confirm(
        1280,
        720,
        UiScaleMode::Auto,
    );
}

fn multiple_owned_characters_follow_active_turn_and_spectators_cannot_confirm(
    width: u32,
    height: u32,
    scale: UiScaleMode,
) {
    let mut app = app(width, height, scale);
    let active = app
        .world()
        .resource::<LabyrinthView>()
        .combat
        .as_ref()
        .expect("combat")
        .active_actor
        .expect("hero");
    let other = app
        .world()
        .resource::<LabyrinthView>()
        .company
        .iter()
        .find(|member| member.actor != active)
        .expect("second hero")
        .actor;
    {
        let mut view = app.world_mut().resource_mut::<LabyrinthView>();
        view.local = false;
        view.host = false;
        view.player = Some(1);
        for member in &mut view.company {
            member.owner = if [active, other].contains(&member.actor) {
                1
            } else {
                0
            };
        }
        let company = view.company.clone();
        for player in &mut view.players {
            player.actors = company
                .iter()
                .filter(|m| m.owner == player.slot)
                .map(|m| m.actor)
                .collect();
        }
    }
    run_frames(&mut app, 3);
    let wait = find_named(app.world_mut(), "Wait").expect("universal action");
    assert!(click_action(&mut app, wait));
    let confirm = find_named(app.world_mut(), "Confirm Combat Action").expect("confirmation");
    assert!(activation_eligible(app.world_mut(), confirm));
    assert!(click_action(&mut app, confirm));
    assert!(app.world_mut().resource_mut::<Messages<LabyrinthIntent>>().drain().any(|intent| matches!(intent, LabyrinthIntent::Combat { actor, assignment_revision: 1, .. } if actor == active)));
    app.world_mut().resource_mut::<LabyrinthView>().player = Some(2);
    run_frames(&mut app, 3);
    let confirm = find_named(app.world_mut(), "Confirm Combat Action").expect("confirmation");
    assert!(!activation_eligible(app.world_mut(), confirm));
    tap_key(&mut app, KeyCode::Digit1);
    tap_key(&mut app, KeyCode::Enter);
    assert!(!app
        .world_mut()
        .resource_mut::<Messages<LabyrinthIntent>>()
        .drain()
        .any(|intent| matches!(intent, LabyrinthIntent::Combat { .. })));
}
