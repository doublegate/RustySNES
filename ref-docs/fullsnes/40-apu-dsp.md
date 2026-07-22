# Fullsnes — Audio Processing Unit (APU / SPC700 / S-DSP) & Maths

[Index](00-index.md) · [« Picture Processing Unit](30-ppu.md) · [Controllers & Input Peripherals »](50-controllers.md)

**Sections in this file:**

- [SNES Audio Processing Unit (APU)](#snes-audio-processing-unit-apu)
- [SNES APU Memory and I/O Map](#snes-apu-memory-and-io-map)
- [SNES APU Block Diagram](#snes-apu-block-diagram)
- [SNES APU SPC700 CPU Overview](#snes-apu-spc700-cpu-overview)
- [SNES APU SPC700 CPU Load/Store Commands](#snes-apu-spc700-cpu-loadstore-commands)
- [SNES APU SPC700 CPU ALU Commands](#snes-apu-spc700-cpu-alu-commands)
- [SNES APU SPC700 CPU Jump/Control Commands](#snes-apu-spc700-cpu-jumpcontrol-commands)
- [SNES APU SPC700 I/O Ports](#snes-apu-spc700-io-ports)
- [SNES APU Main CPU Communication Port](#snes-apu-main-cpu-communication-port)
- [SNES APU DSP BRR Samples](#snes-apu-dsp-brr-samples)
- [SNES APU DSP BRR Pitch](#snes-apu-dsp-brr-pitch)
- [SNES APU DSP ADSR/Gain Envelope](#snes-apu-dsp-adsrgain-envelope)
- [SNES APU DSP Volume Registers](#snes-apu-dsp-volume-registers)
- [SNES APU DSP Control Registers](#snes-apu-dsp-control-registers)
- [SNES APU DSP Echo Registers](#snes-apu-dsp-echo-registers)
- [SNES APU Low Level Timings](#snes-apu-low-level-timings)
- [SNES Maths Multiply/Divide](#snes-maths-multiplydivide)

---

<a id="snesaudioprocessingunitapu"></a>

## SNES Audio Processing Unit (APU)

**Overview**

[SNES APU Memory and I/O Map](#snes-apu-memory-and-io-map)

[SNES APU Block Diagram](#snes-apu-block-diagram)

**SPC700 CPU**

[SNES APU SPC700 CPU Overview](#snes-apu-spc700-cpu-overview)

[SNES APU SPC700 CPU Load/Store Commands](#snes-apu-spc700-cpu-loadstore-commands)

[SNES APU SPC700 CPU ALU Commands](#snes-apu-spc700-cpu-alu-commands)

[SNES APU SPC700 CPU Jump/Control Commands](#snes-apu-spc700-cpu-jumpcontrol-commands)

**I/O Ports**

[SNES APU SPC700 I/O Ports](#snes-apu-spc700-io-ports)

[SNES APU Main CPU Communication Port](#snes-apu-main-cpu-communication-port)

[SNES APU DSP BRR Samples](#snes-apu-dsp-brr-samples)

[SNES APU DSP BRR Pitch](#snes-apu-dsp-brr-pitch)

[SNES APU DSP ADSR/Gain Envelope](#snes-apu-dsp-adsrgain-envelope)

[SNES APU DSP Volume Registers](#snes-apu-dsp-volume-registers)

[SNES APU DSP Control Registers](#snes-apu-dsp-control-registers)

[SNES APU DSP Echo Registers](#snes-apu-dsp-echo-registers)

**Pinouts/Misc**

[SNES APU Low Level Timings](#snes-apu-low-level-timings)

[SNES Audio/Video Connector Pinouts](80-timings-unpredictable-pinouts.md#snes-audiovideo-connector-pinouts)

[SNES Pinouts APU Chips](80-timings-unpredictable-pinouts.md#snes-pinouts-apu-chips)

<a id="snesapumemoryandiomap"></a>

## SNES APU Memory and I/O Map

**SPC700 Memory Map**

```text
  0000h..00EFh  RAM (typically used for CPU pointers/variables)
  00F0h..00FFh  I/O Ports (writes are also passed to RAM)
  0100h..01FFh  RAM (typically used for CPU stack)
  0200h..FFBFh  RAM (code, data, dir-table, brr-samples, echo-buffer, etc.)
  FFC0h..FFFFh  64-byte Boot ROM or RAM (selectable via Port 00F1h)
```

**Audio-related Registers on Main CPU**

The main CPU has four write-only 8bit outputs, and (mapped to the same
addresses) four read-only 8bit inputs:

```text
  2140h - APUI00  - Main CPU to Sound CPU Communication Port 0
  2141h - APUI01  - Main CPU to Sound CPU Communication Port 1
  2142h - APUI02  - Main CPU to Sound CPU Communication Port 2
  2143h - APUI03  - Main CPU to Sound CPU Communication Port 3
```

The registers are used to communicate with Port 00F4h..00F7h on the SPC700 CPU
(on power-up, this is done using a 64-byte BIOS in the SPC700).

Note: All CPU-APU communications are passed through these registers by
software, there are no additional communication methods (like IRQs).

**SPC700 I/O Ports**

The SPC700 CPU includes 16 memory mapper ports at address 00F0h..00FFh:

```text
  00F0h - TEST    - Testing functions (W)                                  0Ah
  00F1h - CONTROL - Timer, I/O and ROM Control (W)                         80h
  00F2h - DSPADDR - DSP Register Index (R/W)                              (FFh)
  00F3h - DSPDATA - DSP Register Data (R/W)                          (DSP[7Fh])
  00F4h - CPUIO0  - CPU Input and Output Register 0 (R and W)      R=00h,W=00h
  00F5h - CPUIO1  - CPU Input and Output Register 1 (R and W)      R=00h,W=00h
  00F6h - CPUIO2  - CPU Input and Output Register 2 (R and W)      R=00h,W=00h
  00F7h - CPUIO3  - CPU Input and Output Register 3 (R and W)      R=00h,W=00h
  00F8h - AUXIO4  - External I/O Port P4 (S-SMP Pins 34-27) (R/W) (unused) FFh
  00F9h - AUXIO5  - External I/O Port P5 (S-SMP Pins 25-18) (R/W) (unused) FFh
  00FAh - T0DIV   - Timer 0 Divider (for 8000Hz clock source) (W)         (FFh)
  00FBh - T1DIV   - Timer 1 Divider (for 8000Hz clock source) (W)         (FFh)
  00FCh - T2DIV   - Timer 2 Divider (for 64000Hz clock source) (W)        (FFh)
  00FDh - T0OUT   - Timer 0 Output (R)                                    (00h)
  00FEh - T1OUT   - Timer 1 Output (R)                                    (00h)
  00FFh - T2OUT   - Timer 2 Output (R)                                    (00h)
```

**DSP Registers**

The 128 DSP Registers are indirectly accessed via SPC700 Ports 00F2h/00F3h.

("x" can be 0..7, for selecting one of the 8 voices, or of the 8 filters).

```text
  x0h - VxVOLL   - Left volume for Voice 0..7 (R/W)
  x1h - VxVOLR   - Right volume for Voice 0..7 (R/W)
  x2h - VxPITCHL - Pitch scaler for Voice 0..7, lower 8bit (R/W)
  x3h - VxPITCHH - Pitch scaler for Voice 0..7, upper 6bit (R/W)
  x4h - VxSRCN   - Source number for Voice 0..7 (R/W)
  x5h - VxADSR1  - ADSR settings for Voice 0..7, lower 8bit (R/W)
  x6h - VxADSR2  - ADSR settings for Voice 0..7, upper 8bit (R/W)
  x7h - VxGAIN   - Gain settings for Voice 0..7 (R/W)
  x8h - VxENVX   - Current envelope value for Voice 0..7 (R)
  x9h - VxOUTX   - Current sample value for Voice 0..7 (R)
  xAh - NA       - Unused (8 bytes of general-purpose RAM) (R/W)
  xBh - NA       - Unused (8 bytes of general-purpose RAM) (R/W)
  0Ch - MVOLL    - Left channel master volume (R/W)
  1Ch - MVOLR    - Right channel master volume (R/W)
  2Ch - EVOLL    - Left channel echo volume (R/W)
  3Ch - EVOLR    - Right channel echo volume (R/W)
  4Ch - KON      - Key On Flags for Voice 0..7 (W)
  5Ch - KOFF     - Key Off Flags for Voice 0..7 (R/W)
  6Ch - FLG      - Reset, Mute, Echo-Write flags and Noise Clock (R/W)
  7Ch - ENDX     - Voice End Flags for Voice 0..7 (R) (W=Ack)
  0Dh - EFB      - Echo feedback volume (R/W)
  1Dh - NA       - Unused (1 byte of general-purpose RAM) (R/W)
  2Dh - PMON     - Pitch Modulation Enable Flags for Voice 1..7 (R/W)
  3Dh - NON      - Noise Enable Flags for Voice 0..7 (R/W)
  4Dh - EON      - Echo Enable Flags for Voice 0..7 (R/W)
  5Dh - DIR      - Sample table address (R/W)
  6Dh - ESA      - Echo ring buffer address (R/W)
  7Dh - EDL      - Echo delay (ring buffer size) (R/W)
  xEh - NA       - Unused (8 bytes of general-purpose RAM) (R/W)
  xFh - FIRx     - Echo FIR filter coefficient 0..7 (R/W)
```

Register 80h..FFh are read-only mirrors of 00h..7Fh.

Technically, the DSP registers are a RAM-like 128-byte buffer; and are copied
to internal registers when needed. At least some registers (KON and FLG) seem
to have changed-flags (and the buffer-values are processed only when the flag
is set), ENDX seems to be a real register (not using the RAM-like buffer).

Upon Reset, FLG is internally set to E0h (but reading the buffered DSP[6Ch]
value return garbage. The 17 status register are updated (a few cycles after
reset) to ENDX=FFh, ENVx=00h, OUTx=00h. Aside from ENDX/ENVx/OUTx, all other
DSP registers contain garbage. Upon Power-up, that garbage randomly "tends" to
some patterns, but those patterns change from day to day (some examples: one
day all registers were set to 43h, on other days, the eight FIRx were all FFh,
and other registers had whichever values).

**APU RAM**

Upon power-up, APU RAM tends to contain a stable repeating 64-byte pattern:
32x00h, 32xFFh (that, for APUs with two Motorola MCM51L832F12 32Kx8 SRAM chips;
other consoles may use different chips with different garbage/patterns). After
Reset, the boot ROM changes [0000h..0001h]=Entrypoint, and [0002h..00EFh]=00h).

<a id="snesapublockdiagram"></a>

## SNES APU Block Diagram

**DSP Voice Block Diagram (n=voice, 0..7)**

```text
                    OUTx(n-1)   PITCHn
                     PMON         |    ADSRn/ENV
                       |____MUL___|     |
  DIR*256                    |          +-------------------------------> ENVxn
  +SRCn*4                    |          |             .-----------------> OUTxn
   _____      _______      __V___       |             |        _____
  |     |    |  BRR  |    | BRR  |      |    _____    |       |VOLnL|
  | RAM |--->|Decoder|--->| Time |---o  '-->|     |   |   .-->| MUL |---> Ln
  |_____|    |_______|    |______|    \     | MUL |   |   |   |_____|
                           ______ NONn o--->|     |---+---+    _____
                          | Noise|          |_____|       |   |VOLnR|
                          | Time |---o                    '-->| MUL |---> Rn
                          |______|                            |_____|
                             ^
                             |
                            FLG
```

**DSP Mixer/Reverb Block Diagram (c=channel, L/R)**

```text
          ________                        _____                   _____
  c0 --->| ADD    |                      |MVOLc| Master Volume   |     |
  c1 --->| Output |--------------------->| MUL |---------------->|     |
  c2 --->| Mixing |                      |_____|                 |     |
  c3 --->|        |                       _____                  | ADD |--> c
  c4 --->|        |                      |EVOLc| Echo Volume     |     |
  c5 --->|        |   Feedback   .------>| MUL |---------------->|     |
  c6 --->|        |   Volume     |       |_____|                 |_____|
  c7 --->|________|    _____     |                      _________________
                      | EFB |    |                     |                 |
     EON  ________    | MUL |<---+---------------------|   Add FIR Sum   |
  c0 -:->|        |   |_____|                          |_________________|
  c1 -:->|        |      |                              _|_|_|_|_|_|_|_|_
  c2 -:->|        |      |                             |   MUL FIR7..0   |
  c3 -:->|        |      |         ESA=Addr, EDL=Len   |_7_6_5_4_3_2_1_0_|
  c4 -:->| ADD    |    __V__  FLG   _______________     _|_|_|_|_|_|_|_|_
  c5 -:->| Echo   |   |     | ECEN | Echo Buffer c |   | FIR Buffer c    |
  c6 -:->| Mixing |-->| ADD |--:-->|   RAM         |-->| (Hardware regs) |
  c7 -:->|________|   |_____|      |_______________|   |_________________|
                                   Newest --> Oldest    Newest --> Oldest
```

**External Sound Output &amp; Input**

```text
   _____     _____     ___________     _______     _______
  |     |   |     |   | Pre-      |   |       |   | Post- |
  | DSP |-->| D/A |-->| Amplifier |-->| Analog|-->| Ampl. |--> Multi-Out
  |_____|   |_____|   |___________|   | Mixer |   |       |    (Stereo Out)
     |                                |       |   | (with |
     |   Cartridge Slot Stereo In --->|       |   | phase |--> TV Modulator
     |                                |       |   | inver-|    (Mono Out)
     |   Expansion Port Stereo In --->|       |   | sion) |
     |                                |_______|   |       |--> Expansion Port
     |   /MUTE signal                     |       |       |    (Mono Out)
     '------------------------------->----'       |_______|
```

Note: The Cartridge Audio input is used only by SGB and MSU1. Unknown if it is
also used by X-Band modem (for injecting dial/connection sounds)? The Expansion
port input might be used by the Satellaview (?) and SNES CD prototype.

The Nintendo Super System (NSS) also allows to mute the SNES sound via its Z80
CPU.

<a id="snesapuspc700cpuoverview"></a>

## SNES APU SPC700 CPU Overview

The sound unit is controlled by a S-SMP chip: an 8bit CPU that uses Sony's
SPC700 instruction set. The SPC700's opcodes and it's three main registers
(A,X,Y) are clearly inspired by 6502 instruction set, although the opcodes are
numbered differently, and it has some new/changed instructions.

Aside from the SNES sound processor, the SPC700 is also found in Sony's
CXP8nnnn single-chip microprocessors. Another variant, called SPC700 alpha II,
is also found in their CXP7nnnn chips, which have only 211 opcodes (two less
than the normal SPC700).

**Registers**

```text
  A     8bit accumulator
  X     8bit index
  Y     8bit index
  SP    8bit stack pointer (addresses memory at 0100h..01FFh)
  PSW   8bit flags
  YA    16bit combination of Y=MSB, and A=LSB
  PC    16bit program counter
```

**Flags (PSW)**

```text
  Bit7  N  Sign Flag          (0=Positive, 1=Negative)
  Bit6  V  Overflow Flag      (0=None, 1=Overflow)
  Bit5  P  Zero Page Location (0=00xxh, 1=01xxh)
  Bit4  B  Break Flag         (0=Reset, 1=BRK opcode; set <after> BRK opcode)
  Bit3  H  Half-carry         (0=Borrow, or no-carry, 1=Carry, or no-borrow)
  Bit2  I  Interrupt Enable   (0=Disable, 1=Enable) (no function in SNES APU)
  Bit1  Z  Zero Flag          (0=Non-zero, 1=Zero)
  Bit0  C  Carry Flag         (0=Borrow, or no-carry, 1=Carry, or no-borrow)
```

**Addressing Modes**

```text
  Native Syntax  Nocash Syntax
  aa             [aa]           ;\addresses memory at [0000..00FF]
  aa+X           [aa+X]         ; (or at [0100..01FF when flag P=1)
  aa+Y           [aa+Y]         ; (aa+X and aa+Y wrap within that area,
  (X)            [X]            ; ie. addr = (aa+X) AND 0FFh)
  (Y)            [Y]            ;/
  aaaa           [aaaa]         ;\
  aaaa+X         [aaaa+X]       ; addresses memory at [0000..FFFF]
  aaaa+Y         [aaaa+Y]       ;/
  [aa]+Y         [[aa]+Y]       ;-Byte[Word[aa]+Y]  ;\double-indirect (using
  [aa+X]         [[aa+X]]       ;-Byte[Word[aa+X]]  ;/16bit pointer in RAM)
  aa.b           [aa].b         ;-Bit0..7 of address [0000..00FF] (8bit addr)
  aaa.b          [aaa].b        ;-Bit0..7 of address [0000..1FFF] (13bit addr)
  stack (push/pop/call/ret)     ;-addresses memory at [0100..01FF] (SP+100h)
```

**Formatting of Command Descriptions**

```text
  Native Syntax  Nocash Syntax   Opcode     Clk Expl                   NVPBHIZC
```

<a id="snesapuspc700cpuloadstorecommands"></a>

## SNES APU SPC700 CPU Load/Store Commands

**Register Manipulation**

```text
  MOV  A,#nn     MOV  A,nn       E8 nn       2  A=nn                   N.....Z.
  MOV  X,#nn     MOV  X,nn       CD nn       2  X=nn                   N.....Z.
  MOV  Y,#nn     MOV  Y,nn       8D nn       2  Y=nn                   N.....Z.
  MOV  A,X       MOV  A,X        7D          2  A=X                    N.....Z.
  MOV  X,A       MOV  X,A        5D          2  X=A                    N.....Z.
  MOV  A,Y       MOV  A,Y        DD          2  A=Y                    N.....Z.
  MOV  Y,A       MOV  Y,A        FD          2  Y=A                    N.....Z.
  MOV  X,SP      MOV  X,SP       9D          2  X=SP                   N.....Z.
  MOV  SP,X      MOV  SP,X       BD          2  SP=X  ;at 0100..01FF   ........
```

**Memory Load**

```text
  MOV  A,aa      MOV  A,[aa]     E4 aa       3  A=[aa]                 N.....Z.
  MOV  A,aa+X    MOV  A,[aa+X]   F4 aa       4  A=[aa+X]               N.....Z.
  MOV  A,!aaaa   MOV  A,[aaaa]   E5 aa aa    4  A=[aaaa]               N.....Z.
  MOV  A,!aaaa+X MOV  A,[aaaa+X] F5 aa aa    5  A=[aaaa+X]             N.....Z.
  MOV  A,!aaaa+Y MOV  A,[aaaa+Y] F6 aa aa    5  A=[aaaa+Y]             N.....Z.
  MOV  A,(X)     MOV  A,[X]      E6          3  A=[X]                  N.....Z.
  MOV  A,(X)+    MOV  A,[X]+     BF          4  A=[X], X=X+1           N.....Z.
  MOV  A,[aa]+Y  MOV  A,[[aa]+Y] F7 aa       6  A=[[aa]+Y]             N.....Z.
  MOV  A,[aa+X]  MOV  A,[[aa+X]] E7 aa       6  A=[[aa+X]]             N.....Z.
  MOV  X,aa      MOV  X,[aa]     F8 aa       3  X=[aa]                 N.....Z.
  MOV  X,aa+Y    MOV  X,[aa+Y]   F9 aa       4  X=[aa+Y]               N.....Z.
  MOV  X,!aaaa   MOV  X,[aaaa]   E9 aa aa    4  X=[aaaa]               N.....Z.
  MOV  Y,aa      MOV  Y,[aa]     EB aa       3  Y=[aa]                 N.....Z.
  MOV  Y,aa+X    MOV  Y,[aa+X]   FB aa       4  Y=[aa+X]               N.....Z.
  MOV  Y,!aaaa   MOV  Y,[aaaa]   EC aa aa    4  Y=[aaaa]               N.....Z.
  MOVW YA,aa     MOVW YA,[aa]    BA aa       5  YA=Word[aa]            N.....Z.
```

**Memory Store**

```text
  MOV  aa,#nn    MOV  [aa],nn    8F nn aa    5  [aa]=nn      (read)    ........
  MOV  aa,bb     MOV  [aa],[bb]  FA bb aa    5  [aa]=[bb]    (no read) ........
  MOV  aa,A      MOV  [aa],A     C4 aa       4  [aa]=A       (read)    ........
  MOV  aa,X      MOV  [aa],X     D8 aa       4  [aa]=X       (read)    ........
  MOV  aa,Y      MOV  [aa],Y     CB aa       4  [aa]=Y       (read)    ........
  MOV  aa+X,A    MOV  [aa+X],A   D4 aa       5  [aa+X]=A     (read)    ........
  MOV  aa+X,Y    MOV  [aa+X],Y   DB aa       5  [aa+X]=Y     (read)    ........
  MOV  aa+Y,X    MOV  [aa+Y],X   D9 aa       5  [aa+Y]=X     (read)    ........
  MOV  !aaaa,A   MOV  [aaaa],A   C5 aa aa    5  [aaaa]=A     (read)    ........
  MOV  !aaaa,X   MOV  [aaaa],X   C9 aa aa    5  [aaaa]=X     (read)    ........
  MOV  !aaaa,Y   MOV  [aaaa],Y   CC aa aa    5  [aaaa]=Y     (read)    ........
  MOV  !aaaa+X,A MOV  [aaaa+X],A D5 aa aa    6  [aaaa+X]=A   (read)    ........
  MOV  !aaaa+Y,A MOV  [aaaa+Y],A D6 aa aa    6  [aaaa+Y]=A   (read)    ........
  MOV  (X)+,A    MOV  [X]+,A     AF          4  [X]=A, X=X+1 (no read) ........
  MOV  (X),A     MOV  [X],A      C6          4  [X]=A        (read)    ........
  MOV  [aa]+Y,A  MOV  [[aa]+Y],A D7 aa       7  [[aa]+Y]=A   (read)    ........
  MOV  [aa+X],A  MOV  [[aa+X]],A C7 aa       7  [[aa+X]]=A   (read)    ........
  MOVW aa,YA     MOVW [aa],YA    DA aa       5  Word[aa]=YA  (read lsb)........
```

Most of the Memory Store opcodes are implemented like ALU opcodes (ie. as RMW
opcodes, issuing a dummy read from the destination address). In result, they
are a bit slower than necessary, and they can trigger read-sensitive I/O ports
(namely the TnOUT registers in the SNES). Only exceptions are opcodes AFh and
FAh (which don't include any dummy read), and opcode DAh (which performs the
dummy read only on the LSB of the 16bit Word).

**Push/Pop**

```text
  PUSH A         PUSH A          2D          4  [SP]=A,     SP=SP-1    ........
  PUSH X         PUSH X          4D          4  [SP]=X,     SP=SP-1    ........
  PUSH Y         PUSH Y          6D          4  [SP]=Y,     SP=SP-1    ........
  PUSH PSW       PUSH PSW        0D          4  [SP]=Flags, SP=SP-1    ........
  POP  A         POP  A          AE          4  SP=SP+1, A=[SP]        ........
  POP  X         POP  X          CE          4  SP=SP+1, X=[SP]        ........
  POP  Y         POP  Y          EE          4  SP=SP+1, Y=[SP]        ........
  POP  PSW       POP  PSW        8E          4  SP=SP+1, Flags=[SP]    NVPBHIZC
```

<a id="snesapuspc700cpualucommands"></a>

## SNES APU SPC700 CPU ALU Commands

**8bit ALU Operations**

```text
  OR   a,b       OR   a,b        00+x ...    .. a=a OR b               N.....Z.
  AND  a,b       AND  a,b        20+x ...    .. a=a AND b              N.....Z.
  EOR  a,b       XOR  a,b        40+x ...    .. a=a XOR b              N.....Z.
  CMP  a,b       CMP  a,b        60+x ...    .. a-b                    N.....ZC
  ADC  a,b       ADC  a,b        80+x ...    .. a=a+b+C                NV..H.ZC
  SBC  a,b       SBC  a,b        A0+x ...    .. a=a-b-not C            NV..H.ZC
```

Above OR/AND/EOR/CMP/ADC/SBC can be used with following operands:

```text
  cmd  A,#nn     cmd  A,nn       x+08 nn     2  A,nn
  cmd  A,(X)     cmd  A,[X]      x+06        3  A,[X]
  cmd  A,aa      cmd  A,[aa]     x+04 aa     3  A,[aa]
  cmd  A,aa+X    cmd  A,[aa+X]   x+14 aa     4  A,[aa+X]
  cmd  A,!aaaa   cmd  A,[aaaa]   x+05 aa aa  4  A,[aaaa]
  cmd  A,!aaaa+X cmd  A,[aaaa+X] x+15 aa aa  5  A,[aaaa+X]
  cmd  A,!aaaa+Y cmd  A,[aaaa+Y] x+16 aa aa  5  A,[aaaa+Y]
  cmd  A,[aa]+Y  cmd  A,[[aa]+Y] x+17 aa     6  A,[[aa]+Y]
  cmd  A,[aa+X]  cmd  A,[[aa+X]] x+07 aa     6  A,[[aa+X]]
  cmd  aa,bb     cmd  [aa],[bb]  x+09 bb aa  6  [aa],[bb]
  cmd  aa,#nn    cmd  [aa],nn    x+18 nn aa  5  [aa],nn
  cmd  (X),(Y)   cmd  [X],[Y]    x+19        5  [X],[Y]
```

Compare can additionally have the following forms:

```text
  CMP  X,#nn     CMP  X,nn       C8 nn       2  X-nn                   N.....ZC
  CMP  X,aa      CMP  X,[aa]     3E aa       3  X-[aa]                 N.....ZC
  CMP  X,!aaaa   CMP  X,[aaaa]   1E aa aa    4  X-[aaaa]               N.....ZC
  CMP  Y,#nn     CMP  Y,nn       AD nn       2  Y-nn                   N.....ZC
  CMP  Y,aa      CMP  Y,[aa]     7E aa       3  Y-[aa]                 N.....ZC
  CMP  Y,!aaaa   CMP  Y,[aaaa]   5E aa aa    4  Y-[aaaa]               N.....ZC
```

Note: There's also a compare and jump if non-zero command (see jumps).

**8bit Increment/Decrement and Rotate/Shift Commands**

```text
  ASL  a         SHL  a          00+x ..     .. Left shift, bit0=0     N.....ZC
  ROL  a         RCL  a          20+x ..     .. Left shift, bit0=C     N.....ZC
  LSR  a         SHR  a          40+x ..     .. Right shift, bit7=0    N.....ZC
  ROR  a         RCR  a          60+x ..     .. Right shift, bit7=C    N.....ZC
  DEC  a         DEC  a          80+x ..     .. a=a-1                  N.....Z.
  INC  a         INC  a          A0+x ..     .. a=a+1                  N.....Z.
```

The Increment/Decrement and Rotate/Shift commands can have following forms:

```text
  cmd  A         cmd  A          x+1C        2  A
  cmd  X         cmd  X          x+1D-80     2  X    ;\increment/decrement only
  cmd  Y         cmd  Y          x+DC-80     2  Y    ;/(not rotate/shift)
  cmd  aa        cmd  [aa]       x+0B aa     4  [aa]
  cmd  aa+X      cmd  [aa+X]     x+1B aa     5  [aa+X]
  cmd  !aaaa     cmd  [aaaa]     x+0C aa aa  5  [aaaa]
```

Note: There's also a decrement and jump if non-zero command (see jumps).

**16bit ALU Operations**

```text
  ADDW YA,aa     ADDW YA,[aa]    7A aa       5  YA=YA+Word[aa]         NV..H.ZC
  SUBW YA,aa     SUBW YA,[aa]    9A aa       5  YA=YA-Word[aa]         NV..H.ZC
  CMPW YA,aa     CMPW YA,[aa]    5A aa       4  YA-Word[aa]            N.....ZC
  INCW aa        INCW [aa]       3A aa       6  Word[aa]=Word[aa]+1    N.....Z.
  DECW aa        DECW [aa]       1A aa       6  Word[aa]=Word[aa]-1    N.....Z.
  DIV  YA,X      DIV  YA,X       9E         12  A=YA/X, Y=YA MOD X     NV..H.Z.
  MUL  YA        MUL  YA         CF          9  YA=Y*A, NZ on Y only   N.....Z.
```

For ADDW/SUBW, H is carry from bit11 to bit12.

**1bit ALU Operations**

```text
  CLR1 aa.b      CLR  [aa].b     b*20+12 aa  4  [aa].bit_b=0           ........
  SET1 aa.b      SET  [aa].b     b*20+02 aa  4  [aa].bit_b=1           ........
  NOT1 aaa.b     NOT  [aaa].b    EA aa ba    5  invert [aaa].bit_b     ........
  MOV1 aaa.b,C   MOV  [aaa].b,C  CA aa ba    6  [aaa].bit_b=C          ........
  MOV1 C,aaa.b   MOV  C,[aaa].b  AA aa ba    4  C=[aaa].bit_b          .......C
  OR1  C,aaa.b   OR   C,[aaa].b  0A aa ba    5  C=C OR [aaa].bit_b     .......C
  OR1  C,/aaa.b  OR   C,not[].b  2A aa ba    5  C=C OR not[aaa].bit_b  .......C
  AND1 C,aaa.b   AND  C,[aaa].b  4A aa ba    4  C=C AND [aaa].bit_b    .......C
  AND1 C,/aaa.b  AND  C,not[].b  6A aa ba    4  C=C AND not[aaa].bit_b .......C
  EOR1 C,aaa.b   XOR  C,[aaa].b  8A aa ba    5  C=C XOR [aaa].bit_b    .......C
  CLRC           CLR  C          60          2  C=0                    .......0
  SETC           SET  C          80          2  C=1                    .......1
  NOTC           NOT  C          ED          3  C=not C                .......C
  CLRV           CLR  V,H        E0          2  V=0, H=0               .0..0...
```

**Special ALU Operations**

```text
  DAA   A        DAA  A          DF          3  BCD adjust after ADC   N.....ZC
  DAS   A        DAS  A          BE          3  BCD adjust after SBC   N.....ZC
  XCN   A        XCN  A          9F          5  A = (A>>4) | (A<<4)    N.....Z.
  TCLR1 !aaaa    TCLR [aaaa],A   4E aa aa    6  [aaaa]=[aaaa]AND NOT A N.....Z.
  TSET1 !aaaa    TSET [aaaa],A   0E aa aa    6  [aaaa]=[aaaa]OR A      N.....Z.
```

For TCLR/TSET, Z+N flags are set as for "CMP A,[aaaa]" (before [aaaa] gets
changed).

Reportedly, TCLR/TSET can access only 0000h..7FFFh, that is nonsense.

<a id="snesapuspc700cpujumpcontrolcommands"></a>

## SNES APU SPC700 CPU Jump/Control Commands

**Conditional Jumps**

```text
  BPL dest       JNS dest        10 rr         2/4  if N=0 --> JR dest ........
  BMI dest       JS  dest        30 rr         2/4  if N=1 --> JR dest ........
  BVC dest       JNO dest        50 rr         2/4  if V=0 --> JR dest ........
  BVS dest       JO  dest        70 rr         2/4  if V=1 --> JR dest ........
  BCC dest       JNC dest        90 rr         2/4  if C=0 --> JR dest ........
  BCS dest       JC  dest        B0 rr         2/4  if C=1 --> JR dest ........
  BNE dest       JNZ dest        D0 rr         2/4  if Z=0 --> JR dest ........
  BEQ dest       JZ  dest        F0 rr         2/4  if Z=1 --> JR dest ........
  BBS aa.b,dest  JNZ [aa].b,dest b*20+03 aa rr 5/7  if [aa].bit_b=1 -> ........
  BBC aa.b,dest  JZ  [aa].b,dest b*20+13 aa rr 5/7  if [aa].bit_b=0 -> ........
  CBNE aa,dest   CJNE A,[aa],d   2E aa rr      5/7  if A<>[aa]   -->   ........
  CBNE aa+X,dest CJNE A,[aa+X],d DE aa rr      6/8  if A<>[aa+X] -->   ........
  DBNZ Y,dest    DJNZ Y,dest     FE rr         4/6  Y=Y-1, if Y<>0 --> ........
  DBNZ aa,dest   DJNZ [aa],dest  6E aa rr      5/7  [aa]=[aa]-1, if .. ........
```

Aliases: JZ=JE, JNZ=JNE, JC=JAE, JNC=JB.

**Normal Jumps/Calls**

```text
  BRA  dest      JR   dest       2F rr       4  PC=PC+/-rr             ........
  JMP  !aaaa     JMP  aaaa       5F aa aa    3  PC=aaaa                ........
  JMP  [!aaaa+X] JMP  [aaaa+X]   1F aa aa    6  PC=Word[a+X]           ........
  CALL !aaaa     CALL aaaa       3F aa aa    8  [S-1]=PC,S=S-2,PC=aaaa ........
  TCALL n        CALL [FFnn]     n1 ;n=0..F  8  Push PC, PC=[FFDE-n*2] ........
  PCALL uu       PCALL FFnn      4F nn       6  Push PC, PC=FF00..FFFF ........
  RET            RET             6F          5  PC=[S+1],S=S+2         ........
  RET1           RETI            7F          6  Pop Flags, PC          NVPBHIZC
  BRK            BRK             0F          8  Push $+1,PSW,PC=[FFDE] ...1.0..
  /RESET         /RESET          -           ?  PC=[FFFEh]             ..00.0..
```

Note: Calls are pushing the exact retadr (unlike 6502, which pushes retadr-1).

**Wait/Delay/Control**

```text
  NOP            NOP             00          2  do nothing             ........
  SLEEP          SLEEP           EF          ?  Halts the processor    ........
  STOP           STOP            FF          ?  Halts the processor    ........
  CLRP           CLR P           20          2  P=0 ;zero page at 00aa ..0.....
  SETP           SET P           40          2  P=1 ;zero page at 01aa ..1.....
  EI             EI              A0          3  I=1 ;interrupt enable  .....1..
  DI             DI              C0          3  I=0 ;interrupt disable .....0..
```

Note: The SNES APU doesn't have any interrupt sources, so SLEEP/STOP will hang
the CPU forever, and DI/EI have no effect (other than changing to I-flag in
PSW).

<a id="snesapuspc700ioports"></a>

## SNES APU SPC700 I/O Ports

**00F0h - TEST - Testing functions (W)**

```text
  0    Timer-Enable     (0=Normal, 1=Timers don't work)
  1    RAM Write Enable (0=Disable/Read-only, 1=Enable SPC700 & S-DSP writes)
  2    Crash SPC700     (0=Normal, 1=Crashes the CPU)
  3    Timer-Disable    (0=Timers don't work, 1=Normal)
  4-5  Waitstates on RAM Access         (0..3 = 0/1/4/9 cycles) (0=Normal)
  6-7  Waitstates on I/O and ROM Access (0..3 = 0/1/4/9 cycles) (0=Normal)
```

Default setting is 0Ah, software should never change this register. Normal
memory access time is 1 cycle (adding 0/1/4/9 waits gives access times of
1/2/5/10 cycles). Using 4 or 9 waits doesn't work with some opcodes (0 or 1
waits seem to work stable).

Internal cycles (those that do not access RAM, ROM, nor I/O) are either using
the RAM or I/O access time (see notes at bottom of this chapter).

**00F1h - CONTROL - Timer, I/O and ROM Control (W)**

```text
  0-2  Timer 0-2 Enable (0=Disable, set TnOUT=0 & reload divider, 1=Enable)
  3    Not used
  4    Reset Port 00F4h/00F5h Input-Latches (0=No change, 1=Reset to 00h)
  5    Reset Port 00F6h/00F7h Input-Latches (0=No change, 1=Reset to 00h)
        Note: The CPUIO inputs are latched inside of the SPC700 (at time when
        the Main CPU writes them), above two bits allow to reset these latches.
  6    Not used
  7    ROM at FFC0h-FFFFh (0=RAM, 1=ROM) (writes do always go to RAM)
```

On power on or reset, it seems to be set to B0h.

**00F2h - DSPADDR - DSP Register Index (R/W)**

```text
  0-7  DSP Register Number (usually 00h..7Fh) (80h..FFh are read-only mirrors)
```

**00F3h - DSPDATA - DSP Register Data (R/W)**

```text
  0-7  DSP Register Data (read/write the register selected via Port 00F2h)
```

**00F4h - CPUIO0 - CPU Input and Output Register 0 (R and W)**

**00F5h - CPUIO1 - CPU Input and Output Register 1 (R and W)**

**00F6h - CPUIO2 - CPU Input and Output Register 2 (R and W)**

**00F7h - CPUIO3 - CPU Input and Output Register 3 (R and W)**

These registers are used in communication with the 5A22 S-CPU. There are eight
total registers accessed by these four addresses: four write-only output ports
to the S-CPU and four read-only input ports from the S-CPU.

```text
  0-7  Data written to/read from corresponding register on main 5A22 CPU
```

If the SPC700 writes to an output port while the S-CPU is reading it, the S-CPU
will read the logical OR of the old and new values. Possibly the same thing
happens the other way around, but the details are unknown?

**00F8h - AUXIO4 - External I/O Port P4 (S-SMP Pins 34-27) (R/W) (unused)**

**00F9h - AUXIO5 - External I/O Port P5 (S-SMP Pins 25-18) (R/W) (unused)**

Writing changes the output levels. Reading normally returns the same value as
the written value; unless external hardware has pulled the pins LOW or HIGH.
Reading from AUXIO5 additionally produces a short /P5RD read signal (Pin17).

```text
  0-7  Input/Output levels (0=Low, 1=High)
```

In the SNES, these pins are unused (not connected), so the registers do
effectively work as if they'd be "RAM-like" general purpose storage registers.

**00FAh - T0DIV - Timer 0 Divider (for 8000Hz clock source) (W)**

**00FBh - T1DIV - Timer 1 Divider (for 8000Hz clock source) (W)**

**00FCh - T2DIV - Timer 2 Divider (for 64000Hz clock source) (W)**

```text
  0-7  Divider (01h..FFh=Divide by 1..255, or 00h=Divide by 256)
```

If timers are enabled (via Port 00F1h), then the TnOUT registers are
incremented at the selected rate (ie. the 8kHz or 64kHz clock source, divided
by the selected value).

**00FDh - T0OUT - Timer 0 Output (R)**

**00FEh - T1OUT - Timer 1 Output (R)**

**00FFh - T2OUT - Timer 2 Output (R)**

```text
  0-3  Incremented at the rate selected via TnDIV (reset to 0 after reading)
  4-7  Not used (always zero)
```

**SPC700 Waitstates on Internal Cycles**

Below lists the number of I/O-Waitstates applied on Internal Cycles of SPC700
opcodes 00h..FFh (that implies: any further Internal Cycles have
RAM-Waitstates).

```text
  00..1F  0,3,0,1,0,0,0,1,0,0,1,0,0,1,0,2, 2,3,0,3,1,1,1,1,0,0,0,1,0,0,0,1
  20..3F  0,3,0,1,0,0,0,1,0,0,1,0,0,1,3,2, 0,3,0,3,1,1,1,1,0,0,0,1,0,0,0,3
  40..5F  0,3,0,1,0,0,0,1,0,0,0,0,0,1,0,3, 2,3,0,3,1,1,1,1,0,0,0,1,0,0,0,0
  60..7F  0,3,0,1,0,0,0,1,0,1,0,0,0,1,2,1, 0,3,0,3,1,1,1,1,1,1,1,1,0,0,0,1
  80..9F  0,3,0,1,0,0,0,1,0,0,1,0,0,0,1,0, 2,3,0,3,1,1,1,1,0,0,1,1,0,0,10,3
  A0..BF  1,3,0,1,0,0,0,1,0,0,0,0,0,0,1,1, 0,3,0,3,1,1,1,1,0,0,1,1,0,0,1,1
  C0..DF  1,3,0,1,0,0,0,1,0,0,1,0,0,0,1,7, 2,3,0,3,1,1,1,1,0,1,0,1,0,0,4,1
  E0..FF  0,3,0,1,0,0,0,1,0,0,0,0,0,1,1,0, 0,3,0,3,1,1,1,1,0,1,0,1,0,0,3,0
  Note: For conditional jumps (with condition=true), add 2 additional cylces.
```

That is, the above list_entries are meant to be used like so:

```text
  number_of_I/O_waits = list_entry
  number_of_RAM_waits = total_number_of_internal_cycles - list_entry
```

For example, Opcode 00h (NOP) has one internal cycle (and it's having RAM
timings). Opcode 01h (TCALL) has 3 internal cycles (and all 3 of them have I/O
timings).

<a id="snesapumaincpucommunicationport"></a>

## SNES APU Main CPU Communication Port

**2140h - APUI00  - Main CPU to Sound CPU Communication Port 0 (R/W)**

**2141h - APUI01  - Main CPU to Sound CPU Communication Port 1 (R/W)**

**2142h - APUI02  - Main CPU to Sound CPU Communication Port 2 (R/W)**

**2143h - APUI03  - Main CPU to Sound CPU Communication Port 3 (R/W)**

```text
  7-0   APU I/O Data   (Write: Data to APU, Read: Data from APU)
```

Caution: These registers should be written only in 8bit mode (there is a
hardware glitch that can cause a 16bit write to [2140h..2141h] to destroy
[2143h], this might happen only in some situations, like when the cartridge
contains too many ROM chips which apply too much load on the bus).

**Uploader**

```text
  Wait until Word[2140h]=BBAAh
  kick=CCh                  ;start-code for first command
  for block=1..num_blocks
    Word[2142h]=dest_addr   ;usually 200h or higher (above stack and I/O ports)
    Byte[2141h]=01h         ;command=transfer (can be any non-zero value)
    Byte[2140h]=kick        ;start command (CCh on first block)
    Wait until Byte[2140h]=kick
    for index=0 to length-1
      Byte[2141h]=[src_addr+index]      ;send data byte
      Byte[2140h]=index.lsb             ;send index LSB (mark data available)
      Wait until Byte[2140h]=index.lsb  ;wait for acknowledge (see CAUTION)
    next index
    kick=(index+2 AND FFh) OR 1 ;-kick for next command (must be bigger than
  next block  ;(if any)         ;         last index+1, and must be non-zero)
  [2142h]=entry_point           ;entrypoint, must be below FFC0h (ROM region)
  [2141h]=00h                   ;command=entry (must be zero value)
  [2140h]=kick                  ;start command
  Wait until Byte[2140h]=kick   ;wait for acknowledge
```

CAUTION: The acknowledge for the last data byte lasts only for a few clock
cycles, if the uploader is too slow then it may miss it (for example if it's
interrupted); as a workaround, disable IRQs and NMIs during upload, or replace
the last "wait for ack" by hardcoded delay.

**Boot ROM Disassembly**

```text
  FFC0 CD EF        mov  x,EF           ;\
  FFC2 BD           mov  sp,x           ; zerofill RAM at [0001h..00EFh]
  FFC3 E8 00        mov  a,00           ; (ie. excluding I/O Ports at F0h..FFh)
                   @@zerofill_lop:      ; (though [00h..01h] destroyed below)
  FFC5 C6           mov  [x],a          ; (also sets stacktop to 01EFh, kinda
  FFC6 1D           dec  x              ; messy, nicer would be stacktop 01FFh)
  FFC7 D0 FC        jnz  @@zerofill_lop ;/
  FFC9 8F AA F4     mov  [F4],AA        ;\notify Main CPU that APU is ready
  FFCC 8F BB F5     mov  [F5],BB        ;/for communication
                   @@wait_for_cc:       ;\
  FFCF 78 CC F4     cmp  [F4],CC        ; wait for initial "kick" value
  FFD2 D0 FB        jnz  @@wait_for_cc  ;/
  FFD4 2F 19        jr   main
                   ;---
                   @@transfer_data:
                   @@wait_for_00:                               ;\
  FFD6 EB F4        mov  y,[F4]     ;index (should become 0)    ;
  FFD8 D0 FC        jnz  @@wait_for_00                          ;/
                   @@transfer_lop:
  FFDA 7E F4        cmp  y,[F4]
  FFDC D0 0B        jnz  FFE9          ------->
  FFDE E4 F5        mov  a,[F5]     ;get data
  FFE0 CB F4        mov  [F4],y     ;ack data
  FFE2 D7 00        mov  [[00]+y],a ;store data
  FFE4 FC           inc  y          ;addr lsb
  FFE5 D0 F3        jnz  @@transfer_lop
  FFE7 AB 01        inc  [01]       ;addr msb
                   @@
  FFE9 10 EF        jns  @@transfer_lop     ;strange...
  FFEB 7E F4        cmp  y,[F4]
  FFED 10 EB        jns  @@transfer_lop
                   ;- - -
                   main:
  FFEF BA F6        movw ya,[F6]                ;\copy transfer (or entrypoint)
  FFF1 DA 00        movw [00],ya    ;addr       ;/address to RAM at [0000h]
  FFF3 BA F4        movw ya,[F4]    ;cmd:kick
  FFF5 C4 F4        mov  [F4],a     ;ack kick
  FFF7 DD           mov  a,y        ;cmd
  FFF8 5D           mov  x,a        ;cmd
  FFF9 D0 DB        jnz  @@transfer_data
  FFFB 1F 00 00     jmp  [0000+x]   ;in: A=0, X=0, Y=0, SP=EFh, PSW=02h
                   ;---
  FFFE C0 FF        dw   FFC0  ;reset vector
```

**Notes**

Many games are jumping to ROM:FFC0h in order to "reset" the SPC700 back to the
transmission phase. Some programs are also injumping to ROM:FFC9h (eg. mic_'s
"The 700 Club" demo is using that address as dummy-entrypoint after each
transfer block; until applying the actual entrypoint after the last block).

Some games may "wrap" from opcodes at RAM:FFBFh to opcodes at ROM:FFC0h. Only
known example is World Cup Striker, which contains following code at FFBCh in
RAM: "MOV A,80h, MOV [F1h],A", that ensures ROM enabled, and does then continue
at FFC0h in ROM (in this specific case, emulators can cheat by handling wraps
in their PortF1h handler, rather than checking for PC&gt;=FFC0h after every
single opcode fetch).

<a id="snesapudspbrrsamples"></a>

## SNES APU DSP BRR Samples

**x4h - VxSRCN - Source number for Voice 0..7 (R/W)**

```text
  0-7   Instrument number (index in DIR table)
```

Points to the BRR Start &amp; Loop addresses (via a table entry at
VxSRCN*4+DIR*100h), used when voices are Keyed-ON or Looped.

**5Dh - DIR - Sample table address (R/W)**

```text
  0-7   Sample Table Address (in 256-byte steps) (indexed via VxSRCN)
```

The table can contain up to 256 four-byte entries (max 1Kbyte). Each entry is:

```text
  Byte 0-1  BRR Start Address (used when voice is Keyed-ON)
  Byte 2-3  BRR Restart/Loop Address (used when end of BRR data reached)
```

Changing DIR or VxSRCN has no immediate effect (until/unless voices are newly
Looped or Keyed-ON).

**Bit Rate Reduction (BRR) Format**

The sample data consists of 9-byte block(s). The first byte of each block is:

```text
  7-4  Shift amount   (0=Silent, 12=Loudest, 13-15=Reserved)
  3-2  Filter number  (0=None, 1..3=see below)
  1-0  Loop/End flags (0..3=see below)
```

The next 8 bytes contain two samples (or nibbles) each:

```text
  7-4  First Sample  (signed -8..+7)
  3-0  Second Sample (signed -8..+7)
```

The Loop/End bits can have following values:

```text
  Code 0 = Normal   (continue at next 9-byte block)
  Code 1 = End+Mute (jump to Loop-address, set ENDx flag, Release, Env=000h)
  Code 2 = Ignored  (same as Code 0)
  Code 3 = End+Loop (jump to Loop-address, set ENDx flag)
```

The Shift amount is used to convert the 4bit nibbles to 15bit samples:

```text
  sample = (nibble SHL shift) SAR 1
  Accordingly, shift=0 is rather useless (since it strips the low bit).
  When shift=13..15, decoding works as if shift=12 and nibble=(nibble SAR 3).
```

The Filter bits allow to select the following filter modes:

```text
  Filter 0: new = sample
  Filter 1: new = sample + old*0.9375
  Filter 2: new = sample + old*1.90625  - older*0.9375
  Filter 3: new = sample + old*1.796875 - older*0.8125
```

More precisely, the exact formulas are:

```text
  Filter 0: new = sample
  Filter 1: new = sample + old*1+((-old*1) SAR 4)
  Filter 2: new = sample + old*2+((-old*3) SAR 5)  - older+((older*1) SAR 4)
  Filter 3: new = sample + old*2+((-old*13) SAR 6) - older+((older*3) SAR 4)
```

When creating BRR data, take care that "new" does never exceed -3FFAh..+3FF8h,
otherwise a number of hardware glitches will occur:

```text
  If new>+7FFFh then new=+7FFFh (but, clipped to +3FFFh below) ;\clamp 16bit
  If new<-8000h then new=-8000h (but, clipped to ZERO below)   ;/(dirt-effect)
  If new=(+4000h..+7FFFh) then new=(-4000h..-1)                ;\clip 15bit
  If new=(-8000h..-4001h) then new=(-0..-3FFFh)                ;/(lost-sign)
  If new>+3FF8h OR new<-3FFAh then overflows can occur in Gauss section
```

The resulting 15bit "new" value is then passed to the Gauss filter, and
additionally re-used for the next 1-2 sample(s) as "older=old, old=new".

**BRR Notes**

The first 9-byte BRR sample block should always use Filter 0 (so it isn't
disturbed by uninitialized old/older values). Same for the first block at the
Loop address (unless the old/older values of the initial-pass should happen to
match the ending values of the looped-passes).

<a id="snesapudspbrrpitch"></a>

## SNES APU DSP BRR Pitch

**x2h - VxPITCHL - Pitch scaler for Voice 0..7, lower 8bit (R/W)**

**x3h - VxPITCHH - Pitch scaler for Voice 0..7, upper 6bit (R/W)**

```text
  0-13  Sample rate (0=stop, 3FFFh=fastest) (1000h = 32000Hz)
  14-15 Not used (read/write-able)
```

Defines the BRR sample rate. This register (and PMON) does affect only the BRR
sample frequency (but not on the Noise frequency, which is defined - and shared
for all voices - in the FLG register).

**2Dh - PMON - Pitch Modulation Enable Flags for Voice 1..7 (R/W)**

Pitch modulation allows to generate "Frequency Sweep" effects by mis-using the
amplitude from channel (x-1) as pitch factor for channel (x).

```text
  0    Not used
  1-7  Flags for Voice 1..7 (0=Normal, 1=Modulate by Voice 0..6)
```

For example, output a very loud 1Hz sine-wave on channel 4 (with Direct
Gain=40h, and with Left/Right volume=0; unless you actually want to output it
to the speaker). Then additionally output a 2kHz sine wave on channel 5 with
PMON.Bit5 set. The "2kHz" sound should then repeatedly sweep within 1kHz..3kHz
range (or, for a more decent sweep in 1.8kHz..2.2kHz range, drop the Gain level
of channel 4).

**Pitch Counter**

The pitch counter is adjusted at 32000Hz rate as follows:

```text
  Step = VxPitch                   ;range 0..3FFFh (0..128 kHz)
  IF PMON.Bit(x)=1 AND (x>0)       ;pitch modulation enable
    Factor = VxOUTX(x-1)           ;range -4000h..+3FFFh (prev voice amplitude)
    Factor = (Factor SAR 4)+400h   ;range +000h..+7FFh (factor = 0.00 .. 1.99)
    Step = (Step * Factor) SAR 10  ;range 0..7FEEh (0..256 kHz)
 XXX somewhere here, STEP (or the COUNTER-RESULT) is cropped to 128kHz max) XX?
  Counter = Counter + Step         ;range 0..FFFFh, carry=next BRR block
```

Counter.Bit15-12 indicates the current sample (within a BRR block).

Counter.Bit11-3 are used as gaussian interpolation index.

**Maximum Sound Frequency**

The pitch counter generates sample rates up to 128kHz. However, the Mixer and
DAC are clocked at 32kHz, so the higher rates will skip (or interpolate)
samples rather than outputting all samples.

The 32kHz output rate means that one can produce max 16kHz tones.

**4-Point Gaussian Interpolation**

Interpolation is applied on the 4 most recent 15bit BRR samples
(new,old,older,oldest), using bit4-11 of the pitch counter as interpolation
index (i=00h..FFh):

```text
  out =       ((gauss[0FFh-i] * oldest) SAR 10) ;-initial 16bit value
  out = out + ((gauss[1FFh-i] * older)  SAR 10) ;-no 16bit overflow handling
  out = out + ((gauss[100h+i] * old)    SAR 10) ;-no 16bit overflow handling
  out = out + ((gauss[000h+i] * new)    SAR 10) ;-with 16bit overflow handling
  out = out SAR 1                               ;-convert 16bit result to 15bit
```

The Gauss table contains the following values (in hex):

```text
  000,000,000,000,000,000,000,000,000,000,000,000,000,000,000,000  ;\
  001,001,001,001,001,001,001,001,001,001,001,002,002,002,002,002  ;
  002,002,003,003,003,003,003,004,004,004,004,004,005,005,005,005  ;
  006,006,006,006,007,007,007,008,008,008,009,009,009,00A,00A,00A  ;
  00B,00B,00B,00C,00C,00D,00D,00E,00E,00F,00F,00F,010,010,011,011  ;
  012,013,013,014,014,015,015,016,017,017,018,018,019,01A,01B,01B  ; entry
  01C,01D,01D,01E,01F,020,020,021,022,023,024,024,025,026,027,028  ; 000h..0FFh
  029,02A,02B,02C,02D,02E,02F,030,031,032,033,034,035,036,037,038  ;
  03A,03B,03C,03D,03E,040,041,042,043,045,046,047,049,04A,04C,04D  ;
  04E,050,051,053,054,056,057,059,05A,05C,05E,05F,061,063,064,066  ;
  068,06A,06B,06D,06F,071,073,075,076,078,07A,07C,07E,080,082,084  ;
  086,089,08B,08D,08F,091,093,096,098,09A,09C,09F,0A1,0A3,0A6,0A8  ;
  0AB,0AD,0AF,0B2,0B4,0B7,0BA,0BC,0BF,0C1,0C4,0C7,0C9,0CC,0CF,0D2  ;
  0D4,0D7,0DA,0DD,0E0,0E3,0E6,0E9,0EC,0EF,0F2,0F5,0F8,0FB,0FE,101  ;
  104,107,10B,10E,111,114,118,11B,11E,122,125,129,12C,130,133,137  ;
  13A,13E,141,145,148,14C,150,153,157,15B,15F,162,166,16A,16E,172  ;/
  176,17A,17D,181,185,189,18D,191,195,19A,19E,1A2,1A6,1AA,1AE,1B2  ;\
  1B7,1BB,1BF,1C3,1C8,1CC,1D0,1D5,1D9,1DD,1E2,1E6,1EB,1EF,1F3,1F8  ;
  1FC,201,205,20A,20F,213,218,21C,221,226,22A,22F,233,238,23D,241  ;
  246,24B,250,254,259,25E,263,267,26C,271,276,27B,280,284,289,28E  ;
  293,298,29D,2A2,2A6,2AB,2B0,2B5,2BA,2BF,2C4,2C9,2CE,2D3,2D8,2DC  ;
  2E1,2E6,2EB,2F0,2F5,2FA,2FF,304,309,30E,313,318,31D,322,326,32B  ; entry
  330,335,33A,33F,344,349,34E,353,357,35C,361,366,36B,370,374,379  ; 100h..1FFh
  37E,383,388,38C,391,396,39B,39F,3A4,3A9,3AD,3B2,3B7,3BB,3C0,3C5  ;
  3C9,3CE,3D2,3D7,3DC,3E0,3E5,3E9,3ED,3F2,3F6,3FB,3FF,403,408,40C  ;
  410,415,419,41D,421,425,42A,42E,432,436,43A,43E,442,446,44A,44E  ;
  452,455,459,45D,461,465,468,46C,470,473,477,47A,47E,481,485,488  ;
  48C,48F,492,496,499,49C,49F,4A2,4A6,4A9,4AC,4AF,4B2,4B5,4B7,4BA  ;
  4BD,4C0,4C3,4C5,4C8,4CB,4CD,4D0,4D2,4D5,4D7,4D9,4DC,4DE,4E0,4E3  ;
  4E5,4E7,4E9,4EB,4ED,4EF,4F1,4F3,4F5,4F6,4F8,4FA,4FB,4FD,4FF,500  ;
  502,503,504,506,507,508,50A,50B,50C,50D,50E,50F,510,511,511,512  ;
  513,514,514,515,516,516,517,517,517,518,518,518,518,518,519,519  ;/
```

The gauss table is slightly bugged: Theoretically, each four values
(gauss[000h+i], gauss[0FFh-i], gauss[100h+i], gauss[1FFh-i]) should sum up to
800h, but in practice they do sum up to 7FFh..801h. Of which, 801h can cause
math overflows. For example, when outputting three or more "-8 SHL 12" BRR
samples with Filter 0, some interpolation results will be +3FF8h (instead of
-4000h).

When adding the four new,old,older,oldest values there's some partial overflow
handling: The 1st addition can't overflow, the 2nd addition can overflow (when
i=0..1Fh), the 3rd addition can overflow (when i=20h..FFh). Of which, the 2nd
one bugs, the 3rd one is saturated to Min=-8000h/Max=+7FFFh (giving
-4000h/+3FFFh after the final SAR 1).

**Waveform Examples**

```text
  Incoming BRR Data ---> Interpolated Data
   _   _   _   _
  | | | | | | | |         .   .   .   .    Nibbles=79797979, Shift=12, Filter=0
  | | | | | | | |   ---> / \ / \ / \ / \   HALF-volume ZIGZAG-wave
  | |_| |_| |_| |_          '   '   '   '
   ___     ___
  |   |   |   |            .'.     .'.     Nibbles=77997799, Shift=12, Filter=0
  |   |   |   |     --->  /   \   /   \    FULL-volume SINE-wave
  |   |___|   |___       '     '.'     '.
   _______                   ___
  |       |                .'   '.         Nibbles=77779999, Shift=12, Filter=0
  |       |         --->  /       \        SQUARE wave (with rounded edges)
  |       |_______       '         '.____
   _______                   ___
  |       |                .'   '.   | |   Nibbles=77778888, Shift=12, Filter=0
  |       |         --->  /       \  | |   OVERFLOW glitch on -4000h*801h
  |       |_______       '         '.|_|_
   _____         _           __
  |     |_     _|          .'  ''.    .'   Nibbles=7777CC44, Shift=12, Filter=0
  |       |___|     --->  /       '..'     CUSTOM wave-form
  |                      '
   ___     __
  |   |___|  |    _       \ ! /  .  \ ! /  Nibbles=77DE9HZK, Shift=3M, Filter=V
  |_     ____|  _|  --->  - + -  +  - + -  SOLAR STORM wave-form
  __|   |______|___       / ! \  '  / ! \
```

<a id="snesapudspadsrgainenvelope"></a>

## SNES APU DSP ADSR/Gain Envelope

**x5h - VxADSR1 - ADSR settings for Voice 0..7, lower 8bit (R/W)**

**x6h - VxADSR2 - ADSR settings for Voice 0..7, upper 8bit (R/W)**

```text
  0-3   4bit Attack rate   ;Rate=N*2+1, Step=+32 (or Step=+1024 when Rate=31)
  4-6   3bit Decay rate    ;Rate=N*2+16, Step=-(((Level-1) SAR 8)+1)
  7     ADSR/Gain Select   ;0=Use VxGAIN, 1=Use VxADSR (Attack/Decay/Sustain)
  8-12  5bit Sustain rate  ;Rate=N, Step=-(((Level-1) SAR 8)+1)
  13-15 3bit Sustain level ;Boundary=(N+1)*100h
  N/A   0bit Release rate  ;Rate=31, Step=-8 (or Step=-800h when BRR-end)
```

If a voice is Keyed-ON: Enter Attack mode, set Level=0, and increase level. At
Level&gt;=7E0h: Switch from Attack to Decay mode (and clip level to max=7FFh if
level&gt;=800h). At Level&lt;=Boundary: Switch from Decay to Sustain mode.

If the voice is Keyed-OFF (or the BRR sample contains the Key-Off flag): Switch
from Attack/Decay/Sustain/Gain mode to Release mode.

**x7h - VxGAIN - Gain settings for Voice 0..7 (R/W)**

When VxADSR1.Bit7=1 (Attack/Decay/Sustain Mode):

```text
  0-7   Not used (instead, Attack/Decay/Sustain parameters are used)
```

When VxADSR1.Bit7=0 and VxGAIN.Bit7=0 (Direct Gain):

```text
  0-6   Fixed Volume (Envelope Level = N*16, Rate=Infinite)  ;Volume=N*16/800h
  7     Must be 0 for this mode
```

When VxADSR1.Bit7=0 and VxGAIN.Bit7=1 (Custom Gain):

```text
  0-4   Gain rate (Rate=N)
  5-6   Gain mode (see below)
  7     Must be 1 for this mode
```

In the latter case, the four Gain Modes are:

```text
  Mode 0 = Linear Decrease  ;Rate=N, Step=-32  (if Level<0 then Level=0)
  Mode 1 = Exp Decrease     ;Rate=N, Step=-(((Level-1) SAR 8)+1)
  Mode 2 = Linear Increase  ;Rate=N, Step=+32
  Mode 3 = Bent Increase    ;Rate=N, If Level<600h then Step=+32 else Step=+8
  In all cases, clip E to 0 or 0x7ff rather than wrapping.
```

The Direct/Custom Gain modes are overriding Attack/Decay/Sustain modes (when
Keyed-ON), Release (when Keyed-OFF) keeps working as usually.

**x8h - VxENVX - Current envelope value for Voice 0..7 (R)**

```text
  0-6  Upper 7bit of the 11bit envelope volume (0..127)
  7    Not used (zero)
```

Technically, this register IS writable. But whatever value you write will be
overwritten at 32000 Hz.

**x9h - VxOUTX - Current sample value for Voice 0..7 (R)**

This returns the high byte of the current sample for this voice, after envelope
volume adjustment but before VxVOL[LR] is applied.

```text
  0-7  Upper 8bit of the current 15bit sample value (-128..+127)
```

Technically, this register IS writable. But whatever value you write will be
overwritten at 32000 Hz.

**ADSR/Gain (and Noise) Rates**

The various ADSR/Gain/Noise rates are defined as 5bit rate values (or 3bit/4bit
values which are converted to 5bit values as described above). The meaning of
these 5bit values is as follows (the table shows the number of 32000Hz sample
units it takes until the next Step is applied):

```text
  00h=Stop   04h=1024  08h=384   0Ch=160   10h=64   14h=24   18h=10   1Ch=4
  01h=2048   05h=768   09h=320   0Dh=128   11h=48   15h=20   19h=8    1Dh=3
  02h=1536   06h=640   0Ah=256   0Eh=96    12h=40   16h=16   1Ah=6    1Eh=2
  03h=1280   07h=512   0Bh=192   0Fh=80    13h=32   17h=12   1Bh=5    1Fh=1
```

Note: All values are "1,3,5 SHL n". Hardware-wise they are probably implemented
as 3bit counters, driven at 32kHz, 16kHz, 8kKz, etc. (depending on the shift
amount); accordingly, when entering a new ADSR phase, the location of the 1st
step may vary depending on when the hardware generates the next 8kHz cycles,
for example.

**Gain Notes**

Even in Gain modes, the hardware does internally keep track of whether it is in
Attack/Decay/Sustain phase: In attack phase it switches to Decay at
Level&gt;=7E0h. In Decay phase it switches to Sustain at Level&lt;=Boundary
(though accidently reading a garbage boundary value from VxGAIN.Bit7-5 instead
of from VxADSR2.Bit7-5). Whereas, switching phases doesn't have any audible
effect, it just marks the hardware as being in a new phase (and, if software
disables Gain-Mode by setting VxADSR1.Bit7, then hardware will process the
current phase).

Direct Gain mode can be used to set a fixed volume, or to define the starting
point for a different mode (in the latter case, mind that the hardware
processes I/O ports only once per sample, so, after setting Direct Gain mode:
wait at least 32 cpu cycles before selecting a different mode).

**Misc Notes**

```text
  Save the new value, *clipped* to 11 bits, to determine the
  increment for GAIN Bent Increase mode next sample. Note that a
  negative value for the new value will result in the clipped
  version being greater than 0x600.
```

<a id="snesapudspvolumeregisters"></a>

## SNES APU DSP Volume Registers

**0Ch - MVOLL - Left channel master volume (R/W)**

**1Ch - MVOLR - Right channel master volume (R/W)**

```text
  0-7   Volume (-127..+127) (negative = phase inverted) (sample=sample*vol/128)
```

Value -128 causes multiply overflows (-8000h*-80h=-400000h).

**x0h - VxVOLL - Left volume for Voice 0..7 (R/W)**

**x1h - VxVOLR - Right volume for Voice 0..7 (R/W)**

```text
  0-7   Volume (-128..+127) (negative = phase inverted) (sample=sample*vol/128)
```

Value -128 can be safely used (unlike as for all other volume registers, there
is no overflow; because ADSR/Gain does lower the incoming sample to
max=sample*7FFh/800h).

**Output Mixer**

```text
  sum =       sample0*V0VOLx SAR 6      ;\
  sum = sum + sample1*V0V1Lx SAR 6      ;
  sum = sum + sample2*V0V2Lx SAR 6      ; with 16bit overflow handling
  sum = sum + sample3*V0V3Lx SAR 6      ; (after each addition)
  sum = sum + sample4*V0V4Lx SAR 6      ;
  sum = sum + sample5*V0V5Lx SAR 6      ;
  sum = sum + sample6*V0V6Lx SAR 6      ;
  sum = sum + sample7*V0V7Lx SAR 6      ;
  sum =       (sum*MVOLx SAR 7)         ;
  sum = sum + (fir_out*EVOLx SAR 7)     ;/
  if FLG.MUTE then sum = 0000h
  sum = sum XOR FFFFh  ;-final phase inversion (as done by built-in post-amp)
```

<a id="snesapudspcontrolregisters"></a>

## SNES APU DSP Control Registers

**4Ch - KON - Key On Flags for Voice 0..7 (R/W) (W)**

```text
  0-7  Flags for Voice 0..7 (0=No change, 1=Key On)
```

```text
        Writing 1 to the KON bit will set the envelope to 0, the state to
        Attack, and will start the channel from the beginning (see DIR and
        VxSRCN). Note that this happens even if the channel is already playing
        (which may cause a click/pop), and that there are 5 'empty' samples
        before envelope updates and BRR decoding actually begin.
```

**5Ch - KOFF - Key Off Flags for Voice 0..7 (R/W) (W)**

```text
  0-7  Flags for Voice 0..7 (0=No change, 1=Key Off)
```

```text
        Setting 1 to the KOFF bit will transition the voice to the Release
        state. Thus, the envelope will decrease by 8 every sample (regardless
        of the VxADSR and VxGAIN settings) until it reaches 0, where it will
        stay until the next KON.
```

**6Ch - FLG - Reset, Mute, Echo-Write flags and Noise Clock (R/W)**

The initial value on Reset is E0h (KeyedOff/Muted/EchoWriteOff/NoiseStopped),
although reading the FLG register after reset will return a garbage value.

```text
  0-4  Noise frequency    (0=Stop, 1=16Hz, 2=21Hz, ..., 1Eh=16kHz, 1Fh=32kHz)
  5    Echo Buffer Writes (0=Enable, 1=Disable) (doesn't disable echo-reads)
  6    Mute Amplifier     (0=Normal, 1=Mute) (doesn't stop internal processing)
  7    Soft Reset         (0=Normal, 1=KeyOff all voices, and set Envelopes=0)
```

Disabling Echo-Writes doesn't affect Echo-Reads (ie. echo buffer is still
output, unless Echo Volume L+R or FIR0..7 are set to zero). Mute affects only
the external amplifier (internally all sound/echo generation is kept
operating).

**7Ch - ENDX - Voice End Flags for Voice 0..7 (R) (W=Ack)**

```text
  0-7  Flags for Voice 0..7 (0=Keyed ON, 1=BRR-End-Bit encountered)
```

Any write to this register will clear ALL bits, no matter what value is
written. On reset, all bits are cleared. Though bits may get set shortly after
reset or shortly after manual-write (once when the BRR decoder reaches an
End-code).

```text
        Note that the bit is set at the START of decoding the BRR block, not
        at the end. Recall that BRR processing, and therefore the setting of
        bits in this register, continues even for voices in the Release state.
```

**3Dh - NON - Noise Enable Flags for Voice 0..7 (R/W)**

Allows to enable Noise on one or more channels, however, there is only one
noise-generator (and only one noise frequency) shared for all voices. The 5bit
noise frequency is defined in FLG register (see above), and works same as the
5bit ADSR rates (see ADSR chapter for details). The NON bits are:

```text
  0-7  Flags for Voice 0..7 (0=Output BRR Samples, 1=Output Noise)
```

The noise generator produces 15bit output level (range -4000h..+3FFFh;
initially -4000h after reset). At the selected noise rate, the level is updated
as follows:

```text
  Level = ((Level SHR 1) AND 3FFFh) OR ((Level.Bit0 XOR Level.Bit1) SHL 14)
```

Caution: Even in noise mode, the hardware keeps decoding BRR sample blocks,
and, when hitting a BRR block with End Code 1 (End+Mute), it will switch to
Release state with Envelope=0, thus immediately terminating the Noise output.
To avoid that, set VxSRCN to an endless looping dummy BRR block.

Note: The VxPITCH registers have no effect on noise frequency, and Gaussian
interpolation isn't applied on noise.

**KON/KOFF Notes**

```text
        These registers seem to be polled only at 16000 Hz, when every other
        sample is due to be output. Thus, if you write two values in close
        succession, usually but not always only the second value will have an
        effect:
          ; assume KOFF = 0, but no voices playing
          mov $f2, #$4c  ; KON = 1 then KON = 2
          mov $f3, #$01  ; -> *usually* only voice 2 is keyed on. If both are,
          mov $f3, #$02  ; voice 1 will be *2* samples ahead rather than one.
        and
          ; assume various voices playing
          mov $f2, #$5c  ; KOFF = $ff then KOFF = 0
          mov $f3, #$ff
          mov $f3, #$00  ; -> *usually* all voices remain playing
        FLG bit 7, however, is polled every sample and polled for each voice.
```

```text
        These registers and FLG bit 7 interact as follows:
          1. If FLG bit 7 or the KOFF bit for the channel is set, transition
             to the Release state. If FLG bit 7 is set, also set the envelope
             to 0.
          2. If the 'internal' value of KON has the channel's bit set, perform
             the KON actions described above.
          3. Set the 'internal' value of KON to 0.
```

```text
        This has a number of consequences:
          * KON effectively takes effect 'on write', even though a non-zero
            value can be read back much later. KOFF and FLG.7, on the other
            hand, exert their influence constantly until a new value is
            written.
          * Writing KON while KOFF or FLG.7 will not result in any samples
            being output by the channel. The channel is keyed on, but it is
            turned off again 2 samples later. Since there is a 5 sample delay
            after KON before the channel actually beings processing, the net
            effect is no output.
          * However, if KOFF is cleared within 63 SPC700 cycles of the
            KON write above, the channel WILL be keyed on as normal. If KOFF
            is cleared betwen 64 and 127 SPC700 cycles later, the channel
            MIGHT be keyed on with decreasing probability depending on how
            many cycles before the KON/KOFF poll the KON write occurred.
          * Setting both KOFF and KON for a channel will turn the channel
            off much faster than just KOFF alone, since the KON will set the
            envelope to 0. This can cause a click/pop, though.
```

**xAh - NA - Unused (8 bytes of general-purpose RAM) (R/W)**

**xBh - NA - Unused (8 bytes of general-purpose RAM) (R/W)**

**1Dh - NA - Unused (1 byte of general-purpose RAM) (R/W)**

**xEh - NA - Unused (8 bytes of general-purpose RAM) (R/W)**

These registers seem to have no function at all. Data written to them seems to
have no effect on sound output, the written values seem to be left intact (ie.
they aren't overwritten by voice or echo status information).

<a id="snesapudspechoregisters"></a>

## SNES APU DSP Echo Registers

**2Ch - EVOLL - Left channel echo volume (R/W)**

**3Ch - EVOLR - Right channel echo volume (R/W)**

```text
  0-7   Volume (-128..+127) (negative = phase inverted) (sample=sample*vol/128)
```

This is the adjustment applied to the FIR filter outputs before mixing with the
main signal (after master volume adjustment).

**0Dh - EFB - Echo feedback volume (R/W)**

Specifies the feedback volume (the volume at which the echo-output is mixed
back to the echo-input).

```text
  0-7   Volume (-128..+127) (negative = phase inverted) (sample=sample*vol/128)
```

Medium values (like 40h) produce "normal" echos (with decreasing volume on each
repetition). Value 00h would produce a one-shot echo, value 7Fh would repeat
the echo almost infinitely (assuming that FIRx leaves the overall volume
unchanged).

**4Dh - EON - Echo Enable Flags for Voice 0..7 (R/W)**

```text
  0-7  Flags for Voice 0..7 (0=Direct Output, 1=Echo (and Direct) Output)
```

When the bit is set and echo buffer write is enabled, this voice will be mixed
into the sample to be written to the echo buffer for later echo processing.

**6Dh - ESA - Echo ring buffer address (R/W)**

```text
  0-7   Echo Buffer Base Address (in 256-byte steps)
```

The echo buffer consists of one or more 4-byte entries:

```text
  Byte 0: Lower 7bit of Left sample  (stored in bit1-7) (bit0=unused/zero)
  Byte 1: Upper 8bit of Left sample  (stored in bit0-7)
  Byte 2: Lower 7bit of Right sample (stored in bit1-7) (bit0=unused/zero)
  Byte 3: Upper 8bit of Right sample (stored in bit0-7)
```

At 32000Hz rate, the oldest 4-byte entry is removed (and passed on to the FIR
filter), and, if echo-writes are enabled in FLG register, the removed entry is
then overwritten by new values (from the Echo Mixer; consisting of all Voices
enabled in EON, plus echo feedback).

Changes to the ESA register are are realized (almost) immediately: After
changing ESA, the hardware may keep access 1-2 samples at [OldBase+index], and
does then continue at [NewBase+index] (with index being increased as usually,
or reset to 0 when Echo Size has ellapsed).

**7Dh - EDL - Echo delay (ring buffer size) (R/W)**

Specifies the size of the Echo RAM buffer (and thereby the echo delay).
Additionally to that RAM buffer, there is a 8x4-byte buffer in the FIR unit.

```text
  0-3  Echo Buffer Size (0=4 bytes, or 1..15=Size in 2K-byte steps) (max=30K)
  4-7  Not used (read/write-able)                (ie. 16 ms steps) (max=240ms)
```

Caution: It may take up to about 240ms until the hardware realizes changes to
the EDL register; ie. when re-allocating the buffer size, the hardware may keep
writing up to 30Kbytes according to the old size. See Echo Buffer Notes below.

**Echo Buffer Notes**

```text
        Note that the ESA register is accessed 32 cycles before the value is
        used for a write; at a sample level, this causes writes to appear to be
        delayed by at least a full sample before taking effect.
```

```text
        The EDL register value is only used under certain conditions:
         * Write the echo buffer at sample 'idx' (cycles 29 and 30)
         * If idx==0, set idx_max = EDL<<9       (cycle 30-ish)
         * Increment idx. If idx>=idx_max, idx=0 (cycle 30-ish)
        This means that it can take up to .25s for a newly written value to
        actually take effect, if the old value was 0x0f and the new value is
        written just after the cycle 30 in which buffer index 0 was written.
```

**xFh - FIRx - Echo FIR filter coefficient 0..7 (R/W)**

```text
  0-7  Echo Coefficient for 8-tap FIR filter (-80h..+7Fh)
```

Value -128 should not be used for any of the FIRx registers (to avoid multiply
overflows). To avoid addition overflows: The sum of POSITIVE values in first
seven registers (FIR0..FIR6) should not exceed +7Fh, and the sum of NEGATIVE
values should not exceed -7Fh.

The sum of all eight registers (FIR0..FIR7) should be usually around +80h (for
leaving the overall output volume unchanged by the FIR unit; instead, echo
volumes are usually adjusted via EFB/EVOLx registers).

The FIR formula is:

```text
  addr = (ESA*100h+ram_index*4) AND FFFFh           ;-Echo RAM read/write addr
  buf[(i-0) AND 7] = EchoRAM[addr] SAR 1            ;-input 15bit from Echo RAM
  sum =       buf[(i-7) AND 7]*FIR0 SAR 6  ;oldest  ;\
  sum = sum + buf[(i-6) AND 7]*FIR1 SAR 6           ; calculate 16bit sum of
  sum = sum + buf[(i-5) AND 7]*FIR2 SAR 6           ; oldest 7 values, these
  sum = sum + buf[(i-4) AND 7]*FIR3 SAR 6           ; additions are done
  sum = sum + buf[(i-3) AND 7]*FIR4 SAR 6           ; without overflow
  sum = sum + buf[(i-2) AND 7]*FIR5 SAR 6           ; handling
  sum = sum + buf[(i-1) AND 7]*FIR6 SAR 6           ;/
  sum = sum + buf[(i-0) AND 7]*FIR7 SAR 6  ;newest  ;-with overflow handling
  if overflow occurred in LAST addition: saturate to min/max=-8000h/+7FFFh
  audio_output=NormalVoices+((sum*EVOLx) SAR 7)     ;-output to speakers
  echo_input=EchoVoices+((sum*EFB) SAR 7)           ;-feedback to echo RAM
  echo_input=echo_input AND FFFEh                   ;-isolate 15bit/bit0=0
  if echo write enabled: EchoRAM[addr]=echo_input   ;-write (if enabled in FLG)
  i = i + 1                                         ;-FIR index for next sample
  ram_index=ram_index+1                             ;-RAM index for next sample
  decrease remain, if remain=0, reload remain from EDL and set ram_index=0
```

Note that the left and right stereo channels are filtered separately (no
crosstalk), but with identical coefficients.

**Filter Examples**

```text
  FIR0 FIR1 FIR2 FIR3 FIR4 FIR5 FIR6 FIR7  EFB
  FF   08   17   24   24   17   08   FF    40  Echo with Low-pass (bugged)
  7F   00   00   00   00   00   00   00    7F  Echo (nearly endlessly repeated)
  7F   00   00   00   00   00   00   00    40  Echo (repeat w. decreasing vol)
  7F   00   00   00   00   00   00   00    00  Echo (one-shot)
```

Note: The "bugged" example (with the sum of FIR0..FIR6 exceeding +7Fh) is from
official (!) specs (and thus possibly actually used by some games(?)).

**Echo Overflows**

Setting FIRx, EFB, or EVOLx to -128 does probably cause multiply overflows?

<a id="snesapulowleveltimings"></a>

## SNES APU Low Level Timings

**Register and RAM Access Timing Chart**

On every CPU cycle, the hardware can access 3 bytes from RAM (2 DSP accesses,
and 1 CPU access), and 4 bytes from DSP Register Array (3 DSP accesses, and 1
CPU access), plus some special DSP Registers (ENDX/FLG). Audio is generated at
32000Hz Rate (every 32 CPU cycles):

```text
  Time <--- RAM Access --->   <---- Register Array ---->   <--Extra-->
  T0   [V0BRR.2nd.dta1]       --      ,V0VOLL    ,V2SRCN
  T1   [V1SRCN/DIR.lsb/msb]   V0VOLR  ,V1PITCHL  ,V1ADSR1  ENDX.0, Timer0,1,2
  T2   [V1BRR.1st.hdr/dta0]   V0ENVX  ,V1PITCHH  ,V1ADSR2  V1:FLG.7
  T3   [V1BRR.2nd.dta1]       V0OUTX  ,V1VOLL    ,V3SRCN
  T4   [V2SRCN/DIR.lsb/msb]   V1VOLR  ,V2PITCHL  ,V2ADSR1  ENDX.1
  T5   [V2BRR.1st.hdr/dta0]   V1ENVX  ,V2PITCHH  ,V2ADSR2  V2:FLG.7
  T6   [V2BRR.2nd.dta1]       V1OUTX  ,V2VOLL    ,V4SRCN
  T7   [V3SRCN/DIR.lsb/msb]   V2VOLR  ,V3PITCHL  ,V3ADSR1  ENDX.2
  T8   [V3BRR.1st.hdr/dta0]   V2ENVX  ,V3PITCHH  ,V3ADSR2  V3:FLG.7
  T9   [V3BRR.2nd.dta1]       V2OUTX  ,V3VOLL    ,V5SRCN
  T10  [V4SRCN/DIR.lsb/msb]   V3VOLR  ,V4PITCHL  ,V4ADSR1  ENDX.3
  T11  [V4BRR.1st.hdr/dta0]   V3ENVX  ,V4PITCHH  ,V4ADSR2  V4:FLG.7
  T12  [V4BRR.2nd.dta1]       V3OUTX  ,V4VOLL    ,V6SRCN
  T13  [V5SRCN/DIR.lsb/msb]   V4VOLR  ,V5PITCHL  ,V5ADSR1  ENDX.4
  T14  [V5BRR.1st.hdr/dta0]   V4ENVX  ,V5PITCHH  ,V5ADSR2  V5:FLG.7
  T15  [V5BRR.2nd.dta1]       V4OUTX  ,V5VOLL    ,V7SRCN
  T16  [V6SRCN/DIR.lsb/msb]   V5VOLR  ,V6PITCHL  ,V6ADSR1  ENDX.5
  T17  [V6BRR.1st.hdr/dta0]   V5ENVX  ,V6PITCHH  ,V6ADSR2  V6:FLG.7, Timer2
  T18  [V6BRR.2nd.dta1]       V5OUTX  ,V6VOLL    ,V0SRCN
  T19  [V7SRCN/DIR.lsb/msb]   V6VOLR  ,V7PITCHL  ,V7ADSR1  ENDX.6
  T20  [V7BRR.1st.hdr/dta0]   V6ENVX  ,V7PITCHH  ,V7ADSR2  V7:FLG.7
  T21  [V7BRR.2nd.dta1]       V6OUTX  ,V7VOLL    ,V1SRCN
  T22  [V0SRCN/DIR.lsb/msb]   V7VOLR  ,V0PITCHL  ,V0ADSR1  ENDX.7
  T23  [RdEchoLeft.lsb/msb]   V7ENVX  ,V0PITCHH  ,FIR0
  T24  [RdEchoRight.lsb/msb]  V7OUTX  ,FIR1      ,FIR2
  T25  --- ;SRAM./OE=no       FIR3    ,FIR4      ,FIR5
  T26  [V0BRR.1st.hdr/dta0]   FIR6    ,FIR7      ,---
  T27  --- ;SRAM./OE=yes?!    MVOLL   ,EVOLL     ,EFB
  T28  --- ;SRAM./OE=yes?!    MVOLR   ,EVOLR     ,PMON
  T29  --- ;SRAM./OE=no       NON     ,EON       ,DIR      FLG.5
  T30  [WrEchoLeft.lsb/msb]   EDL     ,ESA       ,KON?     FLG.5
  T31  [WrEchoRight.lsb/msb]  KOFF    ,FLG.LSB   ,V0ADSR2  FLG.0-4,V0:FLG.7,KON
```

Note: In Gain-Mode, above reads VxGAIN instead of VxADSR2.

KON and KOFF are processed only each 64 clk cycles (each 2nd pass).

The SPC and DSP chips are started via same /RESET and clocked via same 2.048MHz
signal, causing the SPC Timers to be incremented in sync with DSP timings: at
T1 and T17 (unless one pauses the SPC Timers via TEST register).

Additionally (non-RAM-array):

```text
  ENDX         8bits
  KON-changed  1bit
  FLG.MSBs     3bit (or maybe/rather it's full 8bit, including LSBs?)
```

**Internal Signals**

LRCK is HIGH during T0..T15, and LOW during T16..T31.

**APU Signals**

```text
  Time  15 16 17 18 19 20 21 22 23 24 25 26 27 28 29 30 31  0  1  (Tnn)
  LRCK  ---________________________________________________------ (DSP.Pin43)
  MX1   _-__-__-__-__-__-__-__-__-__-__-__-__-__-__-__-__-__-__-- (DSP.Pin3)
  MX2   __-__-__-__-__-__-__-__-__-__-__-__-__-__-__-__-__-__-__- (DSP.Pin4)
  MX3   __'__'__'__'__'__'__'__'__'__'__'__'__'__'__'__'__'__'__' (DSP.Pin5)
  /WE   --C--C--C--C--C--C--C--C--C--C--C--C--C--C--CEECEEC--C--C (SRAM.Pin27)
  /OE   DDCDDCDDCDDCDDCDDCDDCDDCEECEECddCDDCddCddCddC--C--CDDCDDC (SRAM.Pin22)
  /CE0  DDCDDCDDCDDCDDCDDCDDCDDC--C--C--CDDCddCddC--C--C--CDDCDDC (SRAM.Pin20a)
  /CE1  --C--C--C--C--C--C--C--CEECEEC--C--C--C--C--CEECEEC--C--C (SRAM.Pin20b)
```

Whereas:

```text
  "-"  High
  "_"  Low
  "'"  Very short High (near falling edges of MX2)
  "C"  CPU access           ;-(occurs when MX2=High)
  "EE" Echo access          ;\
  "DD" DSP access (DIR/BRR) ; (occurs when MX2=Low)
  "dd" DSP access (dummy)   ;/
```

For /CE0 and /CE1: Assume DIR/BRR in lower 32K, Echo in upper 32K RAM.

The "DD" and "dd" signals seem to be always going Low (even when no new BRR/DIR
data is needed).

<a id="snesmathsmultiplydivide"></a>

## SNES Maths Multiply/Divide

> **Note (RustySNES ref):** Both units deliver their result *progressively*, so an emulator that must pass cycle-accurate tests should model the delay, not just the final value. The multiply (write to `WRMPYB` $4203) completes 8 CPU cycles after the write, and reading `RDMPYL/H` earlier returns a valid *intermediate* product; the result is valid one cycle earlier per leading zero bit in `WRMPYA`. The divide (write to `WRDIVB` $4206) takes up to 16 CPU cycles, with the quotient (`RDDIV`) and remainder (`RDMPY`) updated as it runs. See [SNESdev Wiki: Multiplication](https://snes.nesdev.org/wiki/Multiplication) and [Division](https://snes.nesdev.org/wiki/Division).

**4202h - WRMPYA - Set unsigned 8bit Multiplicand (W)**

**4203h - WRMPYB - Set unsigned 8bit Multiplier and Start Multiplication (W)**

Set WRMPYA (or leave it unchanged, if it already contains the desired value),
then set WRMPYB, wait 8 clk cycles, then read the 16bit result from Port
4216h-4217h. For some reason, the hardware does additionally set RDDIVL=WRMPYB,
and RDDIVH=00h.

**4204h - WRDIVL - Set unsigned 16bit Dividend (lower 8bit) (W)**

**4205h - WRDIVH - Set unsigned 16bit Dividend (upper 8bit) (W)**

**4206h - WRDIVB - Set unsigned 8bit Divisor and Start Division (W)**

Set WRDIVL/WRDIVH (or leave it unchanged, if it already contains the desired
value), then set WRDIVB, wait 16 clk cycles, then read the 16bit result and/or
16bit remainder from Port 4214h-4217h.

Division by zero returns Result=FFFFh, Remainder=Dividend. Note: Almost all
commercial SNES games are zero-filling I/O ports upon initialization, thereby
causing division by zero (so, debuggers should ignore division errors).

**4214h - RDDIVL - Unsigned Division Result (Quotient) (lower 8bit) (R)**

**4215h - RDDIVH - Unsigned Division Result (Quotient) (upper 8bit) (R)**

See Ports 4204h-4206h (divide). Destroyed by 4203h (multiply).

**4216h - RDMPYL - Unsigned Division Remainder / Multiply Product (lo.8bit) (R)**

**4217h - RDMPYH - Unsigned Division Remainder / Multiply Product (up.8bit) (R)**

See Ports 4204h-4206h (divide), and 4202h-4203h (multiply).

**Timing Notes**

The 42xxh Ports are clocked by the CPU Clock, meaning that one needs the same
amount of "wait" opcodes no matter if the CPU Clock is 3.5MHz or 2.6MHz. When
reading the result, the "MOV r,[421xh]" opcode does include 3 cycles (spent on
reading the 3-byte opcode), meaning that one needs to insert only 5 cycles for
MUL and only 13 for DIV.

Some special cases: If the the upper "N" bits of 4202h are all zero, then it
seems that one may wait "N" cycles less. If memory REFRESH occurs (once and
when), then the result seems to be valid within even less wait opcodes.

The maths operations are started only on WRMPYB/WRDIVB writes (not on
WRMPYA/WRDIVL/WRDIVH writes; unlike the PPU maths which start on any M7A/M7B
write).

=== PPU Ports ===

Below Ports 21xxh are PPU registers. The registers are also used for
rotation/scaling effects in BG Mode 7. In BG Mode 0-6 they can be used freely
for multiplications. In Mode 7 they are usable ONLY during V-Blank and
Forced-Blank (during the Mode 7 Drawing &amp; H-Blank periods, they return
garbage in MPYL/MPYM/MPYH, and of course writing math-parameters to M7A/M7B
would also mess-up the display).

**211Bh - M7A - Rotation/Scaling Parameter A (and Maths 16bit operand) (W)**

```text
  1st Write: Lower 8bit of signed 16bit Multiplicand  ;\1st/2nd write mechanism
  2nd Write: Upper 8bit of signed 16bit Multiplicand  ;/uses "M7_old" (Mode7)
```

**211Ch - M7B - Rotation/Scaling Parameter B (and Maths 8bit operand) (W)**

```text
  Any Write: Signed 8bit Multiplier                   ;-also affects "M7_old"
```

After writing to 211Bh or 211Ch, the result can be read immediately from
2134h-2136h (the 21xxh Ports are rapidly clocked by the PPU, there's no delay
needed when reading via "MOV A,[211Ch]" or via "MOV A,[1Ch]" (with D=2100h),
both works even when the CPU runs at 3.5MHz).

**2134h - MPYL - Signed Multiply Result (lower 8bit) (R)**

**2135h - MPYM - Signed Multiply Result (middle 8bit) (R)**

**2136h - MPYH - Signed Multiply Result (upper 8bit) (R)**

See Ports 211Bh-211Ch.

**Notes**

Some cartridges contain co-processors with further math functions:

[SNES Cartridges](60-cartridge-header-and-mapping.md#snes-cartridges)
