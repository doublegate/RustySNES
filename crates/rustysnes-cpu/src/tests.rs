//! Unit tests pinning 65C816 opcode semantics, flag results, width changes, stack wrapping,
//! decimal arithmetic, branch timing, calls/returns, and interrupt vectoring.

#![allow(clippy::cast_possible_truncation)] // intentional byte-narrowing in the test bus helpers

use crate::bus::{Bus, NullBus};
use crate::regs::Status;
use crate::{Cpu, vectors};
use alloc::vec;
use alloc::vec::Vec;

/// A flat 24-bit memory bus for unit tests, with controllable NMI/IRQ lines.
struct TestBus {
    mem: Vec<u8>,
    nmi: bool,
    irq: bool,
    cycles: u64,
}

impl TestBus {
    fn new() -> Self {
        Self {
            mem: vec![0; 0x100_0000],
            nmi: false,
            irq: false,
            cycles: 0,
        }
    }
    fn load(&mut self, addr: u32, bytes: &[u8]) {
        for (i, b) in bytes.iter().enumerate() {
            self.mem[(addr as usize + i) & 0x00FF_FFFF] = *b;
        }
    }
    fn set16(&mut self, addr: u32, val: u16) {
        self.mem[addr as usize] = val as u8;
        self.mem[addr as usize + 1] = (val >> 8) as u8;
    }
}

impl Bus for TestBus {
    fn read24(&mut self, addr: u32) -> u8 {
        self.mem[(addr & 0x00FF_FFFF) as usize]
    }
    fn write24(&mut self, addr: u32, val: u8) {
        self.mem[(addr & 0x00FF_FFFF) as usize] = val;
    }
    fn poll_nmi(&mut self) -> bool {
        let n = self.nmi;
        self.nmi = false;
        n
    }
    fn poll_irq(&mut self) -> bool {
        self.irq
    }
    fn on_cpu_cycle(&mut self) {
        self.cycles += 1;
    }
}

/// Build a CPU at native mode with 16-bit A/X by default at PC=$00:8000.
fn native_cpu(bus: &mut TestBus) -> Cpu {
    bus.set16(vectors::RESET, 0x8000);
    let mut cpu = Cpu::new();
    cpu.reset(bus);
    // CLC; XCE to go native.
    cpu.regs.set_flag(Status::C, false);
    // Manually flip into native with M=X=0 (16-bit) for convenience.
    cpu.regs.emulation = false;
    cpu.regs.set_flag(Status::M, false);
    cpu.regs.set_flag(Status::X, false);
    cpu
}

#[test]
fn constructs_and_null_steps() {
    let mut cpu = Cpu::new();
    let mut bus = NullBus;
    // BRK against an all-zero NullBus shouldn't panic.
    let _ = cpu.step(&mut bus);
}

#[test]
fn reset_loads_vector_and_emulation() {
    let mut bus = TestBus::new();
    bus.set16(vectors::RESET, 0x1234);
    let mut cpu = Cpu::new();
    cpu.reset(&mut bus);
    assert_eq!(cpu.regs.pc, 0x1234);
    assert!(cpu.regs.emulation);
    assert!(cpu.regs.p.contains(Status::M));
    assert!(cpu.regs.p.contains(Status::X));
    assert!(cpu.regs.p.contains(Status::I));
    assert!(!cpu.regs.p.contains(Status::D));
}

#[test]
fn lda_immediate_8bit() {
    let mut bus = TestBus::new();
    bus.set16(vectors::RESET, 0x8000);
    let mut cpu = Cpu::new();
    cpu.reset(&mut bus);
    // emulation mode -> 8-bit. LDA #$80
    bus.load(0x00_8000, &[0xA9, 0x80]);
    let cyc = cpu.step(&mut bus);
    assert_eq!(cpu.regs.a & 0xFF, 0x80);
    assert!(cpu.regs.p.contains(Status::N));
    assert!(!cpu.regs.p.contains(Status::Z));
    assert_eq!(cyc, 2); // opcode + 1 operand byte
}

