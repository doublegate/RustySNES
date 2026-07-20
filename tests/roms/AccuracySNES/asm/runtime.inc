; AccuracySNES — shared constants and imports.
;
; Included by both the hand-written runtime and the generated test files.

; --- WRAM map (bank $7E) ---------------------------------------------------------------------
; $0000-$2FFF   test scratch. Tests own this outright and may corrupt any of it.
;               NOTE: $7E:0000-$7E:1FFF is also the bank-$00 low-WRAM mirror, which is where the
;               stack lives — so the runtime stack sits at the very top ($1FFF, growing down)
;               and no test writes above $2400.
; $E000-$E0FF   runtime variables (long-addressed; outside every test's scratch range).
; $F000-$F0FF   the results block the host harness reads.

VAR_BASE        = $7EE000
V_CURSOR        = VAR_BASE + $00      ; selected menu row (u16)
V_SCROLL        = VAR_BASE + $02      ; first visible row (u16)
V_PAD_HELD      = VAR_BASE + $04      ; controller 1, currently held (u16)
V_PAD_LAST      = VAR_BASE + $06      ; previous frame (u16)
V_PAD_NEW       = VAR_BASE + $08      ; newly pressed this frame (u16)
V_SAVED_S       = VAR_BASE + $0A      ; stack pointer captured before a test runs (u16)
V_TEST_IDX      = VAR_BASE + $0C      ; index of the test currently running (u16)
V_TMP           = VAR_BASE + $0E      ; scratch (u16)
V_TEST_RESULT   = VAR_BASE + $10      ; the running test's verdict byte (u8)
V_DIRTY         = VAR_BASE + $12      ; non-zero when the screen needs a redraw (u8)

; ---------------------------------------------------------------------------------------------
; Power-on capture block ($E040-$E04F).
;
; Sampled by `capture_power_on` at the very top of reset, BEFORE `init_registers` runs. This
; exists because `init_registers` deliberately puts every PPU and CPU register into a known state
; — precisely because hardware does not — which means any test that runs in the normal battery is
; measuring the runtime, not the machine. Power-on behaviour has to be read before that, once, and
; stashed for a test to report later.
;
; Several of these registers are WRITE-ONLY, so "read the power-on value" means observing the
; latch through the unit it feeds: writing only the multiplier's second operand reveals the first.
V_PO_MPY        = VAR_BASE + $40      ; RDMPY after $4203=2 with $4202 never written (u16)
V_PO_DIV        = VAR_BASE + $42      ; RDDIV after $4206=2 with $4204/05 never written (u16)
V_PO_DIVREM     = VAR_BASE + $44      ; RDMPY (remainder) from that same divide (u16)

; ---------------------------------------------------------------------------------------------
; Raw measurement channel ($E200-$E2FF) — 128 u16 slots.
;
; A test's verdict is ONE BYTE, which is fine for pass/fail/variant and useless for a measurement:
; a dot count does not fit. Reporting a 32-NOP baseline through the variant code returned "21",
; which is impossible — the true value had wrapped past 256. Any timing diagnostic read back that
; way is unfalsifiable, because a wrapped value and a real one are indistinguishable.
;
; So measurements go here, full width, and the host harness reads them alongside the results block.
; This is also what a 256-opcode cycle sweep (T-04-I) needs: it produces 256 numbers and the status
; array has nowhere to put them.
;
; Slots are assigned per test and documented at the test that owns them; nothing here is
; interpreted on-cart.
MEAS_BASE       = $7EE200
MEAS_SLOTS      = 128

; --- results block ---------------------------------------------------------------------------
RESULTS         = $7EF000
R_MAGIC         = RESULTS + $00       ; "ACSN"
R_VERSION       = RESULTS + $04       ; u16 format version
R_COUNT         = RESULTS + $06       ; u16 number of tests
R_DONE          = RESULTS + $08       ; u8, $A5 once every test has run
R_PASSED        = RESULTS + $0A       ; u16
R_FAILED        = RESULTS + $0C       ; u16
R_SKIPPED       = RESULTS + $0E       ; u16
R_GOLDEN        = RESULTS + $10       ; u16
R_STATUS        = RESULTS + $20       ; one byte per test, catalog-index aligned

R_FORMAT_VERSION = 1
R_DONE_MARK      = $A5

; --- verdict bytes ---------------------------------------------------------------------------
VERDICT_NOTRUN  = $00
VERDICT_PASS    = $01
VERDICT_SKIP    = $FF

; --- PPU / CPU registers we touch ------------------------------------------------------------
INIDISP         = $2100
BGMODE          = $2105
BG1SC           = $2107
BG12NBA         = $210B
BG1HOFS         = $210D
BG1VOFS         = $210E
VMAIN           = $2115
VMADDL          = $2116
VMDATAL         = $2118
CGADD           = $2121
CGDATA          = $2122
TM              = $212C
RDNMI           = $4210
HVBJOY          = $4212
NMITIMEN        = $4200
JOYSER0         = $4016

; JMP (abs) takes a 16-bit pointer in bank $00, so the dispatch slot cannot live in the
; long-addressed variable block. $00:0040 is inside the low-WRAM mirror and is not touched by
; any test's scratch usage.
V_DISPATCH      = $0040

; Software-interrupt trampoline targets. The cartridge vectors are fixed at link time, so
; BRK/COP/IRQ point at trampolines that jump through these bank-$00 pointers, letting each test
; install its own handler (AccuracyCoin does the same thing by vectoring into RAM).
V_BRK_VEC       = $0042
V_COP_VEC       = $0044

; Cycle-measurement scratch (see hv_sync / hv_h in runtime.s).
V_H0            = $0046
V_H1            = $0048

FONT_SIZE       = 1024                ; 128 glyphs x 8 rows; asserted in font.rs

; --- screen layout ---------------------------------------------------------------------------
MAP_BASE        = $0400               ; BG1 tilemap, VRAM word address
SCREEN_COLS     = 32
VISIBLE_ROWS    = 26                  ; rows 2..27 list tests; rows 0-1 are the header

; --- controller bits (manual $4016 read, MSB first) -------------------------------------------
PAD_B           = $8000
PAD_Y           = $4000
PAD_SELECT      = $2000
PAD_START       = $1000
PAD_UP          = $0800
PAD_DOWN        = $0400
PAD_LEFT        = $0200
PAD_RIGHT       = $0100
PAD_A           = $0080
PAD_X           = $0040

; The runtime defines these; every other translation unit imports them.
.ifndef RUNTIME_IMPL
    .import test_restore
    .import hv_begin
    .import hv_end
.import wait_vblank
.endif
.ifndef CATALOG_IMPL
    .import _test_count
    .import _test_entries
    .import _test_flags
    .import _test_names
.endif
