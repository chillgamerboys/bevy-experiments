use super::*;
use crate::{EnemyKind, LifeState, SkillId, CORPSE_ROUNDS};

fn fixture() -> Combat {
    Combat::with_party(
        19,
        [
            HeroClass::Gatekeeper,
            HeroClass::Knifehand,
            HeroClass::Scout,
            HeroClass::FieldMedic,
            HeroClass::LanternWagon,
        ]
        .into_iter()
        .enumerate()
        .map(|(i, class)| HeroSetup::preset(ActorId(i as u16 + 1), class))
        .collect(),
    )
    .expect("valid fixture")
}

fn hit(combat: &mut Combat, id: ActorId, amount: u16) -> Vec<CombatEvent> {
    let mut events = Vec::new();
    combat
        .damage(
            ActorId(1),
            id,
            amount,
            DamageKind::Direct,
            &mut events,
            &mut Work(MAX_WORK),
        )
        .expect("valid fixture");
    events
}

#[test]
fn footprints_are_unique_and_reach_intersects_any_occupied_rank() {
    let mut combat = fixture();
    let s = combat.snapshot();
    assert_eq!(s.ranks(ActorId(5)), Some(5..=6));
    assert_eq!(s.ranks(ActorId(101)), Some(3..=4));
    assert_eq!(s.occupant(Team::Heroes, 6), Some(ActorId(5)));
    assert_eq!(s.occupant(Team::Enemies, 4), Some(ActorId(101)));
    assert_eq!(s.actors.len(), 10);
    assert_eq!(
        s.initiative
            .iter()
            .filter(|e| e.actor == ActorId(5))
            .count(),
        1
    );
    assert!(s.adjacent(ActorId(4), ActorId(5)));
    combat.state.enemy_formation.swap(2, 3); // Hauler now spans ranks 4–5.
    assert_eq!(combat.state.ranks(ActorId(101)), Some(4..=5));
    assert!(combat
        .state
        .validate_action_target(
            ActorId(3),
            &CombatAction::Skill {
                skill: SkillId::BackRankShot,
                target: ActorId(101)
            }
        )
        .is_ok());
    let roundtrip: CombatSnapshot =
        serde_json::from_str(&serde_json::to_string(&combat.snapshot()).expect("valid fixture"))
            .expect("valid fixture");
    assert_eq!(roundtrip, combat.snapshot());
}

#[test]
fn default_enemy_formation_gives_every_kind_an_in_range_attack() {
    let snapshot = fixture().snapshot();
    assert_eq!(snapshot.rank(ActorId(103)), Some(1));
    assert_eq!(snapshot.rank(ActorId(104)), Some(2));
    assert_eq!(snapshot.ranks(ActorId(101)), Some(3..=4));
    assert_eq!(snapshot.rank(ActorId(105)), Some(5));
    assert_eq!(snapshot.rank(ActorId(106)), Some(6));
    for enemy in snapshot
        .actors
        .iter()
        .filter(|actor| actor.team() == Team::Enemies)
    {
        assert!(
            enemy
                .skills()
                .iter()
                .any(|skill| snapshot.hero_formation.iter().any(|target| snapshot
                    .validate_action_target(
                        enemy.id,
                        &CombatAction::Skill {
                            skill: *skill,
                            target: *target,
                        }
                    )
                    .is_ok())),
            "{} cannot attack from its authored position",
            enemy.name()
        );
    }
}

#[test]
fn setup_rejects_overcapacity_and_duplicate_occupants_on_either_team() {
    assert!(Combat::new(1, [HeroClass::LanternWagon; PARTY_SIZE]).is_err());
    let heroes = vec![HeroSetup::preset(ActorId(1), HeroClass::Gatekeeper)];
    assert!(Combat::with_rosters(
        1,
        heroes.clone(),
        vec![(ActorId(101), EnemyKind::OssuaryHauler); 2]
    )
    .is_err());
    assert!(Combat::with_rosters(
        1,
        heroes,
        (101..105)
            .map(|id| (ActorId(id), EnemyKind::OssuaryHauler))
            .collect()
    )
    .is_err());
    let mut wire = serde_json::to_value(fixture().snapshot()).expect("valid fixture");
    *wire
        .pointer_mut("/hero_formation/1")
        .expect("formation entry") = serde_json::json!(1);
    assert!(serde_json::from_value::<CombatSnapshot>(wire).is_err());
}

