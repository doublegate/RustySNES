//! SNES controller-port peripherals beyond the standard gamepad: Mouse, Super Scope, and Super
//! Multitap (`v0.9.0`, Phase 7's "niche peripherals" exit criterion).
//!
//! Ported from ares' `sfc/controller/{mouse,super-scope,super-multitap}` — real hardware's
//! 2-bit-per-clock (`data1`/`data2`) serial-shift-register protocol per controller port, selected
//! via [`PortDevice`]. [`PortDevice::Gamepad`] (the default, both ports) is this project's
//! original, unchanged single-bit 16-bit-shift-register model (`crate::Bus`'s own `joypad`
//! field) — every other device is opt-in, selected explicitly via [`crate::Bus::set_port_device`],
//! and touches no code on the default path.
//!
//! Each device here owns exactly the same two operations real hardware's controller-port pin 2
//! (clock/latch) and pins 4-5 (`data1`/`data2`) expose: `latch(strobe)` (the `$4016` bit-0 write,
//! wired to BOTH ports simultaneously on real hardware — there is only one physical strobe line)
//! reloads/repacks the device's shift register from its latest host-supplied input; `clock()`
//! shifts one bit (or, for `MultitapState`, one bit from each of two sub-pads at once) out MSB
//! first, refilling with `1` past the real bit count — matching every real SNES serial peripheral's
//! floating/pulled-high behavior once its shift register empties.

use rustysnes_savestate::{SaveReader, SaveStateError, SaveWriter};

/// Which peripheral occupies a controller port (`0` = port 1, `1` = port 2).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum PortDevice {
    /// The original standard SNES pad — [`crate::Bus`]'s own 16-bit `joypad` shift register,
    /// unchanged by this module.
    #[default]
    Gamepad,
    /// SNES Mouse — a 32-bit `data1`-only shift register (relative X/Y + buttons + speed).
    Mouse,
    /// Super Scope light gun — an 8-bit `data1`-only shift register, plus a PPU H/V-counter-latch
    /// side channel driven from `crate::Bus::advance_master` (real hardware: wired only to
    /// controller port 2's IOBIT pin — a Super Scope in port 1 never receives a beam-position
    /// latch, matching ares' own documented hardware note).
    SuperScope,
    /// Super Multitap — four independent standard-pad sub-ports, `data1`/`data2` both live
    /// simultaneously, the currently-addressed pair (`[0,1]` vs `[2,3]`) selected by the shared
    /// IOBIT pin.
    Multitap,
}

impl PortDevice {
    const fn to_tag(self) -> u8 {
        match self {
            Self::Gamepad => 0,
            Self::Mouse => 1,
            Self::SuperScope => 2,
            Self::Multitap => 3,
        }
    }

    const fn from_tag(tag: u8) -> Self {
        match tag {
            1 => Self::Mouse,
            2 => Self::SuperScope,
            3 => Self::Multitap,
            _ => Self::Gamepad,
        }
    }
}

/// SNES Mouse peripheral state (ares `Controller::Mouse`).
#[derive(Debug, Clone, Copy, Default)]
pub(crate) struct MouseState {
    /// Host-supplied relative delta pending the next latch, set once per frame by
    /// [`crate::Bus::set_mouse`] — mirrors [`crate::Bus::set_joypad`]'s "always replace,
    /// re-synced once per frame" convention (unscaled/unclamped host units; the speed multiplier
    /// and 127-magnitude clamp below are applied at latch time, matching real hardware).
    pending_dx: i32,
    pending_dy: i32,
    pending_left: bool,
    pending_right: bool,
    /// 0 = slow (1.0x), 1 = normal (1.5x), 2 = fast (2.0x). Real hardware quirk: this cycles on
    /// every read taken *while strobe is held high*, not via any CPU register write — modeled in
    /// [`Self::clock`].
    speed: u8,
    /// The packed 32-bit shift register, computed at [`Self::latch`] time.
    shift: u32,
    /// Last-observed strobe level; latching is a no-op unless it actually changes (ares'
    /// `if(latched==data) return;`).
    latched: bool,
}

