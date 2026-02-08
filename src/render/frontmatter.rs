use std::collections::HashMap;

use serde::Deserialize;

use crate::PaperSize;
use crate::error::SilkprintError;
use crate::warnings::{SilkprintWarning, WarningCollector};

/// Parsed YAML front matter from a Markdown document.
///
/// All fields are optional. Unknown fields are captured in `extras`
/// and surfaced as warnings rather than hard errors.
#[derive(Debug, Clone, Default, Deserialize)]
#[serde(default)]
pub struct FrontMatter {
    pub title: Option<String>,
    pub subtitle: Option<String>,
    pub author: Option<String>,
    pub date: Option<FlexibleDate>,
    pub lang: Option<String>,
    pub theme: Option<String>,
    pub paper: Option<String>,
    pub toc: Option<bool>,
    #[serde(rename = "toc-depth")]
    pub toc_depth: Option<u8>,
    pub numbering: Option<String>,
    #[serde(rename = "font-size")]
    pub font_size: Option<String>,

    /// Unknown fields from the front matter YAML.
    ///
    /// These are captured rather than rejected so the parse doesn't fail
    /// on forward-compatible or user-custom metadata.
    #[serde(flatten)]
    pub extras: HashMap<String, serde_yaml_ng::Value>,
}

/// A date value that accepts either a YAML date or a plain string.
///
/// YAML natively parses `2026-02-07` as a date, but users may also write
/// quoted strings like `"February 2026"`. This wrapper handles both.
#[derive(Debug, Clone, Default)]
pub struct FlexibleDate(pub String);

impl<'de> Deserialize<'de> for FlexibleDate {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let value = serde_yaml_ng::Value::deserialize(deserializer)?;
        // Coerce any YAML scalar (string, date, number) to its string representation.
        let s = match &value {
            serde_yaml_ng::Value::String(s) => s.clone(),
            other => {
                // serde_yaml_ng formats dates, numbers, booleans, etc. into strings
                let mut buf = Vec::new();
                serde_yaml_ng::to_writer(&mut buf, other)
                    .ok()
                    .and_then(|()| String::from_utf8(buf).ok())
                    .map_or_else(|| format!("{other:?}"), |s| s.trim().to_string())
            }
        };
        Ok(FlexibleDate(s))
    }
}

/// Extract front matter from input, returning (`front_matter`, `body_without_front_matter`).
///
/// Front matter is delimited by `---` lines at the start of the document.
/// Handles both Unix (`\n`) and Windows (`\r\n`) line endings.
/// On parse failure, returns a [`SilkprintError::FrontMatter`] with a miette span
/// pointing at the YAML content.
pub fn extract(input: &str) -> Result<(Option<FrontMatter>, String), SilkprintError> {
    let trimmed = input.trim_start();

    if !trimmed.starts_with("---") {
        return Ok((None, input.to_string()));
    }

    // The opening delimiter line: skip "---" and any trailing whitespace on that line.
    let after_opener = &trimmed[3..];
    let rest = after_opener.trim_start_matches(['\r', '\n']);

    // Locate the closing `---` delimiter. We need to find `\n---` to ensure
    // it's at the start of a line (works for both `\n---` and `\r\n---`).
    let Some(end_pos) = rest.find("\n---") else {
        // Also try `\r\n---` in case the entire file uses bare \r (unlikely but safe).
        if let Some(cr_end) = rest.find("\r\n---") {
            return parse_yaml_section(input, rest, cr_end);
        }
        // No closing delimiter found -- treat entire input as body, no front matter.
        return Ok((None, input.to_string()));
    };

    parse_yaml_section(input, rest, end_pos)
}

/// Parse the YAML section between the two `---` delimiters and split the body.
fn parse_yaml_section(
    original_input: &str,
    rest: &str,
    yaml_end: usize,
) -> Result<(Option<FrontMatter>, String), SilkprintError> {
    let yaml_content = &rest[..yaml_end];

    // Body starts after the closing `---` line. Skip "\n---" (4 chars) or "\r\n---" (5 chars),
    // then consume the remainder of that line (trailing whitespace/newlines).
    let after_closer = &rest[yaml_end..];
    let after_delimiter = skip_closing_delimiter_line(after_closer);

    // Calculate the byte offset of the YAML content within the original input
    // so miette can highlight the right region on error.
    let yaml_offset = byte_offset_within(original_input, yaml_content);

    let front_matter: FrontMatter = serde_yaml_ng::from_str(yaml_content).map_err(|e| {
        let span = yaml_error_span(&e, yaml_content.len());
        SilkprintError::FrontMatter {
            src: miette::NamedSource::new(
                "front matter",
                original_input[yaml_offset..yaml_offset + yaml_content.len()].to_string(),
            ),
            span,
        }
    })?;

    Ok((Some(front_matter), after_delimiter.to_string()))
}

