//! The SA-1 board — the Super Accelerator system (a second 65C816 + ASIC) on the cartridge.
//!
//! SA-1 (Nintendo's "Super Accelerator 1") is the most capable on-cart coprocessor: a second WDC
//! 65C816 clocked at ~10.74 MHz (twice the S-CPU's fast rate) plus a support ASIC. Unlike the DSP
//! family there is **no chip-ROM dump** — the SA-1 program lives in the cartridge ROM, so the board
//! is functional the moment an SA-1 cart loads (`docs/cart.md`, `docs/adr/0003`). Titles: Super
//! Mario RPG, Kirby Super Star, Kirby's Dream Land 3, Marvelous, SD F-1 Grand Prix, …
//!
//! ## Why this board is split across two crates
//!
//! The one-directional chip-crate graph forbids `rustysnes-cart` from depending on `rustysnes-cpu`
//! (`docs/architecture.md`). So this board owns the entire **SA-1 system state** — the $2200–$23FF
//! register file, the Super-MMC ROM banking, BW-RAM (shared) + I-RAM (2 KiB internal), the
//! arithmetic unit, the (normal + character-conversion) DMA, the variable-length bit unit, and the
//! H/V timer — and exposes the SA-1 CPU's *memory view* through the [`crate::board::Board`]
//! second-CPU hooks. `rustysnes-core` (which already depends on `rustysnes-cpu`) instantiates the
//! second `rustysnes_cpu::Cpu`, wires a thin bus adapter to these hooks, and steps it in the
//! scheduler alongside the main CPU. See `docs/scheduler.md` §SA-1.
//!
//! ## Memory maps (clean-room from ares `sfc/coprocessor/sa1`, ISC)
//!
//! S-CPU (main) view — handled by [`Board::read24`]/[`Board::write24`]:
//!
//! | Region (banks : addr)              | Target                                    |
//! |------------------------------------|-------------------------------------------|
//! | `$00-$3F,$80-$BF : $2200-$23FF`    | SA-1 registers (S-CPU side)               |
//! | `$00-$3F,$80-$BF : $3000-$37FF`    | I-RAM (2 KiB)                             |
//! | `$00-$3F,$80-$BF : $6000-$7FFF`    | BW-RAM (8 KiB block, selected by `$2224`) |
//! | `$00-$3F,$80-$BF : $8000-$FFFF`    | ROM (Super-MMC blocks C/D)               |
//! | `$40-$4F : $0000-$FFFF`            | BW-RAM (linear)                          |
//! | `$C0-$FF : $0000-$FFFF`            | ROM (Super-MMC blocks via `$C/D/E/Fxx`)  |
//!
//! SA-1 (second-CPU) view — handled by the second-CPU hooks: the same ROM/BW-RAM/I-RAM/regs plus
//! the BW-RAM bitmap window (`$60-$6F`, 2/4 bpp) and the linear window (`$40-$5F`).

// Chip-name jargon (SA-1, BW-RAM, I-RAM, MMC, Super-MMC, …) is not Rust code; the four parallel
// MMC bank fields are deliberately similar names; SA-1 register math narrows widths at well-defined
// boundaries (matches the bus/exec cast-precision allowances). `if_not_else` / `branches_sharing_
// code` are allowed so the register/DMA/arithmetic logic stays structurally faithful to the ares
// reference (clean-room traceability); `missing_const_for_fn` is a nursery suggestion on the pure
// decode helpers.
#![allow(
    clippy::doc_markdown,
    clippy::similar_names,
    clippy::cast_possible_truncation,
    clippy::cast_lossless,
    clippy::cast_sign_loss,
    clippy::cast_possible_wrap,
    clippy::struct_excessive_bools,
    clippy::too_many_lines,
    clippy::if_not_else,
    clippy::branches_sharing_code,
    clippy::missing_const_for_fn
)]

use alloc::boxed::Box;
use alloc::vec;

use rustysnes_savestate::{SaveReader, SaveStateError, SaveWriter};

use crate::board::{Board, Coprocessor, MappedAddr};
use crate::header::Region;

/// I-RAM size — 2 KiB internal SA-1 RAM.
const IRAM_SIZE: usize = 0x800;
/// Default BW-RAM size when the header declares none (SA-1 always has shared work RAM); 256 KiB is
/// the SA-1 hardware maximum, so a generous default never under-allocates the linear/bitmap views.
const BWRAM_MAX: usize = 0x4_0000;

/// Fold a linear offset into a `size`-byte image with hardware-accurate mirroring (ares
/// `Bus::mirror`; the same algorithm as [`crate::board`]'s private `mirror`). Power-of-two sizes
/// reduce to a mask; the non-power-of-two tail mirrors within itself.
const fn mirror(mut address: u32, size: u32) -> u32 {
    if size == 0 {
        return 0;
    }
    let mut base = 0u32;
    let mut mask = 1u32 << 23;
    let mut size = size;
    while address >= size {
        while address & mask == 0 {
            mask >>= 1;
        }
        address -= mask;
        if size > mask {
            size -= mask;
            base += mask;
        }
        mask >>= 1;
    }
    base + address
}

/// The SA-1 $2200–$23FF register file (clean-room from ares `sa1.hpp` `IO`).
#[derive(Debug, Clone)]
struct Io {
    // $2200 CCNT — SA-1 control (written by S-CPU).
    sa1_irq: bool,
    sa1_rdyb: bool,
    sa1_resb: bool,
    sa1_nmi: bool,
    smeg: u8,
    // $2201 SIE / $2202 SIC — S-CPU interrupt enable/clear.
    cpu_irqen: bool,
    chdma_irqen: bool,
    cpu_irqcl: bool,
    chdma_irqcl: bool,
    // $2203-$2208 — SA-1 reset/NMI/IRQ vectors.
    crv: u16,
    cnv: u16,
    civ: u16,
    // $2209 SCNT — S-CPU control (written by SA-1).
    cpu_irq: bool,
    cpu_ivsw: bool,
    cpu_nvsw: bool,
    cmeg: u8,
    // $220A CIE / $220B CIC — SA-1 interrupt enable/clear.
    sa1_irqen: bool,
    timer_irqen: bool,
    dma_irqen: bool,
    sa1_nmien: bool,
    sa1_irqcl: bool,
    timer_irqcl: bool,
    dma_irqcl: bool,
    sa1_nmicl: bool,
    // $220C-$220F — S-CPU NMI/IRQ vectors (when redirected).
    snv: u16,
    siv: u16,
    // $2210 TMC / $2212-$2215 — H/V timer.
    hvselb: bool,
    ven: bool,
    hen: bool,
    hcnt: u16,
    vcnt: u16,
    // $2220-$2223 — Super-MMC bank registers.
    cbmode: bool,
    dbmode: bool,
    ebmode: bool,
    fbmode: bool,
    cb: u32,
    db: u32,
    eb: u32,
    fb: u32,
    // $2224 BMAPS / $2225 BMAP — BW-RAM block select (S-CPU / SA-1).
    sbm: u8,
    sw46: bool,
    cbm: u8,
    // $2226 SWBE / $2227 CWBE / $2228 BWPA — BW-RAM write enable + protect.
    swen: bool,
    cwen: bool,
    bwp: u8,
    // $2229 SIWP / $222A CIWP — I-RAM write protection.
    siwp: u8,
    ciwp: u8,
    // $2230 DCNT / $2231 CDMA — DMA control.
    dmaen: bool,
    dprio: bool,
    cden: bool,
    cdsel: bool,
    dd: u8,
    sd: u8,
    chdend: bool,
    dmasize: u8,
    dmacb: u8,
    // $2232-$2239 — DMA addresses + counter.
    dsa: u32,
    dda: u32,
    dtc: u16,
    // $223F BBF / $2240-$224F BRF — bitmap format + register file.
    bbf: bool,
    brf: [u8; 16],
    // $2250-$2254 — arithmetic unit.
    acm: bool,
    md: bool,
    ma: u16,
    mb: u16,
    // $2258-$225B — variable-length bit processing.
    hl: bool,
    vb: u8,
    va: u32,
    vbit: u8,
    // $2300 SFR / $2301 CFR — flag reads.
    cpu_irqfl: bool,
    chdma_irqfl: bool,
    sa1_irqfl: bool,
    timer_irqfl: bool,
    dma_irqfl: bool,
    sa1_nmifl: bool,
    // $2302-$2305 — latched H/V counters.
    hcr: u16,
    vcr: u16,
    // $2306-$230B — arithmetic result + overflow.
    mr: u64,
    overflow: bool,
}

