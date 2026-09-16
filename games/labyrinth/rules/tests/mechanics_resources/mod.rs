use crate::mechanics_support::*;
use labyrinth_rules::{
    scenario::StartingStatus, ActorId, CombatAction, LifeState, SkillId, StatusKind,
};

#[test]
fn every_limited_heal_exhausts_without_losing_moveset_membership_or_spending_rejected_turn() {
    for (key, uses, heal, rank, target) in [
        ("field_dressing", 2, 8, 1, 1),
        ("spare_bandage", 2, 3, 1, 6),
        ("mend", 3, 8, 3, 6),
    ] {
        let (catalog, mut input) = scenario(None, &[key], rank, &[1]);
        input
            .heroes
            .iter_mut()
            .find(|a| a.id == ActorId(target))
            .expect("target")
            .starting_hp = Some(10);
        let mut game = combat(&catalog, &input);
        for used in 1..=uses {
            wait_for(&mut game, ActorId(1));
            let command = action(&game, key, target);
            commit(&mut game, command);
            let state = game.snapshot();
            assert_eq!(
                state.actor(ActorId(target)).expect("target").hp,
                10 + heal * used
            );
        }
        wait_for(&mut game, ActorId(1));
        let command = action(&game, key, target);
        reject(&mut game, command);
        assert!(game
            .snapshot()
            .actor(ActorId(1))
            .expect("source")
            .skill_index(&id(key))
            .is_some());
    }
}

#[test]
fn rally_and_legacy_alias_share_a_bounded_use_counter() {
    let (catalog, mut input) = scenario(None, &["rally"], 1, &[1]);
    for actor in &mut input.heroes {
        if matches!(actor.id.0, 4..=6) {
            actor.starting_hp = Some(0);
        }
    }
    let mut game = combat(&catalog, &input);
    for (target, legacy) in [(6, false), (5, true)] {
        wait_for(&mut game, ActorId(1));
        let command = if legacy {
            CombatAction::LegacySkill {
                skill: SkillId::Rally,
                target: ActorId(target),
            }
        } else {
            action(&game, "rally", target)
        };
        // Enumeration deliberately uses canonical indexed commands, not aliases.
        game.apply(ActorId(1), command).expect("Rally/alias");
        assert_eq!(
            game.snapshot().actor(ActorId(target)).expect("rescued").hp,
            50
        );
    }
    wait_for(&mut game, ActorId(1));
    let command = action(&game, "rally", 4);
    reject(&mut game, command);
    reject(
        &mut game,
        CombatAction::LegacySkill {
            skill: SkillId::Rally,
            target: ActorId(4),
        },
    );
}

#[test]
fn heal_caps_and_full_health_casts_spend_one_use_without_cleansing_bleed() {
    for (key, rank, target) in [
        ("field_dressing", 1, 1),
        ("spare_bandage", 1, 6),
        ("mend", 3, 6),
    ] {
        for initial in [99, 100] {
            let (catalog, mut input) = scenario(None, &[key], rank, &[1]);
            let recipient = input
                .heroes
                .iter_mut()
                .find(|a| a.id == ActorId(target))
                .expect("target");
            recipient.starting_hp = Some(initial);
            if initial == 99 {
                recipient.starting_statuses.push(StartingStatus {
                    kind: StatusKind::Bleed,
                    source: Some(ActorId(101)),
                    remaining: Some(3),
                });
            }
            let mut game = combat(&catalog, &input);
            let command = action(&game, key, target);
            commit(&mut game, command);
            let state = game.snapshot();
            let target_actor = state.actor(ActorId(target)).expect("target");
            assert_eq!(target_actor.hp, 100);
            assert_eq!(target_actor.statuses.len(), usize::from(initial == 99));
            let source = state.actor(ActorId(1)).expect("source");
            let index = source.skill_index(&id(key)).expect("Skill");
            assert_eq!(
                source.remaining_skill_uses(index),
                Some(if key == "mend" { 2 } else { 1 })
            );
        }
    }
}

