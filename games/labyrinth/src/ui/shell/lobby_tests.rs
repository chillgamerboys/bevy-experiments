//! Production plugin, layout and keyboard input evidence; desktop pixels are separate.
use super::*;
use bevy_gamekit::testing::{
    find_named, focus_action, run_frames, tap_key, visible_control_rect, TestAppBuilder,
};
use bevy_gamekit::ui::UiDisabled;
use labyrinth_rules::{
    catalog::ContentCatalog,
    scenario::{Scenario, StockScenario},
    ActorKind, Team,
};

#[derive(Resource, Default)]
struct Captured(Vec<LabyrinthIntent>);
fn capture_intents(mut messages: MessageReader<LabyrinthIntent>, mut captured: ResMut<Captured>) {
    captured.0.extend(messages.read().cloned());
}
fn app(scale: UiScaleMode, sparse: bool) -> App {
    let catalog = ContentCatalog::builtin().expect("catalog");
    let mut scenario = Scenario::stock(StockScenario::Prototype, 42, &catalog).expect("scenario");
    let mut formation = crate::view::LobbyFormation::compact(&scenario);
    if sparse {
        let removed = scenario.heroes.remove(1).id;
        formation.heroes.retain(|p| p.actor != removed);
    }
    let company = scenario
        .heroes
        .iter()
        .map(|a| crate::view::CompanyMember {
            actor: a.id,
            hero: match a.actor.appearance {
                ActorKind::Hero(hero) => hero,
                _ => unreachable!(),
            },
            abilities: a.actor.resolve(&catalog).expect("build"),
            owner: 0,
        })
        .collect();
    let players = vec![crate::view::PlayerView {
        slot: 0,
        actors: scenario.heroes.iter().map(|a| a.id).collect(),
        name: "Captain".into(),
        occupied: true,
        connected: true,
        ready: true,
    }];
    let mut builder = TestAppBuilder::new().with_ui(1280, 720);
    builder
        .app_mut()
        .insert_resource(LabyrinthView {
            mode: ViewMode::Lobby,
            local: true,
            host: true,
            admitted: true,
            player: Some(0),
            setup_revision: 8,
            deployment_error: formation.deployment_error(&scenario),
            formation: Some(formation),
            scenario: Some(scenario),
            catalog: Some(catalog),
            company,
            players,
            ..default()
        })
        .insert_resource(UiScalePreference(scale))
        .add_plugins(LabyrinthUiPlugin)
        .init_resource::<Captured>()
        .add_systems(Update, capture_intents.after(LabyrinthUiSystems::Input));
    let mut app = builder.build();
    run_frames(&mut app, 6);
    app
}
fn activate(app: &mut App, name: &str) {
    let entity = find_named(app.world_mut(), name).expect(name);
    assert!(focus_action(app.world_mut(), entity), "{name}");
    run_frames(app, 3);
    tap_key(app, KeyCode::Enter);
    run_frames(app, 3);
}
fn text(app: &mut App, name: &str) -> String {
    let entity = find_named(app.world_mut(), name).expect(name);
    app.world().get::<Text>(entity).expect("text").0.clone()
}
fn disabled(app: &mut App, name: &str) -> bool {
    let entity = find_named(app.world_mut(), name).expect(name);
    app.world().get::<UiDisabled>(entity).is_some()
}
fn full_control(app: &mut App, name: &str) {
    let entity = find_named(app.world_mut(), name).expect(name);
    let visible = visible_control_rect(
        app.world(),
        entity,
        Rect::from_corners(Vec2::ZERO, Vec2::new(1280.0, 720.0)),
    )
    .expect("visible control");
    let node = app.world().get::<ComputedNode>(entity).expect("computed");
    let size = node.size() * node.inverse_scale_factor;
    assert!(
        visible.width() + 0.5 >= size.x && visible.height() + 0.5 >= size.y,
        "{name}: {visible:?} versus {size:?}"
    );
}

