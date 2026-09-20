// Copyright (C) 2026 ERRANT DIGITAL INSTITUTE OF TECHNOLOGY (SEIT NGO) % SNAPKITTY COLLECTIVE
// SPDX-License-Identifier: AGPL-3.0-only OR LicenseRef-Commercial
// See LICENSE-AGPL and LICENSE-COMMERCIAL in this repository for license terms.

//! Aggregates a [`jxcl_trace::TraceLog`], a per-instruction cycle-cost
//! model, and a cache-access model into run statistics: instruction
//! count, total cycles, and cache hit rate.
//!
//! ## Cross-batch integration note
//!
//! `docs/crates.toml` lists `jxcl-cycle-model` and `jxcl-cache-model` as
//! this crate's dependencies (both still scaffolded placeholders as of
//! this batch, alongside `jxcl-trace`, which is real — same batch).
//! Rather than hard-depend on their not-yet-existing concrete types
//! (`CycleCounter`/`cycle_cost` and `CacheModel`), this crate defines
//! the two small capability traits it actually needs
//! ([`CycleCostModel`] and [`CacheAccessModel`]) and is generic over
//! them. This is a real, immediately testable dependency-inversion:
//! once `jxcl-cycle-model::CycleCounter` and
//! `jxcl-cache-model::CacheModel` land, implementing these two traits
//! for them (a couple of lines each, delegating to their real
//! `cycle_cost`/access-recording methods) is all `profile_run` needs to
//! start consuming real cycle/cache data — nothing in this crate
//! changes.

#![forbid(unsafe_code)]

use jxcl_trace::TraceLog;

/// What [`profile_run`] needs from a cycle-cost model: the cost, in
/// cycles, of the instruction that produced one [`jxcl_trace::TraceStep`].
///
/// A real implementor (`jxcl-cycle-model::CycleCounter`, once landed)
/// looks the instruction's opcode up in `CYCLE_COST_TABLE`; this trait
/// only asks for the *result* of that lookup so `jxcl-profiler` never
/// needs to depend on the opcode table itself.
pub trait CycleCostModel {
    /// The cycle cost of executing the instruction whose rendered text
    /// is `instruction_text` (i.e. `TraceStep::instruction`).
    fn cycle_cost(&self, instruction_text: &str) -> u64;
}

/// What [`profile_run`] needs from a cache model: whether one memory
/// access at `address` was a hit or a miss, and whatever
/// state-mutation that access implies for subsequent lookups.
///
/// A real implementor (`jxcl-cache-model::CacheModel`, once landed)
/// performs the actual set-associative/direct-mapped lookup and
/// eviction; this trait only asks for the hit/miss outcome.
pub trait CacheAccessModel {
    /// Record one access to `address`, returning `true` for a hit.
    fn access(&mut self, address: u64) -> bool;
}

/// Aggregated statistics for one recorded run.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ProfileReport {
    pub instruction_count: u64,
    pub total_cycles: u64,
    pub cache_accesses: u64,
    pub cache_hits: u64,
}

impl ProfileReport {
    /// Cache hit rate in `[0.0, 1.0]`; `1.0` (vacuously) if the run
    /// performed no memory accesses at all.
    pub fn cache_hit_rate(&self) -> f64 {
        if self.cache_accesses == 0 {
            1.0
        } else {
            self.cache_hits as f64 / self.cache_accesses as f64
        }
    }
}

/// Walk `trace` once, charging each step's cycle cost via `cycle_model`
/// and feeding each of its memory writes through `cache_model` as one
/// access apiece, producing the aggregated [`ProfileReport`].
///
/// Charging cache accesses for `TraceStep::memory_writes` (rather than
/// also modeling reads, which the trace format does not currently
/// record) is a deliberate, documented scope choice: it is exactly the
/// memory traffic `jxcl-trace` captures today, and the same aggregation
/// logic below applies unchanged once reads are added to `TraceStep`.
pub fn profile_run(
    trace: &TraceLog,
    cycle_model: &impl CycleCostModel,
    cache_model: &mut impl CacheAccessModel,
) -> ProfileReport {
    let mut report = ProfileReport {
        instruction_count: 0,
        total_cycles: 0,
        cache_accesses: 0,
        cache_hits: 0,
    };

    for step in trace.iter() {
        report.instruction_count += 1;
        report.total_cycles += cycle_model.cycle_cost(&step.instruction);

        for (address, _old, _new) in &step.memory_writes {
            report.cache_accesses += 1;
            if cache_model.access(*address) {
                report.cache_hits += 1;
            }
        }
    }

    report
}

#[cfg(test)]
mod tests {
    use super::*;
    use jxcl_trace::TraceStep;
    use std::collections::HashMap;

    /// A fixed lookup-table cycle model, standing in for
    /// `jxcl-cycle-model::CycleCounter`'s eventual `CYCLE_COST_TABLE`
    /// lookup.
    struct FixedCycleModel {
        default_cost: u64,
        overrides: HashMap<&'static str, u64>,
    }

