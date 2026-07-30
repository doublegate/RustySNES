//! Opt-in per-present CSV performance log (`v1.25.0`, T-FP-B).
//!
//! [`crate::perf`] answers "how is it running *now*" over a ~5 s window. This answers the question
//! that window cannot: "what happened during that 20-minute session, and does this build regress
//! against the last one?" A hitch that happened four minutes ago is gone from the ring buffer but
//! is still a row here.
//!
//! Deliberately CSV rather than a binary trace format: the point is that the file opens in anything
//! (a spreadsheet, `csvlens`, pandas, gnuplot) without this project shipping a reader for it.
//!
//! **Off unless the user asks for it.** Writing a row per present is a syscall per present, which is
//! exactly the kind of thing that perturbs the measurement it is taking, so nothing here runs — and
//! no file is created — until the Performance panel's toggle turns it on. Native only: `wasm32` has
//! no filesystem to write to.

#![cfg(not(target_arch = "wasm32"))]

use std::fs::File;
use std::io::{BufWriter, Write};
use std::path::{Path, PathBuf};

/// The CSV header, also the authoritative column order for [`PerfLog::row`].
///
/// `frame` is the present index, not an emulated-frame index: `produced` records how many emulated
/// frames that present ran, so a catch-up burst is visible as a row with `produced > 1` rather than
/// being flattened away.
pub const HEADER: &str = "frame,elapsed_s,produce_ms,present_ms,gpu_ms,audio_pct,produced,fps";

/// A buffered CSV writer for one session's performance rows.
pub struct PerfLog {
    writer: BufWriter<File>,
    path: PathBuf,
    /// Presents written so far (the `frame` column).
    rows: u64,
    /// Wall-clock start, for the `elapsed_s` column.
    start: web_time::Instant,
    /// Rows written since the last flush — see [`Self::FLUSH_EVERY`].
    since_flush: u32,
}

impl PerfLog {
    /// How many rows to buffer before flushing.
    ///
    /// A flush per row would make the logger's own I/O a visible cost in the numbers it is
    /// recording; never flushing would lose the tail of a session that ends in a crash — which is
    /// precisely the session worth having a log of. ~1 s at 60 Hz is the compromise.
    const FLUSH_EVERY: u32 = 60;

    /// Create (truncating) `path` and write the header.
    ///
    /// # Errors
    /// If the file cannot be created or the header cannot be written.
    pub fn create(path: impl AsRef<Path>) -> std::io::Result<Self> {
        let path = path.as_ref().to_path_buf();
        if let Some(dir) = path.parent()
            && !dir.as_os_str().is_empty()
        {
            std::fs::create_dir_all(dir)?;
        }
        let mut writer = BufWriter::new(File::create(&path)?);
        writeln!(writer, "{HEADER}")?;
        Ok(Self {
            writer,
            path,
            rows: 0,
            start: web_time::Instant::now(),
            since_flush: 0,
        })
    }

    /// The file being written, for the panel to display.
    #[must_use]
    pub fn path(&self) -> &Path {
        &self.path
    }

    /// Rows written so far.
    #[must_use]
    pub const fn rows(&self) -> u64 {
        self.rows
    }

    /// Append one present's row.
    ///
    /// A `None` measurement is written as an **empty field**, not `0` — an idle present did not
    /// take zero milliseconds to emulate, it emulated nothing, and every consumer of a CSV treats
    /// an empty cell as missing while `0` silently drags an average down.
    ///
    /// # Errors
    /// If the underlying write fails.
    pub fn row(
        &mut self,
        produce_ms: Option<f32>,
        present_ms: f32,
        gpu_ms: Option<f32>,
        audio_pct: Option<f32>,
        produced: u32,
        fps: f32,
    ) -> std::io::Result<()> {
        let f = |v: Option<f32>| v.map_or_else(String::new, |v| format!("{v:.3}"));
        writeln!(
            self.writer,
            "{},{:.3},{},{present_ms:.3},{},{},{produced},{fps:.2}",
            self.rows,
            self.start.elapsed().as_secs_f32(),
            f(produce_ms),
            f(gpu_ms),
            f(audio_pct),
        )?;
        self.rows += 1;
        self.since_flush += 1;
        if self.since_flush >= Self::FLUSH_EVERY {
            self.since_flush = 0;
            self.writer.flush()?;
        }
        Ok(())
    }

