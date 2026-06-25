#!/usr/bin/env python3
"""SNES test-corpus coverage tool (the RustySNES port of RustyNES's scripts/coverage/coverage.py).

Stdlib-only. Scans a directory of (optionally zipped) SNES ROMs, parses each internal header,
classifies board (LoROM/HiROM/ExHiROM) + coprocessor, and either:

  survey  — read-only: print a `board | coprocessor | tier | avail | target | gap` coverage
            table over the whole library. Default. Never writes.
  stage   — copy >=N distinct dumps per (board, coprocessor) bucket into the gitignored
            tests/roms/external/commercial/ tree. Dry-run unless --execute.
  catalog — emit the committed corpus MANIFEST (JSON, metadata only: sha256 + classification,
            NEVER rom bytes) for the staged selection.

Classification strategy (mirrors how bsnes/ares actually do it):
  * map mode  <- the header OFFSET that scores valid (LoROM $7FC0 / HiROM $FFC0 / ExHiROM
                 $40FFC0), cross-checked against header byte $15.
  * region    <- destination byte $19 (NTSC: 0x00/0x01/0x0D; else PAL).
  * sram/batt <- ram-size byte $18 + cartridge-type low nibble $16.
  * coproc    <- cartridge-type high nibble $16 (DSP/SuperFX/OBC1/SA-1/S-DD1/S-RTC), and for the
                 "Custom" ($F) class the chipset-subtype byte at $FFBF; THEN a curated
                 title->chip override (authoritative for the handful of custom-chip games and to
                 split DSP-1/2/3/4 + GSU-1/2, which the header cannot distinguish).

The SNES `Coprocessor` enum (crates/rustysnes-cart/src/header.rs) is the committed vocabulary;
this tool additionally tracks a fine-grained `chip` tag (DSP-2/3/4, ST010/011/018, GSU-2, ...)
for the coverage matrix even where the enum (#[non_exhaustive]) has no variant yet.
"""

from __future__ import annotations

import argparse
import hashlib
import io
import json
import os
import sys
import zipfile
from dataclasses import dataclass, field, asdict

# --- classification vocabulary ------------------------------------------------

# Header cartridge-type high-nibble -> coprocessor family (when low nibble >= 3).
COPRO_BY_HIGH_NIBBLE = {
    0x0: "DSP", 0x1: "SuperFX", 0x2: "OBC1", 0x3: "SA1",
    0x4: "SDD1", 0x5: "SRTC", 0xE: "Other", 0xF: "Custom",
}
# Chipset-subtype byte ($FFBF) when high nibble == Custom ($F).
CUSTOM_SUBTYPE = {0x00: "SPC7110", 0x01: "ST010", 0x02: "ST018", 0x10: "CX4"}

# Curated title-substring -> fine chip. AUTHORITATIVE: applied last, overrides header guesswork.
# Specific known titles only (no broad regexes — those produce false positives).
TITLE_OVERRIDES = [
    # (substring lowercased, fine-chip, enum-coprocessor)
    ("mega man x2", "CX4", "Cx4"), ("mega man x3", "CX4", "Cx4"),
    ("rockman x2", "CX4", "Cx4"), ("rockman x3", "CX4", "Cx4"),
    ("star ocean", "S-DD1", "SDd1"),
    ("street fighter alpha 2", "S-DD1", "SDd1"), ("street fighter zero 2", "S-DD1", "SDd1"),
    ("far east of eden zero", "SPC7110", "Spc7110"), ("tengai makyou zero", "SPC7110", "Spc7110"),
    ("momotarou dentetsu happy", "SPC7110", "Spc7110"), ("super power league 4", "SPC7110", "Spc7110"),
    ("metal combat", "OBC1", "Obc1"),
    ("f1 roc ii", "ST010", "Dsp"), ("exhaust heat ii", "ST010", "Dsp"),
    ("hayazashi nidan morita shougi 2", "ST018", "Dsp"),
    ("hayazashi nidan morita shougi", "ST011", "Dsp"),
    ("daikaijuu monogatari ii", "S-RTC", "SRtc_unmapped"),
    # DSP sub-variants (header reports plain DSP family):
    ("dungeon master", "DSP-2", "Dsp"), ("sd gundam gx", "DSP-3", "Dsp"),
    ("top gear 3000", "DSP-4", "Dsp"), ("planet's champ tg 3000", "DSP-4", "Dsp"),
    # Super FX GSU-2 (distinguish from GSU-1; both header-report SuperFX). `doom` is EXACT-title
    # (4th element) so it doesn't swallow "Doom Troopers" / "Fortress of Doom".
    ("yoshi's island", "GSU-2", "SuperFx"), ("super mario world 2", "GSU-2", "SuperFx"),
    ("doom", "GSU-2", "SuperFx", True), ("winter gold", "GSU-2", "SuperFx"),
]