#[test]
fn rescue_rounds_up_but_does_not_cleanse() {
    for (skill, expected) in [(false, 26), (true, 51)] {
        let (catalog, mut input) = scenario(None, &["rally"], 1, &[1]);
        let recipient = input
            .heroes
            .iter_mut()
            .find(|a| a.id == ActorId(6))
            .expect("target");
        recipient.actor.max_hp = 101;
        recipient.starting_hp = Some(0);
        recipient.starting_statuses.push(StartingStatus {
            kind: StatusKind::Bleed,
            source: Some(ActorId(101)),
            remaining: Some(3),
        });
        let mut game = combat(&catalog, &input);
        let command = if skill {
            action(&game, "rally", 6)
        } else {
            CombatAction::Rescue { ally: ActorId(6) }
        };
        commit(&mut game, command);
        let state = game.snapshot();
        let target = state.actor(ActorId(6)).expect("target");
        assert_eq!(target.hp, expected);
        assert_eq!(target.life, LifeState::Alive);
        assert_eq!(target.statuses.len(), 1);
    }
}

#[test]
fn support_skills_and_utilities_cannot_restore_or_reposition_corpses() {
    let (catalog, mut input) = scenario(
        None,
        &[
            "spare_bandage",
            "field_dressing",
            "mend",
            "staunch",
            "clean_blade",
            "rally",
            "exchange",
        ],
        3,
        &[1, 1],
    );
    input
        .heroes
        .iter_mut()
        .find(|a| a.id == ActorId(6))
        .expect("target")
        .starting_hp = Some(0);
    let enemy = input.enemies.first_mut().expect("enemy");
    enemy.actor.base_speed = 80;
    enemy
        .actor
        .build
        .skills
        .push(labyrinth_rules::build::SkillGrant {
            skill: id("hurled_scrap"),
            provenance: id("scenario"),
        });
    let mut game = combat(&catalog, &input);
    for _ in 0..50 {
        let state = game.snapshot();
        if state.actor(ActorId(6)).expect("target").is_corpse() {
            break;
        }
        let active = state.active_actor.expect("turn");
        let command = if active == ActorId(101) {
            CombatAction::Skill {
                index: state
                    .actor(active)
                    .expect("enemy")
                    .skill_index(&id("hurled_scrap"))
                    .expect("Skill"),
                target: ActorId(6),
            }
        } else {
            CombatAction::Wait
        };
        game.apply(active, command)
            .expect("scripted external action");
    }
    wait_for(&mut game, ActorId(1));
    assert!(game
        .snapshot()
        .actor(ActorId(6))
        .expect("target")
        .is_corpse());
    for key in [
        "spare_bandage",
        "field_dressing",
        "mend",
        "staunch",
        "clean_blade",
        "rally",
        "exchange",
    ] {
        let command = action(&game, key, 6);
        reject(&mut game, command);
    }
    reject(&mut game, CombatAction::Rescue { ally: ActorId(6) });
    reject(&mut game, CombatAction::Reposition { ally: ActorId(6) });
}

#[test]
fn rescue_replaces_a_nonzero_death_failure_state() {
    let (catalog, mut input) = scenario(None, &["rally"], 1, &[1, 1]);
    input
        .heroes
        .iter_mut()
        .find(|a| a.id == ActorId(6))
        .expect("target")
        .starting_hp = Some(0);
    let enemy = input.enemies.first_mut().expect("enemy");
    enemy.actor.base_speed = 80;
    enemy
        .actor
        .build
        .skills
        .push(labyrinth_rules::build::SkillGrant {
            skill: id("hurled_scrap"),
            provenance: id("scenario"),
        });
    let mut game = combat(&catalog, &input);
    game.apply(ActorId(1), CombatAction::Wait)
        .expect("source waits");
    wait_for(&mut game, ActorId(101));
    let index = game
        .snapshot()
        .actor(ActorId(101))
        .expect("enemy")
        .skill_index(&id("hurled_scrap"))
        .expect("Skill");
    game.apply(
        ActorId(101),
        CombatAction::Skill {
            index,
            target: ActorId(6),
        },
    )
    .expect("damage while dying");
    wait_for(&mut game, ActorId(1));
    assert!(matches!(
        game.snapshot().actor(ActorId(6)).expect("target").life,
        LifeState::Dying { failures: 1..=2 }
    ));
    let command = action(&game, "rally", 6);
    commit(&mut game, command);
    assert_eq!(
        game.snapshot().actor(ActorId(6)).expect("target").life,
        LifeState::Alive
    );
}

