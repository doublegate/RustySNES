//! Group G — the cartridge itself: header, memory map, and the power-on state (ticket **T-04-G**).
//!
//! The group's dossier scope also covers power-on and reset state, which the battery cannot reach
//! directly: it runs long after reset, through a runtime that has already written most of the
//! registers whose power-on values the dossier enumerates. What makes those assertions reachable is
//! `runtime.s::capture_power_on` plus the two instructions at the very top of `reset` — a snapshot
//! taken *before* `init_registers` puts the machine into a known state, stashed in the capture
//! block at `$7E:E040`. The tests below report out of that snapshot rather than reading the
//! registers themselves, which by then would only describe the runtime.
//!
//! The one group whose subject is not a chip. What it asserts is that the *emulator's* view of the
//! cartridge matches the one every other assertion silently depends on: that the header is where
//! LoROM says it is, that its self-check holds, and that a bank number decodes to the ROM offset
//! the mapping formula gives. Every test in every other group is running out of a ROM addressed
//! through that formula — if it were wrong, the failures would appear anywhere but here.
//!
//! Which is exactly why these are worth writing down rather than assuming. A mapping bug that
//! happens to be self-consistent produces a battery that passes and a commercial ROM that does not
//! boot.

use crate::dsl::{Asm, Kind, Provenance, Test};

/// Every Group G test, in menu order.
#[must_use]
pub fn all() -> Vec<Test> {
    vec![
        g1_02(),
        g1_04(),
        g1_08(),
        g1_10(),
        g1_11(),
        g1_12(),
        g1_14(),
        g1_19(),
    ]
}

/// Neither `$4210` nor `$4211` has its flag set when the machine starts.
///
/// Both are read-once registers: reading clears the flag, so the value they hold at reset is
/// visible exactly once and only to whoever reads first. That is `capture_power_on`, before the
/// runtime's own vblank polling has had a chance to consume either.
///
/// A zero-valued assertion needs a control that a hard-wired zero would fail, and both halves have
/// one, in the battery rather than here. For `$4210`: `B4.03` asserts bit 7 *does* set at the start
/// of vblank and `B4.04` that reading it clears it again. For `$4211`: `B4.12` spins until bit 7
/// sets before it asserts anything, so a core returning a constant `$00` there does not fail that
/// test — it never leaves it. Here the claim is only about the starting value, and it matters
/// because a driver that samples `$4210` before enabling NMI would otherwise see a vblank that
/// never happened.
///
/// **The snapshot is taken tens of cycles after reset**, not at it. If a core began its frame such
/// that vblank started inside that window, bit 7 would legitimately be set — so this is an
/// assertion about reset state on the assumption that reset does not land in vblank, which is what
/// every reference does and what the documented power-on values describe.
fn g1_02() -> Test {
    let mut a = Asm::new();
    a.l("rep #$30");
    a.l("phk");
    a.l("plb");
    a.l("sep #$20");
    a.l("lda f:$7EE047     ; V_PO_RDNMI, the first read of $4210 after reset");
    a.l("and #$80");
    a.assert_a8(
        0x00,
        "$4210 bit 7 (NMI pending) was already set when the machine started",
    );
    a.l("lda f:$7EE048     ; V_PO_TIMEUP, the first read of $4211 after reset");
    a.l("and #$80");
    a.assert_a8(
        0x00,
        "$4211 bit 7 (IRQ pending) was already set when the machine started",
    );
    a.finish(
        "G1.02",
        'G',
        "Reset: $4210/$4211 clear",
        Provenance::Documented("SNESdev Wiki, power-on state; fullsnes"),
        Kind::Scored,
        None,
    )
}

