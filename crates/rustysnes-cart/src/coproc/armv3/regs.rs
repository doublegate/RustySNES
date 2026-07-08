//! The ARM register file (with mode banking) and the 3-stage instruction pipeline.
//!
//! Clean-room port of the relevant pieces of Mesen2's `ArmV3Cpu`/`ArmV3CpuState` (see the parent
//! module doc). This is step 2+3 of the build order in `docs/st018-arm-notes.md`: register
//! banking and the pipeline's implicit "PC reads as address+8" timing, both landed before any
//! instruction decode/execute exists (nothing here depends on it, and the pipeline model is the
//! single highest-risk fidelity point every later instruction depends on getting right).

use crate::coproc::armv3::primitives::Flags;

/// The 7 real ARM processor modes.
///
/// A `u8`, not a Rust enum, because [`Regs::switch_mode`] deliberately mirrors the source's own
/// `default:`-falls-back-to-User/System behavior for any OTHER 5-bit pattern reachable via `MSR`
/// — a real ARM CPU accepts an out-of-range mode field as "reserved/UNPREDICTABLE," and this
/// port's fallback (treat it like User/System for banking purposes) matches Mesen2's own choice
/// rather than rejecting it outright.
pub mod mode {
    /// Bit 4 is always set on every real mode value; ORed into `switch_mode`'s input
    /// unconditionally (matching the source), so a caller never needs to set it explicitly.
    pub const BIT: u8 = 0b1_0000;
    /// Unprivileged mode — the normal running state; no SPSR, no banked `R13`/`R14` of its own
    /// (shares the same bank as [`SYSTEM`]).
    pub const USER: u8 = 0b1_0000;
    /// Fast interrupt mode — the only mode with a fully private `R8-R14` bank.
    pub const FIQ: u8 = 0b1_0001;
    /// Normal interrupt mode.
    pub const IRQ: u8 = 0b1_0010;
    /// Entered on `SWI` (software interrupt).
    pub const SUPERVISOR: u8 = 0b1_0011;
    /// Entered on a memory abort.
    pub const ABORT: u8 = 0b1_0111;
    /// Entered on an undefined-instruction trap.
    pub const UNDEFINED: u8 = 0b1_1011;
    /// Privileged mode that shares [`USER`]'s register bank (no SPSR of its own either).
    pub const SYSTEM: u8 = 0b1_1111;
}

/// A `u8`-backed ARM mode value — see the [`mode`] module for the 7 real constants and the
/// fallback-to-User/System posture for anything else.
pub type Mode = u8;

/// The full CPSR (or an SPSR): mode + interrupt-mask bits + the [`Flags`] condition codes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Cpsr {
    /// The active processor mode (see the [`mode`] module).
    pub mode: Mode,
    /// FIQ (fast-interrupt) mask — set to disable FIQ.
    pub fiq_disable: bool,
    /// IRQ mask — set to disable normal interrupts.
    pub irq_disable: bool,
    /// The N/Z/C/V condition-code flags.
    pub flags: Flags,
}

impl Default for Cpsr {
    fn default() -> Self {
        Self {
            mode: mode::USER,
            fiq_disable: false,
            irq_disable: false,
            flags: Flags::default(),
        }
    }
}

impl Cpsr {
    /// Pack into the 32-bit CPSR/SPSR register layout (Mesen2 `ArmV3CpuFlags::ToInt32`) — used by
    /// `MRS` and the (not-yet-ported) exception-entry path.
    #[must_use]
    pub const fn to_u32(self) -> u32 {
        ((self.flags.n as u32) << 31)
            | ((self.flags.z as u32) << 30)
            | ((self.flags.c as u32) << 29)
            | ((self.flags.v as u32) << 28)
            | ((self.irq_disable as u32) << 7)
            | ((self.fiq_disable as u32) << 6)
            | (self.mode as u32)
    }
}

