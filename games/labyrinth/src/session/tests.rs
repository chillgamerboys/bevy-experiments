//! Game-owned policy tests. These do not claim transport or process-restart evidence.

use super::*;
use labyrinth_rules::{CombatOutcome, Team, DEFAULT_HERO_ROSTER};

#[test]
fn formation_size_never_shrinks_participant_capacity_or_evicts_reservations() {
    let mut authority = PartyAuthority::new(42, false);
    assert_eq!(authority.company.len(), 5);
    assert_eq!(authority.capacity(), 6);
    for slot in 1..PLAYER_CAPACITY {
        let identity = peer(slot);
        assert_eq!(authority.reserve(identity), Ok(slot));
        authority.connected(identity, true);
    }
    let original = authority.snapshot(0);
    assert!(original
        .player_views()
        .iter()
        .skip(1)
        .all(|p| p.actors.is_empty()));
    assert_eq!(
        request(
            &mut authority,
            0,
            SessionCommand::ChooseHero {
                actor: ActorId(5),
                hero: HeroClass::Gatekeeper,
            }
        )
        .rejection,
        None
    );
    assert_eq!(authority.capacity(), 6);
    assert_eq!(
        request(
            &mut authority,
            0,
            SessionCommand::ChooseHero {
                actor: ActorId(5),
                hero: HeroClass::LanternWagon,
            }
        )
        .rejection,
        None
    );
    let restored = authority.snapshot(0);
    assert_eq!(restored.players, original.players);
    assert_eq!(restored.company, original.company);
    assert_eq!(restored.validate_successor(&original), Ok(()));
    restored
        .validate()
        .expect("six participants independent of five heroes");
    for slot in 0..PLAYER_CAPACITY {
        assert_eq!(
            request(&mut authority, slot, SessionCommand::Ready(true)).rejection,
            None
        );
    }
    assert_eq!(
        request(&mut authority, 0, SessionCommand::Start).rejection,
        None
    );
    let before = authority.snapshot(0);
    assert_eq!(
        before.combat.as_ref().expect("combat").ranks(ActorId(5)),
        Some(5..=6)
    );
    authority.connected(peer(5), false);
    assert!(!authority.paused(), "spectators do not suspend combat");
    authority.connected(peer(5), true);
    assert_eq!(authority.snapshot(0).combat, before.combat);
}

#[test]
fn local_prototype_has_four_ordinary_heroes_and_one_weak_wagon() {
    let mut a = PartyAuthority::new(42, true);
    let hosted = PartyAuthority::new(42, false);
    assert_eq!(
        hosted.company.iter().map(|p| p.hero).collect::<Vec<_>>(),
        labyrinth_rules::PROTOTYPE_HERO_ROSTER
    );
    assert_eq!(
        a.company.iter().map(|p| p.hero).collect::<Vec<_>>(),
        labyrinth_rules::PROTOTYPE_HERO_ROSTER
    );
    assert!(request(&mut a, 0, SessionCommand::Start)
        .rejection
        .is_none());
    let snapshot = a.snapshot(0);
    snapshot.validate().expect("local prototype");
    let combat = snapshot.combat.expect("combat");
    let wagon = combat.actor(ActorId(5)).expect("wagon");
    assert_eq!(wagon.kind, ActorKind::Hero(HeroClass::LanternWagon));
    assert_eq!(
        wagon.skills(),
        &[
            labyrinth_rules::SkillId::HurledScrap,
            labyrinth_rules::SkillId::SpareBandage
        ]
    );
    assert_eq!(combat.ranks(wagon.id), Some(5..=6));
}

#[test]
fn multiple_disconnects_and_faults_have_distinct_validated_suspension_reasons() {
    let (mut authority, peers) = started_party();
    let a = *peers.first().expect("first guest");
    let b = *peers.get(1).expect("second guest");
    authority.connected(a, false);
    authority.connected(b, false);
    assert_eq!(
        authority.snapshot(0).interruption,
        CombatInterruption::WaitingForPlayers
    );
    authority.connected(a, true);
    assert!(
        authority.paused(),
        "one remaining disconnect still suspends combat"
    );
    authority.faulted = true;
    authority.connected(b, true);
    assert_eq!(
        authority.snapshot(0).interruption,
        CombatInterruption::Halted
    );
    assert!(!authority.advance_enemy());
    let mut invalid = authority.snapshot(0);
    invalid.interruption = CombatInterruption::Reconnecting;
    assert!(
        invalid.validate().is_err(),
        "local admission state is not host authority"
    );
    invalid.interruption = CombatInterruption::WaitingForPlayers;
    assert!(invalid.validate().is_err(), "do not invent missing players");
    assert!(authority.snapshot(0).validate().is_ok());
}

fn peer(value: u8) -> PeerId {
    PeerId::from_bytes([value; PeerId::BYTE_LENGTH])
}

fn request(authority: &mut PartyAuthority, slot: u8, command: SessionCommand) -> RequestResult {
    let snapshot = authority.snapshot(slot);
    authority.apply(
        slot,
        GameRequest {
            sequence: snapshot.next_sequence,
            encounter: snapshot.encounter,
            decision: snapshot.combat.as_ref().map_or(0, |combat| combat.turn_id),
            assignment_revision: snapshot.assignment_revision,
            command,
        },
    )
}

