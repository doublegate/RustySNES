//! HD texture pack manifest types (`v1.3.0`, `hd-pack` feature).
//!
//! This module owns only the manifest **schema** — a TOML `pack.toml` deserializes into
//! [`HdPackManifest`], keyed per tile by the hex tile-identity hash `rustysnes_ppu::hdtag::hash_tile`
//! computes core-side. There is no direct crate dependency on `rustysnes-ppu` here (the
//! one-directional crate-graph rule — the frontend depends on `rustysnes-core` only); the
//! [`TileClass`] enum in this module mirrors the core's `rustysnes_ppu::hdtag::TileClass`
//! discriminants by convention, not by shared type.
//!
//! Directory convention (mirrors `save_states.rs`'s per-ROM-hash layout):
//! `<data_dir>/hd-packs/<rom_sha256_hex>/<pack-name>/pack.toml` + `tiles/*.png`. [`HdPack::load`]
//! parses the manifest and decodes every referenced PNG into RGBA8 (via the pure-Rust `png`
//! crate — no system libpng dependency); [`discover_packs`] enumerates the pack subdirectories
//! available for a given ROM hash. Actually compositing a loaded pack's tiles onto the live
//! framebuffer is `crate::hd_compositor`'s job, not this module's. Always `None`/empty on
//! `wasm32` (no filesystem — same posture as `save_states.rs`/`crate::config::Config::path`).

use std::collections::HashMap;
use std::fmt;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

/// The current manifest format version.
///
/// Bump on any breaking schema change (mirrors the `rustysnes-savestate` `FORMAT_VERSION` gate
/// convention) — a pack whose `format_version` this loader doesn't recognize should be rejected
/// explicitly, never silently misparsed.
pub const FORMAT_VERSION: u32 = 1;

/// Which PPU render path a tile entry targets.
///
/// Mirrors `rustysnes_ppu::hdtag::TileClass`'s three variants (`Bg` = 0, `Obj` = 1, `Mode7` = 2)
/// — the same tile can only ever have been produced by one of these three paths, so a manifest
/// entry must say which to disambiguate a hash collision across paths (the same reasoning as the
/// core-side hash's own class discriminant).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum TileClass {
    /// A background-layer character (BG1-4, any mode).
    Bg,
    /// A sprite (OBJ) character.
    Obj,
    /// A Mode 7 affine-map character.
    Mode7,
}

/// One replaceable tile's manifest entry.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TileEntry {
    /// The tile-identity hash, hex-encoded (e.g. `"3a7493ef1d8af377"`), matching
    /// `rustysnes_ppu::hdtag::hash_tile`'s `u64` output formatted as lowercase hex with no `0x`
    /// prefix — the same convention `save_states.rs::hex` uses for ROM hashes.
    pub hash: String,
    /// Which render path produced this tile.
    pub class: TileClass,
    /// Bits per pixel of the source tile (2/4/8), included for the loader's own sanity check
    /// against the replacement image's declared size — not itself part of matching (the hash
    /// already folds bpp in).
    pub bpp: u8,
    /// The replacement image path, relative to the pack directory (e.g. `"tiles/0042.png"`).
    pub image: String,
}

impl TileEntry {
    /// Parse [`TileEntry::hash`] as a `u64`, or `None` if it isn't valid lowercase hex.
    #[must_use]
    pub fn hash_value(&self) -> Option<u64> {
        u64::from_str_radix(&self.hash, 16).ok()
    }
}

/// A parsed `pack.toml` HD texture pack manifest.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct HdPackManifest {
    /// The manifest schema version this pack was written against (see [`FORMAT_VERSION`]).
    pub format_version: u32,
    /// Human-readable pack name, shown in the Settings pack selector.
    pub name: String,
    /// Every replaceable tile this pack provides.
    #[serde(default)]
    pub tiles: Vec<TileEntry>,
}

impl HdPackManifest {
    /// Whether this manifest's [`HdPackManifest::format_version`] matches the loader's
    /// [`FORMAT_VERSION`] — the gate a loader should check before trusting any other field.
    #[must_use]
    pub const fn is_supported_version(&self) -> bool {
        self.format_version == FORMAT_VERSION
    }
}

