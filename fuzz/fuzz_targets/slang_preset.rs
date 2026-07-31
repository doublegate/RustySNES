//! Fuzz the shader-preset pipeline: `.slangp`/`.cgp` parsing **and** the GLSL→WGSL bridge.
//!
//! A libretro shader preset is a text file plus a tree of `.slang` sources, both downloaded from a
//! shader repository, so both halves are untrusted input.
//!
//! The bridge is the interesting half. `translate` returns `Result`, but the rewriters it runs
//! first — `split_stages`, `rewrite_push_constants`, `split_combined_samplers`,
//! `strip_descriptor_sets`, `parse_pragma_parameter` — are infallible `&str -> String` transforms.
//! An infallible string rewriter over adversarial input has exactly one failure mode available to
//! it, a slice-index panic, and nothing in the type signature guards against it.
//!
//! The design contract this protects is the honesty gate: naga's GLSL frontend covers a subset of
//! `#version 450`, so every failure must produce a named `Unsupported { pass, reason }` and fall
//! back to the built-in chain — **never a silent black frame, and never a crash**. A panic here
//! turns "this preset is unsupported" into "the emulator closed".
//!
//! The input is split so the preset text and the shader source vary independently; feeding the same
//! bytes to both would only ever reach the bridge with something that also parses as a preset.

#![no_main]

use libfuzzer_sys::fuzz_target;
use rustysnes_frontend::{glsl_bridge, slang_preset};
use std::path::Path;

fuzz_target!(|data: &[u8]| {
    if data.len() < 2 {
        return;
    }
    let split = usize::from(u16::from_le_bytes([data[0], data[1]])).min(data.len() - 2);
    let (preset_bytes, shader_bytes) = data[2..].split_at(split);

    let preset_text = String::from_utf8_lossy(preset_bytes);
    let shader_text = String::from_utf8_lossy(shader_bytes);

    // The rewriters, directly — they are infallible, so reaching them at all is the whole test.
    let _ = glsl_bridge::split_stages(&shader_text);
    let _ = glsl_bridge::rewrite_push_constants(&shader_text);
    let _ = glsl_bridge::split_combined_samplers(&shader_text);
    let _ = glsl_bridge::strip_descriptor_sets(&shader_text);
    let _ = glsl_bridge::parameters_of(&shader_text);
    // Capped: a per-line call over an input that is nothing but newlines spends the whole per-input
    // budget without adding coverage, since every iteration takes the same early-return path. The
    // ceiling is far above any real shader (RetroArch's largest presets are a few thousand lines).
    for line in shader_text.lines().take(4096) {
        let _ = glsl_bridge::parse_pragma_parameter(line);
    }
    let _ = glsl_bridge::translate(&shader_text, 0, "fuzz");

    // Then the preset, and the chain build that feeds each declared pass through the bridge. `base`
    // is never opened — `parse` only joins paths against it, and `to_chain` reads through the
    // closure below — so this target stays filesystem-free.
    if let Ok(preset) = slang_preset::parse(&preset_text, Path::new(".")) {
        let chain = slang_preset::to_chain(&preset, "fuzz", |_path| Some(shader_text.to_string()));
        // A pass that could not be translated must have said why. An empty chain with no recorded
        // reason is the silent-black-frame failure this whole design exists to prevent.
        assert!(
            !chain.passes.is_empty()
                || !chain.unsupported.is_empty()
                || preset.passes.is_empty(),
            "a preset with passes produced neither a pass nor an Unsupported reason"
        );
    }
});
