#![expect(
    clippy::unwrap_used,
    clippy::indexing_slicing,
    reason = "tests use bounded authored fixtures and assert validation"
)]
use super::*;
use crate::{
    build::{ActorBuild, CharacterBuild, SkillGrant},
    catalog::{ContentCatalog, ContentId},
    scenario::{ControllerPolicy, Scenario, ScenarioActor, StockScenario, SCENARIO_SCHEMA_VERSION},
    LifeState, SkillId,
};
fn id(key: &str) -> ContentId {
    ContentId::new(key).unwrap()
}
fn actor(catalog: &ContentCatalog, n: u16, hero: bool) -> ScenarioActor {
    ScenarioActor {
        id: ActorId(n),
        actor: ActorBuild::from_preset(catalog, &id(if hero { "gatekeeper" } else { "ash_brute" }))
            .unwrap(),
        controller: if hero {
            ControllerPolicy::Manual
        } else {
            ControllerPolicy::Ai
        },
        starting_hp: None,
        starting_statuses: vec![],
    }
}
fn fixture(weapon: &str, widths: &[u8]) -> (ContentCatalog, Scenario) {
    let catalog = ContentCatalog::builtin().unwrap();
    let mut hero = actor(&catalog, 1, true);
    hero.actor.base_speed = 100;
    hero.actor.max_hp = 100;
    hero.actor.build = CharacterBuild {
        weapon: Some(id(weapon)),
        ..Default::default()
    };
    let enemies = widths
        .iter()
        .enumerate()
        .map(|(i, width)| {
            let mut e = actor(&catalog, 101 + i as u16, false);
            e.actor.max_hp = 100;
            e.actor.footprint = *width;
            e
        })
        .collect();
    (
        catalog,
        Scenario {
            schema_version: SCENARIO_SCHEMA_VERSION,
            name: "Execution fixture".into(),
            seed: 91,
            heroes: vec![hero],
            enemies,
        },
    )
}
fn ability_action(combat: &Combat, key: &str, target: u16) -> CombatAction {
    let index = combat
        .state
        .actor(ActorId(1))
        .unwrap()
        .skill_index(&id(key))
        .unwrap();
    CombatAction::Skill {
        index,
        target: ActorId(target),
    }
}
#[test]
fn all_stock_scenarios_roundtrip_and_replay_identical_actions() {
    let catalog = ContentCatalog::builtin().unwrap();
    for choice in StockScenario::ALL {
        for seed in [42, 91] {
            let scenario = Scenario::stock(choice, seed, &catalog).unwrap();
            let copy = Scenario::from_json(&scenario.to_json().unwrap(), &catalog).unwrap();
            assert_eq!(copy, scenario);
            let mut left = Combat::from_scenario(&catalog, &scenario).unwrap();
            let mut right = Combat::from_scenario(&catalog, &copy).unwrap();
            assert_eq!(left, right);
            for _ in 0..30 {
                let Some(actor) = left.state.active_actor else {
                    break;
                };
                let action = left
                    .ai_action()
                    .unwrap_or_else(|| left.legal_actions(actor)[0]);
                assert_eq!(
                    left.apply(actor, action).unwrap(),
                    right.apply(actor, action).unwrap()
                );
                assert_eq!(left, right);
            }
            let bytes = serde_json::to_vec(&left.snapshot()).unwrap();
            let restored: CombatSnapshot = serde_json::from_slice(&bytes).unwrap();
            assert_eq!(restored, left.snapshot());
        }
    }
}
#[test]
fn every_weapon_move_executes_and_uses_shared_preview() {
    let catalog = ContentCatalog::builtin().unwrap();
    for weapon in &catalog.definition().weapons {
        for key in &weapon.skills {
            let definition = catalog.skill(key).unwrap();
            let (catalog, mut scenario) = fixture(weapon.id.as_str(), &[1, 1, 1, 1, 1, 1]);
            let source_rank = (1..=6).find(|r| definition.allows_source_rank(*r)).unwrap();
            for n in 1..source_rank {
                let mut filler = actor(&catalog, n as u16 + 1, true);
                filler.actor.build = CharacterBuild::default();
                scenario.heroes.insert(0, filler);
            }
            let target_rank = (1..=6).find(|r| definition.allows_target_rank(*r)).unwrap();
            let mut combat = Combat::from_scenario(&catalog, &scenario).unwrap();
            let action = ability_action(&combat, key.as_str(), 100 + target_rank as u16);
            let original = combat.clone();
            let preview = combat
                .snapshot()
                .preview_action(ActorId(1), &action)
                .unwrap();
            assert_eq!(combat, original);
            let events = combat.apply(ActorId(1), action).unwrap();
            assert!(
                events
                    .iter()
                    .any(|e| matches!(e.kind, CombatEventKind::Damage { .. })),
                "{}",
                key
            );
            for predicted in &preview.damage {
                assert!(events.iter().any(|event|matches!(event.kind,CombatEventKind::Damage{source,target,amount,kind} if source==predicted.source && target==predicted.target && amount==predicted.hp_loss && kind==predicted.kind)),"{}",key);
            }
        }
    }
}
#[test]
fn cleave_captures_two_distinct_actors_and_one_large_actor_once() {
    for (widths, expected) in [
        (vec![1, 1, 1], vec![ActorId(101), ActorId(102)]),
        (vec![2, 1], vec![ActorId(101)]),
        (vec![1], vec![ActorId(101)]),
    ] {
        let (catalog, scenario) = fixture("greatsword", &widths);
        let mut combat = Combat::from_scenario(&catalog, &scenario).unwrap();
        let action = ability_action(&combat, "greatsword_cleave", 101);
        let before = combat.clone();
        let preview = combat
            .snapshot()
            .preview_action(ActorId(1), &action)
            .unwrap();
        assert_eq!(
            preview.damage.iter().map(|d| d.target).collect::<Vec<_>>(),
            expected
        );
        assert_eq!(combat, before);
        let events = combat.apply(ActorId(1), action).unwrap();
        assert_eq!(
            events
                .iter()
                .filter_map(|e| if let CombatEventKind::Damage { target, .. } = e.kind {
                    Some(target)
                } else {
                    None
                })
                .collect::<Vec<_>>(),
            expected
        );
    }
}
#[test]
fn cleave_does_not_retarget_after_corpse_compaction_or_drop_terminal_targets() {
    let (catalog, scenario) = fixture("greatsword", &[1, 1, 1]);
    let mut combat = Combat::from_scenario(&catalog, &scenario).unwrap();
    let first = combat.actor_mut(ActorId(101)).unwrap();
    first.hp = 0;
    first.life = LifeState::Corpse {
        hp: 1,
        max_hp: 25,
        created_round: 1,
    };
    let action = ability_action(&combat, "greatsword_cleave", 101);
    let preview = combat
        .snapshot()
        .preview_action(ActorId(1), &action)
        .unwrap();
    assert_eq!(
        preview.damage.iter().map(|d| d.target).collect::<Vec<_>>(),
        [ActorId(101), ActorId(102)]
    );
    combat.apply(ActorId(1), action).unwrap();
    assert_eq!(
        combat.state.actor(ActorId(101)).unwrap().life,
        LifeState::Removed
    );
    assert_eq!(combat.state.actor(ActorId(102)).unwrap().hp, 94);
    assert_eq!(combat.state.actor(ActorId(103)).unwrap().hp, 100);
    assert_eq!(combat.state.rank(ActorId(103)), Some(2));

    let (catalog, mut scenario) = fixture("greatsword", &[1, 1]);
    scenario.enemies[0].starting_hp = Some(1);
    let mut combat = Combat::from_scenario(&catalog, &scenario).unwrap();
    let second = combat.actor_mut(ActorId(102)).unwrap();
    second.hp = 0;
    second.life = LifeState::Corpse {
        hp: 1,
        max_hp: 25,
        created_round: 1,
    };
    let action = ability_action(&combat, "greatsword_cleave", 101);
    let preview = combat
        .snapshot()
        .preview_action(ActorId(1), &action)
        .unwrap();
    assert_eq!(preview.damage.len(), 2);
    let events = combat.apply(ActorId(1), action).unwrap();
    let removed = events
        .iter()
        .position(|e| {
            matches!(
                e.kind,
                CombatEventKind::CorpseRemoved {
                    actor: ActorId(102),
                    ..
                }
            )
        })
        .unwrap();
    let finished = events
        .iter()
        .position(|e| matches!(e.kind, CombatEventKind::Finished { .. }))
        .unwrap();
    assert!(removed < finished);
    assert_eq!(combat.state.outcome, Some(CombatOutcome::Victory));
    assert_eq!(
        combat.state.actor(ActorId(102)).unwrap().life,
        LifeState::Removed
    );
}
#[test]
fn personal_skills_passive_upgrades_and_legacy_alias_share_one_use_counter() {
    let (catalog, mut scenario) = fixture("dagger", &[1]);
    scenario.heroes[0].actor.build.skills.push(SkillGrant {
        skill: id("assassin_feint"),
        provenance: id("assassin"),
    });
    scenario.heroes[0].actor.build.abilities =
        vec![id("assassin_bleeding_dagger"), id("duelist_dagger_power")];
    let mut combat = Combat::from_scenario(&catalog, &scenario).unwrap();
    let action = ability_action(&combat, "dagger_stab", 101);
    assert_eq!(
        combat
            .state
            .actor(ActorId(1))
            .unwrap()
            .resolved_skills()
            .len(),
        3
    );
    let preview = combat
        .snapshot()
        .preview_action(ActorId(1), &action)
        .unwrap();
    assert_eq!(preview.damage[0].base, 7);
    combat.apply(ActorId(1), action).unwrap();
    assert!(combat
        .state
        .actor(ActorId(101))
        .unwrap()
        .statuses
        .iter()
        .any(|s| s.kind == StatusKind::Bleed));
    assert!(combat
        .state
        .actor(ActorId(1))
        .unwrap()
        .skill_uses
        .is_empty());

    let mut raw = catalog.definition().clone();
    raw.abilities.push(crate::catalog::AbilityDefinition {
        id: id("legacy_training"),
        name: "Legacy training".into(),
        description: "Adds damage to the granted legacy move.".into(),
        provenance: id("test"),
        personal_selectable: true,
        requirements: vec![],
        effects: vec![],
        upgrades: vec![crate::catalog::SkillUpgrade {
            skill: id("front_strike"),
            operations: vec![crate::catalog::UpgradeOperation::AddDamage {
                effect_index: 0,
                amount: 3,
            }],
        }],
    });
    raw.skills
        .iter_mut()
        .find(|a| a.id == id("front_strike"))
        .unwrap()
        .max_uses = Some(2);
    let catalog = ContentCatalog::new(raw).unwrap();
    let catalog = crate::scenario::legacy_catalog(&catalog, [&[SkillId::FrontStrike][..]]).unwrap();
    scenario.heroes[0].actor.build = crate::scenario::legacy_build(&[SkillId::FrontStrike]);
    scenario.heroes[0].actor.build.abilities = vec![id("legacy_training")];
    let mut alias = Combat::from_scenario(&catalog, &scenario).unwrap();
    let mut indexed = alias.clone();
    alias
        .apply(
            ActorId(1),
            CombatAction::LegacySkill {
                skill: SkillId::FrontStrike,
                target: ActorId(101),
            },
        )
        .unwrap();
    indexed
        .apply(
            ActorId(1),
            CombatAction::Skill {
                index: 0,
                target: ActorId(101),
            },
        )
        .unwrap();
    assert_eq!(alias.snapshot(), indexed.snapshot());
    assert_eq!(alias.state.actor(ActorId(101)).unwrap().hp, 90);
    assert_eq!(
        alias
            .state
            .actor(ActorId(1))
            .unwrap()
            .remaining_uses(SkillId::FrontStrike),
        Some(1)
    );
}
#[test]
fn arbitrary_authored_ids_beyond_eight_execute_and_external_policy_uses_same_seam() {
    let (catalog, mut scenario) = fixture("dagger", &[1]);
    let mut raw = catalog.definition().clone();
    for n in 0..12 {
        let mut definition = catalog.skill(&id("dagger_stab")).unwrap().clone();
        definition.id = id(&format!("new_move_{n}"));
        definition.personal_selectable = true;
        definition.requirements.clear();
        definition.source_ranks = 63;
        definition.target_ranks = 63;
        definition.effects = vec![Effect::Damage(n + 1)];
        raw.skills.push(definition);
    }
    let catalog = ContentCatalog::new(raw).unwrap();
    scenario.heroes[0].actor.build = CharacterBuild {
        skills: (0..12)
            .map(|n| SkillGrant {
                skill: id(&format!("new_move_{n}")),
                provenance: id("test"),
            })
            .collect(),
        ..Default::default()
    };
    scenario.heroes[0].controller = ControllerPolicy::External;
    let mut combat = Combat::from_scenario(&catalog, &scenario).unwrap();
    assert_eq!(combat.ai_action(), None);
    for index in 0..12 {
        assert!(combat
            .legal_actions(ActorId(1))
            .contains(&CombatAction::Skill {
                index,
                target: ActorId(101)
            }));
    }
    combat
        .apply(
            ActorId(1),
            CombatAction::Skill {
                index: 11,
                target: ActorId(101),
            },
        )
        .unwrap();
    assert_eq!(combat.state.actor(ActorId(101)).unwrap().hp, 88);
    scenario.heroes[0].controller = ControllerPolicy::Ai;
    let mut combat = Combat::from_scenario(&catalog, &scenario).unwrap();
    let decision = combat.ai_action().unwrap();
    assert!(combat.legal_actions(ActorId(1)).contains(&decision));
    combat.apply(ActorId(1), decision).unwrap();
}
#[test]
fn scenario_rejections_and_invalid_ability_commands_leave_authority_unchanged() {
    let (catalog, scenario) = fixture("greatsword", &[1]);
    let mut invalid = scenario.clone();
    invalid.enemies[0].id = ActorId(1);
    assert!(invalid.validate(&catalog).is_err());
    let mut invalid = scenario.clone();
    invalid.heroes[0].actor.footprint = 6;
    invalid.heroes.push(actor(&catalog, 2, true));
    assert!(invalid.validate(&catalog).is_err());
    let mut invalid = scenario.clone();
    invalid.enemies[0].starting_hp = Some(101);
    assert!(invalid.validate(&catalog).is_err());
    let mut invalid = scenario.clone();
    invalid.enemies[0].actor.max_hp = 0;
    assert!(invalid.validate(&catalog).is_err());
    let mut invalid = scenario.clone();
    invalid.heroes[0].starting_hp = Some(0);
    assert!(invalid.validate(&catalog).is_err());
    let mut combat = Combat::from_scenario(&catalog, &scenario).unwrap();
    let previous = combat.clone();
    for command in [
        CombatAction::Skill {
            index: 63,
            target: ActorId(101),
        },
        CombatAction::Skill {
            index: 0,
            target: ActorId(1),
        },
        CombatAction::Skill {
            index: 0,
            target: ActorId(999),
        },
    ] {
        assert!(combat.apply(ActorId(1), command).is_err());
        assert_eq!(combat, previous);
    }
    let action = ability_action(&combat, "greatsword_cleave", 101);
    assert_eq!(
        combat.apply_with_budget(ActorId(1), action, 1),
        Err(RuleError::WorkLimit)
    );
    assert_eq!(combat, previous);
    let mut wire = serde_json::to_value(combat.snapshot()).unwrap();
    wire["actors"][0]["resolved_build"]["moveset"]["skills"][0]["definition"]["effects"] =
        serde_json::json!([{"Damage":999}]);
    assert!(serde_json::from_value::<CombatSnapshot>(wire).is_err());
}
#[test]
fn stock_snapshot_sizes_are_measured_and_catalog_freezing_ignores_later_edits() {
    let catalog = ContentCatalog::builtin().unwrap();
    let mut sizes = Vec::new();
    for choice in StockScenario::ALL {
        let scenario = Scenario::stock(choice, 42, &catalog).unwrap();
        let combat = Combat::from_scenario(&catalog, &scenario).unwrap();
        let bytes = serde_json::to_vec(&combat.snapshot()).unwrap();
        sizes.push((format!("{choice:?}"), bytes.len()));
    }
    let mut raw = catalog.definition().clone();
    let template = catalog.skill(&id("dagger_stab")).unwrap();
    for n in 0..64 {
        let mut skill = template.clone();
        skill.id = id(&format!("capacity_{n}"));
        skill.personal_selectable = true;
        skill.requirements.clear();
        raw.skills.push(skill);
    }
    let full_catalog = ContentCatalog::new(raw).unwrap();
    let mut full = Scenario::stock(StockScenario::WeaponComparison, 42, &full_catalog).unwrap();
    let build = CharacterBuild {
        skills: (0..64)
            .map(|n| SkillGrant {
                skill: id(&format!("capacity_{n}")),
                provenance: id("test"),
            })
            .collect(),
        ..Default::default()
    };
    for actor in full.heroes.iter_mut().chain(&mut full.enemies) {
        actor.actor.build = build.clone();
    }
    let full_combat = Combat::from_scenario(&full_catalog, &full).unwrap();
    let full_size = serde_json::to_vec(&full_combat.snapshot()).unwrap().len();
    assert!(full_size < crate::MAX_COMBAT_SNAPSHOT_BYTES);
    assert!(full_combat
        .state
        .actors
        .iter()
        .all(|a| a.resolved_skills().len() == 64));
    sizes.push(("TwelveActorsSixtyFourMoves".into(), full_size));
    let mut oversized = full_catalog.definition().clone();
    for skill in &mut oversized.skills {
        if skill.id.as_str().starts_with("capacity_") {
            skill.description = "x".repeat(2048);
        }
    }
    let oversized = ContentCatalog::new(oversized).unwrap();
    assert_eq!(
        full.validate(&oversized).unwrap_err().path,
        "scenario.payload"
    );

    let review = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../../target/review");
    std::fs::create_dir_all(&review).unwrap();
    std::fs::write(
        review.join("runtime-wire-sizes.json"),
        serde_json::to_vec_pretty(&sizes).unwrap(),
    )
    .unwrap();
    let (catalog, scenario) = fixture("greatsword", &[1]);
    let mut combat = Combat::from_scenario(&catalog, &scenario).unwrap();
    let mut raw = catalog.definition().clone();
    raw.skills
        .iter_mut()
        .find(|a| a.id == id("greatsword_cleave"))
        .unwrap()
        .effects = vec![Effect::Damage(50)];
    let edited = ContentCatalog::new(raw).unwrap();
    assert_ne!(catalog.fingerprint(), edited.fingerprint());
    let action = ability_action(&combat, "greatsword_cleave", 101);
    combat.apply(ActorId(1), action).unwrap();
    assert_eq!(combat.state.actor(ActorId(101)).unwrap().hp, 94);
}

