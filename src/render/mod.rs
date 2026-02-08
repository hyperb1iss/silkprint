pub mod emoji;
pub mod frontmatter;
pub mod image;
pub mod markdown;
pub mod mermaid;
pub mod preamble;
pub mod typst;

use std::path::Path;

use crate::RenderOptions;
use crate::error::SilkprintError;
use crate::theme::ResolvedTheme;
use crate::warnings::WarningCollector;

use self::frontmatter::FrontMatter;

/// Orchestrates the full render pipeline: parse → emit → compile → PDF.
///
/// Front matter has already been extracted by the caller (`lib.rs`) so
/// the theme can be resolved with front-matter overrides applied.
pub fn render_pipeline(
    body: &str,
    front_matter: Option<&FrontMatter>,
    input_path: Option<&Path>,
    options: &RenderOptions,
    theme: &ResolvedTheme,
    warnings: &mut WarningCollector,
) -> Result<Vec<u8>, SilkprintError> {
    // 1. Parse markdown to AST
    let arena = comrak::Arena::new();
    let root = markdown::parse(&arena, body);

    // 1b. Run content checks (remote images, unknown languages)
    markdown::check_content(root, warnings);

    // 2. Generate Typst preamble from theme + front matter + options
    let preamble = preamble::generate(theme, front_matter, options);

    // 3. Emit Typst content from AST (mermaid blocks become image refs)
    let (content, mermaid_sources) = markdown::emit_typst(root, theme, warnings);

    // 3b. Render mermaid diagrams to SVGs (native Rust — always available)
    let mermaid_svgs = if mermaid_sources.is_empty() {
        std::collections::HashMap::new()
    } else {
        tracing::info!(count = mermaid_sources.len(), "rendering mermaid diagrams");
        mermaid::render_all(&mermaid_sources, theme, warnings)
    };

    // 4. Combine preamble + content
    let typst_source = format!("{preamble}\n\n{content}");

    // 5. Compile to PDF
    let root_dir = input_path
        .and_then(Path::parent)
        .unwrap_or_else(|| Path::new("."));

    typst::compile_to_pdf(
        &typst_source,
        theme,
        root_dir,
        &options.font_dirs,
        &mermaid_svgs,
        warnings,
    )
}

/// Orchestrates the pipeline up to Typst source generation (no compilation).
pub fn render_to_typst_source(
    body: &str,
    front_matter: Option<&FrontMatter>,
    options: &RenderOptions,
    theme: &ResolvedTheme,
    warnings: &mut WarningCollector,
) -> Result<String, SilkprintError> {
    let arena = comrak::Arena::new();
    let root = markdown::parse(&arena, body);
    markdown::check_content(root, warnings);

    let preamble = preamble::generate(theme, front_matter, options);
    let (content, _mermaid_sources) = markdown::emit_typst(root, theme, warnings);
    Ok(format!("{preamble}\n\n{content}"))
}