#[test]
fn corpses_keep_spaces_status_identity_and_separate_hp_until_destroyed() {
    let mut c = fixture();
    c.add_status(
        ActorId(2),
        ActorId(101),
        StatusKind::Bleed,
        &mut Vec::new(),
        &mut Work(MAX_WORK),
    )
    .expect("valid fixture");
    c.add_status(
        ActorId(101),
        ActorId(101),
        StatusKind::Brace,
        &mut Vec::new(),
        &mut Work(MAX_WORK),
    )
    .expect("valid fixture");
    let bleed = c
        .state
        .actor(ActorId(101))
        .expect("valid fixture")
        .statuses
        .first()
        .expect("bleed")
        .clone();
    hit(&mut c, ActorId(101), 100);
    let a = c.state.actor(ActorId(101)).expect("valid fixture");
    assert_eq!(a.hp, 0);
    assert_eq!(a.health(), (11, 11));
    assert_eq!(a.statuses, vec![bleed.clone()]);
    assert_eq!(c.state.rank(ActorId(105)), Some(5));
    c.boundary(
        Boundary::RoundEnd,
        None,
        &mut Vec::new(),
        &mut Work(MAX_WORK),
    )
    .expect("valid fixture");
    let a = c.state.actor(ActorId(101)).expect("valid fixture");
    assert_eq!(a.health().0, 9);
    assert_eq!(a.statuses.first().expect("bleed").id, bleed.id);
    assert_eq!(
        a.statuses.first().expect("bleed").remaining,
        bleed.remaining - 1
    );
    hit(&mut c, ActorId(101), 9);
    assert_eq!(c.state.rank(ActorId(105)), Some(3));
    assert_eq!(
        c.state.actor(ActorId(101)).expect("valid fixture").life,
        LifeState::Removed
    );
    assert!(c
        .state
        .actor(ActorId(101))
        .expect("valid fixture")
        .statuses
        .is_empty());
}

#[test]
fn expiry_excludes_creation_round_and_is_identical_for_both_teams() {
    let mut c = fixture();
    hit(&mut c, ActorId(101), 100);
    hit(&mut c, ActorId(5), 100);
    for _ in 0..3 {
        c.death_save(ActorId(5), 1, &mut Vec::new(), &mut Work(MAX_WORK))
            .expect("valid fixture");
    }
    for elapsed in 0..=CORPSE_ROUNDS {
        c.state.round = 1 + elapsed;
        c.expire_corpses(&mut Vec::new(), &mut Work(MAX_WORK))
            .expect("valid fixture");
        for id in [ActorId(101), ActorId(5)] {
            assert_eq!(c.state.rank(id).is_some(), elapsed < CORPSE_ROUNDS);
        }
    }
}

#[test]
fn saves_damage_rescue_and_permanent_death_are_distinct() {
    let mut c = fixture();
    hit(&mut c, ActorId(5), 100);
    c.death_save(ActorId(5), 20, &mut Vec::new(), &mut Work(MAX_WORK))
        .expect("valid fixture");
    assert_eq!(
        c.state.actor(ActorId(5)).expect("valid fixture").life,
        LifeState::Dying { failures: 0 }
    );
    hit(&mut c, ActorId(5), 1);
    assert_eq!(
        c.state.actor(ActorId(5)).expect("valid fixture").life,
        LifeState::Dying { failures: 1 }
    );
    c.effect(
        ActorId(4),
        ActorId(5),
        Effect::Rescue(25),
        0,
        &mut Vec::new(),
        &mut Work(MAX_WORK),
    )
    .expect("valid fixture");
    assert_eq!(
        c.state.actor(ActorId(5)).expect("valid fixture").life,
        LifeState::Alive
    );
    hit(&mut c, ActorId(5), 100);
    for _ in 0..3 {
        hit(&mut c, ActorId(5), 1);
    }
    assert!(c
        .state
        .actor(ActorId(5))
        .expect("valid fixture")
        .is_corpse());
    assert_eq!(
        c.state
            .validate_action_target(ActorId(4), &CombatAction::Rescue { ally: ActorId(5) }),
        Err(RuleError::NotDowned)
    );
    assert!(c
        .state
        .validate_action_target(
            ActorId(3),
            &CombatAction::Skill {
                skill: SkillId::SnapShot,
                target: ActorId(5)
            }
        )
        .is_ok());
}

