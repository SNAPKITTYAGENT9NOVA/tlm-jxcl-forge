// Copyright (C) 2026 ERRANT DIGITAL INSTITUTE OF TECHNOLOGY (SEIT NGO) % SNAPKITTY COLLECTIVE
// SPDX-License-Identifier: AGPL-3.0-only OR LicenseRef-Commercial
// See LICENSE-AGPL and LICENSE-COMMERCIAL in this repository for license terms.

//! The read/write consistency guarantee a database-shaped resource
//! offers.
//!
//! The three levels form a total order from weakest to strongest --
//! `Eventual < BoundedStaleness < Strong` -- via `#[derive(Ord)]`'s
//! own rule that variants order by declaration position, rather than
//! a hand-rolled comparison that could quietly drift out of sync with
//! the enum's definition. [`ConsistencyLevel::satisfies`] is the one
//! operation every future database-shaped resource needs: "is what
//! I'm offering at least as strong as what the caller asked for?"
#![forbid(unsafe_code)]

use cloud_errors::CloudError;
use std::fmt;
use std::str::FromStr;

/// A consistency guarantee, from weakest to strongest.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum ConsistencyLevel {
    Eventual,
    BoundedStaleness,
    Strong,
}

impl ConsistencyLevel {
    /// Whether `self` is at least as strong as `required` -- the
    /// check a caller asking for a minimum guarantee actually needs.
    pub fn satisfies(self, required: ConsistencyLevel) -> bool {
        self >= required
    }
}

impl fmt::Display for ConsistencyLevel {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let s = match self {
            ConsistencyLevel::Eventual => "eventual",
            ConsistencyLevel::BoundedStaleness => "bounded-staleness",
            ConsistencyLevel::Strong => "strong",
        };
        write!(f, "{s}")
    }
}

impl FromStr for ConsistencyLevel {
    type Err = CloudError;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "eventual" => Ok(ConsistencyLevel::Eventual),
            "bounded-staleness" => Ok(ConsistencyLevel::BoundedStaleness),
            "strong" => Ok(ConsistencyLevel::Strong),
            _ => Err(CloudError::InvalidFormat {
                what: "consistency level",
                value: s.to_string(),
                reason: "must be one of 'eventual', 'bounded-staleness', 'strong'".to_string(),
            }),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ConsistencyLevel::*;

    #[test]
    fn ordering_runs_weakest_to_strongest() {
        assert!(Eventual < BoundedStaleness);
        assert!(BoundedStaleness < Strong);
        assert!(Eventual < Strong);
    }

    #[test]
    fn strong_satisfies_every_requirement() {
        assert!(Strong.satisfies(Eventual));
        assert!(Strong.satisfies(BoundedStaleness));
        assert!(Strong.satisfies(Strong));
    }

    #[test]
    fn eventual_only_satisfies_an_eventual_requirement() {
        assert!(Eventual.satisfies(Eventual));
        assert!(!Eventual.satisfies(BoundedStaleness));
        assert!(!Eventual.satisfies(Strong));
    }

    #[test]
    fn bounded_staleness_satisfies_eventual_and_itself_but_not_strong() {
        assert!(BoundedStaleness.satisfies(Eventual));
        assert!(BoundedStaleness.satisfies(BoundedStaleness));
        assert!(!BoundedStaleness.satisfies(Strong));
    }

    #[test]
    fn satisfies_is_reflexive_for_every_level() {
        for level in [Eventual, BoundedStaleness, Strong] {
            assert!(level.satisfies(level));
        }
    }

    #[test]
    fn display_and_fromstr_round_trip() {
        for level in [Eventual, BoundedStaleness, Strong] {
            let parsed: ConsistencyLevel = level.to_string().parse().unwrap();
            assert_eq!(level, parsed);
        }
    }

    #[test]
    fn fromstr_rejects_an_unknown_level() {
        assert!("causal".parse::<ConsistencyLevel>().is_err());
    }
}
