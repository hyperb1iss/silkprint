//! Render a [`RenderedDoc`] to styled ANSI for one-shot output.
//!
//! Produces width-bounded lines with SGR colors (degrading truecolor → 256 →
//! 16 → none), OSC 8 hyperlinks, and the active glyph tier's markers. Block
//! renderers return `Vec<String>` of styled lines so containers (quotes,
//! alerts, lists) can re-prefix them with gutters at a reduced width.

use std::fmt::Write as _;

use crate::theme::ResolvedTheme;

use super::caps::{Capabilities, ColorTier};
use super::glyphs::Glyphs;
use super::layout::{display_width, truncate, wrap_spans};
use super::model::{
    AlertKind, Block, ItemMarker, LinkTarget, ListItem, Mods, RenderedDoc, Rgb, Role, Span,
    TableBlock,
};
use super::style::{ContentStyleResolver, Style, parse_hex};

const MARGIN: &str = "  ";
const MAX_CONTENT_WIDTH: usize = 100;
const RESET: &str = "\x1b[0m";

/// Render a document to a styled ANSI string (terminating newline included).
pub fn render(
    doc: &RenderedDoc,
    theme: &ResolvedTheme,
    caps: &Capabilities,
    glyphs: Glyphs,
) -> String {
    render_with_offsets(doc, theme, caps, glyphs).0
}

/// Render, also returning each top-level block's `(start_line, line_count)`.
///
/// The spans (one per `doc.blocks` entry) let the TUI map outline headings to
/// scroll positions and size graphical bands to the full block they cover.
pub fn render_with_offsets(
    doc: &RenderedDoc,
    theme: &ResolvedTheme,
    caps: &Capabilities,
    glyphs: Glyphs,
) -> (String, Vec<(usize, usize)>) {
    let renderer = Renderer {
        resolver: ContentStyleResolver::new(theme),
        theme,
        caps: *caps,
        glyphs,
        links: &doc.links,
    };
    let width = renderer.content_width();
    let mut out = String::new();
    let mut spans = vec![(0usize, 0usize); doc.blocks.len()];
    let mut line_no = 0usize;
    let mut first = true;
    for (idx, block) in doc.blocks.iter().enumerate() {
        let lines = renderer.block_lines(block, width);
        if lines.is_empty() {
            spans[idx] = (line_no, 0);
            continue;
        }
        if !first {
            out.push('\n');
            line_no += 1;
        }
        first = false;
        spans[idx] = (line_no, lines.len());
        for line in lines {
            out.push_str(MARGIN);
            out.push_str(&line);
            out.push('\n');
            line_no += 1;
        }
    }
    (out, spans)
}

pub(super) struct Renderer<'a> {
    resolver: ContentStyleResolver<'a>,
    theme: &'a ResolvedTheme,
    caps: Capabilities,
    glyphs: Glyphs,
    links: &'a [LinkTarget],
}