#[test]
fn whole_footprint_displacement_never_exceeds_rank_budget() {
    let mut c = fixture();
    c.move_actor(ActorId(105), -1, &mut Vec::new(), &mut Work(MAX_WORK))
        .expect("valid fixture");
    assert_eq!(c.state.rank(ActorId(105)), Some(5)); // Cannot cross half a Hauler.
    c.move_actor(ActorId(105), -2, &mut Vec::new(), &mut Work(MAX_WORK))
        .expect("valid fixture");
    assert_eq!(c.state.ranks(ActorId(101)), Some(4..=5));
    c.resolver()
        .swap(ActorId(5), ActorId(4), &mut Vec::new(), &mut Work(MAX_WORK))
        .expect("valid fixture");
    assert_eq!(c.state.ranks(ActorId(5)), Some(4..=5));
    assert_eq!(c.state.rank(ActorId(4)), Some(6));
    c.state.validate().expect("valid fixture");
}

#[test]
fn corpse_remains_do_not_prevent_victory_and_bad_life_wire_is_rejected() {
    let mut c = fixture();
    for id in c.state.enemy_formation.clone() {
        hit(&mut c, id, 100);
    }
    assert_eq!(c.state.outcome, Some(CombatOutcome::Victory));
    assert_eq!(c.state.enemy_formation.len(), 5);
    c.state.validate().expect("valid fixture");
    let mut wire = serde_json::to_value(c.snapshot()).expect("valid fixture");
    *wire.pointer_mut("/actors/0/life").expect("life state") =
        serde_json::json!({"Dying":{"failures":3}});
    assert!(serde_json::from_value::<CombatSnapshot>(wire).is_err());
}

#[test]
fn real_round_advancement_replays_saves_death_and_expiry_without_bonus_actions() {
    for seed in 0..16 {
        let mut c = fixture();
        c.rng = Rng(seed);
        hit(&mut c, ActorId(5), 100);
        let mut replay = c.clone();
        let mut saves = 0;
        let mut created = None;
        for _ in 0..500 {
            let actor = c.state.active_actor.expect("other combatants remain alive");
            assert_ne!(
                actor,
                ActorId(5),
                "dying/corpse actors never receive decisions"
            );
            let events = c
                .apply(actor, CombatAction::Wait)
                .expect("valid boundary progression");
            assert_eq!(
                events,
                replay
                    .apply(actor, CombatAction::Wait)
                    .expect("deterministic replay")
            );
            assert_eq!(c, replay);
            saves += events
                .iter()
                .filter(|e| {
                    matches!(
                        e.kind,
                        CombatEventKind::DeathSave {
                            actor: ActorId(5),
                            roll: Some(_),
                            ..
                        }
                    )
                })
                .count();
            let snapshot = c.snapshot();
            let decoded: CombatSnapshot =
                serde_json::from_str(&serde_json::to_string(&snapshot).expect("wire"))
                    .expect("validated lifecycle wire");
            assert_eq!(snapshot, decoded);
            match snapshot.actor(ActorId(5)).expect("retained identity").life {
                LifeState::Corpse { created_round, .. } => {
                    created = Some(created_round);
                }
                LifeState::Removed => {
                    assert_eq!(
                        snapshot.round,
                        created.expect("observed corpse") + CORPSE_ROUNDS + 1
                    );
                    break;
                }
                _ => {}
            }
        }
        assert!(saves >= 3);
        assert_eq!(
            c.state.actor(ActorId(5)).expect("identity").life,
            LifeState::Removed
        );
    }
}

#[test]
fn corpse_damage_ticks_before_expiry_and_clears_exactly_once() {
    let mut c = fixture();
    c.add_status(
        ActorId(2),
        ActorId(101),
        StatusKind::Bleed,
        &mut Vec::new(),
        &mut Work(MAX_WORK),
    )
    .expect("bleed");
    hit(&mut c, ActorId(101), 100);
    c.actor_mut(ActorId(101)).expect("corpse").life = LifeState::Corpse {
        hp: 2,
        max_hp: 11,
        created_round: 1,
    };
    c.state.round = 4;
    let mut events = Vec::new();
    c.boundary(Boundary::RoundEnd, None, &mut events, &mut Work(MAX_WORK))
        .expect("corpse tick");
    c.expire_corpses(&mut events, &mut Work(MAX_WORK))
        .expect("expiry");
    let removals: Vec<_> = events
        .iter()
        .filter(|e| matches!(e.kind, CombatEventKind::CorpseRemoved { .. }))
        .collect();
    assert_eq!(removals.len(), 1);
    assert!(matches!(
        removals.first().expect("removal").kind,
        CombatEventKind::CorpseRemoved { expired: false, .. }
    ));
    assert_eq!(c.state.rank(ActorId(103)), Some(1));
}
