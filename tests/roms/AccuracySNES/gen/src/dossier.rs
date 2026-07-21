//! The cart-test → dossier-assertion map (ticket **T-04-J**).
//!
//! # Why this file exists
//!
//! The cartridge numbers its tests sequentially per sub-group. The dossier
//! (`docs/accuracysnes-research-dossier.md` §5) numbers *assertions*. The two schemes look
//! identical and are not: cart `A1.04` is dossier `A1.06`, cart `A2.05` is dossier `A2.06`, cart
//! `A3.05` is dossier `A3.10`. Reading coverage off the ID numbers therefore reports gaps that do
//! not exist — and it already cost real rework, when a batch of seven "remaining Group A" tests
//! was written against that assumption and four turned out to duplicate existing tests.
//!
//! So the mapping is written down here, once, and checked at generation time. Coverage becomes a
//! query (`docs/accuracysnes-coverage.md`, regenerated with the ROM) instead of a guess, and an
//! accidental duplicate becomes a build failure instead of something spotted by eye.
//!
//! # The rules the generator enforces
//!
//! 1. Every test in the battery must appear in [`MAP`]. Adding a test without mapping it fails the
//!    build — the whole point is that the map cannot silently fall behind.
//! 2. A dossier assertion claimed by **more than one** test must be declared in [`SPLITS`] with a
//!    reason. Splitting one assertion across two tests is legitimate and common here (opposite
//!    failure modes usually deserve their own failure codes), but it has to be deliberate. An
//!    undeclared double-claim is exactly the bug this ticket exists to prevent.
//! 3. A test that implements no enumerated assertion must map to `&[]` and be declared in
//!    [`UNENUMERATED`] with a reason. Supporting tests and golden vectors are legitimate; silently
//!    unmapped ones are not.
//!
//! # The denominator is complete
//!
//! All **43** sub-groups of the dossier's Part V now enumerate their assertions in per-ID tables —
//! **443** of them. That was not true when this file was written: 23 sub-groups were prose, so
//! coverage could only be reported for the 232 assertions that happened to sit in tables, and
//! figures for the rest were guesses. Converting the prose (content preserved verbatim, only
//! restructured) removed the last place where "we don't know what we haven't tested" could hide.
//!
//! Consequence worth knowing: `docs/accuracysnes-coverage.md` is now a complete statement of what
//! the battery does and does not cover. If an assertion is missing a test, it is in that file.

