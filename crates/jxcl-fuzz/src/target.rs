//! Fuzzing target abstraction and harness for running fuzz campaigns.
//!
//! This module provides the core trait for plugging different fuzzing targets
//! (decoder, encoder, validators, etc.) into the fuzzing harness, along with
//! the campaign driver that executes them with test cases and tracks statistics.

use crate::prng::Xorshift64;

/// The outcome of executing a test case against a fuzzing target.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FuzzResult {
    /// The target processed the input successfully.
    Success,
    /// The target returned a decode error (expected for invalid input).
    DecodeError,
    /// The target encountered an unexpected condition or anomaly.
    Anomaly,
}

/// Statistics collected from a single fuzzing campaign.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FuzzStats {
    /// Total number of test cases executed.
    pub total_cases: usize,
    /// Number of test cases that succeeded.
    pub successful: usize,
    /// Number of test cases that resulted in decode errors.
    pub decode_errors: usize,
    /// Number of test cases that caused anomalies.
    pub anomalies: usize,
}

impl FuzzStats {
    /// Create new statistics with all counters at zero.
    pub fn new() -> Self {
        FuzzStats {
            total_cases: 0,
            successful: 0,
            decode_errors: 0,
            anomalies: 0,
        }
    }

    /// Calculate the coverage percentage (successful cases + decode errors).
    pub fn coverage_percent(&self) -> f64 {
        if self.total_cases == 0 {
            0.0
        } else {
            ((self.successful + self.decode_errors) as f64 / self.total_cases as f64) * 100.0
        }
    }

    /// Check if the campaign had any anomalies.
    pub fn has_anomalies(&self) -> bool {
        self.anomalies > 0
    }
}

impl Default for FuzzStats {
    fn default() -> Self {
        Self::new()
    }
}

/// A trait for types that can be fuzzed.
///
/// Implement this trait to plug a custom fuzzing target (decoder, encoder,
/// validator, etc.) into the fuzzing harness. The harness will generate
/// test cases and call `fuzz()` for each one, tracking the results.
pub trait FuzzTarget {
    /// Execute the fuzz target on the given input.
    ///
    /// This method should:
    /// - Never panic, regardless of input
    /// - Return `Success` if the input is valid and processing succeeded
    /// - Return `DecodeError` if the input is invalid and a structured error is returned
    /// - Return `Anomaly` if something unexpected occurs
    fn fuzz(&mut self, input: &[u8]) -> FuzzResult;

    /// Get a human-readable name for this fuzz target.
    fn name(&self) -> &'static str;
}

/// A fuzzing campaign that runs a target against a series of test cases.
///
/// The campaign generates test cases using a deterministic PRNG and executes
/// the fuzz target against each one, collecting statistics about the outcomes.
pub struct FuzzCampaign<'a, T: FuzzTarget> {
    target: &'a mut T,
}

impl<'a, T: FuzzTarget> FuzzCampaign<'a, T> {
    /// Create a new fuzzing campaign for the given target.
    pub fn new(target: &'a mut T) -> Self {
        FuzzCampaign { target }
    }

    /// Run the fuzzing campaign with the specified number of iterations.
    ///
    /// # Arguments
    ///
    /// * `iterations` - Number of test cases to generate and execute
    ///
    /// # Returns
    ///
    /// A `FuzzStats` structure containing the results of the campaign,
    /// including success counts, error counts, and coverage metrics.
    pub fn run(&mut self, iterations: usize) -> FuzzStats {
        let mut rng = Xorshift64::new(0xF00D_CAFE_1234_5678);
        let mut stats = FuzzStats::new();

        for _ in 0..iterations {
            // Generate a random-length buffer
            let len = (rng.next_u64() % 16) as usize;
            let mut buf = vec![0u8; len];
            rng.fill_bytes(&mut buf);

            // Execute the target and track the result
            let result = self.target.fuzz(&buf);
            match result {
                FuzzResult::Success => stats.successful += 1,
                FuzzResult::DecodeError => stats.decode_errors += 1,
                FuzzResult::Anomaly => stats.anomalies += 1,
            }

            stats.total_cases += 1;
        }

        stats
    }

