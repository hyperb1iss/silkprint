use std::collections::HashMap;
use std::fmt::Write;

use comrak::Options;
use comrak::nodes::{AstNode, ListType, NodeValue, TableAlignment};

use crate::theme::ResolvedTheme;
use crate::warnings::{SilkprintWarning, WarningCollector};

use super::escape::{escape_typst_content, escape_typst_string};

/// Configure comrak with all extensions enabled per SPEC Section 8.2.
pub fn comrak_options() -> Options<'static> {
    let mut options = Options::default();

    // Core extensions
    options.extension.strikethrough = true;
    options.extension.table = true;
    options.extension.autolink = true;
    options.extension.tasklist = true;
    options.extension.superscript = true;
    options.extension.subscript = true;
    options.extension.footnotes = true;
    options.extension.description_lists = true;
    options.extension.highlight = true;
    options.extension.underline = true;

    // Math, front matter, alerts
    options.extension.math_dollars = true;
    options.extension.front_matter_delimiter = Some("---".to_owned());
    options.extension.alerts = true;

    // Emoji and wikilinks
    options.extension.shortcodes = true;
    options.extension.wikilinks_title_after_pipe = true;

    options
}

/// Parse markdown into a comrak AST.
pub fn parse<'a>(arena: &'a comrak::Arena<'a>, input: &str) -> &'a AstNode<'a> {
    let options = comrak_options();
    comrak::parse_document(arena, input, &options)
}

/// Walk a comrak AST and emit Typst markup.
///
/// Traverses every node in the tree, converting each to valid Typst syntax.
/// Footnote definitions are collected during traversal and inlined at their
/// reference sites via `#footnote[...]`.
///
/// Mermaid code blocks are emitted as image references to virtual SVG files.
/// The collected mermaid sources are returned so the caller can render them
/// before Typst compilation.
pub fn emit_typst<'a>(
    root: &'a AstNode<'a>,
    _theme: &ResolvedTheme,
    warnings: &mut WarningCollector,
) -> (String, Vec<String>) {
    // First pass: collect footnote definitions by name so we can inline them
    // at the reference site (Typst's #footnote[...] model).
    let footnotes = collect_footnote_definitions(root, warnings);

    let mut ctx = EmitContext {
        out: String::with_capacity(8192),
        indent: 0,
        footnotes,
        table_alignments: Vec::new(),
        table_cell_index: 0,
        in_table_header: false,
        in_tight_list: false,
        warnings,
        mermaid_sources: Vec::new(),
        mermaid_counter: 0,
    };

    emit_node(root, &mut ctx);

    (ctx.out, ctx.mermaid_sources)
}

// ═══════════════════════════════════════════════════════════════════
// Emitter context & recursive walker
// ═══════════════════════════════════════════════════════════════════

/// Mutable state carried through the recursive tree walk.
struct EmitContext<'w> {
    out: String,
    indent: usize,
    footnotes: HashMap<String, String>,
    table_alignments: Vec<TableAlignment>,
    table_cell_index: usize,
    in_table_header: bool,
    in_tight_list: bool,
    warnings: &'w mut WarningCollector,
    mermaid_sources: Vec<String>,
    mermaid_counter: usize,
}

impl EmitContext<'_> {
    fn push(&mut self, s: &str) {
        self.out.push_str(s);
    }

    fn push_indent(&mut self) {
        for _ in 0..self.indent {
            self.out.push_str("  ");
        }
    }

    fn newline(&mut self) {
        self.out.push('\n');
    }
}

/// Extract data we need from a node, cloning what's necessary to avoid
/// holding the `RefCell` borrow across child traversal.
///
/// This enum mirrors `NodeValue` but owns all the data it needs so we can
/// drop the `Ref` immediately after extraction.
enum ExtractedNode {
    Document,
    FrontMatter,
    Paragraph,
    Heading {
        level: u8,
    },
    ThematicBreak,
    Text(String),
    SoftBreak,
    LineBreak,
    Strong,
    Emph,
    Strikethrough,
    Underline,
    Superscript,
    Subscript,
    Highlight,
    Code {
        literal: String,
        num_backticks: usize,
    },
    Link {
        url: String,
    },
    Image {
        url: String,
    },
    WikiLink {
        url: String,
    },
    BlockQuote,
    MultilineBlockQuote,
    List {
        is_ordered: bool,
        tight: bool,
        start: usize,
    },
    Item,
    TaskItem {
        checked: bool,
    },
    CodeBlock {
        info: String,
        literal: String,
    },
    HtmlBlock {
        literal: String,
    },
    HtmlInline(String),
    Table {
        alignments: Vec<TableAlignment>,
        num_columns: usize,
    },
    TableRow {
        is_header: bool,
    },
    TableCell,
    FootnoteDefinition,
    FootnoteReference {
        name: String,
    },
    Math {
        literal: String,
        display: bool,
    },
    Alert {
        title: String,
        icon: &'static str,
    },
    DescriptionList,
    DescriptionItem,
    DescriptionTerm,
    DescriptionDetails,
    ShortCode {
        emoji: String,
    },
    Escaped,
    EscapedTag,
    SpoileredText,
    Subtext,
    Raw(String),
}