impl Io {
    fn power() -> Self {
        Self {
            sa1_irq: false,
            sa1_rdyb: false,
            sa1_resb: true,
            sa1_nmi: false,
            smeg: 0,
            cpu_irqen: false,
            chdma_irqen: false,
            cpu_irqcl: false,
            chdma_irqcl: false,
            crv: 0,
            cnv: 0,
            civ: 0,
            cpu_irq: false,
            cpu_ivsw: false,
            cpu_nvsw: false,
            cmeg: 0,
            sa1_irqen: false,
            timer_irqen: false,
            dma_irqen: false,
            sa1_nmien: false,
            sa1_irqcl: false,
            timer_irqcl: false,
            dma_irqcl: false,
            sa1_nmicl: false,
            snv: 0,
            siv: 0,
            hvselb: false,
            ven: false,
            hen: false,
            hcnt: 0,
            vcnt: 0,
            cbmode: false,
            dbmode: false,
            ebmode: false,
            fbmode: false,
            cb: 0,
            db: 1,
            eb: 2,
            fb: 3,
            sbm: 0,
            sw46: false,
            cbm: 0,
            swen: false,
            cwen: false,
            bwp: 0x0f,
            siwp: 0,
            ciwp: 0,
            dmaen: false,
            dprio: false,
            cden: false,
            cdsel: false,
            dd: 0,
            sd: 0,
            chdend: false,
            dmasize: 0,
            dmacb: 0,
            dsa: 0,
            dda: 0,
            dtc: 0,
            bbf: false,
            brf: [0; 16],
            acm: false,
            md: false,
            ma: 0,
            mb: 0,
            hl: false,
            vb: 16,
            va: 0,
            vbit: 0,
            cpu_irqfl: false,
            chdma_irqfl: false,
            sa1_irqfl: false,
            timer_irqfl: false,
            dma_irqfl: false,
            sa1_nmifl: false,
            hcr: 0,
            vcr: 0,
            mr: 0,
            overflow: false,
        }
    }

    /// Write every register field, in declaration order, into the caller's section.
    fn save_state(&self, s: &mut SaveWriter) {
        s.write_bool(self.sa1_irq);
        s.write_bool(self.sa1_rdyb);
        s.write_bool(self.sa1_resb);
        s.write_bool(self.sa1_nmi);
        s.write_u8(self.smeg);
        s.write_bool(self.cpu_irqen);
        s.write_bool(self.chdma_irqen);
        s.write_bool(self.cpu_irqcl);
        s.write_bool(self.chdma_irqcl);
        s.write_u16(self.crv);
        s.write_u16(self.cnv);
        s.write_u16(self.civ);
        s.write_bool(self.cpu_irq);
        s.write_bool(self.cpu_ivsw);
        s.write_bool(self.cpu_nvsw);
        s.write_u8(self.cmeg);
        s.write_bool(self.sa1_irqen);
        s.write_bool(self.timer_irqen);
        s.write_bool(self.dma_irqen);
        s.write_bool(self.sa1_nmien);
        s.write_bool(self.sa1_irqcl);
        s.write_bool(self.timer_irqcl);
        s.write_bool(self.dma_irqcl);
        s.write_bool(self.sa1_nmicl);
        s.write_u16(self.snv);
        s.write_u16(self.siv);
        s.write_bool(self.hvselb);
        s.write_bool(self.ven);
        s.write_bool(self.hen);
        s.write_u16(self.hcnt);
        s.write_u16(self.vcnt);
        s.write_bool(self.cbmode);
        s.write_bool(self.dbmode);
        s.write_bool(self.ebmode);
        s.write_bool(self.fbmode);
        s.write_u32(self.cb);
        s.write_u32(self.db);
        s.write_u32(self.eb);
        s.write_u32(self.fb);
        s.write_u8(self.sbm);
        s.write_bool(self.sw46);
        s.write_u8(self.cbm);
        s.write_bool(self.swen);
        s.write_bool(self.cwen);
        s.write_u8(self.bwp);
        s.write_u8(self.siwp);
        s.write_u8(self.ciwp);
        s.write_bool(self.dmaen);
        s.write_bool(self.dprio);
        s.write_bool(self.cden);
        s.write_bool(self.cdsel);
        s.write_u8(self.dd);
        s.write_u8(self.sd);
        s.write_bool(self.chdend);
        s.write_u8(self.dmasize);
        s.write_u8(self.dmacb);
        s.write_u32(self.dsa);
        s.write_u32(self.dda);
        s.write_u16(self.dtc);
        s.write_bool(self.bbf);
        s.write_bytes(&self.brf);
        s.write_bool(self.acm);
        s.write_bool(self.md);
        s.write_u16(self.ma);
        s.write_u16(self.mb);
        s.write_bool(self.hl);
        s.write_u8(self.vb);
        s.write_u32(self.va);
        s.write_u8(self.vbit);
        s.write_bool(self.cpu_irqfl);
        s.write_bool(self.chdma_irqfl);
        s.write_bool(self.sa1_irqfl);
        s.write_bool(self.timer_irqfl);
        s.write_bool(self.dma_irqfl);
        s.write_bool(self.sa1_nmifl);
        s.write_u16(self.hcr);
        s.write_u16(self.vcr);
        s.write_u64(self.mr);
        s.write_bool(self.overflow);
    }

    /// The inverse of [`Self::save_state`].
    ///
    /// # Errors
    /// [`SaveStateError`] on truncated/corrupt input. Every field below that's masked/clamped on
    /// every normal register write (cited per-field against the exact write site) is masked or
    /// clamped identically on load, since several (`bwp`/`dmacb`/`dmasize` in particular) feed
    /// bit-shift amounts elsewhere in this board — an out-of-range value from a hand-edited or
    /// corrupted save-state would otherwise risk a shift-overflow panic.
    fn load_state(&mut self, s: &mut SaveReader) -> Result<(), SaveStateError> {
        self.sa1_irq = s.read_bool()?;
        self.sa1_rdyb = s.read_bool()?;
        self.sa1_resb = s.read_bool()?;
        self.sa1_nmi = s.read_bool()?;
        self.smeg = s.read_u8()? & 0x0F; // write_io_cpu: data & 0x0f
        self.cpu_irqen = s.read_bool()?;
        self.chdma_irqen = s.read_bool()?;
        self.cpu_irqcl = s.read_bool()?;
        self.chdma_irqcl = s.read_bool()?;
        self.crv = s.read_u16()?;
        self.cnv = s.read_u16()?;
        self.civ = s.read_u16()?;
        self.cpu_irq = s.read_bool()?;
        self.cpu_ivsw = s.read_bool()?;
        self.cpu_nvsw = s.read_bool()?;
        self.cmeg = s.read_u8()? & 0x0F; // write_io_sa1: data & 0x0f
        self.sa1_irqen = s.read_bool()?;
        self.timer_irqen = s.read_bool()?;
        self.dma_irqen = s.read_bool()?;
        self.sa1_nmien = s.read_bool()?;
        self.sa1_irqcl = s.read_bool()?;
        self.timer_irqcl = s.read_bool()?;
        self.dma_irqcl = s.read_bool()?;
        self.sa1_nmicl = s.read_bool()?;
        self.snv = s.read_u16()?;
        self.siv = s.read_u16()?;
        self.hvselb = s.read_bool()?;
        self.ven = s.read_bool()?;
        self.hen = s.read_bool()?;
        self.hcnt = s.read_u16()?;
        self.vcnt = s.read_u16()?;
        self.cbmode = s.read_bool()?;
        self.dbmode = s.read_bool()?;
        self.ebmode = s.read_bool()?;
        self.fbmode = s.read_bool()?;
        self.cb = s.read_u32()? & 0x07;
        self.db = s.read_u32()? & 0x07;
        self.eb = s.read_u32()? & 0x07;
        self.fb = s.read_u32()? & 0x07;
        self.sbm = s.read_u8()? & 0x1F;
        self.sw46 = s.read_bool()?;
        self.cbm = s.read_u8()? & 0x7F;
        self.swen = s.read_bool()?;
        self.cwen = s.read_bool()?;
        self.bwp = s.read_u8()? & 0x0F;
        self.siwp = s.read_u8()?;
        self.ciwp = s.read_u8()?;
        self.dmaen = s.read_bool()?;
        self.dprio = s.read_bool()?;
        self.cden = s.read_bool()?;
        self.cdsel = s.read_bool()?;
        self.dd = s.read_u8()? & 0x01;
        self.sd = s.read_u8()? & 0x03;
        self.chdend = s.read_bool()?;
        // dmasize/dmacb are masked then additionally clamped on every normal write (see
        // write_io_sa1's own `if > N { = N }` follow-up); 6-dmacb / 7-dmacb / 2-dmacb are used as
        // subtraction-then-shift amounts elsewhere in this board, so an unclamped restored value
        // risks the same subtraction-underflow-then-shift-overflow panic these clamps prevent.
        self.dmasize = (s.read_u8()? & 0x07).min(5);
        self.dmacb = (s.read_u8()? & 0x03).min(2);
        self.dsa = s.read_u32()? & 0xFF_FFFF;
        self.dda = s.read_u32()? & 0xFF_FFFF;
        self.dtc = s.read_u16()?;
        self.bbf = s.read_bool()?;
        self.brf.copy_from_slice(s.read_bytes(16)?);
        self.acm = s.read_bool()?;
        self.md = s.read_bool()?;
        self.ma = s.read_u16()?;
        self.mb = s.read_u16()?;
        self.hl = s.read_bool()?;
        // vb: write_io_sa1 masks with & 0x0f then maps a masked-to-zero result to 16 (never 0).
        let vb = s.read_u8()? & 0x0F;
        self.vb = if vb == 0 { 16 } else { vb };
        self.va = s.read_u32()? & 0xFF_FFFF;
        self.vbit = s.read_u8()? & 0x07;
        self.cpu_irqfl = s.read_bool()?;
        self.chdma_irqfl = s.read_bool()?;
        self.sa1_irqfl = s.read_bool()?;
        self.timer_irqfl = s.read_bool()?;
        self.dma_irqfl = s.read_bool()?;
        self.sa1_nmifl = s.read_bool()?;
        self.hcr = s.read_u16()?;
        self.vcr = s.read_u16()?;
        self.mr = s.read_u64()?;
        self.overflow = s.read_bool()?;
        Ok(())
    }
}

