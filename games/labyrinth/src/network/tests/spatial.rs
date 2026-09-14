//! Two real encrypted Apps on loopback; not cross-machine or desktop evidence.

use super::*;

#[test]
fn spatial_encrypted_reservations_guest_type_choice_gaps_and_save_load_converge() {
    let directory = tempfile::tempdir().expect("scenario directory");
    let path = directory.path().join("spatial.json");
    let mut apps = vec![socket_app(None), socket_app(None)];
    open_default_host(&mut apps, "");
    let code = hosted_code(app(&mut apps, 0).world(), 0).expect("invite");
    start::join_code(app(&mut apps, 1).world_mut(), &code).expect("join");
    assert!(
        pump_until(&mut apps, Duration::from_secs(10), |apps| all_admitted(
            apps
        ) && converged(
            apps
        )),
        "{}",
        admission_diagnostics(&apps)
    );
    let original = host_snapshot(&mut apps);
    app(&mut apps, 0)
        .world_mut()
        .write_message(LabyrinthIntent::SaveScenario(
            path.to_string_lossy().into_owned(),
        ));
    assert!(pump_until(&mut apps, Duration::from_secs(5), |_| path.exists()));
    let original_file = std::fs::read(&path).expect("initial portable scenario");
    app(&mut apps, 0)
        .world_mut()
        .write_message(LabyrinthIntent::RemoveScenarioActor {
            actor: ActorId(2),
            expected_revision: original.setup_revision,
        });
    assert!(pump_until(
        &mut apps,
        Duration::from_secs(5),
        |apps| host_snapshot(apps).setup_revision > original.setup_revision && converged(apps)
    ));
    let sparse = host_snapshot(&mut apps);
    assert_eq!(sparse.formation.rank(ActorId(3)), Some(3));
    for app in &apps {
        let view = app.world().resource::<LabyrinthView>();
        assert!(view
            .deployment_error
            .as_deref()
            .is_some_and(|e| e.contains("rank 2")));
        assert_eq!(view.formation.as_ref(), Some(&sparse.formation));
    }
    app(&mut apps, 0)
        .world_mut()
        .write_message(LabyrinthIntent::SaveScenario(
            path.to_string_lossy().into_owned(),
        ));
    assert!(pump_until(&mut apps, Duration::from_secs(5), |apps| app(
        apps, 0
    )
    .world()
    .resource::<LabyrinthView>()
    .notice
    .as_deref()
    .is_some_and(|e| e.contains("rank 2"))));
    assert_eq!(
        std::fs::read(&path).expect("saved file intact"),
        original_file
    );
    app(&mut apps, 0)
        .world_mut()
        .write_message(LabyrinthIntent::AssignFormationRank {
            rank: 2,
            owner: 1,
            expected_revision: sparse.setup_revision,
        });
    assert!(pump_until(
        &mut apps,
        Duration::from_secs(5),
        |apps| host_snapshot(apps).formation.owner(2) == Some(1) && converged(apps)
    ));
    let reserved = host_snapshot(&mut apps);
    assert!(reserved
        .player_views()
        .iter()
        .find(|p| p.slot == 1)
        .expect("guest")
        .actors
        .is_empty());
    let preset = reserved
        .catalog
        .definition()
        .actor_presets
        .iter()
        .find(|p| p.appearance == labyrinth_rules::ActorKind::Hero(HeroClass::Knifehand))
        .expect("type selected before mutation")
        .id
        .clone();
    app(&mut apps, 1)
        .world_mut()
        .write_message(LabyrinthIntent::PlaceScenarioActor {
            team: Team::Heroes,
            rank: 2,
            preset: preset.clone(),
            expected_revision: reserved.setup_revision,
        });
    assert!(pump_until(
        &mut apps,
        Duration::from_secs(5),
        |apps| host_snapshot(apps).setup_revision > reserved.setup_revision && converged(apps)
    ));
    let chosen = host_snapshot(&mut apps);
    let actor = chosen
        .formation
        .occupant(&chosen.scenario, Team::Heroes, 2)
        .expect("placed character");
    assert!(chosen
        .company
        .iter()
        .any(|m| m.actor == actor && m.owner == 1 && m.hero == HeroClass::Knifehand));
    assert!(chosen
        .formation
        .deployment_error(&chosen.scenario)
        .is_none());
    // A delayed type commit may not overwrite a later selection, even when the owner still matches.
    app(&mut apps, 1)
        .world_mut()
        .write_message(LabyrinthIntent::PlaceScenarioActor {
            team: Team::Heroes,
            rank: 2,
            preset,
            expected_revision: reserved.setup_revision,
        });
    assert!(pump_until(&mut apps, Duration::from_secs(5), |apps| app(
        apps, 1
    )
    .world()
    .resource::<LabyrinthView>()
    .notice
    .as_deref()
    .is_some_and(|e| e.contains("Setup changed"))));
    assert_eq!(host_snapshot(&mut apps).scenario, chosen.scenario);
    let mut build = chosen
        .scenario
        .heroes
        .iter()
        .find(|a| a.id == actor)
        .expect("owned build")
        .clone();
    build.actor.name = "Reserved rank dagger user".into();
    build.actor.build.weapon =
        Some(labyrinth_rules::catalog::ContentId::new("dagger").expect("weapon ID"));
    app(&mut apps, 1)
        .world_mut()
        .write_message(LabyrinthIntent::CustomizeActor {
            actor: build.clone(),
            expected_revision: chosen.setup_revision,
        });
    assert!(pump_until(
        &mut apps,
        Duration::from_secs(5),
        |apps| host_snapshot(apps)
            .scenario
            .heroes
            .iter()
            .any(|a| a == &build)
            && converged(apps)
    ));
    let saved = host_snapshot(&mut apps);
    app(&mut apps, 0)
        .world_mut()
        .write_message(LabyrinthIntent::SaveScenario(
            path.to_string_lossy().into_owned(),
        ));
    assert!(pump_until(&mut apps, Duration::from_secs(5), |_| {
        std::fs::read_to_string(&path).is_ok_and(|json| json.contains("Reserved rank dagger user"))
    }));
    let portable = labyrinth_rules::scenario::Scenario::from_json(
        &std::fs::read_to_string(&path).expect("saved input"),
        &saved.catalog,
    )
    .expect("deployable portable scenario");
    assert_eq!(portable, saved.scenario);
    // Both-side positions converge as shared construction, and loading restores explicit compact input.
    let enemy = saved.scenario.enemies.get(1).expect("enemy").id;
    app(&mut apps, 0)
        .world_mut()
        .write_message(LabyrinthIntent::RemoveScenarioActor {
            actor: enemy,
            expected_revision: saved.setup_revision,
        });
    assert!(pump_until(
        &mut apps,
        Duration::from_secs(5),
        |apps| host_snapshot(apps).setup_revision > saved.setup_revision && converged(apps)
    ));
    assert!(host_snapshot(&mut apps)
        .formation
        .deployment_error(&host_snapshot(&mut apps).scenario)
        .is_some_and(|e| e.contains("Enemies")));
    app(&mut apps, 0)
        .world_mut()
        .write_message(LabyrinthIntent::LoadScenario(
            path.to_string_lossy().into_owned(),
        ));
    assert!(pump_until(
        &mut apps,
        Duration::from_secs(5),
        |apps| host_snapshot(apps).scenario == saved.scenario && converged(apps)
    ));
    let loaded = host_snapshot(&mut apps);
    assert_eq!(loaded.formation, saved.formation);
    for app in &mut apps {
        app.world_mut().write_message(LabyrinthIntent::Ready(true));
    }
    assert!(pump_until(
        &mut apps,
        Duration::from_secs(5),
        |apps| host_snapshot(apps)
            .players
            .iter()
            .filter(|p| p.occupied)
            .all(|p| p.ready)
            && converged(apps)
    ));
    app(&mut apps, 0)
        .world_mut()
        .write_message(LabyrinthIntent::StartEncounter);
    assert!(pump_until(
        &mut apps,
        Duration::from_secs(5),
        |apps| host_snapshot(apps).combat.is_some() && converged(apps)
    ));
    let started = host_snapshot(&mut apps);
    started.validate().expect("complete compact snapshot");
    assert_eq!(started.combat.expect("combat").ranks(actor), Some(2..=2));
}

