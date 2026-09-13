//! Native keyboard/layout evidence for authored moves; no desktop-render claim.
use super::*;
use bevy_gamekit::ui::{UiTooltipCatalog, UiTooltipSource};
use labyrinth_rules::{
    build::{ActorBuild, CharacterBuild, InnateGrant},
    catalog::{
        AbilityUpgrade, ContentCatalog, ContentId, LearnedSkillDefinition, TargetPattern,
        UpgradeOperation,
    },
    scenario::{ControllerPolicy, Scenario, ScenarioActor, SCENARIO_SCHEMA_VERSION},
    Effect,
};

fn id(value: &str) -> ContentId {
    ContentId::new(value).expect("authored ID")
}

fn authored_combat() -> Combat {
    let builtin = ContentCatalog::builtin().expect("catalog");
    let mut definition = builtin.definition().clone();
    let template = definition.abilities.first().expect("ability").clone();
    let mut grants = Vec::new();
    for index in 0..12 {
        let mut ability = template.clone();
        ability.id = id(&format!("technique_{index}"));
        ability.name = format!("Technique {index}");
        ability.description = format!("Captain's practiced technique {index}.");
        ability.source_ranks = 63;
        ability.target_ranks = 3;
        ability.target_rule = labyrinth_rules::TargetRule::EnemyStanding;
        ability.target_pattern = TargetPattern::FrontPair;
        ability.effects = vec![Effect::Damage(2)];
        ability.max_uses = Some(3);
        grants.push(InnateGrant {
            ability: ability.id.clone(),
            provenance: id("captain_training"),
        });
        definition.abilities.push(ability);
    }
    definition.learned_skills.push(LearnedSkillDefinition {
        id: id("captain_mastery"),
        name: "Captain mastery".into(),
        description: "Stronger first technique".into(),
        provenance: id("training"),
        grants: vec![],
        upgrades: vec![AbilityUpgrade {
            ability: id("technique_0"),
            operations: vec![UpgradeOperation::AddDamage {
                effect_index: 0,
                amount: 3,
            }],
        }],
    });
    let catalog = ContentCatalog::new(definition).expect("authored catalog");
    let mut hero = ActorBuild::from_preset(&catalog, &id("gatekeeper")).expect("hero");
    hero.name = "Captain Custom".into();
    hero.base_speed = 100;
    hero.max_hp = 100;
    hero.build = CharacterBuild {
        innate: grants,
        learned_skills: vec![id("captain_mastery")],
        weapon: None,
    };
    let enemy = |n| {
        let mut actor = ActorBuild::from_preset(&catalog, &id("ash_brute")).expect("enemy");
        actor.max_hp = 100;
        actor.footprint = 1;
        actor.name = format!("Guard {n}");
        ScenarioActor {
            id: ActorId(n),
            actor,
            controller: ControllerPolicy::Ai,
            starting_hp: None,
            starting_statuses: vec![],
        }
    };
    Combat::from_scenario(
        &catalog,
        &Scenario {
            schema_version: SCENARIO_SCHEMA_VERSION,
            name: "Twelve techniques".into(),
            seed: 91,
            heroes: vec![ScenarioActor {
                id: ActorId(1),
                actor: hero,
                controller: ControllerPolicy::Manual,
                starting_hp: None,
                starting_statuses: vec![],
            }],
            enemies: vec![enemy(101), enemy(102)],
        },
    )
    .expect("authored combat")
}

#[derive(Resource, Default)]
struct Commands(Vec<(ActorId, CombatAction)>);
fn capture(mut intents: MessageReader<LabyrinthIntent>, mut commands: ResMut<Commands>) {
    for intent in intents.read() {
        if let LabyrinthIntent::Combat { actor, action, .. } = intent {
            commands.0.push((*actor, *action));
        }
    }
}

