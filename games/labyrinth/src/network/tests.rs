//! Actual same-machine transport tests. Fake listing is not multicast evidence;
//! replacing an App is not an abrupt operating-system process restart.

use std::{path::Path, thread};

use bevy_game_discovery::{DiscoveryEndpoint, FakeDiscoveryProvider};
use bevy_game_multiplayer::{CredentialStoreError, ReconnectCredentialStore};
use bevy_game_test::TestAppBuilder;
use labyrinth_rules::{ActorId, CombatAction, CombatSnapshot, Effect, HeroClass, StatusKind, Team};

use super::*;

mod process;

// This is an acceptance requirement, deliberately not derived from production
// capacity: accidentally reverting the host to four seats must fail these tests.
const TEST_PLAYERS: usize = 6;
const LAST_GUEST: usize = TEST_PLAYERS - 1;

fn socket_app(path: Option<&Path>) -> App {
    let mut builder = TestAppBuilder::new().with_minimal_plugins();
    let store = path.map_or_else(
        || ReconnectCredentialStorage::new(MemoryReconnectCredentialStore::default()),
        |path| ReconnectCredentialStorage::new(AtomicFileReconnectCredentialStore::new(path)),
    );
    builder
        .app_mut()
        .insert_resource(store)
        .add_plugins(LabyrinthNetworkPlugin);
    builder.build()
}

fn app(apps: &mut [App], index: usize) -> &mut App {
    apps.get_mut(index).expect("test App index exists")
}

fn pump_until(
    apps: &mut [App],
    timeout: Duration,
    mut predicate: impl FnMut(&mut [App]) -> bool,
) -> bool {
    let deadline = Instant::now() + timeout;
    while Instant::now() < deadline {
        for app in &mut *apps {
            app.update();
        }
        if predicate(apps) {
            return true;
        }
        thread::sleep(Duration::from_millis(5));
    }
    false
}

fn open_host(apps: &mut [App], password: &str) {
    let socket = std::net::UdpSocket::bind("127.0.0.1:0").expect("available loopback UDP port");
    let port = socket.local_addr().expect("allocated address").port();
    drop(socket);
    start::host(
        app(apps, 0).world_mut(),
        HostSettings {
            name: "Socket regression party".into(),
            password: SecretText(password.into()),
            address: "127.0.0.1".into(),
            port,
            lan: false,
            tailnet: false,
            seed: 42,
        },
    )
    .expect("asynchronous host preparation starts");
    assert!(
        pump_until(apps, Duration::from_secs(10), |apps| {
            app(apps, 0).world().contains_resource::<Hosted>()
        }),
        "host preparation did not complete"
    );
}

fn host_snapshot(apps: &mut [App]) -> SessionSnapshot {
    app(apps, 0)
        .world()
        .resource::<PartyAuthority>()
        .snapshot(0)
}

fn combat(apps: &mut [App]) -> CombatSnapshot {
    host_snapshot(apps).combat.expect("encounter is running")
}

fn all_admitted(apps: &[App]) -> bool {
    apps.iter()
        .all(|app| app.world().resource::<Runtime>().admitted)
}

fn converged(apps: &mut [App]) -> bool {
    let expected = host_snapshot(apps);
    apps.iter().all(|app| {
        app.world()
            .resource::<Runtime>()
            .latest
            .as_ref()
            .is_some_and(|snapshot| {
                snapshot.combat == expected.combat
                    && snapshot.players == expected.players
                    && snapshot.paused == expected.paused
            })
    })
}

fn join_codes(apps: &mut [App]) -> Vec<String> {
    assert_eq!(apps.len(), TEST_PLAYERS);
    let codes: Vec<_> = (0..LAST_GUEST)
        .map(|index| {
            hosted_code(app(apps, 0).world(), index)
                .expect("host owns five independent invitations")
        })
        .collect();
    let decoded: Vec<_> = codes
        .iter()
        .map(|code| DirectConnectionCode::parse(code).expect("valid BGN1"))
        .collect();
    for (index, first) in decoded.iter().enumerate() {
        for second in decoded.iter().skip(index + 1) {
            assert_ne!(first.invite_token, second.invite_token);
            assert_eq!(first.session_id, second.session_id);
            assert_eq!(first.endpoint, second.endpoint);
            assert_eq!(
                first.certificate_fingerprint,
                second.certificate_fingerprint
            );
        }
    }
    for (index, code) in codes.iter().enumerate() {
        let guest = index + 1;
        start::join_code(app(apps, guest).world_mut(), code)
            .expect("private direct attempt starts");
        assert!(pump_until(apps, Duration::from_secs(10), |apps| {
            app(apps, guest).world().resource::<Runtime>().admitted
        }));
    }
    assert!(
        pump_until(apps, Duration::from_secs(15), |apps| all_admitted(apps)
            && converged(apps)),
        "one host and five clients did not finish real transport admission"
    );
    let slots: BTreeSet<_> = apps
        .iter()
        .filter_map(|app| app.world().resource::<Runtime>().player)
        .collect();
    assert_eq!(slots, BTreeSet::from([0, 1, 2, 3, 4, 5]));
    assert_eq!(
        app(apps, LAST_GUEST).world().resource::<Runtime>().player,
        Some(5),
        "sequential admission must establish the late-seat fixture"
    );
    let peers: BTreeSet<_> = host_snapshot(apps)
        .players
        .iter()
        .filter_map(|player| player.peer)
        .collect();
    assert_eq!(peers.len(), LAST_GUEST);
    let actors: BTreeSet<_> = host_snapshot(apps)
        .players
        .iter()
        .map(|player| player.actor)
        .collect();
    assert_eq!(
        actors.len(),
        TEST_PLAYERS,
        "every player owns a distinct hero"
    );
    assert_eq!(
        app(apps, 0)
            .world()
            .resource::<Hosted>()
            .security
            .reserved_peer_count(),
        LAST_GUEST
    );
    codes
}

