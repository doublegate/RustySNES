//! Sony **S-DSP** (CXD1222Q) — the 8-voice wavetable sound generator.
//!
//! Clean-room port of ares' S-DSP (`sfc/dsp`, ISC): the 32-step voice/echo/misc micro-sequence is
//! reproduced verbatim and **cycle-stepped** — [`Dsp::tick`] executes one of the 32 interleaved
//! phases (ares `DSP::main`), so BRR decode, 4-point Gaussian interpolation, ADSR/GAIN envelopes
//! (with the exact counter-rate table), the noise LFSR, pitch + PMON, KON/KOFF/ENDX edge timing,
//! and the 8-tap echo FIR + feedback all match ares cycle-for-cycle, with sub-sample resolution for
//! the OUTX/ENVX/ENDX register writes. One stereo 16-bit sample is produced every 32 ticks (the
//! SNES 32 kHz rate; the DAC latches at phase 27). [`Dsp::run_sample`] is the batched `32 × tick`
//! wrapper. All integer math; the only float is the one-time Gaussian table build at construction.
//!
//! The S-DSP is a hardware register machine: bytes are reinterpreted as signed/unsigned, BRR
//! decode and the FIR/envelope math rely on deliberate wrapping shifts, and the Gaussian table
//! build is the lone float→int conversion. The cast lints below are blanket-allowed for this
//! module because every cast here is an intentional, hardware-bounded reinterpretation.
#![allow(
    clippy::cast_possible_truncation,
    clippy::cast_possible_wrap,
    clippy::cast_sign_loss,
    clippy::cast_lossless,
    clippy::cast_precision_loss,
    clippy::similar_names,
    clippy::struct_excessive_bools,
    clippy::used_underscore_binding,
    clippy::large_stack_arrays,
    clippy::missing_const_for_fn
)]

extern crate alloc;
use alloc::boxed::Box;

/// Number of bytes of audio RAM the DSP addresses.
pub const ARAM_SIZE: usize = 0x1_0000;

const fn sclamp16(v: i32) -> i32 {
    if v < -0x8000 {
        -0x8000
    } else if v > 0x7FFF {
        0x7FFF
    } else {
        v
    }
}

/// Per-voice envelope phase.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
enum EnvMode {
    #[default]
    Release,
    Attack,
    Decay,
    Sustain,
}

#[derive(Debug, Clone, Copy, Default)]
struct Voice {
    index: u8, // register base: voice n at n<<4
    volume: [i8; 2],
    pitch: u16, // 14-bit
    source: u8,
    adsr0: u8,
    adsr1: u8,
    gain: u8,
    envx: u8,
    keyon: bool,
    keyoff: bool,
    modulate: bool,
    noise: bool,
    echo: bool,

    buffer: [i16; 12],
    buffer_offset: u8,
    gaussian_offset: u16,
    brr_address: u16,
    brr_offset: u8, // 1..=8
    keyon_delay: u8,
    envelope_mode: EnvMode,
    envelope: u16, // 0..=2047

    env_internal: i32, // GAIN mode-7 quirk latch
    keylatch: bool,
    _keyon: bool,
    _keyoff: bool,
    _modulate: bool,
    _noise: bool,
    _echo: bool,
    _end: bool,
    _looped: bool,
}

#[derive(Debug, Clone, Copy, Default)]
struct MainVol {
    reset: bool,
    mute: bool,
    volume: [i8; 2],
    output: [i32; 2],
}

#[derive(Debug, Clone, Copy)]
struct Echo {
    feedback: i8,
    volume: [i8; 2],
    fir: [i8; 8],
    history: [[i16; 8]; 2],
    page: u8,
    delay: u8,
    readonly: bool,
    input: [i32; 2],
    output: [i32; 2],

    _page: u8,
    _readonly: bool,
    address: u16,
    offset: u16,
    length: u16,
    history_offset: u8, // 3-bit
}

impl Default for Echo {
    fn default() -> Self {
        Self {
            feedback: 0,
            volume: [0; 2],
            fir: [0; 8],
            history: [[0; 8]; 2],
            page: 0,
            delay: 0,
            readonly: true,
            input: [0; 2],
            output: [0; 2],
            _page: 0,
            _readonly: true,
            address: 0,
            offset: 0,
            length: 0,
            history_offset: 0,
        }
    }
}

#[derive(Debug, Clone, Copy)]
struct Noise {
    frequency: u8,
    lfsr: u16,
}

impl Default for Noise {
    fn default() -> Self {
        Self {
            frequency: 0,
            lfsr: 0x4000,
        }
    }
}

#[derive(Debug, Clone, Copy, Default)]
struct Brr {
    bank: u8,
    _bank: u8,
    source: u8,
    address: u16,
    next_address: u16,
    header: u8,
    byte: u8,
}

#[derive(Debug, Clone, Copy, Default)]
struct Latch {
    adsr0: u8,
    envx: u8,
    outx: u8,
    pitch: u16, // 15-bit
    output: i16,
}

#[derive(Debug, Clone, Copy)]
struct Clock {
    counter: u16, // 15-bit, counts down through 0x7800
    sample: bool,
}

impl Default for Clock {
    fn default() -> Self {
        Self {
            counter: 0,
            sample: true,
        }
    }
}

/// Number of samples per counter event (`CounterRate[0]` never triggers).
const COUNTER_RATE: [u16; 32] = [
    0, 2048, 1536, 1280, 1024, 768, 640, 512, 384, 320, 256, 192, 160, 128, 96, 80, 64, 48, 40, 32,
    24, 20, 16, 12, 10, 8, 6, 5, 4, 3, 2, 1,
];

/// Counter offset from zero (counters are not aligned at zero for all rates).
const COUNTER_OFFSET: [u16; 32] = [
    0, 0, 1040, 536, 0, 1040, 536, 0, 1040, 536, 0, 1040, 536, 0, 1040, 536, 0, 1040, 536, 0, 1040,
    536, 0, 1040, 536, 0, 1040, 536, 0, 1040, 0, 0,
];

