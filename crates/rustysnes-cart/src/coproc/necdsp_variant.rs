//! Single-game NEC DSP variant boards — DSP-2, DSP-3, DSP-4, ST010, ST011.
//!
//! Riding the shared [`crate::coproc::upd77c25::Upd77c25`] engine
//! ([`Dsp1Board`](super::dsp1::Dsp1Board) covers DSP-1 itself, which uses a different,
//! board-specific window split). DR/SR splits differ per chip — ares' generic `NECDSP::read`/`write`
//! (`sfc/coprocessor/necdsp/memory.cpp`) suggests a uniform low-address-bit split (even=DR,
//! odd=SR) for the whole non-DSP-1 family, and that's what DSP-2/ST010 use — but DSP-4 (Top Gear
//! 3000) does NOT: it uses the SAME half-window boundary split DSP-1 does (`docs/cart.md`
//! §DSP-1), confirmed empirically against its own boot-time hardware-presence check (a 16-bit
//! compare of the masked window's first two bytes against `$FFFF`, which only passes if both
//! bytes read the same port). Board attributions from `ares` `System/Super Famicom/boards.bml`:
//!
//! | Chip (game)                    | Board          | Register window (bank:addr)         | DR/SR split | Revision  |
//! |---------------------------------|----------------|--------------------------------------|-------------|-----------|
//! | DSP-2 (Dungeon Master)          | SHVC-1B5B-02   | `$20–3F,$A0–BF:$8000–FFFF` mask `$3FFF` | low bit (even=DR) | `Upd7725`  |
//! | DSP-3 (SD Gundam GX)            | SHVC-1B3B-20   | `$20–3F,$A0–BF:$8000–FFFF` (full upper half) | low bit (even=DR) | `Upd7725`  |
//! | DSP-4 (Top Gear 3000)           | SHVC-1B0N-03   | `$30–3F,$B0–BF:$8000–FFFF` mask `$3FFF` | half-window boundary at `$2000` (below=DR) | `Upd7725`  |
//! | ST010 (F1 ROC II)               | SHVC-1DS0B-20  | `$60–67,$E0–E7:$0000–3FFF` (registers) + `$68–6F,$E8–EF:$0000–7FFF` (battery data RAM, direct [`Upd77c25::read_dp`]/[`write_dp`](Upd77c25::write_dp) port) | low bit (even=DR) | `Upd96050` |
//! | ST011 (2-dan Morita Shougi)     | SHVC-1DS0B-20  | same windows as ST010 | low bit (even=DR) | `Upd96050` **@ 15 MHz** |
//!
//! **DSP-3 and ST011 are now wired (v1.24.0), validated against their real carts.** They were held
//! until a validation ROM existed for each (`docs/adr/0003`: an unvalidated window is an untestable
//! claim); once *SD Gundam GX* and *Hayazashi 2-dan Morita Shougi* were supplied, both were wired
//! from their reference-pinned specs and confirmed live (`dsp3_st011_oncart`: the game reaches the
//! register window, `host_accesses > 0`, deterministically):
//!
//!   - **DSP-3 (SD Gundam GX).** Window banks `$20–3F,$A0–BF` over the FULL `$8000–FFFF` (snes9x
//!     `M_DSP3_LOROM`), generic low-bit split (even=DR, odd=SR — bsnes `NECDSP::read`/`write`
//!     `addr & 1`), NOT DSP-4's half-boundary. It is "not simply DSP-2's" in the *window* (DSP-2 is
//!     `$6000–6FFF + $8000–BFFF`), not the split. Same 7.6 MHz `Upd7725` rate. Detection is special:
//!     the internal title is Shift-JIS (`SDｶﾞﾝﾀﾞﾑGX`), so the header's UTF-8 title decode is empty and
//!     the string [`Variant::detect`] cannot see it — it is matched on the raw title bytes by
//!     [`Variant::detect_dsp3_raw`] from the DSP board-selection path instead.
//!   - **ST011 (2-dan Morita Shougi).** Identical `EXNEC`/`Upd96050` board to ST010 — same register
//!     window `$60–67,$E0–E7` and battery data-RAM window `$68–6F,$E8–EF` — but the oscillator is
//!     **15 MHz, not ST010's 11 MHz** (ares + bsnes). Since the two share the `Upd96050`
//!     register-width revision but not the clock, the board constructs the engine with the pinned,
//!     compile-time-verified `upd77c25::UPD96050_ST011_RATE` (`1_500_000/2_147_727`) via
//!     [`Upd77c25::with_rate`] rather than the revision default. Its shogi AI is gated behind menu
//!     input, so the liveness test drives Start/A to reach it (the same gameplay-gated signal DSP-1's
//!     Pilotwings/SMK carry). ST011 declares the `$F` "custom" chipset nibble rather than the DSP
//!     family's usual `$0`, so `header::coprocessor_from_chipset` routes its ASCII title back to the
//!     DSP family; distinct from ST018's `NIDAN MORITASHOGI2` (SHOUGI vs SHOGI2).
//!
//! Both are `BestEffort` (`docs/adr/0003`): liveness- and determinism-validated against the real
//! games, but not golden-framebuffer-blessed (no multi-reference agreement is pinned for them here).
//!
//! There is no header-byte signal that distinguishes DSP-1 from DSP-2/4/ST010 (the chipset byte
//! only flags "has an NEC DSP" generically) — real emulators resolve this via a cartridge
//! database; lacking one, `detect` matches the 21-byte internal title against each chip's one
//! known game, the same single-game-chip approach ares' own database reduces to for these titles.