/// Extract all needed data from a node value, cloning strings so we can
/// drop the `Ref<Ast>` borrow immediately.
#[allow(clippy::too_many_lines)]
fn extract_node(node: &AstNode<'_>) -> ExtractedNode {
    let data = node.data.borrow();
    match &data.value {
        NodeValue::Document => ExtractedNode::Document,
        NodeValue::FrontMatter(_) => ExtractedNode::FrontMatter,
        NodeValue::Paragraph => ExtractedNode::Paragraph,
        NodeValue::Heading(h) => ExtractedNode::Heading { level: h.level },
        NodeValue::ThematicBreak => ExtractedNode::ThematicBreak,
        NodeValue::Text(t) => ExtractedNode::Text(t.to_string()),
        NodeValue::SoftBreak => ExtractedNode::SoftBreak,
        NodeValue::LineBreak => ExtractedNode::LineBreak,
        NodeValue::Strong => ExtractedNode::Strong,
        NodeValue::Emph => ExtractedNode::Emph,
        NodeValue::Strikethrough => ExtractedNode::Strikethrough,
        NodeValue::Underline => ExtractedNode::Underline,
        NodeValue::Superscript => ExtractedNode::Superscript,
        NodeValue::Subscript => ExtractedNode::Subscript,
        NodeValue::Highlight => ExtractedNode::Highlight,
        NodeValue::Code(c) => ExtractedNode::Code {
            literal: c.literal.clone(),
            num_backticks: c.num_backticks,
        },
        NodeValue::Link(l) => ExtractedNode::Link { url: l.url.clone() },
        NodeValue::Image(l) => ExtractedNode::Image { url: l.url.clone() },
        NodeValue::WikiLink(w) => ExtractedNode::WikiLink { url: w.url.clone() },
        NodeValue::BlockQuote => ExtractedNode::BlockQuote,
        NodeValue::MultilineBlockQuote(_) => ExtractedNode::MultilineBlockQuote,
        NodeValue::List(list) => ExtractedNode::List {
            is_ordered: list.list_type == ListType::Ordered,
            tight: list.tight,
            start: list.start,
        },
        NodeValue::Item(_) => ExtractedNode::Item,
        NodeValue::TaskItem(t) => ExtractedNode::TaskItem {
            checked: t.symbol.is_some(),
        },
        NodeValue::CodeBlock(cb) => ExtractedNode::CodeBlock {
            info: cb.info.clone(),
            literal: cb.literal.clone(),
        },
        NodeValue::HtmlBlock(h) => ExtractedNode::HtmlBlock {
            literal: h.literal.clone(),
        },
        NodeValue::HtmlInline(h) => ExtractedNode::HtmlInline(h.clone()),
        NodeValue::Table(t) => ExtractedNode::Table {
            alignments: t.alignments.clone(),
            num_columns: t.num_columns,
        },
        NodeValue::TableRow(is_header) => ExtractedNode::TableRow {
            is_header: *is_header,
        },
        NodeValue::TableCell => ExtractedNode::TableCell,
        NodeValue::FootnoteDefinition(_) => ExtractedNode::FootnoteDefinition,
        NodeValue::FootnoteReference(f) => ExtractedNode::FootnoteReference {
            name: f.name.clone(),
        },
        NodeValue::Math(m) => ExtractedNode::Math {
            literal: m.literal.clone(),
            display: m.display_math,
        },
        NodeValue::Alert(a) => {
            let alert_type = a.alert_type;
            let title = a
                .title
                .clone()
                .unwrap_or_else(|| alert_type.default_title().to_string());
            let (icon, _) = alert_icon_and_color(alert_type);
            ExtractedNode::Alert { title, icon }
        }
        NodeValue::DescriptionList => ExtractedNode::DescriptionList,
        NodeValue::DescriptionItem(_) => ExtractedNode::DescriptionItem,
        NodeValue::DescriptionTerm => ExtractedNode::DescriptionTerm,
        NodeValue::DescriptionDetails => ExtractedNode::DescriptionDetails,
        NodeValue::ShortCode(s) => ExtractedNode::ShortCode {
            emoji: s.emoji.clone(),
        },
        NodeValue::Escaped => ExtractedNode::Escaped,
        NodeValue::EscapedTag(_) => ExtractedNode::EscapedTag,
        NodeValue::SpoileredText => ExtractedNode::SpoileredText,
        NodeValue::Subtext => ExtractedNode::Subtext,
        NodeValue::Raw(r) => ExtractedNode::Raw(r.clone()),
    }
}

