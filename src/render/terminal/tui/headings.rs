//! Rasterize large headings (H1/H2) to images using a bundled font, so the
//! reader can show real typographic hierarchy on graphics-capable terminals
//! instead of same-height bold text.

use ab_glyph::{Font, FontVec, PxScale, ScaleFont, point};
use image::{DynamicImage, Rgba, RgbaImage};

use super::super::model::Rgb;

const PAD: f32 = 6.0;

/// Rasterize `text` at a size scaled to the heading `level`, with `fg` glyphs
/// over a `bg` fill. Returns `None` if the font is unavailable or text is empty.
// Pixel layout is inherently cast-heavy (f32 metrics ↔ integer coordinates).
#[allow(
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    clippy::cast_precision_loss,
    clippy::cast_possible_wrap,
    clippy::as_conversions
)]
pub fn rasterize(text: &str, level: u8, fg: Rgb, bg: Rgb) -> Option<DynamicImage> {
    let trimmed = text.trim();
    if trimmed.is_empty() {
        return None;
    }

    let bytes = crate::fonts::font_by_path("source-serif/SourceSerif4-Bold.ttf")?;
    let font = FontVec::try_from_vec(bytes).ok()?;
    let px: f32 = match level {
        1 => 58.0,
        2 => 42.0,
        _ => 32.0,
    };
    let scale = PxScale::from(px);
    let scaled = font.as_scaled(scale);
    let ascent = scaled.ascent();
    let descent = scaled.descent();

    let baseline = ascent + PAD;
    let mut caret = PAD;
    let mut outlines = Vec::new();
    for ch in trimmed.chars() {
        let gid = font.glyph_id(ch);
        let glyph = gid.with_scale_and_position(scale, point(caret, baseline));
        caret += scaled.h_advance(gid);
        if let Some(outline) = font.outline_glyph(glyph) {
            outlines.push(outline);
        }
    }

    let width = (caret + PAD).ceil().max(1.0) as u32;
    let height = (ascent - descent + 2.0 * PAD).ceil().max(1.0) as u32;
    let mut img = RgbaImage::from_pixel(width, height, Rgba([bg.0, bg.1, bg.2, 255]));

    for outline in outlines {
        let bounds = outline.px_bounds();
        let ox = bounds.min.x;
        let oy = bounds.min.y;
        outline.draw(|gx, gy, coverage| {
            let x = ox as i32 + gx as i32;
            let y = oy as i32 + gy as i32;
            if x < 0 || y < 0 {
                return;
            }
            let (x, y) = (x as u32, y as u32);
            if x >= width || y >= height {
                return;
            }
            img.put_pixel(
                x,
                y,
                Rgba([
                    blend(bg.0, fg.0, coverage),
                    blend(bg.1, fg.1, coverage),
                    blend(bg.2, fg.2, coverage),
                    255,
                ]),
            );
        });
    }

    Some(DynamicImage::ImageRgba8(img))
}

#[allow(
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    clippy::as_conversions
)]
fn blend(bg: u8, fg: u8, coverage: f32) -> u8 {
    let coverage = coverage.clamp(0.0, 1.0);
    (f32::from(bg) * (1.0 - coverage) + f32::from(fg) * coverage).round() as u8
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rasterizes_nonempty_heading() {
        let img = rasterize("Hello", 1, Rgb(255, 255, 255), Rgb(0, 0, 0));
        let img = img.expect("bundled Source Serif font should rasterize");
        assert!(img.width() > 10 && img.height() > 10);
    }

    #[test]
    fn empty_heading_is_none() {
        assert!(rasterize("   ", 1, Rgb(0, 0, 0), Rgb(255, 255, 255)).is_none());
    }
}