/// One decoded tile-replacement image, normalized to RGBA8 regardless of the source PNG's color
/// type/bit depth.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DecodedTile {
    /// Image width in pixels (not assumed to be a fixed multiple of 8 — a pack may ship
    /// higher-resolution replacements at any upscale factor).
    pub width: u32,
    /// Image height in pixels.
    pub height: u32,
    /// `width * height * 4` RGBA8 bytes, row-major.
    pub rgba: Vec<u8>,
}

/// A loaded HD texture pack: the parsed manifest plus every tile's decoded replacement image.
///
/// Keyed by [`TileEntry::hash_value`]. Entries whose hash fails to parse, or whose image fails
/// to decode, make the whole [`HdPack::load`] call fail rather than silently dropping a tile —
/// a malformed pack should be rejected up front, not partially applied.
#[derive(Debug, Clone, Default)]
pub struct HdPack {
    /// The parsed manifest (name, format version, tile entry list).
    pub manifest: HdPackManifest,
    /// Decoded replacement images, keyed by tile-identity hash.
    pub tiles: HashMap<u64, DecodedTile>,
}

/// Why loading or decoding an HD texture pack failed.
#[derive(Debug)]
pub enum HdPackError {
    /// A filesystem operation failed (missing pack directory, unreadable `pack.toml`/image).
    Io(std::io::Error),
    /// `pack.toml` did not parse as valid [`HdPackManifest`] TOML.
    Toml(toml::de::Error),
    /// A tile's referenced image failed to decode as PNG.
    Png(png::DecodingError),
    /// The manifest's `format_version` isn't [`FORMAT_VERSION`].
    UnsupportedVersion(u32),
    /// A [`TileEntry::hash`] isn't valid lowercase hex.
    InvalidHash(String),
}

impl fmt::Display for HdPackError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Io(e) => write!(f, "I/O error: {e}"),
            Self::Toml(e) => write!(f, "pack.toml parse error: {e}"),
            Self::Png(e) => write!(f, "tile image decode error: {e}"),
            Self::UnsupportedVersion(v) => write!(
                f,
                "unsupported pack format_version {v} (expected {FORMAT_VERSION})"
            ),
            Self::InvalidHash(h) => write!(f, "tile entry has an invalid hash {h:?}"),
        }
    }
}

impl std::error::Error for HdPackError {}

impl From<std::io::Error> for HdPackError {
    fn from(e: std::io::Error) -> Self {
        Self::Io(e)
    }
}

impl From<toml::de::Error> for HdPackError {
    fn from(e: toml::de::Error) -> Self {
        Self::Toml(e)
    }
}

impl From<png::DecodingError> for HdPackError {
    fn from(e: png::DecodingError) -> Self {
        Self::Png(e)
    }
}

impl HdPack {
    /// Load a pack from `dir` (must contain `pack.toml` plus every tile image it references,
    /// relative to `dir`).
    ///
    /// # Errors
    /// See [`HdPackError`]'s variants.
    pub fn load(dir: &Path) -> Result<Self, HdPackError> {
        let manifest_text = std::fs::read_to_string(dir.join("pack.toml"))?;
        let manifest: HdPackManifest = toml::from_str(&manifest_text)?;
        if !manifest.is_supported_version() {
            return Err(HdPackError::UnsupportedVersion(manifest.format_version));
        }
        let mut tiles = HashMap::with_capacity(manifest.tiles.len());
        for entry in &manifest.tiles {
            let hash = entry
                .hash_value()
                .ok_or_else(|| HdPackError::InvalidHash(entry.hash.clone()))?;
            tiles.insert(hash, decode_png(&dir.join(&entry.image))?);
        }
        Ok(Self { manifest, tiles })
    }
}

