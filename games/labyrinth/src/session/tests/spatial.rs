//! Construction policy exercises real requests and validated wire roundtrips.

use super::*;

fn remove(authority: &mut PartyAuthority, actor: ActorId) -> RequestResult {
    let expected_revision = authority.setup_revision;
    request(
        authority,
        0,
        SessionCommand::RemoveScenarioActor {
            actor,
            expected_revision,
        },
    )
}

fn place(
    authority: &mut PartyAuthority,
    slot: u8,
    team: Team,
    rank: u8,
    kind: ActorKind,
) -> RequestResult {
    let preset = authority
        .catalog
        .definition()
        .actor_presets
        .iter()
        .find(|p| p.appearance == kind)
        .expect("authored preset")
        .id
        .clone();
    let expected_revision = authority.setup_revision;
    request(
        authority,
        slot,
        SessionCommand::PlaceScenarioActor {
            team,
            rank,
            preset,
            expected_revision,
        },
    )
}

fn assign_rank(authority: &mut PartyAuthority, rank: u8, owner: u8) -> RequestResult {
    let expected_revision = authority.setup_revision;
    request(
        authority,
        0,
        SessionCommand::AssignFormationRank {
            rank,
            owner,
            expected_revision,
        },
    )
}

fn move_to(authority: &mut PartyAuthority, actor: ActorId, rank: u8) -> RequestResult {
    let expected_revision = authority.setup_revision;
    request(
        authority,
        0,
        SessionCommand::MoveScenarioActor {
            actor,
            rank,
            expected_revision,
        },
    )
}

fn wire_roundtrip(authority: &PartyAuthority) -> SessionSnapshot {
    let snapshot = authority.snapshot(0);
    snapshot.validate().expect("valid shared preparation");
    let bytes = serde_json::to_vec(&snapshot).expect("encode");
    let received: SessionSnapshot = serde_json::from_slice(&bytes).expect("validated decode");
    assert_eq!(received, snapshot);
    received
}

#[test]
fn spatial_removal_preserves_real_gaps_and_small_deployment_remains_compact() {
    let mut authority = PartyAuthority::new(42, true);
    assert!(remove(&mut authority, ActorId(2)).rejection.is_none());
    let sparse = wire_roundtrip(&authority);
    assert_eq!(sparse.formation.rank(ActorId(3)), Some(3));
    assert_eq!(
        sparse.formation.occupant(&sparse.scenario, Team::Heroes, 2),
        None
    );
    assert!(sparse
        .formation
        .deployment_error(&sparse.scenario)
        .expect("gap reason")
        .contains("rank 2"));
    assert!(request(&mut authority, 0, SessionCommand::Ready(true))
        .rejection
        .is_none());
    let before = authority.snapshot(0);
    assert!(request(&mut authority, 0, SessionCommand::Start)
        .rejection
        .expect("blocked")
        .contains("rank 2"));
    assert_eq!(authority.scenario, before.scenario);
    assert_eq!(authority.formation, before.formation);
    assert!(authority.combat.is_none());
    assert!(move_to(&mut authority, ActorId(3), 2).rejection.is_none());
    assert!(move_to(&mut authority, ActorId(4), 3).rejection.is_none());
    assert!(move_to(&mut authority, ActorId(5), 4).rejection.is_none());
    assert!(authority
        .formation
        .deployment_error(&authority.scenario)
        .is_none());
    assert!(request(&mut authority, 0, SessionCommand::Ready(true))
        .rejection
        .is_none());
    assert!(request(&mut authority, 0, SessionCommand::Start)
        .rejection
        .is_none());
    let started = wire_roundtrip(&authority);
    let combat = started.combat.expect("compact combat");
    assert_eq!(combat.ranks(ActorId(5)), Some(4..=5));
    assert_eq!(combat.occupant(Team::Heroes, 6), None);
    assert!(request(&mut authority, 0, SessionCommand::Rematch)
        .rejection
        .is_none());
    assert_eq!(authority.formation, started.formation);
    assert_eq!(authority.scenario, started.scenario);
}

