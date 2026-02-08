use super::tokens::SyntaxTokens;

/// Map syntax token names to `TextMate` scope selectors.
///
/// This defines how `SilkPrint`'s token names correspond to tmTheme scopes,
/// which Typst uses for syntax highlighting.
pub const TOKEN_SCOPE_MAP: &[(&str, &[&str])] = &[
    ("text", &["source"]),
    ("keyword", &["keyword", "keyword.control", "keyword.operator.word"]),
    ("string", &["string", "string.quoted"]),
    ("number", &["constant.numeric"]),
    ("function", &["entity.name.function", "support.function"]),
    ("type", &["entity.name.type", "support.type", "storage.type"]),
    ("comment", &["comment", "comment.line", "comment.block"]),
    ("constant", &["constant", "constant.language"]),
    ("boolean", &["constant.language.boolean"]),
    ("operator", &["keyword.operator"]),
    ("property", &["variable.other.property", "entity.other.attribute-name"]),
    ("tag", &["entity.name.tag"]),
    ("attribute", &["entity.other.attribute-name"]),
    ("variable", &["variable", "variable.other"]),
    ("builtin", &["support.function.builtin", "support.class.builtin"]),
    ("punctuation", &["punctuation"]),
    ("escape", &["constant.character.escape"]),
];

/// Get the resolved syntax token styles for tmTheme generation.
pub fn resolve_syntax_tokens(_tokens: &SyntaxTokens) -> Vec<ResolvedSyntaxStyle> {
    // Stub — Wave 2D builds the full resolver
    Vec::new()
}

/// A resolved syntax style ready for tmTheme XML generation.
#[derive(Debug, Clone)]
pub struct ResolvedSyntaxStyle {
    pub name: String,
    pub scope: String,
    pub foreground: String,
    pub bold: bool,
    pub italic: bool,
}
