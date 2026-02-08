//! HTML-to-Typst converter for embedded HTML blocks and inline HTML.
//!
//! `SilkPrint`'s markdown pipeline encounters raw HTML in two forms:
//! - **Block HTML**: `<table>`, `<div align="center">`, etc.
//! - **Inline HTML**: `<strong>`, `<a>`, `<br>`, etc.
//!
//! This module parses HTML via `scraper` and emits equivalent Typst markup,
//! pushing warnings for unsupported tags and remote images.

use std::fmt::Write;

use ego_tree::NodeRef;
use scraper::node::{Element, Node};
use scraper::Html;

use crate::warnings::{SilkprintWarning, WarningCollector};

use super::escape::{escape_typst_content, escape_typst_string};

// ─── Public API ──────────────────────────────────────────────────────

/// Convert a block-level HTML string into Typst markup.
///
/// Parses the HTML as a full document and walks the DOM tree, emitting
/// Typst equivalents for supported elements.
pub fn emit_html_block(html: &str, warnings: &mut WarningCollector) -> String {
    let doc = Html::parse_document(html);
    let mut out = String::new();

    for child in doc.tree.root().children() {
        emit_dom_node(child, &mut out, warnings, Context::Block);
    }

    out
}

/// Convert an inline HTML fragment into Typst markup.
///
/// Parses the HTML as a fragment and emits only inline-level Typst.
pub fn emit_html_inline(html: &str, warnings: &mut WarningCollector) -> String {
    let doc = Html::parse_fragment(html);
    let mut out = String::new();

    for child in doc.tree.root().children() {
        emit_dom_node(child, &mut out, warnings, Context::Inline);
    }

    out
}

// ─── Internal Types ──────────────────────────────────────────────────

/// Whether we're emitting block-level, inline-level, or table-cell content.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Context {
    Block,
    Inline,
    /// Inside a table cell — images use bare `image()` instead of `#figure()`.
    TableCell,
}

// ─── DOM Walker ──────────────────────────────────────────────────────

/// Recursively emit a single DOM node and its children.
fn emit_dom_node(
    node: NodeRef<'_, Node>,
    out: &mut String,
    warnings: &mut WarningCollector,
    ctx: Context,
) {
    match node.value() {
        Node::Text(text) => {
            out.push_str(&escape_typst_content(text));
        }

        Node::Element(el) => {
            let tag = el.name();
            emit_element(tag, node, el, out, warnings, ctx);
        }

        // Document/Fragment roots, doctype, comments — recurse through children
        _ => {
            for child in node.children() {
                emit_dom_node(child, out, warnings, ctx);
            }
        }
    }
}

