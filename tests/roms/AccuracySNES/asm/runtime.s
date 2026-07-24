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
; Bytes 8-9 of the bank $00 and bank $01 blocks hold the bank-probe stub addresses, so a test can
; aim an `(a,X)` indirect jump at $xx:8008 and find out which bank the pointer actually came from.
;
; Offsets 8-9 and NOT 6-7: `A2.11` forms a `(dp,X)` pointer at $00:8005-8006 and reads through it,
; so putting an address in byte 6 changes what that test loads. Found by A2.11 failing the moment
; the first version of this landed. Bytes 6-7 and 10-15 stay zero.
.segment "SIG0"
    .byte "SIG0", $00, $A0, $00, $00
    .addr bankprobe_0
    .byte $00, $00, $00, $00, $00, $00
; BANK1-3 sit at $01/$02/$03:$8000 under LoROM. A HiROM image maps those CPU addresses to ROM
; offsets that a small second image does not contain, so the per-bank signature blocks are LoROM-only.
; The generator defines HIROM_BUILD when assembling this file for the HiROM/ExHiROM images (whose
; battery excludes the bank-crossing A4 tests these serve). SIG0 stays: $00:$8000 is a valid HiROM
; window address and the block is inert when no test reads it.
.ifndef HIROM_BUILD
.segment "BANK1"
    .byte "SIG1", $00, $A1, $00, $00
    .addr bankprobe_1
    .byte $00, $00, $00, $00, $00, $00
.segment "BANK2"
    .byte "SIG2", $00, $A2, $00, $00, $00, $00, $00, $00, $00, $00, $00, $00
.segment "BANK3"
    .byte "SIG3", $00, $A3, $00, $00, $00, $00, $00, $00, $00, $00, $00, $00
.endif

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

    ; Power-on path: this is a real reset, not a menu restart. `restart_entry` is below and does
    ; NOT pass through here, so a Select restart leaves V_RESTARTED at the 1 its handler wrote.
    lda #$00
    sta f:V_RESTARTED

    ; V_MENU_MODE gates every test's verdict path (test_restore_target reads it to decide whether to
    ; tally or bounce to the menu), so it MUST be zero before the battery runs. WRAM powers up zeroed
    ; on some cores and with garbage on others -- snes9x hung the whole battery here, taking the
    ; menu branch after the first test, until this landed. Cross-validation caught it; the in-repo
    ; harness could not, because RustySNES zeroes WRAM. Same class as the cursor variables below.
    sta f:V_MENU_MODE

    rep #$20
    .a16
    lda #irq_stub
    sta a:V_IRQ_VEC             ; default: the IRQ trampoline behaves exactly like the old stub
    sta a:V_COP_VEC_E           ; and the emulation COP pointer, so a COP in a test that has
                                ; installed no handler returns instead of jumping into RAM
    sta a:V_NMI_VEC             ; and NMI, for the same reason: the battery keeps NMITIMEN clear,
                                ; so this is unreachable until a test opts in, but a test that
                                ; enables NMI without installing a handler must not jump into RAM
    sep #$20
    .a8

    lda #$00                    ; clear the capture-complete marker BEFORE capturing (G1.05 asserts
    sta f:V_PO_READY            ; it) — STZ has no long-addressing form, so LDA/STA
    jsr capture_power_on        ; MUST precede init_registers — see below

    ; Cold-boot only: clear the user-skip bitmap. WRAM powers up holding garbage, so without this a
    ; random set of tests would report SKIP on the very first run. A Select restart re-enters at
    ; restart_entry below (past this), which is what preserves the marks the user set with B.
    rep #$30
    .a16
    .i16
    ldx #$0000
    lda #$0000
@clr_skip:
    sta f:V_USER_SKIP,x
    inx
    inx
    cpx #USER_SKIP_BYTES
    bne @clr_skip

    ; Start, from the menu, restarts the whole battery -- but re-enters HERE, after the power-on
    ; capture, not at reset. The V_PO_* values are only meaningful in the first instructions after a
    ; real reset; re-capturing them from a fully-initialised machine would fill the Group G power-on
    ; rows with garbage. init_registers onward is safe to repeat.
restart_entry:
    jsr init_registers
    jsr clear_vram
    jsr load_palette
    jsr load_font
    jsr load_sprite_font        ; 4bpp OBJ code font at $4000, for the skyline variant-code sprites
    jsr init_oam                ; point OBSEL at it and clear OAM (scenes/battery never touch $4000)
    jsr clear_tilemap

    ; --- Initial menu, BEFORE the battery runs: show every test as not-run ("TEST") across all the
    ; pages, then wait for Start to begin -- AccuracyCoin's "here are the tests, press Start to run".
    ; The results array is zeroed first so nothing reads back as a stale PASS/FAIL from power-on
    ; garbage, and V_MENU_MODE is cleared so the battery's verdict path tallies (rather than diverting
    ; to the menu as a single-test A re-run would). WRAM powers up garbage (`G1.07` measures that), so
    ; the menu state is initialised here too -- a garbage V_PAGE / V_CURSOR would index the page
    ; tables out of range and draw a nonsense list.
    jsr reset_results
    rep #$30
    .a16
    .i16
    lda #$0000
    sta f:V_PAGE
    sta f:V_VIEW                ; the paged test menu
    lda #$FFFF
    sta f:V_CURSOR             ; land on the page header
    sep #$20
    .a8
    lda #$00
    sta f:V_DIRTY               ; STZ has no long addressing mode
    sta f:V_MENU_MODE
    rep #$30
    .a16
    .i16
    jsr draw_screen
    jsr setup_bg_unblank
    jsr wait_for_start          ; a contract-holding host returns at once; a human browses + presses Start

    ; --- Run the battery under forced blank, then the rendered scenes. ---
    sep #$20
    .a8
    lda #$8F
    sta INIDISP
    rep #$30
    .a16
    .i16
    jsr run_all_tests

    ; Rendered scenes (ADR 0013) are a LoROM-image feature: they need the scene table from scenes.s,
    ; which the minimal HiROM image does not link. HiROM self-scores its battery and stops.
    .ifndef HIROM_BUILD
    jsr run_scenes              ; rendered scenes for the host framebuffer oracle (ADR 0013)
    .endif

    ; --- Skyline results, at the END: the AccuracyCoin "city" is what you land on once the battery
    ; concludes. Re-establish the font and palette (the battery and scenes leave both in a test-defined
    ; state; a scene overwrites the $0800 inverse-font copy with its offset-per-tile map). From here
    ; Start toggles to the menu (now showing the verdicts) and back.
    sep #$20
    .a8
    lda #$8F
    sta INIDISP
    rep #$30
    .a16
    .i16
    jsr load_font
    jsr load_sprite_font        ; a VRAM test may have overwritten the $4000 sprite font
    jsr init_oam
    jsr load_palette
    rep #$30
    .a16
    .i16
    lda #$0000
    sta f:V_PAGE
    lda #$FFFF
    sta f:V_CURSOR
    sep #$20
    .a8
    lda #$01
    sta f:V_VIEW               ; skyline
    lda #$00
    sta f:V_SKY_SCREEN
    sta f:V_DIRTY
    rep #$30
    .a16
    .i16
    ; Prime the edge-detector so a host holding the input contract does not act on the first frame.
    jsr read_pad
    rep #$30
    .a16
    .i16
    lda f:V_PAD_HELD
    sta f:V_PAD_LAST
    jsr draw_current
    jsr setup_bg_unblank
    jmp main_loop
.endproc

; Establish the menu/skyline background (mode 0, BG1 at MAP_BASE) and release forced blank. Shared by
; the initial menu, the wait-for-Start preview, and the battery-done skyline.
.proc setup_bg_unblank
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
    lda #$11
    sta TM                      ; BG1 + OBJ on the main screen (OAM is empty unless the skyline fills it)
    lda #$0F
    sta INIDISP                 ; release forced blank
    rep #$30
    .a16
    .i16
    rts
.endproc

; Zero the per-test results array (so the pre-battery menu shows every test not-run) and publish the
; test count. WRAM powers up garbage, so without this the initial menu would show stale verdicts.
.proc reset_results
    rep #$30
    .a16
    .i16
    lda f:_test_count
    sta f:R_COUNT
    ; ceil(count / 2) word-writes cover the byte array. A down-counter in Y avoids a `cpx` against the
    ; bank-$7E `_test_count` (CPX has no long-addressing mode; only CMP does).
    lda f:_test_count
    lsr
    inc a
    tay                         ; Y = number of 16-bit stores
    ldx #$0000                  ; X = byte offset into R_STATUS
    lda #$0000                  ; the zero written each iteration (unchanged by STA)
@z:
    sta f:R_STATUS,x            ; two bytes at a time (one extra past an odd count is harmless)
    inx
    inx
    dey
    bne @z
    rts
.endproc

; The pre-battery menu loop: show the test list and wait for Start to begin the battery, with
; Left/Right browsing the pages meanwhile. The pad edge state is zeroed first so a host holding the
; input contract (Start is one of its buttons) registers a Start edge on the very first frame and the
; battery starts at once, while a human sees the menu and starts when ready.
.proc wait_for_start
    rep #$30
    .a16
    .i16
    lda #$0000
    sta f:V_PAD_HELD
    sta f:V_PAD_LAST
@loop:
    jsr wait_vblank
    jsr read_pad
    rep #$30
    .a16
    .i16
    lda f:V_PAD_NEW
    bit #PAD_START
    bne @go
    lda f:V_PAD_NEW
    bit #PAD_LEFT
    beq :+
    jsr page_prev
:
    lda f:V_PAD_NEW
    bit #PAD_RIGHT
    beq :+
    jsr page_next
:
    lda f:V_DIRTY
    and #$00FF
    beq @loop
    sep #$20
    .a8
    lda #$00
    sta f:V_DIRTY
    lda #$80
    sta INIDISP
    rep #$30
    .a16
    .i16
    jsr draw_screen
    sep #$20
    .a8
    lda #$0F
    sta INIDISP
    rep #$30
    .a16
    .i16
    bra @loop
