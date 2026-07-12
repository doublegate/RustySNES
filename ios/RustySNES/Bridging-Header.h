// Bridging header for `rustysnes-ios`'s plain C-ABI FFI surface (see that crate's `src/lib.rs`
// module doc for the full contract). Mirrors `android/app/src/main/kotlin/com/doublegate/
// rustysnes/NativeRenderer.kt`'s JNI declarations -- same four-call lifecycle
// (created/changed/destroyed/present-frame), just C instead of JNI.
//
// NOT generated -- these four signatures are hand-written to match `rustysnes-ios/src/lib.rs`'s
// `#[unsafe(no_mangle)] pub unsafe extern "C" fn` declarations exactly. If that crate's function
// signatures ever change, this header must be updated in the same commit (there is no build step
// that keeps the two in sync automatically, unlike the UniFFI-generated `rustysnes_mobileFFI.h`
// used for `MobileCore`).
#ifndef RUSTYSNES_IOS_BRIDGING_HEADER_H
#define RUSTYSNES_IOS_BRIDGING_HEADER_H

#include <stdint.h>
#include <stddef.h>

void rustysnes_ios_surface_created(void *ui_view, uint32_t width, uint32_t height);
void rustysnes_ios_surface_changed(uint32_t width, uint32_t height);
void rustysnes_ios_surface_destroyed(void);
void rustysnes_ios_present_frame(const uint8_t *rgba, size_t len, uint32_t width, uint32_t height);

#endif /* RUSTYSNES_IOS_BRIDGING_HEADER_H */