fn begin_encounter(apps: &mut [App]) {
    assert_eq!(apps.len(), TEST_PLAYERS);
    for app in apps.iter_mut().take(LAST_GUEST) {
        app.world_mut().write_message(LabyrinthIntent::Ready(true));
    }
    assert!(pump_until(apps, Duration::from_secs(5), |apps| {
        host_snapshot(apps)
            .players
            .iter()
            .filter(|player| player.ready)
            .count()
            == LAST_GUEST
            && converged(apps)
    }));
    app(apps, 0)
        .world_mut()
        .resource_mut::<LabyrinthView>()
        .notice = None;
    app(apps, 0)
        .world_mut()
        .write_message(LabyrinthIntent::StartEncounter);
    assert!(pump_until(apps, Duration::from_secs(5), |apps| {
        app(apps, 0)
            .world()
            .resource::<LabyrinthView>()
            .notice
            .is_some()
    }));
    assert!(
        host_snapshot(apps).combat.is_none(),
        "five ready humans must not start a six-player fight"
    );
    app(apps, LAST_GUEST)
        .world_mut()
        .write_message(LabyrinthIntent::Ready(true));
    assert!(
        pump_until(apps, Duration::from_secs(5), |apps| {
            host_snapshot(apps)
                .players
                .iter()
                .all(|player| player.ready && player.connected)
                && converged(apps)
        }),
        "six independent ready intents did not converge"
    );
    app(apps, 0)
        .world_mut()
        .write_message(LabyrinthIntent::StartEncounter);
    assert!(
        pump_until(apps, Duration::from_secs(5), |apps| {
            host_snapshot(apps).combat.is_some() && converged(apps)
        }),
        "host did not start the six-player encounter"
    );
    let snapshot = combat(apps);
    assert_eq!(snapshot.hero_formation.len(), TEST_PLAYERS);
    assert_eq!(snapshot.enemy_formation.len(), TEST_PLAYERS);
}

fn actor_owner(apps: &[App], actor: ActorId) -> usize {
    let slot = apps
        .first()
        .expect("host exists")
        .world()
        .resource::<PartyAuthority>()
        .snapshot(0)
        .players
        .into_iter()
        .find(|player| player.actor == actor)
        .expect("hero has an authoritative owner")
        .slot;
    apps.iter()
        .position(|app| app.world().resource::<Runtime>().player == Some(slot))
        .expect("hero has an admitted owner")
}

fn wait_for_hero(apps: &mut [App]) -> CombatSnapshot {
    assert!(
        pump_until(apps, Duration::from_secs(5), |apps| {
            let snapshot = combat(apps);
            (snapshot.outcome.is_some()
                || snapshot
                    .active_actor
                    .and_then(|id| snapshot.actor(id))
                    .is_some_and(|actor| actor.team() == Team::Heroes))
                && converged(apps)
        }),
        "enemy progression did not reach a committed hero/terminal boundary"
    );
    combat(apps)
}

fn send_action(apps: &mut [App], actor: ActorId, action: CombatAction) {
    let before = combat(apps).revision;
    let owner = actor_owner(apps, actor);
    let presented = app(apps, owner)
        .world()
        .resource::<Runtime>()
        .latest
        .clone()
        .expect("visible owner snapshot");
    let decision = presented.combat.as_ref().expect("visible combat").turn_id;
    app(apps, owner)
        .world_mut()
        .write_message(LabyrinthIntent::Combat {
            actor,
            action,
            encounter: presented.encounter,
            decision,
        });
    assert!(
        pump_until(apps, Duration::from_secs(5), |apps| {
            combat(apps).revision > before && converged(apps)
        }),
        "owned legal hero action was not accepted and projected"
    );
}

fn aggressive_action(snapshot: &CombatSnapshot, actor: ActorId) -> CombatAction {
    let actions = snapshot.legal_actions(actor);
    actions.iter().copied().find(|action| matches!(action,
        CombatAction::Skill { skill, .. } if labyrinth_rules::skill_definition(*skill).effects.iter()
            .any(|effect| matches!(effect, Effect::Damage(damage) if *damage > 0))))
        .or_else(|| actions.iter().copied().find(|action| matches!(action, CombatAction::Rescue { .. })))
        .unwrap_or(CombatAction::Defend)
}

fn choose_repeated_sixth_class(apps: &mut [App]) {
    let before = host_snapshot(apps);
    let sixth = before
        .players
        .iter()
        .find(|player| player.slot == 5)
        .expect("sixth reservation");
    let actor = sixth.actor;
    assert_ne!(
        sixth.hero,
        HeroClass::Knifehand,
        "fixture exercises a class change"
    );
    assert!(before
        .players
        .iter()
        .any(|player| player.hero == HeroClass::Knifehand));
    app(apps, LAST_GUEST)
        .world_mut()
        .write_message(LabyrinthIntent::SelectHero(HeroClass::Knifehand));
    assert!(
        pump_until(apps, Duration::from_secs(5), |apps| {
            host_snapshot(apps).players.iter().any(|player| {
                player.slot == 5 && player.actor == actor && player.hero == HeroClass::Knifehand
            }) && converged(apps)
        }),
        "sixth player could not independently choose an already-used class"
    );
}

