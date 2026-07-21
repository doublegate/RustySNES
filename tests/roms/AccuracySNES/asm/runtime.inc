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
V_TMP2          = VAR_BASE + $14      ; second scratch word, for code that needs two at once
; Offset-per-tile entry values handed to `scene_opt_map`: the H row (0) and the V row (1), each
; with an even-column and an odd-column value. Alternating columns is what makes the leftmost-tile
; exemption visible — the shifted columns come out odd-numbered rather than even.
V_OPT_H_EVEN    = VAR_BASE + $16
V_OPT_H_ODD     = VAR_BASE + $18
V_OPT_V_EVEN    = VAR_BASE + $1A
V_OPT_V_ODD     = VAR_BASE + $1C
; APU upload parameters, read by `apu_upload` (see runtime.s).
V_APU_SRC       = VAR_BASE + $1E      ; 16-bit source address of the SPC700 image
V_APU_BANK      = VAR_BASE + $20      ; source bank
V_APU_LEN       = VAR_BASE + $22      ; byte count
V_APU_DEST      = VAR_BASE + $24      ; APU RAM destination
V_APU_ENTRY     = VAR_BASE + $26      ; APU entry point to jump to
; The source pointer is copied into DIRECT PAGE before the transfer: `lda [dp],y` is the only
; addressing mode that reaches an arbitrary bank with an index, and it requires a direct-page
; pointer. $0050 is clear of every other bank-$00 slot the runtime uses.
V_APU_STAGE     = VAR_BASE + $28      ; which handshake step failed, if one did
V_APU_PTR       = $0050

; ---------------------------------------------------------------------------------------------
; Power-on capture block ($E040-$E04F, plus the DMA snapshot at $E060-$E06B).
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
V_PO_EMU        = VAR_BASE + $46      ; 1 if the CPU was in emulation mode at reset, 0 if not (u8)
V_PO_RDNMI      = VAR_BASE + $47      ; $4210 as first read after reset (u8) — bit 7 and version
V_PO_TIMEUP     = VAR_BASE + $48      ; $4211 as first read after reset (u8) — bit 7
; DMA channel 0's registers ($4300-$430B) as they were at power-on, captured before any test can
; write them. The runtime clears $420B/$420C at init but never touches $43xx, so the only thing that
; would destroy these is an earlier test -- which is exactly why they are sampled in
; capture_power_on rather than read from a test body. 12 bytes, $60-$6B; the previous highest
; VAR_BASE offset in use is $50.
V_PO_DMA        = VAR_BASE + $60      ; $4300-$430B at power-on (12 bytes)
; G1.01's remaining power-on registers. $4200-$420D are write-only, so these are the two channels
; that observe them: $4213 (RDIO) reflects $4201's output pins, and an IRQ armed on the untouched
; HTIME/VTIME comparators tests whether those really power on at $1FF -- a value no counter reaches.
; Both must be sampled before init_registers, which overwrites the whole $4200-$420D block.
; G1.03's readable half. The row lists APUIOn, WMDATA, WMADD*, JOYSER, HDMAEN, MDMAEN and JOY1-4 as
; indeterminate at power-on and says to report them, never assert. Most of that list is write-only
; and has nothing to report: WMADD, JOYSER's output, HDMAEN and MDMAEN cannot be read back at all.
; What can be sampled is captured here, before init_registers writes over it.
;
; $2180 is included and its read *increments WMADD* -- which is harmless only because the address it
; increments is itself one of the indeterminate values this row is about, and init_registers sets it
; before anything depends on it.
V_PO_MISC       = VAR_BASE + $70      ; $2140-43, $2180, $4218, $4219 at power-on (7 bytes)
V_PO_RDIO       = VAR_BASE + $6C      ; $4213 as first read after reset (u8)
V_PO_TFIRED     = VAR_BASE + $6D      ; 1 if an IRQ fired on the power-on comparators (u8)

