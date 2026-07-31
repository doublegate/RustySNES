//! SNES internal-header detection.
//!
//! Unlike the NES iNES header (a clean 16-byte prefix), the SNES header lives INSIDE the ROM
//! at a map-mode-dependent offset: LoROM `$7FC0`, HiROM `$FFC0`, ExHiROM `$40FFC0` (plus an
//! optional 512-byte copier prefix to skip). Detection scores each candidate offset by
//! checksum / reset-vector / printable-title / map-mode plausibility and picks the best.
//!
//! See `docs/cartridge-format.md` for the authoritative header-byte layout (`$xFC0`–`$xFDF`)
//! and the score heuristic.

use alloc::string::String;
use thiserror::Error;

/// Errors from header detection.
#[derive(Debug, Error, PartialEq, Eq)]
#[non_exhaustive]
pub enum HeaderError {
    /// The ROM image is too small to contain any candidate header.
    #[error("rom too small: {0} bytes (need at least one 64 KiB bank)")]
    TooSmall(usize),
    /// No candidate offset scored as a valid SNES header.
    #[error("no valid SNES header found at any candidate offset")]
    NoValidHeader,
}

/// The cartridge map mode (header byte `$xFD5`, low bits). Determines the base board.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MapMode {
    /// LoROM (`$20`).
    LoRom,
    /// HiROM (`$21`).
    HiRom,
    /// ExHiROM (`$25`).
    ExHiRom,
    /// ExLoROM (unofficial — no dedicated `$xFD5` value; homebrew/flashcart >4 MiB titles that
    /// keep LoROM's 32 KiB bank windowing instead of switching to HiROM's linear banks).
    ExLoRom,
}

/// Console region, derived from the destination-code header byte (`$xFD9`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Region {
    /// NTSC (60 Hz) — Japan / North America destination codes.
    Ntsc,
    /// PAL (50 Hz) — Europe / Australia destination codes.
    Pal,
}

/// On-cart coprocessor, derived from the chipset header byte (`$xFD6`).
///
/// Tier-annotated in [`crate::tier`]. Base carts are always [`Coprocessor::None`] in Phase 2;
/// the concrete coprocessor boards land in Phase 4.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum Coprocessor {
    /// No coprocessor — plain LoROM/HiROM/ExHiROM ROM (+ optional SRAM).
    None,
    /// NEC uPD77C25 DSP-1..4 (Pilotwings, Super Mario Kart, ...).
    Dsp,
    /// Super FX / GSU-1/2 (Star Fox, Yoshi's Island).
    SuperFx,
    /// SA-1 (Super Mario RPG, Kirby Super Star).
    Sa1,
    /// S-DD1 decompression (Star Ocean, Street Fighter Alpha 2).
    SDd1,
    /// SPC7110 decompression + RTC (Far East of Eden Zero).
    Spc7110,
    /// Capcom CX4 (Mega Man X2/X3).
    Cx4,
    /// OBC1 (Metal Combat).
    Obc1,
    /// Sharp RTC-4513 standalone real-time clock (Daikaijuu Monogatari II).
    Srtc,
    /// ST018 — a full `ARMv3` (ARM6) CPU coprocessor (Hayazashi Nidan Morita Shogi 2).
    St018,
}

/// A parsed SNES internal header.
///
/// Detection scores every candidate offset (`$7FC0` / `$FFC0` / `$40FFC0`, after copier-prefix
/// skip) and keeps the highest-scoring one. See [`Header::detect`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Header {
    /// The byte offset the header was found at, relative to the *copier-prefix-stripped* ROM
    /// (LoROM `$7FC0` / HiROM `$FFC0` / ExHiROM `$40FFC0`).
    pub offset: usize,
    /// The number of leading bytes that are a copier prefix (0 or 512); the board ROM is the
    /// image with these bytes stripped.
    pub copier_prefix: usize,
    /// The base map mode.
    pub map_mode: MapMode,
    /// Whether the cart runs the FastROM (3.58 MHz) access window (`$xFD5` bit 4).
    pub fast_rom: bool,
    /// The console region.
    pub region: Region,
    /// The on-cart coprocessor (or [`Coprocessor::None`]).
    pub coprocessor: Coprocessor,
    /// ROM size in bytes (the actual image size after copier strip, not the header claim).
    pub rom_size: usize,
    /// SRAM (battery save-RAM) size in bytes (0 if none).
    pub sram_size: usize,
    /// Whether the cart is battery-backed (has persistent SRAM / RTC).
    pub has_battery: bool,
    /// The 21-byte internal title (`$xFC0`), non-printable bytes replaced with spaces and
    /// trailing padding trimmed. Best-effort: the SNES header has no encoding guarantee, so this
    /// is display text, not a validated field (`v1.20.0`, added for the ROM Info debugger panel).
    pub title: String,
}