impl Renderer<'_> {
    pub(super) fn glyphs(&self) -> Glyphs {
        self.glyphs
    }

    pub(super) fn theme(&self) -> &ResolvedTheme {
        self.theme
    }

    fn content_width(&self) -> usize {
        let term = usize::from(self.caps.width);
        term.saturating_sub(MARGIN.len())
            .clamp(20, MAX_CONTENT_WIDTH)
    }

    // ─── Block dispatch ──────────────────────────────────────────

    fn blocks_lines(&self, blocks: &[Block], width: usize) -> Vec<String> {
        let mut out = Vec::new();
        let mut first = true;
        for block in blocks {
            let lines = self.block_lines(block, width);
            if lines.is_empty() {
                continue;
            }
            if !first {
                out.push(String::new());
            }
            first = false;
            out.extend(lines);
        }
        out
    }

    #[allow(clippy::too_many_lines)]
    fn block_lines(&self, block: &Block, width: usize) -> Vec<String> {
        match block {
            Block::Heading { level, spans, .. } => self.heading(*level, spans, width),
            Block::Paragraph(spans) => self.wrap_render(spans, width),
            Block::CodeBlock { lang, lines } => self.code_block(lang.as_deref(), lines, width),
            Block::Quote(inner) => self.quote(inner, width),
            Block::List(list) => self.list(&list.items, list.tight, width),
            Block::Table(table) => self.table(table, width),
            Block::Alert { kind, title, body } => self.alert(*kind, title, body, width),
            Block::Image { src, alt } => self.image(src, alt),
            Block::Rule => vec![self.rule(width)],
            Block::Math { source, display } => self.math(source, *display, width),
            Block::DescriptionList(items) => self.description_list(items, width),
            Block::FieldStack(lines) => self.field_stack(lines, width),
        }
    }

    // ─── Inline rendering ────────────────────────────────────────

    /// Wrap spans to `width` and render each visual line to a styled string.
    fn wrap_render(&self, spans: &[Span], width: usize) -> Vec<String> {
        wrap_spans(spans, width)
            .iter()
            .map(|line| self.inline_line(line))
            .collect()
    }

    pub(super) fn inline_line(&self, spans: &[Span]) -> String {
        let mut out = String::new();
        for span in spans {
            let style = self.resolver.resolve(span.role, span.mods);
            let painted = self.paint(&span.text, style);
            match span.link.and_then(|id| self.links.get(id)) {
                Some(LinkTarget::Url(url)) if self.caps.is_tty => {
                    let safe = super::layout::sanitize(url);
                    let _ = write!(out, "\x1b]8;;{safe}\x1b\\{painted}\x1b]8;;\x1b\\");
                }
                _ => out.push_str(&painted),
            }
        }
        out
    }

    // ─── Block renderers ─────────────────────────────────────────

    fn heading(&self, level: u8, spans: &[Span], width: usize) -> Vec<String> {
        let mut lines = self.wrap_render(spans, width);
        let color = self
            .resolver
            .resolve(Role::Heading(level), Mods::default())
            .fg;
        match level {
            1 => lines.push(self.colored_rule(width, color, false)),
            2 => lines.push(self.colored_rule(width.min(48), color, true)),
            _ => {}
        }
        lines
    }

    fn code_block(&self, lang: Option<&str>, lines: &[Vec<Span>], width: usize) -> Vec<String> {
        let accent = parse_hex(&self.theme.tokens.code_block.left_accent_color)
            .or_else(|| parse_hex(&self.theme.tokens.links.color));
        let bar = self.paint(
            "\u{2503}",
            Style {
                fg: accent,
                ..Style::default()
            },
        );

        let mut out = Vec::new();
        if let Some(lang) = lang.filter(|l| !l.is_empty()) {
            let icon = self.glyphs.language(lang);
            let label = if icon.is_empty() {
                lang.to_string()
            } else {
                format!("{icon} {lang}")
            };
            out.push(self.paint(
                &label,
                Style {
                    fg: accent,
                    dim: true,
                    ..Style::default()
                },
            ));
        }
        let body_width = width.saturating_sub(2);
        for code_line in lines {
            let rendered = self.inline_line(&clamp_spans(code_line, body_width));
            out.push(format!("{bar} {rendered}"));
        }
        out
    }

    fn quote(&self, inner: &[Block], width: usize) -> Vec<String> {
        let color = parse_hex(&self.theme.tokens.blockquote.border_color)
            .or_else(|| parse_hex(&self.theme.tokens.links.color));
        let bar = self.paint(
            self.glyphs.quote_bar(),
            Style {
                fg: color,
                ..Style::default()
            },
        );
        let gutter_w = display_width(self.glyphs.quote_bar()) + 1;
        self.blocks_lines(inner, width.saturating_sub(gutter_w))
            .into_iter()
            .map(|line| format!("{bar} {line}"))
            .collect()
    }

    fn list(&self, items: &[ListItem], tight: bool, width: usize) -> Vec<String> {
        let mut out = Vec::new();
        for (idx, item) in items.iter().enumerate() {
            if !tight && idx > 0 {
                out.push(String::new());
            }
            let (marker, marker_w) = self.list_marker(item.marker);
            let indent = " ".repeat(display_width_plain(&marker));
            let inner = self.blocks_lines(&item.blocks, width.saturating_sub(marker_w));
            for (i, line) in inner.into_iter().enumerate() {
                if i == 0 {
                    out.push(format!("{marker}{line}"));
                } else {
                    out.push(format!("{indent}{line}"));
                }
            }
        }
        out
    }

    fn list_marker(&self, marker: ItemMarker) -> (String, usize) {
        match marker {
            ItemMarker::Bullet => {
                let color = parse_hex(&self.theme.tokens.list.bullet_color);
                let m = self.paint(
                    self.glyphs.bullet(),
                    Style {
                        fg: color,
                        ..Style::default()
                    },
                );
                (format!("{m} "), 2)
            }
            ItemMarker::Ordered(n) => {
                let color = parse_hex(&self.theme.tokens.list.bullet_color);
                let text = format!("{n}.");
                let w = text.len() + 1;
                let m = self.paint(
                    &text,
                    Style {
                        fg: color,
                        bold: true,
                        ..Style::default()
                    },
                );
                (format!("{m} "), w)
            }
            ItemMarker::Task(checked) => {
                let color = parse_hex(if checked {
                    &self.theme.tokens.list.task_checked_color
                } else {
                    &self.theme.tokens.list.task_unchecked_color
                });
                let glyph = self.glyphs.task(checked);
                let m = self.paint(
                    glyph,
                    Style {
                        fg: color,
                        ..Style::default()
                    },
                );
                (format!("{m} "), display_width(glyph) + 1)
            }
        }
    }

    fn alert(&self, kind: AlertKind, title: &str, body: &[Block], width: usize) -> Vec<String> {
        let color = self.alert_color(kind);
        let bar = self.paint(
            "\u{2503}",
            Style {
                fg: color,
                ..Style::default()
            },
        );
        let label = Glyphs::alert_label(kind);
        let icon = self.glyphs.alert(kind);
        let heading = self.paint(
            &format!("{icon} {label}"),
            Style {
                fg: color,
                bold: true,
                ..Style::default()
            },
        );
        let title_extra = if title.eq_ignore_ascii_case(label) {
            String::new()
        } else {
            format!("  {}", super::layout::sanitize(title))
        };

        let mut out = vec![format!("{bar} {heading}{title_extra}")];
        let inner = self.blocks_lines(body, width.saturating_sub(2));
        for line in inner {
            out.push(format!("{bar} {line}"));
        }
        out
    }

    fn alert_color(&self, kind: AlertKind) -> Option<Rgb> {
        let a = &self.theme.tokens.alerts;
        let key = match kind {
            AlertKind::Note => &a.note_color,
            AlertKind::Tip => &a.tip_color,
            AlertKind::Important => &a.important_color,
            AlertKind::Warning => &a.warning_color,
            AlertKind::Caution => &a.caution_color,
        };
        parse_hex(key).or_else(|| parse_hex(&self.theme.tokens.links.color))
    }

    fn image(&self, src: &str, alt: &str) -> Vec<String> {
        let label = if alt.is_empty() { src } else { alt };
        let text = format!("{} [image: {label}]", self.glyphs.link());
        let painted = self.paint(
            text.trim(),
            Style {
                dim: true,
                fg: self.resolver.body_color(),
                ..Style::default()
            },
        );
        if self.caps.is_tty && !src.is_empty() {
            let safe_src = super::layout::sanitize(src);
            vec![format!("\x1b]8;;{safe_src}\x1b\\{painted}\x1b]8;;\x1b\\")]
        } else {
            vec![painted]
        }
    }

    fn rule(&self, width: usize) -> String {
        let color = parse_hex(&self.theme.tokens.horizontal_rule.color);
        self.colored_rule(width, color, true)
    }

    fn colored_rule(&self, width: usize, color: Option<Rgb>, dim: bool) -> String {
        let glyph = self.glyphs.rule();
        let line = glyph.repeat(width.max(1));
        self.paint(
            &line,
            Style {
                fg: color,
                dim,
                ..Style::default()
            },
        )
    }

    fn math(&self, source: &str, display: bool, width: usize) -> Vec<String> {
        let span = Span::new(source, Role::Math, Mods::default());
        let mut lines = self.wrap_render(&[span], width);
        if display {
            // Indent display math a touch to set it apart.
            lines = lines.into_iter().map(|l| format!("  {l}")).collect();
        }
        lines
    }

    fn description_list(
        &self,
        items: &[super::model::DescriptionItem],
        width: usize,
    ) -> Vec<String> {
        let mut out = Vec::new();
        for (idx, item) in items.iter().enumerate() {
            if idx > 0 {
                out.push(String::new());
            }
            let term_spans: Vec<Span> = item
                .term
                .iter()
                .map(|s| Span {
                    mods: s.mods.with_bold(),
                    role: Role::Heading(6),
                    text: s.text.clone(),
                    link: s.link,
                })
                .collect();
            out.extend(self.wrap_render(&term_spans, width));
            let details = self.blocks_lines(&item.details, width.saturating_sub(2));
            for line in details {
                out.push(format!("  {line}"));
            }
        }
        out
    }

    fn field_stack(&self, lines: &[Vec<Span>], width: usize) -> Vec<String> {
        let mut out = Vec::new();
        for line in lines {
            out.extend(self.wrap_render(line, width));
        }
        out
    }

    // ─── SGR painting ────────────────────────────────────────────

    pub(super) fn paint(&self, text: &str, style: Style) -> String {
        let text = super::layout::sanitize(text);
        let open = self.sgr(style);
        if open.is_empty() {
            return text.into_owned();
        }
        format!("{open}{text}{RESET}")
    }

    fn sgr(&self, style: Style) -> String {
        let mut params: Vec<String> = Vec::new();
        if style.bold {
            params.push("1".into());
        }
        if style.dim {
            params.push("2".into());
        }
        if style.italic {
            params.push("3".into());
        }
        if style.underline {
            params.push("4".into());
        }
        if style.strikethrough {
            params.push("9".into());
        }
        if self.caps.color != ColorTier::None {
            if let Some(rgb) = style.fg {
                params.push(self.fg_code(rgb));
            }
            if let Some(rgb) = style.bg {
                params.push(self.bg_code(rgb));
            }
        }
        if params.is_empty() {
            String::new()
        } else {
            format!("\x1b[{}m", params.join(";"))
        }
    }

    fn fg_code(&self, rgb: Rgb) -> String {
        match self.caps.color {
            ColorTier::TrueColor => format!("38;2;{};{};{}", rgb.0, rgb.1, rgb.2),
            ColorTier::Ansi256 => format!("38;5;{}", rgb_to_256(rgb)),
            ColorTier::Ansi16 => format!("38;5;{}", rgb_to_16(rgb)),
            ColorTier::None => String::new(),
        }
    }

    fn bg_code(&self, rgb: Rgb) -> String {
        match self.caps.color {
            ColorTier::TrueColor => format!("48;2;{};{};{}", rgb.0, rgb.1, rgb.2),
            ColorTier::Ansi256 => format!("48;5;{}", rgb_to_256(rgb)),
            ColorTier::Ansi16 => format!("48;5;{}", rgb_to_16(rgb)),
            ColorTier::None => String::new(),
        }
    }

    fn table(&self, table: &TableBlock, width: usize) -> Vec<String> {
        super::table::render(self, table, width)
    }
}

