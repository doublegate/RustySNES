# Fullsnes — Hotel Boxes & Arcade Machines (NSS, SFC-Box, Z80, HD64180) & Decompression Formats

[Index](00-index.md) · [« FLASH Backup, Cheat Devices, Tri-Star, Pirate Multicarts, Copiers & CD-ROM Drive](63-copiers-cheat-devices-cdrom.md) · [Unpredictable Things, Timings, Pinouts, Chipset & Mods »](80-timings-unpredictable-pinouts.md)

**Sections in this file:**

- [SNES Hotel Boxes and Arcade Machines](#snes-hotel-boxes-and-arcade-machines)
- [NSS Memory and I/O Maps](#nss-memory-and-io-maps)
- [NSS I/O Ports - Control Registers](#nss-io-ports-control-registers)
- [NSS I/O Ports - Button Inputs and Coin Control](#nss-io-ports-button-inputs-and-coin-control)
- [NSS I/O Ports - RTC and OSD](#nss-io-ports-rtc-and-osd)
- [NSS I/O Ports - EEPROM and PROM](#nss-io-ports-eeprom-and-prom)
- [NSS BIOS and INST ROM Maps](#nss-bios-and-inst-rom-maps)
- [NSS Interpreter Tokens](#nss-interpreter-tokens)
- [NSS Controls](#nss-controls)
- [NSS Games, BIOSes and ROM-Images](#nss-games-bioses-and-rom-images)
- [NSS Component Lists](#nss-component-lists)
- [NSS On-Screen Controller (OSD)](#nss-on-screen-controller-osd)
- [SFC-Box Overview](#sfc-box-overview)
- [SFC-Box Coprocessor (HD64180) (extended Z80)](#sfc-box-coprocessor-hd64180-extended-z80)
- [SFC-Box Memory & I/O Maps](#sfc-box-memory-io-maps)
- [SFC-Box I/O Ports (Custom Ports)](#sfc-box-io-ports-custom-ports)
- [SFC-Box I/O Ports (HD64180 Ports)](#sfc-box-io-ports-hd64180-ports)
- [SFC-Box OSD Chip (On-Screen Display Controller)](#sfc-box-osd-chip-on-screen-display-controller)
- [SFC-Box GROM Format](#sfc-box-grom-format)
- [SFC-Box Component List (Cartridges)](#sfc-box-component-list-cartridges)
- [SFC-Box Component List (Console)](#sfc-box-component-list-console)
- [RTC S-3520 (Real-Time Clock)](#rtc-s-3520-real-time-clock)
- [Z80 CPU Specifications](#z80-cpu-specifications)
- [Z80 Register Set](#z80-register-set)
- [Z80 Flags](#z80-flags)
- [Z80 Instruction Format](#z80-instruction-format)
- [Z80 Load Commands](#z80-load-commands)
- [Z80 Arithmetic/Logical Commands](#z80-arithmeticlogical-commands)
- [Z80 Rotate/Shift and Singlebit Operations](#z80-rotateshift-and-singlebit-operations)
- [Z80 Jumpcommands & Interrupts](#z80-jumpcommands-interrupts)
- [Z80 I/O Commands](#z80-io-commands)
- [Z80 Interrupts](#z80-interrupts)
- [Z80 Meaningless and Duplicated Opcodes](#z80-meaningless-and-duplicated-opcodes)
- [Z80 Garbage in Flag Register](#z80-garbage-in-flag-register)
- [Z80 Compatibility](#z80-compatibility)
- [Z80 Pin-Outs](#z80-pin-outs)
- [Z80 Local Usage](#z80-local-usage)
- [HD64180](#hd64180)
  - [HD64180 Internal I/O Map](#hd64180-internal-io-map)
  - [HD64180 New Opcodes (Z80 Extension)](#hd64180-new-opcodes-z80-extension)
  - [HD64180 Serial I/O Ports (ASCI and CSI/O)](#hd64180-serial-io-ports-asci-and-csio)
  - [HD64180 Timers (PRT and FRC)](#hd64180-timers-prt-and-frc)
  - [HD64180 Direct Memory Access (DMA)](#hd64180-direct-memory-access-dma)
  - [HD64180 Interrupts](#hd64180-interrupts)
  - [HD64180 Memory Mapping and Control](#hd64180-memory-mapping-and-control)
  - [HD64180 Extensions](#hd64180-extensions)
- [SNES Decompression Formats](#snes-decompression-formats)

---

<a id="sneshotelboxesandarcademachines"></a>

## SNES Hotel Boxes and Arcade Machines

**Nintendo Super System (NSS) (USA) (1991)**

Arcade Cabinet. Contains up to three special cartridges (with one game each).

[NSS Memory and I/O Maps](#nss-memory-and-io-maps)

[NSS I/O Ports - Control Registers](#nss-io-ports-control-registers)

[NSS I/O Ports - Button Inputs and Coin Control](#nss-io-ports-button-inputs-and-coin-control)

[NSS I/O Ports - RTC and OSD](#nss-io-ports-rtc-and-osd)

[NSS I/O Ports - EEPROM and PROM](#nss-io-ports-eeprom-and-prom)

[NSS BIOS and INST ROM Maps](#nss-bios-and-inst-rom-maps)

[NSS Interpreter Tokens](#nss-interpreter-tokens)

[NSS Controls](#nss-controls)

[NSS Games, BIOSes and ROM-Images](#nss-games-bioses-and-rom-images)

[NSS Component Lists](#nss-component-lists)

[NSS On-Screen Controller (OSD)](#nss-on-screen-controller-osd)

[SNES Pinouts NSS Connectors](80-timings-unpredictable-pinouts.md#snes-pinouts-nss-connectors)

[Z80 CPU Specifications](#z80-cpu-specifications)

**Super Famicom Box (Japan/Nintendo/HAL) (1993)**

For use in hotel rooms. Typically contains two special multi-carts (which,
together, contain a menu-program and 5 games).

[SFC-Box Overview](#sfc-box-overview)

[SFC-Box Coprocessor (HD64180) (extended Z80)](#sfc-box-coprocessor-hd64180-extended-z80)

[SFC-Box Memory &amp; I/O Maps](#sfc-box-memory-io-maps)

[SFC-Box I/O Ports (Custom Ports)](#sfc-box-io-ports-custom-ports)

[SFC-Box I/O Ports (HD64180 Ports)](#sfc-box-io-ports-hd64180-ports)

[SFC-Box GROM Format](#sfc-box-grom-format)

[SFC-Box OSD Chip (On-Screen Display Controller)](#sfc-box-osd-chip-on-screen-display-controller)

[SFC-Box Component List (Cartridges)](#sfc-box-component-list-cartridges)

[SFC-Box Component List (Console)](#sfc-box-component-list-console)

[RTC S-3520 (Real-Time Clock)](#rtc-s-3520-real-time-clock)

[Z80 CPU Specifications](#z80-cpu-specifications)

[HD64180](#hd64180)

[HD64180 Internal I/O Map](#hd64180-internal-io-map)

[HD64180 New Opcodes (Z80 Extension)](#hd64180-new-opcodes-z80-extension)

[HD64180 Serial I/O Ports (ASCI and CSI/O)](#hd64180-serial-io-ports-asci-and-csio)

[HD64180 Timers (PRT and FRC)](#hd64180-timers-prt-and-frc)

[HD64180 Direct Memory Access (DMA)](#hd64180-direct-memory-access-dma)

[HD64180 Interrupts](#hd64180-interrupts)

[HD64180 Memory Mapping and Control](#hd64180-memory-mapping-and-control)

[HD64180 Extensions](#hd64180-extensions)

**DS-109S (Third-Party/Japan?) (Osaka Tu...?)**

For hotels or so. Contains 10 regular SNES/SFC cartridges. Game selection is
done via a push button and single-digit 7-segment LED display. Not much more
known about the hardware.

**Nintendo Gateway System / Lodgenet (USA/Third-Party)**

For hotels and airports or so. Reportedly offers 18 SNES games or so (later
versions allow games for other/newer consoles). There seem to be no photos of
the hardware. With the "net" in in "Lodgenet" mind... it might be "server
based" (with only Controller &amp; TV-Set located in the hotel room, and the
actual Console &amp; Games located elsewhere?).

<a id="nssmemoryandiomaps"></a>

## NSS Memory and I/O Maps

**Z80 Memory Map**

```text
  0000h-7FFFh : 32K BIOS
  8000h-9FFFh : 8K RAM (upper 4K with write-protect)
  A000h       : EEPROM Input (R)
  C000h-DFFFh : Upper 8K of 32K Instruction EPROM (in Cartridge) (INST-ROM)
  E000h       : EEPROM Output (W)
  Exxxh       : PROM Input AND Output AND Program Code (RST opcodes) (R/W/EXEC)
```

Note: For some reason, Nintendo has stored the 8K INST-ROM in 32K EPROMs - the
first 24K of that EPROMs are unused (usually 00h-filled or FFh-filled, and
EPROM pins A13 and A14 are wired to VCC, so there is no way to access the
unused 24K area).

**Z80 IN-Ports**

```text
  Port 00h.R - IC46/74LS540 - Joypad Buttons and Vsync Flag
  Port 01h.R - IC38/74LS540 - Front-Panel Buttons & Game Over Flag
  Port 02h.R - IC32/74LS540 - Coin and Service Buttons Inputs
  Port 03h.R - IC31/74HC367 - Real-Time Clock (RTC) Input
  Port 04h.R - Returns FFh (unused)
  Port 05h.R - Returns FFh (unused)
  Port 06h.R - Returns FFh (unused)
  Port 07h.R - Returns FFh (same effect as write-any-value to Port 07h.W)
```

Port 0008h..FFFFh are mirrors of above ports (whereof, mirrors at xx00h..xx03h
are often used).

**Z80 OUT-Ports**

```text
  Port 00h/80h.W         - IC40/74HC161 - NMI Control and RAM-Protect
  Port 01h/81h.W         - IC39/74HC377 - Unknown and Slot Select
  Port 02h/82h/72h/EAh.W - IC45/74HC377 - RTC and OSD
  Port 03h/83h.W         - IC47/74HC377 - Unknown and LED control
  Port 84h.W             - IC25/74HC161 - Coin Counter Outputs
  Port 05h.W             - Unused (bug: written by mistake)
  Port 06h.W             - Unused
  Port 07h.W - IC23/74HC109 - SNES Watchdog: Acknowledge SNES Joypad Read Flag
```

These ports seem to be decoded by A0..A2 only (upper address bits are sometimes
set to this or that value, but seem to have no meaning).

**SNES Memory Map**

Normal SNES memory map, plus some special registers:

```text
  4100h/Read.Bit0-7  - DIP-Switches (contained in some NSS cartridges)
  4016h/Write.Bit0   - Joypad Strobe (probably clears the SNES Watchdog flag?)
                          (OR, maybe that occurs not on 4016h-writes,
                          but rather on 4016h/4017h-reads, OR elsewhere?)
  4016h/Write.Bit2   - Joypad OUT2 indicates Game Over (in Skill Mode games)
  4016h/4017h/4218h..421Bh - Joypad Inputs (can be disabled)
```

<a id="nssioportscontrolregisters"></a>

## NSS I/O Ports - Control Registers

**Port WHERE.W**

Somewhere, following OUTPUT signals should be found:

```text
  SNES Reset Signal (maybe separate CPU/PPU resets, and stop, as on PC10)
  SNES Joypad Disable
  SNES Power Supply Enable (SNES VCC switched via Q1 transistor)
  Maybe support for sending data from Z80 to SNES (eg. to 4016h/4017h/4213h)?
```

**Port 00h/80h.W - NMI Control and RAM-Protect (IC40/74HC161)**

```text
  7-4 Unknown/unused      (should be always 0)
  3     Maybe SNES CPU/PPU reset (usually same as Port 01h.W.Bit1)
  2   RAM at 9000h-9FFFh  (0=Disable/Protect, 1=Enable/Unlock)
  1     Looks like maybe somehow NMI Related ?  ;\or one of these is PC10-style
  0     Looks like NMI Enable                   ;/hardware-watchdog reload?
```

Usually accessed as "Port 80h", sometimes as "Port 00h".

**Port 01h/81h.W - Unknown and Slot Select (IC39/74HC377)**

```text
  7     Maybe SNES Joypad Enable? (0=Disable/Demo, 1=Enable/Game)
  6   Unknown/unused        (should be always 0)
  5   SNES Sound Mute       (0=Normal, 1=Mute) (for optional mute in demo mode)
  4   Player 2 Controls (0=CN4 Connector, 1=Normal/Joypad 2) (INST ROM Flags.0)
  3-2 Slot Select (0..2=1st..3rd Slot, 3=None) (mapping to both SNES and Z80)
  1     Maybe SNES CPU pause?  (cleared on deposit coin to continue) (1=Run)
  0     Maybe SNES CPU/PPU reset?   (0=Reset, 1=Run)
```

Sometimes accessed as "Port 81h", sometimes as "Port 01h".

**Port 03h/83h.W - Unknown and LED control (IC47/74HC377)**

```text
  7     Layer SNES Enable?             (used by token proc, see 7A46h) SNES?
  6     Layer OSD Enable?
  5-4 Unknown/unused (should be always 0)
  3   LED Instructions (0=Off, 1=On)  ;-glows in demo (prompt for INST button)
  2   LED Game 3       (0=Off, 1=On)  ;\
  1   LED Game 2       (0=Off, 1=On)  ; blinked when enough credits inserted
  0   LED Game 1       (0=Off, 1=On)  ;/
```

Usually accessed as "Port 83h", sometimes as "Port 03h".

**Port 05h.W - Unused/Bug**

```text
  7-0 Unknown
```

Accessed only as "Port 05h" (via "outd" opcode executed 5 times; but that seems
to be just a bugged attempt to access Port 04h downto 00h).

**Port 07h.W - SNES Watchdog: Acknowledge SNES Joypad Read Flag (IC23/74HC109)**

```text
  7-0 Unknown/unused (write any dummy value)
```

Accessed only as "Port 07h". Writing any value seems to switch Port 00h.R.Bit7
back to "1". That bit is used for the SNES Watchdog feature; the SNES must read
joypads at least once every some frames (the exact limit can be set in INST
ROM).

If the watchdog expires more than once, then the game is removed from the
cartridge list, and used credits are returned to the user (then allowing to
play other games; as long as there are any other games installed).

Note: Judging from hardware tests, there seem to be other ways to acknowledge
the flag (probably via Port 07h.R, or maybe even via Port 00h.R itself).

**NMI**

The NMI source is unknown. Maybe Vblank/Vsync, maybe from SNES or OSD, or some
other timer signal.

**Game/Demo-Mode Detection**

The original NSS games seem to be unable to detect if a coin is inserted (ie.
if they should enter game or demo mode). However, it's possible to do that kind
of detection:

Joypad Disable does work much like disconnecting the joypad, so one can check
the 17th joypad bit to check if the joypad is connected/enabled (aka if money
is inserted). The Magic Floor game is using that trick to switch between game
and demo mode (this has been tested by DogP and works on real hardware, ie. the
NSS does really disable the whole joypad bitstream, unlike the PC10 which seems
to disable only certain buttons).

<a id="nssioportsbuttoninputsandcoincontrol"></a>

## NSS I/O Ports - Button Inputs and Coin Control

**Port 00h.R - Joypad Buttons (IC46/74LS540)**

```text
  7   SNES Watchdog (0=SNES did read Joypads, 1=Didn't do so) (ack via 07h.W)
  6   Vsync (from OSD or SNES ?)  (0=Vsync, 1=No) (zero for ca. 3 scanlines)
  5   Button "Joypad Button B?"   (0=Released, 1=Pressed)
  4   Button "Joypad Button A"    (0=Released, 1=Pressed)
  3   Button "Joypad Down"        (0=Released, 1=Pressed)
  2   Button "Joypad Up"          (0=Released, 1=Pressed)
  1   Button "Joypad Left"        (0=Released, 1=Pressed)
  0   Button "Joypad Right"       (0=Released, 1=Pressed)
```

**Port 01h.R - Front-Panel Buttons &amp; Game Over Flag (IC38/74LS540)**

```text
  7   From SNES Port 4016h.W.Bit2 (0=Game Over Flag, 1=Normal) (Inverted!)
  6   Button "Restart"            (0=Released, 1=Pressed) ;-also resets SNES?
  5   Button "Page Up"            (0=Released, 1=Pressed)
  4   Button "Page Down"          (0=Released, 1=Pressed)
  3   Button "Instructions"       (0=Released, 1=Pressed)
  2   Button "Game 3"             (0=Released, 1=Pressed) ;\if present (single
  1   Button "Game 2"             (0=Released, 1=Pressed) ; cartridge mode does
  0   Button "Game 1"             (0=Released, 1=Pressed) ;/work without them)
```

**Port 02h.R - Coin and Service Buttons Inputs (IC32/74LS540)**

```text
  7-3 External 5bit input (usually CN5 isn't connected: always 0=High)
  2   Service Button (1=Pressed: Add Credit; with INST button: Config)
  1   Coin Input 2   (1=Coin inserted in coin-slot 2)
  0   Coin Input 1   (1=Coin inserted in coin-slot 1)
```

**Port 84h.W - Coin Counter Outputs (IC25/74HC161)**

```text
  7-4 Unknown/unused (should be always 0) (probably not connected anywhere)
  3-2 Unknown/unused (should be always 0) (probably wired to 74HC161)
  1   Coin Counter 2 (0=No change, 1=Increment external counter)
  0   Coin Counter 1 (0=No change, 1=Increment external counter)
```

Accessed only as "Port 84h". To increase a counter, the bit should be set for
around 4 frames, and cleared for at least 3 frames (before sending a second
pulse).

<a id="nssioportsrtcandosd"></a>

## NSS I/O Ports - RTC and OSD

Real-Time Clock (RTC) and On-Screen Display (OSD) Registers

**Port 03h.R - Real-Time Clock (RTC) Input (IC31/74HC367)**

```text
  7-1 Unknown/unused    (seems to be always 7Eh, ie. all seven bits set)
  0   RTC Data In       (0=Low=Zero, 1=High=One)
```

**Port 02h/82h/72h/EAh.W - RTC and OSD (IC45/74HC377)**

```text
  7   OSD Clock ?       (usually same as Bit6)  ;\Chip Select when Bit6=Bit7 ?
  6   OSD Clock ?       (usually same as Bit7)  ;/
  5   OSD Data Out      (0=Low=Zero, 1=High=One)
  4   OSD Special       (?)  ... or just /CS ? (or software index DC3F/DD3F?)
  3   RTC /CLK          (0=Low=Clock,  1=High=Idle)              ;S-3520
  2   RTC Data Out      (0=Low=Zero,   1=High=One)
  1   RTC Direction     (0=Low=Write,  1=High=Read)
  0   RTC /CS           (0=Low/Select, 1=High/No)
```

RTC is accessed via "Port 82h", OSD via "Port 02h/72h/EAh". For OSD access, the
BIOS toggles a LOT of data (and address) lines; not quite clear which of those
lines are OSD CLK and OSD Chip Select.

**RTC Real-Time Clock (S-3520)**

The NSS-BIOS supports year 1900..2099 (century 00h=19xx, FFh=20xx is stored in
RAM at 8F8Dh/978Dh/9F8Dh; in the two version "03" BIOSes). The current time is
shown when pressing Restart in the Bookkeeping screen.

[RTC S-3520 (Real-Time Clock)](#rtc-s-3520-real-time-clock)

**OSD On-Screen Display (M50458-001SP)**

[NSS On-Screen Controller (OSD)](#nss-on-screen-controller-osd)

<a id="nssioportseepromandprom"></a>

## NSS I/O Ports - EEPROM and PROM

**Memory A000h.R - EEPROM Input**

```text
  7   EEPROM Data In (0=Low=Zero, 1=High=One)
  6   EEPROM Ready   (0=Low=Busy, 1=High=Ready)
  5-0 Unknown/unused
```

**Memory E000h.W - EEPROM Output**

```text
  7   Unknown/set     (should be always 1)
  6-5 Unknown/unused  (should be always 0)
  4   EEPROM Clock    (0=Low=Clock, 1=High=Idle) ;(Data In/Out must be stable
  3   EEPROM Data Out (0=Low=Zero, 1=High=One)   ;on raising CLK edge)
  2-1 Unknown/unused  (should be always 0)       ;(and updated on falling edge)
  0   EEPROM Select   (0=High=No, 1=Low=Select)
```

**Note**

E000h (W) and Exxxh (W) are probably mirrors of each other. If so, some care
should be taken not to conflict PROM and EEPROM accesses.

**Memory Exxxh.R.W.EXEC - Ricoh RP5H01 serial 72bit PROM (Decryption Key)**

Data Write:

```text
  7-5  Unknown/unused
  4    PROM Test Mode (0=Low=6bit Address, 1=High=7bit Address)
  3    PROM Clock     (0=Low, 1=High) ;increment address on 1-to-0 transition
  2-1  Unknown/unused
  0    PROM Address Reset (0=High=Reset Address to zero, 1=Low=No Change)
```

Data Read and Opcode Fetch:

```text
  7-5  Always set (MSBs of RST Opcode)
  4    PROM Counter Out (0=High=One, 1=Low=Zero) ;PROM Address Bit5
  3    PROM Data Out    (0=High=One, 1=Low=Zero)
  2-0  Always set (LSBs of RST Opcode)
```

The BIOS accesses the PROM in two places:

```text
  1st PROM check: Accessed via E37Fh, this part decrypts the 32h-byte area.
    the first data bit is read at a time when PROM reset is still high,
    and reset is then released after reading that data bit. At this point,
    there's a critical glitch: If the data bit was 1=Low, then the decryption
    code chooses to issue a 1-to-0 CLK transition at SAME time as when
    releasing reset - the PROM must ignore this CLK edge (otherwise half
    of the games won't work).
  2nd PROM check: Accessed via EB27h, this part decrypts the double-encrypted
    title (from within the 32h-byte area) and displays on the OSD layer,
    alongsides it does verify a checksum at DC3Fh.
    Note: The program code hides in the OSD write string function, and gets
    executed when passing invalid VRAM addresses to it; this is usually done
    via Token 06h.
    This is initially done shortly after the 1st PROM check (at that point
    just for testing DC3Fh, with "invisible" black-on-black color attributes).
```

And, there are two more (unused/bugged) places:

```text
  3rd PROM check: Accessed via FB37h, this part is similar to 2nd PROM check,
    but sends garbage to OSD screen, and is just meant to verify checksum at
    DD3Fh. However, this part seems to be bugged (passing FB37h to the RST
    handler will hang the BIOS). The stuff would be invoked via Token 4Eh,
    but (fortunately) the BIOS is never doing that.
  4th PROM check: Accessed via ExExh, this part is comparing the 1st eight
    bytes of the PROM with a slightly encrypted copy in INST ROM. However,
    in F-Zero, the required pointer at [2Eh-2Fh] in the 32h-byte area is
    misaligned, thus causing the check to fail. The stuff would be invoked
    from inside of NMI handler (when [80ECh] nonzero), but (fortunately) the
    BIOS is never doing that.
```

Note: All (used) PROM reading functions use RST vectors which are executing Z80
code in INST ROM. Accordingly, the code in INST ROM can be programmed so that
it works with PROM-less cartridges.

**PROM Dumps**

Theoretically, dumping serial PROMs is ways easier than dumping parallel
ROMs/EPROMs - but, as by now, nobody does ever seem to have done this. Anyways,
with a brute-force program, it's possible to find matching PROM values for
decrypting known title strings.

```text
  Title                 PROM content
  ActRaiser             B9,4B,F5,72,E4,9E,25,FF,F2,F2,00,00,F2,F2,00,00
  AMAZING TENNIS        2D,EB,21,3B,9A,81,86,93,57,57,00,00,57,57,00,00
  F-ZERO                49,63,FA,03,B5,DF,F6,17,B7,B7,00,00,B7,B7,00,00
  LETHAL WEAPON         7F,9B,42,99,D4,C2,A9,0A,CB,CB,00,00,CB,CB,00,00
  NCAA Basketball       DB,35,54,07,A0,EF,A2,72,F8,F8,00,00,F8,F8,00,00
  New Game 1 [Contra 3] 3A,BC,E6,47,10,DD,45,AF,FC,FC,00,00,FC,FC,00,00
  ROBOCOP 3             6A,06,DC,99,5F,3A,5C,D1,5D,5D,00,00,5D,5D,00,00
  Super Mario World     AE,D4,A8,1C,EC,DA,8D,EA,7D,7D,00,00,7D,7D,00,00
  SUPER SOCCER          6C,57,7E,3C,8F,1F,AB,F2,3D,3D,00,00,3D,3D,00,00
  Super Tennis          86,B7,8E,BD,74,A3,6E,56,9F,9F,00,00,9F,9F,00,00
  The Addams Family     C1,70,F2,7F,3A,EC,D3,02,67,67,00,00,67,67,00,00
  The Irem Skins Game   D7,3F,FE,6A,B7,3A,18,AA,D6,D6,00,00,D6,D6,00,00
```

**Mitsubishi M6M80011 64x16 Serial EEPROM Protocol**

All values transferred LSB first.

```text
  Write Enable:  Send C5h,xxh
  Write Disable: Send 05h,xxh
  Write Word:    Send 25h,addr, Send lsb,msb
  Read Word:     Send 15h,addr, Read lsb,msb
  Read Status:   Send 95h,mode, Read stat...
    (mode: 0=Busy, 1=WriteEnable, 2=ECC Flag)
    (stat: endless repeated bits, 0=Busy/WriteEnable/ECC_Correct)
    (                             1=Ready/WriteDisable/ECC_Incorrect)
```

M6M80011 Pin-Out (2x4pin version)

```text
  1=/CS, 2=/CLK, 3=DTA.IN, 4=DTA.OUT, 5=GND, 6=RESET, 7=RDY/BUSY, 8=VCC
```

**NSS EEPROM Format (Coinage Settings)**

```text
  00h-3Bh Fifteen 4-byte chunks (unused entry when 1st byte = 00h)
           Byte0: Upper Nibble: Checksum (all other 7 nibbles added together)
           Byte0: Lower Nibble: Price (Number of credits for this game, 1..9)
           Byte1: GameID
           Byte2: Time Minutes (BCD) (time limit per game)
           Byte3: Time Seconds (BCD) (time limit per game)
  3Ch     Right Coinage and Unused (bit7-4=Unused, but must be 1..9)
  3Dh     Left Coinage and Flags (bit7=Music, bit6=Freeplay, bit5-4=Unused)
  3Eh-3Fh Checksum (all bytes at [00h..3Dh] added together)
  40h-7Fh Backup Copy of 00h..3Fh
```

<a id="nssbiosandinstrommaps"></a>

## NSS BIOS and INST ROM Maps

**NSS BIOS ROM (32K mapped to 0000h-7FFFh)**

```text
  0000h   Reset Vector
  0008h   RST Handlers (internally used by PROM checks)
  0066h   NMI Handler (unknown source, probably Vblank or Vsync or so)
  3FFDh   Hardcoded Token Address (used by F-Zero INST ROM)
  5F30h   Hardcoded Return-Address from 2nd PROM check in INST ROM
```

**NSS INST ROM (8K mapped to C000h-DFFFh)**

```text
  [C034h]+00h..31h   Encrypted Data (to be decrypted via PROM data)
  [C034h]+32h..33h   Chksum on above 32h bytes (all BYTEs added together)
  [C67Fh]+C600h      RST 38h for 1st PROM check ;\
  [C67Fh]*100h+7Fh   RST 28h for 1st PROM check ; for decrypting the
  [C77Fh]+C700h      RST 20h for 1st PROM check ; 32h-byte area
  [C77Fh]*100h+7Fh   RST 30h for 1st PROM check ;/
  [D627h]+D600h      RST 38h for 2nd PROM check ;\for decrypting the
  [D627h]*100h+27h   RST 28h for 2nd PROM check ; 21-byte title (and
  [D727h]+D700h      RST 20h for 2nd PROM check ; verifying [DC3Fh])
  [D727h]*100h+27h   RST 30h for 2nd PROM check ;/
  [(where are?)]     RST's   for 3rd PROM check ;-this part looks bugged
  [DC15h+00h..29h]   Spaces,FFh,"-credit play" (with underline attr) (for Menu)
  [DC3Fh]            8bit chksum for 2nd PROM security check
  [DD3Fh]            8bit chksum for 3rd PROM security check
  [DEF1h..DEFFh]     Title (for Bookkeeping) (in 8bit OSD characters)
  [DF00h..DF02h]     Token Entrypoint 1 (Goto token)
  [DF05h..DF07h]     Token Entrypoint 2 (Goto token) (overlaps below DF06h!)
  [[DF06h]+6]        Title Xloc+Odd MSBs (for title-centering via token 66h)
  [NNNNh]            Further locations accessed via pointers in 32h-byte area
  [C032h]            16bit Ptr to inst.chksum.lsb ;\all WORDs at C000..DFFF
  [DFFEh]            16bit Ptr to inst.chksum.msb ;/added together
```

**32h-Byte Area at [C034h]+00h..31h (encrypted via PROM data)**

```text
  00h      Flags
             Bit0 Player 2 Controls (0=CN4 Connector, 1=Normal/Joypad 2)
             Bit1 Unused (should be 0)
             Bit2 Unused (should be 0)
             Bit3 Continue Type (0=Normal/Resume Game, 1=Reset Game)
             Bit4 Continue (1=Prompt "Insert Coin to Continue" in Skill Mode)
             Bit5 Used entry (must be 1) (otherwise treated as empty slot)
             Bit6 Checksum Type ([2Ah,2Bh] and num "0" bits in chk[2Eh-2Fh])
             Bit7 Skill Mode (0=Time-Limit Mode, 1=Skill Mode)
  01h      GameID (must be a unique value; BIOS rejects carts with same IDs)
  02h-16h  Title (21 OSD chars) (needs second PROM decryption pass)
  17h-18h  Attraction/Demo Time (in "NMI" units) ("You Are Now Viewing...")
  19h-1Ah  VRAM Addr for Inserted Credits string (during game play)
  1Bh-1Ch  Ptr to List of Encrypted Instruction Text Lines
           (len byte, followed by len+1 pointers to 24-word text strings)
  1Dh      Default Price (number of credits per game) (LSB must be 01h..09h)
  1Eh      Time Minutes (BCD) ;\(TIME mode: MUST be 01:00 .. 30:00 and LSB
  1Fh      Time Seconds (BCD) ;/MUST be 0 or 5)
           (In SKILL mode: [1Eh]=0Dh, some Continue delay used when Flags.4=1)
  20h-21h  VRAM Addr for Remaining Time value (unused in Skill Mode)
  22h      SNES Watchdog (SNES must read joypads every N frames; 00h=Disable)
  23h         ??? Byte... (jump enable for token 60h) (allow money-back?)
  24h         ???Byte, alternate for [25h]?
  25h         ???Byte, time-limit related; combined with [1Eh..1Fh,26h..27h]?
  26h-27h     ???Word (unused for GameID 00-02; these use 00C0h/0140h)
  28h-29h  Unused (0000h)
  2Ah-2Bh  Checksum adjust (optional XOR value for [30h-31h], when Flags.6=1)
  2Ch-2Dh  Encrypted.ptr to 4th check xfer.order.XOR.byte (eg.byte 07h=reverse)
  2Eh-2Fh  Encrypted.ptr to 4th check 8-byte key (sometimes depends [01h])
  30h-31h  Checksum accross [00h..2Fh], eventually XORed with [2Ah]:[2Bh]
```

Note: After decryption, the above 32h-bytes are stored at 8s00h..8s31h (with
s=0..2 for slot 1-3).

Note: Instructions can be viewed by pressing Instructions Button, either during
game, or in demo mode.

**Skill Mode Notes**

There are some variants (unknown how exactly to select which variant):

```text
  Game RESTARTS after Game Over (if one still has credits)
  Game CONTINUES after Game Over (if one still has credits)
```

And, if one does NOT have credits remaining:

```text
  Game PROMPTS insert coin to CONTINUE (eg. ActRaiser)
  Game ABORTS and goes to Game Menu
```

And, for supporting Skill Mode, the DF00h function must contain a
Poke(8060h,00h) token.

**GameID Notes**

Known values used by original games are 00h..09h, FDh, and FFh. The homebrew
Magic Floor game is using ID 3Fh. The no$sns/a22i tool assigns IDs 40h..BFh
based on the game Title checksum (that assignment does more or less reduce risk
that different homebrew games could conflict with each other).

**Tools**

The a22i assembler (in no$sns debugger, v1.3 and up) allows to create INST ROM
files with title, instructions, checksums, time/skill settings, and special
PROM-less RST handlers. For details see the "magicnss.a22" sample source code
in the "magicsns.zip" package.

<a id="nssinterpretertokens"></a>

## NSS Interpreter Tokens

**Tokens**

```text
  00h  Reboot_Bios()
  02h  Osd_Wrstr_Direct(Len8,VramAddr16,Data16[Len], ... ,FFh,Sleep0)
  04h  Osd_Wrstr_Encrypted_Txt_Line(Yloc*12,Sleep0)
  06h  Osd_Wrstr_Prom_Title_Slot_80C0h(Len8-1,VramAddr+2000h*N,Sleep0)    ?
  08h  Osd_Wrstr_Prom_Title(Slot+80h,Len8-1,VramAddr+2000h*N,Sleep0)      ?
  0Ah  Port_00h_W_Set_Bits(OrValue)
  0Ch  Port_01h_W_Set_Bits(OrValue)
  0Eh  Port_03h_W_Set_Bits(OrValue)
  10h  Port_00h_W_Mask_Bits(AndValue)
  12h  Port_01h_W_Mask_Bits(AndValue)
  14h  Port_03h_W_Mask_Bits(AndValue)
  16h  Set_80C2h_To_Immediate(Imm8)
  18h  Set_80C3h_To_Immediate(Imm8)
  1Ah  Set_80C4h_To_Immediate(Imm8)
  1Ch  Set_80C5h_To_Immediate(Imm8)
  1Eh  Compare_And_Goto_If_Equal(Addr16,Imm8,Target)          ;\
  20h  Compare_And_Goto_If_Not_Equal(Addr16,Imm8,Target)      ; unsigned
  22h  Compare_And_Goto_If_Below_or_Equal(Addr16,Imm8,Target) ; cmp [addr],imm
  24h  Compare_And_Goto_If_Above(Addr16,Imm8,Target)          ;/
  26h  Decrement_And_Goto_If_Nonzero(Addr16,Target)
  28h  Poke_Immediate(Addr16,Imm8)
  2Ah  Sleep_Long(Sleep16)
  2Ch  Disable_Interpreter_and_Reset_Gosub_Stack()
  2Eh  Osd_Display_Num_Credit_Play(Slot*4,VramAddr16,Sleep0)
  30h  Test_And_Goto_If_Nonzero(Addr16,Imm8,Target)
  32h  Test_And_Goto_If_Zero(Addr16,Imm8,Target)
  34h  Osd_Wrstr_Indirect(Addr16,Sleep0)
  36h  Gosub_To_Subroutine(Target)   ;\max 3 nesting levels
  38h  Return_From_Subroutine()      ;/
  3Ah  Goto(Target)
  3Ch    _xxx()        ... init some values
  3Eh    _xxx()     ... init more, based on inst rom
  40h  Wait_Vblank()          ;or so (waits for Port[00h].bit6)
  42h  Osd_Wrstr_Indexed(index8,Sleep0)
  44h  Reload_Attraction_Timer()
  46h    _xxx()    ... advance to next instruction page ... or so
  48h  Handle_PageUpDown_For_Multipage_Instructions()
  4Ah  Reload_SNES_Watchdog()
  4Ch  Decrease_SNES_Watchdog_and_Goto_if_Expired(Target)
  4Eh    _xxx_osd_SPECIAL...(Slot+80h,Len8-1,VramAddr+2000h*N,Sleep0) ? bugged?
  50h    _copy_cart_flag_bit0_to_port_01_w_bit4()    ... joypad2 vs CN4
  52h  Map_Slot_80C0h()
  54h  Osd_Wrstr_Indirect_Encrypted(Addr16,Sleep0)
```

Below exist in BIOS version "03" only:

```text
  56h  Osd_Wrstr_Num_Credit_Play(VramAddr16,Sleep0)
  58h  Map_Slot_804Ch()
  5Ah    _xxx()      ;two lines: SubtractVramAddrBy1Ah_and_Strip_Underline ?
  5Ch  Osd_Wrstr_Prom_Title_Slot_804Ch_unless_Slot1_Empty(Len8,VramAddr,Sleep0)
  5Eh     Copy_8s19h_To_81E9h()      ;=VRAM Addr for Credits String
  60h  Goto_If_8s23h_Nonzero(Target)
  62h    _xxx(Target)       ;load timer from 8s24h or 8s25h goto if zero
  64h  Goto_If_GameID_is_00h_or_01h_or_02h(Target)
  66h  Create_Centered_Osd_Wrstr_Title_Function_at_84C0h(yloc*24)
  68h    _xxx()           ;... 8s25h, 8s26h, and MM:SS time-limit related ?
```

And, some general token values:

```text
  56h..7Eh  Unused_Lockup()   ;unused version "02" tokens ;\jump to an
  6Ah..7Eh  Unused_Lockup()   ;unused version "03" tokens ;/endless loop
  01h..7Fh  Crash()           ;odd token numbers jump to garbage addresses
  80h..FFh  Sleep_Short(Sleep7)  ;00h..7Fh (in LSBs of Token)
```

Sleep0 is an optional 00h-byte that can be appended after the Wrstr(Params)
commands. If the 00h-byte is NOT there, then a Sleep occurs for 1 frame. If the
00h-byte is there, then token execution continues (after skipping the 00h)
without Sleeping.

**Note**

INST ROM contains two interpreter functions (invoked via Gosub DF00h and Gosub
DF05h).

```text
  DF00h - Custom code (quite simple in F-Zero, very bizarre in ActRaiser)
  DF05h - Display centered & underlined Title in first line
```

Available stack depth is unknown (at least one stack level is used, so there
are max two free levels, or maybe less) (the DF00h function CAN use at least
one stack level).

The DF05h function is used for displaying the instructions headline (when
viewing instructions in Demo mode). The purpose/usage of the DF00h function is
unknown; essentially, everything works fine even if it just contains a Return
token; for Skill Mode games it also seems to require a Poke(8060h,00h) token.

<a id="nsscontrols"></a>

## NSS Controls

**Front Panel**

```text
  .---------------------------------------------------------------------------.
  |         _________________   _________________   _________________         |
  |        |                 | |                 | |                 |        |
  |        |  game 1 logo    | |  game 2 logo    | |  game 3 logo    |        |
  |        |                 | |                 | |                 |        |
  |        |                 | |                 | |                 |        |
  |        |_________________| |_________________| |_________________|        |
  |                ( )                 ( )                 ( )                |
  |               GAME 1              GAME 2              GAME 3              |
  |---------------------------------------------------------------------------|
  |                                     PAGE PAGE         RESTART             |
  |                       INSTRUCTIONS  UP   DOWN           GAME              |
  |  __                         __ ( ) ( ) ( ) __           ( )           __  |
  | /  '''---...__   __...---'''  \           /  '''---...__   __...---'''  \ |
  ||    _         '''              |         |    _         '''              ||
  ||  _| |_                ( )     |         |  _| |_                ( )     ||
  |' |_   _|            ( )   ( )  '         ' |_   _|            ( )   ( )  '|
  | |  |_|     ( ) ( )     ( )    |           |  |_|     ( ) ( )     ( )    | |
  |  '...........................'             '...........................'  |
   \                                                                         /
    '-----------------------------------------------------------------------'
```

Unlike normal cabinets, the NSS doesn't have arcade joysticks. Instead, there
are two huge SNES joypads firmly mounted on the front-plate (about twice as big
as normal SNES joypads). L/R symbols are depicted on the joypad "surface"
(although the actual L/R buttons seem to be on "rear" shoulders as usually).

GAME 1..3 and INSTRUCTION buttons are fitted with LEDs

For single-cartridge use, there may be a different front-panel without GAME 1-3
buttons (there is no Game Menu, and the Config screen is joypad controlled in
single-cart mode).

Plus, TEST Button, SERVICE Button

Plus, TWO Coin input/switches

Plus, DIP-Switches in Cartridge

<a id="nssgamesbiosesandromimages"></a>

## NSS Games, BIOSes and ROM-Images

**Nintendo Super System BIOS (Nintendo)**

The BIOS is stored in a 32Kx8 EPROM on the mainboard. There are at least three
BIOS versions (the version number, "02" for oldest version, and "03" for the
two newer versions, is shown at the top of the Selftest result screen). The
"02" version is incompatible with newer games (works only with the 3 oldest
titles).

```text
  NSS-v02.bin  aka NSS-C.DAT    ;CRC32: A8E202B3 (version "02" oldest)
  NSS-v03a.bin aka NSS-IC14.02  ;CRC32: E06CB58F (version "03" older)
  NSS-v03b.bin aka NSS-V3.ROM   ;CRC32: AC385B53 (version "03" newer/patch)
```

**NSS Cartridge ROM-Images**

ROM-Images should consist of following components in following order:

```text
  1. PRG-ROM (the SNES game) (usually 512Kbytes or 1024Kbytes)
  2. INST-ROM (the Z80 title & instructions) (32Kbytes)
  3. PROM (decryption key) (16 bytes)
```

Note: For the Type B/C PCBs, the PROM is 16 bytes in size. The Type A PCBs seem
to be somehow different - details are still unknown; the ROM-image format may
need to be changed in case that those details are discovered.

The existing cartridges don't contain any coprocessors - if somebody should
make such cartridges, please insert the coprocessor ROM (eg. DSP1) between
PRG-ROM and INST-ROM.

**NSS Games**

```text
  PCB Title
  C   Act Raiser (NSS) 1992 Enix (Two EPROMs+DIPSW)
  C   Addams Family, The (NSS) 1992 Ocean (Two EPROMs+DIPSW)
  C   Contra 3: The Alien Wars (NSS) 1992 Konami (Two EPROMs+SRAM+DIPSW)
  C   David Crane's Amazing Tennis (NSS) 1992 Abs.Ent.Inc. (Two EPROMs+DIPSW)
  B   F-Zero (NSS) 1991 Nintendo (ROM+SRAM)
  C   Irem Skins Game, The (NSS) 1992 Irem (Two EPROMs+DIPSW)
  C   Lethal Weapon (NSS) 1992 Ocean (Two EPROMs+DIPSW)
  -   Magic Floor (NSS) 2012 nocash (EPROM+DIPSW, works without PROM)
  C   NCAA Basketball (NSS) 1992 Sculptured Software Inc. (Two EPROMs+DIPSW)
  C   Robocop 3 (NSS) 1992 Ocean (Two EPROMs+DIPSW)
  A   Super Mario World (NSS) 1991 Nintendo (ROM)
  A   Super Soccer (NSS) 1992 Human Inc. (EPROM)
  A   Super Tennis (NSS) 1991 Nintendo (ROM)
```

Additionally, Ocean has announced Push-Over (unknown if that was ever
released). And, there seems to have been a Super Copa cartridge in Mexico. And,
there is somebody owning a NHL Stanley Cup prototype cartridge.

Contra 3 also appears to exist as prototype only (its INST-ROM
title/instructions are just saying "New Game 1" and "To be announced").

<a id="nsscomponentlists"></a>

## NSS Component Lists

**Cartridge PCB "NSS-01-ROM-A" (1991 Nintendo)**

```text
  IC1   32pin  PRG ROM (LH534J ROM or TC574000 EPROM) (512Kx8 LoROM)
  IC2   16pin  74HC367 (2bit + 4bit drivers) (unknown purpose... for PROM?)
  IC3   28pin  INST-ROM (27C256) (32Kx8 EPROM)
  IC4   8pin   Key-Chip (RP5H01 serial 72bit PROM)
  CL/SL 2pin   Jumpers (see notes)
  CN?   100pin Cartridge connector (2x50pin)
```

Used by Super Mario World (ROM), Super Tennis (ROM), and Super Soccer (EPROM).

For ROM: Short CL1-CL5, Open SL1-SL5. For EPROM: Short SL1-SL5, Open CL1-CL5.

**Cartridge PCB "NSS-01-ROM-B" (1991 Nintendo)**

```text
  IC1   28pin  SRAM (LH5168FB-10L)
  IC2   32pin  PRG ROM (LH534J ROM) (512Kx8 LoROM)
  IC3   16pin  74LS139 (demultiplexer) (for ROM vs SRAM mapping)
  IC4   16pin  74HC367 (2bit + 4bit drivers) (unknown purpose... for PROM?)
  IC5   14pin  74HC27 (3x3 NOR) (for SW1) (not installed on the F-Zero board)
  IC6   14pin  74HC10 (3x3 NAND)(for SW1) (not installed on the F-Zero board)
  IC7   20pin  74HC540 (inv.drv)(for SW1) (not installed on the F-Zero board)
  IC8   28pin  INST-ROM (27C256) (32Kx8 EPROM)
  IC9   8pin   Key-Chip (RP5H01 serial 72bit PROM)
  SW1   16pin  DIP-Switch (8 switches)    (not installed on the F-Zero board)
  AR1   9pin   Resistor network (for SW1) (not installed on the F-Zero board)
  BAT1  2pin   Battery (CR2032 3V coin) (with socket)
  CL/SL 2pin   Jumpers (see notes)
  CN?   100pin Cartridge connector (2x50pin)
```

Used only by F-Zero. For that game: Short CL1-CL7, Open SL1-SL7. Other settings
might allow to use EPROM instead ROM, or to change ROM/SRAM capacity.

**Cartridge PCB "NSS-01-ROM-C" (1992 Nintendo)**

Judging from low-res photos, the PCB is basically same as NSS-01-ROM-B, but
with two PRG ROM chips (for double capactity). Exact components are unknown,
except for a few ones:

```text
  IC1   28pin  SRAM (6116, 2Kx8) (DIP24 in 28pin socket?) (Contra III only)
  IC2   32pin  PRG-ROM-1 (TC574000 EPROM) (512Kx8 LoROM, upper half)
  IC3   32pin  PRG-ROM-0 (TC574000 EPROM) (512Kx8 LoROM, lower half)
  IC4   16pin  74LS139 (demultiplexer) (for ROM vs SRAM mapping)
  IC5   16pin  74HC367 (2bit + 4bit drivers) (unknown purpose... for PROM?)
  IC6   14pin  74HC27 (3x3 NOR) (for SW1)
  IC7   14pin  74HC10 (3x3 NAND)(for SW1)
  IC8   28pin  INST ROM (27C256) (32Kx8 EPROM)
  IC9   20pin  74HC540 (inv.drv)(for SW1)
  IC10  8pin   Key-Chip (RP5H01 serial 72bit PROM)
  SW1?  16pin  DIP-Switch (8 switches)  (installed)
  AR1   9pin   Resistor network for SW1 (installed)
  BAT1? 2pin   Battery (CR2032 3V coin) (with socket) (Contra III only)
  CL/SL 2pin   Jumpers (see notes)
  CN?   100pin Cartridge connector (2x50pin)
```

Used by ActRaiser, Addams Family, Amazing Tennis, Irem Skins Game, Lethal
Weapon, NCAA Basketball, Robocop 3 (all without SRAM), and, by Contra III (with
SRAM). Default (for all those games) is reportedly: Short
CL2-CL6,CL12-CL13,CL15,CL17-CL19, Open SL1,SL7-SL12,SL14,SL16,SL20-SL22.

DIP Switches are usually/always installed. Battery/SRAM is usually NOT
installed, except on the Contra III cartridge (which has "NSS-01-ROM-C" PCB
rebadged as "NSS-X1-ROM-C" with a sticker).

**Mainboard NSS-01-CPU MADE IN JAPAN (C) 1991 Nintendo**

Below lists only the main chipset (not the logic chips; which are mostly
located on the bottom side of the PCB).

Standard SNES Chipset

```text
  S-CPU 5A22-02 (QFP100)
  S-PPU1 5C77-01 (QFP100)
  S-PPU2 5C78-01 (QFP100)
  S-WRAM LH68120 (SOP64) 128Kx8 DRAM with sequential access feature (SNES WRAM)
  Fujitsu MB84256-10L 32Kx8 SRAM (SOP28) (SNES VRAM LSBs)
  Fujitsu MB84256-10L 32Kx8 SRAM (SOP28) (SNES VRAM MSBs)
```

NSS/Z80 Specific Components

```text
  Zilog Z84C0006FEC Z80 CPU, clock input 4.000MHz (QFP44)
  27C256 32Kx8 EPROM "NSS-C_IC14_02" (DIP28) (Z80 BIOS)
  Sharp LH5168N-10L 8Kx8 SRAM (SOP28) (Z80 WRAM)
  Mitsubishi M50458-001SP On-Screen Display (OSD) Chip (NDIP32)
  Mitsubishi M6M80011 64x16 Serial EEPROM (DIP8)
   (Pinout: 1=CS, 2=CLK, 3=DATA IN, 4=DATA OUT, 5=VSS, 6=RESET, 7=RDY, 8=VCC)
  Seiko Epson S-3520 Real Time Clock (SOIC14)
```

Amplifiers/Converters/Battery and so

```text
  Sharp IR3P32A (chroma/luma to RGB converter... what is that for???) (NDIP30)
  Hitachi HA13001 Dual 5.5W Power Amplifier IC
  Matsushita AN5836 DC Volume and Tone Control IC (SIL12)
  Mitsumi Monolithic MM1026BF Battery Controller (SOIC8) (on PCB bottom side)
  5.5V - 5.5 volt supercap
```

Oscillators

```text
  21.47724MHz SNES NTSC Master Clock <-- not 21.47727MHz, unlike NTSC (?)
  14.31818MHz (unknown purpose, maybe for OSD chip or RGB converter or so)
  4.000MHz for Z80 CPU
  32.678kHz for RTC
  <unknown clock source> for OSD Dotclock
```

Connectors

```text
  CN1 - 2x28 pin connector - "JAMMA" - Audio/Video/Supply/Coin/Joypad
  CN2 - 10 pin connector - 10P Connector (Extra Joypad Buttons)
  CN3 - 13 pin connector - 13P Connector (Front Panel LEDs/Buttons)
  CN4 - 8 pin connector - alternate player 2 controller (eg. lightgun) (unused)
  CN5 - 7 pin connector - external 5bit input (Port 02h.R.bit3-7) (unused)
  CN6 - 24 pin connector (to APU daughterboard)
  CN11/12/13 - 2x50 pin connectors for game cartridges
```

Jumpers

```text
  SL1/SL2/SL3/CL1/CL2 - Mono/stero mode (for details see PCB text layer)
  SL4 - Use Audio+ (pin 11 on edge connector)
  SL5 - Unknown purpose
  TB1 - Z80 Watchdog Disable
```

APU Daughterboard (shielded unit, plugged into CN6 on mainboard)

```text
  Nintendo S-SMP (M) SONY (C) Nintendo '89' (QFP80) (SNES SPC700 CPU)
  Nintendo S-DSP (M) (C) SONY '89' (QFP80) (SNES sound chip)
  Toshiba TC51832FL-12 32Kx8 SRAM (SOP28) (1st half of APU RAM)
  Toshiba TC51832FL-12 32Kx8 SRAM (SOP28) (2nd half of APU RAM)
  Japan Radio Co. JRC2904 Dual Low Power Op Amp (SOIC8)
  NEC D6376 Audio 2-Channel 16-Bit D/A Converter (SOIC16)
  CN1 - 24 pin connector (to CN6 on mainboard)
  <unknown clock source> for APU (probably SNES/APU standard 24.576MHz)
```

<a id="nssonscreencontrollerosd"></a>

## NSS On-Screen Controller (OSD)

On-Screen Display Controller M50458-001SP (Mitsubishi Microcomputers)

**OSD Addresses**

The OSD Address is transferred as first word (after chip select):

```text
  0000h..011Fh  Character RAM (24x12 tiles, aka 288 tiles, aka 120h tiles)
  0120h..0127h  Configuration Registers (8 registers)
```

Further words are then written to the specified address (which is
auto-incremented after each word).

**Character Codes (for OSD Address 000h..011Fh)**

```text
  0-6   Character Number (non-ASCII)
  7     Unused (zero)
  8-10  Text Color     (on NSS: 3bit RGB) (Bit0=Red, Bit1=Green, Bit2=Blue)
  11    Blinking flag  (0=Normal, 1=Blink)
  12    Underline flag (0=Normal, 1=Underline)
  13-15 Unused (zero)  (on NSS: used as hidden PROM check flags by NSS BIOS)
```

The M50458-001SP charset has been dumped by DogP, letters &amp; punctuation
marks are:

```text
  Character  <---00h..0Fh---><---10h..1Fh---><---20h..2Fh---><---30h..3Fh--->
  00h..3Fh  "0123456789-:/.,'ABCDEFGHIJKLMNOPQRSTUVWXYZ[]();?| "
  40h..7Fh  "_abcdefghijklmnopqrstuvwxyz+*=# "
```

All characters are 12x18 pixels in size.

**OSD M50458 Register 0 - Port Output Control**

```text
  0     P0 Usage (0=Manual Control, 1=YM; Luminance)
  1     P1 Usage (0=Manual Control, 1=BLNK; Blanking)
  2     P2 Usage (0=Manual Control, 1=B; Blue)
  3     P3 Usage (0=Manual Control, 1=G; Green)
  4     P4 Usage (0=Manual Control, 1=R; Red)
  5     P5 Usage (0=Manual Control, 1=CSYN; Composite Sync)
  6-11  Manual P0-P5 Output Level (0=Low, 1=High)
  12    Synchronize Port Output with Vsync (0=No, 1=Yes)
  13-15 Unused (zero)
```

NSS uses values 003Fh (whatever/maybe SNES as backdrop), and 00BDh (maybe solid
backdrop).

**OSD M50458 Register 1 - Horizontal Display Start/Zoom**

```text
  0-5   Horizontal Display Start in 4-pixel (?) units
  6-7   Horizontal Character Size in Line 1     (0..3 = 1,2,3,4 pixels/dot)
  8-9   Horizontal Character Size in Line 2..11 (0..3 = 1,2,3,4 pixels/dot)
  10-11 Horizontal Character Size in Line 12    (0..3 = 1,2,3,4 pixels/dot)
  12    PAL: Interlace Lines (0=625 Lines, 1=627 Lines) NTSC: Unused (zero)
  13-15 Unused (zero)
```

NSS uses 0018h (normal centered display) and 011Bh (fine-adjusted position in
intro screen).

**OSD M50458 Register 2 - Vertical Display Start/Zoom**

```text
  0-5   Vertical Display Start in 4-scanline (?) units
  6-7   Vertical Character Size in Line 1     (0..3 = 1,2,3,4 pixels/dot)
  8-9   Vertical Character Size in Line 2..11 (0..3 = 1,2,3,4 pixels/dot)
  10-11 Vertical Character Size in Line 12    (0..3 = 1,2,3,4 pixels/dot)
  12    Halftone in Superimpose Display (0=Halftone Off, Halftone On)
  13-15 Unused (zero)
```

NSS uses 0009h (normal centered display) and 0107h (fine-adjusted position in
intro screen).

**OSD M50458 Register 3 - Character Size**

```text
  0-4   Vertical Scroll Dot Offset (within char) (0..17) (18..31=Reserved)
  5-6   Vertical Space between Line 1 and 2 (0..3 = 0,18,36,54 scanlines)
  7     Control RS,CB Terminals (0=Both Off, 1=Both On)
  8-11  Vertical Scroll Char Offset (0=No Scroll, 1..11=Line 2-12, 12..15=Res.)
  12    PAL: Revise 25Hz Vsync (0=No, 1=Yes/Revice)  NTSC: Unused
  13-15 Unused (zero)
```

NSS uses 0000h (normal 1x1 pix size) and 082Ah (large 2x2 pix "NINTENDO" in
intro), 0y20h (in-demo: instructions with double-height headline? and y-scroll
on 2nd..10th line), 0y00h (in-game: instructions without headline and
fullscreen scroll).

Verical Scroll OFF: Show 12 lines

Verical Scroll ON: Show 11 lines (1st line fixed, 10 lines scrolled)

(in scroll mode only 11 lines are shown)

(allowing to update the hidden 12th line without disturbing the display)

**OSD M50458 Register 4 - Display Mode**

```text
  0-11  Display Mode Flags for Line 1..12 (0=Via BLK0,BLK1, 1=Via Different)
  12    LINEU - Underline Display (0=Off, 1=On) "depends on above bit0-bit11"
  13-15 Unused (zero)
```

NSS uses 0000h.

**OSD M50458 Register 5 - Blinking and so on**

```text
  0-1   Blink Duty  (0=Off, 1=25%, 2=50%, 3=75%) (WHAT color during WHAT time?)
  2     Blink Cycle (0=64 Frames, 1=32 Frames)
  3     Horizontal Border Size (0..1 = 1,2 dots)
  4-5   Blink/Inverse Mode (0=Cursor, 1=ReverseChr, 2=ReverseBlink, 3=AltBlink)
           aka EXP0,EXP1 (see details below)
  6     Horizontal Display Range when all chars are in matrix-outline (0..1=?)
  7     OSCIN frequency (0=4*fsec, 1=2*fsec) (for NTSC only)
  8     Color Burst Width (0=Standard, 1=Altered)
  9     Vsync Signal separated from Composite Sync (0=No, 1=Separated Circut)
  10-12 Test Register "Exception video RAM display mode" (should be zero)
  13-15 Unused (zero)
```

NSS uses 0240h, 0241h, 0247h.

**OSD M50458 Register 6 - Raster Color**

```text
  0-2   Raster Color    (on NSS: 3bit RGB) (Bit0=Red, Bit1=Green, Bit2=Blue)
          (aka Backdrop color?)
  3     Composite Signal BIAS (0=Internal BIAS Off, 1=Internal BIAS On)
  4-6   Character Background Color         (Bit0=Red, Bit1=Green, Bit2=Blue)
  7     Blanking Level (0=White, 1=Black)
  8-10  Cursor and Underline Display Color (Bit0=Red, Bit1=Green, Bit2=Blue)
  11    Cursor/Underline Color for Dot 1  (0=From VRAM, 1=From above bit8-10)
  12    Cursor/Underline Color for Dot 18 (0=From VRAM, 1=From above bit8-10)
  13-15 Unused (zero)
```

NSS uses 1804h, 1880h, 1882h, 1884h.

**OSD M50458 Register 7 - Control Display**

```text
  0     Raster (backdrop?) blanking (0=By Mode;bit2-3?, 1=Whole TV full raster)
  1     Background Color Brightness for RGB (0=Normal, 1=Variable) huh?
  2-3   Mode (0=Blanking OFF, 1=Chr Size, 2=Border Size, 3=Matrix-outline Size)
            aka special meanings in conjunction with register 4 (?)
  4     Mode (0=External Sync, 1=Internal Sync)
  5     Erase RAM (0=No, 1=Erase RAM) (=clear screen?)
  6     Display Output Enable for Composite Signal (0=Off, 1=On)
  7     Display Output Enable for RGB Signal       (0=Off, 1=On)
  8     Stop OSCIN/OSCOUT (0=Oscillate, 1=Stop) (for sync signals)
  9     Stop OSC1/OSC2    (0=Oscillate, 1=Stop) (for display)
  10    Exchange External C by Internal C in Y-C Mode (0=Normal, 1=Exchange)
  11    Video Signal (0=Composite, 1=Y-C output)
  12    Interlace Enable (0=Enable, 1=Disable) (only in Internal Sync mode)
  13-15 Unused (zero)
```

NSS uses 1289h, 12A9h and 12B9h.

**NSS OSD Dotclock**

The OSD chip is having an unknown dotclock (somewhat higher than the SNES
dotclock: 12 pixels on OSD are having roughly the same width as 8 pixels on
SNES).

**Blink/Underline**

```text
  <Register> <VramAttr> Shape
  EXP1 EXP0  EXP BLINK
  x    x     0   0      " A "             Normal
  x    x     0   1      " A " <--> "   "  Character is blinking
  0    0     1   0      "_A_"             Underlined
  0    0     1   1      "_A_" <--> " A "  Underline is blinking
  0    1     1   0      "[A]"             Inverted Character
  0    1     1   1      "[A]" <--> " A "  Inversion is blinking
  1    0     1   0      "[A]"             Inverted Character
  1    0     1   1      "[A]" <--> " A "  Inversion is blinking
  1    1     1   0      "   " <--> " A "  Character is blinking, duty swapped
  1    1     1   1      " A " <--> "_ _"  Character and Underline alternating
```

<a id="sfcboxoverview"></a>

## SFC-Box Overview

**Main Menu**

```text
  Allows to select from 5 games
```

Note: If the two cartridges do contain more than 5 games in total, then the GUI
is divided into two pages with 4 games, plus prev/next page option (unknown if
more than 8 games are also supported).

**Per Game Menu (after selecting a Game in Main Menu)**

```text
  1. Game Start
  2. Game Instructions
  3. Game Preview
  4. Return to Main Menu
```

**Soft-Reset Feature**

```text
  Press L+R+Select+Start (on Joypad) --> Reset Current Game
  Press Reset Button (on SFC-Box Front Panel) --> Restart Boot Menu
```

**GAME/TV Button**

Allows to switch between Game &amp; TV mode. The purpose is totally unclear...
maybe it just allows to disable forwarding the Antenna-input to the RF-Out
connector... but, &lt;why&gt; should one want to disable that?

**SFC-Box Cartridges**

The SFC-Box contains two special multi-game cartridges. There have been only 4
cartridges produced. The first cartridge MUST be always PSS61 (contains 3
games, plus the required GUI and 128Kbyte SRAM). The second cartridge can be
PSS62, PSS63, or PSS64 (which contain 2 games each; these carts have no own
SRAM, but they can share portions of the SRAM from the PSS61 cart).

**SFC-Box Special ROM/EPROMs**

```text
  KROM 1     EPROM 64Kbytes     (HD64180 BIOS in SFC-Box console)
  GROM1-1    EPROM 32Kbytes     (Directory) (IC1 in PSS61 cart)
  GROM2-1    EPROM 32Kbytes     (Directory) (IC1 in PSS62 cart)
  GROM3-1    EPROM 32Kbytes     (Directory) (IC1 in PSS63 cart)
  GROM4-1    EPROM 32Kbytes     (Directory) (IC1 in PSS64 cart)
  ATROM-4S-0 LoROM 512Kbytes    (GUI "Attraction" Menu) (ROM5 in PSS61 cart)
  DSP1       DSP ROM 8Kbytes    (or with padding: 10Kbytes)
  MB90082    OSD ROM 9Kbytes    (OSD-Character Set in MB90082-001 chip)
```

ATROM-4S-0 contains a regular SNES header at 7FC0h, interesting entries are:

```text
  7FC0h Title "4S ATTRACTION        "
  7FD6h Coprocessors (00h) (none, but, ATROM can communicate with the HD64180)
  7FD8h RAM Size (00h) (none, but, GROM indicates 32Kbytes allocated to ATROM)
  7FDAh Maker (B6h) (HAL)
```

Note: "GROM3-1" is dumped (but its ROM-image is "conventionally" misnamed as
"GROM1-3"). There is reportedly also a "different" GUI version (not
confirmed/details unknown, maybe there's just a configuration setting, in SRAM
or EPROMs or so, that changes the GUI appearance).

**SFC-Box Game ROMs**

```text
  SHVC-4M-1  LoROM 2048Kbytes (Mario Collection) (ROM3 in PSS61 cart)
  SHVC-MK-0  HiROM 512Kbytes  (Mario Kart)       (ROM12 in PSS61 cart)
  SHVC-FO-1  LoROM 1024Kbytes (Starfox)          (IC20 in PSS61 cart)
  SHVC-GC-0  LoROM 1024Kbytes (WaiaraeGolf)      (ROM1 in PSS62 cart)
  SHVC-2A-1  HiROM 512Kbytes  (Mahjong)          (ROM9 in PSS62 cart)
  SHVC-8X-1  HiROM 4096Kbytes (Donkey Kong)      (ROM7 in PSS63 cart)
  SHVC-T2-1  LoROM 1024Kbytes (Tetris2/Bombliss) (ROM3 in PSS63 cart)
  SHVC-8X-1  HiROM 4096Kbytes (Donkey Kong)      (ROM7 in PSS64 cart)
  SHVC-M4-0  HiROM 1024Kbytes (Bomberman2)       (ROM9 in PSS64 cart)
```

All Game ROMs seem to be identical as in normal (japanese) cartridges, (ie.
without any SFC-Box specific revisions).

**SFC-Box ROM-Images**

ROM-Images should contain all EPROMs/ROMs from the cartridge, ordered as so:

```text
  GROM + ROM0(+ROM1(+ROM2(+etc))) (+DSP1)
```

The GROM at the begin of the file does also serve as file header:

```text
  The size of the GROM (1 SHL N kbytes) is found in GROM [0001h].
  The number of ROMs is found in GROM [0000h].
  Title & Size of ROM<n> can be found at [[0008h]+n*2]*1000h.
  Physical IC Socket ID for ROM<n> can be found in GROM at [0008h]+[0000h]*2+n.
  The presence of a DSP ROM Image is indicated in GROM [0004h].Bit1.
```

With that information, one can calculate the file-offsets for each ROM.

If desired, one may merge two cartridges images in one file, eg.

```text
  GROM1+ROM0+ROM1+ROM2+ROM3+DSP + GROM2+ROM0+ROM1
```

Before merging GROM+ROMs, make sure that the ROMs are raw-images (without
512-byte copier headers), and that the DSP ROM is unpadded (8Kbytes), in
little-endian format.

The additional "non-cartridge" ROMs of the SFC-Box (KROM1 and MB90082) should
be located in a separate BIOS folder; not in the cartridge ROM-Image.

**SFC-Box Crashes**

Some bugged ATROM functions (7E2125h and 7E2173h) are messing up the SNES
stack, causing the SNES to run into endless execution of BRK opcodes (thereby
destroying lower 8K of WRAM, any enabled SRAM, and all I/O ports). Normally,
SNES emulators could stop emulation in such "beyond-repair" situations -
however, for the SFC-Box, emulation must be kept running (or better: crashing),
since the KROM can restore normal operation by issuing a /RESET to the SNES
(for example, this is happens near completion of the "**********" progress bar
in the SFC-Box boot screen). Moreover, the KROM does change SNES mapping (via
Port C0h/C1h), apparently without pausing/resetting the SNES CPU during that
time, thus causing SNES to execute garbage code (though there are also working
situations: eg. when checking the GAME headers, the SNES executes ATROM code
relocated to WRAM). And, there seems to be a situation (maybe caused by above
stuff) where the SNES NMI handler jumps to Open Bus regions. Note: In most or
all cases, the crashing program is running into BRK opcodes (emulating BRK
opcodes as "leave PC and SP unchanged" helps avoiding the more hazardous
crash-effects).

<a id="sfcboxcoprocessorhd64180extendedz80"></a>

## SFC-Box Coprocessor (HD64180) (extended Z80)

This is the "heart" of the SFC-Box. The two central parts are a HD64180 CPU
(with extended Z80 instruction set), and a 64Kbyte EPROM labelled "KROM 1"
(HD64180 BIOS). Plus, a frightening amount of about 50 small logic chips on the
mainboard &amp; daughterboard.

**Overall Features are (probably)...**

```text
  - Injecting Controller Data (for Demo/Preview mode)
  - Sniffing Controller Data (for L+R+Select+Start Soft-Reset feature)
  - Send/Receive Data to the SNES Menu Program (via WRIO/RDIO ports)
  - Mapping the selected Game ROM (or Menu Program) into SNES memory
  - Maybe also mapping GAME-SRAM bank(s) and/or the DSP-1 chip
  - Resetting the SNES, for starting the Game ROM (or Menu Program)
  - Reading "GROM" data from EPROMs in the cartridges
  - Reportedly drawing an extra "OSD" video layer on top of the SNES picture
  - Accessing the RTC Real-Time-Clock (unknown purpose)
  - Somehow logging/counting or restricting the "pay-per-play" time
  - Maybe handling the GAME/TV button in whatever fashion
  - Maybe handling the RESET button by software
  - Maybe controlling the two GAME/TV LEDs
```

**Pay-per-play**

Not much known there. Some people say the SFC-Box was coin-operated... but, it
doesn't contain any coin-slot, and there seem to be no external connectors for
external coin-slot hardware. And, there seem to be no external connectors for a
"network-cable" for automatically charging the room-bill.

**Usage of [4201]=RDIO / [4213]=WRIO on SNES Side (used by Menu Program)**

Default WRIO output value is 00E6h.

```text
  bit0 Out (usually Output=LOW, from SNES) (maybe indicate ready)
  bit1 In  (data in, to SNES)
  bit2 In  (status/ready/malfunction or so, to SNES)
  bit3 Out (clock/ack out, from SNES)
  bit4 Out (data out, from SNES)
  bit5 In  (clock in, to SNES)
  bit6 -   (probably normal joy1 io-line)
  bit7 -   (probably normal joy2 io-line & lightgun latch)
```

After booting, the SNES menu program checks the initial "1bit" status, and does
then repeatedly receive 32bit packets (one command byte, and three parameter
bytes) (and, in response to certain commands, it does additionally send or
receive further bytes; in some cases these extra transfers are done via
joy1/joy2 shift-registers instead of via WRIO, that probably because the
HLL-coded KROM is so incredibly inefficient that it needs
"hardware-accelerated" serial shifts).

<a id="sfcboxmemoryiomaps"></a>

## SFC-Box Memory & I/O Maps

most of KROM is high-level-language based crap,

this is ACTUALLY WORSE than 6502-code compiled

to run on a Z80 CPU.

**Physical 19bit Memory Map**

```text
  00000h..00FFFFh   KROM
  20000h..207FFFh   WRAM (mainly 204000h..207FFFh used)
                           (area at 200000h..203FFFh used as battery-ram?
                           with read/write-protect via [A0].7?)
  40000h..407FFFh   GROM-Slot 0
  60000h..607FFFh   GROM-Slot 1
```

**Virtual 16bit Memory Map**

```text
  0000h..7FFFh      KROM (first 32K)
  8000h..BFFFh      Bank Area (16K banks, KROM,GROM,WRAM)
  C000h..FFFFh      WRAM (last 16K)
```

**RAM (as used by KROM1)**

```text
 [8000...]               <-- extra 16K RAM bank (unchanged on
                             reset/entrypoint... probably battery backed?)
  ...                        (that 16K are read/write-protected via [A0].7 ?)
 [C000..FFFF]  work ram
```

**I/O Map**

```text
  [00h..3Fh]  HD64180 (CPU on-chip I/O ports)
  [40h..7Fh]  Unused (reading returns FFh)
  [80h].R     Keyswitch and Button Inputs
  [80h].W     SNES Transfer and Misc Output
  [81h].R     SNES Transfer and Misc Input
  [81h].W     Misc Output
  [82h].R/W   Unknown/unused
  [83h].R     Joypad Input/Status
  [83h].W     Joypad Output/Control
  [84h].R/W   Joypad 1, MSB (1st 8 bits) (eg. Bit7=ButtonB, 0=Low=Pressed)
  [85h].R/W   Joypad 1, LSB (2nd 8 bits) (eg. Bit0=LSB of ID, 0=Low=One)
  [86h].R/W   Joypad 2, MSB (1st 8 bits) (eg. Bit7=ButtonB, 0=Low=Pressed)
  [87h].R/W   Joypad 2, LSB (2nd 8 bits) (eg. Bit0=LSB of ID, 0=Low=One)
  [88h..9Fh]  Unused (mirrors of Port 80h..87h)
  [A0h].R     Real Time Clock Input
  [A0h].W     Real Time Clock Output
  [A1h..BFh]  Unused (mirror of Port A0h)
  [C0h].R     Unknown/unused (reading returns FFh)
  [C0h].W     SNES Mapping Register 0
  [C1h].R     Unknown/unused (reading returns FFh)
  [C1h].W     SNES Mapping Register 1
  [C2h..FFh]  Unused (maybe mirrors of Port C0h..C1h) (reading returns FFh)
```

16bit I/O Space (when address MSB=xx=nonzero):

```text
  [xx00h..xx7Fh]  Unused (reading returns FFh) (no mirror of 0000h..003Fh)
  [xx80h..xxBFh]  Mirror of 0080h..00BFh
  [xxC0h..xxFFh]  Unknown (probably mirror of 00C0h..00FFh)
```

<a id="sfcboxioportscustomports"></a>

## SFC-Box I/O Ports (Custom Ports)

**[80h].R - Keyswitch and Button Inputs**

```text
  0   Switch Pin0 Position ("OFF") Play Mode? (2nd from left) (0=Yes, 1=No)
  1   Switch Pin1 Position ("ON")  Play Mode? (3rd from left) (0=Yes, 1=No)
  2   Switch Pin2 Position ("2")   Play Mode? (4th from left) (0=Yes, 1=No)
  3   Switch Pin3 Position ("3")   Self-Test  (5th from left) (0=Yes, 1=No)
  4   Switch Pin9 Position ("1")   Options    (1st from left) (0=Yes, 1=No)
  5   Switch Pin4 Position (N/A)   Relay Off? (6th from left) (0=Yes, 1=No)
  6   TV/GAME Button (0=On, 1=Off)
  7   RESET Button   (0=On, 1=Off)
```

**[80h].W - SNES Transfer and Misc Output**

```text
  0   SNES Transfer STAT to SNES  (Bit2 of WRIO/RDIO on SNES side)
  1   SNES Transfer CLOCK to SNES (Bit5 of WRIO/RDIO on SNES side)
  2   SNES Transfer DATA to SNES  (Bit1 of WRIO/RDIO on SNES side)
  3   Unknown/unused
  4     ?? pulsed while [C094] is nonzero (0370h timer0 steps)
  5     ??         PLENTY used (often same as bit7)
  6   Unknown/unused
  7     ??                     (often same as bit5)
```

**[81h].R - SNES Transfer and Misc Input**

```text
  0   Int0 Request (Coin-Input, Low for 44ms..80ms) (0=IRQ, 1=No)
  1   SNES Transfer ACK from SNES  (Bit3 of WRIO/RDIO on SNES side)
  2   SNES Transfer DATA from SNES (Bit4 of WRIO/RDIO on SNES side)
  3   Boot mode or so (maybe a jumper, or watchdog-flag, or Bit0 of WRIO/RDIO?)
  4   Unknown/unused (0) ;\joy1/slot0 or so, used by an UNUSED function (08A0h)
  5   Unknown/unused (0)  ;/(for "joy2/slot1" or so, use [A0].4-5)
  6   Int1 Request (Joypad is/was accessed by SNES or so?) (0=IRQ, 1=No)
  7   Vblank, Vsync, or Whatever flag (seems to toggle at 100..200Hz or so?)
```

**[81h].W - Misc Output**

```text
  0   SNES Reset CPU/PPU/APU/GSU/DSP1 or so (0=Reset, 1=Normal)
  1     ??         PLENTY used
  2     ??  something basic, ATROM related (or maybe HALT snes CPU?)
  3   Int1 Acknowledge (Joypad related) (0=Ack, 1=Normal)
  4     ??         PLENTY used
  5     ?? set to 1-then-0 upon init (maybe ACK/RESET something?)
  6   Watchdog Reload (must be pulsed during mem tests/waits/transfers/etc)
  7   OSD Chip Select (for CSI/O) (0=No, 1=Select)
```

**[83h].R - Joypad Input/Status**

```text
  0   Joy2 Port [86h..87h] ready for reading (0=No, 1=Yes)     ;\Automatic
  1   Joy1 Port [84h..85h] ready for reading (0=No, 1=Yes)     ;/Reading
  2   Unknown/unused (usually/always 0)
  3   Unknown/unused (usually/always 1) (maybe joy4 Data?)
  4   Unknown/unused (usually/always 1) (maybe joy3 Data?)
  5   Joy2 Data   (0=Low, 1=High) ;\that is inverse as on SNES ;\Manual
  6   Joy1 Data   (0=Low, 1=High) ;/(where it'd be 1=Low)      ;/Reading
  7   Unknown/unused (usually/always 0)
```

**[83h].W - Joypad Output/Control**

```text
  0   Joypad Strobe  (0=No, 1=Yes)                             ;\Manual
  1   Joypad2? Clock (0=Yes, 1=No)                             ; Reading
  2   Joypad1? Clock (0=Yes, 1=No)                             ;/
  3   Joypad Reading (0=Automatic, 1=Manual)
  4   Joypad Swap    (0=Normal, 1=Swap Joy1/Joy2)
  5-7 Unknown/unused (should be 0)
```

Not quite clear if the "Swap" feature affects... software/hardware? upon
reading/writing? upon manual/automatic access?

**[84h].R/W - Joypad 1, MSB (1st 8 bits) (eg. Bit7=ButtonB, 0=Low=Pressed)**

**[85h].R/W - Joypad 1, LSB (2nd 8 bits) (eg. Bit0=LSB of ID, 0=Low=One)**

**[86h].R/W - Joypad 2, MSB (1st 8 bits) (eg. Bit7=ButtonB, 0=Low=Pressed)**

**[87h].R/W - Joypad 2, LSB (2nd 8 bits) (eg. Bit0=LSB of ID, 0=Low=One)**

2x16bit Joypad data from Controller / to SNES. In Automatic Reading mode, data
is automatically forwarded to SNES, and if desired, it can be read from
[84h..87h] (when [83h].0-1 are zero). The clock source for reading is unknown
(maybe it comes from the SNES, so it'd work only IF the SNES is reading).

In Manual Reading mode, joypad can be read via [83h], and data can be then
forwarded to SNES by writing to [84h..87h].

Notes: Observe that the bits are inverse as on SNES (where it'd be 1=Low).
Aside from [83h..87h], joypad seems to also somehow wired to INT1 interrupt
(and [81h].R.Bit6 and [81h.W.Bit3). Also observe that [84h/86h] are containing
the MSBs (not LSBs). The KROM1/ATROM are also mis-using [84h..87h] for
general-purpose "high-speed" data transfers (that is, faster than the crude HLL
coded software transfers in KROM1).

**[A0h].R - Real Time Clock Input (S-3520)**

```text
  0   RTC Data In      (0=Low=Zero, 1=High=One)
  1   Unknown/unused (usually/always 0)
  2   Unknown/unused (usually/always 0)
  3   Unknown/unused (usually/always 1)
  4   Unknown/unused (0) ;\joy2/slot1 or so, used by an UNUSED function (08A0h)
  5   Unknown/unused (0) ;/(for "joy1/slot0" or so, use [81].4-5)
  6   Unknown/used?! (usually/always 1)   used/flag ?   extra BUTTON ?
  7   Unknown/used?! (usually/always 1)   used/flag ?   extra BUTTON ?
```

**[A0h].W - Real Time Clock Output (S-3520)**

```text
  0   RTC Chip Select  (0=High=No,   1=Low=Select)
  1   RTC Direction    (0=Low=Write, 1=High=Read)
  2   RTC Data Out     (0=Low=Zero,  1=High=One)
  3   RTC Serial Clock (0=Low=Clk,   1=High=Idle)
  4     ??     cleared after "C632" offhold (5 timer1 steps)
  5   Unknown/Set to 0 (can be changed via 0A2Dh)
  6   Unknown/Unused   (can be changed via 0A26h)
  7   Unlock access to lower 16K of WRAM (0=Lock, 1=Unlock) (save area)
```

**[C0h].W - SNES Mapping Register 0**

```text
  0-1 ROM Socket  (0=ROM5, 1=ROM1/7/12, 2=ROM3/9, 3=IC20)
  2   ROM Slot    (0=Slot0, 1=Slot1)
  3   SRAM Enable (0=Disable, 1=Enable)
  4   SRAM Slot   (0=Slot0, 1=Slot1)
  5   DSP Enable  (0=Disable, 1=Enable)
  6   DSP Slot    (0=Slot0, 1=Slot1)
  7   ROM, DSP, and/or SRAM Mapping (0=LoROM, 1=HiROM)
```

**[C1h].W - SNES Mapping Register 1**

```text
  0-1 ROM, DSP, and/or SRAM Mapping (0=Reserved, 1=GSU, 2=LoROM, 3=HiROM)
  2-3 SRAM Base   (in 32Kbyte units) (range 0..3)
  4   GSU Slot    (0=Slot0, 1=Slot1)
  5   Zero/Unused?
  6-7 SRAM Size   (0=2K, 1=8K, 2=Reserved, 3=32K)
```

**[82h].R/W - Unknown/unused**

**[C0h].R - Unknown/unused**

**[C1h].R - Unknown/unused**

Not used by KROM1 (nor GROMs). Reading from these ports usually/always returns
FFh.

<a id="sfcboxioportshd64180ports"></a>

## SFC-Box I/O Ports (HD64180 Ports)

**System Clock**

The CPU and Timer/Baudrate-Prescalers are clocked at PHI=4.608MHz (derived from
a 9.216MHz oscillator, and internally divided by 2 in the HD64180).

**asci_ch0 - implemented, tx is "used" for UNUSED joypad recording**

**asci_ch1 - implemented, but rx+tx both unused**

```text
  initialized to 8N1 with baudrate 28.8 kbit/s (PHI/10/16 SHR 0)
```

**csio - implemented, tx is used - OSD video chip**

```text
  initialized to 230.4 kbit/s (PHI/20 SHR 0)
  chipselect is controlled via port [81h].W.Bit7 (1=select, 0=deselect)
  the OSD chip is having an unknown dotclock (higher than the SNES)
  (12 pixels on OSD are having roughly the same width as 8 pixels on SNES)
```

**timer0**

```text
  timer0 should run at 4.608MHz/20/130 --> 1772.3 Hz
```

**timer1**

```text
  timer1 should run at 4.608MHz/20/3840 --> 60.0 Hz
```

**external interrupts**

```text
  reset  power-up, and maybe watchdog? (but, probably not "RESET" button?)
  nmi    unknown/unused
  int0   coin ? (must be low for 78..140 timer0 ticks) (44ms..80ms)
  int1   joypad is/was accessed by snes ?
  int2   unknown/unused
```

**CPU Registers**

```text
  Port Name      Expl.                                    (On Reset)
  [00] CNTLA0    ASCI Channel 0 Control Reg A             (10h, bit3=var)
  [01] CNTLA1    ASCI Channel 1 Control Reg A             (10h, bit3=var)
  [02] CNTLB0    ASCI Channel 0 Control Reg B             (07h, bit7/bit5=var)
  [03] CNTLB1    ASCI Channel 1 Control Reg B             (07h, bit7=var)
  [04] STAT0     ASCI Channel 0 Status Register           (00h, bit1/2=var)
  [05] STAT1     ASCI Channel 1 Status Register           (02h)
  [06] TDR0      ASCI Channel 0 Transmit Data Register
  [07] TDR1      ASCI Channel 1 Transmit Data Register
  [08] RDR0      ASCI Channel 0 Receive Data Register
  [09] RDR1      ASCI Channel 1 Receive Data Register
```

```text
  [0A] CNTR      CSI/O Control Register                      (0Fh)
  [0B] TRDR      CSI/O Transmit/Receive Data Register
```

```text
  [0C] TMDR0L    Timer 0 Counter "Data" Register, Bit0-7     (FFh)
  [0D] TMDR0H    Timer 0 Counter "Data" Register, Bit8-15    (FFh)
  [0E] RLDR0L    Timer 0 Reload Register, Bit0-7             (FFh)
  [0F] RLDR0H    Timer 0 Reload Register, Bit8-15            (FFh)
  [10] TCR       Timer Control Register                      (00h)
  [14] TMDR1L    Timer 1 Counter "Data" Register, Bit0-7     (FFh)
  [15] TMDR1H    Timer 1 Counter "Data" Register, Bit8-15    (FFh)
  [16] RLDR1L    Timer 1 Reload Register, Bit0-7             (FFh)
  [17] RLDR1H    Timer 1 Reload Register, Bit8-15            (FFh)
```

```text
  [18] FRC       Free Running Counter (not used by SFC-Box)  (FFh)
  [20-31] (DMA)  DMA Registers        (not used by SFC-Box)
  [36] RCR       Refresh Control Reg  (not used by SFC-Box)  (FCh)
  [3F] ICR       I/O Control Register (not used by SFC-Box)  (1Fh)
  [11-13]        Reserved             (not used by SFC-Box)
  [19-1F]        Reserved             (not used by SFC-Box)
  [35]           Reserved             (not used by SFC-Box)
  [37]           Reserved             (not used by SFC-Box)
  [3B-3E]        Reserved             (not used by SFC-Box)
```

```text
  [32] DCNTL     DMA/WAIT Control Register                   (F0h)
  [33] IL        Interrupt Vector Low Register               (00h)
  [34] ITC       INT/TRAP Control Register                   (39h)
  [38] CBR       MMU Common Base Register (Common Area 1)    (00h)
  [39] BBR       MMU Bank Base Register (Bank Area)          (00h)
  [3A] CBAR      MMU Common/Bank Area Register               (F0h)
```

OSD_INIT

```text
  OUT[81h]=00h                  ;osd chip deselect
  OUT[0Ah]=00h                  ;init CSIO
  for i=1 to 4,OUT[81H]=80h,OUT[81H]=00h,next  ;osd wake-up from reset-state
```

OSD_SEND_CMD

```text
  ;in: HL=param10bit, A=(80h OR cmd*8)
  SHL  L    ;move bit7 to cy
  RCL  H    ;shift-in cy
  SHR  L    ;undo SHL (now bit7=0 for second byte)
  OR   A,H  ;merge command and 3bit data
  CALL osd_send_byte_a
  LD   A,L  ;7bit data
  JMP  osd_send_byte_a
```

OSD_SEND_BYTE

```text
  set OUT[81h]=80h                ;osd chip select
  set OUT[0Bh]=data               ;prepare TX data
  set OUT[0Ah]=10h                ;start TX
  wait until (IN[0Ah] AND 10h)=0  ;wait until TX ready
  set OUT[81h]=00h                ;osd chip deselect
```

<a id="sfcboxosdchiponscreendisplaycontroller"></a>

## SFC-Box OSD Chip (On-Screen Display Controller)

**OSD Command Summary**

```text
  CMD, First Byte (Command+Data)   Second Byte (More Data)  Function
  BASE  b7 b6 b5 b4 b3 b2 b1 b0    b7 b6 b5 b4 b3 b2 b1 b0
  ---  +--+-----------+--------+  +--+--------------------+ -----------------
  0 80 |1 |0  0  0  0 |FL A8 A7|  |0 |A6 A5 A4 A3 A2 A1 A0| Preset VRAM Addr
  1 88 |1 |0  0  0  1 |D2 D1 D0|  |0 |C2 C1 C0 BS B2 B1 B0| Select Color
  2 90 |1 |0  0  1  0 |AT -  M7|  |0 |M6 M5 M4 M3 M2 M1 M0| Write Character
  3 98 |1 |0  0  1  1 |S2 S1 S0|  |0 |SC SC SC -  SB SB SB| Sprite Ctrl 1
  4 A0 |1 |0  1  0  0 |IE IN EB|  |0 |MM CM MP NP -  -  DC| Screen Ctrl 1
  5 A8 |1 |0  1  0  1 |LP DM SG|  |0 |FM SV SD -  W2 W1 W0| Screen Ctrl 2
  6 B0 |1 |0  1  1  0 |BK G1 G0|  |0 |BC VD DG N3 N2 N1 N0| Line Control
  7 B8 |1 |0  1  1  1 |EC XE FO|  |0 |-  -  Y4 Y3 Y2 Y1 Y0| Vertical Offset
  8 C0 |1 |1  0  0  0 |SC XS FC|  |0 |-  X5 X4 X3 X2 X1 X0| Horizontal Offset
  9 C8 |1 |1  0  0  1 |-  -  - |  |0 |-  -  -  -  -  -  - | Reserved
  A D0 |1 |1  0  1  0 |XC XB RA|  |0 |R2 R1 R0 RS U2 U1 U0| Set under-color
  B D8 |1 |1  0  1  1 |-  -  - |  |0 |-  -  -  -  -  -  - | Reserved (Used?)
  C E0 |1 |1  1  0  0 |-  XC XC|  |0 |XC XC XC XD XD XD XD| Sprite Ctrl 2
  D E8 |1 |1  1  0  1 |-  YC YC|  |0 |YC YC YD YD YD YD YD| Sprite Ctrl 3
  E F0 |1 |1  1  1  0 |-  -  - |  |0 |-  -  -  -  -  -  - | Reserved
  F F8 |1 |1  1  1  1 |-  -  - |  |0 |-  -  -  -  -  -  - | Reserved
```

Note: Below descriptions are showing only the 10bit parameter values (without
command bits in 1st byte, and without zero-bit in 2nd byte).

**OSD Command 0 (80h) - Preset VRAM Address**

```text
  9    FL  Fill Mode (0=Normal, 1=Fill)
  8-5  An  Address A8-A5 (aka Bit3-0 of Y) (range 0..11)
  4-0  An  Address A4-A0 (aka Bit4-0 of X) (range 0..23)
```

**OSD Command 1 (88h) - Select Color**

```text
  9-7  Dn  Unknown Color?   ;SFCBOX/MB90089 only, not MB90075 (per CHARACTER)
  6-4  Cn  Character Color (can be GRAYSCALE or COLOR)        (per CHARACTER)
  3    BS  Unknown (Shade?) ;MB90089 only, not MB90075/SFCBOX
  2-0  Bn  Background Color (always GRAYSCALE)       (per SCREEN or per LINE?)
```

**OSD Command 2 (90h) - Write Character**

```text
  9    AT  Character Background (0=Normal/Transp, 1=Solid)  ;SFCBOX/MB90089
  8    0   Character Blink (0=Off, 1=Blink)   ;SFCBOX only, not MB90075/MB90089
  7-0  Mn  Character Tile Number (ASCII) (20h=Normal Space, FFh=Transp Space)
```

Before Command 2: Change the VRAM address and Character Color via Commands 0
and 1 (if needed).

Upon Command 2: The specified character is stored in VRAM (together with
previously specified color attributes), VRAM address is automatically
incremented (and wraps from X=23 to X=0 in next line). If Fill Mode is enabled,
then Command 2 repeats until reaching the end of VRAM (Fill may take up to 1ms,
do not send further commands during that time).

Writes aren't performed during /HSYNC period (approx 3us), as a simple
workaround, configure serial access rate so that an 8-bit transfer takes more
than 3us.

**OSD Command 4 (A0h) - Screen Control 1**

```text
  9    IE  Internal/External Sync (0=Internal/Color, 1=External/Mono)
  8    IN  Interlace Mode (0=On, 1=Off)
  7    EB  Unknown (EB)     ;MB90089 only, not MB90075/SFCBOX
  6    MM  Unknown (MM)     ;MB90089 only, not MB90075/SFCBOX
  5    CM  Color/Monochrome (0=Mono, 1=Color) (affects Character + Undercolor)
  4    MP  Unknown (MP)     ;MB90089 only, not MB90075/SFCBOX
  3    NP  NTSC/PAL Mode    (0=NTSC, 1=PAL)
  2-1  0   Reserved (should be 0)
  0    DC  Display Enable   (0=Backdrop, 1=Backdrop+Background+Characters)
```

xxx pg17 - details in IE

**OSD Command 5 (A8h) - Screen Control 2**

```text
  9    LP  Unknown    ;MB90089 only, not MB90075/SFCBOX
  8    DM  Unknown    ;MB90089 only, not MB90075/SFCBOX
  7    SG  Unknown    ;MB90089 only, not MB90075/SFCBOX
  6    FM  Unknown    ;MB90089 only, not MB90075/SFCBOX
  5    SV  Unknown    ;MB90089 only, not MB90075/SFCBOX
  4    SD  Unknown    ;MB90089 only, not MB90075/SFCBOX
  3    -   Reserved (should be 0)
  2-0  Wn  Line Spacing    ;SFCBOX/MB90089 only, not MB90075
```

Used by SFC-Box.

**OSD Command 6 (B0h) - Line Control**

```text
  9    BK  Background Type (0=Bordered, 1=Solid 12x18)        ;-per LINE
  8    G1  Character Y-Size  (0=Normal/18pix, 1=Zoomed/36pix) ;-per LINE
  7    G0  Character X-Size  (0=Normal/12pix, 1=Zoomed/24pix) ;-per LINE
           (old MB90075: G0=Unused, G1=Affects both X+Y Size)
  6    BC  Background Control (0=Transparent, 1=Displayed)    ;-per LINE
  5    VD  Analog VOUT,YOUT,COUT Video Enable (0=Off, On)     ;\per SCREEN
  4    DG  Digital VOC2-VOC0,VOB Video Enable (0=Off, On)     ;/
  3-0  Nn  Vertical Line Number (N3-N0) (range 0..11) <-- for per LINE bits
```

Note: On the screen, a double-height line at Line Y extends through Line Y and
Y+1, drawing does then continue reading the next VRAM source data from line Y+2
(ie. line Y+1 is NOT drawn).

**OSD Command 7 (B8h) - Vertical Offset**

```text
  9    EC  Output on /HSYNC Pin (0=Composite Video, 1=Hsync)
  8    XE  Unknown (XE)     ;SFCBOX/MB90089 only, not MB90075
  7    FO  Output on FSCO Pin (0=Low, 1=Color Burst)  ;SFCBOX/MB90089 only
  6    0   Reserved (should be 0)
  5    Y5  MSB of Yn?       ;SFCBOX only, not MB90075/MB90089
  4-0  Yn  Vertical Display Start Position (Y4-Y0) (in 2-pixel steps)
```

Vertical Display Start is at "Y*2+1" lines after raising /VBLK. Whereas,
raising /VBLK is 15h (NTSC) or 20h (PAL) lines after raising /VSYNC).

SFC-Box seems to use a 6bit offset (so maybe MB90082 has 1-pixel steps).
SFC-Box writes totally bugged values to bit7-9 (writes them to bit8-10, and
replaces them by HORIZONTAL bits when changing the VERTICAL setting).

**OSD Command 8 (C0h) - Horizontal Offset**

```text
  9    SC  Input on /EXHSYN Pin (0=Composite Video, 1=Hsync)
  8    XS  Unknown (XS)     ;SFCBOX/MB90089 only, not MB90075
  7    FC  Input on /EXHSYN Pin (0=Use3usFilter, 1=NoFilter)  ;MB90089 only
  6-5  0   Reserved (should be 0)
  5    X5  MSB of Xn        ;SFCBOX/MB90089 only, not MB90075
  4-0  Xn  Horizontal Display Start Position (X4-X0)
```

MB90075: Horizontal Display Start is at "(X+15)*12" dots after raising /HSYNC.

MB90082: Horizontal Display Start is at "(X+?)*?" dots after raising /HSYNC.

MB90089: Horizontal Display Start is at "(X+?)*3" dots after raising /HSYNC.

**OSD Command A (D0h) - Set Under-color**

```text
  9    XC  Unknown (XC)     ;MB90089 only, not MB90075  ;sth on SFCBOX?
  8    XB  Unknown (XB)     ;MB90089 only, not MB90075  ;sth on SFCBOX?
  7    RA  Unknown (RA)     ;MB90089 only, not MB90075/SFCBOX
  6-4  Rn  Unknown (R2-R0)  ;MB90089 only, not MB90075/SFCBOX
  3    RS  Unknown (RS)     ;MB90089 only, not MB90075/SFCBOX
  2-0  Un  Under Color (U2-U0) (aka Backdrop (and Border?) color)
```

Under Color can be COLOR or GRAYSCALE (select via CM bit). Under Color is shown
only in INTERNAL sync mode.

**OSD Command B (D8h) - Reserved (Used?)**

This, in SFCBOX and MB90092, is similar to "Sprite Control 1" in MB90089 ???

```text
  9    -   Unknown (unused by SFC-Box)    ;not MB90075, not MB90089
  8    ?   Unknown (used by SFC-Box)      ;not MB90075, not MB90089
  7    -   Unknown (unused by SFC-Box)    ;not MB90075, not MB90089
  6-4  ?   Unknown (used by SFC-Box)      ;not MB90075, not MB90089
  3    -   Unknown (unused by SFC-Box)    ;not MB90075, not MB90089
  2-0  ?   Unknown (used by SFC-Box)      ;not MB90075, not MB90089
```

Used by SFC-Box. Is that a MB90082-only feature? The SFC-Box software contains
a function for setting a 1bit flag, and two 3bit parameters (however, it's
clipping the "3bit" values to range 0..3); the function is used only once (with
flag=00h, and with the other two parameters each set to 01h, and "unused" bits
set to zero).

**OSD Command 3 (98h) - Sprite Control 1 (TileNo, Char/BG Colors?)**

**OSD Command C (E0h) - Sprite Control 2 (Horizontal Position?)**

**OSD Command D (E8h) - Sprite Control 3 (Vertical Position?)**

```text
  Unknown    ;MB90089 only, not MB90075
```

Not used by SFC-Box. According to the poor MB90089 data sheet, TileNo seems to
select char "8Fh+(0..7)*10h". And X/Y coordinates seem to consist of character
coordinates &amp; pixel-offsets within that character cell.

**OSD Wake-Up from Reset**

After Power-on, the OSD chip is held in Reset-state (with IE=0 and DC=0). To
wake-up from that state, issue four /CS=LOW pulses.

**OSD Video RAM (VRAM)**

Main VRAM is 288 cells (24x12), each cell contains:

```text
  8bit Character Tile Number
  3bit Character Color
  probably also 1bit "AT" flag (on chips that do support it)
  plus maybe some more per-character stuff
```

Additionally, there's an array with 12 per-line settings:

```text
  .. zoom bit(s)
  plus maybe some more per-line stuff
```

Other settings (like background &amp; backdrop colors) are per-screen only.

**SFC-Box Character/Outline/Background Styles**

```text
  AT=0      --> draw Background transparent (=Undercolor, or TV layer)
  AT=1      --> draw Background solid by using "Dn" Unknown Color
  AT=0, BK=1, BC=1    --> draw Background solid by "Bn" color (unless Char=FFh)
  AT=x, BK=0, BC=1    --> draw Outline by "Bn" color
```

**OSD Character Generator ROM (CGROM)**

There are 256 characters in CGROM. The Character Set is undocumented, it seems
that one can order chips with different/custom character sets, chips with
suffix -001 in the part number are probably containing some kind of a
"standard" charset.

That "standard" charset contains normal ASCII characters (uppercase &amp;
lowercase, with normal ASCII codes eg. 41h="A", but some missing chars like
"@|\"), plus japanese symbols, and some graphics symbols (volume bar, AM, PM,
No, Tape and arrow symbols).

Character FFh is said to be a "blank/end" code, the meaning there is unknown.
"Blank" might refer to a normal SPACE, but maybe with BG forced to be
transparent (even when using solid BG). "End" might be nonsense, or maybe it
forces the remaining chars (in the current line, and/or following lines) to be
hidden? And/or maybe it acts as CRLF (moving Address to X=0, Y=Y+1)?

**OSD Color Table**

Color Table (according to MB90075 datasheet):

```text
  Value   0     1     2     3       4      5      6      7
  Color   Black Blue  Red   Magenta Green  Cyan   Yellow White
  Mono    Black     ... Increasing  Gray Levels ...      White
```

(ie. Colors are RGB with Bit0=Blue, Bit1=Red, Bit2=Green)

```text
  Characters      --> Grayscale or Color (depending on CM bit)
  Background      --> Always Grayscale
  Backdrop        --> Grayscale or Color (depending on CM bit)
```

Colors should be enabled (via CM bit) only in INTERNAL sync mode.

**OSD Datasheets / Application Manuals**

```text
  MB80075 commands described on 9 pages  ;\these are all 24x12 cells
  MB80082 no datasheet exists?           ; (with extra features in
  MB80089 commands summarized on 1 page  ;/MB80082 and MB80089)
  MB80092 commands described on .. pages ;-similar/newer chip or so
  MB80050 commands summarized on 1 page  ;-different/newer chip or so
```

**MB90092**

```text
  pg80 and up
```

**MB90089**

```text
  vram 24x12 cells (288x216 pixels)
  cgrom (character generator rom) (256 chars, 12x18 pixels)
  base.x  3pix (1/4 character)  ;\relative to END of vblank/hblank
  base.y  2 pix                 ;/
  8 color/grayscales
  shaded (3d) or bordered chars or solid bg
  8 custom chars, one can be displayed
  colors only in INTERNAL sync mode
  mono in EXTERNAL sync mode
  zoom.x / zoom.y  12x18 pix ---> 24x36 pix
  pg10 --> background
  pg11 --> shade
  pg11 --> sprite
  pg11 --> base xy   (6bit x, 5bit y) line space 0..7
  pg12 --> serial access
  pg13 --> COMMANDS
  pg15 --> wake-up
  dot clock (pins EXD,XD) can be 6MHz .. 8MHz (=affects horizontal resolution)
```

FFh is "blank" or "end" code?

display details pg10,11

**MB90075**

```text
  base.x is only 5bit (not 6bit)
  zoom affects BOTH x+y (cannot be done separately)
  no SHADED drawing
  vram fill function ?
  CMD 01 --> without D2..D0
  CMD 02 --> without AT
  CMD 03,0C,0D --> reserved (no sprite functions)
  CMD 04 -->
  CMD 05 --> reserved (no screen ctrl 2)
  CMD 06 --> lacks G0 (but DOES have G1 ?)
  CMD 07 --> lacks XE,FO
  CMD 08 --> lacks XS,FC,X5
  CMD 0A --> only U2..U0 (lacks XC,XB,RA,R2,R1,R0,RS)
```

details on pg14..21

<a id="sfcboxgromformat"></a>

## SFC-Box GROM Format

All SFC-Box Cartridges are containing a 32Kbyte "GROM" EPROM, the chip contains
info about the ROMs in the cartridge (title, instructions, etc).

**GROM Overall Memory Map**

```text
  0000h - Root Header and HD64180 Code
  1000h - ROM File Info Block(s)  ;usually at 1000h,2000h,3000h,etc.
  7FFCh - Checksum (above bytes at 0000h..7FFBh added together)
  7FFEh - Complement (same as above Checksum, XORed by FFFFh)
```

The various unused locations are usually FFh-filled. Checksum/Complement are
located at the end of the EPROM (7FFCh in case of the usual 32Kbyte EPROMs).

**GROM - Root Header (located at 0000h)**

```text
  0000h 1   Number of ROMs (01h..08h) (usually 02h or 04h) (NumROMs)
  0001h 1   GROM size (1 SHL N) kbytes (usually 05h=32Kbytes) (FFh=None)
  0002h 1   Unknown (00h or 01h or 09h)
  0003h 1   Unknown (00h)
  0004h 1   Chipset (07h or 00h) (Bit0:SRAM, Bit1:DSP, Bit2:GSU?)
  0005h 1   Unknown (01h or 00h)   Menu Flag?
  0006h 2   Offset to HD64180 code  (usually 0020h or 0030h)
  0008h 2   Offset to ROM Directory (usually 0010h)
  000Ah 2   Unknown (0000h)
  000Ch 1   Unknown (78h) (aka 120 decimal)
  000Dh 1   Unknown (B0h or 00h) (theoretically unused... but isn't FFh)
  000Eh 2   Unknown (FFFFh) (probably unused)
```

ROM Directory (usually located at 0010h):

```text
  NumROM words  Offset to ROM info, div 1000h (usually 0001h..NumROMs)
  NumROM bytes  Physical Socket on PCB (usually 00h..03h or 01h..02h)
```

The above byte-values (usually located at 0010h+NumROMs*2) can have following
known values:

```text
  00h ROM5            (upper-right) (AttractionMenu)
  01h ROM1/ROM7/ROM12 (lower-right or upper-left) (MarioKar,Mahjong,Donkey)
  02h ROM3/ROM9       (middle-right) (MarioCol,Waiarae,Tetris)
  03h IC20            (special GSU ROM location) (StarFox)
```

HD64180 code (usually at 0020h or 0030h): Around 256-bytes of HD64180 code,
called with following parameters:

```text
  A=function (00h=Change Mapping, FFh=Boot Callback, other=Reserved)
  BC=ptr to 10-bytes
  E=same as [BC+5]
  [BC+0]  ROM Slot (0 or 1)                                ;Cartridge Slot 0-1
  [BC+1]  ROM Socket (0..3)                                ;from GROM[8]
  [BC+2]  Mapmode (0=LoROM, 1=HiROM, 2=GSU)                ;from GROM[P0+16h]
  [BC+3]  Used Chipset (bit0=SRAM, bit1=DSP)               ;from GROM[P0+2Ah]
  [BC+4]  SRAM Size (0=None, 1=2K, 3=8K, 5=32K)            ;from GROM[P0+17h]
  [BC+5]  SRAM Base (0..3) (0..7 when 2 chips)             ;from GROM[P0+1Ch]
  [BC+6]  Slot 1 Chipset (bit0=SRAM, bit1=DSP, bit2=GSU?)  ;from Slot1.GROM[4]
  [BC+7]  Slot 0 Chipset (bit0=SRAM, bit1=DSP, bit2=GSU?)  ;from Slot0.GROM[4]
  [BC+8]  Copy of Port[C0h]  ;\the function must update these values alongside
  [BC+9]  Copy of Port[C1h]  ;/with the new values written to Port C0h/C1h
```

Note: During execution, the 1st 16K of GROM are mapped to 8000h..BFFFh.

**GROM - ROM File n Info (located at 1000h,2000h,3000h,etc.)**

```text
  x000h 2  P0 Offset to ASCII Title and Configuration (usually 000Eh)
  x002h 2  P1 Offset to Bitmap-Title-Tiles  ;\bitmap 128x24 pix (16x3 tiles)
  x004h 2  P2 Offset to Bitmap-Padding-Tile ; padded to 160x24 pix (20x3 tiles)
  x006h 2  P3 Offset to Bitmap-Palette      ;/with 16-color (4bpp) palette
  x008h 2  P4 Offset to Shift-JIS Instruction Pages
  x00Ah 2  P5 Offset to Demo-Joypad-Data (for demo/preview feature)
  x00Ch 2  P6 Offset to Unused-Joypad-Data  ;<-- not included in Attraction ROM
```

ASCII Title and Configuration Field (at P0):

```text
  00h 22 ASCII Title (uppercase ASCII, 22 bytes, padded with spaces)
  16h 1  ROM/SRAM mapping/speed? (00h=SlowLoROM, 01h=FastHiROM, 02h=GSU/NoSRAM)
  17h 1  SRAM Size (1 SHL N Kbytes) (but for Menu: ATROM Header claims NoSRAM?)
  18h 1  Coprocessor is DSP1 (00h=No, 01h=Yes)
  19h 1  ROM Size (in 1MBit units, aka in 128Kbyte Units)
  1Ah 1   Unknown (01h or 02h or 03h)
  1Bh 1  Demo/Preview enable (00h=Off, 01h=On)
  1Ch 1  SRAM Base (0..3) (or 0..7 when SRAMs in BOTH slots) (CHANGED by KROM1)
  1Dh 1  Preferred Title (00h=SNES[FFC0h]/Destroys SNES stack, 01h=GROM[P0])
  1Eh 1   Unknown ("strange values") (can be edited in menu point "2-4-1:3")
  1Fh 1  Always Zero (00h) (seems to be MSB of above entry) (always zero)
  20h 4  Whatever (01h,00h,00h,01h=Menu or 00h,30h,30h,05h=Game)
  24h 1   Unknown (00h or 01h or 02h) (maybe... num players/joypads?)
  25h 1   Unknown (00h or 05h or 1Eh) (aka decimal 0,5,30)
  26h 1  Game Flag (00h=Menu, 01h=Game)
  27h 1   Unknown (00h or 05h or 1Eh) (aka decimal 0,5,30)
  28h 1   Unknown (00h or 80h or 90h or A0h or D0h)
  29h 1   Unknown (00h or 21h or 22h or 23h)
  2Ah 1  Chipset (bit0=Uses SRAM, bit1=Uses DSP)     ;<-- missing in Star Fox
  2Bh 22 Unknown (all 2Eh-filled)       ;<-- located at index 2Ah in Star Fox
```

Bitmap-Title-Tiles (at P1):

```text
  1 byte - Unknown (Should be 80h) (probably bit7=compression flag?)
  2 bytes - Number of following bytes (N) (varies 02D5h..0573h) (max=600h)
  N bytes - Compressed Title Bitmap (128x24 pix, 4bpp) (16x3 Tiles)
  (the uncompressed bitmap consists of 3 rows of 16 bit-planed 4bpp SNES tiles)
  (see below for the compression format)
```

Bitmap-Padding-Tile (at P2):

```text
  32 bytes - Uncompressed Padding Tile (8x8 pix, 4bpp)
  (used to pad the 128x24 pix title bitmap, centered within a 160x24 pix area)
  (should be usually uni-colored tile, with same color as bitmap's background)
  (or, alternately, one could probably also use a "hatched" background pattern)
```

Bitmap-Palette (at P3):

```text
  32 bytes - 16-color Palette for Title Bitmap (words in range 0000h..7FFFh)
  (color 0 is unused/transparent, usually contains 0038h as dummy value)
```

Shift-JIS Instruction Pages (at P4):

```text
  1 byte - Number of Pages
  N bytes - Page(s) ;max 1372 bytes per page  ;21 lines = max 6+(32*2+1)*21+1
  Each page starts with a 6-byte header (usually 8,2,4,1,4,4), followed
  by Text (mixed 7bit ASCII, 8bit JIS, and 2x8bit Shift-JIS), lines are
  terminated by chr(09h), each page terminated by chr(00h).
```

Demo-Joypad-Data (at P5):  ;&lt;-- if none: eight 00h-bytes

```text
  1 byte - Unknown (usually 05h)
  2 byte - Number of following 4-Byte Pairs (N)
  N*4 bytes - data (most 4-byte pairs are "xx,FF,FF,FF")
  (controller-data for demo/preview, in format: Time,Lsb,Msb,FFh)
  (or rather: 8bit time, 12bit joy1, 12bit joy2 ...?)
```

Unused-Joypad-Data (at P6):   ;&lt;-- if none: four 00h-bytes

```text
  Unknown purpose. The GROMs have more controller data here (similar as
  at P5), but the existing KROM/ATROM do not seem to use that extra data.
```

**Title Bitmap Compression**

[SNES Decompression Formats](#snes-decompression-formats)

<a id="sfcboxcomponentlistcartridges"></a>

## SFC-Box Component List (Cartridges)

**SFC-Box Cartridge PCB (GS 0871-102)**

```text
  IC1  28pin  DIP 27C256 EPROM "GROMn-1" (usually 28pin; 28pin/32pin possible)
  IC2  20pin  SMD Philips 74HC273D
  IC3  20pin  SMD Philips 74HC541D
  IC4  20pin  SMD Philips 74HC273D
  IC5  14pin  SMD <unknown>
  IC6  14pin  SMD <unknown>
  IC7  14pin  SMD <unknown>
  IC8  16pin  SMD <unknown> 74HC138                     (semi-optional)
  IC9  16pin  SMD <unknown> 74HC138                     (semi-optional)
  IC10 16pin  SMD <unknown> HC138 or HC130 or so?       (semi-optional)
  IC11 16pin  SMD <unknown> 74HC138 or 74HC130 or so ?  (semi-optional)
  IC12 16pin  SMD <unknown> (near IC16)                 (semi-optional)
  IC13 16pin  SMD Philips 74HC153D                      (semi-optional)
  IC14 20pin  DIP GAL16V8B
  IC15 14pin  SMD 74AC125 (near IC17)
  IC16 32pin  DIP Sony CXK581000P-12L (SRAM 128Kx8)          (optional)
  IC17 28pin  DIP Nintendo DSP1 A/B (for Mario Kart)         (optional)
  IC18 14pin  SMD <unknown> 74HC04 (below X1)           (semi-optional)
  IC19 100pin SMD Mario Chip 1 (Star Fox GSU)                (optional)
  IC20 32pin  SMD SHVC-FO-1    (Star Fox ROM)                (optional)
  IC21 28pin  SMD HY62256A     (Star Fox RAM, 32Kx8)         (optional)
  IC22 14pin  SMD <unknown> (below IC4)
  IC23 14pin  SMD <unknown> HC08                        (semi-optional)
  ROM1 36pin  DIP -or- ROM7  36pin DIP ;\solder pads for up to six ROMs
  ROM2 36pin  DIP -or- ROM8  36pin DIP ; (each with two alternate pin-outs,
  ROM3 36pin  DIP -or- ROM9  36pin DIP ; eg. ROM1=LoROM or ROM7=HiROM)
  ROM4 36pin  DIP -or- ROM10 36pin DIP ; (can be fitted with 32pin/36pin chips
  ROM5 36pin  DIP -or- ROM11 36pin DIP ; except, ROM6 can be 32pin only)
  ROM6 32pin  DIP -or- ROM12 36pin DIP ;/(see IC1 & IC20 for further (EP)ROMs)
  X1   ?pin   DIP <unknown>, oscillator for DSP1, probably 2-3 pins?(optional)
  CN1  100pin DIP OMRON XC5F-0122 Cartridge Connector (female 2x50pin)
```

IC16 is 128K SRAM, this chip is installed in the PSS61 cartridge only, but,
it's shared for multiple games (including games in PSS62-PSS64 carts).

The SRAM isn't battery-backed, however, the SFC-Box cannot be switched off
(unless when unplugging supply cables), so SRAM should be always receiving a
standby-voltage from the console.

The hardware might allow to share the DSP1 chip in similar fashion (?), in the
existing carts, it's used only for Mario Kart.

The "optional" components are installed in PSS61 only. The "semi-optional" ones
are installed in PSS61 and (for unknown reason) also in PSS62 (whilst,
PSS63/PSS64 don't have them, although they should be functionally same as
PSS62).

Unknown how many different programs are possible (there are pads for max 6 DIP
ROMs, plus 1 SMD ROM, but maybe the SMD is alternate to one DIP, and maybe some
pads are reserved for games with 2 ROMs; and unknown if the GUI menu supports
more than 8 games).

<a id="sfcboxcomponentlistconsole"></a>

## SFC-Box Component List (Console)

**SFC-Box Mainboard "MAIN 0871-100A"**

```text
 Section 1 (CPU/PPU) (Front-Left)
  U1  100pin S-CPU B
  U2  100pin S-PPU1
  U3  100pin S-PPU2 C
  U4  28pin  LH2A256N-10PLL (Mosel-Vitelic, VRAM, 32Kx8)
  U5  28pin  LH2A256N-10PLL (Mosel-Vitelic, VRAM, 32Kx8)
  U6  64pin  S-WRAM A
  U7  24pin  S-ENC A (near APU section)
  U8         N/A ?
  U9  14pin  74HCU04
  U10 8pin   unknown (maybe audio amplifier) (near relay)
  U11 8pin   unknown (maybe audio amplifier) (near relay)
  X1  D21M4 Oscillator (21.47727MHz for S-CPU)
  TC1 Red Trimmer (for above oscillator)
 Section 2 (APU) (Rear-Left)
  IC1 64pin  S-SMP
  IC2 80pin  S-DSP A
  IC3 28pin  HM9453100FP (APU-RAM, 32Kx8)
  IC4 28pin  HM9453100FP (APU-RAM, 32Kx8)
  IC5 16pin  NEC uPD6376 (serial audio D/A converter)
  IC6 8pin   unknown (maybe audio amplifier)
  IC72 28pin MB90082-001 (OSD video controller) (near S-ENC A)
  X2  Blue oscillator (maybe 24.576MHz for APU?)
  X4  D143A4 oscillator (maybe separate NTSC color clock 3.579545MHz mul 4)
  TC2 Trimmer (for X4/D143A4)
  TC3 Trimmer (for IC72/OSD-Chip pin16)
 Section 3 (Rear-Right)
  IC30 80pin  HD64180RF6X (extended Z80 CPU)
  X3  D921B4 oscillator (9.216MHz) (ie. HD64180 clocked at PHI=4.608MHz)
  24 small logic chips (details unknown) (plenty 74HCxxx & 74LSxxx)
 Section 4 (Front-Right)
  18 small logic chips (details unknown)
 Connectors
  CN1 100pin cartridge slot (2x50pin male) (via adaptor to TWO 2x50 slots)
  CN2 44pin  daugtherboard socket (2x22pin male)
  CN3 3pin   unknown/unused (without cable?) (front-right) (coin mechanics?)
  CN4 5pin   Yellow Cable to Front Panel (FR 0871-105) (front-middle)
  CN5 7pin   Yellow Cable to 6-position Keyswitch (front-middle)
  CN6 11pin  Multi-colored Cable (to joypad connectors) (front-left)
  CN? 7pin   Yellow Cable to Modulator (rear-left)
  CN8 6pin   unknown/unused (without cable?) (front-right) (maybe RS232 ???)
      2pin   Black cable to "Nintendo AC Adapter" (Input: DC 5V 10W) (rear)
      2pin   RCA Audio Out Left (rear)
      2pin   RCA Audio Out Right (rear)
 Options & Specials
  SP1 3pin   unknown / jumper (near HD64180) (usually two pins bridged)
  SP2 2pin   unknown / jumper or so (near OSD chip) (usually not bridged)
  SP3 3x5pin unknown / not installed (located in center of mainboard)
  JP1,JP2,JP3,JP4,JP5 - seem to allow to disconnect Shield from GND
  TR1 3pin   unknown (big transistor or so)
  ?   8pin   OMRON G5V-2-5VDC (dual two-position relay) (rear-left)
```

**SFC-Box Daughterboard "(unknown PCB name)" (Modulator)**

Shielded box with whatever contents, plus external connectors:

```text
  2pin   RCA Audio Out Mono      ;\raw A/V (stereo is also available, via
  2pin   RCA Video Out Composite ;/the external connectors on mainboard)
  2pin   RF Out                  ;\
  2pin   ANT In                  ; RF modulated
  ?pin   Channel Select 1CH/2CH  ;/
  7pin   Yellow Cable to Mainboard
```

**SFC-Box Daughterboard "Nintendo AC Adapter" (Power Supply)**

Remove-able metal box with whatever contents, plus external connectors:

```text
  4pin AC OUT 200W MAX (dual 2pin or so)
  2pin DC OUT 5V 5A (via short EXTERNAL cable to Mainboard)
  2pin AC IN (cable to wall socket)
  AC125 5A (Fuse?)
```

**SFC-Box Daughterboard "PU 0871-101"**

```text
  IC1  28pin  DIP 27C512 EPROM "KROM 1" (usually 28pin; 28pin/32pin possible)
  IC2  28pin  SMD SRM20257 (SRAM 32Kx8) (Work-RAM for HD64180, battery-backed)
  IC3  16pin  SMD 74HC139
  IC4  20pin  SMD 74HC273D (Philips)
  IC5  20pin  SMD 74LS541
  IC6  16pin  SMD MB3790 (Fujitsu battery controller)
  IC7  14pin  SMD S-3520CF (Seiko RTC, Real Time Clock, battery-backed)
  IC8  14pin  SMD <unknown>
  IC9  14pin  SMD <unknown>
  X1   2pin   Oscillator (for RTC) ("S441") (probably 32kHz or so)
  TC1         Osc-Adjust (for RTC)
  BAT  2pin   Battery (for IC7/RTC and IC2/SRAM)
  TM1  8pin   Massive connector (not installed) (maybe FamicomBox-style CATV?)
  CN1  44pin  DIP Female Connector 2x22pin (to Mainboard)
```

**SFC-Box Daughterboard "GD 0871-103" (Game Cartridge Connectors)**

```text
  CN?  100pin DIP Female Connector 2x22pin (to Mainboard)
  CN?  100pin DIP Male Connector 2x22pin (to Cartridge 1)
  CN?  100pin DIP Male Connector 2x22pin (to Cartridge 2)
```

**SFC-Box Daughterboard "CC 0871-104" (Controller Connectors)**

```text
  11pin  Multicolored Cable to Mainboard
  7pin   Controller 1  ;\Standard SNES Joypad connectors (for two standard
  7pin   Controller 2  ;/joypads with extra-long cables)
```

**SFC-Box Daughterboard "FR 0871-105" (Front Panel)**

```text
  TV-LED and GAME-LED
  GAME/TV-Button
  RESET-Button
  5pin Yellow Cable (to Mainboard)
```

**SFC-Box Daughterboard "(unknown PCB name)" (Keyswitch)**

```text
  10-Position Keyswitch (requires a key) (6-positions connected)
  7pin Yellow Cable to (to Mainboard) (one common pin, plus 6 switch positions)
```

The 10-position keyswitch is mechanically limited to 6 positions (9,0,1,2,3,4).

There are different keys for different purposes. For example, a "visitor" key
can select only the "ON" and "OFF" positions. The right-most position does
switch a relay, which does... what? Probably switch-off the SFC-Box?

<a id="rtcs3520realtimeclock"></a>

## RTC S-3520 (Real-Time Clock)

**Seiko/Epson S-3520CF Serial 4bit Real-Time Clock (RTC)**

Contains the usual Time/Date registers, plus 120bit battery-backed RAM (aka 15
bytes) (organized in 2 pages of 15 x 4bits).

This chip is used in both Nintendo Super System (NSS), and in Super Famicom
Box.

**Seiko/Epson S-3520CF Register Table**

```text
  Index  Bit3   Bit2   Bit1    Bit0  ;Expl.
  ___Registers in Mode 0_____________ ______________
  0      Sec3   Sec2   Sec1    Sec0  ;Seconds, Low     ;\
  1      0      Sec6   Sec5    Sec4  ;Seconds, High    ;
  2      Min3   Min2   Min1    Min0  ;Minutes, Low     ; Read/Increment-able
  3      0      Min6   Min5    Min4  ;Minutes, High    ;
  4      Hour3  Hour2  Hour1   Hour0 ;Hours, Low       ; (reading returns the
  5      PM/AM  0      Hour5   Hour4 ;Hours, High      ; counter value)
  6      0      Week2  Week1   Week0 ;Day of Week      ;
  7      Day3   Day2   Day0    Day0  ;Day, Low         ; (writing any dummy
  8      0      0      Day5    Day4  ;Day, High        ; value does increment
  9      Mon3   Mon2   Mon1    Mon0  ;Month, Low       ; counter value by 1)
  A      0      0      0       Mon4  ;Month, High      ;
  B      Year3  Year2  Year1   Year0 ;Year, Low        ;
  C      Year7  Year6  Year5   Year4 ;Year, High       ;/
  D      TPS    30ADJ  CNTR    24/12 ;Control Register ;-Read/Write-able
  E      STA    LOST   0       0     ;Status Register  ;-Read only
  ___Registers in Mode 1_____________ ________________
  0-E    x      x      x       x     ;Reserved         ;-Don't use
  ___Registers in Mode 2_____________ ________________
  0-E    SRAM   SRAM   SRAM    SRAM  ;SRAM Page 0      ;-Read/Write-able
  ___Registers in Mode 3_____________ ________________
  0-E    SRAM   SRAM   SRAM    SRAM  ;SRAM Page 1      ;-Read/Write-able
  ___Mode Register (in Mode 0..3)____ ________________
  F      SYSR   TEST   Mode1   Mode0 ;Mode Register    ;-Read/Write-able
```

Whereas, the meaning of the various bits is:

```text
  Sec    Seconds (BCD, 00h..59h)
  Min    Minutes (BCD, 00h..59h)
  Hour   Hours   (BCD, 00h..23h or 01h..12h)
  Day    Day     (BCD, 01h..31h)
  Month  Month   (BCD, 01h..12h)
  Year   Year    (BCD, 00h..99h)
  Week   Day of Week (0..6) (SFC-Box: Unknown assignment) (NSS: 0=Sunday)
  PM/AM  Set for PM, cleared for AM (this is done even when in 24-hour mode)
  24/12  24-Hour Mode (0=12, 1=24) (Time/Date may get corrupted when changed?)
  TPS    Select Reference Waveform for output on Pin8 (0=1024Hz, 1=1Hz)
  30ADJ  Set seconds to zero, and, if seconds was>=30, increase minutes
  CNTR   Reset Counters (0=Normal, 1=Reset)
  SYSR   Reset Counters and Control/Status/Mode Registers (0=Normal, 1=Reset)
  LOST   Time Lost (0=Okay, 1=Lost/Battery failure) (can be reset... how?)
  STA    Time Stable (0=Stable/Sec won't change in next 3.9ms, 1=Unstable)
  Mode   Mode for Register 0-E (0=RTC, 1=Reserved, 2=SramPage0, 3=SramPage1)
```

If STA=0 then it's safe to read the time (counters won't change within next
3.9ms aka 1/256 seconds). If STA=1 then one should wait until STA=0 before
reading the time (else one may miss counter-carry-outs).

**Serial Access**

Set /CLK and /CS to HIGH as default level. Set /WR to desired direction (before
dragging /CS low). Then set /CS to LOW to invoke transfer. Then transfer
index/data/garbage (usually 8 clks for WRITES, and 16 clks for READS). Then set
/CS back HIGH.

Index/Data/Garbage Nibbles are 4bit each (transferred LSB first). Bits should
be output (to DataIn) on falling CLK edge (note: the NSS is doing that
properly, the SFC-Box actually outputs data shortly after falling CLK), and can
be read (from DataOut) at-or-after raising CLK edge. The separate nibbles are:

```text
  Nibble   To RTC                       From RTC
  1st      Index I                      Garbage (old index or so)
  2nd      Data I    (or dummy)         Garbage (data from old index or so)
  3rd      Index II  (or dummy)         Garbage (index I or so)
  4th      Data II   (or dummy)         Data I
  5th      Index III (or dummy)         Garbage (index II or so)
  6th      Data III  (or dummy)         Data II
```

For Writes, one needs to send only 2 nibbles (of which, 2nd nibble is used only
for Control &amp; SRAM writes, for Counter-Increment writes it's only a dummy
value).

For Reads, one needs to send/receive at least 4 nibbles (though most of them
are dummies/garbage; actually used are 1st-To-RTC, and 4th-From-RTC). If
desired, one can read two or more registers by reading/writing 6 or more
nibbles (the NSS BIOS does so).

**Pin-Outs**

[SNES Pinouts RTC Chips](80-timings-unpredictable-pinouts.md#snes-pinouts-rtc-chips)

<a id="z80cpuspecifications"></a>

## Z80 CPU Specifications

[Z80 Register Set](#z80-register-set)

[Z80 Flags](#z80-flags)

[Z80 Instruction Format](#z80-instruction-format)

[Z80 Load Commands](#z80-load-commands)

[Z80 Arithmetic/Logical Commands](#z80-arithmeticlogical-commands)

[Z80 Rotate/Shift and Singlebit Operations](#z80-rotateshift-and-singlebit-operations)

[Z80 Jumpcommands &amp; Interrupts](#z80-jumpcommands-interrupts)

[Z80 I/O Commands](#z80-io-commands)

[Z80 Interrupts](#z80-interrupts)

[Z80 Meaningless and Duplicated Opcodes](#z80-meaningless-and-duplicated-opcodes)

[Z80 Garbage in Flag Register](#z80-garbage-in-flag-register)

[Z80 Compatibility](#z80-compatibility)

[Z80 Pin-Outs](#z80-pin-outs)

[Z80 Local Usage](#z80-local-usage)

<a id="z80registerset"></a>

## Z80 Register Set

**Register Summary**

```text
  16bit Hi   Lo   Name/Function
  ---------------------------------------
  AF    A    -    Accumulator & Flags
  BC    B    C    BC
  DE    D    E    DE
  HL    H    L    HL
  AF'   -    -    Second AF
  BC'   -    -    Second BC
  DE'   -    -    Second DE
  HL'   -    -    Second HL
  IX    IXH  IXL  Index register 1
  IY    IYH  IYL  Index register 2
  SP    -    -    Stack Pointer
  PC    -    -    Program Counter/Pointer
  -     I    R    Interrupt & Refresh
```

**Normal 8bit and 16bit Registers**

The Accumulator (A) is the allround register for 8bit operations. Registers B,
C, D, E, H, L are normal 8bit registers, which can be also accessed as 16bit
register pairs BC, DE, HL.

The HL register pair is used as allround register for 16bit operations. B and
BC are sometimes used as counters. DE is used as DEstination pointer in block
transfer commands.

**Second Register Set**

The Z80 includes a second register set (AF',BC',DE',HL') these registers cannot
be accessed directly, but can be exchanged with the normal registers by using
the EX AF,AF and EXX instructions.

**Refresh Register**

The lower 7 bits of the Refresh Register (R) are incremented with every
instruction. Instructions with at least one prefix-byte (CB,DD,ED,FD, or
DDCB,FDCB) will increment the register twice. Bit 7 can be used by programmer
to store data. Permanent writing to this register will suppress memory refresh
signals, causing Dynamic RAM to lose data.

**Interrupt Register**

The Interrupt Register (I) is used in interrupt mode 2 only (see command "im
2"). In other modes it can be used as simple 8bit data register.

**IX and IY Registers**

IX and IY are able to manage almost all the things that HL is able to do. When
used as memory pointers they are additionally including a signed index byte
(IX+d). The disadvantage is that the opcodes occupy more memory bytes, and that
they are less fast than HL-instructions.

**Undocumented 8bit Registers**

IXH, IXL, IYH, IYL are undocumented 8bit registers which can be used to access
high and low bytes of the IX and IY registers (much like H and L for HL). Even
though these registers do not officially exist, they seem to be available in
all Z80 CPUs, and are quite commonly used by various software.

<a id="z80flags"></a>

## Z80 Flags

**Flag Summary**

The Flags are located in the lower eight bits of the AF register pair.

```text
  Bit Name  Set  Clr  Expl.
  0   C     C    NC   Carry Flag
  1   N     -    -    Add/Sub-Flag (BCD)
  2   P/V   PE   PO   Parity/Overflow-Flag
  3   -     -    -    Undocumented
  4   H     -    -    Half-Carry Flag (BCD)
  5   -     -    -    Undocumented
  6   Z     Z    NZ   Zero-Flag
  7   S     M    P    Sign-Flag
```

**Carry Flag (C)**

This flag signalizes if the result of an arithmetic operation exceeded the
maximum range of 8 or 16 bits, ie. the flag is set if the result was less than
Zero, or greater than 255 (8bit) or 65535 (16bit). After rotate/shift
operations the bit that has been 'shifted out' is stored in the carry flag.

**Zero Flag (Z)**

Signalizes if the result of an operation has been zero (Z) or not zero (NZ).
Note that the flag is set (1) if the result was zero (0).

**Sign Flag (S)**

Signalizes if the result of an operation is negative (M) or positive (P), the
sign flag is just a copy of the most significant bit of the result.

**Parity/Overflow Flag (P/V)**

This flag is used as Parity Flag, or as Overflow Flag, or for other purposes,
depending on the instruction.

Parity: Bit7 XOR Bit6 XOR Bit5 ... XOR Bit0 XOR 1.

8bit Overflow: Indicates if the result was greater/less than +127/-128.

HL Overflow: Indicates if the result was greater/less than +32767/-32768.

After LD A,I or LD A,R: Contains current state of IFF2.

After LDI,LDD,CPI,CPD,CPIR,CPDR: Set if BC&lt;&gt;0 at end of operation.

**BCD Flags (H,N)**

These bits are solely supposed to be used by the DAA instruction. The N flag
signalizes if the previous operation has be an addition or substraction. The H
flag indicates if the lower 4 bits exceeded the range from 0-0Fh. (For 16bit
instructions: H indicates if the lower 12 bits exceeded the range from
0-0FFFh.)

After adding/subtracting two 8bit BCD values (0-99h) the DAA instruction can be
used to convert the hexadecimal result in the A register (0-FFh) back to BCD
format (0-99h). Note that DAA also requires the carry flag to be set correctly,
and thus should not be used after INC A or DEC A.

**Undocumented Flags (Bit 3,5)**

The content of these undocumented bits is filled by garbage by all instructions
that affect one or more of the normal flags (for more info read the chapter
Garbage in Flag Register), the only way to read out these flags would be to
copy the flags register onto the stack by using the PUSH AF instruction.

However, the existence of these bits makes the AF register a full 16bit
register, so that for example the code sequence PUSH DE, POP AF, PUSH AF, POP
HL would set HL=DE with all 16bits intact.

<a id="z80instructionformat"></a>

## Z80 Instruction Format

**Commands and Parameters**

Each instruction consists of a command, and optionally one or two parameters.
Usually the leftmost parameter is modified by the operation when two parameters
are specified.

**Parameter Placeholders**

The following placeholders are used in the following chapters:

```text
  r      8bit  register A,B,C,D,E,H,L
  rr     16bit register BC, DE, HL/IX/IY, AF/SP   (as described)
  i      8bit  register A,B,C,D,E,IXH/IYH,IXL/IYL
  ii     16bit register IX,IY
  n      8bit  immediate 00-FFh                   (unless described else)
  nn     16bit immediate 0000-FFFFh
  d      8bit  signed offset -128..+127
  f      flag  condition nz,z,nc,c AND/OR po,pe,p,m  (as described)
  (..)   16bit pointer to byte/word in memory
```

**Opcode Bytes**

Each command (including parameters) consists of 1-4 bytes. The respective bytes
are described in the following chapters. In some cases the register number or
other parameters are encoded into some bits of the opcode, in that case the
opcode is specified as "xx". Opcode prefix bytes "DD" (IX) and "FD" (IY) are
abbreviated as "pD".

**Clock Cycles**

The clock cycle values in the following chapters specify the execution time of
the instruction. For example, an 8-cycle instruction would take 2 microseconds
on a CPU which is operated at 4MHz (8/4 ms). For conditional instructions two
values are specified, for example, 17;10 means 17 cycles if condition true, and
10 cycles if false.

Note that in case that WAIT signals are sent to the CPU by the hardware then
the execution may take longer.

**Affected Flags**

The instruction tables below are including a six character wide field for the
six flags: Sign, Zero, Halfcarry, Parity/Overflow, N-Flag, and Carry (in that
order). The meaning of the separate characters is:

```text
  s    Indicates Signed result
  z    Indicates Zero
  h    Indicates Halfcarry
  o    Indicates Overflow
  p    Indicates Parity
  c    Indicates Carry
  -    Flag is not affected
  0    Flag is cleared
  1    Flag is set
  x    Flag is destroyed (unspecified)
  i    State of IFF2
  e    Indicates BC<>0 for LDX(R) and CPX(R), or B=0 for INX(R) and OUTX(R)
```

<a id="z80loadcommands"></a>

## Z80 Load Commands

**8bit Load Commands**

```text
 Instruction    Opcode  Cycles Flags  Notes
 ld   r,r       xx           4 ------ r=r
 ld   i,i       pD xx        8 ------ i=i
 ld   r,n       xx nn        7 ------ r=n
 ld   i,n       pD xx nn    11 ------ i=n
 ld   r,(HL)    xx           7 ------ r=(HL)
 ld   r,(ii+d)  pD xx dd    19 ------ r=(ii+d)
 ld   (HL),r    7x           7 ------ (HL)=r
 ld   (ii+d),r  pD 7x dd    19 ------
 ld   (HL),n    36 nn       10 ------
 ld   (ii+d),n  pD 36 dd nn 19 ------
 ld   A,(BC)    0A           7 ------
 ld   A,(DE)    1A           7 ------
 ld   A,(nn)    3A nn nn    13 ------
 ld   (BC),A    02           7 ------
 ld   (DE),A    12           7 ------
 ld   (nn),A    32 nn nn    13 ------
 ld   A,I       ED 57        9 sz0i0- A=I  ;Interrupt Register
 ld   A,R       ED 5F        9 sz0i0- A=R  ;Refresh Register
 ld   I,A       ED 47        9 ------
 ld   R,A       ED 4F        9 ------
```

**16bit Load Commands**

```text
 Instruction    Opcode  Cycles Flags  Notes
 ld   rr,nn     x1 nn nn    10 ------ rr=nn    ;rr may be BC,DE,HL or SP
 ld   ii,nn     pD 21 nn nn 13 ------ ii=nn
 ld   HL,(nn)   2A nn nn    16 ------ HL=(nn)
 ld   ii,(nn)   pD 2A nn nn 20 ------ ii=(nn)
 ld   rr,(nn)   ED xB nn nn 20 ------ rr=(nn)  ;rr may be BC,DE,HL or SP
 ld   (nn),HL   22 nn nn    16 ------ (nn)=HL
 ld   (nn),ii   pD 22 nn nn 20 ------ (nn)=ii
 ld   (nn),rr   ED x3 nn nn 20 ------ (nn)=rr  ;rr may be BC,DE,HL or SP
 ld   SP,HL     F9           6 ------ SP=HL
 ld   SP,ii     pD F9       10 ------ SP=ii
 push rr        x5          11 ------ SP=SP-2, (SP)=rr  ;rr may be BC,DE,HL,AF
 push ii        pD E5       15 ------ SP=SP-2, (SP)=ii
 pop  rr        x1          10 (-AF-) rr=(SP), SP=SP+2  ;rr may be BC,DE,HL,AF
 pop  ii        pD E1       14 ------ ii=(SP), SP=SP+2
 ex   DE,HL     EB           4 ------ exchange DE <--> HL
 ex   AF,AF     08           4 xxxxxx exchange AF <--> AF'
 exx            D9           4 ------ exchange BC,DE,HL <--> BC',DE',HL'
 ex   (SP),HL   E3          19 ------ exchange (SP) <--> HL
 ex   (SP),ii   pD E3       23 ------ exchange (SP) <--> ii
```

**Blocktransfer**

```text
 Instruction    Opcode  Cycles Flags  Notes
 ldi            ED A0       16 --0e0- (DE)=(HL), HL=HL+1, DE=DE+1, BC=BC-1
 ldd            ED A8       16 --0e0- (DE)=(HL), HL=HL-1, DE=DE-1, BC=BC-1
 ldir           ED B0  bc*21-5 --0?0- ldi-repeat until BC=0
 lddr           ED B8  bc*21-5 --0?0- ldd-repeat until BC=0
```

<a id="z80arithmeticlogicalcommands"></a>

## Z80 Arithmetic/Logical Commands

**8bit Arithmetic/Logical Commands**

```text
 Instruction    Opcode  Cycles Flags  Notes
 daa            27           4 szxp-x decimal adjust akku
 cpl            2F           4 --1-1- A = A xor FF
 neg            ED 44        8 szho1c A = 00-A
 <arit>  r      xx           4 szhonc see below
 <arit>  i      pD xx        8 szhonc see below, UNDOCUMENTED
 <arit>  n      xx nn        7 szhonc see below
 <arit>  (HL)   xx           7 szhonc see below
 <arit>  (ii+d) pD xx dd    19 szhonc see below
 <cnt>   r      xx           4 szhon- see below
 <cnt>   i      pD xx        8 szhon- see below, UNDOCUMENTED
 <cnt>   (HL)   xx          11 szhon- see below
 <cnt>   (ii+d) pD xx dd    23 szhon- see below
 <logi>  r      xx           4 szhp00 see below
 <logi>  i      pD xx        8 szhp00 see below, UNDOCUMENTED
 <logi>  n      xx nn        7 szhp00 see below
 <logi>  (HL)   xx           7 szhp00 see below
 <logi>  (ii+d) pD xx dd    19 szhp00 see below
```

Arithmetic &lt;arit&gt; commands:

```text
 add   A,op     see above 4-19 szho0c A=A+op
 adc   A,op     see above 4-19 szho0c A=A+op+cy
 sub   op       see above 4-19 szho1c A=A-op
 sbc   A,op     see above 4-19 szho1c A=A-op-cy
 cp    op       see above 4-19 szho1c compare, ie. VOID=A-op
```

Increment/Decrement &lt;cnt&gt; commands:

```text
 inc   op       see above 4-23 szho0- op=op+1
 dec   op       see above 4-23 szho1- op=op-1
```

Logical &lt;logi&gt; commands:

```text
 and   op       see above 4-19 sz1p00 A=A & op
 xor   op       see above 4-19 sz0p00 A=A XOR op
 or    op       see above 4-19 sz0p00 A=A | op
```

**16bit Arithmetic Commands**

```text
 Instruction    Opcode  Cycles Flags  Notes
 add  HL,rr     x9          11 --h-0c HL = HL+rr    ;rr may be BC,DE,HL,SP
 add  ii,rr     pD x9       15 --h-0c ii = ii+rr    ;rr may be BC,DE,ii,SP (!)
 adc  HL,rr     ED xA       15 szho0c HL = HL+rr+cy ;rr may be BC,DE,HL,SP
 sbc  HL,rr     ED x2       15 szho1c HL = HL-rr-cy ;rr may be BC,DE,HL,SP
 inc  rr        x3           6 ------ rr = rr+1     ;rr may be BC,DE,HL,SP
 inc  ii        pD 23       10 ------ ii = ii+1
 dec  rr        xB           6 ------ rr = rr-1     ;rr may be BC,DE,HL,SP
 dec  ii        pD 2B       10 ------ ii = ii-1
```

**Searchcommands**

```text
 Instruction    Opcode  Cycles Flags  Notes
 cpi            ED A1       16 szhe1- compare A-(HL), HL=HL+1, DE=DE+1, BC=BC-1
 cpd            ED A9       16 szhe1- compare A-(HL), HL=HL-1, DE=DE-1, BC=BC-1
 cpir           ED B1   x*21-5 szhe1- cpi-repeat until BC=0 or compare fits
 cpdr           ED B9   x*21-5 szhe1- cpd-repeat until BC=0 or compare fits
```

<a id="z80rotateshiftandsinglebitoperations"></a>

## Z80 Rotate/Shift and Singlebit Operations

**Rotate and Shift Commands**

```text
 Instruction    Opcode  Cycles Flags  Notes
 rlca           07           4 --0-0c rotate akku left
 rla            17           4 --0-0c rotate akku left through carry
 rrca           0F           4 --0-0c rotate akku right
 rra            1F           4 --0-0c rotate akku right through carry
 rld            ED 6F       18 sz0p0- rotate left low digit of A through (HL)
 rrd            ED 67       18 sz0p0- rotate right low digit of A through (HL)
 <cmd> r        CB xx        8 sz0p0c see below
 <cmd> (HL)     CB xx       15 sz0p0c see below
 <cmd> (ii+d)   pD CB dd xx 23 sz0p0c see below
 <cmd> r,(ii+d) pD CB dd xx 23 sz0p0c see below, UNDOCUMENTED modify and load
```

Whereas &lt;cmd&gt; may be:

```text
 rlc    rotate left
 rl     rotate left through carry
 rrc    rotate right
 rr     rotate right through carry
 sla    shift left arithmetic (b0=0)
 sll    UNDOCUMENTED shift left (b0=1)
 sra    shift right arithmetic (b7=b7)
 srl    shift right logical (b7=0)
```

**Singlebit Operations**

```text
 Instruction    Opcode  Cycles Flags  Notes
 bit  n,r       CB xx        8 xz1x0- test bit n  ;n=0..7
 bit  n,(HL)    CB xx       12 xz1x0-
 bit  n,(ii+d)  pD CB dd xx 20 xz1x0-
 set  n,r       CB xx        8 ------ set bit n   ;n=0..7
 set  n,(HL)    CB xx       15 ------
 set  n,(ii+d)  pD CB dd xx 23 ------
 set r,n,(ii+d) pD CB dd xx 23 ------ UNDOCUMENTED set n,(ii+d) and ld r,(ii+d)
 res  n,r       CB xx        8 ------ reset bit n ;n=0..7
 res  n,(HL)    CB xx       15 ------
 res  n,(ii+d)  pD CB dd xx 23 ------
 res r,n,(ii+d) pD CB dd xx 23 ------ UNDOCUMENTED res n,(ii+d) and ld r,(ii+d)
 ccf            3F           4 --h-0c h=cy, cy=cy xor 1
 scf            37           4 --0-01 cy=1
```

<a id="z80jumpcommandsinterrupts"></a>

## Z80 Jumpcommands & Interrupts

**General Jump Commands**

```text
 Instruction    Opcode  Cycles Flags  Notes
 jp   nn        C3 nn nn    10 ------ jump to nn, ie. PC=nn
 jp   HL        E9           4 ------ jump to HL, ie. PC=HL
 jp   ii        pD E9        8 ------ jump to ii, ie. PC=ii
 jp   f,nn      xx nn nn 10;10 ------ jump to nn if nz,z,nc,c,po,pe,p,m
 jr   nn        18 dd       12 ------ relative jump to nn, ie. PC=PC+d
 jr   f,nn      xx dd     12;7 ------ relative jump to nn if nz,z,nc,c
 djnz nn        10 dd     13;8 ------ B=B-1 and relative jump to nn if B<>0
 call nn        CD nn nn    17 ------ call nn ie. SP=SP-2, (SP)=PC, PC=nn
 call f,nn      xx nn nn 17;10 ------ call nn if nz,z,nc,c,po,pe,p,m
 ret            C9          10 ------ pop PC ie. PC=(SP), SP=SP+2
 ret  f         xx        11;5 ------ pop PC if nz,z,nc,c,po,pe,p,m
 rst  n         xx          11 ------ call n  ;n=00,08,10,18,20,28,30,38
 nop            00           4 ------ no operation
```

**Interrupt Related Commands**

```text
 Instruction    Opcode  Cycles Flags  Notes
 di             F3           4 ------ IFF1=0, IFF2=0  ;disable interrupts
 ei             FB           4 ------ IFF1=1, IFF2=1  ;enable interrupts
 im   0         ED 46        8 ------ read opcode from databus on interrupt
 im   1         ED 56        8 ------ execute call 0038h on interrupt
 im   2         ED 5E        8 ------ execute call (i*100h+databus) on int.
 halt           76         N*4 ------ repeat until interrupt occurs
 reti           ED 4D       14 ------ pop PC, IFF1=IFF2, ACK (ret from INT)
 retn           ED 45       14 ------ pop PC, IFF1=IFF2      (ret from NMI)
 </INT=LOW,IM=0,IFF1=1>  1+var ------ IFF1=0,IFF2=0, exec opcode from databus
 </INT=LOW,IM=1,IFF1=1>     12 ------ IFF1=0,IFF2=0, CALL 0038h
 </INT=LOW,IM=2,IFF1=1>     18 ------ IFF1=0,IFF2=0, CALL [I*100h+databus]
 </NMI=falling_edge>         ? ------ IFF1=0,        CALL 0066h
```

<a id="z80iocommands"></a>

## Z80 I/O Commands

```text
 Instruction    Opcode  Cycles Flags  Notes
 in   A,(n)     DB nn       11 ------ A=PORT(A*100h+n)
 in   r,(C)     ED xx       12 sz0p0- r=PORT(BC)
 in   (C)       ED 70       12 sz0p0- **undoc/illegal** VOID=PORT(BC)
 out  (n),A     D3 nn       11 ------ PORT(A*100h+n)=A
 out  (C),r     ED xx       12 ------ PORT(BC)=r
 out  (C),0     ED 71       12 ------ **undoc/illegal** PORT(BC)=00
 ini            ED A2       16 xexxxx MEM(HL)=PORT(BC), HL=HL+1, B=B-1
 ind            ED AA       16 xexxxx MEM(HL)=PORT(BC), HL=HL-1, B=B-1
 outi           ED A3       16 xexxxx B=B-1, PORT(BC)=MEM(HL), HL=HL+1
 outd           ED AB       16 xexxxx B=B-1, PORT(BC)=MEM(HL), HL=HL-1
 inir           ED B2   b*21-5 x1xxxx same than ini, repeat until b=0
 indr           ED BA   b*21-5 x1xxxx same than ind, repeat until b=0
 otir           ED B3   b*21-5 x1xxxx same than outi, repeat until b=0
 otdr           ED BB   b*21-5 x1xxxx same than outd, repeat until b=0
```

<a id="z80interrupts"></a>

## Z80 Interrupts

**Interrupt Flip-Flop (IFF1,IFF2)**

The IFF1 flag is used to enable/disable INTs (maskable interrupts).

In a raw INT-based system, IFF2 is always having the same state than IFF1.
However, in a NMI-based system the IFF2 flag is used to backup the recent IFF1
state prior to NMI execution, and may be used to restore IFF1 upon NMI
completion by RETN opcode.

Beside for the above 'backup' function, IFF2 itself is having no effect.
Neither IFF1 nor IFF2 affect NMIs which are always enabled.

The following opcodes/events are modifying IFF1 and/or IFF2:

```text
  EI     IFF1=1, IFF2=1
  DI     IFF1=0, IFF2=0
  <INT>  IFF1=0, IFF2=0
  <NMI>  IFF1=0
  RETI   IFF1=IFF2
  RETN   IFF1=IFF2
```

When using the EI instruction, the new IFF state isn't applied until the next
instruction has completed (this ensures that an interrupt handler which is
using the sequence "EI, RET" may return to the main program before the next
interrupt is executed).

Interrupts can be disabled by the DI instruction (IFF=0), and are additionally
automatically each time when an interrupt is executed.

**Interrupt Execution**

An interrupt is executed when an interrupt is requested by the hardware, and
IFF is set. Whenever both conditions are true, the interrupt is executed after
the completion of the current opcode.

Note that repeated block commands (such like LDIR) can be interrupted also, the
interrupt return address on the stack then points to the interrupted opcode, so
that the instruction may continue as normal once the interrupt handler returns.

**Interrupt Modes (IM 0,1,2)**

The Z80 supports three interrupt modes which can be selected by IM 0, IM 1, and
IM 2 instructions. The table below describes the respective operation and
execution time in each mode.

```text
  Mode  Cycles  Refresh  Operation
  0     1+var   0+var    IFF1=0,IFF2=0, read and execute opcode from databus
  1     12      1        IFF1=0,IFF2=0, CALL 0038h
  2     18      1        IFF1=0,IFF2=0, CALL [I*100h+databus]
```

Mode 0 requires an opcode to be output to the databus by external hardware, in
case that no byte is output, and provided that the 'empty' databus is free of
garbage, then the CPU might tend to read a value of FFh (opcode RST 38h, 11
cycles, 1 refresh) - the clock cycles (11+1), refresh cycles (1), and executed
operation are then fully identical as in Mode 1.

Mode 1 interrupts always perform a CALL 0038h operation. The downside is that
many systems may have ROM located at this address, making it impossible to hook
the interrupt handler directly.

Mode 2 calls to a 16bit address which is read from a table in memory, the table
pointer is calculated from the "I" register (initialized by LD I,A instruction)
multiplied by 100h, plus an index byte which is read from the databus. The
following trick may be used to gain stable results in Mode 2 even if no index
byte is supplied on the databus: For example, set I=40h the origin of the table
will be then at 4000h in memory. Now fill the entire area from 4000h to 4100h
(101h bytes, including 4100h) by the value 41h. The CPU will then perform a
CALL 4141h upon interrupt execution - regardless of whether the randomized
index byte is an even or odd number.

**Non-Maskable Interrupts (NMIs)**

Unlike INTs, NMIs cannot be disabled by the CPU, ie. DI and EI instructions and
the state of IFF1 and IFF2 do not have effect on NMIs. The NMI handler address
is fixed at 0066h, regardless of the interrupt mode (IM). Upon NMI execution,
IFF1 is cleared (disabeling maskable INTs - NMIs remain enabled, which may
result in nested execution if the handler does not return before next NMI is
requested). IFF2 remains unchanged, thus containing the most recent state of
IFF1, which may be used to restore IFF1 if the NMI handler returns by RETN
instruction.

Execution time for NMIs is unknown (?).

**RETN (return from NMI and restore IFF1)**

Intended to return from NMI and to restore the old IFF1 state (assuming the old
state was IFF1/IFF2 both set or both cleared).

**RETI (return from INT with external acknowledge)**

Intended to return from INT and to notify peripherals about completion of the
INT handler, the Z80 itself doesn't send any such acknowledge signal (instead,
peripherals like Z80-PIO or Z80-SIO must decode the databus during /M1 cycles,
and identify the opcode sequence EDh,4Fh as RETI). Aside from such external
handling, internally, RETI is exactly same as RETN, and, like RETN it does set
IFF1=IFF2 (though in case of RETI this is a dirt effect without practical use;
within INT handlers IFF1 and IFF2 are always both zero, or when EI was used
both set). Recommended methods to return from INT are: EI+RETI (when needing
the external acknowledge), or EI+RET (faster).

<a id="z80meaninglessandduplicatedopcodes"></a>

## Z80 Meaningless and Duplicated Opcodes

**Mirrored Instructions**

NEG  (ED44) is mirrored to ED4C,54,5C,64,6C,74,7C.

RETN (ED45) is mirrored to ED55,65,75.

RETI (ED4D) is mirrored to ED5D,6D,7D.

**Mirrored IM Instructions**

IM 0,X,1,2 (ED46,4E,56,5E) are mirrored to ED66,6E,76,7E.

Whereas IM X is an undocumented mirrored instruction itself which appears to be
identical to either IM 0 or IM 1 instruction (?).

**Duplicated LD HL Instructions**

LD (nn),HL (opcode 22NNNN) is mirrored to ED63NNNN.

LD HL,(nn) (opcode 2ANNNN) is mirrored to ED6BNNNN.

Unlike the other instructions in this chapter, these two opcodes are officially
documented. The clock/refresh cycles for the mirrored instructions are then
20/2 instead of 16/1 as for the native 8080 instructions.

**Mirrored BIT N,(ii+d) Instructions**

Unlike as for RES and SET, the BIT instruction does not support a third
operand, ie. DD or FD prefixes cannot be used on a BIT N,r instruction in order
to produce a BIT r,N,(ii+d) instruction. When attempting this, the 'r' operand
is ignored, and the resulting instruction is identical to BIT N,(ii+d).

Except that, not tested yet, maybe undocumented flags are then read from 'r'
instead of from ii+d(?).

**Non-Functional Opcodes**

The following opcodes behave much like the NOP instruction.

ED00-3F, ED77, ED7F, ED80-9F, EDA4-A7, EDAC-AF, EDB4-B7, EDBC-BF, EDC0-FF.

The execution time for these opcodes is 8 clock cycles, 2 refresh cycles.

Note that some of these opcodes appear to be used for additional instructions
by the R800 CPU in newer turbo R (MSX) models.

**Ignored DD and FD Prefixes**

In some cases, DD-prefixes (IX) and FD-prefixes (IY) may be ignored by the CPU.
This happens when using one (or more) of the above prefixes prior to
instructions that already contain an ED, DD, or FD prefix, or prior to any
instructions that do not support IX, IY, IXL, IXH, IYL, IYH operands. In such
cases, 4 clock cycles and 1 refresh cycle are counted for each ignored prefix
byte.

<a id="z80garbageinflagregister"></a>

## Z80 Garbage in Flag Register

**Nocash Z80-flags description**

This chapter describes the undocumented Z80 flags (bit 3 and 5 of the Flags
Register), these flags are affected by ALL instructions that modify one or more
of the normal flags - all OTHER instructions do NOT affect the undocumented
flags.

For some instructions, the content of some flags has been officially documented
as 'destroyed', indicating that the flags contain garbage, the exact garbage
calculation for these instructions will be described here also.

All information below just for curiosity. Keep in mind that Z80 compatible CPUs
(or emulators) may not supply identical results, so that it wouldn't be a good
idea to use these flags in any programs (not that they could be very useful
anyways).

**Normal Behaviour for Undocumented Flags**

In most cases, undocumented flags are copied from the Bit 3 and Bit 5 of the
result byte. That is "A AND 28h" for:

```text
  RLD; CPL; RLCA; RLA; LD A,I; ADD OP; ADC OP; XOR OP; AND OP;
  RRD; NEG; RRCA; RRA; LD A,R; SUB OP; SBC OP; OR OP ; DAA.
```

When other operands than A may be modified, "OP AND 28h" for:

```text
  RLC OP; RL OP; SLA OP; SLL OP; INC OP; IN OP,(C);
  RRC OP; RR OP; SRA OP; SRL OP; DEC OP
```

For 16bit instructions flags are calculated as "RR AND 2800h":

```text
  ADD RR,XX; ADC RR,XX; SBC RR,XX.
```

**Slightly Special Undocumented Flags**

For 'CP OP' flags are calculated as "OP AND 28h", that is the unmodified
operand, and NOT the internally calculated result of the comparision.

For 'SCF' and 'CCF' flags are calculated as "(A OR F) AND 28h", ie. the flags
remain set if they have been previously set.

For 'BIT N,R' flags are calculated as "OP AND 28h", additionally the P-Flag is
set to the same value than the Z-Flag (ie. the Parity of "OP AND MASK"), and
the S-flag is set to "OP AND MASK AND 80h".

**Fatal MEMPTR Undocumented Flags**

For 'BIT N,(HL)' the P- and S-flags are set as for BIT N,R, but the
undocumented flags are calculated as "MEMPTR AND 2800h", for more info about
MEMPTR read on below.

The same applies to 'BIT N,(ii+d)', but the result is less unpredictable
because the instruction sets MEMPTR=ii+d, so that undocumented flags are
"&lt;ii+d&gt; AND 2800h".

**Memory Block Command Undocumented Flags**

For LDI, LDD, LDIR, LDDR, undocumented flags are "((A+DATA) AND 08h) +
((A+DATA) AND 02h)*10h".

For CPI, CPD, CPIR, CPDR, undocumented flags are "((A-DATA-FLG_H) AND 08h) +
((A-DATA-FLG_H) AND 02h)*10h", whereas the CPU first calculates A-DATA, and
then internally subtracts the resulting H-flag from the result.

**Chaotic I/O Block Command Flags**

The INI, IND, INIR, INDR, OUTI, OUTD, OTIR, OTDR instructions are doing a lot
of obscure things, to simplify the description a placeholder called DUMMY is
used in the formulas.

```text
  DUMMY = "REG_C+DATA+1"    ;for INI/INIR
  DUMMY = "REG_C+DATA-1"    ;for IND/INDR
  DUMMY = "REG_L+DATA"      ;for OUTI,OUTD,OTIR,OTDR
  FLG_C = Carry  of above "DUMMY" calculation
  FLG_H = Carry  of above "DUMMY" calculation (same as FLG_C)
  FLG_N = Sign   of "DATA"
  FLG_P = Parity of "REG_B XOR (DUMMY AND 07h)"
  FLG_S = Sign   of "REG_B"
  UNDOC = Bit3,5 of "REG_B AND 28h"
```

The above registers L and B are meant to contain the new values which are
already incremented/decremented by the instruction.

Note that the official docs mis-described the N-Flag as set, and the C-Flag as
not affected.

**DAA Flags**

Addition (if N was 0):

```text
  FLG_H = (OLD_A AND 0Fh) > 09h
  FLG_C = Carry of result
```

Subtraction (if N was 1):

```text
  FLG_H = (NEW_A AND 0Fh) > 09h
  FLG_C = OLD_CARRY OR (OLD_A>99h)
```

For both addition and subtraction, N remains unmodified, and S, Z, P contain
"Sign", Zero, and Parity of result (A). Undocumented flags are set to (A AND
28h) as normal.

**Mis-documented Flags**

For all XOR/OR: H=N=C=0, and for all AND: H=1, N=C=0, unlike described else in
Z80 docs. Also note C,N flag description bug for I/O block commands (see
above).

**Internal MEMPTR Register**

This is an internal Z80 register, modified by some instructions, and usually
completely hidden to the user, except that Bit 11 and Bit 13 can be read out at
a later time by BIT N,(HL) instructions.

The following list specifies the resulting content of the MEMPTR register
caused by the respective instructions.

```text
  Content Instruction
  A*100h  LD (xx),A               ;xx=BC,DE,nn
  xx+1    LD A,(xx)               ;xx=BC,DE,nn
  nn+1    LD (nn),rr; LD rr,(nn)  ;rr=BC,DE,HL,IX,IY
  rr      EX (SP),rr              ;rr=HL,IX,IY (MEMPTR=new value of rr)
  rr+1    ADD/ADC/SBC rr,xx       ;rr=HL,IX,IY (MEMPTR=old value of rr+1)
  HL+1    RLD and RRD
  dest    JP nn; CALL nn; JR nn   ;dest=nn
  dest    JP f,nn; CALL f,nn      ;regardless of condition true/false
  dest    RET; RETI; RETN         ;dest=value read from (sp)
  dest    RET f; JR f,nn; DJNZ nn ;only if condition=true
  00XX    RST n
  adr+1   IN A,(n)                ;adr=A*100h+n, memptr=A*100h+n+1
  bc+1    IN r,(BC); OUT (BC),r   ;adr=bc
  ii+d    All instructions with operand (ii+d)
```

Also the following might or might not affect MEMPTR, not tested yet:

```text
  OUT (N),A and block commands LDXX, CPXX, INXX, OUTXX
  and probably interrupts in IM 0, 1, 2
```

All other commands do not affect the MEMPTR register - this includes all
instructions with operand (HL), all PUSH and POP instructions, not executed
conditionals JR f,d, DJNZ d, RET f (ie. with condition=false), and the JP
HL/IX/IY jump instructions.

<a id="z80compatibility"></a>

## Z80 Compatibility

The Z80 CPU is (almost) fully backwards compatible to older 8080 and 8085 CPUs.

**Instruction Format**

The Z80 syntax simplifies the chaotic 8080/8085 syntax. For example, Z80 uses
the command "LD" for all load instructions, 8080/8085 used various different
commands depending on whether the operands are 8bit registers, 16bit registers,
memory pointers, and/or an immediates. However, these changes apply to the
source code only - the generated binary code is identical for both CPUs.

**Parity/Overflow Flag**

The Z80 CPU uses Bit 2 of the flag register as Overflow flag for arithmetic
instructions, and as Parity flag for other instructions. 8080/8085 CPUs are
always using this bit as Parity flag for both arithmetic and non-arithmetic
instructions.

**Z80 Specific Instructions**

The following instructions are available for Z80 CPUs only, but not for older
8080/8085 CPUs:

All CB-prefixed opcodes (most Shift/Rotate, all BIT/SET/RES commands).

All ED-prefixed opcodes (various instructions, and all block commands).

All DD/FD-prefixed opcodes (registers IX and IY).

As well as DJNZ nn; JR nn; JR f,nn; EX AF,AF; and EXX.

**8085 Specific Instructions**

The 8085 instruction set includes two specific opcodes in addition to the 8080
instruction set, used to control 8085-specifc interrupts and SID and SOD
input/output signals. These opcodes, RIM (20h) and SIM (30h), are not supported
by Z80/8080 CPUs.

**Z80 vs Z80A**

Both Z80 and Z80A are including the same instruction set, the only difference
is the supported clock frequency (Z80 = max 2.5MHz, Z80A = max 4MHz).

**NEC-780 vs Zilog-Z80**

These CPUs are apparently fully compatible to each other, including for
undocumented flags and undocumented opcodes.

<a id="z80pinouts"></a>

## Z80 Pin-Outs

```text
         _____   _____
        |     |_|     |
    A11 |1          40| A10
    A12 |2          39| A9
    A13 |3          38| A8
    A14 |4          37| A7
    A15 |5          36| A6
    CLK |6          35| A5
     D4 |7          34| A4
     D3 |8          33| A3
     D5 |9          32| A2
     D6 |10   Z80   31| A1
    VCC |11   CPU   30| A0
     D2 |12         29| GND
     D7 |13         28| /RFSH
     D0 |14         27| /M1
     D1 |15         26| /RST
   /INT |16         25| /BUSRQ
   /NMI |17         24| /WAIT
  /HALT |18         23| /BUSAK
  /MREQ |19         22| /WR
  /IORQ |20         21| /RD
        |_____________|
```

<a id="z80localusage"></a>

## Z80 Local Usage

**Nintendo Super System (Z80)**

Clocked at 4.000MHz.

NMIs are used for something (probably Vblank or Vsync or so). Normal interrupts
seem to be unused. There is MAYBE no watchdog hardware (but the BIOS is using a
software-based watchdog; namely, it's misusing the "I" register as watchdog
timer; decreased by NMI handler). ALTHOUGH, like the PC10, it might
ADDITIONALLY have a hardware watchdog...?

**Super Famicom Box (HD64180)**

Clocked at by a 9.216MHz oscillator, ie. the HD64180 is internally clocked at
PHI=4.608MHz.

<a id="hd64180"></a>

## HD64180

The HD64180/Z180 are extended Z80 CPUs, the HD64180 was originally made by
Hitachi, and later adopted as Z180 by Zilog.

[HD64180 Internal I/O Map](#hd64180-internal-io-map)

[HD64180 New Opcodes (Z80 Extension)](#hd64180-new-opcodes-z80-extension)

[HD64180 Serial I/O Ports (ASCI and CSI/O)](#hd64180-serial-io-ports-asci-and-csio)

[HD64180 Timers (PRT and FRC)](#hd64180-timers-prt-and-frc)

[HD64180 Direct Memory Access (DMA)](#hd64180-direct-memory-access-dma)

[HD64180 Interrupts](#hd64180-interrupts)

[HD64180 Memory Mapping and Control](#hd64180-memory-mapping-and-control)

[HD64180 Extensions](#hd64180-extensions)

The system clock (PHI) is half the frequency of the crystal. Supported PHI
values are 6.144MHz, 4.608MHz, 3.072MHz (these values allow to program ASCI
channels to valid RS232 baudrates).

<a id="hd64180internaliomap"></a>

### HD64180 Internal I/O Map

**HD64180 Internal Registers**

Internal I/O Ports are initially mapped to Port 0000h..003Fh (but can be
reassigned to 0040h..007Fh, 0080h..00BFh, or 00C0h..00FFh via ICR register).

```text
  Port Name      Expl.                                    (On Reset)
  00h  CNTLA0    ASCI Channel 0 Control Reg A             (10h, bit3=var)
  01h  CNTLA1    ASCI Channel 1 Control Reg A             (10h, bit3/bit4=var)
  02h  CNTLB0    ASCI Channel 0 Control Reg B             (07h, bit7/bit5=var)
  03h  CNTLB1    ASCI Channel 1 Control Reg B             (07h, bit7=var)
  04h  STAT0     ASCI Channel 0 Status Register           (00h, bit2/bit1=var)
  05h  STAT1     ASCI Channel 1 Status Register           (02h)
  06h  TDR0      ASCI Channel 0 Transmit Data Register
  07h  TDR1      ASCI Channel 1 Transmit Data Register
  08h  RDR0      ASCI Channel 0 Receive Data Register
  09h  RDR1      ASCI Channel 1 Receive Data Register
  0Ah  CNTR      CSI/O Control Register                   (0Fh)
  0Bh  TRDR      CSI/O Transmit/Receive Data Register
  0Ch  TMDR0L    Timer 0 Counter "Data" Register, Bit0-7  (FFh)
  0Dh  TMDR0H    Timer 0 Counter "Data" Register, Bit8-15 (FFh)
  0Eh  RLDR0L    Timer 0 Reload Register, Bit0-7          (FFh)
  0Fh  RLDR0H    Timer 0 Reload Register, Bit8-15         (FFh)
  10h  TCR       Timer Control Register                   (00h)
  11h-13h        Reserved
   12h  ASEXT0    ASCI Channel 0 Extension Control Reg ;\Z8S180/Z8L180 only
   13h  ASEXT1    ASCI Channel 0 Extension Control Reg ;/(not Z80180/HD64180)
  14h  TMDR1L    Timer 1 Counter "Data" Register, Bit0-7  (FFh)
  15h  TMDR1H    Timer 1 Counter "Data" Register, Bit8-15 (FFh)
  16h  RLDR1L    Timer 1 Reload Register, Bit0-7          (FFh)
  17h  RLDR1H    Timer 1 Reload Register, Bit8-15         (FFh)
  18h  FRC       Free Running Counter                     (FFh)
  19h-1Fh        Reserved
   1Ah  ASTC0L    ASCI Channel 0 Time Constant, Bit0-7  ;\
   1Bh  ASTC0H    ASCI Channel 0 Time Constant, Bit8-15 ; Z8S180/Z8L180 only
   1Ch  ASTC1L    ASCI Channel 1 Time Constant, Bit0-7  ; (not Z80180/HD64180)
   1Dh  ASTC1H    ASCI Channel 1 Time Constant, Bit8-15 ;
   1Eh  CMR       Clock Multiplier Register             ;
   1Fh  CCR       CPU Control Register                  ;/
  20h  SAR0L     DMA Channel 0 Source Address, Bit0-7 (Memory or I/O)
  21h  SAR0H     DMA Channel 0 Source Address, Bit8-15 (Memory or I/O)
  22h  SAR0B     DMA Channel 0 Source Address, Bit16-19 (Memory or DRQ)
  23h  DAR0L     DMA Channel 0 Destination Address, Bit0-7 (Memory or I/O)
  24h  DAR0H     DMA Channel 0 Destination Address, Bit8-15 (Memory or I/O)
  25h  DAR0B     DMA Channel 0 Destination Address, Bit16-19 (Memory or DRQ)
  26h  BCR0L     DMA Channel 0 Byte Count Register, Bit0-7
  27h  BCR0H     DMA Channel 0 Byte Count Register, Bit8-15
  28h  MAR1L     DMA Channel 1 Memory Address, Bit0-7 (Source or Dest)
  29h  MAR1H     DMA Channel 1 Memory Address, Bit8-15 (Source or Dest)
  2Ah  MAR1B     DMA Channel 1 Memory Address, Bit16-19 (Source or Dest)
  2Bh  IAR1L     DMA Channel 1 I/O Address, Bit0-7 (Dest or Source)
  2Ch  IAR1H     DMA Channel 1 I/O Address, Bit8-15 (Dest or Source)
   2Dh            Reserved ;IAR1B on Z8S180/Z8L180 (not Z80180/HD64180)
  2Eh  BCR1L     DMA Channel 1 Byte Count Register, Bit0-7
  2Fh  BCR1H     DMA Channel 1 Byte Count Register, Bit8-15
  30h  DSTAT     DMA "Status" Register                   (32h on Reset)
  31h  DMODE     DMA Mode Register                       (C1h on Reset)
  32h  DCNTL     DMA/WAIT Control Register               (F0h on Reset)
  33h  IL        Interrupt Vector Low Register           (00h on Reset)
  34h  ITC       INT/TRAP Control Register               (39h on Reset)
  35h            Reserved
  36h  RCR       Refresh Control Register                (FCh on Reset)
  37h            Reserved
  38h  CBR       MMU Common Base Register (Common Area 1)(00h on Reset)
  39h  BBR       MMU Bank Base Register (Bank Area)      (00h on Reset)
  3Ah  CBAR      MMU Common/Bank Area Register           (F0h on Reset)
  3Bh-3Dh        Reserved
  3Eh  OMCR      Operation Mode, Z180 only (not HD64180) (FFh on Reset)
  3Fh  ICR       I/O Control Register                    (1Fh on Reset)
```

<a id="hd64180newopcodesz80extension"></a>

### HD64180 New Opcodes (Z80 Extension)

**New HD64180 Opcodes**

```text
  ED 00 nn  IN0 B,(nn)    ED 01 nn  OUT0 (nn),B    ED 04     TST B
  ED 08 nn  IN0 C,(nn)    ED 09 nn  OUT0 (nn),C    ED 0C     TST C
  ED 10 nn  IN0 D,(nn)    ED 11 nn  OUT0 (nn),D    ED 14     TST D
  ED 18 nn  IN0 E,(nn)    ED 19 nn  OUT0 (nn),E    ED 1C     TST E
  ED 20 nn  IN0 H,(nn)    ED 21 nn  OUT0 (nn),H    ED 24     TST H
  ED 28 nn  IN0 L,(nn)    ED 29 nn  OUT0 (nn),L    ED 2C     TST L
  ED 30 nn  IN0 (nn)                               ED 34     TST (HL)
  ED 38 nn  IN0 A,(nn)    ED 39 nn  OUT0 (nn),A    ED 3C     TST A
  ED 4C     MULT BC       ED 83     OTIM           ED 64 nn  TST nn
  ED 5C     MULT DE       ED 8B     OTDM           ED 70     IN (C)
  ED 6C     MULT HL       ED 93     OTIMR          ED 74 nn  TSTIO nn
  ED 7C     MULT SP       ED 9B     OTDMR          ED 76     SLP
```

On a real Z80, ED-4C/5C/6C/7C and ED-64/74 have been mirrors of NEG.

On a real Z80, ED-70 did the same (but was undocumented).

On a real Z80, ED-76 has been mirror of IM 2.

On a real Z80, ED-00..3F and ED-80..9F have acted as NOP.

**Notes**

IN0/OUT0/OTxMx same as IN/OUT/OTxx but with I/O-address bit8-15 forced 00h.

TST op: Test A,op.  ;non-destructive AND (only flags changed)

TSTIO nn: Test Port[C],nn  ;\hitachi lists BOTH definitions (page 75)

TSTIO nn: Test Port[nn],A  ;/zilog also lists BOTH definitions (page 173,174)

TSTIO nn: Test Port[C],nn  ;&lt;-- this is reportedly the correct definition

MLT xy: xy=x*y   ;unsigned multiply (flags=unchanged)

SLP (SLEEP) stops internal clock (including stopping DRAM refresh and DMAC).

IOSTOP: stops ASCI, CSI/O, PRT.

**Z80 incompatible opcodes (according to Zilog's Z180 Application Note)**

```text
  Opcode    Z80                     Z180
  DAA       Checks Cy and A>99h     Checks Cy only? (when N=1)
  RLD/RRD   Sets flags for A        Sets flags for [HL]
```

**Opcode Execution Time**

Some opcodes are slightly faster as on real Z80. For example, some (not all)
4-cycle Z80 opcodes take only 3-cyles on HD64180.

**Undefined Opcodes**

On the HD64180, undefined opcodes are causing a TRAP exception (this feature
cannot be disabled). So, while the real Z80 does have some useful (and some
useless) undocumented opcodes, none (?) of these is working on HD64180 (except
for the now-official ED-70 opcode).

The HD64180 datasheet doesn't list "SLL" as valid opcode.

The HD64180 datasheet doesn't list the "SET-and-LD" or "RES-and-LD" opcodes.

The HD64180 datasheet doesn't list opcodes with "IXL,IXH,IYL,IYH" operands,
however, it does mention existence of "IXL" here and there (however, that seems
to refer only to 16bit operations like "PUSH IX" (which do internally split
16bit IX into two 8bit units).

The HD64180 datasheet lists EX DE,HL with IX/IY-prefix as invalid.

NEWER INFO:

The HD64180 is actually trapping all undocumented opcodes, even those that are
more or less commonly used on Z80 CPUs, ie. the HD64180 doesn't support
accessing IX/IY 16bit registers as 8bit fragments (IXH,IXL,IYH,IYL), doesn't
support "SLL" opcode, nor useless opcode mirrors (like alternate
NEG/IM/RETN/RETI/NOP mirrors).

<a id="hd64180serialioportsasciandcsio"></a>

### HD64180 Serial I/O Ports (ASCI and CSI/O)

Asynchronous Serial Communication Interfaces (ASCI) and Clocked Serial I/O
(CSI/I)

XXX pg 51... 56

**00h - CNTLA0 - ASCI Channel 0 Control Reg A (10h on Reset, bit3=var)**

**01h - CNTLA1 - ASCI Channel 1 Control Reg A (10h on Reset, bit3/bit4=var)**

```text
  7   MPE   RX Multi Processor Filter (0=RX all bytes, 1=RX flagged bytes)
  6   RE    RX Receiver Enable        (0=Disable, 1=Enable)
  5   TE    TX Transmitter Enable     (0=Disable, 1=Enable)
  4   /RTS0 for Ch0: Request to Send output (0=Low, 1=High) (/RTS pin)
      CKL1D for Ch1: CKA1 Clock Disable (CKA1/TEND pin)
  3   MPBR  Read:  RX Multi Processor Bit (Received Flag-Bit)
      EFR   Write: RX Error Flag Reset (0=Reset OVRN,PE,FE-Flags, 1=No Change)
  2   MOD2  Number of Data bits   (0=7bit, 1=8bit)
  1   MOD1  Number of Parity bits (0=None, 1=1bit) (only if MP=0)
  0   MOD0  Number of Stop bits   (0=1bit, 1=2bit)
```

**02h - CNTLB0 - ASCI Channel 0 Control Reg B (07h on Reset, bit7/bit5=var)**

**03h - CNTLB1 - ASCI Channel 1 Control Reg B (07h on Reset, bit7=var)**

```text
  7   MPBT  TX Multi Processor Bit (Flag-Bit to be Transmitted)
  6   MP    Multiprocessor Mode (0=Off/Normal, 1=Add Flag-bit to all bytes)
  5   CTS   Read: /CTS-pin (0=Low, 1=High),
      PS    Write: Prescaler (0=Div10, 1=Div30)
  4   PEO   Parity Even/Odd (0=Even, 1=Odd) (ignored when MOD1=0 or MP=1)
  3   DR    Divide Ratio (0=Div16, 1=Div64)
  2-0 SS    Speed Select (0..6: "(PHI SHR N)", 7=External clock)
```

The baudrate is "SS div PS div DR" (or "External_Clock div DR").

**04h - STAT0 - ASCI Channel 0 Status Register (00h on Reset, bit2/bit1=var)**

**05h - STAT1 - ASCI Channel 1 Status Register (02h on Reset)**

```text
  7   RDRF  RX Receive Data Register Full (0=No, 1=Yes)             (R)
  6   OVRN  RX Overrun Error (0=Okay, 1=Byte received while RDRF=1) (R)
  5   PE    RX Parity Error  (0=Okay, 1=Wrong Parity Bit)           (R)
  4   FE    RX Framing Error (0=Okay, 1=Wrop Stop Bit)              (R)
  3   RIE   RX Receive Interrupt Enable                 (R/W)
  2   /DCD0 For Ch0: Data Carrier Detect (/DCD pin)     (R)
      CTS1E For Ch1: CTS input enable (/CTS pin)        (R/W)
  1   TDRE  TX Transmit Data Register Empty             (R)
  0   TIE   TX Transmit Interrupt Enable                (R/W)
```

Note: RDRD/TDRE can be used as DRQ signal for DMA channel 0.

**06h - TDR0 - ASCI Channel 0 Transmit Data Register**

**07h - TDR1 - ASCI Channel 1 Transmit Data Register**

**08h - RDR0 - ASCI Channel 0 Receive Data Register**

**09h - RDR1 - ASCI Channel 1 Receive Data Register**

```text
  7-0  Data
```

The hardware can hold one byte in the data register (plus one byte currently
processed in a separate shift register).

**0Ah - CNTR - CSI/O Control Register (0Fh on Reset)**

```text
  7   EF    End Flag, completion of Receive/Transmit (0=No/Busy, 1=Yes/Ready)
  6   EIE   End Interrupt Enable (0=Disable, 1=Enable)
  5   RE    Receive Enable  (0=Off/Ready, 1=Start/Busy)
  4   TE    Transmit Enable (0=Off/Ready, 1=Start/Busy)
  3   -     Unused (should be all-ones)
  2-0 SS    Speed Select (0..6: "(20 shl N) clks per bit", 7=External clock)
```

The select "speed" is output on CKS pin (or input from CKS pin when selecting
External clock). Bit7 is read-only (cleared when reading/writing TRDR).

**0Bh - TRDR - CSI/O Transmit/Receive Data Register**

```text
  7-0  Data (8bit) (called TRDR by Hitachi, called TRD by Zilog)
```

Data is output on TXS pin, and input on RXS pin (both LSB first). Despite of
the separate pins, one may NOT set RE and TE simultanoulsy (for whatever
reason... or maybe it's meant to WORK ONLY if RX and TX are STARTED
simultaneously). The RXS pin is also used as /CTS1 (for ASCI channel 1).

**ASCI Multi Processor "Network" Feature**

This feature allows to share the serial bus by multiple computers. Each byte is
transferred with a "MPB" Multi Processor Flag Bit (located between Data and
Stop bits) (Parity is forcefully disabled in Multi Processor Mode).

Assume broadcasting "Header+Data" Packets (with "Header" bytes flagged as
MPB=1, and "Data" as MPB=0): The RX-Filter can select to receive only "Header"
bytes, and, if the receiver treats itself to be addressed by the header, it can
change the filter setting depending on whether it wants to receive/skip the
following "Data" bytes.

<a id="hd64180timersprtandfrc"></a>

### HD64180 Timers (PRT and FRC)

Programmable Reload Timers (PRT) and Free Running Counter (FRC)

**10h - TCR - Timer Control Register (00h on Reset)**

```text
  7   TIF1  Timer 1 Interrupt Flag (0=No, 1=Yes/Decrement reached 0000h) (R)
  6   TIF0  Timer 0 Interrupt Flag (0=No, 1=Yes/Decrement reached 0000h) (R)
  5   TIE1  Timer 1 Interrupt Enable (0=Disable, 1=Enable)
  4   TIE0  Timer 0 Interrupt Enable (0=Disable, 1=Enable)
  3-2 TOC   Timer 1 Output Control to A18-Pin (0=A18, 1=Toggled, 2=Low, 3=High)
  1   TDIE1 Timer 1 Decrement Enable (0=Stop, 1=Decrement; once every 20 clks)
  0   TDIE0 Timer 0 Decrement Enable (0=Stop, 1=Decrement; once every 20 clks)
```

TIF1 is reset when reading TCR or TMDR1L or TMDR1H.

TIF0 is reset when reading TCR or TMDR0L or TMDR0H.

The TOC bits control the A18/TOUT pin (it can be either A18 address line, or
forced to Low or High, or "toggled": that is, inverted when TMDR1 decremts to
0.

**0Ch - TMDR0L - Timer 0 Counter "Data" Register, Bit0-7 (FFh on Reset)**

**0Dh - TMDR0H - Timer 0 Counter "Data" Register, Bit8-15 (FFh on Reset)**

**0Eh - RLDR0L - Timer 0 Reload Register, Bit0-7 (FFh on Reset)**

**0Fh - RLDR0H - Timer 0 Reload Register, Bit8-15 (FFh on Reset)**

Timer 0 counter/reload values. The counter is decremented once every 20 clks,
and triggers IRQ and gets reloaded when reaching 0000h. Reading TMDR0L returns
current timer LSB, and latches current timer MSB. Reading TMDR0H returns that
LATCHED timer MSB. Accordingly reads should be always done in order LSB, MSB.

If the timer is stopped TMDR0L/TMDR0H can be written (and read) in any order.

**14h - TMDR1L - Timer 1 Counter "Data" Register, Bit0-7 (FFh on Reset)**

**15h - TMDR1H - Timer 1 Counter "Data" Register, Bit8-15 (FFh on Reset)**

**16h - RLDR1L - Timer 1 Reload Register, Bit0-7 (FFh on Reset)**

**17h - RLDR1H - Timer 1 Reload Register, Bit8-15 (FFh on Reset)**

Timer 1 counter/reload values. Same as for Timer 0 (see above).

**18h - FRC - Free Running Counter (FFh on Reset)**

```text
  7-0 FRC   Free Running Counter (decremented every 10 clks)
```

This register should be read-only, writing to FRC may mess up DRAM refresh,
ASCI and CSI/O baud rates.

<a id="hd64180directmemoryaccessdma"></a>

### HD64180 Direct Memory Access (DMA)

**20h - SAR0L - DMA Channel 0 Source Address, Bit0-7 (Memory or I/O)**

**21h - SAR0H - DMA Channel 0 Source Address, Bit8-15 (Memory or I/O)**

**22h - SAR0B - DMA Channel 0 Source Address, Bit16-19 (Memory or DRQ)**

**23h - DAR0L - DMA Channel 0 Destination Address, Bit0-7 (Memory or I/O)**

**24h - DAR0H - DMA Channel 0 Destination Address, Bit8-15 (Memory or I/O)**

**25h - DAR0B - DMA Channel 0 Destination Address, Bit16-19 (Memory or DRQ)**

**26h - BCR0L - DMA Channel 0 Byte Count Register, Bit0-7**

**27h - BCR0H - DMA Channel 0 Byte Count Register, Bit8-15**

DMA Channel 1 Source/Dest/Len. Direction can be Memory-to-Memory,
Memory-to-I/O, I/O-to-Memory, or I/O-to-I/O, Memory-Address can be Fixed,
Incrementing, or Decrementing, I/O-Address is Fixed (see DMODE Register).

For I/O transfers, Bit16-17 of SAR/DAR are selecting the DRQ type:

```text
  00h DRQ by /DREQ0-Pin (normal case)
  01h DRQ by ASCI Channel 0 (RDRF-Bit for Source, or TDRE-Bit for Dest)
  02h DRQ by ASCI Channel 1 (RDRF-Bit for Source, or TDRE-Bit for Dest)
  03h Reserved
```

Memory-to-Memory DMA clock can be selected in MMOD bit ("Burst" pauses CPU
until transfer is completed, "Cycle Steal" keeps the CPU running at roughly
half-speed during DMA).

**28h - MAR1L - DMA Channel 1 Memory Address, Bit0-7 (Source or Dest)**

**29h - MAR1H - DMA Channel 1 Memory Address, Bit8-15 (Source or Dest)**

**2Ah - MAR1B - DMA Channel 1 Memory Address, Bit16-19 (Source or Dest)**

**2Bh - IAR1L - DMA Channel 1 I/O Address, Bit0-7 (Dest or Source)**

**2Ch - IAR1H - DMA Channel 1 I/O Address, Bit8-15 (Dest or Source)**

**2Eh - BCR1L - DMA Channel 1 Byte Count Register, Bit0-7**

**2Fh - BCR1H - DMA Channel 1 Byte Count Register, Bit8-15**

DMA Channel 1 Source/Dest/Len. Direction can be Memory-to-I/O or I/O-to-Memory,
Memory-Address can be Incrementing or Decrementing, I/O-Address is Fixed (see
DCNTL Register). DRQ is taken from /DREQ1-Pin.

**30h - DSTAT - DMA "Status" Register (32h on Reset)**

```text
  7   DE1   DMA Channel 1 Enable (0=Ready, 1=Start/Busy)
  6   DE0   DMA Channel 0 Enable (0=Ready, 1=Start/Busy)
  5   /DWE1 Writing to DE1 (0=Allowed, 1=Ignored, keep Bit7 unchanged)
  4   /DWE0 Writing to DE0 (0=Allowed, 1=Ignored, keep Bit6 unchanged)
  3   DIE1  DMA Channel 1 Interrupt Enable (0=Disable, 1=Enable)
  2   DIE0  DMA Channel 0 Interrupt Enable (0=Disable, 1=Enable)
  1   -     Unused (should be all-ones)
  0   DME   DMA Main Enable
```

**31h - DMODE - DMA Mode Register (E1h on Reset)**

```text
  7-6 -     Unused (should be all-ones)
  5-4 DM    DMA Channel 0 Dest (0=Mem/Inc, 1=Mem/Dec, 2=Mem/Fix, 3=IO/Fix)
  3-2 SM    DMA Channel 0 Src  (0=Mem/Inc, 1=Mem/Dec, 2=Mem/Fix, 3=IO/Fix)
  1   MMOD  DMA Channel 0 Mem-to-Mem Mode (0=Cycle Steal, 1=Burst)
  0   -     Unused (should be all-ones)
```

**32h - DCNTL - DMA/WAIT Control Register (F0h on Reset)**

```text
  7-6 MW   Memory Waitstates (0..3 = 0..3)
  5-4 IW   External I/O Waitstates (0..3 = 1..4) and /INT/LIR and more XXX
  3   DMS1 DMA Channel 1 Sense /DREQ1-Pin (0=Sense Level, 1=Sense Edge)
  2   DMS0 DMA Channel 0 Sense /DREQ0-Pin (0=Sense Level, 1=Sense Edge)
  1   DIM1 DMA Channel 1 Src-to-Dest Direction (0=Mem-to-I/O, 1=I/O-to-Mem)
  0   DIM0 DMA Channel 1 Memory-Step Direction (0=Increment, 1=Decrement)
```

**Note**

On some chip versions address bus is only 19bits, namely that does apply on
64pin chips (68pin/80pin chips should have 20bits). Regardless of the pin-outs,
the extra bit might (maybe) exist internally on newer 64pin chips(?)

<a id="hd64180interrupts"></a>

### HD64180 Interrupts

**Interrupts**

```text
  Prio                                     Vector
  0  /RES  Reset (non-maskable)            (PC=0000h, with TRAP=0 in ITC)
  1  TRAP  Undefined Opcode (non-maskable) (PC=0000h, with TRAP=1 in ITC)
  2  /NMI  Non-maskable Interrupt          (PC=0066h)
  3  /INT0 Maskable Interrupt Level 0      (PC=[I*100h+databus], or PC=0038h)
  4  /INT1 Maskable Interrupt Level 1      (PC=[I*100h+IL*20h+00h])
  5  /INT2 Maskable Interrupt Level 2      (PC=[I*100h+IL*20h+02h])
  6  Timer 0                               (PC=[I*100h+IL*20h+04h])
  7  Timer 1                               (PC=[I*100h+IL*20h+06h])
  8  DMA Channel 0 Ready                   (PC=[I*100h+IL*20h+08h])
  9  DMA Channel 1 Ready                   (PC=[I*100h+IL*20h+0Ah])
  10 Clocked Serial I/O Port (CSI/O)       (PC=[I*100h+IL*20h+0Ch])
  11 Asynchronous SCI channel 0            (PC=[I*100h+IL*20h+0Eh])
  12 Asynchronous SCI channel 1            (PC=[I*100h+IL*20h+10h])
  Below whatever only (not HD64180 and not Z180)
  ?  Input Capture                         (PC=[I*100h+IL*20h+10h])
  ?  Output Compare                        (PC=[I*100h+IL*20h+12h])
  ?  Timer Overflow                        (PC=[I*100h+IL*20h+16h])
```

Note: "I" is a CPU-register (set via MOV I,A opcode). "IL" is new I/O port (set
via OUT opcode). /INT0 works same as on real Z80 (and depends on mode set via
IM 0/1/2 opcodes).

**33h - IL - Interrupt Vector Low Register (00h on Reset)**

```text
  7-5 IL   Bit7-5 of IM 2 Interrupt Vector Table Address
  4-0 -    Unused (should be zero)
```

**34h - ITC - INT/TRAP Control Register (39h on Reset)**

```text
  7   TRAP Undefined Opcode occurred (0=No, 1=Yes)
  6   UFO  Addr of Undef Opcode (aka Undefined Fetch Object) (0=PC-1, 1=PC-2)
  5-3 -    Unused (should be all-ones)
  2   ITE2 Interrupt /INT2 Enable (0=Disable, 1=Enable)
  1   ITE1 Interrupt /INT1 Enable (0=Disable, 1=Enable)
  0   ITE0 Interrupt /INT0 Enable (0=Disable, 1=Enable)
```

TRAP gets set upon Undefined Opcodes (TRAP and RESET are both using vector
0000h, the TRAP bit allows to sense if the vector was called by Reset or Undef
Opcode). The TRAP bit can be cleared by software by writing "0" to it (however,
software cannot write "1" to it).

<a id="hd64180memorymappingandcontrol"></a>

### HD64180 Memory Mapping and Control

**Memory Managment Unit (MMU)**

The Memory Managment Unit translates "virtual" 16bit CPU memory addresses to
"physical" 19bit address bus.

```text
  0000h -------> +--------------------+
                 | Common Area 0      | Phys19bit = Virt16bit + 00000h
  BA*1000h ----> +--------------------+
                 | Bank Area          | Phys19bit = Virt16bit + BBR*1000h
  CA*1000h ----> +--------------------+
                 | Common Area 1      | Phys19bit = Virt16bit + CBR*1000h
  FFFFh -------> +--------------------+
```

The 16bit CPU address space is divided into three areas (of which, the first
two areas can be 0 bytes in size: BA=0 disables Common Area 0, CA=BA disables
Bank Area).

**38h - CBR - MMU Common Base Register (Common Area 1) (00h on Reset)**

**39h - BBR - MMU Bank Base Register (Bank Area) (00h on Reset)**

```text
  7   Unused (should be zero) (but, used on chips with 20bit address bus)
  0-6 Base in 4K-units within "physical" 19bit 512K address space
```

**3Ah - CBAR - MMU Common/Bank Area Register (F0h on Reset)**

```text
  4-7 CA Start of Common Area 1 (End of Bank Area) (0Fh upon Reset)
  0-3 BA Start of Bank Area (End of Common Area 0) (00h upon Reset)
```

This is in 4K-units within the "virtual" 16bit 64K address space. Results on
CA&lt;BA are undefined.

**36h - RCR - Refresh Control Register (FCh on Reset)**

```text
  7   REFE  DRAM Refresh Enable (0=Disable, 1=Enable)
  6   REFW  DRAM Refresh Wait (0=Two Clocks, 1=Three Clks)
  5-2 -     Unused (should be all-ones)
  1-0 CYC   DRAM Refresh Interval (0..3=10,20,40,80 states)
```

Note: The hardware outputs an 8bit Refresh address on A0..A7. A classic Z80 did
output only 7bits, via using the CPU's "R" register (accessible with MOV A,R
and MOV R,A opcodes). The HD64180 does still increment lower 7bit of "R" in
same/similar fashion as on Z80, but, as far as I understand, without affecting
any bits of the actual refresh address (and vice-versa, without the
RCR-register settings affecting the way how "R" gets incremented).

**3Fh - ICR - I/O Control Register (1Fh on Reset)**

```text
  7-6 IOA   Base Address of Internal I/O ports (0..3=0000h,0040h,0080h,00C0h)
  5   IOSTP Stop Internal ASCI, CSI/O, PRT-Timers (0=No, 1=Pause)
  4-0 -     Unused (all ones on Reset)
```

Note: There is a "z180.h" file in the internet that claims that ICR "does not
move" (ie. that the "IOA" bits affect only Port 00h-3Eh, but not Port 3Fh
itself). Unknow where that info comes from, and unknown if it's correct (the
HD64180 and Z180 datasheets do not mention that effect).

**Memory Address Bus Width**

According to official specs, the address bus is 19bits wide (although the same
specs claim that it can address up to 1Mbyte, which would require 20bits,
unknown how that could work). [The 68pin and 80pin chip versions do actually
have a new A19 pin, which doesn't exist on 64pin chips]

Observe that A18 can be misused as Square-Wave output or as General-purpose
output (see Timer chapter); when using that feature, one should normally
connect only A0..A17 to memory address bus - otherwise, if A18 is wired to
memory, the feature would cause the physical address to be ANDed with 3FFFFh or
ORed with 40000h (this "allows" to some futher, but rather useless,
bankswitching).

**Waitstate Control**

For Memory and I/O Waitstate control, see DCNTL register (in DMA chapter).

**DMA**

DMA transfers can directly access 19bit addresses, without using the MMU.

**Unused Bits**

Several internal registers contain unused bits. According to the datasheet,
upon reset, these bits are set to all-ones, or all-zero (the setting varies
from register to register). Unknown if it's possible and/or allowed to change
these bits.

**Reserved Registers**

Registers 11h-13h, 19h-1Fh, 2Dh, 35h, 37h, 3Bh-3Eh are Reserved. Unknown if
it's possible and/or allowed to read/write these registers.

<a id="hd64180extensions"></a>

### HD64180 Extensions

Port 3Eh on Z8x180 only (not HD64180).

Port 12h..13h,1Ah..1Fh,2Dh on Z8S180/Z8L180 only (not Z80180/HD64180).

**3Eh - OMCR - Operation Mode Control - Z180 only (not HD64180) (FFh on Reset)**

```text
  7   M1E   /M1 Enable           (0=Z180, 1=HD64180; Problems with RETI)  (R/W)
  6   /M1TE /M1 Temporary Enable (0=Z180, 1=HD64180; Problems with Z80PIO)  (W)
  5   /IOC  I/O Compatibility    (0=Z180, 1=HD64180; Delayed falling /WR) (R/W)
  4-0 -     Unused (should be all-ones)
```

Allows to fix some signal &amp; timing glitches of the HD64180 (or to maintain
them for compatibility with HD64180 based designs).

**12h - ASEXT0 - ASCI Channel 0 Extension Control Reg 0 (00h on Reset)**

**13h - ASEXT1 - ASCI Channel 1 Extension Control Reg 1 (00h on Reset)**

```text
  7   RDRF Interrupt Inhibit
  6   DCD0 Disable    ;\ASCI Channel 0 only (not Channel 1)
  5   CTS0 Disable    ;/
  4   X1 Bit Clk ASCI
  3   BRG Mode (Time Constant based Baud Rate Generator)
  2   Break Feature Enable
  1   Break Detect (RO)
  0   Send Break
```

**1Ah - ASTC0L - ASCI Channel 0 Time Constant, Bit0-7 (00h on Reset)**

**1Bh - ASTC0H - ASCI Channel 0 Time Constant, Bit8-15 (00h on Reset)**

**1Ch - ASTC1L - ASCI Channel 1 Time Constant, Bit0-7 (00h on Reset)**

**1Dh - ASTC1H - ASCI Channel 1 Time Constant, Bit8-15 (00h on Reset)**

16bit Time Constants (see BRG bit in ASEXT0/ASEXT1).

**1Eh - CMR - Clock Multiplier Register (7Fh or so on Reset)**

```text
  7   X2  Enable X2 Clock Multiplier Mode (0=Disable, 1=Enable)
  6-0 -   Unused (should be all-ones) (or so)
```

Purpose undocumented, maybe doubles the CPU speed (and/or Timer or ASCI or
whatever speeds).

**1Fh - CCR - CPU Control Register (00h on Reset)**

```text
  7  Clock Divide (0=XTAL/2, 1=XTAL/1)
  6  Standby/Idle Mode, Bit1
  5  BREXT     (0=Ignore BUSREQ in Standby/Idle, Exit Standby/Idle on BUSREQ)
  4  LNPHI     (0=Standard Drive, 1=33% Drive on EXTPHI Clock)
  3  Standby/Idle Mode, Bit0
  2  LNIO      (0=Standard Drive, 1=33% Drive on certain external I/O)
  1  LNCPUCTL  (0=Standard Drive, 1=33% Drive on CPU control signals)
  0  LNAD/DATA (0=Standard Drive, 1=33% Drive on A10-A0, D7-D0)
```

Standby/Idle Mode is combined of CCR bit6/bit3 (0=No Standby, 1=Idle after
Sleep, 2=Standby after Sleep, 3=Standby after Sleep with 64 Cycle Exit Quick
Recovery).

**2Dh - IAR1B - DMA "I/O Address Ch 1" (Absurde name) (00h on Reset)**

```text
  7   Alternating Channels
  6   Currently selected DMA channel when Bit7=1
  5-4 Unused (should be zero) (must be 0)
  3   TOUT/DREQ-Pin (0=DREQ Input, 1=TOUT Output)
  2-0 DMA Channel 1 (0=TOUT/DREQ, 1=ASCI0, 2=ASCI1, 3=ESCC, 7=PIA)
```

<a id="snesdecompressionformats"></a>

## SNES Decompression Formats

**Nintendo-specific Compression Overall Format (BSX and SFC-Box)**

Compressed data consists of "code/length" pairs encoded in 1 or 2 bytes:

```text
  cccNnnnn           --> code (ccc=0..6) len 5bit (Nnnnn+1)
  111cccNn.nnnnnnnn  --> code (ccc=0..7) len 10bit (Nnnnnnnnnn+1)
  11111111           --> end code (FFh)
```

The "code/length" pairs are then follwed by "src" data, or "disp" offsets
(depending on the "ccc" codes). The meaning of the "ccc" codes varies from
program to program (see below for how they are used by BSX and SFC-Box).

Note: As seen above, ccc=7 works only with 10bit len (not 5bit len) and only
with max len=2FFh+1 (not 3FFh+1).

**Nintendo-specific Compression Codes (BSX) (Satellaview)**

Used to decompress various data (including custom person OBJs in the Directory
packet). The decompression functions are at 80939Fh (to RAM) and 80951Eh (to
VRAM) in the BSX-BIOS. The meaning of the "ccc" codes is:

```text
  0  Copy_bytes_from_src
  1  Fill_byte_from_src
  2  Fill_word_from_src
  3  Fill_incrementing_byte_from_src
  4  Copy_bytes_from_dest_base_plus_16bit_disp
  5  Copy_bytes_from_dest_base_plus_16bit_disp_with_invert
  6  Copy_bytes_from_current_dest_addr_minus_8bit_disp
  7  Copy_bytes_from_current_dest_addr_minus_8bit_disp_with_invert
```

For all codes (including ccc=2), len is the number of BYTEs to be
copied/filled. For ccc=4..6, the code is followed by a 16bit offset in
LITTLE-ENDIAN format. For ccc=5/7, copied data is inverted (XORed with FFh).

**Nintendo-specific Compression Codes (SFC-Box) (Super Famicom Box)**

Used (among others) to decompress the Title-Bitmaps in "GROM" EPROMs, the
decompression function is at 0088A2h in the "ATROM" menu program. The meaning
of the "ccc" codes is:

```text
  0  Copy_bytes_from_src
  1  Fill_byte_from_src
  2  Fill_word_from_src
  3  Fill_incrementing_byte_from_src
  4  Copy_bytes_from_dest_base_plus_16bit_disp
  5  Copy_bytes_from_dest_base_plus_16bit_disp_with_xflip
  6  Copy_bytes_from_dest_base_plus_16bit_disp_with_yflip
  7  Unused (same as ccc=4)
```

For ccc=2, len is the number of WORDs to be filled, for all other codes, it's
the number of BYTEs to be copied/filled. For ccc=4..7, the code is followed by
a 16bit offset in BIG-ENDIAN format. For ccc=5 (xflip), bit-order of all bytes
is reversed (bit0/1/2/3 &lt;--&gt; bit7/6/5/4). For ccc=6 (yflip), reading
starts at dest_base+disp (as usually), but the read-address is then decremented
after each byte-transfer (instead of incremented).

**SNES Decompression Hardware**

The APU automatically decompresses BRR-encoded audio samples (4bit to 15bit
ADPCM, roughly similar to CD-XA format). Cartridges with SPC7110 or S-DD1 chips
can decompress (roughly JPEG-style) video data, and convert it to SNES
bit-plane format. Cartridges with SA-1 chips include a "Variable-Length Bit
Processing" feature for reading "N" bits from a compressed bit-stream.