/// The SA-1 H/V timer counters (ares `Status`).
#[derive(Debug, Clone)]
struct Timer {
    scanlines: u16,
    vcounter: u16,
    hcounter: u16,
}

/// A cartridge carrying the SA-1 system: the S-CPU memory map plus the entire SA-1 ASIC + the
/// SA-1 CPU's memory view (the second 65C816 itself lives in `rustysnes-core`).
pub struct Sa1Board {
    rom: Box<[u8]>,
    bwram: Box<[u8]>,
    iram: Box<[u8; IRAM_SIZE]>,
    io: Io,
    timer: Timer,
    /// Set when the S-CPU clears RESB (1→0); consumed once by core to reset the SA-1 CPU.
    reset_pending: bool,
    /// `bwram.dma` — character-conversion type-1 DMA is staged; BW-RAM reads convert on the fly.
    bwram_dma: bool,
    /// `dma.line` — the type-2 character-conversion DMA line counter.
    dma_line: u8,
    /// Liveness / debugger counter: host (S-CPU) accesses to the SA-1 register window.
    host_accesses: u64,
}

impl core::fmt::Debug for Sa1Board {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("Sa1Board")
            .field("rom_len", &self.rom.len())
            .field("bwram_len", &self.bwram.len())
            .field("sa1_resb", &self.io.sa1_resb)
            .field("host_accesses", &self.host_accesses)
            .finish_non_exhaustive()
    }
}

impl Sa1Board {
    /// Build an SA-1 board over `rom`, sizing BW-RAM from the header (clamped up to the 256 KiB
    /// SA-1 maximum and rounded to a power of two so the linear/bitmap mirror views never wrap
    /// short). `region` sets the timer's scanline count (262 NTSC / 312 PAL).
    #[must_use]
    pub fn new(rom: Box<[u8]>, sram_size: usize, region: Region) -> Self {
        let bw_size = sram_size.max(BWRAM_MAX).next_power_of_two();
        let scanlines = match region {
            Region::Pal => 312,
            Region::Ntsc => 262,
        };
        Self {
            rom,
            bwram: vec![0u8; bw_size].into_boxed_slice(),
            iram: Box::new([0u8; IRAM_SIZE]),
            io: Io::power(),
            timer: Timer {
                scanlines,
                vcounter: 0,
                hcounter: 0,
            },
            reset_pending: false,
            bwram_dma: false,
            dma_line: 0,
            host_accesses: 0,
        }
    }

    // --- ROM (Super-MMC). ------------------------------------------------------------------

    fn rom_byte(&self, off: u32) -> u8 {
        let size = self.rom.len() as u32;
        self.rom
            .get(mirror(off, size) as usize)
            .copied()
            .unwrap_or(0)
    }

    /// Read a Super-MMC-packed ROM address (ares `ROM::readCPU`). `packed` carries the lo/hi flag
    /// in bit 22 and the 4 MiB linear offset in bits 0–21. Includes the S-CPU NMI/IRQ
    /// vector-redirect override (`$00:FFEA/EE` → SNV/SIV when armed).
    fn rom_read_packed(&self, packed: u32) -> u8 {
        // Reset/NMI/IRQ vector overrides ($00:FFE0-FFEF, packed as $007FE0-$007FEF).
        if packed & 0xff_ffe0 == 0x00_7fe0 {
            match packed {
                0x7fea if self.io.cpu_nvsw => return self.io.snv as u8,
                0x7feb if self.io.cpu_nvsw => return (self.io.snv >> 8) as u8,
                0x7fee if self.io.cpu_ivsw => return self.io.siv as u8,
                0x7fef if self.io.cpu_ivsw => return (self.io.siv >> 8) as u8,
                _ => {}
            }
        }
        let lo = packed < 0x40_0000;
        let address = packed & 0x3f_ffff;
        let block = address >> 20; // 0..3 selects C/D/E/F
        let within = address & 0x0f_ffff;
        let (mode, bank) = match block {
            0 => (self.io.cbmode, self.io.cb),
            1 => (self.io.dbmode, self.io.db),
            2 => (self.io.ebmode, self.io.eb),
            _ => (self.io.fbmode, self.io.fb),
        };
        if lo && !mode {
            self.rom_byte(address)
        } else {
            self.rom_byte((bank << 20) | within)
        }
    }

    /// Pack an S-CPU ROM address `(bank, addr)` into the MMC linear space.
    fn rom_cpu_packed(bank: u32, addr: u32) -> u32 {
        if bank >= 0xC0 {
            // HiROM-style: $C0-$FF:$0000-$FFFF (bit 22 set).
            let block = (bank - 0xC0) >> 4; // C0-CF=0, D0-DF=1, E0-EF=2, F0-FF=3
            let within = ((bank & 0x0F) << 16) | addr;
            0x40_0000 | (block << 20) | within
        } else {
            // LoROM-style window: $00-$1F=C, $20-$3F=D, $80-$9F=E, $A0-$BF=F.
            let block = match bank {
                0x00..=0x1F => 0,
                0x20..=0x3F => 1,
                0x80..=0x9F => 2,
                _ => 3,
            };
            let within = ((bank & 0x1F) << 15) | (addr & 0x7FFF);
            (block << 20) | within
        }
    }

    fn rom_read_cpu(&self, bank: u32, addr: u32) -> u8 {
        self.rom_read_packed(Self::rom_cpu_packed(bank, addr))
    }

    /// SA-1-view ROM read (ares `ROM::readSA1`): translate the SA-1 `$8000-$FFFF` window, then run
    /// the shared MMC decode.
    fn rom_read_sa1(&self, address: u32) -> u8 {
        let packed = if address & 0x40_8000 == 0x00_8000 {
            ((address & 0x80_0000) >> 2) | ((address & 0x3f_0000) >> 1) | (address & 0x00_7fff)
        } else {
            address
        };
        self.rom_read_packed(packed)
    }

    // --- BW-RAM. ---------------------------------------------------------------------------

    fn bwram_read(&self, off: u32) -> u8 {
        if self.bwram.is_empty() {
            return 0;
        }
        let size = self.bwram.len() as u32;
        self.bwram
            .get(mirror(off, size) as usize)
            .copied()
            .unwrap_or(0)
    }

    fn bwram_write(&mut self, off: u32, val: u8) {
        if self.bwram.is_empty() {
            return;
        }
        let size = self.bwram.len() as u32;
        let idx = mirror(off, size) as usize;
        if let Some(slot) = self.bwram.get_mut(idx) {
            *slot = val;
        }
    }

    /// BW-RAM bitmap read (ares `BWRAM::readBitmap`): 2/4 bpp virtual unpack of `$60-$6F`.
    fn bwram_read_bitmap(&self, address: u32) -> u8 {
        if !self.io.bbf {
            // 4 bpp
            let shift = address & 1;
            let byte = self.bwram_read(address >> 1);
            if shift == 0 { byte & 0x0F } else { byte >> 4 }
        } else {
            // 2 bpp
            let shift = address & 3;
            let byte = self.bwram_read(address >> 2);
            (byte >> (shift * 2)) & 0x03
        }
    }

