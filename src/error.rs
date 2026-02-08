use std::io;

/// All errors that can occur during `SilkPrint` operation.
#[derive(Debug, thiserror::Error, miette::Diagnostic)]
pub enum SilkprintError {
    #[error("Failed to read input file: {path}")]
    #[diagnostic(help("Check that the file exists and is readable"))]
    InputRead {
        path: String,
        #[source]
        source: io::Error,
    },

    #[error("Invalid front matter in document")]
    #[diagnostic(code(silkprint::frontmatter))]
    FrontMatter {
        #[source_code]
        src: miette::NamedSource<String>,
        #[label("parse error here")]
        span: miette::SourceSpan,
    },

    #[error("Theme '{name}' not found")]
    #[diagnostic(
        help("Did you mean: {suggestions}\nRun `silkprint --list-themes` for all options"),
        code(silkprint::theme::not_found)
    )]
    ThemeNotFound { name: String, suggestions: String },

    #[error("Invalid theme configuration")]
    #[diagnostic(code(silkprint::theme::invalid))]
    ThemeInvalid {
        #[source_code]
        src: miette::NamedSource<String>,
        #[label("{message}")]
        span: miette::SourceSpan,
        message: String,
    },

    #[error("Theme inheritance cycle detected: {chain}")]
    #[diagnostic(code(silkprint::theme::cycle))]
    ThemeCycle { chain: String },

    #[error("Theme inheritance depth exceeded (max 5): {chain}")]
    #[diagnostic(code(silkprint::theme::depth))]
    ThemeInheritanceDepth { chain: String },

    #[error("Invalid paper size: {size}")]
    #[diagnostic(help("Valid sizes: a4, letter, a5, legal"))]
    InvalidPaperSize { size: String },

    #[error("Conflicting CLI options")]
    #[diagnostic(code(silkprint::cli::conflict))]
    ConflictingOptions { details: String },

    #[error("No fonts available for '{role}' — all fallbacks exhausted")]
    #[diagnostic(code(silkprint::font::exhausted))]
    FontExhausted { role: String, tried: Vec<String> },

    #[error("Typst compilation failed")]
    #[diagnostic(
        code(silkprint::render::typst),
        help("This is likely a SilkPrint bug — please report it with your input file")
    )]
    TypstCompilation { diagnostics: Vec<String> },

    #[error("Rendering failed")]
    #[diagnostic(code(silkprint::render), help("{hint}"))]
    RenderFailed { details: String, hint: String },

    #[error("Failed to write output: {path}")]
    OutputWrite {
        path: String,
        #[source]
        source: io::Error,
    },
}