    /// Run the fuzzing campaign with custom PRNG seeding (for reproducible testing).
    ///
    /// # Arguments
    ///
    /// * `iterations` - Number of test cases to generate and execute
    /// * `seed` - The seed value for the PRNG
    ///
    /// # Returns
    ///
    /// A `FuzzStats` structure containing the results of the campaign.
    pub fn run_with_seed(&mut self, iterations: usize, seed: u64) -> FuzzStats {
        let mut rng = Xorshift64::new(seed);
        let mut stats = FuzzStats::new();

        for _ in 0..iterations {
            // Generate a random-length buffer
            let len = (rng.next_u64() % 16) as usize;
            let mut buf = vec![0u8; len];
            rng.fill_bytes(&mut buf);

            // Execute the target and track the result
            let result = self.target.fuzz(&buf);
            match result {
                FuzzResult::Success => stats.successful += 1,
                FuzzResult::DecodeError => stats.decode_errors += 1,
                FuzzResult::Anomaly => stats.anomalies += 1,
            }

            stats.total_cases += 1;
        }

        stats
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A simple test target that always succeeds.
    struct AlwaysSuccessTarget;

    impl FuzzTarget for AlwaysSuccessTarget {
        fn fuzz(&mut self, _input: &[u8]) -> FuzzResult {
            FuzzResult::Success
        }

        fn name(&self) -> &'static str {
            "always_success"
        }
    }

    /// A simple test target that rejects empty input.
    struct RejectEmptyTarget;

    impl FuzzTarget for RejectEmptyTarget {
        fn fuzz(&mut self, input: &[u8]) -> FuzzResult {
            if input.is_empty() {
                FuzzResult::DecodeError
            } else {
                FuzzResult::Success
            }
        }

        fn name(&self) -> &'static str {
            "reject_empty"
        }
    }

    #[test]
    fn fuzz_stats_creation() {
        let stats = FuzzStats::new();
        assert_eq!(stats.total_cases, 0);
        assert_eq!(stats.successful, 0);
        assert_eq!(stats.decode_errors, 0);
        assert_eq!(stats.anomalies, 0);
    }

    #[test]
    fn fuzz_stats_coverage_calculation() {
        let mut stats = FuzzStats::new();
        stats.total_cases = 100;
        stats.successful = 60;
        stats.decode_errors = 40;
        stats.anomalies = 0;

        assert_eq!(stats.coverage_percent(), 100.0);
    }

    #[test]
    fn fuzz_stats_partial_coverage() {
        let mut stats = FuzzStats::new();
        stats.total_cases = 100;
        stats.successful = 30;
        stats.decode_errors = 20;
        stats.anomalies = 50;

        assert_eq!(stats.coverage_percent(), 50.0);
    }

    #[test]
    fn fuzz_stats_no_division_by_zero() {
        let stats = FuzzStats::new();
        assert_eq!(stats.coverage_percent(), 0.0);
    }

    #[test]
    fn fuzz_stats_has_anomalies() {
        let mut stats = FuzzStats::new();
        assert!(!stats.has_anomalies());

        stats.anomalies = 1;
        assert!(stats.has_anomalies());
    }

    #[test]
    fn fuzz_campaign_runs_successfully() {
        let mut target = AlwaysSuccessTarget;
        let mut campaign = FuzzCampaign::new(&mut target);

        let stats = campaign.run(100);

        assert_eq!(stats.total_cases, 100);
        assert_eq!(stats.successful, 100);
        assert_eq!(stats.decode_errors, 0);
        assert_eq!(stats.anomalies, 0);
    }

    #[test]
    fn fuzz_campaign_tracks_decode_errors() {
        let mut target = RejectEmptyTarget;
        let mut campaign = FuzzCampaign::new(&mut target);

        let stats = campaign.run(100);

        assert_eq!(stats.total_cases, 100);
        // Some test cases should be empty (rejected), others should succeed
        assert!(stats.decode_errors > 0);
        assert!(stats.successful > 0);
        assert_eq!(stats.anomalies, 0);
    }

    #[test]
    fn fuzz_campaign_with_seed_is_deterministic() {
        let mut target1 = AlwaysSuccessTarget;
        let mut campaign1 = FuzzCampaign::new(&mut target1);
        let stats1 = campaign1.run_with_seed(50, 0xDEADBEEF);

        let mut target2 = AlwaysSuccessTarget;
        let mut campaign2 = FuzzCampaign::new(&mut target2);
        let stats2 = campaign2.run_with_seed(50, 0xDEADBEEF);

        assert_eq!(stats1, stats2);
    }

    #[test]
    fn fuzz_campaign_different_seeds_produce_different_cases() {
        let mut target1 = RejectEmptyTarget;
        let mut campaign1 = FuzzCampaign::new(&mut target1);
        let stats1 = campaign1.run_with_seed(100, 0x1111111111111111);

        let mut target2 = RejectEmptyTarget;
        let mut campaign2 = FuzzCampaign::new(&mut target2);
        let stats2 = campaign2.run_with_seed(100, 0x2222222222222222);

        // Different seeds should produce different results (very likely)
        assert!(stats1.successful != stats2.successful || stats1.decode_errors != stats2.decode_errors);
    }
}