@go:
    rts
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

    ; DMA channel 0's registers, before any test can write them. init_registers clears $420B/$420C
    ; but leaves $43xx alone, so what is here is what reset left -- and every source says that is
    ; $FF across the board (D1.11, the row Heian Fuuunden depends on).
    sep #$20
    .a8
    rep #$10
    .i16
    ldx #$0000
@po_dma:
    lda f:$004300,x
    sta f:V_PO_DMA,x
    inx
    cpx #$000C
    bne @po_dma

    ; G1.03: the readable part of the "everything else is indeterminate" list, sampled before
    ; init_registers writes over it. Reported, never asserted -- that is what the row asks for.
    sep #$20
    .a8
    rep #$10
    .i16
    ldx #$0000
@po_misc:
    lda f:$002140,x
    sta f:V_PO_MISC,x
    inx
    cpx #$0004
    bne @po_misc                ; $2140-$2143, the four APU ports
    lda $2180
    sta f:V_PO_MISC + 4         ; WMDATA; this read also increments WMADD, which is itself
    lda $4218
    sta f:V_PO_MISC + 5         ; one of the indeterminate values this row is about
    lda $4219
    sta f:V_PO_MISC + 6

    ; G1.01, part 1: $4213 (RDIO) reflects $4201's output pins, and $4201 powers on at $FF.
    sep #$20
    .a8
    lda $4213
    sta f:V_PO_RDIO

    ; G1.01, part 2: HTIME and VTIME power on at $1FF. Nothing here writes $4207-$420A, so arming
    ; the timers now tests the values reset left. 511 is past both the 340-dot line and the
    ; 262/312-line frame, so a correct machine never fires. This has to happen before
    ; init_registers, which writes the whole $4200-$420D block and destroys the evidence.
    lda #$00
    sta f:V_PO_TFIRED
    lda $4211                   ; clear any latch
    lda #$30                    ; H-IRQ and V-IRQ, both on the untouched comparators
    sta $4200
    rep #$10
    .i16
    ldx #$0003                  ; three frames is many thousands of comparator matches, if any
@po_frame:
@po_active:
    lda $4211
    and #$80
    bne @po_fired
    lda $4212
    and #$80
    bne @po_active              ; still in vblank; wait for active display
@po_vbl:
    lda $4211
    and #$80
    bne @po_fired
    lda $4212
    and #$80
    beq @po_vbl                 ; wait for the next vblank edge
    dex
    bne @po_frame
    bra @po_done
@po_fired:
    lda #$01
    sta f:V_PO_TFIRED
@po_done:
    sep #$20
    .a8
    stz $4200                   ; disarm; init_registers will set the real state next
    lda $4211

    ; G1.05: the readable PPU registers, sampled last (after the timing-sensitive comparator test
    ; above, still before init_registers writes the PPU). $2134-$2136 is the Mode 7 multiply of the
    ; power-on-undefined M7A/M7B; $213E/$213F are STAT77/STAT78 (defined version bits, undefined
    ; flags). Their address-increment/latch side effects don't matter — init_registers resets the
    ; PPU immediately after. Reported, never asserted, exactly as G1.03 does for the CPU/APU side.
    lda $2134
    sta f:V_PO_PPU + 0
    lda $2135
    sta f:V_PO_PPU + 1
    lda $2136
    sta f:V_PO_PPU + 2
    lda $213E
    sta f:V_PO_PPU + 3
    lda $213F
    sta f:V_PO_PPU + 4

    ; The capture-complete marker, set LAST — so G1.05 can prove every store above ran rather than
    ; reporting "captured" over stale WRAM if this proc were ever bypassed or exited early.
    lda #$A5
    sta f:V_PO_READY

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
; AccuracyCoin-style palette. The font is 2bpp with only bitplane 0 populated, so a glyph uses colour
; 0 (background) and colour 1 (ink) of whichever BG palette its tilemap word selects. Four BG palettes
; share a grey backdrop in colour 0 and carry the four label inks in colour 1, so a label's colour is
; a per-tile palette-bit choice (high-byte bits 2-4), no second tile copy needed:
;
;   pal 0  ink WHITE  $7BBD  -- normal text, TEST, DRAW
;   pal 1  ink BLUE   $7669  -- PASS
;   pal 2  ink RED    $31BD  -- FAIL (and the fail code)
;   pal 3  ink BLACK  $0000  -- SKIP and the in-progress "...."
;
; Colour 0 of palette 0 is CGRAM index 0, which is also the screen backdrop -> grey $1CE7 (NES $2D,
; #3C3C3C), the AccuracyCoin background. Reloaded right before draw_screen because the rendered scenes
; overwrite CGRAM with their own palettes.
;
; A macro keeps the eight two-byte writes honest; CGADD auto-increments, so this walks CGRAM 0..15.
.macro cgcolor lo, hi
    lda #lo
    sta CGDATA
    lda #hi
    sta CGDATA
.endmacro
; Each 2bpp BG palette is FOUR CGRAM entries, so palette N's colours are indices N*4..N*4+3 and the
; ink (colour 1) is at N*4+1. Only colours 0 (grey backdrop) and 1 (ink) are used, so CGADD is set to
; each palette's base and its two used colours written; colours 2/3 are left as whatever (unused).
.proc load_palette
    sep #$20
    .a8
    stz CGADD                   ; palette 0, colour 0 = CGRAM 0 = the screen backdrop
    cgcolor $E7, $1C            ; pal0 col0 grey $1CE7
    cgcolor $BD, $7B            ; pal0 col1 white $7BBD  (normal/TEST/DRAW ink)
    lda #4
    sta CGADD                   ; palette 1 base
    cgcolor $E7, $1C            ; pal1 col0 grey
    cgcolor $69, $76            ; pal1 col1 blue $7669   (PASS ink)
    lda #8
    sta CGADD                   ; palette 2 base
    cgcolor $E7, $1C            ; pal2 col0 grey
    cgcolor $BD, $31            ; pal2 col1 red $31BD    (FAIL ink)
    lda #12
    sta CGADD                   ; palette 3 base
    cgcolor $E7, $1C            ; pal3 col0 grey
    cgcolor $00, $00            ; pal3 col1 black $0000  (SKIP / in-progress ink)
    ; The cursor/skyline highlight does NOT use swapped "inverse" palettes: on a background layer
    ; palette colour 0 is transparent (the backdrop shows through), so swapping colours 0 and 1 cannot
    ; draw a solid bar. The highlight instead uses the inverse-GLYPH font copy at tile $100 (see
    ; load_font) drawn in the state palette 0-3, which is why palettes 4-7 are left unused.
    ; Sprite palette 0 (OBJ palettes begin at CGRAM index 128): grey bg + light-blue $7735, reserved
    ; for the multi-behaviour success-code sprites on the skyline (not yet drawn).
    lda #$80
    sta CGADD
    cgcolor $E7, $1C
    cgcolor $35, $77            ; $7735 light blue
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
    stx VMADDL                  ; word $0000 = tile $00 (upright font)
    ldx #$0000
@upright:
    sep #$20
    .a8
    lda f:font_data,x
    sta VMDATAL
    rep #$30
    .a16
    .i16
    inx
    cpx #FONT_SIZE
    bne @upright

    ; Inverse-video copy at word $0800 (tile $100), for the AccuracyCoin cursor / page-header
    ; highlight bar. A background layer treats palette colour 0 as transparent, so a swapped-colour
    ; palette cannot draw a solid bar -- the inverse GLYPH does: complementing bitplane 0 fills the
    ; cell with ink (colour 1) as the bar and punches the character through as colour 0, the grey
    ; backdrop. $0800 is clear of the upright font ($0000-$03FF) and the menu tilemap
    ; (MAP_BASE $0400-$07FF); the menu reloads the font before drawing, so a scene's use of $0800
    ; for its offset-per-tile map does not have to survive into the menu.
    ldx #$0800
    stx VMADDL
    ldx #$0000
@inverse:
    sep #$20
    .a8
    lda f:font_data,x
    eor #$FF
    sta VMDATAL
    rep #$30
    .a16
    .i16
    inx
    cpx #FONT_SIZE
    bne @inverse
    rts
.endproc

; ---------------------------------------------------------------------------------------------
; Upload the code font a second time as 4bpp OBJ (sprite) tiles at name base word $4000, indexed by
; ASCII so an OAM tile is just the glyph's character. A 4bpp tile is 16 words: 8 of (bitplane-0 = a
; font row, bitplane-1 = 0) then 8 zero words (bitplanes 2/3). Only colour 1 is ever set, so a sprite
; renders in OBJ palette 0's light-blue -- the multi-behaviour success-code overlay on the skyline.
; ---------------------------------------------------------------------------------------------
.proc load_sprite_font
    sep #$20
    .a8
    lda #$80
    sta VMAIN                   ; advance after the high byte -> a 16-bit store writes one word
    rep #$30
    .a16
    .i16
    ldx #$4000
    stx VMADDL
    ldx #$0000                  ; source byte index into font_data
@glyph:
    ldy #$0000                  ; 8 rows -> 8 words, low = row, high = 0
@row:
    sep #$20
    .a8
    lda f:font_data,x
    rep #$30
    .a16
    .i16
    and #$00FF
    sta VMDATAL                 ; word = font row (bitplane 0), high byte (bitplane 1) = 0
    inx
    iny
    cpy #$0008
    bne @row
    lda #$0000                  ; 8 zero words (bitplanes 2 and 3)
    ldy #$0000
@zero:
    sta VMDATAL
    iny
    cpy #$0008
    bne @zero
    cpx #FONT_SIZE
    bne @glyph
    rts
.endproc

; Point OBSEL at the sprite font and clear OAM. Called once at boot after load_sprite_font.
.proc init_oam
    sep #$20
    .a8
    lda #SKY_OBJ_OBSEL
    sta OBSEL
    rep #$30
    .a16
    .i16
    jsr clear_oam
    rts
.endproc

