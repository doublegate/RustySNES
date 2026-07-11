//! The emulation core wrapper the frontend drives: load a ROM, step a frame, expose the
//! framebuffer + audio for the present path.
//!
//! [`EmuCore`] owns the `rustysnes-core` [`System`] (the master-clock lockstep scheduler) and the
//! frontend-side derived state the present path consumes under a brief lock: an RGBA8 framebuffer
//! (decoded from the PPU's 15-bit BGR555 output) and the per-frame audio FIFO drained from the
//! S-DSP. The determinism contract lives in the core; the frontend NEVER injects timing/RNG into
//! synthesis — rate control + run-ahead are pure frontend concerns (in `app.rs` / `audio.rs`).

use rustysnes_core::System;
use rustysnes_core::cart::Cart;
use rustysnes_core::cart::header::Coprocessor;
use rustysnes_core::cpu::disasm::disassemble_one;

use crate::config::Region;
use crate::debug_snapshot::{
    ApuSnapshot, CartSnapshot, DebugSnapshot, GsuSnapshot, PpuSnapshot, VRAM_WINDOW_LEN,
    VoiceSnapshot, WatchHit,
};
use crate::gfx::{MAX_H, MAX_W, SNES_W, bgr555_to_rgba8};
use crate::input::Buttons;

/// How many instructions the debugger's disassembly view shows per snapshot (`v0.9.0`,
/// T-81-001 PR B) — enough for a useful window around PC without a real per-frame cost (each is
/// one `disassemble_one` call, a handful of bus peeks).
const DISASSEMBLY_WINDOW_LEN: usize = 24;

/// A CPU instruction budget for [`EmuCore::step_over`]'s "run until the call returns" loop — high
/// enough to cover any real subroutine, low enough that a runaway/self-modifying-code edge case
/// can't hang the debugger (mirrors `rustysnes-core::scheduler`'s own `MAX_STEPS_PER_FRAME`
/// safety-cap posture).
const MAX_STEP_OVER_INSTRUCTIONS: u32 = 1_000_000;

/// Coprocessor firmware dumps the frontend will try, in order, for a cart that carries a
/// chip-ROM-dump coprocessor. The matching dump (when the user has supplied it) is the only one
/// [`Cart::install_coprocessor_firmware`] accepts; the rest are rejected and left unchanged.
const fn firmware_candidates(co: Coprocessor) -> &'static [&'static str] {
    match co {
        // The µPD77C25 DSP-1..4 family — the right dump depends on the game; try the common ones.
        Coprocessor::Dsp => &["dsp1.rom", "dsp1b.rom", "dsp2.rom", "dsp3.rom", "dsp4.rom"],
        Coprocessor::Cx4 => &["cx4.rom"],
        // Logic-only / on-die coprocessors (Super FX, SA-1, S-DD1, SPC7110, OBC1) need no external
        // firmware dump in this core.
        _ => &[],
    }
}

/// The frontend's view of the emulator. Lives behind an `Arc<Mutex<…>>` on native (shared with
/// the dedicated emulation thread) and is stepped synchronously on wasm.
pub struct EmuCore {
    /// The master-clock scheduler + Bus (owns every chip).
    system: System,
    /// The current console region (drives the active framebuffer height + pacing).
    region: Region,
    /// The frontend-side RGBA8 framebuffer (sized to the hi-res worst case; the active sub-rect is
    /// `fb_dims` large). Copied out under one brief lock by the present path.
    framebuffer: Vec<u8>,
    /// The active framebuffer dims `(w, h)` for the current video mode.
    fb_dims: (u32, u32),
    /// The 32 kHz stereo samples the S-DSP emitted during the most recent [`Self::run_frame`].
    audio: Vec<(i16, i16)>,
    /// The latest latched controller state for P1 / P2 (late-latched by the window handler).
    pads: [Buttons; 2],
    /// Whether a ROM is currently loaded (the present path shows a blank frame otherwise).
    rom_loaded: bool,
    /// The raw ROM image, retained so Power-Cycle can rebuild a clean machine deterministically.
    rom: Vec<u8>,
    /// The coprocessor firmware dump installed for this cart (if any), retained for Power-Cycle.
    firmware: Vec<u8>,
    /// The debugger overlay's VRAM viewer scroll position (word address). Only meaningful when
    /// the debugger is open; `debug_snapshot` reads it regardless (cheap, and keeps this struct
    /// free of `debug-hooks`-conditional fields).
    debug_vram_scroll: u16,
    /// Armed 65C816 PC breakpoints (`v0.9.0`, T-81-001 PR B) — 24-bit `pbr:pc` addresses. Checked
    /// once per instruction (not per bus access, unlike read/write watchpoints — a PC breakpoint
    /// is an instruction-boundary concept), so this costs nothing on the fast path when empty and
    /// `Vec::contains`'s linear scan otherwise (this list is normally a handful of entries, never
    /// a hot-loop concern).
    breakpoints: Vec<u32>,
    /// Whether the debugger has paused execution (a breakpoint fired, or the user clicked Pause).
    /// While `true`, [`Self::run_frame`] does not advance the `System` at all — the debugger's
    /// Step Into / Step Over buttons single-step it instead.
    paused: bool,
}

