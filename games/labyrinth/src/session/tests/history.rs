use super::*;
use crate::session::history::{
    EncounterHistory, HistoryError, HistoryPage, HistoryRequest, HISTORY_PAGE_BYTES,
    HISTORY_PAGE_EVENTS,
};
use labyrinth_rules::{CombatEvent, CombatEventKind};

fn page_request(authority: &PartyAuthority, from: u64) -> HistoryRequest {
    HistoryRequest {
        request_id: 1,
        encounter: authority.encounter,
        from,
        limit: HISTORY_PAGE_EVENTS as u16,
    }
}

fn long_archive(authority: &mut PartyAuthority) {
    authority.record((1..=241).map(|round| CombatEvent {
        id: round as u64 + 100,
        kind: CombatEventKind::RoundStarted { round },
    }));
}

#[test]
fn complete_archive_preserves_initial_outcomes_beyond_recent_window_without_query_side_effects() {
    let (mut authority, _) = started_party();
    let initial = authority.snapshot(0).events;
    assert!(matches!(
        initial.first().expect("initial event").event.kind,
        CombatEventKind::RoundStarted { round: 1 }
    ));
    long_archive(&mut authority);
    let before = authority.snapshot(0);
    assert_eq!(before.events.len(), LOG_LIMIT);
    assert_eq!(before.log.len(), LOG_LIMIT);
    assert_eq!(
        before.history.next - before.history.first,
        initial.len() as u64 + 241
    );
    before.validate().expect("bounded recent snapshot");
    let mut recovered = Vec::new();
    let mut from = before.history.first;
    while from < before.history.next {
        let page = authority
            .history_page(0, page_request(&authority, from))
            .expect("page");
        assert!(page.events.len() <= HISTORY_PAGE_EVENTS);
        assert!(serde_json::to_vec(&page).expect("encoding").len() <= HISTORY_PAGE_BYTES);
        let roundtrip: HistoryPage =
            serde_json::from_slice(&serde_json::to_vec(&page).expect("valid history fixture"))
                .expect("validated page");
        assert_eq!(roundtrip, page);
        from = page.events.last().expect("nonempty prefix").id + 1;
        recovered.extend(page.events);
    }
    assert_eq!(
        recovered
            .get(..initial.len())
            .expect("initial prefix retained"),
        &initial
    );
    assert_eq!(
        recovered,
        authority.events.iter().cloned().collect::<Vec<_>>()
    );
    assert_eq!(
        authority.snapshot(0),
        before,
        "reads cannot mutate state, revision or sequence"
    );
}

#[test]
fn pages_reject_unadmitted_stale_invalid_ranges_and_oversized_decoding() {
    let (mut authority, peers) = started_party();
    long_archive(&mut authority);
    let valid = page_request(&authority, authority.history_bounds().first);
    let before = authority.snapshot(0);
    for invalid in [
        HistoryRequest { from: 0, ..valid },
        HistoryRequest {
            from: authority.next_event + 1,
            ..valid
        },
        HistoryRequest { limit: 0, ..valid },
        HistoryRequest { limit: 65, ..valid },
        HistoryRequest {
            request_id: 0,
            ..valid
        },
    ] {
        assert_eq!(
            authority.history_page(0, invalid),
            Err(HistoryError::InvalidRange)
        );
    }
    assert_eq!(
        authority.history_page(
            0,
            HistoryRequest {
                encounter: valid.encounter + 1,
                ..valid
            }
        ),
        Err(HistoryError::Unavailable)
    );
    assert_eq!(
        authority.history_page(PLAYER_CAPACITY, valid),
        Err(HistoryError::Unavailable)
    );
    assert_eq!(authority.snapshot(0), before);
    authority.connected(peers[0], false);
    assert_eq!(
        authority.history_page(1, valid),
        Err(HistoryError::Unavailable)
    );
    let page = authority.history_page(0, valid).expect("host page");
    let mut malformed = serde_json::to_value(&page).expect("valid history fixture");
    malformed
        .get_mut("events")
        .expect("events field")
        .as_array_mut()
        .expect("valid history fixture")
        .push(
            serde_json::to_value(page.events.first().expect("first event"))
                .expect("valid history fixture"),
        );
    assert!(serde_json::from_value::<HistoryPage>(malformed).is_err());
    let mut unordered = page.clone();
    unordered.events.swap(0, 1);
    assert!(unordered.validate().is_err());
    let mut wrong_bounds = page;
    wrong_bounds.bounds.next = wrong_bounds.from + 1;
    assert!(wrong_bounds.validate().is_err());
}

