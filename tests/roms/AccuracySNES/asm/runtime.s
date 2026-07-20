; AccuracySNES — cart runtime.
;
; Boot -> initialise -> run every test -> publish the results block -> draw the summary ->
; interactive list. The battery runs to completion **without any input**, so a host harness only
; has to boot the ROM and read WRAM; the menu exists for humans.
;
; Two deliberate choices, both to avoid depending on behaviour under test:
;
;   * Controller input is read MANUALLY through $4016, not via auto-joypad. The auto-read has a
;     documented start-window race ($4212 bit 0 reads clear before the read begins), and RustySNES
;     does not model bit 0 at all — the usual "wait for busy to set, then clear" idiom would
;     deadlock there. Manual reads have no such dependency.
;   * VBlank is detected by polling $4212 bit 7 rather than by NMI, so no interrupt can fire in
;     the middle of a test that is deliberately corrupting the stack or the E flag.

.p816
.import font_data
RUNTIME_IMPL = 1                ; suppress runtime.inc's imports of what we define here
.include "runtime.inc"

; ---------------------------------------------------------------------------------------------
; Per-bank signature blocks.
;
; Several addressing tests must tell "the effective address wrapped inside bank $00" from "it
; crossed into bank $01". Each bank carries its own signature byte at $xx:8005 so the difference
; is observable; inside a mirrored 32 KiB image it would not be.
; ---------------------------------------------------------------------------------------------
.segment "SIG0"
    .byte "SIG0", $00, $A0, $00, $00, $00, $00, $00, $00, $00, $00, $00, $00
.segment "BANK1"
    .byte "SIG1", $00, $A1, $00, $00, $00, $00, $00, $00, $00, $00, $00, $00
.segment "BANK2"
    .byte "SIG2", $00, $A2, $00, $00, $00, $00, $00, $00, $00, $00, $00, $00
.segment "BANK3"
    .byte "SIG3", $00, $A3, $00, $00, $00, $00, $00, $00, $00, $00, $00, $00

.segment "CODE"

; ---------------------------------------------------------------------------------------------
; Reset
; ---------------------------------------------------------------------------------------------
.export reset
.proc reset
    sei
    clc
    xce                         ; leave 6502 emulation -> native
    ; XCE *exchanges* C and E, so the carry now holds the E flag as it was at reset — the only
    ; moment that value is readable, and it is gone one instruction later. `sta f:` needs no DBR
    ; and no DP, both of which are still whatever the machine powered up with.
    ;
    ; SEP #$20 first, and not as tidiness. A machine that reached here from emulation has M set
    ; already, since emulation forces it and XCE does not clear it — but a core that wrongly booted
    ; NATIVE is exactly the failure G1.04 exists to catch, and such a core may hand us M clear. The
    ; `lda #$00` below would then be a three-byte instruction, the assembler having emitted two,
    ; and the CPU would decode `sta`'s first byte as an operand and run off into the weeds. SEP
    ; leaves the carry alone, so it costs the capture nothing and turns a crash into a report.
    sep #$20
    lda #$00
    rol a                       ; A = 1 if the CPU booted in emulation mode (G1.04)
    sta f:V_PO_EMU
    cld
    rep #$38                    ; A/X/Y 16-bit, decimal off
    .a16
    .i16
    ldx #$1FFF
    txs                         ; stack at the top of the low-WRAM mirror
    lda #$0000
    tcd                         ; DP = $0000
    sep #$20
    .a8
    phk
    plb                         ; DBR = $00, so absolute $21xx/$42xx reach MMIO

    lda #$8F
    sta INIDISP                 ; forced blank while we set up

    rep #$20
    .a16
    lda #irq_stub
    sta a:V_IRQ_VEC             ; default: the IRQ trampoline behaves exactly like the old stub
    sep #$20
    .a8

    jsr capture_power_on        ; MUST precede init_registers — see below

    jsr init_registers
    jsr clear_vram
    jsr load_palette
    jsr load_font
    jsr clear_tilemap

    jsr run_all_tests           ; runs with the screen still blanked

    jsr run_scenes              ; rendered scenes for the host framebuffer oracle (ADR 0013)

    jsr draw_screen

    sep #$20
    .a8
    stz BGMODE                  ; mode 0, 8x8 tiles
    lda #(MAP_BASE >> 8)
    sta BG1SC                   ; tilemap base, 32x32
    stz BG12NBA                 ; BG1 character data at word $0000
    stz BG1HOFS
    stz BG1HOFS                 ; scroll registers are written twice
    stz BG1VOFS
    stz BG1VOFS
    lda #$01
    sta TM                      ; BG1 on the main screen
    lda #$0F
    sta INIDISP                 ; release forced blank

    jmp main_loop
.endproc

; ---------------------------------------------------------------------------------------------
; Power-on capture. Runs ONCE, before init_registers touches anything.
;
; init_registers puts every PPU register $2101-$2133 and every CPU register $4200-$420D into a
; known state, because hardware does not. That is right for the battery — a test must not depend
; on what a previous test left behind — but it destroys exactly the state a power-on test wants to
; observe. So the handful of power-on facts we can read are read here, first, and stashed for a
; test to report later out of the capture block ($E040-, see runtime.inc).
;
; The multiply/divide latches are write-only, so their power-on contents are observed through the
; unit rather than read back: writing only $4203 runs the multiply against whatever $4202 already
; held, and writing only $4206 divides whatever $4204/05 already held. Nothing here writes $4202
; or $4204/05, which is the whole point.
.proc capture_power_on
    sep #$20
    .a8
    .i16
    ; $4210 and $4211 are read FIRST, and reading either one clears its flag — so this is the only
    ; chance to see what reset left there. Both are also read-once-per-frame registers the runtime
    ; touches later, which is the other reason nothing may come before them.
    lda RDNMI
    sta f:V_PO_RDNMI            ; bit 7 = NMI pending, bits 3-0 = 5A22 version
    lda TIMEUP
    sta f:V_PO_TIMEUP           ; bit 7 = IRQ pending

    lda #$02
    sta $4203                   ; multiply: $4202 (power-on) x 2
    nop
    nop
    nop
    nop
    nop
    nop
    nop
    nop
    rep #$20
    .a16
    lda $4216
    sta f:V_PO_MPY

    sep #$20
    .a8
    lda #$02
    sta $4206                   ; divide: $4204/05 (power-on) / 2
    .repeat 16
    nop
    .endrepeat
    rep #$20
    .a16
    lda $4214
    sta f:V_PO_DIV
    lda $4216
    sta f:V_PO_DIVREM
    sep #$20
    .a8
    rts