/// Dispatch an element node to the appropriate handler by tag name.
#[allow(clippy::too_many_lines)]
fn emit_element(
    tag: &str,
    node: NodeRef<'_, Node>,
    el: &Element,
    out: &mut String,
    warnings: &mut WarningCollector,
    ctx: Context,
) {
    match tag {
        // ─── Headings ────────────────────────────────────────────
        "h1" | "h2" | "h3" | "h4" | "h5" | "h6" => {
            if ctx != Context::Block {
                return;
            }
            emit_heading(tag, node, el, out, warnings);
        }

        // ─── Block containers ────────────────────────────────────
        "p" | "div" => {
            if ctx != Context::Block {
                // In inline/table-cell context, just emit children directly
                emit_children(node, out, warnings, ctx);
                return;
            }
            emit_aligned_block(node, el, out, warnings);
        }

        // ─── Table ───────────────────────────────────────────────
        "table" => {
            if ctx != Context::Block {
                return;
            }
            emit_table(node, out, warnings);
        }

        // ─── Images ──────────────────────────────────────────────
        "img" => emit_image(el, out, warnings, ctx),

        // ─── Links ───────────────────────────────────────────────
        "a" => emit_link(node, el, out, warnings, ctx),

        // ─── Inline formatting ───────────────────────────────────
        "strong" | "b" => {
            out.push('*');
            emit_children(node, out, warnings, ctx);
            out.push('*');
        }

        "em" | "i" => {
            out.push('_');
            emit_children(node, out, warnings, ctx);
            out.push('_');
        }

        "code" => {
            out.push('`');
            // Code content: collect raw text, no escaping inside backticks
            collect_text(node, out);
            out.push('`');
        }

        "sub" => {
            out.push_str("#sub[");
            emit_children(node, out, warnings, ctx);
            out.push(']');
        }

        "sup" => {
            out.push_str("#super[");
            emit_children(node, out, warnings, ctx);
            out.push(']');
        }

        // ─── Line break ─────────────────────────────────────────
        "br" => {
            out.push('\\');
            out.push('\n');
        }

        // ─── Horizontal rule ────────────────────────────────────
        "hr" => {
            if ctx == Context::Block {
                out.push_str("#line(length: 100%)\n");
            }
        }

        // ─── Lists ──────────────────────────────────────────────
        "ul" => {
            if ctx != Context::Block {
                return;
            }
            emit_list(node, out, warnings, false);
        }

        "ol" => {
            if ctx != Context::Block {
                return;
            }
            emit_list(node, out, warnings, true);
        }

        // ─── Transparent wrappers ────────────────────────────────
        // Table section wrappers, stray row/cell elements, list items
        // outside list context, spans, and structural html/head/body
        // all just pass through to their children.
        "thead" | "tbody" | "tfoot" | "tr" | "td" | "th" | "li" | "span" | "html" | "head"
        | "body" => {
            emit_children(node, out, warnings, ctx);
        }

        // ─── Unknown tags ────────────────────────────────────────
        _ => {
            warnings.push(SilkprintWarning::UnsupportedHtmlTag {
                tag: tag.to_string(),
            });
            emit_children(node, out, warnings, ctx);
        }
    }
}

// ─── Element Handlers ────────────────────────────────────────────────

/// Emit a heading element (h1-h6), optionally wrapped in `#align(...)`.
fn emit_heading(
    tag: &str,
    node: NodeRef<'_, Node>,
    el: &Element,
    out: &mut String,
    warnings: &mut WarningCollector,
) {
    let level = heading_level(tag);
    let prefix = "=".repeat(level);

    let mut content = String::new();
    emit_children(node, &mut content, warnings, Context::Inline);

    // Strip line breaks (`\` + newline) and collapse whitespace in heading text.
    // HTML headings like `<h1><br>Title<br></h1>` shouldn't produce Typst breaks.
    let content = clean_heading_content(&content);

    if content.is_empty() {
        return;
    }

    if let Some(align) = parse_alignment(el) {
        let _ = writeln!(out, "#align({align})[{prefix} {content}]");
    } else {
        let _ = writeln!(out, "{prefix} {content}");
    }
}

/// Emit a `<p>` or `<div>` with optional alignment.
fn emit_aligned_block(
    node: NodeRef<'_, Node>,
    el: &Element,
    out: &mut String,
    warnings: &mut WarningCollector,
) {
    let mut content = String::new();
    emit_children(node, &mut content, warnings, Context::Block);

    if let Some(align) = parse_alignment(el) {
        let _ = writeln!(out, "#align({align})[{content}]");
    } else {
        out.push_str(&content);
        if !content.ends_with('\n') {
            out.push('\n');
        }
    }
}

/// Emit an `<a>` link as `#link("url")[text]`.
fn emit_link(
    node: NodeRef<'_, Node>,
    el: &Element,
    out: &mut String,
    warnings: &mut WarningCollector,
    ctx: Context,
) {
    let href = el.attr("href").unwrap_or("");
    let _ = write!(out, "#link(\"{}\")", escape_typst_string(href));

    let mut content = String::new();
    emit_children(node, &mut content, warnings, ctx);

    if !content.is_empty() {
        let _ = write!(out, "[{content}]");
    }
}

