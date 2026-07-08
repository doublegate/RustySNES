//! The ST018 board — the ARMv3 (ARM6) coprocessor wired into a cartridge.
//!
//! The one confirmed commercial cart is *Hayazashi Nidan Morita Shogi 2* (SETA, 1995,
//! Japan-only; internal title `NIDAN MORITASHOGI2`) — a LoROM cart (512 KiB ROM + 8 KiB
//! battery-backed SRAM) using the chip's ARM core to strengthen its shogi AI. Confirmed against
//! two independent sources: Mesen2's own header detection (`BaseCartridge::GetCoprocessorType`,
//! `RomType` high nibble `$F` + `CartridgeType == 0x02`) and ares' heuristic detector
//! (`mia/medium/super-famicom.cpp`, the identical `cartridgeTypeHi==0xf && cartridgeSubType==2`
//! signature at the extended-header byte `$xFBF`) — see `docs/st018-arm-notes.md` for the full
//! research trail, including the earlier (wrong) assumption that this chip was Star Ocean's
//! (Star Ocean uses S-DD1 only; no ARM coprocessor). This project's own [`crate::header`]
//! parser doesn't read `$xFBF` for the OTHER `$F`-nibble customs (CX4/SPC7110/S-RTC all resolve
//! by title match instead, after an earlier investigation found that byte unreliable for THOSE
//! chips against a real Mega Man X2 dump) — [`crate::header`]'s `coprocessor_from_chipset`
//! mirrors that same title-match convention here rather than introducing a new header field this
//! codebase has otherwise deliberately chosen not to trust.
//!
//! Board/bus protocol ported from Mesen2's `St018` (`Core/SNES/Coprocessors/ST018/St018.cpp`),
//! architecturally an SA-1-style deterministic master-clock catch-up (`Run()`, called before
//! every register access and at end-of-frame in the reference; here driven every single master
//! tick by [`Board::coprocessor_tick`] instead — strictly more granular, functionally
//! equivalent, and avoids needing any `rustysnes-core`-side plumbing since — unlike SA-1's
//! second 65C816 — this ARM core is entirely self-contained within `rustysnes-cart` already).
//! SNES-side register window `$00-3F,$80-BF:$3000-$3FFF` (the whole block, not just
//! `$3800-38FF` as `boards.bml` implies at a glance): `$3800` (read: pull one byte the ARM
//! placed for the SNES), `$3802` (write: push one byte to the ARM; read: clear the ack flag),
//! `$3804` (status on read; a `1->0` write transition resets the ARM, preserving its own cycle
//! counter). ARM-side 32-bit address space (top nibble selects region): `0x0` = 128 KiB PRG ROM,
//! `0x4` = the same handshake registers mirrored in, `0xA` = 32 KiB data ROM, `0xE` = 16 KiB
//! work RAM. A firmware dump is a single combined `0x28000`-byte file (PRG then data,
//! `docs/st018-arm-notes.md`) — never bundled, user-supplied only (`docs/adr/0003`).

// Chip-name jargon (ST018, ARMv3, ...) is not Rust code. The handshake state is a fixed set of
// independent hardware flags (Mesen2 `St018State`), not a state-machine candidate for an enum.
#![allow(clippy::doc_markdown, clippy::struct_excessive_bools)]

use alloc::boxed::Box;
use alloc::vec;

use rustysnes_savestate::{SaveReader, SaveStateError, SaveWriter};

use crate::board::{Board, Coprocessor, MappedAddr};
use crate::coproc::armv3::{ArmBus, Cpu};

/// ARM-side PRG ROM size (Mesen2 `St018::PrgRomSize`).
const PRG_ROM_SIZE: usize = 0x2_0000;
/// ARM-side data ROM size (Mesen2 `St018::DataRomSize`).
const DATA_ROM_SIZE: usize = 0x8000;
/// ARM-side work RAM size (Mesen2 `St018::WorkRamSize`).
const WORK_RAM_SIZE: usize = 0x4000;
/// The single combined firmware dump this board accepts (PRG ROM immediately followed by data
/// ROM — Mesen2 `FirmwareHelper::LoadSt018Firmware` splits the same way).
const FIRMWARE_SIZE: usize = PRG_ROM_SIZE + DATA_ROM_SIZE;

