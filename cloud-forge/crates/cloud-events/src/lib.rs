//! An event envelope and an in-memory, append-only event log.
//!
//! This is the substrate a future audit trail is built on (the
//! roadmap's **CLOUD-I005: every security-sensitive operation
//! produces an audit event**) -- not itself an audit log, since it
//! has no concept yet of *which* events are security-sensitive or
//! where they should be durably persisted. That's later-phase work;
//! this crate only guarantees the one thing every later use needs:
//! events are appended in strict, gapless, increasing order and
//! nothing already appended can be removed or reordered.
#![forbid(unsafe_code)]

use cloud_types::{ResourceId, Timestamp};

/// A single recorded event.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Event {
    /// Strictly increasing, gapless, starting at 1. Never reused.
    pub id: u64,
    pub source: ResourceId,
    pub kind: String,
    pub timestamp: Timestamp,
    pub detail: String,
}

/// An append-only, strictly-ordered log of [`Event`]s.
#[derive(Debug, Clone, Default)]
pub struct EventLog {
    events: Vec<Event>,
}

impl EventLog {
    pub fn new() -> Self {
        EventLog { events: Vec::new() }
    }

    /// Appends a new event and returns its assigned id.
    pub fn append(
        &mut self,
        source: ResourceId,
        kind: impl Into<String>,
        timestamp: Timestamp,
        detail: impl Into<String>,
    ) -> u64 {
        let id = self.events.len() as u64 + 1;
        self.events.push(Event {
            id,
            source,
            kind: kind.into(),
            timestamp,
            detail: detail.into(),
        });
        id
    }

    pub fn len(&self) -> usize {
        self.events.len()
    }

    pub fn is_empty(&self) -> bool {
        self.events.is_empty()
    }

    /// All events, in append order.
    pub fn events(&self) -> &[Event] {
        &self.events
    }

    /// Events with `id` strictly greater than `after` -- the shape a
    /// consumer resuming from a checkpoint needs. `after = 0` returns
    /// every event.
    pub fn since(&self, after: u64) -> &[Event] {
        // Event ids are 1-based and gapless, so `after` is also the
        // count of events strictly before the cutoff.
        let start = (after as usize).min(self.events.len());
        &self.events[start..]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn source() -> ResourceId {
        ResourceId::new("i-1").unwrap()
    }

    #[test]
    fn append_assigns_strictly_increasing_gapless_ids_starting_at_one() {
        let mut log = EventLog::new();
        let id1 = log.append(source(), "resource.created", Timestamp::EPOCH, "");
        let id2 = log.append(source(), "resource.updated", Timestamp::EPOCH, "");
        let id3 = log.append(source(), "resource.deleted", Timestamp::EPOCH, "");
        assert_eq!((id1, id2, id3), (1, 2, 3));
    }

    #[test]
    fn events_preserves_append_order() {
        let mut log = EventLog::new();
        log.append(source(), "a", Timestamp::EPOCH, "");
        log.append(source(), "b", Timestamp::EPOCH, "");
        log.append(source(), "c", Timestamp::EPOCH, "");
        let kinds: Vec<&str> = log.events().iter().map(|e| e.kind.as_str()).collect();
        assert_eq!(kinds, vec!["a", "b", "c"]);
    }

    #[test]
    fn since_zero_returns_every_event() {
        let mut log = EventLog::new();
        log.append(source(), "a", Timestamp::EPOCH, "");
        log.append(source(), "b", Timestamp::EPOCH, "");
        assert_eq!(log.since(0).len(), 2);
    }

    #[test]
    fn since_a_checkpoint_returns_only_later_events() {
        let mut log = EventLog::new();
        log.append(source(), "a", Timestamp::EPOCH, "");
        let checkpoint = log.append(source(), "b", Timestamp::EPOCH, "");
        log.append(source(), "c", Timestamp::EPOCH, "");
        let later = log.since(checkpoint);
        assert_eq!(later.len(), 1);
        assert_eq!(later[0].kind, "c");
    }

    #[test]
    fn since_a_checkpoint_past_the_end_returns_nothing() {
        let mut log = EventLog::new();
        log.append(source(), "a", Timestamp::EPOCH, "");
        assert!(log.since(999).is_empty());
    }

    #[test]
    fn len_and_is_empty_track_the_log() {
        let mut log = EventLog::new();
        assert!(log.is_empty());
        log.append(source(), "a", Timestamp::EPOCH, "");
        assert_eq!(log.len(), 1);
        assert!(!log.is_empty());
    }
}