/// Emit an `<img>` as a `#figure(image(...))` or alt-text placeholder.
///
/// In `TableCell` context, emits bare `image()` without `#figure()` wrapper
/// to avoid caption spacing overhead inside table cells.
fn emit_image(
    el: &Element,
    out: &mut String,
    warnings: &mut WarningCollector,
    ctx: Context,
) {
    let src = el.attr("src").unwrap_or("");
    let alt = el.attr("alt").unwrap_or("");

    // Remote images: emit alt text as placeholder + warning
    if src.starts_with("http://") || src.starts_with("https://") {
        if !alt.is_empty() {
            out.push_str(&escape_typst_content(alt));
        }
        warnings.push(SilkprintWarning::RemoteImageSkipped {
            url: src.to_string(),
        });
        return;
    }

    let width = parse_image_width(el);
    let escaped_src = escape_typst_string(src);

    if ctx == Context::TableCell {
        // Bare #image() inside table cell — no figure wrapper to avoid spacing overhead
        let _ = write!(out, "#image(\"{escaped_src}\", width: {width})");
    } else {
        let _ = write!(out, "#figure(image(\"{escaped_src}\", width: {width}))");
    }
}

/// Two-pass table emitter: count columns, then emit `#table(columns: N, ...)`.
fn emit_table(
    node: NodeRef<'_, Node>,
    out: &mut String,
    warnings: &mut WarningCollector,
) {
    let rows = collect_table_rows(node);
    if rows.is_empty() {
        return;
    }

    // First pass: max column count
    let num_cols = rows
        .iter()
        .map(|row| count_row_cells(*row))
        .max()
        .unwrap_or(0);

    if num_cols == 0 {
        return;
    }

    let _ = writeln!(out, "#table(");
    let _ = writeln!(out, "  columns: {num_cols},");

    // Second pass: emit cells
    for row in &rows {
        emit_table_row(*row, out, warnings);
    }

    out.push_str(")\n");
}

/// Emit list items with `- ` (unordered) or `+ ` (ordered) markers.
fn emit_list(
    node: NodeRef<'_, Node>,
    out: &mut String,
    warnings: &mut WarningCollector,
    ordered: bool,
) {
    let marker = if ordered { "+ " } else { "- " };

    for child in node.children() {
        if let Node::Element(ref el) = *child.value() {
            if el.name() == "li" {
                out.push_str(marker);
                let mut item_content = String::new();
                emit_children(child, &mut item_content, warnings, Context::Inline);
                out.push_str(item_content.trim());
                out.push('\n');
            }
        }
    }
}

// ─── Table Helpers ───────────────────────────────────────────────────

/// Collect all `<tr>` elements from a table, looking through `<thead>`/`<tbody>`.
fn collect_table_rows(table_node: NodeRef<'_, Node>) -> Vec<NodeRef<'_, Node>> {
    let mut rows = Vec::new();

    for child in table_node.children() {
        if let Node::Element(ref el) = *child.value() {
            match el.name() {
                "tr" => rows.push(child),
                "thead" | "tbody" | "tfoot" => {
                    for grandchild in child.children() {
                        if let Node::Element(ref gc_el) = *grandchild.value() {
                            if gc_el.name() == "tr" {
                                rows.push(grandchild);
                            }
                        }
                    }
                }
                _ => {}
            }
        }
    }

    rows
}

/// Count the number of `<td>`/`<th>` cells in a `<tr>`.
fn count_row_cells(row: NodeRef<'_, Node>) -> usize {
    row.children()
        .filter(|child: &NodeRef<'_, Node>| {
            if let Node::Element(ref el) = *child.value() {
                matches!(el.name(), "td" | "th")
            } else {
                false
            }
        })
        .count()
}

