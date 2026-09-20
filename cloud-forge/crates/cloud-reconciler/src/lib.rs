// Copyright (C) 2026 ERRANT DIGITAL INSTITUTE OF TECHNOLOGY (SEIT NGO) % SNAPKITTY COLLECTIVE
// SPDX-License-Identifier: AGPL-3.0-only OR LicenseRef-Commercial
// See LICENSE-AGPL and LICENSE-COMMERCIAL in this repository for license terms.

//! Reconciliation: given where a resource's lifecycle currently is and
//! where it should end up, compute the shortest sequence of individual
//! `cloud_lifecycle::Lifecycle` transitions that gets there.
//!
//! `cloud-lifecycle` only answers "is this one step allowed?" -- it
//! has no notion of a multi-step plan. This crate treats the six
//! `Lifecycle` states as a graph (an edge exists wherever
//! `Lifecycle::can_transition_to` says so) and runs a breadth-first
//! search, so the plan it returns is always the *shortest* valid
//! sequence, not just *some* valid one.
#![forbid(unsafe_code)]

use cloud_errors::CloudError;
use cloud_lifecycle::Lifecycle;
use std::collections::{HashMap, VecDeque};

const ALL_STATES: [Lifecycle; 6] = [
    Lifecycle::Creating,
    Lifecycle::Active,
    Lifecycle::Updating,
    Lifecycle::Deleting,
    Lifecycle::Deleted,
    Lifecycle::Failed,
];

/// Computes the shortest sequence of `Lifecycle` states taking
/// `current` to `desired`, one valid transition apart, with `desired`
/// as the final element. Returns an empty plan if `current == desired`
/// already. Returns `CloudError::InvalidTransition` if `desired` is
/// not reachable from `current` by any sequence of valid transitions
/// (e.g. `desired` is `Creating`, which nothing transitions into, or
/// `current` is `Deleted`, which transitions into nothing).
pub fn reconcile(current: Lifecycle, desired: Lifecycle) -> Result<Vec<Lifecycle>, CloudError> {
    if current == desired {
        return Ok(Vec::new());
    }

    let mut predecessor: HashMap<Lifecycle, Lifecycle> = HashMap::new();
    let mut queue = VecDeque::new();
    queue.push_back(current);
    predecessor.insert(current, current);

    while let Some(node) = queue.pop_front() {
        if node == desired {
            let mut path = Vec::new();
            let mut cur = desired;
            while cur != current {
                path.push(cur);
                cur = predecessor[&cur];
            }
            path.reverse();
            return Ok(path);
        }
        for &next in &ALL_STATES {
            if node.can_transition_to(next) {
                predecessor.entry(next).or_insert_with(|| {
                    queue.push_back(next);
                    node
                });
            }
        }
    }

    Err(CloudError::InvalidTransition {
        what: "lifecycle reconciliation",
        from: format!("{current:?}"),
        to: format!("{desired:?}"),
    })
}

/// Whether `desired` is reachable from `current` at all (including
/// trivially, when they're equal).
pub fn is_reachable(current: Lifecycle, desired: Lifecycle) -> bool {
    current == desired || reconcile(current, desired).is_ok()
}

#[cfg(test)]
mod tests {
    use super::*;
    use Lifecycle::*;

    #[test]
    fn reconciling_to_the_current_state_is_a_no_op() {
        assert_eq!(reconcile(Active, Active).unwrap(), Vec::new());
    }

    #[test]
    fn a_single_valid_step_returns_a_one_element_plan() {
        assert_eq!(reconcile(Creating, Active).unwrap(), vec![Active]);
    }

    #[test]
    fn a_multi_step_plan_is_the_shortest_one() {
        // Creating -> Deleted requires at least 3 steps either via
        // Active or via Failed; BFS must return a 3-element plan, not
        // a longer, still-valid one.
        let plan = reconcile(Creating, Deleted).unwrap();
        assert_eq!(plan.len(), 3);
        assert_eq!(plan.last(), Some(&Deleted));
        // Every consecutive pair in the plan (including from `current`)
        // must itself be a valid single transition.
        let mut prev = Creating;
        for &step in &plan {
            assert!(
                prev.can_transition_to(step),
                "{prev:?} -> {step:?} should be a valid transition"
            );
            prev = step;
        }
    }

    #[test]
    fn nothing_transitions_back_into_creating() {
        let err = reconcile(Active, Creating).unwrap_err();
        assert!(matches!(err, CloudError::InvalidTransition { .. }));
        assert!(!is_reachable(Active, Creating));
    }

    #[test]
    fn deleted_cannot_reconcile_to_anything_else() {
        for target in [Creating, Active, Updating, Deleting, Failed] {
            assert!(
                reconcile(Deleted, target).is_err(),
                "Deleted -> {target:?} should be unreachable"
            );
        }
    }

    #[test]
    fn failed_can_still_reach_deleted_via_deleting() {
        let plan = reconcile(Failed, Deleted).unwrap();
        assert_eq!(plan, vec![Deleting, Deleted]);
    }

    #[test]
    fn is_reachable_is_true_for_the_current_state_itself() {
        assert!(is_reachable(Active, Active));
    }
}