fn admitted_party() -> (PartyAuthority, [PeerId; PARTY_SIZE - 1]) {
    let mut authority = PartyAuthority::with_roster(42, false, &DEFAULT_HERO_ROSTER)
        .expect("explicit six-human fixture");
    assert_eq!(
        request(
            &mut authority,
            0,
            SessionCommand::ChooseHero {
                actor: ActorId(1),
                hero: HeroClass::FieldMedic
            }
        )
        .rejection,
        None
    );
    let peers =
        std::array::from_fn(|index| peer(u8::try_from(index + 1).expect("guest count fits")));
    for (index, identity) in peers.into_iter().enumerate() {
        assert_eq!(
            authority.reserve(identity),
            Ok(u8::try_from(index + 1).expect("guest slot"))
        );
        authority.connected(identity, true);
        assert_eq!(
            request(
                &mut authority,
                0,
                SessionCommand::Assign {
                    actor: ActorId(u16::try_from(index + 2).expect("actor")),
                    owner: u8::try_from(index + 1).expect("slot"),
                }
            )
            .rejection,
            None
        );
    }
    (authority, peers)
}

fn started_party() -> (PartyAuthority, [PeerId; PARTY_SIZE - 1]) {
    let (mut authority, peers) = admitted_party();
    for slot in 0..PLAYER_CAPACITY {
        assert_eq!(
            request(&mut authority, slot, SessionCommand::Ready(true)).rejection,
            None
        );
    }
    assert_eq!(
        request(&mut authority, 0, SessionCommand::Start).rejection,
        None
    );
    (authority, peers)
}

fn advance_to_hero(authority: &mut PartyAuthority) -> (ActorId, u8) {
    for _ in 0..PARTY_SIZE * 2 {
        let combat = authority.snapshot(0).combat.expect("live combat");
        let active = combat.active_actor.expect("nonterminal encounter");
        if combat.actor(active).expect("active character").team() == Team::Heroes {
            let owner = authority
                .company
                .iter()
                .find(|player| player.actor == active)
                .expect("active hero has one owner")
                .owner;
            return (active, owner);
        }
        assert!(authority.advance_enemy());
    }
    unreachable!("at least one standing hero must have a decision after bounded initial AI turns")
}

#[test]
fn six_unique_reservations_count_pending_and_disconnected_capacity() {
    let mut authority = PartyAuthority::with_roster(42, false, &DEFAULT_HERO_ROSTER)
        .expect("explicit six-human fixture");
    assert_eq!(authority.occupied(), 1);
    for slot in 1..PLAYER_CAPACITY {
        let identity = peer(slot);
        assert_eq!(authority.reserve(identity), Ok(slot));
        assert_eq!(
            authority.reserve(identity),
            Ok(slot),
            "retry preserves its reservation"
        );
    }
    assert_eq!(authority.occupied(), PLAYER_CAPACITY);
    assert!(!authority.has_space());
    assert!(authority.reserve(peer(PLAYER_CAPACITY)).is_err());
    assert!(authority
        .snapshot(0)
        .players
        .iter()
        .filter(|player| player.slot != 0)
        .all(|player| !player.connected));
    authority.connected(peer(1), true);
    authority.connected(peer(1), false);
    assert_eq!(
        authority.occupied(),
        PLAYER_CAPACITY,
        "disconnected identity keeps its reserved seat"
    );
}

#[test]
fn all_six_players_must_be_ready_and_only_host_can_start() {
    let (mut authority, _) = admitted_party();
    for slot in 0..PLAYER_CAPACITY - 1 {
        assert_eq!(
            request(&mut authority, slot, SessionCommand::Ready(true)).rejection,
            None
        );
    }
    assert!(request(&mut authority, 0, SessionCommand::Start)
        .rejection
        .is_some());
    assert!(authority.in_lobby());
    assert_eq!(
        request(
            &mut authority,
            PLAYER_CAPACITY - 1,
            SessionCommand::Ready(true)
        )
        .rejection,
        None
    );
    assert!(request(&mut authority, 1, SessionCommand::Start)
        .rejection
        .is_some());
    assert!(authority.in_lobby());
    assert_eq!(
        request(&mut authority, 0, SessionCommand::Start).rejection,
        None
    );
    assert!(!authority.in_lobby());
}

#[test]
fn duplicate_cached_request_and_replay_after_eviction_never_reapply() {
    let mut authority = PartyAuthority::with_roster(42, false, &DEFAULT_HERO_ROSTER)
        .expect("explicit six-human fixture");
    let original = GameRequest {
        sequence: 1,
        encounter: 0,
        decision: 0,
        assignment_revision: 1,
        command: SessionCommand::Ready(false),
    };
    let first = authority.apply(0, original.clone());
    assert_eq!(first.rejection, None);
    let revision = authority.snapshot(0).revision;
    let altered_replay = GameRequest {
        command: SessionCommand::Ready(true),
        ..original.clone()
    };
    assert_eq!(authority.apply(0, altered_replay), first);
    assert_eq!(authority.snapshot(0).revision, revision);
    for _ in 0..100 {
        assert_eq!(
            request(&mut authority, 0, SessionCommand::Ready(true)).rejection,
            None
        );
    }
    assert_eq!(
        authority.results.get(&0).expect("bounded cache").len(),
        RESULT_CACHE
    );
    let before = authority.snapshot(0);
    assert!(authority.apply(0, original).rejection.is_some());
    assert_eq!(
        authority.snapshot(0),
        before,
        "evicted request cannot undo later readiness"
    );
}