impl EmuCore {
    /// Power on with a determinism seed and a region. No ROM is loaded yet.
    #[must_use]
    pub fn new(seed: u64, region: Region) -> Self {
        Self {
            system: System::new(seed),
            region,
            framebuffer: vec![0u8; (MAX_W * MAX_H * 4) as usize],
            fb_dims: (SNES_W, region.active_height()),
            audio: Vec::new(),
            pads: [Buttons::default(); 2],
            rom_loaded: false,
            rom: Vec::new(),
            firmware: Vec::new(),
            debug_vram_scroll: 0,
            breakpoints: Vec::new(),
            paused: false,
        }
    }

    /// Load a raw ROM image, transparently unwrapping a zip archive first if `rom` is one (see
    /// `extract_rom_bytes` — the common case of a `.sfc`/`.smc` distributed zipped). Header
    /// detection + board selection happen in `rustysnes-cart`. On success the cart is installed
    /// in a fresh Bus and the system is left ready to boot from the cart's reset vector on the
    /// first [`Self::run_frame`].
    ///
    /// # Errors
    /// Returns an [`EmuError`] if the image is empty, is a zip archive that can't be opened or
    /// contains no recognizable ROM entry, or no valid SNES header is found.
    pub fn load_rom(&mut self, rom: &[u8]) -> Result<(), EmuError> {
        if rom.is_empty() {
            return Err(EmuError::Empty);
        }
        let rom = extract_rom_bytes(rom)?;
        if rom.is_empty() {
            return Err(EmuError::Empty);
        }
        let cart = Cart::from_rom(&rom).map_err(|e| EmuError::Header(format!("{e:?}")))?;
        let mut system = System::new(0);
        system.bus.cart = Some(cart);
        self.system = system;
        self.rom = rom.into_owned();
        self.firmware.clear();
        self.rom_loaded = true;
        self.audio.clear();
        Ok(())
    }

