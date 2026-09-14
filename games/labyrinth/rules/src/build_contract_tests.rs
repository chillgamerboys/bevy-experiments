#![expect(
    clippy::unwrap_used,
    clippy::indexing_slicing,
    reason = "bounded content fixtures assert validation"
)]
use crate::{build::*, catalog::*, scenario::*, ActorKind, Effect, HeroClass, StatusKind};
fn id(value: &str) -> ContentId {
    ContentId::new(value).unwrap()
}
fn personal(key: &str) -> SkillGrant {
    SkillGrant {
        skill: id(key),
        provenance: id("test"),
    }
}
fn catalog() -> ContentCatalog {
    ContentCatalog::builtin().unwrap()
}

#[test]
fn personal_requirements_deactivate_and_reactivate_without_losing_selections() {
    let catalog = catalog();
    let mut build = CharacterBuild {
        skills: vec![personal("mend"), personal("assassin_feint")],
        abilities: vec![id("resilient"), id("duelist_dagger_power")],
        weapon: Some(id("dagger")),
    };
    let original = build.clone();
    let active = catalog.resolve_build(&build).unwrap();
    assert!(active.inactive_skills.is_empty());
    assert!(active.abilities.iter().all(ResolvedAbility::active));
    assert_eq!(active.moveset.skills.len(), 4);
    build.weapon = None;
    let inactive = catalog.resolve_build(&build).unwrap();
    assert_eq!(inactive.moveset.skills.len(), 1);
    assert_eq!(inactive.moveset.skills[0].definition.id, id("mend"));
    assert_eq!(
        inactive.inactive_skills[0].definition.id,
        id("assassin_feint")
    );
    assert!(inactive.inactive_skills[0].reason.contains("dagger"));
    assert!(inactive
        .abilities
        .iter()
        .find(|a| a.definition.id == id("resilient"))
        .unwrap()
        .active());
    assert!(!inactive
        .abilities
        .iter()
        .find(|a| a.definition.id == id("duelist_dagger_power"))
        .unwrap()
        .active());
    build.weapon = Some(id("dagger"));
    assert_eq!(build, original);
    assert_eq!(catalog.resolve_build(&build).unwrap(), active);
}

#[test]
fn exact_item_and_kind_requirements_are_distinct_and_invalid_contracts_fail() {
    let mut raw = catalog().definition().clone();
    raw.skills
        .iter_mut()
        .find(|s| s.id == id("mend"))
        .unwrap()
        .requirements = vec![
        EquipmentRequirement::Kind(id("staff")),
        EquipmentRequirement::Item(id("medic_staff")),
    ];
    let catalog = ContentCatalog::new(raw.clone()).unwrap();
    let mut build = CharacterBuild {
        skills: vec![personal("mend")],
        weapon: Some(id("staff")),
        ..Default::default()
    };
    assert_eq!(
        catalog.resolve_build(&build).unwrap().inactive_skills.len(),
        1
    );
    build.weapon = Some(id("medic_staff"));
    assert!(catalog
        .resolve_build(&build)
        .unwrap()
        .inactive_skills
        .is_empty());
    for requirements in [
        vec![EquipmentRequirement::Kind(id("unknown"))],
        vec![EquipmentRequirement::Item(id("missing"))],
        vec![
            EquipmentRequirement::Kind(id("bow")),
            EquipmentRequirement::Item(id("medic_staff")),
        ],
        vec![
            EquipmentRequirement::Kind(id("staff")),
            EquipmentRequirement::Kind(id("staff")),
        ],
    ] {
        raw.skills
            .iter_mut()
            .find(|s| s.id == id("mend"))
            .unwrap()
            .requirements = requirements;
        assert!(ContentCatalog::new(raw.clone()).is_err());
    }
}

#[test]
fn equipment_only_skill_and_ability_grants_cannot_become_personal_via_saved_input() {
    let mut raw = catalog().definition().clone();
    raw.abilities
        .iter_mut()
        .find(|a| a.id == id("resilient"))
        .unwrap()
        .personal_selectable = false;
    raw.weapons
        .iter_mut()
        .find(|w| w.id == id("staff"))
        .unwrap()
        .abilities
        .push(id("resilient"));
    let catalog = ContentCatalog::new(raw).unwrap();
    let equipment = CharacterBuild {
        weapon: Some(id("staff")),
        ..Default::default()
    };
    let resolved = catalog.resolve_build(&equipment).unwrap();
    assert_eq!(resolved.abilities[0].grants[0].kind, GrantKind::Equipment);
    assert!(resolved.abilities[0].active());
    let mut scenario = Scenario::stock(StockScenario::Prototype, 42, &catalog).unwrap();
    for build in [
        CharacterBuild {
            skills: vec![personal("dagger_stab")],
            ..Default::default()
        },
        CharacterBuild {
            abilities: vec![id("resilient")],
            ..Default::default()
        },
    ] {
        scenario.heroes[0].actor.build = build;
        let input = serde_json::to_string(&scenario).unwrap();
        assert!(Scenario::from_json(&input, &catalog)
            .unwrap_err()
            .message
            .contains("equipment-only"));
    }
    let removed = catalog.resolve_build(&CharacterBuild::default()).unwrap();
    assert!(removed.abilities.is_empty());
}

