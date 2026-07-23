; AccuracySNES — ExHiROM cartridge header, interrupt vectors, and the two half-signature bytes.
;
; The ExHiROM sibling of header.s / header-exhirom's HiROM cousin. It differs in the map-mode byte
; ($FFD5 = $25) and carries the two landmark bytes G1.16 reads to prove the A23->A22 half-selection:
; EXSIG_LO at ROM $000000 (first 4 MiB, read via $C0:0000) and EXSIG_HI at ROM $400000 (extra 4 MiB,
; read via $40:0000). A core that inverts A23 into ROM bit 22 correctly returns the two distinct
; bytes; one that maps both banks to the same half returns the same byte for both.
;
; The checksum/complement words are placeholders; accuracysnes-gen patches them after linking, at the
; ExHiROM header offset ($40FFDC).

.p816

.import reset
.import irq_stub
.import irq_trampoline
.import nmi_trampoline
.import brk_trampoline
.import cop_trampoline_e
.import cop_trampoline

; The first-half landmark: ROM $000000, reachable through $C0:0000 (and $80:8000).
.segment "EXSIG_LO"
    .byte $A1

; The extra-half landmark: ROM $400000, reachable through $40:0000. A different byte than EXSIG_LO.
.segment "EXSIG_HI"
    .byte $E2

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
    .byte "ACCURACYSNES-EX"     ; $FFC0  title, space-padded to exactly 21 bytes
    .assert * - title <= 21, error, "cartridge title exceeds 21 bytes"
    .res   title + 21 - *, $20

    .byte $25                   ; $FFD5  map mode: ExHiROM, SlowROM
    .byte $00                   ; $FFD6  cartridge type: ROM only
    .byte $0D                   ; $FFD7  ROM size: log2(8 MiB) - 10 = 13 (ExHiROM's >4 MiB slot)
    .byte $00                   ; $FFD8  RAM size: none
    .byte $01                   ; $FFD9  country: NTSC / US
    .byte $33                   ; $FFDA  developer ID
    .byte $00                   ; $FFDB  ROM version
    .word $0000                 ; $FFDC  checksum complement (patched post-link)
    .word $0000                 ; $FFDE  checksum            (patched post-link)

.segment "VECTORS"
    ; --- native mode, $FFE0-$FFEF ---
    .word $0000, $0000          ; $FFE0, $FFE2  unused
    .addr cop_trampoline        ; $FFE4  COP
    .addr brk_trampoline        ; $FFE6  BRK
    .addr irq_stub              ; $FFE8  ABORT
    .addr nmi_trampoline        ; $FFEA  NMI
    .word $0000                 ; $FFEC  unused
    .addr irq_trampoline        ; $FFEE  IRQ

    ; --- emulation mode, $FFF0-$FFFF ---
    .word $0000, $0000          ; $FFF0, $FFF2  unused
    .addr cop_trampoline_e      ; $FFF4  COP (emulation)
    .word $0000                 ; $FFF6  unused
    .addr irq_stub              ; $FFF8  ABORT
    .addr nmi_trampoline        ; $FFFA  NMI (emulation)
    .addr reset                 ; $FFFC  RESET — the entry point
    .addr brk_trampoline        ; $FFFE  IRQ / BRK (emulation)
