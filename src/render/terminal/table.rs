//! Box-drawn table rendering for the one-shot terminal renderer.
//!
//! Column widths are derived from cell content, capped, and shrunk to fit the
//! available width. Cells preserve inline styling and are truncated with an
//! ellipsis when a column is narrowed.

use super::ansi::Renderer;
use super::caps::GlyphTier;
use super::layout::display_width;
use super::model::{Align, Span, TableBlock};
use super::style::{Style, parse_hex};

const MAX_COL: usize = 40;
const MIN_COL: usize = 3;

struct BorderChars {
    h: &'static str,
    v: &'static str,
    tl: &'static str,
    tm: &'static str,
    tr: &'static str,
    ml: &'static str,
    mm: &'static str,
    mr: &'static str,
    bl: &'static str,
    bm: &'static str,
    br: &'static str,
}

fn border_chars(tier: GlyphTier) -> BorderChars {
    if tier == GlyphTier::Ascii {
        BorderChars {
            h: "-",
            v: "|",
            tl: "+",
            tm: "+",
            tr: "+",
            ml: "+",
            mm: "+",
            mr: "+",
            bl: "+",
            bm: "+",
            br: "+",
        }
    } else {
        BorderChars {
            h: "\u{2500}",
            v: "\u{2502}",
            tl: "\u{250c}",
            tm: "\u{252c}",
            tr: "\u{2510}",
            ml: "\u{251c}",
            mm: "\u{253c}",
            mr: "\u{2524}",
            bl: "\u{2514}",
            bm: "\u{2534}",
            br: "\u{2518}",
        }
    }
}

pub(super) fn render(r: &Renderer, table: &TableBlock, width: usize) -> Vec<String> {
    let ncols = table
        .aligns
        .len()
        .max(table.header.len())
        .max(table.rows.iter().map(Vec::len).max().unwrap_or(0));
    if ncols == 0 {
        return Vec::new();
    }

    let mut col_w = vec![0usize; ncols];
    let mut note = |cells: &[Vec<Span>]| {
        for (i, cell) in cells.iter().enumerate() {
            if i < ncols {
                col_w[i] = col_w[i].max(cell_width(cell).min(MAX_COL));
            }
        }
    };
    note(&table.header);
    for row in &table.rows {
        note(row);
    }
    for w in &mut col_w {
        *w = (*w).max(MIN_COL);
    }
    shrink_to_fit(&mut col_w, width);

    let chars = border_chars(r.glyphs().tier());
    let border_style = Style {
        fg: parse_hex(&r.theme().tokens.table.row_border_color)
            .or_else(|| parse_hex(&r.theme().tokens.horizontal_rule.color)),
        dim: true,
        ..Style::default()
    };

    let mut out = Vec::new();
    out.push(rule_row(r, &col_w, &chars, border_style, Pos::Top));

    let has_header = !table.header.is_empty();
    if has_header {
        out.push(data_row(
            r,
            &table.header,
            &col_w,
            &table.aligns,
            &chars,
            border_style,
            true,
        ));
        out.push(rule_row(r, &col_w, &chars, border_style, Pos::Mid));
    }
    for row in &table.rows {
        out.push(data_row(
            r,
            row,
            &col_w,
            &table.aligns,
            &chars,
            border_style,
            false,
        ));
    }
    out.push(rule_row(r, &col_w, &chars, border_style, Pos::Bottom));
    out
}

#[derive(Clone, Copy)]
enum Pos {
    Top,
    Mid,
    Bottom,
}

fn rule_row(r: &Renderer, col_w: &[usize], chars: &BorderChars, style: Style, pos: Pos) -> String {
    let (left, mid, right) = match pos {
        Pos::Top => (chars.tl, chars.tm, chars.tr),
        Pos::Mid => (chars.ml, chars.mm, chars.mr),
        Pos::Bottom => (chars.bl, chars.bm, chars.br),
    };
    let mut line = String::from(left);
    for (i, w) in col_w.iter().enumerate() {
        line.push_str(&chars.h.repeat(w + 2));
        line.push_str(if i + 1 == col_w.len() { right } else { mid });
    }
    r.paint(&line, style)
}

#[allow(clippy::too_many_arguments)]
fn data_row(
    r: &Renderer,
    cells: &[Vec<Span>],
    col_w: &[usize],
    aligns: &[Align],
    chars: &BorderChars,
    border_style: Style,
    header: bool,
) -> String {
    let v = r.paint(chars.v, border_style);
    let mut line = v.clone();
    for (i, w) in col_w.iter().enumerate() {
        let empty = Vec::new();
        let cell = cells.get(i).unwrap_or(&empty);
        let align = aligns.get(i).copied().unwrap_or(Align::None);
        let (rendered, used) = render_cell(r, cell, *w, header);
        let pad = w.saturating_sub(used);
        let (lpad, rpad) = match align {
            Align::Right => (pad, 0),
            Align::Center => (pad / 2, pad - pad / 2),
            _ => (0, pad),
        };
        line.push(' ');
        line.push_str(&" ".repeat(lpad));
        line.push_str(&rendered);
        line.push_str(&" ".repeat(rpad));
        line.push(' ');
        line.push_str(&v);
    }
    line
}

/// Render a cell to a styled string and report its visible width.
fn render_cell(r: &Renderer, cell: &[Span], col_w: usize, header: bool) -> (String, usize) {
    let styled: Vec<Span> = if header {
        cell.iter()
            .map(|s| Span {
                mods: s.mods.with_bold(),
                role: s.role,
                text: s.text.clone(),
                link: s.link,
            })
            .collect()
    } else {
        cell.to_vec()
    };
    let clamped = super::ansi::clamp_spans(&styled, col_w);
    let used = clamped
        .iter()
        .map(|s| display_width(&s.text))
        .sum::<usize>()
        .min(col_w);
    (r.inline_line(&clamped), used)
}

fn cell_width(cell: &[Span]) -> usize {
    cell.iter().map(|s| display_width(&s.text)).sum()
}

/// Shrink columns proportionally so the rendered table fits `width`.
fn shrink_to_fit(col_w: &mut [usize], width: usize) {
    let overhead = col_w.len() * 3 + 1; // "│ " per col + trailing "│"
    let budget = width.saturating_sub(overhead).max(col_w.len() * MIN_COL);
    let mut total: usize = col_w.iter().sum();
    while total > budget {
        // Shrink the widest column by one until we fit.
        let Some((idx, _)) = col_w
            .iter()
            .enumerate()
            .filter(|(_, w)| **w > MIN_COL)
            .max_by_key(|(_, w)| **w)
        else {
            break;
        };
        col_w[idx] -= 1;
        total -= 1;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn shrink_keeps_minimum() {
        let mut cols = vec![30, 30, 30];
        shrink_to_fit(&mut cols, 20);
        assert!(cols.iter().all(|&w| w >= MIN_COL));
    }

    #[test]
    fn no_shrink_when_fits() {
        let mut cols = vec![5, 5];
        shrink_to_fit(&mut cols, 80);
        assert_eq!(cols, vec![5, 5]);
    }

    #[test]
    fn cell_width_sums_spans() {
        let cell = vec![Span::body("ab"), Span::body("cd")];
        assert_eq!(cell_width(&cell), 4);
    }
}
