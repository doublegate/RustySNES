//! The SPC7110 decompression unit's Hudson-style adaptive binary range coder.
//!
//! Clean-room port of ares' `SPC7110::Decompressor` (ISC, `sfc/coprocessor/spc7110/decompressor.cpp`,
//! attributed there to "neviksti" / "talarubi"); the `evolution` state-machine table is copied
//! byte-for-byte — this is the real Hudson Soft hardware's adaptive model, not something a general
//! algorithm description would recover.

#![allow(clippy::doc_markdown, clippy::similar_names)]

use rustysnes_savestate::{SaveReader, SaveStateError, SaveWriter};

const MPS: u32 = 0;
const LPS: u32 = 1;
const HALF: u16 = 0x55;
const MAX: u16 = 0xff;

#[derive(Debug, Clone, Copy, Default)]
struct ModelState {
    probability: u8,
    next: [u8; 2],
}

/// `Decompressor::evolution` — ares `decompressor.cpp`, copied verbatim.
#[rustfmt::skip]
const EVOLUTION: [ModelState; 53] = [
    ModelState { probability: 0x5a, next: [ 1, 1] }, ModelState { probability: 0x25, next: [ 2, 6] }, ModelState { probability: 0x11, next: [ 3, 8] },
    ModelState { probability: 0x08, next: [ 4,10] }, ModelState { probability: 0x03, next: [ 5,12] }, ModelState { probability: 0x01, next: [ 5,15] },

    ModelState { probability: 0x5a, next: [ 7, 7] }, ModelState { probability: 0x3f, next: [ 8,19] }, ModelState { probability: 0x2c, next: [ 9,21] },
    ModelState { probability: 0x20, next: [10,22] }, ModelState { probability: 0x17, next: [11,23] }, ModelState { probability: 0x11, next: [12,25] },
    ModelState { probability: 0x0c, next: [13,26] }, ModelState { probability: 0x09, next: [14,28] }, ModelState { probability: 0x07, next: [15,29] },
    ModelState { probability: 0x05, next: [16,31] }, ModelState { probability: 0x04, next: [17,32] }, ModelState { probability: 0x03, next: [18,34] },
    ModelState { probability: 0x02, next: [ 5,35] },

    ModelState { probability: 0x5a, next: [20,20] }, ModelState { probability: 0x48, next: [21,39] }, ModelState { probability: 0x3a, next: [22,40] },
    ModelState { probability: 0x2e, next: [23,42] }, ModelState { probability: 0x26, next: [24,44] }, ModelState { probability: 0x1f, next: [25,45] },
    ModelState { probability: 0x19, next: [26,46] }, ModelState { probability: 0x15, next: [27,25] }, ModelState { probability: 0x11, next: [28,26] },
    ModelState { probability: 0x0e, next: [29,26] }, ModelState { probability: 0x0b, next: [30,27] }, ModelState { probability: 0x09, next: [31,28] },
    ModelState { probability: 0x08, next: [32,29] }, ModelState { probability: 0x07, next: [33,30] }, ModelState { probability: 0x05, next: [34,31] },
    ModelState { probability: 0x04, next: [35,33] }, ModelState { probability: 0x04, next: [36,33] }, ModelState { probability: 0x03, next: [37,34] },
    ModelState { probability: 0x02, next: [38,35] }, ModelState { probability: 0x02, next: [ 5,36] },

    ModelState { probability: 0x58, next: [40,39] }, ModelState { probability: 0x4d, next: [41,47] }, ModelState { probability: 0x43, next: [42,48] },
    ModelState { probability: 0x3b, next: [43,49] }, ModelState { probability: 0x34, next: [44,50] }, ModelState { probability: 0x2e, next: [45,51] },
    ModelState { probability: 0x29, next: [46,44] }, ModelState { probability: 0x25, next: [24,45] },

    ModelState { probability: 0x56, next: [48,47] }, ModelState { probability: 0x4f, next: [49,47] }, ModelState { probability: 0x47, next: [50,48] },
    ModelState { probability: 0x41, next: [51,49] }, ModelState { probability: 0x3c, next: [52,50] }, ModelState { probability: 0x37, next: [43,51] },
];