.endproc

; ---------------------------------------------------------------------------------------------
; Register initialisation. The canonical SNES power-on block: every PPU register $2101-$2133 and
; every CPU register $4200-$420D put into a known state, because hardware does not.
; ---------------------------------------------------------------------------------------------
.proc init_registers
    sep #$20
    .a8
    .i16
    stz $2101                   ; OBSEL
    stz $2102                   ; OAMADDL
    stz $2103                   ; OAMADDH
    stz $2105                   ; BGMODE
    stz $2106                   ; MOSAIC
    stz $2107                   ; BG1SC
    stz $2108                   ; BG2SC
    stz $2109                   ; BG3SC
    stz $210A                   ; BG4SC
    stz $210B                   ; BG12NBA
    stz $210C                   ; BG34NBA
    ldx #$0000
@scroll:                        ; $210D-$2114 are write-twice ports
    stz $210D,x
    stz $210D,x
    inx
    cpx #$0008
    bne @scroll
    lda #$80
    sta VMAIN
    stz $2116
    stz $2117
    stz $211A                   ; M7SEL
    stz $211B
    lda #$01
    sta $211B                   ; M7A = $0100 (identity)
    stz $211C
    stz $211C                   ; M7B = 0
    stz $211D
    stz $211D                   ; M7C = 0
    stz $211E
    sta $211E                   ; M7D = $0100
    stz $211F
    stz $211F                   ; M7X = 0
    stz $2120
    stz $2120                   ; M7Y = 0
    stz $2121                   ; CGADD
    stz $2123                   ; W12SEL
    stz $2124                   ; W34SEL
    stz $2125                   ; WOBJSEL
    stz $2126                   ; WH0
    stz $2127                   ; WH1
    stz $2128                   ; WH2
    stz $2129                   ; WH3
    stz $212A                   ; WBGLOG
    stz $212B                   ; WOBJLOG
    stz $212C                   ; TM
    stz $212D                   ; TS
    stz $212E                   ; TMW
    stz $212F                   ; TSW
    lda #$30
    sta $2130                   ; CGWSEL: no colour math
    stz $2131                   ; CGADSUB
    lda #$E0
    sta $2132                   ; COLDATA
    stz $2133                   ; SETINI

    stz NMITIMEN                ; no NMI, no IRQ, no auto-joypad (we read $4016 by hand)
    lda #$FF
    sta $4201                   ; WRIO
    stz $4202
    stz $4203
    stz $4204
    stz $4205
    stz $4206
    stz $4207
    stz $4208
    stz $4209
    stz $420A
    stz $420B                   ; MDMAEN
    stz $420C                   ; HDMAEN
    stz $420D                   ; MEMSEL: SlowROM
    rts
.endproc

; ---------------------------------------------------------------------------------------------
; Zero all 32 KiB words of VRAM. Required: the font is uploaded as bitplane 0 only, so bitplane 1
; must already be zero for the glyphs to come out as colour 1.
; ---------------------------------------------------------------------------------------------
.proc clear_vram
    sep #$20
    .a8
    lda #$80
    sta VMAIN                   ; step 1 word, increment after the high byte
    rep #$30
    .a16
    .i16
    ldx #$0000
    stx VMADDL
    lda #$0000
    ldx #$8000
@loop:
    sta VMDATAL                 ; a 16-bit store covers $2118 and $2119
    dex
    bne @loop
    rts
.endproc

; ---------------------------------------------------------------------------------------------
; Two-colour palette: index 0 black (also the backdrop), index 1 white.
; ---------------------------------------------------------------------------------------------
.proc load_palette
    sep #$20
    .a8
    stz CGADD
    stz CGDATA                  ; colour 0 = $0000, black
    stz CGDATA
    lda #$FF
    sta CGDATA                  ; colour 1 = $7FFF, white
    lda #$7F
    sta CGDATA
    rts
.endproc

; ---------------------------------------------------------------------------------------------
; Upload the 1bpp font into the LOW bytes of consecutive VRAM words.
;
; With VMAIN = $00 the address advances after each write to $2118, so writing only low bytes
; fills bitplane 0 and leaves bitplane 1 at zero — a legal 2bpp tile using colours 0 and 1.
; Tile n therefore holds the glyph for ASCII n, which is what makes `tile_index & $FF == ASCII`
; hold in the tilemap.
; ---------------------------------------------------------------------------------------------
.proc load_font
    sep #$20
    .a8
    stz VMAIN                   ; increment after the LOW byte
    rep #$30
    .a16
    .i16
    ldx #$0000
    stx VMADDL                  ; word $0000 = tile $00
    ldx #$0000
@loop:
    sep #$20
    .a8
    lda f:font_data,x
    sta VMDATAL
    rep #$30
    .a16
    .i16
    inx
    cpx #FONT_SIZE
    bne @loop
    rts
.endproc

; ---------------------------------------------------------------------------------------------
; Fill the visible tilemap with spaces.
; ---------------------------------------------------------------------------------------------
.proc clear_tilemap
    sep #$20
    .a8
    stz VMAIN
    rep #$30
    .a16
    .i16
    ldx #MAP_BASE
    stx VMADDL
    ldx #(SCREEN_COLS * 32)
    sep #$20
    .a8
