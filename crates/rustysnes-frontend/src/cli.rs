//! Native CLI (clap 4) + the structured help-topic registry.
//!
//! - `rustysnes <ROM>` loads + runs the ROM (positional).
//! - `rustysnes` with no ROM opens the menu shell (load via File → Open / drag-and-drop).
//! - `rustysnes help [<topic>]` prints a help topic; `--interactive` opens the ratatui browser
//!   (behind the `help-tui` feature, on a TTY).
//! - `rustysnes completions <shell>` prints a shell-completion script.
//! - a bad argument exits with clap's default usage-error code (2).
//!
//! **Native-only.** The wasm entry point is an empty shim (a browser tab has no terminal); the
//! clap / ratatui dep cluster is gated out of the wasm target in `Cargo.toml`. Zero determinism
//! surface — everything here runs before any emulation.
//!
//! `TOPICS`/`topic_text` are the single content source shared by the static `help <topic>` page
//! and the interactive ratatui browser (`help_tui.rs`) — the two surfaces can never drift, since
//! the TUI just iterates `TOPICS` and calls `topic_text` for the selected one.

use std::path::PathBuf;

use clap::builder::styling::{AnsiColor, Effects, Styles};
use clap::{Parser, Subcommand, ValueEnum};

/// The clap colour palette for the help / usage output.
#[must_use]
pub fn cli_styles() -> Styles {
    Styles::styled()
        .header(AnsiColor::Green.on_default() | Effects::BOLD)
        .usage(AnsiColor::Green.on_default() | Effects::BOLD)
        .literal(AnsiColor::Cyan.on_default() | Effects::BOLD)
        .placeholder(AnsiColor::Cyan.on_default())
        .valid(AnsiColor::Green.on_default())
        .invalid(AnsiColor::Yellow.on_default())
        .error(AnsiColor::Red.on_default() | Effects::BOLD)
}

const AFTER_HELP: &str = "\
Examples:
  rustysnes game.sfc           Load and run a ROM
  rustysnes help controls      Show the keyboard/gamepad reference
  rustysnes help               Browse all help topics (interactive on a TTY)
  rustysnes completions fish   Print a shell-completion script

Keyboard (P1, rebindable in Settings -> Input):
  Arrows D-pad   X=A  Z=B  S=X  A=Y   Q=L  W=R   RShift=Select  Enter=Start

Global hotkeys (v1.0.1): Esc=Quit  F1=Save  F2=Reset  F3=Power-Cycle  F4=Load
F5=Rewind  F9=Save States  F11=Fullscreen  F12=Open ROM  Space=Pause -- see
`rustysnes help hotkeys`.

See `rustysnes help <topic>` for: controls, hotkeys, gamepad, features, coprocessors, config,
scripting, netplay, about.";

/// `RustySNES` — a cycle-accurate Super Nintendo Entertainment System emulator.
#[derive(Debug, Parser)]
#[command(
    name = "rustysnes",
    bin_name = "rustysnes",
    version,
    author,
    about = "RustySNES — a cycle-accurate SNES / Super Famicom emulator (winit + wgpu + cpal + egui).",
    long_about = "RustySNES — a cycle-accurate Super Nintendo Entertainment System emulator \
                  written in pure Rust, targeting the Mesen2 / higan / ares accuracy bar.\n\n\
                  Pass a ROM path to load and run it. Once a session is open you can load \
                  further ROMs from the File menu.",
    after_help = AFTER_HELP,
    styles = cli_styles(),
    disable_help_subcommand = true,
)]
pub struct Cli {
    /// Path to the `.sfc` / `.smc` ROM to load and run (zip-archived ROMs are transparently
    /// extracted). Load further ROMs from the File menu or by drag-and-drop once a session is
    /// open.
    #[arg(value_name = "ROM", value_hint = clap::ValueHint::FilePath)]
    pub rom: Option<PathBuf>,

    /// Control when colored output is used (also honours `NO_COLOR`).
    #[arg(long, value_name = "WHEN", value_enum, default_value_t = ColorWhen::Auto, global = true)]
    pub color: ColorWhen,