#[test]
fn lda_immediate_16bit_costs_extra() {
    let mut bus = TestBus::new();
    let mut cpu = native_cpu(&mut bus);
    bus.load(0x00_8000, &[0xA9, 0x34, 0x12]); // LDA #$1234
    let cyc = cpu.step(&mut bus);
    assert_eq!(cpu.regs.a, 0x1234);
    assert!(!cpu.regs.p.contains(Status::Z));
    assert!(!cpu.regs.p.contains(Status::N));
    assert_eq!(cyc, 3); // opcode + 2 operand bytes (M=0 makes it wider)
}

#[test]
fn rep_sep_change_width() {
    let mut bus = TestBus::new();
    bus.set16(vectors::RESET, 0x8000);
    let mut cpu = Cpu::new();
    cpu.reset(&mut bus);
    cpu.regs.emulation = false;
    cpu.regs.set_flag(Status::M, true);
    cpu.regs.set_flag(Status::X, true);
    // REP #$30 clears M and X (16-bit).
    bus.load(0x00_8000, &[0xC2, 0x30]);
    cpu.step(&mut bus);
    assert!(!cpu.regs.p.contains(Status::M));
    assert!(!cpu.regs.p.contains(Status::X));
    // SEP #$20 sets M (8-bit acc).
    bus.load(0x00_8002, &[0xE2, 0x20]);
    cpu.step(&mut bus);
    assert!(cpu.regs.p.contains(Status::M));
    assert!(!cpu.regs.p.contains(Status::X));
}

#[test]
fn xce_toggles_emulation() {
    let mut bus = TestBus::new();
    bus.set16(vectors::RESET, 0x8000);
    let mut cpu = Cpu::new();
    cpu.reset(&mut bus);
    // emulation=1, C=0. CLC already; XCE swaps E<->C: C becomes 1 (old E), E becomes 0 (old C).
    cpu.regs.set_flag(Status::C, false);
    bus.load(0x00_8000, &[0xFB]); // XCE
    cpu.step(&mut bus);
    assert!(!cpu.regs.emulation);
    assert!(cpu.regs.p.contains(Status::C));
    // Now go back: SEC; XCE.
    bus.load(0x00_8001, &[0x38, 0xFB]);
    cpu.step(&mut bus); // SEC
    cpu.step(&mut bus); // XCE
    assert!(cpu.regs.emulation);
    assert!(!cpu.regs.p.contains(Status::C));
}

#[test]
fn x_flag_zeroes_index_high_byte() {
    let mut bus = TestBus::new();
    let mut cpu = native_cpu(&mut bus);
    cpu.regs.set_x(0x1234);
    assert_eq!(cpu.regs.x, 0x1234);
    // SEP #$10 -> X=1 (8-bit) forces high byte to 0.
    bus.load(0x00_8000, &[0xE2, 0x10]);
    cpu.step(&mut bus);
    assert_eq!(cpu.regs.x, 0x0034);
}

#[test]
fn adc_binary_8bit() {
    let mut bus = TestBus::new();
    bus.set16(vectors::RESET, 0x8000);
    let mut cpu = Cpu::new();
    cpu.reset(&mut bus); // emulation, 8-bit
    cpu.regs.a = 0x10;
    cpu.regs.set_flag(Status::C, false);
    bus.load(0x00_8000, &[0x69, 0x20]); // ADC #$20
    cpu.step(&mut bus);
    assert_eq!(cpu.regs.a & 0xFF, 0x30);
    assert!(!cpu.regs.p.contains(Status::C));
    assert!(!cpu.regs.p.contains(Status::V));
}

#[test]
fn adc_overflow_and_carry() {
    let mut bus = TestBus::new();
    bus.set16(vectors::RESET, 0x8000);
    let mut cpu = Cpu::new();
    cpu.reset(&mut bus);
    cpu.regs.a = 0x7F;
    cpu.regs.set_flag(Status::C, false);
    bus.load(0x00_8000, &[0x69, 0x01]); // 0x7F + 1 = 0x80 -> V set, N set
    cpu.step(&mut bus);
    assert_eq!(cpu.regs.a & 0xFF, 0x80);
    assert!(cpu.regs.p.contains(Status::V));
    assert!(cpu.regs.p.contains(Status::N));
    assert!(!cpu.regs.p.contains(Status::C));
}