/// The CPU starts in **emulation mode**, and the reset vector is the one LoROM puts at `$00FFFC`.
///
/// The E flag is unreadable — there is no instruction that loads it — and the only way to observe
/// it is to destroy it: `XCE` *exchanges* C and E, so the first `clc`/`xce` of `reset` leaves the
/// old E flag in the carry for exactly one instruction. The runtime captures it there.
///
/// A core that came up native would run this same code correctly (`xce` from native is harmless)
/// and every other test in the battery would still pass, which is why the value has to be caught at
/// the top of reset rather than inferred. What it protects is a real class of game: a title whose
/// reset code assumes 8-bit registers and a page-1 stack, and which crashes on a core that hands it
/// native mode.
///
/// The second half reads the reset vector and follows it: the byte it points at must be the `SEI`
/// that opens `reset`. That does not prove the CPU *used* `$00FFFC` — nothing running on-cart can,
/// since by then it has already happened — but it does prove the word LoROM's map exposes at
/// `$00FFFC` is a working entry point, and the fact that this test is executing is the rest of the
/// argument.
fn g1_04() -> Test {
    let mut a = Asm::new();
    a.l("rep #$30");
    a.l("phk");
    a.l("plb");
    a.l("sep #$20");
    a.l("lda f:$7EE046     ; V_PO_EMU, the carry XCE left at the top of reset");
    a.assert_a8(
        0x01,
        "the CPU was not in emulation mode at reset — XCE's carry said E was already clear",
    );
    a.c("Follow the reset vector: bank $00, and the byte there is reset's opening SEI.");
    a.l("rep #$30");
    a.l("lda f:$00FFFC");
    a.l("tax");
    a.l("sep #$20");
    a.l("lda f:$000000,x");
    a.assert_a8(
        0x78,
        "the word at $00FFFC does not point at code beginning with SEI, so the reset vector is \
         not where LoROM puts it",
    );
    a.finish(
        "G1.04",
        'G',
        "Reset: emulation mode",
        Provenance::Documented("SNESdev Wiki, power-on state; WDC 65C816 datasheet, XCE"),
        Kind::Scored,
        None,
    )
}

/// Reading a **write-only** MMIO register returns the CPU's open bus, not `$00` and not `$FF`.
///
/// Nothing drives the data bus for an address no device answers, so what the CPU latches is
/// whatever was there last — and for a load that is the final byte of the *operand it just
/// fetched*. `$4200` (`NMITIMEN`) is written by every game and readable by none, which makes it the
/// cleanest address to ask the question at.
///
/// The test reads that one register twice, through two addressing modes whose last operand byte
/// differs:
///
/// * `lda a:$4200` fetches `$00`, `$42` — so the bus last held **`$42`**, the address's high byte.
/// * `lda f:$004200` fetches `$00`, `$42`, `$00` — the bank byte comes last, so the bus last held
///   **`$00`**.
///
/// Same register, same cycle position, two different answers. That is what makes the assertion
/// about the *bus* rather than about the register: a core returning a constant (`$00`, `$FF`, or a
/// stale last-read value) gets at most one of the two right, and a core that returns the address
/// high byte because someone hardcoded it fails the long-addressed half.
///
/// **The B bus (`$21xx`) is deliberately not asserted here.** Reads of write-only PPU registers
/// return the *PPU's own* MDR — a latch on the far side of the B bus, updated by writes to PPU
/// registers as well as reads — not the CPU's. The two are separate latches that usually hold
/// different bytes, and which one a given address exposes is a per-chip question this assertion
/// does not settle.
fn g1_08() -> Test {
    let mut a = Asm::new();
    a.l("rep #$30");
    a.l("phk");
    a.l("plb");
    a.l("sep #$20");
    a.c("Absolute: the operand's high byte is the last thing fetched, so the bus holds $42.");
    a.l("lda a:$4200");
    a.assert_a8(
        0x42,
        "reading write-only $4200 with absolute addressing did not return the open-bus value $42 \
         — a core answering $00 or $FF is not modelling the bus at all",
    );
    a.c("Long: the BANK byte comes last, so the same register now reads back as $00.");
    a.l("lda f:$004200");
    a.assert_a8(
        0x00,
        "reading write-only $4200 with long addressing did not return $00, the bank byte the CPU \
         fetched last — so the value returned is fixed rather than whatever was last on the bus",
    );
    a.finish(
        "G1.08",
        'G',
        "Write-only read: openbus",
        Provenance::Documented("SNESdev Wiki, open bus; fullsnes, memory map notes"),
        Kind::Scored,
        None,
    )
}