/// Emit a single AST node and all its children.
#[allow(clippy::too_many_lines, clippy::cognitive_complexity)]
fn emit_node<'a>(node: &'a AstNode<'a>, ctx: &mut EmitContext<'_>) {
    // Extract all data we need and drop the borrow immediately.
    let extracted = extract_node(node);

    match extracted {
        // ─── Front matter (already extracted, skip) / footnote def ──
        ExtractedNode::FrontMatter | ExtractedNode::FootnoteDefinition => {}

        // ─── Paragraph ───────────────────────────────────────────
        ExtractedNode::Paragraph => {
            if !ctx.in_tight_list {
                ctx.newline();
            }
            emit_children(node, ctx);
            if !ctx.in_tight_list {
                ctx.newline();
            }
        }

        // ─── Heading ─────────────────────────────────────────────
        ExtractedNode::Heading { level } => {
            ctx.newline();
            for _ in 0..level {
                ctx.push("=");
            }
            ctx.push(" ");
            emit_children(node, ctx);
            ctx.newline();
        }

        // ─── Thematic break (horizontal rule) ────────────────────
        ExtractedNode::ThematicBreak => {
            ctx.newline();
            ctx.push("#line(length: 100%)\n");
        }

        // ─── Text ────────────────────────────────────────────────
        ExtractedNode::Text(text) => {
            let escaped = escape_typst_content(&text);
            ctx.push(&escaped);
        }

        // ─── Soft break ──────────────────────────────────────────
        ExtractedNode::SoftBreak => {
            ctx.push("\n");
        }

        // ─── Hard break ──────────────────────────────────────────
        ExtractedNode::LineBreak => {
            ctx.push(" \\\n");
        }

        // ─── Strong (bold) ───────────────────────────────────────
        ExtractedNode::Strong => {
            ctx.push("*");
            emit_children(node, ctx);
            ctx.push("*");
        }

        // ─── Emphasis (italic) ───────────────────────────────────
        ExtractedNode::Emph => {
            ctx.push("_");
            emit_children(node, ctx);
            ctx.push("_");
        }

        // ─── Strikethrough / spoilered text ────────────────────────
        ExtractedNode::Strikethrough | ExtractedNode::SpoileredText => {
            ctx.push("#strike[");
            emit_children(node, ctx);
            ctx.push("]");
        }

        // ─── Underline ───────────────────────────────────────────
        ExtractedNode::Underline => {
            ctx.push("#underline[");
            emit_children(node, ctx);
            ctx.push("]");
        }

        // ─── Superscript ─────────────────────────────────────────
        ExtractedNode::Superscript => {
            ctx.push("#super[");
            emit_children(node, ctx);
            ctx.push("]");
        }

        // ─── Subscript / subtext ───────────────────────────────────
        ExtractedNode::Subscript | ExtractedNode::Subtext => {
            ctx.push("#sub[");
            emit_children(node, ctx);
            ctx.push("]");
        }

        // ─── Highlight / mark ────────────────────────────────────
        ExtractedNode::Highlight => {
            ctx.push("#highlight[");
            emit_children(node, ctx);
            ctx.push("]");
        }

        // ─── Inline code ─────────────────────────────────────────
        ExtractedNode::Code {
            literal,
            num_backticks,
        } => {
            let ticks_count = num_backticks.max(1);
            let ticks: String = "`".repeat(ticks_count);
            if literal.starts_with('`') || literal.ends_with('`') {
                let _ = write!(ctx.out, "{ticks} {literal} {ticks}");
            } else {
                let _ = write!(ctx.out, "{ticks}{literal}{ticks}");
            }
        }

        // ─── Link / wikilink ────────────────────────────────────────
        ExtractedNode::Link { url } | ExtractedNode::WikiLink { url } => {
            let _ = write!(ctx.out, "#link(\"{}\")[", escape_typst_string(&url));
            emit_children(node, ctx);
            ctx.push("]");
        }

        // ─── Image ───────────────────────────────────────────────
        ExtractedNode::Image { url } => {
            // Skip remote images (already warned by check_content)
            if url.starts_with("http://") || url.starts_with("https://") {
                return;
            }

            // In WASM there's no filesystem — emit a placeholder instead of
            // a broken #image() that would crash Typst compilation.
            #[cfg(target_arch = "wasm32")]
            {
                let mut alt_text = String::new();
                collect_text(node, &mut alt_text);
                let label = if alt_text.is_empty() {
                    escape_typst_content(&url)
                } else {
                    escape_typst_content(&alt_text)
                };
                ctx.push("\n#align(center)[\n");
                ctx.push("#block(width: 80%, inset: 12pt, stroke: 0.5pt + luma(180), radius: 4pt)[\n");
                let _ = writeln!(ctx.out, "#align(center)[#text(size: 0.85em, fill: luma(120))[\\[image: {label}\\]]]");
                ctx.push("]\n]\n");
            }

            #[cfg(not(target_arch = "wasm32"))]
            {
                ctx.push("\n#figure(\n");
                let _ = writeln!(
                    ctx.out,
                    "  image(\"{}\", width: 100%),",
                    escape_typst_string(&url)
                );

                // Collect alt text from children
                let mut alt_text = String::new();
                collect_text(node, &mut alt_text);
                if !alt_text.is_empty() {
                    let _ =
                        writeln!(ctx.out, "  caption: [{}],", escape_typst_content(&alt_text));
                }

                ctx.push(")\n");
            }
        }

        // ─── Block quote ─────────────────────────────────────────
        ExtractedNode::BlockQuote | ExtractedNode::MultilineBlockQuote => {
            ctx.newline();
            ctx.push("#quote(block: true)[\n");
            emit_children(node, ctx);
            ctx.push("]\n");
        }

        // ─── List ────────────────────────────────────────────────
        ExtractedNode::List {
            is_ordered,
            tight,
            start,
        } => {
            let prev_tight = ctx.in_tight_list;
            ctx.in_tight_list = tight;

            ctx.newline();
            if is_ordered && start > 1 {
                let _ = writeln!(ctx.out, "#set enum(start: {start})");
            }

            for child in node.children() {
                let child_extracted = extract_node(child);
                match child_extracted {
                    ExtractedNode::Item => {
                        ctx.push_indent();
                        if is_ordered {
                            ctx.push("+ ");
                        } else {
                            ctx.push("- ");
                        }
                        ctx.indent += 1;
                        emit_list_item_children(child, ctx);
                        ctx.indent -= 1;
                        ctx.newline();
                    }
                    ExtractedNode::TaskItem { checked } => {
                        ctx.push_indent();
                        if checked {
                            ctx.push("#box[\\u{2611}] ");
                        } else {
                            ctx.push("#box[\\u{2610}] ");
                        }
                        ctx.indent += 1;
                        emit_list_item_children(child, ctx);
                        ctx.indent -= 1;
                        ctx.newline();
                    }
                    _ => {
                        emit_node(child, ctx);
                    }
                }
            }

            ctx.in_tight_list = prev_tight;
        }

        // ─── Transparent containers (just emit children) ──────────
        ExtractedNode::Item
        | ExtractedNode::TaskItem { .. }
        | ExtractedNode::Document
        | ExtractedNode::Escaped
        | ExtractedNode::EscapedTag => {
            emit_children(node, ctx);
        }

        // ─── Code block ──────────────────────────────────────────
        ExtractedNode::CodeBlock { info, literal } => {
            let lang = info.split([' ', ',', '\t']).next().unwrap_or("");

            if lang == "mermaid" {
                // Emit image reference — SVG will be rendered before compilation
                let idx = ctx.mermaid_counter;
                ctx.mermaid_counter += 1;
                ctx.mermaid_sources.push(literal.clone());
                ctx.newline();
                let vpath = super::mermaid::MERMAID_VPATH_PREFIX;
                let _ = writeln!(ctx.out, "#align(center)[#image(\"{vpath}{idx}.svg\")]");
            } else {
                // Use enough backticks to avoid collision with content
                let fence = backtick_fence(&literal);

                ctx.newline();
                if lang.is_empty() {
                    let _ = writeln!(ctx.out, "{fence}");
                } else {
                    let _ = writeln!(ctx.out, "{fence}{lang}");
                }

                let content = literal.strip_suffix('\n').unwrap_or(&literal);
                ctx.push(content);
                ctx.newline();
                let _ = writeln!(ctx.out, "{fence}");
            }
        }

        // ─── HTML block → convert to Typst ──────────────────────
        ExtractedNode::HtmlBlock { literal } => {
            let typst = super::html::emit_html_block(&literal, ctx.warnings);
            ctx.push(&typst);
        }

        // ─── HTML inline → entity decode or convert ─────────────
        ExtractedNode::HtmlInline(html_str) => {
            if html_str.starts_with('<') && html_str.len() > 1 {
                let typst = super::html::emit_html_inline(&html_str, ctx.warnings);
                ctx.push(&typst);
            } else {
                let decoded = decode_html_entity(&html_str);
                ctx.push(&decoded);
            }
        }

        // ─── Table ───────────────────────────────────────────────
        ExtractedNode::Table {
            alignments,
            num_columns,
        } => {
            ctx.table_alignments.clone_from(&alignments);
            ctx.newline();

            // Detect empty header rows (GFM requires headers, but they may be blank)
            let empty_header = has_empty_header(node);

            let align_strs: Vec<&str> = alignments
                .iter()
                .map(|a| match a {
                    TableAlignment::Left => "left",
                    TableAlignment::Center => "center",
                    TableAlignment::Right => "right",
                    TableAlignment::None => "auto",
                })
                .collect();
            let align_list = align_strs.join(", ");

            if empty_header {
                // Wrap in a code scope that resets header styling so the first
                // data row isn't accidentally styled as a header.
                ctx.push("#{\n");
                ctx.push("show table.cell.where(y: 0): set text(weight: 400)\n");
                ctx.push("show table.cell.where(y: 0): set table.cell(fill: none, stroke: none)\n");
                // Inside #{ }, no # prefix needed
                let _ = writeln!(
                    ctx.out,
                    "table(\n  columns: {num_columns},\n  align: ({align_list},),"
                );
            } else {
                let _ = writeln!(
                    ctx.out,
                    "#table(\n  columns: {num_columns},\n  align: ({align_list},),"
                );
            }

            for child in node.children() {
                // Skip the empty header row entirely
                if empty_header {
                    let data = child.data.borrow();
                    if matches!(&data.value, NodeValue::TableRow(true)) {
                        continue;
                    }
                }
                emit_node(child, ctx);
            }

            ctx.push(")\n");
            if empty_header {
                ctx.push("}\n");
            }
            ctx.table_alignments.clear();
        }

        // ─── Table row ───────────────────────────────────────────
        ExtractedNode::TableRow { is_header } => {
            ctx.in_table_header = is_header;
            ctx.table_cell_index = 0;
            for child in node.children() {
                emit_node(child, ctx);
            }
        }

        // ─── Table cell ──────────────────────────────────────────
        ExtractedNode::TableCell => {
            ctx.push("  [");
            emit_children(node, ctx);
            ctx.push("],\n");
            ctx.table_cell_index += 1;
        }

        // ─── Footnote reference ──────────────────────────────────
        ExtractedNode::FootnoteReference { name } => {
            if let Some(content) = ctx.footnotes.get(name.as_str()).cloned() {
                let _ = write!(ctx.out, "#footnote[{}]", content.trim());
            } else {
                let _ = write!(ctx.out, "#super[{}]", escape_typst_content(&name));
                ctx.warnings.push(SilkprintWarning::FootnoteNotFound {
                    name: name.clone(),
                });
            }
        }

        // ─── Math ────────────────────────────────────────────────
        ExtractedNode::Math { literal, display } => {
            let trimmed = literal.trim();
            if display {
                let _ = write!(ctx.out, "$ {trimmed} $");
            } else {
                let _ = write!(ctx.out, "${trimmed}$");
            }
        }

        // ─── Alert (GitHub-style) ────────────────────────────────
        ExtractedNode::Alert { title, icon } => {
            ctx.newline();
            ctx.push("#block(\n");
            ctx.push("  stroke: (left: 3pt + rgb(\"#4a5dbd\")),\n");
            ctx.push("  radius: (right: 4pt),\n");
            ctx.push("  inset: 12pt,\n");
            ctx.push("  width: 100%,\n");
            ctx.push(")[\n");
            let _ = writeln!(ctx.out, "  *{icon} {title}* \\");
            emit_children(node, ctx);
            ctx.push("]\n");
        }

        // ─── Description list ────────────────────────────────────
        ExtractedNode::DescriptionList => {
            ctx.newline();
            for child in node.children() {
                emit_node(child, ctx);
            }
        }

        // ─── Description item ────────────────────────────────────
        ExtractedNode::DescriptionItem => {
            for child in node.children() {
                emit_node(child, ctx);
            }
        }

        // ─── Description term ────────────────────────────────────
        ExtractedNode::DescriptionTerm => {
            ctx.push("\n/ ");
            emit_children(node, ctx);
        }

        // ─── Description details ─────────────────────────────────
        ExtractedNode::DescriptionDetails => {
            ctx.push(": ");
            emit_children(node, ctx);
            ctx.newline();
        }

        // ─── Emoji shortcode (resolved to unicode by comrak) ─────
        ExtractedNode::ShortCode { emoji } => {
            ctx.push(&emoji);
        }

        // ─── Raw output node (programmatic only) ─────────────────
        ExtractedNode::Raw(raw_str) => {
            ctx.push(&raw_str);
        }
    }
}

