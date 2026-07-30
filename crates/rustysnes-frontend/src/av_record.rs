//! A/V capture to disk (`v1.25.0`, T-FP-G1).
//!
//! # Why raw streams and not a container
//!
//! Muxing MP4/MKV means either an `FFmpeg` dependency (a C toolchain on every platform, a licensing
//! surface, and a build that breaks when the system library moves) or hand-rolling a container,
//! which is a large amount of format code whose bugs are invisible until someone tries to play the
//! file six months later.
//!
//! Instead this writes what an emulator can produce **exactly**: uncompressed frames and PCM, each
//! in a format that is trivially correct and that `ffmpeg` turns into anything in one command. The
//! recorder prints that command when it stops, so "raw output" does not mean "your problem".
//!
//! - **Video**: `.y4m` (YUV4MPEG2) — a text header per stream and per frame, then raw planes.
//!   Every tool reads it, and the header carries the frame rate and aspect, so a re-encode does not
//!   need to be told them separately.
//! - **Audio**: `.wav` — a 44-byte header then interleaved 16-bit PCM, which is what the emulator
//!   already produces.
//!
//! # A/V sync
//!
//! The two streams are written from the same per-frame call, and each carries its own count. Video
//! is frame-locked by construction; audio drift is bounded by the emulator producing a fixed number
//! of samples per frame. [`Recorder::sync_drift_samples`] reports the accumulated difference so a
//! recording that *did* drift says so rather than being silently out of sync.

#![cfg(not(target_arch = "wasm32"))]

use std::fs::File;
use std::io::{BufWriter, Write};
use std::path::{Path, PathBuf};

/// The emulator's audio sample rate.
pub const SAMPLE_RATE: u32 = 32_000;

/// A running A/V capture.
pub struct Recorder {
    video: BufWriter<File>,
    audio: BufWriter<File>,
    video_path: PathBuf,
    audio_path: PathBuf,
    /// Frames written.
    frames: u64,
    /// Audio frames (sample pairs) written.
    samples: u64,
    /// The video dimensions the stream header declared. A frame of any other size is refused —
    /// Y4M has one size for the whole stream, and a hi-res toggle mid-recording would otherwise
    /// write a frame the header lies about.
    dims: (u32, u32),
    /// The declared frame rate, for the ffmpeg hint.
    fps: f64,
}

impl Recorder {
    /// Start a capture, writing `<stem>.y4m` and `<stem>.wav`.
    ///
    /// # Errors
    /// If either file cannot be created or its header cannot be written.
    pub fn start(stem: &Path, dims: (u32, u32), fps: f64) -> std::io::Result<Self> {
        if let Some(dir) = stem.parent()
            && !dir.as_os_str().is_empty()
        {
            std::fs::create_dir_all(dir)?;
        }
        let video_path = stem.with_extension("y4m");
        let audio_path = stem.with_extension("wav");
        let mut video = BufWriter::new(File::create(&video_path)?);
        let mut audio = BufWriter::new(File::create(&audio_path)?);

        // Y4M stream header. The frame rate is written as an exact rational (scaled by 1000)
        // rather than rounded: NTSC is 60.0988 Hz, and a `60:1` header makes a 20-minute recording
        // drift about two seconds against its own audio.
        #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
        let rate_num = (fps * 1000.0).round() as u64;
        writeln!(
            video,
            "YUV4MPEG2 W{} H{} F{rate_num}:1000 Ip A1:1 C444",
            dims.0, dims.1
        )?;

        // WAV header with placeholder sizes, patched in `finish`. Streaming to a file means the
        // final length is unknown when the header is written, which is what the patch is for.
        write_wav_header(&mut audio, 0)?;

        Ok(Self {
            video,
            audio,
            video_path,
            audio_path,
            frames: 0,
            samples: 0,
            dims,
            fps,
        })
    }

    /// Frames captured so far.
    #[must_use]
    pub const fn frames(&self) -> u64 {
        self.frames
    }

    /// The video file being written.
    #[must_use]
    pub fn video_path(&self) -> &Path {
        &self.video_path
    }

