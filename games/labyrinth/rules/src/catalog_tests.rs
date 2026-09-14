#![expect(
    clippy::unwrap_used,
    clippy::indexing_slicing,
    reason = "test fixtures assert validated catalog contents"
)]

use crate::{build::*, catalog::*, legacy_skill_definition, ActorKind, Effect, HeroClass, SkillId};

fn id(value: &str) -> ContentId {
    ContentId::new(value).unwrap()
}
fn catalog() -> ContentCatalog {
    ContentCatalog::builtin().unwrap()
}
fn dagger() -> CharacterBuild {
    CharacterBuild {
        weapon: Some(id("dagger")),
        ..Default::default()
    }
}
fn snake(value: &str) -> String {
    let mut text = String::new();
    for (i, c) in value.chars().enumerate() {
        if c.is_uppercase() && i != 0 {
            text.push('_');
        }
        text.extend(c.to_lowercase());
    }
    text
}

#[test]
fn embedded_data_preserves_all_legacy_abilities_and_actor_defaults() {
    let catalog = catalog();
    for skill in SkillId::ALL {
        let legacy = legacy_skill_definition(skill);
        let key = snake(serde_json::to_value(skill).unwrap().as_str().unwrap());
        let authored = catalog.skill(&id(&key)).unwrap();
        assert_eq!(authored.name, legacy.name);
        assert_eq!(authored.description, legacy.description);
        assert_eq!(authored.source_ranks, legacy.source_ranks);
        assert_eq!(authored.target_ranks, legacy.target_ranks);
        assert_eq!(authored.target_rule, legacy.target_rule);
        assert_eq!(authored.effects, legacy.effects);
        assert_eq!(authored.max_uses, legacy.max_uses);
    }
    for kind in HeroClass::ALL
        .into_iter()
        .map(ActorKind::Hero)
        .chain(crate::EnemyKind::ALL.into_iter().map(ActorKind::Enemy))
    {
        let preset = catalog
            .definition()
            .actor_presets
            .iter()
            .find(|p| p.appearance == kind)
            .unwrap();
        assert_eq!((preset.max_hp, preset.base_speed), kind.stats());
        assert_eq!(preset.footprint, kind.footprint());
        let resolved = catalog.resolve_build(&preset.build).unwrap();
        assert_eq!(resolved.moveset.skills.len(), crate::skills_for(kind).len());
    }
}
#[test]
fn data_only_new_weapon_and_ability_have_no_enum_or_consumer_mapping() {
    let mut raw = catalog().definition().clone();
    let mut skill = raw.skills[0].clone();
    skill.id = id("test_lance_pierce");
    skill.requirements.clear();
    skill.effects = vec![Effect::Damage(19)];
    skill.source_ranks = 63;
    skill.target_ranks = 15;
    raw.skills.push(skill);
    raw.weapons.push(WeaponDefinition {
        id: id("test_lance"),
        name: "Test Lance".into(),
        description: "New data-only weapon.".into(),
        handedness: Handedness::Two,
        kind: id("spear"),
        abilities: vec![],
        skills: vec![id("test_lance_pierce")],
    });
    let encoded = serde_json::to_string(&raw).unwrap();
    let catalog: ContentCatalog = serde_json::from_str(&encoded).unwrap();
    let resolved = catalog
        .resolve_build(&CharacterBuild {
            weapon: Some(id("test_lance")),
            ..Default::default()
        })
        .unwrap();
    assert_eq!(
        resolved.moveset.skills[0].definition.effects,
        vec![Effect::Damage(19)]
    );
    assert_eq!(resolved.moveset.skills[0].definition.target_ranks, 15);
}
#[test]
fn all_six_weapons_resolve_and_throw_is_unlimited() {
    let catalog = catalog();
    assert_eq!(catalog.definition().weapons.len(), 11);
    for weapon in &catalog.definition().weapons {
        let resolved = catalog
            .resolve_build(&CharacterBuild {
                weapon: Some(weapon.id.clone()),
                ..Default::default()
            })
            .unwrap();
        assert!(!resolved.moveset.skills.is_empty());
        assert!(resolved
            .moveset
            .skills
            .iter()
            .all(|a| a.grants[0].kind == GrantKind::Equipment));
    }
    assert_eq!(catalog.skill(&id("dagger_throw")).unwrap().max_uses, None);
    assert_eq!(
        catalog
            .skill(&id("greatsword_cleave"))
            .unwrap()
            .target_pattern,
        TargetPattern::FrontPair
    );
}
#[test]
fn grants_deduplicate_without_losing_sources_and_passives_apply_once() {
    let mut raw = catalog().definition().clone();
    let weapon = raw
        .weapons
        .iter_mut()
        .find(|w| w.id == id("dagger"))
        .unwrap();
    weapon.skills.push(id("mend"));
    weapon.abilities.push(id("duelist_dagger_power"));
    let catalog = ContentCatalog::new(raw).unwrap();
    let mut build = dagger();
    build.skills.push(SkillGrant {
        skill: id("mend"),
        provenance: id("medic"),
    });
    build.abilities = vec![id("duelist_dagger_power"), id("assassin_bleeding_dagger")];
    let resolved = catalog.resolve_build(&build).unwrap();
    assert_eq!(resolved.moveset.skills.len(), 3);
    assert_eq!(resolved.moveset.skills[0].grants.len(), 2);
    let stab = resolved
        .moveset
        .skills
        .iter()
        .find(|s| s.definition.id == id("dagger_stab"))
        .unwrap();
    assert_eq!(
        stab.definition.effects,
        vec![
            Effect::Damage(7),
            Effect::ApplyStatus(crate::StatusKind::Bleed)
        ]
    );
    assert_eq!(stab.upgrades.len(), 2);
    assert_eq!(
        resolved
            .abilities
            .iter()
            .find(|a| a.definition.id == id("duelist_dagger_power"))
            .unwrap()
            .grants
            .len(),
        2
    );
    build.abilities.reverse();
    assert_eq!(catalog.resolve_build(&build).unwrap(), resolved);
    build.weapon = None;
    let removed = catalog.resolve_build(&build).unwrap();
    assert_eq!(removed.moveset.skills.len(), 1);
    assert_eq!(removed.moveset.skills[0].definition.id, id("mend"));
    assert_eq!(removed.moveset.skills[0].grants.len(), 1);
    assert!(removed.abilities.iter().all(|a| !a.active()));
    build.weapon = Some(id("dagger"));
    assert_eq!(catalog.resolve_build(&build).unwrap(), resolved);
}
#[test]
fn missing_targets_deactivate_passives_and_repeated_selections_are_rejected() {
    let catalog = catalog();
    let mut build = CharacterBuild {
        weapon: Some(id("knifehand_daggers")),
        abilities: vec![id("assassin_bleeding_dagger")],
        ..Default::default()
    };
    let resolved = catalog.resolve_build(&build).unwrap();
    assert!(resolved.abilities[0]
        .inactive_reason
        .as_ref()
        .unwrap()
        .contains("dagger_stab"));
    build.abilities.push(id("assassin_bleeding_dagger"));
    assert!(catalog
        .resolve_build(&build)
        .unwrap_err()
        .message
        .contains("duplicate"));
}
#[test]
fn resolved_builds_serialize_beyond_eight_and_reject_tampered_views() {
    let mut raw = catalog().definition().clone();
    for skill in &mut raw.skills {
        skill.personal_selectable = true;
        skill.requirements.clear();
    }
    let catalog = ContentCatalog::new(raw).unwrap();
    let build = CharacterBuild {
        skills: catalog
            .definition()
            .skills
            .iter()
            .take(12)
            .map(|a| SkillGrant {
                skill: a.id.clone(),
                provenance: id("skills"),
            })
            .collect(),
        ..Default::default()
    };
    let resolved = catalog.resolve_build(&build).unwrap();
    assert_eq!(resolved.moveset.skills.len(), 12);
    let mut copy: ResolvedBuild =
        serde_json::from_str(&serde_json::to_string(&resolved).unwrap()).unwrap();
    catalog.validate_resolved(&build, &copy).unwrap();
    copy.moveset.skills[0].definition.effects = vec![Effect::Damage(999)];
    assert!(catalog.validate_resolved(&build, &copy).is_err());
    assert_eq!(catalog.resolve_build(&build).unwrap(), resolved);
}
#[test]
fn builds_own_their_resolved_definitions_and_presets_do_not_constrain_stats() {
    let catalog = catalog();
    let mut first = ActorBuild::from_preset(&catalog, &id("gatekeeper")).unwrap();
    first.max_hp = 123;
    first.base_speed = 99;
    first.footprint = 3;
    first.build = dagger();
    let mut resolved = first.resolve(&catalog).unwrap();
    resolved.moveset.skills[0].definition.effects.clear();
    let second = ActorBuild::from_preset(&catalog, &id("gatekeeper")).unwrap();
    assert_eq!(
        (second.max_hp, second.base_speed, second.footprint),
        (32, 2, 1)
    );
    assert_eq!(
        first.resolve(&catalog).unwrap().moveset.skills[0]
            .definition
            .effects,
        vec![Effect::Damage(5)]
    );
    assert!(catalog
        .resolve_build(&CharacterBuild::default())
        .unwrap()
        .moveset
        .skills
        .is_empty());
    first.footprint = 7;
    assert_eq!(first.resolve(&catalog).unwrap_err().path, "actor.footprint");
}
#[test]
fn canonical_fingerprint_ignores_definition_order_but_tracks_values() {
    let catalog = catalog();
    let mut raw = catalog.definition().clone();
    raw.skills.reverse();
    raw.weapons.reverse();
    raw.abilities.reverse();
    raw.actor_presets.reverse();
    assert_eq!(
        ContentCatalog::new(raw.clone()).unwrap().fingerprint(),
        catalog.fingerprint()
    );
    raw.skills[0].name.push('!');
    assert_ne!(
        ContentCatalog::new(raw).unwrap().fingerprint(),
        catalog.fingerprint()
    );
}
#[test]
fn invalid_catalogs_report_reference_value_and_upgrade_context() {
    let base = catalog().definition().clone();
    let mut raw = base.clone();
    raw.skills.push(raw.skills[0].clone());
    assert!(ContentCatalog::new(raw)
        .unwrap_err()
        .message
        .contains("duplicate id"));
    let mut raw = base.clone();
    raw.weapons[0].skills.push(id("missing_move"));
    assert!(ContentCatalog::new(raw)
        .unwrap_err()
        .path
        .contains("weapons."));
    for mask in [0, 64, 255] {
        let mut raw = base.clone();
        raw.skills[0].source_ranks = mask;
        assert!(ContentCatalog::new(raw)
            .unwrap_err()
            .path
            .ends_with("source_ranks"));
    }
    for effect in [
        Effect::Damage(0),
        Effect::Damage(10001),
        Effect::Heal(0),
        Effect::Move(0),
        Effect::Move(6),
        Effect::Rescue(101),
        Effect::StatusDamage(crate::DamageKind::Bleed),
    ] {
        let mut raw = base.clone();
        raw.skills[0].effects = vec![effect];
        assert!(ContentCatalog::new(raw)
            .unwrap_err()
            .path
            .contains("effects"));
    }
    let mut raw = base.clone();
    raw.skills[0].max_uses = Some(0);
    assert!(ContentCatalog::new(raw)
        .unwrap_err()
        .path
        .ends_with("max_uses"));
    let mut raw = base.clone();
    raw.schema_version = 999;
    assert_eq!(ContentCatalog::new(raw).unwrap_err().path, "schema_version");
    let mut raw = base.clone();
    raw.abilities[0].upgrades = vec![SkillUpgrade {
        skill: id("mend"),
        operations: vec![UpgradeOperation::AddDamage {
            effect_index: 0,
            amount: 2,
        }],
    }];
    assert!(ContentCatalog::new(raw)
        .unwrap_err()
        .message
        .contains("base Damage"));
    let mut raw = base;
    raw.abilities[0].upgrades = vec![SkillUpgrade {
        skill: id("greatsword_cleave"),
        operations: vec![UpgradeOperation::ExtendTargetRanks(63)],
    }];
    assert!(ContentCatalog::new(raw)
        .unwrap_err()
        .message
        .contains("FrontPair"));
}
#[test]
fn parser_rejects_unknown_effects_fields_passives_and_bad_ids() {
    let source = include_str!("../content/catalog.toml");
    for (old, new) in [
        ("Damage = 2", "UnimplementedMagic = 2"),
        ("source_ranks = 63", "source_rank_typo = 63"),
        (
            "source_ranks = 63",
            "behavior = \"Passive\"\nsource_ranks = 63",
        ),
        ("id = \"hurled_scrap\"", "id = \"Bad Id\""),
    ] {
        let malformed = source.replacen(old, new, 1);
        assert_ne!(malformed, source);
        assert!(ContentCatalog::from_toml(&malformed).is_err());
    }
    assert!(ContentCatalog::from_toml(&" ".repeat(MAX_CATALOG_BYTES + 1)).is_err());
    for invalid in ["", "Bad", "space id", "é", "x/y"] {
        assert!(ContentId::new(invalid).is_err());
    }
}
#[test]
fn combined_upgrades_and_move_count_are_bounded_without_silent_truncation() {
    let mut raw = catalog().definition().clone();
    for (key, amount) in [("a_power", 6000), ("b_power", 6000)] {
        raw.abilities.push(AbilityDefinition {
            id: id(key),
            name: key.into(),
            description: "Large valid individual bonus.".into(),
            provenance: id("test"),
            personal_selectable: true,
            requirements: vec![],
            effects: vec![],
            upgrades: vec![SkillUpgrade {
                skill: id("dagger_stab"),
                operations: vec![UpgradeOperation::AddDamage {
                    effect_index: 0,
                    amount,
                }],
            }],
        });
    }
    let catalog = ContentCatalog::new(raw).unwrap();
    let mut build = dagger();
    build.abilities = vec![id("a_power"), id("b_power")];
    assert!(catalog
        .resolve_build(&build)
        .unwrap_err()
        .message
        .contains("combined damage"));
    let mut raw = catalog.definition().clone();
    let mut template = raw.skills[0].clone();
    template.personal_selectable = true;
    template.requirements.clear();
    for i in 0..65 {
        let mut a = template.clone();
        a.id = id(&format!("overflow_{i}"));
        raw.skills.push(a);
    }
    let catalog = ContentCatalog::new(raw).unwrap();
    let mut build = CharacterBuild {
        skills: (0..64)
            .map(|i| SkillGrant {
                skill: id(&format!("overflow_{i}")),
                provenance: id("skills"),
            })
            .collect(),
        ..Default::default()
    };
    assert_eq!(
        catalog.resolve_build(&build).unwrap().moveset.skills.len(),
        64
    );
    build.weapon = Some(id("dagger"));
    assert!(catalog
        .resolve_build(&build)
        .unwrap_err()
        .message
        .contains("maximum 64"));
}
