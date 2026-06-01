//! Terminal capability detection: color depth, glyph tier, graphics protocol,
//! size, and multiplexer awareness.
//!
//! Each axis degrades independently. The reader stays usable at the lowest tier
//! of every axis; richer tiers only enhance.

use std::env;
use std::io::IsTerminal;

#[cfg(unix)]
use std::fs::OpenOptions;
#[cfg(unix)]
use std::io::Write;
#[cfg(unix)]
use std::os::fd::{AsFd, AsRawFd};
#[cfg(unix)]
use std::time::{Duration, Instant};

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

/// Light/dark classification for the terminal's own background.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BackgroundTone {
    Light,
    Dark,
}

impl BackgroundTone {
    pub fn silk_default_theme(self) -> &'static str {
        match self {
            Self::Light => "silk-light",
            Self::Dark => "silk-dark",
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

/// Query the terminal background and classify it as light or dark.
pub fn detect_background_tone() -> Option<BackgroundTone> {
    query_osc11_background().or_else(detect_colorfgbg_background)
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
    if let Ok(ct) = env::var("COLORTERM")
        && (ct.contains("truecolor") || ct.contains("24bit"))
    {
        return ColorTier::TrueColor;
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

#[cfg(unix)]
fn query_osc11_background() -> Option<BackgroundTone> {
    let mut tty = OpenOptions::new()
        .read(true)
        .write(true)
        .open("/dev/tty")
        .ok()?;
    let _raw_mode = RawMode::enable()?;
    tty.write_all(b"\x1b]11;?\x07").ok()?;
    tty.flush().ok()?;

    let deadline = Instant::now() + Duration::from_millis(90);
    let mut response = Vec::new();
    let mut buf = [0_u8; 96];

    while Instant::now() < deadline && response.len() < 512 {
        let ready = {
            let mut fds = [nix::poll::PollFd::new(
                tty.as_fd(),
                nix::poll::PollFlags::POLLIN,
            )];
            nix::poll::poll(&mut fds, 15_u16).ok()?
        };
        if ready == 0 {
            continue;
        }
        let read = nix::unistd::read(tty.as_raw_fd(), &mut buf).ok()?;
        if read == 0 {
            break;
        }
        response.extend_from_slice(&buf[..read]);
        if response.contains(&b'\x07') || response.windows(2).any(|win| win == b"\x1b\\") {
            break;
        }
    }

    parse_osc11_rgb(&response).map(tone_from_rgb)
}

#[cfg(not(unix))]
fn query_osc11_background() -> Option<BackgroundTone> {
    None
}

#[cfg(unix)]
struct RawMode;

#[cfg(unix)]
impl RawMode {
    fn enable() -> Option<Self> {
        ratatui::crossterm::terminal::enable_raw_mode().ok()?;
        Some(Self)
    }
}

#[cfg(unix)]
impl Drop for RawMode {
    fn drop(&mut self) {
        let _ = ratatui::crossterm::terminal::disable_raw_mode();
    }
}

fn parse_osc11_rgb(bytes: &[u8]) -> Option<(u8, u8, u8)> {
    let text = std::str::from_utf8(bytes).ok()?;
    let prefix = "\x1b]11;";
    let rest = text.split_once(prefix)?.1;
    let end = rest
        .find('\x07')
        .or_else(|| rest.find("\x1b\\"))
        .unwrap_or(rest.len());
    parse_color_payload(rest[..end].trim())
}

fn parse_color_payload(payload: &str) -> Option<(u8, u8, u8)> {
    if let Some(hex) = payload.strip_prefix('#') {
        return parse_hex_rgb(hex);
    }
    let rgb = payload
        .strip_prefix("rgb:")
        .or_else(|| payload.strip_prefix("rgba:"))?;
    let mut components = rgb.split('/');
    let r = parse_x_color_component(components.next()?)?;
    let g = parse_x_color_component(components.next()?)?;
    let b = parse_x_color_component(components.next()?)?;
    Some((r, g, b))
}

fn parse_hex_rgb(hex: &str) -> Option<(u8, u8, u8)> {
    if hex.len() != 6 {
        return None;
    }
    let (r, rest) = hex.split_at(2);
    let (g, b) = rest.split_at(2);
    Some((
        u8::from_str_radix(r, 16).ok()?,
        u8::from_str_radix(g, 16).ok()?,
        u8::from_str_radix(b, 16).ok()?,
    ))
}

fn parse_x_color_component(component: &str) -> Option<u8> {
    let digits = component.trim();
    if digits.is_empty() || digits.len() > 4 {
        return None;
    }
    let value = u32::from(u16::from_str_radix(digits, 16).ok()?);
    let max = (1_u32 << (digits.len() * 4)) - 1;
    u8::try_from((value * 255 + (max / 2)) / max).ok()
}

fn tone_from_rgb((r, g, b): (u8, u8, u8)) -> BackgroundTone {
    let luma = (2_126 * u32::from(r)) + (7_152 * u32::from(g)) + (722 * u32::from(b));
    if luma >= 1_280_000 {
        BackgroundTone::Light
    } else {
        BackgroundTone::Dark
    }
}

fn detect_colorfgbg_background() -> Option<BackgroundTone> {
    let value = env::var("COLORFGBG").ok()?;
    colorfgbg_tone(&value)
}

fn colorfgbg_tone(value: &str) -> Option<BackgroundTone> {
    let bg = value.rsplit(';').next()?.parse::<u8>().ok()?;
    match bg {
        0..=6 | 8 => Some(BackgroundTone::Dark),
        7 | 9..=15 => Some(BackgroundTone::Light),
        _ => None,
    }
}

fn detect_graphics() -> GraphicsProtocol {
    if env::var_os("KITTY_WINDOW_ID").is_some()
        || env::var("TERM").is_ok_and(|t| t.contains("kitty"))
        || env::var("TERM_PROGRAM").is_ok_and(|t| t == "ghostty")
    {
        return GraphicsProtocol::Kitty;
    }
    if env::var("TERM_PROGRAM").is_ok_and(|t| t == "iTerm.app")
        || env::var_os("WEZTERM_PANE").is_some()
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

    #[test]
    fn parses_osc11_rgb_backgrounds() {
        assert_eq!(
            parse_osc11_rgb(b"\x1b]11;rgb:ffff/ffff/ffff\x07").map(tone_from_rgb),
            Some(BackgroundTone::Light)
        );
        assert_eq!(
            parse_osc11_rgb(b"\x1b]11;rgb:0000/1010/2020\x1b\\").map(tone_from_rgb),
            Some(BackgroundTone::Dark)
        );
    }

    #[test]
    fn parses_colorfgbg_background_index() {
        assert_eq!(colorfgbg_tone("15;0"), Some(BackgroundTone::Dark));
        assert_eq!(colorfgbg_tone("0;15"), Some(BackgroundTone::Light));
        assert_eq!(colorfgbg_tone("0;99"), None);
    }
}
