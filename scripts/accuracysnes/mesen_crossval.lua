-- AccuracySNES cross-validation driver for Mesen2's headless test runner.
--
-- Mesen2 is an independent, mature SNES emulator. Running AccuracySNES on it answers the one
-- question our own harness structurally cannot: are the cart's expected values right, or does it
-- merely agree with RustySNES? A test that passes here and fails there (or vice versa) is a
-- finding either way.
--
-- Usage (from the repo root, after `make` in ref-proj/Mesen2):
--   dotnet ref-proj/Mesen2/bin/linux-x64/Release/linux-x64/publish/Mesen.dll \
--       --testrunner tests/roms/AccuracySNES/build/accuracysnes.sfc \
--       scripts/accuracysnes/mesen_crossval.lua --timeout=60
--
-- The process exit code is the number of FAILING scored tests (0 = full agreement), or 254 on
-- timeout. Per-test detail goes to the script log.
--
-- snesWorkRam offset 0 corresponds to $7E:0000, so the results block at $7E:F000 is offset $F000.

local BASE   = 0xF000
local MAGIC  = BASE + 0x00
local COUNT  = BASE + 0x06
local DONE   = BASE + 0x08
local STATUS = BASE + 0x20

local MAX_FRAMES = 900
local frames = 0

local function rd(a)
    return emu.read(a, emu.memType.snesWorkRam)
end

local function rd16(a)
    return rd(a) + rd(a + 1) * 256
end

local function onFrame()
    frames = frames + 1

    if rd(DONE) ~= 0xA5 then
        if frames > MAX_FRAMES then
            emu.log("ACCURACYSNES-TIMEOUT after " .. frames .. " frames")
            emu.stop(254)
        end
        return
    end

    -- Confirm we are reading a real block and not uninitialised WRAM.
    local magic = string.char(rd(MAGIC), rd(MAGIC + 1), rd(MAGIC + 2), rd(MAGIC + 3))
    if magic ~= "ACSN" then
        emu.log("ACCURACYSNES-BADMAGIC '" .. magic .. "'")
        emu.stop(253)
        return
    end

    local n = rd16(COUNT)
    local pass, fail, other = 0, 0, 0
    emu.log("ACCURACYSNES-BEGIN frames=" .. frames .. " count=" .. n)
    for i = 0, n - 1 do
        local b = rd(STATUS + i)
        local verdict
        if b == 0x00 then
            verdict = "NOTRUN"
            other = other + 1
        elseif b == 0xFF then
            verdict = "SKIP"
            other = other + 1
        elseif b % 2 == 1 then
            verdict = "PASS"
            if b ~= 0x01 then
                verdict = "PASS variant " .. math.floor(b / 2)
            end
            pass = pass + 1
        else
            verdict = "FAIL code " .. math.floor(b / 2)
            fail = fail + 1
        end
        emu.log(string.format("test %02d = %02X  %s", i, b, verdict))
    end
    emu.log("ACCURACYSNES-END pass=" .. pass .. " fail=" .. fail .. " other=" .. other)
    emu.stop(fail)
end

-- The host input contract (tests/roms/AccuracySNES/asm/runtime.inc, PAD_CONTRACT = $9050):
-- B + Start + X + R held on controller 1 for the whole run. Group F has no observable at all with
-- nothing held, so every runner holds the same mask and the cart asserts against it. Mesen2's docs
-- are explicit that setInput belongs in an inputPolled callback, since otherwise the state may not
-- be applied before the ROM reads it.
local function onInput()
    emu.setInput({ b = true, start = true, x = true, r = true,
                   y = false, select = false, a = false, l = false,
                   up = false, down = false, left = false, right = false }, 0)
end

emu.addEventCallback(onInput, emu.eventType.inputPolled)
emu.addEventCallback(onFrame, emu.eventType.endFrame)
