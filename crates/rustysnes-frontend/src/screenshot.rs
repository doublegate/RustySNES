//! Screenshot capture (`v1.25.0`) — PNG encode, save-to-file, and copy-to-clipboard.
//!
//! Ported from RustyNES's own screenshot feature. The captured pixels are the **presented**
//! framebuffer: whatever buffer transforms have already run (HD-pack composite, overscan crop) are
//! included, and the post-filter/letterbox pass is not, because that happens on the GPU after this
//! point. So a screenshot is the emulator's output image at its native resolution, not a grab of
//! the window — which is what makes two screenshots of the same frame byte-identical regardless of
//! window size, and what makes them usable as reference images.
//!
//! The input format is the same RGBA8 buffer `crate::gfx::Gfx::upload` takes, so capture costs one
//! encode of a buffer that already exists rather than any readback from the GPU.

/// Encode an RGBA8 buffer as a PNG byte stream.
///
/// # Errors
/// Returns the underlying [`png::EncodingError`] if `rgba` is not `w * h * 4` bytes or the encoder
/// fails. The length is checked up front rather than trusted: a mismatch would otherwise be an
/// out-of-bounds read inside the encoder.
pub fn encode_png(rgba: &[u8], w: u32, h: u32) -> Result<Vec<u8>, png::EncodingError> {
    let expected = (w as usize) * (h as usize) * 4;
    if rgba.len() < expected || w == 0 || h == 0 {
        return Err(png::EncodingError::LimitsExceeded);
    }
    let mut out = Vec::new();
    {
        let mut encoder = png::Encoder::new(&mut out, w, h);
        encoder.set_color(png::ColorType::Rgba);
        encoder.set_depth(png::BitDepth::Eight);
        let mut writer = encoder.write_header()?;
        writer.write_image_data(&rgba[..expected])?;
    }
    Ok(out)
}

/// Where screenshots are written, given the configured override.
///
/// Falls back to the platform picture directory, then the current directory — a screenshot is a
/// convenience, so a missing well-known folder must not turn into an error the user has to fix.
#[cfg(not(target_arch = "wasm32"))]
#[must_use]
pub fn screenshot_dir(configured: Option<&str>) -> std::path::PathBuf {
    if let Some(dir) = configured {
        return std::path::PathBuf::from(dir);
    }
    directories::UserDirs::new()
        .and_then(|d| d.picture_dir().map(std::path::Path::to_path_buf))
        .unwrap_or_else(|| std::path::PathBuf::from("."))
}

/// Build a collision-resistant screenshot filename stem from a ROM title and a counter.
///
/// The counter (rather than a timestamp) keeps this deterministic and testable; the caller probes
/// for the first free index, so two screenshots in the same second still get distinct files.
#[must_use]
pub fn file_stem(rom_title: Option<&str>, index: u32) -> String {
    let base = rom_title
        .map(sanitize)
        .filter(|s| !s.is_empty())
        .unwrap_or_else(|| "rustysnes".to_string());
    format!("{base}-{index:04}")
}

/// Reduce a ROM title to characters safe in a filename on every supported platform.
///
/// Internal SNES titles legitimately contain spaces, punctuation, and (Shift-JIS-decoded) symbols;
/// writing those straight into a path is how a screenshot silently fails to save on Windows.
fn sanitize(title: &str) -> String {
    title
        .chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() || c == '-' || c == '_' {
                c
            } else {
                '_'
            }
        })
        .collect::<String>()
        .trim_matches('_')
        .to_string()
}

