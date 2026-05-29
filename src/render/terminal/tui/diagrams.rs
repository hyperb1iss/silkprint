//! Rasterize mermaid diagrams for the TUI.
//!
//! Reuses the PDF pipeline's native mermaid → SVG renderer, then rasterizes the
//! SVG with resvg (already in the tree via typst) onto an opaque page-colored
//! background so the result is a flat RGBA image ready for ratatui-image.

use image::{DynamicImage, RgbaImage};
use resvg::tiny_skia;
use resvg::usvg;

use crate::theme::ResolvedTheme;

use super::super::model::Rgb;

const TARGET_WIDTH_PX: f32 = 760.0;

/// Render a mermaid source to a rasterized image, or `None` on failure.
pub fn mermaid_image(source: &str, theme: &ResolvedTheme, bg: Rgb) -> Option<DynamicImage> {
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
    let mut fontdb = usvg::fontdb::Database::new();
    for font in crate::fonts::load_bundled_fonts() {
        fontdb.load_font_data(font);
    }
    let options = usvg::Options {
        fontdb: std::sync::Arc::new(fontdb),
        ..usvg::Options::default()
    };

    let tree = usvg::Tree::from_data(svg, &options).ok()?;
    let size = tree.size();
    let scale = (TARGET_WIDTH_PX / size.width()).clamp(0.2, 4.0);
    let width = (size.width() * scale).ceil().max(1.0) as u32;
    let height = (size.height() * scale).ceil().max(1.0) as u32;

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
