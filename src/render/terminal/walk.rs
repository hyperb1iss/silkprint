//! Lower a comrak AST into a [`RenderedDoc`].
//!
//! This is the terminal counterpart to the Typst emitter in
//! [`crate::render::markdown`]. It walks the same AST but builds a
//! width-independent block model whose inline runs carry semantic roles. The
//! two walkers are kept honest by parity fixtures rather than a shared trait;
//! see the terminal-reader plan §2.1.

use std::collections::HashMap;

use comrak::nodes::{AstNode, ListType, NodeValue, TableAlignment};

use crate::render::origin::DocumentOrigin;
use crate::warnings::{SilkprintWarning, WarningCollector};

use super::highlight::highlight_block;
use super::model::{
    AlertKind, Align, Block, DescriptionItem, ItemMarker, LinkTarget, ListBlock, ListItem, Mods,
    OutlineItem, RenderedDoc, Role, Span,
};

/// Walk the AST into a `RenderedDoc`.
pub fn walk<'a>(root: &'a AstNode<'a>, warnings: &mut WarningCollector) -> RenderedDoc {
    walk_with_origin(root, warnings, None)
}

/// Walk the AST into a `RenderedDoc`, resolving relative remote references.
pub fn walk_with_origin<'a>(
    root: &'a AstNode<'a>,
    warnings: &mut WarningCollector,
    origin: Option<&DocumentOrigin>,
) -> RenderedDoc {
    let footnotes = collect_footnotes(root, warnings, origin);
    let mut walker = Walker {
        warnings,
        footnotes,
        footnote_order: Vec::new(),
        doc: RenderedDoc::default(),
        origin,
        _marker: std::marker::PhantomData,
    };

    let mut blocks = walker.block_children(root);
    walker.append_footnotes(&mut blocks);

    let mut doc = walker.doc;
    doc.title = first_heading_text(&blocks);
    doc.outline = build_outline(&blocks);
    doc.blocks = blocks;
    doc.origin = origin.cloned();
    doc
}

struct Walker<'a, 'w> {
    warnings: &'w mut WarningCollector,
    footnotes: HashMap<String, Vec<Block>>,
    footnote_order: Vec<String>,
    doc: RenderedDoc,
    origin: Option<&'w DocumentOrigin>,
    _marker: std::marker::PhantomData<&'a ()>,
}