#[test]
fn repeated_class_players_can_only_command_their_own_hero_instance() {
    let mut apps: Vec<_> = (0..TEST_PLAYERS).map(|_| socket_app(None)).collect();
    open_host(&mut apps, "");
    join_codes(&mut apps);
    choose_repeated_sixth_class(&mut apps);
    begin_encounter(&mut apps);
    let players = host_snapshot(&mut apps).players;
    let repeated: Vec<_> = players
        .iter()
        .filter(|player| player.hero == HeroClass::Knifehand)
        .collect();
    assert!(repeated.len() >= 2);
    let mut before = wait_for_hero(&mut apps);
    for _ in 0..TEST_PLAYERS {
        if before
            .active_actor
            .is_some_and(|actor| repeated.iter().any(|player| player.actor == actor))
        {
            break;
        }
        let actor = before
            .active_actor
            .expect("hero decision before repeated class");
        send_action(&mut apps, actor, CombatAction::Wait);
        before = wait_for_hero(&mut apps);
    }
    let actor = before.active_actor.expect("repeated class decision");
    let owner = repeated
        .iter()
        .find(|player| player.actor == actor)
        .expect("active hero belongs to repeated class");
    let other = repeated
        .iter()
        .find(|player| player.actor != actor)
        .expect("same class, distinct hero instance");
    assert_ne!(owner.slot, other.slot);
    let wrong = actor_owner(&apps, other.actor);
    let encounter = host_snapshot(&mut apps).encounter;
    app(&mut apps, wrong)
        .world_mut()
        .write_message(LabyrinthIntent::Combat {
            actor,
            action: CombatAction::Wait,
            encounter,
            decision: before.turn_id,
        });
    assert!(pump_until(&mut apps, Duration::from_secs(5), |apps| {
        app(apps, wrong)
            .world()
            .resource::<LabyrinthView>()
            .notice
            .as_ref()
            .is_some_and(|notice| notice.contains("not your hero"))
    }));
    assert_eq!(
        combat(&mut apps),
        before,
        "shared class granted another player's authority"
    );
    send_action(&mut apps, actor, CombatAction::Wait);
}

fn stored(app: &App) -> StoredReconnectCredential {
    app.world()
        .resource::<ReconnectCredentialStorage>()
        .store()
        .load()
        .expect("credential store reads")
        .expect("credential persisted before ACK")
}

fn wait_guest_detached(apps: &mut [App], peer: PeerId) {
    assert!(
        pump_until(apps, Duration::from_secs(10), |apps| {
            let world = app(apps, 0).world();
            let host = world.resource::<Hosted>();
            !host.connections.iter().any(|(entity, (other, _))| {
                *other == peer
                    && host
                        .security
                        .peer_for_connection(entity.to_bits())
                        .is_some()
            })
        }),
        "host did not observe guest transport disconnection"
    );
}

#[test]
fn real_udp_six_players_reject_seventh_and_wrong_ownership_then_finish_encounter() {
    let mut apps: Vec<_> = (0..TEST_PLAYERS).map(|_| socket_app(None)).collect();
    open_host(&mut apps, "");
    let codes = join_codes(&mut apps);

    // A consumed code does not allocate a sixth peer or steal the first identity.
    apps.push(socket_app(None));
    start::join_code(
        app(&mut apps, TEST_PLAYERS).world_mut(),
        codes.first().expect("first code"),
    )
    .expect("duplicate code attempts transport");
    assert!(pump_until(&mut apps, Duration::from_secs(5), |apps| {
        app(apps, TEST_PLAYERS)
            .world()
            .resource::<LabyrinthView>()
            .notice
            .as_ref()
            .is_some_and(|notice| notice.contains("refused") || notice.contains("full"))
    }));
    assert!(
        !app(&mut apps, TEST_PLAYERS)
            .world()
            .resource::<Runtime>()
            .admitted
    );
    assert_eq!(
        app(&mut apps, 0)
            .world()
            .resource::<Hosted>()
            .security
            .reserved_peer_count(),
        LAST_GUEST
    );
    assert_eq!(
        host_snapshot(&mut apps)
            .players
            .iter()
            .filter(|player| player.occupied)
            .count(),
        TEST_PLAYERS
    );

    // Even a fresh valid bearer invitation cannot exceed game-owned capacity.
    let extra_code = {
        let world = app(&mut apps, 0).world_mut();
        let clock = now(world);
        let mut host = world.resource_mut::<Hosted>();
        let token = host
            .security
            .issue_invite(clock, clock + Duration::from_secs(60))
            .expect("independent bounded invitation");
        let mut code = host.template.clone();
        code.invite_token = token;
        code.encode().expose_for_sharing().to_owned()
    };
    *app(&mut apps, TEST_PLAYERS) = socket_app(None);
    start::join_code(app(&mut apps, TEST_PLAYERS).world_mut(), &extra_code)
        .expect("valid seventh invite attempts transport");
    assert!(pump_until(&mut apps, Duration::from_secs(5), |apps| {
        app(apps, TEST_PLAYERS)
            .world()
            .resource::<LabyrinthView>()
            .notice
            .as_ref()
            .is_some_and(|notice| notice.contains("full"))
    }));
    assert_eq!(
        app(&mut apps, 0)
            .world()
            .resource::<Hosted>()
            .security
            .reserved_peer_count(),
        LAST_GUEST
    );
    drop(apps.pop());
    begin_encounter(&mut apps);

    let expected_actors: BTreeSet<_> = host_snapshot(&mut apps)
        .players
        .iter()
        .map(|player| player.actor)
        .collect();
    let before = wait_for_hero(&mut apps);
    let actor = before.active_actor.expect("first hero decision");
    let owner = actor_owner(&apps, actor);
    let wrong = (owner + 1) % apps.len();
    let encounter = host_snapshot(&mut apps).encounter;
    app(&mut apps, wrong)
        .world_mut()
        .write_message(LabyrinthIntent::Combat {
            actor,
            action: CombatAction::Wait,
            encounter,
            decision: before.turn_id,
        });
    assert!(pump_until(&mut apps, Duration::from_secs(5), |apps| {
        app(apps, wrong)
            .world()
            .resource::<LabyrinthView>()
            .notice
            .as_ref()
            .is_some_and(|notice| notice.contains("not your hero"))
    }));
    assert_eq!(
        combat(&mut apps),
        before,
        "unauthorized action changed combat"
    );

    let mut acted = BTreeSet::new();
    let deadline = Instant::now() + Duration::from_secs(30);
    for _ in 0..160 {
        let snapshot = wait_for_hero(&mut apps);
        if snapshot.outcome.is_some() {
            break;
        }
        assert!(
            Instant::now() < deadline,
            "authored encounter did not terminate within the test budget"
        );
        let actor = snapshot.active_actor.expect("hero decision");
        acted.insert(actor);
        send_action(&mut apps, actor, aggressive_action(&snapshot, actor));
    }
    assert!(
        combat(&mut apps).outcome.is_some(),
        "six players did not finish an encounter"
    );
    assert_eq!(acted, expected_actors);
    assert!(converged(&mut apps));
}