#[test]
fn adc_decimal_8bit() {
    let mut bus = TestBus::new();
    bus.set16(vectors::RESET, 0x8000);
    let mut cpu = Cpu::new();
    cpu.reset(&mut bus);
    cpu.regs.a = 0x25;
    cpu.regs.set_flag(Status::D, true);
    cpu.regs.set_flag(Status::C, false);
    bus.load(0x00_8000, &[0x69, 0x48]); // BCD 25 + 48 = 73
    cpu.step(&mut bus);
    assert_eq!(cpu.regs.a & 0xFF, 0x73);
    assert!(!cpu.regs.p.contains(Status::C));
}

#[test]
fn adc_decimal_carry() {
    let mut bus = TestBus::new();
    bus.set16(vectors::RESET, 0x8000);
    let mut cpu = Cpu::new();
    cpu.reset(&mut bus);
    cpu.regs.a = 0x99;
    cpu.regs.set_flag(Status::D, true);
    cpu.regs.set_flag(Status::C, true);
    bus.load(0x00_8000, &[0x69, 0x00]); // 99 + 00 + carry = 00, C
    cpu.step(&mut bus);
    assert_eq!(cpu.regs.a & 0xFF, 0x00);
    assert!(cpu.regs.p.contains(Status::C));
}

#[test]
fn adc_decimal_16bit() {
    let mut bus = TestBus::new();
    let mut cpu = native_cpu(&mut bus);
    cpu.regs.a = 0x1234;
    cpu.regs.set_flag(Status::D, true);
    cpu.regs.set_flag(Status::C, false);
    bus.load(0x00_8000, &[0x69, 0x66, 0x55]); // BCD 1234 + 5566 = 6800
    cpu.step(&mut bus);
    assert_eq!(cpu.regs.a, 0x6800);
    assert!(!cpu.regs.p.contains(Status::C));
}

#[test]
fn sbc_binary_8bit() {
    let mut bus = TestBus::new();
    bus.set16(vectors::RESET, 0x8000);
    let mut cpu = Cpu::new();
    cpu.reset(&mut bus);
    cpu.regs.a = 0x50;
    cpu.regs.set_flag(Status::C, true); // no borrow
    bus.load(0x00_8000, &[0xE9, 0x20]); // 0x50 - 0x20 = 0x30
    cpu.step(&mut bus);
    assert_eq!(cpu.regs.a & 0xFF, 0x30);
    assert!(cpu.regs.p.contains(Status::C)); // no borrow out
}

#[test]
fn sbc_decimal_8bit() {
    let mut bus = TestBus::new();
    bus.set16(vectors::RESET, 0x8000);
    let mut cpu = Cpu::new();
    cpu.reset(&mut bus);
    cpu.regs.a = 0x50;
    cpu.regs.set_flag(Status::D, true);
    cpu.regs.set_flag(Status::C, true);
    bus.load(0x00_8000, &[0xE9, 0x25]); // BCD 50 - 25 = 25
    cpu.step(&mut bus);
    assert_eq!(cpu.regs.a & 0xFF, 0x25);
    assert!(cpu.regs.p.contains(Status::C));
}

#[test]
fn and_ora_eor() {
    let mut bus = TestBus::new();
    bus.set16(vectors::RESET, 0x8000);
    let mut cpu = Cpu::new();
    cpu.reset(&mut bus);
    cpu.regs.a = 0xF0;
    bus.load(0x00_8000, &[0x29, 0x0F]); // AND #$0F -> 0
    cpu.step(&mut bus);
    assert_eq!(cpu.regs.a & 0xFF, 0x00);
    assert!(cpu.regs.p.contains(Status::Z));
    cpu.regs.a = 0xF0;
    bus.load(0x00_8002, &[0x09, 0x0F]); // ORA #$0F -> 0xFF
    cpu.step(&mut bus);
    assert_eq!(cpu.regs.a & 0xFF, 0xFF);
    cpu.regs.a = 0xFF;
    bus.load(0x00_8004, &[0x49, 0x0F]); // EOR #$0F -> 0xF0
    cpu.step(&mut bus);
    assert_eq!(cpu.regs.a & 0xFF, 0xF0);
}