impl<'a> Walker<'a, '_> {
    // ─── Block level ─────────────────────────────────────────────

    fn block_children(&mut self, node: &'a AstNode<'a>) -> Vec<Block> {
        let mut out = Vec::new();
        for child in node.children() {
            self.block_node(child, &mut out);
        }
        out
    }

    fn block_node(&mut self, node: &'a AstNode<'a>, out: &mut Vec<Block>) {
        let value = node.data.borrow().value.clone();
        match value {
            NodeValue::Document | NodeValue::Item(_) | NodeValue::TaskItem(_) => {
                out.extend(self.block_children(node));
            }
            NodeValue::FrontMatter(_) | NodeValue::FootnoteDefinition(_) => {}

            NodeValue::Paragraph => self.paragraph(node, out),

            NodeValue::Heading(h) => {
                let mut spans = self.inline_children(node);
                // Plain text in a heading takes the heading color; inline code
                // and links keep their own roles.
                for span in &mut spans {
                    if span.role == Role::Body {
                        span.role = Role::Heading(h.level);
                    }
                }
                let title = spans_to_text(&spans);
                let anchor = slug(&title);
                out.push(Block::Heading {
                    level: h.level,
                    spans,
                    anchor,
                });
            }

            NodeValue::ThematicBreak => out.push(Block::Rule),

            NodeValue::CodeBlock(cb) => {
                let lang_token = cb.info.split([' ', ',', '\t']).next().unwrap_or("");
                if lang_token == "math" {
                    out.push(Block::Math {
                        source: cb.literal.trim().to_string(),
                        display: true,
                    });
                } else if lang_token == "csv"
                    && let Some(rows) = crate::render::csv::parse_rows(&cb.literal)
                {
                    out.push(csv_table_block(rows));
                } else {
                    let lang = (!lang_token.is_empty()).then(|| lang_token.to_string());
                    let lines = highlight_block(&cb.literal, lang.as_deref());
                    out.push(Block::CodeBlock { lang, lines });
                }
            }

            NodeValue::BlockQuote | NodeValue::MultilineBlockQuote(_) => {
                let inner = self.block_children(node);
                out.push(Block::Quote(inner));
            }

            NodeValue::List(list) => {
                out.push(self.list_block(node, list.list_type, list.tight, list.start));
            }

            NodeValue::Table(t) => out.push(self.table_block(node, &t.alignments)),

            NodeValue::Alert(a) => {
                let kind = alert_kind(a.alert_type);
                let title = a
                    .title
                    .clone()
                    .unwrap_or_else(|| a.alert_type.default_title().to_string());
                let body = self.block_children(node);
                out.push(Block::Alert { kind, title, body });
            }

            NodeValue::DescriptionList => out.push(self.description_list(node)),

            NodeValue::Math(m) => out.push(Block::Math {
                source: m.literal.trim().to_string(),
                display: m.display_math,
            }),

            NodeValue::HtmlBlock(html) => {
                out.extend(super::html::to_blocks_with_origin(
                    &html.literal,
                    &mut self.doc.links,
                    self.origin,
                ));
            }

            // Anything else at block position is treated as inline content.
            _ => {
                let spans = self.inline_children(node);
                if !spans.is_empty() {
                    out.push(Block::Paragraph(spans));
                }
            }
        }
    }

    fn paragraph(&mut self, node: &'a AstNode<'a>, out: &mut Vec<Block>) {
        let children: Vec<&'a AstNode<'a>> = node.children().collect();

        // Standalone image: a paragraph whose only child is an image.
        if children.len() == 1
            && let NodeValue::Image(link) = &children[0].data.borrow().value
        {
            let mut alt = String::new();
            collect_text(children[0], &mut alt);
            out.push(Block::Image {
                src: self.resolve_reference(&link.url),
                alt,
            });
            return;
        }

        // Display math alone in a paragraph becomes a math block.
        if children.len() == 1
            && let NodeValue::Math(m) = &children[0].data.borrow().value
            && m.display_math
        {
            out.push(Block::Math {
                source: m.literal.trim().to_string(),
                display: true,
            });
            return;
        }

        if let Some(lines) = field_stack(&children) {
            let span_lines = lines
                .iter()
                .map(|line| self.inlines_of(line))
                .filter(|spans| !spans.is_empty())
                .collect::<Vec<_>>();
            if span_lines.len() >= 2 {
                out.push(Block::FieldStack(span_lines));
                return;
            }
        }

        let spans = self.inline_children(node);
        if !spans.is_empty() {
            out.push(Block::Paragraph(spans));
        }
    }

    fn list_block(
        &mut self,
        node: &'a AstNode<'a>,
        list_type: ListType,
        tight: bool,
        start: usize,
    ) -> Block {
        let ordered = list_type == ListType::Ordered;
        let mut items = Vec::new();
        let mut number = start.max(1);

        for child in node.children() {
            let value = child.data.borrow().value.clone();
            match value {
                NodeValue::Item(_) => {
                    let blocks = self.block_children(child);
                    let marker = if ordered {
                        ItemMarker::Ordered(number)
                    } else {
                        ItemMarker::Bullet
                    };
                    number += 1;
                    items.push(ListItem { marker, blocks });
                }
                NodeValue::TaskItem(task) => {
                    let blocks = self.block_children(child);
                    items.push(ListItem {
                        marker: ItemMarker::Task(task.symbol.is_some()),
                        blocks,
                    });
                }
                _ => {}
            }
        }

        Block::List(ListBlock {
            ordered,
            tight,
            items,
        })
    }

    fn table_block(&mut self, node: &'a AstNode<'a>, alignments: &[TableAlignment]) -> Block {
        let aligns = alignments.iter().map(|a| convert_align(*a)).collect();
        let mut header = Vec::new();
        let mut rows = Vec::new();

        for row in node.children() {
            let is_header = matches!(&row.data.borrow().value, NodeValue::TableRow(true));
            let cells: Vec<Vec<Span>> = row
                .children()
                .map(|cell| self.inline_children(cell))
                .collect();
            if is_header {
                if cells.iter().any(|c| !spans_to_text(c).trim().is_empty()) {
                    header = cells;
                }
            } else {
                rows.push(cells);
            }
        }

        Block::Table(super::model::TableBlock {
            aligns,
            header,
            rows,
        })
    }

    fn description_list(&mut self, node: &'a AstNode<'a>) -> Block {
        let mut items = Vec::new();
        for item in node.children() {
            if !matches!(&item.data.borrow().value, NodeValue::DescriptionItem(_)) {
                continue;
            }
            let mut term = Vec::new();
            let mut details = Vec::new();
            for part in item.children() {
                let value = part.data.borrow().value.clone();
                match value {
                    NodeValue::DescriptionTerm => term = self.inline_children(part),
                    NodeValue::DescriptionDetails => details.extend(self.block_children(part)),
                    _ => {}
                }
            }
            if !term.is_empty() {
                items.push(DescriptionItem { term, details });
            }
        }
        Block::DescriptionList(items)
    }

    // ─── Inline level ────────────────────────────────────────────

    fn inline_children(&mut self, node: &'a AstNode<'a>) -> Vec<Span> {
        let children: Vec<&'a AstNode<'a>> = node.children().collect();
        self.inlines_of(&children)
    }

    fn inlines_of(&mut self, nodes: &[&'a AstNode<'a>]) -> Vec<Span> {
        let mut out = Vec::new();
        for &child in nodes {
            self.inline_node(child, Role::Body, Mods::default(), None, &mut out);
        }
        out
    }

    fn inline_node(
        &mut self,
        node: &'a AstNode<'a>,
        role: Role,
        mods: Mods,
        link: Option<usize>,
        out: &mut Vec<Span>,
    ) {
        let value = node.data.borrow().value.clone();
        match value {
            NodeValue::Text(t) => out.push(Span {
                text: t.to_string(),
                role,
                mods,
                link,
            }),
            NodeValue::SoftBreak => out.push(Span {
                text: " ".to_string(),
                role,
                mods,
                link,
            }),
            NodeValue::LineBreak => out.push(Span {
                text: "\n".to_string(),
                role,
                mods,
                link,
            }),
            NodeValue::Strong => self.inline_kids(node, role, mods.with_bold(), link, out),
            NodeValue::Emph => self.inline_kids(node, role, mods.with_italic(), link, out),
            NodeValue::Strikethrough | NodeValue::SpoileredText => {
                self.inline_kids(node, role, mods.with_strikethrough(), link, out);
            }
            NodeValue::Underline => self.inline_kids(node, role, mods.with_underline(), link, out),
            NodeValue::Superscript
            | NodeValue::Subscript
            | NodeValue::Subtext
            | NodeValue::Escaped => {
                self.inline_kids(node, role, mods, link, out);
            }
            NodeValue::Highlight => self.inline_kids(node, Role::Highlight, mods, link, out),
            NodeValue::Code(c) => out.push(Span {
                text: c.literal,
                role: Role::InlineCode,
                mods,
                link,
            }),
            NodeValue::Link(l) => {
                let url = self.resolve_reference(&l.url);
                let id = self.doc.add_link(LinkTarget::from_url(&url));
                self.inline_kids(node, Role::Link, mods.with_underline(), Some(id), out);
            }
            NodeValue::WikiLink(w) => {
                let url = self.resolve_reference(&wikilink_target(&w.url));
                let id = self.doc.add_link(LinkTarget::from_url(&url));
                self.inline_kids(node, Role::Link, mods.with_underline(), Some(id), out);
            }
            NodeValue::Image(l) => {
                let mut alt = String::new();
                collect_text(node, &mut alt);
                let label = if alt.is_empty() { l.url.clone() } else { alt };
                out.push(Span {
                    text: format!("[{label}]"),
                    role: Role::Muted,
                    mods,
                    link,
                });
            }
            NodeValue::Math(m) => out.push(Span {
                text: m.literal.trim().to_string(),
                role: Role::Math,
                mods,
                link,
            }),
            NodeValue::FootnoteReference(f) => {
                let n = self.footnote_number(&f.name);
                out.push(Span {
                    text: format!("[{n}]"),
                    role: Role::Muted,
                    mods,
                    link,
                });
            }
            NodeValue::ShortCode(s) => out.push(Span {
                text: s.emoji,
                role,
                mods,
                link,
            }),
            NodeValue::HtmlInline(h) => {
                if let Some(text) = inline_html_text(&h) {
                    if !text.is_empty() {
                        out.push(Span {
                            text,
                            role,
                            mods,
                            link,
                        });
                    }
                } else {
                    self.warnings
                        .push(SilkprintWarning::UnsupportedHtmlTag { tag: tag_name(&h) });
                }
            }
            // Transparent / block-ish containers: descend.
            _ => self.inline_kids(node, role, mods, link, out),
        }
    }

    fn inline_kids(
        &mut self,
        node: &'a AstNode<'a>,
        role: Role,
        mods: Mods,
        link: Option<usize>,
        out: &mut Vec<Span>,
    ) {
        for child in node.children() {
            self.inline_node(child, role, mods, link, out);
        }
    }

    // ─── Footnotes ───────────────────────────────────────────────

    fn footnote_number(&mut self, name: &str) -> usize {
        if let Some(pos) = self.footnote_order.iter().position(|n| n == name) {
            return pos + 1;
        }
        if !self.footnotes.contains_key(name) {
            self.warnings.push(SilkprintWarning::FootnoteNotFound {
                name: name.to_string(),
            });
        }
        self.footnote_order.push(name.to_string());
        self.footnote_order.len()
    }

    fn append_footnotes(&mut self, blocks: &mut Vec<Block>) {
        if self.footnote_order.is_empty() {
            return;
        }
        blocks.push(Block::Rule);
        let order = std::mem::take(&mut self.footnote_order);
        for (idx, name) in order.iter().enumerate() {
            let n = idx + 1;
            let mut def = self.footnotes.get(name).cloned().unwrap_or_default();
            prefix_footnote_number(&mut def, n);
            blocks.extend(def);
        }
    }

    fn resolve_reference(&self, target: &str) -> String {
        self.origin.map_or_else(
            || target.to_string(),
            |origin| origin.resolve_reference(target),
        )
    }
}

