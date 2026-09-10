//! Six-rank and loadout contracts, independent of lobby, networking, or Bevy.

use super::*;
use crate::{EnemyKind, SkillId, DEFAULT_HERO_ROSTER, MAX_EQUIPPED_ABILITIES};

fn setup() -> [HeroSetup; PARTY_SIZE] {
    let mut ids = [90, 12, 42, 7, 900, 300].into_iter();
    DEFAULT_HERO_ROSTER.map(|class| HeroSetup::preset(ActorId(ids.next().expect("six IDs")), class))
}

fn wait_for(combat: &mut Combat, actor: ActorId) {
    for _ in 0..(MAX_ACTORS * 2) {
        if combat.state.active_actor == Some(actor) {
            return;
        }
        let current = combat.state.active_actor.expect("not terminal");
        combat.apply(current, CombatAction::Wait).expect("wait");
    }
    assert_eq!(combat.state.active_actor, Some(actor));
}

#[test]
fn explicit_ids_repeated_classes_and_initial_ranks_are_independent() {
    let heroes = setup();
    let ids = heroes.clone().map(|hero| hero.id);
    let combat = Combat::with_heroes(15, heroes).expect("explicit setup");
    let snapshot = combat.snapshot();
    assert_eq!(snapshot.hero_formation, ids);
    assert_eq!(snapshot.actors.len(), MAX_ACTORS);
    assert_eq!(snapshot.initiative.len(), MAX_ACTORS);
    assert_eq!(snapshot.enemy_formation, DEFAULT_ENEMY_IDS);
    assert_eq!(
        snapshot
            .actor(ActorId(900))
            .expect("arbitrary hero ID")
            .team(),
        Team::Heroes
    );
    assert_eq!(snapshot.rank(ActorId(900)), Some(5));
    assert_eq!(
        snapshot.actor(ActorId(12)).expect("first knife").kind,
        snapshot.actor(ActorId(42)).expect("second knife").kind
    );
    assert_eq!(
        snapshot.actor(ActorId(103)).expect("stalker").kind,
        snapshot.actor(ActorId(104)).expect("second stalker").kind
    );
    let json = serde_json::to_vec(&snapshot).expect("serialize explicit IDs/loadouts");
    assert_eq!(
        serde_json::from_slice::<CombatSnapshot>(&json).expect("validated snapshot"),
        snapshot
    );
}

#[test]
fn invalid_and_colliding_setup_ids_fail_before_combat_starts() {
    for bad_id in [ActorId(0), ActorId(101), ActorId(106)] {
        let mut heroes = setup();
        heroes.first_mut().expect("first").id = bad_id;
        assert_eq!(
            Combat::with_heroes(15, heroes),
            Err(RuleError::InvalidActorId)
        );
    }
    let mut heroes = setup();
    heroes.first_mut().expect("first").id = ActorId(12);
    assert_eq!(
        Combat::with_heroes(15, heroes),
        Err(RuleError::DuplicateActor)
    );
    let mut heroes = setup();
    heroes.first_mut().expect("first").id = ActorId(u16::MAX);
    assert!(Combat::with_heroes(15, heroes).is_ok());
    assert!(HeroSetup::new(
        ActorId(0),
        HeroClass::Scout,
        AbilityLoadout::new([]).expect("empty")
    )
    .is_err());
}