/// The S-DSP. Owns its 128 mirror registers; the parent supplies the 64 KiB ARAM by reference.
pub struct Dsp {
    registers: [u8; 128],
    voice: [Voice; 8],
    mainvol: MainVol,
    echo: Echo,
    noise: Noise,
    brr: Brr,
    latch: Latch,
    clock: Clock,
    gaussian: Box<[i16; 512]>,
    /// Position in the 32-step DSP micro-sequence (ares `DSP::main` phase, 0..=31). Each
    /// [`Self::tick`] executes one phase and advances this; the output sample latches at phase 27
    /// (`echo27`). A full 32-tick cycle is one 32 kHz output sample (64 SMP base clocks).
    phase: u8,
    /// Most-recent stereo output sample, latched once per 32-tick frame (at phase 27).
    last_sample: (i16, i16),
}

impl core::fmt::Debug for Dsp {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("Dsp")
            .field("last_sample", &self.last_sample)
            .finish_non_exhaustive()
    }
}

impl Default for Dsp {
    fn default() -> Self {
        Self::new()
    }
}

impl Dsp {
    /// Construct at power-on (builds the Gaussian interpolation table once).
    #[must_use]
    pub fn new() -> Self {
        let mut dsp = Self {
            registers: [0; 128],
            voice: [Voice::default(); 8],
            mainvol: MainVol {
                reset: true,
                mute: true,
                ..MainVol::default()
            },
            echo: Echo::default(),
            noise: Noise::default(),
            brr: Brr::default(),
            latch: Latch::default(),
            clock: Clock::default(),
            gaussian: Box::new([0; 512]),
            phase: 0,
            last_sample: (0, 0),
        };
        for (n, v) in dsp.voice.iter_mut().enumerate() {
            v.index = (n as u8) << 4;
        }
        dsp.build_gaussian();
        dsp
    }

    /// Read a DSP register (`0x80..=0xFF` mirror `0x00..=0x7F`).
    #[must_use]
    pub fn read(&self, address: u8) -> u8 {
        self.registers[(address & 0x7F) as usize]
    }

    /// The last stereo sample the DSP emitted (left, right), 16-bit signed.
    #[must_use]
    pub const fn last_sample(&self) -> (i16, i16) {
        self.last_sample
    }

    /// Whether main output is muted (FLG.6).
    #[must_use]
    pub const fn muted(&self) -> bool {
        self.mainvol.mute
    }

    /// Write a DSP register, decoding it into the live voice/echo/global state (ares `write`).
    #[allow(clippy::too_many_lines)]
    pub fn write(&mut self, address: u8, data: u8) {
        if address & 0x80 != 0 {
            return; // high half is a read-only mirror
        }
        self.registers[address as usize] = data;

        match address {
            0x0C => self.mainvol.volume[0] = data as i8,
            0x1C => self.mainvol.volume[1] = data as i8,
            0x2C => self.echo.volume[0] = data as i8,
            0x3C => self.echo.volume[1] = data as i8,
            0x4C => {
                for (n, v) in self.voice.iter_mut().enumerate() {
                    v.keyon = data & (1 << n) != 0;
                    v.keylatch = data & (1 << n) != 0;
                }
            }
            0x5C => {
                for (n, v) in self.voice.iter_mut().enumerate() {
                    v.keyoff = data & (1 << n) != 0;
                }
            }
            0x6C => {
                self.noise.frequency = data & 0x1F;
                self.echo.readonly = data & 0x20 != 0;
                self.mainvol.mute = data & 0x40 != 0;
                self.mainvol.reset = data & 0x80 != 0;
            }
            0x7C => {
                for v in &mut self.voice {
                    v._end = false;
                }
                self.registers[0x7C] = 0;
            }
            0x0D => self.echo.feedback = data as i8,
            0x2D => {
                for (n, v) in self.voice.iter_mut().enumerate() {
                    v.modulate = data & (1 << n) != 0;
                }
                self.voice[0].modulate = false;
            }
            0x3D => {
                for (n, v) in self.voice.iter_mut().enumerate() {
                    v.noise = data & (1 << n) != 0;
                }
            }
            0x4D => {
                for (n, v) in self.voice.iter_mut().enumerate() {
                    v.echo = data & (1 << n) != 0;
                }
            }
            0x5D => self.brr.bank = data,
            0x6D => self.echo.page = data,
            0x7D => self.echo.delay = data & 0x0F,
            _ => {}
        }

        let n = ((address >> 4) & 0x07) as usize;
        match address & 0x0F {
            0x00 => self.voice[n].volume[0] = data as i8,
            0x01 => self.voice[n].volume[1] = data as i8,
            0x02 => self.voice[n].pitch = (self.voice[n].pitch & 0xFF00) | u16::from(data),
            0x03 => {
                self.voice[n].pitch =
                    (self.voice[n].pitch & 0x00FF) | (u16::from(data & 0x3F) << 8);
            }
            0x04 => self.voice[n].source = data,
            0x05 => self.voice[n].adsr0 = data,
            0x06 => self.voice[n].adsr1 = data,
            0x07 => self.voice[n].gain = data,
            0x08 => self.latch.envx = data,
            0x09 => self.latch.outx = data,
            0x0F => self.echo.fir[n] = data as i8,
            _ => {}
        }
    }