// ─── Free helpers ────────────────────────────────────────────────

fn collect_footnotes<'a>(
    root: &'a AstNode<'a>,
    warnings: &mut WarningCollector,
    origin: Option<&DocumentOrigin>,
) -> HashMap<String, Vec<Block>> {
    let mut map = HashMap::new();
    for node in root.descendants() {
        let name = match &node.data.borrow().value {
            NodeValue::FootnoteDefinition(def) => def.name.clone(),
            _ => continue,
        };
        let mut sub = Walker {
            warnings,
            footnotes: HashMap::new(),
            footnote_order: Vec::new(),
            doc: RenderedDoc::default(),
            origin,
            _marker: std::marker::PhantomData,
        };
        let blocks = sub.block_children(node);
        map.insert(name, blocks);
    }
    map
}

fn prefix_footnote_number(blocks: &mut [Block], n: usize) {
    if let Some(Block::Paragraph(spans)) = blocks.first_mut() {
        spans.insert(
            0,
            Span::new(format!("[{n}] "), Role::Muted, Mods::default()),
        );
    }
}

fn first_heading_text(blocks: &[Block]) -> Option<String> {
    blocks.iter().find_map(|b| match b {
        Block::Heading {
            level: 1, spans, ..
        } => Some(spans_to_text(spans)),
        _ => None,
    })
}

