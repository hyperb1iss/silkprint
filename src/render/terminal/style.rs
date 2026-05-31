//! Resolve semantic [`Role`]s into concrete terminal styles.
//!
//! The [`ContentStyleResolver`] reads the same `ResolvedTheme` tokens the PDF
//! pipeline uses, so terminal content mirrors the PDF's color decisions for
//! everything the PDF derives from tokens. (Two PDF-side values are hardcoded
//! in the Typst emitter rather than token-driven — the generic alert accent and
//! image border — so the terminal, which is fully token-driven, can differ
//! there; see the plan's §8.3.)

use crate::theme::ResolvedTheme;
use crate::theme::tokens::SyntaxStyleTokens;

use super::model::{Mods, Rgb, Role, SyntaxRole};

/// A fully resolved terminal style for one inline run.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
#[allow(clippy::struct_excessive_bools)]
pub struct Style {
    pub fg: Option<Rgb>,
    pub bg: Option<Rgb>,
    pub bold: bool,
    pub italic: bool,
    pub underline: bool,
    pub strikethrough: bool,
    pub dim: bool,
}

/// Parse a `#rrggbb` (or `#rgb`) hex string into [`Rgb`].
///
/// Returns `None` for empty or malformed values so callers can fall back.
pub fn parse_hex(value: &str) -> Option<Rgb> {
    let hex = value.strip_prefix('#')?;
    let (r, g, b) = match hex.len() {
        6 => (
            u8::from_str_radix(&hex[0..2], 16).ok()?,
            u8::from_str_radix(&hex[2..4], 16).ok()?,
            u8::from_str_radix(&hex[4..6], 16).ok()?,
        ),
        3 => {
            let expand = |c: &str| u8::from_str_radix(&c.repeat(2), 16).ok();
            (
                expand(&hex[0..1])?,
                expand(&hex[1..2])?,
                expand(&hex[2..3])?,
            )
        }
        _ => return None,
    };
    Some(Rgb(r, g, b))
}

/// Resolves semantic roles to concrete styles for a given theme.
pub struct ContentStyleResolver<'a> {
    theme: &'a ResolvedTheme,
}

impl<'a> ContentStyleResolver<'a> {
    pub fn new(theme: &'a ResolvedTheme) -> Self {
        Self { theme }
    }

    fn tokens(&self) -> &crate::theme::tokens::ThemeTokens {
        &self.theme.tokens
    }

    /// Body text color, the document's baseline foreground.
    pub fn body_color(&self) -> Option<Rgb> {
        parse_hex(&self.tokens().text.color)
    }

    /// Page background color (used to pick a sensible default when a terminal
    /// theme differs from the document theme).
    pub fn page_background(&self) -> Option<Rgb> {
        parse_hex(&self.tokens().page.background)
    }

    fn heading_color(&self, level: u8) -> Option<Rgb> {
        let h = &self.tokens().headings;
        let per_level = match level {
            1 => &h.h1.color,
            2 => &h.h2.color,
            3 => &h.h3.color,
            4 => &h.h4.color,
            5 => &h.h5.color,
            _ => &h.h6.color,
        };
        parse_hex(per_level)
            .or_else(|| parse_hex(&h.color))
            .or_else(|| self.body_color())
    }

    fn link_color(&self) -> Option<Rgb> {
        parse_hex(&self.tokens().links.color).or_else(|| self.body_color())
    }

    fn strong_color(&self) -> Option<Rgb> {
        parse_hex(&self.tokens().alerts.note_color)
            .or_else(|| parse_hex(&self.tokens().title_page.separator_color))
            .or_else(|| self.link_color())
    }

    fn italic_color(&self) -> Option<Rgb> {
        parse_hex(&self.tokens().alerts.tip_color)
            .or_else(|| parse_hex(&self.tokens().blockquote.border_color))
            .or_else(|| self.link_color())
    }

    fn strong_italic_color(&self) -> Option<Rgb> {
        parse_hex(&self.tokens().alerts.warning_color)
            .or_else(|| parse_hex(&self.tokens().highlight.fill))
            .or_else(|| self.strong_color())
    }

    fn emphasis_color(&self, mods: Mods) -> Option<Rgb> {
        if mods.strikethrough {
            parse_hex(&self.tokens().emphasis.strikethrough_color).or_else(|| self.body_color())
        } else if mods.bold && mods.italic {
            self.strong_italic_color()
        } else if mods.bold {
            self.strong_color()
        } else if mods.italic {
            self.italic_color()
        } else if mods.underline {
            self.link_color()
        } else {
            None
        }
    }

    /// Resolve the color + intrinsic flags for a syntax token role.
    pub fn syntax_style(&self, role: SyntaxRole) -> Style {
        let syntax = &self.tokens().syntax;
        let tok: &SyntaxStyleTokens = match role {
            SyntaxRole::Keyword => &syntax.keyword,
            SyntaxRole::String => &syntax.string,
            SyntaxRole::Number => &syntax.number,
            SyntaxRole::Function => &syntax.function,
            SyntaxRole::Type => &syntax.type_,
            SyntaxRole::Comment => &syntax.comment,
            SyntaxRole::Constant => &syntax.constant,
            SyntaxRole::Boolean => &syntax.boolean,
            SyntaxRole::Operator => &syntax.operator,
            SyntaxRole::Property => &syntax.property,
            SyntaxRole::Tag => &syntax.tag,
            SyntaxRole::Attribute => &syntax.attribute,
            SyntaxRole::Variable => &syntax.variable,
            SyntaxRole::Builtin => &syntax.builtin,
            SyntaxRole::Punctuation => &syntax.punctuation,
            SyntaxRole::Escape => &syntax.escape,
            SyntaxRole::Text => &syntax.text,
        };
        let fg = parse_hex(&tok.color)
            .or_else(|| parse_hex(&syntax.text.color))
            .or_else(|| self.body_color());
        Style {
            fg,
            bold: tok.bold.unwrap_or(false),
            italic: tok.italic.unwrap_or(false),
            ..Style::default()
        }
    }

