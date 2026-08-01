local TARGET = tonumber(os.getenv("MCE_FRAMES") or "600")
local HOLD_FROM = tonumber(os.getenv("HOLD_FROM") or "0")
local RES = os.getenv("MCE_RESULT") or "/tmp/held.txt"
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
  local function w(off) -- VAR_BASE $7E:E000 -> snesWorkRam offset $E000
    return emu.read(0xE000 + off, emu.memType.snesWorkRam)
       | (emu.read(0xE000 + off + 1, emu.memType.snesWorkRam) << 8)
  end
  local f = io.open(RES, "w")
  f:write(string.format("hold_from=%d frames=%d V_PAD_HELD=%04x V_PAD_LAST=%04x V_PAD_NEW=%04x\n",
                        HOLD_FROM, frames, w(4), w(6), w(8)))
  f:close()
  emu.stop(0)
end
emu.addEventCallback(onInput, emu.eventType.inputPolled)
emu.addEventCallback(onEndFrame, emu.eventType.endFrame)
