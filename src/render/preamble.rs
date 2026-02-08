use crate::render::frontmatter::FrontMatter;
use crate::theme::ResolvedTheme;
use crate::RenderOptions;

/// Generate the Typst preamble (set/show rules) from theme + front matter + options.
pub fn generate(
    _theme: &ResolvedTheme,
    _front_matter: Option<&FrontMatter>,
    _options: &RenderOptions,
) -> String {
    // Stub — Wave 3E builds the full preamble generator
    String::from("// SilkPrint preamble (stub)")
}
