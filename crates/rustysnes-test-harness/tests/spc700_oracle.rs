#![allow(missing_docs)]
//! SPC700 per-opcode oracle cross-check (Phase 3 / T-31-001).
//!
//! Replays the SingleStepTests/spc700 JSON through `rustysnes_apu::Spc700::step` against a flat
//! 64 KiB RAM (the suite's memory model: no IPL/IO interception) and diffs the final register
//! file, RAM, and cycle count. SingleStepTests/spc700 is MIT-licensed, so the committed sample
//! at `tests/roms/spc700-singlestep/v1/*.json` (256 files, all opcodes) ships in-tree; the full
//! per-opcode set, if fetched, lives in the gitignored `tests/roms/external/spc700-singlestep-full/`
//! tier and is preferred automatically. When neither dir is present the test prints SKIP and passes.
//!
//! Knobs (env):
//!   `RUSTYSNES_ORACLE_PER_FILE`   tests per opcode file (default 200; 0 = all)
//!   `RUSTYSNES_ORACLE_MAX_FILES`  cap on opcode files scanned (default 0 = all 256)
//!   `RUSTYSNES_ORACLE_FLOOR`      minimum state pass-rate to require (default 0.0 = report only)
#![cfg(feature = "test-roms")]
// JSON `u64` fields are narrowed to register widths (u8/u16) — bounded by the test format — and
// pass-rate math casts counts to f64. Short binding names are idiomatic in the tight test loop.
#![allow(
    clippy::cast_possible_truncation,
    clippy::cast_precision_loss,
    clippy::many_single_char_names,
    clippy::doc_markdown,
    clippy::large_stack_arrays
)]

use std::collections::HashMap;
use std::path::PathBuf;

use rustysnes_apu::psw::Psw;
use rustysnes_apu::spc700::{Spc700, Spc700Bus};
use serde_json::Value;

/// A flat 64 KiB SPC700 address space the single-step tests seed and diff. Every `read`/`write`/
/// `idle` is one cycle, so `cycles` equals the instruction's bus-activity length.
struct TestBus {
    mem: Box<[u8; 0x1_0000]>,
    cycles: u32,
}

impl Spc700Bus for TestBus {
    fn read(&mut self, address: u16) -> u8 {
        self.cycles += 1;
        self.mem[address as usize]
    }
    fn write(&mut self, address: u16, data: u8) {
        self.cycles += 1;
        self.mem[address as usize] = data;
    }
    fn idle(&mut self) {
        self.cycles += 1;
    }
}

/// Prefer the (gitignored) full set; fall back to the committed 256-opcode sample.
fn oracle_dir() -> Option<PathBuf> {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");
    let full = root.join("tests/roms/external/spc700-singlestep-full");
    if full.is_dir() {
        return Some(full);
    }
    let sample = root.join("tests/roms/spc700-singlestep/v1");
    sample.is_dir().then_some(sample)
}

fn env_usize(key: &str, default: usize) -> usize {
    std::env::var(key)
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(default)
}

fn set_regs(cpu: &mut Spc700, st: &Value) {
    let g = |k: &str| st[k].as_u64().unwrap_or(0);
    cpu.regs.pc = g("pc") as u16;
    cpu.regs.a = g("a") as u8;
    cpu.regs.x = g("x") as u8;
    cpu.regs.y = g("y") as u8;
    cpu.regs.sp = g("sp") as u8;
    cpu.regs.psw = Psw::from_bits(g("psw") as u8);
}

/// Returns (register_ok, ram_ok, cycle_ok) for one test object.
fn run_one(t: &Value) -> (bool, bool, bool) {
    let init = &t["initial"];
    let mut bus = TestBus {
        mem: Box::new([0; 0x1_0000]),
        cycles: 0,
    };
    for pair in init["ram"].as_array().into_iter().flatten() {
        let a = pair[0].as_u64().unwrap_or(0) as usize & 0xFFFF;
        let v = pair[1].as_u64().unwrap_or(0) as u8;
        bus.mem[a] = v;
    }
    let mut cpu = Spc700::new();
    set_regs(&mut cpu, init);
    cpu.step(&mut bus);

    let fin = &t["final"];
    let r = &cpu.regs;
    let g = |k: &str| fin[k].as_u64().unwrap_or(0);
    let reg_ok = u64::from(r.a) == g("a")
        && u64::from(r.x) == g("x")
        && u64::from(r.y) == g("y")
        && u64::from(r.sp) == g("sp")
        && u64::from(r.pc) == g("pc")
        && u64::from(r.psw.bits()) == g("psw");

    let ram_ok = fin["ram"].as_array().into_iter().flatten().all(|pair| {
        let a = pair[0].as_u64().unwrap_or(0) as usize & 0xFFFF;
        let v = pair[1].as_u64().unwrap_or(0) as u8;
        bus.mem[a] == v
    });

    let expected = t["cycles"].as_array().map_or(0, Vec::len) as u32;
    let cycle_ok = bus.cycles == expected;
    (reg_ok, ram_ok, cycle_ok)
}