    /// Accumulated A/V drift, in audio frames.
    ///
    /// Positive means audio is ahead. Reported rather than corrected: silently resampling to hide
    /// drift would make a genuinely broken capture look fine, and the number is what tells you
    /// whether the recording is usable.
    #[must_use]
    // Frame and sample counts stay far inside `f64`'s exact-integer range for any real recording
    // (2^53 frames is ~4.7 million years at 60 Hz), so the conversions are exact in practice.
    #[allow(
        clippy::cast_possible_truncation,
        clippy::cast_sign_loss,
        clippy::cast_precision_loss,
        clippy::cast_possible_wrap
    )]
    pub fn sync_drift_samples(&self) -> i64 {
        let expected = (self.frames as f64 / self.fps * f64::from(SAMPLE_RATE)).round() as i64;
        self.samples as i64 - expected
    }

    /// Append one frame of video and its audio.
    ///
    /// `rgba` must be `w * h * 4` bytes at the dimensions the stream was started with.
    ///
    /// # Errors
    /// If a write fails, or if the frame's dimensions differ from the stream header's — Y4M has one
    /// size for the whole stream, so writing a differently-sized frame would produce a file whose
    /// header lies about its own contents.
    pub fn push(
        &mut self,
        rgba: &[u8],
        dims: (u32, u32),
        audio: &[(i16, i16)],
    ) -> std::io::Result<()> {
        if dims != self.dims {
            return Err(std::io::Error::new(
                std::io::ErrorKind::InvalidInput,
                format!(
                    "frame is {}x{} but the stream declared {}x{} (a resolution change mid-\
                     recording needs a new recording)",
                    dims.0, dims.1, self.dims.0, self.dims.1
                ),
            ));
        }
        let expected = (dims.0 as usize) * (dims.1 as usize) * 4;
        #[allow(clippy::items_after_statements)]
        if rgba.len() < expected {
            return Err(std::io::Error::new(
                std::io::ErrorKind::InvalidInput,
                "frame buffer is shorter than its declared dimensions",
            ));
        }

        self.video.write_all(b"FRAME\n")?;
        let (y, u, v) = rgb_to_yuv444(rgba, expected);
        self.video.write_all(&y)?;
        self.video.write_all(&u)?;
        self.video.write_all(&v)?;
        self.frames += 1;

        for (l, r) in audio {
            self.audio.write_all(&l.to_le_bytes())?;
            self.audio.write_all(&r.to_le_bytes())?;
        }
        self.samples += audio.len() as u64;
        Ok(())
    }

    /// Finish both files and return an `ffmpeg` command that muxes them.
    ///
    /// # Errors
    /// If a flush or the WAV header patch fails.
    pub fn finish(mut self) -> std::io::Result<String> {
        self.video.flush()?;
        self.audio.flush()?;
        // The WAV header's two size fields could not be known when it was written; patch them now
        // that the stream is complete. A player reading the placeholder would report a zero-length
        // file, which is indistinguishable from a failed capture.
        let data_bytes = self.samples * 4;
        let mut file = self.audio.into_inner().map_err(std::io::Error::other)?;
        patch_wav_sizes(&mut file, data_bytes)?;
        file.sync_all()?;
        Ok(format!(
            "ffmpeg -i {} -i {} -c:v libx264 -crf 18 -c:a aac out.mp4",
            self.video_path.display(),
            self.audio_path.display()
        ))
    }
}

/// Convert an RGBA8 frame to three full-resolution YUV planes (Y4M `C444`).
///
/// 4:4:4 rather than 4:2:0: chroma subsampling would throw away colour detail on an image whose
/// pixels are already the size of the artwork, and the file is a lossless intermediate — the
/// re-encode is where size is meant to be traded for quality.
///
/// BT.601 coefficients, matching what a consumer decoder assumes for standard-definition content.
#[must_use]
fn rgb_to_yuv444(rgba: &[u8], len: usize) -> (Vec<u8>, Vec<u8>, Vec<u8>) {
    let pixels = len / 4;
    let mut luma = Vec::with_capacity(pixels);
    let mut chroma_b = Vec::with_capacity(pixels);
    let mut chroma_r = Vec::with_capacity(pixels);
    for px in rgba.chunks_exact(4).take(pixels) {
        let (red, green, blue) = (f32::from(px[0]), f32::from(px[1]), f32::from(px[2]));
        let yy = 0.299f32.mul_add(red, 0.587f32.mul_add(green, 0.114 * blue));
        let uu = 0.492f32.mul_add(blue - yy, 128.0);
        let vv = 0.877f32.mul_add(red - yy, 128.0);
        #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
        {
            luma.push(yy.clamp(0.0, 255.0) as u8);
            chroma_b.push(uu.clamp(0.0, 255.0) as u8);
            chroma_r.push(vv.clamp(0.0, 255.0) as u8);
        }
    }
    (luma, chroma_b, chroma_r)
}

