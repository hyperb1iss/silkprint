//! Inline image loading for the TUI.
//!
//! Decodes local images and keeps the decoded pixels so a band can be scrolled
//! through: the widget draws only the vertical slice currently in the viewport
//! (ratatui-image can't clip a partially scrolled image, so we crop the source
//! ourselves and build a protocol for just the visible rows). Protocols target
//! Kitty / iTerm2 / Sixel where the terminal supports them, halfblocks else.

use std::collections::{HashMap, VecDeque};
use std::path::{Path, PathBuf};

use image::DynamicImage;
use ratatui_image::picker::Picker;
use ratatui_image::protocol::StatefulProtocol;

/// A decoded image plus its source pixel dimensions. The pixels are retained so
/// a tall band can be cropped to whatever vertical slice is currently visible.
pub struct Loaded {
    pub image: DynamicImage,
    pub width: u32,
    pub height: u32,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
struct SliceKey {
    src: String,
    start_row: u16,
    rows: u16,
    band_rows: u16,
}

/// The protocol for a visible image slice. A small FIFO cache keeps recent
/// slices warm while scrolling back and forth without retaining every row of a
/// tall image forever.
struct SliceProto {
    proto: StatefulProtocol,
}

/// Where a loaded image sits in the (reserved) content flow.
#[derive(Debug, Clone)]
pub struct Placement {
    pub src: String,
    pub line: u16,
    pub rows: u16,
}

/// Loads and caches inline images, keyed by source path.
pub struct ImageStore {
    picker: Option<Picker>,
    base_dir: Option<PathBuf>,
    cache: HashMap<String, Option<Loaded>>,
    slices: HashMap<SliceKey, SliceProto>,
    slice_order: VecDeque<SliceKey>,
    /// Terminal cell size in pixels, for sizing reserved row bands.
    cell: (u32, u32),
}

impl ImageStore {
    pub fn new(picker: Option<Picker>, base_dir: Option<PathBuf>) -> Self {
        let cell = picker.as_ref().map_or((8, 16), |p| {
            let fs = p.font_size();
            (u32::from(fs.width.max(1)), u32::from(fs.height.max(1)))
        });
        Self {
            picker,
            base_dir,
            cache: HashMap::new(),
            slices: HashMap::new(),
            slice_order: VecDeque::new(),
            cell,
        }
    }

    /// Whether a graphics-capable picker is available.
    pub fn enabled(&self) -> bool {
        self.picker.is_some()
    }

    /// Terminal cell size in pixels.
    pub fn cell(&self) -> (u32, u32) {
        self.cell
    }

    /// Drop cached images and slice protocols (e.g. on live reload, where the
    /// underlying files may have changed).
    pub fn clear_cache(&mut self) {
        self.cache.clear();
        self.slices.clear();
        self.slice_order.clear();
    }

    /// Drop generated rasters while preserving decoded document images.
    pub fn clear_generated(&mut self) {
        self.cache
            .retain(|key, _loaded| !key.starts_with(GENERATED_KEY_PREFIX));
        self.slices
            .retain(|key, _proto| !key.src.starts_with(GENERATED_KEY_PREFIX));
        self.slice_order
            .retain(|key| !key.src.starts_with(GENERATED_KEY_PREFIX));
    }

    /// Ensure a generated image (e.g. a mermaid diagram) is cached under `key`,
    /// building it on first request.
    pub fn ensure_generated(
        &mut self,
        key: &str,
        build: impl FnOnce() -> Option<DynamicImage>,
    ) -> Option<&mut Loaded> {
        if !self.cache.contains_key(key) {
            let loaded = self.picker.as_ref().and_then(|_picker| {
                let image = build()?;
                Some(Loaded {
                    width: image.width(),
                    height: image.height(),
                    image,
                })
            });
            self.cache.insert(key.to_string(), loaded);
        }
        self.cache.get_mut(key).and_then(Option::as_mut)
    }