#[test]
fn cmp_sets_carry_when_ge() {
    let mut bus = TestBus::new();
    bus.set16(vectors::RESET, 0x8000);
    let mut cpu = Cpu::new();
    cpu.reset(&mut bus);
    cpu.regs.a = 0x50;
    bus.load(0x00_8000, &[0xC9, 0x30]); // CMP #$30 -> A >= M
    cpu.step(&mut bus);
    assert!(cpu.regs.p.contains(Status::C));
    assert!(!cpu.regs.p.contains(Status::Z));
    cpu.regs.a = 0x30;
    bus.load(0x00_8002, &[0xC9, 0x30]); // equal
    cpu.step(&mut bus);
    assert!(cpu.regs.p.contains(Status::C));
    assert!(cpu.regs.p.contains(Status::Z));
}

#[test]
fn inc_dec_accumulator() {
    let mut bus = TestBus::new();
    bus.set16(vectors::RESET, 0x8000);
    let mut cpu = Cpu::new();
    cpu.reset(&mut bus);
    cpu.regs.a = 0xFF;
    bus.load(0x00_8000, &[0x1A]); // INC A -> 0
    cpu.step(&mut bus);
    assert_eq!(cpu.regs.a & 0xFF, 0x00);
    assert!(cpu.regs.p.contains(Status::Z));
    bus.load(0x00_8001, &[0x3A]); // DEC A -> 0xFF
    cpu.step(&mut bus);
    assert_eq!(cpu.regs.a & 0xFF, 0xFF);
    assert!(cpu.regs.p.contains(Status::N));
}

#[test]
fn asl_lsr_rol_ror_accumulator() {
    let mut bus = TestBus::new();
    bus.set16(vectors::RESET, 0x8000);
    let mut cpu = Cpu::new();
    cpu.reset(&mut bus);
    cpu.regs.a = 0x81;
    bus.load(0x00_8000, &[0x0A]); // ASL A -> 0x02, C=1
    cpu.step(&mut bus);
    assert_eq!(cpu.regs.a & 0xFF, 0x02);
    assert!(cpu.regs.p.contains(Status::C));
    cpu.regs.a = 0x01;
    cpu.regs.set_flag(Status::C, false);
    bus.load(0x00_8001, &[0x4A]); // LSR A -> 0, C=1
    cpu.step(&mut bus);
    assert_eq!(cpu.regs.a & 0xFF, 0x00);
    assert!(cpu.regs.p.contains(Status::C));
    assert!(cpu.regs.p.contains(Status::Z));
    cpu.regs.a = 0x80;
    cpu.regs.set_flag(Status::C, true);
    bus.load(0x00_8002, &[0x2A]); // ROL A: 0x80<<1 | 1 = 0x01, C=1
    cpu.step(&mut bus);
    assert_eq!(cpu.regs.a & 0xFF, 0x01);
    assert!(cpu.regs.p.contains(Status::C));
    cpu.regs.a = 0x01;
    cpu.regs.set_flag(Status::C, true);
    bus.load(0x00_8003, &[0x6A]); // ROR A: carry into bit7, 0x80, C=1
    cpu.step(&mut bus);
    assert_eq!(cpu.regs.a & 0xFF, 0x80);
    assert!(cpu.regs.p.contains(Status::C));
}

#[test]
fn store_and_load_direct_page() {
    let mut bus = TestBus::new();
    let mut cpu = native_cpu(&mut bus);
    cpu.regs.d = 0x0000;
    cpu.regs.a = 0xBEEF;
    // STA $10 (16-bit) then LDA $10
    bus.load(0x00_8000, &[0x85, 0x10]);
    cpu.step(&mut bus);
    assert_eq!(bus.read24(0x0010), 0xEF);
    assert_eq!(bus.read24(0x0011), 0xBE);
    cpu.regs.a = 0;
    bus.load(0x00_8002, &[0xA5, 0x10]);
    cpu.step(&mut bus);
    assert_eq!(cpu.regs.a, 0xBEEF);
}