; ---------------------------------------------------------------------------------------------
; Wide (frame-relative) cycle measurement. See `hv_latch_wide` in runtime.s.
;
; The ordinary hv_begin/hv_end pair measures the 9-BIT H COUNTER, which wraps every scanline, so a
; measurement longer than a line comes back small with nothing saying so. These hold H and V from a
; single latch, letting a delta span line boundaries.
V_HW_H          = VAR_BASE + $4A      ; H from the most recent wide latch (u16)
V_HW_V          = VAR_BASE + $4C      ; V from that same latch (u16)
V_HW_V0         = VAR_BASE + $4E      ; V at hv_begin_wide (u16)
V_HW_DV         = VAR_BASE + $50      ; scratch: lines crossed, counted down (u16)
; The longest span hv_end_wide will report. Beyond this the varying line length has accumulated past
; usefulness, and far beyond it V wraps the field and the delta goes negative — so both are refused
; with $FFFF rather than returned as a plausible number. 64 lines is ~21,800 dots, two orders of
; magnitude more than anything measured here.
MAX_SPAN_LINES  = 64

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
; --- host input contract ----------------------------------------------------------------------
;
; Every host that runs this cart holds controller 1 at PAD_CONTRACT for the whole run: the in-repo
; harness, the snes9x libretro driver, and the Mesen2 test-runner script. Group F is otherwise
; almost entirely untestable — with nothing plugged in and no button held, every controller
; observable the cart can reach is $0000, so a test of the read order, of the auto-read signature,
; or of what a disabled auto-read preserves has no value to distinguish from any other. `F1.07` was
; written and withdrawn for exactly that reason before this existed.
;
; $9050 is B + Start + X + R, chosen against three requirements:
;
;   * No d-pad. The post-battery menu scrolls on Up/Down, and although it acts on newly-pressed
;     rather than held bits -- so a held direction would move the cursor once and then stop -- a
;     contract that cannot disturb the menu at all is one less thing to reason about.
;   * Bits in both bytes ($90 high, $50 low), so a host or core that reports only one half, or
;     swaps them, is visibly wrong rather than accidentally right.
;   * Asymmetric under reversal ($9050 reversed bit-wise is $0A09), so reading the shift register
;     LSB-first instead of MSB-first does not produce the same answer.
;
; Holding buttons does not affect any test outside Group F: nothing else reads $4016 or $4218, and
; the two Group F tests that predate the contract mask off the data bits they would see.
PAD_CONTRACT    = $9050

; A *second* controller, held at a different mask, is what `F1.03` (one $4016 write latches both
; ports) needs — with the same buttons on both, "port 2 latched" and "port 2 is echoing port 1"
; are the same sixteen bits. It was built and withdrawn: Mesen2's headless --testrunner has no
; device in port 2, and calling emu.setInput on it clobbers port 1's pending state rather than
; failing, which silently cost every contract-dependent test its input. The harness and the snes9x
; driver both handled it fine. Re-adding needs a way to attach a second controller in Mesen2's
; test runner; until then F1.03 stays uncovered rather than green on two references out of three.

MEAS_BASE       = $7EE200
; 512 slots, widened from 240 and from 192 before that. The channel ends at $7EE600, and the
; results block does not begin until $7EF000, so there is room for 1792 -- the ceiling here is the
; constant, not the memory. Widening it is a change in four places (this file, the generator, the
; harness, the libretro cross-validator) and the generator's collision gate prints an empty free
; list when the channel is full, which is how the last two widenings were prompted.
;
; 240 was reached with 25 slots left against 130-150 tests still to write, so this one was
; deliberate rather than forced. It also had to widen the slot *type* from `u8` to `u16`, which is
; why it is not simply a larger number: 255 was the real ceiling before.
MEAS_SLOTS      = 512

; Rendered-scene pacing. A scene is set up, allowed to SETTLE for whole frames, and only THEN
; publishes its ID for FRAMES frames before clearing it again.
;
; Both numbers are larger than they need to be, deliberately. A host samples WRAM at ITS frame
; boundary, which need not be the boundary the cart's vblank poll sees, so the first and last
; frames of the published window are both at risk of being off by one: snes9x captured scene 1
; with a black band across the top (the frame where forced blank was released) and the in-repo
; harness captured the previous scene entirely. The window is therefore wide enough that hosts can
; agree to take an interior sighting and be well inside the steady state either way.
;
; Widened from 4/4 once the battery grew long enough to shift the phase between the cart's vblank
; polling and a host's frame callback: with only four published frames, "the second sighting"
; occasionally landed on a transition and the gate went red on a different scene each run. An
; intermittently-red gate is worse than a slow one, because it gets ignored.
SCENE_SETTLE    = 6
SCENE_FRAMES    = 8

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
R_SCENE         = RESULTS + $12       ; u8, current rendered-scene ID (0 = none/not started)
R_SCENE_DONE    = RESULTS + $13       ; u8, $5A once every scene has been shown
R_STATUS        = RESULTS + $20       ; one byte per test, catalog-index aligned

R_FORMAT_VERSION = 1
R_DONE_MARK      = $A5

; --- verdict bytes ---------------------------------------------------------------------------
VERDICT_NOTRUN  = $00
VERDICT_PASS    = $01
VERDICT_SKIP    = $FF

APUIO0          = $2140
APUIO1          = $2141
APUIO2          = $2142
APUIO3          = $2143

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
TIMEUP          = $4211
HVBJOY          = $4212
NMITIMEN        = $4200
JOYSER0         = $4016
JOYSER1         = $4017

; JMP [abs] takes its pointer from bank $00, so the dispatch slot cannot live in the
; long-addressed variable block. $00:0040 is inside the low-WRAM mirror and is not touched by
; any test's scratch usage.
;
; THREE CONTIGUOUS BYTES, not two: test bodies no longer all fit in bank $00, so the dispatch
; table holds 24-bit entry points and the jump is JMP [abs] rather than JMP (abs). `JMP [abs]`
; fetches all three bytes from consecutive addresses, so the bank byte cannot be parked somewhere
; convenient — it has to be V_DISPATCH+2. $004C-$004E is the only free triple below V_APU_PTR.
V_DISPATCH      = $004C

