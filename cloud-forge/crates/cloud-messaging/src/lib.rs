// Copyright (C) 2026 ERRANT DIGITAL INSTITUTE OF TECHNOLOGY (SEIT NGO) % SNAPKITTY COLLECTIVE
// SPDX-License-Identifier: AGPL-3.0-only OR LicenseRef-Commercial
// See LICENSE-AGPL and LICENSE-COMMERCIAL in this repository for license terms.

//! `MessagingService`: the fourth and last of the originally-planned
//! composed services, following the shape `cloud-compute` (Phase 8),
//! `cloud-storage` (Phase 9), and `cloud-database` (Phase 10)
//! established -- compose already-real primitives, introduce none.
//!
//! Unlike those three, this service has **two** top-level resource
//! kinds instead of one: a queue (a [`QueueSpec`], holding messages
//! directly) and a topic (a [`TopicSpec`], holding no messages of its
//! own, only a fan-out list of subscriber queues via
//! [`cloud_fanout::FanoutRegistry`]). This is exactly the composition
//! `cloud-fanout`'s own doc comment named as deferred: "actually
//! fanning a published message out to each subscriber ... is a
//! concern for whatever future service composes all three [delivery,
//! visibility, fanout]." This is that service.
//!
//! A message enqueued directly (`enqueue`) or fanned out from a topic
//! (`publish`) is tracked as a [`cloud_visibility::MessageLease`], so
//! `receive` inherits real at-least-once mechanics for free: a message
//! already in flight cannot be received again until its visibility
//! timeout elapses, and a message received too many times is routed to
//! a per-queue dead-letter set instead of redelivered forever.
//!
//! This phase adds two cross-cutting gates, each of a kind already
//! established in an earlier composed service but applied to a new
//! operation:
//!
//! - `subscribe` refuses to link a queue to a topic unless the queue's
//!   own [`cloud_delivery::DeliverySemantics`] satisfies the topic's
//!   required semantics ([`DeliverySemantics::satisfies`]) -- a
//!   cross-primitive check at *subscribe* time, mirroring
//!   `cloud-storage`'s cross-primitive check at *delete* time
//!   (Phase 9: a volume's own attachment state gated its deletion;
//!   here, a queue's own delivery spec gates its subscription).
//! - `delete_queue`/`delete_topic` each refuse while their own
//!   resource still holds live state -- unacknowledged messages for a
//!   queue, remaining subscribers for a topic -- mirroring
//!   `cloud-database`'s own-state delete gate (Phase 10: a database
//!   with remaining snapshots could not be deleted).
//!
//! `publish` validates every subscriber has room for the new message
//! id *before* delivering to any of them, so a duplicate message id
//! collision partway through a fan-out never leaves some subscribers
//! holding the message and others without it -- the same
//! validate-before-apply discipline every pipeline in this workspace
//! follows.
#![forbid(unsafe_code)]

use cloud_account::AccountRegistry;
use cloud_delivery::DeliverySemantics;
use cloud_errors::CloudError;
use cloud_events::EventLog;
use cloud_fanout::FanoutRegistry;
use cloud_identity::Principal;
use cloud_lifecycle::Lifecycle;
use cloud_policy::Policy;
use cloud_quota::QuotaTracker;
use cloud_region::RegionRegistry;
use cloud_resource::Resource;
use cloud_resource_registry::ResourceRegistry;
use cloud_types::{AccountId, Arn, RegionId, ResourceId, ResourceType, Timestamp};
use cloud_visibility::MessageLease;
use std::collections::{BTreeMap, BTreeSet};

/// A queue's service-specific payload.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct QueueSpec {
    pub delivery: DeliverySemantics,
    pub visibility_timeout_millis: u64,
    pub max_receives: u32,
}

/// A request to create one new queue.
pub struct CreateQueueRequest {
    pub id: ResourceId,
    pub account: AccountId,
    pub region: RegionId,
    pub principal: Principal,
    pub delivery: DeliverySemantics,
    pub visibility_timeout_millis: u64,
    pub max_receives: u32,
    pub created_at: Timestamp,
}

/// A topic's service-specific payload: the minimum delivery guarantee
/// every subscriber must offer.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TopicSpec {
    pub required_delivery: DeliverySemantics,
}

/// A request to create one new topic.
pub struct CreateTopicRequest {
    pub id: ResourceId,
    pub account: AccountId,
    pub region: RegionId,
    pub principal: Principal,
    pub required_delivery: DeliverySemantics,
    pub created_at: Timestamp,
}

/// The outcome of a single `receive` call.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ReceiveOutcome {
    /// The message was successfully leased to the caller.
    Delivered { receive_count: u32 },
    /// The message had already been received `max_receives` times and
    /// has been moved to the queue's dead-letter set instead.
    DeadLettered,
}