/// Every cart test, mapped to the dossier assertion(s) it implements.
///
/// Entries are ordered by group then by the order the tests appear in the battery.
pub const MAP: &[(&str, &[&str])] = &[
    // --- Group A: 65C816 ---------------------------------------------------------------------
    ("A1.01", &["A1.01"]),
    ("A1.02", &["A1.04"]),
    ("A1.03", &["A1.01"]),
    ("A1.04", &["A1.06"]),
    ("A1.05", &["A1.07"]),
    ("A1.06", &["A1.08"]),
    ("A2.01", &["A2.01"]),
    ("A2.02", &["A2.02"]),
    ("A2.03", &["A2.03"]),
    ("A2.04", &["A2.04"]),
    ("A2.05", &["A2.06"]),
    ("A3.01", &["A3.01"]),
    ("A3.02", &["A3.02"]),
    ("A3.03", &["A3.03", "A3.04"]),
    ("A3.04", &["A3.05"]),
    ("A3.05", &["A3.10"]),
    ("A4.01", &["A4.01"]),
    ("A4.02", &["A4.02"]),
    ("A4.03", &["A4.06"]),
    ("A4.04", &["A4.07"]),
    ("A4.05", &["A4.08"]),
    ("A5.01", &["A5.11"]),
    ("A5.02", &["A5.12"]),
    ("A5.03", &["A5.13"]),
    ("A5.04", &["A5.21"]),
    ("A5.05", &["A5.17"]),
    ("A5.06", &["A5.15"]),
    ("A5.07", &["A5.14"]),
    ("A5.08", &["A5.22"]),
    ("A1.08", &["A1.02"]),
    ("A1.09", &["A1.03"]),
    ("A8.05", &["A8.04"]),
    ("A1.10", &["A1.05"]),
    ("A4.07", &["A4.03"]),
    ("A2.12", &["A2.08"]),
    ("A4.09", &["A4.09"]),
    ("A4.10", &["A4.10"]),
    ("A8.06", &["A8.05"]),
    ("A3.08", &["A3.08"]),
    ("A3.06", &["A3.06"]),
    ("A5.09", &["A5.09"]),
    ("A5.10", &["A5.10"]),
    ("C14.03", &["C14.03"]),
    ("A6.11", &["A6.11"]),
    ("A8.07", &["A8.06"]),
    ("A6.12", &["A6.12"]),
    ("A4.11", &["A4.04"]),
    ("A4.12", &["A4.05"]),
    ("A6.13", &["A6.09"]),
    ("A6.14", &["A6.10"]),
    ("A2.13", &["A2.05"]),
    ("B4.17", &["B4.06"]),
    ("D2.07", &["D2.07"]),
    ("D1.14", &["D1.14"]),
    ("D1.11", &["D1.11"]),
    ("D1.08", &["D1.08"]),
    ("B4.16", &[]),
    // --- T-04-I opcode sweep: many tests, one enumerated assertion (declared in SPLITS) ---
    ("A5.S01", &["A5.01-08"]),
    ("A5.S02", &["A5.01-08"]),
    ("A5.S03", &["A5.01-08"]),
    ("A5.S04", &["A5.01-08"]),
    ("A5.S05", &["A5.01-08"]),
    ("A5.S06", &["A5.01-08"]),
    ("A5.S07", &["A5.01-08"]),
    ("A5.S08", &["A5.01-08"]),
    ("A5.S09", &["A5.01-08"]),
    ("A5.S10", &["A5.01-08"]),
    ("A5.S11", &["A5.01-08"]),
    ("A5.S12", &["A5.01-08"]),
    ("A5.S13", &["A5.01-08"]),
    ("A5.S14", &["A5.01-08"]),
    ("A5.S15", &["A5.01-08"]),
    ("A5.S16", &["A5.01-08"]),
    ("A5.S17", &["A5.01-08"]),
    ("A5.S18", &["A5.01-08"]),
    ("A5.S19", &["A5.01-08"]),
    ("A5.S20", &["A5.01-08"]),
    ("A5.S21", &["A5.01-08"]),
    ("A5.S22", &["A5.01-08"]),
    ("A5.S23", &["A5.01-08"]),
    ("A5.S24", &["A5.01-08"]),
    ("A5.S25", &["A5.01-08"]),
    ("A5.S26", &["A5.01-08"]),
    ("A5.S27", &["A5.01-08"]),
    ("A5.S28", &["A5.01-08"]),
    ("A5.S29", &["A5.01-08"]),
    ("A5.S30", &["A5.01-08"]),
    ("A5.S31", &["A5.01-08"]),
    ("A5.S32", &["A5.01-08"]),
    ("A5.S33", &["A5.01-08"]),
    ("A5.S34", &["A5.01-08"]),
    ("A6.01", &["A6.01"]),
    ("A6.02", &["A6.01"]),
    ("A6.03", &["A6.06"]),
    ("A6.04", &["A6.07"]),
    ("A6.05", &["A6.03"]),
    ("A6.06", &["A6.04"]),
    ("A6.07", &["A6.08"]),
    ("A6.08", &["A6.14"]),
    ("A6.09", &["A6.05"]),
    ("A7.01", &["A7.01"]),
    ("A7.02", &["A7.02"]),
    ("A7.03", &["A7.03"]),
    ("A7.04", &["A7.05"]),
    ("A8.01", &["A8.02"]),
    ("A8.02", &["A8.02"]),
    ("A8.03", &["A8.03"]),
    ("A8.04", &["A8.01"]),
    ("A9.01", &["A9.01", "A9.02"]),
    ("A9.02", &[]),
    ("A9.03", &[]),
    // --- Group B: 5A22 -----------------------------------------------------------------------
    ("B1.01", &["B1.01"]),
    ("B1.02", &["B1.02"]),
    ("B1.03", &["B1.03"]),
    ("B1.04", &["B1.04"]),
    ("B2.06", &["B2.06"]),
    ("B2.10", &["B2.10"]),
    ("B4.07", &["B4.07"]),
    ("B4.09", &["B4.09"]),
    ("B2.04", &["B2.04"]),
    ("B2.05", &["B2.05"]),
    ("B4.14", &["B4.14"]),
    ("B3.01", &["B3.01", "B3.02", "B3.03"]),
    ("B4.13", &["B4.13"]),
    ("B4.11", &["B4.11"]),
    ("C9.05", &["C9.05"]),
    ("C2.09", &["C2.09"]),
    ("C3.10", &["C3.05"]),
    ("C3.11", &["C3.05"]),
    ("G1.19", &["G1.01"]),
    ("E4.03", &["E4.03"]),
    ("E4.11", &["E4.11"]),
    ("E1.07", &["E1.07"]),
    ("E3.02", &["E3.02"]),
    ("E5.10", &["E5.10"]),
    ("E8.07", &["E8.07"]),
    ("D2.09", &["D2.09"]),
    ("G1.20", &["G1.03"]),
    ("F1.04", &["F1.04"]),
    ("F1.14", &["F1.14"]),
    ("E2.04", &["E2.04"]),
    ("G1.07", &["G1.07"]),
    ("E9.03", &["E9.03"]),
    ("E9.01", &["E9.01"]),
    ("E7.04", &["E7.04"]),
    ("E7.09", &["E7.09"]),
    ("E7.05", &["E7.05"]),
    ("E7.06", &["E7.06"]),
    ("E7.07", &["E7.07"]),
    ("E7.03", &["E7.03"]),
    ("E7.12", &["E7.12"]),
    ("C1.08", &["C1.08"]),
    ("E8.03", &["E8.03"]),
    ("E1.14", &["E1.14"]),
    ("E10.01", &["E10.01"]),
    ("E10.05", &["E10.05"]),
    ("E6.11", &["E6.11"]),
    ("E6.09", &["E6.09"]),
    ("E9.15", &["E9.15"]),
    ("C7.04", &["C7.04"]),
    ("E5.12", &["E5.12"]),
    ("F1.01", &["F1.01"]),
    ("F1.07", &["F1.07"]),
    ("F1.05", &["F1.05"]),
    ("F1.06", &["F1.06"]),
    ("F1.11", &["F1.11"]),
    ("F1.12", &["F1.12"]),
    ("E9.05", &["E9.05"]),
    ("E9.13", &["E9.13"]),
    ("E8.10", &["E8.10"]),
    ("E5.01", &["E5.01"]),
    ("D1.01", &["D1.01"]),
    ("D1.01b", &["D1.01"]),
    ("D1.02", &["D1.02"]),
    ("D1.06", &["D1.06"]),
    ("D1.07", &["D1.07"]),
    ("D1.07b", &["D1.07"]),
    ("D1.10", &["D1.10"]),
    ("D1.05", &["D1.05"]),
    ("D1.09", &["D1.09", "D1.15"]),
    ("D2.03", &["D2.03"]),
    ("D2.04", &["D2.04"]),
    ("D1.03", &["D1.03"]),
    ("D1.04", &["D1.04"]),
    ("D2.05", &["D2.05"]),
    ("D2.06", &["D2.06"]),
    ("E1.01", &["E1.01"]),
    ("E1.02", &["E1.02"]),
    ("E1.04", &["E1.04"]),
    ("E1.05", &["E1.05"]),
    ("E1.13", &["E1.13"]),
    ("E2.01", &["E2.01"]),
    ("E2.05", &["E2.05"]),
    ("E3.01", &["E3.01"]),
    ("E3.11", &["E3.11"]),
    ("E3.11b", &[]),
    ("E3.11c", &[]),
    ("E9.19", &["E9.19"]),
    ("F1.02", &["F1.02"]),
    ("A1.07", &["A1.09"]),
    ("A2.11", &["A2.09"]),
    ("A6.10", &["A6.02"]),
    ("A7.05", &["A7.04"]),
    ("A9.04", &["A9.03"]),
    ("E3.06", &["E3.06"]),
    ("E3.08", &["E3.08"]),
    ("E6.02", &["E6.02"]),
    ("E6.02b", &["E6.02"]),
    ("E6.02c", &["E6.02"]),
    ("E6.02d", &["E6.02"]),
    ("G1.02", &["G1.02"]),
    ("G1.04", &["G1.04"]),
    ("G1.08", &["G1.08"]),
    ("G1.10", &["G1.10"]),
    ("G1.11", &["G1.11"]),
    ("G1.12", &["G1.12"]),
    ("G1.14", &["G1.14"]),
    ("A2.07", &["A2.07"]),
    ("A3.07", &["A3.07"]),
    ("A3.09", &["A3.09"]),
    ("A2.10", &["A2.10"]),
    ("E1.03", &["E1.03"]),
    ("E1.08", &["E1.08"]),
    ("E2.08", &["E2.08"]),
    ("E2.09", &["E2.09"]),
    ("E3.03", &["E3.03"]),
    ("E3.04", &["E3.04"]),
    ("E3.05", &["E3.05"]),
    ("E3.10", &["E3.10"]),
    ("E1.09", &["E1.09"]),
    ("E1.10", &["E1.10"]),
    ("E1.12", &["E1.12"]),
    ("E2.02", &["E2.02"]),
    ("E2.03", &["E2.03"]),
    ("E2.06", &["E2.06"]),
    ("E2.07", &["E2.07"]),
    ("E4.01", &["E4.01"]),
    ("E4.02", &["E4.02"]),
    ("E4.04", &["E4.04"]),
    ("E5.02", &["E5.02"]),
    ("E7.16", &["E7.16"]),
    ("E8.04", &["E8.04"]),
    ("E9.04", &["E9.04"]),
    ("E9.06", &["E9.06"]),
    ("E9.10", &["E9.10"]),
    ("E9.12", &["E9.12"]),
    ("E9.17", &["E9.17"]),
    ("E9.18", &["E9.18"]),
    ("E5.03", &["E5.03"]),
    ("E5.04", &["E5.04"]),
    ("E5.05", &["E5.05"]),
    ("E7.14", &["E7.14"]),
    ("E5.07", &["E5.07"]),
    ("E5.08", &["E5.08"]),
    ("E5.09", &["E5.09"]),
    ("E5.11", &["E5.11"]),
    ("E7.10", &["E7.10"]),
    ("E7.01", &["E7.01"]),
    ("E7.08", &["E7.08"]),
    ("E7.11", &["E7.11"]),
    ("E7.15", &["E7.15"]),
    ("E3.14", &["E3.14"]),
    ("E1.06", &["E1.06"]),
    ("E1.15", &["E1.15"]),
    ("B4.03", &["B4.03"]),
    ("B4.04", &["B4.04"]),
    ("B4.05", &["B4.05"]),
    ("B4.08", &["B4.08"]),
    ("B4.12", &["B4.12"]),
    ("B4.15", &["G1.09"]),
    ("B5.01", &["B5.01"]),
    ("B5.02", &["B5.02"]),
    ("B5.03", &[]),
    ("B5.04", &["B5.03"]),
    ("B5.05", &["B5.04"]),
    // --- Group C: S-PPU1 / S-PPU2 ------------------------------------------------------------
    ("C1.01", &["C1.02"]),
    ("C1.02", &["C1.02"]),
    ("C1.03", &["C1.01"]),
    ("C1.04", &["C1.05"]),
    ("C1.05", &["C1.04"]),
    ("C1.06", &["C1.06"]),
    ("C1.03b", &["C1.03"]),
    ("C1.07", &["C1.07"]),
    ("C2.12", &["C2.12"]),
    ("C7.09", &["C7.09"]),
    ("C2.01", &["C2.01"]),
    ("C2.02", &["C2.02"]),
    ("C2.03", &["C2.08"]),
    ("C2.04", &["C2.07"]),
    ("C2.05", &["C2.01"]),
    ("C2.06", &["C2.03-05", "C2.06"]),
    ("C2.10", &["C2.10"]),
    ("C2.11", &["C2.11"]),
    ("C3.01", &["C3.01"]),
    ("C3.02", &["C3.02"]),
    ("C3.03", &["C3.06"]),
    ("C3.04", &[]),
    ("C3.05", &["C3.08"]),
    ("C7.01", &["C7.01"]),
    ("C7.02", &["C7.02"]),
    ("C7.08", &["C7.08"]),
    ("C9.04", &["C9.04", "B4.02"]),
    ("C11.06", &["C11.06"]),
    ("C11.06b", &["C11.06"]),
    ("C3.07", &["C3.07"]),
    ("C3.03b", &["C3.03"]),
    ("C3.09", &["C3.09"]),
    ("C13.01", &["C13.07", "C13.10"]),
    ("C13.02", &["C13.08", "C13.10"]),
    ("C13.03", &["C13.09"]),
    ("C14.01", &["C14.01"]),
    ("C14.02", &["C14.02"]),
];

