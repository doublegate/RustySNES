//! Standalone binding-generator binary — `cargo run --bin uniffi-bindgen -- generate
//! --library <path-to-cdylib> --language kotlin --out-dir <dir>` produces the Kotlin (or Swift)
//! bindings for [`rustysnes_mobile`]'s `#[uniffi::export]` surface. Not shipped to mobile
//! targets (host-only tooling); see `docs/mobile-readiness.md` for the full binding-generation
//! workflow.

fn main() {
    uniffi::uniffi_bindgen_main();
}