#[test]
fn character_ownership_is_seat_identity_not_current_formation_rank() {
    let (mut authority, _) = admitted_party();
    // Deliberately non-arithmetic IDs and repeated classes make inferred ownership fail.
    let mut scenario = authority.scenario.clone();
    for (index, actor) in scenario.heroes.iter_mut().enumerate() {
        actor.id = ActorId(500 + index as u16 * 37);
    }
    let revision = authority.setup_revision;
    assert!(request(
        &mut authority,
        0,
        SessionCommand::ConfigureBattle {
            scenario,
            expected_revision: revision
        }
    )
    .rejection
    .is_none());
    for slot in 0..PLAYER_CAPACITY {
        assert!(request(
            &mut authority,
            0,
            SessionCommand::Assign {
                actor: ActorId(500 + u16::from(slot) * 37),
                owner: slot
            }
        )
        .rejection
        .is_none());
    }
    for slot in 0..PLAYER_CAPACITY {
        assert_eq!(
            request(
                &mut authority,
                slot,
                SessionCommand::ChooseHero {
                    actor: ActorId(500 + u16::from(slot) * 37),
                    hero: HeroClass::Knifehand
                }
            )
            .rejection,
            None
        );
    }
    for slot in 0..PLAYER_CAPACITY {
        assert_eq!(
            request(&mut authority, slot, SessionCommand::Ready(true)).rejection,
            None
        );
    }
    assert_eq!(
        request(&mut authority, 0, SessionCommand::Start).rejection,
        None
    );
    let (actor, slot) = advance_to_hero(&mut authority);
    let before = authority.snapshot(slot).combat.expect("combat");
    let ally = before
        .legal_actions(actor)
        .into_iter()
        .find_map(|action| {
            if let CombatAction::Reposition { ally } = action {
                Some(ally)
            } else {
                None
            }
        })
        .expect("adjacent standing ally");
    let wrong_owner = authority
        .company
        .iter()
        .find(|player| player.actor == ally)
        .expect("ally owner")
        .owner;
    assert!(request(
        &mut authority,
        wrong_owner,
        SessionCommand::Act {
            actor,
            action: CombatAction::Wait
        }
    )
    .rejection
    .is_some());
    assert_eq!(authority.snapshot(slot).combat, Some(before.clone()));
    assert_eq!(
        request(
            &mut authority,
            slot,
            SessionCommand::Act {
                actor,
                action: CombatAction::Reposition { ally }
            }
        )
        .rejection,
        None
    );
    let after = authority.snapshot(slot);
    assert_ne!(
        after.combat.as_ref().expect("combat").rank(actor),
        before.rank(actor)
    );
    assert_eq!(
        after
            .company
            .iter()
            .find(|member| member.owner == slot)
            .expect("owner")
            .actor,
        actor
    );
    assert_eq!(
        request(
            &mut authority,
            wrong_owner,
            SessionCommand::Act {
                actor,
                action: CombatAction::Wait
            }
        )
        .rejection
        .as_deref(),
        Some("That is not your hero.")
    );
}

#[test]
fn stale_turn_rejection_does_not_mutate_combat_or_replay_old_action() {
    let (mut authority, _) = started_party();
    let (actor, slot) = advance_to_hero(&mut authority);
    let before = authority.snapshot(slot);
    let old_turn = before.combat.as_ref().expect("combat").turn_id;
    assert_eq!(
        request(
            &mut authority,
            slot,
            SessionCommand::Act {
                actor,
                action: CombatAction::Wait
            }
        )
        .rejection,
        None
    );
    let committed = authority.snapshot(slot).combat;
    let result = authority.apply(
        slot,
        GameRequest {
            sequence: authority.next_sequence(slot),
            encounter: before.encounter,
            decision: old_turn,
            assignment_revision: before.assignment_revision,
            command: SessionCommand::Act {
                actor,
                action: CombatAction::Wait,
            },
        },
    );
    assert!(result.rejection.is_some());
    assert_eq!(authority.snapshot(slot).combat, committed);
}

#[test]
fn disconnected_guest_pauses_rules_and_reconnect_restores_same_phase_without_ticks() {
    let (mut authority, peers) = started_party();
    let (actor, slot) = advance_to_hero(&mut authority);
    let before = authority.snapshot(0).combat;
    let missing = *peers.first().expect("guest");
    let reserved_slot = authority.slot_for(missing);
    authority.connected(missing, false);
    assert!(authority.paused());
    assert!(!authority.advance_enemy());
    assert!(request(
        &mut authority,
        slot,
        SessionCommand::Act {
            actor,
            action: CombatAction::Wait
        }
    )
    .rejection
    .is_some());
    assert_eq!(authority.snapshot(0).combat, before);
    authority.release(missing);
    assert_eq!(
        authority.slot_for(missing),
        reserved_slot,
        "combat departure cannot remove a controlled hero"
    );
    assert_eq!(authority.occupied(), PLAYER_CAPACITY);
    assert!(!authority.has_space());
    authority.connected(missing, true);
    assert!(!authority.paused());
    assert_eq!(
        authority.snapshot(0).combat,
        before,
        "connection bookkeeping does not run phase entry"
    );
    assert_eq!(
        request(
            &mut authority,
            slot,
            SessionCommand::Act {
                actor,
                action: CombatAction::Wait
            }
        )
        .rejection,
        None
    );
}

#[test]
fn lobby_release_clears_only_departing_identity_history_and_allows_replacement() {
    let (mut authority, peers) = admitted_party();
    let departing = *peers.first().expect("guest");
    let slot = authority.slot_for(departing).expect("slot");
    for _ in 0..4 {
        assert_eq!(
            request(&mut authority, slot, SessionCommand::Ready(true)).rejection,
            None
        );
    }
    let host_sequence = authority.next_sequence(0);
    assert!(authority.next_sequence(slot) > 1);
    authority.release(departing);
    assert_eq!(authority.slot_for(departing), None);
    assert_eq!(authority.occupied(), PLAYER_CAPACITY - 1);
    assert_eq!(authority.next_sequence(slot), 1);
    assert_eq!(authority.next_sequence(0), host_sequence);
    let newcomer = peer(99);
    assert_eq!(authority.reserve(newcomer), Ok(slot));
    authority.connected(newcomer, true);
    assert_eq!(
        request(&mut authority, slot, SessionCommand::Ready(true)).rejection,
        None
    );
}