#[test]
fn equipped_cross_class_skill_is_authorized_and_unequipped_starter_is_not() {
    let mut heroes = setup();
    heroes.first_mut().expect("gatekeeper").abilities =
        AbilityLoadout::new([SkillId::SnapShot]).expect("scout ability");
    let mut combat = Combat::with_heroes(15, heroes).expect("cross-class loadout");
    let actor = ActorId(90);
    wait_for(&mut combat, actor);
    assert_eq!(
        combat.state.actor(actor).expect("gatekeeper").skills(),
        &[SkillId::SnapShot]
    );
    let before = combat.clone();
    assert_eq!(
        combat.apply(
            actor,
            CombatAction::Skill {
                skill: SkillId::FrontStrike,
                target: ActorId(101)
            }
        ),
        Err(RuleError::UnknownSkill)
    );
    assert_eq!(
        combat, before,
        "class membership must not bypass the equipped loadout"
    );
    let hp = combat.state.actor(ActorId(106)).expect("rear enemy").hp;
    combat
        .apply(
            actor,
            CombatAction::Skill {
                skill: SkillId::SnapShot,
                target: ActorId(106),
            },
        )
        .expect("equipped cross-class attack");
    assert_eq!(
        combat.state.actor(ActorId(106)).expect("rear enemy").hp,
        hp - 4
    );
    assert_eq!(
        combat.state.actor(actor).expect("same archetype").kind,
        ActorKind::Hero(HeroClass::Gatekeeper)
    );
}

#[test]
fn repeated_class_actors_keep_independent_uses_hp_and_statuses() {
    let mut combat = Combat::with_heroes(15, setup()).expect("party");
    let left = ActorId(900);
    let right = ActorId(300);
    wait_for(&mut combat, left);
    combat.actor_mut(left).expect("first medic").hp = 10;
    combat
        .apply(
            left,
            CombatAction::Skill {
                skill: SkillId::Mend,
                target: left,
            },
        )
        .expect("Mend from rank five");
    combat
        .add_status(
            ActorId(103),
            left,
            StatusKind::Bleed,
            &mut Vec::new(),
            &mut Work(MAX_WORK),
        )
        .expect("bleed first medic");
    let left_state = combat.state.actor(left).expect("first medic");
    let right_state = combat.state.actor(right).expect("second medic");
    assert_eq!(left_state.kind, right_state.kind);
    assert_eq!(left_state.remaining_uses(SkillId::Mend), Some(2));
    assert_eq!(right_state.remaining_uses(SkillId::Mend), Some(3));
    assert_eq!(left_state.hp, 18);
    assert_eq!(right_state.hp, right_state.max_hp);
    assert_eq!(left_state.statuses.len(), 1);
    assert!(right_state.statuses.is_empty());
    assert_eq!(left_state.statuses.first().expect("status").bearer, left);
    combat
        .snapshot()
        .validate()
        .expect("independent actor state");
}

