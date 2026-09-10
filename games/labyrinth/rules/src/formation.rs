//! Packed linear formations: entries are identities, never duplicated rank cells.

use crate::{ActorId, CombatSnapshot, Team};

impl CombatSnapshot {
    /// Ordered occupants, including dying actors and corpses.
    #[must_use]
    pub fn formation(&self, team: Team) -> &[ActorId] {
        match team {
            Team::Heroes => &self.hero_formation,
            Team::Enemies => &self.enemy_formation,
        }
    }

    /// All occupied ranks for an actor; a multi-rank target is still one identity.
    #[must_use]
    pub fn ranks(&self, id: ActorId) -> Option<std::ops::RangeInclusive<u8>> {
        let actor = self.actor(id)?;
        let mut front = 1;
        for occupant in self.formation(actor.team()) {
            let width = self.actor(*occupant)?.kind.footprint();
            if *occupant == id {
                return Some(front..=front + width - 1);
            }
            front += width;
        }
        None
    }

    /// Resolve a space to its single occupying identity.
    #[must_use]
    pub fn occupant(&self, team: Team, rank: u8) -> Option<ActorId> {
        self.formation(team)
            .iter()
            .copied()
            .find(|id| self.ranks(*id).is_some_and(|ranks| ranks.contains(&rank)))
    }

    /// Adjacency is between whole footprints, not their leading rank numbers.
    #[must_use]
    pub fn adjacent(&self, left: ActorId, right: ActorId) -> bool {
        self.actor(left)
            .zip(self.actor(right))
            .is_some_and(|(a, b)| {
                a.team() == b.team()
                    && self
                        .ranks(left)
                        .zip(self.ranks(right))
                        .is_some_and(|(a, b)| {
                            a.end() + 1 == *b.start() || b.end() + 1 == *a.start()
                        })
            })
    }
}