/// Emit all cells in a `<tr>` as `[content],` entries.
fn emit_table_row(
    row: NodeRef<'_, Node>,
    out: &mut String,
    warnings: &mut WarningCollector,
) {
    for child in row.children() {
        if let Node::Element(ref el) = *child.value() {
            let tag = el.name();
            if matches!(tag, "td" | "th") {
                let mut cell_content = String::new();
                emit_children(child, &mut cell_content, warnings, Context::TableCell);

                let align = parse_alignment(el);

                // <th> wraps content in bold
                let formatted = if tag == "th" {
                    format!("*{}*", cell_content.trim())
                } else {
                    cell_content.trim().to_string()
                };

                if let Some(a) = align {
                    let _ = writeln!(out, "  [#align({a})[{formatted}]],");
                } else {
                    let _ = writeln!(out, "  [{formatted}],");
                }
            }
        }
    }
}

// ─── Shared Helpers ──────────────────────────────────────────────────

/// Emit all children of a node into the output buffer.
fn emit_children(
    node: NodeRef<'_, Node>,
    out: &mut String,
    warnings: &mut WarningCollector,
    ctx: Context,
) {
    for child in node.children() {
        emit_dom_node(child, out, warnings, ctx);
    }
}

/// Collect raw text from all descendant text nodes (no escaping).
/// Used for `<code>` content where Typst backtick-delimited text is literal.
fn collect_text(node: NodeRef<'_, Node>, out: &mut String) {
    for child in node.children() {
        match *child.value() {
            Node::Text(ref t) => out.push_str(t),
            Node::Element(_) => collect_text(child, out),
            _ => {}
        }
    }
}

/// Strip Typst line breaks (`\` + newline) and collapse whitespace in heading content.
///
/// HTML headings often contain `<br>` tags for layout purposes (e.g., GitHub README
/// headers like `<h1><br>Title<br></h1>`). These produce `\` line breaks that look
/// terrible inside Typst headings. We strip them and collapse runs of whitespace.
fn clean_heading_content(s: &str) -> String {
    s.replace("\\\n", " ")
        .replace('\n', " ")
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
}

/// Parse the `align` attribute into a Typst alignment keyword.
fn parse_alignment(el: &Element) -> Option<&'static str> {
    el.attr("align").and_then(|a| match a.to_lowercase().as_str() {
        "center" => Some("center"),
        "right" => Some("right"),
        "left" => Some("left"),
        _ => None,
    })
}

/// Extract heading level from tag name (h1 -> 1, h6 -> 6).
fn heading_level(tag: &str) -> usize {
    tag.strip_prefix('h')
        .and_then(|n| n.parse::<usize>().ok())
        .unwrap_or(1)
}

/// Max sensible pixel width before capping to 100%.
///
/// A4 text area with 25mm margins is ~160mm = ~454pt. Values above this
/// would overflow the page, so we clamp them to `100%` instead.
const MAX_IMAGE_PT: f64 = 454.0;

/// Parse the `width` attribute of an `<img>` into a Typst width expression.
///
/// - `"50%"` -> `"50%"`
/// - `"200"` or `"200px"` -> `"200pt"` (capped at page width)
/// - absent -> `"100%"`
fn parse_image_width(el: &Element) -> String {
    let Some(raw) = el.attr("width") else {
        return "100%".to_string();
    };

    let trimmed = raw.trim();

    if trimmed.ends_with('%') {
        return trimmed.to_string();
    }

    // Strip trailing "px" if present, then treat as pt
    let numeric = trimmed.strip_suffix("px").unwrap_or(trimmed);

    if numeric.chars().all(|c| c.is_ascii_digit() || c == '.') {
        // Cap large pixel values to 80% of text width — prevents images
        // from consuming entire pages in PDF output.
        if let Ok(val) = numeric.parse::<f64>() {
            if val > MAX_IMAGE_PT {
                return "80%".to_string();
            }
        }
        format!("{numeric}pt")
    } else {
        "100%".to_string()
    }
}