/// The ARM register file: `R0-R15` plus every mode's banked registers and SPSR.
///
/// Banking follows real ARM hardware (and Mesen2's `SwitchMode`) exactly: `R8-R12` are shared by
/// every mode EXCEPT FIQ (which gets its own private `R8-R12`); `R13`/`R14` are banked separately
/// per mode, INCLUDING a distinct "User" bank from the four privileged non-FIQ modes' banks. This
/// project's other CPU cores don't need this pattern (the 65C816 and SA-1 have no register
/// banking), so it has no existing precedent to crib the shape from — ported straight from the
/// reference `memcpy`-based save/restore sequence as explicit slice copies.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Regs {
    /// `R0-R15` (R15 = PC) — the currently-banked-in view every instruction reads/writes.
    pub r: [u32; 16],
    /// The live CPSR (current mode + interrupt masks + condition flags).
    pub cpsr: Cpsr,

    /// `R8-R14` (7 slots) for User/System mode; ALSO the shared source of `R8-R12` (only the
    /// first 5 slots) when banking into/out of Irq/Supervisor/Abort/Undefined — see
    /// [`Self::switch_mode`]'s doc for exactly how the two roles interact.
    user_bank: [u32; 7],
    /// `R8-R14` (7 slots) for FIQ mode — the only mode with its own private `R8-R12`.
    fiq_bank: [u32; 7],
    /// `R13-R14` (2 slots) private to IRQ.
    irq_bank: [u32; 2],
    /// `R13-R14` (2 slots) private to Supervisor.
    svc_bank: [u32; 2],
    /// `R13-R14` (2 slots) private to Abort.
    abt_bank: [u32; 2],
    /// `R13-R14` (2 slots) private to Undefined.
    und_bank: [u32; 2],

    fiq_spsr: Cpsr,
    irq_spsr: Cpsr,
    svc_spsr: Cpsr,
    abt_spsr: Cpsr,
    und_spsr: Cpsr,
}

impl Regs {
    /// Switch the active processor mode, banking `R8-R14` in/out exactly like real ARM hardware
    /// (Mesen2 `SwitchMode`).
    ///
    /// `new_mode` is OR'd with [`mode::BIT`] unconditionally (bit 4 is always set on real
    /// hardware — every caller, including `MSR`'s raw 5-bit mode field, relies on this rather
    /// than validating it themselves). A no-op if the mode is already active (matches the
    /// source's early-return, and avoids a redundant full 7-register bank round-trip on every
    /// same-mode `MSR`).
    pub fn switch_mode(&mut self, new_mode: Mode) {
        let new_mode = new_mode | mode::BIT;
        if self.cpsr.mode == new_mode {
            return;
        }
        let org_mode = self.cpsr.mode;

        // Save the OUTGOING mode's banked registers. FIQ banks all 7 (R8-R14) into its own
        // array; the four other privileged modes bank only their private R13-R14 (2) into their
        // own array, and ALSO refresh the shared `user_bank[0..5]` (R8-R12) — since those 5 are
        // visible from every non-FIQ mode, not just User/System. Leaving User/System (or any
        // unrecognized mode, matching the source's `default:` fallback) banks the full 7 into
        // `user_bank`, INCLUDING User mode's own private R13-R14 in slots 5-6.
        match org_mode {
            mode::FIQ => self.fiq_bank.copy_from_slice(&self.r[8..15]),
            mode::IRQ => {
                self.user_bank[0..5].copy_from_slice(&self.r[8..13]);
                self.irq_bank.copy_from_slice(&self.r[13..15]);
            }
            mode::SUPERVISOR => {
                self.user_bank[0..5].copy_from_slice(&self.r[8..13]);
                self.svc_bank.copy_from_slice(&self.r[13..15]);
            }
            mode::ABORT => {
                self.user_bank[0..5].copy_from_slice(&self.r[8..13]);
                self.abt_bank.copy_from_slice(&self.r[13..15]);
            }
            mode::UNDEFINED => {
                self.user_bank[0..5].copy_from_slice(&self.r[8..13]);
                self.und_bank.copy_from_slice(&self.r[13..15]);
            }
            _ => self.user_bank.copy_from_slice(&self.r[8..15]),
        }

        self.cpsr.mode = new_mode;

        // Load the INCOMING mode's banked registers, the mirror image of the save above.
        if new_mode == mode::FIQ {
            self.r[8..15].copy_from_slice(&self.fiq_bank);
        } else {
            self.r[8..15].copy_from_slice(&self.user_bank);
            match new_mode {
                mode::IRQ => self.r[13..15].copy_from_slice(&self.irq_bank),
                mode::SUPERVISOR => self.r[13..15].copy_from_slice(&self.svc_bank),
                mode::ABORT => self.r[13..15].copy_from_slice(&self.abt_bank),
                mode::UNDEFINED => self.r[13..15].copy_from_slice(&self.und_bank),
                _ => {}
            }
        }
    }

