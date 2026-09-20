// Copyright (C) 2026 ERRANT DIGITAL INSTITUTE OF TECHNOLOGY (SEIT NGO) % SNAPKITTY COLLECTIVE
// SPDX-License-Identifier: AGPL-3.0-only OR LicenseRef-Commercial
// See LICENSE-AGPL and LICENSE-COMMERCIAL in this repository for license terms.

//! The execution-state machine for something actually running: a
//! compute instance, a container, a function invocation.
//!
//! This is deliberately separate from [`cloud_lifecycle::Lifecycle`],
//! which governs whether a resource *record* exists at all
//! (`Creating`/`Active`/.../`Deleted`). [`RuntimeState`] governs
//! whether the thing that record represents is currently executing,
//! and the two vary independently: a resource can sit `Active` in
//! `cloud-lifecycle` terms for its whole existence while cycling
//! between `Running` and `Stopped` in `cloud-runtime` terms many times
//! over -- exactly the distinction `CLOUD_ARCHITECTURE.md`'s Phase 1
//! section cited when it deferred this crate: "a runtime only becomes
//! a real, distinct primitive once something is actually executing."
#![forbid(unsafe_code)]

use cloud_errors::CloudError;

/// A running thing's execution state.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum RuntimeState {
    Pending,
    Running,
    Stopping,
    Stopped,
    Terminating,
    Terminated,
}

impl RuntimeState {
    fn name(self) -> &'static str {
        match self {
            RuntimeState::Pending => "Pending",
            RuntimeState::Running => "Running",
            RuntimeState::Stopping => "Stopping",
            RuntimeState::Stopped => "Stopped",
            RuntimeState::Terminating => "Terminating",
            RuntimeState::Terminated => "Terminated",
        }
    }

    /// Whether moving from `self` to `to` is allowed.
    ///
    /// The table, spelled out rather than computed:
    ///
    /// - `Pending`     -> `Running`, `Terminating`
    /// - `Running`     -> `Stopping`, `Terminating`
    /// - `Stopping`    -> `Stopped`, `Terminating`
    /// - `Stopped`     -> `Pending` (restart), `Terminating`
    /// - `Terminating` -> `Terminated`
    /// - `Terminated`  -> nothing; terminal.
    pub fn can_transition_to(self, to: RuntimeState) -> bool {
        use RuntimeState::*;
        matches!(
            (self, to),
            (Pending, Running)
                | (Pending, Terminating)
                | (Running, Stopping)
                | (Running, Terminating)
                | (Stopping, Stopped)
                | (Stopping, Terminating)
                | (Stopped, Pending)
                | (Stopped, Terminating)
                | (Terminating, Terminated)
        )
    }

    /// Attempts the transition, returning [`CloudError::InvalidTransition`]
    /// if it isn't allowed.
    pub fn transition_to(self, to: RuntimeState) -> Result<RuntimeState, CloudError> {
        if self.can_transition_to(to) {
            Ok(to)
        } else {
            Err(CloudError::InvalidTransition {
                what: "runtime",
                from: self.name().to_string(),
                to: to.name().to_string(),
            })
        }
    }

    /// A terminal state has no outgoing transitions at all.
    pub fn is_terminal(self) -> bool {
        self == RuntimeState::Terminated
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use RuntimeState::*;

    const ALL: [RuntimeState; 6] = [Pending, Running, Stopping, Stopped, Terminating, Terminated];

    #[test]
    fn terminated_is_the_only_terminal_state() {
        for state in ALL {
            assert_eq!(state.is_terminal(), state == Terminated);
        }
    }

    #[test]
    fn terminated_has_no_outgoing_transitions() {
        for target in ALL {
            assert!(
                !Terminated.can_transition_to(target),
                "Terminated -> {target:?} should be rejected"
            );
        }
    }

    #[test]
    fn the_happy_path_pending_through_terminated_is_allowed() {
        let mut state = Pending;
        for next in [
            Running,
            Stopping,
            Stopped,
            Pending,
            Running,
            Terminating,
            Terminated,
        ] {
            state = state.transition_to(next).unwrap();
        }
        assert_eq!(state, Terminated);
    }

    #[test]
    fn a_running_instance_can_be_stopped_and_restarted() {
        assert!(Running.can_transition_to(Stopping));
        assert!(Stopping.can_transition_to(Stopped));
        assert!(Stopped.can_transition_to(Pending));
        assert!(Pending.can_transition_to(Running));
    }

    #[test]
    fn every_non_terminal_state_can_move_toward_termination() {
        for state in [Pending, Running, Stopping, Stopped] {
            assert!(
                state.can_transition_to(Terminating),
                "{state:?} -> Terminating should be allowed"
            );
        }
        assert!(Terminating.can_transition_to(Terminated));
    }

    #[test]
    fn pending_cannot_skip_straight_to_stopping_or_stopped() {
        assert!(!Pending.can_transition_to(Stopping));
        assert!(!Pending.can_transition_to(Stopped));
    }

    #[test]
    fn running_cannot_go_back_to_pending_without_stopping_first() {
        assert!(!Running.can_transition_to(Pending));
    }

    #[test]
    fn transition_to_returns_a_descriptive_error_on_rejection() {
        let err = Terminated.transition_to(Running).unwrap_err();
        let message = err.to_string();
        assert!(message.contains("Terminated"));
        assert!(message.contains("Running"));
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
}