/// Dossier assertions deliberately implemented by more than one cart test, with the reason.
///
/// Every entry here is a claim that the two tests assert genuinely different things about one
/// enumerated behaviour — usually because the failure modes are opposite and each deserves its own
/// failure code. Anything not listed here that is claimed twice is treated as an accidental
/// duplicate and fails the build.
pub const SPLITS: &[(&str, &str)] = &[
    (
        "C3.05",
        "the row makes two claims with different provenance and opposite failure modes. That \
         $2137 latches only while $4201 bit 7 is set is corroborated -- snes9x and Mesen2 both \
         gate it, and RustySNES was fixed to match (C3.10, scored). What the read *returns* is \
         not: snes9x presents PPU1's open-bus latch and Mesen2 the CPU's, so it is recorded \
         instead (C3.11, golden). Keeping them in one test would have forced one verdict on two \
         independent questions",
    ),
    (
        "E6.02",
        "one row for a rate — and a single reading of ENDX cannot establish a rate, only \
         \"finished\" or \"not finished\", which bounds it on one side. So each pitch is read \
         twice, at waits either side of where it finishes: E6.02/E6.02b bracket $1000 to 24-64 \
         samples per wait and E6.02c/E6.02d bracket $2000 to 64-128. Both windows contain the \
         documented rate and the two do not overlap, which is the increase; none of the four means \
         anything alone, and the pair-of-pairs is what the row is worth",
    ),
    (
        "D1.01",
        "the dossier states \"transfer modes 0-7, one test each\" as a single row, so it is a \
         range in all but name. Cart D1.01 covers mode 0 (every byte to one register) and D1.01b \
         mode 1 (alternating between two) — the pair is what makes either meaningful, since a \
         core that confuses the two still writes the right bytes to the wrong places",
    ),
    (
        "D1.07",
        "one row for three address-step behaviours that share a two-bit field (0 = increment, \
         1 = fixed, 2 = decrement, 3 = fixed). Cart D1.07 asserts FIXED and D1.07b DECREMENT; a \
         core that reads the field as two independent flags gets exactly one of them wrong and \
         the other right, which either test alone would miss",
    ),
    (
        "A5.01-08",
        "the opcode cycle sweep (T-04-I). The dossier states the base sweep as a single ranged \
         assertion covering all 256 opcodes; the cart implements it as one test per opcode so a \
         failure names the instruction rather than the batch. Every A5.Sxx test is one row of it",
    ),
    (
        "A1.01",
        "cart A1.01 asserts XCE clears XH/YH; cart A1.03 asserts it forces SH=$01. \
         One dossier line, two independent register effects",
    ),
    (
        "A6.01",
        "cart A6.01 covers the native BRK vector, cart A6.02 the separate COP vector. \
         The dossier lists the whole native vector table as one assertion",
    ),
    (
        "A8.02",
        "cart A8.01 asserts the terminal A=$FFFF, cart A8.02 the permanent DBR=destination. \
         A core can get either right and the other wrong",
    ),
    (
        "C1.02",
        "cart C1.01 asserts the word commits low byte first, cart C1.02 that an odd trailing \
         byte stays in the latch. Commit order and commit trigger are separate bugs",
    ),
    (
        "C2.01",
        "cart C2.01 covers step 1, cart C2.05 the 32/128/128 steps — including that both \
         encodings of 128 mean 128, which is its own trap",
    ),
    (
        "C11.06",
        "cart C11.06 asserts the product magnitude and that M7B's low byte cannot leak in; \
         cart C11.06b asserts signedness on both operands and the 24-bit sign extension",
    ),
    (
        "C13.10",
        "cart C13.01 asserts $213E bit 4 tracks PPU1's latch, cart C13.02 that $213F bit 5 \
         tracks PPU2's. The dossier states both in one line",
    ),
];