/// Emit all children of a node, with inline HTML accumulation.
///
/// When an opening `HtmlInline` tag is encountered, adjacent `HtmlInline` and
/// `Text` siblings are accumulated until the tag stack balances. The combined
/// HTML fragment is then passed to `html::emit_html_inline` for conversion.
fn emit_children<'a>(node: &'a AstNode<'a>, ctx: &mut EmitContext<'_>) {
    let children: Vec<_> = node.children().collect();
    let mut i = 0;

    while i < children.len() {
        let extracted = extract_node(children[i]);

        // Check if this is an opening HTML inline tag (not a self-closing or entity)
        if let ExtractedNode::HtmlInline(ref html_str) = extracted {
            if is_opening_html_tag(html_str) {
                // Accumulate siblings until tags balance
                let mut buf = html_str.clone();
                let mut depth: usize = 1;
                i += 1;

                while i < children.len() && depth > 0 {
                    match extract_node(children[i]) {
                        ExtractedNode::HtmlInline(ref s) => {
                            buf.push_str(s);
                            if is_opening_html_tag(s) {
                                depth += 1;
                            } else if is_closing_html_tag(s) {
                                depth = depth.saturating_sub(1);
                            }
                            // Self-closing tags don't change depth
                        }
                        ExtractedNode::Text(ref t) => {
                            buf.push_str(t);
                        }
                        _ => {
                            // Non-HTML/text node breaks accumulation
                            break;
                        }
                    }
                    i += 1;
                }

                let typst = super::html::emit_html_inline(&buf, ctx.warnings);
                ctx.push(&typst);
                continue;
            }
        }

        emit_node(children[i], ctx);
        i += 1;
    }
}

