//! Typst compilation engine — World trait implementation and PDF export.
//!
//! Implements SPEC Section 10.1: direct `World` trait impl against typst 0.14,
//! giving full control over font loading, file resolution, and compilation
//! without depending on a third-party wrapper.

use std::path::{Path, PathBuf};

use typst::foundations::{Bytes, Datetime};
use typst::layout::PagedDocument;
use typst::syntax::{FileId, Source};
use typst::text::{Font, FontBook};
use typst::utils::LazyHash;
use typst::{Library, LibraryExt, World};

use crate::error::SilkprintError;
use crate::theme::ResolvedTheme;
use crate::warnings::WarningCollector;

/// The virtual path where the tmTheme XML is served to Typst.
///
/// Referenced in the emitted Typst source as:
/// `#set raw(theme: "/__silkprint_theme.tmTheme")`
const TMTHEME_VPATH: &str = "/__silkprint_theme.tmTheme";

/// Typst world implementation for `SilkPrint`.
///
/// Provides the compiler with everything it needs: standard library, fonts,
/// source files, and file resolution rooted at the input document's directory.
struct SilkWorld {
    library: LazyHash<Library>,
    book: LazyHash<FontBook>,
    fonts: Vec<Font>,
    main_source: Source,
    main_id: FileId,
    root: PathBuf,
    tmtheme_data: Vec<u8>,
}

impl SilkWorld {
    /// Construct a new world from Typst source, font data, theme, and root directory.
    fn new(
        typst_source: &str,
        theme: &ResolvedTheme,
        root_dir: &Path,
        font_data: Vec<Vec<u8>>,
    ) -> Self {
        // Build the main source — detached (no package, virtual path "main.typ")
        let main_source = Source::detached(typst_source);
        let main_id = main_source.id();

        // Build the font book and font collection from raw font bytes
        let mut book = FontBook::new();
        let mut fonts = Vec::new();

        for data in font_data {
            let bytes = Bytes::new(data);
            for font in Font::iter(bytes) {
                book.push(font.info().clone());
                fonts.push(font);
            }
        }

        tracing::debug!(font_count = fonts.len(), "loaded fonts into SilkWorld");

        Self {
            library: LazyHash::new(Library::default()),
            book: LazyHash::new(book),
            fonts,
            main_source,
            main_id,
            root: root_dir.to_path_buf(),
            tmtheme_data: theme.tmtheme_xml.as_bytes().to_vec(),
        }
    }
}

impl World for SilkWorld {
    fn library(&self) -> &LazyHash<Library> {
        &self.library
    }

    fn book(&self) -> &LazyHash<FontBook> {
        &self.book
    }

    fn main(&self) -> FileId {
        self.main_id
    }

    fn source(&self, id: FileId) -> typst::diag::FileResult<Source> {
        if id == self.main_id {
            Ok(self.main_source.clone())
        } else {
            Err(typst::diag::FileError::NotFound(
                id.vpath().as_rooted_path().to_path_buf(),
            ))
        }
    }

    fn file(&self, id: FileId) -> typst::diag::FileResult<Bytes> {
        let vpath = id.vpath();
        let path_str = vpath.as_rooted_path().to_string_lossy();

        // Serve the virtual tmTheme file for syntax highlighting
        if path_str == TMTHEME_VPATH {
            return Ok(Bytes::new(self.tmtheme_data.clone()));
        }

        // Resolve relative to the document root directory (input file's parent)
        let resolved = vpath.resolve(&self.root).ok_or_else(|| {
            typst::diag::FileError::NotFound(vpath.as_rooted_path().to_path_buf())
        })?;

        let data = std::fs::read(&resolved)
            .map_err(|err| typst::diag::FileError::from_io(err, &resolved))?;

        Ok(Bytes::new(data))
    }

    fn font(&self, index: usize) -> Option<Font> {
        self.fonts.get(index).cloned()
    }

    fn today(&self, offset: Option<i64>) -> Option<Datetime> {
        // Use std::time to get the current UTC timestamp, then compute date components.
        // This avoids depending on the `time` or `chrono` crates directly.
        let now = std::time::SystemTime::now();
        let secs = now.duration_since(std::time::UNIX_EPOCH).ok()?.as_secs();

        // Apply UTC offset in hours if requested
        let offset_secs = offset.unwrap_or(0) * 3600;
        let adjusted = i64::try_from(secs).ok()?.checked_add(offset_secs)?;

        let (year, month, day, hour, minute, second) = unix_to_ymd_hms(adjusted);
        Datetime::from_ymd_hms(year, month, day, hour, minute, second)
    }
}

