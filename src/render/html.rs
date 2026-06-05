//! HTML-to-Typst converter for embedded HTML blocks and inline HTML.
//!
//! `SilkPrint`'s markdown pipeline encounters raw HTML in two forms:
//! - **Block HTML**: `<table>`, `<div align="center">`, etc.
//! - **Inline HTML**: `<strong>`, `<a>`, `<br>`, etc.
//!
//! This module parses HTML via `scraper` and emits equivalent Typst markup,
//! pushing warnings for unsupported tags and image fallbacks when needed.

use std::fmt::Write;

use ego_tree::NodeRef;
use scraper::Html;
use scraper::node::{Element, Node};

use crate::warnings::{SilkprintWarning, WarningCollector};

use super::escape::escape_typst_content;
use super::image::PreparedImages;

mod images;
mod links;
mod tables;

pub(crate) use images::collect_sources as collect_image_sources;

// ─── Public API ──────────────────────────────────────────────────────

/// Convert a block-level HTML string into Typst markup.
///
/// Parses the HTML as a full document and walks the DOM tree, emitting
/// Typst equivalents for supported elements.
pub fn emit_html_block(
    html: &str,
    images: &PreparedImages,
    warnings: &mut WarningCollector,
) -> String {
    let doc = Html::parse_document(html);
    let mut out = String::new();

    for child in doc.tree.root().children() {
        emit_dom_node(child, &mut out, images, warnings, Context::Block);
    }

    out
}

