//! Deterministic pseudo-random number generator for fuzzing.
//!
//! A fixed-seed xorshift64 generator that provides fully reproducible
//! "random" byte streams for fuzzing campaigns, ensuring that any failure
//! can be reliably reproduced from the same seed.
//!
//! This is the same PRNG used by `jxcl/tests/common/mod.rs` (spec §40: no
//! unnecessary dependencies); a deterministic generator is essential for
//! reproducible testing (spec §1: "Given identical input bytes ... must
//! produce exactly the same architectural result").

/// A deterministic pseudo-random number generator using the xorshift64 algorithm.
///
/// This PRNG is seeded with a fixed value, ensuring that the sequence of
/// generated values is always identical for reproducible fuzzing campaigns.
/// If the seed is 0, it is replaced with a default non-zero value to ensure
/// the generator always produces values.
///
/// # Example
///
/// ```
/// use jxcl_fuzz::Xorshift64;
///
/// let mut rng = Xorshift64::new(0x1234567890ABCDEF);
/// let value1 = rng.next_u64();
/// let value2 = rng.next_u64();
///
/// // Create another generator with the same seed
/// let mut rng2 = Xorshift64::new(0x1234567890ABCDEF);
/// assert_eq!(value1, rng2.next_u64());
/// assert_eq!(value2, rng2.next_u64());
/// ```
#[derive(Debug, Clone)]
pub struct Xorshift64 {
    state: u64,
}

impl Xorshift64 {
    /// Create a new PRNG seeded with the given value.
    ///
    /// If `seed` is 0, it is replaced with 0xDEADBEEFCAFEBABE to ensure
    /// the generator always produces non-zero sequences.
    pub fn new(seed: u64) -> Self {
        Xorshift64 {
            state: if seed == 0 {
                0xDEAD_BEEF_CAFE_BABE
            } else {
                seed
            },
        }
    }

    /// Generate the next pseudo-random 64-bit unsigned integer.
    ///
    /// Uses the xorshift64 algorithm:
    /// 1. XOR the state left by 13 bits
    /// 2. XOR the state right by 7 bits
    /// 3. XOR the state left by 17 bits
    /// 4. Update internal state
    /// 5. Return the updated state
    pub fn next_u64(&mut self) -> u64 {
        let mut x = self.state;
        x ^= x << 13;
        x ^= x >> 7;
        x ^= x << 17;
        self.state = x;
        x
    }

    /// Generate the next pseudo-random 8-bit unsigned integer.
    pub fn next_u8(&mut self) -> u8 {
        (self.next_u64() & 0xFF) as u8
    }

    /// Fill a buffer with pseudo-random bytes.
    ///
    /// Each byte is generated independently using `next_u8()`, ensuring
    /// full coverage of the buffer with deterministic values.
    pub fn fill_bytes(&mut self, buf: &mut [u8]) {
        for b in buf.iter_mut() {
            *b = self.next_u8();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn xorshift64_is_deterministic() {
        let mut rng1 = Xorshift64::new(0x1234567890ABCDEF);
        let mut rng2 = Xorshift64::new(0x1234567890ABCDEF);

        for _ in 0..1000 {
            assert_eq!(rng1.next_u64(), rng2.next_u64());
        }
    }

    #[test]
    fn xorshift64_zero_seed_is_mapped() {
        let rng1 = Xorshift64::new(0);
        let rng2 = Xorshift64::new(0xDEAD_BEEF_CAFE_BABE);

        assert_eq!(rng1.state, rng2.state);
    }

    #[test]
    fn xorshift64_next_u8_is_valid() {
        let mut rng = Xorshift64::new(0xCAFECAFE);
        for _ in 0..1000 {
            let _val = rng.next_u8();
            // next_u8() always returns a valid u8 by definition
        }
    }

    #[test]
    fn xorshift64_fill_bytes_is_deterministic() {
        let mut rng1 = Xorshift64::new(0x5555555555555555);
        let mut buf1 = vec![0u8; 100];
        rng1.fill_bytes(&mut buf1);

        let mut rng2 = Xorshift64::new(0x5555555555555555);
        let mut buf2 = vec![0u8; 100];
        rng2.fill_bytes(&mut buf2);

        assert_eq!(buf1, buf2);
    }

    #[test]
    fn xorshift64_fill_bytes_produces_variety() {
        let mut rng = Xorshift64::new(0xAAAAAAAAAAAAAAAA);
        let mut buf = vec![0u8; 100];
        rng.fill_bytes(&mut buf);

        // Count distinct values (should have some variety)
        let mut seen = [false; 256];
        for &b in &buf {
            seen[b as usize] = true;
        }

        let distinct = seen.iter().filter(|&&x| x).count();
        assert!(distinct > 10, "PRNG should produce variety in 100 bytes");
    }
}
