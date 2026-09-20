// Copyright (C) 2026 ERRANT DIGITAL INSTITUTE OF TECHNOLOGY (SEIT NGO) % SNAPKITTY COLLECTIVE
// SPDX-License-Identifier: AGPL-3.0-only OR LicenseRef-Commercial
// See LICENSE-AGPL and LICENSE-COMMERCIAL in this repository for license terms.

//! The topic-to-subscriber pub/sub topology: which subscriber
//! resources should receive a copy of whatever is published to a
//! given topic resource.
//!
//! [`FanoutRegistry`] owns exactly this mapping and nothing about
//! delivery itself -- it has no idea what a "publish" is, what a
//! message looks like, or how delivery is retried. Actually fanning a
//! published message out to each subscriber (and applying
//! [`cloud_delivery::DeliverySemantics`]/[`cloud_visibility::MessageLease`]
//! per subscriber) is a concern for whatever future service composes
//! all three, exactly as [`cloud_resource_registry`]'s `Arn` registry
//! (Phase 3) owns only the name-to-resource mapping and nothing about
//! provisioning.
#![forbid(unsafe_code)]

use cloud_errors::CloudError;
use cloud_types::ResourceId;
use std::collections::{BTreeMap, BTreeSet};

/// A registry of topic subscriptions.
#[derive(Debug, Clone, Default)]
pub struct FanoutRegistry {
    subscriptions: BTreeMap<ResourceId, BTreeSet<ResourceId>>,
}

impl FanoutRegistry {
    pub fn new() -> Self {
        FanoutRegistry {
            subscriptions: BTreeMap::new(),
        }
    }

    /// Subscribes `subscriber` to `topic`. Rejects a second
    /// subscription of the same pair.
    pub fn subscribe(
        &mut self,
        topic: ResourceId,
        subscriber: ResourceId,
    ) -> Result<(), CloudError> {
        let subscribers = self.subscriptions.entry(topic.clone()).or_default();
        if !subscribers.insert(subscriber.clone()) {
            return Err(CloudError::Conflict {
                what: "subscription",
                id: format!("{topic} -> {subscriber}"),
            });
        }
        Ok(())
    }

    /// Removes `subscriber`'s subscription to `topic`. `NotFound` if
    /// either the topic has no subscribers at all, or `subscriber`
    /// specifically isn't among them.
    pub fn unsubscribe(
        &mut self,
        topic: &ResourceId,
        subscriber: &ResourceId,
    ) -> Result<(), CloudError> {
        let subscribers =
            self.subscriptions
                .get_mut(topic)
                .ok_or_else(|| CloudError::NotFound {
                    what: "topic",
                    id: topic.to_string(),
                })?;
        if !subscribers.remove(subscriber) {
            return Err(CloudError::NotFound {
                what: "subscription",
                id: format!("{topic} -> {subscriber}"),
            });
        }
        Ok(())
    }

    /// `topic`'s subscribers, in deterministic (sorted) order. Empty
    /// for a topic with no subscriptions, rather than an error --
    /// having zero subscribers is a valid, unremarkable state.
    pub fn subscribers(&self, topic: &ResourceId) -> impl Iterator<Item = &ResourceId> {
        self.subscriptions.get(topic).into_iter().flatten()
    }

    pub fn is_subscribed(&self, topic: &ResourceId, subscriber: &ResourceId) -> bool {
        self.subscriptions
            .get(topic)
            .is_some_and(|subscribers| subscribers.contains(subscriber))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn id(s: &str) -> ResourceId {
        ResourceId::new(s).unwrap()
    }

    #[test]
    fn subscribing_then_listing_returns_the_subscriber() {
        let mut reg = FanoutRegistry::new();
        reg.subscribe(id("topic-1"), id("sub-1")).unwrap();
        let subs: Vec<&ResourceId> = reg.subscribers(&id("topic-1")).collect();
        assert_eq!(subs, vec![&id("sub-1")]);
    }

    #[test]
    fn duplicate_subscription_is_a_conflict() {
        let mut reg = FanoutRegistry::new();
        reg.subscribe(id("topic-1"), id("sub-1")).unwrap();
        let err = reg.subscribe(id("topic-1"), id("sub-1")).unwrap_err();
        assert!(matches!(err, CloudError::Conflict { .. }));
    }

    #[test]
    fn unsubscribing_removes_the_subscriber() {
        let mut reg = FanoutRegistry::new();
        reg.subscribe(id("topic-1"), id("sub-1")).unwrap();
        reg.unsubscribe(&id("topic-1"), &id("sub-1")).unwrap();
        assert_eq!(reg.subscribers(&id("topic-1")).count(), 0);
    }

    #[test]
    fn unsubscribing_from_an_unknown_topic_is_not_found() {
        let mut reg = FanoutRegistry::new();
        let err = reg.unsubscribe(&id("topic-1"), &id("sub-1")).unwrap_err();
        assert!(matches!(err, CloudError::NotFound { .. }));
    }

    #[test]
    fn unsubscribing_a_non_subscriber_is_not_found() {
        let mut reg = FanoutRegistry::new();
        reg.subscribe(id("topic-1"), id("sub-1")).unwrap();
        let err = reg.unsubscribe(&id("topic-1"), &id("sub-2")).unwrap_err();
        assert!(matches!(err, CloudError::NotFound { .. }));
    }

    #[test]
    fn a_topic_can_have_multiple_subscribers_listed_in_sorted_order() {
        let mut reg = FanoutRegistry::new();
        reg.subscribe(id("topic-1"), id("sub-2")).unwrap();
        reg.subscribe(id("topic-1"), id("sub-1")).unwrap();
        let subs: Vec<&ResourceId> = reg.subscribers(&id("topic-1")).collect();
        assert_eq!(subs, vec![&id("sub-1"), &id("sub-2")]);
    }

    #[test]
    fn different_topics_have_independent_subscriber_sets() {
        let mut reg = FanoutRegistry::new();
        reg.subscribe(id("topic-1"), id("sub-1")).unwrap();
        reg.subscribe(id("topic-2"), id("sub-2")).unwrap();
        assert!(reg.is_subscribed(&id("topic-1"), &id("sub-1")));
        assert!(!reg.is_subscribed(&id("topic-1"), &id("sub-2")));
        assert!(reg.is_subscribed(&id("topic-2"), &id("sub-2")));
    }

    #[test]
    fn is_subscribed_reflects_registration_state() {
        let mut reg = FanoutRegistry::new();
        assert!(!reg.is_subscribed(&id("topic-1"), &id("sub-1")));
        reg.subscribe(id("topic-1"), id("sub-1")).unwrap();
        assert!(reg.is_subscribed(&id("topic-1"), &id("sub-1")));
    }
}
