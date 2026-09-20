//! Mutation strategies for generating diverse fuzzing test cases.
//!
//! This module provides different strategies for mutating test cases to explore
//! different regions of the input space. The mutation strategies are designed to
//! find edge cases and corner cases that might otherwise be missed by purely
//! random generation.

use crate::corpus::TestCase;
use crate::prng::Xorshift64;

/// Different mutation strategies for test case generation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MutationStrategy {
    /// Flip individual bits in the input.
    ///
    /// This strategy randomly selects a byte and bit position, then toggles
    /// that bit. It's effective for finding boolean condition boundaries.
    Flip,

    /// Havoc: apply multiple random mutations in sequence.
    ///
    /// This strategy applies a random number of bit flips to different
    /// positions in the input, simulating the cumulative effect of multiple
    /// bit corruptions.
    Havoc,

    /// Byte swap: exchange bytes at two random positions.
    ///
    /// This strategy randomly selects two byte positions and swaps them,
    /// useful for testing byte-order assumptions and memory layout handling.
    ByteSwap,

    /// Insert random bytes at a random position.
    ///
    /// This strategy inserts one or more random bytes at a random position,
    /// useful for testing buffer overflow handling and truncation.
    Insert,

    /// Delete bytes at a random position.
    ///
    /// This strategy removes one or more bytes at a random position,
    /// useful for testing underflow handling and missing data.
    Delete,

    /// Overwrite a section with random bytes.
    ///
    /// This strategy replaces a random section of the input with random bytes,
    /// simulating data corruption.
    Overwrite,
}

impl MutationStrategy {
    /// Get a descriptive name for this strategy.
    pub fn name(&self) -> &'static str {
        match self {
            MutationStrategy::Flip => "flip",
            MutationStrategy::Havoc => "havoc",
            MutationStrategy::ByteSwap => "byte_swap",
            MutationStrategy::Insert => "insert",
            MutationStrategy::Delete => "delete",
            MutationStrategy::Overwrite => "overwrite",
        }
    }
}

/// A mutation engine that applies various mutation strategies to test cases.
pub struct Mutator<'a> {
    rng: &'a mut Xorshift64,
}

impl<'a> Mutator<'a> {
    /// Create a new mutator using the given PRNG.
    pub fn new(rng: &'a mut Xorshift64) -> Self {
        Mutator { rng }
    }

    /// Apply a mutation strategy to a test case, returning a new mutated test case.
    pub fn mutate(&mut self, case: &TestCase, strategy: MutationStrategy) -> TestCase {
        let mutated = match strategy {
            MutationStrategy::Flip => self.mutate_flip(case),
            MutationStrategy::Havoc => self.mutate_havoc(case),
            MutationStrategy::ByteSwap => self.mutate_byte_swap(case),
            MutationStrategy::Insert => self.mutate_insert(case),
            MutationStrategy::Delete => self.mutate_delete(case),
            MutationStrategy::Overwrite => self.mutate_overwrite(case),
        };
        TestCase::new(mutated)
    }

    /// Flip a random bit in the input.
    fn mutate_flip(&mut self, case: &TestCase) -> Vec<u8> {
        let mut buf = case.input.clone();
        if !buf.is_empty() {
            let byte_idx = (self.rng.next_u64() as usize) % buf.len();
            let bit_idx = (self.rng.next_u64() % 8) as usize;
            buf[byte_idx] ^= 1 << bit_idx;
        }
        buf
    }

    /// Apply multiple random bit flips (havoc mutation).
    fn mutate_havoc(&mut self, case: &TestCase) -> Vec<u8> {
        let mut buf = case.input.clone();
        let flip_count = ((self.rng.next_u64() % 8) as usize) + 1;

        for _ in 0..flip_count {
            if !buf.is_empty() {
                let byte_idx = (self.rng.next_u64() as usize) % buf.len();
                let bit_idx = (self.rng.next_u64() % 8) as usize;
                buf[byte_idx] ^= 1 << bit_idx;
            }
        }
        buf
    }

    /// Swap two random bytes.
    fn mutate_byte_swap(&mut self, case: &TestCase) -> Vec<u8> {
        let mut buf = case.input.clone();
        if buf.len() >= 2 {
            let idx1 = (self.rng.next_u64() as usize) % buf.len();
            let idx2 = (self.rng.next_u64() as usize) % buf.len();
            buf.swap(idx1, idx2);
        }
        buf
    }

    /// Insert random bytes at a random position.
    fn mutate_insert(&mut self, case: &TestCase) -> Vec<u8> {
        let mut buf = case.input.clone();
        let insert_count = ((self.rng.next_u64() % 4) as usize) + 1;
        let insert_pos = if buf.is_empty() {
            0
        } else {
            (self.rng.next_u64() as usize) % buf.len()
        };

        for i in 0..insert_count {
            let byte = self.rng.next_u8();
            buf.insert(insert_pos + i, byte);
        }
        buf
    }

