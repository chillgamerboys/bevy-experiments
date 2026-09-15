use crate::mechanics_support::*;
use labyrinth_rules::{
    catalog::{ContentCatalog, EquipmentRequirement, UpgradeOperation},
    scenario::{Scenario, StartingStatus},
    ActorId, CombatAction, Effect, Stat, StatusKind,
};

fn passive_fixture(abilities: &[&str], target_resilient: bool) -> (ContentCatalog, Scenario) {
    let (catalog, mut input) = scenario(Some("dagger"), &[], 1, &[1, 1]);
    input
        .heroes
        .first_mut()
        .expect("source")
        .actor
        .build
        .abilities = abilities.iter().map(|a| id(a)).collect();
    if target_resilient {
        input
            .enemies
            .first_mut()
            .expect("target")
            .actor
            .build
            .abilities
            .push(id("resilient"));
    }
    (catalog, input)
}

#[test]
fn every_builtin_passive_has_an_explicit_runtime_case() {
    let catalog = ContentCatalog::builtin().expect("catalog");
    let actual: std::collections::BTreeSet<_> = catalog
        .definition()
        .abilities
        .iter()
        .map(|a| a.id.as_str())
        .collect();
    assert_eq!(
        actual,
        [
            "assassin_bleeding_dagger",
            "duelist_dagger_power",
            "resilient"
        ]
        .into_iter()
        .collect()
    );
}

#[test]
fn dagger_upgrades_execute_individually_and_together_without_becoming_actions() {
    for (abilities, damage, bleed) in [
        (vec![], 5, false),
        (vec!["assassin_bleeding_dagger"], 5, true),
        (vec!["duelist_dagger_power"], 7, false),
        (
            vec!["duelist_dagger_power", "assassin_bleeding_dagger"],
            7,
            true,
        ),
    ] {
        let (catalog, input) = passive_fixture(&abilities, false);
        let mut game = combat(&catalog, &input);
        let source = game.snapshot();
        let source = source.actor(ActorId(1)).expect("source");
        assert_eq!(source.moveset().skills.len(), 2);
        assert!(source.resolved_build.abilities.iter().all(|a| a.active()));
        let command = action(&game, "dagger_stab", 101);
        commit(&mut game, command);
        let state = game.snapshot();
        let target = state.actor(ActorId(101)).expect("target");
        assert_eq!(target.hp, 100 - damage);
        assert_eq!(
            target.statuses.iter().any(|s| s.kind == StatusKind::Bleed),
            bleed
        );
    }
}

#[test]
fn missing_weapon_or_upgrade_target_disables_entire_ability_and_restoring_dagger_reactivates() {
    for weapon in [
        None,
        Some("staff"),
        Some("knifehand_daggers"),
        Some("dagger"),
    ] {
        let (catalog, mut input) =
            passive_fixture(&["assassin_bleeding_dagger", "duelist_dagger_power"], false);
        input.heroes.first_mut().expect("source").actor.build.weapon = weapon.map(id);
        let game = combat(&catalog, &input);
        let state = game.snapshot();
        let source = state.actor(ActorId(1)).expect("source");
        assert_eq!(source.build.abilities.len(), 2, "retained selection");
        assert!(source
            .resolved_build
            .abilities
            .iter()
            .all(|a| a.active() == (weapon == Some("dagger"))));
        if weapon == Some("dagger") {
            let mut game = game;
            let command = action(&game, "dagger_stab", 101);
            commit(&mut game, command);
            assert_eq!(game.snapshot().actor(ActorId(101)).expect("target").hp, 93);
        }
    }
}

#[test]
fn duplicate_personal_and_equipment_passive_grants_apply_once_in_combat() {
    let (catalog, mut input) = passive_fixture(
        &[
            "assassin_bleeding_dagger",
            "duelist_dagger_power",
            "resilient",
        ],
        true,
    );
    let mut raw = catalog.definition().clone();
    raw.weapons
        .iter_mut()
        .find(|w| w.id == id("dagger"))
        .expect("weapon")
        .abilities = vec![
        id("assassin_bleeding_dagger"),
        id("duelist_dagger_power"),
        id("resilient"),
    ];
    input
        .enemies
        .first_mut()
        .expect("target")
        .actor
        .build
        .weapon = Some(id("dagger"));
    let catalog = ContentCatalog::new(raw).expect("authored grants");
    let mut game = combat(&catalog, &input);
    let state = game.snapshot();
    let source = state.actor(ActorId(1)).expect("source");
    assert!(source
        .resolved_build
        .abilities
        .iter()
        .all(|a| a.grants.len() == 2));
    let command = action(&game, "dagger_stab", 101);
    commit(&mut game, command);
    let state = game.snapshot();
    let target = state.actor(ActorId(101)).expect("target");
    assert_eq!(target.hp, 93);
    assert_eq!(target.statuses.len(), 1);
    assert_eq!(
        target.statuses.first().expect("bleed").remaining,
        2,
        "Resilient is not doubled"
    );
}

