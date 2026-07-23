-- MesenCE side of the $213E over-flag eval-line probe.
-- The ROM samples STAT77 into WRAM[$1000 + scanline] every scanline via H-IRQ. After a few frames
-- have populated the array, read it back and print the first scanline whose bit 6 (range over) and
-- bit 7 (time over) are set.

local frames = 0

local function onEndFrame()
  frames = frames + 1
  if frames < 8 then return end

  local function firstSet(bit)
    for s = 80, 130 do
      local v = emu.read(0x1000 + s, emu.memType.snesWorkRam)
      if (v & bit) ~= 0 then return s end
    end
    return -1
  end

  local rangeS = firstSet(0x40)
  local timeS = firstSet(0x80)
  -- Dump the raw window around the transition for confidence.
  local dump = {}
  for s = 96, 112 do
    dump[#dump + 1] = string.format("%d:%02x", s, emu.read(0x1000 + s, emu.memType.snesWorkRam))
  end
  print(string.format("MESEN range_over first-set scanline=%d  time_over=%d", rangeS, timeS))
  print("MESEN window " .. table.concat(dump, " "))
  emu.stop(0)
end

emu.addEventCallback(onEndFrame, emu.eventType.endFrame)