    /// BW-RAM bitmap write (ares `BWRAM::writeBitmap`).
    fn bwram_write_bitmap(&mut self, address: u32, val: u8) {
        if !self.io.bbf {
            let shift = address & 1;
            let a = address >> 1;
            let cur = self.bwram_read(a);
            let data = if shift == 0 {
                (cur & 0xF0) | (val & 0x0F)
            } else {
                (cur & 0x0F) | ((val & 0x0F) << 4)
            };
            self.bwram_write(a, data);
        } else {
            let shift = address & 3;
            let a = address >> 2;
            let cur = self.bwram_read(a);
            let mask = !(0x03u8 << (shift * 2));
            let data = (cur & mask) | ((val & 0x03) << (shift * 2));
            self.bwram_write(a, data);
        }
    }

    /// BW-RAM linear write (ares `BWRAM::writeLinear`) with the SWEN/CWEN/BWPA write-protect.
    fn bwram_write_linear(&mut self, off: u32, val: u8) {
        if !self.io.swen && !self.io.cwen && (off & 0x3_ffff) < (0x100u32 << self.io.bwp) {
            return;
        }
        self.bwram_write(off, val);
    }

    fn bwram_read_cpu(&mut self, bank: u32, addr: u32) -> u8 {
        if (0x6000..=0x7FFF).contains(&addr) {
            let off = self.io.sbm as u32 * 0x2000 + (addr & 0x1FFF);
            if self.bwram_dma {
                return self.dma_cc1_read(off);
            }
            return self.bwram_read(off);
        }
        // $40-$4F linear.
        let raw = (bank << 16) | addr;
        self.bwram_read(raw)
    }

    fn bwram_write_cpu(&mut self, bank: u32, addr: u32, val: u8) {
        let off = if (0x6000..=0x7FFF).contains(&addr) {
            self.io.sbm as u32 * 0x2000 + (addr & 0x1FFF)
        } else {
            (bank << 16) | addr
        };
        if !self.io.swen && !self.io.cwen && (off & 0x3_ffff) < (0x100u32 << self.io.bwp) {
            return;
        }
        self.bwram_write(off, val);
    }

    /// SA-1-view BW-RAM read (ares `memory.cpp` read() BW-RAM branch + `BWRAM::readSA1`).
    fn bwram_read_sa1(&self, address: u32) -> u8 {
        if address & 0x40_0000 != 0 && address & 0x20_0000 != 0 {
            // $60-$6F bitmap window.
            return self.bwram_read_bitmap(address & 0x0f_ffff);
        }
        if address & 0x40_0000 != 0 {
            // $40-$5F linear.
            return self.bwram_read(address);
        }
        // $6000-$7FFF window.
        if !self.io.sw46 {
            self.bwram_read((self.io.cbm as u32 & 0x1f) * 0x2000 + (address & 0x1fff))
        } else {
            self.bwram_read_bitmap(self.io.cbm as u32 * 0x2000 + (address & 0x1fff))
        }
    }

    fn bwram_write_sa1(&mut self, address: u32, val: u8) {
        if address & 0x40_0000 != 0 && address & 0x20_0000 != 0 {
            self.bwram_write_bitmap(address & 0x0f_ffff, val);
            return;
        }
        if address & 0x40_0000 != 0 {
            self.bwram_write_linear(address, val);
            return;
        }
        if !self.io.sw46 {
            self.bwram_write_linear(
                (self.io.cbm as u32 & 0x1f) * 0x2000 + (address & 0x1fff),
                val,
            );
        } else {
            self.bwram_write_bitmap(self.io.cbm as u32 * 0x2000 + (address & 0x1fff), val);
        }
    }

    // --- I-RAM. ----------------------------------------------------------------------------

    fn iram_read(&self, off: u32) -> u8 {
        self.iram[(off & 0x7FF) as usize]
    }

    fn iram_write(&mut self, off: u32, val: u8) {
        self.iram[(off & 0x7FF) as usize] = val;
    }

    fn iram_write_cpu(&mut self, off: u32, val: u8) {
        let block = (off >> 8) & 0x7;
        if self.io.siwp & (1 << block) == 0 {
            return;
        }
        self.iram_write(off, val);
    }

    fn iram_write_sa1(&mut self, off: u32, val: u8) {
        let block = (off >> 8) & 0x7;
        if self.io.ciwp & (1 << block) == 0 {
            return;
        }
        self.iram_write(off, val);
    }

    // --- Variable-length bit read (ares `readVBR`). ----------------------------------------

    fn read_vbr(&self, address: u32) -> u8 {
        if address & 0x40_8000 == 0x00_8000 || address & 0xc0_0000 == 0xc0_0000 {
            return self.rom_read_sa1(address);
        }
        if address & 0x40_e000 == 0x00_6000 || address & 0xf0_0000 == 0x40_0000 {
            return self.bwram_read(address);
        }
        if address & 0x40_f800 == 0x00_0000 || address & 0x40_f800 == 0x00_3000 {
            return self.iram_read(address);
        }
        0xff
    }

    // --- Arithmetic unit ($2254 trigger). --------------------------------------------------

    fn arith_trigger(&mut self) {
        if !self.io.acm {
            if !self.io.md {
                // signed multiplication
                self.io.mr =
                    ((self.io.ma as i16 as i32) * (self.io.mb as i16 as i32)) as u32 as u64;
                self.io.mb = 0;
            } else {
                // unsigned division
                if self.io.mb == 0 {
                    self.io.mr = 0;
                } else {
                    let dividend = self.io.ma as i16 as i32;
                    let divisor = self.io.mb as i32;
                    let remainder = if dividend >= 0 {
                        (dividend % divisor) as u16
                    } else {
                        (((dividend % divisor) + divisor) % divisor) as u16
                    };
                    let quotient = ((dividend - remainder as i32) / divisor) as u16;
                    self.io.mr = ((remainder as u64) << 16) | quotient as u64;
                }
                self.io.ma = 0;
                self.io.mb = 0;
            }
        } else {
            // cumulative sum (sigma)
            let prod = (self.io.ma as i16 as i64) * (self.io.mb as i16 as i64);
            self.io.mr = (self.io.mr as i64).wrapping_add(prod) as u64;
            self.io.overflow = (self.io.mr >> 40) & 1 != 0;
            self.io.mr &= 0xff_ffff_ffff;
            self.io.mb = 0;
        }
    }

    // --- Timer (ares `SA1::step`). ---------------------------------------------------------

    fn timer_trigger_irq(&mut self) {
        self.io.timer_irqfl = true;
        if self.io.timer_irqen {
            self.io.timer_irqcl = false;
        }
    }

    /// Advance the H/V timer by `clocks` SA-1 master clocks (always even; 2 clocks per CPU cycle),
    /// firing the timer IRQ on an exact comparator match.
    fn tick(&mut self, clocks: u32) {
        let mut remaining = clocks;
        while remaining >= 2 {
            remaining -= 2;
            if !self.io.hvselb {
                self.timer.hcounter = self.timer.hcounter.wrapping_add(2);
                if self.timer.hcounter >= 1364 {
                    self.timer.hcounter = 0;
                    self.timer.vcounter = self.timer.vcounter.wrapping_add(1);
                    if self.timer.vcounter >= self.timer.scanlines {
                        self.timer.vcounter = 0;
                    }
                }
            } else {
                self.timer.hcounter = self.timer.hcounter.wrapping_add(2);
                self.timer.vcounter = self.timer.vcounter.wrapping_add(self.timer.hcounter >> 11);
                self.timer.hcounter &= 0x07ff;
                self.timer.vcounter &= 0x01ff;
            }
            match (self.io.hen, self.io.ven) {
                (false, false) => {}
                (true, false) => {
                    if self.timer.hcounter == self.io.hcnt << 2 {
                        self.timer_trigger_irq();
                    }
                }
                (false, true) => {
                    if self.timer.vcounter == self.io.vcnt && self.timer.hcounter == 0 {
                        self.timer_trigger_irq();
                    }
                }
                (true, true) => {
                    if self.timer.vcounter == self.io.vcnt
                        && self.timer.hcounter == self.io.hcnt << 2
                    {
                        self.timer_trigger_irq();
                    }
                }
            }
        }
    }

    // --- DMA (ares `dma.cpp`). -------------------------------------------------------------