    /// Flush pending rows to disk (called when the log is turned off, and on drop).
    ///
    /// # Errors
    /// If the underlying flush fails.
    pub fn flush(&mut self) -> std::io::Result<()> {
        self.since_flush = 0;
        self.writer.flush()
    }
}

impl Drop for PerfLog {
    fn drop(&mut self) {
        // Best-effort: a failure here has nowhere useful to go, and losing the last <1 s of a
        // performance log must never take down the frontend on exit.
        let _ = self.writer.flush();
    }
}

/// The default log path for a session: `<screenshot dir sibling>/rustysnes-perf-<n>.csv`.
///
/// Placed next to the config directory rather than the CWD so a log is findable after the fact
/// regardless of where the binary was launched from — the same reasoning as
/// [`crate::screenshot::screenshot_dir`].
#[must_use]
pub fn default_log_path(session: u32) -> PathBuf {
    let dir = directories::BaseDirs::new().map_or_else(
        || PathBuf::from("."),
        |d| d.data_dir().join("rustysnes").join("perf"),
    );
    dir.join(format!("rustysnes-perf-{session}.csv"))
}

#[cfg(test)]
mod tests {
    use super::{HEADER, PerfLog};

    fn tmp(name: &str) -> std::path::PathBuf {
        std::env::temp_dir().join(format!("rustysnes-perflog-test-{name}.csv"))
    }

    #[test]
    fn writes_header_and_rows_in_column_order() {
        let path = tmp("basic");
        {
            let mut log = PerfLog::create(&path).expect("create");
            log.row(Some(12.5), 2.25, Some(1.5), Some(50.0), 1, 60.1)
                .expect("row");
            log.row(Some(11.0), 2.0, None, Some(49.0), 2, 60.0)
                .expect("row");
            assert_eq!(log.rows(), 2);
            log.flush().expect("flush");
        }
        let text = std::fs::read_to_string(&path).expect("read");
        let mut lines = text.lines();
        assert_eq!(lines.next().expect("header"), HEADER);
        let first = lines.next().expect("row 0");
        assert_eq!(
            first.split(',').count(),
            HEADER.split(',').count(),
            "row must have exactly one field per header column: {first}"
        );
        assert!(first.starts_with("0,"), "{first}");
        assert!(first.contains("12.500"), "{first}");
        let second = lines.next().expect("row 1");
        assert!(second.starts_with("1,"), "{second}");
        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn missing_measurements_are_empty_fields_not_zeros() {
        // A paused present emulated nothing; writing 0 would drag every downstream average toward
        // zero and make an idle session look impossibly fast.
        let path = tmp("missing");
        {
            let mut log = PerfLog::create(&path).expect("create");
            log.row(None, 1.0, None, None, 0, 0.0).expect("row");
            log.flush().expect("flush");
        }
        let text = std::fs::read_to_string(&path).expect("read");
        let row = text.lines().nth(1).expect("row 0");
        let fields: Vec<&str> = row.split(',').collect();
        assert_eq!(fields.len(), HEADER.split(',').count());
        // produce_ms, gpu_ms, audio_pct are columns 2, 4, 5.
        for idx in [2, 4, 5] {
            assert!(
                fields[idx].is_empty(),
                "column {idx} should be empty, got {:?} in {row}",
                fields[idx]
            );
        }
        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn dropping_the_log_flushes_buffered_rows() {
        // The session worth having a log of is the one that ended badly, so the tail must survive
        // without an explicit flush.
        let path = tmp("drop");
        {
            let mut log = PerfLog::create(&path).expect("create");
            log.row(Some(1.0), 1.0, None, None, 1, 60.0).expect("row");
        }
        let text = std::fs::read_to_string(&path).expect("read");
        assert_eq!(text.lines().count(), 2, "header + one row: {text:?}");
        let _ = std::fs::remove_file(&path);
    }
}
