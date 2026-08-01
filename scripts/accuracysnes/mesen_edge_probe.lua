-- MesenCE probe: does the AccuracySNES battery need a press EDGE on the contract buttons?
--
-- HOLD_FROM is the frame at which the input contract is first applied, so HOLD_FROM=0 reproduces the
-- constant hold every runner uses and any larger value manufactures a rising edge.
--
-- RESULT (2026-08-01): REFUTED. HOLD_FROM 0, 30 and 120 all produce the same shape at 1200 frames --
-- magic never reads ACSN, R_DONE never reads $A5, and R_STATUS has exactly 24 non-zero bytes. The
-- edge is not what the battery is waiting for. Kept so the next attempt does not re-run it.
--
-- The 24 is the live lead, not the zero: it is CONSTANT across runs while the magic bytes VARY, so it
-- is not uninitialised WRAM (which would vary too), and indices 0-39 read zero, so those bytes sit at
-- index >= 40. Find which indices they are before theorising further.

local TARGET = tonumber(os.getenv("MCE_FRAMES") or "900")
local HOLD_FROM = tonumber(os.getenv("HOLD_FROM") or "0")   -- frame at which the contract is applied
local RES = os.getenv("MCE_RESULT") or "/tmp/edge.txt"
local frames = 0
local function onInput()
  local on = frames >= HOLD_FROM
  emu.setInput({ b=on, start=on, x=on, r=on, y=false, select=false, a=false, l=false,
                 up=false, down=false, left=false, right=false }, 0)
  emu.setInput({ y=on, select=on, a=on, l=on, b=false, start=false, x=false, r=false,
                 up=false, down=false, left=false, right=false }, 1)
end
local function onEndFrame()
  frames = frames + 1
  if frames < TARGET then return end
  local f = io.open(RES, "w")
  local magic = ""
  for i = 0, 3 do magic = magic .. string.format("%02x", emu.read(0xF000 + i, emu.memType.snesWorkRam)) end
  local done = emu.read(0xF008, emu.memType.snesWorkRam)
  local nonzero = 0
  for i = 0, 359 do
    if emu.read(0xF020 + i, emu.memType.snesWorkRam) ~= 0 then nonzero = nonzero + 1 end
  end
  f:write(string.format("hold_from=%d frames=%d magic=%s done=%02x status_nonzero=%d\n",
                        HOLD_FROM, frames, magic, done, nonzero))
  f:close()
  emu.stop(0)
end
emu.addEventCallback(onInput, emu.eventType.inputPolled)
emu.addEventCallback(onEndFrame, emu.eventType.endFrame)
