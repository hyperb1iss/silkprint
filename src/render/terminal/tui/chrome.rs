//! Map the active document theme to an opaline chrome theme for the TUI shell.
//!
//! The document content stays themed by silkprint's `ResolvedTheme`; opaline
//! themes only the surrounding chrome (borders, status bar, outline, popups).
//! We bridge by loading the opaline builtin whose slug matches the silkprint
//! theme, falling back to a sensible default when there's no match — so chrome
//! and content share a palette wherever the families overlap (silkcircuit,
//! nord, dracula, catppuccin, gruvbox, tokyo-night, rose-pine, …).

use ratatui::style::Color;

/// Resolved chrome colors for the TUI shell.
#[derive(Debug, Clone, Copy)]
pub struct Chrome {
    pub panel_bg: Color,
    pub text: Color,
    pub muted: Color,
    pub accent: Color,
    pub accent2: Color,
    pub border: Color,
    pub border_focused: Color,
    pub selection_bg: Color,
}

impl Chrome {
    /// Build chrome from the opaline theme matching `theme_name`, or a default.
    pub fn for_theme(theme_name: &str) -> Self {
        let theme = opaline::load_by_name(theme_name).unwrap_or_default();
        let color = |token: &str| Color::from(theme.color(token));
        Self {
            panel_bg: color("bg.panel"),
            text: color("text.primary"),
            muted: color("text.muted"),
            accent: color("accent.primary"),
            accent2: color("accent.secondary"),
            border: color("border.unfocused"),
            border_focused: color("border.focused"),
            selection_bg: color("bg.selection"),
        }
    }
}
