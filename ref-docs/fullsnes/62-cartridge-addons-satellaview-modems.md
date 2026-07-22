# Fullsnes — Cartridge Add-Ons (Super Game Boy, Satellaview, Data Pack, Nintendo Power, Sufami Turbo, X-Band Modem)

[Index](00-index.md) · [« Cartridge Coprocessors](61-coprocessors.md) · [FLASH Backup, Cheat Devices, Tri-Star, Pirate Multicarts, Copiers & CD-ROM Drive »](63-copiers-cheat-devices-cdrom.md)

**Sections in this file:**

- [SNES Cart Super Gameboy](#snes-cart-super-gameboy)
- [SNES Cart Satellaview (satellite receiver & mini flashcard)](#snes-cart-satellaview-satellite-receiver-mini-flashcard)
- [SNES Cart Satellaview I/O Map](#snes-cart-satellaview-io-map)
- [SNES Cart Satellaview I/O Ports of MCC Memory Controller](#snes-cart-satellaview-io-ports-of-mcc-memory-controller)
- [SNES Cart Satellaview I/O Receiver Data Streams](#snes-cart-satellaview-io-receiver-data-streams)
  - [SNES Cart Satellaview I/O Receiver Data Streams (Notes)](#snes-cart-satellaview-io-receiver-data-streams-notes)
- [SNES Cart Satellaview I/O Receiver Control](#snes-cart-satellaview-io-receiver-control)
- [SNES Cart Satellaview I/O FLASH Detection (Type 1,2,3,4)](#snes-cart-satellaview-io-flash-detection-type-1234)
- [SNES Cart Satellaview I/O FLASH Access (Type 1,3,4)](#snes-cart-satellaview-io-flash-access-type-134)
- [SNES Cart Satellaview I/O FLASH Access (Type 2)](#snes-cart-satellaview-io-flash-access-type-2)
- [SNES Cart Satellaview Packet Headers and Frames](#snes-cart-satellaview-packet-headers-and-frames)
- [SNES Cart Satellaview Channels and Channel Map](#snes-cart-satellaview-channels-and-channel-map)
- [SNES Cart Satellaview Town Status Packet](#snes-cart-satellaview-town-status-packet)
- [SNES Cart Satellaview Directory Packet](#snes-cart-satellaview-directory-packet)
- [SNES Cart Satellaview Expansion Data (at end of Directory Packets)](#snes-cart-satellaview-expansion-data-at-end-of-directory-packets)
- [SNES Cart Satellaview Other Packets](#snes-cart-satellaview-other-packets)
- [SNES Cart Satellaview Buildings](#snes-cart-satellaview-buildings)
- [SNES Cart Satellaview People](#snes-cart-satellaview-people)
- [SNES Cart Satellaview Items](#snes-cart-satellaview-items)
- [SNES Cart Satellaview SRAM (Battery-backed)](#snes-cart-satellaview-sram-battery-backed)
- [SNES Cart Satellaview FLASH File Header](#snes-cart-satellaview-flash-file-header)
- [SNES Cart Satellaview BIOS Function Summary](#snes-cart-satellaview-bios-function-summary)
- [SNES Cart Satellaview Interpreter Token Summary](#snes-cart-satellaview-interpreter-token-summary)
- [SNES Cart Satellaview Chipsets](#snes-cart-satellaview-chipsets)
- [SNES Cart Data Pack Slots (satellaview-like mini-cartridge slot)](#snes-cart-data-pack-slots-satellaview-like-mini-cartridge-slot)
- [SNES Cart Nintendo Power (flashcard)](#snes-cart-nintendo-power-flashcard)
- [SNES Cart Nintendo Power - New Stuff](#snes-cart-nintendo-power-new-stuff)
- [SNES Cart Nintendo Power - I/O Ports](#snes-cart-nintendo-power-io-ports)
- [SNES Cart Nintendo Power - FLASH Commands](#snes-cart-nintendo-power-flash-commands)
- [SNES Cart Nintendo Power - Directory](#snes-cart-nintendo-power-directory)
- [SNES Cart Sufami Turbo (Mini Cartridge Adaptor)](#snes-cart-sufami-turbo-mini-cartridge-adaptor)
- [SNES Cart Sufami Turbo General Notes](#snes-cart-sufami-turbo-general-notes)
- [SNES Cart Sufami Turbo ROM/RAM Headers](#snes-cart-sufami-turbo-romram-headers)
- [SNES Cart Sufami Turbo BIOS Functions & Charset](#snes-cart-sufami-turbo-bios-functions-charset)
- [SNES Cart X-Band (2400 baud Modem)](#snes-cart-x-band-2400-baud-modem)
- [SNES Cart X-Band Misc](#snes-cart-x-band-misc)
- [SNES Cart X-Band I/O Map](#snes-cart-x-band-io-map)
- [SNES Cart X-Band I/O - Memory Patch/Mapping](#snes-cart-x-band-io-memory-patchmapping)
- [SNES Cart X-Band I/O - Smart Card Reader](#snes-cart-x-band-io-smart-card-reader)
- [SNES Cart X-Band I/O - LED and Debug](#snes-cart-x-band-io-led-and-debug)
- [SNES Cart X-Band I/O - Whatever Stuff (External FIFO for Modem?)](#snes-cart-x-band-io-whatever-stuff-external-fifo-for-modem)
- [SNES Cart X-Band I/O - Rockwell Modem Ports](#snes-cart-x-band-io-rockwell-modem-ports)
- [SNES Cart X-Band Rockwell Notes](#snes-cart-x-band-rockwell-notes)
- [SNES Cart X-Band BIOS Functions](#snes-cart-x-band-bios-functions)

---

<a id="snescartsupergameboy"></a>

## SNES Cart Super Gameboy

The Super Gameboy (SGB) is some kind of an adaptor for monochrome handheld
Gameboy games. The SGB cartridge contains a fully featured gameboy (with CPU,
Video &amp; Audio controllers), but without LCD screen and without joypad
buttons.

The 4-grayshade 160x144 pixel video signal is forwarded to SNES VRAM and shown
on TV Set, and in the other direction, the SNES joypad data is forwarded to SGB
CPU.

Some gameboy games include additional SGB features, allowing to display a
256x224 pixel border that surrounds the 160x144 pixel screen, there are also
some (rather limited) functions for colorizing the monochrome screen, plus some
special Sound, OBJ, Joypad functions. Finally, the gameboy game can upload
program code to the SNES and execute it.

**Chipset**

```text
  SGB CPU - 80pin - Super Gameboy CPU/Video/Audio Chip
  ICD2-R (or ICD2-N) - 44pin - Super Gameboy SGB-to-SNES Interface Chip
```

Plus VRAM/WRAM for SGB CPU, plus SNES SGB BIOS, plus CIC chip.

**SGB I/O Map (ICD2-R)**

```text
  6000       R  LCD Character Row and Buffer Write-Row
  6001       W  Character Buffer Read Row Select
  6002       R  16-Byte Packet Available Flag
  6003       W  Reset/Multiplayer/Speed Control
  6004-6007  W  Controller Data for Player 1-4
  6008-600E  -  Unused (Open Bus, or mirror of 600Fh on some chips)
  600F       R  Chip Version (21h or 61h)
  6800-680F  -  Unused (Open Bus)
  7000-700F  R  16-byte command packet (addr 7000..700F)
  7800       R  Character Buffer Data (320 bytes of currently selected row)
  7801-780F  R  Unused (Mirrors of 7800h, not Open Bus)
```

The ICD2 chips decodes only A0-A3,A11-A15,A22 (so above is mirrored to various
addresses at xx6xxN/xx7xxN). Reading the Unused registers (and write-only ones)
returns garbage. On chips with [600Fh]=61h, that garbage is:

```text
  CPU Open Bus values (though, for some reason, usually with bit3=1).
```

On chips with [600Fh]=21h, that garbage is:

```text
  6001h.R, 6004h-6005h.R --> mirror of 6000h.R
  6003h.R, 6006h-6007h.R --> mirror of 6002h.R
  6008h-600Eh.R          --> mirror of 600Fh.R
```

On ICD2-N chips and/or such with [600Fh]=other, that garbage is: Unknown.

**SGB Port 6000h - LCD Character Row and Buffer Write-Row (R)**

```text
  7-3  Current Character Row on Gameboy LCD (0..11h) (11h=Last Row, or Vblank)
  2    Seems to be always zero
  1-0  Current Character Row WRITE Buffer Number (0..3)
```

**SGB Port 6001h - Character Buffer Read Row Select (W)**

```text
  7-2  Unknown/unused      (should be zero)
  1-0  Select Character Row READ Buffer Number (0..3)
```

Selects one of the four buffer rows (for reading via Port 7800h). Only the
three "old" buffers should be selected, ie. not the currently written row
(which is indicated in 6000h.Bit1-0).

**SGB Port 6002h - 16-Byte Packet Available Flag (R)**

```text
  7-1  Seems to be always zero
  0    New 16-byte Packet Available (0=None, 1=Yes)
```

When set, a 16-byte SGB command packet can be read from 7000h-700Fh; of which,
reading 7000h does reset the flag in 6002h.

**SGB Port 6003h - Reset/Multiplayer/Speed Control (W)**

```text
  7    Reset Gameboy CPU   (0=Reset, 1=Normal)
  6    Unknown/unused      (should be zero)
  5-4  num_controllers     (0,1,3=One,Two,Four)  (default 0=One Player)
  3-2  Unknown/unused      (should be zero)
  1-0  SGB CPU Speed       (0..3 = 5MHz,4MHz,3MHz,2.3MHz) (default 1=4MHz)
```

The LSBs select the SGB CPU Speed (the SNES 21MHz master clock divided by
4,5,7,9). Unknown if/how/when the SGB BIOS does use this. For the SGB, the
exact master clock depends on the console (PAL or NTSC). For the SGB2 it's
derived from a separate 20.9MHz oscillator.

**SGB Port 6004h-6007h - Controller Data for Player 1-4 (W)**

```text
  7    Start     (0=Pressed, 1=Released)
  6    Select    (0=Pressed, 1=Released)
  5    Button B  (0=Pressed, 1=Released)
  4    Button A  (0=Pressed, 1=Released)
  3    Down      (0=Pressed, 1=Released)
  2    Up        (0=Pressed, 1=Released)
  1    Left      (0=Pressed, 1=Released)
  0    Right     (0=Pressed, 1=Released)
```

Used to forward SNES controller data to the gameboy Joypad inputs. Ports
6005h-6007h are used only in 2-4 player mode (which can be activated via 6003h;
in practice: this can be requested by SGB games via MLT_REQ (command 11h), see
SGB section in Pan Docs for details).

**SGB Port 600Fh - Chip Version (R)**

```text
  7-0  ICD2 Chip Version
```

Seems to indicate the ICD2 Chip Version. Known values/versions are:

```text
  21h = ICD2-R (without company logo on chip package)
  61h = ICD2-R (with company logo on chip package)
  ??  = ICD2-N (this one is used in SGB2)
```

The versions differ on reading unused/write-only ports (see notes in SGB I/O
map).

**SGB Port 7000h-700Fh - 16-byte Command Packet (R)**

```text
  7-0  Data
```

Reading from 7000h (but not from 7001h-700Fh) does reset the flag in 6002h

Aside from regular SGB commands, the SGB BIOS (that in the SGB CPU chip) does
transfer six special packets upon Reset; these do contain gameboy cartridge
header bytes 104h..14Fh (ie. Nintendo Logo, Title, ROM/RAM Size, SGB-Enable
bytes, etc).

**SGB Port 7800h - Character Buffer Data (R)**

```text
  7-0  Data (320 bytes; from Buffer Row number selected in Port 6001h)
```

This port should be used as fixed DMA source address for transferring 320 bytes
(one 160x8 pixel character row) to WRAM (and, once when the SNES is in Vblank,
the whole 160x144 pixels can be DMAed from WRAM to VRAM).

The ICD2 chip does automatically re-arrange the pixel color signals (LD0/LD1)
back to 8x8 pixel tiles with two bit-planes (ie. to the same format as used in
Gameboy and SNES VRAM).

The buffer index (0..511) is reset to 0 upon writing to Port 6001h, and is
automatically incremented on reading 7800h. When reading more than 320 bytes,
indices 320..511 return FFh bytes (black pixels), and, after 512 bytes, it
wraps to index 0 within the same buffer row.

**Gameboy Audio**

The stereo Gameboy Audio Output is fed to the External Audio Input on SNES
cartridge port, so sound is automatically forwarded to the TV Set, ie. software
doesn't need to process sound data (however, mind that the /MUTE signal of the
SNES APU must be released).

**SGB Commands**

Above describes only the SNES side of the Super Gameboy. For the Gameboy side
(ie. for info on sending SGB packets, etc), see SGB section in Pan Docs:

```text
  http://problemkaputt.de/pandocs.htm
  http://problemkaputt.de/pandocs.txt
```

Some details that aren't described in (current) Pan Docs:

```text
 * JUMP does always destroy the NMI vector (even if it's 000000h)
 * (The SGB BIOS doesn't seem to use NMIs, so destroying it doesn't harm)
 * JUMP can return via 16bit retadr (but needs to force program bank 00h)
 * After JUMP, all RAM can be used, except [0000BBh..0000BDh] (=NMI vector)
 * The IRQ/COP/BRK vectors/handlers are in ROM, ie. only NMIs can be hooked
 * APU Boot-ROM can be executed via MOV [2140h],FEh (but Echo-Write is kept on)
 * The TEST_EN command points to a RET opcode (ie. it isn't implemented)
 * Upon RESET, six packets with gameboy cart header are sent by gameboy bios
 * command 19h does allow to change an undoc flag (maybe palette related?)
 * command 1Ah..1Fh point to RET (no function) (except 1Eh = boot info)
 * sgb cpu speed can be changed (unknown if/how supported by sgb bios)
```

**Note**

There is a special controller, the SGB Commander (from Hori), which does
reportedly have special buttons for changing the CPU speed - unknown how it is
doing that (ie. unknown what data and/or ID bits it is transferring to the SNES
controller port).

Probably done by sending button sequences (works also with normal joypad):

```text
 Codes for Super GameBoy Hardware
 Enter these codes very quickly for the desired effect.
  After choosing a border from 4 - 10, press L + R to exit.
   Press L, L, L, L, R, L, L, L, L, R. - Screen Savers
  At the Super Game Boy,
   press L, L, L, R, R, R, L, L, L, R, R, R, R, R, R, R - Super Gameboy Credits
  Hold UP as you turn on the SNES and then press L, R, R, L, L, R - Toggle Speed
  During a game, press L, R, R, L, L, R - Toggle Speed
  During a game, press R, L, L, R, R, L - Toggle Sound
  --
```

Screen Savers --&gt; Choose a border from 4 to 10 and press L + R to exit.
Press L(4), R, L(4), R.

Super Gameboy Credits --&gt; When you see the Super Game Boy screen appear,
press L, L, L, R, R, R, L, L, L, R, R, R, R, R, R, R

Toggle Speed (Fast, Normal, Slow, Very Slow)    Hold Up when powering up the
SNES, then press L, R, R, L, L, R very fast.

Toggle Speed (Normal, Slow, Very Slow)  During Gameplay, press L, R, R, L, L, R
very fast.

Un/Mute Sound --&gt; During Gameplay, press R, L, L, R, R, L quite fast.

<a id="snescartsatellaviewsatellitereceiverminiflashcard"></a>

## SNES Cart Satellaview (satellite receiver & mini flashcard)

**Satellaview I/O Ports**

[SNES Cart Satellaview I/O Map](#snes-cart-satellaview-io-map)

[SNES Cart Satellaview I/O Ports of MCC Memory Controller](#snes-cart-satellaview-io-ports-of-mcc-memory-controller)

[SNES Cart Satellaview I/O Receiver Data Streams](#snes-cart-satellaview-io-receiver-data-streams)

[SNES Cart Satellaview I/O Receiver Data Streams (Notes)](#snes-cart-satellaview-io-receiver-data-streams-notes)

[SNES Cart Satellaview I/O Receiver Control](#snes-cart-satellaview-io-receiver-control)

[SNES Cart Satellaview I/O FLASH Detection (Type 1,2,3,4)](#snes-cart-satellaview-io-flash-detection-type-1234)

[SNES Cart Satellaview I/O FLASH Access (Type 1,3,4)](#snes-cart-satellaview-io-flash-access-type-134)

[SNES Cart Satellaview I/O FLASH Access (Type 2)](#snes-cart-satellaview-io-flash-access-type-2)

**Satellaview Transmission Format**

[SNES Cart Satellaview Packet Headers and Frames](#snes-cart-satellaview-packet-headers-and-frames)

[SNES Cart Satellaview Channels and Channel Map](#snes-cart-satellaview-channels-and-channel-map)

[SNES Cart Satellaview Town Status Packet](#snes-cart-satellaview-town-status-packet)

[SNES Cart Satellaview Directory Packet](#snes-cart-satellaview-directory-packet)

[SNES Cart Satellaview Expansion Data (at end of Directory Packets)](#snes-cart-satellaview-expansion-data-at-end-of-directory-packets)

[SNES Cart Satellaview Other Packets](#snes-cart-satellaview-other-packets)

[SNES Cart Satellaview Buildings](#snes-cart-satellaview-buildings)

[SNES Cart Satellaview People](#snes-cart-satellaview-people)

[SNES Cart Satellaview Items](#snes-cart-satellaview-items)

**Satellaview Memory**

[SNES Cart Satellaview SRAM (Battery-backed)](#snes-cart-satellaview-sram-battery-backed)

[SNES Cart Satellaview FLASH File Header](#snes-cart-satellaview-flash-file-header)

[SNES Cart Satellaview BIOS Function Summary](#snes-cart-satellaview-bios-function-summary)

[SNES Cart Satellaview Interpreter Token Summary](#snes-cart-satellaview-interpreter-token-summary)

**Other Satellaview Info**

[SNES Cart Satellaview Chipsets](#snes-cart-satellaview-chipsets)

[SNES Pinouts BSX Connectors](80-timings-unpredictable-pinouts.md#snes-pinouts-bsx-connectors)

<a id="snescartsatellaviewiomap"></a>

## SNES Cart Satellaview I/O Map

**Receiver I/O Map (DCD-BSA chip)**

```text
  2188h Stream 1 Hardware Channel Number, Lsb (R/W)
  2189h Stream 1 Hardware Channel Number, Msb (R/W)
  218Ah Stream 1 Queue Size (number of received 1+22 byte Units) (R)
  218Bh Stream 1 Queue 1-byte Status Units (Read=Data, Write=Reset)
  218Ch Stream 1 Queue 22-byte Data Units  (Read=Data, Write=Reset/Ack)
  218Dh Stream 1 Status Summary (R)
  218Eh Stream 2 Hardware Channel Number, Lsb (R/W)
  218Fh Stream 2 Hardware Channel Number, Msb (R/W)
  2190h Stream 2 Queue Size (number of received 1+22 byte Units) (R)
  2191h Stream 2 Queue 1-byte Status Unit(s?) (Read=Data, Write=Reset)
  2192h Stream 2 Queue 22-byte? Data Unit(s?) (Read=Data, Write=Reset/Ack)
  2193h Stream 2 Status Summary (R)
  2194h POWER (bit0) and ACCESS (bit2-3) LED Control? (R/W)
  2195h Unknown/Unused, maybe for EXT Expansion Port (?)
  2196h Status (only bit1 is tested) (R)
  2197h Control (only bit7 is modified) (R/W)
  2198h Serial I/O Port 1 (R/W)
  2199h Serial I/O Port 2 (R/W)
```

**Flash Card I/O Map (when mapped to bank C0h and up)**

```text
  C00000h  Type 1-4   Detection Command             (W)
  C00002h  Type 1-4   Detection Status              (R)
  C0FFxxh  Type 1-4   Detection Response            (R)
  C00000h  Type 1,3,4 Command for Type 1,3,4        (W)
  C00000h  Type 1,3,4 Status (normal commands)      (R)
  C00004h  Type 1,3   Status (erase-entire command) (R)
  C02AAAh  Type 2     Command/Key for Type2         (W)
  C05555h  Type 2     Command/Status for Type2      (R/W)
  xx0000h  Type 1-4   Erase 64K Sector Address      (W)
  xxxxxxh  Type 1-4   Write Data Address            (W)
```

**BIOS Cartridge MCC-BSC Chip Ports**

```text
  005000h Unknown/Unused
  015000h Bank 00h-3Fh and 80h-FFh (0=FLASH, 1=PSRAM) (?)
  025000h Mapping for PSRAM/FLASH (0=32K/LoROM, 1=64K/HiROM)
  035000h Bank 60h-6Fh (0=FLASH, 1=PSRAM) (?)
  045000h Unknown (set when mapping PSRAM as Executable or Streaming Buffer)
  055000h Bank 40h-4Fh (0=PSRAM, 1=FLASH) ;\probably also affects Banks 00h-3Fh
  065000h Bank 50h-5Fh (0=PSRAM, 1=FLASH) ;/and maybe 80h-BFh when BIOS is off?
  075000h Bank 00h-1Fh (0=PSRAM/FLASH, 1=BIOS)
  085000h Bank 80h-9Fh (0=PSRAM/FLASH, 1=BIOS)
  095000h Unknown/Unused (except: used by BS Dragon Quest, set to 00h)
  0A5000h Unknown/Unused (except: used by BS Dragon Quest, set to 80h)
  0B5000h Unknown/Unused (except: used by BS Dragon Quest, set to 80h)
  0C5000h Bank C0h-FFh FLASH Reads? (0=Disable, 1=Enable)
  0D5000h Bank C0h-FFh FLASH Writes (0=Disable, 1=Enable)
  0E5000h Apply Changes to Other MCC Registers (0=Unused/Reserved, 1=Apply)
  0F5000h Unknown/Unused
```

Bits C and D are R/W (the other ones maybe, too).

<a id="snescartsatellaviewioportsofmccmemorycontroller"></a>

## SNES Cart Satellaview I/O Ports of MCC Memory Controller

**MCC I/O Ports**

The MCC chip is a simple 16bit register, with the bits scattered across various
memory banks (probably because the MCC chip doesn't have enough pins to decode
lower address bits).

To change a bit: [bit_number*10000h+5000h]=bit_value*80h

```text
  005000h Unknown/Unused
  015000h Bank 00h-3Fh and 80h-FFh (0=FLASH, 1=PSRAM) (?)
  025000h Mapping for PSRAM/FLASH (0=32K/LoROM, 1=64K/HiROM)
  035000h Bank 60h-6Fh (0=FLASH, 1=PSRAM) (?)
  045000h Unknown (set when mapping PSRAM as Executable or Streaming Buffer)
  055000h Bank 40h-4Fh (0=PSRAM, 1=FLASH) ;\probably also affects Banks 00h-3Fh
  065000h Bank 50h-5Fh (0=PSRAM, 1=FLASH) ;/and maybe 80h-BFh when BIOS is off?
  075000h Bank 00h-1Fh (0=PSRAM/FLASH, 1=BIOS)
  085000h Bank 80h-9Fh (0=PSRAM/FLASH, 1=BIOS)
  095000h Unknown/Unused
  0A5000h Unknown/Unused
  0B5000h Unknown/Unused
  0C5000h Bank C0h-FFh FLASH Reads? (0=Disable, 1=Enable)
  0D5000h Bank C0h-FFh FLASH Writes (0=Disable, 1=Enable)
  0E5000h Apply Changes to Other MCC Registers (0=Unused/Reserved, 1=Apply)
  0F5000h Unknown/Unused
```

Bits C and D are R/W (the other ones maybe, too, probably except bit E)

Bit 5,6 might also enable FLASH reads,writes in bank 40h-7Dh ?

**Satellaview BIOS Cartridge Memory Map**

```text
  00-0F:5000       MCC I/O Ports (Memory Control, BIOS/PSRAM/FLASH Enable)
  10-1F:5000-5FFF  SRAM             (32Kbyte SRAM in 4K-banks)
  xx-3F:6000-7FFF  PSRAM        (Mirror of 8K at PSRAM offset 06000h..07FFFh)
  00-3F:8000-FFFF  PSRAM/FLASH/BIOS in 32K-banks (Slow LoROM mapping)
  40-4F:0000-FFFF  PSRAM/FLASH  (for Executables with Slow HiROM mapping)
  50-5F:0000-FFFF  PSRAM/FLASH  (for Executables with Slow HiROM mapping)
  60-6F:0000-FFFF  FLASH/PSRAM  (for use as Work RAM or Data Files)
  70-77:0000-FFFF  PSRAM
  80-BF:8000-FFFF  PSRAM/FLASH/BIOS  in 32K-banks (Fast LoROM mapping)
  C0-FF:0000-FFFF  PSRAM/FLASH       (FLASH with R/W Access)
```

**Memory**

```text
  BIOS ROM  1MByte (LoROM mapping, 20h banks of 32Kbytes each)
  FLASH     1Mbyte   (can be mapped as LoROM, HiROM, or Work Storage)
  PSRAM     512Kbyte (can be mapped as LoROM, HiROM, or Work RAM)
  SRAM      32Kbyte (mapped in eight 4K banks)
```

Note: FLASH is on an external cartridge, size is usually 1MByte (as shown
above).

<a id="snescartsatellaviewioreceiverdatastreams"></a>

## SNES Cart Satellaview I/O Receiver Data Streams

The receiver can be programmed to watch (and receive) two different Hardware
Channels simultaneously. In practice, Stream 1 is used only by the BIOS, and
Stream 2 is used only by a few BS FLASH games (Dragon Quest 1, Satella2 1, BS
Fire Emblem Akaneia Senki 1, and maybe some others) (which do use it for
receiving Time Channel Packets).

**2188h/2189h Stream 1 Hardware Channel Number, Lsb/Msb (R/W)**

**218Eh/218Fh Stream 2 Hardware Channel Number, Lsb/Msb (R/W)**

```text
  0-15  Hardware Channel Number (16bit)
             XXX reportedly only 14bit !?
```

Values written to these registers should be taken from the Channel Map packet
(or for receiving the Channel Map itself, use fixed value 0124h). Be sure to
reset the Queues after changing the channel number (so you won't receive old
data from old channel).

**218Ah Stream 1 Queue Size (number of received 1+22 byte Units) (R)**

**2190h Stream 2 Queue Size (number of received 1+22 byte Units) (R)**

```text
  0-6  Number of received Units contained in the Queue (0..127)
  7    Overrun Error Flag (set when received more than 127 units)
```

Indicates how many frames are in the queues. One doesn't need to process all
frames at once; when reading only a few frames, the Queue Size is decremented
accordingly, and the remaining frames stay in the Queue so they can be
processed at a later time. The decrement occurs either after reading 1 byte
from the Status Queue, or after reading 22 bytes from the Data Queue (anyways,
to keep the queues in sync, one should always read the same amount of 1/22-byte
Units from both Queues, so it doesn't matter when the decrement occurs).

**218Bh Stream 1 Queue 1-byte Status Units (Read=Data, Write=Reset)**

**2191h Stream 2 Queue 1-byte Status Unit(s?) (Read=Data, Write=Reset)**

Contains Header/Data Start/End flags for the received Data Frames, the format
seems to be same as for Port 218Dh/2193h (see there for details) (if it's
really same, then the two Error bits should be also contained in the Status
Queue, though the BIOS doesn't use them in that place).

**218Ch Stream 1 Queue 22-byte Data Units (Read=Data, Write=Reset/Ack)**

**2192h Stream 2 Queue 22-byte? Data Unit(s?) (Read=Data, Write=Reset/Ack)**

Contains the received Data Frames, or in case of Header Frames: The 5/10-byte
Frame Header, followed by the Packet/Fragment Header, followed by the actual
Data.

**218Dh Stream 1 Status Summary (R)**

**2193h Stream 2 Status Summary (R)**

These registers seem to contain a summary of the Status bytes being most
recently removed from the Queue. Ie. status bits are probably getting set by
ORing all values being read from Port 218Ah/2190h. The bits are probably
cleared after reading 218Dh/2193h.

```text
  0-1  Unknown/unused
  2-3  Error Flags (probably set on checksum errors or lost data/timeouts)
  4    Packet Start Flag (0=Normal, 1=First Frame of Packet) (with Header)
  5-6  Unknown/unused
  7    Packet End Flag   (0=Normal, 1=Last Frame of Packet)
```

Bit 2-3 are more or less self-explaining: Don't use the queued data, and
discard any already (but still incompletely) received packet fragments. Bit 4,7
are a bit more complicated. See Notes in next chapter for details.

[SNES Cart Satellaview I/O Receiver Data Streams (Notes)](#snes-cart-satellaview-io-receiver-data-streams-notes)

<a id="snescartsatellaviewioreceiverdatastreamsnotes"></a>

### SNES Cart Satellaview I/O Receiver Data Streams (Notes)

**Resetting the Queues**

Clearing the Status &amp; Data Queues is needed on power-up, after Overrun, or
after changing the Hardware Channel number. The procedure is:

```text
  MOV A,01h     ;\
  MOV [218Bh],A ; must be executed in FAST memory (at 3.58MHz) (otherwise the
  NOP           ; the Status Queue may be not in sync with the Data Queue)
  NOP           ; (for Stream 2 do the same with Port 2192h/2193h accordingly,
  NOP           ; though the existing games that do use Stream 2 are including
  NOP           ; several near-excessive timing bugs in that section)
  MOV [218Ch],A ;/
```

Thereafter, Status &amp; Data queue are empty, and the Queue Size register is
00h (both 7bit counter, and Overrun flag cleared).

**Reading the Queues**

```text
  N=[218Ah]                                      ;-get queue size
  if N=0 then exit                               ;-exit if no data in queues
  if N.Bit7=1 then reset_queue/abort_packet/exit ;-handle overrun error
  N=max(20,N)                                    ;-limit to max 20 (if desired)
  for i=0 to (N-1), stat[i]=[219Bh], next        ;-read status units
  stat_summary=[219Dh]                           ;-get status summary
  for i=0 to (N*22-1), data[i]=[219Ch], next     ;-read data units
```

**Channel Disable**

After receiving a full packet, the BIOS issues a "MOV [218Ch],00h", this might
acknowledge something, or (more probably) disable the Channel so that no new
data is added to the Queue. The mechanism for re-enabling the channel is
unknown (prossibly resetting the Queue, or writing the Channel register). For
Stream 2, "MOV [2192h],00h" should do the same thing.

**Overrun Notes**

Overrun means that one hasn't processed the queues fast enough. If so, one
should Reset the queues and discard any already-received incomplete packet
fragments. There seems to be no problem if an overrun occurs WHILE reading from
the queue (ie. overrun seems to stop adding data to the queue, rather than
overwriting the old queued data) (of course AFTER reading the queue, one will
need to handle the overrun, ie. discard all newer data).

Note: Stream 1 can queue 127 frames (presumably plus 1 incomplete frame, being
currently received). As far as known, Stream 2 is used only for Time Channels
(with single-frame packets), so it's unknown if Stream 2 is having the same
queue size.

**Packet Start/End Flags**

The status queue values (with start/end bits isolated) would be:

```text
  90h               ;packet is 1 frame  (10-byte header + 12-byte data)
  10h,80h           ;packet is 2 frames (10-byte header + 34-byte data)
  10h,00h,80h       ;packet is 3 frames (10-byte header + 56-byte data)
  10h,00h,00h,80h   ;packet is 4 frames (10-byte header + 78-byte data)
```

and so on. For Channel Map, header is only 5-byte, and data is 5 bigger.

Caution: After having received the header (ie. at time when receiving the
data), the BIOS treats the Header-Start Flag in the Status Summary register (!)
as Error-Flag, to some point that makes sense in the Data-phase, but it will
cause an error if a new header frame is received shortly AFTER the Data-phase.
As a workaround, the transmitter should not send new Packet Fragments (on the
same Hardware Channel) for at least 1/60 seconds (preferably longer, say 1/20
seconds) after the end of the last Data Frame.

**Transfer Rate**

The transfer rate is unknown. Aside from the actual transmission speed, the
effective download rate will also depend on how often data is transmitted on a
specific channel (there are probably pauses between packet fragments, and maybe
also between 22-byte frames), this may vary depening on how many other packets
are transmitted, and how much priority is given to individual packets.

The download rate is slow enough for allowing the BIOS to write incoming data
directly to FLASH memory. Moreover, the BIOS Vblank NMI Handler processes only
max twenty 22-byte frames per 60Hz PPU frame. Knowing that, the download rate
must be definetly below 26400 bytes/second (20*22*60).

As far as known, the Satellaview broadcasts replaced former St.GIGA radio
broadcasts, assuming that the radio used uncompressed CD quality (2x16bit at
44.1kHz), and assuming that Satellaview used the same amount of data, the
transfer rate may have been 176400 bytes/second (which could have been divided
to transfer 8 different packets at 22050 bytes/second, for example).

<a id="snescartsatellaviewioreceivercontrol"></a>

## SNES Cart Satellaview I/O Receiver Control

**2194h POWER (bit0) and ACCESS (bit2-3) LED Control? (R/W)**

```text
  0   Usually set  <-- is ZERO by Itoi (maybe POWER LED) (see? 2196h.Bit0)
  1   Usually zero <-- is SET by Itoi                    (see? 2196h.Bit0)
  2-3 Usually both set or both cleared (maybe ACCESS LED) (Bit2 is Access LED)
  4-7 Usually zero
```

Bit2/3 are toggled by software when writing to FLASH memory. Bit0 is usually
set. Might control the POWER and ACCESS LEDs on the Satellaview's Front Panel
(assuming that the LEDs are software controlled). Using other values than
listed above might change the LED color (assuming they are two-color LEDs).

**2195h Unknown/Unused, maybe for EXT Expansion Port (?)**

This register isn't used by the BIOS, nor by any games. Maybe it does allow to
input/output data to the Satellaview's EXT Port.

**2196h Status (only bit1 is tested) (R)**

```text
  0    Unknown (reportedly toggles at fast speed when 2194h.Bit0-or-1? is set)
  1    Status (0=Okay, 1=Malfunction)
  2-7  Unknown/unused
```

The BIOS is using only Bit1, that bit is tested shortly after the overall
hardware detection, and also during NMI handling. Probably indicates some kind
of fundamental problem (like low supply voltage, missing EXPAND-Pin connection
in cartridge, or no Satellite Tuner connected).

**2197h Control (only bit7 is modified) (R/W)**

```text
  0-6  Unknown/unused (should be left unchanged)
  7    Power Down Mode? (0=Power Down, 1=Operate/Normal) (Soundlink enable?)
```

Bit7 is set by various BIOS functions, and, notably: When [7FD9h/FFD9h].Bit4
(in Satellaview FLASH File Header) is set. Also notably: Bit7 is set/cleared
depending on Town Status Entry[07h].Bit6-7.

**2198h Serial I/O Port 1 (R/W)**

**2199h Serial I/O Port 2 (R/W)**

These ports are basically 3-bit parallel ports, which can be used as three-wire
serial ports (with clock, data.in, data.out lines) (by doing the "serial"
transfer by software). Outgoing data must be written before toggling clock,
incoming data can be read thereafter.

```text
  0    Clock (must be manually toggled per data bit)
  1-5  Unknown/unused (should be 0)
  6    Chip Select - For Port 1: 1=Select / For Port 2: 0=Select
  7    Data (Write=Data.Out, Read=Data.in) (data-in is directly poll-able)
```

Bits are transferred MSB first.

Unknown which chips these ports are connected to. One port does most probably
connect to the 64pin MN88821 chip (which should do have a serial port; assuming
that it is a MN88831 variant). The other port &lt;might&gt; connect to the
small 8pin SPR-BSA chip?

Possible purposes might be configuration/calibration, Audio volume control, and
Audio channel selection (assuming that the hardware can decode audio data and
inject it to SNES Expansion Port sound inputs).

**Serial Port 1 (2198h)**

The BIOS contains several functions for sending multi-byte data to, and
receiving 16bit-units from this port. Though the functions seem to be left
unused? (at least, they aren't used in the low-level portion in first 32K of
the BIOS).

Port 1 specific notes: When reading (without sending), the outgoing dummy-bits
should be set to all zero. Chip is selected when Bit6=1. Aside from receiving
data from bit7, that bit is also polled in some cases for sensing if the chip
is ready (0=Busy, 1=Ready).

**Serial Port 2 (2199h)**

Data written to this port consists of simple 2-byte pairs (index-byte,
data-byte), apparently to configure some 8bit registers. Used values are:

```text
  Reg[0] = 88h (or 00h when Power-Down?) (soundlink on/off?)
  Reg[1] = 80h
  Reg[2] = 04h
  Reg[3] = 00h
  Reg[4] = 08h
  Reg[5] = 00h
  Reg[6] = 70h
  Reg[7] = Not used
  Reg[8] = 00h
  Reg[9..FF] = Not used
```

There are also BIOS functions for reading 1-byte or 3-bytes from this Port, but
they seem to be left unused (but, BS Dragon Quest, and Itoi are doing 24bit
reads via direct I/O, whereas Itoi wants the 1st bit to be 0=ready/okay).

Port 2 specific notes: When reading (without sending), the outgoing dummy-bits
should be set to all ones. Chip(-writing) is selected when Bit6=0.

<a id="snescartsatellaviewioflashdetectiontype1234"></a>

## SNES Cart Satellaview I/O FLASH Detection (Type 1,2,3,4)

The Satellaview FLASH cartridges contain slightly customized "standard" FLASH
chips; with a custom Nintendo-specific Chip Detection sequence:

**Detection Sequence**

```text
  [C00000h]=38h, [C00000h]=D0h                  ;request chip info part 1
  delay (push/pop A, three times each)          ;delay
  [C00000h]=71h                                 ;enter status mode
  repeat, X=[C00002h], until (X.bit7=1)         ;wait until ready
  [C00000h]=72h, [C00000h]=75h                  ;request chip info part 2
  FOR i=0 to 9, info[i]=BYTE[C0FF00h+i*2], NEXT ;read chip info (10 bytes)
  [C00000h]=FFh   ;somewhat bugged, see below   ;terminate status mode
```

Note: Nintendo Power flashcarts are also using very similar nonstandard FLASH
commands as above (there, for reading hidden mapping info, instead of for chip
detection).

BUG: For Type 2 chips, one &lt;should&gt; use "[C05555h]=AAh, [C02AAAh]=55h,
[C05555h]=F0h" instead of "[C00000h]=FFh" (the BIOS is actually &lt;trying&gt;
to do that, but it's doing it before having deciphered the Type bits).

**Detection Values**

```text
  info[0] - ID1 (Must be "M" aka 4Dh)
  info[1] - ID2 (Must be "P" aka 50h)
  info[2] - Flags (Must be bit7=0 and bit0=0) (other bits unknown)
  info[3] - Device Info (upper 4bit=Type, lower 4bit=Size)
  info[4..9] - Unknown/Unused (BIOS copies them to RAM, but doesn't use them)
```

Type must be 01h..04h for Type 1-4 accordingly. Size must be 07h..0Ch for
128Kbyte, 256Kbyte, 512Kbyte, 1Mbyte, 2Mbyte, 4Mbyte accordingly (ie. 1 SHL N
Kbytes).

**Rejected Values**

Wrong ID1/ID2 or wrong Flag Bit7/Bit0 are rejected. Type 00h or 05h..0Fh are
rejected. Size 00h..05h is rejected. Size 06h would be a half 128Kbyte block,
which is rounded-down to 0 blocks by the BIOS. Size 0Dh would exceed the 32bit
block allocation flags in header entry 7FD0h/FFD0h. Size 0Fh would additionally
exceed the 8bit size number.

**Special Cases**

If no FLASH cartridge is inserted, then the detection does probably rely on
open bus values, ie. "MOV A,[C00002h]" probably needs to return C0h (the last
opcode byte) which has bit7=1, otherwise the detection wait-loop would hang
forever.

There are reportedly some "write-protected" cartridges. Unknown what that
means, ROM-cartridges, or FLASH-cartridges with some (or ALL) sectors being
write-protected. And unknown what detection values they do return.

**FLASH Base Address**

The Satellaview BIOS always uses C00000h as Base Address when writing commands
to FLASH, the MCC chip could be programmed to mirror FLASH to other locations
(although unknown if they are write-able, if so, commands could be also written
to that mirrors).

Game Cartridges with built-in FLASH cartridge slot may map FLASH to other
locations than C00000h, details on that games are unknown. The game carts don't
include MCC chips, but other mapping hardware: In at least some of them the
mapping seems to be controlled by a SA-1 chip (an external 10.74MHz 65C816 CPU
with on-chip I/O ports, including memory-mapping facilities), the SA-1 doesn't
seem to have FLASH-specific mapping registers, so the FLASH might be mapped as
secondary ROM-chip. There may be also other games without SA-1, using different
FLASH mapping mechanism(s)? For some details, see:

[SNES Cart Data Pack Slots (satellaview-like mini-cartridge slot)](#snes-cart-data-pack-slots-satellaview-like-mini-cartridge-slot)

**General Notes**

FLASH erase sets all bytes to FFh. FLASH writes can only change bits from 1 to
0. Thus, one must normally erase before writing (exceptions are, for example,
clearing the "Limited-Start" bits in Satellaview file header). Type 2 can write
128 bytes at once (when writing less bytes, the other bytes in that area are
left unchanged). The status/detection values may be mirrored to various
addresses; the normal FLASH memory may be unavailable during
write/erase/detection (ie. don't try to access FLASH memory, or even to execute
program code in it, during that operations).

<a id="snescartsatellaviewioflashaccesstype134"></a>

## SNES Cart Satellaview I/O FLASH Access (Type 1,3,4)

The Type 1,3,4 protocol is somewhat compatible to Sharp LH28F032SU/LH28F320SK
chips (which has also a same/similar 52pin package). Concerning the commands
used by the BIOS, Type 1,3,4 seems to be exactly the same - except that Type 3
doesn't support the Erase-Entire chip command.

**Erase Entire Chip (Type 1 and 4 only) (not supported by Type 3)**

```text
  [C00000h]=50h                                 ;clear status register
  [C00000h]=71h                                 ;enter status mode
  repeat, X=[C00004h], until (X.bit3=0)         ;wait until VPP voltage okay
  [C00000h]=A7h  ;"erase all unlocked pages"?   ;select erase entire-chip mode
  [C00000h]=D0h                                 ;start erase
  [C00000h]=71h                                 ;enter status mode
  repeat, X=[C00004h], until (X.bit7=1)         ;wait until ready
  if (X.bit5=1) then set erase error flag       ;check if erase error
  [C00000h]=FFh                                 ;terminate status mode
```

**Unknown Command**

Same as Erase Entire (see above), but using 97h instead of A7h, and implemented
ONLY for Type 1 and 4, that is: NOT supported (nor simulated by other commands)
for neither Type 2 nor Type 3. Maybe A7h erases only unlocked pages, and 97h
tries to erase all pages (and fails if some are locked?).

**Erase 64KByte Sector**

```text
  [C00000h]=50h                                 ;clear status register
  [C00000h]=20h                                 ;select erase sector mode
  [nn0000h]=D0h                                 ;start erase 64K bank nn
  [C00000h]=70h                                 ;enter status mode
  repeat, X=[C00000h], until (X.bit7=1)         ;wait until ready
 ;; if (X.bit5=1) then set erase error flag       ;check if erase error
  [C00000h]=FFh                                 ;terminate status mode
```

**Write Data**

```text
  FOR i=first to last
    [C00000h]=10h                               ;write byte command
    [nnnnnnh+i]=data[i]                         ;write one data byte
    [C00000h]=70h                               ;enter status mode
    repeat, X=[C00000h], until (X.bit7=1)       ;wait until ready
  NEXT i
  [C00000h]=70h                                 ;enter status mode
  repeat, X=[C00000h], until (X.bit7=1)         ;hmmm, wait again
  if (X.bit4=1) then set write error flag       ;check if write error
  [C00000h]=FFh                                 ;terminate status mode
```

**Enter Erase-Status Mode**

```text
  [C00000h]=71h                                 ;enter status mode
  X=[C00004h]                                   ;read status byte
  IF (X.bit7=0) THEN busy
  IF (X.bit3=1) THEN not-yet-ready-to-erase (VPP voltage low)
  IF (X.bit7=1) AND (X.bit5=0) THEN ready/okay
  IF (X.bit7=1) AND (X.bit5=1) THEN erase error ?
```

**Enter Other-Status Mode**

```text
  [C00000h]=70h                                 ;enter status mode
```

**Terminate Command**

```text
  [C00000h]=FFh                                 ;terminate
```

Used to leave status or chip-detection mode.

BUGs: On Type 3 chips, the BIOS tries to simulate the "Erase-Entire" command by
issuing multiple "Erase-Sector" commands, the bug there is that it tests bit4
of the flash_size in 128Kbyte block units (rather than bit4 of the
flash_status) as erase-error flag (see 80BED2h); in practice, that means that
the erase-entire will always fail on 2MByte chips (and always pass okay on all
other chips); whereas, erase-entire is used when downloading files (except for
small files that can be downloaded or relocated to PSRAM).

<a id="snescartsatellaviewioflashaccesstype2"></a>

## SNES Cart Satellaview I/O FLASH Access (Type 2)

Type 2 protocol is completely different as for Type 1,3,4 (aside from the Chip
Detection sequence, which is same for all types).

**Erase Entire Chip**

```text
  [C05555h]=AAh, [C02AAAh]=55h, [C05555h]=80h   ;unlock erase
  [C05555h]=AAh, [C02AAAh]=55h, [C05555h]=10h   ;do erase entire chip
  [C05555h]=AAh, [C02AAAh]=55h, [C05555h]=70h   ;enter status mode
  repeat, X=[C05555h], until (X.bit7=1)         ;wait until ready
  if (X.bit5=1) then set erase error flag       ;check if erase error
  [C05555h]=AAh, [C02AAAh]=55h, [C05555h]=F0h   ;terminate status mode
```

**Erase 64KByte Sector**

```text
  [C00000h]=50h                                 ;huh? (maybe a BIOS bug)
  [C05555h]=AAh, [C02AAAh]=55h, [C05555h]=80h   ;unlock erase
  [C05555h]=AAh, [C02AAAh]=55h, [nn0000h]=30h   ;do erase bank nn
  repeat, X=[C05555h], until (X.bit7=1)         ;wait until ready
  if (X.bit5=1) then set erase error flag       ;check if erase error
  [C05555h]=AAh, [C02AAAh]=55h, [C05555h]=F0h   ;terminate status mode
```

**Write 1..128 Bytes (within a 128-byte boundary)**

```text
  [C05555h]=AAh, [C02AAAh]=55h, [C05555h]=A0h   ;enter write mode
  FOR i=first to last, [nnnnnn+i]=data[i]       ;write 1..128 byte(s)
  [nnnnnn+last]=DATA[last]   ;write LAST AGAIN  ;start write operation
  [C05555h]=AAh, [C02AAAh]=55h, [C05555h]=70h   ;enter status mode
  repeat, X=[C05555h], until (X.bit7=1)         ;wait until ready
  [C05555h]=AAh, [C02AAAh]=55h, [C05555h]=F0h   ;terminate status mode
```

**Enter Status Mode**

```text
  [C05555h]=AAh, [C02AAAh]=55h, [C05555h]=70h   ;enter status mode
  X=[C05555h]                                   ;read status byte
  IF (X.bit7=0) THEN busy
  IF (X.bit7=1) AND (X.bit5=0) THEN ready/okay
  IF (X.bit7=1) AND (X.bit5=1) THEN ready/erase error ?
```

**Terminate Command**

```text
  [C05555h]=AAh, [C02AAAh]=55h, [C05555h]=F0h   ;terminate
```

Used to leave status or chip-detection mode.

**Bugged Commands**

The are some cases (BIOS addresses 80BF88h, 80DBA4h, 80E0D5h) where Type 2
programming is mixed up with some non-Type-2 commands (setting [C00000h]=50h
and [C00000h]=FFh), these commands are probably ignored by chip (or switching
it back default mode).

<a id="snescartsatellaviewpacketheadersandframes"></a>

## SNES Cart Satellaview Packet Headers and Frames

**Packet Fragment Format (10-byte header)**

```text
  00h 1    Transmission ID (in upper 4bit) (must stay same for all fragments)
  01h 1    Current Fragment Number (in lower 7bit)
  02h 3    Fragment Size (N) (big-endian) (excluding first 5 bytes at [00..04])
  05h 1    Fixed, Must be 01h
  06h 1    Total Number of Fragments (00h=Infinite Streaming?)
  07h 3    Target Offset (big-endian) (location of fragment within whole file)
  0Ah N-5  Data Body (first 12 bytes located directly in header frame)
  ... ...  Unused/Padding (until begin of next 22-byte Frame)
```

This is the normal format used by all packets (except Channel Map packet).

**Channel Map Packet Format (5-byte header)**

```text
  00h 1    Unknown/unused (would be 4bit Transmission ID for normal packets)
  01h 1    Unknown/unused (would be 7bit Fragment Number for normal packets)
  02h 3    Packet Size (N) (big-endian) (excluding first 5 bytes at [00..04])
  05h N    Data Body (first 17 bytes located directly in header frame)
  ... ...  Unused/Padding (until begin of next 22-byte Frame)
```

**Frames (22-bytes)**

Packets are divided into one or more 22-byte frames.

For each frame, a 1-byte status info can be read from Port 218Bh/2191h, the
status contains error flags (indicating bad checksums or lost-frames or so),
and header flags (indicating begin/end of header/data or so).

The 22-byte frame data can be read from Port 218Ch/2192h. If it is header
frame, then its first 10 (or 5) bytes contain the header (as described above),
and the remaining 12 (or 17) bytes are data. If it is a data frame, then all 22
bytes are plain data.

**Fragmented Packets**

A packet can consist of 1..128 fragments. The packet transmission is repeated
several times (say, for one hour). If it is consisting of several fragments,
one can start downloading anywhere, eg. with the middle fragment, and one can
keep downloading even if some fragments had transmission errors (in both cases,
one must download the missing fragments in the next pass(es) when the
transmission is repeated).

Software should maintain a list of fragments that are already received (if a
fragment is already received: just remove it from the queue without writing to
target area; both for saving CPU load, and for avoiding to destroy previously
received packets in case of transmission errors).

At some time (say, after an hour), transmission of the packet will end, and a
different packet may be transferred on the same hardware channel. Verify the
4bit Transmission ID to avoid mixing up fragments from the old (desired) file
with the new file. Ideally, that ID &lt;should&gt; be stored in the Channel Map
or Directory (this may actually be so), however, the Satellaview BIOS simply
takes the ID from the first received Fragment, and then compares it against
following Fragments (if the ID changed shortly after checking the Directory,
and before receiving the first fragment, then the BIOS will download a "wrong"
file).

**Fragment Size Bug**

The Satellaview BIOS supports only 16bit fragment sizes, it does try to handle
24bit sizes, but if the fragment size exceeds 65535 then it does receive only
the first few bytes, and then treats the whole fragment as "fully" received.

**Text Strings**

Text strings (file/folder names and descriptions) can contain ASCII (and
presumably JIS and SHIFT-JIS), and following specials:

```text
  00h        End of Line (or return from a "\s" Sub-String)
  0Dh        Carriage Return Line Feed (in descriptions)
  20h..7Eh   ASCII 6pix characters (unlike SHIFT-JIS 12pix ones)
  80h..9Fh   Prefixes for double-byte characters (SHIFT-JIS)
  A0h..DFh   Japanese single-byte characters (JIS or so)
  E0h..EAh   Prefixes for double-byte characters (SHIFT-JIS)
  F0h        Prefix for Symbols (40h..51h:Music-Note,Heart,Dots,Faces,MaKenji)
  "\\"           Yen symbol (unlike ASCII, not a backslash)
  "\b0".."\b3"   Insert Username/Money/Gender/NumItems (12pix SHIFT-JIS)
  "\c0".."\c5"   Changes color or palette or so
  "\d#",p24bit   Insert 16bit Decimal at [p24bit] using 6pix-font
  "\D#",p24bit   Insert 16bit Decimal at [p24bit] using 12pix-font
  "\du#",v24bit  Insert 16bit Decimal Interpreter-Variable using 6pix-font
  "\Du#",v24bit  Insert 16bit Decimal Interpreter-Variable using 12pix-font
                    # = 00     Variable width (no leading spaces/zeroes)
                    # = 1..6   Width 1..6 chars (with leading spaces)
                    # = 01..06 Width 1..6 chars (with leading zeroes)
  "\s",ptr24bit  Insert Sub-string (don't nest with further "\s,\d,\D")
  "\g",ptr24bit  Insert Custom Graphics/Symbol (ptr to xsiz,ysiz,bitmap)
  "\i"           Carriage Return (set x=0, keep y=unchanged) (not so useful)
  "\m0".."\m3"   Flags (bit0=ForceHorizontal16pixGrid, bit1=DonNotUpdateBg3Yet)
  "\n"           Carriage Return Line Feed (same as 0Dh)
  "\p00".."\p07" Palette
  "\w00".."\w99" Character Delay in Frames (00=None)
  "\x00".."\xNN" Set Xloc
  "\y00".."\yNN" Set Yloc
```

Note: "\m","\p","\w","\x","\y" are slightly bugged (causing stack overflows
when using them too often within a single string; the text output thread quits
at the string-end, which somewhat 'fixes' the stack-problem).

CRLF can be used in Item Activation Messages, Folder or Download-File
Descriptions.

Multi-line messages are wrapped to the next line when reaching the end of a
line (the wrapping can occur anywhere within words, to avoid that effect one
must manually insert CRLF's (0Dh) at suitable locations). Some message boxes
are clipped to the visible number of lines, other messages boxes prompt the
user to push Button-A to read further lines.

Caution: CRLF will hang in Item-Descriptions (they do work in item shops, but
will hang in the Inventory menu; the only way to implement longer descriptions
here is to space-pad them so that the wrapping occurs at a suitable location).

<a id="snescartsatellaviewchannelsandchannelmap"></a>

## SNES Cart Satellaview Channels and Channel Map

**Channels**

Transmission is organized in "channels". Each can Channel transmits a single
Packet (which contains a File, or other special information like in Directory
and Time packets). There can be (theoretically) up to 4 billion logical
"Software Channels", but only 65536 physical "Hardware Channels". The latter
ones are those being currently transmitted, and which can be received by
programming the channel number into Port 2188h/218Eh.

**Channel Map Packet (Hardware Channel 0124h)**

Unlike normal packets (with 10-byte headers), this packet is preceeded by a
5-byte header.

Loaded to 7E9BECh. Unspecified size (size should be max 1485 bytes or less,
otherwise it'd overlap the Welcome Message at 7EA1B9h) (with that size limit,
the map can contain max 113 channels) (but, caution: Itoi works only with max
1007 bytes (1024-byte buffer, cropped to N*22-bytes, minus 5-byte packet
header)).

```text
  00h 2    ID 53h,46h ("SF")
  02h 4    Unknown/unused
  06h 1    Number of entries (must be at least 1)
  07h 1    Checksum (above 7 bytes at [00..06] added together)
  08h ..   Entries (each one is 3+N*13 bytes)
```

Each entry is: (Packet Groups)

```text
  00h 2    Software Channel Number (first 2 bytes, of total 4 bytes)
  02h 1    Number of sub-entries (N) (must be at least 1)
  03h N*13 Sub-entries (each one is 13 bytes)
```

Each sub-entry is: (Separate Packets)

```text
  00h 1    Unknown/unused
  01h 2    Software Channel Number (last 2 bytes, of total 4 bytes)
  03h 5    Unknown/unused
  08h 2    Fragment Interval (in seconds) (big-endian) (for use as timeout)
  0Ah 1    Type/Target (lower 4bit indicate transfer method or so)
            Bit0-1: Autostart after Download (0=No, 1=Optional, 2=Yes, 3=Crash)
            Bit2-3: Target (0=WRAM, 1=PSRAM, 2=EntireFLASH, 3=FreeFLASH)
            Bit4-7: Unknown/Unused
  0Bh 2    Hardware Channel Number (2 bytes) (for Port 2188h/218Eh)
```

The transmission timeout for the Channel Map itself is 7 seconds.

**Hardware Channels (2-byte / 16bit) (XXX or only 14bit?!)**

```text
  0121h     Used for hardware-connection test (received data is ignored)
  0124h     Channel Map
  AAEEh     Dummy number (often used to indicate an absent Time Channel)
  NNNNh     Other Hardware Channels (as listed in Channel Map)
  [7FFFF7h] Incoming Time Channel value for some games (from separate loader?)
```

**Software Channels (4-byte pairs / 32bit)**

```text
  1.1.0.4    Welcome Message (100 bytes)
  1.1.0.5    Town Status (256 bytes)
  1.1.0.6    Directory (16Kbytes)
  1.1.0.7    SNES Patch (16Kbytes)
  1.1.0.8    Time Channel (used by BS Satella2 1, BS Fire Emblem, and Itoi)
  1.2.0.48   Time Channel (used by Dragon Quest 1, BS Zelda no Densetsu Remix)
  ?.?.?.?    Time Channel (for BS Zelda - Kodai no Sekiban Dai 3 Hanashi)
  1.2.129.0  Special Channel used by Derby Stallion 96 <-- on roof of building
  1.2.129.16 Special Channel used by Derby Stallion 96 <-- 6th main menu option
  1.2.130.N  Special Channel(s) used by Itoi Shigesato no Bass Tsuri No. 1
  N.N.N.N    Other Software Channels (as listed in Directory)
  N.N.0.0    None (for directory entries that have no File or Include File)
```

**Endianess of Numbers in Satellite Packets**

In the satellite packets, all 16bit/24bit values (such like length or address
offsets) are in big-endian format (opposite of the SNES CPUs byte-order). For
the Hardware/Software Channel Numbers it's hard to say if they are meant to be
big-endian, little-endian, or if they are meant to be simple byte-strings
without endianess (see below for their ordering in practice).

**Endianess of Hardware Channels**

The endiness of the Hardware Channel numbers in Channel Map is same as in Port
2188h/218Eh. The endianess of the fixed values (0121h and 0124h) is also same
as Port 2188h/218Eh. Ie. one can use that values without needing to swap LSBs
and MSBs.

**Endianess of Software Channels**

The fixed 4-byte pairs are using same byte-order as how they are ordered in
Channel Map (for example, 1.2.0.48 means that 48 (30h) is at highest address).
So far it's simple. The (slightly) confusing part is that SNES software usually
encodes them as 2-word pairs (since the SNES uses a little-endian CPU, the
above example values would be 0201h.3000h).

<a id="snescartsatellaviewtownstatuspacket"></a>

## SNES Cart Satellaview Town Status Packet

**Town Status (Software Channel 1.1.0.5)**

Loaded to 7EA31Dh, then copied to 7EA21Dh. Size (max) 256 bytes.

Uses a normal 10-byte fragment header, but with "4bit Transmission ID" and
"7bit Fragment Number" ignored, but still does use "Target Offset" for
fragment(s)?

```text
  00h   1   Flags (bit0=1=Invalid) (bit1-7=unknown/unused)
  01h   1   Town Status ID (packet must be processed only if this ID changes)
  02h   1   Directory ID   (compared to Directory ID in Directory packet)
  03h   4   Unknown/unused
  07h   1   APU Sound Effects/Music & BSX Receiver Power-Down
             Bit0-3 Unknown/unused
             Bit4-5 APU (0=Mute, 1=Effects, 2=Effects/MusicA, 3=Effects/MusicB)
             Bit6   BSX (0=Normal, 1=Power-down with Port 2199h Reg[0]=88h)
             Bit7   BSX (0=Normal, 1=Power-down with Port 2199h Reg[0]=00h)
             (Or, maybe, the "Power-down" stuff enables satellite radio,
             being injected to audio-inputs on expansion port...?)
  08h   1   Unknown/unused
  09h   8   People Present Flags (Bit0-63) (max 5)        (LITTLE-ENDIAN)
  11h   2   Fountain Replacement & Season Flags (Bit0-15) (LITTLE-ENDIAN)
  13h   4   Unknown/unused
  17h   1   Number of File IDs (X) (may be 00h=none) (max=E8h)
  18h   X   File IDs (one byte each) (compared against File ID in Directory)
```

This packet should be (re-)downloaded frequently. The File IDs indicate which
Directory entries are valid (whereas, it seems to be possible to share the same
ID for ALL files), the Directory ID indicates if the Directory itself is still
valid.

**Fountain/Replacement and Season**

The animated Fountain (near beach stairs) has only decorative purposes (unlike
buildings/people that can contain folders). Optionally, the fountain can be
replaced by other (non-animated) decorative elements via Fountain Replacement
Flags in the Town Status packet: Bit0-11 are selecting element 1-12 (if more
than one bit is set, then the lowest bit is used) (if no bits are set, then the
fountain is used).

```text
  None  00h Default Fountain (default when no bits set) (animated)
  Bit0  01h Jan     Altar with Apple or so
  Bit1  02h Feb     Red Roses arranged as a Heart
  Bit2  03h Mar     Mohican-Samurai & Batman-Joker
  Bit3  04h Apr     Pink Tree
  Bit4  05h May     Origami with Fish-flag
  Bit5  06h Jun     Decorative Mushrooms
  Bit6  07h Jul     Palmtree Christmas with Plastic-Blowjob-Ghost?
  Bit7  08h Aug     Melons & Sunshade
  Bit8  09h Sep     White Rabbit with Joss Sticks & Billiard Balls
  Bit9  0Ah Oct     National Boule-Basketball
  Bit10 0Bh Nov     Red Hemp Leaf (cannabis/autum)
  Bit11 0Ch Dec     Christmas Tree (with special Gimmick upon accessing it)
```

As shown above, the 12 replacements are eventually associated with months
(Christmas in December makes sense... and Fish in May dunno why).

Bit12-15 of the above flags are selecting Season:

```text
  None  00h Default (when no bits set)
  Bit12 01h Spring  (pale green grass) (seems to be same as default)
  Bit13 02h Summery (poppy colors with high contrast)
  Bit14 03h Autumn  (yellow/brown grass)
  Bit15 04h Winter  (snow covered)
```

As for the Fountain, default is used when no bits set, and lowest bit is used
if more than one bit is set.

<a id="snescartsatellaviewdirectorypacket"></a>

## SNES Cart Satellaview Directory Packet

**Directory (Software Channel 1.1.0.6)**

Loaded to 7FC000h and then copied to 7EC000h. Size (max) 16Kbytes.

```text
  00h   1   Directory ID (compared to Directory ID in Town Status packet)
  01h   1   Number of Folders (Buildings, People, or hidden folders)
  02h   3   Unknown/unused
  05h   ..  Folders (and File) entries (see below)       ;\together
  ..    1   Number of Expansion Data Entries (00h=none)  ; max 3FFBh bytes
  ..    ..  Expansion Data Entries (see next chapter)    ;/
```

**Folder Entry Format**

```text
  00h   1   Flags (bit0=1=Invalid) (bit1-7=unknown/unused)
  01h   1   Number of File Entries (if zero: buildings are usually closed)
  02h   15h Folder Name (max 20 chars, terminated by 00h) (shown in building)
  17h   1   Length of Folder Message (X) (in bytes, including ending 00h)
  18h   X   Folder Message/Description (terminated by 00h)
  18h+X 1   More Flags (Folder Type)
             Bit0   : Folder Content (0=Download/Files, 1=Shop/Items)
             Bit1-3 : Folder Purpose (0=Building, 1=Person, 2=Include-Files)
               000b = Indoors (Building ID at [19h+X])     (Bit3-1=000b)   (0)
               x01b = Outdoors (Person ID at [19h+X])      (Bit2-1=01b)  (1,5)
               x1xb = Folder contains hidden Include Files (Bit2=1b) (2,3,6,7)
               100b = Unknown/unused (may be useable for "Files at Home"?) (4)
             Bit4-7 : Unknown/unused
  19h+X 1   Folder 6bit ID (eg. for Building:01h=News, for People:01h=Hiroshi)
  1Ah+X 1   Unknown/Unused
  1Bh+X 1   Unknown/Unused
  1Ch+X 1   Clerk/Avatar (00h..10h) (eg. 0Eh=Robot, 10h=BS-X) (11h..FFh=Crash)
  1Dh+X 1   Unknown/Unused
  1Eh+X 1   Unknown/Unused
  1Fh+X 1   Unknown/Unused
  20h+X ..  File/Item Entries (each one is 32h+X bytes; for items: fixed X=79h)
```

**File/Item Entry Format**

For Both Files and Items:

```text
  00h   1   File ID (compared to File IDs in Town Status Packet)
  01h   1   Flag (bit0=used by "Town Status" check (1=Not Available))
  02h   15h File Name (max 20 chars, terminated by 00h) (shown in building)
```

For Files: (in File Folders)

```text
  17h   1   Length of File Message (X) (in bytes, including ending 00h)
  18h   X   File Message/Description (terminated by 00h)
```

For Items: (in Item Folders)

```text
  17h   1   Length of Item Description+Activation+Price+Flag (X) (fixed=79h)
  18h   25h Item Description (max 36 chars, plus ending 00h)
  3Dh   47h Item Activation Message (min 1, max 70 chars, plus ending 00h)
  84h   12  Item Price (12-Digit ASCII String, eg. "000000001200" for 1200G)
  90h   1   Item Drop/Keep Flag (00h=Drop after Activation, 01h=Keep Item)
```

For Both Files and Items:

```text
  18h+X 4   Software Channel Number of Current File (for Items: N.N.0.0=None)
  1Ch+X 3   Big-Endian Filesize
  1Fh+X 3   Unknown/unused (except, first 2 bytes used by Derby Stallion 96)
  22h+X 1   Flags
              Bit0: used by "Town Status" check (1=Not Available)
              Bit1: Unknown/unused
              Bit2: Building Only (0=Also available at Home, 1=Building only)
              Bit3: Low Download Accuracy / Streaming or so
               0=High-Download-Accuracy (for programs or other important data)
               1=Low-Download-Accuracy (for audio/video data streaming)
              Bit4: Unused (except, must be 0 for Derby Stallion 96 files)
              Bit5-7: Unknown/unused
  23h+X 1   Unknown/unused
  24h+X 1   Flags/Target (seems to be same/similar as in Channel Map)
             Bit2-3:
              0=Download to WRAM (not really implemented, will crash badly)
              1=Download to PSRAM (without saving in FLASH)
              2=Download to Continous FLASH banks (erases entire chip!)
              3=Download to FREE-FLASH banks (relocate to PSRAM upon execution)
  25h+X 2   Unknown/unused
  27h+X 1   Date (Bit7-4=Month, Bit3-0=0)             ;\copied to Satellaview
  28h+X 1   Date (Bit7-3=Day, Bit2=0, Bit1-0=Unknown) ;/FLASH File Header FFD6h
  29h+X 1   Timeslot (Bit7-3=StartHours.Bit4-0, Bit2-0=Start Minutes.Bit5-3)
  2Ah+X 1   Timeslot (Bit4-0=EndHours.Bit4-0,   Bit7-5=Start Minutes.Bit2-0)
  2Bh+X 1   Timeslot (Bit7-2=EndMinutes.Bit5-0, Bit1-0=Unused)
  2Ch+X 4   Software Channel Number for additional Include File (N.N.0.0=None)
  30h+X 2   Unknown/unused
```

The directory may contain files that aren't currently transmitted; check the
Town Status for list of currently available File IDs. Also check the Directory
ID in the Town Status, if it doesn't match up with the ID of the Directory
packet, then one must download the Directory again (otherwise one needs to
download it only once after power-up).

**Include Files**

Each File in the directory has an Include File entry. Before downloading a
file, the BIOS checks if the Include File's Software Channel Number is listed
in the Directory (in any folder(s) that are marked as containing Include
Files). If it isn't listed (or if it's N.N.0.0), then there is no include file.
If it is listed, then the BIOS does first download the include file, and
thereafter download the original file. Whereas, the include file itself may
also have an include file, and so on (the download order starts with the LAST
include file, and ends with the original file).

There are some incomplete dumps of some games, which have 1Mbyte FLASH dumped,
but do require additional data in WRAM/PSRAM:

```text
  BS Dragon Quest (copy of channel_map in PSRAM, episode_number in WRAM)
  BS Zelda no Densetsu Kodai no Sekiban Dai 3 Hanashi (hw_channel in WRAM)
```

The missing data was probably transferred in form of Include files.

**Streaming Files**

There seems to be some streaming support:

Files flagged as FILE[22h+X].Bit3=1 are repeatedly downloaded-and-executed
again and again (possibly intended to produce movies/slideshows) (details: max
256K are loaded to upper half of PSRAM, and, if successfully received: copied
to lower 256K of PSRAM and executed in that place; whereas, the execution
starts once when receiving the next streaming block, that is: with a different
4bit transmission ID in the 10-byte packet header).

Moreover, Packets marked as having 00h fragments, are treated as having
infinite fragments (this would allow to overwrite received data by newer data;
though none of the higher-level BIOS functions seems to be using that feature).

**Files Available at Home**

Some files can be downloaded at the "Home" building (via the 3rd of the 4
options in that building), these "Home" files may be located in any folders,
namely, including following folders:

```text
  FOLDER[00h].Bit0=Don't care (folder may be marked as hidden)
  FOLDER[18h+X]=Don't care    (folder may be also used as building/person/etc.)
```

For downloading them at "Home", the files must be defined as so:

```text
  FILE[1Ah+X]<>0000h   (software channel isn't N.N.0.0)
  FILE[22h+X].Bit2=0   (flagged as available at home)
  FILE[18h]=Don't care (file description isn't shown at home)
```

BUG: If there are more than 32 of such "Home" files, then the BIOS tries to
skip the additional files, but destroys the stack alongside that attempt.

Note: Unlike other downloads, Home ones are always having a wooden-plank
background, and are done without transmission Interval timeouts. And, Include
Files are ignored. And, Autostart is disabled (ie. downloads to PSRAM are
totally useless, downloads to FLASH can/must be manually started).

<a id="snescartsatellaviewexpansiondataatendofdirectorypackets"></a>

## SNES Cart Satellaview Expansion Data (at end of Directory Packets)

**Directory (Software Channel 1.1.0.6)**

Loaded to 7FC000h and then copied to 7EC000h. Size (max) 16Kbytes.

```text
  00h   1   Directory ID (compared to Directory ID in Town Status packet)
  01h   1   Number of Folders (Buildings, People, or hidden folders)
  02h   3   Unknown/unused
  05h   ..  Folders (and File) entries (see previous chapter) ;\together
  ..    1   Number of Expansion Data Entries (00h=none)       ; max 3FFBh
  ..    ..  Expansion Data Entries                            ;/bytes
```

**Expansion Data Entry Format (after folder/file area)**

```text
  00h   1   Flags (bit0=1=Invalid) (bit1-7=unknown/unused)
  01h   1   Unknown/unused
  02h   2   Length (N) (16bit) (this one is BIG-ENDIAN)
  04h   N   Expansion Chunk(s) (all values in chunks are LITTLE-ENDIAN)
```

For for some reason, there may be more than one of these entries, but the BIOS
does use only the first entry with [00h].Bit0=0=Valid (any further entries are
simply ignored).

**Chunk 00h (End)**

```text
  00h        1   Chunk ID (00h)
```

Ends (any further following chunks are ignored). Chunk 00h should be probably
always attached at the end of the chunk list (although it can be omitted in
some cases, eg. after Chunk 02h).

**Chunk 01h (Custom Building)**

All values in this chunk are LITTLE-ENDIAN.

```text
  00h        1   Chunk ID (01h)
  01h        2   Chunk Length (73h+L1..L5) (LITTLE-ENDIAN)
  03h        11h Message Box Headline (max 16 chars, terminated by 00h)      ;\
  14h        20h BG Palette 5 (copied to WRAM:7E20A0h/CGRAM:50h)  (16 words) ;
  34h        38h BG Data (copied to WRAM:7E4C8Ch/BG1 Map[06h,0Dh])(4x7 words);/
  6Ch        2   Length of BG Animation Data (L1)                            ;\
  6Eh        L1  BG Animation Data (for Custom Building) (see below)         ;/
  6Eh+L1     2   Tile Length (L2) (MUST be nonzero) (zero would be 64Kbytes) ;\
                 BUG: Below Data may not start at WRAM addr 7Exx00h/7Exx01h  ;
                 (if so, data is accidently read from address-100h)          ;
  70h+L1     L2  Tile Data (DMAed to VRAM Word Addr 4900h) (byte addr 9200h) ;/
  70h+L1+L2  2   Length (L3) (should be even, data is copied in 16bit units) ;\
  72h+L1+L2  L3  Cell to Tile Xlat (copied to 7E4080h, BUG:copies L3+2 bytes);/
  72h+L1..L3 2   Length (L4) (MUST be even, MUST be nonzero, max 30h)        ;\
  74h+L1..L3 L4  Cell Solid/Priority List (copied to 7E45D0h,copies L4 bytes);/
  74h+L1..L4 2   Unknown/unused, probably Length (L5)                        ;\
  76h+L1..L4 L5  Door Location(s) (byte-pairs: xloc,yloc, terminated FFh,FFh);/
                 Door Locations work only if BG1 cells also have bit15 set!
                 Bit15 must be set IN FRONT of the door, this is effectively
                 reducing the building size from 4x7 to 4x6 cells.
                 Note: Animated BG cells are FORCEFULLY having bit15 cleared!
```

This chunk may be followed by Chunk 02h. If so, it processes Chunk 02h (and
ends thereafter). Otherwise it ends immediately (ie. when the following Chunk
has ID=00h) (or also on any "garbage" ID other than 02h).

**Chunk 02h (Custom Persons)**

```text
  00h        1   Chunk ID (02h)
  01h        2   Chunk Length (N) (LITTLE-ENDIAN) (N may be even,odd,zero)
  03h        N   Data (copied to 7F0000h) (N bytes) (max 0A00h bytes or so)
                  00h 4  7F0000h  Person 2Ch - Token Interpreter Entrypoint
                  04h 4  7F0004h  Person 2Dh - Token Interpreter Entrypoint
                  08h .. 7F0008h  General Purpose (Further Tokens and Data)
```

Copies the Data, and ends (any further following chunks are ignored). If Person
2Ch/2Dh are enabled in the Town Status packet, then the person thread(s) are
created with above entrypoint(s). The threads may then install whatever OBJs
(for examples, see the table at 99DAECh, which contains initial X/Y-coordinates
and Entrypoints for Person 00h..3Fh).

**Chunk 03h..FFh (Ignored/Reserved for future)**

```text
  00h        1   Chunk ID (03h..FFh)
  01h        2   Chunk Length (N) (LITTLE-ENDIAN)
  03h        N   Data (ignored/skipped)
```

Skips the data, and then goes on processing the following chunk.

**BG Animation Data (within Chunk 01h) (for Custom Building)**

```text
  00h     2   Base.xloc (0..47) (FFFFh=No Animation Data)
  02h     2   Base.yloc (0..47)
  04h     X*4 Group(s) of 2 words (offset_to_frame_data,duration_in_60hz_units)
  04h+X*4 2   End (FFFEh=Loop/Repeat animation, FFFFh=Bugged/One-shot animat.)
  06h+X*4 ..  BG Animation Frame Data Block(s) (see below)
  ..      0-2 Padding (to avoid 100h-byte boundary BUG in L2)
```

Xloc/Yloc should be usually X=0006h,Y=000Dh (the location of the Custom
Building, at 7E4C8Ch) (although one can mis-use other xloc/yloc values to
animate completely different map locations; this works only if the Custom
Building's folder contains files/items).

BUG: The one-shot animation is applied ONLY to map cells that are INITIALLY
visible (ie. according to BG scroll offsets at time when entering the town).

**BG Animation Frame Data Block(s)**

```text
  00h     Y*8 Group(s) of 4 words (xloc, yloc, bg1_cell, bg2_cell)
  00h+Y*8 2   End of Frame List (8000h) (ie. xloc=8000h=end)
```

NOTE: offset_to_frame_data is based at [94] (whereas, [94] points to the
location after Chunk 01h ID/Length, ie. to the base address of Chunk 01h plus
3).

If the animation goes forwards/backwards, one may use the same offsets for both
passes.

The Custom Bulding has 4x7 cells in non-animated form (so, usually one should
use xloc=0..3, yloc=0..6). Cells can be FFFFh=Don't change (usually one would
change only BG1 foreground cells, and set BG2 background cells to FFFFh). For
animated BG1 cells, Bit10-15 are stripped for some stupid reason (in result,
animated BG1 cells cannot be flagged as Doors via Bit15).

**BG Cell to Tile Translation**

Each 16x16 pixel Cell consists of four 8x8 Tiles. The Translation table
contains tiles arranged as Upper-Left, Lower-Left, Upper-Right, Lower-Right.
The 16bit table entries are 10bit BG tile numbers, plus BG attributes (eg.
xflip).

**Building/Map Notes**

The PPU runs in BG Mode 1 (BG1/BG2=16-color, BG3=4-color). VRAM used as so:

```text
  BG1 64x32 map at VRAM:0000h-07FFh, 8x8 tiles at VRAM:1000h-4FFFh (foreground)
  BG2 64x32 map at VRAM:0800h-0FFFh, 8x8 tiles at VRAM:1000h-4FFFh (background)
  BG3 32x32 map at VRAM:5000h-53FFh, 8x8 tiles at VRAM:5000h-6FFFh (menu text)
  OBJ 8x8 and 16x16 tiles at VRAM:6000h-7FFFh (without gap)        (people)
  Custom BG1/BG2 Tiles are at VRAM:4900h-xxxxh (BG.Tile No 390h-xxxh)
  Custom OBJ Tiles at VRAM:7C00h-xxxxh (OBJ.Tile No 1C0h..xxxh)
```

The PPU BG Palettes are:

```text
  BG.PAL0 Four 4-Color Palettes (for Y-Button BG3 Menu)
  BG.PAL1 Four 4-Color Palettes (for Y-Button BG3 Menu)
  BG.PAL2 Buildings
  BG.PAL3 Buildings
  BG.PAL4 Buildings
  BG.PAL5 Custom Palette
  BG.PAL6 Landscape (Trees, Phone Booth)         (colors changing per season)
  BG.PAL7 Landscape (Lawn, Streets, Water, Sky)  (colors changing per season)
```

The town map is 48x48 cells of 16x16 pix each (whole map=768x768pix).

```text
  Custom BG1/BG2 Cells are at WRAM:7E4080h-7E41FFh (Cell No 3D0h-3FFh)
  Custom Cell Solid/Priority...
```

<a id="snescartsatellaviewotherpackets"></a>

## SNES Cart Satellaview Other Packets

**SNES Patch Packet (by 105BBC) (Software Channel 1.1.0.7)**

Loaded to 7FC000h, data portions then copied to specified addresses. Size (max)
16Kbytes.

```text
  00h  1   Number of entries (01h..FFh) (MUST be min 01h)
  01h  ..  Entries (max 3FFFh bytes)
```

Each entry is:

```text
  00h  1   Flags (bit0=1=Invalid) (bit1-7=unknown/unused)
  01h  1   Unknown/unused
  02h  2   Length (N) (16bit, big-endian) (MUST be min 0001h)
  04h  3   SNES-specific Memory Address (24bit, big-endian)
  07h  N   Data (N bytes)
```

The data portions are copied directly to the specified SNES addresses (the BIOS
doesn't add any base-offset to the addresses; ie. if the data is to be copied
to WRAM, then the satellite would transmit 7E0000h..7FFFFFh as address; there
is no address checking, ie. the packet can overwrite stack or I/O ports).

**Welcome Message Packet (Software Channel 1.1.0.4)**

Loaded to 7EA1B9. Size max 64h bytes (100 decimal). Displayed in text window
with 37x4 ASCII cells.

```text
  00h  100 Custom Message (max 99 characters, plus ending 00h)
```

If this packet is included in the Channel Map (and if it's successfully
received), then the Custom Message is displayed right before entering the town.
The japanese Default Message is still displayed, too (so one gets two messages
- which is causing some annoying additional slowdown).

**Time Channel Packet (Software channel 1.1.0.8) (BS Fire Emblem)**

**Time Channel Packet (Software channel 1.1.0.8) (BS Satella2 1)**

**Time Channel Packet (Software channel 1.1.0.8) (BS Parlor Parlor 2)**

**Time Channel Packet (Software channel 1.1.0.8) (BS Shin Onigashima 1)**

**Time Channel Packet (Software channel 1.1.0.8) (BS Tantei Club)**

**Time Channel Packet (Software channel 1.1.0.8) (BS Kodomo Tyosadan..)**

**Time Channel Packet (Software channel 1.1.0.8) (BS Super Mario USA 3)**

**Time Channel Packet (Software channel 1.1.0.8) (BS Super Mario Collection 3)**

**Time Channel Packet (Software channel 1.1.0.8) (BS Excitebike - Mario .. 4)**

**Time Channel Packet (Software channel 1.1.0.8) (Itoi Shigesato no Bass Tsuri)**

**Time Channel Packet (Software channel 1.2.0.48) (BS Dragon Quest 1)**

**Time Channel Packet (Software channel 1.2.0.48) (BS Zelda .. Remix)**

**Time Channel Packet (HW channel [7FFFF7h]) (BS Zelda - Kodai .. Dai 3 ..)**

**Time Channel Packet (HW channel [7FFFF7h]) (BS Marvelous Camp Arnold 1)**

**Time Channel Packet (HW channel [7FFFF7h]) (BS Marvelous Time Athletic 4)**

Preceeded by a 10-byte packet header. Of which, first 5 bytes are ignored (Body
is 8-bytes, so packet size should be probably 5+8). Fixed 01h must be 01h.
Number of Fragments must be 01h. Target Offset must be 000000h. The 8-byte Data
Body is then:

```text
  00h  1  Unknown/unused (probably NOT seconds) ;(un-)used by Itoi only
  01h  1  Minutes     (0..3Bh for 0..59)
  02h  1  Hours       (0..17h for 0..23) (or maybe 24..26 after midnight)
  03h  1  Day of Week (01h..07h) (rather not 00h..06h) (?=Monday)
  04h  1  Day         (01h..1Fh for 1..31)
  05h  1  Month       (01h..0Ch for 1..12)
  06h  1  Unknown/unused (maybe year)    ;\could be 2x8bit (00:00 to 99:99)
  07h  1  Unknown/unused (maybe century) ;/or maybe 16bit (0..65535) or so
```

Caution: The BS satellite program specified hours from 11..26 (ie. it didn't
wrap from 23 to 0), the Time Channel(s) might have been following that
notation; if so, then Date values might have also been stuck on the previous
day. Unknown if the Time Channels have been online 24 hours a day, or if their
broadcast ended at some time late-night.

Time Channel are often used by Soundlink games, in so far, it's also quite
possible that the "hours:minutes" are referring to the time within the
broadcast (eg. from 0:00 to 0:59 for a 1-hour broadcast duration), rather than
to a real-time-clock-style time-of-the-day.

Differences between 1.1.0.8 and 1.2.0.48 are unknown. Some games require
incoming Hardware channel number at [7FFFF7h] (from a separate loader or so).

One would expect TIME[0] to contain seconds - however, there aren't any games
using that entry as seconds (instead, they wait for minutes to change, and
reset seconds to zero; due to that wait-for-next-minute mechanism, many BSX
games seem to "hang" for up to 60 seconds after booting them).

TIME[4..7] aren't really used as "date" by any games (a few games seem to be
using TIME[4] or TIME[5] to determine how often the user has joined the game).
Itoi uses (and displays) TIME[4..5] as date.

Some games are treating TIME[6..7] as a 16bit little-endian value, others are
using only either TIME[6] or TIME[7] (ie. half of the games that use the "year"
values are apparently bugged).

**File Packet (Software Channel N.N.N.N as taken from Directory)**

```text
  00h  N   Data, N bytes (N=Filesize as specified in Directory)
```

The file must contain a Satellaview Transmit Header (at file offset 7Fxxh or
FFxxh), that header is similar to the FLASH File Header. For details &amp;
differences, see:

[SNES Cart Satellaview FLASH File Header](#snes-cart-satellaview-flash-file-header)

The filesize should be usually a multiple of 128Kbytes (because the checksum in
File Header is computed accross that size). Transmitting smaller files is
possible with some trickery: For FLASH download, assume FLASH to be erased (ie.
expand checksum to FFh-filled 128Kbyte boundary). For PSRAM download, the
checksum isn't verified (so no problem there). Moreover, filesize should be
usually at least 32Kbytes (for LoROM header at 7Fxxh), however, one could
manipulate the fragment-offset in packet header (eg. allowing a 4Kbyte file to
be loaded to offset 7000h..7FFFh).

**Special Channel(s) used by Itoi Shigesato no Bass Tsuri No. 1 (1.2.130.N)**

Itoi supports four special channel numbers to unlock special contests. The game
doesn't try to receive any data on that channels (it only checks if one of them
is listed in the Channel Map).

```text
  1.2.130.0      ;\Special Contests 1..4 (or so)
  1.2.130.16     ; The 4 contests are looking more or less the same, possibly
  1.2.130.32     ; with different parameters, different japanese descriptions,
  1.2.130.48     ;/contest 2-3 have "No fishing" regions in some lake-areas.
  1.2.130.other  ;-Invalid (don't use; shows error with TV-style "test screen")
```

For Itoi, the Channel Map may be max 1019 bytes (or even LESS?!) (unlike
usually, where it could be 1485 bytes). There should be only one channel with
value 1.2.130.N in the Channel Map. The game additionally requires the 1.1.0.8
Time Channel. And, uses the "APU" byte in the Town Status packet.

**1.2.129.0  Special Channel used by Derby Stallion 96 (Dish on Building/Roof)**

**1.2.129.16 Special Channel used by Derby Stallion 96 (6th Main Menu Option)**

Differences between the two channels are unknown; both are processed by the
same callback function (and thus seem to have the same data-format). The
packet(s) are both loaded to 7F0000h, size could be max 8000h (although, actual
size might be 7E00h, as indicated by the checksum calculation). The overall
format is:

```text
  0000h 3    Unknown/unused (3 bytes)
  0003h 8    ID "SHVCZDBJ" (compared against [B289D6])
  000Bh 2    Number of Chunks at 0010h and up (16bit) (little-endian) (min 1)
  000Dh 1    Unknown/unused? (8bit)
  000Eh 1    Checksum (bytes at [0000h..7DFFh] added together) (8bit)
  000Fh 1    Checksum complement (8bit)
  0010h DF0h Chunks
  7E00h 200h Begin of non-checksummed area (IF ANY) (if it DOES exist,
             then it MIGHT contain a file-style header at 7FB0h..7FFFh ?)
```

Note: The Chunks are processed via function at B28A10h. Despite of the
hardcoded channel numbers, the packets must be also listed (with the same
channel numbers) in the Directory Packet (as hidden Include File entries or
so).

**Hardware Channel 0121h - Test Channel**

Used for hardware-connection test (received data is ignored). Used by BIOS
function 105B6Ch; which is used only when pressing X+L+R buttons while the
Welcome Message is displayed. Transmission Timeout for the Test Channel is 10
seconds.

<a id="snescartsatellaviewbuildings"></a>

## SNES Cart Satellaview Buildings

**Home Building (Starting Point)**

This building can be entered at any time, the four japanese options are:

```text
  1) Load File from FLASH Card
  2) Delete File from FLASH Card
  3) Download File (only files that are "Available at Home") (max 32 files)
  4) Delete Settings in SRAM
```

**Buildings**

The buildings are numbered roughly anti-clockwise from 00h..1Fh, starting at
lower-left of the town map:

```text
  00h Robot Skyscraper (lower-left)
  01h News Center
  02h Parabol Antenna
  03h Junkfood
  04h Police
  05h Maths +-x/
  06h Beach Shop (Shop for Predefined ROM Items)
  07h Turtle-Arena
  08h C-Skyscaper (Shop for Predefined ROM Items)
  09h Red-Heart Church
  0Ah Red \\\ Factory (upper-right corner)
  0Bh Dracula Gift-Shop
  0Ch Cow-Skull Church
  0Dh Spintop/Abacus (near Maths +-X/)
  0Eh Blank Skyscraper (near Parabol Antenna)
  0Fh Sign (near Red Factory) (works only ONCE)     (or custom building)
  10h Greek Buddah Temple (upper-end)
  11h Bigger Neighbor's Building
  12h Smaller Neighbor's Building (unknown how to get in there)
  13h Phone Booth (can be entered only with Telephone Card item)
  14h Sewerage (near Spintop) (Shop for Predefined ROM Items)
  15h Unused
  16h Unused ;\these Building-Folders MUST not exist (else BIOS randomly
  17h Unused ;/crashes, accidently trying to animate Building number "44h/3")
  18h Special Location without folder: Player's Home
  19h Special Location without folder: Hydrant (near police)
  1Ah Special Location without folder: Talking Tree (near C-Skyscraper)
  1Bh Special Location without folder: Fountain (or Fountain Replacement)
  1Ch Special Location without folder: Beach Toilets (Railway Station)
  1Dh Special Location without folder: Ocean's Shore
  1Eh Special Location without folder: Unused
  1Fh Special Location without folder: Unused
  20h-3Fh Building-Folders with these IDs do destroy memory (don't use!)
```

Buildings can be entered only if there is a corresponding folder (in the
directory), and only if the folder contains at least one file or item (for the
three pre-defined Shops it works also if the folder is empty).

<a id="snescartsatellaviewpeople"></a>

## SNES Cart Satellaview People

**People**

People are showing up only if they are flagged in the 64bit People Present
Flags in the Town Status packet. If more than 5 people are flagged, then only
the first 5 are shown (regardless of that limit, the 4 frogs and the ship can
additionally be there).

```text
  00h Red Ball (on beach) (disappears after access)
  01h Spring-boot (aka Dr.Hiroshi's Shop) (near news center) (sells items)
  02h General Pee (showing up here and there pissing against buildings)
  03h Brown Barbarian on Cocaine (near temple)
  04h Blue Depressive Barbarian (near temple)
  05h Ghost Waver (near phone booth)
  06h Boy in Neighborhood
  07h Older Elvis (near churches)
  08h Purple Helmet (on beach)
  09h Surfer (near beach shop)
  0Ah Grayhaired (northwest lawn)
  0Bh Alien Man (near phone booth)
  0Ch Uncle Bicycle (near parabol antenna)
  0Dh Circus Man (near temple/lake)
  0Eh Speedy Blind Man (near parabol antenna/spintop)
  0Fh Blonde Boy (near factory)
  10h Girl with Pink Dress (near Bigger Neighbor's Home)
  11h Brunetty Guy (near Bigger Neighbor's Home)
  12h Brunette (near junkfood)
  13h Darkhaired (near junkfood)
  14h Blue Longhair (near junkfood)
  15h Brunette Longhair (near junkfood)
  16h Brunette Longhair (near red-heart church)
  17h Green Longhair (near red-heart church)
  18h Bicycle Girl (near C-Skyscraper)
  19h Brunette Office Woman (near C-Skyscraper)
  1Ah Blue Longhair (near parabol antenna)
  1Bh Turquoise Longhair (near home)
  1Ch Blue Longhair (near maths/spintop)
  1Dh Brunette Longhair (near news center/beach stairs)
  1Eh Black Longhair (near police)
  1Fh Red Longhair (southeast beach)
  20h Blackhaired Girl (near police)
  21h Greenhaired Girl (on bench between temple and lake)
  22h Graybluehaired older Woman (east of C-skyscraper)
  23h Darkhaired Housewife (near home)
  24h Traditional Woman (west of Robot-Skyscraper)
  25h Greenhaired Girl (near Turtle-Arena)
  26h Pinkhaired Girl (near Cow-Skull Church)
  27h Brown Dog (northeast lawn)
  28h White Dog (near home)
  29h Gray Duck (near temple/lake)
  2Ah Portable TV-Headed Guy (near Robot-Skyscraper)
  2Bh Satellite Wide-screen TV-Headed Guy (near Robot-Skyscraper)
  2Ch Custom Person 2Ch  ;\may be enabled only if defined in Expansion Area
  2Dh Custom Person 2Dh  ;/of Directory Packet (otherwise crashes)
```

Below 2Eh-37h are specials which cannot have a Folder assigned to them:

```text
  2Eh Dead Dentist (on bench near lake) (gives Money when owning Fishing Pole)
  2Fh Gimmick: Allows to use Bus/Taxi/Ferrari Tickets at Fountain
  30h Gimmick: Allows to use Express/Museum-Train-Tickets at Railways Station
  31h Gimmick: Special Event when accessing the Hydrant
  32h Frog 32h (west of Robot-Skyscraper)       ;Change Identity Item
  33h Frog 33h (west of Robot-Skyscraper, too)  ;Change GUI Border Scheme
               (or on street near turtle arena?)
  34h Frog 34h (northwest lawn)                 ;Change GUI Color Scheme
  35h Frog 35h (near Cow-Skull Church)          ;Change GUI Cursor Shape
  36h Gimmick: Allows to use Whale/Dolphin/Fish Food at Oceans Shore
  37h Ship (cannot be accessed?) (near factory)
  38h Mr.Money (near police) (donates a 500G coin)  ;\only one can be present
  39h Mr.Money (near police) (donates a 1000G coin) ; at once, after the coin,
  3Ah Mr.Money (near police) (donates a 5000G coin) ;/all do act as Folder 38h
  3Bh-3Fh Unused?
```

Like Buildings, People can have a Folder associated to them (using above values
as Folder ID, at least, that works for People 00h..2Bh).

If there is no corresponding folder transmitted, then People aren't doing
anything useful (aside from producing whatever japanese sentences). One
exception: If there's no People-File-Folder with ID=01h, then the Spring-boot
guy acts as "Dr Hiroshi's Shop" where one can buy question-marked items for
3000G. The Frogs can be picked up (adding an Item to the inventory). Frog
positions seem to be random (west of Robot-Skyscraper, street near Turtle
Arena, northwest lawn, or near Cow-Skull Church).

**Avatars**

These are assigned for folders (either for people in buildings, or people on
the streets).

```text
  00h Geisha (faecher)
  01h Snorty (nose bubble)
  02h Gold hat
  03h Naked guy
  04h Soldier (lanze)
  05h Whore (lipstick/eye blinking)
  06h Wise man1 (huge white eyebrows)
  07h Wise man2 (huge stirn, huge ears)
  08h DJ Proppy (headphones, sunglasses, muscles)
  09h Casino manager (fat guy with red slip-knot)
  0Ah Student girl (manga, karierte bluse)
  0Bh School girl (satchel ranzen/smiley)
  0Ch Kinky gay (green hair, sunglasses, gold-ohrring)
  0Dh Yankee (glistening teeth/dauerwelle)
  0Eh Robot (blech-mann)
  0Fh Blonde chick (blonde, lipstick)
  10h BS-X logo (not a person, just the "BS-X" letters)
  11h-FFh Unknown/Unused/Crashes
  30h None (seems to be an un-intended effect, probably unstable, don't use)
```

<a id="snescartsatellaviewitems"></a>

## SNES Cart Satellaview Items

**Predefined Items (and their 24bit memory pointers)**

Items sold in C-Skyscraper:

```text
 00 88C229h Transfer Device (allows to teleport to any building) (unlimited)
 01 88C2B8h Telephone Card (5) (allows to enter phone booth) ;\
 02 88C347h Telephone Card (4) (allows to enter phone booth) ; decreases
 03 88C3D6h Telephone Card (3) (allows to enter phone booth) ; after usage
 04 88C465h Telephone Card (2) (allows to enter phone booth) ;
 05 88C4F4h Telephone Card (1) (allows to enter phone booth) ;/
 06 88C583h Fishing Pole (allows to get Money from Dead Dentist, Person 2Eh)
 07 88C612h Express Train Ticket                  ;\these are treated special
 08 88C6A1h Museum Train Ticket                   ;/by code at 88936Ch
 09 88C630h Bus Ticket (at Fountain)    ;\these all have same description
 0A 88C7BFh Taxi Ticket                 ;
 0B 88C84Eh Ferrari Blowjob Ticket      ;/
```

Items sold by Dr.Hiroshi (spring-boot guy near News Center)

```text
 0C 88C8DDh Doping Item (walk/run faster when pushing B Button)
 0D 88C96Ch Unknown (disappears after usage)
```

Items sold in Beach Shop:

```text
 0E 88C9FBh Whale Food (can be used at Oceans Shore)
 0F 88CA8Ah Dolphin Food (can be used at Oceans Shore)
 10 88CB19h Fish Food (can be used at Oceans Shore)
```

Items sold in Sewerage:

```text
 11 88CBA8h Boy/Girl Gender Changer (can be used only once)
 12 88CC37h Transform Boy/Girl into Purple Helmet guy (Person 08h)(temporarily)
 13 88CCC6h Transform Boy/Girl into Brunette chick    (Person 1Dh)(temporarily)
 14 88CD55h Smaller Neighbor's Home Door Key (allows to enter that building)
```

Items obtained when picking-up Frogs:

```text
 15 88CDE4h Change Identity (edit user name) (from Frog 32h) (works only once)
 16 88CE73h Change GUI Border Scheme         (from Frog 33h) (works only once)
 17 88CF02h Change GUI Color Scheme          (from Frog 34h) (works only once)
 18 88CF91h Change GUI Cursor Shape          (from Frog 35h) (works only once)
```

**Item Format**

As shown above, 25 items are defined in ROM at 99C229h-88D020h with 8Fh bytes
per item. Custom Items (defined in Directory packet's "File" entries) can be
stored at 10506Ah. The item format is:

```text
  00h 15h Item Name (max 20 chars, plus ending 00h) (First 2 bytes 00h = Free)
  15h 1   Length of following (Description, Pointer, Whatever) (always 79h?)
  16h 25h Item Description (max 36 chars, plus ending 00h)
  3Bh 47h Item Activation Message (max 70 chars, plus ending 00h)
   If Activation Message = empty (single 00h byte), then Item Function follows:
   3Ch 3   Pointer to Interpreter Tokens (eg. 99974Dh for Transfer Device)
   3Fh 43h Unknown/Unused/Padding (should be zero)
   (there is no SRAM allocated for custom item functions,
   so this part may be used only for predefined ROM items)
  82h 12  Item Price (12-Digit ASCII String, eg. "000000001200" for 1200G)
  8Eh 1   Item Drop/Keep Flag (00h=Drop after Activation, 01h=Keep Item)
```

In case of Custom Items, above ITEM[00h..8Eh] is copied from FILE[02h..90h]
(ie. a fragment of "File" Entries in the Directory Packet).

Entry [15h] seems to be always 79h, giving a total length of 8Fh per item.

The Item Message is used for items that cannot be activated (eg. "You can't use
telephone card outside of the phone booth.",00h). If the message is empty
(00h), then the next 24bit are a pointer to the item handler (eg. the Teleport
function for the Transfer Device).

Note: Items can be listed, activated, and dropped via Y-Button. The teleport
device can be also activated via X-button.

**Shops**

There are four pre-defined shops: Dr.Hiroshi's appears when Person 01h exists,
WITHOUT folder assigned, or WITH an item-folder. The Beach Shop, C-Skyscraper
and Sewerage Shops appear if they HAVE an folder assigned, the folder must be
flagged as Item/Shop. In all cases, the folder may contain additional items
which are added to the Shop's predefined item list. Custom Shops can be created
by assigning Item-Folders to other People/Buildings (in that case, the Folder
MUST contain at least one item, otherwise the BIOS shows garbage). Shops may
contain max 0Ah items (due to 7E865Eh array size).

<a id="snescartsatellaviewsrambatterybacked"></a>

## SNES Cart Satellaview SRAM (Battery-backed)

The Satellaview BIOS cartridge contains 32Kbyte battery-backed SRAM, mapped to
eight 4Kbyte chunks at 5000h..5FFFh in Bank 10h..17h.

**SRAM Map**

```text
  0000h 2    ID "SG" (aka 53h,47h)
  0002h 2    Checksum Complement (same as below checksum XORed with FFFFh)
  0004h 2    Checksum (bytes 0..2FFFh added together) (assume [2..5]=0,0,FF,FF)
  0006h 20   User's Name (Shift-JIS)
  001Ch 2    User's Gender (0000h=Boy, 0001h=Girl)
  001Eh 6    Money (max 00E8D4A50FFFh; aka 999,999,999,999 decimal)
  0024h 2    Number of Items (0..10h) (or temporarily up to 11h)
  0026h 44h  Item Entries (4-bytes each: Type=00/01=ROM/RAM, and 24bit pointer)
  006Ah 8F0h Custom RAM Items (8Fh bytes each) (First 2 bytes 00h = free entry)
  095Ah 2    Remaining Time on Doping Item (decreases when entering buildings)
  095Ch 2    Number of Doping Items (walk/run faster when pushing B Button)
  095Eh 2    Remaining Calls on first Telephone Card Item minus 1
  0960h 2    Number of Telephone Card Items (unlocks Phone Booth)
  0962h 2    Number of Transfer Devices (enables Teleport via menu or X-Button)
  0964h 2    Number of Fishing Poles (allows to get Money from Dead Dentist)
  0966h 2    Number of Smaller Neighbor's Home Keys (0000h=Lock, other=Unlock)
  0968h 2    GUI Cursor Shape 16bit selector (0000h..0005h) (other=crash)
  096Ah 3    GUI Border Scheme 24bit pointer (def=9498D9h) (MUST be 94xxxxh)
  096Dh 3    GUI Color Scheme 24bit pointer (initially 94A431h)
  0970h 1    Player got 500 coin from Person 38h      ;\(00h=No, 01h=Yes,
  0971h 1    Player got 1000 coin from Person 39h     ; flags stay set until
  0972h 1    Player got 5000 coin from Person 3Ah     ; that Person leaves
  0973h 1    Player picked-up Red Ball aka Person 00h ;/the town)
  0974h 18h  BIOS Boot/NMI/IRQ Hook Vectors (retf's) (mapped to 105974h and up)
  098Ch 2B0h BIOS Function Hook Vectors (jmp far's) (mapped to 10598Ch and up)
  0C3Ch 64h  BIOS Reset Function (and some zero-filled bytes)
  0CA0h 100h BIOS Interpreter Token Handlers (16bit addresses in bank 81h)
  0DA0h 100h Garbage Filled (reserved for unused Tokens number 80h..FFh)
  0EA0h 2    Garbage Filled (for impossible 8bit token number 100h)
  0EA2h 215Eh Reserved (but, mis-used for game positions by some games)
  3000h 3000h Backup Copy of 0..2FFFh
  6000h 2000h General Purpose (used for game positions by various games)
```

**Game Positions in SRAM**

```text
  0000h-0EA1h  BX-X BIOS (see above)
  1400h-14FFh  BS Super Mario USA 3 (256 bytes)
  1500h-15FFh  BS Super Mario Collection 3 (256 bytes)
  1500h-15FFh  BS Kodomo Tyosadan Mighty Pockets 3 (256 bytes)
  1600h-1626h  BS Satella Walker 2 (27h bytes)
  1600h-1626h  BS Satella2 1 (27h bytes)
  1700h-17FFh  BS Excitebike Bun Bun Mario Battle Stadium 4 (256 bytes)
  2000h-27FFh  BS Marvelous Camp Arnold Course 1 (2Kbytes)
  2800h-2F89h  BS Dragon Quest 1 (1.9Kbytes) (probably 1K, plus 1K backup copy)
  2006h-2FF5h  BS Zelda no Densetsu Remix (3.9Kbytes)
  2000h-2EFFh  BS Zelda no Densetsu Kodai no Sekiban Dai 3 Hanashi (3.75K)
  2000h-2FFFh  BS Super Famicom Wars (V1.2) (first 4K of 8Kbytes)
  3000h-5FFFh  Backup Copy of 0..2FFFh (not useable for other purposes)
  6000h-63FFh  BS Treasure Conflix (1Kbyte)
  6020h-62F9h  BS Sutte Hakkun 98 Winter Event Version (0.7Kbytes)
  6000h-7FFFh  BS Chrono Trigger - Jet Bike Special (8Kbytes)
  6800h-6FFFh  BS Super Famicom Wars (V1.2) (middle 2K of 8Kbytes)
  7500h-7529h  BS Cu-On-Pa (2Ah bytes)
  7800h-7FFFh  BS Super Famicom Wars (V1.2) (last 2K of 8Kbytes)
  7826h-7827h  BS Dr. Mario (only 2 bytes used?)
 (7C00h-7FFFh) BS Radical Dreamers (default, if free, at 7C00h) (1Kbyte)
```

There is no filesystem with filenames nor SRAM allocation. Most games are using
hardcoded SRAM addresses, and do overwrite any other data that uses the same
addresses.

One exception is BS Radical Dreamers: The game searches for a free (zerofilled)
1K block (defaults to using 7C00h-7FFFh), if there isn't any free block, then
it prompts the user to select a 1K memory block (at 6000h-7FFFh) to be
overwritten.

Some games are saving data in PSRAM (eg. Zelda no Densetsu: Kamigami no
Triforce, in bank 70h) rather than SRAM - that kind of saving survives Reset,
but gets lost on power-off.

There aren't any known BS games that save data in FLASH memory. Some BS games
are using passwords instead of saving.

**BIOS Hooks**

These are 4-byte fields, usually containing a "JMP 80xxxxh" opcode (or a "RETF"
opcode in a few cases), changing them to "JMP 1x5xxxh" allows to replace the
normal BIOS functions by updated functions that are installed in SRAM. Unknown
if any such BIOS updates do exist, and at which SRAM locations they are
intended/allowed to be stored.

**SRAM Speed**

The Satellaview SRAM is mapped to a FAST memory area with 3.58MHz access time
(unlike ALL other SNES RAM chips like internal WRAM or external SRAM in other
cartridges).

The bad news is that this wonderful FAST memory isn't usable: It's located in
Bank 10h-17h, so it isn't usable as CPU Stack or CPU Direct Page (both S and D
registers can access Bank 00h only). And, the first 24Kbytes are used as
reserved (and checksummed) area.

<a id="snescartsatellaviewflashfileheader"></a>

## SNES Cart Satellaview FLASH File Header

**Satellaview FLASH File Header**

Located at offset 7Fxxh or FFxxh in file, mapped to FFxxh in bank 00h.

```text
  FFB0h 2  Maker Code (2-letter ASCII)                   ;\garbage when
  FFB2h 4  Program Type (00000100h=Tokens, Other=65C816) ; [FFDBh]=01h
  FFB6h 10 Reserved (zero)                               ;/
  FFC0h 16 Title (7bit ASCII, 8bit JIS (?), and 2x8bit SHIFT-JIS supported)
  FFD0h 4  Block Allocation Flags (for 32 blocks of 128Kbytes each) (1=used)
              Retail (demo) games usually have ffff here -- Uh ???
              (exception BS Camp Arnold Marvelous)       -- Uh ???
  FFD4h 2  Limited Starts (bit15=0=Infinite, otherwise bit14-0=Remaining Flags)
  FFD6h 1  Date (Bit7-4=Month, Bit3-0=0)             ;\copied to from
  FFD7h 1  Date (Bit7-3=Day, Bit2=0, Bit1-0=Unknown) ;/Directory Packet
  FFD8h 1  Map Mode (20h=LoROM, 21h=HiROM) (or sometimes 30h/31h)
  FFD9h 1  File/Execution Type
            Bit0-3  Unknown/unused (usually/always 0)
            Bit4    Receiver Power Down (0=No/Sound Link, 1=Power-Down 2197h)
            Bit5-6  Execution Area (0=FLASH, 1=Reloc FLASH-to-PSRAM, 2/3=Fail)
            Bit7    Skip the "J033-BS-TDM1 St.GIGA" Intro (0=Normal, 1=Skip)
  FFDAh 1  Fixed (33h)
  FFDBh 1  Unknown (usually 02h, sometimes 01h, or rarely 00h) (see FFBxh)
  FFDCh 2  Checksum complement (same as below, XORed with FFFFh)
  FFDEh 2  Checksum (all bytes added together; assume [FFB0-DF]=00h-filled)
  FFE0h 32 Exception Vectors (IRQ,NMI,Entrypoint,etc.) (for 65C816 code)
```

Entrypoint is at 800000h+[FFFCh] for 65C816 Machine Code, or at 400000h for
Interpreter Tokens (ie. when [FFB2h]=00000100h, as used by few magazines like
BS Goods Press 6 Gatsu Gou).

Caution: Machine Code programs are started WITHOUT even the most basic
initialization (one of the more bizarre pieces: the download "screensaver" may
leave HDMAs enabled when auto-starting a downloaded file).

**Satellaview PSRAM File Header (download to PSRAM without saving in FLASH)**

Basically same as FLASH headers. FFD9h.Bit7 (skip intro) seems to be ignored
(accidently using FFD9h from FLASH cartridge instead from PSRAM?). The checksum
isn't verified (FFDEh must match with FFDCh, but doesn't need to match the
actual file content).

**Satellaview Transmit Header (located at 7Fxxh or FFxxh in File)**

Basically same as normal Satellaview FLASH File Header. However, after
downloading a file (and AFTER storing it in FLASH memory), the BIOS does
overwrite some Header entries:

```text
  FFD0h  4-byte  Block Allocation field (set to whichever used FLASH Blocks)
  FFD6h  2-byte  Date field (set to Date from Satellite Directory Entry)
  FFDAh  1-byte  Fixed Value (set to 33h)
```

Since FLASH cannot change bits from 0 to 1 (without erasing), the above values
must be FFh-filled in the transmitted file (of course, for the fixed value, 33h
or FFh would work) (and of course, the FFh-requirement applies only to FLASH
downloads, not PSRAM downloads).

**Notes**

The title can be 16 bytes (padded with 20h when shorter), either in 7bit ASCII,
8bit JIS (or so?), or 2x8bit SHIFT-JIS, or a mixup thereof. A few files are
somewhat corrupted: Title longer than 16 bytes (and thereby overlapping the
Block Allocation flags).

Limited Starts consists of 15 flags (bit14-0), if the limit is enabled (bit15),
then one flag is changed from 1-to-0 each time when starting the file (the
1-to-0 change can be done without erasing the FLASH sector).

Note that the checksum excludes bytes at FFB0h-FFDFh, this is different as in
normal cartridges (and makes it relative easy to detect if a file contains a
SNES ROM-image or a Satellaview FLASH-file/image.

Files are treated as "deleted" if their Fixed Value isn't 33h, if Limited
Starts is 8000h (limit enabled, and all other bits cleared), or if Checksum
Complement entry isn't equal to Checksum entry XOR FFFFh. Some FLASH dumps in
the internet do have experired Limited Starts entries (so they can be used only
when changing [FFD4h] to a value other than 8000h).

The FLASH card PCBs can be fitted with 1/2/4 MByte chips (8/16/32 Mbit). As far
as known, all existing FLASH cards contain 1MByte chips (so Block Allocation
bit8-31 should be usually always 0). Most files are occupying the whole 1MByte
(so bit0-7 should be usually all set). There are also some 256Kbyte and
512Kbyte files (where only 2 or 4 bits would be set). Minimum file size would
be 128Kbyte. Odd sizes like 768Kbytes would be also possible.

Unlike the Satellite Packet Headers, the FLASH/Transmit-File header contains
"normal" little-endian numbers.

<a id="snescartsatellaviewbiosfunctionsummary"></a>

## SNES Cart Satellaview BIOS Function Summary

BIOS functions must be called with BIOS ROM enabled in Bank 80h-9Fh (in some
cases it may be required also in Bank 00h-1Fh), and with DB=80h.
Incoming/outgoing Parameters are passed in whatever CPU registers and/or
whatever WRAM locations. WRAM is somewhat reserved for the BIOS (if a FLASH
file changes WRAM, then it should preserve a backup-copy in PSRAM, and restore
WRAM before calling BIOS functions) (WRAM locations that MAY be destroyed are
7E00C0h..7E00FFh and 7E1500h..7E15FFh, and, to some level: 7F0000h..7FFFFFh,
which is used only as temporary storage).

**Hooks (usually containing RETF opcodes)**

```text
  105974 boot_hook (changed by nocash fast-boot patch)
  105978 nmi_hook
  10597C irq_vector
  105980 download_start_hook --> see 9B8000
  105984 file_start_hook --> see 958000
  105988 whatever_hook --> see 99xxxx
```

**SRAM Vectors**

```text
  10598C detect_receiver
  105990 port_2194_clr_bit0
  105994 port_2196_test_bit1
```

**Copy Data Queue to RAM Buffer**

```text
  105998 set_port_218B_and_218C_to_01h
  10599C set_port_218C_to_00h
  1059A0 read_data_queue
```

**Port 2199h (serial port 2) (maybe satellite audio related)**

```text
  1059A4 init_port_2199_registers
  1059A8 send_array_to_port_2199  ;BUGGED?
  1059AC recv_3x8bit_from_port_2199
  1059B0 send_16bit_to_port_2199
  1059B4 recv_8bit_from_port_2199
```

**Port 2198h (serial port 1) (unused/expansion or so)**

```text
  1059B8 port_2198_send_cmd_recv_multiple_words
  1059BC port_2198_send_cmd_recv_single_word
  1059C0 port_2198_send_cmd_send_verify_multiple_words
  1059C4 port_2198_send_cmd_send_verify_single_word
  1059C8 port_2198_send_cmd_send_single_word
  1059CC port_2198_send_10h_send_verify_single_word
  1059D0 port_2198_send_cmd_verify_FFFFh
  1059D4 port_2198_send_20h_verify_FFFFh
  1059D8 recv_2198_skip_x BUGGED!
  1059DC recv_2198_want_x
  1059E0 send_30h_to_port_2198
  1059E4 send_00h_to_port_2198
  1059E8 send_8bit_to_port_2198
  1059EC wait_port_2198_bit7
```

**Forward Data Queue from RAM to Target**

```text
  1059F0 forward_data_queue_to_target
  1059F4 forward_queue_to_wram
  1059F8 forward_queue_to_psram
  1059FC forward_queue_to_entire_flash
  105A00 forward_queue_to_entire_flash_type1
  105A04 forward_queue_to_entire_flash_type2
  105A08 forward_queue_to_entire_flash_type3
  105A0C forward_queue_to_entire_flash_type4
  105A10 forward_queue_to_flash_sectors
  105A14 forward_queue_to_flash_sectors_type1
  105A18 forward_queue_to_flash_sectors_type2
  105A1C forward_queue_to_flash_sectors_type3
  105A20 forward_queue_to_flash_sectors_type4
  105A24 forward_queue_to_channel_map  ;with 5-byte frame-header
  105A28 forward_queue_to_town_status
```

**FLASH Files**

```text
  105A2C scan_flash_directory
  105A30 allocate_flash_blocks
  105A34 .. prepare exec / map file or so
  105A38 verify_file_checksum
  105A3C get_flash_file_header_a
  105A40 delete_flash_file_a
  105A44 get_flash_file_header_5A
  105A48 copy_file_header
  105A4C search_test_file_header, out:[57]
  105A50 test_gamecode_field
  105A54 copy_file_to_psram
  105A58 get_file_size
  105A5C decrease_limited_starts
```

**Memory Mapping**

```text
  105A60 map_flash_as_data_file  (for non-executable data-files?)
  105A64 map_psram_as_data_file  (for non-executable data-files?)
  105A68 .. mapping and copy 512Kbytes ?
  105A6C map_flash_for_rw_access
  105A70 map_flash_for_no_rw_access
  105A74 map_flash_for_reloc_to_psram
  105A78 .. mapping (unused?)
  105A7C map_flash_as_lorom_or_hirom
  105A80 execute_game_code
  105A84 .. map_psram_for_streaming ???
  105A88 map_psram_as_lorom_or_hirom
  105A8C .. copy 256Kbytes...
```

**FLASH Memory**

```text
  105A90 flash_abort
  105A94 flash_abort_type1
  105A98 flash_abort_type2
  105A9C flash_abort_type3
  105AA0 flash_abort_type4
  105AA4 flash_erase_entire
  105AA8 flash_erase_entire_type1
  105AAC flash_erase_entire_type2
  105AB0 flash_erase_entire_type4 ;4!
  105AB4 flash_erase_entire_type3
  105AB8 flash_test_status   ERASE-PROGRESS
  105ABC flash_test_status_type1
  105AC0 flash_test_status_type2
  105AC4 flash_test_status_type4 ;4!
  105AC8 flash_test_status_type3
  105ACC flash_erase_first_sector
  105AD0 flash_erase_first_sector_type1
  105AD4 flash_erase_first_sector_type2
  105AD8 flash_erase_first_sector_type3
  105ADC flash_erase_first_sector_type4
  105AE0 flash_erase_next_sector
  105AE4 flash_erase_next_sector_type1
  105AE8 flash_erase_next_sector_type2
  105AEC flash_erase_next_sector_type3
  105AF0 flash_erase_next_sector_type4
  105AF4 flash_write_byte
  105AF8 flash_write_byte_type1
  105AFC flash_write_byte_type2
  105B00 flash_write_byte_type3
  105B04 flash_write_byte_type4
  105B08 flash_get_free_memory_size
  105B0C flash_get_and_interprete_id
  105B10 flash_get_id
  105B14 flash_init_chip
  105B18 flash_init_chip_type1
  105B1C flash_init_chip_type2
  105B20 flash_init_chip_type3
  105B24 flash_init_chip_type4
```

**Satellite Directory**

```text
  105B28 apply_satellite_directory
  105B2C directory_find_8bit_folder_id
  105B30 directory_find_32bit_file_channel
  105B34 test_if_file_available
  105B38 download_file_and_include_files
  105B3C directory_find_32bit_bugged
```

**Misc...**

```text
  105B40 .. initialize stuff on reset
  105B44 download_nmi_handling (with download_callback etc.)
  105B48 download_nmi_do_timeout_counting
  105B4C nmi_do_led_blinking
  105B50 mark_flash_busy
  105B54 mark_flash_ready
  105B58 set_port_2197_bit7
  105B5C clr_port_2197_bit7
  105B60 detect_receiver_and_port_2196_test_bit1
  105B64 init_flash_chip_with_err_29h
  105B68 init_flash_chip_with_err_2Ah
  105B6C detect_receiver_and_do_downloads
  105B70 do_download_function
  105B74 retry_previous_download
  105B78 set_target_id_and_search_channel_map
  105B7C apply_target_for_download
  105B80 clear_queue_and_set_13D1_13D2
  105B84 flush_old_download   ;[218C]=0, clear some bytes
```

**Invoke Download Main Functions**

```text
  105B88 download_to_whatever (BUGGED)
  105B8C download_channel_map
  105B90 download_welcome_message
  105B94 download_snes_patch
  105B98 download_town_status
  105B9C download_town_directory
  105BA0 download_to_memory
```

**Download sub functions**

```text
  105BA4 add_download_array
  105BA8 wait_if_too_many_downloads
  105BAC do_download_callback
  105BB0 dload_channel_map_callback_1
  105BB4 dload_channel_map_callback_2
  105BB8 dload_welcome_message_callback
  105BBC dload_snes_patch_callback
  105BC0 dload_town_status_callback_1
  105BC4 dload_town_status_callback_2
  105BC8 dload_town_directory_callback_1
  105BCC dload_town_directory_callback_2
  105BD0 .. flash status
  105BD4 dload_to_mem_wram_callback1          ;\
  105BD8 dload_to_mem_wram_callback2          ;
  105BDC dload_to_mem_psram_callback1         ;
  105BE0 dload_to_mem_psram_callback2         ; dload_to_memory_callbacks
  105BE4 dload_to_mem_entire_flash_callback1  ;
  105BE8 dload_to_mem_entire_flash_callback2  ;
  105BEC dload_to_mem_free_flash_callback1    ;
  105BF0 dload_to_mem_free_flash_callback2    ;/
  105BF4 dload_to_mem_entire_flash_callback_final
  105BF8 dload_to_mem_free_flash_callback_final
  105BFC reset_interpreter_and_run_thread_958000h
  105C00 verify_channel_map_header
  105C04 raise_error_count_check_retry_limit
  105C08 search_channel_map
  105C0C post_download_error_handling
  105C10 .. erase satellite info ?
```

**APU Functions**

```text
  105C14 apu_flush_and_clear_queues
  105C18 apu_flush_raw
  105C1C apu_message
  105C20 apu_nmi_handling
  105C24 apu_upload_extra_thread
  105C28 apu_upload_curr_thread
  105C2C apu_enable_effects_music_b
  105C30 apu_enable_effects_music_a
  105C34 apu_mute_effects_and_music
  105C38 apu_enable_effects_only
```

**Reset**

```text
  105C3C reboot_bios (this one works even when BIOS=disabled or WRAM=destroyed)
```

**Further Stuff**

```text
  105C96 Unused 7 bytes (used for nocash fast-boot patch)
  105C9D Unused 3 bytes (zero)
  105CA0 Token Vectors (16bit offsets in bank 81h)
```

**BIOS Tables**

```text
  105xxx Tables in SRAM (see above)
  808000 Unsorted ptrs to BIOS Functions, Token-Extensions, and OBJ-Tile-Data
  9FFFF0 Pointers to source data for APU uploads
```

**Additional BIOS Functions (without SRAM-Table vectors)**

These are some hardcoded BIOS addresses (used by some FLASH programs).

```text
  808C2A Invoke_dma_via_ax_ptr
  8091B6 Create_machine_code_thread
  809238 Pause_machine_code_thread
  80938F Do nothing (retf) (used as dummy callback address)
  80ABC8 ...whatever
  80AC01 ...whatever
  80B381 Upload_gui_border_shape_to_vram
  80B51B Clear_text_window_content
  80B91E Fill_400h_words_at_7E76000_by_0080h   ;clear whole BG3 map in WRAM
  80EB99 Injump_to_APU_Town_Status_handling (requires incoming pushed stuff)
  81C210 Reset_interpreter
  81C29A Set_interpreter_enable_flag
  81C2B0 Create_interpreter_thread
  81C80E Deallocate_all_obj_tiles_and_obj_palettes
```

Note: Some of the above functions are also listed in the table at 808000h.

**Returning to BIOS**

If an executable file wants to return control to BIOS, it must first reset the
APU (if it has uploaded code to it), and then it can do one the following:

Perform a warmboot (the BIOS intro is skipped, but the Welcome message is
re-displayed, and the player is moved back to the Home building):

```text
  jmp 105C3Ch   ;srv_reboot_bios (simple, but quite annoying)
```

Or return to the BIOS NMI handler (from within which the executable was
started) this is done by many games (player returns to the most recently
entered building, this is more elegant from the user's view, though requires a
messy hack from the programmer's view):

```text
  call restore_wram             ;-restore WRAM (as how it was owned by BIOS)
  jmp  far (($+4) AND 00FFFFh)  ;-PB=00h (so below can map BIOS to bank 80h)
  mov  a,80h   ;\                               ;\
  push a ;=80h ;                                ; set DB=80h, and
  pop  db      ;/                               ; enable BIOS in bank 80h-9Fh
  mov  [085000h],a ;map BIOS to bank 80h-9Fh    ; (though not yet in 00h-1Fh)
  mov  [0E5000h],a ;apply                       ;/
  call far 99D732h  ;super-slow ;out: M=0       ;-upload [9FFFF0h] to APU
 .assume p=10h  ;(above set M=0, and keeps X=unchanged)
  call far 81C210h                              ;-Reset Token Interpreter
  call far 81C29Ah                              ;-Enable/Unpause Interpreter
  call far 80937Fh ;set NMI callback to RETF prevent FILE to be executed AGAIN)
  mov  x,[13B2h]       ;BIOS online flag (8bit)  ;\skip below if offline
  jz   @@skip                                    ;/
  push pb       ;\retadr for below               ;\
  push @@back-1 ;/                               ;
  push db       ;-incoming pushed DB for below   ;
  push 7E00h    ;\                               ; init apu effects/music
  pop  db       ; incoming current DB for below  ; (according to APU bits
  pop  db ;=7Eh ;/                               ; in town status packet)
  jmp  far 80EB99h ;--> injump to 105BC0h        ;
 @@back:                                         ;
  .assume p=20h  ;(above set's it so)            ;
  ;(if executed, ie. not when @@skip'ed)         ;/
 @@skip:
  clr  p,30h // .assume p=00h  ;below call 81C2B0h requires M=0, X=0
  mov  [0CDEh],0000h                            ;-mark fade-in/out non-busy
  mov  a,0099h     ;\                           ;\
  mov  [0BEh],a    ; 99D69A ;BIOS - enter town  ; create_interpreter_thread
  mov  a,0D69Ah    ;/                           ; (99D69Ah = enter town)
  call far 81C2B0h                              ;/
  set  p,20h // .assume p=20h
  mov  a,81h       ;\enable NMI and joypad (unstable: BIOS isn't yet mapped!)
  mov  [4200h],a   ;/caution: ensure that no NMI occurs in next few clk cycles
  mov  a,80h                                    ;\enable BIOS also in bank 0,
  mov  [075000h],a ;map BIOS to bank 00h-1Fh    ; and return to BIOS NMI handler
  jmp  far 80BC27h ;apply [0E5000h]=a, and retf ;/
```

<a id="snescartsatellaviewinterpretertokensummary"></a>

## SNES Cart Satellaview Interpreter Token Summary

**Interpreter Tokens**

```text
  00h  ControlSubThread(pEntrypoint)  ;special actions upon xx0000h..xx0005h
  01h  SetXYsignViewDirectionToSignsOfIncomingValues(vX,vY) ;not if both zero
  02h  SleepWithFixedObjShape(wSleep,pObjShape)
  03h  SleepWithXYstepAs9wayObjShape(wSleep,pObjShape1,..,pObjShape9)
  04h  SleepWithXYsignAs9wayObjShape(wSleep,pObjShape1,..,pObjShape9)
  05h  ClearForcedBlankAndFadeIn(wSleep,wSpeedRange?)
  06h  MasterBrightnessFadeOut(wSleep,wSpeedRange?) ;OptionalForcedBlank?
  07h  SetMosaicAndSleep(wSleep,wBgFlags,wMosaicSize)
  08h  N/A (hangs)
  09h  SleepAndBlendFromCurrentToNewPalette(wSleep,vPalIndex,pNewPalette)
  0Ah  HdmaEffectsOnBg3(wSleep,wEffectType,vScrollOffset,vExtraOffset)
  0Bh  SleepWithAngleAs9wayObjShape(wSleep,pObjShape1,..,pObjShape9) ;[18A8+X]
  0Ch  DisableObjsOfAllThreads()
  0Dh  ReEnableObjsOfAllThreads()
  0Eh  SleepWithXYsignAs9wayPlayerGenderObjShape(wSleep,pObjShape1,..,Shape9)
  0Fh  N/A (hangs)
  10h  SleepAndSetXYpos(wSleep,vX,vY)
  11h  SleepAndMoveTowardsTargetXYpos(wSleep,vX,vY)
  12h  SleepAndMoveByIncomingXYstep(wSleep,vX,vY)
  13h  SleepAndMoveAndAdjustXYstep(wSleep,vRotationAngleToOldXYstepOrSo?)
  14h  SleepAndMoveWithinBoundary(wSleep,vX1,vX2,vY1,vY2,wFactor?)
  15h  SleepAndMoveChangeBothXYstepsIfCollideOtherThread(wSleep,wBounceSpeed?)
  16h  SleepAndMoveAndIncrementXYstep(wSleep,vXincr,vYincr,qXlimit,qYlimit)
  17h  SleepAndMoveByIncomingYstepAndWavingXstep(wSleep,wY)
  18h  SleepAndMoveAndAccelerateTowardsTarget(wSleep,vX,vY,vSpeed)
  19h  SleepAndMoveAndSomethingComplicated?(wSleep,vX,vY)  ;out: X,Y=modified
  1Ah  AdjustXYstep(wNewSpeedOrSo?) ;in: [18A8+X]=angle
  1Bh  MoveByOldXYstepWithoutSleep()
  1Ch  SleepAndMoveChangeXYstepIfCollideOtherThread(wSleep,vMask,vX?,vY?)
  1Dh  N/A (hangs)
  1Dh  N/A (hangs)
  1Fh  N/A (hangs)
  20h  Goto(pTarget)
  21h  Gosub(pTarget)   ;max nesting=8 (or less when also using Loops)
  22h  Return()         ;return from Gosub
  23h  QuitThread()     ;terminate thread completely
  24h  LoopStart(wRepeatCount)  ;see token 62h (LoopNext)
  25h  Sleep(wSleep)
  26h  MathsLet(vA,vB)       ;A=B
  27h  MathsAdd(vA,vB)       ;A=A+B      ;1998 if unsigned carry
  28h  MathsSub(vA,vB)       ;A=A-B      ;1998 if signed overflow
  29h  MathsAnd(vA,vB)       ;A=A AND B  ;1998 if nonzero
  2Ah  MathsOr(vA,vB)        ;A=A OR B   ;1998 if nonzero
  2Bh  MathsXor(vA,vB)       ;A=A XOR B  ;1998 if nonzero
  2Ch  MathsNot(vA)          ;A=NOT A    ;1998 if nonzero
  2Dh  MathsMulSigned(vA,vB) ;A=A*B/100h ;1998 never (tries to be overflow)
  2Eh  MathsDivSigned(vA,vB) ;A=A/B*100h ;1998 if division by 0
  2Fh  SignedCompareWithConditionalGoto(vA,wOperator,vB,pTarget)
  30h  GotoIf_1998_IsNonzero(pTarget)
  31h  GotoIf_1998_IsZero(pTarget)
  32h  GotoArray(vArrayIndex,pPointerToArrayWithTargets)
  33h  ReadJoypad(bJoypadNumber,wX,wY)
  34h  CreateAnotherInterpreterThreadWithLimit(vThreadCount,bLimit,pEntry)
  35h  CheckIfXYposCollidesWithFlaggedThreads(vFlagMask) ;out: 1998=ID
  36h  GetUnsignedRandomValue(vA,wB) ;A=Random MOD B, special on B>7FFFh
  37h  SetObjWidthDepthFlagmask(vWidth,vDepth,vMask) ;for collide checks
  38h  CreateAnotherInterpreterThreadWithIncomingXYpos(vX,vY,pEntrypoint)
  39h  N/A (hangs)
  3Ah  SoundApuMessage00h_nnh(vParameter8bit)
  3Bh  SoundApuMessage01h_nnnh(vLower6bit,bMiddle2bit,bUpper2bit)
  3Ch  SoundApuMessage02h_nnnnh(vLower6bit,bMiddle2bit,bUpper2bit)
  3Dh  SoundApuUpload(bMode,pPtrToPtrToData)
  3Eh  SetPpuBgModeKillAllOtherThreadsAndResetVariousStuff(bBgMode)
  3Fh  SetTemporaryTableForBanksF1hAndUp(vTableNumber,pTableBase)
  40h  KillAllFlaggedThreads(vMask)  ;ignores flags, and kills ALL when Mask=0
  41h  SetBUGGEDTimerHotspot(wHotspot) ;BUG: accidently ORed with AE09h
  42h  Ppu_Bg1_Bg2_SetScrollPosition(vX,vY)
  43h  Ppu_Bg1_Bg2_ApplyScrollOffsetAndSleep(wSleep,vX,vY)
  44h  NopWithDummyParameters(wUnused,wUnused)
  45h  NopWithoutParameters()
  46h  AllocateAndInitObjTilesOrUseExistingTiles(wLen,pSrc)
  47h  AllocateAndInitObjPaletteOrUseExistingPalette(pSrc)
  48h  DmaObjTilesToVram(wObjVramAddr,wOBjVramEnd,pSrc)
  49h  SetObjPalette(wObjPalIndex,wObjPalEnd,pSrc)
  4Ah  SramAddSubOrSetMoney(bAction,vLower16bit,vMiddle16bit,vUpper16bit)
  4Bh  SramUpdateChksumAndBackupCopy()
  4Ch  N/A (hangs)
  4Dh  N/A (hangs)
  4Eh  N/A (hangs)
  4Fh  N/A (hangs)
  50h  TestAndGotoIfNonzero(vA,vB,pTarget)  ;Goto if (A AND B)<>0
  51h  TestAndGotoIfZero(vA,vB,pTarget)     ;Goto if (A AND B)==0
  52h  InitNineGeneralPurposePrivateVariables(wA,wB,wC,wD,wE,wF,wG,wH,wI)
  53h  MultipleCreateThreadBySelectedTableEntries(vFlags,vLimit,pPtrToTable)
  54h  PrepareMultipleGosub()  ;required prior to token 6Ah
  55h  StrangeXYposMultiplyThenDivide(wA,wB) ;Pos=Pos*((B-A)/2)/((B-A)/2)
  56h  BuggedForceXYposIntoScreenArea() ;messes up xpos and/or hangs endless
  57h  Maths32bitAdd16bitMul100h(vA(Msw),vB) ;A(Msw:Lsw)=A(Msw:Lsw)+B*100h
  58h  Maths32bitSub16bitMul100h(vA(Msw),vB) ;A(Msw:Lsw)=A(Msw:Lsw)-B*100h
  59h  SoundApuUploadWithTimeout(wTimeout,pPtrToPtrToData)
  5Ah  N/A (hangs)
  5Bh  N/A (hangs)
  5Ch  N/A (hangs)
  5Dh  N/A (hangs)
  5Eh  N/A (hangs)
  5Fh  N/A (hangs)
  60h  CallMachineCodeFunction(pTarget)
  61h  SetTemporaryOffsetFor0AxxxxhVariables(vOffset)
  62h  LoopNext()  ;see token 24h (LoopStart)
  63h  SetForcedBlankAndSleepOnce()
  64h  ClearForcedBlankAndSleepOnce()
  65h  AllocateAndInitObjPaletteAndObjTilesOrUseExistingOnes(pSrc) ;fragile
  66h  WriteBgTiles(wBgNumber,pPtrTo16bitLenAnd24bitSrcPtr)
  67h  WritePalette(pPtrTo16bitLenAnd24bitSrcPtr)  ;to backdrop/color0 and up
  68h  WriteBgMap(wBgNumber,pPtrTo16bitLenAnd24bitSrcPtr)
  69h  KillAllOtherThreads()
  6Ah  MultipleGosubToSelectedTableEntries(vFlags,pPtrToTable) ;see token 54h
  6Bh  AllocateAndInitBgPaletteTilesAndMap2(vX1,vY1,pPtrToThreePtrs,vBgMapSize)
  6Ch  DeallocateAllObjTilesAndObjPalettes()
  6Dh  BuggedSetBgParameters(bBgNumber,pPtr,wXsiz,wYsiz,wUnused,wUnused)
  6Eh  BuggedSetUnusedParameters(bSomeNumber,pPtr,wX,wY)
  6Fh  BuggedChangeBgScrolling(wX,wY)
  70h  PauseAllOtherThreads()
  71h  UnPauseAllOtherThreads()
  72h  GosubIfAccessedByPlayer(pGosubTargetOrPeopleFolderID)
  73h  Dma16kbyteObjTilesToTempBufferAt7F4000h()   ;Backup OBJ Tiles
  74h  Dma16kbyteObjTilesFromTempBufferAt7F4000h() ;Restore OBJ Tiles
  75h  SetFixedPlayerGenderObjShape(pSrc,wLen1,wLen2)
  76h  InstallPeopleIfSatelliteIsOnline() ;create all people-threads
  77h  KillAllOtherThreadsAndGotoCrash()  ;Goto to FFh-filled ROM at 829B5Eh
  78h  ZerofillBgBufferInWram(vBgNumber)
  79h  ChangePtrToObjPriority(vVariableToBePointedTo)  ;default is <Ypos>
  7Ah  ChangeObjVsBgPriority(vPriorityBits)     ;should be (0..3 * 1000h)
  7Bh  SetXYposRelativeToParentThread(vX,vY)
  7Ch  TransferObjTilesAndObjPaletteToVram(pPtrToPtrsToPaletteAndTileInfo)
  7Dh  AllocateAndInitBgPaletteTilesAndMap1(vX1,vY1,pPtrToThreePtrs,vBgMapSize)
  7Eh  DrawMessageBoxAllAtOnce(vWindowNumber,vDelay,vX,vY,pPtrToString)
  7Fh  DrawMessageBoxCharByCharBUGGED(..)  ;works only via CALL, not token 7Fh
  80h..FFh  Reserved/Crashes (jumps to garbage function addresses)
```

**Legend for Token Parameters**

```text
  v   16bit Global or Private Variable or Immediate (encoded as 3 token bytes)
  p   24bit Pointer (3 token bytes) (banks F0h..FFh translated, in most cases)
  b   8bit  Immediate (encoded directly as 1 token byte)
  w   16bit Immediate (encoded directly as 2 token bytes)
  q   16bit Immediate (accidently encoded as 3 token bytes, last byte unused)
```

**3-byte Variable Encoding (v)**

```text
  +/-00nnnnh  -->  +/-nnnnh          R     ;immediate
  +/-01nnnnh  -->  +/-[nnnnh+X]      R/W   ;private variable (X=thread_id*2)
  +/-02nnnnh  -->  +/-[nnnnh]        R/W   ;global variable
  +  03nnnnh  -->  +[nnnnh+[19A4h]]  W     ;special (write-only permission)
  +  09nnnnh  -->  +[nnnnh+[19A4h]]  R/W   ;special (read/write permission)
  +  0Annnnh  -->  +[nnnnh+[19A4h]]  R     ;special (read-only permission)
  Examples: 000001h or FF0001h (aka -00FFFFh) are both meaning "+0001h".
  021111h means "+[1111h]", FDEEEF (aka -021111h) means "-[1111h]".
```

**3-byte Pointer Encoding (p)**

```text
  00nnnnh..EFnnnnh     --> 00nnnnh..EFnnnnh      (unchanged)
  F0nnnnh              --> TokenProgramPtr+nnnn  (relative)
  F1nnnnh (or F2nnnnh) --> [[AFh+0]+nnnn*3]      (indexed by immediate)
  F3nnnnh              --> [[AFh+0]+[nnnn+X]*3]  (indexed by thread-variable)
  F4nnnnh              --> [[AFh+0]+[nnnn]*3]    (indexed by global-variable)
  F5nnnnh (or F6nnnnh) --> [[AFh+3]+nnnn*3]      (indexed by immediate)
  F7nnnnh              --> [[AFh+3]+[nnnn+X]*3]  (indexed by thread-variable)
  F8nnnnh              --> [[AFh+3]+[nnnn]*3]    (indexed by global-variable)
  F9nnnnh (or FAnnnnh) --> [[AFh+6]+nnnn*3]      (indexed by immediate)
  FBnnnnh              --> [[AFh+6]+[nnnn+X]*3]  (indexed by thread-variable)
  FCnnnnh              --> [[AFh+6]+[nnnn]*3]    (indexed by global-variable)
  FDnnnnh..FFnnnnh     --> crashes               (undefined/reserved)
```

**2-byte Operators for Signed Compare (Token 2Fh)**

```text
  0000h Goto_if_less              ;A<B
  0001h Goto_if_less_or_equal     ;A<=B
  0002h Goto_if_equal             ;A=B
  0003h Goto_if_not_equal         ;A<>B
  0004h Goto_if_greater           ;A>B
  0005h Goto_if_greater_or_equal  ;A>=B
```

**ControlSubThread(pEntrypoint) values**

```text
  xx0000h Pause
  xx0001h UnpauseSubThreadAndReenableObj
  xx0002h PauseAfterNextFrame
  xx0003h PauseAndDisableObj
  xx0004h ResetAndRestartSubThread
  xx0005h KillSubThread
  NNNNNNh Entrypoint (with automatic reset; only if other than old entrypoint)
```

Maximum Stack Nesting is 4 Levels (Stack is used by Gosub and Loop tokens).

**Token Extensions (some predefined functions with Token-style parameters)**

These are invoked via CALL Token (60h), call address, followed by params.

```text
  809225h CallKillAllMachineCodeThreads()
  80B47Dh CallGetTextLayerVramBase()
  80B91Eh CallClearBg3TextLayer()
  818EF9h CallSetApuRelatedPtr()
  818F06h CallDrawMessageBoxCharByChar(vWindowNumber,vDelay,vX,vY,pPtrToString)
  818FF0h CallDrawBlackCircleInLowerRightOfWindow()
  81903Dh CallDisplayButton_A_ObjInLowerRightOfWindow()
  81A508h CallSetGuiBorderScheme(pAddr1,pAddr2)
  81A551h CallSetTextWindowBoundaries(wWindowNumber,bXpos,bYpos,bXsiz,bYsiz)
  81A56Eh CallHideTextWindow(wWindowNumber)
  81A57Bh CallSelectWindowBorder(wWindowNumber,wBorder) ;0..3, or FFh=NoBorder
  81A59Ah CallSelectTextColor(wWindowNumber,bColor,bTileBank,bPalette)
  81A5C3h CallClearTextWindowDrawBorder(wWindowNumber)
  81A5D2h CallZoomInTextWindow(wWindowNumber,wZoomType)  ;\1,2,3=Zoom HV,V,H
  81A603h CallZoomOutTextWindow(wWindowNumber,wZoomType) ;/0=None/BuggyWinDiv2
  81A634h CallSetGuiColorScheme(pAddr)
  81A65Dh CallChangePaletteOfTextRow(vX,vY,vWidth,vPalette)
  81A693h CallPeekMemory16bit(vDest,pSource)
  81A6B4h CallPokeMemory16bit(vSource,pDest)
  81C7D0h CallInitializeAndDeallocateAllObjTilesAndObjPalettes()
  81C871h CallDeallocateAllObjs()
  81CDF9h CallBackupObjPalette()
  81CE09h CallRestoreObjPalette()
  829699h CallUploadPaletteVram(pSource,wVramAddr,bPaletteIndex)
  88932Fh CallTestIfFolderExists()  ;in: 0780, out: 1998,077C,077E
  88D076h CallTestIfDoor()
  99D9A4h CallSelectPlayerAsSecondaryThread  ;[19A4]=PlayerThreadId*2
```

Note: Some of these Call addresses are also listed in a 24bit-pointer table at
address 808000h (though the (BIOS-)code uses direct 8xxxxxh values instead of
indirect [808xxxh] values).

**Token Functions (some predefined token-functions)**

These can be invoked with GOSUB token (or GOTO or used as thread entrypoint):

```text
  99D69A EnterTown (use via goto, or use as entrypoint)
  828230 DeallocMostBgPalettesAndBgTiles ;except tile 000h and color 00h-1Fh
  88C1C6 SetCursorShape0
  88C1D0 SetCursorShape1
  88C1E0 SetCursorShape2
  88C1EA SetCursorShape3
  88C1F4 SetCursorShape4
  88C1FE SetCursorShape5
  99D8AB PauseSubThreadIfXYstepIsZero
  99D8CD MoveWithinX1andX2boundaries
  99D903 MoveWithinY1andY2boundaries
```

Note: Some of the above functions are also listed in the table at 808000h.

**Compressed Data**

Some of the Functions can (optionally) use compressed Tile/Map/Palette data.

[SNES Decompression Formats](70-hotel-arcade-nss-sfcbox.md#snes-decompression-formats)

Note: The actual compressed data is usually preceeded-by or bundled-with
compression flags, length entry, and/or (in-)direct src/dest pointers (that
"header" varies from function to function).

<a id="snescartsatellaviewchipsets"></a>

## SNES Cart Satellaview Chipsets

**BSC-1A5B9P-01 (1995) (BIOS cartridge PCB)**

```text
  U1  44pin  MCC-BSC LR39197 Nintendo
  U2  36pin  ROM (36pin/40pin possible)
  U3  32pin  658512LFP-85 (4Mbit PSRAM)
  U4  28pin  LH52B256NB-10PLL (256Kbit SRAM)
  U5  8pin   MM1134 (battery controller for SRAM)
  BT1 2pin   Battery
  CN1 62pin  SNES Cartridge Edge (pin 2,33 used)
  CN2 62pin  Flash Cartridge Connector (male?)
```

There's no CIC chip (either it's contained in the MCC-chip... or in the flash
card, but in that case the thing won't work without flash card?)

**MAIN-BSA-01 (1995) (receiver unit/expansion port PCB)**

```text
  U1 20pin  74LS541 8-bit 3-state buffer/line driver
  U2 20pin  74LS541 8-bit 3-state buffer/line driver
  U3 20pin  74LS245 8-bit 3-state bus transceiver
  U4 8pin   SPR-BSA (unknown, might be controlled via port 2198h or 2199h?)
  U5 100pin DCD-BSA (custom Nintendo chip)
  U6 64pin  MN88821 (maybe a MN88831 variant: Satellite Audio Decoder)
  U7 18pin  AN3915S Clock Regenerator (for amplifying/stabilizing Y1 crystal)
  U8 4pin   PQ05RH1L (5V regulator with ON/OFF control)
  U9 14pin  LM324 Quad Amplifier
  Y1 2pin   18.432MHz crystal
  T1 4pin   ZJYS5102-2PT Transformator
  T2 4pin   ZJYS5102-2PT Transformator
  CN1 28pin SNES Expansion Port
  CN2 38pin Expansion Port (EXT) (believed to be for modem)
  CN3 3pin  To POWER and ACCESS LEDs on Front Panel
  CN4 7pin  Rear connector (satellite and power supply?)
```

**BSMC-AF-01 (Memory Card PCB) (to be plugged into BIOS cartridge)**

```text
  U1  56pin Sharp LH28F800SUT-ZI (or -Z1?) (1Mbyte FLASH)
  CN1 62pin Flash Cartridge Connector (female?)
```

There are no other chips on this PCB (only capacitors and resistors).

**BSMC-CR-01 (Memory Card PCB) (to be plugged into GAME cartridges)**

```text
  U1  ?pin  unknown (reportedly read-only... mask ROM?)
  CN1 62pin Flash Cartridge Connector (female?)
```

**BSC-1A5M-01 (1995) (GAME cartridge with onboard FLASH cartridge slot)**

```text
  U1  36pin  ROM
  U2  28pin  SRAM (32Kbytes)
  U3  16pin  MAD-1A
  U4  16pin  CIC D411B
  BT1 2pin   Battery CR2032
  CN1 62pin  SNES Cartridge Edge (pin 2,33 used)
  CN2 62pin  Flash Cartridge Connector (male 2x31 pins)
```

Used by "Derby Stallion 96" (and maybe other games, too).

**BSC-1L3B-01 (1996) (GAME cartridge with SA1 and onboard FLASH cartridge slot)**

```text
  U1  44pin  ROM
  U2  28pin  SRAM (8Kbytes)
  U3  128pin SA1
  U4  8pin   MM1026AF (battery controller for SRAM)
  BT1 2pin   Battery
  CN1 62pin  SNES Cartridge Edge (pin 2,33 used)
  CN2 62pin  Flash Cartridge Connector (male?)
```

Used by "Itoi Shigesato no Bass Tsuri No. 1" (and maybe other games, too).

**Nintendo Power flashcarts**

Theoretically, Nintendo Power flashcarts are also compatible with the BSX
expansion hardware (in terms of connecting EXPAND to SYSCK via 100 ohms),
unknown if any Nintendo Power titles did actually use that feature.

<a id="snescartdatapackslotssatellaviewlikeminicartridgeslot"></a>

## SNES Cart Data Pack Slots (satellaview-like mini-cartridge slot)

**Data Packs**

Data Packs are Satellaview 8M Memory Packs which have data meant to be used as
expansion for a Data Pack-compatible game. Data Pack-compatible game cartridges
have a resemblence to the BS-X Cartridge itself.

**Usage**

For most of these games, Data was distributed via St.GIGA's Satellaview
services. Same Game and SD Gundam G-Next had some Data Packs sold as retail in
stores. RPG Tsukuru 2, Sound Novel Tsukuru and Ongaku Tsukuru Kanaderu could
save user-created data to 8M Memory Packs.

**Cartridges with Data Pack Slot**

```text
  Derby Stallion 96                  (SpecialLoROM, 3MB ROM, 32K RAM)
  Itoi Shigesato no Bass Tsuri No. 1 (SA-1, map-able 4MB ROM, 8K RAM)
  Joushou Mahjong Tenpai             (HiROM, 1MB)
  Ongaku Tukool/Tsukuru Kanaderu     (HiROM, 1MB)
  RPG Tukool/Tsukuru 2               (LoROM, 2MB)
  Same Game Tsume Game               (HiROM, 1MB)
  Satellaview BS-X BIOS              (MCC, 1MB ROM) (FLASH at C00000h)
  SD Gundam G-NEXT                   (SA-1, map-able 1.5MB ROM, 32K RAM)
  Sound Novel Tukool/Tsukuru         (SpecialLoROM, 3MB ROM, 64K RAM)
```

Aside from the BS-X BIOS, two of the above games are also accessing BS-X
hardware via I/O Ports 2188h/2194h/etc (Derby Stallion 96, Itoi Shigesato no
Bass Tsuri No. 1). For doing that, the cartridges do probably require the
EXPAND pin to be wired via 100 ohm to SYSCK.

**Cartridge Header of cartridges with Data Pack Slot**

The presence of Data Pack Slots is indicated by a "Z" as first letter of Game
Code:

```text
  [FFB2h]="Z"   ;first letter of game code
  [FFB5h]<>20h  ;game code must be 4-letters (not space padded 2-letters)
  [FFDAh]=33h   ;game code must exist (ie. extended header must be present)
```

**Data Pack Mapping**

```text
  MCC (BSX-BIOS)        FLASH at C00000h (continous) (mappable via MCC chip)
  SA-1                  FLASH at <unknown address> (probably mappable via SA1)
  HiROM                 FLASH at E00000h (probably continous)
  LoROM/SpecialLoROM    FLASH at C00000h (looks like 32K chunks)
```

**LoROM/SpecialLoROM Mapping Notes**

The FLASH memory seems to be divided into 32K chunks (mirrored to Cn0000h and
Cn8000h) (of which, Derby Stallion 96 uses Cn8000h, RPG Tukool uses Cn0000h,
and Ongaku Tukool uses both Cn0000h and Cn8000h).

The two 3MB SpecialLoROM games also have the ROM mapped in an unconventional
fashion:

```text
  1st 1MB of ROM mapped to banks 00-1F
  2nd 1MB of ROM mapped to banks 20-3F and A0-BF
  3rd 1MB of ROM mapped to banks 80-9F
  1MB of Data Pack FLASH mapped to banks C0-DF
  32K..64K SRAM mapped to banks 70-71
```

Despite of memory-mirroring of 2nd MB, the checksum-mirroring goes on 3rd MB?

Note: Above mapping differs from "normal" 3MB LoROM games like Wizardry 6
(which have 3rd 1MB in banks 40h-5Fh).

<a id="snescartnintendopowerflashcard"></a>

## SNES Cart Nintendo Power (flashcard)

Nintendo Power cartridges are official FLASH cartridges from Nintendo (released
only in Japan). Unlike the older Satellaview FLASH cartridges, they do connect
directly to the SNES cartridge slot. The capacity is 4MByte FLASH and 32KByte
battery-backed SRAM.

**FLASH (512Kbyte blocks)**

The FLASH is divided into eight 512Kbyte blocks. The first block does usually
contain a Game Selection Menu, the other blocks can contain up to seven
512KByte games, or other combinations like one 3MByte game and one 512KByte
game. Alternately, the cartridge can contain a single 4MByte game (in that
case, without the Menu).

**SRAM (2Kbyte blocks) (battery-backed)**

The SRAM is divided into sixteen 2Kbyte blocks for storing game positions.
Games can use one or more (or all) of these blocks (the menu doesn't use any of
that memory).

[SNES Cart Nintendo Power - New Stuff](#snes-cart-nintendo-power-new-stuff)

[SNES Cart Nintendo Power - I/O Ports](#snes-cart-nintendo-power-io-ports)

[SNES Cart Nintendo Power - FLASH Commands](#snes-cart-nintendo-power-flash-commands)

[SNES Cart Nintendo Power - Directory](#snes-cart-nintendo-power-directory)

[SNES Pinouts Nintendo Power Flashcarts](80-timings-unpredictable-pinouts.md#snes-pinouts-nintendo-power-flashcarts)

**Nintendo Power Games**

Games have been available at kiosks with FLASH Programming Stations. There are
around 150 Nintendo Power games: around 21 games exclusively released only for
Nintendo Power users, and around 130 games which have been previously released
as normal ROM cartridges.

**Nintendo Power PCB "SHVC-MMS-X1" or "SHVC-MMS-02" (1997) Chipset (SNES)**

```text
  U1  18pin CIC       ("F411B Nintendo")
  U2 100pin MX15001   ("Mega Chips MX15001TFC")
  U3  44pin 16M FLASH ("MX 29F1601MC-11C3") (2Mbyte FLASH, plus hidden sector)
  U4  44pin 16M FLASH ("MX 29F1601MC-11C3") (2Mbyte FLASH, plus hidden sector)
  U5  44pin 16M FLASH (N/A, not installed)
  U6  28pin SRAM      ("SEC KM62256CLG-7L") (32Kbyte SRAM)
  U7   8pin MM1134    ("M 707 134B") (battery controller)
  BAT1 2pin Battery   ("Panasonic CR2032 +3V")
```

**Nintendo Power PCB "DMG-A20-01" (199x) Chipset (Gameboy version)**

```text
  U1  80pin G-MMC1    ("MegaChips MX15002UCA"
  U2  40pin 8M FLASH  ("MX29F008ATC-14") (plus hidden sector)
  U3  32pin 1M SRAM   ("UT621024SC-70LL")
  X1   3pin N/A       (oscillator? not installed)
  BAT1 2pin Battery   ("Panasonic CR2025")
```

**Nintendo Power Menu SNES Cartridge Header**

```text
  Gamecode:        "MENU" (this somewhat indicates the "MX15001" chip)
  ROM Size:        512K (the menu size, not including the other FLASH blocks)
  SRAM Size:       0K (though there is 32Kbyte SRAM for use by the games)
  Battery Present: Yes
  Checksum:        Across 512Kbyte menu, with Directory assumed to be
                   FFh-filled (except for the "MULTICASSETTE 32" part)
```

The PCB doesn't contain a ROM (the Menu is stored in FLASH, too).

**Nintendo Power Menu Content**

```text
  ROM Offset  SNES Address Size   Content
  000000h     808000h      4xxxh  Menu Code (around 16K, depending on version)
  004xxxh     80xxxxh      3xxxh  Unused (FFh-filled)
  007FB0h     80FFB0h      50h    Cartridge Header
  008000h     818000h      40000h Unused (FFh-filled)
  048000h     898000h      372Bh  Something (APU code/data or so)
  04B72Bh     8xxxxxh      47D5h  Unused (FFh-filled)
  050000h     8A8000h      8665h  Something (VRAM data or so)
  058665h     8Bxxxxh      798Bh  Unused (FFh-filled)
  060000h     8C8000h      10000h Directory (File 0..7) (2000h bytes/entry)
  070000h     8E8000h      10000h Unused (FFh-filled)
```

**Note**

Nintendo has used the name "Nintendo Power" for various different things:

```text
  Super Famicom Flashcards (in Japan)
  Gameboy Color Flashcards (in Japan)
  Super Famicom Magazine (online via Satellaview BS-X) (in Japan)
  Official SNES Magazine (printout) (in USA)
```

<a id="snescartnintendopowernewstuff"></a>

## SNES Cart Nintendo Power - New Stuff

**Operation during /RESET=LOW**

```text
  [0AAAAh]=AAh, [05554h]=55h, [0AAAAh]=F0h   ;FLASH read/reset command
  [00000h]=38h, [00000h]=D0h, [00000h]=71h   ;FLASH request chip info part 1
  dummy=[00004h]                             ;Read Ready-status (bit7=1=ready)
  [00000h]=72h, [00000h]=75h                 ;FLASH request chip info part 2
  Port[2404h..2407h]=[0FF00h+(n*8)+0,2,4,6]  ;Read mapping info for File(n)
  [0AAAAh]=AAh, [05554h]=55h, [0AAAAh]=F0h   ;FLASH read/reset command
```

**Detailed**

```text
  [00000h]=38h   ;copy hidden sector to page buffer?
  [00000h]=D0h   ;  ...confirm above
  [00000h]=71h   ;read extended status register
  dummy=[00004h] ;  ...do read it (bit7=1=ready)
  [00000h]=72h   ;swap page buffer (map above buffer to cpu side?)
  [00000h]=75h   ;read page buffer to cpu
  xx=[0FFxxh]    ;  ...do read it
```

other interesting commands:

```text
  [00000h]=74h   ;write page buffer single byte from cpu
  [0xxxxh]=xx    ;  ...do write it
```

or sequential:

```text
  [00000h]=E0h   ;sequential load from cpu to page buffer
  [00000h]=num.L ;  ...byte count lsb (minus 1 ?) (0=one byte, what=100h bytes)
  [00000h]=num.H ;  ...byte cound msb (zero)
  [?]=data       ;  ...data?
  [00000h]=0Ch   ;forward page buffer to flash
  [00000h]=num.L ;  ...byte count lsb (minus 1 ?) (0=one byte, what=100h bytes)
  [addr]=num.H   ;  ...byte cound msb (zero)
  [...]?         ;  ...do something to wait until ready
```

**Hidden Mapping Info Example (chip 1 at C0FFxxh, chip 2 at E0FFxxh)**

```text
  C0FF00      03 11 AA 74 AA 97 00 12  ;Menu              (512K Lorom, no SRAM)
  C0FF08      00 08 29 15 4A 12 10 01  ;Super Mario World (512K Lorom, 2K SRAM)
  C0FF10      0B FF AA FF AA FF 21 FF  ;Doraemon 4        (1.5M Lorom, no SRAM)
  C0FF18      49 FF 61 FF A5 FF 51 FF  ;Dragon Slayer II  (1.5M Hirom, 8K SRAM)
  C0FF20..FF  FF-filled (byte at C0FF7Fh is 00h in some carts)  ;-unused
  E0FF00..8F  FF-filled (other values at E0FF8xh in some carts) ;\garbage, from
  E0FF90      FF FF 55 00 FF FF FF FF FF FF FF FF FF FF FF FF   ; chip-testing
  E0FFA0      FF FF FF FF FF FF 55 00 FF FF FF FF FF FF FF FF   ; or so
  E0FFB0      FF FF FF FF FF FF 55 00 FF FF FF FF FF FF FF FF   ;/
  E0FFC0..FF  FF-filled                                         ;-unused
```

There are always 8 bytes at odd addresses at C0FF01..0F, interleaved with the
mapping entries 0 and 1 (though no matter if the cart uses 1, 2, or 3 mapping
entries). The 'odd' bytes are some serial number, apart from the first two
bytes, it seems to be just a BCD date/time stamp, ie. formatted as
11-xx-YY-MM-DD-HH-MM-SS.

New findings are that the "xx" in the "11-xx-YY-MM-DD-HH-MM-SS" can be non-BCD
(spotted in the Super Puyo Puyo cart).

Some carts have extra 'garbage' at C0FF7F and E0FF80..BF.

**Nintendo Power Commands**

```text
  if [002400h]<>7Dh then skip unlocking   ;else locking would be re-enabled
  [002400h]=09h       ;\
  dummy=[002400h]     ;
  [002401h]=28h       ; wakeup sequence (needed before sending other commands,
  [002401h]=84h       ; and also enables reading from port 2400h..2407h)
  [002400h]=06h       ;
  [002400h]=39h       ;/
```

After wakeup, single-byte commands can be written to [002400h]:

```text
  [002400h]=00h   RESET and map GAME14 ? (issues /RESET pulse)
  [002400h]=01h    causes always 8x7D
  [002400h]=02h   Set STATUS.bit2=1 (/WP=HIGH, release Write protect)
  [002400h]=03h   Set STATUS.bit2=0 (/WP=LOW, force Write protect)
  [002400h]=04h   HIROM:ALL  (map whole FLASH in HiROM mode)
  [002400h]=05h   HIROM:MENU (map MENU in HiROM mode instead normal LoROM mode)
  [002400h]=06h    causes always 8x7D (aka, undoes toggle?)
  [002400h]=07h    causes always 8x7D
  [002400h]=08h    causes always 8x7D
  [002400h]=09h    no effect  ;\
  [002400h]=0ah    no effect  ;/
  [002400h]=0bh    causes always 8x7D
  [002400h]=0ch    causes always 8x7D
  [002400h]=0dh    causes always 8x7D
  [002400h]=0eh    causes always 8x7D
  [002400h]=0fh    causes always 8x7D
  [002400h]=10h    causes always 8x7D
  [002400h]=14h    causes always 8x7D
  [002400h]=20h    Set STATUS.bit3=0 (discovered by skaman) (default)
  [002400h]=21h    Set STATUS.bit3=1 (discovered by skaman) (disable ROM read?)
  [002400h]=24h    causes always 8x7D
  [002400h]=44h    no effect (once caused crash with green rectangle)
  [002400h]=80h..8Fh  ;-Issue /RESET to SNES and map GAME 0..15
  [002400h]=C5h    causes always 8x7D
  [002400h]=FFh    sometimes maps GAME14 or GAME15? (unreliable)
```

<a id="snescartnintendopowerioports"></a>

## SNES Cart Nintendo Power - I/O Ports

**Nintendo Power I/O Map**

```text
 Write registers:
  2400h        - Command
  2401h        - Extra parameter key (used only for wakeup command)
  2402h..2407h - Unknown/unused
 Read registers (before wakeup):
  2400h..2407h - Fixed 7Dh
 Read registers (after wakeup):
  2400h        - Fixed 2Ah
  2401h        - Status
  2402h..2403h - Fixed 2Ah
  2404h        - Mapping Info: ROM/RAM Size         ;\these four bytes are
  2405h..2406h - Mapping Info: SRAM Mapping related ; initialized from the
  2407h        - Mapping Info: ROM/RAM Base         ;/hidden flash sector
```

**Port 2401h = Status (R)**

```text
  0-1 zero
  2   release /WP state    (set by CMD_02h, cleared by CMD_03h)
  3   disable ROM reading? (set by CMD_21h, cleared by CMD_20h)
  4-7 Selected Slot (0=Menu/File0, 1..15=File1..15) (via CMD_8xh)
```

**Port 2404h = Size (R)**

```text
  0-1 SRAM Size (0=2K, 1=8K, 2=32K, 3=None) ;ie. 2K SHL (N*2)
  2-4 ROM Size (0=512K, 2=1.5M, 5=3M, 7=4M) ;ie. 512K*(N+1)
  5   Maybe ROM Size MSB for carts with three FLASH chips (set for HIROM:ALL)
  6-7 Mode (0=Lorom, 1=Hirom, 2=Forced HIROM:MENU, 3=Forced HIROM:ALL)
```

**Port 2407h = Base (R)**

```text
  0-3 SRAM Base in 2K units
  4-7 ROM Base in 512K units (bit7 set for HIROM:MENU on skaman's blank cart)
```

**Port 2405h,2406h = SRAM Mapping Related (R)**

The values for port 2405h/2406h are always one of these three sets, apparently
related to SRAM mapping:

```text
  29,4A for Lorom with SRAM
  61,A5 for Hirom with SRAM
  AA,AA for Lorom/Hirom without SRAM
  61,A5 (when forcing HIROM:ALL)
  D5,7F (when forcing HIROM:MENU)
  8A,8A (when forcing HIROM:MENU on skaman's blank cart)
```

Probably selecting which bank(s) SRAM is mapped/mirrored in the SNES memory
space.

**Nintendo Power I/O Ports**

The I/O ports at 002400h-002401h are used for mapping a selected game. Done as
follows:

```text
  mov  [002400h],09h
  cmp  [002400h],7Dh
  jne  $  ;lockup if invalid
  mov  [002401h],28h
  mov  [002401h],84h
  mov  [002400h],06h
  mov  [002400h],39h
  mov  [002400h],80h+(Directory[n*2000h+0] AND 0Fh)
  jmp  $  ;lockup (until reset applies)
```

After the last write, the MX15001 chip maps the desired file, and does then
inject a /RESET pulse to the SNES console, which resets the CPU, APU (both SPC
and DSP), WRAM (address register), and any Expansion Port hardware (like
Satellaview), or piggyback cartridges (like Xband modem). The two PPU chips and
the CIC chip aren't affected by the /RESET signal. The overall effect is that
it boots the selected file via its Reset vector at [FFFCh].

<a id="snescartnintendopowerflashcommands"></a>

## SNES Cart Nintendo Power - FLASH Commands

Before sending write/erase commands, one must initialize the MX15001 chip via
port 240xh (particulary: release the /WP pin), selecting the HIROM_ALL mapping
mode may be also recommended (for getting the whole 4Mbyte FLASH memory mapped
as continous memory block at address C00000h-FFFFFFh).

Observe that the cart contains two FLASH chips. In HIROM_ALL mode, one chip is
at C00000h-DFFFFFh, the other one at E00000h-FFFFFFh (ie. commands must be
either written to C0AAAAh/C05554h or E0AAAAh/E05554h, depending on which chip
is meant to be accessed; when programming large files that occupy both chips,
it would be fastest to program both chips simultaneously).

**FLASH Command Summary**

The FLASH chips are using more or less using standard FLASH commands, invoked
by writing to low-bytes at word-addresses 05555h and 02AAAh (aka writing bytes
to byte-addresses 0AAAAh and 05554h).

```text
  [0AAAAh]=AAh, [05554h]=55h, [0AAAAh]=F0h, data=[addr..] ;Read/Reset
  [0AAAAh]=AAh, [05554h]=55h, [0AAAAh]=90h, ID=[00000h]   ;Get Maker ID
  [0AAAAh]=AAh, [05554h]=55h, [0AAAAh]=90h, ID=[00002h]   ;Get Device ID
  [0AAAAh]=AAh, [05554h]=55h, [0AAAAh]=90h, WP=[x0004h]   ;Get Sector Protect
  [0AAAAh]=AAh, [05554h]=55h, [0AAAAh]=70h, SRD=[00000h]  ;Read Status Reg
  [0AAAAh]=AAh, [05554h]=55h, [0AAAAh]=50h                ;Clear Status Reg
  [0AAAAh]=AAh, [05554h]=55h, [0AAAAh]=A0h, [addr..]=data ;Page/Byte Program
  [0AAAAh]=AAh, [05554h]=55h, [0AAAAh]=80h                ;Prepare Erase...
  [0AAAAh]=AAh, [05554h]=55h, [0AAAAh]=10h                ;...do Chip Erase
  [0AAAAh]=AAh, [05554h]=55h, [x0000h]=30h                ;...do Sector Erase
  [0xxxxh]=B0h                                            ;...Erase suspend
  [0xxxxh]=D0h                                            ;...Erase resume
  [0AAAAh]=AAh, [05554h]=55h, [0AAAAh]=60h                ;Prepare Protect...
  [0AAAAh]=AAh, [05554h]=55h, [addr]=20h                  ;...do Sector Protect
  [0AAAAh]=AAh, [05554h]=55h, [addr]=40h                  ;...do Sector Unprot.
  [0AAAAh]=AAh, [05554h]=55h, [0AAAAh]=C0h                ;Sleep
  [0AAAAh]=AAh, [05554h]=55h, [0AAAAh]=E0h                ;Abort
```

Undocumented commands for hidden sector:

```text
  [0AAAAh]=AAh, [05554h]=55h, [0AAAAh]=77h                ;Prepare Hidden...
  [0AAAAh]=AAh, [05554h]=55h, [0AAAAh]=99h, [addr..]=data ;...do Hidden Write
  [0AAAAh]=AAh, [05554h]=55h, [0AAAAh]=E0h                ;...do Hidden Erase
  [00000h]=38h, [00000h]=D0h, [00000h]=71h, dummy=[00004] ;Prepare Hidden Rd...
  [00000h]=72h, [00000h]=75h, data=[addr...]              ;...do Hidden Read
```

**FLASH Read/Reset Command (F0h)**

Resets the chip to normal Read Data mode; this is required after most commands
(in order to resume normal operation; for leaving the Get Status, Get ID, or
Sleep states).

**FLASH Get Status (70h) and Clear Status (50h)**

Clear Status resets the error flags in bit4,5 (required because those bits
would otherwise stay set forever). Get Status switches to read-status mode
(this is usually not required because the erase/program/protect/sleep commands
are automatically entering read-status mode). The separate status bits are:

```text
  7   Write/Erase State     (0=Busy, 1=Ready)
  6   Erase Suspend         (0=Normal, 1=Sector Erase Suspended)
  5   Erase Failure         (0=Okay, 1=Fail in erase)
  4   Program Failure       (0=Okay, 1=Fail in program)
  3   Reserved (zero)                           (MX29F1610A/B)
  3   Sector-Protect Status (0=?, 1=?)          (MX29F1611 only)
  2   Sleep Mode            (0=Normal, 1=Sleep) (MX29F1611 only)
  1-0 Reserved (zero)
```

**FLASH Get Maker/Device ID and Sector Protect Bytes (90h)**

Allows to read Maker/Device ID and/or Sector Protect Byte(s) from following
address(es):

```text
 [00000h]=Manufacturer ID:
  C2h = Macronix
 [00002h]=Device ID:
  FAh = MX29F1610A  ;\with sector_protect, suspend/resume, without sleep/abort
  FBh = MX29F1610B  ;/
  F7h = MX29F1611   ;-with sector_protect, suspend/resume, sleep/abort
  6Bh = MX29F1615   ;-without sector_protect, suspend/resume, sleep/abort
  F3h = MX29F1601MC ;<-- undocumented, used in SNES nintendo power carts
 [x0004h]=Sector Protect State:
  00h = normal unprotected 128Kbyte sector (can occur on all sectors)
  C2h = write-protected 128Kbyte sector (can occur on first & last sector only)
```

**FLASH Erase: Prepare (80h), and Chip Erase (10h) or Sector Erase (30h)**

Allows to erase the whole 2Mbyte chip (ie. half of the Nintendo Power cart), or
a specific 128Kbyte sector.

Some MX29F16xx chips are also allowing to suspend (B0h) or resume (D0h) sector
erase (allowing to access other sectors during erase, if that should be
desired).

**FLASH Page/Byte Program (A0h)**

Allows to write one or more bytes (max 80h bytes) to a 128-byte page.

The Page/Byte Program command doesn't auto-erase the written page, so the
sector(s) should be manually erased prior to programming (otherwise the new
bytes will be ANDed with old data).

Caution: The chips in Nintendo Power carts require the LAST BYTE written TWICE
in order to start programming (unlike as in offical MX29F16xx specs, which
claim programmig to start automatically after not sending further bytes for
about 30..100us).

**FLASH Protect: Prepare (60h), and Protect (20h) or Unprotect (40h)**

Allows to write-protect or unprotect separate 128Kbyte sectors (this works only
for the first and last sector of each chip) (/WP=HIGH overrides the
protection).

**FLASH Sleep (C0h)**

Switches the chip to sleep state; can be resumed only via Read/Reset command
(F0h). Sleep mode is supported on MX29F1611 only.

**FLASH Abort (E0h)**

Aborts something. Supported on MX29F1611 only.

**Basic MX29F16xx specs**

JEDEC-standard EEPROM commands

Endurance: 100,000 cycles

Fast access time: 70/90/120ns

Sector erase architecture

- 16 equal sectors of 128k bytes each

- Sector erase time: 1.3s typical

Page program operation

- Internal address and data latches for 128 bytes/64 words per page

- Page programming time: 0.9ms typical

- Byte programming time: 7us in average

<a id="snescartnintendopowerdirectory"></a>

## SNES Cart Nintendo Power - Directory

**Directory Area**

```text
  ROM Offset  SNES Address Size   Content
  060000h     8C8000h      2000h  File 0 (Menu)
  062000h     8CA000h      2000h  File 1
  064000h     8CC000h      2000h  File 2
  066000h     8CE000h      2000h  File 3
  068000h     8D8000h      2000h  File 4
  06A000h     8DA000h      2000h  File 5
  06C000h     8DC000h      2000h  File 6
  06E000h     8DE000h      2000h  File 7
  070000h     8E8000h      10000h Unused (FFh-filled)
```

The last 64Kbyte are probably usable as further file entries in cartridges
bigger than 4Mbyte (the Menu software in the existing cartridges is hardcoded
to process only files 1..7) (whilst Port 2400h seems to accept 4bit file
numbers).

**Directory Entry Format**

```text
  0000h 1    Directory index (00h..07h for Entry 0..7) (or FFh=Unused Entry)
  0001h 1    First 512K-FLASH block (00h..07h for block 0..7)
  0002h 1    First 2K-SRAM block    (00h..0Fh for block 0..15)
  0003h 2    Number of 512K-FLASH blocks (mul 4) (=0004h..001Ch for 1..7 blks)
  0005h 2    Number of 2K-SRAM blocks (mul 16)   (=0000h..0100h for 0..16 blks)
  0007h 12   Gamecode (eg. "SHVC-MENU-  ", "SHVC-AGPJ-  ", or "SHVC-CS  -  ")
  0013h 44   Title in Shift-JIS format (padded with 00h's) (not used by Menu)
  003Fh 384  Title Bitmap (192x12 pixels, in 30h*8 bytes, ie. 180h bytes)
  01BFh 10   Date "MM/DD/YYYY" (or "YYYY/MM/DD" on "NINnnnnn" carts)
  01C9h 8    Time "HH:MM:SS"
  01D1h 8    Law  "LAWnnnnn" or "NINnnnnn" (eg. "LAW01712", or "NIN11001")
  01D9h 7703 Unused (1E17h bytes, FFh-filled)
  1FF0h 16   For File0: "MULTICASSETTE 32" / For Files 1-7: Unused (FFh-filled)
```

**Directory Index**

Directory Index indicates if the Entry is used (FFh=unused). If it is used,
then it must be equal to the current directory entry number (ie. a rather
redundant thing, where the index is indexing itself). The lower 4bit of the
index value is used for game selection via Port 2400h.

**First FLASH/SRAM Block**

The First FLASH block number is stored in lower some bits of [0001h].

The First SRAM block number is stored in lower some bits of [0002h].

The directory doesn't contain any flag that indicates HiROM or LoROM mapping.

There is no support for fragmented FLASH/SRAM files (ie. the programming
station must erase &amp; rewrite the entire cartridge, with the old used/unused
blocks re-ordered so that they do form continous memory blocks).

**Number of FLASH/SRAM Blocks (displayed in Menu)**

These entries are used to display the amount of used/unused blocks. Free FLASH
blocks are shown as blue "F" symbols, free SRAM blocks as red "B" symbols, used
blocks as gray "F" and "B" symbols. Pressing the X-button in the menu indicates
which blocks are being used by the currently selected game.

**Title Bitmap (displayed in Menu)**

The 192x12 pixel title bitmap is divided into eight 24x12 pixel sections, using
a most bizarre encoding: Each section is 30h bytes in size (enough for 32 pixel
width, but of these 32 pixels, the "middle" 4 pixels are overlapping each
other, and the "right-most" 4 pixels are unused. The byte/pixel order for 12
rows (y=0..11) is:

```text
  Left 8 pixels   = (Byte[00h+y*2])
  Middle 8 pixels = (Byte[01h+y*2]) OR (Byte[18h+y*2] SHR 4)
  Right 8 pixels  = (Byte[18h+y*2] SHL 4) OR (Byte[19h+y*2] SHR 4)
```

The result is displayed as a normal 192-pixel bitmap (without any spacing
between the 24-pixel sections). The bits in the separate bytes are bit7=left,
bit0=right. Color depth is 1bit (0=dark/background, 1=bright/text). The bitmap
does usually contain japanese text without "special" features, though it could
also be used for small icons, symbols, bold text, greek text, etc.

**Text Fields (not used by Menu)**

The Shift-JIS Title, and ASCII Game Code, Date, Time, Law &amp; Multicassette
strings aren't used by the Menu. The 5-digit Law number is usually (but not
always) same for all files on the cartridge, supposedly indicating the station
that has programmed the file.

```text
  LAW = games installed on kiosks in Lawson Convenience Store chain
  NIN = games pre-installed by nintendo (eg. derby 98)
```

The Multicassette number does probably indicate the FLASH size in MBits (it's
always 32 for the existing 32Mbit/4MByte cartridges).

<a id="snescartsufamiturbominicartridgeadaptor"></a>

## SNES Cart Sufami Turbo (Mini Cartridge Adaptor)

The Sufami Turbo from Bandai is an adaptor for low-cost mini-cartridges. Aside
from cost-reduction, one special feature is that one can connect two cartridges
at once (so two games could share ROM or SRAM data). The BIOS in the adaptor
provides a huge character set, which may allow to reduce ROM size of the games.

[SNES Cart Sufami Turbo General Notes](#snes-cart-sufami-turbo-general-notes)

[SNES Cart Sufami Turbo ROM/RAM Headers](#snes-cart-sufami-turbo-romram-headers)

[SNES Cart Sufami Turbo BIOS Functions &amp; Charset](#snes-cart-sufami-turbo-bios-functions-charset)

<a id="snescartsufamiturbogeneralnotes"></a>

## SNES Cart Sufami Turbo General Notes

**Sufami Turbo Hardware**

The "adaptor" connects to the SNES cartridge socket, it contains the BIOS ROM,
and two slots for "mini-carts". Slot A for the game being played, Slot B can
contain another game (some games include features that allow to access game
position data from other games, some may also access ROM data from other
games).

**Sufami Turbo Memory Map**

```text
  00-1F:8000h-FFFFh  BIOS ROM (always 256Kbytes)             (max 1MByte)
  20-3F:8000h-FFFFh  Cartridge A ROM (usually 512Kbytes)     (max 1MByte)
  40-5F:8000h-FFFFh  Cartridge B ROM (usually 512Kbytes)     (max 1MByte)
  60-63:8000h-FFFFh  Cartridge A SRAM (usually 0/2/8 Kbytes) (max 128Kbyte)
  70-73:8000h-FFFFh  Cartridge B SRAM (usually 0/2/8 Kbytes) (max 128Kbyte)
  80-FF:8000h-FFFFh  Mirror of above banks
```

**Memory Notes**

The BIOS detects max 128Kbyte (64 pages) SRAM per slot, some games are (maybe
accidently) exceeding that limit (eg. Poi Poi Ninja zerofills 256 pages). Some
games (eg. SD Gundam Part 1) access SRAM slot B at 700000h rather than 708000h.

Some games (eg. SD Ultra Battle) may fail if the SRAM in slot B is
uninitialized (ie. before linking games in Slot A and B, first launch them
separately in Slot A).

When not using BIOS functions, one can safely destroy all WRAM locations,
except for WRAM[00000h] (which MUST be nonzero to enable the Game NMI handler
&amp; disable the BIOS NMI handler).

**Sufami Turbo ROM Images**

The games are typically 512Kbyte or 1MByte in size. Existing ROM-Images are
often 1.5Mbytes or 2MBytes - those files do include the 256KByte BIOS-ROM
(banks 00h-07h), plus three mirrors of the BIOS (banks 08h-1Fh), followed by
the actual 512Kbyte or 1MByte Game ROM (bank 20h-2Fh or 20h-3Fh).

There are also a few 3MByte ROM-images, with additional mirrors of the game
(bank 30h-3Fh), followed by a second game (bank 40h-4Fh), followed by mirrors
of the second game (bank 50h-5Fh).

That formats are simple (but very bloated) solutions to load the BIOS &amp;
Game(s) as a "normal" LoROM file.

**Sufami Turbo Games**

There have been only 13 games released:

```text
  Crayon Shin Chan
  Gegege No Kitarou
  Gekisou Sentai Car Ranger
  Poi Poi Ninja                    ;-link-able with itself (2-player sram)
  Sailor Moon Stars Panic 2
  SD Gundam Generations: part 1    ;\
  SD Gundam Generations: part 2    ;
  SD Gundam Generations: part 3    ; link-able with each other
  SD Gundam Generations: part 4    ;
  SD Gundam Generations: part 5    ;
  SD Gundam Generations: part 6    ;/
  SD Ultra Battle: Seven Legend    ;\link-able with each other
  SD Ultra Battle: Ultraman Legend ;/
```

All of them available only in Japan, released between June &amp; September
1996. Thereafter, the games may have been kept available for a while, but
altogether, it doesn't seem to have been a too successful product.

**Component List for Sufami Turbo Adaptor**

PCB "SHVC TURBO, BASE CASSETTE, BANDAI, PT-923"

```text
  IC1   18pin  unknown (CIC)
  IC2   16pin  "74AC139" or so
  IC3   40pin  SUFAMI TURBO "LH5326NJ" or so (BIOS ROM) (256Kbyte)
  IC4   8pin   unknown
  CP1   unknown (flashlight? oscillator? strange capacitor?)
  CN1   62pin  SNES cartridge edge (male)
  CN2   40pin  Sufami Cartridge Slot A (Game to be played)
  CN3   40pin  Sufami Cartridge Slot B (Other game to be "linked")
  C1..4  2pin  capacitors for IC1..4
  R1..4  2pin  resistors for unknown purpose
```

Note: Of the 62pin cartridge edge, only 43 pins are actually connected (the
middle 46 pins, excluding Pin 40,48,57, aka A15/A23/SYSCK).

**Component Lists for Sufami Turbo Game Carts**

All unknown. Probably contains only ROM, and (optionally) SRAM and Battery.
Physical SRAM size(s) are unknown (ie. unknown if there is enough memory for
more than one file). Cartridge slot pin-outs are unknown.

<a id="snescartsufamiturboromramheaders"></a>

## SNES Cart Sufami Turbo ROM/RAM Headers

**Sufami Turbo BIOS ROM Header**

The BIOS has a rather incomplete Nintendo-like header at ROM Offset 07FB0h
(mapped to 00FFB0h):

```text
  FFB0h Maker Code "B2"                        ;\extended header, present
  FFB2h Game Code "A9PJ"                       ; even though [FFDAh]<>33h
  FFB6h Reserved (10x00h)                      ;/
  FFC0h Title "ADD-ON BASE CASSETE  " (really mis-spelled, with only one "T")
  FFD4h Mapmode (always 30h = Fast LoROM)
  FFD5h Reserved (6x00h) (no ROM/RAM size entries, no ext.header-flag, etc.)
  FFDCh Dummy "checksum" value (always FFh,FFh,00h,00h)
  FFE0h Exception Vectors (IRQ,NMI,Entrypoint,etc.)
```

And, there is a header-like data field at ROM-Offset 00000h (mapped to 808000h)
(this part isn't really a header, but rather contains ID strings that are used
by the BIOS, for comparing them with Game ROM/SRAM):

```text
  8000h 16 "BANDAI SFC-ADX",0,0   ;Game ROM ID
  8010h 16 "SFC-ADX BACKUP",0,0   ;Game SRAM ID
```

**Sufami Turbo Game ROM Header (40h bytes)**

Located at ROM Offset 00000h (mapped to 208000h/408000h for Slot A/B):

```text
  00h 14 ID "BANDAI SFC-ADX" (required, compared against 14-byte ID in BIOS)
  0Eh 2  Zero-filled
  10h 14 Title, padded with spaces (can be 7bit ASCII and 8bit Japanese)
  1Eh 2  Zero-filled
  20h 2  Entrypoint (in bank 20h) ;game starts here (if it is in Slot A)
  22h 2  NMI Vector (in bank 20h) ;if RAM[000000h]=00h: use BIOS NMI handler
  24h 2  IRQ Vector (in bank 20h)
  26h 2  COP Vector (in bank 20h)
  28h 2  BRK Vector (in bank 20h)
  2Ah 2  ABT Vector (in bank 20h)
  2Ch 4  Zero-filled
  30h 3  Unique 24bit ID of a Game (or series of games) (usually 0xh,00h,0yh)
  33h 1  Index within a series (01h and up) (eg. 01h..06h for Gundam 1-6)
  34h 1  ROM Speed (00h=Slow/2.68Mhz, 01h=Fast=3.58MHz)
  35h 1  Chipset/Features (00h=Simple, 01h=SRAM or Linkable?, 03h=Special?)
  36h 1  ROM Size in 128Kbyte Units (04h=512K, 08h=1024K)
  37h 1  SRAM Size in 2Kbyte Units (00h=None, 01h=2K, 04h=8K)
  38h 8  Zero-filled
```

Some games have additional 64 header-like bytes at ROM Offset 40h..7Fh

```text
  40h 1  Program code/data in some carts, 00h or 01h in other carts
  41h 63 Program code/data in some carts, 00h-filled in other carts
```

The game cartridges don't use/need a Nintendo-like header at 7Fxxh/FFxxh, but
some games like SDBATTLE SEVEN do have one.

**Sufami Turbo SRAM File Header (30h bytes)**

```text
  0000h 15 ID "SFC-ADX BACKUP",0   ;Other = begin of free memory
  000Fh 1  Zero
  0010h 14 Title (same as 0010h..001Dh in ROM Header)
  001Eh 1  Zero
  001Fh 1  Zero (except, 01h in Poi Poi Ninja)
  0020h 4  Unique ID and Index in Series (same as 0030h..0033h in ROM Header)
  0024h 1  Filesize (in 2Kbyte units)    (same as 0037h in ROM Header)
  0025h 11 Zero-filled
```

The BIOS file-functions are only reading entry 0000h (ID) and 0024h (Filesize),
the BIOS doesn't write anything, all IDs and values must be filled-in by the
game.

SRAM is organized so that used 2Kbyte pages are at lower addresses, free pages
at higher addresses (deleting a file in the middle will relocate any pages at
higher addresses). Accordingly, files are always consisting of unfragmented
continous page numbers (leaving apart that there are 32Kbyte gaps in the memory
map).

<a id="snescartsufamiturbobiosfunctionscharset"></a>

## SNES Cart Sufami Turbo BIOS Functions & Charset

**Sufami Turbo BIOS Function Summary**

BIOS Function vectors (jmp 80xxxxh opcodes) are located at 80FF00h..80FF3Bh,

(the first 12 (of the 15) functions are also duplicated at 80FF80h..80FFAFh).

```text
  80FF00  FillSramPages  ;in: AL=num, AH=slot, XL=first, [09h]=fillword
  80FF04  CopySramToSram ;in: AL=num, AH=direction, X/Y=first (slot A/B)
  80FF08  CopySramToWram ;in: AL=num, AH=direction, X=first, Y=slot, [09h]=addr
  80FF0C  GetChar2bpp    ;in: A=char(0000h..0FFFh), [06h]=dest_addr (64 bytes)
  80FF10  GetChar4bpp    ;in: A=char(0000h..0FFFh), [06h]=dest_addr (128 bytes)
  80FF14  GetCartType    ;out: AL/AH=Types for Slot A/B, b0=ROM, b1=SRAM, b2=?
  80FF18  GetSramSize    ;out: AL/AH=Sizes for Slot A/B, 0-4=0,2,8,32,128Kbyte
  80FF1C  FindFreeSram   ;in: AL=slot, out: AL=first_free_page, FFh=none
  80FF20  GetSramAddrTo6 ;in: AL=slot, XL=page, out: [06h]=addr
  80FF24  GetSramAddrTo9 ;in: AL=slot, XL=page, out: [09h]=addr
  80FF28  ShowHelpSwap   ;display instructions how to exchange cartridges
  80FF2C  ShowHelpNoSwap ;display instructions how to remove cartridges
  80FF30  DeleteFile     ;in: AL=first, AH=slot
  80FF34  TestSramId     ;in: AL=page, AH=slot, out: CY: 0=Used, 1=Free
  80FF38  SramToSramCopy ;in: AL=num, X=src, Y=dst; XH/YH=slot, XL/YL=first
```

Whereas,

```text
  num = number of 2Kbyte pages
  slot = slot (0 or 1 for slot A or B)
  first = first 2Kbyte page number (of a file/area) (within selected slot)
  page = single 2Kbyte page number (within selected slot)
  addr = 24bit SNES memory address
  AL/AH, XL/XH, YL/YH = LSB/MSB of A,X,Y registers
```

The BIOS functions use first 16 bytes in WRAM [0000h..000Fh] for parameters,
return values, and internal workspace; when using BIOS functions, don't use
that memory for other purposes. [0000h] is NMI mode, don't change that even
when NOT using BIOS functions.

**File/SRAM Functions**

These functions may be (not very) helpful for managing SRAM, they are extremly
incomplete, there are no functions for creating files, or for searching
specific files. See the "Header" chapter for details on SRAM headers (again,
the BIOS doesn't create any headers or IDs, the game must fill-in all IDs,
Titles, and other values on its own).

**Character Set**

The BIOS ROM contains 4096 characters (each 16x16 pixel, aka 2x2 tiles). The
characters are stored at 1bit color depth in banks 04h..07h, offset 8000h-FFFFh
(20h bytes/character). The GetChar2bpp and GetChat4bpp functions can be used to
copy a selected character to WRAM, with bits in plane0, and the other plane(s)
zerofilled.

**Help Functions**

The two help functions are showing some endless repeated japanese instructions
about how to use, insert, remove, and exchange cartridges (similar to the
instructions shown when booting the BIOS without Game cartridges inserted). If
you have uploaded code to the APU, be sure to return control to the APU
boot-rom, otherwise the help functions will hang.

<a id="snescartxband2400baudmodem"></a>

## SNES Cart X-Band (2400 baud Modem)

The X-Band is a 2400 baud modem from Catapult Entertainment Inc., licensed by
Nintendo, originally released 1994 in USA, and 199x? in Japan. Aside from the
SNES version, there have been also Genesis and Saturn versions.

[SNES Cart X-Band Misc](#snes-cart-x-band-misc)

[SNES Cart X-Band I/O Map](#snes-cart-x-band-io-map)

[SNES Cart X-Band I/O - Memory Patch/Mapping](#snes-cart-x-band-io-memory-patchmapping)

[SNES Cart X-Band I/O - Smart Card Reader](#snes-cart-x-band-io-smart-card-reader)

[SNES Cart X-Band I/O - LED and Debug](#snes-cart-x-band-io-led-and-debug)

[SNES Cart X-Band I/O - Whatever Stuff (External FIFO for Modem?)](#snes-cart-x-band-io-whatever-stuff-external-fifo-for-modem)

[SNES Cart X-Band I/O - Rockwell Modem Ports](#snes-cart-x-band-io-rockwell-modem-ports)

[SNES Cart X-Band Rockwell Notes](#snes-cart-x-band-rockwell-notes)

[SNES Cart X-Band BIOS Functions](#snes-cart-x-band-bios-functions)

[SNES Controllers X-Band Keyboard](50-controllers.md#snes-controllers-x-band-keyboard)

**Note**

There's also another modem (which connects to controller port):

[SNES Add-On SFC Modem (for JRA PAT)](50-controllers.md#snes-add-on-sfc-modem-for-jra-pat)

<a id="snescartxbandmisc"></a>

## SNES Cart X-Band Misc

**Info...**

It was used for networked gaming via phone lines.

The Xband worked by sending controller instructions, by intercepting code from
the game, and patching it with its own instructions, much like the Game Genie
works. (that are, probably, two separate features messed into one sentence?)

The system worked by dialing up the main server, which was located in
Cupertino, California (USA), and somewhere else (Japan). The server then sent
the Xband newsletters (called Bandwidth and Xband News). It also sent any
patches that were needed. You could then search for opponents.

**Unknown Features**

There seems to be no CIC chip, so the BIOS does likewise work only with another
SNES cart connected.

There is switch, for whatever on/off/mode selection. There are three LEDs for
whatever purpose. And, there is some kind of a credit-card (or so) reader.

**Memory Map**

```text
  D00000h-DFFFFFh  1MB ROM (executed here, not at C00000h-CFFFFFh)
  E00000h-E0FFFFh  64K SRAM (in two 32Kx8 chips) (unknown if BOTH have battery)
  FBC000h-FBC17Fh  I/O Ports (unknown functions?)
  FBC180h-FBC1BFh  I/O Ports (Rockwell Modem Chip)
  FBFC02h          I/O Port  (unknown functions?)
  FBFE00h          I/O Port  (unknown functions?)
  FFC000h          I/O Port  (unknown functions?)
  004F02h          I/O Port  (unknown functions?)
  00F000h          Dummy/strobe read?
  00FFE0h          Dummy/strobe read?
```

I/O Ports seem to be 8bit-wide / word-aligned (ie. one can use 8bit or 16bit
writes, with the MSB ignored in the latter case). Normally ONLY the even
addresses are used (some exceptions are: 8bit write 00h to FBC153h, 16bit write
0000h to FBC160h).

Some of the I/O ports outside of the FBCxxxh region might belong to other
hardware? (eg. the X-Band might automatically disable any Game Genie BIOS in
order to access the Game ROM).

**Unknown 100pin Chip**

Unknown. Probably controls the cart reader, the cheat/patching feature, and
maybe also memory &amp; I/O mapping of the other chips.

**Games supported by the X-Band modem**

```text
  Doom                           +
  Ken Griffey Jr. Baseball       ? (not listed in stats)
  Killer Instinct                +
  Madden NFL '95                 +
  Madden NFL '96                 +
  Mortal Kombat II               +
  Mortal Kombat 3                +
  NBA Jam TE                     +
  NHL '95                        ? (not listed in stats)
  NHL '96                        ? (not listed in stats)
  Super Mario Kart               +
  Weaponlord                     + (listed in sf2dxb stats only)
```

and,

```text
  Kirby's Avalanche              +
  Super Street Fighter II        +
  The Legend of Zelda: A Link to the Past (secret maze game)   +
  Super Mario World (chat function)
```

"First of all, the Legend of Zelda wasn't the only cartridge that would
activate the hidden maze game -- basically, any unsupported SNES cart would do
it. I usually used Super Mario World."

CZroe: "Zelda triggered the XBAND's built-in maze game (someone reported that
their copy didn't work... Zelda 1.1?!). Mario World triggered the Chat
function."

CZroe: "This is how I identified that there was a second version of Killer
Instinct long before it debuted on this site (all US Killer Instinct bundle

SNES consoles would not work with the XBAND)."

gainesvillefrank: "I remember XBAND tried this experimental use of Mario World
after a while. If you dialed in to XBAND with Mario World in your SNES then it
would treat the cartridge as a chat room."

"The black switch on the side needs to be in the down position. Otherwise it
passes through."

Most of the above games, don't include any built-in Xband support, instead,
Catapult reverse-engineered how they work, and patched them to work with the
modem. Exceptions are Weaponlord (and Doom?), which were released with "modem
support" (unknown what that means exactly... do they control modem I/O ports...
interact with the modem BIOS... or are they patched the same way as other
games, and the only difference is that the developers created the patches
before releasing the game?)

Note: The japanese BIOS does read the Game cartridge header several times
(unlike the US version which reads it only once), basically there is no good
reason for those multiple reads, but it might indicate the japanese version
includes multiple patches built-in in ROM?)

**CODES/SECRETS (still working, even when offline)**

Maze mini-game

Press Down(2), Left(2), Right, B at the main menu.

Blockade mini-game (tron clone)

Press Up(2), Left, Right, Left(2), Right, L at the main menu.

Fish Pong mini-game

Genesis only?

Change Font

To change the text font, enter these codes at the Player Select screen.

Green and yellow font - Up, Up, Right, Right, Down, Down, Left

Rainbow font - Left, Left, Up, Up, Right, Right, Down

Searchlight font - Down, Down, Left, Left, Up, Up, Right

Alternate screen

Press Up, Up, Left, Right on the title screen.

Screen Saver

Press Left, Right, Down, Down, R at the "X-Mail" and "Newsletters" screens.

**SNES X-Band SRAM Dumps**

```text
  benner  3.26.97 (main character with most stats is lower-right)
  sf2dxb  4.30.97
  luke2   3.1.97
```

contains stats (for played game titles; separately for each of the 4 player
accounts), and the most recent bandwidth/newletter magazines, and x-mails.

**PCB "123-0002-16, Cyclone Rev 9, Catapult (C) 1995" Component List**

```text
  U1   28pin Winbond W24257S-70L                    (32Kx8 SRAM)
  U2   36pin X X, X BAND, X X, SNES US ROM 1.0.1    (BIOS ROM)
  U3  100pin FredIIH, H3A4D1049, 9511 Korea (with Hyundai logo)
  U4   68pin RC2324DPL, R6642-14, Rockwell 91, 9439 A49172-2, Mexico
  U5    6pin LITEON 4N25 (optocoupler) (near TN0) (back side)
  U6   28pin Winbond W24257S-70L                    (32Kx8 SRAM)
  U7    6pin AT&T LF1504 (solid state relay) (near TN0) (back side)
  BT0   2pin Battery (not installed) (component side)
  BT200 2pin Battery (3V Lithium Penata CR2430) (back side)
  SW1   3pin Two-position switch (purpose unknown... battery off ??)
  J0   10pin Card-reader (for credit cards or so?) 8 contacts, plus 2pin switch
  J1   62pin SNES Cartridge Edge (to be plugged into the SNES console)
  J2   62pin SNES Cartridge Slot (for game-cart plugged on top of the modem)
  J3  4/6pin RJ socket (to phone line)
  Y1    2pin Oscillator (R24AKBB4, =24MHz or so?) (back side)
  TN0   4pin Transformator (671-8001 MIDCOM C439)
  LEDs       Three red LEDs (purpose/usage unknown?)
```

**PCB "123-0002-17, Catapult (C) 1995" Component List**

```text
  MODEM is  "RC2424DPL, R6642-25, Rockwell 91, 9507 A61877.2, Hong Kong"
```

**PCB "123-0003-04, Tornado, Catapult (C) 1995" (Japan)**

```text
  SRAMs are "SEC KOREA, 550A, KM62256CLG-7L"
  BIOS  is  "X X 9549, X BAND, X X, SUPER FAMICOM, ROM1.0"
  FRED  is  "Catapult, FRED5S, 549D" (100pin)
  MODEM is  "RC2424DPL, R6642-25, Rockwell 91, 9609 A62975-2, Mexico"
  Y1    is  "A24.000"
  BT201 is  "C?2032" (installed instead of bigger BT200)
```

<a id="snescartxbandiomap"></a>

## SNES Cart X-Band I/O Map

Below I/O Map is based on source code of the Sega Genesis X-Band version (files
i\harddef.a and i\feq.a). The I/O Map of the SNES version might differ in some
places.

default base addresses

```text
  kDefaultInternal:   equ     ($1de000*2)     ;;=3BC000h   ;aka SNES: FBC000h
  kDefaultControl:    equ     ($1dff00*2)     ;;=3BFE00h   ;aka SNES: FBFE00h
```

**X-Band I/O Map**

```text
  Addr  $nn*2 i\harddef.a      i\feq.a     ;Comment
  ----------------------------------------------------------------------------
  C000h $00*2 kPatch_0_Byte0   -   (lo?)   ;Translation (Patch Addr) regs ...
  C002h $01*2 kPatch_0_Byte1   -   (mid?)  ;(aka "Vectors 0..10"?)
  C004h $02*2 kPatch_0_Byte2   -   (hi?)
  C006h       N/A              -
  C008h $04*2 kPatch_1_Byte0   -
  C00Ah $05*2 kPatch_1_Byte1   -
  C00Ch $06*2 kPatch_1_Byte2   -
  C00Eh       N/A              -
  C010h $08*2 kPatch_2_Byte0   -
  C012h $09*2 kPatch_2_Byte1   -
  C014h $0A*2 kPatch_2_Byte2   -
  C016h       N/A              -
  C018h $0C*2 kPatch_3_Byte0   -
  C01Ah $0D*2 kPatch_3_Byte1   -
  C01Ch $0E*2 kPatch_3_Byte2   -
  C01Eh       N/A              -
  C020h $10*2 kPatch_4_Byte0   -
  C022h $11*2 kPatch_4_Byte1   -
  C024h $12*2 kPatch_4_Byte2   -
  C026h       N/A              -
  C028h $14*2 kPatch_5_Byte0   -
  C02Ah $15*2 kPatch_5_Byte1   -
  C02Ch $16*2 kPatch_5_Byte2   -
  C02Eh       N/A              -
  C030h $18*2 kPatch_6_Byte0   -
  C032h $19*2 kPatch_6_Byte1   -
  C034h $1A*2 kPatch_6_Byte2   -
  C036h       N/A              -
  C038h $1C*2 kPatch_7_Byte0   -
  C03Ah $1D*2 kPatch_7_Byte1   -
  C03Ch $1E*2 kPatch_7_Byte2   -
  C03Eh       N/A              -
  C040h $20*2 kPatch_8_Byte0   -
  C042h $21*2 kPatch_8_Byte1   -
  C044h $22*2 kPatch_8_Byte2   -
  C046h       N/A              -
  C048h $24*2 kPatch_9_Byte0   -
  C04Ah $25*2 kPatch_9_Byte1   -
  C04Ch $26*2 kPatch_9_Byte2   -
  C04Eh       N/A              -
  C050h $28*2 kPatch_10_Byte0  -
  C052h $29*2 kPatch_10_Byte1  -
  C054h $2A*2 kPatch_10_Byte2  -
  C056h       N/A              -
  C058h $2C*2 kRange0Start     -
  C05Ah         ""-mid?        -
  C05Ch         ""-hi?         -
  C05Eh       N/A              -
  C060h $30*2 kRange1Start     -
  C062h         ""-mid?        -
  C064h         ""-hi?         -
  C066h       N/A              -
  C068h       N/A              -
  C06Ah       N/A              -
  C06Ch       N/A              -
  C06Eh       N/A              -
  C070h $38*2 kMagicAddrByte0  kmagicl
  C072h $39*2 kMagicAddrByte1  kmagicm
  C074h $3A*2 kMagicAddrByte2  kmagich
  C076h       N/A              -
  C078h       N/A              -
  C07Ah       N/A              -
  C07Ch       N/A              -
  C07Eh       N/A              -
  C080h $40*2 kRange0End       krangel
  C082h         ""-mid?        krangem
  C084h         ""-hi?         krangeh
  C086h       N/A              -
  C088h $44*2 kRange1End       -
  C08Ah         ""-mid?        -
  C08Ch         ""-hi?         -
  C08Eh       N/A              -
  C090h       N/A              -
  C092h       N/A              -
  C094h       N/A              -
  C096h       N/A              -
  C098h       N/A              -
  C09Ah       N/A              -
  C09Ch       N/A              -
  C09Eh       N/A              -
  C0A0h $50*2 kRange0Dest      ktrbl
  C0A2h         ""-hi?         ktrbh
  C0A4h $52*2 kRange0Mask      ktrm
  C0A6h       N/A              -
  C0A8h $54*2 kRange1Dest      -
  C0AAh         ""-hi?         -
  C0ACh $56*2 kRange1Mask      -
  C0AEh       N/A              -
  C0B0h       N/A              -
  C0B2h       N/A              -
  C0B4h       N/A              -
  C0B6h       N/A              -
  C0B8h       N/A              -
  C0BAh       N/A              -
  C0BCh       N/A              -
  C0BEh       N/A              -
  C0C0h $60*2 kRAMBaseByte0    ksaferambasel
  C0C2h $61*2 kRAMBaseByte1    ksaferambaseh
  C0C4h       N/A              -
  C0C6h       N/A              -
  C0C8h $64*2 kRAMBoundByte0   ksaferambndl
  C0CAh $65*2 kRAMBoundByte1   ksaferambndh
  C0CCh       N/A              -
  C0CEh       N/A              -
  C0D0h $68*2 kVTableBaseByte0 kvtablel ;\vector table base address?
  C0D2h $69*2 kVTableBaseByte1 kvtableh ;/ (in 32-byte, or 32-word steps maybe?)
  C0D4h       N/A              -
  C0D6h       N/A              -
  C0D8h $6c*2 kEnableByte0     kenbll
  C0DAh $6d*2 kEnableByte1     kenblh
  C0DCh       N/A              -
  C0DEh       N/A              -
  C0E0h $70*2 kROMBound        ksaferombnd
  C0E2h       N/A              -
  C0E4h       N/A              -
  C0E6h       N/A              -
  C0E8h $74*2 kROMBase         ksaferombase
  C0EAh       N/A              -
  C0ECh       N/A              -
  C0EEh       N/A              -
  C0F0h       N/A              -              ;<-- but this is used on SNES !?!
  C0F2h       N/A              -              ;<-- but this is used on SNES !?!
  C0F4h       N/A              -
  C0F6h       N/A              -
  C0F8h $7c*2 kAddrStatus      kaddrstatusl
  C0FAh         ""-hi?         kaddrstatush
  C0FCh       N/A              -
  C0FEh       N/A              -
  C100h $80*2 kSControl        ksctl                  ;smart card control
  C102h       N/A              -
  C104h       N/A              -
  C106h       N/A              -
  C108h $84*2 kSStatus         ksstatus               ;smart card status
  C10Ah       N/A              -
  C10Ch       N/A              -
  C10Eh       N/A              -
  C110h $88*2 kReadMVSyncLow   kreadmvsync    ;<--Low? ;\Range of 0 to $61.
  C112h $89*2 kReadMVSyncHigh  kreadmvsynclow ;<--low? ;/Equal to
  C114h       N/A              -                       ; ReadSerialVCnt/2.
  C116h       N/A              -                       ; Value is $5c at start
  C118h $8c*2 kMStatus1        kmstatus1               ; of VBlank.
  C11Ah       N/A              -
  C11Ch       N/A              -
  C11Eh       N/A              -
  C120h $90*2 kTxBuff          ktxbuff            ; modem (and serial) bits ...
  C122h       N/A              -
  C124h       N/A              -
  C126h       N/A              -
  C128h $94*2 kRxBuff          krxbuff
  C12Ah       N/A              -
  C12Ch       N/A              -
  C12Eh       N/A              -
  C130h $98*2 kReadMStatus2    kreadmstatus2
  C132h       N/A              -
  C134h       N/A              -
  C136h       N/A              -
  C138h $9c*2 kReadSerialVCnt  kreadserialvcnt
  C13Ah       N/A              -
  C13Ch       N/A              -
  C13Eh       N/A              -
  C140h $a0*2 kReadMStatus1    kreadmstatus1
  C142h       N/A              -
  C144h       N/A              -
  C146h       N/A              -
  C148h $a4*2 kGuard           kguard
  C14Ah       N/A              -
  C14Ch       N/A              -
  C14Eh       N/A              -
  C150h $a8*2 kBCnt            kbcnt
  C152h       N/A              -
  C154h       N/A              -
  C156h       N/A              -
  C158h $ac*2 kMStatus2        kmstatus2
  C15Ah       N/A              -
  C15Ch       N/A              -
  C15Eh       N/A              -
  C160h $b0*2 kVSyncWrite      kvsyncwrite
  C162h       N/A              -
  C164h       N/A              -
  C166h       N/A              -
  C168h $b4*2 kLEDData         kleddata
  C16Ah $b5*2 kLEDEnable       kledenable
  C16Ch       N/A              -
  C16Eh       N/A              -
  C170h       N/A              -
  C172h       N/A              -
  C174h       N/A              -
  C176h       N/A              -
  C178h       N/A              -
  C17Ah       N/A              -
  C17Ch       N/A              -
  C17Eh       N/A              -
  C180h $c0*2 kModem           - ;<-- base for rockwell registers (C180h-C1BEh)
```

```text
  FC02h       N/A              -      ;<-- unknown, but this is used by SNES
  FE00h $00*2 kKillReg         kkillhere ;same as killheresoft...trans register
  FE02h $01*2 kControlReg      kreghere
  FF80h $c0*2 kKillHereSoft    kkillheresoft  ;\maybe some sort of mirrors of
  FF82h $c1*2 kCtlRegSoft      kctlregsoft    ;/FE00h and FE02h ?
```

```text
  617000h ? weirdness kSNESKillHereSoft       ;\maybe some sort of mirrors of
  617001h ? weirdness kSNESCtlRegSoft         ;/FE00h and FE02h ?
```

```text
  FFC000h          I/O Port  (unknown functions?) ;-bank FFh ;\
  004F02h          I/O Port  (unknown functions?) ;\         ; whatever, used
  00F000h          Dummy/strobe read?             ; bank 00h ; by SNES version
  00FFE0h          Dummy/strobe read?             ;/         ;/
```

<a id="snescartxbandiomemorypatchmapping"></a>

## SNES Cart X-Band I/O - Memory Patch/Mapping

**FE00h - KillReg (aka killhere) ;same as killheresoft...trans register**

;kill register bits  ;aka "kKillReg" and/or "kKillHereSoft"?

```text
  0   HereAssert:      equ     $01 ; "Here" = cannot see cart
  1   Unknown/unused
  2   DecExcept:       equ     $04 ;
  3   Force:           equ     $08 ;
  4-7 Unknown/unused
```

**FE02h - ControlReg (aka reghere)**

;control bits for control register ;aka "kControlReg"? and/or "kCtlRegSoft"?

```text
  0   EnTwoRam:        equ     $01 ;<-- maybe disable one of the two SRAMs?
  1   EnSafeRom:       equ     $02 ;<-- maybe SRAM read-only? or FlashROM?
  2   RomHi:           equ     $04 ;
  3   EnInternal:      equ     $08 ;<-- maybe disable ports C000h..C1FFh?
  4   EnFixedInternal: equ     $10 ;<-- maybe whatever related to above?
  5   EnSNESExcept:    equ     $20 ;
  6-7 Unknown/unused
```

**FF80h - KillHereSoft (aka kkillheresoft)**

**FF82h - CtlRegSoft (aka kctlregsoft)**

Unknown, maybe some sort of mirrors of FE00h and FE02h ?

**617000h ? weirdness kSNESKillHereSoft**

**617001h ? weirdness kSNESCtlRegSoft**

Unknown, maybe some sort of mirrors of FE00h and FE02h ?

Maybe non-Sega, SNES only stuff? Or maybe weird/ancient prototype stuff?

**C000h/C002h/C004h - Patch 0, Byte0/Byte1/Byte2 (Lo/Mid/Hi?)**

**C008h/C00Ah/C00Ch - Patch 1, Byte0/Byte1/Byte2 (Lo/Mid/Hi?)**

**C010h/C012h/C014h - Patch 2, Byte0/Byte1/Byte2 (Lo/Mid/Hi?)**

**C018h/C01Ah/C01Ch - Patch 3, Byte0/Byte1/Byte2 (Lo/Mid/Hi?)**

**C020h/C022h/C024h - Patch 4, Byte0/Byte1/Byte2 (Lo/Mid/Hi?)**

**C028h/C02Ah/C02Ch - Patch 5, Byte0/Byte1/Byte2 (Lo/Mid/Hi?)**

**C030h/C032h/C034h - Patch 6, Byte0/Byte1/Byte2 (Lo/Mid/Hi?)**

**C038h/C03Ah/C03Ch - Patch 7, Byte0/Byte1/Byte2 (Lo/Mid/Hi?)**

**C040h/C042h/C044h - Patch 8, Byte0/Byte1/Byte2 (Lo/Mid/Hi?)**

**C048h/C04Ah/C04Ch - Patch 9, Byte0/Byte1/Byte2 (Lo/Mid/Hi?)**

**C050h/C052h/C054h - Patch 10, Byte0/Byte1/Byte2 (Lo/Mid/Hi?)**

aka "Vectors 0..10"?

**C070h/C072h/C074h - MagicAddr Byte0/1/2 (Lo/mid/hi) (aka magicl/m/h)**

```text
  0-23 Unknown (also referred to as "transition address"?)
```

**C058h/C05Ah/C05Ch - Range0Start (Lo/mid/hi?)**

**C060h/C062h/C064h - Range1Start (Lo/mid/hi?)**

**C080h/C082h/C084h - Range0End (Lo/mid/hi) (aka rangel/m/h)**

**C088h/C08Ah/C08Ch - Range1End (Lo/mid/hi?)**

```text
  0-23 Unknown (maybe ROM start/end addresses for BIGGER patch regions?)
```

**C0A0h/C0A2h - Range0Dest (Lo/hi) (aka trbl/h)**

**C0A8h/C0AAh - Range1Dest (Lo/hi?)**

```text
  0-15 Unknown (maybe SRAM mapping target for above ROM ranges?)
```

**C0A4h - Range0Mask (aka trm)**

**C0ACh - Range1Mask**

```text
  0-7  Unknown
```

**C0D0h/C0D2h - VTableBase Byte0/1 (Lo/hi) (aka kvtablel/h)**

```text
  0-15 Unknown (maybe SRAM mapping target for ROM patch vectors?)
```

vector table base address? (in 32-byte, or 32-word steps maybe?)

**C0D8h/C0DAh - Enable Byte0/1 (Lo/hi) (aka enbll/h)**

```text
  0-10 Vector 0-10 Enable (aka enable "kPatch_0..10"?) (?=off, ?=on)
  11   range0ena
  12   range1ena
  13   unknown/unused
  14   transAddrEnable aka magicena   ;enable transition address
  15   zeroPageEnable                 ;enable zero page  <-- game cart access?
```

**C0C0h/C0C2h - RAMBase, Byte0/1 (Lo/hi) (aka saferambasel/h)**

```text
  0-15 Unknown
```

**C0C8h/C0CAh - RAMBound Byte0/1 (Lo/hi) (aka saferambndl/h)**

```text
  0-15 Unknown
```

**C0E0h - ROMBound (aka saferombnd)**

```text
  0-7  Unknown
```

**C0E8h - ROMBase (aka saferombase)**

```text
  0-7  Unknown
```

**C0F8h/C0FAh - AddrStatus (Lo/hi?) (aka addrstatusl/h)**

```text
  0-15 Unknown
```

<a id="snescartxbandiosmartcardreader"></a>

## SNES Cart X-Band I/O - Smart Card Reader

The X-Band contains a built-in Smart Card reader (credit card shaped chip cards
with 8 gold contacts). The X-Band BIOS contains messages that refer to "XBand
Cards" and "XBand Rental Cards". There aren't any photos (or other info) of
these cards in the internet, maybe X-Band requested customers to return the
cards, or the cards got lost for another reason.

**Purpose**

Not much known. Reportedly the card reader was used for Prepaid Cards (for
users whom didn't want Xband to charge their credit cards automatically), if
that is correct, then only those users would have received cards, and other
users didn't need to use the card reader? Note: The Options/Account Info screen
show entries "Account" and "Card".

**Smart Card I/O Ports**

The BIOS seems to be accessing the cards via these I/O ports:

```text
  FBC100h Card Data/Control/Whatever (out)
  FBC108h.Bit0 (In) Card Switch (1=card inserted, 0=card missing)
  FBC108h.Bit1 (In) Card Data (input)
```

Related BIOS functions are function 0380h..0386h (on SNES/US).

**C100h - SControl (aka sctl) ;smart card control**

```text
  0   outputClk:   equ     $01
  1   enOutputData:equ     $02  ;aka data direction?
  2   outputData:  equ     $04
  3   outputReset: equ     $08
  4   outputVcc:   equ     $10
  5-7 unknown/unused
```

**C108h - SStatus (aka sstatus) ;smart card status**

```text
  0   detect:      equ     $01 Card Switch (1=card inserted, 0=card missing)
  1   dataIn:      equ     $02 Card Data (input)
  2   outputClk:   equ     $04   ;\
  3   enOutputData:equ     $08   ; Current state of Port C100h.Bit0-4 ?
  4   outputData:  equ     $10   ;
  5   outputReset: equ     $20   ;
  6   outputVcc:   equ     $40   ;/
  7   outputVpp:   equ     $80   ;-Current state of Port ????.Bit0 ?
```

**???? - Smart Card control ii**

parameters for control ii  ;&lt;-- from i\feq.a (uh, "control ii" is what?)

```text
  0   ksoutputvpp: equ     $01
  1-7 unknown/unused
```

```text
               _______ _______
       VCC C1 |       |       | C5 GND          common smart card pinout
              |____   |   ____|                 (unknown if xband is actually
       RST C2 |    \__|  /    | C6 VPP          using that same pinout)
              |____/     \____|
       CLK C3 |    \_____/    | C7 I/O
              |____/  |  \____|
       NC? C4 |       |       | C8 NC?
              |_______|_______|
```

<a id="snescartxbandioledanddebug"></a>

## SNES Cart X-Band I/O - LED and Debug

**C168h - LEDData (aka leddata)**

```text
  0-7  probably controls the LEDs (can be also used for other stuff)
```

Note: Sega version has 7 LEDs, SNES version has only 3 LEDs. Unknown which of
the 8 bits are controlling which LEDs.

**C16Ah - LEDEnable (aka ledenable)**

```text
  0-7  seems to select data-direction for LED pins (0=input, 1=output)
```

**Debug Connection via LED ports**

People at Catapult have reportedly used modified X-Band PCBs during debugging:
The seven genesis LEDs replaced by a DB25 connector with 8 wires (7 debug
signals, plus GND, probably connected to a PC parallel/printer port). That
hardware mod also used special software (some custom X-Band BIOS on FLASH/ROM,
plus whatever software on PC side).

**Unknown 64bit Number via LED ports**

The SNES X-Band BIOS is reading a 64bit number via serial bus (which might
connect to exteral debug hardware, or to 'unused' smart card pins, or to
whatever), done via two I/O Ports:

```text
  FBC168h Data        ;bit2 (data, in/out)       ;\there is maybe also a reset
  FBC16Ah Direction   ;bit2 (0=input, 1=output)  ;/flag, eventually in bit5 ?
```

The sequence for reading the 64bits is somewhat like so:

```text
  Data=Output(0), Delay (LOOPx01F4h)
  Data=Output(1), Delay (LOOPx01F4h)
  Data=Input
  wait until Data=1 or fail if timeout
  wait until Data=0 or fail if timeout
  wait until Data=1 or fail if timeout
  Delay (LOOPx02BCh)
  for i=1 to 4
    Data=Output(0), Delay (NOPx8)
    Data=Output(1), Delay (NOPx8)
    Data=Input, Delay (LOOPx0050h)
  for i=1 to 4
    Data=Output(0), Delay (NOPx8)
    Data=Output(1), Delay (LOOPx003Ch)
    Data=Input, Delay (LOOPx001Eh)
  Data=Input, Delay (LOOPx0064h)
  for i=0 to 63
    Data=Output(1), Delay (NOPx8)
    Data=Input, Delay (LOOPx000Ah)
    key.bit(i)=Data, Delay (LOOPx004Bh)
```

For the exact timings (Delays and other software overload), see the BIOS
function (at D7BE78h). Before doing the above stuff, the BIOS initializes
[FBC168h]=40h, [FBC16Ah]=FFh (this may be also required).

The 64bit number is received LSB first, and stored in SRAM at 3FD8h-3FDFh.
Whereas the last byte is a checksum across the first 7 bytes, calculated as so:

```text
  sum=00h
  for i=0 to 55
    if (sum.bit(0) xor key.bit(i))=1 then sum=sum/2 xor 8Ch else sum=sum/2
```

For example, if the 7 bytes are "testkey", then the 8th byte must be 2Fh. Or,
another simplier example would be setting all 8 bytes to 00h.

<a id="snescartxbandiowhateverstuffexternalfifoformodem"></a>

## SNES Cart X-Band I/O - Whatever Stuff (External FIFO for Modem?)

Below is some additional modem stuff (additionally to the normal Rockwell Modem
registers at C180h-C1BEh). The original source code refers to that extra stuff
as "modem (and serial) bits". Purpose is unknown...

Maybe the Rockwell Modem chip lacks internal FIFOs, so the VBlank handler could
transfer max 60 bytes/second. As a workaround, the Fred chip might contain some
sort of external FIFOs, allowing to send around 4 bytes per Vblank (which would
gain 240 bytes/second, ie. gaining the full bandwidth of the 2400 baud modem).

If so, then the Fred chip should be wired either to the Rockwell databus, or to
the Rockwell serial bus. Despite of the possible FIFO feature, directly
accessing the Rockwell RX/TX registers seems to be also supported.

**C118h - MStatus1 (aka mstatus1)**

```text
  0  enModem
  1  resetModem
  2  bit_8
  3  enstop
  4  onestop
  5  enparity
  6  oddparity
  7  break
```

**C120h - TxBuff (aka txbuff)**

**C128h - RxBuff (aka rxbuff)**

Some TX/RX FIFOs?

**C130h - ReadMStatus2 (aka readmstatus2)**

```text
  0   kRMrxready:    rxready:         equ $01  ;1 = have rx data, 0 = no data
  1   kRMframeerr:   ltchedframeerr:  equ $02
  2   kRMparityerr:  ltchedparityerr: equ $04
  3-4 kRMframecount: sfcnt:           equ $18
  6-7 unknown/unused
```

Bit3-4 is a 2bit framecounter to tell whether a byte arrived this frame or a
prev frame. It's a little wacky to use because unlike VCnt, there is no
separate place to read it on Fred other than right here, sharing it with the
FIFO. So, you must do the following:

If there is data in the FIFO, framecount reflects the frame number of the
oldest byte in the FIFO. If the FIFO is empty, however, it reflects the current
frame number. Used carefullly (i.e. make sure rxready is 0 if you are using it
for the current framecount), it should allow you to determine if a byte arrived
in the current frame or up to 3 previous frames ago.

**C140h - ReadMStatus1 (aka readmstatus1)**

```text
  0   txfull:       equ $01      ; 1 = full, 0 = not full
  1   txempty:      equ $02
  2   rxbreak:      equ $04
  3   overrun:      equ $08
  4-5 smartrxretry: equ $30   smartrxnumretry:      equ     $30
  6-7 smarttxretry: equ $c0   smarttxnumretry:      equ     $c0
```

**C148h - Guard (aka guard)**

```text
  0-7 unknown
```

**C150h - BCnt (aka bcnt)**

```text
  0-7 unknown (whatever... B control... or B counter?)
```

**C158h - MStatus2 (aka mstatus2)**

```text
  0   ensmartrxretry: equ     $1 ;
  1   ensmarttxretry: equ     $2 ;
  2   smart:          equ     $4 ;
  3   sync:           equ     $8 ;
  4-7 unknown/unused
```

**C160h - VSyncWrite (aka vsyncwrite)**

```text
  0-7 unknown
```

Maybe the vsync/vblank handler must write here by software in order to reset to
"V" counters?

**C110h - ReadMVSyncLow (aka readmvsync)     ;&lt;--Low?**

**C112h - ReadMVSyncHigh (aka readmvsynclow) ;&lt;--low?**

```text
  Range of 0 to $61. Equal to ReadSerialVCnt/2.
  Value is $5c at start of VBlank.
```

**C138h - ReadSerialVCnt (aka readserialvcnt)**

```text
  0-7 some incrementing counter...
             ;i\feq.a: kreadserialvcnt
             ; top 8 bits of 20 bit counter tied to
             ; input clock, or it increments 1 each 4096 clks
             ; resets to zero at vblank
             ; at 24 mhz, each 170.667 usec
             ; in 1/60 sec, counts up to 97 ($61), so
             ; range is 0 to $61 (verified by observation)
```

```text
                     ;i\harddef.a: kReadSerialVCnt
                     ; Top 8 bits of 19 bit counter tied to
  kFirstVCnt equ $5c ; input clock, i.e. it increments 1 each 2048 clks.
  kLastVCnt  equ $5b ; At 24 MHz, each 85.333 usec
  kMaxVCnt   equ $61 ; in 1/60 sec, counts up to 195 ($C3), so
  kMinVCnt   equ $00 ; range is 0 to $C3 (not yet verified by testing)
                     ; Value is about $B8 at start of vblank, counts up to $C3,
                     ; wraps to 0.  Note that ReadMVSyncHigh VCnt is equal
                     ; ReadSerialVCnt/2. Note also that if there is no data
                     ; in the read fifo, it appears that ReadSerialVCnt has
                     ; the value of ReadMVSyncHigh (i.e. 1/2 the resolution)
```

```text
  kVCntsPerModemBit: equ     $5 ; 1 modem bit time is 1/2400 sec, or 417 usec
                                ; 417/85.333 (1 VCnt) = 4.89, rounded up gives
                                ; 5 VCnts per modem bit. Not that this refers
                                ; to ReadSerialVCnt.
```

```text
  kLinesPerModemBit: equ     $7 ; 417/64 (1 horiz line time) = 6.51, rounded up
                                ; gives 7 Lines per modem bit
```

```text
  ; for rx:
  ; 1. read status until rxready
  ; 2. read serialVcnt               <-- uhm, what/why?
  ; 3. read Rxbuff (reading rxbuff clears the full fifo entry)
```

<a id="snescartxbandiorockwellmodemports"></a>

## SNES Cart X-Band I/O - Rockwell Modem Ports

Below are the I/O Ports of the Rockwell chip. In the SNES, Rockwell registers
00h-1Fh are mapped to EVEN memory addresses at FBC180h-FBC1BEh. The chip used
in the SNES supports data/voice modem functions (but not fax modem functions).

**FBC180h/FBC182h - 00h/01h - Receive Data Buffer**

```text
  0-7  RBUFFER Received Data Buffer. Contains received byte of data
  8    RXP     Received Parity bit (or ninth data bit)
  9-15 N/A     Unused
```

**FBC184h/FBC186h - 02h/03h - Control**

```text
  0-8  N/A     Unused
  9    GTE     TX 1800Hz Guard Tone Enable (CCITT configuration only)
  10   SDIS    TX Scrambler Disable
  11   ARC     Automatic on-line Rate Change sequence Enable
  12   N/A     Unused
  13   SPLIT   Extended Overspeed TX/RX Split. Limit TX to basic overspeed rate
  14   HDLC    High Level HDLC Protocol Enable (in parallel data mode)
  15   NRZIE   Unknown (listed in datasheet without further description)
```

**FBC188h/FBC18Ah - 04h/05h - Control**

```text
  0     CRFZ   Carrier Recovery Freeze. Disable update of receiver's carrier
               recovery phase lock loop
  1     AGCFZ  AGC Freeze. Inhibit updating of receiver AGC
  2     IFIX   Eye Fix. Force EYEX and EYEY serial data to be rotated
               equalizer output
  3     EQFZ   Equalizer Freeze. Inhibit update of receiver's adaptive
               equalizer taps
  4-5   N/A    Unused
  6     SWRES  Software Reset. Reinitialize modem to its power turn-on state
  7     EQRES  Equalizer Reset. Reset receiver adaptive equalizer taps to zero
  8     N/A    Unused
  9     TXVOC  Transmit Voice. Enable sending of voice samples
  10    RCEQ   Receiver Compromise Equalizer Enable. Control insertion of
               receive passband digital compromise equalizer into receive path
  11    CEQ(E) Compromise Equalizer Enable. Enable transmit passband digital
               compromise equalizer
  12    TXSQ   Transmitter Squelch. Disable transmission of energy
  13-15 N/A    Unused
```

**FBC18Ch/FBC18Eh - 06h/07h - Control**

```text
  0,1 WDSZ   Data Word Size, in asynchronous mode (5, 6, 7, or 8 bits)
  2   STB    Stop Bit Number (number of stop bits in async mode)
  3   PEN    Parity Enable (generate/check parity in async parallel data mode)
  4,5 PARSL  Parity Select (stuff/space/even/odd in async parallel data mode)
  6   EXOS   Extended Overspeed. Selects extended overspeed mode in async mode
  7   BRKS   Break Sequence. Send of continuous space in parallel async mode
  8   ABORT  HDLC Abort. Controls sending of continuous mark in HDLC mode
  9   RA     Relay A Activate. Activate RADRV output
  10  RB     Relay B Activate. Activate RBDVR output
  11  L3ACT  Loop 3 (Local Analog Loopback) Activate. Select connection of
             transmitter's analog output Internally to receiver's analog input
  12  N/A    Unused
  13  L2ACT  Loop 2 (Local Digital Loopback) Activate. Select connection of
             receiver's digital output Internally to transmitter's digital
             input (locally activated digital loopback)
  14  RDL    Remote Digital Loopback Request. Initiate a request for remote
             modem to go into digital loop-back
  15  RDLE   Remote Digital Loopback Response Enable. Enable modem to respond
             to remote modem's digital loopback request
```

**FBC190h/FBC192h - 08h/09h - Control**

```text
  0   RTS    Request to Send. Request transmitter to send data
  1   RTRN   Retrain. Send retrain-request or auto-rate-change to remote modem
  2   N/A    Unused
  3   TRFZ   Timing Recovery Freeze. Inhibit update of receiver's timing
             recovery algorithm
  4   DDIS   Descrambler Disable. Disable receiver's descrambler circuit
  5   N/A    Unused
  6   TPDM   Transmitter Parallel Data Mode. Select parallel/serial TX mode
  7   ASYNC  Asynchronous/Synchronous. Select sync/async data mode
  8   SLEEP  Sleep Mode. Enter SLEEP mode (wakeup upon pulse on RESET pin)
  9   N/A    Unused
  10  DATA   Data Mode. Select idle or data mode
  11  LL     Leased Line. Select leased line data mode or handshake mode
  12  ORG    Originate. Select originate or answer mode (see TONEC)
  13  DTMF   DTMF Dial Select. Select DTMF or Pulse dialing in dial mode
  14  CC     Controlled Carrier. Select controlled or constant carrier mode
  15  NV25   Disable V.25 Answer Sequence (Data Modes), Disable Echo Suppressor
             Tone (Fax Modes). Disable transmitting of 2100Hz CCITT answer tone
             when a handshake sequence is initiated in a data mode or disables
             sending of echo suppressor tone in a fax mode
```

**FBC194h/FBC196h - 0Ah/0Bh - Status**

```text
  0   CRCS   CRC Sending. Sending status of 2-byte CRC in HDLC mode
  1-7 N/A    Unused
  8   BEL1O3 Bell 103 Mark Frequency Detected. Status of 1270Hz Bell 103 mark
  9   DTDET  DTMF Digit Detected. Valid DTFM digit has been detected
  10  PNSUC  PN Success. Receiver has detected PN portion of training sequence
  11  ATBELL Bell Answer Tone Detected. Detection status of 2225Hz answer tone
  12  ATV25  V25 Answer Tone Detected. Detection status of 2100Hz answer tone
  13  TONEC  Tone Filter C Energy Detected. Status of 1650Hz or 980Hz (selected
             by ORG bit) FSK tone energy detection by Tone C bandpass filter in
             Tone Detector configuration
  14  TONEB  Tone Filter B Energy Detected. Status of 390Hz FSK tone energy
             detection by Tone B bandpass filter in Tone Detector configuration
  15  TONEA  Tone Filter A Energy Detected. Status of energy above threshold
             detection by Call Progress Monitor filter in Dial Configuration or
             1300 Hz FSK tone energy detection by Tone A bandpass filter in
             Tone Detector configuration
```

**FBC198h/FBC19Ah - 0Ch/0Dh - Status**

```text
  0-3 DTDIG  Detected DTMF Digit. Hexadecimal code of detected DTMF digit
  4-6 N/A    Unused
  7   EDET   Early DTMF Detect. High group frequency of DTMF tone pair detected
  8-9 N/A    Unused
  10  SADET  Scrambled Alternating Ones Sequence Detected
  11  U1DET  Unscrambled Ones Sequence Detected
  12  SCR1   Scrambled Ones Sequence Detected
  13  S1DET  S1 Sequence Detected
  14  PNDET  Unknown (listed in datasheet without further description)
  15  N/A    Unused
```

**FBC19Ch/FBC19Eh - 0Eh/0Fh - Status**

```text
  0-2 SPEED  Speed Indication. Data rate at completion of a connection
  3   OE     Overrun Error. Overrun status of Receiver Data Buffer (RBUFFER)
  4   FE     Framing Error. Framing error or detection of an ABORT sequence
  5   PE     Parity Error. Parity error status or bad CRC
  6   BRKD   Break Detected. Receipt status of continuous space
  7   RTDET  Retrain Detected. Detection status of a retrain request sequence
  8   FLAGS  Flag Sequence. Transmission status of Flag sequence in HDLC mode,
             or transmission of a constant mark in parallel asynchronous mode
  9   SYNCD  Unknown (listed in datasheet without further description)
  10  TM     Test Mode. Active status of selected test mode
  11  RI     Ring Indicator. Detection status of a valid ringing signal
  12  DSR    Data Set Ready. Data transfer state
  13  CTS    Clear to Send. Training sequence has been completed (see TPDM)
  14  FED    Fast Energy Detected. Energy above turn-on threshold is detected
  15  RLSD   Received Line Signal Detector (carrier and receipt of valid data)
```

**FBC1A0h/FBC1A2h - 10h/11h - Transmit Data Buffer**

```text
  0-7   TBUFFER Transmitter Data Buffer. Byte to be sent in parallel mode
  8     TXP     Transmit Parity Bit (or 9th Data Bit)
  9-15  N/A     Unused
```

**FBC1A4h/FBC1A6h - 12h/13h - Control**

```text
  0-7   CONF   Modem Configuration Select. Modem operating mode (see below)
  8-9   TXCLK  Transmit Clock Select (internal, disable, slave, or external)
  10-11 VOL    Volume Control. Speaker volume (off, low, medium, high)
  12-15 TLVL   Transmit Level Attenuation Select. Select transmitter analog
               output level attenuation in 1 dB steps. The host can fine tune
               transmit level to a value lying within a 1 dB step in DSP RAM
```

**FBC1A8h/FBC1AAh - 14h/15h - Unused**

```text
  0-15  N/A    Unused
```

**FBC1ACh/FBC1AEh - 16h/17h - Y-RAM Data (16bit)**

**FBC1B0h/FBC1B2h - 18h/19h - X-RAM Data (16bit)**

```text
  0-15  DATA  RAM data word (R/W)
```

**FBC1B4h/FBC1B6h - 1Ah/1Bh - Y-RAM Addresss/Control**

**FBC1B8h/FBC1BAh - 1Ch/1Dh - X-RAM Addresss/Control**

```text
  0-8   ADDR  RAM Address
  9     WT    RAM Write (controls read/write direction for RAM Data registers)
  10    CRD   RAM Continuous Read. Enables read of RAM every sample from
              location addressed by ADDR Independent of ACC and WT bits
  11    IOX   X-RAM only: I/O Register Select. Specifies that X RAM ADDRESS
              bit0-7 (Port 1Ch) is an internal I/O register address
  11    N/A   Y-RAM only: Unused
  12-14 N/A   Unused
  15    ACC   RAM Access Enable. Controls DSP access of RAM associated with
              address ADDR bits. WT determines if a read or write is performed
```

**FBC1BCh/FBC1BEh - 1Eh/1Fh - Interrupt Handling**

```text
  0   RDBF    Receiver Data Buffer Full (RBUFFER Full)
  1   N/A     Unused
  2   RDBIE   Receiver Data Buffer Full Interrupt Enable
  3   TDBE    Transmitter Data Buffer Empty (TBUFFER Empty)
  4   N/A     Unused
  5   TDBIE   Transmitter Data Buffer Empty Interrupt
  6   RDBIA   Receiver Data Buffer Full Interrupt Active (IRQ Flag)
  7   TDBIA   Transmitter Data Buffer Empty Interrupt Active (IRQ Flag)
  8   NEWC    New Configuration. Initiates new configuration (cleared by modem
  9   N/A     Unused                   upon completion of configuration change)
  10  NCIE    New Configuration Interrupt Enable
  11  NEWS    New Status. Detection of a change in selected status bits
  12  NSIE    New Status Interrupt Enable
  13  N/A     Unused
  14  NCIA    New Configuration Interrupt Active (IRQ Flag)
  15  NSIA    New Status Interrupt Active (IRQ Flag)
```

**CONF Values**

Below are CONF values taken from RC96DT/RC144DT datasheet (the
RC96V24DP/RC2324DPL datasheet doesn't describe CONF values). Anyways, the
values hopefully same for both chip versions (except that, the higher baudrates
obviously won't work on older chips).

```text
  CONF  Bits/sec Mode Name
  01h   2400     V.27 ter
  02h   4800     V.27 ter
  11h   4800     V.29
  12h   7200     V.29
  14h   9600     V.29
  52h   1200     V.22
  51h   600      V.22
  60h   0-300    Bell 103
  62h   1200     Bell 212A
  70h   -        V.32 bis/V.23 clear down    ;\
  71h   4800     V.32                        ;
  72h   12000    V.32 bis TCM                ; RC96DT/RC144DT only
  74h   9600     V.32 TCM                    ; (not RC96V24DP/RC2324DPL)
  75h   9600     V.32                        ;
  76h   14400    V.32 bis TCM                ;
  78h   7200     V.32 bis TCM                ;/
  80h   -        Transmit Single Tone
  81h   -        Dialing                  ;used by SNES X-Band (dial mode)
  82h   1200     V.22 bis
  83h   -        Transmit Dual Tone
  84h   2400     V.22 bis                 ;used by SNES X-Band (normal mode)
  86h   -        DTMF Receiver
  A0h   0-300    V.21
  A1h   75/1200  V.23 (TX/RX)
  A4h   1200/75  V.23 (TX/RX)
  A8h   300      V.21 channel 2
  B1h   14400    V.17 TCM                    ;\
  B2h   12000    V.17 TCM                    ; RC96DT/RC144DT only
  B4h   9600     V.17 TCM                    ; (not RC96V24DP/RC2324DPL)
  B8h   7200     V.17 TCM                    ;/
```

**XBand X/Y RAM Rockwell**

Below are X/Y RAM addresses that can be accessed via Ports 16h-1Dh.

Addresses 000h-0FFh are "Data RAM", 100h-1FFh are "Coefficient RAM".

X-RAM is "Real RAM", Y-RAM is "Imaginary RAM" (whatever that means).

```text
  XRAM     YRAM     Parameter
  032      -        Turn-on Threshold
  03C      -        Lower Part of Phase Error (this, in X RAM ?)
  -        03C      Upper Part of Phase Error (this, in Y RAM ?)
  -        03D      Rotation Angle for Carrier Recovery
  03F      -        Max AGC Gain Word
  049      049      Rotated Error, Real/Imaginary
  059      059      Rotated Equalizer Output, Real/Imaginary
  05E      05E      Real/Imaginary Part of Error
  06C      -        Tone 1 Angle Increment Per Sample (TXDPHI1)
  06D      -        Tone 2 Angle Increment Per Sample (TXDPHI2)
  06E      -        Tone 1 Amplitude (TXAMP1)
  06F      -        Tone 2 Amplitude (TXAMP2)
  070      -        Transmit Level Output Attenuation
  071      -        Pulse Dial Interdigit Time
  072      -        Pulse Dial Relay Make Time
  073      -        Max Samples Per Ring Frequency Period (RDMAXP)
  074      -        Min Samples Per Ring Frequency Period (RDMINP)
  07C      -        Tone Dial Interdigit Time
  07D      -        Pulse Dial Relay Break Time
  07E      -        DTMF Duration
  110-11E  100-11E  Adaptive Equalizer Coefficients, Real/Imag.
  110      100      First coefficient, Real/Imag. (1) (Data/Fax)
  110      110      Last Coefficient, Real/Imag. (17) (Data)
  11E      11E      Last Coefficient, Real/Imag. (31) (Fax)
  -        121      RLSD Turn-off Time
  12D      -        Phase Error
  12E      -        Average Power
  12F      -        Tone Power (TONEA)
  130      -        Tone Power (TONEB,ATBELL,BEL103)
  131      -        Tone Power (TONEC,ATV25)
  136      -        Tone Detect Threshold for TONEA               (THDA)
  137      -        Tone Detect Threshold for TONEB,ATBELL,BEL103 (THDB)
  138      -        Tone Detect Threshold for TONEC,ATV25         (THDC)
  13E      -        Lower Part of AGC Gain Word
  13F      -        Upper Part of AGC Gain Word
  152      -        Eye Quality Monitor (EQM)
  -        162-166  Biquad 5 Coefficients a0,a1,a2,b1,b2
  -        167-16B  Biquad 6 Coefficients a0,a1,a2,b1,b2
  -        16C-170  Biquad 1 Coefficients a0,a1,a2,b1,b2
  -        171-175  Biquad 2 Coefficients a0,a1,a2,b1,b2
  -        176-17A  Biquad 3 Coefficients a0,a1,a2,b1,b2
  179      -        Turn-off Threshold
  -        17B-17F  Biquad 4 Coefficients a0,a1,a2,b1,b2
```

<a id="snescartxbandrockwellnotes"></a>

## SNES Cart X-Band Rockwell Notes

**Rockwell Configuration Changes**

Various changes (to ASYNC, WDSZ, etc.) seem to be not immediately applied.
Instead, one must apply them by setting NEWC=1 by software (and then wait until
hardware sets NEWC=0).

**Rockwell Dialing**

Dialing is done by setting CONF=81h, and then writing the telephone number
digits (range 00h..09h) to TBUFFER; before each digit wait for TDBE=1 (TX
buffer empty), the BIOS also checks for TONEA=1 before dialing.

The telephone number for the X-Band server is stored as ASCII string in the
BIOS ROM:

```text
  "18002071194"  at D819A0h in US-BIOS (leading "800" = Toll-free?)
  "03-55703001"  at CE0AB2h in Japanese BIOS (leading "3" = Tokyo?)
```

Notes: Before dialing the above 'ASCII' numbers, the US-BIOS first dials
0Ah,07h,00h, and the japanese one first dials 01h. The "-" dash in the japanese
string isn't dialed.

**Rockwell Offline**

There seems to be no explicit offline mode (in CONF register). Instead, one
must probably change the Relay A/B bits (RA/RB) to go online/offline.

**X-Band Fred Chip Pin-Outs**

```text
  1-100 unknown
```

**X-Band Rockwell Pin-Outs**

```text
  Pin Number Signal Name I/O Type
  1 RS2 IA
  2 RS1 IA
  3 RS0 IA
  4 /TEST1
  5 /SLEEP OA
  6 RING
  7 EYEY OB
  8 EYEX OB
  9 EYESYNC OB
  10 RESET ID
  11 XTLI IE
  12 XTLO OB
  13 +5VD
  14 GP18 OA
  15 GP16 OA
  16 XTCLK IA
  17 DGND1
  18 TXD IA
  19 TDCLK OA
  20 TRSTO MI
  21 TSTBO MI
  22 TDACO MI
  23 RADCI MI
  24 RAGCO MI
  25 MODEO MI
  26 RSTBO MI
  27 RRSTO MI
  28 /RDCLK OA
  29 RXD OA
  30 TXA2 O(DD)
  31 TXA1 O(DD)
  32 RXA I(DA)
  33 RFILO MI
  34 AGCIN MI
  35 VC
  36 NC
  37 NC
  38 NC
  39 /RBDVR OD
  40 AGND
  41 /RADRV OD
  42 /SLEEP1 IA
  43 RAGCI MI
  44 NC
  45 RSTBI MI
  46 RRSTI MI
  47 RADCO MI
  48 TDACI MI
  49 TRSTI MI
  50 TSTBI MI
  51 MODE1 MI
  52 +5VA
  53 SPKR O(OF)
  54 DGND2
  55 D7 IA/OB
  56 D6 IA/OB
  57 D5 IA/OB
  58 D4 IA/OB
  59 D3 IA/OB
  60 D2 IA/OB
  61 D1 IA/OB
  62 D0 IA/OB
  63 /IRQ OC
  64 /WRITE IA
  65 /CS IA
  66 /READ IA
  67 RS4 IA
  68 RS3 IA
```

Notes:

(1) MI = Modem Interconnection

(2) NC = No connection (may have internal connection; leave pin disconnected
(open).

(3) I/O types are described in Table 2-3 (digital signals) and Table 2-4
(analog signals).

<a id="snescartxbandbiosfunctions"></a>

## SNES Cart X-Band BIOS Functions

**X-Band BIOS Functions (CALL E00040h)**

Invoked via CALL E00040h, with X=function_number (0001h..054xh on SNES/US),
with parameters pushed on stack, and with return value in A register (16bit) or
X:A register pair (32bit), and with zeroflag matched to the A return value.

The function table isn't initialized by the compiler/linker, instead, the BIOS
boot code is starting the separate components (such like "controls.c"), which
are then installing their function set via calls to "SetDispatchedFunction".

The Sega function numbers are based on the string list in file
"SegaServer\Server\Server_OSNumbers.h" (which is part of the SERVER sources,
but it does hopefully contain up to date info on the retail BIOS functions).

```text
  Sega SNES SNES Function
  Gen. US   JP
```

**Sourceless - Misc**

```text
  000h           RestoreSegaOS
  001h           AskForReplay
  002h           ThankYouScreen            ;thankyou shown at next coldboot?
  003h           InstallDispatchedManager
  004h           CallManagerControl
  005h           SoftInitOS
  006h           GetDispatchedFunction
  007h 007h 007h SetDispatchedFunction     ;change/install BIOS function vector
  008h           SetDispatchedGroup
  009h           GetManagerGlobals
  00Ah           SetManagerGlobals
  00Bh           AllocateGlobalSpace
  00Ch           FreeGlobalSpace
  00Dh           DisposePatch
  00Eh           CompactOSCodeHeap
  00Fh           GetPatchVersion
  010h           SetPatchVersion
```

**Sourceless - Memory**

```text
  011h           InitHeap
  012h           NewMemory
  013h           NewMemoryHigh
  014h           NewMemoryClear
  015h           DisposeMemory
  016h 01Ah 01Ah GetMemorySize          ;get size of an item
  017h           MaxFreeMemory
  018h           TotalFreeMemory
  019h           SwitchPermHeap
  01Ah           SwtichTempHeap  ;uh, Swtich?
  01Bh           CreateTempHeap
  01Ch           CreateHeapFromPtr
  01Dh           CreateTempSubHeap
  01Eh           AllocPermHeapZone
  01Fh           DisposePermHeapZone
  020h           CompactHeap
  021h           MoveHeap
  022h           PrepareHeapForMove
  023h           ComputeHeapPtrDelta
  024h           ResizeHeap
  025h           BlockMove
  026h           WhichMemory
  027h           GetHeapSize
  028h           VerifySegaHeap
  029h           PurgePermHeaps
  02Ah           ByteCopy
  02Bh           UnpackBytes
  02Ch           FillMemory
  02Dh           GetCurrentHeap
  02Eh           FindLastAllocatedBlock
  02Fh           SetOSUnstable
  030h           SetDBUnstable
  031h           SetAddressUnstable
  032h           InstallReliableAddress
  033h           CheckOSReliable
```

**GameLib\controls.c - Keyboard/Joypad Controls**

```text
  034h ?         InitControllers
  035h 033h      ReadHardwareController      ;get joypad data
  036h 034h      ControllerVBL               ;do joypad and keyboard scanning
  037h ?         ReadAllControllers
  038h 036h      FlushHardwareKeyboardBuffer ;flush char_queue
  039h 037h      GetNextHardwareKeyboardChar ;read char_queue
  03Ah 038h      GetHardwareKeyboardFlags
  03Bh 039h      SetHardwareKeyboardFlags
  03Ch 03Ah      GetNextESKeyboardRawcode ;read scancode_queue  ;ES=Eric Smith
  03Dh ?         GetNextESKeyboardStatus
  03Eh 03Ch      GetNextESKeyboardChar    ;read scancode_queue, xlat to char
  03Fh ?         SendCmdToESKeyboard
  -    03Eh 03Fh keyb_io_read_scancodes
  -    03Fh      keyb_blah_do_nothing
  -    040h      keyb_io_read_verify_id_code
  -    041h 043h keyb_forward_scancode_queue_to_char_queue
```

**Sourceless - Misc**

```text
  040h           GetGlobal
  041h           SetGlobal
```

**Database\PatchDB.c - Game/Patch (SNES: installed at D6:4F93 ?)**

```text
  042h 042h 044h AddGamePatch
  043h           LoadGamePatch
  044h           DisposeGamePatch
  045h           GetGamePatchVersion
  046h           GetGamePatchFlags
  047h 04Ah 04Eh FindGamePatch
  048h 054h 058h CreateGameDispatcher
  049h           InitGamePatch
  04Ah           StartGame
  04Bh           GameOver
  04Ch           ResumeGame
  04Dh           GameDoDialog
  04Eh           UpdateGameResultsAfterError
  04Fh           HandleGameError
  050h           PlayCurrentGame
  051h 053h 057h InstallGameFunction
  052h 055h 059h DisposeOldestGamePatch
  053h           MarkGamePatchUsed
```

**Sourceless - Messages**

```text
  054h           InitMessages
  055h           ProcessServerData
  056h           ProcessPeerData
  057h           SendMessage
  058h           GetSendMessageHandler
  059h           GetPeerMessageHandler
  05Ah           GetSerialOpCode
  05Bh           GetServerMessageHandler
  05Ch           InstallPeerHandler
  05Dh           InstallReceiveServerHandler
  05Eh           InstallSendMessageHandler
  05Fh           ReceivePeerMessageDispatch
  060h           ReceiveServerMessageDispatch
  061h           GobbleMessage
  062h           SetClearLoginMisc
  063h           GetLoginMisc
```

**Graphics\Sprites.c**

```text
  064h           CreateSprite
  065h           CreateSpriteInFront
  066h           CreateSpriteHigh
  067h           DisposeSprite
  068h           MoveSprite
  069h           DrawSprite
  06Ah           IncrementSpriteFrame
  06Bh           SetSpriteFrame
  06Ch           GetSpriteFrame
  06Dh           FlipSprite
  06Eh           CreateSpriteData
  06Fh           CreateTextSprite
  070h           CreateTextSpriteFromBitmap
  071h           ExplodeSprite
  072h           SetSpriteGrayFlag
  073h           SetSpriteTilePosition
  074h           SetSpriteImage
  075h           SetSpritePalette
  076h           WriteSpriteToVDP
  077h           FigureTileSize
  078h           AllocateSprite
  079h           FreeSprite
  07Ah           GetSpriteLastTile
  07Bh           GetSpriteFirstTile
  07Ch           NewSpark
  07Dh           DisposeSpark
  07Eh           GetSparkSprite
  07Fh           StartSpark
  080h           StopSpark
  081h           DrawXBandLogo
  082h           DisposeXBandLogoRef
  083h           DisposeXBandLogoSparks
  084h           SyncOTron
```

**Graphics\Decompress.c**

```text
  085h           InitDecompression
  086h           CreateDecompressor
  087h           DisposeDecompressor
  088h           SetDstPattern
  089h           SetImageTiling
  08Ah           SetImageOrigin
  08Bh           GetImageClut
  08Ch           DisposeImagePatterns
  08Dh           DecompressFrame
  08Eh           SetDecompressorOptionsSelector
  08Fh           SetDecompressorPixelMappingSelector
  090h           SetDecompressorPaletteSelector
  091h           GetDictionaryCache
  092h           ReleaseDictionaryCache
  093h           SetDecompressorImage
  094h           ExpandPatternDictionary
  095h           GetDecompressorCache
  096h           ReleaseDecompressorCache
  097h           JoshDecompress
```

**Sourceless - Time...**

```text
  098h           AddTimeRequest
  099h           RemoveTimeRequest
  09Ah           TimeIdle
  09Bh           IncCurrentTime
  09Ch           DelayMS
  09Dh           DelayTicks
  09Eh           SetOSIdle
  09Fh           SegaOSIdle
  0A0h           GetJesusTime
  0A1h           SetJesusTime
  0A2h           GetJesusDate
  0A3h           SetJesusDate
```

**Graphics\animation.c - Animations**

```text
  0A4h           InitAnimateProcs
  0A5h           SpawnAnimation
  0A6h           SpawnDBAnimation
  0A7h           CreateAnimation
  0A8h           DisposeAnimation
  0A9h           DrawAnimationFrame
  0AAh           StartAnimation
  0ABh           StopAnimation
  0ACh           SuspendAnimations
  0ADh           SetAnimationPriority
  0AEh           SetAnimationGrayFlag
  0AFh           GetAnimationSuspendLevel
```

**Graphics\paths.c - Paths (and maybe also LinePath.c?)**

```text
  0B0h           InitPathManager
  0B1h           CreatePath
  0B2h           DisposePath
  0B3h           SetPathPoints
  0B4h           SetPathFrames
  0B5h           SetPathVelocity
  0B6h           GetPathPoint
  0B7h           DistBetweenPoints
```

**Graphics\Pattern.c**

```text
  0B8h           InitPatternManager
  0B9h           NewPatternBlock
  0BAh           NewPatternBlockHigh
  0BBh           FreePatternBlock
  0BCh           DeallocateTopPatternBlock
  0BDh           NewFirstPatternBlock
  0BEh           SetRange
  0BFh           ClearRange
  0C0h           RangeIsFree
  0C1h           FindFreeRange
  0C2h           GetLeftOnesTable
  0C3h           GetRightOnesTable
```

**Graphics\Cursor.c**

```text
  0C4h           CreateSegaCursor
  0C5h           DisposeSegaCursor
  0C6h           MoveSegaCursor
  0C7h           HideSegaCursor
  0C8h           ShowSegaCursor
  0C9h           GetSegaCursorPos
  0CAh           SetSegaCursorImage
  0CBh           LoadCursorFromVRAM
  0CCh           DrawSegaCursor
  0CDh           LoadCursorPattern
```

**Graphics\SegaText.c (1)**

```text
  0CEh           InitSegaFonts
  0CFh           SetCurFont
  0D0h           GetCurFont
  0D1h           GetCurFontHeight
  0D2h           GetCurFontLineHeight
  0D3h           SetFontColors
  0D4h           GetFontColors
  0D5h           SetupTextGDevice
  0D6h           GetTextPatternAddress
  0D7h           GetTextGDeviceOrigin
  0D8h           DrawSegaString
  0D9h           RenderSegaString
  0DAh           MeasureSegaText
  0DBh           CenterSegaText
  0DCh           DrawClippedSegaText
  0DDh           DrawCenteredClippedSegaText
  0DEh           DrawPaddedClippedSegaText
  0DFh           GetCharWidth
  0E0h           SegaNumToString
  0E1h           SegaNumToDate
  0E2h           SegaAppendText
  0E3h           CompareDates
  0E4h           CompareStrings
  0E5h           SetupTextSpriteGDevice
  0E6h           EraseTextGDevice
  0E7h           GetStringLength
```

**Graphics\SegaText.c (2) and Database\StringDB.c**

```text
  0E8h           DrawDBXYString              ;Database\StringDB.c
  0E9h           GetDBXYString               ;Database\StringDB.c
  0EAh           GetSegaString               ;Database\StringDB.c
  0EBh           GetWriteableString          ;Database\StringDB.c
  0ECh           SetWriteableString          ;Database\StringDB.c
  0EDh           DeleteWriteableString       ;Database\StringDB.c
  0EEh           GetUniqueWriteableStringID  ;Database\StringDB.c
  -              AddDBXYString               ;Database\StringDB.c (simulator)
```

**Graphics\SegaText.c (3)**

```text
  0EFh           CopyCString
  0F0h           SetTextPatternStart
  0F1h           EqualCStrings
  0F2h           GetTextStateReference
  0F3h           SaveTextState
  0F4h           RestoreTextState
  0F5h           DisposeTextStateReference
  0F6h           VDPCopyBlitDirect
  0F7h           VDPCopyBlitDirectBGColor
  0F8h           VDPCopyBlitTiled
  0F9h           VDPCopyBlitTiledBGColor
  0FAh           OrBlit2to4
  0FBh           OrBlit1to4
```

**Sourceless - Modem? (parts related to GameLib\CommManager.c?)**

```text
  0FCh           PInit
  0FDh           POpen
  0FEh           PListen
  0FFh           POpenAsync
  100h           PListenAsync
  101h           PClose
  102h           PNetIdle
  103h           PCheckError
  104h           PWritePacketSync
  105h           PWritePacketASync
  106h           PGetError
  107h           PUOpenPort
  108h           PUClosePort
  109h           PUProcessIdle
  10Ah           PUProcessSTIdle
  10Bh           PUReadSerialByte
  10Ch           PUWriteSerialByte
  10Dh           PUTransmitBufferFree
  10Eh           PUReceiveBufferAvail
  10Fh           PUTestForConnection
  110h           PUReadTimeCallback
  111h           PUWriteTimeCallback
  112h           PUSetupServerTalk
  113h           PUTearDownServerTalk
  114h           PUSetError
  115h           PUIsNumberBusy
  116h           PUOriginateAsync
  117h           PUAnstondet
  118h           PUWaitForRLSD
  119h           PUInitCallProgress
  11Ah           PUCallProgress
  11Bh           PUDialNumber
  11Ch           PUWaitDialTone
  11Dh           PUAnswerAsync
  11Eh           PUCheckAnswer
  11Fh           PUCheckRing
  120h           PUResetModem
  121h           PUSetTimerTicks
  122h           PUSetTimerSecs
  123h           PUTimerExpired
  124h           PUHangUp
  125h           PUPickUp
  126h           PUWriteXRAM
  127h           PUWriteYRAM
  128h 13Dh      PUReadXRAM
  129h 13Eh      PUReadYRAM
  12Ah           PUIdleMode
  12Bh           PUDataMode
  12Ch           PUDialMode
  12Dh           PUToneMode
  12Eh           PUCheckLine
  12Fh           PUCheckCarrier
  130h           PUDetectLineNoise
  131h           PUListenToLine
  132h           PUDisableCallWaiting
  133h           PUAsyncReadDispatch
  134h           PUDoSelectorLogin
  135h           PUMatchString
  136h           PGetDebugChatScript
```

**Sourceless - Transport?**

```text
  137h           TInit
  138h           TOpen
  139h           TListen
  13Ah           TOpenAsync
  13Bh           TListenAsync
  13Ch           TClose
  13Dh           TCloseAsync
  13Eh           TUnthread
  13Fh           TNetIdle
  140h           TUCheckTimers
  141h           TReadDataSync
  142h           TReadDataASync
  143h           TWriteDataSync
  144h           TWriteDataASync
  145h           TAsyncWriteFifoData
  146h           TReadData
  147h           TWriteData
  148h           TReadAByte
  149h           TWriteAByte
  14Ah           TQueueAByte
  14Bh           TReadBytesReady
  14Ch           TDataReady
  14Dh           TDataReadySess
  14Eh           TIndication
  14Fh           TForwardReset
  150h           TNetError
  151h           TCheckError
  152h           TUInitSessRec
  153h           TUSendCtl
  154h           TUDoSendCtl
  155h           TUDoSendOpenCtl
  156h           TUUpdateSessionInfo
  157h           TUSendOpen
  158h           TUSendOpenAck
  159h           TUSendCloseAdv
  15Ah           TUSendFwdReset
  15Bh           TUSendFwdResetAck
  15Ch           TUSendFwdResetPacket
  15Dh           TUSendRetransAdv
  15Eh           TUOpenDialogPacket
  15Fh           TUFwdResetPacket
  160h           TUCloseConnPacket
  161h           TURetransAdvPacket
  162h           TUAllowConnection
  163h           TUDenyConnection
  164h           TUSetError
  165h           TUGetError
  166h           TGetUserRef
  167h           TSetUserRef
  168h           TGetTransportHold
  169h           TGetTransportHoldSession
  16Ah           TSetTransportHold
  16Bh           TSetTransportHoldSession
```

**Database\DB.c - Database**

```text
  16Ch           InitPermDatabase
  16Dh           CompactPermDatabase
  16Eh 185h      DBGetItem
  16Fh           DBAddItem
  170h 188h 19Bh DBDeleteItem
  171h 189h 19Ch DBGetUniqueID
  172h           DBGetUniqueIDInRange
  173h           DBGetItemSize
  174h           DBCountItems
  175h 18Dh      DBGetFirstItemID
  176h 18Eh      DBGetNextItemID
  177h           DBNewItemType
  178h           DBGetTypeFlags
  179h           DBSetTypeFlags
  17Ah           DBDeleteItemType
  17Bh           DBPurge
  17Ch           DBTypeChanged
  17Dh           ComputeTypeCheckSum
  17Eh           DBVerifyDatabase
  17Fh           DBROMSwitch
  180h           DBAddItemPtrSize
  181h 199h 1ACh DBAddItemHighPtrSize
  182h 19Ah 1ADh DBPreflight    ;check if enough free mem for new item
  183h           GetItemSize
  184h           DBGetTypeNode
  185h           DBGetPrevTypeNode
  186h           DBTNGetItem
  187h           DBTNGetPrevItem
  188h           DBTNDisposeList
  189h           DeleteItem
  18Ah           AddItemToDB
  18Bh           AllowDBItemPurge
```

**Graphics\SegaScrn.c - Video/Screen**

```text
  18Ch           LinearizeScreenArea
  18Dh           GetSegaScreenBaseAddr
  18Eh           InitSegaGDevices
  18Fh           SetCurrentDevice
  190h           GetCurrentDevice
  191h           RequestClut
  192h           ReleaseClut
  193h           IncrementClutReferences
  194h           SetupClutDB
  195h           GetSegaScreenOrigin
  196h           GetSegaGDevice
  197h           EraseGDevice
  198h           SetupVDP
  199h           BlankClut
  19Ah           FadeInClut
  19Bh           FadeInScreen
  19Ch           GenerateGrayMap
  19Dh           WaitVBlank
  19Eh           SetBackgroundColor
  19Fh           GetBackgroundColor
  1A0h           RequestUniqueClut
  1A1h           RequestSpecificClut
  1A2h           SetupClut
  1A3h           GetClut
  1A4h           GetColorLuminance
  1A5h           FillNameTable
```

**Sourceless - VRAM...**

```text
  1A6h           DMAToVRAM
  1A7h           CopyToVRAM
  1A8h           CopyToCRAM
  1A9h           CopyToVSRAM
  1AAh           CopyToVMap
  1ABh           FillVRAM
  1ACh           FillCRAM
  1ADh           FillVSRAM
```

**Database\Opponent.c - Opponent**

```text
  1AEh           GetOpponentPhoneNumber
  1AFh           SetOpponentPhoneNumber
  1B0h           GetCurOpponentIdentification
  1B1h           SetCurOpponentIdentification
  1B2h           GetCurOpponentTaunt
  1B3h           GetCurOpponentInfo
  1B4h           ClearOldOpponent
  1B5h           GetOpponentVerificationTag
  1B6h           SetOpponentVerificationTag
```

**Database\UsrConfg.c - User/Password**

```text
  1B7h           GetCurrentLocalUser
  1B8h           FillInUserIdentification
  1B9h           GetLocalUserTaunt
  1BAh           SetLocalUserTaunt
  1BBh           GetLocalUserInfo
  1BCh           SetLocalUserInfo
  1BDh           IsUserValidated
  1BEh           SetCurUserID
  1BFh           GetCurUserID
  1C0h           VerifyPlayerPassword
  1C1h           IsEmptyPassword
  1C2h           ComparePassword
  1C3h           GetPlayerPassword
```

**UserInterface\DitlMgr.c - DITL (also related to Database\DITLItemSetup.c?)**

```text
  1C4h           NewDITL
  1C5h           GiveDITLTime
  1C6h           DisposeDITL
  1C7h           GetDITLItem
  1C8h           InitDITLMgr
  1C9h           ClearDITLDone
  1CAh           ProcessDITLScreen
  1CBh           SetupDITLItemList
  1CCh           SetupDITLObjectData
  1CDh           DisposeDITLItemList
  1CEh           SetupControlTable
  1CFh           DisposeControlTable
  1D0h           GetDITLObjectData
```

**UserInterface\Events.c**

```text
  1D1h           InitUserEvents
  1D2h           FlushUserEvents
  1D3h           WaitForUserButtonPress
  1D4h           CheckUserButtonPress
  1D5h           GetNextControllerEvent
  1D6h           GetNextCommand
  1D7h           QueueGet
  1D8h           QueueInsert
```

**Sourceless - Sound**

```text
  1D9h           SetBGMDisable
  1DAh           GetBGMDisable
  1DBh           InitSoundMgr
  1DCh           ShutDownSoundMgr
  1DDh           StartDBBGM
  1DEh           StopBGM
  1DFh           PlayDBFX
  1E0h           FX1NoteOff
  1E1h           FX2NoteOff
  1E2h           ShutUpFXVoice1
  1E3h           ShutUpFXVoice2
```

**Sourceless - Misc**

```text
  1E4h           GetDataSync
  1E5h           GetDataBytesReady
  1E6h           GetDataError
```

**Database\Challnge.c - Challenge**

```text
  1E7h           GetChallengePhoneNumber
  1E8h           SetChallengePhoneNumber
  1E9h           GetChallengeIdentification
  1EAh           SetChallengeIdentification
```

**Database\GameID.c - Game ID**

```text
  1EBh 210h 224h GetGameID     ;out:A=SnesCartStandardChksum, X=SnesHeaderCCITT
  -    211h 225h   ... related to GameID ?
```

**Sourceless - Misc**

```text
  1ECh           IsRemoteModemTryingToConnect
  1EDh           SetRemoteModemTryingToConnectState
  1EEh           InitScreen
  1EFh           PreflightScreen
  1F0h           SetupScreen
  1F1h           SendCommandToScreen
  1F2h           KillScreen
  1F3h           GetNewScreenIdentifier
  1F4h           GetCurScreenIdentifier
  1F5h           GetScreenStateTable
  1F6h           ResetCurrentScreen
  1F7h           GetScreenLayoutRectangleCount
  1F8h           GetScreenLayoutRect
  1F9h           GetScreenLayoutCharRect
  1FAh           GetScreenLayoutPointCount
  1FBh           GetScreenLayoutPoint
  1FCh           GetScreenLayoutStringCount
  1FDh           GetScreenLayoutString
  1FEh           DrawScreenLayoutString
  1FFh           BoxScreenLayoutString
  200h           GetScreensEnteredCount
```

**Graphics\Backdrops.c**

```text
  201h           SetBackdropID
  202h           SetBackdropBitmap
  203h           ClearBackdrop
  204h           HideBackdrop
  205h           SetAuxBackgroundGraphic
  206h           ShowBackdrop
  207h           GetBlinkySprite
```

**Database\BoxSer.c (1)**

```text
  208h           GetBoxSerialNumber
  209h           SetBoxSerialNumber
  20Ah           GetHiddenBoxSerialNumbers
  20Bh           GetBoxHometown
  20Ch           SetBoxHometown
  20Dh           SetBoxState
  20Eh           ResetBoxState
  20Fh           GetBoxState
  210h           SetLastBoxState
  211h           ResetLastBoxState
  212h           GetLastBoxState
  213h           GetGameWinsLosses
  214h           SetCompetitionResults
  215h           GetCompetitionResults
  216h           SetGameErrorResults
  217h           GetGameErrorResults
  218h           UpdateGameResults
  219h           ClearGameResults
  21Ah           ClearNetErrors
  21Bh           GetLocalGameValue
  21Ch           SetLocalGameValue
  21Dh           GetOppGameValue
  21Eh           SetOppGameValue
  21Fh           IsBoxMaster
  220h           SetBoxMaster
  221h 24Bh 25Fh SetCurGameID            ;SNES/US: [3631,3633]
  222h 24Ch      GetCurGameID
  223h           CheckBoxIDGlobals
  224h           InitBoxIDGlobals
  225h           ChangedBoxIDGlobals
  226h           DBAddConstant
  227h           DBGetConstant
  228h           DBSetConstants
  229h           SetDialNetworkAgainFlag
  22Ah           CheckDialNetworkAgainFlag
  22Bh           SetBoxXBandCard
  22Ch           GetBoxXBandCard
  22Dh           GetBoxLastCard
  22Eh           SetBoxMagicToken
  22Fh           SetBoxProblemToken
  230h           GetBoxProblemToken
  231h           UseBoxProblemToken
  232h           SetBoxValidationToken
  233h           GetBoxValidationToken
  234h           SetIMovedOption
  235h           SetQwertyKeyboardOption
  236h           SetCallWaitingOption
  237h           SetAcceptChallengesOption
  238h           GetAcceptChallengesOption
  239h           GetIMovedOption
  23Ah           GetQwertyKeyboardOption
  23Bh           GetCallWaitingOption
  23Ch           GetNetErrors
```

**Database\BoxSer.c (2), and also Database\PhoneNumbers.c ?**

```text
  23Dh           GetBoxPhoneNumber
  23Eh           SetBoxPhoneNumber
  23Fh           GetLocalAccessPhoneNumber
  240h           SetLocalAccessPhoneNumber
  241h           Get800PhoneNumber
```

**Database\BoxSer.c (3)**

```text
  242h           GetLocalUserName
  243h           SetLocalUserName
  244h           GetLocalUserROMIconID
  245h           SetLocalUserROMIconID
  246h           GetLocalUserCustomROMClutID
  247h           SetLocalUserCustomROMClutID
  248h           GetLocalUserPassword
  249h           SetLocalUserPassword
  24Ah           ValidateUserPersonification
  24Bh           InvalidateUserPersonification
```

**Database\PlayerDB.c**

```text
  24Ch           GetAddressBookTypeForCurrentUser
  24Dh           GetAddressBookIDFromIndex
  24Eh           CountAddressBookEntries
  24Fh           RemoveAddressBookEntry
  250h           GetIndexAddressBookEntry
  251h           AddAddressBookEntry
  252h           GetUserAddressBookIndex
  253h           DeleteAddressBookEntry
  254h           SendNewAddressesToServer
  255h           MarkAddressBookUnchanged
  256h           AddressBookHasChanged
  257h           CorrelateAddressBookEntry
  -              PreflightNewAddressEntry
```

**UserInterface\NewAddressMgr.c**

```text
  258h           AddPlayerToAddressBook
  259h           UpdateAddressBookStuff
  25Ah           AddOnDeckAddressBookEntry
  25Bh           MinimizeUserHandle
```

**Database\GraphicsDB.c**

```text
  25Ch           GetDBGraphics
  25Dh           DrawDBGraphic
  25Eh           DrawDBGraphicAt
  25Fh           DrawGraphic
  260h           DisposeGraphicReference
  261h           GetGraphicReferenceClut
  262h           DrawPlayerIcon
  263h           NukePlayerRAMIcon
  264h           GetPlayerRAMIconBitMap
  265h           GetPlayerIconBitMap
  266h           GetIconBitMap
  267h           PlayerRAMIconExists
  268h           DisposeIconReference
  269h           GetDBButtonFrame
  26Ah           DrawGraphicGray
  26Bh           HueShift
```

**Graphics\TextUtls.c - Text Edit**

```text
  26Ch           FindLineBreak
  26Dh           SegaBoxText
  26Eh           DrawSegaStringLength
  26Fh           MeasureSegaTextLength
  270h           InitTextEdit
  271h           SetTextEditLineHeight
  272h           TextEditAppend
  273h           TextEditDelete
  274h           DisposeTextEdit
  275h           TextEditActivate
  276h           TextEditDeactivate
  277h           TextEditPreflightAppend
  278h           TextEditGetLineLength
  279h           DrawTextBox
  27Ah           SetJizzleBehavior
  27Bh           GetJizzleBehavior
  27Ch           StartTextBoxAnimation
  27Dh           StopTextBoxAnimation
  27Eh           DisposeTextBoxReference
  27Fh           DrawSegaTextPlusSpaces
  280h           UpdateTECaret
  281h           EraseTextEditLine
  282h           GetCompressedJizzlers
```

**Database\News.c (and NewsUtils.c) - News**

```text
  283h           FindNextNewsString
  284h           AddPageToNewsBox
  285h           GetPageFromNewsBox
  286h           GetNewsForm
  287h           GetNumNewsPages
  288h           EmptyNewsBox
  289h           DrawNewsPage
  28Ah           ValidateNews
  28Bh           InvalidateNews
  28Ch           SetupNewsForServerConnect
  28Dh           ServerConnectNewsDone
  28Eh           DoNewsControlIdle
  28Fh           KillCurNewsPage
  290h           GetNewsGraphicsID
  291h           ShowLeftRightPageControls
  292h           DrawNewsReturnIcon
  293h           SetNewsCountdownTimeConst
  294h           DrawXBandNews
  295h           DisposeXBandNews
```

**Database\GameDB.c - Network Game Database (NGP)**

```text
  296h           GetNGPListGamePatchInfo
  297h           GetNGPListGamePatchVersion
  298h           GetNGPVersion
  299h           UpdateNGPList
  29Ah           UpdateNameList
  29Bh           GetGameName
```

**Database\Personification.c**

```text
  29Ch           ChangeUserPersonificationPart
  29Dh           InstallOpponentPersonification
  29Eh           GetPersonificationPart
  29Fh           PutPersonificationOnWire
  2A0h           GetPersonificationFromWire
  2A1h           DisposePersonificationSetup
  2A2h           ReceivePersonficationBundle
  2A3h           ParsePersonificationBundle
  2A4h           CreatePersonificationBundle
```

**Database\Mail.c - MailCntl**

```text
  2A5h           CountInBoxEntries
  2A6h           CountOutBoxEntries
  2A7h           AddMailToOutBox
  2A8h           AddMailToInBox
  2A9h           RemoveMailFromInBox
  2AAh           GetIndexInBoxMail
  2ABh           GetIndexOutBoxMail
  2ACh           GetInBoxGraphicID
  2ADh           MarkMailItemRead
  2AEh           DeleteAllOutBoxMail
  2AFh           GetInBoxTypeForCurrentUser
  2B0h           GetOutBoxTypeForCurrentUser
  2B1h           GetOutBoxIDFromIndex
  2B2h           GetInBoxIDFromIndex
  2B3h           GetBoxIDFromIndex
```

**Database\SendQ.c - Send Queue or so?**

```text
  2B4h           AddItemToSendQ
  2B5h           AddItemSizeToSendQ
  2B6h           DeleteSendQ
  2B7h           KillSendQItem
  2B8h           GetFirstSendQElementID
  2B9h           GetNextSendQElementID
  2BAh           CountSendQElements
  2BBh           GetSendQElement
  2BCh           RemoveItemFromSendQ
```

**UserInterface\DialogMgr.c**

```text
  2BDh           SetDialogColors
  2BEh           DoDialog
  2BFh           DialogParameterText
  2C0h           DoDialogItem
  2C1h           DoDialogParam
  2C2h           DoPlayAgainDialog
  2C3h           CopyString
  2C4h           DoAnyResponse
  2C5h           DoDataDrivenDismissal
  2C6h           DoPassword
  2C7h           DrawDialogFrame
  2C8h           FillTextRectangle
  2C9h           HorizontalLine
  2CAh           KillProgressTimer
  2CBh           ReplaceParameters
  2CCh           SetupProgressTimer
  2CDh           VerticalLine
  2CEh           CreateShiners
  2CFh           DisposeShiners
```

**Sourceless - Fred Chip Hardware**

```text
  2D0h           SetVector
  2D1h           SetVectorTblAddr
  2D2h           SetSafeRamSrc
  2D3h           SetSafeRomSrc
  2D4h 323h 33Dh SetLEDs
  2D5h           SetLEDScreenAnimation
```

**Sourceless - Joggler**

```text
  2D6h           InitJoggler
  2D7h           DisplayJoggler
  2D8h           StopJoggler
```

**Database\DeferredDialogMgr.c**

```text
  2D9h           QDefDialog
  2DAh           ShowDefDialogs
  2DBh           CountDefDialogs
  2DCh           DisableDefDialogs
  2DDh           EnableDefDialogs
```

**Sourceless - Misc**

```text
  2DEh           CheckNetRegister
  2DFh           NetRegister
  2E0h           NetRegisterDone
  2E1h           SetNetTimeoutValue
  2E2h           GetNetTimeoutValue
  2E3h           GetNetWaitSoFar
  2E4h           NetRegisterTimeOutTimeProc
  2E5h           IsBoxNetRegistered
  2E6h           GetNetRegisterCase
```

**Database\Capture.c - Session Capture (not actually implemented?)**

```text
  -              BeginSession
  -              DeleteSession
  -              EndSession
  -              BeginStreamCapture
  -              AddDataToStream
```

**Database\Playback.c - Session Playback (not actually implemented?)**

```text
  -              BeginSessionPlayback
  -              SessionExists
  -              PlaybackNextStream
  -              PlaybackCurrentStream
  -              PlaybackPreviousStream
  -              DoesNextSessionStreamExist
  -              DoesPreviousSessionStreamExist
```

**GameLib\Synch.c - Synch (not actually implemented?)**

```text
  -              SynchModems
  -              SynchVbls
```

**Sourceless - Game Talk Session?**

```text
  2E7h           GTSInit
  2E8h           GTSShutdown
  2E9h           GTSFlushInput
  2EAh           GTSessionPrefillFifo
  2EBh           GTSessionEstablishSynch
  2ECh           GTSessionExchangeCommands
  2EDh           GTSessionValidateControl
  2EEh           GTSErrorRecover
  2EFh           GTSCloseSessionSynch
  2F0h           GTSDoCommand
  2F1h           GTSDoResend
  2F2h           GTSResendFromFrame
  2F3h           GTSSetPacketFormat
  2F4h           GTSSetRamRomOffset
  2F5h           GTSessionSetLatency
  2F6h           GTSessionSendController8
  2F7h           GTSessionReadController8
  2F8h           GTSessionSendController12
  2F9h           GTSessionReadController12
  2FAh           GTSessionSendController16
  2FBh           GTSessionReadController16
  2FCh           GTSessionSendController18
  2FDh           GTSessionReadController18
  2FEh           GTSessionSendController24
  2FFh           GTSessionReadController24
  300h           GTSessionSendController27
  301h           GTSessionReadController27
```

**Sourceless - Game Talk Modem?**

```text
  302h           GTModemInit
  303h           GTModemGetModemError
  304h           GTModemClearFifo
  305h           GTModemClockInByte
  306h           GTModemClockOutByte
  307h           GTModemAbleToSend
  308h           GTModemSendBytes
  309h           GTModemCheckLine
  30Ah           GTModemReadModem
  30Bh           GTSendReceiveBytes
  30Ch           GTCloseSessionSafe
  30Dh           GTCreateLooseSession
  30Eh           GTLooseSessionIdle
  30Fh           GTCloseLooseSession
  310h           GTSyncotron
  311h           GTMasterCalculateLatency
  312h           GTSlaveCalculateLatency
  313h           GTSyncoReadModemVBL
  314h           GTSyncronizeVBLs
  315h           GTSyncronizeMasterLeave
  316h           GTSyncronizeSlaveLeave
  317h           GTSyncoTronVBLHandler
  318h           GTUnused1
  319h           GTUnused2
  31Ah           GTUnused3
  31Bh           GTUnused4
  31Ch           GTUnused5
  31Dh           GTUnused6
```

**UserInterface\Keyboard.c - Keyboard**

```text
  31Eh           SetupKeyboardEntryLayout
  31Fh           DisposeKeyboardEntryLayout
  320h           DoKeyboardEntry
  321h           InitKeyboardEntry
  322h           SendCommandToKeyboard
  323h           FinishKeyboardEntry
  324h           RefreshKeyboard
  325h           StuffCurrentKeyboardField
  326h           SelectKeyboardField
  327h           SendCommandToChatKeyboard
  328h           GetKeyLayoutFieldCount
  329h           GetKeyLayoutFieldSize
  32Ah           SetKeyboardEntryMeasureProc
  32Bh           SetFocusField
  32Ch           DrawKeyboard
  32Dh           ComputeCursorLineNumber
  32Eh           CacheKeyboardGraphics
  32Fh           ReleaseKeyboardGraphicsCache
```

**Sourceless - Smart Card**

```text
  330h           GetCardType
  331h           CardInstalled
  332h 381h      ReadCardBytes       ;read smart card byte(s)
  333h           WriteCardBit
  334h           GotoCardAddress
  335h           IncrementCardAddress
  336h 385h      ReadCardBit         ;read smart card bit
  337h           ResetCard
  338h           PresentSecretCode
  339h           GetRemainingCredits
  33Ah           FindFirstOne
  33Bh           CountCardBits
  33Ch           DebitCardForConnect
  33Dh           DebitSmartCard
  33Eh           CheckValidDebitCard
  33Fh           IsGPM896
  340h           IsGPM103
  341h           IsGPM256
  342h           Debit896Card
  343h           Debit103Card
  344h           Get896Credits
  345h           Get103Credits
  346h           CheckWipeCard
  347h           UserWantsToDebitCard
```

**Sourceless - Sort**

```text
  348h           QSort
```

**UserInterface\Secrets.c**

```text
  349h           TrySecretCommand
  34Ah           TestThisSequence
  34Bh           ExecCommands
  34Ch           GetSecretList
  34Dh           GetSecretSequence
  34Eh           ResetSecretCommand
  34Fh           TestSequence
  350h           PlayMaze
  351h           EndPlayMaze
```

**Sourceless - Maths**

```text
  352h           LongDivide
  353h           LongMultiply
  354h           Sqrt
  355h           RandomShort
  356h           Sine
  357h           Cosine
```

**Database\RankingMgr.c - Ranking**

```text
  358h           GetFirstRanking
  359h           GetNextRanking
  35Ah           GetPrevRanking
  35Bh           GetHiddenStat
  35Ch           NextRankingExists
  35Dh           PrevRankingExists
  35Eh           CountRankings
  35Fh           GetFirstRankingID
  360h           GetNextRankingID
  361h           GetUniqueRankingID
  362h           GetRankingSize
  363h           DeleteRanking
  364h           AddRanking
  365h           GetRanking
```

**Graphics\Progress.c - Progress Bar Manager**

```text
  366h           InitProgressProcs
  367h           SpawnProgressProc
  368h           DisposeProgressProc
  369h           SetProgressPosition
  36Ah           ProgressIdle
```

**UserInterface\RadioButtons.c**

```text
  36Bh           SetupRadioButton
  36Ch           DrawRadioButton
  36Dh           ActivateRadioButton
  36Eh           DeactivateRadioButton
  36Fh           RadioButtonSelectNext
  370h           RadioButtonSelectPrevious
  371h           RadioButtonGetSelection
  372h           RadioButtonSetSelection
  373h           RadioButtonIdle
  374h           DisposeRadioButtonRef
  375h           DrawRadioSelection
```

**Sourceless - Misc**

```text
  376h           NetIdleFunc
  377h           CheckError
  378h 3D9h 3F8h ccitt_updcrc
```

**UserInterface\PeerConnect.c**

```text
  379h           DoPeerConnection
  37Ah           ConnectToPeer
  37Bh           DisplayPeerInfo
  37Ch           DoSlavePeerConnect
  37Dh           DoMasterPeerConnect
  37Eh           PeerConnectionDropped
  37Fh           DoPeerRestoreOS
  380h           DoExchangePeerData
  381h           DoPeerDialog
  382h           Chat
  383h           PeerStartVBL
  384h           PeerStopVBL
  385h           PeerVBLHandler
```

**Sourceless - Fifo**

```text
  386h           FifoInit
  387h           FifoActive
  388h           FifoWrite
  389h           FifoRead
  38Ah           FifoPeek
  38Bh           FifoPeekEnd
  38Ch           FifoAvailable
  38Dh           FifoRemaining
  38Eh           FifoSkip
  38Fh           FifoCopy
  390h           FifoChkSum
  391h           GetFifoIn
  392h           FifoLastCharIn
  393h           FifoUnwrite
  394h           FifoSize
  395h           FifoFlush
  396h           FifoUnread
  397h           FifoResetConsumption
  398h           FifoAdjustConsumption
```

**Database\Results.c - Result FIFO (not implemented?)**

```text
  -              AddToResultFIFO
  -              ReplaceTopEntryOfResultFIFO
  -              GetTopEntryOfResultFIFO
  -              GetIndexEntryInResultFIFO
  -              CountEntriesInResultFIFO
```

**Database\FourWayMailView.c**

```text
  -              FourWayMail stuff
```

**Sourceless - Misc**

```text
  399h           AddVBLRequest
  39Ah           RemoveVBLRequest
  39Bh           VBLIdle
  39Ch           PatchRangeStart                              <--- ??
  39Dh           PatchRangeEnd = kPatchRangeStart + 50        <--- ???
  -    54xh      SNES table end
```

**X-Band GAME Functions (CALL E000CCh)**

The GAME functions are just aliases for the normal BIOS functions. The idea
seems to have been that the BIOS function numbering might change in later BIOS
revisions, which would cause compatibility issues for older game patches. As a
workaround, there's a separate GAME function table which contains copies of
some important BIOS function vectors (and which is probably intendend to
maintain fixed function numbers even in later BIOS revisions).

The GAME functions are invoked via CALL E000CCh, with X=function_number
(0000h..004Dh on SNES/US).

The Game Function numbers for Sega are enumerated (among others) in
"Database\GamePatch.h". The Game Function table is initialized by
"CreateGameDispatcher" (which is using a lot of "InstallGameFunction" calls to
transfer the separate function vectors from BIOS table to GAME table).

```text
  Sega SNES SNES Function
  Gen. US   JP
```

**general game stuff**

```text
  00h  00h?      kOSHandleGameError
  01h  01h?      kOSGameOver
```

**basic os stuff**

```text
  02h            kOSNewMemory
  03h            kOSDisposeMemory
  04h            kOSDelayTicks
```

**hardware stuff**

```text
  05h            kOSSetSafeRomSrc
  06h            kOSSetSafeRamSrc
  07h            kOSSetVectorTableAddr
  08h            kOSSetVector
  09h  12h       kOSSetLEDs
```

**PModem**

```text
  0Ah            kOSReadSerialByte
  0Bh            kOSWriteSerialByte
  0Ch            kOSReceiveBufferAvail
  0Dh            kOSTransmitBufferFree
  0Eh            kOSCheckLine
  0Fh            kOSDetectLineNoise
  10h            kOSCheckCarrier
  11h            kOSListenToLine
  12h            kOSSetTimerTicks
  13h            kOSTimerExpired
  14h            kOSToneMode
  15h  20h       kOSReadXRAM
  16h  21h       kOSReadYRAM
  17h            kOSWriteXRAM
  18h            kOSWriteYRAM
```

**gametalk**

```text
  19h            kOSGTSSetPacketFormat
  1Ah            kOSGTSSetRamRomOffset
  1Bh            kOSGTSessionSetLatency
  1Ch            kOSGTSessionPrefillFifo
  1Dh            kOSGTSessionEstablishSynch
  1Eh            kOSGTSErrorRecover
  1Fh            kOSGTSCloseSessionSynch
  10h            kOSGTSFlushInput
  11h            kOSGTSessionValidateControl
  12h            kOSGTSessionExchangeCommands
  13h            kOSGTSDoCommand
  14h            kOSGTSDoResend
  15h            kOSGTSResendFromFrame
  16h            kOSGTModemInit
  17h            kOSGTModemGetModemError
  18h            kOSGTModemClearFifo
  19h            kOSGTModemClockInByte
  1Ah            kOSGTModemClockOutByte
  1Bh            kOSGTModemAbleToSend
  1Ch            kOSGTModemSendBytes
  1Dh            kOSGTModemCheckLine
```

**controller should probably be in "hardware stuff"**

```text
  1Eh            kOSInitControllers
  1Fh            kOSReadControllers
```

**stinkotron**

```text
  20h            kOSGTSyncotron
  21h            kOSGTMasterCalculateLatency
  22h            kOSGTSlaveCalculateLatency
  23h            kOSGTSyncoReadModemVBL
  24h            kOSGTSyncronizeVBLs
  25h            kOSGTSyncronizeMasterLeave
  26h            kOSGTSyncronizeSlaveLeave
  27h            kOSGTSyncoTronVBLHandler
```

**keep this one**

```text
  28h  4Eh       kOSLastFunction
```
