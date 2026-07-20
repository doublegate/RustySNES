# AccuracySNES — Hardware-Behavior Research & Test-List Design Corpus

Research corpus seeding the design of "AccuracySNES", a single-cart SNES hardware-accuracy test
ROM modeled on the NES `AccuracyCoin` ROM. Sources fetched live 2026-07-19.

**Status: DRAFT — sections filled as research agents report.**

---

## 0. The AccuracyCoin model (what we are cloning)

Source: <https://github.com/100thCoin/AccuracyCoin>

- **141 tests**, menu-driven, spread over **20 pages**.
- Targets one canonical hardware revision (NTSC RP2A03G / RP2C02G); tests that cannot pass on
  another revision are **auto-skipped** rather than reported as failures.
- Navigation: D-Pad to move, **A** runs one test, **B** marks a test skipped. Cursor on the page
  header: **A** runs the whole page, **Start** runs the entire ROM and emits a results table.
- Result granularity: per-test **PASS/FAIL plus a hexadecimal error code** identifying *which*
  sub-assertion failed. Tests with several hardware-legal outcomes print a **light-blue variant
  number** saying which legal behavior was observed.
- **Select** opens a debug menu exposing the RAM cells the test used.

**Design implications for AccuracySNES:**
1. One self-contained `.sfc`, no external corpus, no host-side harness required.
2. Per-test hex error codes are essential — "FAIL" alone is useless for an emulator author.
3. The "multiple legal outcomes" concept maps directly onto SNES chip revisions
   (5A22 v1/v2, PPU2 v1/v2/v3, 1CHIP vs 3CHIP) and onto NTSC/PAL.
4. Auto-skip must key off `$4210` bits 3-0 (CPU version), `$213E` bits 3-0 (PPU1 version),
   `$213F` bits 3-0 (PPU2 version), and `$213F` bit 4 (50/60 Hz).
5. A machine-readable results dump (a known WRAM block + a screen hash) so an emulator CI can
   assert on it headlessly — AccuracyCoin lacks this and it is the single biggest usability
   gap for emulator projects.

---

## 1. Global reference: the SNESdev errata list

Source: <https://snes.nesdev.org/wiki/Errata> — this page is effectively a pre-made test list.
Every entry below is a candidate test. Grouped as the wiki groups them.

### Video
- Offset-per-tile **never affects the leftmost tile**.
- Color math on sprites applies **only to sprites using the last four palettes** (palettes 4-7).
- Enabling NMI via `NMITIMEN` while the `RDNMI` flag is already set fires an **immediate NMI**,
  possibly outside vblank.
- Sprite overflow drops **high-priority** slivers, not low-priority ones.
- **Time Over flag bug**: erroneously set when the first sprite is 16x16/32x32/64x64 with X in
  0-255 while other sprites have negative X.
- **Sprite X = -256 ($100)** still counts toward the 32-sprite limit and all its slivers count
  toward the 34-sliver limit.
- `INIDISP` ($2100) brightness is **not instant** — may fade over 72+ pixels on a 1CHIP.
- `INIDISP` **early-read bug**: the PPU reads an incorrect bus value before the correct one.
- 16x32 / 32x64 sprites do not work with OBJ interlacing; vertical flip flips the halves
  independently.

### Audio — S-SMP
- 16-bit writes to `$2140`/`$2141` may **also write `$2143`**.
- Simultaneous read/write on `$2140-$2143` can produce incorrect data.
- `TEST` register `$F0` writes can crash the SPC700, disable ARAM access, or halt timers.

### Audio — S-DSP
- Release rate is fixed; custom release must be emulated with GAIN.
- **Race** when changing ADSR/GAIN mode mid-note — write ADSR2/GAIN *before* ADSR1.
- Noise output is **highpass-filtered** as a consequence of the 15-bit LFSR interpretation.
- `EDL = 0` continuously overwrites **4 bytes at the echo buffer start**; set `FLG` bit 5 to
  protect.
- `EDL` writes take effect only at the echo buffer end — up to **7680 samples / 240 ms** delay.
- `ESA` writes can be delayed by one sample.
- Echo buffer **wraps at a 16-bit boundary**, potentially corrupting page zero.
- **KON/KOFF are polled every second sample**; clearing too early prevents key-on/off.
- Three overflow bugs: BRR decoder clipping, FIR clipping on the first 7 taps, and Gaussian
  interpolation overflow with **three consecutive maximum-negative samples**.

### Audio — SPC700 core
- `TSET1`/`TCLR1` perform an **equality test**, not a bit test.
- `MUL` flags are based **only on Y** (the high byte).
- `DIV` output is only valid if the quotient <= 511.
- `DIV` flags are based **only on A** (bits 0-7).

### Mode 7 multiplier
- `MPY` result is corrupted if an interrupt or HDMA writes **BG1 scroll or Mode 7 matrix
  registers** between the two `M7A` writes — they share a latch.

### 65C816
- Setting index registers to 8-bit (`SEP`/`PLP`/`XCE`) **clears the high byte of X and Y**.
- `JMP (addr)` / `JMP [addr]` read the pointer from **bank $00**.
- `JMP (addr,X)` / `JSR (addr,X)` read from the **program bank**.
- `MVN`/`MVP` change DB to the **destination** bank.

### S-CPU (5A22)
- Starting a multiply (`$4203`) or divide (`$4206`) while the previous one is in flight yields
  **erroneous `RDDIV`/`RDMPY`**.
- **Invalid DMA A-bus addresses**: any `$21xx`, `$4000-$41FF`, `$4200-$421F`, `$4300-$437F`, and
  WRAM as A-bus source when B-bus is `WMDATA` ($2180).
- **5A22 v1**: crash if a DMA finishes just before HDMA.
- **5A22 v2**: a recent HDMA to `INIDISP` prevents a subsequent DMA from completing.
- HDMA fails **for the whole frame** if a DMA ends at the start of scanline 0 and the previous
  read value was 0.
- Enabling HDMA outside vblank causes erroneous PPU writes from incorrect table addresses.

### Input
- Automatic controller reading begins between **H = 32.5 and H = 95.5** of the first vblank
  scanline.
- Auto-read results may change during **lag frames**.

### Hardware
- The cartridge **/RESET pin resets only the S-CPU, APU, and S-WRAM — not the PPU.**

---

## 2. Behavior-to-game motivation table

Source: <https://snes.nesdev.org/wiki/Tricky-to-emulate_games>. Every row is a real commercial
title that breaks when the behavior is wrong — i.e. a test with proven real-world stakes.

| Behavior | Titles |
|---|---|
| CPU/PPU open bus | Captain America and the Avengers, The Combatribes, Home Alone, Rock n' Roll Racing, Super 3D Noah's Ark |
| BRK/COP implementation | Actraiser, Sailor Moon Another Story, Cybernator, Dekitate High School, Illusion of Gaia, Kamaitachi no Yoru, Soul Blazer |
| `ORA [d]` | Super Mario World |
| OAM priority rotation | Super Mario World |
| VRAM write during rendering | Hook |
| VRAM address increment during rendering | Kick Off |
| VRAM read behavior | Breath of Fire |
| Offset-per-tile wraparound | Super Famista 5 |
| Offset-per-tile general | Axelay, Chrono Trigger |
| Mode 7 window logic | The Atlas: Renaissance Voyager, MechWarrior |
| Mode 7 scroll offset latch timing | NHL '94 |
| Mode 7 direct color | Aerobiz |
| Color window + DMA B-bus register access | Krusty's Super Fun House |
| Color math on subscreen in hi-res | Jurassic Park |
| OAM write timing during rendering | Uniracers |
| OAM fetch/render timing | Mega lo Mania |
| DMA power-on state | Heian Fuuunden |
| CPU cycle timing before DMA start | MMPR: The Fighting Edition |
| DMA/HDMA timing | Circuit USA, Jumbo Ozaki no Hole in One |
| HDMA fixed-transfer flag | Batman Forever, The Lost Vikings |
| HDMA decrement flag | The Adventures of Kid Kleets |
| HDMA direction flag + NMI enable timing | Pocky & Rocky |
| HDMA transfer-flag state management | Aladdin, Super Ghouls'n Ghosts |
| DMA suspension during HDMA | Dekitate High School |
| V-IRQ trigger conditions | RoboCop versus The Terminator |
| NMI during vblank | Alien vs Predator |
| NMI vector execution timing | Jaki Crush |
| SPC cycle-level timing | ActRaiser 2, Hiouden, Tales of Phantasia, Illusion of Gaia |
| CPU read effect timing | Rendering Ranger R2 |
| DSP `KOF` register init | Chester Cheetah, King of Dragons |
| SRAM mapping | Fire Emblem: Thracia 776, Ys III |
| RAM power-on state | Death Brade, Power Drive, Sailor Moon Another Story |
| SRAM power-on state | Super Keiba 2 |
| Super FX `RPIX` | Yoshi's Island |
| HDMA x open bus | Speedy Gonzales: Los Gatos Bandidos |

