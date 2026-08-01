-- MesenCE probe: WHICH R_STATUS indices are non-zero, and the raw RESULTS header.
--
-- Written to check a "24 non-zero bytes" reading that turned out to be an over-read: those bytes are
-- at indices 336-359, past the end of the 335-entry array. Reading a byte count without reading the
-- indices is how that went unnoticed for a whole round -- hence this probe.
--
-- What it establishes (2026-08-01): R_STATUS[0..334] is entirely ZERO while the RESULTS header at
-- $F000 is RANDOM and differs every run. MesenCE powers on with randomised WRAM, so an all-zero
-- array is not the power-on state -- runtime.s's own clear loop zeroed it, which means the cart's
-- init runs and it reaches the pre-battery menu. The open question is why it never leaves that menu,
-- whose exit condition the source names as "wait for Start".

local TARGET = tonumber(os.getenv("MCE_FRAMES") or "1200")
local RES = os.getenv("MCE_RESULT") or "/tmp/idx.txt"
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
  local hits = {}
  for i = 0, 359 do
    local b = emu.read(0xF020 + i, emu.memType.snesWorkRam)
    if b ~= 0 then hits[#hits+1] = string.format("%d=%02x", i, b) end
  end
  f:write("nonzero_indices: " .. table.concat(hits, " ") .. "\n")
  -- Also dump the header words so we can see whether ANY of RESULTS was written.
  local hdr = {}
  for i = 0, 31 do hdr[#hdr+1] = string.format("%02x", emu.read(0xF000 + i, emu.memType.snesWorkRam)) end
  f:write("header F000..F01F: " .. table.concat(hdr, " ") .. "\n")
  f:close()
  emu.stop(0)
end
emu.addEventCallback(onInput, emu.eventType.inputPolled)
emu.addEventCallback(onEndFrame, emu.eventType.endFrame)