/// Tests that implement no enumerated dossier assertion, with the reason each is legitimate.
pub const UNENUMERATED: &[(&str, &str)] = &[
    (
        "B4.16",
        "The before/after guard for T-06-A. It records where an H-IRQ fires at an HTIME below \
         the long dots and one above, because nothing else covers raster-IRQ position -- so the \
         dot-model change would pass its own acceptance criteria while shifting every H-IRQ. It \
         implements no enumerated assertion of its own; B4.07 and B4.14 own the H-IRQ rows",
    ),
    (
        "E3.11c",
        "DSP global-register addressing. The companion to E3.11b: the global block is decoded \
         from the same latch by a different part of the address, so a core that gets the voice \
         registers right and aliases the globals passes one and fails the other",
    ),
    (
        "E3.11b",
        "DSP register addressing through the $F2/$F3 latch. Not an enumerated assertion of its \
         own: it is the mechanism every other DSP assertion is reached through, so a core that \
         mis-decodes it makes those tests meaningless rather than failing",
    ),
    (
        "A9.02",
        "XBA's flag behaviour. A9 enumerates BIT and ORA [d] but not XBA, so there is no \
         assertion to cite — the behaviour is from the WDC datasheet",
    ),
    (
        "A9.03",
        "the emulation-mode R-M-W modify-cycle write. Not an enumerated assertion — it comes from \
         the cross-vendor comparison in docs/accuracysnes-timing-oracle.md §8, where WDC's note \
         (17) stands alone against two silent renderings",
    ),
    (
        "B5.03",
        "divide by zero saturating to $FFFF with the dividend left as the remainder. \
         Documented in fullsnes but not enumerated in B5, which covers only the two operations, \
         the undefined overlap, and the power-on state",
    ),
    (
        "C3.04",
        "that the H counter advances at all. A supporting test rather than a hardware assertion \
         — it pins the primitive every Group A and Group B cycle measurement is built on, so a \
         broken counter fails here rather than as noise in a dozen timing tests",
    ),
];