/// Convert a Unix timestamp (seconds since epoch) to (year, month, day, hour, minute, second).
///
/// Civil-time algorithm from Howard Hinnant's date library — handles all valid
/// Unix timestamps without external dependencies.
#[allow(
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    clippy::as_conversions
)]
fn unix_to_ymd_hms(secs: i64) -> (i32, u8, u8, u8, u8, u8) {
    let day_secs = secs.rem_euclid(86400);
    let hour = (day_secs / 3600) as u8;
    let minute = ((day_secs % 3600) / 60) as u8;
    let second = (day_secs % 60) as u8;

    // Days since epoch (civil day number from 1970-01-01)
    let mut days = secs.div_euclid(86400);

    // Shift epoch from 1970-01-01 to 0000-03-01 for easier calendar math
    days += 719_468;

    let era = days.div_euclid(146_097);
    let doe = days.rem_euclid(146_097); // day of era [0, 146096]
    let yoe = (doe - doe / 1460 + doe / 36524 - doe / 146_096) / 365;
    let y = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100); // day of year [0, 365]
    let mp = (5 * doy + 2) / 153; // month in [0, 11] starting from March
    let d = doy - (153 * mp + 2) / 5 + 1; // day [1, 31]
    let m = if mp < 10 { mp + 3 } else { mp - 9 }; // month [1, 12]
    let year = if m <= 2 { y + 1 } else { y };

    (year as i32, m as u8, d as u8, hour, minute, second)
}

/// Compile Typst source to PDF bytes.
///
/// This is the main entry point for Wave 3F. It:
/// 1. Loads bundled fonts
/// 2. Constructs a `SilkWorld` with all resources
/// 3. Compiles the Typst source to a paged document
/// 4. Exports the document to PDF bytes
pub fn compile_to_pdf(
    typst_source: &str,
    theme: &ResolvedTheme,
    root_dir: &Path,
    font_dirs: &[PathBuf],
    _warnings: &mut WarningCollector,
) -> Result<Vec<u8>, SilkprintError> {
    // Load bundled fonts (Inter, Source Serif 4, JetBrains Mono)
    let mut font_data = crate::fonts::load_bundled_fonts();
    tracing::debug!(font_files = font_data.len(), "loaded bundled font files");

    // Load fonts from user-specified directories
    for dir in font_dirs {
        if let Ok(entries) = std::fs::read_dir(dir) {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.extension().is_some_and(|ext| {
                    let e = ext.to_ascii_lowercase();
                    e == "ttf" || e == "otf" || e == "ttc" || e == "otc"
                }) {
                    if let Ok(data) = std::fs::read(&path) {
                        tracing::debug!(path = %path.display(), "loaded user font");
                        font_data.push(data);
                    }
                }
            }
        } else {
            tracing::warn!(dir = %dir.display(), "font directory not found");
        }
    }

    // Build the world
    let world = SilkWorld::new(typst_source, theme, root_dir, font_data);

    // Compile to a paged document
    let result = typst::compile::<PagedDocument>(&world);

    // Collect compilation warnings — font fallback misses are expected (debug level),
    // everything else gets warn level
    for diag in &result.warnings {
        let msg = diag.message.to_string();
        if msg.contains("unknown font family") {
            tracing::debug!(message = %msg, "Typst font fallback miss (expected)");
        } else {
            tracing::warn!(message = %msg, severity = ?diag.severity, "Typst compilation warning");
        }
    }

    // Handle compilation errors
    let document = result.output.map_err(|diagnostics| {
        let messages: Vec<String> = diagnostics
            .iter()
            .map(|d| {
                use std::fmt::Write;
                let mut msg = d.message.to_string();
                for hint in &d.hints {
                    let _ = write!(msg, "\n  hint: {hint}");
                }
                msg
            })
            .collect();

        tracing::error!(count = messages.len(), "Typst compilation failed");
        for msg in &messages {
            tracing::error!("{msg}");
        }

        SilkprintError::TypstCompilation {
            diagnostics: messages,
        }
    })?;

    // Build PDF options — only set timestamp, everything else default.
    // Title/author come from #set document() in the Typst source, NOT PdfOptions.
    let timestamp = build_utc_timestamp();

    let pdf_options = typst_pdf::PdfOptions {
        timestamp,
        ..Default::default()
    };

    // Export to PDF
    let pdf_bytes = typst_pdf::pdf(&document, &pdf_options).map_err(|diagnostics| {
        let messages: Vec<String> = diagnostics.iter().map(|d| d.message.to_string()).collect();

        tracing::error!(count = messages.len(), "PDF export failed");

        SilkprintError::TypstCompilation {
            diagnostics: messages,
        }
    })?;

    tracing::info!(bytes = pdf_bytes.len(), "PDF export complete");

    Ok(pdf_bytes)
}

/// Build a UTC timestamp for PDF metadata from the current system time.
fn build_utc_timestamp() -> Option<typst_pdf::Timestamp> {
    let now = std::time::SystemTime::now();
    let secs = now.duration_since(std::time::UNIX_EPOCH).ok()?.as_secs();
    let secs_i64 = i64::try_from(secs).ok()?;
    let (year, month, day, hour, minute, second) = unix_to_ymd_hms(secs_i64);
    let dt = Datetime::from_ymd_hms(year, month, day, hour, minute, second)?;
    Some(typst_pdf::Timestamp::new_utc(dt))
}