    /// The current mode's SPSR (Mesen2 `GetSpsr`). User/System (and any unrecognized mode,
    /// matching the source's `default:` fallback) have no real SPSR; reading/writing it there is
    /// architecturally UNPREDICTABLE, and this port follows the source's own choice of aliasing
    /// the live CPSR itself as a harmless, safe fallback rather than inventing new behavior.
    #[must_use]
    pub const fn spsr(&self) -> Cpsr {
        match self.cpsr.mode {
            mode::FIQ => self.fiq_spsr,
            mode::IRQ => self.irq_spsr,
            mode::SUPERVISOR => self.svc_spsr,
            mode::ABORT => self.abt_spsr,
            mode::UNDEFINED => self.und_spsr,
            _ => self.cpsr,
        }
    }

    /// Mutable access to the current mode's SPSR — see [`Self::spsr`].
    pub const fn spsr_mut(&mut self) -> &mut Cpsr {
        match self.cpsr.mode {
            mode::FIQ => &mut self.fiq_spsr,
            mode::IRQ => &mut self.irq_spsr,
            mode::SUPERVISOR => &mut self.svc_spsr,
            mode::ABORT => &mut self.abt_spsr,
            mode::UNDEFINED => &mut self.und_spsr,
            _ => &mut self.cpsr,
        }
    }
}

/// One fetched-but-not-yet-executed instruction word, tagged with the address it was fetched
/// from (Mesen2 `ArmV3InstructionData`).
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Insn {
    /// The byte address this word was fetched from.
    pub address: u32,
    /// The raw 32-bit instruction word.
    pub opcode: u32,
}

/// The 3-stage Fetch/Decode/Execute pipeline (Mesen2 `ArmV3CpuPipeline`).
///
/// This is the ENTIRE mechanism behind ARM's well-known "PC reads as address+8" quirk: `r15`
/// (owned by [`Regs`], threaded through [`Self::process`] rather than duplicated here) tracks the
/// FETCH stage's address, two stages ahead of whatever instruction is currently in Execute. No
/// `+8` constant exists anywhere in this port — it falls straight out of this stage timing, which
/// is exactly why getting this model right BEFORE porting any instruction that reads R15 as an
/// operand matters (see `docs/st018-arm-notes.md`'s pipeline section).
///
/// Bus-side access-mode bits (`Sequential`/`Word`/`Byte`/`Prefetch`, used by the board wrapper for
/// its own cycle-timing bookkeeping) are deliberately NOT modeled here yet — they don't affect
/// address/opcode sequencing, only the caller's `read_code` timing side effects, and land with
/// the board wrapper (`docs/st018-arm-notes.md` step 9).
#[derive(Debug, Clone, Copy, Default)]
pub struct Pipeline {
    /// The most recently fetched, not-yet-decoded word.
    pub fetch: Insn,
    /// The word decoded on the prior step, about to become `execute` on the next one.
    pub decode: Insn,
    /// The instruction actually running on the current step.
    pub execute: Insn,
    reload_requested: bool,
}

impl Pipeline {
    /// Request a full pipeline flush + refill on the next [`Self::process`] call — set whenever
    /// an instruction writes R15 (a taken branch, a data-processing `MOV PC, ...`, an `LDR PC,
    /// ...`, an exception entry, etc.). Not yet wired to any register-write path (that lands with
    /// instruction execute); exposed now so the pipeline model itself is independently testable.
    pub const fn request_reload(&mut self) {
        self.reload_requested = true;
    }

    /// Word-align `r15`, then fetch twice — landing the branch target in Decode and the
    /// instruction after it in Fetch (Mesen2 `ReloadPipeline`). Leaves `execute` holding whatever
    /// was in `decode` before the reload (stale, discarded); [`Self::process`]'s own unconditional
    /// shift immediately after this call promotes the real target into `execute` before anything
    /// reads it, so the staleness is never observable.
    fn reload(&mut self, r15: &mut u32, read_code: &mut impl FnMut(u32) -> u32) {
        self.reload_requested = false;
        *r15 &= !0x3;
        self.fetch = Insn {
            address: *r15,
            opcode: read_code(*r15),
        };
        self.execute = self.decode;
        self.decode = self.fetch;
        *r15 = r15.wrapping_add(4);
        self.fetch = Insn {
            address: *r15,
            opcode: read_code(*r15),
        };
    }