#[test]
fn unlimited_dagger_throw_consumes_no_charge_or_equipment() {
    let (catalog, input) = scenario(Some("dagger"), &[], 3, &[1]);
    let mut game = combat(&catalog, &input);
    for count in 1..=5 {
        wait_for(&mut game, ActorId(1));
        let command = action(&game, "dagger_throw", 101);
        commit(&mut game, command);
        let state = game.snapshot();
        assert_eq!(
            state.actor(ActorId(101)).expect("target").hp,
            100 - count * 4
        );
        let source = state.actor(ActorId(1)).expect("source");
        assert!(source.skill_uses.is_empty());
        assert_eq!(source.build.weapon, Some(id("dagger")));
    }
}

#[test]
fn brace_and_weakened_modify_direct_damage_with_zero_floor() {
    for (weapon, key, base, expected) in [
        (None, "hurled_scrap", 2, 0),
        (Some("dagger"), "dagger_stab", 5, 1),
    ] {
        let personal = if weapon.is_none() { vec![key] } else { vec![] };
        let (catalog, mut input) = scenario(weapon, &personal, 1, &[1]);
        input
            .heroes
            .first_mut()
            .expect("source")
            .starting_statuses
            .push(StartingStatus {
                kind: StatusKind::Weakened,
                source: None,
                remaining: Some(2),
            });
        input
            .enemies
            .first_mut()
            .expect("target")
            .starting_statuses
            .push(StartingStatus {
                kind: StatusKind::Brace,
                source: None,
                remaining: Some(1),
            });
        let mut game = combat(&catalog, &input);
        let command = action(&game, key, 101);
        let preview = game
            .snapshot()
            .preview_action(ActorId(1), &command)
            .expect("preview");
        assert_eq!(preview.damage.first().expect("damage").base, base);
        commit(&mut game, command);
        assert_eq!(
            game.snapshot().actor(ActorId(101)).expect("target").hp,
            100 - expected
        );
    }
}

#[test]
fn defend_protects_direct_damage_but_not_bleed_and_expires_at_owner_start() {
    let (catalog, mut input) = scenario(None, &[], 1, &[1]);
    input
        .heroes
        .first_mut()
        .expect("source")
        .starting_statuses
        .push(StartingStatus {
            kind: StatusKind::Bleed,
            source: None,
            remaining: Some(3),
        });
    let mut game = combat(&catalog, &input);
    assert_eq!(game.snapshot().actor(ActorId(1)).expect("source").hp, 98);
    commit(&mut game, CombatAction::Defend);
    assert!(game
        .snapshot()
        .actor(ActorId(1))
        .expect("source")
        .statuses
        .iter()
        .any(|s| s.kind == StatusKind::Brace));
    wait_for(&mut game, ActorId(1));
    let state = game.snapshot();
    let source = state.actor(ActorId(1)).expect("source");
    assert_eq!(source.hp, 96);
    assert!(!source.statuses.iter().any(|s| s.kind == StatusKind::Brace));
}

#[test]
fn cleanses_remove_only_bleeding_and_allow_empty_no_op_targets() {
    for (key, target) in [("clean_blade", 1), ("staunch", 6)] {
        for bleeding in [true, false] {
            let (catalog, mut input) = scenario(None, &[key], 1, &[1]);
            let actor = input
                .heroes
                .iter_mut()
                .find(|a| a.id == ActorId(target))
                .expect("target");
            actor.starting_statuses.push(StartingStatus {
                kind: StatusKind::Haste,
                source: None,
                remaining: Some(2),
            });
            if bleeding {
                actor.starting_statuses.push(StartingStatus {
                    kind: StatusKind::Bleed,
                    source: None,
                    remaining: Some(3),
                });
            }
            let mut game = combat(&catalog, &input);
            let hp = game.snapshot().actor(ActorId(target)).expect("target").hp;
            let command = action(&game, key, target);
            commit(&mut game, command);
            let state = game.snapshot();
            let recipient = state.actor(ActorId(target)).expect("target");
            assert_eq!(recipient.hp, hp);
            assert_eq!(recipient.statuses.len(), 1);
            assert_eq!(
                recipient.statuses.first().expect("kept buff").kind,
                StatusKind::Haste
            );
        }
    }
}
