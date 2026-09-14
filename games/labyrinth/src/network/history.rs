//! Bounded authenticated history reads, independent of gameplay request sequences.

use std::collections::VecDeque;

use super::*;

const QUEUED_PER_CONNECTION: usize = 8;
const PAGE_TIMEOUT: Duration = Duration::from_secs(5);

#[derive(Default)]
pub(super) struct HistoryQueues(BTreeMap<Entity, VecDeque<HistoryRequest>>);

#[derive(Resource, Default)]
pub(super) struct HistoryTransfer {
    pending: Option<(HistoryRequest, SessionId, Instant)>,
    preferred: Option<(u64, u64)>,
    next_request: u64,
}

pub(super) fn cancel(world: &mut World) {
    let mut transfer = world.resource_mut::<HistoryTransfer>();
    transfer.pending = None;
    transfer.preferred = None;
}

pub(super) fn clear(world: &mut World) {
    cancel(world);
    world.insert_resource(EncounterHistory::default());
}

pub(super) fn observe_snapshot(
    world: &mut World,
    snapshot: &SessionSnapshot,
) -> Result<(), &'static str> {
    if world.resource::<EncounterHistory>().encounter != snapshot.encounter {
        cancel(world);
    }
    world.resource_mut::<EncounterHistory>().observe(
        snapshot.encounter,
        snapshot.history,
        &snapshot.events,
    )
}

pub(super) fn prioritize(world: &mut World, encounter: u64, from: u64) {
    let history = world.resource::<EncounterHistory>();
    if history.encounter == encounter
        && from >= history.bounds.first
        && from < history.bounds.next
        && !history.contains(from)
    {
        world.resource_mut::<HistoryTransfer>().preferred = Some((encounter, from));
    }
}

pub(super) fn dispatch(world: &mut World, hosted: &mut Hosted) {
    let admitted = hosted
        .connections
        .keys()
        .copied()
        .filter(|entity| {
            hosted.security.is_connection_admitted(entity.to_bits())
                && world.get::<ConnectedClient>(*entity).is_some()
                && world.get::<InboundRejected>(*entity).is_none()
                && !hosted.rejected.contains_key(entity)
        })
        .collect::<BTreeSet<_>>();
    hosted
        .history
        .0
        .retain(|entity, _| admitted.contains(entity));
    let mut overflow = BTreeSet::new();
    for message in world
        .resource_mut::<Messages<FromClient<HistoryRequest>>>()
        .drain()
    {
        let Some(entity) = message.client_id.entity() else {
            continue;
        };
        if !admitted.contains(&entity) || overflow.contains(&entity) {
            continue;
        }
        let queue = hosted.history.0.entry(entity).or_default();
        if queue.len() == QUEUED_PER_CONNECTION {
            hosted.history.0.remove(&entity);
            overflow.insert(entity);
        } else {
            queue.push_back(message.message);
        }
    }
    for entity in overflow {
        world
            .entity_mut(entity)
            .insert(InboundRejected)
            .remove::<AuthorizedClient>();
        world.trigger(Disconnect::new(entity, "history request queue full"));
    }
    // One page per admitted connection per tick: a noisy peer cannot monopolize
    // archive copying or starve gameplay dispatch, which runs before this handler.
    for (entity, queue) in &mut hosted.history.0 {
        let Some(request) = queue.pop_front() else {
            continue;
        };
        let Some((_, slot)) = hosted.connections.get(entity).copied() else {
            continue;
        };
        let Some(attempt) = hosted.attempts.get(entity).copied() else {
            continue;
        };
        let result = world
            .resource::<PartyAuthority>()
            .history_page(slot, request);
        to_client(
            world,
            *entity,
            HistoryReply {
                attempt,
                request,
                result,
            },
        );
    }
    hosted.history.0.retain(|_, queue| !queue.is_empty());
}

pub(super) fn receive(world: &mut World) {
    for reply in drain::<HistoryReply>(world) {
        let runtime = world.resource::<Runtime>();
        let pending = world.resource::<HistoryTransfer>().pending;
        if runtime.role != Role::Guest
            || !runtime.admitted
            || runtime.attempt != Some(reply.attempt)
            || !runtime
                .connection
                .is_some_and(|entity| world.get::<AuthenticatedPeer>(entity).is_some())
            || !pending.is_some_and(|(request, attempt, _)| {
                request == reply.request && attempt == reply.attempt
            })
            || reply.request.encounter != world.resource::<EncounterHistory>().encounter
        {
            continue;
        }
        world.resource_mut::<HistoryTransfer>().pending = None;
        let Ok(page) = reply.result else {
            continue;
        };
        let valid = page.request_id == reply.request.request_id
            && page.encounter == reply.request.encounter
            && page.from == reply.request.from
            && page.events.len() <= usize::from(reply.request.limit);
        if !valid
            || world
                .resource_mut::<EncounterHistory>()
                .merge_page(&page)
                .is_err()
        {
            start::disconnect_guest(world);
            notice(world, "The host sent an invalid encounter history page.");
        }
    }
}

pub(super) fn tick(world: &mut World) {
    let runtime = world.resource::<Runtime>();
    if !runtime.admitted {
        cancel(world);
        return;
    }
    let role = runtime.role;
    let attempt = runtime.attempt;
    let slot = runtime.player;
    let Some(slot) = slot else {
        return;
    };
    if world
        .resource::<HistoryTransfer>()
        .pending
        .is_some_and(|(_, expected, sent)| {
            attempt == Some(expected) && sent.elapsed() < PAGE_TIMEOUT
        })
    {
        return;
    }
    world.resource_mut::<HistoryTransfer>().pending = None;
    let preferred = world.resource_mut::<HistoryTransfer>().preferred.take();
    let history = world.resource::<EncounterHistory>();
    let from = preferred
        .filter(|(encounter, from)| {
            *encounter == history.encounter
                && *from >= history.bounds.first
                && *from < history.bounds.next
                && !history.contains(*from)
        })
        .map(|(_, from)| from)
        .or_else(|| history.first_missing());
    let Some(from) = from else {
        return;
    };
    let encounter = history.encounter;
    let limit = (history.bounds.next - from).min(HISTORY_PAGE_EVENTS as u64) as u16;
    let mut transfer = world.resource_mut::<HistoryTransfer>();
    transfer.next_request = transfer.next_request.saturating_add(1).max(1);
    let request = HistoryRequest {
        request_id: transfer.next_request,
        encounter,
        from,
        limit,
    };
    if matches!(role, Role::Host | Role::Local) {
        let result = world
            .resource::<PartyAuthority>()
            .history_page(slot, request);
        if let Ok(page) = result {
            world
                .resource_mut::<EncounterHistory>()
                .merge_page(&page)
                .expect("local archive page matches its recent snapshot");
        }
    } else if role == Role::Guest {
        if let Some(attempt) = attempt {
            world.resource_mut::<HistoryTransfer>().pending =
                Some((request, attempt, Instant::now()));
            world.write_message(request);
        }
    }
}
