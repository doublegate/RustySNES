//! The Sharp RTC-4513 standalone real-time clock — the only commercial cart is Daikaijuu
//! Monogatari II (an ExHiROM title; ares board `EXHIROM-RAM-SHARPRTC`).
//!
//! Clean-room port of ares' `SharpRTC` (ISC, `sfc/coprocessor/sharprtc/`): a 2-register (`$2800`
//! data, `$2801` unused/pass-through) handshake that walks a 13-slot clock register file
//! (second/minute/hour/day/month/year, each as one or two decimal digits, plus an
//! auto-computed weekday) through a `Ready -> Command -> Write` or `-> Read` state machine
//! driven entirely by magic values written to `$2800` (`$0D`=enter read mode, `$0E`=enter
//! command mode, then `$00`=write / `$04`=reset-to-epoch as the command byte). This is a
//! DIFFERENT chip/protocol from [`crate::coproc::epsonrtc::EpsonRtc`] (the SPC7110-paired Epson
//! RTC-4513) despite the similar name — distinct register windows, distinct handshake, distinct
//! state machine, per ares treating them as two unrelated components.
//!
//! Like [`EpsonRtc`](crate::coproc::epsonrtc::EpsonRtc), this port seeds a fixed epoch and never
//! advances the clock other than via explicit register writes (real wall-clock ticking would
//! break this project's determinism contract, `docs/adr/0004`); no released game logic depends
//! on the clock's absolute value, only on the read/write handshake completing.
//!
//! No commercial Daikaijuu Monogatari II dump exists in this project's local corpus, so this
//! board has unit-test-level coverage only, not golden-framebuffer validation — the same honesty
//! gap already carried openly for ExLoROM/PAL auto-detect (`docs/adr/0003`). The chipset-byte
//! detection in [`crate::header`] is title-matched (best-effort, no cartridge database), mirroring
//! the existing CX4/SPC7110 `$F`-nibble disambiguation.
//!
//! Bus window (ares board `EXHIROM-RAM-SHARPRTC`): `$00-3F,$80-BF:$2800-$2801` (the 2-byte
//! handshake); ROM/SRAM otherwise delegate to the wrapped ExHiROM base board.

// Chip-name jargon (RTC-4513, ...) is not Rust code.
#![allow(clippy::doc_markdown)]

use alloc::boxed::Box;

use rustysnes_savestate::{SaveReader, SaveStateError, SaveWriter};

use crate::board::{Board, Coprocessor, MappedAddr};

/// Days in each month (non-leap), ares `SharpRTC::daysInMonth` — used only by
/// [`calculate_weekday`]'s per-month accumulation (the deterministic port never ticks the clock,
/// so [`SharpRtcBoard`]'s own `tick*` family from ares is intentionally not ported).
const DAYS_IN_MONTH: [u32; 12] = [31, 28, 31, 30, 31, 30, 31, 31, 30, 31, 30, 31];

/// The RTC-4513's protocol state machine (ares `SharpRTC::State`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
enum State {
    #[default]
    Ready,
    Command,
    Read,
    Write,
}

/// Day-of-week for `year`/`month`/`day` (`0`=Sunday .. `6`=Saturday), ares
/// `SharpRTC::calculateWeekday`: the SharpRTC epoch is `1000-01-01` (a Wednesday), so this walks
/// whole years then whole months accumulating day counts (clamping `year`/`month`/`day` to the
/// chip's valid range first, exactly as ares does) rather than using a general Gregorian formula.
fn calculate_weekday(year: u32, month: u32, day: u32) -> u32 {
    let year = year.max(1000);
    let month = month.clamp(1, 12);
    let day = day.clamp(1, 31);

    let is_leap = |y: u32| y.is_multiple_of(4) && (!y.is_multiple_of(100) || y.is_multiple_of(400));

    let mut sum = 0u32;
    let mut y = 1000u32;
    while y < year {
        sum += 365 + u32::from(is_leap(y));
        y += 1;
    }

    let mut m = 1u32;
    while m < month {
        let days = DAYS_IN_MONTH[usize::try_from((m - 1) % 12).unwrap_or(0)];
        let leap_month = days == 28 && is_leap(y);
        sum += days + u32::from(leap_month);
        m += 1;
    }

    sum += day - 1;
    (sum + 3) % 7 // 1000-01-01 was a Wednesday
}

