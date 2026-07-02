//! The S-DD1 entropy decoder — a Golomb-code-run + adaptive-binary-probability-estimation
//! decompressor (Ricoh design). Clean-room port of ares' `SDD1::Decompressor` (ISC,
//! `sfc/coprocessor/sdd1/decompressor.cpp`); every constant table below (`RUN_COUNT`,
//! `EVOLUTION_TABLE`) is copied byte-for-byte from that source, not re-derived — this is real
//! Ricoh/Nintendo hardware behavior, not something a general algorithm description would recover.
//!
//! Structurally: an [`InputManager`] reads raw (still-compressed) ROM bytes and peels off
//! variable-length codewords; 8 parallel [`BitsGenerator`]s each decode Golomb-coded "run of the
//! most-probable-symbol, then one least-probable-symbol" sequences via the shared
//! [`GolombCodeDecoder`]; a [`ProbabilityEstimationModule`] picks WHICH of the 8 generators to
//! draw from per bit, adapting a 32-entry context table as it goes (the `EVOLUTION_TABLE` finite
//! state machine); a [`ContextModel`] derives that context index from a per-bitplane history of
//! recently-decoded bits; and [`OutputLogic`] assembles the final decompressed byte from 1, 2, or
//! 4 bitplanes' worth of [`ContextModel`] output depending on the compressed stream's declared
//! pixel format.

#![allow(
    clippy::doc_markdown,
    clippy::similar_names,
    clippy::cast_possible_truncation,
    // `&[u8; 4]` is threaded through the whole decode call chain (IM -> GCD -> BG -> PEM -> CM ->
    // OL) to avoid a Decompressor->SDD1 back-reference (see the module doc); clippy's by-value
    // suggestion would just move the same 4 bytes onto every stack frame instead of the caller's.
    clippy::trivially_copy_pass_by_ref
)]

/// `GolombCodeDecoder::runCount` — ares `decompressor.cpp`, copied verbatim.
#[rustfmt::skip]
const RUN_COUNT: [u8; 256] = [
    0x00, 0x00, 0x01, 0x00, 0x03, 0x01, 0x02, 0x00,
    0x07, 0x03, 0x05, 0x01, 0x06, 0x02, 0x04, 0x00,
    0x0f, 0x07, 0x0b, 0x03, 0x0d, 0x05, 0x09, 0x01,
    0x0e, 0x06, 0x0a, 0x02, 0x0c, 0x04, 0x08, 0x00,
    0x1f, 0x0f, 0x17, 0x07, 0x1b, 0x0b, 0x13, 0x03,
    0x1d, 0x0d, 0x15, 0x05, 0x19, 0x09, 0x11, 0x01,
    0x1e, 0x0e, 0x16, 0x06, 0x1a, 0x0a, 0x12, 0x02,
    0x1c, 0x0c, 0x14, 0x04, 0x18, 0x08, 0x10, 0x00,
    0x3f, 0x1f, 0x2f, 0x0f, 0x37, 0x17, 0x27, 0x07,
    0x3b, 0x1b, 0x2b, 0x0b, 0x33, 0x13, 0x23, 0x03,
    0x3d, 0x1d, 0x2d, 0x0d, 0x35, 0x15, 0x25, 0x05,
    0x39, 0x19, 0x29, 0x09, 0x31, 0x11, 0x21, 0x01,
    0x3e, 0x1e, 0x2e, 0x0e, 0x36, 0x16, 0x26, 0x06,
    0x3a, 0x1a, 0x2a, 0x0a, 0x32, 0x12, 0x22, 0x02,
    0x3c, 0x1c, 0x2c, 0x0c, 0x34, 0x14, 0x24, 0x04,
    0x38, 0x18, 0x28, 0x08, 0x30, 0x10, 0x20, 0x00,
    0x7f, 0x3f, 0x5f, 0x1f, 0x6f, 0x2f, 0x4f, 0x0f,
    0x77, 0x37, 0x57, 0x17, 0x67, 0x27, 0x47, 0x07,
    0x7b, 0x3b, 0x5b, 0x1b, 0x6b, 0x2b, 0x4b, 0x0b,
    0x73, 0x33, 0x53, 0x13, 0x63, 0x23, 0x43, 0x03,
    0x7d, 0x3d, 0x5d, 0x1d, 0x6d, 0x2d, 0x4d, 0x0d,
    0x75, 0x35, 0x55, 0x15, 0x65, 0x25, 0x45, 0x05,
    0x79, 0x39, 0x59, 0x19, 0x69, 0x29, 0x49, 0x09,
    0x71, 0x31, 0x51, 0x11, 0x61, 0x21, 0x41, 0x01,
    0x7e, 0x3e, 0x5e, 0x1e, 0x6e, 0x2e, 0x4e, 0x0e,
    0x76, 0x36, 0x56, 0x16, 0x66, 0x26, 0x46, 0x06,
    0x7a, 0x3a, 0x5a, 0x1a, 0x6a, 0x2a, 0x4a, 0x0a,
    0x72, 0x32, 0x52, 0x12, 0x62, 0x22, 0x42, 0x02,
    0x7c, 0x3c, 0x5c, 0x1c, 0x6c, 0x2c, 0x4c, 0x0c,
    0x74, 0x34, 0x54, 0x14, 0x64, 0x24, 0x44, 0x04,
    0x78, 0x38, 0x58, 0x18, 0x68, 0x28, 0x48, 0x08,
    0x70, 0x30, 0x50, 0x10, 0x60, 0x20, 0x40, 0x00,
];