    /// The coprocessor firmware dumps this cart will accept (filenames), in try order. Empty when
    /// the cart needs no external firmware. The UI uses this to locate + install the dump.
    #[must_use]
    pub fn firmware_candidates(&self) -> &'static [&'static str] {
        self.system
            .bus
            .cart
            .as_ref()
            .map_or(&[], |c| firmware_candidates(c.header.coprocessor))
    }

    /// Whether the loaded cart carries a coprocessor that needs a (not-yet-installed) firmware dump
    /// to function. The honesty posture of `docs/adr/0003`: without the dump the coprocessor is
    /// non-functional, so the UI should prompt for it.
    #[must_use]
    pub fn needs_firmware(&self) -> bool {
        !self.firmware_candidates().is_empty() && self.firmware.is_empty()
    }

    /// Supply a coprocessor firmware dump. Returns `true` if the cart's board accepted it (right
    /// coprocessor + size); on success the bytes are retained for Power-Cycle.
    pub fn install_firmware(&mut self, bytes: &[u8]) -> bool {
        let accepted = self
            .system
            .bus
            .cart
            .as_mut()
            .is_some_and(|c| c.install_coprocessor_firmware(bytes));
        if accepted {
            self.firmware = bytes.to_vec();
        }
        accepted
    }

    /// Restore battery SRAM from a `.srm` image (truncated/zero-padded to the board's SRAM size).
    pub fn load_sram(&mut self, data: &[u8]) {
        if let Some(c) = self.system.bus.cart.as_mut() {
            c.load_sram(data);
        }
    }

    /// The current battery SRAM contents (empty when the cart has no SRAM), for a `.srm` save.
    #[must_use]
    pub fn save_sram(&self) -> &[u8] {
        self.system.bus.cart.as_ref().map_or(&[], Cart::save_sram)
    }

    /// Soft reset: re-run the cart's reset vector without clearing RAM (the SNES front-panel
    /// Reset button). A no-op when no ROM is loaded.
    pub fn reset(&mut self) {
        if self.rom_loaded {
            self.system.reset();
            self.audio.clear();
        }
    }

    /// Power-cycle (hard reset): rebuild a clean machine from the retained ROM + firmware. Battery
    /// SRAM is NOT preserved here (the caller reloads `.srm` if desired).
    pub fn power_cycle(&mut self) {
        if !self.rom_loaded {
            return;
        }
        if let Ok(cart) = Cart::from_rom(&self.rom) {
            let mut system = System::new(0);
            system.bus.cart = Some(cart);
            if let Some(c) = system
                .bus
                .cart
                .as_mut()
                .filter(|_| !self.firmware.is_empty())
            {
                let _ = c.install_coprocessor_firmware(&self.firmware);
            }
            self.system = system;
            self.audio.clear();
        }
    }

    /// Close the loaded ROM and present a clean blank frame.
    pub fn close_rom(&mut self) {
        self.system = System::new(0);
        self.rom.clear();
        self.firmware.clear();
        self.rom_loaded = false;
        self.audio.clear();
        self.framebuffer.iter_mut().for_each(|b| *b = 0);
        self.fb_dims = (SNES_W, self.region.active_height());
    }

    /// Whether a ROM is loaded.
    #[must_use]
    pub const fn rom_loaded(&self) -> bool {
        self.rom_loaded
    }

    /// The loaded cartridge's board name (for the status bar / title), if any.
    #[must_use]
    pub fn cart_name(&self) -> Option<&'static str> {
        self.system.bus.cart.as_ref().map(|c| c.board.name())
    }

    /// The raw ROM byte image currently loaded (empty if none) — for TAS movie recording's
    /// ROM-identity hash (`rustysnes_core::movie::hash_rom`/`Movie::verify_rom`), which needs the
    /// exact bytes rather than `Cart`'s parsed/header-stripped internal representation.
    #[must_use]
    pub fn rom(&self) -> &[u8] {
        &self.rom
    }

    /// Direct mutable access to the deterministic core, for TAS movie record/playback
    /// (`rustysnes_core::movie`) and Lua scripting (`rustysnes_script::ScriptEngine`) — both need
    /// genuine read/write reach into the running `System`/`Bus`, unlike the debugger overlay's
    /// read-only [`Self::debug_snapshot`] copy.
    pub const fn system_mut(&mut self) -> &mut System {
        &mut self.system
    }

    /// Latch the controller state for a player (`0` = P1, `1` = P2). Late-latched by the window
    /// handler each frame; applied to the Bus at the top of [`Self::run_frame`].
    pub fn set_pad(&mut self, player: usize, buttons: Buttons) {
        if let Some(slot) = self.pads.get_mut(player) {
            *slot = buttons.sanitize_dpad();
        }
    }

    /// Select which peripheral is attached to controller port `port` (`v0.9.0`, Phase 7 niche
    /// peripherals). A host/session choice re-applied whenever the frontend's config changes —
    /// not carried by save-states any differently than [`Self::set_pad`]'s own live pad state is
    /// (`rustysnes_core::controller::PortState`'s own doc has the full rationale).
    pub fn set_port_device(&mut self, port: usize, device: rustysnes_core::controller::PortDevice) {
        self.system.bus.set_port_device(port, device);
    }

    /// Feed one frame's worth of SNES Mouse input for port `port` — see
    /// [`rustysnes_core::Bus::set_mouse`].
    pub fn set_mouse(&mut self, port: usize, dx: i32, dy: i32, left: bool, right: bool) {
        self.system.bus.set_mouse(port, dx, dy, left, right);
    }

    /// Set the 8 per-voice audio mute toggles (`v1.0.1`) — see
    /// [`rustysnes_core::Bus::set_voice_mutes`]'s doc.
    pub const fn set_voice_mutes(&mut self, mutes: [bool; 8]) {
        self.system.bus.set_voice_mutes(mutes);
    }

    /// Feed one frame's worth of Super Scope input for port `port` — see
    /// [`rustysnes_core::Bus::set_superscope`].
    pub fn set_superscope(&mut self, port: usize, x: i32, y: i32, buttons: u8) {
        self.system.bus.set_superscope(port, x, y, buttons);
    }

    /// Feed one frame's worth of input for Super Multitap sub-pad `sub_index` of port `port` —
    /// see [`rustysnes_core::Bus::set_multitap_pad`].
    pub fn set_multitap_pad(&mut self, port: usize, sub_index: usize, buttons: u16) {
        self.system.bus.set_multitap_pad(port, sub_index, buttons);
    }

    /// Advance one full video frame: feed the latched pads to the Bus, run the scheduler to the
    /// next frame boundary, then decode the PPU framebuffer + drain the S-DSP audio. A no-op
    /// (beyond re-presenting the already-current frame) while [`Self::is_paused`] — the debugger
    /// owns advancing the `System` in that state, via [`Self::step_into`]/[`Self::step_over`].
    pub fn run_frame(&mut self) {
        self.system.bus.set_joypad(0, self.pads[0].0);
        self.system.bus.set_joypad(1, self.pads[1].0);
        if !self.paused {
            if self.breakpoints.is_empty() {
                self.system.run_frame();
            } else {
                self.run_frame_checking_breakpoints();
            }
        }
        self.present_current_frame();
    }

    /// [`Self::run_frame`]'s slow path: steps one instruction at a time (mirroring
    /// `System::run_frame`'s own loop exactly — same frame-boundary condition, same SA-1
    /// catch-up), stopping early and setting [`Self::paused`] the instant the CPU's `pbr:pc`
    /// matches an armed breakpoint. Only reached when at least one breakpoint is armed, so the
    /// default (no breakpoints) path above is completely unaffected.
    fn run_frame_checking_breakpoints(&mut self) {
        if self.system.bus.cart.is_none() {
            return; // matches `System::run_frame`'s own early return.
        }
        let start_frame = self.system.bus.ppu.frame_count();
        let mut steps = 0u32;
        while self.system.bus.ppu.frame_count() == start_frame && steps < MAX_STEP_OVER_INSTRUCTIONS
        {
            self.system.step_instruction();
            steps += 1;
            if self.breakpoints.contains(&self.pbr_pc()) {
                self.paused = true;
                return;
            }
        }
    }

    /// The CPU's current `pbr:pc` as one 24-bit address (`$bank:offset`).
    #[must_use]
    pub const fn pbr_pc(&self) -> u32 {
        ((self.system.cpu.regs.pbr as u32) << 16) | (self.system.cpu.regs.pc as u32)
    }

    /// Install the debugger's armed PC-breakpoint list (`v0.9.0`, T-81-001 PR B), replacing any
    /// previously installed set — same "always replace, re-synced once per frame" convention as
    /// [`Self::set_pad`]/cheats/watchpoints.
    pub fn set_breakpoints(&mut self, addrs: &[u32]) {
        self.breakpoints.clear();
        self.breakpoints.extend_from_slice(addrs);
    }

    /// Whether the debugger currently has execution paused (a breakpoint fired, or the user
    /// clicked Pause/Step).
    #[must_use]
    pub const fn is_paused(&self) -> bool {
        self.paused
    }

    /// Enter the paused state without waiting for a breakpoint (the debugger's Pause button).
    pub const fn pause(&mut self) {
        self.paused = true;
    }

    /// Leave the paused state — [`Self::run_frame`] resumes driving the `System` normally (still
    /// breakpoint-checked, if any remain armed).
    pub const fn resume(&mut self) {
        self.paused = false;
    }

    /// Single-step exactly one CPU instruction, then refresh the framebuffer/audio/debug view —
    /// the debugger's Step Into. A no-op (does not advance the `System`) unless already paused,
    /// so a stray click can't accidentally desync a running frame.
    pub fn step_into(&mut self) {
        if !self.paused || self.system.bus.cart.is_none() {
            return;
        }
        self.system.step_instruction();
        self.present_current_frame();
    }

    /// Step over the instruction at the current PC — a plain [`Self::step_into`] unless it's a
    /// subroutine call (`JSR`/`JSL`), in which case this runs (breakpoint-checked, same as
    /// `Self::run_frame_checking_breakpoints`) until control returns to the instruction right
    /// after the call, bounded by `MAX_STEP_OVER_INSTRUCTIONS` so a subroutine that never
    /// returns (or self-modifying code) can't hang the debugger — it simply stops there, still
    /// paused, same as hitting the instruction budget mid-subroutine.
    pub fn step_over(&mut self) {
        if !self.paused || self.system.bus.cart.is_none() {
            return;
        }
        let start_pbr = self.system.cpu.regs.pbr;
        let start_pc = self.system.cpu.regs.pc;
        let (text, len) = disassemble_one(
            |addr| self.system.bus.peek(addr),
            start_pbr,
            start_pc,
            self.system.cpu.regs.m8(),
            self.system.cpu.regs.x8(),
        );
        if !(text.starts_with("JSR") || text.starts_with("JSL")) {
            self.step_into();
            return;
        }
        #[allow(clippy::cast_possible_truncation)]
        let return_pc = start_pc.wrapping_add(len as u16);
        let mut steps = 0u32;
        while steps < MAX_STEP_OVER_INSTRUCTIONS {
            self.system.step_instruction();
            steps += 1;
            if self.system.cpu.regs.pbr == start_pbr && self.system.cpu.regs.pc == return_pc {
                break;
            }
            if self.breakpoints.contains(&self.pbr_pc()) {
                break; // stay paused on a breakpoint hit mid-subroutine, same as a full run.
            }
        }
        self.present_current_frame();
    }

    /// Disassemble [`DISASSEMBLY_WINDOW_LEN`] instructions starting at the current PC, for the
    /// debugger's 65C816 panel — a linear byte-walk (not flow-tracing), tracking `REP`/`SEP`
    /// (`$C2`/`$E2`) along the way so the `M`/`X` widths used for each subsequent instruction's
    /// operand length stay correct across a width change, the one thing that actually matters for
    /// decoding a straight-line instruction stream correctly (an unconditional jump target
    /// wouldn't be reached by this simple walk regardless, same limitation any linear disassembler
    /// has).
    fn disassembly_window(&mut self) -> Vec<(u32, String)> {
        let mut out = Vec::with_capacity(DISASSEMBLY_WINDOW_LEN);
        let pbr = self.system.cpu.regs.pbr;
        let mut pc = self.system.cpu.regs.pc;
        let mut m8 = self.system.cpu.regs.m8();
        let mut x8 = self.system.cpu.regs.x8();
        for _ in 0..DISASSEMBLY_WINDOW_LEN {
            let addr = (u32::from(pbr) << 16) | u32::from(pc);
            let opcode = self.system.bus.peek(addr);
            let (text, len) = disassemble_one(|a| self.system.bus.peek(a), pbr, pc, m8, x8);
            if len == 2 && (opcode == 0xC2 || opcode == 0xE2) {
                let operand = self.system.bus.peek(addr.wrapping_add(1));
                let (m_bit, x_bit) = (operand & 0x20 != 0, operand & 0x10 != 0);
                if opcode == 0xC2 {
                    // REP: clear bits -> WIDER (8-bit becomes false).
                    m8 &= !m_bit;
                    x8 &= !x_bit;
                } else {
                    // SEP: set bits -> NARROWER (8-bit becomes true).
                    m8 |= m_bit;
                    x8 |= x_bit;
                }
            }
            out.push((addr, text));
            #[allow(clippy::cast_possible_truncation)]
            {
                pc = pc.wrapping_add(len as u16);
            }
        }
        out
    }

    /// Decode the PPU framebuffer + drain the S-DSP audio from the `System`'s CURRENT state,
    /// without advancing it — the second half of [`Self::run_frame`], split out for netplay
    /// (`v0.8.0`, T-82-002): `rustysnes_netplay::RollbackSession::advance` drives
    /// `System::run_frame` directly (it operates on the core crate, not this frontend type), so
    /// the frontend calls this afterward to pick up the result. A rollback's internal
    /// re-simulation passes (`RollbackSession`'s own `apply_and_run`) run several frames per
    /// `advance()` call without presenting each one — only the settled result matters
    /// user-visibly. **Known limitation, shared with rollback netplay generally, not specific to
    /// this implementation:** video always reflects the corrected state cleanly, but audio
    /// already sent to a real output device during a since-corrected misprediction can't be
    /// "unplayed" — a rollback event may audibly glitch, same as GGPO-family netcode elsewhere.
    pub fn present_current_frame(&mut self) {
        self.audio.clear();
        if self.rom_loaded {
            self.system.bus.apu.drain_audio(&mut self.audio);
            self.render_framebuffer();
        }
    }

    /// Decode the PPU's (256|512)×(224|239) BGR555 framebuffer into the RGBA8 present buffer.
    /// Width tracks [`rustysnes_ppu::Ppu::visible_width`] — 512-wide for a hi-res (Modes 5/6)
    /// frame, 256-wide otherwise (`docs/ppu.md` §Hi-res (Modes 5/6) color-math precision).
    fn render_framebuffer(&mut self) {
        let h = u32::from(self.system.bus.ppu.visible_height()).min(crate::gfx::SNES_H_PAL);
        // `visible_width()` is always SCREEN_WIDTH (256) or MAX_SCREEN_WIDTH (512) — never near
        // u32::MAX, so this narrowing cast can't actually truncate.
        #[allow(clippy::cast_possible_truncation)]
        let w = self.system.bus.ppu.visible_width() as u32;
        self.fb_dims = (w, h);
        let count = (w * h) as usize;
        let src = self.system.bus.framebuffer();
        for (i, &px) in src.iter().take(count).enumerate() {
            let bytes = bgr555_to_rgba8(px).to_le_bytes();
            let o = i * 4;
            self.framebuffer[o..o + 4].copy_from_slice(&bytes);
        }
    }

    /// The current RGBA8 framebuffer slice (the active mode's `w*h*4` bytes), for the present path.
    #[must_use]
    pub fn framebuffer(&self) -> &[u8] {
        let (w, h) = self.fb_dims;
        let len = (w * h * 4) as usize;
        &self.framebuffer[..len.min(self.framebuffer.len())]
    }

    /// The audio samples (32 kHz stereo) produced during the most recent [`Self::run_frame`].
    #[must_use]
    pub fn audio(&self) -> &[(i16, i16)] {
        &self.audio
    }

    /// The active framebuffer dimensions `(w, h)`.
    #[must_use]
    pub const fn fb_dims(&self) -> (u32, u32) {
        self.fb_dims
    }

    /// The active region.
    #[must_use]
    pub const fn region(&self) -> Region {
        self.region
    }

    /// Copy out a [`DebugSnapshot`] of the current CPU/PPU/APU/Cart state, for the debugger
    /// overlay. Read-only — never mutates anything. The caller must not hold this (or any
    /// borrow of `self`) while an egui pass runs (`ui_shell.rs`'s non-negotiable rule); copy it
    /// out under the same brief lock `ShellInfo` already uses, then drop the lock.
    ///
    /// # Panics
    /// Never in practice: every index below is bounded by a fixed, small array length
    /// (`VRAM_WINDOW_LEN` = 1024, CGRAM = 256, OAM = 544, DSP voices = 8), so the `u8`/`u16`
    /// narrowing conversions from `usize` can never actually truncate.
    #[must_use]
    #[allow(clippy::cast_possible_truncation)]
    pub fn debug_snapshot(&mut self) -> DebugSnapshot {
        let ppu = &self.system.bus.ppu;
        let vram_window_start = self.debug_vram_scroll;
        let mut vram_window = [0u16; VRAM_WINDOW_LEN];
        for (i, word) in vram_window.iter_mut().enumerate() {
            *word = ppu.vram_word(vram_window_start.wrapping_add(i as u16));
        }
        let mut cgram = [0u16; 256];
        for (i, word) in cgram.iter_mut().enumerate() {
            *word = ppu.cgram_word(i as u8);
        }
        let mut oam = [0u8; 544];
        for (i, byte) in oam.iter_mut().enumerate() {
            *byte = ppu.oam_byte(i as u16);
        }

        let apu = &self.system.bus.apu;
        let voices = core::array::from_fn(|v| {
            let base = (v as u8) << 4;
            VoiceSnapshot {
                vol: (
                    apu.dsp_read(base).cast_signed(),
                    apu.dsp_read(base | 0x01).cast_signed(),
                ),
                pitch: u16::from(apu.dsp_read(base | 0x02))
                    | (u16::from(apu.dsp_read(base | 0x03)) << 8),
                srcn: apu.dsp_read(base | 0x04),
                adsr: (apu.dsp_read(base | 0x05), apu.dsp_read(base | 0x06)),
                gain: apu.dsp_read(base | 0x07),
                envx: apu.dsp_read(base | 0x08),
                outx: apu.dsp_read(base | 0x09),
            }
        });

        let board = self.system.bus.cart.as_ref().map(|c| &c.board);
        let cart = CartSnapshot {
            board_name: board.as_ref().map(|b| b.name()),
            sa1: self.system.sa1_regs(),
            gsu: board
                .as_ref()
                .and_then(|b| b.debug_gsu_state())
                .map(|(r, sfr, pbr)| GsuSnapshot { r, sfr, pbr }),
        };

        DebugSnapshot {
            cpu: self.system.cpu.regs,
            ppu: PpuSnapshot {
                bg_mode: ppu.bg_mode(),
                display_brightness: ppu.display_brightness(),
                is_hires: ppu.is_hires(),
                scanline: ppu.scanline(),
                dot: ppu.dot(),
                in_vblank: ppu.in_vblank(),
                in_hblank: ppu.in_hblank(),
                cgram,
                vram_window,
                vram_window_start,
                oam,
            },
            apu: ApuSnapshot {
                smp_pc: apu.smp_pc(),
                smp_stopped: apu.smp_stopped(),
                voices,
            },
            cart,
            disassembly: self.disassembly_window(),
            paused: self.paused,
            watchpoint_hits: self.take_watchpoint_hits(),
        }
    }

    /// Drain the watchpoint hits recorded since the last call (`v0.8.0` T-81-001b), translated
    /// into the frontend's own decoupled [`WatchHit`] shape (see that type's doc for why). Always
    /// empty when the `debug-hooks` feature is off.
    #[cfg(feature = "debug-hooks")]
    fn take_watchpoint_hits(&mut self) -> Vec<WatchHit> {
        self.system
            .bus
            .take_watchpoint_hits()
            .into_iter()
            .map(|h| WatchHit {
                address: h.address,
                value: h.value,
                is_write: h.is_write,
                pbr_pc: h.pbr_pc,
            })
            .collect()
    }

    // `&self` is unavoidably unused in this feature-off stub — matches `watchpoint_hits`'s own
    // "always compiles, practically dead when the feature is off" posture (`debug_snapshot.rs`).
    #[cfg(not(feature = "debug-hooks"))]
    #[allow(clippy::unused_self)]
    const fn take_watchpoint_hits(&self) -> Vec<WatchHit> {
        Vec::new()
    }

    /// Scroll the debugger's VRAM viewer window (word address, wraps at 64Ki words).
    pub const fn set_debug_vram_scroll(&mut self, word_addr: u16) {
        self.debug_vram_scroll = word_addr;
    }

    /// Snapshot the full deterministic core state (`rustysnes_core::System::save_state`,
    /// `docs/adr/0006`) for rewind/run-ahead/quick-save. Frontend-only state (the decoded RGBA8
    /// framebuffer, the retained ROM/firmware bytes for Power-Cycle, latched pads) is NOT part of
    /// this — it's outside the deterministic core and is re-derived after [`Self::load_state`].
    #[must_use]
    pub fn save_state(&self) -> Vec<u8> {
        self.system.save_state()
    }

    /// Restore a snapshot taken by [`Self::save_state`] from a `System` with the SAME cart
    /// already loaded (a save-state never embeds ROM bytes, `docs/adr/0006`) — the caller must
    /// have already `load_rom`'d the matching ROM. Re-renders the framebuffer immediately so the
    /// UI reflects the restored frame without waiting for the next [`Self::run_frame`], and
    /// clears the audio FIFO (a state load jumps time discontinuously; there is no continuous
    /// audio stream to drain across that jump).
    ///
    /// # Errors
    /// Propagates [`rustysnes_savestate::SaveStateError`] if `bytes` is truncated/corrupt, from
    /// an incompatible format version, or doesn't match this `System`'s currently-loaded cart
    /// (SRAM size, coprocessor presence) — the state is left unchanged on error.
    pub fn load_state(&mut self, bytes: &[u8]) -> Result<(), rustysnes_savestate::SaveStateError> {
        self.system.load_state(bytes)?;
        self.audio.clear();
        if self.rom_loaded {
            self.render_framebuffer();
        }
        Ok(())
    }
}