    fn build_gaussian(&mut self) {
        use core::f64::consts::PI;

        let mut table = [0.0_f64; 512];
        for (n, slot) in table.iter_mut().enumerate() {
            let k = 0.5 + n as f64;
            let s = libm_sin(PI * k * 1.280 / 1024.0);
            let t = (libm_cos(PI * k * 2.000 / 1023.0) - 1.0) * 0.50;
            let u = (libm_cos(PI * k * 4.000 / 1023.0) - 1.0) * 0.08;
            *slot = s * (t + u + 1.0) / k;
        }
        // table[511 - n] = r assignment above is folded by reversing index usage below.
        let mut rev = [0.0_f64; 512];
        for n in 0..512 {
            rev[511 - n] = table[n];
        }
        for phase in 0..128 {
            let sum = rev[phase] + rev[phase + 256] + rev[511 - phase] + rev[255 - phase];
            let scale = 2048.0 / sum;
            self.gaussian[phase] = (rev[phase] * scale + 0.5) as i16;
            self.gaussian[phase + 256] = (rev[phase + 256] * scale + 0.5) as i16;
            self.gaussian[511 - phase] = (rev[511 - phase] * scale + 0.5) as i16;
            self.gaussian[255 - phase] = (rev[255 - phase] * scale + 0.5) as i16;
        }
    }

    fn counter_tick(&mut self) {
        if self.clock.counter == 0 {
            self.clock.counter = 0x7800; // 30720
        }
        self.clock.counter -= 1;
    }

    fn counter_poll(&self, rate: u8) -> bool {
        if rate == 0 {
            return false;
        }
        let r = rate as usize;
        (u32::from(self.clock.counter) + u32::from(COUNTER_OFFSET[r])) % u32::from(COUNTER_RATE[r])
            == 0
    }

    /// Advance the DSP by one full 32 kHz output frame (32 micro-ticks), reading/writing `aram`.
    /// Updates [`Self::last_sample`]. This is exactly 32 [`Self::tick`] calls — the batched
    /// convenience wrapper for callers that don't need sub-sample granularity (unit tests,
    /// `.spc` rendering). The cycle-exact integration in `lib.rs` instead calls [`Self::tick`]
    /// once per 2 SMP base clocks so the SMP sees cycle-correct DSP register state mid-instruction.
    pub fn run_sample(&mut self, aram: &mut [u8; ARAM_SIZE]) {
        for _ in 0..32 {
            self.tick(aram);
        }
    }

    /// Execute one of the 32 DSP micro-sequence phases (ares `DSP::main`), advancing the internal
    /// phase counter. The S-DSP is a hard pipeline: the nine per-voice steps (`voice1..voice9`),
    /// the echo path (`echo22..echo30`), and the housekeeping latches (`misc27..misc30`) are
    /// **interleaved across the 8 voices** over a 32-tick schedule, not run per-voice-at-once. This
    /// is what gives sub-sample timing to the OUTX/ENVX/ENDX register writes and the BRR/envelope
    /// latches that blargg's `spc_dsp6` / `spc_mem_access_times` use the DSP as a reference for. The
    /// stereo output sample latches at phase 27 (`echo27`). Reproduced verbatim from ares
    /// (`sfc/dsp/dsp.cpp::main`, ISC).
    #[allow(clippy::too_many_lines)] // the 32-entry phase table is one flat, verbatim schedule
    pub fn tick(&mut self, aram: &mut [u8; ARAM_SIZE]) {
        match self.phase {
            0 => {
                self.voice5(0);
                self.voice2(1, aram);
            }
            1 => {
                self.voice6(0);
                self.voice3(1, aram);
            }
            2 => {
                self.voice7(0);
                self.voice4(1, aram);
                self.voice1(3);
            }
            3 => {
                self.voice8(0);
                self.voice5(1);
                self.voice2(2, aram);
            }
            4 => {
                self.voice9(0);
                self.voice6(1);
                self.voice3(2, aram);
            }
            5 => {
                self.voice7(1);
                self.voice4(2, aram);
                self.voice1(4);
            }
            6 => {
                self.voice8(1);
                self.voice5(2);
                self.voice2(3, aram);
            }
            7 => {
                self.voice9(1);
                self.voice6(2);
                self.voice3(3, aram);
            }
            8 => {
                self.voice7(2);
                self.voice4(3, aram);
                self.voice1(5);
            }
            9 => {
                self.voice8(2);
                self.voice5(3);
                self.voice2(4, aram);
            }
            10 => {
                self.voice9(2);
                self.voice6(3);
                self.voice3(4, aram);
            }
            11 => {
                self.voice7(3);
                self.voice4(4, aram);
                self.voice1(6);
            }
            12 => {
                self.voice8(3);
                self.voice5(4);
                self.voice2(5, aram);
            }
            13 => {
                self.voice9(3);
                self.voice6(4);
                self.voice3(5, aram);
            }
            14 => {
                self.voice7(4);
                self.voice4(5, aram);
                self.voice1(7);
            }
            15 => {
                self.voice8(4);
                self.voice5(5);
                self.voice2(6, aram);
            }
            16 => {
                self.voice9(4);
                self.voice6(5);
                self.voice3(6, aram);
            }
            17 => {
                self.voice1(0);
                self.voice7(5);
                self.voice4(6, aram);
            }
            18 => {
                self.voice8(5);
                self.voice5(6);
                self.voice2(7, aram);
            }
            19 => {
                self.voice9(5);
                self.voice6(6);
                self.voice3(7, aram);
            }
            20 => {
                self.voice1(1);
                self.voice7(6);
                self.voice4(7, aram);
            }
            21 => {
                self.voice8(6);
                self.voice5(7);
                self.voice2(0, aram);
            }
            22 => {
                self.voice3a(0);
                self.voice9(6);
                self.voice6(7);
                self.echo22(aram);
            }
            23 => {
                self.voice7(7);
                self.echo23(aram);
            }
            24 => {
                self.voice8(7);
                self.echo24();
            }
            25 => {
                self.voice3b(0, aram);
                self.voice9(7);
                self.echo25();
            }
            26 => self.echo26(),
            27 => {
                self.misc27();
                self.echo27();
            }
            28 => {
                self.misc28();
                self.echo28();
            }
            29 => {
                self.misc29();
                self.echo29(aram);
            }
            30 => {
                self.misc30();
                self.voice3c(0);
                self.echo30(aram);
            }
            31 => {
                self.voice4(0, aram);
                self.voice1(2);
            }
            _ => {}
        }
        self.phase = (self.phase + 1) & 31;
    }