/// An ExHiROM cartridge carrying a standalone Sharp RTC.
pub struct SharpRtcBoard {
    inner: Box<dyn Board>,

    second: u32,
    minute: u32,
    hour: u32,
    day: u32,
    month: u32,
    year: u32,
    weekday: u32,

    state: State,
    /// `-1` is the handshake's own "not yet primed" sentinel (ares `index = -1`), distinct from
    /// every real slot `0..=12`.
    index: i32,
}

impl core::fmt::Debug for SharpRtcBoard {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("SharpRtcBoard")
            .field("inner", &self.inner.name())
            .field("second", &self.second)
            .field("minute", &self.minute)
            .field("hour", &self.hour)
            .field("day", &self.day)
            .field("month", &self.month)
            .field("year", &self.year)
            .field("weekday", &self.weekday)
            .field("state", &self.state)
            .field("index", &self.index)
            .finish()
    }
}

/// Classify a 24-bit CPU address into the RTC's `$2800`/`$2801` window, returning the register
/// index (`0` or `1`) if it lands there.
fn classify(addr24: u32) -> Option<u8> {
    let bank = (addr24 >> 16) & 0xFF;
    let addr = addr24 & 0xFFFF;
    (matches!(bank, 0x00..=0x3F | 0x80..=0xBF) && matches!(addr, 0x2800 | 0x2801))
        .then_some((addr & 1) as u8)
}

impl SharpRtcBoard {
    /// Wrap a base board (`inner`, the cart's ExHiROM ROM/SRAM decode) with a standalone Sharp
    /// RTC, seeded to the chip's own epoch (ares `SharpRTC::power`: `state = Read, index = -1`)
    /// with an all-zero clock rather than the host's wall-clock time (see the module doc).
    #[must_use]
    pub fn new(inner: Box<dyn Board>) -> Self {
        Self {
            inner,
            second: 0,
            minute: 0,
            hour: 0,
            day: 0,
            month: 0,
            year: 0,
            weekday: 0,
            state: State::Read,
            index: -1,
        }
    }

    /// Read one BCD-ish decimal digit of the clock file (ares `SharpRTC::rtcRead`, `address` is
    /// the 0-12 slot). Every arm is `< 100`, well within `u8` range.
    ///
    /// Not marked `const fn`: `SharpRtcBoard` carries a heap-allocated `inner: Box<dyn Board>`
    /// field, so `const`-ness here is cosmetic and fragile against future field changes — the
    /// same posture `coproc::hg51b` already documents for its own dense register-port methods.
    #[allow(clippy::missing_const_for_fn, clippy::cast_possible_truncation)]
    fn rtc_read(&self, address: u8) -> u8 {
        let v = match address {
            0 => self.second % 10,
            1 => self.second / 10,
            2 => self.minute % 10,
            3 => self.minute / 10,
            4 => self.hour % 10,
            5 => self.hour / 10,
            6 => self.day % 10,
            7 => self.day / 10,
            8 => self.month,
            9 => self.year % 10,
            10 => (self.year / 10) % 10,
            11 => self.year / 100,
            12 => self.weekday,
            _ => 0,
        };
        v as u8
    }

