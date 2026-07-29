//! DSP-1 continuous-mode Raster regression: the per-scanline Mode-7 matrix must RAMP.
//!
//! Pins the fix for the long-standing DSP-1 Mode-7 flat-floor bug (Pilotwings flight, SMK track).
//! Root cause: `Upd77c25::run_until_rqm` stopped at the first `RQM=set` — which a host-input read
//! (`src == 8`) raises as a side effect — *before* the firmware finished dispatching the command and
//! clearing DRC to 16-bit for the parameter transfer. The next host write then used the stale 8-bit
//! framing, so the whole parameter block landed misaligned and the DSP computed a degenerate
//! (constant-across-scanlines) projection → a flat Mode-7 ground. The fix runs the engine to its
//! genuine host-wait spin, matching ares' continuous stepping.
//!
//! This replays the exact command sequence Pilotwings issues for a flight frame — cmd `0x02`
//! (parameter) with real captured flight params, then cmd `0x0a` (continuous raster) — against a
//! fresh core, and asserts consecutive raster cycles differ (a real perspective floor). With the
//! pre-fix `run_until_rqm` every cycle is identical (`fe7f 01ff fe00 fe7f`), so this fails.
//!
//! Self-skips when the DSP-1 firmware dump is absent (it is gitignored), keeping fresh clones green.
#![cfg(feature = "test-roms")]
use rustysnes_core::cart::{Revision, Upd77c25};
use std::path::PathBuf;

fn firmware() -> Option<Vec<u8>> {
    let dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../tests/roms/external/firmware");
    for n in ["dsp1b.rom", "dsp1.rom"] {
        if let Ok(b) = std::fs::read(dir.join(n)) {
            return Some(b);
        }
    }
    None
}

#[test]
fn dsp1_continuous_raster_ramps_per_scanline() {
    let Some(fw) = firmware() else {
        eprintln!("SKIP: DSP-1 firmware (dsp1b.rom) absent");
        return;
    };
    let mut dsp = Upd77c25::new(Revision::Upd7725);
    assert!(dsp.load_firmware(&fw), "firmware loaded");

    // Host command byte (8-bit while parked) then 16-bit LE parameter words. In the pin-exact model
    // read_dr/write_dr no longer advance the DSP (only the master-clock tick does), so this standalone
    // harness — which has no bus — drives the chip explicitly with `run_until_rqm` after each host op,
    // exactly the catch-up the Bus performs continuously in-game.
    let cmd = |dsp: &mut Upd77c25, c: u8, params: &[u16]| {
        dsp.write_dr(c);
        dsp.run_until_rqm();
        for &p in params {
            dsp.write_dr((p & 0xff) as u8);
            dsp.run_until_rqm();
            dsp.write_dr((p >> 8) as u8);
            dsp.run_until_rqm();
        }
    };
    let read_word = |dsp: &mut Upd77c25| -> u16 {
        let lo = dsp.read_dr();
        dsp.run_until_rqm();
        let hi = dsp.read_dr();
        dsp.run_until_rqm();
        u16::from(lo) | (u16::from(hi) << 8)
    };

    // Real Pilotwings flight parameters, decoded from the DR write stream:
    // cmd 0x02 = [Fx=652, Fy=4526, Fz=300, Lfe=32, Les=256, Aas=0, Azs=0x3800].
    cmd(&mut dsp, 0x02, &[652, 4526, 300, 32, 256, 0, 0x3800]);
    let _param_out: [u16; 4] = std::array::from_fn(|_| read_word(&mut dsp));

    // cmd 0x0a = continuous raster: each read cycle returns the next scanline's [A, B, C, scale];
    // the DSP must advance the raster line and recompute per cycle.
    cmd(&mut dsp, 0x0a, &[0]);
    let cycles: Vec<[u16; 4]> = (0..8)
        .map(|_| std::array::from_fn(|_| read_word(&mut dsp)))
        .collect();

    // A perspective floor advances the matrix per line, so consecutive cycles differ; a degenerate
    // (flat) floor repeats one matrix. `windows(2).any(neq)` is exactly "not all equal" and avoids a
    // heap allocation.
    assert!(
        cycles.windows(2).any(|w| w[0] != w[1]),
        "DSP-1 continuous raster did not advance: every cycle returned {:04x?} (flat Mode-7 floor). \
         run_until_rqm must run to the firmware's host-wait spin, not the first RQM=set.",
        cycles[0]
    );
}