#[test]
fn queued_old_ui_intent_cannot_be_reinterpreted_as_the_same_heros_next_turn() {
    let mut apps: Vec<_> = (0..TEST_PLAYERS).map(|_| socket_app(None)).collect();
    open_host(&mut apps, "");
    join_codes(&mut apps);
    begin_encounter(&mut apps);
    let before = wait_for_hero(&mut apps);
    let actor = before.active_actor.expect("initial hero decision");
    let owner = actor_owner(&apps, actor);
    let encounter = app(&mut apps, owner)
        .world()
        .resource::<Runtime>()
        .latest
        .as_ref()
        .expect("visible encounter")
        .encounter;
    let old_intent = LabyrinthIntent::Combat {
        actor,
        action: CombatAction::Wait,
        encounter,
        decision: before.turn_id,
    };
    send_action(&mut apps, actor, CombatAction::Wait);
    let mut current = wait_for_hero(&mut apps);
    for _ in 0..24 {
        if current.active_actor == Some(actor) {
            break;
        }
        let other = current
            .active_actor
            .expect("hero while waiting for next original turn");
        send_action(&mut apps, other, CombatAction::Wait);
        current = wait_for_hero(&mut apps);
    }
    assert_eq!(current.active_actor, Some(actor));
    assert_ne!(current.turn_id, before.turn_id);
    app(&mut apps, owner)
        .world_mut()
        .resource_mut::<LabyrinthView>()
        .notice = None;
    app(&mut apps, owner).world_mut().write_message(old_intent);
    assert!(
        pump_until(&mut apps, Duration::from_secs(5), |apps| {
            app(apps, owner)
                .world()
                .resource::<LabyrinthView>()
                .notice
                .is_some()
        }),
        "stale UI intent was not explicitly rejected"
    );
    assert_eq!(
        combat(&mut apps),
        current,
        "an old click was applied to the hero's new turn"
    );
    send_action(&mut apps, actor, CombatAction::Wait);
}

#[test]
fn real_udp_fresh_sixth_guest_restores_actor_class_loadout_and_live_combat() {
    let directory = tempfile::tempdir().expect("isolated profiles");
    let paths: Vec<_> = (0..LAST_GUEST)
        .map(|index| directory.path().join(format!("guest-{index}.json")))
        .collect();
    let mut apps = vec![socket_app(None)];
    apps.extend(paths.iter().map(|path| socket_app(Some(path))));
    open_host(&mut apps, "");
    join_codes(&mut apps);
    choose_repeated_sixth_class(&mut apps);
    begin_encounter(&mut apps);

    let mut before = wait_for_hero(&mut apps);
    for _ in 0..24 {
        if before.actors.iter().any(|actor| {
            actor
                .statuses
                .iter()
                .any(|status| status.kind == StatusKind::Bleed)
        }) {
            break;
        }
        let actor = before
            .active_actor
            .expect("hero decision before bleed fixture");
        let action = before
            .legal_actions(actor)
            .into_iter()
            .find(|action| {
                matches!(
                    action,
                    CombatAction::Skill {
                        skill: labyrinth_rules::SkillId::BleedingCut,
                        ..
                    }
                )
            })
            .unwrap_or(CombatAction::Wait);
        send_action(&mut apps, actor, action);
        before = wait_for_hero(&mut apps);
    }
    assert!(
        before.actors.iter().any(|actor| actor
            .statuses
            .iter()
            .any(|status| status.kind == StatusKind::Bleed)),
        "real actions never established the live bleed reconnect fixture"
    );
    let original = stored(app(&mut apps, LAST_GUEST));
    let others: Vec<_> = apps
        .iter()
        .skip(1)
        .take(LAST_GUEST - 1)
        .map(stored)
        .collect();
    let slot = app(&mut apps, LAST_GUEST)
        .world()
        .resource::<Runtime>()
        .player;
    assert_eq!(slot, Some(5));
    let owned = host_snapshot(&mut apps)
        .players
        .into_iter()
        .find(|player| Some(player.slot) == slot)
        .expect("sixth owned hero");
    let owned_actor = before
        .actor(owned.actor)
        .expect("sixth hero in combat")
        .clone();
    assert_eq!(owned.hero, HeroClass::Knifehand);
    assert_eq!(owned_actor.abilities, owned.abilities);
    // Drop the live App without a Leave message or graceful disconnect helper.
    // The host must observe actual transport loss; the replacement knows only its file.
    *app(&mut apps, LAST_GUEST) = socket_app(paths.last().map(std::path::PathBuf::as_path));
    wait_guest_detached(&mut apps, original.peer_id);
    assert!(host_snapshot(&mut apps).paused);
    assert_eq!(
        combat(&mut apps),
        before,
        "disconnect progressed a combat boundary"
    );
    start::reconnect(app(&mut apps, LAST_GUEST).world_mut())
        .expect("fresh App reads profile and starts reconnect");
    assert!(
        pump_until(&mut apps, Duration::from_secs(10), |apps| all_admitted(
            apps
        ) && converged(
            apps
        )),
        "fresh guest did not reclaim the running host's reserved state"
    );
    let recovered = stored(app(&mut apps, LAST_GUEST));
    assert_eq!(recovered.peer_id, original.peer_id);
    assert_ne!(
        recovered.reconnect_credential,
        original.reconnect_credential
    );
    assert_eq!(
        app(&mut apps, LAST_GUEST)
            .world()
            .resource::<Runtime>()
            .player,
        slot
    );
    let recovered_player = host_snapshot(&mut apps)
        .players
        .into_iter()
        .find(|player| Some(player.slot) == slot)
        .expect("recovered sixth reservation");
    assert_eq!(
        recovered_player, owned,
        "restart changed hero ownership, class or loadout"
    );
    assert_eq!(
        combat(&mut apps).actor(owned.actor),
        Some(&owned_actor),
        "restart changed sixth hero HP, equipped abilities or status state"
    );
    assert_eq!(
        combat(&mut apps),
        before,
        "reconnect re-entered a bleed/initiative boundary"
    );
    assert!(!host_snapshot(&mut apps).paused);
    let recovered_others: Vec<_> = apps
        .iter()
        .skip(1)
        .take(LAST_GUEST - 1)
        .map(stored)
        .collect();
    assert_eq!(
        recovered_others, others,
        "sixth guest rotation changed another player's credential"
    );
}