#[test]
fn rematch_preserves_identities_watermarks_and_invalidates_prior_encounter_commands() {
    let (mut authority, peers) = started_party();
    let previous = authority.snapshot(0);
    assert!(request(&mut authority, 1, SessionCommand::Rematch)
        .rejection
        .is_some());
    let guest_next = authority.next_sequence(1);
    assert_eq!(
        request(&mut authority, 0, SessionCommand::Rematch).rejection,
        None
    );
    let lobby = authority.snapshot(0);
    assert!(lobby.combat.is_none());
    assert_ne!(lobby.encounter, previous.encounter);
    assert!(lobby
        .players
        .iter()
        .all(|player| !player.ready && player.occupied && player.connected));
    assert_eq!(authority.next_sequence(1), guest_next);
    for identity in peers {
        assert!(authority.slot_for(identity).is_some());
    }
    let stale = authority.apply(
        1,
        GameRequest {
            sequence: guest_next,
            encounter: previous.encounter,
            decision: 0,
            assignment_revision: previous.assignment_revision,
            command: SessionCommand::Ready(true),
        },
    );
    assert!(stale.rejection.is_some());
    assert!(authority.in_lobby());
    for slot in 0..PLAYER_CAPACITY {
        assert_eq!(
            request(&mut authority, slot, SessionCommand::Ready(true)).rejection,
            None
        );
    }
    assert_eq!(
        request(&mut authority, 0, SessionCommand::Start).rejection,
        None
    );
    assert!(authority.snapshot(0).encounter > lobby.encounter);
}

#[test]
fn local_mode_uses_same_rules_but_explicitly_controls_all_heroes() {
    let mut authority = PartyAuthority::new(42, true);
    assert_eq!(
        request(&mut authority, 0, SessionCommand::Start).rejection,
        None
    );
    let (actor, _) = advance_to_hero(&mut authority);
    assert_eq!(
        request(
            &mut authority,
            0,
            SessionCommand::Act {
                actor,
                action: CombatAction::Wait
            }
        )
        .rejection,
        None
    );
    assert!(!authority.paused());
}

#[test]
fn accepted_gameplay_records_typed_monotonic_events_with_bounded_history() {
    let (mut authority, _) = started_party();
    let mut largest_id = 0;
    for _ in 0..160 {
        let snapshot = authority.snapshot(0);
        let combat = snapshot.combat.expect("combat");
        if combat.outcome.is_some() {
            break;
        }
        let actor = combat.active_actor.expect("active");
        if combat.actor(actor).expect("actor").team() == Team::Enemies {
            authority.advance_enemy();
        } else {
            let slot = authority
                .company
                .iter()
                .find(|player| player.actor == actor)
                .expect("hero owner")
                .owner;
            let action = combat
                .legal_actions(actor)
                .into_iter()
                .find(|action| {
                    combat
                        .action_ability(actor, *action)
                        .ok()
                        .flatten()
                        .and_then(|(index, _)| {
                            combat.actor(actor).and_then(|source| source.ability(index))
                        })
                        .is_some_and(|ability| {
                            ability
                                .effects
                                .iter()
                                .any(|effect| matches!(effect, labyrinth_rules::Effect::Damage(_)))
                        })
                })
                .unwrap_or(CombatAction::Wait);
            assert_eq!(
                request(&mut authority, slot, SessionCommand::Act { actor, action }).rejection,
                None
            );
        }
        let updated = authority.snapshot(0);
        assert!(updated.events.len() <= LOG_LIMIT);
        assert!(updated.log.len() <= LOG_LIMIT);
        let ids: Vec<_> = updated.events.iter().map(|event| event.id).collect();
        assert!(ids.windows(2).all(|pair| pair.first() < pair.get(1)));
        if let Some(last) = updated.events.last() {
            assert!(last.id > largest_id);
            largest_id = last.id;
        }
    }
    assert!(matches!(
        authority.snapshot(0).combat.expect("combat").outcome,
        Some(CombatOutcome::Victory | CombatOutcome::Defeat)
    ));
}

#[test]
fn repeated_class_selection_keeps_actor_ownership_and_requires_fresh_party_readiness() {
    let (mut authority, _) = admitted_party();
    for slot in 0..PLAYER_CAPACITY {
        assert_eq!(
            request(&mut authority, slot, SessionCommand::Ready(true)).rejection,
            None
        );
    }
    let before = authority.snapshot(0);
    let chooser = PLAYER_CAPACITY - 1;
    assert_eq!(
        request(
            &mut authority,
            chooser,
            SessionCommand::ChooseHero {
                actor: ActorId(u16::from(chooser) + 1),
                hero: HeroClass::Knifehand
            }
        )
        .rejection,
        None
    );
    let changed = authority.snapshot(0);
    assert!(changed.players.iter().all(|player| !player.ready));
    for player in &changed.players {
        let prior = before
            .players
            .iter()
            .find(|prior| prior.slot == player.slot)
            .expect("same participant");
        assert_eq!(player.peer, prior.peer);
    }
    for member in &changed.company {
        let prior = before
            .company
            .iter()
            .find(|prior| prior.actor == member.actor)
            .expect("same character");
        assert_eq!(member.owner, prior.owner);
        if member.owner == chooser {
            assert_eq!(member.hero, HeroClass::Knifehand);
            assert_eq!(
                member.abilities,
                changed
                    .scenario
                    .heroes
                    .iter()
                    .find(|a| a.id == member.actor)
                    .expect("configured actor")
                    .actor
                    .resolve(&changed.catalog)
                    .expect("resolved preset")
            );
        } else {
            assert_eq!(member, prior);
        }
    }
    assert!(
        changed
            .company
            .iter()
            .filter(|member| member.hero == HeroClass::Knifehand)
            .count()
            >= 3
    );
    assert!(request(&mut authority, 0, SessionCommand::Start)
        .rejection
        .is_some());
}

