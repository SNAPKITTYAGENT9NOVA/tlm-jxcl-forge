// Copyright (C) 2026 ERRANT DIGITAL INSTITUTE OF TECHNOLOGY (SEIT NGO) % SNAPKITTY COLLECTIVE
// SPDX-License-Identifier: AGPL-3.0-only OR LicenseRef-Commercial
// See LICENSE-AGPL and LICENSE-COMMERCIAL in this repository for license terms.

//! The attach/detach state machine for a volume-like resource: one
//! that gets connected to and disconnected from a compute-like
//! resource repeatedly over its lifetime, rather than being wired up
//! once at creation and left alone.
//!
//! This is a third state machine alongside [`cloud_lifecycle::Lifecycle`]
//! and `cloud_runtime::RuntimeState`, and deliberately not a rename of
//! either: `Lifecycle` tracks whether the volume's resource *record*
//! exists; `RuntimeState` (a different crate entirely) tracks whether a
//! *compute* resource is executing, which a volume never does;
//! `AttachmentState` tracks whether *this* volume is currently
//! connected to something, which varies independently of both -- a
//! volume can sit `Lifecycle::Active` for its whole existence while
//! cycling `Attached`/`Detached` many times as it's moved between
//! instances.
//!
//! Unlike the other two state machines, this one has **no terminal
//! state**: every non-`Detached` state has a path back to `Detached`,
//! because attachment cycles for as long as the volume itself exists
//! -- termination of the volume's record is `Lifecycle`'s job, not
//! this crate's.
#![forbid(unsafe_code)]

use cloud_errors::CloudError;

/// A volume-like resource's current attachment state.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum AttachmentState {
    Detached,
    Attaching,
    Attached,
    Detaching,
    Failed,
}

impl AttachmentState {
    fn name(self) -> &'static str {
        match self {
            AttachmentState::Detached => "Detached",
            AttachmentState::Attaching => "Attaching",
            AttachmentState::Attached => "Attached",
            AttachmentState::Detaching => "Detaching",
            AttachmentState::Failed => "Failed",
        }
    }

    /// Whether moving from `self` to `to` is allowed.
    ///
    /// - `Detached`  -> `Attaching`
    /// - `Attaching` -> `Attached`, `Failed`
    /// - `Attached`  -> `Detaching`
    /// - `Detaching` -> `Detached`, `Failed`
    /// - `Failed`    -> `Detached` (the only way out: retry from a clean state)
    pub fn can_transition_to(self, to: AttachmentState) -> bool {
        use AttachmentState::*;
        matches!(
            (self, to),
            (Detached, Attaching)
                | (Attaching, Attached)
                | (Attaching, Failed)
                | (Attached, Detaching)
                | (Detaching, Detached)
                | (Detaching, Failed)
                | (Failed, Detached)
        )
    }

    /// Attempts the transition, returning [`CloudError::InvalidTransition`]
    /// if it isn't allowed.
    pub fn transition_to(self, to: AttachmentState) -> Result<AttachmentState, CloudError> {
        if self.can_transition_to(to) {
            Ok(to)
        } else {
            Err(CloudError::InvalidTransition {
                what: "attachment",
                from: self.name().to_string(),
                to: to.name().to_string(),
            })
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use AttachmentState::*;

    const ALL: [AttachmentState; 5] = [Detached, Attaching, Attached, Detaching, Failed];

    #[test]
    fn attaching_then_attached_is_the_happy_path() {
        let mut state = Detached;
        for next in [Attaching, Attached, Detaching, Detached] {
            state = state.transition_to(next).unwrap();
        }
        assert_eq!(state, Detached);
    }

    #[test]
    fn detached_cannot_skip_straight_to_attached() {
        assert!(!Detached.can_transition_to(Attached));
    }

    #[test]
    fn attached_cannot_go_directly_back_to_detached_without_detaching() {
        assert!(!Attached.can_transition_to(Detached));
    }

    #[test]
    fn an_attach_attempt_can_fail() {
        assert!(Attaching.can_transition_to(Failed));
    }

    #[test]
    fn a_detach_attempt_can_fail() {
        assert!(Detaching.can_transition_to(Failed));
    }

    #[test]
    fn a_failed_attachment_can_only_be_retried_from_a_clean_detached_state() {
        assert!(Failed.can_transition_to(Detached));
        assert!(!Failed.can_transition_to(Attached));
        assert!(!Failed.can_transition_to(Attaching));
    }

    #[test]
    fn every_state_has_a_path_back_to_detached() {
        // Unlike Lifecycle or RuntimeState, this machine has no
        // terminal sink: an operator must always be able to retry to
        // a clean state, whatever went wrong.
        fn reaches_detached(start: AttachmentState) -> bool {
            let mut seen = vec![start];
            let mut frontier = vec![start];
            while let Some(state) = frontier.pop() {
                if state == Detached {
                    return true;
                }
                for &next in &ALL {
                    if state.can_transition_to(next) && !seen.contains(&next) {
                        seen.push(next);
                        frontier.push(next);
                    }
                }
            }
            false
        }
        for state in ALL {
            assert!(reaches_detached(state), "{state:?} cannot reach Detached");
        }
    }

    #[test]
    fn no_state_can_transition_to_itself() {
        for state in ALL {
            assert!(
                !state.can_transition_to(state),
                "{state:?} -> itself should not be a no-op transition"
            );
        }
    }

    #[test]
    fn transition_to_returns_a_descriptive_error_on_rejection() {
        let err = Detached.transition_to(Attached).unwrap_err();
        let message = err.to_string();
        assert!(message.contains("Detached"));
        assert!(message.contains("Attached"));
    }
}