; Move every sprite off-screen (Y = $F0, below the 224-line display) so no stale sprite shows. The
; low table is 128 * 4 bytes then the 32-byte high table, written straight through ($2104 latches a
; byte pair, so each sprite is two word-writes). Leaves V_OAM_N = 0 for the next skyline pass.
.proc clear_oam
    sep #$20
    .a8
    rep #$10
    .i16
    stz OAMADDL
    stz OAMADDH
    ldx #$0000
@lo:
    stz OAMDATA                 ; X = 0
    lda #$F0
    sta OAMDATA                 ; Y = $F0 (off-screen)
    stz OAMDATA                 ; tile = 0
    stz OAMDATA                 ; attr = 0
    inx
    cpx #128
    bne @lo
    ldx #$0000
@hi:
    stz OAMDATA                 ; high table (X bit 8 = 0, size = 8x8)
    inx
    cpx #32
    bne @hi
    lda #$00
    sta f:V_OAM_N               ; STZ has no long-addressing form
    rep #$30
    .a16
    .i16
    rts
.endproc

; If the brick's test is a multi-behaviour pass (verdict odd and > 1, i.e. a pass carrying a variant
; code), add a light-blue OBJ sprite of the code glyph over the brick. V_TMP = battery index, V_SKY_J
; = height above baseline, V_MENU_I = column. Sprites are appended at V_OAM_N (capped at 127).
.proc maybe_add_variant_sprite
    rep #$30
    .a16
    .i16
    lda f:V_TMP                 ; battery index
    tax
    sep #$20
    .a8
    lda f:R_STATUS,x
    rep #$30
    .a16
    .i16
    and #$00FF
    cmp #$00FF
    beq @no                     ; skip verdict -> not a pass
    bit #$0001
    beq @no                     ; even -> fail / not-run -> no sprite
    lsr
    beq @no                     ; code 0 -> a plain pass, no variant to show
    ; A = code (1-127) -> the glyph's ASCII.
    cmp #10
    bcc @dig
    clc
    adc #('A' - 10)
    bra @tile
@dig:
    clc
    adc #'0'
@tile:
    sta f:V_TMP2                ; tile = code glyph ASCII (reuse V_TMP2)
    ; OAM word address = V_OAM_N * 2 (each sprite is two words in the low table).
    lda f:V_OAM_N
    and #$00FF
    asl
    sep #$20
    .a8
    sta OAMADDL
    stz OAMADDH                 ; low table, X < 256
    ; X pixel = (SKY_X0 + V_MENU_I) * 8
    rep #$30
    .a16
    .i16
    lda f:V_MENU_I
    clc
    adc #SKY_X0
    asl
    asl
    asl
    sep #$20
    .a8
    sta OAMDATA                 ; X
    ; Y pixel = (SKY_TOP + V_SKY_J) * 8
    rep #$30
    .a16
    .i16
    lda #SKY_TOP
    clc
    adc f:V_SKY_J
    asl
    asl
    asl
    sep #$20
    .a8
    sta OAMDATA                 ; Y
    lda f:V_TMP2
    sta OAMDATA                 ; tile
    lda #SKY_OBJ_ATTR
    sta OAMDATA                 ; attr
    ; advance the slot, capped at 127
    rep #$30
    .a16
    .i16
    lda f:V_OAM_N
    and #$00FF
    cmp #127
    bcs @no
    inc a
    sep #$20
    .a8
    sta f:V_OAM_N
    rep #$30
    .a16
    .i16
@no:
    rep #$30
    .a16
    .i16
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
    ; Forced blank is the battery's standing precondition (see the module header in tests/ppu.rs):
    ; the OAM/VRAM/CGRAM ports are only architecturally accessible outside active display, and
    ; nearly every test reads or writes them. The cold-boot caller sets INIDISP = $8F before it
    ; jsr's here, but a Select restart re-enters at `restart_entry` with the display ON (the menu was
    ; on screen) and `init_registers` never touches INIDISP — so a restart would run the whole
    ; battery mid-render. That is invisible under the batch compositor but not under the per-dot PPU:
    ; an OAM access during active display is correctly redirected to the sprite-evaluation index, so
    ; C1.01-C1.05/C1.03b read back the render address rather than the CPU-written value and fail.
    ; Establish forced blank here, at the battery entry, so every entry path shares the precondition.
    sep #$20
    .a8
    lda #$8F
    sta INIDISP
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

    ; Honour a user skip mark (menu B). The bitmap is all-zero on the harness's cold boot, so this is
    ; a no-op there and the scored run is byte-identical; only a human toggling B before a Select
    ; restart sets a bit. A marked test records SKIP and is tallied as skipped without running.
    lda f:V_TEST_IDX
    jsr user_skip_check         ; Z set = not skipped
    beq @not_skipped
    sep #$20
    .a8
    lda #VERDICT_SKIP
    sta f:V_TEST_RESULT
    rep #$30
    .a16
    .i16
    jmp test_restore_target     ; record SKIP + tally, then advance -- no dispatch
@not_skipped:

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

    ; A menu re-run records the one verdict and stops here: no re-tally (the counters describe the
    ; whole battery and the verdict is deterministic anyway), no advance to the next index. Control
    ; returns to the menu, which redraws.
    lda f:V_MENU_MODE
    beq @batch
    lda #$00
    sta f:V_MENU_MODE           ; STZ has no long addressing mode
    lda #$01
    sta f:V_DIRTY
    rep #$30
    .a16
    .i16
    jmp main_loop
@batch:

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
    ; Disarm interrupts unconditionally. A test that enables NMI or IRQ and then FAILS leaves
    ; through the failure path, so leaving the disable to the test body would arm interrupts for
    ; every test after it — one failure becoming a cascade, and results depending on test order.
    ; This is the only reason the NMI vector can be wired at all.
    ;
    ; 8-bit deliberately: `stz $4200` with a 16-bit accumulator writes TWO bytes and would clobber
    ; $4201 (WRIO), whose bit 7 gates the H/V counter latch that C3.07 and the wide timing
    ; instrument depend on.
    sep #$20
    .a8
    stz NMITIMEN
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

; X = VRAM word address. V_STR_PTR = 24-bit pointer to a length-prefixed ASCII string, anywhere.
;
; It reads through `lda [V_STR_PTR],y` rather than `lda a:0,y` because the catalog holding every
; test's name moved out of bank $00, and a data-bank-relative read can only see one bank at a time.
; Making the pointer explicit means the routine works for the header strings in bank $00 and the
; names in the catalog's bank without knowing which is which -- and it keeps working if either
; moves again.
.proc draw_str
    sep #$20
    .a8
    stz VMAIN                   ; advance after the low byte only
    rep #$30
    .a16
    .i16
    stx VMADDL
    ldy #$0000
    sep #$20
    .a8
    lda [V_STR_PTR],y           ; length, at offset 0
    beq @done
    rep #$20
    .a16
    and #$00FF
    tax                         ; X = remaining characters
    sep #$20
    .a8
@loop:
    iny
    lda [V_STR_PTR],y
    sta VMDATAL
    rep #$10
    .i16
    dex
    beq @done
    sep #$20
    .a8
    bra @loop
@done:
    ; Return with A and the index registers 16-bit. draw_screen calls this between `lda f:R_PASSED`
    ; loads and `draw_dec3`, all of which want 16 bits; leaving A 8-bit made every tally read only
    ; the low byte of its counter and, further on, made draw_list compute its row bounds from an
    ; 8-bit accumulator and bail before the first row.
    rep #$30
    .a16
    .i16
    rts
.endproc

; Point V_STR_PTR at a bank-$00 string whose 16-bit address is in Y. Leaves A 16-bit.
;
; The two header strings live in RODATA and the catalog's names do not, so only this path can
; assume a bank -- and it says so rather than leaving it implicit in `draw_str`.
.proc str_ptr_bank0
    rep #$20
    .a16
    tya
    sta f:V_STR_PTR
    sep #$20
    .a8
    lda #$00
    sta f:V_STR_PTR+2
    rep #$20
    .a16
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

; Draw the whole paged menu (AccuracyCoin-style): a centered page-group title, the "PAGE x / N"
; line, and the current page's tests double-spaced with a coloured verdict label each. The selected
; row (or the page header, when V_CURSOR == $FFFF) is drawn in the inverse-video font as the cursor.
.proc draw_screen
    rep #$30
    .a16
    .i16
    jsr clear_oam               ; the menu has no sprites; hide any the skyline left in OAM
    ; Clear the whole visible tilemap first: pages have different lengths, so last page's tail rows
    ; must not linger under a shorter page.
    ldx #MAP_BASE
    ldy #(28 * SCREEN_COLS)
    jsr blank_rows
    jsr draw_title
    jsr draw_page_header
    jsr draw_list
    rts
.endproc

; Centered page-group title at MENU_TITLE_ROW, always white.
.proc draw_title
    rep #$30
    .a16
    .i16
    lda f:V_PAGE
    asl
    tax
    lda f:_page_names,x         ; 16-bit address of this page's length-prefixed name
    sta f:V_STR_PTR
    sep #$20
    .a8
    lda #^_page_names           ; the CATALOG segment's bank
    sta f:V_STR_PTR+2
    lda #ATTR_WHITE
    sta f:V_ATTR
    rep #$30
    .a16
    .i16
    ; col = (SCREEN_COLS - length) / 2
    ldy #$0000
    sep #$20
    .a8
    lda [V_STR_PTR],y           ; length byte
    rep #$20
    .a16
    and #$00FF
    sta f:V_TMP
    lda #SCREEN_COLS
    sec
    sbc f:V_TMP
    lsr
    clc
    adc #(MAP_BASE + MENU_TITLE_ROW * SCREEN_COLS)
    tax
    jsr attr_set_addr
    jsr attr_str
    rts
.endproc

; "PAGE x / N" at MENU_PAGE_ROW. Inverse when the page header is the selected line (V_CURSOR==$FFFF).
.proc draw_page_header
    rep #$30
    .a16
    .i16
    sep #$20
    .a8
    lda #ATTR_WHITE
    sta f:V_ATTR
    rep #$30
    .a16
    .i16
    lda f:V_CURSOR
    cmp #$FFFF
    bne :+
    sep #$20
    .a8
    lda #(ATTR_WHITE | ATTR_INVERSE)
    sta f:V_ATTR
    rep #$30
    .a16
    .i16