/// The messaging service: holds every registry `create_queue`/
/// `create_topic` and friends need, plus the stores of queues, topics,
/// in-flight messages, dead letters, and topic subscriptions.
pub struct MessagingService {
    partition: String,
    service: String,
    accounts: AccountRegistry,
    regions: RegionRegistry,
    policy: Policy,
    quota: QuotaTracker,
    events: EventLog,
    queue_names: ResourceRegistry,
    topic_names: ResourceRegistry,
    queues: BTreeMap<ResourceId, Resource<QueueSpec>>,
    topics: BTreeMap<ResourceId, Resource<TopicSpec>>,
    messages: BTreeMap<ResourceId, BTreeMap<ResourceId, MessageLease>>,
    dead_letters: BTreeMap<ResourceId, BTreeSet<ResourceId>>,
    fanout: FanoutRegistry,
}

impl MessagingService {
    pub fn new(policy: Policy, partition: impl Into<String>, service: impl Into<String>) -> Self {
        MessagingService {
            partition: partition.into(),
            service: service.into(),
            accounts: AccountRegistry::new(),
            regions: RegionRegistry::new(),
            policy,
            quota: QuotaTracker::new(),
            events: EventLog::new(),
            queue_names: ResourceRegistry::new(),
            topic_names: ResourceRegistry::new(),
            queues: BTreeMap::new(),
            topics: BTreeMap::new(),
            messages: BTreeMap::new(),
            dead_letters: BTreeMap::new(),
            fanout: FanoutRegistry::new(),
        }
    }

    pub fn register_account(&mut self, account: cloud_account::Account) -> Result<(), CloudError> {
        self.accounts.register(account)
    }

    pub fn register_region(&mut self, region: RegionId, az_count: u32) -> Result<(), CloudError> {
        self.regions.register(region, az_count)
    }

    pub fn set_quota_limit(&mut self, resource_type: impl Into<String>, limit: u64) {
        self.quota.set_limit(resource_type, limit);
    }

    pub fn events(&self) -> &EventLog {
        &self.events
    }

    pub fn get_queue(&self, id: &ResourceId) -> Option<&Resource<QueueSpec>> {
        self.queues.get(id)
    }

    pub fn get_topic(&self, id: &ResourceId) -> Option<&Resource<TopicSpec>> {
        self.topics.get(id)
    }

    pub fn list_queues(&self) -> impl Iterator<Item = &Resource<QueueSpec>> {
        self.queues.values()
    }

    pub fn list_topics(&self) -> impl Iterator<Item = &Resource<TopicSpec>> {
        self.topics.values()
    }

    pub fn resolve_queue_arn(&self, arn: &Arn) -> Option<&Resource<QueueSpec>> {
        self.queue_names
            .resolve(arn)
            .and_then(|id| self.queues.get(id))
    }

    pub fn queue_arn_of(&self, id: &ResourceId) -> Option<Arn> {
        self.queue_names.arn_of(id)
    }

    pub fn resolve_topic_arn(&self, arn: &Arn) -> Option<&Resource<TopicSpec>> {
        self.topic_names
            .resolve(arn)
            .and_then(|id| self.topics.get(id))
    }

    pub fn topic_arn_of(&self, id: &ResourceId) -> Option<Arn> {
        self.topic_names.arn_of(id)
    }

    /// The message ids a queue has dead-lettered, if the queue exists.
    pub fn dead_letters(&self, id: &ResourceId) -> Option<impl Iterator<Item = &ResourceId>> {
        self.dead_letters.get(id).map(|set| set.iter())
    }

    /// Creates a new queue: validates the account and region exist,
    /// authorizes the request, reserves one unit of the account's
    /// `"queues"` quota, and starts it with an empty inbox and no dead
    /// letters. Any failure after the quota was reserved releases it.
    pub fn create_queue(
        &mut self,
        request: CreateQueueRequest,
    ) -> Result<&Resource<QueueSpec>, CloudError> {
        if !self.accounts.contains(&request.account) {
            return Err(CloudError::NotFound {
                what: "account",
                id: request.account.to_string(),
            });
        }
        if self.regions.get(&request.region).is_none() {
            return Err(CloudError::NotFound {
                what: "region",
                id: request.region.to_string(),
            });
        }
        if self.queues.contains_key(&request.id) {
            return Err(CloudError::Conflict {
                what: "queue",
                id: request.id.to_string(),
            });
        }

        self.policy.authorize(
            &request.principal,
            "messaging:create-queue",
            request.id.as_str(),
        )?;

        self.quota.try_reserve("queues", 1)?;

        let resource_type = ResourceType::new("message-queue").expect("valid literal");
        let resource = Resource::new(
            request.id.clone(),
            resource_type,
            Some(request.region.clone()),
            request.account.clone(),
            request.created_at,
            QueueSpec {
                delivery: request.delivery,
                visibility_timeout_millis: request.visibility_timeout_millis,
                max_receives: request.max_receives,
            },
        );

        let arn = match Arn::new(
            self.partition.clone(),
            self.service.clone(),
            Some(request.region),
            Some(request.account),
            request.id.clone(),
        ) {
            Ok(arn) => arn,
            Err(e) => {
                self.quota.release("queues", 1);
                return Err(e);
            }
        };
        if let Err(e) = self.queue_names.register(&arn, request.id.clone()) {
            self.quota.release("queues", 1);
            return Err(e);
        }

        self.events.append(
            request.id.clone(),
            "queue.created",
            request.created_at,
            format!(
                "account={} max_receives={}",
                resource.account(),
                request.max_receives
            ),
        );

        self.queues.insert(request.id.clone(), resource);
        self.messages.insert(request.id.clone(), BTreeMap::new());
        self.dead_letters
            .insert(request.id.clone(), BTreeSet::new());
        Ok(self
            .queues
            .get(&request.id)
            .expect("just inserted under this exact id"))
    }

