-- Mid-line-raster cross-check: MesenCE side. Report the RED-run length (A columns) per row, so the
-- raster boundary is measured exactly (not as a frame-wide colour-count average). RED = BG1 (colour
-- A), which the ROM draws from column 0 up to the draw cursor at the mid-line TM write; the rest is
-- the blue backdrop (B). Output: the modal boundary and the min/max across picture rows.
-- Env: MCE_RESULT = output path; MCE_FRAMES = frame to sample (default 16).
local RES = os.getenv("MCE_RESULT") or "/dev/stdout"
local TARGET = tonumber(os.getenv("MCE_FRAMES") or "16")
local frames = 0

local function onEndFrame()
  frames = frames + 1
  if frames < TARGET then return end
  local buf = emu.getScreenBuffer()
  local W = 256
  local H = #buf // W
  -- RED (colour A) is any pixel whose red channel dominates: r5 high, g/b low.
  local function is_red(c)
    local r = (c >> 16) & 0xff
    local g = (c >> 8) & 0xff
    local b = c & 0xff
    return r > 128 and g < 64 and b < 64
  end
  -- Per row, the boundary = count of leading RED pixels (BG1 shows [0, boundary)).
  local counts = {}
  local rows = 0
  for y = 0, H - 1 do
    local red = 0
    local any = false
    for x = 0, W - 1 do
      local c = buf[y * W + x + 1]
      if is_red(c) then red = red + 1; any = true end
    end
    -- Only count picture rows (skip pure-black overscan rows with no red and no blue).
    if any then
      counts[red] = (counts[red] or 0) + 1
      rows = rows + 1
    end
  end
  -- Modal boundary + range.
  local modal, modal_n, lo, hi = -1, -1, 1e9, -1
  for b, n in pairs(counts) do
    if n > modal_n then modal, modal_n = b, n end
    if b < lo then lo = b end
    if b > hi then hi = b end
  end
  local parts = {}
  for b, n in pairs(counts) do parts[#parts + 1] = string.format("%d:%d", b, n) end
  table.sort(parts)
  local f = io.open(RES, "w")
  f:write(string.format("MCE_BOUNDARY modal=%d rows=%d min=%d max=%d dist=%s\n",
    modal, rows, lo, hi, table.concat(parts, ",")))
  f:close()
  emu.stop(0)
end

emu.addEventCallback(onEndFrame, emu.eventType.endFrame)