#[derive(Debug, Clone, Copy, Default)]
struct Context {
    prediction: u8,
    swap: u8,
}

/// Inverse Morton-code transform: unpack big-endian packed pixels. Returns odd bits in the lower
/// half, even bits in the upper half (ares `deinterleave`).
const fn deinterleave(data: u64, bits: u32) -> u32 {
    let data = data & ((1u64 << bits) - 1);
    let data = 0x5555_5555_5555_5555 & (data << bits | data >> 1);
    let data = 0x3333_3333_3333_3333 & (data | data >> 1);
    let data = 0x0f0f_0f0f_0f0f_0f0f & (data | data >> 2);
    let data = 0x00ff_00ff_00ff_00ff & (data | data >> 4);
    let data = 0x0000_ffff_0000_ffff & (data | data >> 8);
    (data | data >> 16) as u32
}

/// Extract a nibble and move it to the low four bits (ares `moveToFront`).
const fn move_to_front(list: u64, nibble: u32) -> u64 {
    let mut n = 0u32;
    let mut mask = !15u64;
    while n < 64 {
        if (list >> n & 15) as u32 == nibble {
            return (list & mask) + (list << 4 & !mask) + nibble as u64;
        }
        n += 4;
        mask <<= 4;
    }
    list
}

/// The SPC7110 tile decompressor. Takes an explicit `&[u8]` data-ROM read callback via
/// [`Decompressor::decode`]'s caller rather than a back-reference, matching this codebase's S-DD1
/// decompressor convention (`coproc::sdd1::decompressor`'s module doc has the rationale).
#[derive(Debug, Clone, Copy, Default)]
pub struct Decompressor {
    context: [[Context; 15]; 5],
    /// Bits per pixel (1, 2, or 4).
    pub bpp: u32,
    /// SPC7110 data-ROM read offset (advances as bytes are consumed).
    pub offset: u32,
    bits: u32,
    range: u16,
    input: u16,
    output: u8,
    pixels: u64,
    colormap: u64,
    /// Decompressed word after the most recent [`Decompressor::decode`] call.
    pub result: u32,
}

impl Decompressor {
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Start a new decompression stream: `mode` selects 1/2/4bpp, `origin` is the data-ROM byte
    /// offset. `read` supplies the next raw (compressed) data-ROM byte.
    pub fn initialize(&mut self, mode: u32, origin: u32, mut read: impl FnMut(u32) -> u8) {
        for root in &mut self.context {
            for node in root.iter_mut() {
                *node = Context::default();
            }
        }
        self.bpp = 1 << mode;
        self.offset = origin;
        self.bits = 8;
        self.range = MAX + 1;
        let hi = read(self.offset);
        self.offset += 1;
        let lo = read(self.offset);
        self.offset += 1;
        self.input = (u16::from(hi) << 8) | u16::from(lo);
        self.output = 0;
        self.pixels = 0;
        self.colormap = 0xfedc_ba98_7654_3210;
    }

