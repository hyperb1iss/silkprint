//! Built-in theme registry.
//!
//! All built-in themes are embedded as TOML strings at compile time.
//! The default theme is `silkcircuit-dawn`.

/// Theme metadata for `--list-themes`.
#[derive(Debug, Clone)]
pub struct ThemeInfo {
    pub name: &'static str,
    pub variant: &'static str,
    pub description: &'static str,
    pub print_safe: bool,
    pub family: &'static str,
}

// ─── Embedded theme TOML sources ───────────────────────────────────────────

// Base syntax fallbacks
pub const BASE_SYNTAX_LIGHT_TOML: &str = include_str!("../../themes/_base-syntax-light.toml");
pub const BASE_SYNTAX_DARK_TOML: &str = include_str!("../../themes/_base-syntax-dark.toml");

// Signature
const SILK_LIGHT_TOML: &str = include_str!("../../themes/signature/silk-light.toml");
const SILK_DARK_TOML: &str = include_str!("../../themes/signature/silk-dark.toml");
const MANUSCRIPT_TOML: &str = include_str!("../../themes/signature/manuscript.toml");
const MONOCHROME_TOML: &str = include_str!("../../themes/signature/monochrome.toml");

// SilkCircuit
const SILKCIRCUIT_NEON_TOML: &str = include_str!("../../themes/silkcircuit/silkcircuit-neon.toml");
const SILKCIRCUIT_VIBRANT_TOML: &str =
    include_str!("../../themes/silkcircuit/silkcircuit-vibrant.toml");
const SILKCIRCUIT_SOFT_TOML: &str = include_str!("../../themes/silkcircuit/silkcircuit-soft.toml");
const SILKCIRCUIT_GLOW_TOML: &str = include_str!("../../themes/silkcircuit/silkcircuit-glow.toml");
const SILKCIRCUIT_DAWN_TOML: &str = include_str!("../../themes/silkcircuit/silkcircuit-dawn.toml");

// Greyscale
const GREYSCALE_WARM_TOML: &str = include_str!("../../themes/greyscale/greyscale-warm.toml");
const GREYSCALE_COOL_TOML: &str = include_str!("../../themes/greyscale/greyscale-cool.toml");
const HIGH_CONTRAST_TOML: &str = include_str!("../../themes/greyscale/high-contrast.toml");

// Classic
const ACADEMIC_TOML: &str = include_str!("../../themes/classic/academic.toml");
const TYPEWRITER_TOML: &str = include_str!("../../themes/classic/typewriter.toml");
const NEWSPAPER_TOML: &str = include_str!("../../themes/classic/newspaper.toml");
const PARCHMENT_TOML: &str = include_str!("../../themes/classic/parchment.toml");

// Futuristic
const CYBERPUNK_TOML: &str = include_str!("../../themes/futuristic/cyberpunk.toml");
const TERMINAL_TOML: &str = include_str!("../../themes/futuristic/terminal.toml");
const HOLOGRAM_TOML: &str = include_str!("../../themes/futuristic/hologram.toml");
const SYNTHWAVE_TOML: &str = include_str!("../../themes/futuristic/synthwave.toml");
const MATRIX_TOML: &str = include_str!("../../themes/futuristic/matrix.toml");

// Nature
const FOREST_TOML: &str = include_str!("../../themes/nature/forest.toml");
const OCEAN_TOML: &str = include_str!("../../themes/nature/ocean.toml");
const SUNSET_TOML: &str = include_str!("../../themes/nature/sunset.toml");
const ARCTIC_TOML: &str = include_str!("../../themes/nature/arctic.toml");
const SAKURA_TOML: &str = include_str!("../../themes/nature/sakura.toml");

// Artistic
const NOIR_TOML: &str = include_str!("../../themes/artistic/noir.toml");
const CANDY_TOML: &str = include_str!("../../themes/artistic/candy.toml");
const BLUEPRINT_TOML: &str = include_str!("../../themes/artistic/blueprint.toml");
const WITCH_TOML: &str = include_str!("../../themes/artistic/witch.toml");