impl MouseState {
    pub(crate) const fn set_input(&mut self, dx: i32, dy: i32, left: bool, right: bool) {
        self.pending_dx = dx;
        self.pending_dy = dy;
        self.pending_left = left;
        self.pending_right = right;
    }

    /// `$4016`/`$4017` bit-0 write. Real hardware re-samples on *every* transition (ares' own
    /// `latch()` doesn't distinguish direction), so this does too.
    #[allow(clippy::similar_names)] // `neg_x`/`neg_y` are the real, distinct hardware direction bits.
    pub(crate) fn latch(&mut self, strobe: bool) {
        if self.latched == strobe {
            return;
        }
        self.latched = strobe;
        let neg_x = u32::from(self.pending_dx < 0);
        let neg_y = u32::from(self.pending_dy < 0);
        let multiplier: f64 = match self.speed {
            1 => 1.5,
            2 => 2.0,
            _ => 1.0,
        };
        #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
        let cx = (f64::from(self.pending_dx.unsigned_abs()) * multiplier).min(127.0) as u32;
        #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
        let cy = (f64::from(self.pending_dy.unsigned_abs()) * multiplier).min(127.0) as u32;
        self.shift = (u32::from(self.pending_right) << 23)
            | (u32::from(self.pending_left) << 22)
            | (u32::from((self.speed >> 1) & 1) << 21)
            | (u32::from(self.speed & 1) << 20)
            | (1 << 16) // device signature 0,0,0,1 (bits 19..16)
            | (neg_y << 15)
            | ((cy & 0x7F) << 8)
            | (neg_x << 7)
            | (cx & 0x7F);
    }

    /// `$4016`/`$4017` clock (one bit per read).
    pub(crate) const fn clock(&mut self) -> u8 {
        if self.latched {
            self.speed = (self.speed + 1) % 3;
            return 0;
        }
        let bit = ((self.shift >> 31) & 1) as u8;
        self.shift = (self.shift << 1) | 1;
        bit
    }

    fn save_state(self, s: &mut SaveWriter) {
        s.write_u8(self.speed);
        s.write_u32(self.shift);
        s.write_bool(self.latched);
    }

    fn load_state(s: &mut SaveReader) -> Result<Self, SaveStateError> {
        Ok(Self {
            speed: s.read_u8()?,
            shift: s.read_u32()?,
            latched: s.read_bool()?,
            ..Self::default()
        })
    }
}

/// Super Scope button bitmask constants for [`crate::Bus::set_superscope`] — real, independent
/// physical switches, not mutually-exclusive states, hence a bitmask rather than an enum.
pub mod scope {
    /// The trigger.
    pub const TRIGGER: u8 = 1 << 0;
    /// The cursor button (a secondary "select" button on the gun's body).
    pub const CURSOR: u8 = 1 << 1;
    /// The Turbo switch — toggles the trigger into level-sensitive (rapid-fire) mode.
    pub const TURBO: u8 = 1 << 2;
    /// The Pause button.
    pub const PAUSE: u8 = 1 << 3;
}

/// Super Scope light-gun peripheral state (ares `Controller::SuperScope`). Genuinely needs this
/// many independent booleans — each is a distinct real hardware latch/edge-detector, not an API
/// design choice (`crate::controller::scope`'s public bitmask keeps the *external* API to one
/// packed byte instead).
#[allow(clippy::struct_excessive_bools)]
#[derive(Debug, Clone, Copy, Default)]
pub(crate) struct SuperScopeState {
    /// Host-supplied absolute screen-space target. Clamped to ares' own `-16..=256+16`/
    /// `-16..=240+16` fringe (the small negative/over-max margin is what makes off-screen
    /// detection possible — a hard `0..255` clamp would make "aim off-screen" unrepresentable).
    x: i32,
    y: i32,
    /// Live physical button/switch state, a `scope::{TRIGGER,CURSOR,TURBO,PAUSE}` bitmask.
    buttons: u8,

