//! Width-aware text layout helpers.
//!
//! Wave 0 needs span-preserving word wrapping for the one-shot renderer. The
//! richer per-(width, caps, theme) `LaidOutDoc` with screen-coordinate hit
//! regions belongs to the TUI wave and is layered on top of these primitives.

use std::borrow::Cow;

use unicode_width::{UnicodeWidthChar, UnicodeWidthStr};

use super::model::Span;

/// Display width of a string in terminal cells.
pub fn display_width(s: &str) -> usize {
    UnicodeWidthStr::width(s)
}

/// Strip terminal control characters from untrusted content.
///
/// A markdown reader renders untrusted files, so any `ESC`/C0/C1/DEL bytes
/// embedded in the source (which could smuggle escape sequences like OSC 52
/// clipboard writes straight to the terminal) must be removed before the text
/// is interpolated into our own ANSI output. Tab is preserved.
pub fn sanitize(s: &str) -> Cow<'_, str> {
    if s.chars().any(is_unsafe_control) {
        Cow::Owned(s.chars().filter(|c| !is_unsafe_control(*c)).collect())
    } else {
        Cow::Borrowed(s)
    }
}

fn is_unsafe_control(c: char) -> bool {
    c.is_control() && c != '\t'
}

/// Truncate a string to at most `max` display cells, appending `ellipsis` if it
/// was shortened. Width-aware (handles wide glyphs).
pub fn truncate(s: &str, max: usize, ellipsis: &str) -> String {
    if display_width(s) <= max {
        return s.to_string();
    }
    let ell_w = display_width(ellipsis);
    let budget = max.saturating_sub(ell_w);
    let mut out = String::new();
    let mut w = 0;
    for ch in s.chars() {
        let cw = ch.width().unwrap_or(0);
        if w + cw > budget {
            break;
        }
        out.push(ch);
        w += cw;
    }
    out.push_str(ellipsis);
    out
}

/// A unit of wrappable content. A word may be composed of pieces from several
/// adjacent spans (e.g. emphasized `italic` directly followed by a plain `,`),
/// which is exactly why wrapping operates on a token stream rather than
/// splitting each span independently — that preserved the original spacing.
enum Token {
    Word(Vec<Span>),
    Space,
    HardBreak,
}

/// Wrap a run of styled spans into visual lines no wider than `width`.
///
/// Whitespace runs collapse to single spaces; a literal `\n` is a hard break.
/// Words with no separating whitespace stay glued together even across span
/// boundaries, so punctuation never drifts away from the word it follows. Words
/// wider than `width` overflow onto their own line rather than splitting.
pub fn wrap_spans(spans: &[Span], width: usize) -> Vec<Vec<Span>> {
    let width = width.max(1);
    let tokens = tokenize(spans);

    let mut lines: Vec<Vec<Span>> = Vec::new();
    let mut line: Vec<Span> = Vec::new();
    let mut line_w = 0usize;
    let mut pending_space = false;

    for token in tokens {
        match token {
            Token::HardBreak => {
                lines.push(std::mem::take(&mut line));
                line_w = 0;
                pending_space = false;
            }
            Token::Space => {
                if !line.is_empty() {
                    pending_space = true;
                }
            }
            Token::Word(pieces) => {
                let word_w: usize = pieces.iter().map(|p| display_width(&p.text)).sum();
                let advance = word_w + usize::from(pending_space && !line.is_empty());
                if !line.is_empty() && line_w + advance > width {
                    lines.push(std::mem::take(&mut line));
                    line_w = 0;
                    pending_space = false;
                }
                if pending_space && !line.is_empty() {
                    if let Some(last) = line.last_mut() {
                        last.text.push(' ');
                    }
                    line_w += 1;
                }
                line.extend(pieces);
                line_w += word_w;
                pending_space = false;
            }
        }
    }

    if !line.is_empty() || lines.is_empty() {
        lines.push(line);
    }
    lines
}

fn tokenize(spans: &[Span]) -> Vec<Token> {
    let mut tokens: Vec<Token> = Vec::new();
    let mut word: Vec<Span> = Vec::new();

    for span in spans {
        for ch in span.text.chars() {
            if ch == '\n' {
                flush_word(&mut word, &mut tokens);
                tokens.push(Token::HardBreak);
            } else if ch.is_whitespace() {
                flush_word(&mut word, &mut tokens);
                if !matches!(tokens.last(), Some(Token::Space | Token::HardBreak)) {
                    tokens.push(Token::Space);
                }
            } else {
                push_char(&mut word, ch, span);
            }
        }
    }
    flush_word(&mut word, &mut tokens);
    tokens
}

fn flush_word(word: &mut Vec<Span>, tokens: &mut Vec<Token>) {
    if !word.is_empty() {
        tokens.push(Token::Word(std::mem::take(word)));
    }
}

/// Append `ch` to the current word, merging into the last piece when it shares
/// the incoming span's styling, otherwise starting a new styled piece.
fn push_char(word: &mut Vec<Span>, ch: char, span: &Span) {
    let same_style = word.last().is_some_and(|last| {
        last.role == span.role && last.mods == span.mods && last.link == span.link
    });
    if same_style {
        if let Some(last) = word.last_mut() {
            last.text.push(ch);
        }
    } else {
        let mut text = String::new();
        text.push(ch);
        word.push(Span {
            text,
            role: span.role,
            mods: span.mods,
            link: span.link,
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::render::terminal::model::{Mods, Role};

    fn body(text: &str) -> Span {
        Span::body(text)
    }

    fn line_text(line: &[Span]) -> String {
        line.iter().map(|s| s.text.as_str()).collect()
    }

    #[test]
    fn wraps_on_word_boundaries() {
        let spans = vec![body("the quick brown fox jumps")];
        let lines = wrap_spans(&spans, 10);
        for line in &lines {
            assert!(display_width(&line_text(line)) <= 10, "line too wide");
        }
        let joined: String = lines
            .iter()
            .map(|l| line_text(l))
            .collect::<Vec<_>>()
            .join(" ");
        assert_eq!(joined, "the quick brown fox jumps");
    }

    #[test]
    fn hard_break_forces_new_line() {
        let spans = vec![Span::new("alpha\nbeta", Role::Body, Mods::default())];
        let lines = wrap_spans(&spans, 80);
        assert_eq!(lines.len(), 2);
        assert_eq!(line_text(&lines[0]), "alpha");
        assert_eq!(line_text(&lines[1]), "beta");
    }

    #[test]
    fn truncate_respects_width() {
        assert_eq!(truncate("hello world", 8, "\u{2026}"), "hello w\u{2026}");
        assert_eq!(truncate("short", 10, "\u{2026}"), "short");
    }

    #[test]
    fn sanitize_strips_terminal_escapes() {
        // An OSC 52 clipboard-write smuggled into markdown text.
        let evil = "before\u{1b}]52;c;ZXZpbA==\u{7}after\u{1b}[31m";
        let clean = sanitize(evil);
        assert!(!clean.contains('\u{1b}'), "ESC must be stripped");
        assert!(!clean.contains('\u{7}'), "BEL must be stripped");
        assert!(clean.contains("before") && clean.contains("after"));
    }

    #[test]
    fn sanitize_keeps_clean_text_borrowed() {
        assert!(matches!(sanitize("plain\ttext"), Cow::Borrowed(_)));
    }
}
