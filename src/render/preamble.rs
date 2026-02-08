use std::fmt::Write;

use crate::render::frontmatter::FrontMatter;
use crate::theme::ResolvedTheme;
use crate::RenderOptions;

/// Generate the Typst preamble (set/show rules) from theme + front matter + options.
///
/// Produces `#set` and `#show` rules that configure page layout, typography,
/// headings, code blocks, links, tables, blockquotes, footnotes, and more.
/// The preamble is a standalone Typst fragment prepended to the emitted content.
#[allow(clippy::too_many_lines)]
pub fn generate(
    theme: &ResolvedTheme,
    front_matter: Option<&FrontMatter>,
    options: &RenderOptions,
) -> String {
    let mut out = String::with_capacity(4096);
    let t = &theme.tokens;

    // ─── Document Metadata ───────────────────────────────────────
    emit_document_metadata(&mut out, front_matter);

    // ─── Page Setup ──────────────────────────────────────────────
    emit_page_setup(&mut out, t, options);

    // ─── Syntax Highlighting Theme ───────────────────────────────
    out.push_str("#set raw(theme: \"/__silkprint_theme.tmTheme\")\n\n");

    // ─── Text ────────────────────────────────────────────────────
    emit_text_setup(&mut out, t);

    // ─── Paragraph ───────────────────────────────────────────────
    emit_paragraph_setup(&mut out, t);

    // ─── Headings ────────────────────────────────────────────────
    emit_heading_rules(&mut out, t);

    // ─── Code Blocks ─────────────────────────────────────────────
    emit_code_block_rule(&mut out, t);

    // ─── Inline Code ─────────────────────────────────────────────
    emit_inline_code_rule(&mut out, t);

    // ─── Links ───────────────────────────────────────────────────
    emit_link_rule(&mut out, t);

    // ─── Blockquotes ─────────────────────────────────────────────
    emit_blockquote_rule(&mut out, t);

    // ─── Tables ──────────────────────────────────────────────────
    emit_table_rules(&mut out, t);

    // ─── Footnotes ───────────────────────────────────────────────
    emit_footnote_rule(&mut out, t);

    out
}

// ═══════════════════════════════════════════════════════════════════
// Helper emitters — each writes a focused chunk of the preamble
// ═══════════════════════════════════════════════════════════════════

fn emit_document_metadata(out: &mut String, front_matter: Option<&FrontMatter>) {
    let Some(fm) = front_matter else { return };

    let has_title = fm.title.is_some();
    let has_author = fm.author.is_some();

    if !has_title && !has_author {
        return;
    }

    out.push_str("#set document(\n");
    if let Some(title) = &fm.title {
        let _ = writeln!(out, "  title: \"{}\",", escape_typst_string(title));
    }
    if let Some(author) = &fm.author {
        let _ = writeln!(out, "  author: (\"{}\",),", escape_typst_string(author));
    }
    out.push_str(")\n\n");
}

fn emit_page_setup(
    out: &mut String,
    t: &crate::theme::tokens::ThemeTokens,
    options: &RenderOptions,
) {
    let paper = options.paper.as_typst_str();
    let margin_top = default_if_empty(&t.page.margin_top, "25mm");
    let margin_bottom = default_if_empty(&t.page.margin_bottom, "30mm");
    let margin_left = default_if_empty(&t.page.margin_left, "25mm");
    let margin_right = default_if_empty(&t.page.margin_right, "25mm");

    out.push_str("#set page(\n");
    let _ = writeln!(out, "  paper: \"{paper}\",");
    let _ = writeln!(
        out,
        "  margin: (top: {margin_top}, bottom: {margin_bottom}, left: {margin_left}, right: {margin_right}),"
    );

    if !t.page.background.is_empty() {
        let _ = writeln!(out, "  fill: rgb(\"{}\"),", t.page.background);
    }

    // Page numbering
    if t.page_numbers.enabled {
        let fmt = default_if_empty(&t.page_numbers.format, "1");
        let _ = writeln!(out, "  numbering: \"{fmt}\",");

        let position = &t.page_numbers.position;
        let align = if position.contains("left") {
            "left + bottom"
        } else if position.contains("right") {
            "right + bottom"
        } else {
            "center + bottom"
        };
        let _ = writeln!(out, "  number-align: {align},");
    }

    out.push_str(")\n\n");
}