    /// Build (or reuse) the protocol for the vertical slice of `src` spanning
    /// band rows `[start_row, start_row + rows)` of a `band_rows`-tall band.
    /// Crops the source to the matching pixel range so the visible portion of a
    /// tall image draws correctly while scrolling. `None` if `src` isn't loaded.
    pub fn slice_protocol(
        &mut self,
        src: &str,
        start_row: u16,
        rows: u16,
        band_rows: u16,
    ) -> Option<&mut StatefulProtocol> {
        let picker = self.picker.as_ref()?;
        let loaded = self.cache.get(src).and_then(Option::as_ref)?;
        let key = SliceKey {
            src: src.to_string(),
            start_row,
            rows,
            band_rows,
        };
        if !self.slices.contains_key(&key) {
            let (y0, y1) = slice_bounds(loaded.height, start_row, rows, band_rows)?;
            let crop = loaded.image.crop_imm(0, y0, loaded.width, y1 - y0);
            let proto = picker.new_resize_protocol(crop);
            while self.slices.len() >= MAX_SLICE_PROTOS {
                let Some(oldest) = self.slice_order.pop_front() else {
                    break;
                };
                self.slices.remove(&oldest);
            }
            self.slice_order.push_back(key.clone());
            self.slices.insert(key.clone(), SliceProto { proto });
        }
        self.slices.get_mut(&key).map(|s| &mut s.proto)
    }

    /// Get a cached image (loading it on first request). `None` if the source
    /// is remote-blocked, missing, jailed, or undecodable.
    pub fn get(&mut self, src: &str) -> Option<&mut Loaded> {
        if !self.cache.contains_key(src) {
            let loaded = self.load(src);
            self.cache.insert(src.to_string(), loaded);
        }
        self.cache.get_mut(src).and_then(Option::as_mut)
    }

