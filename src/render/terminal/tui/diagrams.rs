//! Rasterize mermaid diagrams for the TUI.
//!
//! Reuses the PDF pipeline's native mermaid → SVG renderer, then rasterizes the
//! SVG with resvg (already in the tree via typst) onto an opaque page-colored
//! background so the result is a flat RGBA image ready for ratatui-image.

use std::sync::{Arc, LazyLock};

use image::{DynamicImage, RgbaImage};
use resvg::tiny_skia;
use resvg::usvg;

use crate::theme::ResolvedTheme;

use super::super::model::Rgb;

/// Bundled-font database for SVG text, built once and shared across diagrams.
/// Parsing the bundled fonts (which include the multi-MB color emoji font)
/// takes ~0.5s, so rebuilding it per diagram made multi-diagram documents
/// stutter on open.
static FONTDB: LazyLock<Arc<usvg::fontdb::Database>> = LazyLock::new(|| {
    let mut fontdb = usvg::fontdb::Database::new();
    for font in crate::fonts::load_bundled_fonts() {
        fontdb.load_font_data(font);
    }
    Arc::new(fontdb)
});

const TARGET_WIDTH_PX: f32 = 760.0;
/// Reject oversized mermaid input and cap the rasterized output dimensions so
/// untrusted documents can't drive unbounded render/allocation work.
const MAX_MERMAID_BYTES: usize = 32 * 1024;
const MAX_RASTER_DIM: f32 = 2000.0;

/// Render a mermaid source to a rasterized image, or `None` on failure.
pub fn mermaid_image(source: &str, theme: &ResolvedTheme, bg: Rgb) -> Option<DynamicImage> {
    if source.len() > MAX_MERMAID_BYTES {
        return None;
    }
    let svg = crate::render::mermaid::render_one(source, theme)?;
    svg_to_image(&svg, bg)
}

// SVG sizing is f32 metrics mapped onto integer pixel dimensions.
#[allow(
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    clippy::cast_precision_loss,
    clippy::as_conversions
)]
fn svg_to_image(svg: &[u8], bg: Rgb) -> Option<DynamicImage> {
    let options = usvg::Options {
        fontdb: FONTDB.clone(),
        ..usvg::Options::default()
    };

    let tree = usvg::Tree::from_data(svg, &options).ok()?;
    let size = tree.size();
    let scale = (TARGET_WIDTH_PX / size.width()).clamp(0.2, 4.0);
    let width = (size.width() * scale).ceil().clamp(1.0, MAX_RASTER_DIM) as u32;
    let height = (size.height() * scale).ceil().clamp(1.0, MAX_RASTER_DIM) as u32;

    let mut pixmap = tiny_skia::Pixmap::new(width, height)?;
    pixmap.fill(tiny_skia::Color::from_rgba8(bg.0, bg.1, bg.2, 255));
    resvg::render(
        &tree,
        tiny_skia::Transform::from_scale(scale, scale),
        &mut pixmap.as_mut(),
    );

    // Opaque background → premultiplied RGBA equals straight RGBA.
    RgbaImage::from_raw(width, height, pixmap.data().to_vec()).map(DynamicImage::ImageRgba8)
}
