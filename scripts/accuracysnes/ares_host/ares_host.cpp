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
  string rom = arguments[0];
  u32 budget = (u32)toNatural(arguments[1]);

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

  ares::Node::System root;
  if(!ares::SuperFamicom::load(root, "[Nintendo] Super Famicom (NTSC)")) {
    fprintf(stderr, "ares_host: ares::SuperFamicom::load failed\n");
    exit(3);
  }
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
  root->power();

  while(platform_.frames < budget) root->run();

  // The results block, straight out of WRAM. $7E:F000 is WRAM offset $F000.
  const auto& wram = ares::SuperFamicom::cpu.wram;
  const u32 RESULTS = 0xF000;
  printf("ACCURACYSNES-BEGIN\n");
  printf("magic %c%c%c%c\n", wram[RESULTS], wram[RESULTS + 1], wram[RESULTS + 2], wram[RESULTS + 3]);
  printf("done %02x\n", wram[RESULTS + 0x08]);
  u32 count = wram[RESULTS + 0x0A] | (wram[RESULTS + 0x0B] << 8);
  printf("count %u\n", count);
  for(u32 i = 0; i < count && i < 512; i++) {
    printf("status %u %02x\n", i, wram[RESULTS + 0x20 + i]);
  }
  printf("ACCURACYSNES-END\n");
}
}