    fn misc27(&mut self) {
        for v in &mut self.voice {
            v._modulate = v.modulate;
        }
    }

    fn misc28(&mut self) {
        for v in &mut self.voice {
            v._noise = v.noise;
            v._echo = v.echo;
        }
        self.brr._bank = self.brr.bank;
    }

    fn misc29(&mut self) {
        self.clock.sample = !self.clock.sample;
        if self.clock.sample {
            for v in &mut self.voice {
                v.keylatch &= !v._keyon;
            }
        }
    }

    fn misc30(&mut self) {
        if self.clock.sample {
            for v in &mut self.voice {
                v._keyon = v.keylatch;
                v._keyoff = v.keyoff;
            }
        }
        self.counter_tick();
        if self.counter_poll(self.noise.frequency) {
            let feedback = (i32::from(self.noise.lfsr) << 13) ^ (i32::from(self.noise.lfsr) << 14);
            self.noise.lfsr = ((feedback & 0x4000) as u16) | (self.noise.lfsr >> 1);
        }
    }

    // --- Per-voice pipeline steps (ares `sfc/dsp/voice.cpp`). Each is one slot of the 32-tick
    // schedule; `tick` dispatches the interleaved set. `vi` is the voice index 0..=7.

    /// voice1: latch this voice's BRR sample-pointer base (from the previous voice's source).
    fn voice1(&mut self, vi: usize) {
        self.brr.address =
            (u16::from(self.brr._bank) << 8).wrapping_add(u16::from(self.brr.source) << 2);
        self.brr.source = self.voice[vi].source;
    }

    /// voice2: read the sample-directory entry (start / loop address) and latch ADSR0 + pitch low.
    fn voice2(&mut self, vi: usize, aram: &[u8; ARAM_SIZE]) {
        let mut addr = self.brr.address;
        if self.voice[vi].keyon_delay == 0 {
            addr = addr.wrapping_add(2);
        }
        let lo = aram[addr as usize];
        let hi = aram[addr.wrapping_add(1) as usize];
        self.brr.next_address = u16::from(lo) | (u16::from(hi) << 8);
        self.latch.adsr0 = self.voice[vi].adsr0;
        self.latch.pitch = self.voice[vi].pitch & 0xFF;
    }

    /// voice3 (combined a+b+c) — used for voices 1..=7 at a single phase. Voice 0 is split across
    /// phases 22/25/30 (it wraps the sample boundary), so it calls 3a/3b/3c individually.
    fn voice3(&mut self, vi: usize, aram: &[u8; ARAM_SIZE]) {
        self.voice3a(vi);
        self.voice3b(vi, aram);
        self.voice3c(vi);
    }

    /// voice3a: assemble the full 14-bit pitch (high byte merged onto the low byte from `voice2`).
    fn voice3a(&mut self, vi: usize) {
        self.latch.pitch |= self.voice[vi].pitch & !0xFF;
    }

    /// voice3b: fetch the current BRR byte and the block header from ARAM.
    fn voice3b(&mut self, vi: usize, aram: &[u8; ARAM_SIZE]) {
        self.brr.byte = aram[self.voice[vi]
            .brr_address
            .wrapping_add(u16::from(self.voice[vi].brr_offset))
            as usize];
        self.brr.header = aram[self.voice[vi].brr_address as usize];
    }

    /// voice3c: pitch modulation, KON setup, interpolation, envelope, and the output latch.
    fn voice3c(&mut self, vi: usize) {
        if self.voice[vi]._modulate {
            let factor = ((i32::from(self.latch.output) >> 5) * i32::from(self.latch.pitch)) >> 10;
            self.latch.pitch = (i32::from(self.latch.pitch) + factor) as u16;
        }

        if self.voice[vi].keyon_delay != 0 {
            if self.voice[vi].keyon_delay == 5 {
                self.voice[vi].brr_address = self.brr.next_address;
                self.voice[vi].brr_offset = 1;
                self.voice[vi].buffer_offset = 0;
                self.brr.header = 0;
            }
            self.voice[vi].envelope = 0;
            self.voice[vi].env_internal = 0;
            self.voice[vi].gaussian_offset = 0;
            self.voice[vi].keyon_delay -= 1;
            if self.voice[vi].keyon_delay & 3 != 0 {
                self.voice[vi].gaussian_offset = 0x4000;
            }
            self.latch.pitch = 0;
        }

        let output = if self.voice[vi]._noise {
            i32::from((self.noise.lfsr << 1) as i16)
        } else {
            self.gaussian_interpolate(vi)
        };

        self.latch.output = (((output * i32::from(self.voice[vi].envelope)) >> 11) & !1) as i16;
        self.voice[vi].envx = (self.voice[vi].envelope >> 4) as u8;

        if self.mainvol.reset || (self.brr.header & 0x03) == 1 {
            self.voice[vi].envelope_mode = EnvMode::Release;
            self.voice[vi].envelope = 0;
        }

        if self.clock.sample {
            if self.voice[vi]._keyoff {
                self.voice[vi].envelope_mode = EnvMode::Release;
            }
            if self.voice[vi]._keyon {
                self.voice[vi].keyon_delay = 5;
                self.voice[vi].envelope_mode = EnvMode::Attack;
            }
        }

        if self.voice[vi].keyon_delay == 0 {
            self.envelope_run(vi);
        }
    }

    /// voice4: BRR decode (when due), pitch advance, and the left-channel output mix.
    fn voice4(&mut self, vi: usize, aram: &[u8; ARAM_SIZE]) {
        self.voice[vi]._looped = false;
        if self.voice[vi].gaussian_offset >= 0x4000 {
            self.brr_decode(vi, aram);
            self.voice[vi].brr_offset += 2;
            if self.voice[vi].brr_offset >= 9 {
                self.voice[vi].brr_address = self.voice[vi].brr_address.wrapping_add(9);
                if self.brr.header & 0x01 != 0 {
                    self.voice[vi].brr_address = self.brr.next_address;
                    self.voice[vi]._looped = true;
                }
                self.voice[vi].brr_offset = 1;
            }
        }
        self.voice[vi].gaussian_offset =
            (self.voice[vi].gaussian_offset & 0x3FFF).wrapping_add(self.latch.pitch);
        if self.voice[vi].gaussian_offset > 0x7FFF {
            self.voice[vi].gaussian_offset = 0x7FFF;
        }
        self.voice_output(vi, 0);
    }

