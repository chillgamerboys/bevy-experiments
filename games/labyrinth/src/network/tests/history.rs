//! One host and one guest over the production encrypted loopback transport.
use super::*;
use crate::session::history::{HISTORY_PAGE_EVENTS, HistoryError, HistoryRequest};

fn host_command(apps: &mut [App], command: SessionCommand) {
    let mut authority = app(apps, 0).world_mut().resource_mut::<PartyAuthority>();
    let snapshot = authority.snapshot(0);
    let result = authority.apply(
        0,
        GameRequest {
            sequence: snapshot.next_sequence,
            encounter: snapshot.encounter,
            decision: snapshot.combat.as_ref().map_or(0, |combat| combat.turn_id),
            assignment_revision: snapshot.assignment_revision,
            command,
        },
    );
    assert_eq!(result.rejection, None);
}

fn join_guest(apps: &mut [App]) {
    let code = hosted_code(app(apps, 0).world(), 0).expect("invitation");
    start::join_code(app(apps, 1).world_mut(), &code).expect("join");
    assert!(pump_until(apps, Duration::from_secs(10), all_admitted_mut));
}

fn all_admitted_mut(apps: &mut [App]) -> bool {
    all_admitted(apps)
}

fn prepare_history_encounter(apps: &mut [App]) {
    let snapshot = host_snapshot(apps);
    let mut scenario = snapshot.scenario;
    for actor in scenario.heroes.iter_mut().chain(&mut scenario.enemies) {
        actor.actor.max_hp = 10_000;
    }
    host_command(
        apps,
        SessionCommand::ConfigureBattle {
            scenario,
            expected_revision: snapshot.setup_revision,
        },
    );
    host_command(apps, SessionCommand::Ready(true));
    host_command(apps, SessionCommand::Start);
    // Test drives explicit accepted decisions; transport polling does not add an
    // unrelated enemy turn while assertions compare history reads with authority.
    app(apps, 0)
        .world_mut()
        .resource_mut::<Runtime>()
        .next_enemy = Instant::now() + Duration::from_secs(120);
}

fn advance_past(apps: &mut [App], minimum_next: u64) {
    for _ in 0..300 {
        let snapshot = host_snapshot(apps);
        if snapshot.history.next >= minimum_next {
            return;
        }
        let combat = snapshot.combat.expect("combat");
        let actor = combat.active_actor.expect("live decision");
        if combat.actor(actor).unwrap().team() == Team::Enemies {
            assert!(
                app(apps, 0)
                    .world_mut()
                    .resource_mut::<PartyAuthority>()
                    .advance_enemy()
            );
        } else {
            host_command(
                apps,
                SessionCommand::Act {
                    actor,
                    action: CombatAction::Defend,
                },
            );
        }
    }
    panic!("bounded encounter did not produce enough history");
}

fn all_history_recovered(apps: &mut [App]) -> bool {
    let snapshot = host_snapshot(apps);
    apps.iter().all(|app| {
        let history = app.world().resource::<EncounterHistory>();
        history.encounter == snapshot.encounter
            && history.bounds == snapshot.history
            && history.complete()
    })
}

fn assert_exact_archive(apps: &mut [App]) {
    let before = host_snapshot(apps);
    let mut from = before.history.first;
    while from < before.history.next {
        let request = HistoryRequest {
            request_id: 1,
            encounter: before.encounter,
            from,
            limit: HISTORY_PAGE_EVENTS as u16,
        };
        let page = app(apps, 0)
            .world()
            .resource::<PartyAuthority>()
            .history_page(0, request)
            .expect("host archive");
        for app in apps.iter() {
            assert_eq!(
                app.world()
                    .resource::<EncounterHistory>()
                    .page(from, HISTORY_PAGE_EVENTS),
                page.events
            );
            assert!(app.world().resource::<LabyrinthView>().events.len() <= 80);
        }
        from = page.events.last().unwrap().id + 1;
    }
    assert_eq!(host_snapshot(apps), before);
}

