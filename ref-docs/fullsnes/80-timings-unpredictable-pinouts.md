# Fullsnes — Unpredictable Things, Timings, Pinouts, Chipset & Mods

[Index](00-index.md) · [« Hotel Boxes & Arcade Machines](70-hotel-arcade-nss-sfcbox.md) · [CPU 65XX / 65C816 Microprocessor Reference »](90-cpu-65816.md)

**Sections in this file:**

- [SNES Unpredictable Things](#snes-unpredictable-things)
- [SNES Timings](#snes-timings)
- [SNES Timing Oscillators](#snes-timing-oscillators)
- [SNES Timing H/V Counters](#snes-timing-hv-counters)
- [SNES Timing H/V Events](#snes-timing-hv-events)
- [SNES Timing PPU Memory Accesses](#snes-timing-ppu-memory-accesses)
- [SNES Pinouts](#snes-pinouts)
- [SNES Controllers Pinouts](#snes-controllers-pinouts)
- [SNES Audio/Video Connector Pinouts](#snes-audiovideo-connector-pinouts)
- [SNES Power Supply](#snes-power-supply)
- [SNES Expansion Port (EXT) Pinouts](#snes-expansion-port-ext-pinouts)
- [SNES Cartridge Slot Pinouts](#snes-cartridge-slot-pinouts)
- [SNES Chipset](#snes-chipset)
- [SNES Pinouts CPU Chip](#snes-pinouts-cpu-chip)
- [SNES Pinouts PPU Chips](#snes-pinouts-ppu-chips)
- [SNES Pinouts APU Chips](#snes-pinouts-apu-chips)
- [SNES Pinouts ROM Chips](#snes-pinouts-rom-chips)
- [SNES Pinouts RAM Chips](#snes-pinouts-ram-chips)
- [SNES Pinouts CIC Chips](#snes-pinouts-cic-chips)
- [SNES Pinouts MAD Chips](#snes-pinouts-mad-chips)
- [SNES Pinouts RTC Chips](#snes-pinouts-rtc-chips)
- [SNES Pinouts Misc Chips](#snes-pinouts-misc-chips)
- [SNES Pinouts GSU Chips](#snes-pinouts-gsu-chips)
- [SNES Pinouts CX4 Chip](#snes-pinouts-cx4-chip)
- [SNES Pinouts SA1 Chip](#snes-pinouts-sa1-chip)
- [SNES Pinouts Decompression Chips](#snes-pinouts-decompression-chips)
- [SNES Pinouts BSX Connectors](#snes-pinouts-bsx-connectors)
- [SNES Pinouts NSS Connectors](#snes-pinouts-nss-connectors)
- [SNES Pinouts Nintendo Power Flashcarts](#snes-pinouts-nintendo-power-flashcarts)
- [SNES Common Mods](#snes-common-mods)
- [SNES Controller Mods](#snes-controller-mods)
- [SNES Xboo Upload (WRAM Boot)](#snes-xboo-upload-wram-boot)

---

<a id="snesunpredictablethings"></a>

## SNES Unpredictable Things

**Open Bus**

Garbage appears on reads from unused addresses (see below), on reads from
registers that contain less than 8 bits (see more below), and on reads from
(most) write-only registers (see even more below). The received data is the
value being previously output on the data bus. In most cases, that is the last
byte of the opcode (direct reads), or the upper byte of indirect address
(indirect reads), ie.

```text
  LDA IIJJ,Y   aka MOV A,[IIJJ+Y]  --> garbage = II
  LDA (NN),Y   aka MOV A,[[NN]+Y]  --> garbage = [NN+1]
```

When using General Purpose DMA, it'd be probably rely on the store opcode that
has started the DMA (in similar fashion as above loads). In case of HDMA things
would be most unpredictable, garbage would be whatever current-opcode related
value. And, if two DMA transfers follow directly after each other, garbage
would probably come from the previous DMA.

Also, in 6502-mode (and maybe also in 65816-mode), the CPU may insert
dummy-fetches from unintended addresses on page-wraps?

**Unused Addresses in the System Region**

```text
  Address       Size
  2000h..20FFh  100h    ;unused addresses
  2181h..2183h  3       ;write-only WRAM Address registers
  2184h..21FFh  7Ch     ;unused addresses in the B-BUS region
  2200h..3FFFh  1E00h   ;unused addresses
  4000h..4015h  16h     ;unused slow-CPU Ports
  4018h..41FFh  1E8h    ;unused slow-CPU Ports
  4200h..420Dh  0Eh     ;write-only CPU Ports
  420Eh..420Fh  2       ;unused CPU Ports
  4220h..42FFh  E0h     ;unused CPU Ports
  43xCh..43xEh  3*8     ;unused DMA Ports
  4380h..7FFFh  3C80h   ;unused/expansion area
```

So, of the total of 8000h bytes, a large number of 5EF4h is left unused.

Ports 2144h..217Fh are APU mirrors, NOT open bus.

**Unused bits (in Ports with less than 8 used bits)**

```text
  Addr  Mask Name    Unused Bits
  4016h FCh  JOYA    Bit7-2 are open bus
  4017h E0h  JOYB    Bit7-5 are open bus
  4210h 70h  RDNMI   Bit6-4 are open bus
  4211h 7Fh  TIMEUP  Bit6-0 are open bus
  4212h 3Eh  HVBJOY  Bit5-1 are open bus
```

**PPU1 Open Bus**

PPU1 Open Bus is relies on the value most recently read from Ports 2134h-2136h,
2138h-213Ah, 213Eh. This memorized value shows up on later reads from read-only
Ports 21x4h..21x6h and 21x8h..21xAh (with x=0,1,2) (in all 8bits), as well as
in Port 213Eh.Bit4.

**PPU2 Open Bus**

PPU2 Open Bus is relies on the value most recently read from Ports 213Bh-213Dh,
213Fh. This memorized value shows up on later reads from Port
213Bh.2nd_read.Bit7, 213Ch/213Dh.2nd_read.Bits7-1, and Port 213Fh.Bit5.

**PPU Normal Open Bus**

Other write-only PPU registers &amp; the HV-latch "strobe" register are acting
like "normal" CPU Open Bus (ie. usually returning the most recent opcode byte).
These are 21x0h..21x3h, 21x7h (with x=0,1,2,3), and 21xBh..21xFh (with
x=0,1,2).

**Open Bus for DMA**

DMA cannot read from most I/O ports, giving it some additional open bus areas:

```text
  2100h-21FFh  Open Bus (when used as A-Bus) (of course they work as B-Bus)
  4000h-41FFh  Open Bus (name 4016h/4017h cannot be read)
          actually, 4017h <does> return bit4-2 set (1=GNDed joypad input)
  4210h-421Fh  These do work (the only I/O ports that are not open bus)
  4300h-437Fh  Special Open Bus (DMA registers, may return [PC] instead of FFh)
```

For DMA reads, one may expect the same garbage as for CPU reads (ie. when
starting a DMA via "MOV [420Bh],A", one would expect 42h as open bus value).
However, it takes a few cycles before the DMA transfer does actually start,
during that time the hardware "forgets" the 42h value, and instread, DMA does
always read FFh (High-Z) as open bus value. With two exceptions: If DMA wraps
from an used to unused address (eg. from 1FFFh/WRAM to 2000h/unused) then the
first "unused" byte will be same as the last "used" byte, thereafter, it
forgets that value (and returns FFh on further unused addresses).

The other exception is if DMA &lt;starts&gt; at 4300h..437Fh: In this case it
will read [PC] for that region (and will also "memorize" it when reaching the
first unused address at 4380h, and then returns FFh for 4381h and up). For
example: "MOV [420Bh],A" followed by "ADC A,33h" would return 69h (the first
byte of the ADC opcode). This effect occurs only if the transfer &lt;starts&gt;
in that region, ie. if starts below that area, and does then wrap from 42FFh to
4300h, then it returns FFh instead of 69h. (The reason of this special effect
is probably that the DMA somehow "ignores the databus", so external HIGH-Z
levels (like from XBOO cable or Cartridge) cannot drag the "memorized" value to
HIGH.)

EDIT: The above "memorize for next ONE unused address" applies only when XBOO
cable is connected (which pulls the databus to HIGH rather quickly). Without
XBOO cable (and without cartridge connected) the "memorized" value may last for
the next 2000h (!) unused addresses, then it may slowly get corrupted (some
bits going to HIGH state, until, after some more time, all bits are HIGH).

Actually, it seems to last even longer than 2000h -- possibly forever (until
DMA ends, or until it reaches a used address).

**SPC700 Division Overflow/Result (DIV YA,X opcode)**

The overall division mechanism (with and without overflows) is:

```text
  H = (X AND 0Fh)<=(Y AND 0Fh)   ;half carry flag (odd dirt effect)
  Temp = YA
  FOR i=1 TO 9
    Temp=Temp*2                                     ;\rotate within 17bits
    IF Temp AND 20000h THEN Temp=(Temp XOR 20001h)  ;/
    IF Temp>=(X*200h)  THEN Temp=(Temp XOR 1)
    IF Temp AND 1      THEN Temp=(Temp-(X*200h)) AND 1FFFFh
  NEXT i
  A = (Temp AND FFh) ;result.bit7-0
  V = (Temp.Bit8=1)  ;result.bit8
  Y = (Temp/200h)    ;remainder (aka temp.bit9-16)
  N = (A.Bit7=1)     ;sign-flag (on result.bit7) (though division is unsigned)
  Z = (A=00h)        ;zero-flag (on result.bit7-0)
```

That is, normally (when result = 0000h..00FFh):

```text
  A=YA/X, Y=YA MOD X, N=ResultingA.Bit7, Z=(ResultingA=00h), V=0, H=(see above)
```

An intact 9bit result can be read from V:A (when result = 0000h..01FFh).
Otherwise return values are useless garbage (when result = 0200h..Infinite).

<a id="snestimings"></a>

## SNES Timings

> **Note (RustySNES ref):** For an authoritative, independently-cross-checked account of dot/scanline counts, the 5A22 memory-access cycle map, and the exact H/V-counter latch behavior used to validate accuracy work, cross-reference [SNESdev Wiki: Timing](https://snes.nesdev.org/wiki/Timing) alongside the numbers below.

[SNES Timing Oscillators](#snes-timing-oscillators)

[SNES Timing H/V Counters](#snes-timing-hv-counters)

[SNES Timing H/V Events](#snes-timing-hv-events)

[SNES Timing PPU Memory Accesses](#snes-timing-ppu-memory-accesses)

<a id="snestimingoscillators"></a>

## SNES Timing Oscillators

**NTSC Timings**

```text
  NTSC crystal      21.4772700MHz (X1, type number D214K1)
  NTSC color clock  3.57954500MHz (21.47727MHz/6)  (generated by PPU2 chip)
  NTSC master clock 21.4772700MHz (21.47727MHz/1)  (without multiplier/divider)
  NTSC dot clock    5.36931750MHz (21.47727MHz/4)  (generated by PPU chip)
  NTSC cpu clock    3.57954500MHz (21.47727MHz/6)  (without waitstates)
  NTSC cpu clock    2.68465875MHz (21.47727MHz/8)  (short waitstates)
  NTSC cpu clock    1.78977250MHz (21.47727MHz/12) (joypad waitstates)
  NTSC frame rate   60.09880627Hz (21.477270MHz/(262*1364-4/2))
  NTSC interlace    30.xxxxxxxxHz (21.477270MHz/(525*1364))
```

**PAL Timings**

```text
  PAL crystal       17.7344750MHz (X1, type number D177F2)
  PAL color clock   4.43361875MHz (17.7344750MHz/4)   (generated by S-CLK chip)
  PAL master clock  21.2813700MHz (17.7344750MHz*6/5) (generated by S-CLK chip)
  PAL dot clock     5.32034250MHz (21.2813700MHz/4)   (generated by PPU chip)
  PAL cpu clock     3.54689500MHz (21.2813700MHz/6)   (without waitstates)
  PAL cpu clock     2.66017125MHz (21.2813700MHz/8)   (short waitstates)
  PAL cpu clock     1.77344750MHz (21.2813700MHz/12)  (joypad waitstates)
  PAL frame rate    50.00697891Hz (21.281370MHz/(312*1364))
  PAL interlace     25.xxxxxxxxHz (21.281370MHz/(625*1364+4/2))
```

**APU Timings**

```text
  APU oscillator    24.576MHz (X2, type number 24.57MX)
  DSP sample rate   32000Hz   (24.576MHz/24/32)
  SPC700 cpu clock  1.024MHz  (24.576MHz/24)
  SPC700 timer 0+1  8000Hz    (24.576MHz/24/128)
  SPC700 timer 2    64000Hz   (24.576MHz/24/16)
  CIC clock         3.072MHz  (24.576MHz/8)
  Expansion Port    8.192MHz  (24.576MHz/3)
```

**CPU Clock Notes**

CPU Clock cycles (opcode fetches, data transfers, and internal cycles) are
usually clocked at 3.5MHz or 2.6MHz (or a mixup thereof).

```text
  3.5MHz   Used for Fast ROM, most I/O ports, and internal CPU cycles
  2.6MHz   Used for Slow ROM, for WRAM, and for DMA/HDMA transfers
  1.7MHz   Used only for (some) Joypad I/O Ports
```

The CPU is paused for 40 master cycles (per 1364 cycle scanline) for memory
REFRESH purposes, effectively making the CPU around 3% slower. The CPU is also
paused when using DMA/HDMA transfers.

Nintendo specifies the following ROM timings to be required:

```text
  3.5MHz   use 120ns or faster ROM/EPROMs
  2.6MHz   use 200ns or faster ROM/EPROMs
```

**Dot Clock Notes**

The above values apply for the drawing period, in the hblank period some cycles
are a bit longer. This "stuttering" effect appears also on the dotclk output on
expansion port.

**External Oscillators (in Cartridges)**

```text
  DSPn      7.600MHz    Plastic Type "[M]7600A"  (used without divider)
  ST010     22.000MHz   Plastic Type "[M]22000C" (internally 11.000MHz)
  ST011     15.000MHz   Ceramic Type "15.00X"    (used without divider)
  ST018     21.440MHz   Plastic Type "[M]21440C"
  CX4       20.000MHz   Plastic Type "[M]20000C" or "20.0MC/TDKY"
  MC1       <master>    SNES Master Clock
  GSU1      21.4MHz     Plastic Type "21.4MC/TDKT"
  GSU2      21.44MHz    Plastic Type "[M]21440C"
  SA-1      <master>    SNES Master Clock
  S-DD1     <master>    SNES Master Clock
  SPC7110   <master>    SNES Master Clock
  MX15001   <master>    SNES Master Clock (Nintendo Power Flashcarts)
  SGB       <master>    SNES Master Clock
  SGB2      20.9MHz     External oscillator (located on PCBs solder-side)
  BS-X      18.432MHz   Satellaview Receiver Unit (on expansion port)
  RTC-4513  32.768kHz   On-chip 32.768kHz quartz crystal in RTC chip
  S-3520    32.768kHz   External 32.768kHz quartz crystal (SFC-Box)
  S-RTC     ? kHz       External unknown-frequency crystal
  ACE       <dotclk>    SNES Dot Clock (Exertainment RS232 on expansion port)
```

SNES Master Clock, &lt;master&gt; = 21.4772700MHz (NTSC), or 21.2813700MHz
(PAL).

<a id="snestiminghvcounters"></a>

## SNES Timing H/V Counters

**Horizontal Timings**

```text
  Scanline Length        1364 master cycles (341 dot cycles)
    Except, Line F0h in Field.Bit=1 of Interlace: 1360 master cycles
  Refresh (per scanline)   40 master cycles (10 dot cycles)
```

```text
  50*312*1364 = 21.278400 MHz   // 21.281370MHz/(312*1364) = 50.00697891 Hz
  60*262*1364 = 21.442080 MHz   // 21.477270MHz/(262*1364-2) = 60.09880627 Hz
```

**Long and Short Scanlines**

A normal scanline is 1364 master cycles long. But, there are two special cases,
in which lines are 4 cycles longer or shorter:

```text
  Short Line --> at 60Hz frame rate + interlace=off + field=1 + line=240
  Long Line  --> at 50Hz frame rate + interlace=on + field=1 + line=311
  (in both cases, the selected picture size, 224 or 239 lines, doesn't matter)
```

Technically, the effects work as so:

```text
  Normal Line : 1364 cycles, 340 dots (0-339), four dots are 5-cycles long
  Long Line   : 1368 cycles, 341 dots (0-340), four dots are 5-cycles long
  Short Line  : 1360 cycles, 340 dots (0-339), all dots are 4-cycles long
```

Glitch: The long scanline is placed in the last line (directly after the Hsync
for line 0, thus shifting the Hsync position of Line 1, ie. of the first line
of the drawing period), accordingly, the upper some scanlines in interlaced
50Hz mode are visibly shifted to the right (by around one pixel), until after a
handful of scanlines the picture stablizes on the new hsync position (ie.
trying to display a vertical line will appear a little curved).

**Long and Short Scanlines (Purpose)**

The Scanline Rate doesn't match up with the PAL/NTSC Color Clocks, so, for
example, a red rectangle on black background will look like so:

```text
  RGB-Output             Composite-Output        Composite-Output
  Flawless               Static-Error            Flimmering-Error
  RRRRRRRRRRRRRRRR       RRRRRRRRRRRRRRRR        rRRRRRRRRRRRRRRRr
  RRRRRRRRRRRRRRRR        RRRRRRRRRRRRRRRR       rrRRRRRRRRRRRRRRrr
  RRRRRRRRRRRRRRRR         RRRRRRRRRRRRRRRR       rRRRRRRRRRRRRRRRr
  RRRRRRRRRRRRRRRR       RRRRRRRRRRRRRRRR        rRRRRRRRRRRRRRRRr
  RRRRRRRRRRRRRRRR        RRRRRRRRRRRRRRRR       rrRRRRRRRRRRRRRRrr
  RRRRRRRRRRRRRRRR         RRRRRRRRRRRRRRRR       rRRRRRRRRRRRRRRRr
```

Inserting the long/short scanlines does synchronize the Frame Rate with the
PAL/NTSC color clocks:

```text
  PAL Mode        Master Clocks (21MHz)       Color Clocks (PAL:4.4MHz)
  50Hz Normal     425568 (312*1364)           88660 (425568/6*5/4)
  50Hz Interlace  426936 (313*1364+4)         88945 (426936/6*5/4)
  NTSC Mode       Master Clocks (21MHz)       Color Clocks (NTSC:3.5MHz)
  30Hz Normal     714732 ((262+262)*1364-4)   119122 (714732/6)
  30Hz Interlace  716100 ((262+263)*1364)     119350 (716100/6)
```

The result is that the composite video output is producing the "Static Error"
effect (in the above example, the rectangle has sawtooth-edges). And the
"Flimmering" effect is avoided (which would have blurry edges, and which would
also appear as if the edges were wandering up) (Note: The flimmering effect can
be seen when switching a modded 50Hz PAL console to 60Hz mode).

<a id="snestiminghvevents"></a>

## SNES Timing H/V Events

**Summary of Vertical Timings**

```text
  V=0              End of Vblank, toggle Field, prefetch OBJs of 1st scanline
  V=0..224/239     Perform HDMA transfers (before each line & after last line)
  V=1              Begin of Drawing Period
  V=225/240        Begin of Vblank Period (NMI, joypad read, reload OAMADD)
  V=240            Short scanline in Non-interlaced 60Hz field=1
  V=311            Long scanline in Interlaced 50Hz field=1
  V=261/311        Last Line (in normal frames)
  V=262/312        Extra scanline (occurs only in Interlace Field=0)
  V=VTIME          Trigger V-IRQ or HV-IRQ
```

**Detailed List (H=Horizontal, V=Vertical, F=Field)**

```text
  H=0, V=0, F=0         SNES starts at this time after /RESET
  H=0, V=0              clear Vblank flag, and reset NMI flag (auto ack)
  H=0, V=225            set Vblank flag
  H=0.5, V=225          set NMI flag
  H=1                   clear hblank flag
  H=1, V=0              toggle interlace FIELD flag
  H=HTIME+3.5           H-IRQ
  H=2.5, V=VTIME        V-IRQ   (or HV-IRQ with HTIME=0)
  H=HTIME+3.5, V=VTIME  HV-IRQ  (when HTIME=1..339)
  H=6, V=0              reload HDMA registers
  H=10, V=225           reload OAMADD
  H=22-277(?), V=1-224  draw picture
  H=32.5..95.5, V=225   around here, joypad read begins (duration 4224 clks)
  H=133.5               around here, REFRESH begins (duration 40 clks/10 dots)
  H=274                 set hblank flag
  H=278, V=0..224       perform HDMA transfers
  H=323,327             seen as long-PPU-dots (but not as long-CPU-dots)
  H=323,327, V=240, F=1 seen as normal-PPU-dots (in short scanline 240) (60Hz)
  H=339                 this is last PPU-dot (in normal and short scanlines)
  H=340, V=311, F=1     this is last PPU-dot in long scanlines (50Hz+Interlace)
  CPU.H=339             this is last CPU-dot (in normal scanlines)
  CPU.H=338, V=240, F=1 this is last CPU-dot (in short scanlines)
  CPU.H=340, V=311, F=1 this is last CPU-dot (in long scanlines)
  H=0?, V=0             reset OBJ overflow flags in 213Eh (only if not f-blank)
  H=0?+INDEX*2, V=YLOC  set OBJ overflow bit6 (too many OBJs in next line)
  H=0?, V=YLOC+1        set OBJ overflow bit7 (too many OBJ pix in this line)
  ...?
```

xxx joypad read

xxx reload mosaic h/v counter (at some point during vblank)

xxx count mosaic v counter (at some point in each scanline)

Note that a PAL TV-set can display around 264 lines (about 25 more than
supported by the 239-line mode).

Note that a superscope pointing at pixel (X, Y) on the screen will latch

approximately dot X+40 on scanline Y+1.

**PPU H-Counter-Latch Quantities**

When latching PPU H/V-latches via reading [2137h] by software, 341*4 times at
evenly spread locations, one will statistically get following H values:

```text
  0..132    4 times (normal)
  133       3 times (occurs sometimes at H=133.5, and always at H=133.0)
  134       1 time  (occurs sometimes at H=134.x)
  135..142  never   (refresh is busy, cpu is stopped)
  143       1 time  (occurs sometimes at H=143.x)
  144       3 times (occurs sometimes at H=144.0, and always at H=144.5)
  145..322  4 times (normal)
  323       6 times (seen as long dot) (or 5.99 times if NTSC+InterlaceOff)
  324..326  4 times (normal)
  327       6 times (seen as long dot) (or 5.99 times if NTSC+InterlaceOff)
  328..339  4 times (normal)
  340       never   (doesn't exist) (or 0.01 times if PAL+InterlaceOn)
  341-511   never   (doesn't exist)
```

For the 1 and 3 times effect, one would expect the 40 clk refresh as so:

```text
  --- H=133.5--><--H=134.0 ------------------ H=143.5--><--H=144.0 ---
  ccccccccccccccRRRRRRRRRRRRRRRRRRRRRRRRRRRRRRRRRRRRRRRRcccccccccccccc
```

but, sometimes (randomly at 50:50 chance) it occurs somewhat like so:

```text
  ccccccccccccRRccRRRRRRRRRRRRRRRRRRRRRRRRRRRRRRRRRRRRRccRRccccccccccc
```

whereas "c"=CPU, "R"=Refresh (ie. sometimes, the CPU wakes up within the
Refresh period). Unknown why &amp; when exactly that stuttering refresh occurs.

Opcodes may begin before Refresh, and end after Refresh (rather than forcefully
finishing the Opcode before starting Refresh), so above statistics are same for
latching via "MOV A,[2137h]" (4 cycles) and "MOV A,[002137h]" (5 cycles)
opcodes.

Writing [4201h] (instead of reading [3137h]) shifts the visible refresh-related
H values (H=136..143=never, H=135/144 once, H=134/H=145 thrice), but obviously
doesn't alter the amounts of visible PPU-related H values.

<a id="snestimingppumemoryaccesses"></a>

## SNES Timing PPU Memory Accesses

Below is some info/guesses on what is happening during the 1364 master cycles
of a scanline. Plus some info/guesses on if/when/why it is (or isn't) possible
the change VRAM/CGRAM/OAM outside of V-Blank or Forced-Blank.

**PPU VRAM Load**

VRAM access time is 4 master cycles per word (aka 1 dot-cycle per word). In
each line, the PPU fetches 34*2 words (for 34 OBJ tiles of 8pix/4bpp), plus
33*8 words (for BgMap+BgTiles) (eg. 33*4 BgMap entries plus 33*4 BgTiles at
2bpp in BG Mode 0) (or less BG layers, but more bpp per tile in other BG
Modes).

As a whole, 4 master cycles per 34*2 + 33*8 words sums up to 1328 master cycles
(or possibly a bit more if accesses during "long dots" should happen to take 5
master cycles per word). 1328 would leave 36 unused master cycles per scanline
(or maybe MORE unused cycles if some BG layers are disabled? and/or if less
than 34 OBJ tiles are located in the scanline? unknown if VRAM can be accessed
during any such unused cycles? as far as known, existing games aren't trying to
do so).

**PPU Palette Load**

Not much known about CGRAM access timings. There would be some theories:

```text
  1) Maybe 512 accesses/line (for frontmost main/sub-screen pixels)
  2) Maybe 1296 accesses/line (for 256*4 BG pixels + 34*8 OBJ pixels)
  3) Maybe 1360 accesses/line (for 33*4*8 BG pixels + 34*8 OBJ pixels)
```

For the 1296/1360 accesses therories, CGRAM would be inaccessible for most of
the 1364 cycles.

In practice, it is possible to change CGRAM during certain timespots in the
Hblank-period (not during the &lt;whole&gt; Hblank period, but it works
&lt;sometimes&gt;, though doing so may require some research - and may end up
with more or less fragile timings). As for when/why it is working: Maybe there
some totally unused cycles, or it depends on how many OBJs are displayed in the
current scanline, possibly also on the number of used and/or enabled BG layers
(and thereby, maybe also allowing to change CGRAM outside of Hblank during
BG-drawing period?).

**PPU OAM load**

OAM handling is done in three steps:

```text
 1) scan 128 entries (collect max 32 entries per line)
 2) scan 32 entries (collect max 34 tiles per line)
 3) scan 34 entries (collect max 34x8 pixels per line)
```

OAM-Access time for Step 1 is 128*2 dots aka 128*8 master cycles (as seen in
STAT77.Bit6). OAM-Access times for Steps 2 and 3 is unknown (might be some
cycles per entry... or might be NULL cycles; in case OAM entries were copied to
separate buffer during previous Step... and/or NULL cycles if no OBJs are in
current scanline).

So, aside from the 256 known/used dot-cycles, there may (or may not) be up to
84 unused dot-cycles... possibly allowing to change OAM during Hblank(?).

Note: Mario Kart is using Forced Blank to change OAM in middle of screen.
Observe that the OAM address in Port 2102h is scattered during drawing period.

<a id="snespinouts"></a>

## SNES Pinouts

**External Connector Pinouts**

[SNES Controllers Pinouts](#snes-controllers-pinouts)

[SNES Audio/Video Connector Pinouts](#snes-audiovideo-connector-pinouts)

[SNES Power Supply](#snes-power-supply)

[SNES Expansion Port (EXT) Pinouts](#snes-expansion-port-ext-pinouts)

[SNES Cartridge Slot Pinouts](#snes-cartridge-slot-pinouts)

**Chipset Pinouts**

[SNES Chipset](#snes-chipset)

[SNES Pinouts CPU Chip](#snes-pinouts-cpu-chip)

[SNES Pinouts PPU Chips](#snes-pinouts-ppu-chips)

[SNES Pinouts APU Chips](#snes-pinouts-apu-chips)

[SNES Pinouts ROM Chips](#snes-pinouts-rom-chips)

[SNES Pinouts RAM Chips](#snes-pinouts-ram-chips)

[SNES Pinouts CIC Chips](#snes-pinouts-cic-chips)

[SNES Pinouts MAD Chips](#snes-pinouts-mad-chips)

[SNES Pinouts RTC Chips](#snes-pinouts-rtc-chips)

[SNES Pinouts Misc Chips](#snes-pinouts-misc-chips)

[SNES Pinouts GSU Chips](#snes-pinouts-gsu-chips)

[SNES Pinouts CX4 Chip](#snes-pinouts-cx4-chip)

[SNES Pinouts SA1 Chip](#snes-pinouts-sa1-chip)

[SNES Pinouts Decompression Chips](#snes-pinouts-decompression-chips)

[SNES Pinouts BSX Connectors](#snes-pinouts-bsx-connectors)

[SNES Pinouts NSS Connectors](#snes-pinouts-nss-connectors)

[SNES Pinouts Nintendo Power Flashcarts](#snes-pinouts-nintendo-power-flashcarts)

**Mods**

[SNES Common Mods](#snes-common-mods)

[SNES Controller Mods](#snes-controller-mods)

[SNES Xboo Upload (WRAM Boot)](#snes-xboo-upload-wram-boot)

<a id="snescontrollerspinouts"></a>

## SNES Controllers Pinouts

**Joypads (2)**

```text
  Pin  Dir  Port 1         Port2            ____________ _________________
  1    -    VCC +5VDC      VCC +5VDC       / 7   6   5  |  4   3   2   1  |
  2    Out  JOY-1/3 Clock  JOY-2/4 Clock  | GND IO6 IN3 | IN1 STB CK1 VCC | 1
  3    Out  JOY-STROBE     JOY-STROBE      \____________|_________________|
  4    In   JOY-1 Data     JOY-2 Data       ____________ _________________
  5    In   JOY-3 Data     JOY-4 Data      / 7  PEN  5  |  4   3   2   1  |
  6    I/O  I/O bit6       I/O bit7, Pen  | GND IO7 IN4 | IN2 STB CK2 VCC | 2
  7    -    GND            GND             \____________|_________________|
```

Pin 6 on Port 2 is shared for I/O and Lightpen input.

**Internal Connector**

The two joypad connectors (and power LED) are located on a small daughterboard,
which connects to the mainboard via an 11pin connector:

```text
  1 VCC
  2 IO6       ;-pad1
  3 IO7 / pen ;\
  4 IN2       ; pad2
  5 IN4       ;/
  6 IN1       ;\pad1
  7 IN3       ;/
  8 CK1 (one short LOW pulse per JOY1/JOY3 data bit)
  9 CK2 (one short LOW pulse per JOY2/JOY4 data bit)
  10 STB (one short HIGH pulse at begin of transfer)
  11 GND
```

For PAL consoles: The daughterboard contains diodes in the CK1, CK2, STB lines,
effectively making them open-collector outputs (so the joypad may require
pull-up resistors for that signals).

**SNES PAL vs NTSC Controllers**

SNES PAL consoles are passing CK1, CK2, STB lines through diodes (the diodes
are located on the controller connector daughterboard inside of the console,
and the diodes are effectively making that lines open-collector outputs, so PAL
controllers do require pull-up resistors for that signals).

SNES NTSC consoles don't have that diodes, and don't require pull-ups. Using
PAL controllers on NTSC consoles should work without problems.

For using NTSC controllers on PAL consoles: Remove or shortcut the diodes
inside of the SNES, or install pull-ups inside of the controller.

<a id="snesaudiovideoconnectorpinouts"></a>

## SNES Audio/Video Connector Pinouts

**RF Out (TV Modulator)**

Cinch with channel switch. Modulated video signal with mono-audio.

**Multi Out**

```text
  1   RGB - Red analog video out       ________________---________________
  2   RGB - Green analog video out    /  11    9     7     5     3     1  \
  3   RGB - H/V sync out             |                                     |
  4   RGB - Blue analog video out     \__12____10____8_____6_____4_____2__/
  5   Ground (used for Video)
  6   Ground (used for Audio)
  7   S-Video Y (luminance) out
  8   S-Video C (chroma) out
  9   Video Composite out (Yellow Cinch)
  10  +5V DC
  11  Audio Left out      (White Cinch)
  12  Audio Right out     (Red Cinch)
```

Pin 1,2,4: Red/Green/Blue (1V DC offset, 1V pp video into 75 ohms)

Pin 3,7,8,9: (1V pp into 75 ohms)

Pin 11,12: Left/Right (5V pp)

In cost-down SNES models, pin 1-4 and 7-8 are reportedly not connected (though
one can upgrade them with some small modifications on the mainboard).

<a id="snespowersupply"></a>

## SNES Power Supply

**AC 9V (PAL version)**

Power Supply input. The 9V AC input is internally converted to 9V DC, and is
then converted to 5V DC by a 7805 IC which turns the additional volts into
heat. The CPU/PPU/APU are solely operated at 5V DC (ie. you can feed 5V to the
7805 output). However, 9V DC are used by the Audio Amplifier, the console works
without the 9V supply, but you won't hear any sounds.

The Amplifier does work more less okay with 5V supply (ie. you can shortcut the
7805 input and output pins, and feed 5V to both of them) (when doing that
disconnect the original 9V input to make sure that 9V aren't accidently passed
to the CPU/PPU). However, at 5V, the middle amplitude levels are generated
linearily intact, but the MIN/MAX levels are chopped off, for example a
sawtooth wave looks like so:

```text
  Amplifier Output at 9V:                 Amplifier Output at 5V:
     /|   /|   /|   /|   /| -8000h +2.5V     _    _    _    _    _
    / |  / |  / |  / |  / |                 / |  / |  / |  / |  / | -3XXXh +1V
   /  | /  | /  | /  | /  |               _/  |_/  |_/  |_/  |_/  | +3XXXh -1V
  /   |/   |/   |/   |/   | +7FFFh -2.5V
```

The effect applies only if a game does use MIN/MAX levels (ie. there'd be no
problem at Master Volume of 40h or less; unless additional output comes from
Echo Unit).

**10V DC 850mA (NTSC version)**

Same as above, but using a DC supply (not AC), passed through a diode (so
internally the voltage may drop from 10V to around 9V).

**Power Switch / Anti-Eject**

In older SNES consoles the Power switch comes with an anti-eject lever, which
goes into a notch in the cartridge. Some carts have notches with square edges;
cart cannot be removed while power is on. Other carts have notches with
diagonal edge; the lever gets pushed (while sliding along the diagonal edge),
and power is switched off automatically when removing the cartridge.

The anti-eject type is probably (?) used by carts with battery-backed SRAM, in
order to prevent garbage writes (which might occur when simultaneously ejecting
and powering-down).

<a id="snesexpansionportextpinouts"></a>

## SNES Expansion Port (EXT) Pinouts

**EXT (at bottom of console) (used by Satellaview and Exertainment)**

Most pins are exactly the same as on the cartridge slot, the only special pins,
which aren't found on Cart slot, are EXT pins 21, 22, and 25: SMPCK is 8.192MHz
(24.576MHz/3) from APU, DOTCK is the PPU dot clock (around 5.3MHz, with some
stuttering during hblank period), MONO is a mono audio output.

```text
             .--------.--- SHIELD=GND
         PA0 |1      2| PA1          Bottom View of console:
         PA2 |3      4| PA3        .-------------------------------------.
         PA4 |5      6| PA5        |              (rear side)            |
         PA6 |7      8| PA7        | +----+---------------+              |
       /PAWR |9     10| /PARD      | |snap|  28 ...... 2  |              |
          D0 |11    12| D1         | | in |  27 ...... 1  |              |
          D2 |13    14| D3         | +----+---------------+              |
          D4 |15    16| D5         |           EXT                       |
          D6 |17    18| D7         |                                     |
      /RESET |19    20| +5VDC      |                                     |
      SMPCLK |21    22| DOTCK      |                                     |
         GND |23    24| EXPAND     |                                     |
  MONO-AUDIO |25    26| /IRQ       |                                     |
     L-AUDIO |27    28| R-AUDIO    |              (front side)           |
             '--------'            '-------------------------------------'
```

The L/R-AUDIO inputs are essentially same as on cart slot, although they are
passed through separate capacitors, so there is no 0 Ohm connection between EXT
and Cart audio pins. EXT Pin 24 connects to Cart pin 2 (aside from a 10K
pull-up it isn't connected anywhere inside of the console) so this pin is
reserved for communication between cartridge hardware and ext hardware (used by
Satellaview).

<a id="snescartridgeslotpinouts"></a>

## SNES Cartridge Slot Pinouts

**Cartridge Slot 62 pins (31x2 pins)**

Most cartridges are using only the middle 46 pins.

```text
  Front/Round    Rear/Flat
  Solder side    Component side
  MCK 21M - 01   32 - /WRAMSEL
  EXPAND  - 02   33 - REFRESH
  PA6     - 03   34 - PA7
  /PARD   - 04   35 - /PAWR
             <key>
  GND     - 05   36 - GND
  A11     - 06   37 - A12
  A10     - 07   38 - A13
  A9      - 08   39 - A14
  A8      - 09   40 - A15
  A7      - 10   41 - A16
  A6      - 11   42 - A17
  A5      - 12   43 - A18
  A4      - 13   44 - A19
  A3      - 14   45 - A20
  A2      - 15   46 - A21
  A1      - 16   47 - A22
  A0      - 17   48 - A23
  /IRQ    - 18   49 - /ROMSEL
  D0      - 19   50 - D4
  D1      - 20   51 - D5
  D2      - 21   52 - D6
  D3      - 22   53 - D7
  /RD     - 23   54 - /WR
  CIC0    - 24   55 - CIC1
  CIC2    - 25   56 - CIC3 3.072MHz (or 4.00MHz on older SNES)
  /RESET  - 26   57 - SYSCK
  +5V     - 27   58 - +5V
             <key>
  PA0     - 28   59 - PA1
  PA2     - 29   60 - PA3
  PA4     - 30   61 - PA5
  SOUND-L - 31   62 - SOUND-R
  GND     - SHIELD  - GND
```

Caution: The connector uses a nonstandard 2.5mm pitch (not 2.54mm). And, the
PCB is only 1.2mm thick (not 1.5mm).

The width of the key gaps equals to 2 pins each (ie. the overall connector size
is 35x2 pins, with 31x2 used pins, and two unused 2x2 pin clusters).

**Pin assignments**

```text
 A23-0, D7-0, /WR, /RD - CPU address/data bus, read/write signals
 /IRQ      - Interrupt Request (used by SA-1 and GSU)
 /RESET    - When the system is reset (power-up or hard reset) this goes low
 /WRAMSEL  - Work RAM select (00-3F,80-BF:0000-1FFF, 7E-7F:0000-FFFF)
 /ROMSEL   - Cart ROM select (00-3F,80-BF:8000-FFFF, 40-7D,C0-FF:0000-FFFF)
 PA7-0     - Address bus for $2100-$21FF range in banks $00-$3F/$80-$BF (B-Bus)
 /PAWR     - Write strobe for B-Bus
 /PARD     - Read strobe for B-Bus
 MCK       - 21.47727 MHz master clock (used by SGB1 and MarioChip1)
 SYSCK     - Unknown, is an output from the CPU.
 SOUND-L/R - Left/Right Analog Audio Input, mixed with APU output (SGB, MSU1)
 EXPAND    - Connected to pin 24 of the EXT expansion port (for Satellaview)
 REFRESH   - DRAM refresh (connects to WRAM, also used by SGB and SA-1)
              four HIGH pulses every 60us (every scanline)
              Used by SGB (maybe to sense SNES hblanks?)
 CIC0      - Lockout Data to CIC chip in console    ;\from/to=initial direction
 CIC1      - Lockout Data from CIC chip in console  ;/(on random-seed transfer)
 CIC2      - Lockout Start (short HIGH pulse when releasing reset button)
 CIC3      - Lockout Clock (3.072MHz) (24.576MHz/8 from APU) (or 4.00MHz)
 SHIELD    - GND (connected in SA-1 carts, SGB-also has provisions)
```

**Physical Cartridge Shape**

```text
                            Front Side
        _________________                _________________
   .--''                 ''--.    .-----'                 '-----.
  /    Japan NTSC and PAL     \   |           US NTSC           |
  |___________________________|   |_:":_____________________:":_|
```

```text
                             Rear Side
```

<a id="sneschipset"></a>

## SNES Chipset

**Chipset (PAL)**

```text
 Board:     (C) 1992 Nintendo, SNSP-CPU-01        ;BOARD
 U1  100pin Nintendo, S-CPU A, 5A22-02, 2FF 7S    ;CPU  (ID=2 in 4210h)
 U2  100pin Nintendo, S-PPU1, 5C77-01, 2EU 64     ;PPU1 (ID=1 in 213Eh)
 U3  100pin Nintendo, S-PPU2 B, 5C78-03, 2EV 7G   ;PPU2 (ID=3 in 213Fh)
 U4  28pin  SONY JAPAN, CXK58257AM-12L, 227M87EY  ;VRAM1 32Kx8 SRAM
 U5  28pin  SONY JAPAN, CXK58257AM-12L, 227M87EY  ;VRAM2 32Kx8 SRAM
 U6  64pin  Nintendo, W-WRAM, 9227 T23 F          ;WRAM
 U7  24pin  S-ENC, Nintendo, S (for Sony) 9226 B  ;video RGB to composite
 U8  18pin  F413, (C) 1992, Nintendo, 9209 A      ;CIC
 U9  -      N/A (NTSC version only, type 74HCU04) ;hex inverter (for X1 & CIC)
 U10 14pin  (M)224, AN1324S (equivalent to LM324) ;SND Quad Amplifier
 U11 3pin   T529D, 267                            ;GND,VCC,/RESET
 U12 3pin   17805, 2F2, SV, JAPAN                 ;5V
 U13 64pin  Nintendo, S-SMP, SONY, Nintendo'89... ;SND1 (SPC700 CPU)
 U14 80pin  Nintendo, S-DSP, SONY'89, WWW149D4X   ;SND2 (sound chip)
 U15 28pin  MCM51L832F12, (M) JAPAN RZZZZ9224     ;SND-RAM1 32Kx8 SRAM
 U16 28pin  MCM51L832F12, (M) JAPAN RZZZZ9224     ;SND-RAM2 32Kx8 SRAM
 U17 16pin  NEC, D6376, 9225CJ (ie. NEC uPD6376)  ;SND-Dual 16bit D/A
 U18 14pin  S-CLK, 2FS 4A (for PAL only)          ;X1 to 21.2MHz and 4.43MHz
 TC1 2pin   Red Trimmer                           ;X1-ADJUST
 X1  2pin   D177F2                                ;CPU/PPU 17.7344750MHz (PAL)
 X2  2pin   CSA, 24.57MX, Gm J                    ;SND 24.576MHz
 F1  2pin   SOC, 1.5A                             ;FUSE (supply-input)
 T1  4pin   TDK, ZJYS-2, t                        ;DUAL-LOOP (supply-input)
 DB1 4pin   TOSHIBA, 4B1, 1B 2-E JAPAN            ;AC-DC (PAL/AC-version only)
 L1  2pin   220 (22uH)                            ;LOOP (color clock to GND)
 VR1 2pin   (M)ZNR, FK220, 26                     ;? (supply)
 J1? 2pin   AC Input 9V                           ;AC-IN
 J2  4pin   SNSP CCIR-EEC, A E210265, 250142A     ;RF-Unit (modulator)
 SW  4pin   Reset Button (on board)
 P1  64pin  Cartridge Slot
 P2  11pin  To Front Panel (Controllers/Power LED)
 P3  2pin   To Power Switch
 P4  12pin  Multi Out
 P5  28pin  EXT Expansion Port (bottom side)
```

**Costdown SNES chipset**

```text
  U1 160pin  Nintendo S-CPUN A, RF5A122 (CPU, PPU1, PPU2, S-CLK)
  U2 100pin  Nintendo S-APU             (S-SMP, S-DSP, 64Kx8 Sound RAM)
  U3  64pin  Nintendo S-WRAM B
  U4  28pin  32Kx8 SRAM (video ram)
  U5  28pin  32Kx8 SRAM (video ram)
  U6?  8pin? ?
  U7  24pin  Nintendo S-RGB A
  U8  18pin  Nintendo F411B (CIC)
  U9   3pin  17805 (5V supply)
  U10 14pin  S-MIX A (maybe sound amplifier?)
  U11  3pin  Reset?
  X1   2pin  D21G8N (21.4MHz NTSC, or 17.7MHz PAL)
  X2   2pin  APU clock (probably the usual 24.576MHz?)
```

51832

```text
  Toshiba TC51832FL-12 32k x8 SRAM (SOP28)
```

CXK58257AM-12L

```text
  32768-word x 8-bit high speed CMOS static RAM, 120ns,
  standby 2.5uW in 28-pin SOP package.
  Operational temperature range from 0'C to 70'C.
```

<a id="snespinoutscpuchip"></a>

## SNES Pinouts CPU Chip

**CPU 5A22**

```text
  1     In  VCC (supply)
  2-17  Out A8..A23
  18    In  GND (supply)
  19-26 I/O JOY-IO-0..7 (Port 4201h/4213h.Bit0..7)
  27-31 In  JOY-2 (4017h.Read.Bit0..4) (Pin 29-31 wired to GND)
  32-33 In  JOY-1 (4016h.Read.Bit0..1)
  34    In   "VCC" (unknown, this is NOT 4016h.Bit2) (wired to VCC)
  35    Out JOY-1-CLK (strobed on 4016h.Read)
  36    Out JOY-2-CLK (strobed on 4017h.Read)
  37-39 Out JOY-OUT0..2 (4016h.Write.Bit0-2, OUT0=JOY-STROBE,OUT1-OUT2=UNUSED?)
  40    Out REFRESH (DRAM refresh for WRAM, four HIGH pulses per scanline)
  41-42 In  TCKSEL0,TCKSEL1 (wired to GND) (purpose unknown)
  43-44 In  HBLANK,VBLANK (from PPU, for H/V-timers and V-Blank NMI)
  45    In  /NMI (wired to VCC)
  46    In  /IRQ (wired to Cartridge and Expansion Port)
  47    In  GND (supply)
  48    In  MCK ;21.47727 MHz master clock ;measured ca.21.666666MHz? low volts
  49    In  /DRAMMODE (wired to GND) (allows to disable DRAM refresh)
  50    In  /RESET
  51-58 Out PA0..PA7
  59    In  VCC (supply)
  60-67 I/O D0-D7
  68    Out /PARD
  69    Out /PAWR
  70    Out /DMA (NC)    HI
  71    Out CPUCK (NC)      LOOKS SAME AS PIN72 (MAYBE PHASE-SHIFTED?)
  72    Out SYSCK (to WRAM and Cartridge) FAST CLK... TYPICALLY 2xHI, 1xLO
  73    In  TM (wired to GND) (purpose unknown)
  74    In  HVCMODE (wired to GND) (purpose unknown)
  75    In  HALT (wired to GND) (purpose unknown) (related to RDY?)
  76    In  /ABORT (wired to VCC)
  77    Out /ROMSEL (access to 00-3F/80-BF:8000-FFFF or 40-7D/C0-FF:0000-FFFF)
  78    Out /WRAMSEL (access to 00-3F/80-BF:0000-1FFF or 7E-7F:0000-FFFF)
  79    In  GND (supply)
  80    Out R/W (NC) (ie. almost same as /WR, but with longer LOW-duty)
  81    In  RDY (wired to VCC) (schematic says "PE" or RE" or so?)
  82    Out /ML (NC) (memory lock,low on read-modify,ie.inc/dec/shift/etc)
  83    Out MF (NC) (CPU's M-Flag, 8bit/16bit mode)
  84    Out XF (NC) (CPU's X-Flag, 8bit/16bit mode)
  85    In  VCC (supply)
  86    Out VFB or VPB or so (NC)    RAPID PULSED
  87    Out VFA or VPA or so (NC)    RAPID PULSED
  88    Out ALCK                     LOOKS LIKE INVERSE OF PIN71 or PIN72
  89    Out /VP (NC) (vector pull, low when executing exception vector)
  90    In  GND
  91    Out /WR (low on any memory write, including io-writes to 21xxh/4xxxh)
  92    Out /RD
 93-100 Out  A0..A7
```

The three VCC pins are interconnected inside of the chip (verified).

The various GND pins are not verified (some may be supply, or inputs).

<a id="snespinoutsppuchips"></a>

## SNES Pinouts PPU Chips

**S-PPU1, 5C77**

```text
  1     ?      TST1 (GND)
  2     ?      TST0 (GND)
  3     CPU    /PARD
  4     CPU    /PAWR
  5-12  CPU    PA7-PA0 (main cpu b-bus)
  13    Supply VCC
  14-21 CPU    D7-D0 (main cpu)
  22    Supply GND
  23    ?      HVCMODE (GND)
  24    Mode   PALMODE (VCC=PAL or GND=NTSC)
  25    ?      /MASTER (GND)
  26    ?      /EXTSYNC (VCC)
  27    ?      NC (GND)
  28-35 SRAM   DH0-DH7 (sram data bus high-bytes)
  36    Supply VCC
  37-44 SRAM   DL0-DL7 (sram data bus low-bytes)
  45    Supply GND
  46    SRAM   VA15 (NC)  (would be for 64K-word RAM, SNES has only 32K-words)
  47    SRAM   VA14       (sram address bus for upper/lower 8bit data)
  48-61 SRAM   VAB13-VAB0 (sram address bus for upper 8bit data)
  62    Supply VCC
  63-76 VRAM   VAA13-VAA0 (sram address bus for lower 8bit data)
  77    Supply GND
  78    VRAM   /VAWR (sram write lower 8bit data)
  79    VRAM   /VBWR (sram write upper 8bit data)
  80    VRAM   /VRD  (sram read 16bit data)
  81    Supply VCC
  82-85 PPU    CHR3-CHR0      ;\
  86-87 PPU    PRIO1-PRIO0    ;
  88-90 PPU    COLOR2-COLOR0  ;/
  91           /VCLD (20ms high, 60us low)  (LOW during V=0)
  92           /HCLD (60us high, 0.2us low) (low during 11th dot-cycle
                                             of the 15-cycle color burst)
  93           /5MOUT (shortcut with pin 97, /5MIN) (and to PPU2, /5MIN)
                           8 clks = 7.5*0.2us = 5.333MHz
  94           /OVEP (always high?) (to PPU2 /OVER1 and /OVER2)
  95           FIELD (NTSC: 30Hz, PAL: 25Hz) (signalizes even/odd frame)
  96    Supply GND
  97           /5MIN (shortcut with pin 93, /5MOUT)  (as above 5mout)
  98    PPU    /RESET (from PPU2 /RESOUT0)
  99    ?      TST2 (GND)
  100   System XIN (21MHz)
```

**S-PPU2, 5C78**

```text
  1     Video  /BURST    (LOW for 15 dot-clocks, thereof 1st dot is LONG)
  2     ?      /PED (NC) (ca. 15kHz, hblank related, 50us high, 10us low)
  3     Video  3.58M (to NTSC encoder)    5 clks = 1.4us
  4     ?      /TOUMEI (NC) (LOW during V-Blank and H-Blank) (or vram access?)
  5     Supply VCC
  6     CPU    /PAWR
  7     CPU    /PARD
  8-15  CPU    D7-D0
  16    Supply GND
  17-24 CPU    PA7-PA0
  25    CPU    HBLANK (for CPU h/v-timers)
                high during last some pixels, right border HSYNC,lead,burst
                low during last some burst clks, left border, and most pixels
  26    CPU    VBLANK (for CPU h/v-timers)
                high during VBLANK (line 225-261)
                low during prepare (line 0) and picture (line 1-224)
  27           /5MOUT (via 100 ohm to DOTCK on Expansion Port Pin22)
  28    System /RESOUT1 (via 1K to CPU, APU, Cartridge, Expansion, etc.)
  29    Joy    EXTLATCH (Lightpen signal)
  30    Mode   PALMODE (VCC=PAL or GND=NTSC)
  31    System XIN (21MHz)
  32    Supply VCC
  33    PPU    /RESOUT0 (to PPU1 /RESET)
  34    CIC    /RESET (from CIC Lockout chip & Reset Button)
  35    Supply GND
  36           FIELD (NTSC: 30Hz, PAL: 25Hz) (signalizes even/odd frame)
  37           /OVER1   ??
  38           /5MIN (from PPU1)
  39           /HCLD (low during 11th dot-cycle of the 15-cycle color burst)
  40           /VCLD (LOW during V=0)
  41-43 PPU    OBJ0-OBJ2 (COLOR0-COLOR2)
  44-45 PPU    OBJ3-OBJ4 (PRIO0-PRIO1)
  46-49 PPU    OBJ5-OBJ8 (CHR0-CHR3)
  50           /OVER2
  51-58 SRAM   VDB0-VDB7 (sram data upper 8bit)
  59    Supply VCC
  60-67 SRAM   VDA0-VDA7 (sram data lower 8bit)
  68    Supply GND
  69-76 SRAM   EXT0-EXT7 (sram data upper 8bit) (shortcut with VDB0-VDB7)
  77-82 ?      TST0-TST5 (NC) (always low?)
  83    Supply VCC
  84-89 ?      TST6-TST11 (NC) (always low?)
  90-93 ?      TST12-TST15 (GND)
  94    Supply AVCC (VCC)
  95-97 Video  R,G,B (Analog RGB Output)
  98    ?      HVCMODE (GND)
  99    Supply GND
  100   Video  /CSYNC (normally LOW during Hsync, but inverted during Vsync)
```

**CPUN-A (160pin chip with CPU and PPU1 and PPU2 in one chip)**

Used in newer cost-down SNES consoles. Pinouts unknown.

<a id="snespinoutsapuchips"></a>

## SNES Pinouts APU Chips

**S-DSP Pinouts (Sound Chip)**

```text
  1     CK     DKD (NC)              (5 clks = 1.2us) 4.096MHz (24.576MHz/6)
  2     CK     MXK or MYX or so (NC) (5 clks = 1.6us) 3.072MHz (24.576MHz/8)
  3-5   CK     MX1-MX3 (NC) 1.024MHz (24.576MHz/24) (3pins: phase/duty shifted)
  6-8   SRAM   MD2-MD0 (SRAM Data)
  9-11  SRAM   MA0-MA2 (SRAM Address)
  12    Supply GND
  13-19 SRAM   MA3-MA7,MA12,MA14 (SRAM Address)
  20    SRAM   MA15 (NC) (instead, upper/lower 32K selected via /CE1 and /CE0)
  21    ?      DIP (NC) (always high?)
  22-32 SRAM   MD3,MD4,MD5,MD6,MD7,/CE1,/CE0,MA10,/OE,MA11,MA9
  33    Supply VCC
  34-36 SRAM   MA8,MA13,/WE
  37    ?      TF (GND)    ;\wiring TF and/or TK to VCC crashes the SPC700
  38    ?      TK (GND)    ;/(ie. they seem to mess up CPUK clock or SRAM bus)
  39    Audio  /MUTE (to/after amplifier)
  40    CK     MCK (NC) 64000Hz  (24.576MHz/24/16)
  41    CIC    SCLK     3.072MHz (24.576MHz/8) (via inverters to "CIC" chips)
  42    Audio  BCK      1.536MHz (24.576/16)       BitClk    ;\to uPD6376
  43    Audio  LRCK     32000Hz  (24.576MHz/16/48) StereoClk ; D/A converter
  44    Audio  DATA Data Bits (8xZeroPadding+16xData)        ;/
  45-46 Osc    XTAO,XTAI (24.576MHz)
  47    System /RESET
  48    SPC700 CPUK     2.048MHz (24.576MHz/12) (to S-SMP)
  49    SPC700 PD2 (on boot: always high?)
  50    SPC700 PD3 (on boot: always low?)
  51    SPC700 D0
  52    Supply GND
  53-59 SPC700 D1-D7
  60-72 SPC700 A0-A12
  73    Supply VCC
  74-76 SPC700 A13-A15
  77    CK     XCK 24.576MHz (24.576MHz/1) (NC)
  78    Exp.   DCK  8.192MHz (24.576MHz/3) (to Expansion Port Pin 21, SMPCLK)
  79    CK     CK1 12.288MHz (24.576MHz/2) (NC)
  80    CK     CK2  6.144MHz (24.576MHz/4) (NC)
```

**S-SMP Pinouts (SPC700 CPU)**

```text
  1-5   DSP    A4..A0 Address Bus
  6-13  DSP    D7..D0 Data Bus
  14    DSP    PD3 (maybe R/W signal or RAM/DSP select?)
  15    DSP    PD2 (maybe R/W signal or RAM/DSP select?)
  16    DSP    CPUK (2.048MHz from DSP chip)
  17    AUX    /P5RD (NC)
  18-25 AUX    P57..P50 (NC)
  26    Supply GND
  27-34 AUX    P47..P40 (NC)
  35    ?      T1 (NC or wired to VCC) (maybe test or timer?)
  36    ?      T0 (NC or wired to VCC) (maybe test or timer?)
  37    System /RESET
  38-45 CPU    D7..D0 Data Bus
  46-51 CPU    /PARD,/PAWR,PA1,PA0,PA6 (aka CS), PA7 (aka /CS) B-Address Bus
  52-56 DSP    A15-A11 Address Bus
  57    Supply VCC (5V)
  58    Supply GND
  59-64 DSP    A10-A5 Address Bus
```

Pin 1-16 and 52-64 to S-DSP chip, Pin 17-34 Aux (not connected), Pin35-36 are
NC on real hardware (but are wired to VCC in schematic), pin 37-51 to main CPU.

CPUK caution:

```text
 scope measure with "x10" ref (gives the correct signal):
   -_-_-_-_-_-_-_-_ 2.048MHz (0.5us per cycle)
 during (and AFTER) "x1" ref (this seems to "crash" the clock generator):
   ---_---_---_---_ 1.024MHz (1.0us per cycle) (with triple-high duty)
```

**S-APU Pinouts (S-SMP, S-DSP, 64Kx8 Sound RAM)**

This 100pin chip is used in newer cost-down SNES consoles.

```text
  1-100 Unknown
```

It combines the S-SMP, S-DSP, and the two 32Kx8 SRAMs in one chip. And,
possibly also the NEC uPD6376 D/A converter?

<a id="snespinoutsromchips"></a>

## SNES Pinouts ROM Chips

```text
             Standard SNES ROMs                        EPROM-style ROMs
           __________   __________
      GND | 01       \_/       40 | VCC
      GND | 02 ......   .......39 | VCC               ________   ________
  --> A20 | 03 01    \./    36 38 | VCC            ? | 01     \_/     36 | VCC
  GND,A21 | 04 02 ...   ... 35 37 | A22,GND <--    ? | 02 ....   .... 35 | ?
  --> A17 | 05 03 01 \./ 32 34 36 | NC,VCC  <--   NC | 03 01  \./  32 34 | VCC
  --> A18 | 06 04 02     31 33 35 | /CS <--      A16 | 04 02       31 33 | NC
      A15 | 07 05 03     30 32 34 | A19 <--      A15 | 05 03       30 32 | A17
      A12 | 08 06 04     29 31 33 | A14          A12 | 06 04       29 31 | A14
       A7 | 09 07 05 ROM 28 30 32 | A13           A7 | 07 05 EPROM 28 30 | A13
       A6 | 10 08 06     27 29 31 | A8            A6 | 08 06 style 27 29 | A8
       A5 | 11 09 07     26 28 30 | A9            A5 | 09 07 (eg.  26 28 | A9
       A4 | 12 10 08     25 27 29 | A11           A4 | 10 08 in    25 27 | A11
       A3 | 13 11 09     24 26 28 | A16 <--       A3 | 11 09 SGB)  24 26 | /OE
       A2 | 14 12 10     23 25 27 | A10           A2 | 12 10       23 25 | A10
       A1 | 15 13 11     22 24 26 | /RD           A1 | 13 11       22 24 | /CS
       A0 | 16 14 12     21 23 25 | D7            A0 | 14 12       21 23 | D7
       D0 | 17 15 13     20 22 24 | D6            D0 | 15 13       20 22 | D6
       D1 | 18 16 14     19 21 23 | D5            D1 | 16 14       19 21 | D5
       D2 | 19 17 15     18 20 22 | D4            D2 | 17 15       18 20 | D4
      GND | 20 18 16     17 19 21 | D3           GND | 18 16       17 19 | D3
          |_______________________|                  |___________________|
```

Note that Standard SNES ROMs have /CS and A16..A22 located elsewhere as on
normal EPROMs. Most common SNES ROMs are 32pin or 36pin (the 40pin ROMs are
used by some SPC7110 games; these chips are using a bigger package, though
without actually having more address lines). Most SNES carts are using DIP
chips (smaller SMD ROMs are used only in carts that contain SMD coprocessors).

Mind that SNES "LoROM" cartridges are leaving SNES.A15 unused (and do instead
connect "ROM.A15 and up" to "SNES.A16 and up").

**44pin ROMs**

```text
          _____   _____
 /WE A22 |  1  \_/  44 | A21 /WP
     A19 |  2       43 | A20
     A18 |  3       42 | A9
     A8  |  4       41 | A10
     A7  |  5       40 | A11
     A6  |  6       39 | A12
     A5  |  7       38 | A13
     A4  |  8       37 | A14
     A3  |  9       36 | A15
     A2  | 10       35 | A16
     A1  | 11       34 | A17
     /CE | 12       33 | BHE (HI)
     GND | 13       32 | GND
     /OE | 14       31 | D15,A0
     D0  | 15       30 | D7
     D8  | 16       29 | D14
     D1  | 17       28 | D6
     D9  | 18       27 | D13
     D2  | 19       26 | D5
     D10 | 20       25 | D12
     D3  | 21       24 | D4
     D11 | 22       23 | VCC
         |_____________|
```

44pin ROMs are used by (some) SPC7110 boards (with 8bit databus), and by (all)
S-DD1 and SA-1 boards (existing photos look as if: with 16bit databus).

44pin FLASH is used in Nintendo Power carts (with 8bit databus) (with /WE and
/WP instead of A22 and A21).

<a id="snespinoutsramchips"></a>

## SNES Pinouts RAM Chips

**128K WRAM Pinouts**

```text
  1 VCC      9  -            17 GND 25 A1  33 GND 41 A8      49 VCC    57 /RD
  2 D4       10 CS,VCC       18 -   26 A10 34 A13 42 ENA,A22 50 PS,PA7 58 /PAWR
  3 D5       11 CS,VCC       19 -   27 A2  35 A5  43 /PS,PA2 51 PS,VCC 59 /WR
  4 D6       12 CS,VCC       20 -   28 A11 36 A14 44 /PS,PA3 52 PS,VCC 60 D0
  5 D7       13 /CS,GND      21 -   29 A3  37 A6  45 /PS,PA4 53 PA0    61 D1
  6 SYSCK    14 /CS,GND      22 -   30 A12 38 A15 46 /PS,PA5 54 PA1    62 D2
  7 REFRESH  15 /CS,/WRAMSEL 23 A0  31 A4  39 A7  47 /PS,PA6 55 G (NC) 63 D3
  8 /RESET   16 VCC          24 A9  32 VCC 40 A16 48 GND     56 /PARD  64 GND
```

Note: The WRAM is Dynamic RAM (DRAM) and does require REFRESH pulses. If
REFRESH is disabled (via DRAMMODE pin) then WRAM forgets its content after some
minutes. Whereas, refresh occurs also on /RD (with the refresh row output on
A0..A8), so, for example, games that are DMA'ing 512 bytes from WRAM to OAM in
every frame should work perfectly without REFRESH.

Interestingly, WRAM is kept intact even if the RESET button is held down for
about 30 minutes (although the CPU doesn't generate any /RD, REFRESH, nor
A0..A8 cycles during that time).

**SRAM Pinouts**

```text
         .-------------__-------------.
      NC |1                         36| VCC
     A20 |2  ..........__.......... 35| A19
  NC,A18 |3  1                   32 34| NC,VCC (NC, ie. not CE2)
     A16 |4  2  .......__....... 31 33| A15
  NC,A14 |5  3  1             28 30 32| A17,CE2,VCC
     A12 |6  4  2  ....__.... 27 29 31| /WE
      A7 |7  5  3  1       24 26 28 30| A13,CE2,VCC
      A6 |8  6  4  2       23 25 27 29| A8
      A5 |9  7  5  3       22 24 26 28| A9
      A4 |10 8  6  4  SRAM 21 23 25 27| A11,/WE
      A3 |11 9  7  5       20 22 24 26| /OE
      A2 |12 10 8  6       19 21 23 25| A10
      A1 |13 11 9  7       18 20 22 24| /CE,/CE1
      A0 |14 12 10 8       17 19 21 23| D7
      D0 |15 13 11 9       16 18 20 22| D6
      D1 |16 14 12 10      15 17 19 21| D5
      D2 |17 15 13 11      14 16 18 20| D4
     GND |18 16 14 12      13 15 17 19| D3
         '----------------------------'
```

28pin 32Kbyte SRAM is used for Video RAM and Sound RAM (on mainboard)

Various SRAM sizes are used in game cartridges.

<a id="snespinoutscicchips"></a>

## SNES Pinouts CIC Chips

**CIC Pinouts**

F411/F413: 18pin SMD-chip (used in console and in some carts)

D411/D413: 16pin DIP-chip (used in most carts)

```text
  SMD DIP Pin Dir Usage  In Console               In Cartridge
  1   1   P00 Out DTA0   Cart.55 CIC1             Cart.24 CIC0
  2   2   P01 In  DTA1   Cart.24 CIC0             Cart.55 CIC1
  3   3   P02 In  RANDOM Via capacitor to VCC     NC
  4   4   P03 In  MODE   VCC=Console (Lock)       GND=Cartridge (Key)
  5       NC  -   (NC)   NC                       NC
  6   5   CL2 -   (NC)   NC                       SMD:NC or DIP:GND
  7   6   CL1 In  CLK    3.072MHz (from APU)      Cart.56 CIC3 (3.072MHz)
  8   7   RES In  RESET  From Reset button        Cart.25 CIC2 (START)
  9   8   GND -   GND    Supply                   Supply
  10  9   P10 Out /RESET To PPU (and CPU/APU/etc) NC (or to ROM, eg. in SGB)
  11  10  P11 Out START  Cart.25 CIC2             NC
  12  11  P12 -   (NC)   NC                       NC (or SlotID in FamicomBox)
  13  12  P13 -   (NC)   NC                       NC (or SlotID in FamicomBox)
  14      NC  -   (NC)   NC                       NC
  15  13  P20 -   (NC)   NC                       NC
  16  14  P21 -   (NC)   NC                       NC (or SlotID in FamicomBox)
  17  15  P22 -   (NC)   NC                       NC (or SlotID in FamicomBox)
  18  16  VCC -   VCC    Supply                   Supply
```

P00=Out,P01=In are the initial directions (for the Random Seed transfer), later
on the directions are randomly swapped (ie. P00=In,P01=Out or P00=Out,P01=In).

START: short HIGH pulse on power-up or when releasing reset button.

/RESET: in console: to PPU, and from there to CPU,APU,Cart,Expansion.

<a id="snespinoutsmadchips"></a>

## SNES Pinouts MAD Chips

**MAD-1 (and MAD-1 A) Pinouts (Memory Address Decoder 1)**

```text
  1  OUT1 /ROM.CS1 ;Chipselect to Upper ROM (NC if single ROM)
  2  OUT2 /SRAM.CS ;Chipselect to SRAM
  3  OUT3 /AUX.CS  ;Chipselect to Expansion I/O or so (usually NC)
  4  OUT4 /ROM.CS  ;Chipselect to Single ROM (NC if two ROMs)
  5  Vout          ;Supply to SRAM (+3V when VCC=off, +5V when VCC=on)
  6  VCC           ;Supply from SNES (+5V)
  7  Vbat          ;Supply from Battery via resistor (+3V)
  8  GND           ;Supply Ground
  9  IN6  /RESET   ;From cart.26
  10 IN5  MODE     ;HiROM: VCC                  | LoROM: GND
  11 IN4  /ROMSEL  ;From cart.49
  12 IN3  Addr3    ;HiROM: A22 (400000h) or A15 | LoROM: A22 (400000h) or VCC
  13 IN2  Addr2    ;HiROM: A21 (200000h)        | LoROM: A21 (200000h)
  14 IN1  Addr1    ;HiROM: A14 (4000h)          | LoROM: A20 (100000h)
  15 IN0  Addr0    ;HiROM: A13 (2000h)          | LoROM: A15 (8000h)
  16 OUT0 /ROM.CS0 ;Chipselect to Lower ROM (NC if single ROM)
```

Note that Addr3 is sometimes wired this or that way. And, when using two ROMs,
Addr2 is used as upper ROM address line (eg. Addr2=A20 for a cart with two
1Mbyte ROM chips) (so the ROM-size may also affect the SRAM/AUX mapping;
according to the Addr2 wiring).

**MAD-1 (and MAD-1 A) Logic Table**

```text
  IN0   IN1   IN2   IN3   IN4   IN5   IN6   --> Output being
  Addr0 Addr1 Addr2 Addr3 /ROM  MODE  /RES      dragged LOW
  -----------------------------------------------------------
  HIGH  x     x     x     LOW   LOW   HIGH  --> /ROM.CS=LOW   ;\
  HIGH  x     LOW   x     LOW   LOW   HIGH  --> /ROM.CS0=LOW  ;
  HIGH  x     HIGH  x     LOW   LOW   HIGH  --> /ROM.CS1=LOW  ; LoROM
  LOW   LOW   HIGH  HIGH  LOW   LOW   HIGH  --> /AUX.CS=LOW   ;
  LOW   HIGH  HIGH  HIGH  LOW   LOW   HIGH  --> /SRAM.CS=LOW  ;/
  x     x     x     x     LOW   HIGH  HIGH  --> /ROM.CS=LOW   ;\
  x     x     LOW   x     LOW   HIGH  HIGH  --> /ROM.CS0=LOW  ;
  x     x     HIGH  x     LOW   HIGH  HIGH  --> /ROM.CS1=LOW  ; HiROM
  HIGH  HIGH  LOW   LOW   HIGH  HIGH  HIGH  --> /AUX.CS=LOW   ;
  HIGH  HIGH  HIGH  LOW   HIGH  HIGH  HIGH  --> /SRAM.CS=LOW  ;/
```

**MAD-2 Pinouts (Memory Address Decoder 2)**

Unknown. Used in some newer DSPn cartridges (mainly/only DSP1B ones?) (probably
similar to MAD-1, but maybe with some pins replaced by a clock amplifier for
the DSPn oscillator... and/or maybe it supplies an inverted RESET signal to the
DSPn chip).

**MAD-R Pinouts (Memory Address Decoder with Reset-Inverter)**

Pinouts are same as MAD-1, except for one pin: Pin4 outputs RESET (inverse of
/RESET input) (there aren't any known cartridges using this feature).

The SHVC-2A3M-01, SHVC-2A3M-10, SHVC-2J3M-01 boards can be fitted with either
MAD-1 or MAD-R (later SHVC-2A3M-11, SHVC-2J3M-11, SHVC-2J3M-20 boards are used
with MAD-1 only, so MAD-R appears to have been discontinued after soon).

```text
  XXX The following boards do (also) accept either MAD-1 or MAD-R:
  2A1M-01
  2A5M-01
  2J5M-01
```

**MAD-R Logic Table**

```text
  IN0   IN1   IN2   IN3   IN4   IN5   IN6   --> Output being
  Addr0 Addr1 Addr2 Addr3 /ROM  MODE  /RES      dragged LOW
  -----------------------------------------------------------
  x     x     x     x     x     x     HIGH  --> RESET=LOW    * ;-Reset
  x     x     LOW   LOW   LOW   LOW   HIGH  --> /ROM.CS0=LOW * ;\
  x     x     HIGH  LOW   LOW   LOW   HIGH  --> /ROM.CS1=LOW * ; LoROM
  x     x     LOW   HIGH  LOW   LOW   HIGH  --> /AUX.CS=LOW  * ;
  LOW   HIGH  HIGH  HIGH  LOW   LOW   HIGH  --> /SRAM.CS=LOW   ;/
  x     x     LOW   x     LOW   HIGH  HIGH  --> /ROM.CS0=LOW   ;\
  x     x     HIGH  x     LOW   HIGH  HIGH  --> /ROM.CS1=LOW   ; HiROM
  HIGH  HIGH  LOW   LOW   HIGH  HIGH  HIGH  --> /AUX.CS=LOW    ;
  HIGH  HIGH  HIGH  LOW   HIGH  HIGH  HIGH  --> /SRAM.CS=LOW   ;/
```

The four "*" marked lines are different as MAD-1.

**Other Battery Controllers**

Battery Controllers are used to write-protect the SRAM during
power-up/power-down, and to switch from Vbat to VCC supply when available.
Early carts used a transistor/diode circuit, later carts are using
MAD-1/MAD-2/MAD-R or MM1026/MM1134 chips.

**MM1026/MM1134 Pinouts**

```text
  1 GND
  2 /RESET (output)
  3 CS (to SRAM) (usually NC when using /CS)
  4 Vbat (from battery)
  5 /CS (to SRAM)
  6 Vout (to SRAM)
  7 NC (MM1026) or /Y (MM1134) (aka /CS input)
  8 VCC (from snes)
```

On MM1026, /CS is simply inverse of CS (both true when VCC=good). On MM1134,
the additional /Y input allows to force /CS false (but doesn't affect CS).

On SA-1 "SNSP-1L3B-20" boards, the PCB text layer says MM1026AF, but the actual
chip is labelled "6129A, 6C33"; which refers to a "BA6129A" chip.

**Batteries**

```text
  CR2032 (3 volt lithium cells, 20mm diameter, 3.2mm width) <-- most SNES carts
  CR2430 (3 volt lithium cells, 24.5mm diameter, 3mm width) <-- X-Band Modem
  NiCd (3.6V, rechargeable, but acid-leaking) <-- many Copiers
```

**MAD Versions/Revisions &amp; Power Down Notes**

```text
  BU2230           MAD-1
  XLU2230          MAD-1
  BU2230A          MAD-1A
  BU2231A          MAD-2A
  BU2220           MAD-R
  TexasInstruments MAD-1
```

At VCC&gt;Vbat, the chips do operate normally (as shown in above Logic Tables).

At VCC=Vbat, the BU2230,BU2230A,XLU2230 (MAD1/MAD1A) force /SRAM.CS=high,
whilst TexasInstrument (MAD1) and BU2220 (MAD-R) keep /SRAM.CS operating as
normally.

At VCC=0, the BU2230,BU2230A,XLU2230 (MAD1/MAD1A) chips and BU2220 (MAD-R)
switch the other 4 outputs to LOW, whilst TexasInstrument (MAD1) switch them
HIGH (or maybe they are left floating and get pulled high by the test circuit).

<a id="snespinoutsrtcchips"></a>

## SNES Pinouts RTC Chips

**Sharp S-RTC Pin-Outs (used by Dai Kaiju Monogatari 2)**

```text
  1-24 Unknown (should have an address decoder and 4bit data bus or so)
```

24pin chip. Still unknown which &amp; how many address/data lines are
connected, and if there are "specials" like /IRQs (?)

**Epson/Seiko RTC-4513 Pin-Outs (for Far East of Eden Zero) (via SPC7110 chip)**

```text
  1 NC
  2 DATA
  3 STD.P
  4 NC
  5 NC
  6 VCC
  7 NC
  8 NC
  9 GND
  10 NC
  11 NC
  12 CE
  13 CLK
  14 NC
```

**Seiko/Epson S-3520CF Pin-Outs (used in SFC-Box and NSS)**

```text
  1 Xin
  2 NC
  3 Xout
  4 /CLK
  5 DataIn
  6 /WR
  7 GND
  8 /TPOUT
  9 DataOut
  10 PDW
  11 /CS
  12 Capacitor
  13 NC
  14 VCC
```

Crystal = 32.768kHz (see datasheet page 13)

<a id="snespinoutsmiscchips"></a>

## SNES Pinouts Misc Chips

**S-ENC Pin-Outs**

```text
  1 (R-Y)O     5 VCC    9 YI        13 VC (clk)   17 PHA (NC)    21 AG (Green)
  2 GND        6 CO     10 (B-Y)I   14 VB (clk)   18 PDO (NC)    22 AB (Blue)
  3 PCP,/BURST 7 VO     11 (R-Y)I   15 VA (NC)    19 NTSC/PAL    23 YO
  4 SW (VCC)   8 SYNC   12 BLA      16 BFP,/BURST 20 AR (Red)    24 (B-Y)O
```

Analog RGB to composite converter. Pin19 allows to select PAL or NTSC mode
(also requires the correct PAL or NTSC clock input on Pin13,14).

The "S-RGB A" has reportedly other pinouts:

```text
  1 ?          5 ?      9 ?           13 ?        17 Y (luma)    21 ?
  2 ?          6 ?      10 ?          14 ?        18 ?           22 Green
  3 ?          7 CSYNC  11 ?          15 ?        19 ?           23 ?
  4 ?          8 ?      12 C (chroma) 16 ?        20 Red         24 Blue
```

**S-CLK Pin-Outs (PAL only)**

```text
  1 17.7MHz(X1.A)   4 21.28MHz(MCK)   7 3.072MHz(Cart)  10 GND    13 Low
  2 17.7MHz(X1.B)   5 4.433MHz(PAL)   8 3.072MHz(APU)   11 Low    14 VCC
  3 VCC             6 3.072MHz(CIC)   9 3.072MHz(APU)   12 Low
```

Clock multiplier/divider for PAL consolses (none such in NTSC consoles).

Pin6-9 are two inverters (for APU-generated CIC clock) (NTSC consoles have
equivalent inverters in a 74HCU04 chip).

**NEC uPD6376 (two-channel serial 16bit D/A Converter)**

```text
  1 DSSEL (GND)    5 AGND (NC)     9 RREF           13 LRCK (32000Hz)
  2 DGND (GND)     6 ROUT          10 LREF          14 LRSEL (GND)
  3 NC (VCC)       7 AVDD (VCC)    11 LOUT          15 SI (DATA)
  4 DVDD (VCC)     8 AVDD (VCC)    12 AGND (GND)    16 CLK (1.536MHz)
```

DATA changes on falling CLK and is valid on raising CLK, falling LRCK indicates
that the most recent 16 bits were LEFT data, raising LRCK indicates RIGHT data.
Each 16bit sample is preceeded by 8 dummy bits (which are always zero). The
16bit values are signed without BIAS offset (0000h=silence).

```text
           _   _   _   _   _   _       _   _   _   _   _   _   _
  CLK     | |_| |_| |_| |_| |_| |_   _| |_| |_| |_| |_| |_| |_| |_  Pin 16
          __________:_____________ .. __________________:
  LRCK              :MSB    <--- LEFT SAMPLE -->     LSB|_________  Pin 13
                    :___ ___ ___ _    __ ___ ___ ___ ___:
  DATA    __________/___X___X___X_ .. __X___X___X___X___\_________  Pin 15
                    :b15 b14 b13         b3  b2  b1  b0 :
```

If the MUTE bit is set in FLG.6, then DATA is always 0, and additionally
/MUTE=LOW is output to the Amplifier (so the DSP is "double-muted", and, the
/MUTE signal also mutes external sound inputs like from SGB).

**S-MIX Pin-Outs**

Unknown. This chip is found on some cost-down SNES mainboards. Maybe a sound
amplifier.

**LM324 Quad Amplifier**

```text
  1 LeftPostOut      8 LeftPreOut
  2 LeftPostIn-      9 LeftPreIn-
  3 LeftPostIn+      10 LeftPreIn+
  4 VS (not VCC)     11 GND
  5 RightPostIn+     12 RightPreIn+
  6 RightPostIn-     13 RightPreIn-
  7 RightPostOut     14 RightPreOut
```

The Pre-amplifiers do amplify the signal from the D/A converter. The
pre-amplified signal is then mixed (via resisters and capacitors) with the
other two audio sources (from cartridge and expansion port), the result is then
passed through the Post-amplifier stage, eventually muted via transistors (if
DSP outputs /MUTE=Low), and amplified through further transistors. The final
stereo signal is output to A/V connector, and mono signals are mixed (via
resistors) for Expansion Port and TV Modulator). The LM324 chip and the
transistors are using the VS supply, rather than normal 5V VCC.

<a id="snespinoutsgsuchips"></a>

## SNES Pinouts GSU Chips

**GSU Chip Packages**

```text
     100       81           112       85             111       86
      .----------.         .------------.           .------------.
   1 /O          |80      1| O          |84      112|          O |85
    |   MC1      |         |            |          1|            |
    |   GSU1     |         |   GSU2     |           |  GSU2-SP1  |
    |   GSU1A    |         |            |           |            |
  30|            |51     28|            |57       29|            |56
    '------------'         '------------'           '------------'
     31        50           29        56             30        55
```

GSU2-SP1 is having odd pin numbering (with pin1 being the SECOND pin; which was
apparently done to maintain same pin numbers as for GSU2).

**MC1, GSU1, and GSU1A**

```text
  1 GND
  2 ROM.A18
  3 ROM.A17
  4 ROM.A16
  5 ROM.A15
  6 ROM.A14
  7 ROM.A13
  8 ROM.A12
  9 ROM.A11
 10 ROM.A10
 11 ROM.A9
 12 ROM.A8
 13 ROM.A7
 14 ROM.A6
 15 ROM.A5
 16 ROM.A4
 17 ROM.A3
 18 ROM.A2
 19 ROM.A1
 20 ROM.A0
 21 ROM.D7
 22 ROM.D6
 23 ROM.D5
 24 ROM.D4
 25 ROM.D3
 26 ROM.D2
 27 GND
 28 ROM.D1
 29 ROM.D0
 30 VCC
 --
 31 ?
 32 /WR
 33 /RD
 34 /RESET
 35 D7
 36 D6
 37 D5
 38 D4
 39 D3
 40 GND    ;\swapped on GSU2
 41 VCC    ;/
 42 D2
 43 D1
 44 D0
 45 A22
 46 A21
 47 A20
 48 A19
 49 A18
 50 A17
 --
 51 A16
 52 A15
 53 A14
 54 A13
 55 A12
 56 /IRQ
 57 A0
 58 A1
 59 A2
 60 A3
 61 A4
 62 A5
 63 A6
 64 A7
 65 A8
 66 A9
 67 A10
 68 A11
 69 GND
 70 X1 (21.44MHz ?)
 71 SRAM.D0
 72 SRAM.D1
 73 SRAM.D2
 74 SRAM.D3
 75 SRAM.D4
 76 SRAM.D5
 77 SRAM.D6
 78 SRAM.D7
 79 SRAM.A0
 80 SRAM.A1
 --
 81 SRAM.A2
 82 SRAM.A3
 83 SRAM.A4
 84 SRAM.A5
 85 SRAM.A6
 86 SRAM.A7
 87 SRAM.A8
 88 SRAM.A9
 89 VCC
 90 GND
 91 SRAM.A10
 92 SRAM.A11
 93 SRAM.A12
 94 SRAM.A13
 95 SRAM.A14
 96 GND
 97 SRAM.A15
 98 SRAM./OE
 99 SRAM./WE
 100 ROM.A19
```

**GSU2, and GSU2-SP1**

```text
  1 ROM.A17
  2 ROM.A16
  3 ROM.A15
  4 ROM.A14
  5 ROM.A13
  6 ROM.A12
  7 ROM.A11
  8 ROM.A10
  9 ROM.A9
 10 ROM.A8
 11 ROM.A7
 12 ROM.A6
 13 ROM.A5
 14 VCC
 15 ROM.A4
 16 ROM.A3
 17 ROM.A2
 18 ROM.A1
 19 ROM.A0
 20 ROM./CE
 21 ?       (NC, probably /CE for 2nd ROM chip)
 22 ROM.D7
 23 ROM.D6
 24 ROM.D5
 25 ROM.D4
 26 ROM.D3
 27 ROM.D2
 28 GND?
 --
 29 ROM.D1
 30 ROM.D0
 31 ?
 32 ?
 33 /WR
 34 /RD
 35 /RESET
 36 GND?
 37 D7
 38 D6
 39 D5
 40 D4
 41 D3
 42 VCC
 43 GND
 44 D2
 45 D1
 46 D0
 47 A23
 48 A22
 49 A21
 50 A20
 51 A19
 52 A18
 53 A17
 54 A16
 55 A15
 56 A14
 --
 57 A13
 58 A12
 59 /IRQ
 60 A0
 61 A1
 62 A2
 63 A3
 64 A4
 65 GND?
 66 A5
 67 A6
 68 A7
 69 A8
 70 VCC
 71 A9
 72 A10
 73 A11
 74 GND
 75 X1 (21.44MHz)
 76 VCC
 77 SRAM.D0
 78 SRAM.D1
 79 SRAM.D2
 80 SRAM.D3
 81 SRAM.D4
 82 SRAM.D5
 83 SRAM.D6
 84 SRAM.D7
 --
 85 SRAM.A0
 86 SRAM.A1
 87 SRAM.A2
 88 SRAM.A3
 89 SRAM.A4
 90 SRAM.A5
 91 SRAM.A6
 92 SRAM.A7
 93 SRAM.A8
 94 SRAM.A9
 95 NC?
 96 NC?
 97 VCC
 98 VCC
 99 GND
 100 SRAM.A10
 101 SRAM.A11
 102 SRAM.A12
 103 SRAM.A13
 104 SRAM.A14
 105 NC/SRAM.A15
 106 NC/SRAM.A16
 107 SRAM./OE
 108 SRAM./WE
 109 ROM.A20
 110 ROM.A19
 111 GND
 112 ROM.A18
```

<a id="snespinoutscx4chip"></a>

## SNES Pinouts CX4 Chip

**Capcom CX4 (used in Mega Man X2/X3)**

```text
  1  A3     21 A15    41 RA8    61 /IRQ
  2  A4     22 A14    42 RA7    62 D7
  3  A5     23 A13    43 RA6    63 D6
  4  A6     24 A12    44 RA5    64 D5
  5  A7     25 /SRAM  45 RA4    65 D4
  6  A8     26 /ROM2  46 RA3    66 Vcc
  7  A9     27 /ROM1  47 RA2    67 D3
  8  A10    28 RA19   48 RA1    68 D2
  9  A11    29 RA18   49 RA0    69 D1
  10 GND    30 RA17   50 GND    70 D0
  11 XIN    31 Vcc    51 /RWE   71 Vcc
  12 XOUT   32 RA16   52 /ROE   72 /RST
  13 A23    33 RA15   53 RD7    73 GND
  14 A22    34 RA20   54 RD6    74 GNDed
  15 A21    35 RA14   55 RD5    75 GNDed
  16 A20    36 RA13   56 RD4    76 /RD
  17 A19    37 RA12   57 RD3    77 /WR
  18 A18    38 RA11   58 RD2    78 A0
  19 A17    39 RA10   59 RD1    79 A1
  20 A16    40 RA9    60 RD0    80 A2
```

SNES bus (cartridge slot) connects to Pin 1-24 and 61-80, CX4 bus (ROM/SRAM) to
pin 25-60. Pin 74 and 75 are GNDed (but not interconnected to GND inside of the
chip); of these, Pin 75 can be reconfigured on some PCBs (via CL and R4
options); maybe one of the pins is for HiROM mapping.

<a id="snespinoutssa1chip"></a>

## SNES Pinouts SA1 Chip

**SA-1**

```text
  1 SNES./IRQ
  2 SNES.D7
  3 SNES.D3
  4 SNES.D6
  5 SNES.D2
  6 SNES.D5
  7 SNES.D1
  8 SNES.D4
  9 SNES.D0
  10 VCC
  11 GND
  12 SNES.A23
  13 SNES.A0
  14 SNES.A22
  15 SNES.A1
  16 SNES.A21
  17 SNES.A2
  18 SNES.A20
  19 SNES.A3
  20 SNES.A19
  21 SNES.A4
  22 SNES.A18
  23 SNES.A5
  24 SNES.A17
  25 SNES.A6
  26 SNES.A16
  27 SNES.A7
  28 SNES.A15
  29 SNES.A8
  30 SNES.A14
  31 SNES.A9
  32 SNES.A13
  33 SNES.A10
  34 SNES.A12
  35 SNES.A11
  36 VCC
  37 GND
  38 REFRESH
 ---
  39 GND
  40 X.?     MasterClock (21.477MHz)
  41 X.?     MasterClock (21.477MHz)
  42 GND
  43 ROM.D15 pin31 (D15/A0)
  44 ROM.D7  pin30
  45 ROM.D14 pin29
  46 ROM.D6  pin28
  47 ROM.D11 pin22
  48 ROM.D3  pin21
  49 ROM.D10 pin20
  50 ROM.D2  pin19
  51 ROM.D13 pin27
  52 ROM.D5  pin26
  53 ROM.D12 pin25
  54 ROM.D4  pin24
  55 ROM.D9  pin18
  56 ROM.D1  pin17
  57 ROM.D8  pin16
  58 ROM.D0  pin15
  59 ROM.A1  pin11
  60 ROM.A2  pin10
  61 ROM.A3  pin9
  62 ROM.A4  pin8
  63 ROM.A5  pin7
  64 ROM.A6  pin6
 ---
  65 ROM.A7  pin5
  66 ROM.A8  pin4
  67 ROM.A9  pin42
  68 ROM.A10 pin41
  69 ROM.A11 pin40
  70 ROM.A12 pin39
  71 ROM.A13 pin38
  72 ROM.A14 pin37
  73 ROM.A15 pin36
  74 ROM.A16 pin35
  75 ROM.A17 pin34
  76 ROM.A19 pin2
  77 ROM.A18 pin3
  78 ROM.A20 pin43
  79 ROM.A21 pin44
  80 ROM.A22 pin1
  81           maybe A23 ?
  82 GND?
  83 VCC
  84 GND
  85 GND?
  86 SRAM. A16?  pin1-1 (extra pin)
  87 SRAM. A14   pin1
  88 SRAM. A12   pin2
  89 SRAM.A7  pin3
  90 SRAM.A6  pin4
  91 SRAM.A5  pin5
  92 SRAM.A4  pin6
  93 SRAM.A3  pin7
  94 SRAM.A2  pin8
  95 SRAM.A1  pin9
  96 SRAM.A0  pin10
  97 SRAM. A10   pin21
  98 SRAM. A11   pin23
  99 SRAM. A9    pin24
  100 GND
  101 VCC
  102 SRAM. A8   pin25
 ---
  103 to left-solder pads (U4.3.CS) (aka SRAM.A13)
  104 SRAM. A18?? pin1-2 (extra pin)
  105 SRAM. A15  pin28+1 (extra pin)
  106
  107
  108 SRAM. /OE  pin22
  109 SRAM. /WE  pin27
  110 SRAM.D0 pin11
  111 SRAM.D1 pin12
  112 SRAM.D2 pin13
  113 SRAM.D3 pin15
  114 SRAM.D4 pin16
  115 SRAM.D5 pin17
  116 SRAM.D6 pin18
  117 SRAM.D7 pin19
  118 GND
  119 VCC
  120 SNES./RESET
  121 SNES.SYSCK
  122 SNES.CIC3 (3.072MHz)
  123 SNES.CIC2
  124 SNES.CIC1
  125 SNES.CIC0
  126 SNES./WR
  127 PAL/NTSC (GND=NTSC, VCC=PAL) (for CIC mode and/or HV-timer?)
  128 SNES./RD
```

ROM-Chip Note: ROM./CE and ROM./OE are wired to GND (always enabled).

ROM-Chip Note: ROM.BHE is wired to VCC (always 16bit databus mode).

```text
  U4.Pin5./CS ---> SRAM./CS   pin20  (U4:6129A aka PCB:MM1026AF)
  U4.Pin3.CS  ---> SRAM.A13?  pin26
  (left 4 solder-pads near U4 --> SRAM.pin26 = CS or A14)
  (right 4 solder-pads near U4 --> SRAM.pin28 = CS or Vbat)
```

Cart Slot Unused: /ROMSEL

Cart Slot Used: SHIELD (!)

<a id="snespinoutsdecompressionchips"></a>

## SNES Pinouts Decompression Chips

**SPC7110F0A Pin-Outs**

```text
  1 SnsA8   16 SnsA15   31 DatD7   46 GND     61 DatA9  76 SramCE2  91 GND
  2 GND     17 SnsA16   32 DatD6   47 Prg/CE  62 DatA8  77 RtcData  92 SnsD3
  3 SnsA7   18 SnsA17   33 DatD5   48 Dat/CE  63 VCC    78 RtcClk   93 SnsD2
  4 SnsA6   19 SnsA18   34 DatD4   49 DatA18  64 GND    79 Rtc/CE   94 SnsD1
  5 SnsA5   20 SnsS19   35 GND     50 DatA17  65 DatA7  80 VCC      95 SnsD0
  6 SnsA4   21 SnsA20   36 DatD3   51 VCC     66 DatA6  81 VCC      96 GND
  7 SnsA3   22 SnsA21   37 DatD2   52 GND     67 DatA5  82 GND      97 VCC
  8 SnsA2   23 SnsA22   38 DatD1   53 DatA16  68 DatA4  83 GND      98 SnsA11
  9 SnsA1   24 SnsA23   39 DatD0   54 DatA15  69 GND    84 GND      99 SnsA10
  10 SnsA0  25 Sns/RD   40 VCC     55 DatA14  70 DatA3  85 VCC      100 SnsA9
  11 GND    26 Sns/WR   41 GND     56 DatA13  71 DatA2  86 GND
  12 VCC    27 SnsRESET 42 DatA22  57 DatA12  72 DatA1  87 SnsD7
  13 SnsA12 28 Sns21MHz 43 DatA21  58 GND     73 DatA0  88 SnsD6
  14 SnsA13 29 Sns21MHz 44 DatA20  59 DatA11  74 VCC    89 SnsD5
  15 SnsA14 30 GND      45 DatA19  60 DatA10  75 GND    90 SnsD4
```

Sns=SNES Cart-Edge, Prg=Program ROM (/CE), Dat=Data ROM, Rtc=RTC, Sram=SRAM.

**S-DD1 Pin-Outs**

```text
  Pin 1..100 = unknown
  Pin 82 is possibly CIC mode (PAL/NTSC mode)
```

<a id="snespinoutsbsxconnectors"></a>

## SNES Pinouts BSX Connectors

**FLASH Card Slot (as found on a "BSC-1A5M-01" board)**

There are two conflicting numbering schemes for the 62pin connector.

Pin-numbering on the black plastic connector:

```text
  Rear/Left  --> 62 ............................... 32 <-- Rear/Right
  Front/Left --> 31 ............................... 1  <-- Front/Right
```

Pin-numbering on SNES cartridge PCB text layer:

```text
  Rear/Left  --> 62 ............................... 2  <-- Rear/Right
  Front/Left --> 61 ............................... 1  <-- Front/Right
```

Below is using the PCB text layer's numbering scheme:

```text
  1 GND
  2 GND
  3 D0
  4 D4 (with cap to gnd)
  5 D1 (with cap to gnd)
  6 D5
  7 D2
  8 D6
  9 D3
  10 D7
  11 A12
  12 -
  13 A7
  14 via R2 to /RD (33 ohm)
  15 A6
  16 via R3 to /WR (33 ohm)
  17 A5
  18 VCC
  19 A4
  20 -
  21 A3
  22 via R4 to VCC (47kOhm)
  23 A2
  24 via R5 to GND (47kOhm)
  25 A1
  26 via R6 to GND (47kOhm)
  27 A0
  28 -
  29 A14
  30 VCC
  31 VCC
  32 VCC
  33 via R7 to VCC (47kOhm)
  34 3/5 (GNDed=5V)
  35 A13
  36 REFRESH    to SNES.pin.33
  37 A8
  38 A15 rom     SNES.A16 SNES.pin.41
  39 A9
  40 A16 rom     SNES.A17 SNES.pin.42
  41 A11
  42 A17 rom     SNES.A18 SNES.pin.43
  43 A10
  44 A18 rom     SNES.A19 SNES.pin.44
  45 SYSCK       SNES.pin57 (and via R1 to SNES.pin.2 EXPAND) (100 ohm)
  46 A19 rom     SNES.A20 SNES.pin.45
  47 /RESET
  48 A20 rom     SNES.A21 SNES.pin.46
  49 -
  50 A21 rom     SNES.A23 SNES.pin.48 (NOT SNES.A22 !!!)
  51 /CS (from MAD-1A.pin1)
  52 GND
  53 Dx
  54 Dx
  55 Dx
  56 Dx     ... pins here are D8-D15 (on PCBs with 16bit databus)
  57 Dx
  58 Dx
  59 Dx
  60 Dx
  61 GND
  62 GND
```

pitch: 38.1mm per 30 pins === 1.27mm per pin

There are some connection variants: The Itoi cartridge with SA-1 is using 16bit
databus (with extra data lines somewhere on pin53-60), pin12 seems to be
connected to something, some of the the pull-up/pull-downs and VCC/GND pins on
pin 14-34 and 52 may be wired differently.

**SNES Cartridge Slot Usage for Satellaview BIOS and Datapack carts**

REFRESH (SNES.pin33) is forwarded to FLASH cart slot (for unknown reason, maybe
it is ACTUALLY used for deselecting FLASH during REFRESH, or maybe it was
INTENDED for unreleased DRAM cartridges).

SYSCK (SNES.pin57) is forwarded to FLASH cart slot (for unknown reason), and is
also forwarded (via 100 ohms) to EXPAND (SNES.pin2) (and from there forwarded
to the BSX Receiver Unit on SNES Expansion port).

**BSX-EXT-Port Pinouts (half-way rev-engineered by byuu)**

```text
  1  = +5V
  2  = +5V
  3  = +5V
  4  = +5V
  5  = GND
  6  = GND
  7  = GND
  8  = GND
  9  = GND
  10 = GND
  11 = U3.pin17 (B2) ;\
  12 = U3.pin18 (B1) ;
  13 = U3.pin15 (B4) ;
  14 = U3.pin16 (B3) ;
  15 = U3.pin13 (B6) ;
  16 = U3.pin14 (B5) ;
  17 = U3.pin11 (B8) ;
  18 = U3.pin12 (B7) ;/
  19 = U2.pin11 (Y8)
  20 = GND
  21 = U2.pin12 (Y7)
  22 = GND
  23 = ???
  24 = GND
  25 = U1.pin12 (Y7)
  26 = GND
  27 = U1.pin11 (Y8)
  28 = U1.pin13 (Y6)
  29 = ???
  30 = ???
  31 = ???
  32 = U2.pin14 (Y5)
  33 = ???
  34 = ???
  35 = GND
  36 = GND
  37 = GND
  38 = GND
```

The U3 transceiver is probably passing databus to/from SNES, the U1/U2 drivers
are maybe passing some address/control signals from SNES. The indirect
connection via U1/U2/U3 may be intended to amplify the bus, or to disconnect
the bus (in case when the receiver power supply isn't connected).

<a id="snespinoutsnssconnectors"></a>

## SNES Pinouts NSS Connectors

**NSS - CN11/12/13 - Cartridge Slots (3 slots, 2x50pin each)**

```text
            Solder side    Component side
                      A    B
  WRAM.64         GND - 1  - VCC2        INST.28                ;\
  WRAM.64         GND - 2  - VCC2        INST.28                ; PROM
  PROM.7-R3  PROM.RES - 3  - PROM.CLK    PROM.6                 ; (and SNES
  PROM.5-R2  PROM.TST - 4  - PROM.CNT    PROM.8                 ; select)
              /SNES_# - 5  - PROM.DTA    PROM.1                 ;/
  INST.15          D3 - 6  - D4          INST.16                ;\
  INST.13          D2 - 7  - D5          INST.17                ;
  INST.12          D1 - 8  - D6          INST.18                ;
  INST.11          D0 - 9  - D7          INST.19                ;
  INST.10          A0 - 10 - /CE_#       INST.20                ;
  INST.9           A1 - 11 - A10         INST.21                ; INST ROM
  INST.8           A2 - 12 - /OE         INST.22                ;
  INST.7           A3 - 13 - A11         INST.23                ;
  INST.6           A4 - 14 - A9          INST.24                ;
  INST.5           A5 - 15 - A8          INST.25                ;
  INST.4           A6 - 16 - A7          INST.3                 ;
  INST.2          A12 - 17 - GND         WRAM.64                ;
  WRAM.64         GND - 18 - VCC2        INST.28                ;
  WRAM.64 _______ GND - 19 - VCC2 ______ INST.28                ;/
  WRAM.64         GND - 20 - VCC         WRAM.1                 ;\
  WRAM.64         GND - 21 - VCC         WRAM.1                 ;
  WRAM.56       /PARD - 22 - /PAWR       WRAM.58                ;
  WRAM.47         PA6 - 23 - PA7         WRAM.50                ;
  WRAM.45         PA4 - 24 - PA5         WRAM.46                ;
  WRAM.43         PA2 - 25 - PA3         WRAM.44                ; SNES Bus
  WRAM.53         PA0 - 26 - PA1         WRAM.54                ; (and PROM
  WRAM.57         /RD - 27 - /WR         WRAM.59                ; select)
  WRAM.63          D3 - 28 - D4          WRAM.2   ;\D4..D7 in   ;
  WRAM.62          D2 - 29 - D5          WRAM.3   ; opposite    ;
  WRAM.61          D1 - 30 - D6          WRAM.4   ; order as    ;
  WRAM.60          D0 - 31 - D7          WRAM.5   ;/on SNES     ;
  CPU.46         /IRQ - 32 - /ROMSEL     CPU.77                 ;
  CPU.93           A0 - 33 - A23         CPU.17                 ;
  CPU.94           A1 - 34 - A22         CPU.16                 ;
  CPU.95           A2 - 35 - A21         CPU.15                 ;
  CPU.96           A3 - 36 - A20         CPU.14                 ;
  CPU.97           A4 - 37 - A19         CPU.13                 ;
  CPU.98           A5 - 38 - A18         CPU.12                 ;
  CPU.99           A6 - 39 - A17         CPU.11                 ;
  CPU.100          A7 - 40 - A16         CPU.10                 ;
  CPU.2            A8 - 41 - A15         CPU.9                  ;
  CPU.3            A9 - 42 - A14         CPU.8                  ;
  CPU.4           A10 - 43 - A13         CPU.7                  ;
  CPU.5           A11 - 44 - A12         CPU.6                  ;
  WRAM.7      REFRESH - 45 - /WRAMSEL    WRAM.15                ;
              AUDIO_L - 46 - AUDIO_R                            ;
  PROM.2   PROM./CE_# - 47 - SYSCLK      WRAM.6                 ;
  CPU.48      MCK 21M - 48 - /RESET      WRAM.8                 ;
  WRAM.64         GND - 49 - VCC         WRAM.1                 ;
  WRAM.64         GND - 50 - VCC         WRAM.1                 ;/
```

The NSS motherboard uses female Matsushita AXD100271 connectors, and the NSS
cartridges have male Matsushita AXD200251 connectors. Both are obsolete as of a
few years ago, but they're just shrouded 0.1" headers.

**RICOH RP5H01 PROM Pinout (Decryption Key PROM on NSS Cartridges)**

```text
  1 DATA.OUT
  2 /CE (VPP)
  3 VCC
  4 GND
  5 TEST
  6 DATA.CLK
  7 RESET
  8 COUNTER.OUT
```

**NSS - CN1 - Big Edge Connector "JAMMA" - 2x28 pin**

```text
  1  GND (from Power Supply)
  A  GND (from Power Supply)
  2  GND (NC)
  B  GND (from Power Supply)
  3  +5V (from Power Supply)
  C  +5V (to joypads; and NC there)
  4  +5V (from Power Supply)
  D  +5V (from Power Supply)
  5  NC  (NC)
  E  -5V (from Power Supply)
  6  +12V (to Coin Lamps and Coin Counter)
  F  +12V (from Power Supply)
  7       KEY
  H       KEY
  8  Coin Counter 1
  J  Coin Counter 2
  9  NC
  K  NC
  10 SPEAKER (Right)
  L  SPEAKER (Left)
  11 AUDIO (+) (NC)
  M  AUDIO GND
  12 VIDEO RED
  N  VIDEO GREEN
  13 VIDEO BLUE
  P  VIDEO SYNC
  14 VIDEO GND
  R  SERVICE SW
  15 TEST SW
  S  NC
  16 COIN SW 1
  T  COIN SW 2
  17 1P START
  U  2P START
  18 1P UP
  V  2P UP
  19 1P DOWN
  W  2P DOWN
  20 1P LEFT
     2P LEFT
  21 1P RIGHT
     2P RIGHT
  22 1P A
     2P A
  23 1P B
     2P B
  24 1P SELECT
     2P SELECT
  25 VOLUME ?   (POT Center Pin)
     VOLUME ?   (POT Outer Pin)
  26 VOLUME GND (POT Outer Pin)
     NC
  27 GND
     GND
  28 GND
     GND
```

**NSS - CN2 - 10P Connector (Extra Joypad Buttons)**

```text
  1  GND
  2  2P TR
  3  2P TL
  4  2P Y
  5  2P X
  6  1P TR
  7  1P TL
  8  1P Y
  9  1P X
  10 GND
```

**NSS - CN3 - 13P Connector (Front Panel LEDs/Buttons)**

```text
  1  GND (for Buttons)
  2  Button Restart
  3  Button Page Down
  4  Button Page Up
  5  Button Instructions
  6  Button Game #3
  7  Button Game #2
  8  Button Game #1
  9  LED Instructions
  10 LED Game #3
  11 LED Game #2
  12 LED Game #1
  13 +5V or so (for LEDs)
```

**NSS - CN4**

```text
  1 GND      (to SNES Controller pin 7)
  2 /EXT_CTRL2 (Low=External CN4 controller, High=Internal Joypad2 selected)
  3 JPIO7    (to SNES Controller pin 6)  ;\
  4 JPSTR    (to SNES Controller pin 3)  ; always connected
  5 JPCLK2   (to SNES Controller pin 2)  ;/
  6 4017.D1  (to SNES Controller pin 5)  ;\only when CN4 selected
  7 4017.D0  (to SNES Controller pin 4)  ;/
  8 SNES +5V (to SNES Controller pin 1)
```

The external input is enabled when setting INST ROM Flags Bit0=0 (that bit is
copied to Port 01h.W bit4).

**NSS - CN5**

```text
  1 GND
  2 IC32/74LS540 pin 9 (Port 02h.R bit 7)
  3 IC32/74LS540 pin 8 (Port 02h.R bit 6)
  4 IC32/74LS540 pin 7 (Port 02h.R bit 5)
  5 IC32/74LS540 pin 6 (Port 02h.R bit 4)
  6 IC32/74LS540 pin 5 (Port 02h.R bit 3)
  7 +5V
```

**NSS Repair (Blank Screen / Washed out colors)**

There seems to be a fairly common hardware problem that causes the NSS to show
a picture with washed out colors or a completely blank screen; in some cases
the problem appears or disappears when the unit has warmed up.

The problem is related to the power supply of the IR3P32A chip: The supply
should be around 9V, and video glitches appear when it drops below 8V. For
deriving the "9V", Nintendo has strapped the IR3P32A to the 12V line via a 100
ohm resistor; which is rather crude and unreliable.

As workaround one could add a second resistor in parallel with the 100 ohms
(which is equally crude, though it should help temporarily), a more reliable
solution should be to replace the 100 ohms by a 7809 voltage regulator (and
eventually some capacitors as far as needed).

The actual reason for the problem is unknown - apparently some odd aging effect
on the IR3P32A chip and/or other components connected to it. No info if the
problem occurs both with original monitor and power supply as well as with
third-party hardware.

**NSS-to-SNES-cartridge adaptor (signal quality)**

Using SNES cartridges with coprocessors (eg. DSP1 carts) on NSS requires some
fine tuning:

DogP's older solution: The /RD and /WR pins seem to have high slew rates and
overshoot badly (by around 3V, for just a few ns)... a regular Mario Kart
cartridge works perfectly with LPFs added to those pins. The PowerPak seems to
still have some issues though.

DogP's newer solution: I actually ended up just adding small resistors in
series with the data bus, which helped reduce the overshoot/ringing. This also
fixed the PowerPak issues.

**NSS-to-SNES-cartridge adaptor (CIC)**

A fully functional NSS-to-SNES-cartridge adaptor would also require a CIC chip
(as a few SNES cartridges with special protections won't work if the 'console'
doesn't output the correct CIC signals).

Accordingly, the adaptor would also need something that generates the 3.072MHz
CIC clock signal (on a real SNES that would be 24.576MHz/8 coming from APU) (on
the NSS adaptor it would require a separate oscillator, or if accuracy doesn't
matter, then one might get away with 21.xxxMHz PAL/NTSC master clock divided by
7 (or dirtier: divided by 8)).

Unless there should be another way to get those protected cartridges to work
(maybe by simply wiring CIC clock to VCC or GND, or by feeding it only a few
dozen of CIC clks after reset, so it could initialize itself, but would never
reach the point where the protection could do something harmful).

<a id="snespinoutsnintendopowerflashcarts"></a>

## SNES Pinouts Nintendo Power Flashcarts

**MX15001TFC**

```text
  1 GND         21 SNES_A6     41 VCC         61 FLASH_A13   81 SNES_D7
  2 VCC         22 SNES_A5     42 SRAM_A11    62 FLASH_A14   82 SNES_D6
  3 SNES_A23    23 SNES_A4     43 SRAM_A12    63 FLASH_A15   83 SNES_D5
  4 SNES_A22    24 SNES_A3     44 SRAM_A13    64 FLASH_A16   84 SNES_D4
  5 SNES_A21    25 SNES_A2     45 SRAM_A14    65 FLASH_A17   85 SNES_D3
  6 SNES_A20    26 SNES_A1     46 MEM_A0      66 GND         86 SNES_D2
  7 SNES_A19    27 SNES_A0     47 MEM_A1      67 FLASH_A18   87 SNES_D1
  8 SNES_A18    28 GND         48 MEM_A2      68 FLASH_A19   88 SNES_D0
  9 SNES_A17    29 VCC         49 MEM_A3      69 FLASH_A20   89 FLASH_OE
  10 SNES_A16   30 SNES_21MHZ  50 MEM_A4      70 GND         90 GND
  11 SNES_A15   31 SNES_21MHZ  51 MEM_A5      71 FLASH_CS3   91 VCC
  12 SNES_A14   32 FLASH_WP    52 VCC         72 NC          92 GND
  13 SNES_A13   33 GND         53 GND         73 NC          93 CIC_ERROR
  14 SNES_A12   34 GND         54 MEM_A6      74 FLASH_CS2   94 GND
  15 GND        35 VCC_GOOD    55 MEM_A7      75 FLASH_CS1   95 GND
  16 SNES_A11   36 SRAM_CS     56 MEM_A8      76 FLASH_WE1   96 SNES_RESET1
  17 SNES_A10   37 VCC         57 MEM_A9      77 FLASH_WE2   97 MODESEL2?
  18 SNES_A9    38 MODESEL1?   58 MEM_A10     78 MEM_WE3     98 SNES_WR
  19 SNES_A8    39 SRAM_OE     59 FLASH_A11   79 VCC         99 SNES_RD
  20 SNES_A7    40 GND         60 FLASH_A12   80 GND         100 SNES_RESET2
```

The "MEM_xxx" signals are wired to both FLASH and SRAM.

The CIC_ERROR pin should be held LOW. For PAL (with CIC disabled in the
console), the cart does somewhat work when rebooting several times, but it
works more reliable when cutting CIC_ERROR (and preferably GNDing pin93, though
pin93 seems to be floating/low anyways).

<a id="snescommonmods"></a>

## SNES Common Mods

**CIC Disable**

The console contains a F411 (NTSC) or F413 (PAL) chip that verifies if the
cartridge contains an identical chip, if it doesn't, then it resets the SNES,
preventing to use unlicensed carts, or to use NTSC carts on PAL consoles.

```text
  F411/F413 Pin 4 (GND=Disable/Unlock, VCC=Enable/Lock)
```

Even when disabled, some newer games (eg. Donkey Kong Country) may verify the
PAL/NTSC framerate by software and refuse to run if it doesn't match the
expected setting, this can be solved by adding a framerate switch (see below),
the verification is often done only after power-up, so one can restore the
desired setting after power-up.

Some newer games are reportedly also refusing to run if the CIC chip in the
console is disabled, as a workaround, one would usually add a switch that
allows to re-enables the CIC when needed. Eventually one could also modify the
cartridges (they are probably connecting the CIC /RESET output to ROM CE2 pin
or so?).

Games with SA-1 or S-DD1 chips won't work.

**50Hz/60Hz Switch**

```text
  PPU1 Pin 24 (GND=60Hz, VCC=50Hz)
  PPU2 Pin 30 (GND=60Hz, VCC=50Hz)
```

**50Hz/60Hz Switch on newer cost-down SNES (those with 160pin S-CPUN A)**

Basically, the frame rate is selected by a single pin:

```text
  S-CPUN A, Pin 111 - PAL/NTSC  (high=PAL, low=NTSC)
```

An unwanted side effect is that this pin also changes the expected clock input:

```text
  X1 oscillator (21.47727MHz=NTSC, 17.7344750MHz=PAL)
```

as a workaround, buy the missing oscillator, and use a "stereo" switch that
simultaneously toggles the oscillator and the PAL/NTSC pin.

Another unwanted side effect is that it does (probably) change the color clock
output for the S-RGB A chip, making the composite video signal unusable. As a
workaround, one could use a TV set with RGB input (this would also require to
connect the R,G,B,SYNC pins, which are left unconnected on the Multi-Out port
of the cost-down SNES). Eventually it might be also possible to use composite
video by connecting a matching oscillator directly to the S-RGB A chip
(NTSC:3.579545MHz, PAL:4.43361875MHz) (not tested).

<a id="snescontrollermods"></a>

## SNES Controller Mods

**Shift Registers**

SNES Joypads are basically consisting of buttons wired to a 16bit shift
register. This could be reproduced using two 4021 chips (two 8bit parallel-in
serial-out shift registers).

**SNES PAL vs NTSC Controllers**

For using SNES NTSC controllers on SNES PAL consoles:

[SNES Controllers Pinouts](#snes-controllers-pinouts)

**SNESPAD (SNES Controller to PC Parallel Port)**

This is a circuit for connecting up to five SNES joypads to a PC Parallel Port,
using 25pin DSUB or 36pin Centronics connector. The circuit can be used with
drivers like "Direct Pad Pro" or "PPJoy", or by emulators with built-in SNESPAD
support.

```text
  Pin DB25  CNTR
  d3  5     5 ---|>|--.            .---.
  d4  6     6 ---|>|--+------------| O | 1 vcc
  d5  7     7 ---|>|--|  .---------| O | 2 clk
  d6  8     8 ---|>|--|  | .-------| O | 3 stb
  d7  9     9 ---|>|--'  | | .-----|_O_| 4 dta1
  d0  2     2 -----------' | |     | O | 5 dta3
  d1  3     3 -------------' |     | O | 6 io
  x   x     x ---------------' .---| O | 7 gnd
  gnd 18-25 19-30 -------------'    \_/
```

For Pad 1..5, wire Pin "x" to ack,pe,slct,err,busy (aka DB25 pin 10, 12, 13,
15, 11) (aka CNTR pin 10, 12, 13, 32, 11) (aka bit6, bit5, bit4, bit3,
NOT(bit7) in the PC's I/O Port). The circuit is pretty well standarized (there
is only one variant, a so-called "Linux" circuit with messed-up pin ordering:
ack,busy,pe,slct,err for pad 1..5).

**7pin Connectors (1mm pin diameter)**

With some efforts, these can be made pulling contacts from regular DSUB
connectors (which have same pin diameter, but different pin spacing). Solder
the contacts onto a piece of board, and eventually build some plastic block
with holes/notches as in real SNES connectors. Alternately, SNES extension
cables (with one male &amp; one female connector) are reportedly available.

<a id="snesxboouploadwramboot"></a>

## SNES Xboo Upload (WRAM Boot)

**WRAM-Boot Circuit (for WRAM-boot-compatible ROMs, max 128K bytes)**

```text
                   ____
  CTR.01./STB-----|AND \_____WRAM.58./PAWR    VCC-------/cut/--CPU.81.RDY
  EXT.09./PAWR----|____/                      PA7-------/cut/--WRAM.50.PA7
  CTR.36./SELECT--|XOR \_____HIGHSEL          /PAWR-----/cut/--WRAM.58./PAWR
  EXT.20.VCC -----|____/                      /WRAMSEL--/cut/--WRAM.15./WRAMSEL
  HIGHSEL---------|OR  \_____WRAM.50.PA7      /ROMSEL---/cut/--SLT.49./ROMSEL
  EXT.08.PA7 -----|____/                      CTR.01./STB-----[10K]--EXT.20.VCC
  HIGHSEL---------|OR  \___ ____              CTR.14./AUTOLF--[10K]--EXT.20.VCC
  SLT.32./WRAMSEL-|____/   |AND \___WRAM.15.  CTR.36./SELECT--[10K]--EXT.20.VCC
  CPU.77./ROMSEL--|OR  \___|____/   /WRAMSEL  CTR.01./STB-----|10n|--EXT.23.GND
  CTR.14./AUTOLF--|____/  ________*           CTR.14./AUTOLF--|10n|--EXT.23.GND
  CTR.14./AUTOLF--|XOR \_|_ ____              CTR.36./SELECT--|10n|--EXT.23.GND
  EXT.20.VCC------|____/   |OR  \___SLT.49.   CTR.31.INIT----------RESET_BUTTON
  CPU.77./ROMSEL___________|____/   /ROMSEL   CTR.36./SELECT---------CPU.81.RDY
  CTR.19-30.GND__________________EXT.23.GND   CTR.02-09.D0-D7---EXT.11-18.D0-D7
```

**Extended Circuit (for larger ROMs, and for non-WRAM-boot-compatible ROMs)**

```text
                   ____
  CTR.14./AUTOLF--|AND \____(XOR)          /STB------/cut/-----------(AND)
  CTR.01./STB-----|____/                   /AUTOLF---/cut/-----------(XOR)
  CTR.01./STB-----|OR  \____(AND)          /PARD-----/cut/---WRAM.56./PARD
  CTR.36./SELECT--|____/                   ________________
  CPU.77./ROMSEL--|OR  \__________________|/CS   SRAM   CS2|__VCC (if any)
  CTR.01./STB-----|____/        A0..A14___|A0..A14   D0..D7|__D0..D7
  SLT.nn.A15------|XOR \__________________|A15          /OE|__CPU.92./RD
  SLT.nn.A(hi+1)--|____/     A16..A(hi)___|A16..A(hi)   /WE|__CPU.91./WR
  CTR.14./AUTOLF--|OR  \___ ____          |________________|
  CTR.36./SELECT--|____/   |AND \___WRAM.56./PARD
  EXT.10./PARD_____________|____/
  CTR.10./ACK_______________________CPU.39.OUT2
```

**Revision Nov 2012 (/RD disable, for /ROMSEL-less carts like GSU)**

```text
                 ____
  * ------------|OR  \_____ SLT.23.     CPU.92./RD----/cut/---SLT.23./RD
  CPU.92./RD ---|____/      /RD
```

**Functional Description (WRAM Boot)**

RESET/INIT and RDY/SELECT are used to reset and stop the CPU, in that state,
PA0-PA7 and /WRAMSEL are all LOW (of which, PA7 and /WRAMSEL need to be changed
for DMA), and WRAM address is reset to zero. Data is then DMA transferred to
WRAM via databus and /STB/PAWR. Finally, WRAM is mirrored (in HiROM fashion) to
the ROM-region via /AUTOLF, and entrypoint and program code are fetched from
WRAM.

WRAM-Boot compatible files are identified by ID "XBOO" at FFE0h in ROM Header.
WRAM-boot files should be also bootable from normal ROM (unless written by
less-than-lame people).

The WRAM upload function is found in no$sns "Utility" menu. WRAM boot works
only if a regular cartride is inserted, or if the lockout chip is disabled.
Normal ROM-cartridges can be used if the cable is disconnected, or if the
parallel port is set to /STB=HIGH, /AUTOLF=HIGH, /SELECT=HIGH, /INIT=LOW, and
DATA=HIGH-Z.

**Functional Description (Extended Circuit)**

Data is uploaded (in 127.5K blocks) to WRAM, and is then automatically
relocated (by a 0.5K stub) from WRAM to SRAM by software, /ACK is used to
indicate completion of relocation, after uploading all block(s) the console
gets reset with SRAM mapped to the ROM-region.

The XOR gate maps the bottom halves of the HiROM banks as additional LoROM
banks. About A(hi) and A(hi+1): For example, for 128Kx8 chip (A0..A16): connect
A(hi) to A16, A(hi+1) to A17. For more than 2Mx8 SRAM: connect the A(hi+1)
input to GND; or leave out the XOR gate completely.

/STB and /AUTOLF are used (while /SELECT=HIGH) to select ROM, WRAM, or SRAM
mapping, and as /PARD and /PAWR (while /SELECT=LOW). The /PARD pin allows to
download status information and other data from WRAM.

**Parts List**

```text
  1  74LS08 Quad 2-Input AND gates
  1  74LS32 Quad 2-Input OR gates
  1  74LS86 Quad 2-Input XOR gates
  3  10K Ohm Resistors (required when cable is disconnected)
  3  10nF capacitor (to eliminate dirt on some ports)
  1  36pin centronics socket (plus standard printer cable)
```

**Additional components for Extended Circuit**

```text
  1  74LS32 Quad 2-Input OR gates
  1  nn-pin DIP Nx8 Static RAM (SRAM)
  1  40-pin DIP Socket for SRAM
```

Requires a bi-directional parallel port (very old printer ports aren't
bi-directional, also some hyper-modern ports aren't bi-directional when
configured to "ECP" mode, for that ports: use "EPP" mode).

Currently SRAMs of up to 512Kx8 (32pin) should be available for less than $5,
also 2Mx8 chips (36pin) are manufactured, but I've no idea where to buy them,
best use a 40pin DIP socket for future expansion.

**TEST Mode**

Optionally, one can replace the "XBOO" ID-string in cartridge header by "TEST",
this will cause normal Cartridge ROM to be mapped (instead of mirroring WRAM to
the ROM area). The advantage is that the uploaded program can examine cartridge
memory (eg. for reverse-engineering purposes), the disadvantage is that it
cannot set up it's own IRQ/NMI vectors.

The program should redirect the reset vector from Bank 00h to Bank 7Eh by
executing a JMP 7Exxxxh immediately after reset, and then wait until WRAM is no
longer mapped at 008xxxxh; thereafter, ROM is freely accessible (the switch
from WRAM to ROM mapping occurs circa 100us after RESET).
