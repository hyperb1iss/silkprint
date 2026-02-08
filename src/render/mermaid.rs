//! Native mermaid diagram rendering via `mermaid-rs-renderer`.
//!
//! Detects `mermaid` code blocks during Typst emission and renders them to SVG
//! using a pure-Rust mermaid renderer. The SVGs are served as virtual files
//! through the Typst `World::file()` implementation.

use std::collections::HashMap;

use crate::theme::ResolvedTheme;
use crate::warnings::{SilkprintWarning, WarningCollector};

/// Virtual path prefix for mermaid SVGs served through the Typst World.
pub const MERMAID_VPATH_PREFIX: &str = "/__mermaid_";

/// Render collected mermaid diagram sources to SVG data.
///
/// Returns a map from virtual path (e.g., `/__mermaid_0.svg`) to SVG bytes.
/// Failed renders produce a placeholder SVG and emit a warning.
pub fn render_all(
    sources: &[String],
    theme: &ResolvedTheme,
    warnings: &mut WarningCollector,
) -> HashMap<String, Vec<u8>> {
    let options = build_render_options(theme);
    let mut results = HashMap::new();

    for (idx, source) in sources.iter().enumerate() {
        let vpath = format!("{MERMAID_VPATH_PREFIX}{idx}.svg");
        match mermaid_rs_renderer::render_with_options(source, options.clone()) {
            Ok(svg) => {
                let svg = sanitize_svg_fonts(&svg);
                tracing::debug!(index = idx, bytes = svg.len(), "rendered mermaid diagram");
                results.insert(vpath, svg.into_bytes());
            }
            Err(err) => {
                let msg = format!("{err}");
                tracing::warn!(index = idx, error = %msg, "mermaid render failed");
                warnings.push(SilkprintWarning::MermaidRenderFailed {
                    index: idx,
                    message: msg,
                });
                results.insert(vpath, placeholder_svg(idx));
            }
        }
    }

    results
}

/// Build mermaid `RenderOptions` from the `SilkPrint` theme.
///
/// Maps theme colors to mermaid's node fill, stroke, text, and line colors
/// so diagrams match the document's visual identity.
fn build_render_options(theme: &ResolvedTheme) -> mermaid_rs_renderer::RenderOptions {
    let t = &theme.tokens;

    let mut opts = mermaid_rs_renderer::RenderOptions::modern();

    // Map SilkPrint colors → mermaid theme fields
    let bg = if t.page.background.is_empty() {
        "#ffffff"
    } else {
        &t.page.background
    };
    let node_fill = if t.code_block.background.is_empty() {
        "#f4f4f8"
    } else {
        &t.code_block.background
    };
    let border = if t.code_block.border_color.is_empty() {
        "#c8c8d4"
    } else {
        &t.code_block.border_color
    };
    let text_color = if t.text.color.is_empty() {
        "#1a1a2e"
    } else {
        &t.text.color
    };
    let accent = if t.headings.color.is_empty() {
        &t.text.color
    } else {
        &t.headings.color
    };
    let line_color = if t.table.header_border_color.is_empty() {
        border
    } else {
        &t.table.header_border_color
    };

    // Font
    let font = if t.fonts.body.is_empty() {
        "Inter, ui-sans-serif, system-ui, sans-serif"
    } else {
        &t.fonts.body
    };

    opts.theme.background = bg.to_string();
    opts.theme.primary_color = node_fill.to_string();
    opts.theme.primary_text_color = text_color.to_string();
    opts.theme.primary_border_color = border.to_string();
    opts.theme.line_color = line_color.to_string();
    opts.theme.text_color = text_color.to_string();
    opts.theme.secondary_color = bg.to_string();
    opts.theme.tertiary_color = node_fill.to_string();
    opts.theme.edge_label_background = bg.to_string();
    opts.theme.cluster_background = node_fill.to_string();
    opts.theme.cluster_border = border.to_string();
    opts.theme.font_family = font.to_string();
    opts.theme.font_size = 13.0;

    // Sequence diagram colors
    opts.theme.sequence_actor_fill = node_fill.to_string();
    opts.theme.sequence_actor_border = border.to_string();
    opts.theme.sequence_note_fill = node_fill.to_string();
    opts.theme.sequence_note_border = border.to_string();
    opts.theme.sequence_activation_fill = node_fill.to_string();
    opts.theme.sequence_activation_border.clone_from(accent);

    tracing::debug!(
        background = bg,
        node_fill,
        text = text_color,
        accent,
        "built mermaid theme from SilkPrint tokens"
    );

    opts
}

/// Fix font-family attributes that contain unescaped inner quotes.
///
/// The mermaid renderer produces font-family values like:
///   `font-family="Inter, ..., "Segoe UI", sans-serif"`
/// where the inner `"Segoe UI"` breaks XML attribute parsing.
/// We replace inner double quotes with single quotes.
fn sanitize_svg_fonts(svg: &str) -> String {
    let mut result = String::with_capacity(svg.len());
    let mut rest = svg;

    while let Some(start) = rest.find("font-family=\"") {
        // Copy everything before this attribute
        result.push_str(&rest[..start]);
        rest = &rest[start..];

        // Find the font-family="..." span
        let attr_start = "font-family=\"".len();
        if let Some(end) = find_attr_end(&rest[attr_start..]) {
            let attr_value = &rest[attr_start..attr_start + end];
            // Replace inner double quotes with single quotes
            let fixed = attr_value.replace('"', "'");
            result.push_str("font-family=\"");
            result.push_str(&fixed);
            result.push('"');
            rest = &rest[attr_start + end + 1..]; // skip past closing "
        } else {
            // Can't find end — just copy as-is and move on
            result.push_str(&rest[..attr_start]);
            rest = &rest[attr_start..];
        }
    }
    result.push_str(rest);
    result
}

/// Find the closing `"` of an XML attribute value, accounting for the fact
/// that mermaid SVGs may contain inner quotes for font names.
///
/// Heuristic: the closing quote is the last `"` before a `>` or ` ` that follows.
fn find_attr_end(s: &str) -> Option<usize> {
    // Find the next `"` that is followed by `>`, ` `, `/`, or is at end
    // We need to find where the attribute value *actually* ends.
    // Strategy: find all `"` positions, the correct one is followed by
    // a space, `/`, or `>`.
    let bytes = s.as_bytes();
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'"' {
            // Check what follows the quote
            let next = bytes.get(i + 1).copied().unwrap_or(b'>');
            if next == b' ' || next == b'>' || next == b'/' || next == b'\n' {
                return Some(i);
            }
        }
        i += 1;
    }
    None
}

/// Generate a placeholder SVG for a failed mermaid render.
fn placeholder_svg(index: usize) -> Vec<u8> {
    let mut svg = String::from(
        "<svg xmlns=\"http://www.w3.org/2000/svg\" viewBox=\"0 0 400 60\">\n\
         <rect width=\"400\" height=\"60\" fill=\"#fff3cd\" stroke=\"#ffc107\" rx=\"4\"/>\n\
         <text x=\"20\" y=\"35\" font-family=\"sans-serif\" font-size=\"13\" fill=\"#856404\">",
    );
    {
        use std::fmt::Write;
        let _ = write!(svg, "Mermaid diagram {index} failed to render");
    }
    svg.push_str("</text>\n</svg>");
    svg.into_bytes()
}