    impl CycleCostModel for FixedCycleModel {
        fn cycle_cost(&self, instruction_text: &str) -> u64 {
            for (prefix, cost) in &self.overrides {
                if instruction_text.starts_with(prefix) {
                    return *cost;
                }
            }
            self.default_cost
        }
    }

    /// A tiny direct-mapped cache test double, standing in for
    /// `jxcl-cache-model::CacheModel`.
    struct DirectMappedCache {
        lines: Vec<Option<u64>>,
    }

    impl DirectMappedCache {
        fn new(line_count: usize) -> Self {
            DirectMappedCache {
                lines: vec![None; line_count],
            }
        }
    }

    impl CacheAccessModel for DirectMappedCache {
        fn access(&mut self, address: u64) -> bool {
            let line = (address as usize) % self.lines.len();
            let tag = address / self.lines.len() as u64;
            let hit = self.lines[line] == Some(tag);
            self.lines[line] = Some(tag);
            hit
        }
    }

    fn sample_trace() -> TraceLog {
        let mut trace = TraceLog::new();
        trace.push(TraceStep {
            pc: 0,
            pc_after: 4,
            instruction: "MOVI R1, 10".to_string(),
            register_changes: vec![(1, 0, 10)],
            flags_before: 0,
            flags_after: 0,
            sp_before: 256,
            sp_after: 256,
            memory_writes: vec![],
        });
        trace.push(TraceStep {
            pc: 4,
            pc_after: 8,
            instruction: "STORE [64], R1".to_string(),
            register_changes: vec![],
            flags_before: 0,
            flags_after: 0,
            sp_before: 256,
            sp_after: 256,
            memory_writes: vec![(64, vec![0], vec![10])],
        });
        // Same address again: the second access to it should hit.
        trace.push(TraceStep {
            pc: 8,
            pc_after: 12,
            instruction: "STORE [64], R2".to_string(),
            register_changes: vec![],
            flags_before: 0,
            flags_after: 0,
            sp_before: 256,
            sp_after: 256,
            memory_writes: vec![(64, vec![10], vec![20])],
        });
        trace.push(TraceStep {
            pc: 12,
            pc_after: 16,
            instruction: "HALT".to_string(),
            register_changes: vec![],
            flags_before: 0,
            flags_after: 0,
            sp_before: 256,
            sp_after: 256,
            memory_writes: vec![],
        });
        trace
    }

    #[test]
    fn profile_counts_instructions_and_charges_cycles() {
        let trace = sample_trace();
        let cycle_model = FixedCycleModel {
            default_cost: 1,
            overrides: HashMap::new(),
        };
        let mut cache_model = DirectMappedCache::new(16);

        let report = profile_run(&trace, &cycle_model, &mut cache_model);
        assert_eq!(report.instruction_count, 4);
        assert_eq!(report.total_cycles, 4);
    }

    #[test]
    fn profile_uses_per_instruction_cycle_costs() {
        let trace = sample_trace();
        let mut overrides = HashMap::new();
        overrides.insert("STORE", 3u64);
        overrides.insert("HALT", 1u64);
        overrides.insert("MOVI", 2u64);
        let cycle_model = FixedCycleModel {
            default_cost: 1,
            overrides,
        };
        let mut cache_model = DirectMappedCache::new(16);

        let report = profile_run(&trace, &cycle_model, &mut cache_model);
        // MOVI(2) + STORE(3) + STORE(3) + HALT(1) = 9
        assert_eq!(report.total_cycles, 9);
    }

    #[test]
    fn profile_computes_cache_hit_rate_from_repeated_addresses() {
        let trace = sample_trace();
        let cycle_model = FixedCycleModel {
            default_cost: 1,
            overrides: HashMap::new(),
        };
        let mut cache_model = DirectMappedCache::new(16);

        let report = profile_run(&trace, &cycle_model, &mut cache_model);
        // Two memory writes, both to address 64: first is a cold miss,
        // second is a hit against the same cache line/tag.
        assert_eq!(report.cache_accesses, 2);
        assert_eq!(report.cache_hits, 1);
        assert!((report.cache_hit_rate() - 0.5).abs() < f64::EPSILON);
    }

    #[test]
    fn empty_trace_has_a_vacuous_perfect_hit_rate() {
        let trace = TraceLog::new();
        let cycle_model = FixedCycleModel {
            default_cost: 1,
            overrides: HashMap::new(),
        };
        let mut cache_model = DirectMappedCache::new(16);
        let report = profile_run(&trace, &cycle_model, &mut cache_model);
        assert_eq!(report.instruction_count, 0);
        assert_eq!(report.cache_hit_rate(), 1.0);
    }

    /// The required determinism test: profiling the *same* trace twice,
    /// with fresh model instances each time, must produce identical
    /// reports.
    #[test]
    fn profiling_the_same_trace_twice_is_deterministic() {
        let trace = sample_trace();

        let run = || {
            let cycle_model = FixedCycleModel {
                default_cost: 2,
                overrides: HashMap::new(),
            };
            let mut cache_model = DirectMappedCache::new(8);
            profile_run(&trace, &cycle_model, &mut cache_model)
        };

        let first = run();
        let second = run();
        assert_eq!(first, second);
    }
}