# Board tier (ADR 0003), keyed by enum coprocessor. Mirrors crates/rustysnes-cart/src/tier.rs.
TIER = {
    "None": "Core", "Dsp": "Core",
    "SuperFx": "Curated", "Sa1": "Curated",
    "SDd1": "BestEffort", "Spc7110": "BestEffort", "Cx4": "BestEffort", "Obc1": "BestEffort",
    "SRtc_unmapped": "BestEffort",
}

ROM_EXTS = (".sfc", ".smc", ".swc", ".fig", ".bin")

# Hack / translation / non-retail markers. A ROM whose archive name or internal title contains
# one of these is DESELECTED in favour of a clean dump — but only as long as the bucket still has
# a clean candidate; the fallback in _select keeps a marked ROM rather than lose a board entirely.
HACK_MARKERS = (
    "redux", "zero project", " project j", "engv", "-eng", "(t-", "[t-", "(t+", "[t+",
    "translation", "(hack", "[hack", "hack)", "(beta", "(demo", "(proto", "(sample", "(aftermarket",
    "(pirate", "(unl)", "music kart", "kart r", "street kart", "hind strike", "deluxe edition",
    "remix", "enhanced)", "uncensored", "restoration",
    # Mario Kart hack cluster (this set carries many; the real Super Mario Kart is LoROM):
    "kart 8", "epic racers", "super circuit", "baldy", "kart -", "mariokart 8",
)


def _is_marked(r: "RomInfo") -> bool:
    s = (r.inner + " " + r.title + " " + os.path.basename(r.path)).lower()
    return any(m in s for m in HACK_MARKERS)


@dataclass
class RomInfo:
    path: str
    inner: str          # name inside the archive (or "" if raw)
    sha256: str
    size: int
    map_mode: str       # LoRom / HiRom / ExHiRom
    region: str         # Ntsc / Pal
    coprocessor: str    # enum-level: None/Dsp/SuperFx/Sa1/SDd1/Spc7110/Cx4/Obc1/...
    chip: str           # fine-grained: GSU-1/GSU-2/DSP-1.../ST010...
    sram_kb: int
    has_battery: bool
    title: str
    score: int          # header-detection confidence


# --- header parsing -----------------------------------------------------------

def _strip_copier(data: bytes) -> bytes:
    # 512-byte copier prefix when the image size is 512 past a 32 KiB multiple.
    if len(data) % 0x8000 == 0x200:
        return data[0x200:]
    return data


def _printable_title(b: bytes) -> str:
    return "".join(chr(c) if 0x20 <= c < 0x7F else " " for c in b).strip()


def _score_offset(data: bytes, base: int) -> int:
    """Confidence that a valid SNES header sits at `base`. Higher = better."""
    if base + 0x40 > len(data):
        return -1
    title = data[base:base + 0x15]
    printable = sum(1 for c in title if 0x20 <= c < 0x7F)
    chk = int.from_bytes(data[base + 0x1E:base + 0x20], "little")
    cmp = int.from_bytes(data[base + 0x1C:base + 0x1E], "little")
    score = printable  # 0..21
    if (chk ^ cmp) == 0xFFFF and chk != 0:
        score += 32
    # reset vector ($FFFC) should point into ROM space (>= $8000)
    return score


def _region(byte: int) -> str:
    return "Ntsc" if byte in (0x00, 0x01, 0x0D) else "Pal"