#[test]
#[allow(clippy::too_many_lines)]
fn spc700_oracle_cross_check() {
    let Some(dir) = oracle_dir() else {
        eprintln!(
            "SKIP spc700_oracle: neither tests/roms/spc700-singlestep/v1 nor the external full set present"
        );
        return;
    };
    let per_file = env_usize("RUSTYSNES_ORACLE_PER_FILE", 200);
    let max_files = env_usize("RUSTYSNES_ORACLE_MAX_FILES", 0);
    let floor: f64 = std::env::var("RUSTYSNES_ORACLE_FLOOR")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(0.0);

    let mut files: Vec<_> = std::fs::read_dir(&dir)
        .expect("read oracle dir")
        .filter_map(Result::ok)
        .map(|e| e.path())
        .filter(|p| p.extension().is_some_and(|x| x == "json"))
        .collect();
    files.sort();
    if max_files > 0 && files.len() > max_files {
        files.truncate(max_files);
    }

    let (mut total, mut full_pass, mut state_pass, mut cyc_pass) = (0u64, 0u64, 0u64, 0u64);
    let mut worst: HashMap<String, u64> = HashMap::new();
    let mut worst_cyc: HashMap<String, u64> = HashMap::new();

    for path in &files {
        let bytes = std::fs::read(path).expect("read json");
        let tests: Vec<Value> = serde_json::from_slice(&bytes).expect("parse json array");
        let stem = path
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("?")
            .to_string();
        let take = if per_file == 0 {
            tests.len()
        } else {
            per_file.min(tests.len())
        };
        for t in tests.iter().take(take) {
            let (reg_ok, ram_ok, cyc_ok) = run_one(t);
            total += 1;
            if reg_ok && ram_ok {
                state_pass += 1;
            } else {
                *worst.entry(stem.clone()).or_default() += 1;
            }
            if cyc_ok {
                cyc_pass += 1;
            } else {
                *worst_cyc.entry(stem.clone()).or_default() += 1;
            }
            if reg_ok && ram_ok && cyc_ok {
                full_pass += 1;
            }
        }
    }

    let pct = |n: u64| {
        if total == 0 {
            0.0
        } else {
            n as f64 * 100.0 / total as f64
        }
    };
    eprintln!(
        "\n=== SPC700 oracle cross-check ({} files, {} tests) ===",
        files.len(),
        total
    );
    eprintln!("  source           : {}", dir.display());
    eprintln!(
        "  state (regs+ram) : {state_pass:>7} / {total} = {:.2}%",
        pct(state_pass)
    );
    eprintln!(
        "  cycle count      : {cyc_pass:>7} / {total} = {:.2}%",
        pct(cyc_pass)
    );
    eprintln!(
        "  full (state+cyc) : {full_pass:>7} / {total} = {:.2}%",
        pct(full_pass)
    );
    let mut top: Vec<_> = worst.iter().collect();
    top.sort_by(|a, b| b.1.cmp(a.1));
    eprintln!("  worst opcodes (file: state-fails):");
    for (op, n) in top.iter().take(12) {
        eprintln!("    {op}: {n}");
    }
    let mut topc: Vec<_> = worst_cyc.iter().collect();
    topc.sort_by(|a, b| b.1.cmp(a.1));
    eprintln!("  worst opcodes (file: cycle-fails):");
    for (op, n) in topc.iter().take(16) {
        eprintln!("    {op}: {n}");
    }

    let rate = if total == 0 {
        0.0
    } else {
        state_pass as f64 / total as f64
    };
    assert!(
        rate >= floor,
        "state pass-rate {rate:.4} below floor {floor:.4}"
    );
}
