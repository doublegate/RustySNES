//! The debugger overlay's per-frame read-only state copy.
//!
//! Mirrors [`crate::ui_shell::ShellInfo`]'s own pattern exactly: [`crate::emu::EmuCore::debug_snapshot`]
//! copies plain data out of the [`rustysnes_core::System`] under the SAME brief lock `ShellInfo`
//! already uses, before the lock is dropped and the egui pass runs — the shell's non-negotiable
//! rule (`ui_shell.rs`'s module doc) is that egui NEVER touches the emu lock directly.

use rustysnes_core::cpu::Regs;

/// One frame's worth of read-only chip state for the debugger overlay's 4 panels.
///
/// Built by [`crate::emu::EmuCore::debug_snapshot`] under the brief emu lock, then handed to
/// [`crate::ui_shell::ShellState::render`] after the lock is released.
#[derive(Debug, Clone)]
pub struct DebugSnapshot {
    /// The main 65C816's architectural register file.
    pub cpu: Regs,
    /// PPU1/PPU2 state.
    pub ppu: PpuSnapshot,
    /// SPC700 + S-DSP state.
    pub apu: ApuSnapshot,
    /// The loaded cart's board + any coprocessor state.
    pub cart: CartSnapshot,
    /// A disassembly window starting at the current PC (`v0.9.0`, T-81-001 PR B) — `(24-bit
    /// address, decoded text)` pairs, computed via `EmuCore::disassembly_window`'s non-intrusive
    /// `Bus::peek` (never the live, side-effecting `CpuBus::read24` — a debugger peek must not
    /// perturb the open-bus latch or trip watchpoints).
    pub disassembly: Vec<(u32, String)>,
    /// Whether the debugger has execution paused (a breakpoint fired, or the user paused/stepped).
    pub paused: bool,
    /// Read/write watchpoint hits recorded since the debugger last polled (`v0.8.0` T-81-001b).
    /// Always empty when the `debug-hooks` feature is off (or no watchpoints are armed) — kept as
    /// a plain, unconditional field (not `#[cfg]`-gated) so `DebugSnapshot` itself stays a single
    /// always-compiled shape, matching this whole struct's existing "practically dead but
    /// harmless to compile" posture when the debugger overlay is unreachable.
    pub watchpoint_hits: Vec<WatchHit>,
    /// A [`MEMORY_WINDOW_LEN`]-byte window of the full 24-bit CPU bus starting at
    /// `memory_window_start`, for the debugger's Memory panel (`v1.7.0`). Read via the same
    /// non-intrusive `Bus::peek` the CPU panel's disassembler already uses — never the live,
    /// side-effecting `CpuBus::read24`, so viewing memory can never itself trip a watchpoint or
    /// perturb the open-bus latch.
    pub memory_window: [u8; MEMORY_WINDOW_LEN],
    /// The 24-bit address `memory_window` starts at.
    pub memory_window_start: u32,
}

/// Bytes per memory-viewer window (`v1.7.0`).
///
/// Enough for a meaningful hex-dump page (32 rows of 16 bytes), small enough that a per-frame
/// `Bus::peek` loop while the debugger is open is not a real cost next to a whole CPU/PPU/APU
/// tick pass.
pub const MEMORY_WINDOW_LEN: usize = 512;

/// A recorded watchpoint hit.
///
/// Decoupled from `rustysnes_core::watchpoint::WatchpointHit` (which only exists when
/// `rustysnes-core`'s own `debug-hooks` feature is on) so [`DebugSnapshot`] doesn't need
/// conditional compilation for this field.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct WatchHit {
    /// The 24-bit CPU-bus address (`$bank:offset`) that was accessed.
    pub address: u32,
    /// The byte value read or written.
    pub value: u8,
    /// `true` if this hit was a write, `false` if a read.
    pub is_write: bool,
    /// The CPU's `PBR:PC` at the moment of the access.
    pub pbr_pc: u32,
}

/// One watchpoint the user has armed (`v0.8.0` T-81-001b).
///
/// The frontend's own copy of what's currently installed into the `Bus`, kept here (not
/// `rustysnes_core::watchpoint::Watchpoint` directly) for the same reason `WatchHit` exists: this
/// list lives in `Active`, which is not itself `debug-hooks`-gated.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct WatchpointEntry {
    /// The 24-bit CPU-bus address (`$bank:offset`) to watch.
    pub address: u32,
    /// Which access kind(s) trigger it.
    pub kind: WatchpointKind,
}

