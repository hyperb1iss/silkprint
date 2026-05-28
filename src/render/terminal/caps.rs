//! Terminal capability detection: color depth, glyph tier, graphics protocol,
//! size, and multiplexer awareness.
//!
//! Each axis degrades independently. The reader stays usable at the lowest tier
//! of every axis; richer tiers only enhance.

use std::env;
use std::io::IsTerminal;

/// Color depth the terminal supports.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ColorTier {
    TrueColor,
    Ansi256,
    Ansi16,
    None,
}

/// Which glyph set to draw chrome and markers with.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GlyphTier {
    NerdFont,
    Unicode,
    Ascii,
}

impl GlyphTier {
    /// Parse a `--glyphs` / `SILKPRINT_GLYPHS` value.
    pub fn parse(value: &str) -> Option<Self> {
        match value.trim().to_ascii_lowercase().as_str() {
            "nerd" | "nerdfont" | "nerd-font" => Some(Self::NerdFont),
            "unicode" | "uni" => Some(Self::Unicode),
            "ascii" | "plain" => Some(Self::Ascii),
            _ => None,
        }
    }
}

/// Inline-image graphics protocol available, if any.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GraphicsProtocol {
    Kitty,
    Iterm2,
    Sixel,
    None,
}

/// How the user asked for color (mirrors the existing `--color` flag).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ColorChoice {
    Auto,
    Always,
    Never,
}

impl ColorChoice {
    pub fn parse(value: &str) -> Self {
        match value {
            "always" => Self::Always,
            "never" => Self::Never,
            _ => Self::Auto,
        }
    }
}

/// Detected terminal capabilities.
#[derive(Debug, Clone, Copy)]
pub struct Capabilities {
    pub color: ColorTier,
    pub glyphs: GlyphTier,
    pub graphics: GraphicsProtocol,
    pub width: u16,
    pub height: u16,
    pub is_tty: bool,
    pub in_tmux: bool,
}

impl Capabilities {
    /// Detect capabilities, applying explicit user overrides.
    ///
    /// `glyph_override` and `color` come from CLI flags; `images` is the
    /// negation of `--no-images`.
    pub fn detect(color: ColorChoice, glyph_override: Option<GlyphTier>, images: bool) -> Self {
        let is_tty = std::io::stdout().is_terminal();
        let in_tmux = env::var_os("TMUX").is_some()
            || env::var("TERM").is_ok_and(|t| t.starts_with("screen") || t.starts_with("tmux"));

        Self {
            color: detect_color(color, is_tty),
            glyphs: glyph_override
                .or_else(detect_glyphs_from_env)
                .unwrap_or(GlyphTier::NerdFont),
            graphics: if images {
                detect_graphics()
            } else {
                GraphicsProtocol::None
            },
            width: terminal_size().0,
            height: terminal_size().1,
            is_tty,
            in_tmux,
        }
    }
}

fn detect_color(choice: ColorChoice, is_tty: bool) -> ColorTier {
    match choice {
        ColorChoice::Never => return ColorTier::None,
        ColorChoice::Auto if !is_tty => return ColorTier::None,
        _ => {}
    }
    if env::var_os("NO_COLOR").is_some() && choice != ColorChoice::Always {
        return ColorTier::None;
    }
    if let Ok(ct) = env::var("COLORTERM") {
        if ct.contains("truecolor") || ct.contains("24bit") {
            return ColorTier::TrueColor;
        }
    }
    match env::var("TERM") {
        Ok(term) if term.contains("256color") => ColorTier::Ansi256,
        Ok(term) if term == "dumb" => ColorTier::None,
        // Modern emulators that don't always advertise COLORTERM still do truecolor.
        _ if is_truecolor_term_program() => ColorTier::TrueColor,
        _ => ColorTier::Ansi16,
    }
}

fn is_truecolor_term_program() -> bool {
    matches!(
        env::var("TERM_PROGRAM").as_deref(),
        Ok("ghostty" | "iTerm.app" | "WezTerm" | "vscode" | "Apple_Terminal")
    ) || env::var_os("KITTY_WINDOW_ID").is_some()
        || env::var_os("WEZTERM_PANE").is_some()
}

fn detect_glyphs_from_env() -> Option<GlyphTier> {
    env::var("SILKPRINT_GLYPHS")
        .ok()
        .and_then(|v| GlyphTier::parse(&v))
}

fn detect_graphics() -> GraphicsProtocol {
    if env::var_os("KITTY_WINDOW_ID").is_some()
        || env::var("TERM").is_ok_and(|t| t.contains("kitty"))
        || env::var("TERM_PROGRAM").is_ok_and(|t| t == "ghostty")
    {
        return GraphicsProtocol::Kitty;
    }
    if env::var("TERM_PROGRAM").is_ok_and(|t| t == "iTerm.app") || env::var_os("WEZTERM_PANE").is_some()
    {
        return GraphicsProtocol::Iterm2;
    }
    GraphicsProtocol::None
}

fn terminal_size() -> (u16, u16) {
    ratatui::crossterm::terminal::size().unwrap_or((80, 24))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_glyph_tier() {
        assert_eq!(GlyphTier::parse("nerd"), Some(GlyphTier::NerdFont));
        assert_eq!(GlyphTier::parse("UNICODE"), Some(GlyphTier::Unicode));
        assert_eq!(GlyphTier::parse("ascii"), Some(GlyphTier::Ascii));
        assert_eq!(GlyphTier::parse("bogus"), None);
    }

    #[test]
    fn color_choice_never_forces_no_color() {
        assert_eq!(detect_color(ColorChoice::Never, true), ColorTier::None);
    }

    #[test]
    fn color_choice_auto_disables_color_when_not_tty() {
        assert_eq!(detect_color(ColorChoice::Auto, false), ColorTier::None);
    }
}
