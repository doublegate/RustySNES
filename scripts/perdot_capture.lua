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
  -- Check the write AND the close: a flush failure (disk full, I/O error) surfaces at close, and a
  -- silently-truncated MCE_RESULT would then be diffed as a partial capture. Fail loudly instead.
  -- Optional per-ROW signature, mirroring `perdot_dump`'s `PERDOT_ROWS`. The histogram above is
  -- position-blind by design, and therefore blind to a change that only moves a band -- which is the
  -- entire content of a raster test. One token per row lets the two sides be aligned by band
  -- boundary. A uniform row prints its colour; a mixed row prints `----`.
  local rowsig = ""
  if os.getenv("PERDOT_ROWS") ~= nil then
    local width = 256
    local rows = #buf // width
    local toks = {}
    for y = 0, rows - 1 do
      local base = y * width
      local function canon_at(i)
        local c = buf[base + i]
        return (((c >> 19) & 0x1f) << 10) | (((c >> 11) & 0x1f) << 5) | ((c >> 3) & 0x1f)
      end
      local first = canon_at(1)
      local uniform = true
      for i = 2, width do
        if canon_at(i) ~= first then uniform = false break end
      end
      toks[#toks + 1] = uniform and string.format("%04x", first) or "----"
    end
    rowsig = string.format("PERDOTROWS rows=%d sig=%s\n", rows, table.concat(toks, ","))
  end

  local wok, werr = f:write(string.format("PERDOT distinct=%d colors=%s\n", #keys, table.concat(parts, ",")) .. rowsig)
  if not wok then
    io.stderr:write(string.format("perdot_capture: write to '%s' failed: %s\n", RES, werr or "?"))
    f:close()
    emu.stop(1)
    return
  end
  local cok, cerr = f:close()
  if not cok then
    io.stderr:write(string.format("perdot_capture: close of '%s' failed: %s\n", RES, cerr or "?"))
    emu.stop(1)
    return
  end
  emu.stop(0)
end

emu.addEventCallback(onEndFrame, emu.eventType.endFrame)