#[test]
fn real_udp_duplicate_and_evicted_replays_never_repeat_an_action() {
    let mut apps: Vec<_> = (0..TEST_PLAYERS).map(|_| socket_app(None)).collect();
    open_host(&mut apps, "");
    join_codes(&mut apps);
    begin_encounter(&mut apps);
    let mut before = wait_for_hero(&mut apps);
    for _ in 0..8 {
        let actor = before.active_actor.expect("hero decision");
        if actor_owner(&apps, actor) != 0 {
            break;
        }
        send_action(&mut apps, actor, CombatAction::Wait);
        before = wait_for_hero(&mut apps);
    }
    let actor = before.active_actor.expect("guest decision");
    let owner = actor_owner(&apps, actor);
    assert_ne!(owner, 0, "fixture must exercise a remote sender");
    // Freeze enemy presentation pacing only: commands still traverse the production
    // protocol, authorization, host reducer and target snapshot pipeline.
    app(&mut apps, 0)
        .world_mut()
        .resource_mut::<Runtime>()
        .next_enemy = Instant::now() + Duration::from_secs(60);
    let snapshot = host_snapshot(&mut apps);
    let runtime = app(&mut apps, owner).world().resource::<Runtime>();
    let slot = runtime.player.expect("remote slot");
    let request = GameRequest {
        sequence: runtime.sequence,
        encounter: snapshot.encounter,
        decision: before.turn_id,
        command: SessionCommand::Act {
            actor,
            action: CombatAction::Wait,
        },
    };
    app(&mut apps, owner)
        .world_mut()
        .write_message(request.clone());
    app(&mut apps, owner)
        .world_mut()
        .write_message(request.clone());
    assert!(pump_until(
        &mut apps,
        Duration::from_secs(5),
        |apps| combat(apps).revision > before.revision && converged(apps)
    ));
    let after = combat(&mut apps);
    assert_eq!(
        after.revision,
        before.revision + 1,
        "duplicate command applied twice"
    );
    // Consume more than the bounded result cache with stale-turn commands. They
    // are rejected but still advance the per-peer watermark, without combat mutation.
    for sequence in request.sequence + 1..=request.sequence + 70 {
        app(&mut apps, owner)
            .world_mut()
            .write_message(GameRequest {
                sequence,
                ..request.clone()
            });
        assert!(pump_until(&mut apps, Duration::from_secs(3), |apps| {
            app(apps, 0)
                .world()
                .resource::<PartyAuthority>()
                .next_sequence(slot)
                > sequence
        }));
    }
    assert_eq!(combat(&mut apps), after);
    app(&mut apps, owner)
        .world_mut()
        .resource_mut::<LabyrinthView>()
        .notice = None;
    app(&mut apps, owner).world_mut().write_message(request);
    assert!(pump_until(&mut apps, Duration::from_secs(3), |apps| {
        app(apps, owner)
            .world()
            .resource::<LabyrinthView>()
            .notice
            .as_ref()
            .is_some_and(|notice| notice.contains("Stale request"))
    }));
    assert_eq!(
        combat(&mut apps),
        after,
        "cache eviction made an old request executable again"
    );
}

fn publish_fake_listing(app: &mut App, metadata: SessionMetadata, target: DiscoveredDirectTarget) {
    discovery::browse_fake(app.world_mut());
    let mut provider = FakeDiscoveryProvider::default();
    provider.publish(metadata, target, Duration::from_secs(600));
    for observation in provider.drain() {
        app.world_mut().write_message(observation);
    }
}