:
    ldx #(MAP_BASE + MENU_PAGE_ROW * SCREEN_COLS + 2)
    jsr attr_set_addr
    ldy #str_page
    jsr str_ptr_bank0
    jsr attr_str
    lda f:V_PAGE
    inc a                       ; 1-based for display
    jsr attr_dec3
    ldy #str_slash
    jsr str_ptr_bank0
    jsr attr_str
    lda f:_page_count
    jsr attr_dec3
    rts
.endproc

; X = VRAM word address. Point the attributed streamers at it, in full-word (VMAIN=$80) mode.
.proc attr_set_addr
    sep #$20
    .a8
    lda #$80
    sta VMAIN                   ; advance after the HIGH byte -> a 16-bit store writes one tile word
    rep #$30
    .a16
    .i16
    stx VMADDL
    rts
.endproc

; Stream a length-prefixed string (V_STR_PTR) as full tilemap words, high byte = V_ATTR. VMADDL and
; VMAIN=$80 must be set by the caller (attr_set_addr); auto-increment leaves VMADDL past the string,
; so several attr_str / attr_dec3 calls chain into consecutive columns.
.proc attr_str
    rep #$30
    .a16
    .i16
    ldy #$0000
    sep #$20
    .a8
    lda [V_STR_PTR],y           ; length
    beq @done
    rep #$20
    .a16
    and #$00FF
    tax                         ; X = remaining characters
@loop:
    iny
    sep #$20
    .a8
    lda [V_STR_PTR],y
    sta VMDATAL
    lda f:V_ATTR
    sta VMDATAH
    rep #$20
    .a16
    dex
    bne @loop
@done:
    rep #$30
    .a16
    .i16
    rts
.endproc

; A (16-bit) = value 0-999 -> three digits streamed as full words with V_ATTR. Address pre-set.
.proc attr_dec3
    rep #$30
    .a16
    .i16
    sta f:V_TMP
    ; hundreds
    ldx #$0000
@h:
    cmp #100
    bcc @hd
    sec
    sbc #100
    inx
    bra @h
@hd:
    sta f:V_TMP                 ; remainder after hundreds
    txa
    clc
    adc #'0'
    jsr attr_putdigit
    ; tens
    lda f:V_TMP
    ldx #$0000
@t:
    cmp #10
    bcc @td
    sec
    sbc #10
    inx
    bra @t
@td:
    sta f:V_TMP                 ; units
    txa
    clc
    adc #'0'
    jsr attr_putdigit
    ; units
    lda f:V_TMP
    clc
    adc #'0'
    jsr attr_putdigit
    rts
.endproc

; A (low byte) = character -> one tilemap word (low = char, high = V_ATTR). Address pre-set.
.proc attr_putdigit
    sep #$20
    .a8
    sta VMDATAL
    lda f:V_ATTR
    sta VMDATAH
    rep #$30
    .a16
    .i16
    rts
.endproc

; A (16-bit, 0-35) = value -> one hex glyph (0-9, A-Z) streamed with V_ATTR. Address pre-set.
.proc attr_hexdigit
    rep #$30
    .a16
    .i16
    cmp #10
    bcc @dig
    clc
    adc #('A' - 10)
    bra @put
@dig:
    clc
    adc #'0'
@put:
    jsr attr_putdigit
    rts
.endproc

; Blank a run of tilemap words to spaces. X = first VRAM word address, Y = count.
;
; The rendered scenes leave their own tilemap behind and the drawing routines write only as many
; columns as their text needs, so without this the leftovers show through: a tail of the last
; scene's canvas after every name, and whole rows of it past the end of the list. `@blank_rest` in
; `draw_list` is an alias for `@done` and has never blanked anything.
.proc blank_rows
    sep #$20
    .a8
    lda #$80
    sta VMAIN                   ; advance after the HIGH byte: full words are written here
    rep #$30
    .a16
    .i16
    stx VMADDL
    ; A full tilemap word, not just the tile index. The high byte carries the palette, priority and
    ; flip bits, and every drawing routine after this writes only VMDATAL -- so whatever palette is
    ; left in the high byte is the palette the text is drawn in. The rendered scenes leave their
    ; own there, per-tile, which is why the results list came out in two colours alternating
    ; character by character. Blanking to a *word* of $0020 sets tile = space and palette = 0 for
    ; the whole area, and the low-byte-only writes that follow inherit it.
    lda #$0020
@lp:
    sta VMDATAL                 ; 16-bit store: low byte then high byte, then VMADDL advances
    dey
    bne @lp
    sep #$20
    .a8
    stz VMAIN                   ; back to low-byte-only advance for the text routines
    rep #$30
    .a16
    .i16
    rts
.endproc

; Draw the current page's tests, each as a coloured verdict label + name, double-spaced from
; MENU_LIST_ROW. The battery index of a row is _page_tests[_page_off[V_PAGE] + row].
.proc draw_list
    rep #$30
    .a16
    .i16
    ; Base entry index of this page (from _page_off, a word table).
    lda f:V_PAGE
    asl
    tax
    lda f:_page_off,x
    sta f:V_MENU_BASE
    ; Test count on this page (from _page_len, a byte table).
    lda f:V_PAGE
    tax
    sep #$20
    .a8
    lda f:_page_len,x
    rep #$30
    .a16
    .i16
    and #$00FF
    sta f:V_MENU_CNT
    lda #$0000
    sta f:V_MENU_I
@row:
    lda f:V_MENU_I
    cmp f:V_MENU_CNT
    bcc :+
    jmp @done
:
    ; Battery index for this row -> V_TMP2 (a word into _page_tests).
    lda f:V_MENU_BASE
    clc
    adc f:V_MENU_I
    asl
    tax
    lda f:_page_tests,x
    sta f:V_TMP2
    jsr draw_test_row
    lda f:V_MENU_I
    inc a
    sta f:V_MENU_I
    jmp @row
@done:
    rts
.endproc

; Draw one test row. V_MENU_I = row within page (also the cursor comparison), V_TMP2 = battery index.
; Classifies R_STATUS + _test_flags into a label + palette, then draws the label, an optional FAIL
; code glyph, and the name -- all inverse-video when this row is the selected one.
; The classification stays uniformly 16-bit (A and index), reading the two byte tables in brief
; `.a8` windows that immediately re-widen. Keeping one width throughout avoids the trap where a block
; that ends `.a16` leaves the *next* fall-through / branch label assembled 16-bit while the CPU runs
; it 8-bit -- the `cmp #imm` then emits a 3-byte immediate the CPU decodes as 2, and the stray byte
; runs as an opcode (`.smart` does not save straight-line code; this file tracks width by hand).
.proc draw_test_row
    rep #$30
    .a16
    .i16
    lda f:V_TMP2                ; battery index
    tax                         ; X indexes the byte tables
    ; flags byte -> A, masked to 16 bits.
    sep #$20
    .a8
    lda f:_test_flags,x
    rep #$30
    .a16
    .i16
    and #$0001                  ; does it score at all?
    beq @draw
    ; status byte -> A, masked to 16 bits.
    sep #$20
    .a8
    lda f:R_STATUS,x
    rep #$30
    .a16
    .i16
    and #$00FF
    cmp #VERDICT_NOTRUN
    beq @test
    cmp #$00FF                  ; VERDICT_SKIP, masked to a byte
    beq @skip
    bit #$0001                  ; bit 0 set = pass (possibly with a variant code); else fail
    bne @pass
    ; fail: code = byte >> 1.
    lsr
    sta f:V_TMP                 ; FAIL code (0-127)
    ldy #str_fail
    jsr str_ptr_bank0
    lda #ATTR_RED
    sta f:V_ATTR                ; 16-bit store also zeroes V_VIEW (its high byte) -- 0 is the menu view
    bra @have_label
@draw:
    ; Non-scoring (Contested / Novel): informational, drawn "DRAW" in white -- like AccuracyCoin.
    ldy #str_draw
    jsr str_ptr_bank0
    lda #ATTR_WHITE
    sta f:V_ATTR
    lda #$FFFF
    sta f:V_TMP                 ; no FAIL code
    bra @have_label
@test:
    ldy #str_test
    jsr str_ptr_bank0
    lda #ATTR_WHITE
    sta f:V_ATTR
    lda #$FFFF
    sta f:V_TMP
    bra @have_label
@skip:
    ldy #str_skip
    jsr str_ptr_bank0
    lda #ATTR_BLACK
    sta f:V_ATTR
    lda #$FFFF
    sta f:V_TMP
    bra @have_label
@pass:
    ldy #str_pass
    jsr str_ptr_bank0
    lda #ATTR_BLUE
    sta f:V_ATTR
    lda #$FFFF
    sta f:V_TMP
@have_label:
    rep #$30
    .a16
    .i16
    ; Highlight (inverse-video) when this row is selected.
    lda f:V_MENU_I
    cmp f:V_CURSOR
    bne @notsel
    sep #$20
    .a8
    lda f:V_ATTR
    ora #ATTR_INVERSE
    sta f:V_ATTR
    rep #$30
    .a16
    .i16
@notsel:
    ; Label at MENU_LABEL_COL.
    jsr row_vram
    txa
    clc
    adc #MENU_LABEL_COL
    tax
    jsr attr_set_addr
    jsr attr_str
    ; FAIL code glyph, if any, at MENU_CODE_COL (same palette / highlight).
    lda f:V_TMP
    cmp #$FFFF
    beq @no_code
    jsr row_vram
    txa
    clc
    adc #MENU_CODE_COL
    tax
    jsr attr_set_addr
    lda f:V_TMP
    jsr attr_hexdigit
@no_code:
    ; Name at MENU_NAME_COL, white (inverse when selected).
    sep #$20
    .a8
    lda #ATTR_WHITE
    sta f:V_ATTR
    rep #$30
    .a16
    .i16
    lda f:V_MENU_I
    cmp f:V_CURSOR
    bne :+
    sep #$20
    .a8
    lda #(ATTR_WHITE | ATTR_INVERSE)
    sta f:V_ATTR
    rep #$30
    .a16
    .i16