#[test]
fn spatial_empty_teams_and_all_down_heroes_are_shared_drafts_but_cannot_start() {
    let mut authority = PartyAuthority::new(42, true);
    let ids = authority
        .scenario
        .heroes
        .iter()
        .chain(&authority.scenario.enemies)
        .map(|a| a.id)
        .collect::<Vec<_>>();
    for id in ids {
        assert!(remove(&mut authority, id).rejection.is_none());
    }
    let empty = wire_roundtrip(&authority);
    assert!(empty.company.is_empty());
    assert!(empty.players.iter().all(|p| empty
        .player_views()
        .iter()
        .find(|v| v.slot == p.slot)
        .expect("slot")
        .actors
        .is_empty()));
    assert!(request(&mut authority, 0, SessionCommand::Start)
        .rejection
        .expect("blocked")
        .contains("at least one"));
    assert!(place(
        &mut authority,
        0,
        Team::Heroes,
        1,
        ActorKind::Hero(HeroClass::Gatekeeper)
    )
    .rejection
    .is_none());
    assert!(place(
        &mut authority,
        0,
        Team::Enemies,
        1,
        ActorKind::Enemy(labyrinth_rules::EnemyKind::AshBrute)
    )
    .rejection
    .is_none());
    let mut actor = authority.scenario.heroes.first().expect("hero").clone();
    actor.starting_hp = Some(0);
    let expected_revision = authority.setup_revision;
    assert!(request(
        &mut authority,
        0,
        SessionCommand::CustomizeActor {
            actor,
            expected_revision
        }
    )
    .rejection
    .is_none());
    wire_roundtrip(&authority);
    assert!(request(&mut authority, 0, SessionCommand::Start)
        .rejection
        .expect("blocked")
        .contains("standing"));
}

#[test]
fn spatial_selected_type_replaces_identity_and_uses_collision_preview_for_both_teams() {
    for team in [Team::Heroes, Team::Enemies] {
        let mut authority = PartyAuthority::new(42, true);
        let (small, large) = match team {
            Team::Heroes => (
                ActorKind::Hero(HeroClass::Gatekeeper),
                ActorKind::Hero(HeroClass::LanternWagon),
            ),
            Team::Enemies => (
                ActorKind::Enemy(labyrinth_rules::EnemyKind::AshBrute),
                ActorKind::Enemy(labyrinth_rules::EnemyKind::OssuaryHauler),
            ),
        };
        let id = authority
            .formation
            .occupant(&authority.scenario, team, 1)
            .expect("front actor");
        let next = authority
            .formation
            .occupant(&authority.scenario, team, 2)
            .expect("next actor");
        let before = authority.snapshot(0);
        let preview =
            authority
                .formation
                .placement_error(&authority.scenario, team, 1, 2, Some(id), None);
        assert!(preview.as_ref().is_some_and(|e| e.contains("occupied")));
        assert_eq!(place(&mut authority, 0, team, 1, large).rejection, preview);
        assert_eq!(authority.scenario, before.scenario);
        assert_eq!(authority.formation, before.formation);
        assert!(remove(&mut authority, next).rejection.is_none());
        assert!(place(&mut authority, 0, team, 1, large).rejection.is_none());
        assert_eq!(
            authority.formation.occupant(&authority.scenario, team, 1),
            Some(id)
        );
        assert_eq!(
            authority.formation.occupant(&authority.scenario, team, 2),
            Some(id)
        );
        assert!(
            place(&mut authority, 0, team, 2, small).rejection.is_none(),
            "covered-rank replacement uses the original leading rank"
        );
        assert_eq!(authority.formation.rank(id), Some(1));
        assert_eq!(
            authority.formation.occupant(&authority.scenario, team, 2),
            None
        );
        wire_roundtrip(&authority);
        assert_eq!(
            place(&mut authority, 0, team, 7, large).rejection,
            authority.formation.placement_error(
                &authority.scenario,
                team,
                7,
                2,
                authority.formation.occupant(&authority.scenario, team, 7),
                None
            )
        );
    }
}

