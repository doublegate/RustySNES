//! Board accuracy-tiering — the honesty marker (ADR 0003, the RustyNES ADR 0011 port).
//!
//! The headline claim ("N boards / coprocessors, accuracy-battery 100%") is only honest if no
//! `BestEffort`-tier board (register-decode-only, NOT covered by the accuracy/commercial
//! oracle) silently backs an oracle ROM. The tier is an honesty marker, NOT a behavioural
//! one: a board's runtime behaviour is identical regardless of tier. [`board_tier`] is the
//! single source of truth the harness honesty gate reads. See `docs/adr/0003`.

use crate::header::Coprocessor;

/// Accuracy-evidence tier for a board / coprocessor family.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum BoardTier {
    /// Spec-implemented + accuracy/commercial-oracle-gated (the base map modes, DSP-1).
    Core,
    /// Curated long-tail: demand + a redistributable fixture/spec; unit + boot-smoke tested.
    Curated,
    /// Best-effort: reference-ported, register-decode tested only, NEVER accuracy-gated.
    BestEffort,
}

impl BoardTier {
    /// Human-readable tier name (docs / UI badges / logs).
    #[must_use]
    pub const fn name(self) -> &'static str {
        match self {
            Self::Core => "Core",
            Self::Curated => "Curated",
            Self::BestEffort => "BestEffort",
        }
    }

    /// Whether this tier is covered by the accuracy / commercial-ROM oracle gate.
    /// `Core` and `Curated` are; `BestEffort` is structurally never gated.
    #[must_use]
    pub const fn is_accuracy_gated(self) -> bool {
        matches!(self, Self::Core | Self::Curated)
    }
}

/// Classify a coprocessor family into a [`BoardTier`].
///
/// The base map modes ([`Coprocessor::None`] LoROM/HiROM/ExHiROM) are `Core`. As real
/// coprocessor boards land they move from `BestEffort` → `Curated`/`Core` once a
/// redistributable fixture or the accuracy battery backs them. The honesty gate reads THIS
/// classifier, so it can never be the case that a `BestEffort` board sits in the oracle set.
#[must_use]
pub const fn board_tier(copro: Coprocessor) -> BoardTier {
    match copro {
        // Base ROM mapping + DSP-1 are the well-understood, oracle-gated families.
        Coprocessor::None | Coprocessor::Dsp => BoardTier::Core,
        // The big well-documented enhancement chips: curated as fixtures land.
        Coprocessor::SuperFx | Coprocessor::Sa1 => BoardTier::Curated,
        // The decompression / niche chips: best-effort until a fixture + oracle exist.
        Coprocessor::SDd1 | Coprocessor::Spc7110 | Coprocessor::Cx4 | Coprocessor::Obc1 => {
            BoardTier::BestEffort
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn base_mapping_is_core_and_gated() {
        assert_eq!(board_tier(Coprocessor::None), BoardTier::Core);
        assert!(board_tier(Coprocessor::None).is_accuracy_gated());
    }

    #[test]
    fn best_effort_is_never_gated() {
        assert!(!BoardTier::BestEffort.is_accuracy_gated());
        assert!(!board_tier(Coprocessor::Cx4).is_accuracy_gated());
    }
}