// ─── Tests ───────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::warnings::WarningCollector;

    #[test]
    fn test_centered_heading() {
        let mut w = WarningCollector::new();
        let result = emit_html_block("<h1 align=\"center\">Title</h1>", &mut w);
        assert!(result.contains("#align(center)"), "got: {result}");
        assert!(result.contains("= Title"), "got: {result}");
    }

    #[test]
    fn test_heading_levels() {
        let mut w = WarningCollector::new();
        let result = emit_html_block("<h3>Third</h3>", &mut w);
        assert!(result.contains("=== Third"), "got: {result}");
    }

    #[test]
    fn test_table() {
        let mut w = WarningCollector::new();
        let result = emit_html_block(
            "<table><tr><th>A</th><th>B</th></tr><tr><td>1</td><td>2</td></tr></table>",
            &mut w,
        );
        assert!(result.contains("#table("), "got: {result}");
        assert!(result.contains("columns: 2"), "got: {result}");
        assert!(result.contains("[*A*]"), "got: {result}");
    }

    #[test]
    fn test_table_with_sections() {
        let mut w = WarningCollector::new();
        let result = emit_html_block(
            "<table><thead><tr><th>H</th></tr></thead><tbody><tr><td>D</td></tr></tbody></table>",
            &mut w,
        );
        assert!(result.contains("#table("), "got: {result}");
        assert!(result.contains("[*H*]"), "got: {result}");
        assert!(result.contains("[D]"), "got: {result}");
    }

    #[test]
    fn test_image_with_width() {
        let mut w = WarningCollector::new();
        let result = emit_html_block("<img src=\"logo.png\" width=\"200\">", &mut w);
        assert!(result.contains("image(\"logo.png\""), "got: {result}");
        assert!(result.contains("width: 200pt"), "got: {result}");
    }

    #[test]
    fn test_image_percentage_width() {
        let mut w = WarningCollector::new();
        let result = emit_html_block("<img src=\"wide.png\" width=\"80%\">", &mut w);
        assert!(result.contains("width: 80%"), "got: {result}");
    }

    #[test]
    fn test_image_default_width() {
        let mut w = WarningCollector::new();
        let result = emit_html_block("<img src=\"photo.jpg\">", &mut w);
        assert!(result.contains("width: 100%"), "got: {result}");
    }

    #[test]
    fn test_remote_image_placeholder() {
        let mut w = WarningCollector::new();
        let result =
            emit_html_block("<img src=\"https://example.com/img.png\" alt=\"Badge\">", &mut w);
        assert!(result.contains("Badge"), "got: {result}");
        assert!(!result.contains("image("), "got: {result}");
        assert_eq!(w.warnings().len(), 1);
    }

    #[test]
    fn test_link() {
        let mut w = WarningCollector::new();
        let result = emit_html_inline("<a href=\"https://example.com\">Click</a>", &mut w);
        assert!(
            result.contains("#link(\"https://example.com\")[Click]"),
            "got: {result}"
        );
    }

    #[test]
    fn test_strong() {
        let mut w = WarningCollector::new();
        let result = emit_html_inline("<strong>bold</strong>", &mut w);
        assert!(result.contains("*bold*"), "got: {result}");
    }

    #[test]
    fn test_bold_tag() {
        let mut w = WarningCollector::new();
        let result = emit_html_inline("<b>bold</b>", &mut w);
        assert!(result.contains("*bold*"), "got: {result}");
    }

    #[test]
    fn test_emphasis() {
        let mut w = WarningCollector::new();
        let result = emit_html_inline("<em>italic</em>", &mut w);
        assert!(result.contains("_italic_"), "got: {result}");
    }

    #[test]
    fn test_inline_code() {
        let mut w = WarningCollector::new();
        let result = emit_html_inline("<code>foo</code>", &mut w);
        assert!(result.contains("`foo`"), "got: {result}");
    }

    #[test]
    fn test_sub_sup() {
        let mut w = WarningCollector::new();
        let sub = emit_html_inline("<sub>2</sub>", &mut w);
        let sup = emit_html_inline("<sup>n</sup>", &mut w);
        assert!(sub.contains("#sub[2]"), "got: {sub}");
        assert!(sup.contains("#super[n]"), "got: {sup}");
    }

    #[test]
    fn test_br() {
        let mut w = WarningCollector::new();
        let result = emit_html_inline("before<br>after", &mut w);
        assert!(result.contains('\\'), "got: {result}");
    }

    #[test]
    fn test_br_self_closing() {
        let mut w = WarningCollector::new();
        let result = emit_html_inline("before<br/>after", &mut w);
        assert!(result.contains('\\'), "got: {result}");
    }

    #[test]
    fn test_hr() {
        let mut w = WarningCollector::new();
        let result = emit_html_block("<hr>", &mut w);
        assert!(result.contains("#line(length: 100%)"), "got: {result}");
    }

    #[test]
    fn test_unordered_list() {
        let mut w = WarningCollector::new();
        let result = emit_html_block("<ul><li>Alpha</li><li>Beta</li></ul>", &mut w);
        assert!(result.contains("- Alpha"), "got: {result}");
        assert!(result.contains("- Beta"), "got: {result}");
    }

    #[test]
    fn test_ordered_list() {
        let mut w = WarningCollector::new();
        let result = emit_html_block("<ol><li>One</li><li>Two</li></ol>", &mut w);
        assert!(result.contains("+ One"), "got: {result}");
        assert!(result.contains("+ Two"), "got: {result}");
    }

    #[test]
    fn test_span_transparent() {
        let mut w = WarningCollector::new();
        let result = emit_html_inline("<span>hello</span>", &mut w);
        assert!(result.contains("hello"), "got: {result}");
        assert!(w.is_empty());
    }

    #[test]
    fn test_div_with_alignment() {
        let mut w = WarningCollector::new();
        let result = emit_html_block("<div align=\"center\">Centered</div>", &mut w);
        assert!(result.contains("#align(center)"), "got: {result}");
        assert!(result.contains("Centered"), "got: {result}");
    }

    #[test]
    fn test_unknown_tag_warns() {
        let mut w = WarningCollector::new();
        let result = emit_html_inline("<marquee>scroll</marquee>", &mut w);
        assert!(result.contains("scroll"), "got: {result}");
        assert!(!w.is_empty());
        let warning = &w.warnings()[0];
        assert!(
            matches!(warning, SilkprintWarning::UnsupportedHtmlTag { tag } if tag == "marquee"),
            "got: {warning:?}"
        );
    }

    #[test]
    fn test_nested_inline() {
        let mut w = WarningCollector::new();
        let result = emit_html_inline("<strong><em>bold italic</em></strong>", &mut w);
        assert!(result.contains("*_bold italic_*"), "got: {result}");
    }

    #[test]
    fn test_p_alignment() {
        let mut w = WarningCollector::new();
        let result = emit_html_block("<p align=\"right\">Right text</p>", &mut w);
        assert!(result.contains("#align(right)"), "got: {result}");
        assert!(result.contains("Right text"), "got: {result}");
    }

    #[test]
    fn test_image_px_width() {
        let mut w = WarningCollector::new();
        let result = emit_html_block("<img src=\"icon.png\" width=\"32px\">", &mut w);
        assert!(result.contains("width: 32pt"), "got: {result}");
    }

    #[test]
    fn test_empty_html() {
        let mut w = WarningCollector::new();
        let block = emit_html_block("", &mut w);
        let inline = emit_html_inline("", &mut w);
        assert!(block.is_empty() || block.trim().is_empty(), "got: {block}");
        assert!(inline.is_empty() || inline.trim().is_empty(), "got: {inline}");
    }

    #[test]
    fn test_table_aligned_cells() {
        let mut w = WarningCollector::new();
        let result = emit_html_block(
            "<table><tr><td align=\"right\">R</td><td>L</td></tr></table>",
            &mut w,
        );
        assert!(result.contains("#align(right)"), "got: {result}");
        assert!(result.contains("[L]"), "got: {result}");
    }
}