/// The header's checksum and its complement must XOR to `$FFFF`.
///
/// Two words at `$FFDC` and `$FFDE` that are bitwise inverses. Every emulator uses the pair to
/// *find* the header — a candidate location whose two words do not complement is not a header — so
/// the invariant is load-bearing before a single instruction runs, and a cart that fails it may be
/// detected as the wrong mapping or rejected outright.
///
/// The cart checking its own header is not circular. The generator computes the checksum and the
/// linker places it; this asserts that what the CPU *reads back through the memory map* is that
/// pair, which is a statement about the map as much as about the bytes.
fn g1_10() -> Test {
    let mut a = Asm::new();
    a.l("rep #$30");
    a.l("lda f:$00FFDC        ; the complement");
    a.l("eor f:$00FFDE        ; XOR the checksum");
    a.assert_a16(
        0xFFFF,
        "the header's checksum and complement do not XOR to $FFFF — the pair every emulator uses \
         to recognise a header at all",
    );
    a.finish(
        "G1.10",
        'G',
        "Checksum XOR complement",
        Provenance::Documented("SNESdev Wiki, cartridge header; fullsnes"),
        Kind::Scored,
        None,
    )
}

/// The checksum is the sum of every byte in the image, and the cart adds them all up to prove it.
///
/// A 131,072-byte sum, computed by the cart at run time and compared against the word the generator
/// wrote into the header. What it really tests is the **memory map**: reaching every byte means
/// walking all four banks through `((bank & $7F) << 15) | (addr & $7FFF)`, so a decode that mirrors
/// a bank, drops one, or gets the stride wrong produces a different total. `G1.14` proves the
/// formula on three sample bytes; this proves it on all 131,072.
///
/// The two header fields are neutralised, since a value cannot be part of its own sum. The dossier
/// states the convention as `$FFDC = $FFFF` (the complement) and `$FFDE = $0000` (the checksum);
/// this cart's generator uses the mirror image of that, `$FFDC = $0000` and `$FFDE = $FFFF`. **The
/// two are arithmetically identical** — both contribute `$FF + $FF + $00 + $00` — which is why the
/// correction below is a single `+$1FE` and holds under either reading. Rather than branch inside a
/// loop that runs 131,072 times, the sum is taken over the image as it stands and corrected
/// afterwards: subtract the four bytes actually there, add back what the convention counts.
///
/// **Two things it does not cover**, both worth stating rather than leaving to be assumed:
///
/// * **The algorithm.** The generator computes the checksum the same way, so a shared
///   misunderstanding would agree with itself. What is validated is that an emulator presents the
///   whole image, correctly mapped, to a program that reads it byte by byte.
/// * **The non-power-of-two rule** (largest prefix plus mirrored remainder), which this image
///   cannot exercise: it is exactly 128 KiB. Reaching that needs a second, deliberately odd-sized
///   image, which is a build-system change rather than a test.
fn g1_11() -> Test {
    const SUM: &str = "$7E0110";

    let mut a = Asm::new();
    a.l("rep #$30");
    a.l("lda #$0000");
    a.l(&format!("sta f:{SUM}"));
    a.c("Four banks of 32 KiB, each walked with long indexed addressing so the data bank never");
    a.c("comes into it. Unrolled because the bank is part of the address, not a variable.");
    for bank in 0u8..4 {
        a.l("ldx #$0000");
        a.label(&format!("bank{bank}"));
        a.l("sep #$20");
        a.l(&format!("lda f:${bank:02X}8000,x"));
        a.l("rep #$20");
        a.l("and #$00FF");
        a.l("clc");
        a.l(&format!("adc f:{SUM}"));
        a.l(&format!("sta f:{SUM}"));
        a.l("inx");
        a.l("cpx #$8000");
        a.l(&format!("bne @bank{bank}"));
    }
    a.c("Correct the two header fields out of the total: take away the four bytes that are");
    a.c(
        "actually there and put back the $0000 complement and $FFFF checksum the algorithm counts.",
    );
    a.l("sep #$20");
    a.l("lda f:$00FFDC");
    a.l("rep #$20");
    a.l("and #$00FF");
    a.l(&format!(
        "sta f:{SUM}+2         ; scratch: the running correction"
    ));
    for (addr, label) in [
        ("$00FFDD", "complement high"),
        ("$00FFDE", "checksum low"),
        ("$00FFDF", "checksum high"),
    ] {
        a.l("sep #$20");
        a.l(&format!("lda f:{addr}         ; {label}"));
        a.l("rep #$20");
        a.l("and #$00FF");
        a.l("clc");
        a.l(&format!("adc f:{SUM}+2"));
        a.l(&format!("sta f:{SUM}+2"));
    }
    a.l(&format!("lda f:{SUM}"));
    a.l("sec");
    a.l(&format!(
        "sbc f:{SUM}+2         ; the four header bytes are not part of the sum"
    ));
    a.l("clc");
    a.l("adc #$01FE             ; ...and $FF + $FF + $00 + $00 goes back in their place");
    a.l("eor f:$00FFDE");
    a.assert_a16(
        0x0000,
        "the sum of all 131,072 bytes does not match the checksum in the header — an image that \
         is short, mirrored, or mapped with the wrong bank stride sums differently",
    );
    a.finish(
        "G1.11",
        'G',
        "Checksum over the image",
        Provenance::Documented("SNESdev Wiki, cartridge header checksum; fullsnes"),
        Kind::Scored,
        None,
    )
}

