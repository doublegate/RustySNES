# ADR 0010 — HD texture pack system: hashing, the `TileTag` hook, and the core/frontend split

## Status

Accepted (`v1.3.0`). The PPU-side hook, the frontend loader, the pure CPU compositor, the
Settings-UI/config pack selection, and (final integration) the live wgpu present-path wiring are
all implemented — a selected pack's replacements are actually visible on screen.

## Context

RustySNES had a bare `hd-pack = []` Cargo feature with no format, no loader, no hook point, and
no ADR — `docs/STATUS.md` implied more scaffolding existed than actually did. Building the real
system required four genuinely load-bearing decisions, none of which had an existing in-repo
precedent to follow verbatim:

1. **Where does tile identity come from, and how is it hashed?** By the time a pixel reaches the
   final composited BGR555 framebuffer (`Ppu::compose_dac`), all information about which VRAM
   tile + CGRAM palette produced it is gone — `docs/adr/0004`'s determinism boundary also means
   this can't be reconstructed after the fact from outside the PPU.
2. **How does a hot rendering path record that identity without becoming an accuracy or
   performance liability?** The PPU's per-scanline compositor is squarely a hot path (this
   project's own testing/performance discipline — module 30 — treats it as one); adding a feature
   most builds never use must not cost anything when it's off.
3. **Where does pack loading/matching/compositing live?** The core is not allowed to become
   pack-aware (`docs/adr/0004`); Mesen2's own NES HD Pack system (the direct external precedent
   this design studied) draws exactly this line, and Dolphin's independent convergence on the
   same "fast non-cryptographic hash keyed by content, not asset ID" pattern corroborates the
   hashing choice below.
4. **What does a pack look like on disk, and how is it versioned?** No existing format in this
   project's ecosystem (RustyNES has no equivalent feature) to copy.

## Decision

1. **Palette-inclusive XXH3-64 hashing, computed once, in `rustysnes-ppu`.**
   `hdtag::hash_tile(class, bpp, tile_words, palette)` hashes a fixed byte sequence — a 1-byte
   [`TileClass`] discriminant, a 1-byte `bpp`, the tile's raw **pre-flip** VRAM words, and the
   resolved `2^bpp`-color CGRAM palette this specific instance uses — into a `u64` via XXH3 (fast,
   non-cryptographic; this is a cache key, not a security boundary, matching Dolphin's own
   convergence on the xxHash family). **Palette-inclusive** means the same bitmap under two
   different palettes hashes to two different entries — trading pack storage for zero recoloring
   math in the render hot path, exactly Mesen2's own `HdTileKey` precedent. Flip is deliberately
   excluded from the hash (both orientations share one pack entry; the frontend mirrors the source
   rect at composite time instead).