/// `PEM::evolutionTable` — `(codeNumber, nextIfMps, nextIfLps)`, ares `decompressor.cpp`, copied
/// verbatim (a 33-state finite state machine over the 8 `BitsGenerator`s).
const EVOLUTION_TABLE: [(u8, u8, u8); 33] = [
    (0, 25, 25),
    (0, 2, 1),
    (0, 3, 1),
    (0, 4, 2),
    (0, 5, 3),
    (1, 6, 4),
    (1, 7, 5),
    (1, 8, 6),
    (1, 9, 7),
    (2, 10, 8),
    (2, 11, 9),
    (2, 12, 10),
    (2, 13, 11),
    (3, 14, 12),
    (3, 15, 13),
    (3, 16, 14),
    (3, 17, 15),
    (4, 18, 16),
    (4, 19, 17),
    (5, 20, 18),
    (5, 21, 19),
    (6, 22, 20),
    (6, 23, 21),
    (7, 24, 22),
    (7, 24, 23),
    (0, 26, 1),
    (1, 27, 2),
    (2, 28, 4),
    (3, 29, 8),
    (4, 30, 12),
    (5, 31, 16),
    (6, 32, 18),
    (7, 24, 22),
];

use rustysnes_savestate::{SaveReader, SaveStateError, SaveWriter};

/// Read a byte through the S-DD1 MMC bank mapping (ares `mmcRead`): bits 20-21 of `address`
/// select which of the four `mmc_map` registers supplies the top 4 bits of a 24-bit ROM offset.
fn mmc_read(rom: &[u8], mmc_map: &[u8; 4], address: u32) -> u8 {
    let reg = mmc_map[((address >> 20) & 0x3) as usize];
    let off = (u32::from(reg & 0xF) << 20) | (address & 0x0F_FFFF);
    rom.get((off as usize) % rom.len().max(1))
        .copied()
        .unwrap_or(0)
}

#[derive(Debug, Clone, Copy, Default)]
struct InputManager {
    offset: u32,
    bit_count: u32,
}

impl InputManager {
    const fn init(&mut self, offset: u32) {
        self.offset = offset;
        self.bit_count = 4;
    }

    fn get_code_word(&mut self, code_length: u8, rom: &[u8], mmc_map: &[u8; 4]) -> u8 {
        // ares' `n8` (nall `Natural<8>`) operands undergo C++'s usual integer promotion to `int`
        // before a shift, so `codeWord >> (9 - bitCount)` is well-defined (= 0) even when the
        // shift amount is exactly 8 (`bitCount == 1`, reachable whenever the prior call ended
        // with `bitCount == 0`). A `u8 >> 8` in Rust has no such promotion and is a real
        // shift-amount-overflow bug, so both shifts widen to `u32` first and truncate back.
        let mut code_word =
            ((u32::from(mmc_read(rom, mmc_map, self.offset))) << self.bit_count) as u8;
        self.bit_count += 1;

        if code_word & 0x80 != 0 {
            code_word |=
                (u32::from(mmc_read(rom, mmc_map, self.offset + 1)) >> (9 - self.bit_count)) as u8;
            self.bit_count += u32::from(code_length);
        }

        if self.bit_count & 0x08 != 0 {
            self.offset += 1;
            self.bit_count &= 0x07;
        }
        code_word
    }
}

