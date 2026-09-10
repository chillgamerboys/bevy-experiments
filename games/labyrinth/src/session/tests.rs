//! Game-owned policy tests. These do not claim transport or process-restart evidence.

use super::*;
use labyrinth_rules::{CombatOutcome, Team};

#[test]
fn wagon_capacity_preserves_players_and_reconnect_ownership() {
    let mut a = PartyAuthority::with_roster(42, false, &DEFAULT_HERO_ROSTER)
        .expect("explicit six-human fixture");
    let original = a.snapshot(0);
    assert!(request(
        &mut a,
        0,
        SessionCommand::ChooseHero(HeroClass::LanternWagon)
    )
    .rejection
    .is_none());
    let selected = a.snapshot(0);
    selected.validate().expect("valid five-member company");
    selected
        .validate_successor(&original)
        .expect("lobby capacity change");
    assert_eq!(a.capacity(), 5);
    assert!(
        request(&mut a, 0, SessionCommand::ChooseHero(HeroClass::Gatekeeper))
            .rejection
            .is_none()
    );
    let restored = a.snapshot(0);
    restored.validate().expect("six spaces restored");
    restored
        .validate_successor(&selected)
        .expect("empty seat restored");
    assert_eq!(a.capacity(), 6);
    assert!(request(
        &mut a,
        0,
        SessionCommand::ChooseHero(HeroClass::LanternWagon)
    )
    .rejection
    .is_none());
    for p in 1..5 {
        let id = peer(p);
        a.reserve(id).expect("guest fits");
        a.connected(id, true);
    }
    let before = a.snapshot(1);
    assert!(request(
        &mut a,
        1,
        SessionCommand::ChooseHero(HeroClass::LanternWagon)
    )
    .rejection
    .is_some());
    assert_eq!(
        a.snapshot(1).players,
        before.players,
        "never evict an admitted player"
    );
    for slot in 0..5 {
        assert!(request(&mut a, slot, SessionCommand::Ready(true))
            .rejection
            .is_none());
    }
    assert!(request(&mut a, 0, SessionCommand::Start)
        .rejection
        .is_none());
    let before = a.snapshot(0);
    before.validate().expect("combat ownership valid");
    assert_eq!(
        before.combat.as_ref().expect("combat").ranks(ActorId(1)),
        Some(5..=6)
    );
    a.connected(peer(4), false);
    a.connected(peer(4), true);
    assert_eq!(
        a.snapshot(0).combat,
        before.combat,
        "reconnection does not move actors or tick effects"
    );
}

#[test]
fn local_prototype_has_four_ordinary_heroes_and_one_weak_wagon() {
    let mut a = PartyAuthority::new(42, true);
    let hosted = PartyAuthority::new(42, false);
    assert_eq!(
        hosted.players.iter().map(|p| p.hero).collect::<Vec<_>>(),
        labyrinth_rules::PROTOTYPE_HERO_ROSTER
    );
    assert_eq!(
        a.players.iter().map(|p| p.hero).collect::<Vec<_>>(),
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
            SessionCommand::ChooseHero(HeroClass::FieldMedic)
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
                .players
                .iter()
                .find(|player| player.actor == active)
                .expect("active hero has one owner")
                .slot;
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
    for player in &mut authority.players {
        player.actor = ActorId(500 + u16::from(player.slot) * 37);
    }
    for slot in 0..PLAYER_CAPACITY {
        assert_eq!(
            request(
                &mut authority,
                slot,
                SessionCommand::ChooseHero(HeroClass::Knifehand)
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
        .players
        .iter()
        .find(|player| player.actor == ally)
        .expect("ally owner")
        .slot;
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
            .players
            .iter()
            .find(|player| player.slot == slot)
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
                .players
                .iter()
                .find(|player| player.actor == actor)
                .expect("hero owner")
                .slot;
            let action = combat.legal_actions(actor).into_iter().find(|action| matches!(action, CombatAction::Skill { skill, .. } if labyrinth_rules::skill_definition(*skill).effects.iter().any(|effect| matches!(effect, labyrinth_rules::Effect::Damage(_))))).unwrap_or(CombatAction::Wait);
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
            SessionCommand::ChooseHero(HeroClass::Knifehand)
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
            .expect("same player");
        assert_eq!((player.actor, player.peer), (prior.actor, prior.peer));
        if player.slot != chooser {
            assert_eq!(
                (player.hero, &player.abilities),
                (prior.hero, &prior.abilities)
            );
        } else {
            assert_eq!(player.hero, HeroClass::Knifehand);
            assert_eq!(
                player.abilities,
                HeroSetup::preset(player.actor, player.hero).abilities
            );
        }
    }
    assert!(
        changed
            .players
            .iter()
            .filter(|player| player.hero == HeroClass::Knifehand)
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
    let sixth = authority.players.last_mut().expect("sixth player");
    sixth.actor = ActorId(909);
    sixth.abilities =
        AbilityLoadout::new(vec![labyrinth_rules::SkillId::DeepStrike]).expect("instance loadout");
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
    bad.players.last_mut().expect("guest").actor = first.actor;
    assert_invalid_snapshot(&bad);
    let mut bad = valid.clone();
    bad.players.last_mut().expect("guest").peer = Some(first_peer);
    assert_invalid_snapshot(&bad);
    let mut bad = valid.clone();
    bad.players.last_mut().expect("guest").hero = HeroClass::Gatekeeper;
    assert_invalid_snapshot(&bad);
    let mut bad = valid.clone();
    bad.players.last_mut().expect("guest").abilities =
        AbilityLoadout::new(Vec::new()).expect("empty is bounded");
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
        .players
        .iter_mut()
        .filter(|player| player.hero == HeroClass::Knifehand);
    let first = twins.next().expect("first Knifehand");
    let second = twins.next().expect("second Knifehand");
    std::mem::swap(&mut first.actor, &mut second.actor);
    assert_eq!(
        after.validate(),
        Ok(()),
        "static roster remains valid but its ownership changed"
    );
    assert!(after.validate_successor(&before).is_err());
}

#[test]
fn local_rematch_class_change_can_ready_the_entire_locally_controlled_party() {
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
            SessionCommand::ChooseHero(HeroClass::FieldMedic)
        )
        .rejection,
        None
    );
    assert!(authority
        .snapshot(0)
        .players
        .iter()
        .all(|player| !player.ready));
    assert!(request(&mut authority, 0, SessionCommand::Start)
        .rejection
        .is_some());
    assert_eq!(
        request(&mut authority, 0, SessionCommand::Ready(true)).rejection,
        None
    );
    assert!(authority
        .snapshot(0)
        .players
        .iter()
        .all(|player| player.ready));
    assert_eq!(
        request(&mut authority, 0, SessionCommand::Ready(false)).rejection,
        None
    );
    assert!(authority
        .snapshot(0)
        .players
        .iter()
        .all(|player| !player.ready));
    assert_eq!(
        request(&mut authority, 0, SessionCommand::Ready(true)).rejection,
        None
    );
    assert_eq!(
        request(&mut authority, 0, SessionCommand::Start).rejection,
        None
    );
    let restarted = authority.snapshot(0);
    assert!(restarted.combat.is_some());
    assert!(restarted.encounter > original.encounter);
    assert_eq!(restarted.validate(), Ok(()));
    assert_eq!(
        restarted.players.first().expect("local controller").hero,
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
