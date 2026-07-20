/*
 * AccuracySNES cross-validation host for any libretro SNES core.
 *
 * Why this exists: AccuracySNES is self-scoring and publishes its verdicts into WRAM, and
 * libretro exposes WRAM directly as RETRO_MEMORY_SYSTEM_RAM. So a ~200-line host can run the
 * cartridge on *any* libretro SNES core — bsnes, Mesen-S, snes9x — and read the same results
 * block the in-repo harness reads, with no GUI and no screen scraping.
 *
 * That matters because the alternative is circular: a test we wrote, grading an emulator we
 * wrote, proves nothing. Independent cores either agree with the cart's expected values or they
 * do not, and either answer is informative.
 *
 * Build:
 *   cc -O2 -o /tmp/lrcv scripts/accuracysnes/libretro_crossval.c -ldl
 * Run:
 *   /tmp/lrcv <core.so> <accuracysnes.sfc> [max_frames]
 *
 * Run (rendered-scene mode, ADR 0013):
 *   /tmp/lrcv <core.so> <accuracysnes.sfc> [max_frames] --scenes
 *
 * In --scenes mode the host keeps running past the battery, captures the framebuffer at the end of
 * each scene's hold, and prints one `scene<N>\t0x<hash>` line per scene. The hash is FNV-1a over a
 * fixed 256x224 region of canonical 0RRRRRGGGGGBBBBB pixels — fixed and canonical because emulators
 * do not agree about geometry or pixel format, and a golden must compare pictures rather than
 * output conventions.
 *
 * Exit code: number of FAILING scored tests, or 253 (bad magic) / 254 (timeout) / 255 (setup).
 */

#include <dlfcn.h>
#include <stdbool.h>
#include <stdint.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>

/* --- the slice of the libretro ABI we need ------------------------------------------------- */

#define RETRO_MEMORY_SYSTEM_RAM 2
#define RETRO_ENVIRONMENT_SET_PIXEL_FORMAT 10
#define RETRO_ENVIRONMENT_GET_SYSTEM_DIRECTORY 9
#define RETRO_ENVIRONMENT_GET_SAVE_DIRECTORY 31
#define RETRO_ENVIRONMENT_GET_LOG_INTERFACE 27
#define RETRO_ENVIRONMENT_GET_VARIABLE 15
#define RETRO_ENVIRONMENT_GET_VARIABLE_UPDATE 17
#define RETRO_ENVIRONMENT_SET_VARIABLES 16
#define RETRO_ENVIRONMENT_GET_CAN_DUPE 22

struct retro_game_info {
    const char *path;
    const void *data;
    size_t size;
    const char *meta;
};

typedef bool (*env_t)(unsigned cmd, void *data);
typedef void (*video_t)(const void *data, unsigned w, unsigned h, size_t pitch);
typedef void (*audio_t)(int16_t l, int16_t r);
typedef size_t (*audio_batch_t)(const int16_t *data, size_t frames);
typedef void (*input_poll_t)(void);
typedef int16_t (*input_state_t)(unsigned port, unsigned dev, unsigned idx, unsigned id);

static char sys_dir[] = ".";

/* --- framebuffer capture ------------------------------------------------------------------- */

#define SCENE_W 256u
#define SCENE_H 224u
/* The buffer row this host's picture starts on. An output convention, exactly like pixel format:
 * snes9x's libretro core already hands back the first visible line, RustySNES composites from
 * scanline 0, and Mesen2 starts 7 rows into a 239-row buffer. Calibrated by comparing renders —
 * with the wrong value two emulators that agree completely still produce different hashes. */
#define FIRST_ROW 0u

/* Pixel formats, as libretro numbers them. The default is 0RGB1555 and a core announces anything
 * else through SET_PIXEL_FORMAT, so the value has to be recorded rather than assumed: snes9x asks
 * for RGB565, and reading its output as 1555 silently shifts every channel. */
#define FMT_0RGB1555 0u
#define FMT_XRGB8888 1u
#define FMT_RGB565   2u

static unsigned pixel_format = FMT_0RGB1555;
static uint64_t last_frame_hash;
static bool last_frame_ok;
/* The canonical pixels behind `last_frame_hash`, kept so a disagreement can be diffed pixel by
 * pixel instead of only observed as two different 64-bit numbers. A hash says *that* two renders
 * differ; only the pixels say *where*, which is the difference between a finding and a shrug. */
static uint16_t last_frame_px[SCENE_W * SCENE_H];


static void log_stub(int level, const char *fmt, ...) { (void)level; (void)fmt; }
struct log_cb { void (*log)(int, const char *, ...); };
static struct log_cb logger = { log_stub };