    /// Delete bytes at a random position.
    fn mutate_delete(&mut self, case: &TestCase) -> Vec<u8> {
        let mut buf = case.input.clone();
        if !buf.is_empty() {
            let delete_count =
                ((self.rng.next_u64() % (buf.len() as u64)).max(1) as usize).min(buf.len());
            let delete_pos = (self.rng.next_u64() as usize) % buf.len();

            for _ in 0..delete_count {
                if delete_pos < buf.len() {
                    buf.remove(delete_pos);
                }
            }
        }
        buf
    }

    /// Overwrite a section with random bytes.
    fn mutate_overwrite(&mut self, case: &TestCase) -> Vec<u8> {
        let mut buf = case.input.clone();
        if !buf.is_empty() {
            let overwrite_len = ((self.rng.next_u64() % 8) as usize) + 1;
            let overwrite_pos = (self.rng.next_u64() as usize) % buf.len();

            for i in 0..overwrite_len {
                if overwrite_pos + i < buf.len() {
                    buf[overwrite_pos + i] = self.rng.next_u8();
                }
            }
        }
        buf
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mutation_strategy_names() {
        assert_eq!(MutationStrategy::Flip.name(), "flip");
        assert_eq!(MutationStrategy::Havoc.name(), "havoc");
        assert_eq!(MutationStrategy::ByteSwap.name(), "byte_swap");
        assert_eq!(MutationStrategy::Insert.name(), "insert");
        assert_eq!(MutationStrategy::Delete.name(), "delete");
        assert_eq!(MutationStrategy::Overwrite.name(), "overwrite");
    }

    #[test]
    fn mutate_flip_changes_input() {
        let mut rng = Xorshift64::new(0x1234);
        let mut mutator = Mutator::new(&mut rng);

        let case = TestCase::new(vec![0x00, 0x00, 0x00]);
        let mutated = mutator.mutate(&case, MutationStrategy::Flip);

        // After flipping a bit, should have changed
        assert_ne!(mutated.input, case.input);
    }

    #[test]
    fn mutate_havoc_creates_variant() {
        let mut rng = Xorshift64::new(0x5678);
        let mut mutator = Mutator::new(&mut rng);

        let case = TestCase::new(vec![0x42; 10]);
        let mutated = mutator.mutate(&case, MutationStrategy::Havoc);

        // Havoc should typically change something
        assert_ne!(mutated.input, case.input);
    }

    #[test]
    fn mutate_byte_swap_changes_order() {
        let mut rng = Xorshift64::new(0xABCD);
        let mut mutator = Mutator::new(&mut rng);

        let case = TestCase::new(vec![0x11, 0x22, 0x33]);
        let mutated = mutator.mutate(&case, MutationStrategy::ByteSwap);

        // Should preserve all bytes
        let mut sorted_original = case.input.clone();
        let mut sorted_mutated = mutated.input.clone();
        sorted_original.sort();
        sorted_mutated.sort();
        assert_eq!(sorted_original, sorted_mutated);
    }

    #[test]
    fn mutate_insert_increases_length() {
        let mut rng = Xorshift64::new(0xDEF0);
        let mut mutator = Mutator::new(&mut rng);

        let case = TestCase::new(vec![0x42; 5]);
        let mutated = mutator.mutate(&case, MutationStrategy::Insert);

        // Should be longer after insertion
        assert!(mutated.input.len() > case.input.len());
    }

    #[test]
    fn mutate_delete_decreases_length() {
        let mut rng = Xorshift64::new(0x1111);
        let mut mutator = Mutator::new(&mut rng);

        let case = TestCase::new(vec![0x42; 10]);
        let mutated = mutator.mutate(&case, MutationStrategy::Delete);

        // Should be shorter after deletion
        assert!(mutated.input.len() < case.input.len());
    }

    #[test]
    fn mutate_overwrite_preserves_length() {
        let mut rng = Xorshift64::new(0x2222);
        let mut mutator = Mutator::new(&mut rng);

        let case = TestCase::new(vec![0x42; 8]);
        let mutated = mutator.mutate(&case, MutationStrategy::Overwrite);

        // Should preserve length in most cases (overwrite doesn't add/remove)
        assert_eq!(mutated.input.len(), case.input.len());
    }

    #[test]
    fn mutate_flip_empty_input_is_safe() {
        let mut rng = Xorshift64::new(0x3333);
        let mut mutator = Mutator::new(&mut rng);

        let case = TestCase::new(vec![]);
        let mutated = mutator.mutate(&case, MutationStrategy::Flip);

        // Should stay empty (no bytes to flip)
        assert!(mutated.input.is_empty());
    }

    #[test]
    fn mutate_all_strategies_work_on_valid_input() {
        let mut rng = Xorshift64::new(0x4444);
        let mut mutator = Mutator::new(&mut rng);

        let case = TestCase::new(vec![0x42; 8]);

        let strategies = vec![
            MutationStrategy::Flip,
            MutationStrategy::Havoc,
            MutationStrategy::ByteSwap,
            MutationStrategy::Insert,
            MutationStrategy::Delete,
            MutationStrategy::Overwrite,
        ];

        for strategy in strategies {
            let mutated = mutator.mutate(&case, strategy);
            // Should never panic and should always return something
            assert!(!mutated.input.is_empty() || case.input.is_empty());
        }
    }
}