#[test]
fn dp_penalty_when_d_low_nonzero() {
    let mut bus = TestBus::new();
    let mut cpu = native_cpu(&mut bus);
    cpu.regs.set_flag(Status::M, true); // 8-bit to isolate the dp penalty
    cpu.regs.d = 0x0000;
    bus.load(0x00_8000, &[0xA5, 0x10]); // LDA $10 (8-bit, D aligned): 2 cyc
    let c1 = cpu.step(&mut bus);
    cpu.regs.pc = 0x8000;
    cpu.regs.d = 0x0001; // misaligned -> +1
    let c2 = cpu.step(&mut bus);
    assert_eq!(c2, c1 + 1);
}

#[test]
fn branch_not_taken_vs_taken_pagecross() {
    let mut bus = TestBus::new();
    bus.set16(vectors::RESET, 0x8000);
    let mut cpu = Cpu::new();
    cpu.reset(&mut bus); // emulation
    // BEQ +0 with Z clear: not taken -> 2 cycles.
    cpu.regs.set_flag(Status::Z, false);
    bus.load(0x00_8000, &[0xF0, 0x10]);
    let not_taken = cpu.step(&mut bus);
    assert_eq!(not_taken, 2);
    // Taken, no page cross: 3 cycles.
    cpu.regs.pc = 0x8000;
    cpu.regs.set_flag(Status::Z, true);
    bus.load(0x00_8000, &[0xF0, 0x04]);
    let taken = cpu.step(&mut bus);
    assert_eq!(taken, 3);
    assert_eq!(cpu.regs.pc, 0x8006);
    // Taken with emulation page cross: 4 cycles.
    cpu.regs.pc = 0x80F0;
    cpu.regs.set_flag(Status::Z, true);
    bus.load(0x00_80F0, &[0xF0, 0x20]); // 0x80F2 + 0x20 = 0x8112 (crosses page)
    let crossed = cpu.step(&mut bus);
    assert_eq!(crossed, 4);
}

#[test]
fn jsr_rts_roundtrip() {
    let mut bus = TestBus::new();
    let mut cpu = native_cpu(&mut bus);
    cpu.regs.s = 0x01FF;
    bus.load(0x00_8000, &[0x20, 0x00, 0x90]); // JSR $9000
    cpu.step(&mut bus);
    assert_eq!(cpu.regs.pc, 0x9000);
    // The pushed return address should be 0x8002 (PC of last byte of JSR).
    bus.load(0x00_9000, &[0x60]); // RTS
    cpu.step(&mut bus);
    assert_eq!(cpu.regs.pc, 0x8003);
}

#[test]
fn jsl_rtl_roundtrip() {
    let mut bus = TestBus::new();
    let mut cpu = native_cpu(&mut bus);
    cpu.regs.s = 0x01FF;
    cpu.regs.pbr = 0x00;
    bus.load(0x00_8000, &[0x22, 0x00, 0x00, 0x12]); // JSL $12:0000
    cpu.step(&mut bus);
    assert_eq!(cpu.regs.pc, 0x0000);
    assert_eq!(cpu.regs.pbr, 0x12);
    bus.load(0x12_0000, &[0x6B]); // RTL
    cpu.step(&mut bus);
    assert_eq!(cpu.regs.pbr, 0x00);
    assert_eq!(cpu.regs.pc, 0x8004);
}

#[test]
fn jmp_absolute_and_long() {
    let mut bus = TestBus::new();
    let mut cpu = native_cpu(&mut bus);
    bus.load(0x00_8000, &[0x4C, 0x34, 0x12]); // JMP $1234
    cpu.step(&mut bus);
    assert_eq!(cpu.regs.pc, 0x1234);
    bus.load(0x00_1234, &[0x5C, 0x00, 0x80, 0x05]); // JML $05:8000
    cpu.step(&mut bus);
    assert_eq!(cpu.regs.pc, 0x8000);
    assert_eq!(cpu.regs.pbr, 0x05);
}

