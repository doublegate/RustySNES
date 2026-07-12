//! Standalone binding-generator binary — `cargo run -p rustysnes-mobile --features bindgen
//! --bin uniffi-bindgen -- generate --library <path-to-cdylib> --language kotlin --out-dir <dir>`
//! produces the Kotlin (or Swift) bindings for [`rustysnes_mobile`]'s `#[uniffi::export]`
//! surface. `--features bindgen` is required (`required-features` in `Cargo.toml`) — the CLI
//! machinery it pulls in (`askama`/`clap`/`goblin`) is host-only tooling, never shipped to a
//! mobile build; see `docs/mobile-readiness.md` for the full binding-generation workflow.

fn main() {
    uniffi::uniffi_bindgen_main();
}
