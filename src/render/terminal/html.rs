//! Lightweight HTML → [`Block`] conversion for the terminal walker.
//!
//! Many real-world READMEs wrap their header, badges, and navigation in raw
//! HTML (`<div align=center>`, `<img>`, `<a>`, `<picture>`). comrak hands these
//! through as `HtmlBlock`/`HtmlInline` nodes; here we walk the fragment DOM with
//! scraper and produce the same block model as the markdown path so the content
//! actually renders instead of vanishing. Center alignment (`align="center"`,
//! `<center>`, `text-align:center`) is honored via [`Block::Center`]; other
//! styling attributes are ignored — content, links, and structure are what
//! matter in a reader.

use ego_tree::NodeRef;
use scraper::Html;
use scraper::node::{Element, Node};

use crate::render::origin::DocumentOrigin;

use super::model::{
    Align, Block, ItemMarker, LinkTarget, ListBlock, ListItem, Mods, Role, Span, TableBlock,
};

/// Convert an HTML fragment into blocks, registering link targets.
pub fn to_blocks(html: &str, links: &mut Vec<LinkTarget>) -> Vec<Block> {
    to_blocks_with_origin(html, links, None)
}

/// Convert an HTML fragment into blocks, resolving remote-relative references.
pub fn to_blocks_with_origin(
    html: &str,
    links: &mut Vec<LinkTarget>,
    origin: Option<&DocumentOrigin>,
) -> Vec<Block> {
    let fragment = Html::parse_fragment(html);
    let mut builder = Builder {
        blocks: Vec::new(),
        inline: Vec::new(),
        links,
        inline_only: false,
        origin,
    };
    for child in fragment.tree.root().children() {
        builder.walk(child, Mods::default(), Role::Body, None);
    }
    builder.flush();
    builder.blocks
}

struct Builder<'a> {
    blocks: Vec<Block>,
    inline: Vec<Span>,
    links: &'a mut Vec<LinkTarget>,
    /// True inside contexts that can only hold inline content (table cells), so
    /// images stay text placeholders instead of being promoted to image blocks.
    inline_only: bool,
    origin: Option<&'a DocumentOrigin>,
}