    /// Creates a new topic: validates the account and region exist,
    /// authorizes the request, reserves one unit of the account's
    /// `"topics"` quota, and starts it with no subscribers. Any
    /// failure after the quota was reserved releases it.
    pub fn create_topic(
        &mut self,
        request: CreateTopicRequest,
    ) -> Result<&Resource<TopicSpec>, CloudError> {
        if !self.accounts.contains(&request.account) {
            return Err(CloudError::NotFound {
                what: "account",
                id: request.account.to_string(),
            });
        }
        if self.regions.get(&request.region).is_none() {
            return Err(CloudError::NotFound {
                what: "region",
                id: request.region.to_string(),
            });
        }
        if self.topics.contains_key(&request.id) {
            return Err(CloudError::Conflict {
                what: "topic",
                id: request.id.to_string(),
            });
        }

        self.policy.authorize(
            &request.principal,
            "messaging:create-topic",
            request.id.as_str(),
        )?;

        self.quota.try_reserve("topics", 1)?;

        let resource_type = ResourceType::new("message-topic").expect("valid literal");
        let resource = Resource::new(
            request.id.clone(),
            resource_type,
            Some(request.region.clone()),
            request.account.clone(),
            request.created_at,
            TopicSpec {
                required_delivery: request.required_delivery,
            },
        );

        let arn = match Arn::new(
            self.partition.clone(),
            self.service.clone(),
            Some(request.region),
            Some(request.account),
            request.id.clone(),
        ) {
            Ok(arn) => arn,
            Err(e) => {
                self.quota.release("topics", 1);
                return Err(e);
            }
        };
        if let Err(e) = self.topic_names.register(&arn, request.id.clone()) {
            self.quota.release("topics", 1);
            return Err(e);
        }

        self.events.append(
            request.id.clone(),
            "topic.created",
            request.created_at,
            format!("account={}", resource.account()),
        );

        self.topics.insert(request.id.clone(), resource);
        Ok(self
            .topics
            .get(&request.id)
            .expect("just inserted under this exact id"))
    }

    /// Subscribes `queue_id` to `topic_id`. Refused unless the queue's
    /// own delivery semantics satisfy the topic's required semantics --
    /// e.g. an `AtMostOnce` queue cannot subscribe to a topic that
    /// requires `ExactlyOnce`.
    pub fn subscribe(
        &mut self,
        topic_id: &ResourceId,
        queue_id: &ResourceId,
        now: Timestamp,
    ) -> Result<(), CloudError> {
        let topic = self
            .topics
            .get(topic_id)
            .ok_or_else(|| CloudError::NotFound {
                what: "topic",
                id: topic_id.to_string(),
            })?;
        let queue = self
            .queues
            .get(queue_id)
            .ok_or_else(|| CloudError::NotFound {
                what: "queue",
                id: queue_id.to_string(),
            })?;
        if !queue
            .payload()
            .delivery
            .satisfies(topic.payload().required_delivery)
        {
            return Err(CloudError::Conflict {
                what: "subscription",
                id: format!("{queue_id} does not satisfy {topic_id}'s delivery requirement"),
            });
        }

        self.fanout.subscribe(topic_id.clone(), queue_id.clone())?;
        self.events.append(
            topic_id.clone(),
            "topic.subscribed",
            now,
            format!("queue={queue_id}"),
        );
        Ok(())
    }

    pub fn unsubscribe(
        &mut self,
        topic_id: &ResourceId,
        queue_id: &ResourceId,
        now: Timestamp,
    ) -> Result<(), CloudError> {
        self.fanout.unsubscribe(topic_id, queue_id)?;
        self.events.append(
            topic_id.clone(),
            "topic.unsubscribed",
            now,
            format!("queue={queue_id}"),
        );
        Ok(())
    }

