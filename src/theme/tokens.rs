use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Complete theme token hierarchy, deserialized from TOML.
#[derive(Debug, Clone, Default, Deserialize, Serialize)]
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

#[derive(Debug, Clone, Default, Deserialize, Serialize)]
#[serde(default)]
pub struct MetaTokens {
    pub name: String,
    pub version: String,
    pub variant: String,
    pub description: String,
    pub print_safe: bool,
    pub extends: String,
}

#[derive(Debug, Clone, Default, Deserialize, Serialize)]
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

#[derive(Debug, Clone, Default, Deserialize, Serialize)]
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

#[derive(Debug, Clone, Default, Deserialize, Serialize)]
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

#[derive(Debug, Clone, Default, Deserialize, Serialize)]
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

#[derive(Debug, Clone, Default, Deserialize, Serialize)]
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

#[derive(Debug, Clone, Default, Deserialize, Serialize)]
#[serde(default)]
pub struct HeadingLevelTokens {
    /// Optional per-level color override. Falls back to `[headings].color`
    /// when empty. Resolves through the `[colors]` table just like other
    /// semantic color tokens.
    pub color: String,
    pub weight: u16,
    pub line_height: Option<f64>,
    pub border: Option<bool>,
    pub above: String,
    pub below: String,
    pub page_break_before: Option<bool>,
    pub uppercase: Option<bool>,
    pub letter_spacing: Option<String>,
}

#[derive(Debug, Clone, Default, Deserialize, Serialize)]
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

#[derive(Debug, Clone, Default, Deserialize, Serialize)]
#[serde(default)]
pub struct CodeInlineTokens {
    pub background: String,
    pub border_color: String,
    pub border_radius: String,
}

#[derive(Debug, Clone, Default, Deserialize, Serialize)]
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

#[derive(Debug, Clone, Default, Deserialize, Serialize)]
#[serde(default)]
pub struct TableTokens {
    pub header_background: String,
    pub header_text_color: String,
    pub header_border_color: String,
    pub header_border_width: String,
    pub header_font: String,
    pub header_weight: u16,
    pub font_size: String,
    pub row_border_color: String,
    pub row_border_width: String,
    pub stripe_background: String,
    pub vertical_lines: bool,
    pub cell_padding: String,
}

#[derive(Debug, Clone, Default, Deserialize, Serialize)]
#[serde(default)]
pub struct HorizontalRuleTokens {
    pub color: String,
    pub width: String,
    pub thickness: String,
    pub style: String,
}

#[derive(Debug, Clone, Default, Deserialize, Serialize)]
#[serde(default)]
pub struct LinkTokens {
    pub color: String,
    pub underline: bool,
}

#[derive(Debug, Clone, Default, Deserialize, Serialize)]
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

#[derive(Debug, Clone, Default, Deserialize, Serialize)]
#[serde(default)]
pub struct ListTokens {
    pub bullet_color: String,
    pub indent: String,
    pub nested_indent: String,
    pub task_checked_color: String,
    pub task_unchecked_color: String,
}

#[derive(Debug, Clone, Default, Deserialize, Serialize)]
#[serde(default)]
pub struct FootnoteTokens {
    pub separator_color: String,
    pub separator_width: String,
    pub text_size: String,
    pub number_color: String,
    pub backref_color: String,
}

#[derive(Debug, Clone, Default, Deserialize, Serialize)]
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

#[derive(Debug, Clone, Default, Deserialize, Serialize)]
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

#[derive(Debug, Clone, Default, Deserialize, Serialize)]
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

#[derive(Debug, Clone, Default, Deserialize, Serialize)]
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

#[derive(Debug, Clone, Default, Deserialize, Serialize)]
#[serde(default)]
pub struct EmphasisTokens {
    pub strikethrough_color: String,
}

#[derive(Debug, Clone, Default, Deserialize, Serialize)]
#[serde(default)]
pub struct MathTokens {
    pub color: String,
}

#[derive(Debug, Clone, Default, Deserialize, Serialize)]
#[serde(default)]
pub struct HighlightTokens {
    pub fill: String,
    pub fill_opacity: f64,
    pub text_color: String,
    pub border_radius: String,
}

#[derive(Debug, Clone, Default, Deserialize, Serialize)]
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
#[derive(Debug, Clone, Default, Deserialize, Serialize)]
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

#[derive(Debug, Clone, Default, Deserialize, Serialize)]
#[serde(default)]
pub struct SyntaxStyleTokens {
    pub color: String,
    pub bold: Option<bool>,
    pub italic: Option<bool>,
}

