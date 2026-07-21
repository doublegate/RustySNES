//! The blit shader validates — kept as an integration test so it runs on the host.
//!
//! `rustysnes-android`'s library is `cfg(target_os = "android")`, because `ndk-sys` will not
//! compile for anything else. This check has nothing Android about it: it validates
//! `rustysnes-gfx-shaders::BLIT_WGSL`, the same shader `rustysnes-frontend::gfx` uses. Leaving it
//! inside the gated library would have silently dropped shader validation from every host CI job,
//! which is the kind of coverage loss a `cfg` is very good at hiding.

use rustysnes_gfx_shaders::BLIT_WGSL;

#[test]
fn blit_wgsl_validates() {
    let module = naga::front::wgsl::parse_str(BLIT_WGSL).expect("WGSL parses");
    let mut validator = naga::valid::Validator::new(
        naga::valid::ValidationFlags::all(),
        naga::valid::Capabilities::all(),
    );
    validator.validate(&module).expect("WGSL validates");
}
