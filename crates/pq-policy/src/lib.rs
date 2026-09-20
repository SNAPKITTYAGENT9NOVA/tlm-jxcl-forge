// Copyright (C) 2026 ERRANT DIGITAL INSTITUTE OF TECHNOLOGY (SEIT NGO) % SNAPKITTY COLLECTIVE
// SPDX-License-Identifier: AGPL-3.0-only OR LicenseRef-Commercial
// See LICENSE-AGPL and LICENSE-COMMERCIAL in this repository for license terms.

//! Consolidates scattered policy decisions (TLS-required-in-production, minimum seed/key length) into one crate with a real Policy trait, replacing duplicated ad hoc checks in photo-cache-service and pq-crypto.
//!
//! Owns: The Policy trait and the concrete TlsRequiredInProduction/MinimumSeedLength policies.
//!
//! Status: scaffolded -- real implementation lands per
//! `docs/CRATE_GENERATION_PLAN.md`'s batch schedule for the
//! `crypto` category. Planned public API: Policy, TlsRequiredInProduction, MinimumSeedLength.
#![forbid(unsafe_code)]
#![allow(dead_code)]

#[cfg(test)]
mod tests {
    #[test]
    fn scaffold_placeholder() {
        // Real tests land with this crate's implementation batch --
        // see docs/crates.toml's `tests` field for what's planned:
        // unit, boundary.
    }
}
