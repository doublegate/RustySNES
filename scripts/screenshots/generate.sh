#!/usr/bin/env bash
# ROM screenshot generator — boots EVERY staged ROM under tests/roms/ (commercial + krom +
# undisbeliever + blargg + gilyon), captures two frames each (a title-window shot and a
# gameplay-window shot, each the best-content frame in its window to dodge black transitions),
# converts to PNG mirroring the ROM tree under screenshots/, and builds title + gameplay montages.
# Only the PNGs are committed; ROMs/firmware stay gitignored (ADR 0003).
#
# Usage:  scripts/screenshots/generate.sh
#   Frame windows are overridable via RUSTYSNES_TITLE_LO/HI, RUSTYSNES_PLAY_LO/HI (see the test).
# Requires: ImageMagick (magick), a built workspace, the gitignored corpus + firmware.
set -euo pipefail

REPO="$(cd "$(dirname "$0")/../.." && pwd)"
cd "$REPO"
DUMP="/tmp/rustysnes-screenshots"
SHOTS="$REPO/screenshots"

echo ">> capturing title + gameplay frames for every staged ROM (PPM dump)"
rm -rf "$DUMP"; mkdir -p "$DUMP"
RUSTYSNES_DUMP_FRAMES=1 RUSTYSNES_DUMP_DIR="$DUMP" \
  cargo test -p rustysnes-test-harness --features "test-roms commercial-roms" --test commercial_screenshots --release -- --nocapture 2>&1 \
  | grep -iE 'commercial_screenshots:|SKIP' || true

echo ">> converting PPM -> PNG (mirroring the ROM tree under screenshots/)"
# Wipe the previously-committed layouts (both the old tier-split and any prior mirror) so a re-run
# is a clean rebuild rather than a mix of stale + fresh files.
find "$SHOTS" -mindepth 1 -maxdepth 1 -type d -not -name '.*' -exec rm -rf {} +
count=0
while IFS= read -r ppm; do
  rel="${ppm#"$DUMP"/}"                 # <path-under-tests/roms>/<name>_{title,gameplay}.ppm
  dest="$SHOTS/$(dirname "$rel")"
  mkdir -p "$dest"
  magick "$ppm" "$dest/$(basename "$rel" .ppm).png"
  count=$((count+1))
done < <(find "$DUMP" -name '*.ppm' | sort)
echo ">> wrote $count screenshots under $SHOTS/"

# Montages: build separate title + gameplay boards from the commercial games (the visually
# meaningful showcase); the full per-ROM PNG tree covers everything including test ROMs.
build_montage() {
  local suffix="$1" out="$2"
  mapfile -t pngs < <(find "$SHOTS/external/commercial" -name "*_$suffix.png" 2>/dev/null | sort)
  if [ "${#pngs[@]}" -gt 0 ]; then
    magick montage "${pngs[@]}" -tile 8x -geometry 256x224+2+2 -background black "$out"
    echo ">> $out (${#pngs[@]} tiles)"
  fi
}
echo ">> building montages"
build_montage title    "$SHOTS/montage_title.png"
build_montage gameplay "$SHOTS/montage_gameplay.png"
rm -f "$SHOTS/montage.png"   # superseded by the split title/gameplay montages
echo "done."
