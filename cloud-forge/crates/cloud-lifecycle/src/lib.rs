//! The resource lifecycle state machine every `cloud-forge` resource
//! goes through, and the transition table that governs it.
//!
//! This directly implements the roadmap's invariant
//! **CLOUD-I003: deleted resources cannot silently reappear** --
//! [`Lifecycle::Deleted`] has no outgoing transitions at all, checked
//! exhaustively below rather than left to convention.
#![forbid(unsafe_code)]

use cloud_errors::CloudError;

/// A resource's lifecycle state.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Lifecycle {
    Creating,
    Active,
    Updating,
    Deleting,
    Deleted,
    Failed,
}

impl Lifecycle {
    fn name(self) -> &'static str {
        match self {
            Lifecycle::Creating => "Creating",
            Lifecycle::Active => "Active",
            Lifecycle::Updating => "Updating",
            Lifecycle::Deleting => "Deleting",
            Lifecycle::Deleted => "Deleted",
            Lifecycle::Failed => "Failed",
        }
    }

    /// Whether moving from `self` to `to` is allowed.
    ///
    /// The table, spelled out rather than computed, so it is the
    /// single place anyone auditing this crate needs to read:
    ///
    /// - `Creating`  -> `Active`, `Failed`
    /// - `Active`    -> `Updating`, `Deleting`, `Failed`
    /// - `Updating`  -> `Active`, `Failed`
    /// - `Deleting`  -> `Deleted`, `Failed`
    /// - `Failed`    -> `Deleting` (so a failed resource can still be torn down)
    /// - `Deleted`   -> nothing; terminal.
    pub fn can_transition_to(self, to: Lifecycle) -> bool {
        use Lifecycle::*;
        matches!(
            (self, to),
            (Creating, Active)
                | (Creating, Failed)
                | (Active, Updating)
                | (Active, Deleting)
                | (Active, Failed)
                | (Updating, Active)
                | (Updating, Failed)
                | (Deleting, Deleted)
                | (Deleting, Failed)
                | (Failed, Deleting)
        )
    }

    /// Attempts the transition, returning [`CloudError::InvalidTransition`]
    /// if it isn't allowed.
    pub fn transition_to(self, to: Lifecycle) -> Result<Lifecycle, CloudError> {
        if self.can_transition_to(to) {
            Ok(to)
        } else {
            Err(CloudError::InvalidTransition {
                what: "lifecycle",
                from: self.name().to_string(),
                to: to.name().to_string(),
            })
        }
    }

    /// A terminal state has no outgoing transitions at all.
    pub fn is_terminal(self) -> bool {
        self == Lifecycle::Deleted
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use Lifecycle::*;

    const ALL: [Lifecycle; 6] = [Creating, Active, Updating, Deleting, Deleted, Failed];

    #[test]
    fn deleted_is_the_only_terminal_state() {
        for state in ALL {
            assert_eq!(state.is_terminal(), state == Deleted);
        }
    }

    #[test]
    fn deleted_has_no_outgoing_transitions() {
        for target in ALL {
            assert!(
                !Deleted.can_transition_to(target),
                "Deleted -> {target:?} should be rejected"
            );
        }
    }

    #[test]
    fn the_happy_path_creating_through_deleted_is_allowed() {
        let mut state = Creating;
        for next in [Active, Updating, Active, Deleting, Deleted] {
            state = state.transition_to(next).unwrap();
        }
        assert_eq!(state, Deleted);
    }

    #[test]
    fn a_failed_resource_can_still_be_torn_down() {
        assert!(Creating.can_transition_to(Failed));
        assert!(Failed.can_transition_to(Deleting));
        assert!(Deleting.can_transition_to(Deleted));
    }

    #[test]
    fn creating_cannot_skip_straight_to_deleting_or_deleted() {
        assert!(!Creating.can_transition_to(Deleting));
        assert!(!Creating.can_transition_to(Deleted));
    }

    #[test]
    fn active_cannot_go_back_to_creating() {
        assert!(!Active.can_transition_to(Creating));
    }

    #[test]
    fn transition_to_returns_a_descriptive_error_on_rejection() {
        let err = Deleted.transition_to(Active).unwrap_err();
        let message = err.to_string();
        assert!(message.contains("Deleted"));
        assert!(message.contains("Active"));
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