impl Builder<'_> {
    fn flush(&mut self) {
        if self.inline.is_empty() {
            return;
        }
        let spans = std::mem::take(&mut self.inline);
        if spans.iter().any(|s| !s.text.trim().is_empty()) {
            self.blocks.push(Block::Paragraph(spans));
        }
    }

    fn add_link(&mut self, url: &str) -> usize {
        let url = self.resolve_reference(url);
        let target = url.strip_prefix('#').map_or_else(
            || LinkTarget::Url(url.clone()),
            |anchor| LinkTarget::Anchor(anchor.to_string()),
        );
        self.links.push(target);
        self.links.len() - 1
    }

    fn children(&mut self, node: NodeRef<'_, Node>, mods: Mods, role: Role, link: Option<usize>) {
        for child in node.children() {
            self.walk(child, mods, role, link);
        }
    }

    fn resolve_reference(&self, target: &str) -> String {
        self.origin.map_or_else(
            || target.to_string(),
            |origin| origin.resolve_reference(target),
        )
    }

    fn walk(&mut self, node: NodeRef<'_, Node>, mods: Mods, role: Role, link: Option<usize>) {
        match node.value() {
            Node::Text(text) => {
                let collapsed = collapse_whitespace(text);
                if !collapsed.is_empty() {
                    self.inline.push(Span {
                        text: collapsed,
                        role,
                        mods,
                        link,
                    });
                }
            }
            Node::Element(el) => self.element(node, el, mods, role, link),
            _ => self.children(node, mods, role, link),
        }
    }

    #[allow(clippy::too_many_lines)]
    fn element(
        &mut self,
        node: NodeRef<'_, Node>,
        el: &Element,
        mods: Mods,
        role: Role,
        link: Option<usize>,
    ) {
        match el.name() {
            "br" => self.inline.push(Span {
                text: "\n".to_string(),
                role,
                mods,
                link,
            }),
            "strong" | "b" => self.children(node, mods.with_bold(), role, link),
            "em" | "i" | "cite" | "dfn" => self.children(node, mods.with_italic(), role, link),
            "u" | "ins" => self.children(node, mods.with_underline(), role, link),
            "del" | "s" | "strike" => self.children(node, mods.with_strikethrough(), role, link),
            "code" | "kbd" | "samp" | "tt" => self.children(node, mods, Role::InlineCode, link),
            "mark" => self.children(node, mods, Role::Highlight, link),
            "a" => {
                let id = el.attr("href").map(|href| self.add_link(href)).or(link);
                self.children(node, mods.with_underline(), Role::Link, id);
            }
            "img" => {
                let src = el.attr("src").unwrap_or_default();
                let resolved_src = self.resolve_reference(src);
                let alt = el.attr("alt").unwrap_or_default();
                // Promote real raster images (logos, screenshots) to block-level
                // so they render as actual graphics. Decorative inline images —
                // shields.io-style SVG badges with no raster extension — stay as
                // text placeholders so a badge row doesn't explode into a stack.
                if is_raster_src(&resolved_src) && !self.inline_only {
                    self.flush();
                    self.blocks.push(Block::Image {
                        src: resolved_src,
                        alt: alt.to_string(),
                    });
                } else {
                    let label = if alt.is_empty() {
                        if src.is_empty() { "image" } else { src }
                    } else {
                        alt
                    };
                    self.inline.push(Span {
                        text: format!("[{label}]"),
                        role: Role::Muted,
                        mods,
                        link,
                    });
                }
            }
            "h1" | "h2" | "h3" | "h4" | "h5" | "h6" => {
                self.flush();
                let level = el.name()[1..].parse::<u8>().unwrap_or(1);
                let mut sub = Builder {
                    blocks: Vec::new(),
                    inline: Vec::new(),
                    links: self.links,
                    inline_only: self.inline_only,
                    origin: self.origin,
                };
                sub.children(node, mods, Role::Heading(level), link);
                let spans = sub.inline;
                let anchor = slug(&spans_text(&spans));
                let heading = Block::Heading {
                    level,
                    spans,
                    anchor,
                };
                if is_centered(el) {
                    self.blocks.push(Block::Center(vec![heading]));
                } else {
                    self.blocks.push(heading);
                }
            }
            "hr" => {
                self.flush();
                self.blocks.push(Block::Rule);
            }
            "ul" | "ol" => {
                self.flush();
                self.list(node, el.name() == "ol");
            }
            "table" => {
                self.flush();
                self.table(node);
            }
            "blockquote" => {
                self.flush();
                let mut sub = Builder {
                    blocks: Vec::new(),
                    inline: Vec::new(),
                    links: self.links,
                    inline_only: self.inline_only,
                    origin: self.origin,
                };
                sub.children(node, mods, Role::Quote, link);
                sub.flush();
                self.blocks.push(Block::Quote(sub.blocks));
            }
            // Block containers: flush surrounding inline, recurse, flush again.
            "p" | "div" | "section" | "article" | "header" | "footer" | "main" | "center"
            | "figure" | "figcaption" | "nav" | "details" | "summary" | "aside" => {
                self.flush();
                if el.name() == "center" || is_centered(el) {
                    let mut sub = Builder {
                        blocks: Vec::new(),
                        inline: Vec::new(),
                        links: self.links,
                        inline_only: self.inline_only,
                        origin: self.origin,
                    };
                    sub.children(node, mods, role, link);
                    sub.flush();
                    // Hoist block-level images out of the centered wrapper so they
                    // get a real image band (reserve_bands only scans top-level
                    // blocks); keep contiguous non-image content centered, in order.
                    let mut centered: Vec<Block> = Vec::new();
                    for block in sub.blocks {
                        if matches!(block, Block::Image { .. }) {
                            if !centered.is_empty() {
                                self.blocks
                                    .push(Block::Center(std::mem::take(&mut centered)));
                            }
                            self.blocks.push(block);
                        } else {
                            centered.push(block);
                        }
                    }
                    if !centered.is_empty() {
                        self.blocks.push(Block::Center(centered));
                    }
                } else {
                    self.children(node, mods, role, link);
                    self.flush();
                }
            }
            // Drop non-visual elements entirely.
            "script" | "style" | "head" | "title" | "noscript" => {}
            // Everything else (span, picture, source, td, th, li outside a list, …):
            // transparent — just descend.
            _ => self.children(node, mods, role, link),
        }
    }

    fn list(&mut self, node: NodeRef<'_, Node>, ordered: bool) {
        let mut items = Vec::new();
        let mut number = 1;
        for li in node.children() {
            let Node::Element(el) = li.value() else {
                continue;
            };
            if el.name() != "li" {
                continue;
            }
            let mut sub = Builder {
                blocks: Vec::new(),
                inline: Vec::new(),
                links: self.links,
                inline_only: self.inline_only,
                origin: self.origin,
            };
            sub.children(li, Mods::default(), Role::Body, None);
            sub.flush();
            if sub.blocks.is_empty() {
                continue;
            }
            let marker = if ordered {
                ItemMarker::Ordered(number)
            } else {
                ItemMarker::Bullet
            };
            number += 1;
            items.push(ListItem {
                marker,
                blocks: sub.blocks,
            });
        }
        if !items.is_empty() {
            self.blocks.push(Block::List(ListBlock {
                ordered,
                tight: true,
                items,
            }));
        }
    }

    fn table(&mut self, node: NodeRef<'_, Node>) {
        let mut header: Vec<Vec<Span>> = Vec::new();
        let mut rows: Vec<Vec<Vec<Span>>> = Vec::new();

        for row in node.descendants() {
            let Node::Element(el) = row.value() else {
                continue;
            };
            if el.name() != "tr" {
                continue;
            }
            let mut cells = Vec::new();
            let mut is_header = false;
            for cell in row.children() {
                let Node::Element(cell_el) = cell.value() else {
                    continue;
                };
                match cell_el.name() {
                    "th" => is_header = true,
                    "td" => {}
                    _ => continue,
                }
                let mut sub = Builder {
                    blocks: Vec::new(),
                    inline: Vec::new(),
                    links: self.links,
                    inline_only: true,
                    origin: self.origin,
                };
                sub.children(cell, Mods::default(), Role::Body, None);
                cells.push(sub.inline);
            }
            if cells.is_empty() {
                continue;
            }
            if is_header && header.is_empty() {
                header = cells;
            } else {
                rows.push(cells);
            }
        }

        if header.is_empty() && rows.is_empty() {
            return;
        }
        let ncols = header
            .len()
            .max(rows.iter().map(Vec::len).max().unwrap_or(0));
        self.blocks.push(Block::Table(TableBlock {
            aligns: vec![Align::None; ncols],
            header,
            rows,
        }));
    }
}

