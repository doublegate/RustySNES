//! On-cart coprocessors that plug into the [`crate::board::Board`] trait.
//!
//! The headline economy of the SNES coprocessor breadth (`docs/cart.md`, `docs/adr/0003`) is the
//! **shared NEC DSP engine**: one [`upd77c25::Upd77c25`] LLE core backs DSP-1/2/3/4 + ST010/011
//! (six chips), parameterized only by firmware + register widths. Phase 4 lands DSP-1
//! ([`dsp1::Dsp1Board`], `Core`/`Curated`); the BestEffort siblings reuse the same engine in
//! Phase 7.
//!
//! Phase 4's second Core/Curated coprocessor is the **Super FX / GSU** ([`superfx::SuperFxBoard`]
//! over the [`gsu::Gsu`] Argonaut RISC core). Unlike the DSP family it carries no chip-ROM dump —
//! the GSU program lives in the cartridge ROM — so it is functional the moment the cart loads. It
//! reuses the same host-sync idea: the GSU runs to completion the instant the CPU sets the Go
//! flag, so no free-running core-scheduler tick is needed.
//!
//! Phase 4's third Core/Curated coprocessor is the **SA-1** ([`sa1::Sa1Board`]) — a second WDC
//! 65C816 @ ~10.74 MHz plus a support ASIC. Like Super FX it carries no chip-ROM dump (the SA-1
//! program is in the cartridge ROM). Unlike the host-sync chips it is a real parallel CPU: this
//! board owns the entire SA-1 *system* (registers, Super-MMC banking, BW-RAM, I-RAM, arithmetic
//! unit, DMA, var-len, H/V timer) and exposes the SA-1 CPU's memory view via the `Board`
//! second-CPU hooks; `rustysnes-core` instantiates and steps the second CPU (the crate graph
//! forbids `rustysnes-cart` from depending on `rustysnes-cpu`).

// Chip-name jargon (µPD77C25, µPD96050, ST010, …) is not Rust code.
#![allow(clippy::doc_markdown)]

pub mod cx4;
pub mod dsp1;
pub mod epsonrtc;
pub mod gsu;
pub mod hg51b;
pub mod necdsp_variant;
pub mod obc1;
pub mod sa1;
pub mod sdd1;
pub mod spc7110;
pub mod superfx;
pub mod upd77c25;

pub use cx4::Cx4Board;
pub use dsp1::Dsp1Board;
pub use epsonrtc::EpsonRtc;
pub use gsu::Gsu;
pub use hg51b::{Hg51b, Hg51bBus};
pub use necdsp_variant::{NecDspVariantBoard, Variant as NecDspVariant};
pub use obc1::Obc1Board;
pub use sa1::Sa1Board;
pub use sdd1::Sdd1Board;
pub use spc7110::Spc7110Board;
pub use superfx::SuperFxBoard;
pub use upd77c25::{Revision, Upd77c25};