#[test]
fn spatial_empty_place_reservations_authorize_guest_types_and_follow_whole_footprints() {
    let (mut authority, _) = admitted_party();
    assert!(remove(&mut authority, ActorId(1)).rejection.is_none());
    assert!(remove(&mut authority, ActorId(2)).rejection.is_none());
    assert!(assign_rank(&mut authority, 1, 1).rejection.is_none());
    assert!(assign_rank(&mut authority, 2, 2).rejection.is_none());
    let before = wire_roundtrip(&authority);
    assert!(before
        .company
        .iter()
        .all(|m| m.actor != ActorId(1) && m.actor != ActorId(2)));
    assert_eq!(before.formation.owner(1), Some(1));
    let large = ActorKind::Hero(HeroClass::LanternWagon);
    assert!(place(&mut authority, 1, Team::Heroes, 1, large)
        .rejection
        .expect("owner boundary")
        .contains("different players"));
    assert!(assign_rank(&mut authority, 2, 1).rejection.is_none());
    assert!(place(&mut authority, 2, Team::Heroes, 1, large)
        .rejection
        .expect("not owner")
        .contains("assigned places"));
    assert!(place(&mut authority, 1, Team::Heroes, 1, large)
        .rejection
        .is_none());
    let actor = authority
        .formation
        .occupant(&authority.scenario, Team::Heroes, 1)
        .expect("guest character");
    assert!(authority
        .company
        .iter()
        .any(|m| m.actor == actor && m.owner == 1));
    assert!(assign_rank(&mut authority, 2, 2).rejection.is_none());
    assert_eq!(
        (authority.formation.owner(1), authority.formation.owner(2)),
        (Some(2), Some(2))
    );
    assert!(authority
        .company
        .iter()
        .any(|m| m.actor == actor && m.owner == 2));
    assert!(place(&mut authority, 1, Team::Heroes, 1, large)
        .rejection
        .is_some());
    assert!(place(
        &mut authority,
        2,
        Team::Enemies,
        1,
        ActorKind::Enemy(labyrinth_rules::EnemyKind::AshBrute)
    )
    .rejection
    .is_some());
    let expected_revision = authority.setup_revision;
    for command in [
        SessionCommand::MoveScenarioActor {
            actor,
            rank: 2,
            expected_revision,
        },
        SessionCommand::RemoveScenarioActor {
            actor,
            expected_revision,
        },
        SessionCommand::AssignFormationRank {
            rank: 1,
            owner: 2,
            expected_revision,
        },
    ] {
        assert!(request(&mut authority, 2, command).rejection.is_some());
    }
    assert!(remove(&mut authority, actor).rejection.is_none());
    assert_eq!(
        (authority.formation.owner(1), authority.formation.owner(2)),
        (Some(2), Some(2))
    );
    authority.release(peer(2));
    assert_eq!(
        (authority.formation.owner(1), authority.formation.owner(2)),
        (Some(0), Some(0))
    );
    assert!(authority.company.iter().all(|m| m.owner != 2));
    wire_roundtrip(&authority);
}

#[test]
fn spatial_move_uses_destination_reservation_and_never_silently_swaps_or_compacts() {
    let (mut authority, _) = admitted_party();
    assert!(remove(&mut authority, ActorId(2)).rejection.is_none());
    let before = authority.snapshot(0);
    assert!(move_to(&mut authority, ActorId(3), 1).rejection.is_some());
    assert_eq!(authority.formation, before.formation);
    assert!(move_to(&mut authority, ActorId(3), 2).rejection.is_none());
    assert_eq!(
        authority.formation.owner(3),
        Some(2),
        "source reservation stays in place"
    );
    assert!(authority
        .company
        .iter()
        .any(|m| m.actor == ActorId(3) && m.owner == 1));
    assert_eq!(authority.formation.rank(ActorId(4)), Some(4));
    assert!(authority.players.iter().all(|p| !p.ready));
    wire_roundtrip(&authority);
}

#[test]
fn spatial_stale_type_selection_and_assignment_away_back_cannot_mutate_draft() {
    let (mut authority, _) = admitted_party();
    let original = authority.snapshot(1);
    assert!(assign_rank(&mut authority, 2, 2).rejection.is_none());
    assert!(assign_rank(&mut authority, 2, 1).rejection.is_none());
    let preset = authority
        .catalog
        .definition()
        .actor_presets
        .first()
        .expect("preset")
        .id
        .clone();
    let before = authority.snapshot(1);
    let command = SessionCommand::PlaceScenarioActor {
        team: Team::Heroes,
        rank: 2,
        preset,
        expected_revision: original.setup_revision,
    };
    assert!(request(&mut authority, 1, command.clone())
        .rejection
        .is_some());
    let mut stale_assignment = GameRequest {
        sequence: authority.next_sequence(1),
        encounter: authority.encounter,
        decision: 0,
        assignment_revision: original.assignment_revision,
        command,
    };
    if let SessionCommand::PlaceScenarioActor {
        expected_revision, ..
    } = &mut stale_assignment.command
    {
        *expected_revision = authority.setup_revision;
    }
    assert!(authority
        .apply(1, stale_assignment)
        .rejection
        .expect("assignment guard")
        .contains("assignments changed"));
    assert_eq!(authority.scenario, before.scenario);
    assert_eq!(authority.formation, before.formation);
    assert_eq!(authority.company, before.company);
}