    /// voice5: right-channel output mix + the ENDX edge (set on BRR loop, cleared on fresh KON).
    fn voice5(&mut self, vi: usize) {
        self.voice_output(vi, 1);
        self.voice[vi]._end |= self.voice[vi]._looped;
        if self.voice[vi].keyon_delay == 5 {
            self.voice[vi]._end = false;
        }
    }

    /// voice6: latch OUTX (high byte of the voice output) for the deferred writeback. Takes the
    /// voice index for schedule symmetry with ares, though OUTX is sourced from the shared latch.
    fn voice6(&mut self, _vi: usize) {
        self.latch.outx = (i32::from(self.latch.output) >> 8) as u8;
    }

    /// voice7: refresh the ENDX register (0x7C) from every voice's end flag, latch ENVX.
    fn voice7(&mut self, vi: usize) {
        let mut endx = 0u8;
        for (n, v) in self.voice.iter().enumerate() {
            if v._end {
                endx |= 1 << n;
            }
        }
        self.registers[0x7C] = endx;
        self.latch.envx = self.voice[vi].envx;
    }

    /// voice8: write the latched OUTX to this voice's `$x9` register.
    fn voice8(&mut self, vi: usize) {
        self.registers[self.voice[vi].index as usize | 0x09] = self.latch.outx;
    }

    /// voice9: write the latched ENVX to this voice's `$x8` register.
    fn voice9(&mut self, vi: usize) {
        self.registers[self.voice[vi].index as usize | 0x08] = self.latch.envx;
    }

    fn voice_output(&mut self, vi: usize, channel: usize) {
        let amp = (i32::from(self.latch.output) * i32::from(self.voice[vi].volume[channel])) >> 7;
        self.mainvol.output[channel] = sclamp16(self.mainvol.output[channel] + amp);
        if self.voice[vi]._echo {
            self.echo.output[channel] = sclamp16(self.echo.output[channel] + amp);
        }
    }

    fn gaussian_interpolate(&self, vi: usize) -> i32 {
        let v = &self.voice[vi];
        let off8 = (v.gaussian_offset >> 4) as usize & 0xFF;
        let fwd = 255 - off8;
        let rev = off8;
        let mut offset = (usize::from(v.buffer_offset) + (v.gaussian_offset >> 12) as usize) % 12;

        let mut output: i32;
        output = (i32::from(self.gaussian[fwd]) * i32::from(v.buffer[offset])) >> 11;
        offset += 1;
        if offset >= 12 {
            offset = 0;
        }
        output += (i32::from(self.gaussian[fwd + 256]) * i32::from(v.buffer[offset])) >> 11;
        offset += 1;
        if offset >= 12 {
            offset = 0;
        }
        output += (i32::from(self.gaussian[rev + 256]) * i32::from(v.buffer[offset])) >> 11;
        offset += 1;
        if offset >= 12 {
            offset = 0;
        }
        output = i32::from(output as i16);
        output += (i32::from(self.gaussian[rev]) * i32::from(v.buffer[offset])) >> 11;
        sclamp16(output) & !1
    }

    fn brr_decode(&mut self, vi: usize, aram: &[u8; ARAM_SIZE]) {
        let next = self.voice[vi]
            .brr_address
            .wrapping_add(u16::from(self.voice[vi].brr_offset))
            .wrapping_add(1);
        let mut nybbles = (i32::from(self.brr.byte) << 8) | i32::from(aram[next as usize]);
        let filter = (self.brr.header >> 2) & 0x03;
        let scale = (self.brr.header >> 4) & 0x0F;

        for _ in 0..4 {
            let mut s = i32::from((nybbles as i16) >> 12); // sign-extended top nybble
            nybbles <<= 4;

            if scale <= 12 {
                s = (s << scale) >> 1;
            } else {
                s &= !0x7FF;
            }

            let bo = self.voice[vi].buffer_offset;
            let p1 = i32::from(self.voice[vi].buffer[if bo == 0 { 11 } else { bo - 1 } as usize]);
            let p2_i = if bo == 0 {
                10
            } else if bo == 1 {
                11
            } else {
                bo - 2
            } as usize;
            let p2 = i32::from(self.voice[vi].buffer[p2_i]) >> 1;

            match filter {
                0 => {}
                1 => {
                    s += p1 >> 1;
                    s += (-p1) >> 5;
                }
                2 => {
                    s += p1;
                    s -= p2;
                    s += p2 >> 4;
                    s += (p1 * -3) >> 6;
                }
                _ => {
                    s += p1;
                    s -= p2;
                    s += (p1 * -13) >> 7;
                    s += (p2 * 3) >> 4;
                }
            }

            s = sclamp16(s);
            let stored = (s << 1) as i16;
            self.voice[vi].buffer[bo as usize] = stored;
            self.voice[vi].buffer_offset += 1;
            if self.voice[vi].buffer_offset >= 12 {
                self.voice[vi].buffer_offset = 0;
            }
        }
    }