    latched: bool,
    shift: u16,
    /// Set on every latch transition (ares' `counter=0`), cleared and acted on the next time
    /// [`Self::clock`] is called — the button/edge sampling must happen exactly ONCE per read
    /// pass (ares' `if(counter==0)` inside `data()`, not inside `latch()`), since a real read
    /// pass is always a strobe-HIGH write followed by a strobe-LOW write before any clocking
    /// happens; sampling inside `latch()` itself would run the (stateful) edge-detectors twice —
    /// once per transition — corrupting `trigger_lock`/`turbo_prev`/`pause_lock`.
    pending_sample: bool,

    turbo_edge: bool,
    turbo_prev: bool,
    trigger_lock: bool,
    pause_edge: bool,
    pause_lock: bool,
}

impl SuperScopeState {
    pub(crate) fn set_input(&mut self, x: i32, y: i32, buttons: u8) {
        self.x = x.clamp(-16, 256 + 16);
        self.y = y.clamp(-16, 240 + 16);
        self.buttons = buttons;
    }

    fn offscreen(&self, visible_height: u16) -> bool {
        self.x < 0 || self.y < 0 || self.x >= 256 || self.y >= i32::from(visible_height)
    }

    /// `$4016`/`$4017` bit-0 write.
    pub(crate) const fn latch(&mut self, strobe: bool) {
        if self.latched == strobe {
            return;
        }
        self.latched = strobe;
        self.pending_sample = true;
    }

    /// The button/edge sample this read pass will use, computed once (see [`Self::pending_sample`]'s
    /// doc) then packed into [`Self::shift`].
    fn sample(&mut self, visible_height: u16) {
        self.pending_sample = false;

        let turbo = self.buttons & scope::TURBO != 0;
        if turbo && !self.turbo_prev {
            self.turbo_edge = !self.turbo_edge;
        }
        self.turbo_prev = turbo;

        let trigger = self.buttons & scope::TRIGGER != 0;
        let mut trigger_value = false;
        if trigger && (self.turbo_edge || !self.trigger_lock) {
            trigger_value = true;
            self.trigger_lock = true;
        } else if !trigger {
            self.trigger_lock = false;
        }

        let pause = self.buttons & scope::PAUSE != 0;
        self.pause_edge = false;
        if pause && !self.pause_lock {
            self.pause_edge = true;
            self.pause_lock = true;
        } else if !pause {
            self.pause_lock = false;
        }

        let cursor = self.buttons & scope::CURSOR != 0;
        let offscreen = self.offscreen(visible_height);
        self.shift = (u16::from(trigger_value && !offscreen) << 15)
            | (u16::from(cursor) << 14)
            | (u16::from(self.turbo_edge) << 13)
            | (u16::from(self.pause_edge) << 12)
            | (u16::from(offscreen) << 9)
            | 0x00FF; // bits 7..0 pre-filled `1`, matching real hardware's post-bit-8 free-run.
    }

    pub(crate) fn clock(&mut self, visible_height: u16) -> u8 {
        if self.pending_sample {
            self.sample(visible_height);
        }
        let bit = ((self.shift >> 15) & 1) as u8;
        self.shift = (self.shift << 1) | 1;
        bit
    }

    /// The beam position (in this project's dot-space, `dot = master_clock / 4`) this Super Scope
    /// is "aimed at", for [`crate::Bus`]'s per-master-clock H/V-counter auto-latch check — `None`
    /// while aimed off-screen (real hardware: the light sensor simply never fires,
    /// `if(!offscreen) { ... }`).
    pub(crate) fn beam_target(&self, visible_height: u16) -> Option<(u16, u16)> {
        if self.offscreen(visible_height) {
            return None;
        }
        // +24 dots is the light sensor's own fixed detection delay (ares' `(cx+24)*4` hcounter
        // offset — already dot-space here since our `dot()` is hcounter/4, so the `*4`/`/4`
        // cancel).
        #[allow(clippy::cast_sign_loss, clippy::cast_possible_truncation)]
        Some((self.y as u16, (self.x + 24) as u16))
    }