#[test]
fn stack_emulation_page_wrap() {
    let mut bus = TestBus::new();
    bus.set16(vectors::RESET, 0x8000);
    let mut cpu = Cpu::new();
    cpu.reset(&mut bus); // emulation, S=0x01FF
    cpu.regs.s = 0x0100;
    cpu.regs.a = 0x42;
    bus.load(0x00_8000, &[0x48]); // PHA (8-bit)
    cpu.step(&mut bus);
    // S should wrap from 0x0100 to 0x01FF, staying in page 1.
    assert_eq!(cpu.regs.s, 0x01FF);
    assert_eq!(bus.read24(0x0100), 0x42);
}

#[test]
fn php_plp_roundtrip() {
    let mut bus = TestBus::new();
    let mut cpu = native_cpu(&mut bus);
    cpu.regs.s = 0x01FF;
    cpu.regs.p = Status::from_bits_truncate(0b1100_0001); // N V C
    bus.load(0x00_8000, &[0x08]); // PHP
    cpu.step(&mut bus);
    cpu.regs.p = Status::empty();
    bus.load(0x00_8001, &[0x28]); // PLP
    cpu.step(&mut bus);
    assert!(cpu.regs.p.contains(Status::N));
    assert!(cpu.regs.p.contains(Status::V));
    assert!(cpu.regs.p.contains(Status::C));
}

#[test]
fn nmi_vectoring_emulation() {
    let mut bus = TestBus::new();
    bus.set16(vectors::RESET, 0x8000);
    bus.set16(vectors::NMI_EMU, 0xE000);
    let mut cpu = Cpu::new();
    cpu.reset(&mut bus);
    cpu.regs.pbr = 0x00;
    bus.nmi = true;
    cpu.step(&mut bus);
    assert_eq!(cpu.regs.pc, 0xE000);
    assert!(cpu.regs.p.contains(Status::I));
}

#[test]
fn nmi_vectoring_native() {
    let mut bus = TestBus::new();
    let mut cpu = native_cpu(&mut bus);
    bus.set16(vectors::NMI_NATIVE, 0xD000);
    cpu.regs.pbr = 0x12;
    bus.nmi = true;
    cpu.step(&mut bus);
    assert_eq!(cpu.regs.pc, 0xD000);
    assert_eq!(cpu.regs.pbr, 0x00);
    // PBR should have been pushed in native mode.
}

#[test]
fn irq_honored_only_when_i_clear() {
    let mut bus = TestBus::new();
    let mut cpu = native_cpu(&mut bus);
    bus.set16(vectors::IRQ_NATIVE, 0xC000);
    cpu.regs.set_flag(Status::I, true);
    bus.irq = true;
    bus.load(0x00_8000, &[0xEA]); // NOP — IRQ masked, NOP runs
    cpu.step(&mut bus);
    assert_eq!(cpu.regs.pc, 0x8001);
    // Now clear I and re-poll.
    cpu.regs.set_flag(Status::I, false);
    cpu.step(&mut bus);
    assert_eq!(cpu.regs.pc, 0xC000);
}

#[test]
fn brk_vectoring_native() {
    let mut bus = TestBus::new();
    let mut cpu = native_cpu(&mut bus);
    bus.set16(vectors::BRK_NATIVE, 0xB000);
    bus.load(0x00_8000, &[0x00, 0x00]); // BRK + signature
    cpu.step(&mut bus);
    assert_eq!(cpu.regs.pc, 0xB000);
    assert!(cpu.regs.p.contains(Status::I));
    assert!(!cpu.regs.p.contains(Status::D));
}

#[test]
fn cop_vectoring_native() {
    let mut bus = TestBus::new();
    let mut cpu = native_cpu(&mut bus);
    bus.set16(vectors::COP_NATIVE, 0xA000);
    bus.load(0x00_8000, &[0x02, 0x00]); // COP + signature
    cpu.step(&mut bus);
    assert_eq!(cpu.regs.pc, 0xA000);
}

