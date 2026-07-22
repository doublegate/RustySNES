# Fullsnes — FLASH Backup, Cheat Devices, Tri-Star, Pirate Multicarts, Copiers & CD-ROM Drive

[Index](00-index.md) · [« Cartridge Add-Ons](62-cartridge-addons-satellaview-modems.md) · [Hotel Boxes & Arcade Machines »](70-hotel-arcade-nss-sfcbox.md)

**Sections in this file:**

- [SNES Cart FLASH Backup](#snes-cart-flash-backup)
- [SNES Cart Cheat Devices](#snes-cart-cheat-devices)
  - [SNES Cart Cheat Devices - Code Formats](#snes-cart-cheat-devices-code-formats)
  - [SNES Cart Cheat Devices - Game Genie](#snes-cart-cheat-devices-game-genie)
  - [SNES Cart Cheat Devices - Pro Action Replay I/O Ports](#snes-cart-cheat-devices-pro-action-replay-io-ports)
  - [SNES Cart Cheat Devices - Pro Action Replay Memory](#snes-cart-cheat-devices-pro-action-replay-memory)
  - [SNES Cart Cheat Devices - X-Terminator & Game Wizard](#snes-cart-cheat-devices-x-terminator-game-wizard)
  - [SNES Cart Cheat Devices - Game Saver](#snes-cart-cheat-devices-game-saver)
  - [SNES Cart Cheat Devices - Theory](#snes-cart-cheat-devices-theory)
- [SNES Cart Tri-Star (aka Super 8) (allows to play NES games on the SNES)](#snes-cart-tri-star-aka-super-8-allows-to-play-nes-games-on-the-snes)
- [SNES Cart Pirate X-in-1 Multicarts (1)](#snes-cart-pirate-x-in-1-multicarts-1)
- [SNES Cart Pirate X-in-1 Multicarts (2)](#snes-cart-pirate-x-in-1-multicarts-2)
- [SNES Cart Copiers](#snes-cart-copiers)
  - [SNES Cart Copiers - Front Fareast (Super Magicom & Super Wild Card)](#snes-cart-copiers-front-fareast-super-magicom-super-wild-card)
  - [SNES Cart Copiers - CCL (Supercom & Pro Fighter)](#snes-cart-copiers-ccl-supercom-pro-fighter)
  - [SNES Cart Copiers - Bung (Game Doctor)](#snes-cart-copiers-bung-game-doctor)
  - [SNES Cart Copiers - Super UFO](#snes-cart-copiers-super-ufo)
  - [SNES Cart Copiers - Sane Ting (Super Disk Interceptor)](#snes-cart-copiers-sane-ting-super-disk-interceptor)
  - [SNES Cart Copiers - Gamars Copier](#snes-cart-copiers-gamars-copier)
  - [SNES Cart Copiers - Venus (Multi Game Hunter)](#snes-cart-copiers-venus-multi-game-hunter)
  - [SNES Cart Copiers - Others](#snes-cart-copiers-others)
  - [SNES Cart Copiers - Misc](#snes-cart-copiers-misc)
  - [SNES Cart Copiers - Floppy Disc Controllers](#snes-cart-copiers-floppy-disc-controllers)
  - [SNES Cart Copiers - Floppy Disc NEC uPD765 Commands](#snes-cart-copiers-floppy-disc-nec-upd765-commands)
  - [SNES Cart Copiers - Floppy Disc FAT12 Format](#snes-cart-copiers-floppy-disc-fat12-format)
  - [SNES Cart Copiers - BIOSes](#snes-cart-copiers-bioses)
- [SNES Cart CDROM Drive](#snes-cart-cdrom-drive)
- [SNES Cart CDROM - Memory and I/O Map](#snes-cart-cdrom-memory-and-io-map)
- [SNES Cart CDROM - CDROM Bootsector and Volume Descriptor](#snes-cart-cdrom-cdrom-bootsector-and-volume-descriptor)
- [SNES Cart CDROM - BIOS Cartridge](#snes-cart-cdrom-bios-cartridge)
- [SNES Cart CDROM - BIOS Functions](#snes-cart-cdrom-bios-functions)
- [SNES Cart CDROM - Mechacon](#snes-cart-cdrom-mechacon)
- [SNES Cart CDROM - Decoder/FIFO](#snes-cart-cdrom-decoderfifo)
- [SNES Cart CDROM - Component List](#snes-cart-cdrom-component-list)

---

<a id="snescartflashbackup"></a>

## SNES Cart FLASH Backup

Most SNES games are using battery-backed SRAM for storing data, the only
exception - which do use FLASH memory - are the JRA PAT BIOS cartridges for the
SFC Modem:

[SNES Add-On SFC Modem (for JRA PAT)](50-controllers.md#snes-add-on-sfc-modem-for-jra-pat)

There are two JRA PAT versions, the older one (1997) supports only AMD FLASH,
the newer one (1999) supports AMD/Atmel/Sharp FLASH chips.

```text
  ID=2001h - AM29F010 AMD (128Kbyte)      ;supported by BOTH bios versions
  ID=D51Fh - AT29C010A Atmel (128Kbyte)   ;supported only by newer bios version
  ID=32B0h - LH28F020SUT Sharp (256Kbyte?);supported only by newer bios version
```

The FLASH Size size defined in entry [FFBCh] of the Cartridge Header (this is
set to 07h in JRA PAT, ie. "(1K SHL 7)=128Kbytes").

There don't seem to be any data sheets for the Sharp LH28F020SUT-N80 chip (ID
B0h,32h) (so not 100% sure if it's really 256Kbytes), anyways, it does somehow
resemble LH28F020SU-N (5V, ID B0h,30h) and LH28F020SU-L (5V/3.3V, ID B0h,31h).

**JRA PAT Memory Map**

```text
  80h-9Fh:8000h-FFFFh  ;1Mbyte LoROM (broken into 32 chunks of 32Kbytes)
  C0h-C3h:0000h-7FFFh  ;128Kbyte FLASH (broken into 4 chunks of 32Kbytes)
```

AMD FLASH

**Get Device ID (Type 1 - AMD)**

```text
  [C05555h]=AAh, [C02AAAh]=55h, [C05555h]=90h           ;enter ID mode
  manufacturer=01h=[C00000h], device_type=20h=[C00001h] ;read ID (AM29F010)
  [C05555h]=AAh, [C02AAAh]=55h, [C05555h]=F0h           ;terminate command
```

**Erase Entire Chip (Type 1 - AMD)**

```text
  [C05555h]=AAh, [C02AAAh]=55h, [C05555h]=80h           ;prepare erase
  [C05555h]=AAh, [C02AAAh]=55h, [C05555h]=10h           ;erase entire chip
  repeat, stat=[C00000h], until stat.bit7=1=okay, or stat.bit5=1=timeout
```

**Erase 16Kbyte Sector (Type 1 - AMD)**

```text
  [C05555h]=AAh, [C02AAAh]=55h, [C05555h]=80h           ;prepare erase
  [C05555h]=AAh, [C02AAAh]=55h, [Cxx000h]=30h           ;erase 16kbyte sector
  repeat, stat=[Cxx000h], until stat.bit7=1=okay, or stat.bit5=1=timeout
```

**Write Single Data Byte (Type 1 - AMD)**

```text
  [C05555h]=AAh, [C02AAAh]=55h, [C05555h]=A0h           ;write 1 byte command
  [Cxxxxxh]=dta                                         ;write the data byte
  repeat, stat=[Cxxxxxh], until stat.bit7=dta.bit7=okay, or stat.bit5=1=timeout
```

**Notes**

After AMD timeout errors, one should issue one dummy/status read from [C00000h]
to switch the device back into normal data mode (at least, JRA PAT is doing it
like so, not too sure if that is really required/correct).

ATMEL FLASH

**Get Device ID (Type 2 - Atmel)**

```text
  [C05555h]=AAh, [C02AAAh]=55h, [C05555h]=90h           ;enter ID mode
  manufacturer=1Fh=[C00000h], device_type=D5h=[C00001h] ;read ID (AT29C010A)
  [C05555h]=AAh, [C02AAAh]=55h, [C05555h]=F0h           ;terminate command
```

**Erase Entire Chip (Type 2 - Atmel)**

```text
  [C05555h]=AAh, [C02AAAh]=55h, [C05555h]=80h           ;prepare erase
  [C05555h]=AAh, [C02AAAh]=55h, [C05555h]=10h           ;erase entire chip
  wait two frames (or check if bit6 toggles on each read from [C00000h])
```

**Erase 16Kbyte Sector (Type 2 - Atmel)**

```text
  No such command (one can write data without erasing)
  (to simulate a 16K-erase: write 128 all FFh-filled 128-byte blocks)
```

**Write 1..128 Data Bytes (within 128-byte boundary) (Type 2 - Atmel)**

```text
  [C05555h]=AAh, [C02AAAh]=55h, [C05555h]=A0h           ;write 1..128 byte(s)
  [Cxxxxxh+0..n]=dta[0..n]                              ;write the data byte(s)
  repeat, stat=[Cxxxxxh+n], until stat=dta[n]           ;wait last written byte
```

**Notes**

The JRA PAD functions do include a number of wait-vblank delays between various
ATMEL commands, that delays aren't shown in above flowcharts.

SHARP FLASH

**Get Device ID (Type 3 - Sharp)**

```text
  [C00000h]=90h                                         ;enter ID mode
  manufacturer=B0h=[C00000h], device_type=32h=[C00001h] ;read ID (LH28F020SUT)
  [C00000h]=FFh                                         ;terminate command
```

**Set/Reset Protection (Type 3 - Sharp)**

```text
  [C00000h]=57h/47h, [C000FFh]=D0h    ;<-- C000FFh (!)  ;set/reset protection
  repeat, stat=[C00000h], until stat.bit7=1             ;wait busy
  if stat.bit4=1 or stat.bit5=1 then [C00000h]=50h      ;error --> clear status
  [C00000h]=FFh                                         ;terminate command
```

**Erase Entire Chip (Type 3 - Sharp)**

```text
  [C00000h]=A7h, [C00000h]=D0h                          ;erase entire chip
  repeat, stat=[C00000h], until stat.bit7=1             ;wait busy
  if stat.bit4=1 or stat.bit5=1 then [C00000h]=50h      ;error --> clear status
  [C00000h]=FFh                                         ;terminate command
```

**Erase 16Kbyte Sector (Type 3 - Sharp)**

```text
  [C00000h]=20h, [Cxx000h]=D0h                          ;erase 16kbyte sector
  repeat, stat=[C00000h], until stat.bit7=1             ;wait busy
  if stat.bit4=1 or stat.bit5=1 then [C00000h]=50h      ;error --> clear status
  [C00000h]=FFh                                         ;terminate command
  if failed, issue "Reset Protection", and retry
```

**Write Single Data Byte (Type 3 - Sharp)**

```text
  [C00000h]=40h                                         ;write 1 byte command
  [Cxxxxxh]=dta                                         ;write the data byte
  repeat, stat=[C00000h], until stat.bit7=1             ;wait busy
  ;below error-check & terminate are needed only after writing LAST byte
  if stat.bit4=1 or stat.bit5=1 then [C00000h]=50h      ;error --> clear status
  [C00000h]=FFh                                         ;terminate command
```

PCB VERSIONS

**Older PCB "SHVC-1A9F-01" (1996) (DIP) (for JRA-PAT and SPAT4)**

```text
  U1 32pin ROM
  U2 32pin AMD AM29F010-90PC (FLASH)
  U3 16pin SN74LS139AN
  U4 16pin D411B (CIC)
```

**Newer PCB "SHVC-1A8F-01" (1999) (SMD) (for JRA-PAT-Wide)**

```text
  U1 32pin ROM
  U2 32pin Sharp LH28F020SUT-N80 (FLASH)
  U3 16pin 74AC139
  U4 18pin F411B (CIC)
  U5 14pin 74AC08
```

**See Also**

Another approach for using FLASH backup is used in carts with Data Pack slots:

[SNES Cart Data Pack Slots (satellaview-like mini-cartridge slot)](62-cartridge-addons-satellaview-modems.md#snes-cart-data-pack-slots-satellaview-like-mini-cartridge-slot)

<a id="snescartcheatdevices"></a>

## SNES Cart Cheat Devices

**Code Format Summary**

```text
  Pro Action Replay         AAAAAADD        raw 8-digits         WRAM
  Pro Action Replay Mk2/Mk3 AAAAAADD        raw 8-digits         WRAM/ROM/SRAM
  X-Terminator/Game Wizard  AAAAAADD        raw 8-digits         WRAM
  Game Genie/Game Mage      DDAA-AAAA       encrypted 4-4 digits ROM/SRAM
  Gold Finger               AAAAADDDDDDCCW  raw 14-digits        DRAM/SRAM
  Front Far East            NNAAAAAADD..    raw 10..80 digits    DRAM offset
```

**Code Format Details**

[SNES Cart Cheat Devices - Code Formats](#snes-cart-cheat-devices-code-formats)

**Hardware Details**

[SNES Cart Cheat Devices - Game Genie](#snes-cart-cheat-devices-game-genie)

[SNES Cart Cheat Devices - Pro Action Replay I/O Ports](#snes-cart-cheat-devices-pro-action-replay-io-ports)

[SNES Cart Cheat Devices - Pro Action Replay Memory](#snes-cart-cheat-devices-pro-action-replay-memory)

[SNES Cart Cheat Devices - X-Terminator &amp; Game Wizard](#snes-cart-cheat-devices-x-terminator-game-wizard)

[SNES Cart Cheat Devices - Game Saver](#snes-cart-cheat-devices-game-saver)

[SNES Cart Cheat Devices - Theory](#snes-cart-cheat-devices-theory)

**Cheat Devices &amp; Number of Hardware/Software patches &amp; Built-in codes**

```text
  Name                            Hardware/ROM  Software/WRAM  Built-in
  Pro Action Replay               None  (of 4)  4              None
  Pro Action Replay Mk2a/b        0/2/4 (of 4)  100            None
  Pro Action Replay Mk3           1/5   (of 7)  100            ? games
  Game Genie (Codemasters/Galoob) 5     (of 6)  None           None
  Game Mage (Top Game & Company)  8?            None?          250 codes?
  X-Terminator (Fire)             None  (of 0)  4              None
  X-Terminator 2 (noname)         None  (of 0)  64             307 games
  Game Wizard (Innovation)        None?         ?              ?
  Game Saver (Nakitek) allows to save WRAM/VRAM snapshots in non-battery DRAM
  Game Saver+ (Nakitek) allows to save WRAM/VRAM snapshots in battery DRAM
  Super UFO (copier, supports Gold Finger and X-Terminator codes)
  Super Wild Card/Magicom (copiers, support Gold Finger and Front Far East)
  Parame ROM Cassette Vol 1-5 (by Game Tech) (expansions for X-Terminator 2)
```

Note: The Game Mage's stylished "GAME|~AGE)" logo is often misread as
"Gametaged".

**Links**

http://www.gamegenie.com/cheats/gamegenie/snes/index.html

http://www.world-of-nintendo.com/pro_action_replay/super_nes.shtml

http://www.gamefaqs.com/snes/562623-harvest-moon/faqs/10690

http://www.gamefaqs.com/snes/588741-super-metroid/faqs/5667

<a id="snescartcheatdevicescodeformats"></a>

### SNES Cart Cheat Devices - Code Formats

**PAR AAAAAADD - Normal Pro Action Replay Codes (Datel)**

The Pro Action Replay is a cheat device for the SNES produced by Datel. The
original PAR only support 3 codes, but the PAR2 supports 255 and has a built-in
trainer for code searcher. There is also a PAR3, but the added features are
unknown.

```text
  AAAAAADD  ;-address (AAAAAA) and data (DD)
```

Address can be a ROM, SRAM, or WRAM location. Patching cartridge memory (both
ROM and SRAM) is implemented by hardware (supported by PAR2-PAR3 only, not by
PAR1 or X-Terminator 1-2). Patching WRAM is done by software (rewriting the
values on each Vblank NMI). WRAM addresses must be specified as 7E0000h-7FFFFFh
(mirrors at nn0000h-nn1FFFh aren't recognized by the BIOSes).

**PAR 7E000000 - Do nothing**

This is the most important PAR code (required as padding value, since the GUI
doesn't allow to remove items from the code list):

**PAR FE0000xx..FFFFFFxx - Pre-boot WRAM patch (PAR1 only)**

Writes xx to the corresponding WRAM address at 7E0000h..7FFFFFh, this is done
only once, and it's done BEFORE starting the game (purpose unknown - if any).

**PAR 00600000 - Disable Game's NMI handler (PAR3 only)**

Disables the game's NMI handler (executes only the NMI handler of the PAR
BIOS).

**PAR DEADC0DE - Special Multi-Byte Code Prefix (PAR2a/b and PAR3 only)**

Allows to hook program code, this feature is rarely used.

```text
  DEADC0DE  ;-prefix (often misspelled as "DEADCODE", with "O" instead "0")
  AAAAAANN  ;-address (AAAAAA) and number of following 4-byte groups (NN)
  DDEEFFGG  ;-first 4-byte group    (DD=1st byte, .. GG=4th byte)
  HHIIJJKK  ;-second 4-byte group   (HH=5th byte, .. KK=8th byte) (if any)
  ...       ;-further 4-byte groups (etc.)                        (if any)
```

The data portion (DD,EE,FF..) (max 62h*4 = 188h bytes) is relocated to SRAM (in
the PAR cartridge), and the ROM address AAAAAA is patched by a 4-byte "JMP
nnnnnn" opcode (doing a far jump to the address of the relocated SRAM code;
this would be at 006A80h in PAR3, at 006700h in PAR2a/b, and isn't supported in
PAR1). There seems to be no special action required when returning control to
the game (such like disabling the SRAM - if the hardware does support that at
all?) (or such actions are required only for HiROM games that have their own
SRAM at 6000h?) (or games are typically accessing SRAM at 306xxxh, so there is
no conflict with PAR memory at 006xxxh?).

One can use only one DEADC0DE at a time, and, when using it, there are some
more restrictions: On PAR2a/b one cannot use ANY other hardware/software
patches. On PAR3 one can keep using ONE hardware patch (and any number of
software patches).

**PAR C0DEnn00 - Whatever (X-Terminator 2 only - not an official PAR code)**

Somehow changes the NMI (and IRQ) handling of the X-Terminator 2, "nn" can be
00..06.

**Game Genie Codes (Codemasters/Galoob)**

```text
  DDAA-AAAA  ;-encrypted data (DD) and encrypted/shuffled address (AA-AAAA)
```

Address can be a ROM, or SRAM location (internal WRAM isn't supported). To
decrypt the code, first replace the Genie Hex digits by normal Hex Digits:

```text
  Genie Hex:    D  F  4  7  0  9  1  5  6  B  C  8  A  2  3  E
  Normal Hex:   0  1  2  3  4  5  6  7  8  9  A  B  C  D  E  F
```

Thereafter, DD is okay, but AAAAAA still needs to be deshuffled:

```text
  ijklqrst opabcduv wxefghmn   ;Genie Address (i=Bit23 ... n=Bit0)
  abcdefgh ijklmnop qrstuvwx   ;SNES Address (a=Bit23 ... x=Bit0)
```

Aside from being generally annoying, the encryption makes it impossible to make
codes like "Start in Level NN" (instead, one would need to make separate codes
for each level).

Game Genie codes can be reportedly also used with the Game Mage. And, the PAR3
includes a "CONV" button for converting Game Genie codes. When manually
decrypting them, Game Genie codes would also work on PAR2 (although the PAR2
won't allow to use five ROM patches at once).

**Gold Finger / Goldfinger Codes (unknown who created this format)**

These codes are rarely used, and there isn't much known about them. Reportedly,
they have been supported by "certain copiers" (unknown which ones... Super UFO,
and also copiers from Front Far East?).

```text
  AAAAADDEEFFCCW  ;-Address (AAAAA), Data (DD,EE,FF), Checksum (CC), Area (W)
```

The Address is a ROM address, not a CPU address. Data can be 1-3 bytes, when
using less than 3 bytes, pad the EE,FF fields with "XX" (and treat them as zero
in the checksum calculation). Checksum is calculated as so:

```text
  CC = A0h + AAAAA/10000h + AAAAA/100h + AAAAA + DD (+ EE (+ FF))
```

W tells the copier whether to replace the byte in the DRAM (ROM image) or the
SRAM (Saved game static RAM) of the copier:

```text
  W=0  DRAM (ROM image) (reportedly also for W=2,8,A,C,F)
  W=1  SRAM (Saved game image)
```

The 5-digit address allows to access max 1Mbyte. The address is an offset
within the ROM-image (excluding any 200h-byte header). The first LoROM byte
would be at address 00000h. Some copiers are using interleaved HiROM images -
unknown if any such interleave is used on code addresses - if so, first HiROM
byte would be at "ROMsize/2" (in middle of ROM-image), otherwise it'd be at
00000h (at begin of ROM-image).

Note: It doesn't seem to be possible to enter "X" digits in all copiers. Double
Pro Fighter Q allows to enter "X".

**Front Far East Codes (Front Far East)**

Supported by Front Far East copiers (Super Magicom and/or Super Wild Card?).
This format is even less popular than the Gold Finger format.

```text
  NNAAAAAADD..    Number of bytes (NN), Address (AAAAAA), Data (DD..)
```

Allows to change 1..36 bytes; resulting in a code length of 10..80 digits (to
avoid confusion with 14-digit Gold Finger codes, Front Far East wants Gold
Finger codes to be prefixed by a "G").

AAAAAA is a 24bit offset within the ROM-image (excluding any 200h-byte header).
As far as known, Front Far East didn't use interleaved ROM-images, so the
offset should be straight.

<a id="snescartcheatdevicesgamegenie"></a>

### SNES Cart Cheat Devices - Game Genie

**Game Genie BIOS Versions**

There are at least three BIOS versions, named "GENSRC", "K7", and "ed":

```text
  "GENSRC"  32Kbytes LoROM, straight I/O addresses  CRC32=AC94F94Ah
  "K7"      32Kbytes LoROM, messy I/O addresses     CRC32=F8D4C303h
  "ed"      64Kbytes LoROM, messy I/O addresses     CRC32=58CBF2FEh
```

The names are stored at ROM-offset 7FE0h (aka SNES address 00FFE0h). Most of
the cartridge header contains garbage (no checksum, wrong ROM size, etc.), only
the 21-byte title field is (more or less) correct:

```text
  "Game Genie      ",0,0,0,0,0   ;32K versions (GENSRC & K7)
  "Game Genie         Jo"        ;64K version (ed)
```

Note: All three versions support only 5 codes to be entered.

**Game Genie I/O Ports**

The "GENSRC" version uses quite straight I/O addresses, the "K7" and "ed"
versions have the addresses messed up (unused gaps between the codes, several
mirrors, of which, mirrors with address bit8 and bits16-23 all zero are having
address bit0 inverted).

```text
  Version   "GENSRC"          "K7" & "ed"
  Control   W:008000h         W:xx8100h, W:008001h  ;ed: xx=00, K7: xx=FF
  CodeFlags R/W:008001h       R:FF8001h, W:008000h  ;bit 0-4 = enable code 1-5
  CodeMsb   R/W:008003h+N*4   R:FF8005h+N*6, W:008004h+N*6  ;\
  CodeMid   R/W:008004h+N*4   R:FF8006h+N*6, W:008007h+N*6  ; N=0-4 for
  CodeLsb   R/W:008005h+N*4   R:FF8007h+N*6, W:008006h+N*6  ; code 1-5
  CodeData  R/W:008006h+N*4   R:FF8008h+N*6, W:008009h+N*6  ;/
```

Other (accidently) used I/O addresses are: W:004017h (bugged joypad access),
and W:00FFEAh/00FFEBh (used in an attempt to install a bugged NMI handler with
RET instead of RETI opcode; the hardware is hopefully ignoring that attempt).

**Control Register**

Allows to select what is mapped to memory. Used values are:

```text
  00h Select Game Genie BIOS
  02h Select Game Genie I/O Ports
  06h Select Game Cartridge
  07h Select Game Cartridge and keep it selected
```

This register is set to 00h on /RESET (warmboot &amp; coldboot).

**Code Flags Register**

Used to enable the 5 codes:

```text
  Bit0-4  Enable Code 1..5  (0=Disable, 1=Enable)
  Bit5-7  Should be zero
```

This register is set to 00h on initial power-up (coldboot), but kept intact on
/RESET (warmboot), allowing to restore the previously enabled codes.

**CodeAddress/CodeData Registers (5*4 bytes)**

Contains the 24bit address and 8bit data values (in decrypted form). The
registers are kept intact on /RESET (warmboot), allowing to restore the
previously entered codes - however, when restoring the codes, the "K7" and "ed"
BIOS versions are ORing the LSB of of the 24bit address value with 01h, thereby
destroying all codes with even addresses (unknown why that's been done).

**Chipset**

The exact chipset is unknown, there should be the ROM and some logic (and no
SRAM). There is also a LED and a 2-position (?) switch (unknown function).

**XXX**

For more in-depth info see "genie.txt" from Charles MacDonald.

```text
  http://cgfm2.emuviews.com/txt/genie.txt
```

**Game Genie 2 (prototype)**

There is also an unreleased Game Genie 2 prototype, the thing includes a small
LCD screen, five push buttons, four LEDs, battery backed SRAM, a 8255 PIO, a
huge ACTEL chip, and some expansion connectors.

<a id="snescartcheatdevicesproactionreplayioports"></a>

### SNES Cart Cheat Devices - Pro Action Replay I/O Ports

**PAR1 I/O Ports (W)**

```text
  008000h                  ;Code 0-3 (MID) (shared for code 0..3)
  010000h,010001h,010002h  ;Code 0 (DTA,LSB,MSB) (not used by BIOS)
  010003h                  ;Control (set to FFh)
  010004h,010005h,010006h  ;Code 1 (DTA,LSB,MSB) (not used by BIOS)
  010007h,010008h,010009h  ;Code 2 (DTA,LSB,MSB) (used as NMI vector.LSB)
  01000Ah,01000Bh,01000Ch  ;Code 3 (DTA,LSB,MSB) (used as NMI vector.MSB)
```

Most I/O ports are overlapping WRAM in bank 01h (unlike PAR2-PAR3 which use
bank 10h). The four code registers would allow to apply 4 hardware patches, the
PAR1 BIOS actually has provisions for doing that, but, before applying those
patches it erases code 0-3 (and does then use code 2-3 for patching the NMI
vector at 00FFEAh for applying the codes as WRAM software patches).

The PAR1 supports only LoROM addresses (address bit15 removed, and bit23-16
shifted down). The PAR1 does not (maybe cannot) disable unused codes; instead,
it directs them to an usually unused ROM location at 00FFF6h (aka 007FF6h after
removing bit15). Applying the codes is done in order DTA,MID,LSB,MSB, whereof,
the "shared" MID value is probably applied on the following LSB port write.

The control register is set to FFh before starting the game, purpose is unknown
(maybe enable the codes, or write-protect them, or disable the BIOS ROM; in
case that can be done by software).

**PAR2 I/O Ports (W)**

```text
  100000h,100001h,100002h,100003h  ;Code 0 (DTA,LSB,MID,MSB) (code 0)
  100004h,100005h,100006h,100007h  ;Code 1 (DTA,LSB,MID,MSB) (code 1)
  100008h,100009h,10000Ah,10000Bh  ;Code 2 (DTA,LSB,MID,MSB) (code 2/NMI.LSB)
  10000Ch,10000Dh,10000Eh,10000Fh  ;Code 3 (DTA,LSB,MID,MSB) (code 3/NMI.MSB)
  100010h      ;Control A (set to 00h or FFh)
  C0A00nh      ;Control B (address LSBs n=0..7) (written data=don't care)
```

The registers overlapping the WRAM area are similar as for PAR1. The PAR2 BIOS
allows to use the code registers for ROM patches (and/or hooking the NMI
handler for WRAM patches).

Similar as in PAR1, address bit23-16 are shifted down, but with bit15 being
moved to bit23 (still a bit messy, but HiROM is now supported). Unused codes
are redirected to 00FFF6h (aka 807FF6h after moving bit15).

The control register is set to FFh before starting the game, purpose is unknown
(maybe enable the codes, or write-protect them, or disable the BIOS ROM; in
case that can be done by software).

The lower address bits of the newly added C0A00nh Register are:

```text
  Address bit0 - set if one or more codes use bank 7Fh..FEh
  Address bit1 - set/cleared for PAL/NTSC selection (or vice-versa NTSC/PAL?)
  Address bit2 - set to... maybe, forcing the selection in bit1 (?)
```

The exact purpose of that three bits is unknown (and their implemention in
PAR2a/b BIOSes looks bugged). Doing anything special on bank 7Fh-FEh doesn't
make any sense, maybe the programmer wanted to use banks 80h-FFh, but that
wouldn't make much more sense either; it might be something for
enabling/disabling memory mirrors or so. The default PAL/NTSC flag is
auto-detected by reading 213Fh.Bit4 (the BIOS is doing that detection twice,
with opposite results on each detection, which seems to be a bug), the flag can
be also manually changed in the BIOS menu; purpose of the PAL/NTSC thing is
unknown... maybe directing a transistor to shortcut D4 to GND/VCC when games
are reading 213Fh.

**PAR3 I/O Ports**

```text
  100000h,100001h,100002h,100003h  ;Code 0 (DTA,LSB,MID,MSB) (code 0)
  100004h,100005h,100006h,100007h  ;Code 1 (DTA,LSB,MID,MSB) (code 1)
  100008h,100009h,10000Ah,10000Bh  ;Code 2 (DTA,LSB,MID,MSB) (code 2)
  10000Ch,10000Dh,10000Eh,10000Fh  ;Code 3 (DTA,LSB,MID,MSB) (code 3)
  100010h,100011h,100012h,100013h  ;Code 4 (DTA,LSB,MID,MSB) (code 4)
  100014h,100015h,100016h,100017h  ;Code 5 (DTA,LSB,MID,MSB) (always NMI.LSB)
  100018h,100019h,10001Ah,10001Bh  ;Code 6 (DTA,LSB,MID,MSB) (always NMI.MSB)
  10001Ch         ;Control A    (bit4,6,7)
  10001Dh-10001Fh ;Set to zero  (maybe accidently, trying to init "code 7")
  10003Ch         ;Control B    (set to 01h upon game start)
  086000h         ;Control LEDs (bit0,1)
  206000h         ;Control C    (bit0)
  008000h         ;Control D    (set to 00h upon PAR-NMI entry)
```

Control A (10001Ch):

```text
  Bit0-3 Should be 0
  Bit4   ROM Mapping (0=Normal, 1=Temporarily disable BIOS & enable GAME ROM)
  Bit5   Should be 0
  Bit6-7 Select/force Video Type (0=Normal, 1=NTSC, 2=PAL, 3=Reserved)
```

Control LEDs (086000h) (LEDs are in sticker area on front of cartridge):

```text
  Bit0   Control left or right LED? (0=on or off?, 1=off or on?)
  Bit1   Control other LED          ("")
  Bit2-7 Should be 0
```

Control C (206000h):

```text
  Bit0   Whatever (0=BIOS or PAR-NMI Execution, 1=GAME Execution)
  Bit1-7 Should be 0
```

Code0-6:

Unused codes are set to 00000000h (unlike PAR1/PAR2), and, codes use linear
24bit addresses (without moving/removing bit15). Of the seven codes, code 5-6
are always used for hooking the NMI handler (even when not using any WRAM
software patches), so one can use max 5 hardware ROM patches.

<a id="snescartcheatdevicesproactionreplaymemory"></a>

### SNES Cart Cheat Devices - Pro Action Replay Memory

**PAR1-PAR3 SRAM**

All PAR versions contain 32Kbytes SRAM, divided into 8K chunks, which are
unconventionally mapped to EVEN bank numbers.

```text
  00/02/04/06:6000h..7FFFh      ;-32Kbyte SRAM (four 8K banks)
```

The SRAM is used as internal workspace (stack &amp; variables, code list, NMI
handler, deadcode handler, and list of possible-matches for the code finder).

Unknown if the SRAM is battery backed (the way how it is used by the BIOS
suggests that it is NOT battery backed).

Note: Many HiROM games have their own SRAM mapped to 6000h-7FFFh, unknown
if/how/when the PAR can disable its SRAM for compatibility with such games
(PAR1 seems to be designed for LoROM games only, but newer PAR2-PAR3
&lt;should&gt; have HiROM support - if so, then the hardware must somehow
switch the SRAM on/off depending on whether it executes game code, or PAR code
like NMI &amp; deadcode handlers).

**PAR1-PAR3 Switch**

All PAR versions do have a 3-position switch (on the right edge of the
cartridge). The way how Datel wants it to be used seems to be:

```text
  1) Boot game with switch in MIDDLE position (maybe needed only for testing)
  2) Set LOWER position & push RESET button (to enter the BIOS menu)
  3) After selecting codes/cheat finder, start game with MIDDLE position
  4) Finally, UPPER position enables codes (best in-game, AFTER intro/menu)
```

Technically, the switch seems to work like so:

```text
  UPPER Position   "Codes on"    Enable GAME and enable codes
  MIDDLE Position  "Codes off"   Enable GAME and disable codes
  LOWER Position   "Trainer On"  Enable BIOS and (maybe) enable codes
```

The "Codes off" setting may be required for booting some games (which may use
WRAM for different purposes during intro &amp; game phases). The purpose of the
"Trainer" setting is unclear, GAME/BIOS mapping could be as well done via I/O
ports (and at least PAR3 does actually have such a feature for reading the GAME
header during BIOS execution).

There seems to be no I/O ports for sensing the switch setting, however, the
"Codes off" setting can be sensed by testing if the patches (namely the patched
NMI vector) are applied to memory or not.

**PAR BIOS Versions**

```text
  Pro Action Replay Mk1 v2.1  1992        32K CRC32=81A67556h
  Pro Action Replay Mk2 v1.0  1992,93     32K CRC32=83B1D39Eh
  Pro Action Replay Mk2 v1.1  1992,93,94  32K CRC32=70D6B036h
  Pro Action Replay Mk3 v1.0U 1995       128K CRC32=0D7F770Ah
```

The two Mk2 versions are 99.9% same (v1.1 is only 10 bytes bigger than v1.0,
major change seems to be the copyright message).

Aside from v1.0/v1.1, there are reportedly further PAR2 BIOS versions (named
v2.P, v2.T, v2.H). Moreover, there's reportedly at least one localized BIOS (a
german PAR3 with unknown version number).

**PAR Component List**

The exact component list is all unknown. Some known components are:

```text
  PAR1-3  3-position switch (on right edge of the cartridge)
  PAR1-3  32Kbytes SRAM (probably not battery-backed)
  PAR1-2  32Kbytes BIOS
  PAR3    128Kbytes BIOS (with modernized GUI and built-in "PRESET" codes)
  PAR1    46pin cartridge slot (incompatible with coprocessors that use 62pins)
  PAR2-3  62pin cartridge slot
  PAR2-3? second Npin cartridge slot at rear side (for CIC from other region)
  PAR3    two LEDs (within sticker-area on front of cartridge)
  PAR1-3  whatever logic chip(s)
```

<a id="snescartcheatdevicesxterminatorgamewizard"></a>

### SNES Cart Cheat Devices - X-Terminator & Game Wizard

**Pro Action Replay (PAR1) clone**

Wide parts of the X-Terminator BIOS are copied 1:1 from a disassembled PAR1
BIOS. The similarities begin at the entrypoint (with some entirely useless
writes to 2140h-2143h), and go as far as using the ASCII characters "W H B ."
as default dummy data values for the 4 codes (the initials of the PAR1
programmer W.H.BECKETT). There are some differences to the original PAR1: The
hardware and I/O ports are a custom design, the GUI does resemble the PAR2
rather than the PAR1, and english words like "relation" are translated to odd
expressions like "differentship". Nonetheless, the thing was called back (in
some countries at least), presumably due to all too obvious copyright
violations.

**I/O Ports &amp; Memory Map**

```text
  X-Terminator 1         X-Terminator 2
  00FFE8h.W              00FFEAh.W            ;map BIOS (by writing any value)
  00FFE9h.W              00FFEBh.W            ;map GAME (by writing any value)
  00FFEAh.R (NMI read)   00FFEAh.R (NMI read) ;map BIOS/GAME (switch-selection)
  008000h-00FFFFh        008000h-00FFFFh      ;BIOS (32Kbytes)
  N/A                    028000h-02FFFFh      ;Expansion ROM 32Kbytes
  00,02,04,06:6000-7FFF  00-1F:02C00-2FFF     ;SRAM (32Kbytes)
```

Note: Both BIOS versions are confusingly using 16bit writes to the I/O ports in
some cases; the LSB-write to [addr+0] has no effect (or lasts only for 1 cpu
cycle), the MSB-write to [addr+1] is the relevant part.

The uncommon SRAM mapping in EVEN banks at 6000h-7FFFh was cloned from PAR. The
later mapping to 2C00h-2FFFh was probably invented for compatibility with HiROM
games that use 6000h-7FFFh for their own SRAM (or possibly just to look less
like a PAR clone).

Aside from NMI, the X-Terminator 2 is also using IRQ vectors (though unknown if
they are used only during BIOS execution or also during GAME execution, in
latter case reads from FFEEh would probably also trigger memory mapping).

**Game Wizard (by Innovation)**

The Game Wizard seems to be a rebadged X-Terminator. Unknown if the I/O
addresses are same as for X-Terminator 1 or 2.

**BIOS Versions**

There are at least two versions:

```text
  X-Terminator    1993 (english)  (CRC32=243C4A53h) (no built-in codes)
  X-Terminator 2  19xx (japanese) (CRC32=5F75CE9Eh) (codes for 307 games)
```

There should be probably also a separate version for Game Wizard. And,
considering that the BIOS is stored on ERPOM, there might be many further
versions &amp; revisions.

Cartridge header is FFh-filled (except for exception vectors), BIOS is 32Kbytes
LoROM.

**X-Terminator Expansion ROMs**

There have been at least 5 expansion cartridges released:

```text
  Parame ROM Cassette Vol 1-5 (by Game Tech)
```

The cartridges contain 256Kbytes LoROM, and they can be used in two ways:

As normal executable (via normal ROM header at ROM-offset 7FC0h aka SNES
address 00FFC0h), or as cheat-code database extension for the X-Terminator 2
(via a special ROM header at ROM-offset 10000h aka SNES address 028000h).

```text
  028000h - ID "FU O9149" (aka "UFO 1994" with each 2 bytes swapped)
  028008h - Boot callback (usually containing a RETF opcode)
  028010h - List of 16bit pointers (80xxh-FFFFh), terminated by 0000h
```

The 16bit pointers do address following structures (in ROM bank 02h):

```text
  2   checksum (MSB,LSB) taken from GAME cartridge ROM header [FFDCh]
  1   number of following 5-byte codes (N)
  5*N codes (MID,MSB,DTA,LSB,TYPE)  ;TYPE=predefined description (00h..23h)
```

Unknown how the cartridges are intended to be connected (between X-Terminator
and Game cartridge... or maybe to a separate expansion slot).

**Super UFO Copier**

The Super UFO copiers are somehow closely related to X-Terminator (probably
both made by the same company). X-Terminator codes are supported by various
Super UFO versions. Later Super UFO versions also include/support Parame
expansion ROMs:

```text
  Super UFO Pro-8 V8.8c BIOS
```

This versions seems to detect "FU O9149" IDs (ie. Parame carts), moreover, it
seems to include it's own "FU O9149" ID (but, strangely, at 048000h instead of
028000h, so the X-Terminator won't find it?).

**X-Terminator Chipset (whatever X-Terminator version)**

```text
  Goldstar GM76C256ALL-70 (32Kbytes SRAM, not battery-backed)
  D27256 (32Kbytes UV-Eraseable EPROM)
  two logic chips & two PALs or so (part numbers not legible on existing photo)
  3-position switch (on PCB solder-side) (SCAN/NORMAL/ACTION)
  two cartridge slots (for PAL and NTSC cartridges or so)
```

<a id="snescartcheatdevicesgamesaver"></a>

### SNES Cart Cheat Devices - Game Saver

The Game Saver from Nakitek allows to load/save snapshots of (most of) the SNES
memory and I/O ports.

**Game Saver Controls (works with joypad in port 1 only)**

```text
  L+R       upon boot --> test screen / version number
  L+R+START upon boot --> toggle slow DRAM checksumming on/off
  SELECT    in title  --> enter revival codes
  R+SELECT  in game   --> save state
  L+SELECT  in game   --> load state
  R+START   in game   --> toggle slow motion on/off  ;\one of these keeps
  L+START   in game   --> toggle slow motion on/off  ;/HDMA enabled (or so)
```

**Missing Save Data**

The SNES cannot directly access the APU, so APU RAM, DSP I/O Ports, and SPC700
registers aren't saved. The WRAM address (2181h-2183h) isn't saved. The
VRAM/OAM/CGRAM addresses are saved (but may have wrong values since the
autoincrement isn't handled). Any coprocessor I/O ports or cartridge SRAM
aren't saved.

**Hardware Versions (Game Saver and Game Saver+)**

The original Game Saver didn't have any power supply, which made (and still
makes) it the most controverse SNES add-on: Some people just like it, other
people are crying tears because they don't understand why the DRAM isn't
battery-backed.

This has led to the creation of the Game Saver Plus - a surreal product that
&lt;does&gt; use battery-backed DRAM (according to the booklet, where it is
called "portability" feature, six new AA batteries last 8-10 hours). Aside from
batteries, the Game Saver Plus is powered via the 9V DC supply of NTSC-SNES
consoles (even when the console itself is switched off). That's allowing to
switch off the SNES during supper in order to "save" energy (though after some
weeks, the permanently powered DRAM may negate that energy "saving" effect).

**BIOS Versions**

There seem to be several BIOS versions (DDMMYY formatted date and version
number are shown in the test screen). Known versions are:

```text
  Game Saver v1.3 (19xx)
  Game Saver v1.7 (31 Jul 1995)
```

Unknown if Game Saver &amp; Game Saver Plus use different BIOSes. Unknown if
any new/changed I/O ports were invented alongside with the BIOS versions.

**Game Saver Memory and I/O Map**

```text
  002100h-0021xxh PPU ports (logged at 2081xxh) (or at 2080xxh on 2nd write)
  004200h-0042xxh CPU ports (logged at 2082xxh)
  008000h-00FFFFh BIOS ROM 32Kbytes
  0080xFh         Switch to GAME mapping (upon opcodes that end at 80xFh)
  00FFEAh         Switch to BIOS mapping (upon NMI execution; when enabled)
  108000h-108001h I/O - First/second write flags for write-twice PPU ports
  108002h-108003h I/O - Exception Mode/Status (bit0-1=BRK, bit2=NMI)
  208000h-2087FFh SRAM 2Kbytes (includes auto-logged writes to PPU/CPU ports)
  400000h-73FFFFh DRAM 256Kbytes (for saving WRAM/VRAM/OAM/CGRAM, CPU/DMA regs)
  808000h-80FFFFh GAME ROM (even while BIOS mapping is enabled)
```

The Game Saver can trap BRK or NMI exceptions. Of which, BIOS v1.7 seems to use
only BRKs (which are probably generated by outputting a 00h opcode in response
to joypad access, ie. [4218h] reads and [4017h]=01h writes).

**Game Saver Revival Codes**

The 5-digit "Revival Codes" are used to improve compatibility with different
games. Most commonly used are 2xxxx codes, which cause a byte in WRAM to be
left unchanged when loading data (probably in order to keep the Main CPU aware
of the state of the APU). The Code Format is:

```text
  00000-0FFFF Blank (no action) (shown as "XXXXX" in GUI)
  10000-1FFFF Exception Mode (value for [108002]) (not used for any games)
  20000-3FFFF Preserve WRAM byte at 7E0000-7FFFFF (used for most games)
  40000-4FFFF PPU write-twice related     (used only for Starfox/Star Wing)
  50000-5FFFF PPU write-twice related     (not used for any games)
  60000-6FFFF Reserved (no action)        (not used for any games)
  70000-7FFFF Select special BRK handler  (used only for Aero the Acro Bat)
  80000-9FFFF Preserve WRAM byte at 7E0000-7FFFFF and pass it to 2140h ;\not
  A0000-BFFFF Preserve WRAM byte at 7E0000-7FFFFF and pass it to 2141h ; used
  C0000-DFFFF Preserve WRAM byte at 7E0000-7FFFFF and pass it to 2142h ; by any
  E0000-FFFFF Preserve WRAM byte at 7E0000-7FFFFF and pass it to 2143h ;/games
```

Codes (and updates) have been available as print-outs from Nakitek (the list
from 1995 contains codes for around 200 games; to be entered when pressing
SELECT in title screen). Moreover some (or all) BIOSes contain automatically
applied built-in codes (via checksumming portions of the game ROM header). The
v1.7 BIOS contains 284 codes (however, the code list does (maybe accidently)
contain an entry with NULL checksum, which causes the last 108 codes to be
ignored).

**Component List (Game Saver Plus)**

```text
  24pin SRAM (2Kbytes) (probably used only because DRAM is too slow for I/O)
  28pin ROM/EPROM (32Kbytes)
  62pin cartridge slot (on rear side of device)
  14pin eight DRAM chips (256Kbytes in total)
  Xpin huge chip (whatever logic)
  3pin 7805 or so (for turning much of the 9 volts into heat)
  2pin oscillator (20.000MHz) (for DRAM refresh generator when power-off)
  socket/cable/plug for NTSC-SNES 9V DC supply (not PAL-SNES 9V AC supply)
  battery box for six 1.5V AA batteries, battery LED, and battery switch
  resistors, capacitors, and maybe diodes, transistors
```

**Note**

Some Copiers include a similar feature, allowing to load/save "real time saves"
on floppy disks and/or temporarily in unused portions of their built-in DRAM.

<a id="snescartcheatdevicestheory"></a>

### SNES Cart Cheat Devices - Theory

**ROM Patches**

There are two possible ways for patching ROMs or ROM-images:

```text
  1) rewrite ROM-image in RAM once before game starts        (GF/FFE/emulators)
  2) patch on ROM reading (by watching address bus)          (GG and PAR2-3)
```

Both are basically having same results, there may be some variations concerning
memory mirrors (depending on how the ROM-image in RAM is mirrored, or on how
the GG/PAR2-3 do decode the ROM address).

**WRAM Patches**

Implemented by rewriting WRAM upon NMI, variations would involve mirrors:

```text
  1) allow WRAM addresses 7E0000-7FFFFF                      (PAR1-3, XT1-2)
  2) allow WRAM addresses 7E0000-7FFFFF and nn0000-nn1FFF    (N/A)
```

**SRAM Patches**

There are three possible ways for patching battery-backed SRAM:

```text
  1) rewrite once before game starts                         (GF)
  2) patch on SRAM reading (like hardware based ROM patches) (GG and PAR2-3)
  3) rewrite repeatedly on NMI execution (like WRAM patches) (N/A)
```

SRAM is usually checksummed, so SRAM patches need to be usually combined with
ROM patches which do disable the checksum verification. Some devices (like Game
Genie) rely on the /ROMSEL signal, and thus probably can only patch SRAM in the
ROM area at 70xxxxh (but not in the Expansion area at 306xxxh).

**Slow Motion Feature**

Implemented by inserting delays in Vblank NMI handler. The feature can be
usually configured in the BIOS menu, and/or controlled via joypad button
combinations from within NMI handler.

**Cheat Finders (for WRAM Patches) (eventually also for SRAM patches)**

Implemented by searching selected values from within Vblank NMI handler, or
more simple: from within BIOS RESET handler. The search can be enabled/disabled
mechanically via switch, or in some cases, via joypad button combinations. The
searched value can be configured on RESET, or in some cases, via joypad button
combinations.

**Game Saver (Nakitek)**

Allows to save a copy of WRAM/VRAM and I/O ports (but not APU memory) in DRAM,
done upon joypad button-combinations sensed within BRK/NMI exception handlers.

<a id="snescarttristarakasuper8allowstoplaynesgamesonthesnes"></a>

## SNES Cart Tri-Star (aka Super 8) (allows to play NES games on the SNES)

The Tri-Star is an adaptor for playing NES games on the SNES (similar to the
Super Gameboy which allows to play Gameboy games on SNES). The thing have three
cartridge slots (two for western/japanese NES/Famicom cartridges, and one for
SNES cartridges).

NES or SNES mode can be selected in BIOS boot menu. SNES mode does simply
disable the BIOS and jump to game entrypoint at [FFFCh]. NES mode executes the
games via a NOAC (NES-on-a-Chip, a black blob, which is also used in various
other NES clones), in this mode, the SNES video signal is disabled, and, aside
from the BIOS passing joypad data to the NES, the SNES does merely serve as
power-supply for the NES.

**Memory and I/O Map**

```text
  00E000h-00FFFFh.R - BIOS ROM (8Kbytes)
  00FFF0h.W - NES Joypad 1 (8bit data, transferred MSB first, 1=released)
  00FFF1h.W - NES Joypad 2 (bit4-5: might be NES reset and/or whatever?)
  00FFF2h.W - Enter NES Mode (switch to NES video signal or so)
  00FFF3h.W - Disable BIOS and map SNES cartridge
```

**Joypad I/O**

In NES mode, the BIOS is reading SNES joypads once per frame (via automatic
reading), and forwards the first 8bit of the SNES joypad data to the NES
(accordingly, it will work only with normal joypads, not with special hardware
like multitaps or lightguns). Like on japanese Famicoms, there are no
Start/Select buttons transferred to joypad 2. Instead, FFF1h.Bit5/4 are set to
Bit5=0/Bit4=1 in SNES mode, and to Bit5=1/Bit4=0 in NES mode (purpose is
unknown, one of the bits might control NES reset signal, the other might select
NES/SNES video signal, unless that part is controlled via FFF3h).

**Mode Selection I/O**

When starting a NES/SNES game, ports FFF2h or FFF3h are triggered by writing
twice to them (probably writing any value will work, and possibly writing only
once might work, too).

**BIOS Versions (and chksum, shown when pressing A+X on power-up)**

```text
  Tri-Star (C) 1993                        ;ROM CHKSUM: 187C
  Tri-Star Super 8 by Innovation (C) 1995  ;ROM CHKSUM: F61E
```

Both BIOSes are 8Kbytes in size (although ROM-images are often overdumped). The
versions seem to differ only by the changed copyright message. The GUI does
resemble that of the X-Terminator and Super UFO (which were probably made by
the same anonymous company).

A third version would have been the (unreleased) Superdeck (a similar device
that has been announced by Innovation and some other companies).

**Component List (Board: SFFTP_C/SFFTP_S; component/solder side)**

```text
  82pin NOAC chip (black blob on 82pin daughterboard) (on PCB bottom side)
  28pin EPROM 27C64 (8Kx8) (socketed)
  16pin SNES-CIC clone (NTSC: ST10198S) (PAL: probably ST10198P) (socketed)
  20pin sanded-chip (probably 8bit latch for joypad 1)
  20pin sanded-chip (probably 8bit latch for joypad 2)
  16pin sanded-chip (probably 8bit parallel-in shift-register for joypad 1)
  16pin sanded-chip (probably 8bit parallel-in shift-register for joypad 2)
  16pin sanded-chip (probably analog switch for SNES/NES audio or video)
  20pin sanded-chip (probably PAL for address decoding or so) (socketed)
  2pin  oscillator (? MHz) (for NES cpu-clock and/or NES color-clock or so)
  62pin cartridge edge (SNES) (on PCB bottom side)
  12pin cartridge edge (A/V MultiOut) (to TV set) (on PCB rear side)
  62pin cartridge slot (SNES)
  60pin cartridge slot (Famicom) (japanense NES)
  72pin cartridge slot (NES) (non-japanense NES)
  6pin  socket for three shielded wires (Composite & Stereo Audio in from SNES)
  TV Modulator (not installed on all boards)
  four transistors, plus some resistors & capacitors
```

<a id="snescartpiratexin1multicarts1"></a>

## SNES Cart Pirate X-in-1 Multicarts (1)

There are several X-in-1 Multicarts, all containing the same type of text based
GUI, and thus probably all made by the same company.

**Cartridge Header**

The first 4 bytes of the title string at FFC0h do usually (or always) contain
values 5C,xx,xx,80 (a "JMP FAR 80xxxxh" opcode, which jumps to the GAME
entrypoint). The next 4 title bytes are sometimes containing another JMP FAR
opcode, the rest of the header is unmodified header of the first game; except
that [FFFCh] contains the MENU entrypoint.

**ROM-images**

ROM-images found in the internet are usually incomplete dumps, containing only
the first 4MBytes (clipped to the maximum size for normal unmapped LoROM
games), or only the first game (clipped to the ROM size entry of the 1st game
header). Whilst, the actual multicarts are usually 8MBytes in size (there's one
4Mbyte cartridge, which is actually fully dumped).

**ROM Size**

Most cartridges seem to contain 8Mbyte ROMs. There is one 4MByte cartridge. And
there's one cartridge that contains an 8Kbyte EPROM (plus unknown amount of
ROM).

**LoROM/HiROM**

Most games seem to be LoROM. Eventually "Donkey Kong Land 3" is HiROM? Unknown
if HiROM banks can be also accessed (dumped) in LoROM mode, and if so, unknown
how they are ordered; with/without 32Kbyte interleave...?

**SRAM**

According to photos, most or all X-in-1 carts do not contain any SRAM. Though
some might do so?

**DSPn**

According to photos, most or all X-in-1 carts do not contain any DSP chips.
Though the "Super 11 in 1" cartridge with "Top Gear 3000" seems to require a
DSP4 clone?

**Port FFFFxxh**

```text
  A0-A3 Bank Number bit0-3 (base offset in 256Kbyte units)
  A4    Bank Number bit4 (or always one in "1997 New 7 in 1")
  A5    Always 0         (or Bank bit4 in "1997 New 7 in 1")
  A6    Varies (always 0, or always 1, or HiROM-flag in "Super 7 in 1")
  A7    Always 1 (maybe locks further access to the I/O port)
```

The bank number is somehow merged with the SNES address. As for somehow: This
may be ORed, XORed, or even ADDed - in most cases OR/XOR/ADD should give the
same result; in case of "1997 New 7 in 1" it looks as if it's XORed(?)

The special meaning of A4-A5 can be detected by sensing MOV [FFFFnn],A opcodes
(rather than normal MOV [FFFF00+x],A opcodes).

The special meaning of A6 can be detected by checking if the selected bank
contains a HiROM-header.

**Port 6FFFxxh**

Unknown. Some games write to both FFFFxxh and 6FFFxxh (using same data &amp;
address LSBs for both ports). Maybe the ROM bank address changed to 6FFFxxh on
newer boards, and FFFFxxh was kept in there for backwards compatibility. Or
maybe 6FFFxxh controls SRAM mapping instead ROM mapping?

**X-in-1 Cartridges**

```text
  Title               FFFFxx      6FFFxx    Size/Notes
  8 in 1 and 10 in 1  C0-DF       N/A       8MB (8 big games + 10 mini games?)
  1997 New 7 in 1     D0-DF,F0-FF N/A       ? MB
  Super 5 in 1        80-9F       80-9F     8MB
  Super 6 in 1        80-8F       N/A       4MB
  Super 7 in 1        80-8F,D0    80-8F,D0  8MB? (mario all stars + 3 games)
  Super 11 in 1       80-9F       N/A       8MB+DSP4 ?
```

**Chipset 7-in-1 (Board: SSF-07, REV.1)**

```text
  U1 16pin CIVIC CT6911 (CIC clone)
  U2 16pin 74LS13x or so (not legible on photo)
  U3 16pin whatever      (not legible on photo)
  U4 14pin 74LS02 or so  (not legible on photo)
  U5 black blob
  U6 black blob
```

**Chipset 8-in-1 (Board: MM32-2)**

```text
  U  20pin iCT PEEL18CV8P-25
  U  16pin 93C26 A60841.1 9312 (CIC clone)
  U  42pin 56C001 12533A-A 89315
  U  42pin 56C005-4X 12534A-A 89317
```

**Chipset 8-in-1 (Board: NES40M, 20045)**

```text
  U  16pin CIVIC 74LS13 (CIC clone)
  U  16pin not installed
  U  28pin 27C64Q EPROM (8Kx8)
  U  20pin iCT PEEL18CV8P-25
  U  42pin JM62301
  U  42pin JM62305
```

<a id="snescartpiratexin1multicarts2"></a>

## SNES Cart Pirate X-in-1 Multicarts (2)

There's at least one korean multicart (called "C20H" or "super20hab" or so),
with 20 small games stored on a relative small 1Mbyte ROM. The games are NES
games ported to work on SNES, some with typical pirate mods (like removing
copyright strings, or renaming the game to bizarre names).

**I/O Ports**

```text
  20xxh NES PPU left-overs (written to, but ignored by the SNES)
  40xxh NES APU left-overs (written to, but ignored by the SNES)
  8000h ROM Bank Size/Base
```

Port 8000h works around as so:

```text
  0-4  ROM Base Offset (in 32Kbyte units)
  5    Unknown/unused (always zero)
  6-7  ROM Bank Size (0=Used/unknown, 1=Unused/Unknown, 2=1x32K, 3=2x32K)
```

The ROM is mapped in LoROM fashion (with 1 or 2 banks of 32Kbyte).

SRAM might also exist (the photo shows some unidentified 24pin chip).

**Component List**

```text
  PCB Name: Unknown (it has one, but isn't legible on lousy photo)
  32pin C20H (1Mbyte ROM)
  24pin Unknown (maybe SRAM) (there is no battery visible on PCB front side)
  20pin Unknown (looks like a sanded chip; presumably memory mapper)
  16pin CIVIC CTxxxx? (CIC clone)
  46pin Cartridge Edge Connector
```

<a id="snescartcopiers"></a>

## SNES Cart Copiers

**Copiers**

[SNES Cart Copiers - Front Fareast (Super Magicom &amp; Super Wild Card)](#snes-cart-copiers-front-fareast-super-magicom-super-wild-card)

[SNES Cart Copiers - CCL (Supercom &amp; Pro Fighter)](#snes-cart-copiers-ccl-supercom-pro-fighter)

[SNES Cart Copiers - Bung (Game Doctor)](#snes-cart-copiers-bung-game-doctor)

[SNES Cart Copiers - Super UFO](#snes-cart-copiers-super-ufo)

[SNES Cart Copiers - Sane Ting (Super Disk Interceptor)](#snes-cart-copiers-sane-ting-super-disk-interceptor)

[SNES Cart Copiers - Gamars Copier](#snes-cart-copiers-gamars-copier)

[SNES Cart Copiers - Venus (Multi Game Hunter)](#snes-cart-copiers-venus-multi-game-hunter)

[SNES Cart Copiers - Others](#snes-cart-copiers-others)

**Misc**

[SNES Cart Copiers - Misc](#snes-cart-copiers-misc)

**Floppy Disc Controllers**

[SNES Cart Copiers - Floppy Disc Controllers](#snes-cart-copiers-floppy-disc-controllers)

[SNES Cart Copiers - Floppy Disc NEC uPD765 Commands](#snes-cart-copiers-floppy-disc-nec-upd765-commands)

[SNES Cart Copiers - Floppy Disc FAT12 Format](#snes-cart-copiers-floppy-disc-fat12-format)

**BIOSes**

[SNES Cart Copiers - BIOSes](#snes-cart-copiers-bioses)

**See also**

[SNES Cartridge ROM-Image Headers and File Extensions](60-cartridge-header-and-mapping.md#snes-cartridge-rom-image-headers-and-file-extensions)

<a id="snescartcopiersfrontfareastsupermagicomsuperwildcard"></a>

### SNES Cart Copiers - Front Fareast (Super Magicom & Super Wild Card)

**Front/CCL/Clones**

The Front Fareast I/O addresses are used by Front's own models, by early CCL
models, and by some third-party clones:

```text
  Super Magicom (Front/CCL)
  Super Wild Card (Front)
  Supercom Pro (CCL) (later CCL models use other I/O ports)
  Super Drive Pro-3 UFO (noname) (later UFO models use other I/O ports)
```

**I/O Ports (in banks 00h..7Dh and 80h..FFh)**

```text
  C000.R    FDC Flags (Bit7: MCS3201 IRQ Signal, Bit6: Drive 'Index' Signal)
              Note: Index signal is (mis-)used for Disk Insert Check
  C002.W    FDC MCS3201 Drive Control Register (motor on, etc.)
  C004.R    FDC MCS3201 Main Status Register
  C005.RW   FDC MCS3201 Command/Data Register
  C007.R    FDC MCS3201 Diagnostics Register (bit7=disk change; MCS-chip only)
  C007.W    FDC MCS3201 Density Select Register (bit0-1=Transfer rate)
  C008.R    Parallel Data Input (Reading this register reverses busy flag)
  C008.W    Parallel Data Output (bit0-3) and DRAM/SRAM mapping (bit0-1)
              Bit 0: 0=LoROM/Mode 20, 1=HiROM/Mode 21 (DRAM Mapping)
              Bit 1: 0=LoROM/Mode  1, 1=HiROM/Mode  2 (SRAM Mapping)
  C009.R    Parallel Port Busy Flag, Bit 7 (older EP1810 Version) (Altera chip)
  C000.R    Parallel Port Busy Flag, Bit 5 (newer FC9203 Version) (FRONT chip)
  C00A-C00F Unused (mirrors of C008h-C009h)
  C010-DFFF Unused (mirrors of C000h-C00Fh)
```

Below E000h-E00Dh are triggered by writing any value

```text
  E000.W    Memory Page 0  ;\Select an 8Kbyte page, CART/DRAM/SRAM address is:
  E001.W    Memory Page 1  ;    SNES address AND 1FFFh      ;lower bits
  E002.W    Memory Page 2  ;    +Selected Page * 2000h      ;upper bits
  E003.W    Memory Page 3  ;/   +SNES address AND FF0000h   ;bank number
  E004.W    Set System Mode 0 (BIOS Mode)             (with all I/O enabled)
  E005.W    Set System Mode 1 (Play Cartridge)        (with all I/O disabled)
  E006.W    Set System Mode 2 (Cartridge Emulation 1) (with E004-E007 kept on)
  E007.W    Set System Mode 3 (Cartridge Emulation 2) (with all I/O disabled)
  E008.W    Select 44256 DRAM Type  (for 2,4,6,8 Mega DRAM Card)
  E009.W    Select 441000 DRAM Type (for 8,16,24,32 Mega DRAM Card)
  E00C.W    BIOS Mode:CART at A000-BFFF, DRAM Mode:DRAM in bank 20-5F/A0-DF
  E00D.W    BIOS Mode:SRAM at A000-BFFF, DRAM Mode:CART in bank 20-5F/A0-DF
```

Later Wild Card DX models have various extra ports, eg. E0FDh, F083h, C108h.

Ports C00xh seem to be used by models up to DX and DX96.

In DX2, Ports C00xh seem to be moved to CF8xh/DF8xh.

**System Mode 0 (BIOS Mode) (selected via E004h)**

```text
  bb2000-bb3FFF RW: SRAM or CART (E00C/E00D)  bb-40-7D,C0-FF ;\8K page via
  bb8000-bb9FFF RW: DRAM                      bb-00-7D,80-FF ; E000-E003
  bbA000-bbBFFF RW: SRAM or CART (E00C/E00D)  bb=00-7D,80-FF ;/
  bbC000-bbC00x RW: I/O Ports                 bb=00-7D,80-FF
  bbE000-bbE00x W : I/O Ports                 bb=00-7D,80-FF
  bbE000-bbFFFF R : BIOS ROM (8/16/256Kbytes) bb=00-1F
```

**System Mode 1 (CART Mode) (selected via E005h)**

```text
  bb0000-bbFFFF RW: CART
```

**System Mode 2/3 (DRAM Modes) (selected via E006h/E007h)**

```text
  bb0000-bb7FFF R : DRAM Mapping, bb=40-6F, C0-DF. (HiROM/Mode 21)
  bb8000-bbFFFF R : DRAM Mapping, bb=00-6F, 80-DF. (AnyROM/Mode 20,21)
  708000-70FFFF RW: SRAM Mode 1 Mapping.         ;<-- typically for LoROM
  306000-307FFF RW: SRAM Mode 2 Mapping, Page 0. ;<-- typically for HiROM
  316000-317FFF RW: SRAM Mode 2 Mapping, Page 1. ;\extra banks for HiROM
  326000-327FFF RW: SRAM Mode 2 Mapping, Page 2. ; (do any 'real' cartridges
  336000-337FFF RW: SRAM Mode 2 Mapping, Page 3. ;/do actually have that?)
```

DRAM mapping (LoROM/HiROM), and corresponding SRAM mapping are selected via
(sharing) Bit0-1 of the Parallel Data Output (Port C008h.W)

HiROM/Mode 21:

```text
  Even DRAM Bank is mapped to bb0000-bb7FFF.
  Odd DRAM Bank is mapped to  bb8000-bbFFFF.
```

Optionally, banks 20-5F and A0-DF can be mapped to CART instead of DRAM (via
E00Dh), probably intended to allow ROM-images in DRAM to access DSP chips in
CART.

**BIOS Notes**

Observe that the BIOS is divided into 8Kbyte banks (so, the exception vectors
are at offset 1Fxxh in the ROM-image) (however, there are some overdumped
ROM-images that contain 24K padding prior to each 8K ROM-bank, ie. with LoROM
mapping style exception vectors at offset 7Fxxh). Aside from the exception
vectors, there isn't any title, nor other valid cartridge header entries.

Note: Unlike most SNES programs, Magicom &amp; Wild Card (until v1.8) BIOSes
are running in 6502 emulation mode (with E=1), rather than 65C816 mode (with
E=0).

**Parallel Port Protocol (on SNES side)**

Data is received via 8bit data register, and sent via 4bit "status" register
(which is seen as status on PC side). Strobe/busy aren't clearly documented in
official Front specs; probably, Busy gets set automatically when sensing Strobe
(from PC side), and gets cleared automatically when reading Data from Port
C008h (on SNES side).

**Parallel Port Protocol (on PC side)**

Byte Output Procedure:

```text
  Wait Busy Bit = 1           ;Status  PC Port 379h/279h/3BDh.Bit7
  Write One Byte              ;Data    PC Port 378h/278h/3BCh.Bit0-7
  Reverse Strobe Bit          ;Control PC Port 37Ah/27Ah/3BEh.Bit0
```

Byte Input Procedure:

```text
  Wait Busy Bit = 0           ;Status  PC Port 379h/279h/3BDh.Bit7
  Read Low 4 Bits of Byte     ;Status  PC Port 379h/279h/3BDh.Bit3-6
  Reverse Strobe Bit          ;Control PC Port 37Ah/27Ah/3BEh.Bit0
  Wait Busy Bit = 0           ;Status  PC Port 379h/279h/3BDh.Bit7
  Read High 4 Bits of Byte    ;Status  PC Port 379h/279h/3BDh.Bit3-6
  Reverse Strobe Bit          ;Control PC Port 37Ah/27Ah/3BEh.Bit0
```

Receiving 4bit units via status line is done for compatibility with old
one-directional PC parallel (printer) ports.

Unknown if "Wait Busy Bit = X" means to wait while-or-until Bit=X?

**Parallel Port Command Format**

Commands are 9-bytes in length, sent from PC side.

```text
  00h 3   ID (D5h,AAh,96h)
  03h 1   Command Code (00h-01h, or 04h-06h)
  04h 2   Address (LSB,MSB)
  06h 2   Length (LSB,MSB)
  08h 1   Checksum (81h XORed by Bytes 03h..07h)
  Followed by <Length> bytes of data (upload/download commands only)
```

Commands can be:

```text
  Command 00h : Download Data (using page:address,length)   ;to-or-from PC?
  Command 01h : Upload Data (using page:address,length)     ;from-or-to PC?
  Command 04h : Force SFC Program to JMP (to address... plus page/bank?)
  Command 05h : Select 8Kbyte Memory Page Number (using address)
  Command 06h : Sub Function (address: 0=InitialDevice: 1=ExecDRAM, 2=ExecCART)
```

InitialDevice does probably reset the BIOS? ExecDRAM allows to run the uploaded
ROM-image, but unknow how to select LoROM/HiROM mode, or is it automatically
done by examining the uploaded cartridge header. The usage of the 16bit address
isn't quite clear: The lower 13bit are somehow combined with the 8Kbyte page
number, the upper 3bit might be used to select DRAM/SRAM?

**Super Magicom V3H - BIOS upgrade**

This ROM-image is a Magicom BIOS upgrade, and it's a pain in the ass:

The upgrade works ONLY as ROM-image (ie. must be loaded to DRAM), and does NOT
work as real ROM (ie. cannot be burned to EPROM), the reason is that it doesn't
include a character set (and uses that from the original Magicom BIOS at
E000h).

The upgrade isn't compatible with the parallel port (the original BIOS
relocates parallel port code from ROM to WRAM, and the upgrade relocates itself
from DRAM to WRAM - but still expects the parallel port code in the same WRAM
location and crashes when Busy-bit gets set).

The upgrade exists as 32Kbyte or 32.5Kbyte ROM-image (an 8K upgrade, 24K
garbage with entrypoint at end of garbage, plus 0.5K extra garbage), emulators
and other tools will be typically interprete the 32.5K file as 32Kbyte ROM with
512-byte header (which is NOT correct in that special case).

The upgrade exists in two variants: One using the standard Front Fareast I/O
addresses, one using the I/O addresses at C000h-C00Fh re-ordered as so:

```text
  8000h-FFFFh RW DRAM-mode: DRAM (containing the Magicom V3H upgrade)
  E000h-FFFFh R  BIOS-mode: BIOS (containing the Character set)
  C000h       W  DRAM bank mapped to 8000h? (set to 00,20,40 upon DRAM detect)
  C001h       W  Memory Control?
  C002h       -  Unused
  C003h       R  Parallel Port Busy (bit7) (when set: crashes the V3H upgrade)
  C004h-C008h -  Unused
  C009h       R  Status (bit7=ready?,bit5=busy/timeout?)
  C00Ah       -  FDC Unused
  C00Bh       W  FDC Motor Control (set to 00h,29h,2Dh)
  C00Ch       RW FDC Command/Data
  C00Dh       R  FDC Main Status
  C00Eh       W  FDC Transfer Rate? (set to 00h,01h,02h or so)
  C00Fh       -  FDC Unused
  E004h       W  Map BIOS ROM (instead V3H upgrade) ;\
  E006h-E007h W  Something on/off                   ; seems to be same/similar
  E008h-E009h W  Something on/off                   ; as Front-like I/O ports
  E00Ch-E00Dh W  Something on/off                   ;/
```

Unknown which hardware uses that re-ordered addresses. Note: The V3H version
with re-ordered I/O addresses does also contain different 24K garbage (sorts of
as if it were created from original source code, rather than just patched?).

**Component List - Super Magicom Plus**

```text
  U1  24pin  DRAM (onboard)
  U2  20pin  SN74LS245N (8-bit 3-state transceiver)
  U3  24pin  DRAM (onboard)
  U4  24pin  DRAM (onboard)
  U5  24pin  DRAM (onboard)
  U6  28pin  27C128-25 EPROM (16Kx8)
  U7  68pin  MCCS3201FN (=MCS3201FN without double-C) (disc controller)
  U8  100pin ?
  U9  28pin  HM62256 (SRAM 32Kx8)
  U10 20pin  ?
  U11 14pin  ?  (does not exist in later versions?)
  U12 20pin  AMI 16CVB8PC-25
  U13 20pin  AMI 16CVB8PC-25
  U14 16pin  ST10198S (newer version only) (mounted on top of U10 in old ver)
  BT1 2pin   ?
  J1  25pin  DB-25 parallel port
  J2? 25pin  DB-25 external floppy (not installed)
  J3  40pin  DRAM expansion board
  J4  34pin  internal floppy (flat cable)
  J5  26pin  (not installed)
  J6  4pin   floppy power supply
  J7  62pin  cartridge edge
  J8  62pin  cartridge slot
  J9  12pin  jumpers (I:I:I: or :I:I:I) (enable internal or external CIC)
  Y1  2pin   24.000 MHz
```

**Component List - Super Wild Card DX (AH-558001-02 Made in Japan 94.8.23)**

```text
  U1  100pin CPU      FRONT FC9203 HG62E22926F9 (or so) (SMD)
  U2   28pin S-RAM    NEC D43256AC-10L (uPD43256AC) (SRAM, 32Kx8)
  U3   20pin          SN74LS245 (to parallel port) (8-bit 3-state transceiver)
  U4    ?pin PAL-2    L GAL20V84 25LP
  U5   20pin          SN74LS...? (or so)
  U6   32pin BIOS-ROM BIOS
  U7   16pin U7       SN74LS139AN (decoder/demultiplexer)
  U8   14pin U8       SN74LS125AN (quad 3-state buffer)
  U9   44pin          GoldStar GM82C765B (SMD) (floppy disc controller)
  U10  16pin DECODER  SNC4011 (or so)
  U11  20pin PAL-3    iCT PEEL17CV8P CTN24053
  U12?  3pin 7805H    voltage regulator
  U13  20pin PAL-1    AMI
  X1    2pin 16MHZ    16.000 MHz
  CN?   2pin AC/DC-IN power supply input
  CN?   4pin ..POW    power supply to internal disc drive (only 2pin connected)
  CN2  62pin          female cartridge slot
  CN3? 46pin RAM-SLOT to DRAM daughterboard (only 40pin used on remote side)
  CN5  25pin PC-I/F   DB-25 parallel port
  CN6  34pin FDD-I/F  cable to internal disc drive
  CN01 34pin          goes to one 1st of male 62pin cartridge edge
  CN02 34pin          goes to one 2nd of male 62pin cartridge edge
  SW1   3pin RESET-SW reset switch/button or so, for whatever purpose
  DB1   4pin          AC-DC converter
  BT1   2pin          3V battery
  J1   12pin          jumpers (near cartridge slot)
  J2   20pin          jumpers (near cartridge slot)
  J3    2pin          jumper (near power-input)
  J4    2pin          jumper (near power-input)
  J5    2pin          jumper (near power-input)
  DRAM Daughterboard:
  U1,U2,U7,U8   16pin  ST T74LS139B (decoder/demultiplexer) (four pieces)
  U3-U6,U9-U10  28pin  NEC D424900G5 (or so) (six pieces) (SMD)
  U11-U12       28pin  M5M44800ATP           (two pieces) (SMD)
```

**Component List - Supercom Pro (SP3200) (dated around 1992)**

(probably uses Front-like I/O)

```text
  U1  20pin  SN74LS245N (to parallel port) (8-bit 3-state transceiver)
  U2  16pin  HD74LS174 (to parallel port?)
  U4  68pin  MCCS3201FN (=MCS3201FN without double-C) (floppy disc controller)
  U?  68pin  Altera EP1810LC-45 D9219
  U?  28pin  EPROM
  U?  28pin  SRAM Winbond W24256-10L 9149
  U7  20pin  SN74LS245N (8-bit 3-state transceiver)
  U8  16pin  not installed
  U?  20pin  modded (?) chip (soldered on cart-edge connector at bottom side)
  J1  25pin  DB-25 parallel port
  J2  25pin  DB-25 external floppy disc connector
  J3  40pin  to DRAM daughterboard
  J4  34pin  not installed (internal floppy disc connector)
  J5  62pin  cartridge edge
  J6  62pin  cartridge slot
  Y1  2pin   24.000 MHz
  BT1 2pin   VARTA Ni/Cd, 3.6V 60mA, 14h 6mA (recharge-able & acid-leaking)
```

**Component List - SMD800 Super Magic Drive (requires SNES-to-Genesis adaptor)**

```text
  U1  20pin  SN74LS245N (to parallel port?) (8-bit 3-state transceiver)
  U2  16pin  SN74LS174 (to parallel port?)
  U3  20pin  SN74HC245P (8-bit 3-state transceiver)
  U4  20pin  SN74HC245P (8-bit 3-state transceiver)
  U5? 68pin  MCS3201FN (floppy disk controller)
  U6  20pin  SN74HC245P (8-bit 3-state transceiver)
  U   28pin  27C64A-15 (EPROM, 8Kx8) (with Genesis Z80 code, non-SNES code)
  U   28pin  HY62256ALP-10 (SRAM, 32Kx8)
  U     pin  Altera EP1810LC-45
  U10 16pin  MC74HC157 (decoder/demultiplexer)
  U11 16pin  MC74HC157 (decoder/demultiplexer)
  U12 14pin  xxxx
  J   25pin  DB-25 parallel port
  J2  25pin  DB-25 external floppy
  J3  40pin  internal floppy (not installed) (likely only 34pins of 40pin used)
  J   64pin  cartridge edge (genesis)
  J   64pin  cartridge slot (genesis)
  J   40pin  to DRAM daughterboard
  Y   2pin   oscillator
  BT  2pin   VARTA Ni/Cd, 3.6V 60mA, 14h 6mA (recharge-able & acid-leaking)
 DRAM Daughterboard:
  U1  20pin  HY514400J-70 (DRAM)
  U2  20pin  HY514400J-70 (DRAM)
  U3  20pin  HY514400J-70 (DRAM)
  U4  20pin  HY514400J-70 (DRAM)
  U5  14pin  74LS08 (quad 2-input AND gates)
  U6  16pin  HD74LS157P (decoder/demultiplexer)
  U7  14pin  74LS08 (quad 2-input AND gates)
  CN1 40pin  connector to mainboard
 Super Magicom-Drive (SNES-to-Genesis adaptor for above):
  xxx        components unknown
```

<a id="snescartcopierscclsupercomprofighter"></a>

### SNES Cart Copiers - CCL (Supercom & Pro Fighter)

Below is for Supercom Partner &amp; Pro Fighter models from CCL (China Coach
Limited). See the Front Fareast chapter for their earlier Super Magicom models
(which were produced by Front &amp; CCL), and also for Supercom Pro 2 (which
was made by CCL alone, but still used the Front-like I/O ports).

**Pro Fighter 1993 by H.K. / Supercom Partner A**

Ports are somewhat based on the Front design (BIOS is expanded from 8K at
E000h-FFFFh to 16K at C000h-FFFFh, accordingly FDC Ports C000h-C007h are moved
to 2800h-2807h, and Parallel Port Ports C008h-C009h are simply removed. Ports
at E00xh are somehow changed, but might be still similar to the Front design
(?)

```text
  2800.R   FDC MCS General Purpose Input (bit7,bit6 used)
  2802.W   FDC MCS Motor Control (set to 00h,29h,2Dh)
  2804.R   FDC MCS Main Status
  2805.RW  FDC MCS Command/Data Status
  2807.W   FDC MCS Transfer Rate/Density (set to 0..3)
  Below 2808-2810 only in newer "Pro Fighter Q"
   2808.R   Parallel Port Data (bit0-7)
   2809.W   Parallel Port Data (4bit or 8bit?)
   2810.R   Parallel Port Busy (bit5)
    (there seem to be 4bit & 8bit parallel port modes supported, one of them
    also WRITING to 2808h, and in some cases reading "FDC" register 2800 looks
    also parallel port DATA and/or BUSY related)
  Again changed for Double Pro Fighter
   2803.R   Parallel Port Busy (bit7)
   2808.R   Parallel Port Data (bit0-7)
   2809.W   Parallel Port Data (4bit or 8bit?)
   2804 =FDC DATA   ;\swapped ! (unlike older "non-double" models)
   2805 =FDC STAT   ;/
   280x =other ports in this region may be changed, too ?
   004800    ROM (from offset 8800-9FFF) (contains program code)
   014800    ROM (from offset A800-BFFF) (contains character set)
   E00x
   E800+x
   Note: Having BIOS portions mapped to the fast 3.58MHz region at 4800h-5FFFh
         was probably done unintentionally; this would require 120ns EPROMs,
         whilst some Double Pro Fighter boards are fitted with 200ns EPROMs
         (which are stable at 2.68MHz only, and may cause crashes, or charset
         glitches in this case)
   Double Pro Fighter BIOS is 64Kbytes:
     0000-3FFF  Genesis/Z80 BIOS
     4000-7FFF  Same content as 0000-3FFF
     8000-87FF  Unused (zerofilled)
     8800-9FFF  SNES BIOS (6K mapped to 004800-005FFF)
     A000-A7FF  Unused (zerofilled)
     A800-BFFF  SNES BIOS (6K mapped to 014800-015FFF)
     C000-FFFF  SNES BIOS (16K mapped to 00C000-00FFFF)
  7000.R
  A000.RW               ;7000-related
  C000-FFFF.R BIOS ROM (16Kbytes)
  E002.W   set to 00h   ;7000-related
  E003.W   set to BFh ;then compares BFFD with BFFC,BFFA,BFFB,BFEA,BFEB
  E00C.W   set to 00h   ;7000-related
  E00E.W   set to E0h
  008000.RW   DRAM detection?
  208000.RW   DRAM detection?
  408000.RW   DRAM detection?
  608000.RW   DRAM detection?
```

<a id="snescartcopiersbunggamedoctor"></a>

### SNES Cart Copiers - Bung (Game Doctor)

Game Doctor SF7

**Memory Map (in BIOS mode)**

```text
  00:8000-807F     I/O Ports
  00:8080-FFFF     BIOS ROM (1st 32kBytes)
  01:8000-FFFF     BIOS ROM (2nd 32kBytes) (if any)
  02:8000-FFFF     unused
  03:8000-FFFF     unused
  04:8000-FFFF     SRAM for game positions (32Kbyte)
  05:8000-FFFF     SRAM for real time save data (4kByte)
  06:8000-FFFF     SRAM for copier settings (4kByte)
  07:8000-FFFF     DRAM for ROM-image (32Kbyte page, selected via Port 8030h)
  08-7D:8000-FFFF  Mirror of above banks 00-07
  80-FF:8000-FFFF  Mirror of above banks 00-07 or Cartridge banks 00-7F/80-FF
```

```text
  FFBFh compared to FFh ?
```

**I/O Ports (in BIOS mode) in bank 00h**

```text
  8000h-800Fh RW 512Kbyte DRAM chunk, mapped to upper 32Kbyte of Bank 0xh-Fxh
  8010h-8013h RW 512Kbyte DRAM chunk, mapped to lower 32Kbyte of Bank 4xh-7xh
  8014h-8017h RW 512Kbyte DRAM chunk, mapped to lower 32Kbyte of Bank Cxh-Fxh
  8018h-8019h W  SRAM Flags (bit0-15=Enable SRAM at 6000-7000 in banks 0xh-Fxh)
  8018h       R  bit1 = realtime.$4016.bit0, read bit7 = ? , bit = ?
  8018h.R  Flags (bit7/6 FDC IRQ?, and more)
  8019h       R  bit1 = ?
  801Ah       R  realtime.word, latch settings for double write word registers
  801Ah       W  write ?
  801Bh       W  write ?
  801Dh       W  BIOS mode mapping: changes what is mapped into banks $80-$FF
                   only bit0-bit1 seem to matter
                   0 = use cartridge banks $00-$7F
                   1 = use cartridge banks $80-$FF
                   2 = mirror banks $00-$7F (BIOS regs and all?)
                   3 = mirror banks $00-$7F (BIOS regs and all?)
  801Eh write ?
  __Floppy Disc__
  8020h       R  FDC Main Status
  8021h       RW FDC Command/Data
  8022h       W  FDC Transfer Rate/Density (?) (set to 00h,01h)
  8023h       -  FDC Unused
  8024h       W  FDC Motor Control (set to 00h,08h,0Ch,1Ch,2Dh)
  8025h-8027h -  FDC Unused
  8028h       W  set to same value (ANY VALUE?) as 8022/8029)
  8029h       W  set to same value (ANY VALUE?) as 8029)
  802Ah       W  set to 01-then-00 (once) (thereafter do sth to 8022)
  802Bh       W  set to 01h during FDC COMMAND-BYTEs (else to 00h) (maybe LED?)
  __Parallel Port__
  802Ch       RW Parallel Port Data Lines
  802Dh       RW Parallel Port Status Lines
  802Eh       RW Parallel Port Control Lines
  802Fh       W  Parallel Port? Unknown (set to 00h,01h) (data direction?)
  802Fh       R  Parallel Port? Unused (reads same as $00802D)
  __Memory__
  8030h-8031h W  Select 32Kbyte-DRAM-Page (0000h..01FFh) mapped to 078000h
  8030h-803Dh R  this is a 7 word table?? (gotten from code at 80/AE80)
  8040h-805Fh R  read same as 802Dh (uh, but, some are used for sth else?)
  8040h       R  used, parallel port related (or other mainboard version?)
  8043h       W  used, parallel port related (or other mainboard version?)
  8060h-807Fh R  read = FFh
  80xFh          any access to 0080xFh (x=8..F) switches to cartridge mode
```

**802Dh - Parallel Port status (not direct pin reading?)**

```text
 read
  bit0 = /C1 (direct pin14, /AutoLF) (/Ctrl.Bit1 on PC side)
  bit1 = C2  (direct pin16, /INIT)   (Ctrl.Bit2 on PC side)
  bit2 = /C3 (direct pin17, /Select) (/Ctrl.Bit3 on PC side)
  bit3 = "write bit3"
  bit4 = "write bit4"
  bit5 = "write bit4" (uh, not bit5 here?)
  bit6 = "write bit4" (uh, not bit6 here?)
  bit7 = /S7 (direct pin11) = "write bit7" AND not "write bit0"
 write
  bit0 Enable/Disable Busy bit (0=Enable, 1=Disable) (?)
  bit3 => S3 (direct pin15, /ERR) (Stat.bit3 on PC side)
  bit4 => S4 (direct pin13, SLCT) (Stat.bit4 on PC side)
  bit5 => S5 (direct pin12, PE)   (Stat.bit5 on PC side)
  bit6 => S6 (direct pin10, /ACK) (Stat.bit6 on PC side)
  bit7 ...   (direct pin11, BUSY  (/Stat.bit7 on PC side) (ANDed with /bit0?)
```

**802Eh - Parallel Port control (not direct control reg values)**

```text
 often write 12h-then-10h
 read/write?
  bit0 = /C1 (direct pin14, /AutoLF) (/Ctrl.Bit1 on PC side)   W
  bit1 = C2  (direct pin16, /INIT)   (Ctrl.Bit2 on PC side)    W
  bit2 = /C3 (direct pin17, /Select) (/Ctrl.Bit3 on PC side)   W
  bit3-bit6, read = bit3-bit6 of $00802D                       W
  bit7 = /C0 (direct pin1,  /STB) (/Ctrl.bit0 on PC side)      R
```

**Component List - Bung Game Doctor SF6 (Board CT401)**

```text
  U    3pin 7805 or so
  U   40pin GoldStar xxx (=probably GM82C765B) (floppy disc controller)
  U  ???pin huge chip (200 pins or so)
  U   18pin 265111
  U   20pin 74LS744 or so (not installed)
  U   28pin SRAM or so
  U   28pin SRAM or so
  U   28pin EPROM (GDSF_6.0)
  U   14pin whatever/modded chip (wired top-down near EPROM)
  P   40pin to DRAM daughterboard 1 (2x10 male pins, 2x10 female pins)
  P   40pin to DRAM daughterboard 2 (2x10 male pins, 2x10 female pins)
  P   62pin cartridge port
  P   62pin cartridge port (on PCB back side)
  P   25pin DB-25 parallel port (on PCB back side)
  P    2pin power supply (on PCB back side)
  P    2pin floppy supply (on PCB back side)
  P   34pin floppy data
  X    2pin oscillator
```

<a id="snescartcopierssuperufo"></a>

### SNES Cart Copiers - Super UFO

UFO Super Drive Pro / Super UFO

**UFO3**

The UFO-3 is a Front Fareast clone. BIOS is 8Kbytes mapped to E000h-FFFFh, FDC
Registers are at C000h-C007h, Parallel Port at C008h-C009h, Memory Control at
E000h-E00Dh. For details see Front Fareast chapter.

**UFO6**

I/O ports for this version are unknown.

**UFO7/UFO8**

```text
  2184.W   ... set to 00h/0Ch/0Fh
  2185.W   ... set to 00h/0Fh
  2186.W   ... set to 00h/0Fh
  2187.W   ... set to 08h/00h/0Bh
  2188.W   ... set to 00h..0Fh or so
  2189.W   ... set to 0Fh/0Eh
  218A.W   ... set to 00h
  218B.W   ... set to 0Ah/0Fh
  218C.R   FDC Main Status Register
  218D.RW  FDC Command/Data Register (emit 03h,DFh,03h = spd/dma)(then 07h,01h)
  218E.W   FDC Motor Control (set to 00h, 29h-then-2Dh on disc access)
  218F.W   FDC Transfer Rate
  218F.R   FDC Flags (bit7=irq?,bit6=index?) (UFO8: bit5=?)
  003F68.R         warmboot flag? if A581 --> JMP 3D00
  003FD0..3FFF   cartridge header? (or copy of it?)
  003C00..003FFF SRAM 1Kbyte (BIOS settings, I/O logging?, last 32-byte OAM)
  013C00..013FFF SRAM 1Kbyte (512-byte Palette and 1st 512-byte OAM)
  008000h and up BIOS 64Kbytes (UFO7) or more 128K..256K (UFO8)
  708000h and up SRAM 32Kbytes (for game positions)
  808000h and up DRAM (variable size detected) (via calls to 9025)
```

ufo7 rom chksum calculated at 9505 (32K ROM at 8000-FFFF must sum up to 00h)

ufo8 (and maybe ufo7 too) should have 8K SRAM (ie. MORE than above 2x1K...?)

**UFO6 Component List - UFO Super Drive Pro (with Pro-6 BIOS)**

```text
  U    3pin 7805 or so
  U    ?pin xxxx (near 7805)
  U   40pin GoldStar GM82C765B
  U   14pin xxxx
  U   20pin L GALxxxx
  U   20pin L GALxxxx
  U   20pin L GALxxxx
  U   20pin AMI xxxxx
  U   20pin L GALxxxx
  U   20pin L GALxxxx
  U   20pin L GALxxxx
  U   20pin L GALxxxx
  U   20pin LS245 (not installed, near DB-25) (8-bit 3-state transceiver)
  U   14pin HC74  (not installed, near DB-25) (dual flip-flop)
  U   16pin xxxx  (installed, near DB-25)
  U   28pin EPROM
  U   28pin Winbond W24256-10L (SRAM 32Kx8)
  U   20pin Philips PC74xxxx
  U   16pin 74LS112 (reportedly a cloned/mislabelled CIC chip)
  X    2pin oscillator (near 7805)
  BT   2pin 3V or so
  P   25pin DB-25 parallel port or so
  P    2pin power supply
  P    2pin floppy supply
  P   34pin floppy data
  P   40pin to DRAM daughterboard
  P   46pin cartridge slot (only 46 soldering points)
  P   62pin cartridge edge (has 62 soldering points, but only 46 connected?)
```

**UFO8 Component List Super UFO Super Drive PRO8 (REV 7.8 2)**

```text
  1x 84pin Altera EPMxxxxxxx84-15
  1x 40pin GoldStar GM82C765B (DIL) (floppy disc controller)
  1x 32pin BIOS ROM/EPROM (located on PCB solder side)
  1x 28pin UM62256D-70L (SRAM 32Kx8)
  1x 28pin UT6264PC-70LL (SRAM 8Kx8)
  1x 28pin DSP chip (not installed)
  2x 24pin NN5117405BJ-60 (DRAM, two pieces, located on daughterboard)
  1x 16pin D1
  1x 14pin FT4066
  1x 14pin DSP_74HC74 (not installed) (dual flip-flop)
  1x 14pin 74LS00 or so
  1x 14pin whatever (near oscillator)
  1x 14pin 74LSxxx whatever (near PAL/NTSC jumpers)
  2x 16pin SN74HC157N (decoder/demultiplexer)
  1x 3pin  7805 or so
  1x 34pin connector/cable to internal disc drive
  2x 62pin cartridge connectors (one male, one female)
  1x 2pin  wire (supply to internal disc drive)
  1x 2pin  connector (external power supply input)
  no battery, no parallel port
```

<a id="snescartcopierssanetingsuperdiskinterceptor"></a>

### SNES Cart Copiers - Sane Ting (Super Disk Interceptor)

The Super Disk Interceptor is a SNES copier from KL818 B.C./Sane Ting Co. Ltd.,
the company also made a copier for Mega Drive, called Mega Disk Interceptor.

**I/O &amp; Memory**

```text
  8000-9FFF memory (SRAM/DRAM or so)
  A000.W  set to 00,03-then-01, or 40,80
  A001.W  set to 00,04 (as MSB of A000) or to 04,24,08
  A001.R  tests bit4,bit5
  A002.W  FDC Transfer Rate/Density (set to ([0B] XOR 1)*2)
  A003.W  FDC Motor Control (set to 08,0C,1C)
  A004    FDC Unused
  A005.RW FDC Command/Data
  A006.R  FDC Main Status
  A007    FDC Unused
  A008.W  set to [1802] (bit3,bit4 used)
  B000-B01F ...    I/O or RAM or RegisterFile workspace?
   B000.W  set to 00
   B000.R  checked if 00h
   B001.W  set to 00
   B002.W  set to xx OR 80h
   B002.R  read and ORed with 06h
   B003.W  set to xx OR 80h OR 03h
   B003.R  bit5 isolated, ORed with 04h, then written to A001h
   B004.W  set to 00 or 00..03h
   B004.R  whatever, if (N+1)=00..03 --> written to B004 and B005
   B005.W  set to 00
   B006.RW set to [4219h] = MSB of joypad1 (?)
   B00F.W  set to [FFDC]=00h or ([FFDC] XOR 1)=01h
   B00F.R  checked if 00h (if nonzero --> WRITE PROTECT)
   B01x    ...
  C000.R  dummy read within waitvblank
  C001.R  dummy read within waitvblank
  E000.W
  E001.W
  E002.W
  E000-FFFF BIOS (32Kbytes, in 8Kbyte units, in banks 00h-03h)
  704000
  708000
```

**Component List - Super Disk Interceptor (version dated around 1992)**

```text
  U1  40pin  GoldStar GM82C765B PL (DIP) (floppy disk controller)
  U2  84pin  MD1812 9211 (with socket)
  U3  28pin  2xxx4A-25 (PROM, presumably 8Kx8, non-eraseable)
  U4  20pin  not installed (DIP) (BANK2.3)
  U4A 20pin  not installed (DIP) (BANK2.3)
  U5  20pin  GoldStar GMxxxxx (SMD) (BANK0.1) (DRAM)
  U5A 20pin  GoldStar GMxxxxx (SMD) (BANK0.1) (DRAM)
  U6  28pin  Hyundai HY62256ALP-10 (SRAM, 32Kx8)
  U7  20pin  not installed (DIP) (BANK2.3)
  U7A 20pin  not installed (DIP) (BANK2.3)
  U8  20pin  GoldStar GMxxxxx (SMD) (BANK0.1) (DRAM)
  U8A 20pin  GoldStar GMxxxxx (SMD) (BANK0.1) (DRAM)
  U9  16pin  74HC157N (decoder/demultiplexer)
  U10 16pin  74HC157N (decoder/demultiplexer)
  U11 20pin  HY-xxxxxx-30
  U12 20pin  HY-xxxxxx-30
  XTAL 2pin  16.000MHz
  J   46pin  Cartridge edge (snes)
  J   46pin  Cartridge slot (snes)
  J   34pin  Floppy data
  J    2pin  Floppy supply
  BT  2pin   3.6V Battery
```

**Component List - Super Disk Interceptor (version dated around 1993)**

```text
  U   44pin  GoldStar GM82C765B PL (SMD) (floppy disk controller)
  U   28pin  27C64SDM (PROM, 8Kx8, non-eraseable)
  U   28pin  GoldStar GM76C256ALLFW70 (SRAM, 32Kx8)
  U   20pin  HD74HC373P (8-bit 3-state transparent latch)
  U   14pin  xxxx
  U   80pin  SD1812 349 (SMD, without socket)
  U   14pin  not installed
  U   28pin  KM48C2100J-7 (DRAM, 2Mx8)  ;\
  U   20pin  KM44C1000CJ-6 (DRAM, 1Mx4) ; all installed,
  U   20pin  KM44C1000CJ-6 (DRAM, 1Mx4) ; together = 4Mx8
  U   20pin  KM44C1000CJ-6 (DRAM, 1Mx4) ;
  U   20pin  KM44C1000CJ-6 (DRAM, 1Mx4) ;/
  X    2pin  16.000MHz
  X    2pin  16.257MHz
  J   46pin  Cartridge edge (snes)
  J   46pin  Cartridge slot (snes)
  J   34pin  not installed (alternate floppy connector?)
  J   34pin  Floppy data
  J    2pin  Floppy supply
  BT  2pin   3.6V Battery
```

<a id="snescartcopiersgamarscopier"></a>

### SNES Cart Copiers - Gamars Copier

Known as:

```text
  ALMA Super Disk F-16
  Gamars Super Disk FC-301
  FR-402 Super Disk (bundled with "FR-402 Super 16bit" SNES clone)
```

```text
  2K SRAM at 005000 with REQUIRED mirror at 005800
            3F5Fxx.W  set to FFh,FFh,FFh...
            3F5FC0.R  FDC stat  (bit7,bit5)
            3F5FD2.W  FDC motor? (set to 0Ch,1Ch,08h,0Ch)
            3F5FE4.R  FDC Main Status
            3F5FED.RW FDC Command/Data (emit 03,DF,03)
```

**Gamars Puzzle**

Aside from the Gamars BIOSes, there's a mis-named ROM-image in the internet:
"Gamars (Copier BIOS)", this file is made by the same company, but it's a
Puzzle game, not a copier BIOS.

<a id="snescartcopiersvenusmultigamehunter"></a>

### SNES Cart Copiers - Venus (Multi Game Hunter)

MGH (Multi Game Hunter) from Venus.

The 32Kbyte BIOS contains both SNES/65C816 code (entrypoint at [FFFCh]) and
Genesis/Z80 code (entrypoint at 0000h).

```text
  006000..007FFF -- RAM or so
  035800..035807 -- I/O Ports
  ---
  006400.R          id "SFCJ"
  007D00..007EFF.R  checksummed
  035800.W    set to C0h
  035801.W    set to A0h
  035802.W    set to 0000h or 06h
  035803.W    set to 04h
  035804.R    disk status?  (bit7,bit6)
  035805.W    disk command? (set to 0Bh) (not a uPD765 command?)
  035806.W    set to 00h or ([AAh] ROR 1)
  035807.W    set to 00h or [ABh]
```

**FDC Accress via 80C51 CPU**

Like many other copiers, the MGH does use a "normal" MCS3201FN controller, but,
it does indirectly access it through a 80C51 CPU. For example,

```text
    05       cmd (write sec)                                 ;\
    [0B6B]   track          ;\less parameters as than        ; write sector
    [0BA4]   head           ; directly accessing a uPD765    ; command
    [0BA2]   sector         ;/                               ; (at PC=CBAFh)
    [[18]+y] data... (200h bytes)                            ;/
```

<a id="snescartcopiersothers"></a>

### SNES Cart Copiers - Others

**Component List - Board "GP-003 REV. B" (used in Special Partner)**

```text
  U   14pin  GD74HC04 (hex inverters)
  U   16pin  LR74HC158 (decoder/demultiplexer)
  U   16pin  LR74HC158 (decoder/demultiplexer)
  U   16pin  GD74HC138 (decoder/demultiplexer)
  U   16pin  GD74HC138 (decoder/demultiplexer)
  U   20pin  PALCE16V8H-25
  U   28pin  K-105                                               ;DSP clone?
  U   28pin  EPROM (28pin 27C512 64Kx8 installed, optionally 32pin possible)
  U   28pin  NEC D43256BGU-70L (uPD43256BGU) (SRAM 32Kx8)
  U   28pin  NEC 4364C-20L (SRAM 8Kx8)
  U   44pin  GoldStar GM82C765B PL (SMD)
  U   44pin  Lattice ispLSI 1016-60LJ B501B06 (SMD)
  U    3pin  7805 or so
  J2  62pin  to cartridge edge (snes)
  J   62pin  cartridge slot (snes game cartridge)
  J   62pin  cartridge slot (an expansion slot, not for any game carts)
  J   40pin  to DRAM daughterboard
  J   34pin  to internal floppy drive
  J2   2pin  floppy power supply
  J    2pin  external power supply
  J6   2pin  jumper (near 34pin floppy cable)
  J8   3pin  jumper (near EPROM; maybe ROM size select?)
  X    2pin  oscillator (160)
  X    2pin  oscillator (?)                                      ;DSP clock?
  BT   2pin  NiCd 3.6V
```

The board doesn't contain a CIC-clone (unless it's 'hidden' in one of the
chips).

**Components - Supercom, 24m DSP, CD-ROM, FX-32, High Density, Real Time Save**

```text
  U1  40pin GoldStar GM82C765B (floppy disc controller)
  U2  20pin 16V8 (not installed)
  U3? 20pin PALCE16V8H-25PC/4
  U4  24pin PALCE20V8H-25PC/4
  U5  28pin not installed (probably for DSP clone)
  U6? 14pin xxx (below U5)
  U7? 20pin LS245 (not installed) (8-bit 3-state transceiver)
  U8? 24pin PALCE20V8H-25PC/4
  U9  20pin 74HC273 (8bit latch with reset)
  U10 28pin 27C256G-20 (EPROM 32Kx8) (boots as "FX-32 CD-ROM & DSP, 1994 H.K.")
  U11 28pin ST MK4864 (SRAM 8Kx8)
  U12 28pin xxx (SRAM ?Kx8)
  U13 20pin xxx
  U14 20pin xxx
  U15 20pin xxx
  U16 20pin xxx
  U17 20pin xxx
  U18 20pin xxx
  U19  ?pin Toshiba xxx (16pin chip, mounted in a 20pin socket)
  U20 16pin ST101xxx
  Q1   3pin 7805
  Y1   2pin oscillator
  Y2   2pin oscillator (not installed, probably for DSP chip)
  J?  34pin floppy data
  J?   2pin floppy power
  J?   2pin power supply
  J3? 25pin DB-25 (parallel port and/or external CD-ROM drive?)
  J4  62pin cartridge edge
  J5  62pin cartridge slot
  J6  40pin to DRAM daughterboard
```

**Component List - Double Pro Fighter (CCL) (1994)**

```text
  U1  28pin  Hyundai HY62256ALP-10 (SRAM 32Kx8)
  U2  28pin  AM27C512-205DC (EPROM 64Kx8)
  U3  N/A    N/A
  U4  N/A    N/A
  U5  20pin  HD74LS245P (8-bit 3-state transceiver)
  U6  20pin  HD74LS245P (8-bit 3-state transceiver)
  U7  20pin  HD74LS245P (8-bit 3-state transceiver)
  U8  24pin  GoldStar GM76C28A-10 (SRAM 2Kx8)
  U9  16pin  noname-chip-without-part-number (or, marked 10198 on other boards)
  U10  3pin  AN7805 (voltage regulator)
  U11 14pin  HD74HC00P (quad 2-input NAND gates)
  U12 40pin  Goldstar GM82C765B (floppy disc controller)
  U13 68pin  Altera EP1810LC-45 D9407
  U14 16pin  74HC139 (decoder/demultiplexer)
  U15 24pin  PALCE20V8H
  U16 20pin  GAL16V8xxx
  U17 20pin  PALCxxx
  U18 16pin  74HC139 (decoder/demultiplexer)
  Y1   2pin  16.00 TDX (16 MHz oscillator)
  J1   2pin  power supply input
  J2   2pin  power supply connector (alternately to J1 or so, not installed)
  P4  50pin  ro dram daughterboard ?
  SL1  64pin connector for remove-able snes-or-sega? cartridge edge
  SL2  64pin connector for remove-able sega-or-snes? cartridge edge
  SL3  62pin cartridge slot (snes)
  SL4  64pin cartridge slot (sega genesis)
  ?     2pin connector for disc drive (supply)
  ?    34pin connector for disc drive (data)
  DRAM Daughterboard
  -    40pin connector (to 40pins of the 50pin socket on Double Pro Fighter)
  -    20pin NEC 424400-80 (EIGHT pieces)
  Optional Parallel Port (plugged into SL3-socket, ie. into SNES slot):
  U1   20pin PALCE16V8H-25PC/4
  U2   20pin HD74HC245P (8-bit 3-state transceiver) (no latch here ???)
  P1   25pin DB-25 parallel port connector
  -    62pin cartridge edge (to be plugged into SL3 of Double Pro Fighter)
```

**Component List - Super Smart Disc (same as Pro Fighter X?)**

```text
  U   16pin 10198
  U   28pin GRAPHIC DSP1-1  (or is it "DCP1-1" or so?)
  U   28pin STxxxx (SRAM, ?x8)
  U   28pin xxxxxx (SRAM, ?x8)
  U   28pin EPROM (28pin chip mounted in 32pin socket)
  U   14pin xxxx
  U   40pin ICT PA7140T CTM42027JC
  U   40pin ICT PA7140T CTM42027JC
  U   40pin GoldStar GM82C765B
  U   24pin xxxxx (PAL or so)
  U    3pin 7805 or so
  X    2pin oscillator
  P1  64pin cartridge edge (via remove-able adaptor) (snes)
  P   62pin cartridge slot (snes)
  P   32pin cartridge slot (gameboy)
  P   34pin floppy data
  P    2pin power supply
  P    2pin floppy supply
  P   50pin to DRAM daughterboard
```

<a id="snescartcopiersmisc"></a>

### SNES Cart Copiers - Misc

**Parallel Ports (DB-25)**

Parallel Ports are used to upload/download data from PCs. Later copiers seem to
be additionally using the Parallel Port for connecting CD-ROM drives.

Some copiers have fully working parallel ports installed (eg. Front Fareast),
some have them incompletely installed (eg. some Supercom seem to require
additional 74LS245 (8-bit 3-state transceiver), and a specially programmed
PAL16V8 chip?).

Other copiers don't have any provisions for parallel ports onboard - but can be
eventually upgraded externally by plugging a parallel port cartridge into the
SNES cartridge slot: There are at least two such upgrade cartridges (one
contains pure logic, the other one additionally contains a BIOS upgrade).

**DSP Chips**

Some copiers include DSP-clones onboard (or do at least have sockets or
soldering points for mounting DSP chips), other copiers can be upgraded
externally: By plugging a DSP cartridge into the SNES cartridge slot (either a
regular game cartridge with DSP chip, or a plain DSP-clone-cartridge without
any game in it).

Of course, this will work only with the correct DSP chip (DSP1 for most games;
unless there are any DSP clones that support more than one DSP chip at one?).
Another problem may be I/O addresses (different games expect DSP chips at
different addresses).

**Batteries**

Some boards contain batteries for the internal SRAM. Either 3V Lithium cells
(coin-shaped), or rechargeable 3.6V NiCd batteries (usually with blue coating,
which tend to leak acid, and to destroy wires on the PCB). Other boards don't
have any batteries at all (they are said to use capacitors instead of
batteries, which might be nonsense, or might last only a few minutes?) (there
seems to be no way to switch-off the external power-supply, so batteries aren't
needed to power SRAM) (eventually some boards might even power the DRAM in
standby mode (?), which would require a DRAM refresh generator). And, aside
from battery-backup, most or all copiers are allowing to save SRAM to floppy.

<a id="snescartcopiersfloppydisccontrollers"></a>

### SNES Cart Copiers - Floppy Disc Controllers

**FDC Chips**

Most (or all) SNES copiers are using one of the following FDCs:

```text
  40pin  GM82C765B (DIP)  (Supercom, Ufo, Pro Fighter, Smart Disc, Bung?)
  44pin  GM82C765B (SMD)  (Wild Card, GP-003)
  68pin  MCS3201FN (SMD)  (used by OLD copiers: Super Magic Drive)
  68pin  MCCS3201FN (SMD) (used by OLD copiers: Supercom & Super Magicom)
```

**FDC Address Decoding**

The 68pin MCS3201FN chips include a 10bit address bus (for decoding address
3F0h-3F7h on IBM PCs; whereas, SNES copiers are using only the lower some
address bits), and an 8bit General Purpose Input. The 40pin/44pin GM82C765B
chips include a 1bit address bus bundled with 3 select lines, and have no
General Purpose Input register.

```text
  GM82C765B   MCS3201FN  Dir  Register
  N/A         A0-A2=0    R    General Purpose Input (pins I0..I7)
  /LDOR       A0-A2=2    W    Motor Control (bit0-7)
  /CS+A0=0    A0-A2=4    R    Main Status  (NEC uPD765 compatible)
  /CS+A0=1    A0-A2=5    RW   Command/Data (NEC uPD765 compatible)
  /LDCR       A0-A2=7    W    Transfer Rate (Density) (bit0-1)
  N/A         A0-A2=7    R    Bit7=DiskChange, Bit6-0=Zero
```

Accordingly, MCS3201FN ports are always ordered as shown above, whilst
GM82C765B ports can be arranged differently (in case of the Super Wild Card,
Front Fareast kept them arranged the same way as on their older Super Magicom).

**FDC Command/Data and Main Status**

[SNES Cart Copiers - Floppy Disc NEC uPD765 Commands](#snes-cart-copiers-floppy-disc-nec-upd765-commands)

**FDC Motor Control**

```text
  GM Bit0-7:  DSEL ,X    ,/RES,DMAEN,MOTOR1,MOTOR2,X     ,MSEL
  MCS Bit0-7: DSEL0,DSEL1,/RES,DMAEN,MOTOR1,MOTOR2,MOTOR3,MOTOR4
```

Note that, for whatever reason, most SNES Copiers are using the SECOND drive
(ie. DSEL=1 instead of DSEL=0, and MOTOR2=1 instead of MOTOR1=1).

**Transfer Rate (Density)**

```text
  Val  Usage                 MCS3201FN       GM82C765B
  00h  HD (high density)     500K if /RWC=1  MFM:500K or FM:250K
  01h  DD 5.25" (double den) 300K if /RWC=0  MFM:300K if DRV=1, 250K if DRV=0
  02h  DD 3.5"(double den)   250K if /RWC=0  MFM:250K or FM:125K
  03h  N/A                   Reserved        125K
```

**Disk Change (MCS3201FN only) (not GM82C765B)**

```text
  7   Disk Change Flag
  6-0 Unused (zero)
```

Possibly useful, but purpose/usage is unclear. According to the datasheet it is
for "diagnostics" purposes. Unknown when the flag gets reset, and unknown for
which drive(s) it does apply.

<a id="snescartcopiersfloppydiscnecupd765commands"></a>

### SNES Cart Copiers - Floppy Disc NEC uPD765 Commands

**Accessing the FDC 765**

The Data Register is used to write Commands and Parameters, to read/write data
bytes, and to receive result bytes. These three operations are called Command-,
Execution-, and Result-Phase. The Main Status Register signalizes when the FDC
is ready to send/receive the next byte through the Data Register.

**Command Phase**

A command consists of a command byte (eventually including the MF, MK, SK
bits), and up to eight parameter bytes.

**Execution Phase**

During this phase, the actual data is transferred (if any). Usually that are
the data bytes for the read/written sector(s), except for the Format Track
Command, in that case four bytes for each sector are transferred.

**Result Phase**

Returns up to seven result bytes (depending on the command) that are containing
status information. The Recalibrate and Seek Track commands do not return
result bytes directly, instead the program must wait until the Main Status
Register signalizes that the command has been completed, and then it must (!)
send a Sense Interrupt State command to 'terminate' the Seek/Recalibrate
command.

**FDC Command Table**

```text
 Command     Parameters              Exm Result               Description
 02+MF+SK    HU TR HD ?? SZ NM GP SL <R> S0 S1 S2 TR HD NM SZ read track
 03          XX YY                    -                       specify spd/dma
 04          HU                       -  S3                   sense drive state
 05+MT+MF    HU TR HD SC SZ LS GP SL <W> S0 S1 S2 TR HD LS SZ write sector(s)
 06+MT+MF+SK HU TR HD SC SZ LS GP SL <R> S0 S1 S2 TR HD LS SZ read sector(s)
 07          HU                       -                       recalib.seek TP=0
 08          -                        -  S0 TP                sense int.state
 09+MT+MF    HU TR HD SC SZ LS GP SL <W> S0 S1 S2 TR HD LS SZ wr deleted sec(s)
 0A+MF       HU                       -  S0 S1 S2 TR HD LS SZ read ID
 0C+MT+MF+SK HU TR HD SC SZ LS GP SL <R> S0 S1 S2 TR HD LS SZ rd deleted sec(s)
 0D+MF       HU SZ NM GP FB          <W> S0 S1 S2 TR HD LS SZ format track
 0F          HU TP                    -                       seek track n
 11+MT+MF+SK HU TR HD SC SZ LS GP SL <W> S0 S1 S2 TR HD LS SZ scan equal
 19+MT+MF+SK HU TR HD SC SZ LS GP SL <W> S0 S1 S2 TR HD LS SZ scan low or equal
 1D+MT+MF+SK HU TR HD SC SZ LS GP SL <W> S0 S1 S2 TR HD LS SZ scan high or eq.
```

Parameter bits that can be specified in some Command Bytes are:

```text
  MT  Bit7  Multi Track (continue multi-sector-function on other head)
  MF  Bit6  MFM-Mode-Bit (Default 1=Double Density)
  SK  Bit5  Skip-Bit (set if secs with deleted DAM shall be skipped)
```

Parameter/Result bytes are:

```text
  HU  b0,1=Unit/Drive Number, b2=Physical Head Number, other bits zero
  TP  Physical Track Number
  TR  Track-ID (usually same value as TP)
  HD  Head-ID
  SC  First Sector-ID (sector you want to read)
  SZ  Sector Size (80h shl n) (default=02h for 200h bytes)
  LS  Last Sector-ID (should be same as SC when reading a single sector)
  GP  Gap (default=2Ah except command 0D: default=52h)
  SL  Sectorlen if SZ=0 (default=FFh)
  Sn  Status Register 0..3
  FB  Fillbyte (for the sector data areas) (default=E5h)
  NM  Number of Sectors (default=09h)
  XX  b0..3=headunload n*32ms (8" only), b4..7=steprate (16-n)*2ms
  YY  b0=DMA_disable, b1-7=headload n*4ms (8" only)
```

Format Track: output TR,HD,SC,SZ for each sector during execution phase

Read Track: reads NM sectors (starting with first sec past index hole)

Read ID: read ID bytes for current sec, repeated/undelayed read lists all IDs

Recalib: walks up to 77 tracks, 80tr-drives may need second recalib if failed

Seek/Recalib: All read/write commands will be disabled until succesful senseint

Senseint: Set's IC if unsuccesful (no int has occured) (until IC=0)

**FDC Status Registers**

The Main Status register can be always read through an I/O Port. The other four
Status Registers cannot be read directly, instead they are returned through the
data register as result bytes in response to specific commands.

**Main Status Register (I/O Port)**

```text
  b0..3  DB  FDD0..3 Busy (seek/recalib active, until succesful sense intstat)
  b4     CB  FDC Busy (still in command-, execution- or result-phase)
  b5     EXM Execution Mode (still in execution-phase, non_DMA_only)
  b6     DIO Data Input/Output (0=CPU->FDC, 1=FDC->CPU) (see b7)
  b7     RQM Request For Master (1=ready for next byte) (see b6 for direction)
```

**Status Register 0**

```text
  b0,1   US  Unit Select (driveno during interrupt)
  b2     HD  Head Address (head during interrupt)
  b3     NR  Not Ready (drive not ready or non-existing 2nd head selected)
  b4     EC  Equipment Check (drive failure or recalibrate failed (retry))
  b5     SE  Seek End (Set if seek-command completed)
  b6,7   IC  Interrupt Code (0=OK, 1=aborted:readfail/OK if EN, 2=unknown cmd
             or senseint with no int occured, 3=aborted:disc removed etc.)
```

**Status Register 1**

```text
  b0     MA  Missing Address Mark (Sector_ID or DAM not found)
  b1     NW  Not Writeable (tried to write/format disc with wprot_tab=on)
  b2     ND  No Data (Sector_ID not found, CRC fail in ID_field)
  b3,6   0   Not used
  b4     OR  Over Run (CPU too slow in execution-phase (ca. 26us/Byte))
  b5     DE  Data Error (CRC-fail in ID- or Data-Field)
  b7     EN  End of Track (set past most read/write commands) (see IC)
```

**Status Register 2**

```text
  b0     MD  Missing Address Mark in Data Field (DAM not found)
  b1     BC  Bad Cylinder (read/programmed track-ID different and read-ID = FF)
  b2     SN  Scan Not Satisfied (no fitting sector found)
  b3     SH  Scan Equal Hit (equal)
  b4     WC  Wrong Cylinder (read/programmed track-ID different) (see b1)
  b5     DD  Data Error in Data Field (CRC-fail in data-field)
  b6     CM  Control Mark (read/scan command found sector with deleted DAM)
  b7     0   Not Used
```

**Status Register 3**

```text
  b0,1   US  Unit Select (pin 28,29 of FDC)
  b2     HD  Head Address (pin 27 of FDC)
  b3     TS  Two Side (0=yes, 1=no (!))   GM82C765: Also WP (same as bit6)?
  b4     T0  Track 0 (on track 0 we are)
  b5     RY  Ready (drive ready signal)   GM82C765: Always 1=Ready
  b6     WP  Write Protected (write protected)
  b7     FT  Fault (if supported: 1=Drive failure) GM82C765: Always 0=Okay
```

**Notes:**

Before accessing a disk you should first Recalibrate the drive, that'll move
the head backwards until it reaches Track 0 (that's required to initialize the
FDCs track counter). On a 80 track drive you may need to repeat that in case
that the first recalibration attempt wasn't successful (that's because the FDC
stops searching after 77 steps) (at least older uPD765 chips did so, maybe the
MCS3201FN/GM82C765B chips don't).

Now if you want to format, read or write a sector on a specific track you must
first Seek that track (command 0Fh). That'll move the read/write head to the
physical track number. If you don't do that, then the FDC will attempt to
read/write data to/from the current physical track, independendly of the
specified logical Track-ID.

The Track-, Sector-, and Head-IDs are logical IDs only. These logical IDs are
defined when formatting the disk, and aren't required to be identical to the
physical Track, Sector, or Head numbers. However, when reading or writing a
sector you must specify the same IDs that have been used during formatting.

Despite of the confusing name, a sector with a "Deleted Data Address Mark"
(DAM) is not deleted. The DAM-flag is just another ID-bit, and (if that ID-bit
is specified correctly in the command) it can be read/written like normal data
sectors.

**DMA/IRQ**

Most (or all) SNES copiers don't support DMA or IRQs (some are allowing to poll
the IRQ flag by software I/O).

**Terminal Count (TC)**

*** Below info applies to Amstrad CPC with uPD765 chip.

*** Unknown if anything similar applies to SNES with MCS3201FN/GM82C765B chips.

At the end of a successful read/write command, the program should send a
Terminal Count (TC) signal to the FDC. However, in the CPC the TC pin isn't
connected to the I/O bus, making it impossible for the program to confirm a
correct operation. For that reason, the FDC will assume that the command has
failed, and it'll return both Bit 6 in Status Register 0 and Bit 7 in Status
Register 1 set. The program should ignore this errormessage.

<a id="snescartcopiersfloppydiscfat12format"></a>

### SNES Cart Copiers - Floppy Disc FAT12 Format

The SNES Copier floppy format is compatible to that used under DOS on PCs.

Typical formats are 3.5", Double Density, 80 Tracks/9 Sectors, Double Sided
(720KB). The Sectors are logically numbered 01h..09h, and each sized 200h
bytes.

XXX HD-disks have more sectors

XXX snes copiers are usually HD (or maybe some are DD?)

XXX snes copiers support 1.44MB and 1.6MB (FDFORMAT-like)

**Boot-Record**

The first sector is always used as bootsector, giving information about the
usage of the following sectors, and including the boot procedure (for loading
MSDOS etc).

```text
  00-02       80x86 boot procedure (jmp opcode) (not used for SNES)
  03-0A       ascii disk name
  0B-0C       bytes / sector
  0D          sectors / cluster
  0E-0F       sectors / boot-record
  10          number of FAT-copys
  11-12       entrys / root-directory
  13-14       sectors / disk
  15          ID: F8=hdd, F9=3.5", FC=SS/9sec, FD=DS9, FE=SS8,FF=DS8
  16-17       sectors / FAT
  18-19       sectors / track
  1A-1B       heads / disk
  1C-1D       number of reserved sectors
  1E-1FF      MSX boot procedure (Z80 code) (not used for SNES)
```

**FAT and FAT copy(s)**

The following sectors are occupied by the File Allocation Table (FAT), which
contains 12- or 16-bit entries for each cluster:

```text
  (0)000      unused, free
  (0)001      ???
  (0)002...   pointer to next cluster in chain (0)002..(F)FEF
  (F)FF0-6    reserved (no part of chain, not free)
  (F)FF7      defect cluster, don't use
  (F)FF8-F    last cluster of chain
```

Number and size of FATs can be calculated by the information in the boot
sector.

**Root directory**

The following sectors are the Root directory, again, size depends on the info
in bootsector. Each entry consists of 32 bytes:

```text
  00-07       Filename (first byte: 00=free entry,2E=dir, E5=deleted entry)
  08-0A       Filename extension
  0B          Fileattribute
  0C-15       reserved
  16-17       Timestamp: HHHHHMMM, MMMSSSSS
  18-19       Datestamp: YYYYYYYM, MMMDDDDD
  1A-1B       Pointer to first cluster of file
  1C-1F       Filesize in bytes
```

The 'cluster' entry points to the first used cluster of the file. The FAT entry
for that cluster points to the next used cluster (if any), the FAT entry for
that cluster points to the next cluster, and so on.

**Reserved Sectors (if any)**

Usually the number of reserved sectors is zero. If it is non-zero, then the
following sector(s) are reserved (and could be used by the boot procedure for
whatever purposes).

**Data Clusters 0002..nnnn**

Finally all following sectors are data clusters. The first cluster is called
cluster number (0)002, followed by number (0)003, (0)004, and so on.

**Special Features**

Unknown if any copiers support sub-directories.

Unknown if any copiers support long file names.

Unknown if any copiers support compressed files (ZIP or such).

<a id="snescartcopiersbioses"></a>

### SNES Cart Copiers - BIOSes

**Copier BIOSes**

```text
  Name                                    I/O   BIOS Size
  Double Pro Fighter (1994)               2800  64K(6+6+16)
  Gamars Puzzle (not a Copier BIOS)       -     1M (32x32K) GAMARS~5 1,048,576
  Gamars Super Disk FC-301 V6.0  Kaiser94 5Fxx  64K (1x64K) GAMARS~4    65,536
  Gamars Super Disk FC-301 V7.13 Kaiser94 5Fxx 256K (4x64K) GAMARS~3   262,144
  Gamars Super Disk FC-301 V7.16 Kaiser94 5Fxx 256K (4x64K) GAMARS~2   262,144
  Game Doctor SF 3 V3.3C                  8000  32K (1x32K) GAMEDO~3    32,768
  Game Doctor SF 6 V6.2  (Professor SF)   8000  64K (2x32K) GAMEDO~4    65,536
  Game Doctor SF 6 V6.21 (Professor SF)   8000  64K (2x32K) GAMEDO~6    65,536
  Game Doctor SF 7 V7.11 (Professor SF 2) 8000  64K (2x32K) GAMEDO~2    65,536
  Multi Game Hunter V1.2 (Venus)          5800  32K (1x32K) MULTIG~2    32,768
  Multi Game Hunter V1.3 (Venus)          5800  32K (1x32K) MULTIG~3    32,768
  Multi Game Hunter V1.4 (Venus)          5800  32K (1x32K) MULTIG~3    32,768
  Pro Fighter Q (H.K.)           xx-xx-93 2800  16K (1x16K) SUPERP~1    16,xxx
  Supercom Partner A   [o1]               2800  16K (1x16K) SUPERC~1 3,145,728
  Supercom Pro 2 (CCL) ports=FFE 06-21-92 FFE    8K (1x8K)  SUPERC~2    32,768
  Super Disk Interceptor v5.2 (Sane Ting) A000  32K (4x8K)  SUPERD~1    32,768
  Super Magicom V1H (Front/CCL)  12-23-91 FFE    8K (1x8K)  SUPERM~2    32,768
  Super Magicom V31 (Front/CCL)  xx-xx-92 FFE    8K (1x8K)  SUPERM~4    32,768
  Super Magicom V3H SoftUpgrade  xx-xx-9x FFE   32K (DRAM)  SUPERM~8    32,768
  Super Pro Fighter (H.K.)       xx-xx-93 2800  16K (1x16K) SUPERP~1    16,xxx
  Super Pro Fighter (H.K.) [a1]  xx-xx-93 2800  16K (1x16K) SUPERP~1    16,xxx
  Super Wild Card V1.6   (Front) 93-01-26 FFE   16K (2x8K)  SUPERW~6    16,384
  Super Wild Card V1.8   (Front) 93-02-19 FFE   16K (2x8K)  SUPERW~7    16,384
  Super Wild Card V2.0XL (Front) 93-04-12 FFE   16K (2x8K)  SUPERW~9    16,384
  Super Wild Card V2.1B  (Front) 93-04-28 FFE   16K (2x8K)  SUPER~10    16,384
  Super Wild Card V2.1C  (Front) 93-04-28 FFE   16K (2x8K)  SUPER~11    16,384
  Super Wild Card V2.2CC (Front) 93-05-03 FFE   16K (2x8K)  SUPER~12    16,384
  Super Wild Card V2.6CC (Front) 93-07-17 FFE   16K (2x8K)  SUPER~15    16,384
  Super Wild Card V2.6F  (Front) 93-07-17 FFE   16K (2x8K)  SUPER~16    16,384
  Super Wild Card V2.6FX (Front) 93-07-17 FFE   16K (2x8K)  SUPER~17    16,384
  Super Wild Card V2.7CC (Front) 93-12-07 FFE   16K (2x8K)  SUPER~18    16,384
  Super Wild Card V2.8CC (Front) 06-08-94 FFE   16K (2x8K)  SUPER~19    16,384
  Super Wild Card V2.8CC   [o1]  06-28-94 FFE   16K (2x8K)  SUPER~22    65,536
  Super Wild Card DX             10-14-94 FFE  256K (32x8K) SUPERW~2   262,144
  Super Wild Card DX             11-03-94 FFE  256K (32x8K) SUPERW~3   262,144
  Super Wild Card DX96           01-04-96 FFE  256K (32x8K) SUPERW~1   262,144
  Super Wild Card DX2            06-08-96 FFE  256K (32x8K) SUPERW~4   262,144
  UFO - Super Drive PRO 3  [o1 as 4x8K]   FFE    8K (1x8K)  UFOSUP~1    32,768
  UFO - Pro 6                             ?       ? (?)
  UFO - Super UFO Pro-7 V7.3     1994     2184  64K (2x32K) SUPERU~1    65,536
  UFO - Super UFO Pro-8 V8.1     1995     2184 128K (4x32K) SUPERU~2   131,072
  UFO - Super UFO Pro-8 V8.8c    1995     2184 256K (8x32K) SUPERU~3   262,144
```

<a id="snescartcdromdrive"></a>

## SNES Cart CDROM Drive

**SNES Sony CDROM (unreleased)**

Nintendo and Sony originally planned a partnership where Sony would produce a
CDROM drive add-on for the SNES, additionally Sony would have produced a SNES
compatible console with the CDROM drive built-in. The project progressed far
enough to produce some prototype, and publish some press releases.

However, the deal failed, and Sony finally produced their own console (the Sony
Playstation). Anyways, the unreleased SNES CD Prototype worked as so:

[SNES Cart CDROM - Memory and I/O Map](#snes-cart-cdrom-memory-and-io-map)

[SNES Cart CDROM - CDROM Bootsector and Volume Descriptor](#snes-cart-cdrom-cdrom-bootsector-and-volume-descriptor)

[SNES Cart CDROM - BIOS Cartridge](#snes-cart-cdrom-bios-cartridge)

[SNES Cart CDROM - BIOS Functions](#snes-cart-cdrom-bios-functions)

[SNES Cart CDROM - Mechacon](#snes-cart-cdrom-mechacon)

[SNES Cart CDROM - Decoder/FIFO](#snes-cart-cdrom-decoderfifo)

[SNES Cart CDROM - Component List](#snes-cart-cdrom-component-list)

For general info about CDROM discs, see the documentation for my PSX debugger:

```text
  http://problemkaputt.de/psx.htm - no$psx homepage
  http://problemkaputt.de/psx-spx.htm - psx specifications
```

That docs are covering about everything about sector headers, sector encoding,
subchannels, tocs, tracks, sectors, frames, sessions, volume descriptors,
filesystem, xa-adpcm, cd-da audio, plus specs for different cdrom-image file
formats.

**SNES Philips CDROM (unreleased)**

After the deal with Sony had failed, Nintendo tried a new deal with Philips -
which failed, too.

**SNES Copier CDROM (released)**

Whilst Nintendo failed on producing an official CDROM drive, some SNES Copiers
are allowing to load ROM-images from CDROMs.

<a id="snescartcdrommemoryandiomap"></a>

## SNES Cart CDROM - Memory and I/O Map

**I/O Ports**

```text
  21D0h.W   - BIOS Cartridge Battery RAM Lock (write 00h)
  21E0h.W   - BIOS Cartridge Battery RAM Unlock Step 2 (write 0Fh downto 01h)
  21E1h.R/W - CDROM Unit Mechacon CPU (probably the NEC chip on daughterboard)
  21E2h.R/W - CDROM Unit Decoder/FIFO Index (CXD1800Q chip)
  21E3h.R/W - CDROM Unit Decoder/FIFO Data  (CXD1800Q chip)
  21E4h.W   - CDROM Unit (?) Whatever Control/Enable or so
  21E5h.W   - BIOS Cartridge Battery RAM Unlock Step 1 (write FFh)
  ???.R/W   - NEXT connector? (maybe some kind of UART, like PSX serial port?)
  ???.R/W   - BIOS Cartridge S-WRAM chip(s) (seem be wired to /PARD and /PAWR)
  IRQ       - used for Decoder and Mechacon
```

**APU I/O Ports**

The SNES CD prototype has APU chips with uncommon part numbers, which might
work slightly different than standard SNES APUs. However, adding that chips
wouldn't be possible with SNES CD expansions (for existing SNES consoles).
Either old SNES consoles would need to stick with old APUs, or, theoretically,
the SNES CD expansions could contain an extra APU unit (but, mapped elsewhere
than 2140h-2143h).

**Memory**

```text
  00h-03h:8000h-FFFFh  BIOS Cart ROM (128Kbyte LoROM)
  80h-87h:8000h-FFFFh  BIOS Cart Work RAM (256Kbyte DRAM) (two S-WRAM chips)
  90h    :8000h-9FFFh  BIOS Cart Battery RAM (8Kbyte SRAM)
```

Special Memory regions/addresses:

```text
  00h:1Fxxh  Work RAM reserved for BIOS functions
  00h:1FF8h  Work RAM containing NMI vector (should be 4-byte "JMP far" opcode)
  00h:1FFCh  Work RAM containing IRQ vector (should be 4-byte "JMP far" opcode)
  00h:0000h  Work RAM containing IRQ/BRK/COP vectors (if used)
  00h:1000h  Load address for 800h-byte boot sector
  00h:1080h  Entrypoint for 800h-byte boot sector
  00h:E000h  CD BIOS Functions in BIOS ROM
  83h:C000h  Work RAM reserved for loading cdrom data in "VRAM mode" (16Kbyte)
```

Caution: Initial/empty SRAM may NOT be zerofilled (else the BIOS treats the
checksum to be okay, with 0 files installed - but with 0000h bytes free space,
which is making it impossible to create/delete any files).

Caution: RAM at 1Fxxh is reserved for BIOS functions (and NMI/IRQ vectors, even
when not using any other BIOS functions), so stacktop should be 1EFFh (not
1FFFh, where it'd be usually located).

Unknown if the memory is mirrored anywhere; particulary mirroring the S-WRAMs
to C0h-C3h:0000h-FFFFh would be useful for HiROM-style games.

Unknown if the two S-WRAM chips are also mapped to B-bus (the B-bus would be
useful only for DMA from ROM carts, ie. not useful for CDROM games).

**21E4h.W - Whatever Control/Enable or so**

```text
  7-4   Unknown/Unused (always set to 0)
  3     Enable Mechacon?      (0=Off, 1=On)
  2     Enable Decoder?       (0=Off, 1=On)
  1     Maybe Reset?          (0=Normal, 1=What?)
  0     Unknown/Unused (always set to 0)
```

Set to 0Eh,00h,04h,08h,0Ch.

**Decoder/FIFO Registers (CXD1800Q) (accessed via 21E2h/21E3h)**

```text
 Decoder Write Registers
  00h     -           Reserved
  01h     DRVIF       DRIVE Interface (W)
  02h     CHPCTL      Chip Control (W)
  03h     DECCTL      Decoder Control (W)
  04h     INTMSK      Interrupt Mask (0=Disable, 1=Enable) (W)      ;\interrupt
  05h     INTCLR      Interrupt Clear/Ack (0=No change, 1=Clear/ack);/
  06h     CI          ADPCM Coding Information (to be used when AUTOCI=0)
  07h     DMAADRC_L   SRAM-to-CPU Xfer Address, Low (W)               ;\
  08h     DMAADRC_H   SRAM-to-CPU Xfer Address, High (W)              ;
  09h     DMAXFRC_L   SRAM-to-CPU Xfer Length, Low (W)                ;
  0Ah     DMAXFRC_H   SRAM-to-CPU Xfer Length, High & DMA Control (W) ;/
  0Bh     DRVADRC_L   Disc-to-SRAM Xfer Address, Low (W)              ;\
  0Ch     DRVADRC_H   Disc-to-SRAM Xfer Address, High (W)             ;/
  0Dh-0Fh -           Unspecified
  0Dh     "PLBA"      <-- shown as so in SNES CD's "CXD1800" test screen
  10h-1Ch -           Mirrors of 00h-0Ch
  1Dh     -           Reserved (TEST2)
  1Eh     -           Reserved (TEST1)
  1Fh     -           Reserved (TEST0)
 Decoder Read Registers
  00h     DMADATA     SRAM-to-CPU Xfer Data (R)             ;-Sector Data
  01h     INTSTS      Interrupt Status (0=No IRQ, 1=IRQ) (R);-Interrupt
  02h     STS         Status (R)                            ;\
  03h     HDRFLG      Header Flags (R)                      ;
  04h     HDR_MIN     Header "MM" Minute (R)                ; important info on
  05h     HDR_SEC     Header "SS" Second (R)                ; current sector
  06h     HDR_BLOCK   Header "FF" Frame (R)                 ; (to be handled
  07h     HDR_MODE    Header Mode (R)                       ; upon "DECINT"
  08h     SHDR_FILE   Sub-Header File (R)                   ; interrupt)
  09h     SHDR_CH     Sub-Header Channel (R)                ;
  0Ah     SHDR_S-MODE Sub-Header SubMode (R)                ;
  0Bh     SHDR_CI     Sub-Header Coding Info (R)            ;
  0Ch     CMADR_L     Current Minute Address, Low (R)       ;
  0Dh     CMADR_H     Current Minute Address, High (R)      ;/
  0Eh     MDFM        MODE/FORM (R)                         ;\extra details on
  0Fh     ADPCI       ADPCM Coding Information (R)          ;/current sector
  10h-to-2            Reserved (TEST 0 to 2) (R)
  13h     -           Unspecified
  14h-17h -           Mirrors of 04h-07h (HDR_xxx)
  18h.R   DMAXFRC_L - SRAM-to-CPU Xfer Length, Low (R)      ;\allows to read
  19h.R   DMAXFRC_H - SRAM-to-CPU Xfer Length, High (R)     ; address/remain
  1Ah.R   DMAADRC_L - SRAM-to-CPU Xfer Address, Low (R)     ; values
  1Bh.R   DMAADRC_H - SRAM-to-CPU Xfer Address, High (R)    ; (needed only for
  1Ch.R   DRVADRC_L - Disc-to-SRAM Xfer Address, Low (R)    ; diagnostics)
  1Dh.R   DRVADRC_H - Disc-to-SRAM Xfer Address, High (R)   ;/
  1Eh-1Fh -           Mirrors of 0Eh-0Fh (MDFM and ADPCI)
```

<a id="snescartcdromcdrombootsectorandvolumedescriptor"></a>

## SNES Cart CDROM - CDROM Bootsector and Volume Descriptor

SNES CD can be in MODE1 or MODE2/FORM1 format. The disc requires an 28h-byte ID
in sector 16, and a 800h-byte bootsector in sector 0, which may then loaded
further data via BIOS functions, or via direct access to the cdrom I/O ports.

The BIOS doesn't contain any filesystem support, however, the games may
implement a standard ISO filesystem (or some custom format), if desired.

Aside from data sectors, the drive controller does also support CD-DA audio
tracks and playing compressed ADPCM audio sectors.

**SNES CD Bootsector (sector 0)**

Located on Sector 0 (address 00:02:00), loaded to 00:1000h..17FFh, and then
started by jumping to 00:1080h.

**Primary Volume Descriptor (sector 16)**

Located on Sector 16 (address 00:02:16), the first 28h bytes must have
following values for boot-able SNES CDs.

```text
  000h 1    Volume Descriptor Type        (01h=Primary Volume Descriptor)
  001h 5    Standard Identifier           ("CD001")
  006h 1    Volume Descriptor Version     (01h=Standard)
  007h 1    Reserved                      (00h)
  008h 32   System Identifier             (a-characters) ("SUPERDISC")
  028h ...  (further ISO primary volume descriptor entries may follow here)
```

**Note**

Aside from booting executable software, the CD BIOS does also contain code for
some "ELECTRONIC BOOK" format, but the volume descriptor detection lacks
support for detecting that disc type.

<a id="snescartcdrombioscartridge"></a>

## SNES Cart CDROM - BIOS Cartridge

Contains extra DRAM, some small battery-backed SRAM, and the BIOS ROM. The DRAM
and SRAM are rather small, and there's no coprocessor. However, this is only
prototype, and Nintendo could have easly expanded the BIOS cartridge (without
needing to modify the actual CDROM hardware).

For example, there have been rumours about a 32bit CPU being planned, and SRAM
might have been intended to be replaced by a bigger memory chip (or possibly by
an external FLASH cart as used in Satellaview BIOS carts).

**BIOS User Interface**

```text
  START  --> Load CDROM (if any)
  SELECT --> SRAM Manager (in there: Up/Down=Select, B=Delete, Y=Exit)
  A+X    --> Test Screen (in there: Up/Down/B --> Menu Selection)
```

Self Check tests:

```text
  Page1: VRAM, CGRAM, OAM, WRAM, DMA, TIMER, SOUND (sound test works only once)
  Page2: BIOS_DRAM, BIOS_SRAM, CDROM DECODER, CD-PLAYER I/F
  The DECODER test seems to try to count sectors/second on STOPPED drive,
  that might fail on real HW, or it might work with the NOSYNC bit triggered?
```

ADPCM Test:

```text
  Use Up/Down and L/R Buttons to select File/Channel and MM:SS:FF
  Press B to play ADPCM audio (eg. from PSX disc with ADPCM at selected values)
  Press Y to toggle Normal/Double speed, press Select to go back to menu
  Observe that APU is muting sound output (unless previously running Selfcheck)
```

Communication (Mechacon) Test:

```text
  Use L/R Buttons to select a command, use B to issue the command, Select=Exit
  Use Up/Down and L/R Buttons to change variable parameters
```

CXD-1800 (Decoder) Test:

```text
  Use Up/Down and L/R Buttons to change Write values
  Use Y to toggle Read/Write, X to toggle IRQ, Select=Exit
```

**00h-03h:8000h-FFFFh - BIOS Cart ROM (128Kbyte LoROM) (Sticker 0.95 SX)**

The BIOS has CRC32=3B64A370h and the ROM/EPROM is badged "0.95 SX", there are
some ASCII strings in the file:

```text
  "Super Disc boot ROM ver.0.95 Jul. 14, 1992 by Tomomi Abe at SONY "
  "Super Disc BIOS program ver.0.93 by Tomomi Abe. May. 26 1992 at SONY. "
  01h,"CD001",01h,00h,"SUPERDISC",23x00h  ;28h-byte ISO volume descriptor
```

The cart header at 7FC0h-7FDFh is just FFh-filled and IRQ/NMI vectors point to
RAM:

```text
  7FC0  FF,FF,FF,FF,FF,FF,FF,FF,FF,FF,FF,FF,FF,FF,FF,FF
  7FD0  FF,FF,FF,FF,FF,FF,FF,FF,FF,FF,FF,FF,FF,FF,FF,FF
  7FE0  00,00,00,00,00,00,00,00,00,00,F8,1F,00,00,FC,1F
  7FF0  00,00,00,00,00,00,00,00,00,00,00,00,00,80,00,00
```

That uncommon combination of FFh's and IRQ/NMI vectors can be used to detect if
a ROM image is having Super Disc support.

**80h-87h:8000h-FFFFh - BIOS Cart Work RAM (256Kbyte DRAM) (two S-WRAM chips)**

This expands the SNES's internal 128KBytes to a total of 384Kbytes Work RAM.
Allowing to load code and data from CDROM to CPU memory space.

**90h:8000h-9FFFh - BIOS Cart Battery RAM (8Kbyte SRAM)**

**21D0h.W - BIOS Cartridge Battery RAM Lock (write 00h)**

**21E0h.W - BIOS Cartridge Battery RAM Unlock Step 2 (write 0Fh downto 01h)**

**21E5h.W - BIOS Cartridge Battery RAM Unlock Step 1 (write FFh)**

These ports seem to be used to write-protect the battery backed SRAM (the BIOS
functions are automatically locking/unlocking the SRAM when saving/deleting
game position files).

<a id="snescartcdrombiosfunctions"></a>

## SNES Cart CDROM - BIOS Functions

**SNES CD BIOS Function Summary (jump opcodes at 00:E0xxh)**

```text
  00h:E000h  cdrom_InitDetect          ;00h       ;\
  00h:E003h  cdrom_LoadFromDisc        ;01h       ;
  00h:E006h  cdrom_SendMechaconCommand ;02h       ; Main Functions
  00h:E009h  cdrom_WramToVramDMA       ;03h       ;
  00h:E00Ch  cdrom_PollMechacon        ;04h       ;/
  00h:E00Fh  no_function               ;05h..0Fh
  00h:E030h  cdrom_SramTest            ;10h       ;\
  00h:E033h  cdrom_SramGetDirectory    ;11h       ;
  00h:E036h  cdrom_SramSaveFile        ;12h       ; SRAM Functions
  00h:E039h  cdrom_SramLoadFile        ;13h       ;
  00h:E03Ch  cdrom_SramDeleteFile      ;14h       ;/
  00h:E03Fh  no_function               ;15h..1Fh
  00h:E060h  cdrom_DecoderDataMode     ;20h       ;\
  00h:E063h  cdrom_DecoderAudioMode    ;21h       ; Misc Functions
  00h:E066h  cdrom_DecoderTestDecint   ;22h       ;/
  00h:E069h  no_function               ;23h
  00h:Exxxh  crash                     ;24h and up
```

```text
 ______________________________ Main Functions ______________________________
```

**00h:E000h - cdrom_InitDetect**

Initializes variables and NMI/IRQ handlers at [1Fxxh], and tries to flush any
old mechacon IRQs, and to issue a mechacon get status command.

```text
  out: cy=error (0=okay, 1=no cdrom hardware)
```

**00h:E003h - cdrom_LoadFromDisc**

```text
  in: [1F00h]=source address (24bit LBA, or 3-byte MM,SS,FF address)
  in: [1F03h]=read mode (flag byte) (usually 40h for LBA with normal data)
  in: [1F04h]=destination address (24bit wram address) (or 16bit vram address)
  in: [1F07h]=transfer length (max 7FFFh bytes, or maybe max 87FFh works, too)
  in: [1F09h]=max number of sub-q mismatches or so (usually 0Fh)
  in: [1F33h]=file and channel bytes (for ADPCM mode only)
  out: cy=error (0=okay, 1=bad)
```

Flag byte format:

```text
  7    VRAM Mode (0=Load to WRAM, 1=Forward sectors from WRAM to VRAM)
  6    Source Address format (0=MM:SS:FF in non-BCD, 1=24bit LBA)
  5    ADPCM Mode (0=No, 1=Play ADPCM file/channel until EOR/EOF)
  4    Prevent loading (0=No, 1=Skip everything except ADPCM, if enabled)
  3-0  Unused (should be 0)
```

The cdrom_LoadFromDisc function uses DMA7 to transfer data from Disc to WRAM,
the "VRAM Mode" additionally uses DMA6 for forwarding incoming data from a WRAM
buffer (at 83h:C000h-FFFFh) to VRAM.

**00h:E006h - cdrom_SendMechaconCommand**

Allows to send mechacon commands (normally not required, the LoadFromDisc
functions does automatically issue seek+play+pause commands).

```text
  in: a=command (8bit)
  in: [1Fxxh]=optional parameters (for command 00h and 01h)
  out: cy=error (0=okay, 1=bad)
  out: [1F2E]=last response digit (unknown purpose, checked after seek_mmssff)
```

Command numbers are:

```text
  00h  seek_tr_indx   CxxxxF --> FFFFFx       ;in: [1F0Fh..1F12h]=four nibbles
  01h  seek_mmssff    BxxxxxxF --> FFFFFFFx   ;in: [1F13h..1F18h]=six nibbles
  02h  stop           D01F --> FFFx
  03h  play           D02F --> FFFx
  04h  pause          D03F --> FFFx
  05h  open_close     D04F --> FFFx
  06h  fast_forward   D10F --> FFFx
  07h  fast_reverse   D11F --> FFFx
  08h  forward        D12F --> FFFx
  09h  reverse        D13F --> FFFx
  0Ah  key_direct     D40F --> FFFx
  0Bh  key_ignore     D41F --> FFFx
  0Ch  continous      D42F --> FFFx
  0Dh  track_pause    D43F --> FFFx
  0Eh  index_pause    D44F --> FFFx
  0Fh  req_sub_q      D50F_0000000000000000F  ;out:[1F1Eh..1F2Dh]=16 nibbles
  10h  req_status     D51F_01234F             ;out:[1F19h..1F1Dh]=5 nibbles
  11h  normal_speed   D45F --> FFFx
  12h  double_speed   D46F --> FFFx
  13h  flush          F --> a
  N/A  ?              D14F --> FFFx
  N/A  ?              D15F --> FFFx
```

**00h:E009h - cdrom_WramToVramDMA (custom NMI handler callback)**

Usually done automatically by the default BIOS NMI handler: If CDROM loading is
done in "VRAM mode", then this functions forwards the incoming CDROM data from
WRAM to VRAM.

**00h:E00Ch - cdrom_PollMechacon (custom IRQ handler callback)**

Usually done automatically by the default BIOS IRQ handler: If a mechacon
command is being transmitted, then this function handles incoming mechacon
response nibbles, and sends further mechacon parameter nibbles (until
completion of the command sequence).

```text
 ______________________________ SRAM Functions ______________________________
```

**00h:E030h - cdrom_SramTest**

Tests the SRAM checksum, does range checks on free memory size and number of
files, automatically reformats/erases the SRAM in case of errors.

```text
  out: cy=error (0=okay, 1=bad, reformatted sram)
```

**00h:E033h - cdrom_SramGetDirectory**

Returns the whole SRAM directory with max 32 files, each 16-byte entry consists
of 14-byte filename, folled by 16bit filesize value.

```text
  in: DB:Y = destination address (200h byte buffer)
  out: cy=error (0=okay, 1=bad)
  out: a=number of files actually used                      ;\returned only
  out: [DB:Y+0..1FF]=directory (unused entries 00h-filled)  ;/when cy=0=okay
```

**00h:E036h - cdrom_SramSaveFile**

```text
  in: DB:Y = source address (14-byte name, 16bit length, filebody[length])
  out: cy=error (0=okay, 1=bad, directory or memory full)
```

**00h:E039h - cdrom_SramLoadFile**

```text
  in: DB:Y = source address (14-byte name, 16bit length, filebody[length])
  out: filebody[length] is overwritten by loaded file
       (zeropadded if the specified length exceeded the specified filesize)
  out: cy=error (0=okay, 1=bad, file not found)
```

**00h:E03Ch - cdrom_SramDeleteFile**

```text
  in: DB:Y = source address (14-byte name)
  out: cy=error (0=okay, 1=bad, file not found)
```

**Character Set for SRAM Filenames (shown when pressing SELECT in BIOS)**

```text
  00h..09h  "0..9"
  0Ah..23h  "A..Z"
  24h..27h  Space, Slash, Dash, Dot
  28h..7Fh  Japanese symbols
  80h..FFh  Cause directory sort-order corruption when creating/deleting files
```

```text
 ______________________________ Misc Functions ______________________________
```

**00h:E060h - cdrom_DecoderDataMode**

**00h:E063h - cdrom_DecoderAudioMode (CD-DA)**

These functions are just setting the decoder to data/audio mode, there are no
parameters or return values.

**00h:E066h - cdrom_DecoderTestDecint**

Runs a test on measuring the number of DECINT's per second (aka sectors per
second), passes okay when measuring 75+/-5 or 150+/-10 DECINTs (ie. both single
&amp; double speed mode should pass). Execution time of the test is 1 second.

```text
  out: cy=error (0=okay, 1=bad)
```

```text
 ______________________________ Bugs & Glitches _____________________________
```

Instead of using unsigned maths, the BIOS used a lot of signed comparisions
without overflow checking.

This is restricting the CDROM filesize to max 7FFFh (or possibly 87FFh might
work when subtracting the first sector unit).

SRAM filename characters are also using that signed maths for the filename sort
order (using characters 80h..FFh can have unpredictable results when
adding/removing SRAM files; which may cause new comparision overflows to
occur/disappear).

SRAM is intended to hold max 32 files, however, that limit is checked when
overwriting old files (not when creating new files): Results are that one
cannot overwrite any files if the cart contains 32 files or more, whilst, on
the other hand, one could create even more then 32 files.

Booting the BIOS seems to be instantly STOPPING the drive motor (after the BIOS
intro/delay), apparently preventing the drive to spin-up, and to read the TOC,
or even to load data from the disc - until going through the "PRESS START" nag
screen.

<a id="snescartcdrommechacon"></a>

## SNES Cart CDROM - Mechacon

The Mechacon handles all the drive mechanics (motor start/stop, seeking,
tracking, gain, balance). Essentinally it's covering only the "Audio" part
(streaming bits and watching the SubQ-channel's position info) without being
aware of "Digital" data in CDROM Headers &amp; Data Blocks.

However, the same mechanics are also used for "Playing" CDROM data discs (ie.
seek the desired sector in MM:SS:FF notation, then issue Play command to start
reading).

Observe that seeking may inaccuratly settle "nearby" of the desired target
address (ie. one must check the Data header's MM:SS:FF bytes from the Decoder
chip, and ignore any sectors with smaller sector numbers, or eventually retry
seeking if the sector number is higher as planned).

**21E1h.R/W - CDROM Unit Mechacon CPU (probably the NEC chip on daughterboard)**

```text
  7     Transfer Ready IRQ      (R)
  6-4   -
  3-0   Data                    (R/W)
```

**Mechacon Commands**

```text
  Access MM/SS/FF     BmmssffF                --> FFFFFFFx
  Access Track/Index  CttiiF                  --> FFFFFx
  Stop                D01F                    --> FFFx
  Play                D02F                    --> FFFx
  Pause               D03F                    --> FFFx
  Open/Close          D04F                    --> FFFx
  Fast Forward        D10F                    --> FFFx
  Fast Reverse        D11F                    --> FFFx
  Forward             D12F                    --> FFFx
  Reverse             D13F                    --> FFFx
  Key Direct          D40F                    --> FFFx
  Key Ignore          D41F                    --> FFFx
  Continous Play      D42F                    --> FFFx
  Auto Track Pause    D43F                    --> FFFx
  Auto Index Pause    D44F                    --> FFFx
  Normal Speed        D45F                    --> FFFx
  Double Speed        D46F                    --> FFFx
  Q-Data Request      D50F 0000000000000000F  --> FFFx ................x
  Status Request      D51F 01234F             --> FFFx .....x
  Nop/Flush ?         F                       --> x
```

**Q-Data Request Digits**

These 16 digits are probably 8 bytes straight from 12-byte SubQ Position data
in BCD format (probably Track, Index, MM:SS:FF, AMM:ASS:AFF) (ie. probably
excluding the ADR/Control byte, Reserved byte, and the two CRC bytes).

**Status Request Digits**

```text
 Digit(0) - Disc Type
  Bit0: Disc Type (or maybe Track Type) (0=Audio, 1=Data)
  Bit1-3: Unknown/unused
 Digit(1)
  Unknown/unused
 Digit(2) - Drive state
  00h  No Disc
  01h  Stop
  02h  Play
  03h  Pause
  04h  Fast Reverse
  05h  Fast Forward
  06h  Slow Reverse
  07h  Slow Forward
  08h  ?
  09h  ?
  0Ah  Access, Seek
  0Bh  Access, Read TOC
  0Ch  Tray Open
  0Dh  ?
  0Eh  ?
  0Fh  ?
 Digit(3)
  Unknown/unused
 Digit(4)
  Unknown/unused
```

Unknown bits &amp; digits might include double-speed flag, LCD pad buttons, or
such stuff.

<a id="snescartcdromdecoderfifo"></a>

## SNES Cart CDROM - Decoder/FIFO

CXD1800Q chip (equivalent to CXD1196AR datasheet).

IRQs can be sensed via CXD1800 Register(01h.R).

**21E2h.R/W - CDROM Unit CXD1800 Index (REGADR) (R/W)**

```text
  7-5  -      Reserved (should be 0)
  4-0  RA4-0  Register Index
```

This register is used for selection of the internal registers.

```text
 --> When the low order 4 bits of REGADR are not 0 (hex), and a register write
     or read is made by setting A0=1 and /CS=0, the low order 4 bits of
     REGADR are incremented
 --> REGADR is cleared to 00h by rising edge of DMAEN (in DMA Control register)
```

**21E3h.R/W - CDROM Unit CXD1800 Data (R/W)**

```text
  7-0  Data for register selected via REGADR
```

```text
 _________________________ Configuration _________________________
```

**X1h.W - DRVIF - DRIVE Interface (W)**

```text
  7   XSLOW    DMA/SRAM Speed (0=Slow/12 clks/320ns, 1=Fast/4 clks/120ns)
  6   C2PL1ST  DATA input C2PO-byte-order (0=Upper first, 1=Lower first)
  5   LCHLOW   Audio LRCK Polarity for Left channel (0=High, 1=Low)
  4   BCKRED   Audio BCLK Edge for strobing DATA (0=Falling, 1=Rising)
  3-2 BCKMD1-0 Audio BCLKs per WCLK cycle (0=16, 1=24, 2/3=32)
  1   LSB1ST   Audio DATA (bit?-)ordering (0=MSB First, 1=LSB first)
  0   CLKLOW   CLK Pin Output (0=8.4672MHz, 1=Fixed Low)
```

Configures how the drive is wired up. The SNES CD doesn't touch this register
and leaves it at it's power-up default. The Decoder should be disabled before
changing the register.

**X2h.W - CHPCTL - Chip Control (W)**

```text
  7-5 -        Reserved (should be 0)
  4   CHPRST   Chip Reset (takes 500ns)   (0=No change, 1=Reset the chip)
  3   CD-DA    CD-Digital Audio Mode      (0=Data/CDROM, 1=Audio/CD-DA)
  2   SWOPN    Sync Detection Window      (0=Only if Sync expected, 1=Anytime)
  1   RPSTART  Repeat Correction Start  (0=No change, 1=Repeat if repeat mode)
  0   ADPEN    ADPCM Decode (to be set max 11.5ms after DECINT) (0=No, 1=Yes)
```

**X3h.W - DECCTL - Decoder Control (W)**

```text
  7   AUTOCI    ADPCM Coding Information (0=Use CI Register, 1=Disc Subheader)
  6   -         Reserved (should be 0)
  5   MODESEL   Mode Select (when AUTODIST=0)               (0=MODE1, 1=MODE2)
  4   FORMSEL   Form Select (when AUTODIST=0 and MODESEL=1) (0=FORM1, 1=FORM2)
  3   AUTODIST  Auto Distinction        (0=Use MODESEL/FORMSEL, 1=Disc Header)
  2-0 DECMD2-0  Decoder Mode            (00h-07h, see below)
```

Decoder Mode values:

```text
  00h/01h = Decoder disable (to be used for CD-DA Audio mode & during config)
  02h/03h = Monitor only    (read Header/Subheader, but don't write SRAM?)
  04h     = Write only mode (write sectors to SRAM without error correction?)
  05h     = Real time correction (abort correction if it takes too long?)
  06h     = Repeat correction (allow resume via RPSTART for important sectors?)
  07h     = Inhibit (reserved)
```

**X6h.W - CI - ADPCM Coding Information (to be used when AUTOCI=0) (W)**

```text
  7   -        Reserved (should be 0)
  6   EMPHASIS ADPCM Emphasis           (0=Normal/Off, 1=Emphasis)
  5   -        Reserved (should be 0)
  4   BITL4H8  ADPCM Bit Length         (0=Normal/4bit, 1=8bit)
  3   -        Reserved (should be 0)
  2   FSL3H1   ADPCM Sampling Frequency (0=37800Hz, 1=18900Hz)
  1   -        Reserved (should be 0)
  0   MONOSTE  ADPCM Mono/Stereo        (0=Mono, 1=Stereo)
```

This register is used only when AUTOCI=0, allowing to use the correct ADPCM
format even in case of read errors on the CI byte in sector sub header (if
AUTOCI=1, such errors would trigger CIERR interrupt and omit playback of the
ADPCM sector with bad CI byte).

**0Dh.W - "PLBA" - Unknown  &lt;-- shown as so in SNES CD's "CXD1800" test screen**

```text
  7-0  PLBA?    ;Maybe PLBA means "PLayBAck" or even "PLayBAckwards" or so?
```

```text
 _________________________ Interrupt / Status _________________________
```

**01h.R - INTSTS - Interrupt Status (0=No IRQ, 1=IRQ) (R)**

**X4h.W - INTMSK - Interrupt Mask (0=Disable, 1=Enable) (W)**

**X5h.W - INTCLR - Interrupt Clear/Ack (0=No change, 1=Clear/ack) (W)**

```text
  7   ADPEND  ADPCM sector decode completed, and ADPCM disabled for next sector
  6   DECTOUT Decoder Time Out (no Sync within 3 sectors)
                Can occurs (only?) after the DECODER has been set to
                monitor only mode, or real time correction mode.
  5   DMACMP  DMA Complete (by DMAXFRC=0)                       (0=No, 1=Yes)
  4   DECINT  Decoder Interrupt (new "current sector" arrived)  (0=No, 1=Yes)
                If a SYNC mark is detected or internally inserted during
                execution of the write only, monitor only and real time
                correction modes by the DECODER, the DECINT status is created.
                  When the SYNC mark detected window is open, however, if the
                SYNC mark spacing is less than 2352 bytes, the DECINT status
                is not created.
                  During execution of the repeat correction mode by the DECODER,
                the DECINT status is created each time a correction ends.
  3   CIERR   Coding Info Error  (0=Okay, 1=Bad CI in ADPCM sector & AUTOCI=1)
  2-0 -       Reserved (should be 0)
```

**DECINT Handling (new "current sector" successfully/unsuccessfully received)**

First check the error flags in STS and HDRFLG registers (if desired, also check
MDFM and ADPCI to see how the decoder interpreted the sector).

Then check the MM:SS:FF values in HDR_xxx registers and ignore the sector if
the values aren't matching up with the desired values (that may happen if the
mechacon settled on sector number slightly lower than the requested seek
address, it might also happen during seek-busy phase, and it might happen if a
sector was skipped for some reason, which would require to issue a new seek
command and to retry reading the skipped sector).

When using ADPCM playback, also check SHDR_xxx registers to see if the sector
contains ADPCM data, and if it's having the desired file/channel numbers, if
so, set the ADPEN bit in CHPCTL.

Otherwise, if the sector is desired to be loaded to SNES memory: Handle the
CMADR either immediately, or if that isn't possible, memorize it in a queue,
and handle it as soon as possible, ie. after processing older queue entries,
but before the Sector Buffer location gets overwritten by newer sectors; the
32K SRAM can probably hold at least 8 sectors (8 x 924h bytes, plus some unused
padding areas, possibly plus some ADPCM area; as so on PSX).

As for handling CMADR: Usually one would only read the 800h-byte data portion
(without Header and Subheader), done by writing CMDADR+4 (for MODE1) or
CMDADR+0Ch (for MODE2) to DMAADRC, then writing 8800h to DMAXFRC, and then
reading 800h bytes from port 21E2h (usually via a SNES DMA channel).

**02h.R - STS - Status (R)**

```text
  7   DRQ     Data Request (DRQ Pin)                            (0=?, 1=?)
  6   ADPBSY  ADPCM Playback Busy                               (0=No, 1=Busy)
  5   ERINBLK Erasure in Block; C2 flg anywhere except Syncmark (0=Okay, 1=Bad)
  4   CORINH  Correction Inhibit; MODE/FORM error & AUTODIST=1  (0=Okay, 1=Bad)
  3   EDCOK   EDC Error Detect Checksum (optional for FORM2)    (0=Bad, 1=Okay)
  2   ECCOK   ECC Error Correction Codes (not for FORM2)        (0=Bad, 1=Okay)
  1   SHRTSCT Sync Mark too early, no ECC/EDC done              (0=Okay, 1=Bad)
  0   NOSYNC  Sync Mark too late/missing, unreal SYNC inserted  (0=Okay, 1=Bad)
```

**03h.R - HDRFLG - Header C2-Error Flags (R)**

```text
  7  MIN     Header MM   (0=Okay, 1=Error) ;\
  6  SEC     Header SS   (0=Okay, 1=Error) ; Header from MODE1/MODE2 data
  5  BLOCK   Header FF   (0=Okay, 1=Error) ; sector (ie. not for audio)
  4  MODE    Header MODE (0=Okay, 1=Error) ;/
  3  FILE    Sub-Header  (0=Okay, 1=Error) ;\Subheader exists for MODE2 only
  2  CHANNEL Sub-Header  (0=Okay, 1=Error) ; (the SNES CD BIOS wants these
  1  SUBMODE Sub-Header  (0=Okay, 1=Error) ; bits to be zero for MODE1, too)
  0  CI      Sub-Header  (0=Okay, 1=Error) ;/
```

**X4h.R - HDR_MIN - Header "MM" Minute (R)**

**X5h.R - HDR_SEC - Header "SS" Second (R)**

**X6h.R - HDR_BLOCK - Header "FF" Frame (R)**

**X7h.R - HDR_MODE - Header Mode (R)**

**08h.R - SHDR_FILE - Sub-Header File (R)**

**09h.R - SHDR_CH - Sub-Header Channel (R)**

**0Ah.R - SHDR_S-MODE - Sub-Header SubMode (R)**

**0Bh.R - SHDR_CI - Sub-Header Coding Info (R)**

Contains current sector's 4-byte Header (and 4-byte Subheader for MODE2 discs).

**0Ch/0Dh.R - CMADR_L/H - Current Minute Address, Low/High (R)**

```text
  15    Unused
  14-0  Pointer to 1st byte of current sector (ie. to MM:SS:FF:MODE header)
```

Note: "Minute" is meaning the "1st byte of the sector". Named so because the
1st byte the "MM" value from the "MM:SS:FF:MODE" header. The sector stored in
SRAM is 924h bytes in size (ie. the whole 930h-byte sector, excluding the 12
Sync bytes).

**XEh.R - MDFM - MODE/FORM (R)**

```text
  7-5 X        Unused
  4   RMODE2   Raw MODE byte, Bit2-7 ("logic sum") (aka all six bits ORed?)
                  Indicates the logic sum of the value of the high-order 6 bits
                  of the raw MODE byte AND THE POINTER (whut pointer?).
  3   RMODE1   Raw MODE byte, Bit1
  2   RMODE0   Raw MODE byte, Bit0
  1   CMODE    Correction Mode (0=MODE1, 1=MODE2)
  0   CFORM    Correction Form (0=FORM1, 1=FORM2) (for MODE2 only)
```

These bits indicate which of the MODEs and FORMs this IC determined that the
current sector was associated with when it corrected errors.

**XFh.R - ADPCI - ADPCM Coding Information (R)**

```text
  7   MUTE     DA data is muted on      (0=No, 1=Muted)      <--- from where?
  6   EMPHASIS ADPCM Emphasis           (0=Normal/Off, 1=Emphasis)
  5   EOR      End of Record                         <--- (from SubMode.Bit0)
  4   BITLNGTH ADPCM Bit Length         (0=Normal/4bit, 1=8bit)
  3   X        Unused
  2   FS       ADPCM Sampling Frequency (0=37800Hz, 1=18900Hz)
  1   X        Unused
  0   M/S      ADPCM Mono/Stereo        (0=Mono, 1=Stereo)
```

Bit5 gets 1 when the SubMode.bit0=1 and there is no error in the SubMode byte.

```text
 _________________________ DMA / Sector Buffer _________________________
```

**00h.R - DMADATA - SRAM-to-CPU Xfer Data (R)**

```text
  7-0    Data from Sector buffer at [DMAADRC]
```

Reading increments DMAADRC and decrements DMAXFRC. However, for this special
case, REGADR is NOT incremented (allowing to read DMADATA continously without
needing to reset REGADR).

**X7h/X8h.W - DMAADRC_L/H - SRAM-to-CPU Xfer Address, Low/High (W)**

**1Ah/1Bh.R - DMAADRC_L/H - SRAM-to-CPU Xfer Address, Low/High (R)**

```text
  15     Unused
  14-0   Current Read address for SRAM-to-CPU transfer (incrementing)
```

**X9h/XAh.W - DMAXFRC_L/H - SRAM-to-CPU Xfer Length &amp; DMA Control, Low/High (W)**

**18h/19h.R - DMAXFRC_L/H - SRAM-to-CPU Xfer Length, Low/High (R)**

For writing X9h/XAh (with DMAEN bit inserted between other bits):

```text
  15-12 DMAXFRC11-8 Transfer Length Remain Counter DMAXFRC, bit11-8
  11    DMAEN       CPU DMA Enable (0=Inhibit, 1=Enable)
  10-8  -           Reserved (should be 0)
  7-0   DMAXFRC7-0  Transfer Length Remain Counter DMAXFRC, bit7-0
```

For reading 18h/19h (without DMAEN bit, but instead with 15bit counter range):

```text
  15    Unused      Unused
  14-0  DMAXFRC14-0 Transfer Length Remain Counter DMAXFRC, bit14-0
```

Setting DMAEN=1 does automatically set REGADR=00h (ie. select the DMADATA
register). DMAEN=1 should be used whenever starting a transfer (not matter if
the data is transferred via DMA, or if it's manually polled from DMADATA
register).

The DMACMP IRQ will occur when DMAXFRX reaches zero (to avoid that effect, one
may write DMAXFRC=0800h (DMAEN=1 and counter=000h); that will reportedly
prevent the IRQ; either because the counter doesn't decrease beyond zero, or
maybe it wraps to 7FFFh and thus won't expire anytime soon).

**XBh/XCh.W - DRVADRC_L/H - Disc-to-SRAM Xfer Address, Low/High (W)**

**1Ch/1Dh.R - DRVADRC_L/H - Disc-to-SRAM Xfer Address, Low/High (R)**

```text
  15     Unused
  14-0   Disc-to-SRAM Xfer Address (incrementing)
```

This register is automatically advanced when storing incoming disc data in
Sector Buffer. The SNES CD BIOS doesn't touch this register at all.

Note: The datasheet has some obscure notes about needing to write the register
before "write only mode and real time correction mode" (unknown how/why/when to
do that).

<a id="snescartcdromcomponentlist"></a>

## SNES Cart CDROM - Component List

"based on the on photos that have been posted, the main board has the same
parts as a Super Famicom, but with 7 additional chips:

```text
  1) CXD2500 CD-DSP
  2) CXD1800 CD-ROM decoder/interface
  3) 32K SRAM (presuambly the CD-ROM sector buffer)
  4) some 20 pin SOP device that looks like a bus buffer
  5) a QFP with no markings (mechacon MCU?)
  6) A Sanyo 16 bit stereo DAC
  7) an 8 pin SOP - probably a dual opamp (it's next to the DAC outputs,
     so probably a buffer)
  The top board has a 4-bit MCU and a liquid crystal display.
```

There are also 5 visible ICs on the back of the CD-ROM control board - one of
them is a Rohm BTL driver another looks like a Sony CXA1272 (old CD drive focus
/ tracking servo) - the other chips are small SOP devices with numbers I can't
read. No sign of an RF amp chip, but on a lot of those older drives it was
built into the optical pickup. Basically, it has all the chips you would expect
for a basic data/audio CD drive of that vintage and nothing else."

**Sony Playstation SFX-100 Console Component List**

```text
 Mainboard (MA-115, 0-396-987-04)
  IC101 100pin Nintendo S-CPU, 5A22-01  (65816 CPU with joypad I/O ports)
  IC102 100pin Nintendo S-PPU1, 5C77-01 (Video Chip 1)
  IC103 100pin Nintendo S-PPU2, 5C78-01 (Video Chip 2)
  IC104 28pin  NEC uPD43256A6U-10L? (32Kx8 SRAM, Video RAM 1)
  IC105 28pin  NEC uPD43256A6U-10L? (32Kx8 SRAM, Video RAM 2)
  IC106        ... whatever, maybe S-ENC or similar (Video RGB to composite)
  IC107 64pin  Nintendo S-WRAM (128Kx8 DRAM with B-bus)
  IC108 28pin  65256BLFP-12T   (32Kx8 SRAM, Sound RAM 1)
  IC109 18pin  Nintendo F411   (NTSC CIC)
  IC110 28pin  65256BLFP-12T   (32Kx8 SRAM, Sound RAM 2)
  IC111 64pin  SONY CXP1100Q-1 (APU, some newer S-SMP revision, SPC700 CPU)
  IC112 80pin  SONY CXD1222Q-1 (APU, some newer S-DSP revision, Sound Chip)
  IC113 20pin  LC78815M        (Two-channel 16bit D/A converter 1)
  IC201 80pin  SONY CXD2500AQ  (CDROM Signal Processor)
  IC202 20pin  LC78815M        (Two-channel 16bit D/A converter 2)
  IC203 48pin  Noname   ...  maybe Servo Amplifier (like CXA1782BR on PSX?)
  IC204 80pin  SONY CXD1800Q    (CDROM Decoder/FIFO, equivalent to CXD1196AR)
  IC205 18pin  74xxxx? (PCB has 20pin solderpoints, but chip is only 18pin)
  IC206 28pin  SONY CXK58257AM-70L (32Kx8 SRAM, CDROM Sector Buffer)
  IC301 8pin   Texas Instruments RC4558, "R4558 TI 25" (Dual Op-Amp 1)
  IC302 ...    ... whatever, maybe one of the 8pin IC???'s
  IC303 8pin   Texas Instruments RC4558, "R4558 TI 25" (Dual Op-Amp 2)
  ICxxx ...    if any... ?
  IC??? 8pin   whatever (front board edge, near headphone socket)
  IC??? 8pin   whatever (front board edge, near headphone socket)
  IC??? 24pin  whatever (front/mid board edge) (probably S-ENC or so)
  IC??? 3pin   voltage regulator (7805 or similar)
  IC??? ??     address decoder for I/O ports, 21E4h latch, NEXT port...?
               (maybe IC203 is doing that? but then where's Servo Amplifier?)
  CN201 29pin  To LCD Board
  CN..  ..     To CDROM Drive
  CN..  ..     To Controller Ports
  CN..  62pin  SNES Cartridge Slot
  CN..  ..     Rear panel
 Daughterboard with LCD
  IC701  80pin NEC uPD75P308GF  (CDROM Mechacon?)
  IC7xx ...    if any... ?
  X701         oscillator
  CN701  28pin LCD and six Buttons    (28pin, or maybe 2x28 pins?)
  CN702   4pin to somewhere  (2 LEDs ?, left of drive tray)
  CN703  29pin to Mainboard           (29pin, or maybe 2x29 pins?)
  CN704   3pin to front panel (disc eject button?, right of drive tray)
  ICxxx ...    if any... ?
  N/A?   28pin Something like BA6297AFP,BA6398FP,BA6397FP,AN8732SB,etc ?
 Daughterboard with controller ports
  ???   ...    whatever
 Daughterboard with Eject button
  ???   ...    whatever, a button, and maybe more stuff for the 3pin wire
 Daughterboard with LEDs
  ???   ...    whatever, two LEDs, and maybe more stuff for the 4pin wire
 Components in actual CD Drive unit
  ???   ...    whatever
 External Connectors
  1x snes cartridge slot (top)
  2x controller ports (front)
  1x 3.5mm headphone socket with "voltage level" regulator (front)
  1x "NEXT" port (serial link like PSX maybe?)
  1x Audio R        (red) (apparently with mono-switch when not connected)
  1x Audio L (MONO) (white)
  1x Video          (yellow)
  1x S VIDEO
  1x RF DC OUT
  1x MULTI OUT
  1x DC IN 7.6V
 Note: Some other/similar model has three RCA jacks instead headphone on front
```

**BIOS Cartridge - Case Sticker "'92.10.6." (plus some japanese symbols)**

```text
  PCB "RB-01, K-PE1-945-01"
  IC1  64pin  Nintendo S-WRAM (128Kx8 DRAM)
  IC2  64pin  Nintendo S-WRAM (128Kx8 DRAM)
  IC3  32pin  HN2xxxxxx? (Sticker 0.95 SX) (ROM/EPROM)
  IC4  28pin  SONY CXK5864BM-12LL (8Kx8 SRAM)
  IC5  16pin  Noname?
  IC6  14pin  74F32 (Quad 2-input OR gates)
  IC7  16pin  74F138? (1-of-8 inverting decoder/demultiplexer?)
  IC8  14pin  Noname?
  IC9  14pin  Noname?
  IC10 16pin  Noname?
  IC11 16pin  Nintendo D411 (NTSC CIC)
  ?    ?      white space (in upper left)
  ?    2pin   something with 2 pins is apparently on PCB back side (battery?)
```

**LCD/Button Panel**

```text
           PlayStation
                       SFX-100
  .---------------------------.
  |    TRACK  STEP/MIN SEC    |
  |  .---------------------.  |
  |  |        (LCD)        |  |
  |  '---------------------'  |
  |   PLAY MODE     REMAIN    |
  |  =========== ===========  |
  |      |<<         >>|      |
  |  =========== ===========  |
  |     |> ||         []      |
  |  =========== ===========  |
  '---------------------------'
```

Note: The date codes on the three S-WRAM's, D411, and uPD75P308GF seem to be
from 1991. Sticker on case of BIOS cart seems to be from 1992.