/// Write a 44-byte canonical WAV header for 16-bit stereo at [`SAMPLE_RATE`].
fn write_wav_header(out: &mut impl Write, data_bytes: u64) -> std::io::Result<()> {
    let data = u32::try_from(data_bytes).unwrap_or(u32::MAX);
    let byte_rate = SAMPLE_RATE * 4;
    out.write_all(b"RIFF")?;
    out.write_all(&(36 + data).to_le_bytes())?;
    out.write_all(b"WAVEfmt ")?;
    out.write_all(&16u32.to_le_bytes())?; // PCM fmt chunk size
    out.write_all(&1u16.to_le_bytes())?; // PCM
    out.write_all(&2u16.to_le_bytes())?; // stereo
    out.write_all(&SAMPLE_RATE.to_le_bytes())?;
    out.write_all(&byte_rate.to_le_bytes())?;
    out.write_all(&4u16.to_le_bytes())?; // block align
    out.write_all(&16u16.to_le_bytes())?; // bits per sample
    out.write_all(b"data")?;
    out.write_all(&data.to_le_bytes())?;
    Ok(())
}

/// Patch the two size fields a streamed WAV could not know up front.
fn patch_wav_sizes(file: &mut File, data_bytes: u64) -> std::io::Result<()> {
    use std::io::{Seek, SeekFrom};
    let data = u32::try_from(data_bytes).unwrap_or(u32::MAX);
    file.seek(SeekFrom::Start(4))?;
    file.write_all(&(36 + data).to_le_bytes())?;
    file.seek(SeekFrom::Start(40))?;
    file.write_all(&data.to_le_bytes())?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::{Recorder, SAMPLE_RATE, rgb_to_yuv444};

    fn tmp(name: &str) -> std::path::PathBuf {
        std::env::temp_dir().join(format!("rustysnes-av-test-{name}"))
    }

    fn frame(w: u32, h: u32, colour: [u8; 4]) -> Vec<u8> {
        colour
            .iter()
            .copied()
            .cycle()
            .take((w * h * 4) as usize)
            .collect::<Vec<u8>>()
            .chunks_exact(4)
            .flat_map(|_| colour)
            .collect()
    }

    #[test]
    fn writes_a_playable_y4m_and_wav_pair() {
        let stem = tmp("basic");
        let (w, h) = (16u32, 8u32);
        let mut rec = Recorder::start(&stem, (w, h), 60.0988).expect("start");
        let img = frame(w, h, [255, 0, 0, 255]);
        for _ in 0..3 {
            rec.push(&img, (w, h), &[(100, -100); 533]).expect("push");
        }
        assert_eq!(rec.frames(), 3);
        let hint = rec.finish().expect("finish");
        assert!(hint.contains("ffmpeg"), "{hint}");

        let y4m = std::fs::read(stem.with_extension("y4m")).expect("read y4m");
        let header_end = y4m.iter().position(|b| *b == b'\n').expect("header");
        let header = std::str::from_utf8(&y4m[..header_end]).expect("utf8");
        assert!(header.starts_with("YUV4MPEG2"), "{header}");
        assert!(header.contains("W16 H8"), "{header}");
        // The exact rational, not a rounded 60:1 — a 20-minute capture drifts ~2s otherwise.
        assert!(header.contains("F60099:1000"), "{header}");
        // Three FRAME markers, each followed by three full planes.
        let frames = y4m.windows(6).filter(|w| *w == b"FRAME\n").count();
        assert_eq!(frames, 3);
        let expected_len = header_end + 1 + 3 * (6 + (w * h * 3) as usize);
        assert_eq!(y4m.len(), expected_len, "plane sizes must be exact");

        let wav = std::fs::read(stem.with_extension("wav")).expect("read wav");
        assert_eq!(&wav[0..4], b"RIFF");
        assert_eq!(&wav[8..12], b"WAVE");
        // The size fields must be PATCHED, not left at the placeholder — a player reading zero
        // reports an empty file, indistinguishable from a failed capture.
        let data_size = u32::from_le_bytes(wav[40..44].try_into().expect("4 bytes"));
        assert_eq!(data_size, 3 * 533 * 4);
        let riff_size = u32::from_le_bytes(wav[4..8].try_into().expect("4 bytes"));
        assert_eq!(riff_size, 36 + data_size);
        assert_eq!(wav.len(), 44 + data_size as usize);
        // Sample rate is the emulator's.
        assert_eq!(
            u32::from_le_bytes(wav[24..28].try_into().expect("4 bytes")),
            SAMPLE_RATE
        );

        let _ = std::fs::remove_file(stem.with_extension("y4m"));
        let _ = std::fs::remove_file(stem.with_extension("wav"));
    }

    /// A resolution change mid-recording is refused: Y4M declares one size for the whole stream,
    /// so writing a differently-sized frame produces a file whose header lies about its contents.
    #[test]
    fn a_resolution_change_is_refused_not_silently_written() {
        let stem = tmp("resize");
        let mut rec = Recorder::start(&stem, (16, 8), 60.0).expect("start");
        rec.push(&frame(16, 8, [1, 2, 3, 255]), (16, 8), &[])
            .expect("same size");
        let err = rec
            .push(&frame(32, 8, [1, 2, 3, 255]), (32, 8), &[])
            .expect_err("different size");
        assert!(err.to_string().contains("declared"), "{err}");
        // A short buffer is refused too, rather than writing garbage planes.
        let err = rec.push(&[0u8; 4], (16, 8), &[]).expect_err("short buffer");
        assert!(err.to_string().contains("shorter"), "{err}");
        let _ = rec.finish();
        let _ = std::fs::remove_file(stem.with_extension("y4m"));
        let _ = std::fs::remove_file(stem.with_extension("wav"));
    }

    /// Drift is reported, not corrected — silently resampling would make a broken capture look
    /// fine, and the number is what says whether the recording is usable.
    #[test]
    fn drift_is_measured_against_the_declared_frame_rate() {
        let stem = tmp("drift");
        let mut rec = Recorder::start(&stem, (8, 8), 60.0).expect("start");
        let img = frame(8, 8, [0, 0, 0, 255]);
        // Exactly the right number of samples for 60 frames at 60 fps.
        let per_frame = (SAMPLE_RATE / 60) as usize;
        for _ in 0..60 {
            rec.push(&img, (8, 8), &vec![(0, 0); per_frame])
                .expect("push");
        }
        assert!(
            rec.sync_drift_samples().abs() < 64,
            "drift {} should be near zero",
            rec.sync_drift_samples()
        );

        // Now starve the audio: drift must go negative and say so.
        for _ in 0..10 {
            rec.push(&img, (8, 8), &[]).expect("push");
        }
        assert!(
            rec.sync_drift_samples() < -1000,
            "{}",
            rec.sync_drift_samples()
        );
        let _ = rec.finish();
        let _ = std::fs::remove_file(stem.with_extension("y4m"));
        let _ = std::fs::remove_file(stem.with_extension("wav"));
    }

    /// The colour conversion must round-trip the primaries recognisably, or every recording is
    /// subtly wrong in a way nobody notices until it is played back.
    #[test]
    fn yuv_conversion_places_the_primaries_correctly() {
        let one = |c: [u8; 4]| {
            let (y, u, v) = rgb_to_yuv444(&c, 4);
            (y[0], u[0], v[0])
        };
        // Black and white sit at the ends of the luma range with neutral chroma.
        let (y, u, v) = one([0, 0, 0, 255]);
        assert_eq!(y, 0);
        assert!(u.abs_diff(128) <= 1 && v.abs_diff(128) <= 1);
        let (y, u, v) = one([255, 255, 255, 255]);
        assert_eq!(y, 255);
        assert!(u.abs_diff(128) <= 1 && v.abs_diff(128) <= 1);
        // Red is high V, blue is high U — the defining property of the colour difference planes.
        let (_, _, v_red) = one([255, 0, 0, 255]);
        assert!(v_red > 200, "red should have high V, got {v_red}");
        let (_, u_blue, _) = one([0, 0, 255, 255]);
        assert!(u_blue > 200, "blue should have high U, got {u_blue}");
        // And green sits low on both.
        let (_, u_g, v_g) = one([0, 255, 0, 255]);
        assert!(u_g < 128 && v_g < 128, "green: u={u_g} v={v_g}");
    }
}