/// Check if a string looks like an opening HTML tag (e.g., `<strong>`).
fn is_opening_html_tag(s: &str) -> bool {
    s.starts_with('<')
        && !s.starts_with("</")
        && !s.ends_with("/>")
        && s.len() > 2
        && s.ends_with('>')
}

/// Check if a string looks like a closing HTML tag (e.g., `</strong>`).
fn is_closing_html_tag(s: &str) -> bool {
    s.starts_with("</") && s.ends_with('>')
}

/// Emit list item children, handling tight vs loose lists.
fn emit_list_item_children<'a>(node: &'a AstNode<'a>, ctx: &mut EmitContext<'_>) {
    for child in node.children() {
        let is_paragraph = matches!(extract_node(child), ExtractedNode::Paragraph);
        if is_paragraph && ctx.in_tight_list {
            // Tight list: unwrap paragraph, emit content inline
            emit_children(child, ctx);
        } else {
            emit_node(child, ctx);
        }
    }
}

// ═══════════════════════════════════════════════════════════════════
// Footnote collection
// ═══════════════════════════════════════════════════════════════════

/// First-pass: walk the tree and collect footnote definitions.
///
/// Returns a map from footnote name to the Typst-rendered content of
/// the definition body. These are inlined at `#footnote[...]` reference sites.
fn collect_footnote_definitions<'a>(
    root: &'a AstNode<'a>,
    warnings: &mut WarningCollector,
) -> HashMap<String, String> {
    let mut map = HashMap::new();

    for node in root.descendants() {
        let name = {
            let data = node.data.borrow();
            if let NodeValue::FootnoteDefinition(def) = &data.value {
                Some(def.name.clone())
            } else {
                None
            }
        };

        if let Some(name) = name {
            // Render the footnote body into a standalone Typst fragment.
            // Pass already-collected footnotes so nested refs can resolve.
            let mut fn_ctx = EmitContext {
                out: String::new(),
                indent: 0,
                footnotes: map.clone(),
                table_alignments: Vec::new(),
                table_cell_index: 0,
                in_table_header: false,
                in_tight_list: false,
                warnings,
                mermaid_sources: Vec::new(),
                mermaid_counter: 0,
            };
            emit_children(node, &mut fn_ctx);
            map.insert(name, fn_ctx.out);
        }
    }

    map
}