    /// Normal (direct) DMA: ROM/BW-RAM/I-RAM → BW-RAM/I-RAM, `dtc` bytes.
    fn dma_normal(&mut self) {
        const SOURCE_ROM: u8 = 0;
        const SOURCE_BWRAM: u8 = 1;
        const SOURCE_IRAM: u8 = 2;
        const DEST_IRAM: u8 = 0;
        const DEST_BWRAM: u8 = 1;
        while self.io.dtc != 0 {
            self.io.dtc = self.io.dtc.wrapping_sub(1);
            let source = self.io.dsa;
            let target = self.io.dda;
            self.io.dsa = self.io.dsa.wrapping_add(1) & 0xff_ffff;
            self.io.dda = self.io.dda.wrapping_add(1) & 0xff_ffff;
            match (self.io.sd, self.io.dd) {
                (SOURCE_ROM, DEST_BWRAM) => {
                    let data = self.rom_read_sa1(source);
                    self.bwram_write(target, data);
                }
                (SOURCE_ROM, DEST_IRAM) => {
                    let data = self.rom_read_sa1(source);
                    self.iram_write(target, data);
                }
                (SOURCE_BWRAM, DEST_IRAM) => {
                    let data = self.bwram_read(source);
                    self.iram_write(target, data);
                }
                (SOURCE_IRAM, DEST_BWRAM) => {
                    let data = self.iram_read(source);
                    self.bwram_write(target, data);
                }
                _ => {}
            }
        }
        self.io.dma_irqfl = true;
        if self.io.dma_irqen {
            self.io.dma_irqcl = false;
        }
    }

    /// Type-1 character-conversion DMA arm (ares `dmaCC1`): stages BW-RAM-side conversion and
    /// raises the character-conversion IRQ to the S-CPU.
    fn dma_cc1(&mut self) {
        self.bwram_dma = true;
        self.io.chdma_irqfl = true;
        if self.io.chdma_irqen {
            self.io.chdma_irqcl = false;
        }
    }

    /// Type-1 character-conversion BW-RAM read (ares `dmaCC1Read`): on a character boundary,
    /// transcode the next tile from BW-RAM (linear) into I-RAM (planar), then serve from I-RAM.
    fn dma_cc1_read(&mut self, address: u32) -> u8 {
        let charmask = (1u32 << (6 - self.io.dmacb)) - 1;
        if address & charmask == 0 {
            let bpp = 2u32 << (2 - self.io.dmacb);
            let bpl = (8u32 << self.io.dmasize) >> self.io.dmacb;
            let bwmask = self.bwram.len() as u32 - 1;
            let tile = ((address.wrapping_sub(self.io.dsa)) & bwmask) >> (6 - self.io.dmacb);
            let ty = tile >> self.io.dmasize;
            let tx = tile & ((1 << self.io.dmasize) - 1);
            let mut bwaddr = self.io.dsa + ty * 8 * bpl + tx * bpp;
            for y in 0..8u32 {
                let mut data: u64 = 0;
                for byte in 0..bpp {
                    data |= (self.bwram_read((bwaddr + byte) & bwmask) as u64) << (byte << 3);
                }
                bwaddr += bpl;
                let mut out = [0u8; 8];
                for x in 0..8u32 {
                    out[0] |= ((data & 1) as u8) << (7 - x);
                    data >>= 1;
                    out[1] |= ((data & 1) as u8) << (7 - x);
                    data >>= 1;
                    if self.io.dmacb == 2 {
                        continue;
                    }
                    out[2] |= ((data & 1) as u8) << (7 - x);
                    data >>= 1;
                    out[3] |= ((data & 1) as u8) << (7 - x);
                    data >>= 1;
                    if self.io.dmacb == 1 {
                        continue;
                    }
                    out[4] |= ((data & 1) as u8) << (7 - x);
                    data >>= 1;
                    out[5] |= ((data & 1) as u8) << (7 - x);
                    data >>= 1;
                    out[6] |= ((data & 1) as u8) << (7 - x);
                    data >>= 1;
                    out[7] |= ((data & 1) as u8) << (7 - x);
                    data >>= 1;
                }
                for byte in 0..bpp {
                    let p = self.io.dda + (y << 1) + ((byte & 6) << 3) + (byte & 1);
                    self.iram_write(p & 0x07ff, out[byte as usize]);
                }
            }
        }
        self.iram_read((self.io.dda + (address & charmask)) & 0x07ff)
    }

    /// Type-2 character-conversion DMA (ares `dmaCC2`): pack the bitmap register file into I-RAM
    /// planar form, advancing the line counter.
    fn dma_cc2(&mut self) {
        let base = ((self.dma_line & 1) << 3) as usize;
        let bpp = 2u32 << (2 - self.io.dmacb);
        let mut address = self.io.dda & 0x07ff;
        address &= !((1u32 << (7 - self.io.dmacb)) - 1);
        address += (self.dma_line as u32 & 8) * bpp;
        address += (self.dma_line as u32 & 7) * 2;
        for byte in 0..bpp {
            let mut output = 0u8;
            for bit in 0..8u32 {
                output |= ((self.io.brf[base + bit as usize] >> byte) & 1) << (7 - bit);
            }
            self.iram_write(address + ((byte & 6) << 3) + (byte & 1), output);
        }
        self.dma_line = (self.dma_line + 1) & 15;
    }

    // --- I/O register access (ares `io.cpp`). ----------------------------------------------

    fn read_io_cpu(&self, address: u32) -> u8 {
        match 0x2200 | (address & 0x1FF) {
            // (SFR) S-CPU flag read.
            0x2300 => {
                (self.io.cmeg & 0x0f)
                    | (u8::from(self.io.cpu_nvsw) << 4)
                    | (u8::from(self.io.chdma_irqfl) << 5)
                    | (u8::from(self.io.cpu_ivsw) << 6)
                    | (u8::from(self.io.cpu_irqfl) << 7)
            }
            _ => 0,
        }
    }

    fn read_io_sa1(&mut self, address: u32) -> u8 {
        match 0x2200 | (address & 0x1FF) {
            // (CFR) SA-1 flag read.
            0x2301 => {
                (self.io.smeg & 0x0f)
                    | (u8::from(self.io.sa1_nmifl) << 4)
                    | (u8::from(self.io.dma_irqfl) << 5)
                    | (u8::from(self.io.timer_irqfl) << 6)
                    | (u8::from(self.io.sa1_irqfl) << 7)
            }
            // (HCR/VCR) latch counters then read low byte.
            0x2302 => {
                self.io.hcr = self.timer.hcounter >> 2;
                self.io.vcr = self.timer.vcounter;
                self.io.hcr as u8
            }
            0x2303 => (self.io.hcr >> 8) as u8,
            0x2304 => self.io.vcr as u8,
            0x2305 => (self.io.vcr >> 8) as u8,
            // (MR) arithmetic result, 5 bytes.
            0x2306 => self.io.mr as u8,
            0x2307 => (self.io.mr >> 8) as u8,
            0x2308 => (self.io.mr >> 16) as u8,
            0x2309 => (self.io.mr >> 24) as u8,
            0x230a => (self.io.mr >> 32) as u8,
            // (OF) overflow.
            0x230b => u8::from(self.io.overflow) << 7,
            // (VDPL/VDPH) variable-length data read.
            0x230c => {
                let data = self.varlen_word();
                data as u8
            }
            0x230d => {
                let data = self.varlen_word();
                if self.io.hl {
                    self.io.vbit += self.io.vb;
                    self.io.va = self.io.va.wrapping_add((self.io.vbit >> 3) as u32) & 0xff_ffff;
                    self.io.vbit &= 7;
                }
                (data >> 8) as u8
            }
            _ => 0xff,
        }
    }

    fn varlen_word(&self) -> u32 {
        let mut data = self.read_vbr(self.io.va) as u32;
        data |= (self.read_vbr(self.io.va.wrapping_add(1) & 0xff_ffff) as u32) << 8;
        data |= (self.read_vbr(self.io.va.wrapping_add(2) & 0xff_ffff) as u32) << 16;
        data >> self.io.vbit
    }

