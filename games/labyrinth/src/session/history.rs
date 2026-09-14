//! Game-owned encounter archive pages and incremental presentation cache.

use std::collections::BTreeMap;

use bevy::prelude::*;
use serde::{Deserialize, Serialize};

use super::PartyAuthority;
use crate::view::PresentedEvent;

/// Maximum records returned by one history page or presentation range query.
pub const HISTORY_PAGE_EVENTS: usize = 64;
pub(crate) const HISTORY_PAGE_BYTES: usize = 32 * 1024;

/// Half-open range of ordered session event IDs belonging to one encounter.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct HistoryBounds {
    /// First event ID, retained for the entire encounter.
    pub first: u64,
    /// ID immediately after the latest recorded event.
    pub next: u64,
}

impl Default for HistoryBounds {
    fn default() -> Self {
        Self { first: 1, next: 1 }
    }
}

impl HistoryBounds {
    pub(crate) fn validate(self) -> Result<(), &'static str> {
        if self.first == 0 || self.first > self.next {
            return Err("Invalid encounter history bounds.");
        }
        Ok(())
    }

    /// Whether this encounter has no recorded outcomes yet.
    pub fn is_empty(self) -> bool {
        self.first == self.next
    }
}

#[derive(Message, Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) struct HistoryRequest {
    pub request_id: u64,
    pub encounter: u64,
    pub from: u64,
    pub limit: u16,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) enum HistoryError {
    Unavailable,
    InvalidRange,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(try_from = "UncheckedHistoryPage")]
pub(crate) struct HistoryPage {
    pub request_id: u64,
    pub encounter: u64,
    pub bounds: HistoryBounds,
    pub from: u64,
    pub events: Vec<PresentedEvent>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct UncheckedHistoryPage {
    request_id: u64,
    encounter: u64,
    bounds: HistoryBounds,
    from: u64,
    #[serde(deserialize_with = "bounded_events")]
    events: Vec<PresentedEvent>,
}

fn bounded_events<'de, D: serde::Deserializer<'de>>(
    deserializer: D,
) -> Result<Vec<PresentedEvent>, D::Error> {
    struct Visitor;
    impl<'de> serde::de::Visitor<'de> for Visitor {
        type Value = Vec<PresentedEvent>;
        fn expecting(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            write!(f, "at most {HISTORY_PAGE_EVENTS} encounter events")
        }
        fn visit_seq<A: serde::de::SeqAccess<'de>>(
            self,
            mut sequence: A,
        ) -> Result<Self::Value, A::Error> {
            if sequence
                .size_hint()
                .is_some_and(|size| size > HISTORY_PAGE_EVENTS)
            {
                return Err(serde::de::Error::custom(
                    "History page exceeds event limit.",
                ));
            }
            let mut events = Vec::new();
            while let Some(event) = sequence.next_element()? {
                if events.len() == HISTORY_PAGE_EVENTS {
                    return Err(serde::de::Error::custom(
                        "History page exceeds event limit.",
                    ));
                }
                events.push(event);
            }
            Ok(events)
        }
    }
    deserializer.deserialize_seq(Visitor)
}

impl TryFrom<UncheckedHistoryPage> for HistoryPage {
    type Error = &'static str;
    fn try_from(value: UncheckedHistoryPage) -> Result<Self, Self::Error> {
        let page = Self {
            request_id: value.request_id,
            encounter: value.encounter,
            bounds: value.bounds,
            from: value.from,
            events: value.events,
        };
        page.validate()?;
        Ok(page)
    }
}

impl HistoryPage {
    pub(crate) fn validate(&self) -> Result<(), &'static str> {
        self.bounds.validate()?;
        if self.request_id == 0
            || self.encounter == 0
            || self.from < self.bounds.first
            || self.from > self.bounds.next
            || self.events.len() > HISTORY_PAGE_EVENTS
            || (self.from < self.bounds.next && self.events.is_empty())
            || self.events.iter().enumerate().any(|(offset, item)| {
                self.from.checked_add(offset as u64) != Some(item.id)
                    || item.id >= self.bounds.next
                    || item.event.id == 0
            })
        {
            return Err("Invalid encounter history page.");
        }
        if serde_json::to_vec(self)
            .map_err(|_| "Cannot encode history page.")?
            .len()
            > HISTORY_PAGE_BYTES
        {
            return Err("History page exceeds byte limit.");
        }
        Ok(())
    }
}

impl PartyAuthority {
    pub(crate) fn history_bounds(&self) -> HistoryBounds {
        HistoryBounds {
            first: self
                .events
                .front()
                .map_or(self.next_event, |event| event.id),
            next: self.next_event,
        }
    }