    fn save_state(self, s: &mut SaveWriter) {
        s.write_bool(self.latched);
        s.write_u16(self.shift);
        s.write_bool(self.turbo_edge);
        s.write_bool(self.turbo_prev);
        s.write_bool(self.trigger_lock);
        s.write_bool(self.pause_edge);
        s.write_bool(self.pause_lock);
    }

    fn load_state(s: &mut SaveReader) -> Result<Self, SaveStateError> {
        Ok(Self {
            latched: s.read_bool()?,
            shift: s.read_u16()?,
            turbo_edge: s.read_bool()?,
            turbo_prev: s.read_bool()?,
            trigger_lock: s.read_bool()?,
            pause_edge: s.read_bool()?,
            pause_lock: s.read_bool()?,
            ..Self::default()
        })
    }
}

/// Super Multitap peripheral state (ares `Controller::SuperMultitap`) — four independent
/// standard-pad sub-ports on one physical port, `data1`/`data2` both live simultaneously, the
/// currently-addressed pair selected by the shared IOBIT pin (real hardware: `[0,1]` vs `[2,3]`).
#[derive(Debug, Clone, Copy, Default)]
pub(crate) struct MultitapState {
    /// Each sub-pad's own 16-bit shift register — the exact `crate::Bus::joypad` model, one per
    /// virtual player.
    pads: [u16; 4],
    latched: bool,
}

impl MultitapState {
    /// Reload sub-pad `index`'s (`0..=3`) shift register from a fresh 16-bit button snapshot
    /// (`BYsSUDLR....`, matching [`crate::Bus::set_joypad`]'s own format/convention exactly).
    pub(crate) fn set_pad(&mut self, index: usize, buttons: u16) {
        if let Some(pad) = self.pads.get_mut(index) {
            *pad = buttons;
        }
    }

    pub(crate) fn pad(&self, index: usize) -> u16 {
        self.pads.get(index).copied().unwrap_or(0)
    }

    /// `$4016`/`$4017` bit-0 write.
    pub(crate) const fn latch(&mut self, strobe: bool) {
        self.latched = strobe;
    }

    /// `$4016`/`$4017` clock — returns `(data1, data2)`. While latched, both lines report the
    /// fixed Super Multitap device-detection signature (`data1=0, data2=1`); otherwise the
    /// IOBIT-selected pair's sub-pads each shift one bit (only the SELECTED pair's counters
    /// advance — the other pair's registers stay untouched until they're the ones selected,
    /// matching real hardware exactly).
    pub(crate) const fn clock(&mut self, iobit: bool) -> (u8, u8) {
        if self.latched {
            return (0, 1);
        }
        let (a, b) = if iobit { (0, 1) } else { (2, 3) };
        let bit_a = ((self.pads[a] >> 15) & 1) as u8;
        self.pads[a] = (self.pads[a] << 1) | 1;
        let bit_b = ((self.pads[b] >> 15) & 1) as u8;
        self.pads[b] = (self.pads[b] << 1) | 1;
        (bit_a, bit_b)
    }

    fn save_state(self, s: &mut SaveWriter) {
        for pad in self.pads {
            s.write_u16(pad);
        }
        s.write_bool(self.latched);
    }

    fn load_state(s: &mut SaveReader) -> Result<Self, SaveStateError> {
        let mut pads = [0u16; 4];
        for pad in &mut pads {
            *pad = s.read_u16()?;
        }
        Ok(Self {
            pads,
            latched: s.read_bool()?,
        })
    }
}