/// Plain (unstyled) display width of a string already containing SGR codes is
/// ambiguous; list markers are short and ASCII-ish, so measure the visible glyph.
fn display_width_plain(marker: &str) -> usize {
    // Strip SGR/OSC sequences to measure visible width.
    let visible = strip_ansi(marker);
    display_width(&visible)
}

fn strip_ansi(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    let mut chars = s.chars().peekable();
    while let Some(c) = chars.next() {
        if c == '\x1b' {
            // Skip until a letter (CSI) or ST for OSC.
            for n in chars.by_ref() {
                if n.is_ascii_alphabetic() || n == '\\' {
                    break;
                }
            }
        } else {
            out.push(c);
        }
    }
    out
}

/// Limit a line of spans to `max` display cells (used for code lines).
pub(super) fn clamp_spans(spans: &[Span], max: usize) -> Vec<Span> {
    let total: usize = spans.iter().map(|s| display_width(&s.text)).sum();
    if total <= max {
        return spans.to_vec();
    }
    let mut out = Vec::new();
    let mut used = 0;
    for span in spans {
        let w = display_width(&span.text);
        if used + w <= max {
            out.push(span.clone());
            used += w;
        } else {
            let remaining = max.saturating_sub(used);
            out.push(Span {
                text: truncate(&span.text, remaining, "\u{2026}"),
                role: span.role,
                mods: span.mods,
                link: span.link,
            });
            break;
        }
    }
    out
}