#[test]
fn fake_discovery_hands_five_password_joins_to_real_pinned_transport() {
    let mut apps: Vec<_> = (0..TEST_PLAYERS).map(|_| socket_app(None)).collect();
    open_host(&mut apps, "temporary-party-pass");
    let (metadata, target) = {
        let world = app(&mut apps, 0).world();
        let host = world.resource::<Hosted>();
        (
            host.metadata.clone(),
            DiscoveredDirectTarget {
                session_id: host.template.session_id,
                endpoint: host.template.endpoint.clone(),
                certificate_fingerprint: host.template.certificate_fingerprint,
                certificate_expires_unix_seconds: host.template.certificate_expires_unix_seconds,
            },
        )
    };
    assert_eq!(metadata.player_capacity(), 6);
    assert_eq!(metadata.claimed_players(), 1);
    for guest in apps.iter_mut().skip(1) {
        publish_fake_listing(guest, metadata.clone(), target.clone());
    }
    assert!(
        pump_until(&mut apps, Duration::from_secs(5), |apps| apps
            .iter()
            .skip(1)
            .all(|guest| {
                guest
                    .world()
                    .resource::<LabyrinthView>()
                    .listings
                    .iter()
                    .any(|listing| listing.id == target.session_id && listing.compatible)
            })),
        "fake provider did not drive the production browser projection"
    );
    start::join_discovered(
        app(&mut apps, 1).world_mut(),
        target.session_id,
        "incorrect-temporary-pass".into(),
    )
    .expect("wrong password still uses the pinned transport");
    assert!(
        pump_until(&mut apps, Duration::from_secs(10), |apps| {
            app(apps, 1)
                .world()
                .resource::<LabyrinthView>()
                .notice
                .as_ref()
                .is_some_and(|notice| notice.contains("refused admission"))
        }),
        "wrong password did not produce a generic admission refusal"
    );
    assert!(!app(&mut apps, 1).world().resource::<Runtime>().admitted);
    assert_eq!(
        app(&mut apps, 0)
            .world()
            .resource::<Hosted>()
            .security
            .reserved_peer_count(),
        0
    );
    assert_eq!(
        host_snapshot(&mut apps)
            .players
            .iter()
            .filter(|player| player.occupied)
            .count(),
        1
    );
    for guest in apps.iter_mut().skip(1) {
        start::join_discovered(
            guest.world_mut(),
            target.session_id,
            "temporary-party-pass".into(),
        )
        .expect("provider-neutral selected join");
    }
    assert!(
        pump_until(&mut apps, Duration::from_secs(15), |apps| all_admitted(
            apps
        ) && converged(
            apps
        )),
        "five discovered/password guests did not finish real admission"
    );
    assert_eq!(
        host_snapshot(&mut apps)
            .players
            .iter()
            .filter(|player| player.connected)
            .count(),
        TEST_PLAYERS
    );
    assert_eq!(
        app(&mut apps, 0)
            .world()
            .resource::<Hosted>()
            .security
            .reserved_peer_count(),
        LAST_GUEST
    );
    assert_eq!(
        app(&mut apps, 0)
            .world()
            .resource::<Hosted>()
            .metadata
            .claimed_players(),
        6
    );
    begin_encounter(&mut apps);
    let snapshot = wait_for_hero(&mut apps);
    let actor = snapshot.active_actor.expect("hero decision");
    send_action(&mut apps, actor, CombatAction::Defend);

    // A listing removal affects discovery, never the authenticated live session.
    for guest in apps.iter_mut().skip(1) {
        let mut provider = FakeDiscoveryProvider::default();
        provider.remove(target.session_id);
        for observation in provider.drain() {
            guest.world_mut().write_message(observation);
        }
    }
    assert!(pump_until(&mut apps, Duration::from_secs(5), |apps| apps
        .iter()
        .skip(1)
        .all(|guest| {
            guest
                .world()
                .resource::<DiscoveryRegistry>()
                .resolve(target.session_id)
                .is_err()
        })));
    assert!(all_admitted(&apps));
}

#[derive(Resource, Default)]
struct DropHandshake {
    offers: bool,
    acknowledgements: bool,
    dropped_offers: usize,
    dropped_acknowledgements: usize,
}

fn discard_offers(mut messages: ResMut<Messages<Offer>>, mut fault: ResMut<DropHandshake>) {
    if fault.offers {
        fault.dropped_offers += messages.drain().count();
    }
}

fn discard_acknowledgements(
    mut messages: ResMut<Messages<Persisted>>,
    mut fault: ResMut<DropHandshake>,
) {
    if fault.acknowledgements {
        fault.dropped_acknowledgements += messages.drain().count();
    }
}

fn install_handshake_faults(app: &mut App) {
    app.init_resource::<DropHandshake>()
        .add_systems(
            PreUpdate,
            discard_offers
                .after(MultiplayerSystems::Receive)
                .before(MultiplayerSystems::GameAuthority),
        )
        .add_systems(
            PostUpdate,
            discard_acknowledgements
                .after(MultiplayerSystems::Send)
                .before(ClientSystems::Send),
        );
}

