//! `rustysnes` — the RustySNES frontend binary (native).
//!
//! A thin shim over `lib.rs`, which owns the module tree. The wasm32 entry point lives at
//! `lib.rs::wasm::start` (gated `#[cfg(target_arch = "wasm32")]`); when cargo builds this bin
//! for the wasm32 target we compile an empty `main` instead — the real entry is `wasm::start`.
//!
//! The native path uses a clap 4 CLI (`cli.rs`): `rustysnes <ROM>` loads + runs; `rustysnes`
//! with no ROM opens the menu shell; `rustysnes help [<topic>]` + `completions <shell>` are
//! the native-only help/UX subcommands. See `docs/frontend.md` for the architecture.

#[cfg(target_arch = "wasm32")]
fn main() {}

#[cfg(not(target_arch = "wasm32"))]
use std::process::ExitCode;

#[cfg(not(target_arch = "wasm32"))]
use clap::{CommandFactory as _, Parser as _};

#[cfg(not(target_arch = "wasm32"))]
use rustysnes_frontend::app::App;
#[cfg(not(target_arch = "wasm32"))]
use rustysnes_frontend::cli::{Cli, CliCommand};
#[cfg(not(target_arch = "wasm32"))]
use rustysnes_frontend::config::Config;

#[cfg(not(target_arch = "wasm32"))]
fn main() -> ExitCode {
    let cli = match Cli::try_parse() {
        Ok(cli) => cli,
        Err(e) => {
            // clap prints help/version to stdout (exit 0) and errors to stderr (exit 2).
            let _ = e.print();
            return ExitCode::from(u8::try_from(e.exit_code()).unwrap_or(2));
        }
    };

    match cli.command {
        Some(CliCommand::Help { topic, interactive }) => run_help(topic.as_deref(), interactive),
        Some(CliCommand::Completions { shell }) => {
            let mut cmd = Cli::command();
            clap_complete::generate(shell, &mut cmd, "rustysnes", &mut std::io::stdout());
            ExitCode::SUCCESS
        }
        None => run_emulator(cli.rom),
    }
}

#[cfg(not(target_arch = "wasm32"))]
fn run_emulator(rom: Option<std::path::PathBuf>) -> ExitCode {
    let config = Config::load();
    let app = App::new(config, rom);
    match app.run() {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("rustysnes: {e}");
            ExitCode::FAILURE
        }
    }
}

#[cfg(not(target_arch = "wasm32"))]
fn run_help(topic: Option<&str>, interactive: bool) -> ExitCode {
    use rustysnes_frontend::cli::{TOPICS, topic_text};

    #[cfg(feature = "help-tui")]
    if interactive {
        if let Err(e) = rustysnes_frontend::help_tui::run() {
            eprintln!("rustysnes: help TUI error: {e}");
            return ExitCode::FAILURE;
        }
        return ExitCode::SUCCESS;
    }
    #[cfg(not(feature = "help-tui"))]
    let _ = interactive;

    topic.map_or_else(
        || {
            println!("RustySNES help topics:");
            for t in TOPICS {
                println!("  {t}");
            }
            println!("\nRun `rustysnes help <topic>` (or `--interactive` for the TUI browser).");
            ExitCode::SUCCESS
        },
        |t| {
            topic_text(t).map_or_else(
                || {
                    eprintln!(
                        "rustysnes: unknown help topic '{t}'. Known: {}",
                        TOPICS.join(", ")
                    );
                    ExitCode::FAILURE
                },
                |body| {
                    println!("{body}");
                    ExitCode::SUCCESS
                },
            )
        },
    )
}
