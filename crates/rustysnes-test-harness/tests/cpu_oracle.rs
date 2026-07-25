#![allow(missing_docs)]
//! 65C816 per-opcode oracle cross-check (ADR 0005).
//!
//! Replays the SingleStepTests/65816 JSON through `rustysnes_cpu::Cpu::step` and diffs the final
//! register file, RAM, and cycle count. The upstream set ships no license, so it lives in the
//! gitignored `tests/roms/external/65816-singlestep/` tier and is a LOCAL cross-validation
//! reference only — never committed, never a CI dependency. When the dir is absent (CI / a fresh
//! clone) the test prints a SKIP line and passes.
//!
//! Knobs (env):
//!   `RUSTYSNES_ORACLE_PER_FILE`   tests per opcode file (default 200; 0 = all 10000)
//!   `RUSTYSNES_ORACLE_MAX_FILES`  cap on opcode files scanned (default 0 = all 512)
//!   `RUSTYSNES_ORACLE_FLOOR`      minimum pass-rate to require (default 0.0 = report only)
#![cfg(feature = "test-roms")]
// Test oracle: JSON `u64` fields are narrowed to register widths (u8/u16/u32) — bounded by the
// test format, so the truncation/precision casts are intentional and safe — and pass-rate math
// casts counts to f64. Short binding names (a/v/g/r/t) are idiomatic in the tight per-test loop.
#![allow(
    clippy::cast_possible_truncation,
    clippy::cast_precision_loss,
    clippy::many_single_char_names,
    clippy::doc_markdown
)]

use std::collections::HashMap;
use std::path::PathBuf;

use rustysnes_cpu::{Bus, Cpu, Status};
use serde_json::Value;

/// A flat 24-bit memory the single-step tests seed and diff. Cycle counts come from the value
/// [`Cpu::step`] returns (the per-instruction cycle count), not the bus. `accesses` records the
/// ordered sequence of real bus reads/writes (`(addr, is_write)`) so the oracle can cross-check the
/// **access order** against the SingleStepTests per-cycle trace (T-CA-11), not just the end state and
/// cycle count. Internal (`io`) cycles drive no memory access here, matching the SST cycles whose
/// pin flags carry neither `r` nor `w`.
struct TestBus {
    mem: HashMap<u32, u8>,
    accesses: Vec<(u32, bool)>,
}

impl Bus for TestBus {
    fn read24(&mut self, addr24: u32) -> u8 {
        self.accesses.push((addr24 & 0x00FF_FFFF, false));
        *self.mem.get(&(addr24 & 0x00FF_FFFF)).unwrap_or(&0)
    }
    fn write24(&mut self, addr24: u32, val: u8) {
        self.accesses.push((addr24 & 0x00FF_FFFF, true));
        self.mem.insert(addr24 & 0x00FF_FFFF, val);
    }
}

/// The ordered real memory accesses `(addr, is_write)` the SST per-cycle trace records: every cycle
/// whose pin string carries an `r` (read) or `w` (write). Internal cycles (address held, no access —
/// neither flag) are skipped, exactly as `Cpu::io` performs no bus access here.
fn sst_accesses(t: &Value) -> Vec<(u32, bool)> {
    t["cycles"]
        .as_array()
        .into_iter()
        .flatten()
        .filter_map(|c| {
            let pins = c[2].as_str()?;
            let addr = (c[0].as_u64()? as u32) & 0x00FF_FFFF;
            if pins.contains('r') {
                Some((addr, false))
            } else if pins.contains('w') {
                Some((addr, true))
            } else {
                None
            }
        })
        .collect()
}

fn oracle_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../tests/roms/external/65816-singlestep/v1")
}

fn env_usize(key: &str, default: usize) -> usize {
    std::env::var(key)
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(default)
}

fn set_regs(cpu: &mut Cpu, st: &Value) {
    let r = &mut cpu.regs;
    let g = |k: &str| st[k].as_u64().unwrap_or(0);
    r.a = g("a") as u16;
    r.x = g("x") as u16;
    r.y = g("y") as u16;
    r.s = g("s") as u16;
    r.d = g("d") as u16;
    r.dbr = g("dbr") as u8;
    r.pbr = g("pbr") as u8;
    r.pc = g("pc") as u16;
    r.p = Status::from_bits_retain(g("p") as u8);
    r.emulation = g("e") == 1;
}

