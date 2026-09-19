//! Property tests (spec §32): invariants that must hold for *every*
//! instruction in the registry, not just a hand-picked few.
//!
//! - `encode(decode(bytes)) == bytes` and `decode(encode(instr)) == instr`
//!   for every opcode, across several pseudo-random operand fillings.
//! - `snapshot(restore(state)) == state` for `MachineState`.
//! - `write(read(addr)) == value` for every memory access width.

mod common;

use common::Xorshift64;
use jxcl::encoding::decoder::decode_one;
use jxcl::encoding::encoder::encode;
use jxcl::isa::constants::NUM_GP_REGISTERS;
use jxcl::isa::instruction::DecodedInstruction;
use jxcl::isa::opcodes::all_defs;
use jxcl::isa::operand::{Format, Operands};

fn sample_reg(rng: &mut Xorshift64) -> u8 {
    (rng.next_u64() % NUM_GP_REGISTERS as u64) as u8
}

fn sample_operands(format: Format, rng: &mut Xorshift64) -> Operands {
    match format {
        Format::None => Operands::None,
        Format::R => Operands::R { rd: sample_reg(rng) },
        Format::RR => Operands::RR { rd: sample_reg(rng), rs: sample_reg(rng) },
        Format::RRR => Operands::RRR { rd: sample_reg(rng), rs1: sample_reg(rng), rs2: sample_reg(rng) },
        Format::RImm64 => Operands::RImm64 { rd: sample_reg(rng), imm: rng.next_u64() },
        Format::RMem => {
            Operands::RMem { rd: sample_reg(rng), base: sample_reg(rng), disp: rng.next_u64() as i32 }
        }
        Format::MemR => {
            Operands::MemR { base: sample_reg(rng), disp: rng.next_u64() as i32, rs: sample_reg(rng) }
        }
        Format::Cas => Operands::Cas {
            rd: sample_reg(rng),
            base: sample_reg(rng),
            rs_new: sample_reg(rng),
            disp: rng.next_u64() as i32,
        },
        Format::BranchImm32 => Operands::BranchImm32 { disp: rng.next_u64() as i32 },
        Format::Imm16 => Operands::Imm16 { imm: (rng.next_u64() & 0xFFFF) as u16 },
    }
}

#[test]
fn every_opcode_round_trips_encode_decode() {
    let mut rng = Xorshift64::new(0x1234_5678_9ABC_DEF0);
    const TRIALS_PER_OPCODE: usize = 8;

    for def in all_defs() {
        for _ in 0..TRIALS_PER_OPCODE {
            let operands = sample_operands(def.format, &mut rng);
            let instr = DecodedInstruction::new(def.mnemonic, operands);

            let mut bytes = Vec::new();
            encode(&instr, &mut bytes).unwrap_or_else(|e| {
                panic!("encode failed for {}: {}", def.mnemonic.text(), e)
            });
            assert_eq!(bytes.len(), def.format.len(), "encoded length mismatch for {}", def.mnemonic.text());
            assert_eq!(bytes[0], def.opcode);

            let decoded = decode_one(&bytes, 0)
                .unwrap_or_else(|e| panic!("decode failed for {}: {}", def.mnemonic.text(), e));
            assert_eq!(decoded, instr, "decode(encode(x)) != x for {}", def.mnemonic.text());

            let mut re_encoded = Vec::new();
            encode(&decoded, &mut re_encoded).unwrap();
            assert_eq!(re_encoded, bytes, "encode(decode(bytes)) != bytes for {}", def.mnemonic.text());
        }
    }
}

#[test]
fn memory_write_read_roundtrip_every_width() {
    use jxcl::memory::Memory;
    let mut rng = Xorshift64::new(42);
    let mut mem = Memory::new(4096);

    for _ in 0..200 {
        let addr8 = rng.next_u64() % 4096;
        let v8 = rng.next_u8();
        mem.write8(addr8, v8).unwrap();
        assert_eq!(mem.read8(addr8).unwrap(), v8);

        let addr16 = (rng.next_u64() % 2048) * 2;
        let v16 = (rng.next_u64() & 0xFFFF) as u16;
        mem.write16(addr16, v16).unwrap();
        assert_eq!(mem.read16(addr16).unwrap(), v16);

        let addr32 = (rng.next_u64() % 1024) * 4;
        let v32 = (rng.next_u64() & 0xFFFF_FFFF) as u32;
        mem.write32(addr32, v32).unwrap();
        assert_eq!(mem.read32(addr32).unwrap(), v32);

        let addr64 = (rng.next_u64() % 512) * 8;
        let v64 = rng.next_u64();
        mem.write64(addr64, v64).unwrap();
        assert_eq!(mem.read64(addr64).unwrap(), v64);
    }
}

#[test]
fn machine_snapshot_restore_is_lossless_under_random_mutation() {
    use jxcl::machine::MachineState;
    use jxcl::memory::Memory;

    let mut rng = Xorshift64::new(7);
    let mut state = MachineState::new(Memory::new(256));
    for r in 1..NUM_GP_REGISTERS as u8 {
        state.registers.write(r, rng.next_u64()).unwrap();
    }
    state.registers.pc = rng.next_u64() % 256;
    state.registers.flags = rng.next_u64() & 0b1111;
    state.cycle_count = rng.next_u64();

    let snap = state.snapshot();
    let mut restored = MachineState::new(Memory::new(256));
    restored.restore(snap.clone());
    assert_eq!(restored.snapshot(), snap);
}

#[test]
fn add_sub_are_inverse_on_random_operands() {
    use jxcl::alu;
    let mut rng = Xorshift64::new(99);
    for _ in 0..500 {
        let a = rng.next_u64();
        let b = rng.next_u64();
        let sum = alu::add(a, b).value;
        let back = alu::sub(sum, b).value;
        assert_eq!(back, a, "SUB(ADD(a,b), b) != a for a={:#x}, b={:#x}", a, b);
    }
}