/// Convert an inline HTML fragment into Typst markup.
///
/// Parses the HTML as a fragment and emits only inline-level Typst.
pub fn emit_html_inline(
    html: &str,
    images: &PreparedImages,
    warnings: &mut WarningCollector,
) -> String {
    let doc = Html::parse_fragment(html);
    let mut out = String::new();

    for child in doc.tree.root().children() {
        emit_dom_node(child, &mut out, images, warnings, Context::Inline);
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
    images: &PreparedImages,
    warnings: &mut WarningCollector,
    ctx: Context,
) {
    match node.value() {
        Node::Text(text) => {
            out.push_str(&escape_typst_content(text));
        }

        Node::Element(el) => {
            let tag = el.name();
            emit_element(tag, node, el, out, images, warnings, ctx);
        }

        // Document/Fragment roots, doctype, comments — recurse through children
        _ => {
            for child in node.children() {
                emit_dom_node(child, out, images, warnings, ctx);
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
    images: &PreparedImages,
    warnings: &mut WarningCollector,
    ctx: Context,
) {
    match tag {
        // ─── Headings ────────────────────────────────────────────
        "h1" | "h2" | "h3" | "h4" | "h5" | "h6" => {
            if ctx != Context::Block {
                return;
            }
            emit_heading(tag, node, el, out, images, warnings);
        }

        // ─── Block containers ────────────────────────────────────
        "p" | "div" => {
            if ctx != Context::Block {
                // In inline/table-cell context, just emit children directly
                emit_children(node, out, images, warnings, ctx);
                return;
            }
            emit_aligned_block(node, el, out, images, warnings);
        }

        // ─── Table ───────────────────────────────────────────────
        "table" => {
            if ctx != Context::Block {
                return;
            }
            tables::emit(node, out, images, warnings);
        }

        // ─── Images ──────────────────────────────────────────────
        "img" => images::emit(el, out, images, warnings, ctx),

        // ─── Links ───────────────────────────────────────────────
        "a" => links::emit(node, el, out, images, warnings),

        // ─── Inline formatting ───────────────────────────────────
        "strong" | "b" => {
            out.push('*');
            emit_children(node, out, images, warnings, ctx);
            out.push('*');
        }

        "em" | "i" => {
            out.push('_');
            emit_children(node, out, images, warnings, ctx);
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
            emit_children(node, out, images, warnings, ctx);
            out.push(']');
        }

        "sup" => {
            out.push_str("#super[");
            emit_children(node, out, images, warnings, ctx);
            out.push(']');
        }

        // ─── Line break ─────────────────────────────────────────
        "br" => {
            out.push_str("#linebreak()\n");
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
            emit_list(node, out, images, warnings, false);
        }

        "ol" => {
            if ctx != Context::Block {
                return;
            }
            emit_list(node, out, images, warnings, true);
        }

        // ─── Transparent wrappers ────────────────────────────────
        // Table section wrappers, stray row/cell elements, list items
        // outside list context, spans, and structural html/head/body
        // all just pass through to their children.
        "thead" | "tbody" | "tfoot" | "tr" | "td" | "th" | "li" | "span" | "html" | "head"
        | "body" => {
            emit_children(node, out, images, warnings, ctx);
        }

        // ─── Unknown tags ────────────────────────────────────────
        _ => {
            warnings.push(SilkprintWarning::UnsupportedHtmlTag {
                tag: tag.to_string(),
            });
            emit_children(node, out, images, warnings, ctx);
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
    images: &PreparedImages,
    warnings: &mut WarningCollector,
) {
    let level = heading_level(tag);
    let prefix = "=".repeat(level);

    let mut content = String::new();
    emit_children(node, &mut content, images, warnings, Context::Inline);

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
    images: &PreparedImages,
    warnings: &mut WarningCollector,
) {
    let mut content = String::new();
    emit_children(node, &mut content, images, warnings, Context::Block);

    if let Some(align) = parse_alignment(el) {
        let _ = writeln!(out, "#align({align})[{content}]");
    } else {
        out.push_str(&content);
        if !content.ends_with('\n') {
            out.push('\n');
        }
    }
}

/// Emit list items with `- ` (unordered) or `+ ` (ordered) markers.
fn emit_list(
    node: NodeRef<'_, Node>,
    out: &mut String,
    images: &PreparedImages,
    warnings: &mut WarningCollector,
    ordered: bool,
) {
    let marker = if ordered { "+ " } else { "- " };

    for child in node.children() {
        if let Node::Element(ref el) = *child.value()
            && el.name() == "li"
        {
            out.push_str(marker);
            let mut item_content = String::new();
            emit_children(child, &mut item_content, images, warnings, Context::Inline);
            out.push_str(item_content.trim());
            out.push('\n');
        }
    }
}

// ─── Shared Helpers ──────────────────────────────────────────────────

/// Emit all children of a node into the output buffer.
fn emit_children(
    node: NodeRef<'_, Node>,
    out: &mut String,
    images: &PreparedImages,
    warnings: &mut WarningCollector,
    ctx: Context,
) {
    for child in node.children() {
        emit_dom_node(child, out, images, warnings, ctx);
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

/// Strip Typst line break primitives and collapse whitespace in heading content.
///
/// HTML headings often contain `<br>` tags for layout purposes (e.g., GitHub README
/// headers like `<h1><br>Title<br></h1>`). These produce explicit line breaks that
/// look terrible inside Typst headings. We strip them and collapse runs of whitespace.
fn clean_heading_content(s: &str) -> String {
    s.replace("#linebreak()", " ")
        .replace('\n', " ")
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
}

/// Parse the `align` attribute into a Typst alignment keyword.
fn parse_alignment(el: &Element) -> Option<&'static str> {
    el.attr("align")
        .and_then(|a| match a.to_lowercase().as_str() {
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

// ─── Tests ───────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::render::image::PreparedImages;
    use crate::warnings::WarningCollector;

    fn images() -> PreparedImages {
        PreparedImages::default()
    }

    #[test]
    fn test_centered_heading() {
        let mut w = WarningCollector::new();
        let result = emit_html_block("<h1 align=\"center\">Title</h1>", &images(), &mut w);
        assert!(result.contains("#align(center)"), "got: {result}");
        assert!(result.contains("= Title"), "got: {result}");
    }

    #[test]
    fn test_heading_levels() {
        let mut w = WarningCollector::new();
        let result = emit_html_block("<h3>Third</h3>", &images(), &mut w);
        assert!(result.contains("=== Third"), "got: {result}");
    }

    #[test]
    fn test_table() {
        let mut w = WarningCollector::new();
        let result = emit_html_block(
            "<table><tr><th>A</th><th>B</th></tr><tr><td>1</td><td>2</td></tr></table>",
            &images(),
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
            &images(),
            &mut w,
        );
        assert!(result.contains("#table("), "got: {result}");
        assert!(result.contains("[*H*]"), "got: {result}");
        assert!(result.contains("[D]"), "got: {result}");
    }

    #[test]
    fn test_image_with_width() {
        let mut w = WarningCollector::new();
        let result = emit_html_block("<img src=\"logo.png\" width=\"200\">", &images(), &mut w);
        assert!(result.contains("image(\"logo.png\""), "got: {result}");
        assert!(result.contains("width: 200pt"), "got: {result}");
    }

    #[test]
    fn test_image_percentage_width() {
        let mut w = WarningCollector::new();
        let result = emit_html_block("<img src=\"wide.png\" width=\"80%\">", &images(), &mut w);
        assert!(result.contains("width: 80%"), "got: {result}");
    }

    #[test]
    fn test_image_default_width() {
        let mut w = WarningCollector::new();
        let result = emit_html_block("<img src=\"photo.jpg\">", &images(), &mut w);
        // No explicit width attr → image uses natural size (no width param)
        assert!(!result.contains("width:"), "got: {result}");
        assert!(result.contains("image(\"photo.jpg\")"), "got: {result}");
    }

    #[test]
    fn test_remote_image_placeholder() {
        let mut w = WarningCollector::new();
        let result = emit_html_block(
            "<img src=\"https://example.com/img.png\" alt=\"Badge\">",
            &images(),
            &mut w,
        );
        assert!(result.contains("Badge"), "got: {result}");
        assert!(!result.contains("image("), "got: {result}");
        assert_eq!(w.warnings().len(), 1);
    }

    #[test]
    fn test_link() {
        let mut w = WarningCollector::new();
        let result = emit_html_inline(
            "<a href=\"https://example.com\">Click</a>",
            &images(),
            &mut w,
        );
        assert!(
            result.contains("#link(\"https://example.com\")[Click]"),
            "got: {result}"
        );
    }

    #[test]
    fn test_strong() {
        let mut w = WarningCollector::new();
        let result = emit_html_inline("<strong>bold</strong>", &images(), &mut w);
        assert!(result.contains("*bold*"), "got: {result}");
    }

    #[test]
    fn test_bold_tag() {
        let mut w = WarningCollector::new();
        let result = emit_html_inline("<b>bold</b>", &images(), &mut w);
        assert!(result.contains("*bold*"), "got: {result}");
    }

    #[test]
    fn test_emphasis() {
        let mut w = WarningCollector::new();
        let result = emit_html_inline("<em>italic</em>", &images(), &mut w);
        assert!(result.contains("_italic_"), "got: {result}");
    }

    #[test]
    fn test_inline_code() {
        let mut w = WarningCollector::new();
        let result = emit_html_inline("<code>foo</code>", &images(), &mut w);
        assert!(result.contains("`foo`"), "got: {result}");
    }

    #[test]
    fn test_sub_sup() {
        let mut w = WarningCollector::new();
        let sub = emit_html_inline("<sub>2</sub>", &images(), &mut w);
        let sup = emit_html_inline("<sup>n</sup>", &images(), &mut w);
        assert!(sub.contains("#sub[2]"), "got: {sub}");
        assert!(sup.contains("#super[n]"), "got: {sup}");
    }

    #[test]
    fn test_br() {
        let mut w = WarningCollector::new();
        let result = emit_html_inline("before<br>after", &images(), &mut w);
        assert!(result.contains("#linebreak()"), "got: {result}");
    }

    #[test]
    fn test_br_self_closing() {
        let mut w = WarningCollector::new();
        let result = emit_html_inline("before<br/>after", &images(), &mut w);
        assert!(result.contains("#linebreak()"), "got: {result}");
    }

    #[test]
    fn test_hr() {
        let mut w = WarningCollector::new();
        let result = emit_html_block("<hr>", &images(), &mut w);
        assert!(result.contains("#line(length: 100%)"), "got: {result}");
    }

    #[test]
    fn test_unordered_list() {
        let mut w = WarningCollector::new();
        let result = emit_html_block("<ul><li>Alpha</li><li>Beta</li></ul>", &images(), &mut w);
        assert!(result.contains("- Alpha"), "got: {result}");
        assert!(result.contains("- Beta"), "got: {result}");
    }

    #[test]
    fn test_ordered_list() {
        let mut w = WarningCollector::new();
        let result = emit_html_block("<ol><li>One</li><li>Two</li></ol>", &images(), &mut w);
        assert!(result.contains("+ One"), "got: {result}");
        assert!(result.contains("+ Two"), "got: {result}");
    }

    #[test]
    fn test_span_transparent() {
        let mut w = WarningCollector::new();
        let result = emit_html_inline("<span>hello</span>", &images(), &mut w);
        assert!(result.contains("hello"), "got: {result}");
        assert!(w.is_empty());
    }

    #[test]
    fn test_div_with_alignment() {
        let mut w = WarningCollector::new();
        let result = emit_html_block("<div align=\"center\">Centered</div>", &images(), &mut w);
        assert!(result.contains("#align(center)"), "got: {result}");
        assert!(result.contains("Centered"), "got: {result}");
    }

    #[test]
    fn test_unknown_tag_warns() {
        let mut w = WarningCollector::new();
        let result = emit_html_inline("<marquee>scroll</marquee>", &images(), &mut w);
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
        let result = emit_html_inline("<strong><em>bold italic</em></strong>", &images(), &mut w);
        assert!(result.contains("*_bold italic_*"), "got: {result}");
    }

    #[test]
    fn test_p_alignment() {
        let mut w = WarningCollector::new();
        let result = emit_html_block("<p align=\"right\">Right text</p>", &images(), &mut w);
        assert!(result.contains("#align(right)"), "got: {result}");
        assert!(result.contains("Right text"), "got: {result}");
    }

    #[test]
    fn test_image_px_width() {
        let mut w = WarningCollector::new();
        let result = emit_html_block("<img src=\"icon.png\" width=\"32px\">", &images(), &mut w);
        assert!(result.contains("width: 32pt"), "got: {result}");
    }

    #[test]
    fn test_empty_html() {
        let mut w = WarningCollector::new();
        let block = emit_html_block("", &images(), &mut w);
        let inline = emit_html_inline("", &images(), &mut w);
        assert!(block.is_empty() || block.trim().is_empty(), "got: {block}");
        assert!(
            inline.is_empty() || inline.trim().is_empty(),
            "got: {inline}"
        );
    }

    #[test]
    fn test_table_aligned_cells() {
        let mut w = WarningCollector::new();
        let result = emit_html_block(
            "<table><tr><td align=\"right\">R</td><td>L</td></tr></table>",
            &images(),
            &mut w,
        );
        assert!(result.contains("#align(right)"), "got: {result}");
        assert!(result.contains("[L]"), "got: {result}");
    }
}
