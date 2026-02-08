use std::path::Path;

use crate::error::SilkprintError;
use crate::theme::ResolvedTheme;
use crate::warnings::WarningCollector;

/// Compile Typst source to PDF bytes.
///
/// Implements the World trait, loads fonts, resolves files,
/// and exports to PDF.
pub fn compile_to_pdf(
    _typst_source: &str,
    _theme: &ResolvedTheme,
    _root_dir: &Path,
    _warnings: &mut WarningCollector,
) -> Result<Vec<u8>, SilkprintError> {
    // Stub — Wave 3F builds the full World implementation
    Err(SilkprintError::RenderFailed {
        details: "Typst compilation not yet implemented".to_string(),
        hint: "This is a development stub".to_string(),
    })
}