@loop:
    lda #' '
    sta VMDATAL
    rep #$10
    .i16
    dex
    bne @loop
    rts
.endproc

; ---------------------------------------------------------------------------------------------
; Test runner
;
; Walks the generated dispatch table, calling each test with the canonical entry state and
; recording its verdict. Runs with the screen blanked and interrupts off.
; ---------------------------------------------------------------------------------------------
.proc run_all_tests
    rep #$30
    .a16
    .i16
    ; Publish the block header up front so a harness that samples early can tell the difference
    ; between "not started" and "no such block".
    lda #$4341                  ; "AC" little-endian
    sta f:R_MAGIC
    lda #$4E53                  ; "SN"
    sta f:R_MAGIC + 2
    lda #R_FORMAT_VERSION
    sta f:R_VERSION
    lda f:_test_count
    sta f:R_COUNT
    lda #0
    sta f:R_PASSED
    sta f:R_FAILED
    sta f:R_SKIPPED
    sta f:R_GOLDEN
    sep #$20
    .a8
    lda #$00
    sta f:R_DONE                ; STZ has no long addressing mode
    rep #$30
    .a16
    .i16

    lda #$0000
    sta f:V_TEST_IDX

rt_next:
    lda f:V_TEST_IDX
    cmp f:_test_count
    bcc :+
    jmp rt_finished
:
    ; Mark not-run, then call.
    sep #$20
    .a8
    lda #VERDICT_NOTRUN
    sta f:V_TEST_RESULT
    rep #$30
    .a16
    .i16

    tsc
    sta f:V_SAVED_S             ; test_restore rebuilds the stack from this

    ; index -> 3-byte table offset. The entries are 24-bit because test bodies outgrew bank $00;
    ; see V_DISPATCH in runtime.inc.
    lda f:V_TEST_IDX
    sta a:V_DISPATCH_TMP        ; the untripled index
    asl
    clc
    adc a:V_DISPATCH_TMP
    tax
    lda f:_test_entries,x       ; low and mid bytes
    sta a:V_DISPATCH
    sep #$20
    .a8
    lda f:_test_entries+2,x     ; bank byte, which must land at V_DISPATCH+2
    sta a:V_DISPATCH+2
    rep #$30
    .a16
    .i16

    ; Canonical entry state: native, 16-bit A/X/Y, DP = $0000, DBR = $00.
    lda #$0000
    tcd
    phk
    plb
    jsr call_indirect

test_restore_target:            ; every test returns or jumps here
    ; --- record the verdict ---
    rep #$30
    .a16
    .i16
    lda f:V_TEST_IDX
    tax
    sep #$20
    .a8
    lda f:V_TEST_RESULT
    sta f:R_STATUS,x

    ; --- tally ---
    ldy #$0000
    lda f:V_TEST_IDX            ; low byte is enough: the battery is far under 256 tests
    rep #$10
    .i16
    tax
    sep #$20
    .a8
    lda f:_test_flags,x
    and #$02                    ; golden-vector?
    beq rt_not_golden
    rep #$20
    .a16
    lda f:R_GOLDEN
    inc a
    sta f:R_GOLDEN
    sep #$20
    .a8
    bra rt_tallied
rt_not_golden:
    lda f:_test_flags,x
    and #$01                    ; does it score at all?
    beq rt_tallied                ; Contested/Novel: recorded, never counted
    lda f:V_TEST_RESULT
    cmp #VERDICT_SKIP
    beq rt_skipped
    and #$01                    ; bit 0 set = pass (possibly with a variant code)
    beq rt_failed
    rep #$20
    .a16
    lda f:R_PASSED
    inc a
    sta f:R_PASSED
    sep #$20
    .a8
    bra rt_tallied
rt_failed:
    rep #$20
    .a16
    lda f:R_FAILED
    inc a
    sta f:R_FAILED
    sep #$20
    .a8
    bra rt_tallied
rt_skipped:
    rep #$20
    .a16
    lda f:R_SKIPPED
    inc a
    sta f:R_SKIPPED
    sep #$20
    .a8
rt_tallied:
    rep #$30
    .a16
    .i16
    lda f:V_TEST_IDX
    inc a
    sta f:V_TEST_IDX
    jmp rt_next

rt_finished:
    sep #$20
    .a8
    lda #R_DONE_MARK
    sta f:R_DONE                ; the completion sentinel the harness polls for
    rts
.endproc

; Indirect call into the test whose 24-bit entry point is in V_DISPATCH. Kept as its own routine
; so the return address the test sees is well-defined.
;
; No test actually returns through it: every one exits with `jml test_restore`, which is a long
; jump for the same reason this is a long indirect — a body may live outside bank $00, and
; test_restore does not.
.proc call_indirect
    jmp [V_DISPATCH]            ; JMP [abs]: a 24-bit indirect
.endproc

; ---------------------------------------------------------------------------------------------
; test_restore — the universal test exit path.
;
; A test may have corrupted the stack pointer, the direct page, the data bank, and the E/M/X
; flags. All of it is rebuilt here from V_SAVED_S, which is why tests can be written to abuse
; emulation mode and the stack without any per-test cleanup.
; ---------------------------------------------------------------------------------------------
.proc test_restore_impl
    clc
    xce                         ; force native regardless of what the test left
    rep #$30
    .a16
    .i16
    lda f:V_SAVED_S
    tcs                         ; stack back to its pre-call value
    lda #$0000
    tcd                         ; DP = $0000
    phk
    plb                         ; DBR = $00
    jmp run_all_tests::test_restore_target
.endproc

.export test_restore
test_restore := test_restore_impl

; ---------------------------------------------------------------------------------------------
; Drawing
; ---------------------------------------------------------------------------------------------