fn spans_text(spans: &[Span]) -> String {
    spans.iter().map(|s| s.text.as_str()).collect()
}

/// Whether an image `src` points at a decodable raster image, judged by file
/// extension (ignoring any query string). SVG and extension-less endpoints
/// (e.g. shields.io badges) return false and stay inline placeholders.
fn is_raster_src(src: &str) -> bool {
    let path = src.split(['?', '#']).next().unwrap_or(src);
    let ext = path.rsplit('.').next().unwrap_or("");
    matches!(
        ext.to_ascii_lowercase().as_str(),
        "png" | "jpg" | "jpeg" | "gif" | "webp" | "bmp" | "avif" | "ico"
    ) && path.contains('.')
}

/// Whether an element requests center alignment via `align="center"` or an
/// inline `text-align: center` style. (`<center>` is handled by tag name.)
fn is_centered(el: &Element) -> bool {
    if el
        .attr("align")
        .is_some_and(|a| a.eq_ignore_ascii_case("center"))
    {
        return true;
    }
    el.attr("style").is_some_and(|style| {
        style.split(';').any(|decl| {
            let mut parts = decl.splitn(2, ':');
            matches!(
                (parts.next(), parts.next()),
                (Some(prop), Some(val))
                    if prop.trim().eq_ignore_ascii_case("text-align")
                        && val.trim().eq_ignore_ascii_case("center")
            )
        })
    })
}

fn collapse_whitespace(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    let mut prev_space = false;
    for c in s.chars() {
        if c.is_whitespace() {
            if !prev_space {
                out.push(' ');
                prev_space = true;
            }
        } else {
            out.push(c);
            prev_space = false;
        }
    }
    out
}