fn status_fixture(kind: StatusKind, resilient: bool) -> (ContentCatalog, Scenario) {
    let (catalog, mut input) = passive_fixture(&[], resilient);
    let mut raw = catalog.definition().clone();
    let mut skill = raw
        .skills
        .iter()
        .find(|s| s.id == id("dagger_stab"))
        .expect("fixture Skill")
        .clone();
    skill.id = id("status_probe");
    skill.personal_selectable = true;
    skill.effects = vec![Effect::ApplyStatus(kind)];
    raw.skills.push(skill);
    input
        .heroes
        .first_mut()
        .expect("source")
        .actor
        .build
        .skills
        .push(labyrinth_rules::build::SkillGrant {
            skill: id("status_probe"),
            provenance: id("scenario"),
        });
    (
        ContentCatalog::new(raw).expect("authored status-only fixture"),
        input,
    )
}

#[test]
fn resilient_shortens_negative_applications_but_not_buffs() {
    for (kind, normal, shortened) in [
        (StatusKind::Bleed, 3, 2),
        (StatusKind::Weakened, 2, 1),
        (StatusKind::Haste, 2, 2),
        (StatusKind::Brace, 1, 1),
    ] {
        for resilient in [false, true] {
            let (catalog, input) = status_fixture(kind, resilient);
            let mut game = combat(&catalog, &input);
            let command = action(&game, "status_probe", 101);
            commit(&mut game, command);
            let state = game.snapshot();
            let target = state.actor(ActorId(101)).expect("target");
            assert_eq!(target.hp, 100);
            assert_eq!(
                target.statuses.first().expect("status").remaining,
                if resilient { shortened } else { normal }
            );
        }
    }
}

#[test]
fn bleed_ticks_exactly_its_accepted_owner_start_duration_with_and_without_resilient() {
    for (resilient, ticks) in [(false, 3u8), (true, 2)] {
        let (catalog, input) = status_fixture(StatusKind::Bleed, resilient);
        let mut game = combat(&catalog, &input);
        let command = action(&game, "status_probe", 101);
        commit(&mut game, command);
        for n in 1..=ticks + 1 {
            wait_for(&mut game, ActorId(101));
            let state = game.snapshot();
            let target = state.actor(ActorId(101)).expect("target");
            assert_eq!(target.hp, 100 - 2 * u16::from(n.min(ticks)));
            assert_eq!(
                target.statuses.first().map(|s| s.remaining),
                if n < ticks { Some(ticks - n) } else { None }
            );
            game.apply(ActorId(101), CombatAction::Wait)
                .expect("consume owner turn");
        }
    }
}

#[test]
fn weakened_expires_at_owner_end_and_restores_power() {
    for (resilient, ticks) in [(false, 2u8), (true, 1)] {
        let (catalog, input) = status_fixture(StatusKind::Weakened, resilient);
        let mut game = combat(&catalog, &input);
        let command = action(&game, "status_probe", 101);
        commit(&mut game, command);
        for n in 1..=ticks {
            wait_for(&mut game, ActorId(101));
            assert_eq!(
                game.snapshot()
                    .actor(ActorId(101))
                    .expect("target")
                    .modifier(Stat::OutgoingDamage),
                -2
            );
            game.apply(ActorId(101), CombatAction::Wait)
                .expect("owner end");
            assert_eq!(
                game.snapshot()
                    .actor(ActorId(101))
                    .expect("target")
                    .modifier(Stat::OutgoingDamage),
                if n == ticks { 0 } else { -2 }
            );
        }
    }
}

#[test]
fn haste_keeps_current_order_and_expires_after_two_round_ends_even_with_resilient() {
    let (catalog, input) = status_fixture(StatusKind::Haste, true);
    let mut game = combat(&catalog, &input);
    let before = game
        .snapshot()
        .initiative
        .iter()
        .map(|i| i.actor)
        .collect::<Vec<_>>();
    let command = action(&game, "status_probe", 101);
    commit(&mut game, command);
    assert_eq!(
        game.snapshot()
            .initiative
            .iter()
            .map(|i| i.actor)
            .collect::<Vec<_>>(),
        before
    );
    for round in 2..=3 {
        wait_for(&mut game, ActorId(1));
        let state = game.snapshot();
        assert_eq!(state.round, round);
        assert_eq!(
            state
                .actor(ActorId(101))
                .expect("target")
                .modifier(Stat::Speed),
            if round == 2 { 3 } else { 0 }
        );
        if round == 2 {
            game.apply(ActorId(1), CombatAction::Wait)
                .expect("continue");
        }
    }
}

