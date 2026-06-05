use std::fmt::Write;

use scraper::node::Element;
use scraper::{Html, Selector};

use crate::render::escape::{escape_typst_content, escape_typst_string};
use crate::render::image::{PreparedImage, PreparedImages, is_remote_image};
use crate::warnings::{SilkprintWarning, WarningCollector};

use super::Context;

pub(crate) fn collect_sources(html: &str) -> Vec<String> {
    let document = Html::parse_fragment(html);
    let selector = Selector::parse("img").expect("valid img selector");

    document
        .select(&selector)
        .filter_map(|element| element.value().attr("src"))
        .map(str::to_string)
        .collect()
}

pub(super) fn emit(
    el: &Element,
    out: &mut String,
    images: &PreparedImages,
    warnings: &mut WarningCollector,
    ctx: Context,
) {
    let src = el.attr("src").unwrap_or("");
    let alt = el.attr("alt").unwrap_or("");
    let title = el.attr("title").unwrap_or("");
    let label = if !title.is_empty() {
        title
    } else if !alt.is_empty() {
        alt
    } else {
        src
    };

    let typst_path = match images.resolve(src) {
        Some(PreparedImage::Available { typst_path }) => Some(typst_path.as_str()),
        None if !is_remote_image(src) => Some(src),
        _ => None,
    };

    if let Some(typst_path) = typst_path {
        let width_arg = parse_width(el)
            .map(|width| format!(", width: {width}"))
            .unwrap_or_default();
        let escaped_src = escape_typst_string(typst_path);
        let escaped_label = escape_typst_content(label);

        if matches!(ctx, Context::Inline | Context::TableCell) {
            let _ = write!(out, "#box(image(\"{escaped_src}\"{width_arg}))");
        } else if escaped_label.is_empty() {
            let _ = write!(out, "#figure(image(\"{escaped_src}\"{width_arg}))");
        } else {
            let _ = write!(
                out,
                "#figure(image(\"{escaped_src}\"{width_arg}), caption: [{escaped_label}])"
            );
        }
    } else {
        emit_placeholder(out, label, ctx);

        if images.resolve(src).is_none() && is_remote_image(src) {
            warnings.push(SilkprintWarning::RemoteImageSkipped {
                url: src.to_string(),
            });
        }
    }
}

fn emit_placeholder(out: &mut String, label: &str, ctx: Context) {
    let escaped = escape_typst_content(label);

    if ctx == Context::Block {
        let _ = write!(
            out,
            "#block(width: 80%, inset: 12pt, stroke: 0.5pt + luma(180), radius: 4pt)[#align(center)[#text(size: 0.85em, fill: luma(120))[\\[image: {escaped}\\]]]]"
        );
    } else {
        out.push_str(&escaped);
    }
}

const MAX_IMAGE_PT: f64 = 454.0;

fn parse_width(el: &Element) -> Option<String> {
    let raw = el.attr("width")?;
    let trimmed = raw.trim();

    if trimmed.ends_with('%') {
        return Some(trimmed.to_string());
    }

    let numeric = trimmed.strip_suffix("px").unwrap_or(trimmed);

    if numeric.chars().all(|c| c.is_ascii_digit() || c == '.') {
        if let Ok(val) = numeric.parse::<f64>()
            && val > MAX_IMAGE_PT
        {
            return Some("80%".to_string());
        }
        Some(format!("{numeric}pt"))
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::collect_sources;

    #[test]
    fn collects_html_image_sources_from_nested_markup() {
        let html = r#"<div><img src="one.png"><p><img src="two.svg"></p></div>"#;
        let sources = collect_sources(html);
        assert_eq!(sources, vec!["one.png", "two.svg"]);
    }
}