// Chip-name jargon (DSP-1..4, ST010, uPD7725, ...) is not Rust code.
#![allow(clippy::doc_markdown)]

use alloc::boxed::Box;

use rustysnes_savestate::{SaveReader, SaveStateError, SaveWriter};

use crate::board::{Board, Coprocessor, MappedAddr};
use crate::coproc::upd77c25::{Revision, Upd77c25};

/// Which single-game NEC DSP variant a cart carries.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Variant {
    /// DSP-2 — Dungeon Master.
    Dsp2,
    /// DSP-3 — SD Gundam GX. Same µPD7725 engine + generic low-bit DR/SR split as DSP-2, but the
    /// register window is the full `$8000–FFFF` (snes9x `M_DSP3_LOROM`), not DSP-2's split range.
    Dsp3,
    /// DSP-4 — Top Gear 3000.
    Dsp4,
    /// ST010 — F1 ROC II: Race of Champions.
    St010,
    /// ST011 — Hayazashi 2-dan Morita Shougi. Identical board to ST010 (same µPD96050 window, split,
    /// and battery-RAM port) except the firmware dump and the **15 MHz** clock (ST010 is 11 MHz).
    St011,
}

impl Variant {
    /// Detect the variant from the cart's 21-byte internal title (uppercased), if it matches one of
    /// the title-detectable single-game carts. `None` for every other ROM (including plain DSP-1).
    ///
    /// DSP-3 (SD Gundam GX) is NOT detected here: its internal title is Shift-JIS (`SDｶﾞﾝﾀﾞﾑGX`), so
    /// the header's `from_utf8` title decode yields an empty string — it is matched on the raw title
    /// bytes instead, by [`Variant::detect_dsp3_raw`], from the DSP board-selection path.
    #[must_use]
    pub fn detect(title_upper: &str) -> Option<Self> {
        if title_upper.contains("DUNGEON MASTER") {
            Some(Self::Dsp2)
        } else if title_upper.contains("TOP GEAR 3000") {
            Some(Self::Dsp4)
        } else if title_upper.contains("F1 ROC") {
            Some(Self::St010)
        } else if title_upper.contains("2DAN MORITA SHOUGI") {
            // ST011 — Hayazashi 2-dan Morita Shougi (ASCII title). Distinct from ST018's
            // `NIDAN MORITASHOGI2`, which is handled in `header::coprocessor_from_chipset`.
            Some(Self::St011)
        } else {
            None
        }
    }

