//! The Epson RTC-4513 real-time clock — the second ASIC on the one commercial cart that pairs it
//! with SPC7110 (Far East of Eden Zero / Tengai Makyou Zero).
//!
//! Clean-room port of ares' `EpsonRTC` (ISC, `sfc/coprocessor/epsonrtc/`): a 3-register (`$4840`
//! chip-select, `$4841` data, `$4842` ready-status) handshake over a 16-nibble register file (the
//! clock fields + IRQ/mode bits). ares ticks a real wall-clock time into the register file
//! (`EpsonRTC::synchronize`); this port instead seeds a fixed epoch and never advances it other
//! than via explicit register writes — real wall-clock time would break this project's
//! determinism contract (same seed + ROM + input ⇒ bit-identical output, `docs/adr`), and no game
//! logic here depends on the clock's absolute value, only on the read/write handshake completing.
//!
//! This project's host-synced coprocessors (Super FX/CX4/the NEC DSP family) complete a triggered
//! operation instantly rather than modeling ares' `wait`-cycle countdown (`Thread::step`); the RTC
//! follows the same convention — every write/read leaves `ready` set immediately, so a game's
//! poll-for-ready loop always succeeds on its very next check.

#![allow(clippy::doc_markdown)]

/// The RTC-4513's protocol state machine (ares `EpsonRTC::State`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
enum State {
    #[default]
    Mode,
    Seek,
    Read,
    Write,
}

/// The Epson RTC-4513 register file + handshake state.
#[derive(Debug, Clone)]
pub struct EpsonRtc {
    // clock fields (all nibbles; see ares memory.cpp rtcRead/rtcWrite)
    secondlo: u8,
    secondhi: u8,
    batteryfailure: u8,
    minutelo: u8,
    minutehi: u8,
    resync: u8,
    hourlo: u8,
    hourhi: u8,
    meridian: u8,
    daylo: u8,
    dayhi: u8,
    dayram: u8,
    monthlo: u8,
    monthhi: u8,
    monthram: u8,
    yearlo: u8,
    yearhi: u8,
    weekday: u8,
    hold: u8,
    calendar: u8,
    irqflag: u8,
    roundseconds: u8,
    irqmask: u8,
    irqduty: u8,
    irqperiod: u8,
    pause: u8,
    stop: u8,
    atime: u8,
    test: u8,

    // handshake state
    chipselect: u8,
    state: State,
    offset: u8,
    ready: bool,
    mdr: u8,
}

impl Default for EpsonRtc {
    fn default() -> Self {
        Self::new()
    }
}

impl EpsonRtc {
    /// Build an RTC seeded to a fixed epoch (all-zero clock fields) rather than the host's real
    /// wall-clock time (see the module doc's determinism note).
    #[must_use]
    pub const fn new() -> Self {
        Self {
            secondlo: 0,
            secondhi: 0,
            batteryfailure: 1,
            minutelo: 0,
            minutehi: 0,
            resync: 0,
            hourlo: 0,
            hourhi: 0,
            meridian: 0,
            daylo: 0,
            dayhi: 0,
            dayram: 0,
            monthlo: 0,
            monthhi: 0,
            monthram: 0,
            yearlo: 0,
            yearhi: 0,
            weekday: 0,
            hold: 0,
            calendar: 0,
            irqflag: 0,
            roundseconds: 0,
            irqmask: 0,
            irqduty: 0,
            irqperiod: 0,
            pause: 0,
            stop: 0,
            atime: 0,
            test: 0,
            chipselect: 0,
            state: State::Mode,
            offset: 0,
            ready: false,
            mdr: 0,
        }
    }

    const fn rtc_reset(&mut self) {
        self.state = State::Mode;
        self.offset = 0;
        self.resync = 0;
        self.pause = 0;
        self.test = 0;
    }

    fn rtc_read(&mut self, address: u8) -> u8 {
        match address & 0xF {
            0 => self.secondlo,
            1 => self.secondhi | (self.batteryfailure << 3),
            2 => self.minutelo,
            3 => self.minutehi | (self.resync << 3),
            4 => self.hourlo,
            5 => self.hourhi | (self.meridian << 2) | (self.resync << 3),
            6 => self.daylo,
            7 => self.dayhi | (self.dayram << 2) | (self.resync << 3),
            8 => self.monthlo,
            9 => self.monthhi | (self.monthram << 1) | (self.resync << 3),
            10 => self.yearlo,
            11 => self.yearhi,
            12 => self.weekday | (self.resync << 3),
            13 => {
                let readflag = u8::from(self.irqflag != 0 && self.irqmask == 0);
                self.irqflag = 0;
                self.hold | (self.calendar << 1) | (readflag << 2) | (self.roundseconds << 3)
            }
            14 => self.irqmask | (self.irqduty << 1) | (self.irqperiod << 2),
            _ => self.pause | (self.stop << 1) | (self.atime << 2) | (self.test << 3),
        }
    }