2. **A write-only, off-by-default `TileTag` side-channel — never part of `save_state`.**
   `Ppu::tile_tags()` parallels `Ppu::framebuffer()` pixel-for-pixel, populated only when
   `Ppu::set_hd_pack_tagging(true)` is set. Both the side-buffer and the tagging flag are
   `#[cfg(feature = "hd-pack")]`-gated out of existence (not just runtime-disabled) when the
   feature is off, and even when the feature is compiled in, leaving tagging at its default
   `false` is proven byte-identical to every prior release
   (`hd_pack_tagging_toggle_does_not_alter_framebuffer_output`, `docs/ppu.md`). Neither field is
   ever written to a save-state — the same host/frontend-convenience carve-out already established
   for cheats, watchpoints, per-voice mutes, and the port-2 peripheral selection (a texture pack
   choice isn't part of the deterministic, replayable machine state).
3. **All loading, caching, and compositing lives in the frontend; the core stays pack-agnostic.**
   `rustysnes-core` and `rustysnes-ppu` know nothing of `pack.toml`, PNG decoding, or file paths —
   they expose exactly one hook (`Ppu::tile_tags`) and nothing else. `rustysnes-frontend::hd_pack`
   owns the manifest schema, the loader (`HdPack::load`, PNG decode via the pure-Rust `png` crate,
   with path-traversal validation on every manifest-declared image path), and per-ROM discovery.
   `rustysnes-frontend::hd_compositor` owns the actual per-pixel substitution as a pure function
   (framebuffer + tags + pack in, a new RGBA8 buffer out) — deliberately free of any wgpu/`EmuCore`
   dependency, so it is fully unit-testable without a GPU adapter. v1 does one CPU-side
   compose-then-single-upload per frame (no incrementally-populated GPU atlas) — simpler to get
   correct first; an atlas is the natural v2 performance escape hatch if profiling ever demands it,
   not built speculatively now.
4. **A versioned TOML manifest.** `<data-dir>/hd-packs/<rom-sha256-hex>/<pack-name>/pack.toml` +
   `tiles/*.png`, mirroring `save_states.rs`'s proven per-ROM-hash directory convention. The
   manifest carries an explicit `format_version` (mirroring the `rustysnes-savestate`
   `FORMAT_VERSION` gate convention) that the loader checks before trusting anything else in the
   file — a pack written against a future, incompatible schema is rejected outright, not
   partially/incorrectly parsed. Duplicate tile hashes within one manifest are also rejected
   outright (an authoring mistake, not something to silently resolve by entry order).

## Consequences

- (+) The determinism contract is untouched: no core crate gained a real dependency on pack
  content, disk I/O, or an image decoder. A `--no-default-features` `no_std` build and every
  existing golden/oracle test are provably unaffected by this feature's mere existence.
- (+) The hashing recipe is verified two ways: a pinned known-vector regression test in
  `rustysnes-ppu` (`hdtag::tests::known_vector_is_stable`), and an independent-recompute test that
  rebuilds the same hash from raw VRAM/CGRAM bytes read back out of a real rendered scene
  (`hd_pack_tagging_records_the_documented_hash_for_a_known_bg_tile`).
- (+) The frontend's loader and compositor are both unit-tested without any GPU or real pack asset
  — the compositor via synthetic in-memory framebuffers/tags/tiles, the loader via real (small,
  generated) PNG fixtures round-tripped through `HdPack::load`.
- (−) **No real community HD pack exists to validate against.** Every test here is synthetic
  (hand-built manifests and 2×2 fixture PNGs), the same honestly-tracked-open posture this
  project already applies to hi-res real-title validation (`docs/ppu.md`). This is a gap to close
  when a real pack — or motivation to author one — appears, not something to fabricate evidence
  for now.
- (−) **v1's CPU-compose-then-upload is not the fastest possible design at scale.** A pack with
  many large replacement tiles recomposites the *entire* frame's worth of 8×8 cells every frame,
  even when the great majority are untagged/native. This is an accepted, documented tradeoff for
  correctness-first delivery, not an oversight — see decision 3's atlas note for the known
  escape hatch.
- (+) **The compositor is wired into the live wgpu present path.** `Gfx`'s streaming texture,
  previously a fixed `MAX_W × MAX_H` allocation, now grows on demand
  (`Gfx::ensure_texture_capacity`) to fit whatever the composited output needs, and its UV math
  divides by the texture's actual current size rather than the `MAX_W`/`MAX_H` constants — a
  pure generalization that leaves the no-pack-active path byte-identical (the texture never grows
  past its original allocation when nothing composites a larger buffer into it). The upscale
  factor is fixed at 2x (`HD_PACK_SCALE` in `app.rs`) rather than user-configurable — a scoped v1
  choice, not a technical ceiling; a pack author wanting less resampling loss on higher-resolution
  source images would need a future config knob, not an architecture change.
- (−) **`emu-thread` + `hd-pack` together don't composite.** The `emu-thread` build's framebuffer
  arrives via a lock-free `PresentBuffer` handoff outside the locked block the compositor reads
  `Ppu::tile_tags` from; that build silently falls back to the plain native framebuffer when a
  pack is selected. Building an equivalent `TileTag` handoff channel is a separate, deferred
  scope (`docs/frontend.md`).