#[test]
fn every_preset_preserves_its_exact_skills_and_effects_through_real_sources() {
    let catalog = catalog();
    for preset in &catalog.definition().actor_presets {
        let resolved = catalog.resolve_build(&preset.build).unwrap();
        let mut actual = resolved
            .moveset
            .skills
            .iter()
            .map(|skill| skill.definition.id.clone())
            .collect::<Vec<_>>();
        let mut expected = crate::skills_for(preset.appearance)
            .iter()
            .map(|skill| legacy_skill_id(*skill))
            .collect::<Vec<_>>();
        actual.sort();
        expected.sort();
        assert_eq!(actual, expected, "{}", preset.id);
        for skill in &resolved.moveset.skills {
            assert_eq!(
                skill.definition.effects,
                catalog.skill(&skill.definition.id).unwrap().effects
            );
        }
        for grant in &preset.build.skills {
            assert!(catalog.skill(&grant.skill).unwrap().personal_selectable);
        }
    }
    for kind in [
        ActorKind::Hero(HeroClass::Scout),
        ActorKind::Hero(HeroClass::FieldMedic),
    ] {
        let preset = catalog.preset_for_appearance(kind).unwrap();
        assert!(preset.build.weapon.is_some());
    }
}

#[test]
fn passive_duration_reduction_is_bounded_and_deduplicated_across_sources() {
    let mut raw = catalog().definition().clone();
    raw.weapons
        .iter_mut()
        .find(|w| w.id == id("dagger"))
        .unwrap()
        .abilities
        .push(id("resilient"));
    let catalog = ContentCatalog::new(raw).unwrap();
    let build = CharacterBuild {
        weapon: Some(id("dagger")),
        abilities: vec![id("resilient")],
        ..Default::default()
    };
    let resolved = catalog.resolve_build(&build).unwrap();
    assert_eq!(resolved.abilities[0].grants.len(), 2);
    assert_eq!(resolved.status_duration(StatusKind::Bleed, 3), 2);
    assert_eq!(resolved.status_duration(StatusKind::Weakened, 2), 1);
    assert_eq!(resolved.status_duration(StatusKind::Weakened, 1), 1);
    assert_eq!(resolved.status_duration(StatusKind::Haste, 2), 2);
    assert_eq!(resolved.status_duration(StatusKind::Brace, 1), 1);
    assert!(resolved
        .moveset
        .skills
        .iter()
        .all(|s| s.definition.id != id("resilient")));
}

#[test]
fn old_schema_and_active_grants_inside_passive_definitions_are_rejected() {
    let catalog = catalog();
    let mut wire = serde_json::to_value(catalog.definition()).unwrap();
    wire["abilities"][0]["grants"] = serde_json::json!(["mend"]);
    assert!(serde_json::from_value::<ContentCatalog>(wire).is_err());
    let mut old = catalog.definition().clone();
    old.schema_version = 1;
    assert!(ContentCatalog::new(old)
        .unwrap_err()
        .message
        .contains("schema 2"));
    let mut scenario = Scenario::stock(StockScenario::Prototype, 42, &catalog).unwrap();
    scenario.schema_version = 1;
    assert!(
        Scenario::from_json(&serde_json::to_string(&scenario).unwrap(), &catalog)
            .unwrap_err()
            .message
            .contains("schema 2")
    );
    let mut forged = catalog.resolve_build(&CharacterBuild::default()).unwrap();
    forged.abilities.push(ResolvedAbility {
        definition: catalog.ability(&id("resilient")).unwrap().clone(),
        grants: vec![],
        inactive_reason: None,
    });
    assert!(catalog
        .validate_resolved(&CharacterBuild::default(), &forged)
        .is_err());
    // Runtime move definitions remain typed active effects; passives cannot enter here.
    assert!(catalog
        .skill(&id("mend"))
        .unwrap()
        .effects
        .contains(&Effect::Heal(8)));
}

#[test]
fn legacy_equipment_is_explicit_and_cannot_override_an_existing_conflicting_item() {
    let catalog = catalog();
    let loadout = &[crate::SkillId::Mend][..];
    let build = legacy_build(loadout);
    assert!(catalog.resolve_build(&build).is_err());
    let authored = legacy_catalog(&catalog, [loadout]).unwrap();
    assert_eq!(
        authored.resolve_build(&build).unwrap().moveset.skills[0]
            .definition
            .id,
        id("mend")
    );
    let mut raw = authored.definition().clone();
    raw.weapons
        .iter_mut()
        .find(|weapon| Some(&weapon.id) == build.weapon.as_ref())
        .unwrap()
        .skills = vec![id("dagger_stab")];
    let conflict = ContentCatalog::new(raw).unwrap();
    assert!(legacy_catalog(&conflict, [loadout]).is_err());
}