    /// Write one decimal digit of the clock file (ares `SharpRTC::rtcWrite`).
    ///
    /// Not marked `const fn`: same rationale as [`Self::rtc_read`] (the enclosing struct carries
    /// a heap-allocated `Box<dyn Board>`).
    #[allow(clippy::missing_const_for_fn)]
    fn rtc_write(&mut self, address: u8, data: u32) {
        match address {
            0 => self.second = self.second / 10 * 10 + data,
            1 => self.second = data * 10 + self.second % 10,
            2 => self.minute = self.minute / 10 * 10 + data,
            3 => self.minute = data * 10 + self.minute % 10,
            4 => self.hour = self.hour / 10 * 10 + data,
            5 => self.hour = data * 10 + self.hour % 10,
            6 => self.day = self.day / 10 * 10 + data,
            7 => self.day = data * 10 + self.day % 10,
            8 => self.month = data,
            9 => self.year = self.year / 10 * 10 + data,
            10 => self.year = self.year / 100 * 100 + data * 10 + self.year % 10,
            11 => self.year = data * 100 + self.year % 100,
            12 => self.weekday = data,
            _ => {}
        }
    }

    /// `$2800`/`$2801` register read (ares `SharpRTC::read`, `address & 1`). `$2801` is not a
    /// real RTC register (ares passes through the bus's open-bus fallback); this port returns 0,
    /// matching [`crate::coproc::epsonrtc::EpsonRtc`]'s existing convention for unhandled slots.
    fn read_register(&mut self, address: u8) -> u8 {
        if address != 0 || self.state != State::Read {
            return 0;
        }
        if self.index < 0 {
            self.index += 1;
            return 15;
        }
        if self.index > 12 {
            self.index = -1;
            return 15;
        }
        #[allow(clippy::cast_sign_loss, clippy::cast_possible_truncation)]
        let slot = self.index as u8;
        self.index += 1;
        self.rtc_read(slot)
    }

    /// `$2800`/`$2801` register write (ares `SharpRTC::write`, `address & 1`, `data & 15`).
    fn write_register(&mut self, address: u8, data: u8) {
        if address != 0 {
            return;
        }
        let data = data & 0xF;
        if data == 0x0D {
            self.state = State::Read;
            self.index = -1;
            return;
        }
        if data == 0x0E {
            self.state = State::Command;
            return;
        }
        if data == 0x0F {
            return; // unknown behavior (ares comment)
        }

        match self.state {
            State::Command => {
                if data == 0 {
                    self.state = State::Write;
                    self.index = 0;
                } else if data == 4 {
                    self.state = State::Ready;
                    self.index = -1;
                    self.second = 0;
                    self.minute = 0;
                    self.hour = 0;
                    self.day = 0;
                    self.month = 0;
                    self.year = 0;
                    self.weekday = 0;
                } else {
                    self.state = State::Ready; // unknown behavior (ares comment)
                }
            }
            State::Write => {
                if (0..12).contains(&self.index) {
                    #[allow(clippy::cast_sign_loss, clippy::cast_possible_truncation)]
                    let slot = self.index as u8;
                    self.rtc_write(slot, u32::from(data));
                    self.index += 1;
                    if self.index == 12 {
                        self.weekday = calculate_weekday(1000 + self.year, self.month, self.day);
                    }
                }
            }
            State::Ready | State::Read => {}
        }
    }
}

