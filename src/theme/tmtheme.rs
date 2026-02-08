use super::syntax::ResolvedSyntaxStyle;

/// Generate a tmTheme XML document from resolved syntax styles.
///
/// Typst uses tmTheme (`TextMate`) format for syntax highlighting.
/// The generated XML is served as a virtual file via `World::file()`
/// at `/__silkprint_theme.tmTheme`.
pub fn generate_tmtheme(
    _name: &str,
    _background: &str,
    _foreground: &str,
    _styles: &[ResolvedSyntaxStyle],
) -> String {
    // Stub — Wave 2D builds the full generator
    String::new()
}
