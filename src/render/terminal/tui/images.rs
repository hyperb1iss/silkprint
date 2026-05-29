//! Inline image loading for the TUI.
//!
//! Decodes local images and builds ratatui-image protocols (Kitty / iTerm2 /
//! Sixel where the terminal supports them, halfblocks otherwise). Each image is
//! given a reserved row band in the content flow; the widget is drawn over that
//! band when it scrolls fully into view.

use std::collections::HashMap;
use std::path::{Path, PathBuf};

use image::DynamicImage;
use ratatui_image::picker::Picker;
use ratatui_image::protocol::StatefulProtocol;

const MAX_IMAGE_ROWS: u16 = 20;

/// A decoded image protocol plus its source pixel dimensions. Reserved rows are
/// computed fresh per content width (not cached) so resizes stay correct.
pub struct Loaded {
    pub protocol: StatefulProtocol,
    pub width: u32,
    pub height: u32,
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
}

impl ImageStore {
    pub fn new(picker: Option<Picker>, base_dir: Option<PathBuf>) -> Self {
        Self {
            picker,
            base_dir,
            cache: HashMap::new(),
        }
    }

    /// Whether a graphics-capable picker is available.
    pub fn enabled(&self) -> bool {
        self.picker.is_some()
    }

    /// Drop cached protocols (e.g. on live reload, where images may have changed).
    pub fn clear_cache(&mut self) {
        self.cache.clear();
    }

    /// Ensure a generated image (e.g. a rasterized heading) is cached under
    /// `key`, building it on first request.
    pub fn ensure_generated(
        &mut self,
        key: &str,
        build: impl FnOnce() -> Option<DynamicImage>,
    ) -> Option<&mut Loaded> {
        if !self.cache.contains_key(key) {
            let loaded = self.picker.as_ref().and_then(|picker| {
                let image = build()?;
                Some(Loaded {
                    width: image.width(),
                    height: image.height(),
                    protocol: picker.new_resize_protocol(image),
                })
            });
            self.cache.insert(key.to_string(), loaded);
        }
        self.cache.get_mut(key).and_then(Option::as_mut)
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
        let picker = self.picker.as_ref()?;
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
            protocol: picker.new_resize_protocol(image),
        })
    }
}

/// Max decoded image dimension (px per side) and total allocation.
const MAX_IMAGE_DIM: u32 = 8000;
const MAX_IMAGE_ALLOC: u64 = 256 * 1024 * 1024;

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

/// Reserve a row band sized to the image's aspect ratio, assuming a terminal
/// cell is roughly twice as tall as it is wide. Integer math; capped.
pub(super) fn reserved_rows(width: u32, height: u32, content_width: u16) -> u16 {
    let width = u64::from(width.max(1));
    let height = u64::from(height);
    let rows = u64::from(content_width) * height / width / 2;
    u16::try_from(rows.clamp(1, u64::from(MAX_IMAGE_ROWS))).unwrap_or(MAX_IMAGE_ROWS)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reserved_rows_scales_with_aspect_and_caps() {
        // Square image at width 40 → ~20 rows, capped at MAX.
        assert_eq!(reserved_rows(100, 100, 40), MAX_IMAGE_ROWS);
        // Wide banner stays short.
        assert!(reserved_rows(1000, 100, 60) < 6);
        // Never zero.
        assert!(reserved_rows(100, 1, 80) >= 1);
    }

    #[test]
    fn resolve_rejects_missing() {
        assert!(resolve("does-not-exist.png", Some(Path::new("/tmp"))).is_none());
    }
}