; Scratch for the index-times-three arithmetic that indexes the table above. Shares $0040 with
; PAD_X, which is only live while the menu is reading the controller.
V_DISPATCH_TMP  = $0040

; Software-interrupt trampoline targets. The cartridge vectors are fixed at link time, so
; BRK/COP/IRQ point at trampolines that jump through these bank-$00 pointers, letting each test
; install its own handler (AccuracyCoin does the same thing by vectoring into RAM).
V_BRK_VEC       = $0042
V_COP_VEC       = $0044
; The IRQ vector goes through RAM for the same reason BRK and COP do: a test that wants to observe
; interrupt DISPATCH (rather than just the comparator through $4211) has to run its own handler.
; Initialised to `irq_stub` at reset, so a test that never touches it sees the old behaviour.
V_IRQ_VEC       = $004A
; The EMULATION-mode COP vector ($FFF4) goes through its own pointer, and that separation is the
; point rather than tidiness: while both modes' COP vectors shared a trampoline, a core that fetched
; the native vector in emulation mode ran the same handler and nothing could tell. Initialised to
; `irq_stub` at reset, so a COP in a test that has installed no handler returns harmlessly instead
; of jumping into whatever RAM held. BRK is deliberately not split — see runtime.s.
;
; $0054 and not $0052: V_APU_PTR at $0050 is a 24-bit pointer, so it owns $0050-$0052, and the
; first draft of this put the COP vector on top of its bank byte.
V_COP_VEC_E     = $0054

; The NMI vector ($FFEA native / $FFFA emulation), through RAM for the same reason as IRQ. The
; battery never enables NMI — `init_registers` clears NMITIMEN and VBlank is polled from $4212 —
; so this is unreachable until a test opts in; it is initialised to `irq_stub` at reset so a test
; that enables NMI without installing a handler is harmless rather than jumping into RAM.
;
; $0056 is the next free word: the low-RAM map is $0040 PAD_X/V_DISPATCH_TMP, $0042 V_BRK_VEC,
; $0044 V_COP_VEC, $0046 V_H0, $0048 V_H1, $004A V_IRQ_VEC, $004C V_DISPATCH, $0050-$0052
; V_APU_PTR (24-bit, owns three bytes), $0054 V_COP_VEC_E, $0080 PAD_A.
V_NMI_VEC       = $0056

; Bank-probe result and return vector, for the `(a,X)` indirect-jump rows (A4.04/A4.05).
;
; Those tests must tell "the pointer address wrapped inside the program bank" from "it carried into
; the next bank". Low WRAM cannot show that — banks $00-$3F all alias the same 8 KiB below $2000, so
; both candidate addresses are literally the same bytes, which is what made an earlier pair of tests
; vacuous. The discriminating fixture is the per-bank signature block at $xx:8000, whose reserved
; bytes hold the address of `bankprobe_0` in bank $00 and `bankprobe_1` in bank $01. Each stub
; writes its own identity here and returns through V_BANKPROBE_RET, so BOTH outcomes come back
; through the same path and neither answer hangs.
; V_BANKPROBE_RET is a 24-BIT pointer — the stubs return with `jml [V_BANKPROBE_RET]`, so it owns
; $005A-$005C, not just $005A-$005B. Writing that down because the same mistake has been made here
; once already: V_COP_VEC_E was first placed on top of V_APU_PTR's bank byte for exactly this
; reason. The next free word after this block is $005E.
V_BANKPROBE     = $0058
V_BANKPROBE_RET = $005A

; A 24-bit pointer to the length-prefixed string `draw_str` is drawing.
;
; It is 24-bit because the catalog -- which holds every test's name -- lives outside bank $00, and
; a 16-bit pointer plus an implicit bank cannot reach it. `draw_str` reads through `lda [ptr],y`,
; so the caller says which bank it means and the routine no longer cares where strings live.
V_STR_PTR       = $005C

; Cycle-measurement scratch (see hv_sync / hv_h in runtime.s).
V_H0            = $0046
V_H1            = $0048

FONT_SIZE       = 1024                ; 128 glyphs x 8 rows; asserted in font.rs

; --- screen layout ---------------------------------------------------------------------------
MAP_BASE        = $0400               ; BG1 tilemap, VRAM word address
; BG3's tilemap in modes 2/4/6, where it is the offset-per-tile source rather than a layer.
; Placed clear of the font (words $0000-$01FF) and of MAP_BASE's 1024 words.
OPT_MAP_BASE    = $0800
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
    .import irq_stub
    .import hv_begin
    .import hv_end
    .import hv_begin_wide
    .import hv_end_wide
.import wait_vblank
    .import apu_upload
.import frame_step
.import apu_upload_far
    ; Far-callable wrappers, for test bodies placed outside bank $00 (see OUT_OF_BANK in emit.rs).
    .import wait_vblank_far
    .import frame_step_far
    .import hv_begin_far
    .import hv_end_far
.endif
.ifndef CATALOG_IMPL
    .import _test_count
    .import _test_entries
    .import _test_flags
    .import _test_names
.ifndef SCENES_IMPL
.import _scene_count
.import _scene_entries
.endif
.endif
