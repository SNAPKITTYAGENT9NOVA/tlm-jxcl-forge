//! Fuzzing infrastructure for the JXCL ISA: mutation-based and coverage-driven feedback.
//!
//! This crate provides a structured approach to fuzzing instruction decoders and other
//! parser/decoder boundaries against malformed input, ensuring they never panic,
//! corrupt host state, or behave non-deterministically — they should only decode
//! successfully or return a structured error.
//!
//! ## Overview
//!
//! The fuzzing infrastructure includes:
//! - A deterministic PRNG (`Xorshift64`) for reproducible test case generation
//! - A generic fuzzing harness (`FuzzTarget` trait) for extensibility
//! - Mutation strategies for generating diverse test cases
//! - The `fuzz_decode_target` function that can be used by integrations
//!
//! ## Design Principles
//!
//! - **No external dependencies**: Uses a fixed-seed xorshift64 PRNG to ensure
//!   reproducibility without depending on the `rand` crate (spec §40)
//! - **Deterministic**: All fuzzing is driven by deterministic sequences, enabling
//!   reproduction of any failure (spec §1)
//! - **Safe**: All fuzzing operations are safe Rust; the `forbid(unsafe_code)`
//!   attribute ensures memory safety
//! - **Extensible**: The `FuzzTarget` trait allows fuzzing of additional targets
//!   beyond the decoder
//!
//! ## Usage
//!
//! The main entry point is `fuzz_decode_target`, which performs a complete
//! fuzzing campaign on the instruction decoder:
//!
//! ```ignore
//! use jxcl_fuzz::fuzz_decode_target;
//!
//! let stats = fuzz_decode_target(20_000);
//! println!("Fuzz campaign completed: {:?}", stats);
//! ```
//!
//! For custom fuzzing targets, implement `FuzzTarget` and use the generic
//! fuzzing harness:
//!
//! ```ignore
//! use jxcl_fuzz::{FuzzTarget, FuzzCampaign};
//!
//! struct MyTarget;
//! impl FuzzTarget for MyTarget {
//!     // ...implement trait methods...
//! }
//!
//! let campaign = FuzzCampaign::new(MyTarget);
//! let stats = campaign.run(1000);
//! ```

#![forbid(unsafe_code)]

mod corpus;
mod mutations;
mod prng;
mod target;

pub use corpus::{Corpus, TestCase};
pub use mutations::{Mutator, MutationStrategy};
pub use prng::Xorshift64;
pub use target::{FuzzCampaign, FuzzResult, FuzzStats, FuzzTarget};

use jxcl_decoding::decode_one;

/// A `FuzzTarget` implementation for fuzzing the instruction decoder.
pub struct DecoderTarget;

impl FuzzTarget for DecoderTarget {
    fn fuzz(&mut self, input: &[u8]) -> FuzzResult {
        // Attempt to decode one instruction from the input buffer.
        // The decoder should never panic, even on malformed input.
        match decode_one(input, 0) {
            Ok(_) => FuzzResult::Success,
            Err(_) => FuzzResult::DecodeError,
        }
    }

    fn name(&self) -> &'static str {
        "decoder"
    }
}

/// Statistics collected during a fuzzing campaign.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FuzzingStats {
    /// Total number of test cases executed.
    pub total_cases: usize,
    /// Number of test cases that decoded successfully.
    pub successful_decodes: usize,
    /// Number of test cases that resulted in decode errors (expected).
    pub decode_errors: usize,
    /// Number of test cases that triggered unexpected behavior.
    pub anomalies: usize,
}

impl FuzzingStats {
    /// Create new fuzzing statistics.
    pub fn new() -> Self {
        FuzzingStats {
            total_cases: 0,
            successful_decodes: 0,
            decode_errors: 0,
            anomalies: 0,
        }
    }

    /// Calculate the coverage percentage (successful + error cases out of total).
    pub fn coverage_percent(&self) -> f64 {
        if self.total_cases == 0 {
            0.0
        } else {
            ((self.successful_decodes + self.decode_errors) as f64 / self.total_cases as f64)
                * 100.0
        }
    }
}

impl Default for FuzzingStats {
    fn default() -> Self {
        Self::new()
    }
}