; X = VRAM word address, Y = pointer to a length-prefixed ASCII string in bank $00.
.proc draw_str
    sep #$20
    .a8
    stz VMAIN                   ; advance after the low byte only
    rep #$30
    .a16
    .i16
    stx VMADDL
    sep #$20
    .a8
    lda a:0,y                   ; length
    beq @done
    rep #$20
    .a16
    and #$00FF
    tax                         ; X = remaining characters
    sep #$20
    .a8
    iny
@loop:
    lda a:0,y
    sta VMDATAL
    iny
    rep #$10
    .i16
    dex
    beq @done
    sep #$20
    .a8
    bra @loop
@done:
    rts
.endproc

; X = VRAM word address, A (8-bit) = character.
.proc draw_char
    pha
    sep #$20
    .a8
    stz VMAIN
    rep #$10
    .i16
    stx VMADDL
    pla
    sta VMDATAL
    rts
.endproc

; Draw the whole screen: header, tallies, and one row per test.
.proc draw_screen
    rep #$30
    .a16
    .i16
    ldx #MAP_BASE
    ldy #str_title
    jsr draw_str

    ; Row 1: the tallies.
    ldx #(MAP_BASE + SCREEN_COLS)
    ldy #str_tally
    jsr draw_str
    lda f:R_PASSED
    ldx #(MAP_BASE + SCREEN_COLS + 2)
    jsr draw_dec3
    lda f:R_FAILED
    ldx #(MAP_BASE + SCREEN_COLS + 9)
    jsr draw_dec3
    lda f:R_GOLDEN
    ldx #(MAP_BASE + SCREEN_COLS + 16)
    jsr draw_dec3
    lda f:R_COUNT
    ldx #(MAP_BASE + SCREEN_COLS + 23)
    jsr draw_dec3

    jsr draw_list
    rts
.endproc

; Draw the scrollable test list starting at V_SCROLL.
.proc draw_list
    rep #$30
    .a16
    .i16
    lda #$0000
    sta f:V_TMP                 ; row counter
@row:
    lda f:V_TMP
    cmp #VISIBLE_ROWS
    bcc :+
    jmp @done
  :
    clc
    adc f:V_SCROLL
    cmp f:R_COUNT
    bcc :+
    jmp @blank_rest
  :

    ; VRAM address for this row: MAP_BASE + (row + 3) * 32
    pha                         ; test index
    lda f:V_TMP
    clc
    adc #3
    asl
    asl
    asl
    asl
    asl                         ; * 32
    clc
    adc #MAP_BASE
    tax
    pla                         ; test index back
    pha
    asl
    phx
    tax
    lda f:_test_names,x         ; pointer to the length-prefixed name
    tay
    plx
    txa
    clc
    adc #2                      ; leave two columns for the cursor
    tax
    jsr draw_str

    ; Verdict character at column 29 of this row.
    lda f:V_TMP
    clc
    adc #3
    asl
    asl
    asl
    asl
    asl
    clc
    adc #(MAP_BASE + 29)
    tax
    pla                         ; test index
    pha
    phx
    tax
    sep #$20
    .a8
    lda f:R_STATUS,x
    jsr verdict_char
    plx
    jsr draw_char
    rep #$30
    .a16
    .i16
    pla                         ; discard the saved index

    ; Cursor marker.
    lda f:V_TMP
    clc
    adc f:V_SCROLL
    cmp f:V_CURSOR
    bne @no_cursor
    lda f:V_TMP
    clc
    adc #3
    asl
    asl
    asl
    asl
    asl
    clc
    adc #MAP_BASE
    tax
    sep #$20
    .a8
    lda #'>'
    jsr draw_char
    rep #$30
    .a16
    .i16
@no_cursor:
    lda f:V_TMP
    inc a
    sta f:V_TMP
    jmp @row

@blank_rest:
@done:
    rts
.endproc

; A (8-bit) = verdict byte -> A (8-bit) = display character.
.proc verdict_char
    .a8
    cmp #VERDICT_NOTRUN
    bne :+
    lda #'.'
    rts
:
    cmp #VERDICT_SKIP
    bne :+
    lda #'S'
    rts
:
    and #$01
    beq :+
    lda #'P'
    rts
:
    lda #'F'
    rts
.endproc

; A (16-bit) = value 0-999, X = VRAM word address. Writes three digits.
.proc draw_dec3
    rep #$30
    .a16
    .i16
    sta f:V_TMP
    phx
    ; hundreds
    lda f:V_TMP
    ldx #$0000
@h:
    cmp #100
    bcc @h_done
    sec
    sbc #100
    inx
    bra @h
@h_done:
    sta f:V_TMP
    txa
    clc
    adc #'0'
    plx
    phx
    pha
    sep #$20
    .a8
    pla
    jsr draw_char
    rep #$30
    .a16
    .i16
    ; tens
    plx
    inx
    phx
    lda f:V_TMP
    ldx #$0000
@t:
    cmp #10
    bcc @t_done
    sec
    sbc #10
    inx
    bra @t
@t_done:
    sta f:V_TMP
    txa
    clc
    adc #'0'
    plx
    phx
    pha
    sep #$20
    .a8
    pla
    jsr draw_char
    rep #$30
    .a16
    .i16
    ; units
    plx
    inx
    lda f:V_TMP
    clc
    adc #'0'
    pha
    sep #$20
    .a8
    pla
    jsr draw_char
    rep #$30
    .a16
    .i16
    rts
.endproc

; ---------------------------------------------------------------------------------------------
; Input — manual $4016 read, MSB (B) first.
; ---------------------------------------------------------------------------------------------
.proc read_pad
    sep #$20
    .a8
    lda #$01
    sta JOYSER0                 ; strobe high: latch the buttons
    stz JOYSER0                 ; strobe low: start shifting
    rep #$30
    .a16
    .i16
    lda f:V_PAD_HELD
    sta f:V_PAD_LAST
    lda #$0000
    sta f:V_PAD_HELD
    ldx #16