// ─── Color quantization for lower tiers ──────────────────────────

fn rgb_to_256(rgb: Rgb) -> u8 {
    let Rgb(r, g, b) = rgb;
    // Grayscale ramp when the channels are close.
    if r.abs_diff(g) < 8 && g.abs_diff(b) < 8 && r.abs_diff(b) < 8 {
        let level = (u16::from(r) + u16::from(g) + u16::from(b)) / 3;
        if level < 8 {
            return 16;
        }
        if level > 248 {
            return 231;
        }
        return u8::try_from(232 + (level - 8) * 24 / 247).unwrap_or(255);
    }
    let q = |c: u8| u16::from(c) * 5 / 255;
    u8::try_from(16 + 36 * q(r) + 6 * q(g) + q(b)).unwrap_or(231)
}

fn rgb_to_16(rgb: Rgb) -> u8 {
    let Rgb(r, g, b) = rgb;
    let bright = u16::from(r) + u16::from(g) + u16::from(b) > 384;
    let bit = |c: u8| u8::from(c > 110);
    let base = bit(r) | (bit(g) << 1) | (bit(b) << 2);
    if bright { base + 8 } else { base }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::render::terminal::caps::{ColorChoice, GlyphTier, GraphicsProtocol};

    fn caps(color: ColorTier) -> Capabilities {
        Capabilities {
            color,
            glyphs: GlyphTier::Unicode,
            graphics: GraphicsProtocol::None,
            width: 80,
            height: 24,
            is_tty: false,
            in_tmux: false,
        }
    }

    #[test]
    fn no_color_tier_emits_plain_text() {
        let theme = ResolvedTheme {
            tokens: crate::theme::tokens::ThemeTokens::default(),
            tmtheme_xml: String::new(),
        };
        let r = Renderer {
            resolver: ContentStyleResolver::new(&theme),
            theme: &theme,
            caps: caps(ColorTier::None),
            glyphs: Glyphs::new(GlyphTier::Ascii),
            links: &[],
        };
        let out = r.paint(
            "hello",
            Style {
                fg: Some(Rgb(255, 0, 0)),
                ..Style::default()
            },
        );
        assert_eq!(out, "hello");
        let _ = ColorChoice::Auto;
    }

    #[test]
    fn truecolor_emits_sgr() {
        let theme = ResolvedTheme {
            tokens: crate::theme::tokens::ThemeTokens::default(),
            tmtheme_xml: String::new(),
        };
        let r = Renderer {
            resolver: ContentStyleResolver::new(&theme),
            theme: &theme,
            caps: caps(ColorTier::TrueColor),
            glyphs: Glyphs::new(GlyphTier::Unicode),
            links: &[],
        };
        let out = r.paint(
            "x",
            Style {
                fg: Some(Rgb(225, 53, 255)),
                ..Style::default()
            },
        );
        assert_eq!(out, "\x1b[38;2;225;53;255mx\x1b[0m");
    }

    #[test]
    fn rgb_to_256_grayscale_and_cube() {
        assert_eq!(rgb_to_256(Rgb(0, 0, 0)), 16);
        assert_eq!(rgb_to_256(Rgb(255, 255, 255)), 231);
        // A saturated color maps into the cube, not grayscale.
        assert!(rgb_to_256(Rgb(255, 0, 0)) >= 16);
    }
}
