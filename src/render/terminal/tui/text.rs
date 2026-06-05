use std::io::{self, Write as _};

use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};

use crate::render::terminal::model::{Block, Span as ModelSpan};

pub(super) fn truncate_plain(s: &str, max: usize) -> String {
    if s.chars().count() <= max {
        return s.to_string();
    }
    let mut out: String = s.chars().take(max.saturating_sub(1)).collect();
    out.push('\u{2026}');
    out
}

pub(super) fn selected_text(
    lines: &[Line<'static>],
    start: (usize, u16),
    end: (usize, u16),
) -> Option<String> {
    let ((start_line, start_col), (end_line, end_col)) = if start <= end {
        (start, end)
    } else {
        (end, start)
    };
    if start_line == end_line && start_col == end_col {
        return None;
    }
    let mut out = String::new();
    for line_idx in start_line..=end_line {
        let raw = plain_line(lines.get(line_idx)?);
        let part = if start_line == end_line {
            slice_chars(&raw, usize::from(start_col), usize::from(end_col))
        } else if line_idx == start_line {
            slice_chars(&raw, usize::from(start_col), raw.chars().count())
        } else if line_idx == end_line {
            slice_chars(&raw, 0, usize::from(end_col))
        } else {
            raw
        };
        if line_idx > start_line {
            out.push('\n');
        }
        out.push_str(&part);
    }
    (!out.is_empty()).then_some(out)
}

fn plain_line(line: &Line<'static>) -> String {
    line.spans
        .iter()
        .map(|span| span.content.as_ref())
        .collect()
}

pub(super) fn source_line_for_block(
    block: &Block,
    source: &str,
    start_line: usize,
) -> Option<usize> {
    let needle = match block {
        Block::Heading { spans, .. }
        | Block::Paragraph(spans)
        | Block::Details { summary: spans, .. } => spans_plain_text(spans),
        Block::Image { src, alt } => {
            if alt.is_empty() {
                src.clone()
            } else {
                alt.clone()
            }
        }
        Block::CodeBlock { lang, lines } => lang.clone().unwrap_or_else(|| {
            lines
                .first()
                .map_or_else(String::new, |line| spans_plain_text(line))
        }),
        Block::Math { source, .. } => source.clone(),
        Block::Table(table) => table
            .header
            .first()
            .map_or_else(String::new, |cell| spans_plain_text(cell)),
        Block::Quote(inner) | Block::Center(inner) => {
            return inner
                .first()
                .and_then(|block| source_line_for_block(block, source, start_line));
        }
        Block::Alert { title, .. } => title.clone(),
        Block::List(list) => {
            return list
                .items
                .first()
                .and_then(|item| item.blocks.first())
                .and_then(|block| source_line_for_block(block, source, start_line));
        }
        Block::DescriptionList(items) => items
            .first()
            .map_or_else(String::new, |item| spans_plain_text(&item.term)),
        Block::FieldStack(lines) => lines
            .first()
            .map_or_else(String::new, |line| spans_plain_text(line)),
        Block::Rule => String::new(),
    };
    let needle = needle.trim();
    source
        .lines()
        .enumerate()
        .skip(start_line)
        .find_map(|(idx, line)| {
            if needle.is_empty() {
                (!line.trim().is_empty()).then_some(idx)
            } else {
                (line.contains(needle)
                    || line.contains(&needle.chars().take(24).collect::<String>()))
                .then_some(idx)
            }
        })
}

fn spans_plain_text(spans: &[ModelSpan]) -> String {
    spans.iter().map(|span| span.text.as_str()).collect()
}

fn slice_chars(value: &str, start: usize, end: usize) -> String {
    value
        .chars()
        .skip(start)
        .take(end.saturating_sub(start))
        .collect()
}

pub(super) fn copy_osc52(text: &str) -> io::Result<()> {
    let encoded = base64_encode(text.as_bytes());
    let mut stdout = io::stdout();
    write!(stdout, "\x1b]52;c;{encoded}\x07")?;
    stdout.flush()
}

pub(super) fn base64_encode(bytes: &[u8]) -> String {
    const TABLE: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut out = String::with_capacity(bytes.len().div_ceil(3) * 4);
    for chunk in bytes.chunks(3) {
        let b0 = chunk[0];
        let b1 = *chunk.get(1).unwrap_or(&0);
        let b2 = *chunk.get(2).unwrap_or(&0);
        out.push(char::from(TABLE[usize::from(b0 >> 2)]));
        out.push(char::from(
            TABLE[usize::from(((b0 & 0b0000_0011) << 4) | (b1 >> 4))],
        ));
        if chunk.len() > 1 {
            out.push(char::from(
                TABLE[usize::from(((b1 & 0b0000_1111) << 2) | (b2 >> 6))],
            ));
        } else {
            out.push('=');
        }
        if chunk.len() > 2 {
            out.push(char::from(TABLE[usize::from(b2 & 0b0011_1111)]));
        } else {
            out.push('=');
        }
    }
    out
}

pub(super) fn code_lines_source(lines: &[Vec<ModelSpan>]) -> String {
    lines
        .iter()
        .map(|spans| spans.iter().map(|s| s.text.as_str()).collect::<String>())
        .collect::<Vec<_>>()
        .join("\n")
}

/// Style applied to search matches: electric yellow on black.
pub(super) fn search_highlight_style() -> Style {
    Style::default()
        .bg(Color::Rgb(241, 250, 140))
        .fg(Color::Black)
        .add_modifier(Modifier::BOLD)
}

pub(super) fn highlight_line(line: &Line<'static>, needle: &[char], hl: Style) -> Line<'static> {
    let cells: Vec<(char, Style)> = line
        .spans
        .iter()
        .flat_map(|span| span.content.chars().map(move |ch| (ch, span.style)))
        .collect();
    if cells.len() < needle.len() {
        return line.clone();
    }

    let lower: Vec<char> = cells
        .iter()
        .map(|(c, _)| c.to_lowercase().next().unwrap_or(*c))
        .collect();
    let mut marks = vec![false; cells.len()];
    let nlen = needle.len();
    let mut i = 0;
    while i + nlen <= lower.len() {
        if lower[i..i + nlen] == *needle {
            for mark in &mut marks[i..i + nlen] {
                *mark = true;
            }
            i += nlen;
        } else {
            i += 1;
        }
    }
    if !marks.iter().any(|m| *m) {
        return line.clone();
    }

    let mut spans: Vec<Span<'static>> = Vec::new();
    let mut buf = String::new();
    let mut current: Option<(Style, bool)> = None;
    for (idx, (ch, style)) in cells.iter().enumerate() {
        let key = (*style, marks[idx]);
        if current != Some(key) {
            if let Some((st, marked)) = current.take() {
                spans.push(Span::styled(
                    std::mem::take(&mut buf),
                    if marked { st.patch(hl) } else { st },
                ));
            }
            current = Some(key);
        }
        buf.push(*ch);
    }
    if let Some((st, marked)) = current {
        spans.push(Span::styled(buf, if marked { st.patch(hl) } else { st }));
    }
    Line::from(spans)
}