static bool environment(unsigned cmd, void *data) {
    switch (cmd) {
    case RETRO_ENVIRONMENT_SET_PIXEL_FORMAT:
        pixel_format = *(const unsigned *)data;
        return true;
    case RETRO_ENVIRONMENT_GET_SYSTEM_DIRECTORY:
    case RETRO_ENVIRONMENT_GET_SAVE_DIRECTORY:
        *(const char **)data = sys_dir;
        return true;
    case RETRO_ENVIRONMENT_GET_LOG_INTERFACE:
        *(struct log_cb *)data = logger;
        return true;
    case RETRO_ENVIRONMENT_GET_CAN_DUPE:
        *(bool *)data = true;
        return true;
    default:
        return false;
    }
}

/* FNV-1a over canonical 0RRRRRGGGGGBBBBB pixels — the same value the Rust harness computes from
 * its own BGR555 framebuffer, so the two are directly comparable. */
static void video_refresh(const void *d, unsigned w, unsigned h, size_t p) {
    if (!d || w < SCENE_W || h < SCENE_H + FIRST_ROW) {
        /* A duped frame arrives as a NULL pointer; a hi-res or overscan frame is outside the
         * contract. Either way the previous hash stands rather than being silently replaced. */
        return;
    }
    uint64_t hash = 0xcbf29ce484222325ull;
    for (unsigned y = 0; y < SCENE_H; y++) {
        const uint8_t *row = (const uint8_t *)d + (size_t)(y + FIRST_ROW) * p;
        for (unsigned x = 0; x < SCENE_W; x++) {
            unsigned r, g, b;
            if (pixel_format == FMT_XRGB8888) {
                uint32_t v = ((const uint32_t *)row)[x];
                r = (v >> 19) & 0x1F; g = (v >> 11) & 0x1F; b = (v >> 3) & 0x1F;
            } else {
                uint16_t v = ((const uint16_t *)row)[x];
                if (pixel_format == FMT_RGB565) {
                    /* Green is 6 bits here because the core widened a 5-bit channel; dropping the
                     * low bit recovers the original rather than inventing precision. */
                    r = (v >> 11) & 0x1F; g = (v >> 6) & 0x1F; b = v & 0x1F;
                } else {
                    r = (v >> 10) & 0x1F; g = (v >> 5) & 0x1F; b = v & 0x1F;
                }
            }
            uint16_t canonical = (uint16_t)((r << 10) | (g << 5) | b);
            last_frame_px[y * SCENE_W + x] = canonical;
            hash ^= (uint64_t)canonical;
            hash *= 0x00000100000001b3ull;
        }
    }
    last_frame_hash = hash;
    last_frame_ok = true;
}
static void audio_sample(int16_t l, int16_t r) { (void)l; (void)r; }
static size_t audio_batch(const int16_t *d, size_t f) { (void)d; return f; }
static void input_poll(void) {}
static int16_t input_state(unsigned a, unsigned b, unsigned c, unsigned d) {
    (void)a; (void)b; (void)c; (void)d;
    return 0;
}

/* One typedef per entry point: a function-pointer type cannot be passed positionally to a
 * macro that declares `type name`, so each gets a name of its own. */
typedef void (*set_env_fn)(env_t);
typedef void (*set_video_fn)(video_t);
typedef void (*set_audio_fn)(audio_t);
typedef void (*set_audio_batch_fn)(audio_batch_t);
typedef void (*set_input_poll_fn)(input_poll_t);
typedef void (*set_input_state_fn)(input_state_t);
typedef void (*void_fn)(void);
typedef bool (*load_game_fn)(const struct retro_game_info *);
typedef void *(*mem_data_fn)(unsigned);
typedef size_t (*mem_size_fn)(unsigned);