    fn write_io_cpu(&mut self, address: u32, data: u8) {
        match 0x2200 | (address & 0x1FF) {
            // (CCNT) SA-1 control.
            0x2200 => {
                if self.io.sa1_resb && data & 0x20 == 0 {
                    // RESB 1→0: reset the SA-1 CPU.
                    self.reset_pending = true;
                    self.io.ciwp = 0x00;
                }
                self.io.smeg = data & 0x0f;
                self.io.sa1_nmi = data & 0x10 != 0;
                self.io.sa1_resb = data & 0x20 != 0;
                self.io.sa1_rdyb = data & 0x40 != 0;
                self.io.sa1_irq = data & 0x80 != 0;
                if self.io.sa1_irq {
                    self.io.sa1_irqfl = true;
                    if self.io.sa1_irqen {
                        self.io.sa1_irqcl = false;
                    }
                }
                if self.io.sa1_nmi {
                    self.io.sa1_nmifl = true;
                    if self.io.sa1_nmien {
                        self.io.sa1_nmicl = false;
                    }
                }
            }
            // (SIE) S-CPU interrupt enable.
            0x2201 => {
                self.io.chdma_irqen = data & 0x20 != 0;
                self.io.cpu_irqen = data & 0x80 != 0;
            }
            // (SIC) S-CPU interrupt clear.
            0x2202 => {
                self.io.chdma_irqcl = data & 0x20 != 0;
                self.io.cpu_irqcl = data & 0x80 != 0;
                if self.io.chdma_irqcl {
                    self.io.chdma_irqfl = false;
                }
                if self.io.cpu_irqcl {
                    self.io.cpu_irqfl = false;
                }
            }
            // (CRV) SA-1 reset vector.
            0x2203 => self.io.crv = (self.io.crv & 0xff00) | data as u16,
            0x2204 => self.io.crv = (self.io.crv & 0x00ff) | ((data as u16) << 8),
            // (CNV) SA-1 NMI vector.
            0x2205 => self.io.cnv = (self.io.cnv & 0xff00) | data as u16,
            0x2206 => self.io.cnv = (self.io.cnv & 0x00ff) | ((data as u16) << 8),
            // (CIV) SA-1 IRQ vector.
            0x2207 => self.io.civ = (self.io.civ & 0xff00) | data as u16,
            0x2208 => self.io.civ = (self.io.civ & 0x00ff) | ((data as u16) << 8),
            // (CXB/DXB/EXB/FXB) Super-MMC banks.
            0x2220 => {
                self.io.cb = (data & 0x07) as u32;
                self.io.cbmode = data & 0x80 != 0;
            }
            0x2221 => {
                self.io.db = (data & 0x07) as u32;
                self.io.dbmode = data & 0x80 != 0;
            }
            0x2222 => {
                self.io.eb = (data & 0x07) as u32;
                self.io.ebmode = data & 0x80 != 0;
            }
            0x2223 => {
                self.io.fb = (data & 0x07) as u32;
                self.io.fbmode = data & 0x80 != 0;
            }
            // (BMAPS) S-CPU BW-RAM block.
            0x2224 => self.io.sbm = data & 0x1f,
            // (SWBE) S-CPU BW-RAM write enable.
            0x2226 => self.io.swen = data & 0x80 != 0,
            // (BWPA) BW-RAM write-protect area.
            0x2228 => self.io.bwp = data & 0x0f,
            // (SIWP) S-CPU I-RAM write protect.
            0x2229 => self.io.siwp = data,
            0x2231..=0x2237 => self.write_io_shared(address, data),
            _ => {}
        }
    }

    fn write_io_sa1(&mut self, address: u32, data: u8) {
        match 0x2200 | (address & 0x1FF) {
            // (SCNT) S-CPU control.
            0x2209 => {
                self.io.cmeg = data & 0x0f;
                self.io.cpu_nvsw = data & 0x10 != 0;
                self.io.cpu_ivsw = data & 0x40 != 0;
                self.io.cpu_irq = data & 0x80 != 0;
                if self.io.cpu_irq {
                    self.io.cpu_irqfl = true;
                    if self.io.cpu_irqen {
                        self.io.cpu_irqcl = false;
                    }
                }
            }
            // (CIE) SA-1 interrupt enable.
            0x220a => {
                self.io.sa1_nmien = data & 0x10 != 0;
                self.io.dma_irqen = data & 0x20 != 0;
                self.io.timer_irqen = data & 0x40 != 0;
                self.io.sa1_irqen = data & 0x80 != 0;
            }
            // (CIC) SA-1 interrupt clear.
            0x220b => {
                self.io.sa1_nmicl = data & 0x10 != 0;
                self.io.dma_irqcl = data & 0x20 != 0;
                self.io.timer_irqcl = data & 0x40 != 0;
                self.io.sa1_irqcl = data & 0x80 != 0;
                if self.io.sa1_nmicl {
                    self.io.sa1_nmifl = false;
                }
                if self.io.sa1_irqcl {
                    self.io.sa1_irqfl = false;
                }
                if self.io.timer_irqcl {
                    self.io.timer_irqfl = false;
                }
                if self.io.dma_irqcl {
                    self.io.dma_irqfl = false;
                }
            }
            // (SNV) S-CPU NMI vector.
            0x220c => self.io.snv = (self.io.snv & 0xff00) | data as u16,
            0x220d => self.io.snv = (self.io.snv & 0x00ff) | ((data as u16) << 8),
            // (SIV) S-CPU IRQ vector.
            0x220e => self.io.siv = (self.io.siv & 0xff00) | data as u16,
            0x220f => self.io.siv = (self.io.siv & 0x00ff) | ((data as u16) << 8),
            // (TMC) H/V timer control.
            0x2210 => {
                self.io.hen = data & 0x01 != 0;
                self.io.ven = data & 0x02 != 0;
                self.io.hvselb = data & 0x80 != 0;
            }
            // (CTR) SA-1 timer restart.
            0x2211 => {
                self.timer.vcounter = 0;
                self.timer.hcounter = 0;
            }
            // (HCNT/VCNT).
            0x2212 => self.io.hcnt = (self.io.hcnt & 0xff00) | data as u16,
            0x2213 => self.io.hcnt = (self.io.hcnt & 0x00ff) | ((data as u16) << 8),
            0x2214 => self.io.vcnt = (self.io.vcnt & 0xff00) | data as u16,
            0x2215 => self.io.vcnt = (self.io.vcnt & 0x00ff) | ((data as u16) << 8),
            // (BMAP) SA-1 BW-RAM block.
            0x2225 => {
                self.io.cbm = data & 0x7f;
                self.io.sw46 = data & 0x80 != 0;
            }
            // (CWBE) SA-1 BW-RAM write enable.
            0x2227 => self.io.cwen = data & 0x80 != 0,
            // (CIWP) SA-1 I-RAM write protect.
            0x222a => self.io.ciwp = data,
            // (DCNT) DMA control.
            0x2230 => {
                self.io.sd = data & 0x03;
                self.io.dd = (data >> 2) & 1;
                self.io.cdsel = data & 0x10 != 0;
                self.io.cden = data & 0x20 != 0;
                self.io.dprio = data & 0x40 != 0;
                self.io.dmaen = data & 0x80 != 0;
                if !self.io.dmaen {
                    self.dma_line = 0;
                }
            }
            0x2231..=0x2237 => self.write_io_shared(address, data),
            // (DTC) DMA terminal counter.
            0x2238 => self.io.dtc = (self.io.dtc & 0xff00) | data as u16,
            0x2239 => self.io.dtc = (self.io.dtc & 0x00ff) | ((data as u16) << 8),
            // (BBF) bitmap format.
            0x223f => self.io.bbf = data & 0x80 != 0,
            // (BRF) bitmap register files.
            0x2240..=0x224f => {
                let idx = (address & 0x0f) as usize;
                self.io.brf[idx] = data;
                if (idx == 7 || idx == 15) && self.io.dmaen && self.io.cden && !self.io.cdsel {
                    self.dma_cc2();
                }
            }
            // (MCNT) arithmetic control.
            0x2250 => {
                self.io.md = data & 0x01 != 0;
                self.io.acm = data & 0x02 != 0;
                if self.io.acm {
                    self.io.mr = 0;
                }
            }
            // (MA/MB) operands.
            0x2251 => self.io.ma = (self.io.ma & 0xff00) | data as u16,
            0x2252 => self.io.ma = (self.io.ma & 0x00ff) | ((data as u16) << 8),
            0x2253 => self.io.mb = (self.io.mb & 0xff00) | data as u16,
            0x2254 => {
                self.io.mb = (self.io.mb & 0x00ff) | ((data as u16) << 8);
                self.arith_trigger();
            }
            // (VBD) variable-length bit control.
            0x2258 => {
                self.io.vb = data & 0x0f;
                self.io.hl = data & 0x80 != 0;
                if self.io.vb == 0 {
                    self.io.vb = 16;
                }
                if !self.io.hl {
                    self.io.vbit += self.io.vb;
                    self.io.va = self.io.va.wrapping_add((self.io.vbit >> 3) as u32) & 0xff_ffff;
                    self.io.vbit &= 7;
                }
            }
            // (VDA) variable-length start address.
            0x2259 => self.io.va = (self.io.va & 0xff_ff00) | data as u32,
            0x225a => self.io.va = (self.io.va & 0xff_00ff) | ((data as u32) << 8),
            0x225b => {
                self.io.va = (self.io.va & 0x00_ffff) | ((data as u32) << 16);
                self.io.vbit = 0;
            }
            _ => {}
        }
    }

