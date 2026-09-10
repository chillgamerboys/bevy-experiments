//! Physical-connection FIFO queues; authority alone owns sequences and results.

use super::*;
use std::collections::VecDeque;

const PENDING_PER_CONNECTION: usize = 128;
const DISPATCH_PER_CONNECTION: usize = 16;
const DISPATCH_PER_TICK: usize = 128;

#[derive(Default)]
pub(super) struct RequestQueues {
    pending: BTreeMap<Entity, VecDeque<GameRequest>>,
    last: Option<Entity>,
}

impl RequestQueues {
    fn push(&mut self, connection: Entity, request: GameRequest) -> Result<(), ()> {
        let queue = self.pending.entry(connection).or_default();
        if queue.len() >= PENDING_PER_CONNECTION {
            self.pending.remove(&connection);
            return Err(());
        }
        queue.push_back(request);
        Ok(())
    }

    fn take(&mut self, global: usize, per_connection: usize) -> Vec<(Entity, GameRequest)> {
        let mut ready = Vec::new();
        let mut counts = BTreeMap::<Entity, usize>::new();
        for _ in 0..global {
            let eligible = |entity: &Entity, queue: &VecDeque<GameRequest>| {
                !queue.is_empty() && counts.get(entity).copied().unwrap_or(0) < per_connection
            };
            let next = self
                .pending
                .iter()
                .find(|(entity, queue)| {
                    self.last.is_none_or(|last| **entity > last) && eligible(entity, queue)
                })
                .or_else(|| {
                    self.pending
                        .iter()
                        .find(|(entity, queue)| eligible(entity, queue))
                })
                .map(|(entity, _)| *entity);
            let Some(entity) = next else {
                break;
            };
            if let Some(request) = self.pending.get_mut(&entity).and_then(VecDeque::pop_front) {
                *counts.entry(entity).or_default() += 1;
                self.last = Some(entity);
                ready.push((entity, request));
            }
        }
        self.pending.retain(|_, queue| !queue.is_empty());
        ready
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
        .requests
        .pending
        .retain(|entity, _| admitted.contains(entity));
    let mut overflow = BTreeSet::new();
    // The transport gate bounds production input. Do not make a second unbounded
    // copy, and do not let a global prefix decide which connection gets service.
    for request in world
        .resource_mut::<Messages<FromClient<GameRequest>>>()
        .drain()
    {
        let Some(entity) = request.client_id.entity() else {
            continue;
        };
        if admitted.contains(&entity)
            && !overflow.contains(&entity)
            && hosted.requests.push(entity, request.message).is_err()
        {
            overflow.insert(entity);
        }
    }
    for entity in overflow {
        world
            .entity_mut(entity)
            .insert(InboundRejected)
            .remove::<AuthorizedClient>();
        world.trigger(Disconnect::new(entity, "game request queue full"));
    }
    for (entity, request) in hosted
        .requests
        .take(DISPATCH_PER_TICK, DISPATCH_PER_CONNECTION)
    {
        let Some((_, slot)) = hosted.connections.get(&entity).copied() else {
            continue;
        };
        let result = world.resource_mut::<PartyAuthority>().apply(slot, request);
        to_client(world, entity, result);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn request(sequence: u64) -> GameRequest {
        GameRequest {
            sequence,
            encounter: 0,
            decision: 0,
            command: SessionCommand::Ready(true),
        }
    }

    #[test]
    fn noisy_prefix_does_not_discard_quiet_peer_and_fifo_survives_ticks() {
        let mut world = World::new();
        let noisy = world.spawn_empty().id();
        let quiet = world.spawn_empty().id();
        let mut queues = RequestQueues::default();
        for sequence in 1..=128 {
            queues.push(noisy, request(sequence)).expect("capacity");
        }
        queues.push(quiet, request(1)).expect("quiet");
        let first = queues.take(128, 16);
        assert_eq!(first.iter().filter(|(id, _)| *id == noisy).count(), 16);
        assert_eq!(first.iter().filter(|(id, _)| *id == quiet).count(), 1);
        let mut sequences = first
            .into_iter()
            .filter(|(id, _)| *id == noisy)
            .map(|(_, req)| req.sequence)
            .collect::<Vec<_>>();
        for _ in 0..7 {
            sequences.extend(
                queues
                    .take(128, 16)
                    .into_iter()
                    .map(|(_, req)| req.sequence),
            );
        }
        assert_eq!(sequences, (1..=128).collect::<Vec<_>>());
        assert!(queues.pending.is_empty());
    }

    #[test]
    fn global_cursor_rotates_and_overflow_clears_only_its_connection() {
        let mut world = World::new();
        let a = world.spawn_empty().id();
        let b = world.spawn_empty().id();
        let mut queues = RequestQueues::default();
        for sequence in 1..=128 {
            queues.push(a, request(sequence)).expect("capacity");
        }
        queues.push(b, request(1)).expect("quiet");
        let first = queues.take(1, 16).pop().expect("first").0;
        let second = queues.take(1, 16).pop().expect("second").0;
        assert_ne!(first, second);
        queues.push(a, request(129)).expect("freed space");
        assert!(queues.push(a, request(130)).is_err());
        assert!(!queues.pending.contains_key(&a));
        queues
            .push(b, request(2))
            .expect("other connection remains usable");
        assert_eq!(
            queues.take(1, 16).pop().expect("quiet result").1.sequence,
            2
        );
    }
}