/// The largest SRAM this bound will hand to a board: 512 KiB.
///
/// Chosen as the smallest value that cannot refuse a real cartridge. No SNES board can *address*
/// more: LoROM's SRAM window (`$70-$7D:$0000-$7FFF`) reaches 448 KiB, HiROM's
/// (`$20-$3F:$6000-$7FFF`) reaches 256 KiB, and SA-1 BW-RAM tops out at 256 KiB — so 512 KiB is
/// already past every documented window, and a header claiming more is describing memory the
/// console has no way to reach.
///
/// See the clamp site in [`Header::detect`] for why this is a clamp and not a rejection.
pub const MAX_SRAM_SIZE: usize = 512 * 1024;

/// The largest shift applied to the `$xFD8` RAM-size byte before [`MAX_SRAM_SIZE`] takes over.
///
/// Both bounds are needed, and neither is redundant: the `min` on the *result* is what enforces the
/// policy, but it cannot run until the shift has already happened — and an unbounded shift is
/// itself the panic (debug) or the wrong answer (release). So the shift amount is capped first, at
/// a value comfortably inside `usize` on **32-bit targets too** (`wasm32`), and the result is
/// clamped second.
const MAX_SRAM_SIZE_SHIFT: u8 = 16;

/// Header field offsets relative to the header base (`$xFC0`).
mod field {
    /// 21-byte ASCII title at `$xFC0`.
    pub const TITLE: usize = 0x00;
    /// Title length in bytes.
    pub const TITLE_LEN: usize = 21;
    /// Map-mode + speed byte at `$xFD5`.
    pub const MAP_MODE: usize = 0x15;
    /// Chipset (cartridge type) byte at `$xFD6`.
    pub const CHIPSET: usize = 0x16;
    /// RAM-size byte at `$xFD8`.
    pub const RAM_SIZE: usize = 0x18;
    /// Destination (region) code at `$xFD9`.
    pub const REGION: usize = 0x19;
    /// Checksum complement at `$xFDC` (little-endian u16).
    pub const COMPLEMENT: usize = 0x1C;
    /// Checksum at `$xFDE` (little-endian u16).
    pub const CHECKSUM: usize = 0x1E;
    /// Emulation-mode reset vector at `$xFFC` (little-endian u16); `$xFC0 + 0x3C`.
    pub const RESET_VECTOR: usize = 0x3C;
}

impl Header {
    /// The candidate header *base* offsets (`$xFC0`) before any copier-prefix adjustment,
    /// paired with the map mode each location implies.
    //
    // The extended (>4 MiB) candidates are listed FIRST: `Self::detect`'s tie-break keeps
    // whichever candidate is found first when two offsets score identically (only strictly
    // higher scores replace `best`), and a >4 MiB image's real header can, in principle, tie a
    // spurious high-scoring match at the smaller offset — ordering extended modes first means a
    // tie favors the model that actually explains the full image, not the truncated one.
    const CANDIDATES: [(usize, MapMode); 4] = [
        (0x40_FFC0, MapMode::ExHiRom),
        (0x40_7FC0, MapMode::ExLoRom),
        (0x7FC0, MapMode::LoRom),
        (0xFFC0, MapMode::HiRom),
    ];

