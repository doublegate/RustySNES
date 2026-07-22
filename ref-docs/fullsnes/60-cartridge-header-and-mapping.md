# Fullsnes — Cartridge Header, PCBs, CIC & Memory Mapping

[Index](00-index.md) · [« Controllers & Input Peripherals](50-controllers.md) · [Cartridge Coprocessors »](61-coprocessors.md)

**Sections in this file:**

- [SNES Cartridges](#snes-cartridges)
- [SNES Cartridge ROM Header](#snes-cartridge-rom-header)
- [SNES Cartridge PCBs](#snes-cartridge-pcbs)
- [SNES Cartridge ROM-Image Headers and File Extensions](#snes-cartridge-rom-image-headers-and-file-extensions)
- [SNES Cartridge ROM-Image Interleave](#snes-cartridge-rom-image-interleave)
- [SNES Cartridge CIC Lockout Chip](#snes-cartridge-cic-lockout-chip)
- [SNES Cartridge CIC Pseudo Code](#snes-cartridge-cic-pseudo-code)
- [SNES Cartridge CIC Instruction Set](#snes-cartridge-cic-instruction-set)
- [SNES Cartridge CIC Notes](#snes-cartridge-cic-notes)
- [SNES Cartridge CIC Versions](#snes-cartridge-cic-versions)
- [SNES Cart LoROM Mapping (ROM divided into 32K banks) (around 1500 games)](#snes-cart-lorom-mapping-rom-divided-into-32k-banks-around-1500-games)
- [SNES Cart HiROM Mapping (ROM divided into 64K banks) (around 500 games)](#snes-cart-hirom-mapping-rom-divided-into-64k-banks-around-500-games)

---

<a id="snescartridges"></a>

## SNES Cartridges

**General Cartridge Info**

[SNES Cartridge ROM Header](#snes-cartridge-rom-header)

[SNES Cartridge PCBs](#snes-cartridge-pcbs)

[SNES Cartridge ROM-Image Headers and File Extensions](#snes-cartridge-rom-image-headers-and-file-extensions)

[SNES Cartridge ROM-Image Interleave](#snes-cartridge-rom-image-interleave)

[SNES Cartridge CIC Lockout Chip](#snes-cartridge-cic-lockout-chip)

[SNES Cartridge Slot Pinouts](80-timings-unpredictable-pinouts.md#snes-cartridge-slot-pinouts)

**Basic Mapping Schemes**

[SNES Cart LoROM Mapping (ROM divided into 32K banks) (around 1500 games)](#snes-cart-lorom-mapping-rom-divided-into-32k-banks-around-1500-games)

[SNES Cart HiROM Mapping (ROM divided into 64K banks) (around 500 games)](#snes-cart-hirom-mapping-rom-divided-into-64k-banks-around-500-games)

**Cartridges with External Programmable CPUs**

[SNES Cart SA-1 (programmable 65C816 CPU) (aka Super Accelerator) (35 games)](61-coprocessors.md#snes-cart-sa-1-programmable-65c816-cpu-aka-super-accelerator-35-games)

[SNES Cart GSU-n (programmable RISC CPU) (aka Super FX/Mario Chip) (10 games)](61-coprocessors.md#snes-cart-gsu-n-programmable-risc-cpu-aka-super-fxmario-chip-10-games)

[SNES Cart Capcom CX4 (programmable RISC CPU) (Mega Man X 2-3) (2 games)](61-coprocessors.md#snes-cart-capcom-cx4-programmable-risc-cpu-mega-man-x-2-3-2-games)

**Cartridges with External Pre-Programmed CPUs**

[SNES Cart DSP-n/ST010/ST011 (pre-programmed NEC uPD77C25 CPU) (23 games)](61-coprocessors.md#snes-cart-dsp-nst010st011-pre-programmed-nec-upd77c25-cpu-23-games)

[SNES Cart Seta ST018 (pre-programmed ARM CPU) (1 game)](61-coprocessors.md#snes-cart-seta-st018-pre-programmed-arm-cpu-1-game)

**Cartridges with other custom chips**

[SNES Cart OBC1 (OBJ Controller) (1 game)](61-coprocessors.md#snes-cart-obc1-obj-controller-1-game)

[SNES Cart S-DD1 (Data Decompressor) (2 games)](61-coprocessors.md#snes-cart-s-dd1-data-decompressor-2-games)

[SNES Cart SPC7110 (Data Decompressor) (3 games)](61-coprocessors.md#snes-cart-spc7110-data-decompressor-3-games)

[SNES Cart Unlicensed Variants](61-coprocessors.md#snes-cart-unlicensed-variants)

**Cartridges with Real-Time Clocks**

[SNES Cart S-RTC (Realtime Clock) (1 game)](61-coprocessors.md#snes-cart-s-rtc-realtime-clock-1-game)

[SNES Cart SPC7110 with RTC-4513 Real Time Clock (1 game)](61-coprocessors.md#snes-cart-spc7110-with-rtc-4513-real-time-clock-1-game)

Moreover, the Satellaview has a time function (allowing to receive the time via
satellite dish). And, there are S-3520 (Seiko RTC's) in the Super Famicom Box
and in the Nintendo Super System.

**Special Non-game Cartridges/Expansions**

[SNES Cart Super Gameboy](62-cartridge-addons-satellaview-modems.md#snes-cart-super-gameboy)

[SNES Cart Satellaview (satellite receiver &amp; mini flashcard)](62-cartridge-addons-satellaview-modems.md#snes-cart-satellaview-satellite-receiver-mini-flashcard)

[SNES Cart Data Pack Slots (satellaview-like mini-cartridge slot)](62-cartridge-addons-satellaview-modems.md#snes-cart-data-pack-slots-satellaview-like-mini-cartridge-slot)

[SNES Cart Nintendo Power (flashcard)](62-cartridge-addons-satellaview-modems.md#snes-cart-nintendo-power-flashcard)

[SNES Cart Sufami Turbo (Mini Cartridge Adaptor)](62-cartridge-addons-satellaview-modems.md#snes-cart-sufami-turbo-mini-cartridge-adaptor)

[SNES Cart X-Band (2400 baud Modem)](62-cartridge-addons-satellaview-modems.md#snes-cart-x-band-2400-baud-modem)

[SNES Cart FLASH Backup](63-copiers-cheat-devices-cdrom.md#snes-cart-flash-backup)

[SNES Cart Cheat Devices](63-copiers-cheat-devices-cdrom.md#snes-cart-cheat-devices)

[SNES Cart Tri-Star (aka Super 8) (allows to play NES games on the SNES)](63-copiers-cheat-devices-cdrom.md#snes-cart-tri-star-aka-super-8-allows-to-play-nes-games-on-the-snes)

[SNES Cart Pirate X-in-1 Multicarts (1)](63-copiers-cheat-devices-cdrom.md#snes-cart-pirate-x-in-1-multicarts-1)

[SNES Cart Pirate X-in-1 Multicarts (2)](63-copiers-cheat-devices-cdrom.md#snes-cart-pirate-x-in-1-multicarts-2)

[SNES Cart Copiers](63-copiers-cheat-devices-cdrom.md#snes-cart-copiers)

[SNES Cart CDROM Drive](63-copiers-cheat-devices-cdrom.md#snes-cart-cdrom-drive)

<a id="snescartridgeromheader"></a>

## SNES Cartridge ROM Header

The Cartridge header is mapped to 00FFxxh in SNES memory (near the exception
vectors). In ROM-images it is found at offset 007Fxxh (LoROM), 00FFxxh (HiROM),
or 40FFxxh (ExHiROM); add +200h to that offsets if "(imagesize AND 3FFh)=200h",
ie. if there's an extra header from SWC/UFO/etc. copiers.

**Cartridge Header (Area FFC0h..FFCFh)**

```text
  FFC0h  Cartridge title (21 bytes, uppercase ascii, padded with spaces)
  FFC0h  First byte of title (or 5Ch far-jump-opcode in Pirate X-in-1 Carts)
  FFD4h  Last byte of title (or 00h indicating Early Extended Header)
  FFD5h  Rom Makeup / ROM Speed and Map Mode (see below)
  FFD6h  Chipset (ROM/RAM information on cart) (see below)
  FFD7h  ROM size (1 SHL n) Kbytes (usually 8=256KByte .. 0Ch=4MByte)
          Values are rounded-up for carts with 10,12,20,24 Mbits
  FFD8h  RAM size (1 SHL n) Kbytes (usually 1=2Kbyte .. 5=32Kbyte) (0=None)
  FFD9h  Country (also implies PAL/NTSC) (see below)
  FFDAh  Developer ID code  (00h=None/Homebrew, 01h=Nintendo, etc.) (33h=New)
  FFDBh  ROM Version number (00h=First)
  FFDCh  Checksum complement (same as below, XORed with FFFFh)
  FFDEh  Checksum (all bytes in ROM added together; assume [FFDC-F]=FF,FF,0,0)
```

**Extended Header (Area FFB0h..FFBFh) (newer carts only)**

Early Extended Header (1993) (when [FFD4h]=00h; Last byte of Title=00h):

```text
  FFB0h  Reserved   (15 zero bytes)
```

Later Extended Header (1994) (when [FFDAh]=33h; Old Maker Code=33h):

```text
  FFB0h  Maker Code (2-letter ASCII, eg. "01"=Nintendo)
  FFB2h  Game Code  (4-letter ASCII) (or old 2-letter padded with 20h,20h)
  FFB6h  Reserved   (6 zero bytes)
  FFBCh  Expansion FLASH Size (1 SHL n) Kbytes (used in JRA PAT)
  FFBDh  Expansion RAM Size (1 SHL n) Kbytes (in GSUn games) (without battery?)
  FFBEh  Special Version      (usually zero) (eg. promotional version)
```

Both Early and Later Extended Headers:

```text
  FFBFh  Chipset Sub-type (usually zero) (used when [FFD6h]=Fxh)
```

Note: The early-extension is used only with ST010/11 games.

If the first letter of the 4-letter Game Code is "Z", then the cartridge does
have Satellaview-like Data Pack/FLASH cartridge slot (this applies ONLY to
"Zxxx" 4-letter codes, not to old "Zx  " space padded 2-letter codes).

**Cartridge Header Variants**

The BS-X Satellaview FLASH Card Files, and Sufami Turbo Mini-Cartridges are
using similar looking (but not fully identical) headers (and usually the same
.SMC file extension) than normal ROM cartridges. Detecting the content of .SMC
files can be done by examining ID Strings (Sufami Turbo), or differently
calculated checksum values (Satellaview). For details, see:

[SNES Cart Satellaview (satellite receiver &amp; mini flashcard)](62-cartridge-addons-satellaview-modems.md#snes-cart-satellaview-satellite-receiver-mini-flashcard)

[SNES Cart Sufami Turbo (Mini Cartridge Adaptor)](62-cartridge-addons-satellaview-modems.md#snes-cart-sufami-turbo-mini-cartridge-adaptor)

Homebrew games (and copiers &amp; cheat devices) are usually having several
errors in the cartridge header (usually no checksum, zero-padded title, etc),
they should (hopefully) contain valid entryoints in range 8000h..FFFEh. Many
Copiers are using 8Kbyte ROM bank(s) - in that special case the exception
vectors are located at offset 1Fxxh within the ROM-image.

**CPU Exception Vectors (Area FFE0h..FFFFh)**

```text
  FFE0h  Zerofilled (or ID "XBOO" for WRAM-Boot compatible files)
  FFE4h  COP vector     (65C816 mode) (COP opcode)
  FFE6h  BRK vector     (65C816 mode) (BRK opcode)
  FFE8h  ABORT vector   (65C816 mode) (not used in SNES)
  FFEAh  NMI vector     (65C816 mode) (SNES V-Blank Interrupt)
  FFECh  ...
  FFEEh  IRQ vector     (65C816 mode) (SNES H/V-Timer or External Interrupt)
  FFF0h  ...
  FFF4h  COP vector     (6502 mode)
  FFF6h  ...
  FFF8h  ABORT vector   (6502 mode) (not used in SNES)
  FFFAh  NMI vector     (6502 mode)
  FFFCh  RESET vector   (6502 mode) (CPU is always in 6502 mode on RESET)
  FFFEh  IRQ/BRK vector (6502 mode)
```

Note: Exception Vectors are variable in SA-1 and CX4, and fixed in GSU.

**Text Fields**

The ASCII fields can use chr(20h..7Eh), actually they are JIS (with Yen instead
backslash).

**ROM Size / Checksum Notes**

The ROM size is specified as "(1 SHL n) Kbytes", however, some cartridges
contain "odd" sizes:

```text
  * Game uses 2-3 ROM chips (eg. one 8MBit plus one 2MBit chip)
  * Game originally designed for 2 ROMs, but later manufactured as 1 ROM (?)
  * Game uses a single 24MBit chip (23C2401)
```

In all three cases the ROM Size entry in [FFD7h] is rounded-up. In memory, the
"bigger half" is mapped to address 0, followed by the "smaller half", then
followed by mirror(s) of the smaller half. Eg. a 10MBit game would be rounded
to 16MBit, and mapped (and checksummed) as "8Mbit + 4x2Mbit". In practice:

```text
  Title                       Hardware          Size   Checksum
  Dai Kaiju Monogatari 2 (J)  ExHiROM+S-RTC     5MB    4MB + 4 x Last 1MB
  Tales of Phantasia (J)      ExHiROM           6MB    <???>
  Star Ocean (J)              LoROM+S-DD1       6MB    4MB + 2 x Last 2MB
  Far East of Eden Zero (J)   HiROM+SPC7110+RTC 5MB    5MB
  Momotaro Dentetsu Happy (J) HiROM+SPC7110     3MB    2 x 3MB
  Sufami Turbo BIOS           LoROM in Minicart xx     without checksum
  Sufami Turbo Games          LoROM in Minicart xx     without checksum
  Dragon Ball Z - Hyper Dimension    LoROM+SA-1 3MB       Overdump 4MB
  SD Gundam GNext (J)                LoROM+SA-1 1.5MB     Overdump 2MB
  Megaman X2                         LoROM+CX4  1.5MB     Overdump 2MB
  BS Super Mahjong Taikai (J)        BS           Overdump/Mirr+Empty
  Demon's Crest... reportedly 12MBit ? but, that's bullshit ?
```

```text
  SPC7110 Title               ROM Size (Header value)    Checksum
  Super Power League 4        2MB      (rounded to 2MB)  1x(All 2MB)
  Momotaro Dentetsu Happy (J) 3MB      (rounded to 4MB)  2x(All 3MB)
  Far East of Eden Zero (J)   5MB      (rounded to 8MB)  1x(All 5MB)
```

On-chip ROM contained in external CPUs (DSPn,ST01n,CX4) is NOT counted in the
ROM size entry, and not included in the checksum.

Homebrew files often contain 0000h,0000h or FFFFh,0000h as checksum value.

**ROM Speed and Map Mode (FFD5h)**

```text
  Bit7-6 Always 0
  Bit5   Always 1 (maybe meant to be MSB of bit4, for "2" and "3" MHz)
  Bit4   Speed (0=Slow, 1=Fast)              (Slow 200ns, Fast 120ns)
  Bit3-0 Map Mode
```

Map Mode can be:

```text
  0=LoROM/32K Banks             Mode 20 (LoROM)
  1=HiROM/64K Banks             Mode 21 (HiROM)
  2=LoROM/32K Banks + S-DD1     Mode 22 (mappable) "Super MMC"
  3=LoROM/32K Banks + SA-1      Mode 23 (mappable) "Emulates Super MMC"
  5=HiROM/64K Banks             Mode 25 (ExHiROM)
  A=HiROM/64K Banks + SPC7110   Mode 25? (mappable)
```

Note: ExHiROM is used only by "Dai Kaiju Monogatari 2 (JP)" and "Tales of
Phantasia (JP)".

**Chipset (ROM/RAM information on cart) (FFD6h) (and some subclassed via FFBFh)**

```text
  00h     ROM
  01h     ROM+RAM
  02h     ROM+RAM+Battery
  x3h     ROM+Co-processor
  x4h     ROM+Co-processor+RAM
  x5h     ROM+Co-processor+RAM+Battery
  x6h     ROM+Co-processor+Battery
  x9h     ROM+Co-processor+RAM+Battery+RTC-4513
  xAh     ROM+Co-processor+RAM+Battery+overclocked GSU1 ? (Stunt Race)
  x2h     Same as x5h, used in "F1 Grand Prix Sample (J)" (?)
  0xh     Co-processor is DSP    (DSP1,DSP1A,DSP1B,DSP2,DSP3,DSP4)
  1xh     Co-processor is GSU    (MarioChip1,GSU1,GSU2,GSU2-SP1)
  2xh     Co-processor is OBC1
  3xh     Co-processor is SA-1
  4xh     Co-processor is S-DD1
  5xh     Co-processor is S-RTC
  Exh     Co-processor is Other  (Super Gameboy/Satellaview)
  Fxh.xxh Co-processor is Custom (subclassed via [FFBFh]=xxh)
  Fxh.00h Co-processor is Custom (SPC7110)
  Fxh.01h Co-processor is Custom (ST010/ST011)
  Fxh.02h Co-processor is Custom (ST018)
  Fxh.10h Co-processor is Custom (CX4)
```

In practice, following values are used:

```text
  00h     ROM             ;if gamecode="042J" --> ROM+SGB2
  01h     ROM+RAM (if any such produced?)
  02h     ROM+RAM+Battery ;if gamecode="XBND" --> ROM+RAM+Batt+XBandModem
                          ;if gamecode="MENU" --> ROM+RAM+Batt+Nintendo Power
  03h     ROM+DSP
  04h     ROM+DSP+RAM (no such produced)
  05h     ROM+DSP+RAM+Battery
  13h     ROM+MarioChip1/ExpansionRAM (and "hacked version of OBC1")
  14h     ROM+GSU+RAM                    ;\ROM size up to 1MByte -> GSU1
  15h     ROM+GSU+RAM+Battery            ;/ROM size above 1MByte -> GSU2
  1Ah     ROM+GSU1+RAM+Battery+Fast Mode? (Stunt Race)
  25h     ROM+OBC1+RAM+Battery
  32h     ROM+SA1+RAM+Battery (?) "F1 Grand Prix Sample (J)"
  34h     ROM+SA1+RAM (?) "Dragon Ball Z - Hyper Dimension"
  35h     ROM+SA1+RAM+Battery
  43h     ROM+S-DD1
  45h     ROM+S-DD1+RAM+Battery
  55h     ROM+S-RTC+RAM+Battery
  E3h     ROM+Super Gameboy      (SGB)
  E5h     ROM+Satellaview BIOS   (BS-X)
  F5h.00h ROM+Custom+RAM+Battery     (SPC7110)
  F9h.00h ROM+Custom+RAM+Battery+RTC (SPC7110+RTC)
  F6h.01h ROM+Custom+Battery         (ST010/ST011)
  F5h.02h ROM+Custom+RAM+Battery     (ST018)
  F3h.10h ROM+Custom                 (CX4)
```

**Country (also implies PAL/NTSC) (FFD9h)**

```text
  00h -  International (eg. SGB)  (any)
  00h J  Japan                    (NTSC)
  01h E  USA and Canada           (NTSC)
  02h P  Europe, Oceania, Asia    (PAL)
  03h W  Sweden/Scandinavia       (PAL)
  04h -  Finland                  (PAL)
  05h -  Denmark                  (PAL)
  06h F  France                   (SECAM, PAL-like 50Hz)
  07h H  Holland                  (PAL)
  08h S  Spain                    (PAL)
  09h D  Germany, Austria, Switz  (PAL)
  0Ah I  Italy                    (PAL)
  0Bh C  China, Hong Kong         (PAL)
  0Ch -  Indonesia                (PAL)
  0Dh K  South Korea              (NTSC) (North Korea would be PAL)
  0Eh A  Common (?)               (?)
  0Fh N  Canada                   (NTSC)
  10h B  Brazil                   (PAL-M, NTSC-like 60Hz)
  11h U  Australia                (PAL)
  12h X  Other variation          (?)
  13h Y  Other variation          (?)
  14h Z  Other variation          (?)
```

Above shows the [FFD9h] value, and the last letter of 4-character game codes.

**Game Codes (FFB2h, exists only when [FFDAh]=33h)**

```text
  "xxxx"  Normal 4-letter code (usually "Axxx") (or "Bxxx" for newer codes)
  "xx  "  Old 2-letter code (space padded)
  "042J"  Super Gameboy 2
  "MENU"  Nintendo Power FLASH Cartridge Menu
  "Txxx"  NTT JRA-PAT and SPAT4 (SFC Modem BIOSes)
  "XBND"  X-Band Modem BIOS
  "Zxxx"  Special Cartridge with satellaview-like Data Pack Slot
```

The last letter indicates the region (see Country/FFD9h description) (except in
2-letter codes and "MENU"/"XBND" codes).

<a id="snescartridgepcbs"></a>

## SNES Cartridge PCBs

**Cartridge PCB Naming (eg. SHVC-XXXX-NN)**

Prefix

```text
  SHVC  Normal cartridge (japan, usa, europe)
  SNSP  Special PAL version (for SA1 and S-DD1 with built-in CIC)
  BSC   BIOS (or game cartridge) with external Satellaview FLASH cartridge slot
  MAXI  Majesco Sales Inc cartridge (Assembled in Mexico)
  MJSC  Majesco Sales Inc cartridge (Assembled in Mexico)
  WEI   Whatever? (Assembled in Mexico)
  EA    Electronics Arts cartridge
```

First Character

```text
  1   One ROM chip (usually 36pin, sometimes 32pin)
  Y   Two 4Mbit ROM chips     (controlled by 74LS00)
  2   Two 8Mbit ROM chips     (controlled by 74LS00,MAD-1,etc.)
  B   Two 16Mbit ROM chips    (controlled by 74LS00 or MAD-1)
  L   Two 32Mbit ROM chips    (controlled by SPC7110F,S-DD1 or MAD-1)
  3   Three 8Mbit ROM chips   (controlled by 74LS139) (decoder/demultiplexer)
  4   Four ROM chips (used only for 4PVnn/4QW EPROM prototype boards)
  8   Eight ROM chips (used only for 8PVnn/8Xnn EPROM prototype boards)
```

Second Character(s)

```text
  A   LoRom (A15 / Pin40 not connected to ROM)   (uh, 1A3B-20 ?)
  B   LoRom plus DSP-N chip
  C   LoRom plus Mario Chip 1               62pin  (and 36pin ROM) (no X1)
  CA  LoRom plus GSU-1                      62pin  (and 32pin ROM)
  CB  LoRom plus GSU-2 or GSU-2-SP1         62pin  (and 40pin ROM)
  DC  LoRom plus CX4                        62pin  (and 32pin ROMs)
  DH  HiRom plus SPC7110F                   62pin  (and 32pin+44pin ROMs)
  DE  LoRom plus ST018                      62pin  (and 32..40pin ROM possible)
  DS  LoRom plus ST010/ST011                62pin  (and 32..36pin ROM possible)
  E   LoRom plus OBC1 chip                  62pin  (and 32pin ROMs)
  J   HiRom (A15 / Pin40 is connected to ROM)
  K   HiRom plus DSP-N chip
  L   LoRom plus SA1 chip       62pin  (and 44pin ROM) (16bit data?)
  N   LoRom plus S-DD1 chip     62pin  (and 44pin ROM)
  P   LoRom with 2 prototype EPROMs (=unlike ROM A16..Ahi,/CS)
  PV  WhateverRom with 4 prototype EPROMs (=unlike ROM A16..Ahi,/CS)
  Q   WhateverRom prototype (see book2.pdf)
  QW  WhateverRom prototype (see book2.pdf)
  RA  LoRom plus GSU1A with prototype EPROMs (=unlike ROM A16..Ahi,/CS)  62pin
  X   WhateverRom prototype (see book2.pdf)
```

Third Character

```text
  0   No SRAM
  1   2Kx8 SRAM (usually narrow 24pin DIP, sometimes wide 24pin DIP)
  2   prototype variable size SRAM
  3   8Kx8 SRAM (usually wide 28pin DIP)
  5   32Kx8 SRAM (usually wide 28pin DIP)
  6   64Kx8 SRAM (32pin SMD, found on boards with GSU)
  8   64Kx8 SRAM (in one SA1 cart) (seems to be a 64Kx8 chip, not 256Kx8)
```

Forth Character

```text
  N   No battery
  B   Battery (with Transistor+Diodes or MM1026/MM1134 chip)
  M   Battery (with MAD-1 chip; or with rare MAD-R chip)
  X   Battery (with MAD-2 chip; maybe amplifies X1 oscillator for DSP1B chips)
  C   Battery and RTC-4513
  R   Battery and S-RTC
  F   FLASH Memory (instead of SRAM) (used by JRA-PAT and SPAT4)
```

Fifth Characters (only if cart contains RAM that is NOT battery-backed)

```text
  5S  32Kx8 SRAM (for use by GSU, not battery backed)
  6S  64Kx8 SRAM (for use by GSU, not battery backed)
  7S  64Kx8 or 128Kx8 SRAM (for use by GSU, not battery backed)
  9P  512Kx8 PSRAM (32pin 658512LFP-85) (for satellaview)
  (the black-blob Star Fox PCB also contains RAM, but lacks the ending "nS")
```

Suffix

```text
  -NN revision number (unknown if this indicates any relevant changes)
```

**Other Cartridge PCBs (that don't follow the above naming system)**

```text
  CPU2 SGB-R-10  Super Gameboy (1994)
  SHVC-MMS-X1    Nintendo Power FLASH Cartridge (1997) (older version)
  SHVC-MMS-02    Nintendo Power FLASH Cartridge (1997) (newer version)
  SHVC-MMSA-1    Nintendo Power FLASH Cartridge (19xx) ???
  SHVC-SGB2-01   Super Gameboy 2 (1998)
  SHVC-1C0N      Star Fox (black blob version) (PCB name lacks ending nS-NN)
  SHVC TURBO     Sufami Turbo BASE CASSETTE (Bandai)
  <unknown?>     Sufami Turbo game cartridges
  123-0002-16    X-Band Modem (1995 by Catapult / licensed by Nintendo)
  BSMC-AF-01     Satellaview Mini FLASH Cartridge (plugged into BIOS cartridge)
  BSMC-CR-01     Satellaview Mini FLASH Cartridge (???) (not rewriteable ?)
  GPC-RAMC-4M    SRAM Cartridge (without ROM)?
  GPC-RAMC-S1    SRAM Cartridge (without ROM)?
  GS 0871-102    Super Famicom Box multi-game cartridge
  NSS-01-ROM-A   Nintendo Super System (NSS) cartridge
  NSS-01-ROM-B   Nintendo Super System (NSS) cartridge
  NSS-01-ROM-C   Nintendo Super System (NSS) cartridge
  NSS-X1-ROM-C   Rebadged NSS-01-ROM-C board (plus battery/sram installed)
  RB-01, K-PE1-945-01   SNES CD Super Disc BIOS Cartridge (prototype)
```

**ROM Chips used in SNES cartridges**

```text
  2Mbit 256Kbyte        LH532 TC532 N-2001 (2nd chip/2A0N) (+SGB) (+Sufami)
  4Mbit 512Kbyte 23C401 LH534 TC534 HN623n4 HN623x5 23C4001 LH5S4 CAT534 CXK384
  8Mbit  1Mbyte  23C801 LH538 TC538 HN623n8 23C8001 TC23C8003 CAT548
  16Mbit 2Mbyte  23C1601 LH537 LHMN7 TC5316 M5316
  24Mbit 3Mbyte  23C2401 (seen on SHVC-1J3M board)
  32Mbit 4Mbyte  23C3201 LH535 LHMN5 M5332 23C3202/40pin/SA1 N-32000/44pin/DD1
```

**DIP vs SMD vs Blobs**

Most SNES carts are using DIP chips. SMD chips are used only in carts with
coprocessors (except S-RTC, DSP-n, ST01n). Black blobs are found in several
pirate carts (and in Star Fox, which contains Nintendo's Mario Chip 1, so it's
apparently not a pirate cart).

<a id="snescartridgeromimageheadersandfileextensions"></a>

## SNES Cartridge ROM-Image Headers and File Extensions

Below file headers are dated back to back-up units, which allowed to load
ROM-images from 1.44MB floppy disks into RAM, larger images have been split
into "multi files".

Many of these files do have 512-byte headers. The headers don't contain any
useful information. So, if they are present: Just ignore them. Best way to
detect them is: "IF (filesize AND 3FFh)=200h THEN HeaderPresent=True"
(Headerless cartridges are always sized N*1024 bytes, Carts with header are
N*1024+512 bytes).

**.SMC - Super MagiCom (by Front Far East)**

This extension is often used for ANY type of SNES ROM-images, including for SWC
files.

**.SWC - Super Wild Card (SWC) Header (by Front Far East)**

```text
  000h-001h  ROM Size (in 8Kbyte units)
  002h       Program execution mode
              Bit   Expl.
              7     Entrypoint (0=Normal/Reset Vector, 1=JMP 8000h)
              6     Multi File (0=Normal/Last file, 1=Further file(s) follow)
              5     SRAM mapping    (0=mode20, 1=mode21)
              4     Program mapping (0=mode20, 1=mode21)
              3-2   SRAM Size  (0=32Kbytes, 1=8Kbytes, 2=2Kbytes, 3=None)
              1     Reserved   (zero)
              0     Unknown    (seems to be randomly set to 0 or 1)
  003h       Reserved (zero) (but, set to 01h in homebrew "Pacman" and "Nuke")
  004h-007h  Reserved (zero)
  008h-009h  SWC File ID (AAh, BBh)
  00Ah       File Type (04h=Program ROM, 05h=Battery SRAM, 08h=real-time save)
  00Bh-1FFh  Reserved (zero)
```

**.FIG - Pro Fighter (FIG) header format (by China Coach Ltd)**

```text
  000h-001h  ROM Size (in 8Kbyte units)
  002h       Multi File (00h=Normal/Last file, 40h=Further file(s) follow)
                        (02h=Whatever, used in homebrew Miracle,Eagle,Cen-Dem)
  003h       ROM Mode   (00h=LoROM, 80h=HiROM)
  004h-005h  DSP1/SRAM Mode (8377h=ROM, 8347h=ROM+DSP1, 82FDh=ROM+DSP1+SRAM)
  006h-1FFh  Reserved (zero) (or garbage at 01FCh in homebrew Darkness Demo)
```

**.BIN - Raw Binary**

Contains a raw ROM-image without separate file header.

**.078 - Game Doctor file name format (by Bung)**

Contains a raw ROM-image without separate file header.

Information about multi files is encoded in the "SFxxyyyz.078" filenames.

```text
  SF   Abbreviation for Super Famicom
  xx   Image size in Mbit (2,4,8,16,32) (1-2 chars, WITHOUT leading zero)
  yyy  Game catalogue number (or random number if unknown)
  z    Indicates multi file (A=first, B=second, etc.)
  078  File extension (should be usually 078)
```

**.MGD - Multi Game Doctor ? (by Bung)**

Format Unknown?

**Another Game Doctor version...**

```text
  000h-00Fh  ID "GAME DOCTOR SF 3"
  010h       Unknown (80h)      ;-SRAM size limit
  011h       Unknown (20h)      ;\
  012h       Unknown (21h)      ; DRAM mapping related
  013h-018h  Unknown (6x60h)    ;
  019h       Unknown (20h)      ;
  01Ah       Unknown (21h)      ;
  01Bh-028h  Unknown (14x60h)   ;/
  029h-02Ah  Zero               ;-SRAM mapping related
   011h-020h  512Kbyte DRAM chunk, mapped to upper 32Kbyte of Bank 0xh-Fxh
   021h-024h  512Kbyte DRAM chunk, mapped to lower 32Kbyte of Bank 4xh-7xh
   025h-028h  512Kbyte DRAM chunk, mapped to lower 32Kbyte of Bank Cxh-Fxh
   029h-02Ah  SRAM Flags (bit0-15 = Enable SRAM at 6000-7000 in banks 0xh-Fxh)
  02Bh-1FFh  Zero (Reserved)
```

**Superufo**

```text
  000h       Unknown (20h or 40h)   ;maybe ROM size in 8K units   ?
  001h-007h  Zero
  008h-00Fh  ID "SUPERUFO"
  010h       Unknown (01h)
  011h       Unknown (02h or 04h)   ;maybe rom speed              ?
  012h       Unknown (E1h or F1h)   ;MSB=chipset (Exh or Fxh)     ?
  013h       Unknown (00h)
  014h       Unknown (01h)
  015h       Unknown (03h)
  016h       Unknown (00h)
  017h       Unknown (03h)
  018h-1FFh  Zero
```

**.SFC - Nintendo Developer File (Nintendo)**

Contains a raw ROM-image without separate file header.

Information about multi files is encoded in the "NnnnVv-N.SFC" filenames.

```text
  Nnnn  Game code (4 letters)
  Vv    ROM Version
  N     Disk Number (0=First)
  SFC   Fixed extension (Super FamiCom)
```

This is how Nintendo wanted developers to name their files.

**NSRT Header (can be generated by Nach's NSRT tool)**

This format stores some additional information in a formerly unused 32-byte
area near the end of the 512-byte copier headers.

```text
  1D0h  Unknown/unspecified (LSB=01h..03h, MSB=00h..0Dh) ;maybe ROM mapping
  1D1h  Unknown/unspecified   ;maybe title and/or NSRT version
  1E8h  ID1 "NSRT"
  1ECh  ID2 16h (22 decimal)
  1EDh  Controllers (MSB=Port1, LSB=Port2)
  1EEh  Checksum (sum of bytes at [1D0h..1EDh]+FFh)
  1EFh  Checksum Complement (Checksum XOR FFh)
```

Controller Values:

```text
  00h Gamepad
  01h Mouse
  02h Mouse or Gamepad
  03h Super Scope
  04h Super Scope or Gamepad
  05h Justifier
  06h Multitap
  07h Mouse, Super Scope, or Gamepad
  08h Mouse or Multitap
  09h Lasabirdie
  0Ah Barcode Battler
  0Bh..0Fh Reserved
```

Most copiers also include a parallel PC port interface, allowing

your PC to control the unit and store images on your hard drive.

Copier's contain DRAM from 1 Megabyte to 16 Megabytes, 8MegaBits to

128MegaBits respectively. This is the reason why they are so expensive.

<a id="snescartridgeromimageinterleave"></a>

## SNES Cartridge ROM-Image Interleave

Some ROM images are "interleaved", meaning that their content is ordered
differently as how it appears in SNES memory. The interleaved format dates back
to old copiers, most modern tools use/prefer normal ROM-images without
interleave, but old interleaved files may still show up here and there.

**Interleave used by Game Doctor &amp; UFO Copiers**

These copiers use interleave so that the ROM Header is always stored at file
offset 007Fxxh. For HiROM files, interleave is applied as so: store upper 32K
of all 64K banks, followed by lower 32K of all 64K banks (which moves the
header from 00FFxxh to 007Fxxh). For example, with a 320Kbyte ROM, the ten 32K
banks would be ordered as so:

```text
  0,1,2,3,4,5,6,7,8,9 - Original
  1,3,5,7,9,0,2,4,6,8 - Interleaved
```

For LoROM files, there's no interleave applied (since the header is already at
007Fxxh).

Detecting an interleaved file could be done as so:

```text
  Header must be located at file offset 007Fxxh (ie. in LoROM fashion)
  Header must not be a Sufami Turbo header (=title "ADD-ON BASE CASSETE")
  Header must not be a Satellaview header (=different chksum algorithm)
  Header should not contain corrupted entries
  The "Map Mode" byte at "[007FD5h] ANDed with 0Fh" is 01h,05h,0Ah (=HiROM)
```

If so, the file is interleaved (or, possibly, it's having a corrupted header
with wrong map mode setting).

**Interleave used by Human Stupidity**

There are interleaving &amp; deinterleaving tools, intended to convert normal
ROM-images to/from the format used by above copiers. Using that tools on files
that are already in the desired format will result in messed-up garbage. For
example, interleaving a 320Kbyte file that was already interleaved:

```text
  1,3,5,7,9,0,2,4,6,8 - Interleaved
  3,7,0,4,8,1,5,9,2,6 - Double-Interleaved
```

Or, trying to deinterleave a 320Kbyte file that wasn't interleaved:

```text
  0,1,2,3,4,5,6,7,8,9 - Original
  5,0,6,1,7,2,8,3,9,4 - Mis-de-interleaved
```

One can eventually repair such files by doing the opposite (de-)interleaving
action. Or, in worst case, the user may repeat the wrong action, ending up with
a Triple-Interleaved, or Double-mis-de-interleaved file.

Another case of stupidity would be applying interleave to a LoROM file (which
would move the header &lt;away from&gt; 007Fxxh towards the middle of the file,
ie. the opposite of the intended interleaving effect of moving it &lt;to&gt;
007Fxxh).

**ExHiROM Files**

ExHiROM Files are also having the data ordered differently as in SNES memory.
However, in this special case, the data SHOULD be ordered as so. The ordering
is: Fast HiROM (4Mbytes, banks C0h..FFh), followed by Slow HiROM banks (usually
1-2MByte, banks 40h..4Fh/5Fh) (of which, the header and exception vectors are
in upper 32K of the first Slow HiROM bank, ie. at file offset 40FFxxh). There
are only 2 games using the ExHiROM format:

```text
  Dai Kaiju Monogatari 2 (JP) (5Mbytes) PCB: SHVC-LJ3R-01
  Tales of Phantasia (JP) (6Mbytes)     PCB: SHVC-LJ3M-01
```

The ExHiROM ordering is somewhat "official" as it was defined in Nintendo's
developer manuals. Concerning software, the ordering does match-up with the
checksum calculation algorithm (8MB chksum across 4MB plus mirror(s) of
remaining 1-2MB). Concerning hardware, the ordering may have been 'required' in
case Nintendo did (or planned to) use "odd" sized 5Mbyte/6Mbyte-chips (they DID
produce cartridges with 3MByte/24Mbit chips).

<a id="snescartridgeciclockoutchip"></a>

## SNES Cartridge CIC Lockout Chip

SNES cartridges are required to contain a CIC chip (security chip aka lockout
chip). The CIC is a small 4bit CPU with built-in ROM. An identical CIC is
located in the SNES console. The same 4bit CPU (but with slightly different
code in ROM) is also used in NES consoles/cartridges.

The CIC in the console is acting as "lock", and that in the cartridge is acting
as "key". The two chips are sending random-like bitstreams to each other, if
the data (or transmission timing) doesn't match the expected values, then the
"lock" issues a RESET signal to the console. Thereby rejecting cartridges
without CIC chip (or such with CICs for wrong regions).

**CIC Details**

[SNES Cartridge CIC Pseudo Code](#snes-cartridge-cic-pseudo-code)

[SNES Cartridge CIC Instruction Set](#snes-cartridge-cic-instruction-set)

[SNES Cartridge CIC Notes](#snes-cartridge-cic-notes)

[SNES Cartridge CIC Versions](#snes-cartridge-cic-versions)

[SNES Pinouts CIC Chips](80-timings-unpredictable-pinouts.md#snes-pinouts-cic-chips)

**CIC Disable**

[SNES Common Mods](80-timings-unpredictable-pinouts.md#snes-common-mods)

<a id="snescartridgecicpseudocode"></a>

## SNES Cartridge CIC Pseudo Code

**CicMain**

```text
  CicInitFirst, CicInitTiming, CicRandomSeed, CicInitStreams
  time=data_start, a=1, noswap=1, if snes then noswap=0
 mainloop:
  for x=a to 0Fh
    if nes then Wait(time-5), else if snes then (time-7)     ;\verify idle
    if (nes_6113=0) and (P0.0=1 or P0.1=1) then Shutdown     ;/
    Wait(time+0)                                             ;\
    if (console xor snes) then a=[00h+x].0, else a=[10h+x].0 ; output data
    if noswap then P0.0=a, else P0.1=a                       ;/
    Wait(time+2-data_rx_error)                               ;\
    if (console xor snes) then a=[10h+x].0, else a=[00h+x].0 ; verify input
    if noswap then a=(a xor P0.1), else a=(a xor P0.0)       ;
    if a=1 then Shutdown                                     ;/
    Wait(time+3)                                             ;\output idle
    if noswap then P0.0=0, else P0.1=0                       ;/
    if snes then time=time+92, else if nes then time=time+79
  next x
  CicMangle(00h), CicMangle(10h)                        ;\mangle
  if snes then CicMangle(00h), CicMangle(10h)           ; (thrice on SNES)
  if snes then CicMangle(00h), CicMangle(10h)           ;/
  if snes then noswap=[17h].0   ;eventually swap input/output pins (SNES only)
  a=[17h]
  if a=0 then a=1, time=time+2
  if snes then time=time+44, else if nes then time=time+29
  goto mainloop
```

**CicMangle(buf)**

```text
  for i=[buf+0Fh]+1 downto 1
    a=[buf+2]+[buf+3h]+1
    if a<10h then x=[buf+3], [buf+3]=a, a=x, x=1, else x=0
    [buf+3+x]=[buf+3+x]+a
    for a=x+6 to 0Fh, [buf+a]=[buf+a]+[buf+a-1]+1, next a
    a=[buf+4+x]+8, if a<10h then [buf+5+x]=[buf+5+x]+a, else [buf+5+x]=a
    [buf+4+x]=[buf+4+x]+[buf+3+x]
    [buf+1]=[buf+1]+i
    [buf+2]=NOT([buf+2]+[buf+1]+1)
    time=time+84-(x*6)
  next i
```

Note: All values in [buf] are 4bit wide (aka ANDed with 0Fh).

**CicInitFirst**

```text
  timer=0                       ;reset timer (since reset released)
  P0=00h
  console=P0.3                  ;get console/cartridge flag
  if console
    while P0.2=1, r=r+1         ;get 4bit random seed (capacitor charge time)
    P1.1=1, P1.1=0              ;issue reset to CIC in cartridge
    timer=0                     ;reset timer (since reset released)
  if nes_6113 and (console=1)
    Wait(3), nes_6113_in_console=1, P0.0=1      ;request special 6113 mode
  if nes_6113 and (console=0)
    Wait(6), nes_6113_in_console=P0.1           ;check if 6113 mode requested
```

**CicRandomSeed**

```text
  time=seed_start
  for i=0 to 3                  ;send/receive 4bit random seed (r)
    bit=((i+3) and 3)           ;bit order is 3,0,1,2 (!)
    if console=1 Wait(time+0+i*15), P0.0=r.bit, Wait(time+3+i*15), P0.0=0 ;send
    if console=0 Wait(time+2+i*15), r.bit=P0.1                            ;recv
  next i
```

**CicInitStreams**

```text
  if snes
    if ntsc then x=9, else if pal then x=6
    [01h..0Fh]=B,1,4,F,4,B,5,7,F,D,6,1,E,9,8   ;init stream from cartridge (!)
    [11h..1Fh]=r,x,A,1,8,5,F,1,1,E,1,0,D,E,C   ;init stream from console   (!)
  if nes_usa                ;3193A
    [01h..0Fh]=1,9,5,2,F,8,2,7,1,9,8,1,1,1,5   ;init stream from console
    [11h..1Fh]=r,9,5,2,1,2,1,7,1,9,8,5,7,1,5   ;init stream from cartridge
    if nes_6113_in_console then overwrite [01h]=5 or so ???   ;special-case
  if nes_europe             ;3195A
    [01h..0Fh]=F,7,B,E,F,8,2,7,D,7,8,E,E,1,5   ;init stream from console
    [11h..1Fh]=r,7,B,D,1,2,1,7,E,6,7,A,7,1,5   ;init stream from cartridge
  if nes_hongkong_asia      ;3196A
    [01h..0Fh]=E,6,A,D,F,8,2,7,E,6,7,E,E,E,A   ;init stream from console
    [11h..1Fh]=r,6,A,D,E,D,E,8,E,6,7,A,7,1,5   ;init stream from cartridge
  if nes_uk_italy_australia ;3197A
    [01h..0Fh]=3,5,8,9,3,7,2,8,8,6,8,5,E,E,B   ;init stream from console
    [11h..1Fh]=r,7,9,A,A,1,6,8,5,8,9,1,5,1,7   ;init stream from cartridge
  if_nes_famicombox         ;3198A
    (unknown)
```

Note: In most cases, the PAL region changes are simply inverted or negated NTSC
values (not/neg), except, one NES-EUR value, and most of the NES-UK values are
somehow different. The rev-engineered NES-UK values may not match the exact
original NES-UK values (but they should be working anyways).

**CicInitTiming**

```text
  if snes_d411           -> seed_start=630, data_start=817   ;snes/ntsc
  if snes_d413           -> (unknown?) (same as d411?)       ;snes/pal
  if nes_3193            -> (seems to be same as nes_3195?)  ;nes/usa (v1)
  if nes_3195            -> seed_start=32, data_start=200    ;nes/europe
  if nes_3196            -> (unknown?)                       ;nes/asia
  if nes_3197            -> (unknown?) ("burns five")        ;nes/uk
  if nes_6113            -> seed_start=32, data_start=201    ;nes/usa (v2)
  if nes_6113_in_console -> seed_start=33, data_start=216    ;nes/special
  if nes_tengen          -> seed_start=32, data_start=201    ;nes/cic-clone
  ;now timing errors...
  data_rx_error=0  ;default
  if console=0 and nes_3193a -> randomly add 0 or 0.25 to seed_start/data_start
  if console=0 and snes_d413 -> always add 1.33 to seed_start/data_start (bug)
  if console=0 and nes_6113  -> data_rx_error=1 (and maybe +1.25 on seed/data?)
  if other_chips & chip_revisions -> (unknown?)
```

Note: 3197 reportedly "burns five extra cycles before initialization", but
unknown if that is relative to 3193 &lt;or&gt; 3195 timings, and unknown if it
applies to &lt;both&gt; seed_start and data_start, and unknown if it means 1MHz
&lt;or&gt; 4MHz cycles.

Note: The "data_rx_error" looks totally wrong, but it is somewhat done
intentionally, so there might be a purpose (maybe some rounding, in case 6113
and 3193 are off-sync by a half clock cycle, or maybe an improper bugfix in
case they are off-sync by 1 or more cycles).

**Wait(time)**

Wait until "timer=time", whereas "timer" runs at 1MHz (NES) or 1.024MHz (SNES).
The "time" values are showing the &lt;completion&gt; of the I/O opcodes (ie.
the I/O opcodes &lt;begin&gt; at "time-1").

**Shutdown (should never happen, unless cartridge is missing or wrong region)**

```text
  a=0, if nes then time=830142, else if snes then time=1037682
 endless_loop:          ;timings here aren't 100.000% accurate
  if nes_3195 then time=xlat[P1/4]*174785  ;whereas, xlat[0..3]=(3,2,4,5)
  if (console=0) and (snes or nes_6113) then P0=03h, P1=01h
  if (console=1) then P1=a, Wait(timer+time), a=a xor 4  ;toggle reset on/off
  goto endless_loop
```

<a id="snescartridgecicinstructionset"></a>

## SNES Cartridge CIC Instruction Set

**CIC Registers**

```text
  A  4bit Accumulator
  X  4bit General Purpose Register
  L  4bit Pointer Register (lower 4bit of 6bit HL)
  H  2bit Pointer Register (upper 2bit of 6bit HL)
  C  1bit Carry Flag (changed ONLY by "set/clr c", not by "add/adc" or so)
  PC 10bit Program Counter (3bit bank, plus 7bit polynomial counter)
```

**CIC Memory**

```text
  ROM   512x8bit (program ROM) (NES/EUR=768x8) (max 1024x8 addressable)
  RAM   32x4bit  (data RAM) (max 64x4 addressable)
  STACK 4x10bit  (stack for call/ret opcodes)
  PORTS 4x4bit   (external I/O ports & internal RAM-like ports) (max 16x4)
```

**Newer CIC Opcodes (6113, D411) (and probably F411,D413,F413)**

```text
  00      nop             no operation (aka "addsk A,0" opcode)
  00+n    addsk  A,n      add, A=A+n, skip if result>0Fh
  10+n    cmpsk  A,n      compare, skip if A=n
  20+n    mov    L,n      set L=n
  30+n    mov    A,n      set A=n
  40      mov    A,[HL]   set A=RAM[HL]
  41      xchg   A,[HL]   exchange A <--> RAM[HL]
  42      xchgsk A,[HL+]  exchange A <--> RAM[HL], L=L+1, skip if result>0Fh
  43      xchgsk A,[HL-]  exchange A <--> RAM[HL], L=L-1, skip if result<00h
  44      neg    A        negate, A=0-A                 ;(used by 6113 mode)
  45      ?
  46      out    [L],A    output, PORT[L]=A
  47      out    [L],0    output, PORT[L]=0
  48      set    C        set carry, C=1
  49      clr    C        reset carry, C=0
  4A      mov    [HL],A   set RAM[HL]=A
  4B      ?
  4C      ret             return, pop PC from stack
  4D      retsk           return, pop PC from stack, skip
  4E+n    ?
  52      movsk  A,[HL+]  set A=RAM[HL], L=L+1, skip if result>0Fh
  53      ?                    (guess: movsk  A,[HL-])
  54      not    A        complement, A=A XOR 0Fh
  55      in     A,[L]    input, A=PORT[L]
  56      ?
  57      xchg   A,L      exchange A <--> L
  58+n    ?
  5C      mov    X,A      set X=A
  5D      xchg   X,A      exchange X <--> A
  5E      ???             "SPECIAL MYSTERY INSTRUCTION" ;(used by 6113 mode)
  5F      ?
  60+n    testsk [HL].n   skip if RAM[HL].Bit(n)=1
  64+n    testsk A.n      skip if A.Bit(n)=1
  68+n    clr    [HL].n   set RAM[HL].Bit(n)=0
  6C+n    set    [HL].n   set RAM[HL].Bit(n)=1
  70      add    A,[HL]   add, A=A+RAM[HL]
  71      ?                    (guess: addsk  A,[HL])
  72      adc    A,[HL]   add with carry, A=A+RAM[HL]+C
  73      adcsk  A,[HL]   add with carry, A=A+RAM[HL]+C, skip if result>0Fh
  74+n    mov    H,n      set H=n  ;2bit range, n=0..3 only (used: 0..1 only)
  78+n mm jmp    nmm      long jump, PC=nmm
  7C+n mm call   nmm      long call, push PC+2, PC=nmm
  80+nn   jmp    nn       short jump, PC=(PC AND 380h)+nn
  -       reset           PC=000h
```

Note: "skip" means "do not execute next instruction"

**Older CIC Opcodes (3195) (and probably 3193,3196,3197,etc.)**

```text
  Exchanged opcodes 48 <--> 49 (set/clr C)
  Exchanged opcodes 44 <--> 54 (neg/not A)
  ROM Size is 768x8 (although only 512x8 are actually used)
```

**Note**

The CIC is a 4bit Sharp CPU (maybe a Sharp SM4, but no datasheet exists) (the
instruction seems to be an older version of that in the Sharp SM5K1..SM5K7
datasheets).

<a id="snescartridgecicnotes"></a>

## SNES Cartridge CIC Notes

**Program Counter (PC)**

The 10bit PC register consists of a 3bit bank (which gets changed only by
call/jmp/ret opcodes), and a 7bit polynomial counter (ie. not a linear
counter). After fetching opcode bytes, PC is "incremented" as so:

```text
  PC = (PC AND 380h) + (PC.Bit0 XOR PC.Bit1)*40h + (PC AND 7Eh)/2
```

Ie. the lower 7bit will "increment" through 127 different values (and wrap to
00h thereafter). Address 7Fh is unused (unless one issues a JMP 7Fh opcode,
which would cause the CPU to hang on that address).

```text
  Format     <------------- Valid Address Area ---------->      <--Stuck-->
  Linear     00 01 02 03 04 05 06 07 08 09 0A ... 7C 7D 7E  or  7F 7F 7F 7F
  Polynomial 00 40 60 70 78 7C 7E 3F 5F 6F 77 ... 05 02 01  or  7F 7F 7F 7F
```

To simplify things, programming tools like assemblers/disassemblers may use
"normal" linear addresses (and translate linear/polynomial addressses when
needed - the polynomial addresses are relevant only for encoding bits in
jmp/call opcodes, and for how the data is physically arranged in the chip ROMs
and in ROM-images).

**ROM-Images**

The existing ROM-images are .txt files, containing "0" and "1" BITS in ASCII
format, arranged as a 64x64 (or 96x64) matrix (as seen in decapped chips).

```text
  Line 1..32   --->   Address X+9Fh..80h            ;\Lines (Y)
  Line 33..64  --->   Address X+1Fh..00h            ;/
  Column  1+(n*W) --> Data Bit(n) of Address 000h+Y ;\  ;\
  Column  2+(n*W) --> Data Bit(n) of Address 020h+Y ;   ; Columns (X)
  Column  3+(n*W) --> Data Bit(n) of Address 040h+Y ;   ;
  Column  4+(n*W) --> Data Bit(n) of Address 060h+Y ;   ; chips with 200h-byte
  Column  5+(n*W) --> Data Bit(n) of Address 100h+Y ;   ; (W=8) (64x64 bits)
  Column  6+(n*W) --> Data Bit(n) of Address 120h+Y ;   ;
  Column  7+(n*W) --> Data Bit(n) of Address 140h+Y ;   ;
  Column  8+(n*W) --> Data Bit(n) of Address 160h+Y ;   ;/
  Column  9+(n*W) --> Data Bit(n) of Address 200h+Y ;
  Column 10+(n*W) --> Data Bit(n) of Address 220h+Y ;  chips with 300h-byte
  Column 11+(n*W) --> Data Bit(n) of Address 240h+Y ;  (W=12) (96x64 bits)
  Column 12+(n*W) --> Data Bit(n) of Address 260h+Y ;/
```

Cautions: The bits are inverted (0=1, 1=0) in some (not all) dumps. Mind that
the bytes are arranged in non-linear polynomial fashion (see PC register).
Recommended format for binary ROM-images would be to undo the inversion (if
present), and to maintain the polynomial byte-order.

Note: Known decapped/dumped CICs are D411 and 3195A, and... somebody
decapped/dumped a CIC without writing down its part number (probably=6113).

**CIC Timings**

The NES CICs are driven by a 4.000MHz CIC oscillator (located in the console,
and divided by 4 in the NES CIC). The SNES CICs are driven by the 24.576MHz APU
oscillator (located and divided by 8 in the console's audio circuit, and
further divided by 3 in the SNES CIC) (exception are older SNES mainboards,
which are having a separate 4.00MHz resonator, like the NES).

Ie. internally, the CICs are clocked at 1.000MHz (NES) or 1.024MHz (SNES). All
opcodes are executed within 1 clock cycles, except for the 2-byte long jumps
(opcodes 78h-7Fh) which take 2 clock cycles. The "skip" opcodes are forcing the
following opcode to be executed as a "nop" (ie. the skipped opcode still takes
1 clock cycle; or possibly 2 cycles when skipping long jump opcodes, in case
the CPU supports skipping 2-byte opcodes at all).

After Reset gets released, the CICs execute the first opcode after a short
delay (3195A: randomly 1.0 or 1.25 cycles, D413A: constantly 1.33 cycles)
(whereas, portions of that delay may rely on a poorly falling edge of the
incoming Reset signal).

**CIC Ports**

```text
  Name  Pin  Dir  Expl
  P0.0  1    Out  Data Out    ;\SNES version occassionally swaps these
  P0.1  2    In   Data In     ;/pins by software (ie. Pin1=In, Pin2=Out)
  P0.2  3    In   Random Seed (0=Charged/Ready, 1=Charging/Busy)
  P0.3  4    In   Lock/Key    (0=Cartridge/Key, 1=Console/Lock)
  P1.0  9    Out  Reset SNES  (0=Reset Console, 1=No)
  P1.1  10   Out  Reset Key   (0=No, 1=Reset Key)
  P1.2  11   In   Unused, or Reset Speed A (in 3195A) ;\blink speed of reset
  P1.3  12   In   Unused, or Reset Speed B (in 3195A) ;/signal (and Power LED)
  P2.0  13   -    Unused
  P2.1  14   -    Unused
  P2.2  15   -    Unused
  P2.3  -    -    Unused
  P3.0  -    RAM  Unused, or used as "noswap" flag (in SNES CIC)
  P3.1  -    -    Unused
  P3.2  -    -    Unused
  P3.3  -    -    Unused
```

P0.0-P2.2 are 11 external I/O lines (probably all bidirectional, above
directions just indicates how they are normally used). P2.3-P3.3 are 5 internal
bits (which seem to be useable as "RAM"). Pin numbers are for 16pin NES/SNES
DIP chips (Pin numbers on 18pin SNES SMD chips are slightly rearranged).
P4,P5,P6,P7,P8,P9,PA,PB,PC,PD,PF are unknown/unused (maybe 12x4 further bits,
or mirrors of P0..P3).

**CIC Stream Seeds**

There are different seeds used for different regions. And, confusingly, there
is a NES-CIC clone made Tengen, which uses different seeds than the real CIC
(some of the differences automatically compensated when summing up values, eg.
8+8 gives same 4bit result as 0+0, other differences are manually adjusted by
Tengen's program code).

Many of the reverse-engineered NES seeds found in the internet are based on the
Tengen design (the USA-seeds extracted from the decapped Tengen chip, the
EUR/ASIA/UK-seeds based on sampled Nintendo-CIC data-streams, and then
converted to a Tengen-compatible seed format). To convert them to real CIC
seeds:

```text
  Nintendo[1..F] = Tengen[1..F] - (2,0,0,0,0,0,8,8,8,8,8,8,8,8,2)
```

There are other (working) variations possible, for example:

```text
  Nintendo[1..F] = Tengen[1..F] - (2,0,0,0,0,A,E,8,8,8,8,8,8,8,2)
  (That, for Tengen-USA seeds. The Tengen-style-EUR/ASIA/UK seeds may differ)
```

Whereas, the random seed in TengenKEY[1] is meant to be "r+2" (so subtracting 2
restores "r").

**CIC Stream Logs**

There are some stream logs with filename "XXXX-N.b" where XXXX is the chip
name, and N is the random seed, and bytes in the file are as so:

```text
  Byte 000h, bit0-7 = 1st-8th bit on Pin 1 (DTA.OUT on NES)(DTA.OUT/IN on SNES)
  Byte 001h, bit0-7 = 1st-8th bit on Pin 2 (DTA.IN on NES) (DTA.IN/OUT on SNES)
  Byte 002h, bit0-7 = 9th-16th bit on Pin 1
  Byte 003h, bit0-7 = 9th-16th bit on Pin 2
  etc.
```

Caution: The "N" in the filename is taken as if the seed were transferred in
order Bit 3,2,1,0 (actually it is Bit 3,0,1,2). Ie. file "3195-1.b" would refer
to a NES-EUR-CIC with seed r=4. The signals in the files are sampled at 1MHz
(ie. only each fourth 4MHz cycle).

**The 6113 Chip**

The 6113 chip was invented in 1987, and it replaced the 3193 chip in
US/Canadian cartridges (while US/Canadian consoles kept using 3193 chips). When
used in cartridges, the 6113 does usually "emulate" a 3193 chip. But, for
whatever reason, it can do more:

```text
  Console  Cartridge  Notes
  3193     3193       Works (the "old" way)        ;\used combinations
  3193     6113       Works (the "new" way)        ;/
  6113     6113       Works (special seed/timing)  ;\
  6113     3193       Doesn't work                 ; not used as far as known
  6113     ??         Might work (??=unknown chip) ;/
```

When used in consoles, the 6113 uses slightly different timings and seed values
(and does request cartridges with 6113 chips to use the 6113-mode, too, rather
than emulating the 3193).

One guess: Maybe Nintendo originally used different CICs for NTSC regions (like
3193/3194 for USA/Canada/SouthKorea), and later combined them to one region (if
so, all NES consoles in Canada or SouthKorea should contain 3194/6113 chips,
unlike US consoles which have 3193 chips).

**3195A Signals (NES, Europe)**

The I/O ports are HIGH (for Output "1"), or LOW-Z (for Output "0" or Input).
Raising edges take circa 0.5us, falling edges take circa 3us.

```text
  4MHz Clock Units     ...............................
  1MHz Clock Units       .   .   .   .   .   .   .   .
                          ___________                  ;\Console+Cartridge
  Data Should-be       __|           |________________ ;/should be 3us High
                           __________                  ;\actually 2.5us High
  Data From Console    __.'          ''----.......____ ;/and 3us falling
                           __________                  ;\
  Data From Cartridge  __.'          ''----.......____ ; either same as console
    or, delayed:            __________                 ; or 0.25us later
  Data From Cartridge  ___.'          ''----.......___ ;/
```

After Power-up, the Cartridge CIC does randomly start with good timing, or with
all signals delayed by 0.25us. In other words, the Cartridge CIC executes the
first opcode 1.0us or 1.25us (four or five 4MHz cycles) after Reset gets
released. However, for some reason, pushing the Reset Button doesn't alter the
timing, the random-decision occurs only on Power-up.

**D413A Signals (SNES, Europe)**

The D413A signals are looking strange. First, the software switches signals
High for 3us, but the actual signals are 3.33us High. Second, the signals on
one pin are constantly jumping back'n'forth by 1.33us (in relation to the other
pin).

```text
  3.072MHz Clock Units ...............................
  1.024MHz Clock Units   .  .  .  .  .  .  .  .  .  .
                                ________               ;\Console+Cartridge
  Data Should-be       ________|        |_____________ ;/should be 3us High
                                _________              ;\actually 3.33us high
  Data From/To Console ________|         '--..._______ ;/and 2us falling
                            _________                  ;\
  Data From/To Cart    ____|         '--...___________ ; 1.33us earlier
   or, delayed                      _________          ; or 1.33us later
  Data From/To Cart    ____________|         '--...___ ;/
```

The earlier/later effect occurs because the SNES CICs are occassionally
reversing the data-direction of the pins. Ie. in practice, Data from Cartridge
is constantly 1.33us LATER than from Console.

Software-wise, the D411 (and probably D413A) is programmed as if the Cartridge
CIC would start "immediately", but in practice, it starts 1.33us (four 3.072MHz
cycles) after releasing Reset (that offset seems to be constant, unlike as on
the 3195A where it randomly changes between 1.0us and 1.25us).

<a id="snescartridgecicversions"></a>

## SNES Cartridge CIC Versions

**NES CIC Versions**

```text
  3193,3193A        NES NTSC Cartridges and Consoles       ;\USA,Canada
  6113,6113A,6113B1 NES NTSC Cartridges (not consoles)     ;/(and Korea?)
  3194              Unknown/doesn't exist?
  3195,3193A        NES PAL Cartridges and Consoles "PAL-B";-Europe
  3196(A?)          NES PAL Cartridges and Consoles        ;-Hong Kong,Asia
  3197(A?)          NES PAL Cartridges and Consoles "PAL-A";-UK,Italy,Australia
  3198(A?)          FamicomBox CIC Cartridges and Consoles ;\
  3199(A?)          FamicomBox Coin Timer (not a CIC)      ; Japan
  N/A               Famicom Cartridges and Consoles        ;/
  RFC-CPU10 (?)     NES R.O.B. robot (no CIC, but maybe a 4bit Sharp CPU, too?)
```

**SNES CIC Versions**

```text
  F411,F411A,F411B   SNES NTSC Cartridges-with-SMD-Chipset and Consoles
  D411,D411A,D411B   SNES NTSC Cartridges-with-DIP-Chipset
  F413,F413A,F413B   SNES PAL Cartridges-with-SMD-Chipset and Consoles
  D413,D413A,D413B   SNES PAL Cartridges-with-DIP-Chipset
  SA-1,S-DD1,MCC-BSC SNES Cartridges (coprocessors/mappers with on-chip CIC)
```

**NES CIC Clones**

```text
  23C1033
  337002   ;Tengen's 16pin "Rabbit" CIC clone
  337006   ;Tengen's 40pin "RAMBO-1" mapper with built-in CIC clone
  4051
  7660
  KC5373B
  MX8018
  NINA
  Ciclone  ;homebrew multi-region CIC clone (based on Tengen design)
```

Aside from using cloned CICs, many unlicensed NES cartridges used a different
approach: injecting "wrong" voltages to the console, and "stunning" its CIC.

**SNES CIC Clones**

```text
  10198    - CIC clone
  noname   - CIC clone (black chip without any part number)
  ST10198S - NTSC CIC clone
  ST10198P - PAL CIC clone
  265111   - maybe also a CIC clone (used in Bung Game Doctor SF6)
  D1       - maybe also a CIC clone (used in Super UFO Pro8)
  74LS112  - reportedly also a CIC clone (with fake part number) (UFO Pro6)
  CIVIC 74LS13   16pin - CIC/D411 clone (used in a 8-in-1 pirate cart)
  CIVIC CT6911   16pin - CIC      clone (used in a 7-in-1 pirate cart)
  93C26          16pin - CIC      clone (used in a 8-in-1 pirate cart)
  D1             16pin - CIC? (used in Super VG pirate)
  STS9311A 52583 16pin - CIC clone (used in Donkey King Country 3 pirate)
  black blob     16pin - CIC/D411 clone (used in Sonic the Hedgehog pirate)
```

**CIC Chip Year/Week Date Codes**

```text
  Name   YYWW-YYWW
  3193   8539-8642
  3193A  8547-8733 (in cartridges) (but should be in consoles for more years)
  3195   8627-8638
  3195A  8647-9512
  3197A  8647-9227
  6113   8734-8823
  6113A  8823-8933
  6113B1 8847-9344
```

<a id="snescartlorommappingromdividedinto32kbanksaround1500games"></a>

## SNES Cart LoROM Mapping (ROM divided into 32K banks) (around 1500 games)

**Plain LoROM**

```text
  Board Type               ROM Area               ROM Mirrors
  SHVC-1A0N-01,02,10,20,30 00-7D,80-FF:8000-FFFF  40-7D,C0-FF:0000-7FFF
  SHVC-2A0N-01,10,11,20    00-7D,80-FF:8000-FFFF  40-7D,C0-FF:0000-7FFF
  SHVC-BA0N-01,10          00-7D,80-FF:8000-FFFF  40-7D,C0-FF:0000-7FFF
  SHVC-YA0N-01             00-7D,80-FF:8000-FFFF  40-7D,C0-FF:0000-7FFF
```

**LoROM with SRAM**

```text
  Board Type               ROM Area               SRAM Area
  SHVC-1A1B-04,05,06       00-1F,80-9F:8000-FFFF  70-7D,F0-FF:0000-FFFF
  SHVC-1A3B-11,12,13       00-1F,80-9F:8000-FFFF  70-7D,F0-FF:0000-FFFF
  SHVC-1A5B-02,04          00-1F,80-9F:8000-FFFF  70-7D,F0-FF:0000-FFFF
  SHVC-2A3B-01             00-3F,80-BF:8000-FFFF  70-7D,F0-FF:0000-7FFF
  SHVC-2A3M-01 with MAD-R  00-3F,80-BF:8000-FFFF  70-7D,F0-FF:0000-7FFF
  SHVC-2A3M-01,11,20       00-7D,80-FF:8000-FFFF  70-7D,F0-FF:0000-7FFF
  SHVC-1A3B-20             00-7D,80-FF:8000-FFFF  70-7D,F0-FF:0000-7FFF
  SHVC-1A1M-01,11,20       00-7D,80-FF:8000-FFFF  70-7D,F0-FF:0000-7FFF
  SHVC-2A1M-01             00-7D,80-FF:8000-FFFF  70-7D,F0-FF:0000-7FFF
  SHVC-BA1M-01             00-7D,80-FF:8000-FFFF  70-7D,F0-FF:0000-7FFF
  SHVC-1A3M-10,20,21,30    00-7D,80-FF:8000-FFFF  70-7D,F0-FF:0000-7FFF
  SHVC-BA3M-01             00-7D,80-FF:8000-FFFF  70-7D,F0-FF:0000-7FFF
  SHVC-1A5M-01,11,20       00-7D,80-FF:8000-FFFF  70-7D,F0-FF:0000-7FFF
  SHVC-2A5M-01             00-7D,80-FF:8000-FFFF  70-7D,F0-FF:0000-7FFF
  SHVC-1A7M-01             ?                      ?
```

Note that 2A3M-01 exists with/without MAD-R (and have different mappings).

Note that 1A3B-20 differs from earlier 1A3B-xx versions.

The older boards map SRAM to the whole 64K areas at banks 70h-7Dh/F0-FFh.

The newer boards map SRAM to the lower 32K areas at banks 70h-7Dh/F0-FFh (this
allows "BigLoROM" games to use the upper 32K of that banks as additional LoROM
banks, which is required for games with more than 3MB LoROM).

Most of the existing boards contain 0K, 2K, 8K, or 32K SRAM. A few games
contail 64K or 128K SRAM, which is divided into 32K chunks, mapped to bank 70h,
71h, etc.)

Some LoROM games are bigger than 2Mbytes (eg. Super Metroid, Gunple, Wizardry
6, Derby Stallion 3), these have bank 0-3Fh mapped in the 32K LoROM banks as
usually, and bank 40h and up each mapped twice in the 64K hirom banks.

Note: There's also a different "SpecialLoROM" mapping scheme for 3MByte ROMs
(used by Derby Stallion 96 and Sound Novel Tsukuru; aside from the special ROM
mapping, these cartridges have an additional Data Pack Slot).

<a id="snescarthirommappingromdividedinto64kbanksaround500games"></a>

## SNES Cart HiROM Mapping (ROM divided into 64K banks) (around 500 games)

**Plain HiROM**

```text
  Board               ROM Area      ROM Mirrors   SRAM Area
  Type                at 0000-FFFF  at 8000-FFFF  (none such)
  SHVC-BJ0N-01,20     40-7d,c0-ff   00-3f,80-bf   N/A
  SHVC-YJ0N-01        40-7d,c0-ff   00-3f,80-bf   N/A
  SHVC-1J0N-01,10,20  40-7d,c0-ff   00-3f,80-bf   N/A
  SHVC-2J0N-01,10,11  40-7d,c0-ff   00-3f,80-bf   N/A
  SHVC-3J0N-01        40-6f,c0-ef   00-2f,80-af   N/A
```

The SHVC-3J0N-01 board contains 3 ROM chips (memory is divided into chunks of
16 banks, with one ROM per chunk, and with each 4th chunk being left empty, ie.
bank 30-3F,70-7D,B0-BF,F0-FF are open-bus).

**HiROM with SRAM**

```text
  Board               ROM Area      ROM Mirrors   SRAM Area
  Type                at 0000-FFFF  at 8000-FFFF  at 6000-7FFF
  SHVC-1J3B-01        40-7d,c0-ff   00-3f,80-bf   20-3f,a0-bf
  SHVC-1J1M-11,20     40-7d,c0-ff   00-3f,80-bf   20-3f,a0-bf
  SHVC-1J3M-01,11,20  40-7d,c0-ff   00-3f,80-bf   20-3f,a0-bf
  SHVC-BJ3M-10        40-7d,c0-ff   00-3f,80-bf   20-3f,a0-bf
  SHVC-1J5M-11,20     40-7d,c0-ff   00-3f,80-bf   20-3f,a0-bf
  SHVC-2J3M-01,11,20  40-7d,c0-ff   00-3f,80-bf   10-1f,30-3f,90-9f,b0-bf
  SHVC-2J5M-01        40-7d,c0-ff   00-3f,80-bf   10-1f,90-9f,30-3f,b0-bf
  SHVC-LJ3M-01        40-7d,c0-ff   00-3f,80-bf   80-bf
```

The SHVC-LJ3M-01 board uses ExHiROM mapping (meaning that bank 00h-7Dh contain
different ROM banks than 80h-FFh).
