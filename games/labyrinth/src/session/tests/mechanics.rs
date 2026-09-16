//! Command and projection boundary; no window or network process is required.
use super::*;

#[test]
fn hook_shot_request_projects_the_accepted_whole_occupant_pull() {
    let mut authority = PartyAuthority::new(91, true);
    let mut scenario = authority.scenario.clone();
    for actor in scenario.heroes.iter_mut().chain(&mut scenario.enemies) {
        actor.actor.max_hp = 100;
        actor.actor.base_speed = if actor.id == ActorId(3) {
            100
        } else if actor.id == ActorId(1) {
            90
        } else {
            0
        };
        actor.controller = ControllerPolicy::External;
    }
    let revision = authority.setup_revision;
    assert_eq!(
        request(
            &mut authority,
            0,
            SessionCommand::ConfigureBattle {
                scenario,
                expected_revision: revision
            }
        )
        .rejection,
        None
    );
    assert_eq!(
        request(&mut authority, 0, SessionCommand::Ready(true)).rejection,
        None
    );
    assert_eq!(
        request(&mut authority, 0, SessionCommand::Start).rejection,
        None
    );
    let before = authority.snapshot(0);
    let combat = before.combat.as_ref().expect("combat");
    assert_eq!(combat.active_actor, Some(ActorId(3)));
    let index = combat
        .actor(ActorId(3))
        .expect("Scout")
        .skill_index(&labyrinth_rules::catalog::ContentId::new("hook_shot").expect("ID"))
        .expect("equipped Skill");
    let command = CombatAction::Skill {
        index,
        target: ActorId(105),
    };
    let forecast = crate::presentation::ForecastDisplay::build(
        combat,
        &crate::presentation::CombatDisclosure::default(),
        ActorId(3),
        &command,
    )
    .expect("forecast");
    assert!(forecast
        .movement
        .as_ref()
        .expect("movement")
        .contains("Pull forward 2 ranks"));
    assert_eq!(
        request(
            &mut authority,
            0,
            SessionCommand::Act {
                actor: ActorId(3),
                action: command
            }
        )
        .rejection,
        None
    );
    let after = authority.snapshot(0);
    after.validate().expect("session projection");
    let combat = after.combat.as_ref().expect("combat");
    assert_eq!(combat.actor(ActorId(105)).expect("Stalker").hp, 97);
    assert_eq!(combat.ranks(ActorId(105)), Some(3..=3));
    assert_eq!(combat.ranks(ActorId(101)), Some(4..=5));
    let restored: SessionSnapshot =
        serde_json::from_slice(&serde_json::to_vec(&after).expect("wire bytes"))
            .expect("validated projection");
    assert_eq!(restored, after);
}