    /// Detect the internal header in a raw ROM image.
    ///
    /// Skips a 512-byte copier prefix when `len % 0x8000 == 0x200`, then scores each candidate
    /// offset and keeps the highest. A non-zero score is required, so an all-zero / garbage
    /// image yields [`HeaderError::NoValidHeader`].
    ///
    /// # Errors
    /// [`HeaderError::TooSmall`] if the (de-prefixed) image can't hold a single 32 KiB bank, or
    /// [`HeaderError::NoValidHeader`] if no candidate offset scores above zero.
    pub fn detect(rom: &[u8]) -> Result<Self, HeaderError> {
        let copier_prefix = if rom.len() % 0x8000 == 0x200 {
            0x200
        } else {
            0
        };
        let image = &rom[copier_prefix..];
        if image.len() < 0x8000 {
            return Err(HeaderError::TooSmall(rom.len()));
        }

        let mut best: Option<(u32, usize, MapMode)> = None;
        for (offset, map_mode) in Self::CANDIDATES {
            // The 64-byte header region `offset..offset+0x40` must fit in the de-prefixed image.
            if image.len() < offset + 0x40 {
                continue;
            }
            let score = score_candidate(image, offset, map_mode);
            if score > 0 && best.is_none_or(|(b, ..)| score > b) {
                best = Some((score, offset, map_mode));
            }
        }

        let (_, offset, map_mode) = best.ok_or(HeaderError::NoValidHeader)?;
        Ok(Self::parse(image, offset, map_mode, copier_prefix))
    }

    /// Parse the concrete header fields at a known-good `(offset, map_mode)`.
    fn parse(image: &[u8], offset: usize, map_mode: MapMode, copier_prefix: usize) -> Self {
        let h = &image[offset..];
        let chipset = h[field::CHIPSET];
        let region = region_from_code(h[field::REGION]);

        let raw_sram = h[field::RAM_SIZE];
        let fast_rom = h[field::MAP_MODE] & 0x10 != 0;

        let title_bytes = &image[offset + field::TITLE..offset + field::TITLE + field::TITLE_LEN];
        let title_upper = core::str::from_utf8(title_bytes)
            .unwrap_or("")
            .to_uppercase();
        let coprocessor = coprocessor_from_chipset(chipset, &title_upper);

        // `$xFD6` low nibble: 2 / 5 / 6 imply battery-backed RAM (RAM+battery, RAM+battery+RTC).
        let has_battery = matches!(chipset & 0x0F, 0x2 | 0x5 | 0x6);

        // `$xFD8` is `1 << N` KiB, and `N` is an arbitrary byte out of an untrusted image — a bad
        // dump, a fan hack, or a crafted file. Shifting by it unbounded is a real defect, found by
        // `fuzz/fuzz_targets/rom_header.rs`:
        //
        //   * debug builds panic outright ("attempt to shift left with overflow") for `N >= 64`;
        //   * release builds mask the shift instead, so `N = 22` silently asks
        //     `board::select` for a **4 GiB** zeroed allocation at `vec![0u8; header.sram_size]`;
        //   * `wasm32` is worse on both counts — `usize` is 32 bits there, so the panic starts at
        //     `N >= 32` and `N = 21` already exceeds `isize::MAX`.
        //
        // Clamping rather than rejecting is deliberate. The field is only an allocation hint: every
        // board wraps its accesses to `sram_size` anyway (see `Board`'s own contract), and real
        // dumps do carry garbage here, so a hard rejection would refuse images that load correctly
        // today. bsnes and ares sidestep the question entirely by resolving size from a board
        // database rather than the header; absent that database, a bound is the equivalent.
        let mut sram_size = match raw_sram {
            0 => 0,
            n => MAX_SRAM_SIZE.min(1024usize << n.min(MAX_SRAM_SIZE_SHIFT)),
        };

        // GSU games often declare 0 SRAM size but have 32 KiB or 64 KiB of on-cart RAM for the plot buffer.
        if coprocessor == Coprocessor::SuperFx && sram_size == 0 {
            if title_upper.contains("DOOM")
                || title_upper.contains("WINTER GOLD")
                || title_upper.contains("STARFOX2")
                || title_upper.contains("STAR FOX 2")
            {
                sram_size = 0x1_0000; // 64 KiB
            } else {
                sram_size = 0x8000; // 32 KiB
            }
        }

        Self {
            offset,
            copier_prefix,
            map_mode,
            fast_rom,
            region,
            coprocessor,
            rom_size: image.len(),
            sram_size,
            has_battery,
            title: decode_title(title_bytes),
        }
    }
}

/// Best-effort decode of the raw 21-byte title field: non-printable bytes become spaces, then
/// trailing padding is trimmed. See [`Header::title`].
fn decode_title(title_bytes: &[u8]) -> String {
    let decoded: String = title_bytes
        .iter()
        .map(|&b| {
            if (0x20..=0x7E).contains(&b) {
                b as char
            } else {
                ' '
            }
        })
        .collect();
    decoded.trim_end().into()
}

