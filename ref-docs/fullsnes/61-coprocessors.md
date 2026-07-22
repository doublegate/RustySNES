# Fullsnes — Cartridge Coprocessors (SA-1, Super FX, CX4, DSP, ST018/ARM, OBC1, S-DD1, SPC7110, S-RTC)

[Index](00-index.md) · [« Cartridge Header, PCBs, CIC & Memory Mapping](60-cartridge-header-and-mapping.md) · [Cartridge Add-Ons »](62-cartridge-addons-satellaview-modems.md)

**Sections in this file:**

- [SNES Cart SA-1 (programmable 65C816 CPU) (aka Super Accelerator) (35 games)](#snes-cart-sa-1-programmable-65c816-cpu-aka-super-accelerator-35-games)
- [SNES Cart SA-1 Games](#snes-cart-sa-1-games)
- [SNES Cart SA-1 I/O Map](#snes-cart-sa-1-io-map)
- [SNES Cart SA-1 Interrupt/Control on SNES Side](#snes-cart-sa-1-interruptcontrol-on-snes-side)
- [SNES Cart SA-1 Interrupt/Control on SA-1 Side](#snes-cart-sa-1-interruptcontrol-on-sa-1-side)
- [SNES Cart SA-1 Timer](#snes-cart-sa-1-timer)
- [SNES Cart SA-1 Memory Control](#snes-cart-sa-1-memory-control)
- [SNES Cart SA-1 DMA Transfers](#snes-cart-sa-1-dma-transfers)
- [SNES Cart SA-1 Character Conversion](#snes-cart-sa-1-character-conversion)
- [SNES Cart SA-1 Arithmetic Maths](#snes-cart-sa-1-arithmetic-maths)
- [SNES Cart SA-1 Variable-Length Bit Processing](#snes-cart-sa-1-variable-length-bit-processing)
- [SNES Cart GSU-n (programmable RISC CPU) (aka Super FX/Mario Chip) (10 games)](#snes-cart-gsu-n-programmable-risc-cpu-aka-super-fxmario-chip-10-games)
- [SNES Cart GSU-n List of Games, Chips, and PCB versions](#snes-cart-gsu-n-list-of-games-chips-and-pcb-versions)
- [SNES Cart GSU-n Memory Map](#snes-cart-gsu-n-memory-map)
- [SNES Cart GSU-n I/O Map](#snes-cart-gsu-n-io-map)
- [SNES Cart GSU-n General I/O Ports](#snes-cart-gsu-n-general-io-ports)
- [SNES Cart GSU-n Bitmap I/O Ports](#snes-cart-gsu-n-bitmap-io-ports)
- [SNES Cart GSU-n CPU MOV Opcodes](#snes-cart-gsu-n-cpu-mov-opcodes)
- [SNES Cart GSU-n CPU ALU Opcodes](#snes-cart-gsu-n-cpu-alu-opcodes)
- [SNES Cart GSU-n CPU JMP and Prefix Opcodes](#snes-cart-gsu-n-cpu-jmp-and-prefix-opcodes)
- [SNES Cart GSU-n CPU Pseudo Opcodes](#snes-cart-gsu-n-cpu-pseudo-opcodes)
- [SNES Cart GSU-n CPU Misc](#snes-cart-gsu-n-cpu-misc)
- [SNES Cart GSU-n Code-Cache](#snes-cart-gsu-n-code-cache)
- [SNES Cart GSU-n Pixel-Cache](#snes-cart-gsu-n-pixel-cache)
- [SNES Cart GSU-n Other Caches](#snes-cart-gsu-n-other-caches)
- [SNES Cart Capcom CX4 (programmable RISC CPU) (Mega Man X 2-3) (2 games)](#snes-cart-capcom-cx4-programmable-risc-cpu-mega-man-x-2-3-2-games)
- [SNES Cart Capcom CX4 - I/O Ports](#snes-cart-capcom-cx4-io-ports)
- [SNES Cart Capcom CX4 - Opcodes](#snes-cart-capcom-cx4-opcodes)
- [SNES Cart Capcom CX4 - Functions](#snes-cart-capcom-cx4-functions)
- [SNES Cart DSP-n/ST010/ST011 (pre-programmed NEC uPD77C25 CPU) (23 games)](#snes-cart-dsp-nst010st011-pre-programmed-nec-upd77c25-cpu-23-games)
- [SNES Cart DSP-n/ST010/ST011 - NEC uPD77C25 - Registers & Flags & Overview](#snes-cart-dsp-nst010st011-nec-upd77c25-registers-flags-overview)
- [SNES Cart DSP-n/ST010/ST011 - NEC uPD77C25 - ALU and LD Instructions](#snes-cart-dsp-nst010st011-nec-upd77c25-alu-and-ld-instructions)
- [SNES Cart DSP-n/ST010/ST011 - NEC uPD77C25 - JP Instructions](#snes-cart-dsp-nst010st011-nec-upd77c25-jp-instructions)
- [SNES Cart DSP-n/ST010/ST011 - List of Games using that chips](#snes-cart-dsp-nst010st011-list-of-games-using-that-chips)
- [SNES Cart DSP-n/ST010/ST011 - BIOS Functions](#snes-cart-dsp-nst010st011-bios-functions)
- [SNES Cart Seta ST018 (pre-programmed ARM CPU) (1 game)](#snes-cart-seta-st018-pre-programmed-arm-cpu-1-game)
- [ARM CPU Reference](#arm-cpu-reference)
- [ARM Register Set](#arm-register-set)
- [ARM Flags & Condition Field (cond)](#arm-flags-condition-field-cond)
- [ARM 26bit Memory Interface](#arm-26bit-memory-interface)
- [ARM Exceptions](#arm-exceptions)
- [ARM Instruction Summary](#arm-instruction-summary)
- [ARM Opcodes: Branch and Branch with Link (B, BL, SWI)](#arm-opcodes-branch-and-branch-with-link-b-bl-swi)
- [ARM Opcodes: Data Processing (ALU)](#arm-opcodes-data-processing-alu)
- [ARM Opcodes: PSR Transfer (MRS, MSR)](#arm-opcodes-psr-transfer-mrs-msr)
- [ARM Opcodes: Multiply and Multiply-Accumulate (MUL, MLA)](#arm-opcodes-multiply-and-multiply-accumulate-mul-mla)
- [ARM Opcodes: Memory: Block Data Transfer (LDM, STM)](#arm-opcodes-memory-block-data-transfer-ldm-stm)
- [ARM Opcodes: Memory: Single Data Transfer (LDR, STR)](#arm-opcodes-memory-single-data-transfer-ldr-str)
- [ARM Opcodes: Memory: Single Data Swap (SWP)](#arm-opcodes-memory-single-data-swap-swp)
- [ARM Opcodes: Coprocessor Instructions (MRC/MCR, LDC/STC, CDP)](#arm-opcodes-coprocessor-instructions-mrcmcr-ldcstc-cdp)
- [ARM Pseudo Instructions and Directives](#arm-pseudo-instructions-and-directives)
- [ARM Instruction Cycle Times](#arm-instruction-cycle-times)
- [ARM Versions](#arm-versions)
- [SNES Cart OBC1 (OBJ Controller) (1 game)](#snes-cart-obc1-obj-controller-1-game)
- [SNES Cart S-DD1 (Data Decompressor) (2 games)](#snes-cart-s-dd1-data-decompressor-2-games)
- [SNES Cart S-DD1 Decompression Algorithm](#snes-cart-s-dd1-decompression-algorithm)
- [SNES Cart SPC7110 (Data Decompressor) (3 games)](#snes-cart-spc7110-data-decompressor-3-games)
- [SNES Cart SPC7110 Memory and I/O Map](#snes-cart-spc7110-memory-and-io-map)
- [SNES Cart SPC7110 Decompression I/O Ports](#snes-cart-spc7110-decompression-io-ports)
- [SNES Cart SPC7110 Direct Data ROM Access](#snes-cart-spc7110-direct-data-rom-access)
- [SNES Cart SPC7110 Multiply/Divide Unit](#snes-cart-spc7110-multiplydivide-unit)
- [SNES Cart SPC7110 with RTC-4513 Real Time Clock (1 game)](#snes-cart-spc7110-with-rtc-4513-real-time-clock-1-game)
- [SNES Cart SPC7110 Decompression Algorithm](#snes-cart-spc7110-decompression-algorithm)
- [SNES Cart SPC7110 Notes](#snes-cart-spc7110-notes)
- [SNES Cart Unlicensed Variants](#snes-cart-unlicensed-variants)
- [SNES Cart S-RTC (Realtime Clock) (1 game)](#snes-cart-s-rtc-realtime-clock-1-game)

---

<a id="snescartsa1programmable65c816cpuakasuperaccelerator35games"></a>

## SNES Cart SA-1 (programmable 65C816 CPU) (aka Super Accelerator) (35 games)

[SNES Cart SA-1 Games](#snes-cart-sa-1-games)

[SNES Cart SA-1 I/O Map](#snes-cart-sa-1-io-map)

[SNES Cart SA-1 Interrupt/Control on SNES Side](#snes-cart-sa-1-interruptcontrol-on-snes-side)

[SNES Cart SA-1 Interrupt/Control on SA-1 Side](#snes-cart-sa-1-interruptcontrol-on-sa-1-side)

[SNES Cart SA-1 Timer](#snes-cart-sa-1-timer)

[SNES Cart SA-1 Memory Control](#snes-cart-sa-1-memory-control)

[SNES Cart SA-1 DMA Transfers](#snes-cart-sa-1-dma-transfers)

[SNES Cart SA-1 Character Conversion](#snes-cart-sa-1-character-conversion)

[SNES Cart SA-1 Arithmetic Maths](#snes-cart-sa-1-arithmetic-maths)

[SNES Cart SA-1 Variable-Length Bit Processing](#snes-cart-sa-1-variable-length-bit-processing)

[SNES Pinouts SA1 Chip](80-timings-unpredictable-pinouts.md#snes-pinouts-sa1-chip)

**Memory Map (SNES Side)**

```text
  00h-3Fh/80h-BFh:2200h-23FFh  I/O Ports
  00h-3Fh/80h-BFh:3000h-37FFh  I-RAM (2Kbytes, on-chip, 10MHz fast RAM)
  00h-3Fh/80h-BFh:6000h-7FFFh  One mappable 8Kbyte BW-RAM block
  00h-3Fh/80h-BFh:8000h-FFFFh  Four mappable 1MByte LoROM blocks (max 8Mbyte)
  40h-4Fh:0000h-FFFFh          Entire 256Kbyte BW-RAM (mirrors in 44h-4Fh)
  C0h-FFh:0000h-FFFFh          Four mappable 1MByte HiROM blocks (max 8Mbyte)
```

The SA-1 supports both LoROM and HiROM mappings (eg. LoROM banks 00h-01h mirror
to HiROM bank 40h). Default exception vectors (and cartridge header) are always
in LoROM bank 00h (ie. at ROM offset 7Fxxh).

**Memory Map (SA-1 Side)**

Same as on SNES Side (of course without access to SNES internal WRAM and I/O
ports), plus following additional areas:

```text
  00h-3Fh/80h-BFh:0000h-07FFh  I-RAM (at both 0000h-07FFh and 3000h-37FFh)
  60h-6Fh:0000h-FFFFh          BW-RAM mapped as 2bit or 4bit pixel buffer
```

Some other differences to SNES Side are: I/O Ports are different, on SA-1 side,
the mappable BW-RAM area (at 6000h-7FFFh) can be also assigned as 2bit/4bit
pixel buffer (on SNES Side it's always normal 8bit memory).

**Misc**

65C816 CPU at 10.74MHz

```text
  2Kbytes internal I-RAM (work ram/stack) (optionally battery backed)
  Optional external backup/work BW-RAM up to 2MByte (or rather only 2Mbit?)
  Addressable ROM up to 8MByte (64MBits)
```

The SA-1 CPU can access memory at 10.74MHz rate (or less, if the SNES does
simultaneouly access cartridge memory).

The SNES CPU can access memory at 2.68MHz rate (or 3.5MHz, but that mode may
not be used in combination with the SA-1).

When interrupts are disabled (in CIE/SIE), then it sounds as if the interrupt
flags still do get set?

"BW-RAM cannot be used during character conversion DMA."

IRQ/NMI/Reset vectors can be mapped. Other vectors (BRK/COP etc) are always
taken from ROM (for BOTH CPUs).

```text
    XXX pg 62..66 timings
 ok XXX pg 67..78 char/bitmap
 ok XXX pg 79..81 arit
    XXX pg 82..86 var-len
 ok XXX pg 87..90 dma
```

**SA-1 Pinouts**

```text
  1-126  Unknown
  127    PAL/NTSC (for CIC mode and/or HV-timer?)
  128    Unknown
```

**SA-1 PCBs**

```text
  BSC-1L3B-01    NTSC SRAM Battery FLASH-Slot (Itoi Shig. no Bass Tsuri No.1)
  SHVC-1L0N3S-20 NTSC SRAM NoBattery (Dragon Ball Z Hyper Dimension)
  SHVC-1L3B-11   NTSC SRAM Battery
  SHVC-1L5B-10   NTSC SRAM Battery
  SHVC-1L5B-11   NTSC SRAM Battery
  SHVC-1L8B-10   NTSC SRAM Battery
  SNSP-1L0N3S-01 PAL  SRAM NoBattery (Dragon Ball Z Hyper Dimension)
  SNSP-1L3B-20   PAL  SRAM Battery
```

The battery can be wired to I-RAM (on-chip SA-1 memory) or BW-RAM (aka SRAM) or
both; unknown how it is wired in practice (probably to BW-RAM?).

**Chipset/Components**

```text
  U1  44pin  ROM (probably with full 16bit databus connected)
  U2  28pin  SRAM (LH52A64N-YL or LH52256ANZ or 32pin LH52A512NF)
  U3  128pin SA1 (SA1 RF5A123)
  U4  8pin   Battery controller MM1026AF  ;\only if PCB does include a battery
  BATT 2pin  CR2032                       ;/
  CN1 62pin  SNES cartridge edge-connector
  CN2 62pin  Satellaview FLASH cartridge slot  ;-only on BSC-boards
```

<a id="snescartsa1games"></a>

## SNES Cart SA-1 Games

SA1     - 128pin - Super Accelerator (book2) (10.74MHz 65C816 CPU)

Used by 35 games:

```text
 #Asahi Shinbun Rensai Kato Ichi-Ni-San Kudan Shogi Shingiru (1995) Varie (JP)
  Daisenryaku Expert WWII: War in Europe (1996) SystemSoftAlpha/ASCII Corp (JP)
  Derby Jockey 2 (1995) Muse Soft/Asmik (JP)
  Dragon Ball Z: Hyper Dimension (1996) TOSE/Bandai (JP) (EU)
 #Habu Meijin no Omoshiro Syouhi -Unverified (19xx) Hiroshi/etc. (JP)
  Itoi Shigesato no Bass Tsuri No. 1 (1997) HAL Laboratory/Nintendo (JP)
  J. League '96 Dream Stadium (1996) Hudson Soft (JP)
  Jikkyou Oshaberi Parodius (1995) Konami (JP)
  Jumpin' Derby (1996) Naxat Soft (JP)
 #Kakinoki Shogi (1995) ASCII Corporation (JP)
  Kirby Super Star (1996) HAL Laboratory/Nintendo (NA) (JP) (EU)
  Kirby's Dream Land 3 (1997) HAL Laboratory/Nintendo (NA) (JP)
  Marvelous: Mouhitotsu no Takarajima (1996) Nintendo/R&D2 (JP)
  Masoukishin: Super Robot Wars Gaiden: Lord of Elemental (19xx) Banpresto (JP)
  Masters New: Haruka Naru Augusta 3 (1995) T&E Soft (JP)
  Mini Yonku/4WD Shining Scorpion - Let's & Go!! (1996) KID/ASCII Corp (JP)
  Pachi Slot Monogatari PAL Kogyo Special -Unverified (1995) PAL/KSS (JP)
  Pebble Beach no Hotou: New Tournament Edition (1996) T&E Soft (JP)
  PGA European Tour (1996) Halestorm/THQ/Black Pearl Software (NA)
  PGA Tour '96 (1995) Black Pearl Software/Electronic Arts (NA)
  Power Rangers Zeo: Battle Racers (1996) Natsume/Bandai (NA)
 #Pro Kishi Simulation Kishi No Hanamichi (1996) Atlus (JP)
 xRin Kaihou 9 Dan No Igo Taidou -Unverified (1996) .. (JP)
  SD F-1 Grand Prix (and "Sample" version) (1995) Video System (JP)
  SD Gundam G NEXT (1995) BEC/Bandai (JP)
 #Shin Shogi/Syogi Club (1995) Hect/Natsu (JP)
 #Shogi Saikyou (1995) Magical Company (JP) (unverified?)
 #Shogi Saikyou 2 (1996) Magical Company (JP)
 #Shougi Mahjong (1995) Varie Corp (JP)
  Super Bomberman Panic Bomber World (1995) Hudson Soft (JP)
  Super Mario RPG: Legend of the Seven Stars (1996) Square/Nintendo (NA) (JP)
  Super Robot T.G.: The Lord Of Elemental (?) (1996) Winkysoft/Banpresto (JP)
 #Super Shogi 3 Kitaihei -Unverified (1995) I'Max (JP)
 xTaikyoku-Igo Idaten -Unverified (1995) BPS (JP)
 xTakemiya Masaki Kudan No Igo Taisyou -Unverified (1995) KSS (JP)
```

The nine Shogi/Shougi/Syouhi/Kishi/Syogi titles are japanese Chess games, the
three Igo titles are Go games; that 12 titles are mainly using the SA-1 CPU for
calculating moves, without doing any impressive things with the SA-1 I/O ports.

<a id="snescartsa1iomap"></a>

## SNES Cart SA-1 I/O Map

**SA-1 I/O Map (Write Only Registers)**

```text
  Port  Side  Name  Reset Expl.
  2200h SNES  CCNT  20h   SA-1 CPU Control (W)
  2201h SNES  SIE   00h   SNES CPU Int Enable (W)
  2202h SNES  SIC   00h   SNES CPU Int Clear  (W)
  2203h SNES  CRV   -     SA-1 CPU Reset Vector Lsb (W)
  2204h SNES  CRV   -     SA-1 CPU Reset Vector Msb (W)
  2205h SNES  CNV   -     SA-1 CPU NMI Vector Lsb (W)
  2206h SNES  CNV   -     SA-1 CPU NMI Vector Msb (W)
  2207h SNES  CIV   -     SA-1 CPU IRQ Vector Lsb (W)
  2208h SNES  CIV   -     SA-1 CPU IRQ Vector Msb (W)
  2209h SA-1  SCNT  00h   SNES CPU Control (W)
  220Ah SA-1  CIE   00h   SA-1 CPU Int Enable (W)
  220Bh SA-1  CIC   00h   SA-1 CPU Int Clear  (W)
  220Ch SA-1  SNV   -     SNES CPU NMI Vector Lsb (W)
  220Dh SA-1  SNV   -     SNES CPU NMI Vector Msb (W)
  220Eh SA-1  SIV   -     SNES CPU IRQ Vector Lsb (W)
  220Fh SA-1  SIV   -     SNES CPU IRQ Vector Msb (W)
  2210h SA-1  TMC   00h   H/V Timer Control (W)
  2211h SA-1  CTR   -     SA-1 CPU Timer Restart (W)
  2212h SA-1  HCNT  -     Set H-Count Lsb (W)
  2213h SA-1  HCNT  -     Set H-Count Msb (W)
  2214h SA-1  VCNT  -     Set V-Count Lsb (W)
  2215h SA-1  VCNT  -     Set V-Count Msb (W)
  2216h -     -     -     -
  2220h SNES  CXB   00h   MMC Bank C - Hirom C0h-CFh / LoRom 00h-1Fh (W)
  2221h SNES  DXB   01h   MMC Bank D - Hirom D0h-DFh / LoRom 20h-3Fh (W)
  2222h SNES  EXB   02h   MMC Bank E - Hirom E0h-EFh / LoRom 80h-9Fh (W)
  2223h SNES  FXB   03h   MMC Bank F - Hirom F0h-FFh / LoRom A0h-BFh (W)
  2224h SNES  BMAPS 00h   SNES CPU BW-RAM Mapping to 6000h-7FFFh (W)
  2225h SA-1  BMAP  00h   SA-1 CPU BW-RAM Mapping to 6000h-7FFFh (W)
  2226h SNES  SBWE  00h   SNES CPU BW-RAM Write Enable (W)
  2227h SA-1  CBWE  00h   SA-1 CPU BW-RAM Write Enable (W)
  2228h SNES  BWPA  FFh   BW-RAM Write-Protected Area (W)
  2229h SNES  SIWP  00h   SNES I-RAM Write-Protection (W)
  222Ah SA-1  CIWP  00h   SA-1 I-RAM Write-Protection (W)
  222Bh -     -     -     -
  2230h SA-1  DCNT  00h   DMA Control (W)
  2231h Both  CDMA  00h   Character Conversion DMA Parameters (W)
  2232h Both  SDA   -     DMA Source Device Start Address Lsb (W)
  2233h Both  SDA   -     DMA Source Device Start Address Mid (W)
  2234h Both  SDA   -     DMA Source Device Start Address Msb (W)
  2235h Both  DDA   -     DMA Dest Device Start Address Lsb (W)
  2236h Both  DDA   -     DMA Dest Device Start Address Mid (Start/I-RAM) (W)
  2237h Both  DDA   -     DMA Dest Device Start Address Msb (Start/BW-RAM)(W)
  2238h SA-1  DTC   -     DMA Terminal Counter Lsb (W)
  2239h SA-1  DTC   -     DMA Terminal Counter Msb (W)
  223Ah -     -     -     -
  223Fh SA-1  BBF   00h   BW-RAM Bit Map Format for 600000h-6FFFFFh (W)
  224xh SA-1  BRF   -     Bit Map Register File (2240h..224Fh) (W)
  2250h SA-1  MCNT  00h   Arithmetic Control (W)
  2251h SA-1  MA    -     Arithmetic Param A Lsb (Multiplicand/Dividend) (W)
  2252h SA-1  MA    -     Arithmetic Param A Msb (Multiplicand/Dividend) (W)
  2253h SA-1  MB    -     Arithmetic Param B Lsb (Multiplier/Divisor) (W)
  2254h SA-1  MB    -     Arithmetic Param B Msb (Multiplier/Divisor)/Start (W)
  2255h -     -     -     -
  2258h SA-1  VBD   -     Variable-Length Bit Processing (W)
  2259h SA-1  VDA   -     Var-Length Bit Game Pak ROM Start Address Lsb (W)
  225Ah SA-1  VDA   -     Var-Length Bit Game Pak ROM Start Address Mid (W)
  225Bh SA-1  VDA   -     Var-Length Bit Game Pak ROM Start Address Msb & Kick
  225Ch -     -     -     -
  2261h -     -     -     Unknown/Undocumented (Jumpin Derby writes 00h)
  2262h -     -     -     Unknown/Undocumented (Super Bomberman writes 00h)
```

**SA-1 I/O Map (Read Only Registers)**

```text
  Port  Side  Name  Reset Expl.
  2300h SNES  SFR   SNES CPU Flag Read (R)
  2301h SA-1  CFR   SA-1 CPU Flag Read (R)
  2302h SA-1  HCR   H-Count Read Lsb / Do Latching (R)
  2303h SA-1  HCR   H-Count Read Msb (R)
  2304h SA-1  VCR   V-Count Read Lsb (R)
  2305h SA-1  VCR   V-Count Read Msb (R)
  2306h SA-1  MR    Arithmetic Result, bit0-7   (Sum/Product/Quotient) (R)
  2307h SA-1  MR    Arithmetic Result, bit8-15  (Sum/Product/Quotient) (R)
  2308h SA-1  MR    Arithmetic Result, bit16-23 (Sum/Product/Remainder) (R)
  2309h SA-1  MR    Arithmetic Result, bit24-31 (Sum/Product/Remainder) (R)
  230Ah SA-1  MR    Arithmetic Result, bit32-39 (Sum) (R)
  230Bh SA-1  OF    Arithmetic Overflow Flag (R)
  230Ch SA-1  VDP   Variable-Length Data Read Port Lsb (R)
  230Dh SA-1  VDP   Variable-Length Data Read Port Msb (R)
  230Eh SNES  VC    Version Code Register (R)
```

**Reset**

Port 2200h = 20h. Port 2228h = FFh. Ports 2220h-2223h = 00h,01h,02h,03h. Ports
2201h-2202h, 2209h-220Bh, 2210h, 2224h-2227h, 2229h-222Ah, 2230h-2231h, 223Fh,
2250h = 00h. Ports 2203h-2208h, 220Ch-220Fh, 2211h-2215h, 2232h-2239h,
2240h-224Fh, 2251h-2254h, 2258h-225Bh = N/A.

<a id="snescartsa1interruptcontrolonsnesside"></a>

## SNES Cart SA-1 Interrupt/Control on SNES Side

**2200h SNES CCNT - SA-1 CPU Control (W)**

```text
  0-3 Message from SNES to SA-1 (4bit value)
  4   NMI from SNES to SA-1   (0=No Change?, 1=Interrupt)
  5   Reset from SNES to SA-1 (0=No Reset, 1=Reset)
  6   Wait from SNES to SA-1  (0=No Wait, 1=Wait)
  7   IRQ from SNES to SA-1   (0=No Change?, 1=Interrupt)
```

Unknown if Wait freezes the whole SA1 (CPU, plus Timer and DMA?).

Unknown if Reset resets any I/O Ports (such like DMA or interrupts) or if it
does only reset the CPU?

**2201h SNES SIE - SNES CPU Int Enable (W)**

```text
  0-4 Not used (should be 0)
  5   IRQ Enable (Character conversion DMA) (0=Disable, 1=Enable)
  6   Not used (should be 0)
  7   IRQ Enable (from SA-1) (0=Disable, 1=Enable)
```

**2202h SNES SIC - SNES CPU Int Clear (W)**

```text
  0-4 Not used (should be 0)
  5   IRQ Acknowledge (Character conversion DMA) (0=No change, 1=Clear)
  6   Not used (should be 0)
  7   IRQ Acknowledge (from SA-1) (0=No change, 1=Clear)
```

**2203h SNES CRV - SA-1 CPU Reset Vector Lsb (W)**

**2204h SNES CRV - SA-1 CPU Reset Vector Msb (W)**

**2205h SNES CNV - SA-1 CPU NMI Vector Lsb (W)**

**2206h SNES CNV - SA-1 CPU NMI Vector Msb (W)**

**2207h SNES CIV - SA-1 CPU IRQ Vector Lsb (W)**

**2208h SNES CIV - SA-1 CPU IRQ Vector Msb (W)**

Exception Vectors on SA-1 side (these are ALWAYS replacing the normal vectors
in ROM).

**2300h SNES SFR - SNES CPU Flag Read (R)**

```text
  0-3 Message from SA-1 to SNES (4bit value)          (same as 2209h.Bit0-3)
  4   NMI Vector for SNES (0=ROM FFExh, 1=Port 220Ch) (same as 2209h.Bit4)
  5   IRQ from Character Conversion DMA (0=None, 1=Interrupt) (ready-to-do-DMA)
  6   IRQ Vector for SNES (0=ROM FFExh, 1=Port 220Eh) (same as 2209h.Bit6)
  7   IRQ from SA-1 to SNES   (0=None, 1=Interrupt) (triggered by 2209h.Bit7)
```

Bit0-3,4,6 are same as in Port 2209h. Bit5 is set via ..DMA..? Bit7 is set via
Port 2209h. Bit5,7 can be cleared via Port 2202h.

**230Eh SNES VC - Version Code Register (R)**

```text
  0-7  SA-1 Chip Version
```

Existing value(s) are unknown. There seems to be only one chip version (labeled
SA-1 RF5A123, used for both PAL and NTSC). The "VC" register isn't read by any
games (except, accidently, by a bugged memcopy function at 059E92h in Derby
Jockey 2).

<a id="snescartsa1interruptcontrolonsa1side"></a>

## SNES Cart SA-1 Interrupt/Control on SA-1 Side

**2209h SA-1 SCNT - SNES CPU Control (W)**

```text
  0-3 Message from SA-1 to SNES (4bit value)
  4   NMI Vector for SNES (0=ROM FFEAh, 1=Port 220Ch)
  5   Not used (should be 0)
  6   IRQ Vector for SNES (0=ROM FFEEh, 1=Port 220Eh)
  7   IRQ from SA-1 to SNES   (0=No Change?, 1=Interrupt)
```

**220Ah SA-1 CIE - SA-1 CPU Int Enable (W)**

```text
  0-3 Not used (should be 0)
  4   NMI Enable (from SNES)  (0=Disable, 1=Enable)
  5   IRQ Enable (from DMA)   (0=Disable, 1=Enable)
  6   IRQ Enable (from Timer) (0=Disable, 1=Enable)
  7   IRQ Enable (from SNES)  (0=Disable, 1=Enable)
```

**220Bh SA-1 CIC - SA-1 CPU Int Clear (W)**

```text
  0-3 Not used (should be 0)
  4   NMI Acknowledge (from SNES)  (0=No change, 1=Clear)
  5   IRQ Acknowledge (from DMA)   (0=No change, 1=Clear)
  6   IRQ Acknowledge (from Timer) (0=No change, 1=Clear)
  7   IRQ Acknowledge (from SNES)  (0=No change, 1=Clear)
```

**220Ch SA-1 SNV - SNES CPU NMI Vector Lsb (W)**

**220Dh SA-1 SNV - SNES CPU NMI Vector Msb (W)**

**220Eh SA-1 SIV - SNES CPU IRQ Vector Lsb (W)**

**220Fh SA-1 SIV - SNES CPU IRQ Vector Msb (W)**

Exception Vectors on SNES side (these are optionally replacing the normal
vectors in ROM; depending on bits in Port 2209h; the "I/O" vectors are used
only by Jumpin Derby, all other games are using the normal ROM vectors).

**2301h SA-1 CFR - SA-1 CPU Flag Read (R)**

```text
  0-3 Message from SNES to SA-1 (4bit value)       (same as 2200h.Bit0-3)
  4   NMI from SNES to SA-1   (0=No, 1=Interrupt)  (triggered by 2200h.Bit4)
  5   IRQ from DMA to SA-1    (0=No, 1=Interrupt)  (triggered by DMA-finished)
  6   IRQ from Timer to SA-1  (0=No, 1=Interrupt)  (triggered by Timer)
  7   IRQ from SNES to SA-1   (0=No, 1=Interrupt)  (triggered by 2200h.Bit7)
```

<a id="snescartsa1timer"></a>

## SNES Cart SA-1 Timer

**2210h SA-1 TMC - H/V Timer Control (W)**

```text
  0   HEN             ;\Enables Interrupt or so ?
  1   VEN             ;/
  2-6 Not used (should be 0)
  7   Timer Mode (0=HV Timer, 1=Linear Timer)
```

**2211h SA-1 CTR - SA-1 CPU Timer Restart (W)**

```text
  0-7 Don't care (writing any value restarts the timer at 0)
```

**2212h SA-1 HCNT - Set H-Count Lsb (W)**

**2213h SA-1 HCNT - Set H-Count Msb (W)**

```text
  0-8  H-Counter (9bit)
  9-15 Not used (should be 0)
```

Ranges from 0-340 (in HV mode), or 0-511 (in Linear mode).

**2214h SA-1 VCNT - Set V-Count Lsb (W)**

**2215h SA-1 VCNT - Set V-Count Msb (W)**

```text
  0-8  V-Counter (9bit)
  9-15 Not used (should be 0)
```

Ranges from 0-261 (in HV/NTSC mode), 0-311 (in HV/PAL mode), or 0-511 (in
Linear mode). The PAL/NTSC selection is probably done by a soldering point on
the PCB (which is probably also used for switching the built-in CIC to PAL/NTSC
mode).

**2302h SA-1 HCR - H-Count Read Lsb / Do Latching (R)**

**2303h SA-1 HCR - H-Count Read Msb (R)**

**2304h SA-1 VCR - V-Count Read Lsb (R)**

**2305h SA-1 VCR - V-Count Read Msb (R)**

Reading from 2302h automatically latches the other HV-Counter bits to
2303h-2305h.

**Notes**

In HV-mode, the timer clock is obviously equivalent to the dotclock (four 21MHz
master cycles per dot). The time clock in linear mode is unknown (probably same
as in HV-mode).

H-counter has 341 dots (one more as in SNES, but without long dots). Unknown if
the short-scanline (in each 2nd NTSC non-interlaced frame) is reproduced (if it
isn't, then one must periodically reset the timer in order to keep it in sync
with the PPU). There is no provision for interlaced video timings.

The meaning of Port 2212h-2215h is totally unknown (according to existing specs
it &lt;sounds&gt; as if they do set the &lt;current&gt; counter value - though
alltogether it'd be more likely that they do contain &lt;compare&gt; values).

Unknown what happens when setting both HEN and VEN (probably IRQ triggers only
if &lt;both&gt; H+V do match, ie. similar as for the normal SNES timers).

<a id="snescartsa1memorycontrol"></a>

## SNES Cart SA-1 Memory Control

**2220h SNES CXB - Set Super MMC Bank C - Hirom C0h-CFh / LoRom 00h-1Fh (W)**

**2221h SNES DXB - Set Super MMC Bank D - Hirom D0h-DFh / LoRom 20h-3Fh (W)**

**2222h SNES EXB - Set Super MMC Bank E - Hirom E0h-EFh / LoRom 80h-9Fh (W)**

**2223h SNES FXB - Set Super MMC Bank F - Hirom F0h-FFh / LoRom A0h-BFh (W)**

```text
  0-2  Select 1Mbyte ROM-Bank (0..7)
  3-6  Not used (should be 0)
  7    Map 1Mbyte ROM-Bank (0=To HiRom, 1=To LoRom and HiRom)
```

If LoRom mapping is disabled (bit7=0), then first 2 MByte of ROM are mapped to
00h-3Fh, and next 2 MByte to 80h-BFh. The registers do affect both SNES and
SA-1 mapping.

**2224h SNES BMAPS - SNES CPU BW-RAM Mapping to 6000h-7FFFh (W)**

```text
  0-4  Select 8Kbyte BW-RAM Block for mapping to 6000h-7FFFh (0..31)
  5-7  Not used (should be 0)
```

BW-RAM is always mapped to bank 40h-43h (max 256 Kbytes).

This register allows to map an 8Kbyte chunk to offset 6000h-7FFFh in bank 0-3Fh
and 80h-BFh.

**2225h SA-1 BMAP - SA-1 CPU BW-RAM Mapping to 6000h-7FFFh (W)**

```text
  0-6  Select 8Kbyte BW-RAM Block for mapping to 6000h-7FFFh (0..31 or 0..127)
  7    Select source (0=Normal/Bank 40h..43h, 1=Bitmap/Bank 60h..6Fh)
```

**223Fh SA-1 BBF - BW-RAM Bit Map Format for 600000h-6FFFFFh (W)**

```text
  0-6 Not used (should be "..") (whatever ".." means, maybe "0"?)
  7   Format (0=4bit, 1=2bit)
```

"BW-RAM bitmap logical space format setting from perspective of the SA-1 CPU"

```text
  600000h.Bit0-1 or Bit0-3 mirrors to 400000h.Bit0-1 or 400000h.Bit0-3
  600001h.Bit0-1 or Bit0-3 mirrors to 400000h.Bit2-3 or 400000h.Bit4-7
  600002h.Bit0-1 or Bit0-3 mirrors to 400000h.Bit4-5 or 400001h.Bit0-3
  600003h.Bit0-1 or Bit0-3 mirrors to 400000h.Bit6-7 or 400001h.Bit4-7
  etc.
```

Note that the LSBs in the packed-area contain the left-most pixel (not the
right-most one). The MSBs in the unpacked area are "ignored" (this is obvious
in case of writing; for reading it's unknown what it means - are reads
supported at all, and if so, do they return zero's or garbage in MSBs?)

**2226h SNES SBWE - SNES CPU BW-RAM Write Enable (W)**

**2227h SA-1 SBWE - SA-1 CPU BW-RAM Write Enable (W)**

```text
  0-6  Not used (should be 0)
  7    Write Enable BW-RAM (0=Protect, 1=Write Enable)
```

**2228h SNES BWPA - BW-RAM Write-Protected Area (W)**

```text
  0-3  Select size of Write-Protected Area ("256 SHL N" bytes)
  4-7  Not used (should be 0)
```

Selects how many bytes (originated at 400000h) shall be write protected.

It isn't possible to set the size to "none" (min is 256 bytes), though, one can
probably completely disable the protection via ports 2226h/2227h?

**2229h SNES SIWP - SNES I-RAM Write-Protection (W)**

**222Ah SA-1 CIWP - SA-1 I-RAM Write-Protection (W)**

```text
  0-7  Write enable flags for eight 256-byte chunks (0=Protect, 1=Write Enable)
```

Bit0 for I-RAM 3000h..30FFh, bit1 for 3100h..31FFh, etc. bit7 for 3700h..37FFh.

<a id="snescartsa1dmatransfers"></a>

## SNES Cart SA-1 DMA Transfers

**2230h SA-1 DCNT - DMA Control (W)**

```text
  0-1 DMA Source Device      (0=ROM, 1=BW-RAM, 2=I-RAM, 3=Reserved);\for
  2   DMA Destination Device (0=I-RAM, 1=BW-RAM)                   ;/Normal DMA
  3   Not used (should be 0)
  4   DMA Char Conversion Type (0=Type 2/Semi-Automatic, 1=Type 1/Automatic)
  5   DMA Char Conversion Enable (0=Normal DMA, 1=Character Conversion DMA)
  6   DMA Priority (0=SA-1 CPU Priority, 1=DMA Priority) ;<-- for Normal DMA
  7   DMA Enable (0=Disable, 1=Enable... and Clear Parameters?)
```

Bit6 is only valid for Normal DMA between BW-RAM and I-RAM. Source and
Destination may not be the same devices (ie. no I-RAM to I-RAM, or BW-RAM to
BW-RAM).

**2231h Both CDMA - Character Conversion DMA Parameters (W)**

```text
  0-1 Color Depth (0=8bit, 1=4bit, 2=2bit, 3=Reserved)
  2-4 Virtual VRAM Width (0..5 = 1,2,4,8,16,32 characters) (6..7=Reserved)
  5-6 Not used (should be 0)
  7   Terminate Character Conversion 1 (0=No change, 1=Terminate DMA)
```

**2232h Both SDA - DMA Source Device Start Address Lsb (W)**

**2233h Both SDA - DMA Source Device Start Address Mid (W)**

**2234h Both SDA - DMA Source Device Start Address Msb (W)**

```text
  0-23  24bit Memory Address (translated to 23bit ROM Offset via 2220h..2223h)
  0-17  18bit BW-RAM Offset
  0-10  11bit I-RAM Offset
```

Used bits are 24bit/18bit/11bit for ROM/BW-RAM/I-RAM.

**2235h Both DDA - DMA Destination Device Start Address Lsb (W)**

**2236h Both DDA - DMA Destination Device Start Address Mid (Start/I-RAM) (W)**

**2237h Both DDA - DMA Destination Device Start Address Msb (Start/BW-RAM)(W)**

```text
  0-17  BW-RAM Offset (transfer starts after writing 2237h)
  0-10  I-RAM Offset  (transfer starts after writing 2236h) (2237h is unused)
```

**2238h SA-1 DTC - DMA Terminal Counter Lsb (W)**

**2239h SA-1 DTC - DMA Terminal Counter Msb (W)**

```text
  0-15  DMA Transfer Length in bytes (1..65535) (0=Reserved/unknown)
```

DTC is used only for Normal DMA (whilst Character Conversion DMA lasts endless;
for Type 1: as long as SNES reads "BW-RAM" / until it sets 2231h.Bit7, for Type
2: as long as SA-1 writes BRF / until it clears 2230h.Bit0).

**224xh SA-1 BRF - Bit Map Register File (2240h..224Fh) (W)**

These 16 registers can hold two 8 pixel rows (with 2bit/4bit/8bit per pixel).

```text
  0-1  2bit pixel (bit 2-7=unused)
  0-3  4bit pixel (bit 4-7=unused)
  0-7  8bit pixel
```

Used only for (semi-automatic) Character Conversion Type 2, where the "DMA"
source data is to be written pixel-by-pixel to these registers; writing to one
8 pixel row can be done while transferring the other row to the SNES.

**Normal DMA (memory transfer within cartridge memory)**

```text
  ROM    --> I-RAM     10.74MHz
  ROM    --> BW-RAM    5.37MHz
  BW-RAM --> I-RAM     5.37MHz
  I-RAM  --> BW-RAM    5.37MHz
```

For normal DMA:

```text
  Set DCNT (select source/dest/prio/enable)
  Set SDA (set source offset)
  Set DTC (set transfer length)
  Set DDA (set destination offset, and start transfer)
  If desired, wait for CFR.Bit5 (DMA completion interrupt)
```

Normal DMA is used by J. League '96, Jumpin Derby, Marvelous. For ROM, SDA
should be usually C00000h and up (HiROM mapping); Jumpin Derby is
unconventionally using SDA at 2x8xxxh and up (LoROM mapping).

**Character Conversion DMA**

Used to convert bitmaps or pixels to bit-planed tiles. For details, see

[SNES Cart SA-1 Character Conversion](#snes-cart-sa-1-character-conversion)

**SNES DMA (via Port 43xxh)**

Can be used to transfer "normal" data from ROM/BW-RAM/I-RAM to SNES memory,
also used for forwarding temporary Character Conversion data from I-RAM to
SNES.

**Unknown details**

Unknown if SDA/DDA are increased and if DTC is decreased (or if that operations
appear only on internal registers) (MSBs of DDA are apparently NOT increased on
char conversion DMAs).

<a id="snescartsa1characterconversion"></a>

## SNES Cart SA-1 Character Conversion

**Character Conversion Types**

```text
  Conversion  DMA-Transfer     Source / Pixel-Format
  Type 1      Automatic        BW-RAM, Packed Pixels, Bitmap Pixel Array
  Type 2      Semi-Automatic   CPU, Unpacked Pixels, 8x8 Pixel Tiles
```

Both Conversion types are writing data to a temporary buffer in I-RAM:

```text
  I-RAM buffer 32/64/128 bytes (two 8x8 tiles at 2bit/4bit/8bit color depth)
```

From that buffer, data is forwarded to SNES (via a simultanously executed SNES
DMA, ie. via ports 43xxh).

**Character Conversion 1 - Automatically Convert Packed BW-RAM Pixels**

Can be used only if the cartridge DOES contain BW-RAM (most or all do so).

First, do this on SA-1 side:

```text
  Set DCNT (Port 2230h) set to Char Conversion Type 1   (...and no DMA-enable?)
```

Then do following on SNES side:

```text
  Set SDA (Port 2232h-2234h)=BW-RAM offset, align by (bytes/char)*(chars/line)
  Set CDMA (Port 2231h) = store bits/pixel and chars/line
  Set DDA (Port 2235h-2236h)=I-RAM offset, align (bytes/char)*2 (2237h=unused)
  Wait for SFR.Bit5 (Port 2300h) Char_DMA_IRQ (=first character available)
  Launch SNES-DMA via Port 43xxh from "Virtual BW-RAM?" to PPU-VRAM
    (this can transfer the WHOLE bitmap in one pass)
```

Finally, after the SNES-DMA has finished, do this on SA-1 side:

```text
  Set CDMA.Bit7=1 (Port 2231h) - terminate SA-1 DMA
    (that stops writing to I-RAM on SA-1 side)
    (and stops tile-data to be mapped to 400000h-43FFFFh on SNES-side)
```

During conversion, the SA-1 can execute other program code (but waits may occur
on BW-RAM and I-RAM accesses). The SNES CPU is paused (by the DMA) for most of
the time, except for the time slots shortly before/after the DMA; in that time
slots, the SNES may access I-RAM, but may not access BW-RAM.

Conversion 1 is used by Haruka Naru Augusta 3 and Pebble Beach no Hotou.

**Character Conversion 2 - Semi-Automatic Convert Unpacked CPU Pixels**

First, do this on SA-1 side:

```text
  Set DCNT (Port 2230h) set to Char Conversion Type 2 and set DMA-enable
  Set CDMA (Port 2231h) = store bits/pixel (chars/line is not used)
  Set DDA (Port 2235h-2236h)=I-RAM offset, align (bytes/char)*2 (2237h=unused)
```

Then repeat for each character:

```text
  for y=0 to 7, for x=0 to 7, [2240h+x+(y and 1)]=pixel(x,y), next x,y
  On SNES side: Transfer DMA from 1st/2nd I-RAM buffer half to VRAM or WRAM
```

Finally,

```text
  Set DCNT.Bit7=0 (Port 2230h) - disable DMA
```

Conversion 2 is used by Haruka Naru Augusta 3 and SD Gundam G NEXT.

<a id="snescartsa1arithmeticmaths"></a>

## SNES Cart SA-1 Arithmetic Maths

**2250h SA-1 MCNT - Arithmetic Control (W)**

```text
  0-1 Arithmetic Mode (0=Multiply, 1=Divide, 2=MultiplySum, 3=Reserved)
  2-7 Not used (should be "..") (whatever ".." means, maybe "0"?)
```

Note: Writing Bit1=1 does reset the Sum (aka "Cumulative Sum" aka "Accumulative
Sum") to zero.

**2251h SA-1 MA - Arithmetic Parameter A Lsb (Multiplicand/Dividend) (W)**

**2252h SA-1 MA - Arithmetic Parameter A Msb (Multiplicand/Dividend) (W)**

```text
  0-15  SIGNED multiplicand or dividend (that is, both are signed)
```

The value in this register is kept intact after multiplaction, but gets
destroyed after division.

**2253h SA-1 MB - Arithmetic Parameter B Lsb (Multiplier/Divisor) (W)**

**2254h SA-1 MB - Arithmetic Parameter B Msb (Multiplier/Divisor)/Start (W)**

```text
  0-15  SIGNED multiply parameter, or UNSIGNED divisor
```

The value in this register gets destroyed after both multiplaction and
division. Writing to 2254h starts the operation. Execution time is 5 cycles (in
10.74MHz units) for both Multiply and Divide, and 6 cycles for Multiply/Sum.

**2306h SA-1 MR - Arithmetic Result, bit0-7   (Sum/Product/Quotient) (R)**

**2307h SA-1 MR - Arithmetic Result, bit8-15  (Sum/Product/Quotient) (R)**

**2308h SA-1 MR - Arithmetic Result, bit16-23 (Sum/Product/Remainder) (R)**

**2309h SA-1 MR - Arithmetic Result, bit24-31 (Sum/Product/Remainder) (R)**

**230Ah SA-1 MR - Arithmetic Result, bit32-39 (Sum) (R)**

```text
  32bit Multiply Result    (SIGNED)
  40bit Multiply/Sum       (SIGNED)
  16bit Division Result    (SIGNED)
  16bit Division Remainder (UNSIGNED !!!)
```

**230Bh SA-1 OF - Arithmetic Overflow Flag (R)**

This bit is reportedly set on 40bit multiply/addition overflows (rather than on
more useful 32bit overflows), thereby overflow can't occur unless one is doing
at least 512 continous multiply/additions.

```text
  0-6 Not used (reportedly "..") (whatever ".." means, maybe 0 or open bus?)
  7   Arithmetic Sum Overflow Flag (0=No overflow, 1=Overflow)
```

Unknown when this bit gets cleared (all operations, or mode changes)?

Division by zero returns result=0000h and remainder=0000h (other info claims
other values?) (but, as far as known, doesn't set set overflow flag).

<a id="snescartsa1variablelengthbitprocessing"></a>

## SNES Cart SA-1 Variable-Length Bit Processing

**2258h SA-1 VBD - Variable-Length Bit Processing (W)**

```text
  0-3  Data Length (1..15=1..15 bits, or 0=16 bits)
  4-6  Not used (should be "..") (whatever ".." means, maybe "0"?)
  7    Data Read Mode (0=Fixed Mode, 1=Auto-increment)
```

Manual/Fixed Mode is used by Jumpin Derby. Auto-increment isn't used by any
known games.

**2259h SA-1 VDA - Variable-Length Bit Game Pak ROM Start Address Lsb (W)**

**225Ah SA-1 VDA - Variable-Length Bit Game Pak ROM Start Address Mid (W)**

**225Bh SA-1 VDA - Variable-Length Bit Game Pak ROM Start Address Msb &amp; Kick**

```text
  0-23  Game Pak ROM Address
```

Reading starts on writing to 225Bh.

The ROM address is probably originated at 000000h (rather than using
LoROM/HiROM like CPU addresses)?

**230Ch SA-1 VDP - Variable-Length Data Read Port Lsb (R)**

**230Dh SA-1 VDP - Variable-Length Data Read Port Msb (R)**

```text
  0-15  Data
```

Unknown what happens on data length less than 16bits:

```text
  Are the selected bits located in MSBs or LSBs?
  Are the other bits set to zero? To next/prev values? Sign-expanded??
```

There is an "auto-increment" feature, which may trigger on reading 230Ch? or on
reading or 230Dh?

;*******PRELOAD:

;Preload occurs after writing VDA

;        bitpos = [2259h]*8

;        [230Ch] = WORD[bitpos/8]

;*******INCREMENT:

;Increment occurs AFTER reading VDP (when auto-increment enabled),

;and after writing VDB (reportedly always, but SHOULD be ONLY when inc=off)?

;        bitpos=bitpos+(([2258h]-1) AND 0Fh)+1

;        [230Ch] = dword[bitpos/16*2] shr (bitpos and 15) AND FFFFh

<a id="snescartgsunprogrammablerisccpuakasuperfxmariochip10games"></a>

## SNES Cart GSU-n (programmable RISC CPU) (aka Super FX/Mario Chip) (10 games)

Graphic Support Unit (GSU) (10.74MHz RISC-like CPU)

[SNES Cart GSU-n List of Games, Chips, and PCB versions](#snes-cart-gsu-n-list-of-games-chips-and-pcb-versions)

[SNES Cart GSU-n Memory Map](#snes-cart-gsu-n-memory-map)

[SNES Cart GSU-n I/O Map](#snes-cart-gsu-n-io-map)

[SNES Cart GSU-n General I/O Ports](#snes-cart-gsu-n-general-io-ports)

[SNES Cart GSU-n Bitmap I/O Ports](#snes-cart-gsu-n-bitmap-io-ports)

**GSU Opcodes**

[SNES Cart GSU-n CPU MOV Opcodes](#snes-cart-gsu-n-cpu-mov-opcodes)

[SNES Cart GSU-n CPU ALU Opcodes](#snes-cart-gsu-n-cpu-alu-opcodes)

[SNES Cart GSU-n CPU JMP and Prefix Opcodes](#snes-cart-gsu-n-cpu-jmp-and-prefix-opcodes)

[SNES Cart GSU-n CPU Pseudo Opcodes](#snes-cart-gsu-n-cpu-pseudo-opcodes)

**Misc**

[SNES Cart GSU-n CPU Misc](#snes-cart-gsu-n-cpu-misc)

**GSU Caches**

[SNES Cart GSU-n Code-Cache](#snes-cart-gsu-n-code-cache)

[SNES Cart GSU-n Pixel-Cache](#snes-cart-gsu-n-pixel-cache)

[SNES Cart GSU-n Other Caches](#snes-cart-gsu-n-other-caches)

**Pinouts**

[SNES Pinouts GSU Chips](80-timings-unpredictable-pinouts.md#snes-pinouts-gsu-chips)

<a id="snescartgsunlistofgameschipsandpcbversions"></a>

## SNES Cart GSU-n List of Games, Chips, and PCB versions

**GSU1/Mario Chip1 is used by six games:**

```text
  Dirt Racer (1994) MotiveTime/Elite Systems (EU)
  Dirt Trax FX (1995) Sculptured Software/Acclaim Entertainment (NA)
  Powerslide (cancelled, but unfinished prototype leaked) Elite Systems (EU)
  Star Fox / Starwing (1993) Argonaut/Nintendo EAD (NA) (JP) (EU)
  Star Fox / Starwing: Competition Edition (demo version) (1993) (NA) (EU)
  Stunt Race FX / Wild Trax (1994) Argonaut/Nintendo EAD (NA) (JP) (EU)
  Vortex (1994) Argonaut Games/Electro Brain (NA), Pack-In-Video (JP)
```

**GSU2/GSU2-SP1 is used by four games:**

```text
  Doom (1996) Sculptured Software/Williams (NA), Imagineer (JP), Ocean (EU)
  Super Mario World 2: Yoshi's Island (1995) Nintendo EAD (NA) (JP) (EU)
  Winter Gold / FX Skiing (1997) Funcom/Nintendo (NA) (EU)
  Star Fox 2 (cancelled, but near-finished Beta version leaked into internet)
```

Reportedly, there have been another three GSU2 games planned:

```text
  FX Fighter (Beta) (cancelled) Argonaut Games/GTE Entertainment (NA) (EU)
  Comanche (cancelled) Nova Logic (NA)
  Super Mario FX (cancelled) Nintendo EAD
```

**GSU Chip Versions**

```text
  MC1      - 100pin - A/N Inc. Nintendo Mario Chip 1 (reportedly "FX-chip 1")
  GSU1     - 100pin - A/N Inc. Nintendo Super FX 1 (10.74MHz RISC-like CPU)
  GSU1A    - 100pin - A/N Inc. Nintendo Super FX 1
  GSU2     - 112pin - A/N Inc. Nintendo Super FX 2 (as above, but 21MHz)
  GSU2-SP1 - 112pin - A/N Inc. Nintendo Super FX 2 (as above, but 21MHz)
```

XXX according to MotZilla, GSU1 supports 21MHz, too? (but with less memory)

**GSU PCB Versions**

```text
  SHVC-1C0N         Mario Chip 1      Star Fox (Blob)
  SHVC-1C0N5S-01    Mario Chip 1      Star Fox (SMD)
  SHVC-1CA0N5S-01   GSU-1             Dirt Racer & Vortex
  SHVC-1CA0N6S-01   GSU-1             Dirt Trax FX
  SHVC-1CA6B-01     GSU-1 Battery     Stunt Race FX
  SHVC-1CB0N7S-01   GSU-2             Doom
  SHVC-1CB5B-01     GSU-2 Battery     Super Mario World 2: Yoshi's Island
  SHVC-1CB5B-20     GSU-2-SP1 Battery Super Mario World 2: Yoshi's Island
  SHVC-1RA2B6S-01   GSU1A Batt+Eprom  Powerslide (prototype board)
  GS 0871-102       Mario Chip 1      Super Famicom Box PSS61 multi-game-cart
```

Note: Doom's "1CB0N7S" board has only 64K RAM installed (not 128K).

<a id="snescartgsunmemorymap"></a>

## SNES Cart GSU-n Memory Map

**MC1 Memory Map (at SNES Side)**

```text
  00-3F/80-BF:3000-347F  GSU I/O Ports
  00-1F/80-9F:8000-FFFF  Game Pak ROM in LoRom mapping (1Mbyte max)
  60-7D/E0-FF:0000-FFFF  Game Pak RAM with mirrors (64Kbyte max?, usually 32K)
  Other Addresses        Open Bus
```

**GSU1 Memory Map (at SNES Side)**

```text
  00-3F/80-BF:3000-34FF? GSU I/O Ports
  00-3F/80-BF:6000-7FFF  Mirror of 70:0000-1FFF (ie. FIRST 8K of Game Pak RAM)
  00-3F/80-BF:8000-FFFF  Game Pak ROM in LoRom mapping (1Mbyte max?)
  40-5F/C0-DF:0000-FFFF  Game Pak ROM in HiRom mapping (mirror of above)
  70-71/F0-F1:0000-FFFF  Game Pak RAM with mirrors (64Kbyte max?, usually 32K)
  78-7x/F8-Fx:0000-FFFF  Unknown (maybe Additional "Backup" RAM like GSU2)
  Other Addresses        Open Bus
```

**GSU2 Memory Map (at SNES Side)**

```text
  00-3F/80-BF:3000-34FF  GSU I/O Ports
  00-3F/80-BF:6000-7FFF  Mirror of 70:0000-1FFF (ie. FIRST 8K of Game Pak RAM)
  00-3F:8000-FFFF        Game Pak ROM in LoRom mapping (2Mbyte max)
  40-5F:0000-FFFF        Game Pak ROM in HiRom mapping (mirror of above)
  70-71:0000-FFFF        Game Pak RAM       (128Kbyte max, usually 32K or 64K)
  78-79:0000-FFFF        Additional "Backup" RAM  (128Kbyte max, usually none)
  80-BF:8000-FFFF        Additional "CPU" ROM LoROM (2Mbyte max, usually none)
  C0-FF:0000-FFFF        Additional "CPU" ROM HiROM (4Mbyte max, usually none)
  Other Addresses        Open Bus
```

For HiROM mapping the address bits are shifted, so both LoROM and HiROM are
linear (eg. Bank 40h contains mirrors of Bank 00h and 01h).

Although both LoROM and HiROM are supported, the header &amp; exception vectors
are located at ROM Offset 7Fxxh (in LoROM fashion), accordingly the cartridge
header declares the cartridge as LoROM.

The additional ROM/RAM regions would be mapped to SNES CPU only (not to GSU),
they aren't installed in existing cartridges, that implies that the "Fast" ROM
banks (80h-FFh) are unused, so GSU games are restricted to "Slow" ROM.

**GSU2 Memory Map (at GSU Side)**

```text
  00-3F:0000-7FFF  Mirror of LoROM at 00-3F:8000-FFFF (for "GETB R15" vectors)
  00-3F:8000-FFFF  Game Pak ROM in LoRom mapping (2Mbyte max)
  40-5F:0000-FFFF  Game Pak ROM in HiRom mapping (mirror of above 2Mbyte)
  70-71:0000-FFFF  Game Pak RAM       (128Kbyte max, usually 32K or 64K)
  PBR:0000-01FF    Code-Cache (when having manually stored opcodes in it)
```

PBR can be set to both ROM/RAM regions (or cache region), ROMBR only to ROM
region (00h-5Fh), RAMBR only to RAM region (70h-71h).

**GSU Interrupt Vectors**

The SNES Exception Vectors (at FFE4h-FFFFh) are normally located in Game Pak
ROM. When the GSU is running (with GO=1 and RON=1), ROM isn't mapped to SNES
memory, instead, fixed values are appearing as ROM (depending of the lower 4bit
of the address):

```text
  Any Address     Exception Vectors
  [xxx0h]=0100h   -
  [xxx2h]=0100h   -
  [xxx4h]=0104h   [FFE4h]=0104h  COP Vector in 65C816 mode (COP opcode)
  [xxx6h]=0100h   [FFE6h]=0100h  BRK Vector in 65C816 mode (BRK opcode)
  [xxx8h]=0100h   [FFE8h]=0100h  ABT Vector in 65C816 mode (Not used in SNES)
  [xxxAh]=0108h   [FFEAh]=0108h  NMI Vector in 65C816 mode (Vblank)
  [xxxCh]=0100h   -
  [xxxEh]=010Ch   [FFEEh]=010Ch  IRQ Vector in 65C816 mode (H/V-IRQ & GSU-STOP)
```

It'd be best to set the Game Pak ROM vectors to the same addresses, otherwise
the vectors would change when the GSU is running (or possibly, the fixed-LSBs
may be mixed-up with ROM-MSBs).

**GSU Cartridge Header (always at ROM Offset 7Fxxh, in LoROM fashion)**

```text
  [FFD5h]=20h        Set to "Slow/LoROM" (although both LoROM/HiROM works)
  [FFD6h]=13h..1Ah   Chipset = GSUn (plus battery present/absent info)
  [FFD8h]=00h        Normal SRAM Size (None) (always use the Expansion entry)
  [FFBDh]=05h..06h   Expansion RAM Size (32Kbyte and 64Kbyte exist)
  Caution: Starfox/Star Wing, Powerslide, and Starfox 2 do not have extended
  headers (and thereby no [FFBDh] entry). RAM Size for Starfox/Starwing is
  32Kbytes, RAM Size for Powerslide and Starfox 2 is unknown.
```

There is no info in the header (nor extended header) whether the game uses a
GSU1 or GSU2. Games with 2MByte ROM are typically using GSU2 (though that rule
doesn't always match: Star Fox 2 is only 1MByte).

**GSU Busses**

The GSU seems to have 4 address/data busses (three external ones, and one
internal cache bus):

```text
  SNES bus (for forwarding ROM/RAM access to SNES)
  ROM bus (for GSU opcode fetches, GETxx reads, and SNES reads)
  RAM bus (for GSU opcode fetches, LOAD/STORE/PLOT/RPIX, and SNES access)
  Code cache bus (for GSU opcode fetches only) (and SNES I/O via 3100h..32FFh)
```

To some level, this allows to do multiple things simultaneously: Reading a GSU
opcode from cache at the same time while prefetching ROM data and forwarding
the RAM or Pixel cache to RAM.

<a id="snescartgsuniomap"></a>

## SNES Cart GSU-n I/O Map

**GSU I/O Map (in banks 00h-3Fh and 80h-BFh)**

During GSU operation, only SFR, SCMR, and VCR may be accessed.

```text
  3000h-3001h R0  Default source/destination register (Sreg/Dreg) (R/W)
  3002h-3003h R1  PLOT opcode: X coordinate (0000h on reset) (R/W)
  3004h-3005h R2  PLOT opcode: Y coordinate (0000h on reset) (R/W)
  3006h-3007h R3  General purpose (R/W)
  3008h-3009h R4  LMULT opcode: lower 16bits of result (R/W)
  300Ah-300Bh R5  General purpose (R/W)
  300Ch-300Dh R6  LMULT and FMULT opcodes: multiplier (R/W)
  300Eh-300Fh R7  MERGE opcode (R/W)
  3010h-3011h R8  MERGE opcode (R/W)
  3012h-3013h R9  General purpose (R/W)
  3014h-3015h R10 General purpose (conventionally stack pointer) (R/W)
  3016h-3017h R11 LINK opcode: destination (R/W)
  3018h-3019h R12 LOOP opcode: counter (R/W)
  301Ah-301Bh R13 LOOP opcode: address (R/W)
  301Ch-301Dh R14 GETxx opcodes: Game Pak ROM Address Pointer (R/W)
  301Eh-301Fh R15 Program Counter, writing MSB starts GSU operation (R/W)
  3020h-302Fh -
  3030h-3031h SFR Status/Flag Register (R) (Bit1-5: R/W)
  3032h       -
  3033h       BRAMR Back-up RAM Register (W)
  3034h       PBR   Program Bank Register (8bit, bank 00h..FFh) (R/W)
  3035h       -
  3036h       ROMBR Game Pak ROM Bank Register (8bit, bank 00h..FFh) (R)
  3037h       CFGR  Config Register (W)
  3038h       SCBR  Screen Base Register (8bit, in 1Kbyte units) (W)
  3039h       CLSR  Clock Select Register (W)
  303Ah       SCMR  Screen Mode Register (W)
  303Bh       VCR   Version Code Register (R)
  303Ch       RAMBR Game Pak RAM Bank Register (1bit, bank 70h/71h) (R)
  303Dh       -
  303Eh-303Fh CBR   Cache Base Register (in upper 12bit; lower 4bit=unused) (R)
  N/A         COLR  Color Register (COLOR,GETC,PLOT opcodes)
  N/A         POR   Plot Option Register (CMODE opcode)
  N/A         Sreg/Dreg    Memorized TO/FROM Prefix Selections
  N/A         ROM Read Buffer (1 byte) (prefetched from [ROMBR:R14])
  N/A         RAM Write Buffer (1 byte/word)
  N/A         RAM Address (1 word, or word+rambr?) (for SBK opcode)
  N/A         Pixel Write Buffer (two buffers for one 8-pixel row each)
  3100h-32FFh Cache RAM
```

**Full I/O Map with Mirrors for Black Blob (VCR=01h)**

```text
  3000h..301Fh  20h  R0-R15
  3020h..302Fh  10h  open bus
  3030h..3031h  2    status reg
  3032h..303Fh  0Eh  mirrors of status reg (except 303Bh=01h=VCR)
  3040h..305Fh  20h  mirror of R0-R15
  3060h..307Fh  20h  mirrors of status reg (except 307Bh=01h=VCR)
  3080h..30FFh  80h  open bus
  3100h..32FFh  200h cache
  3300h..332Fh  30h  open bus
  3330h..333Fh  10h  mirrors of status reg (except 333Bh=01h=VCR)
  3340h..335Fh  20h  mirror of R0-R15
  3360h..337Fh  20h  mirrors of status reg (except 337Bh=01h=VCR)
  3380h..33FFh  80h  open bus
  3400h..342Fh  30h  open bus
  3430h..343Fh  10h  mirrors of status reg (except 343Bh=01h=VCR)
  3440h..345Fh  20h  mirror of R0-R15
  3460h..347Fh  20h  mirrors of status reg (except 347Bh=01h=VCR)
  3480h..3FFFh  B80h open bus
```

**Full I/O Map with Mirrors for GSU2 (VCR=04h)**

```text
  3000h..301Fh  20h   R0-R15
  3020h..302Fh  10h   mirror of 3030h..303Fh
  3030h..303Fh  10h   status regs (unused or write-only ones return 00h)
  3040h..30FFh  C0h   mirrors of 3000h..303Fh
  3100h..32FFh  200h  cache
  3300h..34FFh  200h  mirrors of 3000h..303Fh
  3500h..3FFFh  B00h  open-bus
```

<a id="snescartgsungeneralioports"></a>

## SNES Cart GSU-n General I/O Ports

**3000h-301Fh - R0-R15 - CPU Registers (R/W)**

16bit CPU registers (see GSU I/O map for additional details on each register).

Writes to 3000h-301Eh (even addresses) do set LATCH=data.

Writes to 3001h-301Fh (odd addresses) do apply LSB=LATCH and MSB=data.

Writes to 301Fh (R15.MSB) do also set GO=1 (and start GSU code execution).

**3030h/3031h - SFR - Status/Flag Register (R) (Bit1-5: R/W)**

```text
  0  -    Always 0                                                        (R)
  1  Z    Zero Flag     (0=NotZero/NotEqual, 1=Zero/Equal)                (R/W)
  2  CY   Carry Flag    (0=Borrow/NoCarry, 1=Carry/NoBorrow)              (R/W)
  3  S    Sign Flag     (0=Positive, 1=Negative)                          (R/W)
  4  OV   Overflow Flag (0=NoOverflow, 1=Overflow)                        (R/W)
  5  GO   GSU is running (cleared on STOP) (can be forcefully=0 via 3030h)(R/W)
  6  R    ROM[R14] Read (0=No, 1=Reading ROM via R14 address)             (R)
  7  -    Always 0                                                        (R)
  8  ALT1 Prefix Flag           ;\for ALT1,ALT2,ALT3 prefixes             (R)
  9  ALT2 Prefix Flag           ;/                                        (R)
  10 IL   Immediate lower 8bit flag ;\Unknown, probably set/reset internally
  11 IH   Immediate upper 8bit flag ;/when processing opcodes with imm operands
  12 B    Prefix Flag           ;-for WITH prefix (used by MOVE/MOVES opcodes)
  13 -    Always 0                                                        (R)
  14 -    Always 0                                                        (R)
  15 IRQ  Interrupt Flag (reset on read, set on STOP) (also set if IRQ masked?)
```

This register is read/write-able even when the GSU is running; reading mainly
makes sense for checking GO and IRQ bits, writing allows to clear the GO flag
(thereby aborting the GSU program; the write does most likely also destroy the
other SFR bits, so one cannot pause/resume).

**3034h - PBR - Program Bank Register (8bit, bank 00h..5Fh,70h..71h) (R/W)**

**3036h - ROMBR - Game Pak ROM Bank Register (8bit, bank 00h..5Fh) (R)**

**303Ch - RAMBR - Game Pak RAM Bank Register (1bit, bank 70h..71h) (R)**

Memory banks for GSU opcode/data accesses. PBR can be set to both ROM and RAM
regions, ROMBR/RAMBR only to ROM or RAM regions respectively. Existing
cartridges have only 32Kbyte or 64Kbyte RAM, so RAMBR should be always zero.

According to book2 (page 258), the screen base is also affected by RAMBR
(unknown if that is true, theoretically, SCBR is large enough to address more
than 64Kbytes without RAMBR).

**303Eh/303Fh - CBR - Cache Base Register (upper 12bit; lower 4bit=unused) (R)**

Code-Cache Base for Game Pak ROM/RAM. The register is read-only, so the SNES
cannot directly write to it, however, the SNES can set CBR=0000h by writing
GO=0 (in SFR register).

**3033h - BRAMR - Back-up RAM Register (W)**

```text
  0   BRAM Flag (0=Disable/Protect, 1=Enable)
  1-7 Not used (should be zero)
```

This register would be used only if the PCB does have a separate "Backup" RAM
chip mapped to 780000h-79FFFFh (additionally to the Game Pak RAM chip). None of
the existing PCBs is having that extra RAM chip, so the register is having no
function. (Note: However, some PCBs do include a battery wired to Game Pak RAM
chip, anyways, that type of "backup" isn't affected by this register).

**303Bh - VCR - Version Code Register (R)**

```text
  0-7 GSU Chip Version (01h..0xh ?)
```

Known versions: 1=MC1/Blob, ?=MC1/SMD, ?=GSU1, ?=GSU1A, 4=GSU2, ?=GSU2-SP1.

**3037h - CFGR - Config Register (W)**

```text
  0-4 -   Not used (should be zero)
  5   MS0 Multiplier Speed Select (0=Standard, 1=High Speed Mode)
  6   -   Not used (should be zero)
  7   IRQ Interrupt Mask (0=Trigger IRQ on STOP opcode, 1=Disable IRQ)
```

MS0 &lt;must&gt; be zero in 21MHz mode (ie. only CFGR.Bit5 or CLSR.Bit0 may be
set).

MS0 is implemented in GSU2 (maybe also other chips), it is not implemented on
Black Blob MC1 (which is always using slow multiply mode).

**3039h - CLSR - Clock Select Register (W)**

```text
  0   CLS Clock Select (0=10.7MHz, 1=21.4MHz)
  1-7 -   Not used (should be zero)
```

CLS exists on all GSU variants (including Black Blob MC1) (however, there are
rumours that the fast mode was "bugged" on older MC1, unknown if that's true).

**N/A - ROM Buffer - Prefetched Byte(s?) at [ROMBR:R14]**

**N/A - Sreg/Dreg - Memorized TO/FROM Prefix Selections**

**3100h..32FFh - Cache RAM**

<a id="snescartgsunbitmapioports"></a>

## SNES Cart GSU-n Bitmap I/O Ports

**3038h - SCBR - Screen Base Register (8bit, in 1Kbyte units) (W)**

```text
  0-7  Screen Base in 1K-byte Units (Base = 700000h+N*400h)
```

**303Ah - SCMR - Screen Mode Register (W)**

```text
  0-1 MD0-1 Color Gradient (0=4-Color, 1=16-Color, 2=Reserved, 3=256-Color)
  2   HT0   Screen Height  (0=128-Pixel, 1=160-Pixel, 2=192-Pixel, 3=OBJ-Mode)
  3   RAN   Game Pak RAM bus access (0=SNES, 1=GSU)
  4   RON   Game Pak ROM bus access (0=SNES, 1=GSU)
  5   HT1   Screen Height  (MSB of HT0 bit)
  6-7 -     Not used (should be zero)
```

RON/RAN can be temporarily cleared during GSU operation, this causes the GSU to
enter WAIT status (if it accesses ROM or RAM), and continues when RON/RAN are
changed back to 1.

Note that "OBJ Mode" can be also selected by POR.Bit4 (if so, HT0/HT1 bits are
ignored).

```text
  256x128 pixels   256x160 pixels   256x192 pixels   OBJ Mode 256x256 pixel
  000 010 .. 1F0 | 000 014 .. 26C | 000 018 .. 1E8 | 000 .. 00F 100 .. 10F
  001 011 .. 1F1 | 001 015 .. 26D | 001 019 .. 1E9 | ..  .. ..  ..  .. ..
  ..  ..  .. ..  | ..  ..  .. ..  | ..  ..  .. ..  | 0F0 .. 0FF 1F0 .. 1FF
  ..  ..  .. ..  | ..  ..  .. ..  | ..  ..  .. ..  | 200 .. 20F 300 .. 30F
  00E 01E .. 1FE | 012 026 .. 27E | 016 02E .. 2FE | ..  .. ..  ..  .. ..
  00F 01F .. 1FF | 013 027 .. 27F | 017 02F .. 2FF | 2F0 .. 2FF 3F0 .. 3FF
```

In the first three cases, BG Map is simply filled with columns containing
increasing tile numbers. The fourth case is matched to the SNES two-dimensional
OBJ mapping; it can be used for BG Map (with entries 0..3FF as shown above), or
for OBJ tiles (whereas, mind that the SNES supports only 0..1FF OBJs, not
200..3FF).

The Tile Number is calculated as:

```text
  Height 128 --> (X/8)*10h + (Y/8)
  Height 160 --> (X/8)*14h + (Y/8)
  Height 192 --> (X/8)*18h + (Y/8)
  OBJ Mode --> (Y/80h)*200h + (X/80h)*100h + (Y/8 AND 0Fh)*10h + (X/8 AND 0Fh)
```

The Tile-Row Address is:

```text
  4 Color Mode    TileNo*10h + SCBR*400h + (Y AND 7)*2
  16 Color Mode   TileNo*20h + SCBR*400h + (Y AND 7)*2
  256 Color Mode  TileNo*40h + SCBR*400h + (Y AND 7)*2
```

With Plane0,1 stored at Addr+0, Plane 2,3 at Addr+10h, Plane 4,5 at Addr+20h,
Plane 6,7 at Addr+30h.

**N/A - COLR - Color Register**

```text
  0-7 CD0-7 Color Data
```

**N/A - POR - Plot Option Register (CMODE)**

```text
  0   PLOT Transparent       (0=Do Not Plot Color 0, 1=Plot Color 0)
  1   PLOT Dither            (0=Normal, 1=Dither; 4/16-color mode only)
  2   COLOR/GETC High-Nibble (0=Normal, 1=Replace incoming LSB by incoming MSB)
  3   COLOR/GETC Freeze-High (0=Normal, 1=Write-protect COLOR.MSB)
  4   OBJ Mode               (0=Normal, 1=Force OBJ mode; ignore SCMR.HT0/HT1)
  5-7 Not used (should be zero)
```

Can be changed by CMODE opcode, used for COLOR/GETC/PLOT opcodes.

Dither can mix transparent &amp; non-transparent pixels.

Bit0=0 (Transparent) causes PLOT to skip color 0 (so PLOT does only increment
R1 (X-coordinate), but doesn't draw a pixel). Depending on color depth, the
color 0 check tests the lower 2/4/8 bits of the drawing color (if POR.Bit3
(Freeze-High) is set, then it checks only the lower 2/4 bits, and ignores upper
4bit even when in 256-color mode).

Bit1=1 (Dither) causes PLOT to use dithering, that is, if "(r1.bit0 XOR
r2.bit0)=1" then COLOR/10h is used as drawing color; using Color 0 as one of
the two colors can produce a semi-transparency effect. Dither is ignored in
256-color mode.

Bit2=1 (High-Nibble) causes COLOR/GETC to replace the LSB of the incoming data
by the MSB of the incoming data; this allows two 4bit bitmaps being stored at
the same memory area (one in the LSBs, the other in MSBs).

Bit3=1 (Freeze-High) causes COLOR/GETC to change only the LSB of the color
register; this allows the MSB to be used as fixed palette-like value in
256-color mode, it might be also useful for fixed dither-colors in 4/16 color
mode.

Bit3=1 forces OBJ Mode (same as when setting SCMR.HT0/HT1 to OBJ Mode).

```text
  <------- COLOR/GETC TO COLOR ------->    <------- PLOT COLOR TO RAM -------->
               ______              __________                    ______
  Bit7-4 --+--|Freeze|- - - - - ->|          |---+------------->|      |
           |  |POR.3 |            |  COLOR   |   |              |Transp|
           |  |______|  ______    | _ _ _ _  |   |    ______    |POR.0 |--> RAM
           '---------->|Nibble|   |          |   '-->|Dither|   |      |
                       |POR.2 |-->| Register |       |POR.1 |-->|      |
  Bit3-0 ------------->|______|   |__________|------>|______|   |______|
```

<a id="snescartgsuncpumovopcodes"></a>

## SNES Cart GSU-n CPU MOV Opcodes

**GSU MOV Opcodes (Register/Immediate)**

```text
  Opcode     Clks Flags   Native       Nocash
  2s 1d         2 000---- MOVE Rd,Rs   mov Rd,Rs   ;Rd=Rs
  2d Bs         2 000vs-z MOVES Rd,Rs  movs Rd,Rs  ;Rd=Rs (with flags, OV=bit7)
  An pp         2 000---- IBT Rn,#pp   mov Rn,pp   ;Rn=SignExpanded(pp)
  Fn xx yy      3 000---- IWT Rn,#yyxx mov Rn,yyxx ;Rn=yyxx
```

**GSU MOV Opcodes (Load BYTE from ROM)**

```text
  EF          1-6 000---- GETB         movb Rd,[romb:r14]    ;hi=zero-expanded
  3D EF       2-6 000---- GETBH        movb Rd.hi,[romb:r14] ;lo=unchanged
  3E EF       2-6 000---- GETBL        movb Rd.lo,[romb:r14] ;hi=unchanged
  3F EF       2-6 000---- GETBS        movbs Rd,[romb:r14]   ;hi=sign-expanded
```

**GSU MOV Opcodes (Load/Store Byte/Word to/from RAM)**

```text
  3D 4n         6 000---- LDB (Rn)     movb Rd,[ramb:Rn]  ;Rd=Byte[..] ;n=0..11
  4n            7 000---- LDW (Rn)     mov Rd,[ramb:Rn]   ;Rd=Word[..] ;n=0..11
  3D Fn lo hi  11 000---- LM Rn,(hilo) mov Rn,[ramb:hilo] ;Rn=Word[..]
  3D An kk     10 000---- LMS Rn,(yy)  mov Rn,[ramb:kk*2] ;Rn=Word[..]
  3D 3n       2-5 000---- STB (Rn)     movb [ramb:Rn],Rs  ;Byte[..]=Rs ;n=0..11
  3n          1-6 000---- STW (Rn)     mov [ramb:Rn],Rs   ;Word[..]=Rs ;n=0..11
  3E Fn lo hi 4-9 000---- SM (hilo),Rn mov [ramb:hilo],Rn ;Word[..]=Rn
  3E An kk    3-8 000---- SMS (yy),Rn  mov [ramb:kk*2],Rn ;Word[..]=Rn
  90          1-6 000---- SBK          mov [ram:bk],Rs    ;Word[LastRamAddr]=Rs
```

Words at odd addresses are accessing [addr AND NOT 1], with data LSB/MSB
swapped. LDB does zero-expand result (Rd.hi=00h). STB does store Rs.lo (ignores
Rs.hi). SBK does "writeback" to most recently used RAM address (eg. can be used
after LM) (unknown if whole 17bit, including ramb, are saved).

**GSU ROM/RAM Banks**

```text
  3E DF         2 000---- RAMB         movb ramb,Rs ;RAMBR=Rs & 01h ;RAM Bank
  3F DF         2 000---- ROMB         movb romb,Rs ;ROMBR=Rs & FFh ;ROM Bank
```

**GSU Bitmap Opcodes**

```text
  3D 4E         2 000---- CMODE        movb por,Rs           ;=Rs&1Fh
  4E            1 000---- COLOR        movb color,Rs         ;=Rs&FFh
  DF          1-6 000---- GETC         movb color,[romb:r14] ;=[membyte]
  4C         1-48 000---- PLOT         plot [r1,r2],color ;Pixel=COLR, R1=R1+1
  3D 4C     20-74 000-s-z RPIX         rpix Rd,[r1,r2] ;Rd=Pixel? FlushPixCache
```

Unknown if RPIX always sets SF=0, theoretically 2bit/4bit/8bit pixel-colors
cannot be negative, unless it uses bit7 as sign, or so?

<a id="snescartgsuncpualuopcodes"></a>

## SNES Cart GSU-n CPU ALU Opcodes

**GSU ALU Opcodes**

```text
  Opcode     Clks Flags   Native       Nocash
  5n            1 000vscz ADD Rn       add Rd,Rs,Rn ;Rd=Rs+Rn
  3E 5n         2 000vscz ADD #n       add Rd,Rs,n  ;Rd=Rs+n
  3D 5n         2 000vscz ADC Rn       adc Rd,Rs,Rn ;Rd=Rs+Rn+Cy
  3F 5n         2 000vscz ADC #n       adc Rd,Rs,n  ;Rd=Rs+n+Cy
  6n            1 000vscz SUB Rn       sub Rd,Rs,Rn ;Rd=Rs-Rn
  3E 6n         2 000vscz SUB #n       sub Rd,Rs,n  ;Rd=Rs-n
  3D 6n         2 000vscz SBC Rn       sbc Rd,Rs,Rn ;Rd=Rs-Rn-(Cy XOR 1)
  3F 6n         2 000vscz CMP Rn       cmp Rs,Rn    ;Rs-Rn
  7n            1 000-s-z AND Rn       and Rd,Rs,Rn ;Rd=Rs AND Rn     ;n=1..15!
  3E 7n         2 000-s-z AND #n       and Rd,Rs,n  ;Rd=Rs AND n      ;n=1..15!
  3D 7n         2 000-s-z BIC Rn       bic Rd,Rs,Rn ;Rd=Rs AND NOT Rn ;n=1..15!
  3F 7n         2 000-s-z BIC #n       bic Rd,Rs,n  ;Rd=Rs AND NOT n  ;n=1..15!
  Cn            1 000-s-z OR  Rn       or  Rd,Rs,Rn ;Rd=Rs OR Rn      ;n=1..15!
  3E Cn         2 000-s-z OR  #n       or  Rd,Rs,n  ;Rd=Rs OR n       ;n=1..15!
  3D Cn (?)     2 000-s-z XOR Rn       xor Rd,Rs,Rn ;Rd=Rs XOR Rn     ;n=1..15?
  3F Cn (?)     2 000-s-z XOR #n       xor Rd,Rs,n  ;Rd=Rs XOR n      ;n=1..15?
  4F            1 000-s-z NOT          not Rd,Rs    ;Rd=Rs XOR FFFFh
```

**GSU Rotate/Shift/Inc/Dec Opcodes**

```text
  03            1 000-0cz LSR          shr Rd,Rs,1  ;Rd=Rs SHR 1
  96            1 000-scz ASR          sar Rd,Rs,1  ;Rd=Rs SAR 1
  04            1 000-scz ROL          rcl Rd,Rs,1  ;Rd=Rs RCL 1 ;\through
  97            1 000-scz ROR          rcr Rd,Rs,1  ;Rd=Rs RCR 1 ;/carry
  3D 96         2 000-scz DIV2         div2 Rd,Rs   ;Rd=Rs SAR 1, Rd=0 if Rs=-1
  Dn            1 000-s-z INC Rn       inc Rn       ;Rn=Rn+1          ;n=0..14!
  En            1 000-s-z DEC Rn       dec Rn       ;Rn=Rn-1          ;n=0..14!
```

**GSU Byte Operations**

```text
  4D            1 000-s-z SWAP         ror Rd,Rs,8    ;Rd=Rs ROR 8
  95            1 000-s-z SEX          movbs Rd,Rs    ;Rd=SignExpanded(Rs&FFh)
  9E            1 000-s-z LOB          and Rd,Rs,0FFh ;Rd=Rs AND FFh  ;SF=Bit7
  C0            1 000-s-z HIB          shr Rd,Rs,8    ;Rd=Rs SHR 8    ;SF=Bit7
  70            1 000xxxx MERGE        merge Rd,r7,r8 ;Rd=R7&FF00 + R8/100h
```

Flags for MERGE are:

```text
  S = set if (result AND 8080h) is nonzero
  V = set if (result AND C0C0h) is nonzero
  C = set if (result AND E0E0h) is nonzero
  Z = set if (result AND F0F0h) is nonzero (not set when zero!)
```

**GSU Multiply Opcodes**

```text
  9F          4,8 000-scz FMULT     smulw Rd:nul,Rs,r6 ;Rd=signed(Rs*R6/10000h)
  3D 9F       5,9 000-scz LMULT     smulw Rd:R4,Rs,R6  ;Rd:R4=signed(Rs*R6)
  8n          1,2 000-s-z MULT Rn      smulb Rd,Rs,Rn ;Rd=signed(RsLsb*RnLsb)
  3E 8n       2,3 000-s-z MULT #n      smulb Rd,Rs,n  ;Rd=signed(RsLsb*0..15)
  3D 8n       2,3 000-s-z UMULT Rn     umulb Rd,Rs,Rn ;Rd=unsigned(RsLsb*RnLsb)
  3F 8n (?)   2,3 000-s-z UMULT #n     umulb Rd,Rs,n  ;Rd=unsigned(RsLsb*0..15)
```

The multiply speed can be selected via CFGR register. Do not use FMULT with
Dreg=R4 (this will reportedly leave R4 unchanged). When using LMULT with
Dreg=R4 then the result will be R4=MSB (and LSB is lost). Ie. if that is true
then, strangely, LMULT R4 &lt;does&gt; work as how FMULT R4 &lt;should&gt;
work.

<a id="snescartgsuncpujmpandprefixopcodes"></a>

## SNES Cart GSU-n CPU JMP and Prefix Opcodes

**GSU Special Opcodes**

```text
  Opcode     Clks Flags   Native    Nocash
  00            1 000---- STOP      stop  ;SFR.GO=0, SFR.IRQ=1, R15=$+2
  01            1 000---- NOP       nop   ;NOP (often used as dummy after jump)
  02           1* 000---- CACHE     cache ;IF CBR<>PC&FFF0 then CBR=PC&FFF0
```

STOP at $+0 does prefetch another opcode byte at $+1 (but without executing
it), and does then stop with R15=$+2, SFR.GO=0, SFR.IRQ=1 (that, even if IRQ is
disabled in CFGR.IRQ).

BUG: On MC1 (maybe also GSU1), STOP hangs when executed after a RAM write
(there must be at least 2 cycles after write, eg. insert two NOPs before STOP;
the required delay might vary depending on CPU speed or code cache? the bug
doesn't occur on GSU2).

**GSU Jump Opcodes**

```text
  Opcode     Clks Flags   Native    Nocash
  05 nn         2 ------- BRA addr  jr  addr   ;Always, R15=R15+signed(nn)
  06 nn         2 ------- BGE addr  jge addr   ;If (S XOR V)=0 then ..
  07 nn         2 ------- BLT addr  jl  addr   ;If (S XOR V)=1 then ..
  08 nn         2 ------- BNE addr  jne addr   ;If ZF=0 then R15=R15+signed(nn)
  09 nn         2 ------- BEQ addr  je  addr   ;If ZF=1 then R15=R15+signed(nn)
  0A nn         2 ------- BPL addr  jns addr   ;If SF=0 then R15=R15+signed(nn)
  0B nn         2 ------- BMI addr  js  addr   ;If SF=1 then R15=R15+signed(nn)
  0C nn         2 ------- BCC addr  jnc addr   ;If CY=0 then R15=R15+signed(nn)
  0D nn         2 ------- BCS addr  jc  addr   ;If CY=1 then R15=R15+signed(nn)
  0E nn         2 ------- BVC addr  jno addr   ;If OV=0 then R15=R15+signed(nn)
  0F nn         2 ------- BVS addr  jo  addr   ;If OV=1 then R15=R15+signed(nn)
  9n            1 000---- JMP Rn    jmp Rn     ;R15=Rn                ;n=8..13!
  3D 9n         2 000---- LJMP Rn   jmp Rn:Rs  ;R15=Rs, PBR=Rn, CBR=? ;n=8..13!
  3C            1 000-s-z LOOP    loop r12,r13 ;r12=r12-1, if Zf=0 then R15=R13
  9n            1 000---- LINK #n link r11,addr;R11=R15+n             ;n=1..4
```

Jumps can be also implemented by using R15 (PC) as destination register (eg. in
MOV/ALU commands).

Observe that the NEXT BYTE after any jump/branch opcodes is fetched before
continuing at the jump destination address. The fetched byte is executed after
the jump, but before executing following opcodes at the destination (in case of
multi-byte opcodes, this results in a 1-byte-fragment being located after the
jump-origin, and the remaining byte(s) at the destination).

**GSU Prefix Opcodes**

ALT1/ALT2/ALT3 prefixes do change the operation of an opcode, these are usually
implied in the opcode description (for example, "3F 6n" is "CMP R0,Rn").

TO/WITH/FROM prefixes allow to select source/destination registers (otherwise
R0 is used as default register) (for example, "Bs 3F 6n" is "CMP Rs,Rn").

The prefixes are normally reset after execution of any opcode, the only
exception are the Bxx (branch) opcodes, these leave prefixes unchanged
(allowing to "split" opcodes, for example placing ALT1/TO/etc. before Bxx, and
the next opcode byte after Bxx).

Aside from setting Sreg+Dreg, WITH does additionally set the B-flag, this
causes any following 1nh/Bnh bytes to act as MOVE/MOVES opcodes (rather than as
TO/FROM prefixes).

```text
  Opcode Clks Flags   Name    Bflg ALT1 ALT2 Rs Rd
  3D        1 -1----- ALT1     -    1    -   -  -  ;prefix for 3D xx opcodes
  3E        1 --1---- ALT2     -    -    1   -  -  ;prefix for 3E xx opcodes
  3F        1 -11---- ALT3     -    1    1   -  -  ;prefix for 3F xx opcodes
  1n        1 ------- TO Rn    -    -    -   -  Rn ;select Rn as Rd
  2n        1 1------ WITH Rn  1    -    -   Rn Rn ;select Rn as Rd & Rs
  Bn        1 ------- FROM Rn  -    -    -   Rn -  ;select Rn as Rs
  05..0F nn 2 ------- Bxx addr -    -    -   -  -  ;branch opcodes (no change)
  other    .. 000---- other    0    0    0   R0 R0 ;other opcodes (reset all)
```

Other opcodes do reset B=0, ALT1=0, ALT2=0, Sreg=R0, Dreg=R0; that does really
apply to ALL other opcodes, namely including JMP/LOOP (unlike Bxx branches),
NOP (ie. NOP isn't exactly &lt;no&gt; operation), MOVE/MOVES (where 1n/Bn are
treated as 'real' opcodes rather than as TO/FROM prefixes).

**Ignored Prefixes**

ALT1/ALT2 prefixes are ignored if the opcode doesn't exist (eg. if "3D xx"
doesn't exist, then the CPU does instead execute "xx") (normally, doing that
wouldn't make any sense, however, "Doom" is using ALT1/ALT2 alongside with
conditional jumps, resulting in situations where the prefix is used/ignored
depending on the jump condition).

ALT3 does reportedly mirror to ALT1 (eg. if "3F xx" doesn't exist, then it acts
as "3D xx", and, if that doesn't either, as "xx").

TO/WITH/FROM are ignored if the following opcode doesn't use Dreg/Sreg.

**Program Counter (R15) Notes**

R15 can be used as source operand in MOV/ALU opcodes (and is also implied as
such in Bxx,LINK,CACHE opcodes); in all cases R15 contains the address of the
next opcode.

<a id="snescartgsuncpupseudoopcodes"></a>

## SNES Cart GSU-n CPU Pseudo Opcodes

**Official GSU Pseudo/Macro Opcodes**

```text
  --            3 000---- LEA Rn,yyxx    ;Alias for IWT, without "#"
  --            - 000---- MOVE Rn,#hilo  ;Alias for IBT/IWT (depending on size)
  --            - 000---- MOVE Rn,(xx)   ;Alias for LM/LMS (depending on size)
  --            - 000---- MOVE (xx),Rn   ;Alias for SM/SMS (depending on size)
  --            - 000---- MOVEB Rn,(Rm)  ;Alias for LDB/TO+LDB (depending Rn)
  --            - 000---- MOVEB (Rm),Rn  ;Alias for STB/FROM+STB
  --            - 000---- MOVEW Rn,(Rm)  ;Alias for LDW/TO+LDW (depending Rn)
  --            - 000---- MOVEW (Rm),Rn  ;Alias for STW/FROM+STW
```

Above are official pseudo opcodes for native syntax (the nocash syntax "MOV"
opcode is doing that things by default).

**Nocash GSU Pseudo Opcodes**

```text
   jmp  nnnn      alias for "mov r15,nnnn"
   jz/jnz/jae/jb  alias for "je/jne/jc/jnc"
```

Further possible pseudo opcodes (not yet supported in a22i):

```text
   push rs        mov [r10],rs, 2xinc_r10     ;\INCREASING on PUSH? or MEMFILL?
   pop  rd        2xdec_r10, mov rd,[r10]     ;/ (see Star Fox 1:ACA4)
   cmp  rn,0      alias for "sub rn,rn,0"
   call           alias for link+jmp
   ret            alias for jmp r11
   alu  rd,op     short for "alu rd,rs,op"
   and  rd,rs,n   alias for "bic rd,rs,not n"
```

<a id="snescartgsuncpumisc"></a>

## SNES Cart GSU-n CPU Misc

**Uncached ROM/RAM-Read-Timings**

```text
  ROM Read:   5 cycles per byte at 21MHz, or 3 cycles per byte at 10MHz
  RAM Write: 10 cycles per word at 21MHz, or unknown at 10MHz?
  RAM Write: unknown number of cycles per byte?
  ROM/RAM Opcode-byte-read: 3 cycles at both 21MHz and 10MHz?
```

The uncached timings aren't well documented. Possibly ROM/RAM-byte read/write
are all having the same timing (3/5 clks at 10/21MHz) (and RAM-word 6/10)?

**Jump Notes**

Jumps can be implemented by JMP/Bxx opcodes, or by using R15 as destination
register. In all cases, the next BYTE after the jump opcode is fetched as
opcode byte, and is executed before continuing at the jump-target address.
Possible situations are:

```text
  1) jump + NOP                 ;very simple
  2) jump + ONE-BYTE-OPCODE     ;still quite simple
  3) jump + MULTI-BYTE-OPCODE   ;rather strange
  4) Prefix + jump + ONE-BYTE-SUFFIX
  5) Prefix + jump + MULTI-BYTE-SUFFIX
```

In case 3, the first opcode-byte is picked from the address after jump, the
following byte(s) from the jump-destination.

In case 4/5, the prefix is located before the jump, the next byte after the
jump (this works only with Bxx jumps) (whilst JMP/LJMP or MOV/ALU R15,dest do
reset the prefix), and any further bytes at the jump-destination.

**Mistakes in book2.pdf**

BGE/BLT are exchanged with each other. MOVES src/dst operands are exchanged.
LJMP bank/offs operands are exchanged.

**GSU Undoc opcodes**

UMULT #n, WITH, XOR Rn, XOR #n are sorts of undocumented; they should be
described (on page 280), but the alphabetical list ends abruptly after UMULT
Rn. However, they are listed in the summary (page 101) and in the index (page
409). The WITH opcode is also mentioned in various other places.

page 121: R15 after STOP (strange, is that true?) (yes, it is)

page 122: cache/cbr after ABORT

MOV R13,R15  sets R13 to addr of next opcode after MOV (eg. for LOOP start)

LINK n       sets R11 to addr+n of next opcode (eg. for "CALLs" via jmp)

**GSU Power Consumption**

The GSU does (when it is running) increase the power consumption, this can
overload the SNES power supply if additional peripherals are connected. GSU
software should detect which controllers are connected, and refuse to start the
GSU if a controller with high power consumption (or with unknown power
consumption) is connected. The standard joypads are okay. A Multiplayer 5
adaptor isn't okay (at least, when multiple controllers are connected to it).

**After STOP**

Restarting (somewhere(?) after STOP) is possible by setting GO-flag (done by
Dirt Trax FX).

<a id="snescartgsuncodecache"></a>

## SNES Cart GSU-n Code-Cache

**ROM/RAM-Code-Cache (512-byte cache)**

This cache is used only for Opcode fetches from ROM or RAM (not for
reading/writing Data to/from ROM nor RAM) (however, it does slightly increase
data access speed in so far that data can be read/written via Gamepak bus,
simultaneously while fetching opcodes from the cache).

32 lines of 16-bytes.

CACHE

LJMP

STOP

ABORT

after STOP, one "must" clear GO by software to clear the cache

**Cache Area**

"SNES_Addr = (CBR AND 1FFh)+3100h". For example, a CACHE opcode at C3A5h will
set CBR to C3A0h, and the (initially empty) cached region will be C3A0h..C59Fh,
when code gets loaded into the cache, GSU:C3A0h..C3FFh shows up at
SNES:32A0h..32FFh, and GSU:C400h..C59Fh at SNES:3100h..329Fh.

**Writing to Code-Cache (by SNES CPU)**

First of, set GO=0 (ie. write SFR=0000h), this forces CBR=0000h, and marks all
cache lines as empty. Then write opcodes 16-byte lines at 3100h..32FFh, writing
the last byte of a line at [3xxFh] will mark the line as not-empty.

Thereafter, the cached code can be excuted (by setting R15 to 0000h..01Fxh), in
this case, the GSU can be operated without RON/RAN flags being set - unless R15
leaves the cached area (this occurs also when a STOP is located in last byte of
last cache line; the hardware tries to prefetch one byte after STOP), or unless
ROM-DATA is accessed (via GETxx opcodes) or unless RAM-DATA is accessed (via
LOAD/STORE or PLOT/RPIX opcodes). Ie. usually one would have RAN set (unless
all incoming/outgoing parameters can be passed though R0..R14 registers).

**Code-Cache Loading Notes**

The 16-byte cache-lines are loaded alongside while executing opcodes (rather
than first loading the whole 16-byte-line, and then executing the opcodes
within it; which would be slightly slower). There are two special cases related
to jumps: If current cache-line isn't fully loaded then hardware keeps loading
the remaining bytes (from jump-origin to end-of-line). If the jump-target isn't
aligned by 16 (and isn't yet cached), then the hardware loads the leading bytes
(from start-of-line to jump-target). After that two steps, normal execution
continues at the jump-target address.

The leading-stuff also occurs on CACHE instruction.

```text
  CACHE sets CBR to "R15 AND FFF0h" (whereas R15=address after CACHE opcode)
  LJMP sets CBR to "R15 AND FFF0h" (whereas R15=jump target address)
  SNES write to SFR register with GO=0 sets CBR=0000h
  (all of the above three cases do also mark all cache lines as empty)
```

All Code-Cache lines are marked as empty when executing CACHE or LJMP opcodes,
or when the SNES clears the GO flag (by writing to SFR). The STOP opcode
however (which also clears GO), doesn't empty the cache, so one may eventually
re-use the cached values when restarting the GSU (however, if PBR or code in
GamePak RAM has changed, then one must clear the cache by writing GO=0).

According to cache description (page 132), Cache-Code is 6 times faster than
ROM/RAM. However, according to opcode descriptions (page 160 and up), cache is
only 3 times faster than ROM/RAM. Whereas, maybe 6 times refers to 21MHz mode,
and 3 times to 10MHz mode?

The CACHE opcode is typically executed prior to loops and/or at the begin of
often-used sub-functions (or ideally, the loop and any used subfunctions should
both fit into the 512-byte cache region).

<a id="snescartgsunpixelcache"></a>

## SNES Cart GSU-n Pixel-Cache

**RAM-Pixel-Write-Cache (two 8-pixel rows)**

pixel cache is flushed when:

```text
  1) cache full
  2) doing rpix <--- this does also WAIT until it is flushed
  3) changing r1 or r2 (really?)
```

**Pixel Cache**

Primary Pixel Cache (written to by PLOT)

Secondary Pixel Cache (data copied from Primary Cache, this WAITs if Secondary
cache wasn't yet forwarded to RAM) (if less than 8 flags are set, data is
merged with old RAM data).

Each cache contains 8 pixels (with 2bit/4bit/8bit depth), plus 8 flags
(indicating if (nontransparent) pixels were plotted).

Pixel X/Y coordinates are 8bit wide (using LSBs of R1/R2 registers).

(X and F8h) and (Y and FFh) are memorized, when plotting to different values,
Primary cache is forwarded to Secondary Cache, this happens also when all 8
cache flags are set.

Do not change SCREEN MODE (how to do that at all while GSU is running? SCMR is
writeable for changing RAN/RON, but changing the other SCMR bits during GSU
execution would be rather unpredictable) when data is in pixel caches (use RPIX
to force the caches to be flushed). Before STOP opcode, do also use RPIX to
force the caches to be flushed.

RPIX isn't cached, it does always read data from RAM, not from cache. Moreover,
before reading RAM, RPIX does force both pixel caches (unless they are empty)
to be forwarded to RAM. This is making RPIX very slow (trying to read/modify
pixels via RPIX+PLOT would work very slow). So far, RPIX is mainly useful for
forcing the pixel caches to be forwarded to RAM (and to WAIT until that
forwarding has completed).

<a id="snescartgsunothercaches"></a>

## SNES Cart GSU-n Other Caches

**ROM-Read-Data Cache (1-byte read-ahead)**

The cache is used for GETB/GETBS/GETBL/GETBH/GETC opcodes (which do read from
[ROMBR:R14]). Loading the cache is invoked by any opcodes that do change R14
(such like ADD,MOVE,etc.), allowing following GETxx opcodes to be executed
without Waitstates.

In some situations WAITs can occur: When the cache-load hasn't yet completed
(ie. GETxx executed shortly after changing R14), when an opcode is fetched from
ROM (rather than from RAM or Code-Cache), when ROMBR is changed (caution: in
this special case following GETxx will receive [OldROMBR:R14] rather than
[NewROMBR:R14]).

Caution: Do not execute the CACHE opcode shortly (7 cycles in 21MHz mode, or 4
cycles in 10MHz mode) after changing R14 (when doing that, the read from R14
will fail somehow, and following GETxx will return garbage).

Unknown if SNES writes to R14 (via Port 301Ch) do also prefetch [R14] data?

**RAM-Write-Data Cache (1-byte/1-word write queue)**

This cache is used for STB/STW/SM/SMS/SBK opcodes. After any such store
opcodes, the written byte/word is memorized in the cache, and further opcodes
can be fetched (from ROM or from Code-Cache) immediately without Waitstates,
simultaneously with the cached value being forwarded to RAM.

In some situations WAITs can occur: When cache already contained data (ie. when
executing two store opcodes shortly after each other), when an opcode is
fetched from RAM (rather than from ROM or Code-Cache), when the RAMBR register
is changed (this works as expected, it finishes the write to [OldRAMBR:nnnn]).

Results on doing Data-RAM-Reads while the Data-RAM-write is still busy are
unknown (possibly, this will WAIT, too) (or it may return garbage)?

WAITs should also occur when the pixel-cache gets emptied?

**RAM-Address-Cache (1 word) (Bulk Processing for read-modify-write)**

This very simple cache memorizes the most recently used RAM address (from
LM/LMS opcodes, and probably also from LDB/LDW/STB/STW/SM/SMS opcodes; though
some games insert STW to push data on stack, as if they were intended not to
change the memorized address?), the SBK opcode can be used to write a word to
the memorized address (ie. one can avoid repeating immediate operands in SM/SMS
opcodes).

<a id="snescartcapcomcx4programmablerisccpumegamanx232games"></a>

## SNES Cart Capcom CX4 (programmable RISC CPU) (Mega Man X 2-3) (2 games)

[SNES Cart Capcom CX4 - I/O Ports](#snes-cart-capcom-cx4-io-ports)

[SNES Cart Capcom CX4 - Opcodes](#snes-cart-capcom-cx4-opcodes)

[SNES Cart Capcom CX4 - Functions](#snes-cart-capcom-cx4-functions)

[SNES Pinouts CX4 Chip](80-timings-unpredictable-pinouts.md#snes-pinouts-cx4-chip)

**Capcom CX4 - 80pin chip**

Used only by two games:

```text
  Mega Man X2 (1994) Capcom (NA) (JP) (EU)   ;aka Rockman X2
  Mega Man X3 (1995) Capcom (NA) (JP)
```

The CX4 chip is actually a Hitachi HG51B169 as confirmed by decapping.

Note: The CX4 is occassionally referred to as C4 (the real chip name is CX4,
the C4 variant is some kind of scene slang).

**CX4 Memory Map**

```text
  I/O  00-3F,80-BF:6000-7FFF
  ROM  00-3F,80-BF:8000-FFFF
  SRAM 70-77:0000-7FFF (not installed; reads return 00h)
```

**MISC MISC MISC**

Commands are executed on the CX4 by writing the command to 0x7F4F while bit 6
of 0x7F5E is clear. Bit 6 of 0x7F5E will stay set until the command has
completed, at which time output data will be available.

[Registers]

```text
  $7f49-b = ROM Offset
  $7f4d-e = Page Select
  $7f4f = Instruction Pointer
  Start Address = ((Page_Select * 256) + Instruction Pointer) * 2) + ROM_Offset
```

[Memory layout]

```text
 Program ROM is obviously 256x16-bit pages at a time. (taken from the SNES ROM)
 Program RAM is 2x256x16-bit. (two banks)    ;<-- uh, that means cache?
 Data ROM is 1024x24-bit. (only ROM internal to the Cx4)
 Data RAM is 4x384x16-bit.                   ;<-- uh, but it HAS 8bit data bus?
 Call stack is 8-levels deep, at least 16-bits wide.
```

**CX4ROM (3Kbytes) (1024 values of 24bit each)**

```text
  Index      Name  ;Entry     = Table Contents   = Formula
  -------------------------------------------------------------------------
  000..0FFh  Div   ;N[0..FFh] = FFFFFFh..008080h = 800000h/(00h..FFh)
  100..1FFh  Sqrt  ;N[0..FFh] = 000000h..FF7FDFh = 100000h*Sqrt(00h..FFh)
  200..27Fh  Sin   ;N[0..7Fh] = 000000h..FFFB10h = 1000000h*Sin(0..89')
  280..2FFh  Asin  ;N[0..7Fh] = 000000h..75CEB4h = 800000h/90'*Asin(0..0.99)
  300..37Fh  Tan   ;N[0..7Fh] = 000000h..517BB5h = 10000h*Tan(0..89')
  380..3FFh  Cos   ;N[0..7Fh] = FFFFFFh..03243Ah = 1000000h*Cos(0..89')
```

Sin/Asin/Tan/Cos are spanning only 90' out of 360' degress (aka 80h out of 200h
degrees). Overflows on Div(0) and Cos(0) are truncated to FFFFFFh. All values
are unsigned, and all (except Asin/Tan) are using full 24bits (use SHR opcode
to convert these to signed values with 1bit sign + 23bit integer; for Div one
can omit the SHR if divider&gt;01h).

**CX4 Component List (Megaman X2)**

```text
  PCB "SHVC-2DC0N-01, (C)1994 Nintendo"
  U1 32pin P0 8M MASK ROM  (LH538LN4 = 8Mbit)
  U2 32pin P1 4/8 MASK ROM (LH534BN2 or LH5348N2 or so = 4Mbit)
  U3 80pin CX4 (CAPCOM CX4 DL-2427, BS169FB)
  U4 18pin CIC (F411A)
  X1  2pin 20MHz
  J  62pin Cart Edge connector (unknown if any special pins are actually used)
```

**CX4 Component List (Megaman X3)**

```text
  PCB "SHVC-1DC0N-01, (C)1994 Nintendo"
  U1 40pin MASK ROM  (TC5316003CF = 16Mbit)
  U2 80pin CX4 (CAPCOM CX4 DL-2427, BS169FB)
  U3 18pin CIC (F411A)
  X1  2pin 20MHz
  J  62pin Cart Edge connector (unknown if any special pins are actually used)
```

**CX4 Cartridge Header (as found in Mega Man X2/X3 games)**

```text
  [FFBD]=00h ;expansion RAM size (none) (there is 3KB cx4ram though)
  [FFBF]=10h ;CustomChip=CX4
  [FFD5]=20h ;Slow LoROM (but CX4 opcodes are probably using a faster cache)
  [FFD6]=F3h ;ROM+CustomChip (no battery, no sram)
  [FFD7]=0Bh ;rom size (X2: 1.5MB, rounded-up to 2MB) (X3: real 2MB)
  [FFD8]=00h ;sram size (none) (there is 3KB cx4ram though)
  [FFDA]=33h ;Extended Header (with FFB0h-FFBFh)
```

**ROM Enable**

On SHVC-2DC0N-01 PCBs (ie. PCBs with two ROM chips), the 2nd ROM chip is
reportedly initially disabled, and can be reportedly enabled by setting
[7F48h]=01h (that info doesn't match up with how 7F48h is used by the existing
games; unknown if that info is correct/complete).

**CX4 CPU Misc**

All values are little-endian (opcodes, I/O Ports, cx4rom-ROM-Image, etc).

Call Stack is reportedly 16 levels deep, at least 16bits per level.

Carry Flag is CLEARED on borrow (ie. opposite as on 80x86 CPUs).

**CX4 Timings (Unknown)**

All opcode &amp; DMA timings are 100% unknown. The CX4 is said to be clocked at
20.000MHz, but this might be internally divided, possibly with different
waitstates for different memory regions or different opcodes.

The ROM speed is 2.68Mhz (according to the cartridge header), and 16bit opcodes
are passed through 8bit databus (though one may assume that the CX4 contains an
opcode cache) (cache might be divided into 200h-byte pages, so, far-jumps to
other pages might be slow, maybe/guessed).

The "skip" opcodes are "jumping" to the location after the next opcode (this
probably faster than the actual "jmp" opcodes).

After Multiply opcodes one should insert one "nop" (or another instruction that
doesn't access the MH or ML result registers).

Reading data bytes from SNES ROM requires some complex timing/handling:

```text
  612Eh   movb   ext_dta,[ext_ptr]          ;\these 3 opcodes are used to
  4000h   inc    ext_ptr                    ; read one byte from [ext_ptr],
  1C00h   finish ext_dta                    ;/and to increment ext_ptr by 1
```

The exact meaning of the above opcodes is unknown (which one does what part?).

It is also allowed to use the middle opcode WITHOUT the "prepare/wait" part:

```text
  4000h   inc    ext_ptr                    ;-increment ext_ptr by 1
```

In that case, "ext_ptr" is incremented, but "ext_dta" should not be used (might
be unchanged, or contain garbage, or receive data after some cycles?).

<a id="snescartcapcomcx4ioports"></a>

## SNES Cart Capcom CX4 - I/O Ports

**CX4 I/O Map**

```text
  6000h..6BFFh R/W  CX4RAM (3Kbytes)
  6C00h..7F3Fh ?    Unknown/unused
  7F40h..7F42h ?/W  DMA source, 24bit SNES LoROM address
  7F43h..7F44h ?/W  DMA length, 16bit, in bytes (eg. 0800h = 2Kbytes)
  7F45h..7F46h ?/W  DMA destination, 16bit in CX4RAM (6000h = 1st byte)
  7F47h        ?/W  DMA start (write 00h to transfer direction SNES-to-CX4)
  7F48h        ?/W  Unknown "toggle" (set to 00h/01h, maybe cache load/on/off?)
  7F49h..7F4Bh R/W  Program ROM Base, 24bit LoROM addr (028000h in Mega Man)
  7F4Ch        ?/W  Unknown (set to 00h or 01h) soft_reset? maybe flush_cache?
  7F4Dh..7F4Eh ?/W  Program ROM Instruction Page (PC/200h)
  7F4Fh        ?/W  Program ROM Instruction Pointer (PC/2), starts execution
  7F50h..7F51h R/W  Unknown, set to 0144h (maybe config flags or waitstates?)
  7F52h        R/W  Unknown (set to 00h) hard_reset? maybe force stop?
  7F53h..7F5Dh ?    Unknown/unused
  7F5Eh        R/?  Status (bit6=busy, set upon [7F47],[7F48],[7F4F] writes)
  7F5Fh        ?    Unknown/unused
  7F60h..7F69h ?    Unknown/unused (maybe [FFE0..FFE9])
  7F6Ah..7F6Bh R/W  SNES NMI Vector       [FFEA..FFEB]
  7F6Ch..7F6Dh ?    Unknown/unused (maybe [FFEC..FFED])
  7F6Eh..7F6Fh R/W  SNES IRQ Vector       [FFEE..FFEF]
  7F70h..7F7Fh ?    Unknown/unused (maybe [FFF0..FFFF])
  7F80h..7FAFh R/W  Sixteen 24bit CX4 registers (R0..R15, at 7F80h+N*3)
  7FB0h..7FFFh ?    Unknown/unused
  8000h..FFFFh R    ROM (32Kbyte LoROM Banks) (disabled when CX4 is busy)
  FFExh..FFxxh R/?  Exception Vectors (from above I/O Ports, when CX4 is busy)
```

**Exception Vectors**

Unknown if these can be manually enabled, or if they are automatically enabled
when the CX4 is "busy". In the latter case, they would be REQUIRED to be same
as the ROM vectors (else LSB/MSB might be accidently fetched from different
locations when busy-flag changes at same time).

<a id="snescartcapcomcx4opcodes"></a>

## SNES Cart Capcom CX4 - Opcodes

**CX4 Opcodes (all are 16bit wide)**

```text
  Opcode         Clks NZC Syntax
  0000h            ?? ??? nop     ;nop is used as delay after "mul" opcodes
  0400h            ?? ??? -
  0800h+p0aaaaaaaa ?? ??? jmp   addr/prg_page:addr
  0C00h+p0aaaaaaaa ?? ??? jz    addr/prg_page:addr  ;Z=1 (equal)
  1000h+p0aaaaaaaa ?? ??? jc    addr/prg_page:addr  ;C=1 (above/equal)
  1400h+p0aaaaaaaa ?? ??? js    addr/prg_page:addr  ;N=1 (negative)
  1800h            ?? ??? -
  1C00h            ?? ??? finish ext_dta
  2000h            ?? ??? -
  2400h+nn0000000n ?? ??? skip<?/?/nc/c/nz/z/ns/s>  ;skip next opcode
  2800h+p0aaaaaaaa ?? ??? call  addr/prg_page:addr
  2C00h+p0aaaaaaaa ?? ??? callz addr/prg_page:addr  ;Z=1 (equal)
  3000h+p0aaaaaaaa ?? ??? callc addr/prg_page:addr  ;C=1 (above/equal)
  3400h+p0aaaaaaaa ?? ??? calls addr/prg_page:addr  ;N=1 (negative)
  3800h            ?? ??? -
  3C00h            ?? ??? ret
  4000h            ?? ??? inc   ext_ptr
  4400h            ?? ??? -
  4800h+ssoooooooo ?? ??? cmp   <op>,A/A*2/A*100h/A*10000h     ;\
  4C00h+ssoooooooo ?? ??? cmp   <imm>,A/A*2/A*100h/A*10000h    ; compare
  5000h+ssoooooooo ?? NZC cmp   A/A*2/A*100h/A*10000h,<op>     ;
  5400h+ssoooooooo ?? NZC cmp   A/A*2/A*100h/A*10000h,<imm>    ;/
  5800h+ss00000000 ?? ??? mov   A,A.?/lsb/lsw/?                ;-sign-expand
  5C00h            ?? ??? -
  6000h+nnoooooooo ?? ??? mov   A/ext_dta/?/prg_page,<op>
  6400h+nnoooooooo ?? ??? mov   A/?/?/prg_page,<imm>
  6800h+nnoooooooo ?? ??? movb  ram_dta.lsb/mid/msb/?,cx4ram[<op>]
  6C00h+nnoooooooo ?? ??? movb  ram_dta.lsb/mid/msb/?,cx4ram[ram_ptr+<imm>]
  7000h+00oooooooo ?? ??? mov   rom_dta,cx4rom[<op>*3]
  7400h            ?? ??? -
  7800h+0noooooooo ?? ??? mov   prg_page.lsb/msb,<op>
  7C00h+0noooooooo ?? ??? mov   prg_page.lsb/msb,<imm>
  8000h+ssoooooooo ?? ??C add   A,A/A*2/A*100h/A*10000h,<op>   ;\
  8400h+ssoooooooo ?? ?Z? add   A,A/A*2/A*100h/A*10000h,<imm>  ;
  8800h+ssoooooooo ?? ??? sub   A,<op>,A/A*2/A*100h/A*10000h   ; add/subtract
  8C00h+ssoooooooo ?? ??C sub   A,<imm>,A/A*2/A*100h/A*10000h  ;
  9000h+ssoooooooo ?? NZC sub   A,A/A*2/A*100h/A*10000h,<op>   ;
  9400h+ssoooooooo ?? NZC sub   A,A/A*2/A*100h/A*10000h,<imm>  ;/
  9800h+00oooooooo ?? ??? smul  MH:ML,A,<op>    ;\use NOP or other opcode,
  9C00h+00oooooooo ?? ??? smul  MH:ML,A,<imm>   ;/result is signed 48bit
  A000h            ?? ??? -
  A400h            ?? ??? -
  A800h+ssoooooooo ?? ??? xor   A,A/A*2/A*100h/A*10000h,<op>   ;\
  AC00h+ssoooooooo ?? ??? xor   A,A/A*2/A*100h/A*10000h,<imm>  ;
  B000h+ssoooooooo ?? ?Z? and   A,A/A*2/A*100h/A*10000h,<op>   ; logic
  B400h+ssoooooooo ?? ?Z? and   A,A/A*2/A*100h/A*10000h,<imm>  ;
  B800h+ssoooooooo ?? ??? or    A,A/A*2/A*100h/A*10000h,<op>   ;
  BC00h+ssoooooooo ?? ??? or    A,A/A*2/A*100h/A*10000h,<imm>  ;/
  C000h+00oooooooo ?? ??? shr   A,<op>                         ;\
  C400h+00oooooooo ?? NZ? shr   A,<imm>                        ;
  C800h+00oooooooo ?? ??? sar   A,<op>                         ;
  CC00h+00oooooooo ?? N?? sar   A,<imm>                        ; shift/rotate
  D000h+00oooooooo ?? ??? ror   A,<op>                         ;
  D400h+00oooooooo ?? ??? ror   A,<imm>                        ;
  D800h+00oooooooo ?? ??? shl   A,<op>                         ;
  DC00h+00oooooooo ?? N?? shl   A,<imm>                        ;/
  E000h+00oooooooo ?? ??? mov   <op>,A
  E400h            ?? ??? -
  E800h+nnoooooooo ?? ??? movb  cx4ram[<op>],ram_dta.lsb/mid/msb/?
  EC00h+nnoooooooo ?? ??? movb  cx4ram[ram_ptr+<imm>],ram_dta.lsb/mid/msb/?
  F000h+00oooooooo ?? ??? xchg  <op>,A
  F400h            ?? ??? -
  F800h            ?? ??? -
  FC00h            ?? ??? stop          ;stop, and clear Port [FF5E].bit6
```

**Opcode "Middle" 2bits (Bit9-8)**

Selects different parameters for some opcodes (eg. lsb/mid/msb, as shown in
above descriptions).

**Opcode Lower 8bits (Bit7-0)**

Lower Bits &lt;op&gt;:

```text
  00h Register A
  01h Register MH       ;multiply.result.upper.24bit (MSBs are sign-expanded)
  02h Register ML       ;multiply.result.lower.24bit (same for signed/unsigned)
  03h Register ext_dta
  08h Register rom_dta
  0Ch Register ram_dta
  13h Register ext_ptr  ;24bit SNES memory address
  1Ch Register ram_ptr
  2Eh Special  snesrom[ext_ptr] (?)  ;for use by opcode 612Eh only (?)
  50h Constant 000000h
  51h Constant FFFFFFh
  52h Constant 00FF00h
  53h Constant FF0000h
  54h Constant 00FFFFh
  55h Constant FFFF00h
  56h Constant 800000h
  57h Constant 7FFFFFh
  58h Constant 008000h
  59h Constant 007FFFh
  5Ah Constant FF7FFFh
  5Bh Constant FFFF7Fh
  5Ch Constant 010000h
  5Dh Constant FEFFFFh
  5Eh Constant 000100h
  5Fh Constant 00FEFFh
  6xh Register R0..R15, aka Port [7F80h+x*3] ;(x=0h..Fh)
```

Lower Bits &lt;imm&gt;:

```text
  nnh Immediate 000000h..0000FFh (unsigned)
```

Lower Bits jump/call&lt;addr&gt;:

```text
  nnh Program Counter LSBs (within 256-word page) (absolute, non-relative)
```

Lower Bits skip&lt;cond&gt;:

```text
  00h Skip next opcode if selected flag is zero (conditions ?/nc/nz/ns)
  01h Skip next opcode if selected flag is set  (conditions ?/c/z/s)
```

Lower Bits for opcodes that don't use them (uuuuuuuu):

```text
  00h Unused, should be zero
```

<a id="snescartcapcomcx4functions"></a>

## SNES Cart Capcom CX4 - Functions

**CX4 Functions (as contained in Mega Man X2/X3 ROMs)**

The CX4 functions are located at SNES address 02:8000-02:9FFF (aka CX4
addresses at PAGE:PC=0000:00..000F:FF with BASE=028000):

```text
  PAGE:PC__Function_____________________________
  0000:00  build_oam
  0001:00  scale_tiles  ;<-- (seems to be unused by Mega Man games)
  0002:00  hires_sqrt   ;<-- (seems to be unused by Mega Man games)
  0002:03  sqrt         ;<-- (seems to be unused by Mega Man games)
  0002:05  propulsion
  0002:07  get_sin      ;<-- (seems to be unused by Mega Man games)
  0002:0A  get_cos      ;<-- (seems to be unused by Mega Man games)
  0002:0D  set_vector_length
  0002:10  triangle1
  0002:13  triangle2
  0002:15  pythagorean
  0002:1F  arc_tan
  0002:22  trapeziod
  0002:25  multiply
  0002:2D  transform_coordinates
  0003:00  scale_rotate1
  0005:00  transform_lines
  0007:00  scale_rotate2
  0008:00  draw_wireframe_without_clearing_buffer
  0008:01  draw_wireframe_with_clearing_buffer
  000B:00  disintergrate
  000C:00  wave
  000E:00  test_set_r0_to_00h ;\sixteen 4-word functions,
  ...      ...                ; located at 000E:00+4*(0..15)
  000E:3C  test_set_r0_to_0Fh ;/setting R0 to 00h..0Fh
  000E:40  test_2K_ram_chksum
  000E:54  test_square           ;R1:R2 = R0*R0
  000E:5C  test_immediate_register  ;copy 16 cpu constants to 30h-bytes RAM
  000E:89  test_3K_rom_chksum  ;"immediate_rom"
```

Both Mega Man X2 and X3 are containing 1:1 the same CX4 code (the only two
differences are different ROM bank numbers for "Wireframe" vertices):

```text
  Mega Man X2:  [0008:3B]="or a,a,28h"  [000A:C4]="mov a,28h"  ;ROM bank 28h
  Mega Man X3:  [0008:3B]="or a,a,08h"  [000A:C4]="mov a,08h"  ;ROM bank 08h
```

That differences apply to the US/Canada versions. There &lt;might&gt; be
further differences in Japanese and/or European versions(?)

<a id="snescartdspnst010st011preprogrammednecupd77c25cpu23games"></a>

## SNES Cart DSP-n/ST010/ST011 (pre-programmed NEC uPD77C25 CPU) (23 games)

**Nintendo DSP-n Chips**

The DSP-n chips are 28pin NEC uPD77C25 CPUs with internal ROM/RAM. There are
six versions:

```text
  DSP-1, DSP-1A, DSP-1B, DSP-2, DSP-3, DSP-4
```

DSP-1 and DSP-1A contain exactly the same Program/Data ROM. DSP-1B contains a
bug-fixed DSP1/1A version. DSP2/3/4 contain custom ROMs.

**Seta ST010/ST011 Chips**

These are 64pin chips, containing a slightly extended NEC uPD77C25 with more
ROM and RAM, faster CPU clock.

```text
  64pin  SETA ST010 D96050CW-012 (PCB SHVC-1DS0B-01)
  64pin  SETA ST011 D96050CW-013 (PCB SHVC-1DS0B-10; with extra transistor)
```

The onchip RAM is battery-backed and is accessible directly via SNES address
bus.

**NEC uPD77C25 Specs**

[SNES Cart DSP-n/ST010/ST011 - NEC uPD77C25 - Registers &amp; Flags &amp; Overview](#snes-cart-dsp-nst010st011-nec-upd77c25-registers-flags-overview)

[SNES Cart DSP-n/ST010/ST011 - NEC uPD77C25 - ALU and LD Instructions](#snes-cart-dsp-nst010st011-nec-upd77c25-alu-and-ld-instructions)

[SNES Cart DSP-n/ST010/ST011 - NEC uPD77C25 - JP Instructions](#snes-cart-dsp-nst010st011-nec-upd77c25-jp-instructions)

**Game specific info**

[SNES Cart DSP-n/ST010/ST011 - List of Games using that chips](#snes-cart-dsp-nst010st011-list-of-games-using-that-chips)

[SNES Cart DSP-n/ST010/ST011 - BIOS Functions](#snes-cart-dsp-nst010st011-bios-functions)

**DSPn/ST010/ST011 Cartridge Header**

For DSPn Cartridges:

```text
  [FFD6h]=03h..05h   Chipset = DSPn (plus battery present/absent info)
```

For ST010/ST011 Cartridges:

```text
  [FFD6h]=F6h   Chipset = Custom (plus battery; for the on-chip RAM)
  [FFD4h]=00h   Last byte of Title=00h (indicate early extended header)
  [FFBFh]=01h   Chipset Sub Type = ST010/ST011
```

Note: The uPD77C25's ROM/RAM aren't counted in the ROM Size, ROM Checksum, SRAM
Size (nor Expansion RAM Size) entries. The header (nor extended header)
includes no info whether a DSPn game uses a DSP1, DSP2, DSP3, or DSP4, and no
info if a ST010/ST011 game uses ST010 or ST011. Ideally, the uPD77C25 ROM-Image
should be appended at the end of the SNES ROM-Image. In practice, it's often
not there, so there's no way to detect if the game uses this or that uPD77C25
ROM (except for using a list of known Titles or Checksums).

<a id="snescartdspnst010st011necupd77c25registersflagsoverview"></a>

## SNES Cart DSP-n/ST010/ST011 - NEC uPD77C25 - Registers & Flags & Overview

**DSP Mapping**

```text
  LoROM Mapping:
  DSP       PCB           Mode  ROM RAM Bank       Data (DR)     Status (SR)
  DSP1/DSP4 SHVC-1B0N-01  LoROM 1M  -   30h-3Fh    8000h-BFFFh   C000h-FFFFh
  DSP2      SHVC-1B5B-01  LoROM 1M  32K 20h-3Fh    8000h-BFFFh   C000h-FFFFh
  DSP3      SHVC-1B3B-01  LoROM 1M  8K  20h-3Fh    8000h-BFFFh   C000h-FFFFh
  DSP1      SHVC-2B3B-01  LoROM 2M  8K  60h-6Fh    0000h-3FFFh   4000h-7FFFh
  ST010     SHVC-1DS0B-01 LoROM 1M  -   60h-6xh    0000h         0001h
  ST011     SHVC-1DS0B-10 LoROM 512K-   60h-6xh    0000h         0001h
  HiROM Mapping:
  DSP   PCB          Mode  ROM RAM Bank            Data (DR)     Status (SR)
  DSP1  SHVC-1K0N-01 HiROM  4M   - 00h-1Fh         6000h-6FFFh   7000h-7FFFh
  DSP1  SHVC-1K1B-01 HiROM  4M  2K 00h-1Fh         6000h-6FFFh   7000h-7FFFh
  DSP1B SHVC-1K1X-01 HiROM  4M  2K 00h-0Fh,20h-2Fh 6000h-6FFFh   7000h-7FFFh
  DSP1B SHVC-2K1X-01 HiROM  2M  2K 00h-0Fh,20h-2Fh 6000h-6FFFh   7000h-7FFFh
  DSP1B SHVC-2K3X-01 HiROM  2M  8K 00h-0Fh,20h-2Fh 6000h-6FFFh   7000h-7FFFh
  SFC-Box:
  DSP   PCB          Mode  ROM RAM Bank            Data (DR)     Status (SR)
  DSP1? GS 0871-102  <Might have variable LoROM/HiROM mapping supported?>
```

Some of the above PCB names seem to be nonsense (eg. DSP with 2MbyteLoROM
hasn't ever been produced, except as prototype board).

**SNES I/O Ports**

```text
  Type                 DR               SR                SRAM
  DSPn+LoROM (1MB)     30-3F:8000-BFFF  30-3F:C000-FFFF   None
  DSPn+LoROM (1MB+RAM) 20-3F:8000-BFFF  20-3F:C000-FFFF   70-7D:0000-7FFF
  DSPn+LoROM (2MB+RAM) 60-6F:0000-3FFF  60-6F:4000-7FFF   70-7D:0000-7FFF
  DSPn+HiROM           00-1F:6000-6FFF  00-1F:7000-7FFF   20-3F:6000-7FFF ?
  DSPn+HiROM (MAD-2)   00-0F:6000-6FFF  00-0F:7000-7FFF   30-3F:6000-7FFF ?
  ST010/ST011+LoROM    60-6x:0000       60-6x:0001        68-6F:0000-0FFF
```

All banks in range 00-7F are also mirrored to 80-FF. The "LoROM (2MB)" type
wasn't actually produced (but is defined in Nintendo's specs, see book1.pdf
page 52).

For ST010/ST011, the RAM is contained in the ST01n chip, and is sized 2Kx16bit,
whereas the SNES accesses it as 4Kx8bit (even addresses accessing the LSB, odd
ones the MSB of the 16bit words).

**Registers**

```text
  DP          8-bit Data RAM Pointer               (ST010/11: 11-bit)
  RP          10-bit Data ROM Pointer              (ST010/11: 11-bit)
  PC          11-bit Program ROM Counter           (ST010/11: 14-bit)
  STACK       11-bit x 4-levels (for call/ret/irq) (ST010/11: 14-bit x 8-level)
  K,L         two 16bit registers (multiplier input)
  AccA,AccB   two 16bit registers (ALU accumulators) (aka A and B)
  FlagA,FlagB two 6bit registers with S1,S0,C,Z,OV1,OV0 flags for AccA/AccB
  TR,TRB      two 16bit registers (temporary storage)
  SR          16bit status I/O register
  DR          parallel I/O data (selectable 8bit/16bit via SR's DRC bit)
  SI,SO       serial I/O data (selectable 8bit/16bit via SR's SOC,SIC bits)
```

**FlagA/FlagB**

```text
  S0  Sign Flag     (set if result.bit15)
  Z   Zero Flag     (set if result=0000h)
  C   Carry Flag    (set if carry or borrow)
  OV0 Overflow Flag (set if result>+7FFFh or result<-8000h)
  S1  Direction of Last Overflow (if OV0 then S1=S0, else S1=unchanged)
  OV1 Number of Overflows (0=even, 1=odd) (inverted when OV0 gets set)
```

S0,Z,C,OV0 are "normal" flags as used by various CPUs. S1,OV1 are specials for
use with JSA1/JSB1/JNSA1/JNSB1 and JOVA1/JOVB1/JNOVA1/JNOVB1 conditional jump
opcodes, or with SGN operand (which equals 8000h-SA1). Examples:

```text
  or  a,a      ;SA1=A.Bit15 (undocumented)     ;\officially     ;\No Addition
  mov l,sgn    ;L=8000h-SA1 (but used by DSP1) ;/SA1=Undefined  ;/
  mov a,val1                                                    ;\
  add a,val2   ;affect OVA0 (and, if OVA0 set, also SA1)        ; Adding
  jnova0 skip0 ;test OVA0                                       ; Two Values
  mov a,sgn    ;A=8000h-SA1 (saturate max=+7FFFh, min=-8000h)   ;
 skip0:                                                         ;/
  ;below works with up to three 16bit values,                   ;\
  ;would also work with hundreds of small 8bit values,          ;
  ;ie. works if multiple overflows occur in opposite directions ;
  ;but doesn't work if two overflows occur in same direction)   ; Adding
  xor a,a      ;clear OVA1                                      ; More Values
  add a,val1   ;no overflow OVA1 yet                            ;
  add a,val2   ;this may set OVA1                               ;
  add a,val3   ;this may set/reset OVA1                         ;
  jnova1 skip1 ;test OVA1 (skip if 0 or 2 overflows occurred)   ;
  mov a,sgn    ;A=8000h-SA1 (done if 1 overflow occurred)       ;
 skip1:                                                         ;/
```

Note: The JSA1/JSB1/JNSA1/JNSB1 and JOVA1/JOVB1/JNOVA1/JNOVB1 opcodes aren't
used by any of the DSP1-DSP4 or ST010-ST011 games. The SGN operand is used only
by DSP1/DSP1A/DSP1B (once in conjuntion with JNOVA0, which seems to be ALWAYS
skipping the SGN-part, and once in conjunction with an OR opcode, which loads
an undocumented value to SA1).

**Status Register (SR)**

```text
  15    RQM (R)  Request for Master (0=Busy internally, 1=Request external I/O)
  14-13 USF1-0   User's Flags (general purpose)        (0=Low, 1=High)
  12    DRS (R)  DR Status (for 16bit DR mode; 2x8bit) (0=Ready, 1=Busy)
  11    DMA      Direct Memory Access Mode             (0=Non-DMA, 1=DMA)
  10    DRC      DR Control, parallel data length      (0=16bit, 1=8bit)
  9     SOC      SO Control, serial data output length (0=16bit, 1=8bit)
  8     SIC      SI Control, serial data input length  (0=16bit, 1=8bit)
  7     EI       Interrupt Enable                      (0=Disable, 1=Enable)
  6-2   N/A (R?) Unused/Reserved  (should be zero) (read=always zero?)
  1-0   P1-0     Output to P0,P1 pins (0=Low, 1=High)
```

SR.Bit15-8 are output to D7-0 pins (when /CS=LOW, /RD=LOW, /WR=HIGH, A0=HIGH).

SR.Bit1-0 are always output to P1-0 pins.

**RQM**

RQM gets set when the uPD77C25 does read/write its DR register, and gets
cleared when the remote CPU does complete reading/writing DR (complete: when
DRC=8bit, or when DRC=16bit and DRS=Ready). DRS gets toggled in 16bit mode
after each 8bit fragment being read/written by remote CPU (the fragments are
transferred LSB first, then MSB).

**DMA**

The DRQ-pin isn't connected in SNES cartridges (since the DMA protocol isn't
compatible with the SNES), so there's no real DMA support. However, the
uPD77C25 (or at least the ST011) is fast enough to handle SNES-DMA transfers by
software. Software for DSP2 uses the 65C816's block-transfer command (which is
a bit slower than DMA, but, like DMA, doesn't use handshaking).

**Memory**

```text
  2048 x 24bit Instruction ROM/PROM Opcodes
  2048 x 1bit  Instruction PROM Protection Flags (0=Lock, 1=Allow Dumping)
  1024 x 16bit Data ROM/PROM
  256 x 16bit  Data RAM
```

**ROM-Images**

DSPn/ST010/ST011 ROM-images consist of the Program ROM followed by the Data
ROM. Caution: There are several differently formatted dumps of these ROMs:

```text
  Oldest Files  10K (DSPn)               --> Big-Endian, 24bit-to-32bit padding
  Old Files     8K (DSPn) or 52K (ST01n) --> Big-Endian, raw 24bit opcodes
  Newer Files   8K (DSPn) or 52K (ST01n) --> Little-Endian, raw 24bit opcodes
```

Preferred would be the "Newer" format. To detect the endianness: All existing
ROMs contain "JRQM $" within first four opcodes (97C00xh, with x=0/4/8/C
depending on whether it is 1st/2nd/3rd/4th opcode). Ie. possible cases are:

```text
  Oldest Files  97h,C0h,0xh,FFh  ;big-endian 24bit, plus FFh-padding byte
  Old Files     97h,C0h,0xh      ;big-endian 24bit, without padding
  Newer Files   0xh,C0h,97h      ;little-endian 24bit, without padding
```

If the "JRQM $" opcode doesn't exist, best default to "Newer" format (that
might happen only with uncommon homebrewn DSP ROMs, not with the original
ROMs).

Ideally, the ROM-Image should be attached at the end of the SNES Cartridge
ROM-Image (when doing that, best remove any 200h-byte header, since the
existing headers don't define if/how to adjust their size-entries for
DSPn/ST01n ROMs).

**Chips**

```text
  uPD77C25   Mask ROM
  uPD77P25   Programmable PROM/UVEPROM
```

**uPD77C25 Opocde Encoding**

All opcodes are 24bit wide. All opcodes are executed in one clock cycle (at
max=8.192MHz clock).

```text
   23 22 21 20 19 18 17 16 15 14 13 12 11 10  9  8  7  6  5  4  3  2  1  0
  +--+--+-----+-----------+--+-----+-----------+--+-----------+-----------+
  |0 |RT|  P  | ALU opcode|A | DPL |    DPH    |RP|    SRC    |    DST    | ALU
  +--+--+-----+-----------+--+-----+-----------+--+-----------+-----+-----+
  |1 |0 |    BRCH (jump opcode)    |    NA (11bit Next Address)     |  -  | JP
  +--+--+--------------------------+--------------------+-----+-----+-----+
  |1 |1 |            ID (16bit Immediate Data)          |  -  |    DST    | LD
  +--+--+-----------------------------------------------+-----+-----------+
```

**uPD77C20 Opocde Encoding (older pre-77C25 version) (not used in SNES)**

All opcodes are 23bit wide. All opcodes are executed in one clock cycle (at
max=4.xxxMHz clock).

```text
   22 21 20 19 18 17 16 15 14 13 12 11 10  9  8  7  6  5  4  3  2  1  0
  +--+--+-----+-----------+--+-----+--------+--+-----------+-----------+
  |0 |RT|  P  | ALU opcode|A | DPL |  DPH   |RP|    SRC    |    DST    | ALU
  +--+--+-----+-----------+--+-+---+--------+--+-----------+-----+-----+
  |1 |0 |  BRCH (jump opcode)  |  NA (9bit Next Address)   |     -     | JP
  +--+--+----------------------+--------------------+---+--+-----+-----+
  |1 |1 |            ID (16bit Immediate Data)          |- |    DST    | LD
  +--+--+-----------------------------------------------+--+-----------+
```

DPH is only 3bit (M0..M7), BRCH is only 8bit (without JDPLN0, JDPLNF opcodes).
Data ROM entries are only 13bit wide (sign-expanded to 16bit... or left-shifted
to 16bit?). NA is only 9bit. Internal clock is only 4MHz. TRB register is not
supported.

<a id="snescartdspnst010st011necupd77c25aluandldinstructions"></a>

## SNES Cart DSP-n/ST010/ST011 - NEC uPD77C25 - ALU and LD Instructions

**ALU Instructions (Artithmetic/Logical Unit)**

```text
  23    0   Must be 0 for ALU opcodes
  22    RT  Return after ALU (0=No/Normal, 1=Yes/Return from Call/Interrupt)
  21-20 P   ALU input P (0=RAM[DP], 1=IDB(SRC), 2=K*L*2/10000h, 3=K*L*2)
  19-16 ALU ALU opcode
  15    A   ALU input/output Q (0=AccA, 1=AccB)
  14-13 DPL Data RAM Pointer DP.3-0 adjust (0=DPNOP, 1=DPINC, 2=DPDEC, 3=DPCLR)
  12-9  DPH Data RAM Pointer DP.7-4 adjust (0..0Fh=M0..MF) (XOR by that value)
  8     RP  Data ROM Pointer RP.9-0 adjust (0=RPNOP, 1=RPDEC)
  7-4   SRC Source (copied to DST, and, for ALU, to "IDB" internal data bus)
  3-0   DST Destination (copied from SRC)
```

Allows to combine an ALU operation with memory load &amp; store, DP/RP pointer
adjustment, optional RET from CALL, along with the K*L*2 multiplication.

**LD Instructions (Load)**

```text
  23    1   Must be 1 for LD opcodes
  22    1   Must be 1 for LD opcodes
  21-6  ID  16bit Immediate
  5-4   -   Reserved (should be zero)
  3-0   DST Destination (copied from ID)
```

Load is mainly for initialization purposes &amp; other special cases (normally
it's faster load immediates from Data ROM, via SRC=RO in ALU opcodes).

**ALU Opcode (Bit19-16)**

```text
  Hex Name      ;Expl.                      S1  S0  Cy  Zf OV1 OV0
  00h NOP       ;No operation               -   -   -   -  -   -
  01h OR        ;Acc = Acc OR P             sf  sf  0   zf 0   0
  02h AND       ;Acc = Acc AND P            sf  sf  0   zf 0   0
  03h XOR       ;Acc = Acc XOR P            sf  sf  0   zf 0   0
  04h SUB       ;Acc = Acc - P              *   sf  cy  zf *   ov
  05h ADD       ;Acc = Acc + P              *   sf  cy  zf *   ov
  06h SBB       ;Acc = Acc - P - OtherCy    *   sf  cy  zf *   ov
  07h ADC       ;Acc = Acc + P + OtherCy    *   sf  cy  zf *   ov
  08h DEC       ;Acc = Acc - 1              *   sf  cy  zf *   ov
  09h INC       ;Acc = Acc + 1              *   sf  cy  zf *   ov
  0Ah NOT       ;Acc = Acc XOR FFFFh        sf  sf  0   zf 0   0
  0Bh SAR1      ;Acc = Acc/2    ;signed     sf  sf  cy  zf 0   0
  0Ch RCL1      ;Acc = Acc*2 + OtherCy      sf  sf  cy  zf 0   0
  0Dh SLL2      ;Acc = Acc*4 + 3            sf  sf  0   zf 0   0
  0Eh SLL4      ;Acc = Acc*16 + 15          sf  sf  0   zf 0   0
  0Fh XCHG      ;Acc = Acc ROL 8            sf  sf  0   zf 0   0
```

ADD/ADC/SUB/SBB/INC/DEC set "S1=sf" and "OV1=OV1 XOR 1" upon overflow (and
leave S1 and OV1 both unchanged if no overflow).

OtherCy is the incoming carry flag from other accumulator (ie. Cy from FlagA
when using AccB).

Note: "NOT" is called "CMP"=Complement in official syntax; it isn't Compare.
"SAR/RCL/SLL" are called "SHR/SHL/SHL" in official syntax; though they aren't
"normal" logical shifts. OR/AND/XOR/NOT/SAR/RCL/SLL/XCHG do offially set
S1=Undefined (but actually it seems to be S1=sf; required for loopings in
"Super Air Diver 2"; which uses OR opcode followed by SGN operand).

**Multiplier**

After each instruction (namely after any ALU and LD instructions that have
changed K or L registers), the hardware computes K*L*2 (signed
16bit*16bit-&gt;32bit). Result on overflows (-8000h*-8000h*2) is unknown.

**SRC Field (Bit7-4) and DST Field (Bit3-0)**

```text
  Hex   SRC (Source, Bit7-4)           DST (Destination, Bit3-0)
  00h   TRB  (Temporary B)             @NON (none)
  01h   A    (AccA)                    @A   (AccA)
  02h   B    (AccB)                    @B   (AccB)
  03h   TR   (Temporary A)             @TR  (Temporary A)
  04h   DP   (Data RAM Pointer)        @DP  (Data RAM Pointer)
  05h   RP   (Data ROM Pointer)        @RP  (Data ROM Pointer)
  06h   RO   (ROM[RP])                 @DR  (parallel I/O port)
  07h   SGN  (saturation = 8000h-SA1)  @SR  (status register)
  08h   DR   (parallel I/O port)       @SOL (SO serial LSB first)
  09h   DRNF (DR without RQM/DRQ)      @SOM (SO serial MSB first)
  0Ah   SR   (status register)         @K   (Multiply Factor A)
  0Bh   SIM  (SI serial MSB first)     @KLR (K=SRC and L=ROM[RP])
  0Ch   SIL  (SI serial LSB first)     @KLM (L=SRC and K=RAM[DP OR 40h])
  0Dh   K    (Multiply Factor A)       @L   (Multiply Factor B)
  0Eh   L    (Multiply Factor B)       @TRB (Temporary B)
  0Fh   MEM  (RAM[DP])                 @MEM (RAM[DP])
```

When not using SRC: specify "NON" in source code, and 00h (a dummy TRB fetch)
in binary code.

Following combinations are prohibited in ALU instructions:

```text
  DST field = @KLR or @KLM combined with SRC field = K or L register
  DST field and SRC field specify the same register
  P-SELECT field = RAM, DST field = @MEM (for ALU operation)
```

Everything else should be allowed (included ALU with SRC=Acc, eg ADD AccA,AccA)

**Variants**

The older uPD77C20 doesn't support TRB: SRC=00h is NON (=zero/undefined?),
DST=0Eh (and also DST=00h) is @NON. Opcodes are only 23bit wide (strip
ALU.Bit11/MSB of DPH, and LD.Bit5/Reserved).

<a id="snescartdspnst010st011necupd77c25jpinstructions"></a>

## SNES Cart DSP-n/ST010/ST011 - NEC uPD77C25 - JP Instructions

**JP Instructions (Jump/Call)**

```text
  23    1   Must be 1 for JP opcodes
  22    0   Must be 0 for JP opcodes
  21-13 BRC Jump/Call opcode
  12-2  NA  11bit Next Address Bit0-10 (000h..7FFh, in 24-bit word steps)
  1-0   -   Reserved (should be zero) (ST010/ST011: Bit12-11 of NA)
```

**BRCH Opcode (Bit21-13)**

```text
  Binary    Hex  Op      Expl.
  000000000 000h JMPSO * Unconditional jump to SO register
  100000000 100h JMP     Unconditional jump 0000h..1FFFh
  100000001 101h JMP   * Unconditional jump 2000h..3FFFh
  101000000 140h CALL    Unconditional call 0000h..1FFFh  ;\return via RT-bit
  101000001 141h CALL  * Unconditional call 2000h..3FFFh  ;/in ALU opcodes
  010000000 080h JNCA    CA = 0         ;\
  010000010 082h JCA     CA = 1         ; carry flag of AccA/AccB
  010000100 084h JNCB    CB = 0         ;
  010000110 086h JCB     CB = 1         ;/
  010001000 088h JNZA    ZA = 0         ;\
  010001010 08Ah JZA     ZA = 1         ; zero flag of AccA/AccB
  010001100 08Ch JNZB    ZB = 0         ;
  010001110 08Eh JZB     ZB = 1         ;/
  010010000 090h JNOVA0  OVA0 = 0       ;\
  010010010 092h JOVA0   OVA0 = 1       ; overflow flag for last operation
  010010100 094h JNOVB0  OVB0 = 0       ;
  010010110 096h JOVB0   OVB0 = 1       ;/
  010011000 098h JNOVA1  OVA1 = 0       ;\
  010011010 09Ah JOVA1   OVA1 = 1       ; overflow flag for last 3 operations
  010011100 09Ch JNOVB1  OVB1 = 0       ; (set if 1 or 3 overflows occurred)
  010011110 09Eh JOVB1   OVB1 = 1       ;/
  010100000 0A0h JNSA0   SA0 = 0        ;\
  010100010 0A2h JSA0    SA0 = 1        ; sign bit (ie. Bit15) of AccA/AccB
  010100100 0A4h JNSB0   SB0 = 0        ;
  010100110 0A6h JSB0    SB0 = 1        ;/
  010101000 0A8h JNSA1   SA1 = 0        ;\
  010101010 0AAh JSA1    SA1 = 1        ; extra sign bit ("Bit16")
  010101100 0ACh JNSB1   SB1 = 0        ; indicating direction of overflows
  010101110 0AEh JSB1    SB1 = 1        ;/
  010110000 0B0h JDPL0   DPL = 00h      ;\
  010110001 0B1h JDPLN0  DPL <> 00h     ; lower 4bit of DP (Data RAM Pointer)
  010110010 0B2h JDPLF   DPL = 0Fh      ;
  010110011 0B3h JDPLNF  DPL <> 0Fh     ;/
  010110100 0B4h JNSIAK  SI ACK = 0     ;\
  010110110 0B6h JSIAK   SI ACK = 1     ; serial I/O port (SI/SO serial in/out)
  010111000 0B8h JNSOAK  SO ACK = 0     ;
  010111010 0BAh JSOAK   SO ACK = 1     ;/
  010111100 0BCh JNRQM   RQM = 0        ;\parallel I/O port (DR data register)
  010111110 0BEh JRQM    RQM = 1        ;/
```

Jump addresses should be specified 24bit word units (not in byte units).

(*) Opcodes 000h,101h,141h supported on ST010/ST011 only. On that CPU, PC.bit13
can be manipulated by unconditional jump/call/ret (whilst conditional jumps
affect only PC.bit12-0).

**Reset (Vector 000h)**

Reset is triggered when RST pin is high, and does set PC=000h, FlagA=00h,
FlagB=00h, SR=0000h, DRQ=0, SORQ=0, SI.ACK=0, SO.ACK=0, and RP=3FFh. Other
registers and Data RAM are left unchanged.

**Interrupts (Vector 100h)**

Interrupts are triggered on raising edge of INT pin. If interrupts are enabled
(in SR register), the CPU jumps to address 100h, and pushes PC on stack, the
interrupts are NOT automatically disabled.

**Variants**

The older uPD77C20 doesn't support JDPLN0 and JDPLNF. Opcodes are only 23bit
wide (strip JP.Bit13/LSB of BRCH). And PC/NA is only 9bit wide (replace
JP.Bit3-2 by Reserved).

<a id="snescartdspnst010st011listofgamesusingthatchips"></a>

## SNES Cart DSP-n/ST010/ST011 - List of Games using that chips

**Games using DSPn/ST01n chips**

The DSP-1/1A/1B is used by around 16..19 games:

```text
  Ace Wo Nerae! 3D Tennis (DSP-1A) (1993) Telenet Japan (JP)
  Armored Trooper Votoms: The Battling Road (1993) Takara (JP)
  Ballz 3D, and 3 Jigen Kakutou Ballz (DSP-1B) (1994) PF Magic/Accolade (NA)
  Battle Racers (1995) Banpresto (JP)
  Bike Daisuki! Hashiriya Kon - Rider's Spirits (1994) Genki/NCS (JP)
  Final Stretch (1993) Genki/LOZC (JP)
  Korean League (aka Hanguk Pro Yagu) (1993) Jaleco (KO)
  Lock-On / Super Air Diver (1993) Vic Tokai
  Michael Andretti's Indy Car Challenge (1994) Genki/Bullet-Proof (NA) (JP)
  Pilotwings (1991) Nintendo EAD (NA) (JP) (EU) (DSP-1) (visible DSP1 glitch)
  Shutokou Battle'94: K.T. Drift King (1994) Genki/Bullet-Proof (JP)
  Shutokou Battle 2: Drift King K.T. & M.B. (1995) Genki/Bullet-Proof (JP)
  Super 3D Baseball (?) (is that same as Super Bases Loaded 2 ?)
  Super Air Diver 2 (1995) Asmik (JP)
  Super Bases Loaded 2 (1994) Jaleco (NA) (JP)
  Super F1 Circus Gaiden (1995) Nichibutsu (JP)
  Super Mario Kart (DSP-1/DSP-1B) (1992) Nintendo EAD (NA) (JP) (EU)
  Suzuka 8 Hours (1993) Namco (NA) (JP)
  Touge Densetsu: Saisoku Battle (1996) Genki/Bullet-Proof Software (JP) ?
```

The other five versions are used by only one game each:

```text
  DSP-2: Dungeon Master (DSP-2) (1992) FTL Games/JVC Victor (JP)
  DSP-3: SD Gundam GX (DSP-3) (1994) BEC/Bandai (JP)
  DSP-4: Top Gear 3000 (DSP-4) (1995) Gremlin Interactive/Kemco (NA) (JP) (EU)
  ST010: F1 Race of Champions / Exhaust Heat II (1993) SETA Corp. (NA) (JP)
  ST011: Hayazashi Nidan Morita Shogi (1993) Random House/SETA Corp. (JP)
```

<a id="snescartdspnst010st011biosfunctions"></a>

## SNES Cart DSP-n/ST010/ST011 - BIOS Functions

**DSP1 Commands**

When requesting data from an external device the DSP is oblivious to the type
of operation that occurs to the Data Register. Writing to the Data register
will update the contents of the register and allow the DSP to continue
execution. Reading from the Data Register will also allow the DSP to continue
execution. On completion of a valid command the Data Register should contain
the value 0x80. This is to prevent a valid command from executing should a
device read past the end of output.

```text
  00h  16-bit Multiplication
  10h  Inverse Calculation
  20h  16-bit Multiplication
  01h  Set Attitude A
  11h  Set Attitude B
  21h  Set Attitude C
  02h  Projection Parameter Setting
  03h  Convert from Object to Global Coordinate A
  13h  Convert from Object to Global Coordinate B
  23h  Convert from Object to Global Coordinate C
  04h  Trigonometric Calculation
  14h  3D Angle Rotation
  06h  Object Projection Calculation
  08h  Vector Size Calculation
  18h  Vector Size Comparison
  28h  Vector Absolute Value Calculation (bugged) (fixed in DSP1B)
  38h  Vector Size Comparison
  0Ah  Raster Data Calculation
  0Bh  Calculation of Inner Product with the Forward Attitude A and a Vector
  1Bh  Calculation of Inner Product with the Forward Attitude B and a Vector
  2Bh  Calculation of Inner Product with the Forward Attitude C and a Vector
  0Ch  2D Coordinate Rotation
  1Ch  3D Coordinate Rotation
  0Dh  Convert from Global to Object Coordinate A
  1Dh  Convert from Global to Object Coordinate B
  2Dh  Convert from Global to Object Coordinate C
  0Eh  Coordinate Calculation of a selected point on the Screen
  0Fh  Test Memory Test
  1Fh  Test Transfer DATA ROM
  2Fh  Test ROM Version (0100h=DSP1/DSP1A, 0101h=DSP1B)
```

Command 28h is bugged in DSP1/DSP1A (fixed in DSP1B) bug is evident in
Pilotwings (Plane Demo).

**DSP2 Commands (Dungeon Master)**

This chip does - amazingly - assist 3D labyrinth drawing operations that are
normally implemented on ZX81 computers.

```text
  01h  Convert Bitmap to Bitplane Tile
  03h  Set Transparent Color
  05h  Replace Bitmap using Transparent Color
  06h  Reverse Bitmap
  07h  Add
  08h  Subtract
  09h  Multiply (bugged) (used in Dungeon Master japanese/v1.0)
  0Dh  Scale Bitmap
  0Fh  Process Command (dummy NOP command for re-synchronisation)
  10h..FFh Mirrors of 00h..0Fh
```

**DSP3 Commands (SD Gundam GX)**

The DSP functions inherently similiar to the DSP1 with respect to command
parsing and execution. On completion of a valid command the Data Register
should contain the value 0x80.

```text
  02h  Unknown
  03h  Calculate Cell Offset
  06h  Set Board Dimensions
  07h  Calculate Adjacent Cell
  18h  Convert Bitmap to Bitplane
  38h  Decode Shannon-Fano Bitstream (USF1 bit in SR register = direction)
  1Eh  Calculate Path of Least Travel
  3Eh  Set Start Cell
  0Fh  Test Memory Test
  1Fh  Test Transfer DATA ROM
  2Fh  Test ROM Version (0300h=DSP3)
```

**DSP4 Commands (Top Gear 3000)**

On completion of a valid command the Data Register should contain the value
0xffff. This is to prevent a valid command from executing should an external
device read past the end of output. Unlike previous DSP programs, all data
transfers are 16-bit.

```text
  xxh      Unknown
  13h      Test Transfer DATA ROM
  14h      Test ROM Version (0400h=DSP4)
  15h..1Fh Unused (no function)
  20h..FFh Mirrors of 10h..1Fh
```

**ST010 Commands**

Commands are executed on the ST-0010 by writing the command to 0x0020 and
setting bit7 of 0x0021. Bit7 of 0x0021 will stay set until the Command has
completed, at which time output data will be available. See individual commands
for input and output parameter addresses.

```text
  00h      Set RAM[0010h]=0000h
  01h      Unknown Command
  02h      Sort Driver Placements
  03h      2D Coordinate Scale
  04h      Unknown Command
  05h      Simulated Driver Coordinate Calculation
  06h      Multiply
  07h      Raster Data Calculation
  08h      2D Coordinate Rotation
  09h..0Fh Mirrors of 01h..07h
  10h..FFh Mirrors of 00h..0Fh
```

The ST010 BIOS functions are more or less useless and don't increase the
performance or quality of the game (the only feature that is &lt;really&gt;
used is the battery-backed on-chip RAM, aside from that, the powerful chip is a
waste of resources). Note: The ST010 is also used in "Twin Eagle II" (arcade
game, not a SNES game).

**ST011 Commands (japanese chess engine)**

```text
  00h      Unused (no function)
  01h      ?
  02h      ?
  03h      ?
  04h      ?
  05h      ?
  06h      ?
  07h      ?
  08h      Unused (no function)
  09h      ?
  0Ah      Unused (no function)
  0Bh      ?
  0Ch      ?
  0Dh      Unused (no function)
  0Eh      ?
  0Fh      ?
  10h..F0h Unused (no function)
  F1h      Selftest1 ?
  F2h      Selftest2 ?
  F3h      Dump Data ROM (bugged, doesn't work due to wrong loop address)
  F4h..FFh Unused (no function)
```

<a id="snescartsetast018preprogrammedarmcpu1game"></a>

## SNES Cart Seta ST018 (pre-programmed ARM CPU) (1 game)

**Seta ST018 - 160pin SETA D6984 ST018 chip (PCB SHVC-1DE3B-01)**

The chip is used by a single game only:

```text
  Hayazashi Nidan Morita Shogi 2 (ST018) (1995) Random House/SETA Corp. (JP)
```

**ARM CPU Reference**

[ARM CPU Reference](#arm-cpu-reference)

**ST018 Memory Map (ARM Side)**

```text
  00000000h ROM 128K  -- with 32bit databus
  20000000h
  40000000h I/O ports
  60000000h probably (absent) external ROM/EPROM ;can redirect exceptions here?
  80000000h
  A0000000h ROM 32K ? -- with 8bit databus
  C0000000h
  E0000000h RAM 16K
```

**ST018 I/O Map (ARM Side)**

```text
  40000010h.R  Data from SNES (reset STAT.3 and get latched data-to-arm)
  40000020h.R  Status         (get STAT)
  40000000h.W  Data to SNES   (set STAT.0 and latch data-to-snes)
  40000010h.W  Flag to SNES   (set STAT.2 on writing any value) (IRQ?)
  40000020h.W  Config 1
  40000024h.W  Config 2
  40000028h.W  Config 3
  4000002Ch.W  Config 4
```

**ST018 I/O Map (SNES Side)**

```text
  3800h.R      Data to SNES   (reset STAT.0 and get latched data-from-arm)
  3802h.R      Ack Flag       (reset STAT.2 and get dummy data?)
  3804h.R      Status         (get STAT)
  3802h.W      Data from SNES (set STAT.3 and latch data-from-snes)
  3804h.W      Reset ARM      (00h=Normal, 01h=HardReset, FFh=SoftReset?)
```

**ST018 Status Register**

There are two status registers, ARM:40000020h.R and SNES:3804h.R. Bit0 of that
two registers appears to be same for ARM and SNES, the other are used only by
either CPU (as shown below), although they might be actually existing on both
CPUs, too.

```text
  0 SNES ARM  ARM-to-SNES Data Present     (0=No, 1=Yes)
  1 -    -    Unknown/Unused               (unknown)
  2 SNES -    ARM-to-SNES IRQ Flag?        (0=No, 1=Yes)
  3 -    ARM  SNES-to-ARM Data Present     (0=No, 1=Yes)
  4 SNES -    Fatal Problem                (0=Okay, 1=SNES skips all transfers)
  5 -    ARM  Redirect ARM to 600000xxh    (0=No, 1=Yes)
  6 SNES -    Unused (unless [FF41h]<>00h) (0=Busy, 1=Ready)
  7 SNES -    ARM Reset Ready              (0=Busy, 1=Ready)
```

STAT.2 might be IRQ signal (ST018.pin12 connects to SNES./IRQ pin), but the
Shogi game contains only bugged IRQ handler (without ACK); instead it's just
polling STAT.2 by software.

**ST018 Component List**

```text
  PCB "SHVC-1DE3B-01, (C) 1995 Nintendo"
  U1  32pin LH534BN6 LoROM 512Kx8 (alternately 40pin) (PCB: "4/8/16/32M")
  U2  28pin LH52A64N SRAM 8Kx8                        (PCB: "64K")
  U3 160pin Seta ST018, (C)1994-5 SETA                (PCB: "ST0018/ST0019")
  U4  16pin 74LS139A (demultiplexer)                  (PCB: "LS139")
  U5   8pin /\\ 532 26A (battery controller)          (PCB: "MM1026")
  U6  18pin FA11B CIC                                 (PCB: "CIC")
  BATT 2pin Maxell CR2032T (3V battery for U2)
  X1   3pin [M]21440C 21.44MHz (plastic oscillator)   (PCB: "21.44MHz")
  P1  62pin SNES Cart Edge connector (plus shield)
```

Note: U5 is located on PCB back side for some weird reason. The chip name is
"ST018", although the PCB text layer calls it "ST0018" (with double zero).

**ST018 ARM Timings (mostly unknown)**

The ARM CPU is clocked by a 21.44MHz oscillator, but unknown if there is some
internal clock multiplier/divider for the actual CPU clock (if so, then it
might even be controlled via I/O ports for low power mode purposes).

Unknown if there is any code/data cache, and unknown if there are any memory
waitstates (if so, timings might differ for 8bit/32bit access, for
sequential/nonsequential access, and for different memory regions).

**ST018 ARM Memory (mostly unknown)**

Unknown if there any memory mirrors, or unused regions (possibly filled with
00h, or FFh, or with garbage), or regions that do trap memory exceptions.
Unknown if there any unused extra I/O ports or memory regions.

The 128K ROM, I/O area, and 16K RAM seem to support both 8bit and 32bit access.
The 32K ROM is used only with 8bit access; unknown what happens on 32bit access
to that region.

Effects on misaligned 32bit RAM writes are probably ignoring the lower address
bits, and writing to "ADDR AND (NOT 3)" (at least it like so on ARMv4/ARMv5)
(the case is important because there's a ST018 BUG that does "str r14,[r2,2]",
which should be 8bit STRB, not mis-aligned 32bit STR).

**ST018 ARM Other Stuff (mostly unknown)**

Unknown if there's any coprocessor or SMULL/UMULL extension (the BIOS doesn't
use such stuff, but CP14/CP15 are more or less common to be present).

The CPU seems to use ARMv3 instruction set (since the BIOS is using ARMv3
features: 32bit program counter and CPSR register; but isn't using any ARMv4
features such like BX, LDRH, Sys mode, or THUMB code) (also possible that ARMv4
processors haven't even been available at time when the ST018 was developed in
1994/1995).

**ST018 Commands**

```text
  00h..9Fh Unused
  A0       Debug: Reboot
  A1       Debug: Get Version 4 ;\maybe major/minor version (or vice-versa)
  A2       Debug: Get Version 5 ;/
  A3       Debug: Dump 80h bytes from address NNNNNNNNh
  A4       Debug: Dump NNh bytes from address NNNNNNNNh
  A5       Debug: Write NNh bytes to address NNNNNNNNh
  A8        do_high_level_func_0_1_with_reply_flag
  A9        do_high_level_func_1_1_with_reply_flag
  AA       UploadBoardAndSomethingElse (send 9x9 plus 16 bytes to 0E0000400h)
  AB       Write_1_byte_to_0E0000468h (usually value=02h)
  AC       Read ARM "R12" register value
  AD       Read 1 byte from 0E0000464h (LEN)
  AE        do_high_level_func_2_with_reply_flag
  AF       Read 1 byte from 0E0000464h (LEN+1)*2
  B0       Read (LEN+1)*2 bytes from 0E000046Ch  ;LEN as from cmd ADh/AFh
  B1        do_high_level_func_0_X_Y_with_reply_flag (send 2 bytes: X,Y)
  B2        do_high_level_func_1_X_Y_with_reply_flag (send 2 bytes: X,Y)
  B3        do_high_level_func_4_with_1_reply_byte   (recv 1 byte)
  B4        do_high_level_func_5_with_1_reply_byte   (recv 1 byte)
  B5        do_high_level_func_6_with_1_reply_byte   (recv 1 byte)
  B6        do_high_level_func_7_with_1_reply_byte   (recv 1 byte)
  B7        do_high_level_func_3_with_reply_flag
  B8h..F0h Unused
  F1       Selftest 1  ;if response.bit2=1, receive 2 error bytes
  F2       Selftest 2  ;if response<>00h, receive 2 error bytes
  F3       Debug: Dump 128Kbyte ROM from 00000000h ;\for HEX-DUMP display
  F4       Debug: Dump 32Kbyte ROM from A0000000h  ;/
  F5       Debug: Get Chksum for 128K ROM at 00000000h
  F6       Debug: Get Chksum for 32K ROM at A0000000h
  F7h..FFh Unused
```

Note: Command A5h allows to write code to RAM, and also to manipulate return
addresses on stack, thus allowing to execute custom ARM code.

<a id="armcpureference"></a>

## ARM CPU Reference

The ARM CPU is a 32bit RISC (Reduced Instruction Set Computer) processor,
designed by ARM (Advanced RISC Machines).

**General ARM Information**

[ARM Register Set](#arm-register-set)

[ARM Flags &amp; Condition Field (cond)](#arm-flags-condition-field-cond)

[ARM 26bit Memory Interface](#arm-26bit-memory-interface)

[ARM Exceptions](#arm-exceptions)

**The ARM Instruction Set**

[ARM Instruction Summary](#arm-instruction-summary)

[ARM Opcodes: Branch and Branch with Link (B, BL, SWI)](#arm-opcodes-branch-and-branch-with-link-b-bl-swi)

[ARM Opcodes: Data Processing (ALU)](#arm-opcodes-data-processing-alu)

[ARM Opcodes: PSR Transfer (MRS, MSR)](#arm-opcodes-psr-transfer-mrs-msr)

[ARM Opcodes: Multiply and Multiply-Accumulate (MUL, MLA)](#arm-opcodes-multiply-and-multiply-accumulate-mul-mla)

[ARM Opcodes: Memory: Block Data Transfer (LDM, STM)](#arm-opcodes-memory-block-data-transfer-ldm-stm)

[ARM Opcodes: Memory: Single Data Transfer (LDR, STR)](#arm-opcodes-memory-single-data-transfer-ldr-str)

[ARM Opcodes: Memory: Single Data Swap (SWP)](#arm-opcodes-memory-single-data-swap-swp)

[ARM Opcodes: Coprocessor Instructions (MRC/MCR, LDC/STC, CDP)](#arm-opcodes-coprocessor-instructions-mrcmcr-ldcstc-cdp)

**Further Information**

[ARM Pseudo Instructions and Directives](#arm-pseudo-instructions-and-directives)

[ARM Instruction Cycle Times](#arm-instruction-cycle-times)

[ARM Versions](#arm-versions)

<a id="armregisterset"></a>

## ARM Register Set

**Overview**

The following table shows the ARM7TDMI register set which is available in each
mode. There's a total of 37 registers (32bit each), 31 general registers (Rxx)
and 6 status registers (xPSR).

Note that only some registers are 'banked', for example, each mode has it's own
R14 register: called R14, R14_fiq, R14_svc, etc. for each mode respectively.

However, other registers are not banked, for example, each mode is using the
same R0 register, so writing to R0 will always affect the content of R0 in
other modes also.

```text
  System/User FIQ       Supervisor Abort     IRQ       Undefined
  --------------------------------------------------------------
  R0          R0        R0         R0        R0        R0
  R1          R1        R1         R1        R1        R1
  R2          R2        R2         R2        R2        R2
  R3          R3        R3         R3        R3        R3
  R4          R4        R4         R4        R4        R4
  R5          R5        R5         R5        R5        R5
  R6          R6        R6         R6        R6        R6
  R7          R7        R7         R7        R7        R7
  --------------------------------------------------------------
  R8          R8_fiq    R8         R8        R8        R8
  R9          R9_fiq    R9         R9        R9        R9
  R10         R10_fiq   R10        R10       R10       R10
  R11         R11_fiq   R11        R11       R11       R11
  R12         R12_fiq   R12        R12       R12       R12
  R13 (SP)    R13_fiq   R13_svc    R13_abt   R13_irq   R13_und
  R14 (LR)    R14_fiq   R14_svc    R14_abt   R14_irq   R14_und
  R15 (PC)    R15       R15        R15       R15       R15
  --------------------------------------------------------------
  CPSR        CPSR      CPSR       CPSR      CPSR      CPSR
  --          SPSR_fiq  SPSR_svc   SPSR_abt  SPSR_irq  SPSR_und
  --------------------------------------------------------------
```

**R0-R12 Registers (General Purpose Registers)**

These thirteen registers may be used for whatever general purposes. Basically,
each is having same functionality and performance, ie. there is no 'fast
accumulator' for arithmetic operations, and no 'special pointer register' for
memory addressing.

**R13 Register (SP)**

This register is used as Stack Pointer (SP) in THUMB state. While in ARM state
the user may decided to use R13 and/or other register(s) as stack pointer(s),
or as general purpose register.

As shown in the table above, there's a separate R13 register in each mode, and
(when used as SP) each exception handler may (and MUST!) use its own stack.

**R14 Register (LR)**

This register is used as Link Register (LR). That is, when calling to a
sub-routine by a Branch with Link (BL) instruction, then the return address
(ie. old value of PC) is saved in this register.

Storing the return address in the LR register is obviously faster than pushing
it into memory, however, as there's only one LR register for each mode, the
user must manually push its content before issuing 'nested' subroutines.

Same happens when an exception is called, PC is saved in LR of new mode.

Note: In ARM mode, R14 may be used as general purpose register also, provided
that above usage as LR register isn't required.

**R15 Register (PC)**

R15 is always used as program counter (PC). Note that when reading R15, this
will usually return a value of PC+nn because of read-ahead (pipelining),
whereas 'nn' depends on the instruction.

**CPSR and SPSR (Program Status Registers) (ARMv3 and up)**

The current condition codes (flags) and CPU control bits are stored in the CPSR
register. When an exception arises, the old CPSR is saved in the SPSR of the
respective exception-mode (much like PC is saved in LR).

For details refer to chapter about CPU Flags.

<a id="armflagsconditionfieldcond"></a>

## ARM Flags & Condition Field (cond)

**ARM Condition Field {cond}**

All ARM instructions can be conditionally executed depending on the state of
the CPSR flags (C,N,Z,V). The respective suffixes {cond} must be appended to
the mnemonics. For example: BEQ = Branch if Equal, MOVMI = Move if Signed.

```text
  Code Suffix Flags         Meaning
  0:   EQ     Z=1           equal (zero) (same)
  1:   NE     Z=0           not equal (nonzero) (not same)
  2:   CS/HS  C=1           unsigned higher or same (carry set)
  3:   CC/LO  C=0           unsigned lower (carry cleared)
  4:   MI     N=1           negative (minus)
  5:   PL     N=0           positive or zero (plus)
  6:   VS     V=1           overflow (V set)
  7:   VC     V=0           no overflow (V cleared)
  8:   HI     C=1 and Z=0   unsigned higher
  9:   LS     C=0 or Z=1    unsigned lower or same
  A:   GE     N=V           greater or equal
  B:   LT     N<>V          less than
  C:   GT     Z=0 and N=V   greater than
  D:   LE     Z=1 or N<>V   less or equal
  E:   AL     -             always (the "AL" suffix can be omitted)
  F:   NV     -             never (ARMv1,v2 only) (Reserved on ARMv3 and up)
```

Execution Time: If condition=false: 1S cycle. Otherwise: as specified for the
respective opcode.

**Current Program Status Register (CPSR)**

```text
  Bit   Expl.
  31    N - Sign Flag       (0=Not Signed, 1=Signed)               ;\
  30    Z - Zero Flag       (0=Not Zero, 1=Zero)                   ; Condition
  29    C - Carry Flag      (0=Borrow/No Carry, 1=Carry/No Borrow) ; Code Flags
  28    V - Overflow Flag   (0=No Overflow, 1=Overflow)            ;/
  27    Q - Reserved        (used as Sticky Overflow in ARMv5TE and up)
  26-8  - - Reserved        (For future use) - Do not change manually!
  7     I - IRQ disable     (0=Enable, 1=Disable)                  ;\
  6     F - FIQ disable     (0=Enable, 1=Disable)                  ; Control
  5     T - Reserved        (used as THUMB flag in ARMv4T and up)  ; Bits
  4-0   M4-M0 - Mode Bits   (See below)                            ;/
```

**CPSR Bit 27-8,5: Reserved Bits**

These bits are reserved for possible future implementations. For best forwards
compatibility, the user should never change the state of these bits, and should
not expect these bits to be set to a specific value.

**CPSR Bit 7-0: Control Bits (I,F,T,M4-M0)**

These bits may change when an exception occurs. In privileged modes (non-user
modes) they may be also changed manually.

The interrupt bits I and F are used to disable IRQ and FIQ interrupts
respectively (a setting of "1" means disabled).

The Mode Bits M4-M0 contain the current operating mode.

```text
  Binary Hex Dec  Expl.
  0xx00b 00h 0  - Old User       ;\26bit Backward Compatibility modes
  0xx01b 01h 1  - Old FIQ        ; (supported only on ARMv3, except ARMv3G,
  0xx10b 02h 2  - Old IRQ        ; and on some non-T variants of ARMv4)
  0xx11b 03h 3  - Old Supervisor ;/
  10000b 10h 16 - User (non-privileged)
  10001b 11h 17 - FIQ
  10010b 12h 18 - IRQ
  10011b 13h 19 - Supervisor (SWI)
  10111b 17h 23 - Abort
  11011b 1Bh 27 - Undefined
  11111b 1Fh 31 - Reserved (used as System mode in ARMv4 and up)
```

Writing any other values into the Mode bits is not allowed.

**Saved Program Status Registers (SPSR_&lt;mode&gt;)**

Additionally to above CPSR, five Saved Program Status Registers exist:

SPSR_fiq, SPSR_svc, SPSR_abt, SPSR_irq, SPSR_und

Whenever the CPU enters an exception, the current status register (CPSR) is
copied to the respective SPSR_&lt;mode&gt; register. Note that there is only
one SPSR for each mode, so nested exceptions inside of the same mode are
allowed only if the exception handler saves the content of SPSR in memory.

For example, for an IRQ exception: IRQ-mode is entered, and CPSR is copied to
SPSR_irq. If the interrupt handler wants to enable nested IRQs, then it must
first push SPSR_irq before doing so.

<a id="arm26bitmemoryinterface"></a>

## ARM 26bit Memory Interface

The 26bit Memory Interface was used by ARMv1 and ARMv2. The 32bit interface is
used by ARMv3 and newer, however, 26bit backward compatibility was included in
all ARMv3 (except ARMv3G), and optionally in some non-T variants of ARMv4.

**Format of R15 in 26bit Mode (Program Counter Register)**

```text
  Bit   Name     Expl.
  31-28 N,Z,C,V  Flags (Sign, Zero, Carry, Overflow)
  27-26 I,F      Interrupt Disable bits (IRQ, FIQ) (1=Disable)
  25-2  PC       Program Counter, 24bit, Step 4 (64M range)
  1-0   M1,M0    Mode (0=User, 1=FIQ, 2=IRQ, 3=Supervisor)
```

Branches with +/-32M range wrap the PC register, and can reach all 64M memory.

**Reading from R15**

If R15 is specified in bit16-19 of an opcode, then NZCVIF and M0,1 are masked
(zero), otherwise the full 32bits are used.

**Writing to R15**

ALU opcodes with S=1, and LDM opcodes with PSR=1 can write to all 32bits in R15
(in 26bit mode, that is allowed even in user mode, though it does then affect
only NZCF, not the write protected IFMM bits ???), other opcodes which write to
R15 will modify only the program counter bits. Also, special CMP/CMN/TST/TEQ{P}
opcodes can be used to write to the PSR bits in R15 without modifying the PC
bits.

**Exceptions**

SWIs, Reset, Data/Prefetch Aborts and Undefined instructions enter Supervisor
mode. Interrupts enter IRQ and FIQ mode. Additionally, a special 26bit Address
Exception exists, which enters Supervisor mode on accesses to memory
addresses&gt;=64M as follows:

```text
  R14_svc = PC ($+8, including old PSR bits)
  M1,M0 = 11b = supervisor mode, F=same, I=1, PC=14h,
  to continue at the fault location, return by SUBS PC,LR,8.
```

32bit CPUs with 26bit compatibility mode can be configured to switch into 32bit
mode when encountering exceptions.

<a id="armexceptions"></a>

## ARM Exceptions

**Exception Vectors**

The following are the exception vectors in memory. That is, when an exception
arises, CPU is switched into ARM state, and the program counter (PC) is loaded
by the respective address.

```text
  Address  Prio  Exception                  Mode on Entry      Interrupt Flags
  BASE+00h 1     Reset                      Supervisor (_svc)  I=1, F=1
  BASE+04h 7     Undefined Instruction      Undefined  (_und)  I=1, F=unchanged
  BASE+08h 6     Software Interrupt (SWI)   Supervisor (_svc)  I=1, F=unchanged
  BASE+0Ch 5     Prefetch Abort             Abort      (_abt)  I=1, F=unchanged
  BASE+10h 2     Data Abort                 Abort      (_abt)  I=1, F=unchanged
  BASE+14h ??    Address Exceeds 26bit      Supervisor (_svc)  I=1, F=unchanged
  BASE+18h 4     Normal Interrupt (IRQ)     IRQ        (_irq)  I=1, F=unchanged
  BASE+1Ch 3     Fast Interrupt (FIQ)       FIQ        (_fiq)  I=1, F=1
```

BASE is normally 00000000h, but may be optionally FFFF0000h in some ARM CPUs.
Priority for simultaneously occuring exceptions ranges from Prio=1=Highest to
Prio=7=Lowest.

As there's only space for one ARM opcode at each of the above addresses, it'd
be usually recommended to deposit a Branch opcode into each vector, which'd
then redirect to the actual exception handler address.

**Actions performed by CPU when entering an exception**

```text
  - R14_<new mode>=PC+nn   ;save old PC, ie. return address
  - SPSR_<new mode>=CPSR   ;save old flags
  - CPSR new T,M bits      ;set to T=0 (ARM state), and M4-0=new mode
  - CPSR new I bit         ;IRQs disabled (I=1), done by ALL exceptions
  - CPSR new F bit         ;FIQs disabled (F=1), done by Reset and FIQ only
  - PC=exception_vector    ;see table above
```

Above "PC+nn" depends on the type of exception (due to pipelining).

**Required user-handler actions when returning from an exception**

Restore any general registers (R0-R14) which might have been modified by the
exception handler. Use return-instruction as listed in the respective
descriptions below, this will both restore PC and CPSR - that automatically
involves that the old CPU state (THUMB or ARM) as well as old state of FIQ and
IRQ disable flags are restored.

As mentioned above (see action on entering...), the return address is always
saved in ARM-style format, so that exception handler may use the same
return-instruction, regardless of whether the exception has been generated from
inside of ARM or THUMB state.

**FIQ (Fast Interrupt Request)**

This interrupt is generated by a LOW level on the nFIQ input. It is supposed to
process timing critical interrupts at a high priority, as fast as possible.

Additionally to the common banked registers (R13_fiq,R14_fiq), five extra
banked registers (R8_fiq-R12_fiq) are available in FIQ mode. The exception
handler may freely access these registers without modifying the main programs
R8-R12 registers (and without having to save that registers on stack).

In privileged (non-user) modes, FIQs may be also manually disabled by setting
the F Bit in CPSR.

**IRQ (Normal Interrupt Request)**

This interrupt is generated by a LOW level on the nIRQ input. Unlike FIQ, the
IRQ mode is not having its own banked R8-R12 registers.

IRQ is having lower priority than FIQ, and IRQs are automatically disabled when
a FIQ exception becomes executed. In privileged (non-user) modes, IRQs may be
also manually disabled by setting the I Bit in CPSR.

To return from IRQ Mode (continuing at following opcode):

```text
  SUBS PC,R14,4   ;both PC=R14_irq-4, and CPSR=SPSR_irq
```

**Software Interrupt**

Generated by a software interrupt instruction (SWI). Recommended to request a
supervisor (operating system) function. The SWI instruction may also contain a
parameter in the 'comment field' of the lower 24bit of the 32bit opcode opcode
at [R14_svc-4].

To return from Supervisor Mode (continuing at following opcode):

```text
  MOVS PC,R14   ;both PC=R14_svc, and CPSR=SPSR_svc
```

**Undefined Instruction Exception (supported by ARMv3 and up)**

This exception is generated when the CPU comes across an instruction which it
cannot handle. Most likely signalizing that the program has locked up, and that
an errormessage should be displayed.

However, it might be also used to emulate custom functions, ie. as an
additional 'SWI' instruction (which'd use R14_und and SPSR_und though, and it'd
thus allow to execute the Undefined Instruction handler from inside of
Supervisor mode without having to save R14_svc and SPSR_svc).

To return from Undefined Mode (continuing at following opcode):

```text
  MOVS PC,R14   ;both PC=R14_und, and CPSR=SPSR_und
```

Note that not all unused opcodes are necessarily producing an exception, for
example, an ARM state Multiply instruction with Bit6=1 would be blindly
accepted as 'legal' opcode.

**Abort (supported by ARMv3 and up)**

Aborts (page faults) are mostly supposed for virtual memory systems (ie. not
used in GBA, as far as I know), otherwise they might be used just to display an
error message. Two types of aborts exists:

- Prefetch Abort (occurs during an instruction prefetch)

- Data Abort (occurs during a data access)

A virtual memory systems abort handler would then most likely determine the
fault address: For prefetch abort that's just "R14_abt-4". For Data abort, the
THUMB or ARM instruction at "R14_abt-8" needs to be 'disassembled' in order to
determine the addressed data in memory.

The handler would then fix the error by loading the respective memory page into
physical memory, and then retry to execute the SAME instruction again, by
returning as follows:

```text
  prefetch abort: SUBS PC,R14,#4   ;PC=R14_abt-4, and CPSR=SPSR_abt
  data abort:     SUBS PC,R14,#8   ;PC=R14_abt-8, and CPSR=SPSR_abt
```

Separate exception vectors for prefetch/data abort exists, each should use the
respective return instruction as shown above.

**Address Exceeds 26bit**

This exception can occur only on old ARM CPUs with 26bit address scheme (or in
26bit backwards compatibility mode).

**Reset**

Forces PC=VVVV0000h, and forces control bits of CPSR to T=0 (ARM state), F=1
and I=1 (disable FIQ and IRQ), and M4-0=10011b (Supervisor mode).

<a id="arminstructionsummary"></a>

## ARM Instruction Summary

Modification of CPSR flags is optional for all {S} instructions.

**Logical ALU Operations**

```text
  Instruction                      Cycles    Flags Expl.
  MOV{cond}{S} Rd,Op2              1S+x+y     NZc- Rd = Op2
  MVN{cond}{S} Rd,Op2              1S+x+y     NZc- Rd = NOT Op2
  ORR{cond}{S} Rd,Rn,Op2           1S+x+y     NZc- Rd = Rn OR Op2
  EOR{cond}{S} Rd,Rn,Op2           1S+x+y     NZc- Rd = Rn XOR Op2
  AND{cond}{S} Rd,Rn,Op2           1S+x+y     NZc- Rd = Rn AND Op2
  BIC{cond}{S} Rd,Rn,Op2           1S+x+y     NZc- Rd = Rn AND NOT Op2
  TST{cond}{P}    Rn,Op2           1S+x       NZc- Void = Rn AND Op2
  TEQ{cond}{P}    Rn,Op2           1S+x       NZc- Void = Rn XOR Op2
```

Add x=1I cycles if Op2 shifted-by-register. Add y=1S+1N cycles if Rd=R15.

Carry flag affected only if Op2 contains a non-zero shift amount.

**Arithmetic ALU Operations**

```text
  Instruction                      Cycles    Flags Expl.
  ADD{cond}{S} Rd,Rn,Op2           1S+x+y     NZCV Rd = Rn+Op2
  ADC{cond}{S} Rd,Rn,Op2           1S+x+y     NZCV Rd = Rn+Op2+Cy
  SUB{cond}{S} Rd,Rn,Op2           1S+x+y     NZCV Rd = Rn-Op2
  SBC{cond}{S} Rd,Rn,Op2           1S+x+y     NZCV Rd = Rn-Op2+Cy-1
  RSB{cond}{S} Rd,Rn,Op2           1S+x+y     NZCV Rd = Op2-Rn
  RSC{cond}{S} Rd,Rn,Op2           1S+x+y     NZCV Rd = Op2-Rn+Cy-1
  CMP{cond}{P}    Rn,Op2           1S+x       NZCV Void = Rn-Op2
  CMN{cond}{P}    Rn,Op2           1S+x       NZCV Void = Rn+Op2
```

Add x=1I cycles if Op2 shifted-by-register. Add y=1S+1N cycles if Rd=R15.

**Multiply**

```text
  Instruction                      Cycles    Flags Expl.
  MUL{cond}{S} Rd,Rm,Rs            1S+mI      NZx- Rd = Rm*Rs
  MLA{cond}{S} Rd,Rm,Rs,Rn         1S+mI+1I   NZx- Rd = Rm*Rs+Rn
  UMULL{cond}{S} RdLo,RdHi,Rm,Rs   1S+mI+1I   NZx- RdHiLo = Rm*Rs
  UMLAL{cond}{S} RdLo,RdHi,Rm,Rs   1S+mI+2I   NZx- RdHiLo = Rm*Rs+RdHiLo
  SMULL{cond}{S} RdLo,RdHi,Rm,Rs   1S+mI+1I   NZx- RdHiLo = Rm*Rs
  SMLAL{cond}{S} RdLo,RdHi,Rm,Rs   1S+mI+2I   NZx- RdHiLo = Rm*Rs+RdHiLo
```

**Memory Load/Store**

```text
  Instruction                      Cycles    Flags Expl.
  LDR{cond}{B}{T} Rd,<Address>     1S+1N+1I+y ---- Rd=[Rn+/-<offset>]
  LDM{cond}{amod} Rn{!},<Rlist>{^} nS+1N+1I+y ---- Load Multiple
  STR{cond}{B}{T} Rd,<Address>     2N         ---- [Rn+/-<offset>]=Rd
  STM{cond}{amod} Rn{!},<Rlist>{^} (n-1)S+2N  ---- Store Multiple
  SWP{cond}{B}    Rd,Rm,[Rn]       1S+2N+1I   ---- Rd=[Rn], [Rn]=Rm
```

For LDR/LDM, add y=1S+1N if Rd=R15, or if R15 in Rlist.

**Jumps, Calls, CPSR Mode, and others**

```text
  Instruction                      Cycles    Flags Expl.
  B{cond}   label                  2S+1N      ---- PC=$+8+/-32M
  BL{cond}  label                  2S+1N      ---- PC=$+8+/-32M, LR=$+4
  MRS{cond} Rd,Psr                 1S         ---- Rd=Psr
  MSR{cond} Psr{_field},Op         1S        (psr) Psr[field]=Op
  SWI{cond} Imm24bit               2S+1N      ---- PC=8, ARM Svc mode, LR=$+4
  The Undefined Instruction        2S+1I+1N   ---- PC=4, ARM Und mode, LR=$+4
  condition=false                  1S         ---- Opcodes with {cond}=false
  NOP                              1S         ---- R0=R0
```

**Coprocessor Functions (if any)**

```text
  Instruction                         Cycles  Flags Expl.
  CDP{cond} Pn,<cpopc>,Cd,Cn,Cm{,<cp>} 1S+bI   ----  Coprocessor specific
  STC{cond}{L} Pn,Cd,<Address>         (n-1)S+2N+bI  [address] = CRd
  LDC{cond}{L} Pn,Cd,<Address>         (n-1)S+2N+bI  CRd = [address]
  MCR{cond} Pn,<cpopc>,Rd,Cn,Cm{,<cp>} 1S+bI+1C      CRn = Rn {<op> CRm}
  MRC{cond} Pn,<cpopc>,Rd,Cn,Cm{,<cp>} 1S+(b+1)I+1C  Rn = CRn {<op> CRm}
```

**ARM Binary Opcode Format**

```text
  |..3 ..................2 ..................1 ..................0|
  |1_0_9_8_7_6_5_4_3_2_1_0_9_8_7_6_5_4_3_2_1_0_9_8_7_6_5_4_3_2_1_0|
  |_Cond__|0_0_0|___Op__|S|__Rn___|__Rd___|__Shift__|Typ|0|__Rm___| DataProc
  |_Cond__|0_0_0|___Op__|S|__Rn___|__Rd___|__Rs___|0|Typ|1|__Rm___| DataProc
  |_Cond__|0_0_1|___Op__|S|__Rn___|__Rd___|_Shift_|___Immediate___| DataProc
  |_Cond__|0_0_1_1_0|P|1|0|_Field_|__Rd___|_Shift_|___Immediate___| PSR Imm
  |_Cond__|0_0_0_1_0|P|L|0|_Field_|__Rd___|0_0_0_0|0_0_0_0|__Rm___| PSR Reg
  |_Cond__|0_0_0_0_0_0|A|S|__Rd___|__Rn___|__Rs___|1_0_0_1|__Rm___| Multiply
  |_Cond__|0_0_0_0_1|U|A|S|_RdHi__|_RdLo__|__Rs___|1_0_0_1|__Rm___| MulLong
  |_Cond__|0_0_0_1_0|B|0_0|__Rn___|__Rd___|0_0_0_0|1_0_0_1|__Rm___| TransSwap
  |_Cond__|0_1_0|P|U|B|W|L|__Rn___|__Rd___|_________Offset________| TransImm
  |_Cond__|0_1_1|P|U|B|W|L|__Rn___|__Rd___|__Shift__|Typ|0|__Rm___| TransReg
  |_Cond__|0_1_1|________________xxx____________________|1|__xxx__| Undefined
  |_Cond__|1_0_0|P|U|S|W|L|__Rn___|__________Register_List________| TransBlock
  |_Cond__|1_0_1|L|___________________Offset______________________| B,BL
  |_Cond__|1_1_0|P|U|N|W|L|__Rn___|__CRd__|__CP#__|____Offset_____| CoDataTrans
  |_Cond__|1_1_1_0|_CPopc_|__CRn__|__CRd__|__CP#__|_CP__|0|__CRm__| CoDataOp
  |_Cond__|1_1_1_0|CPopc|L|__CRn__|__Rd___|__CP#__|_CP__|1|__CRm__| CoRegTrans
  |_Cond__|1_1_1_1|_____________Ignored_by_Processor______________| SWI
```

<a id="armopcodesbranchandbranchwithlinkbblswi"></a>

## ARM Opcodes: Branch and Branch with Link (B, BL, SWI)

**Branch and Branch with Link (B, BL)**

Branch (B) is supposed to jump to a subroutine. Branch with Link is meant to be
used to call to a subroutine, return address is then saved in R14/LR (and can
be restored via MOV PC,LR aka MOV R15,R14) (for nested subroutines, use PUSH LR
and POP PC).

```text
  Bit    Expl.
  31-28  Condition
  27-25  Must be "101" for this instruction
  24     Opcode (0-1)
          0: B{cond} label    ;branch      (jump)    PC=PC+8+nn*4
          1: BL{cond} label   ;branch/link (call)    PC=PC+8+nn*4, LR=PC+4
  23-0   nn - Signed Offset, step 4      (-32M..+32M in steps of 4)
```

Execution Time: 2S + 1N

Return: No flags affected.

**Branch via ALU, LDR, LDM**

Most ALU, LDR, LDM opcodes can also change PC/R15.

**Mis-aligned PC/R15 (MOV/ALU/LDR with Rd=R15)**

For ARM code, the low bits of the target address should be usually zero,
otherwise, R15 is forcibly aligned by clearing the lower two bits.

In short, R15 will be always forcibly aligned, so mis-aligned branches won't
have effect on subsequent opcodes that use R15, or [R15+disp] as operand.

**Software Interrupt (SWI) (svc exception)**

SWI supposed for calls to the operating system - Enter Supervisor mode (SVC).

```text
  Bit    Expl.
  31-28  Condition
  27-24  Opcode
          1111b: SWI{cond} nn   ;software interrupt
  23-0   nn - Comment Field, ignored by processor (24bit value)
```

Execution Time: 2S+1N

The exception handler may interprete the Comment Field by examining the lower
24bit of the 32bit opcode opcode at [R14_svc-4].

For Returning from SWI use "MOVS PC,R14", that instruction does restore both PC
and CPSR, ie. PC=R14_svc, and CPSR=SPSR_svc.

Nesting SWIs: SPSR_svc and R14_svc should be saved on stack before either
invoking nested SWIs, or (if the IRQ handler uses SWIs) before enabling IRQs.

**Undefined Instruction (und exception)**

```text
  Bit    Expl.
  31-28  Condition
  27-25  Must be 011b for this instruction
  24-5   Reserved for future use
  4      Must be 1b for this instruction
  3-0    Reserved for future use
```

No assembler mnemonic exists, following bitstreams are (not) reserved.

```text
  cond011xxxxxxxxxxxxxxxxxxxx1xxxx - reserved for future use (except below).
  cond01111111xxxxxxxxxxxx1111xxxx - free for user.
```

Execution time: 2S+1I+1N.

<a id="armopcodesdataprocessingalu"></a>

## ARM Opcodes: Data Processing (ALU)

**Data Processing (ALU)**

```text
  Bit    Expl.
  31-28  Condition
  27-26  Must be 00b for this instruction
  25     I - Immediate 2nd Operand Flag (0=Register, 1=Immediate)
  24-21  Opcode (0-Fh)               ;*=Arithmetic, otherwise Logical
           0: AND{cond}{S} Rd,Rn,Op2    ;AND logical       Rd = Rn AND Op2
           1: EOR{cond}{S} Rd,Rn,Op2    ;XOR logical       Rd = Rn XOR Op2
           2: SUB{cond}{S} Rd,Rn,Op2 ;* ;subtract          Rd = Rn-Op2
           3: RSB{cond}{S} Rd,Rn,Op2 ;* ;subtract reversed Rd = Op2-Rn
           4: ADD{cond}{S} Rd,Rn,Op2 ;* ;add               Rd = Rn+Op2
           5: ADC{cond}{S} Rd,Rn,Op2 ;* ;add with carry    Rd = Rn+Op2+Cy
           6: SBC{cond}{S} Rd,Rn,Op2 ;* ;sub with carry    Rd = Rn-Op2+Cy-1
           7: RSC{cond}{S} Rd,Rn,Op2 ;* ;sub cy. reversed  Rd = Op2-Rn+Cy-1
           8: TST{cond}{P}    Rn,Op2    ;test            Void = Rn AND Op2
           9: TEQ{cond}{P}    Rn,Op2    ;test exclusive  Void = Rn XOR Op2
           A: CMP{cond}{P}    Rn,Op2 ;* ;compare         Void = Rn-Op2
           B: CMN{cond}{P}    Rn,Op2 ;* ;compare neg.    Void = Rn+Op2
           C: ORR{cond}{S} Rd,Rn,Op2    ;OR logical        Rd = Rn OR Op2
           D: MOV{cond}{S} Rd,Op2       ;move              Rd = Op2
           E: BIC{cond}{S} Rd,Rn,Op2    ;bit clear         Rd = Rn AND NOT Op2
           F: MVN{cond}{S} Rd,Op2       ;not               Rd = NOT Op2
  20     S - Set Condition Codes (0=No, 1=Yes) (Must be 1 for opcode 8-B)
  19-16  Rn - 1st Operand Register (R0..R15) (including PC=R15)
              Must be 0000b for MOV/MVN.
  15-12  Rd - Destination Register (R0..R15) (including PC=R15)
              Must be 0000b (or 1111b) for CMP/CMN/TST/TEQ{P}.
  When above Bit 25 I=0 (Register as 2nd Operand)
    When below Bit 4 R=0 - Shift by Immediate
      11-7   Is - Shift amount   (1-31, 0=Special/See below)
    When below Bit 4 R=1 - Shift by Register
      11-8   Rs - Shift register (R0-R14) - only lower 8bit 0-255 used
      7      Reserved, must be zero  (otherwise multiply or undefined opcode)
    6-5    Shift Type (0=LSL, 1=LSR, 2=ASR, 3=ROR)
    4      R - Shift by Register Flag (0=Immediate, 1=Register)
    3-0    Rm - 2nd Operand Register (R0..R15) (including PC=R15)
  When above Bit 25 I=1 (Immediate as 2nd Operand)
    11-8   Is - ROR-Shift applied to nn (0-30, in steps of 2)
    7-0    nn - 2nd Operand Unsigned 8bit Immediate
```

**Second Operand (Op2)**

This may be a shifted register, or a shifted immediate. See Bit 25 and 11-0.

Unshifted Register: Specify Op2 as "Rm", assembler converts to "Rm,LSL#0".

Shifted Register: Specify as "Rm,SSS#Is" or "Rm,SSS Rs" (SSS=LSL/LSR/ASR/ROR).

Immediate: Specify as 32bit value, for example: "#000NN000h", assembler should
automatically convert into "#0NNh,ROR#0ssh" as far as possible (ie. as far as a
section of not more than 8bits of the immediate is non-zero).

**Zero Shift Amount (Shift Register by Immediate, with Immediate=0)**

```text
  LSL#0: No shift performed, ie. directly Op2=Rm, the C flag is NOT affected.
  LSR#0: Interpreted as LSR#32, ie. Op2 becomes zero, C becomes Bit 31 of Rm.
  ASR#0: Interpreted as ASR#32, ie. Op2 and C are filled by Bit 31 of Rm.
  ROR#0: Interpreted as RRX#1 (RCR), like ROR#1, but Op2 Bit 31 set to old C.
```

In source code, LSR#32, ASR#32, and RRX#1 should be specified as such -
attempts to specify LSR#0, ASR#0, or ROR#0 will be internally converted to
LSL#0 by the assembler.

**Using R15 (PC)**

When using R15 as Destination (Rd), note below CPSR description and Execution
time description.

When using R15 as operand (Rm or Rn), the returned value depends on the
instruction: PC+12 if I=0,R=1 (shift by register), otherwise PC+8 (shift by
immediate).

**Returned CPSR Flags**

If S=1, Rd&lt;&gt;R15, logical operations (AND,EOR,TST,TEQ,ORR,MOV,BIC,MVN):

```text
  V=not affected
  C=carryflag of shift operation (not affected if LSL#0 or Rs=00h)
  Z=zeroflag of result
  N=signflag of result (result bit 31)
```

If S=1, Rd&lt;&gt;R15, arithmetic operations (SUB,RSB,ADD,ADC,SBC,RSC,CMP,CMN):

```text
  V=overflowflag of result
  C=carryflag of result
  Z=zeroflag of result
  N=signflag of result (result bit 31)
```

IF S=1, with unused Rd bits=1111b, {P} opcodes (CMPP/CMNP/TSTP/TEQP):

```text
  R15=result  ;modify PSR bits in R15, ARMv2 and below only.
  In user mode only N,Z,C,V bits of R15 can be changed.
  In other modes additionally I,F,M1,M0 can be changed.
  The PC bits in R15 are left unchanged in all modes.
```

If S=1, Rd=R15; should not be used in user mode:

```text
  CPSR = SPSR_<current mode>
  PC = result
  For example: MOVS PC,R14  ;return from SWI (PC=R14_svc, CPSR=SPSR_svc).
```

If S=0: Flags are not affected (not allowed for CMP,CMN,TEQ,TST).

The instruction "MOV R0,R0" is used as "NOP" opcode in 32bit ARM state.

Execution Time: (1+p)S+rI+pN. Whereas r=1 if I=0 and R=1 (ie. shift by
register); otherwise r=0. And p=1 if Rd=R15; otherwise p=0.

<a id="armopcodespsrtransfermrsmsr"></a>

## ARM Opcodes: PSR Transfer (MRS, MSR)

**Opcode Format**

These instructions occupy an unused area (TEQ,TST,CMP,CMN with S=0) of ALU
opcodes.

```text
  Bit    Expl.
  31-28  Condition
  27-26  Must be 00b for this instruction
  25     I - Immediate Operand Flag  (0=Register, 1=Immediate) (Zero for MRS)
  24-23  Must be 10b for this instruction
  22     Psr - Source/Destination PSR  (0=CPSR, 1=SPSR_<current mode>)
  21     Opcode
           0: MRS{cond} Rd,Psr          ;Rd = Psr
           1: MSR{cond} Psr{_field},Op  ;Psr[field] = Op
  20     Must be 0b for this instruction (otherwise TST,TEQ,CMP,CMN)
  For MRS:
    19-16   Must be 1111b for this instruction (otherwise SWP)
    15-12   Rd - Destination Register  (R0-R14)
    11-0    Not used, must be zero.
  For MSR:
    19      f  write to flags field     Bit 31-24 (aka _flg)
    18      s  write to status field    Bit 23-16 (reserved, don't change)
    17      x  write to extension field Bit 15-8  (reserved, don't change)
    16      c  write to control field   Bit 7-0   (aka _ctl)
    15-12   Not used, must be 1111b.
  For MSR Psr,Rm (I=0)
    11-4    Not used, must be zero.
    3-0     Rm - Source Register <op>  (R0-R14)
  For MSR Psr,Imm (I=1)
    11-8    Shift applied to Imm   (ROR in steps of two 0-30)
    7-0     Imm - Unsigned 8bit Immediate
    In source code, a 32bit immediate should be specified as operand.
    The assembler should then convert that into a shifted 8bit value.
```

MSR/MRS and CPSR/SPSR supported by ARMv3 and up.

ARMv2 and below contained PSR flags in R15, accessed by CMP/CMN/TST/TEQ{P}.

The field mask bits specify which bits of the destination Psr are write-able
(or write-protected), one or more of these bits should be set, for example,
CPSR_fsxc (aka CPSR aka CPSR_all) unlocks all bits (see below user mode
restriction though).

Restrictions:

In non-privileged mode (user mode): only condition code bits of CPSR can be
changed, control bits can't.

Only the SPSR of the current mode can be accessed; In User and System modes no
SPSR exists.

Unused Bits in CPSR are reserved for future use and should never be changed
(except for unused bits in the flags field).

Execution Time: 1S.

Note: The A22i assembler recognizes MOV as alias for both MSR and MRS because
it is practically not possible to remember whether MSR or MRS was the load or
store opcode, and/or whether it does load to or from the Psr register.

<a id="armopcodesmultiplyandmultiplyaccumulatemulmla"></a>

## ARM Opcodes: Multiply and Multiply-Accumulate (MUL, MLA)

**Opcode Format**

```text
  Bit    Expl.
  31-28  Condition
  27-25  Must be 000b for this instruction
  24-21  Opcode
          0000b: MUL{cond}{S}   Rd,Rm,Rs        ;multiply   Rd = Rm*Rs
          0001b: MLA{cond}{S}   Rd,Rm,Rs,Rn     ;mul.& accumulate Rd = Rm*Rs+Rn
          0100b: UMULL{cond}{S} RdLo,RdHi,Rm,Rs ;multiply   RdHiLo=Rm*Rs
          0101b: UMLAL{cond}{S} RdLo,RdHi,Rm,Rs ;mul.& acc. RdHiLo=Rm*Rs+RdHiLo
          0110b: SMULL{cond}{S} RdLo,RdHi,Rm,Rs ;sign.mul.  RdHiLo=Rm*Rs
          0111b: SMLAL{cond}{S} RdLo,RdHi,Rm,Rs ;sign.m&a.  RdHiLo=Rm*Rs+RdHiLo
  20     S - Set Condition Codes (0=No, 1=Yes) (Must be 0 for Halfword mul)
  19-16  Rd (or RdHi) - Destination Register (R0-R14)
  15-12  Rn (or RdLo) - Accumulate Register  (R0-R14) (Set to 0000b if unused)
  11-8   Rs - Operand Register               (R0-R14)
  7-4    Must be 1001b for these instructions
  3-0    Rm - Operand Register               (R0-R14)
```

**Multiply and Multiply-Accumulate (MUL, MLA)**

Restrictions: Rd may not be same as Rm. Rd,Rn,Rs,Rm may not be R15.

Note: Only the lower 32bit of the internal 64bit result are stored in Rd, thus
no sign/zero extension is required and MUL and MLA can be used for both signed
and unsigned calculations!

Execution Time: 1S+mI for MUL, and 1S+(m+1)I for MLA. Whereas 'm' depends on
whether/how many most significant bits of Rs are all zero or all one. That is
m=1 for Bit 31-8, m=2 for Bit 31-16, m=3 for Bit 31-24, and m=4 otherwise.

Flags (if S=1): Z=zeroflag, N=signflag, C=destroyed (ARMv4 and below) or C=not
affected (ARMv5 and up), V=not affected. MUL/MLA supported by ARMv2 and up.

**Multiply Long and Multiply-Accumulate Long (MULL, MLAL)**

Optionally supported, INCLUDED in ARMv3M, EXCLUDED in ARMv4xM/ARMv5xM.

Restrictions: RdHi,RdLo,Rm must be different registers. R15 may not be used.

Execution Time: 1S+(m+1)I for MULL, and 1S+(m+2)I for MLAL. Whereas 'm' depends
on whether/how many most significant bits of Rs are "all zero" (UMULL/UMLAL) or
"all zero or all one" (SMULL,SMLAL). That is m=1 for Bit 31-8, m=2 for Bit
31-16, m=3 for Bit 31-24, and m=4 otherwise.

Flags (if S=1): Z=zeroflag, N=signflag, C=destroyed (ARMv4 and below) or C=not
affected (ARMv5 and up), V=destroyed??? (ARMv4 and below???) or V=not affected
(ARMv5 and up).

<a id="armopcodesmemoryblockdatatransferldmstm"></a>

## ARM Opcodes: Memory: Block Data Transfer (LDM, STM)

**Opcode Format**

```text
  Bit    Expl.
  31-28  Condition
  27-25  Must be 100b for this instruction
  24     P - Pre/Post (0=post; add offset after transfer, 1=pre; before trans.)
  23     U - Up/Down Bit (0=down; subtract offset from base, 1=up; add to base)
  22     S - PSR & force user bit (0=No, 1=load PSR or force user mode)
  21     W - Write-back bit (0=no write-back, 1=write address into base)
  20     L - Load/Store bit (0=Store to memory, 1=Load from memory)
          0: STM{cond}{amod} Rn{!},<Rlist>{^}  ;Store (Push)
          1: LDM{cond}{amod} Rn{!},<Rlist>{^}  ;Load  (Pop)
          Whereas, {!}=Write-Back (W), and {^}=PSR/User Mode (S)
  19-16  Rn - Base register                (R0-R14) (not including R15)
  15-0   Rlist - Register List
  (Above 'offset' is meant to be the number of words specified in Rlist.)
```

Return: No Flags affected.

Execution Time: For normal LDM, nS+1N+1I. For LDM PC, (n+1)S+2N+1I. For STM
(n-1)S+2N. Where n is the number of words transferred.

**Addressing Modes {amod}**

The IB,IA,DB,DA suffixes directly specify the desired U and P bits:

```text
  IB  increment before          ;P=1, U=1
  IA  increment after           ;P=0, U=1
  DB  decrement before          ;P=1, U=0
  DA  decrement after           ;P=0, U=0
```

Alternately, FD,ED,FA,EA could be used, mostly to simplify mnemonics for stack
transfers.

```text
  ED  empty stack, descending   ;LDM: P=1, U=1  ;STM: P=0, U=0
  FD  full stack,  descending   ;     P=0, U=1  ;     P=1, U=0
  EA  empty stack, ascending    ;     P=1, U=0  ;     P=0, U=1
  FA  full stack,  ascending    ;     P=0, U=0  ;     P=1, U=1
```

Stack operations are conventionally using Rn=R13/SP as stack pointer in Full
Descending mode (meaning that free memory starts at SP-1 and below, and used
memory at SP+0 and up; that model is also used by other CPUs like 80x86 and
Z80). The following expressions are aliases for each other:

```text
  STMFD=STMDB=PUSH   STMED=STMDA   STMFA=STMIB   STMEA=STMIA
  LDMFD=LDMIA=POP    LDMED=LDMIB   LDMFA=LDMDA   LDMEA=LDMDB
```

**When S Bit is set (S=1)**

If instruction is LDM and R15 is in the list: (Mode Changes)

```text
  While R15 loaded, additionally: CPSR=SPSR_<current mode>
```

Otherwise: (User bank transfer)

```text
  Rlist is referring to User Bank Registers R0-R15 (rather than to registers
  of the current mode; such like R14_svc etc.)
  Base write-back should not be used for User bank transfer.
  Caution - When instruction is LDM:
  If the following instruction reads from a banked register (eg. R14_svc),
  then CPU might still read R14 instead; if necessary insert a dummy NOP.
```

**Transfer Order**

The lowest Register in Rlist (R0 if its in the list) will be loaded/stored
to/from the lowest memory address.

Internally, the rlist registers are always processed with sequentially
INCREASING addresses (ie. for DECREASING addressing modes, the CPU does first
calculate the lowest address, and does then process rlist with increasing
addresses; this detail can be important when accessing memory mapped I/O
ports).

**Mis-aligned STM,LDM,PUSH,POP (forced align)**

The base address should be usually word-aligned. Otherwise, mis-aligned low
bit(s) are ignored, the memory access goes to a forcibly aligned (rounded-down)
memory address "addr AND (NOT 3)".

**Strange Effects on Invalid Rlist's**

Empty Rlist: R15 loaded/stored (ARMv4 only), and Rb=Rb+/-40h (ARMv4-v5).

Writeback with Rb included in Rlist: Store OLD base if Rb is FIRST entry in
Rlist, otherwise store NEW base (STM/ARMv4), always store OLD base (STM/ARMv5),
no writeback (LDM/ARMv4), writeback if Rb is "the ONLY register, or NOT the
LAST register" in Rlist (LDM/ARMv5).

<a id="armopcodesmemorysingledatatransferldrstr"></a>

## ARM Opcodes: Memory: Single Data Transfer (LDR, STR)

**Opcode Format**

```text
  Bit    Expl.
  31-28  Condition
  27-26  Must be 01b for this instruction
  25     I - Immediate Offset Flag (0=Immediate, 1=Shifted Register)
  24     P - Pre/Post (0=post; add offset after transfer, 1=pre; before trans.)
  23     U - Up/Down Bit (0=down; subtract offset from base, 1=up; add to base)
  22     B - Byte/Word bit (0=transfer 32bit/word, 1=transfer 8bit/byte)
  When above Bit 24 P=0 (Post-indexing, write-back is ALWAYS enabled):
    21     T - Memory Management (0=Normal, 1=Force non-privileged access)
  When above Bit 24 P=1 (Pre-indexing, write-back is optional):
    21     W - Write-back bit (0=no write-back, 1=write address into base)
  20     L - Load/Store bit (0=Store to memory, 1=Load from memory)
          0: STR{cond}{B}{T} Rd,<Address>   ;[Rn+/-<offset>]=Rd
          1: LDR{cond}{B}{T} Rd,<Address>   ;Rd=[Rn+/-<offset>]
          Whereas, B=Byte, T=Force User Mode (only for POST-Indexing)
  19-16  Rn - Base register               (R0..R15) (including R15=PC+8)
  15-12  Rd - Source/Destination Register (R0..R15) (including R15=PC+12)
  When above I=0 (Immediate as Offset)
    11-0   Unsigned 12bit Immediate Offset (0-4095, steps of 1)
  When above I=1 (Register shifted by Immediate as Offset)
    11-7   Is - Shift amount      (1-31, 0=Special/See below)
    6-5    Shift Type             (0=LSL, 1=LSR, 2=ASR, 3=ROR)
    4      Must be 0 (Reserved, see The Undefined Instruction)
    3-0    Rm - Offset Register   (R0..R14) (not including PC=R15)
```

**Instruction Formats for &lt;Address&gt;**

An expression which generates an address:

```text
  <expression>                  ;an immediate used as address
  ;*** restriction: must be located in range PC+/-4095+8, if so,
  ;*** assembler will calculate offset and use PC (R15) as base.
```

Pre-indexed addressing specification:

```text
  [Rn]                          ;offset = zero
  [Rn, <#{+/-}expression>]{!}   ;offset = immediate
  [Rn, {+/-}Rm{,<shift>} ]{!}   ;offset = register shifted by immediate
```

Post-indexed addressing specification:

```text
  [Rn], <#{+/-}expression>      ;offset = immediate
  [Rn], {+/-}Rm{,<shift>}       ;offset = register shifted by immediate
```

Whereas...

```text
  <shift>  immediate shift such like LSL#4, ROR#2, etc. (see ALU opcodes).
  {!}      exclamation mark ("!") indicates write-back (Rn will be updated).
```

**Notes**

Shift amount 0 has special meaning, as described for ALU opcodes.

When writing a word (32bit) to memory, the address should be word-aligned.

When reading a byte from memory, upper 24 bits of Rd are zero-extended.

When reading a word from a halfword-aligned address (which is located in the
middle between two word-aligned addresses), the lower 16bit of Rd will contain
[address] ie. the addressed halfword, and the upper 16bit of Rd will contain
[Rd-2] ie. more or less unwanted garbage. However, by isolating lower bits this
may be used to read a halfword from memory. (Above applies to little endian
mode, as used in GBA.)

In a virtual memory based environment (ie. not in the GBA), aborts (ie. page
faults) may take place during execution, if so, Rm and Rn should not specify
the same register when post-indexing is used, as the abort-handler might have
problems to reconstruct the original value of the register.

Return: CPSR flags are not affected.

Execution Time: For normal LDR: 1S+1N+1I. For LDR PC: 2S+2N+1I. For STR: 2N.

**Mis-aligned 32bit STR (forced align)**

The mis-aligned low bit(s) are ignored, the memory access goes to a forcibly
aligned (rounded-down) memory address "addr AND (NOT 3)".

**Mis-aligned 32bit LDR (rotated read)**

Reads from forcibly aligned address "addr AND (NOT 3)", and does then rotate
the data as "ROR (addr AND 3)*8".

<a id="armopcodesmemorysingledataswapswp"></a>

## ARM Opcodes: Memory: Single Data Swap (SWP)

**Opcode Format**

```text
  Bit    Expl.
  31-28  Condition
  27-23  Must be 00010b for this instruction
         Opcode (fixed)
           SWP{cond}{B} Rd,Rm,[Rn]      ;Rd=[Rn], [Rn]=Rm
  22     B - Byte/Word bit (0=swap 32bit/word, 1=swap 8bit/byte)
  21-20  Must be 00b for this instruction
  19-16  Rn - Base register                     (R0-R14)
  15-12  Rd - Destination Register              (R0-R14)
  11-4   Must be 00001001b for this instruction
  3-0    Rm - Source Register                   (R0-R14)
```

SWP/SWPB supported by ARMv2a and up.

Swap works properly including if Rm and Rn specify the same register.

R15 may not be used for either Rn,Rd,Rm. (Rn=R15 would be MRS opcode).

Upper bits of Rd are zero-expanded when using Byte quantity. For info about
byte and word data memory addressing, read LDR and STR opcode description.

Execution Time: 1S+2N+1I. That is, 2N data cycles, 1S code cycle, plus 1I.

**Mis-aligned 32bit SWP (rotated read)**

The SWP opcode works like a combination of LDR and STR, that means, it does
read-rotated, but does write-unrotated.

<a id="armopcodescoprocessorinstructionsmrcmcrldcstccdp"></a>

## ARM Opcodes: Coprocessor Instructions (MRC/MCR, LDC/STC, CDP)

**Coprocessor Register Transfers (MRC, MCR) (with ARM Register read/write)**

```text
  Bit    Expl.
  31-28  Condition
  27-24  Must be 1110b for this instruction
  23-21  CP Opc - Coprocessor operation code         (0-7)
  20     ARM-Opcode (0-1)
          0: MCR{cond} Pn,<cpopc>,Rd,Cn,Cm{,<cp>}   ;move from ARM to CoPro
          1: MRC{cond} Pn,<cpopc>,Rd,Cn,Cm{,<cp>}   ;move from CoPro to ARM
  19-16  Cn     - Coprocessor source/dest. Register  (C0-C15)
  15-12  Rd     - ARM source/destination Register    (R0-R15)
  11-8   Pn     - Coprocessor number                 (P0-P15)
  7-5    CP     - Coprocessor information            (0-7)
  4      Reserved, must be one (1) (otherwise CDP opcode)
  3-0    Cm     - Coprocessor operand Register       (C0-C15)
```

MCR/MRC supported by ARMv2 and up.

A22i syntax allows to use MOV with Rd specified as first (dest), or last
(source) operand. Native MCR/MRC syntax uses Rd as middle operand, &lt;cp&gt;
can be ommited if &lt;cp&gt; is zero.

When using MCR with R15: Coprocessor will receive a data value of PC+12.

When using MRC with R15: Bit 31-28 of data are copied to Bit 31-28 of CPSR (ie.
N,Z,C,V flags), other data bits are ignored, CPSR Bit 27-0 are not affected,
R15 (PC) is not affected.

Execution time: 1S+bI+1C for MCR, 1S+(b+1)I+1C for MRC.

Return: For MRC only: Either R0-R14 modified, or flags affected (see above).

For details refer to original ARM docs. The opcodes irrelevant for GBA/NDS7
because no coprocessor exists (except for a dummy CP14 unit). However, NDS9
includes a working CP15 unit.

**Coprocessor Data Transfers (LDC, STC) (with Memory read/write)**

```text
  Bit    Expl.
  31-28  Condition
  27-25  Must be 110b for this instruction
  24     P - Pre/Post (0=post; add offset after transfer, 1=pre; before trans.)
  23     U - Up/Down Bit (0=down; subtract offset from base, 1=up; add to base)
  22     N - Transfer length (0-1, interpretation depends on co-processor)
  21     W - Write-back bit (0=no write-back, 1=write address into base)
  20     Opcode (0-1)
          0: STC{cond}{L} Pn,Cd,<Address>  ;Store to memory (from coprocessor)
          1: LDC{cond}{L} Pn,Cd,<Address>  ;Read from memory (to coprocessor)
          whereas {L} indicates long transfer (Bit 22: N=1)
  19-16  Rn     - ARM Base Register              (R0-R15)     (R15=PC+8)
  15-12  Cd     - Coprocessor src/dest Register  (C0-C15)
  11-8   Pn     - Coprocessor number             (P0-P15)
  7-0    Offset - Unsigned Immediate, step 4     (0-1020, in steps of 4)
```

LDC/STC supported by ARMv2 and up.

Execution time: (n-1)S+2N+bI, n=number of words transferred.

For details refer to original ARM docs, irrelevant in GBA because no
coprocessor exists.

**Coprocessor Data Operations (CDP) (without Memory or ARM Register operand)**

```text
  Bit    Expl.
  31-28  Condition
  27-24  Must be 1110b for this instruction
         ARM-Opcode (fixed)
           CDP{cond} Pn,<cpopc>,Cd,Cn,Cm{,<cp>}
  23-20  CP Opc - Coprocessor operation code       (0-15)
  19-16  Cn     - Coprocessor operand Register     (C0-C15)
  15-12  Cd     - Coprocessor destination Register (C0-C15)
  11-8   Pn     - Coprocessor number               (P0-P15)
  7-5    CP     - Coprocessor information          (0-7)
  4      Reserved, must be zero (otherwise MCR/MRC opcode)
  3-0    Cm     - Coprocessor operand Register     (C0-C15)
```

CDP supported by ARMv2 and up.

Execution time: 1S+bI, b=number of cycles in coprocessor busy-wait loop.

Return: No flags affected, no ARM-registers used/modified.

For details refer to original ARM docs, irrelevant in GBA because no
coprocessor exists.

<a id="armpseudoinstructionsanddirectives"></a>

## ARM Pseudo Instructions and Directives

**ARM Pseudo Instructions**

```text
  nop              mov r0,r0
  ldr Rd,=Imm      ldr Rd,[r15,disp] ;use .pool as parameter field
  add Rd,=addr     add/sub Rd,r15,disp
  adr Rd,addr      add/sub Rd,r15,disp
  adrl Rd,addr     two add/sub opcodes with disp=xx00h+00yyh
  mov Rd,Imm       mvn Rd,NOT Imm    ;or vice-versa
  and Rd,Rn,Imm    bic Rd,Rn,NOT Imm ;or vice-versa
  cmp Rd,Rn,Imm    cmn Rd,Rn,-Imm    ;or vice-versa
  add Rd,Rn,Imm    sub Rd,Rn,-Imm    ;or vice-versa
```

All above opcodes may be made conditional by specifying a {cond} field.

**A22i Directives**

```text
  org  adr     assume following code from this address on
  .gba         indicate GBA program
  .nds         indicate NDS program
  .fix         fix GBA/NDS header checksum
  .norewrite   do not delete existing output file (keep following data in file)
  .data?       following defines RAM data structure (assembled to nowhere)
  .code        following is normal ROM code/data (assembled to ROM image)
  .include     includes specified source code file (no nesting/error handling)
  .import      imports specified binary file (optional parameters: ,begin,len)
  .radix nn    changes default numeric format (nn=2,8,10,16 = bin/oct/dec/hex)
  .errif expr  generates an error message if expression is nonzero
  .if expr     assembles following code only if expression is nonzero
  .else        invert previous .if condition
  .endif       terminate .if/.ifdef/.ifndef
  .ifdef sym   assemble following only if symbol is defined
  .ifndef sym  assemble following only if symbol is not defined
  .align nn    aligns to an address divisible-by-nn, inserts 00's
  l equ n      l=n
  l:   [cmd]   l=$   (global label)
  @@l: [cmd]   @@l=$ (local label, all locals are reset at next global label)
  end          end of source code
  db ...       define 8bit data (bytes)
  dw ...       define 16bit data (halfwords)
  dd ...       define 32bit data (words)
  defs nn      define nn bytes space (zero-filled)
  ;...         defines a comment (ignored by the assembler)
  //           alias for CRLF, eg. allows <db 'Text',0 // dw addr> in one line
```

**A22i Alias Directives (for compatibility with other assemblers)**

```text
  align        .align 4          code16    .thumb
  align nn     .align nn         .code 16  .thumb
  % nn         defs nn           code32    .arm
  .space nn    defs nn           .code 32  .arm
  ..ds nn      defs nn           ltorg     .pool
  x=n          x equ n           .ltorg    .pool
  .equ x,n     x equ n           ..ltorg   .pool
  .define x n  x equ n           dcb       db (8bit data)
  incbin       .import           defb      db (8bit data)
  @@@...       ;comment          .byte     db (8bit data)
  @ ...        ;comment          .ascii    db (8bit string)
  @*...        ;comment          dcw       dw (16bit data)
  @...         ;comment          defw      dw (16bit data)
  .text        .code             .hword    dw (16bit data)
  .bss         .data?            dcd       dd (32bit data)
  .global      (ignored)         defd      dd (32bit data)
  .extern      (ignored)         .long     dd (32bit data)
  .thumb_func  (ignored)         .word     dw/dd, don't use
  #directive   .directive        .end      end
  .fill nn,1,0 defs nn
```

**Alias Conditions, Opcodes, Operands**

```text
  hs   cs   ;condition higher or same = carry set
  lo   cc   ;condition lower = carry cleared
  asl  lsl  ;arithmetic shift left = logical shift left
```

**A22i Numeric Formats &amp; Dialects**

```text
  Type          Normal       Alias
  Decimal       85           #85  &d85
  Hexadecimal   55h          #55h  0x55  #0x55  $55  &h55
  Octal         125o         0o125  &o125
  Ascii         'U'          "U"
  Binary        01010101b    %01010101  0b01010101  &b01010101
  Roman         &rLXXXV      (very useful for arrays of kings and chapters)
```

Note: The default numeric format can be changed by the .radix directive
(usually 10=decimal). For example, with radix 16, values like "85" and "0101b"
are treated as hexadecimal numbers (in that case, decimal and binary numbers
can be still defined with prefixes &amp;d and &amp;b).

**A22i Numeric Operators Priority**

```text
  Prio  Operator           Aliases
  8     (,) brackets
  7     +,- sign
  6     *,/,MOD,SHL,SHR    MUL,DIV,<<,>>
  5     +,- operation
  4     EQ,GE,GT,LE,LT,NE  =,>=,>,<=,<,<>,==,!=
  3     NOT
  2     AND
  1     OR,XOR             EOR
```

Operators of same priority are processed from left to right.

Boolean operators (priority 4) return 1=TRUE, 0=FALSE.

**A22i Nocash Syntax**

Even though A22i does recognize the official ARM syntax, it's also allowing to
use friendly code:

```text
  mov   r0,0ffh         ;no C64-style "#", and no C-style "0x" required
  stmia [r7]!,r0,r4-r5  ;square [base] brackets, no fancy {rlist} brackets
  mov   r0,cpsr         ;no confusing MSR and MRS (whatever which is which)
  mov   r0,p0,0,c0,c0,0 ;no confusing MCR and MRC (whatever which is which)
  ldr   r0,[score]      ;allows to use clean brackets for relative addresses
  push  rlist           ;alias for stmfd [r13]!,rlist (and same for pop/ldmfd)
  label:                ;label definitions recommended to use ":" colons
```

[A22i is the no$gba debug version's built-in source code assembler.]

<a id="arminstructioncycletimes"></a>

## ARM Instruction Cycle Times

Instruction Cycle Summary

```text
  Instruction      Cycles      Additional
  ---------------------------------------------------------------------
  ALU              1S          +1S+1N if R15 loaded, +1I if SHIFT(Rs)
  MSR,MRS          1S
  LDR              1S+1N+1I    +1S+1N if R15 loaded
  STR              2N
  LDM              nS+1N+1I    +1S+1N if R15 loaded
  STM              (n-1)S+2N
  SWP              1S+2N+1I
  B,BL             2S+1N
  SWI,trap         2S+1N
  MUL              1S+ml
  MLA              1S+(m+1)I
  MULL             1S+(m+1)I
  MLAL             1S+(m+2)I
  CDP              1S+bI
  LDC,STC          (n-1)S+2N+bI
  MCR              1N+bI+1C
  MRC              1S+(b+1)I+1C
  {cond} false     1S
```

Whereas,

```text
  n = number of words transferred
  b = number of cycles spent in coprocessor busy-wait loop
  m = depends on most significant byte(s) of multiplier operand
```

Above 'trap' is meant to be the execution time for exceptions. And '{cond}
false' is meant to be the execution time for conditional instructions which
haven't been actually executed because the condition has been false.

The separate meaning of the N,S,I,C cycles is:

**N - Non-sequential cycle**

Requests a transfer to/from an address which is NOT related to the address used
in the previous cycle. (Called 1st Access in GBA language).

The execution time for 1N is 1 clock cycle (plus non-sequential access
waitstates).

**S - Sequential cycle**

Requests a transfer to/from an address which is located directly after the
address used in the previous cycle. Ie. for 16bit or 32bit accesses at
incrementing addresses, the first access is Non-sequential, the following
accesses are sequential. (Called 2nd Access in GBA language).

The execution time for 1S is 1 clock cycle (plus sequential access waitstates).

**I - Internal Cycle**

CPU is just too busy, not even requesting a memory transfer for now.

The execution time for 1I is 1 clock cycle (without any waitstates).

**C - Coprocessor Cycle**

The CPU uses the data bus to communicate with the coprocessor (if any), but no
memory transfers are requested.

**Memory Waitstates**

Ideally, memory may be accessed free of waitstates (1N and 1S are then equal to
1 clock cycle each). However, a memory system may generate waitstates for
several reasons: The memory may be just too slow. Memory is currently accessed
by DMA, eg. sound, video, memory transfers, etc. Or when data is squeezed
through a 16bit data bus (in that special case, 32bit access may have more
waitstates than 8bit and 16bit accesses). Also, the memory system may separate
between S and N cycles (if so, S cycles would be typically faster than N
cycles).

**Memory Waitstates for Different Memory Areas**

Different memory areas (eg. ROM and RAM) may have different waitstates. When
executing code in one area which accesses data in another area, then the S+N
cycles must be split into code and data accesses: 1N is used for data access,
plus (n-1)S for LDM/STM, the remaining S+N are code access. If an instruction
jumps to a different memory area, then all code cycles for that opcode are
having waitstate characteristics of the NEW memory area.

<a id="armversions"></a>

## ARM Versions

**Version Numbers**

ARM CPUs are distributed by name ARM#, and are described as ARMv# in
specifications, whereas "#" is NOT the same than "v#", for example, ARM7TDMI is
ARMv4TM. That is so confusing, that ARM didn't even attempt to clarify the
relationship between the various "#" and "v#" values.

**Version Variants**

Suffixes like "M" (long multiply), "T" (THUMB support), "E" (Enhanced DSP)
indicate presence of special features, additionally to the standard instruction
set of a given version, or, when preceded by an "x", indicate the absence of
that features.

**ARMv1 aka ARM1**

Some sort of a beta version, according to ARM never been used in any commercial
products.

**ARMv2 and up**

MUL,MLA

CDP,LDC,MCR,MRC,STC

SWP/SWPB (ARMv2a and up only)

Two new FIQ registers

**ARMv3 and up**

MRS,MSR opcodes (instead CMP/CMN/TST/TEQ{P} opcodes)

CPSR,SPSR registers (instead PSR bits in R15)

Removed never condition, cond=NV no longer valid

32bit addressing (instead 26bit addressing in older versions)

26bit addressing backwards comptibility mode (except v3G)

Abt and Und modes (instead handling aborts/undefined in Svc mode)

SMLAL,SMULL,UMLAL,UMULL (optionally, INCLUDED in v3M, EXCLUDED in v4xM/v5xM)

**ARMv4 aka ARM7 and up**

LDRH,LDRSB,LDRSH,STRH

Sys mode (privileged user mode)

BX (only ARMv4T, and any ARMv5 or ARMv5T and up)

THUMB code (only T variants, ie. ARMv4T, ARMv5T)

**ARMv5 aka ARM9 and up**

BKPT,BLX,CLZ (BKPT,BLX also in THUMB mode)

LDM/LDR/POP PC with mode switch (POP PC also in THUMB mode)

CDP2,LDC2,MCR2,MRC2,STC2 (new coprocessor opcodes)

C-flag unchanged by MUL (instead undefined flag value)

changed instruction cycle timings / interlock ??? or not ???

QADD,QDADD,QDSUB,QSUB opcodes, CPSR.Q flag (v5TE and V5TExP only)

SMLAxy,SMLALxy,SMLAWy,SMULxy,SMULWy (v5TE and V5TExP only)

LDRD,STRD,PLD,MCRR,MRRC (v5TE only, not v5, not v5TExP)

**ARMv6**

No public specifications available.

**A Milestone in Computer History**

Original ARMv2 has been used in the relative rare and expensive Archimedes
deluxe home computers in the late eighties, the Archimedes has caught a lot of
attention, particularly for being the first home computer that used a BIOS
being programmed in BASIC language - which has been a absolutely revolutionary
decadency at that time.

Inspired, programmers all over the world have successfully developed even
slower and much more inefficient programming languages, which are nowadays
consequently used by nearly all ARM programmers, and by most non-ARM
programmers as well.

<a id="snescartobc1objcontroller1game"></a>

## SNES Cart OBC1 (OBJ Controller) (1 game)

The OBC1 is a 80pin OBJ Controller chip from Nintendo, used by only one game:

```text
  Metal Combat: Falcon's Revenge (1993) Intelligent Systems/Nintendo
  (Note: the game also requires a Super Scope lightgun)
```

**OBC1 I/O Ports**

```text
  7FF0h OAM Xloc = [Base+Index*4+0]  (R/W)
  7FF1h OAM Yloc = [Base+Index*4+1]  (R/W)
  7FF2h OAM Tile = [Base+Index*4+2]  (R/W)
  7FF3h OAM Attr = [Base+Index*4+3]  (R/W)
  7FF4h OAM Bits = [Base+Index/4+200h].Bit((Index AND 3)*2+0..1) (R?/W)
  7FF5h Base for 220h-byte region (bit0: 0=7C00h, 1=7800h)
  7FF6h Index (OBJ Number) (0..127)
  7FF7h Unknown (set to 00h or 0Ah) (maybe SRAM vs I/O mode select)
```

Other bytes at 6000h..7FFFh contain 8Kbyte battery-backed SRAM (of which,
7800h..7A1Fh and 7C00h..7E1Fh can be used as OBJ workspace).

**Notes**

Port 7FF0h-7FF3h/7FF5h are totally useless. Port 7FF4h/7FF6h are eventually
making it slightly easier to combine the 2bit OAM fragments, though putting a
huge 80pin chip into the cartridge for merging 2bit fragments is definetly
overcomplicated.

As far as known, the Index isn't automatically incremented. Port 7FF4h does
read-modify-write operations which may involve timing restrictions (?), or,
modify-write (when prefetching data on 7FF6h writes) which may come up with
out-dated-prefetch effects.

Reading from 7FF4h does reportedly return the desired BYTE, but WITHOUT
isolating &amp; shifting the desired BITS into place?

Setting Index bits7+5 does reportedly enable SRAM mapping at 6000h..77FFh?

ROM is reportedly mapped to bank 00h..3Fh, and also to bank 70h..71h? Maybe
that info just refers to SRAM not being mapped to that region (as it'd be in
some other LoROM cartridges).

**PCB "SHVC-2E3M-01"**

Contains six chips and a battery. The chips are: Two 1MB ROMs, MAD-1, OBC1,
CIC, 8K SRAM. All chips (except MAD-1) are SMD chips.

<a id="snescartsdd1datadecompressor2games"></a>

## SNES Cart S-DD1 (Data Decompressor) (2 games)

The S-DD1 is a 100pin Data Decompression chip, used by only two games:

```text
  Star Ocean (6MB ROM, 8KB RAM) (1996) tri-Ace/Enix (JP)
  Street Fighter Alpha 2 (4MB ROM, no RAM) (1996) Capcom (NA) (JP) (EU)
```

**S-DD1 Decompression Algorithm**

[SNES Cart S-DD1 Decompression Algorithm](#snes-cart-s-dd1-decompression-algorithm)

**S-DD1 I/O Ports**

```text
  4800h  DMA Enable 1 (bit0..7 = DMA 0..7) (unchanged after DMA)
  4801h  DMA Enable 2 (bit0..7 = DMA 0..7) (automatically cleared after DMA)
  4802h  Unknown   ;\set to 0000h by Star Ocean (maybe SRAM related)
  4803h  Unknown   ;/unused by Street Fighter Alpha 2
  4804h  ROM Bank for C00000h-CFFFFFh (in 1MByte units)
  4805h  ROM Bank for D00000h-DFFFFFh (in 1MByte units)
  4806h  ROM Bank for E00000h-EFFFFFh (in 1MByte units)
  4807h  ROM Bank for F00000h-FFFFFFh (in 1MByte units)
  <DMA>  DMA from ROM returns Decompressed Data (originated at DMA start addr)
```

**S-DD1 Memory Map**

```text
  ???-???          SRAM (if any)
  008000h-00FFFFh  Exception Handlers, mapped in LoROM-fashion (ROM 0..7FFFh)
  C00000h-CFFFFFh  ROM (mapped via Port 4804h) (in HiROM fashion)
  D00000h-DFFFFFh  ROM (mapped via Port 4805h) (in HiROM fashion)
  E00000h-EFFFFFh  ROM (mapped via Port 4806h) (in HiROM fashion)
  F00000h-FFFFFFh  ROM (mapped via Port 4807h) (in HiROM fashion)
```

**S-DD1 PCBs**

```text
  SHVC-1NON-01  CartSlotPin59 not connected (no C12 capacitor on PA1 pin)
  SHVC-1NON-10  Strange revision (capacitor C12 between PA1 and GND)
  SNSP-1NON-10  PAL version (S-DD1.Pin82 wired to ... VCC?) (also with C12)
  SHVC-LN3B-01  Version with additional SRAM for Star Ocean
```

The 1NON board contains only two chips (100pin D-DD1 and 44pin ROM), the CIC
function is included in the S-DD1, whereas Pin82 does probably select
"PAL/NTSC" CIC mode.

The LN3B-board contains five chips (two 44pin ROMs, S-DD1, 8Kx8bit SRAM, and a
MM1026AF battery controller).

**S-DD1 Pinouts**

```text
  1-81   Unknown
  82     PAL/NTSC (for CIC mode)
  83-100 Unknown
```

<a id="snescartsdd1decompressionalgorithm"></a>

## SNES Cart S-DD1 Decompression Algorithm

**decompress_init(src)**

```text
  input=[src], src=src+1
  if (input AND C0h)=00h then num_planes = 2
  if (input AND C0h)=40h then num_planes = 8
  if (input AND C0h)=80h then num_planes = 4
  if (input AND C0h)=C0h then num_planes = 0
  if (input AND 30h)=00h then high_context_bits=01c0h, low_context_bits=0001h
  if (input AND 30h)=10h then high_context_bits=0180h, low_context_bits=0001h
  if (input AND 30h)=20h then high_context_bits=00c0h, low_context_bits=0001h
  if (input AND 30h)=30h then high_context_bits=0180h, low_context_bits=0003h
  input=(input SHL 11) OR ([src+1] SHL 3), src=src+1, valid_bits=5
  for i=0 to 7 do bit_ctr[i]=00h, prev_bits[i]=0000h
  for i=0 to 31 do context_states[i]=00h, context_MPS[i]=00h
  plane=0, yloc=0, raw=0
```

**decompress_byte(src,dst)**

```text
  if num_planes=0
    for plane=0 to 7 do GetBit(plane)
    [dst]=raw, dst=dst+1
  else if (plane AND 1)=0
    for i=0 to 7 do GetBit(plane+0), GetBit(plane+1)
    [dst]=prev_bits[plane] AND FFh, dst=dst+1, plane=plane+1
  else
    [dst]=prev_bits[plane] AND FFh, dst=dst+1, plane=plane-1
    yloc=yloc+1, if yloc=8 then yloc=0, plane = (plane+2) AND (num_planes-1)
```

**GetBit(plane)**

```text
  context = (plane AND 1) SHL 4
  context = context OR ((prev_bits[plane] AND high_context_bits) SHR 5)
  context = context OR (prev_bits[plane] AND low_context_bits)
  pbit=ProbGetBit(context)
  prev_bits[plane] = (prev_bits[plane] SHL 1) + pbit
  if num_planes=0 then raw = (raw SHR 1)+(pbit SHL 7)
```

**ProbGetBit(context)**

```text
  state=context_states[context]
  code_size=EvolutionCodeSize[state]
  if (bit_ctr[code_size] AND 7Fh)=0 then
    bit_ctr[code_size]=GetCodeword(code_size)
  pbit=context_MPS[context]
  bit_ctr[code_size] = bit_ctr[code_size]-1
  if bit_ctr[code_size]=00h    ;"GolombGetBit"
    context_states[context]=EvolutionLpsNext[state]
    pbit=pbit XOR 1
    if state<2 then context_MPS[context]=pbit
  else if bit_ctr[code_size]=80h
    context_states[context]=EvolutionMpsNext[state]
  return pbit
```

**GetCodeword(code_size)**

```text
  if valid_bits=0 then input=input OR [src], src=src+1, valid_bits=8
  input=input SHL 1, valid_bits=valid_bits-1
  if (input AND 8000h)=0 return 80h+(1 SHL code_size)
  tmp=((input SHR 8) AND 7Fh) OR (7Fh SHR code_size)
  input=input SHL code_size, valid_bits=valid_bits-code_size
  if valid_bits<0 then
    input=input OR (([src] SHL (-valid_bits))
    src=src+1, valid_bits=valid_bits+8
  return RunTable[tmp]
```

**EvolutionCodeSize[0..32]**

```text
  0 , 0, 0, 0, 0, 1, 1, 1, 1, 2, 2, 2, 2, 3, 3, 3, 3
  4 , 4, 5, 5, 6, 6, 7, 7, 0, 1, 2, 3, 4, 5, 6, 7
```

**EvolutionMpsNext[0..32]**

```text
  25, 2, 3, 4, 5, 6, 7, 8, 9,10,11,12,13,14,15,16,17
  18,19,20,21,22,23,24,24,26,27,28,29,30,31,32,24
```

**EvolutionLpsNext[0..32]**

```text
  25, 1, 1, 2, 3, 4, 5, 6, 7, 8, 9,10,11,12,13,14,15
  16,17,18,19,20,21,22,23, 1, 2, 4, 8,12,16,18,22
```

**RunTable[0..127]**

```text
  128, 64, 96, 32, 112, 48, 80, 16, 120, 56, 88, 24, 104, 40, 72, 8
  124, 60, 92, 28, 108, 44, 76, 12, 116, 52, 84, 20, 100, 36, 68, 4
  126, 62, 94, 30, 110, 46, 78, 14, 118, 54, 86, 22, 102, 38, 70, 6
  122, 58, 90, 26, 106, 42, 74, 10, 114, 50, 82, 18,  98, 34, 66, 2
  127, 63, 95, 31, 111, 47, 79, 15, 119, 55, 87, 23, 103, 39, 71, 7
  123, 59, 91, 27, 107, 43, 75, 11, 115, 51, 83, 19,  99, 35, 67, 3
  125, 61, 93, 29, 109, 45, 77, 13, 117, 53, 85, 21, 101, 37, 69, 5
  121, 57, 89, 25, 105, 41, 73,  9, 113, 49, 81, 17,  97, 33, 65, 1
```

<a id="snescartspc7110datadecompressor3games"></a>

## SNES Cart SPC7110 (Data Decompressor) (3 games)

The SPC7110 (full name "SPC7110F0A" or "SPC7110Foa") is a 100pin Data
Decompression chip from Epson/Seiko, used only by three games from Hudson soft:

```text
  Far East of Eden Zero (with RTC-4513) (1995) Red Company/Hudson Soft (JP)
  Momotaro Dentetsu Happy (1996) Hudson Soft (JP)
  Super Power League 4 (1996) Hudson Soft (JP)
```

XXX add info from byuu's "spc7110-mcu.txt" file.

[SNES Cart SPC7110 Memory and I/O Map](#snes-cart-spc7110-memory-and-io-map)

[SNES Cart SPC7110 Decompression I/O Ports](#snes-cart-spc7110-decompression-io-ports)

[SNES Cart SPC7110 Direct Data ROM Access](#snes-cart-spc7110-direct-data-rom-access)

[SNES Cart SPC7110 Multiply/Divide Unit](#snes-cart-spc7110-multiplydivide-unit)

[SNES Cart SPC7110 with RTC-4513 Real Time Clock (1 game)](#snes-cart-spc7110-with-rtc-4513-real-time-clock-1-game)

[SNES Cart SPC7110 Decompression Algorithm](#snes-cart-spc7110-decompression-algorithm)

[SNES Cart SPC7110 Notes](#snes-cart-spc7110-notes)

**Pinouts**

[SNES Pinouts Decompression Chips](80-timings-unpredictable-pinouts.md#snes-pinouts-decompression-chips)

<a id="snescartspc7110memoryandiomap"></a>

## SNES Cart SPC7110 Memory and I/O Map

**Memory Map**

```text
  4800h..4842h      SPC7110 I/O Ports
  6000h..7FFFh      Battery-backed SRAM (8K bytes, in all 3 games)
  8000h..FFFFh      Exception Handlers (Program ROM offset 8000h..FFFFh)
  C00000h..CFFFFFh  Program ROM (1MByte) (HiROM)
  D00000h..DFFFFFh  Data ROM (1MByte-fragment mapped via Port 4831h)
  E00000h..EFFFFFh  Data ROM (1MByte-fragment mapped via Port 4832h)
  F00000h..FFFFFFh  Data ROM (1MByte-fragment mapped via Port 4833h)
```

I/O Ports and SRAM are probably mirrored to banks 00h-3Fh and 80h-BFh.

Program/Data ROM is probably mirrored to 400000h-7FFFFFh, the upper 32K
fragments of each 64K bank probably also to banks 00h-3Fh and 80h-BFh.

**Reportedly (probably nonsense?)**

"data decompressed from data rom by spc7110 mapped to $50:0000-$50:FFFF".

That info would imply that "decompressed data" Port 4800h is mirrored to
500000h-5FFFFFh (though more likely, the "un-decompressed data" is mirrored
from D00000h-DFFFFFh).

**ROM-Image Format**

The existing SPC7110 games are 2MB, 3MB, 5MB in size. Stored like so:

```text
  000000h..0FFFFFh  Program ROM (1MByte) (HiROM)
  100000h..xFFFFFh  Data ROM (1MByte, 2MByte, or 4MByte max)
```

Observe that the SPC7110 ROM checksums at [FFDCh..FFDFh] are calculated
unconventionally: 3MB/5MB aren't "rounded-up" to 4MB/8MB. Instead, 3MB is
checksummed twice (rounded to 6MB). 2MB/5MB are checksummed as 2MB/5MB (without
rounding).

**Data ROM Decompression Ports**

```text
  4800h --  Decompressed Data Read
  4801h 00  Compressed Data ROM Directory Base, bit0-7
  4802h 00  Compressed Data ROM Directory Base, bit8-15
  4803h 00  Compressed Data ROM Directory Base, bit16-23
  4804h 00  Compressed Data ROM Directory Index
  4805h 00  Decompressed Data RAM Target Offset, bit0-7    OFFSET IN BANK $50
  4806h 00  Decompressed Data RAM Target Offset, bit8-15   OFFSET IN BANK $50
  4807h 00  Unknown ("DMA Channel for Decompression")
  4808h 00  Unknown ("C r/w option, unknown")
  4809h 00  Decompressed Data Length Counter, bit0-7
  480Ah 00  Decompressed Data Length Counter, bit8-15
  480Bh 00  Unknown ("Decompression Mode")
  480Ch 00  Decompression Status (bit7: 0=Busy/Inactive, 1=Ready/DataAvailable)
```

**Direct Data ROM Access**

```text
  4810h 00  Data ROM Read from [Base] or [Base+Offs], and increase Base or Offs
  4811h 00  Data ROM Base, bit0-7   (R/W)
  4812h 00  Data ROM Base, bit8-15  (R/W)
  4813h 00  Data ROM Base, bit16-23 (R/W)
  4814h 00  Data ROM Offset, bit0-7   ;\optionally Base=Base+Offs
  4815h 00  Data ROM Offset, bit8-15  ;/on writes to both of these registers
  4816h 00  Data ROM Step, bit0-7
  4817h 00  Data ROM Step, bit8-15
  4818h 00  Data ROM Mode
  481Ah 00  Data ROM Read from [Base+Offset], and optionally set Base=Base+Offs
```

**Unsigned Multiply/Divide Unit**

```text
  4820h 00  Dividend, Bit0-7 / Multiplicand, Bit0-7
  4821h 00  Dividend, Bit8-15 / Multiplicand, Bit8-15
  4822h 00  Dividend, Bit16-23
  4823h 00  Dividend, Bit24-31
  4824h 00  Multiplier, Bit0-7
  4825h 00  Multiplier, Bit8-15, Start Multiply on write to this register
  4826h 00  Divisor, Bit0-7
  4827h 00  Divisor, Bit8-15, Start Division on write to this register
  4828h 00  Multiply/Divide Result, Bit0-7
  4829h 00  Multiply/Divide Result, Bit8-15
  482Ah 00  Multiply/Divide Result, Bit16-23
  482Bh 00  Multiply/Divide Result, Bit24-31
  482Ch 00  Divide Remainder, Bit0-7
  482Dh 00  Divide Remainder, Bit8-15
  482Eh 00  Multiply/Divide Reset  (write = reset 4820h..482Dh) (write 00h)
  482Fh 00  Multiply/Divide Status (bit7: 0=Ready, 1=Busy)
```

**Memory Mapping**

```text
  4830h 00  SRAM Chip Enable/Disable (bit7: 0=Disable, 1=Enable)
  4831h 00  Data ROM Bank for D00000h-DFFFFFh (1MByte, using HiROM mapping)
  4832h 01  Data ROM Bank for E00000h-EFFFFFh (1MByte, using HiROM mapping)
  4833h 02  Data ROM Bank for F00000h-FFFFFFh (1MByte, using HiROM mapping)
  4834h 00  SRAM Bank Mapping?, workings unknown
```

**Real-Time Clock Ports (for external RTC-4513)**

```text
  4840h 00  RTC Chip Enable/Disable (bit0: 0=Disable, 1=Enable)
  4841h --  RTC Command/Index/Data Port
  4842h --  RTC Ready Status
```

<a id="snescartspc7110decompressionioports"></a>

## SNES Cart SPC7110 Decompression I/O Ports

**4800h - Decompressed Data Read**

Reading from this register returns one decompressed byte, and does also
decrease the 16bit length counter [4809h] by one.

**4801h - Compressed Data ROM Directory Base, bit0-7**

**4802h - Compressed Data ROM Directory Base, bit8-15**

**4803h - Compressed Data ROM Directory Base, bit16-23**

**4804h - Compressed Data ROM Directory Index**

Selects a directory entry in Data ROM at [Base+Index*4]. Each entry is 4-bytes
in size:

```text
  Byte0  Decompression Mode (00h,01h,02h)
  Byte1  Compressed Data ROM Source Pointer, bit16-23  ;\ordered as so
  Byte2  Compressed Data ROM Source Pointer, bit8-15   ; (ie. big-endian)
  Byte3  Compressed Data ROM Source Pointer, bit0-7    ;/
```

**4805h - Decompressed Data RAM Target Offset, bit0-7    OFFSET IN BANK $50**

**4806h - Decompressed Data RAM Target Offset, bit8-15   OFFSET IN BANK $50**

Reportedly: Destination address in bank 50h, this would imply that the SPC7110
chip contains around 64Kbytes on-chip RAM, which is probably utmost nonsense.

Or, reportedly, too: Causes the first "N" decompressed bytes to be skipped,
before data shows up at 4800h. That sounds more or less reasonable. If so,
unknown if the hardware does decrement the offset value?

**4807h - DMA Channel for Decompression**

Unknown. Reportedly "DMA CHANNEL FOR DECOMPRESSION, set to match snes dma
channel used for compressed data". That info seems to be nonsense; the
registers seems to be always set to 00h, no matter if/which DMA channel is
used.

**4808h - C r/w option, unknown**

Unknown. Reportedly "C r/w option, unknown".

**4809h - Decompressed Data Length Counter, bit0-7**

**480Ah - Decompressed Data Length Counter, bit8-15**

This counter is decremented on reads from [4800h]. One can initialize the
counter before decompression &amp; check its value during decompression.
However, this doesn't seem to be required hardware-wise; the decompression
seems to be working endless (as long as software reads [4800h]), and doesn't
seem to "stop" when the length counter becomes zero.

**480Bh - Decompression Mode**

Reportedly:

```text
  00 - manual decompression, $4800 is used to read directly from the data rom
```

```text
  02 - hardware decompression, decompressed data is mapped to $50:0000,
       $4800 can be used to read sequentially from bank $50
```

**480Ch - Decompression Status (bit7: 0=Busy/Inactive, 1=Ready/DataAvailable)**

Reportedly:

```text
  DECOMPRESSION FINISHED STATUS:
  high bit set = done, high bit clear = processing,
  cleared after successful read,
  high bit is cleared after writing to $4806,
  $4809/A is set to compressed data length
  ---
  decompression mode is activated after writing to $4806
  and finishes after reading the high bit of $480C
```

<a id="snescartspc7110directdataromaccess"></a>

## SNES Cart SPC7110 Direct Data ROM Access

**4810h Data ROM Read from [Base] or [Base+Offs], and increase Base or Offs**

**481Ah Data ROM Read from [Base+Offset], and optionally set Base=Base+Offs**

Reportedly,

Testing leads to believe that the direct ROM read section starts out as
inactive.

One of the ways to activate direct reads is to write a non-zero value to $4813.

No other action need be taken. You can write a non-zero value and immediately

write a zero to it and that's OK.  The order of writes to $4811/2/3 don't

seem to matter so long as $4813 has been written to once with a non-zero

value.  There may be a way to deactivate the direct reads again (maybe a

decompression cycle?).

There appears to be another way to activate direct reads that is more complex.

**4811h Data ROM Base, bit0-7   (R/W)**

**4812h Data ROM Base, bit8-15  (R/W)**

**4813h Data ROM Base, bit16-23 (R/W)**

**4814h Data ROM Offset, bit0-7   ;\optionally Base=Base+Offs**

**4815h Data ROM Offset, bit8-15  ;/on writes to both of these registers**

**4816h Data ROM Step, bit0-7**

**4817h Data ROM Step, bit8-15**

**4818h Data ROM Mode**

```text
  0   Select Step   (for 4810h) (0=Increase by 1, 1=Increase by "Step" Value)
  1   Enable Offset (for 4810h) (0=Disable/Read Ptr, 1=Enable/Read Ptr+Offset)
  2   Expand Step from 16bit to 24bit           (0=Zero-expand, 1=Sign-expand)
  3   Expand Offset from 8bit?/16bit to 24bit   (0=Zero-expand, 1=Sign-expand)
  4   Apply Step (after 4810h read)    (0=On 24bit Pointer, 1=On 16bit Offset)
  5-6 Special Actions (see below)
  7   Unused (should be zero)
```

Special Actions:

```text
  0=No special actions
  1=After Writing $4814/5 --> 8 bit offset addition using $4814
  2=After Writing $4814/5 --> 16 bit offset addition using $4814/5
  3=After Reading $481A   --> 16 bit offset addition using $4814/5
```

Reportedly,

```text
  4818 write: set command mode,
  4818 read: performs action instead of returning value, unknown purpose
  command mode is loaded to $4818 but only set after writing to both $4814
  and $4815 in any order
  $4811/2/3 may increment on a $4810 read depending on mode byte)
  $4814/$4815 is sometimes incremented on $4810 reads (depending on mode byte)
```

Note: the data rom command mode is activated only after registers $4814 and
$4815 have been written to, regardless of the order they were written to

**4831h Data ROM Bank for D00000h-DFFFFFh (1MByte, using HiROM mapping)**

**4832h Data ROM Bank for E00000h-EFFFFFh (1MByte, using HiROM mapping)**

**4833h Data ROM Bank for F00000h-FFFFFFh (1MByte, using HiROM mapping)**

**4830h SRAM Chip Enable/Disable (bit7: 0=Disable, 1=Enable)**

**4834h SRAM Bank Mapping?, workings unknown**

<a id="snescartspc7110multiplydivideunit"></a>

## SNES Cart SPC7110 Multiply/Divide Unit

**Unsigned Multiply/Divide Unit**

```text
  4820h Dividend, Bit0-7 / Multiplicand, Bit0-7
  4821h Dividend, Bit8-15 / Multiplicand, Bit8-15
  4822h Dividend, Bit16-23
  4823h Dividend, Bit24-31
  4824h Multiplier, Bit0-7
  4825h Multiplier, Bit8-15, Start Multiply on write to this register
  4826h Divisor, Bit0-7
  4827h Divisor, Bit8-15, Start Division on write to this register
  4828h Multiply/Divide Result, Bit0-7
  4829h Multiply/Divide Result, Bit8-15
  482Ah Multiply/Divide Result, Bit16-23
  482Bh Multiply/Divide Result, Bit24-31
  482Ch Divide Remainder, Bit0-7
  482Dh Divide Remainder, Bit8-15
  482Eh Multiply/Divide Reset  (write = reset 4820h..482Dh) (write 00h)
  482Fh Multiply/Divide Status (bit7: 0=Ready, 1=Busy)
```

**Unknown Stuff**

Multiply/Divide execution time is unknown. Is it constant/faster for small
values? Behaviour on Divide by 0 is unknown?

Purpose of 482Eh is unknown, does it really "reset" 4820h..482Dh? Meaning that
those registers are set to zero? Is that required/optional?

Are there other modes, like support for signed-numbers, or a fast 8bit*8bit
multiply mode or such?

**Reportedly**

```text
  482Eh.bit0  (0=unsigned, 1=signed)
  (un)signed div0 returns --> result=00000000h, remainder=dividend AND FFFFh
  -80000000h/-1 returns <unknown> ?
```

<a id="snescartspc7110withrtc4513realtimeclock1game"></a>

## SNES Cart SPC7110 with RTC-4513 Real Time Clock (1 game)

RTC from Epson/Seiko. Used by one game from Hudson Soft:

```text
  Far East of Eden Zero (with RTC-4513) (1995) Red Company/Hudson Soft (JP)
```

**SPC7110 I/O Ports for RTC-4513 Access**

```text
  4840h RTC Chip Select (bit0: 0=Deselect: CE=LOW, 1=Select: CE=HIGH)
  4841h RTC Data Port   (bit0-3: Command/Index/Data)
  4842h RTC Status      (bit7: 1=Ready, 0=Busy) (for 4bit transfers)
```

**Usage**

Switch CE from LOW to HIGH, send Command (03h=Write, 0Ch=Read), send starting
Index (00h..0Fh), then read or write one or more 4bit Data units (index will
automatically increment after each access, and wraps from 0Fh to 00h at end of
data stream). Finally, switch CE back LOW.

**Epson RTC-4513 Commands**

```text
  03h    Write-Mode
  0Ch    Read-Mode
```

**Epson RTC-4513 Register Table**

```text
  Index  Bit3   Bit2   Bit1    Bit0   Expl.
  0      Sec3   Sec2   Sec1    Sec0   Seconds, Low
  1      LOST   Sec6   Sec5    Sec4   Seconds, High
  2      Min3   Min2   Min1    Min0   Minutes, Low
  3      WRAP   Min6   Min5    Min4   Minutes, High
  4      Hour3  Hour2  Hour1   Hour0  Hours, Low
  5      WRAP   PM/AM  Hour5   Hour4  Hours, High
  6      Day3   Day2   Day0    Day0   Day, Low     ;\
  7      WRAP   RAM    Day5    Day4   Day, High    ;
  8      Mon3   Mon2   Mon1    Mon0   Month, Low   ; or optionally,
  9      WRAP   RAM    RAM     Mon4   Month, High  ; 6x4bit User RAM
  A      Year3  Year2  Year1   Year0  Year, Low    ;
  B      Year7  Year6  Year5   Year4  Year, High   ;/
  C      WRAP   Week2  Week1   Week0  Day of Week
  D      30ADJ  IRQ-F  CAL/HW  HOLD   Control Register D
  E      RATE1  RATE0  DUTY    MASK   Control Register E
  F      TEST   24/12  STOP    RESET  Control Register F
```

Whereas, the meaning of the various bits is:

```text
  Sec    Seconds (BCD, 00h..59h)
  Min    Minutes (BCD, 00h..59h)
  Hour   Hours   (BCD, 00h..23h or 01h..12h)
  Day    Day     (BCD, 01h..31h)
  Month  Month   (BCD, 01h..12h)
  Year   Year    (BCD, 00h..99h)
  Week   Day of Week (0..6) (Epson suggests 0=Monday as an example)
  PM/AM  Set for PM, cleared for AM (is that also in 24-hour mode?)
  WRAP   Time changed during access (reset on CE=LOW, set on seconds increase)
  HOLD   Pause clock when set (upon clearing increase seconds by 1 if needed)
  LOST   Time lost (eg. battery failure) (can be reset by writing 0)
  IRQ-F  Interrupt Flag (Read-only, set when: See Rate, cleared when: See Duty)
  RATE   Interrupt Rate (0=Per 1/64s, 1=Per Second, 2=Per Minute, 3=Per Hour)
  DUTY   Interrupt Duty (0=7.8ms, 1=Until acknowledge, ie. until IRQ-F read)
  MASK   Interrupt Disable (when set: IRQ-F always 0, STD.P always High-Z)
  TEST   Reserved for Epson's use (should be 0) (auto-cleared on CE=LOW)
  RAM    General purpose RAM (usually 3bits) (24bits when Calendar=off)
  CAL/HW Calendar Enable (1=Yes/Normal, 0=Use Day/Mon/Year as 24bit user RAM)
  24/12  24-Hour Mode (0=12, 1=24) (Time/Date may get corrupted when changed!)
  30ADJ  Set seconds to zero, and, if seconds was>=30, increase minutes
  STOP   Stop clock while set (0=Stop, 1=Normal)
  RESET  Stop clock and reset seconds to 00h (auto-cleared when CE=LOW)
```

If WRAP=1 then one must deselect the chip, and read time/date again.

Serial data is transferred LSB first.

On-chip 32.768kHz quartz crystal.

**Pin-Outs**

[SNES Pinouts RTC Chips](80-timings-unpredictable-pinouts.md#snes-pinouts-rtc-chips)

<a id="snescartspc7110decompressionalgorithm"></a>

## SNES Cart SPC7110 Decompression Algorithm

**decompress_mode0(src,dst,len)**

```text
  initialize
  while len>0
    decoded=0
    con=0, decompression_core
    con=1+decoded, decompression_core
    con=3+decoded, decompression_core
    con=7+decoded, decompression_core
    out = (out SHL 4) XOR (((out SHR 12) XOR decoded) AND Fh)
    decoded=0
    con=15, decompression_core
    con=15+1+decoded, decompression_core
    con=15+3+decoded, decompression_core
    con=15+7+decoded, decompression_core
    out = (out SHL 4) XOR (((out SHR 12) XOR decoded) AND Fh)
    [dst]=(out AND FFh), dst=dst+1, len=len-1
```

**decompress_mode1(src,dst,len)**

```text
  initialize
  while len>0
   if (buf_index AND 01h)=0
    for pixel=0 to 7
      a = (out SHR 2)  AND 03h
      b = (out SHR 14) AND 03h
      decoded=0
      con = get_con(a,b,c)
      decompression_core
      con = con*2+5+decoded
      decompression_core
      do_pixel_order(a,b,c,2,decoded)
    plane0.bits(7..0) = out.bits(15,13,11,9,7,5,3,1)
    plane1.bits(7..0) = out.bits(14,12,10,8,6,4,2,0)
    [dst]=plane0
   else
    [dst]=plane1
   buf_index=buf_index+1, dst=dst+1, len=len-1
```

**decompress_mode2(src,dst,len)**

```text
  initialize
  while len>0
   if (buf_index AND 11h)=0
    for pixel=0 to 7
      a = (out SHR 0)  AND 0Fh
      b = (out SHR 28) AND 0Fh
      decoded=0
      con=0
      decompression_core
      con=decoded+1
      decompression_core
      if con=2 then con=decoded+11 else con = get_con(a,b,c)+3+decoded*5
      decompression_core
      con=Mode2ContextTable[con]+(decoded AND 1)
      decompression_core
      do_pixel_order(a,b,c,4,decoded)
    plane0.bits(7..0) = out.bits(31,27,23,19,15,11,7,3)
    plane1.bits(7..0) = out.bits(30,26,22,18,14,10,6,2)
    plane2.bits(7..0) = out.bits(29,25,21,17,13, 9,5,1)
    plane3.bits(7..0) = out.bits(28,24,20,16,12, 8,4,0)
    bitplanebuffer[buf_index+0] = plane2
    bitplanebuffer[buf_index+1] = plane3
    [dst]=plane0
   else if (buf_index AND 10h)=0
    [dst]=plane1
   else
    [dst]=bitplanebuffer[buf_index AND 0Fh]
   buf_index=buf_index+1, dst=dst+1, len=len-1
```

**initialize**

```text
  src=directory_base+(directory_index*4)
  mode=[src+0]
  src=[src+3]+[src+2]*100h+[src+1]*10000h  ;big-endian (!)
  buf_index=0
  out=00000000h
  c=0
  top=255
  val.msb=[src], val.lsb=00h, src=src+1, in_count=0
  for i=0 to 15 do pixelorder[i]=i
  for i=0 to 31 do ContextIndex[i]=0, ContextInvert[i]=0
```

**decompression_core**

```text
  decoded=(decoded SHL 1) xor ContextInvert[con]
  evl=ContextIndex[con]
  top = top - EvolutionProb[evl]
  if val.msb > top
    val.msb = val.msb-(top-1)
    top = EvolutionProb[evl]-1
    if top>79 then ContextInvert[con] = ContextInvert[con] XOR 1
    decoded = decoded xor 1
    ContextIndex[con] = EvolutionNextLps[evl]
  else
    if top<=126 then ContextIndex[con] = EvolutionNextMps[evl]
  while(top<=126)
    if in_count=0 then val.lsb=[src], src=src+1, in_count=8
    top = (top SHL 1)+1
    val = (val SHL 1), in_count=in_count-1    ;16bit val.msb/lsb
```

**do_pixel_order(a,b,c,shift,decoded)**

```text
  m=0, x=a, repeat, exchange(x,pixelorder[m]), m=m+1, until x=a
  for m=0 to (1 shl shift)-1 do realorder[m]=pixelorder[m]
  m=0, x=c, repeat, exchange(x,realorder[m]), m=m+1, until x=c
  m=0, x=b, repeat, exchange(x,realorder[m]), m=m+1, until x=b
  m=0, x=a, repeat, exchange(x,realorder[m]), m=m+1, until x=a
  out = (out SHL shift) + realorder[decoded]
  c = b
```

**get_con(a,b,c)**

```text
  if (a=b AND b=c) then return=0
  else if (a=b) then return=1
  else if (b=c) then return=2
  else if (a=c) then return=3
  else return=4
```

**EvolutionProb[0..52]**

```text
  90,37,17, 8, 3, 1,90,63,44,32,23,17,12, 9, 7, 5, 4, 3, 2
  90,72,58,46,38,31,25,21,17,14,11, 9, 8, 7, 5, 4, 4, 3, 2
  2 ,88,77,67,59,52,46,41,37,86,79,71,65,60,55
```

**EvolutionNextLps[0..52]**

```text
  1 , 6, 8,10,12,15, 7,19,21,22,23,25,26,28,29,31,32,34,35
  20,39,40,42,44,45,46,25,26,26,27,28,29,30,31,33,33,34,35
  36,39,47,48,49,50,51,44,45,47,47,48,49,50,51
```

**EvolutionNextMps[0..52]**

```text
  1 , 2, 3, 4, 5, 5, 7, 8, 9,10,11,12,13,14,15,16,17,18, 5
  20,21,22,23,24,25,26,27,28,29,30,31,32,33,34,35,36,37,38
  5 ,40,41,42,43,44,45,46,24,48,49,50,51,52,43
```

**Mode2ContextTable[0..14]  ;only entries 3..14 used (entries 0..2 = dummies)**

```text
  0 ,0 ,0 ,15,17,19,21,23,25,25,25,25,25,27,29
```

<a id="snescartspc7110notes"></a>

## SNES Cart SPC7110 Notes

**Compression/Decompression Example**

Uncompressed Data (64-byte ASCII string):

```text
  Test123.ABCDABCDAAAAAAAAaaaabbbbccccdddd7654321076543210.Test123
```

Compressed in Mode0:

```text
  68 91 36 15 F8 BF 42 35 2F 67 3D B7 AA 05 B4 F7 70 7A 26 20 EA 58 2C 09 61 00
  C5 00 8C 6F FF D1 42 9D EE 7F 72 87 DF D6 5F 92 65 00 00
```

Compressed in Mode1:

```text
  4B F6 80 1E 3A 4C 42 6C DA 16 0F C6 44 ED 64 10 77 AF 50 00 05 C0 01 27 22 B0
  83 51 05 32 4A 1E 74 93 08 76 07 E5 32 12 B4 99 9E 55 A3 F8 00
```

Compressed in Mode2:

```text
  13 B3 27 A6 F4 5C D8 ED 6C 6D F8 76 80 A7 87 20 39 4B 37 1A CC 3F E4 3D BE 65
  2D 89 7E 0B 0A D3 46 D5 0C 1F D3 81 F3 AD DD E8 5C C0 BD 62 AA CB F8 B5 38 00
```

**Selftest Program**

All three SPC7110 games include a selftest function (which executes on initial
power-up, ie. when the battery-backed SRAM is still uninitialized). Press
Button A/B to start 1st/2nd test, and push Reset Button after each test.

**PCBs**

```text
  SHVC-BDH3B-01 (without RTC)
  SHVC-LDH3C-01 (with RTC)
```

<a id="snescartunlicensedvariants"></a>

## SNES Cart Unlicensed Variants

**Gamars Puzzle (Kaiser)**

A LoROM game with SRAM at 316000h (unlike normal LoROM games that have SRAM at
70xxxxh). Cartridge Header Maker entry is [FFDAh]=00h, and SRAM size entry is
[FFD8h]=20h (4096 gigabytes), the actual size of the SRAM is unknown.

**Bootlegs**

The "bootleg" games are semi-illegal pirate productions, typically consisting
of a custom (and not-so-professional) game engine, bundled with graphics and
sounds ripped from commercial games. Some of these cartridges are containing
some small copy-protection hardware (see below).

**Copy-Protected Bootlegs (Standard "bitswap" variant)**

This type is used by several games:

```text
  A Bug's Life                          2MB, CRC32=014F0FCFh
  Aladdin 2000                          2MB, CRC32=752A25D3h
  Bananas de Pijamas                    1MB, CRC32=52B0D84Bh
  Digimon Adventure                     2MB, CRC32=4F660972h
  King of Fighters 2000 (aka KOF2000)   3MB, CRC32=A7813943h
  Pocket Monster (aka Picachu)          2MB, CRC32=892C6765h
  Pokemon Gold Silver                   2MB, CRC32=7C0B798Dh
  Pokemon Stadium                       2MB, CRC32=F863C642h
  Soul Edge Vs Samurai                  2MB, CRC32=5E4ADA04h
  Street Fighter EX Plus Alpha          2MB, CRC32=DAD59B9Fh
  X-Men vs. Street Fighter              2MB, CRC32=40242231h
```

The protection hardware is mapped to:

```text
  80-xx:8000-FFFF  Read 8bit Latch (bits re-ordered as: 0,6,7,1,2,3,4,5)
  88-xx:8000-FFFF  Write 8bit Latch (bits ordered as:   7,6,5,4,3,2,1,0)
```

**Copy-Protected Bootlegs (Soulblade "constant" variant)**

This type is used by only one game:

```text
  Soul Blade                            3MB, CRC32=C97D1D7Bh
```

The protection hardware consists of a read-only pattern, mapped to:

```text
  80-BF:8000-FFFF  Filled with a constant 4-byte pattern (55h,0Fh,AAh,F0h)
  C0-FF:0000-FFFF  Open bus (not used)
```

**Copy-Protected Bootlegs (Tekken2 "alu/flipflop" variant)**

This type is used by only one game:

```text
  Tekken 2                              2MB, CRC32=066687CAh
```

The protection hardware is mapped to:

```text
  [80-BF:80xx]=0Fh,00h Clear all 6 bits
  [80-BF:81xx]=xxh     Probably "No Change" (unused, except for Reading)
  [80-BF:82xx]=FFh,00h Set Data bit0
  [80-BF:83xx]=FFh,00h Set Data bit1
  [80-BF:84xx]=FFh,00h Set Data bit2
  [80-BF:85xx]=FFh,00h Set Data bit3
  [80-BF:86xx]=FFh,00h Set ALU Direction bit (0=Up/Left, 1=Down/Right)
  [80-BF:87xx]=FFh,00h Set ALU Function bit  (0=Count, 1=Shift)
  X=[80-BF:81xx]       Return "4bitData plus/minus/shl/shr 1"
  ;the above specs are based on 12 known/guessed results (as guessed by d4s),
  ;the remaining 52 combinations are probably following same rules (not tested
  ;on real hardware). theoretically some ports might do things like "set bitX
  ;and clear bitY", in that case, there would be more than 64 combinations.
```

The hardware is often missing I/O accesses, unless one is repeating them some
dozens of times; the existing game is issuing 240 words (480 bytes) to
write-ports, and reads 256 words (512 bytes) from the read-port. The reads
contain the result in lower 4bit (probably in both low-byte and high-byte of
the words) (and unknown/unused stuff in the other bits).

The set/clear ports are said to react on both reads and writes (which would
imply that the written data is don't care).

<a id="snescartsrtcrealtimeclock1game"></a>

## SNES Cart S-RTC (Realtime Clock) (1 game)

PCB "SHVC-LJ3R-01" with 24pin "Sharp S-RTC" chip. Used only by one japanese
game:

```text
  Dai Kaiju Monogatari 2 (1996) Birthday/Hudson Soft (JP)
```

**S-RTC I/O Ports**

```text
  002800h S-RTC Read  (R)
  002801h S-RTC Write (W)
```

Both registers are 4bits wide. When writing: Upper 4bit should be zero. When
reading: Upper 4bit should be masked-off (they do possibly contain garbage, eg.
open-bus).

**S-RTC Communication**

The sequence for setting, and then reading the time is:

```text
  Send <0Eh,04h,0Dh,0Eh,00h,Timestamp(12 digits),0Dh> to [002801h]
  If ([002800h] AND 0F)=0Fh then read <Timestamp(13 digits)>
  If ([002800h] AND 0F)=0Fh then read <Timestamp(13 digits)>
  If ([002800h] AND 0F)=0Fh then read <Timestamp(13 digits)>
  If ([002800h] AND 0F)=0Fh then read <Timestamp(13 digits)>
  etc.
```

The exact meaning of the bytes is unknown. 0Eh/0Dh seems to invoke/terminate
commands, 04h might be some configuration stuff (like setting 24-hour mode).
00h is apparently the set-time command. There might be further commands (such
like setting interrupts, alarm, 12-hour mode, reading battery low &amp; error
flags, etc.). When reading, 0Fh seems to indicate sth like "time available".

The 12/13-digit "SSMMHHDDMYYY(D)" Timestamps are having the following format:

```text
  Seconds.lo  (BCD, 0..9)
  Seconds.hi  (BCD, 0..5)
  Minutes.lo  (BCD, 0..9)
  Minutes.hi  (BCD, 0..5)
  Hours.lo    (BCD, 0..9)
  Hours.hi    (BCD, 0..2)
  Day.lo      (BCD, 0..9)
  Day.hi      (BCD, 0..3)
  Month       (HEX, 01h..0Ch)
  Year.lo     (BCD, 0..9)
  Year.hi     (BCD, 0..9)
  Century     (HEX, 09h..0Ah for 19xx..20xx)
```

When READING the time, there is one final extra digit (the existing software
doesn't transmit that extra digit on WRITING, though maybe it's possible to do
writing, too):

```text
  Day of Week? (0..6) (unknown if RTC assigns sth like 0=Sunday or 0=Monday)
```

**Pinouts**

[SNES Pinouts RTC Chips](80-timings-unpredictable-pinouts.md#snes-pinouts-rtc-chips)

**Note**

There's another game that uses a different RTC chip: A 4bit serial bus RTC-4513
(as made by Epson) connected to a SPC7110 chip.