// ═══════════════════════════════════════════════════════════════════
// Helpers
// ═══════════════════════════════════════════════════════════════════

/// Check whether a table's header row has all empty cells.
///
/// GFM syntax requires a header row, but the user may leave all cells blank
/// when the table is purely data (e.g., `| | | |`). We detect this so the
/// emitter can skip the empty row and avoid an empty accent-colored bar.
fn has_empty_header<'a>(table_node: &'a AstNode<'a>) -> bool {
    let Some(first_row) = table_node.children().next() else {
        return false;
    };
    let data = first_row.data.borrow();
    if !matches!(&data.value, NodeValue::TableRow(true)) {
        return false;
    }
    drop(data);

    first_row.children().all(|cell| {
        let mut text = String::new();
        collect_text(cell, &mut text);
        text.trim().is_empty()
    })
}

/// Collect all plain text from a node's descendants into a buffer.
fn collect_text<'a>(node: &'a AstNode<'a>, buf: &mut String) {
    for child in node.descendants() {
        let data = child.data.borrow();
        if let NodeValue::Text(text) = &data.value {
            buf.push_str(text);
        }
    }
}


/// Return a backtick fence long enough to not collide with `content`.
///
/// Scans for the longest run of backticks in the content and returns one more.
fn backtick_fence(content: &str) -> String {
    let mut max_run = 0_usize;
    let mut current_run = 0_usize;
    for c in content.chars() {
        if c == '`' {
            current_run += 1;
            max_run = max_run.max(current_run);
        } else {
            current_run = 0;
        }
    }
    let count = max_run.max(2) + 1; // minimum 3 backticks
    "`".repeat(count)
}

