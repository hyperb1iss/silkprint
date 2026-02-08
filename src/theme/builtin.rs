//! Built-in theme registry.
//!
//! All built-in themes are embedded as TOML strings at compile time.
//! The default theme is `silk-light`.

/// Theme metadata for `--list-themes`.
#[derive(Debug, Clone)]
pub struct ThemeInfo {
    pub name: &'static str,
    pub variant: &'static str,
    pub description: &'static str,
    pub print_safe: bool,
    pub family: &'static str,
}

/// Embedded TOML source for `silk-light` — the default theme.
const SILK_LIGHT_TOML: &str = include_str!("../../themes/signature/silk-light.toml");

/// Embedded base syntax theme for light variants.
pub const BASE_SYNTAX_LIGHT_TOML: &str = include_str!("../../themes/_base-syntax-light.toml");

/// Embedded base syntax theme for dark variants.
pub const BASE_SYNTAX_DARK_TOML: &str = include_str!("../../themes/_base-syntax-dark.toml");

/// Get the built-in theme TOML source by name.
///
/// Returns `None` if the theme name is not recognized.
/// Phase 1 only ships `silk-light` with actual TOML content.
pub fn get_builtin_theme(name: &str) -> Option<&'static str> {
    match name {
        "silk-light" => Some(SILK_LIGHT_TOML),
        _ => None,
    }
}

