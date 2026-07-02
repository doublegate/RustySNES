//! Single-game NEC DSP variant boards — DSP-2, DSP-4, ST010.
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
//! | DSP-4 (Top Gear 3000)           | SHVC-1B0N-03   | `$30–3F,$B0–BF:$8000–FFFF` mask `$3FFF` | half-window boundary at `$2000` (below=DR) | `Upd7725`  |
//! | ST010 (F1 ROC II)               | SHVC-1DS0B-20  | `$60–67,$E0–E7:$0000–3FFF` (registers) + `$68–6F,$E8–EF:$0000–7FFF` (battery data RAM, direct [`Upd77c25::read_dp`]/[`write_dp`](Upd77c25::write_dp) port) | low bit (even=DR) | `Upd96050` |
//!
//! DSP-3 and ST011 are NOT wired here: neither has a verified board/window entry (no game ROM in
//! this project's local corpus to validate against), so guessing a window would be an unverified,
//! untestable claim — `docs/adr/0003`'s honesty gate means the cart simply runs as its base board
//! (unmapped coprocessor window) until one can be pinned against a real cart, exactly like every
//! other not-yet-implemented coprocessor.
//!
//! There is no header-byte signal that distinguishes DSP-1 from DSP-2/4/ST010 (the chipset byte
//! only flags "has an NEC DSP" generically) — real emulators resolve this via a cartridge
//! database; lacking one, `detect` matches the 21-byte internal title against each chip's one
//! known game, the same single-game-chip approach ares' own database reduces to for these titles.

// Chip-name jargon (DSP-1..4, ST010, uPD7725, ...) is not Rust code.
#![allow(clippy::doc_markdown)]

use alloc::boxed::Box;

use crate::board::{Board, Coprocessor, MappedAddr};
use crate::coproc::upd77c25::{Revision, Upd77c25};

/// Which single-game NEC DSP variant a cart carries.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Variant {
    /// DSP-2 — Dungeon Master.
    Dsp2,
    /// DSP-4 — Top Gear 3000.
    Dsp4,
    /// ST010 — F1 ROC II: Race of Champions.
    St010,
}

impl Variant {
    /// Detect the variant from the cart's 21-byte internal title (uppercased), if it matches one
    /// of the three known single-game carts. `None` for every other ROM (including plain DSP-1).
    #[must_use]
    pub fn detect(title_upper: &str) -> Option<Self> {
        if title_upper.contains("DUNGEON MASTER") {
            Some(Self::Dsp2)
        } else if title_upper.contains("TOP GEAR 3000") {
            Some(Self::Dsp4)
        } else if title_upper.contains("F1 ROC") {
            Some(Self::St010)
        } else {
            None
        }
    }

    const fn revision(self) -> Revision {
        match self {
            Self::Dsp2 | Self::Dsp4 => Revision::Upd7725,
            Self::St010 => Revision::Upd96050,
        }
    }

    /// `(register-bank-lo, register-bank-hi, register-mirror-bank-lo, register-mirror-bank-hi)`.
    const fn reg_banks(self) -> (u8, u8, u8, u8) {
        match self {
            Self::Dsp2 => (0x20, 0x3F, 0xA0, 0xBF),
            Self::Dsp4 => (0x30, 0x3F, 0xB0, 0xBF),
            Self::St010 => (0x60, 0x67, 0xE0, 0xE7),
        }
    }

    /// `Some((lo, hi, mirror-lo, mirror-hi))` battery data-RAM banks (ST010/011 only — the other
    /// two chips have no separate directly-mapped data-RAM window).
    const fn dp_banks(self) -> Option<(u8, u8, u8, u8)> {
        match self {
            Self::St010 => Some((0x68, 0x6F, 0xE8, 0xEF)),
            Self::Dsp2 | Self::Dsp4 => None,
        }
    }

    /// Firmware file name this project's `firmware_candidates` convention expects.
    #[must_use]
    pub const fn firmware_name(self) -> &'static str {
        match self {
            Self::Dsp2 => "dsp2.rom",
            Self::Dsp4 => "dsp4.rom",
            Self::St010 => "st010.rom",
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
    // ST010's registers sit in `$0000-$3FFF` (no `>= 0x8000` gate, unlike DSP-2/4).
    if variant == Variant::St010 && in_bank(bank, lo, hi, mlo, mhi) && addr <= 0x3FFF {
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
            dsp: Upd77c25::new(variant.revision()),
            variant,
        }
    }
}

impl Board for NecDspVariantBoard {
    fn name(&self) -> &'static str {
        match self.variant {
            Variant::Dsp2 => "LoROM+DSP-2",
            Variant::Dsp4 => "LoROM+DSP-4",
            Variant::St010 => "LoROM+ST010",
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

    fn firmware_hint(&self) -> Option<&'static str> {
        Some(self.variant.firmware_name())
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
        assert_eq!(Variant::detect("SUPER MARIO KART     "), None);
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
    fn inert_without_firmware() {
        let mut b = board(Variant::Dsp2);
        assert!(!b.dsp.firmware_loaded());
        assert_eq!(b.read24(0x20_8000), 0);
    }
}