/// Compute the byte offset of `sub` within `parent`.
///
/// Both slices must originate from the same underlying string allocation.
/// Uses safe address arithmetic to avoid requiring `unsafe`.
#[allow(clippy::as_conversions)]
fn byte_offset_within(parent: &str, sub: &str) -> usize {
    let parent_addr = parent.as_ptr() as usize;
    let sub_addr = sub.as_ptr() as usize;
    debug_assert!(
        sub_addr >= parent_addr,
        "sub must be within parent allocation"
    );
    sub_addr - parent_addr
}

/// Skip past the closing delimiter line (`\n---\n` or `\r\n---\r\n`).
fn skip_closing_delimiter_line(s: &str) -> &str {
    // Expected patterns: "\n---\n...", "\n---\r\n...", "\r\n---\n...", "\r\n---\r\n..."
    let s = strip_newline_prefix(s);
    let s = s.strip_prefix("---").unwrap_or(s);
    strip_newline_prefix(s)
}

/// Strip a leading `\r\n` or `\n` from a string slice.
fn strip_newline_prefix(s: &str) -> &str {
    s.strip_prefix("\r\n")
        .unwrap_or_else(|| s.strip_prefix('\n').unwrap_or(s))
}

/// Attempt to extract a source span from a `serde_yaml_ng` error.
///
/// If the error includes location info, we construct a span pointing there.
/// Otherwise we fall back to spanning the entire YAML block.
fn yaml_error_span(err: &serde_yaml_ng::Error, yaml_len: usize) -> miette::SourceSpan {
    err.location()
        .map_or_else(|| (0, yaml_len).into(), |loc| (loc.index(), 1).into())
}

/// Emit warnings for any unrecognized fields captured in `extras`.
pub fn warn_unknown_fields(front_matter: &FrontMatter, warnings: &mut WarningCollector) {
    for field in front_matter.extras.keys() {
        warnings.push(SilkprintWarning::UnrecognizedFrontMatter {
            field: field.clone(),
        });
    }
}

// ═══════════════════════════════════════════════════════════════════
// Merged options: CLI > front matter > theme defaults > built-in
// ═══════════════════════════════════════════════════════════════════

/// Final resolved document options after merging all sources.
///
/// Precedence (highest to lowest):
/// 1. CLI arguments
/// 2. Front matter YAML
/// 3. Theme defaults
/// 4. Built-in defaults
#[derive(Debug, Clone)]
pub struct MergedOptions {
    pub title: Option<String>,
    pub subtitle: Option<String>,
    pub author: Option<String>,
    pub date: Option<String>,
    pub lang: String,
    pub theme: String,
    pub paper: PaperSize,
    pub toc: bool,
    pub toc_depth: u8,
    pub numbering: Option<String>,
    pub font_size: Option<String>,
}

/// Values from CLI flags relevant to document-level options.
#[derive(Debug, Clone, Default)]
pub struct CliOptionOverrides {
    pub theme: Option<String>,
    pub paper: Option<PaperSize>,
    pub toc: Option<bool>,
    pub font_size: Option<String>,
    pub lang: Option<String>,
}

/// Theme-level defaults for document options.
#[derive(Debug, Clone, Default)]
pub struct ThemeDefaults {
    pub paper: Option<PaperSize>,
    pub toc: Option<bool>,
    pub toc_depth: Option<u8>,
    pub lang: Option<String>,
    pub font_size: Option<String>,
}

