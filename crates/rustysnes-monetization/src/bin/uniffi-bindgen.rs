//! Standalone binding-generator binary — `cargo run -p rustysnes-monetization --features bindgen
//! --bin uniffi-bindgen -- generate --library <path-to-cdylib> --language kotlin --out-dir <dir>`
//! produces the Kotlin (or Swift) bindings for [`rustysnes_monetization`]'s `#[uniffi::export]`
//! surface. Matches `rustysnes-mobile`'s identical binary exactly — see that crate's own doc
//! comment for the full rationale.

fn main() {
    uniffi::uniffi_bindgen_main();
}