/// A LoROM header sits at `$00:FFC0`, and its map-mode byte says so.
///
/// The header's location *is* the mapping: LoROM puts it at ROM offset `$7FC0`, which the LoROM
/// formula exposes at `$00:FFC0`, while HiROM's lives at `$00:FFC0` of a differently-decoded image.
/// An emulator that guesses the mapping wrong reads a header out of the middle of the code and gets
/// a title of garbage — which is why the map-mode byte is checked here from *inside* a running
/// LoROM image: if the guess were wrong, this test would not be executing.
///
/// `$FFD5 = $20` is LoROM, SlowROM. `$FFD7 = $07` is `log2(131072) - 10`, the 128 KiB this image
/// actually is — a second, independent statement about the same header, and the one that catches a
/// map-mode byte read from the right place in the wrong image.
fn g1_12() -> Test {
    let mut a = Asm::new();
    a.l("sep #$20");
    a.l("lda f:$00FFD5");
    a.assert_a8(
        0x20,
        "the map-mode byte at $FFD5 is not $20 (LoROM, SlowROM), so the header is not where LoROM \
         puts it",
    );
    a.l("lda f:$00FFD7");
    a.assert_a8(
        0x07,
        "the ROM-size byte at $FFD7 is not 7 (128 KiB), so the header was read from the right \
         address of the wrong image",
    );
    a.c("And the title, whose first byte is the one thing a human recognises in a hex dump.");
    a.l("lda f:$00FFC0");
    a.assert_a8(
        b'A',
        "the title does not begin at $FFC0 with ACCURACYSNES's first letter",
    );
    a.finish(
        "G1.12",
        'G',
        "LoROM header location",
        Provenance::Documented("SNESdev Wiki, cartridge header; fullsnes"),
        Kind::Scored,
        None,
    )
}