/// One controller port's full peripheral state — which [`PortDevice`] is attached, plus that
/// device's own runtime state (the others stay at their default/idle value, costing only the
/// stack space of the largest variant; no allocation).
#[derive(Debug, Clone, Copy, Default)]
pub(crate) struct PortState {
    pub(crate) device: PortDevice,
    pub(crate) mouse: MouseState,
    pub(crate) super_scope: SuperScopeState,
    pub(crate) multitap: MultitapState,
}

impl PortState {
    pub(crate) fn latch(&mut self, strobe: bool) {
        match self.device {
            PortDevice::Gamepad => {}
            PortDevice::Mouse => self.mouse.latch(strobe),
            PortDevice::SuperScope => self.super_scope.latch(strobe),
            PortDevice::Multitap => self.multitap.latch(strobe),
        }
    }

    /// Returns `(data1, data2)` for this port's device — `Gamepad` is handled by the caller
    /// (`crate::Bus` keeps its own pre-existing `joypad` field/logic untouched), so this only
    /// covers the three peripherals added here.
    pub(crate) fn clock(&mut self, iobit: bool, visible_height: u16) -> (u8, u8) {
        match self.device {
            PortDevice::Gamepad => (0, 0),
            PortDevice::Mouse => (self.mouse.clock(), 0),
            PortDevice::SuperScope => (self.super_scope.clock(visible_height), 0),
            PortDevice::Multitap => self.multitap.clock(iobit),
        }
    }

    pub(crate) fn save_state(self, s: &mut SaveWriter) {
        s.write_u8(self.device.to_tag());
        self.mouse.save_state(s);
        self.super_scope.save_state(s);
        self.multitap.save_state(s);
    }

    pub(crate) fn load_state(s: &mut SaveReader) -> Result<Self, SaveStateError> {
        let device = PortDevice::from_tag(s.read_u8()?);
        let mouse = MouseState::load_state(s)?;
        let super_scope = SuperScopeState::load_state(s)?;
        let multitap = MultitapState::load_state(s)?;
        Ok(Self {
            device,
            mouse,
            super_scope,
            multitap,
        })
    }
}

#[cfg(test)]
mod tests {
    use alloc::vec::Vec;

    use super::{MouseState, MultitapState, PortDevice, SuperScopeState, scope};

    /// Shift out `n` bits MSB-first, as a real CPU polling `$4016`/`$4017` `n` times would.
    fn shift_bits<T>(mut clock: impl FnMut(&mut T) -> u8, state: &mut T, n: usize) -> Vec<u8> {
        (0..n).map(|_| clock(state)).collect()
    }

    #[test]
    fn port_device_tag_round_trips() {
        for device in [
            PortDevice::Gamepad,
            PortDevice::Mouse,
            PortDevice::SuperScope,
            PortDevice::Multitap,
        ] {
            assert_eq!(PortDevice::from_tag(device.to_tag()), device);
        }
    }

    #[test]
    fn mouse_signature_and_button_bits() {
        let mut mouse = MouseState::default();
        // No motion, both buttons held, speed stays at its default (0 = slow).
        mouse.set_input(0, 0, true, true);
        mouse.latch(true); // strobe high: arm.
        mouse.latch(false); // strobe low: pack + begin shifting.
        let bits = shift_bits(MouseState::clock, &mut mouse, 32);
        // bits[0..8]  = 0 (padding)
        assert!(bits[0..8].iter().all(|&b| b == 0));
        // bits[8]=right, bits[9]=left — both held.
        assert_eq!(bits[8], 1, "right button bit");
        assert_eq!(bits[9], 1, "left button bit");
        // bits[10..12] = speed (0,0 = slow, the default).
        assert_eq!(bits[10], 0);
        assert_eq!(bits[11], 0);
        // bits[12..16] = device signature 0,0,0,1.
        assert_eq!(&bits[12..16], &[0, 0, 0, 1], "mouse device signature");
        // No motion: direction bits + magnitude all zero.
        assert_eq!(bits[16], 0, "dy direction");
        assert!(bits[17..24].iter().all(|&b| b == 0), "cy magnitude");
        assert_eq!(bits[24], 0, "dx direction");
        assert!(bits[25..32].iter().all(|&b| b == 0), "cx magnitude");
    }