    fn envelope_run(&mut self, vi: usize) {
        let mut envelope = i32::from(self.voice[vi].envelope);

        if self.voice[vi].envelope_mode == EnvMode::Release {
            envelope -= 0x8;
            if envelope < 0 {
                envelope = 0;
            }
            self.voice[vi].envelope = envelope as u16;
            return;
        }

        let rate;
        let mut envelope_data = i32::from(self.voice[vi].adsr1);
        if self.latch.adsr0 & 0x80 != 0 {
            // ADSR
            if self.voice[vi].envelope_mode as u8 >= EnvMode::Decay as u8 {
                envelope -= 1;
                envelope -= envelope >> 8;
                rate = i32::from(self.voice[vi].adsr1 & 0x1F);
                let rate = if self.voice[vi].envelope_mode == EnvMode::Decay {
                    i32::from((self.latch.adsr0 >> 4) & 0x07) * 2 + 16
                } else {
                    rate
                };
                self.envelope_finish(vi, envelope, envelope_data, rate);
                return;
            }
            // attack
            let rate = i32::from(self.latch.adsr0 & 0x0F) * 2 + 1;
            envelope += if rate < 31 { 0x20 } else { 0x400 };
            self.envelope_finish(vi, envelope, envelope_data, rate);
            return;
        }
        // GAIN
        envelope_data = i32::from(self.voice[vi].gain);
        let mode = envelope_data >> 5;
        if mode < 4 {
            envelope = envelope_data << 4;
            rate = 31;
        } else {
            rate = envelope_data & 0x1F;
            if mode == 4 {
                envelope -= 0x20;
            } else if mode < 6 {
                envelope -= 1;
                envelope -= envelope >> 8;
            } else {
                envelope += 0x20;
                // GAIN mode 7 (bent/two-slope linear increase): the slope halves once the *internal*
                // envelope latch crosses 0x600. The comparison is **unsigned** (blargg `SPC_DSP`
                // `(unsigned) v->hidden_env >= 0x600`, ares `(u32)v._envelope >= 0x600`): when the
                // voice has just come out of a GAIN decrease mode (4/5) that underflowed
                // `env_internal` below zero, the unsigned reinterpretation makes that negative latch
                // satisfy the threshold, so the reduced slope applies. A signed compare here misses
                // that quirk and over-increments — the `Envelope/gain $E0 threshold` divergence in
                // blargg `spc_dsp6`.
                if mode > 6 && (self.voice[vi].env_internal as u32) >= 0x600 {
                    envelope += 0x8 - 0x20;
                }
            }
        }
        self.envelope_finish(vi, envelope, envelope_data, rate);
    }

    fn envelope_finish(&mut self, vi: usize, mut envelope: i32, envelope_data: i32, rate: i32) {
        if (envelope >> 8) == (envelope_data >> 5) && self.voice[vi].envelope_mode == EnvMode::Decay
        {
            self.voice[vi].envelope_mode = EnvMode::Sustain;
        }
        self.voice[vi].env_internal = envelope;

        if (envelope as u32) > 0x7FF {
            envelope = if envelope < 0 { 0 } else { 0x7FF };
            if self.voice[vi].envelope_mode == EnvMode::Attack {
                self.voice[vi].envelope_mode = EnvMode::Decay;
            }
        }

        if self.counter_poll(rate.clamp(0, 31) as u8) {
            self.voice[vi].envelope = envelope as u16;
        }
    }

    /// echo22: history advance, read left echo from ARAM, FIR tap 0.
    fn echo22(&mut self, aram: &[u8; ARAM_SIZE]) {
        self.echo.history_offset = (self.echo.history_offset + 1) & 7;
        self.echo.address = (u16::from(self.echo._page) << 8).wrapping_add(self.echo.offset);
        self.echo_read(0, aram);
        self.echo.input[0] = self.calc_fir(0, 0);
        self.echo.input[1] = self.calc_fir(1, 0);
    }

    /// echo23: FIR taps 1,2 + read right echo from ARAM.
    fn echo23(&mut self, aram: &[u8; ARAM_SIZE]) {
        self.echo.input[0] += self.calc_fir(0, 1) + self.calc_fir(0, 2);
        self.echo.input[1] += self.calc_fir(1, 1) + self.calc_fir(1, 2);
        self.echo_read(1, aram);
    }

    /// echo24: FIR taps 3,4,5.
    fn echo24(&mut self) {
        self.echo.input[0] += self.calc_fir(0, 3) + self.calc_fir(0, 4) + self.calc_fir(0, 5);
        self.echo.input[1] += self.calc_fir(1, 3) + self.calc_fir(1, 4) + self.calc_fir(1, 5);
    }

    /// echo25: FIR taps 6,7 + clamp the FIR result.
    fn echo25(&mut self) {
        for ch in 0..2 {
            let mut l = self.echo.input[ch] + self.calc_fir(ch, 6);
            l = i32::from(l as i16);
            l += i32::from(self.calc_fir(ch, 7) as i16);
            self.echo.input[ch] = sclamp16(l) & !1;
        }
    }

    /// echo26: save the left main+echo mix, apply echo feedback.
    fn echo26(&mut self) {
        self.mainvol.output[0] = self.echo_output(0);
        for ch in 0..2 {
            let fb = i32::from(((self.echo.input[ch] * i32::from(self.echo.feedback)) >> 7) as i16);
            self.echo.output[ch] = sclamp16(self.echo.output[ch] + fb) & !1;
        }
    }

    /// echo27: latch the stereo output sample to the DAC ([`Self::last_sample`]).
    fn echo27(&mut self) {
        let mut outl = self.mainvol.output[0];
        let mut outr = self.echo_output(1);
        self.mainvol.output[0] = 0;
        self.mainvol.output[1] = 0;
        if self.mainvol.mute {
            outl = 0;
            outr = 0;
        }
        self.last_sample = (outl as i16, outr as i16);
    }

    /// echo28: latch the echo read-only flag.
    fn echo28(&mut self) {
        self.echo._readonly = self.echo.readonly;
    }

    /// echo29: advance the echo buffer offset (and the per-loop length latch), write left echo.
    fn echo29(&mut self, aram: &mut [u8; ARAM_SIZE]) {
        self.echo._page = self.echo.page;
        if self.echo.offset == 0 {
            self.echo.length = u16::from(self.echo.delay) << 11;
        }
        self.echo.offset += 4;
        if self.echo.offset >= self.echo.length {
            self.echo.offset = 0;
        }
        self.echo_write(0, aram);
        self.echo._readonly = self.echo.readonly;
    }

