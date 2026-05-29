//! Persisted reader preferences.
//!
//! Stored as TOML under the platform config dir (e.g.
//! `~/.config/silkprint/reader.toml`). The reader saves the last-used theme,
//! outline visibility, and glyph tier on theme-confirm and quit, and restores
//! them on the next launch (unless overridden by a CLI flag).

use std::path::PathBuf;

use serde::{Deserialize, Serialize};

#[derive(Debug, Default, Clone, Serialize, Deserialize)]
pub struct ReaderConfig {
    pub theme: Option<String>,
    pub outline: Option<bool>,
    pub glyphs: Option<String>,
}

fn config_path() -> Option<PathBuf> {
    directories::ProjectDirs::from("tech", "hyperbliss", "silkprint")
        .map(|dirs| dirs.config_dir().join("reader.toml"))
}

/// Load saved preferences, returning defaults if none exist or parsing fails.
pub fn load() -> ReaderConfig {
    let Some(path) = config_path() else {
        return ReaderConfig::default();
    };
    std::fs::read_to_string(path)
        .ok()
        .and_then(|s| toml::from_str(&s).ok())
        .unwrap_or_default()
}

/// Persist preferences. Best-effort: failures (no config dir, read-only fs) are
/// silently ignored — a reader shouldn't error over a settings file.
pub fn save(config: &ReaderConfig) {
    let Some(path) = config_path() else {
        return;
    };
    if let Some(parent) = path.parent() {
        let _ = std::fs::create_dir_all(parent);
    }
    if let Ok(serialized) = toml::to_string_pretty(config) {
        let _ = std::fs::write(path, serialized);
    }
}