/// Merge options from all four layers.
///
/// Precedence: `cli` > `front_matter` > `theme_defaults` > built-in defaults.
pub fn merge_options(
    cli: &CliOptionOverrides,
    front_matter: Option<&FrontMatter>,
    theme_defaults: &ThemeDefaults,
) -> MergedOptions {
    let fm_paper = front_matter
        .and_then(|fm| fm.paper.as_deref())
        .and_then(|s| PaperSize::from_str_case_insensitive(s).ok());

    let fm_toc = front_matter.and_then(|fm| fm.toc);
    let fm_toc_depth = front_matter.and_then(|fm| fm.toc_depth);
    let fm_lang = front_matter.and_then(|fm| fm.lang.clone());
    let fm_theme = front_matter.and_then(|fm| fm.theme.clone());
    let fm_font_size = front_matter.and_then(|fm| fm.font_size.clone());
    let fm_date = front_matter.and_then(|fm| fm.date.as_ref().map(|d| d.0.clone()));

    MergedOptions {
        title: front_matter.and_then(|fm| fm.title.clone()),
        subtitle: front_matter.and_then(|fm| fm.subtitle.clone()),
        author: front_matter.and_then(|fm| fm.author.clone()),
        date: fm_date,
        lang: cli
            .lang
            .clone()
            .or(fm_lang)
            .or_else(|| theme_defaults.lang.clone())
            .unwrap_or_else(|| "en".to_string()),
        theme: cli
            .theme
            .clone()
            .or(fm_theme)
            .unwrap_or_else(|| "silk-light".to_string()),
        paper: cli
            .paper
            .or(fm_paper)
            .or(theme_defaults.paper)
            .unwrap_or_default(),
        toc: cli.toc.or(fm_toc).or(theme_defaults.toc).unwrap_or(false),
        toc_depth: fm_toc_depth.or(theme_defaults.toc_depth).unwrap_or(3),
        numbering: front_matter.and_then(|fm| fm.numbering.clone()),
        font_size: cli
            .font_size
            .clone()
            .or(fm_font_size)
            .or_else(|| theme_defaults.font_size.clone()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn extract_no_front_matter() {
        let input = "# Hello World\n\nSome body text.";
        let (fm, body) = extract(input).expect("should not error");
        assert!(fm.is_none());
        assert_eq!(body, input);
    }

    #[test]
    fn extract_basic_front_matter() {
        let input = "---\ntitle: My Doc\nauthor: Nova\n---\n# Heading\n";
        let (fm, body) = extract(input).expect("should not error");
        let fm = fm.expect("should have front matter");
        assert_eq!(fm.title.as_deref(), Some("My Doc"));
        assert_eq!(fm.author.as_deref(), Some("Nova"));
        assert_eq!(body, "# Heading\n");
    }

    #[test]
    fn extract_windows_line_endings() {
        let input = "---\r\ntitle: Windows\r\n---\r\nBody text\r\n";
        let (fm, body) = extract(input).expect("should not error");
        let fm = fm.expect("should have front matter");
        assert_eq!(fm.title.as_deref(), Some("Windows"));
        assert_eq!(body, "Body text\r\n");
    }

    #[test]
    fn extract_unknown_fields_captured() {
        let input = "---\ntitle: Test\ncustom_field: hello\n---\nBody\n";
        let (fm, _body) = extract(input).expect("should not error");
        let fm = fm.expect("should have front matter");
        assert!(fm.extras.contains_key("custom_field"));
    }

    #[test]
    fn extract_no_closing_delimiter() {
        let input = "---\ntitle: Oops\nno closing\n";
        let (fm, body) = extract(input).expect("should not error");
        assert!(fm.is_none());
        assert_eq!(body, input);
    }

    #[test]
    fn flexible_date_string() {
        let input = "---\ndate: \"February 2026\"\n---\nBody\n";
        let (fm, _) = extract(input).expect("should not error");
        let fm = fm.expect("should have front matter");
        let date = fm.date.expect("should have date");
        assert_eq!(date.0, "February 2026");
    }

    #[test]
    fn merge_cli_wins() {
        let fm = FrontMatter {
            paper: Some("letter".to_string()),
            toc: Some(true),
            ..Default::default()
        };
        let cli = CliOptionOverrides {
            paper: Some(PaperSize::A5),
            toc: Some(false),
            ..Default::default()
        };
        let merged = merge_options(&cli, Some(&fm), &ThemeDefaults::default());
        assert!(matches!(merged.paper, PaperSize::A5));
        assert!(!merged.toc);
    }

    #[test]
    fn merge_frontmatter_over_theme() {
        let fm = FrontMatter {
            lang: Some("de".to_string()),
            ..Default::default()
        };
        let theme = ThemeDefaults {
            lang: Some("fr".to_string()),
            ..Default::default()
        };
        let merged = merge_options(&CliOptionOverrides::default(), Some(&fm), &theme);
        assert_eq!(merged.lang, "de");
    }

    #[test]
    fn merge_falls_through_to_defaults() {
        let merged = merge_options(
            &CliOptionOverrides::default(),
            None,
            &ThemeDefaults::default(),
        );
        assert_eq!(merged.lang, "en");
        assert_eq!(merged.theme, "silk-light");
        assert!(matches!(merged.paper, PaperSize::A4));
        assert!(!merged.toc);
        assert_eq!(merged.toc_depth, 3);
    }
}