/// Bridges [`Cpu::step`]'s [`ArmBus`] calls to the board's ROM/RAM/handshake state, charging one
/// cycle per byte lane touched (Mesen2 `ProcessIdleCycle`/the implicit per-byte-lane cost inside
/// `Read`/`Write` — `docs/st018-arm-notes.md`'s board-bus-protocol section). A short-lived
/// borrow, not stored: [`St018Board::coprocessor_tick`] takes `self.cpu` out via
/// `core::mem::take` before constructing this (mirroring `rustysnes-core`'s SA-1 catch-up's
/// `Option::take`), so the adapter can freely borrow every OTHER field of the board.
struct St018Bus<'a> {
    board: &'a mut St018Board,
}

impl ArmBus for St018Bus<'_> {
    fn read_code(&mut self, addr: u32) -> u32 {
        self.board.arm_cycle_count += 4;
        self.board.read_arm_word(addr)
    }

    fn read(&mut self, addr: u32, byte: bool) -> u32 {
        if byte {
            self.board.arm_cycle_count += 1;
            u32::from(self.board.read_arm_byte(addr))
        } else {
            self.board.arm_cycle_count += 4;
            self.board.read_arm_word(addr)
        }
    }

    fn write(&mut self, addr: u32, value: u32, byte: bool) {
        if byte {
            self.board.arm_cycle_count += 1;
            #[allow(clippy::cast_possible_truncation)]
            self.board.write_arm_byte(addr, value as u8);
        } else {
            self.board.arm_cycle_count += 4;
            self.board.write_arm_word(addr, value);
        }
    }

    fn idle(&mut self) {
        self.board.arm_cycle_count += 1;
    }
}

/// Classify a 24-bit CPU address into the SNES-side handshake window (Mesen2 `St018::Read`/
/// `Write`'s `addr & 0xFF06` dispatch over the registered `$3000-$3FFF` block).
#[allow(clippy::cast_possible_truncation)] // `addr & 0xFF06` is always <= 0xFFFF.
fn classify(addr24: u32) -> Option<u16> {
    let bank = (addr24 >> 16) & 0xFF;
    let addr = addr24 & 0xFFFF;
    (matches!(bank, 0x00..=0x3F | 0x80..=0xBF) && (0x3000..=0x3FFF).contains(&addr))
        .then_some((addr & 0xFF06) as u16)
}

/// A cartridge carrying an ST018 (Hayazashi Nidan Morita Shogi 2's ARMv3 coprocessor).
pub struct St018Board {
    inner: Box<dyn Board>,
    cpu: Cpu,

    prg_rom: Box<[u8]>,
    data_rom: Box<[u8]>,
    work_ram: Box<[u8]>,
    firmware_loaded: bool,

    // SNES<->ARM handshake (Mesen2 `St018State`).
    has_data_for_snes: bool,
    data_snes: u8,
    ack: bool,
    has_data_for_arm: bool,
    data_arm: u8,
    arm_reset: bool,

    /// The ARM's own catch-up target, incremented by one on every [`Board::coprocessor_tick`]
    /// call — mathematically equivalent to Mesen2's `_memoryManager->GetMasterClock()` (both
    /// start at 0 and advance 1:1 with the master clock), just accumulated locally instead of
    /// read from a shared clock each time.
    target_cycle: u64,
    /// The ARM's own elapsed-cycle counter (Mesen2 `ArmV3CpuState::CycleCount`), advanced by
    /// [`St018Bus`] on every bus access/idle cycle.
    arm_cycle_count: u64,
}

impl core::fmt::Debug for St018Board {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("St018Board")
            .field("inner", &self.inner.name())
            .field("cpu", &self.cpu)
            .field("firmware_loaded", &self.firmware_loaded)
            .field("arm_reset", &self.arm_reset)
            .finish_non_exhaustive()
    }
}