@loop:
    sep #$20
    .a8
    lda JOYSER0
    lsr                         ; data bit -> carry
    rep #$20
    .a16
    lda f:V_PAD_HELD
    rol                         ; shift it in, MSB first
    sta f:V_PAD_HELD
    rep #$10
    .i16
    dex
    bne @loop

    ; newly pressed = held AND NOT last
    lda f:V_PAD_LAST
    eor #$FFFF
    and f:V_PAD_HELD
    sta f:V_PAD_NEW
    rts
.endproc

; Wait for the start of vblank: first leave it, then wait to re-enter.
; The naive one-loop form returns immediately when called from inside vblank, leaving only a
; fraction of the period for VRAM writes — a classic source of intermittent corruption.
;
; Exported because the Group C sprite tests need it: they are the only tests that release forced
; blank, and they must render a COMPLETE frame to sample the sprite over-flags deterministically.
; Two back-to-back calls do that — the first lands on a vblank boundary, the second spans a whole
; active period. Calling it once from an arbitrary point mid-frame would evaluate only the
; scanlines that happened to remain.
.export wait_vblank
.proc wait_vblank
    sep #$20
    .a8
:
    lda HVBJOY
    bmi :-                      ; still in vblank -> wait for active display
:
    lda HVBJOY
    bpl :-                      ; wait for vblank to begin
    rts
.endproc

; ---------------------------------------------------------------------------------------------
; Rendered scenes — the host-side framebuffer oracle (docs/adr/0013).
;
; Some PPU behaviour decides only what appears on screen: no register reads back, no counter moves.
; A cart cannot judge those, because there is no path from rendered pixels to the CPU. So the cart
; renders and the HOST judges, and the results stay in their own tier — never folded into the
; on-cart pass rate, because a rendered scene does not have the "runs unmodified anywhere" property
; the rest of the battery does.
;
; The cart drives itself rather than being driven: for each scene it sets up PPU state, publishes
; the scene ID, and holds for SCENE_FRAMES frames. The host watches R_SCENE, and hashes the
; framebuffer on the last frame of each hold. On real hardware the same loop is a slideshow.
; The canvas every scene renders. Set up once, before the scene loop.
;
; The battery's own screen is deliberately austere: two CGRAM entries (black and white) and a
; tilemap of spaces. That is right for a text menu and useless for a framebuffer oracle — a mosaic
; over a blank screen is a blank screen, and colour math over two colours is nearly one. The first
; version of these scenes proved it: `c8-fixed-colour-add` and `c10-mosaic-4x` hashed *identically*,
; because neither had anything to act on.
;
; So: 128 CGRAM entries spread across the colour cube, and a tilemap whose tile index, palette and
; priority all vary per cell. Both are generated arithmetically rather than stored, which keeps the
; ROM small and makes the content reproducible from this code alone.
.proc scene_canvas
    sep #$20
    .a8
    .i16

    ; Forced blank FIRST. VRAM and CGRAM are only writable outside active display, and the battery
    ; leaves the screen in whatever state its last test wanted — so without this the canvas is
    ; uploaded through whatever each emulator does with a blocked write, and the two disagree about
    ; the picture for reasons that have nothing to do with the scene.
    lda #$8F
    sta INIDISP

    ; Rebuild VRAM from scratch rather than inheriting the boot-time upload. Tests write VRAM, and
    ; a test that writes it during active display (or through an open-bus read) leaves DIFFERENT
    ; contents on different emulators — so a scene rendered over the leftovers compares those
    ; leftovers, not the scene. This cost a full round of chasing a phantom disagreement.
    jsr clear_vram
    jsr load_font
    sep #$20
    .a8

    ; --- CGRAM: all 256 entries, none equal, spread over red/green/blue ---
    ;
    ; All 256, not the 128 a 4bpp mode can reach. An 8bpp or Mode 7 scene indexes the whole
    ; palette, so leaving the upper half alone would have it render through whatever the previous
    ; scene or test left in CGRAM — the same cross-scene contamination the per-scene canvas rebuild
    ; exists to prevent, in the one place the rebuild was not reaching.
    stz CGADD
    rep #$30
    .a16
    .i16
    ldx #$0000
@pal:
    ; entry = i*$0111 + $0421 — the multiplier puts a copy of i in each 5-bit channel, so red,
    ; green and blue all vary; the addend keeps every channel non-zero so no entry renders as
    ; pure black and silently drops out of the comparison.
    txa
    asl a
    asl a
    asl a
    asl a                       ; i << 4
    sta f:V_TMP
    txa
    asl a
    asl a
    asl a
    asl a
    asl a
    asl a
    asl a
    asl a                       ; i << 8
    clc
    adc f:V_TMP
    sta f:V_TMP
    txa
    clc
    adc f:V_TMP
    clc
    adc #$0421
    sep #$20
    .a8
    sta CGDATA
    xba
    sta CGDATA
    xba
    rep #$30
    .a16
    .i16
    inx
    cpx #256
    bne @pal

    ; --- BG1 tilemap: varying tile, palette and priority ---
    sep #$20
    .a8
    lda #$80
    sta VMAIN                   ; increment after the HIGH byte, so a 16-bit store writes one entry
    rep #$30
    .a16
    .i16
    ldx #MAP_BASE
    stx VMADDL
    ldx #$0000                  ; cell index, 0..1023