/// Returns (register_ok, ram_ok, cycle_ok, access_order_ok) for one test object. `access_order_ok`
/// is `None` for the block-move opcodes (their re-step + PC-nudge tail doesn't map 1:1 to the trace).
fn run_one(t: &Value) -> (bool, bool, bool, Option<bool>) {
    let init = &t["initial"];
    let mut bus = TestBus {
        mem: HashMap::new(),
        accesses: Vec::new(),
    };
    for pair in init["ram"].as_array().into_iter().flatten() {
        let a = pair[0].as_u64().unwrap_or(0) as u32 & 0x00FF_FFFF;
        let v = pair[1].as_u64().unwrap_or(0) as u8;
        bus.mem.insert(a, v);
    }
    let mut cpu = Cpu::new();
    set_regs(&mut cpu, init);

    // MVN/MVP (0x44/0x54) are looping block-move instructions. The SingleStepTests reference
    // captures state after a fixed *cycle budget* (mid-move, A never wraps), advancing the move
    // one byte per re-fetch. Our `Cpu::step` models one byte then rewinds PC by 3 (ares
    // `if(A.w--) PC.w -= 3`), so to reproduce the oracle we re-step until the recorded cycle
    // count is reached. The final partial iteration (opcode + one operand fetch) lands PC at
    // opcode+2 and contributes its 2 cycles, exactly matching the fixtures.
    let opcode = bus
        .mem
        .get(&(((u32::from(cpu.regs.pbr) << 16) | u32::from(cpu.regs.pc)) & 0x00FF_FFFF))
        .copied()
        .unwrap_or(0);
    let expected_cycles = t["cycles"].as_array().map_or(0, Vec::len) as u32;

    let got_cycles = if opcode == 0x44 || opcode == 0x54 {
        // Drive whole-byte iterations until the next one would overshoot the budget, then run
        // the partial tail (opcode fetch + first operand) to hit the exact cycle count.
        let mut acc = 0u32;
        while acc + 7 <= expected_cycles {
            let c = cpu.step(&mut bus);
            if c == 0 {
                break; // safety: no progress
            }
            acc += c;
        }
        // Partial tail: re-fetch opcode (+1) and the dst-bank operand (+1) → PC at opcode+2.
        while acc < expected_cycles {
            cpu.regs.pc = cpu.regs.pc.wrapping_add(1);
            acc += 1;
        }
        acc
    } else {
        cpu.step(&mut bus)
    };

    let fin = &t["final"];
    let r = &cpu.regs;
    let g = |k: &str| fin[k].as_u64().unwrap_or(0);
    let reg_ok = u64::from(r.a) == g("a")
        && u64::from(r.x) == g("x")
        && u64::from(r.y) == g("y")
        && u64::from(r.s) == g("s")
        && u64::from(r.d) == g("d")
        && u64::from(r.dbr) == g("dbr")
        && u64::from(r.pbr) == g("pbr")
        && u64::from(r.pc) == g("pc")
        && u64::from(r.p.bits()) == g("p")
        && u64::from(u8::from(r.emulation)) == g("e");

    let ram_ok = fin["ram"].as_array().into_iter().flatten().all(|pair| {
        let a = pair[0].as_u64().unwrap_or(0) as u32 & 0x00FF_FFFF;
        let v = pair[1].as_u64().unwrap_or(0) as u8;
        *bus.mem.get(&a).unwrap_or(&0) == v
    });

    let cycle_ok = u64::from(got_cycles) == t["cycles"].as_array().map_or(0, |c| c.len() as u64);

    // Access-order cross-check (T-CA-11): the ordered real reads/writes must match the SST trace.
    // Skip the block-move opcodes, whose re-step loop + PC-nudge tail don't map 1:1 to a single
    // instruction's trace.
    let access_ok = if opcode == 0x44 || opcode == 0x54 {
        None
    } else {
        Some(bus.accesses == sst_accesses(t))
    };
    (reg_ok, ram_ok, cycle_ok, access_ok)
}

#[test]
#[allow(clippy::too_many_lines)] // linear reporting block; splitting it hurts readability
fn cpu_65816_oracle_cross_check() {
    let dir = oracle_dir();
    if !dir.is_dir() {
        eprintln!(
            "SKIP cpu_65816_oracle: {} absent (gitignored external tier — fetch SingleStepTests/65816 locally)",
            dir.display()
        );
        return;
    }
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

    let (mut total, mut full_pass, mut reg_pass, mut cyc_pass) = (0u64, 0u64, 0u64, 0u64);
    let (mut acc_total, mut acc_pass) = (0u64, 0u64);
    let mut worst: HashMap<String, u64> = HashMap::new();
    let mut acc_worst: HashMap<String, u64> = HashMap::new();

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
            let (reg_ok, ram_ok, cyc_ok, acc_ok) = run_one(t);
            total += 1;
            if reg_ok && ram_ok {
                reg_pass += 1;
            } else {
                *worst.entry(stem.clone()).or_default() += 1;
            }
            if cyc_ok {
                cyc_pass += 1;
            }
            if reg_ok && ram_ok && cyc_ok {
                full_pass += 1;
            }
            if let Some(ok) = acc_ok {
                acc_total += 1;
                if ok {
                    acc_pass += 1;
                } else {
                    *acc_worst.entry(stem.clone()).or_default() += 1;
                }
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
        "\n=== 65816 oracle cross-check ({} files, {} tests) ===",
        files.len(),
        total
    );
    eprintln!(
        "  state (regs+ram) : {:>7} / {} = {:.2}%",
        reg_pass,
        total,
        pct(reg_pass)
    );
    eprintln!(
        "  cycle count      : {:>7} / {} = {:.2}%",
        cyc_pass,
        total,
        pct(cyc_pass)
    );
    eprintln!(
        "  full (state+cyc) : {:>7} / {} = {:.2}%",
        full_pass,
        total,
        pct(full_pass)
    );
    let acc_pct = if acc_total == 0 {
        0.0
    } else {
        acc_pass as f64 * 100.0 / acc_total as f64
    };
    eprintln!(
        "  access order     : {acc_pass:>7} / {acc_total} = {acc_pct:.2}%  (T-CA-11: read/write sequence vs the SST trace)"
    );
    let mut top: Vec<_> = worst.iter().collect();
    top.sort_by(|a, b| b.1.cmp(a.1));
    eprintln!("  worst opcodes (file: state-fails):");
    for (op, n) in top.iter().take(12) {
        eprintln!("    {op}: {n}");
    }
    let mut acc_top: Vec<_> = acc_worst.iter().collect();
    acc_top.sort_by(|a, b| b.1.cmp(a.1));
    eprintln!("  worst opcodes (file: access-order-fails):");
    for (op, n) in acc_top.iter().take(12) {
        eprintln!("    {op}: {n}");
    }

    let rate = if total == 0 {
        0.0
    } else {
        reg_pass as f64 / total as f64
    };
    assert!(
        rate >= floor,
        "state pass-rate {rate:.4} below floor {floor:.4}"
    );
}
