use serde::Deserialize;

use crate::error::SilkprintError;

/// Parsed YAML front matter from a Markdown document.
#[derive(Debug, Clone, Default, Deserialize)]
#[serde(default)]
pub struct FrontMatter {
    pub title: Option<String>,
    pub subtitle: Option<String>,
    pub author: Option<String>,
    pub date: Option<String>,
    pub lang: Option<String>,
    pub theme: Option<String>,
    pub paper: Option<String>,
    pub toc: Option<bool>,
    #[serde(rename = "toc-depth")]
    pub toc_depth: Option<u8>,
    pub numbering: Option<String>,
    #[serde(rename = "font-size")]
    pub font_size: Option<String>,
}

/// Extract front matter from input, returning (`front_matter`, `body_without_front_matter`).
///
/// Front matter is delimited by `---` lines at the start of the document.
pub fn extract(input: &str) -> Result<(Option<FrontMatter>, String), SilkprintError> {
    let trimmed = input.trim_start();

    if !trimmed.starts_with("---") {
        return Ok((None, input.to_string()));
    }

    // Find the closing delimiter
    let after_first = &trimmed[3..];
    let rest = after_first.trim_start_matches(['\r', '\n']);

    let Some(end_pos) = rest.find("\n---") else {
        // No closing delimiter — treat entire input as body (no front matter)
        return Ok((None, input.to_string()));
    };

    let yaml_content = &rest[..end_pos];
    let body_start = rest[end_pos + 4..].trim_start_matches(['\r', '\n']);

    let front_matter: FrontMatter =
        serde_yaml_ng::from_str(yaml_content).map_err(|_e| SilkprintError::FrontMatter {
            src: miette::NamedSource::new("front matter", yaml_content.to_string()),
            span: (0, yaml_content.len()).into(),
        })?;

    Ok((Some(front_matter), body_start.to_string()))
}