/// The dossier assertions this test implements, or an empty slice.
///
/// # Panics
/// If `id` is not in [`MAP`] — every test must be mapped, which is what keeps the map honest.
#[must_use]
pub fn for_test(id: &str) -> Vec<&'static str> {
    MAP.iter().find(|(cart, _)| *cart == id).map_or_else(
        || {
            panic!(
                "test {id} is not in the dossier map. Add it to gen/src/dossier.rs::MAP naming \
                 the assertion(s) it implements, or map it to &[] and justify it in UNENUMERATED. \
                 Do NOT assume the cart ID equals the dossier ID — they are different schemes."
            )
        },
        |(_, d)| d.to_vec(),
    )
}

/// Every measurement slot must be claimed by exactly one test.
///
/// The channel has no allocator: a slot is claimed by writing to it. Two tests choosing the same
/// number therefore overwrite each other silently, and every reader of the older one begins
/// reporting the newer one's values under the older one's labels — a wrong number with a
/// confident caption, which is worse than a missing one.
///
/// This is not hypothetical. `E3.02` was written against slots 106 and 107, which `B3.01` already
/// owned; nothing failed, and the DRAM-refresh reporter simply started printing timer counts. The
/// battery was green throughout. Hence a build error rather than a convention.
///
/// # Panics
/// If two tests write the same slot.
pub fn check_slots(tests: &[crate::dsl::Test]) {
    let mut owner: [Option<&str>; crate::dsl::MEAS_SLOTS as usize] =
        [None; crate::dsl::MEAS_SLOTS as usize];
    let mut clashes: Vec<String> = Vec::new();
    for t in tests {
        for &slot in &t.slots {
            let i = usize::from(slot);
            match owner[i] {
                Some(first) if first != t.id => {
                    clashes.push(format!("  slot {slot}: {first} and {}", t.id));
                }
                Some(_) => {}
                None => owner[i] = Some(t.id),
            }
        }
    }
    // Every clash at once. Reporting them one per build turns a five-minute fix into five builds,
    // and the free-slot list below is only correct if the whole picture is in front of you.
    assert!(
        clashes.is_empty(),
        "measurement slots are written by more than one test:\n{}\n\nThe channel has no \
         allocator, so the later writer silently overwrites the earlier one and every reader of \
         the earlier test starts reporting the later one's numbers. Free slots: {:?}",
        clashes.join("\n"),
        (0..crate::dsl::MEAS_SLOTS)
            .filter(|i| owner[usize::from(*i)].is_none())
            .collect::<Vec<_>>()
    );
}