#[test]
fn real_handshake_offer_and_ack_loss_recover_one_peer_from_code_then_profile() {
    let directory = tempfile::tempdir().expect("profile directory");
    let path = directory.path().join("guest.json");
    let mut apps = vec![socket_app(None), socket_app(Some(&path))];
    install_handshake_faults(app(&mut apps, 1));
    app(&mut apps, 1)
        .world_mut()
        .resource_mut::<DropHandshake>()
        .offers = true;
    open_host(&mut apps, "");
    let code = hosted_code(app(&mut apps, 0).world(), 0).expect("invitation");
    start::join_code(app(&mut apps, 1).world_mut(), &code).expect("initial join");
    assert!(
        pump_until(&mut apps, Duration::from_secs(10), |apps| {
            app(apps, 1)
                .world()
                .resource::<DropHandshake>()
                .dropped_offers
                > 0
        }),
        "offer was not observed at the real client receive boundary"
    );
    assert!(app(&mut apps, 1)
        .world()
        .resource::<ReconnectCredentialStorage>()
        .store()
        .load()
        .expect("profile read")
        .is_none());
    let peer = *host_snapshot(&mut apps)
        .players
        .iter()
        .filter_map(|player| player.peer.as_ref())
        .next()
        .expect("pending reserved peer");
    assert!(!app(&mut apps, 1).world().resource::<Runtime>().admitted);
    start::disconnect_guest(app(&mut apps, 1).world_mut());
    wait_guest_detached(&mut apps, peer);
    {
        let mut fault = app(&mut apps, 1)
            .world_mut()
            .resource_mut::<DropHandshake>();
        fault.offers = false;
        fault.acknowledgements = true;
    }
    start::join_code(app(&mut apps, 1).world_mut(), &code)
        .expect("same invitation recovers lost offer");
    assert!(
        pump_until(&mut apps, Duration::from_secs(10), |apps| {
            app(apps, 1)
                .world()
                .resource::<DropHandshake>()
                .dropped_acknowledgements
                > 0
        }),
        "persistence ACK was not observed before actual transport send"
    );
    let persisted = stored(app(&mut apps, 1));
    assert_eq!(persisted.peer_id, peer);
    assert!(!app(&mut apps, 1).world().resource::<Runtime>().admitted);
    start::disconnect_guest(app(&mut apps, 1).world_mut());
    wait_guest_detached(&mut apps, peer);
    *app(&mut apps, 1) = socket_app(Some(&path));
    start::reconnect(app(&mut apps, 1).world_mut())
        .expect("fresh process-style App uses persisted pending credential");
    assert!(pump_until(
        &mut apps,
        Duration::from_secs(10),
        |apps| all_admitted(apps) && converged(apps)
    ));
    assert_eq!(stored(app(&mut apps, 1)).peer_id, peer);
    assert_eq!(
        app(&mut apps, 0)
            .world()
            .resource::<Hosted>()
            .security
            .reserved_peer_count(),
        1
    );

    // Repeat the lost-ACK window during actual rotation of an established peer.
    let before = stored(app(&mut apps, 1));
    install_handshake_faults(app(&mut apps, 1));
    app(&mut apps, 1)
        .world_mut()
        .resource_mut::<DropHandshake>()
        .acknowledgements = true;
    start::disconnect_guest(app(&mut apps, 1).world_mut());
    wait_guest_detached(&mut apps, peer);
    start::reconnect(app(&mut apps, 1).world_mut()).expect("rotation attempt");
    assert!(pump_until(&mut apps, Duration::from_secs(10), |apps| {
        app(apps, 1)
            .world()
            .resource::<DropHandshake>()
            .dropped_acknowledgements
            > 0
    }));
    let pending = stored(app(&mut apps, 1));
    assert_ne!(pending.reconnect_credential, before.reconnect_credential);
    start::disconnect_guest(app(&mut apps, 1).world_mut());
    wait_guest_detached(&mut apps, peer);
    *app(&mut apps, 1) = socket_app(Some(&path));
    start::reconnect(app(&mut apps, 1).world_mut()).expect("persisted successor survives ACK loss");
    assert!(pump_until(
        &mut apps,
        Duration::from_secs(10),
        |apps| all_admitted(apps) && converged(apps)
    ));
    assert_eq!(
        stored(app(&mut apps, 1)),
        pending,
        "retry rotated an already-persisted pending successor again"
    );
    assert_eq!(
        app(&mut apps, 0)
            .world()
            .resource::<Hosted>()
            .security
            .reserved_peer_count(),
        1
    );
}

struct UnavailableStore;
impl ReconnectCredentialStore for UnavailableStore {
    fn load(&self) -> Result<Option<StoredReconnectCredential>, CredentialStoreError> {
        Ok(None)
    }
    fn store_atomically(&self, _: StoredReconnectCredential) -> Result<(), CredentialStoreError> {
        Err(CredentialStoreError::Unavailable)
    }
    fn delete_if_session(&self, _: SessionId) -> Result<bool, CredentialStoreError> {
        Ok(false)
    }
    fn delete_if_expired(&self, _: u64) -> Result<bool, CredentialStoreError> {
        Ok(false)
    }
}

#[test]
fn failed_profile_persistence_never_admits_and_original_invite_can_retry() {
    let mut apps = vec![socket_app(None), socket_app(None)];
    app(&mut apps, 1)
        .world_mut()
        .insert_resource(ReconnectCredentialStorage::new(UnavailableStore));
    open_host(&mut apps, "");
    let code = hosted_code(app(&mut apps, 0).world(), 0).expect("invitation");
    start::join_code(app(&mut apps, 1).world_mut(), &code).expect("join with unavailable profile");
    assert!(pump_until(&mut apps, Duration::from_secs(10), |apps| {
        app(apps, 1)
            .world()
            .resource::<LabyrinthView>()
            .notice
            .as_ref()
            .is_some_and(|notice| notice.contains("save reconnect credentials"))
    }));
    let peer = *host_snapshot(&mut apps)
        .players
        .iter()
        .filter_map(|player| player.peer.as_ref())
        .next()
        .expect("pending reservation");
    wait_guest_detached(&mut apps, peer);
    assert!(!app(&mut apps, 1).world().resource::<Runtime>().admitted);
    assert!(host_snapshot(&mut apps)
        .players
        .iter()
        .filter(|player| player.slot != 0)
        .all(|player| !player.connected));
    app(&mut apps, 1)
        .world_mut()
        .insert_resource(ReconnectCredentialStorage::new(
            MemoryReconnectCredentialStore::default(),
        ));
    start::join_code(app(&mut apps, 1).world_mut(), &code)
        .expect("original invitation retries after fixing storage");
    assert!(pump_until(
        &mut apps,
        Duration::from_secs(10),
        |apps| all_admitted(apps) && converged(apps)
    ));
    assert_eq!(stored(app(&mut apps, 1)).peer_id, peer);
    assert_eq!(
        app(&mut apps, 0)
            .world()
            .resource::<Hosted>()
            .security
            .reserved_peer_count(),
        1
    );
}