/// Decode `path` as a PNG, normalizing to RGBA8 regardless of source color type/bit depth.
fn decode_png(path: &Path) -> Result<DecodedTile, HdPackError> {
    let file = std::io::BufReader::new(std::fs::File::open(path)?);
    let mut decoder = png::Decoder::new(file);
    // Expand palette/grayscale/transparency and strip 16-bit depth down to 8 -- guarantees the
    // reader only ever yields Grayscale/GrayscaleAlpha/Rgb/Rgba at 8 bits/channel, which
    // `rgba8_from_frame` below handles explicitly (never `Indexed`, never 16-bit).
    decoder.set_transformations(png::Transformations::EXPAND | png::Transformations::STRIP_16);
    let mut reader = decoder.read_info()?;
    let buf_size = reader
        .output_buffer_size()
        .ok_or_else(|| std::io::Error::other("PNG frame buffer size overflowed usize"))?;
    let mut buf = vec![0u8; buf_size];
    let info = reader.next_frame(&mut buf)?;
    let rgba = rgba8_from_frame(
        &buf[..info.buffer_size()],
        info.color_type,
        info.width,
        info.height,
    );
    Ok(DecodedTile {
        width: info.width,
        height: info.height,
        rgba,
    })
}

/// Expand a decoded PNG frame's raw bytes to RGBA8, given its (post-transformation) color type.
fn rgba8_from_frame(bytes: &[u8], color_type: png::ColorType, width: u32, height: u32) -> Vec<u8> {
    let pixel_count = (width as usize) * (height as usize);
    let mut out = Vec::with_capacity(pixel_count * 4);
    match color_type {
        png::ColorType::Grayscale => {
            for &g in bytes.iter().take(pixel_count) {
                out.extend_from_slice(&[g, g, g, 255]);
            }
        }
        png::ColorType::GrayscaleAlpha => {
            for px in bytes.chunks_exact(2).take(pixel_count) {
                out.extend_from_slice(&[px[0], px[0], px[0], px[1]]);
            }
        }
        png::ColorType::Rgb => {
            for px in bytes.chunks_exact(3).take(pixel_count) {
                out.extend_from_slice(&[px[0], px[1], px[2], 255]);
            }
        }
        png::ColorType::Rgba => {
            let n = pixel_count * 4;
            out.extend_from_slice(&bytes[..n.min(bytes.len())]);
            out.resize(pixel_count * 4, 0);
        }
        // `Indexed` cannot occur here: `Transformations::EXPAND` always resolves it to
        // Rgb/Rgba before this function ever sees the bytes.
        png::ColorType::Indexed => out.resize(pixel_count * 4, 0),
    }
    out
}

/// The platform HD-texture-pack root directory (`<data_dir>/hd-packs`), or `None` where no data
/// dir is resolvable (always `None` on `wasm32` — no filesystem).
#[must_use]
pub fn base_dir() -> Option<PathBuf> {
    #[cfg(target_arch = "wasm32")]
    {
        None
    }
    #[cfg(not(target_arch = "wasm32"))]
    {
        directories::ProjectDirs::from("io.github", "doublegate", "RustySNES")
            .map(|d| d.data_dir().join("hd-packs"))
    }
}

/// The per-ROM pack directory (`<base_dir>/<rom_sha256_hex>`), or `None` if no data dir is
/// resolvable.
#[must_use]
pub fn rom_dir(rom_sha256: &[u8; 32]) -> Option<PathBuf> {
    base_dir().map(|dir| dir.join(crate::save_states::hex(rom_sha256)))
}

/// List every pack subdirectory name available for `rom_sha256`, sorted.
///
/// Does not attempt to load/validate each one — just enumerates candidates for a Settings
/// pack-selector UI; a name that turns out to have a malformed `pack.toml` is surfaced only when
/// [`load_pack`] is actually called for it.
#[must_use]
pub fn discover_packs(rom_sha256: &[u8; 32]) -> Vec<String> {
    rom_dir(rom_sha256)
        .map(|dir| list_subdirectory_names(&dir))
        .unwrap_or_default()
}

/// The sorted names of every immediate subdirectory of `dir`, or empty if `dir` doesn't exist /
/// isn't readable. A standalone, directly-testable seam for [`discover_packs`]'s logic (which
/// otherwise composes the real, non-overridable platform data directory).
fn list_subdirectory_names(dir: &Path) -> Vec<String> {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return Vec::new();
    };
    let mut names: Vec<String> = entries
        .filter_map(Result::ok)
        .filter(|e| e.path().is_dir())
        .filter_map(|e| e.file_name().into_string().ok())
        .collect();
    names.sort();
    names
}

