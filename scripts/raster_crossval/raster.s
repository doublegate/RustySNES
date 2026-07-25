; Mid-line-raster cross-check ROM (T-CA-10 Phase 4c, docs/adr/0014).
;
; Purpose: a dot-precise MID-LINE register write, repeated every scanline, so the raster boundary
; (where the write takes effect) is captured in the rendered framebuffer and can be cross-checked
; between RustySNES's per-dot compositor and a cycle-accurate reference (MesenCE).
;
; Mechanism: BG1 is a solid colour A over a backdrop B. An HDMA channel rewrites TM ($212C) = BG1-on
; at the START of every scanline; an H-IRQ (HTIME = RASTER_DOT, no V match, so it fires every line)
; disables TM mid-line. Each line therefore shows BG1 (A) up to the DRAW cursor at the write dot,
; then the backdrop (B). The colour histogram is {A : boundary * lines, B : (256-boundary) * lines +
; overscan}; A is never the backdrop, so boundary = A_count / active_lines is overscan-robust and the
; two emulators can be compared on it.
;
; A composite-side register (TM) is written, so the boundary lands at the DRAW cursor. Set
; RASTER_DOT via ca65 -D; the host derives the expected column and compares the two renders' A counts.

.p816
.smart

.ifndef RASTER_DOT
RASTER_DOT = 128        ; overridable with `ca65 -D RASTER_DOT=NNN`
.endif

.segment "CODE"

.proc reset
    sei
    clc
    xce                 ; native mode
    rep #$30            ; A/X/Y 16-bit
    .a16
    .i16
    ldx #$01ff
    txs                 ; stack
    lda #$0000
    tcd                 ; DP = 0
    sep #$20
    .a8
    phk
    plb                 ; DBR = 0

    lda #$8f
    sta $2100           ; force blank during setup
    stz $4200           ; NMI/IRQ off during setup

    ; --- VRAM: tile 0 (2bpp) = solid colour 1 (plane 0 all ones) at char base 0 ---
    lda #$80
    sta $2115           ; VMAIN: increment on high byte
    rep #$20
    .a16
    lda #$0000
    sta $2116           ; VRAM address 0
    sep #$20
    .a8
    ldx #$0000
@tile:
    lda #$ff
    sta $2118           ; plane 0 low byte = all set (colour 1)
    lda #$00
    sta $2119           ; plane 1 high byte = 0
    inx
    cpx #$0008          ; 8 rows = one 2bpp tile
    bne @tile

    ; --- A second solid tile (colour 2) at char base $1000, for the FETCH-cursor variant. The
    ;     FETCH raster switches BG1's char base ($210B) mid-line, so tile 0 reads colour A up to the
    ;     fetch cursor, colour C after. Harmless for the DRAW variant (that never selects char $1000).
    rep #$20
    .a16
    lda #$1000
    sta $2116           ; VRAM address = $1000 (char base 1)
    sep #$20
    .a8
    ldx #$0000
