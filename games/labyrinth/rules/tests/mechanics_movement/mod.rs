use crate::mechanics_support::*;
use labyrinth_rules::{ActorId, ActorKind, CombatAction, EnemyKind, MovementLimit, Team};

#[test]
fn hook_shot_calibration_pulls_in_front_of_a_larger_neighbor() {
    let (catalog, mut input) = scenario(Some("scout_bow"), &[], 3, &[1, 1, 2, 1, 1]);
    // Match the prototype blocker, while retaining explicit HP/initiative.
    input.enemies.get_mut(2).expect("blocker").actor.appearance =
        labyrinth_rules::ActorKind::Enemy(labyrinth_rules::EnemyKind::OssuaryHauler);
    let mut game = combat(&catalog, &input);
    let command = action(&game, "hook_shot", 104);
    let before = game.clone();
    let preview = game
        .snapshot()
        .preview_action(ActorId(1), &command)
        .expect("forecast");
    assert_eq!(game, before, "preview is inert");
    game.apply(ActorId(1), command).expect("legal Hook Shot");
    let after = game.snapshot();
    assert_eq!(after.actor(ActorId(104)).expect("target").hp, 97);
    assert_eq!(
        after.formation(Team::Enemies),
        &[
            ActorId(101),
            ActorId(102),
            ActorId(104),
            ActorId(103),
            ActorId(105)
        ]
    );
    assert_eq!(
        after.ranks(ActorId(104)),
        Some(3..=3),
        "target emerges in front of whole blocker"
    );
    assert_eq!(
        after.ranks(ActorId(103)),
        Some(4..=5),
        "large blocker moves back one rank"
    );
    assert_eq!(preview.movement.first().expect("pull forecast").to, 3);
    after.validate().expect("valid resulting formation");
}

#[test]
fn hook_shot_crosses_every_legal_pair_of_custom_footprints() {
    for blocker_width in 1..=5 {
        for target_width in 1..=6 - blocker_width {
            // Hook Shot reaches ranks 3–6; ensure the target overlaps that range.
            let widths = if blocker_width + target_width < 3 {
                vec![1, blocker_width, target_width]
            } else {
                vec![blocker_width, target_width]
            };
            let (catalog, input) = scenario(Some("scout_bow"), &[], 3, &widths);
            let target = 100 + u16::try_from(widths.len()).expect("bounded roster");
            let blocker = target - 1;
            let mut game = combat(&catalog, &input);
            let before = game.snapshot();
            let destination = before.rank(ActorId(blocker)).expect("blocker rank");
            let command = action(&game, "hook_shot", target);
            commit(&mut game, command);
            let after = game.snapshot();
            assert_eq!(
                after.rank(ActorId(target)),
                Some(destination),
                "blocker {blocker_width}, target {target_width}"
            );
            assert_eq!(
                after.rank(ActorId(blocker)),
                Some(destination + target_width)
            );
            assert_eq!(after.actor(ActorId(target)).expect("target").hp, 97);
            assert_eq!(after.actor(ActorId(blocker)).expect("blocker").hp, 100);
        }
    }
}

#[test]
fn sole_large_target_has_no_preceding_occupant_and_pull_does_not_wrap() {
    let (catalog, input) = scenario(Some("scout_bow"), &[], 3, &[6]);
    let mut game = combat(&catalog, &input);
    let command = action(&game, "hook_shot", 101);
    let preview = game
        .snapshot()
        .preview_action(ActorId(1), &command)
        .expect("forecast");
    assert_eq!(
        preview.movement.first().expect("pull").limit,
        Some(MovementLimit::FormationEdge)
    );
    commit(&mut game, command);
    assert_eq!(game.snapshot().ranks(ActorId(101)), Some(1..=6));
    assert_eq!(game.snapshot().actor(ActorId(101)).expect("target").hp, 97);
}

#[test]
fn reach_intersects_configured_source_and_target_spans_not_only_leading_ranks() {
    let (catalog, mut input) = scenario(Some("scout_bow"), &[], 2, &[3, 1]);
    input.heroes.truncate(5);
    input
        .heroes
        .iter_mut()
        .find(|a| a.id == ActorId(1))
        .expect("source")
        .actor
        .footprint = 2;
    let mut game = combat(&catalog, &input);
    assert_eq!(game.snapshot().ranks(ActorId(1)), Some(2..=3));
    assert_eq!(game.snapshot().ranks(ActorId(101)), Some(1..=3));
    let command = action(&game, "hook_shot", 101);
    commit(&mut game, command);
    assert_eq!(game.snapshot().actor(ActorId(101)).expect("target").hp, 97);
    assert_eq!(game.snapshot().ranks(ActorId(101)), Some(1..=3));
}