impl St018Board {
    /// Wrap a base board (`inner`, the cart's own LoROM ROM/SRAM decode) with an ST018. Inert
    /// (the ARM never steps) until [`Board::load_firmware`] supplies the combined chip dump
    /// (`docs/adr/0003`).
    #[must_use]
    pub fn new(inner: Box<dyn Board>) -> Self {
        let mut board = Self {
            inner,
            cpu: Cpu::default(),
            prg_rom: vec![0u8; PRG_ROM_SIZE].into_boxed_slice(),
            data_rom: vec![0u8; DATA_ROM_SIZE].into_boxed_slice(),
            work_ram: vec![0u8; WORK_RAM_SIZE].into_boxed_slice(),
            firmware_loaded: false,
            has_data_for_snes: false,
            data_snes: 0,
            ack: false,
            has_data_for_arm: false,
            data_arm: 0,
            arm_reset: false,
            target_cycle: 0,
            arm_cycle_count: 0,
        };
        board.power_on_arm();
        board
    }

    /// Power on the ARM core, re-priming its pipeline (Mesen2 `PowerOn(forReset=false)`, used at
    /// construction and whenever firmware is (re)loaded). The priming reads' own bus-cycle cost
    /// is charged to `arm_cycle_count` like any other access — there is no prior state to
    /// preserve at these call sites (see [`Self::reset_arm`] for the one call site that does).
    fn power_on_arm(&mut self) {
        let mut cpu = core::mem::take(&mut self.cpu);
        cpu.power_on(&mut St018Bus { board: self });
        self.cpu = cpu;
    }

    /// A true ARM reset (Mesen2 `PowerOn(forReset=true)`, the SNES-side `$3804` 1->0 edge):
    /// re-primes the pipeline exactly like [`Self::power_on_arm`], but saves `arm_cycle_count`
    /// first and restores it afterward, discarding the priming reads' own cost — the reference
    /// does the identical save-before/restore-after around its own `ProcessPipeline()` call.
    fn reset_arm(&mut self) {
        let saved = self.arm_cycle_count;
        self.power_on_arm();
        self.arm_cycle_count = saved;
    }

    /// `(HasDataForSnes<<0)|(Ack<<2)|(HasDataForArm<<3)|(!ArmReset<<7)` (Mesen2 `GetStatus`).
    const fn status(&self) -> u8 {
        (self.has_data_for_snes as u8)
            | ((self.ack as u8) << 2)
            | ((self.has_data_for_arm as u8) << 3)
            | ((!self.arm_reset as u8) << 7)
    }

    /// SNES-side register read (Mesen2 `St018::Read`) — the ARM is already caught up by the time
    /// this runs (every master tick already ran [`Board::coprocessor_tick`] first), so unlike
    /// the reference there is no explicit `Run()` call needed here.
    ///
    /// Not marked `const fn`: `St018Board` carries a heap-allocated `inner: Box<dyn Board>`
    /// field, so `const`-ness here is cosmetic and fragile against future field changes — the
    /// same rationale `coproc::sharprtc`'s own dense register methods already document.
    #[allow(clippy::missing_const_for_fn)]
    fn read_register(&mut self, window: u16) -> u8 {
        match window {
            0x3800 => {
                self.has_data_for_snes = false;
                self.data_snes
            }
            0x3802 => {
                self.ack = false;
                0 // falls through to open bus in the reference; this port has no open-bus latch.
            }
            0x3804 => self.status(),
            _ => 0,
        }
    }

    /// SNES-side register write (Mesen2 `St018::Write`).
    fn write_register(&mut self, window: u16, val: u8) {
        match window {
            0x3802 => {
                self.data_arm = val;
                self.has_data_for_arm = true;
            }
            0x3804 => {
                let new_reset = val != 0;
                if self.arm_reset && !new_reset {
                    self.reset_arm();
                }
                self.arm_reset = new_reset;
            }
            _ => {}
        }
    }