:
    lda f:V_TMP2
    asl
    tax
    lda f:_test_names,x
    sta f:V_STR_PTR
    sep #$20
    .a8
    lda #^_test_names
    sta f:V_STR_PTR+2
    rep #$30
    .a16
    .i16
    jsr row_vram
    txa
    clc
    adc #MENU_NAME_COL
    tax
    jsr attr_set_addr
    jsr attr_str
    rts
.endproc

; X = tilemap word address of the current row's column 0: MAP_BASE + (MENU_LIST_ROW +
; V_MENU_I * MENU_LIST_STEP) * SCREEN_COLS.
.proc row_vram
    rep #$30
    .a16
    .i16
    lda f:V_MENU_I
    asl                         ; * MENU_LIST_STEP (2)
    clc
    adc #MENU_LIST_ROW
    asl
    asl
    asl
    asl
    asl                         ; * SCREEN_COLS (32)
    clc
    adc #MAP_BASE
    tax
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
; The battery's own screen is deliberately austere: two CGRAM entries (black and green) and a
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

.ifndef HIROM_BUILD
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
.endif

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
    ; V_VIEW selects which input set is live: 0 = the paged test menu, 1 = the skyline results.
    lda f:V_VIEW
    and #$00FF
    beq @menu_input
    jmp @skyline_input

@menu_input:
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
    lda f:V_PAD_NEW
    bit #PAD_LEFT
    beq :+
    jsr page_prev
:
    lda f:V_PAD_NEW
    bit #PAD_RIGHT
    beq :+
    jsr page_next
:
    ; A runs the highlighted test (there is nothing to run on the page-header line, V_CURSOR==$FFFF).
    ; run_selected dispatches exactly as the batch does but with V_MENU_MODE set, so the verdict path
    ; records R_STATUS and jumps back to main_loop rather than tallying and advancing. It does not
    ; return: the test exits through test_restore to test_restore_target, which jmps back here.
    lda f:V_PAD_NEW
    bit #PAD_A
    beq @no_a
    lda f:V_CURSOR
    cmp #$FFFF
    beq @no_a
    jsr run_selected            ; never returns here
@no_a:
    ; B toggles the user-skip mark on the highlighted test (nothing to toggle on the page header).
    ; Like Start, B is inside the input contract, so a contract-holding host never edges it; a human
    ; toggles freely. The mark shows immediately (SKIP / back to TEST) and is applied on a restart.
    lda f:V_PAD_NEW
    bit #PAD_B
    beq @no_b
    lda f:V_CURSOR
    cmp #$FFFF
    beq @no_b
    jsr toggle_skip
@no_b:
    ; Start switches to the skyline results view -- AccuracyCoin's "run tests -> results" (the battery
    ; already ran at boot, so this shows the existing verdicts rather than re-running). A host holding
    ; the input contract (B + Start + X + R) holds Start continuously, but the menu acts on
    ; freshly-pressed edges and the first-frame edge is primed away in the menu setup.
    lda f:V_PAD_NEW
    bit #PAD_START
    beq @no_start
    jsr enter_skyline
@no_start:
    ; Select restarts the whole battery from restart_entry (after the power-on capture).
    lda f:V_PAD_NEW
    bit #PAD_SELECT
    beq @no_select
    jmp do_restart              ; never returns
@no_select:
    jmp @redraw

@skyline_input:
    ; Left/Right page the skyline across its screens; Start returns to the menu; Select restarts.
    lda f:V_PAD_NEW
    bit #PAD_LEFT
    beq :+
    jsr sky_prev
:
    lda f:V_PAD_NEW
    bit #PAD_RIGHT
    beq :+
    jsr sky_next
:
    lda f:V_PAD_NEW
    bit #PAD_START
    beq :+
    jsr enter_menu
:
    lda f:V_PAD_NEW
    bit #PAD_SELECT
    beq @redraw
    jmp do_restart              ; never returns

@redraw:
    lda f:V_DIRTY
    and #$00FF
    bne @do_redraw
    jmp @frame                  ; long: the handlers above pushed @frame out of branch range
@do_redraw:
    ; A whole-screen redraw under forced blank (far more VRAM traffic than a vblank alone covers; a
    ; write during active display is dropped). draw_current dispatches on V_VIEW.
    sep #$20
    .a8
    lda #$00
    sta f:V_DIRTY               ; STZ has no long addressing mode
    lda #$80
    sta INIDISP
    rep #$30
    .a16
    .i16
    jsr draw_current
    sep #$20
    .a8
    lda #$0F
    sta INIDISP
    rep #$30
    .a16
    .i16
    jmp @frame
.endproc

; Redraw whichever view is active.
.proc draw_current
    rep #$30
    .a16
    .i16
    lda f:V_VIEW
    and #$00FF
    bne @sky
    jsr draw_screen
    rts
@sky:
    jsr draw_skyline
    rts
.endproc

; Restart the whole battery from restart_entry. Resets the stack first (the menu has been jsr-ing
; around) and does not return. Shared by the menu and skyline Select handlers.
.proc do_restart
    sei
    sep #$20
    .a8
    lda #$01
    sta f:V_RESTARTED           ; tell the power-on-dependent tests this is not a power-on
    rep #$30
    .a16
    .i16
    ldx #$1FFF
    txs
    jmp reset::restart_entry
.endproc

; Enter the skyline results view from the menu: reset to its first screen and request a redraw.
.proc enter_skyline
    rep #$30
    .a16
    .i16
    sep #$20
    .a8
    lda #$01
    sta f:V_VIEW
    lda #$00
    sta f:V_SKY_SCREEN
    lda #$01
    sta f:V_DIRTY
    rep #$30
    .a16
    .i16
    rts
.endproc

; Return to the paged menu from the skyline.
.proc enter_menu
    rep #$30
    .a16
    .i16
    sep #$20
    .a8
    lda #$00
    sta f:V_VIEW
    lda #$01
    sta f:V_DIRTY
    rep #$30
    .a16
    .i16
    rts
.endproc

; Number of skyline screens = ceil(_page_count / SKY_COLS) -> A (16-bit).
.proc sky_screen_count
    rep #$30
    .a16
    .i16
    lda f:_page_count
    clc
    adc #(SKY_COLS - 1)
    ; divide by SKY_COLS (30) by repeated subtraction -- called rarely, columns are few.
    ldx #$0000
@d:
    cmp #SKY_COLS
    bcc @done
    sec
    sbc #SKY_COLS
    inx
    bra @d
@done:
    txa
    rts
.endproc

; Right: next skyline screen, wrapping.
.proc sky_next
    rep #$30
    .a16
    .i16
    lda f:V_SKY_SCREEN
    and #$00FF
    inc a
    pha
    jsr sky_screen_count
    sta f:V_TMP                 ; screen count
    pla
    cmp f:V_TMP
    bcc :+
    lda #$0000                  ; wrap past the last screen
:
    sep #$20
    .a8
    sta f:V_SKY_SCREEN
    lda #$01
    sta f:V_DIRTY
    rep #$30
    .a16
    .i16
    rts
.endproc

; Left: previous skyline screen, wrapping.
.proc sky_prev
    rep #$30
    .a16
    .i16
    lda f:V_SKY_SCREEN
    and #$00FF
    bne :+
    jsr sky_screen_count        ; wrap: screen 0 -> last
:
    dec a
    sep #$20
    .a8
    sta f:V_SKY_SCREEN
    lda #$01
    sta f:V_DIRTY
    rep #$30
    .a16
    .i16
    rts
.endproc

; A = number of tests on the current page (from _page_len, a byte table).
.proc page_count_a
    rep #$30
    .a16
    .i16
    lda f:V_PAGE
    tax
    sep #$20
    .a8
    lda f:_page_len,x
    rep #$30
    .a16
    .i16
    and #$00FF
    rts
.endproc

; Up: header <- last test <- ... <- first test <- header (wraps through the page header line).
.proc cursor_up
    rep #$30
    .a16
    .i16
    lda f:V_CURSOR
    cmp #$FFFF
    beq @from_header
    cmp #$0000
    beq @to_header
    dec a
    sta f:V_CURSOR
    bra @dirty
@to_header:
    lda #$FFFF
    sta f:V_CURSOR
    bra @dirty
@from_header:
    jsr page_count_a
    dec a                       ; last test row
    sta f:V_CURSOR
@dirty:
    sep #$20
    .a8
    lda #$01
    sta f:V_DIRTY
    rep #$30
    .a16
    .i16
    rts
.endproc

; Down: header -> first test -> ... -> last test -> header.
.proc cursor_down
    rep #$30
    .a16
    .i16
    lda f:V_CURSOR
    cmp #$FFFF
    beq @from_header
    inc a
    sta f:V_TMP                 ; candidate row
    jsr page_count_a            ; A = count
    cmp f:V_TMP
    bcc @to_header              ; count < candidate -> past the end
    beq @to_header              ; count == candidate -> past the end
    lda f:V_TMP
    sta f:V_CURSOR
    bra @dirty
@to_header:
    lda #$FFFF
    sta f:V_CURSOR
    bra @dirty
@from_header:
    lda #$0000
    sta f:V_CURSOR
@dirty:
    sep #$20
    .a8
    lda #$01
    sta f:V_DIRTY
    rep #$30
    .a16
    .i16
    rts
.endproc

; Right: next page, wrapping; land on the new page's header line.
.proc page_next
    rep #$30
    .a16
    .i16
    lda f:V_PAGE
    inc a
    cmp f:_page_count
    bcc :+
    lda #$0000                  ; wrap past the last page
:
    sta f:V_PAGE
    lda #$FFFF
    sta f:V_CURSOR
    sep #$20
    .a8
    lda #$01
    sta f:V_DIRTY
    rep #$30
    .a16
    .i16
    rts
.endproc

; Left: previous page, wrapping; land on the new page's header line.
.proc page_prev
    rep #$30
    .a16
    .i16
    lda f:V_PAGE
    bne :+
    lda f:_page_count           ; wrap: page 0 -> last page
