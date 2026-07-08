//! The host-facing memory surface the ARM core reads/writes through (mirrors the
//! `Hg51bBus`/`rustysnes_cpu::Bus` pattern already used elsewhere in this codebase).

/// A 24-bit-ish ARM-side address space accessor.
///
/// The concrete board wrapper (not yet built, `docs/st018-arm-notes.md` step 9) implements this
/// over PRG ROM / data ROM / work RAM / the SNES-side handshake registers, and is also where the
/// ARM's own cycle counter advances (every method call here corresponds to one real bus cycle on
/// real hardware).
pub trait ArmBus {
    /// Fetch one instruction word (an aligned, sequential-or-not code read).
    fn read_code(&mut self, addr: u32) -> u32;
    /// Read a data word or byte (`LDR`/`SWP`/`LDM`). `byte` selects an 8-bit access.
    fn read(&mut self, addr: u32, byte: bool) -> u32;
    /// Write a data word or byte (`STR`/`SWP`/`STM`).
    fn write(&mut self, addr: u32, value: u32, byte: bool);
    /// One internal/idle bus cycle (register-specified shift amounts, multiply, `SWP`'s
    /// between-read-and-write gap, etc. — real hardware spends a cycle here even though no
    /// address is touched).
    fn idle(&mut self);
}