#[derive(Debug, Clone, Copy, Default)]
struct BitsGenerator {
    mps_count: u8,
    lps_index: bool,
}

impl BitsGenerator {
    const fn init(&mut self) {
        self.mps_count = 0;
        self.lps_index = false;
    }

    /// Returns `(bit, end_of_run)`.
    fn get_bit(
        &mut self,
        code_number: u8,
        im: &mut InputManager,
        rom: &[u8],
        mmc_map: &[u8; 4],
    ) -> (u8, bool) {
        if self.mps_count == 0 && !self.lps_index {
            let code_word = im.get_code_word(code_number, rom, mmc_map);
            if code_word & 0x80 != 0 {
                self.lps_index = true;
                self.mps_count = RUN_COUNT[(code_word >> (code_number ^ 0x07)) as usize];
            } else {
                self.mps_count = 1 << code_number;
            }
        }

        let bit = if self.mps_count > 0 {
            self.mps_count -= 1;
            0
        } else {
            self.lps_index = false;
            1
        };
        let end_of_run = self.mps_count == 0 && !self.lps_index;
        (bit, end_of_run)
    }
}

#[derive(Debug, Clone, Copy, Default)]
struct ContextInfo {
    status: u8,
    mps: u8,
}

#[derive(Debug, Clone, Copy, Default)]
struct Pem {
    context: [ContextInfo; 32],
}

impl Pem {
    fn init(&mut self) {
        self.context = [ContextInfo::default(); 32];
    }

    #[allow(clippy::too_many_arguments)]
    fn get_bit(
        &mut self,
        context: u8,
        bg: &mut [BitsGenerator; 8],
        im: &mut InputManager,
        rom: &[u8],
        mmc_map: &[u8; 4],
    ) -> u8 {
        let info = &mut self.context[usize::from(context)];
        let current_status = info.status;
        let current_mps = info.mps;
        let (code_number, next_if_mps, next_if_lps) = EVOLUTION_TABLE[usize::from(current_status)];

        let (bit, end_of_run) = bg[usize::from(code_number)].get_bit(code_number, im, rom, mmc_map);

        if end_of_run {
            if bit != 0 {
                if current_status & 0xFE == 0 {
                    info.mps ^= 1;
                }
                info.status = next_if_lps;
            } else {
                info.status = next_if_mps;
            }
        }
        bit ^ current_mps
    }
}

#[derive(Debug, Clone, Copy, Default)]
struct ContextModel {
    bitplanes_info: u8,
    context_bits_info: u8,
    bit_number: u8,
    current_bitplane: u8,
    previous_bitplane_bits: [u16; 8],
}

impl ContextModel {
    fn init(&mut self, offset: u32, rom: &[u8], mmc_map: &[u8; 4]) {
        let b = mmc_read(rom, mmc_map, offset);
        self.bitplanes_info = b & 0xC0;
        self.context_bits_info = b & 0x30;
        self.bit_number = 0;
        self.previous_bitplane_bits = [0; 8];
        self.current_bitplane = match self.bitplanes_info {
            0x00 => 1,
            0x40 => 7,
            _ => 3, // 0x80 (0xC0 doesn't reach this path — see get_bit's own switch)
        };
    }