/// Decode common HTML entities to literal characters.
fn decode_html_entity(s: &str) -> String {
    match s {
        "&amp;" => "&".to_string(),
        "&lt;" => "<".to_string(),
        "&gt;" => ">".to_string(),
        "&quot;" => "\"".to_string(),
        "&#39;" | "&apos;" => "'".to_string(),
        "&nbsp;" => "\u{00A0}".to_string(),
        "&mdash;" => "\u{2014}".to_string(),
        "&ndash;" => "\u{2013}".to_string(),
        "&hellip;" => "\u{2026}".to_string(),
        "&laquo;" => "\u{00AB}".to_string(),
        "&raquo;" => "\u{00BB}".to_string(),
        "&copy;" => "\u{00A9}".to_string(),
        "&reg;" => "\u{00AE}".to_string(),
        "&trade;" => "\u{2122}".to_string(),
        "&times;" => "\u{00D7}".to_string(),
        "&divide;" => "\u{00F7}".to_string(),
        _ => {
            // Try numeric entities: &#123; or &#x1F4A9;
            if let Some(stripped) = s.strip_prefix("&#x").and_then(|s| s.strip_suffix(';')) {
                if let Ok(code) = u32::from_str_radix(stripped, 16) {
                    if let Some(c) = char::from_u32(code) {
                        return c.to_string();
                    }
                }
            }
            if let Some(stripped) = s.strip_prefix("&#").and_then(|s| s.strip_suffix(';')) {
                if let Ok(code) = stripped.parse::<u32>() {
                    if let Some(c) = char::from_u32(code) {
                        return c.to_string();
                    }
                }
            }
            escape_typst_content(s)
        }
    }
}

/// Get icon and color field name for an alert type.
fn alert_icon_and_color(alert_type: comrak::nodes::AlertType) -> (&'static str, &'static str) {
    use comrak::nodes::AlertType;
    match alert_type {
        AlertType::Note => ("\u{2139}\u{FE0F}", "note_color"),
        AlertType::Tip => ("\u{1F4A1}", "tip_color"),
        AlertType::Important => ("\u{2757}", "important_color"),
        AlertType::Warning => ("\u{26A0}\u{FE0F}", "warning_color"),
        AlertType::Caution => ("\u{1F6D1}", "caution_color"),
    }
}

/// Inspect a parsed AST for unusual content patterns and emit relevant warnings.
///
/// Returns `true` if the document parsed cleanly with no warnings.
pub fn check_content<'a>(root: &'a AstNode<'a>, warnings: &mut WarningCollector) -> bool {
    let initial_count = warnings.warnings().len();

    for node in root.descendants() {
        let data = node.data.borrow();
        match &data.value {
            NodeValue::CodeBlock(code_block) => {
                check_code_block_language(&code_block.info, warnings);
            }
            NodeValue::Image(link) => {
                check_image_url(&link.url, warnings);
            }
            _ => {}
        }
    }

    warnings.warnings().len() == initial_count
}

/// Well-known code fence language identifiers that `syntect`/Typst can highlight.
const KNOWN_LANGUAGES: &[&str] = &[
    "bash",
    "c",
    "clojure",
    "cpp",
    "c++",
    "csharp",
    "c#",
    "cs",
    "css",
    "dart",
    "diff",
    "dockerfile",
    "elixir",
    "elm",
    "erlang",
    "go",
    "graphql",
    "haskell",
    "html",
    "java",
    "javascript",
    "js",
    "json",
    "jsonc",
    "jsx",
    "julia",
    "kotlin",
    "latex",
    "tex",
    "lua",
    "makefile",
    "markdown",
    "md",
    "nix",
    "objc",
    "objective-c",
    "ocaml",
    "perl",
    "php",
    "plain",
    "text",
    "txt",
    "powershell",
    "python",
    "py",
    "r",
    "ruby",
    "rb",
    "rust",
    "rs",
    "scala",
    "scss",
    "sh",
    "shell",
    "sql",
    "swift",
    "toml",
    "ts",
    "tsx",
    "typescript",
    "typst",
    "vim",
    "xml",
    "yaml",
    "yml",
    "zig",
    "zsh",
    // Diagram languages (handled specially, not syntax-highlighted)
    "mermaid",
];

/// Warn if a code block specifies an unrecognized language identifier.
fn check_code_block_language(info: &str, warnings: &mut WarningCollector) {
    let lang = info.split([' ', ',', '\t']).next().unwrap_or("");
    if lang.is_empty() {
        return;
    }

    let lower = lang.to_lowercase();
    if !KNOWN_LANGUAGES.contains(&lower.as_str()) {
        warnings.push(SilkprintWarning::UnknownLanguage {
            lang: lang.to_string(),
        });
    }
}