@cell:
    txa
    and #$003F
    clc
    adc #$0021                  ; tile: 64 distinct glyphs starting at '!'
    sta f:V_TMP
    txa
    lsr a
    lsr a
    lsr a
    lsr a
    lsr a                       ; row
    clc
    adc f:V_TMP
    and #$00FF                  ; keep it inside the font
    sta f:V_TMP
    txa
    and #$0007
    asl a
    asl a
    asl a
    asl a
    asl a
    asl a
    asl a
    asl a
    asl a
    asl a                       ; palette -> bits 10-12
    and #$1C00
    ora f:V_TMP
    sta VMDATAL                 ; 16-bit store = low byte then high byte, then increment
    inx
    cpx #(SCREEN_COLS * 32)
    bne @cell
    rts
.endproc

; Wait X whole frames. Split out because the scene loop needs it twice and the register juggling
; around `wait_vblank` (which returns with an 8-bit accumulator) is easy to get subtly wrong.
.proc hold_frames
    .a16
    .i16
@loop:
    phx
    jsr wait_vblank
    plx
    rep #$30
    .a16
    .i16
    dex
    bne @loop
    rts
.endproc

.proc run_scenes
    ; The battery leaves DBR and DP wherever the last test put them — a test owns its own bank and
    ; direct page (see the dispatch loop above, which re-establishes both before every call). So
    ; re-establish them here too, or every absolute $21xx store in a scene setup lands somewhere
    ; else entirely. This is not hypothetical: the first version of this loop read its own scene
    ; count through a stale DBR and stopped after one scene.
    rep #$30
    .a16
    .i16
    lda #$0000
    tcd
    phk
    plb                         ; DP = $0000, DBR = $00

    sep #$20
    .a8
    lda #$00
    sta f:R_SCENE
    sta f:R_SCENE_DONE

    rep #$30
    .a16
    ldx #$0000                  ; scene index
@next:
    ; Every scene starts from the canonical state — registers AND memory. Otherwise scene N
    ; renders through whatever scene N-1 left in CGWSEL/CGADSUB/MOSAIC, and the goldens record an
    ; accumulated state rather than the one thing each scene is supposed to be evidence for.
    ;
    ; The canvas is rebuilt per scene rather than once, which is what makes a scene free to rewrite
    ; VRAM for its own purposes — the flip-bit and low-tile scenes need exactly that. Rebuilding it
    ; once was cheaper and wrong: the first scene to rewrite the tilemap silently changed the
    ; picture for every scene after it, and three scenes hashed identically as a result.
    sep #$20
    .a8
    lda #$8F
    sta INIDISP                 ; forced blank while the state is reset
    phx
    jsr init_registers
    jsr scene_canvas
    plx
    rep #$30
    .a16
    .i16
    txa
    cmp f:_scene_count          ; long addressing: independent of DBR by construction
    bcs @finished

    phx
    rep #$30
    txa
    asl a
    tax
    lda f:_scene_entries,x      ; setup routine address
    sta f:V_DISPATCH
    plx
    phx
    jsr @call_setup
    plx

    ; The host samples at frame boundaries and cannot see where inside a hold it is. So the ID is
    ; published only once a whole frame has been rendered with the scene in place, and is cleared
    ; again before anything disturbs it: every frame on which the host sees a non-zero ID is a
    ; frame of that scene at its steady state, and the host can take the first one. Publishing the
    ; ID immediately instead lets the host catch the setup frame or the trailing blank — which it
    ; did, and snes9x captured an all-black scene 3 for exactly that reason.
    rep #$30
    .a16
    .i16
    phx
    ldx #SCENE_SETTLE
    jsr hold_frames
    plx

    sep #$20
    .a8
    txa
    inc a
    sta f:R_SCENE               ; scene IDs are 1-based; 0 means "none yet"

    rep #$30
    .a16
    .i16
    phx
    ldx #SCENE_FRAMES
    jsr hold_frames
    plx

    sep #$20
    .a8
    lda #$00
    sta f:R_SCENE               ; the steady-state window is over (STZ has no long form)

    rep #$30
    .a16
    .i16
    inx
    bra @next

@call_setup:
    jmp (V_DISPATCH)

@finished:
    sep #$20
    .a8
    lda #$5A
    sta f:R_SCENE_DONE
    ; leave the screen blanked again so the menu draws from a known state
    lda #$8F
    sta INIDISP
    stz TM
    rts
.endproc

; ---------------------------------------------------------------------------------------------
; Interactive list. The battery has already finished by the time we get here.
; ---------------------------------------------------------------------------------------------
.proc main_loop
@frame:
    jsr wait_vblank
    jsr read_pad

    rep #$30
    .a16
    .i16
    lda f:V_PAD_NEW
    bit #PAD_UP
    beq :+
    jsr cursor_up
:
    lda f:V_PAD_NEW
    bit #PAD_DOWN
    beq :+
    jsr cursor_down
:
    lda f:V_DIRTY
    and #$00FF
    beq @frame
    sep #$20
    .a8
    lda #$00
    sta f:V_DIRTY               ; STZ has no long addressing mode
    jsr draw_list               ; still inside vblank: VRAM writes are legal here
    jmp @frame
.endproc

.proc cursor_up
    rep #$30
    .a16
    .i16
    lda f:V_CURSOR
    beq @done
    dec a
    sta f:V_CURSOR
    cmp f:V_SCROLL
    bcs @dirty
    sta f:V_SCROLL
@dirty:
    sep #$20
    .a8
    lda #$01
    sta f:V_DIRTY
@done:
    rts
.endproc

.proc cursor_down
    rep #$30
    .a16
    .i16
    lda f:V_CURSOR
    inc a
    cmp f:R_COUNT
    bcs @done
    sta f:V_CURSOR
    sec
    sbc f:V_SCROLL
    cmp #VISIBLE_ROWS
    bcc @dirty
    lda f:V_CURSOR
    sec
    sbc #(VISIBLE_ROWS - 1)
    sta f:V_SCROLL
@dirty:
    sep #$20
    .a8
    lda #$01
    sta f:V_DIRTY
@done:
    rts
.endproc

