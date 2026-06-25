# CLAUDE.md — rustysnes-cart

LoROM / HiROM / ExHiROM mapping + coprocessors. Spec: `../../docs/cart.md` and
`../../docs/cartridge-format.md`. Board logic lives here behind default-no-op trait hooks — chip
crates never reach into the cart. New coprocessors/boards implement those hooks; keep them additive
and default-off so native/no_std/wasm builds stay byte-identical. Never commit commercial ROMs.
Touching mapping/board behavior means updating `../../docs/cart.md` in the same PR.
