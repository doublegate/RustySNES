// Headless ares host for the AccuracySNES cartridge — the third opinion the project has been
// blocked on. Reads the cart's results block out of WRAM and prints it in the same shape the
// snes9x libretro host does, so `crossval.sh` can consume it.
#include <ares/ares.hpp>
#include <sfc/sfc.hpp>
#include <mia/mia.hpp>
#include <nall/main.hpp>

#include <cstdio>
#include <cstring>
#include <cstdlib>
#include <csignal>
#include <execinfo.h>
#include <unistd.h>

namespace {

// A crash here is a setup mistake in this file, not an ares bug, and this sandbox cannot run a
// debugger (`ptrace` is denied), so the backtrace has to come from inside the process. Build with
// `-rdynamic` — `build.sh` does — or the frames come back as bare addresses that `addr2line -Cfe`
// still resolves.
auto crashHandler(int sig) -> void {
  void* frames[40];
  int n = backtrace(frames, 40);
  fprintf(stderr, "\nares_host: signal %d — backtrace follows\n", sig);
  backtrace_symbols_fd(frames, n, 2);
  _exit(9);
}

}  // namespace

namespace {

// The controller mask every AccuracySNES runner holds for the whole run (`asm/runtime.inc`).
constexpr u16 PAD_CONTRACT = 0x9050;   // B + Start + X + R on controller 1
constexpr u16 PAD2_CONTRACT = 0x60A0;  // Y + Select + A + L on controller 2

// $4218 bit order, which is what the contract masks are expressed in.
auto held(u16 mask, const string& name) -> bool {
  if(name == "B")      return mask & 0x8000;
  if(name == "Y")      return mask & 0x4000;
  if(name == "Select") return mask & 0x2000;
  if(name == "Start")  return mask & 0x1000;
  if(name == "Up")     return mask & 0x0800;
  if(name == "Down")   return mask & 0x0400;
  if(name == "Left")   return mask & 0x0200;
  if(name == "Right")  return mask & 0x0100;
  if(name == "A")      return mask & 0x0080;
  if(name == "X")      return mask & 0x0040;
  if(name == "L")      return mask & 0x0020;
  if(name == "R")      return mask & 0x0010;
  return false;
}

struct Headless : ares::Platform {
  std::shared_ptr<mia::Pak> system, game;
  u32 frames = 0;

  auto pak(ares::Node::Object node) -> std::shared_ptr<vfs::directory> override {
    if(node->name() == "Super Famicom") return system->pak;
    if(node->name() == "Super Famicom Cartridge") return game->pak;
    return {};
  }

  auto video(ares::Node::Video::Screen, const u32*, u32, u32, u32) -> void override {
    frames++;
  }

  auto input(ares::Node::Input::Input input) -> void override {
    // Which port a button belongs to is read from its ancestry, the same way desktop-ui does it.
    if(auto button = input->cast<ares::Node::Input::Button>()) {
      u16 mask = PAD_CONTRACT;
      auto wp = button->parent();
      if(!wp.expired()) {
        if(auto device = wp.lock()) {
          auto wpp = device->parent();
          if(!wpp.expired()) {
            if(auto port = wpp.lock()) {
              if(port->name().find("2")) mask = PAD2_CONTRACT;
            }
          }
        }
      }
      button->setValue(held(mask, button->name()));
    }
  }
};

Headless platform_;

}  // namespace

// nall owns `main` and calls this. Exit codes go through `exit()` rather than a return value.
namespace nall {
auto main(Arguments arguments) -> void {
  if(arguments.size() < 2) {
    printf("usage: ares_host <rom.sfc> <frames>\n");
    exit(2);
  }
  bool verbose = (bool)arguments.take("--verbose");
  string rom = arguments[0];
  u32 budget = (u32)toNatural(arguments[1]);

  // MUST come before anything touches a core. `Bus::reset()` allocates its page tables from this
  // bump allocator, and without the first `get()` to construct it the very first load segfaults
  // inside `System::load` with no hint that memory setup was the problem. desktop-ui does the same
  // thing on its first line for the same reason.
  signal(SIGSEGV, crashHandler);
  signal(SIGABRT, crashHandler);
  ares::Memory::FixedAllocator::get();
  ares::platform = &platform_;
  mia::setHomeLocation([]() -> string { return {Path::userData(), "ares/"}; });

  platform_.game = mia::Medium::create("Super Famicom");
  if(platform_.game->load(rom) != successful) {
    fprintf(stderr, "ares_host: could not load %s\n", (const char*)rom);
    exit(3);
  }
  platform_.system = mia::System::create("Super Famicom");
  if(platform_.system->load() != successful) {
    fprintf(stderr, "ares_host: could not load the Super Famicom system pak\n");
    exit(3);
  }

  if(verbose) fprintf(stderr, "ares_host: loaded the game and system paks\n");

  // MUST come before `load`. `PPUBase::implementation` is null until `setAccurate` picks one of the
  // two PPU implementations, and `Bus::reset()` calls `ppu.map()` -> `implementation->map()` during
  // `System::load`. Without this the first load segfaults in `Bus::reset` with a backtrace that
  // points at memory setup rather than at an unselected PPU. desktop-ui passes its own setting here.
  // "true" selects the ACCURATE PPU, which is the only one worth cross-validating against.
  ares::SuperFamicom::option("Pixel Accuracy", "true");

  ares::Node::System root;
  if(!ares::SuperFamicom::load(root, "[Nintendo] Super Famicom (NTSC)")) {
    fprintf(stderr, "ares_host: ares::SuperFamicom::load failed\n");
    exit(3);
  }
  if(verbose) fprintf(stderr, "ares_host: ares::SuperFamicom::load returned\n");
  if(auto port = root->find<ares::Node::Port>("Cartridge Slot")) {
    port->allocate();
    port->connect();
  }
  for(auto name : {"Controller Port 1", "Controller Port 2"}) {
    if(auto port = root->find<ares::Node::Port>(name)) {
      port->allocate("Gamepad");
      port->connect();
    }
  }
  if(verbose) fprintf(stderr, "ares_host: ports connected\n");
  root->power();

  if(verbose) fprintf(stderr, "ares_host: powered on\n");
  while(platform_.frames < budget) root->run();

  // The results block, straight out of WRAM. $7E:F000 is WRAM offset $F000.
  const auto& wram = ares::SuperFamicom::cpu.wram;
  const u32 RESULTS = 0xF000;
  // Offsets are `asm/runtime.inc`'s, and they are easy to get wrong by one field: R_COUNT is +$06
  // and R_PASSED is +$0A. Reading the latter as the former reported "count 299" for a 338-test
  // battery, which looks like a truncated run rather than a misread field.
  auto rd16 = [&](u32 off) -> u32 { return wram[RESULTS + off] | (wram[RESULTS + off + 1] << 8); };
  u32 count = rd16(0x06);
  printf("ACCURACYSNES-BEGIN\n");
  printf("magic %c%c%c%c\n", wram[RESULTS], wram[RESULTS + 1], wram[RESULTS + 2], wram[RESULTS + 3]);
  printf("done %02x\n", wram[RESULTS + 0x08]);
  printf("count %u\n", count);
  printf("passed %u\n", rd16(0x0A));
  printf("failed %u\n", rd16(0x0C));
  printf("skipped %u\n", rd16(0x0E));
  printf("golden %u\n", rd16(0x10));
  for(u32 i = 0; i < count && i < 512; i++) {
    printf("status %u %02x\n", i, wram[RESULTS + 0x20 + i]);
  }
  printf("ACCURACYSNES-END\n");
}
}
