//! Game-owned policy tests. These do not claim transport or process-restart evidence.

use super::*;
use labyrinth_rules::{CombatOutcome, Team};

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

fn admitted_party() -> (PartyAuthority, [PeerId; 3]) {
    let mut authority = PartyAuthority::new(42, false);
    assert_eq!(
        request(
            &mut authority,
            0,
            SessionCommand::ChooseHero(HeroClass::FieldMedic)
        )
        .rejection,
        None
    );
    let peers = [peer(11), peer(22), peer(33)];
    for (index, identity) in peers.into_iter().enumerate() {
        assert_eq!(
            authority.reserve(identity),
            Ok(u8::try_from(index + 1).expect("guest slot"))
        );
        authority.connected(identity, true);
    }
    (authority, peers)
}

fn started_party() -> (PartyAuthority, [PeerId; 3]) {
    let (mut authority, peers) = admitted_party();
    for slot in 0..4 {
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
    for _ in 0..8 {
        let combat = authority.snapshot(0).combat.expect("live combat");
        let active = combat.active_actor.expect("nonterminal encounter");
        if combat.actor(active).expect("active character").team() == Team::Heroes {
            return (active, u8::try_from(active.0 - 1).expect("hero owner slot"));
        }
        assert!(authority.advance_enemy());
    }
    unreachable!("at least one standing hero must have a decision after bounded initial AI turns")
}

#[test]
fn four_unique_reservations_count_pending_and_disconnected_capacity() {
    let mut authority = PartyAuthority::new(42, false);
    assert_eq!(authority.occupied(), 1);
    for (slot, identity) in [(1, peer(1)), (2, peer(2)), (3, peer(3))] {
        assert_eq!(authority.reserve(identity), Ok(slot));
        assert_eq!(
            authority.reserve(identity),
            Ok(slot),
            "retry preserves its reservation"
        );
    }
    assert_eq!(authority.occupied(), 4);
    assert!(!authority.has_space());
    assert!(authority.reserve(peer(4)).is_err());
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
        4,
        "disconnected identity keeps its reserved seat"
    );
}

#[test]
fn all_four_players_must_be_ready_and_only_host_can_start() {
    let (mut authority, _) = admitted_party();
    for slot in 0..3 {
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
        request(&mut authority, 3, SessionCommand::Ready(true)).rejection,
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
    let mut authority = PartyAuthority::new(42, false);
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
    let (mut authority, _) = started_party();
    let initial = authority.snapshot(0).combat.expect("combat");
    assert_eq!(
        initial.actor(ActorId(1)).expect("host hero").kind,
        labyrinth_rules::ActorKind::Hero(HeroClass::FieldMedic)
    );
    assert_eq!(initial.rank(ActorId(1)), Some(4));
    let (actor, slot) = advance_to_hero(&mut authority);
    let before = authority.snapshot(slot).combat.expect("combat");
    let wrong_rank_owner = before.rank(actor).expect("hero rank") - 1;
    assert_ne!(
        wrong_rank_owner, slot,
        "fixture rotates every hero away from seat-index rank"
    );
    assert!(request(
        &mut authority,
        wrong_rank_owner,
        SessionCommand::Act {
            actor,
            action: CombatAction::Wait
        }
    )
    .rejection
    .is_some());
    assert_eq!(authority.snapshot(slot).combat, Some(before));
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
    assert_eq!(authority.occupied(), 4);
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
    assert_eq!(authority.occupied(), 3);
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
    for slot in 0..4 {
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
            let slot = u8::try_from(actor.0 - 1).expect("hero slot");
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