/// Mirrors `rustysnes_core::watchpoint::WatchKind` without depending on it directly (see
/// [`WatchpointEntry`]'s doc for why).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum WatchpointKind {
    /// Fire only on a CPU read of the watched address.
    Read,
    /// Fire only on a CPU write to the watched address.
    Write,
    /// Fire on either a read or a write.
    #[default]
    ReadWrite,
}

/// PPU state for the debugger's PPU panel.
#[derive(Debug, Clone)]
pub struct PpuSnapshot {
    /// `BGMODE` ($2105), 0..=7.
    pub bg_mode: u8,
    /// `INIDISP` ($2100) master brightness, 0..=15.
    pub display_brightness: u8,
    /// Whether the current frame is hi-res (512-wide, `v0.7.0`).
    pub is_hires: bool,
    /// The current scanline (`Ppu::scanline`).
    pub scanline: u16,
    /// The current dot within the scanline (`Ppu::dot`).
    pub dot: u16,
    /// Whether the PPU is in vertical blank.
    pub in_vblank: bool,
    /// Whether the PPU is in horizontal blank.
    pub in_hblank: bool,
    /// The full 256-entry CGRAM palette (512 bytes — cheap to copy wholesale every frame, unlike
    /// VRAM's 64 KiB).
    pub cgram: [u16; 256],
    /// A [`VRAM_WINDOW_LEN`]-word window of VRAM starting at `vram_window_start` (word address) —
    /// copying all 64 KiB every frame would be real, avoidable per-frame cost for a window the
    /// user can only look at part of at once. `EmuCore::set_debug_vram_scroll` moves the window;
    /// no UI control calls it yet (fixed at the window's start address today) — a follow-up.
    pub vram_window: [u16; VRAM_WINDOW_LEN],
    /// The word address `vram_window` starts at.
    pub vram_window_start: u16,
    /// The full 544-byte OAM (small enough to copy wholesale every frame).
    pub oam: [u8; 544],
}

/// Words per VRAM viewer window (2 KiB) — big enough for a meaningful hex-dump page, small
/// enough that copying it every frame is not a real cost next to a whole PPU dot-tick pass.
pub const VRAM_WINDOW_LEN: usize = 1024;

/// APU (SPC700 + S-DSP) state for the debugger's APU panel.
#[derive(Debug, Clone, Copy)]
pub struct ApuSnapshot {
    /// The SMP's program counter.
    pub smp_pc: u16,
    /// Whether the SMP is halted (`STOP`/`SLEEP`).
    pub smp_stopped: bool,
    /// Per-voice `(vol_left, vol_right, pitch, srcn, adsr_lo, adsr_hi, gain, envx, outx)` —
    /// the DSP registers a debugger cares about, read via `Apu::dsp_read` (no side effects).
    pub voices: [VoiceSnapshot; 8],
}

/// One S-DSP voice's key registers (`docs/apu.md`'s DSP register map, per-voice base `v*0x10`).
#[derive(Debug, Clone, Copy, Default)]
pub struct VoiceSnapshot {
    /// `VOLL`/`VOLR`.
    pub vol: (i8, i8),
    /// `PITCHL`/`PITCHH` (14-bit).
    pub pitch: u16,
    /// `SRCN` (the sample source-directory entry).
    pub srcn: u8,
    /// `ADSR1`/`ADSR2`.
    pub adsr: (u8, u8),
    /// `GAIN`.
    pub gain: u8,
    /// `ENVX` (the current envelope level).
    pub envx: u8,
    /// `OUTX` (the current sample output).
    pub outx: u8,
}

/// Cart/coprocessor state for the debugger's Cart panel.
#[derive(Debug, Clone)]
pub struct CartSnapshot {
    /// The active board's name (`Board::name()`), e.g. `"HiROM+SuperFX"`.
    pub board_name: Option<&'static str>,
    /// The SA-1 second CPU's register file, when the loaded cart is an SA-1 board.
    pub sa1: Option<Regs>,
    /// The Super FX/GSU register file (R0-R15, SFR, PBR), when the loaded cart is a Super FX
    /// board.
    pub gsu: Option<GsuSnapshot>,
}

/// The GSU register file, as exposed by `Board::debug_gsu_state`.
#[derive(Debug, Clone, Copy)]
pub struct GsuSnapshot {
    /// R0-R15 (R15 doubles as the GSU program counter).
    pub r: [u16; 16],
    /// The status flag register.
    pub sfr: u16,
    /// The program bank register.
    pub pbr: u8,
}
