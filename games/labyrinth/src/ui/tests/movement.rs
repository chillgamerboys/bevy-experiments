//! Production UI projection and input checks, separate from rendered/native evidence.
use super::*;
use crate::presentation::{ActorDisclosure, CombatDisclosure};

fn movement_app(width: u32, height: u32, scale: UiScaleMode) -> (App, Combat) {
    let mut combat = Combat::with_party(
        42,
        labyrinth_rules::PROTOTYPE_HERO_ROSTER
            .into_iter()
            .enumerate()
            .map(|(i, hero)| HeroSetup::preset(ActorId(i as u16 + 1), hero))
            .collect(),
    )
    .expect("party");
    for _ in 0..24 {
        let actor = combat.snapshot().active_actor.expect("active");
        if actor == ActorId(1) {
            break;
        }
        combat.apply(actor, CombatAction::Wait).expect("wait");
    }
    let mut view = fixture();
    view.combat = Some(combat.snapshot());
    let mut app = App::new();
    app.add_plugins(HeadlessUiPlugin::new(width, height))
        .insert_resource(UiScalePreference(scale))
        .insert_resource(view)
        .add_plugins(LabyrinthUiPlugin);
    app.finish();
    app.cleanup();
    run_frames(&mut app, 5);
    (app, combat)
}

fn select(app: &mut App, target: ActorId, keyboard: bool) {
    let slot = app
        .world()
        .resource::<LabyrinthView>()
        .combat
        .as_ref()
        .and_then(|snapshot| snapshot.actor(ActorId(1)))
        .and_then(|actor| {
            actor
                .skills()
                .iter()
                .position(|skill| *skill == SkillId::DrivingBlow)
        })
        .expect("equipped Driving Blow");
    let skill =
        find_named(app.world_mut(), &format!("Skill {slot}")).expect("Driving Blow control");
    if keyboard {
        assert!(focus_action(app.world_mut(), skill));
        tap_key(app, KeyCode::Enter);
    } else {
        assert!(click_action(app, skill));
    }
    run_frames(app, 2);
    let target = find_named(app.world_mut(), &format!("Actor {}", target.0)).expect("target");
    if keyboard {
        assert!(focus_action(app.world_mut(), target));
        tap_key(app, KeyCode::Space);
    } else {
        assert!(click_action(app, target));
    }
    run_frames(app, 3);
}

fn actor_geometry(app: &mut App) -> Vec<(ActorId, Vec2, Vec2)> {
    let mut geometry = app
        .world_mut()
        .query::<(Entity, &crate::scene::SceneActorAnchor)>()
        .iter(app.world())
        .map(|(entity, anchor)| {
            (
                anchor.actor,
                app.world()
                    .get::<ComputedNode>(entity)
                    .expect("bounds")
                    .size(),
                app.world()
                    .get::<UiGlobalTransform>(entity)
                    .expect("position")
                    .translation,
            )
        })
        .collect::<Vec<_>>();
    geometry.sort_by_key(|(actor, _, _)| *actor);
    geometry
}