#[test]
fn resilient_application_refresh_and_preview_use_one_runtime_rule() {
    let (catalog, mut scenario) = fixture("dagger", &[1]);
    scenario.heroes[0].actor.build.abilities = vec![id("assassin_bleeding_dagger")];
    scenario.enemies[0].actor.build.abilities = vec![id("resilient")];
    let mut combat = Combat::from_scenario(&catalog, &scenario).unwrap();
    let action = ability_action(&combat, "dagger_stab", 101);
    let before = combat.clone();
    let preview = combat
        .snapshot()
        .preview_action(ActorId(1), &action)
        .unwrap();
    assert_eq!(combat, before);
    assert!(preview.events.iter().any(|event| matches!(event, crate::PreviewEvent::StatusApplied(status) if status.kind == StatusKind::Bleed && status.remaining == 2)));
    let applied = combat.apply(ActorId(1), action).unwrap();
    assert!(applied.iter().any(|event| matches!(&event.kind, CombatEventKind::StatusApplied { instance } if instance.kind == StatusKind::Bleed && instance.remaining == 2)));
    let old_id = combat.state.actor(ActorId(101)).unwrap().statuses[0].id;
    let mut refreshed = vec![];
    combat
        .add_status(
            ActorId(1),
            ActorId(101),
            StatusKind::Bleed,
            &mut refreshed,
            &mut Work(MAX_WORK),
        )
        .unwrap();
    assert!(refreshed.iter().any(|event| matches!(&event.kind, CombatEventKind::StatusRefreshed { instance } if instance.id == old_id && instance.remaining == 2)));
    let snapshot = combat.snapshot();
    let restored: CombatSnapshot =
        serde_json::from_slice(&serde_json::to_vec(&snapshot).unwrap()).unwrap();
    assert_eq!(restored, snapshot);
    assert_eq!(
        restored.actor(ActorId(101)).unwrap().statuses[0].remaining,
        2
    );
    assert!(combat
        .legal_actions(combat.state.active_actor.unwrap())
        .iter()
        .all(|action| !matches!(action, CombatAction::LegacySkill { .. })));
}

#[test]
fn resilient_starting_conditions_shorten_defaults_but_preserve_explicit_remaining() {
    for (remaining, expected) in [(None, 2), (Some(2), 2), (Some(1), 1)] {
        let (catalog, mut scenario) = fixture("dagger", &[1]);
        scenario.enemies[0].actor.build.abilities = vec![id("resilient")];
        scenario.enemies[0].starting_statuses = vec![crate::scenario::StartingStatus {
            kind: StatusKind::Bleed,
            source: Some(ActorId(1)),
            remaining,
        }];
        let encoded = scenario.to_json().unwrap();
        let restored = Scenario::from_json(&encoded, &catalog).unwrap();
        let combat = Combat::from_scenario(&catalog, &restored).unwrap();
        assert_eq!(combat.state.active_actor, Some(ActorId(1)));
        assert_eq!(
            combat.state.actor(ActorId(101)).unwrap().statuses[0].remaining,
            expected
        );
        let snapshot = combat.snapshot();
        let roundtrip: CombatSnapshot =
            serde_json::from_slice(&serde_json::to_vec(&snapshot).unwrap()).unwrap();
        assert_eq!(snapshot, roundtrip);
    }
}
