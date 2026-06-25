# Commercial-corpus acquisition list — closing the hardware-coverage gaps

Derived from `scripts/coverage/coverage.py survey` over the local No-Intro set (995 ROMs,
2026-06-25). These boards/coprocessors are **absent from the current dump** and must be sourced
(all are Japan-only, which is why a USA-weighted set lacks them). Acquiring the **4 essential**
titles below completes coverage of every coprocessor family + the ExHiROM map model.

**Legal note:** acquire only ROMs you are entitled to dump from cartridges you own. These land,
gitignored, in `tests/roms/external/commercial/`; only the derived manifest + golden hashes +
screenshots are ever committed (same policy as the rest of the corpus — ADR 0003).

## Essential (4 titles close 5 gaps)

| Priority | Title (region) | Closes | Why it's the only option |
|---|---|---|---|
| 1 | **Dai Kaijuu Monogatari II (J)** | **ExHiROM** map + **S-RTC** | The canonical ExHiROM cart *and* the main S-RTC (real-time-clock) game — one ROM, two gaps. |
| 2 | **SD Gundam GX (J)** | **DSP-3** | The *only* retail game using the DSP-3 (µPD77C25 variant). |
| 3 | **Hayazashi Nidan Morita Shougi (J)** | **ST011** | The only ST011 (µPD96050) game. |
| 4 | **Hayazashi Nidan Morita Shougi 2 (J)** | **ST018** | The only ST018 game (the ARMv3 coprocessor — unique silicon). |

## Optional (breadth, not new hardware)

| Title (region) | Adds | Note |
|---|---|---|
| **Tales of Phantasia (J)** | 2nd ExHiROM exemplar | Largest commercial SNES ROM (6 MB); good ExHiROM stress case beyond Dai Kaijuu II. |
| **Momotarou Dentetsu Happy (J)** | 2nd SPC7110 | Set has only 1 SPC7110 game; 3 exist. |
| **Super Power League 4 (J)** | 3rd SPC7110 | Completes the SPC7110 population. |

## Already population-complete (do NOT chase — the games don't exist)

These are at their **maximum possible** count in the current set; "4–5 each" is impossible
because the chip shipped in fewer games. The corpus takes the entire population and labels it:

- **S-DD1** — 2 games total (Star Ocean + Street Fighter Alpha 2); both present. ✓ complete.
- **CX4** — 2 games (Mega Man X2/X3); present across regions. ✓ complete.
- **OBC1** — 1 game (Metal Combat). ✓ complete.
- **DSP-2** — 1 game (Dungeon Master). ✓ complete.
- **DSP-4** — 1 game (Top Gear 3000). ✓ complete.
- **ST010** — 1 game (F1 ROC II). ✓ complete.

## Verify after acquiring

Drop the new ROMs into the same library dir and re-run:

```bash
python3 scripts/coverage/coverage.py --target 5 survey "<rom-library>"
```

The `gap` column should now show only the genuinely population-limited single-game buckets
(those are expected and acceptable per ADR 0003's BestEffort tier — they never back the
accuracy oracle, they only carry reference screenshots).
