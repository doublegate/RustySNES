local TARGET = tonumber(os.getenv("MCE_FRAMES") or "300")
local RES = os.getenv("MCE_RESULT") or "/tmp/pad.txt"
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
  -- Auto-joypad results ($4218-$421F) and the enable bit ($4200 bit0), read as CPU registers.
  local function reg(a) return emu.read(a, emu.memType.snesMemory) end
  f:write(string.format("frames=%d NMITIMEN($4200)=%02x\n", frames, reg(0x4200)))
  f:write(string.format("JOY1=%02x%02x JOY2=%02x%02x JOY3=%02x%02x JOY4=%02x%02x\n",
    reg(0x4219), reg(0x4218), reg(0x421B), reg(0x421A),
    reg(0x421D), reg(0x421C), reg(0x421F), reg(0x421E)))
  -- What the cart itself believes: the runtime keeps the merged pad somewhere in VAR_BASE, but the
  -- register read above is the decisive one -- it is what the menu polls.
  f:close()
  emu.stop(0)
end
emu.addEventCallback(onInput, emu.eventType.inputPolled)
emu.addEventCallback(onEndFrame, emu.eventType.endFrame)
