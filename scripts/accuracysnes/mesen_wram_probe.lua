-- Minimal MesenCE WRAM probe for the AccuracySNES results block.
--
-- Exists because the battery oracle's diagnosis had been built on `mesen_crossval.lua`'s own
-- reading, with no independent check that the read itself was sound. This script does one thing --
-- run N frames with the input contract held, then dump `RESULTS` ($7E:F000, snesWorkRam offset
-- $F000) -- so a claim about how far the battery got can be made from a second, much smaller
-- instrument.
--
--   MCE_FRAMES=1500 MCE_RESULT=/tmp/w.txt SDL_VIDEODRIVER=offscreen SDL_AUDIODRIVER=dummy \
--     ref-proj/MesenCE/bin/linux-x64/Release/Mesen --testRunner \
--     --snes.port2.type=SnesController scripts/accuracysnes/mesen_wram_probe.lua \
--     tests/roms/AccuracySNES/build/accuracysnes.sfc
--
-- What it found (2026-08-01): the magic is uninitialised WRAM that DIFFERS between runs, and every
-- status byte in the first 40 catalogue slots is zero at 600 and at 1500 frames. So the battery is
-- not stopping partway -- it never writes a verdict at all.

local TARGET = tonumber(os.getenv("MCE_FRAMES") or "900")
local RES = os.getenv("MCE_RESULT") or "/tmp/wram.txt"
local frames = 0
local function onInput()
  emu.setInput({ b=true, start=true, x=true, r=true, y=false, select=false, a=false, l=false,
                 up=false, down=false, left=false, right=false }, 0)
  emu.setInput({ y=true, select=true, a=true, l=true, b=false, start=false, x=false, r=false,
                 up=false, down=false, left=false, right=false }, 1)
end
local function onEndFrame()
  frames = frames + 1
  if frames < TARGET then return end
  local f = io.open(RES, "w")
  -- WRAM offset of $7E:F000 is 0xF000. Dump the magic + a slice of the status array.
  local magic = ""
  for i = 0, 3 do
    magic = magic .. string.format("%02x", emu.read(0xF000 + i, emu.memType.snesWorkRam))
  end
  f:write(string.format("frames=%d magic@F000=%s\n", frames, magic))
  local nonzero, first_zero = 0, -1
  local bytes = {}
  for i = 0, 359 do
    local b = emu.read(0xF020 + i, emu.memType.snesWorkRam)
    if b ~= 0 then nonzero = nonzero + 1 elseif first_zero < 0 then first_zero = i end
    if i < 40 then bytes[#bytes+1] = string.format("%02x", b) end
  end
  f:write(string.format("status nonzero=%d first_zero_index=%d\n", nonzero, first_zero))
  f:write("first40=" .. table.concat(bytes, " ") .. "\n")
  f:close()
  emu.stop(0)
end
emu.addEventCallback(onInput, emu.eventType.inputPolled)
emu.addEventCallback(onEndFrame, emu.eventType.endFrame)