fn emit_text_setup(out: &mut String, t: &crate::theme::tokens::ThemeTokens) {
    let body_font = default_if_empty(&t.fonts.body, "Source Serif 4");
    let body_size = default_if_empty(&t.font_sizes.body, "11pt");

    out.push_str("#set text(\n");

    // Font fallback chain
    let mut fonts = vec![body_font.to_string()];
    for fb in &t.fonts.body_fallback {
        if !fb.is_empty() {
            fonts.push(fb.clone());
        }
    }
    let font_list: String = fonts
        .iter()
        .map(|f| format!("\"{f}\""))
        .collect::<Vec<_>>()
        .join(", ");
    let _ = writeln!(out, "  font: ({font_list}),");
    let _ = writeln!(out, "  size: {body_size},");

    if !t.text.color.is_empty() {
        let _ = writeln!(out, "  fill: rgb(\"{}\"),", t.text.color);
    }

    out.push_str("  lang: \"en\",\n");
    out.push_str("  hyphenate: true,\n");
    out.push_str("  ligatures: true,\n");
    out.push_str(")\n\n");
}

fn emit_paragraph_setup(out: &mut String, t: &crate::theme::tokens::ThemeTokens) {
    let justify = t.text.justification != "left";
    let line_height = if t.text.line_height > 0.0 {
        t.text.line_height
    } else {
        1.5
    };
    // leading = (line_height - 1.0) * font_size, expressed in em
    let leading = line_height - 1.0;

    out.push_str("#set par(\n");
    let _ = writeln!(out, "  justify: {justify},");
    let _ = writeln!(out, "  leading: {leading:.2}em,");

    if t.text.spacing_mode.as_str() == "indent" {
        let indent = default_if_empty(&t.text.first_line_indent, "1.5em");
        let _ = writeln!(out, "  first-line-indent: {indent},");
    } else {
        // "gap" mode (default)
        let spacing = default_if_empty(&t.text.paragraph_gap, "0.85em");
        let _ = writeln!(out, "  spacing: {spacing},");
    }

    out.push_str(")\n\n");
}

struct HeadingLevel<'a> {
    level: u8,
    size: &'a str,
    tokens: &'a crate::theme::tokens::HeadingLevelTokens,
}