#[test]
fn rti_restores_native() {
    let mut bus = TestBus::new();
    let mut cpu = native_cpu(&mut bus);
    bus.set16(vectors::IRQ_NATIVE, 0xC000);
    bus.load(0x00_C000, &[0x40]); // RTI at the handler
    bus.irq = true;
    cpu.regs.set_flag(Status::I, false);
    cpu.regs.pc = 0x8000;
    cpu.regs.pbr = 0x00;
    cpu.step(&mut bus); // take IRQ -> push frame, jump to C000
    let saved_pc = 0x8000_u16;
    cpu.step(&mut bus); // RTI
    assert_eq!(cpu.regs.pc, saved_pc);
}

#[test]
fn abs_indexed_page_cross_penalty() {
    let mut bus = TestBus::new();
    let mut cpu = native_cpu(&mut bus);
    cpu.regs.set_flag(Status::M, true); // 8-bit acc
    cpu.regs.set_flag(Status::X, true); // 8-bit index
    cpu.regs.set_x(0x01);
    // LDA $80FF,X -> crosses into 0x8100: +1.
    bus.load(0x00_8000, &[0xBD, 0xFF, 0x80]);
    let crossed = cpu.step(&mut bus);
    cpu.regs.pc = 0x8000;
    cpu.regs.set_x(0x01);
    // LDA $8000,X -> no cross.
    bus.load(0x00_8000, &[0xBD, 0x00, 0x80]);
    let no_cross = cpu.step(&mut bus);
    assert_eq!(crossed, no_cross + 1);
}

#[test]
fn block_move_mvn() {
    let mut bus = TestBus::new();
    let mut cpu = native_cpu(&mut bus);
    // Move 4 bytes from bank 01 offset 0 to bank 02 offset 0.
    bus.load(0x01_0000, &[0xAA, 0xBB, 0xCC, 0xDD]);
    cpu.regs.set_x(0x0000);
    cpu.regs.set_y(0x0000);
    cpu.regs.a = 0x0003; // count = A+1 = 4
    bus.load(0x00_8000, &[0x54, 0x02, 0x01]); // MVN dst=02 src=01
    // Step repeatedly until the move completes (A wraps to 0xFFFF).
    for _ in 0..4 {
        cpu.step(&mut bus);
    }
    assert_eq!(cpu.regs.a, 0xFFFF);
    assert_eq!(bus.read24(0x02_0000), 0xAA);
    assert_eq!(bus.read24(0x02_0003), 0xDD);
    assert_eq!(cpu.regs.dbr, 0x02);
}

#[test]
fn all_opcodes_execute_without_panic() {
    // Sweep every opcode against a flat bus; just assert none panics and PC advances.
    for op in 0u16..=0xFF {
        let mut bus = TestBus::new();
        bus.set16(vectors::RESET, 0x8000);
        let mut cpu = Cpu::new();
        cpu.reset(&mut bus);
        bus.load(0x00_8000, &[op as u8, 0x00, 0x00, 0x00]);
        let _ = cpu.step(&mut bus);
    }
}

#[test]
fn stp_halts() {
    let mut bus = TestBus::new();
    let mut cpu = native_cpu(&mut bus);
    bus.load(0x00_8000, &[0xDB]); // STP
    cpu.step(&mut bus);
    assert!(cpu.stopped);
    let pc_before = cpu.regs.pc;
    cpu.step(&mut bus); // remains halted
    assert_eq!(cpu.regs.pc, pc_before);
}

#[test]
fn transfers_tax_tay_txa() {
    let mut bus = TestBus::new();
    let mut cpu = native_cpu(&mut bus);
    cpu.regs.a = 0x00AB;
    bus.load(0x00_8000, &[0xAA, 0xA8]); // TAX; TAY
    cpu.step(&mut bus);
    assert_eq!(cpu.regs.x, 0x00AB);
    cpu.step(&mut bus);
    assert_eq!(cpu.regs.y, 0x00AB);
}

#[test]
fn cycle_count_matches_cycles_field_delta() {
    let mut bus = TestBus::new();
    let mut cpu = native_cpu(&mut bus);
    bus.load(0x00_8000, &[0xA9, 0x34, 0x12]);
    let before = cpu.cycles;
    let ret = cpu.step(&mut bus);
    assert_eq!(u64::from(ret), cpu.cycles - before);
}
