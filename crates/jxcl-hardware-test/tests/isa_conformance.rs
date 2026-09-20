// Copyright (C) 2026 ERRANT DIGITAL INSTITUTE OF TECHNOLOGY (SEIT NGO) % SNAPKITTY COLLECTIVE
// SPDX-License-Identifier: AGPL-3.0-only OR LicenseRef-Commercial
// See LICENSE-AGPL and LICENSE-COMMERCIAL in this repository for license terms.

//! The mechanical width/opcode-count cross-check between
//! `jxcl-hardware`'s generated decoder and the real ISA source of
//! truth (`jxcl-opcodes`/`jxcl-constants`).
//!
//! # `jxcl-isa-schema` status
//!
//! `docs/crates.toml` calls for this cross-check to go through
//! `jxcl-isa-schema`. That crate is still a placeholder (no
//! `IsaSchema` type) as of this writing, so this test cross-checks
//! directly against `jxcl-opcodes::all_defs()` and
//! `jxcl-constants`'s real constants instead -- the same real source of
//! truth `jxcl-isa-schema` would itself be generated from. Once
//! `jxcl-isa-schema` lands, this test's assertions should be rewritten
//! against `jxcl_isa_schema::IsaSchema::generate()` in place of the
//! direct `jxcl-opcodes`/`jxcl-constants` calls below, without changing
//! what they actually assert (the numbers are the same either way,
//! since a real `IsaSchema` would itself be generated from these same
//! two crates).

use jxcl_hdl::{CaseValue, Statement};
use std::collections::BTreeSet;

fn decoder_arms() -> Vec<(CaseValue, Vec<Statement>)> {
    let module = jxcl_rtl::build_decoder_module();
    match module
        .top_level_case()
        .expect("decoder module must have a top-level case statement")
    {
        Statement::Case { arms, .. } => arms.clone(),
        _ => unreachable!(),
    }
}

/// The headline mechanical proof this crate exists to carry (per
/// `docs/RTL_CONTRACT.md` and rule #14): the number of real, non-default
/// case arms in the generated decoder equals the number of real entries
/// in `jxcl_opcodes::all_defs()`, one for one, by opcode id -- checked
/// again here at the `jxcl-hardware-test` level (on top of `jxcl-rtl`'s
/// own equivalent test), so a regression in either crate is caught by
/// at least one of the two.
#[test]
fn decoder_opcode_arm_count_matches_jxcl_opcodes_table() {
    let arms = decoder_arms();
    let generated_ids: BTreeSet<u64> = arms
        .iter()
        .filter_map(|(v, _)| match v {
            CaseValue::Literal { value, .. } => Some(*value),
            CaseValue::Default => None,
        })
        .collect();
    let real_ids: BTreeSet<u64> = jxcl_opcodes::all_defs()
        .iter()
        .map(|d| d.opcode as u64)
        .collect();

    assert_eq!(generated_ids.len(), jxcl_opcodes::all_defs().len());
    assert_eq!(generated_ids, real_ids);
}

/// The decoder's `opcode` input port width must equal
/// `jxcl_constants::OPCODE_BITS` exactly -- if the ISA's opcode field
/// ever widened or narrowed without regenerating the RTL, this fails.
#[test]
fn decoder_opcode_field_width_matches_jxcl_constants() {
    let module = jxcl_rtl::build_decoder_module();
    let opcode_port = module
        .ports
        .iter()
        .find(|p| p.name == "opcode")
        .expect("decoder has an opcode port");
    assert_eq!(opcode_port.width, jxcl_constants::OPCODE_BITS);
}

/// Every opcode actually fits in the opcode space `jxcl_constants`
/// declares -- a sanity bound shared with `jxcl-opcodes`'s own test of
/// the same fact, re-checked here at the RTL-generation boundary.
#[test]
fn every_real_opcode_fits_the_declared_opcode_space() {
    for def in jxcl_opcodes::all_defs() {
        assert!((def.opcode as usize) < jxcl_constants::OPCODE_SPACE);
    }
}

/// The register file's address port width must be wide enough to name
/// every real register in `jxcl_constants::REGISTER_COUNT`, and its data
/// port widths must equal `jxcl_constants::WORD_BITS` exactly.
#[test]
fn register_file_widths_match_jxcl_constants() {
    let module = jxcl_rtl::build_register_file_module();
    let addr_port = module
        .ports
        .iter()
        .find(|p| p.name == "read_addr1")
        .expect("register file has a read_addr1 port");
    let addressable = 1usize << addr_port.width;
    assert!(
        addressable >= jxcl_constants::REGISTER_COUNT,
        "register file address port ({} bits, addresses {} entries) can't name all {} real registers",
        addr_port.width,
        addressable,
        jxcl_constants::REGISTER_COUNT
    );

    let data_port = module
        .ports
        .iter()
        .find(|p| p.name == "write_data")
        .expect("register file has a write_data port");
    assert_eq!(data_port.width, jxcl_constants::WORD_BITS);
}