/// Enforce the three map rules, then write the coverage report.
///
/// # Panics
/// On any unmapped test, any undeclared double-claim, or any unjustified empty mapping.
pub fn validate(tests: &[crate::dsl::Test]) {
    // Rule 1: every test is mapped. `for_test` panics with the actionable message.
    for t in tests {
        let mapped = for_test(t.id);
        // Rule 3: an empty mapping must be justified.
        assert!(
            !mapped.is_empty() || UNENUMERATED.iter().any(|(c, _)| *c == t.id),
            "test {} maps to no dossier assertion and is not justified. Add it to \
             gen/src/dossier.rs::UNENUMERATED with the reason it has nothing to cite.",
            t.id
        );
    }

    // The map must not describe tests that do not exist — a stale entry silently distorts the
    // coverage report, which is the one thing this file is for.
    for (cart, _) in MAP {
        assert!(
            tests.iter().any(|t| t.id == *cart),
            "the dossier map lists {cart}, which is not in the battery. Remove the stale entry."
        );
    }

    // Rule 2: any assertion claimed twice must be a declared split.
    let mut claims: Vec<(&str, Vec<&str>)> = Vec::new();
    for t in tests {
        for d in for_test(t.id) {
            match claims.iter_mut().find(|(a, _)| *a == d) {
                Some((_, by)) => by.push(t.id),
                None => claims.push((d, vec![t.id])),
            }
        }
    }
    for (assertion, by) in &claims {
        if by.len() > 1 {
            assert!(
                SPLITS.iter().any(|(a, _)| a == assertion),
                "dossier assertion {assertion} is claimed by {} tests ({}) but is not a declared \
                 split. Either these assert genuinely different things — in which case add \
                 {assertion} to gen/src/dossier.rs::SPLITS with the reason — or one of them is a \
                 DUPLICATE and should be deleted. This check exists because four duplicates were \
                 written and very nearly shipped.",
                by.len(),
                by.join(", ")
            );
        }
    }

    // Declared splits that are no longer split are stale in the other direction.
    for (assertion, _) in SPLITS {
        let n = claims
            .iter()
            .find(|(a, _)| a == assertion)
            .map_or(0, |(_, by)| by.len());
        assert!(
            n > 1,
            "SPLITS declares {assertion} as split across multiple tests, but {n} test(s) claim \
             it. Remove the stale SPLITS entry."
        );
    }
}

