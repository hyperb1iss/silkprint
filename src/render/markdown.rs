use comrak::nodes::AstNode;
use comrak::Options;

use crate::theme::ResolvedTheme;
use crate::warnings::WarningCollector;

/// Configure comrak with all extensions enabled per SPEC Section 8.2.
pub fn comrak_options() -> Options<'static> {
    let mut options = Options::default();

    // Core extensions
    options.extension.strikethrough = true;
    options.extension.table = true;
    options.extension.autolink = true;
    options.extension.tasklist = true;
    options.extension.superscript = true;
    options.extension.subscript = true;
    options.extension.footnotes = true;
    options.extension.description_lists = true;
    options.extension.highlight = true;
    options.extension.underline = true;

    // Math, front matter, alerts
    options.extension.math_dollars = true;
    options.extension.front_matter_delimiter = Some("---".to_owned());
    options.extension.alerts = true;

    // Emoji and wikilinks
    options.extension.shortcodes = true;
    options.extension.wikilinks_title_after_pipe = true;

    options
}

/// Parse markdown into a comrak AST.
pub fn parse<'a>(arena: &'a comrak::Arena<'a>, input: &str) -> &'a AstNode<'a> {
    let options = comrak_options();
    comrak::parse_document(arena, input, &options)
}

/// Walk a comrak AST and emit Typst markup.
pub fn emit_typst(
    _root: &AstNode<'_>,
    _theme: &ResolvedTheme,
    _warnings: &mut WarningCollector,
) -> String {
    // Stub — Wave 3E builds the full emitter
    String::new()
}
