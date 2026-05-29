//! Inline image loading for the TUI.
//!
//! Decodes local images and builds ratatui-image protocols (Kitty / iTerm2 /
//! Sixel where the terminal supports them, halfblocks otherwise). Each image is
//! given a reserved row band in the content flow; the widget is drawn over that
//! band when it scrolls fully into view.

use std::collections::HashMap;
use std::path::{Path, PathBuf};

use ratatui_image::picker::Picker;
use ratatui_image::protocol::StatefulProtocol;

const MAX_IMAGE_ROWS: u16 = 20;

/// A decoded image plus the number of content rows reserved for it.
pub struct Loaded {
    pub protocol: StatefulProtocol,
    pub rows: u16,
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

    /// Get a cached image (loading it on first request). `None` if the source
    /// is remote, missing, or undecodable.
    pub fn get(&mut self, src: &str, content_width: u16) -> Option<&mut Loaded> {
        if !self.cache.contains_key(src) {
            let loaded = self.load(src, content_width);
            self.cache.insert(src.to_string(), loaded);
        }
        self.cache.get_mut(src).and_then(Option::as_mut)
    }

    fn load(&self, src: &str, content_width: u16) -> Option<Loaded> {
        let picker = self.picker.as_ref()?;
        let image = if src.starts_with("http://") || src.starts_with("https://") {
            let bytes = fetch_remote(src)?;
            image::load_from_memory(&bytes).ok()?
        } else {
            let path = resolve(src, self.base_dir.as_deref())?;
            image::ImageReader::open(&path)
                .ok()?
                .with_guessed_format()
                .ok()?
                .decode()
                .ok()?
        };
        let rows = reserved_rows(image.width(), image.height(), content_width);
        let protocol = picker.new_resize_protocol(image);
        Some(Loaded { protocol, rows })
    }
}

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

fn resolve(src: &str, base: Option<&Path>) -> Option<PathBuf> {
    let path = Path::new(src);
    if path.is_absolute() {
        return path.exists().then(|| path.to_path_buf());
    }
    let joined = base?.join(path);
    joined.exists().then_some(joined)
}

/// Reserve a row band sized to the image's aspect ratio, assuming a terminal
/// cell is roughly twice as tall as it is wide. Integer math; capped.
fn reserved_rows(width: u32, height: u32, content_width: u16) -> u16 {
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