/// Derive the on-cart coprocessor from the chipset byte (`$xFD6`) plus (for the ambiguous `$F`
/// "custom" nibble only) the cart's title.
///
/// The low nibble is the cartridge type: `0`/`1`/`2` are plain ROM(+RAM(+battery)); `3`–`6` flag
/// that a coprocessor is present. The high nibble then names it: `0`=DSP (`µPD77C25` family,
/// including the single-game ST010/ST011 variants — see below), `1`=Super FX/GSU, `2`=OBC1,
/// `3`=SA-1, `4`=S-DD1, `F`=custom (SPC7110/CX4/S-RTC/ST018).
///
/// `$F` is genuinely ambiguous from header bytes alone — real emulators disambiguate it (and, for
/// that matter, DSP-1 vs DSP-2/3/4/ST010/011 under the SAME `$0` nibble) via a bundled cartridge
/// database keyed on title/checksum, not header parsing; even ares' own header path never reads
/// an `$xFBF` "subtype" byte (confirmed against `sfc/cartridge/`, and empirically against a real
/// Mega Man X2 dump, whose `$7FBF` is `$10` — not any documented subtype value). Lacking a
/// database, title-match the two known CX4 games here, the same single-game-chip approach
/// [`crate::coproc::necdsp_variant::Variant::detect`] uses for the DSP family's singles.
fn coprocessor_from_chipset(chipset: u8, title_upper: &str) -> Coprocessor {
    // S-RTC (Daikaijuu Monogatari II) declares chipset `$55` on the real cart: low nibble `$5`
    // passes the presence gate below, but the `match chipset >> 4` then routes high nibble `$5` to
    // the `_ => None` arm (S-RTC's title check lives in the `$F` arm, which `$5` never reaches). So
    // title-detect it up front, nibble-agnostically. Its internal title is the no-space,
    // `JYU`-romanized `DAIKAIJYUMONOGATARI2` (verified against a real dump); the spaced
    // `DAIKAIJUU/DAIKAIJU MONOGATARI` variants were earlier best-effort guesses, kept as fallbacks.
    if title_upper.contains("DAIKAIJYUMONOGATARI")
        || title_upper.contains("DAIKAIJUU MONOGATARI")
        || title_upper.contains("DAIKAIJU MONOGATARI")
    {
        return Coprocessor::Srtc;
    }
    // No coprocessor unless the type nibble marks one present. The `$F` custom category doesn't
    // follow the same RAM/battery low-nibble convention as `$0-$4` (e.g. SPC7110+RTC is `$F9`,
    // outside `0x3..=0x6` — confirmed against a real Far East of Eden Zero dump's `$FFD6`), so it
    // skips this gate and relies entirely on the title match below (already safe: no match falls
    // through to `Coprocessor::None`).
    if chipset >> 4 != 0xF && !matches!(chipset & 0x0F, 0x3..=0x6) {
        return Coprocessor::None;
    }
    match chipset >> 4 {
        0x0 => Coprocessor::Dsp,
        0x1 => Coprocessor::SuperFx,
        0x2 => Coprocessor::Obc1,
        0x3 => Coprocessor::Sa1,
        0x4 => Coprocessor::SDd1,
        0xF => {
            if title_upper.contains("MEGA MAN X2")
                || title_upper.contains("MEGAMAN X2")
                || title_upper.contains("ROCKMAN X2")
                || title_upper.contains("MEGA MAN X3")
                || title_upper.contains("MEGAMAN X3")
                || title_upper.contains("ROCKMAN X3")
            {
                Coprocessor::Cx4
            } else if title_upper.contains("TENGAI MAKYO")
                || title_upper.contains("FAR EAST OF EDEN")
            {
                Coprocessor::Spc7110
            } else if title_upper.contains("2DAN MORITA SHOUGI") {
                // ST011 (Hayazashi 2-dan Morita Shougi) — a µPD96050 NEC DSP that declares the `$F`
                // "custom" chipset nibble rather than the DSP family's usual `$0`, so it lands here
                // instead of the `0x0 => Dsp` arm above. Route it back to the DSP family; the exact
                // ST011 variant (window/split/15 MHz rate) is then resolved by
                // [`crate::coproc::necdsp_variant::Variant::detect`] from the same ASCII title.
                // Distinct from ST018's `NIDAN MORITASHOGI2` (SHOUGI vs SHOGI2), handled just below.
                Coprocessor::Dsp
            } else if title_upper.contains("MORITASHOGI2") || title_upper.contains("MORITA SHOGI2")
            {
                // ST018 (Hayazashi Nidan Morita Shogi 2) — internal title `NIDAN MORITASHOGI2`
                // per an independent database (superfamicom.org), not yet verified against a
                // real dump (no commercial copy exists in this project's local corpus, the same
                // honesty gap already carried openly for the other title-matched `$F` customs).
                // Real emulators (Mesen2, ares) disambiguate this one specific chip via the
                // extended-header `CartridgeType`/`cartridgeSubType` byte at `$xFBF` instead of a
                // title match — this project deliberately does NOT read that byte for the OTHER
                // `$F`-nibble customs above (found unreliable against a real Mega Man X2 dump),
                // so this mirrors that same established title-match convention rather than
                // introducing a new, differently-verified header field just for this one chip.
                Coprocessor::St018
            } else {
                // Other undetected `$F` customs stay BestEffort/not-started: the cart runs as
                // its base board (unmapped coprocessor window) rather than guessing.
                Coprocessor::None
            }
        }
        _ => Coprocessor::None,
    }
}

