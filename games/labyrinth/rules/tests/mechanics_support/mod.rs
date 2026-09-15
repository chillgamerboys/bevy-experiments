use labyrinth_rules::{
    build::{ActorBuild, CharacterBuild, SkillGrant},
    catalog::{ContentCatalog, ContentId},
    scenario::{ControllerPolicy, Scenario, ScenarioActor, SCENARIO_SCHEMA_VERSION},
    ActorId, Combat, CombatAction,
};

pub fn id(key: &str) -> ContentId {
    ContentId::new(key).expect("authored content ID")
}

pub fn actor(catalog: &ContentCatalog, number: u16, preset: &str) -> ScenarioActor {
    let mut actor = ActorBuild::from_preset(catalog, &id(preset)).expect("known preset");
    actor.max_hp = 100;
    actor.base_speed = 0;
    actor.footprint = 1;
    actor.build = CharacterBuild::default();
    ScenarioActor {
        id: ActorId(number),
        actor,
        controller: ControllerPolicy::External,
        starting_hp: None,
        starting_statuses: vec![],
    }
}

pub fn scenario(
    weapon: Option<&str>,
    skills: &[&str],
    source_rank: usize,
    widths: &[u8],
) -> (ContentCatalog, Scenario) {
    let catalog = ContentCatalog::builtin().expect("builtin catalog");
    let mut source = actor(&catalog, 1, "scout");
    source.actor.base_speed = 100; // Acts before every speed-0 actor for every d8 result.
    source.actor.build.weapon = weapon.map(id);
    source.actor.build.skills = skills
        .iter()
        .map(|key| SkillGrant {
            skill: id(key),
            provenance: id("scenario"),
        })
        .collect();
    let mut heroes: Vec<_> = (2..=6)
        .map(|n| {
            let mut filler = actor(&catalog, n, "gatekeeper");
            if n == 2 {
                filler.actor.base_speed = 90;
            }
            filler
        })
        .collect();
    heroes.insert(source_rank - 1, source);
    let enemies = widths
        .iter()
        .zip(101..)
        .map(|(width, n)| {
            let mut enemy = actor(&catalog, n, "ash_brute");
            enemy.actor.footprint = *width;
            enemy
        })
        .collect();
    (
        catalog,
        Scenario {
            schema_version: SCENARIO_SCHEMA_VERSION,
            name: "Mechanics acceptance".into(),
            seed: 91,
            heroes,
            enemies,
        },
    )
}

pub fn combat(catalog: &ContentCatalog, scenario: &Scenario) -> Combat {
    let saved = scenario.to_json().expect("serialize explicit input");
    let loaded = Scenario::from_json(&saved, catalog).expect("validate saved input");
    assert_eq!(&loaded, scenario);
    let combat = Combat::from_scenario(catalog, &loaded).expect("authoritative constructor");
    assert_eq!(combat.snapshot().active_actor, Some(ActorId(1)));
    combat
}

pub fn action(combat: &Combat, key: &str, target: u16) -> CombatAction {
    let snapshot = combat.snapshot();
    let index = snapshot
        .actor(ActorId(1))
        .expect("source")
        .skill_index(&id(key))
        .expect("eligible Skill");
    CombatAction::Skill {
        index,
        target: ActorId(target),
    }
}

pub fn reject(game: &mut Combat, command: CombatAction) {
    let before = game.clone();
    assert!(game.validate_action(ActorId(1), &command).is_err());
    assert!(!game.legal_actions(ActorId(1)).contains(&command));
    assert!(game.apply(ActorId(1), command).is_err());
    assert_eq!(
        *game, before,
        "rejection preserves state, RNG and private counters"
    );
}

pub fn commit(game: &mut Combat, command: CombatAction) -> Vec<labyrinth_rules::CombatEvent> {
    assert!(game.legal_actions(ActorId(1)).contains(&command));
    let before = game.clone();
    game.snapshot()
        .preview_action(ActorId(1), &command)
        .expect("legal preview");
    assert_eq!(*game, before);
    let mut replay = before;
    let events = game.apply(ActorId(1), command).expect("legal command");
    assert_eq!(events, replay.apply(ActorId(1), command).expect("replay"));
    assert_eq!(*game, replay);
    let snapshot = game.snapshot();
    snapshot.validate().expect("valid result");
    let restored: labyrinth_rules::CombatSnapshot =
        serde_json::from_slice(&serde_json::to_vec(&snapshot).expect("snapshot bytes"))
            .expect("validated snapshot");
    assert_eq!(snapshot, restored);
    events
}

pub fn wait_for(game: &mut Combat, actor: ActorId) {
    for _ in 0..120 {
        let active = game.snapshot().active_actor.expect("ongoing encounter");
        if active == actor {
            return;
        }
        game.apply(active, CombatAction::Wait)
            .expect("external driver waits");
    }
    panic!("actor must receive a bounded future turn");
}