    /// Detect DSP-3 from the raw (un-decoded) 21-byte title field. SD Gundam GX's title is the
    /// Shift-JIS `"SD" ｶﾞﾝﾀﾞﾑ "GX"` — `53 44 B6 DE DD C0 DE D1 47 58` — which cannot be matched as a
    /// UTF-8 string; match the ASCII `SD` prefix + `GX` immediately after the katakana run so no
    /// other cart collides (a plain DSP-1 `SD ...` title would not have `GX` at bytes 8–9).
    #[must_use]
    pub fn detect_dsp3_raw(title_bytes: &[u8]) -> bool {
        title_bytes.len() >= 10 && &title_bytes[0..2] == b"SD" && &title_bytes[8..10] == b"GX"
    }

    const fn revision(self) -> Revision {
        match self {
            Self::Dsp2 | Self::Dsp3 | Self::Dsp4 => Revision::Upd7725,
            Self::St010 | Self::St011 => Revision::Upd96050,
        }
    }

    /// The master-clock divisor `(num, den)` the engine steps on. Defaults to the revision's rate;
    /// the ST011 shares the µPD96050 revision with the ST010 but runs at 15 MHz, not 11 MHz.
    const fn rate(self) -> (u64, u64) {
        match self {
            Self::St011 => crate::coproc::upd77c25::UPD96050_ST011_RATE,
            _ => self.revision().rates(),
        }
    }

    /// `(register-bank-lo, register-bank-hi, register-mirror-bank-lo, register-mirror-bank-hi)`.
    const fn reg_banks(self) -> (u8, u8, u8, u8) {
        match self {
            // DSP-3's window matches DSP-2's banks (`$20–3F,$A0–BF`); the difference is the address
            // range, handled in `classify` (DSP-3 spans the full `$8000–FFFF`).
            Self::Dsp2 | Self::Dsp3 => (0x20, 0x3F, 0xA0, 0xBF),
            Self::Dsp4 => (0x30, 0x3F, 0xB0, 0xBF),
            Self::St010 | Self::St011 => (0x60, 0x67, 0xE0, 0xE7),
        }
    }

    /// `Some((lo, hi, mirror-lo, mirror-hi))` battery data-RAM banks (ST010/011 only — the DSP-2/3/4
    /// µPD7725 carts have no separate directly-mapped data-RAM window).
    const fn dp_banks(self) -> Option<(u8, u8, u8, u8)> {
        match self {
            Self::St010 | Self::St011 => Some((0x68, 0x6F, 0xE8, 0xEF)),
            Self::Dsp2 | Self::Dsp3 | Self::Dsp4 => None,
        }
    }

    /// Firmware file name this project's `firmware_candidates` convention expects.
    #[must_use]
    pub const fn firmware_name(self) -> &'static str {
        match self {
            Self::Dsp2 => "dsp2.rom",
            Self::Dsp3 => "dsp3.rom",
            Self::Dsp4 => "dsp4.rom",
            Self::St010 => "st010.rom",
            Self::St011 => "st011.rom",
        }
    }
}

fn in_bank(bank: u32, lo: u8, hi: u8, mlo: u8, mhi: u8) -> bool {
    (u32::from(lo)..=u32::from(hi)).contains(&bank)
        || (u32::from(mlo)..=u32::from(mhi)).contains(&bank)
}

/// Classification of a bus address against a variant's windows.
enum Hit {
    Dr,
    Sr,
    Dp(u16),
}

fn classify(variant: Variant, addr24: u32) -> Option<Hit> {
    let bank = (addr24 >> 16) & 0xFF;
    let addr = addr24 & 0xFFFF;

    let (lo, hi, mlo, mhi) = variant.reg_banks();
    if in_bank(bank, lo, hi, mlo, mhi) && addr >= 0x8000 {
        // DSP-4 (Top Gear 3000) splits DR/SR the SAME way DSP-1 does — a half-window boundary,
        // not the low-address-bit alternation ares' generic `NECDSP` component uses for DSP-2/
        // ST010 — confirmed empirically: the boot-time hardware check at $308000/$308001 (a
        // 16-bit compare against `$FFFF`) only succeeds when BOTH bytes read the SAME port (DR),
        // which only holds if they're on the SAME side of a half-window split, not alternating.
        // The window is masked to `$3FFF` (a 0x4000 address space), so the natural boundary sits
        // at its midpoint, `$2000`.
        return Some(if variant == Variant::Dsp4 {
            if addr & 0x3FFF < 0x2000 {
                Hit::Dr
            } else {
                Hit::Sr
            }
        } else if addr & 1 != 0 {
            Hit::Sr
        } else {
            Hit::Dr
        });
    }
    // ST010/ST011 registers sit in `$0000-$3FFF` (no `>= 0x8000` gate, unlike DSP-2/3/4). ST011 is
    // the same µPD96050 board as ST010 (only firmware + clock differ), so it classifies identically.
    if matches!(variant, Variant::St010 | Variant::St011)
        && in_bank(bank, lo, hi, mlo, mhi)
        && addr <= 0x3FFF
    {
        return Some(if addr & 1 != 0 { Hit::Sr } else { Hit::Dr });
    }
    if let Some((dlo, dhi, dmlo, dmhi)) = variant.dp_banks()
        && in_bank(bank, dlo, dhi, dmlo, dmhi)
        && addr <= 0x7FFF
    {
        return Some(Hit::Dp(addr as u16));
    }
    None
}