#[test]
fn movement_preview_is_non_mutating_and_pointer_keyboard_confirm_matches_it() {
    for (width, height, scale, keyboard) in [
        (1280, 720, UiScaleMode::Auto, false),
        (1920, 1080, UiScaleMode::Auto, true),
        (1280, 720, UiScaleMode::Percent200, true),
    ] {
        let (mut app, mut combat) = movement_app(width, height, scale);
        let before = actor_geometry(&mut app);
        let snapshot = combat.snapshot();
        select(&mut app, ActorId(104), keyboard);
        assert_eq!(
            app.world().resource::<UiState>().selected,
            Some(Choice::Skill(SkillId::DrivingBlow))
        );
        assert_eq!(app.world().resource::<UiState>().target, Some(ActorId(104)));
        let preview = crate::presentation::ForecastDisplay::build(
            &snapshot,
            app.world().resource::<CombatDisclosure>(),
            ActorId(1),
            &CombatAction::Skill {
                skill: SkillId::DrivingBlow,
                target: ActorId(104),
            },
        )
        .expect("known legal movement forecast");
        assert!(
            preview.movement.is_some(),
            "shared resolver must expose attempted movement: {preview:?}"
        );
        assert_eq!(
            actor_geometry(&mut app),
            before,
            "a preview cannot move live anchors"
        );
        assert_eq!(
            app.world().resource::<LabyrinthView>().combat.as_ref(),
            Some(&snapshot)
        );
        assert!(app
            .world_mut()
            .resource_mut::<Messages<LabyrinthIntent>>()
            .drain()
            .next()
            .is_none());
        let marker = find_named(app.world_mut(), "Actor 104 Landing Marker").expect("destination");
        assert_eq!(
            app.world().get::<Text>(marker).expect("ranks").0,
            "Iron\nBrute\n2 → 4"
        );
        assert!(app.world().get::<Button>(marker).is_none());
        assert!(app.world().get::<Action>(marker).is_none());
        assert!(app.world().get::<Interaction>(marker).is_none());
        let hauler =
            find_named(app.world_mut(), "Actor 101 Landing Marker").expect("whole footprint");
        assert_eq!(
            app.world().get::<Text>(hauler).expect("ranks").0,
            "Ossuary\nHauler\n3–4 → 2–3"
        );
        assert!(
            app.world()
                .get::<ComputedNode>(hauler)
                .expect("span")
                .size()
                .x
                > app
                    .world()
                    .get::<ComputedNode>(marker)
                    .expect("span")
                    .size()
                    .x
                    * 1.9
        );
        let confirm = find_named(app.world_mut(), "Confirm Combat Action").expect("confirm");
        let labels = app
            .world_mut()
            .query::<(Entity, &Name)>()
            .iter(app.world())
            .filter(|(_, name)| {
                name.as_str().ends_with("Landing Marker") && name.as_str().starts_with("Actor 10")
                    || name.as_str().ends_with("Summary") && name.as_str().starts_with("Actor ")
                    || name.as_str().starts_with("Initiative Actor ")
                        && name.as_str().ends_with(" Label")
                    || name.as_str() == "Command Hero Identity"
            })
            .map(|(entity, _)| entity)
            .collect::<Vec<_>>();
        for marker in labels {
            let bounds = app
                .world()
                .get::<ComputedNode>(marker)
                .expect("marker bounds")
                .size();
            let text = app
                .world()
                .get::<bevy::text::TextLayoutInfo>(marker)
                .expect("measured marker")
                .size;
            assert!(
                text.x <= bounds.x + 0.5 && text.y <= bounds.y + 0.5,
                "{} text must fit at the selected scale: {text:?} in {bounds:?}",
                app.world().get::<Name>(marker).expect("label name")
            );
        }
        if scale == UiScaleMode::Percent200 {
            let marker_bottom = app
                .world()
                .get::<UiGlobalTransform>(marker)
                .expect("marker position")
                .translation
                .y
                + app
                    .world()
                    .get::<ComputedNode>(marker)
                    .expect("marker bounds")
                    .size()
                    .y
                    / 2.0;
            for (entity, anchor) in app
                .world_mut()
                .query::<(Entity, &crate::scene::SceneActorAnchor)>()
                .iter(app.world())
            {
                if snapshot.actor(anchor.actor).expect("actor").team()
                    != labyrinth_rules::Team::Enemies
                {
                    continue;
                }
                let top = app
                    .world()
                    .get::<UiGlobalTransform>(entity)
                    .expect("actor position")
                    .translation
                    .y
                    - app
                        .world()
                        .get::<ComputedNode>(entity)
                        .expect("actor bounds")
                        .size()
                        .y
                        / 2.0;
                assert!(
                    marker_bottom <= top + 0.5,
                    "compact forecast must stay above live actors: {marker_bottom} > {top}"
                );
            }
        }
        if keyboard {
            assert!(focus_action(app.world_mut(), confirm));
            tap_key(&mut app, KeyCode::Enter);
        } else {
            assert!(click_action(&mut app, confirm));
        }
        let intents = app
            .world_mut()
            .resource_mut::<Messages<LabyrinthIntent>>()
            .drain()
            .collect::<Vec<_>>();
        let actions = intents
            .into_iter()
            .filter_map(|intent| match intent {
                LabyrinthIntent::Combat { actor, action, .. } => Some((actor, action)),
                _ => None,
            })
            .collect::<Vec<_>>();
        assert_eq!(
            actions,
            vec![(
                ActorId(1),
                CombatAction::Skill {
                    skill: SkillId::DrivingBlow,
                    target: ActorId(104)
                }
            )]
        );
        let (actor, action) = actions.first().expect("one confirmation");
        combat.apply(*actor, *action).expect("authoritative commit");
        assert_eq!(combat.snapshot().rank(ActorId(104)), Some(4));
        assert_eq!(combat.snapshot().ranks(ActorId(101)), Some(2..=3));
        app.world_mut().resource_mut::<LabyrinthView>().combat = Some(combat.snapshot());
        run_frames(&mut app, 3);
        let strip = find_named(app.world_mut(), "Enemies Movement Preview").expect("strip");
        assert_eq!(
            app.world().get::<Node>(strip).expect("root").display,
            Display::None
        );
        assert!(app
            .world()
            .get::<Text>(marker)
            .expect("revoked")
            .0
            .is_empty());
    }
}