    /// Decode the next 8-pixel word into [`Decompressor::result`].
    pub fn decode(&mut self, mut read: impl FnMut(u32) -> u8) {
        for pixel in 0..8u32 {
            let mut map = self.colormap;
            let mut diff = 0u32;

            if self.bpp > 1 {
                let pa = if self.bpp == 2 {
                    (self.pixels >> 2) & 3
                } else {
                    self.pixels & 15
                } as u32;
                let pb = if self.bpp == 2 {
                    (self.pixels >> 14) & 3
                } else {
                    (self.pixels >> 28) & 15
                } as u32;
                let pc = if self.bpp == 2 {
                    (self.pixels >> 16) & 3
                } else {
                    (self.pixels >> 32) & 15
                } as u32;

                if pa != pb || pb != pc {
                    let matched = pa ^ pb ^ pc;
                    diff = 4;
                    if (matched ^ pc) == 0 {
                        diff = 3;
                    }
                    if (matched ^ pb) == 0 {
                        diff = 2;
                    }
                    if (matched ^ pa) == 0 {
                        diff = 1;
                    }
                }

                self.colormap = move_to_front(self.colormap, pa);

                map = move_to_front(map, pc);
                map = move_to_front(map, pb);
                map = move_to_front(map, pa);
            }

            for plane in 0..self.bpp {
                let bit: u32 = if self.bpp > 1 {
                    1 << plane
                } else {
                    1 << (pixel & 3)
                };
                let history = (bit - 1) & u32::from(self.output);
                // Three independent, possibly-overriding conditions (ares `decode()`'s own
                // sequential `if` cascade) — not an if/else chain, so `useless_let_if_seq`'s
                // single-ternary suggestion would misrepresent the override order.
                #[allow(clippy::useless_let_if_seq)]
                let mut set = 0u32;

                if self.bpp == 1 {
                    set = u32::from(pixel >= 4);
                }
                if self.bpp == 2 {
                    set = diff;
                }
                if plane >= 2 && history <= 1 {
                    set = diff;
                }

                let ctx = &mut self.context[set as usize][(bit + history - 1) as usize];
                let model = &EVOLUTION[ctx.prediction as usize];
                // ares' `lps_offset` is `n8` (an 8-bit *truncating* natural) even though `range`
                // is 16-bit — the subtraction truncates to 8 bits, and the later `<< 8` widens
                // back through C++'s implicit int promotion (well-defined there; a bare `u8 << 8`
                // in Rust would be a real overflow, so the widen happens explicitly here — same
                // footgun class as the S-DD1 `IM::getCodeWord` fix).
                let lps_offset = self.range.wrapping_sub(u16::from(model.probability)) as u8;
                let lps_offset16 = u16::from(lps_offset) << 8;
                let symbol = u32::from(self.input >= lps_offset16);

                self.output = (self.output << 1) | ((symbol ^ u32::from(ctx.swap)) as u8);

                if symbol == MPS {
                    self.range = u16::from(lps_offset);
                } else {
                    self.range -= u16::from(lps_offset);
                    self.input -= lps_offset16;
                }

                while self.range <= MAX / 2 {
                    ctx.prediction = model.next[symbol as usize];

                    self.range <<= 1;
                    self.input <<= 1;

                    self.bits -= 1;
                    if self.bits == 0 {
                        self.bits = 8;
                        self.input += u16::from(read(self.offset));
                        self.offset += 1;
                    }
                }

                if symbol == LPS && u32::from(model.probability) > u32::from(HALF) {
                    ctx.swap ^= 1;
                }
            }

            let mut index = u64::from(self.output) & ((1u64 << self.bpp) - 1);
            if self.bpp == 1 {
                index ^= (self.pixels >> 15) & 1;
            }

            self.pixels = (self.pixels << self.bpp) | ((map >> (4 * index)) & 15);
        }

        self.result = match self.bpp {
            1 => self.pixels as u32,
            2 => deinterleave(self.pixels, 16),
            4 => deinterleave(u64::from(deinterleave(self.pixels, 32)), 32),
            _ => 0,
        };
    }

    /// Write this decoder's full mid-stream state — the 5x15 context table, `bpp`, the data-ROM
    /// cursor, the range-coder's bit/range/input/output accumulators, and the pixel/colormap
    /// history — into an `"SPCD"` section, so a save-state landing mid-tile-decompress resumes
    /// correctly. There is no firmware/chip-ROM byte here to exclude (`docs/adr/0003`).
    pub fn save_state(&self, w: &mut SaveWriter) {
        w.section(*b"SPCD", |s| {
            for root in &self.context {
                for node in root {
                    s.write_u8(node.prediction);
                    s.write_u8(node.swap);
                }
            }
            s.write_u32(self.bpp);
            s.write_u32(self.offset);
            s.write_u32(self.bits);
            s.write_u16(self.range);
            s.write_u16(self.input);
            s.write_u8(self.output);
            s.write_u64(self.pixels);
            s.write_u64(self.colormap);
            s.write_u32(self.result);
        });
    }

