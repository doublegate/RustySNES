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

-- Require an explicit output path — no predictable /tmp fallback. A default like
-- `/tmp/perdot_mce.txt` could be redirected through a pre-planted symlink before the write below
-- truncates it (CWE-377); the driver (perdot_crossval.sh) always sets MCE_RESULT to a file inside
-- its own `mktemp -d` dir.
local RES = os.getenv("MCE_RESULT")
if RES == nil or RES == "" then
  io.stderr:write("perdot_capture: MCE_RESULT must be set to the output file path\n")
  emu.stop(1)
  return
end

-- Frame count shares a positive-integer contract with the RustySNES side (`perdot_dump`): a
-- zero/negative/non-integer TARGET would capture a different frame than RustySNES renders and
-- manufacture a false diff. Default to 60 when unset, but abort on a supplied-yet-invalid value
-- rather than silently falling back.
local TARGET = 60
local raw = os.getenv("MCE_FRAMES")
if raw ~= nil then
  local n = tonumber(raw)
  if n == nil or n < 1 or n % 1 ~= 0 then
    io.stderr:write(string.format("perdot_capture: MCE_FRAMES must be a positive integer, got '%s'\n", raw))
    emu.stop(1)
    return
  end
  TARGET = n
end
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

  local f, err = io.open(RES, "w")
  if f == nil then
    io.stderr:write(string.format("perdot_capture: cannot open MCE_RESULT '%s': %s\n", RES, err or "?"))
    emu.stop(1)
    return
  end
  f:write(string.format("PERDOT distinct=%d colors=%s\n", #keys, table.concat(parts, ",")))
  f:close()
  emu.stop(0)
end

emu.addEventCallback(onEndFrame, emu.eventType.endFrame)