    /// Publishes `message_id` to every one of `topic_id`'s subscriber
    /// queues, returning the list of queues it was delivered to. Checks
    /// every subscriber has room for `message_id` *before* delivering
    /// to any of them, so a duplicate-id collision partway through a
    /// fan-out never leaves some subscribers holding the message and
    /// others without it.
    pub fn publish(
        &mut self,
        topic_id: &ResourceId,
        message_id: ResourceId,
        now: Timestamp,
    ) -> Result<Vec<ResourceId>, CloudError> {
        if !self.topics.contains_key(topic_id) {
            return Err(CloudError::NotFound {
                what: "topic",
                id: topic_id.to_string(),
            });
        }

        let subscribers: Vec<ResourceId> = self.fanout.subscribers(topic_id).cloned().collect();
        for subscriber in &subscribers {
            let inbox = self
                .messages
                .get(subscriber)
                .ok_or_else(|| CloudError::NotFound {
                    what: "queue",
                    id: subscriber.to_string(),
                })?;
            if inbox.contains_key(&message_id) {
                return Err(CloudError::Conflict {
                    what: "message",
                    id: message_id.to_string(),
                });
            }
        }

        for subscriber in &subscribers {
            self.messages
                .get_mut(subscriber)
                .expect("checked above")
                .insert(message_id.clone(), MessageLease::new());
        }

        self.events.append(
            topic_id.clone(),
            "topic.published",
            now,
            format!("message={message_id} fanout={}", subscribers.len()),
        );
        Ok(subscribers)
    }

    /// Enqueues `message_id` directly onto `queue_id`, bypassing any
    /// topic. Rejects a duplicate `message_id` already present in the
    /// queue's inbox.
    pub fn enqueue(
        &mut self,
        queue_id: &ResourceId,
        message_id: ResourceId,
        now: Timestamp,
    ) -> Result<(), CloudError> {
        let inbox = self
            .messages
            .get_mut(queue_id)
            .ok_or_else(|| CloudError::NotFound {
                what: "queue",
                id: queue_id.to_string(),
            })?;
        if inbox.contains_key(&message_id) {
            return Err(CloudError::Conflict {
                what: "message",
                id: message_id.to_string(),
            });
        }
        inbox.insert(message_id.clone(), MessageLease::new());
        self.events.append(
            queue_id.clone(),
            "queue.message-enqueued",
            now,
            format!("message={message_id}"),
        );
        Ok(())
    }

    /// Receives `message_id` from `queue_id`: hides it until the
    /// queue's visibility timeout elapses and increments its receive
    /// count. Rejected (leaving the message's lease unchanged) if it's
    /// already in flight. If this receive pushes the message's receive
    /// count to the queue's `max_receives`, it is moved to the queue's
    /// dead-letter set instead of staying redeliverable.
    pub fn receive(
        &mut self,
        queue_id: &ResourceId,
        message_id: &ResourceId,
        now: Timestamp,
    ) -> Result<ReceiveOutcome, CloudError> {
        let spec = *self
            .queues
            .get(queue_id)
            .ok_or_else(|| CloudError::NotFound {
                what: "queue",
                id: queue_id.to_string(),
            })?
            .payload();
        let inbox = self
            .messages
            .get_mut(queue_id)
            .expect("queues and messages stores are always kept in sync");
        let lease = inbox
            .get_mut(message_id)
            .ok_or_else(|| CloudError::NotFound {
                what: "message",
                id: message_id.to_string(),
            })?;
        lease.receive(now, spec.visibility_timeout_millis)?;
        let receive_count = lease.receive_count();

        if lease.should_dead_letter(spec.max_receives) {
            inbox.remove(message_id);
            self.dead_letters
                .get_mut(queue_id)
                .expect("dead-letter and queue stores are always kept in sync")
                .insert(message_id.clone());
            self.events.append(
                queue_id.clone(),
                "queue.message-dead-lettered",
                now,
                format!("message={message_id} receive_count={receive_count}"),
            );
            return Ok(ReceiveOutcome::DeadLettered);
        }

        self.events.append(
            queue_id.clone(),
            "queue.message-received",
            now,
            format!("message={message_id} receive_count={receive_count}"),
        );
        Ok(ReceiveOutcome::Delivered { receive_count })
    }

    /// Acknowledges and removes `message_id` from `queue_id`'s inbox.
    pub fn delete_message(
        &mut self,
        queue_id: &ResourceId,
        message_id: &ResourceId,
        now: Timestamp,
    ) -> Result<(), CloudError> {
        let inbox = self
            .messages
            .get_mut(queue_id)
            .ok_or_else(|| CloudError::NotFound {
                what: "queue",
                id: queue_id.to_string(),
            })?;
        if inbox.remove(message_id).is_none() {
            return Err(CloudError::NotFound {
                what: "message",
                id: message_id.to_string(),
            });
        }
        self.events.append(
            queue_id.clone(),
            "queue.message-deleted",
            now,
            format!("message={message_id}"),
        );
        Ok(())
    }