/// Save `rgba` as a PNG in `dir`, choosing the first free `<stem>.png`. Returns the written path.
///
/// # Errors
/// Returns an [`std::io::Error`] if the directory cannot be created, the PNG cannot be encoded, or
/// the file cannot be written.
#[cfg(not(target_arch = "wasm32"))]
pub fn save_png(
    dir: &std::path::Path,
    rom_title: Option<&str>,
    rgba: &[u8],
    w: u32,
    h: u32,
) -> std::io::Result<std::path::PathBuf> {
    std::fs::create_dir_all(dir)?;
    let bytes = encode_png(rgba, w, h)
        .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e.to_string()))?;
    // Probe for a free name. Bounded so a full/unwritable directory fails loudly instead of
    // spinning forever.
    for index in 0..10_000 {
        let path = dir.join(format!("{}.png", file_stem(rom_title, index)));
        // `create_new` claims the name atomically. An `exists()` probe followed by a write is a
        // TOCTOU race: two screenshots taken in the same instant (a held hotkey, a second
        // instance) both see the name free and one silently overwrites the other.
        match std::fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&path)
        {
            Ok(mut file) => {
                use std::io::Write as _;
                file.write_all(&bytes)?;
                return Ok(path);
            }
            // Taken between the probe and now — try the next index rather than failing.
            Err(e) if e.kind() == std::io::ErrorKind::AlreadyExists => {}
            Err(e) => return Err(e),
        }
    }
    Err(std::io::Error::new(
        std::io::ErrorKind::AlreadyExists,
        "screenshot directory already holds 10000 numbered captures",
    ))
}

/// Copy `rgba` to the system clipboard as an image.
///
/// # Errors
/// Returns a human-readable string if the clipboard cannot be opened or written — clipboard access
/// legitimately fails (no compositor, a Wayland session without the right protocol), and that must
/// surface as a status line rather than a panic.
#[cfg(not(target_arch = "wasm32"))]
pub fn copy_to_clipboard(rgba: &[u8], w: u32, h: u32) -> Result<(), String> {
    let expected = (w as usize) * (h as usize) * 4;
    if rgba.len() < expected || w == 0 || h == 0 {
        return Err("frame buffer smaller than its declared dimensions".into());
    }
    let img = arboard::ImageData {
        width: w as usize,
        height: h as usize,
        bytes: std::borrow::Cow::Borrowed(&rgba[..expected]),
    };
    let mut cb = arboard::Clipboard::new().map_err(|e| e.to_string())?;
    cb.set_image(img).map_err(|e| e.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn encodes_a_valid_png_signature() {
        let rgba = vec![0x7f; 4 * 4 * 4];
        let png = encode_png(&rgba, 4, 4).expect("encode");
        assert_eq!(
            &png[..8],
            &[0x89, b'P', b'N', b'G', 0x0d, 0x0a, 0x1a, 0x0a],
            "must start with the PNG magic"
        );
        assert!(png.len() > 8);
    }

    #[test]
    fn rejects_a_buffer_shorter_than_its_dimensions() {
        // The guard that stops a mismatch becoming an out-of-bounds read in the encoder.
        assert!(encode_png(&[0; 16], 64, 64).is_err());
        assert!(encode_png(&[], 0, 0).is_err());
    }

    #[test]
    fn png_round_trips_back_to_the_same_pixels() {
        // Encode a known gradient and decode it again: proves the stride/format are right, which a
        // signature check alone cannot show.
        let (w, h) = (8u32, 4u32);
        let mut rgba = Vec::new();
        for y in 0..h {
            for x in 0..w {
                #[allow(clippy::cast_possible_truncation)]
                rgba.extend_from_slice(&[(x * 8) as u8, (y * 16) as u8, 0x20, 0xff]);
            }
        }
        let png = encode_png(&rgba, w, h).expect("encode");
        let decoder = png::Decoder::new(std::io::Cursor::new(png));
        let mut reader = decoder.read_info().expect("read info");
        let mut buf = vec![0; reader.output_buffer_size().expect("size")];
        let info = reader.next_frame(&mut buf).expect("decode");
        assert_eq!((info.width, info.height), (w, h));
        assert_eq!(&buf[..rgba.len()], &rgba[..]);
    }

    #[test]
    fn file_stem_sanitizes_titles_and_numbers_them() {
        assert_eq!(
            file_stem(Some("SUPER MARIO WORLD"), 0),
            "SUPER_MARIO_WORLD-0000"
        );
        assert_eq!(file_stem(None, 12), "rustysnes-0012");
        // Leading/trailing junk is trimmed, path separators can never survive.
        let stem = file_stem(Some("../../etc/passwd"), 1);
        assert!(
            !stem.contains('/'),
            "path separators must not survive: {stem}"
        );
        assert!(!stem.contains(".."), "traversal must not survive: {stem}");
        // An all-punctuation title still yields a usable name rather than an empty one.
        assert_eq!(file_stem(Some("***"), 3), "rustysnes-0003");
    }
}