    /// The inverse of [`Self::save_state`].
    ///
    /// # Errors
    /// [`SaveStateError`] on truncated/corrupt input, a section with unconsumed trailing bytes,
    /// or [`SaveStateError::Invalid`] if a restored value would violate an invariant real
    /// execution can never break: `node.prediction` indexes the 53-entry `EVOLUTION` table
    /// directly (`decode`'s `EVOLUTION[ctx.prediction as usize]`) so an out-of-range value would
    /// panic on the very next decode step (a semantic state-machine index, not a hardware
    /// register width — rejected, not masked, matching `Obc1Board`'s cursor-field posture); `bpp`
    /// must be one of `{0,1,2,4}` — `0` is the power-on/never-`initialize`d `Default` value,
    /// `{1,2,4}` are the only values a real `1 << mode` (`mode` is a 2-bit register field) can
    /// ever produce — since any other value would either shift-overflow (`1 << plane` for
    /// `plane >= self.bpp`) or spin `for plane in 0..self.bpp` unboundedly if left untrusted;
    /// `bits` is clamped to `1..=8` (falling back to 8, `initialize`'s own power-on value) since a
    /// restored `0` risks an underflow panic on the very next decrement once `bpp` is nonzero;
    /// `range` is clamped to `128..=256` (falling back to 256, likewise `initialize`'s value)
    /// since a restored `0` would spin the normalization loop
    /// (`while self.range <= MAX / 2 { self.range <<= 1; }`) forever, hanging the host rather than
    /// just decoding garbage; `node.swap` is masked to 1 bit, a genuine hardware flip-flop.
    pub fn load_state(&mut self, r: &mut SaveReader) -> Result<(), SaveStateError> {
        let mut s = r.expect_section(*b"SPCD")?;
        for root in &mut self.context {
            for node in root.iter_mut() {
                let prediction = s.read_u8()?;
                if usize::from(prediction) >= EVOLUTION.len() {
                    return Err(SaveStateError::Invalid(alloc::format!(
                        "SPC7110 decompressor prediction {prediction} is out of EVOLUTION's range (0-{})",
                        EVOLUTION.len() - 1
                    )));
                }
                node.prediction = prediction;
                node.swap = s.read_u8()? & 1;
            }
        }
        let bpp = s.read_u32()?;
        if !matches!(bpp, 0 | 1 | 2 | 4) {
            return Err(SaveStateError::Invalid(alloc::format!(
                "SPC7110 decompressor bpp {bpp} is not one of 0, 1, 2, or 4"
            )));
        }
        self.bpp = bpp;
        self.offset = s.read_u32()?;
        // Clamped (not rejected): `decode()` decrements `bits` every range-coder step and resets
        // it to 8 only at exactly 0, so a restored value outside 1..=8 would underflow-panic on
        // the very next decrement once `bpp` is nonzero (found in review — the earlier bounds
        // check let `bits == 0` through, missing this exact case). `range` is likewise clamped:
        // the normalization loop (`while self.range <= MAX / 2 { self.range <<= 1; }`) never
        // terminates if `range` is restored as `0`, which would hang the host, not just decode
        // garbage. Both defaults mirror `initialize`'s own power-on values.
        let bits = s.read_u32()?;
        self.bits = if (1..=8).contains(&bits) { bits } else { 8 };
        let range = s.read_u16()?;
        self.range = if (128..=256).contains(&range) {
            range
        } else {
            MAX + 1
        };
        self.input = s.read_u16()?;
        self.output = s.read_u8()?;
        self.pixels = s.read_u64()?;
        self.colormap = s.read_u64()?;
        self.result = s.read_u32()?;
        if s.remaining() != 0 {
            return Err(SaveStateError::Invalid(alloc::format!(
                "SPCD section has {} trailing byte(s)",
                s.remaining()
            )));
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn initialize_then_decode_does_not_panic_on_zeroed_rom() {
        let rom = [0u8; 256];
        let mut d = Decompressor::new();
        d.initialize(0, 0, |o| rom[(o as usize) % rom.len()]);
        for _ in 0..16 {
            d.decode(|o| rom[(o as usize) % rom.len()]);
        }
    }

    #[test]
    fn evolution_table_matches_ares_first_and_last_entries() {
        assert_eq!(EVOLUTION[0].probability, 0x5a);
        assert_eq!(EVOLUTION[0].next, [1, 1]);
        assert_eq!(EVOLUTION[52].probability, 0x37);
        assert_eq!(EVOLUTION[52].next, [43, 51]);
    }
}
