//! Production-layout/input tests; rendered quality is reviewed separately.
use super::*;
use bevy_gamekit::testing::{
    find_named, focus_action, run_frames, tap_key, visible_control_rect, TestAppBuilder,
};
use labyrinth_rules::{
    catalog::ContentCatalog,
    scenario::{Scenario, StockScenario},
    ActorKind,
};

fn app(scale: UiScaleMode) -> App {
    let catalog = ContentCatalog::builtin().unwrap();
    let scenario = Scenario::stock(StockScenario::WeaponComparison, 42, &catalog).unwrap();
    let company = scenario
        .heroes
        .iter()
        .map(|actor| crate::session::CompanyMember {
            actor: actor.id,
            hero: match actor.actor.appearance {
                ActorKind::Hero(hero) => hero,
                _ => unreachable!(),
            },
            abilities: actor.actor.resolve(&catalog).unwrap(),
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
            scenario: Some(scenario),
            catalog: Some(catalog),
            company,
            players,
            ..default()
        })
        .insert_resource(UiScalePreference(scale))
        .add_plugins(LabyrinthUiPlugin);
    let mut app = builder.build();
    run_frames(&mut app, 5);
    app
}

fn activate(app: &mut App, name: &str) {
    let entity = find_named(app.world_mut(), name).expect(name);
    assert!(focus_action(app.world_mut(), entity), "{name}");
    run_frames(app, 3);
    tap_key(app, KeyCode::Enter);
    run_frames(app, 3);
}

#[test]
fn preparation_sections_keep_one_character_entry_and_preserve_scenario_draft() {
    let mut app = app(UiScaleMode::Auto);
    assert!(find_named(app.world_mut(), "Heroes Setup Title").is_some());
    assert!(find_named(app.world_mut(), "Enemies Setup Title").is_none());
    assert!(find_named(app.world_mut(), "Scenario seed").is_none());
    activate(&mut app, "Preparation Enemies");
    assert!(find_named(app.world_mut(), "Heroes Setup Title").is_none());
    assert!(find_named(app.world_mut(), "Enemies Setup Title").is_some());
    let enemy = app
        .world()
        .resource::<LabyrinthView>()
        .scenario
        .as_ref()
        .unwrap()
        .enemies[0]
        .id;
    let button = find_named(app.world_mut(), &format!("Edit Actor {}", enemy.0)).unwrap();
    assert!(
        matches!(app.world().get::<Action>(button), Some(Action::Setup(setup::SetupAction::Edit(id))) if *id == enemy)
    );
    activate(&mut app, "Preparation Scenario");
    assert!(find_named(app.world_mut(), "Scenario seed").is_some());
    let field = find_named(app.world_mut(), "Scenario seed").unwrap();
    {
        let mut text = app
            .world_mut()
            .get_mut::<bevy::text::EditableText>(field)
            .unwrap();
        text.queue_edit(bevy::text::TextEdit::SelectAll);
        text.queue_edit(bevy::text::TextEdit::Insert("771".into()));
    }
    run_frames(&mut app, 3);
    activate(&mut app, "Preparation Party");
    activate(&mut app, "Preparation Scenario");
    assert_eq!(app.world().resource::<UiState>().scenario_seed, "771");
    assert_eq!(
        app.world()
            .resource::<LabyrinthView>()
            .scenario
            .as_ref()
            .unwrap()
            .seed,
        42
    );
    activate(&mut app, "Preparation Players");
    assert!(find_named(app.world_mut(), "Player 0").is_some());
    assert!(find_named(app.world_mut(), "Scenario seed").is_none());
}

#[test]
fn preparation_footer_stays_visible_while_roster_scrolls_at_supported_scales() {
    for scale in [UiScaleMode::Auto, UiScaleMode::Percent200] {
        let mut app = app(scale);
        for page in [
            "Preparation Party",
            "Preparation Enemies",
            "Preparation Scenario",
            "Preparation Players",
        ] {
            activate(&mut app, page);
            for name in [
                "Toggle Ready",
                "Start Encounter",
                "Lobby Settings",
                "Lobby Leave",
            ] {
                let entity = find_named(app.world_mut(), name).unwrap();
                let visible = visible_control_rect(
                    app.world(),
                    entity,
                    Rect::from_corners(Vec2::ZERO, Vec2::new(1280.0, 720.0)),
                )
                .unwrap_or_else(|| panic!("{page} {scale:?}: {name} clipped"));
                let node = app.world().get::<ComputedNode>(entity).unwrap();
                let expected = node.size() * node.inverse_scale_factor;
                assert!(
                    visible.width() + 0.5 >= expected.x && visible.height() + 0.5 >= expected.y,
                    "{page} {scale:?}: {name} {visible:?}"
                );
            }
        }
    }
}