#[test]
fn resilient_default_starting_clocks_shorten_but_explicit_remaining_and_snapshot_do_not() {
    for (kind, default, explicit) in [
        (StatusKind::Bleed, 2, 3),
        (StatusKind::Weakened, 1, 2),
        (StatusKind::Haste, 2, 2),
        (StatusKind::Brace, 1, 1),
    ] {
        for remaining in [None, Some(1), Some(explicit)] {
            let (catalog, mut input) = passive_fixture(&[], true);
            input
                .enemies
                .first_mut()
                .expect("target")
                .starting_statuses
                .push(StartingStatus {
                    kind,
                    source: Some(ActorId(1)),
                    remaining,
                });
            let game = combat(&catalog, &input);
            let state = game.snapshot();
            assert_eq!(
                state
                    .actor(ActorId(101))
                    .expect("target")
                    .statuses
                    .first()
                    .expect("status")
                    .remaining,
                remaining.unwrap_or(default)
            );
            let restored: labyrinth_rules::CombatSnapshot =
                serde_json::from_str(&serde_json::to_string(&state).expect("serialize"))
                    .expect("restore");
            assert_eq!(restored, state);
        }
    }
}

#[test]
fn refresh_preserves_one_instance_and_potency_and_updates_source_and_duration() {
    let (catalog, mut input) = status_fixture(StatusKind::Bleed, true);
    input
        .enemies
        .first_mut()
        .expect("target")
        .starting_statuses
        .push(StartingStatus {
            kind: StatusKind::Bleed,
            source: Some(ActorId(2)),
            remaining: Some(1),
        });
    let mut game = combat(&catalog, &input);
    let old_id = game
        .snapshot()
        .actor(ActorId(101))
        .expect("target")
        .statuses
        .first()
        .expect("bleed")
        .id;
    let command = action(&game, "status_probe", 101);
    commit(&mut game, command);
    let state = game.snapshot();
    let statuses = &state.actor(ActorId(101)).expect("target").statuses;
    assert_eq!(statuses.len(), 1);
    let status = statuses.first().expect("refreshed");
    assert_eq!(
        (status.id, status.potency, status.remaining, status.source),
        (old_id, 2, 2, ActorId(1))
    );
}

#[test]
fn distinct_reducers_add_with_minimum_one_tick() {
    let (catalog, mut input) = status_fixture(StatusKind::Bleed, true);
    let mut raw = catalog.definition().clone();
    let mut second = raw
        .abilities
        .iter()
        .find(|a| a.id == id("resilient"))
        .expect("passive")
        .clone();
    second.id = id("extra_resilience");
    raw.abilities.push(second);
    input
        .enemies
        .first_mut()
        .expect("target")
        .actor
        .build
        .abilities
        .push(id("extra_resilience"));
    let catalog = ContentCatalog::new(raw).expect("distinct passive fixture");
    let mut game = combat(&catalog, &input);
    let command = action(&game, "status_probe", 101);
    commit(&mut game, command);
    assert_eq!(
        game.snapshot()
            .actor(ActorId(101))
            .expect("target")
            .statuses
            .first()
            .expect("bleed")
            .remaining,
        1
    );
}

#[test]
fn exact_item_and_kind_requirements_govern_runtime_passives() {
    for (weapon, duration) in [(None, 3), (Some("staff"), 3), (Some("medic_staff"), 2)] {
        let (catalog, mut input) = status_fixture(StatusKind::Bleed, true);
        let mut raw = catalog.definition().clone();
        raw.abilities
            .iter_mut()
            .find(|a| a.id == id("resilient"))
            .expect("passive")
            .requirements = vec![
            EquipmentRequirement::Kind(id("staff")),
            EquipmentRequirement::Item(id("medic_staff")),
        ];
        let catalog = ContentCatalog::new(raw).expect("exact item requirement");
        input
            .enemies
            .first_mut()
            .expect("target")
            .actor
            .build
            .weapon = weapon.map(id);
        let mut game = combat(&catalog, &input);
        let command = action(&game, "status_probe", 101);
        commit(&mut game, command);
        assert_eq!(
            game.snapshot()
                .actor(ActorId(101))
                .expect("target")
                .statuses
                .first()
                .expect("bleed")
                .remaining,
            duration
        );
    }
}

#[test]
fn authored_rank_upgrades_extend_actual_legality_and_execute_from_added_rank() {
    let (catalog, mut input) = scenario(Some("dagger"), &[], 6, &[1, 1, 1, 1, 1, 1]);
    input
        .heroes
        .iter_mut()
        .find(|a| a.id == ActorId(1))
        .expect("source")
        .actor
        .build
        .abilities
        .push(id("duelist_dagger_power"));
    let mut raw = catalog.definition().clone();
    raw.abilities
        .iter_mut()
        .find(|a| a.id == id("duelist_dagger_power"))
        .expect("passive")
        .upgrades
        .first_mut()
        .expect("upgrade")
        .operations
        .extend([
            UpgradeOperation::ExtendSourceRanks(32),
            UpgradeOperation::ExtendTargetRanks(32),
        ]);
    let catalog = ContentCatalog::new(raw).expect("rank extension");
    let mut game = combat(&catalog, &input);
    let command = action(&game, "dagger_stab", 106);
    commit(&mut game, command);
    assert_eq!(game.snapshot().actor(ActorId(106)).expect("target").hp, 93);
}