#[test]
fn spatial_rank_selection_inspects_type_before_explicit_revision_bound_placement() {
    let mut app = app(UiScaleMode::Auto, true);
    let original = app.world().resource::<LabyrinthView>().scenario.clone();
    for team in ["Heroes", "Enemies"] {
        for rank in 1..=6 {
            assert!(find_named(app.world_mut(), &format!("{team} Rank {rank} Label")).is_some());
        }
    }
    assert!(disabled(&mut app, "Start Encounter"));
    assert_eq!(
        text(&mut app, "Readiness Summary"),
        "Fill party rank 2 to deploy"
    );
    activate(&mut app, "Select Heroes Rank 2");
    assert!(find_named(app.world_mut(), "Commit Placement").is_none());
    activate(&mut app, "Inspect Type scout");
    assert_eq!(app.world().resource::<LabyrinthView>().scenario, original);
    assert!(app.world().resource::<Captured>().0.last().is_none());
    let facts = text(&mut app, "Type Move back_rank_shot Facts");
    assert!(
        facts.contains("Acting ranks:")
            && facts.contains("Target ranks:")
            && facts.contains("Unavailable at current rank 2")
    );
    assert!(find_named(app.world_mut(), "Placement Ghost").is_some());
    activate(&mut app, "Commit Placement");
    assert!(
        matches!(app.world().resource::<Captured>().0.last(),Some(LabyrinthIntent::PlaceScenarioActor{team:Team::Heroes,rank:2,preset,expected_revision:8}) if preset.as_str()=="scout")
    );
}
#[test]
fn multi_rank_collision_is_explained_and_cancel_preserves_the_original_subject() {
    let mut app = app(UiScaleMode::Auto, false);
    activate(&mut app, "Select Heroes Rank 4");
    activate(&mut app, "Replace Selected Actor");
    activate(&mut app, "Inspect Type lantern_wagon");
    assert!(disabled(&mut app, "Commit Placement"));
    assert!(text(&mut app, "Placement Error").contains("occupied"));
    activate(&mut app, "Close Construction Context");
    assert!(find_named(app.world_mut(), "Edit Actor 4").is_some());
    activate(&mut app, "Move Actor 4");
    activate(&mut app, "Select Heroes Rank 1");
    assert!(disabled(&mut app, "Commit Placement"));
    activate(&mut app, "Close Construction Context");
    assert!(text(&mut app, "Selected Place").contains("Rank 4"));
}
#[test]
fn owner_assignment_targets_empty_places_and_guest_authority_is_visible() {
    let mut app = app(UiScaleMode::Auto, true);
    {
        let mut view = app.world_mut().resource_mut::<LabyrinthView>();
        view.local = false;
        view.players.push(crate::view::PlayerView {
            slot: 1,
            actors: Vec::new(),
            name: "Mira".into(),
            occupied: true,
            connected: true,
            ready: false,
        });
    }
    run_frames(&mut app, 4);
    activate(&mut app, "Select Heroes Rank 2");
    activate(&mut app, "Assign Selected Place");
    activate(&mut app, "Assign Place To 1");
    assert!(matches!(
        app.world().resource::<Captured>().0.last(),
        Some(LabyrinthIntent::AssignFormationRank {
            rank: 2,
            owner: 1,
            expected_revision: 8
        })
    ));
    {
        let mut view = app.world_mut().resource_mut::<LabyrinthView>();
        view.host = false;
        view.player = Some(1);
        *view
            .formation
            .as_mut()
            .expect("formation")
            .hero_owners
            .get_mut(1)
            .expect("rank") = 1;
    }
    run_frames(&mut app, 4);
    activate(&mut app, "Select Heroes Rank 2");
    activate(&mut app, "Inspect Type scout");
    assert!(!disabled(&mut app, "Commit Placement"));
    activate(&mut app, "Select Heroes Rank 1");
    assert!(disabled(&mut app, "Edit Actor 1"));
    assert!(find_named(app.world_mut(), "Assign Selected Place").is_none());
}
#[test]
fn constructor_preserves_one_editor_and_secondary_seed_draft() {
    let mut app = app(UiScaleMode::Auto, false);
    activate(&mut app, "Select Enemies Rank 1");
    let enemy = app
        .world()
        .resource::<LabyrinthView>()
        .scenario
        .as_ref()
        .expect("scenario")
        .enemies
        .first()
        .expect("enemy")
        .id;
    activate(&mut app, &format!("Edit Actor {}", enemy.0));
    assert!(app.world().resource::<UiState>().editor.is_some());
    tap_key(&mut app, KeyCode::Escape);
    run_frames(&mut app, 3);
    assert!(text(&mut app, "Selected Place").contains("Enemies"));
    activate(&mut app, "Preparation Scenario");
    let field = find_named(app.world_mut(), "Scenario seed").expect("seed");
    {
        let mut edit = app
            .world_mut()
            .get_mut::<bevy::text::EditableText>(field)
            .expect("editable");
        edit.queue_edit(bevy::text::TextEdit::SelectAll);
        edit.queue_edit(bevy::text::TextEdit::Insert("771".into()));
    }
    run_frames(&mut app, 3);
    activate(&mut app, "Preparation Scenario");
    activate(&mut app, "Preparation Scenario");
    assert_eq!(app.world().resource::<UiState>().scenario_seed, "771");
    activate(&mut app, "Apply Scenario Seed");
    assert!(matches!(
        app.world().resource::<Captured>().0.last(),
        Some(LabyrinthIntent::SetScenarioSeed {
            seed: 771,
            expected_revision: 8
        })
    ));
}
#[test]
fn footer_and_inspection_are_reachable_at_auto_and_two_hundred_percent() {
    for scale in [UiScaleMode::Auto, UiScaleMode::Percent200] {
        let mut app = app(scale, true);
        activate(&mut app, "Select Heroes Rank 2");
        activate(&mut app, "Inspect Type scout");
        full_control(&mut app, "Commit Placement");
        full_control(&mut app, "Start Encounter");
        let board = find_named(app.world_mut(), "Construction Battlefield").expect("board");
        let node = app
            .world()
            .get::<ComputedNode>(board)
            .expect("board layout");
        assert!(node.size().x > 1000.0);
        assert!(text(&mut app, "Type Move back_rank_shot Facts").contains("Acting ranks:"));
        if scale == UiScaleMode::Percent200 {
            activate(&mut app, "Back To Types");
            assert!(find_named(app.world_mut(), "Character Type Picker").is_some());
        }
    }
}
#[test]
fn replaced_authority_requires_a_new_placement_review() {
    let mut app = app(UiScaleMode::Auto, true);
    activate(&mut app, "Select Heroes Rank 2");
    activate(&mut app, "Inspect Type scout");
    app.world_mut()
        .resource_mut::<LabyrinthView>()
        .setup_revision += 1;
    run_frames(&mut app, 4);
    assert!(disabled(&mut app, "Commit Placement"));
    assert!(text(&mut app, "Placement Error").contains("formation changed"));
}

