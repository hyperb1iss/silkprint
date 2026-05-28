//! Terminal/TUI markdown reader.
//!
//! Reuses the upstream pipeline (comrak parse, theme resolution, content
//! checks) and adds a terminal-specific walker ([`walk`]) producing a
//! width-independent [`model::RenderedDoc`], which [`ansi`] renders to styled
//! one-shot output. The TUI front-end is layered on the same model.

pub mod ansi;
pub mod caps;
pub mod glyphs;
pub mod highlight;
pub mod layout;
pub mod model;
pub mod style;
pub mod table;
pub mod walk;

use crate::error::SilkprintError;
use crate::theme::ResolvedTheme;
use crate::warnings::WarningCollector;

use caps::{Capabilities, ColorChoice, GlyphTier};
use glyphs::Glyphs;

/// Terminal rendering knobs, distinct from the PDF `RenderOptions`.
#[derive(Debug, Clone)]
pub struct TerminalRenderOptions {
    /// Color policy (auto/always/never), mirroring the `--color` flag.
    pub color: ColorChoice,
    /// Explicit glyph-tier override; `None` auto-detects (defaults to Nerd Font).
    pub glyphs: Option<GlyphTier>,
    /// Whether inline-image graphics protocols may be used (Wave 2).
    pub images: bool,
    /// Force a content width instead of probing the terminal.
    pub width: Option<u16>,
}

impl Default for TerminalRenderOptions {
    fn default() -> Self {
        Self {
            color: ColorChoice::Auto,
            glyphs: None,
            images: true,
            width: None,
        }
    }
}

/// Render a markdown body to a styled ANSI string (one-shot).
pub fn render_to_string(
    body: &str,
    theme: &ResolvedTheme,
    options: &TerminalRenderOptions,
    warnings: &mut WarningCollector,
) -> Result<String, SilkprintError> {
    let arena = comrak::Arena::new();
    let root = super::markdown::parse(&arena, body);
    super::markdown::check_content(root, warnings);

    let doc = walk::walk(root, warnings);

    let mut caps = Capabilities::detect(options.color, options.glyphs, options.images);
    if let Some(width) = options.width {
        caps.width = width;
    }
    let glyphs = Glyphs::new(caps.glyphs);

    Ok(ansi::render(&doc, theme, &caps, glyphs))
}