/// The documented power-on register state, for the parts a cartridge can actually observe.
///
/// `$4200-$420D` are **write-only**, so "what did they hold at reset" cannot be answered by reading
/// them back. The dossier's row lists seven registers; each needs its own indirect channel, and
/// they are split across the battery by which channel that is:
///
/// | register | power-on | how it is observed | where |
/// |---|---|---|---|
/// | `$4202` | `$FF` | multiply by 2 and read `RDMPY` | `B5.05` |
/// | `$4204/05` | `$FFFF` | divide by 2 and read `RDDIV`/`RDMPY` | `B5.05` |
/// | `$4201` | `$FF` | `$4213` (RDIO) reflects the output pins | here |
/// | `$4207/08`, `$4209/0A` | `$1FF` | arm the timers and watch nothing happen | here |
/// | `$4200` | `$00` | — | not separable from the above |
/// | `$420D` | `$00` | access timing | `B1.01` owns the mechanism |
///
/// # Both samples are taken before `init_registers`
///
/// `init_registers` writes the whole `$4200-$420D` block into a known state — which is the right
/// thing for a test battery to do and destroys every value this row is about. So the sampling lives
/// in `capture_power_on`, the same pre-init hook `B5.05` and `D1.11` already use.
///
/// # Why "no IRQ fired" means `$1FF` here, and is not the weak claim it looks like
///
/// `HTIME`/`VTIME` at `$1FF` is 511, past both the 340-dot line and the 262- or 312-line frame, so
/// the comparators can never match. Three frames are watched, which is several hundred thousand
/// comparator evaluations — a core powering the registers up at anything reachable fires almost
/// immediately, and the common wrong answers are all reachable: `$0000` matches on the first line
/// of every frame, and any 8-bit truncation of `$1FF` gives `$FF` = 255, which is a real line and a
/// real dot.
///
/// The one thing this cannot separate is `$4200 = $00`: the probe enables the timer interrupts
/// itself, so a machine that powered up with them already enabled is indistinguishable. That is
/// stated rather than quietly folded in.
fn g1_19() -> Test {
    let mut a = Asm::new();
    a.l("rep #$30");
    a.l("phk");
    a.l("plb");
    a.l("sep #$20");
    a.l("lda f:V_PO_RDIO");
    a.record(91, "G1.01 $4213 (RDIO) as first read after reset");
    a.assert_a8(
        0xFF,
        "$4213 did not read back as $FF at power-on, so $4201's output pins were not left high",
    );
    a.l("lda f:V_PO_TFIRED");
    a.record(
        92,
        "G1.01 whether an IRQ fired on the power-on HTIME/VTIME (0 = none, correct)",
    );
    a.assert_a8(
        0x00,
        "an IRQ fired with HTIME/VTIME left at their power-on values: those are $1FF, which is \
         past the end of both the line and the frame, so no comparator can ever match. $0000 \
         matches every frame and an 8-bit $FF is a real dot and a real line",
    );
    a.finish(
        "G1.19",
        'G',
        "Power-on $4201/timers",
        Provenance::Documented(
            "fullsnes and the SNESdev Wiki power-on table: $4201 = $FF, HTIME and VTIME = $1FF; \
             the other registers in the row are observed by B5.05 and B1.01",
        ),
        Kind::Scored,
        None,
    )
}

/// LoROM decodes a bank as `((bank & $7F) << 15) | (addr & $7FFF)`.
///
/// Two claims in one formula, and this image is built to make both observable. `& $7F` means banks
/// `$80`-`$FF` mirror `$00`-`$7F`, so `$80:8005` and `$00:8005` are the same ROM byte. `<< 15`
/// means each bank maps its own **32 KiB**, so `$01:8005` is a *different* byte — which is only
/// checkable in an image larger than 32 KiB, and is the reason this cart is 128 KiB rather than the
/// minimum.
///
/// Each bank carries its own signature byte at `$xx:8005` for exactly this. A core that decoded
/// banks as 64 KiB, or ignored the mirror, reads the wrong signature and says which mistake it
/// made.
fn g1_14() -> Test {
    let mut a = Asm::new();
    a.l("sep #$20");
    a.l("lda f:$008005");
    a.assert_a8(
        0xA0,
        "bank $00's signature is wrong — the image is not mapped as expected",
    );
    a.c("Bank $01 is a DIFFERENT 32 KiB. A core using a 64 KiB stride reads bank $00's byte here.");
    a.l("lda f:$018005");
    a.assert_a8(
        0xA1,
        "bank $01 did not map its own 32 KiB — reading $A0 means the bank stride is 64 KiB, not 32",
    );
    a.c("And $80 mirrors $00, because the decode masks the top bit off the bank number.");
    a.l("lda f:$808005");
    a.assert_a8(
        0xA0,
        "bank $80 did not mirror bank $00 — the LoROM decode masks the bank with $7F",
    );
    a.finish(
        "G1.14",
        'G',
        "LoROM bank decode",
        Provenance::Documented("SNESdev Wiki, memory map; fullsnes"),
        Kind::Scored,
        None,
    )
}
