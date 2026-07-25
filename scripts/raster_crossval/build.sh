#!/usr/bin/env bash
# Assemble + link the mid-line-raster cross-check ROM. Usage: build.sh [RASTER_DOT]
set -euo pipefail
cd "$(dirname "$0")"
DOT="${1:-128}"
ca65 --cpu 65816 -D RASTER_DOT="$DOT" -o raster.o raster.s
ld65 -C raster.cfg -o raster.sfc raster.o
echo "built raster.sfc (RASTER_DOT=$DOT)"