    #[allow(clippy::too_many_arguments)]
    fn get_bit(
        &mut self,
        pem: &mut Pem,
        bg: &mut [BitsGenerator; 8],
        im: &mut InputManager,
        rom: &[u8],
        mmc_map: &[u8; 4],
    ) -> u8 {
        match self.bitplanes_info {
            0x00 => self.current_bitplane ^= 1,
            0x40 => {
                self.current_bitplane ^= 1;
                if self.bit_number.trailing_zeros() >= 7 {
                    self.current_bitplane = (self.current_bitplane + 2) & 0x07;
                }
            }
            0x80 => {
                self.current_bitplane ^= 1;
                if self.bit_number.trailing_zeros() >= 7 {
                    self.current_bitplane ^= 2;
                }
            }
            _ => self.current_bitplane = self.bit_number & 0x07, // 0xC0
        }

        let context_bits = self.previous_bitplane_bits[usize::from(self.current_bitplane)];
        let mut current_context = (self.current_bitplane & 0x01) << 4;
        current_context |= match self.context_bits_info {
            0x00 => (((context_bits & 0x01C0) >> 5) | (context_bits & 0x0001)) as u8,
            0x10 => (((context_bits & 0x0180) >> 5) | (context_bits & 0x0001)) as u8,
            0x20 => (((context_bits & 0x00C0) >> 5) | (context_bits & 0x0001)) as u8,
            _ => (((context_bits & 0x0180) >> 5) | (context_bits & 0x0003)) as u8, // 0x30
        };

        let bit = pem.get_bit(current_context, bg, im, rom, mmc_map);
        let context_bits = (context_bits << 1) | u16::from(bit);
        self.previous_bitplane_bits[usize::from(self.current_bitplane)] = context_bits;
        self.bit_number = self.bit_number.wrapping_add(1);
        bit
    }
}

#[derive(Debug, Clone, Copy, Default)]
struct OutputLogic {
    bitplanes_info: u8,
    r0: u8,
    r1: u8,
    r2: u8,
}

impl OutputLogic {
    fn init(&mut self, offset: u32, rom: &[u8], mmc_map: &[u8; 4]) {
        self.bitplanes_info = mmc_read(rom, mmc_map, offset) & 0xC0;
        self.r0 = 0x01;
    }

    #[allow(clippy::too_many_arguments)]
    fn decompress(
        &mut self,
        cm: &mut ContextModel,
        pem: &mut Pem,
        bg: &mut [BitsGenerator; 8],
        im: &mut InputManager,
        rom: &[u8],
        mmc_map: &[u8; 4],
    ) -> u8 {
        match self.bitplanes_info {
            0x00 | 0x40 | 0x80 => {
                if self.r0 == 0 {
                    self.r0 = !self.r0;
                    return self.r2;
                }
                self.r0 = 0x80;
                self.r1 = 0;
                self.r2 = 0;
                while self.r0 != 0 {
                    if cm.get_bit(pem, bg, im, rom, mmc_map) != 0 {
                        self.r1 |= self.r0;
                    }
                    if cm.get_bit(pem, bg, im, rom, mmc_map) != 0 {
                        self.r2 |= self.r0;
                    }
                    self.r0 >>= 1;
                }
                self.r1
            }
            _ => {
                // 0xC0
                self.r0 = 0x01;
                self.r1 = 0;
                while self.r0 != 0 {
                    if cm.get_bit(pem, bg, im, rom, mmc_map) != 0 {
                        self.r1 |= self.r0;
                    }
                    self.r0 = self.r0.wrapping_shl(1);
                }
                self.r1
            }
        }
    }
}

/// The full S-DD1 entropy decoder (see the module doc for the pipeline shape).
#[derive(Debug, Clone, Copy, Default)]
pub struct Decompressor {
    im: InputManager,
    bg: [BitsGenerator; 8],
    pem: Pem,
    cm: ContextModel,
    ol: OutputLogic,
}

impl Decompressor {
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Start a new decompression stream at chip-relative `offset` (the DMA source address).
    pub fn init(&mut self, offset: u32, rom: &[u8], mmc_map: &[u8; 4]) {
        self.im.init(offset);
        for g in &mut self.bg {
            g.init();
        }
        self.pem.init();
        self.cm.init(offset, rom, mmc_map);
        self.ol.init(offset, rom, mmc_map);
    }

    /// Decompress the next output byte (called once per DMA-streamed byte).
    pub fn read(&mut self, rom: &[u8], mmc_map: &[u8; 4]) -> u8 {
        self.ol.decompress(
            &mut self.cm,
            &mut self.pem,
            &mut self.bg,
            &mut self.im,
            rom,
            mmc_map,
        )
    }

