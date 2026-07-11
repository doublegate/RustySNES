//! The emulation core wrapper the frontend drives.
//!
//! Loads a ROM, steps a frame, exposes the framebuffer + audio for the present path, plus the
//! debugger overlay (breakpoints/single-step/register snapshots) that isn't part of every
//! consumer's needs.
//!
//! `v1.2.0`: the pure facade (load/reset/run_frame/framebuffer/audio/save-state — everything a
//! headless embedder like `rustysnes-libretro` also needs) relocated to
//! [`rustysnes_core::facade::EmuCore`] (wrong dependency weight — winit/wgpu/cpal/egui — for a
//! libretro core). This type is now a thin wrapper: an `inner: rustysnes_core::facade::EmuCore`
//! plus the debugger-only fields (the VRAM viewer scroll position, `breakpoints`, `paused`) that
//! stay frontend-side since they return this crate's own [`DebugSnapshot`] types. The determinism
//! contract lives in the core; the frontend NEVER injects timing/RNG into synthesis — rate
//! control + run-ahead are pure frontend concerns (in `app.rs` / `audio.rs`).

use rustysnes_core::cpu::disasm::disassemble_one;
use rustysnes_core::facade;
pub use rustysnes_core::facade::EmuError;
use rustysnes_core::scheduler::System;

use crate::config::Region;
use crate::debug_snapshot::{
    ApuSnapshot, CartSnapshot, DebugSnapshot, GsuSnapshot, PpuSnapshot, VRAM_WINDOW_LEN,
    VoiceSnapshot, WatchHit,
};
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

/// The frontend's view of the emulator. Lives behind an `Arc<Mutex<…>>` on native (shared with
/// the dedicated emulation thread) and is stepped synchronously on wasm.
pub struct EmuCore {
    /// The pure facade (`v1.2.0`, relocated to `rustysnes-core`) — every non-debugger method
    /// below is a one-line delegation to this.
    inner: facade::EmuCore,
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
    /// The active HD texture pack (`v1.3.0`, `hd-pack` feature), if any — see
    /// [`Self::set_hd_pack`]. `None` (the default) means [`rustysnes_core::ppu::Ppu::
    /// set_hd_pack_tagging`] is off, so the PPU's `TileTag` side-buffer is never populated and
    /// `run_frame`'s cost is unaffected, matching the core's own byte-identical-when-off
    /// guarantee (`docs/ppu.md`).
    #[cfg(feature = "hd-pack")]
    hd_pack: Option<crate::hd_pack::HdPack>,
}

/// Translate the frontend's own [`Region`] (config/pacing concerns: `frame_rate`, serde) to the
/// core's canonical [`rustysnes_core::cart::header::Region`] (auto-detected from the ROM header).
const fn to_core_region(region: Region) -> rustysnes_core::cart::Region {
    match region {
        Region::Ntsc => rustysnes_core::cart::Region::Ntsc,
        Region::Pal => rustysnes_core::cart::Region::Pal,
    }
}

/// The reverse of [`to_core_region`].
const fn from_core_region(region: rustysnes_core::cart::Region) -> Region {
    match region {
        rustysnes_core::cart::Region::Ntsc => Region::Ntsc,
        rustysnes_core::cart::Region::Pal => Region::Pal,
    }
}

impl EmuCore {
    /// Power on with a determinism seed and a region. No ROM is loaded yet.
    #[must_use]
    pub fn new(seed: u64, region: Region) -> Self {
        Self {
            inner: facade::EmuCore::new(seed, to_core_region(region)),
            debug_vram_scroll: 0,
            breakpoints: Vec::new(),
            paused: false,
            #[cfg(feature = "hd-pack")]
            hd_pack: None,
        }
    }