/// ROM-load / emulation errors surfaced to the UI.
#[derive(Debug, thiserror::Error)]
pub enum EmuError {
    /// The ROM image was empty.
    #[error("empty ROM image")]
    Empty,
    /// No valid SNES header was found (LoROM/HiROM/ExHiROM detection failed).
    #[error("invalid SNES ROM header: {0}")]
    Header(String),
    /// The image looked like a zip archive but couldn't be opened, or contained no recognizable
    /// SNES ROM entry.
    #[error("zip archive: {0}")]
    Archive(String),
}

/// SNES ROM file extensions recognized inside a zip archive, checked case-insensitively
/// (`.sfc`/`.smc` are by far the most common; `.fig`/`.swc` are older copier-header dumps this
/// project's header detection already strips — see `docs/cartridge-format.md`).
const ROM_EXTENSIONS: [&str; 4] = ["sfc", "smc", "fig", "swc"];

/// Hard cap on a zip entry's decompressed size, enforced while reading (not just checked against
/// the (attacker-controlled, spoofable) declared size up front). The largest official SNES ROM is
/// 6 MiB and the largest known fan hack is ~12 MiB; 32 MiB leaves generous headroom while still
/// bounding a "zip bomb" (a small archive that claims/produces a huge decompressed stream) to a
/// sane memory ceiling instead of unbounded growth.
const MAX_DECOMPRESSED_ROM_SIZE: u64 = 32 * 1024 * 1024;

