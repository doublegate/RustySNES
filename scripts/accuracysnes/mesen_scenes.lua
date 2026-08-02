-- AccuracySNES rendered-scene capture for Mesen2's headless test runner (ADR 0013).
--
-- Companion to mesen_crossval.lua, which grades the self-scoring battery. This one covers what the
-- cart structurally cannot score: the part of the PPU that decides only what appears on screen.
-- The cart renders; a host judges. Mesen2 is the third opinion — with only RustySNES and snes9x, a
-- disagreement has no majority and no scene can be blessed.
--
-- Usage (from the repo root):
--   dotnet ref-proj/Mesen2/bin/linux-x64/Release/linux-x64/publish/Mesen.dll \
--       --testrunner tests/roms/AccuracySNES/build/accuracysnes.sfc \
--       scripts/accuracysnes/mesen_scenes.lua --timeout=120
--
-- Prints one `scene<N>\t0x<hash>` line per scene on stdout (`print`, not `emu.log` — the test
-- runner does not surface the script log). The hash must be computed identically to the
-- other two hosts or nothing is comparable: FNV-1a over a fixed 256x224 region of canonical
-- 0RRRRRGGGGGBBBBB pixels. Fixed and canonical because emulators do not agree about geometry or
-- pixel format, and a golden must compare pictures rather than output conventions.

local BASE       = 0xF000
local DONE       = BASE + 0x08
local SCENE      = BASE + 0x12
local SCENE_DONE = BASE + 0x13

local SCENE_W = 256
local SCENE_H = 224

-- The EXACT length `emu.getScreenBuffer()` returns for an in-contract frame: 256 x 239, the whole
-- output frame, of which the hash reads `SCENE_H` rows starting at `FIRST_ROW`. MEASURED, not
-- derived -- the first attempt at this check computed `SCENE_W * (SCENE_H + FIRST_ROW)` = 59136,
-- which is the size of the region READ rather than of the buffer returned, and rejected every
-- frame.
--
-- Exact, not a lower bound, and that is the point. A lower bound catches a buffer that is too small
-- and misses one that is too LARGE -- and too large is the case that actually happens: a hi-res
-- frame comes back 512 across, the old `#buf < ...` test passed it, and the loop below then walked
-- it with a stride of 256, hashing a diagonal slice of the picture while reporting success. A
-- golden blessed from that would be stable, reproducible and wrong. Widening the region past 256 is
-- real planned work; it has to start from a loud rejection here, not a silent wrong picture.
local SCENE_BUF_LEN = SCENE_W * 239

-- The buffer row Mesen2's picture starts on. An output convention, exactly like pixel format:
-- Mesen hands back 256x239 whose picture begins 7 rows in, snes9x's libretro core already starts
-- at the first visible line, and RustySNES composites from scanline 0. Calibrated by comparing
-- renders — with the wrong value two emulators that agree completely still produce different
-- hashes, which is what made the first three-way comparison look like a triple disagreement.
local FIRST_ROW = 7

-- Which frame of a scene's published window to hash, 1-based. Must match the other hosts: the
-- cart publishes an ID only once the scene has settled and clears it before anything disturbs it,
-- but each host samples at its own frame boundary, so both ends of the window can be off by one.
local CAPTURE_SIGHTING = 4

-- Raised from 2000 when the cartridge grew to 256 KiB: G1.11 walks the whole image byte by byte,
-- so it doubled with the image and the battery no longer finished before this budget ran out. The
-- failure looked like "0 scenes match", not like a timeout, because the run never reached the
-- scene loop at all -- worth knowing, since that reads as a mismatch in the gate's output.
-- The in-repo harness's MAX_FRAMES and libretro_crossval.c's max_frames bound the same run.
local MAX_FRAMES = 4000
local frames = 0
local battery_done = false
local reported = false
local sightings = {}
local hashes = {}
local order = {}

local function rd(a)
    return emu.read(a, emu.memType.snesWorkRam)
end

-- FNV-1a, 64-bit. Lua 5.4 integers are 64-bit and wrap on overflow, which is exactly the
-- arithmetic the C and Rust hosts perform.
local FNV_PRIME = 0x100000001b3

-- Optional pixel dump, mirroring `--scene-dump=` in the libretro host and ACCURACYSNES_SCENE_DUMP
-- in the in-repo harness. Set MESEN_SCENE_DUMP to a path prefix. When three hosts disagree the
-- hashes say only *that* they differ; only the pixels say where.
local dump_prefix = os.getenv and os.getenv("MESEN_SCENE_DUMP") or nil