; ---------------------------------------------------------------------------------------------
; APU program upload, through the IPL boot ROM's handshake.
;
; Group E cannot be tested from the 65816 at all: the SPC700 is a separate processor with its own
; RAM, and the only channel between them is four bytes. So the cart uploads a small SPC700 program,
; lets it run, and reads its answers back through those same four ports — which is exactly what a
; game's sound driver does at boot, and the reason the IPL ROM exists.
;
; The protocol is the documented one and it is unforgiving: every byte is a request/echo round
; trip, and a single missed echo desynchronises the whole transfer with no error signal. It is
; written out step by step here rather than compressed, because a compressed version of a
; handshake is unreadable at exactly the moment you need to read it.
;
; Parameters in V_APU_* (see runtime.inc). Clobbers A/X/Y. Returns with the program running.
; ---------------------------------------------------------------------------------------------
; frame_step — render exactly one frame, then force blank again.
;
; The battery runs under forced blank throughout, which is what makes VRAM, OAM and CGRAM freely
; accessible. A handful of assertions are about things that only happen when rendering STARTS: the
; OAM address reload on `$2100` bit 7's falling edge (`C1.07`), the sprite range/time-over flags
; clearing at the end of vblank (`C7.09`), the mid-frame overscan hazard (`C9.05`). None of them
; can be reached from a straight-line test, because nothing in one crosses a frame boundary — the
; first attempt at `C1.07` read back the un-reloaded address on all three emulators.
;
; The sequence is deliberately anchored at both ends. It waits for vblank before clearing blank, so
; rendering resumes at a known place rather than mid-scanline; then waits for vblank to END, which
; is the transition the hardware acts on; then waits for the NEXT vblank, by which point a whole
; frame has been drawn. Blank goes back on before returning, so the caller is handed the same
; freely-accessible PPU it had.
;
; Costs about two frames per call. Width-neutral (`php`/`plp`).
.export frame_step
.proc frame_step
    php
    sep #$20
    .a8
@wait_vblank:
    lda HVBJOY
    and #$80
    beq @wait_vblank            ; start from inside vblank, not mid-picture
    lda #$0F
    sta INIDISP                 ; forced blank off: the 1->0 edge some tests are about
@wait_active:
    lda HVBJOY
    and #$80
    bne @wait_active            ; rendering has begun
@wait_vblank2:
    lda HVBJOY
    and #$80
    beq @wait_vblank2           ; ...and a whole frame has been drawn
    lda #$8F
    sta INIDISP                 ; blank again, so the caller's PPU access is safe
    plp
    rts
.endproc

; A long-callable wrapper, for test bodies that do not live in bank $00. A plain `jsr` from
; another bank lands at the same 16-bit address *in that bank*, which is not a subroutine — it is
; whatever bytes happen to be there. Group E's bodies are in bank $02, so they call this.
.export apu_upload_far
.proc apu_upload_far
    jsr apu_upload
    rtl
.endproc

.export apu_upload
.proc apu_upload
    php
    rep #$30
    .a16
    .i16
    phb
    phd
    phk
    plb
    ; D = 0 for the duration. The pointer is written and read through the SAME addressing mode
    ; below, which is only meaningful if D is known: `sta a:$50` and `lda [$50],y` name the same
    ; byte when D = 0 and different bytes otherwise, and a test is free to have moved D.
    lda #$0000
    tcd

    ; Copy the source pointer into direct page: `lda [dp],y` is the only mode that reaches an
    ; arbitrary bank with an index, and it needs the pointer there.
    lda f:V_APU_SRC
    sta z:V_APU_PTR
    sep #$20
    .a8
    lda f:V_APU_BANK
    sta z:V_APU_PTR + 2

    ; --- wait for the IPL to announce itself with $AA/$BB ---
    ;
    ; Not optional and not merely polite: the IPL writes these only once it is ready to listen,
    ; and everything below assumes it is.
    rep #$30
    .a16
    .i16
    lda #$0001
    sta f:V_APU_STAGE
    ldx #$0000
@ready:
    sep #$20
    .a8
    lda APUIO0
    cmp #$AA
    bne :+
    lda APUIO1
    cmp #$BB
    beq @open
  :
    rep #$30
    .a16
    .i16
    inx
    bne :+
    jmp @fail                   ; out of branch range: the bounded waits are far from the exit
  :
    bra @ready

@open:
    ; --- open a transfer at the destination address ---
    rep #$30
    .a16
    .i16
    lda #$0002
    sta f:V_APU_STAGE
    lda f:V_APU_DEST
    sta APUIO2                  ; $2142/$2143 take the 16-bit address in one store
    sep #$20
    .a8
    lda #$01
    sta APUIO1                  ; non-zero: begin a transfer rather than jump
    lda #$CC
    sta APUIO0
    rep #$10
    .i16
    ldx #$0000
@kick:
    sep #$20
    .a8
    cmp APUIO0                  ; the IPL echoes the kick value once it has taken it
    beq @loop_init
    rep #$10
    .i16
    inx
    bne :+
    jmp @fail                   ; out of branch range: the bounded waits are far from the exit
  :
    bra @kick

@loop_init:
    ; --- one byte per round trip ---
    ;
    ; Y is both the source index and the protocol counter, because they are the same number: the
    ; IPL's acknowledgement value is "how many bytes have arrived". One variable for one concept,
    ; so they cannot drift apart — a drift would look like a corrupted upload, not a counter bug.
    rep #$30
    .a16
    .i16
    lda #$0003
    sta f:V_APU_STAGE
    ldy #$0000
@byte:
    tya
    cmp f:V_APU_LEN
    beq @done
    sep #$20
    .a8
    lda [V_APU_PTR],y           ; direct-page indirect long: the image may live in any bank
    sta APUIO1                  ; the data byte
    tya
    sta APUIO0                  ; the counter, which is also the acknowledgement
    rep #$10
    .i16
    ldx #$0000
