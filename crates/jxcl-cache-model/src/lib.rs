//! A direct-mapped cache simulation layered over memory, for profiling
//! hit/miss behavior.
//!
//! This is a *simulation*, not a functional cache -- it tracks which
//! line each address maps to and whether that line currently holds a
//! matching tag, purely to produce hit/miss statistics for
//! `jxcl-profiler` (a later batch) to aggregate. It never changes what a
//! load/store returns; the underlying `jxcl-memory::Memory`/
//! `jxcl-address-space::AddressSpace` remain the actual source of truth
//! for data.
//!
//! ## Deviation from `docs/crates.toml`
//!
//! The registry lists this crate's dependencies as `jxcl-memory` and
//! `jxcl-address-space`, but not `jxcl-types` -- yet both of those
//! crates' real, already-implemented public APIs address memory with
//! `jxcl-types::Address` (the workspace-wide address newtype), not a
//! bare `u64`. Naming that type in this crate's own public API (so
//! callers can pass the same `Address` they used to read/write memory)
//! requires `jxcl-types` as a direct dependency; it is added to
//! `Cargo.toml` for that reason alone.
#![forbid(unsafe_code)]

use jxcl_types::Address;

/// The outcome of one cache access.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CacheEvent {
    Hit,
    Miss,
}

/// Aggregate hit/miss counters.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct CacheStats {
    pub hits: u64,
    pub misses: u64,
}

impl CacheStats {
    pub fn total(&self) -> u64 {
        self.hits + self.misses
    }

    /// Hit rate in `[0.0, 1.0]`; `0.0` (not `NaN`) when there have been
    /// no accesses yet.
    pub fn hit_rate(&self) -> f64 {
        if self.total() == 0 {
            0.0
        } else {
            self.hits as f64 / self.total() as f64
        }
    }
}

/// A direct-mapped cache: `num_lines` lines of `line_size` bytes each.
/// Both must be powers of two (so index/tag extraction is a shift+mask,
/// matching a real direct-mapped cache's address decomposition:
/// `[ tag | index | offset ]`).
#[derive(Debug, Clone)]
pub struct CacheModel {
    line_bits: u32,
    index_bits: u32,
    /// One slot per line: `None` = invalid (never filled), `Some(tag)` =
    /// currently holds the block whose tag is `tag`.
    lines: Vec<Option<u64>>,
    stats: CacheStats,
}

/// Why [`CacheModel::new`] rejected its parameters.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CacheModelError {
    LineSizeNotPowerOfTwo,
    NumLinesNotPowerOfTwo,
}

impl CacheModel {
    /// Build a direct-mapped cache. `line_size` and `num_lines` must
    /// both be powers of two and nonzero.
    pub fn new(line_size: u64, num_lines: u64) -> Result<Self, CacheModelError> {
        if line_size == 0 || !line_size.is_power_of_two() {
            return Err(CacheModelError::LineSizeNotPowerOfTwo);
        }
        if num_lines == 0 || !num_lines.is_power_of_two() {
            return Err(CacheModelError::NumLinesNotPowerOfTwo);
        }
        Ok(CacheModel {
            line_bits: line_size.trailing_zeros(),
            index_bits: num_lines.trailing_zeros(),
            lines: vec![None; num_lines as usize],
            stats: CacheStats::default(),
        })
    }

    fn decompose(&self, addr: Address) -> (usize, u64) {
        let a = addr.get();
        let index = (a >> self.line_bits) & (self.lines.len() as u64 - 1);
        let tag = a >> (self.line_bits + self.index_bits);
        (index as usize, tag)
    }

    /// Record one access to the byte at `addr`: hit if the owning line
    /// already holds a matching tag, otherwise a miss that fills the line
    /// with this access's tag (evicting whatever was there).
    pub fn access(&mut self, addr: Address) -> CacheEvent {
        let (index, tag) = self.decompose(addr);
        if self.lines[index] == Some(tag) {
            self.stats.hits += 1;
            CacheEvent::Hit
        } else {
            self.lines[index] = Some(tag);
            self.stats.misses += 1;
            CacheEvent::Miss
        }
    }

