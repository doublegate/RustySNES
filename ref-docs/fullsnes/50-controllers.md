# Fullsnes — Controllers & Input Peripherals

[Index](00-index.md) · [« Audio Processing Unit](40-apu-dsp.md) · [Cartridge Header, PCBs, CIC & Memory Mapping »](60-cartridge-header-and-mapping.md)

**Sections in this file:**

- [SNES Controllers](#snes-controllers)
  - [SNES Controllers I/O Ports - Automatic Reading](#snes-controllers-io-ports-automatic-reading)
  - [SNES Controllers I/O Ports - Manual Reading](#snes-controllers-io-ports-manual-reading)
  - [SNES Controllers Hardware ID Codes](#snes-controllers-hardware-id-codes)
  - [SNES Controllers Detecting Controller Support of ROM-Images](#snes-controllers-detecting-controller-support-of-rom-images)
  - [SNES Controllers Joypad](#snes-controllers-joypad)
  - [SNES Controllers Mouse (Two-button Mouse)](#snes-controllers-mouse-two-button-mouse)
  - [SNES Controllers Mouse Games](#snes-controllers-mouse-games)
  - [SNES Controllers Multiplayer 5 (MP5) (Five Player Adaptor)](#snes-controllers-multiplayer-5-mp5-five-player-adaptor)
  - [SNES Controllers Multiplayer 5 - Unsupported Hardware](#snes-controllers-multiplayer-5-unsupported-hardware)
  - [SNES Controllers Multiplayer 5 - Supported Games](#snes-controllers-multiplayer-5-supported-games)
  - [SNES Controllers SuperScope (Lightgun)](#snes-controllers-superscope-lightgun)
  - [SNES Controllers Konami Justifier (Lightgun)](#snes-controllers-konami-justifier-lightgun)
  - [SNES Controllers M.A.C.S. (Lightgun)](#snes-controllers-macs-lightgun)
  - [SNES Controllers Twin Tap](#snes-controllers-twin-tap)
  - [SNES Controllers Miracle Piano](#snes-controllers-miracle-piano)
  - [SNES Controllers Miracle Piano Controller Port](#snes-controllers-miracle-piano-controller-port)
  - [SNES Controllers Miracle Piano MIDI Commands](#snes-controllers-miracle-piano-midi-commands)
  - [SNES Controllers Miracle Piano Instruments](#snes-controllers-miracle-piano-instruments)
  - [SNES Controllers Miracle Pinouts and Component List](#snes-controllers-miracle-pinouts-and-component-list)
  - [SNES Controllers NTT Data Pad (joypad with numeric keypad)](#snes-controllers-ntt-data-pad-joypad-with-numeric-keypad)
  - [SNES Controllers X-Band Keyboard](#snes-controllers-x-band-keyboard)
  - [SNES Controllers Tilt/Motion Sensors](#snes-controllers-tiltmotion-sensors)
  - [SNES Controllers Lasabirdie (golf club)](#snes-controllers-lasabirdie-golf-club)
  - [SNES Controllers Exertainment (bicycle exercising machine)](#snes-controllers-exertainment-bicycle-exercising-machine)
  - [SNES Controllers Exertainment - I/O Ports](#snes-controllers-exertainment-io-ports)
  - [SNES Controllers Exertainment - RS232 Controller](#snes-controllers-exertainment-rs232-controller)
  - [SNES Controllers Exertainment - RS232 Data Packets & Configuration](#snes-controllers-exertainment-rs232-data-packets-configuration)
  - [SNES Controllers Exertainment - RS232 Data Packets Login Phase](#snes-controllers-exertainment-rs232-data-packets-login-phase)
  - [SNES Controllers Exertainment - RS232 Data Packets Biking Phase](#snes-controllers-exertainment-rs232-data-packets-biking-phase)
  - [SNES Controllers Exertainment - Drawings](#snes-controllers-exertainment-drawings)
  - [SNES Controllers Pachinko](#snes-controllers-pachinko)
  - [SNES Controllers Other Inputs](#snes-controllers-other-inputs)
- [SNES Add-On Turbo File (external backup memory for storing game positions)](#snes-add-on-turbo-file-external-backup-memory-for-storing-game-positions)
- [SNES Add-On Turbo File - TFII Mode Transmission Protocol](#snes-add-on-turbo-file-tfii-mode-transmission-protocol)
- [SNES Add-On Turbo File - TFII Mode Filesystem](#snes-add-on-turbo-file-tfii-mode-filesystem)
- [SNES Add-On Turbo File - STF Mode Transmission Protocol](#snes-add-on-turbo-file-stf-mode-transmission-protocol)
- [SNES Add-On Turbo File - STF Mode Filesystem](#snes-add-on-turbo-file-stf-mode-filesystem)
- [SNES Add-On Turbo File - Games](#snes-add-on-turbo-file-games)
- [SNES Add-On Barcode Battler (barcode reader)](#snes-add-on-barcode-battler-barcode-reader)
- [SNES Add-On Barcode Transmission I/O](#snes-add-on-barcode-transmission-io)
- [SNES Add-On Barcode Battler Drawings](#snes-add-on-barcode-battler-drawings)
- [SNES Add-On SFC Modem (for JRA PAT)](#snes-add-on-sfc-modem-for-jra-pat)
- [SNES Add-On SFC Modem - Data I/O](#snes-add-on-sfc-modem-data-io)
- [SNES Add-On SFC Modem - Misc](#snes-add-on-sfc-modem-misc)
- [SNES Add-On Voice-Kun (IR-transmitter/receiver for use with CD Players)](#snes-add-on-voice-kun-ir-transmitterreceiver-for-use-with-cd-players)
- [SNES 3D Glasses](#snes-3d-glasses)

---

<a id="snescontrollers"></a>

## SNES Controllers

**I/O Ports**

[SNES Controllers I/O Ports - Automatic Reading](#snes-controllers-io-ports-automatic-reading)

[SNES Controllers I/O Ports - Manual Reading](#snes-controllers-io-ports-manual-reading)

**Controller IDs**

[SNES Controllers Hardware ID Codes](#snes-controllers-hardware-id-codes)

[SNES Controllers Detecting Controller Support of ROM-Images](#snes-controllers-detecting-controller-support-of-rom-images)

**Standard Controllers**

[SNES Controllers Joypad](#snes-controllers-joypad)

[SNES Controllers Mouse (Two-button Mouse)](#snes-controllers-mouse-two-button-mouse)

[SNES Controllers Multiplayer 5 (MP5) (Five Player Adaptor)](#snes-controllers-multiplayer-5-mp5-five-player-adaptor)

**Light Guns**

[SNES Controllers SuperScope (Lightgun)](#snes-controllers-superscope-lightgun)

[SNES Controllers Konami Justifier (Lightgun)](#snes-controllers-konami-justifier-lightgun)

[SNES Controllers M.A.C.S. (Lightgun)](#snes-controllers-macs-lightgun)

**Other controllers**

[SNES Controllers Twin Tap](#snes-controllers-twin-tap)

[SNES Controllers Miracle Piano](#snes-controllers-miracle-piano)

[SNES Controllers NTT Data Pad (joypad with numeric keypad)](#snes-controllers-ntt-data-pad-joypad-with-numeric-keypad)

[SNES Controllers X-Band Keyboard](#snes-controllers-x-band-keyboard)

[SNES Controllers Tilt/Motion Sensors](#snes-controllers-tiltmotion-sensors)

[SNES Controllers Lasabirdie (golf club)](#snes-controllers-lasabirdie-golf-club)

[SNES Controllers Exertainment (bicycle exercising machine)](#snes-controllers-exertainment-bicycle-exercising-machine)

[SNES Controllers Pachinko](#snes-controllers-pachinko)

[SNES Controllers Other Inputs](#snes-controllers-other-inputs)

**Other devices that connect to the controller port**

[SNES Add-On Turbo File (external backup memory for storing game positions)](#snes-add-on-turbo-file-external-backup-memory-for-storing-game-positions)

[SNES Add-On Barcode Battler (barcode reader)](#snes-add-on-barcode-battler-barcode-reader)

[SNES Add-On SFC Modem (for JRA PAT)](#snes-add-on-sfc-modem-for-jra-pat)

[SNES Add-On Voice-Kun (IR-transmitter/receiver for use with CD Players)](#snes-add-on-voice-kun-ir-transmitterreceiver-for-use-with-cd-players)

**Other Add-Ons**

[SNES 3D Glasses](#snes-3d-glasses)

**Pinouts**

[SNES Controllers Pinouts](80-timings-unpredictable-pinouts.md#snes-controllers-pinouts)

<a id="snescontrollersioportsautomaticreading"></a>

### SNES Controllers I/O Ports - Automatic Reading

**4218h/4219h - JOY1L/JOY1H - Joypad 1 (gameport 1, pin 4) (R)**

**421Ah/421Bh - JOY2L/JOY2H - Joypad 2 (gameport 2, pin 4) (R)**

**421Ch/421Dh - JOY3L/JOY3H - Joypad 3 (gameport 1, pin 5) (R)**

**421Eh/421Fh - JOY4L/JOY4H - Joypad 4 (gameport 2, pin 5) (R)**

```text
  Register    Serial     Default
  Bit         Transfer   Purpose
  Number______Order______(Joypads)_____
  15          1st        Button B          (1=Low=Pressed)
  14          2nd        Button Y
  13          3rd        Select Button
  12          4th        Start Button
  11          5th        DPAD Up
  10          6th        DPAD Down
  9           7th        DPAD Left
  8           8th        DPAD Right
  7           9th        Button A
  6           10th       Button X
  5           11th       Button L
  4           12th       Button R
  3           13th       0 (High)
  2           14th       0 (High)
  1           15th       0 (High)
  0           16th       0 (High)
```

Before reading above ports, set Bit 0 in port 4200h to request automatic
reading, then wait until Bit 0 of port 4212h gets set-or-cleared? Once 4200h
enabled, seems to be automatically read on every retrace?

Be sure that Out0 in Port 4016h is zero (otherwise the shift register gets
stuck on the first bit, ie. all 16bit will be equal to the B-button state.

```text
 AUTO JOYPAD READ
 ----------------
 When enabled, the SNES will read 16 bits from each of the 4 controller port
 data lines into registers $4218-f. This begins between H=32.5 and H=95.5 of
 the first V-Blank scanline, and ends 4224 master cycles later. Register $4212
 bit 0 is set during this time. Specifically, it begins at H=74.5 on the first
 frame, and thereafter some multiple of 256 cycles after the start of the
 previous read that falls within the observed range.
```

```text
 Reading $4218-f during this time will read back incorrect values. The only
 reliable value is that no buttons pressed will return 0 (however, if buttons
 are pressed 0 could still be returned incorrectly). Presumably reading $4016/7
 or writing $4016 during this time will also screw things up.
```

<a id="snescontrollersioportsmanualreading"></a>

### SNES Controllers I/O Ports - Manual Reading

**4016h/Write - JOYWR - Joypad Output (W)**

```text
  7-3  Not used
  2    OUT2, Output on CPU Pin 39 (seems to be not connected) (1=High)
  1    OUT1, Output on CPU Pin 38 (seems to be not connected) (1=High)
  0    OUT0, Output on CPU Pin 37 (Joypad Strobe) (both gameports, pin 3)
```

Out0-2 are found on CPU Pins 37-39, of which only Out0 seems to be connected.

Note: The NSS (arcade cabinet) uses OUT2 to signalize Game Over to the Z80
coprocessor.

**4016h/Read - JOYA - Joypad Input Register A (R)**

```text
  7-2  Not used
  1    Input on CPU Pin 33, connected to gameport 1, pin 5 (JOY3) (1=Low)
  0    Input on CPU Pin 32, connected to gameport 1, pin 4 (JOY1) (1=Low)
```

Reading from this register automatically generates a clock pulse on CPU Pin 35,
which is connected to gameport 1, pin 2.

**4017h/Read - JOYB - Joypad Input Register B (R)**

```text
  7-5  Not used
  4    Input on CPU Pin 31, connected to GND (always 1=LOW)       (1=Low)
  3    Input on CPU Pin 30, connected to GND (always 1=LOW)       (1=Low)
  2    Input on CPU Pin 29, connected to GND (always 1=LOW)       (1=Low)
  1    Input on CPU Pin 28, connected to gameport 2, pin 5 (JOY4) (1=Low)
  0    Input on CPU Pin 27, connected to gameport 2, pin 4 (JOY2) (1=Low)
```

Reading from this register automatically generates a clock pulse on CPU Pin 36,
which is connected to gameport 2, pin 2.

**4201h - WRIO - Joypad Programmable I/O Port (Open-Collector Output) (W)**

```text
  7-0   I/O PORT  (0=Output Low, 1=HighZ/Input)
  7     Joypad 2 Pin 6 / PPU Lightgun input (should be usually always 1=Input)
  6     Joypad 1 Pin 6
  5-0   Not connected (except, used by SFC-Box; see Hotel Boxes)
```

Note: Due to the weak high-level, the raising "edge" is raising rather slowly,
for sharper transitions one may need external pull-up resistors.

**4213h - RDIO - Joypad Programmable I/O Port (Input) (R)**

```text
  7-0   I/O PORT  (0=Low, 1=High)
```

When used as Input via 4213h, set the corresponding bits in 4201h to HighZ.

I/O Signals 0..7 are found on CPU Pins 19..26 (in that order). IO-6 connects to
Pin 6 of Controller 1. IO-7 connects to Pin 6 of Controller 2, this pin is also
shared for the light pen strobe.

Wires are connected to IO-0..5, but the wires disappear somewhere in the
multi-layer board (and might be dead-ends), none of them is output to any
connectors (not to the Controller ports, not to the cartridge slot, and not to
the EXT port).

<a id="snescontrollershardwareidcodes"></a>

### SNES Controllers Hardware ID Codes

**Controller ID Bits (13th-16th bit, or extended: 13th-24th bit)**

```text
  13th ... 24th Hex  Type
  0000.00000000 0.00 No controller connected
  0000.11111111 0.FF Normal Joypad (probably also 3rd-party joypads/joysticks)
  0001          1    Mouse
  0010          2 ?  Unknown (if any)
  0011          3    SFC Modem (used by JRA PAT)
  0100          4    NTT Data Controller Pad (used by JRA PAT)
  ....          5-C  Unknown (if any)
  1101          D    Voice-Kun (IR-transmitter/receiver, for CD Players)
  1110.xxxxxxxx E.xx Third-Party Devices (see below)
  1110.000000xx E.0x Epoch Barcode Battler II (detection requires DELAYS?!)
  1110.01010101 E.55 Konami Justifier
  1110.01110111 E.77 Sunsoft Pachinko Controller
  1110.11111110 E.FE ASCII Turbo File Twin in STF mode
  1110.11111111 E.FF ASCII Turbo File Twin in TFII mode (or Turbo File Adapter)
  1111          F    Nintendo Super Scope
  N/A           N/A  M.A.C.S. (no ID, returns all bits = trigger button)
```

Note: The Multiplayer 5 can be also detected (using a different mechansim than
reading bits 13th-16th).

**Devices with unknown IDs (if they do have any special controller IDs at all)**

```text
  Lasabirdie            ;\
  Twin Tap              ; these should have custom IDs?
  Miracle Piano         ;
  X-Band Keyboard       ;/
  Exertainment          ;-connects to expansion port (thus no controller id)
  BatterUP                                              ;\
  TeeV Golf                                             ; these might return
  StuntMaster                                           ; normal "joypad" ID?
  Nordic Quest                                          ;
  Hori SGB Commander (in normal mode / in SGB mode)     ;
  Nintendo Joysticks                                    ;/
```

<a id="snescontrollersdetectingcontrollersupportofromimages"></a>

### SNES Controllers Detecting Controller Support of ROM-Images

Below are some methods to detect controller support by examining ROM-images.
The methods aren't fail-proof, but may be useful to track-down controller
support in many games.

**Detection Method Summary**

```text
  Type              Method
  Joypad            <none/default>
  Mouse             String "START OF MOUSE BIOS", or opcodes (see below)
  Multiplayer 5     String "START OF MULTI5 BIOS"
  Super Scope       String "START OF SCOPE BIOS", or Title=<see list>
  Lasabirdie        String "GOLF_READY!"
  X-Band Keyboard   String "ZSAW@",x,x,"CXDE$#" (keyboard translation table)
  Turbo File (STF)  String "FAT0SHVC"
  Turbo File (TFII) Opcodes "MOV Y,000Fh/MOV A,[004017h]/DEC Y/JNZ $-5"
  Exertainment      Opcodes "MOV [21C1h],A/MOV A,0Bh/MOV [21C4h],A/MOV X,20F3h"
  Barcode Battler   Opcodes "INC X/CMP X,(00)0Ah/JNC $-6(-1)/RET/36xNOP/RET"
  Voice-Kun         Opcodes "MOV [004201h],A/CLR P,20h/MOV A,D/INC A"
  Justifier         Title="LETHAL ENFORCERS     "
  M.A.C.S.          Title="MAC:Basic Rifle      "
  Twin Tap          Title="QUIZ OH SUPER        "
  Miracle Piano     Title="MIRACLE              "
  NTT Data Pad      Title="NTT JRA PAT          "
  SFC Modem         Title="NTT JRA PAT          "
  Pachinko          Title=CB,AF,BB,C2,CA,DF,C1,DD,CB,AF,BB,C2,CA,DF,C1,DD,xx(6)
  BatterUP          -  ;\
  TeeV Golf         -  ; these are probably simulating standard joypads
  StuntMaster       -  ; (and thus need no detection)
  Nordic Quest      -  ;/
```

**START OF xxx BIOS Strings**

These strings were included in Nintendo's hardware driver source code files,
the strings themselves have no function (so one could simply remove them), but
Nintendo prompted developers to keep them included. They are typically arranged
like so:

```text
  "START OF xxx BIOS"
  [bios program code...]
  "NINTENDO SHVC xxx BIOS VER x.xx"
  "END OF xxx BIOS"
```

Whereas, the version string may be preceeded by "MODIFIED FROM ", and "VER" may
be "Ver" in some cases, sometimes without space between "Ver" and "x.xx". In
case of custom code one may omit the version string or replace it by "MY BIOS
VERSION" or so, but one should include the "START/END OF xxx BIOS" strings to
ease detection.

**Multiplayer 5**

Games that do SUPPORT the hardware should contain following string:

```text
  "START OF MULTI5 BIOS"
```

Games that do DETECT the hardware should contain following string:

```text
  "START OF MULTI5 CONNECT CHECK"
```

Games that contain only the "CHECK" part (but not the "BIOS" part) may REJECT
to operate with the hardware.

Some MP5 games (eg. "Battle Cross") do lack the "START OF ..." strings.

**Mouse**

Games that do SUPPORT the hardware should contain following string:

```text
  "START OF MOUSE BIOS"
```

Some Mouse games (eg. Arkanoid Doh It Again) do lack the "START OF ..."
strings. For such games, checking following opcodes may help:

```text
  MOV Y,0Ah/LOP:/MOV A,[(00)4016h+X]/DEC Y/JNZ LOP/MOV A,[(00)4016h+X]
```

The official mouse BIOS uses 24bit "004016h+X" (BF 16 40 00), Arkanoid uses
16bit "4016h+X" (BD 16 40).

Warning: Automatically activiting mouse-emulation for mouse-compatible games
isn't a very good idea: Some games expect the mouse in port 1, others in port
2, so there's a 50% chance to pick the wrong port. Moreover, many games are
deactivating normal joypad input when sensing a connected mouse, so automatic
mouse emulation will also cause automatic problems with normal joypad input.

**SuperScope**

Games that do SUPPORT the hardware should (but usually don't) contain following
string:

```text
  "START OF SCOPE BIOS"
```

In practice, the string is included only in "Yoshi's Safari" whilst all other
(older and newer) games contain entirely custom differently implemented program
code without ID strings, making it more or less impossible to detect SuperScope
support. One workaround would be to check known Title strings:

```text
  "BATTLE CLASH         "    "METAL COMBAT         "    "T2 ARCADE            "
  "BAZOOKA BLITZKRIEG   "    "OPERATION THUNDERBOLT"    "TINSTAR              "
  "Hunt for Red October "    "SPACE BAZOOKA        "    "X ZONE               "
  "LAMBORGHINI AMERICAN "    "SUPER SCOPE 6        "    "YOSHI'S SAFARI       "
  "Lemmings 2,The Tribes"
```

**Lasabirdie**

Games that do support the hardware should contain "GOLF_READY!" string (used to
verify the ID received from the hardware).

**Turbo File Twin in STF Mode**

Games that do support the hardware should contain "FAT0" and SHVC" strings,
which are usually (maybe always) stored as continous "FAT0SHVC" string.

**Turbo File Twin in TFII Mode or Turbo File Adapter**

There aren't any specific ASCII Strings. However, most (or all) games contain
the "MOV Y,000F, MOV A,[004017], DEC Y, JNZ $-5" opcode sequence (exactly like
so, ie. with Y=16bit, and address=24bit).

**NSRT Header**

Some ROM-images do contain information about supported controllers in NSRT
Headers. In practice, most ROM-images don't have that header (but can be
"upgraded" by Nach's NSRT tool).

[SNES Cartridge ROM-Image Headers and File Extensions](60-cartridge-header-and-mapping.md#snes-cartridge-rom-image-headers-and-file-extensions)

The NSRT format isn't officially documented. The official way to create headers
seems to be to contact the author (Nach), ask him to add controller flags for a
specific game, download the updated version of the NSRT tool, use it to update
your ROM-image, and then you have the header (which consists of undocumented,
and thereby rather useless values).

<a id="snescontrollersjoypad"></a>

### SNES Controllers Joypad

**Joypad Bits**

```text
  1st          Button B        (0=High=Released, 1=Low=Pressed)
  2nd          Button Y        (0=High=Released, 1=Low=Pressed)
  3rd          Button Select   (0=High=Released, 1=Low=Pressed)
  4th          Button Start    (0=High=Released, 1=Low=Pressed)
  5th          Direction Up    (0=High=Released, 1=Low=Pressed)
  6th          Direction Down  (0=High=Released, 1=Low=Pressed)
  7th          Direction Left  (0=High=Released, 1=Low=Pressed)
  8th          Direction Right (0=High=Released, 1=Low=Pressed)
  9th          Button A        (0=High=Released, 1=Low=Pressed)
  10th         Button X        (0=High=Released, 1=Low=Pressed)
  11th         Button L        (0=High=Released, 1=Low=Pressed)
  12th         Button R        (0=High=Released, 1=Low=Pressed)
  13th         ID Bit3         (always 0=High)
  14th         ID Bit2         (always 0=High, except 1=Low for NTT Data Pad)
  15th         ID Bit1         (always 0=High)
  16th         ID Bit0         (always 0=High)
  17th and up  Padding         (always 1=Low) (or 0=High when no pad connected)
```

**Joypad Physical Appearance**

```text
    __--L--_________________--R--__           Button Colors:
   /    _                          \   PAL and Japan    North America
  |   _| |_                  (X)    |   X = Blue         X = Gray
  |  |_   _|  SLCT STRT   (Y)   (A) |   Y = Green        Y = Gray
  |    |_|                   (B)    |   A = Red          A = Purple
   \_________.-----------._________/    B = Yellow       B = Purple
```

**Joypad/Joystick Variants**

There are numerous variants from various companies, some including extra
features like auto-fire.

```text
  Advanced Control Pad (Mad Catz) (joypad with autofire or so)
  Angler (?) Functionally identical to the ASCII Pad (optional "stick" in dpad)
  asciiGrip (ASCII) (normal joypad for single-handed use)
  asciiPad (ASCIIWARE) (joypad with autofire and slowmotion)
  Capcom Pad Soldier (Capcom) (standard pad in bent/squeezed/melted design)
  Competition Pro (Competition Pro) (joypad with autofire and slowmotion)
  Competition Pro (Competition Pro) (slightly redesigned standard joypad)
  Conqueror 2 (QuickShot?) (joystick with autofire, programmable buttons)
  Cyberpad (Quickshot?) (6-shaped pad, programmable, autofile, slow motion)
  Dual Turbo (Akklaim) (set of 2 wireless joypads with autofire or so)
  Energiser (?) (very odd shaped pad, programmable, auto fire, slow motion)
  Fighter Stick SN (?) (desktop joystick, with autofire or so)
  Gamemaster (Triton) (edgy-shaped pad, one programmable button)
  High Frequency Control Pad (High Frequency) (normal pad, wrong button colors)
  Invader 2 (QuickShot?) (joypad with autofire)
  JS-306 Power Pad Tilt (Champ) (joypad with autofire, slowmotion, tilt-mode)
  Multisystem 6 (Competition Pro) (pad supports Genesis and SNES)
  Nigal Mouncefill Fly Wheel (Logic 3) (wheel-shaped, tilt-sensor instead dpad)
  NTT Data Pad (for JRA PAT) (joypad with numeric keypad) (special ID)
  Pro Control 6 (Naki) (joypad, programmable & whatever extra features)
  Pro-Player (?) (joystick)
  Score Master (Nintendo) (desktop joystick with autofire or so)
  SF-3 (Honey Bee) (very flat normal pad with autofire)
  SGB Controller (?) (joypad ...)
  SN Propad
  SN Propad 2
  SN Propad 6
  SN-6 (Gamester) (standard joypad clone)
  Specialized Fighter Pad (ASCIIWARE) (autofire, L/R as "normal" buttons)
  Speedpad (?) (joypad, one auto-switch, L/R buttons as "normal" buttons)
  Super Control Pad (?) (standard joypad clone, plus 3-position switch?)
  Super Joy Card (Hudson) (standard joypad with auto-fire or so)
  Supercon (QuickShot) (standard joypad, odd shape, odd start/select buttons)
  Superpad (InterAct) (standard joypad clone)
  Superpad (noname) (standard joypad)
  TopFighter (?) (desktop joystick, programmable, LCD panel, auto-fire, slowmo)
  Turbo Touch 360 (Triax) (joypad with autofire)
  V356 (Recoton) (normal joypad, with whatever 3-position switch)
  noname joypads (normal joypad clones without nintendo text nor snes logo)
  joypad (Konami) (wireless joypad, no extra functions) (dish-shaped receiver)
  joypads (Game Partner) (set of 2 wireless joypads with autofire or so)
  AK7017828 or so??? (Game Partner) (joypad, slow motion, auto fire)
  Noname pad (Tomee) (standard joypad clone)
  SNES+MD? (Nakitek) (joypad with whatever special features)
```

Capcom Fighter Power Stick

Super Advantage Joystick

SGB Commander (HORI)

Battle Pachislot Controller (Sammy) (joypad for "one-armed bandit" games)

**Power Plug (Tyco)**

Auto-fire adaptor, plugs between any joypad/joystick and snes console.

<a id="snescontrollersmousetwobuttonmouse"></a>

### SNES Controllers Mouse (Two-button Mouse)

**Mouse Connection**

The mouse can be connected to Controller Port 1 or 2. Default seems to be Port
1 for most games. Exception: Satellaview FLASH games should default to Port 2
(the joypad controlled BS-X BIOS doesn't work with mouse plugged into Port 1).

Mario Paint accepts it ONLY in Port 1, other games may accept either port
(maybe there are also some that accept only Port 2?). Two-player games (eg.
Operation Thunderbolt) may accept two mice to be connected. Some games (eg.
Super Bomberman Panic Bomber World) refuse to run if a mouse is connected. The
mouse should not be connected to Multiplayer 5 adaptors (which allow only 17mA
per controller, whilst the mouse requires 50mA).

**Supported Games**

[SNES Controllers Mouse Games](#snes-controllers-mouse-games)

**Mouse Bits**

```text
  1st..8th     Unused       (always 0=High)
  9th          Right Button (0=High=Released, 1=Low=Pressed)
  10th         Left Button  (0=High=Released, 1=Low=Pressed)
  11th         Sensitivity Bit1   (0=High=Zero)     ;\0=slow, 1=normal, 2=fast
  12th         Sensitivity Bit0   (0=High=Zero)     ;/
  13th         ID Bit3      (always 0=High)
  14th         ID Bit2      (always 0=High)
  15th         ID Bit1      (always 0=High)
  16th         ID Bit0      (always 1=Low)
  17th         Vertical Direction     (0=High=Down, 1=Low=Up)
  18th         Vertical Offset Bit6   (0=High=Zero)    ;\
  19th         Vertical Offset Bit5   (0=High=Zero)    ;
  20th         Vertical Offset Bit4   (0=High=Zero)    ; this is a 7bit
  21th         Vertical Offset Bit3   (0=High=Zero)    ; UNSIGNED value
  22th         Vertical Offset Bit2   (0=High=Zero)    ; (00h=No motion)
  23th         Vertical Offset Bit1   (0=High=Zero)    ;
  24th         Vertical Offset Bit0   (0=High=Zero)    ;/
  25th         Horizontal Direction   (0=High=Right, 1=Low=Left)
  26th         Horizontal Offset Bit6 (0=High=Zero)    ;\
  27th         Horizontal Offset Bit5 (0=High=Zero)    ;
  28th         Horizontal Offset Bit4 (0=High=Zero)    ; this is a 7bit
  29th         Horizontal Offset Bit3 (0=High=Zero)    ; UNSIGNED value
  30th         Horizontal Offset Bit2 (0=High=Zero)    ; (00h=No motion)
  31th         Horizontal Offset Bit1 (0=High=Zero)    ;
  32th         Horizontal Offset Bit0 (0=High=Zero)    ;/
  33th and up  Padding      (always 1=Low)
```

Note that the motion values consist of a Direction Bit and an UNSIGNED 7bit
offset (ie. not a signed 8bit value). After reading, the 7bit offsets are
automatically reset to zero (whilst the direction bits do reportedly stay
unchanged unless/until the mouse is moved in opposite direction).

**Mouse Support ID-String**

Games that support the mouse should contain the string "START OF MOUSE BIOS"
somewhere in the ROM-image.

**Mouse Sensitivity**

The Mouse Resolution is specified as "50 counts/inch (+/-10%)". There are three
selectable Sensitivity (Threshold) settings:

```text
  0 - slow   - linear fixed level (1:1)
  1 - normal - exponential -?- levels (1:1 to ?:1)  (?:1=smaller than 6:1)
  2 - fast   - exponential six levels (1:1 to 6:1)
```

Setting 0 returns raw mickeys (so one must implement effects like double-speed
threshold by software). Settings 1-2 can be used directly as screen-pixel
offsets. To change the sensitivity (for port n=0 or n=1):

```text
  [4016h]=01h           ;set STB=1
  dummy=[4016h+n]       ;issue CLK pulse while STB=1 <-- increments the value,
  [4016h]=00h           ;set STB=0                       or wraps from 2 to 0
  ;Thereafter, one should read the Sensitivity bits, typically like so:
  [4016h]=01h           ;set STB=1  ;\another STB on/off, for invoking reading
  [4016h]=00h           ;set STB=0  ;/(not sure if this part is required)
  for i=11 to 0, dummy=[4016h+n], next i              ;skip first 12 bits
  for i=1 to 0, sensitivity.bit(i)=[4016h+n], next i  ;read 2 sensitivity bits
  ;Repeat the above procedure until the desired sensitivity value is reached.
```

Caution: According to Nintendo, the internal threshold factors aren't
initialized until the change-sensitivty procedure is executed at least once
(ie. after power-up, or after sensing a newly connected mouse, one MUST execute
the change-sensitivity procedure, EVEN if the mouse does return the desired
2bit sensitivity code).

<a id="snescontrollersmousegames"></a>

### SNES Controllers Mouse Games

The SNES Mouse is supported by many games, whereas most of them (except Mario
Paint) can be also used with normal joypads.

**Games that support the SNES Mouse (the list may be incomplete)**

```text
  Acme Animation Factory
  Alice Paint Adventure
  Arkanoid: Doh It Again
  Bishoujo Senshi Sailor Moon S Kondowa Puzzle de Oshioikiyo! (Japan only)
  Brandish 2: Expert (Japan only)
  BreakThru!
  Civilization
  Cameltry (called On The Ball in North America and the UK)
  Cannon Fodder
  Dai3ji Super Robot Taisen (Japan only)
  Dai4ji Super Robot Taisen (Japan only)
  Doom
  Dokyusei 2 (Japan only)
  Dragon Knight 4 (Japan only)
  Eye of the Beholder
  Farland Story 2 (Japan only)
  Fun and Games
  Galaxy Robo (Japan only)
  Hiouden: Mamono-tachi tono Chikai (Japan only)
  Jurassic Park (mouse MUST be in slot 2)
  King Arthur's World
  Koutetsu No Kishi (Japan only)
  Koutetsu No Kishi 2 (Japan only)
  Koutetsu No Kishi 3 (Japan only)
  Lamborghini American Challenge
  Lemmings 2: The Tribes
  Lord Monarch (Japan only)
  The Lord of the Rings
  Mario and Wario (Japan only)
  Mario Paint (1992)
  Mario's Super Picross (Japan only)
  Mario's Early Years: Pre-School
  Mega Lo Mania
  Might and Magic III
  Motoko-chan no Wonder Kitchen (Japan only)
  Nobunaga's Ambition
  On the ball
  Operation Thunderbolt
  Pieces
  Populous II
  Power Monger
  Revolution X
  San Goku Shi Seishi: Tenbu Spirits (Japan only)
  Shien's Revenge
  SimAnt
  Snoopy Concert
  Sound Fantasy (unreleased)
  Spellcraft (unreleased)
  Super Caesars Palace
  Super Game Boy
  Super Castles (Japan only)
  Super Noah's Ark 3D
  Super Pachi-slot Mahjong
  Super Robot Wars 3
  Super Solitaire
  Terminator 2: The Arcade Game
  Tin Star
  Tokimeki Memorial (Japan only)
  Troddlers
  Utopia
  Vegas Stakes
  Warrior of Rome III (unreleased)
  Wolfenstein 3D
  Wonder Project J
  Zan 2: Spirits (Japan only)
  Zan 3: Spirits (Japan only)
```

Plus (!):

```text
  Kaite Tsukutte Asoberu Dezaemon (Japan only)
  Pro Action Replay Mk3
  Kakinoki Shogi (1995) ASCII Corporation (JP)
  Spell Craft: Aspects of Valor (1993) ... same as "Spellcraft (unreleased)"?
    (in Spellcraft: mouse works in-game only, not in title screen)
```

Moreover, the "SNES Test Program" (1991 by Nintendo) includes a Mouse test.

<a id="snescontrollersmultiplayer5mp5fiveplayeradaptor"></a>

### SNES Controllers Multiplayer 5 (MP5) (Five Player Adaptor)

The MP5 plugs into one Controller Port on the SNES (typically Port 2), and has
4 ports for controllers to be plugged into it (labeled 2 through 5). It also
has an override switch which makes it pass through Pad 2 and ignore everything
else.

**Reading Controller Data**

```text
  [4016h].Bit0=1                                 ;-strobe on (to player 1-5)
  [4016h].Bit0=0                                 ;-strobe off (to player 1-5)
  read any number of bits from [4016h].Bit0      ;-read Player 1 data
  read any number of bits from [4017h].Bit0/Bit1 ;-read Player 2/3 data
  [4201h].Bit7=0                                 ;-select Player 4/5
  read any number of bits from [4017h].Bit0/Bit1 ;-read Player 4/5 data
  [4201h].Bit7=1  ;(prepare here for next frame) ;-select Player 2/3
  do no further access until next frame (allow [4201h].Bit7=1 to stabilize)
```

The strobe on/off part, and reading first 16bits for player 1-3 is usually done
via automatic reading (whereas, Player 3 data will obviously show up in "JOY4"
register, not in "JOY3" register). Whilst reading further player 1-3 bits, and
all player 4-5 bits is done via manual reading.

As shown above, player 2-3 should be always read before 4-5, for two reasons:

At least some MP5 devices may respond slowly on 0-to-1 transitions of
[4201h].Bit7 (unless the device contains a pull-up resistor). Some MP5's
(namely the Tribal Tap) are always passing CLK to player 2 (in that case player
2 data would be shifted-out when accessing player 4-5 data).

**Detecting the MP5 Hardware**

Below can be used to detect MP5 in ports 1 (n=0) and 2 (n=1). Games do usually
check both ports (and show an error messages when sensing a MP5 in port 1).

```text
  [4016h].Bit0=1                              ;-strobe on (force MP5 Bit1=1)
  read 8 bits from [4016h+n].Bit1 to byte A   ;-read byte A
  [4016h].Bit0=0                              ;-strobe off (normal data mode)
  read 8 bits from [4016h+n].Bit1 to byte B   ;-read byte B
  if A=FFh and B<>FFh then MP5=present        ;-verify result
```

If there's no MP5 connected, then A and B will be typically 00h (since most
controllers don't use [4017h].Bit1, exceptions are Turbo File, SFC Modem,
Voice-Kun, and X-Band Keyboard).

If a MP5 is connected, then A will be FFh, and B will be first 8bit of data
from joypad 3 or 5 (which can't be FFh since one can't push all four DPAD
directions at once).

Also note that there is nothing preventing the MP5 from functioning perfectly
when plugged in to Port 1, except that the game must use bit 6 of $4201 instead
of bit 7 to set IOBit and must use the Port 1 registers instead of the Port 2
registers. With 2 MP5 units, one could actually create an 8-player game.

**Supported/Unsupported Games/Hardware**

The Multiplayer is supported by more than 100 games, but incompatible with
almost everything except normal joypads.

[SNES Controllers Multiplayer 5 - Unsupported Hardware](#snes-controllers-multiplayer-5-unsupported-hardware)

[SNES Controllers Multiplayer 5 - Supported Games](#snes-controllers-multiplayer-5-supported-games)

**Multitap/Multiplayer Adaptor Versions**

```text
  2or3? Way Multiplay Adaptor (Gamester LMP) (with only 2 (or 3?) sockets)
  5 Player Game Plug (Laing) (same polygonal case as SN-5)
  HORI Multitap HSM-07 (HORI) (4 "top-loading" connectors)
  HORI Super Tetris 3 (HORI) (red case, otherwise same as HORI HSM-07)
  Multi Adaptor Auto (Partyroom21)
  Multi Player Adaptor (unknown manufacturer) (roughly PS1 shaped)
  Multi-Player Adaptor (Super Power) (same case as Multiplay Adaptor from LMP)
  Multiplay Adaptor (Gamester LMP) (square gray case, "crown" shaped LMP logo)
  SN-5 Multitap (Phase 9) (same polygonal case as Super 5 QJ/Super 5-Play)
  SNES MultiPlayer 5 Schematic Diagram (1st May 1992) (Nintendo) (book2.pdf)
  Super 5 QJ (same polygonal case as SN-5)
  Super 5 Multi-Player Adapter by Innovation (same polygonal case as SN-5)
  Super 5-Play (Performance) (same polygonal case as SN-5)
  Super Link by BPS (Bullet Proof Software) (same case as HORI HSM-07)
  Super Multitap (noname) (polyshaped, but different than the SN-5 case)
  Super Multitap (Hudson) (long slim device with 4 connectors on front panel)
  Super Multitap 2 (Hudson) (square device with yellow Bomberman face)
  Super Multitap Honest (same polygonal case as SN-5)
  Tribal-Tap 5 (Nakitek) (same case as Multiplay Adaptor from LMP)
  Tribal Tap, 6 Player Adaptor (Naki)
  Tribal Tap, 6 Player Adaptor (Fire) (same as Naki, but without Naki logo)
```

**SNES MultiPlayer 5 - Schematic Diagram (Rev 2.3) 1st May 1992**

```text
              _________                            _________
             |74HCT4053|                          | 74HC241 |
             |         |               /MODE5P--->|/OE      |
     ??? --->|VEE   /EN|<------STB--------------->|IN    OUT|---> STB'BCD
             |  _ _ _  |                 STB'A--->|IN    OUT|---> DETECT
  IO'SEL --->|SELX   X1|------>CLK1-------------->|IN    OUT|---> CLK'B
     CLK --->|X _ _ _X0|------>CLK0-------------->|IN _ _OUT|---> CLK'CD
  IO'SEL --->|SELY   Y1|<-------------------------|OUT    IN|<--- IN0'A
     IN0 <---|Y _ _ _Y0|<-------------------------|OUT    IN|<--- IN0'C
  IO'SEL --->|SELZ   Z1|<-------------------------|OUT    IN|<--- IN01
     IN1 <---|Z      Z0|<-------------------------|OUT    IN|<--- IN0'D
             |         |                  ??? --->|OE       |
             |_________|                          |_________|
              _________                            _________
             | 74HC126 |                          |4-Channel|
     CLK --->|IN    OUT|---> CLK1         VCC --->|2P Switch|---> /MODE5P
   STB'A --->|OE _ _   |                  GND --->|5P _ _ _ |
    CLK1 --->|IN    OUT|---> CLK'A        GND --->|2P       |---> VCC'BCD
     ??? --->|OE _ _   |                  VCC --->|5P _ _ _ |
     STB --->|IN    OUT|---> STB'A      IN1'A --->|2P       |---> IN01
     ??? --->|OE _ _   |                IN0'B --->|5P _ _ _ |
     GND --->|IN    OUT|---> IN1         IO'A <---|2P       |<--- IO
  DETECT --->|OE       |               IO'SEL <---|5P       |
             |_________|                          |_________|
   __________________________________________
  |   (Female)(............Male.............)|     GND ------[10K]----- DETECT
  |Pin SNES   PORT2   PORT3   PORT4   PORT5  |     VCC ------[10K]----- IO'SEL
  |1   VCC    VCC     VCC'BCD VCC'BCD VCC'BCD|     VCC ------[10K]----- CLK1
  |2   CLK    CLK'A   CLK'B   CLK'CD  CLK'CD |     VCC ------[10K]----- CLK0
  |3   STB    STB'A   STB'BCD STB'BCD STB'BCD|     VCC ------[10K]----- IN0'A
  |4   IN0    IN0'A   IN0'B   IN0'C   IN0'D  |     VCC ------[10K]----- IN01
  |5   IN1    IN1'A   -       -       -      | VCC'BCD ------[10K]----- IN0'C
  |6   IO     IO'A    -       -       -      | VCC'BCD ------[10K]----- IN0'D
  |7   GND    GND     GND     GND     GND    | VCC'BCD --<LED|--[220]-- VCC
  |__________________________________________| VCC'BCD --------||------ GND
```

The schematic was released by Nintendo (included in book2.pdf), components are:

```text
  74HCT4053 (triple 2-to-1 line analog multiplexer/demultiplexer)
  74HC126 (quad 3-state noninverting buffer with active high enables)
  74HC241 (dual 4-bit 3-state noninverting buffer/line driver)
  4-channel 2-position switch (2P/5P-mode selection)
  LED (glows in 2P-mode)
  1 female joypad connector, 4 male joypad connectors
  plus some resistors
```

Connection of the four "???" pins is unclear (maybe just wired to VCC or GND).

Unknown if any of the existing adaptors do actually use the above schematic
(Hudson's Multitap and Multitap 2 are both using a single custom 20pin
"HuC6205B" instead of the above schematic).

**Tribal Tap (Naki)**

This adaptor is supposed to support up to 6 players (one more than the normal
multitaps). The 6-player feature isn't supported by any games, and it's unknown
how to access the 6th port by software - some people do believe that it isn't
possible at all, and the the 6th port is just a fake - but, that theory is
based on the (incorrect) assumption that PALs cannot be programmed to act as
flipflops. However, if the schematic shown below is correct (the "IN0'E" signal
from Port6 being really &amp; solely &lt;input&gt; to the OUT0 &lt;output&gt;
pin; not verified), then it's probably really a fake.

The Tribal Tap schematic should be reportedly looking somehow like so:

```text
                           .-----. .-----.
  VCC--[RP]-------CLK---> 1|IN0  '-'  VCC|20 <---VCC
  VCC--[RP]--------IO---> 2|IN1 16L8 OUT7|19 --->IN0    (out-only)
  VCC--[RP]-----IN0'A---> 3|IN2 PAL  I/O6|18 --->IN1
  VCC--[RP]-----IN1'A---> 4|IN3      I/O5|17 --->STB'A
  VCC--[RP]---/MODE6P---> 5|IN4      I/O4|16 --->IO'A
  VCC--[RP]-----IN0'B---> 6|IN5      I/O3|15 --->CLK'B
  VCC--[RP]---/MODE2P---> 7|IN6      I/O2|14 --->CLK'CDE
  VCC--[RP]-----IN0'D---> 8|IN7      I/O1|13 --->STB'BCDE
  VCC--[RP]-----IN0'C---> 9|IN8      OUT0|12 <---IN0'E  (out-only???)
                  GND -->10|GND       IN9|11 <---/STB----[R]---VCC
                           '-------------'
       .----------.           .-------.             .-------.
       |3-position|           | S9013 |             | S9013 |        .-[R]-STB
       |switch  2P|--/MODE2P  |      E|-->VCC'BCDE  |      E|---GND  |
  GND--|        5P|--NC       | NPN  B|<--/MODE2P   | NPN  B|<-------+
       |        6P|--/MODE6P  |      C|---SNES.VCC  |      C|-->/STB |
       '----------'           '-------'             '-------'        '-[R]-GND
  .------------------------------------------------------.
  |   (Female)(.............Male........................)|    ???--|LED>--???
  |Pin SNES   PORT2   PORT3    PORT4    PORT5    PORT6   |
  |1   VCC    VCC     VCC'BCDE VCC'BCDE VCC'BCDE VCC'BCDE|    further resistors
  |2   CLK    CLK     CLK'B    CLK'CDE  CLK'CDE  CLK'CDE |    ???
  |3   STB    STB'A   STB'BCDE STB'BCDE STB'BCDE STB'BCDE|
  |4   IN0    IN0'A   IN0'B    IN0'C    IN0'D    IN0'E   |    (not installed)
  |5   IN1    (IN1'A) -        -        -        -       |    diodes ???
  |6   IO     (IO'A)  -        -        -        -       |
  |7   GND    GND     GND      GND      GND      GND     |
  '------------------------------------------------------'
     Note: The PCB has wires to all 7 pins of PORT2,
     but the installed connector has only 5 pins, so,
     in practice, IN1'A and IO'A are not connected.
```

<a id="snescontrollersmultiplayer5unsupportedhardware"></a>

### SNES Controllers Multiplayer 5 - Unsupported Hardware

The Multiplayer 5 is incompatible with almost everything except normal
joypads/joysticks. The only other things that do work are Twin Taps, and maybe
also the NTT Data Pad (unless it exceeds 17mA, and unless games do refuse it as
device with "unknown" controller ID).

**Unsupported Hardware in MP5 controller slots (due to missing signals)**

```text
  Lightguns
  Turbo File
  SFC Modem
  Voice-Kun
  X-Band Keyboard
  A second MP5 plugged into the first MP5
```

**Prohibited Hardware**

The hardware/combinations listed below are "prohibited" (and aren't supported
by any offical/licensed games). Nethertheless, they should be working in
practice, hopefully without immediately catching fire or blowing the fuse (but
might act unstable or cause some overheating in some situations; results
&lt;might&gt; vary depending on hardware/production variants, room temperature,
used CPU/PPU/APU load, individual or official safety measures, and on
type/amount of connected cartridges/controllers).

**Prohibited Hardware in MP5 controller slots (due to exceeding 17mA per slot)**

```text
  Mouse (requires 50mA)
  Devices with unknown controller IDs (which might exceed 17mA)
  Maybe also various unlicensed wireless/autofire joypads/joysticks
```

**Prohibited Hardware in CARTRIDGE slot (due to overall power consumption)**

```text
  Cartridges with GSU-n (programmable RISC CPU) (aka Super FX/Mario Chip)
  Maybe also things like X-Band modem and Cheat Devices
```

**Prohibited Hardware in BOTH controller ports (unspecified reason)**

```text
  Two MP5's (connected to port 1 and 2) (maybe also power consumption related)
```

**Prohibited Hardware in FIRST controller port (just by convention)**

```text
  MP5 in port 1 (instead of port 2) (would mess-up the port 2-5 numbering)
```

<a id="snescontrollersmultiplayer5supportedgames"></a>

### SNES Controllers Multiplayer 5 - Supported Games

**Multiplayer 5 Games**

```text
 Game                                                       Languages   Players
 Bakukyuu Renpatsu!! Super B-Daman ("battle mode")                (J)         4
 Bakutou Dochers: Bumps-jima wa Oosawagi ("battle mode")          (J)         4
 Barkley Shut Up and Jam! / Barkey no Power Dunk                  (E,J)       4
 Battle Cross (supports only 5 joypads; player 6 always inactive) (J)         5
 Battle Jockey                                                    (J)         4
 Bill Walsh College Football                                      (E)         4
 Chibi Maruko-chan: Mezase! Minami no Island!!                    (J)         4
 College Slam                                                     (E)         4
 Crystal Beans From Dungeon Explorer                              (J)         3
 Dino Dini's Soccer!                                              (E,F,G)    ??
 Dragon: The Bruce Lee Story                                      (E)         3
 Dream Basketball: Dunk & Hoop (only 5, not 6)                    (J)         5
 Dynamic Stadium                                                  (J)         4
 Elite Soccer / World Cup Striker                                 (E,F,G,J)   4
 ESPN National Hockey Night                                       (E)         5
 FIFA International Soccer                                        (E,J)       4
 FIFA Soccer 96                                                (E,F,G,I,S,SW) 4
 FIFA Soccer 97: Gold Edition / FIFA 97: Gold Edition          (E,F,G,I,S,SW) 5
 FIFA 98: Road to World Cup                                    (E,F,G,I,S,SW) 5
 Finalset                                                         (J)         4
 Fire Striker / Holy Striker                                      (E,J)       4
 Fever Pitch Soccer / Head-On Soccer                              (E,F,G,I,S) 4
 From TV Animation Slam Dunk: SD Heat Up!!                        (J)         5
 Go! Go! Dodge League                                             (J)         4
 HammerLock Wrestling/Tenryuu Genichirou no Pro Wrestling Revol.  (E,J)       4
 Hanna Barbera's Turbo Toons                                      (E)         5
 Hat Trick Hero 2                                                 (J)         4
 Hebereke no Oishii Puzzle wa Irimasenka                          (J)         5
 Human Grand Prix III: F1 Triple Battle                           (J)         3
 Human Grand Prix IV: F1 Dream Battle                             (J)         3
 Hungry Dinosaurs / Harapeko Bakka                                (E,J)      ??
 International Superstar Soccer Deluxe/Jikkyou World Soccer 2     (E,J)       4
 J.League Excite Stage '94 / Capcom's Soccer Shootout             (E,J)       4
 J.League Excite Stage '95                                        (J)         4
 J.League Excite Stage '96                                        (J)         4
 J.League Soccer Prime Goal                                       (J)         ?
 J.League Super Soccer '95 Jikkyo Stadium                         (J)         4
 J.R.R. Tolkien's The Lord of the Rings: Volume 1                 (E,G)       4
 Jikkyou Power Pro Wrestling '96: Max Voltage (only 4, not 5) 5?  (J)         4
 Jimmy Connors Pro Tennis Tour (japanese version only?)           (E,F,G,J)   4
 JWP Joshi Pro Wrestling: Pure Wrestle Queens                     (J)         4
 Kingyo Chuuihou! Tobidase! Game Gakuen                           (J)         3
 Kunio-kun no Dodge Ball Da yo Zenin Shuugou!                     (J)         4
 Looney Tunes B-Ball / Looney Tunes Basketball                    (E)         4
 Madden NFL '94 / NFL Pro Football '94                            (E,J)       4
 Madden NFL 95                                                    (E)         4
 Madden NFL 96                                                    (E)         5
 Madden NFL 97                                                    (E)         4
 Madden NFL 98                                                    (E)         5
 Micro Machines                                                   (E)         4
 Micro Machines 2: Turbo Tournament                               (E)         4
 Mizuki Shigeru no Youkai Hyakkiyakou                             (J)         4
 Multi Play Volleyball                                            (J)         4
 Natsume Championship Wrestling                                   (E)         4
 N-Warp Daisakusen (homebrew) (requires 2 multitaps)              (E)         8
 NBA Give 'n Go / NBA Jikkyou Basket: Winning Dunk                (E,J)       4
 NBA Hang Time                                                    (E)         4
 NBA Jam                                                          (E,J)       4
 NBA Jam: Tournament Edition                                      (E,J)       4
 NBA Live 95                                                      (E,J)       5
 NBA Live 96                                                      (E)         5
 NBA Live 97                                                      (E)         5
 NBA Live 98                                                      (E)         5
 NCAA Final Four Basketball                                       (E)         4
 NCAA Football                                                    (E)         4
 NFL Quarterback Club / NFL Quarterback Club '95                  (E,J)       5
 NFL Quarterback Club 96                                          (E,J)       5
 NHL '94 / NHL Pro Hockey '94                                     (E,J)       5
 NHL '94 / NHL Pro Hockey '94                                     (E,J)       5
 NHL '95                                                          (E)         ?
 NHL '96                                                          (E)         ?
 NHL '97                                                          (E)         ?
 NHL '98                                                          (E)         5
 Olympic Summer Games                                             (E)         5
 Peace Keepers, The / Rushing Beat Shura                          (E,J)       4
 Pieces / Jigsaw Party                                            (E,J)       5
 Rap Jam: Volume One                                              (E,F,S)     4
 Saturday Night Slam Masters / Muscle Bomber: Body Explosion      (E,J)       4
 Secret of Mana / Seiken Densetsu 2                               (E,F,G,J)   3
 Secret of Mana 2 / Seiken Densetsu 3 (with patch by Parlance)    (E,J,patch) 3
 Shijou Saikyou no Quiz Ou Ketteisen Super (uses 4 twin taps)     (J)         8
 Shin Nihon Pro Wrestling: Chou Senshi in Tokyo Dome              (J)         4
 Shin Nihon Pro Wrestling Kounin: '94 Battlefield in Tokyo Dome   (J)         4
 Shin Nihon Pro Wrestling Kounin: '95 Tokyo Dome Battle 7         (J)         4
 Smash Tennis / Super Family Tennis                               (E,J)       4
 Sporting News Power Baseball, The                                (E)         4
 Sterling Sharpe: End 2 End                                       (E)         4
 Street Hockey '95                                                (E)         4
 Street Racer                                                     (E,J)       4
 Sugoi Hebereke                                                   (J)         4
 Sugoro Quest++: Dicenics                                         (J)         4
 Super Bomberman                                                  (E,J)       4
 Super Bomberman: Panic Bomber World                              (J)         4
 Super Bomberman 2                                                (E,J)       4
 Super Bomberman 3                                                (E,J)       5
 Super Bomberman 4                                                (J)         5
 Super Bomberman 5                                                (J)         5
 Super Fire Pro Wrestling: Queen's Special                        (J)         ?
 Super Fire Pro Wrestling Special                                 (J)         ?
 Super Fire Pro Wrestling X                                       (J)         ?
 Super Formation Soccer 94: World Cup Edition                     (J)         ?
 Super Formation Soccer 95: della Serie A                         (J)         ?
 Super Formation Soccer 96: World Club Edition                    (J)         ?
 Super Formation Soccer II                                        (J)         ?
 Super Ice Hockey / Super Hockey '94                              (E,J)       ?
 Super Kyousouba: Kaze no Sylphid                                 (J)         ?
 Super Power League                                               (J)         ?
 Super Puyo Puyo Tsu: Remix                                       (J)         4
 Super Slam Dunk / Magic Johnson no Super Slam Dunk!              (E,J)       ?
 Super Tekkyuu Fight!                                             (J)         ?
 Super Tetris 3                                                   (J)         4
 Syndicate                                                        (E,F,G,J)   4
 Tiny Toon Adventures: Wacky Sports/Dotabata Daiundoukai          (E,J)       4
 Top Gear 3000 / Planet's Champ TG 3000, The                      (E,J)       4
 Vegas Stakes / Las Vegas Dream in Golden Paradise                (E,J)       4
 Virtual Soccer / J.League Super Soccer                           (E,J)       5
 Vs. Collection                                                   (J)         ?
 Wedding Peach                                                    (J)         3
 WWF Raw                                                          (E)         4
 Yuujin no Furi Furi Girls                                        (J)         4
 Zero 4 Champ RR                                                  (J)         ?
 Zero 4 Champ RR-Z                                                (J)         ?
```

Plus...

```text
 Momotaro Dentetsu Happy (1996) Hudson Soft (JP)
 Bomberman B-Daman: 4 players (via Battle)
 Kiteretsu Daihyakka - Choujikuu Sugoroku: 5 players
 J.League '96 Dream Stadium: 4 players
```

Corrections:

```text
 Elite Soccer/World Cup Striker: 5 players (Using the "Multiple Players"
   option while setting up for a game, you can assign different people
   to different controllers.)
 FIFA International Soccer: 5 players
 FIFA Soccer 96: This actually supports 5 players, not four. Pause the game
   and access the controllers with a multi-tap connected... up to five can
   join in.
 Madden NFL '94: 5 players
 Madden NFL '95: 5 players
 Madden NFL '97: 5 players
```

Does this really support the multi-tap?

```text
  Dino Dini's Soccer
  J.League Soccer Prime Goal
  NHL '95 through '97
```

<a id="snescontrollerssuperscopelightgun"></a>

### SNES Controllers SuperScope (Lightgun)

**SUPER SCOPE**

```text
            Front Sight              Sight Tube            Receiver (connect to
             /                 _______ /               ___ / controller slot 2)
            =--               |__.-.__|(       _______|___|_______
             \ \_______________/  __/         |  _______________  |
             /___.___________.___/            | |               | |
                 :           :                | |               | |
                 : Slot 1    : Slot 2         | |    TV SET     | |
                 :           :                | |               | |
                 :           :                | |               | |
                 V           V                | |_______________| |
                                              |___________________|
                                               |__|_|_|_|_|_|_|__|
       Transmitter
           /            Release            Pause
          /     Slot 1     / Slot 2  Fire   / Power Switch (Off/On/Turbo)
       _____      /       /   /        /   /  /
      \\____\____._______,___._______,,__,,_.._________
     ||             ^               \\\\\/////         \
     ||                                          _____/
     ||___      ________________________        /
          \     \\ <-Cursor           _/      /
           \     \                 __/      /
            \     \               \       /______
             \     \               \    _________\
              \_____\              /___/           <-- Shoulder Rest
```

**Batteries**

Takes six "AA" batteries. Which do reportedly last only for a few hours.

**Super Scope Bits**

```text
  1st          Fire Button   (0=High=Released, 1=Low=Pressed/Newly Pressed)
  2nd          Cursor Button (0=High=Released, 1=Low=Pressed)
  3rd          Turbo Switch  (0=Normal/PowerOn, 1=Turbo/PowerOn)
  4th          Pause Button  (0=High=Released, 1=Low=Newly Pressed)
  5th..6th     Extra ID Bits (always 0=High)
  7th          Offscreen     (0=High=Okay, 1=Low=CRT-Transmission Error)
  8th          Noise         (0=High=Okay, 1=Low=IR-Transmission Error)
  9th..12th    Extra ID Bits (always 1=Low)
  13th..16th   ID Bits       (always 1=Low=One) (0Fh)
  17th and up  Unused        (always 1=Low)
```

For obtaining the H/V position &amp; latch flag (Ports 213Ch,213Dh,213Fh), see:

[SNES PPU Timers and Status](30-ppu.md#snes-ppu-timers-and-status)

**Games compatible with the Super Scope**

```text
  Battle Clash (US) (EU) / Space Bazooka (JP) (1992) Nintendo
  Bazooka Blitzkrieg (US) (1992) Bandai
  Hunt for Red October (used for bonus games) (US) (EU) (JP)
  Lamborghini American Challenge (used in special game mode) (US) (EU) (1993)
  Lemmings 2 (US) (EU) (JP) (1994) Psygnosis (at game start: aim at crosshair)
  Metal Combat: Falcon's Revenge (includes OBC1 chip) (US) (1993)
  Operation Thunderbolt (US) (1994) Taito
  Super Scope 6 (bundled with the hardware) (Blastris & LazerBlazer) (1992)
  Terminator 2 - T2: The Arcade Game (US) (EU) (JP) (1993) Carolco/LJN
  Tin Star (US) (1994) Nintendo
  X-Zone (US) (EU) (1992) Kemco
  Yoshi's Safari (US) (EU) (JP) (1993) Nintendo
```

Moreover, the "SNES Test Program" (1991 by Nintendo) includes a Super Scope
test, and, reportedly, there's also a special Super Scope test cartridge (?)

**Notes**

The SuperScope has two modes of operation: normal mode and turbo mode. The
current mode is controlled by a switch on the unit, and is indicated by the 3rd
bit. Note however that the 3rd bit is only updated when the Fire button is
pressed (ie. the 1st bit is set). Thus, when you turn turbo on the 3rd bit
remains clear until you shoot, and similarly when turbo is deactivated the bit
remains set until you fire.

In either mode, the Pause bit will be set for the first strobe after the pause
button is pressed, and then will be clear for subsequent strobes until the
button is pressed again. However, the pause button is ignored if either cursor
or fire are down(?).

In either mode, the Cursor bit will be set while the Cursor button is pressed.

In normal mode, the Fire bit operates like Pause: it is on for only one strobe.
In turbo mode, it remains set as long as the button is held down.

When Fire/Cursor are set, Offscreen will be set if the gun did not latch during
the previous strobe and cleared otherwise (Offscreen is not altered when
Fire/Cursor are both clear).

If the Fire button is being held when turbo mode is activated, the gun sets the
Fire bit and begins latching. If the Fire button is being held when turbo mode
is deactivated, the next poll will have Fire clear but the Turbo bit will stay
set (because it isn't be updated until pressing fire the next time).

The PPU latch operates as follows: When Fire or Cursor is set, IOBit is set to
0 when the gun sees the TV's electron gun, and left a 1 otherwise. Thus, if the
SNES also leaves it one (bit7 of 4201h), the PPU Counters will be latched at
that point. This would also imply that bit7 of 4213h will be 0 at the moment
the SuperScope sees the electron gun.

Since the gun depends on the latching behaviour of IOBit, it will only function
properly when plugged into Port 2. If plugged into Port 1 instead, everything
will work except that there will be no way to tell where on the screen the gun
is pointing.

When creating graphics for the SuperScope, note that the color red is not
detected. For best results, use colors with the blue component over 75% and/or
the green component over 50%.

Data2 is presumably not connected, but this is not known for sure.

<a id="snescontrollerskonamijustifierlightgun"></a>

### SNES Controllers Konami Justifier (Lightgun)

```text
   -_______________________--
  |                :  :....: \_/\
  |________________:__:....:    /
   \________________\.:....; O  \_    <---- O = Start Button
                    |_ _____      |
                      \| ) |/\     \  <---- ) = Trigger
                       \___/  |     |
                              |     |
                              |_____|
               RJ12-socket ____/   \________ Cable (to SNES controller port)
            (for second gun)
```

```text
  Blue Gun --> connects to SNES (and has 6pin RJ12 socket for second gun)
  Pink Gun --> connects to 6pin RJ12 socket of first gun
```

**Justifier Bits**

```text
  1st..12th   Unused             (always 0=High)
  13th..16th  ID Bit3-0          (MSB first, 1=Low=One, always 0Eh = 1110b)
  17th..24th  Extra ID Bit7-0    (MSB first, 1=Low=One, always 55h = 01010101b)
  25th        Gun 1 Trigger      (1=Low=Pressed?)
  26th        Gun 2 Trigger      (1=Low=Pressed?)
  27th        Gun 1 Start Button (1=Low=Pressed?)
  28th        Gun 2 Start Button (1=Low=Pressed?)
  29th        Previous Frame was H/V-latching Gun1/2 (1=Low=Gun1, 0=High=Gun2)
  30th..32th  Unused             (always 0=High)
  33th and up Unused             (always 1=Low)
```

For obtaining the H/V position &amp; latch flag (Ports 213Ch,213Dh,213Fh), see:

[SNES PPU Timers and Status](30-ppu.md#snes-ppu-timers-and-status)

Note that the 29th bit toggles even when Gun2 is not connected.

**SNES Justifier Game(s)**

```text
  Lethal Enforcers (bundled with the hardware) (1993) Konami (US) (EU) (JP)
```

IOBit is used just like for the SuperScope. However, since two guns may be
plugged into one port, which gun is actually connected to IOBit changes each
time Latch cycles. Also note, the Justifier does not wait for the trigger to be
pulled before attempting to latch, it will latch every time it sees the
electron gun. Bit 6 of $213F may be used to determine if the Justifier was
pointed at the screen or not.

Data2 is presumably not connected, but this is not known for sure.

Nardz: "Actually when I was a kid, I bought the SNES Justifier Battle Clash
package. The Weird thing about it is that The package had A Sega Justifier in
it, but it came with this SNES/Sega Adapter, which pluged into your SNES and
the Sega Justifier would plug into the back of the connector." -- But, Battle
Clash is a Super Scope game, not a Justifier game???

The pinouts of the 6pin RJ12-socket are unknown.

**Sega Version**

There's also an identically looking Blue Gun for Sega (but with 9pin joystick
connector). The Pink Gun can be used with both SNES and Sega Blue Gun versions.

<a id="snescontrollersmacslightgun"></a>

### SNES Controllers M.A.C.S. (Lightgun)

**Multi-Purpose Arcade Combat Simulator (M.A.C.S.)**

This lightgun was used by the US Army to introduce beginners how to kill real
people. It was also shown on career days at high schools to win new recruits.
The hardware consists of a small lightpen attached to a M16 rifle. Software
cartridges exist for C64 and SNES.

```text
  Lightpen
    _____                                     ____....-----\  _
   |_____| #\\            ________________   |______________\//______________
  ___"__"__# -------""""""                |--|                               |
 |___#__#__#_                             |  |       Trigger  __             |
     "  "  # |_|------..._________________|--|_         ___  |  '--.__       |
                                               |       | \ \  \       '--.___|
                                               |_ _ _ _|_/_/   \
                                                 |     |   \    \
                                                 |_____|    \    \
                                                             |___/
```

**I/O Ports**

The lightgun connects to the lightpen input on 2nd controller port. Aside from
the HV-Latches, it uses only one I/O signal:

```text
  4017h.Bit0 Trigger Button (1=LOW=Pressed)
```

That is, only a single bit (no serial data transfer with CLK/STB signals). A
standard joypad attached to 1st controller port allows to calibrate the
lightpen (via Select button).

For obtaining the H/V position &amp; latch flag (Ports 213Ch,213Dh,213Fh), see:

[SNES PPU Timers and Status](30-ppu.md#snes-ppu-timers-and-status)

**SNES Software**

```text
  MACS Basic Rifle Marksmanship v1.1e (v1.2a) (1993) Sculptured Software (US)
  MACS Basic Rifle Marksmanship v1994.0 (1994?)
  MACS Moving Target Simulator (?) (1993) Sculptured Software (US)
```

Note: Version "1.1e" is displayed in the title screen, whilst the version
string at ROM offset 05819h identifies it as version "1.2a". The program code
looks crude and amateurish, and (as indicated by the corrupted ROM header) it
never passed through Nintendo's Seal of Quality process.

<a id="snescontrollerstwintap"></a>

### SNES Controllers Twin Tap

The Twin Tap from Partyroom21 (aka Yonezawa PR21) is a special controller for
8-player quiz games. The Twin Tap itself consists of 7pin SNES controller
connector with two cables, and a push-button on each cable-end (one button per
player). For the 8-player mode, four Twin Taps need to be connected to a
multiplayer adaptor (such like Partyroom21's own "Multi Adaptor Auto").

**The Twin Tap is supported by**

```text
  Shijou Saikyou no Quiz Ou Ketteisen Super (1992) TBS/Partyroom21/S'pal (JP)
```

**Transfer Protocol**

```text
  1st         Button 2 (or 4/6/8) (1=Low=Pressed) (would be "B" on joypads)
  2nd         Button 1 (or 3/5/7) (1=Low=Pressed) (would be "Y" on joypads)
  3rd..12th   Unknown
  13th..16th  Unknown (would be ID Bit3..0 on other SNES controllers)
  17th..24th  Unknown (would be Extended ID Bit7..0 on other SNES controllers)
  25th and up Unknown
```

Judging from disassembled game code, the 4bit ID might be 00h or 0Eh, in the
latter case, there &lt;should&gt; be also a unique Extended ID value.

<a id="snescontrollersmiraclepiano"></a>

### SNES Controllers Miracle Piano

**Miracle Piano Teaching System (The Software Toolworks)**

[SNES Controllers Miracle Piano Controller Port](#snes-controllers-miracle-piano-controller-port)

[SNES Controllers Miracle Piano MIDI Commands](#snes-controllers-miracle-piano-midi-commands)

[SNES Controllers Miracle Piano Instruments](#snes-controllers-miracle-piano-instruments)

[SNES Controllers Miracle Pinouts and Component List](#snes-controllers-miracle-pinouts-and-component-list)

```text
   __ _________________________________________________________ __
  |  |::::::::::::    MIRACLE      .. .. .. .. ..  ::::::::::::|  |
  |  |::::::::::::                 .. .. .. .. ..  ::::::::::::|  |
  |  |::::::::::::   #  #  #  #  :    .. .. .. ..  ::::::::::::|  |
  |  |::::::::::::   #  #  #  #  :       .. .. ..  ::::::::::::|  |
  |  |_________________________________________________________|  |
  |  | U U | U U U | U U | U U U | U U | U U U | U U | U U U | |  |
  |  | U U | U U U | U U | U U U | U U | U U U | U U | U U U | |  |Keys
  |  | | | | | | | | | | | | | | | | | | | | | | | | | | | | | |  |
  |__|_|_|_|_|_|_|_|_|_|_|_|_|_|_|_|_|_|_|_|_|_|_|_|_|_|_|_|_|_|__|
      C D E F G A H C D E F G A H C D E F G A H C D E F G A H C    Notes
     <-------------><------------><------------><------------>X    Octaves
         36..47         48..59    |   60..71        72..83    84   Key Numbers
                                  |
                               Middle C
  49 Piano Keys (29 White Keys, 20 Black Keys) (4 octaves, plus next higher C)
  1  Foot Pedal (Sustain)
  8  Push Buttons (Mode/Volume Selection)
```

<a id="snescontrollersmiraclepianocontrollerport"></a>

### SNES Controllers Miracle Piano Controller Port

**Miracle Controller Port Transfer**

Read Direction (invoked by SHORT Strobe signal)

```text
  1st         Data Present Flag (0=High=None, 1=Low=Yes)
  2nd..9th    Data Bit7..0      (MSB First, inverted 1=LOW=Zero)
  10th..12th  Unknown
  13th..16th  Unknown (would be ID Bit3..0 on other SNES controllers)
  17th and up Unknown
```

Write Direction (invoked by LONG Strobe signal, data output on STROBE line)

```text
  1st..8th    Data Bit7..0      (MSB First, 0=LOW=Zero)
```

Observe that read/write direction depends on length of initial Strobe signal
(so games that are reading joypad with other strobe-lengths might mess up
things).

10th bit and up (including the 4bit Controller ID) might be garbage (depending
on how the 8051 CPU in the keyboard handles the data transfer). However, with
appropriate timings, detecting a Miracle could be done via the "Firmware
version request" MIDI command.

Note: The NES and SNES Miracle software expects the piano keyboard connected to
Port 1, and a normal joypad connected to Port 2.

**miracle_recv_byte**

```text
  [004016h]=01h                             ;strobe on
  delay (strobe=1 for 102 master clks)      ;short delay = READ mode
  [004016h]=00h                             ;strobe off
  data_present_flag = [004016h].bit0        ;data present flag (1=LOW=Yes)
  for i=7 to 0
    data.bit(i)=NOT [004016h].bit0          ;data bits (MSB first, 1=LOW=Zero)
  next i
```

**miracle_send_byte**

```text
  [004016h]=01h                             ;strobe on (start bit)
  delay (strobe=1 for 528 master clks)      ;long delay = WRITE mode
  for i=7 to 0
    [004016h].bit0=data.bit(i)              ;data bits (MSB first, 1=HIGH=One)
    dummy=[004016h]                         ;issue short CLK pulse
  next i
  [004016h]=00h                             ;strobe off (stop/idle)
  delay (strobe=0 for min 160 master clks)  ;medium delay
```

<a id="snescontrollersmiraclepianomidicommands"></a>

### SNES Controllers Miracle Piano MIDI Commands

The Miracle is always using MIDI messages (no matter if the messages are
transferred through MIDI or RS232 or NES/SNES/Genesis controller cables). Below
lists the supported MIDI messages (including "Undocumented" messages, which are
used by the Miracle's SNES software, although they aren't mentioned in the
Miracle's Owner's Manual).

**MIDI Information Sent FROM/TO The Miracle keyboard**

```text
  Expl.                     Dir  Hex
  Note off (Undocumented)     W  8#h,<key>,00h   ;same as Note ON with velo=00h
  Note on/off command       R/W  9#h,<key>,<velo>
  Main volume level           W  B0h,07h,<vol>
  Sustain on/off command    R/W  B#h,40h,<flag>
  Local control on/off        W  B0h,7Ah,<flag>
  All notes off               W  B#h,7Bh,00h
  Patch change command (*)  R ?? C#h,<instr>     ;TO keyboard = Undocumented
  Miracle button action     R    F0h,00h,00h,42h,01h,01h,<bb>,F7h
  Unknown (Undocumented)      W  F0h,00h,00h,42h,01h,02h,<??>,F7h   ;???
  Keyboard buffer overflow  R    F0h,00h,00h,42h,01h,03h,01h,F7h
  Midi buffer overflow      R    F0h,00h,00h,42h,01h,03h,02h,F7h
  Firmware version request    W  F0h,00h,00h,42h,01h,04h,F7h
  Miracle firmware version  R    F0h,00h,00h,42h,01h,05h,<maj>,<min>,F7h
  Patch split command         W  F0h,00h,00h,42h,01h,06h,0#h,<lp>,<up>,F7h
  Unknown (Undocumented)      W  F0h,00h,00h,42h,01h,07h,F7h        ;???
  All LEDs on command         W  F0h,00h,00h,42h,01h,08h,F7h
  LEDs to normal command      W  F0h,00h,00h,42h,01h,09h,F7h
  Reset (Undocumented)        W  FFh
```

Direction: R=From keyboard, W=To keyboard

Notes: (*) Patch change FROM Keyboard is sent only in Library mode.

```text
  N#h         Hex-code with #=channel (#=0 from keyb, #=0..7 to keyb)
  <key>       Key (FROM Miracle: 24h..54h) (TO Miracle: 18h..54h/55h?)
  <velo>      Velocity (01h..7Fh, or 00h=Off)
  <vol>       Volume (00h=Lowest, 7Fh=Full)
  <flag>      Flag (00h=Off, 7Fh=On)
  <instr>     Instrument (00h..7Fh) for all notes
  <lp>        Instrument (00h..7Fh) for notes 24?/36-59, lower patch number
  <up>        Instrument (00h..7Fh) for notes 60-83/84?, upper patch number
  <maj>.<min> Version (from version 1.0 to 99.99)
  <bb>        button on/off (bit0-2:button number, bit3:1=on, bit4-7:zero)
```

Data from piano is always sent on first channel (#=0). Sending data to piano
can be done on first 8 channels (#=0..7), different instruments can be assigned
to each channel. Although undocumented, the SNES software does initialize 16
channels (#=0..0Fh), unknown if the hardware does support/ignore those extra
channels (from the instrument table: it sounds as if one could use 16
single-voice channels or 8 dual-voice channels).

<a id="snescontrollersmiraclepianoinstruments"></a>

### SNES Controllers Miracle Piano Instruments

**Available Patches (aka Instruments)**

The following patches are available through both Library Select Mode and MIDI
control:

```text
  000 Grand Piano     032 Marimba         064 Synth Bells    096 Tube Bells'
  001 Detuned Piano   033 Glockenspiel'   065 Vox 1          097 Frogs/Ducks
  002 FM Piano        034 Kalimba'        066 Vox 2          098 Banjo'
  003 Dyno            035 Tube Bells      067 Vox 3          099 Shakuhachi'
  004 Harpsichord     036 Steel Drums     068 Mod Synth      100 Piano'
  005 Clavinet        037 Log Drums'      069 Pluck Synth    101 Vibraphone'
  006 Organ           038 Strings 1       070 Hard Synth     102 FM Piano'
  007 Pipe Organ      039 Pizzicato       071 Syntar         103 Clock Belis'
  008 Steel Guitar    040 Strings 2       072 Effects 1 *    104 Harpsichord'
  009 12-StringGuitar 041 Violin 1'       073 Effects 2 *    105 Clavinet'
  010 Guitar          042 Trumpet'        074 Percussion 1 * 106 Organ'
  011 Banjo           043 Trumpets        075 Percussion 2 * 107 Pipe Organ'
  012 Mandolin        044 Horn'           076 Percussion 3 * 108 Metal Guitar'
  013 Koto'           045 Horns           077 Sine Organ'    109 Stick'
  014 Jazz Guitar'    046 Trombone'       078 Organ #        110 Guitar'
  015 Clean Guitar'   047 Trombones       079 Pipe Organ #   111 Xylophone'
  016 Chorus Guitar   048 CupMuteTrumpet' 080 Harpsichord #  112 Marimba'
  017 Fuzz Guitar     049 Sfz Brass 1     081 Synth Pad 1    113 Syn Trombone'
  018 Stop Guitar     050 Sfz Brass 2     082 Synth Pad 2    114 Syn Trumpet'
  019 Harp'           051 Saw Synth       083 Synth Pad 3    115 Sfz Brass 1'
  020 Detuned Harp    052 Tuba'           084 Synth Pad 4    116 Sfz Brass 2'
  021 Upright Bass'   053 Harmonica       085 Synth Pad 5    117 Saw Synth'
  022 Slap Bass'      054 Flute'          086 Synth Pad 6    118 Church Bells'
  023 Electric Bass'  055 Pan Flute'      087 Synth Pad 7    119 Marcato'
  024 Moog            056 Calliope        088 Synth Pad 8    120 Marcato
  025 Techno Bass     057 Shakuhachi      089 Synth Pad 9    121 Violin 2'
  026 Digital Waves   058 Clarinet'       090 Synth Pad 10   122 Strings 3
  027 Fretless Bass'  059 Oboe'           091 Synth Pad 11   123 Synth Bells'
  028 Stick Bass      060 Bassoon'        092 Synth Pad 12   124 Techno Bass'
  029 Vibraphone      061 Sax'            093 Synth Pad 13   125 Mod Synth'
  030 MotorVibraphone 062 Church Bells    094 Synth Pad 14   126 Pluck Synth'
  031 Xylophone       063 Big Bells       095 Synth Pad 15   127 Hard Synth'
```

Notes:

```text
 ' These programs are single voice, which lets The Miracle play up to 16
   notes simultaneously. All other programs are dual voice, which lets it
   play up to 8 notes simultaneously.
 * 072..076 See below for a list of Effects/Percussion sounds.
 # 078..080 To be true to the nature of the sampled instrument, these patches
   do not respond to velocity.
```

**Effects and Percussion Patches**

When selecting instruments 072..076 (Effects 1-2 and Percussion 1-3), a number
of different sounds are mapped to each six keyboard keys/notes:

```text
  Note      Effects 1   Effects 2    Percussion 1   Percussion 2   Percussion 3
  30-35     Jet         Yes (ding)   -              -              Ratchet
  36-4l     Gunshot     No (buzz)    Kick Drum      Rim Shot       Snap 1
  42-47     RoboDeath   Applause     Snare          Exotic         Snap 2
  48-53     Whoosh      Dogbark      Toms           Congas         Dripdrum 1
  54-59     Punch       Door creak   Cymbal         Timbale        Dripdrum 2
  60-65     Slap        Door slam    Closed Hat     Cowbell        Wet clink
  66-71     Duck        Boom         Open Hat       Bongos         Talk Drum
  72-77     Ow! 1       Car skid     Ride           Whistle        Agogo
  78-83     Ow! 2       Goose        Shaker         Clave          Explosion
```

Note: The piano keys are numbered 36..84 (so notes 30..35 can be used only
through MIDI messages, not via keyboard).

<a id="snescontrollersmiraclepinoutsandcomponentlist"></a>

### SNES Controllers Miracle Pinouts and Component List

**25pin SUBD connector (J6)**

```text
  1  PC/Amiga/Mac RS232 GND (also wired to RTS)
  2  PC/Amiga/Mac RS232 RxD
  3  PC/Amiga/Mac RS232 TxD
  7  NES/SNES/Genesis GND
  10 NES/SNES/Genesis Data
  13 NES/SNES/Genesis Strobe
  14 Sense SENSE0 (0=MIDI Output off, 1=MIDI Output on)
  15 Sense SENSE1 (0=9600 Baud; for RS232, 1=31250 Baud; for MIDI)
  19 NES/SNES/Genesis Clock
  all other pins = not connected
```

For PC/Mac RS232 wire SENSE0=GND, SENSE1=GND

**Miracle NES and SNES Cartridges**

According to the ROM Headers: The SNES cartridge contains 512Kbyte Slow/LoROM,
and no SRAM (nor other storage memory). The NES cartridge contains MMC1 mapper,
256Kbyte PRG-ROM, 64Kbyte CHR-ROM, and no SRAM (nor other storage memory).

**Miracle Piano Component List (Main=Mainboard Section, Snd=Sound Engine)**

```text
  U1   Snd  16pin TDA7053 (stereo amplifier for internal speakers)
  U2   Snd   8pin NE5532 (dual operational amplifier)
  U3   Snd  16pin LM13700 or LM13600 (unclear in schematic) (dual amplifier)
  U4   Snd  14pin LM324 (quad audio amplifier)
  U5   Main  3pin LM78L05 (converts +10V to VLED, supply for 16 LEDs)
  U6   Main 14pin 74LS164 serial-in, parallel-out (to 8 LEDs)
  U7   Main 14pin 74LS164 serial-in, parallel-out (to another 8 LEDs)
  U8   Main  5pin LM2931CT (converts +12V to +10V, and supply for Power LED)
  U9   Main  3pin LM78L05 (converts +10V to +5REF)
  U10  Snd  14pin TL084 (JFET quad operational amplifier)
  U11  Snd  40pin J004 (sound chip, D/A converter with ROM address generator)
  U12  Snd  32pin S631001-200 (128Kx8, Sound ROM for D/A conversion)
  U13  Main  3pin LM78L05 (converts +10V to VCC, supply for CPU and logic)
  U14  Main 40pin AS0012 (ASIC) Keyboard Interface Chip (with A/D for velocity)
  U15  Main 40pin 8032 (8051-compatible CPU) (with Y1=12MHz)
  U16  Snd  40pin AS0013 (ASIC)
  U17  Main 28pin 27C256 EPROM 32Kx8 (Firmware for CPU)
  U18  Main 28pin 6264 SRAM 8Kx8 (Work RAM for CPU)
  U19  Main 16pin LT1081 Driver for RS232 voltages
  U20  Main  8pin 6N138 opto-coupler for MIDI IN signal
  S1-8 Main  2pin Push Buttons
  S9   Main  3pin Power Switch (12V/AC)
  J1   Main  3pin 12V AC Input (1 Ampere)
  J2   Main  2pin Sustain Pedal Connector (polarity is don't care)
  J3   Snd   2pin RCA Jack Right
  J4   Snd   2pin RCA Jack Left
  J5   Snd   5pin Headphone jack with stereo switch (mutes internal speakers)
  J6   Main 25pin DB25 connector (RS232 and SNES/NES/Genesis controller port)
  J7   Main  5pin MIDI Out (DIN)
  J8   Main  5pin MIDI In (DIN)
  JP1  Main 16pin Keyboard socket right connector
  JP2  Main 16pin Keyboard socket left connector
  JP3  Snd   4pin Internal stereo speakers connector
```

Note: The official original schematics are released &amp; can be found in
internet.

<a id="snescontrollersnttdatapadjoypadwithnumerickeypad"></a>

### SNES Controllers NTT Data Pad (joypad with numeric keypad)

Special joypad with numeric keypad, for use with SFC Modem:

[SNES Add-On SFC Modem (for JRA PAT)](#snes-add-on-sfc-modem-for-jra-pat)

**NTT Data Controller Pad - 27-buttons (including the 4 direction keys)**

```text
               _______________
    __--L--___/               \__--R--__
   /    _        (<)(>)   (=)           \
  |   _| |_   (1)(2)(3)(*)(C)     (X)    |
  |  |_   _|  (4)(5)(6)(#)(.)  (Y)   (A) |
  |    |_|    (7)(8)(9)( 0  )     (B)    |
  |                                      |
   \__________.---------------._________/
```

**NTT Data Controller Pad Bits**

```text
  1st   Bit31  Button (B)        (0=High=Released, 1=Low=Pressed)
  2nd   Bit30  Button (Y)        (0=High=Released, 1=Low=Pressed)
  3rd   Bit29  Button (<) Select (0=High=Released, 1=Low=Pressed)
  4th   Bit28  Button (>) Start  (0=High=Released, 1=Low=Pressed)
  5th   Bit27  Direction Up      (0=High=Released, 1=Low=Pressed)
  6th   Bit26  Direction Down    (0=High=Released, 1=Low=Pressed)
  7th   Bit25  Direction Left    (0=High=Released, 1=Low=Pressed)
  8th   Bit24  Direction Right   (0=High=Released, 1=Low=Pressed)
  9th   Bit23  Button (A)        (0=High=Released, 1=Low=Pressed)
  10th  Bit22  Button (X)        (0=High=Released, 1=Low=Pressed)
  11th  Bit21  Button (L)        (0=High=Released, 1=Low=Pressed)
  12th  Bit20  Button (R)        (0=High=Released, 1=Low=Pressed)
  13th  Bit19  ID Bit3           (always 0=High)
  14th  Bit18  ID Bit2           (always 1=Low)
  15th  Bit17  ID Bit1           (always 0=High)
  16th  Bit16  ID Bit0           (always 0=High)
  17th  Bit15  Button (0)        (0=High=Released, 1=Low=Pressed)
  18th  Bit14  Button (1)        (0=High=Released, 1=Low=Pressed)
  19th  Bit13  Button (2)        (0=High=Released, 1=Low=Pressed)
  20th  Bit12  Button (3)        (0=High=Released, 1=Low=Pressed)
  21th  Bit11  Button (4)        (0=High=Released, 1=Low=Pressed)
  22th  Bit10  Button (5)        (0=High=Released, 1=Low=Pressed)
  23th  Bit9   Button (6)        (0=High=Released, 1=Low=Pressed)
  24th  Bit8   Button (7)        (0=High=Released, 1=Low=Pressed)
  25th  Bit7   Button (8)        (0=High=Released, 1=Low=Pressed)
  26th  Bit6   Button (9)        (0=High=Released, 1=Low=Pressed)
  27th  Bit5   Button (*)        (0=High=Released, 1=Low=Pressed)
  28th  Bit4   Button (#)        (0=High=Released, 1=Low=Pressed)
  29th  Bit3   Button (.) Dot    (0=High=Released, 1=Low=Pressed)
  30th  Bit2   Button (C) Clear  (0=High=Released, 1=Low=Pressed)
  31th  Bit1   Unknown/Unused    (unknown, probably always 1 or always 0)
  32th  Bit0   Button (=) End    (0=High=Released, 1=Low=Pressed)
  33th and up  Padding           (unknown, probably always 1 or always 0)
```

Note: The "(=)" button is sunken, somewhat preventing accidently pressing it.

<a id="snescontrollersxbandkeyboard"></a>

### SNES Controllers X-Band Keyboard

The X-Band keyboard is a (rare) optional add-on for the X-Band Modem, intended
to allow faster chatting/mailing as with the joypad controlled on-screen
keyboard.

**Keyboard Layout**

The keyboard has a black case, 84 keys, an X-Band logo in upper left, and
connection cable (to SNES controller port) attached in upper-right.

```text
   ___________________________________________________________________
  |   ><                                       Num  Caps Scroll       |
  |  BAND                                      Lock Lock Lock         |
  |  ___ ___ ___ ___ ___ ___ ___ ___ ___ ___ ___ ___ ___ ___ ___ ___  |
  | |   |   |   |   |   |   |   |   |   |   |   |   | L |Sel|Sta| R | |
  | |___|___|___|___|___|___|___|___|___|___|___|___|___|___|___|___| |
  | |Can|1! |2@ |3# |4$ |5% |6^ |7& |8* |9( |0) |-_ |=+ | <--   | X | |
  | |___|___|___|___|___|___|___|___|___|___|___|___|___|_______|___| |
  | |Switc| Q | W | E | R | T | Y | U | I | O | P |[{ |]} |     | Y | |
  | |_____|___|___|___|___|___|___|___|___|___|___|___|___|     |___| |
  | |Caps  | A | S | D | F | G | H | J | K | L |;: |'" | Enter  | A | |
  | |______|___|___|___|___|___|___|___|___|___|___|___|________|___| |
  | |Shift   | Z | X | C | V | B | N | M |,< |.> |/? |Shift |UP | B | |
  | |________|___|___|___|___|___|___|___|___|___|___|______|___|___| |
  | |`~ |   |>< |Ctr|                       |Ctr|>< |\| |LT |DN |RT | |
  | |___|___|___|___|_______________________|___|___|___|___|___|___| |
  |___________________________________________________________________|
```

**Normal Controller Access (via STB and DATA0)**

```text
  1st-12th    Unknown/unused
  13th-16th   Unknown/unused (should be usually a 4bit ID)
  17th-24th   Unknown/unused (should be sometimes extended 8bit ID)
  25th and up Unknown/unused
```

Note: If the keyboard data is transferred in sync with STB, then "17th and up"
are the LSBs of the 2bit keyboard data pairs (though it might also be in sync
with falling IOBIT, rather than with STB).

**Keyboard Access (read_scancodes) (via IOBIT and DATA0 &amp; DATA1)**

Below might be required to be preceeded by reading normal 16bit controller data
(eg. via auto-joypad-reading) (ie. unknown if below needs leading STB signal,
and preceeding 16xCLK signals).

```text
  [004201]=7Fh                 ;-set IOBIT=0
  id=getbits(8)                ;-read ID byte (must be 78h, aka "x")
  if CAPS=OFF [004201]=FFh     ;-set IOBIT=1 (when CAPS=OFF) (CAPS LED)
  num=getbits(4)               ;-read num scancodes
  if num>0 then for i=1 to num ;\
    [dst]=getbits(8)           ; read that scancodes (if any)
    dst=dst+1                  ;
  next i                       ;/
  [004201]=FFh  ;set IOBIT=1   ;-set IOBIT=1
```

Note: When reading the ID bits, BIOS sets IOBIT=1 after reading the first 2bit
group (purpose unknown). When reading ONLY the ID (without reading following
scancodes), then the scancodes do remain in the keyboard queue.

**getbits(n)**

```text
  for i=1 to n/2                       ;\
    delay (loop 7 times or so)         ; read 2bits at once, LSB first
    x=(x SHR 2) OR ([004017h] SHL 6)   ;
  next                                 ;/
  x=(x XOR FFh) SHR (8-n)              ;-invert & move result to LSBs
```

**Scancode Summary**

```text
  nn               normal key
  F0h,nn           normal key released
  E0h,nn           special key
  E0h,F0h,nn       special key released
```

**Normal Scancodes (without prefix)**

```text
  ____0xh____1xh___2xh___3xh___4xh___5xh___6xh___7xh_____8xh_____9xh___
  x0h ---    ---   ---   ---   ---   ---   ---   NUM-0   ---     <90h>?
  x1h ---    CTR1? C     N     ,<    ---   ---   NUM-.   ---     <91h>?
  x2h ---    SHF1? X     B     K     '"    ---   NUM-2   ---     <92h>?
  x3h ---    ---   D     H     I     ---   ---   NUM-5   ---     <93h>?
  x4h ---    CTR2? E     G     O     [{    ---   NUM-6   NUM-SUB <94h>?
  x5h ---    Q     4$    Y     0)    =+    ---   NUM-8   ---     <95h>?
  x6h ---    1!    3#    6^    9(    ---   BS    CANCEL  JOY-A   <96h>?
  x7h ---    ---   ---   ---   ---   ---   ---   NUM-DIV JOY-B   <97h>?
  x8h ---    ---   ---   ---   ---   CAPS  ---   ---     JOY-X   <98h>?
  x9h ---    ---   SPACE ---   .>    SHF2? NUM-1 NUM-RET JOY-Y   <99h>?
  xAh ---    Z     V     M     /?    ENTER ---   NUM-3   JOY-L   <9Ah>?
  xBh ---    S     F     J     L     ]}    NUM-4 ---     JOY-R   <9Bh>?
  xCh ---    A     T     U     ;:    \|    NUM-7 NUM-ADD SELECT  <9Ch>?
  xDh SWITCH W     R     7&    P     \|    ---   NUM-9   START   <9Dh>?
  xEh `~     2@    5%    8*    -_    ---   ---   NUM-MUL <8Eh>?  ---
  xFh ---    ---   ---   ---   ---   ---   ---   ---     <8Fh>?  ---
```

**Special Scancodes (with E0h-prefix)**

```text
  E0h,5Ah  JOY-A (alternate to normal scancode 86h)
  E0h,6Bh  LEFT
  E0h,72h  DOWN
  E0h,74h  RIGHT
  E0h,75h  UP
```

**Notes**

The Numeric-Keypad (NUM) isn't present on the existing X-Band keyboard. There
is probably only one of the two backslash keys (5Ch/5Dh) and only one of the
two Button-A keys (86h/E0h,5Ah) implemented.

There are three keyboard LEDs (Num,Caps,Scroll) visible in upper right on some
photos (not visible on other photos; either due to bad photo quality, or maybe
some keyboards have no LEDs). The Caps LED is controlled via software (unknown
if Num/Scroll LEDs can be controlled, too).

There are several "unused" keys (which aren't used by the BIOS), unknown
if/which scancodes are assigned to them (12 noname keys in upper left, 1 noname
key in lower left, two control keys, and two xband logo keys). For the two
shift keys it's also unknown which one has which scancode.

Unknown if the japanese BIOS includes support for japanese symbols, and unknown
if there was a keyboard released in japan. (Note: With an emulated US keyboard,
the Japanese BIOS does realize cursor/enter keys, it does also store typed
ASCII characters in a ring-buffer at [3BB6+x]; but, for whatever reason, does
then ignore those characters).

<a id="snescontrollerstiltmotionsensors"></a>

### SNES Controllers Tilt/Motion Sensors

There are a few SNES controllers with Tilt/Motion Sensors, most or all of them
are emulating normal SNES joypad button/direction signals, which is making them
compatible with existing games, but also means that the SNES receives only
digital data (pressed/released) rather than anlogue (slow/fast) data.
Alltogether, the controllers appear to be more uncomfortable than useful.

**BatterUP (1994) (Sports Sciences Inc.)**

The BatterUP is a 24-inch foam-covered plastic baseball bat for Sega Genesis
and Super Nintendo. Reportedly, it "doesn't sense swing speed or location, only
timing" (whatever that means, probably simulating a button-pressed signal at
the time when swinging the bat). Aside from the swing/motion sensor, the middle
of the bat contains joypad buttons.

```text
    ___
   /   \     BatterUP       _________________
  |     |                  |             []  |  Some versions (probably for
  |     |                  |        [] START |  SEGA or so) have a "C" button,
  |     | <-- blue foam   .|        UP       |  and no X/Y/SELECT buttons.
  |     |               .' |  []         []  |
  |     |             .'   | LEFT      RIGHT |  Purpose of the 4 DIP switches
  |_____|   .........'     | ::::   []       |  is unknown (maybe sensitivity,
  |  .' |                  | DIPs  DOWN  []  |  or assigning the swing-sensor
  | '.' |     DPAD         |           SELECT|  to a specific joypad-button?)
  |  .' | <-- buttons -->  |       A[]       |
  |  :  |                  |                 |
   \_'_/    .........      |       B[]       |
   |   |             '.     \               /
   |   |               '.    \     X[]     /
   |   |  <-- handle     '.   \           /
   |   |                       \   Y[]   /
   |   |                        \_______/
   |   |
    \_/
     '.______ cable (to console's controller port)
```

Games compatible with the SNES version (according to instruction manual?):

```text
  Cal Ripken Jr. Baseball, 1992 Mindscape
  ESPN Baseball Tonight, 1994 Sony Imagesoft
  Hardball III, 1994 Accolade
  Ken Griffey Jr. Presents Major League Baseball, 1994 Nintendo
  Ken Griffey, Jr.'s Winning Run, 1996 Nintendo
  MLBPA Baseball, 1994 EA Sports
  Sports Illustrated Championship Football and Baseball, 1993 Malibu Games
  Super Baseball, 1994 EA Techmo
  Super Batter Up, 1993 Namco
```

**TeeV Golf (1993/1995) (Sports Sciences Inc.)**

The TeeV Golf hardware consists of a wireless (battery-powered) golf club, and
a rectangular box which is supposed to be set on the floor. There's a mimmicked
(half) golf ball in the middle of the box. According to photos, there are two
rows of six "red dots" on the box (these might be nonfunctional decorative
elements, or important LEDs/sensors for motion tracking?), some kind of a BIOS
cartridge or so (which seems to contain something customized for specific
games), and two connection cables (one to the consoles controller port, and one
forwarded to a joypad).

```text
       .......................................
                 cables           ___________:___________
                 (to joypad      | O   O   O   O   O   O |__
                 and to          |        .'''''.  TeeV  :  | <-- BIOS
                 console)        |       :   _   : Golf  :  |     cartridge
   Golf Club                     |       :  (_)  :       :__|     or so
   (with 2 AA batteries)         |       :.......:       |        (with "PGA
   ___________                   | O   O :.O   O.: O   O |        Tour Golf"
  |          :\ __               |Mode1  : ''''' :  Mode2|        text on it)
  |          : :  |              |_______:_______:_______|
  |          : :  |                        _____________________
  |          : :__|=======================|_____________________|
  |__________:/
```

According to the box: The TeeV SNES version is compatible with PGA Tour Golf
(unknown if that refers to the whole PGA series, and unknown if there are other
games supported; other games might possibly require other "BIOS" cartridges).
The PGA Tour Golf BIOS cartridge does probably translate motion data to a
specially timed sequence of joypad button pressed/released signals.

There are TeeV versions for SNES, Sega Genesis, and PC. The US Patent number
for the TeeV hardware is 4971325 (with the addition of "other patents
pending").

**StuntMaster (VictorMaxx)**

Advertised as "3-D Virtual Reality Headset".

"Despite what the box says, the StuntMaster VR is not a 3D display. It contains
one extremely grainy low resolution LCD screen in the center of the goggles. If
you put it on, it hurts your face. The display singes your retinas with an
intensely fuzzy, hard-to-focus-on image. The head tracking mechanism is nothing
more than a stick you clip to your shoulder (see picture above) which slides
through a loop on the side of the headset. When you turn your head, the
StuntMaster detects the stick sliding in the loop and translates this into a
left or right button press on a control pad, assuming you've actually hooked it
up to the controller port of your SNES or Genesis. Remember the "point-of-view
instantly scrolls or rotates with the turn of your head" quote? I'd love to see
that happen in Super Mario World. Obviously, it couldn't actually work unless
the game were programmed for that functionality in advance. Unless, of course,
you're playing Doom and you want to turn left or right by moving your head."

```text
  1  +6V
  2  GND
  3  Joypad (SNES:DTA, SEGA:Right In)
  4  Joypad (SNES:CLK, SEGA:N/A)
  5  Joypad (SNES:STB, SEGA:Left In)
  6  GND
  7  VCC (SNES:+5V, SEGA:N/A?) (for Joypad?)
  8  N/A
  9  GND
  10 Video in (NTSC composite)
  11 Joypad (SNES:DTA, SEGA:Right Out)
  12 Joypad (SNES:STB, SEGA:Left Out)
  13 GND
  14 Audio in (Left)
  15 Audio in (Right)
 Resolution:     240x86 color triads
 Field of View:  17 degrees
 Weight:         circa 2.5 pounds
```

**Nordic Quest (interactive ski-exerciser) (Nordic Track)**

The Nordic Quest is an add-on for treadmills (walking-exercising machines) from
Nordic Track. Unlike normal treadmills, the Nordic Track one features two
handles attached to a string, which the user pulls back and forth during
exercising (similar to nordic walking/skiing sticks).

The Nordic Quest includes replacement handles with DPAD (left handle) and
buttons (right handle), allowing to play "virtually any" joypad controlled
games during exercising; there aren't any special "nordic" games for the
controller, instead it can be used for games like golf, car-racing, and flight
simulations (as illustrated on the box).

The exercising intensity is claimed to affect the game speed - unknown how this
works - maybe by toggling the DPAD on/off, or maybe by toggling the Start
(pause) button on/off?

**JS-306 Power Pad Tilt (Champ) (joypad with autofire, slowmotion, tilt-mode)**

A regular joypad with normal DPAD, the tilt-sensors can be optionally used
instead of the DPAD.

**Nigal Mouncefill Fly Wheel (Logic 3) (wheel-shaped, tilt-sensor instead dpad)**

An odd wheel-shaped controller with tilt-sensors instead of DPAD.

<a id="snescontrollerslasabirdiegolfclub"></a>

### SNES Controllers Lasabirdie (golf club)

The Lasabirdie is a golf club made in 1995 by Ricoh. Supported by only one game
(which came shipped with the device):

```text
  Lasabirdie - Get in the Hole (1995) Ricoh/Good House (JP)
```

**Lasabirdie "Golf Mat" and "Golf Club"**

```text
     _______________________________                       __
    |...............................|      insert two     / /
    |:                             :|      AA batteries  / / <-- handle
    |: #      Golf Ball          # :|            |      / /      (rather
    |: #   _ /                   # :|            |     / /       short)
    |: #  (_)       #            # :|            V    / /
    |: #                         # :|          ______/ /
    |: #                         # :|         |       /
   _|:.............................:|          \     / <-- yellow laser symbol
  / |  LT RT   UP DN   A  B   RICOH |           \___/
  | |_______________________________|             |
  / <---- cable (to SNES controller port 2)       |  <-- laser beam
```

The so-called Golf Mat is actually a (not very flat) plastic box, with a
mimmicked (half) golf ball mounted on it, the three black fields (shown as ###
in the ASCII drawing) might contain laser sensors, the front panel has six
buttons: Left/Pause, Right, Up, Down, A/Start and B/Cancel.

For additional/better menu controls, one can connect a normal joypad to SNES
port 1.

Below describes the overall transfer protocol. Unknown if/how/what kind of
motion, speed, and/or direction information is transmitted via that protocol.

**Lasabirdie Controller Data (connected to Port 2)**

```text
  1st         Button B (CANCEL)    (1=Low=Pressed)
  2nd         Button DOWN          (1=Low=Pressed)
  3rd..6th    Nibble Data bit3-0   (1=Low=One?) (MSB first)
  7th         Nibble Available     (toggles CLK like)
  8th         Packet Available     (1=Low=Yes)
  9th         Button A (START)     (1=Low=Pressed)
  10th        Button UP            (1=Low=Pressed)
  11th        Button LEFT (PAUSE)  (1=Low=Pressed)
  12th        Button RIGHT         (1=Low=Pressed)
  13th..16th  ID Bit3-0       (unknown)  ;read, but not checked by software
  17th..24th  Extra ID Bit7-0 (unknown)  ;read, but not checked by software
  25th and up Unknown/Unused (probably all one, or all zero ?)
```

**Command Bytes**

Command bytes are used to select a specific packet, and (during the packet
transfer) to select nibbles within the previously selected packet:

```text
  20h        select "GOLF_READY!" ID string packet
  22h        select version string packet
  30h        select whatever data packet?
  3Fh        select whatever data packet?
  40h..55h   sent while receiving nibbles number 0..21
  5Fh        terminate transfer (or re-select default packet type?)
```

Bytes are output via Port 4201h at a fixed baudrate of circa 10000 bits/second:

```text
  output 1 start bit ("0")
  output 8 data bits (MSB first)
  output 2 stop bits ("0","0")
  release line (output "1", until the next byte transferred in next frame)
```

Exact time per bit is 2140 master cycles (10036 bps at 21.47727MHz NTSC clock).

**Packets**

Packets consist of 11 bytes, transferred in 22 nibbles (of 4bit each). For
whatever reason, the software receives only one nibble per frame, so a complete
packet-transfer takes about 0.36 seconds. The bits are transferred MSB first
(bit3,bit2,bit1,bit0), whilst nibbles are transferred LSB first (bit3-0,
bit7-4). The 11-byte packets can contain following data:

```text
  "GOLF_READY!"                 ;-ID-string packet  ;\without checksum
  FFh,FFh,0,0,0,0,0,0,0,0,0     ;-Empty packet      ;/
  9 chars, 1 unknown, 1 chksum  ;-Version-string    ;\with checksum
  10 data bytes, 1 chksum       ;-Normal packet     ;/
```

The checksum (if it is present) is calculated by summing up all 10 data bytes,
and adding MSB+LSB of the resulting 16bit sum (ie. sum=sum+sum/100h). The
version string packet contains 9 characters (unknown content), one unused byte
(unknown value), and the checksum byte. Other packet(s) contain whatever
controller/motion data (unknown content).

Below is the procedure for receiving a packet (before doing that, one should
first select a packet, eg. send_byte(20h) for receiving the ID string).

```text
  if [421Ah].bit8 = 0 then exit       ;-exit if no packet available
  for i=0 to 21
    old_state = [421Ah].bit9
   @@wait_lop:
    send_byte(40h+i)
    wait_vblank
    if [421Ah].bit9 <> old_state then jmp @@wait_done
    wait_vblank
    if [421Ah].bit9 <> old_state then jmp @@wait_done
    jmp @@wait_lop
   @@wait_done:
    nibble=([421Ah] SHR 10) AND 0Fh
    if (i AND 1)=0 then buf[i/2]=nibble, else buf[i/2]=buf[i/2]+nibble*10h
  next i
  send_byte(5Fh)                ;-terminate transfer or so
```

<a id="snescontrollersexertainmentbicycleexercisingmachine"></a>

### SNES Controllers Exertainment (bicycle exercising machine)

The Exertainment is an exercising machine made by Life Fitness. It consists of
a stationary bicycle, a monitor with TV tuner, a SNES game cartridge, and a
SNES console with some extra hardware plugged into its Expansion Port.

**Technical Info**

[SNES Controllers Exertainment - I/O Ports](#snes-controllers-exertainment-io-ports)

[SNES Controllers Exertainment - RS232 Controller](#snes-controllers-exertainment-rs232-controller)

[SNES Controllers Exertainment - RS232 Data Packets &amp; Configuration](#snes-controllers-exertainment-rs232-data-packets-configuration)

[SNES Controllers Exertainment - RS232 Data Packets Login Phase](#snes-controllers-exertainment-rs232-data-packets-login-phase)

[SNES Controllers Exertainment - RS232 Data Packets Biking Phase](#snes-controllers-exertainment-rs232-data-packets-biking-phase)

**Drawings**

[SNES Controllers Exertainment - Drawings](#snes-controllers-exertainment-drawings)

**Supported Games**

```text
  Cannondale Cup (1993) CEG/American Softworks/RadicalEntertainment (US)
  Exertainment Mountain Bike Rally (1994) LifeFitness/RadicalEntertainment (US)
  Exertainment Mountain Bike Rally & Speed Racer (combo cart) (1995) (USA)
  Exertainment Mountain Bike Rally & Speed Racer (combo cart) (prototype) (EU)
```

Aside from the games, all three Exertainment cartridges are including a
"Program Manager", allowing to view/edit user profiles, and, in the old cart
from 1994 only - also including some "mini games" called Workout and Fit Test).

Cannondale Cup is essentially same as Mountain Bike Rally, it is sending
Exertainment packets (including for checking for the "LIFEFITNES(s)" ID), but
it lacks the Program Manager, and even the actual game doesn't seem to react to
actions on the exertainment hardware(?).

Playing normal SNES games during exercising isn't supported (the SNES cartridge
slot isn't externally accessible, and, selecting the pedal resistance requires
special program code in the game cartridge).

The Mountain Bike game works with/without the Exertainment hardware (with the
Exertainment features being shown only if the hardware is present).

**Joypad Controls**

Joypad like controls are attached to the handlebars, featuring the same 12bit
button/direction signals as normal joypads. In fact, there should be two such
joypads, both mapped as "player 1" input (ie. both wired to Port 4218h).
Turning the handlebars isn't possible, instead, steering is done via DPAD
Left/Right buttons. Whereas, for the Mountain Bike game, steering is needed
only for gaining optional bonus points.

**Other Controls**

Pedaling speed/force info is probably sent via Port 21C0h Data Packets. The
front panel has six buttons - unknown if the buttons states are sent to the
SNES - Volume &amp; Program Up/Down and Picture-in-picture are possibly wired
directly to the TV set unit, the Menu button is possibly wired to SNES Reset
signal(?)

**Exertainment Expansion Port Unit - PCB Component List info from byuu**

```text
  U1 40pin TL16C550AN CF62055 N9304 2342265 TI  ;-RS232 controller
  U2 20pin PEEL18CV8P CTM22065 333FB    ;-some PAL (sticker "K41A-12802-0000")
  U3 20pin 74HC374N (addr.msb & serial) ;\two 8bit latches (13bit SRAM address,
  U4 20pin 74HC374N (addr.lsb)          ;/and the 3bit serial-port outputs)
  U5 28pin LH5268A-10LL 9348 SHARP      ;-8Kx8 SRAM
  U6 16pin ADM232LJN 9403 OF31824       ;-RS232 voltage converter
  BATT1    CR2032                       ;-battery (for SRAM)
  P2 4pin  short cable to rear 623K-6P4C;-SIO (1=LED?, 2=CLK?, 3=DTA?, 4=/SEL?)
  P3 3pin  long cable to front 616M-4P4C;-RS232 (1=GND, 2=N/A, 3=TX, 4=RX)
  Px 28pin Connector to SNES expansion port (at bottom of SNES console)
```

<a id="snescontrollersexertainmentioports"></a>

### SNES Controllers Exertainment - I/O Ports

**Exertainment I/O Port Summary (Expansion Port Unit)**

```text
  21C0h.0 TL16C550AN - RX Data FIFO (R)                          ;\when     (?)
  21C0h.0 TL16C550AN - TX Data FIFO (W)                          ; DLAB=0   (?)
  21C1h.0 TL16C550AN - Interrupt Control (R/W)                   ;/         00h
  21C0h.1 TL16C550AN - Baudrate Divisor Latch LSB, Bit0-7 (R/W)  ;\when     (-)
  21C1h.1 TL16C550AN - Baudrate Divisor Latch MSB, Bit8-15 (R/W) ;/DLAB=1   (-)
  21C2h   TL16C550AN - Interrupt Status (R)                                 01h
  21C2h   TL16C550AN - FIFO Control (W)                                     00h
  21C3h   TL16C550AN - Character Format Control (R/W)     ;<--- Bit7=DLAB   00h
  21C4h   TL16C550AN - Handshaking Control (R/W)                            00h
  21C5h   TL16C550AN - RX/TX Status (R) (Write=reserved for testing)        60h
  21C6h   TL16C550AN - Handshaking Status (R) (Write=unknown/reserved)      0xh
  21C7h   TL16C550AN - Scratch (R/W)                                        (-)
  21C8h   74HC374N (U3) - RAM address MSBs and SPI-style Serial Port (W)    (-)
  21C9h   Not used
  21CAh   74HC374N (U4) - RAM address LSBs (W)                              (-)
  21CBh   Not used
  21CCh   RAM (U5) data byte to/from selected RAM addr (R/W)   (battery backed)
  21CDh   Not used
  21CEh   Not used
  21CFh   ? initially set to 00h (not changed thereafter) (W) ;\maybe one of
  21Dxh   Not used                                            ; these resets
  21DFh   ? initially set to 80h (not changed thereafter) (W) ;/the TL16C550AN?
```

**21C0h..21C7h - TL16C550AN (U1) (RS232 Controller)**

[SNES Controllers Exertainment - RS232 Controller](#snes-controllers-exertainment-rs232-controller)

[SNES Controllers Exertainment - RS232 Data Packets &amp; Configuration](#snes-controllers-exertainment-rs232-data-packets-configuration)

[SNES Controllers Exertainment - RS232 Data Packets Login Phase](#snes-controllers-exertainment-rs232-data-packets-login-phase)

[SNES Controllers Exertainment - RS232 Data Packets Biking Phase](#snes-controllers-exertainment-rs232-data-packets-biking-phase)

**21C8h - 74HC374N (U3) - RAM address MSBs and SPI-style Serial Port (W)**

```text
  0   Serial Port Select (0=Select, 1=Idle)
  1   Serial Port Data   (transferred LSB first)
  2   Serial Port Clock  (0=Idle) (data must be stable on 0-to-1 transition)
  3-7 Upper 5bit of 13bit RAM address (see Ports 21CAh/21CCh)
```

Used to send two 16bit values (20F3h and 0470h) during initialization (and to
send more data later on). This controls some OSD video controller (possibly
also the picture-in-picture function). Used values are:

```text
  20xxh = set address (00h..EFh = yloc*18h+xloc) (24x10 chars)
  20Fxh = set address (F0h..F3h = control registers 0..3)
  1C20h = ascii space with attr=1Ch ?
  1E20h = ascii space with attr=1Eh ?
  1Exxh = ascii chars ":" and "0..9" and "A..Z" (standard ascii codes)
  1Exxh = lowercase chars "a..z" (at "A..Z"+80h instead of "A..Z"+20h)
  0000h = value used for control regs 0 and 1
  0111h = value used for control reg 2
  0070h,0077h,0470h = values used for control reg 3
```

Unknown if/which bicycle versions are actually using the OSD feature, maybe it
has been an optional or unreleased add-on. OSD output is supported in the
Program Manager's Workout &amp; Fit Test, apparently NOT for drawing the OSD
layer on top of the SNES layer, probably rather for displaying OSD while
watching TV programs.

Note: The general purpose "/OUT1" bit (RS232 port 21C4h.Bit2) is also output
via the serial port connector (purpose is unknown, might be OSD related, or TV
enable, or LED control, or whatever).

**21C8h - 74HC374N (U3) - RAM address MSBs and SPI-style Serial Port (W)**

**21CAh - 74HC374N (U4) - RAM address LSBs (W)**

**21CCh - SRAM (U5) - RAM data byte to/from selected RAM address (R/W)**

```text
  Port 21CAh.Bit0-7 = RAM Address Bit0-7 (W)   ;\13bit address, 0000h..1FFFh
  Port 21C8h.Bit3-7 = RAM Address Bit8-12 (W)  ;/
  Port 21C8h.Bit0-2 = See Serial Port description (W)
  Port 21CCh.Bit0-7 = RAM Data Bit0-7 (R/W)
```

Used to access battery-backed 8Kbyte SRAM in the expansion port unit. Note:
There are additional 2Kbytes of SRAM in the Mountain Bike game cartridge
(mapped to 700000h).

**21CFh (W) initially set to 00h (not changed thereafter)**

**21DFh (W) initially set to 80h (not changed thereafter)**

Used to configure/reset whatever stuff during initialization (not used
thereafter). Maybe one of these ports resets the TL16C550AN?

<a id="snescontrollersexertainmentrs232controller"></a>

### SNES Controllers Exertainment - RS232 Controller

**Texas Instruments TL16C550AN - Asynchronous Communications Element (ACE)**

The ACE uses eight I/O addresses (mapped to 21C0h-21C7h in the SNES), the
meaning of the first two addresses depends on the "DLAB" bit (which can be
changed via 21C3h.Bit7).

**21C0h (when DLAB=0) - TL16C550AN - RX Data FIFO (R)**

```text
  0-7  Data (with 16-byte FIFO)
```

**21C0h (when DLAB=0) - TL16C550AN - TX Data FIFO (W)**

```text
  0-7  Data (with 16-byte FIFO)
```

**21C1h (when DLAB=0) - TL16C550AN - Interrupt Control (R/W)**

```text
  0    Received Data Available Interrupt            (0=Disable, 1=Enable)
  1    Transmitter Holding Register Empty Interrupt (0=Disable, 1=Enable)
  2    Receiver Line Status Interrupt               (0=Disable, 1=Enable)
  3    Modem Status Interrupt                       (0=Disable, 1=Enable)
  4-7  Not used (always zero)
```

**21C0h (when DLAB=1) - TL16C550AN - Baudrate Divisor Latch LSB, Bit0-7 (R/W)**

**21C1h (when DLAB=1) - TL16C550AN - Baudrate Divisor Latch MSB, Bit8-15 (R/W)**

```text
  0-7  Divisor Latch LSB/MSB, should be set to "divisor = XIN / (baudrate*16)"
```

**21C2h - TL16C550AN - Interrupt Status (R)**

```text
  0    Interrupt Pending Flag (0=Pending, 1=None)
  1-3  Interrupt ID, 3bit     (0..7=see below) (always 00h when Bit0=1)
  4-5  Not used (always zero)
  6    FIFOs Enabled (always zero in TL16C450 mode) ;\these bits have same
  7    FIFOs Enabled (always zero in TL16C450 mode) ;/value as "FIFO Enable"
```

The 3bit Interrupt ID can have following values:

```text
  ID Prio Expl.
  00h 4   Handshaking inputs CTS,DSR,RI,DCD have changed      (Ack: Read 21C6h)
  01h 3   Transmitter Holding Register Empty   (Ack: Write 21C0h or Read 21C2h)
  02h 2   RX FIFO has reached selected trigger level          (Ack: Read 21C0h)
  03h 1   RX Overrun/Parity/Framing Error, or Break Interrupt (Ack: Read 21C5h)
  06h 2   RX FIFO non-empty & wasn't processed for longer time(Ack: Read 21C0h)
```

Interrupt ID values 04h,05h,07h are not used.

**21C2h - TL16C550AN - FIFO Control (W)**

```text
  0    FIFO Enable (0=Disable, 1=Enable) (Enables access to FIFO related bits)
  1    Receiver FIFO Reset      (0=No Change, 1=Clear RX FIFO)
  2    Transmitter FIFO Reset   (0=No Change, 1=Clear TX FIFO)
  3    DMA Mode Select (Mode for /RXRDY and /TXRDY) (0=Mode 0, 1=Mode 1)
  4-5  Not used (should be zero)
  6-7  Receiver FIFO Trigger    (0..3 = 1,4,8,14 bytes)
```

**21C3h - TL16C550AN - Character Format Control (R/W)**

```text
  0-1  Character Word Length    (0..3 = 5,6,7,8 bits)
  2    Number of Stop Bits      (0=1bit, 1=2bit; for 5bit chars: only 1.5bit)
  3    Parity Enable            (0=None, 1=Enable Parity or 9th data bit)
  4-5  Parity Type/9th Data bit (0=Odd, 1=Even, 2=Set9thBit, 3=Clear9thBit)
  6    Set Break                (0=Normal, 1=Break, Force SOUT to Low)
  7    Divisor Latch Access     (0=Normal I/O, 1=Divisor Latch I/O) (DLAB)
```

**21C4h - TL16C550AN - Handshaking Control (R/W)**

```text
  0    Output Level for /DTR pin  (Data Terminal Ready) (0=High, 1=Low)
  1    Output Level for /RTS pin  (Request to Send)     (0=High, 1=Low)
  2    Output Level for /OUT1 pin (General Purpose)     (0=High, 1=Low)
  3    Output Level for /OUT2 pin (General Purpose)     (0=High, 1=Low)
  4    Loopback Mode (0=Normal, 1=Testmode, loopback TX to RX)
  5-7  Not used (always zero)
```

**21C5h - TL16C550AN - RX/TX Status (R/W, but should accessed as read-only)**

```text
  0    RX Data Ready (DR)       (0=RX FIFO Empty, 1=RX Data Available)
  1    RX Overrun Error (OE)    (0=Okay, 1=Error) (RX when RX FIFO Full)
  2    RX Parity Error (PE)     (0=Okay, 1=Error) (RX parity bad)
  3    RX Framing Error (FE)    (0=Okay, 1=Error) (RX stop bit bad)
  4    RX Break Interrupt (BI)  (0=Normal, 1=Break) (RX line LOW for long time)
  5    Transmitter Holding Register (THRE) (1=TX FIFO is empty)
  6    Transmitter Empty (TEMT) (0=No, 1=Yes, TX FIFO and TX Shift both empty)
  7    At least one Overrun/Parity/Framing Error in RX FIFO (0=No, 1=Yes/Error)
```

Bit7 is always zero in TL16C450 mode. Bit1-3 are automatically cleared after
reading. In FIFO mode, bit2-3 reflect to status of the current (=oldest)
character in the FIFO (unknown/unclear if bit2-3 are also auto-cleared when in
FIFO mode).

**21C6h - TL16C550AN - Handshaking Status (R/W? - should accessed as read-only)**

```text
  0    Change flag for /CTS pin (Clear to Send)       ;\change flags (0=none,
  1    Change flag for /DSR pin (Data Set Ready)      ; 1=changed since last
  2    Change flag for /RI pin  (Ring Indicator)      ; read) (automatically
  3    Change flag for /DCD pin (Data Carrier Detect) ;/cleared after reading)
  4    Input Level on /CTS pin (Clear to Send)        ;\
  5    Input Level on /DSR pin (Data Set Ready)       ; current levels
  6    Input Level on /RI pin  (Ring Indicator)       ; (inverted ?)
  7    Input Level on /DCD pin (Data Carrier Detect)  ;/
```

**21C7h - TL16C550AN - Scratch (R/W)**

```text
  0-7  General Purpose Storage (eg. read/write-able for chip detection)
```

**Note**

The TL16C550AN doesn't seem to support a TX FIFO Full flag, nor automatic
RTS/CTS handshaking.

**Note on Nintendo DSi (newer handheld console, not SNES related)**

The DSi's AR6002 wifi chip is also using a TL16C550AN-style UART (for TTY debug
messages).

<a id="snescontrollersexertainmentrs232datapacketsconfiguration"></a>

### SNES Controllers Exertainment - RS232 Data Packets & Configuration

**From Bike to SNES (16 bytes: ATT code, command, 13-byte-data, checksum)**

```text
  Bike Packet 01h ;\these might both contain same bike data,
  Bike Packet 02h ;/both required to be send (else SNES hangs)
  Bike Packet 03h ;<-- confirms/requests pause mode
  Bike Packet 08h Login Part 1 (ID string)
  Bike Packet 09h PPU Status Request (with ignored content)
  Bike Packet 0Ah Login Part 3 (reply to random values)
  Bike Packet 0Ch Login Part 5 (fixed values 00,FF,00,0C,..)
```

**From SNES to Bike (13 bytes: ACK code, command, 10-byte-data, checksum)**

```text
  SNES Packet 00h Idle (zerofilled) or PPU Status Response (with "RAD" string)
  SNES Packet 01h Biking Start (start biking; probably resets time/distance)
  SNES Packet 02h Biking Active (biking)
  SNES Packet 03h Biking Pause (pause biking)
  SNES Packet 04h Biking Exit (finish or abort biking)
  SNES Packet 05h User Parameters
  SNES Packet 06h Biking ?
  SNES Packet 07h Biking ?
  SNES Packet 08h Biking ?
  SNES Packet 09h Login Part 2 (random values)
  SNES Packet 0Bh Login Part 4 (based on received data)
  SNES Packet 0Dh Login Part 6 (login okay)
  SNES Packet 0Fh Logout (login failed, or want new login)
  SNES Packet 0Ah,0Ch,0Eh (?) (unused?)
```

**Packet Details**

[SNES Controllers Exertainment - RS232 Data Packets Login Phase](#snes-controllers-exertainment-rs232-data-packets-login-phase)

[SNES Controllers Exertainment - RS232 Data Packets Biking Phase](#snes-controllers-exertainment-rs232-data-packets-biking-phase)

**RS232 Character Format**

The character format is initialized as [21C3h]=3Bh, which means,

```text
  1 start bit, 8 data bits, sticky parity, 1 stop bit, or, in other words:
  1 start bit, 9 data bits, no parity, 1 stop bit
```

The sticky parity bit (aka 9th data bit) should be set ONLY in the Bike's ATT
characters (133h), all other data (and ACK codes) should have that bit cleared.

**RS232 Baudrate**

The baudrate is aimed at 9600 bits/sec. The ACE Baudrate Divisor is set to
0023h aka 35 decimal (in both NTSC and PAL versions), with the ACE being driven
by the 5.3MHz Dot Clock. The resulting exact timings are:

```text
  NTSC: 5.36931750MHz/35/16 = 9588.067 Hz
  PAL:  5.32034250MHz/35/16 = 9500.612 Hz
```

Notes: The Dot Clock has some slight stuttering on long dots during hblank (but
doesn't disturb the baudrate too much). The PAL baudrate doesn't match too
well, however, it is the divisor setting closest to 9600 baud.

**RS232 Handshaking**

The RS232 connector has only 3 pins (RX, TX, GND). The RTS/CTS handshaking
signals are thus not used (nor are any Xon/Xoff handshaking characters used).

However, there is some sort of handshaking: The "From Bike" packets are
preceeded by a ATT (Attention) character (value 133h, with 9th data bit set,
aka sticky parity bit set), this allows to resynchronize to packet-start
boundaries in case of lost data bytes.

In the other direction, the "From SNES" packets should be sent only in response
to successfully received "From Bike" packets.

The packets are small enough to fit into the 16-byte FIFOs of the ACE chip. The
baudrate is a bit too low to send 16-byte packets in every frame, so the Bike
is apparently pinging out packets at some lower rate.

Note: The SNES software accepts the ATT (Attention) characters only if [21C5h]
returns exactly E5h (data present, TX fifo empty, and "error" flags indicating
the received parity bit being opposite as normal).

**RS232 Interrupts**

The ACE Interrupts are left unused: the IRQ pin is probably not connected to
SNES, ACE interrupts are disabled via [21C1h]=00h, and ACE interrupt ID in
[21C2h] isn't polled by software.

<a id="snescontrollersexertainmentrs232datapacketsloginphase"></a>

### SNES Controllers Exertainment - RS232 Data Packets Login Phase

**Login Phase**

```text
  Bike Packet 08h Login Part 1 (ID string)
  SNES Packet 09h Login Part 2 (random values)
  Bike Packet 0Ah Login Part 3 (reply to random values)
  SNES Packet 0Bh Login Part 4 (based on received data)
  Bike Packet 0Ch Login Part 5 (fixed values 00,FF,00,0C,..)
  SNES Packet 0Dh Login Part 6 (login okay)
  (communication phase...)
  SNES Packet 0Fh Logout (login failed, or want new login)
```

Login should be done on power-up. And, the SNES software does occassionally
logout and re-login (eg. when starting a new game from within main menu).

**PPU Status Request**

```text
  Bike Packet 09h PPU Status Request (with ignored content)
  SNES Packet 00h PPU Status Response (with "RAD" string)
```

PPU Status can be transferred during Login or Communication Phase, unknown
if/when/why the bike is actually doing that (the data is rather useless, except
maybe for use as random seed).

**From Bike Packet 08h Login Part 1 (ID string)**

```text
  ATT      Attention Code (133h, with 9th bit aka parity set = packet start)
  00h      Command (LSB=08h, MSB=Unknown/unused)
  01h..0Bh ID String ("LIFEFITNESS" or "LIFEFITNESs") ;[0Bh].bit5=flag? [1EEEh]
  0Ch..0Dh Unknown/unused
  0Eh      Checksum (00h-[00h..0Dh])
```

**From SNES Packet 09h Login Part 2 (random values)**

```text
  ACK      Acknowledge Code (33h, received packet with good checksum From Bike)
  00h      Command (LSB=09h, MSB=Zero)
  01h..0Ah Random values (RND1,RND2,RND3,RND4,RND5,RND6,RND7,RND8,RND9,RND10)
  0Bh      Checksum (00h-[00h..0Ah])
```

**From Bike Packet 0Ah Login Part 3 (reply to random values)**

```text
  ATT      Attention Code (133h, with 9th bit aka parity set = packet start)
  00h      Command (LSB=0Ah, MSB=Unknown/unused)
  01h      RND1+RND5                               ;\
  02h      RND2+(RND5*13+9)                        ; RND'values same as in
  03h      RND3+((RND5*13+9)*13+9)                 ; Login Part 2
  04h      RND4+(((RND5*13+9)*13+9)*13+9)          ;
  05h      RND5+((((RND5*13+9)*13+9)*13+9)*13+9)   ;/
  06h..0Ah Unknown/unused <-- these ARE USED for response TO bike
  0Bh..0Dh Unknown/unused <-- these seem to be totally unused
  0Eh      Checksum (00h-[00h..0Dh])
```

**From SNES Packet 0Bh Login Part 4 (based on received data)**

```text
  ACK      Acknowledge Code (33h, received packet with good checksum From Bike)
  00h      Command (LSB=0Bh, MSB=Zero)
  01h..0Ah Values "[01h]+[01h..0Ah]" from Login Part 3
  0Bh      Checksum (00h-[00h..0Ah])
```

**From Bike Packet 0Ch Login Part 5 (fixed values 00,FF,00,0C,..)**

```text
  ATT      Attention Code (133h, with 9th bit aka parity set = packet start)
  00h      Command (LSB=0Ch, MSB=Unknown/unused)
  01h..0Ah Constants (00h,FFh,00h,0Ch,0Ah,63h,00h,FAh,32h,C8h)
  0Bh..0Dh Unknown/unused
  0Eh      Checksum (00h-[00h..0Dh])
```

**From SNES Packet 0Dh Login Part 6 (login okay)**

```text
  ACK      Acknowledge Code (33h, received packet with good checksum From Bike)
  00h      Command (LSB=0Dh, MSB=Zero)
  01h..0Ah All zero (00)
  0Bh      Checksum (00h-[00h..0Ah])
```

**From SNES Packet 0Fh Logout (login failed, or want new login)**

```text
  ACK      Acknowledge Code (33h, received packet with good checksum From Bike)
  00h      Command (LSB=0Fh, MSB=Zero)
  01h..0Ah All zero (00)
  0Bh      Checksum (00h-[00h..0Ah])
```

This is sent upon login mismatch, and also if the game wants to re-enter the
login phase (as done after leaving the main menu).

**From Bike Packet 09h PPU Status Request (with ignored content)**

```text
  ATT      Attention Code (133h, with 9th bit aka parity set = packet start)
  00h      Command (LSB=09h, MSB=Unknown/unused)
  01h..0Dh Unknown/unused
  0Eh      Checksum (00h-[00h..0Dh])
```

**From SNES Packet 00h PPU Status Response (with "RAD" string)**

```text
  ACK      Acknowledge Code (33h, received packet with good checksum From Bike)
  00h      Command (LSB=00h, MSB=Zero)
  01h      PPU2 Status   [213Fh] (PPU2 chip version & Interlace/Lightgun/NTSC)
  02h      PPU1 Status   [213Eh] (PPU1 chip version & OBJ overflow flags)
  03h      CPU  Status   [4210h] (CPU chip version & NMI flag)
  04h      Curr Scanline [213Dh] (lower 8bit of current scanline number)
  05h..0Ah Constants (52h,41h,44h,00h,00h,00h) (aka "RAD",0,0,0)
  0Bh      Checksum (00h-[00h..0Ah])
```

Note: This data is send only after PPU Status Request. There are also cases
(during menues) where SNES is sending Packet 00h with zerofilled data body.

<a id="snescontrollersexertainmentrs232datapacketsbikingphase"></a>

### SNES Controllers Exertainment - RS232 Data Packets Biking Phase

**Communication Phase**

```text
  SNES Packet 00h Idle (while in menu) (also used for PPU Status Response)
  SNES Packet 01h Biking Start (start biking; probably resets time/distance)
  SNES Packet 02h Biking Active (biking)
  SNES Packet 03h Biking Pause (pause biking)
  SNES Packet 04h Biking Exit (finish or abort biking)
  SNES Packet 05h User Parameters
  SNES Packet 06h Biking ?
  SNES Packet 07h Biking ?
  SNES Packet 08h Biking ?
  Bike Packet 01h ;\these might both contain same bike data,
  Bike Packet 02h ;/both required to be send (else SNES hangs)
  Bike Packet 03h ;<-- confirms/requests pause mode
```

Unknown values &amp; commands might include things like TV control.

**From Bike Packet xxh (Communication Phase)**

```text
  ATT      Attention Code (133h, with 9th bit aka parity set = packet start)
  00h      Command (LSB=00h..07h or so, MSB=Curr Level 0..12, Pedal Resistance)
  01h      Speed in pedal rotations per minute (0..xxx) (above 200=glitches?)
  02h      Time (MSB) in 60 second units (0..255)       ;\
  03h      Time (LSB) in 1 second units (0..59)         ;/
  04h      Calories per hour (MSB) in 256 cal/hr units  ;\this used in bank B0h
  05h      Calories per hour (LSB) in 1 cal/hr units    ;/
  06h      Calories burned (MSB) in 256/4 cal units     ;\
  07h      Calories burned (LSB) in 1/4 cal units       ;/
  08h      Distance (MSB) in 65536/3600 miles           ;\
  09h      Distance (MID) in 256/3600 miles             ;
  0Ah      Distance (LSB) in 1/3600 miles               ;/
  0Bh      Pulse in heart beats per minute (1..255 bpm, or 0=No Pulse Sensor)
  0Ch      Fit Test Score (clipped to range 10..60)
  0Dh      Whatever 8bit (invalid values CRASH combo-cart games)
  0Eh      Checksum (00h-[00h..0Dh])
```

In Fit Test mode (when [0Dh]=85h), the bike seems to send Time=Zero when the
test ends (after 5 minutes) - either it's counting time backwards in that mode,
or it's wrapping from 5 minutes to zero? Other modes, like workout, are
counting time upwards, and end when reaching the selected time goal value.

**From SNES Packet xxh**

```text
  ACK      Acknowledge Code (33h, received packet with good checksum From Bike)
  00h      Command (LSB=01h..0xh ?, MSB=Wanted Level 0..12 Pedal Resistance)
  01h      Something (MSB=often same as above MSB, LSB=Present Hill related)
  02h      Present Hill
  03h..09h Upcoming Hills (7 bytes)
  0Ah      Whatever (in MENU: 01h,02h,00h, WORKOUT:80h, FIT-TEST:85h) 80h..86h
  0Bh      Checksum (00h-[00h..0Ah])
```

**From SNES Packet x5h (User Parameters)**

```text
  ACK      Acknowledge Code (33h, received packet with good checksum From Bike)
  00h      Command (LSB=05h, MSB=Wanted Level 0..12, Pedal Resistance)
  01h      Player's Sex (00h=Female, 01h=Male)
  02h      Player's Age in years (0..99)
  03h      Player's Weight in pounds (0..255, plus below Weight Extra)
  04h      Player's Pulse in heart beats per minute
  05h      Player's Weight Extra (added to weight, eg. 399 = values FFh+90h)
  06h..09h Garbage, set to same value as [05h], but NOT counted in checksum?
  0Ah      Whatever (same as in other "From SNES" communication packets)
  0Bh      Checksum (00h-[00h..0Ah]) --- in this case, excluding [06h..09h] ?
```

Single-cart is configuring this packet for Fit Test (but is never sending the
packet)? Combo-cart is often sending this packet (but with all parameters set
to zero)?

<a id="snescontrollersexertainmentdrawings"></a>

### SNES Controllers Exertainment - Drawings

```text
   Exercising Machine (Side View)                      Front Panel
                                             (located below of the monitor)
            Monitor                       ____________________________________
       .\  /        Handles              |                                    |
     .'  \         /          Saddle     |::::::      /\      /\      TV  Menu|
   .'     \     \            /           |::::::    Volume  Program    _    _ |
   \       \     \___                    |:::::: O    \/      \/      |_|  |_||
    \__--""|       //     =====          |____________________________________|
    |      |      //        ||             |     |    |       |       |
    |      |     //         ||             |     |    |       |      TV-Picture
    | Rack |    / |        / |   Pedals    |     |    |       |      in picture
    |      |   /   \      /  |  /          |     |    |       |
    |      |  /     \____/ _ |             |     |    |      TV Program Up/Down
    |      | /             /  \            |     |   Volume Up/Down
    |      |/             O    \           |    Headphone socket
    |______|___________________|          Speaker
```

The monitor, rack and front panel have been spotted on prototype photos,
unknown if the normal retail bikes did have them, too (also possible that one
got only the bike and expansion port unit - and had to use it with a regular
SNES and TV-set).

```text
                   Handlebars with Controllers
   ____ ______                                      ______ ____
  |    |   _  \                                    /      |    |
  |    | /\ /\ |     1st                1st       |   X   |    |
  |    ||  O  || <-- DPAD               XYAB  --> | Y   A |    |
  |    | \/_\/ |                                  |   B   |    |
  |    |       | <-- L-Button        R-Button --> |       |    |
  |    | O O   |     (on bottom      (on bottom   |       |    |
  |____|______/      side)           side)         \______|____|
   |  |  |  \                                              |  |
   |  |   \  Select                                        |  |
   |  |    Start                                           |  |
   |__| ________                                  ________ |__|
  |    |        |<-- 2nd DPAD        2nd XYAB -->|        |    |
  | __ |_______/   (also with Start/Select/L/R)   \_______| __ |
   |   \______________________.---._______________________/   |
    \_________________________|___|__________________________/
                              |   |
```

As depicted, there appear to be absolutely no brakes on this bike. Which must
have made it a frightening kamikaze-like experience to sit on that thing. Even
worse, there is no bell. One could only scream "Out of the way! Out of the
way!" at the monitor.

<a id="snescontrollerspachinko"></a>

### SNES Controllers Pachinko

**Pachinko Controller (Sunsoft)**

The Pachinko controller should be connected to controller Port 2 (plus a normal
joypad in Port 1 for menu selections). After the usual joypad strobing,
Pachinko data can be read serially from [4017h].Bit0:

```text
  1st..8th    Unknown/unused                (would be Buttons/DPAD on joypads)
  9th         Used... probably a button?    (would be Button-A on joypads)
  10th..12th  Unknown/unused                (would be Buttons on joypads)
  13th..16th  ID Bit3-0       (must be 0Eh) (MSB first)
  17th..24th  Extra ID Bit7-0 (must be 77h) (MSB first)
  25th        Unknown/unused
  26th..32th  Analog Dial Position (7bit, MSB first, inverted, 1=Low=Zero)
  33th..      Unknown/padding
```

Average analog 7bit range returned on real hardware is unknown. In software,
the used 7bit range is around 18h=Stopped through 7Fh=Fastest.

The controller looks somewhat like a blue egg, plus a yellow dial with zagged
finger-grips, and probably with a button somewhere (maybe the orange window in
the middle of the head or so?):

```text
            _
  Top-View  \_\___   .             __________   Side-View
          ..'     '..|\          .'  ''''''  '.     <-- Blue Head
      ...'           '.|       _|______________|
    .'.'      ___      '.     <__<_|__<_|_______)   <-- Yello dial
   / .'     .'   '.     '.      |              |
  /  |     |       |     |__    |              |    <-- Blue Base
  |/\|     |SUNSOFT|     | /    |              |
     '.     '.___.'     .'/      |            |
      '.               .'         ''._ __ _.'' \
        '.           .'          _____|__|_____ ''----- cable
          ''._____.''           (______________)
```

Known supported games are:

```text
  Hissatsu Pachinko Collection 1 (J) 1994 Sunsoft/Fuji
  Hissatsu Pachinko Collection 2 (J) 1995 Sunsoft/Fuji
  Hissatsu Pachinko Collection 3 (J) 1995 Sunsoft/Daiichi/Nifty-Serve
  Hissatsu Pachinko Collection 4 (J) 1996 Sunsoft/Kyoraku/Nifty-Serve
```

Note: Pachinko is a japanese gambling game; its appearance is resembling
pinball, but concerning stupidity it's more resembling one-armed-bandit-style
slot machines.

<a id="snescontrollersotherinputs"></a>

### SNES Controllers Other Inputs

**Reset Button (on the console)**

Resets various CPU/PPU/APU registers, but leaves WRAM intact, so for example,
games could use the button to restart the current level, or to enter the main
menu.

Note: The three SPC7110 games are containing a selftest program (executed upon
first boot; when battery-backed SRAM is still unitialized), the user is
prompted to push the Reset Button after each test screen.

**Super Famicom Box**

This thing has a (external) coin input, two push buttons, and a 6-position
switch (all these inputs are accessible only by its HD64180 CPU though, not by
the SNES CPU).

The two attached joypads aren't directly wired to the SNES, instead, they are
first passed to the HD64180, and then forwarded to SNES via 16bit shift
registers. This allows the HD64180 to sense the "L+R+Select+Start" Soft-Reset
key combination, and to inject recorded controller data in Demo/Preview mode.
On the restrictive side, the 16bit shift registers are making it incompatible
with special controllers like mice (which won't work with most of the SFC-Box
games anyways); unless, maybe "automatic-reading" mode bypasses the 16bit
shift-register, and forwards the FULL bitstream directly from SFC-Box to SNES?

Moreover, during Game Selection, some normally unused bits of the WRIO/RDIO
"controller" port are misused to communicate between KROM1 (on HD64180) and
ATROM (on SNES).

<a id="snesaddonturbofileexternalbackupmemoryforstoringgamepositions"></a>

## SNES Add-On Turbo File (external backup memory for storing game positions)

The Turbo File add-ons are an external battery-backed RAM-Disks made by ASCII.
Turbo File hardware has been produced for NES, SNES, 8bit Gameboy, and Gameboy
Advance. It's been sold only in japan, and it's mainly supported by ASCII's own
games. The SNES related hardware versions are:

```text
  SNES Turbo File Twin (160K) (128K in STF mode, 4x8K in TFII mode)
  SNES Turbo File Adapter (SNES adapter for NES Turbo File & Turbo File II)
```

**TFII Mode (old NES mode, 4x8Kbyte)**

[SNES Add-On Turbo File - TFII Mode Transmission Protocol](#snes-add-on-turbo-file-tfii-mode-transmission-protocol)

[SNES Add-On Turbo File - TFII Mode Filesystem](#snes-add-on-turbo-file-tfii-mode-filesystem)

**STF Mode (native SNES mode, 128Kbyte)**

[SNES Add-On Turbo File - STF Mode Transmission Protocol](#snes-add-on-turbo-file-stf-mode-transmission-protocol)

[SNES Add-On Turbo File - STF Mode Filesystem](#snes-add-on-turbo-file-stf-mode-filesystem)

**Compatible Games**

[SNES Add-On Turbo File - Games](#snes-add-on-turbo-file-games)

**NES Turbofile (AS-TF02)**

Original NES version, contains 8Kbytes battery backed RAM, and a 2-position
PROTECT switch, plus a LED (unknown purpose).

**NES Turbo File II (TFII)**

Newer NES version, same as above, but contains 32Kbytes RAM, divided into four
8Kbyte slots, which can be selected with a 4-position SELECT switch.

**SNES Turbo File Adapter**

Allows to connect a Turbo File or Turbo File II to SNES consoles. Aside from
the pin conversion (15pin NES to 7pin SNES), it does additionally contain some
electronics (for generating a SNES controller ID, and a more complicated
protocol for entering the data-transfer phase). Aside from storing SNES game
positions, this can be also used to import NES files to SNES games.

**SNES Turbo File Twin**

SNES version with 160Kbyte SRAM, and with 5-position mode SELECT switch. 128K
used in STF mode ("SNES Super Turbo File"), and 4x8K used in TFII modes 1/2/3/4
(equivalent to NES Turbo File II with SNES Turbo File Adapter).

Small square box that connects via cable to controller port.

Two position PROTECT switch (off/on)

Five position SELECT switch (STF, and "TFII" 1,2,3,4)

There is a red LED. And two 1.5V batteries?

**Hardware Versions**

```text
  Name                Capacity                  Connection
  Turbofile (AS-TF02) 1x8Kbyte                  NES-to-SNES Adapter
  Turbo File II       4x8Kbyte                  NES-to-SNES Adapter
  Turbo File Twin     4x8Kbyte plus 128Kbyte    Direct SNES Connection
  Gameboy version     ?                         N/A ?
  Gameboy Advance     ?                         N/A ?
```

<a id="snesaddonturbofiletfiimodetransmissionprotocol"></a>

## SNES Add-On Turbo File - TFII Mode Transmission Protocol

**oldest_recv_8191_bytes:**

```text
  call oldest_invoke_transfer   ;start transfer
  if invoke_okay then for i=0001h to 1FFFh, oldest_recv_byte(buf[i])
  jmp oldest_reset_turbofile    ;end transfer (always, even if invoke failed)
```

**oldest_send_8191_bytes:**

```text
  call oldest_invoke_transfer   ;start transfer
  if invoke_okay then for i=0001h to 1FFFh, oldest_send_byte(buf[i])
  jmp oldest_reset_turbofile    ;end transfer (always, even if invoke failed)
```

**oldest_invoke_transfer:**

```text
  call oldest_detect_and_get_status
  if no_tf_connected then fail/exit
  if data_phase=1 then            ;oops, already invoked
    oldest_detect_and_get_status  ;abort old transfer
    jmp oldest_invoke_transfer    ;retry invoking new transfer
  [004016]=01h                         ;strobe on      ;\
  for i=1 to 15,dummy=[004017],next i  ;issue 15 clks  ; invoke transfer
  [004016]=00h                         ;strobe off     ; (16 clks total)
  dummy=[004017]                       ;issue 1 clk    ;/
  call oldest_detect_and_get_status                    ;\want flag set now
  if data_phase=0 then fail/exit                       ;/
  for i=1 to 7                                         ;\skip remaining 7 bits
    dummy=[004017]  ;<-- required?     ;issue clk      ; of unused byte 0000h
    [004016]=01h                       ;strobe on      ; (the first bit was
    [004016]=00h                       ;strobe off     ; skipped by STROBE in
  next i                                               ;/detect_and_get_status)
```

After above, the hardware byte-address is 0001h (ie. unlike as in NES version,
the unused byte at address 0000h is already skipped).

**oldest_detect_and_get_status:**

```text
  [004016]=01h    ;strobe on
  [004016]=00h    ;strobe off
  for i=23 to 0   ;get ID/status (MSB first)
    temp=[004017]  ;issue clk & get data
    stat.bit(i)=temp.bit0
  next i
  if stat.bit(11..8)<>0Eh then no_tf_connected=1  ;major 4bit id
  if stat.bit(7..0)<>0FFh then no_tf_connected=1  ;minor 8bit id
  if stat.bit(12)=1 then data_phase=1 else data_phase=0
```

**oldest_reset_turbofile:**

```text
  [004016]=01h    ;strobe on
  dummy=[004017]  ;issue clk
  [004016]=00h    ;strobe off
  dummy=[004017]  ;issue clk
  [004016]=00h    ;strobe off (again?)
  jmp oldest_detect_and_get_status
```

**oldest_recv_byte(data):**

```text
  for i=0 to 7  ;transfer data byte (LSB first)
    temp=[004017]               ;issue clk (required?), and get bit from joy4
    data.bit(i)=temp.bit(1)     ;extract received data bit
    [004016]=01h                ;strobe on
    [004201]=80h*data.bit(i)    ;write SAME/UNCHANGED bit to hardware
    [004016]=00h                ;strobe off (WRITE CLOCK)
  next i
```

**oldest_send_byte(data):**

```text
  for i=0 to 7  ;transfer data byte (LSB first)
    dummy=[004017]              ;issue clk (really required for writing?)
    [004016]=01h                ;strobe on
    [004201]=80h*data.bit(i)    ;write NEW bit to hardware
    [004016]=00h                ;strobe off (WRITE CLOCK)
  next i
```

<a id="snesaddonturbofiletfiimodefilesystem"></a>

## SNES Add-On Turbo File - TFII Mode Filesystem

**Turbo File Memory**

The first byte (at offset 0000h) is unused (possibly because that there is a
risk that other games with other controller access functions may destroy it);
after resetting the address, one should read one dummy byte to skip the unused
byte. The used portion is 8191 bytes (offset 0001h..1FFFh). The "filesystem" is
very simple: Each file is attached after the previous file, an invalid file ID
indicates begin of free memory.

**Turbo File Fileformat (newer files) (1987 and up)**

Normal files are formatted like so:

```text
  2   ID "AB" (41h,42h)
  2   Filesize (16+N+2) (including title and checksum)
  16  Title in ASCII (terminated by 00h or 01h)
  N   Data Portion
  2   Checksum (all N bytes in Data Portion added together)
```

**Turbo File Fileformat (old version) (1986)**

The oldest Turbo File game (NES Castle Excellent from 1986) doesn't use the
above format. Instead, it uses the following format, without filename, and with
hardcoded memory offset 0001h..01FFh (511 bytes):

```text
  1   Don't care (should be 00h)    ;fixed, at offset 0001h
  2   ID AAh,55h                    ;fixed, at offset 0002h..0003h
  508 Data Portion (Data, end code "BEDEUTUN", followed by some unused bytes)
```

CAUTION:

```text
  The early version has transferred all bytes in reversed bit-order,
  so above ID bytes AAh,55h will be seen as 55h,AAh in newer versions!
```

Since the address is hardcoded, Castle Excellent will forcefully destroy any
other/newer files that are located at the same address. Most newer NES/SNES
games (like NES Fleet Commander from 1988, and SNES Wizardry 5 from 1992) do
include support for handling the Castle Excellent file. One exception that
doesn't support the file is NES Derby Stallion - Zenkoku Ban from 1992.

<a id="snesaddonturbofilestfmodetransmissionprotocol"></a>

## SNES Add-On Turbo File - STF Mode Transmission Protocol

**FileTwinSendCommand (28bits)**

```text
  set strobe=1
  8x sendbit (LSB first)    ;command (24h=read or 75h=write)
  20x sendbit (LSB first)   ;address (00000h..FFFFFh)
  set strobe=0
  FileTwinRecvStatusAndID
  error if bad-ID or general-error-flag (for write: also write protect-error)
  retry "FileTwinSendCommand" if desired Read (or Write) Mode bit isn't set
```

Thereafter, send/receive data byte(s), and finish by TerminateCommand

**TerminateCommand**

```text
  set strobe=1
  if command was READ then issue clk (don't do that on WRITE command)
  set strobe=0
  FileTwinRecvStatusAndID
  retry "TerminateCommand" if Data Read/Write Mode bits are still nonzero
```

**FileTwinSendDataByte**

```text
  set strobe=1
  8x sendbit (LSB first)
  set strobe=0
```

**FileTwinRecvDataByte**

```text
  set strobe=1
  set strobe=0
  8x recvbit (from joy4) (LSB first) (inverted)
```

**FileTwinRecvStatusAndID (32bits)**

```text
  set strobe=1
  set strobe=0
  12x recvbit (from joy2) (ignored)
  4x  recvbit (from joy2) (MSB first) (major ID, must be 0Eh)
  8x  recvbit (from joy2) (MSB first) (minor ID, must be FEh)
  1x  recvbit (from joy2) Data Write Mode (0=No/Idle, 1=Yes/Command 75h)
  1x  recvbit (from joy2) Data Read Mode  (0=No/Idle, 1=Yes/Command 24h)
  1x  recvbit (from joy2) General-Hardware-Error (1=Error)
  1x  recvbit (from joy2) Write-Protect-Error    (1=Error/Protected)
  4x  recvbit (from joy2) (MSB first) (capacity) (usually/always 0=128K)
```

**Low Level Functions**

```text
  set strobe=1        --> [004016h]=1
  set strobe=0        --> [004016h]=0
  recvbit (from joy2) --> bit=[004017h].bit0
  recvbit (from joy4) --> bit=NOT [004017h].bit1
  sendbit             --> [004201h]=bit*80h, dummy=[004017h]
  issue clk           --> dummy=[004017h]
```

<a id="snesaddonturbofilestfmodefilesystem"></a>

## SNES Add-On Turbo File - STF Mode Filesystem

**FileTwinCapacityCodes (last 4bit of FileTwinRecvStatusAndID)**

```text
  00h  Single Drive with 128Kbytes (normal) (plus extra 32Kbyte for TFII mode)
  01h  Single Drive with 256Kbytes
  02h  Single Drive with 384Kbytes
  03h  Single Drive with 640Kbytes (really, this is NOT 512Kbytes)
  04h  Multi-Drive with 1 normal 128K Drive  (128K total) ;\allows to READ from
  05h  Multi-Drive with 2 normal 128K Drives (256K total) ; all drives, but
  06h  Multi-Drive with 3 normal 128K Drives (384K total) ; can WRITE only
  07h  Multi-Drive with 5 normal 128K Drives (640K total) ;/to first drive
  08h..0Fh  Reserved (treated same as 00h; Single Drive with 128Kbytes)
```

**FileTwinAddresses (?)**

XXX multiply below by 400h

```text
  000h..00Fh  FAT (4Kbytes)
  010h..1FFh  Entries for 1st 124Kbytes
  200h..7FFh  Unused
  800h..9FFh  Entries for 2nd 128Kbytes (if any)
  A00h..BFFh  Entries for 3rd 128Kbytes (if any)
  C00h..DFFh  Entries for 4th 128Kbytes (if any) (seems to be bugged)
  E00h..FFFh  Unused
  xxxh..xxxh  Partition-Read FAT (though WRITES are to address zero ?)
```

**FileTwinFAT**

The FAT is 4096 bytes in size:

```text
  000h..1EFh  Entries for 1st 124Kbytes
  1F0h..3EFh  Entries for 2nd 128Kbytes (if any)
  3F0h..5EFh  Entries for 3rd 128Kbytes (if any)
  5F0h..7EFh  Entries for 4th 128Kbytes (if any) (seems to be bugged)
  7F0h..FFBh  Unused
  FFCh..FFFh  ID "FAT0"
```

Each FAT Entry is 32bit (4 bytes) wide:

```text
  0-11   filesize in kbyte (1st blk), 000h (2nd..Nth blk), FFFh (free blk)
  12-23  NNNh (next blk), FFFh (last blk), or also FFFh (free blk)
  24-31  8bit sector chksum (all 1024 bytes added together), or FFh (free blk)
```

Note: The above FFFh values should be as so (though older games are checking
only the upper 4bit, thereby treating any Fxxh values as free/last block).

Unused FAT entries (that exceed memory capacity) are also marked as "free".

**FileTwinFileHeaders**

The first 24 bytes (in the first sector) of a file contain File ID &amp; Name:

```text
  000h..003h ID1 (must be "SHVC")
  004h..007h ID2 (should be same as Game Code at [FFB2h] in ROM-Header)
  008h..017h Filename (padded with ...spaces ?)
  018h..     File Data (Filesize from FAT, multiplied by 1024, minus 24 bytes)
```

ID2 and Name may contain ASCII characters 20h..3Fh and 41h..5Ah.

<a id="snesaddonturbofilegames"></a>

## SNES Add-On Turbo File - Games

**SNES Games that support Turbo File Twin in STF-Mode**

```text
  Bahamut Lagoon (1996) Square (INCORRECTLY?! claimed to support Turbo File)
  Daisenryaku Expert WWII: War in Europe (1996) SystemSoftAlpha/ASCII Corp (JP)
  Dark Law - Meaning of Death (1997) ASCII (JP)
  Derby Stallion III (1995) (supports both TFII and STF modes)
  Derby Stallion 96 (1996) (supports TFII and STF and Satellaview-FLASH-cards)
  Derby Stallion 98 (NP) (1998) (supports both TFII and STF modes)
  Gunple/Ganpuru - Gunman's Proof (1997) ASCII/Lenar (JP)
  Mini Yonku/4WD Shining Scorpion - Let's & Go!! (1996) KID/ASCII Corp (JP)
  Ongaku Tukool/Tsukuru Kanaderu (supports STF and Satellaview-FLASH-cards)
  RPG Tukool 1
  RPG Tukool 2 (supports STF and Satellaview-FLASH-cards)
  Sound Novel Tukool (supports STF and Satellaview-FLASH-cards)
  Tactics Ogre - Let Us Cling Together (supports both TFII and STF modes)
  Wizardry 6 - Bane of the Cosmic Forge (1995) (JP) (English)
```

Plus (unverified):

```text
  STF: Tower Dream
  STF: Best Shot Pro Golf (J)
  STF: Jewel of Live (RPG Maker Super Dante) (BS)
  STF: Solid Runner (J)
  STF: Wizardry Gaiden IV - Taima no Kodou (J) (v1.1)
```

**SNES Games that support Turbo File Adapter &amp; Turbo File Twin in TFII-Mode**

```text
  Ardy Lightfoot (1993)
  Derby Stallion II (1994)
  Derby Stallion III (1995) (supports both TFII and STF modes)
  Derby Stallion 96 (1996) (supports TFII and STF and Satellaview-FLASH-cards)
  Derby Stallion 98 (NP) (1998) (supports both TFII and STF modes)
  Down the World: Mervil's Ambition (1994)
  Kakinoki Shogi (1995) ASCII Corporation
  Tactics Ogre - Let Us Cling Together (supports both TFII and STF modes)
  Wizardry 5 - Heart of the Maelstrom (1992) Game Studio/ASCII (JP)
  BS Wizardry 5 (JP) (Satellaview BS-X version)
```

Plus (unverified):

```text
  TFII: Haisei Mahjong - Ryouga (J)
  TFII: Super Fire Pro Wrestling Special, X, and X Premium (J)
  TFII: Super Robot Taisen Gaiden - Masou Kishin - The Lord of Elemental (J)
```

Note: The US version of Wizardry 5 (1993) contains 99% of the turbo file
functions, but lacks one opcode that makes the hardware detection
nonfunctional. Wizardry 5 was announced/rumoured to be able to import game
positions from NES to SNES.

**NES Games that do support the Turbo File / Turbo File II**

```text
  Best Play Pro Yakyuu (1988) ASCII (J)
  Best Play Pro Yakyuu '90 (1990) (J)
  Best Play Pro Yakyuu II (1990) (J)
  Best Play Pro Yakyuu Special (1992) (J)
  Castle Excellent (1986) ASCII (J) (early access method without filename)
  Derby Stallion - Zenkoku Ban (1992) Sonobe Hiroyuki/ASCII (J)
  Downtown - Nekketsu Monogatari (19xx) Technos Japan Corp (J)
  Dungeon Kid (1990) Quest/Pixel (J)
  Fleet Commander (1988) ASCII (J)
  Haja no Fuuin (19xx) ASCII/KGD (J)
  Itadaki Street - Watashi no Mise ni Yottette (1990) ASCII (J)
  Ninjara Hoi! (J)
  Wizardry - Legacy of Llylgamyn (19xx?) (J)
  Wizardry - Proving Grounds of the Mad Overlord (1987) (J)
  Wizardry - The Knight of Diamonds (1991) (J)
```

NES games that do support Turbo File should have a "TF" logo on the cartridge.

Note: Castlequest (US version of Castle Excellent) reportedly lacks support.

<a id="snesaddonbarcodebattlerbarcodereader"></a>

## SNES Add-On Barcode Battler (barcode reader)

The Barcode Battler from Epoch allows to scan barcodes (either from special
paper cards, or from daily-life products like food packagings), games can then
use the barcode digits as Health Points, or other game attributes.

**Standalone-Mode**

The device was originally designed as stand-alone gaming console with some push
buttons, a very simple LCD screen with 7-segment digits &amp; some predefined
LCD symbols, and a built-in game BIOS (ie. without external cartridge slot, and
without any bitmap graphics).

**Link-Mode**

Later versions (with black case) include an "EXT" link port, allowing to link
to other Barcode Battler hardware, or to Famicom/Super Famicom consoles. The
EXT port is probably bi-directional, but existing Famicom/Super Famicom games
seem to be using it only for reading barcodes (without accessing the LCD
screen, push buttons, speaker, or EEPROM).

[SNES Add-On Barcode Transmission I/O](#snes-add-on-barcode-transmission-io)

[SNES Add-On Barcode Battler Drawings](#snes-add-on-barcode-battler-drawings)

**Barcode Battler Famicom (NES) Games**

```text
  Barcode World (1992) Sunsoft (JP) (includes cable with 15pin connector)
```

**Barcode Battler Super Famicom (SNES) Games**

```text
  Alice's Paint Adventure (1995)
  Amazing Spider-Man, The - Lethal Foes (19xx)
  Barcode Battler Senki Coveni Wars (1993) Epoch
  Donald Duck no Mahou no Boushi (19xx)
  Doraemon 2: Nobita's Great Adventure Toys Land (1993)
  Doraemon 3: Nobita and the Jewel of Time (1994)
  Doraemon 4 - Nobita to Tsuki no Oukoku (19xx)
  Doroman (canceled)
  Dragon Slayer - Legend of Heroes 2 (1993) Epoch
  J-League Excite Stage '94 (1994)
  J-League Excite Stage '95 (1995)
  Lupin Sansei - Densetsu no Hihou wo Oe! (19xx)
  Super Warrior Combat (19xx - does this game exist at all?)
```

**Barcode Battler Hardware Versions**

```text
  Region__Case___EXT___Barcode-Reader__Name__________________Year___
  Japan   White  None  Yes             Barcode Battler       1991
  Japan   Black  1     Yes             Barcode Battler II    1992
  Japan   Black  2     None            Barcode Battler II^2  199x
  Europe  Black  1     Yes             Barcode Battler       1992/1993
```

The versions with one EXT socket can be connected to NES/SNES, or to one or
more of the "II^2" units (allowing more players to join the game).

**Connection to SNES/NES consoles**

Connection to Super Famicom or SNES requires a "BBII INTERFACE": a small box
with 4 LEDs and two cables attached (with 3pin/7pin connectors), the interface
has been sold separetedly, it's needed to add a SNES controller ID code to the
transmission protocol.

Connection to Famicom consoles requires a simple cable (without interface box)
(with 3pin/15pin connectors), the cable was shipped with the "Barcode World"
Famicom cartridge, connection to NES would require to replace the 15pin Famicom
connector by 7pin NES connector.

The required 3pin EXT connector is available only on newer Barcode Battlers
(with black case), not on the original Barcode Battler (with white case).

```text
  Unknown if all 3 pins are actually used by NES/SNES cable/interface?
  Unknown if NES/SNES software can access LCD/buttons/speaker/EEPROM ?
```

**Connectivity**

"Connectivity mode is accessible if you plug in a standard 3.5mm mono jack plug
into the expansion port on the left hand side of the unit, hold down the
R-Battle and R-Power buttons and turn the unit on, the Barcode Battler II goes
into scanner mode."

**Barcode Battler II Interface**

The hardware itself was manufactured by Epoch, and licensed by Nintendo (it
says so on the case).

The four lights, from left to right, indicate as follows:

```text
  "OK"    All is well, the device is operating as normal.
  "ER"    Maybe there's something wrong?
  "BBII"  The Barcode Battler is sending data to the device.
  "SFC"   The SFC/SNES is waiting for a signal from the Barcode Battler.
```

**Component List (may be incomplete)**

```text
  80pin NEC uPD75316GF (4bit CPU with on-chip 8Kx8 ROM, 512x4 RAM, LCD driver)
  8pin Seiko S2929A (Serial EEPROM, 128x16 = 2Kbit) (same/similar as S29290)
  3pin EXT socket (3.5mm "stereo" jack) (only in new versions with black case)
  LCD Screen (with 7-segment digits and some predefined words/symbols)
  Five LEDs (labelled "L/R-Battle Side")
  Seven Push Buttons (L/R-POWER, L/R-Battle, Power on/off, Select, Set)
  Speaker with sound on/off switch (both on bottom side)
  Barcode reader (requires card-edges to be pulled through a slot)
  Batteries (four 1.5V AA batteries) (6V)
```

<a id="snesaddonbarcodetransmissionio"></a>

## SNES Add-On Barcode Transmission I/O

The Barcode Battler outputs barcodes as 20-byte ASCII string, at 1200 Baud,
8N1. The NES software receives that bitstream via Port 4017h.Bit2. The SNES
software requires a BBII Interface, which converts the 8bit ASCII digits into
4bit nibbles, and inserts SNES controller ID and status codes, the interface
should be usually connected to Controller Port 2 (although the existing SNES
games seem to accept it also in Port 1).

**Barcode Battler (with BBII Interface) SNES Controller Bits**

```text
  1st..12th   Unknown/unused (probably always 0=High?)
  13th..16th  ID Bits3..0          (MSB first, 1=Low=One) (must be 0Eh)
  17th..24th  Extended ID Bits7..0 (MSB first, 1=Low=One) (must be 00h..03h)
              (the SNES programs accept extended IDs 00h..03h, unknown
              if/when/why the BBII hardware does that send FOUR values)
  25th        Status: Barcode present (1=Low=Yes)
  26th        Status: Error Flag 1 ?
  27th        Status: Error Flag 2 ?
  28th        Status: Unknown      ?
```

Following bits need/should be read ONLY if the "Barcode Present" bit is set.

```text
  29th-32th   1st Barcode Digit, Bits3..0  (MSB first, 1=Low=One)
  33th-36th   2nd Barcode Digit, Bits3..0  (MSB first, 1=Low=One)
  37th-40th   3rd Barcode Digit, Bits3..0  (MSB first, 1=Low=One)
  41th-44th   4th Barcode Digit, Bits3..0  (MSB first, 1=Low=One)
  45th-48th   5th Barcode Digit, Bits3..0  (MSB first, 1=Low=One)
  49th-52th   6th Barcode Digit, Bits3..0  (MSB first, 1=Low=One)
  53th-56th   7th Barcode Digit, Bits3..0  (MSB first, 1=Low=One)
  57th-60th   8th Barcode Digit, Bits3..0  (MSB first, 1=Low=One)
  61th-64th   9th Barcode Digit, Bits3..0  (MSB first, 1=Low=One)
  65th-68th   10th Barcode Digit, Bits3..0 (MSB first, 1=Low=One)
  69th-72th   11th Barcode Digit, Bits3..0 (MSB first, 1=Low=One)
  73th-76th   12th Barcode Digit, Bits3..0 (MSB first, 1=Low=One)
  77th-80th   13th Barcode Digit, Bits3..0 (MSB first, 1=Low=One)
  81th and up Unknown/unused
       Above would be 13-digit EAN-13 codes
       Unknown how 12-digit UPC-A codes are transferred   ;\whatever leading
       Unknown if/how 8-digit EAN-8 codes are transferred ; or ending padding?
       Unknown if/how 8-digit UPC-E codes are transferred ;/
```

For some reason, delays should be inserted after each 8 bits (starting with
24th bit, ie. after 24th, 32th, 40th, 48th, 56th, 64th, 72th bit, and maybe
also after 80th bit). Unknown if delays are also needed after 8th and 16th bit
(automatic joypad reading does probably imply suitable delays, but errors might
occur when reading the ID bits via faster manual reading).

**Barcode Battler RAW Data Output**

Data is send as 20-byte ASCII string. Bytes are transferred at 1200 Bauds:

```text
  1 Start bit (must be LOW)
  8 Data bits (LSB first, LOW=Zero, HIGH=One)
  1 Stop bit  (must be HIGH)
```

The first 13 bytes can contain following strings:

```text
  "nnnnnnnnnnnnn"    ;13-digit EAN-13 code (ASCII chars 30h..39h)
  <Unknown>          ;12-digit UPC-A code (with ending/leading padding?)
  "     nnnnnnnn"    ;8-digit EAN-8 code (with leading SPC-padding, ASCII 20h)
  <Unknown>          ;8-digit UPC-E code (with ending/leading padding?)
  "ERROR        "    ;indicates scanning error
```

The last 7 bytes must contain either one of following ID strings:

```text
  "EPOCH",0Dh,0Ah    ;<-- this is sent/accepted by existing hardware/software
  "SUNSOFT"          ;<-- this would be alternately accepted by the NES game
```

There are rumours that one "must" use a mono 3.5mm plug in order to receive
data - that's obviously bullshit, but it might indicate that the middle pin of
stereo plugs must be GNDed in order to switch the Barcode Battler into transmit
mode(?)

<a id="snesaddonbarcodebattlerdrawings"></a>

## SNES Add-On Barcode Battler Drawings

**Barcode Battler - Handheld Console (Front)**

```text
         _______________________________
  .---"""    _______________________    """---.
  |  /\     |                       |     /\  |
  |  \ \    |                       |    / /  |
  | L \/    |      LCD Screen       |    \/ R |
  | POWER   |                       |   POWER |
  |  /\     |                       |     /\  |
  |  \ \    |                       |    / /  |
  | L \/    |_______________________|    \/ R |
  | BATTLE                             BATTLE |
  |         O     O     O     O     O         |
  |         L  <-- Battle Side -->  R         |
   ) EXT                                      |
  |        On/off     Select     Set          |
  |        [====]     [====]    [====]        |
  '---_____________________________________---'
  -->   : _____________________________ :  --> --> pull card this way
        :/  CARD IN -->                \:
        |_______________________________|
```

**Barcode Battler - Handheld Console (Back)**

```text
         _______________________________
  .---"""  :                         :  """---.
  |        :       Battery Lid       :        |
  |        :   (Four AA Batteries)   :        |
  |        :                         :        |
  |        :.........................:        |
  |                                           |
  |                                           |
  |                                           |
  |                      __                   |
  |          o          |__|                  |
  |        o o o        Sound                 |
  |          o          on/off               ( <-- 3.5mm 3pin
  |       Speaker                             |    EXT socket
  |                                           |
  '---__                                 __---'
        |                               |
        |                               |
        |_______________________________|
```

**Barcode Battler - LCD Screen Layout**

```text
   _______________ ___________ _______________
  | .-------.    _|__ESCAPE___|_    .-------. |
  | | FIGHT |   / < SUPER HIT > \   |RECOVER| |
  | |BARCODE|  /|_______________|\  | MISS  | |
  | '-------'                       '-------' |
  |  (*) (/) (K)  POWER   INPUT  (i) (/) (*)  |
  |  _  _  _  _  _   _______   _  _  _  _  _  |
  | |_||_||_|| || | |ENERGY | |_||_||_|| || | |
  | |_||_||_||_||_| |DAMAGE | |_||_||_||_||_| |
  |  _  _  _  _  _  |_______|  _  _  _  _  _  |
  | |_||_||_|| || | |ATTACK | |_||_||_|| || | |
  | |_||_||_||_||_| | MAGIC | |_||_||_||_||_| |
  |  _  _  _  _  _  |_______|  _  _  _  _  _  |
  | |_||_||_|| || | |DEFENCE| |_||_||_|| || | |
  | |_||_||_||_||_| |SURVIVA| |_||_||_||_||_| |
  |_________________|_______|_________________|
```

**Barcode Batter II Interface (SNES/SuperFamicom) &amp; Simple Cable (Famicom)**

```text
              _________________
  cables ->  /     _______     '''---> 3pin 3.5mm "stereo" EXT connector
       _____|_____|____   '''---> 7pin SNES/SuperFamicom connector
   ___|                |___
  |                        |         _.----> 3pin 3.5mm "stereo" EXT connector
  |     BBII INTERFACE     |      .-'
  |                        |     |
  |    O O O O             |      '-_   cable
  |________________________|         ''-------> 15pin Famicom connector
```

**Paper-Card Front (Picture Side)**

```text
   _________________________________________________
  |                                                 |
  | NINJA STAR                           WEAPON-17  |
  |                                      > INSERT   |
  | ----------------------^------------------------ |
  |                      | |                        |
  |                ___  /___\ ____                  |
  |       ___---"""   /__/ \__\   """---___         |
  |    ---___          __ O __          ___---      |
  |          """---___\ _\ /_ /___---"""          / |
  |                     \   /                    /  |
  |                      | |                    /_  |
  | ----------------------V--------------------'( ) |
  | ST 400                                      ||| |
  |_________________________________________________|
```

**Paper-Card Back (Description &amp; Barcode)**

```text
   _________________________________________________
  |                                                 |
  |                BARCODE BATTLER                  |
  |                   NINJA STAR                    |
  |                                     1992 Epoch  |
  |  A lethal weapon which demands skill, patience  |
  |   and, most important, perfect timing. Send it  |
  | spinning at the enemy when the Battler spirit is|
  |  with you and the effect can be devasting. Time |
  | it wrong and the star may harmlessly bounce off |
  |                 their defences.                 |
  |         || || |||| || || |||| || || ||||        |
  |         || || |||| || || |||| || || ||||        |
  |         || || |||| || || |||| || || ||||        |
  |_________________________________________________|
```

<a id="snesaddonsfcmodemforjrapat"></a>

## SNES Add-On SFC Modem (for JRA PAT)

[SNES Add-On SFC Modem - Data I/O](#snes-add-on-sfc-modem-data-io)

[SNES Add-On SFC Modem - Misc](#snes-add-on-sfc-modem-misc)

**Controller**

The SFC Modem is bundled with a special controller (to be connected to
controller port 1) (required for the JRA PAT software):

[SNES Controllers NTT Data Pad (joypad with numeric keypad)](#snes-controllers-ntt-data-pad-joypad-with-numeric-keypad)

**FLASH Backup**

The JRA PAT Modem BIOS cartridges contain FLASH backup memory (unlike other
SNES cartridges which do use battery-backed SRAM instead of FLASH).

[SNES Cart FLASH Backup](63-copiers-cheat-devices-cdrom.md#snes-cart-flash-backup)

**Baudrates**

The BIOS seems to support max 2400 baud (old BIOS version) and 9600 baud (new
BIOS version) - accordingly, there are probably also two different hardware
versions of the SFC Modem.

**Note**

There's also another modem (which connects to cartridge slot):

[SNES Cart X-Band (2400 baud Modem)](62-cartridge-addons-satellaview-modems.md#snes-cart-x-band-2400-baud-modem)

<a id="snesaddonsfcmodemdataio"></a>

## SNES Add-On SFC Modem - Data I/O

The modem is intended to be connected to controller port 2. RX Data, TX Data,
and Modem Status are simultaneously transferred via three I/O lines. The
overall transfer length (with ID bits) is 16-bit, however, after checking the
ID bits, one can abbreviate the transfer to 9-bit length.

**JOY2: (4017h.Bit0) - RX Data and ID Bits**

```text
  1st          RX Data Bit7    (0=High=Zero, 1=Low=One) ;\
  2nd          RX Data Bit6    (0=High=Zero, 1=Low=One) ;
  3rd          RX Data Bit5    (0=High=Zero, 1=Low=One) ; to be ignored when
  4th          RX Data Bit4    (0=High=Zero, 1=Low=One) ; no RX Data Present
  5th          RX Data Bit3    (0=High=Zero, 1=Low=One) ;
  6th          RX Data Bit2    (0=High=Zero, 1=Low=One) ;
  7th          RX Data Bit1    (0=High=Zero, 1=Low=One) ;
  8th          RX Data Bit0    (0=High=Zero, 1=Low=One) ;/
  9th          RX Data Present (0=High=None, 1=Low=Yes)
  10th         Unknown/Unused
  11th         Unknown/Unused
  12th         Unknown/Unused
  13th         ID Bit3 (always 0=High)
  14th         ID Bit2 (always 0=High)
  15th         ID Bit1 (always 1=Low)
  16th         ID Bit0 (always 1=Low)
  17th and up  Unknown/Unused (probably always whatever)
```

**JOY4: (4017h.Bit1) - Modem Status**

```text
  1st          Unknown Flags Bit7  (1=Low=Busy or so, 0=Ready to get TX Data)
  2nd          Unknown Flags Bit6  (0=High=Error/Abort or so)
  3rd          Unknown Flags Bit5  (1=Low=Busy or so)
  4th          Unknown Flags Bit4  (1=Low=Busy or so)
  5th          Unknown Flags Bit3  Unused?
  6th          Unknown Flags Bit2  Unused?
  7th          Unknown Flags Bit1  Unused?
  8th          Unknown Flags Bit0  Unused?
  9th and up   Unknown/Unused (probably always whatever)
```

**IOBIT (4201h.Bit7) - TX Data**

1st bit should be output immediately after strobing 4016h.Output, 2nd..9th bit
should be output immediately after reading 1st..8th data/status bits from
4017h.

```text
  1st          TX Data Present (0=Low=Yes, 1=HighZ=None)
  2nd          TX Data Bit7    (0=Low=Zero, 1=HighZ=One) ;\
  3rd          TX Data Bit6    (0=Low=Zero, 1=HighZ=One) ; should be DATA
  4th          TX Data Bit5    (0=Low=Zero, 1=HighZ=One) ; when Data Present,
  5th          TX Data Bit4    (0=Low=Zero, 1=HighZ=One) ; or otherwise,
  6th          TX Data Bit3    (0=Low=Zero, 1=HighZ=One) ; should be FFh,
  7th          TX Data Bit2    (0=Low=Zero, 1=HighZ=One) ; or "R" or "C" ?
  8th          TX Data Bit1    (0=Low=Zero, 1=HighZ=One) ; (RTS/CTS or so?)
  9th          TX Data Bit0    (0=Low=Zero, 1=HighZ=One) ;/
  10th and up  Should be "1"   (1=HighZ)
```

<a id="snesaddonsfcmodemmisc"></a>

## SNES Add-On SFC Modem - Misc

"The Modem as far as I know only had one function and that was to allow you to
do online betting via the official JRA (Japanese Horse Racing) online service.
The modem ran on the NTT lines which probably means that NTT (Nippon
Telecommunications) also had something to make out of this service or at least
they thought they did :-D"

```text
  JRA = Japan Racing Association (japanese horse racing)
  PAT = Personal Access Terminal (for telephone/online betting)
  NTT = Nippon Telegraph and Telephone (japanese telecommunications)
```

NTT JRA PAT (1997) (2400 Baud version) (J)

```text
  baud rates seem to be 2400,1200 (see ROM 03:8910) (and "AT%B" strings)
  uses standard AT-commands (ATI0, ATS0?, ATD, ATX1, etc.)
  supports AMD FLASH only
  there are two ROM versions (SHVC-TJAJ-0 and SHVC-TJBJ-0)
```

NTT JRA PAT - Wide Baken Taiyou (1999) (9600 Baud version) (J)

```text
  baud rates seem to be 9600,2400,1200 (see ROM 03:87F0) (and "AT%B" strings)
  uses standard AT-commands (ATI0, ATS0?, ATD, ATX1, etc.)
  supports AMD/ATMEL/SHARP FLASH
  there are two ROM versions (SHVC-TJDJ-0 and SHVC-TJEJ-0)
```

Zaitaku Touhyou System - SPAT4-Wide (1999 or so)

```text
  unknown, reportedly also horse betting with NTT modem
  there is one ROM version (SHVC-TOBJ-0)
```

There is no special "modem-hardware" entry in cartridge header, but: note that
all NTT/SFC modem BIOS have special "SHVC-Txxx-x" game codes.

<a id="snesaddonvoicekunirtransmitterreceiverforusewithcdplayers"></a>

## SNES Add-On Voice-Kun (IR-transmitter/receiver for use with CD Players)

The Voice-Kun (sometimes called Voicer-Kun) from Koei is an Infrared
transmitter/receiver. The transmitter part is used for controlling Audio CD
Players (ie. to select and play tracks from Audio CDs that are included with
supported games). The receiver part is used to "learn" IR-signals from
different Remote Control manufacturers.

**Controller Bits**

The existing games expect the IR-unit connected to Port 2 (and a Joypad or
Mouse in Port 1). The controller ID can be read via 4017h.Bit0 (serially, with
STB/CLK signals). The actual IR-data is transferred via 4017h.Bit1/4201h.Bit7
(directly, without STB/CLK signals).

```text
 4017h.Bit0:
  1st-12th    Unknown/unused (probably always 0=High?)
  13th-16th   ID Bits 3-0 (MSB First, 0=High=Zero) (always 0Dh)
  17th and up Unknown/unused (probably always whatever?)
 4017h.Bit1:
  any bits    Infrared level (receiver) (0=High=Off, 1=Low=On)
 4201h.Bit7:
  any bits    Infrared level (transmit) (0=Low=Off, 1=High=On)
```

**Required Buttons**

```text
  [  ]  Stop
  [|>}  Play
  [||]  Pause
  [<<]  Previous Track
  [>>]  Next Track
  0..9  Numeric Digits
  +10   Plus 10          ;\alternately either one of these
  >10   Two-Digit-Input  ;/can be selected during configuration
```

The sequence for entering 2-digit Track numbers may vary greatly (+10, or
&gt;10 or -/-- buttons, to be pressed before or after the low-digits), and,
unknown if the Phillips "Toggle-Bit" is supported (needed for selecting track
11,22,33,etc. So far, one may expect problems with some CD players (though, as
workaround, maybe the japanese GUI of the Voice-Kun games allows to select the
starting track manually?).

**Low Level Signals**

```text
  Logical  ________--------------------________--------------------________
  Physical ________||||||||||||||||||||________||||||||||||||||||||________
```

The Voice-Kun hardware is automatically modulating/demodulating the
transmitted/received signals, so the SNES software does only need to deal with
"Logical" signals.

**Voice-Kun Games**

```text
  Angelique Voice Fantasy (29 Mar 1996)
  EMIT Vol. 1 - Toki no Maigo (25 Mar 1995)
  EMIT Vol. 2 - Inochigake no Tabi (25 Mar 1995)
  EMIT Vol. 3 - Watashi ni Sayonara wo (25 Mar 1995)
  EMIT Value Set (EMIT Vol. 1-3) (15 Dec 1995)
```

**Note - Soundware**

Koei also made a number of "Soundware" games (mostly for other
consoles/computers), which did also include Audio CDs or Audio Tapes.

For the SNES, Koei did release "Super Sangokushi II", originally on 15/09/1991,
and re-released on 30 Mar 1995, at least one of that two releases (unclear
which one) has been reportedly available with "Soundware" - unknown if that
soundware version is Voice-Kun compatible, or (if it isn't compatible) unknown
how else it was intended to be used...?)

<a id="snes3dglasses"></a>

## SNES 3D Glasses

**SNES 3D Games (there's only one known game)**

```text
  Jim Power: The Lost Dimension in 3-D (1993) (Loriciel) (Pulfrich Effect)
```

**Common 3D Glasses**

Most common glasses would be red/cyan lens (as used on NES) or glasses with LCD
shutters (as used on Famicom). Neither of that types appears to be used on SNES
though.

**Pulfrich Effect 3D Glasses (US) (dark/clear glasses) (aka Nuoptix)**

Another approach is using the Pulfrich effect with dark/clear glasses, this is
some sort psychophysical stuff, related to different signal timings for
bright/dark colors.

```text
  Right Eye -- Clear    ;\as so for SNES Jim Power: The Lost Dimension in 3-D)
  Left Eye  -- Dark     ;/(that is, opposite as for NES Orb-3D)
```

The advantage is that it's cheap, and that it can be even used for colored
images (unlike the red/cyan glasses method). The disadvantage is that it does
work only with permanently moving objects.
