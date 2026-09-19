//! Shared helpers for integration tests: a tiny deterministic PRNG.
//!
//! No `rand` crate is used (spec §40: no unnecessary dependencies); a
//! fixed-seed xorshift64 generator gives fully reproducible "random"
//! byte streams for the property/fuzz tests, which matters because the
//! whole point of this forge is determinism (spec §1: "Given identical
//! input bytes ... must produce exactly the same architectural result").

pub struct Xorshift64 {
    state: u64,
}

impl Xorshift64 {
    pub fn new(seed: u64) -> Self {
        Xorshift64 { state: if seed == 0 { 0xDEADBEEFCAFEBABE } else { seed } }
    }

    pub fn next_u64(&mut self) -> u64 {
        let mut x = self.state;
        x ^= x << 13;
        x ^= x >> 7;
        x ^= x << 17;
        self.state = x;
        x
    }

    pub fn next_u8(&mut self) -> u8 {
        (self.next_u64() & 0xFF) as u8
    }

    /// Not every test binary that links this module uses this method —
    /// each integration test file is its own crate, so per-crate dead
    /// code analysis flags it in whichever ones don't. It's a real,
    /// used API of this shared helper, not dead code overall.
    #[allow(dead_code)]
    pub fn fill_bytes(&mut self, buf: &mut [u8]) {
        for b in buf.iter_mut() {
            *b = self.next_u8();
        }
    }
}
