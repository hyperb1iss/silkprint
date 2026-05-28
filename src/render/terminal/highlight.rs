//! Code syntax highlighting for the terminal.
//!
//! syntect is used only as a *tokenizer*: it parses code into `TextMate` scope
//! stacks, which we classify into the 16 [`SyntaxRole`]s shared with the
//! theme's `[syntax]` section (via the same `TOKEN_SCOPE_MAP` the tmTheme
//! generator uses). Concrete colors are applied later by
//! [`ContentStyleResolver`](super::style::ContentStyleResolver), so code
//! coloring is driven by the silkprint theme — the same source of truth as the
//! PDF — and a live theme switch needs no re-tokenization.

use std::sync::OnceLock;

use syntect::highlighting::ScopeSelectors;
use syntect::parsing::{ParseState, ScopeStack, SyntaxSet};

use crate::theme::syntax::TOKEN_SCOPE_MAP;

use super::model::{Mods, Role, Span, SyntaxRole};

fn syntax_set() -> &'static SyntaxSet {
    static SET: OnceLock<SyntaxSet> = OnceLock::new();
    SET.get_or_init(SyntaxSet::load_defaults_newlines)
}

/// Selectors for each named role, in `TOKEN_SCOPE_MAP` order. Built once.
fn role_selectors() -> &'static [(SyntaxRole, ScopeSelectors)] {
    static SELECTORS: OnceLock<Vec<(SyntaxRole, ScopeSelectors)>> = OnceLock::new();
    SELECTORS.get_or_init(|| {
        TOKEN_SCOPE_MAP
            .iter()
            .filter_map(|(name, scopes)| {
                let role = role_from_name(name)?;
                let selector = scopes.join(", ").parse::<ScopeSelectors>().ok()?;
                Some((role, selector))
            })
            .collect()
    })
}

fn role_from_name(name: &str) -> Option<SyntaxRole> {
    Some(match name {
        "keyword" => SyntaxRole::Keyword,
        "string" => SyntaxRole::String,
        "number" => SyntaxRole::Number,
        "function" => SyntaxRole::Function,
        "type" => SyntaxRole::Type,
        "comment" => SyntaxRole::Comment,
        "constant" => SyntaxRole::Constant,
        "boolean" => SyntaxRole::Boolean,
        "operator" => SyntaxRole::Operator,
        "property" => SyntaxRole::Property,
        "tag" => SyntaxRole::Tag,
        "attribute" => SyntaxRole::Attribute,
        "variable" => SyntaxRole::Variable,
        "builtin" => SyntaxRole::Builtin,
        "punctuation" => SyntaxRole::Punctuation,
        "escape" => SyntaxRole::Escape,
        // "text" classifies as the default; skip so it never out-ranks a real match.
        _ => return None,
    })
}

/// Classify a scope stack into a [`SyntaxRole`], picking the highest-power match.
fn classify(stack: &ScopeStack) -> SyntaxRole {
    let scopes = stack.as_slice();
    let mut best: Option<(SyntaxRole, f64)> = None;
    for (role, selector) in role_selectors() {
        if let Some(power) = selector.does_match(scopes) {
            let power = power.0;
            if best.is_none_or(|(_, p)| power > p) {
                best = Some((*role, power));
            }
        }
    }
    best.map_or(SyntaxRole::Text, |(role, _)| role)
}

/// Highlight a code block into per-line spans tagged with [`SyntaxRole`].
///
/// Unknown languages (or plain text) yield a single `Text` span per line.
pub fn highlight_block(code: &str, lang: Option<&str>) -> Vec<Vec<Span>> {
    let ss = syntax_set();
    let syntax = lang
        .and_then(|l| ss.find_syntax_by_token(l))
        .unwrap_or_else(|| ss.find_syntax_plain_text());

    let mut state = ParseState::new(syntax);
    let mut stack = ScopeStack::new();
    let mut out = Vec::new();

    for line in code.split_inclusive('\n') {
        let display_line = line.strip_suffix('\n').unwrap_or(line);
        let Ok(ops) = state.parse_line(line, ss) else {
            out.push(vec![syntax_span(display_line, SyntaxRole::Text)]);
            continue;
        };

        let mut spans = Vec::new();
        let mut last = 0;
        for (offset, op) in ops {
            if offset > last && offset <= line.len() {
                push_segment(&mut spans, &line[last..offset], &stack);
            }
            if stack.apply(&op).is_err() {
                // Corrupt op stream for this line: bail to plain text.
                spans.clear();
                spans.push(syntax_span(display_line, SyntaxRole::Text));
                last = line.len();
                break;
            }
            last = offset.min(line.len());
        }
        if last < line.len() {
            push_segment(&mut spans, &line[last..], &stack);
        }
        out.push(spans);
    }

    out
}

fn push_segment(spans: &mut Vec<Span>, text: &str, stack: &ScopeStack) {
    let text = text.strip_suffix('\n').unwrap_or(text);
    if text.is_empty() {
        return;
    }
    spans.push(syntax_span(text, classify(stack)));
}

fn syntax_span(text: &str, role: SyntaxRole) -> Span {
    Span::new(text, Role::Syntax(role), Mods::default())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn flatten(lines: &[Vec<Span>]) -> String {
        lines
            .iter()
            .map(|line| line.iter().map(|s| s.text.as_str()).collect::<String>())
            .collect::<Vec<_>>()
            .join("\n")
    }

    #[test]
    fn highlights_rust_and_preserves_text() {
        let lines = highlight_block("fn main() {}\n", Some("rust"));
        assert_eq!(flatten(&lines).trim_end(), "fn main() {}");
        // The `fn` keyword should classify as Keyword, not Text.
        let has_keyword = lines
            .iter()
            .flatten()
            .any(|s| matches!(s.role, Role::Syntax(SyntaxRole::Keyword)));
        assert!(has_keyword, "expected a keyword token in `fn main`");
    }

    #[test]
    fn plain_text_for_unknown_language() {
        let lines = highlight_block("just words\n", Some("nonsense-lang"));
        assert_eq!(flatten(&lines).trim_end(), "just words");
    }

    #[test]
    fn preserves_content_exactly() {
        let code = "let x = 42;\nlet y = \"hi\";\n";
        let lines = highlight_block(code, Some("rust"));
        assert_eq!(flatten(&lines), "let x = 42;\nlet y = \"hi\";");
    }
}
