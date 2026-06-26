#!/usr/bin/env bash
# Commercial-ROM screenshot generator — the RustySNES port of RustyNES's screenshot pipeline
# (Rust capture -> categorize -> montage). Boots every staged commercial dump, captures an
# attract-screen PNG, tier-splits it into screenshots/{external,besteffort}/<chip>/, and builds
# a showcase montage. Only the PNGs are committed; ROMs/firmware stay gitignored (ADR 0003).
#
# Usage:  scripts/screenshots/generate.sh [FRAMES]   (default 360 frames per ROM)
# Requires: ImageMagick (magick), a built workspace, the gitignored commercial corpus + firmware.
set -euo pipefail

REPO="$(cd "$(dirname "$0")/../.." && pwd)"
cd "$REPO"
FRAMES="${1:-360}"
DUMP="/tmp/rustysnes-screenshots"
SHOTS="$REPO/screenshots"

echo ">> capturing $FRAMES frames per staged commercial ROM (PPM dump)"
rm -rf "$DUMP"; mkdir -p "$DUMP"
RUSTYSNES_DUMP_FRAMES=1 RUSTYSNES_SHOT_FRAMES="$FRAMES" RUSTYSNES_DUMP_DIR="$DUMP" \
  cargo test -p rustysnes-test-harness --features test-roms --test commercial_screenshots --release -- --nocapture 2>&1 \
  | grep -iE 'commercial_screenshots:|SKIP' || true

# chip -> tier dir: Core/Curated (verified-bar boards) land in external/, BestEffort in besteffort/.
tier_dir() {
  case "$1" in
    None|DSP-1|GSU-1|GSU-2|SA-1) echo "external" ;;   # Core / Curated
    *)                            echo "besteffort" ;; # CX4/S-DD1/SPC7110/OBC1/ST010/DSP-2/DSP-4/Other
  esac
}

echo ">> converting PPM -> PNG + tier-splitting into screenshots/"
rm -rf "$SHOTS/external" "$SHOTS/besteffort"
count=0
while IFS= read -r ppm; do
  rel="${ppm#"$DUMP"/}"            # <map>/<chip>/<game>.ppm
  chip="$(basename "$(dirname "$rel")")"
  game="$(basename "$rel" .ppm)"
  dest="$SHOTS/$(tier_dir "$chip")/$chip"
  mkdir -p "$dest"
  magick "$ppm" "$dest/$game.png"
  count=$((count+1))
done < <(find "$DUMP" -name '*.ppm' | sort)
echo ">> wrote $count screenshots under $SHOTS/{external,besteffort}/"

echo ">> building montage"
mapfile -t pngs < <(find "$SHOTS/external" "$SHOTS/besteffort" -name '*.png' 2>/dev/null | sort)
if [ "${#pngs[@]}" -gt 0 ]; then
  magick montage "${pngs[@]}" -tile 8x -geometry 256x224+2+2 -background black "$SHOTS/montage.png"
  echo ">> montage: $SHOTS/montage.png (${#pngs[@]} tiles)"
fi
echo "done."
