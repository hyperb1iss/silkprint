use serde::Deserialize;
use std::collections::HashMap;

/// Complete theme token hierarchy, deserialized from TOML.
#[derive(Debug, Clone, Default, Deserialize)]
#[serde(default)]
pub struct ThemeTokens {
    pub meta: MetaTokens,
    pub colors: HashMap<String, String>,
    pub fonts: FontTokens,
    pub font_sizes: FontSizeTokens,
    pub page: PageTokens,
    pub text: TextTokens,
    pub headings: HeadingTokens,
    pub code_block: CodeBlockTokens,
    pub code_inline: CodeInlineTokens,
    pub blockquote: BlockquoteTokens,
    pub table: TableTokens,
    pub horizontal_rule: HorizontalRuleTokens,
    pub links: LinkTokens,
    pub images: ImageTokens,
    pub list: ListTokens,
    pub footnotes: FootnoteTokens,
    pub alerts: AlertTokens,
    pub toc: TocTokens,
    pub page_numbers: PageNumberTokens,
    pub title_page: TitlePageTokens,
    pub emphasis: EmphasisTokens,
    pub math: MathTokens,
    pub highlight: HighlightTokens,
    pub description_list: DescriptionListTokens,
    pub syntax: SyntaxTokens,
}

#[derive(Debug, Clone, Default, Deserialize)]
#[serde(default)]
pub struct MetaTokens {
    pub name: String,
    pub version: String,
    pub variant: String,
    pub description: String,
    pub print_safe: bool,
    pub extends: String,
}

#[derive(Debug, Clone, Default, Deserialize)]
#[serde(default)]
pub struct FontTokens {
    pub heading: String,
    pub heading_weight: u16,
    pub heading_italic: bool,
    pub body: String,
    pub body_weight: u16,
    pub body_italic: bool,
    pub mono: String,
    pub mono_weight: u16,
    pub mono_ligatures: bool,
    pub heading_fallback: Vec<String>,
    pub body_fallback: Vec<String>,
    pub mono_fallback: Vec<String>,
}

#[derive(Debug, Clone, Default, Deserialize)]
#[serde(default)]
pub struct FontSizeTokens {
    pub body: String,
    pub small: String,
    pub code: String,
    pub h1: String,
    pub h2: String,
    pub h3: String,
    pub h4: String,
    pub h5: String,
    pub h6: String,
}

#[derive(Debug, Clone, Default, Deserialize)]
#[serde(default)]
pub struct PageTokens {
    pub background: String,
    pub margin_top: String,
    pub margin_bottom: String,
    pub margin_left: String,
    pub margin_right: String,
    pub paper: String,
    pub columns: u8,
    pub column_gap: String,
}

#[derive(Debug, Clone, Default, Deserialize)]
#[serde(default)]
pub struct TextTokens {
    pub color: String,
    pub line_height: f64,
    pub paragraph_gap: String,
    pub justification: String,
    pub spacing_mode: String,
    pub first_line_indent: String,
    pub orphan_lines: u8,
    pub widow_lines: u8,
}

#[derive(Debug, Clone, Default, Deserialize)]
#[serde(default)]
pub struct HeadingTokens {
    pub color: String,
    pub font: String,
    pub line_height: f64,
    pub letter_spacing: String,
    pub h1: HeadingLevelTokens,
    pub h2: HeadingLevelTokens,
    pub h3: HeadingLevelTokens,
    pub h4: HeadingLevelTokens,
    pub h5: HeadingLevelTokens,
    pub h6: HeadingLevelTokens,
}

#[derive(Debug, Clone, Default, Deserialize)]
#[serde(default)]
pub struct HeadingLevelTokens {
    pub weight: u16,
    pub line_height: Option<f64>,
    pub border: Option<bool>,
    pub above: String,
    pub below: String,
    pub page_break_before: Option<bool>,
    pub uppercase: Option<bool>,
    pub letter_spacing: Option<String>,
}

#[derive(Debug, Clone, Default, Deserialize)]
#[serde(default)]
#[allow(clippy::struct_excessive_bools)]
pub struct CodeBlockTokens {
    pub background: String,
    pub border_color: String,
    pub border_radius: String,
    pub padding_vertical: String,
    pub padding_horizontal: String,
    pub line_height: f64,
    pub left_accent: bool,
    pub left_accent_color: String,
    pub line_numbers: bool,
    pub language_label: bool,
    pub language_label_color: String,
    pub language_label_size: String,
    pub wrap: bool,
}

#[derive(Debug, Clone, Default, Deserialize)]
#[serde(default)]
pub struct CodeInlineTokens {
    pub background: String,
    pub border_color: String,
    pub border_radius: String,
}

#[derive(Debug, Clone, Default, Deserialize)]
#[serde(default)]
pub struct BlockquoteTokens {
    pub border_color: String,
    pub border_width: String,
    pub background: String,
    pub background_opacity: f64,
    pub text_color: String,
    pub italic: bool,
    pub left_padding: String,
}

