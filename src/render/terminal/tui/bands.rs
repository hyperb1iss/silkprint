use std::hash::{Hash, Hasher};
use std::path::PathBuf;

use ratatui::style::Color;

use crate::render::terminal::model::{Block, RenderedDoc, Rgb};
use crate::theme::ResolvedTheme;

use super::images::Placement;
use super::text::code_lines_source;
use crate::render::terminal::style::ContentStyleResolver;

pub(super) enum BandSpec {
    Image(String),
    Mermaid { source: String, bg: Rgb },
    Math { source: String, bg: Rgb },
}

pub(super) fn band_specs(doc: &RenderedDoc, theme: &ResolvedTheme) -> Vec<(usize, BandSpec)> {
    let resolver = ContentStyleResolver::new(theme);
    let bg = resolver.page_background().unwrap_or(Rgb(0, 0, 0));
    doc.blocks
        .iter()
        .enumerate()
        .filter_map(|(i, block)| match block {
            Block::Image { src, .. } => Some((i, BandSpec::Image(src.clone()))),
            Block::CodeBlock {
                lang: Some(lang),
                lines,
            } if lang == "mermaid" => Some((
                i,
                BandSpec::Mermaid {
                    source: code_lines_source(lines),
                    bg,
                },
            )),
            Block::Math {
                source,
                display: true,
            } => Some((
                i,
                BandSpec::Math {
                    source: source.clone(),
                    bg,
                },
            )),
            _ => None,
        })
        .collect()
}

pub(super) fn rgb_to_color(rgb: Rgb) -> Color {
    Color::Rgb(rgb.0, rgb.1, rgb.2)
}

pub(super) fn generated_key(kind: &str, source: &str, theme: u64, bg: Rgb, fonts: u64) -> String {
    let source = hash_value(source);
    format!(
        "\u{0}{kind}:{source:016x}:{theme:016x}:{fonts:016x}:{:02x}{:02x}{:02x}",
        bg.0, bg.1, bg.2
    )
}

pub(super) fn theme_fingerprint(theme: &ResolvedTheme) -> u64 {
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    format!("{:?}", theme.tokens).hash(&mut hasher);
    theme.tmtheme_xml.hash(&mut hasher);
    hasher.finish()
}

pub(super) fn font_dirs_fingerprint(font_dirs: &[PathBuf]) -> u64 {
    hash_value(font_dirs)
}

fn hash_value<T: Hash + ?Sized>(value: &T) -> u64 {
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    value.hash(&mut hasher);
    hasher.finish()
}

pub(super) fn visible_band_rows(
    placement: &Placement,
    scroll: u32,
    viewport: u32,
) -> Option<(u32, u32)> {
    let band_top = u32::from(placement.line);
    let band_bottom = band_top + u32::from(placement.rows);
    let vis_top = band_top.max(scroll);
    let vis_bottom = band_bottom.min(scroll + viewport);
    (vis_top < vis_bottom).then_some((vis_top, vis_bottom))
}