    /// Resolve a role + inline modifiers into a concrete style.
    pub fn resolve(&self, role: Role, mods: Mods) -> Style {
        let mut style = match role {
            Role::Body => Style {
                fg: self.body_color(),
                ..Style::default()
            },
            Role::Heading(level) => Style {
                fg: self.heading_color(level),
                bold: true,
                ..Style::default()
            },
            Role::Link => Style {
                fg: self.link_color(),
                underline: self.tokens().links.underline,
                ..Style::default()
            },
            Role::InlineCode => Style {
                fg: parse_hex(&self.tokens().syntax.text.color).or_else(|| self.body_color()),
                bg: parse_hex(&self.tokens().code_inline.background),
                ..Style::default()
            },
            Role::Highlight => Style {
                fg: parse_hex(&self.tokens().highlight.text_color).or_else(|| self.body_color()),
                bg: parse_hex(&self.tokens().highlight.fill),
                ..Style::default()
            },
            Role::Quote => Style {
                fg: parse_hex(&self.tokens().blockquote.text_color).or_else(|| self.body_color()),
                italic: self.tokens().blockquote.italic,
                ..Style::default()
            },
            Role::Math => Style {
                fg: parse_hex(&self.tokens().math.color).or_else(|| self.body_color()),
                ..Style::default()
            },
            Role::Muted => Style {
                fg: parse_hex(&self.tokens().images.caption_color).or_else(|| self.body_color()),
                dim: true,
                ..Style::default()
            },
            Role::Syntax(s) => self.syntax_style(s),
        };

        if matches!(role, Role::Body | Role::Quote | Role::Muted)
            && let Some(color) = self.emphasis_color(mods)
        {
            style.fg = Some(color);
        }
        style.bold |= mods.bold;
        style.italic |= mods.italic;
        style.underline |= mods.underline;
        style.strikethrough |= mods.strikethrough;
        style.dim |= mods.dim;
        style
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::theme::tokens::ThemeTokens;

    fn theme_with_terminal_accents() -> ResolvedTheme {
        let mut tokens = ThemeTokens::default();
        tokens.text.color = "#111111".to_string();
        tokens.headings.color = "#e135ff".to_string();
        tokens.links.color = "#80ffea".to_string();
        tokens.alerts.note_color = "#ff00ff".to_string();
        tokens.alerts.tip_color = "#00ff00".to_string();
        tokens.alerts.warning_color = "#ffff00".to_string();
        tokens.emphasis.strikethrough_color = "#6a6a82".to_string();
        tokens.images.caption_color = "#444444".to_string();
        tokens.code_inline.background = "#eeeeee".to_string();
        tokens.syntax.text.color = "#222222".to_string();
        ResolvedTheme {
            tokens,
            tmtheme_xml: String::new(),
        }
    }

    #[test]
    fn parses_six_digit_hex() {
        assert_eq!(parse_hex("#e135ff"), Some(Rgb(0xe1, 0x35, 0xff)));
    }

    #[test]
    fn parses_three_digit_hex() {
        assert_eq!(parse_hex("#0af"), Some(Rgb(0x00, 0xaa, 0xff)));
    }

    #[test]
    fn rejects_empty_and_malformed() {
        assert_eq!(parse_hex(""), None);
        assert_eq!(parse_hex("text_primary"), None);
        assert_eq!(parse_hex("#zz0011"), None);
    }

    #[test]
    fn resolves_markdown_emphasis_to_theme_colors() {
        let theme = theme_with_terminal_accents();
        let resolver = ContentStyleResolver::new(&theme);

        assert_eq!(
            resolver.resolve(Role::Body, Mods::default().with_bold()).fg,
            Some(Rgb(0xff, 0x00, 0xff))
        );
        assert_eq!(
            resolver
                .resolve(Role::Body, Mods::default().with_italic())
                .fg,
            Some(Rgb(0x00, 0xff, 0x00))
        );
        assert_eq!(
            resolver
                .resolve(Role::Body, Mods::default().with_bold().with_italic())
                .fg,
            Some(Rgb(0xff, 0xff, 0x00))
        );
        assert_eq!(
            resolver
                .resolve(Role::Body, Mods::default().with_underline())
                .fg,
            Some(Rgb(0x80, 0xff, 0xea))
        );
        assert_eq!(
            resolver
                .resolve(Role::Body, Mods::default().with_strikethrough())
                .fg,
            Some(Rgb(0x6a, 0x6a, 0x82))
        );
    }

    #[test]
    fn keeps_semantic_role_colors_inside_emphasis() {
        let theme = theme_with_terminal_accents();
        let resolver = ContentStyleResolver::new(&theme);

        assert_eq!(
            resolver
                .resolve(Role::InlineCode, Mods::default().with_bold())
                .fg,
            Some(Rgb(0x22, 0x22, 0x22))
        );
        assert_eq!(
            resolver.resolve(Role::Link, Mods::default().with_bold()).fg,
            Some(Rgb(0x80, 0xff, 0xea))
        );
        assert_eq!(
            resolver.resolve(Role::Muted, Mods::default()).fg,
            Some(Rgb(0x44, 0x44, 0x44))
        );
    }
}