Uncommon-mode coverage targets (<https://snes.nesdev.org/wiki/Uncommon_graphics_mode_games>):
Mode 0 (Super Mario Kart driver select, FFIV/FFV menus, Yoshi's Island title), Mode 3 8bpp
(DKC logo, Secret of Mana title, SimCity 2000), Mode 4 8bpp (Bust-a-Move, Rock'n'Roll Racing),
Mode 7 EXTBG (Contra 3 L2, Super Ghouls'n Ghosts, Super Turrican 2), Direct Color (Actraiser 2,
Aerobiz, Secret of Mana world map), hi-res (Jurassic Park, Chrono Trigger, DKC), Mode 5 hi-res
Japanese text (Seiken Densetsu 3, Rudra no Hihou, Marvelous), horizontal OPT (Chrono Trigger
Black Omen), vertical OPT (Axelay L2, Star Fox, Tetris Attack, Yoshi's Island),
overscan/239-line (Dragon Quest I&II, Rendering Ranger R2, PAL SMW).

---

---

# PART II — THE PROPOSED TEST LIST

Numbering scheme: `<SUBSYSTEM><page>.<test>`. Each test is one menu entry with a hex error code
per sub-assertion. Counts at the end of each section.

---

## A. 65C816 CPU — addressing, wrapping, flags (target ~55 tests)

### A1. Emulation vs native mode
| # | Test | Assertion | Source |
|---|---|---|---|
| A1.01 | XCE into emulation | `SEC; XCE` forces m=1, x=1, SH=$01, XH=$00, YH=$00 | 6502.org 65c816opcodes |
| A1.02 | XCE into native | `CLC; XCE` changes nothing except E — m/x stay 1 | same |
| A1.03 | REP ignored in emulation | `REP #$30` while E=1 leaves m=x=1 | same |
| A1.04 | Index narrowing destroys XH/YH | `LDX #$1234; SEP #$10; REP #$10; TXA` → XH=$00 | **Errata** |
| A1.05 | Same via PLP and via XCE | both paths clear XH/YH identically | Errata |
| A1.06 | `TXS` in emulation | X=$FF → S=$01FF not $00FF | 6502.org |
| A1.07 | `TCS` in emulation | A=$1234 → S=$0134 (SH forced $01) | 6502.org |
| A1.08 | `TCD`/`TSC`/`TDC` always 16-bit | unaffected by m | 6502.org |
| A1.09 | `TCS`/`TXS` set no flags | all other transfers set N/Z | 6502.org |

### A2. Direct-page wrapping
| # | Test | Assertion |
|---|---|---|
| A2.01 | `d,X` never crosses bank | `D=$8001, X=$FFFE, LDA $01,X` → reads `$00:8000`, NOT `$01:8000` (superfamicom.org worked example) |
| A2.02 | Emulation + DL=$00 page wrap | `E=1, D=$0000, X=$01, LDA $FF,X` → `$00:0000` |
| A2.03 | Emulation + DL≠$00 no wrap | `E=1, D=$0010, X=$01, LDA $FF,X` → `$00:0110` |
| A2.04 | Native always carries | `E=0, D=$0000, X=$01, LDA $FF,X` → `$00:0100` |
| A2.05 | 16-bit `d` read across page | native word read at D+$FF carries to next page |
| A2.06 | `[dp]` is a "new" mode | `E=1, D=$0000, dp=$FF` → pointer bytes from $0000FF/$000100/$000101, no wrap |
| A2.07 | `(dp),Y` bank carry | adding Y to the loaded pointer always crosses banks |
| A2.08 | `[dp],Y` bank carry | same |
| A2.09 | `(dp,X)` inherits d,X rules | emulation page-wrap + never-cross-bank |
| A2.10 | `PEI (dp)` no page wrap | new mode, even at DL=$00/E=1 |

**Note:** A2.05 is documented by superfamicom.org only as "theoretically" — an explicitly
*unverified* behavior. AccuracySNES resolving this is a genuine contribution to the public record.

### A3. Stack wrapping
| # | Test | Assertion |
|---|---|---|
| A3.01 | Emulation stack page wrap | `E=1, S=$01FF, PLA` → pulls `$00:0100` |
| A3.02 | `PEA` escapes $01xx | `E=1, S=$0100, PEA $1234` → writes $00:0100 and $00:00FF, S=$01FE, $01FF untouched |
| A3.03 | `PLD` escapes $01xx | `E=1, S=$01FF, PLD` → reads `$00:0200/0201` |
| A3.04 | `PLY` does NOT escape | same S → reads `$00:0100`. **A3.03 vs A3.04 is the old-vs-new discriminator** |
| A3.05 | `d,S` escapes | `E=1, S=$01FF, LDA $02,S` → `$00:0201` |
| A3.06 | `(d,S),Y` escapes + bank carry | |
| A3.07 | `JSL`/`RTL` escape | |
| A3.08 | `JSR (a,X)` escapes | |
| A3.09 | `PHD`/`PER` escape | |
| A3.10 | Stack confined to bank $00 | native `S=$0000; PHA` → `$00:0000`, next push wraps to `$00:FFFF` |

### A4. Absolute / long / jump wrapping
| # | Test | Assertion | Source |
|---|---|---|---|
| A4.01 | NMOS JMP bug is FIXED | `JMP ($12FF)` reads high byte from `$00:1300` | Errata |
| A4.02 | `JMP (a)` pointer from bank $00 | destination offset lands in current PBR | **Errata** |
| A4.03 | `JML [a]` pointer from bank $00 | full 24-bit destination from the pointer | Errata |
| A4.04 | `JMP (a,X)` from program bank | PBR=$05, X=$04, `JMP ($FFFE,X)` → pointer at `$05:0002` (wraps in bank) | **Errata** |
| A4.05 | `JSR (a,X)` from program bank | same rule | Errata |
| A4.06 | `abs,X` bank carry | `DBR=$00, X=$80, LDA $FFC0,X` → `$01:0040` (6502 would wrap) |
| A4.07 | 16-bit abs read bank carry | `m=0, LDA $FFFF` → low from `$00:FFFF`, high from `$01:0000` |
| A4.08 | `long,X` bank carry | |
| A4.09 | PC wraps within bank | operand crossing `$xx:FFFF` fetches from `$xx:0000` |
| A4.10 | Branch target wraps within bank | **`r`/`rl` wrap is marked "XXX: untested" upstream — new ground** |

### A5. Cycle counts
| # | Test | Assertion |
|---|---|---|
| A5.01-08 | Base cycle sweep | all 256 opcodes at m=1,x=1,e=0,DL=$00, no page cross (8 tests, 32 opcodes each) |
| A5.09 | `+1 m` sweep | every m-dependent opcode under `REP #$20` |
| A5.10 | `+1 x` sweep | every x-dependent opcode under `REP #$10` |
| A5.11 | `+1 w` (DL≠0) sweep | every DP opcode at D=$0000 vs D=$0001 → exactly +1. **The most commonly mis-implemented penalty** |
| A5.12 | `+1 p` read page cross | `LDA $00FF,X` X=$00 vs X=$01 |
| A5.13 | Store has NO p penalty | `STA $00FF,X` identical both ways (always the higher count) — likewise `STA a,Y`, `STZ a,X`, `STA (d),Y` |
| A5.14 | RMW `abs,X` no p penalty | `ASL $1234,X` flat 7 |
| A5.15 | Branch formula | `2 + t + t*e*p`: E=0 taken cross = 3; E=1 taken cross = 4; E=1 taken no-cross = 3 |
| A5.16 | `BRL` flat 4 | never penalized |
| A5.17 | 16-bit RMW = +2 | `ASL/LSR/ROL/ROR/INC/DEC/TSB/TRB` at m=0 add **2**, not 1 (resolves an undisbeliever table error) |
| A5.18 | `BRK` 8 native / 7 emulation | `+1 e=0` |
| A5.19 | `RTI` 7 native / 6 emulation | |
| A5.20 | `MVN`/`MVP` = 7 cycles per byte | |
| A5.21 | Decimal mode adds ZERO cycles | `SED; ADC` == `CLD; ADC` (unlike 65C02) |
| A5.22 | `PHD`=4, `PLD`=5, `PEA`=5, `PEI`=6+w, `PER`=6, `REP`/`SEP`=3, `XBA`=3 | odd-count spot checks |

### A6. Interrupts
| # | Test | Assertion |
|---|---|---|
| A6.01 | Native vectors | COP `$FFE4`, BRK `$FFE6`, ABORT `$FFE8`, NMI `$FFEA`, IRQ `$FFEE` — all read from and jumped to in **bank $00** |
| A6.02 | Emulation vectors | COP `$FFF4`, ABORT `$FFF8`, NMI `$FFFA`, RESET `$FFFC`, IRQ/BRK `$FFFE` |
| A6.03 | Native push count | 4 bytes: PBR, PCH, PCL, P |
| A6.04 | Emulation push count | 3 bytes (no PBR) |
| A6.05 | B flag discriminates | emulation pushed P has B=1 for BRK, B=0 for hardware IRQ |
| A6.06 | **D cleared on ALL interrupts** | `SED; BRK` → handler sees D=0 (65C02/816 behavior the NMOS 6502 lacks) |
| A6.07 | I set, m/x unchanged | |
| A6.08 | BRK/COP are 2-byte | pushed PC = PC+2, RTI skips the signature byte |
| A6.09 | PBR=$00 in handler | |
| A6.10 | RTI must match mode | native RTI pulls PBR, emulation does not |
| A6.11 | `WAI` + IRQ with I=1 | resumes at the next instruction **without vectoring** |
| A6.12 | `WAI` wake latency | 1 cycle after the interrupt |
| A6.13 | `STP` halts until reset | |
| A6.14 | `WDM ($42)` = 2-byte NOP | |
| A6.15 | No undefined opcodes | all 256 defined; only STP hangs |

### A7. Decimal mode
| # | Test | Assertion |
|---|---|---|
| A7.01 | 8-bit BCD ADC | `SED; CLC; LDA #$09; ADC #$01` → A=$10, C=0 |
| A7.02 | 16-bit BCD ADC | `m=0; LDA #$0999; ADC #$0001` → A=$1000 |
| A7.03 | 8/16-bit BCD SBC | |
| A7.04 | N/Z valid in decimal | reflect the **BCD** result (65C02-like, unlike NMOS 6502) |
| A7.05 | V is meaningless | record as a golden vector, do **not** assert a spec value |

### A8. Block move
| # | Test | Assertion |
|---|---|---|
| A8.01 | Machine encoding | `$54 <dest> <src>` — **destination bank byte FIRST**, opposite of assembly syntax |
| A8.02 | Terminal state | A=$FFFF, DB = destination bank (**permanently**) |
| A8.03 | MVN index deltas | X=X0+N, Y=Y0+N; MVP: X=X0-N, Y=Y0-N |
| A8.04 | Independent bank wrap | X wraps within srcBank, Y within destBank |
| A8.05 | 8-bit index (E=1) | offsets confined to `$00xx` |
| A8.06 | Interruptible mid-block | NMI + RTI mid-MVN resumes correctly (**undocumented upstream — new ground**) |

### A9. Misc flags
| # | Test | Assertion |
|---|---|---|
| A9.01 | `BIT #imm` affects Z only | N and V untouched in immediate mode |
| A9.02 | `BIT abs` sets N=bit15/7, V=bit14/6 of memory, Z from A AND M | |
| A9.03 | `ORA [d]` correctness | the specific bug **Super Mario World** exposes |

---

## B. 5A22 bus, clock, and timing (target ~30 tests)

### B1. Memory access speed
Region table (master clocks per CPU cycle):

| Range | Clocks |
|---|---|
| `$00-$3F:$0000-$1FFF` WRAM mirror | 8 |
| `$00-$3F:$2000-$3FFF` B-bus (PPU/APU) | 6 |
| `$00-$3F:$4000-$41FF` JOYSER0/1 | **12** |
| `$00-$3F:$4200-$5FFF` CPU MMIO | 6 |
| `$00-$3F:$6000-$FFFF` | 8 |
| `$40-$7F` | 8 |
| `$80-$BF:$8000-$FFFF` / `$C0-$FF` | **6 if MEMSEL=1 else 8** |
| Internal (non-memory) CPU cycles | 6 |

| # | Test | Assertion |
|---|---|---|
| B1.01 | FastROM toggle | `$420D` bit 0: banks $80-$FF cost 6 vs 8 clocks |
| B1.02 | Joypad port is 12 clocks | `LDA $4016` measurably slower than `LDA $4200` |
| B1.03 | Internal cycles always 6 | regardless of PBR speed region |
| B1.04 | WRAM is 8, B-bus is 6 | |
| B1.05 | DMA is 8 clocks/byte | **independent of source/destination region** |

### B2. Scanline / frame geometry
| # | Test | Assertion |
|---|---|---|
| B2.01 | 1364 clocks per normal line | 340 dots, dots **323 and 327 are 6 master cycles**, all others 4 |
| B2.02 | Short scanline | scanline **$F0 (240)** on alternating non-interlace frames = **1360** clocks / 340 dots |
| B2.03 | Long scanline | PAL interlace field=1, V=311 = **1368** clocks / 341 dots |
| B2.04 | NTSC frame = 262 lines / 357,368 clocks | period alternates 357,368 / 357,364 |
| B2.05 | PAL frame = 312 lines / 425,568 clocks | |
| B2.06 | Interlace adds a line | 263 NTSC / 313 PAL on frames where `$213F.7 = 0` |
| B2.07 | NTSC rate = 60.0988 Hz | 21,477,272.7 / 357,366 |
| B2.08 | PAL rate = 50.00698 Hz | |
| B2.09 | Picture window | left edge at clock 88, right at clock 1112 |
| B2.10 | Region flag | `$213F` **bit 4** = 0 NTSC / 1 PAL (**not bit 3** — the SNESdev PPU_registers page has this wrong; fullsnes is right) |

### B3. DRAM refresh
| # | Test | Assertion |
|---|---|---|
| B3.01 | 40-clock stall | CPU paused **40 master cycles** per scanline, leaving 1324 active |
| B3.02 | Refresh position | begins at cycle **538** on the first scanline of the first frame; thereafter at the multiple of 8 closest to 536 after the previous pause |
| B3.03 | Refresh is observable | a tight H-counter-timed loop shows the discontinuity |

### B4. Interrupt timing
| # | Test | Assertion |
|---|---|---|
| B4.01 | NMI assert point | internal timer drives /NMI low at **H = 0.5** at the start of vblank |
| B4.02 | V-Blank start line | V=$E1 (225) with `$2133.2=0`, V=$F0 (240) with overscan |
| B4.03 | RDNMI set point | `$4210` bit 7 set at V=225/240, HC=2 |
| B4.04 | RDNMI read-to-clear | reading `$4210` clears bit 7; a second read in the same vblank returns 0 |
| B4.05 | RDNMI auto-clears at end of vblank | |
| B4.06 | **NMITIMEN enable-while-pending fires immediately** | setting `$4200.7` while the RDNMI flag is already set triggers an NMI possibly outside vblank (**Errata**) |
| B4.07 | H-IRQ point | `$4200` bits 5-4 = `01` → every line at H = HTIME + ~3.5 |
| B4.08 | V-IRQ point | `10` → at V=VTIME, H ≈ 2.5 |
| B4.09 | HV-IRQ point | `11` → V=VTIME, H = HTIME + ~3.5 |
| B4.10 | **No IRQ at dot 153 on the short scanline** (non-interlace) | a genuinely obscure exception |
| B4.11 | **No IRQ at dot 153 on the last scanline of any frame** | |
| B4.12 | TIMEUP read-to-clear | reading `$4211` releases IRQ; so does disabling IRQs via `$4200` |
| B4.13 | HTIME range 0-339, VTIME 0-261/311 | |
| B4.14 | Interrupt poll point | check occurs **just before the final CPU cycle** of an instruction → handler entry ≥6-12 master cycles after assertion |

### B5. Multiply / divide
| # | Test | Assertion |
|---|---|---|
| B5.01 | `$4202`/`$4203` unsigned 8x8→16 into RDMPY `$4216/17` | |
| B5.02 | `$4204/05`/`$4206` 16/8 → RDDIV `$4214/15` + remainder in RDMPY | |
| B5.03 | **Overlapping-operation race** | starting a new multiply/divide before the previous completes yields *erroneous* output. **Errata says the result is genuinely undefined — this test must report the observed value as a golden vector, never assert a "correct" one** |
| B5.04 | Power-on values | `$4202`=$FF, `$4204/05`=$FFFF |

---

## C. S-PPU1 / S-PPU2 (target ~85 tests — the largest section)

### C1. Port mechanics — OAM
| # | Test | Assertion |
|---|---|---|
| C1.01 | OAMADDR reload semantics | writing `$2102` **or** `$2103` copies the whole 9-bit reload into the address with bit0 forced 0. Anomie's example: set $104, write 4 bytes, write $1 to `$2103` → address is word **4**, not 6 |
| C1.02 | **Low-table write-twice latch** | addr 0, write $01, write $02, read `$2138`, write $03 → OAM = `01 02 01 03` (**not** `01 02 xx 03`) |
| C1.03 | High table commits immediately | addr > $1FF writes land per-byte, no pairing |
| C1.04 | OAM mirror | `$220-$3FF` mirror `$200-$21F` |
| C1.05 | Shared increment | `$2138` reads and `$2104` writes share one address counter |
| C1.06 | **OAM address reset at vblank** | internal address reloads from `$2102/3` at **H=10 on line 225 (or 240)**, only when force-blank is off |
| C1.07 | Reset on force-blank edge | any **1→0 transition of `$2100` bit 7** also triggers the reload |
| C1.08 | Address destroyed during render | `$2138` read mid-frame returns a position ≠ the programmed one |
| C1.09 | Priority rotation | `$2103` bit 7 set → first-priority sprite index = `(OAMAddr & $FE) >> 1`. **[Conflict: fullsnes says bits 6-1 of the register instead — resolve]** |

### C2. Port mechanics — VRAM
| # | Test | Assertion |
|---|---|---|
| C2.01 | Increment steps | VMAIN bits 1-0 → +1, +32, +128, **+128** (both `10` and `11` are 128) |
| C2.02 | Increment trigger bit | VMAIN bit 7 selects `$2118`/`$2139` vs `$2119`/`$213A`, symmetric across read and write |
| C2.03-05 | Address translation | all three remap rotations (8/9/10-bit) produce the exact documented permutations |
| C2.06 | **Remap affects the bus, not the register** | `$2116/7 = $0003` + remap 1 → access lands at word `$0018` while `$2116/7` increments to `$0004` |
| C2.07 | Bit 15 not connected | `$8000-$FFFF` alias `$0000-$7FFF` |
| C2.08 | **Prefetch on address write** | writing `$2116/17` prefetches 16 bits; the first `$2139/$213A` read returns stale data (the "read twice" requirement) |
| C2.09 | **Read order** | `$2139` returns the latch → refills the latch from VRAM → increments |
| C2.10 | **Out-of-window write is dropped but the address STILL increments** | the cleanest observable of the VRAM lock |
| C2.11 | VRAM accessible only in vblank/force-blank | **H-Blank does not work** |
| C2.12 | Force-blank 1→0 mid-frame closes the window immediately | |

### C3. Port mechanics — CGRAM and counters
| # | Test | Assertion |
|---|---|---|
| C3.01 | CGRAM inherits the OAM low-table latch rule | two-write commit |
| C3.02 | `$2121` write resets the 1st/2nd flipflop | |
| C3.03 | `$213B` 2nd read bit 7 = PPU2 open bus | |
| C3.04 | **CGRAM access during active display hits the color currently being drawn** | requires a dot-based renderer |
| C3.05 | `$2137` SLHV latches H/V | only when `$4201` bit 7 is set; **the value read is open bus** |
| C3.06 | `$213C/$213D` 2nd read | bit 0 = counter bit 8, **bits 7-1 = PPU2 open bus** |
| C3.07 | The two counter flipflops are independent | |
| C3.08 | **`$213F` read resets BOTH flipflops and clears the latch flag (bit 6)** | |
| C3.09 | `$213F` bit 7 toggles at **V=0, H=1** | |
| C3.10 | Superscope latch point | pointing at (X,Y) latches ≈ dot **X+40**, line **Y+1** |

### C4. Scroll registers
| # | Test | Assertion |
|---|---|---|
| C4.01 | H formula | `BGnHOFS = (Cur<<8) \| (Prev & ~7) \| ((Reg>>8) & 7)` |
| C4.02 | V formula | `BGnVOFS = (Cur<<8) \| Prev` |
| C4.03 | **Single shared `Prev` latch across BG1-BG4, H and V** | |
| C4.04 | Mode 7 latch is SEPARATE | |
| C4.05 | `$210D` writes both BG1HOFS and M7HOFS | mode selects which is consumed |

### C5. Backgrounds and modes
| # | Test | Assertion |
|---|---|---|
| C5.01-08 | Per-mode priority order | full front→back list for modes 0-7 including the Mode 1 `BGMODE.3` BG3-to-front case (8 tests) |
| C5.09 | Mode 0 palette segregation | BG1 CGRAM 0-31, BG2 32-63, BG3 64-95, BG4 96-127 |
| C5.10 | Tilemap entry decode | `vhopppcc cccccccc` |
| C5.11 | 16x16 tile assembly | uses +1, +16, +17 |
| C5.12 | BGnSC sizes | 1/2/3 place extra 32x32 maps right / below / both |
| C5.13 | BGnSC/BGnNBA ignored in Mode 7 | |
| C5.14-16 | Tile bitplane layout | 2bpp / 4bpp (two 2bpp halves) / 8bpp (four 2bpp groups) |
| C5.17 | Modes 5/6 use 16-px-wide tiles | |

### C6. Offset-per-tile (modes 2/4/6)
| # | Test | Assertion |
|---|---|---|
| C6.01 | Bit 13 → BG1, bit 14 → BG2 | |
| C6.02 | Mode 4 bit 15 selects H vs V | |
| C6.03 | H offsets keep the BG's low 3 HOFS bits | |
| C6.04 | V offsets replace VOFS entirely | |
| C6.05 | **The leftmost tile is NEVER affected** (Errata) | first entry controls the *second* visible column |
| C6.06 | Each entry affects a whole column | screen Y does not select the BG3 row |
| C6.07 | Wraparound behavior | the case **Super Famista 5** exposes |

### C7. Sprites
| # | Test | Assertion |
|---|---|---|
| C7.01 | 32-sprite range limit | 33 candidates → highest OAM index drops |
| C7.02 | **34-sliver limit, REVERSE evaluation** | slivers evaluated highest→lowest index, so the **lowest** index slivers drop first — i.e. hardware drops the *highest-priority* slivers (Errata) |
| C7.03 | Sliver order within a sprite | left-to-right on screen even when H-flipped; a mid-sprite cutoff drops the rightmost |
| C7.04 | **X = $100 (-256)** | fully offscreen yet consumes a range slot **and** all its slivers count against 34 |
| C7.05 | Range Over set point | `V = OBJ.YLOC, H = OAM.INDEX*2` |
| C7.06 | Time Over set point | `V = OBJ.YLOC+1, H = 0` |
| C7.07 | **Time Over false positive** | first sprite 16x16/32x32/64x64 at X=0-255 with other sprites at negative X (Errata) |
| C7.08 | Flags set regardless of `$212C` OBJ enable | |
| C7.09 | **Flags clear at end of vblank but NOT during forced blank** | |
| C7.10 | OBJSEL sizes 6 and 7 | undocumented 16x32/32x64 and 16x32/32x32 |
| C7.11 | Tile address formula | `((Base<<13) + (tile<<4) + (N ? ((Name+1)<<12) : 0)) & $7FFF` |
| C7.12 | **16x32 under OBJ interlace** | renders as 16x16, bottom half ignored, top squished to 16x8; 32x64 behaves correctly (Errata) |
| C7.13 | **V-flip on tall sizes flips each half independently** (Errata) | |
| C7.14 | Vertical wrap | 64-px sprites wrap bottom→top in 224-line mode; 32-px in 239-line mode |
| C7.15 | Lower OAM index always on top | the priority field only interleaves with BG layers |
| C7.16 | OAM write timing during render | the **Uniracers** case |

### C8. Color math and windows
| # | Test | Assertion |
|---|---|---|
| C8.01 | **Sprite color math only on palettes 4-7** | palettes 0-3 hard-wired off (Errata) |
| C8.02 | Channel clamp | results clamp to 0 and 31, no wrap |
| C8.03 | **Half/div2 ignored for the fixed backdrop** | `$2131` bit 6 has no effect when `$2130` bit 1 = 0 |
| C8.04 | Window bounds inclusive | `left <= X <= right` |
| C8.05 | `left > right` → empty | |
| C8.06 | Inverted + `left > right` → full screen | |
| C8.07 | Both windows disabled → **empty**, not full | |
| C8.08-11 | Mask logic ops | OR / AND / XOR / XNOR per layer |
| C8.12 | CGWSEL force-black field (7-6) | never/outside/inside/always |
| C8.13 | CGWSEL prevent-math field (5-4) | never/inside/outside/never — independent of 7-6 |
| C8.14 | Subtract mode `$2131` bit 7 | |
| C8.15 | COLDATA per-channel select | bits 7/6/5 gate B/G/R independently |
| C8.16 | Color window + DMA B-bus access | the **Krusty's Super Fun House** case |

### C9. Hi-res, pseudo-hires, interlace, overscan
| # | Test | Assertion |
|---|---|---|
| C9.01 | Pseudo-hires column mapping | even columns = subscreen, odd = main; subscreen is the **left** of each pair |
| C9.02 | **Pseudo-hires color-math inheritance** | a subscreen column copies the operation of the **previous main-screen pixel**, using that pixel's **pre-math** value as the operand |
| C9.03 | Hi-res scroll granularity | coarse 2-hires-pixel H steps; V scroll gets 1/480 fine resolution when interlaced |
| C9.04 | Overscan line count | `$2133` bit 2 → 224 vs 239; vblank start moves $E1 → $F0 |
| C9.05 | **Mid-frame overscan toggle** | set then clear between $E0 and $F0 → vblank events defer; setting too late leaves **VRAM locked as if still rendering**. Reproduce: `LDA #'-' / STA $2118 / LDA $2133 / STA $2133 / LDA #'+' / STA $2118` → only one byte lands |
| C9.06 | Screen interlace `$2133` bit 0 | doubles effective height in modes 5/6; jitter elsewhere |
| C9.07 | Modes 5/6 color math restriction | |
| C9.08 | Subscreen color math in hi-res | the **Jurassic Park** case |

### C10. Mosaic
| # | Test | Assertion |
|---|---|---|
| C10.01 | Applied after scrolling, before window/color math | |
| C10.02 | Blocks anchor to screen top-left, not to the scroll origin | |
| C10.03 | **Mid-frame `$2106` write re-anchors the mosaic start line to the current scanline** | |
| C10.04 | Mosaic 1x1 = 2x1 half-pixels in true hi-res | |
| C10.05 | Mode 7 BG2 uses bits A and B separately for V and H | |

### C11. Mode 7
| # | Test | Assertion |
|---|---|---|
| C11.01 | Matrix transform | the standard `[Tx,Ty] = M * [Sx+HOFS-X, Sy+VOFS-Y] + [X,Y]` form |
| C11.02 | **13-bit sign handling** | `ORG.X = (M7HOFS - M7X) AND NOT $1C00; if < 0 then OR $1C00` |
| C11.03 | **Fractional truncation** | each `M7x * ORG` product has its low 6 bits masked (`AND NOT $3F`) before accumulation |
| C11.04 | Screen-over modes | bit7=0 → clamp to 0..1023; bits 7+6 set → out-of-range uses low 3 bits of char 0 |
| C11.05 | VRAM layout | tilemap in low bytes, tiles in high bytes, first 16 KB, fixed |
| C11.06 | MPY = signed16(`$211B`) × signed8(`$211C`) | |
| C11.07 | **MPY latch corruption** | an interrupt or HDMA writing BG1 scroll or an M7 matrix register **between the two M7A writes** corrupts the product (shared latch, Errata) |
| C11.08 | MPY during active display | holds intermediate per-pixel rotation results |
| C11.09 | EXTBG | BG2 splits BG1 by the pixel's high bit into two priority layers |
| C11.10 | **Direct color unavailable on EXTBG BG2**, always available on Mode 7 BG1 | |
| C11.11 | Mode 7 window logic | the **Atlas / MechWarrior** case |
| C11.12 | Scroll latch timing | the **NHL '94** case |

### C12. Direct color
| # | Test | Assertion |
|---|---|---|
| C12.01 | Expansion | `RRRr0 GGGg0 BBb00` — pixel bits supply the high bits, tilemap attribute bits supply one extra per channel (blue gets 2+1) |
| C12.02 | **Pure black is unreachable** | pixel value 0 is always transparent |
| C12.03 | Available on Mode 3/4 BG1 and Mode 7 BG1 only | |

### C13. INIDISP and open bus
| # | Test | Assertion |
|---|---|---|
| C13.01 | **INIDISP early-read: object tile corruption** | write outside vblank with prior data-bus bit 7 set → sprite corruption on **3-chip only** |
| C13.02 | **INIDISP early-read: display flash** | force-blank on + prior bus bit 7 clear → display on for one dot |
| C13.03 | **INIDISP early-read: brightness glitch** | one-dot brightness step |
| C13.04 | Long-addressing workaround | `STA $8F2100` vs `STA $0F2100` produce different artifacts |
| C13.05 | **Brightness ramp ~72+ pixels on 1CHIP** | write `$8F` not `$80` |
| C13.06 | **SETINI has an analogous early-read bug** | |
| C13.07 | PPU1 open bus refreshed by `$2134-36`, `$2138-3A`, `$213E` | write-only `$21x4-6`/`$21x8-A` return it |
| C13.08 | PPU2 open bus refreshed by `$213B-3D`, `$213F` | |
| C13.09 | **PPU1 and PPU2 open bus are SEPARATE latches** | read a PPU2 register then a PPU1 write-only mirror |
| C13.10 | `$213E` bit 4 = PPU1 open bus, `$213F` bit 5 = PPU2 open bus | |

### C14. Version detection
| # | Test | Assertion |
|---|---|---|
| C14.01 | `$213E` bits 3-0 = PPU1 version (only 1 known) | |
| C14.02 | `$213F` bits 3-0 = PPU2 version (1, 2, or 3) | **gates C13.01** |
| C14.03 | `$213E` bit 5 = master/slave | |

---

## D. DMA / HDMA (target ~35 tests)

### D1. General-purpose DMA
| # | Test | Assertion |
|---|---|---|
| D1.01-08 | Transfer modes 0-7 | 1-reg, 2-reg, 2-reg-write-twice, 4-reg, etc. — one test each |
| D1.02b | `$4300` DMAP layout | `da-ttttt`: bit7 direction, bit6 (HDMA indirect), bit4 A-bus fixed, bit3 A-bus decrement, bits 2-0 mode |
| D1.09 | Byte cost | **8 master cycles per byte, region-independent** |
| D1.10 | Startup overhead | 8-cycle overhead plus channel-start alignment |
| D1.11 | Channel priority | lower channel number first |
| D1.12 | `$4302-04` A-bus address, `$4305/06` size, `$4307` indirect bank | size 0 means $10000 bytes |
| D1.13 | Size register decrements to 0 during transfer | |
| D1.14 | A-bus fixed vs increment vs decrement | |
| D1.15 | **Invalid A-bus addresses** (Errata) | `$21xx`, `$4000-$41FF`, `$4200-$421F`, `$4300-$437F` |
| D1.16 | **WRAM→WRAM via `$2180` prohibited** | WRAM as A-bus source when B-bus is WMDATA |
| D1.17 | Register mirroring | `$43x0-$43xF` per channel; unused `$43xB`/`$43xF` behavior; open bus above |
| D1.18 | DMA power-on state | the **Heian Fuuunden** case |
| D1.19 | CPU cycle timing before DMA start | the **MMPR: Fighting Edition** case |
| D1.20 | **DMA reads update open bus; DMA writes never do** | |

### D2. HDMA
| # | Test | Assertion |
|---|---|---|
| D2.01 | Init at V=0, H≈6 | |
| D2.02 | **Per-line transfer at dot 278** | ~18 master cycles overhead plus 8-24 per channel |
| D2.03 | Line-count byte semantics | bit 7 = repeat mode, bits 6-0 = count; `$00` terminates |
| D2.04 | Repeat mode transfers every line; non-repeat transfers once then counts down | |
| D2.05 | Indirect mode | `$4306/07` indirect address + bank |
| D2.06 | `$4308/09` line counter, `$430A` NLTR | |
| D2.07 | **HDMA preempts GP-DMA** | a GP-DMA in progress is paused for the HDMA slot and resumes |
| D2.08 | `$420C` HDMAEN bit set mid-frame | channel starts at the next line |
| D2.09 | **Enabling HDMA outside vblank → erroneous PPU writes from uninitialized A2An/NLTRn** (Errata) |
| D2.10 | **Scanline-0 HDMA failure** | HDMA fails **for the whole frame** if a DMA ends at the start of scanline 0 and the previous read value was 0 (Errata) |
| D2.11 | HDMA fixed-transfer flag | the **Batman Forever / Lost Vikings** case |
| D2.12 | HDMA decrement flag | the **Kid Kleets** case |
| D2.13 | HDMA direction flag + NMI enable timing | the **Pocky & Rocky** case |
| D2.14 | HDMA transfer-flag state management | the **Aladdin / Super Ghouls'n Ghosts** case |
| D2.15 | DMA suspension during HDMA | the **Dekitate High School** case |
| D2.16 | **HDMA-driven register writes take effect the FOLLOWING line** | the PPU composites each line at dot 276, so a per-line HDMA write is visible starting the next line (the "Air Strike Patrol BG3 scroll" case) |
| D2.17 | **Open bus via HDMA latch** | the **Speedy Gonzales stage 6-1** case |

### D3. Revision-gated DMA bugs (auto-skip by `$4210` version)
| # | Test | Assertion |
|---|---|---|
| D3.01 | **5A22 v1**: DMA finishing just before HDMA crashes the chip | report "v1 behavior observed" as a legal variant |
| D3.02 | **5A22 v2**: a recent HDMA to INIDISP prevents the next DMA from completing | workaround `BBADn=$FF` with transfer pattern 1 |

---

## E. APU — SPC700 and S-DSP (target ~75 tests)

### E1. SPC700 arithmetic quirks
| # | Test | Assertion |
|---|---|---|
| E1.01 | **`MUL YA` flags from Y only** | `Y=$10, A=$10` → YA=$0100, Z **clear** despite A==0 |
| E1.02 | **`DIV YA,X` normal branch** | `Y < (X<<1)`: A = YA/X, Y = YA%X |
| E1.03 | **`DIV YA,X` overflow branch** | `Y >= (X<<1)`: `A = 255 - (YA - (X<<9))/(256-X)`, `Y = X + (YA - (X<<9))%(256-X)` |
| E1.04 | **`DIV` H flag = nibble compare** | `H = (Y&15) >= (X&15)` — nothing to do with a real half-carry |
| E1.05 | `DIV` V = quotient bit 8 | set when quotient >= 256 |
| E1.06 | `DIV` N/Z from the quotient only | |
| E1.07 | `DIV` valid only for quotient <= 511 | |
| E1.08 | **`DAA`** | `if (C \|\| A>$99) {A+=$60; C=1;} if (H \|\| (A&15)>9) A+=6;` — note the second test uses the *post-adjustment* value |
| E1.09 | **`DAS`** | `if (!C \|\| A>$99) {A-=$60; C=0;} if (!H \|\| (A&15)>9) A-=6;` |
| E1.10 | **`TSET1`/`TCLR1` are equality tests** | N/Z reflect `CMP A,[addr]` **before** modification (Errata) |
| E1.11 | **`TSET1`/`TCLR1` read the target TWICE** | a read-sensitive `$FD-$FF` target gets cleared twice |
| E1.12 | **`CLRV` clears H as well as V** | |
| E1.13 | `ADDW`/`SUBW` H = bit11→bit12 carry, Z = true 16-bit zero | |
| E1.14 | `XCN` is 5 cycles | |
| E1.15 | `MOVW YA,aa` sets N/Z on the 16-bit value | |

### E2. SPC700 memory-access side effects
| # | Test | Assertion |
|---|---|---|
| E2.01 | **Store opcodes issue a dummy read** | `MOV $FD,A` **clears Timer 0's counter** |
| E2.02 | Exemptions | `MOV aa,bb` ($FA) and `MOV (X)+,A` ($AF) do **not** dummy-read |
| E2.03 | `MOVW aa,YA` dummy-reads the **LSB only** | `MOVW $FE,YA` clears Timer 1 but not Timer 2 |
| E2.04 | `DBNZ aa` is an RMW → triggers read-sensitive ports | |
| E2.05 | Direct-page index wraps within the page | `MOV A,$FF+X` with X=1 reads `$00`, not `$0100` |
| E2.06 | `PSW.P` selects `$00xx` vs `$01xx` for all DP forms including bit ops and `[aa]+Y` pointer fetches | |
| E2.07 | **Calls push the exact return address** (not retaddr-1, unlike 6502) | |
| E2.08 | `TCALL n` → `[$FFDE - n*2]` | `TCALL 15` reads `[$FFC0]`, inside the IPL ROM when mapped |
| E2.09 | `BRK` shares the `TCALL 0` vector `[$FFDE]`, sets B, clears I | |
| E2.10 | Full per-opcode cycle-count sweep | all 256 opcodes |

### E3. SPC700 I/O registers
| # | Test | Assertion |
|---|---|---|
| E3.01 | Reading `$FD/$FE/$FF` returns 4 bits and **zeroes the counter** | bits 4-7 always 0 |
| E3.02 | `$F1` bit0-2 0→1 resets that timer's stage2 **and** stage3 | |
| E3.03 | `$F1` bits 4/5 clear the CPUIO input latches, non-persistent | |
| E3.04 | `$F1` bit 7 unmaps the IPL ROM exposing the RAM shadow | writes to `$FFC0+` always hit RAM regardless |
| E3.05 | `TnDIV = $00` means divide-by-**256** | |
| E3.06 | Timer 0/1 at 8 kHz (128 cycles), Timer 2 at 64 kHz (16 cycles) | |
| E3.07 | **Timers advance on DSP cycles T1 and T17** | |
| E3.08 | TEST `$F0` bit 0/3 halt the timers | |
| E3.09 | **TEST wait-states 2 and 3 cost the CPU 10/20 clocks but the timers only 8/16** | destructive — mark hardware-only |
| E3.10 | **TEST bit 1 = RAM write enable** — clearing it blocks SPC700 *and* S-DSP writes | |
| E3.11 | `$F2` bit 7 set → writes through `$F3` are discarded, reads still work | |
| E3.12 | **CPUIO bus conflict** | SPC writes an output port while the S-CPU reads it → S-CPU sees the **OR** of old and new |
| E3.13 | Writes to `$00F0-$00FF` also land in the RAM shadow | |
| E3.14 | `$F8`/`$F9` behave as plain RAM (unconnected pins) | |

### E4. IPL boot ROM and handshake
| # | Test | Assertion |
|---|---|---|
| E4.01 | IPL ROM is byte-identical to the canonical 64-byte listing | reset vector `$FFC0` |
| E4.02 | Handoff state | `A=0, X=0, Y=0, SP=$EF, PSW=$02` |
| E4.03 | IPL zerofills `$0000-$00EF`, leaves `[$0000-$0001]` = entrypoint | |
| E4.04 | Ready signal | `Word[$2140] == $BBAA` |
| E4.05 | First kick is `$CC` | |
| E4.06 | Subsequent kicks | `((index+2) & $FF) \| 1`, strictly > last index+1 and non-zero |
| E4.07 | `$2141 == 0` → execute; non-zero → transfer | |
| E4.08 | The final data-byte ack window is only a few cycles wide | |
| E4.09 | Transfer address `$00F2` lets the IPL loop poke DSP registers as (reg#, value) pairs | |
| E4.10 | **16-bit writes to `$2140/41` can corrupt `$2143`** (Errata) — write 8-bit only | |
| E4.11 | **Simultaneous CPU/SPC access to `$2140-$2143` produces incorrect data** (Errata) | |
| E4.12 | APU RAM power-on pattern | repeating **32×$00 then 32×$FF** (chip-dependent — informational, not asserted) |

### E5. BRR decoding
| # | Test | Assertion |
|---|---|---|
| E5.01 | Header decode | `ssssffle`: shift, filter, loop, end |
| E5.02 | Nibble order | high nibble first, signed -8..+7 |
| E5.03 | Nibble scaling | `(nibble << shift) >> 1`, arithmetic |
| E5.04 | **Invalid shift 13/14/15** | collapse to `$0000` (non-negative nibble) or `$F800` (negative) — equivalent to shift=12 with `nibble>>3` |
| E5.05-08 | The four filters | exact integer formulas, one test each. **"Games depend on these exact formulas; simplifying will break sound effects"** |
| E5.09 | **15-bit wrap** | clamp to 16 bits, then `+4000h..+7FFFh` becomes `-4000h..-1` and `-8000h..-4001h` becomes `0..3FFFh` — **the sign is lost** |
| E5.10 | Loop/End code 1 (End+Mute) | forces Release with envelope 0 **immediately** |
| E5.11 | Loop/End code 2 behaves as code 0 | |
| E5.12 | **ENDX sets at the START of decoding the end block** | |
| E5.13 | **BRR decoding continues even for released voices** | "voices never actually stop decoding" |
| E5.14 | DIR entry format | `DIR*$100 + SRCN*4`, start addr then loop addr |
| E5.15 | **Mid-playback SRCN change** | not yet looped → new sample starts at its *start* address; already looped → at its *loop* address; rewriting the same value is a no-op |
| E5.16 | **Three consecutive max-negative samples cause an overflow pop** (Errata) | |

### E6. Pitch and gaussian interpolation
| # | Test | Assertion |
|---|---|---|
| E6.01 | Counter layout | bits 15-12 select the sample, bits 11-4 are the gaussian index |
| E6.02 | `$1000` = 1:1 (32 kHz); `$2000` = +1 octave; `$0800` = -1 octave | |
| E6.03 | PMON factor | `(OUTX[x-1] SAR 4) + $400`, then `(Step*Factor) SAR 10` |
| E6.04 | **PMON never affects voice 0** | `$2D` bit 0 has no effect |
| E6.05 | **PMON does not modulate noise** | |
| E6.06 | Pitch counter clamped to `$7FFF` | |
| E6.07 | Gaussian table | all 512 entries byte-exact |
| E6.08 | **The `$801` overflow bug** | `nibbles=77778888, shift=12, filter=0` → `+3FF8h` instead of `-4000h` |
| E6.09 | **Partial overflow rules** | 1st addition can't overflow; **2nd wraps** (i=$00-$1F); **3rd saturates** (i=$20-$FF) |
| E6.10 | Gaussian is bypassed for noise voices | |
| E6.11 | Waveform vectors | `79797979`, `77997799`, `77779999`, `7777CC44` golden outputs |

### E7. Envelopes
| # | Test | Assertion |
|---|---|---|
| E7.01 | Rate table | 32 entries `{0,2048,1536,...,2,1}`; rate 0 never triggers |
| E7.02 | **Counter offset table** | `{0,0,1040,536,...}` — two voices at different rates show the implied phase relationship |
| E7.03 | Attack rate index = `a*2+1`, step +32 | |
| E7.04 | **Attack `a==$F`** | rate = every sample, step **+1024** |
| E7.05 | Decay rate index = `d*2+16`, step `E -= 1; E -= E>>8` | |
| E7.06 | Sustain rate index = `r` verbatim | |
| E7.07 | Sustain boundary = `$100 * (l+1)`; compare is `(E>>8) == SL` | |
| E7.08 | Release | rate forced to every-sample, step -8, overrides everything, ~0.008 s to silence |
| E7.09 | **Release rate is fixed** — custom release requires GAIN (Errata) | |
| E7.10 | Direct gain | `E = G<<4` immediately |
| E7.11-14 | The four custom-gain modes | linear dec (-32), exp dec, linear inc (+32), bent inc (+32 below $600 else +8) |
| E7.15 | **GAIN-mode sustain-boundary bug** | in GAIN mode the Decay→Sustain compare reads the boundary from **`VxGAIN` bits 7-5**, not `VxADSR2` bits 7-5 |
| E7.16 | **Bent-increase uses the CLIPPED previous envelope** | an underflowed (negative) value reads as >= $600 |
| E7.17 | Linear-decrease underflow clamps to 0, never wraps | |
| E7.18 | `VxENVX` = `E >> 4`, bit 7 always 0 | |
| E7.19 | `VxOUTX` = post-envelope, pre-volume, high byte | |
| E7.20 | **ENVX/OUTX writes 1-2 clocks before the DSP writeback are lost** | |
| E7.21 | **ADSR/GAIN mode-change race** (Errata) | write ADSR2/GAIN **before** ADSR1 |

### E8. Key on/off
| # | Test | Assertion |
|---|---|---|
| E8.01 | **KON/KOFF polled every SECOND sample (16 kHz)** | |
| E8.02 | **5-sample key-on delay** before envelope/BRR start |
| E8.03 | KON restarts the sample even if already playing, zeroing the envelope | |
| E8.04 | KON clears the voice's ENDX bit even when suppressed | |
| E8.05 | KON is write-triggered and non-persistent; **KOFF and FLG.7 exert influence continuously** | |
| E8.06 | Collapse case 1 | `KOFF=$FF`, then `KON=$01`, then `KOFF=$00` in immediate succession → voice 1 usually **does** key on |
| E8.07 | Collapse case 2 | `KON=$01` then `KON=$02` → usually only voice 2; if both, voice 1 is **2** samples ahead (proves the 16 kHz rate) |
| E8.08 | Collapse case 3 | `KOFF=$FF` then `KOFF=$00` → usually all voices keep playing |
| E8.09 | **63-cycle KOFF window** | KON while KOFF set produces no output unless KOFF clears within 63 SPC cycles; 64-127 cycles is probabilistic |
| E8.10 | Internal KON bits clear 63 clocks after the poll | |
| E8.11 | **FLG bit 7 is polled EVERY sample and per-voice**, unlike KON/KOFF | |
| E8.12 | KOFF + KON together silences faster than KOFF alone (a click source) | |
| E8.13 | DSP KOF register init | the **Chester Cheetah / King of Dragons** case |

### E9. Noise, echo, mixer
| # | Test | Assertion |
|---|---|---|
| E9.01 | LFSR taps | bit0 XOR bit1 → bit14; initial state `$4000` |
| E9.02 | **Noise output is highpass-filtered** as a consequence of the 15-bit interpretation (Errata) | |
| E9.03 | VxPITCH does not affect noise frequency | |
| E9.04 | **Noise voices still decode BRR** → End+Mute kills the noise | workaround: point SRCN at a looping dummy block |
| E9.05 | Echo buffer layout | 4 bytes/entry, low 7 bits in bits 1-7 |
| E9.06 | **EDL=0 gives a 4-byte (1-sample) buffer, not zero** — and continuously overwrites 4 bytes at the buffer start (Errata) | |
| E9.07 | **EDL change latency up to 0.25 s** | takes effect only at the buffer end (up to 7680 samples / 240 ms) |
| E9.08 | **ESA change delayed by 1-2 samples** | |
| E9.09 | **Echo buffer wraps at a 16-bit boundary, potentially corrupting page zero** (Errata) | and can corrupt the `$FFC0+` IPL RAM shadow |
| E9.10 | **FLG bit 5 disables echo WRITES but not READS** — the buffer becomes a static forever-loop | |
| E9.11 | **FIR: only the final (FIR7) addition saturates; the first seven WRAP** (Errata) | |
| E9.12 | Echo write value masked `& $FFFE` | bit 0 forced 0 |
| E9.13 | Left/right FIR independent, identical coefficients, no crosstalk | |
| E9.14 | **MVOL/EVOL/EFB/FIRx = `$80` (-128) overflows; VxVOL = `$80` does not** | |
| E9.15 | Per-voice mix saturates after each addition | |
| E9.16 | **Final output is XORed with `$FFFF`** by the post-amp (phase inversion) | matters for any bit-exact audio hash |
| E9.17 | FLG.MUTE zeroes output but internal processing (incl. echo RAM writes) continues | |
| E9.18 | FLG.RESET behaves as `KOFF=$FF` + envelopes 0, but echo keeps sounding | |
| E9.19 | **ENDX: any write clears ALL bits** regardless of the value written | |
| E9.20 | The official Nintendo FIR preset `FF 08 17 24 24 17 08 FF` is **bugged** (positive taps exceed +$7F) yet games rely on it | |

### E10. Cycle-level DSP pipeline
| # | Test | Assertion |
|---|---|---|
| E10.01 | 32 SPC cycles per output sample | 32 kHz nominal |
| E10.02 | The 32-slot register-access schedule (T0-T31) | ENDX.n at T1/T4/T7/..., FLG.5 at T29/T30, KON at T31 |
| E10.03 | ENDX/OUTX/ENVX written on three separate cycles (voice7/8/9) | |
| E10.04 | SPC and DSP share `/RESET` and the 2.048 MHz clock | timers tick in lockstep at T1/T17 |
| E10.05 | **Post-reset DSP state** | FLG behaves as `$E0` regardless of what reads back. **[Conflict: nocash says ENDX=$FF, Anomie says 0 — report as a golden vector]** |
| E10.06 | SPC cycle timing | the **ActRaiser 2 / Tales of Phantasia / Illusion of Gaia** case |

---

## F. Input (target ~20 tests)

| # | Test | Assertion |
|---|---|---|
| F1.01 | Manual read bit order | latch (write 1 then 0 to `$4016.0`), then 16 reads → B,Y,Select,Start,Up,Down,Left,Right,A,X,L,R,0,0,0,0 |
| F1.02 | Reads 17-32 return **1** on official pads (some third-party return 0) | |
| F1.03 | Latch is **shared** — writing `$4016.0` latches both ports | |
| F1.04 | `$4016`/`$4017` read bits 7-2 return **CPU open bus** | |
| F1.05 | Auto-read signature nibble | bits 3-0 == `0000` for a standard pad |
| F1.06 | Bit 15 of `$4219` = the first bit clocked = B | |
| F1.07 | With `$4200.0 = 0`, `$4218-$421F` do not update | |
| F1.08 | **Auto-read start window** | `$4212.0` sets no earlier than dot 32.5 and no later than dot 95.5 of the first vblank line (dot 74.5 on the first frame; thereafter a multiple of 256 cycles later) |
| F1.09 | **Auto-read duration exactly 4224 master cycles** | ≈3.097 scanlines |
| F1.10 | **The race** | reading `$4212` immediately at NMI entry can see busy=0 *before* auto-read starts → a naive "wait until not busy" loop returns **stale data** (Errata) |
| F1.11 | **`$4016.0` must stay 0 during auto-read** | writing 1 corrupts `$4218-$421F` |
| F1.12 | Results valid by V=$E3 | |
| F1.13 | **Auto-read results may change during lag frames** (Errata) | |
| F1.14 | `$4201` power-on = `$FF`; `$4213` is open-collector wired-AND | any bit written 0 reads 0; a bit written 1 may read 0 if a device pulls it low |
| F1.15 | Multitap detect | `$4016.0=1` → eight `$4017` D1 reads give $FF; `$4016.0=0` → not $FF |
| F1.16 | Multitap port-pair select is **`$4201` bit 7**, not `$4016` bit 1 | 1 → ports 2/3; 0 → ports 4/5 |
| F1.17 | Multitap pads supply a **17th bit** = controller-connected | |
| F1.18 | Mouse | signature `0001`; **sign-magnitude, not two's complement**; zero magnitude **repeats the previous sign** |
| F1.19 | Mouse timing minimums | >=170 master cycles between bit reads; **>=336 between the byte-2 and byte-3 reads** |
| F1.20 | Mouse sensitivity cycling fails during an active auto-read | |
| F1.21 | Super Scope | port 2 only; auto-read bits 7-4 all high, bits 11-10 low; latches OPHCT/OPVCT after the sensor sees the beam **6 times** |
| F1.22 | NTT Data Keypad | bits 12-15 = `0100` — distinguishable from a standard pad in auto-read data alone |

---

## G. Power-on / reset / cartridge (target ~18 tests)

| # | Test | Assertion |
|---|---|---|
| G1.01 | Documented power-on values | `$4200`=$00, `$4201`=$FF, `$4202`=$FF, `$4204/05`=$FFFF, `$4207/08`=$1FF, `$4209/0A`=$1FF, `$420D`=$00 |
| G1.02 | `$4210`/`$4211` bit 7 clear on power-on and reset | |
| G1.03 | **Everything else is indeterminate** — APUIOn, WMDATA, WMADDL/M/H, JOYSER0/1, HDMAEN, MDMAEN, JOY1-4. A test must report, not assert |
| G1.04 | CPU enters emulation mode and vectors through `$00FFFC` | |
| G1.05 | **The console has no boot ROM; most PPU registers start unknown** | |
| G1.06 | **Cartridge /RESET does not reset the PPU** — only S-CPU, APU, S-WRAM. PPU state survives soft reset (Errata) |
| G1.07 | **No canonical WRAM fill exists** — model-dependent. Report the observed pattern; never assert one. (The **Death Brade / Power Drive / Super Keiba 2** titles depend on RAM/SRAM power-on state) |
| G1.08 | Reads of write-only MMIO return **CPU open bus**, not $00/$FF | |
| G1.09 | `$4210` bits 3-0 = 5A22 version | gates D3.01/D3.02 |
| G1.10 | Header checksum invariant | `checksum XOR complement == $FFFF` |
| G1.11 | Checksum algorithm | sum all bytes with `$FFDC`=$FFFF and `$FFDE`=$0000; non-power-of-2 uses the largest-prefix + mirrored-remainder split |
| G1.12 | Header location by map | LoROM `$007FC0`, HiROM `$00FFC0`, ExHiROM `$40FFC0` |
| G1.13 | `$FFD5` bit 4 = FastROM capability | (**not bit 7** — the SNESdev prose is wrong; the `001smmmm` diagram and fullsnes agree on bit 4) |
| G1.14 | LoROM decode | `offset = ((bank & $7F) << 15) \| (addr & $7FFF)`; cart A15 unconnected |
| G1.15 | HiROM decode | `offset = ((bank & $3F) << 16) \| addr`; banks $40-$7D mirror $C0-$FD |
| G1.16 | ExHiROM decode | A23 inverted into cart A22; banks $80-$FF → first 4 MiB, $00-$7D → second |
| G1.17 | SRAM mapping | the **Fire Emblem: Thracia 776 / Ys III** case |
| G1.18 | Copier-header detection | `filesize % 1024 == 512` |

---

# PART III — EXISTING TEST-ROM LANDSCAPE AND GAP ANALYSIS

## Existing suites

| Suite | Author | License | Coverage | Granularity |
|---|---|---|---|---|
| **SingleStepTests/65816** | community | **NONE** | per-opcode, all modes, 8/16-bit, native+emulation, cycle-by-cycle bus traces | JSON, host-side |
| **SingleStepTests/spc700** | community | MIT | same for SPC700 | JSON, host-side |
| **gilyon/snes-tests** | gilyon | MIT | on-cart CPU + SPC, 1107 assertions | pass/fail `.sfc` + golden `tests*.txt` |
| **undisbeliever/snes-test-roms** | undisbeliever | Zlib | PPU/DMA/HDMA + `src/hardware-glitch-tests/` (INIDISP/SETINI early-read) | visual, some golden |
| **blargg `spc_*`** | blargg | unstated | `spc_smp`, `spc_timer`, `spc_mem_access_times`, `spc_dsp6` — cycle-accurate SPC/DSP | literal PASS/FAIL |
| **240p Test Suite (SNES)** | community | GPL-2.0+ | video/overscan patterns | visual |
| **PeterLemon/SNES (Krom)** | Krom | **NONE** | broad CPU/PPU/SPC/DSP/GSU + reference PNGs | screenshot compare |
| **Nintendo Aging/Test/Controller Program** | Nintendo | not redistributable | RAM/DRAM/VRAM, DMA, mul/div, timers, EXT Latch, HV Timer, VH Flag | official diagnostic |
| **Cx4 / SPC7110 check programs** | Nintendo | not redistributable | coprocessor | |
| **ctrltest / mset** | rainwarrior | — | controller / mouse | visual |
| **gradient-test** | NovaSquirrel | — | CGWSEL | visual |
| **Two Ship / Elasticity / PPU bus activity** | rainwarrior, lidnariq | — | Mode 5 + interlace, Mode 3, modes 0-6 | visual |

## The gap AccuracySNES fills

**1. There is no single canonical SNES battery.** This is stated plainly in the project's own
`docs/testing-strategy.md`: *"The SNES has no single canonical battery — and no
Nintendulator-style textual golden CPU log exists for the 65816."* Today's accuracy story is a
*composed* multi-suite oracle across five heterogeneous corpora with incompatible licenses,
three of which cannot be vendored into an MIT/Apache tree at all.

**2. Licensing is the binding constraint, not coverage.** The single best CPU oracle
(SingleStepTests/65816) ships **no license**. Krom's suite ships no license. blargg's ROMs are
unstated. The 240p Suite is GPL-2.0+. A permissively-licensed, self-contained battery is
*itself* the deliverable — it would let any emulator project gate CI on a vendored artifact.

**3. No machine-readable result surface.** Every existing on-cart suite reports visually or via
a golden text table. AccuracySNES should write a results block to a fixed WRAM address so a
headless harness can assert on it directly — the single biggest usability gap for emulator CI,
and something AccuracyCoin itself lacks.

**4. Specific behaviors with NO public test-ROM coverage found:**

| Gap | Why it matters |
|---|---|
| `r`/`rl` PC-relative wrapping | superfamicom.org marks it **"XXX: untested"** — no public data exists |
| Emulation-mode `d`/`d,X` **word-read** wrap | documented only as "theoretically"; unverified |
| MVN/MVP mid-instruction interrupt behavior | undocumented in every source consulted |
| MVN/MVP with 8-bit index in **native** mode | undocumented |
| `$4203`/`$4206` overlapping mul/div | Errata says genuinely undefined — needs a golden vector, not an assertion |
| The 5A22 v1/v2 DMA-HDMA crash quirks | no test ROM; revision-gated |
| HDMA scanline-0 whole-frame failure | well-defined but no known title or ROM verifies it |
| DRAM refresh position (538 then multiples of 8) | no direct test |
| No-IRQ-at-dot-153 exceptions | obscure, untested |
| GAIN-mode sustain-boundary bug | modeled in ares, no public ROM |
| Bent-increase clipped-value interaction | same |
| 63-cycle KON/KOFF window | same |
| Gaussian `$801` overflow | described by nocash, no packaged test |
| FIR wrap-vs-saturate asymmetry | same |
| EDL/ESA change latency | same |
| PPU1 vs PPU2 **separate** open-bus latches | no dedicated test |
| CGRAM write during active display | byuu documented it; no public ROM |
| OAM address destruction pattern during render | "deterministic but unknown" |
| Overscan mid-frame VRAM lock | Mesen documents the repro; not packaged |
| Auto-read start-window race | Errata warns; no test |
| Mouse 170/336-cycle minimums | no test |
| Multitap 17th connected-bit | no test |

**5. Documentation errors AccuracySNES would settle.** Research surfaced six live
source-conflicts worth resolving on hardware:

| Conflict | Sources | Working answer |
|---|---|---|
| `$213F` 50/60 Hz flag bit | SNESdev PPU_registers says bit 3; fullsnes says bit 4 | **bit 4** (bits 3-0 are the version field) |
| `$FFD5` speed bit | SNESdev prose says bit 7; its own `001smmmm` diagram and fullsnes say bit 4 | **bit 4** |
| `$4212` HBlank flag bit | Mesen prose says bit 4; its own diagram and fullsnes say bit 6 | **bit 6** |
| HBlank set/clear dot | superfamicom says H=274/H=1; Mesen says H≈$121/$12 | unresolved (~15-dot gap) |
| Dots per scanline | SNESdev says 341; Anomie/superfamicom/Mesen say 340 with dots 323/327 stretched | **340 + 2 stretched** (reconciles to 1364) |
| `$2103` priority-rotation source | fullsnes says register bits 6-1; Anomie says `(internal OAMAddr & $FE) >> 1` | unresolved |
| WRIO bit→port mapping | connector page vs Mesen wiki disagree on bit 6/7 | bit 7 → port 2 (matches bsnes/ares) |
| Key-on delay | nocash/blargg/ares say 5 samples; Anomie says ~8 | **5** |
| Post-reset ENDX | nocash says $FF; Anomie says 0 | unresolved — golden vector |
| CGRAM during HBlank | SNESdev says yes; fullsnes says "doesn't work too well" | unresolved |

---

# PART IV — RECOMMENDED STRUCTURE

**Test count: ~320 across 7 subsystems**, roughly 2.3x AccuracyCoin's 141 — appropriate given
the SNES's larger register surface and dual-CPU architecture.

**Pagination (20 tests/page → ~16 pages):**
1-3 CPU addressing/wrapping · 4-6 CPU cycles/interrupts/decimal/block-move · 7 Bus timing ·
8-12 PPU · 13-14 DMA/HDMA · 15-18 APU · 19 Input · 20 Power-on/cart.

**Result-reporting contract:**
- Per test: `PASS` / `FAIL <hex code>` / `SKIP <reason>` / `VARIANT <n>` (legal-alternative
  outcomes — chip revision, region, or genuinely-undefined hardware behavior).
- A **golden-vector class** of tests that report an observed value rather than pass/fail, for
  the behaviors hardware itself does not define (`$4203`/`$4206` race, decimal-mode V flag,
  WRAM power-on fill, post-reset ENDX). This is the honesty gate applied to the ROM itself.
- A fixed WRAM results block + screen hash for headless CI.

**Auto-skip gates:** `$4210` bits 3-0 (5A22 v1/v2), `$213E` bits 3-0 (PPU1), `$213F` bits 3-0
(PPU2 v1/v2/v3), `$213F` bit 4 (NTSC/PAL), and a 1CHIP heuristic (no programmatic detection
exists — the INIDISP tile-corruption test is itself the closest available probe).

**Build constraints for this project:** LoROM, no coprocessor, no SRAM, self-contained, valid
header checksum (some flash carts require it), and permissively licensed so it can be vendored
into `tests/roms/` rather than the gitignored `external/` tier.