/// Warn if an image references a remote URL (not supported in v0.1).
fn check_image_url(url: &str, warnings: &mut WarningCollector) {
    if url.starts_with("http://") || url.starts_with("https://") {
        warnings.push(SilkprintWarning::RemoteImageSkipped {
            url: url.to_string(),
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn options_enables_all_extensions() {
        let opts = comrak_options();
        assert!(opts.extension.strikethrough);
        assert!(opts.extension.table);
        assert!(opts.extension.autolink);
        assert!(opts.extension.tasklist);
        assert!(opts.extension.superscript);
        assert!(opts.extension.subscript);
        assert!(opts.extension.footnotes);
        assert!(opts.extension.description_lists);
        assert!(opts.extension.highlight);
        assert!(opts.extension.underline);
        assert!(opts.extension.math_dollars);
        assert!(opts.extension.alerts);
        assert!(opts.extension.shortcodes);
        assert!(opts.extension.wikilinks_title_after_pipe);
        assert_eq!(
            opts.extension.front_matter_delimiter,
            Some("---".to_owned())
        );
    }

    #[test]
    fn parse_produces_ast() {
        let arena = comrak::Arena::new();
        let root = parse(&arena, "# Hello\n\nWorld");
        let children: Vec<_> = root.children().collect();
        assert!(children.len() >= 2, "expected heading + paragraph");
    }

    #[test]
    fn check_content_warns_remote_image() {
        let arena = comrak::Arena::new();
        let root = parse(&arena, "![alt](https://example.com/img.png)");
        let mut warnings = WarningCollector::new();
        let clean = check_content(root, &mut warnings);
        assert!(!clean);
        assert_eq!(warnings.warnings().len(), 1);
    }

    #[test]
    fn check_content_warns_unknown_language() {
        let arena = comrak::Arena::new();
        let root = parse(&arena, "```qwxyz\ncode\n```");
        let mut warnings = WarningCollector::new();
        let clean = check_content(root, &mut warnings);
        assert!(!clean);
        assert_eq!(warnings.warnings().len(), 1);
    }

    #[test]
    fn check_content_accepts_known_language() {
        let arena = comrak::Arena::new();
        let root = parse(&arena, "```rust\nfn main() {}\n```");
        let mut warnings = WarningCollector::new();
        let clean = check_content(root, &mut warnings);
        assert!(clean);
        assert!(warnings.is_empty());
    }

    #[test]
    fn check_content_ignores_empty_language() {
        let arena = comrak::Arena::new();
        let root = parse(&arena, "```\nplain code\n```");
        let mut warnings = WarningCollector::new();
        let clean = check_content(root, &mut warnings);
        assert!(clean);
    }

    // ─── Emitter tests ──────────────────────────────────────────

    fn test_theme() -> ResolvedTheme {
        ResolvedTheme {
            tokens: crate::theme::tokens::ThemeTokens::default(),
            tmtheme_xml: String::new(),
        }
    }

    fn emit(markdown: &str) -> String {
        let arena = comrak::Arena::new();
        let root = parse(&arena, markdown);
        let theme = test_theme();
        let mut warnings = WarningCollector::new();
        emit_typst(root, &theme, &mut warnings).0
    }

    #[test]
    fn emit_heading() {
        let result = emit("# Hello World");
        assert!(result.contains("= Hello World"));
    }

    #[test]
    fn emit_h2() {
        let result = emit("## Section");
        assert!(result.contains("== Section"));
    }

    #[test]
    fn emit_bold() {
        let result = emit("**bold text**");
        assert!(result.contains("*bold text*"));
    }

    #[test]
    fn emit_italic() {
        let result = emit("*italic text*");
        assert!(result.contains("_italic text_"));
    }

    #[test]
    fn emit_strikethrough() {
        let result = emit("~~struck~~");
        assert!(result.contains("#strike[struck]"));
    }

    #[test]
    fn emit_inline_code() {
        let result = emit("`code`");
        assert!(result.contains("`code`"));
    }

    #[test]
    fn emit_link() {
        let result = emit("[Click](https://example.com)");
        assert!(result.contains("#link(\"https://example.com\")[Click]"));
    }

    #[test]
    fn emit_code_block() {
        let result = emit("```rust\nfn main() {}\n```");
        assert!(result.contains("```rust"));
        assert!(result.contains("fn main() {}"));
    }

    #[test]
    fn emit_blockquote() {
        let result = emit("> A quote");
        assert!(result.contains("#quote(block: true)"));
    }

    #[test]
    fn emit_horizontal_rule() {
        let result = emit("---");
        assert!(result.contains("#line(length: 100%)"));
    }

    #[test]
    fn emit_unordered_list() {
        let result = emit("- item one\n- item two");
        assert!(result.contains("- item one"));
        assert!(result.contains("- item two"));
    }

    #[test]
    fn emit_ordered_list() {
        let result = emit("1. first\n2. second");
        assert!(result.contains("+ first"));
        assert!(result.contains("+ second"));
    }

    #[test]
    fn emit_inline_math() {
        let result = emit("$x^2$");
        assert!(result.contains("$x^2$"));
    }

    #[test]
    fn emit_display_math() {
        let result = emit("$$\nE = mc^2\n$$");
        assert!(result.contains("$ E = mc^2 $"));
    }

    #[test]
    fn emit_table() {
        let result = emit("| A | B |\n|---|---|\n| 1 | 2 |");
        assert!(result.contains("#table("));
        assert!(result.contains("columns: 2"));
    }

    #[test]
    fn emit_escapes_special_chars() {
        let escaped = escape_typst_content("# Hello *world* _foo_");
        assert_eq!(escaped, "\\# Hello \\*world\\* \\_foo\\_");
    }

    #[test]
    fn emit_hard_break() {
        let result = emit("line one  \nline two");
        assert!(result.contains("\\\n"));
    }
}