    /// Deletes a queue: refuses unless its inbox is completely empty
    /// (mirroring `cloud-database`'s refusal to delete a database with
    /// remaining snapshots), reconciles its lifecycle to `Deleted`, and
    /// releases the `"queues"` quota it held.
    pub fn delete_queue(&mut self, id: &ResourceId, now: Timestamp) -> Result<(), CloudError> {
        let inbox = self.messages.get(id).ok_or_else(|| CloudError::NotFound {
            what: "queue",
            id: id.to_string(),
        })?;
        if !inbox.is_empty() {
            return Err(CloudError::Conflict {
                what: "queue",
                id: id.to_string(),
            });
        }

        let resource = self
            .queues
            .get_mut(id)
            .expect("queues and messages stores are always kept in sync");
        let plan = cloud_reconciler::reconcile(resource.lifecycle(), Lifecycle::Deleted)?;
        for step in plan {
            resource
                .transition_lifecycle(step, now)
                .expect("cloud-reconciler only ever returns already-valid single steps");
        }

        self.quota.release("queues", 1);
        self.messages.remove(id);
        self.dead_letters.remove(id);
        self.events
            .append(id.clone(), "queue.deleted", now, String::new());
        Ok(())
    }

    /// Deletes a topic: refuses while it still has subscribers
    /// (mirroring `delete_queue`'s own-state gate), reconciles its
    /// lifecycle to `Deleted`, and releases the `"topics"` quota it
    /// held.
    pub fn delete_topic(&mut self, id: &ResourceId, now: Timestamp) -> Result<(), CloudError> {
        if !self.topics.contains_key(id) {
            return Err(CloudError::NotFound {
                what: "topic",
                id: id.to_string(),
            });
        }
        if self.fanout.subscribers(id).next().is_some() {
            return Err(CloudError::Conflict {
                what: "topic",
                id: id.to_string(),
            });
        }

        let resource = self.topics.get_mut(id).expect("checked above");
        let plan = cloud_reconciler::reconcile(resource.lifecycle(), Lifecycle::Deleted)?;
        for step in plan {
            resource
                .transition_lifecycle(step, now)
                .expect("cloud-reconciler only ever returns already-valid single steps");
        }

        self.quota.release("topics", 1);
        self.events
            .append(id.clone(), "topic.deleted", now, String::new());
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use cloud_account::Account;
    use cloud_policy::{Effect, PrincipalMatcher, Statement};

    fn allow_all_policy() -> Policy {
        let mut policy = Policy::new();
        policy.add_statement(Statement {
            effect: Effect::Allow,
            principals: PrincipalMatcher::Any,
            actions: vec!["*".to_string()],
            resources: vec!["*".to_string()],
        });
        policy
    }

    fn account_id() -> AccountId {
        AccountId::new("000000000001").unwrap()
    }

    fn region_id() -> RegionId {
        RegionId::new("us-west-1").unwrap()
    }

    fn principal() -> Principal {
        Principal::User(ResourceId::new("alice").unwrap())
    }

    fn setup() -> MessagingService {
        let mut svc = MessagingService::new(allow_all_policy(), "core", "messaging");
        svc.register_account(
            Account::new(account_id(), "test", Timestamp::from_millis(0)).unwrap(),
        )
        .unwrap();
        svc.register_region(region_id(), 1).unwrap();
        svc.set_quota_limit("queues", 10);
        svc.set_quota_limit("topics", 10);
        svc
    }

    fn queue_request(id: &str, delivery: DeliverySemantics) -> CreateQueueRequest {
        CreateQueueRequest {
            id: ResourceId::new(id).unwrap(),
            account: account_id(),
            region: region_id(),
            principal: principal(),
            delivery,
            visibility_timeout_millis: 30_000,
            max_receives: 3,
            created_at: Timestamp::from_millis(1000),
        }
    }

    fn topic_request(id: &str, required_delivery: DeliverySemantics) -> CreateTopicRequest {
        CreateTopicRequest {
            id: ResourceId::new(id).unwrap(),
            account: account_id(),
            region: region_id(),
            principal: principal(),
            required_delivery,
            created_at: Timestamp::from_millis(1000),
        }
    }

    #[test]
    fn create_queue_rejects_an_unregistered_account() {
        let mut svc = setup();
        let mut req = queue_request("q-1", DeliverySemantics::ExactlyOnce);
        req.account = AccountId::new("999999999999").unwrap();
        let err = svc.create_queue(req).unwrap_err();
        assert!(matches!(err, CloudError::NotFound { .. }));
    }

    #[test]
    fn create_queue_rejects_an_unregistered_region() {
        let mut svc = setup();
        let mut req = queue_request("q-1", DeliverySemantics::ExactlyOnce);
        req.region = RegionId::new("eu-central-1").unwrap();
        let err = svc.create_queue(req).unwrap_err();
        assert!(matches!(err, CloudError::NotFound { .. }));
    }

    #[test]
    fn create_queue_rejects_a_duplicate_id() {
        let mut svc = setup();
        svc.create_queue(queue_request("q-1", DeliverySemantics::ExactlyOnce))
            .unwrap();
        let err = svc
            .create_queue(queue_request("q-1", DeliverySemantics::ExactlyOnce))
            .unwrap_err();
        assert!(matches!(err, CloudError::Conflict { .. }));
    }

    #[test]
    fn a_successful_create_queue_reserves_quota_and_starts_empty() {
        let mut svc = setup();
        svc.create_queue(queue_request("q-1", DeliverySemantics::ExactlyOnce))
            .unwrap();
        let id = ResourceId::new("q-1").unwrap();
        assert_eq!(svc.get_queue(&id).unwrap().lifecycle(), Lifecycle::Creating);
        assert_eq!(svc.quota.usage("queues"), 1);
        assert_eq!(svc.dead_letters(&id).unwrap().count(), 0);
    }

    #[test]
    fn create_queue_registers_a_resolvable_arn() {
        let mut svc = setup();
        svc.create_queue(queue_request("q-1", DeliverySemantics::ExactlyOnce))
            .unwrap();
        let id = ResourceId::new("q-1").unwrap();
        let arn = svc.queue_arn_of(&id).unwrap();
        assert_eq!(svc.resolve_queue_arn(&arn).unwrap().id(), &id);
    }

    #[test]
    fn exhausting_queue_quota_fails_create_and_reserves_nothing() {
        let mut svc = setup();
        svc.set_quota_limit("queues", 0);
        let err = svc
            .create_queue(queue_request("q-1", DeliverySemantics::ExactlyOnce))
            .unwrap_err();
        assert!(matches!(err, CloudError::QuotaExceeded { .. }));
        assert_eq!(svc.quota.usage("queues"), 0);
    }

    #[test]
    fn a_successful_create_topic_reserves_quota_and_has_no_subscribers() {
        let mut svc = setup();
        svc.create_topic(topic_request("t-1", DeliverySemantics::AtLeastOnce))
            .unwrap();
        assert_eq!(svc.quota.usage("topics"), 1);
    }

    #[test]
    fn create_topic_rejects_a_duplicate_id() {
        let mut svc = setup();
        svc.create_topic(topic_request("t-1", DeliverySemantics::AtLeastOnce))
            .unwrap();
        let err = svc
            .create_topic(topic_request("t-1", DeliverySemantics::AtLeastOnce))
            .unwrap_err();
        assert!(matches!(err, CloudError::Conflict { .. }));
    }

    #[test]
    fn subscribe_rejects_a_queue_that_does_not_satisfy_the_topics_requirement() {
        let mut svc = setup();
        svc.create_topic(topic_request("t-1", DeliverySemantics::ExactlyOnce))
            .unwrap();
        svc.create_queue(queue_request("q-1", DeliverySemantics::AtMostOnce))
            .unwrap();
        let err = svc
            .subscribe(
                &ResourceId::new("t-1").unwrap(),
                &ResourceId::new("q-1").unwrap(),
                Timestamp::from_millis(2000),
            )
            .unwrap_err();
        assert!(matches!(err, CloudError::Conflict { .. }));
    }

    #[test]
    fn subscribe_succeeds_when_the_queue_satisfies_the_topics_requirement() {
        let mut svc = setup();
        svc.create_topic(topic_request("t-1", DeliverySemantics::AtLeastOnce))
            .unwrap();
        svc.create_queue(queue_request("q-1", DeliverySemantics::ExactlyOnce))
            .unwrap();
        svc.subscribe(
            &ResourceId::new("t-1").unwrap(),
            &ResourceId::new("q-1").unwrap(),
            Timestamp::from_millis(2000),
        )
        .unwrap();
    }

    #[test]
    fn publish_fans_out_to_every_subscriber() {
        let mut svc = setup();
        svc.create_topic(topic_request("t-1", DeliverySemantics::AtLeastOnce))
            .unwrap();
        svc.create_queue(queue_request("q-1", DeliverySemantics::ExactlyOnce))
            .unwrap();
        svc.create_queue(queue_request("q-2", DeliverySemantics::AtLeastOnce))
            .unwrap();
        let topic = ResourceId::new("t-1").unwrap();
        svc.subscribe(
            &topic,
            &ResourceId::new("q-1").unwrap(),
            Timestamp::from_millis(2000),
        )
        .unwrap();
        svc.subscribe(
            &topic,
            &ResourceId::new("q-2").unwrap(),
            Timestamp::from_millis(2000),
        )
        .unwrap();

        let delivered = svc
            .publish(
                &topic,
                ResourceId::new("msg-1").unwrap(),
                Timestamp::from_millis(3000),
            )
            .unwrap();
        assert_eq!(delivered.len(), 2);

        let outcome = svc
            .receive(
                &ResourceId::new("q-1").unwrap(),
                &ResourceId::new("msg-1").unwrap(),
                Timestamp::from_millis(4000),
            )
            .unwrap();
        assert_eq!(outcome, ReceiveOutcome::Delivered { receive_count: 1 });
    }

    #[test]
    fn publish_with_a_colliding_message_id_leaves_no_partial_fanout() {
        let mut svc = setup();
        svc.create_topic(topic_request("t-1", DeliverySemantics::AtLeastOnce))
            .unwrap();
        svc.create_queue(queue_request("q-1", DeliverySemantics::ExactlyOnce))
            .unwrap();
        svc.create_queue(queue_request("q-2", DeliverySemantics::AtLeastOnce))
            .unwrap();
        let topic = ResourceId::new("t-1").unwrap();
        let q1 = ResourceId::new("q-1").unwrap();
        let q2 = ResourceId::new("q-2").unwrap();
        svc.subscribe(&topic, &q1, Timestamp::from_millis(2000))
            .unwrap();
        svc.subscribe(&topic, &q2, Timestamp::from_millis(2000))
            .unwrap();

        // q-1 already has msg-1 via a direct enqueue.
        svc.enqueue(
            &q1,
            ResourceId::new("msg-1").unwrap(),
            Timestamp::from_millis(2500),
        )
        .unwrap();

        let err = svc
            .publish(
                &topic,
                ResourceId::new("msg-1").unwrap(),
                Timestamp::from_millis(3000),
            )
            .unwrap_err();
        assert!(matches!(err, CloudError::Conflict { .. }));

        // q-2 must not have received the message either.
        let err = svc
            .receive(
                &q2,
                &ResourceId::new("msg-1").unwrap(),
                Timestamp::from_millis(4000),
            )
            .unwrap_err();
        assert!(matches!(err, CloudError::NotFound { .. }));
    }

    #[test]
    fn enqueue_then_receive_delivers_the_message() {
        let mut svc = setup();
        svc.create_queue(queue_request("q-1", DeliverySemantics::ExactlyOnce))
            .unwrap();
        let id = ResourceId::new("q-1").unwrap();
        svc.enqueue(
            &id,
            ResourceId::new("msg-1").unwrap(),
            Timestamp::from_millis(2000),
        )
        .unwrap();
        let outcome = svc
            .receive(
                &id,
                &ResourceId::new("msg-1").unwrap(),
                Timestamp::from_millis(3000),
            )
            .unwrap();
        assert_eq!(outcome, ReceiveOutcome::Delivered { receive_count: 1 });
    }

    #[test]
    fn enqueue_rejects_a_duplicate_message_id() {
        let mut svc = setup();
        svc.create_queue(queue_request("q-1", DeliverySemantics::ExactlyOnce))
            .unwrap();
        let id = ResourceId::new("q-1").unwrap();
        svc.enqueue(
            &id,
            ResourceId::new("msg-1").unwrap(),
            Timestamp::from_millis(2000),
        )
        .unwrap();
        let err = svc
            .enqueue(
                &id,
                ResourceId::new("msg-1").unwrap(),
                Timestamp::from_millis(2100),
            )
            .unwrap_err();
        assert!(matches!(err, CloudError::Conflict { .. }));
    }

    #[test]
    fn receiving_an_already_in_flight_message_is_rejected() {
        let mut svc = setup();
        svc.create_queue(queue_request("q-1", DeliverySemantics::ExactlyOnce))
            .unwrap();
        let id = ResourceId::new("q-1").unwrap();
        let msg = ResourceId::new("msg-1").unwrap();
        svc.enqueue(&id, msg.clone(), Timestamp::from_millis(2000))
            .unwrap();
        svc.receive(&id, &msg, Timestamp::from_millis(2100))
            .unwrap();
        let err = svc
            .receive(&id, &msg, Timestamp::from_millis(2200))
            .unwrap_err();
        assert!(matches!(err, CloudError::Conflict { .. }));
    }

    #[test]
    fn a_message_received_max_receives_times_is_dead_lettered() {
        let mut svc = setup();
        let mut req = queue_request("q-1", DeliverySemantics::AtLeastOnce);
        req.visibility_timeout_millis = 100;
        req.max_receives = 2;
        svc.create_queue(req).unwrap();
        let id = ResourceId::new("q-1").unwrap();
        let msg = ResourceId::new("msg-1").unwrap();
        svc.enqueue(&id, msg.clone(), Timestamp::from_millis(0))
            .unwrap();

        let first = svc.receive(&id, &msg, Timestamp::from_millis(0)).unwrap();
        assert_eq!(first, ReceiveOutcome::Delivered { receive_count: 1 });

        let second = svc.receive(&id, &msg, Timestamp::from_millis(100)).unwrap();
        assert_eq!(second, ReceiveOutcome::DeadLettered);

        assert!(svc.dead_letters(&id).unwrap().any(|m| m == &msg));
        let err = svc
            .receive(&id, &msg, Timestamp::from_millis(200))
            .unwrap_err();
        assert!(matches!(err, CloudError::NotFound { .. }));
    }

    #[test]
    fn delete_message_acknowledges_and_removes_it() {
        let mut svc = setup();
        svc.create_queue(queue_request("q-1", DeliverySemantics::ExactlyOnce))
            .unwrap();
        let id = ResourceId::new("q-1").unwrap();
        let msg = ResourceId::new("msg-1").unwrap();
        svc.enqueue(&id, msg.clone(), Timestamp::from_millis(2000))
            .unwrap();
        svc.delete_message(&id, &msg, Timestamp::from_millis(2100))
            .unwrap();
        let err = svc
            .receive(&id, &msg, Timestamp::from_millis(2200))
            .unwrap_err();
        assert!(matches!(err, CloudError::NotFound { .. }));
    }

    #[test]
    fn delete_message_of_an_unknown_message_is_not_found() {
        let mut svc = setup();
        svc.create_queue(queue_request("q-1", DeliverySemantics::ExactlyOnce))
            .unwrap();
        let id = ResourceId::new("q-1").unwrap();
        let err = svc
            .delete_message(
                &id,
                &ResourceId::new("ghost").unwrap(),
                Timestamp::from_millis(2000),
            )
            .unwrap_err();
        assert!(matches!(err, CloudError::NotFound { .. }));
    }

    #[test]
    fn delete_queue_refuses_while_messages_remain() {
        let mut svc = setup();
        svc.create_queue(queue_request("q-1", DeliverySemantics::ExactlyOnce))
            .unwrap();
        let id = ResourceId::new("q-1").unwrap();
        svc.enqueue(
            &id,
            ResourceId::new("msg-1").unwrap(),
            Timestamp::from_millis(2000),
        )
        .unwrap();
        let err = svc
            .delete_queue(&id, Timestamp::from_millis(3000))
            .unwrap_err();
        assert!(matches!(err, CloudError::Conflict { .. }));
        assert_eq!(
            svc.quota.usage("queues"),
            1,
            "a refused delete must not release quota"
        );
    }

    #[test]
    fn delete_queue_succeeds_once_empty_and_releases_quota() {
        let mut svc = setup();
        svc.create_queue(queue_request("q-1", DeliverySemantics::ExactlyOnce))
            .unwrap();
        let id = ResourceId::new("q-1").unwrap();
        svc.delete_queue(&id, Timestamp::from_millis(3000)).unwrap();
        assert_eq!(svc.get_queue(&id).unwrap().lifecycle(), Lifecycle::Deleted);
        assert_eq!(svc.quota.usage("queues"), 0);
    }

    #[test]
    fn delete_topic_refuses_while_subscribers_remain() {
        let mut svc = setup();
        svc.create_topic(topic_request("t-1", DeliverySemantics::AtLeastOnce))
            .unwrap();
        svc.create_queue(queue_request("q-1", DeliverySemantics::ExactlyOnce))
            .unwrap();
        let topic = ResourceId::new("t-1").unwrap();
        svc.subscribe(
            &topic,
            &ResourceId::new("q-1").unwrap(),
            Timestamp::from_millis(2000),
        )
        .unwrap();
        let err = svc
            .delete_topic(&topic, Timestamp::from_millis(3000))
            .unwrap_err();
        assert!(matches!(err, CloudError::Conflict { .. }));
    }

    #[test]
    fn delete_topic_succeeds_once_unsubscribed_and_releases_quota() {
        let mut svc = setup();
        svc.create_topic(topic_request("t-1", DeliverySemantics::AtLeastOnce))
            .unwrap();
        svc.create_queue(queue_request("q-1", DeliverySemantics::ExactlyOnce))
            .unwrap();
        let topic = ResourceId::new("t-1").unwrap();
        let queue = ResourceId::new("q-1").unwrap();
        svc.subscribe(&topic, &queue, Timestamp::from_millis(2000))
            .unwrap();
        svc.unsubscribe(&topic, &queue, Timestamp::from_millis(2500))
            .unwrap();
        svc.delete_topic(&topic, Timestamp::from_millis(3000))
            .unwrap();
        assert_eq!(
            svc.get_topic(&topic).unwrap().lifecycle(),
            Lifecycle::Deleted
        );
        assert_eq!(svc.quota.usage("topics"), 0);
    }

    #[test]
    fn delete_of_an_unknown_queue_or_topic_is_not_found() {
        let mut svc = setup();
        let err = svc
            .delete_queue(
                &ResourceId::new("q-ghost").unwrap(),
                Timestamp::from_millis(1000),
            )
            .unwrap_err();
        assert!(matches!(err, CloudError::NotFound { .. }));
        let err = svc
            .delete_topic(
                &ResourceId::new("t-ghost").unwrap(),
                Timestamp::from_millis(1000),
            )
            .unwrap_err();
        assert!(matches!(err, CloudError::NotFound { .. }));
    }

    #[test]
    fn list_includes_deleted_queues_and_topics() {
        let mut svc = setup();
        svc.create_queue(queue_request("q-1", DeliverySemantics::ExactlyOnce))
            .unwrap();
        svc.create_topic(topic_request("t-1", DeliverySemantics::AtLeastOnce))
            .unwrap();
        svc.delete_queue(
            &ResourceId::new("q-1").unwrap(),
            Timestamp::from_millis(2000),
        )
        .unwrap();
        svc.delete_topic(
            &ResourceId::new("t-1").unwrap(),
            Timestamp::from_millis(2000),
        )
        .unwrap();
        assert_eq!(svc.list_queues().count(), 1);
        assert_eq!(svc.list_topics().count(), 1);
    }
}