#[test]
fn twelve_authored_abilities_are_tabbable_inspectable_targetable_and_confirmable() {
    for scale in [UiScaleMode::Auto, UiScaleMode::Percent200] {
        let combat = authored_combat();
        let snapshot = combat.snapshot();
        assert_eq!(snapshot.active_actor, Some(ActorId(1)));
        let mut app = app(1280, 720, scale);
        app.init_resource::<Commands>()
            .add_systems(PostUpdate, capture);
        app.world_mut().resource_mut::<LabyrinthView>().combat = Some(snapshot.clone());
        run_frames(&mut app, 5);
        let controls = (0..12)
            .map(|index| {
                find_named(app.world_mut(), &format!("Skill {index}"))
                    .expect("every granted ability")
            })
            .collect::<Vec<_>>();
        assert!(find_named(app.world_mut(), "Skill 12").is_none());
        let first = *controls.first().expect("first");
        assert!(focus_action(app.world_mut(), first));
        for (index, &control) in controls.iter().enumerate() {
            if index > 0 {
                tap_key(&mut app, KeyCode::Tab);
            }
            run_frames(&mut app, 4);
            assert_eq!(
                app.world().resource::<InputFocus>().get(),
                Some(control),
                "Tab reaches {index} at {scale:?}"
            );
            let rect = visible_control_rect(
                app.world(),
                control,
                Rect::from_corners(Vec2::ZERO, Vec2::new(1280.0, 720.0)),
            )
            .expect("visible focused ability");
            assert!(
                rect.width() >= 43.5 && rect.height() >= 43.5,
                "{index} {scale:?}: {rect:?}"
            );
            tap_key(&mut app, KeyCode::Enter);
            assert_eq!(
                app.world().resource::<UiState>().selected,
                Some(Choice::Ability(index as u8))
            );
            let subject = app
                .world()
                .get::<UiTooltipSource>(control)
                .expect("inspectable")
                .0
                .clone();
            assert!(subject.0.contains("encounter/1/actor/1/ability/technique_"));
            let content = app
                .world()
                .resource::<UiTooltipCatalog>()
                .0
                .get(&subject)
                .expect("effective card");
            assert_eq!(content.title, format!("Technique {index}"));
            assert!(content
                .facts
                .iter()
                .any(|fact| fact.contains("captain_training")));
            assert!(content
                .facts
                .iter()
                .any(|fact| fact.contains("base damage")));
            if index == 0 {
                assert!(content
                    .facts
                    .iter()
                    .any(|fact| fact.contains("5 base damage")));
                assert!(content
                    .facts
                    .iter()
                    .any(|fact| fact.contains("captain_mastery")));
            }
            tap_key(&mut app, KeyCode::KeyT);
            run_frames(&mut app, 5);
            let title =
                find_named(app.world_mut(), "Tooltip Title").expect("native inspect opens card");
            assert_eq!(
                app.world().get::<Text>(title).expect("title").0,
                format!("Technique {index}")
            );
            tap_key(&mut app, KeyCode::Escape);
            let target = find_named(app.world_mut(), "Actor 101").expect("target");
            assert!(focus_action(app.world_mut(), target));
            tap_key(&mut app, KeyCode::Enter);
            assert!(
                app.world().resource::<Commands>().0.is_empty(),
                "selection never commits"
            );
            let confirm = find_named(app.world_mut(), "Confirm Combat Action").expect("confirm");
            assert!(focus_action(app.world_mut(), confirm));
            tap_key(&mut app, KeyCode::Enter);
            let commands = std::mem::take(&mut app.world_mut().resource_mut::<Commands>().0);
            let action = CombatAction::Ability {
                index: index as u8,
                target: ActorId(101),
            };
            assert_eq!(commands, vec![(ActorId(1), action)]);
            let mut resolved = combat.clone();
            resolved
                .apply(ActorId(1), action)
                .expect("real authoritative commit");
            let damage = if index == 0 { 5 } else { 2 };
            for target in [ActorId(101), ActorId(102)] {
                assert_eq!(
                    resolved.snapshot().actor(target).expect("target").hp,
                    100 - damage
                );
            }
            assert_eq!(
                app.world().resource::<LabyrinthView>().combat.as_ref(),
                Some(&snapshot)
            );
            // Restore the previous command focus so the next ability is reached by Tab.
            assert!(focus_action(app.world_mut(), control));
        }
    }
}

#[test]
fn effective_multi_target_forecast_names_every_target_and_conceals_secondary_unknowns() {
    use crate::presentation::{ActorDisclosure, CombatDisclosure, ForecastDisplay, Knowledge};
    let snapshot = authored_combat().snapshot();
    let action = CombatAction::Ability {
        index: 0,
        target: ActorId(101),
    };
    let known =
        ForecastDisplay::build(&snapshot, &CombatDisclosure::default(), ActorId(1), &action)
            .expect("forecast");
    assert_eq!(known.base, "5 base damage");
    for target in [ActorId(101), ActorId(102)] {
        assert!(known.actors.iter().any(|outcome| outcome.actor == target));
        assert!(known.summary.contains(&format!("Guard {}", target.0)));
    }
    assert_eq!(
        crate::presentation::actor_name(&snapshot, snapshot.actor(ActorId(1)).expect("hero")),
        "Captain Custom"
    );
    let mut disclosure = CombatDisclosure::default();
    disclosure.actors.insert(
        ActorId(102),
        ActorDisclosure {
            health: false,
            ..default()
        },
    );
    let hidden = ForecastDisplay::build(&snapshot, &disclosure, ActorId(1), &action)
        .expect("unknown forecast");
    assert!(hidden.uncertainty);
    assert_eq!(hidden.actors.len(), 2);
    assert!(hidden
        .actors
        .iter()
        .all(|outcome| outcome.health == Knowledge::Unknown));
    let mut changed = snapshot.clone();
    changed
        .actors
        .iter_mut()
        .find(|actor| actor.id == ActorId(102))
        .expect("hidden target")
        .hp = 1;
    assert_eq!(
        hidden,
        ForecastDisplay::build(&changed, &disclosure, ActorId(1), &action)
            .expect("same unknown forecast")
    );
}