#[test]
fn sixth_player_reconnect_preserves_nondefault_actor_loadout_and_live_state() {
    let (mut authority, peers) = admitted_party();
    let mut scenario = authority.scenario.clone();
    let sixth = scenario.heroes.last_mut().expect("sixth character");
    sixth.id = ActorId(909);
    sixth.actor.build =
        labyrinth_rules::scenario::legacy_build(&[labyrinth_rules::SkillId::DeepStrike]);
    let revision = authority.setup_revision;
    assert!(request(
        &mut authority,
        0,
        SessionCommand::ConfigureBattle {
            scenario,
            expected_revision: revision
        }
    )
    .rejection
    .is_none());
    assert!(request(
        &mut authority,
        0,
        SessionCommand::Assign {
            actor: ActorId(909),
            owner: 5
        }
    )
    .rejection
    .is_none());
    for slot in 0..PLAYER_CAPACITY {
        assert_eq!(
            request(&mut authority, slot, SessionCommand::Ready(true)).rejection,
            None
        );
    }
    assert_eq!(
        request(&mut authority, 0, SessionCommand::Start).rejection,
        None
    );
    let before = authority.snapshot(PLAYER_CAPACITY - 1);
    let identity = *peers.last().expect("sixth human");
    authority.connected(identity, false);
    assert!(authority.paused());
    assert!(!authority.advance_enemy());
    assert_eq!(authority.reserve(identity), Ok(PLAYER_CAPACITY - 1));
    authority.connected(identity, true);
    let after = authority.snapshot(PLAYER_CAPACITY - 1);
    assert_eq!(after.players, before.players);
    assert_eq!(after.company, before.company);
    assert_eq!(after.combat, before.combat);
    assert_eq!(after.next_sequence, before.next_sequence);
    assert_eq!(after.events, before.events);
    assert_eq!(after.validate(), Ok(()));
}

fn assert_invalid_snapshot(snapshot: &SessionSnapshot) {
    assert!(snapshot.validate().is_err());
    let bytes = serde_json::to_vec(snapshot).expect("malformed fixture encodes");
    assert!(serde_json::from_slice::<SessionSnapshot>(&bytes).is_err());
}

#[test]
fn received_snapshots_validate_six_distinct_owners_even_when_classes_repeat() {
    let (authority, peers) = started_party();
    let valid = authority.snapshot(1);
    let first_peer = *peers.first().expect("guest identity");
    assert_eq!(valid.validate(), Ok(()));
    assert_eq!(valid.validate_recipient(1, first_peer), Ok(()));
    assert!(valid.validate_recipient(2, first_peer).is_err());
    let bytes = serde_json::to_vec(&valid).expect("snapshot");
    assert_eq!(
        serde_json::from_slice::<SessionSnapshot>(&bytes).expect("validated roundtrip"),
        valid
    );

    let first = valid.players.first().expect("host").clone();
    let mut bad = valid.clone();
    bad.players.last_mut().expect("guest").slot = first.slot;
    assert_invalid_snapshot(&bad);
    let mut bad = valid.clone();
    bad.company.last_mut().expect("character").actor =
        valid.company.first().expect("first character").actor;
    assert_invalid_snapshot(&bad);
    let mut bad = valid.clone();
    bad.players.last_mut().expect("guest").peer = Some(first_peer);
    assert_invalid_snapshot(&bad);
    let mut bad = valid.clone();
    bad.company.last_mut().expect("character").hero = HeroClass::Gatekeeper;
    assert_invalid_snapshot(&bad);
    let mut bad = valid.clone();
    bad.company
        .last_mut()
        .expect("character")
        .abilities
        .abilities
        .clear();
    assert_invalid_snapshot(&bad);
    let mut bad = valid.clone();
    bad.players.pop();
    assert_invalid_snapshot(&bad);
    let mut bad = valid.clone();
    bad.players.last_mut().expect("guest").connected = false;
    assert_invalid_snapshot(&bad);
}

#[test]
fn repeated_class_actors_cannot_be_swapped_between_owners_in_later_snapshots() {
    let (authority, _) = started_party();
    let before = authority.snapshot(0);
    let mut after = before.clone();
    let mut twins = after
        .company
        .iter_mut()
        .filter(|player| player.hero == HeroClass::Knifehand);
    let first = twins.next().expect("first Knifehand");
    let second = twins.next().expect("second Knifehand");
    std::mem::swap(&mut first.actor, &mut second.actor);
    for member in &after.company {
        after
            .formation
            .assign_actor(&after.scenario, member.actor, member.owner);
    }
    assert_eq!(
        after.validate(),
        Ok(()),
        "static roster remains valid but its ownership changed"
    );
    assert!(after.validate_successor(&before).is_err());
}

#[test]
fn local_rematch_class_change_redeploys_without_a_readiness_step() {
    let mut authority = PartyAuthority::new(42, true);
    assert_eq!(
        request(&mut authority, 0, SessionCommand::Start).rejection,
        None
    );
    let original = authority.snapshot(0);
    assert_eq!(
        request(&mut authority, 0, SessionCommand::Rematch).rejection,
        None
    );
    assert_eq!(
        request(
            &mut authority,
            0,
            SessionCommand::ChooseHero {
                actor: ActorId(1),
                hero: HeroClass::FieldMedic
            }
        )
        .rejection,
        None
    );
    assert!(authority
        .snapshot(0)
        .players
        .iter()
        .all(|player| !player.ready));
    assert_eq!(
        request(&mut authority, 0, SessionCommand::Start).rejection,
        None
    );
    let restarted = authority.snapshot(0);
    assert!(restarted.combat.is_some());
    assert!(restarted.encounter > original.encounter);
    assert_eq!(restarted.validate(), Ok(()));
    assert_eq!(
        restarted.company.first().expect("local character").hero,
        HeroClass::FieldMedic
    );

    let (mut remote, _) = admitted_party();
    assert_eq!(
        request(&mut remote, 0, SessionCommand::Ready(true)).rejection,
        None
    );
    assert_eq!(
        remote
            .snapshot(0)
            .players
            .iter()
            .filter(|player| player.ready)
            .count(),
        1,
        "network readiness remains one decision per human"
    );
}