impl Board for SharpRtcBoard {
    fn name(&self) -> &'static str {
        "ExHiROM+S-RTC"
    }

    fn coprocessor(&self) -> Coprocessor {
        Coprocessor::Srtc
    }

    fn map(&self, addr24: u32) -> MappedAddr {
        if classify(addr24).is_some() {
            MappedAddr::Coprocessor
        } else {
            self.inner.map(addr24)
        }
    }

    fn read24(&mut self, addr24: u32) -> u8 {
        match classify(addr24) {
            Some(a) => self.read_register(a),
            None => self.inner.read24(addr24),
        }
    }

    fn write24(&mut self, addr24: u32, val: u8) {
        if let Some(a) = classify(addr24) {
            self.write_register(a, val);
        } else {
            self.inner.write24(addr24, val);
        }
    }

    fn rom(&self) -> &[u8] {
        self.inner.rom()
    }

    fn sram(&self) -> &[u8] {
        self.inner.sram()
    }

    fn sram_mut(&mut self) -> &mut [u8] {
        self.inner.sram_mut()
    }

    fn save_state(&self, w: &mut SaveWriter) {
        w.section(*b"SRTC", |s| {
            s.write_u32(self.second);
            s.write_u32(self.minute);
            s.write_u32(self.hour);
            s.write_u32(self.day);
            s.write_u32(self.month);
            s.write_u32(self.year);
            s.write_u32(self.weekday);
            s.write_u8(match self.state {
                State::Ready => 0,
                State::Command => 1,
                State::Read => 2,
                State::Write => 3,
            });
            // `index` ranges -1..=13 (ares' `-1` sentinel plus slots 0-12); store as `index + 1`
            // (0..=14) since the format has no signed-integer primitive.
            #[allow(clippy::cast_sign_loss, clippy::cast_possible_truncation)]
            s.write_u8((self.index + 1) as u8);
        });
        self.inner.save_state(w);
    }

    fn load_state(&mut self, r: &mut SaveReader) -> Result<(), SaveStateError> {
        let mut s = r.expect_section(*b"SRTC")?;
        // Unlike `Obc1Board`'s cursor (an out-of-range `shift` panics the next `$1FF4` access
        // via a `<<`-overflow), NONE of the clock digits below can panic on any `u32` value:
        // `rtc_read`/`rtc_write` only ever `%`/`/` them, which never panics. Accepting them as-is
        // (rather than inventing bit-width bounds these decimal-digit fields don't actually
        // have — each is built from two masked-but-not-decimal-clamped 4-bit register writes, so
        // the real reachable range isn't a clean power-of-two mask either) matches this
        // project's "reject only what would otherwise panic or corrupt an enum" posture.
        let second = s.read_u32()?;
        let minute = s.read_u32()?;
        let hour = s.read_u32()?;
        let day = s.read_u32()?;
        let month = s.read_u32()?;
        let year = s.read_u32()?;
        let weekday = s.read_u32()?;
        let state_byte = s.read_u8()?;
        // `index_byte` is `index + 1` (see `save_state`); clamp to the encoding's valid 0..=14
        // range (decodes to the real `-1..=13` sentinel-plus-slots range) rather than rejecting —
        // `read_register`/`write_register` already re-check `index` against `0..12`/`>12` on
        // every access, so an over-clamped value self-corrects on the very next register access.
        let index_byte = s.read_u8()?.min(14);
        let index = i32::from(index_byte) - 1;
        let state = match state_byte {
            0 => State::Ready,
            1 => State::Command,
            2 => State::Read,
            3 => State::Write,
            _ => {
                return Err(SaveStateError::Invalid(alloc::format!(
                    "SharpRtcBoard state discriminant {state_byte} is not a valid State variant \
                     (0-3)"
                )));
            }
        };
        if s.remaining() != 0 {
            return Err(SaveStateError::Invalid(alloc::format!(
                "SRTC section has {} trailing byte(s)",
                s.remaining()
            )));
        }
        self.second = second;
        self.minute = minute;
        self.hour = hour;
        self.day = day;
        self.month = month;
        self.year = year;
        self.weekday = weekday;
        self.state = state;
        self.index = index;
        self.inner.load_state(r)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::board::ExHiRom;
    use alloc::vec;

    fn board() -> SharpRtcBoard {
        let inner = Box::new(ExHiRom::new(
            vec![0u8; 0x40_0000].into_boxed_slice(),
            vec![0u8; 0x2000].into_boxed_slice(),
        ));
        SharpRtcBoard::new(inner)
    }

    #[test]
    fn window_classify() {
        assert_eq!(classify(0x00_2800), Some(0));
        assert_eq!(classify(0x3F_2801), Some(1));
        assert_eq!(classify(0x80_2800), Some(0));
        assert_eq!(classify(0xBF_2801), Some(1));
        assert_eq!(classify(0x00_2802), None);
        assert_eq!(classify(0x40_2800), None); // outside 00-3f/80-bf
    }

    #[test]
    fn power_on_state_is_read_with_sentinel_index() {
        let b = board();
        assert_eq!(b.state, State::Read);
        assert_eq!(b.index, -1);
    }

    #[test]
    fn write_then_read_clock_roundtrip() {
        let mut b = board();
        b.write24(0x00_2800, 0x0E); // enter Command
        b.write24(0x00_2800, 0x00); // command: Write, index=0
        // second=45, minute=30, hour=12, day=15, month=6, year=1024 (=> stored 24, epoch+1000=1024)
        for digit in [5, 4, 0, 3, 2, 1, 5, 1, 6, 4, 2, 0] {
            b.write24(0x00_2800, digit);
        }
        b.write24(0x00_2800, 0x0D); // enter Read
        assert_eq!(b.read24(0x00_2800), 15); // priming read (ares: index<0 => 15, index=0)
        let mut got = [0u8; 13];
        for slot in &mut got {
            *slot = b.read24(0x00_2800);
        }
        assert_eq!(&got[0..12], &[5, 4, 0, 3, 2, 1, 5, 1, 6, 4, 2, 0]);
        // weekday auto-computed after the 12th write; just confirm it's a valid day index.
        assert!(got[12] < 7);
    }

    #[test]
    fn command_reset_zeroes_the_clock() {
        let mut b = board();
        b.write24(0x00_2800, 0x0E); // Command
        b.write24(0x00_2800, 0x00); // Write, index=0
        b.write24(0x00_2800, 9); // second lo digit = 9
        b.write24(0x00_2800, 0x0E); // Command again
        b.write24(0x00_2800, 0x04); // reset-to-epoch
        assert_eq!(b.second, 0);
        assert_eq!(b.state, State::Ready);
    }

    #[test]
    fn rom_and_sram_delegate_to_inner_board() {
        let mut b = board();
        assert_eq!(b.rom().len(), 0x40_0000);
        // ExHiROM SRAM window: banks 20-3f/a0-bf, $6000-7fff.
        b.write24(0x20_6000, 0x42);
        assert_eq!(b.read24(0x20_6000), 0x42);
    }

    #[test]
    fn clock_and_handshake_state_round_trips_through_save_state() {
        let mut b = board();
        b.write24(0x00_2800, 0x0E);
        b.write24(0x00_2800, 0x00);
        b.write24(0x00_2800, 7); // second lo digit = 7

        let mut w = SaveWriter::new();
        b.save_state(&mut w);
        let bytes = w.into_bytes();

        let mut fresh = board();
        let mut r = SaveReader::new(&bytes);
        fresh.load_state(&mut r).unwrap();

        assert_eq!(fresh.second, 7);
        assert_eq!(fresh.state, State::Write);
        assert_eq!(fresh.index, 1);
        assert_eq!(r.remaining(), 0);
    }

    #[test]
    fn out_of_range_state_discriminant_is_rejected_not_panicked_on() {
        let b = board();
        let mut w = SaveWriter::new();
        b.save_state(&mut w);
        let mut bytes = w.into_bytes();
        // The state discriminant follows the 7 clock u32 fields — corrupt it.
        let offset = 8 + (4 * 7); // section header (tag+len, see SaveWriter::section) + 7 u32s
        bytes[offset] = 99;
        let mut fresh = board();
        let mut r = SaveReader::new(&bytes);
        assert!(matches!(
            fresh.load_state(&mut r),
            Err(SaveStateError::Invalid(_))
        ));
    }

    #[test]
    fn weekday_epoch_is_a_wednesday() {
        assert_eq!(calculate_weekday(1000, 1, 1), 3);
    }
}