#[test]
fn encrypted_history_recovers_missed_windows_and_fresh_guest_reconnect_without_combat_side_effects()
{
    let directory = tempfile::tempdir().unwrap();
    let profile = directory.path().join("history-guest.json");
    let mut apps = vec![socket_app(None), socket_app(Some(&profile))];
    open_default_host(&mut apps, "");
    join_guest(&mut apps);
    let peer = stored(app(&mut apps, 1)).peer_id;
    prepare_history_encounter(&mut apps);
    advance_past(&mut apps, 260);
    let before_reads = host_snapshot(&mut apps);
    assert!(pump_until(
        &mut apps,
        Duration::from_secs(10),
        all_history_recovered
    ));
    assert_eq!(
        host_snapshot(&mut apps),
        before_reads,
        "history exchange never consumes a decision or command sequence"
    );
    assert_exact_archive(&mut apps);

    // A live guest misses a recent window before its next network poll.
    let next = before_reads.history.next + 180;
    advance_past(&mut apps, next);
    assert!(pump_until(
        &mut apps,
        Duration::from_secs(10),
        all_history_recovered
    ));
    assert_exact_archive(&mut apps);
    let retained = app(&mut apps, 1)
        .world()
        .resource::<EncounterHistory>()
        .loaded_len();
    let old_attempt = app(&mut apps, 1)
        .world()
        .resource::<Runtime>()
        .attempt
        .unwrap();
    start::disconnect_guest(app(&mut apps, 1).world_mut());
    assert_eq!(
        app(&mut apps, 1)
            .world()
            .resource::<EncounterHistory>()
            .loaded_len(),
        retained
    );
    assert!(pump_until(&mut apps, Duration::from_secs(5), |apps| {
        !host_snapshot(apps).players[1].connected
    }));
    advance_past(&mut apps, next + 180);

    // Destroy the old guest App, load its saved profile into a fresh App, and
    // re-establish admission against the same live host (not OS-process evidence).
    apps[1] = socket_app(Some(&profile));
    start::reconnect(app(&mut apps, 1).world_mut()).expect("reconnect");
    assert!(pump_until(
        &mut apps,
        Duration::from_secs(10),
        all_admitted_mut
    ));
    assert_eq!(stored(app(&mut apps, 1)).peer_id, peer);
    assert!(pump_until(
        &mut apps,
        Duration::from_secs(10),
        all_history_recovered
    ));
    assert_exact_archive(&mut apps);

    let current = host_snapshot(&mut apps);
    let stale_request = HistoryRequest {
        request_id: 1,
        encounter: current.encounter,
        from: current.history.first,
        limit: 64,
    };
    let stale_page = app(&mut apps, 0)
        .world()
        .resource::<PartyAuthority>()
        .history_page(0, stale_request)
        .unwrap();
    let retained = app(&mut apps, 1)
        .world()
        .resource::<EncounterHistory>()
        .loaded_len();
    app(&mut apps, 1).world_mut().write_message(HistoryReply {
        attempt: old_attempt,
        request: stale_request,
        result: Ok(stale_page.clone()),
    });
    super::super::history::receive(app(&mut apps, 1).world_mut());
    assert_eq!(
        app(&mut apps, 1)
            .world()
            .resource::<EncounterHistory>()
            .loaded_len(),
        retained
    );

    // Reads did not break subsequent gameplay; then a new encounter must reject
    // an old encounter page even if it carries the current physical attempt.
    advance_past(&mut apps, current.history.next + 1);
    host_command(&mut apps, SessionCommand::Rematch);
    host_command(&mut apps, SessionCommand::Ready(true));
    host_command(&mut apps, SessionCommand::Start);
    assert!(pump_until(
        &mut apps,
        Duration::from_secs(5),
        all_history_recovered
    ));
    let attempt = app(&mut apps, 1)
        .world()
        .resource::<Runtime>()
        .attempt
        .unwrap();
    app(&mut apps, 1).world_mut().write_message(HistoryReply {
        attempt,
        request: stale_request,
        result: Ok(stale_page),
    });
    super::super::history::receive(app(&mut apps, 1).world_mut());
    assert_exact_archive(&mut apps);
    start::close(app(&mut apps, 0).world_mut());
    start::close(app(&mut apps, 1).world_mut());
}

#[test]
fn history_admission_ranges_and_queue_budget_preserve_authority_and_quiet_requests() {
    let mut apps = vec![socket_app(None), socket_app(None)];
    open_default_host(&mut apps, "");
    join_guest(&mut apps);
    prepare_history_encounter(&mut apps);
    let before = host_snapshot(&mut apps);
    let guest = *app(&mut apps, 0)
        .world()
        .resource::<Hosted>()
        .connections
        .keys()
        .next()
        .unwrap();
    let stranger = app(&mut apps, 0).world_mut().spawn_empty().id();
    let request = HistoryRequest {
        request_id: 1,
        encounter: before.encounter,
        from: before.history.first,
        limit: 64,
    };
    let world = app(&mut apps, 0).world_mut();
    world
        .resource_mut::<Messages<ToClients<HistoryReply>>>()
        .clear();
    world.write_message(FromClient {
        client_id: ClientId::from(stranger),
        message: request,
    });
    world.write_message(FromClient {
        client_id: ClientId::from(guest),
        message: HistoryRequest {
            limit: 65,
            ..request
        },
    });
    admission::host_messages(world);
    let responses = world
        .resource_mut::<Messages<ToClients<HistoryReply>>>()
        .drain()
        .collect::<Vec<_>>();
    assert_eq!(
        responses.len(),
        1,
        "unadmitted connection receives no history"
    );
    assert_eq!(responses[0].message.result, Err(HistoryError::InvalidRange));
    assert_eq!(world.resource::<PartyAuthority>().snapshot(0), before);
    for request_id in 2..=10 {
        world.write_message(FromClient {
            client_id: ClientId::from(guest),
            message: HistoryRequest {
                request_id,
                ..request
            },
        });
    }
    admission::host_messages(world);
    assert!(
        world.get::<InboundRejected>(guest).is_some()
            || world.get::<ConnectedClient>(guest).is_none()
    );
    assert!(
        world
            .resource_mut::<Messages<ToClients<HistoryReply>>>()
            .drain()
            .next()
            .is_none(),
        "overflow applies no prefix"
    );
    assert_eq!(world.resource::<PartyAuthority>().snapshot(0), before);
    start::close(app(&mut apps, 0).world_mut());
    start::close(app(&mut apps, 1).world_mut());
}
