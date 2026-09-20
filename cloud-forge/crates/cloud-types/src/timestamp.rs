use std::fmt;

/// A point in time, as milliseconds since the Unix epoch.
///
/// This is a pure value type: it carries no clock of its own. Reading
/// "now" is a systemic concern (a future `cloud-core` clock
/// abstraction, so it can be replaced with a deterministic fake in
/// tests) rather than something this type should silently do on
/// construction.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Timestamp(u64);

impl Timestamp {
    pub const EPOCH: Timestamp = Timestamp(0);

    pub const fn from_millis(millis: u64) -> Self {
        Timestamp(millis)
    }

    pub const fn as_millis(self) -> u64 {
        self.0
    }

    /// Saturating addition of a millisecond duration -- never panics
    /// or wraps on overflow, since a corrupted/huge input should
    /// clamp, not crash a caller computing an expiry.
    pub const fn saturating_add_millis(self, millis: u64) -> Self {
        Timestamp(self.0.saturating_add(millis))
    }
}

impl fmt::Display for Timestamp {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}ms", self.0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn epoch_is_zero() {
        assert_eq!(Timestamp::EPOCH.as_millis(), 0);
    }

    #[test]
    fn from_millis_and_as_millis_round_trip() {
        let t = Timestamp::from_millis(1_700_000_000_000);
        assert_eq!(t.as_millis(), 1_700_000_000_000);
    }

    #[test]
    fn ordering_matches_millis_ordering() {
        let a = Timestamp::from_millis(100);
        let b = Timestamp::from_millis(200);
        assert!(a < b);
    }

    #[test]
    fn saturating_add_millis_does_not_panic_on_overflow() {
        let t = Timestamp::from_millis(u64::MAX - 5);
        let result = t.saturating_add_millis(100);
        assert_eq!(result.as_millis(), u64::MAX);
    }
}