def classify(data: bytes, name: str) -> RomInfo | None:
    data = _strip_copier(data)
    if len(data) < 0x8000:
        return None
    candidates = [(0x7FC0, "LoRom"), (0xFFC0, "HiRom"), (0x40FFC0, "ExHiRom")]
    best = None
    for base, mode in candidates:
        s = _score_offset(data, base)
        if best is None or s > best[0]:
            best = (s, base, mode)
    score, base, map_mode = best
    if score < 0:
        return None

    ctype = data[base + 0x16]
    low, high = ctype & 0x0F, (ctype >> 4) & 0x0F
    region = _region(data[base + 0x19])
    ram_byte = data[base + 0x18]
    sram_kb = (1 << ram_byte) if 0 < ram_byte <= 0x0C else 0
    has_battery = low in (0x02, 0x05, 0x06)
    title = _printable_title(data[base:base + 0x15])

    coprocessor, chip = "None", "-"
    if low >= 0x03:
        fam = COPRO_BY_HIGH_NIBBLE.get(high, "Other")
        if fam == "DSP":
            coprocessor, chip = "Dsp", "DSP-1"
        elif fam == "SuperFX":
            coprocessor, chip = "SuperFx", "GSU-1"
        elif fam == "OBC1":
            coprocessor, chip = "Obc1", "OBC1"
        elif fam == "SA1":
            coprocessor, chip = "Sa1", "SA-1"
        elif fam == "SDD1":
            coprocessor, chip = "SDd1", "S-DD1"
        elif fam == "SRTC":
            coprocessor, chip = "SRtc_unmapped", "S-RTC"
        elif fam == "Custom":
            sub = CUSTOM_SUBTYPE.get(data[base - 1] if base >= 1 else 0, "Custom")
            chip = sub
            coprocessor = {"SPC7110": "Spc7110", "CX4": "Cx4",
                           "ST010": "Dsp", "ST018": "Dsp"}.get(sub, "None")
        else:  # "Other" (SGB / BS-X)
            coprocessor, chip = "None", "Other"

    # Authoritative curated override (fixes DSP-N split, GSU-2, and all custom chips).
    low_title = title.lower()
    low_name = name.lower()
    for entry in TITLE_OVERRIDES:
        sub, fine, enum_cop = entry[0], entry[1], entry[2]
        exact = len(entry) > 3 and entry[3]
        hit = (low_title.strip() == sub) if exact else (sub in low_title or sub in low_name)
        if hit:
            coprocessor, chip = enum_cop, fine
            break

    return RomInfo(path="", inner=name, sha256="", size=len(data), map_mode=map_mode,
                   region=region, coprocessor=coprocessor, chip=chip, sram_kb=sram_kb,
                   has_battery=has_battery, title=title, score=score)


# --- library scan -------------------------------------------------------------

def _iter_roms(root: str):
    for dirpath, _dirs, files in os.walk(root):
        for f in sorted(files):
            full = os.path.join(dirpath, f)
            lf = f.lower()
            if lf.endswith(".zip"):
                try:
                    with zipfile.ZipFile(full) as z:
                        for zi in z.infolist():
                            if zi.filename.lower().endswith(ROM_EXTS):
                                yield full, zi.filename, z.read(zi)
                                break
                except (zipfile.BadZipFile, OSError):
                    continue
            elif lf.endswith(ROM_EXTS):
                try:
                    with open(full, "rb") as fh:
                        yield full, "", fh.read()
                except OSError:
                    continue


def scan(root: str, want_sha: bool = False) -> list[RomInfo]:
    out = []
    for full, inner, data in _iter_roms(root):
        info = classify(data, inner or os.path.basename(full))
        if info is None:
            continue
        info.path = full
        if want_sha:
            info.sha256 = hashlib.sha256(data).hexdigest()
        out.append(info)
    return out


# --- subcommands --------------------------------------------------------------

def _bucket(i: RomInfo) -> str:
    return f"{i.map_mode}/{i.chip if i.chip != '-' else i.coprocessor}"


def cmd_survey(args):
    roms = scan(args.romdir)
    buckets: dict[str, list[RomInfo]] = {}
    for r in roms:
        buckets.setdefault(_bucket(r), []).append(r)
    print(f"# scanned {len(roms)} ROMs from {args.romdir}\n")
    print(f"{'board / chip':28} {'tier':10} {'avail':>5} {'target':>6} {'gap':>4}")
    print("-" * 60)
    target = args.target
    total_gap = 0
    for b in sorted(buckets):
        items = buckets[b]
        tier = TIER.get(items[0].coprocessor, "BestEffort")
        avail = len(items)
        # population-limited categories: target is min(target, avail)
        gap = max(0, target - avail)
        flag = "" if gap == 0 else f"-{gap}"
        if gap:
            total_gap += 1
        print(f"{b:28} {tier:10} {avail:5d} {target:6d} {flag:>4}")
    print("-" * 60)
    print(f"buckets under target (population-limited or missing): {total_gap}")
    # regions + battery quick coverage
    ntsc = sum(1 for r in roms if r.region == "Ntsc")
    pal = sum(1 for r in roms if r.region == "Pal")
    batt = sum(1 for r in roms if r.has_battery)
    print(f"region: NTSC={ntsc} PAL={pal} | battery-backed: {batt}")


