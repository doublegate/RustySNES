//! The SA-1 CPU's bus adapter — the bridge that lets `rustysnes-core` step the second 65C816.
//!
//! The one-directional crate graph forbids `rustysnes-cart` from depending on `rustysnes-cpu`, so
//! the SA-1 *system* (registers, Super-MMC banking, BW-RAM, I-RAM, arithmetic unit, DMA, timer)
//! lives in [`rustysnes_cart::coproc::sa1`] and exposes the SA-1 CPU's memory view through the
//! [`rustysnes_cart::Board`] second-CPU hooks. This adapter wraps the cart board behind the
//! [`rustysnes_cpu::Bus`] trait so the second [`rustysnes_cpu::Cpu`] (owned by the scheduler) can
//! borrow it for an instruction.
//!
//! Unlike the main bus this adapter does **not** advance the master clock: the SA-1's own timing is
//! driven by the scheduler, which charges the SA-1 H/V timer from the instruction's returned cycle
//! count (`docs/scheduler.md` §SA-1). Interrupt/reset vector redirection (the SA-1 uses its own
//! CRV/CIV/CNV vectors, not the ROM `$FFEx` vectors) is handled inside the board's
//! [`rustysnes_cart::Board::second_cpu_read`].

use rustysnes_cart::Board;
use rustysnes_cpu::Bus as CpuBus;

/// A [`rustysnes_cpu::Bus`] view over the SA-1 board's second-CPU memory map.
pub(crate) struct Sa1Bus<'a> {
    /// The cart board carrying the SA-1 system state.
    pub board: &'a mut dyn Board,
}

impl CpuBus for Sa1Bus<'_> {
    fn read24(&mut self, addr24: u32) -> u8 {
        self.board.second_cpu_read(addr24)
    }

    fn write24(&mut self, addr24: u32, val: u8) {
        self.board.second_cpu_write(addr24, val);
    }

    fn poll_nmi(&mut self) -> bool {
        self.board.second_cpu_poll_nmi()
    }

    fn poll_irq(&mut self) -> bool {
        self.board.second_cpu_poll_irq()
    }

    // `advance` is intentionally the default no-op: the SA-1's timebase is charged by the
    // scheduler from the instruction's returned cycle count, not per bus access.
}