    /// Record an access spanning `[addr, addr+len)`; every cache line the
    /// range touches is accessed once, in address order. Returns
    /// `CacheEvent::Hit` only if *every* touched line was already a hit
    /// (a real memory access is only fast if nothing in its range misses).
    pub fn access_range(&mut self, addr: Address, len: u64) -> CacheEvent {
        if len == 0 {
            return CacheEvent::Hit;
        }
        let line_size = 1u64 << self.line_bits;
        let mut all_hit = true;
        let mut a = addr.get() - (addr.get() % line_size);
        let end = addr.get() + len;
        while a < end {
            if self.access(Address::new(a)) == CacheEvent::Miss {
                all_hit = false;
            }
            a += line_size;
        }
        if all_hit {
            CacheEvent::Hit
        } else {
            CacheEvent::Miss
        }
    }

    pub fn stats(&self) -> CacheStats {
        self.stats
    }

    /// Invalidate every line (e.g. on a context switch), without
    /// resetting the accumulated hit/miss statistics.
    pub fn flush(&mut self) {
        self.lines.iter_mut().for_each(|l| *l = None);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rejects_non_power_of_two_parameters() {
        assert_eq!(
            CacheModel::new(3, 4).unwrap_err(),
            CacheModelError::LineSizeNotPowerOfTwo
        );
        assert_eq!(
            CacheModel::new(4, 3).unwrap_err(),
            CacheModelError::NumLinesNotPowerOfTwo
        );
    }

    #[test]
    fn known_pattern_first_access_misses_repeat_hits() {
        // 4 lines of 8 bytes each = 32-byte cache footprint.
        let mut c = CacheModel::new(8, 4).unwrap();
        assert_eq!(c.access(Address::new(0)), CacheEvent::Miss);
        assert_eq!(c.access(Address::new(0)), CacheEvent::Hit);
        // Same line (0..8), different byte -> still a hit.
        assert_eq!(c.access(Address::new(4)), CacheEvent::Hit);
        assert_eq!(c.stats(), CacheStats { hits: 2, misses: 1 });
    }

    #[test]
    fn known_pattern_conflicting_addresses_always_miss() {
        // 2 lines of 8 bytes -> addresses 0 and 16 both map to line 0
        // but have different tags, so alternating between them thrashes.
        let mut c = CacheModel::new(8, 2).unwrap();
        assert_eq!(c.access(Address::new(0)), CacheEvent::Miss);
        assert_eq!(c.access(Address::new(16)), CacheEvent::Miss);
        assert_eq!(c.access(Address::new(0)), CacheEvent::Miss);
        assert_eq!(c.access(Address::new(16)), CacheEvent::Miss);
        assert_eq!(c.stats().hits, 0);
        assert_eq!(c.stats().misses, 4);
    }

    #[test]
    fn hit_rate_reports_zero_before_any_access() {
        let c = CacheModel::new(8, 4).unwrap();
        assert_eq!(c.stats().hit_rate(), 0.0);
    }

    #[test]
    fn hit_rate_is_computed_correctly() {
        let mut c = CacheModel::new(8, 4).unwrap();
        c.access(Address::new(0)); // miss
        c.access(Address::new(0)); // hit
        c.access(Address::new(0)); // hit
        assert!((c.stats().hit_rate() - (2.0 / 3.0)).abs() < 1e-9);
    }

    #[test]
    fn access_range_spanning_two_lines_records_both() {
        let mut c = CacheModel::new(8, 4).unwrap();
        // [4, 12) spans line 0 (0..8) and line 1 (8..16): two misses.
        assert_eq!(c.access_range(Address::new(4), 8), CacheEvent::Miss);
        assert_eq!(c.stats().misses, 2);
        // Re-accessing the same range is now a hit on both lines.
        assert_eq!(c.access_range(Address::new(4), 8), CacheEvent::Hit);
    }

    #[test]
    fn flush_invalidates_without_resetting_stats() {
        let mut c = CacheModel::new(8, 4).unwrap();
        c.access(Address::new(0));
        c.access(Address::new(0));
        c.flush();
        assert_eq!(c.access(Address::new(0)), CacheEvent::Miss);
        assert_eq!(c.stats().misses, 2);
    }
}
