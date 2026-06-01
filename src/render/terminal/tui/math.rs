//! Rasterize display math for the TUI through Typst.

use std::collections::HashMap;
use std::path::{Path, PathBuf};

use image::DynamicImage;

use crate::RenderOptions;
use crate::theme::ResolvedTheme;

use super::super::model::Rgb;
use super::diagrams::{MATH_RASTER_LIMITS, raster_svg_to_image};

const MAX_MATH_BYTES: usize = 16 * 1024;

pub fn math_image(
    source: &str,
    theme: &ResolvedTheme,
    font_dirs: &[PathBuf],
    bg: Rgb,
) -> Option<DynamicImage> {
    let svg = math_svg(source, theme, font_dirs, bg)?;
    raster_svg_to_image(svg.as_bytes(), bg, MATH_RASTER_LIMITS)
}

fn math_svg(source: &str, theme: &ResolvedTheme, font_dirs: &[PathBuf], bg: Rgb) -> Option<String> {
    if source.len() > MAX_MATH_BYTES || source.contains('$') {
        return None;
    }
    let typst_source = math_typst_source(source, theme, font_dirs, bg);
    compile_svg(&typst_source, theme, font_dirs)
}

fn compile_svg(typst_source: &str, theme: &ResolvedTheme, font_dirs: &[PathBuf]) -> Option<String> {
    let empty = HashMap::new();
    let world = crate::render::typst::build_world(
        typst_source,
        theme,
        Path::new("."),
        font_dirs,
        &empty,
        &empty,
    )
    .ok()?;
    let document = crate::render::typst::compile_paged(&world).ok()?;
    let page = document.pages.first()?;
    Some(typst_svg::svg(page))
}

fn math_typst_source(
    source: &str,
    theme: &ResolvedTheme,
    font_dirs: &[PathBuf],
    bg: Rgb,
) -> String {
    let options = RenderOptions {
        font_dirs: font_dirs.to_vec(),
        ..RenderOptions::default()
    };
    let bg = rgb_hex(bg);
    let preamble = crate::render::preamble::generate_math_snippet(theme, &options, &bg);
    let source = source.trim();
    format!("{preamble}#pad(x: 0.5em, y: 0.25em)[$ {source} $]\n")
}

fn rgb_hex(rgb: Rgb) -> String {
    format!("#{:02x}{:02x}{:02x}", rgb.0, rgb.1, rgb.2)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::PaperSize;
    use crate::ThemeSource;
    use crate::render::preamble;
    use crate::theme;
    use crate::warnings::WarningCollector;
    use image::GenericImageView;
    use std::fmt::Write as _;

    fn theme() -> ResolvedTheme {
        let mut warnings = WarningCollector::new();
        theme::load_theme(
            &ThemeSource::BuiltIn("silk-light".to_string()),
            &mut warnings,
        )
        .expect("theme")
    }

    #[test]
    fn renders_display_math_to_image() {
        let image = math_image("E = m c^2", &theme(), &[], Rgb(255, 255, 255)).expect("image");
        assert!(image.width() > 0);
        assert!(image.height() > 0);
    }

    #[test]
    fn math_raster_uses_natural_size_not_diagram_target_width() {
        let image = math_image("E = m c^2", &theme(), &[], Rgb(255, 255, 255)).expect("image");
        assert!(
            image.width() < 1000,
            "simple math should keep natural width, got {}px",
            image.width()
        );
    }

    #[test]
    fn pdf_and_terminal_math_contract_rasters_match() {
        let theme = theme();
        let bg = Rgb(255, 255, 255);
        let source = "sum_(i=1)^n i = (n(n+1))/2";
        let terminal = math_image(source, &theme, &[], bg).expect("terminal math");
        let pdf_contract = pdf_contract_image(source, &theme, &[], bg).expect("pdf contract math");

        assert_eq!(terminal.dimensions(), pdf_contract.dimensions());
        assert!(
            images_close(&terminal, &pdf_contract, 2),
            "terminal math raster should match PDF preamble contract"
        );
    }

    #[test]
    fn rejects_oversized_or_dollar_containing_math() {
        assert!(math_image("$bad$", &theme(), &[], Rgb(255, 255, 255)).is_none());
        assert!(
            math_image(
                &"x".repeat(MAX_MATH_BYTES + 1),
                &theme(),
                &[],
                Rgb(255, 255, 255),
            )
            .is_none()
        );
    }

    fn pdf_contract_image(
        source: &str,
        theme: &ResolvedTheme,
        font_dirs: &[PathBuf],
        bg: Rgb,
    ) -> Option<DynamicImage> {
        let typst_source = pdf_contract_source(source, theme, font_dirs, bg);
        let svg = compile_svg(&typst_source, theme, font_dirs)?;
        raster_svg_to_image(svg.as_bytes(), bg, MATH_RASTER_LIMITS)
    }

    fn pdf_contract_source(
        source: &str,
        theme: &ResolvedTheme,
        font_dirs: &[PathBuf],
        bg: Rgb,
    ) -> String {
        let options = RenderOptions {
            paper: PaperSize::A4,
            font_dirs: font_dirs.to_vec(),
            ..RenderOptions::default()
        };
        let mut typst = preamble::generate(theme, None, &options);
        let bg = rgb_hex(bg);
        typst.push_str("#set page(\n");
        typst.push_str("  width: auto,\n");
        typst.push_str("  height: auto,\n");
        typst.push_str("  margin: 0pt,\n");
        let _ = writeln!(typst, "  fill: rgb(\"{bg}\"),");
        typst.push_str(")\n\n");
        typst.push_str("#set par(justify: false, leading: 0em, spacing: 0pt)\n\n");
        let _ = writeln!(typst, "#pad(x: 0.5em, y: 0.25em)[$ {} $]", source.trim());
        typst
    }

    fn images_close(left: &DynamicImage, right: &DynamicImage, tolerance: u8) -> bool {
        if left.dimensions() != right.dimensions() {
            return false;
        }
        left.to_rgba8()
            .pixels()
            .zip(right.to_rgba8().pixels())
            .all(|(a, b)| {
                a.0.iter()
                    .zip(b.0.iter())
                    .all(|(a, b)| a.abs_diff(*b) <= tolerance)
            })
    }
}