trait ColorResolvable {
    fn resolve_colors(&mut self, colors: &HashMap<String, String>);
}

impl ThemeTokens {
    pub(crate) fn resolve_color_refs(&mut self) {
        let colors = self.colors.clone();

        self.page.resolve_colors(&colors);
        self.text.resolve_colors(&colors);
        self.headings.resolve_colors(&colors);
        self.code_block.resolve_colors(&colors);
        self.code_inline.resolve_colors(&colors);
        self.blockquote.resolve_colors(&colors);
        self.table.resolve_colors(&colors);
        self.horizontal_rule.resolve_colors(&colors);
        self.links.resolve_colors(&colors);
        self.images.resolve_colors(&colors);
        self.list.resolve_colors(&colors);
        self.footnotes.resolve_colors(&colors);
        self.alerts.resolve_colors(&colors);
        self.toc.resolve_colors(&colors);
        self.page_numbers.resolve_colors(&colors);
        self.title_page.resolve_colors(&colors);
        self.emphasis.resolve_colors(&colors);
        self.math.resolve_colors(&colors);
        self.highlight.resolve_colors(&colors);
        self.description_list.resolve_colors(&colors);
        self.syntax.resolve_colors(&colors);
    }
}

fn resolve_color(field: &mut String, colors: &HashMap<String, String>) {
    if !field.is_empty()
        && !field.starts_with('#')
        && let Some(hex) = colors.get(field.as_str())
    {
        *field = hex.clone();
    }
}

impl ColorResolvable for PageTokens {
    fn resolve_colors(&mut self, colors: &HashMap<String, String>) {
        resolve_color(&mut self.background, colors);
    }
}

impl ColorResolvable for TextTokens {
    fn resolve_colors(&mut self, colors: &HashMap<String, String>) {
        resolve_color(&mut self.color, colors);
    }
}

impl ColorResolvable for HeadingTokens {
    fn resolve_colors(&mut self, colors: &HashMap<String, String>) {
        resolve_color(&mut self.color, colors);
        self.h1.resolve_colors(colors);
        self.h2.resolve_colors(colors);
        self.h3.resolve_colors(colors);
        self.h4.resolve_colors(colors);
        self.h5.resolve_colors(colors);
        self.h6.resolve_colors(colors);
    }
}

impl ColorResolvable for HeadingLevelTokens {
    fn resolve_colors(&mut self, colors: &HashMap<String, String>) {
        resolve_color(&mut self.color, colors);
    }
}

impl ColorResolvable for CodeBlockTokens {
    fn resolve_colors(&mut self, colors: &HashMap<String, String>) {
        resolve_color(&mut self.background, colors);
        resolve_color(&mut self.border_color, colors);
        resolve_color(&mut self.left_accent_color, colors);
        resolve_color(&mut self.language_label_color, colors);
    }
}

impl ColorResolvable for CodeInlineTokens {
    fn resolve_colors(&mut self, colors: &HashMap<String, String>) {
        resolve_color(&mut self.background, colors);
        resolve_color(&mut self.border_color, colors);
    }
}

impl ColorResolvable for BlockquoteTokens {
    fn resolve_colors(&mut self, colors: &HashMap<String, String>) {
        resolve_color(&mut self.border_color, colors);
        resolve_color(&mut self.background, colors);
        resolve_color(&mut self.text_color, colors);
    }
}

impl ColorResolvable for TableTokens {
    fn resolve_colors(&mut self, colors: &HashMap<String, String>) {
        resolve_color(&mut self.header_background, colors);
        resolve_color(&mut self.header_text_color, colors);
        resolve_color(&mut self.header_border_color, colors);
        resolve_color(&mut self.row_border_color, colors);
        resolve_color(&mut self.stripe_background, colors);
    }
}

impl ColorResolvable for HorizontalRuleTokens {
    fn resolve_colors(&mut self, colors: &HashMap<String, String>) {
        resolve_color(&mut self.color, colors);
    }
}

impl ColorResolvable for LinkTokens {
    fn resolve_colors(&mut self, colors: &HashMap<String, String>) {
        resolve_color(&mut self.color, colors);
    }
}

impl ColorResolvable for ImageTokens {
    fn resolve_colors(&mut self, colors: &HashMap<String, String>) {
        resolve_color(&mut self.caption_color, colors);
    }
}

impl ColorResolvable for ListTokens {
    fn resolve_colors(&mut self, colors: &HashMap<String, String>) {
        resolve_color(&mut self.bullet_color, colors);
        resolve_color(&mut self.task_checked_color, colors);
        resolve_color(&mut self.task_unchecked_color, colors);
    }
}