#[test]
fn incompatible_and_opaque_service_listings_do_not_open_direct_transport() {
    let mut apps = vec![socket_app(None), socket_app(None)];
    open_host(&mut apps, "temporary-party-pass");
    let target = {
        let world = app(&mut apps, 0).world();
        let host = world.resource::<Hosted>();
        DiscoveredDirectTarget {
            session_id: host.template.session_id,
            endpoint: host.template.endpoint.clone(),
            certificate_fingerprint: host.template.certificate_fingerprint,
            certificate_expires_unix_seconds: host.template.certificate_expires_unix_seconds,
        }
    };
    let incompatible = SessionMetadata::new(
        GAME_ID,
        "different",
        fingerprint_text(),
        "Other rules",
        1,
        6,
        true,
    )
    .expect("public metadata");
    publish_fake_listing(app(&mut apps, 1), incompatible, target.clone());
    assert!(pump_until(&mut apps, Duration::from_secs(3), |apps| !app(
        apps, 1
    )
    .world()
    .resource::<LabyrinthView>()
    .listings
    .is_empty()));
    assert!(start::join_discovered(
        app(&mut apps, 1).world_mut(),
        target.session_id,
        "temporary-party-pass".into()
    )
    .is_err());
    assert!(app(&mut apps, 1)
        .world()
        .resource::<Runtime>()
        .connection
        .is_none());
    let metadata = app(&mut apps, 0)
        .world()
        .resource::<Hosted>()
        .metadata
        .clone();
    let mut provider = FakeDiscoveryProvider::default();
    provider.publish(
        metadata,
        DiscoveryEndpoint::Service {
            session_id: target.session_id,
            locator: "opaque-test-lobby".into(),
        },
        Duration::from_secs(600),
    );
    for observation in provider.drain() {
        app(&mut apps, 1).world_mut().write_message(observation);
    }
    assert!(pump_until(&mut apps, Duration::from_secs(3), |apps| {
        app(apps, 1)
            .world()
            .resource::<LabyrinthView>()
            .listings
            .iter()
            .any(|listing| listing.compatible)
    }));
    assert!(start::join_discovered(
        app(&mut apps, 1).world_mut(),
        target.session_id,
        "temporary-party-pass".into()
    )
    .is_err());
    assert!(app(&mut apps, 1)
        .world()
        .resource::<Runtime>()
        .connection
        .is_none());
    assert_eq!(
        app(&mut apps, 0)
            .world()
            .resource::<Hosted>()
            .security
            .reserved_peer_count(),
        0
    );
}

#[test]
fn old_attempt_packets_cannot_admit_overwrite_or_disconnect_a_new_reconnect() {
    let mut apps = vec![socket_app(None), socket_app(None)];
    open_host(&mut apps, "");
    let code = hosted_code(app(&mut apps, 0).world(), 0).expect("invitation");
    start::join_code(app(&mut apps, 1).world_mut(), &code).expect("initial join");
    assert!(pump_until(
        &mut apps,
        Duration::from_secs(10),
        |apps| all_admitted(apps) && converged(apps)
    ));
    let before = stored(app(&mut apps, 1));
    let old_attempt = app(&mut apps, 1)
        .world()
        .resource::<Runtime>()
        .attempt
        .expect("old attempt");
    let mut old_snapshot = app(&mut apps, 1)
        .world()
        .resource::<Runtime>()
        .latest
        .clone()
        .expect("initial state");
    old_snapshot.revision = u64::MAX;
    old_snapshot
        .log
        .push("This stale projection must never appear.".into());
    start::disconnect_guest(app(&mut apps, 1).world_mut());
    wait_guest_detached(&mut apps, before.peer_id);
    start::reconnect(app(&mut apps, 1).world_mut()).expect("new physical attempt starts");
    let connection = app(&mut apps, 1)
        .world()
        .resource::<Runtime>()
        .connection
        .expect("new physical connection");
    assert_ne!(
        app(&mut apps, 1).world().resource::<Runtime>().attempt,
        Some(old_attempt)
    );
    let world = app(&mut apps, 1).world_mut();
    world.write_message(Offer {
        session: before.session_id,
        attempt: old_attempt,
        peer: before.peer_id,
        slot: 3,
        credential: ReconnectCredential::from_bytes([88; 32]),
        reconnected: true,
    });
    world.write_message(Admitted {
        session: before.session_id,
        attempt: old_attempt,
        peer: before.peer_id,
        slot: 3,
        reconnected: true,
    });
    world.write_message(SnapshotEnvelope {
        attempt: old_attempt,
        snapshot: old_snapshot,
    });
    world.write_message(Refusal {
        attempt: old_attempt,
        reason: Refused::Admission,
    });
    world.write_message(Closed {
        attempt: old_attempt,
    });
    // Exercise the real receive implementation at a controlled message boundary;
    // the newly opened socket has not yet had a chance to admit legitimately.
    receive(world);
    let runtime = world.resource::<Runtime>();
    assert_eq!(runtime.connection, Some(connection));
    assert!(!runtime.admitted);
    assert_eq!(runtime.player, None);
    assert!(runtime.latest.is_none());
    assert!(world.resource::<Messages<Persisted>>().is_empty());
    assert_eq!(
        stored(app(&mut apps, 1)),
        before,
        "old offer overwrote the profile credential"
    );
    assert!(
        pump_until(&mut apps, Duration::from_secs(10), |apps| all_admitted(
            apps
        ) && converged(
            apps
        )),
        "stale refusal/close interrupted the real successor connection"
    );
    assert_eq!(stored(app(&mut apps, 1)).peer_id, before.peer_id);
    assert_ne!(
        app(&mut apps, 1)
            .world()
            .resource::<Runtime>()
            .latest
            .as_ref()
            .expect("new state")
            .revision,
        u64::MAX
    );
}