#define SYM(name, type) type name = (type)dlsym(core, #name); \
    if (!name) { fprintf(stderr, "missing symbol %s\n", #name); return 255; }

/* --- results block layout, mirroring asm/runtime.inc --------------------------------------- */
#define BASE   0xF000u   /* $7E:F000, and WRAM offset 0 == $7E:0000 */
#define COUNT  (BASE + 0x06u)
#define DONE   (BASE + 0x08u)
#define STATUS (BASE + 0x20u)
/* The full-width measurement channel. A verdict byte cannot carry a dot count -- anything above
 * 255 wraps and becomes indistinguishable from a real reading -- so timing tests write here. */
#define MEAS   0xE200u
#define MEAS_SLOTS 128u
#define SCENE      (BASE + 0x12u)
#define SCENE_DONE (BASE + 0x13u)
#define MAX_SCENES 64u
/* Which frame of a scene's published window to hash, 1-based. Must match the in-repo harness. */
#define CAPTURE_SIGHTING 2u

int main(int argc, char **argv) {
    if (argc < 3) {
        fprintf(stderr, "usage: %s <core.so> <rom.sfc> [max_frames] [--scenes]\n", argv[0]);
        return 255;
    }
    bool want_scenes = false;
    unsigned fb_after = 0;
    const char *dump_prefix = NULL;
    unsigned max_frames = 1200;
    for (int i = 3; i < argc; i++) {
        if (strcmp(argv[i], "--scenes") == 0) {
            want_scenes = true;
        } else if (strncmp(argv[i], "--scene-dump=", 13) == 0) {
            want_scenes = true;
            dump_prefix = argv[i] + 13;
        } else if (strncmp(argv[i], "--fb-after=", 11) == 0) {
            fb_after = (unsigned)atoi(argv[i] + 11);
        } else {
            max_frames = (unsigned)atoi(argv[i]);
        }
    }

    /* --fb-after runs a plain ROM for N frames and prints the canonical framebuffer hash. It is
     * how a golden that was blessed from OUR OWN output gets an external opinion: the in-repo
     * undisbeliever goldens prove regression-freedom by construction and correctness not at all,
     * and this is the cheapest way to tell those two apart. */

    void *core = dlopen(argv[1], RTLD_LAZY);
    if (!core) {
        fprintf(stderr, "dlopen: %s\n", dlerror());
        return 255;
    }

    SYM(retro_set_environment, set_env_fn)
    SYM(retro_set_video_refresh, set_video_fn)
    SYM(retro_set_audio_sample, set_audio_fn)
    SYM(retro_set_audio_sample_batch, set_audio_batch_fn)
    SYM(retro_set_input_poll, set_input_poll_fn)
    SYM(retro_set_input_state, set_input_state_fn)
    SYM(retro_init, void_fn)
    SYM(retro_load_game, load_game_fn)
    SYM(retro_run, void_fn)
    SYM(retro_get_memory_data, mem_data_fn)
    SYM(retro_get_memory_size, mem_size_fn)
    SYM(retro_deinit, void_fn)

    retro_set_environment(environment);
    retro_set_video_refresh(video_refresh);
    retro_set_audio_sample(audio_sample);
    retro_set_audio_sample_batch(audio_batch);
    retro_set_input_poll(input_poll);
    retro_set_input_state(input_state);
    retro_init();

    FILE *f = fopen(argv[2], "rb");
    if (!f) { perror("rom"); return 255; }
    fseek(f, 0, SEEK_END);
    long len = ftell(f);
    fseek(f, 0, SEEK_SET);
    void *buf = malloc((size_t)len);
    if (!buf) { perror("malloc"); fclose(f); return 255; }
    if (fread(buf, 1, (size_t)len, f) != (size_t)len) {
        perror("read");
        fclose(f);
        free(buf);
        return 255;
    }
    fclose(f);

    struct retro_game_info info = { argv[2], buf, (size_t)len, NULL };
    if (!retro_load_game(&info)) {
        fprintf(stderr, "core refused the ROM\n");
        return 255;
    }

    uint8_t *wram = (uint8_t *)retro_get_memory_data(RETRO_MEMORY_SYSTEM_RAM);
    size_t wram_size = retro_get_memory_size(RETRO_MEMORY_SYSTEM_RAM);
    /* The SNES has 128 KiB of WRAM; require the whole thing rather than merely "enough", so a
     * core that under-reports cannot leave the results block partly outside the buffer. */
    if (!wram || wram_size < 0x20000) {
        fprintf(stderr, "core exposes no usable SYSTEM_RAM (%zu bytes, need 131072)\n", wram_size);
        free(buf);
        return 255;
    }
    printf("core=%s wram=%zu bytes\n", argv[1], wram_size);

    if (fb_after) {
        for (unsigned i = 0; i < fb_after; i++) {
            retro_run();
        }
        printf("FBHASH\t0x%016llx\n", (unsigned long long)last_frame_hash);
        retro_deinit();
        free(buf);
        return 0;
    }

    unsigned frame = 0;
    while (frame < max_frames && wram[DONE] != 0xA5) {
        retro_run();
        frame++;
    }

    if (wram[DONE] != 0xA5) {
        printf("ACCURACYSNES-TIMEOUT after %u frames\n", frame);
        retro_deinit();
        free(buf);
        return 254;
    }
    if (memcmp(wram + BASE, "ACSN", 4) != 0) {
        printf("ACCURACYSNES-BADMAGIC\n");
        retro_deinit();
        free(buf);
        return 253;
    }

    unsigned n = (unsigned)wram[COUNT] | ((unsigned)wram[COUNT + 1] << 8);
    /* `n` comes out of emulated WRAM: if the ROM hung or a core mis-mapped memory it can be
     * arbitrary, so bound it against the buffer before indexing with it. */
    if ((size_t)STATUS + n > wram_size) {
        fprintf(stderr, "implausible test count %u for %zu-byte WRAM\n", n, wram_size);
        retro_deinit();
        free(buf);
        return 253;
    }
    unsigned pass = 0, fail = 0, other = 0;
    printf("ACCURACYSNES-BEGIN frames=%u count=%u\n", frame, n);
    for (unsigned i = 0; i < n; i++) {
        uint8_t b = wram[STATUS + i];
        char detail[48];
        if (b == 0x00)        { other++; snprintf(detail, sizeof detail, "NOTRUN"); }
        else if (b == 0xFF)   { other++; snprintf(detail, sizeof detail, "SKIP"); }
        else if (b & 1)       { pass++;  snprintf(detail, sizeof detail, b == 1 ? "PASS" : "PASS variant %u", b >> 1); }
        else                  { fail++;  snprintf(detail, sizeof detail, "FAIL code %u", b >> 1); }
        printf("test %02u = %02X  %s\n", i, b, detail);
    }
    printf("ACCURACYSNES-END pass=%u fail=%u other=%u\n", pass, fail, other);

    /* Every measurement slot, so a golden-vector timing test can be compared across emulators
     * without this host having to know which slots any particular test owns. Zero slots are
     * skipped: nothing writes zero deliberately, and printing 128 lines of it helps no one. */
    printf("ACCURACYSNES-MEAS-BEGIN\n");
    for (unsigned i = 0; i < MEAS_SLOTS; i++) {
        unsigned v = (unsigned)wram[MEAS + i * 2] | ((unsigned)wram[MEAS + i * 2 + 1] << 8);
        if (v) {
            printf("meas %u\t%u\n", i, v);
        }
    }
    printf("ACCURACYSNES-MEAS-END\n");

    if (want_scenes) {
        /* The cart's scene loop runs after the battery: it sets up each scene, publishes the
         * 1-based scene ID, and holds it for a fixed number of frames. Overwriting on every frame
         * of a hold leaves the LAST frame's hash, which is the scene at its steady state. */
        static uint64_t hashes[MAX_SCENES];
        static bool got[MAX_SCENES];
        static unsigned sightings[MAX_SCENES];
        static uint16_t px[MAX_SCENES][SCENE_W * SCENE_H];
        unsigned scene_frames = 0;
        while (scene_frames < max_frames && wram[SCENE_DONE] != 0x5A) {
            retro_run();
            scene_frames++;
            unsigned id = wram[SCENE];
            if (id != 0 && id <= MAX_SCENES) {
                sightings[id - 1]++;
            }
            /* The SECOND frame of the published window, by agreement with the in-repo harness. A
             * host samples WRAM at its own frame boundary, which need not be the one the cart's
             * vblank poll sees, so both ends of the window are at risk of being off by one — this
             * host once captured scene 1 with a black band where forced blank was released. An
             * interior frame sidesteps that without the two clocks having to agree. */
            if (id != 0 && id <= MAX_SCENES && last_frame_ok
                && sightings[id - 1] == CAPTURE_SIGHTING && !got[id - 1]) {
                hashes[id - 1] = last_frame_hash;
                got[id - 1] = true;
                if (dump_prefix) {
                    memcpy(px[id - 1], last_frame_px, sizeof last_frame_px);
                }
            }
        }
        if (wram[SCENE_DONE] != 0x5A) {
            printf("ACCURACYSNES-SCENES-TIMEOUT after %u frames\n", scene_frames);
        } else {
            printf("ACCURACYSNES-SCENES-BEGIN frames=%u format=%u\n", scene_frames, pixel_format);
            for (unsigned i = 0; i < MAX_SCENES; i++) {
                if (!got[i]) {
                    continue;
                }
                printf("scene%u\t0x%016llx\n", i + 1, (unsigned long long)hashes[i]);
                if (dump_prefix) {
                    char path[512];
                    snprintf(path, sizeof path, "%s.scene%u.bin", dump_prefix, i + 1);
                    FILE *out = fopen(path, "wb");
                    if (out) {
                        fwrite(px[i], sizeof px[i], 1, out);
                        fclose(out);
                    } else {
                        fprintf(stderr, "cannot write %s\n", path);
                    }
                }
            }
            printf("ACCURACYSNES-SCENES-END\n");
        }
    }

    retro_deinit();
    free(buf);
    return (int)fail;
}
