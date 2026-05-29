//! Glyph tiers: Nerd Font icons, with Unicode and ASCII fallbacks.
//!
//! Every chrome/marker glyph is requested by name; the active [`GlyphTier`]
//! decides which codepoint is returned. Nerd Font glyphs are drawn from the
//! Font Awesome range (`U+F000`–`U+F2FF`), which is present across Nerd Font
//! patched fonts.

use super::caps::GlyphTier;
use super::model::AlertKind;

/// Resolves named glyphs for the active tier.
#[derive(Debug, Clone, Copy)]
pub struct Glyphs {
    tier: GlyphTier,
}

impl Glyphs {
    pub fn new(tier: GlyphTier) -> Self {
        Self { tier }
    }

    pub fn tier(self) -> GlyphTier {
        self.tier
    }

    /// Admonition icon for an alert kind.
    pub fn alert(self, kind: AlertKind) -> &'static str {
        match self.tier {
            GlyphTier::NerdFont => match kind {
                AlertKind::Note => "\u{f05a}",      // info-circle
                AlertKind::Tip => "\u{f0eb}",       // lightbulb-o
                AlertKind::Important => "\u{f06a}", // exclamation-circle
                AlertKind::Warning => "\u{f071}",   // exclamation-triangle
                AlertKind::Caution => "\u{f06d}",   // fire
            },
            GlyphTier::Unicode => match kind {
                AlertKind::Note => "\u{2139}",      // ℹ
                AlertKind::Tip => "\u{2726}",       // ✦
                AlertKind::Important => "\u{203c}", // ‼
                AlertKind::Warning => "\u{26a0}",   // ⚠
                AlertKind::Caution => "\u{26a1}",   // ⚡
            },
            GlyphTier::Ascii => match kind {
                AlertKind::Note => "i",
                AlertKind::Tip => "*",
                AlertKind::Important | AlertKind::Warning | AlertKind::Caution => "!",
            },
        }
    }

    /// Uppercase label for an alert kind (always plain text).
    pub fn alert_label(kind: AlertKind) -> &'static str {
        match kind {
            AlertKind::Note => "NOTE",
            AlertKind::Tip => "TIP",
            AlertKind::Important => "IMPORTANT",
            AlertKind::Warning => "WARNING",
            AlertKind::Caution => "CAUTION",
        }
    }

    /// Unordered list bullet.
    pub fn bullet(self) -> &'static str {
        match self.tier {
            GlyphTier::NerdFont | GlyphTier::Unicode => "\u{2022}", // •
            GlyphTier::Ascii => "*",
        }
    }

    /// Nested-list bullet (one level deeper).
    pub fn bullet_nested(self) -> &'static str {
        match self.tier {
            GlyphTier::NerdFont | GlyphTier::Unicode => "\u{25e6}", // ◦
            GlyphTier::Ascii => "-",
        }
    }

    /// Checked / unchecked task markers.
    pub fn task(self, checked: bool) -> &'static str {
        match (self.tier, checked) {
            (GlyphTier::NerdFont, true) => "\u{f14a}",  // check-square
            (GlyphTier::NerdFont, false) => "\u{f0c8}", // square
            (GlyphTier::Unicode, true) => "\u{2611}",   // ☑
            (GlyphTier::Unicode, false) => "\u{2610}",  // ☐
            (GlyphTier::Ascii, true) => "[x]",
            (GlyphTier::Ascii, false) => "[ ]",
        }
    }

    /// Marker shown before a hyperlink.
    pub fn link(self) -> &'static str {
        match self.tier {
            GlyphTier::NerdFont => "\u{f0c1}", // link
            GlyphTier::Unicode => "\u{2197}",  // ↗
            GlyphTier::Ascii => "",
        }
    }

    /// Block-quote left bar.
    pub fn quote_bar(self) -> &'static str {
        match self.tier {
            GlyphTier::NerdFont | GlyphTier::Unicode => "\u{2503}", // ┃
            GlyphTier::Ascii => "|",
        }
    }

    /// Outline / TOC entry marker.
    pub fn outline_marker(self) -> &'static str {
        match self.tier {
            GlyphTier::NerdFont | GlyphTier::Unicode => "\u{203a}", // ›
            GlyphTier::Ascii => ">",
        }
    }

    /// Heading prefix marker (decorative).
    pub fn heading_marker(self) -> &'static str {
        match self.tier {
            GlyphTier::NerdFont => "\u{f0c9}", // bars/section feel
            GlyphTier::Unicode => "\u{00a7}",  // §
            GlyphTier::Ascii => "#",
        }
    }

    /// Horizontal rule fill character.
    pub fn rule(self) -> &'static str {
        match self.tier {
            GlyphTier::NerdFont | GlyphTier::Unicode => "\u{2500}", // ─
            GlyphTier::Ascii => "-",
        }
    }

    /// Truncation ellipsis.
    pub fn ellipsis(self) -> &'static str {
        match self.tier {
            GlyphTier::NerdFont | GlyphTier::Unicode => "\u{2026}", // …
            GlyphTier::Ascii => "...",
        }
    }

    /// Branding diamond for the status/title bar.
    pub fn diamond(self) -> &'static str {
        match self.tier {
            GlyphTier::NerdFont | GlyphTier::Unicode => "\u{25c8}", // ◈
            GlyphTier::Ascii => "*",
        }
    }

    /// Devicon for a code-fence language label.
    ///
    /// Falls back to an empty string outside the Nerd Font tier — the textual
    /// language label carries the meaning there.
    pub fn language(self, lang: &str) -> &'static str {
        if self.tier != GlyphTier::NerdFont {
            return "";
        }
        match lang.to_ascii_lowercase().as_str() {
            "rust" | "rs" => "\u{e7a8}",
            "python" | "py" => "\u{e606}",
            "javascript" | "js" | "jsx" => "\u{e74e}",
            "typescript" | "ts" | "tsx" => "\u{e628}",
            "go" => "\u{e627}",
            "c" => "\u{e61e}",
            "cpp" | "c++" => "\u{e61d}",
            "java" => "\u{e738}",
            "ruby" | "rb" => "\u{e739}",
            "html" => "\u{e736}",
            "css" | "scss" => "\u{e749}",
            "json" | "jsonc" | "yaml" | "yml" => "\u{e60b}",
            "toml" => "\u{e6b2}",
            "bash" | "sh" | "shell" | "zsh" => "\u{f489}",
            "markdown" | "md" => "\u{e609}",
            "haskell" => "\u{e777}",
            "lua" => "\u{e620}",
            "php" => "\u{e73d}",
            "swift" => "\u{e755}",
            "kotlin" => "\u{e634}",
            "sql" => "\u{e706}",
            "docker" | "dockerfile" => "\u{f308}",
            _ => "\u{f121}", // generic code
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ascii_tier_has_no_high_codepoints() {
        let g = Glyphs::new(GlyphTier::Ascii);
        for s in [
            g.bullet(),
            g.quote_bar(),
            g.rule(),
            g.task(true),
            g.task(false),
        ] {
            assert!(s.is_ascii(), "ascii glyph {s:?} must be ASCII");
        }
    }

    #[test]
    fn alert_glyphs_distinct_per_kind_in_unicode() {
        let g = Glyphs::new(GlyphTier::Unicode);
        let icons = [
            g.alert(AlertKind::Note),
            g.alert(AlertKind::Tip),
            g.alert(AlertKind::Warning),
        ];
        assert_eq!(
            icons.iter().collect::<std::collections::HashSet<_>>().len(),
            3
        );
    }

    #[test]
    fn language_icon_only_in_nerd_tier() {
        assert_eq!(Glyphs::new(GlyphTier::Unicode).language("rust"), "");
        assert!(!Glyphs::new(GlyphTier::NerdFont).language("rust").is_empty());
    }
}