#[test]
fn queued_type_or_rank_changes_cannot_commit_a_candidate_the_player_has_not_seen() {
    use bevy::ecs::system::RunSystemOnce;
    for order in [0, 1, 2] {
        let mut app = app(UiScaleMode::Auto, true);
        activate(&mut app, "Select Heroes Rank 2");
        activate(&mut app, "Inspect Type scout");
        let commit =
            find_named(app.world_mut(), "Commit Placement").expect("mounted Scout confirmation");
        let other = find_named(app.world_mut(), "Inspect Type gatekeeper").expect("other type");
        let batch = match order {
            0 => vec![commit],
            1 => vec![other, commit],
            _ => vec![commit, other],
        };
        for entity in batch {
            app.world_mut().write_message(UiActivated { entity });
        }
        app.world_mut()
            .run_system_once(crate::ui::collect_actions)
            .expect("production input translation");
        let intents = app
            .world_mut()
            .resource_mut::<Messages<LabyrinthIntent>>()
            .drain()
            .collect::<Vec<_>>();
        if order == 1 {
            assert!(
                intents.is_empty(),
                "newly inspected type was never previewed"
            );
        } else {
            assert!(
                matches!(intents.as_slice(),[LabyrinthIntent::PlaceScenarioActor{rank:2,preset,..}] if preset.as_str()=="scout")
            );
        }
    }
}
#[test]
fn a_two_rank_character_can_move_into_its_own_second_rank_with_explicit_confirmation() {
    let mut app = app(UiScaleMode::Auto, false);
    {
        let mut view = app.world_mut().resource_mut::<LabyrinthView>();
        view.scenario
            .as_mut()
            .expect("scenario")
            .heroes
            .retain(|a| a.id != ActorId(4));
        let formation = view.formation.as_mut().expect("formation");
        formation.heroes.retain(|p| p.actor != ActorId(4));
        formation
            .heroes
            .iter_mut()
            .find(|p| p.actor == ActorId(5))
            .expect("wagon")
            .rank = 4;
    }
    run_frames(&mut app, 4);
    activate(&mut app, "Select Heroes Rank 4");
    activate(&mut app, "Move Actor 5");
    activate(&mut app, "Select Heroes Rank 5");
    assert!(!disabled(&mut app, "Commit Placement"));
    activate(&mut app, "Commit Placement");
    assert!(matches!(
        app.world().resource::<Captured>().0.last(),
        Some(LabyrinthIntent::MoveScenarioActor {
            actor: ActorId(5),
            rank: 5,
            expected_revision: 8
        })
    ));
}
#[test]
fn queued_owner_assignment_does_not_follow_a_different_selected_place() {
    use bevy::ecs::system::RunSystemOnce;
    let mut app = app(UiScaleMode::Auto, true);
    {
        let mut view = app.world_mut().resource_mut::<LabyrinthView>();
        view.local = false;
        view.players.push(crate::view::PlayerView {
            slot: 1,
            actors: Vec::new(),
            name: "Mira".into(),
            occupied: true,
            connected: true,
            ready: false,
        });
    }
    run_frames(&mut app, 4);
    activate(&mut app, "Select Heroes Rank 2");
    activate(&mut app, "Assign Selected Place");
    let assign = find_named(app.world_mut(), "Assign Place To 1").expect("bound owner choice");
    let other = find_named(app.world_mut(), "Select Heroes Rank 3").expect("different rank");
    for entity in [other, assign] {
        app.world_mut().write_message(UiActivated { entity });
    }
    app.world_mut()
        .run_system_once(crate::ui::collect_actions)
        .expect("production input translation");
    assert!(app
        .world_mut()
        .resource_mut::<Messages<LabyrinthIntent>>()
        .drain()
        .next()
        .is_none());
}