/// Map a destination (region) code (`$xFD9`) to NTSC/PAL.
///
/// Codes 0 (Japan), 1 (North America) and 13 (South Korea) are 60 Hz NTSC; codes 2–12 are the
/// 50 Hz PAL territories (Europe, Scandinavia, France, ...). Anything else defaults NTSC.
const fn region_from_code(code: u8) -> Region {
    match code {
        0x02..=0x0C => Region::Pal,
        _ => Region::Ntsc,
    }
}

/// The map-mode low nibble each candidate location expects (`$xFD5 & 0x0F`).
///
/// ExLoROM has no dedicated value — it's unofficial (confirmed against ares/bsnes: neither
/// assigns it a distinct `$xFD5` nibble) — so real ExLoROM carts fall back to reporting plain
/// LoROM's `$0` nibble here; this candidate is still disambiguated from true LoROM purely by
/// its header *offset* (`$40_7FC0`, only reachable by images >4 MiB).
const fn expected_mode_nibble(map_mode: MapMode) -> u8 {
    match map_mode {
        MapMode::LoRom | MapMode::ExLoRom => 0x0,
        MapMode::HiRom => 0x1,
        MapMode::ExHiRom => 0x5,
    }
}

/// Score a candidate header at `offset` for the map mode that location implies. Higher is
/// better; `0` means "definitely not a header here". See `docs/cartridge-format.md`.
fn score_candidate(image: &[u8], offset: usize, map_mode: MapMode) -> u32 {
    let h = &image[offset..offset + 0x40];
    let mut score = 0u32;

    // 1. Checksum + complement sum to $FFFF — the strongest signal.
    let complement = u16::from_le_bytes([h[field::COMPLEMENT], h[field::COMPLEMENT + 1]]);
    let checksum = u16::from_le_bytes([h[field::CHECKSUM], h[field::CHECKSUM + 1]]);
    if checksum != 0 && complement != 0 && checksum ^ complement == 0xFFFF {
        score += 8;
    }

    // 2. Map-mode byte matches its location (`$20`/`$21`/`$25`, ignoring the fast bit).
    let mode = h[field::MAP_MODE];
    if mode & 0xE0 == 0x20 && mode & 0x0F == expected_mode_nibble(map_mode) {
        score += 4;
    }

    // 3. Plausible reset vector: points into $8000–$FFFF.
    let reset = u16::from_le_bytes([h[field::RESET_VECTOR], h[field::RESET_VECTOR + 1]]);
    if reset >= 0x8000 {
        score += 2;
    }

    // 4. Printable 21-byte title (ASCII space..~), space-padded.
    if h[field::TITLE..field::TITLE + field::TITLE_LEN]
        .iter()
        .all(|&b| (0x20..=0x7E).contains(&b))
    {
        score += 1;
    }

    score
}

#[cfg(test)]
mod tests {
    use super::*;
    use alloc::vec;
    use alloc::vec::Vec;