#[test]
fn admitted_spectators_and_multiple_owned_heroes_follow_explicit_assignment() {
    let mut authority = PartyAuthority::new(42, false);
    authority.reserve(peer(1)).expect("guest");
    authority.connected(peer(1), true);
    let initial = authority.snapshot(1);
    assert_eq!(
        initial
            .player_views()
            .iter()
            .find(|p| p.slot == 0)
            .expect("host")
            .actors
            .len(),
        5
    );
    assert!(initial
        .player_views()
        .iter()
        .find(|p| p.slot == 1)
        .expect("guest")
        .actors
        .is_empty());
    assert!(request(
        &mut authority,
        1,
        SessionCommand::ChooseHero {
            actor: ActorId(1),
            hero: HeroClass::FieldMedic
        }
    )
    .rejection
    .is_some());
    assert!(request(
        &mut authority,
        1,
        SessionCommand::Assign {
            actor: ActorId(1),
            owner: 1
        }
    )
    .rejection
    .is_some());
    for actor in [ActorId(1), ActorId(2)] {
        assert_eq!(
            request(
                &mut authority,
                0,
                SessionCommand::Assign { actor, owner: 1 }
            )
            .rejection,
            None
        );
        assert_eq!(
            request(
                &mut authority,
                1,
                SessionCommand::ChooseHero {
                    actor,
                    hero: HeroClass::FieldMedic
                }
            )
            .rejection,
            None
        );
    }
    let assigned = authority.snapshot(1);
    assert_eq!(
        assigned
            .player_views()
            .iter()
            .find(|p| p.slot == 1)
            .expect("guest")
            .actors,
        vec![ActorId(1), ActorId(2)]
    );
    assert_eq!(
        assigned
            .player_views()
            .iter()
            .find(|p| p.slot == 0)
            .expect("host")
            .actors
            .len(),
        3
    );
    assigned
        .validate()
        .expect("multiple characters per participant");
    authority.release(peer(1));
    assert!(authority
        .snapshot(0)
        .company
        .iter()
        .all(|member| member.owner == 0));
    assert_eq!(authority.reserve(peer(2)), Ok(1));
    authority.connected(peer(2), true);
    assert!(
        authority
            .snapshot(1)
            .player_views()
            .iter()
            .find(|p| p.slot == 1)
            .expect("new guest")
            .actors
            .is_empty(),
        "a replacement identity does not inherit the former player's characters"
    );
}

#[test]
fn paused_reassignment_preserves_combat_and_rejects_old_generation_after_assignment_back() {
    let (mut authority, _) = started_party();
    let (actor, owner) = (0..PARTY_SIZE * 2)
        .find_map(|_| {
            let (actor, owner) = advance_to_hero(&mut authority);
            if owner != 0 {
                return Some((actor, owner));
            }
            assert_eq!(
                request(
                    &mut authority,
                    owner,
                    SessionCommand::Act {
                        actor,
                        action: CombatAction::Wait
                    }
                )
                .rejection,
                None
            );
            None
        })
        .expect("bounded progression to a remote owner");
    assert!(
        request(&mut authority, owner, SessionCommand::AssignmentPause(true))
            .rejection
            .is_some()
    );
    let before = authority.snapshot(owner);
    let stale = GameRequest {
        sequence: before.next_sequence,
        encounter: before.encounter,
        decision: before.combat.as_ref().expect("combat").turn_id,
        assignment_revision: before.assignment_revision,
        command: SessionCommand::Act {
            actor,
            action: CombatAction::Wait,
        },
    };
    assert!(
        request(
            &mut authority,
            0,
            SessionCommand::Assign { actor, owner: 0 }
        )
        .rejection
        .is_some(),
        "combat changes require the paused flow"
    );
    assert_eq!(
        request(&mut authority, 0, SessionCommand::AssignmentPause(true)).rejection,
        None
    );
    assert!(!authority.advance_enemy());
    assert_eq!(
        request(
            &mut authority,
            0,
            SessionCommand::Assign { actor, owner: 0 }
        )
        .rejection,
        None
    );
    assert_eq!(authority.snapshot(0).combat, before.combat);
    assert_eq!(
        request(&mut authority, 0, SessionCommand::Assign { actor, owner }).rejection,
        None
    );
    let assigned = authority.snapshot(owner);
    assert!(assigned.assignment_revision > before.assignment_revision);
    assert_eq!(assigned.company, before.company);
    assert_eq!(assigned.combat, before.combat);
    assert_eq!(assigned.events, before.events);
    assert_eq!(assigned.interruption, CombatInterruption::Assignments);
    assigned.validate().expect("paused assignment snapshot");
    assert_eq!(
        request(&mut authority, 0, SessionCommand::AssignmentPause(false)).rejection,
        None
    );
    assert_eq!(
        authority.apply(owner, stale).rejection.as_deref(),
        Some("Character control changed. Refresh before acting.")
    );
    assert_eq!(
        authority.snapshot(owner).combat,
        before.combat,
        "rejected old generation never grants another action"
    );
    assert_eq!(
        request(
            &mut authority,
            owner,
            SessionCommand::Act {
                actor,
                action: CombatAction::Wait
            }
        )
        .rejection,
        None
    );
}