impl ColorResolvable for FootnoteTokens {
    fn resolve_colors(&mut self, colors: &HashMap<String, String>) {
        resolve_color(&mut self.separator_color, colors);
        resolve_color(&mut self.number_color, colors);
        resolve_color(&mut self.backref_color, colors);
    }
}

impl ColorResolvable for AlertTokens {
    fn resolve_colors(&mut self, colors: &HashMap<String, String>) {
        resolve_color(&mut self.note_color, colors);
        resolve_color(&mut self.tip_color, colors);
        resolve_color(&mut self.important_color, colors);
        resolve_color(&mut self.warning_color, colors);
        resolve_color(&mut self.caution_color, colors);
    }
}

impl ColorResolvable for TocTokens {
    fn resolve_colors(&mut self, colors: &HashMap<String, String>) {
        resolve_color(&mut self.entry_color, colors);
        resolve_color(&mut self.page_number_color, colors);
    }
}

impl ColorResolvable for PageNumberTokens {
    fn resolve_colors(&mut self, colors: &HashMap<String, String>) {
        resolve_color(&mut self.color, colors);
    }
}

impl ColorResolvable for TitlePageTokens {
    fn resolve_colors(&mut self, colors: &HashMap<String, String>) {
        resolve_color(&mut self.title_color, colors);
        resolve_color(&mut self.subtitle_color, colors);
        resolve_color(&mut self.author_color, colors);
        resolve_color(&mut self.date_color, colors);
        resolve_color(&mut self.separator_color, colors);
    }
}

impl ColorResolvable for EmphasisTokens {
    fn resolve_colors(&mut self, colors: &HashMap<String, String>) {
        resolve_color(&mut self.strikethrough_color, colors);
    }
}

impl ColorResolvable for MathTokens {
    fn resolve_colors(&mut self, colors: &HashMap<String, String>) {
        resolve_color(&mut self.color, colors);
    }
}

impl ColorResolvable for HighlightTokens {
    fn resolve_colors(&mut self, colors: &HashMap<String, String>) {
        resolve_color(&mut self.fill, colors);
        resolve_color(&mut self.text_color, colors);
    }
}

impl ColorResolvable for DescriptionListTokens {
    fn resolve_colors(&mut self, colors: &HashMap<String, String>) {
        resolve_color(&mut self.term_color, colors);
    }
}

impl ColorResolvable for SyntaxTokens {
    fn resolve_colors(&mut self, colors: &HashMap<String, String>) {
        resolve_color(&mut self.background, colors);
        self.text.resolve_colors(colors);
        self.keyword.resolve_colors(colors);
        self.string.resolve_colors(colors);
        self.number.resolve_colors(colors);
        self.function.resolve_colors(colors);
        self.type_.resolve_colors(colors);
        self.comment.resolve_colors(colors);
        self.constant.resolve_colors(colors);
        self.boolean.resolve_colors(colors);
        self.operator.resolve_colors(colors);
        self.property.resolve_colors(colors);
        self.tag.resolve_colors(colors);
        self.attribute.resolve_colors(colors);
        self.variable.resolve_colors(colors);
        self.builtin.resolve_colors(colors);
        self.punctuation.resolve_colors(colors);
        self.escape.resolve_colors(colors);
    }
}

impl ColorResolvable for SyntaxStyleTokens {
    fn resolve_colors(&mut self, colors: &HashMap<String, String>) {
        resolve_color(&mut self.color, colors);
    }
}

#[cfg(test)]
mod tests {
    use super::ThemeTokens;

    #[test]
    fn resolves_color_references_across_token_sections() {
        let mut tokens = ThemeTokens::default();
        tokens
            .colors
            .insert("paper".to_string(), "#ffffff".to_string());
        tokens
            .colors
            .insert("accent".to_string(), "#e135ff".to_string());
        tokens.page.background = "paper".to_string();
        tokens.headings.h1.color = "accent".to_string();
        tokens.code_block.left_accent_color = "accent".to_string();
        tokens.syntax.keyword.color = "accent".to_string();

        tokens.resolve_color_refs();

        assert_eq!(tokens.page.background, "#ffffff");
        assert_eq!(tokens.headings.h1.color, "#e135ff");
        assert_eq!(tokens.code_block.left_accent_color, "#e135ff");
        assert_eq!(tokens.syntax.keyword.color, "#e135ff");
    }
}
