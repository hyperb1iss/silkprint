pub mod cli;
pub mod error;
pub mod fonts;
pub mod render;
pub mod theme;
pub mod warnings;

use std::path::{Path, PathBuf};

use error::SilkprintError;
use warnings::WarningCollector;

/// How to find the theme to apply.
#[derive(Debug, Clone)]
pub enum ThemeSource {
    /// A built-in theme by name.
    BuiltIn(String),
    /// A custom theme file path.
    Custom(PathBuf),
    /// Raw TOML string.
    Inline(String),
}

/// Paper size for the output document.
#[derive(Debug, Clone, Copy, Default)]
pub enum PaperSize {
    #[default]
    A4,
    Letter,
    A5,
    Legal,
}

impl PaperSize {
    pub fn from_str_case_insensitive(s: &str) -> Result<Self, SilkprintError> {
        match s.to_lowercase().as_str() {
            "a4" => Ok(Self::A4),
            "letter" => Ok(Self::Letter),
            "a5" => Ok(Self::A5),
            "legal" => Ok(Self::Legal),
            _ => Err(SilkprintError::InvalidPaperSize {
                size: s.to_string(),
            }),
        }
    }

    pub fn as_typst_str(self) -> &'static str {
        match self {
            Self::A4 => "a4",
            Self::Letter => "us-letter",
            Self::A5 => "a5",
            Self::Legal => "us-legal",
        }
    }
}

/// Options for the render pipeline.
#[derive(Debug, Clone)]
pub struct RenderOptions {
    pub theme: ThemeSource,
    /// Whether the theme was explicitly set by the user (vs. default).
    /// When `false`, front matter `theme:` can override.
    pub theme_explicit: bool,
    pub paper: PaperSize,
    pub font_dirs: Vec<PathBuf>,
    pub toc: Option<bool>,
    pub title_page: Option<bool>,
}

impl Default for RenderOptions {
    fn default() -> Self {
        Self {
            theme: ThemeSource::BuiltIn("silkcircuit-dawn".to_string()),
            theme_explicit: false,
            paper: PaperSize::A4,
            font_dirs: Vec::new(),
            toc: None,
            title_page: None,
        }
    }
}

/// Render markdown to PDF bytes.
pub fn render(
    input: &str,
    input_path: Option<&Path>,
    options: &RenderOptions,
) -> Result<(Vec<u8>, Vec<warnings::SilkprintWarning>), SilkprintError> {
    let mut warnings = WarningCollector::new();

    // Extract front matter first — it may override the theme
    let (front_matter, body) = render::frontmatter::extract(input)?;
    if let Some(fm) = &front_matter {
        render::frontmatter::warn_unknown_fields(fm, &mut warnings);
    }
    let effective_theme_source = resolve_effective_theme(options, front_matter.as_ref());
    let resolved_theme = theme::load_theme(&effective_theme_source, &mut warnings)?;

    let pdf_bytes = render::render_pipeline(
        &body,
        front_matter.as_ref(),
        input_path,
        options,
        &resolved_theme,
        &mut warnings,
    )?;
    Ok((pdf_bytes, warnings.into_warnings()))
}

/// Render markdown to Typst source (intermediate representation).
pub fn render_to_typst(
    input: &str,
    options: &RenderOptions,
) -> Result<(String, Vec<warnings::SilkprintWarning>), SilkprintError> {
    let mut warnings = WarningCollector::new();

    let (front_matter, body) = render::frontmatter::extract(input)?;
    if let Some(fm) = &front_matter {
        render::frontmatter::warn_unknown_fields(fm, &mut warnings);
    }
    let effective_theme_source = resolve_effective_theme(options, front_matter.as_ref());
    let resolved_theme = theme::load_theme(&effective_theme_source, &mut warnings)?;

    let typst_source = render::render_to_typst_source(
        &body,
        front_matter.as_ref(),
        options,
        &resolved_theme,
        &mut warnings,
    )?;
    Ok((typst_source, warnings.into_warnings()))
}

/// Determine the effective theme source, respecting precedence:
/// CLI > front matter > default.
///
/// If the CLI theme is the built-in default ("silkcircuit-dawn") and front matter
/// specifies a different theme, the front matter theme wins.
fn resolve_effective_theme(
    options: &RenderOptions,
    front_matter: Option<&render::frontmatter::FrontMatter>,
) -> ThemeSource {
    // CLI explicit theme always wins (CLI > front matter > default)
    if options.theme_explicit {
        return options.theme.clone();
    }
    // Front matter theme overrides the default — apply same name-or-path resolution
    if let Some(fm_theme) = front_matter.and_then(|fm| fm.theme.as_deref()) {
        let path = Path::new(fm_theme);
        if path.extension().is_some_and(|ext| ext == "toml") {
            return ThemeSource::Custom(path.to_path_buf());
        }
        return ThemeSource::BuiltIn(fm_theme.to_string());
    }
    options.theme.clone()
}
