use std::fmt;

/// Non-fatal warnings collected during rendering.
///
/// Displayed after completion in default/verbose modes.
/// Suppressed by `--quiet`. Never cause non-zero exit codes.
#[derive(Debug, Clone)]
pub enum SilkprintWarning {
    ImageNotFound {
        path: String,
    },
    FontNotAvailable {
        name: String,
        fallback: String,
    },
    UnknownLanguage {
        lang: String,
    },
    UnrecognizedFrontMatter {
        field: String,
    },
    ContrastRatio {
        element: String,
        ratio: f64,
        minimum: f64,
    },
    RemoteImageSkipped {
        url: String,
    },
}

impl fmt::Display for SilkprintWarning {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::ImageNotFound { path } => {
                write!(f, "image '{path}' not found, skipping")
            }
            Self::FontNotAvailable { name, fallback } => {
                write!(
                    f,
                    "font '{name}' not available, falling back to '{fallback}'"
                )
            }
            Self::UnknownLanguage { lang } => {
                write!(
                    f,
                    "code block language '{lang}' not recognized for highlighting"
                )
            }
            Self::UnrecognizedFrontMatter { field } => {
                write!(f, "unrecognized front matter field: '{field}'")
            }
            Self::ContrastRatio {
                element,
                ratio,
                minimum,
            } => {
                write!(
                    f,
                    "{element}: contrast ratio {ratio:.2}:1 below minimum {minimum:.1}:1"
                )
            }
            Self::RemoteImageSkipped { url } => {
                write!(f, "remote image skipped (not supported in v0.1): {url}")
            }
        }
    }
}

/// Collects warnings during the rendering pipeline.
#[derive(Debug, Default)]
pub struct WarningCollector {
    warnings: Vec<SilkprintWarning>,
}

impl WarningCollector {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn push(&mut self, warning: SilkprintWarning) {
        self.warnings.push(warning);
    }

    pub fn warnings(&self) -> &[SilkprintWarning] {
        &self.warnings
    }

    pub fn into_warnings(self) -> Vec<SilkprintWarning> {
        self.warnings
    }

    pub fn is_empty(&self) -> bool {
        self.warnings.is_empty()
    }
}