    fn load(&self, src: &str) -> Option<Loaded> {
        // Only load when a graphics protocol is available to draw it.
        self.picker.as_ref()?;
        let bytes = if src.starts_with("http://") || src.starts_with("https://") {
            fetch_remote(src)?
        } else {
            let path = resolve(src, self.base_dir.as_deref())?;
            std::fs::read(path).ok()?
        };
        let mut reader = image::ImageReader::new(std::io::Cursor::new(bytes))
            .with_guessed_format()
            .ok()?;
        // Bound decode work to guard against decompression bombs from untrusted
        // documents.
        let mut limits = image::Limits::default();
        limits.max_image_width = Some(MAX_IMAGE_DIM);
        limits.max_image_height = Some(MAX_IMAGE_DIM);
        limits.max_alloc = Some(MAX_IMAGE_ALLOC);
        reader.limits(limits);

        let image = reader.decode().ok()?;
        Some(Loaded {
            width: image.width(),
            height: image.height(),
            image,
        })
    }
}

/// Max decoded image dimension (px per side) and total allocation.
const MAX_IMAGE_DIM: u32 = 8000;
const MAX_IMAGE_ALLOC: u64 = 256 * 1024 * 1024;
const MAX_SLICE_PROTOS: usize = 12;
const GENERATED_KEY_PREFIX: &str = "\u{0}";

/// Fetch a remote image's bytes, reusing the PDF pipeline's downloader.
/// (SVG bytes won't decode as a raster — those stay placeholders for now.)
#[cfg(not(target_arch = "wasm32"))]
fn fetch_remote(url: &str) -> Option<Vec<u8>> {
    crate::render::image::fetch_remote_image(url)
        .ok()
        .map(|(bytes, _ext)| bytes)
}

#[cfg(target_arch = "wasm32")]
fn fetch_remote(_url: &str) -> Option<Vec<u8>> {
    None
}

/// Resolve a local image path, jailed to the document's directory.
///
/// Untrusted documents must not read arbitrary files, so absolute paths are
/// rejected and the canonicalized target (which resolves `..` and symlinks)
/// must stay within the canonicalized base directory.
fn resolve(src: &str, base: Option<&Path>) -> Option<PathBuf> {
    if Path::new(src).is_absolute() {
        return None;
    }
    let canon_base = base?.canonicalize().ok()?;
    let candidate = canon_base.join(src).canonicalize().ok()?;
    candidate.starts_with(&canon_base).then_some(candidate)
}

/// Reserve the number of rows the image will actually occupy: its natural cell
/// size (pixels / `cell`), downscaled to fit `content_width` (never upscaled),
/// matching how ratatui-image's `Fit` renders. Bounded by `max_rows` so a band
/// cannot flood the content flow; over-tall bands are scrolled through by
/// cropping the visible source slice during draw.
pub(super) fn reserved_rows(
    width: u32,
    height: u32,
    content_width: u16,
    cell: (u32, u32),
    max_rows: u16,
) -> u16 {
    let (cell_w, cell_h) = (cell.0.max(1), cell.1.max(1));
    let natural_cols = width.max(1).div_ceil(cell_w).max(1);
    let natural_rows = height.max(1).div_ceil(cell_h).max(1);
    let limit = u32::from(content_width).max(1);
    let rows = if natural_cols <= limit {
        natural_rows
    } else {
        (natural_rows * limit / natural_cols).max(1)
    };
    let cap = u32::from(max_rows.max(1));
    u16::try_from(rows.min(cap)).unwrap_or(max_rows).max(1)
}

fn slice_bounds(height: u32, start_row: u16, rows: u16, band_rows: u16) -> Option<(u32, u32)> {
    if height == 0 || rows == 0 || band_rows == 0 {
        return None;
    }

    let band = u32::from(band_rows);
    let start = u32::from(start_row);
    if start >= band {
        return None;
    }

    let end = start.saturating_add(u32::from(rows)).min(band);
    if end <= start {
        return None;
    }

    let h = u64::from(height);
    let band_u64 = u64::from(band);
    let y0 = u32::try_from(u64::from(start) * h / band_u64)
        .ok()?
        .min(height - 1);
    let y1 = u32::try_from((u64::from(end) * h).div_ceil(band_u64))
        .ok()?
        .clamp(y0 + 1, height);
    Some((y0, y1))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reserved_rows_use_natural_size_then_downscale() {
        let cell = (8, 16);
        // 100x100 px at 8x16 cells → ~13 cols x 7 rows; fits in 40 cols → 7 rows.
        assert_eq!(reserved_rows(100, 100, 40, cell, 50), 7);
        // Wide banner (1000x100 → 125 cols) exceeds 60 → downscaled to a few rows.
        assert!(reserved_rows(1000, 100, 60, cell, 50) <= 4);
        // Never zero.
        assert!(reserved_rows(100, 1, 80, cell, 50) >= 1);
        // A tall diagram is clamped to the configured band cap, not the viewport.
        assert_eq!(reserved_rows(400, 4000, 80, cell, 30), 30);
    }

    #[test]
    fn slice_bounds_map_band_rows_to_pixels() {
        assert_eq!(slice_bounds(100, 0, 10, 100), Some((0, 10)));
        assert_eq!(slice_bounds(100, 90, 10, 100), Some((90, 100)));
        assert_eq!(slice_bounds(101, 1, 1, 3), Some((33, 68)));
    }

    #[test]
    fn slice_bounds_reject_bad_ranges_without_panicking() {
        assert_eq!(slice_bounds(100, 100, 1, 100), None);
        assert_eq!(slice_bounds(100, 0, 0, 100), None);
        assert_eq!(slice_bounds(100, 0, 1, 0), None);
        assert_eq!(slice_bounds(0, 0, 1, 100), None);
    }

    #[test]
    fn resolve_rejects_missing() {
        assert!(resolve("does-not-exist.png", Some(Path::new("/tmp"))).is_none());
    }
}