    /// Write this decoder's full mid-stream state (input cursor, all 8 bit generators, the PEM's
    /// 32-entry context table, the context model, and the output-logic register triple) into a
    /// `"SDD1"` section — everything needed to resume a decompression stream mid-DMA-transfer.
    /// There is no firmware/ROM byte here to exclude (S-DD1 has none, per `docs/adr/0003`).
    pub fn save_state(&self, w: &mut SaveWriter) {
        w.section(*b"SDD1", |s| {
            s.write_u32(self.im.offset);
            s.write_u32(self.im.bit_count);
            for g in &self.bg {
                s.write_u8(g.mps_count);
                s.write_bool(g.lps_index);
            }
            for c in &self.pem.context {
                s.write_u8(c.status);
                s.write_u8(c.mps);
            }
            s.write_u8(self.cm.bitplanes_info);
            s.write_u8(self.cm.context_bits_info);
            s.write_u8(self.cm.bit_number);
            s.write_u8(self.cm.current_bitplane);
            for &word in &self.cm.previous_bitplane_bits {
                s.write_u16(word);
            }
            s.write_u8(self.ol.bitplanes_info);
            s.write_u8(self.ol.r0);
            s.write_u8(self.ol.r1);
            s.write_u8(self.ol.r2);
        });
    }

    /// The inverse of [`Self::save_state`].
    ///
    /// # Errors
    /// [`SaveStateError`] on truncated/corrupt input, or [`SaveStateError::Invalid`] if a
    /// `ContextInfo::status` is out of `EVOLUTION_TABLE`'s range: `status` indexes that 33-entry
    /// table directly (`Pem::get_bit`), so an out-of-range value from a hand-edited/corrupted
    /// save-state would panic on the very next decode step — this is a semantic state-machine
    /// index (not a natural hardware register width), so it's rejected rather than masked, the
    /// same "enum-like constraint" reasoning `Obc1Board::load_state` already applies to its
    /// cursor fields. `current_bitplane` IS masked (`& 0x07`) rather than rejected: it is a
    /// genuine 3-bit hardware quantity indexing the fixed 8-entry `previous_bitplane_bits`/`bg`
    /// arrays, exactly the same "width mask, not semantic validation" case as the NEC DSP
    /// engine's `pc`/`rp`/`dp`/`sp`.
    pub fn load_state(&mut self, r: &mut SaveReader) -> Result<(), SaveStateError> {
        let mut s = r.expect_section(*b"SDD1")?;
        self.im.offset = s.read_u32()?;
        self.im.bit_count = s.read_u32()?;
        for g in &mut self.bg {
            g.mps_count = s.read_u8()?;
            g.lps_index = s.read_bool()?;
        }
        for c in &mut self.pem.context {
            let status = s.read_u8()?;
            if usize::from(status) >= EVOLUTION_TABLE.len() {
                return Err(SaveStateError::Invalid(alloc::format!(
                    "S-DD1 PEM context status {status} is out of EVOLUTION_TABLE's range (0-{})",
                    EVOLUTION_TABLE.len() - 1
                )));
            }
            c.status = status;
            c.mps = s.read_u8()?;
        }
        self.cm.bitplanes_info = s.read_u8()?;
        self.cm.context_bits_info = s.read_u8()?;
        self.cm.bit_number = s.read_u8()?;
        self.cm.current_bitplane = s.read_u8()? & 0x07;
        for word in &mut self.cm.previous_bitplane_bits {
            *word = s.read_u16()?;
        }
        self.ol.bitplanes_info = s.read_u8()?;
        self.ol.r0 = s.read_u8()?;
        self.ol.r1 = s.read_u8()?;
        self.ol.r2 = s.read_u8()?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use alloc::vec;

    #[test]
    fn init_then_read_does_not_panic_on_zeroed_rom() {
        let rom = vec![0u8; 0x10_0000];
        let mmc_map = [0u8, 1, 2, 3];
        let mut d = Decompressor::new();
        d.init(0, &rom, &mmc_map);
        for _ in 0..64 {
            d.read(&rom, &mmc_map);
        }
    }

    #[test]
    fn run_count_table_matches_ares_first_and_last_entries() {
        assert_eq!(RUN_COUNT[0], 0x00);
        assert_eq!(RUN_COUNT[1], 0x00);
        assert_eq!(RUN_COUNT[255], 0x00);
        assert_eq!(RUN_COUNT[64], 0x3f);
    }
}
