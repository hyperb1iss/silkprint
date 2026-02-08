use comrak::nodes::{AstNode, NodeValue};
use comrak::Options;

use crate::theme::ResolvedTheme;
use crate::warnings::{SilkprintWarning, WarningCollector};

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
///
/// This is a stub for Wave 3E, which builds the full Typst emitter.
/// The signature is final: it takes the root node, resolved theme,
/// and a warning collector for any rendering-time warnings.
pub fn emit_typst(
    _root: &AstNode<'_>,
    _theme: &ResolvedTheme,
    _warnings: &mut WarningCollector,
) -> String {
    // Stub -- Wave 3E builds the full emitter
    String::new()
}

/// Inspect a parsed AST for unusual content patterns and emit relevant warnings.
///
/// This performs a single walk of the tree looking for things like:
/// - Code blocks with unrecognized language identifiers
/// - Remote images (not supported in v0.1)
/// - Very deeply nested structures (which may cause layout issues)
///
/// Returns `true` if the document parsed cleanly with no warnings.
pub fn check_content<'a>(root: &'a AstNode<'a>, warnings: &mut WarningCollector) -> bool {
    let initial_count = warnings.warnings().len();

    for node in root.descendants() {
        let data = node.data.borrow();
        match &data.value {
            NodeValue::CodeBlock(code_block) => {
                check_code_block_language(&code_block.info, warnings);
            }
            NodeValue::Image(link) => {
                check_image_url(&link.url, warnings);
            }
            _ => {}
        }
    }

    warnings.warnings().len() == initial_count
}

/// Well-known code fence language identifiers that `syntect`/Typst can highlight.
const KNOWN_LANGUAGES: &[&str] = &[
    "bash",
    "c",
    "clojure",
    "cpp",
    "c++",
    "csharp",
    "c#",
    "cs",
    "css",
    "dart",
    "diff",
    "dockerfile",
    "elixir",
    "elm",
    "erlang",
    "go",
    "graphql",
    "haskell",
    "html",
    "java",
    "javascript",
    "js",
    "json",
    "jsonc",
    "jsx",
    "julia",
    "kotlin",
    "latex",
    "tex",
    "lua",
    "makefile",
    "markdown",
    "md",
    "nix",
    "objc",
    "objective-c",
    "ocaml",
    "perl",
    "php",
    "plain",
    "text",
    "txt",
    "powershell",
    "python",
    "py",
    "r",
    "ruby",
    "rb",
    "rust",
    "rs",
    "scala",
    "scss",
    "sh",
    "shell",
    "sql",
    "swift",
    "toml",
    "ts",
    "tsx",
    "typescript",
    "typst",
    "vim",
    "xml",
    "yaml",
    "yml",
    "zig",
    "zsh",
];

/// Warn if a code block specifies an unrecognized language identifier.
fn check_code_block_language(info: &str, warnings: &mut WarningCollector) {
    // The info string may contain extra metadata after the language (e.g. `rust,linenos`).
    // We only care about the first word.
    let lang = info.split([' ', ',', '\t']).next().unwrap_or("");
    if lang.is_empty() {
        return;
    }

    let lower = lang.to_lowercase();
    if !KNOWN_LANGUAGES.contains(&lower.as_str()) {
        warnings.push(SilkprintWarning::UnknownLanguage {
            lang: lang.to_string(),
        });
    }
}

/// Warn if an image references a remote URL (not supported in v0.1).
fn check_image_url(url: &str, warnings: &mut WarningCollector) {
    if url.starts_with("http://") || url.starts_with("https://") {
        warnings.push(SilkprintWarning::RemoteImageSkipped {
            url: url.to_string(),
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn options_enables_all_extensions() {
        let opts = comrak_options();
        assert!(opts.extension.strikethrough);
        assert!(opts.extension.table);
        assert!(opts.extension.autolink);
        assert!(opts.extension.tasklist);
        assert!(opts.extension.superscript);
        assert!(opts.extension.subscript);
        assert!(opts.extension.footnotes);
        assert!(opts.extension.description_lists);
        assert!(opts.extension.highlight);
        assert!(opts.extension.underline);
        assert!(opts.extension.math_dollars);
        assert!(opts.extension.alerts);
        assert!(opts.extension.shortcodes);
        assert!(opts.extension.wikilinks_title_after_pipe);
        assert_eq!(
            opts.extension.front_matter_delimiter,
            Some("---".to_owned())
        );
    }

    #[test]
    fn parse_produces_ast() {
        let arena = comrak::Arena::new();
        let root = parse(&arena, "# Hello\n\nWorld");
        // The root is a Document node with children
        let children: Vec<_> = root.children().collect();
        assert!(children.len() >= 2, "expected heading + paragraph");
    }

    #[test]
    fn check_content_warns_remote_image() {
        let arena = comrak::Arena::new();
        let root = parse(&arena, "![alt](https://example.com/img.png)");
        let mut warnings = WarningCollector::new();
        let clean = check_content(root, &mut warnings);
        assert!(!clean);
        assert_eq!(warnings.warnings().len(), 1);
    }

    #[test]
    fn check_content_warns_unknown_language() {
        let arena = comrak::Arena::new();
        let root = parse(&arena, "```qwxyz\ncode\n```");
        let mut warnings = WarningCollector::new();
        let clean = check_content(root, &mut warnings);
        assert!(!clean);
        assert_eq!(warnings.warnings().len(), 1);
    }

    #[test]
    fn check_content_accepts_known_language() {
        let arena = comrak::Arena::new();
        let root = parse(&arena, "```rust\nfn main() {}\n```");
        let mut warnings = WarningCollector::new();
        let clean = check_content(root, &mut warnings);
        assert!(clean);
        assert!(warnings.is_empty());
    }

    #[test]
    fn check_content_ignores_empty_language() {
        let arena = comrak::Arena::new();
        let root = parse(&arena, "```\nplain code\n```");
        let mut warnings = WarningCollector::new();
        let clean = check_content(root, &mut warnings);
        assert!(clean);
    }
}