@tile2:
    lda #$00
    sta $2118           ; plane 0 = 0
    lda #$ff
    sta $2119           ; plane 1 all set = pixel value 2 -> CGRAM[2] (a distinct colour C)
    inx
    cpx #$0008
    bne @tile2

    ; --- Tilemap at word $0400: every entry = tile 0, palette group 0 (don't rely on VRAM init) ---
    rep #$20
    .a16
    lda #$0400
    sta $2116           ; VRAM address = tilemap base
    sep #$20
    .a8
    ldx #$0000
@map:
    stz $2118
    stz $2119           ; entry = $0000 (tile 0), commit + increment
    inx
    cpx #$0400          ; 32x32 = 1024 entries
    bne @map

    ; --- CGRAM: [0] = backdrop B (blue), [1] = A (red) ---
    stz $2121
    lda #$00
    sta $2122
    lda #$7c
    sta $2122           ; CGRAM[0] = $7C00 (blue) — the backdrop / colour B
    lda #$1f
    sta $2122
    lda #$00
    sta $2122           ; CGRAM[1] = $001F (red) — BG1 colour A
    lda #$e0
    sta $2122
    lda #$03
    sta $2122           ; CGRAM[2] = $03E0 (green) — colour C (FETCH variant's post-boundary colour)

    ; --- BG1: mode 0, char base 0, tilemap base $0400 (default-zero entries = tile 0) ---
    stz $2105           ; BGMODE 0
    stz $210b           ; BG1/BG2 char base nibble 0
    lda #$04
    sta $2107           ; BG1SC: tilemap base word $0400, size 32x32
    lda #$01
    sta $212c           ; TM: BG1 on the main screen (HDMA restores this each line)

    ; --- HDMA channel 0: rewrite TM = $01 at every scanline start ---
    lda #$00
    sta $4300           ; ch0: transfer mode 0 (1 byte), A->B, direct
.ifdef FETCH_RASTER
    lda #$0b            ; B-bus target $210B (BG12NBA char base) — a BG-DATA (fetch-cursor) register
.else
    lda #$2c            ; B-bus target $212C (TM main enable) — a composite (draw-cursor) register
.endif
    sta $4301
    rep #$20
    .a16
    lda #.loword(hdma_tm_table)
    sta $4302           ; table address (bank in $4304)
    sep #$20
    .a8
    lda #^hdma_tm_table
    sta $4304           ; table bank
.ifndef NO_HDMA
    lda #$01
    sta $420c           ; HDMAEN: channel 0
.endif

    ; --- H-IRQ every scanline at RASTER_DOT ---
    rep #$20
    .a16
    lda #RASTER_DOT
    sta $4207           ; HTIME (9-bit; high bit in $4208)
    sep #$20
    .a8

    lda #$0f
    sta $2100           ; display on, full brightness
.ifndef NO_IRQ
    lda #$10
    sta $4200           ; IRQ on H match only (no V match => every scanline)
    cli
.endif

@spin:
    wai
    bra @spin
.endproc

; The IRQ handler: acknowledge, disable BG1 on the main screen mid-line (HDMA turns it back on at the
; next line start). Kept minimal so the write commits at a fixed, cycle-deterministic dot offset from
; the H-IRQ — the same offset on any accurate core.
.proc irq
    sep #$20
    .a8
    pha
    lda $4211           ; TIMEUP: acknowledge the H-IRQ
.ifdef FETCH_RASTER
    lda #$01
    sta $210b           ; BG1 char base -> $1000 (colour C) for the rest of this line (fetch cursor)
.else
    stz $212c           ; TM = 0: drop BG1 from the main screen for the rest of this line (draw cursor)
.endif
    pla
    rti
.endproc

.proc nmi
    rti
.endproc

; HDMA table. In "repeat" mode ($80 | count) the DMA transfers a NEW data unit every scanline
; (advancing the source pointer), so a `count`-line entry needs `count` data bytes — here all $01 so
; TM is rewritten to BG1-on at the start of every line, undoing the previous line's mid-line disable.
; 224 visible lines = 127 + 97; a trailing $00 ends the frame's transfer.
hdma_tm_table:
.ifdef FETCH_RASTER
RESET_VAL = $00         ; char base 0 (colour A) restored at each line start
.else
RESET_VAL = $01         ; TM = BG1-on restored at each line start
.endif
    .byte $ff           ; repeat, 127 lines
    .res 127, RESET_VAL  ;   restore each line
    .byte $e1           ; repeat, 97 lines
    .res 97, RESET_VAL   ;   restore each line
    .byte $00           ; end

.segment "HEADER"
; SNES internal header at $00:FFC0 (LoROM). Emulators use it to pick the LoROM memory map.
    .byte "RASTER XVAL 4C       "  ; $FFC0 title, 21 bytes
    .byte $20                       ; $FFD5 map mode: LoROM, slow
    .byte $00                       ; $FFD6 cartridge type: ROM only
    .byte $05                       ; $FFD7 ROM size: 1<<5 = 32 KB
    .byte $00                       ; $FFD8 RAM size: none
    .byte $01                       ; $FFD9 country: US/NTSC
    .byte $33                       ; $FFDA developer id
    .byte $00                       ; $FFDB version
    .word $0000                     ; $FFDC checksum complement (emulators here do not verify)
    .word $ffff                     ; $FFDE checksum

.segment "VECTORS"
; Native-mode vectors ($FFE0-$FFEF), then emulation-mode ($FFF0-$FFFF).
    .word 0, 0          ; $FFE0 reserved
    .word 0             ; $FFE4 COP
    .word 0             ; $FFE6 BRK
    .word 0             ; $FFE8 ABORT
    .addr nmi           ; $FFEA NMI
    .word 0             ; $FFEC reserved
    .addr irq           ; $FFEE IRQ
    .word 0, 0          ; $FFF0 reserved
    .word 0             ; $FFF4 COP (emu)
    .word 0             ; $FFF6 reserved
    .word 0             ; $FFF8 ABORT (emu)
    .addr nmi           ; $FFFA NMI (emu)
    .addr reset         ; $FFFC RESET
    .addr irq           ; $FFFE IRQ/BRK (emu)