    /// Load a raw ROM image — see [`facade::EmuCore::load_rom`].
    ///
    /// Clears any active HD texture pack (`v1.3.0`, `hd-pack` feature) — a pack is keyed to the
    /// ROM it was discovered under, so it never carries over to a newly-loaded, different ROM
    /// (the freshly (re)constructed `System` this creates also already has `Ppu::
    /// set_hd_pack_tagging` at its `false` default, so no explicit toggle is needed here).
    ///
    /// # Errors
    /// See [`facade::EmuCore::load_rom`].
    pub fn load_rom(&mut self, rom: &[u8]) -> Result<(), EmuError> {
        let result = self.inner.load_rom(rom);
        #[cfg(feature = "hd-pack")]
        {
            self.hd_pack = None;
        }
        result
    }

    /// The coprocessor firmware dumps this cart will accept — see
    /// [`facade::EmuCore::firmware_candidates`].
    #[must_use]
    pub fn firmware_candidates(&self) -> &'static [&'static str] {
        self.inner.firmware_candidates()
    }

    /// Whether the loaded cart needs a (not-yet-installed) firmware dump — see
    /// [`facade::EmuCore::needs_firmware`].
    #[must_use]
    pub fn needs_firmware(&self) -> bool {
        self.inner.needs_firmware()
    }

    /// Supply a coprocessor firmware dump — see [`facade::EmuCore::install_firmware`].
    pub fn install_firmware(&mut self, bytes: &[u8]) -> bool {
        self.inner.install_firmware(bytes)
    }

    /// Restore battery SRAM — see [`facade::EmuCore::load_sram`].
    pub fn load_sram(&mut self, data: &[u8]) {
        self.inner.load_sram(data);
    }

    /// The current battery SRAM contents — see [`facade::EmuCore::save_sram`].
    #[must_use]
    pub fn save_sram(&self) -> &[u8] {
        self.inner.save_sram()
    }

    /// Soft reset — see [`facade::EmuCore::reset`].
    pub fn reset(&mut self) {
        self.inner.reset();
    }

    /// Power-cycle (hard reset) — see [`facade::EmuCore::power_cycle`].
    ///
    /// The same ROM stays loaded, so an active HD texture pack (`v1.3.0`, `hd-pack` feature)
    /// stays active too — but `power_cycle` (re)constructs the underlying `System`/`Ppu` from
    /// scratch, whose tagging flag defaults back to `false`, so it's re-enabled here to match
    /// [`Self::hd_pack_name`]'s still-`Some` state.
    pub fn power_cycle(&mut self) {
        self.inner.power_cycle();
        #[cfg(feature = "hd-pack")]
        if self.hd_pack.is_some() {
            self.inner.system_mut().bus.ppu.set_hd_pack_tagging(true);
        }
    }

    /// Close the loaded ROM — see [`facade::EmuCore::close_rom`].
    ///
    /// Clears any active HD texture pack (`v1.3.0`, `hd-pack` feature) along with it — same
    /// reasoning as [`Self::load_rom`].
    pub fn close_rom(&mut self) {
        self.inner.close_rom();
        #[cfg(feature = "hd-pack")]
        {
            self.hd_pack = None;
        }
    }

    /// Whether a ROM is loaded.
    #[must_use]
    pub const fn rom_loaded(&self) -> bool {
        self.inner.rom_loaded()
    }

    /// The loaded cartridge's board name, if any.
    #[must_use]
    pub fn cart_name(&self) -> Option<&'static str> {
        self.inner.cart_name()
    }

    /// The raw ROM byte image currently loaded — see [`facade::EmuCore::rom`].
    #[must_use]
    pub fn rom(&self) -> &[u8] {
        self.inner.rom()
    }

    /// Every HD texture pack name available for the currently-loaded ROM (`v1.3.0`, `hd-pack`
    /// feature) — the candidate list for a Settings pack-selector UI. Empty if no ROM is loaded
    /// or no data directory is resolvable (always the case on `wasm32`).
    #[cfg(feature = "hd-pack")]
    #[must_use]
    pub fn available_hd_packs(&self) -> Vec<String> {
        if !self.rom_loaded() {
            return Vec::new();
        }
        crate::hd_pack::discover_packs(&rustysnes_core::movie::hash_rom(self.rom()))
    }

    /// The active HD texture pack's manifest name, if one is loaded.
    #[cfg(feature = "hd-pack")]
    #[must_use]
    pub fn hd_pack_name(&self) -> Option<&str> {
        self.hd_pack.as_ref().map(|p| p.manifest.name.as_str())
    }

    /// Select (or clear, with `None`) the active HD texture pack for the current ROM.
    ///
    /// Loads `pack_name` from `<data_dir>/hd-packs/<rom_sha256_hex>/<pack_name>/`, enables
    /// [`rustysnes_core::ppu::Ppu::set_hd_pack_tagging`] on success, and disables it (clearing
    /// any previously-active pack) on `None` or on a load failure — a pack either becomes fully
    /// active or the emulator falls all the way back to native rendering, never a half-applied
    /// state.
    ///
    /// # Errors
    /// See [`crate::hd_pack::HdPackError`]'s variants; [`crate::hd_pack::HdPackError::Io`] if no
    /// ROM is loaded.
    #[cfg(feature = "hd-pack")]
    pub fn set_hd_pack(
        &mut self,
        pack_name: Option<&str>,
    ) -> Result<(), crate::hd_pack::HdPackError> {
        let Some(name) = pack_name else {
            self.hd_pack = None;
            self.inner.system_mut().bus.ppu.set_hd_pack_tagging(false);
            return Ok(());
        };
        if !self.rom_loaded() {
            return Err(crate::hd_pack::HdPackError::Io(std::io::Error::other(
                "no ROM loaded",
            )));
        }
        let rom_sha256 = rustysnes_core::movie::hash_rom(self.rom());
        match crate::hd_pack::load_pack(&rom_sha256, name) {
            Ok(pack) => {
                self.hd_pack = Some(pack);
                self.inner.system_mut().bus.ppu.set_hd_pack_tagging(true);
                Ok(())
            }
            Err(e) => {
                self.hd_pack = None;
                self.inner.system_mut().bus.ppu.set_hd_pack_tagging(false);
                Err(e)
            }
        }
    }

    /// Direct mutable access to the deterministic core, for TAS movie record/playback
    /// (`rustysnes_core::movie`) and Lua scripting (`rustysnes_script::ScriptEngine`) — both need
    /// genuine read/write reach into the running `System`/`Bus`, unlike this type's own read-only
    /// [`Self::debug_snapshot`] copy.
    // Deliberately not `const fn` (PR #62 review): trivially const today, but fragile on a
    // struct this size, and no longer possible anyway since `facade::EmuCore::system_mut` isn't
    // `const` either (see that method's own comment).
    #[allow(clippy::missing_const_for_fn)]
    pub fn system_mut(&mut self) -> &mut System {
        self.inner.system_mut()
    }

    /// Latch the controller state for a player (`0` = P1, `1` = P2). Late-latched by the window
    /// handler each frame; applied to the Bus at the top of [`Self::run_frame`].
    pub fn set_pad(&mut self, player: usize, buttons: Buttons) {
        self.inner.set_pad(player, buttons.sanitize_dpad().0);
    }

    /// Select which peripheral is attached to controller port `port` — see
    /// [`facade::EmuCore::set_port_device`].
    pub fn set_port_device(&mut self, port: usize, device: rustysnes_core::controller::PortDevice) {
        self.inner.set_port_device(port, device);
    }

    /// Feed one frame's worth of SNES Mouse input for port `port` — see
    /// [`facade::EmuCore::set_mouse`].
    pub fn set_mouse(&mut self, port: usize, dx: i32, dy: i32, left: bool, right: bool) {
        self.inner.set_mouse(port, dx, dy, left, right);
    }

    /// Set the 8 per-voice audio mute toggles (`v1.0.1`) — see
    /// [`facade::EmuCore::set_voice_mutes`].
    // Deliberately not `const fn` (PR #62 review) — same rationale as `Self::system_mut` above.
    #[allow(clippy::missing_const_for_fn)]
    pub fn set_voice_mutes(&mut self, mutes: [bool; 8]) {
        self.inner.set_voice_mutes(mutes);
    }

    /// Feed one frame's worth of Super Scope input for port `port` — see
    /// [`facade::EmuCore::set_superscope`].
    pub fn set_superscope(&mut self, port: usize, x: i32, y: i32, buttons: u8) {
        self.inner.set_superscope(port, x, y, buttons);
    }

    /// Feed one frame's worth of input for Super Multitap sub-pad `sub_index` of port `port` —
    /// see [`facade::EmuCore::set_multitap_pad`].
    pub fn set_multitap_pad(&mut self, port: usize, sub_index: usize, buttons: u16) {
        self.inner.set_multitap_pad(port, sub_index, buttons);
    }

    /// Advance one full video frame: feed the latched pads to the Bus, run the scheduler to the
    /// next frame boundary, then decode the PPU framebuffer + drain the S-DSP audio. A no-op
    /// (beyond re-presenting the already-current frame) while [`Self::is_paused`] — the debugger
    /// owns advancing the `System` in that state, via [`Self::step_into`]/[`Self::step_over`].
    pub fn run_frame(&mut self) {
        self.inner.apply_pads();
        if !self.paused {
            if self.breakpoints.is_empty() {
                self.inner.system_mut().run_frame();
            } else {
                self.run_frame_checking_breakpoints();
            }
        }
        self.inner.present_current_frame();
    }

    /// Decode the PPU framebuffer + drain the S-DSP audio from the `System`'s CURRENT state,
    /// without advancing it — see [`facade::EmuCore::present_current_frame`]. Netplay
    /// (`rustysnes_netplay::RollbackSession::advance`) drives `System::run_frame` directly (it
    /// operates on the core crate, not this frontend type), so it calls this afterward to pick up
    /// the result.
    pub fn present_current_frame(&mut self) {
        self.inner.present_current_frame();
    }

    /// [`Self::run_frame`]'s slow path: steps one instruction at a time (mirroring
    /// `System::run_frame`'s own loop exactly — same frame-boundary condition, same SA-1
    /// catch-up), stopping early and setting [`Self::paused`] the instant the CPU's `pbr:pc`
    /// matches an armed breakpoint. Only reached when at least one breakpoint is armed, so the
    /// default (no breakpoints) path above is completely unaffected.
    fn run_frame_checking_breakpoints(&mut self) {
        if self.inner.system_mut().bus.cart.is_none() {
            return; // matches `System::run_frame`'s own early return.
        }
        let start_frame = self.inner.system_mut().bus.ppu.frame_count();
        let mut steps = 0u32;
        while self.inner.system_mut().bus.ppu.frame_count() == start_frame
            && steps < MAX_STEP_OVER_INSTRUCTIONS
        {
            self.inner.system_mut().step_instruction();
            steps += 1;
            let pc = self.pbr_pc();
            if self.breakpoints.contains(&pc) {
                self.paused = true;
                return;
            }
        }
    }

    /// The CPU's current `pbr:pc` as one 24-bit address (`$bank:offset`).
    #[must_use]
    pub fn pbr_pc(&mut self) -> u32 {
        let regs = self.inner.system_mut().cpu.regs;
        (u32::from(regs.pbr) << 16) | u32::from(regs.pc)
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
        if !self.paused || self.inner.system_mut().bus.cart.is_none() {
            return;
        }
        self.inner.system_mut().step_instruction();
        self.inner.present_current_frame();
    }

    /// Step over the instruction at the current PC — a plain [`Self::step_into`] unless it's a
    /// subroutine call (`JSR`/`JSL`), in which case this runs (breakpoint-checked, same as
    /// `Self::run_frame_checking_breakpoints`) until control returns to the instruction right
    /// after the call, bounded by `MAX_STEP_OVER_INSTRUCTIONS` so a subroutine that never
    /// returns (or self-modifying code) can't hang the debugger — it simply stops there, still
    /// paused, same as hitting the instruction budget mid-subroutine.
    pub fn step_over(&mut self) {
        if !self.paused || self.inner.system_mut().bus.cart.is_none() {
            return;
        }
        let system = self.inner.system_mut();
        let start_pbr = system.cpu.regs.pbr;
        let start_pc = system.cpu.regs.pc;
        let (text, len) = disassemble_one(
            |addr| system.bus.peek(addr),
            start_pbr,
            start_pc,
            system.cpu.regs.m8(),
            system.cpu.regs.x8(),
        );
        if !(text.starts_with("JSR") || text.starts_with("JSL")) {
            self.step_into();
            return;
        }
        #[allow(clippy::cast_possible_truncation)]
        let return_pc = start_pc.wrapping_add(len as u16);
        let mut steps = 0u32;
        while steps < MAX_STEP_OVER_INSTRUCTIONS {
            self.inner.system_mut().step_instruction();
            steps += 1;
            let system = self.inner.system_mut();
            if system.cpu.regs.pbr == start_pbr && system.cpu.regs.pc == return_pc {
                break;
            }
            let pc = self.pbr_pc();
            if self.breakpoints.contains(&pc) {
                break; // stay paused on a breakpoint hit mid-subroutine, same as a full run.
            }
        }
        self.inner.present_current_frame();
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
        let system = self.inner.system_mut();
        let pbr = system.cpu.regs.pbr;
        let mut pc = system.cpu.regs.pc;
        let mut m8 = system.cpu.regs.m8();
        let mut x8 = system.cpu.regs.x8();
        for _ in 0..DISASSEMBLY_WINDOW_LEN {
            let system = self.inner.system_mut();
            let addr = (u32::from(pbr) << 16) | u32::from(pc);
            let opcode = system.bus.peek(addr);
            let (text, len) = disassemble_one(|a| system.bus.peek(a), pbr, pc, m8, x8);
            if len == 2 && (opcode == 0xC2 || opcode == 0xE2) {
                let operand = system.bus.peek(addr.wrapping_add(1));
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

    /// The current RGBA8 framebuffer slice — see [`facade::EmuCore::framebuffer`].
    #[must_use]
    pub fn framebuffer(&self) -> &[u8] {
        self.inner.framebuffer()
    }

    /// The audio samples produced during the most recent [`Self::run_frame`] — see
    /// [`facade::EmuCore::audio`].
    #[must_use]
    pub fn audio(&self) -> &[(i16, i16)] {
        self.inner.audio()
    }

    /// The active framebuffer dimensions `(w, h)`.
    #[must_use]
    pub const fn fb_dims(&self) -> (u32, u32) {
        self.inner.fb_dims()
    }

    /// The active region.
    #[must_use]
    pub const fn region(&self) -> Region {
        from_core_region(self.inner.region())
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
        let vram_window_start = self.debug_vram_scroll;
        let system = self.inner.system_mut();
        let ppu = &system.bus.ppu;
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

        let apu = &system.bus.apu;
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

        let board = system.bus.cart.as_ref().map(|c| &c.board);
        let cart = CartSnapshot {
            board_name: board.as_ref().map(|b| b.name()),
            sa1: system.sa1_regs(),
            gsu: board
                .as_ref()
                .and_then(|b| b.debug_gsu_state())
                .map(|(r, sfr, pbr)| GsuSnapshot { r, sfr, pbr }),
        };

        let cpu = system.cpu.regs;
        let ppu_snapshot = PpuSnapshot {
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
        };
        let apu_snapshot = ApuSnapshot {
            smp_pc: apu.smp_pc(),
            smp_stopped: apu.smp_stopped(),
            voices,
        };

        DebugSnapshot {
            cpu,
            ppu: ppu_snapshot,
            apu: apu_snapshot,
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
        self.inner
            .system_mut()
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

    /// Snapshot the full deterministic core state — see [`facade::EmuCore::save_state`].
    #[must_use]
    pub fn save_state(&self) -> Vec<u8> {
        self.inner.save_state()
    }

    /// Restore a snapshot taken by [`Self::save_state`] — see [`facade::EmuCore::load_state`].
    ///
    /// # Errors
    /// See [`facade::EmuCore::load_state`].
    pub fn load_state(&mut self, bytes: &[u8]) -> Result<(), rustysnes_savestate::SaveStateError> {
        self.inner.load_state(bytes)
    }
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

    // --- HD texture pack management (`v1.3.0`, `hd-pack` feature) ---
    //
    // Deliberately hermetic: `crate::hd_pack`'s free functions (`discover_packs`/`load_pack`) hit
    // the real platform data directory with no override hook (mirroring `save_states.rs`'s own
    // `base_dir()` design), so these tests only exercise the paths that never touch disk. The
    // real load-success/failure round trip is covered at `crate::hd_pack`'s own module level
    // (`HdPack::load` against a temp dir) and was verified end-to-end via a manual headless
    // (`xvfb-run`) launch against both a real and a missing pack during development.

    #[cfg(feature = "hd-pack")]
    #[test]
    fn available_hd_packs_is_empty_without_a_rom_loaded() {
        let core = EmuCore::new(0, Region::Ntsc);
        assert!(core.available_hd_packs().is_empty());
    }

    #[cfg(feature = "hd-pack")]
    #[test]
    fn hd_pack_name_is_none_by_default() {
        let core = EmuCore::new(0, Region::Ntsc);
        assert_eq!(core.hd_pack_name(), None);
    }

    #[cfg(feature = "hd-pack")]
    #[test]
    fn set_hd_pack_none_is_always_ok_and_disables_tagging() {
        let mut core = EmuCore::new(0, Region::Ntsc);
        assert!(core.set_hd_pack(None).is_ok());
        assert_eq!(core.hd_pack_name(), None);
        assert!(!core.system_mut().bus.ppu.hd_pack_tagging());
    }

    #[cfg(feature = "hd-pack")]
    #[test]
    fn set_hd_pack_some_without_a_rom_loaded_is_an_io_error() {
        let mut core = EmuCore::new(0, Region::Ntsc);
        let err = core
            .set_hd_pack(Some("whatever"))
            .expect_err("no ROM is loaded");
        assert!(matches!(err, crate::hd_pack::HdPackError::Io(_)));
        assert_eq!(core.hd_pack_name(), None);
    }

    #[cfg(feature = "hd-pack")]
    #[test]
    fn set_hd_pack_with_an_unknown_name_after_rom_load_fails_closed() {
        let mut core = EmuCore::new(0, Region::Ntsc);
        core.load_rom(&minimal_lorom()).expect("minimal ROM loads");
        let err = core
            .set_hd_pack(Some("definitely-does-not-exist"))
            .expect_err("no such pack exists on this machine");
        assert!(matches!(err, crate::hd_pack::HdPackError::Io(_)));
        assert_eq!(core.hd_pack_name(), None);
        assert!(!core.system_mut().bus.ppu.hd_pack_tagging());
    }

    #[cfg(feature = "hd-pack")]
    #[test]
    fn load_rom_clears_a_previously_active_pack() {
        // Can't get a REAL pack active without touching the real data dir (see the module doc
        // above), but `load_rom`'s own clearing logic runs unconditionally regardless of whether
        // a pack was actually active -- this proves it's at least always safe to call again.
        let mut core = EmuCore::new(0, Region::Ntsc);
        core.load_rom(&minimal_lorom()).expect("minimal ROM loads");
        core.load_rom(&minimal_lorom())
            .expect("reloading the same ROM also succeeds");
        assert_eq!(core.hd_pack_name(), None);
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

    #[test]
    fn zip_wrapped_rom_loads_end_to_end() {
        let zipped = zip_containing("test.sfc", &minimal_lorom());
        let mut core = EmuCore::new(0, Region::Ntsc);
        core.load_rom(&zipped).expect("zip-wrapped ROM should load");
        assert!(core.rom_loaded());
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
