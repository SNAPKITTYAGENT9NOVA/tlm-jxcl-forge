// Copyright (C) 2026 ERRANT DIGITAL INSTITUTE OF TECHNOLOGY (SEIT NGO) % SNAPKITTY COLLECTIVE
// SPDX-License-Identifier: AGPL-3.0-only OR LicenseRef-Commercial
// See LICENSE-AGPL and LICENSE-COMMERCIAL in this repository for license terms.

//! The delivery guarantee a messaging-shaped resource offers.
//!
//! Unlike [`cloud_consistency::ConsistencyLevel`] (Phase 6), which is
//! a genuine total order, delivery semantics vary along **two
//! independent axes** -- whether loss is possible and whether
//! duplicates are possible -- so this crate deliberately does not
//! derive `Ord`. `AtMostOnce` (may lose, never duplicates) and
//! `AtLeastOnce` (never loses, may duplicate) are **incomparable**:
//! neither satisfies the other, because each permits something the
//! other forbids. Only `ExactlyOnce`, which permits neither, satisfies
//! both. [`DeliverySemantics::satisfies`] implements this partial
//! order directly rather than faking a total one that would give a
//! wrong answer for the incomparable pair.
#![forbid(unsafe_code)]

/// A messaging delivery guarantee.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum DeliverySemantics {
    /// Never delivers a duplicate; may silently lose a message.
    AtMostOnce,
    /// Never loses a message; may deliver a duplicate.
    AtLeastOnce,
    /// Never loses a message and never delivers a duplicate.
    ExactlyOnce,
}

impl DeliverySemantics {
    fn allows_loss(self) -> bool {
        matches!(self, DeliverySemantics::AtMostOnce)
    }

    fn allows_duplicates(self) -> bool {
        matches!(self, DeliverySemantics::AtLeastOnce)
    }

    /// Whether `self` satisfies a caller requiring `required`: `self`
    /// must not permit anything `required` forbids, on either axis.
    pub fn satisfies(self, required: DeliverySemantics) -> bool {
        (!self.allows_loss() || required.allows_loss())
            && (!self.allows_duplicates() || required.allows_duplicates())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use DeliverySemantics::*;

    #[test]
    fn exactly_once_satisfies_every_requirement() {
        assert!(ExactlyOnce.satisfies(AtMostOnce));
        assert!(ExactlyOnce.satisfies(AtLeastOnce));
        assert!(ExactlyOnce.satisfies(ExactlyOnce));
    }

    #[test]
    fn at_most_once_and_at_least_once_are_incomparable() {
        assert!(!AtMostOnce.satisfies(AtLeastOnce));
        assert!(!AtLeastOnce.satisfies(AtMostOnce));
    }

    #[test]
    fn nothing_weaker_than_exactly_once_satisfies_it() {
        assert!(!AtMostOnce.satisfies(ExactlyOnce));
        assert!(!AtLeastOnce.satisfies(ExactlyOnce));
    }

    #[test]
    fn satisfies_is_reflexive_for_every_level() {
        for level in [AtMostOnce, AtLeastOnce, ExactlyOnce] {
            assert!(level.satisfies(level));
        }
    }

    #[test]
    fn at_most_once_satisfies_only_itself_and_nothing_stronger() {
        assert!(AtMostOnce.satisfies(AtMostOnce));
        assert!(!AtMostOnce.satisfies(AtLeastOnce));
        assert!(!AtMostOnce.satisfies(ExactlyOnce));
    }

    #[test]
    fn at_least_once_satisfies_only_itself_and_nothing_stronger() {
        assert!(AtLeastOnce.satisfies(AtLeastOnce));
        assert!(!AtLeastOnce.satisfies(AtMostOnce));
        assert!(!AtLeastOnce.satisfies(ExactlyOnce));
    }
}
