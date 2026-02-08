//! Syntax token resolution.
//!
//! Maps `SilkPrint`'s token names to `TextMate` scope selectors and resolves
//! color references against the theme's `[colors]` table.

use std::collections::HashMap;

use super::tokens::{SyntaxStyleTokens, SyntaxTokens};

/// Map syntax token names to `TextMate` scope selectors.
///
/// This defines how `SilkPrint`'s token names correspond to tmTheme scopes,
/// which Typst uses for syntax highlighting.
pub const TOKEN_SCOPE_MAP: &[(&str, &[&str])] = &[
    ("text", &["source"]),
    (
        "keyword",
        &["keyword", "keyword.control", "keyword.operator.word"],
    ),
    ("string", &["string", "string.quoted"]),
    ("number", &["constant.numeric"]),
    ("function", &["entity.name.function", "support.function"]),
    (
        "type",
        &["entity.name.type", "support.type", "storage.type"],
    ),
    ("comment", &["comment", "comment.line", "comment.block"]),
    ("constant", &["constant", "constant.language"]),
    ("boolean", &["constant.language.boolean"]),
    ("operator", &["keyword.operator"]),
    (
        "property",
        &["variable.other.property", "entity.other.attribute-name"],
    ),
    ("tag", &["entity.name.tag"]),
    ("attribute", &["entity.other.attribute-name"]),
    ("variable", &["variable", "variable.other"]),
    (
        "builtin",
        &["support.function.builtin", "support.class.builtin"],
    ),
    ("punctuation", &["punctuation"]),
    ("escape", &["constant.character.escape"]),
];

/// A resolved syntax style ready for tmTheme XML generation.
#[derive(Debug, Clone)]
pub struct ResolvedSyntaxStyle {
    pub name: String,
    pub scope: String,
    pub foreground: String,
    pub bold: bool,
    pub italic: bool,
}

/// Resolve a single color value against the colors table.
///
/// If the value starts with `#`, it's returned as-is (direct hex).
/// Otherwise it's looked up as a key in the colors table.
/// Returns the original value if lookup fails (the contrast checker
/// will catch invalid colors downstream).
fn resolve_color(value: &str, colors: &HashMap<String, String>) -> String {
    if value.is_empty() {
        return String::new();
    }
    if value.starts_with('#') {
        return value.to_string();
    }
    colors
        .get(value)
        .cloned()
        .unwrap_or_else(|| value.to_string())
}

/// Get a `SyntaxStyleTokens` from the `SyntaxTokens` struct by token name.
fn get_style_for_token<'a>(tokens: &'a SyntaxTokens, name: &str) -> &'a SyntaxStyleTokens {
    match name {
        "keyword" => &tokens.keyword,
        "string" => &tokens.string,
        "number" => &tokens.number,
        "function" => &tokens.function,
        "type" => &tokens.type_,
        "comment" => &tokens.comment,
        "constant" => &tokens.constant,
        "boolean" => &tokens.boolean,
        "operator" => &tokens.operator,
        "property" => &tokens.property,
        "tag" => &tokens.tag,
        "attribute" => &tokens.attribute,
        "variable" => &tokens.variable,
        "builtin" => &tokens.builtin,
        "punctuation" => &tokens.punctuation,
        "escape" => &tokens.escape,
        // "text" and any unknown token fall through to the default text style
        _ => &tokens.text,
    }
}

/// Resolve syntax tokens against the colors table, producing styles for tmTheme generation.
///
/// Each token maps to one or more `TextMate` scopes. Color references
/// (like `"text_primary"`) are resolved to hex values via the colors table.
#[allow(clippy::implicit_hasher)]
pub fn resolve_syntax_tokens(
    tokens: &SyntaxTokens,
    colors: &HashMap<String, String>,
) -> Vec<ResolvedSyntaxStyle> {
    TOKEN_SCOPE_MAP
        .iter()
        .map(|(name, scopes)| {
            let style = get_style_for_token(tokens, name);
            let foreground = resolve_color(&style.color, colors);
            ResolvedSyntaxStyle {
                name: (*name).to_string(),
                scope: scopes.join(", "),
                foreground,
                bold: style.bold.unwrap_or(false),
                italic: style.italic.unwrap_or(false),
            }
        })
        .collect()
}
