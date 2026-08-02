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

-- The catalogue index of the row excluded from the failing count, supplied by the caller because
-- the index MOVES whenever a test is added ahead of it -- which is exactly the circumstance that
-- exposed the problem. -1 (the default) excludes nothing.
local SKIP_INDEX = tonumber(os.getenv("ACCURACYSNES_SKIP_INDEX") or "-1") or -1

-- Bounds the same run as the in-repo harness's MAX_FRAMES (1500), mesen_scenes.lua's (4000) and
-- libretro_crossval.c's max_frames (2000). This one was left at 900 when the others grew -- the
-- harness comment naming the budgets that must move together does not list this file, which is how
-- it was missed. It is NOT why this runner times out (4000 fails identically); see
-- docs/accuracysnes-plan.md on the Mesen2 oracle.
local MAX_FRAMES = 4000
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
        -- One row is EXCLUDED from the returned count, by index passed in from the caller, which
        -- resolves it from SOURCE_CATALOG.tsv by ID. F1.10 samples $4212 right at the vblank edge,
        -- so the cart's execution phase decides it and ANY row added ahead of Group F moves that
        -- phase. Measured: adding B2.07 made this host's PAL verdict on it flip between runs of a
        -- single build. The same exclusion is applied to the ares gate for the same reason; see
        -- crossval.sh's F1.10 entry. It is still logged, just not counted.
        if i == SKIP_INDEX then
            emu.log(string.format("test %02d = %02X  EXCLUDED (phase-fragile row)", i, b))
            goto continue
        end
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
        ::continue::
    end
    emu.log("ACCURACYSNES-END pass=" .. pass .. " fail=" .. fail .. " other=" .. other)
    emu.stop(fail)
end

-- The host input contract (tests/roms/AccuracySNES/asm/runtime.inc, PAD_CONTRACT = $9050):
-- B + Start + X + R held on controller 1 for the whole run. Group F has no observable at all with
-- nothing held, so every runner holds the same mask and the cart asserts against it. Mesen2's docs
-- are explicit that setInput belongs in an inputPolled callback, since otherwise the state may not
-- be applied before the ROM reads it.
--
-- ONE setInput call, deliberately. This is what made the battery never run under MesenCE for
-- several sessions, and it is worth stating exactly, because the failure was silent and total:
--
--   In this MesenCE build's --testRunner, the port argument does NOT select a controller. Sending
--   PAD_CONTRACT to index 0 and again to index 1 both land on CONTROLLER 1 (verified: the cart's own
--   V_PAD_HELD reads $9050 either way; sending to index 2 does the same). So the second call here --
--   intended for port 2 -- OVERWROTE port 1, and the cart saw PAD2_CONTRACT ($60A0) on controller 1.
--
--   $60A0 contains no Start. The pre-battery menu waits for Start. So the cart booted, ran its init,
--   cleared R_STATUS, reached the menu and sat there forever -- with nothing on screen to show it,
--   since the menu is a static picture. Every symptom followed from that: no ACSN magic, R_DONE
--   never $A5, and an all-zero status array that earlier notes misread as "completes 14 of 335".
--
-- With the single call the battery runs to completion: magic ACSN, R_DONE $A5, 335/335 status bytes
-- written. Do not "restore" the port-2 call without first proving the port argument works.
--
-- CONSEQUENCE, stated rather than hidden: port 2 cannot be driven from Lua in this build, so rows
-- that depend on PAD2_CONTRACT ($60A0: Y + Select + A + L) are not cross-validated by this runner.
-- The in-repo harness and the snes9x libretro driver do drive both ports, so those rows are still
-- covered -- just not by MesenCE.
local function onInput()
    emu.setInput({ b = true, start = true, x = true, r = true,
                   y = false, select = false, a = false, l = false,
                   up = false, down = false, left = false, right = false }, 0)
end

emu.addEventCallback(onInput, emu.eventType.inputPolled)
emu.addEventCallback(onFrame, emu.eventType.endFrame)