    /// echo30: write right echo.
    fn echo30(&mut self, aram: &mut [u8; ARAM_SIZE]) {
        self.echo_write(1, aram);
    }

    fn calc_fir(&self, channel: usize, index: i32) -> i32 {
        let idx = ((i32::from(self.echo.history_offset) + index + 1) & 7) as usize;
        let sample = i32::from(self.echo.history[channel][idx]);
        (sample * i32::from(self.echo.fir[index as usize])) >> 6
    }

    fn echo_output(&self, channel: usize) -> i32 {
        let main = i32::from(
            ((self.mainvol.output[channel] * i32::from(self.mainvol.volume[channel])) >> 7) as i16,
        );
        let echo = i32::from(
            ((self.echo.input[channel] * i32::from(self.echo.volume[channel])) >> 7) as i16,
        );
        sclamp16(main + echo)
    }

    fn echo_read(&mut self, channel: usize, aram: &[u8; ARAM_SIZE]) {
        let address = self.echo.address.wrapping_add((channel as u16) * 2);
        let lo = aram[address as usize];
        let hi = aram[address.wrapping_add(1) as usize];
        let s = i32::from(((u16::from(hi) << 8) | u16::from(lo)) as i16);
        self.echo.history[channel][self.echo.history_offset as usize] = (s >> 1) as i16;
    }

    fn echo_write(&mut self, channel: usize, aram: &mut [u8; ARAM_SIZE]) {
        if !self.echo._readonly {
            let address = self.echo.address.wrapping_add((channel as u16) * 2);
            let sample = self.echo.output[channel] as u16;
            aram[address as usize] = sample as u8;
            aram[address.wrapping_add(1) as usize] = (sample >> 8) as u8;
        }
        self.echo.output[channel] = 0;
    }
}

// The crate is `#![no_std]` even on the `std`-feature build, so `f64::sin/cos` aren't available;
// use the in-crate range-reduced series (deterministic across hosted + bare-metal builds).
fn libm_sin(x: f64) -> f64 {
    crate::nostd_math::sin(x)
}
fn libm_cos(x: f64) -> f64 {
    crate::nostd_math::cos(x)
}

#[cfg(test)]
mod tests {
    use super::*;
    extern crate std;

    fn empty_aram() -> Box<[u8; ARAM_SIZE]> {
        Box::new([0; ARAM_SIZE])
    }

    #[test]
    fn gaussian_table_matches_ares() {
        let dsp = Dsp::new();
        // Reference values computed from the ares table-build formula.
        assert_eq!(dsp.gaussian[0], 0);
        assert_eq!(dsp.gaussian[255], 370);
        assert_eq!(dsp.gaussian[256], 374);
        assert_eq!(dsp.gaussian[511], 1305);
        // The four taps at phase 0 sum to ~2048 (the table is normalized to unity gain).
        let s = i32::from(dsp.gaussian[0])
            + i32::from(dsp.gaussian[256])
            + i32::from(dsp.gaussian[511])
            + i32::from(dsp.gaussian[255]);
        assert_eq!(s, 2049);
    }

    #[test]
    fn noise_lfsr_steps_match_ares() {
        let mut dsp = Dsp::new();
        assert_eq!(dsp.noise.lfsr, 0x4000);
        // One feedback step: feedback = lfsr<<13 ^ lfsr<<14; lfsr = (feedback&0x4000)|(lfsr>>1).
        dsp.noise.frequency = 31; // rate 31 fires every counter tick
        // Force a counter state that polls true for rate 31 (CounterRate[31]==1 always triggers).
        dsp.misc30();
        assert_eq!(dsp.noise.lfsr, 0x2000);
    }

    #[test]
    fn brr_decode_filter0_scale11() {
        let mut dsp = Dsp::new();
        let mut aram = empty_aram();
        // header: scale=0xB (11), filter=0 → 0xB0; data byte all 0x11 → top nybble 1 each.
        let v = &mut dsp.voice[0];
        v.brr_address = 0x0200;
        v.brr_offset = 1;
        v.buffer_offset = 0;
        aram[0x0200] = 0xB0; // header
        aram[0x0201] = 0x11; // brr.byte source (offset 1)
        aram[0x0202] = 0x11; // next byte
        dsp.brr.byte = 0x11;
        dsp.brr.header = 0xB0;
        dsp.brr_decode(0, &aram);
        // Each decoded sample: (1 << 11) >> 1 = 1024, stored as <<1 = 2048.
        assert_eq!(&dsp.voice[0].buffer[0..4], &[2048, 2048, 2048, 2048]);
    }

    #[test]
    fn brr_decode_filter1_feeds_back_previous() {
        let mut dsp = Dsp::new();
        let aram = empty_aram();
        // filter 1 uses p1 (previous sample). Seed buffer so p1 is nonzero, then decode zeros.
        let v = &mut dsp.voice[0];
        v.buffer = [0; 12];
        v.buffer[11] = 1000; // p1 when buffer_offset==0
        v.buffer_offset = 0;
        dsp.brr.byte = 0x00;
        dsp.brr.header = 0x04; // filter=1, scale=0
        dsp.brr_decode(0, &aram);
        // First sample s=0, +p1>>1 + (-p1)>>5 = 500 - 32 = 468, clamp, <<1 = 936.
        assert_eq!(dsp.voice[0].buffer[0], 936);
    }

    #[test]
    fn gaussian_interpolate_flat_buffer() {
        let mut dsp = Dsp::new();
        dsp.voice[0].buffer = [100; 12];
        dsp.voice[0].buffer_offset = 0;
        dsp.voice[0].gaussian_offset = 0;
        assert_eq!(dsp.gaussian_interpolate(0), 98);
    }