#[test]
fn spatial_wire_rejects_both_team_overlap_bounds_ids_and_owner_forgery() {
    let (authority, _) = admitted_party();
    for team in [Team::Heroes, Team::Enemies] {
        for mutation in 0..4 {
            let mut forged = authority.snapshot(0);
            let placements = forged.formation.placements_mut(team);
            match mutation {
                0 => placements.get_mut(1).expect("second").rank = 1,
                1 => placements.first_mut().expect("front").rank = 0,
                2 => placements.last_mut().expect("rear").rank = 7,
                _ => placements.first_mut().expect("front").actor = ActorId(u16::MAX),
            }
            assert_invalid_snapshot(&forged);
        }
    }
    let mut forged = authority.snapshot(0);
    *forged.formation.hero_owners.first_mut().expect("rank") = 6;
    assert_invalid_snapshot(&forged);
    let mut forged = authority.snapshot(0);
    *forged.formation.hero_owners.first_mut().expect("rank") = 1;
    assert_invalid_snapshot(&forged);
    let mut authority = PartyAuthority::new(42, true);
    assert!(remove(&mut authority, ActorId(2)).rejection.is_none());
    let mut forged = wire_roundtrip(&authority);
    forged.combat = Some(
        Combat::from_scenario(&forged.catalog, &forged.scenario)
            .expect("roster content compact constructor")
            .snapshot(),
    );
    forged.encounter = 1;
    assert_invalid_snapshot(&forged);
}

#[test]
fn spatial_editor_and_seed_edits_preserve_positions_and_replacement_is_explicit() {
    let mut authority = PartyAuthority::new(42, true);
    assert!(remove(&mut authority, ActorId(2)).rejection.is_none());
    let original = wire_roundtrip(&authority);
    let mut actor = authority
        .scenario
        .heroes
        .iter()
        .find(|a| a.id == ActorId(3))
        .expect("actor")
        .clone();
    actor.actor.max_hp = 91;
    let expected_revision = authority.setup_revision;
    assert!(request(
        &mut authority,
        0,
        SessionCommand::CustomizeActor {
            actor,
            expected_revision
        }
    )
    .rejection
    .is_none());
    assert_eq!(authority.formation, original.formation);
    let expected_revision = authority.setup_revision;
    assert!(request(
        &mut authority,
        0,
        SessionCommand::SetScenarioSeed {
            seed: 91,
            expected_revision
        }
    )
    .rejection
    .is_none());
    assert_eq!(authority.formation, original.formation);
    assert_eq!(authority.scenario.seed, 91);
    let mut wider = authority
        .scenario
        .heroes
        .iter()
        .find(|a| a.id == ActorId(3))
        .expect("actor")
        .clone();
    wider.actor.footprint = 2;
    let expected_revision = authority.setup_revision;
    assert!(request(
        &mut authority,
        0,
        SessionCommand::CustomizeActor {
            actor: wider,
            expected_revision
        }
    )
    .rejection
    .is_some());
    assert_eq!(authority.formation, original.formation);
    let scenario = authority.scenario.clone();
    let expected_revision = authority.setup_revision;
    assert!(request(
        &mut authority,
        0,
        SessionCommand::ConfigureBattle {
            scenario,
            expected_revision
        }
    )
    .rejection
    .is_none());
    assert_eq!(
        authority.formation,
        LobbyFormation::compact(&authority.scenario)
    );
    assert_eq!(authority.formation.rank(ActorId(3)), Some(2));
}

#[test]
fn spatial_successors_require_setup_revision_and_cannot_reposition_frozen_encounter() {
    let mut authority = PartyAuthority::new(42, true);
    assert!(remove(&mut authority, ActorId(2)).rejection.is_none());
    let before = wire_roundtrip(&authority);
    assert!(move_to(&mut authority, ActorId(1), 2).rejection.is_none());
    let after = wire_roundtrip(&authority);
    assert!(after.validate_successor(&before).is_ok());
    let mut stale = after.clone();
    stale.setup_revision = before.setup_revision;
    assert!(stale.validate_successor(&before).is_err());
    let mut authority = PartyAuthority::new(42, true);
    assert!(request(&mut authority, 0, SessionCommand::Start)
        .rejection
        .is_none());
    let before = authority.snapshot(0);
    let mut after = before.clone();
    after.setup_revision += 1;
    after.formation.heroes.first_mut().expect("front").rank = 2;
    assert!(after.validate_successor(&before).is_err());
}

#[test]
fn spatial_empty_reservation_is_a_spectator_until_a_character_is_placed() {
    let mut authority =
        PartyAuthority::with_roster(42, false, &[HeroClass::Gatekeeper]).expect("one character");
    authority.reserve(peer(1)).expect("guest");
    authority.connected(peer(1), true);
    assert!(assign_rank(&mut authority, 6, 1).rejection.is_none());
    authority.connected(peer(1), false);
    assert!(request(&mut authority, 0, SessionCommand::Ready(true))
        .rejection
        .is_none());
    assert!(request(&mut authority, 0, SessionCommand::Start)
        .rejection
        .is_none());
    assert!(!authority.paused());
    wire_roundtrip(&authority);
}
