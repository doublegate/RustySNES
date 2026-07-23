# ADR 0014 — Per-dot PPU compositor

## Status

**Accepted** (2026-07-22; per-dot made the shipped default and the batch path removed 2026-07-23).
Supersedes the per-scanline "simplification point" documented in `docs/ppu.md` §Rendering model and
the `v0.8.0` line-granularity fix (ADR-adjacent, recorded in `docs/ppu.md` §Mid-scanline/HDMA-driven
register timing). The per-dot compositor was landed incrementally behind a hard non-regression
invariant (below) and a default-off `per-dot-compositor` feature; once every phase held, Phase 6
flipped it to the shipped default and then **removed the batch `render_scanline`/`compose_dac` path
and the feature entirely** — the per-dot compositor is now the single code path (it is `no_std`-clean,
so nothing needed the batch fallback). `compose_dac` survives only as a `#[cfg(test)]` driver that
feeds hand-built pixel rows into the shipped per-column `compose_pixel` for the hi-res DAC tests.

## Context

RustySNES composites each visible scanline **once, at `RENDER_DOT` (dot 276)**, using the register
state as it stood at that dot (`crates/rustysnes-ppu/src/render.rs` `render_scanline`, driven from
`Ppu::tick_dot`). `v0.8.0` moved the composite from end-of-line (dot 340) to dot 276 so that a
per-line HDMA register write becomes visible starting the *following* line — correct at **line**
granularity.

