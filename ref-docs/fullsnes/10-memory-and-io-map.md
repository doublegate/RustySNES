# Fullsnes — Memory Map & I/O Map

[Index](00-index.md) · [DMA & HDMA Transfers »](20-dma-hdma.md)

**Sections in this file:**

- [SNES I/O Map](#snes-io-map)
- [SNES Memory](#snes-memory)
  - [SNES Memory Map](#snes-memory-map)
  - [SNES Memory Control](#snes-memory-control)
  - [SNES Memory Work RAM Access](#snes-memory-work-ram-access)
  - [SNES Memory OAM Access (Sprite Attributes)](#snes-memory-oam-access-sprite-attributes)
  - [SNES Memory VRAM Access (Tile and BG Map)](#snes-memory-vram-access-tile-and-bg-map)
  - [SNES Memory CGRAM Access (Palette Memory)](#snes-memory-cgram-access-palette-memory)

---

<a id="snesiomap"></a>

## SNES I/O Map

**First some bytes**

```text
  0000h..1FFFh - WRAM - Mirror of first 8Kbyte of WRAM (at 7E0000h-7E1FFFh)
  2000h..20FFh - N/A  - Unused
```

**PPU Picture Processing Unit (Write-Only Ports)**

```text
  2100h - INIDISP - Display Control 1                                  8xh
  2101h - OBSEL   - Object Size and Object Base                        (?)
  2102h - OAMADDL - OAM Address (lower 8bit)                           (?)
  2103h - OAMADDH - OAM Address (upper 1bit) and Priority Rotation     (?)
  2104h - OAMDATA - OAM Data Write (write-twice)                       (?)
  2105h - BGMODE  - BG Mode and BG Character Size                      (xFh)
  2106h - MOSAIC  - Mosaic Size and Mosaic Enable                      (?)
  2107h - BG1SC   - BG1 Screen Base and Screen Size                    (?)
  2108h - BG2SC   - BG2 Screen Base and Screen Size                    (?)
  2109h - BG3SC   - BG3 Screen Base and Screen Size                    (?)
  210Ah - BG4SC   - BG4 Screen Base and Screen Size                    (?)
  210Bh - BG12NBA - BG Character Data Area Designation                 (?)
  210Ch - BG34NBA - BG Character Data Area Designation                 (?)
  210Dh - BG1HOFS - BG1 Horizontal Scroll (X) (write-twice) / M7HOFS   (?,?)
  210Eh - BG1VOFS - BG1 Vertical Scroll (Y)   (write-twice) / M7VOFS   (?,?)
  210Fh - BG2HOFS - BG2 Horizontal Scroll (X) (write-twice)            (?,?)
  2110h - BG2VOFS - BG2 Vertical Scroll (Y)   (write-twice)            (?,?)
  2111h - BG3HOFS - BG3 Horizontal Scroll (X) (write-twice)            (?,?)
  2112h - BG3VOFS - BG3 Vertical Scroll (Y)   (write-twice)            (?,?)
  2113h - BG4HOFS - BG4 Horizontal Scroll (X) (write-twice)            (?,?)
  2114h - BG4VOFS - BG4 Vertical Scroll (Y)   (write-twice)            (?,?)
  2115h - VMAIN   - VRAM Address Increment Mode                        (?Fh)
  2116h - VMADDL  - VRAM Address (lower 8bit)                          (?)
  2117h - VMADDH  - VRAM Address (upper 8bit)                          (?)
  2118h - VMDATAL - VRAM Data Write (lower 8bit)                       (?)
  2119h - VMDATAH - VRAM Data Write (upper 8bit)                       (?)
  211Ah - M7SEL   - Rotation/Scaling Mode Settings                     (?)
  211Bh - M7A     - Rotation/Scaling Parameter A & Maths 16bit operand(FFh)(w2)
  211Ch - M7B     - Rotation/Scaling Parameter B & Maths 8bit operand (FFh)(w2)
  211Dh - M7C     - Rotation/Scaling Parameter C         (write-twice) (?)
  211Eh - M7D     - Rotation/Scaling Parameter D         (write-twice) (?)
  211Fh - M7X     - Rotation/Scaling Center Coordinate X (write-twice) (?)
  2120h - M7Y     - Rotation/Scaling Center Coordinate Y (write-twice) (?)
  2121h - CGADD   - Palette CGRAM Address                              (?)
  2122h - CGDATA  - Palette CGRAM Data Write             (write-twice) (?)
  2123h - W12SEL  - Window BG1/BG2 Mask Settings                       (?)
  2124h - W34SEL  - Window BG3/BG4 Mask Settings                       (?)
  2125h - WOBJSEL - Window OBJ/MATH Mask Settings                      (?)
  2126h - WH0     - Window 1 Left Position (X1)                        (?)
  2127h - WH1     - Window 1 Right Position (X2)                       (?)
  2128h - WH2     - Window 2 Left Position (X1)                        (?)
  2129h - WH3     - Window 2 Right Position (X2)                       (?)
  212Ah - WBGLOG  - Window 1/2 Mask Logic (BG1-BG4)                    (?)
  212Bh - WOBJLOG - Window 1/2 Mask Logic (OBJ/MATH)                   (?)
  212Ch - TM      - Main Screen Designation                            (?)
  212Dh - TS      - Sub Screen Designation                             (?)
  212Eh - TMW     - Window Area Main Screen Disable                    (?)
  212Fh - TSW     - Window Area Sub Screen Disable                     (?)
  2130h - CGWSEL  - Color Math Control Register A                      (?)
  2131h - CGADSUB - Color Math Control Register B                      (?)
  2132h - COLDATA - Color Math Sub Screen Backdrop Color               (?)
  2133h - SETINI  - Display Control 2                                  00h?
```

**PPU Picture Processing Unit (Read-Only Ports)**

```text
  2134h - MPYL    - PPU1 Signed Multiply Result   (lower 8bit)         (01h)
  2135h - MPYM    - PPU1 Signed Multiply Result   (middle 8bit)        (00h)
  2136h - MPYH    - PPU1 Signed Multiply Result   (upper 8bit)         (00h)
  2137h - SLHV    - PPU1 Latch H/V-Counter by Software (Read=Strobe)
  2138h - RDOAM   - PPU1 OAM Data Read            (read-twice)
  2139h - RDVRAML - PPU1 VRAM Data Read           (lower 8bits)
  213Ah - RDVRAMH - PPU1 VRAM Data Read           (upper 8bits)
  213Bh - RDCGRAM - PPU2 CGRAM Data Read (Palette)(read-twice)
  213Ch - OPHCT   - PPU2 Horizontal Counter Latch (read-twice)         (01FFh)
  213Dh - OPVCT   - PPU2 Vertical Counter Latch   (read-twice)         (01FFh)
  213Eh - STAT77  - PPU1 Status and PPU1 Version Number
  213Fh - STAT78  - PPU2 Status and PPU2 Version Number                Bit7=0
```

**APU Audio Processing Unit (R/W)**

```text
  2140h - APUI00  - Main CPU to Sound CPU Communication Port 0        (00h/00h)
  2141h - APUI01  - Main CPU to Sound CPU Communication Port 1        (00h/00h)
  2142h - APUI02  - Main CPU to Sound CPU Communication Port 2        (00h/00h)
  2143h - APUI03  - Main CPU to Sound CPU Communication Port 3        (00h/00h)
  2144h..217Fh    - APU Ports 2140-2143h mirrored to 2144h..217Fh
```

**WRAM Access**

```text
  2180h - WMDATA  - WRAM Data Read/Write       (R/W)
  2181h - WMADDL  - WRAM Address (lower 8bit)  (W)                        00h
  2182h - WMADDM  - WRAM Address (middle 8bit) (W)                        00h
  2183h - WMADDH  - WRAM Address (upper 1bit)  (W)                        00h
  2184h..21FFh    - Unused region (open bus) / Expansion (B-Bus)          -
  2200h..3FFFh    - Unused region (open bus) / Expansion (A-Bus)          -
```

**CPU On-Chip I/O Ports**

```text
  4000h..4015h        - Unused region (open bus)      ;\These ports have  -
  4016h/Write - JOYWR - Joypad Output (W)             ; long waitstates   00h
  4016h/Read  - JOYA  - Joypad Input Register A (R)   ; (1.78MHz cycles)  -
  4017h/Read  - JOYB  - Joypad Input Register B (R)   ; (all other ports  -
  4018h..41FFh        - Unused region (open bus)      ;/are 3.5MHz fast)  -
```

**CPU On-Chip I/O Ports (Write-only) (Read=open bus)**

```text
  4200h - NMITIMEN- Interrupt Enable and Joypad Request                   00h
  4201h - WRIO    - Joypad Programmable I/O Port (Open-Collector Output)  FFh
  4202h - WRMPYA  - Set unsigned 8bit Multiplicand                        (FFh)
  4203h - WRMPYB  - Set unsigned 8bit Multiplier and Start Multiplication (FFh)
  4204h - WRDIVL  - Set unsigned 16bit Dividend (lower 8bit)              (FFh)
  4205h - WRDIVH  - Set unsigned 16bit Dividend (upper 8bit)              (FFh)
  4206h - WRDIVB  - Set unsigned 8bit Divisor and Start Division          (FFh)
  4207h - HTIMEL  - H-Count Timer Setting (lower 8bits)                   (FFh)
  4208h - HTIMEH  - H-Count Timer Setting (upper 1bit)                    (01h)
  4209h - VTIMEL  - V-Count Timer Setting (lower 8bits)                   (FFh)
  420Ah - VTIMEH  - V-Count Timer Setting (upper 1bit)                    (01h)
  420Bh - MDMAEN  - Select General Purpose DMA Channel(s) and Start Transfer 0
  420Ch - HDMAEN  - Select H-Blank DMA (H-DMA) Channel(s)                    0
  420Dh - MEMSEL  - Memory-2 Waitstate Control                               0
  420Eh..420Fh    - Unused region (open bus)                                 -
```

**CPU On-Chip I/O Ports (Read-only)**

```text
  4210h - RDNMI   - V-Blank NMI Flag and CPU Version Number (Read/Ack)      0xh
  4211h - TIMEUP  - H/V-Timer IRQ Flag (Read/Ack)                           00h
  4212h - HVBJOY  - H/V-Blank flag and Joypad Busy flag (R)                 (?)
  4213h - RDIO    - Joypad Programmable I/O Port (Input)                    -
  4214h - RDDIVL  - Unsigned Division Result (Quotient) (lower 8bit)        (0)
  4215h - RDDIVH  - Unsigned Division Result (Quotient) (upper 8bit)        (0)
  4216h - RDMPYL  - Unsigned Division Remainder / Multiply Product (lower 8bit)
  4217h - RDMPYH  - Unsigned Division Remainder / Multiply Product (upper 8bit)
  4218h - JOY1L   - Joypad 1 (gameport 1, pin 4) (lower 8bit)               00h
  4219h - JOY1H   - Joypad 1 (gameport 1, pin 4) (upper 8bit)               00h
  421Ah - JOY2L   - Joypad 2 (gameport 2, pin 4) (lower 8bit)               00h
  421Bh - JOY2H   - Joypad 2 (gameport 2, pin 4) (upper 8bit)               00h
  421Ch - JOY3L   - Joypad 3 (gameport 1, pin 5) (lower 8bit)               00h
  421Dh - JOY3H   - Joypad 3 (gameport 1, pin 5) (upper 8bit)               00h
  421Eh - JOY4L   - Joypad 4 (gameport 2, pin 5) (lower 8bit)               00h
  421Fh - JOY4H   - Joypad 4 (gameport 2, pin 5) (upper 8bit)               00h
  4220h..42FFh    - Unused region (open bus)                                -
```

**CPU DMA, For below ports, x = Channel number 0..7 (R/W)**

```text
  (additional DMA control registers are 420Bh and 420Ch, see above)
  43x0h - DMAPx   - DMA/HDMA Parameters                                   (FFh)
  43x1h - BBADx   - DMA/HDMA I/O-Bus Address (PPU-Bus aka B-Bus)          (FFh)
  43x2h - A1TxL   - HDMA Table Start Address (low)  / DMA Curr Addr (low) (FFh)
  43x3h - A1TxH   - HDMA Table Start Address (high) / DMA Curr Addr (high)(FFh)
  43x4h - A1Bx    - HDMA Table Start Address (bank) / DMA Curr Addr (bank)(xxh)
  43x5h - DASxL   - Indirect HDMA Address (low)  / DMA Byte-Counter (low) (FFh)
  43x6h - DASxH   - Indirect HDMA Address (high) / DMA Byte-Counter (high)(FFh)
  43x7h - DASBx   - Indirect HDMA Address (bank)                          (FFh)
  43x8h - A2AxL   - HDMA Table Current Address (low)                      (FFh)
  43x9h - A2AxH   - HDMA Table Current Address (high)                     (FFh)
  43xAh - NTRLx   - HDMA Line-Counter (from current Table entry)          (FFh)
  43xBh - UNUSEDx - Unused byte (read/write-able)                         (FFh)
  43xCh+  -         Unused region (open bus)                                -
  43xFh - MIRRx   - Mirror of 43xBh (R/W)                                 (FFh)
  4380h..5FFFh    - Unused region (open bus)                                -
```

**Further Memory**

```text
  6000h..7FFFh    - Expansion (eg. Battery Backed RAM, in HiROM cartridges)
  8000h..FFFFh    - Cartridge ROM
```

Note: The right column shows the initial value on Reset, values in brackets are
left unchanged upon Reset (but do contain the specified value upon initial
Power-up).

**Audio Registers (controlled by the SPC700 CPU, not by the Main CPU)**

[SNES APU Memory and I/O Map](40-apu-dsp.md#snes-apu-memory-and-io-map)

**Expansion Overview**

```text
  0000h-003Fh  Cheat Device: Pro Action Replay 1-3: I/O Ports; overlapping WRAM
  2000h-2007h  MSU1: Media Streaming Unit (homebrew)
  2184h-218Fh  Copier: Super UFO models Pro-7 and Pro-8
  2188h-2199h  Satellaview Receiver Unit (connected to Expansion Port)
  21C0h-21C3h  More or less "used" by Nintendo's "SNES Test" cartridge
  21C0h-21DFh  Exertainment (exercise bicycle) (connected to Expansion Port)
  21D0h-21E5h  Sony Super Disc prototype: CDROM Controller & BIOS Cart RAM lock
  21FCh-21FFh  Nocash Debug Extension (char_out and 21mhz_timer in no$sns emu)
  2200h-230Eh  SA-1 (programmable 65C816 CPU) I/O Ports
  2400h-2407h  Nintendo Power (flashcard) (bank 00h only, no mirrors)
  2800h-2801h  S-RTC Real Time Clock I/O Ports
  2800h-2810h  Copier: FDC I/O Ports CCL (Supercom Partner & Pro Fighter)
  2C00h-2FFFh  Cheat Device: X-Terminator 2: SRAM (32Kbytes, in banks 00h-1Fh)
  3000h-32FFh  GSU-n (programmable RISC CPU) I/O Ports
  3000h-37FFh  SA-1 (programmable 65C816 CPU) on-chip I-RAM
  3800h-3804h  ST018 (Seta) (pre-programmed ARM CPU) (maybe also FF40h..FF63h)
  4100h        Nintendo Super System (NSS) DIP-Switches (on game cartridge)
  4800h-4807h  S-DD1 Data Decompression chip
  4800h-4842h  SPC7110 Data Decompression chip (optionally with RTC-4513)
  5000h-5FFFh  Satellaview MCC mem ctrl (sixteen 1bit I/O ports banks 00h-0Fh)
  5000h-5FFFh  Satellaview 32Kbyte SRAM (eight 4Kbyte-chunks in banks 10h-17h)
  58xxh        Copier: Venus (Multi Game Hunter)
  5Fxxh        Copier: Gamars Super Disk
  6000h-7FFFh  SRAM Battery Backed Static RAM (in HiROM cartridges) (bank 3xh)
  6000h-7FFFh  SGB Super Gameboy I/O Ports
  6000h-7FFFh  DSP-n on HiROM boards (pre-programmed NEC uPD77C25 CPU)
  7F40h-7FAFh  CX4 I/O Ports (with 3K SRAM at 6000h..6BFFh)
  7FF0h-7FF7h  OBC1 OBJ Controller I/O Ports (with 8K SRAM at 6000h..7FFFh)
  8000h-FFFFh  Cartridge ROM (including header/exception vectors at FFxxh)
  8000h        Pirate X-in-1 Multicart 32K-ROM bank (mapping 20 small games)
  8000h-FFFFh  Copier: Various models map ROM, I/O, SRAM, DRAM in ROM area
  8000h-8101h  Cheat Device: Game Genie I/O Ports (in ROM banks 00h and FFh)
  A000h-A007h  Cheat Device: Pro Action Replay 2: I/O Control (in HiROM area)
  FFE0h-FFFFh  Exception Vectors (variable in SA-1 and CX4) (fixed in GSU)
  FFE8h-FFEAh  Cheat Device: X-Terminator 1-2: I/O Ports (in LoROM area)
  FFF0h-FFF3h  Tri-Star/Super 8: NES Joypad 1-2, BIOS-disable, A/V-select
  xF8000h-3FFFFFh  DSP-n on LoROM boards (pre-programmed NEC uPD77C25 CPU)
  600000h-6F7FFFh  DSP-n on 2Mbyte-LoROM boards (planned/prototype only)
  600000h-67FFFFh  ST010/ST011 Command/Status/Parameters I/O Ports
  680000h-6FFFFFh  ST010/ST011 On-chip Battery-backed RAM
  700000h-7xxxxxh  SRAM Battery Backed Static RAM (in LoROM cartridges)
  808000h-BFFFFFh  Bootleg Copy-Protection I/O Ports
  C00000h-FFFFFFh  Satellaview FLASH Cartridges (Detect/Write/Erase commands)
  C00000h-Cn7FFFh  JRA PAT Backup FLASH Memory  (Detect/Write/Erase commands)
  C08000h-FFFFFFh  Satellaview-like FLASH Data Packs in LoROM Cartridges
  E00000h-FFFFFFh  Satellaview-like FLASH Data Packs in HiROM Cartridges
  E00000h-E0FFFFh  X-Band Modem 64K SRAM (in two 32Kx8 chips)
  FBC000h-FBC1BFh  X-Band Modem I/O Ports (mainly in this area)
  FFFF00h-FFFFFFh  Pirate X-in-1 multicart mapper I/O port
```

<a id="snesmemory"></a>

## SNES Memory

[SNES Memory Map](#snes-memory-map)

[SNES Memory Control](#snes-memory-control)

**Work RAM (WRAM)**

Work RAM is mapped directly to the CPU bus, and can be additionally accessed
indirectly via I/O ports (mainly for DMA transfer purposes).

[SNES Memory Work RAM Access](#snes-memory-work-ram-access)

**Video Memory (OAM/VRAM/CGRAM)**

All video memory can be accessed only during V-Blank, or Forced Blank.

Video memory isn't mapped to the CPU bus, and can be accessed only via I/O
ports (for bigger transfers, this would be usually done via DMA).

[SNES Memory OAM Access (Sprite Attributes)](#snes-memory-oam-access-sprite-attributes)

[SNES Memory VRAM Access (Tile and BG Map)](#snes-memory-vram-access-tile-and-bg-map)

[SNES Memory CGRAM Access (Palette Memory)](#snes-memory-cgram-access-palette-memory)

Access during H-Blank doesn't seem to work too well - it is possible to change
palette entries during H-Blank, but seems to work only during a few clock
cycles, not during the full H-blank period.

**Sound RAM**

Sound RAM is mapped to a separate SPC700 CPU, not to the Main CPU. Accordingly,
Sound RAM cannot be directly accessed by the Main CPU (nor by DMA). Instead,
data transfers must be done by using some CPU-to-CPU software communication
protocol. Upon Reset, this done by a Boot-ROM on the SPC700 side. For details,
see:

[SNES APU Main CPU Communication Port](40-apu-dsp.md#snes-apu-main-cpu-communication-port)

**DMA Transfers**

DMA can be used to quickly transfer memory blocks to/from most memory locations
(except Sound RAM isn't accessible via DMA, and WRAM-to-WRAM transfers don't
work).

[SNES DMA Transfers](20-dma-hdma.md#snes-dma-transfers)

<a id="snesmemorymap"></a>

### SNES Memory Map

The SNES uses a 24bit address bus (000000h-FFFFFFh). These 24bit addresses are
often divided into 8bit bank numbers (00h-FFh) plus 16bit offset (0000h-FFFFh).
Some of these banks are broken into two 32Kbyte halves (0000h-7FFFh=System
Area, 8000h-FFFFh=Cartridge ROM). Moreover, memory is divided into WS1 and WS2
areas, which can be configured to have different waitstates.

**Overall Memory Map**

```text
  Bank    Offset       Content                                      Speed
  00h-3Fh:0000h-7FFFh  System Area (8K WRAM, I/O Ports, Expansion)  see below
  00h-3Fh:8000h-FFFFh  WS1 LoROM (max 2048 Kbytes) (64x32K)         2.68MHz
     (00h:FFE0h-FFFFh) CPU Exception Vectors (Reset,Irq,Nmi,etc.)   2.68MHz
  40h-7Dh:0000h-FFFFh  WS1 HiROM (max 3968 Kbytes) (62x64K)         2.68MHz
  7Eh-7Fh:0000h-FFFFh  WRAM (Work RAM, 128 Kbytes) (2x64K)          2.68MHz
  80h-BFh:0000h-7FFFh  System Area (8K WRAM, I/O Ports, Expansion)  see below
  80h-BFh:8000h-FFFFh  WS2 LoROM (max 2048 Kbytes) (64x32K)         max 3.58MHz
  C0h-FFh:0000h-FFFFh  WS2 HiROM (max 4096 Kbytes) (64x64K)         max 3.58MHz
```

Internal memory regions are WRAM and memory mapped I/O ports.

External memory regions are LoROM, HiROM, and Expansion areas.

Additional memory (not mapped to CPU addresses) (accessible only via I/O):

```text
  OAM          (512+32 bytes) (256+16 words)
  VRAM         (64 Kbytes)    (32 Kwords)
  Palette      (512 bytes)    (256 words)
  Sound RAM    (64 Kbytes)
  Sound ROM    (64 bytes BIOS Boot ROM)
```

**System Area (banks 00h-3Fh and 80h-BFh)**

```text
  Offset       Content                                              Speed
  0000h-1FFFh  Mirror of 7E0000h-7E1FFFh (first 8Kbyte of WRAM)     2.68MHz
  2000h-20FFh  Unused                                               3.58MHz
  2100h-21FFh  I/O Ports (B-Bus)                                    3.58MHz
  2200h-3FFFh  Unused                                               3.58MHz
  4000h-41FFh  I/O Ports (manual joypad access)                     1.78MHz
  4200h-5FFFh  I/O Ports                                            3.58MHz
  6000h-7FFFh  Expansion                                            2.68MHz
```

For details on the separate I/O ports (and Expansion stuff), see:

[SNES I/O Map](#snes-io-map)

**Cartridge ROM Capacity**

The 24bit address bus allows to address 16MB, but wide parts are occupied by
WRAM and I/O mirrors, which leaves only around 11.9MB for cartridge ROM in
WS1/WS2 LoROM/HiROM regions (or more when also using Expansion regions and gaps
in I/O area). In most cartridges, WS1 and WS2 are mirrors of each other, and
most games do use only the LoROM, or only the HiROM areas, resulting in
following capacities:

```text
  LoROM games --> max 2MByte ROM (banks 00h-3Fh, with mirror at 80h-BFh)
  HiROM games --> max 4MByte ROM (banks 40h-7Dh, with mirror at C0h-FFh)
```

There are several ways to overcome that limits: Some LoROM games map additional
"LoROM" banks into HiROM area (BigLoROM), or into WS2 area (SpecialLoROM). Some
HiROM games map additional HiROM banks into WS2 area (ExHiROM). And some
cartridges do use bank switching (eg. SA-1, S-DD1, SPC7110, and X-in-1
multicarts).

**32K LoROM (32K ROM banks with System Area in the same bank)**

ROM is broken into non-continous 32K blocks. The advantage is that one can
access ROM and System Area (I/O ports and WRAM) without needing to change the
CPU's current DB and PB register settings.

**HiROM (plain 64K ROM banks)**

HiROM mapping provides continous ROM addresses, but doesn't include I/O and
WRAM regions "inside" of the ROM-banks.

The upper halves of the 64K-HiROM banks are usually mirrored to the
corresponding 32K-LoROM banks (that is important for mapping the Interrupt and
Reset Vectors from 40FFE0h-40FFFFh to 00FFE0h-00FFFFh).

**Battery-backed SRAM**

Battery-backed SRAM is used for saving game positions in many games. SRAM size
is usually 2Kbyte, 8Kbyte, or 32Kbyte (or more than 32Kbyte in a few games).

There are two basic SRAM mapping schemes, one for LoROM games, and one for
HiROM games:

```text
  HiROM ---> SRAM at 30h-3Fh,B0h-BFh:6000h-7FFFh    ;small 8K SRAM bank(s)
  LoROM ---> SRAM at 70h-7Dh,F0h-FFh:0000h-7FFFh    ;big 32K SRAM bank(s)
```

SRAM is usually also mirrored to both WS1 and WS2 areas. SRAM in bank 30h-3Fh
is often also mirrored to 20h-2Fh (or 10h-1Fh in some cases). SRAM in bank
70h-7Dh is sometimes crippled to 70h-71h or 70h-77h, or extended to 60h-7Dh,
and sometimes also mirrored to offset 8000h-FFFFh.

**A-Bus and B-Bus**

Aside from the 24bit address bus (A-Bus), the SNES is having a second 8bit
address bus (B-bus), used to access certain I/O ports. Both address busses are
sharing the same data bus, but each bus is having its own read and write
signals.

The CPU can access the B-Bus at offset 2100h-21FFh within the System Area (ie.
for CPU accesses, the B-Bus is simply a subset of the A-Bus).

The DMA controller can access both B-Bus and A-Bus at once (ie. it can output
source &amp; destination addresses simultaneously to the two busses, allowing
it to "read-and-write" in a single step, instead of using separate
"read-then-write" steps).

**Bank Switching**

Most SNES games are satisfied with the 24bit address space. Bank switching is
used only in a few games with special chips:

```text
  S-DD1, SA-1, and SPC7110 chips (with mappable 1MByte-banks)
  Satellaview FLASH carts (can enable/disable ROM, PSRAM, FLASH)
  Nintendo Power FLASH carts (can map FLASH and SRAM to desired address)
  Pirate X-in-1 multicart mappers (mappable offset in 256Kbyte units)
  Cheat devices (and X-Band modem) can map their BIOS and can patch ROM bytes
  Copiers can map internal BIOS/DRAM/SRAM and external Cartridge memory
  Hotel Boxes (eg. SFC-Box) can map multiple games/cartridges
```

And, at the APU side, one can enable/disable the 64-byte boot ROM.

<a id="snesmemorycontrol"></a>

### SNES Memory Control

**420Dh - MEMSEL - Memory-2 Waitstate Control (W)**

```text
  7-1   Not used
  0     Access Cycle for Memory-2 Area (0=2.68MHz, 1=3.58MHz) (0 on reset)
```

Memory-2 consists of address 8000h-FFFFh in bank 80h-BFh, and address
0000h-FFFFh in bank C0h-FFh. 3.58MHz high speed memory requires 120ns or faster
ROMs/EPROMs. 2.68MHz memory requires 200ns or faster ROMs/EPROMs.

```text
  2.684658 MHz = 21.47727 MHz / 8     ;same access time as WRAM
  3.579545 MHz = 21.47727 MHz / 6     ;faster access than WRAM
```

Programs that do use the 3.58MHz setting should also indicate this in the
Cartridge header at [FFD5h].Bit4.

[SNES Cartridge ROM Header](60-cartridge-header-and-mapping.md#snes-cartridge-rom-header)

**Forced Blank**

Allows to access video memory at any time. See INIDISP Bit7, Port 2100h.

<a id="snesmemoryworkramaccess"></a>

### SNES Memory Work RAM Access

The SNES includes 128Kbytes of Work RAM, which can be accessed in several ways:

```text
  The whole 128K are at 7E0000h-7FFFFFh.
  The first 8K are also mirrored to xx0000h-xx1FFFh (xx=00h..3Fh and 80h..BFh)
  Moreover (mainly for DMA purposes) it can be accessed via Port 218xh.
```

**2180h - WMDATA - WRAM Data Read/Write (R/W)**

```text
  7-0   Work RAM Data
```

Simply reads or writes the byte at the address in [2181h-2183h], and does then
increment the address by one.

Note: Despite of the fast access time on 2180h reads (faster than
7E0000h-7FFFFFh reads), there is no prefetching involved (reading 2180h always
returns the currently addressed byte, even if one mixes it with writes to 2180h
or to 7E0000h-7FFFFFh).

**2181h - WMADDL - WRAM Address (lower 8bit) (W)**

**2182h - WMADDM - WRAM Address (middle 8bit) (W)**

**2183h - WMADDH - WRAM Address (upper 1bit) (W)**

17bit Address (in Byte-steps) for addressing the 128Kbytes of WRAM via 2180h.

**DMA Notes**

WRAM-to-WRAM DMA isn't possible (neither in A-Bus to B-Bus direction, nor
vice-versa). Externally, the separate address lines are there, but the WRAM
chip is unable to process both at once.

**Timing Notes**

Note that WRAM is accessed at 2.6MHz. Meaning that all variables, stack, and
program code in RAM will be slow. The SNES doesn't include any fast RAM.
However, there are a few tricks to get "3.5MHz RAM":

* Sequential read from WRAM via [2180h] is 3.5MHz fast, and has auto-increment.

* DMA registers at 43x0h-43xBh provide 8x12 bytes of read/write-able "memory".

* External RAM could be mapped to 5000h-5FFFh (but usually it's at slow 6000h).

* External RAM could be mapped to C00000h-FFFFFFh (probably rarely done too).

**Other Notes**

The B-Bus feature with auto-increment is making it fairly easy to boot the SNES
without any ROM/EPROM by simply writing program bytes to WRAM (and mirroring it
to the Program and Reset vector to ROM area):

[SNES Xboo Upload (WRAM Boot)](80-timings-unpredictable-pinouts.md#snes-xboo-upload-wram-boot)

Interestingly, the WRAM-to-ROM Area mirroring seems to be stable even when ROM
Area is set to 3.5MHz Access Time - so it's unclear why Nintendo has restricted
normal WRAM Access to 2.6MHz - maybe some WRAM chips are slower than others, or
maybe they become unstable at certain room temperatures.

<a id="snesmemoryoamaccessspriteattributes"></a>

### SNES Memory OAM Access (Sprite Attributes)

**2102h/2103h - OAMADDL/OAMADDH - OAM Address and Priority Rotation (W)**

```text
  15    OAM Priority Rotation  (0=OBJ #0, 1=OBJ #N) (OBJ with highest priority)
  9-14  Not used
  7-1   OBJ Number #N (for OBJ Priority)   ;\bit7-1 are used for two purposes
  8-0   OAM Address   (for OAM read/write) ;/
```

This register contains of a 9bit Reload value and a 10bit Address register
(plus the priority flag). Writing to 2102h or 2103h does change the lower 8bit
or upper 1bit of the Reload value, and does additionally copy the (whole) 9bit
Reload value to the 10bit Address register (with address Bit0=0 so next access
will be an even address).

Caution: During rendering, the PPU is destroying the Address register (using it
internally for whatever purposes), after rendering (at begin of Vblank, ie. at
begin of line 225/240, but only if not in Forced Blank mode) it reinitializes
the Address from the Reload value; the same reload occurs also when
deactivating forced blank anytime during the first scanline of vblank (ie.
during line 225/240).

**2104h - OAMDATA - OAM Data Write (W)**

**2138h - RDOAM - OAM Data Read (R)**

```text
  1st Access: Lower 8bit (even address)
  2nd Access: Upper 8bit (odd address)
```

Reads and Writes to EVEN and ODD byte-addresses work as follows:

```text
  Write to EVEN address      -->  set OAM_Lsb = Data    ;memorize value
  Write to ODD address<200h  -->  set WORD[addr-1] = Data*256 + OAM_Lsb
  Write to ANY address>1FFh  -->  set BYTE[addr] = Data
  Read from ANY address      -->  return BYTE[addr]
```

The address is automatically incremented after every read or write access.

OAM Size is 220h bytes (addresses 220h..3FFh are mirrors of 200h..21Fh).

**OAM Content**

[SNES PPU Sprites (OBJs)](30-ppu.md#snes-ppu-sprites-objs)

<a id="snesmemoryvramaccesstileandbgmap"></a>

### SNES Memory VRAM Access (Tile and BG Map)

**2115h - VMAIN - VRAM Address Increment Mode (W)**

```text
  7     Increment VRAM Address after accessing High/Low byte (0=Low, 1=High)
  6-4   Not used
  3-2   Address Translation    (0..3 = 0bit/None, 8bit, 9bit, 10bit)
  1-0   Address Increment Step (0..3 = Increment Word-Address by 1,32,128,128)
```

The address translation is intended for bitmap graphics (where one would have
filled the BG Map by increasing Tile numbers), technically it does thrice
left-rotate the lower 8, 9, or 10 bits of the Word-address:

```text
  Translation  Bitmap Type              Port [2116h/17h]    VRAM Word-Address
  8bit rotate  4-color; 1 word/plane    aaaaaaaaYYYxxxxx --> aaaaaaaaxxxxxYYY
  9bit rotate  16-color; 2 words/plane  aaaaaaaYYYxxxxxP --> aaaaaaaxxxxxPYYY
  10bit rotate 256-color; 4 words/plane aaaaaaYYYxxxxxPP --> aaaaaaxxxxxPPYYY
```

Where "aaaaa" would be the normal address MSBs, "YYY" is the Y-index (within a
8x8 tile), "xxxxx" selects one of the 32 tiles per line, "PP" is the bit-plane
index (for BGs with more than one Word per plane). For the intended result
(writing rows of 256 pixels) the Translation should be combined with Increment
Step=1.

For Mode 7 bitmaps one could eventually combine step 32/128 with 8bit/10bit
rotate:

```text
  8bit-rotate/step32   aaaaaaaaXXXxxYYY --> aaaaaaaaxxYYYXXX
  10bit-rotate/step128 aaaaaaXXXxxxxYYY --> aaaaaaxxxxYYYXXX
```

Though the SNES can't access enought VRAM for fullscreen Mode 7 bitmaps.

Step 32 (without translation) is useful for updating BG Map columns (eg. after
horizontal scrolling).

**2116h - VMADDL - VRAM Address (lower 8bit) (W)**

**2117h - VMADDH - VRAM Address (upper 8bit) (W)**

VRAM Address for reading/writing. This is a WORD address (2-byte steps), the
PPU could theoretically address up to 64K-words (128K-bytes), in practice, only
32K-words (64K-bytes) are installed in SNES consoles (VRAM address bit15 is not
connected, so addresses 8000h-FFFFh are mirrors of 0-7FFFh).

After reading/writing VRAM Data, the Word-address can be automatically
incremented by 1,32,128 (depending on the Increment Mode in Port 2115h) (Note:
the Address Translation feature is applied only "temporarily" upon memory
accesses, it doesn't affect the value in Port 2116h-17h).

Writing to 2116h/2117h does prefetch 16bit data from the new address (for later
reading).

**2118h - VMDATAL - VRAM Data Write (lower 8bit) (W)**

**2119h - VMDATAH - VRAM Data Write (upper 8bit) (W)**

Writing to 2118h or 2119h does simply modify the LSB or MSB of the currently
addressed VRAM word (with optional Address Translation applied). Depending on
the Increment Mode the address does (or doesn't) get automatically incremented
after the write.

**2139h - RDVRAML - VRAM Data Read (lower 8bit) (R)**

**213Ah - RDVRAMH - VRAM Data Read (upper 8bit) (R)**

Reading from these registers returns the LSB or MSB of an internal 16bit
prefetch register. Depending on the Increment Mode the address does (or
doesn't) get automatically incremented after the read.

The prefetch register is filled with data from the currently addressed VRAM
word (with optional Address Translation applied) upon two situations:

```text
  Prefetch occurs AFTER changing the VRAM address (by writing 2116h/17h).
  Prefetch occurs BEFORE incrementing the VRAM address (by reading 2139h/3Ah).
```

The "Prefetch BEFORE Increment" effect is some kind of a hardware glitch
(Prefetch AFTER Increment would be more useful). Increment/Prefetch in detail:

```text
  1st  Send a byte from OLD prefetch value to the CPU        ;-this always
  2nd  Load NEW value from OLD address into prefetch register;\these only if
  3rd  Increment address so it becomes the NEW address       ;/increment occurs
```

Increments caused by writes to 2118h/19h don't do any prefetching (the prefetch
register is left totally unchanged by writes).

In practice: After changing the VRAM address (via 2116h/17h), the first
byte/word will be received twice, further values are received from properly
increasing addresses (as a workaround: issue a dummy-read that ignores the 1st
or 2nd value).

**VRAM Content**

[SNES PPU Video Memory (VRAM)](30-ppu.md#snes-ppu-video-memory-vram)

<a id="snesmemorycgramaccesspalettememory"></a>

### SNES Memory CGRAM Access (Palette Memory)

**2121h - CGADD - Palette CGRAM Address (Color Generator Memory) (W)**

Color index (0..255). This is a WORD-address (2-byte steps), allowing to access
256 words (512 bytes). Writing to this register resets the 1st/2nd access
flipflop (for 2122h/213Bh) to 1st access.

**2122h - CGDATA - Palette CGRAM Data Write (W)**

**213Bh - RDCGRAM - Palette CGRAM Data Read (R)**

```text
  1st Access: Lower 8 bits (even address)
  2nd Access: Upper 7 bits (odd address) (upper 1bit = PPU2 open bus)
```

Reads and Writes to EVEN and ODD byte-addresses work as follows:

```text
  Write to EVEN address  -->  set Cgram_Lsb = Data    ;memorize value
  Write to ODD address   -->  set WORD[addr-1] = Data*256 + Cgram_Lsb
  Read from ANY address  -->  return BYTE[addr]
```

The address is automatically incremented after every read or write access.

**CGRAM Content (and CGRAM-less Direct Color mode)**

[SNES PPU Color Palette Memory (CGRAM) and Direct Colors](30-ppu.md#snes-ppu-color-palette-memory-cgram-and-direct-colors)