#[test]
fn spatial_local_deploy_is_atomic_after_edits_and_still_rejects_gaps() {
    let mut local = socket_app(None);
    local
        .world_mut()
        .write_message(LabyrinthIntent::StartLocal(42));
    local.update();
    let original = local.world().resource::<PartyAuthority>().snapshot(0);
    local
        .world_mut()
        .write_message(LabyrinthIntent::RemoveScenarioActor {
            actor: ActorId(2),
            expected_revision: original.setup_revision,
        });
    local.update();
    let sparse = local.world().resource::<PartyAuthority>().snapshot(0);
    assert!(sparse.players.iter().all(|player| !player.ready));
    local
        .world_mut()
        .write_message(LabyrinthIntent::StartEncounter);
    local.update();
    let rejected = local.world().resource::<PartyAuthority>().snapshot(0);
    assert!(rejected.combat.is_none());
    assert_eq!(
        rejected.players, sparse.players,
        "rejected deployment must not run a separate Ready mutation"
    );
    assert_eq!(rejected.formation, sparse.formation);
    assert!(local
        .world()
        .resource::<LabyrinthView>()
        .notice
        .as_deref()
        .is_some_and(|error| error.contains("rank 2")));
    let preset = original
        .catalog
        .definition()
        .actor_presets
        .iter()
        .find(|preset| preset.appearance == labyrinth_rules::ActorKind::Hero(HeroClass::Gatekeeper))
        .expect("selected character type")
        .id
        .clone();
    local
        .world_mut()
        .write_message(LabyrinthIntent::PlaceScenarioActor {
            team: Team::Heroes,
            rank: 2,
            preset,
            expected_revision: sparse.setup_revision,
        });
    local.update();
    let configured = local.world().resource::<PartyAuthority>().snapshot(0);
    assert!(configured
        .formation
        .deployment_error(&configured.scenario)
        .is_none());
    assert!(configured.players.iter().all(|player| !player.ready));
    local
        .world_mut()
        .write_message(LabyrinthIntent::StartEncounter);
    local.update();
    let deployed = local.world().resource::<PartyAuthority>().snapshot(0);
    assert!(
        deployed.combat.is_some(),
        "one local Deploy must work after a valid edit without an unavailable Ready control"
    );
    deployed.validate().expect("complete local deployment");
}
