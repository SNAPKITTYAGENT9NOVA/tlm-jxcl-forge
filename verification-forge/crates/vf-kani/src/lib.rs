// Copyright (C) 2026 ERRANT DIGITAL INSTITUTE OF TECHNOLOGY (SEIT NGO) % SNAPKITTY COLLECTIVE
// SPDX-License-Identifier: AGPL-3.0-only OR LicenseRef-Commercial
// See LICENSE-AGPL and LICENSE-COMMERCIAL in this repository for license terms.

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
