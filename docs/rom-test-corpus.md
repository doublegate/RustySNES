# ROM test corpus inventory

This doc catalogs, per mapper mode / coprocessor / region / test category, the best available ROM
to validate against and its current availability on this development machine. It exists so a
future session doesn't have to re-derive "is there a ROM for X" from scratch, and so every
currently-documented "no ROM available" gap (`docs/STATUS.md`, `docs/ppu.md`, `docs/cart.md`,
`docs/audit/`) is tracked in one place instead of scattered across prose.

**Never commit a commercial ROM dump to this repository** (`docs/adr/0003`) — only derived
screenshots/hashes may be committed. This doc's "🟡 in Dropbox" entries name a *locally available*
file for a developer to manually stage into the gitignored `tests/roms/external/commercial/`
corpus; it is not an instruction to copy anything into the repo itself.

**Corpus locations referenced below:**

- `tests/roms/` — committed, permissively-licensed (gilyon, undisbeliever, spc700-singlestep,
  65816-singlestep).
- `tests/roms/external/` — gitignored, larger corpus: `commercial/{LoRom,HiRom}/<Coprocessor>/`
  (commercial dumps, mapper × coprocessor), `krom/` (a large homebrew CC0 test-ROM suite), plus
  `firmware/` (coprocessor firmware dumps used to pair with a cart at runtime), `blargg-spc/`,
  `240p/`, `spc700-singlestep-full/`.
- `/home/parobek/Dropbox/ROMs/Super Nintendo Entertainment System - Super Famicom (2020)/` — a
  ~500-title personal commercial ROM collection on this machine, **not part of the repo and not
  staged into `tests/roms/external/`**. Cross-referenced below purely to answer "does a candidate
  ROM already exist somewhere on this machine." This is a Western-market-focused set (verified by
  sampling filenames end-to-end — no Japan-exclusive titles observed, no explicit
  `(E)`/`(Europe)`/`(PAL)` region tags on any entry either), which turns out to matter a lot below:
  every currently-open gap in this project's corpus is a Japan-exclusive or PAL-region title, and
  none of them are in this collection.

Status legend: ✅ available and in use · 🟡 available locally (Dropbox) but not staged · ❌ not
available anywhere on this machine, needs sourcing.

## Mapper modes

| Mapper | Best ROM | Availability | Notes |
|---|---|---|---|
| LoROM | *(any LoROM title)* | ✅ `tests/roms/external/commercial/LoRom/None/` (25 titles: Super Mario World, Super Metroid, Zelda: A Link to the Past, Contra III, Super Castlevania IV, ...) | Extremely well covered; also every LoROM coprocessor board below carries its own LoROM-mapped dumps. |
| HiROM | *(any HiROM title)* | ✅ `tests/roms/external/commercial/HiRom/None/` (25 titles: Chrono Trigger, Final Fantasy VI, EarthBound, Secret of Mana, Donkey Kong Country, ...) | Equally well covered. |
| ExHiROM | Tales of Phantasia, Dai Kaijuu Monogatari II | ❌ neither present in `tests/roms/external/commercial/` nor in the Dropbox set (both are Japan-only) | No ExHiROM-specific board/golden test currently exists in this project either — a real, not-yet-scoped gap beyond just "no ROM." |
| ExLoROM | *(no known commercial or homebrew ROM exists)* | ❌ **unfillable** — `docs/cart.md`'s own provenance note states this plainly: "No real ExLoROM ROM (commercial or homebrew) exists in this project's local corpus," and per ares/bsnes source (`ref-proj/ares/mia/medium/super-famicom.cpp`), ExLoROM is an *unofficial* mode with no dedicated header value at all — real carts using this layout report plain LoROM's `$20` in their header. | This is not a "go find the ROM" gap — it may be that no shipped SNES cartridge ever actually used a true ExLoROM board in the way the formula models (bsnes's own `EXLOROM`/`EXLOROM-RAM` board entries in `boards.bml` exist for completeness/emulation-family compatibility, not because a specific verified commercial title is known to need them). Formula-level unit tests in `board.rs` are the only verification and likely the only verification that will ever exist for this mode. Stays honestly deferred. |

