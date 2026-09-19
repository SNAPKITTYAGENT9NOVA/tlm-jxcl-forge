//! Kani backend: shells out to real cargo-kani as an untrusted oracle, never claims KERNEL_VERIFIED from a bounded result.
//!
//! Status: scaffolded -- real implementation lands per
//! verification-forge's specified implementation order.
#![forbid(unsafe_code)]

#[cfg(test)]
mod tests {
    #[test]
    fn scaffold_placeholder() {}
}
