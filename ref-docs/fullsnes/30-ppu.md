# Fullsnes — Picture Processing Unit (PPU)

[Index](00-index.md) · [« DMA & HDMA Transfers](20-dma-hdma.md) · [Audio Processing Unit »](40-apu-dsp.md)

**Sections in this file:**

- [SNES Picture Processing Unit (PPU)](#snes-picture-processing-unit-ppu)
- [SNES PPU Control](#snes-ppu-control)
- [SNES PPU BG Control](#snes-ppu-bg-control)
- [SNES PPU Rotation/Scaling](#snes-ppu-rotationscaling)
- [SNES PPU Sprites (OBJs)](#snes-ppu-sprites-objs)
- [SNES PPU Video Memory (VRAM)](#snes-ppu-video-memory-vram)
- [SNES PPU Color Palette Memory (CGRAM) and Direct Colors](#snes-ppu-color-palette-memory-cgram-and-direct-colors)
- [SNES PPU Window](#snes-ppu-window)
- [SNES PPU Color-Math](#snes-ppu-color-math)
- [SNES PPU Timers and Status](#snes-ppu-timers-and-status)
- [SNES PPU Interrupts](#snes-ppu-interrupts)
- [SNES PPU Resolution](#snes-ppu-resolution)
- [SNES PPU Offset-Per-Tile Mode](#snes-ppu-offset-per-tile-mode)

---

<a id="snespictureprocessingunitppu"></a>

## SNES Picture Processing Unit (PPU)

[SNES PPU Control](#snes-ppu-control)

[SNES PPU BG Control](#snes-ppu-bg-control)

[SNES PPU Rotation/Scaling](#snes-ppu-rotationscaling)

[SNES PPU Window](#snes-ppu-window)

[SNES PPU Color-Math](#snes-ppu-color-math)

[SNES PPU Timers and Status](#snes-ppu-timers-and-status)

[SNES PPU Interrupts](#snes-ppu-interrupts)

[SNES PPU Resolution](#snes-ppu-resolution)

[SNES PPU Offset-Per-Tile Mode](#snes-ppu-offset-per-tile-mode)

**Video Memory (OAM/VRAM/CGRAM)**

[SNES PPU Sprites (OBJs)](#snes-ppu-sprites-objs)

[SNES PPU Video Memory (VRAM)](#snes-ppu-video-memory-vram)

[SNES PPU Color Palette Memory (CGRAM) and Direct Colors](#snes-ppu-color-palette-memory-cgram-and-direct-colors)

All video memory can be accessed only during V-Blank, or Forced Blank.

Video memory isn't mapped to the CPU bus, and be accessed only via I/O ports.

[SNES Memory OAM Access (Sprite Attributes)](10-memory-and-io-map.md#snes-memory-oam-access-sprite-attributes)

[SNES Memory VRAM Access (Tile and BG Map)](10-memory-and-io-map.md#snes-memory-vram-access-tile-and-bg-map)

[SNES Memory CGRAM Access (Palette Memory)](10-memory-and-io-map.md#snes-memory-cgram-access-palette-memory)

The above OAM/VRAM/CGRAM I/O ports are usually accessed via DMA,

[SNES DMA Transfers](20-dma-hdma.md#snes-dma-transfers)

**Pinouts**

[SNES Audio/Video Connector Pinouts](80-timings-unpredictable-pinouts.md#snes-audiovideo-connector-pinouts)

[SNES Pinouts PPU Chips](80-timings-unpredictable-pinouts.md#snes-pinouts-ppu-chips)

**Background Priority Chart**

```text
  Mode0    Mode1    Mode2    Mode3    Mode4    Mode5    Mode6    Mode7
  -        BG3.1a   -        -        -        -        -        -
  OBJ.3    OBJ.3    OBJ.3    OBJ.3    OBJ.3    OBJ.3    OBJ.3    OBJ.3
  BG1.1    BG1.1    BG1.1    BG1.1    BG1.1    BG1.1    BG1.1    -
  BG2.1    BG2.1    -        -        -        -        -        -
  OBJ.2    OBJ.2    OBJ.2    OBJ.2    OBJ.2    OBJ.2    OBJ.2    OBJ.2
  BG1.0    BG1.0    BG2.1    BG2.1    BG2.1    BG2.1    -        BG2.1p
  BG2.0    BG2.0    -        -        -        -        -        -
  OBJ.1    OBJ.1    OBJ.1    OBJ.1    OBJ.1    OBJ.1    OBJ.1    OBJ.1
  BG3.1    BG3.1b   BG1.0    BG1.0    BG1.0    BG1.0    BG1.0    BG1
  BG4.1    -        -        -        -        -        -        -
  OBJ.0    OBJ.0    OBJ.0    OBJ.0    OBJ.0    OBJ.0    OBJ.0    OBJ.0
  BG3.0    BG3.0a   BG2.0    BG2.0    BG2.0    BG2.0    -        BG2.0p
  BG4.0    BG3.0b   -        -        -        -        -        -
  Backdrop Backdrop Backdrop Backdrop Backdrop Backdrop Backdrop Backdrop
```

Whereas,

```text
  .N     per-tile priority setting (in BG Map and OAM entries)
  .Np    per-pixel priority setting (for 128-color BG2 in Mode7)
  .Na/b  per-screen priority bit (in port 2105h) (plus .N as usually)
```

<a id="snesppucontrol"></a>

## SNES PPU Control

> **Note (RustySNES ref):** On the open question in the text about *when* forced blank takes effect: the practical, test-ROM-verified rule is that the CPU may freely access VRAM and OAM during Vblank **or** force-blank, and CGRAM during Vblank, Hblank **or** force-blank. Accesses outside those windows are dropped (see the VRAM/CGRAM notes below). See [SNESdev Wiki: Reading and writing PPU memory](https://snes.nesdev.org/wiki/Reading_and_writing_PPU_memory).

**2100h - INIDISP - Display Control 1 (W)**

```text
  7     Forced Blanking (0=Normal, 1=Screen Black)
  6-4   Not used
  3-0   Master Brightness (0=Screen Black, or N=1..15: Brightness*(N+1)/16)
```

In Forced Blank, VRAM, OAM and CGRAM can be freely accessed (otherwise it's
accessible only during Vblank). Even when in forced blank, the TV Set keeps
receiving Vsync/Hsync signals (thus producing a stable black picture). And, the
CPU keeps receiving Hblank/Vblank signals (so any enabled video NMIs, IRQs,
HDMAs are kept generated).

```text
  Forced blank doesn't apply immediately... so one must wait whatever
  (maybe a scanline) before VRAM can be freely accessed... or is it only
  vice-versa: disabling forced blank doesn't apply immediately/shows garbage
  pixels?
```

**212Ch - TM - Main Screen Designation (W)**

**212Dh - TS - Sub Screen Designation (W)**

```text
  7-5  Not used
  4    OBJ (0=Disable, 1=Enable)
  3    BG4 (0=Disable, 1=Enable)
  2    BG3 (0=Disable, 1=Enable)
  1    BG2 (0=Disable, 1=Enable)
  0    BG1 (0=Disable, 1=Enable)
  -    Backdrop (Always enabled)
```

Allows to enable/disable video layers. The Main screen is the "normal" display.
The Sub screen is used only for Color Math and for 512-pixel Hires Mode.

**2133h - SETINI - Display Control 2 (W)**

```text
  7     External Synchronization (0=Normal, 1=Super Impose and etc.)
  6     EXTBG Mode (Screen expand)
          ENABLE THE DATA SUPPLIED FROM THE EXTERNAL LSI.
          FOR THE SFX, ENABLE WHEN THE SCREEN WITH PRIORITY IS USED ON MODE-7.
  5-4   Not used
  3     Horizontal Pseudo 512 Mode (0=Disable, 1=Enable)
          (SHIFT SUBSCREEN HALF DOT TO THE LEFT)
  2     BG V-Direction Display (0=224 Lines, 1=239 Lines) (for NTSC/PAL)
  1     OBJ V-Direction Display (0=Low, 1=High Resolution/Smaller OBJs)
          IN THE INTERLACE MODE, SELECT EITHER OF 1-DOT PER LINE OR 1-DOT
          REPEATED EVERY 2-LINES. IF "1" IS WRITTEN, THE OBJ SEEMS REDUCED
          HALF VERTICALLY IN APPEARANCE.
  0     V-Scanning         (0=Non Interlace, 1=Interlace) (See Port 2105h)
```

<a id="snesppubgcontrol"></a>

## SNES PPU BG Control

**2105h - BGMODE - BG Mode and BG Character Size (W)**

```text
  7    BG4 Tile Size (0=8x8, 1=16x16)  ;\(BgMode0..4: variable 8x8 or 16x16)
  6    BG3 Tile Size (0=8x8, 1=16x16)  ; (BgMode5: 8x8 acts as 16x8)
  5    BG2 Tile Size (0=8x8, 1=16x16)  ; (BgMode6: fixed 16x8?)
  4    BG1 Tile Size (0=8x8, 1=16x16)  ;/(BgMode7: fixed 8x8)
  3    BG3 Priority in Mode 1 (0=Normal, 1=High)
  2-0  BG Screen Mode (0..7 = see below)
```

The BG Screen Modes are:

```text
  Mode   BG1         BG2         BG3         BG4
  0      4-color     4-color     4-color     4-color   ;Normal
  1      16-color    16-color    4-color     -         ;Normal
  2      16-color    16-color    (o.p.t)     -         ;Offset-per-tile
  3      256-color   16-color    -           -         ;Normal
  4      256-color   4-color     (o.p.t)     -         ;Offset-per-tile
  5      16-color    4-color     -           -         ;512-pix-hires
  6      16-color    -           (o.p.t)     -         ;512-pix plus Offs-p-t
  7      256-color   EXTBG       -           -         ;Rotation/Scaling
```

Mode 7 supports rotation/scaling and EXTBG (but doesn't support hv-flip).

Mode 5/6 don't support screen addition/subtraction.

CG Direct Select is support on BG1 of Mode 3/4, and on BG1/BG2? of Mode 7.

**2106h - MOSAIC - Mosaic Size and Mosaic Enable (W)**

Allows to divide the BG layer into NxN pixel blocks, in each block, the
hardware picks the upper-left pixel of each block, and fills the whole block by
the color - thus effectively reducing the screen resolution.

```text
  7-4  Mosaic Size        (0=Smallest/1x1, 0Fh=Largest/16x16)
  3    BG4 Mosaic Enable  (0=Off, 1=On)
  2    BG3 Mosaic Enable  (0=Off, 1=On)
  1    BG2 Mosaic Enable  (0=Off, 1=On)
  0    BG1 Mosaic Enable  (0=Off, 1=On)
```

Horizontally, the first block is always located on the left edge of the TV
screen. Vertically, the first block is located on the top of the TV screen.

When changing the mosaic size mid-frame, the hardware does first finish current
block (using the old vertical size) before applying the new vertical size.
Technically, vertical mosaic is implemented as so: subtract the veritical index
(within the current block) from the vertical scroll register (BGnVOFS).

**2107h - BG1SC - BG1 Screen Base and Screen Size (W)**

**2108h - BG2SC - BG2 Screen Base and Screen Size (W)**

**2109h - BG3SC - BG3 Screen Base and Screen Size (W)**

**210Ah - BG4SC - BG4 Screen Base and Screen Size (W)**

```text
  7-2  SC Base Address in VRAM (in 1K-word steps, aka 2K-byte steps)
  1-0  SC Size (0=One-Screen, 1=V-Mirror, 2=H-Mirror, 3=Four-Screen)
                   (0=32x32, 1=64x32, 2=32x64, 3=64x64 tiles)
               (0: SC0 SC0    1: SC0 SC1  2: SC0 SC0  3: SC0 SC1   )
               (   SC0 SC0       SC0 SC1     SC1 SC1     SC2 SC3   )
```

Specifies the BG Map addresses in VRAM. The "SCn" screens consists of 32x32
tiles each.

Ignored in Mode 7 (Base is always zero, size is always 128x128 tiles).

**210Bh/210Ch - BG12NBA/BG34NBA - BG Character Data Area Designation (W)**

```text
  15-12 BG4 Tile Base Address (in 4K-word steps)
  11-8  BG3 Tile Base Address (in 4K-word steps)
  7-4   BG2 Tile Base Address (in 4K-word steps)
  3-0   BG1 Tile Base Address (in 4K-word steps)
```

Ignored in Mode 7 (Base is always zero).

**210Dh - BG1HOFS - BG1 Horizontal Scroll (X) (W) and M7HOFS**

**210Eh - BG1VOFS - BG1 Vertical Scroll (Y) (W) and M7VOFS**

**210Fh - BG2HOFS - BG2 Horizontal Scroll (X) (W)**

**2110h - BG2VOFS - BG2 Vertical Scroll (Y) (W)**

**2111h - BG3HOFS - BG3 Horizontal Scroll (X) (W)**

**2112h - BG3VOFS - BG3 Vertical Scroll (Y) (W)**

**2113h - BG4HOFS - BG4 Horizontal Scroll (X) (W)**

**2114h - BG4VOFS - BG4 Vertical Scroll (Y) (W)**

```text
  1st Write: Lower 8bit  ;\1st/2nd write mechanism uses "BG_old"
  2nd Write: Upper 2bit  ;/
```

Note: Port 210Dh/210Eh are also used as M7HOFS/M7VOFS, these registers have a
similar purpose, but internally they are separate registers: Writing to 210Dh
does BOTH update M7HOFS (via M7_old mechanism), and also updates BG1HOFS (via
BG_old mechanism). In the same fashion, 210Eh updates both M7VOFS and BG1VOFS.

```text
          BGnHOFS = (Current<<8) | (Prev&~7) | ((Reg>>8)&7);
          Prev = Current;
            or
          BGnVOFS = (Current<<8) | Prev;
          Prev = Current;
```

<a id="snesppurotationscaling"></a>

## SNES PPU Rotation/Scaling

**211Ah - M7SEL - Rotation/Scaling Mode Settings (W)**

```text
  7-6   Screen Over (see below)
  5-2   Not used
  1     Screen V-Flip (0=Normal, 1=Flipped)     ;\flip 256x256 "screen"
  0     Screen H-Flip (0=Normal, 1=Flipped)     ;/
```

Screen Over (when exceeding the 128x128 tile BG Map size):

```text
  0=Wrap within 128x128 tile area
  1=Wrap within 128x128 tile area (same as 0)
  2=Outside 128x128 tile area is Transparent
  3=Outside 128x128 tile area is filled by Tile 00h
```

**211Bh - M7A - Rotation/Scaling Parameter A (and Maths 16bit operand) (W)**

**211Ch - M7B - Rotation/Scaling Parameter B (and Maths 8bit operand) (W)**

**211Dh - M7C - Rotation/Scaling Parameter C (W)**

**211Eh - M7D - Rotation/Scaling Parameter D (W)**

```text
  1st Write: Lower 8bit  ;\1st/2nd write mechanism uses "M7_old"
  2nd Write: Upper 8bit  ;/
```

Signed 16bit values in 1/256 pixel units  (1bit sign, 7bit integer, 8bit
fraction).

**210Dh - M7HOFS/BG1HOFS - BG1 Horizontal Scroll (X) (W)**

**210Eh - M7VOFS/BG1VOFS - BG1 Vertical Scroll (Y) (W)**

**211Fh - M7X - Rotation/Scaling Center Coordinate X (W)**

**2120h - M7Y - Rotation/Scaling Center Coordinate Y (W)**

```text
  1st Write: Lower 8bit  ;\1st/2nd write mechanism uses "M7_old"
  2nd Write: Upper 5bit  ;/
```

Signed 13bit values in pixel units (1bit sign, 12bit integer, 0bit fraction).

**Formula**

Formula for Rotation/Enlargement/Reduction in Matrix Form:

```text
  ( VRAM.X )  =  ( M7A M7B )  *  ( SCREEN.X+M7HOFS-M7X )  +  ( M7X )
  ( VRAM.Y )     ( M7C M7D )     ( SCREEN.Y+M7VOFS-M7Y )     ( M7Y )
```

Parameters:

```text
  M7A=+COS(angle)*ScaleX, M7B=+SIN(angle)*ScaleX
  M7C=-SIN(angle)*ScaleY, M7D=+COS(angle)*ScaleY
  M7X,M7Y       = Center Coordinate
  M7HOFS,M7VOFS = Scroll Offset
  SCREEN.X = Display (Target) X-Coordinate: (0..255) XOR (xflip*FFh)
  SCREEN.Y = Display (Target) Y-Coordinate: (1..224 or 1..239) XOR (yflip*FFh)
  VRAM.X,Y = BG Map (Source) Coordinates (in 1/256 pixel units)
```

To calculate VRAM coordinates for any SCREEN coordinates:

```text
  IF xflip THEN SCREEN.X=((0..255) XOR FFh), ELSE SCREEN.X=(0..255)
  IF yflip THEN SCREEN.Y=((1..224/239) XOR FFh), ELSE SCREEN.Y=(1..224/239)
  ORG.X = (M7HOFS-M7X) AND NOT 1C00h, IF ORG.X<0 THEN ORG.X=ORG.X OR 1C00h
  ORG.Y = (M7VOFS-M7Y) AND NOT 1C00h, IF ORG.Y<0 THEN ORG.Y=ORG.Y OR 1C00h
  VRAM.X = ((M7A*ORG.X) AND NOT 3Fh) + ((M7B*ORG.Y) AND NOT 3Fh) + M7X*100h
  VRAM.Y = ((M7C*ORG.X) AND NOT 3Fh) + ((M7D*ORG.Y) AND NOT 3Fh) + M7Y*100h
  VRAM.X = VRAM.X + ((M7B*SCREEN.Y) AND NOT 3Fh) + (M7A*SCREEN.X)
  VRAM.Y = VRAM.Y + ((M7D*SCREEN.Y) AND NOT 3Fh) + (M7C*SCREEN.X)
```

After calculating the left-most pixel of a scanline, the following pixels on
that scanline can be also calculated by increasing VRAM coordinates as so:

```text
  IF xflip THEN VRAM.X=VRAM.X-M7A, ELSE VRAM.X=VRAM.X+M7A
  IF xflip THEN VRAM.Y=VRAM.Y-M7C, ELSE VRAM.Y=VRAM.Y+M7C
  (The result is same as on hardware, although the real hardware doesn't seem
  to use that method, instead it seems to contain an excessively fast multiply
  unit that recalculates (M7A*SCREEN.X) and (M7C*SCREEN.X) on every pixel.)
```

The VRAM coordinates are then: bit0-7=Fraction, bit8-10=pixel index (within a
tile), bit11-17=map index (within BG map), bit18-and-up=nonzero when exceeding
BG map size (do "screen over" handling).

**M7A/M7B Port Notes**

Port 211Bh/211Ch can be also used for general purpose math multiply:

[SNES Maths Multiply/Divide](40-apu-dsp.md#snes-maths-multiplydivide)

When in BG Mode 7, general purpose multiply works only during V-Blank and
Forced-Blank. During drawing period at SCREEN.Y=0..224/239 (including
SCREEN.Y=0, and during all 340 dots including H-Blank), MPYL/M/H receives two
multiplication results per pixel (one per half-pixel):

```text
  MPY = M7A * ORG.X / 8                                ;at SCREEN.X=-3.0
  MPY = M7D * ORG.Y / 8                                ;at SCREEN.X=-2.5
  MPY = M7B * ORG.Y / 8                                ;at SCREEN.X=-2.0
  MPY = M7C * ORG.X / 8                                ;at SCREEN.X=-1.5
  MPY = M7B * ((SCREEN.Y-MOSAIC.Y) XOR (yflip*FFh))/ 8 ;at SCREEN.X=-1.0
  MPY = M7D * ((SCREEN.Y-MOSAIC.Y) XOR (yflip*FFh))/ 8 ;at SCREEN.X=-0.5
  MPY = M7A * ((SCREEN.X AND FFh) XOR (xflip*FFh)) / 8 ;at SCREEN.X=0.0..336.0
  MPY = M7C * ((SCREEN.X AND FFh) XOR (xflip*FFh)) / 8 ;at SCREEN.X=0.5..336.5
  MPY = M7A * (M7B/100h)                   ;during in V-Blank and Forced-Blank
```

Note: The "/8" suggests that the hardware strips the lower 3bit, however,
before summing up the multiply results, it DOES strip the lower 6bit (hence the
AND NOT 3Fh in the formula).

**M7HOVS/M7VOFS Port Notes**

Port 210Dh/210Eh are also used as BG1HOFS/BG1VOFS, these registers have a
similar purpose, but internally they are separate registers: Writing to 210Dh
does BOTH update M7HOFS (via M7_old mechanism), and also updates BG1HOFS (via
BG_old mechanism). In the same fashion, 210Eh updates both M7VOFS and BG1VOFS.

**M7xx - Write-twice mechanism for Mode 7**

Writing a &lt;new&gt; byte to one of the write-twice M7'registers does:

```text
  M7_reg = new * 100h + M7_old
  M7_old = new
```

M7_old is an internal 8bit register, shared by 210Dh-210Eh and 211Bh-2120h.

**EXTBG**

EXTBG is an "external" BG layer (replacing BG2???) enabled via SETINI.6. On the
SNES, the 8bit external input is simply shortcut with one half of the PPUs
16bit data bus. So, when using EXTBG in BG Mode 0-6, one will just see garbage.
However, in BG Mode 7, it's receiving the same 8bit value as the current BG1
pixel - but, unlike BG1, with bit7 treated as priority bit (and only lower 7bit
used as BG2 pixel color).

<a id="snesppuspritesobjs"></a>

## SNES PPU Sprites (OBJs)

**2101h - OBSEL - Object Size and Object Base (W)**

```text
  7-5   OBJ Size Selection  (0-5, see below) (6-7=Reserved)
         Val Small  Large
         0 = 8x8    16x16    ;Caution:
         1 = 8x8    32x32    ;In 224-lines mode, OBJs with 64-pixel height
         2 = 8x8    64x64    ;may wrap from lower to upper screen border.
         3 = 16x16  32x32    ;In 239-lines mode, the same problem applies
         4 = 16x16  64x64    ;also for OBJs with 32-pixel height.
         5 = 32x32  64x64
         6 = 16x32  32x64 (undocumented)
         7 = 16x32  32x32 (undocumented)
        (Ie. a setting of 0 means Small OBJs=8x8, Large OBJs=16x16 pixels)
        (Whether an OBJ is "small" or "large" is selected by a bit in OAM)
  4-3   Gap between OBJ 0FFh and 100h (0=None) (4K-word steps) (8K-byte steps)
  2-0   Base Address for OBJ Tiles 000h..0FFh  (8K-word steps) (16K-byte steps)
```

**Accessing OAM**

[SNES Memory OAM Access (Sprite Attributes)](10-memory-and-io-map.md#snes-memory-oam-access-sprite-attributes)

**OAM (Object Attribute Memory)**

Contains data for 128 OBJs. OAM Size is 512+32 Bytes. The first part (512
bytes) contains 128 4-byte entries for each OBJ:

```text
  Byte 0 - X-Coordinate (lower 8bit) (upper 1bit at end of OAM)
  Byte 1 - Y-Coordinate (all 8bits)
  Byte 2 - Tile Number  (lower 8bit) (upper 1bit within Attributes)
  Byte 3 - Attributes
```

Attributes:

```text
  Bit7    Y-Flip (0=Normal, 1=Mirror Vertically)
  Bit6    X-Flip (0=Normal, 1=Mirror Horizontally)
  Bit5-4  Priority relative to BG (0=Low..3=High)
  Bit3-1  Palette Number (0-7) (OBJ Palette 4-7 can use Color Math via CGADSUB)
  Bit0    Tile Number (upper 1bit)
```

After above 512 bytes, additional 32 bytes follow, containing 2-bits per OBJ:

```text
  Bit7    OBJ 3 OBJ Size     (0=Small, 1=Large)
  Bit6    OBJ 3 X-Coordinate (upper 1bit)
  Bit5    OBJ 2 OBJ Size     (0=Small, 1=Large)
  Bit4    OBJ 2 X-Coordinate (upper 1bit)
  Bit3    OBJ 1 OBJ Size     (0=Small, 1=Large)
  Bit2    OBJ 1 X-Coordinate (upper 1bit)
  Bit1    OBJ 0 OBJ Size     (0=Small, 1=Large)
  Bit0    OBJ 0 X-Coordinate (upper 1bit)
```

And so on, next 31 bytes with bits for OBJ4..127. Note: The meaning of the OBJ
Size bit (Small/Large) can be defined in OBSEL Register (Port 2101h).

<a id="snesppuvideomemoryvram"></a>

## SNES PPU Video Memory (VRAM)

> **Note (RustySNES ref):** A VRAM write attempted during active display is *silently ignored* — the VRAM address still increments per the `VMAIN` ($2115) setting, but no data is stored. Emulators must reproduce the dropped write plus the address increment, not skip the access entirely. See [SNESdev Wiki: Reading and writing PPU memory](https://snes.nesdev.org/wiki/Reading_and_writing_PPU_memory).

**BG Map (32x32 entries)**

Each BG Map Entry consists of a 16bit value as such:

```text
  Bit 0-9   - Character Number (000h-3FFh)
  Bit 10-12 - Palette Number   (0-7)
  Bit 13    - BG Priority      (0=Lower, 1=Higher)
  Bit 14    - X-Flip           (0=Normal, 1=Mirror horizontally)
  Bit 15    - Y-Flip           (0=Normal, 1=Mirror vertically)
```

In the "Offset-per-Tile" modes (Mode 2,4,6), BG3 entries are different:

```text
  Bit 15    Apply offset to H/V (0=H, 1=V)  ;-Mode 4 only
  Bit 14    Apply offset to BG2             ;\Mode 2 (... and Mode 6, though
  Bit 13    Apply offset to BG1             ;/       Mode 6 has only BG1 ?)
  Bit 12-10 Not used
  Bit 9-0   Scroll offset to be applied to BG1/BG2
  Lower 3bit of HORIZONTAL offsets are ignored.
```

In mode7, BG Maps are 128x128, and only the lower 8bit are used as BG map
entries:

```text
  Bit15-8  Not used (contains tile-data; no relation to the BG-Map entry)
  Bit7-0   Character Number (00h-FFh) (without XYflip or other attributes)
```

**VRAM 8x8 Pixel Tile Data (BG and OBJ)**

Each 8x8 tile occupies 16, 32, or 64 bytes (for 4, 16, or 256 colors). BG tiles
can be 4/16/256 colors (depending on BG Mode), OBJs are always 16 color.

```text
  Color Bits (Planes)     Upper Row ........... Lower Row
  Plane 0 stored in bytes 00h,02h,04h,06h,08h,0Ah,0Ch,0Eh ;\for 4/16/256 colors
  Plane 1 stored in bytes 01h,03h,05h,07h,09h,0Bh,0Dh,0Fh ;/
  Plane 2 stored in bytes 10h,12h,14h,16h,18h,1Ah,1Ch,1Eh ;\for 16/256 colors
  Plane 3 stored in bytes 11h,13h,15h,17h,19h,1Bh,1Dh,1Fh ;/
  Plane 4 stored in bytes 20h,22h,24h,26h,28h,2Ah,2Ch,2Eh ;\
  Plane 5 stored in bytes 21h,23h,25h,27h,29h,2Bh,2Dh,2Fh ; for 256 colors
  Plane 6 stored in bytes 30h,32h,34h,36h,38h,3Ah,3Ch,3Eh ;
  Plane 7 stored in bytes 31h,33h,35h,37h,39h,3Bh,3Dh,3Fh ;/
  In each byte, bit7 is left-most, bit0 is right-most.
  Plane 0 is the LSB of color number.
```

The only exception are Mode 7 BG Tiles, which are stored as 8bit pixels
(without spreading the bits across several bit-planes), and, BG VRAM is divided
into BG Map at even byte addresses, and Tiles at odd addresses, an 8x8 tiles
thus uses the following bytes (64 odd bytes within a 128 byte region):

```text
  Vertical Rows           Left-most .......... Right-Most
  Upper Row      in bytes 01h,03h,05h,07h,09h,0Bh,0Dh,0Fh ;\
  2nd Row        in bytes 11h,13h,15h,17h,19h,1Bh,1Dh,1Fh ;
  3rd Row        in bytes 21h,23h,25h,27h,29h,2Bh,2Dh,2Fh ;
  4th Row        in bytes 31h,33h,35h,37h,39h,3Bh,3Dh,3Fh ; 256-color
  5th Row        in bytes 41h,43h,45h,47h,49h,4Bh,4Dh,4Fh ; Mode 7
  6th Row        in bytes 51h,53h,55h,57h,59h,5Bh,5Dh,5Fh ;
  7th Row        in bytes 61h,63h,65h,67h,69h,6Bh,6Dh,6Fh ;
  Bottom Row     in bytes 71h,73h,75h,77h,79h,7Bh,7Dh,7Fh ;/
```

**16x16 (and bigger) Tiles**

BG tiles can be up to 16x16 pixels in size, and OBJs up to 64x64. In both
cases, the big tiles are combined of multiple 8x8 pixel tiles, whereas VRAM is
organized as "two-dimensional" array of 16x64 BG Tiles and 16x32 OBJ Tiles:

The BG Map or OAM entry contain the Tile number (N) for the upper-left 8x8
tile. The tile(s) right of that tile are N+1 (and N+2, N+3, etc). The tile(s)
under that tile are N+10h (and N+20h, N+30h, etc).

```text
  32x32 pixel OBJ Tile 000h
  Tile000h,  Tile001h,  Tile002h,  Tile003h
  Tile010h,  Tile011h,  Tile012h,  Tile013h
  Tile020h,  Tile021h,  Tile022h,  Tile023h
  Tile030h,  Tile031h,  Tile032h,  Tile033h
```

The hex-tile numbers could be thus thought of as "Yyxh", with "x" being the
4bit x-index, and "Yy" being the y-index in the array. For OBJ tiles, the are
no carry-outs from "x+1" to "y", nor from "y+1" to "Y". Whilst BG tiles are
processing carry-outs. For example:

```text
  16x16 BG Tile 1FFh     16x16 OBJ Tile 1FFh
  Tile1ffh Tile200h      Tile1ffh Tile1f0h
  Tile20fh Tile210h      Tile10fh Tile100h
```

**Accessing VRAM**

[SNES Memory VRAM Access (Tile and BG Map)](10-memory-and-io-map.md#snes-memory-vram-access-tile-and-bg-map)

<a id="snesppucolorpalettememorycgramanddirectcolors"></a>

## SNES PPU Color Palette Memory (CGRAM) and Direct Colors

> **Note (RustySNES ref):** Bit 15 of a 16-bit CGRAM color read is open bus (the MDR), not a defined 0, and should be masked. CGRAM is only reliably writable during Vblank/Hblank/force-blank; a write during active display lands at the wrong CGRAM address. See [SNESdev Wiki: PPU registers](https://snes.nesdev.org/wiki/PPU_registers).

**CGRAM Palette Entries**

```text
  15    Not used (should be zero) (read: PPU2 Open Bus)
  14-10 Blue
  9-5   Green
  4-0   Red
```

For accessing CGRAM, see:

[SNES Memory CGRAM Access (Palette Memory)](10-memory-and-io-map.md#snes-memory-cgram-access-palette-memory)

**CGRAM Palette Indices**

```text
  00h      Main Backdrop color (used when all BG/OBJ pixels are transparent)
  01h-FFh  256-color BG palette (when not using direct-color mode)
  01h-7Fh  128-color BG palette (BG2 in Mode 7)
  01h-7Fh  Eight 16-color BG palettes
  01h-1Fh  Eight 4-color BG palettes (except BG2-4 in Mode 0)
  21h-3Fh  Eight 4-color BG palettes (BG2 in Mode 0 only)
  41h-5Fh  Eight 4-color BG palettes (BG3 in Mode 0 only)
  61h-7Fh  Eight 4-color BG palettes (BG4 in Mode 0 only)
  81h-FFh  Eight 16-color OBJ palettes (half of them with color-math disabled)
  N/A      Sub Backdrop color (not in CGRAM, set via COLDATA, Port 2132h)
```

**Direct Color Mode**

256-color BGs (ie. BG1 in Mode 3,4,7) can be optionally set to direct-color
mode (via 2130h.Bit0; this bit hides in one of the Color-Math registers,
although it isn't related to Color-Math). The 8bit Color number is interpreted
as "BBGGGRRR", the 3bit Palette number (from the BG Map) contains LSBs as
"bgr", together they are forming a 15bit color "BBb00:RRRr0:GGGg0". Whereas the
"bgr" can defined only per tile (not per pixel), and it can be defined only in
BG Modes 3-4 (Mode 7 has no palette attributes in BG Map).

```text
  Color Bit7-0 all zero --> Transparent    ;-Color "Black" is Transparent!
  Color Bit7-6   Blue Bit4-3               ;\
  Palette Bit 2  Blue Bit2                 ; 5bit Blue
  N/A            Blue Bit1-0 (always zero) ;/
  Color Bit5-3   Green Bit4-2              ;\
  Palette Bit 1  Green Bit1                ; 5bit Green
  N/A            Green Bit0 (always zero)  ;/
  Color Bit2-0   Red Bit4-2                ;\
  Palette Bit 0  Red Bit1                  ; 5bit Red
  N/A            Red Bit0 (always zero)    ;/
```

To define Black, either set the Backdrop to Black (works only if there are no
layers behind BG1), or use a dark non-transparent "near-black" color (eg.
01h=Dark Red, 09h=Dark Brown).

**Screen Border Color**

The Screen Border is always Black (no matter of CGRAM settings and Sub Screen
Backdrop Color). NTSC 256x224 images are (more or less) fullscreen, so there
should be no visible screen border (however, a 2-3 pixel border may appear at
the screen edges if the screen isn't properly centered). Both PAL 256x224 and
PAL 256x239 images images will have black upper and lower screen borders (a
fullscreen PAL picture would be around 256x264, which isn't supported by the
SNES).

**Forced Blank Color**

In Forced Blank, the whole screen is Black (no matter of CGRAM settings, Sub
Screen Backdrop Color, and Master Brightness settings). Vsync/Hsync are kept
generated (sending a black picture with valid Sync signals to the TV set).

<a id="snesppuwindow"></a>

## SNES PPU Window

The window feature allows to disable BG/OBJ layers in selected regions, and
also to alter Color-Math effects in selected regions.

**2126h - WH0 - Window 1 Left Position (X1) (W)**

**2127h - WH1 - Window 1 Right Position (X2) (W)**

**2128h - WH2 - Window 2 Left Position (X1) (W)**

**2129h - WH3 - Window 2 Right Position (X2) (W)**

Specifies the horizontal boundaries of the windows. Note that there are no
vertical boundaries (these could be implemented by manipulating the window
registers via IRQ and/or HDMA).

```text
  7-0   Window Position (00h..0FFh; 0=leftmost, 255=rightmost)
```

The "inside-window" region extends from X1 to X2 (that, including the X1 and X2
coordinates), so the window width is X2-X1+1. If the width is zero (or
negative), then the "inside-window" becomes empty, and the whole screen will be
treated "outside-window".

**2123h - W12SEL - Window BG1/BG2 Mask Settings (W)**

**2124h - W34SEL - Window BG3/BG4 Mask Settings (W)**

**2125h - WOBJSEL - Window OBJ/MATH Mask Settings (W)**

```text
  Bit  2123h 2124h 2125h
  7-6  BG2   BG4   MATH  Window-2 Area (0..1=Disable, 1=Inside, 2=Outside)
  5-4  BG2   BG4   MATH  Window-1 Area (0..1=Disable, 1=Inside, 2=Outside)
  3-2  BG1   BG3   OBJ   Window-2 Area (0..1=Disable, 1=Inside, 2=Outside)
  1-0  BG1   BG3   OBJ   Window-1 Area (0..1=Disable, 1=Inside, 2=Outside)
```

Allows to select if the window area is inside or outside the X1,X2 coordinates,
or to disable the area.

**212Ah/212Bh - WBGLOG/WOBJLOG - Window 1/2 Mask Logic (W)**

```text
  Bit  212Ah 212Bh
  7-6  BG4   -     Window 1/2 Mask Logic (0=OR, 1=AND, 2=XOR, 3=XNOR)
  5-4  BG3   -     Window 1/2 Mask Logic (0=OR, 1=AND, 2=XOR, 3=XNOR)
  3-2  BG2   MATH  Window 1/2 Mask Logic (0=OR, 1=AND, 2=XOR, 3=XNOR)
  1-0  BG1   OBJ   Window 1/2 Mask Logic (0=OR, 1=AND, 2=XOR, 3=XNOR)
```

Allows to merge the Window 1 and 2 areas into a single "final" window area
(which is then used by TMW, TSW, and CGWSEL). The OR/AND/XOR/XNOR logic is
applied ONLY if BOTH window 1 and 2 are enabled (in WxxSEL registers). If only
one window is enabled, then that window is used as is as "final" area. If both
are disabled, then the "final" area will be empty. Note: "XNOR" means "1 XOR
area1 XOR area2" (ie. the inverse of the normal XOR result).

**212Eh - TMW - Window Area Main Screen Disable (W)**

**212Fh - TSW - Window Area Sub Screen Disable (W)**

```text
  7-5  Not used
  4    OBJ (0=Enable, 1=Disable)  ;\"Disable" forcefully disables the layer
  3    BG4 (0=Enable, 1=Disable)  ; within the window area (otherwise it is
  2    BG3 (0=Enable, 1=Disable)  ; enabled or disabled as selected in the
  1    BG2 (0=Enable, 1=Disable)  ; master enable bits in port 212Ch/212Dh)
  0    BG1 (0=Enable, 1=Disable)  ;/
  -    Backdrop (Always enabled)
```

Allows to disable video layers within the window region.

<a id="snesppucolormath"></a>

## SNES PPU Color-Math

**Main Screen / Sub Screen Enable**

Main Screen and Sub Screen BG/OBJ can be enabled via Port 212Ch/212Dh (see PPU
Control chapter). When using the Window feature, they can be additionally
disabled in selected areas via Port 212Eh/212Fh.

The Backdrops are always enabled as both Main and Sub screen. Of which, the Sub
screen backdrop can be effectively disabled (made fully transparent) by setting
its color to Black.

The PPU computes the Front-most Non-transparent Main-Screen Pixel, and
Front-most Non-transparent Sub-Screen Pixel (or if there's none, it uses the
Mainscreen or Subscreen Backdrop color).

**2130h - CGWSEL - Color Math Control Register A (W)**

```text
  7-6  Force Main Screen Black (3=Always, 2=MathWindow, 1=NotMathWin, 0=Never)
  5-4  Color Math Enable       (0=Always, 1=MathWindow, 2=NotMathWin, 3=Never)
  3-2  Not used
  1    Sub Screen BG/OBJ Enable    (0=No/Backdrop only, 1=Yes/Backdrop+BG+OBJ)
  0    Direct Color (for 256-color BGs)  (0=Use Palette, 1=Direct Color)
```

**2131h - CGADSUB - Color Math Control Register B (W)**

```text
  7    Color Math Add/Subtract        (0=Add; Main+Sub, 1=Subtract; Main-Sub)
  6    Color Math "Div2" Half Result  (0=No divide, 1=Divide result by 2)
  5    Color Math when Main Screen = Backdrop        (0=Off, 1=On) ;\
  4    Color Math when Main Screen = OBJ/Palette4..7 (0=Off, 1=On) ; OFF: Show
  -    Color Math when Main Screen = OBJ/Palette0..3 (Always=Off)  ; Raw Main,
  3    Color Math when Main Screen = BG4             (0=Off, 1=On) ;   or
  2    Color Math when Main Screen = BG3             (0=Off, 1=On) ; ON: Show
  1    Color Math when Main Screen = BG2             (0=Off, 1=On) ; Main+/-Sub
  0    Color Math when Main Screen = BG1             (0=Off, 1=On) ;/
```

Half-Color (Bit6): Ignored if "Force Main Screen Black" is used, also ignored
on transparent subscreen pixels (those use the fixed color as sub-screen
backdrop without division) (whilst 2130.1 uses the fixed color as
non-transparent one, which allows division).

Bit0-5: Seem to affect MAIN SCREEN layers:

```text
  Disable = Display RAW Main Screen as such (without math)
  Enable  = Apply math on Mainscreen
  (Ie. 212Ch enables the main screen, 2131h selects if math is applied on it)
```

**2132h - COLDATA - Color Math Sub Screen Backdrop Color (W)**

This 8bit port allows to manipulate some (or all) bits of a 15bit RGB value.
Examples: Black: write E0h (R,G,B=0), Cyan: write 20h (R=0) and DFh (G,B=1Fh).

```text
  7    Apply Blue  (0=No change, 1=Apply Intensity as Blue)
  6    Apply Green (0=No change, 1=Apply Intensity as Green)
  5    Apply Red   (0=No change, 1=Apply Intensity as Red)
  4-0  Intensity   (0..31)
```

The Sub Screen Backdrop Color is used when all sub screen layers are disabled
or transparent, in this case the "Div2" Half Color Math isn't applied (ie.
2131h.Bit6 is ignored); there is one exception: If "Sub Screen BG/OBJ Enable"
is off (2130h.Bit1=0), then the "Div2" isn't forcefully ignored.

For a FULLY TRANSPARENT backdrop: Set this register to Black (adding or
subtracting black has no effect, and, with "Div2" disabled/ignored, the raw
Main screen is displayed as is).

**Color Math**

Color Math can be disabled by setting 2130h.Bit4-5, or by clearing
2131h.Bit0-5. When it is disabled, only the Main Screen is displayed, and the
Sub Screen has no effect on the display.

Color Math occurs only if the front-most Main Screen pixel has math enabled
(via 2131h.Bit0-5.), and only if the front-most Sub Screen pixel has same or
higher (XXX or is it same or lower -- or is it ANY priority?) priority than the
Main Screen pixel.

Same priority means that, for example, BG1 can be mathed with itself: BG1+BG1
gives double-brightness (or same brightness when Div2 is enabled), and, BG1-BG1
gives Black.

Addition/Subtraction is done per R,G,B color fragment, the results can be
(optionally) divided by 2, and are then saturated to Max=31 and Min=0).

**Force Main Screen Black**

This feature forces the whole Main Screen (including Backdrop) to become black,
so, normally, the whole screen becomes black. However, color addition can be
still applied (but, with the "Div2" not being applied). Whereas, although it
looks all black, the Main Screen is still divided into black BG1 pixels, black
BG2 pixels, and so on. Of which, one can disable Color Math for some of the
pixels. Color Subtraction has no effect (since the pixels can't get blacker
than black).

**Hires and Pseudo 3-Layer Math**

In Hires modes (BG Mode 5,6 and Pseudo Hires via SETINI), the main/sub screen
pixels are rendered as half-pixels of the high-resolution image. The TV picture
is so blurry, that the result will look quite similar to Color Addition with
Div2 - some games (Jurassic Park and Kirby's Dream Land 3) are actually using
it for that purpose; the advantage is that one can additionally apply COLDATA
addition to (both) main/sub-screen layers, ie. the result looks like
"(main+sub)/2+coldata".

<a id="snespputimersandstatus"></a>

## SNES PPU Timers and Status

**2137h - SLHV - Latch H/V-Counter by Software (R)**

```text
  7-0  Not used (CPU Open Bus; usually last opcode, 21h for "MOV A,[2137h]")
```

Reading from this register latches the current H/V counter values into
OPHCT/OPVCT, Ports 213Ch and 213Dh.

Reading here works "as if" dragging IO7 low (but it does NOT actually output a
LOW level to IO7 on joypad 2).

**213Ch - OPHCT - Horizontal Counter Latch (R)**

**213Dh - OPVCT - Vertical Counter Latch (R)**

There are three situations that do load H/V counter values into the latches:

```text
  Doing a dummy-read from SLHV (Port 2137h) by software
  Switching WRIO (Port 4201h) Bit7 from 1-to-0 by software
  Lightgun High-to-Low transition (Pin6 of 2nd Controller connector)
```

All three methods are working only if WRIO.Bit7 is (or was) set. If so, data is
latched, and (in all three cases) the latch flag in 213Fh.Bit6 is set.

```text
  1st read  Lower 8bit
  2nd read  Upper 1bit (other 7bit PPU2 open bus; last value read from PPU2)
```

There are two separate 1st/2nd-read flipflops (one for OPHCT, one for OPVCT),
both flipflops can be reset by reading from Port 213Fh (STAT78), the flipflops
aren't automatically reset when latching occurs.

```text
        H Counter values range from 0 to 339, with 22-277 being visible on the
        screen. V Counter values range from 0 to 261 in NTSC mode (262 is
        possible every other frame when interlace is active) and 0 to 311 in
        PAL mode (312 in interlace?), with 1-224 (or 1-239(?) if overscan is
        enabled) visible on the screen.
```

**213Eh - STAT77 - PPU1 Status and Version Number (R)**

```text
  7    OBJ Time overflow  (0=Okay, 1=More than 8x34 OBJ pixels per scanline)
  6    OBJ Range overflow (0=Okay, 1=More than 32 OBJs per scanline)
  5    Master/Slave Mode (PPU1.Pin25) (0=Normal=Master)
  4    Not used (PPU1 open bus) (same as last value read from PPU1)
  3-0  PPU1 5C77 Version Number (only version 1 exists as far as I know)
```

The overflow flags are cleared at end of V-Blank, but NOT during forced blank!

The overflow flags are set (regardless of OBJ enable/disable in 212Ch), at
following times: Bit6 when V=OBJ.YLOC/H=OAM.INDEX*2, bit7 when
V=OBJ.YLOC+1/H=0.

**213Fh - STAT78 - PPU2 Status and Version Number (R)**

```text
  7    Current Interlace-Frame (0=1st, 1=2nd Frame)
  6    H/V-Counter/Lightgun/Joypad2.Pin6 Latch Flag (0=No, 1=New Data Latched)
  5    Not used (PPU2 open bus) (same as last value read from PPU2)
  4    Frame Rate (PPU2.Pin30)  (0=NTSC/60Hz, 1=PAL/50Hz)
  3-0  PPU2 5C78 Version Number (version 1..3 exist as far as I know)
```

Reading from this register also resets the latch flag (bit6), and resets the
two OPHCT/OPVCT 1st/2nd-read flipflops.

**Lightgun Coordinates**

The screen coordinates (X,Y; with 0,0 = upper left) are related as so:

```text
  Super Scope  X=OPHCNT-40, Y=OPVCT-1   (games support software calibration)
  Justifier 1  X=OPHCNT-??, Y=OPVCT-?   (games support software calibration)
  Justifier 2  X=OPHCNT-??, Y=OPVCT-?   (games support software calibration)
  M.A.C.S.     X=OPHCNT-76, Y=OPVCT-41  (hardcoded, mechanical calibration)
```

Drawing starts at H=22, V=1 (which explains parts of the offsets). Horizontal
offsets greater than 22 are explained by signal switching/transmission delays.

The huge vertical offset of the M.A.C.S. gun is caused by the lightpen being
mounted above of the barrel (rather than inside of it) (and apparently not
parallel to it, since V&gt;Y instead of V&lt;Y), this implies that the barrel
cannot be aimed at screen coordinates Y=151..191 (since the lightpen would be
offscreen).

Most games support software calibration. The only exception is M.A.C.S. which
requires mechanical hardware calibration (assisted by a test screen, activated
by pressing Button A, and then Select Button on joypad 1).

[SNES Controllers SuperScope (Lightgun)](50-controllers.md#snes-controllers-superscope-lightgun)

[SNES Controllers Konami Justifier (Lightgun)](50-controllers.md#snes-controllers-konami-justifier-lightgun)

[SNES Controllers M.A.C.S. (Lightgun)](50-controllers.md#snes-controllers-macs-lightgun)

<a id="snesppuinterrupts"></a>

## SNES PPU Interrupts

**4200h - NMITIMEN - Interrupt Enable and Joypad Request (W)**

```text
  7     VBlank NMI Enable  (0=Disable, 1=Enable) (Initially disabled on reset)
  6     Not used
  5-4   H/V IRQ (0=Disable, 1=At H=H + V=Any, 2=At V=V + H=0, 3=At H=H + V=V)
  3-1   Not used
  0     Joypad Enable    (0=Disable, 1=Enable Automatic Reading of Joypad)
```

Disabling IRQs (via bit4-5) does additionally acknowledge IRQs. There's no such
effect when disabling NMIs (via bit7).

**4207h/4208h - HTIMEL/HTIMEH - H-Count Timer Setting (W)**

```text
  15-9  Not used
  8-0   H-Count Timer Value (0..339) (+/-1 in long/short lines) (0=leftmost)
```

The H/V-IRQ flag in Bit7 of TIMEUP, Port 4211h gets set when the H-Counter gets
equal to the H-Count register value.

**4209h/420Ah - VTIMEL/VTIMEH - V-Count Timer Setting (W)**

```text
  15-9  Not used
  8-0   V-Count Timer Value (0..261/311, NTSC/PAL) (+1 in interlace) (0=top)
```

The H/V-IRQ flag in Bit7 of TIMEUP, Port 4211h gets set when the V-Counter gets
equal to the V-Count register value.

**4210h - RDNMI - V-Blank NMI Flag and CPU Version Number (R) (Read/Ack)**

```text
  7     Vblank NMI Flag  (0=None, 1=Interrupt Request) (set on Begin of Vblank)
  6-4   Not used
  3-0   CPU 5A22 Version Number (version 2 exists)
```

The NMI flag gets set at begin of Vblank (this happens even if NMIs are
disabled). The flag gets reset automatically at end of Vblank, and gets also
reset after reading from this register.

The SNES has only one NMI source (vblank), and the NMI flag is automatically
reset (on vblank end), so there's normally no need to read/acknowledge the
flag, except one special case: If one does disable and re-enable NMIs, then an
old NMI may be executed again; acknowledging avoids that effect.

The CPU includes another internal NMI flag, which gets set when "[4200h].7 AND
[4210h].7" changes from 0-to-1, and gets cleared when the NMI gets executed
(which should happen around after the next opcode) (if a DMA transfer is in
progress, then it is somewhere after the DMA, in that case the NMI can get
executed outside of the Vblank period, ie. at a time when [4210h].7 is no
longer set).

**4211h - TIMEUP - H/V-Timer IRQ Flag (R) (Read/Ack)**

```text
  7     H/V-Count Timer IRQ Flag (0=None, 1=Interrupt Request)
  6-0   Not used
```

The IRQ flag is automatically reset after reading from this register (except
when reading at the very time when the IRQ condition is true (which lasts for
4-8 master cycles), then the CPU receives bit7=1, but register bit7 isn't
cleared). The flag is also automatically cleared when disabling IRQs (by
setting 4200h.Bit5-4 to zero).

Unlike NMI handlers, IRQ handlers MUST acknowledge IRQs, otherwise the IRQ gets
executed again (ie. immediately after the RTI opcode).

**4212h - HVBJOY - H/V-Blank flag and Joypad Busy flag (R)**

```text
  7     V-Blank Period Flag (0=No, 1=VBlank)
  6     H-Blank Period Flag (0=No, 1=HBlank)
  5-1   Not used
  0     Auto-Joypad-Read Busy Flag (1=Busy) (see 4200h, and 4218h..421Fh)
```

The Hblank flag gets toggled in ALL scanlines (including during Vblank/Vsync).
Both Vblank and Hblank are always toggling (even during Forced Blank, and no
matter if IRQs or NMIs are enabled).

**Other IRQ Sources**

IRQs can be also triggered via Cartridge Slot and Expansion Port. This is done
by cartridges with SA-1 and GSU chips (CX4 carts do also have an IRQ line,
although the existing games don't seem to use it).

<a id="snesppuresolution"></a>

## SNES PPU Resolution

**Physical Resolution**

The physical resolution (of the TV Screen) is:

```text
  256x224 for 60Hz (NTSC) consoles
  256x264 for 50Hz (PAL) consoles
```

The 50Hz/60Hz cannot be changed by software (it can be only changed by modding
some pins on the mainboard).

**Normal Picture Resolution**

The vertical resolution is software-selectable, 224 or 239 lines:

```text
  256x224 near-fullscreen on 60Hz (NTSC) consoles (or Tiny Picture at 50Hz)
  256x239 not-really-fullscreen on 50Hz (PAL) consoles (or Overscan at 60Hz)
```

Most commonly the resolution is selected matching to the frame rate. With 60Hz
Overscan the upper/lower lines are normally not visible (but may be useful to
ensure that there is absolutely no upper/lower border visible, which may be
important for avoiding flickering in interlace mode, especially with bright
pictures; in contrast to the black screen border). 50Hz Tiny Picture would be a
simple solution for porting 224-lines NTSC games to PAL, though it produces
extra-big upper/lower borders.

**High-Resolution Modes**

There are some methods to double the resolution horizontally and/or vertically:

**True High-Resolution (BG Mode 5,6) (optionally with SETINI.0)**

...

**Pseudo Horizontal High-Resolution (SETINI.3)**

...

**Pseudo Vertical High-Resolution (SETINI.0) (Interlace)**

...

**OBJ High-Resolution (SETINI.1)**

...

**Hires Notes**

Horizontal BG Scrolling is always counted in full-pixels; so true-hires cannot
be scrolled in half-pixel units (whilst pseudo hires may be scrolled in
half-pixels by exchanging main/sub-screen layers and using different scroll
offsets for each layer).

Vertical BG Scrolling is counted in half-pixels (only when using true
hv-hires).

Mosaic is always counted in full-pixels; so a mosaic size of 1x1 (which is
normally same as mosaic disabled) acts as 2x1 half-pixels in true h-hires mode
(and as 2x2 half-pixels in true hv-hires mode, reportedly?) (and as 1x2 in
pseudo v-hires, presumably?).

Although h-hires uses main/subscreen for the half-pixels, both main/subscreen
pixels are forced to use Color 0 as backdrop color (rather than using COLDATA
setting as subscreen backdrop) (this applies for both true+pseudo hires).

Window X1/X2 coordinates are always counted in full-pixels (reportedly with an
odd glitch in hires mode?).

OBJ X/Y coordinates are always counted in full-pixels.

**Hires Appearance**

Horizontal hires may appear blurred on most TV sets (due to the quality of the
video signal and TV-screen, and, when not using a RGB-cable, of the composite
color clock). For example, non-continous white pixels (W) on black background
may appear as gray pixels (g):

```text
  Source Pixels       --->  TV-Screen Pixels
  WW   W   WW   W W         WW   g   WW   ggg
  WWW   W   WW   W W        WWW   g   WW   ggg
```

To reproduce that by software: CenterPix=(LeftPix+CenterPix*2+RightPix)/4.

Using the above example on a red background looks even worse: the "g" and "ggg"
portions may be barely visible (appear as slightly pastelized red shades).

Vertical hires is producing a flickering image: The scanlines jump up/down
after each frame (and, due to a hardware glitch: the topmost lines also jump
left/right on PAL consoles). The effect is most annoying with hard-contrast
images (eg. black/white text/lines, or bright pixels close to upper/lower
screen borders), it may look less annoying with blurry images (eg. photos, or
anti-alized text/lines). Some modern TV sets might be buffering the even/odd
frames in internal memory, and display them as flicker-free hires image(?)

**Hires Software**

```text
  Air Strike Patrol (mission overview)       (whatever mode? with Interlace)
  Bishoujo Wrestler Retsuden (some text)     (512x448, BgMode5+Interlace)
  Ball Bullet Gun (in lower screen half)     (512x224, BgMode5)
  Battle Cross (in game) (but isn't hires?)  (512x224, BgMode1+PseudoH)(Bug?)
  BS Radical Dreamers (user name input only) (512x224, BgMode5)
  Chrono Trigger (crash into Lavos sequence) (whatever mode? with Interlace)
  Donkey Kong Country 1 (Nintendo logo)      (512x224, BgMode5)
  G.O.D. (intro & lower screen half)         (512x224, BgMode5)
  Jurassic Park (score text)                 (512x224, BgMode1+PseudoH+Math)
  Kirby's Dream Land 3 (leaves in 1st door)  (512x224, BgMode1+PseudoH)
  Lufia 2 (credits screen at end of game)    (whatever mode?)
  Moryo Senki Madara 2 (text)                (512x224, BgMode5)
  Power Drive (in intro)                     (512x448, BgMode5+Interlace)
  Ranma 1/2: Chounai Gekitou Hen             (256x448, BgMode1+InterlaceBug)
  RPM Racing (in intro and in game)          (512x448, BgMode5+Interlace)
  Rudra no Hihou (RnH/Treasure of the Rudras)(512x224, BgMode5)
  Seiken Densetsu 2 (Secret of Mana) (setup) (512x224, BgMode5)
  Seiken Densetsu 3                          (512x224, BgMode5)
  Shock Issue 1 & 2 (homebrew eZine)         (512x224, BgMode5)
  SNES Test Program (by Nintendo) (Character Test includes BgMode5/BgMode6)
  Super Play Action Football (text)          (512x224, BgMode5)
  World Cup Striker (intro/menu)             (512x224, BgMode5)
```

And, reportedly (may be incorrect, unknown how to see hires in that games):

```text
  Dragonball Z Super Butoden 2-3 (when you start it up in the black screen?)
```

Notes: Ranma is actually only 256x224 (but does accidentally have interlace
enabled, which causes some totally useless flickering). Jurassic/Kirby are
misusing PseudoH for "blending" the main+sub screen layers (via the blurry TV
picture); this allows to use Color Math for coldata additions (ie. resulting in
3-color blending: main+sub+coldata).

**3D Software**

[SNES 3D Glasses](50-controllers.md#snes-3d-glasses)

<a id="snesppuoffsetpertilemode"></a>

## SNES PPU Offset-Per-Tile Mode

XXX - Under construction (see Anomie's docs for now)

**Offset-Per-Tile Mode is used by following Programs**

```text
  Chrono Trigger (title screen, intro's "Black Omen" appearing)
  Star Fox/Starwing (to "rotate" the landscape background)
  Tetris Attack
  Yoshi's Island (dizziness effect, wavy lava)
```