// Developer
const NORD_TOML: &str = include_str!("../../themes/developer/nord.toml");
const DRACULA_TOML: &str = include_str!("../../themes/developer/dracula.toml");
const SOLARIZED_LIGHT_TOML: &str = include_str!("../../themes/developer/solarized-light.toml");
const SOLARIZED_DARK_TOML: &str = include_str!("../../themes/developer/solarized-dark.toml");
const CATPPUCCIN_MOCHA_TOML: &str = include_str!("../../themes/developer/catppuccin-mocha.toml");
const CATPPUCCIN_LATTE_TOML: &str = include_str!("../../themes/developer/catppuccin-latte.toml");
const GRUVBOX_DARK_TOML: &str = include_str!("../../themes/developer/gruvbox-dark.toml");
const GRUVBOX_LIGHT_TOML: &str = include_str!("../../themes/developer/gruvbox-light.toml");
const TOKYO_NIGHT_TOML: &str = include_str!("../../themes/developer/tokyo-night.toml");
const ROSE_PINE_TOML: &str = include_str!("../../themes/developer/rose-pine.toml");

// ─── Theme lookup ──────────────────────────────────────────────────────────

/// Get the built-in theme TOML source by name.
///
/// Returns `None` if the theme name is not recognized.
pub fn get_builtin_theme(name: &str) -> Option<&'static str> {
    match name {
        // Signature
        "silk-light" => Some(SILK_LIGHT_TOML),
        "silk-dark" => Some(SILK_DARK_TOML),
        "manuscript" => Some(MANUSCRIPT_TOML),
        "monochrome" => Some(MONOCHROME_TOML),
        // SilkCircuit
        "silkcircuit-neon" => Some(SILKCIRCUIT_NEON_TOML),
        "silkcircuit-vibrant" => Some(SILKCIRCUIT_VIBRANT_TOML),
        "silkcircuit-soft" => Some(SILKCIRCUIT_SOFT_TOML),
        "silkcircuit-glow" => Some(SILKCIRCUIT_GLOW_TOML),
        "silkcircuit-dawn" => Some(SILKCIRCUIT_DAWN_TOML),
        // Greyscale
        "greyscale-warm" => Some(GREYSCALE_WARM_TOML),
        "greyscale-cool" => Some(GREYSCALE_COOL_TOML),
        "high-contrast" => Some(HIGH_CONTRAST_TOML),
        // Classic
        "academic" => Some(ACADEMIC_TOML),
        "typewriter" => Some(TYPEWRITER_TOML),
        "newspaper" => Some(NEWSPAPER_TOML),
        "parchment" => Some(PARCHMENT_TOML),
        // Futuristic
        "cyberpunk" => Some(CYBERPUNK_TOML),
        "terminal" => Some(TERMINAL_TOML),
        "hologram" => Some(HOLOGRAM_TOML),
        "synthwave" => Some(SYNTHWAVE_TOML),
        "matrix" => Some(MATRIX_TOML),
        // Nature
        "forest" => Some(FOREST_TOML),
        "ocean" => Some(OCEAN_TOML),
        "sunset" => Some(SUNSET_TOML),
        "arctic" => Some(ARCTIC_TOML),
        "sakura" => Some(SAKURA_TOML),
        // Artistic
        "noir" => Some(NOIR_TOML),
        "candy" => Some(CANDY_TOML),
        "blueprint" => Some(BLUEPRINT_TOML),
        "witch" => Some(WITCH_TOML),
        // Developer
        "nord" => Some(NORD_TOML),
        "dracula" => Some(DRACULA_TOML),
        "solarized-light" => Some(SOLARIZED_LIGHT_TOML),
        "solarized-dark" => Some(SOLARIZED_DARK_TOML),
        "catppuccin-mocha" => Some(CATPPUCCIN_MOCHA_TOML),
        "catppuccin-latte" => Some(CATPPUCCIN_LATTE_TOML),
        "gruvbox-dark" => Some(GRUVBOX_DARK_TOML),
        "gruvbox-light" => Some(GRUVBOX_LIGHT_TOML),
        "tokyo-night" => Some(TOKYO_NIGHT_TOML),
        "rose-pine" => Some(ROSE_PINE_TOML),
        _ => None,
    }
}

// ─── Theme listing ─────────────────────────────────────────────────────────

/// List all 40 built-in themes with metadata.
#[allow(clippy::too_many_lines)]
pub fn list_themes() -> Vec<ThemeInfo> {
    vec![
        // ─── Signature Collection ────────────────────────────────
        ThemeInfo {
            name: "silk-light",
            variant: "light",
            description: "Clean, warm, professional — signature serif elegance",
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
            description: "Electric blue & magenta on warm cream — the default",
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