    /// Build a 64 KiB image with a valid header at `base` (`$xFC0`) for `mode_nibble`.
    fn synth_image(base: usize, mode_nibble: u8, ram_byte: u8, region: u8) -> Vec<u8> {
        let mut rom = vec![0u8; 0x1_0000];
        let h = base;
        // Printable title (21 bytes).
        for (i, b) in b"RUSTYSNES TEST ROM   ".iter().enumerate() {
            rom[h + field::TITLE + i] = *b;
        }
        rom[h + field::MAP_MODE] = 0x20 | mode_nibble; // slow + mode nibble
        rom[h + field::CHIPSET] = if ram_byte == 0 { 0x00 } else { 0x02 }; // RAM+battery
        rom[h + field::RAM_SIZE] = ram_byte;
        rom[h + field::REGION] = region;
        // checksum/complement sum to 0xFFFF
        let checksum: u16 = 0x1234;
        let complement = !checksum;
        rom[h + field::COMPLEMENT..h + field::COMPLEMENT + 2]
            .copy_from_slice(&complement.to_le_bytes());
        rom[h + field::CHECKSUM..h + field::CHECKSUM + 2].copy_from_slice(&checksum.to_le_bytes());
        // reset vector -> $8000
        rom[h + field::RESET_VECTOR..h + field::RESET_VECTOR + 2]
            .copy_from_slice(&0x8000u16.to_le_bytes());
        rom
    }

    #[test]
    fn too_small_rejected() {
        assert_eq!(Header::detect(&[]), Err(HeaderError::TooSmall(0)));
    }

    #[test]
    fn detects_lorom() {
        let rom = synth_image(0x7FC0, 0x0, 0x03, 0x01); // 8 KiB SRAM, NTSC
        let h = Header::detect(&rom).expect("lorom header should detect");
        assert_eq!(h.map_mode, MapMode::LoRom);
        assert_eq!(h.offset, 0x7FC0);
        assert_eq!(h.region, Region::Ntsc);
        assert_eq!(h.sram_size, 0x2000);
        assert!(h.has_battery);
        assert_eq!(h.coprocessor, Coprocessor::None);
        assert_eq!(h.title, "RUSTYSNES TEST ROM");
    }

    #[test]
    fn title_replaces_non_printable_bytes_and_trims_padding() {
        let mut rom = synth_image(0x7FC0, 0x0, 0x00, 0x01);
        // "AB" followed by non-printable padding — non-printable bytes become spaces, then the
        // trailing run (padding + replaced bytes) trims off, leaving just "AB".
        rom[0x7FC0 + field::TITLE] = b'A';
        rom[0x7FC0 + field::TITLE + 1] = b'B';
        for i in 2..field::TITLE_LEN {
            rom[0x7FC0 + field::TITLE + i] = 0x00;
        }
        let h = Header::detect(&rom).expect("detect");
        assert_eq!(h.title, "AB");
    }

    #[test]
    fn detects_hirom() {
        let rom = synth_image(0xFFC0, 0x1, 0x00, 0x00); // no SRAM, Japan/NTSC
        let h = Header::detect(&rom).expect("hirom header should detect");
        assert_eq!(h.map_mode, MapMode::HiRom);
        assert_eq!(h.offset, 0xFFC0);
        assert_eq!(h.region, Region::Ntsc);
        assert_eq!(h.sram_size, 0);
        // chipset 0x00 -> no battery
        assert!(!h.has_battery);
    }

    #[test]
    fn pal_region_detected() {
        let rom = synth_image(0x7FC0, 0x0, 0x00, 0x02); // Europe
        let h = Header::detect(&rom).expect("detect");
        assert_eq!(h.region, Region::Pal);
    }

    #[test]
    fn hirom_outscores_lorom_when_both_offsets_present() {
        // A valid HiROM header at $FFC0; the $7FC0 window is zeroed (no checksum match), so
        // HiROM must win the score even though LoROM is the first candidate.
        let rom = synth_image(0xFFC0, 0x1, 0x00, 0x00);
        let h = Header::detect(&rom).expect("detect");
        assert_eq!(h.map_mode, MapMode::HiRom);
    }

    #[test]
    fn copier_prefix_stripped() {
        let core = synth_image(0x7FC0, 0x0, 0x00, 0x01);
        let mut rom = vec![0xAAu8; 0x200];
        rom.extend_from_slice(&core);
        assert_eq!(rom.len() % 0x8000, 0x200);
        let h = Header::detect(&rom).expect("detect after prefix strip");
        assert_eq!(h.copier_prefix, 0x200);
        assert_eq!(h.map_mode, MapMode::LoRom);
        assert_eq!(h.rom_size, 0x1_0000);
    }