local function dumpScene(id, px)
    if not dump_prefix then
        return
    end
    local f = io.open(string.format("%s.scene%d.bin", dump_prefix, id), "wb")
    if not f then
        return
    end
    local out = {}
    for i = 1, #px do
        local v = px[i]
        out[i] = string.char(v & 0xFF, (v >> 8) & 0xFF)
    end
    f:write(table.concat(out))
    f:close()
end

local function hashFrame(id)
    local buf = emu.getScreenBuffer()
    -- Mesen composites 256 pixels wide at 1x; anything else means a hi-res or filtered frame, and
    -- the region contract no longer holds. Say so rather than hashing something else.
    --
    -- The check is an EXACT length, not a lower bound, and that is the whole point. A lower bound
    -- catches a buffer that is too small and misses one that is too LARGE -- and too large is the
    -- case that actually happens: a hi-res frame comes back 512 wide, `#buf < 256 * 231` is false,
    -- and the loop below then walks it with a stride of 256, hashing a diagonal slice of the
    -- picture while reporting success. A golden blessed from that would be stable, reproducible
    -- and wrong. Measured 2026-08-01: every current scene returns exactly 61184 = 256 x 239.
    local width = SCENE_W
    if #buf ~= SCENE_BUF_LEN then
        print("ACCURACYSNES-SCENES-BADGEOMETRY " .. #buf .. " expected " .. SCENE_BUF_LEN)
        return nil
    end
    local h = 0xcbf29ce484222325
    local px = {}
    for y = 0, SCENE_H - 1 do
        local row = (y + FIRST_ROW) * width
        for x = 0, SCENE_W - 1 do
            local v = buf[row + x + 1]
            -- Mesen hands back 24-bit RGB; the SNES channels are 5-bit widened to 8, so the top
            -- five bits recover the original rather than inventing precision.
            local r = (v >> 19) & 0x1F
            local g = (v >> 11) & 0x1F
            local b = (v >> 3) & 0x1F
            local canonical = (r << 10) | (g << 5) | b
            px[#px + 1] = canonical
            h = h ~ canonical
            h = h * FNV_PRIME
        end
    end
    dumpScene(id, px)
    return h
end

local function onFrame()
    frames = frames + 1

    if not battery_done then
        battery_done = rd(DONE) == 0xA5
        if not battery_done and frames > MAX_FRAMES then
            print("ACCURACYSNES-TIMEOUT after " .. frames .. " frames")
            emu.stop(254)
        end
        return
    end

    local id = rd(SCENE)
    if id ~= 0 then
        sightings[id] = (sightings[id] or 0) + 1
        if sightings[id] == CAPTURE_SIGHTING and hashes[id] == nil then
            local h = hashFrame(id)
            if h ~= nil then
                hashes[id] = h
                order[#order + 1] = id
            end
        end
    end

    -- `emu.stop` does not take effect immediately, so without this guard the report block runs
    -- again on the next frame and prints the whole list twice. It stayed hidden while the battery
    -- was short enough that only one frame elapsed; a longer battery made it two, and the
    -- duplicate list read as a scene mismatch rather than as a duplicated report.
    if rd(SCENE_DONE) == 0x5A and not reported then
        reported = true
        print("ACCURACYSNES-SCENES-BEGIN frames=" .. frames)
        table.sort(order)
        for _, i in ipairs(order) do
            print(string.format("scene%d\t0x%016x", i, hashes[i]))
        end
        print("ACCURACYSNES-SCENES-END")
        emu.stop(0)
    end

    if frames > MAX_FRAMES then
        print("ACCURACYSNES-SCENES-TIMEOUT after " .. frames .. " frames")
        emu.stop(254)
    end
end

-- The host input contract, which this script was missing ENTIRELY -- which is why it reported "no
-- scenes at all": with nothing held the cart never leaves its pre-battery menu (that menu waits for
-- Start), so it never runs the battery and never reaches the scene loop that follows it. The symptom
-- read like a scene-loop problem and was an input problem, the same one `mesen_crossval.lua` had in
-- a different form.
--
-- ONE call, port index 0. In this MesenCE build's --testRunner the port argument does not select a
-- controller -- 0, 1 and 2 all land on controller 1 -- so a second call for port 2 would OVERWRITE
-- this one with PAD2_CONTRACT ($60A0), which contains no Start. See `mesen_crossval.lua`'s comment
-- for the full account; do not add a port-2 call here without first proving the argument works.
local function onInput()
    emu.setInput({ b = true, start = true, x = true, r = true,
                   y = false, select = false, a = false, l = false,
                   up = false, down = false, left = false, right = false }, 0)
end

emu.addEventCallback(onInput, emu.eventType.inputPolled)
emu.addEventCallback(onFrame, emu.eventType.endFrame)