fn build_outline(blocks: &[Block]) -> Vec<OutlineItem> {
    blocks
        .iter()
        .enumerate()
        .filter_map(|(idx, b)| match b {
            Block::Heading {
                level,
                spans,
                anchor,
            } => Some(OutlineItem {
                level: *level,
                title: spans_to_text(spans),
                anchor: anchor.clone(),
                block_index: idx,
            }),
            _ => None,
        })
        .collect()
}

fn spans_to_text(spans: &[Span]) -> String {
    spans.iter().map(|s| s.text.as_str()).collect()
}

/// GitHub-style heading slug.
fn slug(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    let mut prev_dash = false;
    for ch in text.chars() {
        if ch.is_alphanumeric() {
            out.extend(ch.to_lowercase());
            prev_dash = false;
        } else if matches!(ch, ' ' | '-' | '_') && !prev_dash && !out.is_empty() {
            out.push('-');
            prev_dash = true;
        }
    }
    while out.ends_with('-') {
        out.pop();
    }
    out
}

fn convert_align(a: TableAlignment) -> Align {
    match a {
        TableAlignment::Left => Align::Left,
        TableAlignment::Center => Align::Center,
        TableAlignment::Right => Align::Right,
        TableAlignment::None => Align::None,
    }
}

fn csv_table_block(rows: Vec<Vec<String>>) -> Block {
    let columns = rows.iter().map(Vec::len).max().unwrap_or(0);
    let mut rows = rows.into_iter().map(|row| pad_row(row, columns));
    let header = rows
        .next()
        .unwrap_or_default()
        .into_iter()
        .map(|cell| vec![Span::body(cell)])
        .collect();
    let rows = rows
        .map(|row| row.into_iter().map(|cell| vec![Span::body(cell)]).collect())
        .collect();

    Block::Table(super::model::TableBlock {
        aligns: vec![Align::None; columns],
        header,
        rows,
    })
}