    #[test]
    fn garbage_rejected() {
        let rom = vec![0u8; 0x1_0000];
        assert_eq!(Header::detect(&rom), Err(HeaderError::NoValidHeader));
    }

    #[test]
    fn detects_exlorom() {
        // ExLoROM has no dedicated $xFD5 nibble, so the header reports plain LoROM's ($0) —
        // disambiguated purely by the $40_7FC0 offset, only reachable by images >4 MiB.
        let mut rom = vec![0u8; 0x40_8000];
        let synth = synth_image(0x7FC0, 0x0, 0x00, 0x01);
        rom[0x40_0000..0x40_8000].copy_from_slice(&synth[0..0x8000]);
        let h = Header::detect(&rom).expect("exlorom header should detect");
        assert_eq!(h.map_mode, MapMode::ExLoRom);
        assert_eq!(h.offset, 0x40_7FC0);
    }

    #[test]
    fn srtc_detected_by_title_regardless_of_chipset_nibble() {
        // Daikaijuu Monogatari II declares chipset $55 (high nibble $5, not the $F custom family),
        // so the nibble gate would drop it — it must be title-detected up front. Real cart title:
        // DAIKAIJYUMONOGATARI2 (verified against a real dump).
        assert_eq!(
            coprocessor_from_chipset(0x55, "DAIKAIJYUMONOGATARI2 "),
            Coprocessor::Srtc
        );
        // A $55 chipset with any other title is nothing (the nibble alone carries no coprocessor).
        assert_eq!(
            coprocessor_from_chipset(0x55, "SOME OTHER RPG       "),
            Coprocessor::None
        );
    }

    #[test]
    fn sram_size_formula() {
        // byte 5 -> 0x400 << 5 = 32 KiB
        let rom = synth_image(0x7FC0, 0x0, 0x05, 0x01);
        let h = Header::detect(&rom).expect("detect");
        assert_eq!(h.sram_size, 0x8000);
    }

    #[test]
    fn a_forged_ram_size_byte_is_clamped_not_shifted_unbounded() {
        // Found by `fuzz/fuzz_targets/rom_header.rs`. `$xFD8` is `1 << N` KiB with `N` an arbitrary
        // byte from an untrusted image, and `1024 << N` was applied straight to it: a debug panic
        // for `N >= 64`, and in release a masked shift that hands `board::select` a multi-gigabyte
        // `vec![0u8; header.sram_size]`. On `wasm32` (`usize` is 32 bits) both start far earlier.
        //
        // Every byte must produce a size no larger than the biggest window any SNES board can
        // address, and must not panic on the way there.
        for n in 0..=u8::MAX {
            let rom = synth_image(0x7FC0, 0x0, n, 0x01);
            let h = Header::detect(&rom).expect("detect must survive every RAM_SIZE byte");
            assert!(
                h.sram_size <= MAX_SRAM_SIZE,
                "RAM_SIZE byte {n:#04x} produced {} bytes, past the {MAX_SRAM_SIZE}-byte bound",
                h.sram_size
            );
        }

        // The specific value the fuzzer landed on: without the shift cap this is a 4 GiB request.
        let rom = synth_image(0x7FC0, 0x0, 22, 0x01);
        assert_eq!(
            Header::detect(&rom).expect("detect").sram_size,
            MAX_SRAM_SIZE
        );
    }

    #[test]
    fn the_sram_clamp_leaves_every_real_cartridge_size_untouched() {
        // The negative control for the test above. A clamp that also truncated legitimate carts
        // would pass "nothing exceeds the bound" while quietly shrinking real saves — so pin the
        // whole range of sizes actual hardware ships, from 2 KiB up to the 256 KiB of an SA-1
        // BW-RAM cart. These must be the exact `1 << N` KiB formula, not the bound.
        for (n, expected) in [
            (1u8, 2 * 1024),
            (2, 4 * 1024),
            (3, 8 * 1024),   // the common battery-save size
            (5, 32 * 1024),  // Super FX plot buffer
            (6, 64 * 1024),  // large RPG saves
            (8, 256 * 1024), // SA-1 BW-RAM, the largest real cart RAM
        ] {
            let rom = synth_image(0x7FC0, 0x0, n, 0x01);
            let h = Header::detect(&rom).expect("detect");
            assert_eq!(
                h.sram_size, expected,
                "RAM_SIZE byte {n} is a real cartridge size and must not be clamped"
            );
        }
    }
}
