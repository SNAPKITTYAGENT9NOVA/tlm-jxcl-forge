// Copyright (C) 2026 ERRANT DIGITAL INSTITUTE OF TECHNOLOGY (SEIT NGO) % SNAPKITTY COLLECTIVE
// SPDX-License-Identifier: AGPL-3.0-only OR LicenseRef-Commercial
// See LICENSE-AGPL and LICENSE-COMMERCIAL in this repository for license terms.

//! The visibility-timeout mechanism that actually implements
//! [`cloud_delivery::DeliverySemantics::AtLeastOnce`] in practice: a
//! received message is hidden from other consumers until a deadline,
//! and becomes visible again automatically if nobody acknowledges it
//! in time -- which is precisely how "never loses a message, may
//! duplicate" is achieved mechanically, rather than just declared.
//!
//! A [`MessageLease`] owns exactly this: whether a message is
//! currently visible, and how many times it's been received. It has
//! no idea what the message's contents are, and no idea when "now"
//! is -- every method takes the current time as an explicit
//! [`Timestamp`] argument, so a caller (or a test) controls the clock.
#![forbid(unsafe_code)]

use cloud_errors::CloudError;
use cloud_types::Timestamp;

/// A message's visibility state: hidden until a deadline, or visible.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MessageLease {
    visible_at: Timestamp,
    receive_count: u32,
}

impl MessageLease {
    /// A freshly-enqueued message: visible immediately, never received.
    pub fn new() -> Self {
        MessageLease {
            visible_at: Timestamp::EPOCH,
            receive_count: 0,
        }
    }

    pub fn receive_count(&self) -> u32 {
        self.receive_count
    }

    /// Whether the message can be delivered to a consumer at `now`.
    pub fn is_visible(&self, now: Timestamp) -> bool {
        now >= self.visible_at
    }

    /// Receives the message: hides it until `now + visibility_timeout_millis`
    /// and increments the receive count. Rejected (leaving the lease
    /// completely unchanged) if the message is not currently visible
    /// -- i.e. it's already in flight to another consumer.
    pub fn receive(
        &mut self,
        now: Timestamp,
        visibility_timeout_millis: u64,
    ) -> Result<(), CloudError> {
        if !self.is_visible(now) {
            return Err(CloudError::Conflict {
                what: "message lease",
                id: "already in flight".to_string(),
            });
        }
        self.visible_at = now.saturating_add_millis(visibility_timeout_millis);
        self.receive_count += 1;
        Ok(())
    }

    /// Whether this message has been received at least `max_receives`
    /// times and should be routed to a dead-letter queue instead of
    /// redelivered again.
    pub fn should_dead_letter(&self, max_receives: u32) -> bool {
        self.receive_count >= max_receives
    }
}

impl Default for MessageLease {
    fn default() -> Self {
        MessageLease::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ms(millis: u64) -> Timestamp {
        Timestamp::from_millis(millis)
    }

    #[test]
    fn a_new_lease_is_immediately_visible_and_unreceived() {
        let lease = MessageLease::new();
        assert!(lease.is_visible(ms(0)));
        assert_eq!(lease.receive_count(), 0);
    }

    #[test]
    fn receiving_hides_the_message_until_the_timeout_elapses() {
        let mut lease = MessageLease::new();
        lease.receive(ms(0), 30_000).unwrap();
        assert!(!lease.is_visible(ms(1)));
        assert!(!lease.is_visible(ms(29_999)));
    }

    #[test]
    fn the_message_becomes_visible_again_once_the_timeout_elapses() {
        let mut lease = MessageLease::new();
        lease.receive(ms(0), 30_000).unwrap();
        assert!(lease.is_visible(ms(30_000)));
    }

    #[test]
    fn receiving_increments_the_receive_count() {
        let mut lease = MessageLease::new();
        lease.receive(ms(0), 1000).unwrap();
        assert_eq!(lease.receive_count(), 1);
        lease.receive(ms(1000), 1000).unwrap();
        assert_eq!(lease.receive_count(), 2);
    }

    #[test]
    fn receiving_an_already_in_flight_message_is_rejected() {
        let mut lease = MessageLease::new();
        lease.receive(ms(0), 30_000).unwrap();
        let err = lease.receive(ms(100), 30_000).unwrap_err();
        assert!(matches!(err, CloudError::Conflict { .. }));
    }

    #[test]
    fn a_rejected_receive_leaves_the_lease_completely_unchanged() {
        let mut lease = MessageLease::new();
        lease.receive(ms(0), 30_000).unwrap();
        assert!(lease.receive(ms(100), 5000).is_err());
        assert_eq!(lease.receive_count(), 1);
        assert!(!lease.is_visible(ms(29_999)));
        assert!(lease.is_visible(ms(30_000)));
    }

    #[test]
    fn should_dead_letter_reflects_the_configured_threshold() {
        let mut lease = MessageLease::new();
        lease.receive(ms(0), 100).unwrap();
        lease.receive(ms(100), 100).unwrap();
        assert!(!lease.should_dead_letter(3));
        assert!(lease.should_dead_letter(2));
        assert!(lease.should_dead_letter(1));
    }

    #[test]
    fn repeated_receive_after_visibility_returns_keeps_incrementing_count() {
        let mut lease = MessageLease::new();
        for expected_count in 1..=3u32 {
            let now = ms(u64::from(expected_count - 1) * 100);
            lease.receive(now, 100).unwrap();
            assert_eq!(lease.receive_count(), expected_count);
        }
    }
}