## Region (NTSC / PAL)

| Category | Best ROM | Availability | Notes |
|---|---|---|---|
| NTSC golden-boot proof | *(any NTSC title)* | ✅ every commercial dump in the corpus is NTSC (US or JP) | Default region path is exhaustively covered by the existing golden suites. |
| PAL golden-boot proof | *(any confirmed PAL-region SNES dump)* | ❌ not available — `tests/roms/external/commercial/` has no PAL dumps, and the Dropbox collection has no explicit region tagging on any filename (sampled the full 500-entry listing; no `(E)`/`(Europe)`/`(PAL)` suffix anywhere, meaning even titles that *exist* in both NTSC and PAL releases can't be distinguished from this collection alone without opening each file and reading the header). | `docs/STATUS.md` names this gap plainly: "PAL ... lack[s] golden-ROM-boot proof (no ROM in the local corpus)." A PAL-region SNES/Super Famicom dump would need to be sourced from a set that explicitly tags region (e.g. a No-Intro-style set with `(Europe)` suffixes), since this Dropbox set cannot supply one with confidence. Any widely-released PAL title works for this purpose (the 262→312 line-count / 50 Hz path is region-generic, not title-specific) — a well-known one like *Super Mario World (Europe)* or *Super Metroid (Europe)* would be the natural first pick once a properly-tagged dump is available. |

## Coprocessors

| Coprocessor | Tier | Best ROM(s) | Availability | Notes |
|---|---|---|---|---|
| None (base LoROM/HiROM) | — | see Mapper modes above | ✅ | — |
| DSP-1 | Core | Pilotwings, Super Mario Kart, Ace wo Nerae! | ✅ `LoRom/DSP-1/` (4 titles) + `HiRom/DSP-1/` (13 titles, incl. multiple Mario Kart romhacks) | Best-covered coprocessor in the corpus by title count. |
| DSP-2 | BestEffort | Dungeon Master | ✅ `LoRom/DSP-2/Dungeon Master.sfc` | Only known DSP-2 title; already the one in use. |
| DSP-3 | — (no board wired) | SD Gundam GX | ❌ not in `tests/roms/external/commercial/` (no `DSP-3/` subdirectory exists at all) nor in Dropbox (Japan-only release, confirmed via web search — the *only* commercial title using DSP-3) | `docs/STATUS.md`: "DSP-3 ... [has] no board wired (no verified board/window entry to pin against)." Since SD Gundam GX is genuinely the *only* DSP-3 title ever released, this ROM is required to validate any future DSP-3 board implementation — there is no substitute title. |
| DSP-4 | BestEffort | Top Gear 3000 | ✅ `LoRom/DSP-4/Top Gear 3000.sfc` | Only known DSP-4 title (besides a licensed variant); already in use. |
| SA-1 | Core | 18 titles incl. Super Mario RPG, Kirby Super Star, SD F-1 Grand Prix | ✅ `LoRom/SA-1/` (18 dumps, several with duplicate/romhack variants) | Best-covered *board-complexity* coprocessor — already drives the `sa1_oncart` golden suite (`tests/golden/sa1-framebuffer.tsv`). |
| Super FX / GSU-1 | Core | Star Fox, Star Fox 2, Stunt Race FX, Vortex | ✅ `LoRom/GSU-1/` (8 files incl. Star Fox 2, a `.blu`/`.ram` pair, and an "Exploration Showcase" homebrew build) + `HiRom/GSU-1/Wonder Project J` (2 files) | Well covered; also backed by the 58-ROM Krom GSU test suite (`tests/roms/external/krom/CHIP/GSU/`) for per-opcode + framebuffer golden coverage, independent of any commercial title. |
| Super FX / GSU-2 | Core | Super Mario World 2: Yoshi's Island, Doom | ✅ `LoRom/GSU-2/` (9 files, several Yoshi's Island variants/hacks) | Well covered. |
| S-DD1 | BestEffort | Star Ocean, Street Fighter Alpha 2 | ✅ `LoRom/S-DD1/` (2 titles) | Both known major S-DD1 titles present. |
| OBC1 | BestEffort | Metal Combat: Falcon's Revenge | ✅ `LoRom/OBC1/` (2 copies, formatting variants) | Only known OBC1 title; already in use. |
| CX4 | Core | Mega Man X2, Mega Man X3 | ✅ `LoRom/CX4/` (5 files incl. JP "Rockman" originals + an X3 romhack) | Well covered. |
| ST010 | BestEffort | F1 ROC II: Race of Champions | ✅ `LoRom/ST010/` (2 copies) | Only known ST010 title (ST010/ST011 share the same physical chip family; ST010 is the arithmetic-only variant). Already in use. |
| ST011 | — (no board wired) | Hayazashi Nidan Morita Shougi | ❌ not in the corpus, not in Dropbox (Japan-only release, confirmed) | `tests/roms/external/firmware/st011.rom` firmware **is** present, but there's no cart to pair it with — the firmware alone can't prove a board implementation boots real content. `docs/STATUS.md` lists ST011 among the boards with no verified board/window entry. |
| ST018 | — (no board wired) | Hayazashi Nidan Morita Shougi 2 | ❌ not in the corpus, not in Dropbox (Japan-only release, confirmed) | Same situation as ST011: `tests/roms/external/firmware/st018.rom` is present but unpaired. Per the earlier `st018-armv3-scoping` investigation (see project memory), the port source (Mesen2's `ArmV3Cpu`) is scoped but implementation hasn't started — this ROM gap doesn't block that scoping work, only the eventual golden-boot proof. |
| S-RTC | — (no board wired) | Daikaijuu Monogatari II | ❌ not in the corpus, not in Dropbox (Japan-only release, confirmed) | `docs/STATUS.md`: "S-RTC ... [has] no board wired (no verified board/window entry)." This is the *only* commercial S-RTC title — no substitute exists. |
| SPC7110 | BestEffort | Tengai Makyou Zero (Far East of Eden Zero), **genuine original-cartridge dump specifically** (sha256 `69d06a3f3a4f3ba769541fe94e92b42142e423e9f0924eab97865b2d826ec82d`, 5 MiB); Momotarou Dentetsu Happy (secondary title) | ❌ Tengai Makyou Zero: `HiRom/SPC7110/Tengai Makyou Zero.sfc` **is present but is the English fan-translation ROM hack, not the original cartridge** (confirmed by SHA256 mismatch + a public forum thread on the patch's memory map — `docs/audit/spc7110-boot-crash-2026-07-08.md`); the patch adds a "Expansion ROM" region no real cartridge has, which is what the boot-crash investigation was actually hitting · ❌ Momotarou Dentetsu Happy: not in the corpus, not in Dropbox | This is now the single highest-value ROM gap in this table: a genuine original-cartridge dump would very likely let SPC7110 boot cleanly (every fix landed through `v0.8.0` — the `bus_mirror` addressing, DCU/ALU trigger timing, `$40-$7D` mapping, DROM sizing, and the systemic open-bus fix — is independently verified and none is fan-translation-specific), moving it from "unit-test only" to real-title-validated alongside DSP-2/DSP-4/ST010/S-DD1/CX4/OBC1. |

## Real-title validation gaps (board works on synthetic/unit tests, but no commercial title has confirmed it)

| Feature | Best ROM | Availability | Notes |
|---|---|---|---|
| Hi-res (Modes 5/6) color-math precision | Bishoujo Janshi Suchie-Pai | ❌ not in the corpus, not in Dropbox (Japan-only, adult-audience mahjong title) | `docs/ppu.md` §Hi-res: "Bishoujo Janshi Suchie-Pai ... has no local dump." |
| Hi-res (Modes 5/6), alternate title | Marvelous: Mouhitotsu no Takarajima | ✅ `LoRom/SA-1/Marvelous - Mouhitotsu no Takarajima.sfc` (already dumped) | Already available and already tried: run for 1200 frames (20s) from power-on and never observed entering hi-res mode (`docs/ppu.md`). Either the hi-res content needs further input/progress this headless run didn't provide, or the "relies on hi-res" premise needs re-confirming against this specific title — this is a *methodology* gap now, not a missing-ROM gap. |

## Test-suite / homebrew corpora (non-commercial, already comprehensive)

These categories don't have a "best ROM" gap — they're purpose-built, licensed test-ROM suites
already fully staged:

| Category | Location | Coverage |
|---|---|---|
| 65C816 per-opcode oracle | `tests/roms/external/65816-singlestep/v1/`, committed `tests/roms/spc700-singlestep/` | SingleStepTests JSON vectors, all 256 opcodes × addressing modes. |
| SPC700 per-opcode oracle | `tests/roms/external/spc700-singlestep-full/`, `tests/roms/spc700-singlestep/` | Same SingleStepTests family for the SMP. |
| CPU/PPU/DMA/HDMA on-cart | `tests/roms/gilyon/`, `tests/roms/undisbeliever/` (both committed, permissive) | gilyon `cputest`/`spctest` (1107 assertions), undisbeliever's 29 PPU/DMA/HDMA hardware-behavior golden ROMs. |
| Audio boot+run | `tests/roms/external/blargg-spc/` | blargg `spc_smp`/`spc_timer`/`spc_mem_access_times`/`spc_dsp6`. |
| Super FX/GSU opcode + framebuffer | `tests/roms/external/krom/CHIP/GSU/` | 58 ROMs across `2BPP`/`4BPP`/`8BPP` × `128`/`160`/`192` × `FillPoly`/`PlotLine`/`PlotPixel`, plus per-opcode `GSUTest` ROMs. |
| General PPU/HDMA/Mode 7/interlace homebrew | `tests/roms/external/krom/PPU/`, `BANK/`, `CPUTest/`, `INPUT/`, `MSU/`, `Compress/`, `Translate/` | Broad homebrew coverage for bank-crossing, mosaic, windows, Mode 7, interlace, HDMA variants, MSU-1 audio/video, LZ77 decompression, and ROM-hacking-adjacent translate-table tooling. |
| 240p test suite | `tests/roms/external/240p/SNES-source/` | Display-timing/geometry reference (source form, not a prebuilt ROM). |

## Legitimate sourcing leads (`v1.1.0` research pass — leads only, nothing staged)

For each `❌` gap above, concrete legitimate leads a developer could manually pursue outside this
repo (never add the resulting ROM to `tests/roms/external/commercial/` if it's not already
permissively licensed — this table is a research pointer, not an instruction to acquire anything on
this project's behalf):

| Gap | Leads |
|---|---|
| SPC7110 genuine dump (sha256 `69d06a3f...ec82d`) | The nesdev.org fan-translation thread already cited in `docs/audit/spc7110-boot-crash-2026-07-08.md` documents the patch's non-standard memory map precisely enough to positively rule out a mismatched candidate. A No-Intro-style verified-dump database entry (e.g. via [No-Intro.org](https://no-intro.org/)'s published DAT files, or the [No-Intro ROM set index on Archive.org](https://archive.org/details/no-intro_romset_collection)) lists the canonical hash for cross-referencing a candidate dump's sha256 before use — verify independently against the target hash above, never trust a filename/label alone. |
| DSP-3 (SD Gundam GX), S-RTC (Daikaijuu Monogatari II), ST011/ST018 (both Hayazashi Nidan Morita Shougi titles) | **Not homebrew-substitutable** — each chip's internal firmware/algorithm (shogi/chess AI, RTC hardware) can only be exercised by its one sole commercial title. Checked [absindx/SNES-TestRoms](https://github.com/absindx/SNES-TestRoms) and [ARM9/snesdev](https://github.com/ARM9/snesdev): both cover SA-1/DSP-1/Super FX test patterns but neither has anything for DSP-3/S-RTC/ST01x. The only legitimate path remains a verified-good-dump database entry (No-Intro/Archive.org) for each specific Japan-exclusive title — a standing sourcing gap, not a near-term unblock. |
| PAL (any region-tagged title) | Any No-Intro-tagged `(Europe)` SNES set entry works — the 262→312-line/50 Hz path is region-generic, not title-specific, so a well-known PAL title (e.g. *Super Mario World (Europe)*) is sufficient once a properly region-tagged dump is available. |
| ExLoROM | Reconfirmed this pass — no commercial or homebrew ROM is known to use it; the ares/bsnes board database agrees. Likely permanently formula-verified-only, not a real sourcing gap. |
| ExHiROM (Tales of Phantasia, Dai Kaijuu Monogatari II) | Same No-Intro/Archive.org path as the Japan-exclusive coprocessor titles above — no homebrew substitute exists for this mapper mode either, since it's purely an address-decode formula (easiest to verify via `board.rs` unit tests plus one real dump for corroboration). |
| SA-1 (already well-covered, supplementary corroboration only) | [absindx/SNES-TestRoms](https://github.com/absindx/SNES-TestRoms) (`SA1RamProtectionTest` and siblings) and [VitorVilela7/SnesSpeedTest](https://github.com/VitorVilela7/SnesSpeedTest) are free, permissively-distributed homebrew SA-1 timing/RAM-protection test ROMs beyond the 18 commercial carts already staged — directly relevant corroboration for `v1.1.0`'s open-bus-via-HDMA-latch and DRAM-refresh timing work (both touch `advance_master`, which SA-1's board also exercises heavily). Worth adding as supplementary Core-tier coverage in a future pass, not a blocker for either timing item. |

## Summary of genuinely unfillable gaps

Every currently-documented "no ROM" gap in this project traces to a **Japan-exclusive or
PAL-region title that simply isn't in this machine's Western-market-focused Dropbox collection** —
none are fillable by staging something already sitting locally. In descending order of how much
they block real work:

1. **PAL golden-boot proof** — blocked on *any* PAL-tagged dump, not a specific title; needs a
   properly region-tagged ROM set, not a specific rare cart.
2. **DSP-3 (SD Gundam GX), ST011/ST018 (both Hayazashi Nidan Morita Shougi games), S-RTC
   (Daikaijuu Monogatari II)** — each is the *sole* commercial title using that coprocessor, all
   Japan-exclusive. No board work can reach golden-boot validation without one of these specific
   dumps.
3. **ExHiROM** (Tales of Phantasia, Dai Kaijuu Monogatari II) and **hi-res's named title**
   (Bishoujo Janshi Suchie-Pai) — same shape: Japan-exclusive, no substitute.
4. **ExLoROM** — not really a missing-ROM problem; per `docs/cart.md`'s own research, no verified
    commercial cartridge is known to require this exact unofficial layout. Likely stays
    permanently formula-verified-only.
5. **SPC7110's primary title, genuine dump** (Tengai Makyou Zero, sha256 `69d06a3f3a4f3ba7695
   41fe94e92b42142e423e9f0924eab97865b2d826ec82d`) — **re-classified, `v0.9.0`: this is now the
   highest-priority ROM gap in this whole table, not a lower-priority one.** The copy already in
   the corpus is the English fan-translation, not the original cartridge (confirmed by SHA256
   mismatch, a checksum-size inconsistency, and a public forum thread on the patch's own memory
   map — `docs/audit/spc7110-boot-crash-2026-07-08.md`); the previously-tracked "boot crash" was
   the emulator correctly declining to support the patch's non-standard `$40-$4F` "Expansion ROM"
   region, not a bug against real hardware. **SPC7110's second title** (Momotarou Dentetsu Happy)
   remains lower priority — a genuine dump of the primary title is very likely sufficient to close
   the boot gap on its own.