#[allow(clippy::too_many_lines)]
fn emit_heading_rules(out: &mut String, t: &crate::theme::tokens::ThemeTokens) {
    let heading_font = default_if_empty(&t.fonts.heading, "Inter");
    let heading_color = default_if_empty(&t.headings.color, &t.text.color);

    let levels = [
        HeadingLevel {
            level: 1,
            size: default_if_empty(&t.font_sizes.h1, "33.5pt"),
            tokens: &t.headings.h1,
        },
        HeadingLevel {
            level: 2,
            size: default_if_empty(&t.font_sizes.h2, "27pt"),
            tokens: &t.headings.h2,
        },
        HeadingLevel {
            level: 3,
            size: default_if_empty(&t.font_sizes.h3, "21.5pt"),
            tokens: &t.headings.h3,
        },
        HeadingLevel {
            level: 4,
            size: default_if_empty(&t.font_sizes.h4, "17pt"),
            tokens: &t.headings.h4,
        },
        HeadingLevel {
            level: 5,
            size: default_if_empty(&t.font_sizes.h5, "14pt"),
            tokens: &t.headings.h5,
        },
        HeadingLevel {
            level: 6,
            size: default_if_empty(&t.font_sizes.h6, "11pt"),
            tokens: &t.headings.h6,
        },
    ];

    for hl in &levels {
        let weight = if hl.tokens.weight > 0 {
            hl.tokens.weight
        } else {
            match hl.level {
                1 => 700,
                4 | 5 => 500,
                _ => 600,
            }
        };
        let above = default_if_empty(
            &hl.tokens.above,
            match hl.level {
                1 => "36pt",
                2 => "28pt",
                3 => "22pt",
                _ => "18pt",
            },
        );
        let below = default_if_empty(
            &hl.tokens.below,
            match hl.level {
                1 => "12pt",
                2 => "8pt",
                3 => "6pt",
                _ => "4pt",
            },
        );

        let _ = writeln!(
            out,
            "#show heading.where(level: {}): it => {{",
            hl.level
        );

        // Page break before if requested
        if hl.tokens.page_break_before == Some(true) {
            out.push_str("  pagebreak(weak: true)\n");
        }

        let _ = writeln!(out, "  v({above})");
        let _ = writeln!(out, "  block(below: {below})[");

        // Per-level line_height override
        let level_lh = hl.tokens.line_height.unwrap_or(
            if t.headings.line_height > 0.0 {
                t.headings.line_height
            } else {
                match hl.level {
                    1 => 1.1,
                    2 => 1.15,
                    3 | 4 => 1.2,
                    _ => 1.25,
                }
            },
        );
        let heading_leading = level_lh - 1.0;

        // Letter spacing
        let letter_spacing = hl
            .tokens
            .letter_spacing
            .as_deref()
            .filter(|s| !s.is_empty())
            .unwrap_or_else(|| {
                if !t.headings.letter_spacing.is_empty() {
                    &t.headings.letter_spacing
                } else if hl.level == 6 {
                    "0.05em"
                } else {
                    ""
                }
            });

        let mut text_args = format!(
            "font: \"{heading_font}\", size: {}, weight: {weight}, fill: rgb(\"{heading_color}\")",
            hl.size
        );
        if !letter_spacing.is_empty() {
            let _ = write!(text_args, ", tracking: {letter_spacing}");
        }

        let _ = writeln!(out, "    #set text({text_args})");

        if heading_leading.abs() > f64::EPSILON {
            let _ = writeln!(out, "    #set par(leading: {heading_leading:.2}em)");
        }

        // H6 uppercase treatment
        if hl.tokens.uppercase == Some(true) {
            out.push_str("    #upper(it.body)\n");
        } else {
            out.push_str("    #it.body\n");
        }

        out.push_str("  ]\n");

        // Bottom border for headings (e.g., H1)
        if hl.tokens.border == Some(true) {
            let border_color = default_if_empty(&t.horizontal_rule.color, "#e2e2e8");
            let _ = writeln!(
                out,
                "  line(length: 100%, stroke: 0.5pt + rgb(\"{border_color}\"))"
            );
        }

        out.push_str("}\n\n");
    }
}

fn emit_code_block_rule(out: &mut String, t: &crate::theme::tokens::ThemeTokens) {
    let mono_font = default_if_empty(&t.fonts.mono, "JetBrains Mono");
    let code_size = default_if_empty(&t.font_sizes.code, "10pt");
    let bg = default_if_empty(&t.code_block.background, "#f4f4f8");
    let border_color = default_if_empty(&t.code_block.border_color, "#e2e2e8");
    let radius = default_if_empty(&t.code_block.border_radius, "6pt");
    let pad_x = default_if_empty(&t.code_block.padding_horizontal, "14pt");
    let pad_y = default_if_empty(&t.code_block.padding_vertical, "12pt");
    let code_lh = if t.code_block.line_height > 0.0 {
        t.code_block.line_height
    } else {
        1.45
    };
    let code_leading = code_lh - 1.0;

    out.push_str("#show raw.where(block: true): it => {\n");
    out.push_str("  block(\n");
    let _ = writeln!(out, "    fill: rgb(\"{bg}\"),");
    let _ = writeln!(out, "    stroke: 0.5pt + rgb(\"{border_color}\"),");
    let _ = writeln!(out, "    radius: {radius},");
    let _ = writeln!(out, "    inset: (x: {pad_x}, y: {pad_y}),");
    out.push_str("    width: 100%,\n");
    out.push_str("    breakable: true,\n");
    out.push_str("  )[\n");
    let _ = writeln!(
        out,
        "    #set text(font: \"{mono_font}\", size: {code_size}, ligatures: false)"
    );
    let _ = writeln!(
        out,
        "    #set par(justify: false, leading: {code_leading:.2}em)"
    );
    out.push_str("    #it\n");
    out.push_str("  ]\n");
    out.push_str("}\n\n");
}

