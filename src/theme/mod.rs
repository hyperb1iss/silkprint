pub mod builtin;
pub mod contrast;
pub mod syntax;
pub mod tmtheme;
pub mod tokens;

use crate::error::SilkprintError;
use crate::warnings::WarningCollector;
use crate::ThemeSource;

/// A fully resolved theme with all color references replaced with hex values,
/// inheritance merged, and syntax highlighting ready.
#[derive(Debug, Clone)]
pub struct ResolvedTheme {
    pub tokens: tokens::ThemeTokens,
    pub tmtheme_xml: String,
}

/// Load and resolve a theme from the given source.
pub fn load_theme(
    _source: &ThemeSource,
    _warnings: &mut WarningCollector,
) -> Result<ResolvedTheme, SilkprintError> {
    // Stub — Wave 2D builds the full theme engine
    // For now, return a default silk-light placeholder
    Ok(ResolvedTheme {
        tokens: tokens::ThemeTokens::default(),
        tmtheme_xml: String::new(),
    })
}