/// Load pack `pack_name` for `rom_sha256` (i.e. `<base_dir>/<rom_sha256_hex>/<pack_name>`).
///
/// # Errors
/// [`HdPackError::Io`] if no data directory is resolvable (always the case on `wasm32`) or the
/// pack directory doesn't exist; see [`HdPack::load`] for the rest.
pub fn load_pack(rom_sha256: &[u8; 32], pack_name: &str) -> Result<HdPack, HdPackError> {
    let dir =
        rom_dir(rom_sha256).ok_or_else(|| std::io::Error::other("no writable data directory"))?;
    HdPack::load(&dir.join(pack_name))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_manifest() -> HdPackManifest {
        HdPackManifest {
            format_version: FORMAT_VERSION,
            name: "Test Pack".into(),
            tiles: vec![
                TileEntry {
                    hash: "3a7493ef1d8af377".into(),
                    class: TileClass::Bg,
                    bpp: 2,
                    image: "tiles/0000.png".into(),
                },
                TileEntry {
                    hash: "00000000deadbeef".into(),
                    class: TileClass::Obj,
                    bpp: 4,
                    image: "tiles/0001.png".into(),
                },
            ],
        }
    }

    #[test]
    fn manifest_round_trips_through_toml() {
        let manifest = sample_manifest();
        let s = toml::to_string_pretty(&manifest).expect("serialize");
        let back: HdPackManifest = toml::from_str(&s).expect("deserialize");
        assert_eq!(back, manifest);
    }

    #[test]
    fn empty_tiles_defaults_when_omitted() {
        let s = "format_version = 1\nname = \"Bare\"\n";
        let manifest: HdPackManifest = toml::from_str(s).expect("deserialize");
        assert_eq!(manifest.tiles, Vec::new());
    }

    #[test]
    fn is_supported_version_checks_the_gate() {
        let mut manifest = sample_manifest();
        assert!(manifest.is_supported_version());
        manifest.format_version = FORMAT_VERSION + 1;
        assert!(!manifest.is_supported_version());
    }

    #[test]
    fn hash_value_parses_valid_hex_and_rejects_garbage() {
        let entry = &sample_manifest().tiles[0];
        assert_eq!(entry.hash_value(), Some(0x3a74_93ef_1d8a_f377_u64));

        let bad = TileEntry {
            hash: "not-hex".into(),
            ..sample_manifest().tiles[0].clone()
        };
        assert_eq!(bad.hash_value(), None);
    }

    #[test]
    fn tile_class_serializes_lowercase() {
        // `toml::to_string` requires a top-level table, so a bare enum can't round-trip alone --
        // exercise the `#[serde(rename_all = "lowercase")]` attribute through a real manifest
        // entry instead, matching how it's actually used.
        for (class, want) in [
            (TileClass::Bg, "bg"),
            (TileClass::Obj, "obj"),
            (TileClass::Mode7, "mode7"),
        ] {
            let entry = TileEntry {
                class,
                ..sample_manifest().tiles[0].clone()
            };
            let s = toml::to_string(&entry).expect("serialize");
            assert!(s.contains(&format!("class = \"{want}\"")), "got: {s}");
        }
    }

    // --- Loader tests (`HdPack::load` + `list_subdirectory_names`) ---

    /// A process-unique scratch directory under the OS temp dir, removed on drop.
    struct TempDir(PathBuf);

    impl TempDir {
        fn new(label: &str) -> Self {
            use std::sync::atomic::{AtomicU32, Ordering};
            static COUNTER: AtomicU32 = AtomicU32::new(0);
            let n = COUNTER.fetch_add(1, Ordering::Relaxed);
            let dir = std::env::temp_dir().join(format!(
                "rustysnes-hdpack-test-{label}-{}-{n}",
                std::process::id()
            ));
            std::fs::create_dir_all(&dir).expect("create temp test dir");
            Self(dir)
        }
    }

    impl Drop for TempDir {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.0);
        }
    }

    /// Write a minimal 2x2 RGBA8 PNG to `path` (four distinct solid-quadrant colors), for
    /// `HdPack::load` round-trip tests.
    fn write_test_png(path: &Path) {
        let file = std::fs::File::create(path).expect("create test png");
        let w = std::io::BufWriter::new(file);
        let mut encoder = png::Encoder::new(w, 2, 2);
        encoder.set_color(png::ColorType::Rgba);
        encoder.set_depth(png::BitDepth::Eight);
        let mut writer = encoder.write_header().expect("write png header");
        let pixels: [u8; 16] = [
            255, 0, 0, 255, // top-left: red
            0, 255, 0, 255, // top-right: green
            0, 0, 255, 255, // bottom-left: blue
            255, 255, 0, 255, // bottom-right: yellow
        ];
        writer.write_image_data(&pixels).expect("write png data");
    }

    #[test]
    fn load_reads_manifest_and_decodes_every_tile_image() {
        let dir = TempDir::new("load-ok");
        std::fs::write(
            dir.0.join("pack.toml"),
            "format_version = 1\nname = \"Fixture Pack\"\n\n\
             [[tiles]]\nhash = \"00000000000000ab\"\nclass = \"bg\"\nbpp = 2\nimage = \"tile.png\"\n",
        )
        .unwrap();
        write_test_png(&dir.0.join("tile.png"));

        let pack = HdPack::load(&dir.0).expect("load should succeed");
        assert_eq!(pack.manifest.name, "Fixture Pack");
        let tile = pack.tiles.get(&0xab).expect("tile 0xab decoded");
        assert_eq!((tile.width, tile.height), (2, 2));
        assert_eq!(&tile.rgba[0..4], &[255, 0, 0, 255]);
        assert_eq!(&tile.rgba[4..8], &[0, 255, 0, 255]);
        assert_eq!(&tile.rgba[8..12], &[0, 0, 255, 255]);
        assert_eq!(&tile.rgba[12..16], &[255, 255, 0, 255]);
    }

    #[test]
    fn load_rejects_unsupported_format_version() {
        let dir = TempDir::new("load-bad-version");
        std::fs::write(
            dir.0.join("pack.toml"),
            "format_version = 999\nname = \"Future\"\n",
        )
        .unwrap();
        let err = HdPack::load(&dir.0).expect_err("must reject the version");
        assert!(matches!(err, HdPackError::UnsupportedVersion(999)));
    }

    #[test]
    fn load_rejects_invalid_tile_hash() {
        let dir = TempDir::new("load-bad-hash");
        std::fs::write(
            dir.0.join("pack.toml"),
            "format_version = 1\nname = \"Bad Hash\"\n\n\
             [[tiles]]\nhash = \"not-hex\"\nclass = \"bg\"\nbpp = 2\nimage = \"tile.png\"\n",
        )
        .unwrap();
        let err = HdPack::load(&dir.0).expect_err("must reject the bad hash");
        assert!(matches!(err, HdPackError::InvalidHash(_)));
    }

    #[test]
    fn load_missing_manifest_is_an_io_error() {
        let dir = TempDir::new("load-missing");
        let err = HdPack::load(&dir.0).expect_err("must fail without a pack.toml");
        assert!(matches!(err, HdPackError::Io(_)));
    }

    #[test]
    fn list_subdirectory_names_returns_only_dirs_sorted() {
        let dir = TempDir::new("discover");
        std::fs::create_dir_all(dir.0.join("zeta")).unwrap();
        std::fs::create_dir_all(dir.0.join("alpha")).unwrap();
        std::fs::write(dir.0.join("not-a-dir.txt"), b"ignored").unwrap();

        assert_eq!(
            list_subdirectory_names(&dir.0),
            vec!["alpha".to_string(), "zeta".to_string()]
        );
    }

    #[test]
    fn list_subdirectory_names_of_a_missing_dir_is_empty() {
        let dir = TempDir::new("discover-missing");
        assert_eq!(
            list_subdirectory_names(&dir.0.join("does-not-exist")),
            Vec::<String>::new()
        );
    }
}