@echo:
    sep #$20
    .a8
    cmp APUIO0
    beq @next
    rep #$10
    .i16
    inx
    bne :+
    jmp @fail                   ; out of branch range: the bounded waits are far from the exit
  :
    bra @echo
@next:
    rep #$30
    .a16
    .i16
    iny
    bra @byte

@done:
    ; --- close the transfer and jump ---
    ;
    ; The final counter is the last one plus TWO: one for the byte that was never sent, one
    ; because the IPL treats the value as "next expected". Plus one is the classic off-by-one here,
    ; and it hangs the handshake rather than corrupting anything — which at least fails loudly.
    rep #$30
    .a16
    .i16
    lda #$0004
    sta f:V_APU_STAGE
    lda f:V_APU_ENTRY
    sta APUIO2
    sep #$20
    .a8
    stz APUIO1                  ; zero: jump instead of continuing the transfer
    tya                         ; 8-bit: the IPL's counter is a byte, so the width buys nothing
    clc
    adc #$02
    sta APUIO0
    rep #$10
    .i16
    ldx #$0000
@final:
    sep #$20
    .a8
    cmp APUIO0
    beq @ok
    rep #$10
    .i16
    inx
    bne :+
    jmp @fail                   ; out of branch range: the bounded waits are far from the exit
  :
    bra @final

@ok:
    rep #$30
    .a16
    .i16
    lda #$0000
    sta f:V_APU_STAGE           ; 0 = the whole handshake completed
    pld
    plb
    plp
    clc
    rts

@fail:
    ; Every wait above is bounded. An unbounded one hangs the entire battery and reports nothing
    ; about the other 150 tests, which is a far worse outcome than one test standing down — and is
    ; exactly what the first version of this did. V_APU_STAGE names the step that gave up.
    rep #$30
    .a16
    .i16
    pld
    plb
    plp
    sec
    rts
.endproc

; ---------------------------------------------------------------------------------------------
; Interrupt handlers. Nothing is enabled, but the vectors must point somewhere sane.
; ---------------------------------------------------------------------------------------------
.export irq_stub
.proc irq_stub
    rti
.endproc

; The IRQ trampoline, for tests that need to observe dispatch rather than the comparator flag.
.export irq_trampoline
.proc irq_trampoline
    jmp (V_IRQ_VEC)
.endproc

; BRK / COP trampolines. The vectors are fixed at link time; these jump through a RAM pointer so
; a test can install its own handler for the duration of one test.
.export brk_trampoline
.proc brk_trampoline
    jmp (V_BRK_VEC)
.endproc

.export cop_trampoline
.proc cop_trampoline
    jmp (V_COP_VEC)
.endproc

; ---------------------------------------------------------------------------------------------
; Cycle measurement, via the PPU's H counter.
;
; Reading SLHV ($2137) latches the H/V counters into OPHCT/OPVCT ($213C/$213D) — a direct numeric
; readout of where in the scanline we are, which is the SNES's equivalent of the sprite-0-hit
; trick AccuracyCoin uses on the NES, and far more precise.
;
; Everything is kept INSIDE one scanline deliberately. Line length is not a constant the tests can
; rely on (NTSC has a short line at V=240, PAL a long one, and emulators disagree on whether the H
; counter tops out at 339 or 340), so crossing a line boundary would make the measurement depend on
; exactly the convention under dispute. `hv_sync` waits until H is small, and every measured
; sequence is short enough that H cannot wrap before it ends.
; ---------------------------------------------------------------------------------------------

; Read the H counter into A (16-bit). Entry/exit: native, A and index 16-bit.
;
; Kept entirely in registers. An earlier version staged the two OPHCT bytes through WRAM, which
; cost 168 dots of overhead per measurement — over half a scanline, so every measured sequence
; wrapped past the end of the line and produced garbage. XBA assembles the halves with no memory
; traffic at all.
.proc hv_read_raw
    sep #$20
    .a8
    lda $213F                   ; reset the OPHCT/OPVCT read flipflops
    lda $2137                   ; SLHV: latch H and V
    lda $213C                   ; OPHCT low 8 bits
    xba                         ; stash it in B
    lda $213C                   ; OPHCT second read: bit 0 = counter bit 8
    and #$01
    xba                         ; A = low, B = high
    rep #$30
    .a16
    .i16
    and #$01FF                  ; C = (high << 8) | low
    rts
.endproc

; Begin a measurement: spin until H is low, and take that reading as the start position.
;
; Both helpers preserve the ENTIRE processor status, including the M/X width bits. That is not
; incidental: the cycle tests measure 8-bit against 16-bit forms of the same instruction, so a
; helper that silently widened the accumulator would make those two measurements identical and
; the test would pass while measuring nothing.
.export hv_begin
.proc hv_begin
    php
    rep #$30
    .a16
    .i16
    cld                         ; see hv_end: the delta arithmetic must not run in BCD
    pha
@wait:
    jsr hv_read_raw
    cmp #16
    bcs @wait
    sta f:V_H0                  ; reuse the poll's own reading — no second latch
    pla
    plp
    rts
.endproc

; End a measurement: record the elapsed dots at V_H1.
.export hv_end
.proc hv_end
    php
    rep #$30
    .a16
    .i16
    cld                         ; SBC honours D. A test measuring with decimal mode set would
                                ; otherwise have its delta computed in BCD — which silently
                                ; under-reports and made decimal-mode ADC look *faster* than
                                ; binary. PLP restores the caller's D on the way out.
    pha
    jsr hv_read_raw
    sec
    sbc f:V_H0
    sta f:V_H1
    pla
    plp
    rts
.endproc

.segment "RODATA"
str_title:
    .byte 15
    .byte "ACCURACYSNES A1"
str_tally:
    .byte 26
    .byte "P:000 F:000 G:000 OF:000  "
