//! Lightweight HTML → [`Block`] conversion for the terminal walker.
//!
//! Many real-world READMEs wrap their header, badges, and navigation in raw
//! HTML (`<div align=center>`, `<img>`, `<a>`, `<picture>`). comrak hands these
//! through as `HtmlBlock`/`HtmlInline` nodes; here we walk the fragment DOM with
//! scraper and produce the same block model as the markdown path so the content
//! actually renders instead of vanishing. Alignment and styling attributes are
//! ignored — content, links, and structure are what matter in a reader.

use ego_tree::NodeRef;
use scraper::Html;
use scraper::node::{Element, Node};

use super::model::{
    Align, Block, ItemMarker, LinkTarget, ListBlock, ListItem, Mods, Role, Span, TableBlock,
};

/// Convert an HTML fragment into blocks, registering link targets.
pub fn to_blocks(html: &str, links: &mut Vec<LinkTarget>) -> Vec<Block> {
    let fragment = Html::parse_fragment(html);
    let mut builder = Builder {
        blocks: Vec::new(),
        inline: Vec::new(),
        links,
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
        let target = url.strip_prefix('#').map_or_else(
            || LinkTarget::Url(url.to_string()),
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
                let label = el
                    .attr("alt")
                    .filter(|a| !a.is_empty())
                    .or_else(|| el.attr("src"))
                    .unwrap_or("image");
                self.inline.push(Span {
                    text: format!("[{label}]"),
                    role: Role::Muted,
                    mods,
                    link,
                });
            }
            "h1" | "h2" | "h3" | "h4" | "h5" | "h6" => {
                self.flush();
                let level = el.name()[1..].parse::<u8>().unwrap_or(1);
                let mut sub = Builder {
                    blocks: Vec::new(),
                    inline: Vec::new(),
                    links: self.links,
                };
                sub.children(node, mods, Role::Heading(level), link);
                let spans = sub.inline;
                let anchor = slug(&spans_text(&spans));
                self.blocks.push(Block::Heading {
                    level,
                    spans,
                    anchor,
                });
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
                };
                sub.children(node, mods, Role::Quote, link);
                sub.flush();
                self.blocks.push(Block::Quote(sub.blocks));
            }
            // Block containers: flush surrounding inline, recurse, flush again.
            "p" | "div" | "section" | "article" | "header" | "footer" | "main" | "center"
            | "figure" | "figcaption" | "nav" | "details" | "summary" | "aside" => {
                self.flush();
                self.children(node, mods, role, link);
                self.flush();
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
    fn renders_image_and_link_from_div() {
        let mut links = Vec::new();
        let blocks = to_blocks(
            r#"<div align="center"><img src="logo.png" alt="Logo"><p>A <a href="https://x.io">link</a> here</p></div>"#,
            &mut links,
        );
        let text: String = blocks
            .iter()
            .flat_map(|b| match b {
                Block::Paragraph(spans) => spans.iter().map(|s| s.text.clone()).collect::<Vec<_>>(),
                _ => Vec::new(),
            })
            .collect();
        assert!(text.contains("[Logo]"), "image alt should render: {text}");
        assert!(text.contains("link"), "link text should render: {text}");
        assert_eq!(links.len(), 1, "one link registered");
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
            .find_map(|b| if let Block::Table(t) = b { Some(t) } else { None })
            .expect("a table block");
        assert_eq!(table.header.len(), 2, "two header cells");
        assert_eq!(table.rows.len(), 1, "one data row");
        assert_eq!(table.rows[0].len(), 2, "two cells in the row");
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