/// If `bytes` is a zip archive (sniffed by the local-file-header magic `PK\x03\x04`, or the
/// empty-archive end-of-central-directory magic `PK\x05\x06`), extract the first non-directory
/// entry whose extension is in [`ROM_EXTENSIONS`] and return its decompressed bytes. Otherwise
/// returns `bytes` unchanged — a plain `.sfc`/`.smc` file passes straight through. Pure in-memory
/// (a `Cursor` over the slice already in hand), so this is identical on native and wasm32.
fn extract_rom_bytes(bytes: &[u8]) -> Result<std::borrow::Cow<'_, [u8]>, EmuError> {
    let is_zip = bytes.len() >= 4 && (bytes[..4] == *b"PK\x03\x04" || bytes[..4] == *b"PK\x05\x06");
    if !is_zip {
        return Ok(std::borrow::Cow::Borrowed(bytes));
    }
    let cursor = std::io::Cursor::new(bytes);
    let mut archive =
        zip::ZipArchive::new(cursor).map_err(|e| EmuError::Archive(format!("{e}")))?;
    let rom_index = (0..archive.len())
        .find(|&i| {
            archive.name_for_index(i).is_some_and(|name| {
                // Directory entries conventionally end with `/` (zip spec) — a directory named
                // e.g. `Game.sfc/` must not match, or extraction below fails on an empty read.
                !name.ends_with('/')
                    && std::path::Path::new(name)
                        .extension()
                        .and_then(|ext| ext.to_str())
                        .is_some_and(|ext| {
                            ROM_EXTENSIONS.iter().any(|r| ext.eq_ignore_ascii_case(r))
                        })
            })
        })
        .ok_or_else(|| {
            EmuError::Archive(format!(
                "no .sfc/.smc/.fig/.swc entry found (tried {} entries)",
                archive.len()
            ))
        })?;
    let mut entry = archive
        .by_index(rom_index)
        .map_err(|e| EmuError::Archive(format!("{e}")))?;
    // `read_to_end` grows the buffer as needed; no need to pre-size from `entry.size()` (a
    // `u64` that would need a lossy cast on 32-bit targets for a capacity hint only) — and that
    // declared size is attacker-controlled anyway, which is exactly what `take` below guards
    // against: capping the ACTUAL bytes read, not trusting the header's claim.
    let mut limited = std::io::Read::take(&mut entry, MAX_DECOMPRESSED_ROM_SIZE + 1);
    let mut out = Vec::new();
    std::io::Read::read_to_end(&mut limited, &mut out)
        .map_err(|e| EmuError::Archive(format!("{e}")))?;
    if out.len() as u64 > MAX_DECOMPRESSED_ROM_SIZE {
        return Err(EmuError::Archive(format!(
            "entry exceeds the {MAX_DECOMPRESSED_ROM_SIZE}-byte decompressed-size limit \
             (zip bomb protection)"
        )));
    }
    Ok(std::borrow::Cow::Owned(out))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn blank_core_presents_cleared_frame_of_region_size() {
        let core = EmuCore::new(0, Region::Ntsc);
        let (w, h) = core.fb_dims();
        assert_eq!((w, h), (256, 224));
        assert_eq!(core.framebuffer().len(), (256 * 224 * 4) as usize);
        assert!(core.framebuffer().iter().all(|&b| b == 0));
        assert!(!core.rom_loaded());
        assert!(core.audio().is_empty());
    }

    #[test]
    fn empty_rom_rejected() {
        let mut core = EmuCore::new(0, Region::Ntsc);
        assert!(matches!(core.load_rom(&[]), Err(EmuError::Empty)));
    }

    #[test]
    fn run_frame_does_not_panic_without_rom() {
        let mut core = EmuCore::new(0, Region::Ntsc);
        core.run_frame();
    }

    #[test]
    fn debug_snapshot_of_blank_core_has_no_cart() {
        let mut core = EmuCore::new(0, Region::Ntsc);
        let snap = core.debug_snapshot();
        assert_eq!(snap.cart.board_name, None);
        assert_eq!(snap.cart.sa1, None);
        assert!(snap.cart.gsu.is_none());
        // Power-on 65C816 state (`rustysnes_cpu::Regs::new`): emulation mode, S parked at $01FF.
        assert!(snap.cpu.emulation);
        assert_eq!(snap.cpu.s, 0x01FF);
    }

    #[test]
    fn debug_snapshot_vram_scroll_moves_the_window() {
        let mut core = EmuCore::new(0, Region::Ntsc);
        assert_eq!(core.debug_snapshot().ppu.vram_window_start, 0);
        core.set_debug_vram_scroll(0x1234);
        assert_eq!(core.debug_snapshot().ppu.vram_window_start, 0x1234);
    }

    fn zip_containing(name: &str, bytes: &[u8]) -> Vec<u8> {
        let mut buf = std::io::Cursor::new(Vec::new());
        let mut writer = zip::ZipWriter::new(&mut buf);
        writer
            .start_file(name, zip::write::SimpleFileOptions::default())
            .unwrap();
        std::io::Write::write_all(&mut writer, bytes).unwrap();
        writer.finish().unwrap();
        buf.into_inner()
    }

    #[test]
    fn non_zip_bytes_pass_through_unchanged() {
        let rom = b"not a zip, just a plain ROM image";
        let out = extract_rom_bytes(rom).unwrap();
        assert_eq!(&*out, rom);
    }

    #[test]
    fn zip_wrapped_rom_is_transparently_extracted() {
        let rom = vec![0xAB_u8; 512];
        let zipped = zip_containing("Game.sfc", &rom);
        let out = extract_rom_bytes(&zipped).unwrap();
        assert_eq!(&*out, rom.as_slice());
    }

    #[test]
    fn zip_with_no_rom_entry_errors() {
        let zipped = zip_containing("readme.txt", b"not a ROM");
        assert!(matches!(
            extract_rom_bytes(&zipped),
            Err(EmuError::Archive(_))
        ));
    }

    #[test]
    fn zip_directory_entry_named_like_a_rom_is_not_matched() {
        // A directory entry conventionally ends with `/` in the zip central directory; a folder
        // literally named "Game.sfc" must not be picked over (or instead of) a real ROM entry.
        let mut buf = std::io::Cursor::new(Vec::new());
        let mut writer = zip::ZipWriter::new(&mut buf);
        writer
            .add_directory("Game.sfc/", zip::write::SimpleFileOptions::default())
            .unwrap();
        let rom = vec![0xCD_u8; 128];
        writer
            .start_file("Real Game.sfc", zip::write::SimpleFileOptions::default())
            .unwrap();
        std::io::Write::write_all(&mut writer, &rom).unwrap();
        writer.finish().unwrap();
        let zipped = buf.into_inner();
        let out = extract_rom_bytes(&zipped).unwrap();
        assert_eq!(&*out, rom.as_slice());
    }

    #[test]
    fn oversized_zip_entry_is_rejected_not_read_unbounded() {
        let huge = vec![0u8; usize::try_from(MAX_DECOMPRESSED_ROM_SIZE + 1).unwrap()];
        let zipped = zip_containing("Big.sfc", &huge);
        assert!(matches!(
            extract_rom_bytes(&zipped),
            Err(EmuError::Archive(_))
        ));
    }

    #[test]
    fn zip_wrapped_rom_loads_end_to_end() {
        let zipped = zip_containing("test.sfc", &minimal_lorom());
        let mut core = EmuCore::new(0, Region::Ntsc);
        core.load_rom(&zipped).expect("zip-wrapped ROM should load");
        assert!(core.rom_loaded());
    }

    /// A minimal-but-valid, all-zero-body LoROM image with just enough of a header at $7FC0 for
    /// `Cart::from_rom` to accept it (mirrors `rustysnes-cart::header`'s permissive scoring —
    /// only the size/map-mode bytes need to line up). An all-zero body means every instruction
    /// fetch decodes to `BRK` ($00), and the reset vector ($00FFFC/D, file offset `0x7FFC`,
    /// itself zero) starts the CPU at `PC=$0000` — a real, deterministic, endlessly-looping
    /// BRK-storm, useful for exercising the debugger's breakpoint/step machinery without needing
    /// a real commercial or test ROM.
    fn minimal_lorom() -> Vec<u8> {
        let mut rom = vec![0u8; 0x8000];
        rom[0x7FC0..0x7FC0 + 21].copy_from_slice(b"TEST ROM             ");
        rom[0x7FD5] = 0x20; // LoROM
        rom[0x7FD6] = 0x00; // no coprocessor
        rom[0x7FD7] = 0x08; // ROM size (2^8 KiB = 256 KiB, permissive)
        rom
    }

    fn minimal_lorom_core() -> EmuCore {
        let mut core = EmuCore::new(0, Region::Ntsc);
        core.load_rom(&minimal_lorom())
            .expect("minimal LoROM should load");
        core
    }

    #[test]
    fn breakpoint_pauses_at_the_armed_pc() {
        let mut core = minimal_lorom_core();
        core.set_breakpoints(&[0x00_0000]);
        core.run_frame();
        assert!(
            core.is_paused(),
            "the all-zero ROM's BRK-storm sits at PC=0, so an armed breakpoint there must fire"
        );
        assert_eq!(core.pbr_pc(), 0x00_0000);
    }

    #[test]
    fn no_breakpoints_never_pauses() {
        let mut core = minimal_lorom_core();
        core.run_frame();
        assert!(!core.is_paused());
    }

    #[test]
    fn step_into_is_a_no_op_unless_paused() {
        let mut core = minimal_lorom_core();
        core.run_frame();
        let cycles_before = core.system_mut().cpu.cycles;
        core.step_into();
        assert_eq!(
            core.system_mut().cpu.cycles,
            cycles_before,
            "step_into must not advance the System while not paused"
        );
        core.pause();
        core.step_into();
        assert!(
            core.system_mut().cpu.cycles > cycles_before,
            "step_into should advance exactly one instruction once paused"
        );
    }

    #[test]
    fn resume_lets_run_frame_advance_again() {
        let mut core = minimal_lorom_core();
        core.pause();
        core.run_frame();
        let cycles_paused = core.system_mut().cpu.cycles;
        core.resume();
        core.run_frame();
        assert!(
            core.system_mut().cpu.cycles > cycles_paused,
            "run_frame should advance the System again once resumed"
        );
    }

    #[test]
    fn breakpoints_sync_replaces_the_previous_set() {
        let mut core = minimal_lorom_core();
        core.set_breakpoints(&[0x00_0000]);
        core.set_breakpoints(&[]); // replace with an empty set
        core.run_frame();
        assert!(
            !core.is_paused(),
            "clearing the breakpoint list should stop it from firing"
        );
    }

    #[test]
    fn disassembly_window_starts_at_pc_and_is_full_length() {
        let mut core = minimal_lorom_core();
        core.run_frame();
        let snap = core.debug_snapshot();
        assert_eq!(snap.disassembly.len(), DISASSEMBLY_WINDOW_LEN);
        assert_eq!(snap.disassembly[0].0, core.pbr_pc());
        // An all-zero ROM decodes entirely as BRK (Immediate8 mode: opcode + one signature byte).
        assert!(snap.disassembly.iter().all(|(_, text)| text == "BRK #$00"));
    }

    #[test]
    fn debug_snapshot_reports_paused_state() {
        let mut core = minimal_lorom_core();
        assert!(!core.debug_snapshot().paused);
        core.pause();
        assert!(core.debug_snapshot().paused);
    }
}