    #[test]
    fn envelope_attack_increments() {
        let mut dsp = Dsp::new();
        let v = &mut dsp.voice[0];
        v.adsr0 = 0xFF; // ADSR enabled, attack rate 0xF
        v.adsr1 = 0xFF;
        v.envelope_mode = EnvMode::Attack;
        v.envelope = 0;
        dsp.latch.adsr0 = 0xFF;
        // Rate (0xF)*2+1 = 31, so attack adds 0x400 — but only commits when counterPoll(31) fires.
        // CounterRate[31]==1 triggers on every tick (counter+offset)%1==0 always true.
        dsp.envelope_run(0);
        assert_eq!(dsp.voice[0].envelope, 0x400);
        assert_eq!(dsp.voice[0].envelope_mode, EnvMode::Attack);
    }

    #[test]
    fn envelope_release_decays_to_zero() {
        let mut dsp = Dsp::new();
        dsp.voice[0].envelope_mode = EnvMode::Release;
        dsp.voice[0].envelope = 0x10;
        for _ in 0..4 {
            dsp.envelope_run(0);
        }
        assert_eq!(dsp.voice[0].envelope, 0); // 0x10 - 4*0x8 = -0x10 → clamped 0
    }

    #[test]
    fn keyon_latches_and_starts_setup() {
        let mut dsp = Dsp::new();
        let mut aram = empty_aram();
        dsp.write(0x6C, 0x00); // FLG: clear mute/reset, noise freq 0
        dsp.write(0x4C, 0x01); // KON voice 0
        // KON propagates through the keylatch→_keyon path across the clock.sample toggle (ares
        // misc29/misc30), so it takes a couple of samples to enter the 5-step setup. Run enough.
        for _ in 0..4 {
            dsp.run_sample(&mut aram);
        }
        assert_ne!(dsp.voice[0].keyon_delay, 0);
        assert_eq!(dsp.voice[0].envelope_mode, EnvMode::Attack);
    }

    #[test]
    fn echo_silent_when_disabled() {
        let mut dsp = Dsp::new();
        let mut aram = empty_aram();
        dsp.write(0x6C, 0x00); // unmute
        dsp.run_sample(&mut aram);
        // No voices active, no echo → DAC output is silence.
        assert_eq!(dsp.last_sample(), (0, 0));
    }

    #[test]
    fn full_sample_run_is_deterministic() {
        let mut a = Dsp::new();
        let mut b = Dsp::new();
        let mut aram_a = empty_aram();
        let mut aram_b = empty_aram();
        for d in [&mut a, &mut b] {
            d.write(0x6C, 0x00);
            d.write(0x0C, 0x7F); // MVOLL
            d.write(0x1C, 0x7F); // MVOLR
        }
        for _ in 0..200 {
            a.run_sample(&mut aram_a);
            b.run_sample(&mut aram_b);
        }
        assert_eq!(a.last_sample(), b.last_sample());
        assert_eq!(&aram_a[..], &aram_b[..]);
    }

    #[test]
    fn run_sample_equals_32_ticks_with_brr_content() {
        // End-state equivalence guard (protects the already-passing DSP output path /
        // undisbeliever_golden / determinism when the DSP became cycle-stepped): a batched
        // `run_sample()` must produce a bit-identical sample stream + ARAM to driving the DSP one
        // `tick()` at a time — the way `lib.rs` now pumps it (one tick per 2 SMP base clocks) when
        // no register access intervenes. Exercises real BRR decode + envelope + echo, not silence.
        let mut a = Dsp::new();
        let mut b = Dsp::new();
        let mut aram_a = empty_aram();
        let mut aram_b = empty_aram();
        for (d, aram) in [
            (&mut a, &mut aram_a as &mut Box<[u8; ARAM_SIZE]>),
            (&mut b, &mut aram_b),
        ] {
            // Sample-directory entry 0 → BRR block at 0x0200 (start == loop point).
            aram[0x0000] = 0x00;
            aram[0x0001] = 0x02;
            aram[0x0002] = 0x00;
            aram[0x0003] = 0x02;
            // One looping BRR block: header scale=11, filter=0, end+loop bits set; nonzero data.
            aram[0x0200] = 0xB3;
            for i in 0..8 {
                aram[0x0201 + i] = 0x57;
            }
            d.write(0x6C, 0x00); // FLG: unmute, clear reset
            d.write(0x0C, 0x60); // MVOLL
            d.write(0x1C, 0x60); // MVOLR
            d.write(0x2C, 0x40); // EVOLL (exercise echo mix)
            d.write(0x3C, 0x40); // EVOLR
            d.write(0x00, 0x60); // V0VOLL
            d.write(0x01, 0x60); // V0VOLR
            d.write(0x02, 0x00); // V0 pitch low
            d.write(0x03, 0x10); // V0 pitch high
            d.write(0x04, 0x00); // V0 source 0
            d.write(0x05, 0x8F); // V0 ADSR0 (ADSR enabled)
            d.write(0x06, 0xE0); // V0 ADSR1
            d.write(0x4D, 0x01); // EON voice 0
            d.write(0x5D, 0x00); // source directory page 0
            d.write(0x4C, 0x01); // KON voice 0
        }
        for _ in 0..400 {
            a.run_sample(&mut aram_a);
            for _ in 0..32 {
                b.tick(&mut aram_b);
            }
            assert_eq!(
                a.last_sample(),
                b.last_sample(),
                "run_sample vs tick×32 sample stream diverged"
            );
        }
        assert_eq!(
            &aram_a[..],
            &aram_b[..],
            "run_sample vs tick×32 ARAM diverged"
        );
    }

    #[test]
    fn endx_register_round_trips_clear() {
        let mut dsp = Dsp::new();
        dsp.registers[0x7C] = 0xFF;
        dsp.write(0x7C, 0x00); // writing ENDX always clears it
        assert_eq!(dsp.read(0x7C), 0);
    }

    #[test]
    fn mute_flag_silences_dac() {
        let mut dsp = Dsp::new();
        let mut aram = empty_aram();
        dsp.write(0x6C, 0x40); // FLG.mute set
        assert!(dsp.muted());
        dsp.run_sample(&mut aram);
        assert_eq!(dsp.last_sample(), (0, 0));
    }
}