/// List all 40 built-in themes with metadata.
///
/// Only `silk-light` has actual TOML content in Phase 1.
/// The rest are placeholders for `--list-themes` display.
#[allow(clippy::too_many_lines)]
pub fn list_themes() -> Vec<ThemeInfo> {
    vec![
        // ─── Signature Collection ────────────────────────────────
        ThemeInfo {
            name: "silk-light",
            variant: "light",
            description: "Clean, warm, professional — the default",
            print_safe: true,
            family: "signature",
        },
        ThemeInfo {
            name: "silk-dark",
            variant: "dark",
            description: "Deep navy-black, refined elegance",
            print_safe: false,
            family: "signature",
        },
        ThemeInfo {
            name: "manuscript",
            variant: "light",
            description: "Warm cream paper, serif-heavy, old-world feel",
            print_safe: true,
            family: "signature",
        },
        ThemeInfo {
            name: "monochrome",
            variant: "light",
            description: "Pure black on white, zero color, maximum ink efficiency",
            print_safe: true,
            family: "signature",
        },
        // ─── SilkCircuit Collection ──────────────────────────────
        ThemeInfo {
            name: "silkcircuit-neon",
            variant: "dark",
            description: "Full Neon (100%) — Electric Purple headings, Neon Cyan accents, Coral constants",
            print_safe: false,
            family: "silkcircuit",
        },
        ThemeInfo {
            name: "silkcircuit-vibrant",
            variant: "dark",
            description: "Vibrant (85%) — maximum vibrancy, saturated spectrum",
            print_safe: false,
            family: "silkcircuit",
        },
        ThemeInfo {
            name: "silkcircuit-soft",
            variant: "dark",
            description: "Soft (70%) — reduced chroma for extended reading",
            print_safe: false,
            family: "silkcircuit",
        },
        ThemeInfo {
            name: "silkcircuit-glow",
            variant: "dark",
            description: "Glow (110%) — maximum contrast, darkest backgrounds",
            print_safe: false,
            family: "silkcircuit",
        },
        ThemeInfo {
            name: "silkcircuit-dawn",
            variant: "light",
            description: "Dawn — deep purples and teals on warm cream",
            print_safe: true,
            family: "silkcircuit",
        },
        // ─── Greyscale Collection ────────────────────────────────
        ThemeInfo {
            name: "greyscale-warm",
            variant: "light",
            description: "Warm grey tones with cream undertones, cozy and readable",
            print_safe: true,
            family: "greyscale",
        },
        ThemeInfo {
            name: "greyscale-cool",
            variant: "light",
            description: "Blue-tinted cool greys, clinical and modern",
            print_safe: true,
            family: "greyscale",
        },
        ThemeInfo {
            name: "high-contrast",
            variant: "light",
            description: "Extreme B&W, no mid-tones, maximum readability/accessibility",
            print_safe: true,
            family: "greyscale",
        },
        // ─── Classic / Literary Collection ───────────────────────
        ThemeInfo {
            name: "academic",
            variant: "light",
            description: "Traditional academic paper, conservative and authoritative",
            print_safe: true,
            family: "classic",
        },
        ThemeInfo {
            name: "typewriter",
            variant: "light",
            description: "Raw mechanical feel, like typed on a real typewriter",
            print_safe: true,
            family: "classic",
        },
        ThemeInfo {
            name: "newspaper",
            variant: "light",
            description: "Dense editorial feel, bold headlines, ink that stains your fingers",
            print_safe: true,
            family: "classic",
        },
        ThemeInfo {
            name: "parchment",
            variant: "light",
            description: "Aged warm paper, old-world scholarly, candlewax and leather",
            print_safe: true,
            family: "classic",
        },
        // ─── Futuristic / Sci-Fi Collection ─────────────────────
        ThemeInfo {
            name: "cyberpunk",
            variant: "dark",
            description: "Hot neon pink + cyan on deep dark, rain-soaked megacity",
            print_safe: false,
            family: "futuristic",
        },
        ThemeInfo {
            name: "terminal",
            variant: "dark",
            description: "Green phosphor on black, classic CRT, cursor blinking in the dark",
            print_safe: false,
            family: "futuristic",
        },
        ThemeInfo {
            name: "hologram",
            variant: "dark",
            description: "Clean blue/white sci-fi, floating projections in a sterile lab",
            print_safe: false,
            family: "futuristic",
        },
        ThemeInfo {
            name: "synthwave",
            variant: "dark",
            description: "Retro-future sunset, chrome sun melting into a grid horizon",
            print_safe: false,
            family: "futuristic",
        },
        ThemeInfo {
            name: "matrix",
            variant: "dark",
            description: "Green cascade on pure void black, reality decoded",
            print_safe: false,
            family: "futuristic",
        },
        // ─── Nature Collection ───────────────────────────────────
        ThemeInfo {
            name: "forest",
            variant: "light",
            description: "Deep greens, bark browns, dappled light through old-growth canopy",
            print_safe: true,
            family: "nature",
        },
        ThemeInfo {
            name: "ocean",
            variant: "dark",
            description: "Navy depths, seafoam teal, living coral accents",
            print_safe: false,
            family: "nature",
        },
        ThemeInfo {
            name: "sunset",
            variant: "light",
            description: "Warm amber to pink, golden hour painting everything warm",
            print_safe: true,
            family: "nature",
        },
        ThemeInfo {
            name: "arctic",
            variant: "light",
            description: "Ice blue, silver, crystalline polar silence",
            print_safe: true,
            family: "nature",
        },
        ThemeInfo {
            name: "sakura",
            variant: "light",
            description: "Cherry blossom pink, matcha green, petals on a garden path",
            print_safe: true,
            family: "nature",
        },
        // ─── Artistic / Bold Collection ──────────────────────────
        ThemeInfo {
            name: "noir",
            variant: "dark",
            description: "Film noir, stark shadows, a single red light cutting through dark",
            print_safe: false,
            family: "artistic",
        },
        ThemeInfo {
            name: "candy",
            variant: "light",
            description: "Sweet pastels, pop art energy, sugar-coated without the toothache",
            print_safe: false,
            family: "artistic",
        },
        ThemeInfo {
            name: "blueprint",
            variant: "dark",
            description: "Engineering blueprint, white lines on Prussian blue",
            print_safe: false,
            family: "artistic",
        },
        ThemeInfo {
            name: "witch",
            variant: "dark",
            description: "Mystical purples, potion green, candlelit grimoire pages",
            print_safe: false,
            family: "artistic",
        },
        // ─── Developer Favorites Collection ──────────────────────
        ThemeInfo {
            name: "nord",
            variant: "dark",
            description: "Arctic blue-grey, calm and muted — the Aurora palette",
            print_safe: false,
            family: "developer",
        },
        ThemeInfo {
            name: "dracula",
            variant: "dark",
            description: "Dark purple elegance — pink, cyan, green, orange accents",
            print_safe: false,
            family: "developer",
        },
        ThemeInfo {
            name: "solarized-light",
            variant: "light",
            description: "Ethan Schoonover's classic — warm yellowed paper, precise accents",
            print_safe: true,
            family: "developer",
        },
        ThemeInfo {
            name: "solarized-dark",
            variant: "dark",
            description: "Ethan Schoonover's classic — teal depths, same precise accents",
            print_safe: false,
            family: "developer",
        },
        ThemeInfo {
            name: "catppuccin-mocha",
            variant: "dark",
            description: "Soothing warm pastels on dark base — cozy and gentle",
            print_safe: false,
            family: "developer",
        },
        ThemeInfo {
            name: "catppuccin-latte",
            variant: "light",
            description: "Soothing warm pastels on light base — the daytime variant",
            print_safe: true,
            family: "developer",
        },
        ThemeInfo {
            name: "gruvbox-dark",
            variant: "dark",
            description: "Retro groove — warm earth tones, bright accents on dark",
            print_safe: false,
            family: "developer",
        },
        ThemeInfo {
            name: "gruvbox-light",
            variant: "light",
            description: "Retro groove — faded accents on warm creamy paper",
            print_safe: true,
            family: "developer",
        },
        ThemeInfo {
            name: "tokyo-night",
            variant: "dark",
            description: "Deep indigo with soft neon — purple, blue, green pop",
            print_safe: false,
            family: "developer",
        },
        ThemeInfo {
            name: "rose-pine",
            variant: "dark",
            description: "Soho vibes — muted rose, gold, iris on dusky purple",
            print_safe: false,
            family: "developer",
        },
    ]
}
