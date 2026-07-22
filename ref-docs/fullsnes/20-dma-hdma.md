# Fullsnes — DMA & HDMA Transfers

[Index](00-index.md) · [« Memory Map & I/O Map](10-memory-and-io-map.md) · [Picture Processing Unit »](30-ppu.md)

**Sections in this file:**

- [SNES DMA Transfers](#snes-dma-transfers)
- [SNES DMA and HDMA Start/Enable Registers](#snes-dma-and-hdma-startenable-registers)
- [SNES DMA and HDMA Channel 0..7 Registers](#snes-dma-and-hdma-channel-07-registers)
- [SNES DMA and HDMA Notes](#snes-dma-and-hdma-notes)

---

<a id="snesdmatransfers"></a>

## SNES DMA Transfers

The SNES includes eight DMA channels, which can be used for H-DMA or GP-DMA.

[SNES DMA and HDMA Start/Enable Registers](#snes-dma-and-hdma-startenable-registers)

[SNES DMA and HDMA Channel 0..7 Registers](#snes-dma-and-hdma-channel-07-registers)

[SNES DMA and HDMA Notes](#snes-dma-and-hdma-notes)

**H-DMA (H-Blank DMA)**

H-DMA transfers are automatically invoked on H-Blank, each H-DMA is limited to
a single unit (max 4 bytes) per scanline. This is commonly used to manipulate
PPU I/O ports (eg. to change scroll offsets). Related registers can found here:

[SNES I/O Map](10-memory-and-io-map.md#snes-io-map)

[SNES Picture Processing Unit (PPU)](30-ppu.md#snes-picture-processing-unit-ppu)

**GP-DMA (General Purpose DMA)**

GP-DMA can manually invoked by software, allowing to transfer larger amounts of
data (max 10000h bytes). This is commonly used to transfer WRAM or ROM (on
A-Bus side) to/from WRAM, OAM, VRAM, CGRAM (on B-Bus side). Related registers
are:

[SNES Memory Work RAM Access](10-memory-and-io-map.md#snes-memory-work-ram-access)

[SNES Memory OAM Access (Sprite Attributes)](10-memory-and-io-map.md#snes-memory-oam-access-sprite-attributes)

[SNES Memory VRAM Access (Tile and BG Map)](10-memory-and-io-map.md#snes-memory-vram-access-tile-and-bg-map)

[SNES Memory CGRAM Access (Palette Memory)](10-memory-and-io-map.md#snes-memory-cgram-access-palette-memory)

<a id="snesdmaandhdmastartenableregisters"></a>

## SNES DMA and HDMA Start/Enable Registers

```text
  DMA and HDMA Transfer order is Channel 0 first... Channel 7 last
  HDMA has higher prio than DMA
  HDMA is running even during Forced Blank.
```

**420Bh - MDMAEN - Select General Purpose DMA Channel(s) and Start Transfer (W)**

```text
  7-0   General Purpose DMA Channel 7-0 Enable (0=Disable, 1=Enable)
```

When writing a non-zero value to this register, general purpose DMA will be
started immediately (after a few clk cycles). The CPU is paused during the
transfer. The transfer can be interrupted by H-DMA transfers. If more than 1
bit is set in MDMAEN, then the separate transfers will be executed in order
channel 0=first through 7=last. The MDMAEN bits are cleared automatically at
transfer completion.

Do not use channels for GP-DMA which are activated as H-DMA in HDMAEN.

**420Ch - HDMAEN - Select H-Blank DMA (H-DMA) Channel(s) (W)**

```text
  7-0   H-DMA Channel 7-0 Enable (0=Disable, 1=Enable)
```

...

<a id="snesdmaandhdmachannel07registers"></a>

## SNES DMA and HDMA Channel 0..7 Registers

For below ports, x = Channel number (0-7)

**43x0h - DMAPx - DMA/HDMA Parameters (R/W)**

```text
  7     Transfer Direction (0=A:CPU to B:I/O, 1=B:I/O to A:CPU)
  6     Addressing Mode    (0=Direct Table, 1=Indirect Table)    (HDMA only)
  5     Not used (R/W) (unused and unchanged by all DMA and HDMA)
  4-3   A-BUS Address Step  (0=Increment, 2=Decrement, 1/3=Fixed) (DMA only)
  2-0   Transfer Unit Select (0-4=see below, 5-7=Reserved)
```

DMA Transfer Unit Selection:

```text
  Mode  Bytes              B-Bus 21xxh Address   ;Usage Examples...
  0  =  Transfer 1 byte    xx                    ;eg. for WRAM (port 2180h)
  1  =  Transfer 2 bytes   xx, xx+1              ;eg. for VRAM (port 2118h/19h)
  2  =  Transfer 2 bytes   xx, xx                ;eg. for OAM or CGRAM
  3  =  Transfer 4 bytes   xx, xx,   xx+1, xx+1  ;eg. for BGnxOFS, M7x
  4  =  Transfer 4 bytes   xx, xx+1, xx+2, xx+3  ;eg. for BGnSC, Window, APU..
  5  =  Transfer 4 bytes   xx, xx+1, xx,   xx+1  ;whatever purpose, VRAM maybe
  6  =  Transfer 2 bytes   xx, xx                ;same as mode 2
  7  =  Transfer 4 bytes   xx, xx,   xx+1, xx+1  ;same as mode 3
```

A HDMA transfers ONE unit per scanline (=max 4 bytes). General Purpose DMA has
a 16bit length counter, allowing to transfer up to 10000h bytes (ie. not 10000h
units).

**43x1h - BBADx - DMA/HDMA I/O-Bus Address (PPU-Bus aka B-Bus) (R/W)**

For both DMA and HDMA:

```text
  7-0   B-Bus Address (selects an I/O Port which is mapped to 2100h-21FFh)
```

For normal DMA this should be usually 04h=OAM, 18h=VRAM, 22h=CGRAM, or
80h=WRAM. For HDMA it should be usually some PPU register (eg. for changing
scroll offsets midframe).

**43x2h - A1TxL - HDMA Table Start Address (low) / DMA Current Addr (low) (R/W)**

**43x3h - A1TxH - HDMA Table Start Address (hi)  / DMA Current Addr (hi) (R/W)**

**43x4h - A1Bx - HDMA Table Start Address (bank) / DMA Current Addr (bank) (R/W)**

For normal DMA:

```text
  23-16  CPU-Bus Data Address Bank (constant, not incremented/decremented)
  15-0   CPU-Bus Data Address (incremented/decremented/fixed, as selected)
```

For HDMA:

```text
  23-16  CPU-Bus Table Address Bank (constant, bank number for 43x8h/43x9h)
  15-0   CPU-Bus Table Address      (constant, reload value for 43x8h/43x9h)
```

**43x5h - DASxL - Indirect HDMA Address (low) / DMA Byte-Counter (low) (R/W)**

**43x6h - DASxH - Indirect HDMA Address (hi)  / DMA Byte-Counter (hi)  (R/W)**

**43x7h - DASBx - Indirect HDMA Address (bank) (R/W)**

For normal DMA:

```text
  23-16  Not used
  15-0   Number of bytes to be transferred (1..FFFFh=1..FFFFh, or 0=10000h)
  (This is really a byte-counter; with a 4-byte "Transfer Unit", len=5 would
  transfer one whole Unit, plus the first byte of the second Unit.)
  (The 16bit value is decremented during transfer, and contains 0000h on end.)
```

For HDMA in direct mode:

```text
  23-0   Not used     (in this mode, the Data is read directly from the Table)
```

For HDMA in indirect mode:

```text
  23-16  Current CPU-Bus Data Address Bank   (this must be set by software)
  16-0   Current CPU-Bus Data Address (automatically loaded from the Table)
```

**43x8h - A2AxL - HDMA Table Current Address (low) (R/W)**

**43x9h - A2AxH - HDMA Table Current Address (high) (R/W)**

For normal DMA:

```text
  15-0  Not used
```

For HDMA:

```text
  -     Current Table Address Bank (taken from 43x4h)
  15-0  Current Table Address (reloaded from 43x2h/43x3h) (incrementing)
```

**43xAh - NTRLx - HDMA Line-Counter (from current Table entry) (R/W)**

For normal DMA:

```text
  7-0   Not used
```

For HDMA:

```text
  7     Repeat-flag                         ;\(loaded from Table, and then
  6-0   Number of lines to be transferred   ;/decremented per scanline)
```

**43xBh - UNUSEDx - Unused Byte (R/W)**

```text
  7-0   Not used (read/write-able)
```

Can be used as a fast RAM location (but NOT as a fixed DMA source address for
memfill). Storing any value in this register seems to have no effect on the
transfer (and the value is left intact, not modified by DMA nor direct nor
indirect HDMAs).

**43xCh..43xEh - Unused region (open bus)**

Unused. Reading returns garbage (open bus), writing seems to have no effect,
even when trying to "disturb" HDMAs.

**43xFh - MIRRx - Read/Write-able mirror of 43xBh (R/W)**

Mirror of 43xBh.

**HDMA Table Formats (in Direct and Indirect Mode)**

In Direct Mode, the table consists of entries in following format:

```text
  1 byte   Repeat-flag & line count
  N bytes  Data (where N=unit size, if repeat=1: multiplied by line count)
```

In Indirect Mode, the table consists of entries in following format:

```text
  1 byte   Repeat-flag & line count
  2 bytes  16bit pointer to N bytes of Data (where N = as for Direct HDMA)
```

In either mode: The "repeat-flag &amp; line count" bytes can be:

```text
  00h       Terminate this HDMA channel (until it restarts in next frame)
  01h..80h  Transfer 1 unit in 1 line, then pause for next "X-01h" lines
  81h..FFh  Transfer X-80h units in X-80h lines ("repeat mode")
```

The "count" and "pointer" values are always READ from the table. The "data"
values are READ or WRITTEN depending on the transfer direction. The transfer
step is always INCREMENTING for HDMA (for both the table itself, as well as for
any indirectly addressed data blocks).

<a id="snesdmaandhdmanotes"></a>

## SNES DMA and HDMA Notes

**Starting HDMA midframe**

Activating HDMA (via port 420Ch) should be usually done only during VBlank (so
that the hardware can reload the HDMA registers automatically at end of
Vblank).

Otherwise, when starting HDMA midframe (outside of Vblank), then one must
manually reload the HDMA registers. For example, during Vblank, init HDMA
registers as so:

```text
  420Ch=00h ;stop all HDMA channels
  43x0h=02h ;transfer two bytes to [bbus+0], and [bbus+0]
  43x1h=88h ;dummy bbus destination address (unused port 2188h)
  42x4h=abus.src.bank       ;with <abus.src> pointing to "02h,77h,55h,00h"
  42x8h=abus.src.offs.lo    ;ie. repeat/pause 2 scanlines (02h), transfer
  42x9h=abus.src.off.hi     ;one data unit (77h,55h), and after the pause,
  42xAh=01h ;remain count   ;finish the transfer (00h).
```

The HDMA starting point seems to depend on whether any HDMA channels were
already active in the current frame. The two scenarios (each with above example
values) are:

**Case 1 - (420Ch was still zero) - First HDMA in current frame**

Start one (or more) HDMA channel(s) somewhere midframe (eg. in line 128), and
watch the src/remain values in 43x8h..43xAh. This will behave as expected (src
increases, and remain decreases from 02h downto 00h).

**Case 2 - (420Ch was already nonzero) - Further HDMA in current frame**

Start another HDMA channel (some scanlines later, after the above transfer).
This will behave differently: It's decreasing remain count in 43xAh from 55h
downwards, ie. the HDMA does apparently start with "do_transfer=1" (for
whatever reason), causing it to transfer 02h,77h as data, and then fetch 55h as
repeat count for next scanlines.

Note: One game does start HDMAs midframe is Super Ghouls 'N Ghosts.

[XXX Case 2 may need more research, and isn't yet accurately emulated in
no$sns]

**External DMA**

The SNES Cartridge Slot doesn't have any special DRQ/DACK pins for DMA
handshaking purposes; DMA from cartridge memory is just implemented as normal
memory access (so the cartridge must respond to DMAs within normal memory
access time; which is fixed 8 master cycles per byte for DMA).

Unknown if the data decompression chips (S-DD1 and SPC7110) are implemented the
same way, or if they do use any "hidden" DMA handshaking mechanisms (they might
be too slow to supply bytes within 8 master cycles, and at least one of them
seems to decode DMA channel numbers; possibly by sensing writes to 420Bh?).
