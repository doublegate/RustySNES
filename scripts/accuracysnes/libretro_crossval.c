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

static void log_stub(int level, const char *fmt, ...) { (void)level; (void)fmt; }
struct log_cb { void (*log)(int, const char *, ...); };
static struct log_cb logger = { log_stub };

static bool environment(unsigned cmd, void *data) {
    switch (cmd) {
    case RETRO_ENVIRONMENT_SET_PIXEL_FORMAT:
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

static void video_refresh(const void *d, unsigned w, unsigned h, size_t p) {
    (void)d; (void)w; (void)h; (void)p;
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

int main(int argc, char **argv) {
    if (argc < 3) {
        fprintf(stderr, "usage: %s <core.so> <rom.sfc> [max_frames]\n", argv[0]);
        return 255;
    }
    unsigned max_frames = (argc > 3) ? (unsigned)atoi(argv[3]) : 1200;

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
    if (fread(buf, 1, (size_t)len, f) != (size_t)len) { perror("read"); return 255; }
    fclose(f);

    struct retro_game_info info = { argv[2], buf, (size_t)len, NULL };
    if (!retro_load_game(&info)) {
        fprintf(stderr, "core refused the ROM\n");
        return 255;
    }

    uint8_t *wram = (uint8_t *)retro_get_memory_data(RETRO_MEMORY_SYSTEM_RAM);
    size_t wram_size = retro_get_memory_size(RETRO_MEMORY_SYSTEM_RAM);
    if (!wram || wram_size < 0x10000) {
        fprintf(stderr, "core exposes no usable SYSTEM_RAM (%zu bytes)\n", wram_size);
        return 255;
    }
    printf("core=%s wram=%zu bytes\n", argv[1], wram_size);

    unsigned frame = 0;
    while (frame < max_frames && wram[DONE] != 0xA5) {
        retro_run();
        frame++;
    }

    if (wram[DONE] != 0xA5) {
        printf("ACCURACYSNES-TIMEOUT after %u frames\n", frame);
        retro_deinit();
        return 254;
    }
    if (memcmp(wram + BASE, "ACSN", 4) != 0) {
        printf("ACCURACYSNES-BADMAGIC\n");
        retro_deinit();
        return 253;
    }

    unsigned n = (unsigned)wram[COUNT] | ((unsigned)wram[COUNT + 1] << 8);
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

    retro_deinit();
    free(buf);
    return (int)fail;
}