#[test]
fn pushes_use_configured_neighbor_width_not_appearance() {
    for (appearance, width, expected_rank) in [
        (EnemyKind::AshBrute, 2, 1),
        (EnemyKind::OssuaryHauler, 1, 2),
    ] {
        let (catalog, mut input) = scenario(Some("spear"), &[], 1, &[1, width]);
        input.enemies.get_mut(1).expect("neighbor").actor.appearance = ActorKind::Enemy(appearance);
        let mut game = combat(&catalog, &input);
        let command = action(&game, "spear_shove", 101);
        commit(&mut game, command);
        assert_eq!(game.snapshot().rank(ActorId(101)), Some(expected_rank));
    }
}

#[test]
fn two_rank_push_spends_actual_width_and_reports_partial_movement() {
    for (widths, expected_rank, limit) in [
        (vec![1, 2, 1], 3, None),
        (
            vec![1, 1, 2],
            2,
            Some(MovementLimit::Footprint {
                actor: ActorId(103),
                ranks: 2,
            }),
        ),
        (
            vec![1, 3],
            1,
            Some(MovementLimit::Footprint {
                actor: ActorId(102),
                ranks: 3,
            }),
        ),
        (vec![1], 1, Some(MovementLimit::FormationEdge)),
    ] {
        let (catalog, input) = scenario(Some("gatekeeper_spear"), &[], 1, &widths);
        let mut game = combat(&catalog, &input);
        let command = action(&game, "driving_blow", 101);
        let preview = game
            .snapshot()
            .preview_action(ActorId(1), &command)
            .expect("forecast");
        assert_eq!(preview.movement.first().expect("push").limit, limit);
        commit(&mut game, command);
        assert_eq!(game.snapshot().rank(ActorId(101)), Some(expected_rank));
        assert_eq!(game.snapshot().actor(ActorId(101)).expect("target").hp, 97);
    }
}

#[test]
fn lethal_hook_shot_damages_but_does_not_pull_new_corpse() {
    let (catalog, mut input) = scenario(Some("scout_bow"), &[], 3, &[1, 1, 1, 1]);
    input.enemies.get_mut(2).expect("target").starting_hp = Some(3);
    let mut game = combat(&catalog, &input);
    let command = action(&game, "hook_shot", 103);
    let before = game.snapshot().enemy_formation;
    commit(&mut game, command);
    assert!(game
        .snapshot()
        .actor(ActorId(103))
        .expect("target")
        .is_corpse());
    assert_eq!(game.snapshot().enemy_formation, before);
}

#[test]
fn whole_actor_exchange_and_reposition_preserve_identity_and_build() {
    for command_kind in ["exchange", "reposition"] {
        let (catalog, mut input) = scenario(None, &["exchange"], 1, &[1]);
        input.heroes.truncate(3);
        input.heroes.get_mut(1).expect("neighbor").actor.footprint = 3;
        let mut game = combat(&catalog, &input);
        let before = game.snapshot();
        let command = if command_kind == "exchange" {
            action(&game, "exchange", 2)
        } else {
            CombatAction::Reposition { ally: ActorId(2) }
        };
        commit(&mut game, command);
        assert_eq!(game.snapshot().ranks(ActorId(1)), Some(4..=4));
        assert_eq!(game.snapshot().ranks(ActorId(2)), Some(1..=3));
        assert_eq!(
            game.snapshot()
                .actor(ActorId(1))
                .expect("source")
                .resolved_build,
            before.actor(ActorId(1)).expect("source").resolved_build
        );
    }
}

#[test]
fn sparse_preparation_is_not_a_second_combat_formation_model() {
    // Saved combat input is an ordered compact roster; unused capacity stays at the rear.
    let (catalog, input) = scenario(Some("staff"), &[], 1, &[1, 1]);
    let mut game = combat(&catalog, &input);
    let command = action(&game, "staff_push", 102);
    commit(&mut game, command);
    assert_eq!(
        game.snapshot().rank(ActorId(102)),
        Some(2),
        "cannot push into unused trailing capacity"
    );
    for rank in 3..=6 {
        assert_eq!(game.snapshot().occupant(Team::Enemies, rank), None);
    }
}