#[test]
fn queued_move_destination_cannot_retarget_a_mounted_confirmation() {
    use bevy::ecs::system::RunSystemOnce;
    let mut app = app(UiScaleMode::Auto, true);
    activate(&mut app, "Select Heroes Rank 1");
    activate(&mut app, "Move Actor 1");
    activate(&mut app, "Select Heroes Rank 2");
    let old = find_named(app.world_mut(), "Commit Placement").expect("preview at rank 2");
    let changed = find_named(app.world_mut(), "Select Heroes Rank 1").expect("new destination");
    for entity in [changed, old] {
        app.world_mut().write_message(UiActivated { entity });
    }
    app.world_mut()
        .run_system_once(crate::ui::collect_actions)
        .expect("production input translation");
    assert!(app
        .world_mut()
        .resource_mut::<Messages<LabyrinthIntent>>()
        .drain()
        .next()
        .is_none());
}
#[test]
fn reconnecting_guest_can_inspect_but_cannot_commit_and_keyboard_scroll_reaches_full_moves() {
    let mut app = app(UiScaleMode::Percent200, true);
    activate(&mut app, "Select Heroes Rank 2");
    activate(&mut app, "Inspect Type scout");
    tap_key(&mut app, KeyCode::PageDown);
    run_frames(&mut app, 3);
    let scroll = find_named(app.world_mut(), "Construction Detail Scroll").expect("detail scroll");
    assert!(
        app.world()
            .get::<ScrollPosition>(scroll)
            .expect("scroll position")
            .y
            > 0.0
    );
    {
        let mut view = app.world_mut().resource_mut::<LabyrinthView>();
        view.local = false;
        view.host = false;
        view.admitted = false;
    }
    run_frames(&mut app, 4);
    assert!(disabled(&mut app, "Commit Placement"));
    assert!(text(&mut app, "Placement Error").contains("Reconnect"));
}