    /// Advance the pipeline by one stage (Mesen2 `ProcessPipeline`): reload first if requested,
    /// then unconditionally shift Execute←Decode←Fetch←(a fresh fetch at `r15+4`). Call this once
    /// per CPU step, AFTER the current Execute-stage instruction has run.
    pub fn process(&mut self, r15: &mut u32, mut read_code: impl FnMut(u32) -> u32) {
        if self.reload_requested {
            self.reload(r15, &mut read_code);
        }
        self.execute = self.decode;
        self.decode = self.fetch;
        *r15 = r15.wrapping_add(4);
        self.fetch = Insn {
            address: *r15,
            opcode: read_code(*r15),
        };
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn r8_to_r12_are_shared_across_every_non_fiq_mode() {
        let mut regs = Regs::default();
        regs.switch_mode(mode::IRQ);
        regs.r[8] = 0xAAAA_AAAA;
        regs.switch_mode(mode::SUPERVISOR);
        // R8 must still read the value written under IRQ -- R8-R12 are the SAME physical
        // registers in every non-FIQ mode, not independently banked per mode.
        assert_eq!(regs.r[8], 0xAAAA_AAAA);
        regs.switch_mode(mode::ABORT);
        assert_eq!(regs.r[8], 0xAAAA_AAAA);
        regs.switch_mode(mode::UNDEFINED);
        assert_eq!(regs.r[8], 0xAAAA_AAAA);
        regs.switch_mode(mode::USER);
        assert_eq!(regs.r[8], 0xAAAA_AAAA);
    }

    #[test]
    fn fiq_has_its_own_private_r8_to_r12() {
        let mut regs = Regs::default();
        regs.r[8] = 0x1111_1111; // written while in the power-on default (User) mode
        regs.switch_mode(mode::FIQ);
        // FIQ's R8 is a DIFFERENT physical register -- starts at the power-on default (0), not
        // the User-mode value just written.
        assert_eq!(regs.r[8], 0);
        regs.r[8] = 0x2222_2222;
        regs.switch_mode(mode::USER);
        // Back in User mode, R8 reads the ORIGINAL User-mode value -- FIQ's write didn't leak.
        assert_eq!(regs.r[8], 0x1111_1111);
    }

    #[test]
    fn r13_r14_are_privately_banked_per_mode_including_a_distinct_user_bank() {
        let mut regs = Regs::default();
        regs.r[13] = 0x1000; // User mode's own SP
        regs.r[14] = 0x1004;

        regs.switch_mode(mode::IRQ);
        regs.r[13] = 0x2000;
        regs.r[14] = 0x2004;

        regs.switch_mode(mode::SUPERVISOR);
        // Supervisor gets its OWN R13/R14 bank, starting fresh (0), not IRQ's values.
        assert_eq!(regs.r[13], 0);
        assert_eq!(regs.r[14], 0);
        regs.r[13] = 0x3000;
        regs.r[14] = 0x3004;

        regs.switch_mode(mode::IRQ);
        assert_eq!(regs.r[13], 0x2000, "IRQ's own bank must round-trip");
        assert_eq!(regs.r[14], 0x2004);

        regs.switch_mode(mode::USER);
        assert_eq!(
            regs.r[13], 0x1000,
            "User mode's bank is distinct from every privileged mode's"
        );
        assert_eq!(regs.r[14], 0x1004);

        regs.switch_mode(mode::SUPERVISOR);
        assert_eq!(
            regs.r[13], 0x3000,
            "Supervisor's own bank must also round-trip"
        );
        assert_eq!(regs.r[14], 0x3004);
    }

    #[test]
    fn system_mode_shares_the_user_bank() {
        // System mode uses the SAME R13/R14 bank as User (both are `_ => user_bank` in the
        // source's switch); confirm a value written under User is visible under System.
        let mut regs = Regs::default();
        regs.r[13] = 0x5000;
        regs.switch_mode(mode::SYSTEM);
        assert_eq!(regs.r[13], 0x5000);
    }

    #[test]
    fn switching_to_the_same_mode_is_a_no_op() {
        let mut regs = Regs::default();
        regs.switch_mode(mode::IRQ);
        regs.r[13] = 0xDEAD_BEEF;
        regs.switch_mode(mode::IRQ); // already active -- must not clobber via a spurious bank swap
        assert_eq!(regs.r[13], 0xDEAD_BEEF);
    }

    #[test]
    fn mode_bit_4_is_always_forced_set() {
        let mut regs = Regs::default();
        // Pass a raw mode value with bit 4 clear -- switch_mode must OR it in, matching every
        // real mode constant, per the source's unconditional `mode | 0x10`.
        regs.switch_mode(0b0001);
        assert_eq!(regs.cpsr.mode, mode::FIQ);
    }

    #[test]
    fn unrecognized_mode_values_bank_like_user_system() {
        // A raw 5-bit pattern matching none of the 7 real modes (e.g. 0b10100) is architecturally
        // reserved/UNPREDICTABLE on real hardware; this port's fallback treats it like User/System
        // for banking purposes, matching the source's own `default:` case.
        let mut regs = Regs::default();
        regs.r[13] = 0x9000;
        regs.switch_mode(0b10100);
        assert_eq!(regs.r[13], 0x9000, "falls back to the User/System bank");
    }

    #[test]
    fn spsr_routes_to_the_current_privileged_modes_own_register() {
        let mut regs = Regs::default();
        regs.switch_mode(mode::IRQ);
        regs.spsr_mut().mode = mode::SUPERVISOR;
        regs.switch_mode(mode::SUPERVISOR);
        // A DIFFERENT physical SPSR from IRQ's -- starts at the power-on default.
        assert_eq!(regs.spsr().mode, mode::USER);
        regs.switch_mode(mode::IRQ);
        assert_eq!(
            regs.spsr().mode,
            mode::SUPERVISOR,
            "IRQ's SPSR must round-trip"
        );
    }

    #[test]
    fn spsr_in_user_or_system_mode_aliases_the_live_cpsr() {
        // User/System have no real SPSR; the source's fallback aliases CPSR itself.
        let mut regs = Regs::default();
        regs.cpsr.flags.n = true;
        assert!(regs.spsr().flags.n);
    }

    #[test]
    fn pipeline_reload_lands_the_branch_target_in_execute_after_one_process_call() {
        // `process` calls `reload` (which lands the target in Decode via its own internal
        // shift), then IMMEDIATELY does its own unconditional shift on top -- so by the time
        // `process` returns, the target has already been promoted one stage further, into
        // Execute, ready to run on the very next `Exec` call. Decode/Fetch sit one/two words past
        // it, exactly the normal steady-state spacing.
        let mut pipeline = Pipeline::default();
        let mut r15 = 0x1000u32;
        pipeline.request_reload();
        pipeline.process(&mut r15, |addr| addr);

        assert_eq!(pipeline.execute.address, 0x1000, "the reload target itself");
        assert_eq!(pipeline.decode.address, 0x1004);
        assert_eq!(pipeline.fetch.address, 0x1008);
        assert_eq!(
            r15, 0x1008,
            "r15 tracks the Fetch stage, execute.address + 8"
        );
    }

    #[test]
    fn pc_plus_8_falls_out_of_pipeline_timing_with_no_explicit_constant() {
        // Reproduce power-on (Mesen2 PowerOn -> ProcessPipeline with ReloadRequested already set)
        // starting at address 0, then confirm the invariant holds continuously across further
        // steps: whenever an instruction sits in Execute, r15 == that instruction's own address + 8.
        let mut pipeline = Pipeline::default();
        let mut r15 = 0u32;
        pipeline.request_reload();
        pipeline.process(&mut r15, |addr| addr);

        assert_eq!(pipeline.execute.address, 0);
        assert_eq!(
            r15, 8,
            "r15 == execute.address + 8, immediately after power-on"
        );

        // Advance further WITHOUT another reload (normal sequential execution) -- the invariant
        // must keep holding for every subsequent instruction that reaches Execute.
        pipeline.process(&mut r15, |addr| addr);
        assert_eq!(pipeline.execute.address, 4);
        assert_eq!(
            r15, 12,
            "r15 == execute.address + 8 again, one instruction later"
        );

        pipeline.process(&mut r15, |addr| addr);
        assert_eq!(pipeline.execute.address, 8);
        assert_eq!(r15, 16);
    }

    #[test]
    fn a_taken_branch_forces_a_full_refetch_not_an_incremental_step() {
        let mut pipeline = Pipeline::default();
        let mut r15 = 0u32;
        pipeline.request_reload();
        pipeline.process(&mut r15, |addr| addr); // power-on refill

        // Simulate the branch instruction at Execute (address 0) jumping to 0x8000.
        pipeline.request_reload();
        let mut branch_target = 0x8000u32;
        pipeline.process(&mut branch_target, |addr| addr);

        assert_eq!(
            pipeline.execute.address, 0x8000,
            "the branch target is in Execute the very next step, not delayed further"
        );
        assert_eq!(
            branch_target, 0x8008,
            "r15 re-establishes +8 at the new target"
        );
    }

    #[test]
    fn fetch_reload_alignment_masks_to_a_word_boundary() {
        let mut pipeline = Pipeline::default();
        let mut r15 = 0x1003u32; // deliberately misaligned
        pipeline.request_reload();
        pipeline.process(&mut r15, |addr| addr);
        assert_eq!(
            pipeline.execute.address, 0x1000,
            "misaligned reload target is word-aligned"
        );
    }
}
