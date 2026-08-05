#!/usr/bin/env bash
# Build the headless ares host. See README.md — this does NOT yet run the cart successfully.
#
# The recipe is the deliverable: every step below was established by doing it, and the previous
# recorded state of this blocker was "ares would need building", which turned out to be wrong.
set -euo pipefail

# REF_PROJ is MANDATORY and must point OUTSIDE this repository -- ref-proj/ was deleted on
# 2026-08-04 and must not be re-created (docs/ai-emulator-provenance-guardrails.md).
#
# NOTE this script COMPILES against ares' source, so unlike crossval.sh it is not pure oracle I/O.
# That is permitted only because the source lives outside the agent-readable tree: a human builds
# this host once, and the agent thereafter invokes the resulting BINARY. Do not relocate the ares
# clone into the repository to make this easier.
if [[ -z ${REF_PROJ:-} ]]; then
    echo "REF_PROJ is not set. Point it at an out-of-tree checkout, e.g. REF_PROJ=\$HOME/emu-references" >&2
    exit 2
fi
case "$REF_PROJ/" in
    "$PWD"/*) echo "REF_PROJ ($REF_PROJ) is inside the repository -- keep references out of tree." >&2; exit 2 ;;
esac
ARES=$REF_PROJ/ares
BUILD=${BUILD:-${TMPDIR:-/tmp}/ares-sfc-build}
OUT=${OUT:-${TMPDIR:-/tmp}/ares_host}
HERE=$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)

[[ -d $ARES ]] || { echo "no ares clone at $ARES — set REF_PROJ" >&2; exit 1; }

# Only the sfc core. The default core list builds a dozen consoles and is pure wall-clock here.
cmake -S "$ARES" -B "$BUILD" -G Ninja -DARES_CORES=sfc -DCMAKE_BUILD_TYPE=Release

# `hiro` is not optional even for a headless host: `mia/mia.hpp` includes it, and its generated
# `resource/resource.hpp` only exists after hiro has been built once.
ninja -C "$BUILD" ares mia hiro ruby
ninja -C "$BUILD" \
  nall/nall/CMakeFiles/nall.dir/main.cpp.o \
  nall/nall/CMakeFiles/nall.dir/nall.cpp.o \
  nall/nall/CMakeFiles/nall.dir/sljitAllocator.cpp.o

# `nall/main.hpp` must be included by THIS translation unit: it emits `::main` only when
# `NALL_MAIN_IMPL` is undefined, and nall's own `main.cpp.o` defines that. Without the include the
# link fails with a bare `undefined reference to 'main'` from crt1.o, which reads like a missing
# object rather than a missing shim.
g++ -std=c++2b -O2 -g -rdynamic -c "$HERE/ares_host.cpp" -o "$BUILD/ares_host.o" \
  -I"$ARES" -I"$ARES/ares" -I"$ARES/nall" -I"$ARES/libco" -I"$ARES/thirdparty" \
  -I"$BUILD" -I"$BUILD/ares" \
  $(pkg-config --cflags gtk+-3.0) -DHIRO_GTK=3

# The library set and order are cribbed from desktop-ui's own link line in `build.ninja`; ares is
# listed twice there and needs to be here too.
g++ -O2 -g -rdynamic -o "$OUT" "$BUILD/ares_host.o" \
  "$BUILD/nall/nall/CMakeFiles/nall.dir/main.cpp.o" \
  "$BUILD/nall/nall/CMakeFiles/nall.dir/nall.cpp.o" \
  "$BUILD/nall/nall/CMakeFiles/nall.dir/sljitAllocator.cpp.o" \
  "$BUILD/ruby/libruby.a" "$BUILD/hiro/libhiro.a" "$BUILD/ares/libares.a" "$BUILD/mia/libmia.a" \
  "$BUILD/thirdparty/chdr-static.a" \
  -lXrandr -lXrender -lXext -lX11 -lGLX -lOpenGL -lSDL3 -lrashader \
  $(pkg-config --libs gtk+-3.0) -lz \
  "$BUILD/ares/libares.a" "$BUILD/libco/liblibco.a" "$BUILD/thirdparty/tzxfile.a" \
  "$BUILD/thirdparty/chdr-static.a" \
  "$BUILD/thirdparty/libchdr/deps/miniz-3.1.1/miniz.a" \
  "$BUILD/thirdparty/libchdr/deps/zstd-1.5.7/zstd.a" "$BUILD/thirdparty/sljit.a"

echo "built $OUT"