/// Every rendered scene must name assertions that actually exist in the dossier.
///
/// The same gate the battery gets, for the same reason: a typo in a scene's `dossier` field would
/// silently claim coverage of nothing. That is worse here than in a test, because a scene has no
/// on-cart verdict to look wrong — the only signal would be a coverage number quietly one too high.
///
/// # Panics
/// If a scene names an assertion the dossier does not enumerate, or names none at all.
pub fn validate_scenes(enumerated: &[(String, Vec<String>)]) {
    let known: std::collections::BTreeSet<&str> = enumerated
        .iter()
        .flat_map(|(_, ids)| ids.iter().map(String::as_str))
        .collect();

    let mut bad = Vec::new();
    for sc in crate::scenes::SCENES {
        // Empty fragments are dropped rather than reported: a trailing comma is a typo in the
        // separator, not a claim about an assertion, and reporting it as `'' is not an enumerated
        // assertion` buries the real question of whether the scene names anything at all.
        let ids: Vec<&str> = sc
            .dossier
            .split(',')
            .map(str::trim)
            .filter(|d| !d.is_empty())
            .collect();
        if ids.is_empty() {
            bad.push(format!("{}: names no assertion", sc.id));
            continue;
        }
        for d in ids {
            if !known.contains(d) {
                bad.push(format!("{}: '{d}' is not an enumerated assertion", sc.id));
            }
        }
    }
    assert!(
        bad.is_empty(),
        "rendered scenes claim assertions the dossier does not enumerate:\n  {}",
        bad.join("\n  ")
    );
}

/// Render `docs/accuracysnes-coverage.md` — which enumerated assertions have tests and which do
/// not, so coverage is a query rather than a guess.
#[must_use]
#[allow(
    clippy::too_many_lines,
    reason = "one report, written top to bottom in output order"
)]
pub fn coverage_report(tests: &[crate::dsl::Test], enumerated: &[(String, Vec<String>)]) -> String {
    /// Assertions covered by a rendered scene rather than by an on-cart test.
    ///
    /// Kept separate throughout, because the two are not the same kind of evidence: a scored test
    /// means the same thing on any emulator and on real hardware, while a rendered scene needs a
    /// host holding the golden. Folding them into one number would quietly redefine what the
    /// headline figure claims — the same reasoning ADR 0013 uses to keep scenes out of the pass
    /// rate. They are reported side by side and totalled separately.
    /// **Only blessed scenes count.** ADR 0013 rule 4 says an unblessed scene "is not yet evidence
    /// of anything" — it renders, and nothing has confirmed the picture is right. Counting one as
    /// coverage would have let a scene claim an assertion by existing, which is exactly the gap
    /// this report is supposed to close for the battery. The blessing list is the golden file, read
    /// here rather than trusted from memory.
    fn scene_assertions() -> Vec<String> {
        // Read, not `unwrap_or_default`: an unreadable golden file would silently report every
        // scene as unblessed and undercount coverage with no failure anywhere, which is the same
        // class of quiet wrongness this whole report exists to prevent. Every other required input
        // in this generator panics on a read error too.
        let path = crate::cart_root().join("../../golden/accuracysnes-scenes.tsv");
        let goldens = std::fs::read_to_string(&path)
            .unwrap_or_else(|e| panic!("read {}: {e}", path.display()));
        let blessed: std::collections::HashSet<&str> = goldens
            .lines()
            .filter(|l| !l.trim_start().starts_with('#') && !l.trim().is_empty())
            .filter_map(|l| l.split('\t').next())
            .collect();
        crate::scenes::SCENES
            .iter()
            .filter(|sc| blessed.contains(sc.id))
            .flat_map(|sc| sc.dossier.split(','))
            .map(|d| d.trim().to_string())
            .collect()
    }

    use core::fmt::Write as _;
    let mut s = String::new();
    let _ = writeln!(s, "# AccuracySNES — dossier coverage\n");
    let _ = writeln!(
        s,
        "GENERATED by `accuracysnes-gen` alongside the ROM — do not edit by hand.\n"
    );
    let _ = writeln!(
        s,
        "Maps `docs/accuracysnes-research-dossier.md` §5 assertions to the cart tests that \
         implement them (`tests/roms/AccuracySNES/gen/src/dossier.rs`). **Cart IDs and dossier \
         IDs are different numbering schemes** — cart `A1.04` is dossier `A1.06` — so this table, \
         not the ID numbers, is what says whether something is covered.\n"
    );
    let _ = writeln!(
        s,
        "Every sub-group of Part V is enumerated, so this is a **complete** statement of coverage \
         — if an assertion has no test, it is listed here. Rows carrying a range (`A5.01-08`, \
         `C2.03-05`, `D2.11-14`) stand for several assertions each, so the assertion total is \
         slightly higher than the row total.\n"
    );

    let scenes = scene_assertions();
    let mut covered_total = 0usize;
    let mut scene_total = 0usize;
    let mut all_total = 0usize;
    let _ = writeln!(
        s,
        "| Sub-group | Enumerated | Covered (on-cart) | Covered (scene) | Uncovered |"
    );
    let _ = writeln!(s, "|---|---:|---:|---:|---|");
    for (sub, ids) in enumerated {
        let mut uncovered = Vec::new();
        let mut covered = 0usize;
        let mut by_scene = 0usize;
        for id in ids {
            if tests.iter().any(|t| for_test(t.id).iter().any(|d| d == id)) {
                covered += 1;
            } else if scenes.iter().any(|d| d == id) {
                by_scene += 1;
            } else {
                uncovered.push(id.clone());
            }
        }
        covered_total += covered;
        scene_total += by_scene;
        all_total += ids.len();
        let list = if uncovered.is_empty() {
            "—".to_string()
        } else {
            uncovered.join(", ")
        };
        let _ = writeln!(
            s,
            "| `{sub}` | {} | {covered} | {by_scene} | {list} |",
            ids.len()
        );
    }
    let _ = writeln!(
        s,
        "\n**{covered_total} of {all_total}** enumerated assertion rows covered by an on-cart \
         test, plus **{scene_total}** covered only by a rendered scene \
         (`docs/adr/0013`) — **{} of {all_total}** in total.\n",
        covered_total + scene_total
    );
    let _ = writeln!(
        s,
        "The two columns are kept apart on purpose. An on-cart result means the same thing on any \
         emulator and on real hardware; a rendered scene needs a host holding the golden. Adding \
         them into one figure would quietly change what the number claims.\n"
    );

    let _ = writeln!(s, "## Assertions split across several tests\n");
    let _ = writeln!(
        s,
        "Declared in `dossier.rs::SPLITS`. Each is a claim that the tests assert different things \
         about one enumerated behaviour; an undeclared double-claim fails the build.\n"
    );
    for (assertion, why) in SPLITS {
        let by: Vec<&str> = tests
            .iter()
            .filter(|t| for_test(t.id).iter().any(|d| d == assertion))
            .map(|t| t.id)
            .collect();
        let _ = writeln!(s, "- **`{assertion}`** — {} · {why}", by.join(", "));
    }

    let _ = writeln!(s, "\n## Assertions covered by a rendered scene\n");
    let _ = writeln!(
        s,
        "Declared in `gen/src/scenes.rs`. Each is reported by the host framebuffer oracle against \
         a cross-validated golden, never scored on-cart (`docs/adr/0013`).\n"
    );
    for sc in crate::scenes::SCENES {
        let _ = writeln!(s, "- **`{}`** — {}", sc.dossier, sc.id);
    }

    let _ = writeln!(s, "\n## Tests with no enumerated assertion\n");
    for (cart, why) in UNENUMERATED {
        let _ = writeln!(s, "- **`{cart}`** — {why}");
    }
    s
}

