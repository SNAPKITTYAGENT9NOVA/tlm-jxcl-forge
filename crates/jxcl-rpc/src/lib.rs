//! A minimal RPC server/client implementing jxcl-protocol over line-delimited JSON on TCP.
//!
//! Owns: RpcServer/RpcClient.
//!
//! Status: scaffolded -- real implementation lands per
//! `docs/CRATE_GENERATION_PLAN.md`'s batch schedule for the
//! `network` category. Planned public API: RpcServer, RpcClient.
#![forbid(unsafe_code)]
#![allow(dead_code)]

#[cfg(test)]
mod tests {
    #[test]
    fn scaffold_placeholder() {
        // Real tests land with this crate's implementation batch --
        // see docs/crates.toml's `tests` field for what's planned:
        // unit, integration.
    }
}