    fn write_io_shared(&mut self, address: u32, data: u8) {
        match 0x2200 | (address & 0x1FF) {
            // (CDMA) character-conversion DMA parameters.
            0x2231 => {
                self.io.dmacb = data & 0x03;
                self.io.dmasize = (data >> 2) & 0x07;
                self.io.chdend = data & 0x80 != 0;
                if self.io.dmacb > 2 {
                    self.io.dmacb = 2;
                }
                if self.io.dmasize > 5 {
                    self.io.dmasize = 5;
                }
                if self.io.chdend {
                    self.bwram_dma = false;
                }
            }
            // (SDA) DMA source.
            0x2232 => self.io.dsa = (self.io.dsa & 0xff_ff00) | data as u32,
            0x2233 => self.io.dsa = (self.io.dsa & 0xff_00ff) | ((data as u32) << 8),
            0x2234 => self.io.dsa = (self.io.dsa & 0x00_ffff) | ((data as u32) << 16),
            // (DDA) DMA destination.
            0x2235 => self.io.dda = (self.io.dda & 0xff_ff00) | data as u32,
            0x2236 => {
                self.io.dda = (self.io.dda & 0xff_00ff) | ((data as u32) << 8);
                if self.io.dmaen {
                    if !self.io.cden && self.io.dd == 0 {
                        self.dma_normal();
                    } else if self.io.cden && self.io.cdsel {
                        self.dma_cc1();
                    }
                }
            }
            0x2237 => {
                self.io.dda = (self.io.dda & 0x00_ffff) | ((data as u32) << 16);
                if self.io.dmaen && !self.io.cden && self.io.dd == 1 {
                    self.dma_normal();
                }
            }
            _ => {}
        }
    }

    // --- The S-CPU (main) memory decode ($00-$3F/$80-$BF + $40-$4F + $C0-$FF). --------------

    fn classify_cpu(addr24: u32) -> CpuRegion {
        let bank = (addr24 >> 16) & 0xFF;
        let addr = addr24 & 0xFFFF;
        let lo = bank & 0x7F; // fold $80-$BF onto $00-$3F for the windowed regions
        if lo <= 0x3F {
            if (0x2200..=0x23FF).contains(&addr) {
                return CpuRegion::Io;
            }
            if (0x3000..=0x37FF).contains(&addr) {
                return CpuRegion::Iram(addr & 0x7FF);
            }
            if (0x6000..=0x7FFF).contains(&addr) {
                return CpuRegion::Bwram;
            }
            if addr >= 0x8000 {
                return CpuRegion::Rom;
            }
        }
        if (0x40..=0x4F).contains(&bank) {
            return CpuRegion::Bwram;
        }
        if bank >= 0xC0 {
            return CpuRegion::Rom;
        }
        CpuRegion::Open
    }
}

/// What a 24-bit S-CPU address decodes to on an SA-1 board.
enum CpuRegion {
    Io,
    Rom,
    Bwram,
    Iram(u32),
    Open,
}

impl Board for Sa1Board {
    fn name(&self) -> &'static str {
        "SA-1"
    }

    fn coprocessor(&self) -> Coprocessor {
        Coprocessor::Sa1
    }

    fn map(&self, addr24: u32) -> MappedAddr {
        match Self::classify_cpu(addr24) {
            CpuRegion::Io | CpuRegion::Iram(_) => MappedAddr::Coprocessor,
            CpuRegion::Rom => MappedAddr::Rom(0),
            CpuRegion::Bwram => MappedAddr::Sram(0),
            CpuRegion::Open => MappedAddr::Open,
        }
    }

    fn read24(&mut self, addr24: u32) -> u8 {
        let bank = (addr24 >> 16) & 0xFF;
        let addr = addr24 & 0xFFFF;
        match Self::classify_cpu(addr24) {
            CpuRegion::Io => {
                self.host_accesses = self.host_accesses.wrapping_add(1);
                self.read_io_cpu(addr24)
            }
            CpuRegion::Rom => {
                let b = if bank >= 0xC0 { bank } else { bank & 0x7F };
                self.rom_read_cpu(b, addr)
            }
            CpuRegion::Bwram => {
                let b = if (0x40..=0x4F).contains(&bank) {
                    bank
                } else {
                    bank & 0x7F
                };
                self.bwram_read_cpu(b, addr)
            }
            CpuRegion::Iram(off) => self.iram_read(off),
            CpuRegion::Open => 0,
        }
    }

    fn write24(&mut self, addr24: u32, val: u8) {
        let bank = (addr24 >> 16) & 0xFF;
        let addr = addr24 & 0xFFFF;
        match Self::classify_cpu(addr24) {
            CpuRegion::Io => {
                self.host_accesses = self.host_accesses.wrapping_add(1);
                self.write_io_cpu(addr24, val);
            }
            CpuRegion::Bwram => {
                let b = if (0x40..=0x4F).contains(&bank) {
                    bank
                } else {
                    bank & 0x7F
                };
                self.bwram_write_cpu(b, addr, val);
            }
            CpuRegion::Iram(off) => self.iram_write_cpu(off, val),
            CpuRegion::Rom | CpuRegion::Open => {}
        }
    }

    fn rom(&self) -> &[u8] {
        &self.rom
    }

    fn sram(&self) -> &[u8] {
        &self.bwram
    }

    fn sram_mut(&mut self) -> &mut [u8] {
        &mut self.bwram
    }

    fn irq_pending(&self) -> bool {
        (self.io.cpu_irqen && self.io.cpu_irqfl) || (self.io.chdma_irqen && self.io.chdma_irqfl)
    }

    fn coprocessor_host_accesses(&self) -> u64 {
        self.host_accesses
    }

    // Write the full SA-1 system state — the $2200-$23FF register file, the 2 KiB I-RAM, the H/V
    // timer counters, and the character-conversion DMA staging flags — into a "SA10" section. ROM
    // is never embedded (docs/adr/0003); BW-RAM is System::save_state's own Board::sram capture,
    // not duplicated here; host_accesses (a debugger counter) is excluded, matching every board.
    fn save_state(&self, w: &mut SaveWriter) {
        w.section(*b"SA10", |s| {
            self.io.save_state(s);
            s.write_u16(self.timer.scanlines);
            s.write_u16(self.timer.vcounter);
            s.write_u16(self.timer.hcounter);
            s.write_bool(self.reset_pending);
            s.write_bool(self.bwram_dma);
            s.write_u8(self.dma_line);
            s.write_bytes(&*self.iram);
        });
    }

    fn load_state(&mut self, r: &mut SaveReader) -> Result<(), SaveStateError> {
        let mut s = r.expect_section(*b"SA10")?;
        self.io.load_state(&mut s)?;
        self.timer.scanlines = s.read_u16()?;
        self.timer.vcounter = s.read_u16()?;
        self.timer.hcounter = s.read_u16()?;
        self.reset_pending = s.read_bool()?;
        self.bwram_dma = s.read_bool()?;
        self.dma_line = s.read_u8()? & 15; // masked identically at every normal increment
        self.iram.copy_from_slice(s.read_bytes(IRAM_SIZE)?);
        if s.remaining() != 0 {
            return Err(SaveStateError::Invalid(alloc::format!(
                "SA10 section has {} trailing byte(s)",
                s.remaining()
            )));
        }
        Ok(())
    }

    // --- Second-CPU (SA-1 65C816) hooks; core owns the CPU + steps it via these. ------------

    fn has_second_cpu(&self) -> bool {
        true
    }

    fn second_cpu_running(&self) -> bool {
        !(self.io.sa1_rdyb || self.io.sa1_resb)
    }

    fn second_cpu_take_reset(&mut self) -> bool {
        core::mem::take(&mut self.reset_pending)
    }

    fn second_cpu_poll_nmi(&mut self) -> bool {
        if self.io.sa1_nmi && !self.io.sa1_nmicl {
            self.io.sa1_nmifl = true;
            self.io.sa1_nmicl = true;
            return true;
        }
        false
    }

    fn second_cpu_poll_irq(&self) -> bool {
        (self.io.timer_irqen && !self.io.timer_irqcl)
            || (self.io.dma_irqen && !self.io.dma_irqcl)
            || (self.io.sa1_irq && !self.io.sa1_irqcl)
    }

    fn second_cpu_tick(&mut self, clocks: u32) {
        self.tick(clocks);
    }

    fn second_cpu_read(&mut self, addr24: u32) -> u8 {
        let bank = (addr24 >> 16) & 0xFF;
        let addr = addr24 & 0xFFFF;
        // SA-1 CPU interrupt/reset vector intercept ($00:FFEA/EE/FA/FC/FE → CNV/CIV/CRV).
        if bank == 0x00 {
            match addr {
                0xFFEA | 0xFFFA => return self.io.cnv as u8, // NMI (native / emulation)
                0xFFEB | 0xFFFB => return (self.io.cnv >> 8) as u8,
                0xFFEE | 0xFFFE => return self.io.civ as u8, // IRQ/BRK (native / emulation)
                0xFFEF | 0xFFFF => return (self.io.civ >> 8) as u8,
                0xFFFC => return self.io.crv as u8, // reset
                0xFFFD => return (self.io.crv >> 8) as u8,
                _ => {}
            }
        }
        // $2200-$23FF — SA-1-side registers.
        if addr24 & 0x40_fe00 == 0x00_2200 {
            return self.read_io_sa1(addr24);
        }
        // ROM: $00-$3F/$80-$BF:$8000-$FFFF and $C0-$FF:$0000-$FFFF.
        if addr24 & 0x40_8000 == 0x00_8000 || addr24 & 0xc0_0000 == 0xc0_0000 {
            return self.rom_read_sa1(addr24);
        }
        // BW-RAM: $00-$3F/$80-$BF:$6000-$7FFF, $40-$5F, $60-$6F.
        if addr24 & 0x40_e000 == 0x00_6000
            || addr24 & 0xe0_0000 == 0x40_0000
            || addr24 & 0xf0_0000 == 0x60_0000
        {
            return self.bwram_read_sa1(addr24);
        }
        // I-RAM: $00-$3F/$80-$BF:$0000-$07FF and $3000-$37FF.
        if addr24 & 0x40_f800 == 0x00_0000 || addr24 & 0x40_f800 == 0x00_3000 {
            return self.iram_read(addr24 & 0x7FF);
        }
        0
    }

    fn second_cpu_write(&mut self, addr24: u32, val: u8) {
        if addr24 & 0x40_fe00 == 0x00_2200 {
            self.write_io_sa1(addr24, val);
            return;
        }
        if addr24 & 0x40_8000 == 0x00_8000 || addr24 & 0xc0_0000 == 0xc0_0000 {
            return; // ROM is read-only.
        }
        if addr24 & 0x40_e000 == 0x00_6000
            || addr24 & 0xe0_0000 == 0x40_0000
            || addr24 & 0xf0_0000 == 0x60_0000
        {
            self.bwram_write_sa1(addr24, val);
            return;
        }
        if addr24 & 0x40_f800 == 0x00_0000 || addr24 & 0x40_f800 == 0x00_3000 {
            self.iram_write_sa1(addr24 & 0x7FF, val);
        }
    }
}