#[test]
fn cache_recovers_gaps_deduplicates_overlap_and_rejects_conflicts_or_stale_encounters() {
    let (mut authority, _) = started_party();
    long_archive(&mut authority);
    let snapshot = authority.snapshot(0);
    let mut cache = EncounterHistory::default();
    cache
        .observe(snapshot.encounter, snapshot.history, &snapshot.events)
        .expect("recent");
    assert_eq!(cache.loaded_len(), LOG_LIMIT);
    assert!(!cache.complete());
    let first = authority
        .history_page(0, page_request(&authority, snapshot.history.first))
        .expect("valid history fixture");
    cache.merge_page(&first).expect("older page");
    let revision = cache.revision;
    cache.merge_page(&first).expect("overlap is idempotent");
    assert_eq!(cache.revision, revision);
    let mut conflict = first.clone();
    conflict.events.first_mut().expect("first event").event.kind =
        CombatEventKind::RoundStarted { round: 999 };
    assert!(cache.merge_page(&conflict).is_err());
    assert_eq!(
        cache.revision, revision,
        "rejected page cannot partly mutate cache"
    );
    while let Some(from) = cache.first_missing() {
        let page = authority
            .history_page(0, page_request(&authority, from))
            .expect("valid history fixture");
        cache.merge_page(&page).expect("valid history fixture");
    }
    assert!(cache.complete());
    assert_eq!(cache.loaded_len(), authority.events.len());
    assert_eq!(
        cache.page(snapshot.history.first, usize::MAX).len(),
        HISTORY_PAGE_EVENTS
    );
    let new_events = [PresentedEvent {
        id: snapshot.history.next + 81,
        event: CombatEvent {
            id: 900,
            kind: CombatEventKind::RoundStarted { round: 900 },
        },
    }];
    cache
        .observe(
            snapshot.encounter,
            crate::view::HistoryBounds {
                next: snapshot.history.next + 82,
                ..snapshot.history
            },
            &new_events,
        )
        .expect("valid history fixture");
    assert_eq!(
        cache.first_missing(),
        Some(snapshot.history.next),
        "missed recent window leaves a recoverable gap"
    );
    let old_revision = cache.revision;
    let next = crate::view::HistoryBounds {
        first: snapshot.history.next + 82,
        next: snapshot.history.next + 82,
    };
    cache
        .observe(snapshot.encounter + 1, next, &[])
        .expect("valid history fixture");
    assert!(cache.revision > old_revision);
    assert_eq!(cache.loaded_len(), 0);
    assert!(cache.merge_page(&first).is_err());
    assert_eq!(
        cache.loaded_len(),
        0,
        "stale page cannot repopulate a new encounter"
    );
}

#[test]
fn rematch_releases_archive_and_a_new_encounter_starts_after_previous_ids() {
    let (mut authority, _) = started_party();
    long_archive(&mut authority);
    let previous = authority.snapshot(0);
    request(&mut authority, 0, SessionCommand::Rematch);
    let lobby = authority.snapshot(0);
    assert!(lobby.events.is_empty());
    assert!(lobby.history.is_empty());
    assert_eq!(lobby.history.first, previous.history.next);
    assert_eq!(
        authority.history_page(
            0,
            HistoryRequest {
                encounter: previous.encounter,
                ..page_request(&authority, previous.history.first)
            }
        ),
        Err(HistoryError::Unavailable)
    );
    for slot in 0..PLAYER_CAPACITY {
        request(&mut authority, slot, SessionCommand::Ready(true));
    }
    assert_eq!(
        request(&mut authority, 0, SessionCommand::Start).rejection,
        None
    );
    let current = authority.snapshot(0);
    assert!(current.encounter > previous.encounter);
    assert_eq!(
        current.events.first().expect("initial event").id,
        previous.history.next
    );
    assert!(matches!(
        current.events.first().expect("initial event").event.kind,
        CombatEventKind::RoundStarted { round: 1 }
    ));
}