fn emit_inline_code_rule(out: &mut String, t: &crate::theme::tokens::ThemeTokens) {
    let mono_font = default_if_empty(&t.fonts.mono, "JetBrains Mono");
    let code_size = default_if_empty(&t.font_sizes.code, "10pt");
    let bg = default_if_empty(&t.code_inline.background, "#f4f4f8");
    let border_color = default_if_empty(&t.code_inline.border_color, "#e2e2e8");
    let radius = default_if_empty(&t.code_inline.border_radius, "3pt");

    out.push_str("#show raw.where(block: false): it => {\n");
    out.push_str("  box(\n");
    let _ = writeln!(out, "    fill: rgb(\"{bg}\"),");
    let _ = writeln!(out, "    stroke: 0.5pt + rgb(\"{border_color}\"),");
    let _ = writeln!(out, "    radius: {radius},");
    out.push_str("    inset: (x: 3pt, y: 1.5pt),\n");
    out.push_str("  )[\n");
    let _ = writeln!(
        out,
        "    #set text(font: \"{mono_font}\", size: {code_size}, ligatures: false)"
    );
    out.push_str("    #it\n");
    out.push_str("  ]\n");
    out.push_str("}\n\n");
}

fn emit_link_rule(out: &mut String, t: &crate::theme::tokens::ThemeTokens) {
    let color = default_if_empty(&t.links.color, "#4a5dbd");

    out.push_str("#show link: it => {\n");
    let _ = writeln!(out, "  set text(fill: rgb(\"{color}\"), ligatures: false)");
    if t.links.underline {
        out.push_str("  underline(it)\n");
    } else {
        out.push_str("  it\n");
    }
    out.push_str("}\n\n");
}

fn emit_blockquote_rule(out: &mut String, t: &crate::theme::tokens::ThemeTokens) {
    let border_color = default_if_empty(&t.blockquote.border_color, "#4a5dbd");
    let border_width = default_if_empty(&t.blockquote.border_width, "2.5pt");
    let left_pad = default_if_empty(&t.blockquote.left_padding, "14pt");
    let text_color = default_if_empty(&t.blockquote.text_color, "#555570");

    out.push_str("#show quote.where(block: true): it => {\n");
    out.push_str("  block(\n");
    let _ = writeln!(
        out,
        "    stroke: (left: {border_width} + rgb(\"{border_color}\")),"
    );
    let _ = writeln!(
        out,
        "    inset: (left: {left_pad}, y: 8pt, right: 8pt),"
    );
    out.push_str("    width: 100%,\n");
    out.push_str("  )[\n");
    let _ = writeln!(out, "    #set text(fill: rgb(\"{text_color}\"))");
    if t.blockquote.italic {
        out.push_str("    #emph(it.body)\n");
    } else {
        out.push_str("    #it.body\n");
    }
    out.push_str("  ]\n");
    out.push_str("}\n\n");
}

fn emit_table_rules(out: &mut String, t: &crate::theme::tokens::ThemeTokens) {
    let cell_padding = default_if_empty(&t.table.cell_padding, "10pt");

    // Parse "Ypt Xpt" format → (x: X, y: Y), or single value → (x: val, y: val)
    let parts: Vec<&str> = cell_padding.split_whitespace().collect();
    let (x_pad, y_pad) = match parts.as_slice() {
        [y, x] => (*x, *y),
        [val] => (*val, *val),
        _ => ("10pt", "6pt"),
    };

    out.push_str("#set table(\n");
    out.push_str("  stroke: none,\n");
    let _ = writeln!(out, "  inset: (x: {x_pad}, y: {y_pad}),");
    out.push_str(")\n");

    // Header cell styling
    let header_weight = if t.table.header_weight > 0 {
        t.table.header_weight
    } else {
        600
    };
    let header_font_raw = if !t.table.header_font.is_empty() {
        &t.table.header_font
    } else if !t.fonts.heading.is_empty() {
        &t.fonts.heading
    } else {
        "Inter"
    };
    // Resolve semantic font names: "heading" → fonts.heading, "body" → fonts.body
    let header_font = resolve_font_name(header_font_raw, t);
    let header_color = default_if_empty(&t.headings.color, &t.text.color);
    let _ = writeln!(
        out,
        "#show table.cell.where(y: 0): set text(font: \"{header_font}\", weight: {header_weight}, fill: rgb(\"{header_color}\"))"
    );

    let header_bg = default_if_empty(&t.table.header_background, "#f4f4f8");
    let header_border_color = default_if_empty(&t.table.header_border_color, "#c8c8d4");
    let header_border_width = default_if_empty(&t.table.header_border_width, "1.5pt");
    let _ = writeln!(
        out,
        "#show table.cell.where(y: 0): set table.cell(fill: rgb(\"{header_bg}\"), stroke: (bottom: {header_border_width} + rgb(\"{header_border_color}\")))"
    );

    out.push('\n');
}