/// Select an SA-1 board for `rom`, sizing BW-RAM from `sram_size` and the timer from `region`.
#[must_use]
pub fn select(rom: Box<[u8]>, sram_size: usize, region: Region) -> Sa1Board {
    Sa1Board::new(rom, sram_size, region)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn board() -> Sa1Board {
        // 1 MiB ROM, default BW-RAM.
        Sa1Board::new(vec![0u8; 0x10_0000].into_boxed_slice(), 0, Region::Ntsc)
    }

    #[test]
    fn detects_sa1_and_has_second_cpu() {
        let b = board();
        assert_eq!(b.coprocessor(), Coprocessor::Sa1);
        assert!(b.has_second_cpu());
        // Held in reset at power-on (RESB=1).
        assert!(!b.second_cpu_running());
    }

    #[test]
    fn cpu_decode_regions() {
        let b = board();
        assert!(matches!(b.map(0x00_2200), MappedAddr::Coprocessor)); // IO
        assert!(matches!(b.map(0x00_3000), MappedAddr::Coprocessor)); // I-RAM
        assert!(matches!(b.map(0x00_6000), MappedAddr::Sram(_))); // BW-RAM window
        assert!(matches!(b.map(0x40_0000), MappedAddr::Sram(_))); // BW-RAM linear
        assert!(matches!(b.map(0x00_8000), MappedAddr::Rom(_))); // ROM window
        assert!(matches!(b.map(0xC0_0000), MappedAddr::Rom(_))); // ROM HiROM
    }

    #[test]
    fn reset_handshake_and_vector_intercept() {
        let mut b = board();
        // Program the SA-1 reset vector via CRV ($2203/$2204), then clear RESB ($2200 bit5=0).
        b.write24(0x00_2203, 0x34);
        b.write24(0x00_2204, 0x12);
        b.write24(0x00_2200, 0x00); // RESB 1->0
        assert!(b.second_cpu_take_reset());
        assert!(!b.second_cpu_take_reset()); // edge consumed
        assert!(b.second_cpu_running());
        // The SA-1 CPU reset vector fetch ($00:FFFC/D) returns CRV.
        assert_eq!(b.second_cpu_read(0x00_FFFC), 0x34);
        assert_eq!(b.second_cpu_read(0x00_FFFD), 0x12);
    }

    #[test]
    fn arithmetic_unit_mul_div() {
        let mut b = board();
        // The arithmetic unit ($2250-$2254) is SA-1-side: written via the second CPU.
        // signed multiply: MD=0, ACM=0. MA=0x0010, MB=0x0010 -> 0x100.
        b.second_cpu_write(0x00_2250, 0x00);
        b.second_cpu_write(0x00_2251, 0x10);
        b.second_cpu_write(0x00_2252, 0x00);
        b.second_cpu_write(0x00_2253, 0x10);
        b.second_cpu_write(0x00_2254, 0x00); // trigger
        assert_eq!(b.second_cpu_read(0x00_2306), 0x00);
        assert_eq!(b.second_cpu_read(0x00_2307), 0x01); // 0x100
        // unsigned divide: MD=1. MA=100, MB=7 -> q=14 r=2.
        b.second_cpu_write(0x00_2250, 0x01);
        b.second_cpu_write(0x00_2251, 100);
        b.second_cpu_write(0x00_2252, 0x00);
        b.second_cpu_write(0x00_2253, 7);
        b.second_cpu_write(0x00_2254, 0x00);
        assert_eq!(b.second_cpu_read(0x00_2306), 14); // quotient low
        assert_eq!(b.second_cpu_read(0x00_2308), 2); // remainder low (mr>>16)
    }

    #[test]
    fn iram_roundtrip_both_views() {
        let mut b = board();
        // S-CPU writes need the write-protect open (SIWP all blocks enabled).
        b.write24(0x00_2229, 0xFF);
        b.write24(0x00_3010, 0x5A);
        assert_eq!(b.read24(0x00_3010), 0x5A);
        // SA-1 view sees the same I-RAM at $3000-$37FF.
        assert_eq!(b.second_cpu_read(0x00_3010), 0x5A);
    }

    #[test]
    fn bwram_roundtrip_main_view() {
        let mut b = board();
        // Enable S-CPU BW-RAM writes (SWEN bit7).
        b.write24(0x00_2226, 0x80);
        b.write24(0x40_0123, 0x77); // linear
        assert_eq!(b.read24(0x40_0123), 0x77);
    }

    #[test]
    fn rom_window_reads_image() {
        let mut rom = vec![0u8; 0x10_0000];
        rom[0x0000] = 0xAA;
        rom[0x8000] = 0xBB;
        let mut b = Sa1Board::new(rom.into_boxed_slice(), 0, Region::Ntsc);
        // $00:$8000 -> block C, within 0 -> ROM 0.
        assert_eq!(b.read24(0x00_8000), 0xAA);
        // $01:$8000 -> within 0x8000 -> ROM 0x8000.
        assert_eq!(b.read24(0x01_8000), 0xBB);
    }

    #[test]
    fn system_state_round_trips_through_save_state() {
        let mut b = board();
        b.write24(0x00_2203, 0x34); // CRV low
        b.write24(0x00_2204, 0x12); // CRV high
        b.write24(0x00_2200, 0x00); // RESB 1->0 (armed reset_pending)
        b.write24(0x00_2229, 0xFF); // SIWP open
        b.write24(0x00_3010, 0x5A); // I-RAM byte

        let mut w = SaveWriter::new();
        b.save_state(&mut w);
        let bytes = w.into_bytes();

        let mut fresh = board();
        let mut r = SaveReader::new(&bytes);
        fresh.load_state(&mut r).unwrap();

        assert_eq!(fresh.second_cpu_read(0x00_FFFC), 0x34);
        assert_eq!(fresh.second_cpu_read(0x00_FFFD), 0x12);
        assert!(fresh.second_cpu_take_reset());
        assert_eq!(fresh.read24(0x00_3010), 0x5A);
        assert_eq!(r.remaining(), 0);
    }
}