It is still not correct at **dot** granularity. A register that the line's rendering reads, written
mid-visible-line (dots 22–277), affects the **whole** line rather than only the pixels after the
write. `docs/ppu.md` states this plainly: the per-scanline model is "bit-identical to a per-dot
renderer only when no register a line's rendering reads is changed mid-line — but it does NOT always
hold." The accuracy ledger lists this as the single largest **Tier-1** (ROM-observable) accuracy gap,
and it cascades: offset-per-tile (Modes 2/4/6), interlace field-doubling, and mid-line raster tricks
(Air Strike Patrol's BG3 mid-scanline scroll, HDMA palette/scroll splits, B2.09 window edges) all
depend on it.

It also blocks a cluster of AccuracySNES rows outright — the mid-line-timing and dot-resolution
assertions — and is the honest prerequisite for true cycle-accuracy to the SNES PPU, which is this
project's stated goal.

The reference implementations (`ref-proj/ares/ares/sfc/ppu/`, `ref-proj/bsnes/bsnes/sfc/ppu/` —
ares' accurate PPU is a lineal descendant of bsnes', line-for-line identical in the cycle
dispatchers) render **per-dot**, and their mechanism was mapped in full for this ADR (below). This
is a port of that *mechanism*, clean-room — nothing copied verbatim, structurally informed by the
map, consistent with `docs/adr/0002`/`0013` methodology.

## Decision

Replace the per-line batch composite with a **per-dot compositor** driven by the master-clock
scheduler, porting the ares/bsnes accurate-PPU mechanism clean-room. The project-wide determinism
contract (`docs/adr/0004`) stands unchanged: the same seed + ROM + input must still yield
bit-identical **framebuffer *and* audio**. This change touches only the framebuffer path (audio is
downstream of the APU and unaffected), so the compositor's obligation is that the finished frame stay
reproducible — and, more strictly, **bit-identical to the current renderer for every static-register
line**, diverging only where a mid-line register change makes the current renderer wrong.

### The mechanism being ported (from the ares/bsnes accurate PPU)

1. **Granularity.** Rendering is per-dot, not per-scanline. One visible dot = 4 master clocks. The
   accurate PPU runs a per-clock dispatcher (`PPU::cycle<N>`, `ares/.../main.cpp:212`) that at each
   dot (a) runs one background tile-fetch slot and (b) once past the fetch-ahead window, drains one
   pixel from each background's shift register and composites objects + windows + color-math + DAC.
2. **Dot-to-pixel timing.** Visible pixel `k` (0..255) is emitted at H-counter clock `58 + 4·k`
   (`x = (hcounter − 56) >> 2`, `background.cpp:196`); the 256 pixels span clocks 58…1078. Prefetch
   begins at clock 0; `cycleBackgroundBegin` at clock 56 shifts off the partial first tile column.
3. **Register-write latency is EMERGENT, not a queue.** There is no "effective at dot X" latch list.
   The accurate PPU `synchronize`s the PPU to the CPU's exact access clock, then writes to shared
   state the per-dot `run()` reads live. Latency comes from exactly two pipeline behaviors that MUST
   be preserved:
   - **14-dot background fetch-ahead.** `fetchNameTable`/`fetchCharacter` fill a `tiles[]` ring ~14
     dots ahead of `run()` draining it. So a mid-line scroll (`BGnHOFS/VOFS`), nametable/character
     base, or `BGMODE` write only affects tile columns **not yet fetched** — the classic ~2-tile
     latency.
   - **In-render CGRAM/OAM address latch.** A CGRAM (`$2121/$2122`) or OAM write during active
     display (`!displayDisable && 0<v<vdisp && 88<=h<1096`) is redirected to `latch.cgramAddress` /
     `latch.oamAddress` (the address the DAC/sprite unit last read), not the programmed address
     (`io.cpp:47-61,31-45`). This is the CGRAM/OAM-write-during-render hazard.

   Every other rendering register — `CGWSEL`/`CGADSUB`/`COLDATA`, `TM`/`TS`, windows
   (`W12SEL`/`WH0-3`/`WBGLOG`), Mode-7 matrix (`M7A-D`, via a byte-assembly latch that is *not* a
   timing delay) — is consumed in the same-dot composite and takes effect on the **next composited
   pixel**, no pipeline delay.
4. **Background fetch pipeline.** An 8-slot per-dot state machine keyed on `(hcounter/4) & 7`
   dispatches nametable / character / (Modes 2/4/6) offset fetches per `BGMODE`, filling `tiles[]`;
   `Background::run` drains 2 bits/plane per dot from `tile.data[]`, advancing a `renderingIndex`.
5. **Hi-res (Modes 5/6, pseudo-hires `SETINI` bit 3).** Each render dot emits **two** framebuffer
   entries: even (left) = the below/sub-screen color (falls back to above when not hi-res, i.e.
   pixel-doubling), odd (right) = the above/main-screen color. `Background::run` is invoked twice per
   dot (Below then Above).
6. **Sprites.** Range evaluation is spread across the scanline (one OAM group per 8 clocks) into a
   **double-buffered** item list (building next line's list while this line displays); tile/pattern
   fetch happens once at end-of-line; the sprite-vs-background composite is genuinely **per-dot**
   from the other buffer.
7. **Offset-per-tile & interlace.** Both fall out at dot resolution once the fetch state machine and
   the per-dot BG/sprite fetch honor `field()` and the BG3 offset lookup — no separate machinery.

### Regression-safety strategy (the load-bearing part)

The current per-scanline renderer already implements BG modes 0–7, Mode-7 affine, the 128-sprite
pipeline, windows, and color-math **correctly** for static lines, verified by 29 undisbeliever
goldens, the coprocessor golden corpus (24 GSU + SA-1 + DSP-1 …), 53 blessed AccuracySNES scenes,
and the commercial-screenshot suite. The port must not cost any of that. Therefore:

- **Invariant:** for any line on which no rendering-relevant register changes between the first and
  last visible dot, the per-dot compositor must produce a **byte-identical** framebuffer line to the
  current renderer. This is a testable property (feed both renderers the same static state; diff).
- **Incremental phases, each gated on the full corpus** (`undisbeliever_golden`, every `*_oncart`,
  the coprocessor goldens, `accuracysnes_scenes` 53/53, commercial screenshots, and both AccuracySNES
  cross-validation references) staying green before the phase lands:
  1. **[LANDED #205]** Extract a per-pixel composite entry point from `render_scanline` that, driven
     in a loop over the 256 dots with **static** state, reproduces the current output bit-for-bit
     (pure refactor; the corpus is the proof it changed nothing). — `compose_pixel`/`DacCarry`.
  2. **[LANDED]** Move the background pixels behind a per-line drain buffer fed by the existing
     per-column fetch, still under static state — corpus stays identical. `render_bg` now runs a
     FETCH pass (each column's resolved `Pixel` into a fixed `[Pixel; 256]` line buffer — the future
     shift-register, allocation-free) then a separate DRAIN pass that does the window+priority
     composite into `above`/`below`. Byte-identical because each column touches only its own
     `above[x]`/`below[x]`; verified against the 53 scene goldens, the undisbeliever framebuffers, and
     the 29 PPU unit tests. This decouples fetch from composite so Phase 4 can drive the drain per-dot.
  3. Introduce the 14-dot fetch-ahead and the in-render CGRAM/OAM address latch — the first
     behaviors that *intentionally* differ from the per-line model, each validated against a
     reference render and an AccuracySNES/undisbeliever ROM that exercises it, and any golden that
     legitimately changes re-blessed **only** from a render the references agree on (ADR 0013 rule
     4), never blind-re-blessed.
  4. Wire the composite to `tick_dot` per-dot (retire the single dot-276 call), then hi-res
     two-sub-pixel output, then offset-per-tile / interlace at dot resolution.
- **Determinism** (`docs/adr/0004`) is unchanged: the per-dot loop is driven by the same
  master-clock scheduler; same seed+ROM+input ⇒ byte-identical frame.
- **Performance:** the hot path stays allocation-free (fixed `tiles[]`/shift-register buffers, no
  per-dot `Vec`); the per-dot loop is more work per line than the batch, so it is measured against
  the current renderer and the budget in `docs/scheduler.md` before and after (profile-first, per the
  quality gates).

## Consequences

- **Positive:** true dot-resolution accuracy — mid-line register writes, offset-per-tile, interlace
  field-doubling, and hi-res dot timing all become correct; unblocks the AccuracySNES mid-line/dot
  cluster and the Air Strike Patrol / HDMA-split raster-trick class; closes the largest Tier-1
  accuracy-ledger gap.
- **Cost/risk:** a substantial rewrite of the most intricate module (`render.rs`, ~1.8 kLOC) against
  the largest golden corpus in the project. Mitigated by the static-line invariant and the phased,
  corpus-gated rollout above — the same discipline `v0.8.0` used when its HDMA-timing fix legitimately
  changed the SA-1 and Super FX/GSU goldens (each re-blessed only after independent row-level
  verification, never blind).
- **Docs to update as phases land:** `docs/ppu.md` §Rendering model (the per-scanline "simplification
  point" language retires), `docs/accuracy-ledger.md` (move the per-scanline row from acknowledged to
  Remediated as each behavior lands), `docs/scheduler.md` (per-dot render call timing), and the
  AccuracySNES coverage as the newly-testable rows land.

## References

- `docs/ppu.md` §Rendering model, §Mid-scanline/HDMA-driven register timing (the v0.8.0 line-fix).
- `docs/accuracy-ledger.md` §PPU/timing residuals; dossier §Part IX (admitted approximations).
- ares accurate PPU: `ref-proj/ares/ares/sfc/ppu/` — `main.cpp` (cycle dispatch), `background.cpp`
  (fetch pipeline + `run`), `object.cpp` (sprite eval/fetch/run), `dac.cpp` (compositor + hi-res),
  `window.cpp`, `mode7.cpp`, `io.cpp` (register writes + CGRAM/OAM in-render latch). bsnes
  equivalents under `ref-proj/bsnes/bsnes/sfc/ppu/` (`screen.cpp` in place of `dac.cpp`).
- `docs/adr/0004` (determinism contract), `docs/adr/0013` (golden re-bless rule 4).
