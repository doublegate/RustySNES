//! Board-tier honesty gate (ADR 0003 — the RustyNES `mapper_tier_honesty` port).
//!
//! The headline claim ("N boards / coprocessors, accuracy-battery 100%") is only honest if no
//! `BestEffort`-tier board (register-decode-only, NOT covered by the accuracy/commercial
//! oracle) silently backs an oracle ROM. This test enforces that invariant: every board in
//! the accuracy-oracle set must resolve to a `Core` or `Curated` (accuracy-gated) tier.
//!
//! The tier classifier (`rustysnes_core::cart::board_tier`) is the single source of truth; the
//! gate reads it. The `ORACLE_COPROCESSORS` set is the explicit list of board families that
//! back a byte-identity oracle ROM — adding a `BestEffort` family here fails CI by
//! construction, which is exactly the protection ADR 0003 specifies.

use rustysnes_core::cart::{BoardTier, Coprocessor, board_tier};

/// The coprocessor families whose boards back an accuracy / commercial-oracle ROM. Base ROM
/// mapping (`Coprocessor::None`) and DSP-1 are oracle-gated; the enhancement-chip families
/// graduate into this set as redistributable fixtures land. A `BestEffort` family must NEVER
/// appear here.
const ORACLE_COPROCESSORS: &[Coprocessor] = &[Coprocessor::None, Coprocessor::Dsp];

/// Every board backing the accuracy oracle must resolve to an accuracy-gated tier.
#[test]
fn no_besteffort_board_backs_the_oracle() {
    for &copro in ORACLE_COPROCESSORS {
        let tier = board_tier(copro);
        assert!(
            tier.is_accuracy_gated(),
            "oracle-backing board family {copro:?} resolves to tier {tier:?} — an accuracy \
             oracle must be backed by a Core/Curated board, never BestEffort (ADR 0003 \
             honesty invariant)"
        );
    }
}

/// The tier sets must stay structurally disjoint: `BestEffort` is never accuracy-gated.
#[test]
fn best_effort_tier_is_structurally_excluded() {
    assert!(!BoardTier::BestEffort.is_accuracy_gated());
    assert!(BoardTier::Core.is_accuracy_gated());
    assert!(BoardTier::Curated.is_accuracy_gated());

    // And no coprocessor classified BestEffort is in the oracle set.
    for copro in [
        Coprocessor::SDd1,
        Coprocessor::Spc7110,
        Coprocessor::Cx4,
        Coprocessor::Obc1,
    ] {
        assert_eq!(board_tier(copro), BoardTier::BestEffort);
        assert!(
            !ORACLE_COPROCESSORS.contains(&copro),
            "BestEffort coprocessor {copro:?} must not be in the oracle set"
        );
    }
}