    /// A read-only query: admission is game-owned and no command watermark moves.
    pub(crate) fn history_page(
        &self,
        slot: u8,
        request: HistoryRequest,
    ) -> Result<HistoryPage, HistoryError> {
        if self.combat.is_none()
            || request.encounter != self.encounter
            || !self
                .players
                .iter()
                .any(|player| player.slot == slot && player.occupied && player.connected)
        {
            return Err(HistoryError::Unavailable);
        }
        let bounds = self.history_bounds();
        if request.request_id == 0
            || request.limit == 0
            || usize::from(request.limit) > HISTORY_PAGE_EVENTS
            || request.from < bounds.first
            || request.from > bounds.next
        {
            return Err(HistoryError::InvalidRange);
        }
        let offset =
            usize::try_from(request.from - bounds.first).map_err(|_| HistoryError::InvalidRange)?;
        let end = offset
            .saturating_add(usize::from(request.limit))
            .min(self.events.len());
        let mut page = HistoryPage {
            request_id: request.request_id,
            encounter: self.encounter,
            bounds,
            from: request.from,
            events: Vec::with_capacity(end - offset),
        };
        for event in self.events.range(offset..end) {
            page.events.push(event.clone());
            // JSON is a conservative byte bound for these typed records. Check
            // the complete page, including its range metadata, before sending.
            if serde_json::to_vec(&page)
                .map_err(|_| HistoryError::Unavailable)?
                .len()
                > HISTORY_PAGE_BYTES
            {
                page.events.pop();
                break;
            }
        }
        page.validate().map_err(|_| HistoryError::Unavailable)?;
        Ok(page)
    }
}

/// Incrementally recovered, non-authoritative history for the displayed encounter.
/// UI queries copy only a bounded range; recent animation input stays separate.
#[derive(Resource, Debug, Default)]
pub struct EncounterHistory {
    /// The displayed encounter. A different hosted session also clears this resource.
    pub encounter: u64,
    /// Complete authoritative extent, even while older pages are loading.
    pub bounds: HistoryBounds,
    /// Changes only when the extent or cached records change.
    pub revision: u64,
    events: BTreeMap<u64, PresentedEvent>,
    first_missing: u64,
}

impl EncounterHistory {
    /// Seed a standalone presentation with a complete contiguous encounter log.
    /// Live sessions use authenticated snapshots/pages; this does not create authority.
    pub fn from_events(encounter: u64, events: &[PresentedEvent]) -> Result<Self, &'static str> {
        let first = events.first().map_or(1, |event| event.id);
        let next = events
            .last()
            .map_or(Some(first), |event| event.id.checked_add(1))
            .ok_or("Encounter event ID overflow.")?;
        let mut history = Self::default();
        history.observe(encounter, HistoryBounds { first, next }, events)?;
        Ok(history)
    }

    /// Copy cached records in `[from, from + min(limit, 64))`, in event-ID order.
    /// Missing ranges stay missing; callers can request them through a history intent.
    pub fn page(&self, from: u64, limit: usize) -> Vec<PresentedEvent> {
        let end = from.saturating_add(limit.min(HISTORY_PAGE_EVENTS) as u64);
        self.events
            .range(from..end)
            .map(|(_, event)| event.clone())
            .collect()
    }

    /// Number of distinct records recovered so far.
    pub fn loaded_len(&self) -> usize {
        self.events.len()
    }

    /// Whether every record in the announced encounter extent has been recovered.
    pub fn complete(&self) -> bool {
        self.bounds.is_empty() || self.first_missing >= self.bounds.next
    }

    pub(crate) fn first_missing(&self) -> Option<u64> {
        (!self.complete()).then_some(self.first_missing)
    }

    pub(crate) fn contains(&self, event: u64) -> bool {
        self.events.contains_key(&event)
    }

    pub(crate) fn observe(
        &mut self,
        encounter: u64,
        bounds: HistoryBounds,
        recent: &[PresentedEvent],
    ) -> Result<(), &'static str> {
        bounds.validate()?;
        if encounter < self.encounter {
            return Err("Stale encounter history.");
        }
        if encounter != self.encounter || self.first_missing == 0 {
            self.encounter = encounter;
            self.bounds = bounds;
            self.events.clear();
            self.first_missing = bounds.first;
            self.revision = self.revision.saturating_add(1);
        } else if bounds.first != self.bounds.first || bounds.next < self.bounds.next {
            return Err("History extent moved backwards.");
        }
        self.check_overlap(bounds, recent)?;
        if self.bounds != bounds {
            self.bounds = bounds;
            self.revision = self.revision.saturating_add(1);
        }
        self.insert(recent);
        Ok(())
    }

    pub(crate) fn merge_page(&mut self, page: &HistoryPage) -> Result<(), &'static str> {
        page.validate()?;
        if page.encounter != self.encounter
            || page.bounds.first != self.bounds.first
            || page.from < self.bounds.first
            || page.from > self.bounds.next
        {
            return Err("Stale encounter history page.");
        }
        // Pages and snapshots use independent channels. Only snapshots grow the
        // known extent, and requests never ask beyond that extent.
        self.check_overlap(self.bounds, &page.events)?;
        self.insert(&page.events);
        Ok(())
    }

    fn check_overlap(
        &self,
        bounds: HistoryBounds,
        events: &[PresentedEvent],
    ) -> Result<(), &'static str> {
        if events.iter().any(|event| {
            event.id < bounds.first
                || event.id >= bounds.next
                || self
                    .events
                    .get(&event.id)
                    .is_some_and(|known| known != event)
        }) || events
            .array_windows::<2>()
            .any(|[a, b]| a.id.checked_add(1) != Some(b.id))
        {
            return Err("Conflicting or invalid history records.");
        }
        Ok(())
    }

    fn insert(&mut self, events: &[PresentedEvent]) {
        let before = self.events.len();
        for event in events {
            self.events.entry(event.id).or_insert_with(|| event.clone());
        }
        if self.events.len() != before {
            self.revision = self.revision.saturating_add(1);
        }
        // Each ID is walked at most once as gaps close, rather than rescanning
        // the whole archive whenever another recent snapshot arrives.
        while self.events.contains_key(&self.first_missing) {
            self.first_missing += 1;
        }
    }
}