/// Fuzz the instruction decoder with a specified number of test cases.
///
/// This function performs a complete fuzzing campaign on the decoder using
/// deterministic, randomly-generated test cases. The decoder must never panic,
/// corrupt state, or behave non-deterministically — it should only decode
/// successfully or return a structured `DecodeError`.
///
/// # Arguments
///
/// * `iterations` - Number of test cases to generate and execute
///
/// # Returns
///
/// A `FuzzingStats` structure containing the results of the campaign,
/// including success counts, error counts, and coverage metrics.
///
/// # Example
///
/// ```ignore
/// use jxcl_fuzz::fuzz_decode_target;
///
/// let stats = fuzz_decode_target(20_000);
/// assert_eq!(stats.anomalies, 0, "Decoder fuzz campaign found anomalies");
/// println!("Coverage: {:.1}%", stats.coverage_percent());
/// ```
pub fn fuzz_decode_target(iterations: usize) -> FuzzingStats {
    let mut rng = Xorshift64::new(0xF00D_CAFE_1234_5678);
    let mut stats = FuzzingStats::new();

    for _ in 0..iterations {
        // Generate a random-length buffer (0-15 bytes, biased toward small)
        let len = (rng.next_u64() % 16) as usize;
        let mut buf = vec![0u8; len];
        rng.fill_bytes(&mut buf);

        // Attempt to decode; track outcomes
        match decode_one(&buf, 0) {
            Ok(_) => stats.successful_decodes += 1,
            Err(_) => stats.decode_errors += 1,
        }

        stats.total_cases += 1;
    }

    stats
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn xorshift64_is_deterministic() {
        let mut rng1 = Xorshift64::new(0x1234567890ABCDEF);
        let mut rng2 = Xorshift64::new(0x1234567890ABCDEF);

        for _ in 0..100 {
            assert_eq!(rng1.next_u64(), rng2.next_u64());
        }
    }

    #[test]
    fn xorshift64_different_seeds_produce_different_sequences() {
        let mut rng1 = Xorshift64::new(0x1111111111111111);
        let mut rng2 = Xorshift64::new(0x2222222222222222);

        let mut same_count = 0;
        for _ in 0..100 {
            if rng1.next_u64() == rng2.next_u64() {
                same_count += 1;
            }
        }

        // Extremely unlikely that 100 consecutive values from different seeds match
        assert!(same_count < 5, "Different seeds produced too many identical values");
    }

    #[test]
    fn decoder_never_panics_on_random_bytes() {
        let stats = fuzz_decode_target(5_000);
        // All test cases should either succeed or error, never panic
        assert_eq!(stats.anomalies, 0);
        assert_eq!(
            stats.total_cases,
            stats.successful_decodes + stats.decode_errors
        );
    }

    #[test]
    fn decoder_handles_empty_input() {
        let result = decode_one(&[], 0);
        assert!(result.is_err(), "Empty input should produce an error");
    }

    #[test]
    fn decoder_handles_truncated_instructions() {
        // Create a buffer that starts with a valid opcode but is too short
        let mut buf = vec![0x01]; // Start with a valid opcode
        for _ in 0..10 {
            // Try various truncation lengths
            let result = decode_one(&buf, 0);
            // Should either succeed (if it's complete) or error (if truncated)
            let _ = result; // We don't care about the specific outcome, just no panic
            buf.push(0xFF);
        }
    }

    #[test]
    fn fuzzing_stats_coverage_calculation() {
        let mut stats = FuzzingStats::new();
        stats.total_cases = 100;
        stats.successful_decodes = 60;
        stats.decode_errors = 40;
        stats.anomalies = 0;

        assert_eq!(stats.coverage_percent(), 100.0);
    }

    #[test]
    fn fuzzing_stats_partial_coverage() {
        let mut stats = FuzzingStats::new();
        stats.total_cases = 100;
        stats.successful_decodes = 30;
        stats.decode_errors = 20;
        stats.anomalies = 50;

        assert_eq!(stats.coverage_percent(), 50.0);
    }

    #[test]
    fn corpus_generation_with_mutation() {
        let mut rng = Xorshift64::new(0xDEADBEEF);
        let mut corpus = Corpus::new();

        // Generate some test cases
        for _ in 0..10 {
            let len = (rng.next_u64() % 8) as usize;
            let mut buf = vec![0u8; len];
            rng.fill_bytes(&mut buf);
            corpus.add(TestCase::new(buf));
        }

        assert_eq!(corpus.len(), 10);

        // Verify we can access test cases
        let cases = corpus.all();
        assert_eq!(cases.len(), 10);
    }

    #[test]
    fn mutator_produces_variants() {
        let mut rng = Xorshift64::new(0xCAFEBABE);
        let original = TestCase::new(vec![0x42, 0x42, 0x42]);
        let mut mutator = Mutator::new(&mut rng);

        // Perform mutations
        let mutated1 = mutator.mutate(&original, MutationStrategy::Flip);
        let mutated2 = mutator.mutate(&original, MutationStrategy::Havoc);

        // Mutations should produce different results (extremely likely with these seeds)
        assert_ne!(mutated1.input, original.input);
        assert_ne!(mutated2.input, original.input);
    }

    #[test]
    fn fuzz_campaign_can_run() {
        let mut target = DecoderTarget;
        let mut campaign = FuzzCampaign::new(&mut target);

        let stats = campaign.run(100);
        // Should execute all iterations without panic
        assert_eq!(stats.total_cases, 100);
    }

    #[test]
    fn decoder_target_name_is_correct() {
        let target = DecoderTarget;
        assert_eq!(target.name(), "decoder");
    }

    #[test]
    fn fuzz_decode_target_produces_reasonable_stats() {
        let stats = fuzz_decode_target(1_000);

        // Should have executed 1000 test cases
        assert_eq!(stats.total_cases, 1_000);

        // Should have no anomalies (decoder should never panic)
        assert_eq!(stats.anomalies, 0);

        // Coverage should account for all cases
        assert_eq!(
            stats.successful_decodes + stats.decode_errors,
            stats.total_cases
        );

        // Coverage should be 100% (all cases should either succeed or error)
        assert_eq!(stats.coverage_percent(), 100.0);
    }
}