:
    dec a
    sta f:V_PAGE
    lda #$FFFF
    sta f:V_CURSOR
    sep #$20
    .a8
    lda #$01
    sta f:V_DIRTY
    rep #$30
    .a16
    .i16
    rts
.endproc

; Run the one highlighted test. battery index = _page_tests[_page_off[V_PAGE] + V_CURSOR]. Sets
; V_MENU_MODE so the verdict path records R_STATUS and jumps back to main_loop. Does not return.
.proc run_selected
    rep #$30
    .a16
    .i16
    lda f:V_PAGE
    asl
    tax
    lda f:_page_off,x
    clc
    adc f:V_CURSOR
    asl
    tax
    lda f:_page_tests,x
    sta f:V_TEST_IDX
    sep #$20
    .a8
    lda #$01
    sta f:V_MENU_MODE
    lda #VERDICT_NOTRUN
    sta f:V_TEST_RESULT
    rep #$30
    .a16
    .i16
    tsc
    sta f:V_SAVED_S
    lda f:V_TEST_IDX
    sta a:V_DISPATCH_TMP
    asl
    clc
    adc a:V_DISPATCH_TMP
    tax
    lda f:_test_entries,x
    sta a:V_DISPATCH
    sep #$20
    .a8
    lda f:_test_entries+2,x
    sta a:V_DISPATCH+2
    rep #$30
    .a16
    .i16
    lda #$0000
    tcd
    phk
    plb
    jsr call_indirect           ; never returns here; lands at test_restore_target
    rts                         ; unreachable, keeps the assembler's flow analysis happy
.endproc

; A (16-bit) = test index -> A (16-bit) = the test's user-skip bit (0 = not skipped; Z reflects it).
; The bitmap byte is idx>>3, the bit is 1 << (idx & 7). Clobbers X.
.proc user_skip_check
    rep #$30
    .a16
    .i16
    sta f:V_TMP                 ; idx
    lsr
    lsr
    lsr                         ; byte index = idx >> 3
    tax
    sep #$20
    .a8
    lda f:V_USER_SKIP,x
    rep #$30
    .a16
    .i16
    and #$00FF
    sta f:V_TMP2                ; the bitmap byte
    ; mask = 1 << (idx & 7)
    lda f:V_TMP
    and #$0007
    tax
    lda #$0001
@sh:
    cpx #$0000
    beq @sh_done
    asl
    dex
    bra @sh
@sh_done:
    and f:V_TMP2                ; mask & byte -> Z clear if the test is skipped
    rts
.endproc

; Toggle the user-skip mark on the highlighted test and reflect it in R_STATUS immediately (SKIP when
; set, back to NOT-RUN when cleared). The battery honours the mark on the next Select restart.
.proc toggle_skip
    rep #$30
    .a16
    .i16
    ; battery index = _page_tests[_page_off[V_PAGE] + V_CURSOR]
    lda f:V_PAGE
    asl
    tax
    lda f:_page_off,x
    clc
    adc f:V_CURSOR
    asl
    tax
    lda f:_page_tests,x
    sta f:V_TMP2                ; battery index
    ; mask = 1 << (idx & 7) -> V_TMP; byte index -> X
    lda f:V_TMP2
    and #$0007
    tax
    lda #$0001
@sh:
    cpx #$0000
    beq @sh_done
    asl
    dex
    bra @sh
@sh_done:
    sta f:V_TMP                 ; mask
    lda f:V_TMP2
    lsr
    lsr
    lsr
    tax                         ; byte index
    ; byte ^= mask
    sep #$20
    .a8
    lda f:V_USER_SKIP,x
    eor f:V_TMP
    sta f:V_USER_SKIP,x
    and f:V_TMP                 ; new bit state
    beq @cleared
    ; now marked -> show SKIP
    rep #$30
    .a16
    .i16
    lda f:V_TMP2
    tax
    sep #$20
    .a8
    lda #VERDICT_SKIP
    sta f:R_STATUS,x
    bra @done
@cleared:
    rep #$30
    .a16
    .i16
    lda f:V_TMP2
    tax
    sep #$20
    .a8
    lda #VERDICT_NOTRUN
    sta f:R_STATUS,x
@done:
    lda #$01
    sta f:V_DIRTY
    rep #$30
    .a16
    .i16
    rts
.endproc

