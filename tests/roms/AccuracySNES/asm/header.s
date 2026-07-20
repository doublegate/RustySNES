; AccuracySNES — cartridge header and interrupt vectors.
;
; The checksum/complement words are placeholders; `accuracysnes-gen` patches them after linking,
; because ld65 cannot compute them. They matter: RustySNES's own header detector scores
; `checksum ^ complement == $FFFF` as its strongest signal (+8 of a possible 15,
; `crates/rustysnes-cart/src/header.rs`), and some flash carts refuse to boot without a valid one.

.p816

.import reset
.import irq_stub
.import brk_trampoline
.import cop_trampoline

.segment "HEADER"
    ; --- extended header, $FFB0-$FFBF ---
    .byte "00"                  ; $FFB0  maker code
    .byte "ACSN"                ; $FFB2  game code
    .res   6, $00               ; $FFB6  reserved
    .byte $00                   ; $FFBC  expansion flash size
    .byte $00                   ; $FFBD  expansion RAM size
    .byte $00                   ; $FFBE  special version
    .byte $00                   ; $FFBF  chipset subtype

    ; --- core header, $FFC0-$FFDF ---
title:
    .byte "ACCURACYSNES"        ; $FFC0  title, space-padded to exactly 21 bytes
    .assert * - title <= 21, error, "cartridge title exceeds 21 bytes"
    .res   title + 21 - *, $20

    .byte $20                   ; $FFD5  map mode: LoROM, SlowROM
    .byte $00                   ; $FFD6  cartridge type: ROM only, no coprocessor, no SRAM
    .byte $07                   ; $FFD7  ROM size: log2(131072) - 10 = 7  (128 KiB)
    .byte $00                   ; $FFD8  RAM size: none
    .byte $01                   ; $FFD9  country: NTSC / US
    .byte $33                   ; $FFDA  developer ID: use the $FFB0 maker code
    .byte $00                   ; $FFDB  ROM version
    .word $0000                 ; $FFDC  checksum complement (patched post-link)
    .word $0000                 ; $FFDE  checksum            (patched post-link)

.segment "VECTORS"
    ; --- native mode, $FFE0-$FFEF ---
    .word $0000, $0000          ; $FFE0, $FFE2  unused
    .addr cop_trampoline        ; $FFE4  COP
    .addr brk_trampoline        ; $FFE6  BRK
    .addr irq_stub              ; $FFE8  ABORT
    .addr irq_stub              ; $FFEA  NMI
    .word $0000                 ; $FFEC  unused (the CPU always boots in emulation mode)
    .addr irq_stub              ; $FFEE  IRQ

    ; --- emulation mode, $FFF0-$FFFF ---
    .word $0000, $0000          ; $FFF0, $FFF2  unused
    .addr cop_trampoline        ; $FFF4  COP
    .word $0000                 ; $FFF6  unused (BRK shares the IRQ vector in emulation)
    .addr irq_stub              ; $FFF8  ABORT
    .addr irq_stub              ; $FFFA  NMI
    .addr reset                 ; $FFFC  RESET — the entry point
    .addr brk_trampoline        ; $FFFE  IRQ / BRK
