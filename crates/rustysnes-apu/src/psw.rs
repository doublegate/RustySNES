//! The SPC700 Processor Status Word (PSW / `P`).
//!
//! Bit layout (LSB→MSB): `C Z I H B P V N` — carry, zero, interrupt-disable, half-carry,
//! break, direct-page select, overflow, negative. Identical ordering to ares' `SPC700::Flags`.

/// SPC700 processor status word, packed into the canonical bit order.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Psw(u8);

macro_rules! flag {
    ($get:ident, $set:ident, $bit:expr, $doc:literal) => {
        #[doc = concat!("Read the ", $doc, " flag.")]
        #[must_use]
        pub const fn $get(self) -> bool {
            self.0 & (1 << $bit) != 0
        }
        #[doc = concat!("Set the ", $doc, " flag.")]
        pub const fn $set(&mut self, value: bool) {
            if value {
                self.0 |= 1 << $bit;
            } else {
                self.0 &= !(1 << $bit);
            }
        }
    };
}

impl Psw {
    flag!(c, set_c, 0, "carry");
    flag!(z, set_z, 1, "zero");
    flag!(i, set_i, 2, "interrupt-disable");
    flag!(h, set_h, 3, "half-carry");
    flag!(b, set_b, 4, "break");
    flag!(p, set_p, 5, "direct-page-select");
    flag!(v, set_v, 6, "overflow");
    flag!(n, set_n, 7, "negative");

    /// Construct from the raw 8-bit packed representation.
    #[must_use]
    pub const fn from_bits(bits: u8) -> Self {
        Self(bits)
    }

    /// The raw 8-bit packed representation (`PUSH P` / oracle diff form).
    #[must_use]
    pub const fn bits(self) -> u8 {
        self.0
    }

    /// Set `N` and `Z` together from an 8-bit result (the common `set_nz` shorthand).
    pub const fn set_nz(&mut self, value: u8) {
        self.set_z(value == 0);
        self.set_n(value & 0x80 != 0);
    }
}