#[test]
fn partial_movement_explains_footprint_and_revokes_concealed_preview() {
    let (mut app, _) = movement_app(1280, 720, UiScaleMode::Auto);
    select(&mut app, ActorId(103), false);
    let explanation = find_named(app.world_mut(), "Enemies Movement Explanation").expect("reason");
    let text = &app.world().get::<Text>(explanation).expect("text").0;
    assert!(text.contains("Push 1/2 ranks"));
    assert!(text.contains("Ossuary Hauler needs 2 ranks to pass"));
    let marker = find_named(app.world_mut(), "Actor 103 Landing Marker").expect("target");
    assert_eq!(
        app.world().get::<Text>(marker).expect("rank").0,
        "Ash\nBrute\n1 → 2"
    );
    for paused in [true, false] {
        app.world_mut().resource_mut::<LabyrinthView>().paused = paused;
        run_frames(&mut app, 3);
        assert_eq!(
            app.world()
                .get::<Text>(marker)
                .expect("marker")
                .0
                .is_empty(),
            paused
        );
    }
    let menu = find_named(app.world_mut(), "Battle Settings").expect("menu");
    assert!(click_action(&mut app, menu));
    run_frames(&mut app, 3);
    assert!(app
        .world()
        .get::<Text>(marker)
        .expect("menu revocation")
        .0
        .is_empty());
    tap_key(&mut app, KeyCode::Escape);
    run_frames(&mut app, 3);
    assert!(!app
        .world()
        .get::<Text>(marker)
        .expect("restored preview")
        .0
        .is_empty());
    let cancel = find_named(app.world_mut(), "Cancel Combat Selection").expect("cancel");
    assert!(click_action(&mut app, cancel));
    run_frames(&mut app, 3);
    assert!(app
        .world()
        .get::<Text>(marker)
        .expect("cancel revocation")
        .0
        .is_empty());
    select(&mut app, ActorId(103), false);
    app.world_mut()
        .resource_mut::<CombatDisclosure>()
        .actors
        .insert(
            ActorId(103),
            ActorDisclosure {
                health: false,
                ..default()
            },
        );
    run_frames(&mut app, 3);
    assert!(app
        .world()
        .get::<Text>(explanation)
        .expect("revoked")
        .0
        .is_empty());
    assert!(app
        .world()
        .get::<Text>(marker)
        .expect("revoked")
        .0
        .is_empty());
    assert!(app.world().get::<AccessibleLabel>(marker).is_none());
    let strip = find_named(app.world_mut(), "Enemies Movement Preview").expect("strip");
    assert_eq!(
        app.world().get::<Node>(strip).expect("root").display,
        Display::None
    );
}
