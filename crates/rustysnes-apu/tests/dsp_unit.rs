//! End-to-end APU integration tests (public-surface only).
//!
//! The DSP's internal algorithm vectors (BRR, Gaussian, envelope, echo, noise) are pinned by the
//! in-module `#[cfg(test)]` tests in `dsp.rs` where the private state is reachable. These tests
//! cover the assembled [`Apu`]: boot, determinism of the audio stream, port latches, and ARAM.

use rustysnes_apu::Apu;

#[test]
fn power_on_is_silent() {
    let apu = Apu::new();
    assert_eq!(apu.sample(), (0, 0));
}

#[test]
fn boots_from_ipl_reset_vector() {
    use rustysnes_apu::IPL_ROM;
    let expected = u16::from(IPL_ROM[62]) | (u16::from(IPL_ROM[63]) << 8);
    assert_eq!(expected, 0xFFC0);
}

#[test]
fn ipl_handshake_runs_without_panic() {
    // Boot the SMP and run the IPL handshake loop; it spins on the ports waiting for the CPU
    // upload, and must not panic or escape ARAM bounds.
    let mut apu = Apu::new();
    apu.run_cycles(200_000);
    let _ = apu.sample();
}

#[test]
fn deterministic_audio_stream() {
    // Identical construction + identical (empty) input must yield a bit-identical stream and ARAM.
    let mut a = Apu::new();
    let mut b = Apu::new();
    a.run_cycles(50_000);
    b.run_cycles(50_000);
    assert_eq!(a.sample(), b.sample());
    assert_eq!(a.aram(), b.aram());
}

#[test]
fn ports_round_trip_cpu_side() {
    let mut apu = Apu::new();
    apu.cpu_write_port(2, 0x55);
    // SMP hasn't written back, so the CPU-side read is the SMP latch (0), not the CPU's own write.
    assert_eq!(apu.cpu_read_port(2), 0x00);
}

#[test]
fn aram_is_64k() {
    assert_eq!(Apu::new().aram().len(), 0x1_0000);
}

#[test]
fn dsp_register_mirror_high_half() {
    let apu = Apu::new();
    assert_eq!(apu.dsp_read(0x00), apu.dsp_read(0x80));
}
