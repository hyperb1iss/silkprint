//! Built-in theme registry.
//!
//! All built-in themes are embedded as TOML strings at compile time.
//! The default theme is `silk-light`.

/// Theme metadata for `--list-themes`.
#[derive(Debug, Clone)]
pub struct ThemeInfo {
    pub name: String,
    pub variant: String,
    pub description: String,
    pub print_safe: bool,
    pub family: String,
}

/// Get the built-in theme TOML source by name.
pub fn get_builtin_theme(_name: &str) -> Option<&'static str> {
    // Stub — Wave 2D populates the full registry
    None
}

/// List all available built-in themes.
pub fn list_themes() -> Vec<ThemeInfo> {
    // Stub — Wave 2D populates the full registry
    vec![ThemeInfo {
        name: "silk-light".to_string(),
        variant: "light".to_string(),
        description: "Clean, warm, professional — the default".to_string(),
        print_safe: true,
        family: "signature".to_string(),
    }]
}
