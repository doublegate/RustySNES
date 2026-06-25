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
        .error(AnsiColor::Red.on_default() | Effects::BOLD)
}

const AFTER_HELP: &str = "\
Examples:
  rustysnes game.sfc           Load and run a ROM
  rustysnes help controls      Show the keyboard/gamepad reference
  rustysnes completions fish   Print a shell-completion script

Keyboard (P1):
  Arrows D-pad   X=A  Z=B  S=X  A=Y   Q=L  W=R   RShift=Select  Enter=Start

See `rustysnes help <topic>` for: controls, gamepad, features, about.";

/// `RustySNES` — a cycle-accurate Super Nintendo Entertainment System emulator.
#[derive(Debug, Parser)]
#[command(
    name = "rustysnes",
    bin_name = "rustysnes",
    version,
    author,
    about = "RustySNES — a cycle-accurate SNES / Super Famicom emulator (winit + wgpu + cpal + egui).",
    after_help = AFTER_HELP,
    styles = cli_styles(),
    disable_help_subcommand = true,
)]
pub struct Cli {
    /// Path to the `.sfc` / `.smc` ROM to load and run. Load further ROMs from the File menu or
    /// by drag-and-drop once a session is open.
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
#[must_use]
pub fn topic_text(topic: &str) -> Option<&'static str> {
    Some(match topic {
        "controls" => {
            "\
Controls (P1 keyboard):
  D-pad   Arrow keys
  A=X  B=Z  X=S  Y=A       (the SNES diamond)
  L=Q  R=W
  Select  Right Shift      Start  Enter
System: Esc quit, F2 reset, F3 power-cycle, F12 open ROM, ` toggle debugger."
        }
        "gamepad" => {
            "\
Gamepad: USB pads auto-bind to P1 (Xbox-style layout).
  Xbox A->SNES B, Xbox B->SNES A, Xbox X->SNES Y, Xbox Y->SNES X.
  LB/RB->L/R, Start->Start, Back/Select->Select, D-pad->D-pad."
        }
        "features" => {
            "\
RustySNES v0.1.0 is a SCAFFOLD: the cycle-accurate chip crates (65C816 / PPU1+2 /
SPC700+S-DSP / cart) are skeletons. The frontend shell, ROM-load, and present
path compile and run; pixel output lands with the PPU model. See README.md."
        }
        "about" => {
            "\
RustySNES — a cycle-accurate Super Nintendo emulator in pure Rust.
Accuracy bar: bsnes / Mesen2 / higan / ares. MIT OR Apache-2.0. Author: DoubleGate."
        }
        _ => return None,
    })
}

/// The list of known help-topic names (for `rustysnes help` with no topic).
pub const TOPICS: &[&str] = &["controls", "gamepad", "features", "about"];

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