    /// Subcommands: `help [<topic>]`, `completions <shell>`.
    #[command(subcommand)]
    pub command: Option<CliCommand>,
}

/// When to color the CLI output.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, ValueEnum)]
pub enum ColorWhen {
    /// Color when stdout is a TTY (default).
    #[default]
    Auto,
    /// Always color.
    Always,
    /// Never color.
    Never,
}

impl From<ColorWhen> for clap::ColorChoice {
    fn from(w: ColorWhen) -> Self {
        match w {
            ColorWhen::Auto => Self::Auto,
            ColorWhen::Always => Self::Always,
            ColorWhen::Never => Self::Never,
        }
    }
}

/// The CLI subcommands.
#[derive(Debug, Subcommand)]
pub enum CliCommand {
    /// Show a help topic (or browse interactively on a TTY).
    Help {
        /// The topic name (see [`topic_text`] for the registry). Omit to list all topics.
        topic: Option<String>,
        /// Open the interactive ratatui browser instead of printing (needs `help-tui` + a TTY).
        #[arg(long, short)]
        interactive: bool,
    },
    /// Print a shell-completion script for the given shell.
    Completions {
        /// The target shell.
        shell: clap_complete::Shell,
    },
}

/// The structured help-topic registry. Returns the body text for `topic`, or `None` if unknown.
/// Keeping it a plain function (not a macro table) makes the SNES content obvious + testable.
///
/// Every body below describes ONLY behavior that is actually wired today — a topic that named an
/// unimplemented shortcut or feature would be exactly the "silently claims support that doesn't
/// exist" anti-pattern this project's accuracy-tiering/honesty-gate posture (`docs/adr/0003`)
/// exists to avoid everywhere else.
// One long match of literal topic bodies; the line count is inherent to having 9 topics of
// real content, not a sign this needs splitting into 9 near-identical one-arm functions.
#[allow(clippy::too_many_lines)]
#[must_use]
pub fn topic_text(topic: &str) -> Option<&'static str> {
    Some(match topic {
        "controls" => {
            "\
Default keyboard controls (P1)
===============================

  D-pad ............. Arrow keys
  A ................. X
  B ................. Z
  X ................. S
  Y ................. A
  L ................. Q
  R ................. W
  Select ............ Right Shift
  Start ............. Enter

All bindings are rebindable in Settings -> Input: click \"Rebind\" next to a
button, then press the new key (Esc cancels). Rebinds persist to config.toml
(auto-saved) and survive a restart.

Run `rustysnes help hotkeys` for system/emulation controls, or
`rustysnes help gamepad` for controller bindings."
        }
        "hotkeys" => {
            "\
System & emulation controls
============================

Global keyboard hotkeys (v1.0.1) work anywhere the window has focus, and are
suppressed while a text field (e.g. Settings) has keyboard focus so typing
never triggers them:

  Escape ....... Quit
  F1 ........... Save State (quick slot)
  F2 ........... Reset
  F3 ........... Power Cycle
  F4 ........... Load State (quick slot)
  F5 ........... Rewind
  F9 ........... Save States... (10-slot thumbnail manager)
  F11 .......... Toggle Fullscreen
  F12 .......... Open ROM
  Space ........ Pause/Resume
  ` (Backquote)  Toggle Debugger overlay (feature-gated: debug-hooks)

Every action above is also reachable from the menu bar, which remains the
authoritative reference if a binding above ever looks stale:

  File ....... Open ROM, Close ROM, Settings, Quit
  Emulation .. Pause/Resume, Reset, Power Cycle, Save/Load State (quick slot),
               Rewind, Save States... (10-slot thumbnail manager), Region,
               Speed (25%-300% presets)
  Tools ...... Cheats / Netplay / RetroAchievements windows (feature-gated)
  View ....... Integer scale, Performance panel, Fullscreen
  Debug ...... Debugger overlay (feature-gated: debug-hooks)"
        }
        "gamepad" => {
            "\
Gamepad support
================

USB/Bluetooth gamepads (via gilrs) auto-bind to P1, Xbox-style layout:

  South (A) ......... SNES B   (bottom face button)
  East (B) .......... SNES A   (right face button)
  West (X) .......... SNES Y   (left face button)
  North (Y) ......... SNES X   (top face button)
  LeftTrigger ....... SNES L
  RightTrigger ...... SNES R
  Start ............. Start
  Select/Back ....... Select
  D-pad ............. D-pad

The SNES diamond is rotated relative to Xbox's, which is why South->B and
East->A (not a straight A->A mapping) -- this matches the physical button
positions, not the labels."
        }
        "features" => {
            concat!(
                "Feature highlights (v",
                env!("CARGO_PKG_VERSION"),
                ")\n",
                "=========================================\n",
                "\n\
Accuracy
  Cycle-accurate 65C816 + SPC700 (0-diff vs. SingleStepTests oracles), a
  master-clock lockstep scheduler, dot-accurate PPU/HDMA, and a deterministic
  audio resync -- same seed+ROM+input always yields a bit-identical AV output.

Coprocessors
  DSP-1, Super FX/GSU, and SA-1 (Core/Curated, oracle-gated); DSP-2, DSP-4,
  ST010, CX4, OBC1, S-DD1 (BestEffort, real-title validated); ST018, S-RTC
  (BestEffort, unit-tested only); SPC7110 (implemented; the one locally
  available dump is a ROM-sourcing gap, not an open bug).
  Run `rustysnes help coprocessors` for the detail.

Save states & time control
  A quick-save slot, a 10-slot disk-backed thumbnail Save States manager,
  rewind, and run-ahead -- all built on a versioned deterministic snapshot.

Desktop UX
  Light/dark/system themes, 25%-300% speed presets, fullscreen, a Performance
  panel (FPS/frame-time/audio-health + a rolling sparkline), and a first-run
  welcome modal.

Optional (cargo --features):
  debug-hooks        CPU/PPU/APU/cart debugger overlay + watchpoints/breakpoints
  scripting          Sandboxed Lua 5.4 (mlua) + TAS movie record/playback
  cheats             Game Genie / Pro Action Replay codes
  netplay            GGPO-style rollback netplay (native UDP)
  retroachievements  RetroAchievements (vendored rcheevos)
  hd-pack            Reserved for a future HD texture-pack loader (not wired)
  emu-thread         A dedicated emulation thread (off by default -- not yet
                     feature-complete: no audio output, doesn't yet drive
                     cheats/watchpoints/scripting/movies/rewind/run-ahead)

See README.md and docs/frontend.md for the full detail."
            )
        }
        "coprocessors" => {
            "\
Cartridge boards & coprocessors
=================================

RustySNES classifies every board/coprocessor into an honesty-gated accuracy
tier (docs/adr/0003) -- a CI gate ensures no unverified BestEffort board ever
backs the accuracy oracle:

  Core/Curated (oracle-gated, cross-checked against an independent reference)
    DSP-1 ........ the shared uPD77C25 LLE engine, real DSP-1 games
    Super FX/GSU . the full Argonaut RISC core, 58-ROM Krom suite
    SA-1 ......... the second 65C816, 18 commercial carts

  BestEffort, real-title validated (boots a real commercial title to gameplay)
    DSP-2, DSP-4, ST010, CX4, OBC1, S-DD1

  BestEffort, unit-tested only (no commercial dump in the local corpus)
    ST018 (full ARMv3 core), standalone S-RTC

  Implemented, boot gap tracked
    SPC7110 -- addressing/timing bugs fixed; the one locally available dump
    turned out to be a fan-translation ROM hack (not an original cartridge),
    a ROM-sourcing gap, not an open emulation bug (docs/audit/).

See docs/cart.md and docs/STATUS.md for the full per-board detail."
        }
        "config" => {
            "\
Configuration
===============

Settings live in a TOML file under the platform config directory:

  Linux ...... ~/.config/rustysnes/config.toml
  macOS ...... ~/Library/Application Support/rustysnes/config.toml
  Windows .... %APPDATA%\\rustysnes\\config.toml

Disk-backed save states (Emulation -> Save States...) live under the matching
data directory (Linux: ~/.local/share/rustysnes/saves/<rom-sha256>/), keyed
per-ROM so different games never collide.

Most settings are editable in-app (the Settings window; changes that need a
running session apply live -- present mode, theme, speed). A missing or
corrupt config.toml falls back to defaults rather than blocking launch.

Shell completions: `rustysnes completions <bash|zsh|fish|powershell>`."
        }
        "scripting" => {
            "\
Lua scripting
===============

RustySNES embeds a sandboxed Lua 5.4 engine (native-only, `mlua`, behind the
off-by-default `scripting` cargo feature). A loaded script can read/write
WRAM through the Bus and registers a per-frame callback; a runaway-loop
instruction-budget guard unloads a script that errors or misbehaves rather
than hanging the emulator. Writes are gated off whenever a TAS movie is
recording or playing, so a script can never perturb a deterministic replay
it doesn't own.

The same feature also enables TAS movie record/playback (a deterministic
input-log format, power-on or embedded-save-state start).

Enable it with:
  cargo run -p rustysnes-frontend --features scripting -- game.sfc"
        }
        "netplay" => {
            "\
Netplay
=========

GGPO-style rollback netcode for 2 players (native UDP transport), behind the
off-by-default `netplay` cargo feature: predict -> advance -> roll back and
re-simulate on the deterministic core, proven bit-identical under both ideal
and adverse (latency/jitter/packet-loss) conditions. A wasm32-clippy-verified
WebRTC transport exists at the crate level; the browser signaling/SDP UI is
an honestly deferred, separate scope (not yet wired into the frontend).

Enable it with:
  cargo run -p rustysnes-frontend --features netplay -- game.sfc"
        }
        "about" => {
            "\
About RustySNES
=================

A cycle-accurate Super Nintendo / Super Famicom emulator written in pure
Rust, targeting the Mesen2 / higan / ares accuracy bar. The frontend is
winit + wgpu + cpal + egui; the chip stack (CPU/PPU/APU/cart) is
no_std + alloc and independently testable/fuzzable/benchmarkable.

  License .... MIT OR Apache-2.0
  Author ..... DoubleGate <parobek@gmail.com>
  Repo ....... https://github.com/doublegate/RustySNES

Run `rustysnes --version` for the build version, or
`rustysnes help features` for the full feature/flag list."
        }
        _ => return None,
    })
}

/// The list of known help-topic names (for `rustysnes help` with no topic).
pub const TOPICS: &[&str] = &[
    "controls",
    "hotkeys",
    "gamepad",
    "features",
    "coprocessors",
    "config",
    "scripting",
    "netplay",
    "about",
];

#[cfg(test)]
mod tests {
    use super::*;
    use clap::CommandFactory as _;

    #[test]
    fn cli_command_parses_rom_positional() {
        let cli = Cli::try_parse_from(["rustysnes", "game.sfc"]).expect("parse");
        assert_eq!(cli.rom, Some(PathBuf::from("game.sfc")));
        assert!(cli.command.is_none());
    }

    #[test]
    fn no_args_means_no_rom_no_command() {
        let cli = Cli::try_parse_from(["rustysnes"]).expect("parse");
        assert!(cli.rom.is_none());
        assert!(cli.command.is_none());
    }

    #[test]
    fn color_flag_parses() {
        let cli =
            Cli::try_parse_from(["rustysnes", "--color", "never", "game.sfc"]).expect("parse");
        assert_eq!(cli.color, ColorWhen::Never);
        assert_eq!(clap::ColorChoice::from(cli.color), clap::ColorChoice::Never);
    }

    #[test]
    fn help_subcommand_parses_topic() {
        let cli = Cli::try_parse_from(["rustysnes", "help", "controls"]).expect("parse");
        match cli.command {
            Some(CliCommand::Help { topic, .. }) => assert_eq!(topic.as_deref(), Some("controls")),
            _ => panic!("expected Help"),
        }
    }

    #[test]
    fn every_listed_topic_has_text() {
        for t in TOPICS {
            assert!(topic_text(t).is_some(), "missing text for topic {t}");
        }
        assert!(topic_text("nonexistent").is_none());
    }

    #[test]
    fn clap_command_is_valid() {
        Cli::command().debug_assert();
    }
}
