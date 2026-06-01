//! Terminal reader configuration.
//!
//! `reader.toml` is machine-written state: last-used theme, outline visibility,
//! and glyph tier. `config.toml` is hand-edited user configuration and is only
//! read, never overwritten.

use std::collections::BTreeMap;
use std::path::PathBuf;

use serde::{Deserialize, Serialize};

#[derive(Debug, Default, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct ReaderConfig {
    pub theme: Option<String>,
    pub outline: Option<bool>,
    pub glyphs: Option<String>,
}

#[derive(Debug, Default, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct UserConfig {
    pub default_theme: Option<String>,
    pub default_width: Option<u16>,
    pub color: Option<String>,
    pub pager: Option<String>,
    pub glyphs: Option<String>,
    pub bookmarks: BTreeMap<String, String>,
    pub keybindings: BTreeMap<String, String>,
}

#[derive(Debug, Default, Clone)]
pub struct ReaderSettings {
    pub reader: ReaderConfig,
    pub user: UserConfig,
}

#[derive(Debug, Default, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct ReaderSession {
    pub active_tab: usize,
    pub tabs: Vec<SessionTab>,
}

#[derive(Debug, Default, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct SessionTab {
    pub path: PathBuf,
    pub scroll: u16,
}

impl ReaderSettings {
    pub fn theme(&self) -> Option<&str> {
        self.user_theme().or_else(|| self.reader_theme())
    }

    pub fn user_theme(&self) -> Option<&str> {
        self.user
            .default_theme
            .as_deref()
            .filter(|value| !value.trim().is_empty())
    }

    pub fn reader_theme(&self) -> Option<&str> {
        self.reader
            .theme
            .as_deref()
            .filter(|value| !value.trim().is_empty())
    }

    pub fn glyphs(&self) -> Option<&str> {
        self.user
            .glyphs
            .as_deref()
            .or(self.reader.glyphs.as_deref())
            .filter(|value| !value.trim().is_empty())
    }

    pub fn width(&self) -> Option<u16> {
        self.user.default_width
    }

    pub fn color(&self) -> Option<&str> {
        self.user
            .color
            .as_deref()
            .filter(|value| !value.trim().is_empty())
    }

    pub fn pager(&self) -> Option<&str> {
        self.user
            .pager
            .as_deref()
            .filter(|value| !value.trim().is_empty())
    }
}

fn project_config_dir() -> Option<PathBuf> {
    directories::ProjectDirs::from("tech", "hyperbliss", "silkprint")
        .map(|dirs| dirs.config_dir().to_path_buf())
}

fn project_data_dir() -> Option<PathBuf> {
    directories::ProjectDirs::from("tech", "hyperbliss", "silkprint")
        .map(|dirs| dirs.data_dir().to_path_buf())
}

fn reader_config_path() -> Option<PathBuf> {
    project_config_dir().map(|dir| dir.join("reader.toml"))
}

fn user_config_path() -> Option<PathBuf> {
    project_config_dir().map(|dir| dir.join("config.toml"))
}

fn session_path() -> Option<PathBuf> {
    project_data_dir().map(|dir| dir.join("state.toml"))
}

fn load_toml<T>(path: Option<PathBuf>) -> T
where
    T: Default + serde::de::DeserializeOwned,
{
    let Some(path) = path else {
        return T::default();
    };
    std::fs::read_to_string(path)
        .ok()
        .and_then(|s| toml::from_str(&s).ok())
        .unwrap_or_default()
}

/// Load saved preferences, returning defaults if none exist or parsing fails.
pub fn load() -> ReaderConfig {
    load_toml(reader_config_path())
}

pub fn load_settings() -> ReaderSettings {
    ReaderSettings {
        reader: load(),
        user: load_toml(user_config_path()),
    }
}

pub fn load_session() -> ReaderSession {
    load_toml(session_path())
}

/// Persist preferences. Best-effort: failures (no config dir, read-only fs) are
/// silently ignored — a reader shouldn't error over a settings file.
pub fn save(config: &ReaderConfig) {
    let Some(path) = reader_config_path() else {
        return;
    };
    if let Some(parent) = path.parent() {
        let _ = std::fs::create_dir_all(parent);
    }
    if let Ok(serialized) = toml::to_string_pretty(config) {
        let _ = std::fs::write(path, serialized);
    }
}

pub fn save_session(session: &ReaderSession) {
    let Some(path) = session_path() else {
        return;
    };
    if let Some(parent) = path.parent() {
        let _ = std::fs::create_dir_all(parent);
    }
    if let Ok(serialized) = toml::to_string_pretty(session) {
        let _ = std::fs::write(path, serialized);
    }
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use super::{ReaderConfig, ReaderSession, ReaderSettings, SessionTab, UserConfig};

    #[test]
    fn parses_user_config_shape() {
        let config: UserConfig = toml::from_str(
            r#"
default_theme = "silk-dark"
default_width = 96
color = "never"
pager = "less -R"
glyphs = "unicode"

[bookmarks]
docs = "/tmp/docs"

[keybindings]
quit = "q"
"#,
        )
        .expect("user config parses");

        assert_eq!(config.default_theme.as_deref(), Some("silk-dark"));
        assert_eq!(config.default_width, Some(96));
        assert_eq!(config.color.as_deref(), Some("never"));
        assert_eq!(config.pager.as_deref(), Some("less -R"));
        assert_eq!(config.glyphs.as_deref(), Some("unicode"));
        assert_eq!(
            config.bookmarks.get("docs").map(String::as_str),
            Some("/tmp/docs")
        );
        assert_eq!(
            config.keybindings.get("quit").map(String::as_str),
            Some("q")
        );
    }

    #[test]
    fn user_config_layers_above_reader_state() {
        let settings = ReaderSettings {
            reader: ReaderConfig {
                theme: Some("silkcircuit-dawn".to_string()),
                outline: Some(true),
                glyphs: Some("nerdfont".to_string()),
            },
            user: UserConfig {
                default_theme: Some("silk-dark".to_string()),
                glyphs: Some("unicode".to_string()),
                ..UserConfig::default()
            },
        };

        assert_eq!(settings.theme(), Some("silk-dark"));
        assert_eq!(settings.glyphs(), Some("unicode"));
    }

    #[test]
    fn parses_reader_session_shape() {
        let session: ReaderSession = toml::from_str(
            r#"
active_tab = 1

[[tabs]]
path = "/tmp/a.md"
scroll = 4

[[tabs]]
path = "/tmp/b.md"
scroll = 9
"#,
        )
        .expect("session parses");

        assert_eq!(session.active_tab, 1);
        assert_eq!(session.tabs.len(), 2);
        assert_eq!(session.tabs[0].path, PathBuf::from("/tmp/a.md"));
        assert_eq!(session.tabs[1].scroll, 9);
    }

    #[test]
    fn serializes_reader_session_shape() {
        let session = ReaderSession {
            active_tab: 0,
            tabs: vec![SessionTab {
                path: PathBuf::from("/tmp/doc.md"),
                scroll: 12,
            }],
        };
        let serialized = toml::to_string_pretty(&session).expect("serialize");

        assert!(serialized.contains("active_tab = 0"));
        assert!(serialized.contains("path = \"/tmp/doc.md\""));
        assert!(serialized.contains("scroll = 12"));
    }
}
