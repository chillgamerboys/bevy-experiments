//! Native keyboard/layout evidence for authored moves; no desktop-render claim.
use super::*;
use bevy_gamekit::ui::{UiTooltipCatalog, UiTooltipSource};
use labyrinth_rules::{
    build::{ActorBuild, CharacterBuild, SkillGrant},
    catalog::{
        AbilityDefinition, ContentCatalog, ContentId, SkillUpgrade, TargetPattern, UpgradeOperation,
    },
    scenario::{ControllerPolicy, Scenario, ScenarioActor, SCENARIO_SCHEMA_VERSION},
    Effect,
};

fn id(value: &str) -> ContentId {
    ContentId::new(value).expect("authored ID")
}

fn authored_scenario() -> (ContentCatalog, Scenario) {
    let builtin = ContentCatalog::builtin().expect("catalog");
    let mut definition = builtin.definition().clone();
    let template = definition.skills.first().expect("skill").clone();
    let mut grants = Vec::new();
    for index in 0..14 {
        let mut skill = template.clone();
        skill.id = id(&format!("technique_{index}"));
        skill.personal_selectable = true;
        skill.requirements.clear();
        skill.name = format!("Technique {index}");
        skill.description = format!("Captain's practiced technique {index}.");
        skill.source_ranks = 63;
        skill.target_ranks = 3;
        skill.target_rule = labyrinth_rules::TargetRule::EnemyStanding;
        skill.target_pattern = TargetPattern::FrontPair;
        skill.effects = vec![Effect::Damage(2)];
        skill.max_uses = Some(3);
        grants.push(SkillGrant {
            skill: skill.id.clone(),
            provenance: id("captain_training"),
        });
        definition.skills.push(skill);
    }
    definition.abilities.push(AbilityDefinition {
        id: id("captain_mastery"),
        name: "Captain mastery".into(),
        description: "Stronger first technique".into(),
        provenance: id("training"),
        personal_selectable: true,
        requirements: vec![],
        effects: vec![],
        upgrades: vec![SkillUpgrade {
            skill: id("technique_0"),
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
        skills: grants,
        abilities: vec![id("captain_mastery")],
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
    let scenario = Scenario {
        schema_version: SCENARIO_SCHEMA_VERSION,
        name: "Fourteen techniques".into(),
        seed: 91,
        heroes: vec![ScenarioActor {
            id: ActorId(1),
            actor: hero,
            controller: ControllerPolicy::Manual,
            starting_hp: None,
            starting_statuses: vec![],
        }],
        enemies: vec![enemy(101), enemy(102)],
    };
    (catalog, scenario)
}

fn authored_combat() -> Combat {
    let (catalog, scenario) = authored_scenario();
    Combat::from_scenario(&catalog, &scenario).expect("authored combat")
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
fn fourteen_authored_abilities_are_tabbable_inspectable_targetable_and_confirmable_normal_1080() {
    fourteen_authored_abilities_are_tabbable_inspectable_targetable_and_confirmable(
        1920,
        1080,
        UiScaleMode::Auto,
    );
}

#[test]
fn fourteen_authored_abilities_are_tabbable_inspectable_targetable_and_confirmable_compatibility() {
    fourteen_authored_abilities_are_tabbable_inspectable_targetable_and_confirmable(
        1280,
        720,
        UiScaleMode::Auto,
    );
    fourteen_authored_abilities_are_tabbable_inspectable_targetable_and_confirmable(
        1280,
        720,
        UiScaleMode::Percent200,
    );
}

fn fourteen_authored_abilities_are_tabbable_inspectable_targetable_and_confirmable(
    width: u32,
    height: u32,
    scale: UiScaleMode,
) {
    let combat = authored_combat();
    let snapshot = combat.snapshot();
    assert_eq!(snapshot.active_actor, Some(ActorId(1)));
    // Sparse formations fit much taller real artwork than the full roster.
    // Help must retain useful body space after the art has loaded.
    let mut app = super::overlay_stability::scene_app(width, height, scale);
    app.init_resource::<Commands>()
        .add_systems(PostUpdate, capture);
    app.world_mut().resource_mut::<LabyrinthView>().combat = Some(snapshot.clone());
    run_frames(&mut app, 5);
    let controls = (0..14)
        .map(|index| {
            find_named(app.world_mut(), &format!("Skill {index}")).expect("every granted skill")
        })
        .collect::<Vec<_>>();
    assert!(find_named(app.world_mut(), "Skill 14").is_none());
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
            Rect::from_corners(Vec2::ZERO, Vec2::new(width as f32, height as f32)),
        )
        .expect("visible focused skill");
        assert!(
            rect.width() >= 43.5 && rect.height() >= 43.5,
            "{index} {scale:?}: {rect:?}"
        );
        tap_key(&mut app, KeyCode::Enter);
        assert_eq!(
            app.world().resource::<UiState>().selected,
            Some(Choice::Skill(index as u8))
        );
        let subject = app
            .world()
            .get::<UiTooltipSource>(control)
            .expect("inspectable")
            .0
            .clone();
        assert!(subject.0.contains("encounter/1/actor/1/skill/technique_"));
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
        let viewport = Rect::from_corners(Vec2::ZERO, Vec2::new(width as f32, height as f32));
        let card = find_named(app.world_mut(), "Tooltip Card 0").expect("help card");
        let card_rect = visible_control_rect(app.world(), card, viewport).expect("visible help");
        for name in [
            "Actor 1 Summary",
            "Actor 101 Summary",
            "Actor 1 HP Track",
            "Actor 101 HP Track",
            "Confirm Combat Action",
        ] {
            let control = find_named(app.world_mut(), name).expect("reserved control");
            let control_rect = visible_control_rect(app.world(), control, viewport)
                .expect("reserved control visible");
            assert!(card_rect.intersect(control_rect).is_empty());
        }
        let facts = app
            .world_mut()
            .query::<(Entity, &Name, &Text)>()
            .iter(app.world())
            .filter(|(_, name, text)| {
                name.as_str() == "Tooltip Fact"
                    && (text.0.contains("base damage")
                        || text.0.starts_with("From  ")
                        || text.0.starts_with("Target "))
            })
            .map(|(entity, _, _)| entity)
            .collect::<Vec<_>>();
        assert_eq!(facts.len(), 3);
        for fact in facts {
            let rect = visible_control_rect(app.world(), fact, viewport)
                .expect("effects and ranks visible in the first help fold");
            let node = app.world().get::<ComputedNode>(fact).expect("fact layout");
            assert!(
                rect.height() + 0.5 >= node.size().y * node.inverse_scale_factor,
                "help clips mechanics at {scale:?}: {rect:?}"
            );
        }
        tap_key(&mut app, KeyCode::End);
        run_frames(&mut app, 3);
        let description =
            find_named(app.world_mut(), "Tooltip Description").expect("authored explanation");
        let description_rect = visible_control_rect(app.world(), description, viewport)
            .expect("help paging reaches its explanation");
        assert!(description_rect.height() >= 20.0);
        let close = find_named(app.world_mut(), "Tooltip Close").expect("close inspection");
        assert!(focus_action(app.world_mut(), close));
        tap_key(&mut app, KeyCode::Enter);
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
        let action = CombatAction::Skill {
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
        // Restore the previous command focus so the next skill is reached by Tab.
        assert!(focus_action(app.world_mut(), control));
    }
    for name in [
        "Reposition",
        "Rescue",
        "Defend",
        "Wait",
        "Confirm Combat Action",
    ] {
        let control = find_named(app.world_mut(), name).expect("utility control");
        let visible = visible_control_rect(
            app.world(),
            control,
            Rect::from_corners(Vec2::ZERO, Vec2::new(width as f32, height as f32)),
        )
        .expect("visible utility");
        let node = app
            .world()
            .get::<ComputedNode>(control)
            .expect("measured utility");
        assert!(
            visible.width() + 0.5 >= node.size().x * node.inverse_scale_factor,
            "{name} clipped at {scale:?}"
        );
    }
    // The final scrolled move also uses the normal native pointer path.
    let last = *controls.last().expect("fourteenth move");
    let target = find_named(app.world_mut(), "Actor 101").expect("target");
    let confirm = find_named(app.world_mut(), "Confirm Combat Action").expect("confirm");
    for control in [last, target, confirm] {
        pointer_control(&mut app, control, Vec2::new(width as f32, height as f32));
    }
    assert_eq!(
        std::mem::take(&mut app.world_mut().resource_mut::<Commands>().0),
        vec![(
            ActorId(1),
            CombatAction::Skill {
                index: 13,
                target: ActorId(101)
            }
        )]
    );
}

#[test]
fn effective_multi_target_forecast_names_every_target_and_conceals_secondary_unknowns() {
    use crate::presentation::{ActorDisclosure, CombatDisclosure, ForecastDisplay, Knowledge};
    let snapshot = authored_combat().snapshot();
    let action = CombatAction::Skill {
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

#[test]
fn ability_cards_are_scoped_by_actor_and_encounter_and_history_uses_authored_names_normal_1080() {
    ability_cards_are_scoped_by_actor_and_encounter_and_history_uses_authored_names(
        1920,
        1080,
        UiScaleMode::Auto,
    );
}

#[test]
fn ability_cards_are_scoped_by_actor_and_encounter_and_history_uses_authored_names_compatibility() {
    ability_cards_are_scoped_by_actor_and_encounter_and_history_uses_authored_names(
        1280,
        720,
        UiScaleMode::Auto,
    );
}

fn ability_cards_are_scoped_by_actor_and_encounter_and_history_uses_authored_names(
    width: u32,
    height: u32,
    scale: UiScaleMode,
) {
    let (catalog, mut scenario) = authored_scenario();
    let mut apprentice = scenario.heroes.first().expect("captain").clone();
    apprentice.id = ActorId(2);
    apprentice.actor.name = "Apprentice Custom".into();
    apprentice.actor.build.abilities.clear();
    scenario.heroes.push(apprentice);
    let snapshot = Combat::from_scenario(&catalog, &scenario)
        .expect("two custom actors")
        .snapshot();
    let mut app = app(width, height, scale);
    {
        let mut view = app.world_mut().resource_mut::<LabyrinthView>();
        network_ownership(&mut view);
        view.combat = Some(snapshot);
        view.player = Some(0);
        view.events = vec![crate::view::PresentedEvent {
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
        }];
    }
    run_frames(&mut app, 5);
    let captain = find_named(app.world_mut(), "Skill 0").expect("captain move");
    let captain_key = app
        .world()
        .get::<UiTooltipSource>(captain)
        .expect("captain card")
        .0
        .clone();
    assert!(focus_action(app.world_mut(), captain));
    tap_key(&mut app, KeyCode::Enter);
    app.world_mut().resource_mut::<LabyrinthView>().player = Some(1);
    run_frames(&mut app, 4);
    assert!(app.world().get_entity(captain).is_err());
    assert_eq!(app.world().resource::<UiState>().selected, None);
    let apprentice = find_named(app.world_mut(), "Skill 0").expect("apprentice move");
    let apprentice_key = app
        .world()
        .get::<UiTooltipSource>(apprentice)
        .expect("apprentice card")
        .0
        .clone();
    assert_ne!(captain_key, apprentice_key);
    for (key, power) in [
        (&captain_key, "5 base damage"),
        (&apprentice_key, "2 base damage"),
    ] {
        assert!(app
            .world()
            .resource::<UiTooltipCatalog>()
            .0
            .get(key)
            .expect("actor effective card")
            .facts
            .iter()
            .any(|fact| fact == power));
    }
    let history = find_named(app.world_mut(), "Battle Log Toggle").expect("history");
    assert!(click_action(&mut app, history));
    run_frames(&mut app, 4);
    assert!(app
        .world_mut()
        .query::<&Text>()
        .iter(app.world())
        .any(|text| text.0.contains("Captain Custom · Technique 0")));
    app.world_mut().resource_mut::<LabyrinthView>().encounter += 1;
    run_frames(&mut app, 4);
    assert!(!app
        .world()
        .resource::<UiTooltipCatalog>()
        .0
        .contains_key(&captain_key));
    assert!(!app
        .world()
        .resource::<UiTooltipCatalog>()
        .0
        .contains_key(&apprentice_key));
    let current = find_named(app.world_mut(), "Skill 0").expect("new encounter move");
    assert!(app
        .world()
        .get::<UiTooltipSource>(current)
        .expect("scoped card")
        .0
         .0
        .contains("encounter/2/actor/2/"));
}

#[test]
fn queued_hotbar_confirmation_cannot_retarget_a_replaced_build_before_present_normal_1080() {
    queued_hotbar_confirmation_cannot_retarget_a_replaced_build_before_present(
        1920,
        1080,
        UiScaleMode::Auto,
    );
}

#[test]
fn queued_hotbar_confirmation_cannot_retarget_a_replaced_build_before_present_compatibility() {
    queued_hotbar_confirmation_cannot_retarget_a_replaced_build_before_present(
        1280,
        720,
        UiScaleMode::Auto,
    );
}

fn queued_hotbar_confirmation_cannot_retarget_a_replaced_build_before_present(
    width: u32,
    height: u32,
    scale: UiScaleMode,
) {
    use bevy::ecs::system::RunSystemOnce;

    // Exercise the production translation boundary with queued native-entity
    // messages. This is not a real-pointer or render-order reproduction.
    for replace_build in [false, true] {
        let (catalog, mut scenario) = authored_scenario();
        let original = Combat::from_scenario(&catalog, &scenario)
            .expect("original scenario")
            .snapshot();
        let mut app = app(width, height, scale);
        app.world_mut().resource_mut::<LabyrinthView>().combat = Some(original.clone());
        run_frames(&mut app, 5);
        let skill = find_named(app.world_mut(), "Skill 0").expect("original technique zero");
        let target = find_named(app.world_mut(), "Actor 101").expect("target");
        for control in [skill, target] {
            assert!(focus_action(app.world_mut(), control));
            tap_key(&mut app, KeyCode::Enter);
        }
        let confirm = find_named(app.world_mut(), "Confirm Combat Action").expect("confirm");
        assert!(activation_eligible(app.world_mut(), confirm));
        assert_eq!(
            app.world().resource::<UiState>().selected,
            Some(Choice::Skill(0))
        );
        assert!(app
            .world_mut()
            .resource_mut::<Messages<LabyrinthIntent>>()
            .drain()
            .next()
            .is_none());

        if replace_build {
            scenario
                .heroes
                .first_mut()
                .expect("first hero")
                .actor
                .build
                .skills
                .swap(0, 1);
            let replacement = Combat::from_scenario(&catalog, &scenario)
                .expect("valid reordered build")
                .snapshot();
            assert_eq!(replacement.active_actor, original.active_actor);
            assert_eq!(replacement.turn_id, original.turn_id);
            assert_ne!(
                replacement
                    .actor(ActorId(1))
                    .expect("new actor")
                    .skill(0)
                    .expect("new zero")
                    .id,
                original
                    .actor(ActorId(1))
                    .expect("old actor")
                    .skill(0)
                    .expect("old zero")
                    .id
            );
            app.world_mut().resource_mut::<LabyrinthView>().combat = Some(replacement);
        }
        // Both controls still belong to the displayed old build. Deliberately
        // translate before Present can unmount them or clear positional selection.
        app.world_mut().write_message(UiActivated { entity: skill });
        app.world_mut()
            .write_message(UiActivated { entity: confirm });
        app.world_mut()
            .run_system_once(collect_actions)
            .expect("translate queued activations");
        let commands = app
            .world_mut()
            .resource_mut::<Messages<LabyrinthIntent>>()
            .drain()
            .filter_map(|intent| match intent {
                LabyrinthIntent::Combat { actor, action, .. } => Some((actor, action)),
                _ => None,
            })
            .collect::<Vec<_>>();
        if replace_build {
            assert!(
                commands.is_empty(),
                "an old button must not confirm the replacement slot's skill: {commands:?}"
            );
        } else {
            assert_eq!(
                commands,
                vec![(
                    ActorId(1),
                    CombatAction::Skill {
                        index: 0,
                        target: ActorId(101)
                    }
                )]
            );
        }
    }
}
