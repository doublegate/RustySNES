-- Per-dot compositor cross-check: MesenCE side (T-CA-10, docs/adr/0014).
--
-- Run a ROM in MesenCE's headless --testRunner for a fixed number of frames, then write its
-- framebuffer as a canonical 0RRRRRGGGGGBBBBB distinct-color histogram in the SAME format the
-- RustySNES `perdot_dump` binary prints, so scripts/perdot_crossval.sh can diff them per ROM.
--
--   PERDOT distinct=<n> colors=<hhhh:count,...>   (sorted by canonical value)
--
-- MesenCE `emu.getScreenBuffer()` returns 0xRRGGBB (8-bit channels) of the rendered frame; we
-- down-sample each channel to 5 bits and pack as R<<10|G<<5|B. The distinct-color SET is the robust
-- signal — it is immune to MesenCE's ~7-row overscan top border vs RustySNES compositing from row 0.
--
-- Env: MCE_RESULT = output file path; MCE_FRAMES = frame count (default 60).

local RES = os.getenv("MCE_RESULT") or "/tmp/perdot_mce.txt"
local TARGET = tonumber(os.getenv("MCE_FRAMES") or "60") or 60
local frames = 0

local function onEndFrame()
  frames = frames + 1
  if frames < TARGET then return end

  local buf = emu.getScreenBuffer()
  local hist = {}
  for i = 1, #buf do
    local c = buf[i]
    local r5 = (c >> 19) & 0x1f
    local g5 = (c >> 11) & 0x1f
    local b5 = (c >> 3) & 0x1f
    local canon = (r5 << 10) | (g5 << 5) | b5
    hist[canon] = (hist[canon] or 0) + 1
  end

  local keys = {}
  for k, _ in pairs(hist) do keys[#keys + 1] = k end
  table.sort(keys)

  local parts = {}
  for _, k in ipairs(keys) do
    parts[#parts + 1] = string.format("%04x:%d", k, hist[k])
  end

  local f = io.open(RES, "w")
  f:write(string.format("PERDOT distinct=%d colors=%s\n", #keys, table.concat(parts, ",")))
  f:close()
  emu.stop(0)
end

emu.addEventCallback(onEndFrame, emu.eventType.endFrame)