fn pad_row(mut row: Vec<String>, columns: usize) -> Vec<String> {
    row.resize(columns, String::new());
    row
}

fn wikilink_target(target: &str) -> String {
    let (path, anchor) = target
        .split_once('#')
        .map_or((target, None), |(path, anchor)| (path, Some(anchor)));
    if path.is_empty()
        || uri_scheme(path).is_some()
        || std::path::Path::new(path).extension().is_some()
    {
        return target.to_string();
    }
    anchor.map_or_else(
        || format!("{path}.md"),
        |anchor| format!("{path}.md#{anchor}"),
    )
}

fn uri_scheme(value: &str) -> Option<&str> {
    let (scheme, _rest) = value.split_once(':')?;
    let mut chars = scheme.chars();
    let first = chars.next()?;
    (first.is_ascii_alphabetic()
        && chars.all(|ch| ch.is_ascii_alphanumeric() || matches!(ch, '+' | '-' | '.')))
    .then_some(scheme)
}

fn alert_kind(t: comrak::nodes::AlertType) -> AlertKind {
    use comrak::nodes::AlertType;
    match t {
        AlertType::Note => AlertKind::Note,
        AlertType::Tip => AlertKind::Tip,
        AlertType::Important => AlertKind::Important,
        AlertType::Warning => AlertKind::Warning,
        AlertType::Caution => AlertKind::Caution,
    }
}

fn collect_text<'a>(node: &'a AstNode<'a>, buf: &mut String) {
    for child in node.descendants() {
        if let NodeValue::Text(t) = &child.data.borrow().value {
            buf.push_str(t);
        }
    }
}

/// Render a small set of inline HTML tags to plain text; `None` means the tag
/// is unsupported (the caller warns).
fn inline_html_text(html: &str) -> Option<String> {
    let trimmed = html.trim();
    match trimmed.to_ascii_lowercase().as_str() {
        "<br>" | "<br/>" | "<br />" => Some("\n".to_string()),
        _ if !trimmed.starts_with('<') => Some(decode_entities(trimmed)),
        // Opening/closing tags for inline emphasis we can safely drop.
        "<b>" | "</b>" | "<strong>" | "</strong>" | "<i>" | "</i>" | "<em>" | "</em>" | "<u>"
        | "</u>" | "<code>" | "</code>" | "<span>" | "</span>" | "<sub>" | "</sub>" | "<sup>"
        | "</sup>" => Some(String::new()),
        _ => None,
    }
}

fn tag_name(html: &str) -> String {
    html.trim_start_matches(['<', '/'])
        .split(['>', ' '])
        .next()
        .unwrap_or(html)
        .to_string()
}

fn decode_entities(s: &str) -> String {
    s.replace("&amp;", "&")
        .replace("&lt;", "<")
        .replace("&gt;", ">")
        .replace("&quot;", "\"")
        .replace("&#39;", "'")
        .replace("&nbsp;", "\u{00a0}")
}

// ─── Field-stack detection (port of the Typst emitter heuristic) ─────

