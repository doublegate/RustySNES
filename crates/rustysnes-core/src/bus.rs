//! The Bus owns everything mutable.
//!
//! It holds the PPU1/PPU2, the SPC700+S-DSP, the cart (→ board / coprocessor), WRAM,
//! controllers, the open-bus latch, and the DMA/HDMA state. The 65C816 borrows `&mut Bus`
//! during an instruction (the TetaNES-postmortem lesson — one owner avoids the borrow-checker
//! fight). The video/audio chips see narrower bus traits, re-exported here from their crates:
//! [`rustysnes_ppu::VideoBus`] and [`rustysnes_apu::AudioBus`]. See `docs/architecture.md`
//! (the load-bearing facts).

use alloc::boxed::Box;

use rustysnes_apu::{Apu, AudioBus};
use rustysnes_cart::Cart;
use rustysnes_cpu::Bus as CpuBus;
use rustysnes_ppu::{Ppu, VideoBus};

/// WRAM size — the SNES has 128 KiB of work RAM (`$7E0000-$7FFFFF`).
const WRAM_SIZE: usize = 128 * 1024;

/// Everything mutable lives here. The 65C816 borrows `&mut Bus`; the PPU/APU see the narrow
/// [`VideoBus`]/[`AudioBus`] traits implemented on this same struct.
pub struct Bus {
    /// The video subsystem (PPU1 + PPU2).
    pub ppu: Ppu,
    /// The audio subsystem (SPC700 + S-DSP + ARAM).
    pub apu: Apu,
    /// The loaded cartridge (board mapping + any coprocessor), or `None` before a ROM loads.
    pub cart: Option<Cart>,
    /// 128 KiB work RAM (`$7E0000-$7FFFFF`).
    // reason: skeleton field — the `$7E/$7F` WRAM routing in `read24`/`write24` is a
    // TODO(T-21); the storage is wired now so the real decode is a body-only change.
    #[allow(dead_code)]
    wram: Box<[u8; WRAM_SIZE]>,
    /// Open-bus latch: the last value driven on the data bus (unmapped reads return it).
    #[allow(clippy::struct_field_names)]
    // reason: "open_bus" is the hardware name for the latch.
    open_bus: u8,
    // TODO(T-21): controllers (auto-joypad read + manual `$4016`/`$4017`), the DMA/HDMA
    // channel state (`$43xx`), and the four CPU↔APU port latches (`$2140-$2143`).
}

impl Default for Bus {
    fn default() -> Self {
        Self {
            ppu: Ppu::new(),
            apu: Apu::new(),
            cart: None,
            // TODO(T-21): seed WRAM from the determinism PRNG (power-on garbage), not zero.
            // Build on the heap (a 128 KiB stack array would blow the frame).
            wram: alloc::vec![0u8; WRAM_SIZE]
                .into_boxed_slice()
                .try_into()
                .unwrap(),
            open_bus: 0,
        }
    }
}

impl core::fmt::Debug for Bus {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("Bus")
            .field("cart", &self.cart.as_ref().map(|c| c.board.name()))
            .field("open_bus", &self.open_bus)
            .finish_non_exhaustive()
    }
}

/// The 65C816's view: route a 24-bit access to WRAM / chip registers / the cart board.
impl CpuBus for Bus {
    fn read24(&mut self, addr24: u32) -> u8 {
        // TODO(T-21): decode `$7E/$7F` → WRAM, `$2100-$21FF` → PPU regs, `$2140-$2143` → APU
        // ports, `$4016/$4017` → controllers, `$4200-$43FF` → CPU/DMA regs, else → cart board.
        // The skeleton routes everything to the cart (or open bus) so the workspace compiles.
        let val = self
            .cart
            .as_mut()
            .map_or(self.open_bus, |c| c.board.read24(addr24));
        self.open_bus = val;
        val
    }

    fn write24(&mut self, addr24: u32, val: u8) {
        self.open_bus = val;
        // TODO(T-21): same decode as `read24` for the write direction.
        if let Some(c) = self.cart.as_mut() {
            c.board.write24(addr24, val);
        }
    }
}

/// The PPU's view: cart-mediated reads + the board scanline/vblank notifies.
impl VideoBus for Bus {
    fn cart_read(&mut self, addr24: u32) -> u8 {
        self.cart
            .as_mut()
            .map_or(self.open_bus, |c| c.board.read24(addr24))
    }

    fn notify_scanline(&mut self) {
        if let Some(c) = self.cart.as_mut() {
            c.board.notify_scanline();
        }
    }

    fn notify_vblank(&mut self) {
        // TODO(T-21): set the NMI line / `$4210` vblank flag when the PPU model lands.
    }
}

/// The APU's view: the four CPU↔APU communication-port latches + the timer IRQ.
impl AudioBus for Bus {
    fn read_port(&mut self, _port: u8) -> u8 {
        // TODO(T-21): return the CPU→APU port latch (the resync boundary between domains).
        0
    }

    fn write_port(&mut self, _port: u8, _val: u8) {
        // TODO(T-21): latch the APU→CPU port byte.
    }

    fn raise_irq(&mut self) {
        // TODO(T-21): OR the SPC700 timer IRQ into the CPU IRQ line.
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_bus_has_no_cart_and_reads_open() {
        let mut bus = Bus::default();
        assert!(bus.cart.is_none());
        // With no cart, a CPU read returns open bus (0 at power-on).
        assert_eq!(<Bus as CpuBus>::read24(&mut bus, 0x00_8000), 0);
    }
}