fn emit_footnote_rule(out: &mut String, t: &crate::theme::tokens::ThemeTokens) {
    let sep_color = default_if_empty(&t.footnotes.separator_color, "#e2e2e8");
    let sep_width = default_if_empty(&t.footnotes.separator_width, "33%");
    let text_size_raw = default_if_empty(&t.footnotes.text_size, "9pt");
    // Convert CSS-like size names to Typst sizes
    let text_size = match text_size_raw {
        "small" | "smaller" => "9pt",
        "x-small" => "8pt",
        "large" | "larger" => "13pt",
        other => other,
    };
    let num_color = default_if_empty(&t.footnotes.number_color, "#4a5dbd");

    out.push_str("#show footnote.entry: it => {\n");
    let _ = writeln!(
        out,
        "  line(length: {sep_width}, stroke: 0.5pt + rgb(\"{sep_color}\"))"
    );
    out.push_str("  v(4pt)\n");
    let _ = writeln!(out, "  set text(size: {text_size})");
    let _ = writeln!(
        out,
        "  [#text(fill: rgb(\"{num_color}\"))[#it.note.counter.display()] #it.note.body]"
    );
    out.push_str("}\n");
}

// ═══════════════════════════════════════════════════════════════════
// Utilities
// ═══════════════════════════════════════════════════════════════════

/// Return `value` if non-empty, otherwise `fallback`.
fn default_if_empty<'a>(value: &'a str, fallback: &'a str) -> &'a str {
    if value.is_empty() { fallback } else { value }
}

/// Resolve semantic font names like "heading", "body", "mono" to actual font names.
fn resolve_font_name<'a>(name: &'a str, t: &'a crate::theme::tokens::ThemeTokens) -> &'a str {
    match name {
        "heading" if !t.fonts.heading.is_empty() => &t.fonts.heading,
        "body" if !t.fonts.body.is_empty() => &t.fonts.body,
        "mono" if !t.fonts.mono.is_empty() => &t.fonts.mono,
        other => other,
    }
}

/// Escape characters that are special in Typst string literals (inside `"`).
fn escape_typst_string(s: &str) -> String {
    s.replace('\\', "\\\\").replace('"', "\\\"")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::theme::ResolvedTheme;
    use crate::theme::tokens::ThemeTokens;

    fn test_theme() -> ResolvedTheme {
        ResolvedTheme {
            tokens: ThemeTokens::default(),
            tmtheme_xml: String::new(),
        }
    }

    #[test]
    fn generates_nonempty_preamble() {
        let theme = test_theme();
        let options = RenderOptions::default();
        let preamble = generate(&theme, None, &options);
        assert!(!preamble.is_empty());
        assert!(preamble.contains("#set page("));
        assert!(preamble.contains("#set text("));
        assert!(preamble.contains("#set par("));
        assert!(preamble.contains("#set raw(theme:"));
    }

    #[test]
    fn includes_document_metadata_from_front_matter() {
        let theme = test_theme();
        let options = RenderOptions::default();
        let fm = FrontMatter {
            title: Some("Test Doc".to_string()),
            author: Some("Nova".to_string()),
            ..Default::default()
        };
        let preamble = generate(&theme, Some(&fm), &options);
        assert!(preamble.contains("#set document("));
        assert!(preamble.contains("\"Test Doc\""));
        assert!(preamble.contains("\"Nova\""));
    }

    #[test]
    fn omits_document_metadata_without_front_matter() {
        let theme = test_theme();
        let options = RenderOptions::default();
        let preamble = generate(&theme, None, &options);
        assert!(!preamble.contains("#set document("));
    }

    #[test]
    fn includes_heading_show_rules() {
        let theme = test_theme();
        let options = RenderOptions::default();
        let preamble = generate(&theme, None, &options);
        assert!(preamble.contains("heading.where(level: 1)"));
        assert!(preamble.contains("heading.where(level: 6)"));
    }
}
