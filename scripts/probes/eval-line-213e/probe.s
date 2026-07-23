; $213E over-flag eval-line probe ROM (self-sampling).
;
; Sets up 40 sprites (8x8) all at Y=100, X = 0,4,...,156 (on-screen); the other 88 parked
; off-screen (Y=$F0). An H-IRQ fires every scanline at HTIME=256 (just after that scanline's sprite
; evaluation, which runs over dots 0..255, but with a wide margin before the line ends at ~340 so
; the handler's OPVCT latch stays within the current line); the handler latches the V-counter, reads
; STAT77 ($213E)
; and stores it to WRAM $7E:1000 + V. So after one full frame, WRAM[$1000+s] = the $213E value
; sampled late on scanline s. The host reads that array and finds the first scanline whose bit 6
; (range over) / bit 7 (time over) is set — the scanline at which the evaluation for a display line
; becomes observable. Both MesenCE (Lua reads snesWorkRam) and RustySNES (peek_wram) read the same
; array, so the comparison is apples-to-apples.

.p816
.smart

INIDISP  = $2100
OBSEL    = $2101
OAMADDL  = $2102
OAMADDH  = $2103
OAMDATA  = $2104
BGMODE   = $2105
SLHV     = $2137
STAT77   = $213E
STAT78   = $213F
OPVCT    = $213D
TM       = $212C
NMITIMEN = $4200
HTIMEL   = $4207
HTIMEH   = $4208
VTIMEL   = $4209
VTIMEH   = $420A
TIMEUP   = $4211

.segment "CODE"

reset:
        sei
        clc
        xce                     ; native mode
        rep     #$38            ; 16-bit A/X/Y, decimal off
        .a16
        .i16
        ldx     #$1fff
        txs
        lda     #$0000
        tcd                     ; DP = 0
        phk
        plb                     ; DBR = program bank $00 (so `sta $1000,x` hits WRAM mirror)

        sep     #$20            ; 8-bit A
        .a8
        lda     #$80
        sta     INIDISP         ; force blank while we set up
        stz     BGMODE          ; mode 0
        stz     OBSEL           ; sprite tile base 0, 8x8 small size

        ; ---- clear OAM low table: 128 sprites off-screen (Y=$F0) ----
        stz     OAMADDL
        stz     OAMADDH
        ldx     #$0000
clear_lo:
        stz     OAMDATA         ; X
        lda     #$f0
        sta     OAMDATA         ; Y off-screen
        stz     OAMDATA         ; tile
        stz     OAMDATA         ; attr
        inx
        cpx     #$0080
        bne     clear_lo

        ; ---- high OAM table: all zero ----
        lda     #$00
        sta     OAMADDL
        lda     #$01
        sta     OAMADDH
        ldx     #$0000
clear_hi:
        stz     OAMDATA
        inx
        cpx     #$0020
        bne     clear_hi

        ; ---- 40 sprites at Y=100, X = i*4 ----
        stz     OAMADDL
        stz     OAMADDH
        ldx     #$0000
        stz     $00             ; $00 = running X
set_spr:
        lda     $00
        sta     OAMDATA         ; X
        lda     #100
        sta     OAMDATA         ; Y
        stz     OAMDATA         ; tile
        stz     OAMDATA         ; attr
        lda     $00
        clc
        adc     #4
        sta     $00
        inx
        cpx     #40
        bne     set_spr

        ; enable sprites + display on
        lda     #$10
        sta     TM
        lda     #$0f
        sta     INIDISP

        ; ---- program H-IRQ at HTIME = 256 ($100), every scanline ----
        ; Right after sprite evaluation (dots 0..255), with a wide margin before the line ends
        ; (~340) so the handler's OPVCT latch stays within the current line: at HTIME=300 the
        ; IRQ-service latency pushed the latch to ~H=340, the V-counter increment, latching V+1.
        lda     #$00
        sta     HTIMEL
        lda     #$01
        sta     HTIMEH
        stz     VTIMEL          ; VTIME unused for H-only IRQ
        stz     VTIMEH
        cli                     ; allow IRQ
        lda     #$90            ; NMI on (bit7) + H-IRQ (bit4)
        sta     NMITIMEN

forever:
        wai
        bra     forever

; H-IRQ handler: sample STAT77 into WRAM[$1000 + V].
irq:
        rep     #$30
        .a16
        .i16
        pha
        phx
        sep     #$20
        .a8
        lda     SLHV            ; latch H/V counters
        lda     STAT78          ; reset the OPHCT/OPVCT 2nd-read flipflop so OPVCT gives the low byte
        lda     OPVCT           ; V low byte (V < 256 here, high bit ignored)
        rep     #$20
        .a16
        and     #$00ff
        clc
        adc     #$1000
        tax                     ; X = $1000 + V
        sep     #$20
        .a8
        lda     STAT77
        sta     $0000,x         ; DBR=0 -> WRAM mirror $00:1000+V
        lda     TIMEUP          ; acknowledge IRQ (clears the flag)
        rep     #$30
        .a16
        .i16
        plx
        pla
        rti

nmi:
        rti

.segment "HEADER"
        .byte   "213E OVERFLOW PROBE  "   ; 21-char title
        .byte   $20             ; map mode: LoROM, slow
        .byte   $00             ; cart type: ROM only
        .byte   $05             ; ROM size: 32 KiB
        .byte   $00             ; RAM size: none
        .byte   $01             ; country: US/NTSC
        .byte   $00             ; dev id
        .byte   $00             ; rom version
        .word   $0000           ; checksum complement
        .word   $0000           ; checksum

.segment "VECTORS"
        .word   $0000           ; $FFE0
        .word   $0000           ; $FFE2
        .word   nmi             ; $FFE4 COP
        .word   $0000           ; $FFE6 BRK
        .word   $0000           ; $FFE8 ABORT
        .word   nmi             ; $FFEA NMI
        .word   $0000           ; $FFEC
        .word   irq             ; $FFEE IRQ
        .word   $0000           ; $FFF0
        .word   $0000           ; $FFF2
        .word   nmi             ; $FFF4 COP (emu)
        .word   $0000           ; $FFF6
        .word   $0000           ; $FFF8 ABORT (emu)
        .word   nmi             ; $FFFA NMI (emu)
        .word   reset           ; $FFFC RESET
        .word   irq             ; $FFFE IRQ/BRK (emu)
