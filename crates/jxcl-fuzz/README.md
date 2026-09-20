# jxcl-fuzz

Fuzzing infrastructure for the JXCL ISA: mutation-based and coverage-driven feedback for instruction codec testing.

## Architecture

**Owns:** Fuzzing harness, test case corpus, mutation strategies, and the main decoder fuzz target.

**Category:** debug · **Source:** extraction:jxcl/tests/fuzz_decoder.rs

## Public API

- `fuzz_decode_target(iterations)` - Execute a complete fuzzing campaign on the decoder
- `FuzzTarget` (trait) - Extensible trait for custom fuzzing targets
- `FuzzCampaign` - Generic fuzzing harness for running campaigns
- `FuzzStats`/`FuzzResult` - Result types for fuzzing outcomes
- `Corpus`, `TestCase` - Corpus management for fuzz test cases
- `Mutator`, `MutationStrategy` - Mutation strategy implementations
- `Xorshift64` - Deterministic PRNG for reproducible fuzzing

## Design Principles

- **No external dependencies**: Uses a fixed-seed xorshift64 PRNG to ensure reproducibility without depending on the `rand` crate (spec §40)
- **Deterministic**: All fuzzing is driven by deterministic sequences, enabling reliable reproduction of any failure (spec §1)
- **Safe**: All fuzzing operations are safe Rust; the `forbid(unsafe_code)` attribute prevents unsafe code
- **Extensible**: The `FuzzTarget` trait allows fuzzing of additional targets beyond the decoder
- **Property-based**: Generates diverse test cases including malformed, truncated, and mutated inputs

## Key Components

### PRNG (Xorshift64)

A deterministic pseudo-random number generator using the xorshift64 algorithm. Ensures fully reproducible fuzzing campaigns without external dependencies.

```rust
use jxcl_fuzz::Xorshift64;

let mut rng = Xorshift64::new(0x1234567890ABCDEF);
let random_value = rng.next_u64();
let random_byte = rng.next_u8();
```

### Fuzzing Harness

The generic `FuzzCampaign` driver runs a `FuzzTarget` against generated test cases:

```rust
use jxcl_fuzz::{FuzzTarget, FuzzCampaign};

let mut campaign = FuzzCampaign::new(&mut my_target);
let stats = campaign.run(20_000);
```

### Corpus Management

Track test cases with metadata (execution counts, input size):

```rust
use jxcl_fuzz::{Corpus, TestCase};

let mut corpus = Corpus::new();
corpus.add(TestCase::new(vec![0x42, 0x43]));
```

### Mutation Strategies

Six mutation strategies for generating diverse test cases:

- **Flip**: Flip individual bits
- **Havoc**: Apply multiple random bit flips
- **ByteSwap**: Exchange bytes at two positions
- **Insert**: Insert random bytes
- **Delete**: Delete bytes at a position
- **Overwrite**: Replace a section with random bytes

```rust
use jxcl_fuzz::{Mutator, MutationStrategy, Xorshift64};

let mut rng = Xorshift64::new(0xDEADBEEF);
let mut mutator = Mutator::new(&mut rng);
let mutated = mutator.mutate(&test_case, MutationStrategy::Flip);
```

### Decoder Fuzzing

The main `fuzz_decode_target` function ensures the decoder never panics on malformed input:

```rust
use jxcl_fuzz::fuzz_decode_target;

let stats = fuzz_decode_target(20_000);
assert_eq!(stats.anomalies, 0, "Decoder should never panic");
println!("Coverage: {:.1}%", stats.coverage_percent());
```

## Testing

The crate includes comprehensive tests covering:

- PRNG determinism and distribution
- Corpus and test case management
- All mutation strategies on edge cases (empty input, single byte, large buffers)
- Decoder safety against random byte streams
- Truncated instruction handling
- Fuzzing campaign statistics and coverage metrics

Run tests with:

```bash
cargo test -p jxcl-fuzz
```

## Dependencies

Workspace crates:

- `jxcl-decoding` - Instruction decoder
- `jxcl-instructions` - Instruction types
- `jxcl-errors` - Structured error types
- `jxcl-simulator` - Reference simulator
- `jxcl-determinism` - Determinism framework

External crates:

*(none)* — No external dependencies (spec §40)

## Status

Implemented in Batch D (debug/simulation category) with full property-based fuzzing support and extensible harness design.

## License

AGPLv3 / Commercial dual-license. See `LICENSE-AGPL`, `LICENSE-COMMERCIAL`, and `LICENSE-NOTICE` at the repository root.
