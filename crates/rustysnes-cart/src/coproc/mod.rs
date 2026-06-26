//! On-cart coprocessors that plug into the [`crate::board::Board`] trait.
//!
//! The headline economy of the SNES coprocessor breadth (`docs/cart.md`, `docs/adr/0003`) is the
//! **shared NEC DSP engine**: one [`upd77c25::Upd77c25`] LLE core backs DSP-1/2/3/4 + ST010/011
//! (six chips), parameterized only by firmware + register widths. Phase 4 lands DSP-1
//! ([`dsp1::Dsp1Board`], `Core`/`Curated`); the BestEffort siblings reuse the same engine in
//! Phase 7.

// Chip-name jargon (µPD77C25, µPD96050, ST010, …) is not Rust code.
#![allow(clippy::doc_markdown)]

pub mod dsp1;
pub mod upd77c25;

pub use dsp1::Dsp1Board;
pub use upd77c25::{Revision, Upd77c25};
