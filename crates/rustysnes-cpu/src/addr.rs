//! Addressing-mode resolution for the 65C816.
//!
//! Each mode resolves to an effective 24-bit address (and, where relevant, whether an indexed
//! access crossed a 256-byte page boundary — the documented `+1` cycle penalty). Operand
//! bytes are fetched from the program bank (`PBR`) by the executor's instruction-fetch path.
//!
//! Bank handling follows the hardware rules: direct-page and stack-relative effective
//! addresses live in bank `0`; `(dp),Y` / `[dp]` / `abs,X` etc. add the index/`DBR` and may
//! carry into the next bank for the long forms. See `docs/cpu.md` ("Addressing modes").

/// The enumerated 65C816 addressing modes the executor dispatches on.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Mode {
    /// Immediate operand whose width follows `M` (memory/accumulator modes).
    ImmediateM,
    /// Immediate operand whose width follows `X` (index modes).
    ImmediateX,
    /// Direct page: `D + dp`.
    Direct,
    /// Direct page indexed by `X`: `D + dp + X`.
    DirectX,
    /// Direct page indexed by `Y`: `D + dp + Y`.
    DirectY,
    /// Direct indirect: `[D + dp]` → 16-bit pointer in bank `DBR`.
    DirectIndirect,
    /// Direct indexed indirect: `[D + dp + X]` → pointer in bank `DBR`.
    DirectXIndirect,
    /// Direct indirect indexed: `[D + dp]` + `Y`, base in bank `DBR`.
    DirectIndirectY,
    /// Direct indirect long: `[D + dp]` → 24-bit pointer.
    DirectIndirectLong,
    /// Direct indirect long indexed: `[D + dp]` (24-bit) + `Y`.
    DirectIndirectLongY,
    /// Absolute: `DBR:operand`.
    Absolute,
    /// Absolute indexed by `X`: `DBR:operand + X`.
    AbsoluteX,
    /// Absolute indexed by `Y`: `DBR:operand + Y`.
    AbsoluteY,
    /// Absolute long: 24-bit operand.
    AbsoluteLong,
    /// Absolute long indexed by `X`: 24-bit operand + `X`.
    AbsoluteLongX,
    /// Stack relative: `S + sr` (bank `0`).
    StackRelative,
    /// Stack relative indirect indexed: `[S + sr]` (bank `0`) + `Y`, base bank `DBR`.
    StackRelativeIndirectY,
}

/// A resolved effective address plus the page-cross flag used for the indexed `+1` penalty.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Effective {
    /// The 24-bit effective address (low byte of a multi-byte operand).
    pub addr: u32,
    /// Whether an indexed mode crossed a 256-byte page boundary.
    pub page_cross: bool,
    /// Whether the operand lives in the bank-`0` direct-page / stack window, where a 16-bit
    /// access wraps its high-byte address within `$0000..=$FFFF` instead of carrying into the
    /// next bank. Set for direct-page and stack-relative modes; clear for absolute/long modes.
    pub bank0_wrap: bool,
}