fn field_stack<'a>(children: &[&'a AstNode<'a>]) -> Option<Vec<Vec<&'a AstNode<'a>>>> {
    let mut lines: Vec<Vec<&'a AstNode<'a>>> = Vec::new();
    let mut current: Vec<&'a AstNode<'a>> = Vec::new();
    let mut saw_break = false;

    for &child in children {
        if matches!(&child.data.borrow().value, NodeValue::SoftBreak) {
            saw_break = true;
            lines.push(std::mem::take(&mut current));
        } else {
            current.push(child);
        }
    }
    lines.push(current);

    if !saw_break {
        return None;
    }

    let nonblank: Vec<Vec<&'a AstNode<'a>>> =
        lines.into_iter().filter(|l| !line_is_blank(l)).collect();
    let first = nonblank.first()?;
    if !line_starts_with_label(first) {
        return None;
    }
    let labels = nonblank
        .iter()
        .filter(|l| line_starts_with_label(l))
        .count();
    if labels < 2 {
        return None;
    }
    Some(nonblank)
}

fn line_is_blank<'a>(line: &[&'a AstNode<'a>]) -> bool {
    line.iter().all(|n| match &n.data.borrow().value {
        NodeValue::Text(t) => t.trim().is_empty(),
        _ => false,
    })
}

fn line_starts_with_label<'a>(line: &[&'a AstNode<'a>]) -> bool {
    let Some(first) = line.iter().find(|n| match &n.data.borrow().value {
        NodeValue::Text(t) => !t.trim().is_empty(),
        _ => true,
    }) else {
        return false;
    };
    if !matches!(&first.data.borrow().value, NodeValue::Strong) {
        return false;
    }
    let mut label = String::new();
    collect_text(first, &mut label);
    let label = label.trim();
    label.ends_with(':') && label.len() <= 40
}

impl LinkTarget {
    fn from_url(url: &str) -> Self {
        if let Some(anchor) = url.strip_prefix('#') {
            LinkTarget::Anchor(anchor.to_string())
        } else {
            LinkTarget::Url(url.to_string())
        }
    }
}

#[cfg(test)]
mod tests {
    use comrak::Arena;
    use url::Url;

    use super::*;

    #[test]
    fn remote_origin_resolves_images_and_links() {
        let arena = Arena::new();
        let root = crate::render::markdown::parse(
            &arena,
            "![logo](assets/logo.png)\n\n[guide](docs/guide.md#intro)\n\n[in-page](#usage)",
        );
        let origin = DocumentOrigin::remote(
            Url::parse("https://raw.githubusercontent.com/o/r/HEAD/README.md").expect("url"),
        );
        let mut warnings = WarningCollector::new();

        let doc = walk_with_origin(root, &mut warnings, Some(&origin));

        assert!(matches!(
            &doc.blocks[0],
            Block::Image { src, .. }
                if src == "https://raw.githubusercontent.com/o/r/HEAD/assets/logo.png"
        ));
        assert!(matches!(
            &doc.links[0],
            LinkTarget::Url(url)
                if url == "https://raw.githubusercontent.com/o/r/HEAD/docs/guide.md#intro"
        ));
        assert!(matches!(&doc.links[1], LinkTarget::Anchor(anchor) if anchor == "usage"));
        assert_eq!(doc.origin, Some(origin));
    }

    #[test]
    fn wikilinks_resolve_to_markdown_targets() {
        let arena = Arena::new();
        let root = crate::render::markdown::parse(&arena, "[[Guide]] and [[Docs/Intro#top]]");
        let mut warnings = WarningCollector::new();

        let doc = walk(root, &mut warnings);

        assert!(matches!(&doc.links[0], LinkTarget::Url(url) if url == "Guide.md"));
        assert!(matches!(&doc.links[1], LinkTarget::Url(url) if url == "Docs/Intro.md#top"));
    }

    #[test]
    fn csv_fences_lower_to_tables() {
        let arena = Arena::new();
        let root =
            crate::render::markdown::parse(&arena, "```csv\nname,count\nalpha,1\nbeta,2\n```\n");
        let mut warnings = WarningCollector::new();

        let doc = walk(root, &mut warnings);

        let Some(Block::Table(table)) = doc.blocks.first() else {
            panic!("expected csv table: {:?}", doc.blocks);
        };
        assert_eq!(table.header[0][0].text, "name");
        assert_eq!(table.rows[1][1][0].text, "2");
    }
}
