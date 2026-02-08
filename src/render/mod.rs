pub mod emoji;
pub mod frontmatter;
pub mod image;
pub mod markdown;
pub mod preamble;
pub mod typst;

use std::path::Path;

use crate::error::SilkprintError;
use crate::theme::ResolvedTheme;
use crate::warnings::WarningCollector;
use crate::RenderOptions;

/// Orchestrates the full render pipeline: parse → emit → compile → PDF.
pub fn render_pipeline(
    input: &str,
    input_path: Option<&Path>,
    options: &RenderOptions,
    theme: &ResolvedTheme,
    warnings: &mut WarningCollector,
) -> Result<Vec<u8>, SilkprintError> {
    // 1. Extract front matter
    let (front_matter, body) = frontmatter::extract(input)?;

    // 2. Parse markdown to AST
    let arena = comrak::Arena::new();
    let root = markdown::parse(&arena, &body);

    // 3. Generate Typst preamble from theme + front matter
    let preamble = preamble::generate(theme, front_matter.as_ref(), options);

    // 4. Emit Typst content from AST
    let content = markdown::emit_typst(root, theme, warnings);

    // 5. Combine preamble + content
    let typst_source = format!("{preamble}\n\n{content}");

    // 6. Compile to PDF
    let root_dir = input_path
        .and_then(Path::parent)
        .unwrap_or_else(|| Path::new("."));

    typst::compile_to_pdf(&typst_source, theme, root_dir, warnings)
}

/// Orchestrates the pipeline up to Typst source generation (no compilation).
pub fn render_to_typst_source(
    input: &str,
    options: &RenderOptions,
    theme: &ResolvedTheme,
    warnings: &mut WarningCollector,
) -> Result<String, SilkprintError> {
    let (front_matter, body) = frontmatter::extract(input)?;
    let arena = comrak::Arena::new();
    let root = markdown::parse(&arena, &body);
    let preamble = preamble::generate(theme, front_matter.as_ref(), options);
    let content = markdown::emit_typst(root, theme, warnings);
    Ok(format!("{preamble}\n\n{content}"))
}
