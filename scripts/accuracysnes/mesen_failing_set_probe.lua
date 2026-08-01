-- Which rows does Mesen2 actually fail? Writes the SET, not the count.
--
-- `mesen_crossval.lua` reports only an exit code, and `emu.log` does not reach a file this
-- environment can find — so `MESEN2_KNOWN_FAILURES`'s per-row comment has never been checked against
-- a measurement. It had drifted: it named catalogue indices 279 and 286, which are now `F1.01` and
-- `F1.08`. This writes `index<TAB>byte` for every non-passing row to `$MESEN_FAIL_OUT`, and the
-- caller maps indices through `SOURCE_CATALOG.tsv` — the index is the unstable key, so the mapping
-- has to be done at read time rather than baked into a comment.
--
-- Read at `R_DONE == $A5`, never at a frame budget: after the battery the cart sits in its results
-- menu with Start held, where menu actions move the very bytes this reads. Probing at a fixed frame
-- count produced a different failing set each time and cost a published-then-retracted claim.
--
-- Usage:
--   MESEN_FAIL_OUT=/tmp/mesen_fail.tsv dotnet <Mesen.dll> --testrunner <rom> \
--       scripts/accuracysnes/mesen_failing_set_probe.lua --timeout=60

local BASE   = 0xF000
local COUNT  = BASE + 0x06
local DONE   = BASE + 0x08
local STATUS = BASE + 0x20

local OUT = os.getenv("MESEN_FAIL_OUT") or "/tmp/mesen_fail.tsv"
local MAX_FRAMES = 2000
local frames = 0

local function rd(off)
    return emu.read(off, emu.memType.snesWorkRam, false)
end

local function rd16(off)
    return rd(off) | (rd(off + 1) << 8)
end

local function onFrame()
    frames = frames + 1
    if frames > MAX_FRAMES then
        local f = io.open(OUT, "w")
        if f then f:write("# timed out before R_DONE\n"); f:close() end
        emu.stop(254)
        return
    end
    if rd(DONE) ~= 0xA5 then return end

    local n = rd16(COUNT)
    local f = io.open(OUT, "w")
    if not f then emu.stop(253) return end
    f:write(string.format("# frames=%d count=%d\n", frames, n))
    local fail = 0
    for i = 0, n - 1 do
        local b = rd(STATUS + i)
        -- odd = pass (odd and not 1 = "pass variant"), $FF = skip, even = fail code b/2, 0 = not run
        if b ~= 0xFF and b % 2 == 0 then
            f:write(string.format("%d\t%02X\n", i, b))
            fail = fail + 1
        end
    end
    f:close()
    emu.stop(fail)
end

-- The input contract, exactly ONE setInput call: in this build the port argument does not select a
-- controller, so a second call for port 2 overwrites port 1 with a mask containing no Start and the
-- cart never leaves its pre-battery menu. See mesen_crossval.lua for the full account.
local function onInput()
    emu.setInput({ b = true, start = true, x = true, r = true }, 0)
end

emu.addEventCallback(onFrame, emu.eventType.endFrame)
emu.addEventCallback(onInput, emu.eventType.inputPolled)