#[test]
fn deliberate_reassignment_resumes_a_missing_controller_but_never_clears_a_rules_fault() {
    let (mut authority, _) = started_party();
    authority.connected(peer(1), false);
    let before = authority.snapshot(0).combat;
    assert!(authority.paused());
    assert_eq!(
        request(&mut authority, 0, SessionCommand::AssignmentPause(true)).rejection,
        None
    );
    assert_eq!(
        request(
            &mut authority,
            0,
            SessionCommand::Assign {
                actor: ActorId(2),
                owner: 0
            }
        )
        .rejection,
        None
    );
    assert_eq!(
        request(&mut authority, 0, SessionCommand::AssignmentPause(false)).rejection,
        None
    );
    assert!(
        !authority.paused(),
        "a disconnected spectator no longer blocks the encounter"
    );
    assert_eq!(authority.snapshot(0).combat, before);
    authority.connected(peer(1), true);
    assert!(authority
        .snapshot(1)
        .player_views()
        .iter()
        .find(|p| p.slot == 1)
        .expect("reconnected player")
        .actors
        .is_empty());
    authority.faulted = true;
    assert!(
        request(&mut authority, 0, SessionCommand::AssignmentPause(false))
            .rejection
            .is_some()
    );
    assert!(request(
        &mut authority,
        0,
        SessionCommand::Assign {
            actor: ActorId(2),
            owner: 1
        }
    )
    .rejection
    .is_some());
    assert_eq!(
        authority.snapshot(0).interruption,
        CombatInterruption::Halted
    );
    assert_eq!(authority.snapshot(0).combat, before);
}

#[test]
fn dying_heroes_keep_a_required_controller_while_permanent_death_makes_a_spectator() {
    let (mut authority, _) = started_party();
    let mut saw_dying = false;
    let mut saw_dead = false;
    for _ in 0..400 {
        let snapshot = authority.snapshot(0);
        let combat = snapshot.combat.as_ref().expect("combat");
        for member in snapshot.company.iter().filter(|member| member.owner != 0) {
            let actor = combat.actor(member.actor).expect("hero");
            if actor.standing() {
                continue;
            }
            let view = snapshot
                .player_views()
                .into_iter()
                .find(|p| p.slot == member.owner)
                .expect("controller");
            if actor.dying() {
                assert_eq!(view.actors, vec![actor.id]);
                authority.connected(peer(member.owner), false);
                assert!(
                    authority.paused(),
                    "a rescueable hero still requires its controller"
                );
                authority
                    .snapshot(0)
                    .validate()
                    .expect("disconnected dying controller");
                authority.connected(peer(member.owner), true);
                saw_dying = true;
            } else {
                assert!(
                    view.actors.is_empty(),
                    "permanent death produces spectator behavior"
                );
                authority.connected(peer(member.owner), false);
                assert!(
                    !authority.paused(),
                    "a dead hero's disconnected spectator cannot freeze combat"
                );
                authority
                    .snapshot(0)
                    .validate()
                    .expect("disconnected spectator");
                authority.connected(peer(member.owner), true);
                saw_dead = true;
            }
            assert_eq!(authority.snapshot(0).combat, snapshot.combat);
        }
        if saw_dying && saw_dead {
            break;
        }
        if combat.outcome.is_some() {
            break;
        }
        let actor = combat.active_actor.expect("live decision");
        if combat.actor(actor).expect("actor").team() == Team::Enemies {
            assert!(authority.advance_enemy());
        } else {
            let owner = snapshot
                .company
                .iter()
                .find(|m| m.actor == actor)
                .expect("owner")
                .owner;
            assert_eq!(
                request(
                    &mut authority,
                    owner,
                    SessionCommand::Act {
                        actor,
                        action: CombatAction::Wait
                    }
                )
                .rejection,
                None
            );
        }
    }
    assert!(saw_dying, "fixture must cross a rescueable Dying boundary");
    assert!(saw_dead, "fixture must cross a permanent death boundary");
}

#[test]
fn configured_both_teams_freeze_stats_builds_and_repeat_the_exact_seed() {
    let mut authority = PartyAuthority::new(42, true);
    let mut scenario =
        Scenario::stock(StockScenario::WeaponComparison, 9001, &authority.catalog).expect("stock");
    let hero = scenario.heroes.first_mut().expect("hero");
    hero.actor.name = "Custom cleaver".into();
    hero.actor.max_hp = 117;
    hero.actor.base_speed = 73;
    let enemy = scenario.enemies.last_mut().expect("enemy");
    enemy.actor.name = "Rear target".into();
    enemy.actor.max_hp = 211;
    enemy.actor.base_speed = 2;
    let revision = authority.setup_revision;
    assert!(request(
        &mut authority,
        0,
        SessionCommand::ConfigureBattle {
            scenario: scenario.clone(),
            expected_revision: revision
        }
    )
    .rejection
    .is_none());
    assert!(authority.players.iter().all(|p| !p.ready));
    assert!(request(&mut authority, 0, SessionCommand::Ready(true))
        .rejection
        .is_none());
    assert!(request(&mut authority, 0, SessionCommand::Start)
        .rejection
        .is_none());
    let before = authority.snapshot(0);
    before.validate().expect("configured snapshot");
    let combat = before.combat.as_ref().expect("combat");
    assert_eq!(
        combat.actor(ActorId(1)).expect("hero").name(),
        "Custom cleaver"
    );
    assert_eq!(combat.actor(ActorId(1)).expect("hero").max_hp, 117);
    assert_eq!(combat.actor(ActorId(106)).expect("enemy").max_hp, 211);
    assert!(combat
        .actor(ActorId(1))
        .expect("hero")
        .resolved_abilities()
        .iter()
        .any(|a| a.definition.id.as_str() == "greatsword_cleave"));
    assert!(request(&mut authority, 0, SessionCommand::Rematch)
        .rejection
        .is_none());
    assert!(request(&mut authority, 0, SessionCommand::Start)
        .rejection
        .is_none());
    let restarted = authority.snapshot(0);
    assert_eq!(
        restarted.combat, before.combat,
        "rematch must not silently increment the seed"
    );
    assert_eq!(restarted.scenario, scenario);
}