    const fn rtc_write(&mut self, address: u8, data: u8) {
        let data = data & 0xF;
        match address & 0xF {
            0 => self.secondlo = data,
            1 => {
                self.secondhi = data & 0x7;
                self.batteryfailure = data >> 3;
            }
            2 => self.minutelo = data,
            3 => self.minutehi = data,
            4 => self.hourlo = data,
            5 => {
                self.hourhi = data;
                self.meridian = (data >> 2) & 1;
                if self.atime == 1 {
                    self.meridian = 0;
                }
                if self.atime == 0 {
                    self.hourhi &= 1;
                }
            }
            6 => self.daylo = data,
            7 => {
                self.dayhi = data & 0x3;
                self.dayram = data >> 2;
            }
            8 => self.monthlo = data,
            9 => {
                self.monthhi = data & 0x1;
                self.monthram = data >> 1;
            }
            10 => self.yearlo = data,
            11 => self.yearhi = data,
            12 => self.weekday = data,
            13 => {
                self.hold = data & 1;
                self.calendar = (data >> 1) & 1;
                self.roundseconds = data >> 3;
            }
            14 => {
                self.irqmask = data & 1;
                self.irqduty = (data >> 1) & 1;
                self.irqperiod = data >> 2;
            }
            _ => {
                self.pause = data & 1;
                self.stop = (data >> 1) & 1;
                self.atime = (data >> 2) & 1;
                self.test = data >> 3;
                if self.atime == 1 {
                    self.meridian = 0;
                }
                if self.atime == 0 {
                    self.hourhi &= 1;
                }
                if self.pause != 0 {
                    self.secondlo = 0;
                    self.secondhi = 0;
                }
            }
        }
    }

    /// `$4840-$4842` register read (ares `EpsonRTC::read`, `address & 3`).
    #[must_use]
    pub fn read(&mut self, address: u32) -> u8 {
        match address & 3 {
            0 => self.chipselect,
            1 => {
                if self.chipselect != 1 || !self.ready {
                    return 0;
                }
                match self.state {
                    State::Write => self.mdr,
                    State::Read => {
                        let offset = self.offset;
                        self.offset = self.offset.wrapping_add(1);
                        self.rtc_read(offset)
                    }
                    State::Mode | State::Seek => 0,
                }
            }
            2 => u8::from(self.ready) << 7,
            _ => 0,
        }
    }

    /// `$4840-$4842` register write (ares `EpsonRTC::write`, `address & 3`, `data & 15`).
    pub const fn write(&mut self, address: u32, data: u8) {
        let data = data & 0xF;
        match address & 3 {
            0 => {
                self.chipselect = data;
                if self.chipselect != 1 {
                    self.rtc_reset();
                }
                self.ready = true;
            }
            1 => {
                if self.chipselect != 1 || !self.ready {
                    return;
                }
                match self.state {
                    State::Mode => {
                        if data != 0x03 && data != 0x0c {
                            return;
                        }
                        self.state = State::Seek;
                        self.mdr = data;
                        self.ready = true; // host-sync: skip the `wait` countdown
                    }
                    State::Seek => {
                        self.state = if self.mdr == 0x03 {
                            State::Write
                        } else {
                            State::Read
                        };
                        self.offset = data;
                        self.mdr = data;
                        self.ready = true;
                    }
                    State::Write => {
                        let offset = self.offset;
                        self.offset = self.offset.wrapping_add(1);
                        self.rtc_write(offset, data);
                        self.mdr = data;
                        self.ready = true;
                    }
                    State::Read => {}
                }
            }
            _ => {}
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn chipselect_then_write_read_roundtrip() {
        let mut rtc = EpsonRtc::new();
        rtc.write(0, 1); // chip select
        assert_eq!(rtc.read(0), 1);
        rtc.write(1, 0x03); // mode: write
        rtc.write(1, 0x00); // seek to offset 0
        rtc.write(1, 0x05); // write secondlo = 5
        rtc.write(0, 0); // deselect
        rtc.write(0, 1); // re-select (chipselect actually changed, so this resets the cursor)
        rtc.write(1, 0x0c); // mode: read
        rtc.write(1, 0x00); // seek to offset 0
        assert_eq!(rtc.read(1), 5);
    }

    #[test]
    fn ready_flag_reads_high_bit() {
        let mut rtc = EpsonRtc::new();
        rtc.write(0, 1);
        assert_eq!(rtc.read(2), 0x80);
    }
}