/// A LoROM cartridge carrying a single-game NEC DSP variant (see the module doc's table).
pub struct NecDspVariantBoard {
    inner: Box<dyn Board>,
    dsp: Upd77c25,
    variant: Variant,
}

impl core::fmt::Debug for NecDspVariantBoard {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("NecDspVariantBoard")
            .field("variant", &self.variant)
            .field("inner", &self.inner.name())
            .field("firmware_loaded", &self.dsp.firmware_loaded())
            .finish()
    }
}

impl NecDspVariantBoard {
    /// Wrap a base board (`inner`) with the detected NEC DSP `variant`. Inert until
    /// [`Board::load_firmware`] supplies the chip dump (`docs/adr/0003`).
    #[must_use]
    pub fn new(inner: Box<dyn Board>, variant: Variant) -> Self {
        Self {
            inner,
            // `.with_rate` is a no-op for every variant except ST011 (15 MHz vs ST010's 11 MHz),
            // since `variant.rate()` defaults to the revision's own rate.
            dsp: Upd77c25::new(variant.revision()).with_rate(variant.rate()),
            variant,
        }
    }
}

impl Board for NecDspVariantBoard {
    fn name(&self) -> &'static str {
        match self.variant {
            Variant::Dsp2 => "LoROM+DSP-2",
            Variant::Dsp3 => "LoROM+DSP-3",
            Variant::Dsp4 => "LoROM+DSP-4",
            Variant::St010 => "LoROM+ST010",
            Variant::St011 => "LoROM+ST011",
        }
    }

    fn coprocessor(&self) -> Coprocessor {
        Coprocessor::Dsp
    }

    fn map(&self, addr24: u32) -> MappedAddr {
        if classify(self.variant, addr24).is_some() {
            MappedAddr::Coprocessor
        } else {
            self.inner.map(addr24)
        }
    }

    fn read24(&mut self, addr24: u32) -> u8 {
        match classify(self.variant, addr24) {
            Some(Hit::Dr) => self.dsp.read_dr(),
            Some(Hit::Sr) => self.dsp.read_sr(),
            Some(Hit::Dp(a)) => self.dsp.read_dp(a),
            None => self.inner.read24(addr24),
        }
    }

    fn write24(&mut self, addr24: u32, val: u8) {
        match classify(self.variant, addr24) {
            Some(Hit::Dr) => self.dsp.write_dr(val),
            Some(Hit::Sr) => self.dsp.write_sr(val),
            Some(Hit::Dp(a)) => self.dsp.write_dp(a, val),
            None => self.inner.write24(addr24, val),
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

    fn load_firmware(&mut self, bytes: &[u8]) -> bool {
        self.dsp.load_firmware(bytes)
    }

    fn coprocessor_host_accesses(&self) -> u64 {
        self.dsp.host_accesses()
    }

    /// Pin-exact model: free-run the NEC DSP on its own divisor (DSP-2/4 at 7.6 MHz, ST010's
    /// µPD96050 at 11 MHz — selected automatically by `Revision` inside `tick_master`), one call per
    /// master clock from the Bus. Replaces the old catch-up-on-DR-access.
    fn coprocessor_tick(&mut self) {
        self.dsp.tick_master();
    }

    fn firmware_hint(&self) -> Option<&'static str> {
        Some(self.variant.firmware_name())
    }

    // `variant` is fixed at construction (title-detected once, never mutated), so it needs no
    // save-state entry — only the engine's own mutable register/RAM state does.
    fn save_state(&self, w: &mut SaveWriter) {
        self.dsp.save_state(w);
        self.inner.save_state(w);
    }

    fn load_state(&mut self, r: &mut SaveReader) -> Result<(), SaveStateError> {
        self.dsp.load_state(r)?;
        self.inner.load_state(r)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::board::LoRom;
    use alloc::vec;

    fn board(variant: Variant) -> NecDspVariantBoard {
        let inner = Box::new(LoRom::new(
            vec![0u8; 0x8_0000].into_boxed_slice(),
            vec![].into_boxed_slice(),
        ));
        NecDspVariantBoard::new(inner, variant)
    }

    #[test]
    fn detect_by_title() {
        assert_eq!(
            Variant::detect("DUNGEON MASTER       "),
            Some(Variant::Dsp2)
        );
        assert_eq!(
            Variant::detect("TOP GEAR 3000        "),
            Some(Variant::Dsp4)
        );
        assert_eq!(
            Variant::detect("F1 ROC II            "),
            Some(Variant::St010)
        );
        assert_eq!(
            Variant::detect("2DAN MORITA SHOUGI   "),
            Some(Variant::St011)
        );
        // ST018's `NIDAN MORITASHOGI2` must NOT collide with ST011's `2DAN MORITA SHOUGI`.
        assert_eq!(Variant::detect("NIDAN MORITASHOGI2   "), None);
        assert_eq!(Variant::detect("SUPER MARIO KART     "), None);
    }

    #[test]
    fn detect_dsp3_from_raw_shift_jis_title() {
        // SD Gundam GX's raw title: "SD" + Shift-JIS ｶﾞﾝﾀﾞﾑ (b6 de dd c0 de d1) + "GX" + padding.
        let sd_gundam = b"SD\xb6\xde\xdd\xc0\xde\xd1GX          ";
        assert!(Variant::detect_dsp3_raw(sd_gundam));
        // A plain DSP-1 `SD ...` title without `GX` at bytes 8-9 must not match.
        assert!(!Variant::detect_dsp3_raw(b"SD KID           GX  ")); // GX not at 8-9
        assert!(!Variant::detect_dsp3_raw(b"SUPER MARIO KART     "));
        assert!(!Variant::detect_dsp3_raw(b"SDGX")); // too short
    }

    #[test]
    fn dsp2_window_split() {
        let b = board(Variant::Dsp2);
        assert!(matches!(classify(b.variant, 0x20_8000), Some(Hit::Dr)));
        assert!(matches!(classify(b.variant, 0x20_8001), Some(Hit::Sr)));
        assert!(matches!(classify(b.variant, 0xA0_8000), Some(Hit::Dr))); // mirror bank
        assert!(classify(b.variant, 0x00_8000).is_none()); // ROM, not DSP-2
    }

    #[test]
    fn dsp4_window_uses_half_boundary_split_not_bit0() {
        // Confirmed empirically against Top Gear 3000's boot-time hardware check (a 16-bit
        // compare of $308000/$308001 against $FFFF, which only an emulator running the ares
        // NECDSP-style bit0 split gets wrong): both bytes of a masked-address pair below the
        // $2000 half-window boundary read the SAME port (DR), unlike DSP-2/ST010.
        let b = board(Variant::Dsp4);
        assert!(matches!(classify(b.variant, 0x30_8000), Some(Hit::Dr)));
        assert!(matches!(classify(b.variant, 0x30_8001), Some(Hit::Dr)));
        assert!(matches!(classify(b.variant, 0x30_9FFF), Some(Hit::Dr)));
        assert!(matches!(classify(b.variant, 0x30_A000), Some(Hit::Sr)));
        assert!(matches!(classify(b.variant, 0x30_BFFF), Some(Hit::Sr)));
        // The mask folds the mirror at $C000 back onto the same $2000 boundary.
        assert!(matches!(classify(b.variant, 0x30_C000), Some(Hit::Dr)));
        assert!(matches!(classify(b.variant, 0x30_E000), Some(Hit::Sr)));
    }

    #[test]
    fn st010_register_and_dp_windows() {
        let b = board(Variant::St010);
        assert!(matches!(classify(b.variant, 0x60_0000), Some(Hit::Dr)));
        assert!(matches!(classify(b.variant, 0x60_0001), Some(Hit::Sr)));
        assert!(matches!(classify(b.variant, 0xE0_0000), Some(Hit::Dr))); // mirror bank
        assert!(matches!(classify(b.variant, 0x68_0000), Some(Hit::Dp(0)))); // battery data RAM
        assert!(matches!(
            classify(b.variant, 0xE8_0010),
            Some(Hit::Dp(0x10))
        ));
        assert!(classify(b.variant, 0x00_8000).is_none()); // ROM, not ST010
    }

    #[test]
    fn dsp3_window_split() {
        // DSP-3 (snes9x M_DSP3_LOROM): banks $20-3F,$A0-BF over the FULL $8000-FFFF, generic low-bit
        // split (even=DR, odd=SR) — same banks as DSP-2 but the whole upper half, and NOT DSP-4's
        // half-boundary. Verified against SD Gundam GX by the dsp3_st011_oncart liveness test.
        let b = board(Variant::Dsp3);
        assert!(matches!(classify(b.variant, 0x20_8000), Some(Hit::Dr)));
        assert!(matches!(classify(b.variant, 0x20_8001), Some(Hit::Sr)));
        assert!(matches!(classify(b.variant, 0xA0_8000), Some(Hit::Dr))); // mirror bank
        // The full upper half is the window, including $C000+ (which DSP-4 splits to SR) — DSP-3
        // keeps alternating low-bit up there.
        assert!(matches!(classify(b.variant, 0x20_C000), Some(Hit::Dr)));
        assert!(matches!(classify(b.variant, 0x20_C001), Some(Hit::Sr)));
        assert!(classify(b.variant, 0x00_8000).is_none()); // ROM, not DSP-3
    }

    #[test]
    fn st011_is_st010_board_at_15mhz() {
        // ST011 classifies identically to ST010 (same µPD96050 window + battery-RAM port)...
        let (st010, st011) = (Variant::St010, Variant::St011);
        for addr in [
            0x60_0000, 0x60_0001, 0xE0_0000, 0x68_0000, 0xE8_0010, 0x00_8000,
        ] {
            assert_eq!(
                classify(st010, addr).map(|h| core::mem::discriminant(&h)),
                classify(st011, addr).map(|h| core::mem::discriminant(&h)),
                "ST011 must classify {addr:#08x} the same as ST010"
            );
        }
        // ...but runs at 15 MHz, not ST010's 11 MHz.
        assert_eq!(st010.rate(), (1_100_000, 2_147_727));
        assert_eq!(st011.rate(), (1_500_000, 2_147_727));
        assert_eq!(st011.rate(), crate::coproc::upd77c25::UPD96050_ST011_RATE);
    }

    #[test]
    fn inert_without_firmware() {
        let mut b = board(Variant::Dsp2);
        assert!(!b.dsp.firmware_loaded());
        assert_eq!(b.read24(0x20_8000), 0);
    }

    #[test]
    fn engine_state_round_trips_through_save_state() {
        let mut b = board(Variant::Dsp2); // Upd7725 revision, 8192-byte firmware
        assert!(b.load_firmware(&[0u8; 8192]));
        b.dsp.write_dp(0x20, 0x5A);
        let before = b.dsp.data_ram_word(0x20 >> 1);
        assert_ne!(before, 0);

        let mut w = SaveWriter::new();
        b.save_state(&mut w);
        let bytes = w.into_bytes();

        let mut fresh = board(Variant::Dsp2);
        assert!(fresh.load_firmware(&[0u8; 8192]));
        let mut r = SaveReader::new(&bytes);
        fresh.load_state(&mut r).unwrap();

        assert_eq!(fresh.dsp.data_ram_word(0x20 >> 1), before);
        assert_eq!(r.remaining(), 0);
    }
}