#[test]
fn loadouts_are_bounded_unique_serde_validated_and_may_be_empty() {
    assert_eq!(
        AbilityLoadout::new([SkillId::Mend, SkillId::Mend]),
        Err(RuleError::InvalidLoadout)
    );
    assert_eq!(
        AbilityLoadout::new(SkillId::ALL.into_iter().take(MAX_EQUIPPED_ABILITIES + 1)),
        Err(RuleError::InvalidLoadout)
    );
    let maximum = AbilityLoadout::new(SkillId::ALL.into_iter().take(MAX_EQUIPPED_ABILITIES))
        .expect("maximum");
    assert_eq!(maximum.as_slice().len(), MAX_EQUIPPED_ABILITIES);
    let wire = serde_json::to_vec(&maximum).expect("serialize");
    assert_eq!(
        serde_json::from_slice::<AbilityLoadout>(&wire).expect("validate"),
        maximum
    );
    assert!(serde_json::from_str::<AbilityLoadout>(r#"["Mend","Mend"]"#).is_err());
    assert!(serde_json::from_str::<AbilityLoadout>(r#"["InventedSkill"]"#).is_err());
    let too_many = serde_json::to_vec(
        &SkillId::ALL
            .into_iter()
            .take(MAX_EQUIPPED_ABILITIES + 1)
            .collect::<Vec<_>>(),
    )
    .expect("fixture");
    assert!(serde_json::from_slice::<AbilityLoadout>(&too_many).is_err());
    let mut heroes = setup();
    heroes.first_mut().expect("first").abilities = AbilityLoadout::new([]).expect("empty");
    let mut combat = Combat::with_heroes(15, heroes).expect("unarmed party member");
    wait_for(&mut combat, ActorId(90));
    let actions = combat.legal_actions(ActorId(90));
    assert!(!actions
        .iter()
        .any(|action| matches!(action, CombatAction::Skill { .. })));
    assert!(actions.contains(&CombatAction::Defend));
    assert!(actions.contains(&CombatAction::Wait));
    assert!(actions.contains(&CombatAction::Reposition { ally: ActorId(12) }));
}

#[test]
fn loadout_and_use_tampering_is_rejected_on_snapshot_ingress() {
    let combat = Combat::with_heroes(15, setup()).expect("party");
    let valid = serde_json::to_value(combat.snapshot()).expect("snapshot");
    for (path, replacement) in [
        (
            "/actors/0/abilities",
            serde_json::json!(["FrontStrike", "FrontStrike"]),
        ),
        ("/actors/0/abilities", serde_json::json!(["InventedSkill"])),
        (
            "/actors/0/abilities",
            serde_json::json!(SkillId::ALL.into_iter().take(9).collect::<Vec<_>>()),
        ),
        ("/actors/0/skill_uses", serde_json::json!({"Mend":1})),
        ("/actors/0/skill_uses", serde_json::json!({"FrontStrike":1})),
        (
            "/actors/0/skill_uses",
            serde_json::json!({"FieldDressing":0}),
        ),
        (
            "/actors/0/skill_uses",
            serde_json::json!({"FieldDressing":3}),
        ),
        ("/actors/0/id", serde_json::json!(12)),
        ("/actors/0/id", serde_json::json!(101)),
        ("/initiative/0/tie_breaker", serde_json::json!(12)),
    ] {
        let mut malformed = valid.clone();
        *malformed.pointer_mut(path).expect("existing path") = replacement;
        assert!(
            serde_json::from_value::<CombatSnapshot>(malformed).is_err(),
            "{path}"
        );
    }
    let mut short = valid.clone();
    short
        .pointer_mut("/hero_formation")
        .expect("formation")
        .as_array_mut()
        .expect("array")
        .pop();
    assert!(serde_json::from_value::<CombatSnapshot>(short).is_err());
    let mut substituted_enemy = valid.clone();
    let enemy = substituted_enemy
        .pointer_mut("/actors/6")
        .expect("authored first enemy");
    let (max_hp, base_speed) = ActorKind::Enemy(EnemyKind::HollowArcher).stats();
    enemy["kind"] = serde_json::json!({"Enemy":"HollowArcher"});
    enemy["max_hp"] = serde_json::json!(max_hp + 1);
    enemy["hp"] = serde_json::json!(max_hp);
    enemy["base_speed"] = serde_json::json!(base_speed);
    assert!(serde_json::from_value::<CombatSnapshot>(substituted_enemy).is_err());
    let mut missing = valid;
    missing
        .pointer_mut("/actors/0")
        .expect("actor")
        .as_object_mut()
        .expect("object")
        .remove("abilities");
    assert!(serde_json::from_value::<CombatSnapshot>(missing).is_err());
}

#[test]
fn every_default_hero_has_rank_appropriate_attacks_and_utilities_in_six_ranks() {
    for id in 1..=6 {
        let actor = ActorId(id);
        let mut combat = Combat::new(42, DEFAULT_HERO_ROSTER).expect("party");
        wait_for(&mut combat, actor);
        let actions = combat.legal_actions(actor);
        assert!(
            actions.iter().any(|action| match action {
                CombatAction::Skill { skill, .. } => skill_definition(*skill)
                    .effects
                    .iter()
                    .any(|effect| matches!(effect, Effect::Damage(_))),
                _ => false,
            }),
            "default actor {id} must have a usable attack at its authored rank"
        );
        if id >= 5 {
            assert!(actions.contains(&CombatAction::Skill {
                skill: SkillId::Mend,
                target: actor
            }));
        }
    }
    for skill in SkillId::ALL {
        let definition = skill_definition(skill);
        assert!(definition.source_ranks != 0 && definition.source_ranks & !0b11_1111 == 0);
        assert!(definition.target_ranks != 0 && definition.target_ranks & !0b11_1111 == 0);
        for invalid in [0, 7, 8, u8::MAX] {
            assert!(!definition.allows_source_rank(invalid));
            assert!(!definition.allows_target_rank(invalid));
        }
    }
}

#[test]
fn front_middle_rear_masks_enforce_linear_source_and_target_edges() {
    for source_rank in 1..=6_u8 {
        for target_rank in 1..=6_u8 {
            for skill in [
                SkillId::FrontStrike,
                SkillId::LongReach,
                SkillId::BackRankShot,
                SkillId::SnapShot,
            ] {
                let mut heroes = setup();
                for hero in &mut heroes {
                    hero.abilities = AbilityLoadout::new([skill]).expect("one ability");
                }
                let mut combat = Combat::with_heroes(15, heroes).expect("party");
                let actor = *combat
                    .state
                    .hero_formation
                    .get(usize::from(source_rank - 1))
                    .expect("source rank");
                let target = *combat
                    .state
                    .enemy_formation
                    .get(usize::from(target_rank - 1))
                    .expect("target rank");
                wait_for(&mut combat, actor);
                let expected = match skill {
                    SkillId::FrontStrike => (source_rank <= 2, target_rank <= 2),
                    SkillId::LongReach => (source_rank <= 4, target_rank <= 4),
                    SkillId::BackRankShot => (source_rank >= 3, target_rank >= 5),
                    SkillId::SnapShot => (true, true),
                    _ => unreachable!("fixed test catalog"),
                };
                let expected = if !expected.0 {
                    Err(RuleError::WrongRank)
                } else if !expected.1 {
                    Err(RuleError::WrongTargetRank)
                } else {
                    Ok(())
                };
                assert_eq!(
                    combat.validate_action(actor, &CombatAction::Skill { skill, target }),
                    expected,
                    "{skill:?}: source {source_rank}, target {target_rank}"
                );
            }
        }
    }
}

#[test]
fn sixth_rank_reposition_does_not_wrap_and_downing_does_not_shift_identity() {
    let mut combat = Combat::with_heroes(15, setup()).expect("party");
    let sixth = ActorId(300);
    let fifth = ActorId(900);
    wait_for(&mut combat, sixth);
    let before = combat.clone();
    assert_eq!(
        combat.apply(sixth, CombatAction::Reposition { ally: ActorId(90) }),
        Err(RuleError::NotAdjacent)
    );
    assert_eq!(combat, before);
    combat
        .apply(sixth, CombatAction::Reposition { ally: fifth })
        .expect("sixth and fifth swap");
    assert_eq!(combat.state.rank(sixth), Some(5));
    assert_eq!(combat.state.rank(fifth), Some(6));
    combat
        .damage(
            ActorId(101),
            fifth,
            100,
            DamageKind::Direct,
            &mut Vec::new(),
            &mut Work(MAX_WORK),
        )
        .expect("down");
    assert_eq!(combat.state.rank(fifth), Some(6));
    assert_eq!(combat.state.hero_formation.len(), PARTY_SIZE);
    combat
        .effect(
            sixth,
            fifth,
            Effect::Rescue(25),
            0,
            &mut Vec::new(),
            &mut Work(MAX_WORK),
        )
        .expect("rescue");
    assert_eq!(combat.state.rank(fifth), Some(6));
    assert!(combat.state.actor(fifth).expect("same actor").standing());
    combat
        .snapshot()
        .validate()
        .expect("valid stable identity after rank-six rescue");
}