    #[test]
    fn mouse_direction_and_magnitude() {
        let mut mouse = MouseState::default();
        mouse.set_input(-5, 10, false, false);
        mouse.latch(true);
        mouse.latch(false);
        let bits = shift_bits(MouseState::clock, &mut mouse, 32);
        assert_eq!(
            bits[16], 0,
            "moving down (positive dy) clears the dy-direction bit"
        );
        let cy = bits[17..24]
            .iter()
            .fold(0u32, |acc, &b| (acc << 1) | u32::from(b));
        assert_eq!(cy, 10, "cy magnitude, no speed multiplier at default speed");
        assert_eq!(
            bits[24], 1,
            "moving left (negative dx) sets the dx-direction bit"
        );
        let cx = bits[25..32]
            .iter()
            .fold(0u32, |acc, &b| (acc << 1) | u32::from(b));
        assert_eq!(cx, 5, "cx magnitude");
    }

    #[test]
    fn mouse_magnitude_clamps_to_127() {
        let mut mouse = MouseState::default();
        mouse.set_input(500, 0, false, false);
        mouse.latch(true);
        mouse.latch(false);
        let bits = shift_bits(MouseState::clock, &mut mouse, 32);
        let cx = bits[25..32]
            .iter()
            .fold(0u32, |acc, &b| (acc << 1) | u32::from(b));
        assert_eq!(
            cx, 127,
            "a huge host delta clamps to the 7-bit magnitude's max"
        );
    }

    #[test]
    fn mouse_speed_cycles_while_strobe_held_high() {
        let mut mouse = MouseState::default();
        mouse.latch(true);
        // Real hardware: every read taken WHILE strobed high cycles the speed setting instead of
        // shifting real data (ares' `latched==1` branch) — three reads should cycle 0->1->2->0.
        assert_eq!(mouse.clock(), 0);
        assert_eq!(mouse.speed, 1);
        assert_eq!(mouse.clock(), 0);
        assert_eq!(mouse.speed, 2);
        assert_eq!(mouse.clock(), 0);
        assert_eq!(mouse.speed, 0);
    }

    #[test]
    fn mouse_free_runs_high_past_32_bits() {
        let mut mouse = MouseState::default();
        mouse.set_input(0, 0, false, false);
        mouse.latch(true);
        mouse.latch(false);
        let bits = shift_bits(MouseState::clock, &mut mouse, 40);
        assert!(
            bits[32..].iter().all(|&b| b == 1),
            "past bit 32, reads free-run high"
        );
    }

    #[test]
    fn superscope_trigger_cursor_and_offscreen_bits() {
        let mut scope = SuperScopeState::default();
        scope.set_input(100, 100, scope::TRIGGER | scope::CURSOR);
        scope.latch(true);
        scope.latch(false);
        let bits = shift_bits(|s: &mut SuperScopeState| s.clock(224), &mut scope, 8);
        assert_eq!(
            bits[0], 1,
            "trigger fires (aimed on-screen, not turbo-locked yet)"
        );
        assert_eq!(bits[1], 1, "cursor is level-sensitive, reads live");
        assert_eq!(
            bits[2], 0,
            "turbo edge hasn't toggled (turbo switch not held)"
        );
        assert_eq!(bits[3], 0, "pause hasn't been pressed");
        assert_eq!(bits[6], 0, "aimed on-screen");
    }