def cmd_catalog(args):
    roms = scan(args.romdir, want_sha=True)
    buckets: dict[str, list[RomInfo]] = {}
    for r in roms:
        buckets.setdefault(_bucket(r), []).append(r)
    selection = []
    for b in sorted(buckets):
        cands = sorted(buckets[b], key=lambda r: (-r.score, r.inner))
        if args.include_hacks:
            picked = cands[:args.target]
        else:
            clean = [r for r in cands if not _is_marked(r)]
            marked = [r for r in cands if _is_marked(r)]
            picked = (clean + marked)[:args.target]
        for r in picked:
            selection.append({
                "bucket": b, "title": r.title.strip(), "sha256": r.sha256,
                "map_mode": r.map_mode, "region": r.region,
                "coprocessor": r.coprocessor, "chip": r.chip,
                "sram_kb": r.sram_kb, "has_battery": r.has_battery,
                "size": r.size, "source_archive": os.path.basename(r.path),
                # NOTE: NO rom bytes. golden hashes get filled by the harness post-capture.
                "golden": None,
            })
    out = {"target_per_bucket": args.target, "selected": len(selection),
           "buckets": len(buckets), "entries": selection}
    text = json.dumps(out, indent=2)
    if args.out:
        with open(args.out, "w") as fh:
            fh.write(text + "\n")
        print(f"wrote {len(selection)} entries across {len(buckets)} buckets -> {args.out}")
    else:
        print(text)


def _safe(s: str) -> str:
    out = "".join(c if (c.isalnum() or c in " -_'.") else "_" for c in s).strip()
    return out or "rom"


def _select(roms: list[RomInfo], target: int, include_hacks: bool = False) -> dict[str, str]:
    """sha256 -> relative target path (<map>/<chip>/<name>.sfc) for the chosen ROMs.

    Clean retail dumps are preferred; marked (hack/translation/proto) ROMs are appended only as
    fallback so a board is never lost just because its only dumps are non-retail.
    """
    buckets: dict[str, list[RomInfo]] = {}
    for r in roms:
        buckets.setdefault(_bucket(r), []).append(r)
    selected: dict[str, str] = {}
    for b in sorted(buckets):
        cands = sorted(buckets[b], key=lambda r: (-r.score, r.inner))
        if include_hacks:
            ordered = cands
        else:
            clean = [r for r in cands if not _is_marked(r)]
            marked = [r for r in cands if _is_marked(r)]
            ordered = clean + marked  # prefer clean; fall back to marked to fill the bucket
        for r in ordered[:target]:
            chipdir = r.chip if r.chip != "-" else r.coprocessor
            base = _safe(os.path.splitext(os.path.basename(r.path))[0])
            selected[r.sha256] = os.path.join(r.map_mode, chipdir, base + ".sfc")
    return selected


def cmd_stage(args):
    # Pass 1: decide the selection (same logic as `catalog`).
    selected = _select(scan(args.romdir, want_sha=True), args.target, args.include_hacks)
    print(f"# selecting {len(selected)} ROMs -> {args.dest}")
    if not args.execute:
        for rel in sorted(selected.values()):
            print(f"  [dry] {rel}")
        print("\ndry-run; pass --execute to write the files")
        return
    # Pass 2: re-walk, match by sha256, write the RAW bytes (so the stored file's sha256 equals
    # the manifest pin — the emulator strips any copier prefix at load time, not here).
    written = 0
    for full, _inner, data in _iter_roms(args.romdir):
        rel = selected.get(hashlib.sha256(data).hexdigest())
        if not rel:
            continue
        dest = os.path.join(args.dest, rel)
        os.makedirs(os.path.dirname(dest), exist_ok=True)
        with open(dest, "wb") as fh:
            fh.write(data)
        written += 1
    print(f"wrote {written}/{len(selected)} ROMs to {args.dest}")
    if written != len(selected):
        print(f"WARNING: {len(selected) - written} selected ROM(s) not found on the second pass")


def main():
    p = argparse.ArgumentParser(description="SNES test-corpus coverage tool")
    p.add_argument("--target", type=int, default=5,
                   help="ROMs wanted per (board,coprocessor) bucket (default 5; capped by population)")
    sub = p.add_subparsers(dest="cmd", required=True)

    s = sub.add_parser("survey", help="read-only coverage table")
    s.add_argument("romdir")
    s.set_defaults(func=cmd_survey)

    c = sub.add_parser("catalog", help="emit the committed manifest (metadata only, no ROM bytes)")
    c.add_argument("romdir")
    c.add_argument("--out", help="write JSON manifest here (default stdout)")
    c.add_argument("--include-hacks", action="store_true", help="keep hack/translation/proto dumps")
    c.set_defaults(func=cmd_catalog)

    st = sub.add_parser("stage", help="copy selected dumps into the gitignored external/ tree")
    st.add_argument("romdir")
    st.add_argument("--dest", default="tests/roms/external/commercial",
                    help="destination root (gitignored; default tests/roms/external/commercial)")
    st.add_argument("--execute", action="store_true", help="actually write files (default dry-run)")
    st.add_argument("--include-hacks", action="store_true", help="keep hack/translation/proto dumps")
    st.set_defaults(func=cmd_stage)

    args = p.parse_args()
    args.func(args)


if __name__ == "__main__":
    main()
