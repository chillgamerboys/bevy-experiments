use crate::mechanics_support::*;
use labyrinth_rules::{
    build::SkillGrant,
    catalog::{ContentCatalog, EquipmentRequirement},
    ActorId, CombatAction,
};

#[test]
fn all_eleven_items_grant_exact_ordered_moves_without_passive_actions() {
    let cases = [
        ("dagger", vec!["dagger_stab", "dagger_throw"]),
        ("greatsword", vec!["greatsword_cleave", "greatsword_thrust"]),
        ("two_handed_axe", vec!["axe_chop"]),
        ("spear", vec!["spear_thrust", "spear_shove"]),
        ("bow", vec!["bow_aimed_shot", "bow_quick_shot"]),
        ("staff", vec!["staff_blow", "staff_push"]),
        (
            "gatekeeper_spear",
            vec!["front_strike", "long_reach", "driving_blow"],
        ),
        (
            "knifehand_daggers",
            vec!["bleeding_cut", "deep_strike", "thrown_knife"],
        ),
        (
            "scout_bow",
            vec!["back_rank_shot", "snap_shot", "hook_shot"],
        ),
        ("medic_staff", vec!["staff_strike"]),
        ("hollow_bow", vec!["hollow_bolt"]),
    ];
    assert_eq!(
        ContentCatalog::builtin()
            .expect("catalog")
            .definition()
            .weapons
            .len(),
        cases.len()
    );
    for (weapon, expected) in cases {
        let (catalog, input) = scenario(Some(weapon), &[], 1, &[1]);
        let game = combat(&catalog, &input);
        let snapshot = game.snapshot();
        let source = snapshot.actor(ActorId(1)).expect("source");
        assert_eq!(
            source
                .moveset()
                .skills
                .iter()
                .map(|s| s.definition.id.as_str())
                .collect::<Vec<_>>(),
            expected
        );
        assert!(source.moveset().skills.iter().all(|s| s.grants.len() == 1));
    }
}

#[test]
fn personal_kind_and_exact_item_requirements_reactivate_without_changing_selection() {
    for exact in [false, true] {
        for weapon in [
            None,
            Some("staff"),
            Some("knifehand_daggers"),
            Some("dagger"),
        ] {
            let (catalog, mut input) = scenario(weapon, &["assassin_feint"], 1, &[1]);
            let mut raw = catalog.definition().clone();
            if exact {
                raw.skills
                    .iter_mut()
                    .find(|s| s.id == id("assassin_feint"))
                    .expect("Skill")
                    .requirements
                    .push(EquipmentRequirement::Item(id("dagger")));
            }
            let catalog = ContentCatalog::new(raw).expect("requirements");
            let mut game = combat(&catalog, &input);
            let state = game.snapshot();
            let source = state.actor(ActorId(1)).expect("source");
            let eligible =
                weapon == Some("dagger") || (!exact && weapon == Some("knifehand_daggers"));
            assert_eq!(
                source.skill_index(&id("assassin_feint")).is_some(),
                eligible
            );
            assert_eq!(source.build.skills.len(), 1);
            if eligible {
                let command = action(&game, "assassin_feint", 101);
                commit(&mut game, command);
                assert_eq!(game.snapshot().actor(ActorId(101)).expect("target").hp, 96);
            }
            input.heroes.first_mut().expect("source").actor.build.weapon = Some(id("dagger"));
            let mut restored = combat(&catalog, &input);
            let command = action(&restored, "assassin_feint", 101);
            commit(&mut restored, command);
            assert_eq!(
                restored.snapshot().actor(ActorId(101)).expect("target").hp,
                96
            );
        }
    }
}

#[test]
fn duplicate_skill_sources_preserve_provenance_without_duplicate_execution() {
    let (catalog, input) = scenario(Some("dagger"), &["hurled_scrap"], 1, &[1]);
    let mut raw = catalog.definition().clone();
    raw.weapons
        .iter_mut()
        .find(|w| w.id == id("dagger"))
        .expect("weapon")
        .skills
        .push(id("hurled_scrap"));
    let catalog = ContentCatalog::new(raw).expect("two grant sources");
    let mut game = combat(&catalog, &input);
    let state = game.snapshot();
    let source = state.actor(ActorId(1)).expect("source");
    let skill = source.moveset().skills.first().expect("personal first");
    assert_eq!(skill.definition.id, id("hurled_scrap"));
    assert_eq!(skill.grants.len(), 2);
    assert_eq!(source.moveset().skills.len(), 3);
    let command = action(&game, "hurled_scrap", 101);
    commit(&mut game, command);
    assert_eq!(game.snapshot().actor(ActorId(101)).expect("target").hp, 98);
}

#[test]
fn empty_moveset_still_has_universal_actions_and_off_turn_commands_reject() {
    let (catalog, input) = scenario(None, &[], 1, &[1]);
    let mut game = combat(&catalog, &input);
    assert!(game
        .snapshot()
        .actor(ActorId(1))
        .expect("source")
        .moveset()
        .skills
        .is_empty());
    assert!(game
        .legal_actions(ActorId(1))
        .contains(&CombatAction::Defend));
    commit(&mut game, CombatAction::Wait);
    reject(&mut game, CombatAction::Wait);
}

#[test]
fn catalog_edits_cannot_thaw_existing_movesets() {
    let (catalog, input) = scenario(Some("dagger"), &[], 1, &[1]);
    let mut game = combat(&catalog, &input);
    let mut raw = catalog.definition().clone();
    raw.skills
        .iter_mut()
        .find(|s| s.id == id("dagger_stab"))
        .expect("Skill")
        .effects = vec![labyrinth_rules::Effect::Damage(20)];
    let changed = ContentCatalog::new(raw).expect("new encounter catalog");
    let command = action(&game, "dagger_stab", 101);
    commit(&mut game, command);
    assert_eq!(game.snapshot().actor(ActorId(101)).expect("target").hp, 95);
    let mut next = combat(&changed, &input);
    let command = action(&next, "dagger_stab", 101);
    commit(&mut next, command);
    assert_eq!(next.snapshot().actor(ActorId(101)).expect("target").hp, 80);
}

#[test]
fn malformed_personal_grants_and_inactive_passive_actions_reject() {
    let (catalog, mut input) = scenario(None, &["assassin_feint"], 1, &[1]);
    let mut game = combat(&catalog, &input);
    reject(
        &mut game,
        CombatAction::Skill {
            index: 0,
            target: ActorId(101),
        },
    );
    input
        .heroes
        .first_mut()
        .expect("source")
        .actor
        .build
        .skills
        .push(SkillGrant {
            skill: id("resilient"),
            provenance: id("forged"),
        });
    assert!(
        input.validate(&catalog).is_err(),
        "an Ability is not an active Skill"
    );
}