#[derive(Debug, Clone, Default, Deserialize)]
#[serde(default)]
pub struct TableTokens {
    pub header_background: String,
    pub header_border_color: String,
    pub header_border_width: String,
    pub header_font: String,
    pub header_weight: u16,
    pub row_border_color: String,
    pub row_border_width: String,
    pub stripe_background: String,
    pub vertical_lines: bool,
    pub cell_padding: String,
}

#[derive(Debug, Clone, Default, Deserialize)]
#[serde(default)]
pub struct HorizontalRuleTokens {
    pub color: String,
    pub width: String,
    pub thickness: String,
    pub style: String,
}

#[derive(Debug, Clone, Default, Deserialize)]
#[serde(default)]
pub struct LinkTokens {
    pub color: String,
    pub underline: bool,
}

#[derive(Debug, Clone, Default, Deserialize)]
#[serde(default)]
pub struct ImageTokens {
    pub max_width: String,
    pub alignment: String,
    pub border: bool,
    pub border_radius: String,
    pub caption_font: String,
    pub caption_size: String,
    pub caption_color: String,
    pub caption_italic: bool,
    pub caption_position: String,
}

#[derive(Debug, Clone, Default, Deserialize)]
#[serde(default)]
pub struct ListTokens {
    pub bullet_color: String,
    pub indent: String,
    pub nested_indent: String,
    pub task_checked_color: String,
    pub task_unchecked_color: String,
}

#[derive(Debug, Clone, Default, Deserialize)]
#[serde(default)]
pub struct FootnoteTokens {
    pub separator_color: String,
    pub separator_width: String,
    pub text_size: String,
    pub number_color: String,
    pub backref_color: String,
}

#[derive(Debug, Clone, Default, Deserialize)]
#[serde(default)]
pub struct AlertTokens {
    pub note_color: String,
    pub tip_color: String,
    pub important_color: String,
    pub warning_color: String,
    pub caution_color: String,
    pub border_width: String,
    pub background_opacity: f64,
    pub show_icon: bool,
    pub show_label: bool,
}

#[derive(Debug, Clone, Default, Deserialize)]
#[serde(default)]
pub struct TocTokens {
    pub title: String,
    pub title_size: String,
    pub entry_color: String,
    pub page_number_color: String,
    pub leader_style: String,
    pub indent: String,
    pub max_depth: u8,
}

#[derive(Debug, Clone, Default, Deserialize)]
#[serde(default)]
pub struct PageNumberTokens {
    pub enabled: bool,
    pub position: String,
    pub format: String,
    pub font: String,
    pub size: String,
    pub color: String,
    pub first_page: bool,
}

#[derive(Debug, Clone, Default, Deserialize)]
#[serde(default)]
pub struct TitlePageTokens {
    pub enabled: bool,
    pub title_font: String,
    pub title_size: String,
    pub title_color: String,
    pub subtitle_color: String,
    pub author_color: String,
    pub date_color: String,
    pub separator_color: String,
}

#[derive(Debug, Clone, Default, Deserialize)]
#[serde(default)]
pub struct EmphasisTokens {
    pub strikethrough_color: String,
}

#[derive(Debug, Clone, Default, Deserialize)]
#[serde(default)]
pub struct MathTokens {
    pub color: String,
}

#[derive(Debug, Clone, Default, Deserialize)]
#[serde(default)]
pub struct HighlightTokens {
    pub fill: String,
    pub fill_opacity: f64,
    pub text_color: String,
    pub border_radius: String,
}

#[derive(Debug, Clone, Default, Deserialize)]
#[serde(default)]
pub struct DescriptionListTokens {
    pub term_font: String,
    pub term_weight: u16,
    pub term_color: String,
    pub definition_indent: String,
    pub term_spacing: String,
    pub item_spacing: String,
}

/// Syntax highlighting tokens, each with optional color/bold/italic.
#[derive(Debug, Clone, Default, Deserialize)]
#[serde(default)]
pub struct SyntaxTokens {
    pub background: String,
    pub text: SyntaxStyleTokens,
    pub keyword: SyntaxStyleTokens,
    pub string: SyntaxStyleTokens,
    pub number: SyntaxStyleTokens,
    pub function: SyntaxStyleTokens,
    #[serde(rename = "type")]
    pub type_: SyntaxStyleTokens,
    pub comment: SyntaxStyleTokens,
    pub constant: SyntaxStyleTokens,
    pub boolean: SyntaxStyleTokens,
    pub operator: SyntaxStyleTokens,
    pub property: SyntaxStyleTokens,
    pub tag: SyntaxStyleTokens,
    pub attribute: SyntaxStyleTokens,
    pub variable: SyntaxStyleTokens,
    pub builtin: SyntaxStyleTokens,
    pub punctuation: SyntaxStyleTokens,
    pub escape: SyntaxStyleTokens,
}

#[derive(Debug, Clone, Default, Deserialize)]
#[serde(default)]
pub struct SyntaxStyleTokens {
    pub color: String,
    pub bold: Option<bool>,
    pub italic: Option<bool>,
}