/// Parse the per-ID assertion tables out of the dossier's Part V.
///
/// Returns `(sub-group, ids)` for every sub-group that enumerates its assertions in a table.
/// Prose sub-groups are absent by construction — there is nothing to parse — which is why the
/// coverage report can only speak for part of the enumeration.
///
/// # Panics
/// If the dossier is missing its Part V markers, since a silently-empty parse would report
/// everything as uncovered.
#[must_use]
pub fn parse_enumeration(dossier: &str) -> Vec<(String, Vec<String>)> {
    let start = dossier
        .find("## Part V —")
        .expect("dossier is missing its Part V heading");
    let end = dossier
        .find("## Part VI")
        .expect("dossier is missing its Part VI heading");
    let part = &dossier[start..end];

    let mut out: Vec<(String, Vec<String>)> = Vec::new();
    for line in part.lines() {
        let trimmed = line.trim_start();
        let Some(rest) = trimmed.strip_prefix('|') else {
            continue;
        };
        let cell = rest.split('|').next().unwrap_or("").trim();
        if !is_assertion_id(cell) {
            continue;
        }
        let key = split_sub(cell);
        match out.iter_mut().find(|(s, _)| *s == key) {
            Some((_, ids)) => ids.push(cell.to_string()),
            None => out.push((key, vec![cell.to_string()])),
        }
    }
    out
}

/// The sub-group part of an assertion ID: `A5.11` -> `A5`, `C13.07` -> `C13`.
fn split_sub(id: &str) -> String {
    id.split('.').next().unwrap_or(id).to_string()
}

/// Whether a table cell looks like an assertion ID (`A1.01`, `C2.03-05`, `A5.01-08`).
fn is_assertion_id(cell: &str) -> bool {
    let mut chars = cell.chars();
    let Some(g) = chars.next() else { return false };
    if !('A'..='G').contains(&g) {
        return false;
    }
    let rest: String = chars.collect();
    let Some((sub, num)) = rest.split_once('.') else {
        return false;
    };
    !sub.is_empty()
        && sub.chars().all(|c| c.is_ascii_digit())
        && !num.is_empty()
        && num.chars().all(|c| c.is_ascii_digit() || c == '-')
}