/// GitHub-style heading slug (mirrors `walk::slug`).
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn raster_image_in_centered_div_is_hoisted_to_block() {
        let mut links = Vec::new();
        let blocks = to_blocks(
            r#"<div align="center"><img src="logo.png" alt="Logo"><p>A <a href="https://x.io">link</a> here</p></div>"#,
            &mut links,
        );
        // The raster image is hoisted out of the Center wrapper so it can band;
        // the remaining text stays centered.
        assert!(
            matches!(blocks.first(), Some(Block::Image { alt, .. }) if alt == "Logo"),
            "logo.png should become a top-level image block: {blocks:?}"
        );
        let centered_text: String = blocks
            .iter()
            .filter_map(|b| match b {
                Block::Center(inner) => Some(inner),
                _ => None,
            })
            .flatten()
            .flat_map(|b| match b {
                Block::Paragraph(spans) => spans.iter().map(|s| s.text.clone()).collect::<Vec<_>>(),
                _ => Vec::new(),
            })
            .collect();
        assert!(
            centered_text.contains("link"),
            "link text centered: {centered_text}"
        );
        assert_eq!(links.len(), 1, "one link registered");
    }

    #[test]
    fn badge_and_table_images_stay_inline_placeholders() {
        let mut links = Vec::new();
        // Extension-less shields.io badge: stays an inline placeholder.
        let badges = to_blocks(
            r#"<p><img src="https://img.shields.io/badge/Rust-2024-e135ff" alt="Rust"></p>"#,
            &mut links,
        );
        assert!(
            matches!(badges.first(), Some(Block::Paragraph(spans)) if spans.iter().any(|s| s.text.contains("[Rust]"))),
            "badge should be an inline placeholder, not an image block: {badges:?}"
        );

        // A raster image inside a table cell can't band, so it stays a
        // placeholder inside the table rather than vanishing.
        let table = to_blocks(
            r#"<table><tr><td><img src="shot.png" alt="Shot"></td></tr></table>"#,
            &mut links,
        );
        let cell = table
            .iter()
            .find_map(|b| {
                if let Block::Table(t) = b {
                    t.rows.first()
                } else {
                    None
                }
            })
            .and_then(|row| row.first())
            .expect("a table cell");
        assert!(
            cell.iter().any(|s| s.text.contains("[Shot]")),
            "table-cell image should stay a placeholder: {cell:?}"
        );
    }

    #[test]
    fn lowers_html_table() {
        let mut links = Vec::new();
        let blocks = to_blocks(
            "<table><tr><th>Feature</th><th>Status</th></tr><tr><td>Parsing</td><td>Done</td></tr></table>",
            &mut links,
        );
        let table = blocks
            .iter()
            .find_map(|b| {
                if let Block::Table(t) = b {
                    Some(t)
                } else {
                    None
                }
            })
            .expect("a table block");
        assert_eq!(table.header.len(), 2, "two header cells");
        assert_eq!(table.rows.len(), 1, "one data row");
        assert_eq!(table.rows[0].len(), 2, "two cells in the row");
    }

    #[test]
    fn centers_aligned_paragraph_and_heading() {
        let mut links = Vec::new();
        let blocks = to_blocks(
            r#"<h1 align="center">Title</h1><p align="center">tagline</p><p>normal</p>"#,
            &mut links,
        );
        // Centered heading is wrapped, normal paragraph is not.
        assert!(
            matches!(blocks.first(), Some(Block::Center(inner)) if matches!(inner.first(), Some(Block::Heading { level: 1, .. }))),
            "centered h1 should be wrapped in Center"
        );
        assert!(
            matches!(blocks.get(1), Some(Block::Center(_))),
            "centered paragraph should be wrapped in Center"
        );
        assert!(
            matches!(blocks.get(2), Some(Block::Paragraph(_))),
            "unaligned paragraph stays a plain Paragraph"
        );
    }

    #[test]
    fn centers_via_style_and_center_tag() {
        let mut links = Vec::new();
        let blocks = to_blocks(
            r#"<div style="text-align: center">styled</div><center>tagged</center>"#,
            &mut links,
        );
        assert!(matches!(blocks.first(), Some(Block::Center(_))));
        assert!(matches!(blocks.get(1), Some(Block::Center(_))));
    }

    #[test]
    fn renders_heading_and_list() {
        let mut links = Vec::new();
        let blocks = to_blocks(
            "<h2>Title</h2><ul><li>one</li><li>two</li></ul>",
            &mut links,
        );
        assert!(
            blocks
                .iter()
                .any(|b| matches!(b, Block::Heading { level: 2, .. }))
        );
        assert!(blocks.iter().any(|b| matches!(b, Block::List(_))));
    }
}