#[test]
fn owned_build_edits_cannot_change_enemy_roster_formation_or_a_newer_draft() {
    let (mut authority, _) = admitted_party();
    let original = authority.snapshot(1);
    let mut hero = original.scenario.heroes.get(1).expect("owned hero").clone();
    hero.actor.max_hp = 77;
    hero.actor.build.weapon = Some(labyrinth_rules::catalog::ContentId::new("dagger").expect("ID"));
    hero.actor.build.learned_skills =
        vec![labyrinth_rules::catalog::ContentId::new("duelist_dagger_power").expect("ID")];
    assert!(request(
        &mut authority,
        1,
        SessionCommand::CustomizeActor {
            actor: hero.clone(),
            expected_revision: original.setup_revision
        }
    )
    .rejection
    .is_none());
    assert_eq!(authority.scenario.heroes.get(1), Some(&hero));
    let changed = authority.snapshot(1);
    // The retained preset adapter obeys the same authority as authored edits.
    assert_eq!(
        request(
            &mut authority,
            1,
            SessionCommand::ChooseHero {
                actor: hero.id,
                hero: HeroClass::LanternWagon,
            }
        )
        .rejection
        .as_deref(),
        Some("The host assigns formation spaces. Ask the host to change this footprint.")
    );
    let sequence = authority.next_sequence(1);
    assert_eq!(
        authority
            .apply(
                1,
                GameRequest {
                    sequence,
                    encounter: changed.encounter,
                    decision: 0,
                    assignment_revision: changed.assignment_revision - 1,
                    command: SessionCommand::ChooseHero {
                        actor: hero.id,
                        hero: HeroClass::Gatekeeper,
                    },
                }
            )
            .rejection
            .as_deref(),
        Some("Character assignments changed. Refresh your draft.")
    );
    let mut stale = hero.clone();
    stale.actor.max_hp = 99;
    assert!(request(
        &mut authority,
        1,
        SessionCommand::CustomizeActor {
            actor: stale,
            expected_revision: original.setup_revision
        }
    )
    .rejection
    .is_some());
    let mut wider = hero.clone();
    wider.actor.footprint = 2;
    assert!(request(
        &mut authority,
        1,
        SessionCommand::CustomizeActor {
            actor: wider,
            expected_revision: changed.setup_revision
        }
    )
    .rejection
    .is_some());
    let mut enemy = changed.scenario.enemies.first().expect("enemy").clone();
    enemy.actor.max_hp = 1;
    assert!(request(
        &mut authority,
        1,
        SessionCommand::CustomizeActor {
            actor: enemy,
            expected_revision: changed.setup_revision
        }
    )
    .rejection
    .is_some());
    assert!(request(
        &mut authority,
        1,
        SessionCommand::ConfigureBattle {
            scenario: changed.scenario.clone(),
            expected_revision: changed.setup_revision
        }
    )
    .rejection
    .is_some());
    assert_eq!(authority.scenario, changed.scenario);
    assert_eq!(authority.company, changed.company);
    assert_eq!(authority.setup_revision, changed.setup_revision);
}

#[test]
fn invalid_scenario_is_rejected_before_ready_without_partial_setup_mutation() {
    let mut authority = PartyAuthority::new(42, true);
    let before = authority.snapshot(0);
    let mut invalid = before.scenario.clone();
    invalid.enemies.first_mut().expect("enemy").actor.max_hp = 0;
    assert!(request(
        &mut authority,
        0,
        SessionCommand::ConfigureBattle {
            scenario: invalid,
            expected_revision: before.setup_revision
        }
    )
    .rejection
    .is_some());
    assert_eq!(authority.scenario, before.scenario);
    assert_eq!(authority.players, before.players);
    assert_eq!(authority.company, before.company);
    assert_eq!(authority.setup_revision, before.setup_revision);
    assert!(authority.combat.is_none());
}

#[test]
fn spectators_do_not_gate_ready_or_start_even_if_the_host_spectates() {
    let mut authority = PartyAuthority::new(42, false);
    let owner = peer(1);
    authority.reserve(owner).expect("owner");
    authority.connected(owner, true);
    let spectator = peer(2);
    authority.reserve(spectator).expect("spectator");
    authority.connected(spectator, true);
    let ids = authority
        .company
        .iter()
        .map(|m| m.actor)
        .collect::<Vec<_>>();
    for actor in ids {
        assert!(request(
            &mut authority,
            0,
            SessionCommand::Assign { actor, owner: 1 }
        )
        .rejection
        .is_none());
    }
    assert!(request(&mut authority, 1, SessionCommand::Ready(true))
        .rejection
        .is_none());
    assert!(authority
        .players
        .iter()
        .filter(|p| p.slot != 1)
        .all(|p| !p.ready));
    authority.connected(spectator, false);
    assert!(request(&mut authority, 0, SessionCommand::Start)
        .rejection
        .is_none());
    let snapshot = authority.snapshot(0);
    snapshot
        .validate()
        .expect("host and guest spectators do not pause");
    assert!(!snapshot.paused);
    assert!(snapshot
        .player_views()
        .first()
        .expect("host")
        .actors
        .is_empty());
}

mod spatial;