; ---------------------------------------------------------------------------------------------
; Skyline results view (AccuracyCoin's "city"): one column per page, one brick per test, stacked up
; from a common baseline so ragged page lengths read as a skyline. A brick is the solid inverse-font
; block in the verdict's palette (blue pass / black skip / white non-scoring); a FAIL brick is the
; red code glyph instead. Columns page across screens with Left/Right; a "TESTS PASSED: N / M"
; footer and a "RESULTS  SCR x/y" header frame it.
; ---------------------------------------------------------------------------------------------
.proc draw_skyline
    rep #$30
    .a16
    .i16
    jsr clear_oam               ; hide last frame's sprites and reset the slot counter to 0
    ldx #MAP_BASE
    ldy #(28 * SCREEN_COLS)
    jsr blank_rows
    jsr draw_sky_header
    jsr draw_sky_columns        ; draws the bricks and appends the variant-code sprites
    jsr draw_sky_footer
    rts
.endproc

; "RESULTS" and the screen indicator at row 0.
.proc draw_sky_header
    rep #$30
    .a16
    .i16
    sep #$20
    .a8
    lda #ATTR_WHITE
    sta f:V_ATTR
    rep #$30
    .a16
    .i16
    ldx #(MAP_BASE + SKY_X0)
    jsr attr_set_addr
    ldy #str_results
    jsr str_ptr_bank0
    jsr attr_str
    ; "SCR x / y" at column 20.
    ldx #(MAP_BASE + 20)
    jsr attr_set_addr
    ldy #str_scr
    jsr str_ptr_bank0
    jsr attr_str
    lda f:V_SKY_SCREEN
    and #$00FF
    inc a                       ; 1-based
    jsr attr_dec3
    ldy #str_slash
    jsr str_ptr_bank0
    jsr attr_str
    jsr sky_screen_count
    jsr attr_dec3
    rts
.endproc

; "TESTS PASSED: N / M" at row 26, where M is the scored total (passed + failed).
.proc draw_sky_footer
    rep #$30
    .a16
    .i16
    sep #$20
    .a8
    lda #ATTR_WHITE
    sta f:V_ATTR
    rep #$30
    .a16
    .i16
    ldx #(MAP_BASE + 26 * SCREEN_COLS + SKY_X0)
    jsr attr_set_addr
    ldy #str_passed
    jsr str_ptr_bank0
    jsr attr_str
    lda f:R_PASSED
    jsr attr_dec3
    ldy #str_slash
    jsr str_ptr_bank0
    jsr attr_str
    lda f:R_PASSED
    clc
    adc f:R_FAILED
    jsr attr_dec3
    rts
.endproc

; Draw every column of the current screen: page = V_SKY_SCREEN*SKY_COLS + i, one brick per test.
.proc draw_sky_columns
    rep #$30
    .a16
    .i16
    lda #$0000
    sta f:V_MENU_I              ; column index i
@col:
    lda f:V_MENU_I
    cmp #SKY_COLS
    bcc :+
    jmp @done
:
    ; page = V_SKY_SCREEN * SKY_COLS + i  (SKY_COLS = 30 = 32 - 2)
    lda f:V_SKY_SCREEN
    and #$00FF
    sta f:V_TMP
    asl
    asl
    asl
    asl
    asl                         ; screen * 32
    sta f:V_TMP2
    lda f:V_TMP
    asl                         ; screen * 2
    sta f:V_TMP
    lda f:V_TMP2
    sec
    sbc f:V_TMP                 ; screen * 30
    clc
    adc f:V_MENU_I
    sta f:V_TMP2                ; page p
    cmp f:_page_count
    bcc :+
    jmp @done                   ; no more pages on this screen
:
    lda f:V_TMP2
    asl
    tax
    lda f:_page_off,x
    sta f:V_MENU_BASE
    lda f:V_TMP2
    tax
    sep #$20
    .a8
    lda f:_page_len,x
    rep #$30
    .a16
    .i16
    and #$00FF
    sta f:V_MENU_CNT
    jsr draw_col_label          ; the page number, vertically above this column (V_TMP2 = page)
    lda #$0000
    sta f:V_SKY_J               ; test-in-page (also the brick's height above the baseline)
@brick:
    lda f:V_SKY_J
    cmp f:V_MENU_CNT
    bcc :+
    jmp @nextcol
:
    lda f:V_MENU_BASE
    clc
    adc f:V_SKY_J
    asl
    tax
    lda f:_page_tests,x
    sta f:V_TMP                 ; battery index of this brick's test
    jsr draw_brick
    lda f:V_SKY_J
    inc a
    sta f:V_SKY_J
    jmp @brick
@nextcol:
    lda f:V_MENU_I
    inc a
    sta f:V_MENU_I
    jmp @col
@done:
    rts
.endproc

; The page number (1-based) above the column at SKY_X0 + V_MENU_I, the AccuracyCoin way: the leading
; (first significant) digit on the top row (SKY_LABEL_ROW), the units digit on the row below ONLY for a
; two-digit page. A single-digit page (1-9) shows just its digit on top with a blank below -- no
; leading zero, so the label rows do not fill with a busy run of 0s. V_TMP2 = page (0-based). Clobbers
; V_TMP / V_TMP2.
.proc draw_col_label
    rep #$30
    .a16
    .i16
    sep #$20
    .a8
    lda #ATTR_WHITE
    sta f:V_ATTR
    rep #$30
    .a16
    .i16
    lda f:V_TMP2
    inc a                       ; 1-based page number
    ldx #$0000                  ; tens digit
@t:
    cmp #10
    bcc @td
    sec
    sbc #10
    inx
    bra @t
@td:
    ; A = units value, X = tens value.
    cpx #$0000
    bne @twodigit
    ; single digit: the digit itself on top, blank below (no leading zero).
    clc
    adc #'0'
    sta f:V_TMP2                ; top char = the single digit
    lda #' '
    sta f:V_TMP                 ; bottom char = blank
    bra @draw
@twodigit:
    clc
    adc #'0'
    sta f:V_TMP                 ; bottom char = units
    txa
    clc
    adc #'0'
    sta f:V_TMP2                ; top char = tens (page no longer needed)
@draw:
    lda #(MAP_BASE + SKY_LABEL_ROW * SCREEN_COLS + SKY_X0)
    clc
    adc f:V_MENU_I
    tax
    lda f:V_TMP2                ; top
    jsr put_tile_at
    lda #(MAP_BASE + (SKY_LABEL_ROW + 1) * SCREEN_COLS + SKY_X0)
    clc
    adc f:V_MENU_I
    tax
    lda f:V_TMP                 ; bottom
    jsr put_tile_at
    rts
.endproc

; Draw one brick. V_TMP = battery index, V_SKY_J = height above baseline, V_MENU_I = column. Chooses
; a solid block (pass/skip/non-scoring) or the red code glyph (fail); a not-run test draws nothing.
.proc draw_brick
    rep #$30
    .a16
    .i16
    lda f:V_TMP
    tax
    sep #$20
    .a8
    lda f:_test_flags,x
    rep #$30
    .a16
    .i16
    and #$0001
    beq @white                  ; non-scoring -> white block
    sep #$20
    .a8
    lda f:R_STATUS,x
    rep #$30
    .a16
    .i16
    and #$00FF
    cmp #VERDICT_NOTRUN
    bne :+
    rts                         ; not run -> no brick
:
    cmp #$00FF
    beq @black                  ; skip -> black block
    bit #$0001
    bne @blue                   ; pass -> blue block
    ; fail -> red code glyph (code = byte >> 1, rendered 0-9 / A-Z).
    lsr
    cmp #10
    bcc :+
    clc
    adc #('A' - 10)
    bra @red_tile
:
    clc
    adc #'0'
@red_tile:
    sta f:V_TMP2                ; tile low = the code glyph (upright font)
    sep #$20
    .a8
    lda #ATTR_RED
    sta f:V_ATTR
    rep #$30
    .a16
    .i16
    bra @emit
@blue:
    lda #SKY_BLOCK
    sta f:V_TMP2
    sep #$20
    .a8
    lda #(ATTR_BLUE | ATTR_INVERSE)
    sta f:V_ATTR
    rep #$30
    .a16
    .i16
    bra @emit
@black:
    lda #SKY_BLOCK
    sta f:V_TMP2
    sep #$20
    .a8
    lda #(ATTR_BLACK | ATTR_INVERSE)
    sta f:V_ATTR
    rep #$30
    .a16
    .i16
    bra @emit
@white:
    lda #SKY_BLOCK
    sta f:V_TMP2
    sep #$20
    .a8
    lda #(ATTR_WHITE | ATTR_INVERSE)
    sta f:V_ATTR
    rep #$30
    .a16
    .i16
@emit:
    ; vram = MAP_BASE + (SKY_TOP + V_SKY_J) * SCREEN_COLS + (SKY_X0 + V_MENU_I). Top-down: test 0 at
    ; SKY_TOP, later tests below it, like AccuracyCoin.
    lda #SKY_TOP
    clc
    adc f:V_SKY_J
    asl
    asl
    asl
    asl
    asl                         ; * SCREEN_COLS (32)
    clc
    adc #(MAP_BASE + SKY_X0)
    clc
    adc f:V_MENU_I
    tax
    lda f:V_TMP2                ; tile low byte
    jsr put_tile_at
    jsr maybe_add_variant_sprite ; light-blue code sprite over a multi-behaviour pass
    rts
.endproc

; X = tilemap word address, A (low byte) = tile index low, V_ATTR = high byte. Writes one full word.
.proc put_tile_at
    pha
    sep #$20
    .a8
    lda #$80
    sta VMAIN                   ; advance after the high byte -> a 16-bit store writes one tile word
    rep #$30
    .a16
    .i16
    stx VMADDL
    pla
    sep #$20
    .a8
    sta VMDATAL
    lda f:V_ATTR
    sta VMDATAH
    rep #$30
    .a16
    .i16
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
; apu_upload_2block — a TWO-block IPL upload, for E4.06.
;
; The single-block `apu_upload` above exercises only two of the IPL's port-1-at-a-boundary
; decisions: a NON-zero at the opening kick (begin the first transfer) and a zero at the close
; (jump to the entry point). The one boundary it never reaches is a NON-zero close: the IPL then
; re-enters its transfer loop against the address left in ports 2/3 rather than jumping — a genuine
; branch ($FFF9 `BNE` in the boot ROM) that no other test drives. This proc drives it: block A is
; transferred, closed with a NON-zero port-1 and block B's destination in ports 2/3, block B is then
; transferred to that second destination, and only the final close carries the zero that jumps.
;
; Block A reuses V_APU_SRC / V_APU_BANK / V_APU_LEN / V_APU_DEST; block B is V_APU_SRC2 (same bank) /
; V_APU_LEN2 / V_APU_DEST2; V_APU_ENTRY is the final jump target. Every wait is bounded exactly as
; `apu_upload`'s are, so a core that mishandles the continue stands the test down (SKIP) rather than
; hanging the battery. V_APU_STAGE names the step that gave up.
.export apu_upload_2block_far
.proc apu_upload_2block_far
    jsr apu_upload_2block
    rtl
.endproc

.export apu_upload_2block
.proc apu_upload_2block
    php
    rep #$30
    .a16
    .i16
    phb
    phd
    phk
    plb
    lda #$0000
    tcd

    ; Block A's source pointer into direct page.
    lda f:V_APU_SRC
    sta z:V_APU_PTR
    sep #$20
    .a8
    lda f:V_APU_BANK
    sta z:V_APU_PTR + 2

    ; --- wait for the IPL's $AA/$BB ready announcement ---
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
    jmp @fail
  :
    bra @ready

@open:
    ; --- open block A's transfer at V_APU_DEST ---
    rep #$30
    .a16
    .i16
    lda #$0002
    sta f:V_APU_STAGE
    lda f:V_APU_DEST
    sta APUIO2
    sep #$20
    .a8
    lda #$01                    ; non-zero: begin a transfer
    sta APUIO1
    lda #$CC
    sta APUIO0
    rep #$10
    .i16
    ldx #$0000
@kickA:
    sep #$20
    .a8
    cmp APUIO0
    beq @loopA
    rep #$10
    .i16
    inx
    bne :+
    jmp @fail
  :
    bra @kickA

@loopA:
    ; --- block A, one byte per round trip ---
    rep #$30
    .a16
    .i16
    lda #$0003
    sta f:V_APU_STAGE
    ldy #$0000
@byteA:
    tya
    cmp f:V_APU_LEN
    beq @contA
    sep #$20
    .a8
    lda [V_APU_PTR],y
    sta APUIO1
    tya
    sta APUIO0
    rep #$10
    .i16
    ldx #$0000
@echoA:
    sep #$20
    .a8
    cmp APUIO0
    beq @nextA
    rep #$10
    .i16
    inx
    bne :+
    jmp @fail
  :
    bra @echoA
@nextA:
    rep #$30
    .a16
    .i16
    iny
    bra @byteA

@contA:
    ; --- close block A with a NON-zero port 1: continue to block B, do NOT jump ---
    ;
    ; Ports 2/3 carry block B's destination; port 1 non-zero is exactly what tells the IPL this is a
    ; new block rather than an entry point. The final counter is Y + 2, the same off-by-one
    ; `apu_upload`'s close uses (one for the byte never sent, one because the IPL reads it as "next
    ; expected"). After this echo the boot ROM sits in its transfer loop waiting for counter 0.
    rep #$30
    .a16
    .i16
    lda #$0004
    sta f:V_APU_STAGE
    lda f:V_APU_DEST2
    sta APUIO2
    sep #$20
    .a8
    lda #$01                    ; NON-zero: continue with another block
    sta APUIO1
    tya
    clc
    adc #$02
    sta APUIO0
    rep #$10
    .i16
    ldx #$0000
@echoCont:
    sep #$20
    .a8
    cmp APUIO0
    beq @ptrB
    rep #$10
    .i16
    inx
    bne :+
    jmp @fail
  :
    bra @echoCont

@ptrB:
    ; Block B's source pointer into direct page (block B shares block A's bank).
    rep #$30
    .a16
    .i16
    lda f:V_APU_SRC2
    sta z:V_APU_PTR
    sep #$20
    .a8
    lda f:V_APU_BANK
    sta z:V_APU_PTR + 2

    rep #$30
    .a16
    .i16
    lda #$0005
    sta f:V_APU_STAGE
    ldy #$0000
@byteB:
    tya
    cmp f:V_APU_LEN2
    beq @doneB
    sep #$20
    .a8
    lda [V_APU_PTR],y
    sta APUIO1
    tya
    sta APUIO0
    rep #$10
    .i16
    ldx #$0000
@echoB:
    sep #$20
    .a8
    cmp APUIO0
    beq @nextB
    rep #$10
    .i16
    inx
    bne :+
    jmp @fail
  :
    bra @echoB
@nextB:
    rep #$30
    .a16
    .i16
    iny
    bra @byteB

@doneB:
    ; --- final close with a ZERO port 1: jump to V_APU_ENTRY ---
    rep #$30
    .a16
    .i16
    lda #$0006
    sta f:V_APU_STAGE
    lda f:V_APU_ENTRY
    sta APUIO2
    sep #$20
    .a8
    stz APUIO1                  ; zero: jump instead of continuing
    tya
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
    jmp @fail
  :
    bra @final

@ok:
    rep #$30
    .a16
    .i16
    lda #$0000
    sta f:V_APU_STAGE
    pld
    plb
    plp
    clc
    rts

@fail:
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

; The NMI trampoline, shared by the native ($FFEA) and emulation ($FFFA) vectors.
;
; Unlike BRK/COP/IRQ this one is unreachable in the ordinary battery: `init_registers` clears
; NMITIMEN and VBlank is detected by polling $4212 bit 7, so no NMI is ever generated. It exists
; for the `WAI`/`STP` and mid-block-move rows, which cannot be written without a real interrupt.
;
; The two modes share a trampoline because, unlike COP, nothing distinguishes them: hardware routes
; native and emulation NMI to different vectors but the handler's job is identical, and the
; emulation-mode COP split (`V_COP_VEC_E`) exists only because a core fetching the wrong table there
; was otherwise unobservable. If a future row needs to tell the two NMI tables apart, split this the
; same way rather than reusing it.
; Far-callable wrappers for the runtime helpers an out-of-bank test body needs.
;
; A test body outside bank $00 cannot reach the runtime with `jsr`: the same 16-bit address in
; another bank is not a subroutine, it is whatever bytes are there. The generator rejects that at
; build time, which is how group C's `jsr frame_step` and group D's `jsr hv_begin` were caught the
; moment those groups were first moved. These wrappers are the way through -- `jsl` in, `rtl` out.
;
; Deliberately thin: the extra `jsr`/`rtl` costs the same on every call, so a differential timing
; measurement that brackets it cancels the overhead exactly, which is what C7's hv_begin/hv_end
; pair relies on.
.export wait_vblank_far
.proc wait_vblank_far
    jsr wait_vblank
    rtl
.endproc

.export frame_step_far
.proc frame_step_far
    jsr frame_step
    rtl
.endproc

.export hv_begin_far
.proc hv_begin_far
    jsr hv_begin
    rtl
.endproc

.export hv_end_far
.proc hv_end_far
    jsr hv_end
    rtl
.endproc

.export nmi_trampoline
.proc nmi_trampoline
    jmp (V_NMI_VEC)
.endproc

; Bank-probe landing stubs for the `(a,X)` indirect-jump rows.
;
; Their addresses sit in the reserved bytes of the bank $00 and bank $01 signature blocks, so which
; one runs says which bank the pointer was fetched from. Each records its identity and returns
; through a RAM vector the test installs — the point being that BOTH answers return, rather than the
; wrong one jumping into whatever ROM happens to be at that offset. A test whose wrong answer
; crashes reports nothing, which is how the withdrawn A4.06/A4.08 managed to assert nothing at all.
;
; `JMP (a,X)` does not change PBR, so both stubs execute in bank $00 whichever pointer was fetched;
; only the pointer FETCH crosses banks, which is the behaviour under test.
; The explicit `.a8` is load-bearing, not tidiness. These stubs are reached by an indirect jump
; from a test whose accumulator width ca65 cannot see, and the assembler's width belief is
; file-global: without it, `lda #$00` assembles as a THREE-byte 16-bit immediate, the CPU (8-bit
; after the SEP) takes two, and the trailing $00 executes as BRK. That is exactly how A6.10 failed
; when it was written, and it is how the first version of these stubs failed too — the jump landed
; correctly and the probe never recorded anything.
.export bankprobe_0
.proc bankprobe_0
    sep #$20
    .a8
    lda #$00
    sta f:V_BANKPROBE
    jml [V_BANKPROBE_RET]
.endproc

.export bankprobe_1
.proc bankprobe_1
    sep #$20
    .a8
    lda #$01
    sta f:V_BANKPROBE
    jml [V_BANKPROBE_RET]
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

; The emulation-mode COP vector ($FFF4) gets its own trampoline and its own pointer, and that
; separation is the point rather than tidiness: while both modes' COP vectors shared a trampoline, a
; core that fetched the NATIVE vector while in emulation mode landed in the same handler and no test
; could see the difference.
;
; BRK deliberately does NOT get the same treatment. In emulation, $FFFE is shared between IRQ and
; BRK — the hardware conflates them — so a pointer behind it could not mean "the BRK handler"
; unambiguously, and splitting it would only invent a distinction the machine does not have. Both
; BRK vectors therefore stay on `brk_trampoline` and `V_BRK_VEC`.
.export cop_trampoline_e
.proc cop_trampoline_e
    jmp (V_COP_VEC_E)
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
; ---------------------------------------------------------------------------------------------
; WIDE measurement — the same instrument, counting dots since the top of the FRAME rather than
; within a scanline, so a measured span may cross line boundaries.
;
; Why it exists: `hv_read_raw` returns the 9-bit H counter, which wraps every scanline. A span
; longer than a line comes back SMALL, and two overrunning spans come back looking nearly equal —
; nothing in the result says "out of range". Measured through the narrow pair, an MVN of 8 bytes and
; one of 32 read 326 and 327 dots, and 64 NOPs read *less* than 32 of them.
;
; The single thing that makes this work, and the thing a first attempt got wrong: **H and V must
; come from ONE latch.** `$2137` latches both counters together; `$213C` and `$213D` then read them
; out through independent flipflops (`C3.07`). Latching twice — once for H, once for V — samples two
; different instants, and when a line boundary falls between them the composite jumps by a whole
; line. That version produced numbers as non-monotonic as the narrow instrument's.
;
; The delta is computed as `(V1 - V0) * 341 + (H1 - H0)`, and the multiply is a countdown loop
; rather than the hardware multiplier: the line count is a handful, the loop is outside the measured
; region, and using `$4202`/`$4203` here would clobber a unit that `B5.05` and `D2` tests read.
;
; **Accuracy.** Not every line is 341 dots — NTSC has a short one at V=240, PAL a long one — so a
; span crossing N lines can be off by up to N dots. For the two- or three-line spans this exists to
; measure that is inside the tolerance the timing tests already use. It is not suitable for spans
; approaching a field, and it cannot measure one that wraps V back to zero.
.proc hv_latch_wide
    php
    sep #$20
    .a8
    rep #$10
    .i16
    ; LONG addressing throughout, so the latch does not depend on DBR. This is not defensive
    ; tidiness: `MVN` leaves DBR = its destination bank, so a measurement wrapped around a block
    ; move reaches `hv_end_wide` with DBR = $7E, and absolute `lda $213F` then reads WRAM instead of
    ; the PPU. The counters come back as whatever bytes happened to be there, the line-countdown
    ; loop below runs thousands of times, and the result is a five-digit number that looks like
    ; data. That is what an 8-byte MVN reported as "11464 dots".
    ;
    ; **`hv_read_raw`, the narrow instrument, still uses absolute addressing and still carries this
    ; hazard.** No current test trips it — none of them measures across an instruction that moves
    ; DBR — but anything that does must restore DBR before `hv_end`, or use this path instead.
    lda f:$00213F               ; reset both read flipflops
    lda f:$002137               ; SLHV: ONE latch, capturing H and V together
    lda f:$00213C               ; OPHCT low 8
    sta f:V_HW_H
    lda f:$00213C               ; OPHCT second read: bit 0 is counter bit 8
    and #$01
    sta f:V_HW_H+1
    lda f:$00213D               ; OPVCT low 8, from the same latch
    sta f:V_HW_V
    lda f:$00213D               ; OPVCT second read: bit 0 is counter bit 8
    and #$01
    sta f:V_HW_V+1
    plp
    rts
.endproc

.export hv_begin_wide
.proc hv_begin_wide
    php
    rep #$30
    .a16
    .i16
    cld                         ; the delta arithmetic in hv_end_wide must not run in BCD
    pha                         ; the caller's A survives: MVN takes its byte count in A, and a
                                ; measurement that clobbers it measures a different instruction
    ; Start the span in the FIRST HALF of the field, and not for tidiness. V wrapping back to zero
    ; mid-measurement makes `V1 - V0` hugely negative, the line-countdown loop below runs about
    ; sixty-five thousand times, and the result is a five-digit number that looks like data. That is
    ; exactly what an MVN of 64 bytes reported before this wait existed.
    ;
    ; V < 150 also keeps the whole span inside active display, where every line really is 341 dots —
    ; NTSC's short line is at V=240 and PAL's long one later still — so the approximation the delta
    ; arithmetic makes is exact here rather than merely close.
@wait:
    jsr hv_latch_wide
    rep #$30
    .a16
    .i16
    lda f:V_HW_V
    cmp #150
    bcs @wait
    lda f:V_HW_H
    sta f:V_H0
    lda f:V_HW_V
    sta f:V_HW_V0
    pla
    plp
    rts
.endproc

.export hv_end_wide
.proc hv_end_wide
    php
    rep #$30
    .a16
    .i16
    cld
    pha                         ; as in hv_begin_wide: the caller's A is not ours to spend
    jsr hv_latch_wide
    rep #$30
    .a16
    .i16
    lda f:V_HW_V
    sec
    sbc f:V_HW_V0
    ; A span that wraps the field, or simply runs too long, must not come back looking like data —
    ; that failure mode is the entire reason this routine exists. V1 < V0 means V wrapped past the
    ; end of the field (the borrow leaves carry clear); more than MAX_SPAN_LINES means the
    ; line-length approximation has accumulated past usefulness. Either way, return $FFFF, which no
    ; real span can produce and which fails any range assertion loudly.
    bcc @overrun
    cmp #MAX_SPAN_LINES + 1
    bcs @overrun
    sta f:V_HW_DV               ; lines crossed
    lda f:V_HW_H
    sec
    sbc f:V_H0
    sta f:V_H1                  ; H delta, which may be negative before the lines are added
@lines:
    lda f:V_HW_DV
    beq @done
    dec a
    sta f:V_HW_DV
    lda f:V_H1
    clc
    adc #341
    sta f:V_H1
    bra @lines
@done:
    pla
    plp
    rts
@overrun:
    lda #$FFFF
    sta f:V_H1
    pla
    plp
    rts
.endproc

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

; AccuracyCoin-style paged-menu strings, each length-prefixed (drawn via str_ptr_bank0 + attr_str).
str_page:
    .byte 5
    .byte "PAGE "
str_slash:
    .byte 3
    .byte " / "
str_test:
    .byte 4
    .byte "TEST"
str_pass:
    .byte 4
    .byte "PASS"
str_fail:
    .byte 4
    .byte "FAIL"
str_skip:
    .byte 4
    .byte "SKIP"
str_draw:
    .byte 4
    .byte "DRAW"
str_results:
    .byte 7
    .byte "RESULTS"
str_scr:
    .byte 4
    .byte "SCR "
str_passed:
    .byte 14
    .byte "TESTS PASSED: "