    #[test]
    fn superscope_offscreen_suppresses_trigger_and_sets_offscreen_bit() {
        let mut scope = SuperScopeState::default();
        scope.set_input(-16, -16, scope::TRIGGER);
        scope.latch(true);
        scope.latch(false);
        let bits = shift_bits(|s: &mut SuperScopeState| s.clock(224), &mut scope, 8);
        assert_eq!(bits[0], 0, "trigger is suppressed while aimed off-screen");
        assert_eq!(bits[6], 1, "offscreen bit set");
        assert!(
            scope.beam_target(224).is_none(),
            "no beam target while offscreen"
        );
    }

    #[test]
    fn superscope_beam_target_matches_ares_formula() {
        let mut scope = SuperScopeState::default();
        scope.set_input(50, 30, 0);
        // cx+24 dots on scanline cy (ares' `(cx+24)*4` hcounter target, already dot-space here).
        assert_eq!(scope.beam_target(224), Some((30, 74)));
    }

    #[test]
    fn superscope_turbo_makes_trigger_level_sensitive() {
        let mut scope = SuperScopeState::default();
        // Toggle turbo on first, forcing a sample via one throwaway clock.
        scope.set_input(10, 10, scope::TURBO);
        scope.latch(true);
        scope.latch(false);
        scope.clock(224);
        assert!(scope.turbo_edge, "turbo toggled on");

        // Now hold trigger across two full latch cycles without releasing it in between — a
        // plain (non-turbo) trigger would only fire once (edge-sensitive), but with turbo active
        // it should fire every time (level-sensitive).
        scope.set_input(10, 10, scope::TURBO | scope::TRIGGER);
        scope.latch(true);
        scope.latch(false);
        let first = scope.clock(224);
        scope.latch(true);
        scope.latch(false);
        let second = scope.clock(224);
        assert_eq!(first, 1, "trigger fires with turbo active");
        assert_eq!(
            second, 1,
            "trigger keeps firing every latch while held, under turbo"
        );
    }

    #[test]
    fn superscope_free_runs_high_past_8_bits() {
        let mut scope = SuperScopeState::default();
        scope.set_input(10, 10, 0);
        scope.latch(true);
        scope.latch(false);
        let bits = shift_bits(|s: &mut SuperScopeState| s.clock(224), &mut scope, 12);
        assert!(
            bits[8..].iter().all(|&b| b == 1),
            "past bit 8, reads free-run high"
        );
    }

    #[test]
    fn multitap_device_detection_signature_while_latched() {
        let mut tap = MultitapState::default();
        tap.latch(true);
        assert_eq!(
            tap.clock(true),
            (0, 1),
            "multitap ID: data1=0, data2=1, while latched"
        );
        assert_eq!(
            tap.clock(false),
            (0, 1),
            "signature holds regardless of iobit"
        );
    }

    #[test]
    fn multitap_iobit_selects_the_addressed_pair() {
        let mut tap = MultitapState::default();
        tap.set_pad(0, 0x8000); // pad 1: only the MSB (first-shifted-out bit) set.
        tap.set_pad(1, 0x0000); // pad 2: nothing set.
        tap.set_pad(2, 0x0000); // pad 3: nothing set.
        tap.set_pad(3, 0x8000); // pad 4: only the MSB set.
        tap.latch(false);

        // iobit=true selects pads [0,1]: data1 should read pad1's set MSB, data2 pad2's clear one.
        assert_eq!(tap.clock(true), (1, 0));
        // iobit=false selects pads [2,3]: pad 3's untouched MSB is still set (independent
        // counters — reading the [0,1] pair above must not have advanced pads 2/3 at all).
        assert_eq!(tap.clock(false), (0, 1));
    }

    #[test]
    fn multitap_pad_get_set_round_trips() {
        let mut tap = MultitapState::default();
        tap.set_pad(2, 0x1234);
        assert_eq!(tap.pad(2), 0x1234);
        assert_eq!(tap.pad(0), 0, "untouched sub-pads stay zero");
    }
}