    /// ARM-side byte read (Mesen2 `St018::ReadCpuByte`) — the top nibble of the 32-bit ARM
    /// address selects the region; anything unmapped reads 0 (matches the source's own
    /// `default: return 0`).
    fn read_arm_byte(&mut self, addr: u32) -> u8 {
        match addr >> 28 {
            0x0 => self.prg_rom[(addr & 0x1_FFFF) as usize],
            0x4 => match addr & 0x3F {
                0x10 => {
                    self.has_data_for_arm = false;
                    self.data_arm
                }
                0x20 => self.status(),
                _ => 0,
            },
            0xA => self.data_rom[(addr & 0x7FFF) as usize],
            0xE => self.work_ram[(addr & 0x3FFF) as usize],
            _ => 0,
        }
    }

    /// ARM-side byte write (Mesen2 `St018::WriteCpuByte`). PRG/data ROM are read-only from the
    /// ARM side. The `$04:...20/24/28/2A` cases are unresolved even in the reference
    /// implementation (commented `//??` there) — ported as no-ops rather than inventing
    /// behavior, per `docs/st018-arm-notes.md`.
    fn write_arm_byte(&mut self, addr: u32, val: u8) {
        match addr >> 28 {
            0x4 => match addr & 0x3F {
                0x00 => {
                    self.has_data_for_snes = true;
                    self.data_snes = val;
                }
                0x10 => self.ack = true,
                _ => {}
            },
            0xE => self.work_ram[(addr & 0x3FFF) as usize] = val,
            _ => {}
        }
    }

    /// Word access is 4 byte-lane accesses at `addr&!3 | {0,1,2,3}`, packed/unpacked
    /// little-endian (Mesen2 `ReadCpu`, which just calls the byte version 4x) — no
    /// misalignment-rotation is applied, confirmed against the real board implementation.
    fn read_arm_word(&mut self, addr: u32) -> u32 {
        let base = addr & !3;
        u32::from(self.read_arm_byte(base))
            | (u32::from(self.read_arm_byte(base | 1)) << 8)
            | (u32::from(self.read_arm_byte(base | 2)) << 16)
            | (u32::from(self.read_arm_byte(base | 3)) << 24)
    }

    /// The write-side mirror of [`Self::read_arm_word`] (Mesen2 `WriteCpu`).
    #[allow(clippy::cast_possible_truncation)]
    fn write_arm_word(&mut self, addr: u32, val: u32) {
        let base = addr & !3;
        self.write_arm_byte(base, val as u8);
        self.write_arm_byte(base | 1, (val >> 8) as u8);
        self.write_arm_byte(base | 2, (val >> 16) as u8);
        self.write_arm_byte(base | 3, (val >> 24) as u8);
    }
}

impl Board for St018Board {
    fn name(&self) -> &'static str {
        "LoROM+ST018"
    }

    fn coprocessor(&self) -> Coprocessor {
        Coprocessor::St018
    }

    fn map(&self, addr24: u32) -> MappedAddr {
        if classify(addr24).is_some() {
            MappedAddr::Coprocessor
        } else {
            self.inner.map(addr24)
        }
    }

    fn read24(&mut self, addr24: u32) -> u8 {
        match classify(addr24) {
            Some(w) => self.read_register(w),
            None => self.inner.read24(addr24),
        }
    }

    fn write24(&mut self, addr24: u32, val: u8) {
        if let Some(w) = classify(addr24) {
            self.write_register(w, val);
        } else {
            self.inner.write24(addr24, val);
        }
    }

    fn rom(&self) -> &[u8] {
        self.inner.rom()
    }

    fn sram(&self) -> &[u8] {
        self.inner.sram()
    }

    fn sram_mut(&mut self) -> &mut [u8] {
        self.inner.sram_mut()
    }

    /// Step the ARM core to catch up with the master clock by exactly one tick (Mesen2 `Run()`,
    /// see the module doc for why this is driven every tick instead of lazily on access/EOF).
    /// Inert (no-op) until firmware is loaded — the chip stays absent, never silently faked.
    fn coprocessor_tick(&mut self) {
        if !self.firmware_loaded {
            return;
        }
        self.target_cycle += 1;
        if self.arm_reset {
            self.arm_cycle_count = self.target_cycle;
            return;
        }
        let mut cpu = core::mem::take(&mut self.cpu);
        while self.arm_cycle_count < self.target_cycle {
            cpu.step(&mut St018Bus { board: self });
        }
        self.cpu = cpu;
    }

    fn load_firmware(&mut self, bytes: &[u8]) -> bool {
        if bytes.len() != FIRMWARE_SIZE {
            return false;
        }
        self.prg_rom.copy_from_slice(&bytes[..PRG_ROM_SIZE]);
        self.data_rom.copy_from_slice(&bytes[PRG_ROM_SIZE..]);
        self.firmware_loaded = true;
        // The constructor already primed the pipeline once, against all-zero ROM (firmware
        // hasn't loaded yet at that point) -- re-power-on now so the pipeline's `execute`/
        // `decode` slots hold the REAL reset-vector opcodes instead of permanently-stale zero
        // opcodes fetched before this firmware existed.
        self.power_on_arm();
        true
    }

    fn firmware_hint(&self) -> Option<&'static str> {
        Some("st018.rom")
    }

    fn save_state(&self, w: &mut SaveWriter) {
        w.section(*b"ST18", |s| {
            self.cpu.save_state(s);
            s.write_bytes(&self.work_ram);
            s.write_bool(self.has_data_for_snes);
            s.write_u8(self.data_snes);
            s.write_bool(self.ack);
            s.write_bool(self.has_data_for_arm);
            s.write_u8(self.data_arm);
            s.write_bool(self.arm_reset);
            s.write_u64(self.target_cycle);
            s.write_u64(self.arm_cycle_count);
        });
        self.inner.save_state(w);
    }

    fn load_state(&mut self, r: &mut SaveReader) -> Result<(), SaveStateError> {
        let mut s = r.expect_section(*b"ST18")?;
        self.cpu.load_state(&mut s)?;
        self.work_ram.copy_from_slice(s.read_bytes(WORK_RAM_SIZE)?);
        self.has_data_for_snes = s.read_bool()?;
        self.data_snes = s.read_u8()?;
        self.ack = s.read_bool()?;
        self.has_data_for_arm = s.read_bool()?;
        self.data_arm = s.read_u8()?;
        self.arm_reset = s.read_bool()?;
        self.target_cycle = s.read_u64()?;
        self.arm_cycle_count = s.read_u64()?;
        if s.remaining() != 0 {
            return Err(SaveStateError::Invalid(alloc::format!(
                "ST18 section has {} trailing byte(s)",
                s.remaining()
            )));
        }
        self.inner.load_state(r)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::board::LoRom;

    fn firmware() -> alloc::vec::Vec<u8> {
        let mut fw = vec![0u8; FIRMWARE_SIZE];
        // A single ARM opcode at the reset vector (address 0): `MOV r0, #1` = 0xE3A00001. Enough
        // to prove the catch-up loop actually steps the core once firmware is present.
        fw[0..4].copy_from_slice(&0xE3A0_0001u32.to_le_bytes());
        fw
    }

    fn board() -> St018Board {
        let inner = Box::new(LoRom::new(
            vec![0u8; 0x8_0000].into_boxed_slice(),
            vec![0u8; 0x2000].into_boxed_slice(),
        ));
        St018Board::new(inner)
    }

    #[test]
    fn window_classify() {
        assert_eq!(classify(0x00_3800), Some(0x3800));
        assert_eq!(classify(0x3F_3802), Some(0x3802));
        assert_eq!(classify(0x80_3804), Some(0x3804));
        assert_eq!(classify(0xBF_3FFE), Some(0x3F06)); // top of the window still folds via &0xFF06
        assert_eq!(classify(0x00_2FFF), None); // just below the window
        assert_eq!(classify(0x40_3800), None); // outside 00-3f/80-bf
    }

    #[test]
    fn inert_without_firmware() {
        let mut b = board();
        assert!(!b.firmware_loaded);
        // The constructor already primes the pipeline once (against all-zero ROM) regardless of
        // firmware state; the property under test is that ticking WITHOUT firmware never grows
        // that baseline further -- the chip stays fully inert, never silently faked.
        let baseline = b.arm_cycle_count;
        for _ in 0..64 {
            b.coprocessor_tick();
        }
        assert_eq!(b.arm_cycle_count, baseline);
        assert_eq!(b.read24(0x00_3804), 0x80); // status: !ArmReset only
    }

    #[test]
    fn rejects_a_wrong_sized_firmware_dump() {
        let mut b = board();
        assert!(!b.load_firmware(&[0u8; 0x100]));
        assert!(!b.firmware_loaded);
    }

    #[test]
    fn accepts_the_combined_dump_and_splits_prg_and_data() {
        let mut fw = firmware();
        fw[PRG_ROM_SIZE] = 0x42; // first data-ROM byte
        let mut b = board();
        assert!(b.load_firmware(&fw));
        assert!(b.firmware_loaded);
        assert_eq!(b.prg_rom[0..4], 0xE3A0_0001u32.to_le_bytes());
        assert_eq!(b.data_rom[0], 0x42);
    }

    #[test]
    fn coprocessor_tick_steps_the_arm_once_firmware_is_loaded() {
        let mut b = board();
        assert!(b.load_firmware(&firmware()));
        // The constructor's power-on (against all-zero ROM) plus `load_firmware`'s re-power-on
        // (against the real dump) each spend a fixed handful of housekeeping bus cycles priming
        // the pipeline before any real instruction can reach the Execute stage; 64 ticks is
        // comfortably past that baseline.
        let before = b.arm_cycle_count;
        for _ in 0..64 {
            b.coprocessor_tick();
        }
        assert!(b.arm_cycle_count > before);
        assert_eq!(b.cpu.regs.r[0], 1); // MOV r0,#1 executed
    }

    #[test]
    fn snes_side_handshake_round_trip() {
        let mut b = board();
        b.write24(0x00_3802, 0x55); // push a byte to the ARM
        assert!(b.has_data_for_arm);
        assert_eq!(b.data_arm, 0x55);
        b.data_snes = 0xAA;
        b.has_data_for_snes = true;
        assert_eq!(b.read24(0x00_3800), 0xAA); // pull it back
        assert!(!b.has_data_for_snes); // read clears the flag
    }

    #[test]
    fn a_reset_edge_reinitializes_the_arm_without_resetting_its_cycle_counter() {
        let mut b = board();
        assert!(b.load_firmware(&firmware()));
        for _ in 0..64 {
            b.coprocessor_tick();
        }
        let cycles_before_reset = b.arm_cycle_count;
        b.write24(0x00_3804, 1); // assert reset
        b.write24(0x00_3804, 0); // 1->0 edge: re-initializes the ARM
        // `power_on_arm` never touches `arm_cycle_count` -- it stays board-owned across a reset,
        // matching Mesen2's `PowerOn(forReset=true)` preserving the cycle counter.
        assert_eq!(b.arm_cycle_count, cycles_before_reset);
        assert_eq!(b.cpu.regs.r[0], 0); // registers ARE re-zeroed by the reset
    }

    #[test]
    fn state_round_trips_through_save_state() {
        let mut b = board();
        assert!(b.load_firmware(&firmware()));
        for _ in 0..64 {
            b.coprocessor_tick();
        }
        b.write24(0x00_3802, 0x77);

        let mut w = SaveWriter::new();
        b.save_state(&mut w);
        let bytes = w.into_bytes();

        let mut fresh = board();
        assert!(fresh.load_firmware(&firmware()));
        let mut r = SaveReader::new(&bytes);
        fresh.load_state(&mut r).unwrap();

        assert_eq!(fresh.arm_cycle_count, b.arm_cycle_count);
        assert_eq!(fresh.cpu.regs.r[0], b.cpu.regs.r[0]);
        assert_eq!(fresh.data_arm, 0x77);
        assert!(fresh.has_data_for_arm);
    }

    #[test]
    fn rom_and_sram_delegate_to_inner_board() {
        let mut b = board();
        assert_eq!(b.rom().len(), 0x8_0000);
        b.write24(0x70_0000, 0x99); // LoROM SRAM window
        assert_eq!(b.read24(0x70_0000), 0x99);
    }
}
